//! Update checks against the latest GitHub release (the endpoint and public key are in tauri.conf.json).
//! Checks at startup, then when the panel closes if a day has passed: no timers, nothing while idle.
//! Nothing installs without the user clicking Install.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use tauri::{AppHandle, Manager, State};
use tauri_plugin_updater::{Update, UpdaterExt};
use tauri_specta::Event;

use crate::settings::SettingsState;

const INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);

#[derive(Default)]
pub struct UpdateState {
    last_check: Mutex<Option<Instant>>,
    found: Mutex<Option<Update>>,
}

/// Sent to the UI when a newer version is found. The payload is its version, like "0.2.0".
#[derive(Clone, serde::Serialize, specta::Type, tauri_specta::Event)]
pub struct UpdateAvailable(pub String);

/// Checks in the background unless switched off, already found, or checked within the last day.
pub fn check(app: &AppHandle) {
    let state = app.state::<UpdateState>();
    if !app.state::<SettingsState>().get().check_updates || state.found.lock().unwrap().is_some() {
        return;
    }
    {
        let mut last = state.last_check.lock().unwrap();
        if last.is_some_and(|t| t.elapsed() < INTERVAL) {
            return;
        }
        *last = Some(Instant::now());
    }
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let update = match app.updater() {
            Ok(updater) => updater.check().await,
            Err(e) => Err(e),
        };
        match update {
            Ok(Some(update)) => {
                let _ = UpdateAvailable(update.version.clone()).emit(&app);
                *app.state::<UpdateState>().found.lock().unwrap() = Some(update);
            }
            Ok(None) => {}
            Err(e) => {
                eprintln!("update check failed: {e}");
                // Offline, say: retry the next time the panel closes instead of a day later.
                *app.state::<UpdateState>().last_check.lock().unwrap() = None;
            }
        }
    });
}

/// The version of an update that's been found, if any.
#[tauri::command]
#[specta::specta]
pub fn update_available(state: State<UpdateState>) -> Option<String> {
    state.found.lock().unwrap().as_ref().map(|u| u.version.clone())
}

/// Downloads and runs the new installer. On success Traywise exits, and the installer restarts it.
#[tauri::command]
#[specta::specta]
pub async fn update_install(state: State<'_, UpdateState>) -> Result<(), String> {
    let update = state.found.lock().unwrap().clone().ok_or("no update found")?;
    update.download_and_install(|_, _| {}, || {}).await.map_err(|e| e.to_string())
}
