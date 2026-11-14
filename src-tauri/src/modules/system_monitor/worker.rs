//! The system-monitor thread. Sleeps while the panel is closed; samples once a second while it's open.

use std::collections::VecDeque;
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};

use std::net::IpAddr;

use sysinfo::{NetworkData, Networks, System};
use tauri::{AppHandle, Manager};
use tauri_specta::Event;
use windows::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;
use windows::Win32::System::Performance::{
    PDH_FMT_COUNTERVALUE, PDH_FMT_DOUBLE, PDH_HCOUNTER, PDH_HQUERY, PdhAddEnglishCounterW, PdhCloseQuery,
    PdhCollectQueryData, PdhGetFormattedCounterValue, PdhOpenQueryW,
};
use windows::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};
use windows::core::{HSTRING, PCWSTR, w};

use crate::settings::{Reading, Settings, SettingsState};

use super::gpu::Gpu;
use super::{Battery, Msg, Point, Shared, SystemSample, SystemSampled, mib, per_second, push_history};

pub fn run(app: AppHandle, rx: Receiver<Msg>) {
    let mut sampler = Sampler {
        system: System::new(),
        cpu: None,
        gpu: None,
        networks: Networks::new(),
        last: Instant::now(),
        history: VecDeque::new(),
    };
    let mut visible = false;
    loop {
        let msg = if visible { rx.recv_timeout(Duration::from_secs(1)) } else { rx.recv().map_err(|_| RecvTimeoutError::Disconnected) };
        let sample = match msg {
            Ok(Msg::PanelVisible(true)) => {
                visible = true;
                sampler.set_gpu(wants_gpu(&app.state::<SettingsState>().get()));
                sampler.begin()
            }
            Ok(Msg::PanelVisible(false)) => {
                visible = false;
                sampler.cpu = None; // closes the counters: nothing is measured while hidden
                sampler.gpu = None;
                sampler.history.clear();
                SystemSample::default()
            }
            Err(RecvTimeoutError::Timeout) => {
                // A GPU reading may have been added to (or taken off) the board since the last second.
                sampler.set_gpu(wants_gpu(&app.state::<SettingsState>().get()));
                sampler.sample()
            }
            Ok(Msg::Stop) | Err(RecvTimeoutError::Disconnected) => break,
        };
        let shared = app.state::<Shared>();
        *shared.state.lock().unwrap() = sample.clone();
        if visible {
            let _ = SystemSampled(sample).emit(&app);
        }
    }
}

struct Sampler {
    /// Memory only; CPU comes from `cpu`.
    system: System,
    /// Open only while the panel is.
    cpu: Option<Cpu>,
    /// Open only while the panel is and a GPU reading is on the board: opening it takes a moment.
    gpu: Option<Gpu>,
    networks: Networks,
    /// When CPU and network counters were last read; rates are measured from here.
    last: Instant,
    history: VecDeque<Point>,
}

impl Sampler {
    /// The panel just opened: start the CPU and network counters, and show what doesn't need them.
    fn begin(&mut self) -> SystemSample {
        self.cpu = Cpu::open();
        self.networks.refresh(true);
        self.last = Instant::now();
        self.history.clear();
        self.read(None, None, None)
    }

    /// Opens or closes the GPU counter to match whether anything on the board shows the GPU.
    fn set_gpu(&mut self, wanted: bool) {
        match (wanted, self.gpu.is_some()) {
            (true, false) => self.gpu = Gpu::open(),
            (false, true) => self.gpu = None,
            _ => {}
        }
    }

    fn sample(&mut self) -> SystemSample {
        let (cpu, cpu_mhz) = self.cpu.as_ref().map_or((None, None), Cpu::read);
        let gpu = self.gpu.as_mut().and_then(Gpu::read);
        self.networks.refresh(true);
        let elapsed = self.last.elapsed();
        self.last = Instant::now();
        // shortcut: virtual adapters (VPNs, Hyper-V) can count the same traffic twice; skip them by
        // type if the numbers look doubled.
        let (received, sent) = self.networks.values().fold((0, 0), |(r, s), n| (r + n.received(), s + n.transmitted()));
        let (download, upload) = (per_second(received, elapsed), per_second(sent, elapsed));
        let mut sample = self.read(cpu, Some(download), Some(upload));
        sample.cpu_mhz = cpu_mhz;
        sample.gpu = gpu;
        let memory = (u64::from(sample.memory_used) * 100 / u64::from(sample.memory_total.max(1))) as u8;
        push_history(&mut self.history, Point { cpu: cpu.unwrap_or(0), gpu: gpu.unwrap_or(0), memory, download, upload });
        sample.history = self.history.clone();
        sample
    }

    fn read(&mut self, cpu: Option<u8>, download: Option<u32>, upload: Option<u32>) -> SystemSample {
        self.system.refresh_memory();
        let (disk_free, disk_total) = system_drive_space();
        SystemSample {
            cpu,
            cpu_mhz: None,
            gpu: None,
            memory_used: mib(self.system.used_memory()),
            memory_total: mib(self.system.total_memory()),
            download,
            upload,
            download_total: mib(self.networks.values().filter(|n| has_address(n)).map(|n| n.total_received()).sum()),
            upload_total: mib(self.networks.values().filter(|n| has_address(n)).map(|n| n.total_transmitted()).sum()),
            local_ip: local_ip(&self.networks),
            battery: battery(),
            disk_free,
            disk_total,
            history: self.history.clone(),
        }
    }
}

/// Whether the GPU is shown anywhere: a GPU tile on the board, or a wide tile reading it.
fn wants_gpu(settings: &Settings) -> bool {
    settings.monitor_readings.contains(&Reading::Gpu)
        || settings.pages.iter().flatten().flatten().any(|t| t.id == "system-monitor/gpu" || t.readings.is_some_and(|r| r.contains(&Reading::Gpu)))
}

/// An adapter with a real address (not just loopback or link-local), so it's a connection and not a leftover.
fn has_address(n: &NetworkData) -> bool {
    n.ip_networks().iter().any(|ip| usable(&ip.addr))
}

fn usable(addr: &IpAddr) -> bool {
    match addr {
        IpAddr::V4(a) => !a.is_loopback() && !a.is_link_local() && !a.is_unspecified(),
        IpAddr::V6(a) => !a.is_loopback() && !a.is_unspecified() && (a.segments()[0] & 0xffc0) != 0xfe80,
    }
}

/// The IPv4 address of the adapter that has carried the most traffic (so a VPN or virtual switch that
/// sits idle doesn't win).
fn local_ip(networks: &Networks) -> Option<String> {
    networks
        .values()
        .filter_map(|n| {
            let ip = n.ip_networks().iter().map(|i| i.addr).find(|a| a.is_ipv4() && usable(a))?;
            Some((n.total_received(), ip))
        })
        .max_by_key(|(received, _)| *received)
        .map(|(_, ip)| ip.to_string())
}

/// Total CPU use as Task Manager counts it: Windows' "% Processor Utility" counter, which scales
/// busy time by clock speed. sysinfo's figure ("% Processor Time") ignores clock speed, so on a
/// laptop it read up to ~15 points off from Task Manager.
struct Cpu {
    query: PDH_HQUERY,
    counter: PDH_HCOUNTER,
    /// The clock speed as Task Manager shows it: the base frequency times the performance percent.
    frequency: PDH_HCOUNTER,
    performance: PDH_HCOUNTER,
}

impl Cpu {
    /// Opens the counter and takes the first reading; `read` reports use since then.
    fn open() -> Option<Self> {
        let mut query = PDH_HQUERY::default();
        let status = unsafe { PdhOpenQueryW(PCWSTR::null(), 0, &mut query) };
        if status != 0 {
            return report_pdh(status);
        }
        // Dropping it closes the query.
        let mut cpu = Self { query, counter: PDH_HCOUNTER::default(), frequency: PDH_HCOUNTER::default(), performance: PDH_HCOUNTER::default() };
        let path = w!(r"\Processor Information(_Total)\% Processor Utility");
        let status = unsafe { PdhAddEnglishCounterW(cpu.query, path, 0, &mut cpu.counter) };
        if status != 0 {
            return report_pdh(status);
        }
        // The speed is extra: without these counters it's just not shown.
        unsafe {
            PdhAddEnglishCounterW(cpu.query, w!(r"\Processor Information(_Total)\Processor Frequency"), 0, &mut cpu.frequency);
            PdhAddEnglishCounterW(cpu.query, w!(r"\Processor Information(_Total)\% Processor Performance"), 0, &mut cpu.performance);
            PdhCollectQueryData(cpu.query);
        }
        Some(cpu)
    }

    /// (use in percent, 0 to 100, as Task Manager caps it; clock speed in MHz) since the previous reading.
    fn read(&self) -> (Option<u8>, Option<u32>) {
        unsafe { PdhCollectQueryData(self.query) };
        let used = value(self.counter).map(|v| v.clamp(0.0, 100.0).round() as u8);
        let mhz = value(self.frequency).zip(value(self.performance)).map(|(f, p)| (f * p / 100.0).round() as u32);
        (used, mhz)
    }
}

fn value(counter: PDH_HCOUNTER) -> Option<f64> {
    let mut value = PDH_FMT_COUNTERVALUE::default();
    let status = unsafe { PdhGetFormattedCounterValue(counter, PDH_FMT_DOUBLE, None, &mut value) };
    (status == 0 && value.CStatus == 0).then_some(unsafe { value.Anonymous.doubleValue })
}

impl Drop for Cpu {
    fn drop(&mut self) {
        unsafe { PdhCloseQuery(self.query) };
    }
}

fn report_pdh<T>(status: u32) -> Option<T> {
    eprintln!("system-monitor: couldn't open the CPU counter (PDH error {status:#x})");
    None
}

pub(crate) fn battery() -> Option<Battery> {
    let mut status = SYSTEM_POWER_STATUS::default();
    unsafe { GetSystemPowerStatus(&mut status) }.ok()?;
    // BatteryFlag 128: no battery; 255: unknown. BatteryLifePercent and BatteryLifeTime use 255 and
    // u32::MAX for "unknown".
    if status.BatteryFlag & 128 != 0 || status.BatteryFlag == 255 || status.BatteryLifePercent > 100 {
        return None;
    }
    Some(Battery {
        percent: status.BatteryLifePercent,
        charging: status.ACLineStatus == 1,
        seconds_left: (status.BatteryLifeTime != u32::MAX).then_some(status.BatteryLifeTime),
    })
}

/// (free, total) MiB on the drive Windows is installed on.
fn system_drive_space() -> (u32, u32) {
    let drive = std::env::var("SystemDrive").unwrap_or_else(|_| "C:".into());
    let (mut free, mut total) = (0u64, 0u64);
    let root = HSTRING::from(format!("{drive}\\"));
    if let Err(e) = unsafe { GetDiskFreeSpaceExW(&root, Some(&mut free), Some(&mut total), None) } {
        eprintln!("system-monitor: couldn't read free space on {drive}: {e}");
    }
    (mib(free), mib(total))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::BoardTile;

    fn tile(id: &str, readings: Option<[Reading; 2]>) -> BoardTile {
        BoardTile { id: id.into(), w: 1, h: 1, x: 0, y: 0, readings }
    }

    #[test]
    fn the_gpu_is_only_measured_when_shown() {
        let mut s = Settings::default();
        assert!(!wants_gpu(&s), "not by default");
        s.pages = Some(vec![vec![tile("system-monitor/cpu", None)], vec![tile("system-monitor/monitor", Some([Reading::Cpu, Reading::Memory]))]]);
        assert!(!wants_gpu(&s));
        s.pages.as_mut().unwrap()[1][0].readings = Some([Reading::Cpu, Reading::Gpu]);
        assert!(wants_gpu(&s), "a wide tile on page 2 reads it");
        s.pages = Some(vec![vec![tile("system-monitor/gpu", None)]]);
        assert!(wants_gpu(&s), "a GPU tile");
        s.pages = None;
        s.monitor_readings = [Reading::Gpu, Reading::Memory];
        assert!(wants_gpu(&s), "the default pair, for wide tiles without their own");
    }
}
