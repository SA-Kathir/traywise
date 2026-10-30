//! Quick toggles: dark mode, desktop icons, and locking the screen.
//!
//! Nothing runs in the background. Each tile reads or changes a Windows setting when it's used,
//! and the state is re-read each time the panel opens, in case it was changed elsewhere.

use serde::Serialize;
use tauri::AppHandle;
use tauri_specta::Event;
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
    /// Apps and Windows itself (taskbar, Start) in dark mode.
    pub dark: bool,
    pub desktop_icons: bool,
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
            publish(&self.app);
        }
    }
}

fn publish(app: &AppHandle) {
    let _ = QuickTogglesChanged(read()).emit(app);
}

fn read() -> QuickTogglesState {
    QuickTogglesState {
        dark: dword(PERSONALIZE, w!("AppsUseLightTheme")) == Some(0),
        // If the desktop can't be found, say shown: that's Windows' default.
        desktop_icons: desktop_icons().is_none_or(|list| unsafe { IsWindowVisible(list) }.as_bool()),
    }
}

#[tauri::command]
#[specta::specta]
pub fn quick_toggles_state() -> QuickTogglesState {
    read()
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
    publish(&app);
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
