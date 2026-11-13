//! Privacy: which apps are using the microphone or camera right now, from Windows' consent store
//! (Settings › Privacy & security › Microphone / Camera, "Recent activity").
//!
//! Windows notes, for each app, when it last started and stopped using a device; an app whose
//! start is later than its stop (or that has no stop yet) is using it now. The registry is read when
//! the panel opens, and again whenever Windows changes anything in the consent store (an app starts
//! or stops using a device): a thread sleeps on a registry change notification, so it costs nothing
//! while nothing changes.

use std::thread::JoinHandle;

use serde::Serialize;
use tauri::AppHandle;
use tauri_specta::Event;
use windows::Win32::Foundation::{CloseHandle, ERROR_SUCCESS, HANDLE};
use windows::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, KEY_NOTIFY, KEY_READ, REG_NOTIFY_CHANGE_LAST_SET, RRF_RT_REG_QWORD, RegCloseKey, RegEnumKeyExW,
    RegGetValueW, RegNotifyChangeKeyValue, RegOpenKeyExW,
};
use windows::Win32::System::Threading::{CreateEventW, INFINITE, SetEvent, Sleep, WaitForMultipleObjects};
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
    /// The thread waiting for changes, and the event that ends it (a handle, as a number to cross threads).
    watcher: Option<(JoinHandle<()>, isize)>,
}

impl Privacy {
    pub fn new(app: AppHandle) -> Self {
        Self { app, watcher: None }
    }
}

impl Module for Privacy {
    fn id(&self) -> ModuleId {
        ModuleId::Privacy
    }

    fn start(&mut self) -> Result<(), String> {
        let stop = unsafe { CreateEventW(None, true, false, None) }.map_err(|e| e.to_string())?;
        let app = self.app.clone();
        let stop_id = stop.0 as isize;
        let thread = std::thread::Builder::new()
            .name("privacy".into())
            .spawn(move || watch(&app, stop_id))
            .map_err(|e| e.to_string())?;
        self.watcher = Some((thread, stop_id));
        Ok(())
    }

    fn stop(&mut self) {
        if let Some((thread, stop)) = self.watcher.take() {
            unsafe {
                let _ = SetEvent(HANDLE(stop as _));
            }
            let _ = thread.join();
            unsafe {
                let _ = CloseHandle(HANDLE(stop as _));
            }
        }
    }

    fn panel_visible(&mut self, visible: bool) {
        if visible {
            let _ = PrivacyChanged(read()).emit(&self.app);
        }
    }
}

/// Sleeps until Windows changes something under the consent store, then sends the state again.
fn watch(app: &AppHandle, stop: isize) {
    let mut key = HKEY::default();
    if unsafe { RegOpenKeyExW(HKEY_CURRENT_USER, &HSTRING::from(STORE), None, KEY_NOTIFY, &mut key) } != ERROR_SUCCESS {
        return; // no live updates: the panel still reads it when it opens
    }
    let Ok(changed) = (unsafe { CreateEventW(None, false, false, None) }) else {
        let _ = unsafe { RegCloseKey(key) };
        return;
    };
    loop {
        // (Asks again each time: a notification is used up once it has fired.)
        if unsafe { RegNotifyChangeKeyValue(key, true, REG_NOTIFY_CHANGE_LAST_SET, Some(changed), true) } != ERROR_SUCCESS {
            break;
        }
        // Index 0 is a change, 1 is the stop request.
        if unsafe { WaitForMultipleObjects(&[changed, HANDLE(stop as _)], false, INFINITE) }.0 != 0 {
            break;
        }
        // Windows writes the start and stop times a moment apart: read once they have settled.
        unsafe { Sleep(150) };
        let _ = PrivacyChanged(read()).emit(app);
    }
    unsafe {
        let _ = CloseHandle(changed);
        let _ = RegCloseKey(key);
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

    /// `cargo test privacy_notify -- --ignored --nocapture`, then change something under the consent
    /// store within 10 s: the notification must fire.
    #[test]
    #[ignore]
    fn privacy_notify() {
        use windows::Win32::System::Threading::WaitForSingleObject;
        let mut key = HKEY::default();
        assert_eq!(unsafe { RegOpenKeyExW(HKEY_CURRENT_USER, &HSTRING::from(STORE), None, KEY_NOTIFY, &mut key) }, ERROR_SUCCESS);
        let changed = unsafe { CreateEventW(None, false, false, None) }.unwrap();
        assert_eq!(unsafe { RegNotifyChangeKeyValue(key, true, REG_NOTIFY_CHANGE_LAST_SET, Some(changed), true) }, ERROR_SUCCESS);
        println!("WAITING");
        let t = std::time::Instant::now();
        let r = unsafe { WaitForSingleObject(changed, 10_000) };
        println!("woke: {r:?} after {:?}", t.elapsed());
    }

    /// `cargo test privacy_live -- --ignored --nocapture`: reads this PC's consent store.
    #[test]
    #[ignore]
    fn privacy_live() {
        println!("{:?}", read());
    }
}
