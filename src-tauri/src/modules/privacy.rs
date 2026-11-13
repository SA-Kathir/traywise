//! Privacy: which apps are using the microphone or camera right now, from Windows' consent store
//! (Settings › Privacy & security › Microphone / Camera, "Recent activity").
//!
//! Nothing runs in the background: the registry is read when the panel opens and when a tile asks.
//! Windows notes, for each app, when it last started and stopped using a device; an app whose
//! start is later than its stop (or that has no stop yet) is using it now.

use serde::Serialize;
use tauri::AppHandle;
use tauri_specta::Event;
use windows::Win32::Foundation::ERROR_SUCCESS;
use windows::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, KEY_READ, RRF_RT_REG_QWORD, RegCloseKey, RegEnumKeyExW, RegGetValueW, RegOpenKeyExW,
};
use windows::core::{HSTRING, PWSTR};

use crate::module::{Module, ModuleId};

const STORE: &str = r"Software\Microsoft\Windows\CurrentVersion\CapabilityAccessManager\ConsentStore";

#[derive(Clone, Debug, Default, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PrivacyState {
    /// Apps using the microphone now, by name.
    pub microphone: Vec<String>,
    /// Apps using the camera now.
    pub camera: Vec<String>,
}

/// Sent when the panel opens, and after a read the UI asked for.
#[derive(Clone, Serialize, specta::Type, tauri_specta::Event)]
pub struct PrivacyChanged(pub PrivacyState);

pub struct Privacy {
    app: AppHandle,
}

impl Privacy {
    pub fn new(app: AppHandle) -> Self {
        Self { app }
    }
}

impl Module for Privacy {
    fn id(&self) -> ModuleId {
        ModuleId::Privacy
    }

    fn start(&mut self) -> Result<(), String> {
        Ok(()) // nothing to start: no thread, no subscriptions
    }

    fn stop(&mut self) {}

    fn panel_visible(&mut self, visible: bool) {
        if visible {
            let _ = PrivacyChanged(read()).emit(&self.app);
        }
    }
}

#[tauri::command]
#[specta::specta]
pub fn privacy_state() -> PrivacyState {
    read()
}

fn read() -> PrivacyState {
    PrivacyState { microphone: using("microphone"), camera: using("webcam") }
}

/// The apps using one capability now. Windows keeps two lists under it: apps from the Store (by
/// package) and desktop apps (under `NonPackaged`, by their file's path with `#` for `\`).
fn using(capability: &str) -> Vec<String> {
    let base = format!(r"{STORE}\{capability}");
    let mut apps: Vec<String> = Vec::new();
    for (path, packaged) in [(base.clone(), true), (format!(r"{base}\NonPackaged"), false)] {
        for name in subkeys(&path) {
            if name == "NonPackaged" {
                continue;
            }
            let key = format!(r"{path}\{name}");
            if let (Some(start), stop) = (qword(&key, "LastUsedTimeStart"), qword(&key, "LastUsedTimeStop"))
                && in_use(start, stop.unwrap_or(0))
            {
                let shown = app_name(&name, packaged);
                if !apps.contains(&shown) {
                    apps.push(shown);
                }
            }
        }
    }
    apps.sort();
    apps
}

/// Using it now: it started, and either never stopped or started after it last stopped.
fn in_use(start: u64, stop: u64) -> bool {
    start > 0 && start > stop
}

/// What to call an app from its key: a desktop app's `C:#Program Files#Zoom#bin#Zoom.exe` is
/// `Zoom`; a Store app's `Microsoft.WindowsCamera_8wekyb3d8bbwe` is `Microsoft.WindowsCamera`.
fn app_name(key: &str, packaged: bool) -> String {
    if packaged {
        return key.split('_').next().unwrap_or(key).to_string();
    }
    let file = key.rsplit('#').next().unwrap_or(key);
    file.strip_suffix(".exe").or_else(|| file.strip_suffix(".EXE")).unwrap_or(file).to_string()
}

fn subkeys(path: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut key = HKEY::default();
    if unsafe { RegOpenKeyExW(HKEY_CURRENT_USER, &HSTRING::from(path), None, KEY_READ, &mut key) } != ERROR_SUCCESS {
        return out;
    }
    for i in 0.. {
        let mut name = [0u16; 512];
        let mut len = name.len() as u32;
        if unsafe { RegEnumKeyExW(key, i, Some(PWSTR(name.as_mut_ptr())), &mut len, None, None, None, None) } != ERROR_SUCCESS {
            break;
        }
        out.push(String::from_utf16_lossy(&name[..len as usize]));
    }
    let _ = unsafe { RegCloseKey(key) };
    out
}

fn qword(path: &str, name: &str) -> Option<u64> {
    let (mut value, mut size) = (0u64, 8u32);
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            &HSTRING::from(path),
            &HSTRING::from(name),
            RRF_RT_REG_QWORD,
            None,
            Some(&mut value as *mut u64 as *mut _),
            Some(&mut size),
        )
    };
    (status == ERROR_SUCCESS).then_some(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn using_now_means_started_after_it_stopped() {
        assert!(in_use(100, 0), "started, never stopped");
        assert!(in_use(200, 100), "started again after stopping");
        assert!(!in_use(100, 200), "stopped");
        assert!(!in_use(100, 100));
        assert!(!in_use(0, 0), "never used");
    }

    #[test]
    fn app_names() {
        assert_eq!(app_name("C:#Program Files#Zoom#bin#Zoom.exe", false), "Zoom");
        assert_eq!(app_name("Microsoft.WindowsCamera_8wekyb3d8bbwe", true), "Microsoft.WindowsCamera");
        assert_eq!(app_name("Teams", false), "Teams");
    }

    /// `cargo test privacy_live -- --ignored --nocapture`: reads this PC's consent store.
    #[test]
    #[ignore]
    fn privacy_live() {
        println!("{:?}", read());
    }
}
