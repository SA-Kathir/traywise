//! Keep awake: stops Windows from going to sleep, and optionally the display from turning off.
//!
//! One background thread (`worker.rs`) holds a Windows power request, the same thing video
//! players use. It sleeps until the UI changes the mode, the PC switches between mains and
//! battery, or a timer runs out. Windows drops the request by itself if Traywise quits or crashes.

mod worker;

use std::sync::Mutex;
use std::sync::mpsc::{Sender, channel};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};

use crate::module::{Module, ModuleId};

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum Mode {
    #[default]
    Off,
    /// Until turned off.
    On,
    /// For this many minutes, then off.
    Minutes(u32),
    /// Only while the PC runs on mains power.
    WhilePluggedIn,
}

/// Everything the panel shows.
#[derive(Clone, Debug, Default, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct KeepAwakeState {
    pub mode: Mode,
    /// Also keep the display on.
    pub display: bool,
    /// Whether Windows is being kept awake right now. False in `whilePluggedIn` mode on battery.
    pub active: bool,
    /// Timed mode only: when it turns off, in milliseconds since 1970 (like JavaScript's `Date.now()`).
    pub ends_at: Option<f64>,
    /// A Modern Standby PC: Windows goes to standby when the screen turns off, whatever apps ask,
    /// so only keeping the display on keeps it awake.
    pub screen_off_sleeps: bool,
}

/// Sent to the UI whenever the state changes.
#[derive(Clone, Serialize, specta::Type, tauri_specta::Event)]
pub struct KeepAwakeChanged(pub KeepAwakeState);

enum Msg {
    SetMode(Mode),
    SetDisplay(bool),
    /// Mains or battery changed: re-check.
    PowerChanged,
    Stop,
}

/// What the user asked for, and since when. Kept apart from the Windows calls so it can be tested.
#[derive(Clone, Copy)]
struct Wish {
    mode: Mode,
    display: bool,
    since: Instant,
}

impl Wish {
    /// When a timed mode runs out.
    fn deadline(&self) -> Option<Instant> {
        match self.mode {
            Mode::Minutes(minutes) => Some(self.since + Duration::from_secs(u64::from(minutes) * 60)),
            _ => None,
        }
    }

    /// (keep the system awake, keep the display on)
    fn wanted(&self, plugged_in: bool) -> (bool, bool) {
        let system = match self.mode {
            Mode::Off => false,
            Mode::On | Mode::Minutes(_) => true,
            Mode::WhilePluggedIn => plugged_in,
        };
        (system, system && self.display)
    }
}

/// Shared between the module, its worker thread and the commands.
#[derive(Default)]
pub struct Shared {
    /// Only set while the module is running.
    tx: Mutex<Option<Sender<Msg>>>,
    state: Mutex<KeepAwakeState>,
}

pub struct KeepAwake {
    app: AppHandle,
    worker: Option<JoinHandle<()>>,
}

impl KeepAwake {
    pub fn new(app: AppHandle) -> Self {
        app.manage(Shared::default());
        Self { app, worker: None }
    }
}

impl Module for KeepAwake {
    fn id(&self) -> ModuleId {
        ModuleId::KeepAwake
    }

    fn start(&mut self) -> Result<(), String> {
        let (tx, rx) = channel();
        *self.app.state::<Shared>().tx.lock().unwrap() = Some(tx.clone());
        let app = self.app.clone();
        let worker = std::thread::Builder::new()
            .name("keep-awake".into())
            .spawn(move || worker::run(app, tx, rx))
            .map_err(|e| e.to_string())?;
        self.worker = Some(worker);
        println!("keep-awake started");
        Ok(())
    }

    fn stop(&mut self) {
        let shared = self.app.state::<Shared>();
        if let Some(tx) = shared.tx.lock().unwrap().take() {
            let _ = tx.send(Msg::Stop);
        }
        if let Some(worker) = self.worker.take() {
            let _ = worker.join(); // the worker releases the power request before it ends
        }
        *shared.state.lock().unwrap() = KeepAwakeState::default();
        println!("keep-awake stopped");
    }
}

fn send(shared: &Shared, msg: Msg) -> Result<(), String> {
    let tx = shared.tx.lock().unwrap();
    tx.as_ref().ok_or("keep awake is off")?.send(msg).map_err(|e| e.to_string())
}

/// The latest state. Default (off) while the module is switched off.
#[tauri::command]
#[specta::specta]
pub fn keep_awake_state(shared: State<Shared>) -> KeepAwakeState {
    shared.state.lock().unwrap().clone()
}

/// Picking a timed mode (again) restarts its timer.
#[tauri::command]
#[specta::specta]
pub fn keep_awake_set_mode(shared: State<Shared>, mode: Mode) -> Result<(), String> {
    send(&shared, Msg::SetMode(mode))
}

#[tauri::command]
#[specta::specta]
pub fn keep_awake_set_display(shared: State<Shared>, display: bool) -> Result<(), String> {
    send(&shared, Msg::SetDisplay(display))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wish(mode: Mode, display: bool) -> Wish {
        Wish { mode, display, since: Instant::now() }
    }

    #[test]
    fn modes() {
        assert_eq!(wish(Mode::Off, true).wanted(true), (false, false));
        assert_eq!(wish(Mode::On, false).wanted(false), (true, false));
        assert_eq!(wish(Mode::On, true).wanted(false), (true, true));
        assert_eq!(wish(Mode::Minutes(30), true).wanted(false), (true, true));
        assert_eq!(wish(Mode::WhilePluggedIn, true).wanted(true), (true, true));
        assert_eq!(wish(Mode::WhilePluggedIn, true).wanted(false), (false, false), "on battery");
    }

    #[test]
    fn only_timed_modes_end() {
        let w = wish(Mode::Minutes(90), false);
        assert_eq!(w.deadline(), Some(w.since + Duration::from_secs(90 * 60)));
        assert_eq!(wish(Mode::On, false).deadline(), None);
        assert_eq!(wish(Mode::WhilePluggedIn, false).deadline(), None);
        assert_eq!(wish(Mode::Off, false).deadline(), None);
    }
}
