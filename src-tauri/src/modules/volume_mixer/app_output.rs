//! Which output device each app plays through. Windows has no public API for this; its own
//! Settings › Sound › Volume mixer uses the undocumented `IAudioPolicyConfigFactory`, as do
//! EarTrumpet and SoundSwitch. Windows remembers the choice per app, across restarts.
//!
//! The interface id changed in Windows 10 21H2; Traywise is Windows 11 only, so only the newer one
//! is tried. If even that's missing, `Router::new` returns None and the feature stays hidden.

#![allow(non_snake_case)] // method names must match Windows' interface

use std::ffi::c_void;

use windows::Win32::Media::Audio::{EDataFlow, ERole, eConsole, eMultimedia, eRender};
use windows::Win32::System::WinRT::RoGetActivationFactory;
use windows::core::{HRESULT, HSTRING, IUnknown, IUnknown_Vtbl, Result, interface};

// Only the last three methods are called. The ones before them are declared so those sit at the
// right place in the interface; their parameters are left out. It's a WinRT interface, so the
// first three are IInspectable's (the `interface` macro can only build on IUnknown).
#[interface("ab3d4648-e242-459f-b02f-541c70306324")]
unsafe trait IAudioPolicyConfigFactory: IUnknown {
    fn GetIids(&self) -> HRESULT;
    fn GetRuntimeClassName(&self) -> HRESULT;
    fn GetTrustLevel(&self) -> HRESULT;
    fn add_CtxVolumeChange(&self) -> HRESULT;
    fn remove_CtxVolumeChanged(&self) -> HRESULT;
    fn add_RingerVibrateStateChanged(&self) -> HRESULT;
    fn remove_RingerVibrateStateChange(&self) -> HRESULT;
    fn SetVolumeGroupGainForId(&self) -> HRESULT;
    fn GetVolumeGroupGainForId(&self) -> HRESULT;
    fn GetActiveVolumeGroupForEndpointId(&self) -> HRESULT;
    fn GetVolumeGroupsForEndpoint(&self) -> HRESULT;
    fn GetCurrentVolumeContext(&self) -> HRESULT;
    fn SetVolumeGroupMuteForId(&self) -> HRESULT;
    fn GetVolumeGroupMuteForId(&self) -> HRESULT;
    fn SetRingerVibrateState(&self) -> HRESULT;
    fn GetRingerVibrateState(&self) -> HRESULT;
    fn SetPreferredChatApplication(&self) -> HRESULT;
    fn ResetPreferredChatApplication(&self) -> HRESULT;
    fn GetPreferredChatApplication(&self) -> HRESULT;
    fn GetCurrentChatApplications(&self) -> HRESULT;
    fn add_ChatContextChanged(&self) -> HRESULT;
    fn remove_ChatContextChanged(&self) -> HRESULT;
    /// `device` is a borrowed HSTRING handle; null means "follow the system default".
    fn SetPersistedDefaultAudioEndpoint(&self, pid: u32, flow: EDataFlow, role: ERole, device: *mut c_void) -> HRESULT;
    fn GetPersistedDefaultAudioEndpoint(&self, pid: u32, flow: EDataFlow, role: ERole, device: *mut HSTRING) -> HRESULT;
    fn ClearAllPersistedApplicationDefaultEndpoints(&self) -> HRESULT;
}

// This API names devices by their full device path, not the short id the rest of Core Audio uses.
const PREFIX: &str = r"\\?\SWD#MMDEVAPI#";
const RENDER_SUFFIX: &str = "#{e6327cad-dcec-4949-ae8a-991e976a79d2}";

pub struct Router(IAudioPolicyConfigFactory);

impl Router {
    /// None on Windows builds without the interface.
    pub fn new() -> Option<Self> {
        let name = HSTRING::from("Windows.Media.Internal.AudioPolicyConfig");
        unsafe { RoGetActivationFactory(&name) }
            .map_err(|e| eprintln!("volume-mixer: per-app output isn't available ({e})"))
            .ok()
            .map(Self)
    }

    /// The device an app is set to, as a Core Audio device id. None: it follows the default.
    pub fn get(&self, pid: u32) -> Option<String> {
        let mut path = HSTRING::new();
        // Fails with "parameter is incorrect" for an app that was never set to a device.
        unsafe { self.0.GetPersistedDefaultAudioEndpoint(pid, eRender, eMultimedia, &mut path).ok().ok()? };
        device_id(&path.to_string_lossy())
    }

    /// Sends an app to a device, or back to the default with None.
    pub fn set(&self, pid: u32, device: Option<&str>) -> Result<()> {
        let path = device.map_or_else(HSTRING::new, |id| HSTRING::from(device_path(id)));
        // Windows' own mixer sets both roles; games and media use one or the other.
        for role in [eConsole, eMultimedia] {
            unsafe { self.0.SetPersistedDefaultAudioEndpoint(pid, eRender, role, std::mem::transmute_copy(&path)).ok()? };
        }
        Ok(())
    }

    /// Sends every app back to the default.
    pub fn reset_all(&self) -> Result<()> {
        unsafe { self.0.ClearAllPersistedApplicationDefaultEndpoints().ok() }
    }
}

fn device_path(id: &str) -> String {
    format!("{PREFIX}{id}{RENDER_SUFFIX}")
}

/// The id inside a device path. None for an empty path (the app follows the default).
fn device_id(path: &str) -> Option<String> {
    let id = path.strip_prefix(PREFIX).unwrap_or(path);
    let id = id.strip_suffix(RENDER_SUFFIX).unwrap_or(id);
    (!id.is_empty()).then(|| id.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    const ID: &str = "{0.0.0.00000000}.{8c5d6a0b-1f2e-4c3d-9a7b-0e1f2a3b4c5d}";

    #[test]
    fn device_path_round_trips() {
        assert_eq!(device_id(&device_path(ID)).as_deref(), Some(ID));
        assert_eq!(device_id(""), None);
    }

    /// Talks to the real Windows API: `cargo test -- --ignored probe`.
    #[test]
    #[ignore]
    fn probe() {
        crate::audio::with_com("probe", || {
            let router = Router::new().expect("interface missing");
            let pid = std::process::id();
            assert_eq!(router.get(pid), None);
            let device = unsafe { crate::audio::enumerator()?.GetDefaultAudioEndpoint(eRender, eMultimedia)? };
            let id = crate::audio::device_id(&device)?;
            router.set(pid, Some(&id)).unwrap();
            assert_eq!(router.get(pid).as_deref(), Some(id.as_str()));
            router.set(pid, None).unwrap();
            assert_eq!(router.get(pid), None);
            println!("probe ok");
            Ok(())
        });
    }
}
