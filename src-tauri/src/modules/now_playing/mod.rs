//! Now playing: what each app is playing (Spotify, a browser tab, a video player), with its
//! controls. It's the same Windows media API behind the media card in Windows' own volume pop-up.
//!
//! One background thread (`worker.rs`) listens for Windows' media events: an app starts or stops
//! playing, the track changes, it's paused. While the panel is hidden it only notes that
//! something changed, and catches up when the panel opens, so it costs nothing while hidden.

mod worker;

use std::sync::Mutex;
use std::sync::mpsc::{Sender, channel};
use std::thread::JoinHandle;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};

use crate::module::{Module, ModuleId};

/// One app's media.
#[derive(Clone, Debug, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct MediaSession {
    /// Which app this is, for the controls: its app id, plus `#2`, `#3` for a second or third
    /// session from the same app.
    pub key: String,
    /// The app's name, like "Spotify".
    pub app: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    /// The cover art as a data URL, if the app gives one.
    pub artwork: Option<String>,
    pub playing: bool,
    /// Which controls the app allows right now.
    pub can: Abilities,
    /// None when the app doesn't say.
    pub shuffle: Option<bool>,
    pub repeat: Option<Repeat>,
    /// Milliseconds into the track, as of `updated_at`. While playing, the UI moves it on by itself.
    pub position_ms: u32,
    /// The track's length in milliseconds; 0 when the app doesn't say (a live stream, say).
    pub duration_ms: u32,
    /// When the app last reported its position, in milliseconds since 1970 (like `Date.now()`).
    pub updated_at: f64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Abilities {
    pub play_pause: bool,
    pub next: bool,
    pub previous: bool,
    pub seek: bool,
    pub shuffle: bool,
    pub repeat: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum Repeat {
    Off,
    Track,
    List,
}

/// Everything the panel shows.
#[derive(Clone, Debug, Default, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct NowPlayingState {
    pub sessions: Vec<MediaSession>,
    /// The session Windows considers current (the one its own media card shows): what the tiles show.
    pub current: Option<String>,
}

/// Sent to the UI whenever the state changes while the panel is open, and when it opens.
#[derive(Clone, Serialize, specta::Type, tauri_specta::Event)]
pub struct NowPlayingChanged(pub NowPlayingState);

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum MediaAction {
    PlayPause,
    Next,
    Previous,
    /// Jump to this many milliseconds into the track.
    Seek(u32),
    Shuffle(bool),
    Repeat(Repeat),
}

enum Msg {
    /// An app started or stopped having media.
    SessionsChanged,
    /// Something about a session changed (track, play state, position).
    Changed,
    Visible(bool),
    Control(String, MediaAction),
    Stop,
}

/// Shared between the module, its worker thread and the commands.
#[derive(Default)]
pub struct Shared {
    /// Only set while the module is running.
    tx: Mutex<Option<Sender<Msg>>>,
    state: Mutex<NowPlayingState>,
}

pub struct NowPlaying {
    app: AppHandle,
    worker: Option<JoinHandle<()>>,
}

impl NowPlaying {
    pub fn new(app: AppHandle) -> Self {
        app.manage(Shared::default());
        Self { app, worker: None }
    }
}

impl Module for NowPlaying {
    fn id(&self) -> ModuleId {
        ModuleId::NowPlaying
    }

    fn start(&mut self) -> Result<(), String> {
        let (tx, rx) = channel();
        *self.app.state::<Shared>().tx.lock().unwrap() = Some(tx.clone());
        let app = self.app.clone();
        let worker = std::thread::Builder::new()
            .name("now-playing".into())
            .spawn(move || worker::run(app, tx, rx))
            .map_err(|e| e.to_string())?;
        self.worker = Some(worker);
        println!("now-playing started");
        Ok(())
    }

    fn stop(&mut self) {
        let shared = self.app.state::<Shared>();
        if let Some(tx) = shared.tx.lock().unwrap().take() {
            let _ = tx.send(Msg::Stop);
        }
        if let Some(worker) = self.worker.take() {
            let _ = worker.join(); // the worker unsubscribes from Windows' events before it ends
        }
        *shared.state.lock().unwrap() = NowPlayingState::default();
        println!("now-playing stopped");
    }

    fn panel_visible(&mut self, visible: bool) {
        if let Some(tx) = self.app.state::<Shared>().tx.lock().unwrap().as_ref() {
            let _ = tx.send(Msg::Visible(visible));
        }
    }
}

/// The latest state. Empty while the module is switched off.
#[tauri::command]
#[specta::specta]
pub fn now_playing_state(shared: State<Shared>) -> NowPlayingState {
    shared.state.lock().unwrap().clone()
}

/// Play/pause, skip, seek, shuffle or repeat in one app (`key` is a session's key).
#[tauri::command]
#[specta::specta]
pub fn now_playing_control(shared: State<Shared>, key: String, action: MediaAction) -> Result<(), String> {
    let tx = shared.tx.lock().unwrap();
    tx.as_ref().ok_or("now playing is off")?.send(Msg::Control(key, action)).map_err(|e| e.to_string())
}

/// A readable name from an app id Windows couldn't name: "Spotify.exe" -> "Spotify",
/// "Microsoft.ZuneMusic_8wekyb3d8bbwe!Microsoft.ZuneMusic" -> "ZuneMusic".
fn name_from_app_id(id: &str) -> String {
    let id = id.split('!').next_back().unwrap_or(id);
    let id = id.strip_suffix(".exe").or_else(|| id.strip_suffix(".EXE")).unwrap_or(id);
    let name = id.rsplit('.').next().unwrap_or(id);
    if name.is_empty() { id.to_string() } else { name.to_string() }
}

/// Keys for sessions in Windows' order: the app id, with `#2`, `#3` for repeats.
fn keys<'a>(app_ids: impl IntoIterator<Item = &'a str>) -> Vec<String> {
    let mut seen: Vec<&str> = Vec::new();
    app_ids
        .into_iter()
        .map(|id| {
            let n = seen.iter().filter(|s| **s == id).count();
            seen.push(id);
            if n == 0 { id.to_string() } else { format!("{id}#{}", n + 1) }
        })
        .collect()
}

/// Windows times: a TimeSpan is in 100 ns ticks; a DateTime counts those since 1601.
fn ms(ticks: i64) -> u32 {
    (ticks / 10_000).clamp(0, i64::from(u32::MAX)) as u32
}
fn unix_ms(date_ticks: i64) -> f64 {
    (date_ticks - 116_444_736_000_000_000) as f64 / 10_000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_names() {
        assert_eq!(name_from_app_id("Spotify.exe"), "Spotify");
        assert_eq!(name_from_app_id("Microsoft.ZuneMusic_8wekyb3d8bbwe!Microsoft.ZuneMusic"), "ZuneMusic");
        assert_eq!(name_from_app_id("Chrome"), "Chrome");
        assert_eq!(name_from_app_id(""), "");
    }

    #[test]
    fn repeated_apps_get_numbered_keys() {
        assert_eq!(keys(["Chrome", "Spotify.exe", "Chrome", "Chrome"]), ["Chrome", "Spotify.exe", "Chrome#2", "Chrome#3"]);
    }

    #[test]
    fn times() {
        assert_eq!(ms(25_000_000), 2_500);
        assert_eq!(ms(-5), 0);
        assert_eq!(unix_ms(116_444_736_000_000_000), 0.0);
        assert_eq!(unix_ms(116_444_736_000_000_000 + 10_000), 1.0);
    }
}
