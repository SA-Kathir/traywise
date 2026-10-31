//! Power: Windows' power mode (Settings › Power & battery › Power mode), the battery, and whether
//! Energy saver is on.
//!
//! No thread of its own. While the module is on, Windows calls back (on its own threads) when the
//! power mode or Energy saver changes, the PC is plugged in or unplugged, or the battery level moves
//! by a percent; each callback reads the state again and sends it to the UI.

use std::ffi::c_void;
use std::ptr;

use serde::{Deserialize, Serialize};
use tauri::AppHandle;
use tauri_specta::Event;
use windows::Win32::Foundation::{HANDLE, HLOCAL, LocalFree};
use windows::Win32::System::Power::{
    DEVICE_NOTIFY_SUBSCRIBE_PARAMETERS, EFFECTIVE_POWER_MODE, EFFECTIVE_POWER_MODE_V2, GetSystemPowerStatus, HPOWERNOTIFY,
    PowerGetActiveScheme, PowerRegisterForEffectivePowerModeNotifications, PowerSettingRegisterNotification,
    PowerSettingUnregisterNotification, PowerUnregisterFromEffectivePowerModeNotifications, SYSTEM_POWER_STATUS,
};
use windows::Win32::System::SystemServices::{GUID_ACDC_POWER_SOURCE, GUID_BATTERY_PERCENTAGE_REMAINING};
use windows::Win32::UI::WindowsAndMessaging::DEVICE_NOTIFY_CALLBACK;
use windows::core::GUID;

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
    let mut status = SYSTEM_POWER_STATUS::default();
    let saver = unsafe { GetSystemPowerStatus(&mut status) }.is_ok() && status.SystemStatusFlag == 1;
    PowerState { mode: mode(), battery: battery(), energy_saver: saver }
}

fn mode() -> Option<PowerMode> {
    // shortcut: OEM plans based on Balanced count as "no power modes", as Windows' slider hides
    // there too; revisit if someone reports the tile missing on such a plan.
    let mut plan: *mut GUID = ptr::null_mut();
    if unsafe { PowerGetActiveScheme(None, &mut plan) }.is_err() {
        return None;
    }
    let balanced = unsafe { *plan } == BALANCED_PLAN;
    unsafe { LocalFree(Some(HLOCAL(plan.cast()))) };
    let mut overlay = GUID::zeroed();
    (balanced && unsafe { PowerGetEffectiveOverlayScheme(&mut overlay) } == 0).then(|| mode_of(overlay))
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
        for setting in [GUID_ACDC_POWER_SOURCE, GUID_BATTERY_PERCENTAGE_REMAINING] {
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

unsafe extern "system" fn on_mode(_: EFFECTIVE_POWER_MODE, context: *const c_void) {
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
    fn modes_round_trip() {
        for mode in [PowerMode::Efficiency, PowerMode::Balanced, PowerMode::Performance] {
            assert_eq!(mode_of(overlay_of(mode)), mode);
        }
        let better_performance = GUID::from_u128(0x3af9b8d9_7c97_431d_ad78_34a8bfea439f);
        assert_eq!(mode_of(better_performance), PowerMode::Balanced);
    }
}
