//! Audio devices: pick the output device every app plays through, and mute every microphone at
//! once, from the panel or with a global shortcut.
//!
//! One background thread (`worker.rs`) owns the Windows audio objects. It sleeps until Windows
//! reports a device change or the UI asks for one, so it costs nothing while nothing happens.

mod policy_config;
mod worker;

use std::sync::Mutex;
use std::sync::mpsc::{Sender, channel};
use std::thread::JoinHandle;

use serde::Serialize;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

use crate::module::{Module, ModuleId};
use crate::settings::Settings;

/// Everything the panel shows.
#[derive(Clone, Debug, Default, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AudioDevicesState {
    pub outputs: Vec<OutputDevice>,
    /// Whether every microphone is muted. None when there are no microphones.
    pub mics_muted: Option<bool>,
    /// The mute-all shortcut. None until it has been set up.
    pub shortcut: Option<ShortcutStatus>,
}

#[derive(Clone, Debug, PartialEq, Serialize, specta::Type)]
pub struct ShortcutStatus {
    /// Like "Ctrl+Alt+M".
    pub keys: String,
    /// Why it couldn't be set up, for example another app already uses it.
    pub error: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct OutputDevice {
    /// Windows' id for the device. Stays the same across restarts.
    pub id: String,
    pub name: String,
    /// The one apps play through.
    pub is_default: bool,
}

/// Sent to the UI whenever a device is plugged in, removed, or becomes the default.
#[derive(Clone, Serialize, specta::Type, tauri_specta::Event)]
pub struct AudioDevicesChanged(pub AudioDevicesState);

enum Msg {
    SetOutput(String),
    MuteMics(bool),
    ToggleMics,
    Shortcut(ShortcutStatus),
    /// Something changed in Windows: re-read and publish.
    Refresh,
    Stop,
}

/// Shared between the module, its worker thread and the commands.
#[derive(Default)]
pub struct Shared {
    /// Only set while the module is running.
    tx: Mutex<Option<Sender<Msg>>>,
    state: Mutex<AudioDevicesState>,
}

pub struct AudioDevices {
    app: AppHandle,
    worker: Option<JoinHandle<()>>,
    /// The registered mute-all shortcut.
    shortcut: Option<String>,
}

impl AudioDevices {
    pub fn new(app: AppHandle) -> Self {
        app.manage(Shared::default());
        Self { app, worker: None, shortcut: None }
    }

    fn unregister_shortcut(&mut self) {
        if let Some(keys) = self.shortcut.take() {
            let _ = self.app.global_shortcut().unregister(keys.as_str());
        }
    }
}

impl Module for AudioDevices {
    fn id(&self) -> ModuleId {
        ModuleId::AudioDevices
    }

    fn start(&mut self) -> Result<(), String> {
        let (tx, rx) = channel();
        *self.app.state::<Shared>().tx.lock().unwrap() = Some(tx.clone());
        let app = self.app.clone();
        let worker = std::thread::Builder::new()
            .name("audio-devices".into())
            .spawn(move || worker::run(app, tx, rx))
            .map_err(|e| e.to_string())?;
        self.worker = Some(worker);
        println!("audio-devices started");
        Ok(())
    }

    fn stop(&mut self) {
        self.unregister_shortcut();
        let shared = self.app.state::<Shared>();
        if let Some(tx) = shared.tx.lock().unwrap().take() {
            let _ = tx.send(Msg::Stop);
        }
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
        *shared.state.lock().unwrap() = AudioDevicesState::default();
        println!("audio-devices stopped");
    }

    fn apply_settings(&mut self, settings: &Settings) {
        let keys = &settings.mic_shortcut;
        if self.shortcut.as_ref() == Some(keys) {
            return;
        }
        self.unregister_shortcut();
        let registered = self.app.global_shortcut().on_shortcut(keys.as_str(), |app, _, event| {
            if event.state == ShortcutState::Pressed {
                let _ = send(&app.state::<Shared>(), Msg::ToggleMics);
            }
        });
        let error = registered.err().map(|e| e.to_string());
        if error.is_none() {
            self.shortcut = Some(keys.clone());
        }
        let _ = send(&self.app.state::<Shared>(), Msg::Shortcut(ShortcutStatus { keys: keys.clone(), error }));
    }
}

fn send(shared: &Shared, msg: Msg) -> Result<(), String> {
    let tx = shared.tx.lock().unwrap();
    tx.as_ref().ok_or("audio devices is off")?.send(msg).map_err(|e| e.to_string())
}

/// The latest snapshot. Empty while the module is off.
#[tauri::command]
#[specta::specta]
pub fn audio_devices_state(shared: State<Shared>) -> AudioDevicesState {
    shared.state.lock().unwrap().clone()
}

/// Makes this output the default for everything: apps, games, and calls.
#[tauri::command]
#[specta::specta]
pub fn audio_devices_set_output(shared: State<Shared>, id: String) -> Result<(), String> {
    send(&shared, Msg::SetOutput(id))
}

/// Mutes or unmutes every microphone.
#[tauri::command]
#[specta::specta]
pub fn audio_devices_mute_mics(shared: State<Shared>, muted: bool) -> Result<(), String> {
    send(&shared, Msg::MuteMics(muted))
}
