//! Core Audio pieces shared by the audio modules (the volume mixer and audio devices).

use windows::Win32::Devices::FunctionDiscovery::PKEY_Device_FriendlyName;
use windows::Win32::Media::Audio::{IMMDevice, IMMDeviceEnumerator, MMDeviceEnumerator};
use windows::Win32::System::Com::StructuredStorage::{PropVariantClear, PropVariantToStringAlloc};
use windows::Win32::System::Com::{
    CLSCTX_ALL, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx, CoTaskMemFree, CoUninitialize, STGM_READ,
};
use windows::core::{PWSTR, Result};

/// Runs `work` with COM set up on this thread, as every Core Audio call needs. Logs a failure.
pub fn with_com(label: &str, work: impl FnOnce() -> Result<()>) {
    if let Err(e) = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) }.ok() {
        return eprintln!("{label}: COM init failed: {e}");
    }
    if let Err(e) = work() {
        eprintln!("{label} stopped on an error: {e}");
    }
    unsafe { CoUninitialize() };
}

/// Windows' list of audio devices, which also sends device notifications.
pub fn enumerator() -> Result<IMMDeviceEnumerator> {
    unsafe { CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL) }
}

/// Copies a string Windows allocated for us, then frees it.
pub fn take_string(s: PWSTR) -> String {
    let text = unsafe { s.to_string() }.unwrap_or_default();
    unsafe { CoTaskMemFree(Some(s.0 as *const _)) };
    text
}

/// A device's id, which stays the same across restarts and replugging.
pub fn device_id(device: &IMMDevice) -> Result<String> {
    Ok(take_string(unsafe { device.GetId()? }))
}

/// The name Windows shows for a device, like "Speakers (Realtek Audio)".
pub fn friendly_name(device: &IMMDevice) -> Result<String> {
    unsafe {
        let store = device.OpenPropertyStore(STGM_READ)?;
        let mut value = store.GetValue(&PKEY_Device_FriendlyName)?;
        let name = PropVariantToStringAlloc(&value).map(take_string);
        let _ = PropVariantClear(&mut value);
        name
    }
}
