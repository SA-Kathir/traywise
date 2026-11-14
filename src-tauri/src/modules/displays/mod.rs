//! Displays: a brightness slider per screen, and how screens are used together (like Win+P).
//!
//! One thread (`worker.rs`) that sleeps while the panel is closed. Each time the panel opens it
//! looks at the screens again (so a monitor plugged in since is found without a listener) and
//! reads their brightness: the laptop's own screen through WMI, external monitors over DDC/CI,
//! which is slow (tens of ms to a second per monitor), hence the thread. While the panel is
//! closed it holds nothing open.

mod worker;

pub(crate) use worker::external_names;

use std::sync::Mutex;
use std::sync::mpsc::{Sender, channel};
use std::thread::JoinHandle;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};

use crate::module::{Module, ModuleId};

/// The four choices of Win+P.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum Project {
    PcOnly,
    Duplicate,
    Extend,
    SecondOnly,
}

#[derive(Clone, Debug, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Display {
    /// Stable while the screen stays connected.
    pub id: String,
    pub name: String,
    /// The laptop's own screen.
    pub internal: bool,
    /// Percent. None when it can't be changed from here (a monitor without DDC/CI, or with it off).
    pub brightness: Option<u8>,
    /// The screen's refresh rate and the ones it offers at its current resolution. None if Windows
    /// won't say.
    pub refresh: Option<Refresh>,
}

#[derive(Clone, Debug, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Refresh {
    /// Hz.
    pub now: u32,
    /// Hz, lowest first. Always includes `now`.
    pub rates: Vec<u32>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct DisplaysState {
    /// The laptop's screen first. Screens that are off (like the laptop's in "Second screen only")
    /// aren't listed.
    pub displays: Vec<Display>,
    /// None while only one screen is connected: there's nothing to choose.
    pub project: Option<Project>,
}

/// Sent when the panel opens (once the screens have been read) and after the Project choice changes.
#[derive(Clone, Serialize, specta::Type, tauri_specta::Event)]
pub struct DisplaysChanged(pub DisplaysState);

#[derive(Debug, PartialEq)]
enum Msg {
    PanelVisible(bool),
    Brightness(String, u8),
    /// Screen id, Hz.
    Refresh(String, u32),
    Project(Project),
    Stop,
}

/// Dragging a slider sends many values, and a DDC/CI monitor takes ~50 ms for each: keep only
/// the latest value per screen, where its first one was, so the monitor catches up at once.
fn coalesce(msgs: Vec<Msg>) -> Vec<Msg> {
    let mut out: Vec<Msg> = Vec::with_capacity(msgs.len());
    for msg in msgs {
        if let Msg::Brightness(id, value) = &msg
            && let Some(Msg::Brightness(_, earlier)) = out.iter_mut().find(|m| matches!(m, Msg::Brightness(i, _) if i == id))
        {
            *earlier = *value;
            continue;
        }
        out.push(msg);
    }
    out
}

/// A DDC/CI reading as a percent: monitors report their own maximum, usually 100.
fn to_percent(value: u32, max: u32) -> u8 {
    if max == 0 { 0 } else { (u64::from(value.min(max)) * 100 / u64::from(max)) as u8 }
}

fn from_percent(percent: u8, max: u32) -> u32 {
    ((u64::from(percent.min(100)) * u64::from(max) + 50) / 100) as u32
}

/// Shared between the module, its worker thread and the commands.
#[derive(Default)]
pub struct Shared {
    /// Only set while the module is running.
    tx: Mutex<Option<Sender<Msg>>>,
    state: Mutex<DisplaysState>,
}

pub struct Displays {
    app: AppHandle,
    worker: Option<JoinHandle<()>>,
}

impl Displays {
    pub fn new(app: AppHandle) -> Self {
        app.manage(Shared::default());
        Self { app, worker: None }
    }
}

impl Module for Displays {
    fn id(&self) -> ModuleId {
        ModuleId::Displays
    }

    fn start(&mut self) -> Result<(), String> {
        let (tx, rx) = channel();
        *self.app.state::<Shared>().tx.lock().unwrap() = Some(tx);
        let app = self.app.clone();
        let worker = std::thread::Builder::new()
            .name("displays".into())
            .spawn(move || worker::run(app, rx))
            .map_err(|e| e.to_string())?;
        self.worker = Some(worker);
        println!("displays started");
        Ok(())
    }

    fn stop(&mut self) {
        let shared = self.app.state::<Shared>();
        if let Some(tx) = shared.tx.lock().unwrap().take() {
            let _ = tx.send(Msg::Stop);
        }
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
        *shared.state.lock().unwrap() = DisplaysState::default();
        println!("displays stopped");
    }

    fn panel_visible(&mut self, visible: bool) {
        let _ = send(&self.app.state::<Shared>(), Msg::PanelVisible(visible));
    }
}

fn send(shared: &Shared, msg: Msg) -> Result<(), String> {
    let tx = shared.tx.lock().unwrap();
    tx.as_ref().ok_or("displays is off")?.send(msg).map_err(|e| e.to_string())
}

/// The screens as last read. Empty while the module is off.
#[tauri::command]
#[specta::specta]
pub fn displays_state(shared: State<Shared>) -> DisplaysState {
    shared.state.lock().unwrap().clone()
}

/// Percent. Sent as the slider moves; the worker keeps up by skipping to the latest value.
#[tauri::command]
#[specta::specta]
pub fn displays_set_brightness(shared: State<Shared>, id: String, percent: u8) -> Result<(), String> {
    send(&shared, Msg::Brightness(id, percent))
}

/// Switches a screen to another refresh rate it offers, at the same resolution.
#[tauri::command]
#[specta::specta]
pub fn displays_set_refresh(shared: State<Shared>, id: String, hz: u32) -> Result<(), String> {
    send(&shared, Msg::Refresh(id, hz))
}

/// The rates a screen offers at its current mode: from every mode Windows lists
/// `(width, height, bits, hz)`, those that match `current`. Lowest first, no duplicates, and
/// "1 Hz" and "0 Hz" (Windows' "default") left out.
fn rates_at(current: (u32, u32, u32), modes: impl Iterator<Item = (u32, u32, u32, u32)>) -> Vec<u32> {
    let mut rates: Vec<u32> = modes.filter(|&(w, h, b, hz)| (w, h, b) == current && hz > 1).map(|m| m.3).collect();
    rates.sort_unstable();
    rates.dedup();
    rates
}

#[tauri::command]
#[specta::specta]
pub fn displays_set_project(shared: State<Shared>, project: Project) -> Result<(), String> {
    send(&shared, Msg::Project(project))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_the_latest_brightness_per_screen() {
        let b = |id: &str, v| Msg::Brightness(id.into(), v);
        let msgs = vec![b("a", 10), b("b", 50), b("a", 20), Msg::Project(Project::Extend), b("a", 30), b("b", 60)];
        assert_eq!(coalesce(msgs), vec![b("a", 30), b("b", 60), Msg::Project(Project::Extend)]);
        assert_eq!(coalesce(vec![Msg::Stop]), vec![Msg::Stop]);
    }

    #[test]
    fn rates_at_the_current_mode() {
        let modes = [(1920, 1080, 32, 60), (1920, 1080, 32, 144), (1920, 1080, 32, 60), (2560, 1440, 32, 240), (1920, 1080, 16, 75), (1920, 1080, 32, 1)];
        assert_eq!(rates_at((1920, 1080, 32), modes.into_iter()), vec![60, 144]);
        assert!(rates_at((800, 600, 32), modes.into_iter()).is_empty());
    }

    #[test]
    fn percents() {
        assert_eq!(to_percent(50, 100), 50);
        assert_eq!(to_percent(25, 50), 50);
        assert_eq!(to_percent(200, 100), 100, "a reading above the maximum");
        assert_eq!(to_percent(5, 0), 0, "a monitor reporting no maximum");
        assert_eq!(from_percent(50, 50), 25);
        assert_eq!(from_percent(33, 255), 84);
        assert_eq!(from_percent(150, 100), 100);
    }
}
