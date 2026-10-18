//! System monitor: CPU, memory, network, battery and free disk space.
//!
//! Sampled once a second, and only while the panel is open. While it's closed the thread sleeps
//! and nothing is measured, so hidden CPU use doesn't change with this module on.

mod worker;

use std::collections::VecDeque;
use std::sync::Mutex;
use std::sync::mpsc::{Sender, channel};
use std::thread::JoinHandle;
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Manager, State};

use crate::module::{Module, ModuleId};

/// How many one-second samples the graphs show.
const HISTORY: usize = 60;
/// Memory and disk sizes are sent in MiB: whole numbers fit in a u32 (a u64 can't cross to
/// TypeScript as a plain number, and a float arrives as `number | null`).
const MIB: u64 = 1024 * 1024;

/// Everything the panel shows. Sent whole every second while the panel is open.
#[derive(Clone, Debug, Default, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SystemSample {
    /// Percent of all cores together. None for the first second after opening.
    pub cpu: Option<u8>,
    /// MiB.
    pub memory_used: u32,
    pub memory_total: u32,
    /// Bytes a second, all network adapters together. None for the first second after opening.
    pub download: Option<u32>,
    pub upload: Option<u32>,
    /// None on a PC without a battery.
    pub battery: Option<Battery>,
    /// The drive Windows is on, usually C:. MiB.
    pub disk_free: u32,
    pub disk_total: u32,
    /// Up to the last 60 seconds, oldest first. Only kept while the panel is open.
    pub history: VecDeque<Point>,
}

#[derive(Clone, Debug, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Battery {
    pub percent: u8,
    pub charging: bool,
    /// Windows' estimate while on battery, when it has one.
    pub seconds_left: Option<u32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, specta::Type)]
pub struct Point {
    pub cpu: u8,
    /// Bytes a second.
    pub download: u32,
    pub upload: u32,
}

/// Sent to the UI every second while the panel is open.
#[derive(Clone, Serialize, specta::Type, tauri_specta::Event)]
pub struct SystemSampled(pub SystemSample);

enum Msg {
    PanelVisible(bool),
    Stop,
}

/// Shared between the module, its worker thread and the commands.
#[derive(Default)]
pub struct Shared {
    state: Mutex<SystemSample>,
}

pub struct SystemMonitor {
    app: AppHandle,
    tx: Option<Sender<Msg>>,
    worker: Option<JoinHandle<()>>,
}

impl SystemMonitor {
    pub fn new(app: AppHandle) -> Self {
        app.manage(Shared::default());
        Self { app, tx: None, worker: None }
    }
}

impl Module for SystemMonitor {
    fn id(&self) -> ModuleId {
        ModuleId::SystemMonitor
    }

    fn start(&mut self) -> Result<(), String> {
        let (tx, rx) = channel();
        let app = self.app.clone();
        let worker = std::thread::Builder::new()
            .name("system-monitor".into())
            .spawn(move || worker::run(app, rx))
            .map_err(|e| e.to_string())?;
        self.tx = Some(tx);
        self.worker = Some(worker);
        println!("system-monitor started");
        Ok(())
    }

    fn stop(&mut self) {
        if let Some(tx) = self.tx.take() {
            let _ = tx.send(Msg::Stop);
        }
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
        *self.app.state::<Shared>().state.lock().unwrap() = SystemSample::default();
        println!("system-monitor stopped");
    }

    fn panel_visible(&mut self, visible: bool) {
        if let Some(tx) = &self.tx {
            let _ = tx.send(Msg::PanelVisible(visible));
        }
    }
}

/// The latest sample. Empty while the module is off or the panel is closed.
#[tauri::command]
#[specta::specta]
pub fn system_monitor_state(shared: State<Shared>) -> SystemSample {
    shared.state.lock().unwrap().clone()
}

/// Bytes counted over `elapsed`, as bytes a second.
fn per_second(bytes: u64, elapsed: Duration) -> u32 {
    let secs = elapsed.as_secs_f64();
    if secs > 0.0 { (bytes as f64 / secs).round().min(u32::MAX as f64) as u32 } else { 0 }
}

fn mib(bytes: u64) -> u32 {
    u32::try_from(bytes / MIB).unwrap_or(u32::MAX)
}

/// Adds a point, dropping the oldest once there are 60.
fn push_history(history: &mut VecDeque<Point>, point: Point) {
    if history.len() == HISTORY {
        history.pop_front();
    }
    history.push_back(point);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rates() {
        assert_eq!(per_second(1000, Duration::from_secs(1)), 1000);
        assert_eq!(per_second(1000, Duration::from_millis(500)), 2000);
        assert_eq!(per_second(1000, Duration::from_millis(1003)), 997, "a late tick");
        assert_eq!(per_second(1000, Duration::ZERO), 0);
        assert_eq!(per_second(u64::MAX, Duration::from_secs(1)), u32::MAX);
        assert_eq!(mib(3 * MIB + 5), 3);
    }

    #[test]
    fn history_keeps_the_last_60() {
        let mut history = VecDeque::new();
        for cpu in 0..=70 {
            push_history(&mut history, Point { cpu, download: 0, upload: 0 });
        }
        assert_eq!(history.len(), 60);
        assert_eq!(history.front().unwrap().cpu, 11);
        assert_eq!(history.back().unwrap().cpu, 70);
    }
}
