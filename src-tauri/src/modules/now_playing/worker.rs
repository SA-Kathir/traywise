//! The now-playing thread and its Windows calls (Windows.Media.Control).

use std::collections::HashMap;
use std::sync::mpsc::{Receiver, Sender};

use base64::Engine;
use tauri::{AppHandle, Manager};
use tauri_specta::Event;
use windows::ApplicationModel::AppInfo;
use windows::Foundation::TypedEventHandler;
use windows::Media::Control::{
    GlobalSystemMediaTransportControlsSession as Session, GlobalSystemMediaTransportControlsSessionManager as SessionManager,
    GlobalSystemMediaTransportControlsSessionPlaybackStatus as Status,
};
use windows::Media::MediaPlaybackAutoRepeatMode;
use windows::Storage::Streams::DataReader;
use windows::core::{HSTRING, Result};

use super::{
    Abilities, MediaAction, MediaSession, Msg, NowPlayingChanged, NowPlayingState, Repeat, Shared, keys, name_from_app_id,
    ms, unix_ms,
};
use crate::audio::with_com;

/// Cover art bigger than this isn't sent to the panel.
const MAX_ARTWORK_BYTES: u64 = 1024 * 1024;

/// A session and its event subscriptions, removed when it goes.
struct Watched {
    key: String,
    session: Session,
    tokens: [i64; 3],
}

impl Drop for Watched {
    fn drop(&mut self) {
        let [media, playback, timeline] = self.tokens;
        let _ = self.session.RemoveMediaPropertiesChanged(media);
        let _ = self.session.RemovePlaybackInfoChanged(playback);
        let _ = self.session.RemoveTimelinePropertiesChanged(timeline);
    }
}

pub fn run(app: AppHandle, tx: Sender<Msg>, rx: Receiver<Msg>) {
    with_com("now-playing", || {
        let manager = SessionManager::RequestAsync()?.join()?;
        let on_sessions = manager.SessionsChanged(&handler(&tx, || Msg::SessionsChanged))?;
        let on_current = manager.CurrentSessionChanged(&handler(&tx, || Msg::Changed))?;

        let mut watched = watch(&manager, &tx)?;
        // The cover art of each session's current track, read once per track: (track, data URL).
        let mut artwork: HashMap<String, (String, Option<String>)> = HashMap::new();
        let mut visible = false;
        let mut dirty = true;
        loop {
            match rx.recv().unwrap_or(Msg::Stop) {
                Msg::Stop => break,
                Msg::SessionsChanged => {
                    drop(std::mem::take(&mut watched)); // unsubscribe from the old ones first
                    watched = watch(&manager, &tx).unwrap_or_else(|e| {
                        eprintln!("now-playing: couldn't list the apps playing media: {e}");
                        Vec::new()
                    });
                    dirty = true;
                }
                Msg::Changed => dirty = true,
                Msg::Visible(v) => visible = v,
                Msg::Control(key, action) => {
                    if let Some(w) = watched.iter().find(|w| w.key == key)
                        && let Err(e) = control(&w.session, action)
                    {
                        eprintln!("now-playing: {action:?} failed: {e}");
                    }
                }
            }
            // Only while the panel is open: some players report their position every second.
            if visible && dirty {
                dirty = false;
                publish(&app, &manager, &watched, &mut artwork);
            }
        }
        let _ = manager.RemoveSessionsChanged(on_sessions);
        let _ = manager.RemoveCurrentSessionChanged(on_current);
        Ok(())
    });
}

/// An event handler that only tells the worker; the work happens on its thread.
fn handler<S, A>(tx: &Sender<Msg>, msg: fn() -> Msg) -> TypedEventHandler<S, A>
where
    S: windows::core::RuntimeType + 'static,
    A: windows::core::RuntimeType + 'static,
{
    let tx = tx.clone();
    TypedEventHandler::new(move |_, _| {
        let _ = tx.send(msg());
        Ok(())
    })
}

/// Subscribes to every session's changes.
fn watch(manager: &SessionManager, tx: &Sender<Msg>) -> Result<Vec<Watched>> {
    let sessions: Vec<Session> = manager.GetSessions()?.into_iter().collect();
    let ids: Vec<String> = sessions.iter().map(|s| s.SourceAppUserModelId().unwrap_or_default().to_string()).collect();
    let keys = keys(ids.iter().map(String::as_str));
    sessions
        .into_iter()
        .zip(keys)
        .map(|(session, key)| {
            let tokens = [
                session.MediaPropertiesChanged(&handler(tx, || Msg::Changed))?,
                session.PlaybackInfoChanged(&handler(tx, || Msg::Changed))?,
                session.TimelinePropertiesChanged(&handler(tx, || Msg::Changed))?,
            ];
            Ok(Watched { key, session, tokens })
        })
        .collect()
}

fn publish(
    app: &AppHandle,
    manager: &SessionManager,
    watched: &[Watched],
    artwork: &mut HashMap<String, (String, Option<String>)>,
) {
    let sessions: Vec<MediaSession> = watched
        .iter()
        .filter_map(|w| match read(w, artwork) {
            Ok(s) => Some(s),
            Err(e) => {
                eprintln!("now-playing: couldn't read {}: {e}", w.key);
                None
            }
        })
        .collect();
    artwork.retain(|key, _| watched.iter().any(|w| &w.key == key));
    // Windows' current session, matched by app: its object isn't the same one GetSessions gives.
    let current_app = manager.GetCurrentSession().and_then(|s| s.SourceAppUserModelId()).ok();
    let current = current_app.and_then(|id| {
        watched.iter().find(|w| w.session.SourceAppUserModelId().is_ok_and(|s| s == id)).map(|w| w.key.clone())
    });
    let state = NowPlayingState { sessions, current };
    let shared = app.state::<Shared>();
    let mut last = shared.state.lock().unwrap();
    if *last != state {
        *last = state.clone();
        let _ = NowPlayingChanged(state).emit(app);
    }
}

fn read(w: &Watched, artwork: &mut HashMap<String, (String, Option<String>)>) -> Result<MediaSession> {
    let session = &w.session;
    let id = session.SourceAppUserModelId()?;
    let media = session.TryGetMediaPropertiesAsync()?.join()?;
    let (title, artist, album) = (media.Title()?.to_string(), media.Artist()?.to_string(), media.AlbumTitle()?.to_string());
    let track = format!("{title}\n{artist}\n{album}");
    let art = match artwork.get(&w.key) {
        Some((t, art)) if *t == track => art.clone(),
        _ => {
            let art = media.Thumbnail().ok().and_then(|thumb| read_artwork(&thumb).ok().flatten());
            artwork.insert(w.key.clone(), (track, art.clone()));
            art
        }
    };

    let playback = session.GetPlaybackInfo()?;
    let controls = playback.Controls()?;
    let can = Abilities {
        play_pause: controls.IsPlayPauseToggleEnabled()? || controls.IsPlayEnabled()? || controls.IsPauseEnabled()?,
        next: controls.IsNextEnabled()?,
        previous: controls.IsPreviousEnabled()?,
        seek: controls.IsPlaybackPositionEnabled()?,
        shuffle: controls.IsShuffleEnabled()?,
        repeat: controls.IsRepeatEnabled()?,
    };
    let repeat = playback.AutoRepeatMode().and_then(|m| m.Value()).ok().map(|m| match m {
        MediaPlaybackAutoRepeatMode::Track => Repeat::Track,
        MediaPlaybackAutoRepeatMode::List => Repeat::List,
        _ => Repeat::Off,
    });

    let timeline = session.GetTimelineProperties()?;
    let start = timeline.StartTime()?.Duration;
    Ok(MediaSession {
        key: w.key.clone(),
        app: app_name(&id),
        title,
        artist,
        album,
        artwork: art,
        playing: playback.PlaybackStatus()? == Status::Playing,
        can,
        shuffle: playback.IsShuffleActive().and_then(|s| s.Value()).ok(),
        repeat,
        position_ms: ms(timeline.Position()?.Duration - start),
        duration_ms: ms(timeline.EndTime()?.Duration - start),
        updated_at: unix_ms(timeline.LastUpdatedTime()?.UniversalTime),
    })
}

/// The name Windows shows for an app, or one made from its id (plain .exe apps have no entry).
fn app_name(id: &HSTRING) -> String {
    AppInfo::GetFromAppUserModelId(id)
        .and_then(|info| info.DisplayInfo())
        .and_then(|d| d.DisplayName())
        .map(|n| n.to_string())
        .unwrap_or_else(|_| name_from_app_id(&id.to_string()))
}

/// The cover art as a data URL, or None if there is none or it's too big.
fn read_artwork(thumb: &windows::Storage::Streams::IRandomAccessStreamReference) -> Result<Option<String>> {
    let stream = thumb.OpenReadAsync()?.join()?;
    let size = stream.Size()?;
    if size == 0 || size > MAX_ARTWORK_BYTES {
        return Ok(None);
    }
    let kind = stream.ContentType()?.to_string();
    let reader = DataReader::CreateDataReader(&stream)?;
    let read = reader.LoadAsync(size as u32)?.join()?;
    let mut bytes = vec![0u8; read as usize];
    reader.ReadBytes(&mut bytes)?;
    let kind = if kind.starts_with("image/") { kind } else { "image/jpeg".into() };
    Ok(Some(format!("data:{kind};base64,{}", base64::engine::general_purpose::STANDARD.encode(bytes))))
}

fn control(session: &Session, action: MediaAction) -> Result<()> {
    // Each returns whether the app accepted it; an app that ignores one is no error of ours.
    let _ = match action {
        MediaAction::PlayPause => session.TryTogglePlayPauseAsync()?.join()?,
        MediaAction::Next => session.TrySkipNextAsync()?.join()?,
        MediaAction::Previous => session.TrySkipPreviousAsync()?.join()?,
        MediaAction::Seek(at) => {
            let start = session.GetTimelineProperties()?.StartTime()?.Duration;
            session.TryChangePlaybackPositionAsync(start + i64::from(at) * 10_000)?.join()?
        }
        MediaAction::Shuffle(on) => session.TryChangeShuffleActiveAsync(on)?.join()?,
        MediaAction::Repeat(mode) => session
            .TryChangeAutoRepeatModeAsync(match mode {
                Repeat::Off => MediaPlaybackAutoRepeatMode::None,
                Repeat::Track => MediaPlaybackAutoRepeatMode::Track,
                Repeat::List => MediaPlaybackAutoRepeatMode::List,
            })?
            .join()?,
    };
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Prints what this PC is playing right now, through the same calls the module makes. By hand:
    /// `cargo test now_playing_live -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn now_playing_live() {
        with_com("now-playing test", || {
            let manager = SessionManager::RequestAsync()?.join()?;
            let (tx, _rx) = std::sync::mpsc::channel();
            let mut artwork = HashMap::new();
            for w in watch(&manager, &tx)? {
                let s = read(&w, &mut artwork)?;
                let art = s.artwork.as_ref().map_or(0, String::len);
                println!("{} ({}): {:?} by {:?}, playing {}, {}/{} ms, {:?}, artwork {art} chars", s.key, s.app, s.title, s.artist, s.playing, s.position_ms, s.duration_ms, s.can);
            }
            Ok(())
        });
    }
}
