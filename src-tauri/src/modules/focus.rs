//! Focus: a Pomodoro-style timer (focus and rest rounds) that switches Windows' Do not disturb, plus the
//! plain Do not disturb switch.
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

/// The plain Do not disturb switch (its own tile).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum FocusMode {
    Off,
    On,
}

/// Which part of the cycle is running.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum Phase {
    Focus,
    Rest,
    /// The longer rest after the last round.
    LongRest,
}

/// What the tile's buttons do to the running cycle.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum FocusAction {
    Pause,
    Resume,
    /// Jump to the next phase now.
    Skip,
    /// Stop the cycle.
    End,
}

/// Everything the tiles show.
#[derive(Clone, Debug, Default, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct FocusState {
    /// Do not disturb is on, however it was switched on.
    pub on: bool,
    /// False if this Windows doesn't have what it needs; the tiles say so.
    pub available: bool,
    /// The running cycle's phase, or None when no cycle is running.
    pub phase: Option<Phase>,
    /// Which round (1 to 4) the cycle is in.
    pub round: Option<u8>,
    /// How long the current phase is, in minutes.
    pub minutes: Option<u32>,
    /// While it runs: when the phase ends, in milliseconds since 1970 (like `Date.now()`).
    pub ends_at: Option<f64>,
    /// While paused (or waiting for a tap to start the phase): the time left, in milliseconds.
    pub left_ms: Option<f64>,
}

/// Sent to the UI whenever the state changes.
#[derive(Clone, Serialize, specta::Type, tauri_specta::Event)]
pub struct FocusChanged(pub FocusState);

#[derive(Clone, Copy)]
enum Msg {
    Set(FocusMode),
    Start { focus: u32, rest: u32 },
    Control(FocusAction),
    /// Windows says Do not disturb changed (here or anywhere else). A cycle carries on regardless.
    Changed,
    /// The phase ran out by itself.
    Ended,
    Stop,
}

/// The longest focus or rest, in minutes (8 hours).
const MAX_MINUTES: u32 = 480;
/// Focus rounds before the long rest.
const ROUNDS: u8 = 4;

/// A running or paused cycle: focus and rest rounds, then one long rest. Kept apart from the Windows
/// calls so it can be tested.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Cycle {
    focus: u32,
    rest: u32,
    long_rest: u32,
    phase: Phase,
    round: u8,
    /// When the phase ends, while it runs.
    ends: Instant,
    /// Time left, while paused.
    paused: Option<Duration>,
}

impl Cycle {
    fn start(focus: u32, rest: u32, long_rest: u32, now: Instant) -> Self {
        let clamp = |m: u32| m.clamp(1, MAX_MINUTES);
        let mut cycle = Self { focus: clamp(focus), rest: clamp(rest), long_rest: clamp(long_rest), phase: Phase::Focus, round: 1, ends: now, paused: None };
        cycle.ends = now + cycle.length();
        cycle
    }

    fn minutes(&self) -> u32 {
        match self.phase {
            Phase::Focus => self.focus,
            Phase::Rest => self.rest,
            Phase::LongRest => self.long_rest,
        }
    }

    fn length(&self) -> Duration {
        Duration::from_secs(u64::from(self.minutes()) * 60)
    }

    fn remaining(&self, now: Instant) -> Duration {
        self.paused.unwrap_or_else(|| self.ends.saturating_duration_since(now))
    }

    fn pause(&mut self, now: Instant) {
        if self.paused.is_none() {
            self.paused = Some(self.remaining(now));
        }
    }

    fn resume(&mut self, now: Instant) {
        if let Some(left) = self.paused.take() {
            self.ends = now + left;
        }
    }

    /// The cycle after this phase: None once the long rest is over. With `auto` the next phase starts
    /// at once; otherwise it waits, paused at its full length, for a tap.
    fn advance(self, now: Instant, auto: bool) -> Option<Self> {
        let (phase, round) = match self.phase {
            Phase::Focus if self.round >= ROUNDS => (Phase::LongRest, self.round),
            Phase::Focus => (Phase::Rest, self.round),
            Phase::Rest => (Phase::Focus, self.round + 1),
            Phase::LongRest => return None,
        };
        let mut next = Self { phase, round, ends: now, paused: None, ..self };
        next.ends = now + next.length();
        if !auto {
            next.paused = Some(next.length());
        }
        Some(next)
    }
}

/// What Do not disturb should become when a phase starts (`None` for the cycle ending), from the two
/// settings; None means leave it alone.
fn dnd_for(entering: Option<Phase>, while_focusing: bool, during_rest: bool) -> Option<bool> {
    match entering {
        Some(Phase::Focus) => while_focusing.then_some(true),
        Some(_) if during_rest => Some(true),
        _ => while_focusing.then_some(false),
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
    let switch = |on: bool| {
        if let Some(settings) = &settings {
            let id = HSTRING::from(if on { ON } else { OFF });
            if let Err(e) = unsafe { settings.put_UserSelectedProfile(PCWSTR(id.as_ptr())) }.ok() {
                eprintln!("focus: couldn't switch Do not disturb: {e}");
            }
        }
    };
    // Do not disturb follows the phase that is starting, per the two settings.
    let enter = |phase: Option<Phase>| {
        let s = app.state::<SettingsState>().get();
        if let Some(on) = dnd_for(phase, s.focus_dnd_focus, s.focus_dnd_rest) {
            switch(on);
        }
    };
    let mut cycle: Option<Cycle> = None;
    loop {
        publish(&app, read(), cycle, settings.is_some() && watch.is_some());

        let msg = match cycle.filter(|c| c.paused.is_none()) {
            Some(c) => rx.recv_timeout(c.ends.saturating_duration_since(Instant::now())).unwrap_or_else(|e| match e {
                RecvTimeoutError::Timeout => Msg::Ended,
                RecvTimeoutError::Disconnected => Msg::Stop,
            }),
            None => rx.recv().unwrap_or(Msg::Stop),
        };
        let now = Instant::now();
        match msg {
            Msg::Set(mode) => switch(mode == FocusMode::On),
            Msg::Start { focus, rest } => {
                cycle = Some(Cycle::start(focus, rest, app.state::<SettingsState>().get().focus_long_rest, now));
                enter(Some(Phase::Focus));
            }
            Msg::Control(action) => match (action, cycle.as_mut()) {
                (FocusAction::Pause, Some(c)) => c.pause(now),
                (FocusAction::Resume, Some(c)) => c.resume(now),
                (FocusAction::Skip | FocusAction::End, Some(_)) => {
                    let auto = app.state::<SettingsState>().get().focus_auto_next;
                    cycle = cycle.and_then(|c| if action == FocusAction::Skip { c.advance(now, auto) } else { None });
                    enter(cycle.map(|c| c.phase));
                }
                _ => {}
            },
            Msg::Ended => {
                let s = app.state::<SettingsState>().get();
                if s.focus_chime {
                    chime();
                }
                cycle = cycle.and_then(|c| c.advance(now, s.focus_auto_next));
                enter(cycle.map(|c| c.phase));
            }
            Msg::Changed => {}
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

fn publish(app: &AppHandle, on: Option<bool>, cycle: Option<Cycle>, works: bool) {
    let ms = |d: Duration| d.as_secs_f64() * 1000.0;
    let ends_at = cycle.filter(|c| c.paused.is_none()).map(|c| {
        let end = SystemTime::now() + c.ends.saturating_duration_since(Instant::now());
        ms(end.duration_since(UNIX_EPOCH).unwrap_or_default())
    });
    let state = FocusState {
        on: on == Some(true),
        available: works && on.is_some(),
        phase: cycle.map(|c| c.phase),
        round: cycle.map(|c| c.round),
        minutes: cycle.map(|c| c.minutes()),
        ends_at,
        left_ms: cycle.and_then(|c| c.paused).map(ms),
    };
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

/// Do not disturb on or off, on its own (a running cycle carries on).
#[tauri::command]
#[specta::specta]
pub fn focus_set(shared: State<Shared>, mode: FocusMode) -> Result<(), String> {
    send(&shared, Msg::Set(mode))
}

/// Starts a cycle of focus and rest rounds, in minutes. Starting again restarts it.
#[tauri::command]
#[specta::specta]
pub fn focus_start(shared: State<Shared>, focus: u32, rest: u32) -> Result<(), String> {
    send(&shared, Msg::Start { focus, rest })
}

/// Pause, resume, skip to the next phase, or end the running cycle.
#[tauri::command]
#[specta::specta]
pub fn focus_control(shared: State<Shared>, action: FocusAction) -> Result<(), String> {
    send(&shared, Msg::Control(action))
}

fn send(shared: &Shared, msg: Msg) -> Result<(), String> {
    let tx = shared.tx.lock().unwrap();
    tx.as_ref().ok_or("Focus is off")?.send(msg).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn last(mut cycle: Cycle, now: Instant) -> Cycle {
        while let Some(next) = cycle.advance(now, true) {
            cycle = next;
        }
        cycle
    }

    #[test]
    fn four_rounds_then_a_long_rest() {
        use Phase::*;
        let now = Instant::now();
        let mut cycle = Cycle::start(25, 5, 15, now);
        let mut seen = vec![(cycle.phase, cycle.round)];
        while let Some(next) = cycle.advance(now, true) {
            seen.push((next.phase, next.round));
            cycle = next;
        }
        assert_eq!(seen, [(Focus, 1), (Rest, 1), (Focus, 2), (Rest, 2), (Focus, 3), (Rest, 3), (Focus, 4), (LongRest, 4)]);
    }

    #[test]
    fn each_phase_has_its_own_length() {
        let now = Instant::now();
        let c = Cycle::start(25, 5, 15, now);
        assert_eq!(c.ends, now + Duration::from_secs(25 * 60));
        assert_eq!(c.advance(now, true).unwrap().ends, now + Duration::from_secs(5 * 60));
        let long = last(c, now);
        assert_eq!((long.phase, long.minutes()), (Phase::LongRest, 15));
        let clamped = Cycle::start(0, 9999, 1, now);
        assert_eq!((clamped.focus, clamped.rest), (1, 480), "lengths are clamped");
    }

    #[test]
    fn without_auto_start_the_next_phase_waits() {
        let now = Instant::now();
        let mut next = Cycle::start(25, 5, 15, now).advance(now, false).unwrap();
        assert_eq!((next.phase, next.paused), (Phase::Rest, Some(Duration::from_secs(5 * 60))));
        let later = now + Duration::from_secs(60);
        next.resume(later);
        assert_eq!((next.paused, next.ends), (None, later + Duration::from_secs(5 * 60)));
    }

    #[test]
    fn pausing_keeps_the_time_left() {
        let now = Instant::now();
        let mut c = Cycle::start(25, 5, 15, now);
        c.pause(now + Duration::from_secs(10 * 60));
        assert_eq!(c.remaining(now + Duration::from_secs(99 * 60)), Duration::from_secs(15 * 60));
        c.resume(now + Duration::from_secs(20 * 60));
        assert_eq!(c.remaining(now + Duration::from_secs(21 * 60)), Duration::from_secs(14 * 60));
    }

    #[test]
    fn do_not_disturb_follows_the_phase() {
        use Phase::*;
        assert_eq!(dnd_for(Some(Focus), true, false), Some(true));
        assert_eq!(dnd_for(Some(Rest), true, false), Some(false), "off for the rest");
        assert_eq!(dnd_for(Some(LongRest), true, true), Some(true), "kept on through rests");
        assert_eq!(dnd_for(None, true, true), Some(false), "off when the cycle ends");
        assert_eq!(dnd_for(Some(Focus), false, false), None, "setting off: never touched");
        assert_eq!(dnd_for(None, false, true), None);
    }
}
