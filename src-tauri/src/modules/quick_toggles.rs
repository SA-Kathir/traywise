//! Quick toggles: Wi-Fi, Bluetooth, dark mode, desktop icons, and locking the screen.
//!
//! Nothing runs in the background. Each tile reads or changes a Windows setting when it's used,
//! and the state is re-read each time the panel opens, in case it was changed elsewhere.

use serde::{Deserialize, Serialize};
use tauri::AppHandle;
use tauri_specta::Event;
use windows::Devices::Radios::{Radio, RadioAccessStatus, RadioKind, RadioState};
use windows::Win32::System::Com::CoIncrementMTAUsage;
use windows::Win32::Foundation::{ERROR_SUCCESS, HWND, LPARAM, WPARAM};
use windows::Win32::System::Registry::{HKEY_CURRENT_USER, REG_DWORD, RRF_RT_REG_DWORD, RegGetValueW, RegSetKeyValueW};
use windows::Win32::System::Shutdown::LockWorkStation;
use windows::Win32::UI::WindowsAndMessaging::{
    FindWindowExW, FindWindowW, GetParent, HWND_BROADCAST, IsWindowVisible, SMTO_ABORTIFHUNG, SendMessageTimeoutW,
    SendMessageW, WM_COMMAND, WM_SETTINGCHANGE,
};
use windows::core::{PCWSTR, w};

use crate::module::{Module, ModuleId};

const PERSONALIZE: PCWSTR = w!(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize");
/// The desktop's own "Show desktop icons" menu command.
const TOGGLE_DESKTOP_ICONS: usize = 0x7402;

#[derive(Clone, Debug, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct QuickTogglesState {
    /// None when this PC has no Wi-Fi radio (or Windows wouldn't say).
    pub wifi: Option<bool>,
    /// None when this PC has no Bluetooth radio.
    pub bluetooth: Option<bool>,
    /// Apps and Windows itself (taskbar, Start) in dark mode.
    pub dark: bool,
    pub desktop_icons: bool,
}

/// The radios the Wi-Fi and Bluetooth tiles switch.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum RadioId {
    Wifi,
    Bluetooth,
}

/// Sent after a toggle, and each time the panel opens.
#[derive(Clone, Serialize, specta::Type, tauri_specta::Event)]
pub struct QuickTogglesChanged(pub QuickTogglesState);

pub struct QuickToggles {
    app: AppHandle,
}

impl QuickToggles {
    pub fn new(app: AppHandle) -> Self {
        Self { app }
    }
}

impl Module for QuickToggles {
    fn id(&self) -> ModuleId {
        ModuleId::QuickToggles
    }

    fn start(&mut self) -> Result<(), String> {
        Ok(()) // nothing to start: no thread, no subscriptions
    }

    fn stop(&mut self) {}

    fn panel_visible(&mut self, visible: bool) {
        if visible {
            // Off the UI's thread: Windows can take a moment to list the radios.
            let app = self.app.clone();
            std::thread::spawn(move || publish(&app));
        }
    }
}

fn publish(app: &AppHandle) {
    let _ = QuickTogglesChanged(read()).emit(app);
}

/// Waits on Windows; call off the UI's thread.
fn read() -> QuickTogglesState {
    let radio_on = |kind| radio(kind).ok().flatten().map(|r| r.State() == Ok(RadioState::On));
    QuickTogglesState {
        wifi: radio_on(RadioKind::WiFi),
        bluetooth: radio_on(RadioKind::Bluetooth),
        dark: dword(PERSONALIZE, w!("AppsUseLightTheme")) == Some(0),
        // If the desktop can't be found, say shown: that's Windows' default.
        desktop_icons: desktop_icons().is_none_or(|list| unsafe { IsWindowVisible(list) }.as_bool()),
    }
}

// Async, so it runs off the UI's thread.
#[tauri::command]
#[specta::specta]
pub async fn quick_toggles_state() -> QuickTogglesState {
    read()
}

/// Switches Wi-Fi or Bluetooth on or off, like the toggles in Windows' quick settings.
#[tauri::command]
#[specta::specta]
pub async fn quick_toggles_set_radio(app: AppHandle, radio_id: RadioId, on: bool) -> Result<(), String> {
    let kind = match radio_id {
        RadioId::Wifi => RadioKind::WiFi,
        RadioId::Bluetooth => RadioKind::Bluetooth,
    };
    let result = set_radio(kind, on).map_err(|e| e.to_string()).and_then(|r| r);
    publish(&app);
    result
}

/// The outer error is Windows failing; the inner one, Windows saying no.
fn set_radio(kind: RadioKind, on: bool) -> windows::core::Result<Result<(), String>> {
    mta();
    if Radio::RequestAccessAsync()?.join()? != RadioAccessStatus::Allowed {
        return Ok(Err("Windows won't let apps switch radios (Settings › Privacy › Radios)".into()));
    }
    let Some(r) = radio(kind)? else { return Ok(Err("no such radio on this PC".into())) };
    let status = r.SetStateAsync(if on { RadioState::On } else { RadioState::Off })?.join()?;
    Ok(if status == RadioAccessStatus::Allowed { Ok(()) } else { Err(format!("Windows refused ({status:?})")) })
}

/// This PC's first radio of a kind, if it has one. Call off the UI's thread.
fn radio(kind: RadioKind) -> windows::core::Result<Option<Radio>> {
    mta();
    Ok(Radio::GetRadiosAsync()?.join()?.into_iter().find(|r| r.Kind() == Ok(kind)))
}

/// Keeps Windows' shared multithreaded apartment open for the app's lifetime, so any background
/// thread can make the radio calls without setting up (and tearing down) COM around each one.
fn mta() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        let _ = unsafe { CoIncrementMTAUsage() };
    });
}

/// Switches apps and Windows itself between dark and light, like Settings > Personalization > Colors.
#[tauri::command]
#[specta::specta]
pub fn quick_toggles_set_dark(app: AppHandle, dark: bool) -> Result<(), String> {
    let light = u32::from(!dark);
    set_dword(PERSONALIZE, w!("AppsUseLightTheme"), light)?;
    set_dword(PERSONALIZE, w!("SystemUsesLightTheme"), light)?;
    // Tell every window the theme changed. A window that's hung can hold this up, so not on the
    // UI's thread.
    std::thread::spawn(move || {
        unsafe {
            SendMessageTimeoutW(
                HWND_BROADCAST,
                WM_SETTINGCHANGE,
                WPARAM(0),
                LPARAM(w!("ImmersiveColorSet").as_ptr() as isize),
                SMTO_ABORTIFHUNG,
                200,
                None,
            )
        };
        publish(&app);
    });
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn quick_toggles_set_desktop_icons(app: AppHandle, show: bool) -> Result<(), String> {
    let list = desktop_icons().ok_or("couldn't find the desktop")?;
    if unsafe { IsWindowVisible(list) }.as_bool() != show {
        // The desktop's own command, so Windows remembers the choice as if it were picked by hand.
        let view = unsafe { GetParent(list) }.map_err(|e| e.to_string())?;
        unsafe { SendMessageW(view, WM_COMMAND, Some(WPARAM(TOGGLE_DESKTOP_ICONS)), None) };
    }
    std::thread::spawn(move || publish(&app)); // off the UI's thread: reading the radios waits on Windows
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn quick_toggles_lock() -> Result<(), String> {
    unsafe { LockWorkStation() }.map_err(|e| e.to_string())
}

/// The desktop's icon list. It lives in Progman, or in a WorkerW window when a wallpaper
/// slideshow or animation is running.
fn desktop_icons() -> Option<HWND> {
    let in_view = |parent: HWND| unsafe {
        let view = FindWindowExW(Some(parent), None, w!("SHELLDLL_DefView"), PCWSTR::null()).ok()?;
        FindWindowExW(Some(view), None, w!("SysListView32"), PCWSTR::null()).ok()
    };
    let progman = unsafe { FindWindowW(w!("Progman"), PCWSTR::null()) }.ok()?;
    if let Some(list) = in_view(progman) {
        return Some(list);
    }
    let mut worker = None;
    loop {
        let next = unsafe { FindWindowExW(None, worker, w!("WorkerW"), PCWSTR::null()) }.ok()?;
        if let Some(list) = in_view(next) {
            return Some(list);
        }
        worker = Some(next);
    }
}

fn dword(key: PCWSTR, name: PCWSTR) -> Option<u32> {
    let (mut value, mut size) = (0u32, 4u32);
    let status = unsafe {
        RegGetValueW(HKEY_CURRENT_USER, key, name, RRF_RT_REG_DWORD, None, Some(&mut value as *mut u32 as *mut _), Some(&mut size))
    };
    (status == ERROR_SUCCESS).then_some(value)
}

fn set_dword(key: PCWSTR, name: PCWSTR, value: u32) -> Result<(), String> {
    let status =
        unsafe { RegSetKeyValueW(HKEY_CURRENT_USER, key, name, REG_DWORD.0, Some(&value as *const u32 as *const _), 4) };
    status.ok().map_err(|e| format!("couldn't change the setting: {e}"))
}

#[cfg(test)]
mod tests {
    /// Prints this PC's toggles, radios included, without changing anything. By hand:
    /// `cargo test quick_toggles_live -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn quick_toggles_live() {
        println!("{:?}", super::read());
    }
}
