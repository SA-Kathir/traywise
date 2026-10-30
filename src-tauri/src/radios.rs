//! Windows' Wi-Fi and Bluetooth radios, shared by the Wi-Fi and Bluetooth modules, and the
//! Windows Settings pages they link to.

use serde::Deserialize;
use windows::Devices::Radios::{Radio, RadioAccessStatus, RadioKind, RadioState};
use windows::Win32::System::Com::CoIncrementMTAUsage;
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
use windows::core::{HSTRING, PCWSTR, w};

/// Keeps Windows' shared multithreaded apartment open for the app's lifetime, so any background
/// thread can make WinRT calls (radios, Wi-Fi, Bluetooth) without setting up and tearing down COM
/// around each one. Call before them; never on the UI's thread.
pub fn mta() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        let _ = unsafe { CoIncrementMTAUsage() };
    });
}

/// Runs work that waits on Windows (a Wi-Fi scan takes seconds) on a thread meant for waiting,
/// not on the async threads every command shares.
pub async fn blocking<T: Send + 'static>(work: impl FnOnce() -> T + Send + 'static) -> T {
    tauri::async_runtime::spawn_blocking(work).await.expect("a blocking task panicked")
}

/// Whether this PC's radio of a kind is on; None if it has none.
pub fn radio_on(kind: RadioKind) -> Option<bool> {
    radio(kind).ok().flatten().map(|r| r.State() == Ok(RadioState::On))
}

/// Switches a radio on or off, like the toggles in Windows' quick settings.
pub fn set_radio(kind: RadioKind, on: bool) -> Result<(), String> {
    let set = || -> windows::core::Result<Result<(), String>> {
        if Radio::RequestAccessAsync()?.join()? != RadioAccessStatus::Allowed {
            return Ok(Err("Windows won't let apps switch radios (Settings › Privacy › Radios)".into()));
        }
        let Some(r) = radio(kind)? else { return Ok(Err("no such radio on this PC".into())) };
        let status = r.SetStateAsync(if on { RadioState::On } else { RadioState::Off })?.join()?;
        Ok(if status == RadioAccessStatus::Allowed { Ok(()) } else { Err(format!("Windows refused ({status:?})")) })
    };
    mta();
    set().map_err(|e| e.to_string()).and_then(|r| r)
}

/// This PC's first radio of a kind, if it has one.
fn radio(kind: RadioKind) -> windows::core::Result<Option<Radio>> {
    mta();
    Ok(Radio::GetRadiosAsync()?.join()?.into_iter().find(|r| r.Kind() == Ok(kind)))
}

/// The Windows Settings pages the UI can open: only these, never an arbitrary link.
#[derive(Clone, Copy, Debug, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum SettingsPage {
    /// Privacy › Location: where Wi-Fi names are allowed.
    Location,
    /// The Wi-Fi network list, for networks that need more than a password.
    Wifi,
    /// Bluetooth & devices, to pair a new device.
    Bluetooth,
}

#[tauri::command]
#[specta::specta]
pub fn open_settings(page: SettingsPage) {
    let uri = match page {
        SettingsPage::Location => w!("ms-settings:privacy-location"),
        SettingsPage::Wifi => w!("ms-availablenetworks:"),
        SettingsPage::Bluetooth => w!("ms-settings:bluetooth"),
    };
    unsafe { ShellExecuteW(None, &HSTRING::from("open"), uri, PCWSTR::null(), PCWSTR::null(), SW_SHOWNORMAL) };
}
