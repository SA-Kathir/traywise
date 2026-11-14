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
    /// Dot only: no panel background, so the cards float over the desktop. (Tint had this too until 0.4; an old file may still say true.)
    pub transparent: bool,
    /// Tint (called Liquid in the code): cards in one even hue (the default), instead of lighter at the top left fading darker.
    pub single_hue: bool,
    pub preset: Preset,
    /// Dot: the accent colour.
    pub accent: Accent,
    /// Dot: a plain gradient behind the tiles instead of the dot grid.
    pub plain_background: bool,
    /// Global shortcut that mutes or unmutes every microphone (Audio devices), like "Ctrl+Alt+M".
    pub mic_shortcut: String,
    /// Audio devices: when the output you were using is unplugged or disconnects and Windows falls back to
    /// another (headphones out, speakers on), lower that one's volume to a quiet level if it's louder.
    pub lower_volume_on_unplug: bool,
    /// Global shortcut that opens or closes the panel, like "Ctrl+Alt+T".
    pub panel_shortcut: String,
    /// Whether Traywise asks GitHub for a newer version (at startup and about once a day).
    pub check_updates: bool,
    /// Whether first-run setup (pick a look, then modules) is done.
    pub onboarded: bool,
    /// The home board's pages, each with its own tiles. None until it's first changed: the UI shows
    /// its default board as one page. (A 0.2 file's single `board` becomes page 1 when read.)
    pub pages: Option<Vec<Vec<BoardTile>>>,
    /// Home board: tiles fill gaps (true), or stay where they're put (false).
    pub auto_arrange: bool,
    /// System monitor: the readings a 4×1 or 4×2 tile shows (left then right) until it's given its own.
    pub monitor_readings: [Reading; 2],
    /// Resting the mouse on the middle of the screen's left or right edge (`edge_side`) opens the panel.
    pub edge_open: bool,
    /// With `edge_open`, a small pill on the edge shows where.
    pub edge_handle: bool,
    /// With `edge_open`: which side of the screen it's on.
    pub edge_side: EdgeSide,
    /// With `edge_handle`: the pill is in the accent colour, not grey.
    pub edge_handle_accent: bool,
    /// Clock: a second time zone its wider tiles show, like "Europe/London". None for none.
    pub clock_zone: Option<String>,
    pub clock_style: ClockStyle,
    /// Focus: play a soft chime when a focus or rest phase ends.
    pub focus_chime: bool,
    /// The version whose "What's new" note was last shown (or that was first installed). None on a fresh install.
    pub last_seen_version: Option<String>,
    /// Focus: how its tiles draw the countdown.
    pub focus_style: FocusStyle,
    /// Focus: saved "focus / rest" lengths in minutes; the tile shows the first two, the first is the default.
    pub focus_presets: Vec<FocusPreset>,
    /// Focus: the long rest after the fourth round, in minutes.
    pub focus_long_rest: u32,
    /// Focus: start each phase by itself, instead of waiting for a tap.
    pub focus_auto_next: bool,
    /// Focus: switch Do not disturb on while focusing (and off when the phase ends).
    pub focus_dnd_focus: bool,
    /// Focus: keep Do not disturb on during rests too.
    pub focus_dnd_rest: bool,
}

/// How the Focus tiles draw the countdown.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum FocusStyle {
    /// A half ring of dots.
    Arc,
    /// An hourglass of dots.
    Hourglass,
}

/// A saved focus and rest length, in minutes.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct FocusPreset {
    pub focus: u32,
    pub rest: u32,
}

/// A reading the system monitor graphs.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum Reading {
    Cpu,
    Memory,
    Download,
    Upload,
    Gpu,
}

/// How the clock tile draws the time.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum ClockStyle {
    /// Dot-matrix digits.
    Digital,
    /// A round face of dots, with dot hands.
    Analogue,
}

/// A tile on the home board: `w` × `h` grid cells, with its top-left cell at column `x`, row `y`.
/// `id` is "<module id>/<kind>"; the kinds are the UI's (`tiles` in src/modules), so Rust doesn't check them.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct BoardTile {
    pub id: String,
    pub w: u32,
    pub h: u32,
    pub x: u32,
    pub y: u32,
    /// Only for the system monitor's wide tile: what its left and right sides show. None uses `monitor_readings`.
    #[serde(default)]
    pub readings: Option<[Reading; 2]>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            enabled_modules: vec![ModuleId::VolumeMixer],
            look: Look::Dot,
            transparent: false,
            single_hue: true,
            preset: Preset::Ocean,
            accent: Accent::Red,
            plain_background: true,
            mic_shortcut: "Ctrl+Alt+M".into(),
            lower_volume_on_unplug: false,
            panel_shortcut: "Ctrl+Alt+T".into(),
            check_updates: true,
            onboarded: false,
            pages: None,
            auto_arrange: true,
            monitor_readings: [Reading::Cpu, Reading::Memory],
            edge_open: false,
            edge_handle: false,
            edge_side: EdgeSide::Right,
            edge_handle_accent: false,
            clock_zone: None,
            clock_style: ClockStyle::Digital,
            focus_chime: true,
            last_seen_version: None,
            focus_style: FocusStyle::Arc,
            focus_presets: vec![FocusPreset { focus: 25, rest: 5 }, FocusPreset { focus: 50, rest: 10 }],
            focus_long_rest: 15,
            focus_auto_next: true,
            focus_dnd_focus: true,
            focus_dnd_rest: false,
        }
    }
}

/// Which side of the screen the panel opens from, when it opens from an edge.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum EdgeSide {
    Left,
    Right,
}

/// How the panel is drawn. The styles are in `src/shell/liquid.css`.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum Look {
    /// Monochrome tiles over a dot grid, one accent colour and dot-matrix numbers (docs/design-system.md).
    Dot,
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

/// Accent colour preset (Dot look). The colours are in `src/shell/dot.css`.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum Accent {
    Red,
    Amber,
    Violet,
    Blue,
    Green,
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
    let merged = serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(&text).and_then(|mut file| {
        // 0.2 had one board: it becomes page 1.
        if let Some(board) = file.remove("board").filter(|b| b.is_array()) {
            file.entry("pages").or_insert(serde_json::Value::Array(vec![board]));
        }
        // A tile's own readings must be exactly two; anything else is dropped (it then uses the default).
        if let Some(pages) = file.get_mut("pages").and_then(|p| p.as_array_mut()) {
            for tile in pages.iter_mut().filter_map(|p| p.as_array_mut()).flatten() {
                if tile.get("readings").is_some_and(|r| serde_json::from_value::<[Reading; 2]>(r.clone()).is_err()) {
                    tile["readings"] = serde_json::Value::Null;
                }
            }
        }
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
    // Before saving, so keys another app already uses are refused instead of silently not working.
    crate::shell::panel::set_shortcut(&app, &settings.panel_shortcut)?;
    write(&state.path, &settings).map_err(|e| format!("couldn't save settings: {e}"))?;
    modules.apply(&settings);
    crate::shell::edge::apply(&app, &settings);
    crate::shell::tray::apply_settings(&app, &settings);
    // Only the look decides the frame. Re-framing resizes the window, which visibly nudges the
    // panel, so don't do it for every other save (like each tile moved on the home board).
    let reframe = {
        let mut current = state.current.lock().unwrap();
        let reframe = current.look != settings.look || current.transparent != settings.transparent;
        *current = settings;
        reframe
    };
    // After saving: the window's frame and size follow the saved settings.
    if reframe {
        crate::shell::panel::refresh_frame(&app);
    }
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
            single_hue: false,
            preset: Preset::Graphite,
            accent: Accent::Blue,
            plain_background: true,
            mic_shortcut: "Ctrl+Shift+F9".into(),
            lower_volume_on_unplug: true,
            panel_shortcut: "Ctrl+Shift+F10".into(),
            check_updates: false,
            onboarded: true,
            pages: Some(vec![vec![BoardTile { id: "quick-toggles/dark".into(), w: 2, h: 1, x: 2, y: 0, readings: None }], vec![]]),
            auto_arrange: false,
            monitor_readings: [Reading::Upload, Reading::Cpu],
            edge_open: true,
            edge_handle: true,
            edge_side: EdgeSide::Left,
            edge_handle_accent: true,
            clock_zone: Some("Asia/Tokyo".into()),
            clock_style: ClockStyle::Analogue,
            focus_chime: false,
            last_seen_version: Some("0.3.0".into()),
            focus_style: FocusStyle::Hourglass,
            focus_presets: vec![FocusPreset { focus: 45, rest: 10 }],
            focus_long_rest: 20,
            focus_auto_next: false,
            focus_dnd_focus: false,
            focus_dnd_rest: true,
        };
        write(&path, &custom).unwrap();
        assert_eq!(read(&path), custom, "saved and read back");

        std::fs::write(&path, r#"{ "preset": "graphite" }"#).unwrap();
        assert_eq!(read(&path), Settings { preset: Preset::Graphite, ..Settings::default() }, "missing fields");

        std::fs::write(&path, r#"{ "preset": "forest", "motion": "onOpen" }"#).unwrap();
        assert_eq!(read(&path), Settings { preset: Preset::Forest, ..Settings::default() }, "old field ignored");

        // A 0.1.0 settings file: every newer setting takes its default, nothing is lost.
        std::fs::write(
            &path,
            r#"{ "enabledModules": ["volume-mixer", "keep-awake", "audio-devices", "system-monitor", "quick-toggles"],
                 "look": "liquid", "transparent": true, "singleHue": false, "preset": "plum",
                 "micShortcut": "Ctrl+Alt+M", "panelShortcut": "Ctrl+Alt+T", "checkUpdates": true, "onboarded": true }"#,
        )
        .unwrap();
        let upgraded = read(&path);
        assert_eq!(
            upgraded,
            Settings {
                enabled_modules: vec![ModuleId::VolumeMixer, ModuleId::KeepAwake, ModuleId::AudioDevices, ModuleId::SystemMonitor, ModuleId::QuickToggles],
                look: Look::Liquid,
                transparent: true,
                single_hue: false,
                preset: Preset::Plum,
                onboarded: true,
                ..Settings::default()
            },
            "0.1.0 file"
        );

        // A tile's own readings that aren't exactly two known ones are dropped, not fatal.
        std::fs::write(
            &path,
            r#"{ "pages": [[{ "id": "system-monitor/monitor", "w": 4, "h": 1, "x": 0, "y": 0, "readings": ["cpu", "memory", "upload"] }]] }"#,
        )
        .unwrap();
        assert_eq!(read(&path).pages.unwrap()[0][0].readings, None, "bad readings");

        // A 0.2 file's single board becomes page 1; a board never changed (null) stays the default.
        std::fs::write(
            &path,
            r#"{ "board": [{ "id": "wifi/wifi", "w": 1, "h": 1, "x": 3, "y": 0 }, { "id": "clock/clock", "w": 2, "h": 1, "x": 0, "y": 1 }] }"#,
        )
        .unwrap();
        assert_eq!(
            read(&path).pages,
            Some(vec![vec![
                BoardTile { id: "wifi/wifi".into(), w: 1, h: 1, x: 3, y: 0, readings: None },
                BoardTile { id: "clock/clock".into(), w: 2, h: 1, x: 0, y: 1, readings: None },
            ]]),
            "0.2 board"
        );
        std::fs::write(&path, r#"{ "board": null }"#).unwrap();
        assert_eq!(read(&path), Settings::default(), "0.2 default board");

        std::fs::write(&path, "{ not json").unwrap();
        assert_eq!(read(&path), Settings::default(), "invalid file");
        assert_eq!(std::fs::read_to_string(dir.join("settings.invalid.json")).unwrap(), "{ not json", "kept aside");

        let _ = std::fs::remove_dir_all(&dir);
    }
}
