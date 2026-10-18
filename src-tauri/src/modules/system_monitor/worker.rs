//! The system-monitor thread. Sleeps while the panel is closed; samples once a second while it's open.

use std::collections::VecDeque;
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};

use sysinfo::{Networks, System};
use tauri::{AppHandle, Manager};
use tauri_specta::Event;
use windows::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;
use windows::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};
use windows::core::HSTRING;

use super::{Battery, Msg, Point, Shared, SystemSample, SystemSampled, mib, per_second, push_history};

pub fn run(app: AppHandle, rx: Receiver<Msg>) {
    let mut sampler = Sampler {
        system: System::new(),
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
    system: System,
    networks: Networks,
    /// When CPU and network counters were last read; rates are measured from here.
    last: Instant,
    history: VecDeque<Point>,
}

impl Sampler {
    /// The panel just opened: start the CPU and network counters, and show what doesn't need them.
    fn begin(&mut self) -> SystemSample {
        self.system.refresh_cpu_usage();
        self.networks.refresh(true);
        self.last = Instant::now();
        self.history.clear();
        self.read(None, None, None)
    }

    fn sample(&mut self) -> SystemSample {
        self.system.refresh_cpu_usage();
        self.networks.refresh(true);
        let elapsed = self.last.elapsed();
        self.last = Instant::now();
        // shortcut: virtual adapters (VPNs, Hyper-V) can count the same traffic twice; skip them by
        // type if the numbers look doubled.
        let (received, sent) = self.networks.values().fold((0, 0), |(r, s), n| (r + n.received(), s + n.transmitted()));
        let cpu = self.system.global_cpu_usage().clamp(0.0, 100.0).round() as u8;
        let (download, upload) = (per_second(received, elapsed), per_second(sent, elapsed));
        push_history(&mut self.history, Point { cpu, download, upload });
        self.read(Some(cpu), Some(download), Some(upload))
    }

    fn read(&mut self, cpu: Option<u8>, download: Option<u32>, upload: Option<u32>) -> SystemSample {
        self.system.refresh_memory();
        let (disk_free, disk_total) = system_drive_space();
        SystemSample {
            cpu,
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
