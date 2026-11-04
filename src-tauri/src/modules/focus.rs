//! Focus: switches Windows' Do not disturb, for a while or until turned off.
//!
//! Windows has no public API for this. Its own switch is the older Focus assist ("quiet hours")
//! setting underneath, so this module uses the two undocumented pieces Windows itself uses:
//! - reading and watching: the WNF state Windows announces each change on (no polling; one
//!   callback per change, from wherever it was changed);
//! - switching: the `IQuietHoursSettings` COM object, set to "priority only" (on) or "unrestricted"
//!   (off), the same two values Windows' own switch writes.
//!
//! Tested on Windows 11 26200. If either piece is missing on some other build, the tile says it's
//! unavailable rather than guessing. One thread, which sleeps until a change or a timer runs out.

#![allow(non_snake_case)] // method names must match Windows' interface

use std::ffi::c_void;
use std::sync::Mutex;
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel};
use std::thread::JoinHandle;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};
use tauri_specta::Event;
use windows::Win32::Media::Audio::{PlaySoundW, SND_ASYNC, SND_MEMORY, SND_NODEFAULT};
use windows::Win32::System::Com::{CLSCTX_ALL, CoCreateInstance};
use windows::core::{GUID, HRESULT, HSTRING, IUnknown, IUnknown_Vtbl, PCWSTR, PWSTR, interface};

use crate::module::{Module, ModuleId};
use crate::settings::SettingsState;

/// WNF_SHEL_QUIETHOURS_ACTIVE_PROFILE_CHANGED: 0 when Do not disturb is off, 1 (priority only) or
/// 2 (alarms only) when it's on.
const QUIET_HOURS: u64 = 0x0D83_063E_A3BF_1C75;
const QUIET_HOURS_SETTINGS: GUID = GUID::from_u128(0xf53321fa_34f8_4b7f_b9a3_361877cb94cf);
const OFF: &str = "Microsoft.QuietHoursProfile.Unrestricted";
const ON: &str = "Microsoft.QuietHoursProfile.PriorityOnly";

// Only the first two methods are used, and they come first.
#[interface("6bff4732-81ec-4ffb-ae67-b6c1bc29631f")]
unsafe trait IQuietHoursSettings: IUnknown {
    fn get_UserSelectedProfile(&self, id: *mut PWSTR) -> HRESULT;
    fn put_UserSelectedProfile(&self, id: PCWSTR) -> HRESULT;
}

type WnfCallback = unsafe extern "system" fn(u64, u32, *const c_void, *const c_void, *const c_void, u32) -> i32;

#[link(name = "ntdll")]
unsafe extern "system" {
    fn NtQueryWnfStateData(
        name: *const u64,
        type_id: *const c_void,
        scope: *const c_void,
        stamp: *mut u32,
        buffer: *mut c_void,
        size: *mut u32,
    ) -> i32;
    fn RtlSubscribeWnfStateChangeNotification(
        subscription: *mut *mut c_void,
        name: u64,
        stamp: u32,
        callback: WnfCallback,
        context: *const c_void,
        type_id: *const c_void,
        group: u32,
        unknown: u32,
    ) -> i32;
    fn RtlUnsubscribeWnfStateChangeNotification(subscription: *mut c_void) -> i32;
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum FocusMode {
    Off,
    /// Until turned off.
    On,
    /// For this many minutes, then off.
    Minutes(u32),
}

/// Everything the tiles show.
#[derive(Clone, Debug, Default, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct FocusState {
    /// Do not disturb is on, however it was switched on.
    pub on: bool,
    /// Switched on here for a while: when it turns off, in milliseconds since 1970 (like `Date.now()`).
    pub ends_at: Option<f64>,
    /// How long that while is, in minutes, for the tile's ring.
    pub minutes: Option<u32>,
    /// False if this Windows doesn't have what it needs; the tiles say so.
    pub available: bool,
}

/// Sent to the UI whenever the state changes.
#[derive(Clone, Serialize, specta::Type, tauri_specta::Event)]
pub struct FocusChanged(pub FocusState);

#[derive(Clone, Copy)]
enum Msg {
    Set(FocusMode),
    /// Windows says Do not disturb changed (here or anywhere else).
    Changed,
    /// The timer ran out by itself.
    Ended,
    Stop,
}

/// A timer Traywise started. Kept apart from the Windows calls so it can be tested.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Timer {
    minutes: u32,
    ends: Instant,
}

impl Timer {
    fn new(mode: FocusMode, now: Instant) -> Option<Self> {
        match mode {
            FocusMode::Minutes(minutes) => Some(Self { minutes, ends: now + Duration::from_secs(u64::from(minutes) * 60) }),
            _ => None,
        }
    }

    /// Turned off elsewhere before it ran out: forget it, so it doesn't later turn off a Do not
    /// disturb someone switched on again by hand.
    fn keep(timer: Option<Self>, on: bool) -> Option<Self> {
        timer.filter(|_| on)
    }
}

#[derive(Default)]
pub struct Shared {
    /// Only set while the module is running.
    tx: Mutex<Option<Sender<Msg>>>,
    state: Mutex<FocusState>,
}

pub struct Focus {
    app: AppHandle,
    worker: Option<JoinHandle<()>>,
}

impl Focus {
    pub fn new(app: AppHandle) -> Self {
        app.manage(Shared::default());
        Self { app, worker: None }
    }
}

impl Module for Focus {
    fn id(&self) -> ModuleId {
        ModuleId::Focus
    }

    fn start(&mut self) -> Result<(), String> {
        let (tx, rx) = channel();
        *self.app.state::<Shared>().tx.lock().unwrap() = Some(tx.clone());
        let app = self.app.clone();
        let worker = std::thread::Builder::new()
            .name("focus".into())
            .spawn(move || {
                crate::audio::with_com("focus", || {
                    run(app, tx, rx);
                    Ok(())
                })
            })
            .map_err(|e| e.to_string())?;
        self.worker = Some(worker);
        Ok(())
    }

    /// Leaves Do not disturb as it is: switching the module off isn't asking for notifications back.
    fn stop(&mut self) {
        let shared = self.app.state::<Shared>();
        if let Some(tx) = shared.tx.lock().unwrap().take() {
            let _ = tx.send(Msg::Stop);
        }
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
        *shared.state.lock().unwrap() = FocusState::default();
    }
}

fn run(app: AppHandle, tx: Sender<Msg>, rx: Receiver<Msg>) {
    let settings: Option<IQuietHoursSettings> = unsafe { CoCreateInstance(&QUIET_HOURS_SETTINGS, None, CLSCTX_ALL) }
        .inspect_err(|e| eprintln!("focus: quiet hours settings unavailable: {e}"))
        .ok();
    let watch = Watch::start(tx);
    let mut timer: Option<Timer> = None;
    loop {
        publish(&app, read(), timer, settings.is_some() && watch.is_some());

        let msg = match timer {
            Some(t) => rx.recv_timeout(t.ends.saturating_duration_since(Instant::now())).unwrap_or_else(|e| match e {
                RecvTimeoutError::Timeout => Msg::Ended,
                RecvTimeoutError::Disconnected => Msg::Stop,
            }),
            None => rx.recv().unwrap_or(Msg::Stop),
        };
        match msg {
            Msg::Set(_) | Msg::Ended => {
                let mode = if let Msg::Set(mode) = msg { mode } else { FocusMode::Off };
                if let Some(settings) = &settings {
                    let id = HSTRING::from(if mode == FocusMode::Off { OFF } else { ON });
                    if let Err(e) = unsafe { settings.put_UserSelectedProfile(PCWSTR(id.as_ptr())) }.ok() {
                        eprintln!("focus: couldn't switch Do not disturb: {e}");
                    }
                }
                timer = Timer::new(mode, Instant::now());
                if matches!(msg, Msg::Ended) && app.state::<SettingsState>().get().focus_chime {
                    chime();
                }
            }
            Msg::Changed => timer = Timer::keep(timer, read() == Some(true)),
            Msg::Stop => break,
        }
    }
    drop(watch);
}

static CHIME: &[u8] = include_bytes!("../../assets/focus-chime.wav");

/// A soft two-note sound, played by Windows in the background (no audio library, no thread).
fn chime() {
    // The bytes are 'static, so Windows can keep reading them after this returns.
    let _ = unsafe { PlaySoundW(PCWSTR(CHIME.as_ptr().cast()), None, SND_MEMORY | SND_ASYNC | SND_NODEFAULT) };
}

/// Whether Do not disturb is on. None if Windows doesn't have the state.
fn read() -> Option<bool> {
    let mut value = 0u32;
    let (mut stamp, mut size) = (0u32, 4u32);
    let status = unsafe {
        NtQueryWnfStateData(&QUIET_HOURS, std::ptr::null(), std::ptr::null(), &mut stamp, (&raw mut value).cast(), &mut size)
    };
    (status >= 0 && size == 4).then_some(value != 0)
}

/// Windows' announcements of Do not disturb changes, while this lives.
struct Watch {
    subscription: *mut c_void,
    /// Where the callback sends; boxed so its address stays put for Windows.
    _tx: Box<Sender<Msg>>,
}

impl Watch {
    fn start(tx: Sender<Msg>) -> Option<Self> {
        let tx = Box::new(tx);
        let mut subscription = std::ptr::null_mut();
        let status = unsafe {
            RtlSubscribeWnfStateChangeNotification(
                &mut subscription,
                QUIET_HOURS,
                0,
                changed,
                (&raw const *tx).cast(),
                std::ptr::null(),
                0,
                0,
            )
        };
        if status < 0 {
            eprintln!("focus: couldn't watch Do not disturb: {status:#x}");
            return None;
        }
        Some(Self { subscription, _tx: tx })
    }
}

impl Drop for Watch {
    fn drop(&mut self) {
        // Waits for a callback that's running, so `_tx` outlives every call.
        unsafe { RtlUnsubscribeWnfStateChangeNotification(self.subscription) };
    }
}

unsafe extern "system" fn changed(_: u64, _: u32, _: *const c_void, context: *const c_void, _: *const c_void, _: u32) -> i32 {
    let tx = unsafe { &*context.cast::<Sender<Msg>>() };
    let _ = tx.send(Msg::Changed);
    0
}

fn publish(app: &AppHandle, on: Option<bool>, timer: Option<Timer>, works: bool) {
    let ends_at = timer.map(|t| {
        let end = SystemTime::now() + t.ends.saturating_duration_since(Instant::now());
        end.duration_since(UNIX_EPOCH).unwrap_or_default().as_secs_f64() * 1000.0
    });
    let state = FocusState { on: on == Some(true), ends_at, minutes: timer.map(|t| t.minutes), available: works && on.is_some() };
    let shared = app.state::<Shared>();
    let mut current = shared.state.lock().unwrap();
    if *current != state {
        *current = state.clone();
        let _ = FocusChanged(state).emit(app);
    }
}

/// The latest state. Default (off, unavailable) while the module is switched off.
#[tauri::command]
#[specta::specta]
pub fn focus_state(shared: State<Shared>) -> FocusState {
    shared.state.lock().unwrap().clone()
}

/// Picking a timed mode (again) restarts its timer.
#[tauri::command]
#[specta::specta]
pub fn focus_set(shared: State<Shared>, mode: FocusMode) -> Result<(), String> {
    let tx = shared.tx.lock().unwrap();
    tx.as_ref().ok_or("Focus is off")?.send(Msg::Set(mode)).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_timed_modes_end() {
        let now = Instant::now();
        assert_eq!(Timer::new(FocusMode::Minutes(25), now), Some(Timer { minutes: 25, ends: now + Duration::from_secs(25 * 60) }));
        assert_eq!(Timer::new(FocusMode::On, now), None);
        assert_eq!(Timer::new(FocusMode::Off, now), None);
    }

    #[test]
    fn turned_off_elsewhere_forgets_the_timer() {
        let timer = Timer::new(FocusMode::Minutes(50), Instant::now());
        assert_eq!(Timer::keep(timer, true), timer);
        assert_eq!(Timer::keep(timer, false), None);
    }
}
