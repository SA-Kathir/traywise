//! User settings, saved as JSON in `%APPDATA%\Traywise\settings.json`.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};
use tauri_plugin_autostart::ManagerExt;

use crate::module::{ModuleId, Modules};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    // shortcut: an unknown module id (one removed in a later version) makes the whole file
    // invalid, so it falls back to defaults; skip unknown ids instead once modules get removed.
    pub enabled_modules: Vec<ModuleId>,
    pub look: Look,
    /// Liquid: no panel background, so the cards float over the desktop.
    pub transparent: bool,
    /// Liquid: cards in one even hue, instead of lighter at the top left fading darker.
    pub single_hue: bool,
    pub preset: Preset,
    /// Global shortcut that mutes or unmutes every microphone (Audio devices), like "Ctrl+Alt+M".
    pub mic_shortcut: String,
    /// Whether Traywise asks GitHub for a newer version (at startup and about once a day).
    pub check_updates: bool,
    /// Whether first-run setup (pick a look, then modules) is done.
    pub onboarded: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            enabled_modules: vec![ModuleId::VolumeMixer],
            look: Look::Liquid,
            transparent: false,
            single_hue: false,
            preset: Preset::Ocean,
            mic_shortcut: "Ctrl+Alt+M".into(),
            check_updates: true,
            onboarded: false,
        }
    }
}

/// How the panel is drawn. The styles are in `src/shell/liquid.css`.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum Look {
    /// Glass cards over a colour preset's background.
    Liquid,
    /// Flat, like Windows 11's own flyouts, following Windows' light or dark mode.
    Windows,
}

/// Background colour preset (Liquid look): one muted hue each. The colours are in `src/shell/liquid.css`.
/// (An old file's `motion` field, from when the background moved, is ignored.)
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum Preset {
    Ocean,
    Graphite,
    Forest,
    Plum,
    Ember,
    Rose,
}

pub struct SettingsState {
    path: PathBuf,
    current: Mutex<Settings>,
}

impl SettingsState {
    pub fn load(app: &AppHandle) -> tauri::Result<Self> {
        let path = app.path().config_dir()?.join("Traywise").join("settings.json");
        Ok(Self { current: Mutex::new(read(&path)), path })
    }

    pub fn get(&self) -> Settings {
        self.current.lock().unwrap().clone()
    }
}

/// No file yet: defaults. A field missing from the file (say, one added in a later version)
/// gets its default. An unreadable file: defaults too, and the bad file is kept as
/// `settings.invalid.json` so the next save doesn't silently destroy it.
fn read(path: &Path) -> Settings {
    let Ok(text) = std::fs::read_to_string(path) else { return Settings::default() };
    // Lay the file's fields over the defaults (rather than `#[serde(default)]`, which would make
    // every field optional in the generated TypeScript type).
    let merged = serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(&text).and_then(|file| {
        let serde_json::Value::Object(mut fields) = serde_json::to_value(Settings::default())? else {
            unreachable!("Settings serializes to a JSON object")
        };
        fields.extend(file);
        serde_json::from_value(serde_json::Value::Object(fields))
    });
    merged.unwrap_or_else(|e| {
        eprintln!("{} is invalid ({e}); using default settings", path.display());
        let _ = std::fs::rename(path, path.with_file_name("settings.invalid.json"));
        Settings::default()
    })
}

/// Writes through a temporary file, so a crash mid-write can't leave a half-written settings.json.
fn write(path: &Path, settings: &Settings) -> std::io::Result<()> {
    std::fs::create_dir_all(path.parent().expect("settings path has a folder"))?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_string_pretty(settings)?)?;
    std::fs::rename(tmp, path)
}

#[tauri::command]
#[specta::specta]
pub fn get_settings(state: State<SettingsState>) -> Settings {
    state.get()
}

/// Saves the settings, then starts or stops modules to match.
#[tauri::command]
#[specta::specta]
pub fn set_settings(
    app: AppHandle,
    settings: Settings,
    state: State<SettingsState>,
    modules: State<Modules>,
) -> Result<(), String> {
    write(&state.path, &settings).map_err(|e| format!("couldn't save settings: {e}"))?;
    modules.apply(&settings);
    *state.current.lock().unwrap() = settings;
    // After saving: the window's frame and size follow the saved settings.
    crate::shell::panel::refresh_frame(&app);
    Ok(())
}

/// Whether Traywise starts with Windows. Windows keeps this (in the registry's Run list), not settings.json.
#[tauri::command]
#[specta::specta]
pub fn get_autostart(app: AppHandle) -> Result<bool, String> {
    app.autolaunch().is_enabled().map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
pub fn set_autostart(app: AppHandle, enabled: bool) -> Result<(), String> {
    let autolaunch = app.autolaunch();
    if enabled { autolaunch.enable() } else { autolaunch.disable() }.map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_valid_and_invalid_files() {
        let dir = std::env::temp_dir().join(format!("traywise-settings-test-{}", std::process::id()));
        let path = dir.join("settings.json");
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(read(&path), Settings::default(), "no file yet");

        let custom = Settings {
            enabled_modules: vec![],
            look: Look::Windows,
            transparent: true,
            single_hue: true,
            preset: Preset::Graphite,
            mic_shortcut: "Ctrl+Shift+F9".into(),
            check_updates: false,
            onboarded: true,
        };
        write(&path, &custom).unwrap();
        assert_eq!(read(&path), custom, "saved and read back");

        std::fs::write(&path, r#"{ "preset": "graphite" }"#).unwrap();
        assert_eq!(read(&path), Settings { preset: Preset::Graphite, ..Settings::default() }, "missing fields");

        std::fs::write(&path, r#"{ "preset": "forest", "motion": "onOpen" }"#).unwrap();
        assert_eq!(read(&path), Settings { preset: Preset::Forest, ..Settings::default() }, "old field ignored");

        std::fs::write(&path, "{ not json").unwrap();
        assert_eq!(read(&path), Settings::default(), "invalid file");
        assert_eq!(std::fs::read_to_string(dir.join("settings.invalid.json")).unwrap(), "{ not json", "kept aside");

        let _ = std::fs::remove_dir_all(&dir);
    }
}
