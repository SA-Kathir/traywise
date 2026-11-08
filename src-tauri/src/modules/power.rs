//! Power: Windows' power mode (Settings › Power & battery › Power mode), the battery, and whether
//! Energy saver is on.
//!
//! No thread of its own. While the module is on, Windows calls back (on its own threads) when the
//! power mode or Energy saver changes, the PC is plugged in or unplugged, or the battery level moves
//! by a percent; each callback reads the state again and sends it to the UI.

use std::ffi::c_void;
use std::ptr;
use std::sync::atomic::{AtomicU8, Ordering};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::AppHandle;
use tauri_specta::Event;
use windows::Win32::Foundation::{HANDLE, HLOCAL, LocalFree};
use windows::Win32::System::Registry::{HKEY_LOCAL_MACHINE, RRF_RT_REG_DWORD, RegGetValueW};
use windows::Win32::System::Power::{
    DEVICE_NOTIFY_SUBSCRIBE_PARAMETERS, EFFECTIVE_POWER_MODE, EFFECTIVE_POWER_MODE_V2, EffectivePowerModeBatterySaver, HPOWERNOTIFY,
    PowerGetActiveScheme, PowerReadDCValueIndex, PowerRegisterForEffectivePowerModeNotifications, PowerSettingRegisterNotification,
    PowerSettingUnregisterNotification, PowerUnregisterFromEffectivePowerModeNotifications,
};
use windows::Win32::System::SystemServices::{
    GUID_ACDC_POWER_SOURCE, GUID_BATTERY_PERCENTAGE_REMAINING, GUID_ENERGY_SAVER_BATTERY_THRESHOLD, GUID_ENERGY_SAVER_SUBGROUP,
};
use windows::Win32::UI::WindowsAndMessaging::DEVICE_NOTIFY_CALLBACK;
use windows::core::{GUID, w};

use crate::module::{Module, ModuleId};
use crate::modules::system_monitor::{Battery, battery};

/// The three choices in Settings › Power & battery › Power mode.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum PowerMode {
    Efficiency,
    Balanced,
    Performance,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PowerState {
    /// None when Windows doesn't offer power modes: the active power plan isn't Balanced.
    pub mode: Option<PowerMode>,
    /// None on a PC without a battery.
    pub battery: Option<Battery>,
    /// Windows' Energy saver (battery saver before Windows 11 24H2).
    pub energy_saver: bool,
    /// The battery level it turns on at by itself: 0 never, 100 always. None if unknown.
    pub energy_saver_at: Option<u8>,
}

/// Sent whenever Windows reports a change while the module is on.
#[derive(Clone, Serialize, specta::Type, tauri_specta::Event)]
pub struct PowerChanged(pub PowerState);

// The power mode is an "overlay" on the Balanced plan. These ids are what Windows' own slider uses;
// Balanced has none (all zeros).
const EFFICIENCY: GUID = GUID::from_u128(0x961cc777_2547_4f9d_8174_7d86181b8a7a);
const PERFORMANCE: GUID = GUID::from_u128(0xded574b5_45a0_4f42_8737_46345c09c238);
const BALANCED_PLAN: GUID = GUID::from_u128(0x381b4222_f694_41f0_9685_ff5bb260df2e);

// Exported by powrprof.dll and used by Windows' own Settings, but missing from Microsoft's headers.
windows_core::link!("powrprof.dll" "system" fn PowerGetEffectiveOverlayScheme(overlay: *mut GUID) -> u32);
windows_core::link!("powrprof.dll" "system" fn PowerSetActiveOverlayScheme(overlay: GUID) -> u32);

fn mode_of(overlay: GUID) -> PowerMode {
    match overlay {
        EFFICIENCY => PowerMode::Efficiency,
        PERFORMANCE => PowerMode::Performance,
        // Includes Windows 10's "Better performance", which Windows 11 shows as Balanced.
        _ => PowerMode::Balanced,
    }
}

fn overlay_of(mode: PowerMode) -> GUID {
    match mode {
        PowerMode::Efficiency => EFFICIENCY,
        PowerMode::Balanced => GUID::zeroed(),
        PowerMode::Performance => PERFORMANCE,
    }
}

fn read() -> PowerState {
    let plan = active_plan();
    let battery = battery();
    let at = plan.and_then(|plan| {
        let mut percent = 0;
        let read = unsafe {
            PowerReadDCValueIndex(None, Some(&plan), Some(&GUID_ENERGY_SAVER_SUBGROUP), Some(&GUID_ENERGY_SAVER_BATTERY_THRESHOLD), &mut percent)
        };
        (read == 0).then_some(percent.min(100) as u8)
    });
    PowerState { mode: plan.and_then(mode), energy_saver: energy_saver_from(battery.as_ref(), at), battery, energy_saver_at: at }
}

/// What Windows itself says the power mode is: Energy saver shows up as a mode of its own, and that is
/// the real state (the guesses below can't tell "switched off by hand" from "turns on by itself on
/// battery"). 0: not heard yet, 1: Energy saver off, 2: on.
static SAVER: AtomicU8 = AtomicU8::new(0);

unsafe extern "system" fn on_effective_mode(mode: EFFECTIVE_POWER_MODE, _: *const c_void) {
    SAVER.store(if mode == EffectivePowerModeBatterySaver { 2 } else { 1 }, Ordering::Relaxed);
}

/// Starts listening for it for as long as the app runs (the Glass look needs it even with the Power
/// module off). Windows answers right away; this waits a moment for that first answer.
pub fn watch_energy_saver() {
    let mut handle = ptr::null_mut();
    // shortcut: never unregistered; it lives as long as the app does.
    let registered = unsafe { PowerRegisterForEffectivePowerModeNotifications(EFFECTIVE_POWER_MODE_V2, Some(on_effective_mode), None, &mut handle) };
    if registered.is_ok() {
        for _ in 0..30 {
            if SAVER.load(Ordering::Relaxed) != 0 {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}

fn energy_saver_from(battery: Option<&Battery>, at: Option<u8>) -> bool {
    match SAVER.load(Ordering::Relaxed) {
        1 => false,
        2 => true,
        _ => saver_on(switched_on(), battery, at),
    }
}

/// Whether Energy saver is on: switched on by hand, or on battery at or below the level it turns
/// on at (0 never; 100 is Settings' "Always use energy saver" on battery, which doesn't touch the
/// switch). `charging` is Windows' "on mains", so a full battery that's plugged in counts too.
fn saver_on(switched_on: bool, battery: Option<&Battery>, at: Option<u8>) -> bool {
    switched_on || matches!((battery, at), (Some(b), Some(at)) if at > 0 && !b.charging && b.percent <= at)
}

fn active_plan() -> Option<GUID> {
    let mut plan: *mut GUID = ptr::null_mut();
    unsafe { PowerGetActiveScheme(None, &mut plan) }.ok().ok()?;
    let guid = unsafe { *plan };
    unsafe { LocalFree(Some(HLOCAL(plan.cast()))) };
    Some(guid)
}

fn mode(plan: GUID) -> Option<PowerMode> {
    // shortcut: OEM plans based on Balanced count as "no power modes", as Windows' slider hides
    // there too; revisit if someone reports the tile missing on such a plan.
    let mut overlay = GUID::zeroed();
    (plan == BALANCED_PLAN && unsafe { PowerGetEffectiveOverlayScheme(&mut overlay) } == 0).then(|| mode_of(overlay))
}

/// Energy saver's switch in Settings. Neither GetSystemPowerStatus nor WinRT's EnergySaverStatus
/// sees it (both stayed "off" with it on, on build 26200); Windows' own state in the registry
/// does: 1 on, 2 off.
fn switched_on() -> bool {
    let (mut value, mut size) = (0u32, 4u32);
    let status = unsafe {
        RegGetValueW(
            HKEY_LOCAL_MACHINE,
            w!(r"SYSTEM\CurrentControlSet\Control\Power"),
            w!("EnergySaverState"),
            RRF_RT_REG_DWORD,
            None,
            Some(&mut value as *mut u32 as *mut _),
            Some(&mut size),
        )
    };
    status.is_ok() && value == 1
}

/// Whether Energy saver is on right now (Glass asks: Windows draws no blur under it).
pub fn energy_saver() -> bool {
    read().energy_saver
}

fn publish(app: &AppHandle) {
    let _ = PowerChanged(read()).emit(app);
}

pub struct Power {
    app: AppHandle,
    events: Option<Events>,
}

impl Power {
    pub fn new(app: AppHandle) -> Self {
        Self { app, events: None }
    }
}

impl Module for Power {
    fn id(&self) -> ModuleId {
        ModuleId::Power
    }

    fn start(&mut self) -> Result<(), String> {
        self.events = Some(Events::register(self.app.clone()));
        println!("power started");
        Ok(())
    }

    fn stop(&mut self) {
        self.events = None; // dropping unregisters
        println!("power stopped");
    }
}

/// Windows' callbacks while the module is on. They point into `_app`, which is dropped only after
/// `drop` has unregistered them.
struct Events {
    _app: Box<AppHandle>,
    params: Box<DEVICE_NOTIFY_SUBSCRIBE_PARAMETERS>,
    settings: Vec<HPOWERNOTIFY>,
    mode: *mut c_void,
}

// The raw handles are only used to unregister, from whichever thread drops this.
unsafe impl Send for Events {}

impl Events {
    fn register(app: AppHandle) -> Self {
        let app = Box::new(app);
        let context = &*app as *const AppHandle as *mut c_void;
        let mut events = Self {
            params: Box::new(DEVICE_NOTIFY_SUBSCRIBE_PARAMETERS { Callback: Some(on_setting), Context: context }),
            _app: app,
            settings: Vec::new(),
            mode: ptr::null_mut(),
        };
        let recipient = HANDLE(&mut *events.params as *mut DEVICE_NOTIFY_SUBSCRIBE_PARAMETERS as *mut c_void);
        for setting in [GUID_ACDC_POWER_SOURCE, GUID_BATTERY_PERCENTAGE_REMAINING, GUID_ENERGY_SAVER_BATTERY_THRESHOLD] {
            let mut handle = ptr::null_mut();
            match unsafe { PowerSettingRegisterNotification(&setting, DEVICE_NOTIFY_CALLBACK, recipient, &mut handle) }.ok() {
                Ok(()) => events.settings.push(HPOWERNOTIFY(handle as isize)),
                Err(e) => eprintln!("power: won't notice battery changes: {e}"),
            }
        }
        // Also covers Energy saver, which Windows reports as a power mode of its own.
        let registered =
            unsafe { PowerRegisterForEffectivePowerModeNotifications(EFFECTIVE_POWER_MODE_V2, Some(on_mode), Some(context), &mut events.mode) };
        if let Err(e) = registered {
            eprintln!("power: won't notice power mode changes: {e}");
        }
        events
    }
}

impl Drop for Events {
    fn drop(&mut self) {
        for handle in self.settings.drain(..) {
            let _ = unsafe { PowerSettingUnregisterNotification(handle) };
        }
        if !self.mode.is_null() {
            let _ = unsafe { PowerUnregisterFromEffectivePowerModeNotifications(self.mode) };
        }
    }
}

unsafe extern "system" fn on_setting(context: *const c_void, _: u32, _: *const c_void) -> u32 {
    publish(unsafe { &*(context as *const AppHandle) });
    0
}

unsafe extern "system" fn on_mode(mode: EFFECTIVE_POWER_MODE, context: *const c_void) {
    // Up to date before the state is read and sent (the other listener may not have run yet).
    unsafe { on_effective_mode(mode, ptr::null()) };
    publish(unsafe { &*(context as *const AppHandle) });
}

#[tauri::command]
#[specta::specta]
pub fn power_state() -> PowerState {
    read()
}

#[tauri::command]
#[specta::specta]
pub fn power_set_mode(app: AppHandle, mode: PowerMode) -> Result<(), String> {
    match unsafe { PowerSetActiveOverlayScheme(overlay_of(mode)) } {
        0 => {
            publish(&app);
            Ok(())
        }
        e => Err(format!("Windows didn't change the power mode (error {e})")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn energy_saver() {
        let on_battery = |percent| Some(Battery { percent, charging: false, seconds_left: None });
        let plugged_in = Some(Battery { percent: 10, charging: true, seconds_left: None });
        assert!(saver_on(true, None, None), "switched on by hand");
        assert!(saver_on(false, on_battery(80).as_ref(), Some(100)), "always on battery");
        assert!(!saver_on(false, plugged_in.as_ref(), Some(100)), "plugged in");
        assert!(saver_on(false, on_battery(30).as_ref(), Some(30)), "at the level");
        assert!(!saver_on(false, on_battery(31).as_ref(), Some(30)), "above it");
        assert!(!saver_on(false, on_battery(0).as_ref(), Some(0)), "never");
        assert!(!saver_on(false, None, Some(100)), "no battery");
    }

    #[test]
    fn modes_round_trip() {
        for mode in [PowerMode::Efficiency, PowerMode::Balanced, PowerMode::Performance] {
            assert_eq!(mode_of(overlay_of(mode)), mode);
        }
        let better_performance = GUID::from_u128(0x3af9b8d9_7c97_431d_ad78_34a8bfea439f);
        assert_eq!(mode_of(better_performance), PowerMode::Balanced);
    }
}
