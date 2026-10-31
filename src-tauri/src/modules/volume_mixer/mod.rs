//! Volume mixer: master volume plus per-app volume and mute for the default output device.
//!
//! One background thread (`worker.rs`) owns every Windows audio object. It sleeps until Windows
//! reports a change or the UI asks for one, so it costs nothing while nothing happens.

mod app_info;
mod worker;

use std::sync::Mutex;
use std::sync::mpsc::{Sender, channel};
use std::thread::JoinHandle;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};

use crate::module::{Module, ModuleId};

/// Everything the panel shows. Sent whole on every change: it's a handful of rows.
#[derive(Clone, Debug, Default, PartialEq, Serialize, specta::Type)]
pub struct MixerState {
    /// None when there's no output device.
    pub master: Option<Level>,
    /// The output device's name, like "Headphones (Realtek(R) Audio)".
    pub device: Option<String>,
    pub apps: Vec<AppAudio>,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, specta::Type)]
pub struct Level {
    /// Percent, 0 to 100. Whole numbers, like Windows' own mixer (and a float would reach
    /// TypeScript as `number | null`, since JSON has no NaN).
    pub volume: u8,
    pub muted: bool,
}

/// One app with audio on the default output device (all of its audio sessions together).
#[derive(Clone, Debug, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AppAudio {
    pub pid: u32,
    /// Windows' own sounds (notifications, errors), not an app.
    pub system_sounds: bool,
    pub name: String,
    /// The app's icon as a PNG data URL, if it has one.
    pub icon: Option<String>,
    /// Percent, 0 to 100.
    pub volume: u8,
    pub muted: bool,
}

/// What a volume or mute change applies to.
#[derive(Clone, Copy, Debug, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum Target {
    Master,
    /// An app, by process id.
    App(u32),
}

/// Sent to the UI whenever anything in the mixer changes, here or elsewhere in Windows.
#[derive(Clone, Serialize, specta::Type, tauri_specta::Event)]
pub struct MixerChanged(pub MixerState);

enum Msg {
    /// Percent, 0 to 100.
    SetVolume(Target, u8),
    SetMute(Target, bool),
    /// Something changed in Windows: re-read and publish.
    Refresh,
    /// The default output device changed: switch to the new one.
    DeviceChanged,
    Stop,
}

/// Shared between the module, its worker thread and the commands.
#[derive(Default)]
pub struct Shared {
    /// Only set while the module is running.
    tx: Mutex<Option<Sender<Msg>>>,
    state: Mutex<MixerState>,
}

pub struct VolumeMixer {
    app: AppHandle,
    worker: Option<JoinHandle<()>>,
}

impl VolumeMixer {
    pub fn new(app: AppHandle) -> Self {
        app.manage(Shared::default());
        Self { app, worker: None }
    }
}

impl Module for VolumeMixer {
    fn id(&self) -> ModuleId {
        ModuleId::VolumeMixer
    }

    fn start(&mut self) -> Result<(), String> {
        let (tx, rx) = channel();
        *self.app.state::<Shared>().tx.lock().unwrap() = Some(tx.clone());
        let app = self.app.clone();
        let worker = std::thread::Builder::new()
            .name("volume-mixer".into())
            .spawn(move || worker::run(app, tx, rx))
            .map_err(|e| e.to_string())?;
        self.worker = Some(worker);
        println!("volume-mixer started");
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
        *shared.state.lock().unwrap() = MixerState::default();
        println!("volume-mixer stopped");
    }
}

fn send(shared: &Shared, msg: Msg) -> Result<(), String> {
    let tx = shared.tx.lock().unwrap();
    tx.as_ref().ok_or("the volume mixer is off")?.send(msg).map_err(|e| e.to_string())
}

/// The latest snapshot. Empty while the module is off.
#[tauri::command]
#[specta::specta]
pub fn mixer_state(shared: State<Shared>) -> MixerState {
    shared.state.lock().unwrap().clone()
}

/// Sets a volume, in percent (0 to 100).
#[tauri::command]
#[specta::specta]
pub fn mixer_set_volume(shared: State<Shared>, target: Target, volume: u8) -> Result<(), String> {
    send(&shared, Msg::SetVolume(target, volume.min(100)))
}

#[tauri::command]
#[specta::specta]
pub fn mixer_set_mute(shared: State<Shared>, target: Target, muted: bool) -> Result<(), String> {
    send(&shared, Msg::SetMute(target, muted))
}
