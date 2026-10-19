//! The keep-awake thread and its Windows calls: one power request, plus a power-source callback.

use std::ffi::c_void;
use std::ptr;
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use tauri::{AppHandle, Manager};
use tauri_specta::Event;
use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::Globalization::{GetTimeFormatEx, TIME_NOSECONDS};
use windows::Win32::System::SystemInformation::GetLocalTime;
use windows::Win32::System::Power::{
    DEVICE_NOTIFY_SUBSCRIBE_PARAMETERS, GetPwrCapabilities, GetSystemPowerStatus, HPOWERNOTIFY, POWER_REQUEST_TYPE, PowerClearRequest,
    PowerCreateRequest, PowerRequestDisplayRequired, PowerRequestSystemRequired, PowerSetRequest,
    PowerSettingRegisterNotification, PowerSettingUnregisterNotification, SYSTEM_POWER_CAPABILITIES,
    SYSTEM_POWER_STATUS,
};
use windows::Win32::System::SystemServices::{GUID_ACDC_POWER_SOURCE, POWER_REQUEST_CONTEXT_VERSION};
use windows::Win32::System::Threading::{POWER_REQUEST_CONTEXT_SIMPLE_STRING, REASON_CONTEXT, REASON_CONTEXT_0};
use windows::Win32::UI::WindowsAndMessaging::DEVICE_NOTIFY_CALLBACK;
use windows::core::{PCWSTR, PWSTR, Result};

use super::{KeepAwakeChanged, KeepAwakeState, Mode, Msg, Shared, Wish};
use crate::shell::tray;

pub fn run(app: AppHandle, tx: Sender<Msg>, rx: Receiver<Msg>) {
    let mut request = match PowerRequest::new() {
        Ok(request) => request,
        Err(e) => return eprintln!("keep-awake: couldn't create a power request: {e}"),
    };
    let _power_events = PowerEvents::register(tx);
    let mut wish = Wish { mode: Mode::Off, display: false, since: Instant::now() };
    let screen_off_sleeps = modern_standby();

    loop {
        let (system, display) = wish.wanted(plugged_in());
        request.set(system, display);
        publish(&app, &wish, system, screen_off_sleeps);

        // Sleeps until a message arrives or the timer runs out.
        // shortcut: the timer counts awake time only, so a PC put to sleep by hand runs late
        // by however long it slept; compare against the wall clock if that ever matters.
        let msg = match wish.deadline() {
            Some(end) => rx.recv_timeout(end.saturating_duration_since(Instant::now())).unwrap_or_else(|e| match e {
                RecvTimeoutError::Timeout => Msg::SetMode(Mode::Off),
                RecvTimeoutError::Disconnected => Msg::Stop,
            }),
            None => rx.recv().unwrap_or(Msg::Stop),
        };
        match msg {
            Msg::SetMode(mode) => wish = Wish { mode, since: Instant::now(), ..wish },
            Msg::SetDisplay(display) => wish.display = display,
            Msg::PowerChanged => {}
            Msg::Stop => break,
        }
    }
    // `request` is dropped here, which releases it.
    tray::set_keep_awake(&app, None);
}

fn publish(app: &AppHandle, wish: &Wish, active: bool, screen_off_sleeps: bool) {
    let ends_at = wish.deadline().map(|end| {
        let end = SystemTime::now() + end.saturating_duration_since(Instant::now());
        end.duration_since(UNIX_EPOCH).unwrap_or_default().as_secs_f64() * 1000.0
    });
    let state = KeepAwakeState { mode: wish.mode, display: wish.display, active, ends_at, screen_off_sleeps };
    let shared = app.state::<Shared>();
    let mut current = shared.state.lock().unwrap();
    if *current != state {
        let line = match wish.deadline() {
            Some(end) => format!("Keeping awake until {}", clock_in(end.saturating_duration_since(Instant::now()))),
            None => "Keeping awake".into(),
        };
        tray::set_keep_awake(app, active.then_some(line));
        *current = state.clone();
        let _ = KeepAwakeChanged(state).emit(app);
    }
}

/// The time of day `left` from now, in the user's own format, like "3:40 PM" or "15:40". A clock time
/// rather than "40 min left", so the tray tooltip stays right without a timer updating it.
fn clock_in(left: Duration) -> String {
    let mut time = unsafe { GetLocalTime() };
    let now = time.wHour as u64 * 3600 + time.wMinute as u64 * 60 + time.wSecond as u64;
    let minute = (now + left.as_secs() + 30) / 60 % (24 * 60);
    (time.wHour, time.wMinute, time.wSecond, time.wMilliseconds) = ((minute / 60) as u16, (minute % 60) as u16, 0, 0);
    let mut text = [0u16; 32];
    let len = unsafe { GetTimeFormatEx(PCWSTR::null(), TIME_NOSECONDS, Some(&time), PCWSTR::null(), Some(&mut text)) };
    String::from_utf16_lossy(&text[..(len as usize).saturating_sub(1)])
}

/// Modern Standby ("S0 low power idle", what `powercfg /a` lists), common on recent laptops.
fn modern_standby() -> bool {
    let mut caps = SYSTEM_POWER_CAPABILITIES::default();
    let read = unsafe { GetPwrCapabilities(&mut caps) };
    read && caps.AoAc
}

/// On mains power. A desktop PC (no battery) or an unknown state counts as plugged in.
fn plugged_in() -> bool {
    let mut status = SYSTEM_POWER_STATUS::default();
    // ACLineStatus: 0 battery, 1 mains, 255 unknown.
    unsafe { GetSystemPowerStatus(&mut status) }.is_err() || status.ACLineStatus != 0
}

/// Traywise's power request. `powercfg /requests` lists it, with the reason below, while it's set.
struct PowerRequest {
    handle: HANDLE,
    system: bool,
    display: bool,
}

impl PowerRequest {
    fn new() -> Result<Self> {
        // Windows copies the reason when the request is created.
        let mut reason: Vec<u16> = "Keep awake is on in Traywise".encode_utf16().chain([0]).collect();
        let context = REASON_CONTEXT {
            Version: POWER_REQUEST_CONTEXT_VERSION,
            Flags: POWER_REQUEST_CONTEXT_SIMPLE_STRING,
            Reason: REASON_CONTEXT_0 { SimpleReasonString: PWSTR(reason.as_mut_ptr()) },
        };
        let handle = unsafe { PowerCreateRequest(&context)? };
        Ok(Self { handle, system: false, display: false })
    }

    fn set(&mut self, system: bool, display: bool) {
        update(self.handle, PowerRequestSystemRequired, &mut self.system, system);
        update(self.handle, PowerRequestDisplayRequired, &mut self.display, display);
    }
}

/// Windows counts set and clear calls, so only make one when the state really changes.
fn update(handle: HANDLE, kind: POWER_REQUEST_TYPE, current: &mut bool, wanted: bool) {
    if *current == wanted {
        return;
    }
    let result = unsafe { if wanted { PowerSetRequest(handle, kind) } else { PowerClearRequest(handle, kind) } };
    match result {
        Ok(()) => *current = wanted,
        Err(e) => eprintln!("keep-awake: {e}"),
    }
}

impl Drop for PowerRequest {
    fn drop(&mut self) {
        self.set(false, false);
        let _ = unsafe { CloseHandle(self.handle) };
    }
}

/// Tells the worker whenever the PC switches between mains and battery. Windows calls
/// `on_power_change` on one of its own threads; it only posts a message.
struct PowerEvents {
    handle: HPOWERNOTIFY,
    // Kept alive for as long as Windows may call back with pointers into them.
    _params: Box<DEVICE_NOTIFY_SUBSCRIBE_PARAMETERS>,
    _tx: Box<Sender<Msg>>,
}

impl PowerEvents {
    fn register(tx: Sender<Msg>) -> Option<Self> {
        let tx = Box::new(tx);
        let mut params = Box::new(DEVICE_NOTIFY_SUBSCRIBE_PARAMETERS {
            Callback: Some(on_power_change),
            Context: &*tx as *const Sender<Msg> as *mut c_void,
        });
        let mut handle = ptr::null_mut();
        let recipient = HANDLE(&mut *params as *mut DEVICE_NOTIFY_SUBSCRIBE_PARAMETERS as *mut c_void);
        let registered = unsafe {
            PowerSettingRegisterNotification(&GUID_ACDC_POWER_SOURCE, DEVICE_NOTIFY_CALLBACK, recipient, &mut handle)
        };
        if let Err(e) = registered.ok() {
            eprintln!("keep-awake: won't notice mains/battery changes: {e}");
            return None;
        }
        Some(Self { handle: HPOWERNOTIFY(handle as isize), _params: params, _tx: tx })
    }
}

unsafe extern "system" fn on_power_change(context: *const c_void, _: u32, _: *const c_void) -> u32 {
    let tx = unsafe { &*(context as *const Sender<Msg>) };
    let _ = tx.send(Msg::PowerChanged);
    0
}

impl Drop for PowerEvents {
    fn drop(&mut self) {
        let _ = unsafe { PowerSettingUnregisterNotification(self.handle) };
    }
}
