//! The system-monitor thread. Sleeps while the panel is closed; samples once a second while it's open.

use std::collections::VecDeque;
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};

use sysinfo::{Networks, System};
use tauri::{AppHandle, Manager};
use tauri_specta::Event;
use windows::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;
use windows::Win32::System::Performance::{
    PDH_FMT_COUNTERVALUE, PDH_FMT_DOUBLE, PDH_HCOUNTER, PDH_HQUERY, PdhAddEnglishCounterW, PdhCloseQuery,
    PdhCollectQueryData, PdhGetFormattedCounterValue, PdhOpenQueryW,
};
use windows::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};
use windows::core::{HSTRING, PCWSTR, w};

use super::{Battery, Msg, Point, Shared, SystemSample, SystemSampled, mib, per_second, push_history};

pub fn run(app: AppHandle, rx: Receiver<Msg>) {
    let mut sampler = Sampler {
        system: System::new(),
        cpu: None,
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
                sampler.begin()
            }
            Ok(Msg::PanelVisible(false)) => {
                visible = false;
                sampler.cpu = None; // closes the counter: nothing is measured while hidden
                sampler.history.clear();
                SystemSample::default()
            }
            Err(RecvTimeoutError::Timeout) => sampler.sample(),
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

    fn sample(&mut self) -> SystemSample {
        let (cpu, cpu_mhz) = self.cpu.as_ref().map_or((None, None), Cpu::read);
        self.networks.refresh(true);
        let elapsed = self.last.elapsed();
        self.last = Instant::now();
        // shortcut: virtual adapters (VPNs, Hyper-V) can count the same traffic twice; skip them by
        // type if the numbers look doubled.
        let (received, sent) = self.networks.values().fold((0, 0), |(r, s), n| (r + n.received(), s + n.transmitted()));
        let (download, upload) = (per_second(received, elapsed), per_second(sent, elapsed));
        let mut sample = self.read(cpu, Some(download), Some(upload));
        sample.cpu_mhz = cpu_mhz;
        let memory = (u64::from(sample.memory_used) * 100 / u64::from(sample.memory_total.max(1))) as u8;
        push_history(&mut self.history, Point { cpu: cpu.unwrap_or(0), memory, download, upload });
        sample.history = self.history.clone();
        sample
    }

    fn read(&mut self, cpu: Option<u8>, download: Option<u32>, upload: Option<u32>) -> SystemSample {
        self.system.refresh_memory();
        let (disk_free, disk_total) = system_drive_space();
        SystemSample {
            cpu,
            cpu_mhz: None,
            memory_used: mib(self.system.used_memory()),
            memory_total: mib(self.system.total_memory()),
            download,
            upload,
            battery: battery(),
            disk_free,
            disk_total,
            history: self.history.clone(),
        }
    }
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

fn battery() -> Option<Battery> {
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
