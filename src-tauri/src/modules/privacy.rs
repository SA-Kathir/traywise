//! Privacy: which apps are using the microphone or camera right now.
//!
//! The microphone comes from the audio engine (an app with an active capture session is recording),
//! looked at once a second while the panel is open. The camera comes from Windows' own camera
//! activity monitor (the one behind the camera indicator in the taskbar): Windows calls back when an
//! app starts or stops streaming from a camera, so it costs nothing while nothing happens. If that
//! isn't available, both fall back to the consent store (Settings › Privacy & security, "Recent
//! activity"), which Windows writes late on some PCs. The thread sleeps while the panel is closed.

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel};
use std::thread::JoinHandle;
use std::time::Duration;

use serde::Serialize;
use tauri::AppHandle;
use tauri_specta::Event;
use windows::Win32::Foundation::ERROR_SUCCESS;
use windows::Win32::Media::MediaFoundation::{
    IMFSensorActivitiesReport, IMFSensorActivitiesReportCallback, IMFSensorActivitiesReportCallback_Impl, MF_VERSION, MFCreateSensorActivityMonitor,
    MFSTARTUP_LITE, MFShutdown, MFStartup,
};
use windows::Win32::System::Registry::{HKEY, HKEY_CURRENT_USER, KEY_READ, RRF_RT_REG_QWORD, RegCloseKey, RegEnumKeyExW, RegGetValueW, RegOpenKeyExW};
use windows::core::{HSTRING, PWSTR, Ref, implement};

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

enum Msg {
    PanelVisible(bool),
    /// The camera activity monitor reported a change.
    Camera,
    Stop,
}

/// Apps streaming from a camera now, as the activity monitor last said; and whether it is running at all.
static CAMERA: Mutex<Vec<String>> = Mutex::new(Vec::new());
static CAMERA_MONITOR: AtomicBool = AtomicBool::new(false);

/// Windows' camera activity callback: it runs on one of Windows' threads, notes who is streaming and
/// tells the worker.
#[implement(IMFSensorActivitiesReportCallback)]
struct CameraWatch(Sender<Msg>);

impl IMFSensorActivitiesReportCallback_Impl for CameraWatch_Impl {
    fn OnActivitiesReport(&self, report: Ref<IMFSensorActivitiesReport>) -> windows::core::Result<()> {
        if let Some(report) = report.as_ref() {
            *CAMERA.lock().unwrap() = camera_apps(report).unwrap_or_default();
        }
        let _ = self.0.send(Msg::Camera);
        Ok(())
    }
}

/// The apps that are streaming from any camera in the report.
fn camera_apps(report: &IMFSensorActivitiesReport) -> windows::core::Result<Vec<String>> {
    let mut names = Vec::new();
    for i in 0..unsafe { report.GetCount()? } {
        let camera = unsafe { report.GetActivityReport(i)? };
        for j in 0..unsafe { camera.GetProcessCount()? } {
            let process = unsafe { camera.GetProcessActivity(j)? };
            if unsafe { process.GetStreamingState()? }.as_bool() {
                let name = crate::modules::volume_mixer::app_name(unsafe { process.GetProcessId()? });
                if !names.contains(&name) {
                    names.push(name);
                }
            }
        }
    }
    Ok(names)
}

pub struct Privacy {
    app: AppHandle,
    worker: Option<(Sender<Msg>, JoinHandle<()>)>,
}

impl Privacy {
    pub fn new(app: AppHandle) -> Self {
        Self { app, worker: None }
    }
}

impl Module for Privacy {
    fn id(&self) -> ModuleId {
        ModuleId::Privacy
    }

    fn start(&mut self) -> Result<(), String> {
        let (tx, rx) = channel();
        let app = self.app.clone();
        let to_worker = tx.clone();
        let thread = std::thread::Builder::new().name("privacy".into()).spawn(move || run(&app, rx, to_worker)).map_err(|e| e.to_string())?;
        self.worker = Some((tx, thread));
        Ok(())
    }

    fn stop(&mut self) {
        if let Some((tx, thread)) = self.worker.take() {
            let _ = tx.send(Msg::Stop);
            let _ = thread.join();
        }
    }

    fn panel_visible(&mut self, visible: bool) {
        if let Some((tx, _)) = &self.worker {
            let _ = tx.send(Msg::PanelVisible(visible));
        }
    }
}

/// The last state sent: what `privacy_state` answers with.
static LATEST: Mutex<PrivacyState> = Mutex::new(PrivacyState { microphone: Vec::new(), camera: Vec::new() });

/// Sleeps while the panel is closed; while it is open, sends the state at once and then whenever it
/// changes (checked every second).
fn run(app: &AppHandle, rx: Receiver<Msg>, tx: Sender<Msg>) {
    crate::audio::with_com("privacy", || {
        // Windows' camera indicator: told when a camera starts or stops streaming. It reports the
        // current state right after it starts.
        let started = unsafe { MFStartup(MF_VERSION, MFSTARTUP_LITE) }.is_ok();
        let monitor = started
            .then(|| unsafe { MFCreateSensorActivityMonitor(&IMFSensorActivitiesReportCallback::from(CameraWatch(tx))) })
            .and_then(|m| m.ok())
            .filter(|m| unsafe { m.Start() }.is_ok());
        CAMERA_MONITOR.store(monitor.is_some(), Ordering::Relaxed);
        let mut visible = false;
        loop {
            let msg = if visible { rx.recv_timeout(Duration::from_secs(1)) } else { rx.recv().map_err(|_| RecvTimeoutError::Disconnected) };
            let opened = match msg {
                Ok(Msg::PanelVisible(v)) => {
                    visible = v;
                    if !v {
                        continue;
                    }
                    true
                }
                Ok(Msg::Stop) | Err(RecvTimeoutError::Disconnected) => break,
                Ok(Msg::Camera) if !visible => continue,
                Ok(Msg::Camera) | Err(RecvTimeoutError::Timeout) => false,
            };
            let state = read();
            let changed = *LATEST.lock().unwrap() != state;
            if changed || opened {
                *LATEST.lock().unwrap() = state.clone();
                let _ = PrivacyChanged(state).emit(app);
            }
        }
        if let Some(monitor) = monitor {
            let _ = unsafe { monitor.Stop() };
        }
        if started {
            let _ = unsafe { MFShutdown() };
        }
        Ok(())
    });
}

#[tauri::command]
#[specta::specta]
pub fn privacy_state() -> PrivacyState {
    LATEST.lock().unwrap().clone()
}

fn read() -> PrivacyState {
    // A device the audio engine can't be asked about: fall back to the consent store.
    let microphone = capture_apps().unwrap_or_else(|_| using("microphone"));
    let camera = if CAMERA_MONITOR.load(Ordering::Relaxed) { CAMERA.lock().unwrap().clone() } else { using("webcam") };
    PrivacyState { microphone, camera }
}

/// Apps with an active session on any microphone, straight from the audio engine (what the volume
/// mixer reads for output): live, unlike the consent store, which Windows writes late.
fn capture_apps() -> windows::core::Result<Vec<String>> {
    use windows::Win32::Media::Audio::{AudioSessionStateActive, DEVICE_STATE_ACTIVE, IAudioSessionControl2, IAudioSessionManager2, eCapture};
    use windows::Win32::System::Com::CLSCTX_ALL;
    use windows::core::Interface;
    let mut names = Vec::new();
    let devices = unsafe { crate::audio::enumerator()?.EnumAudioEndpoints(eCapture, DEVICE_STATE_ACTIVE)? };
    for i in 0..unsafe { devices.GetCount()? } {
        let manager: IAudioSessionManager2 = unsafe { devices.Item(i)?.Activate(CLSCTX_ALL, None)? };
        let sessions = unsafe { manager.GetSessionEnumerator()? };
        for j in 0..unsafe { sessions.GetCount()? } {
            let control = unsafe { sessions.GetSession(j)? };
            let control2: IAudioSessionControl2 = control.cast()?;
            let pid = unsafe { control2.GetProcessId()? };
            if pid != 0 && unsafe { control.GetState()? } == AudioSessionStateActive {
                let name = crate::modules::volume_mixer::app_name(pid);
                if !names.contains(&name) {
                    names.push(name);
                }
            }
        }
    }
    Ok(names)
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

    /// `cargo test camera_live -- --ignored --nocapture`: who is streaming from a camera now, as Windows' monitor says.
    #[test]
    #[ignore]
    fn camera_live() {
        crate::audio::with_com("privacy test", || {
            let (tx, rx) = channel();
            unsafe { MFStartup(MF_VERSION, MFSTARTUP_LITE)? };
            let monitor = unsafe { MFCreateSensorActivityMonitor(&IMFSensorActivitiesReportCallback::from(CameraWatch(tx)))? };
            unsafe { monitor.Start()? };
            let mut n = 0;
            while rx.recv_timeout(Duration::from_secs(3)).is_ok() {
                n += 1;
                println!("callback #{n}; camera apps: {:?}", CAMERA.lock().unwrap());
            }
            println!("callbacks: {n}");
            unsafe { monitor.Stop()? };
            Ok(())
        });
    }

    /// `cargo test capture_live -- --ignored --nocapture`: who has an active capture session right now.
    #[test]
    #[ignore]
    fn capture_live() {
        crate::audio::with_com("privacy test", || {
            println!("capture sessions: {:?}", capture_apps()?);
            Ok(())
        });
    }

    /// `cargo test privacy_live -- --ignored --nocapture`: reads this PC's consent store.
    #[test]
    #[ignore]
    fn privacy_live() {
        println!("{:?}", read());
    }
}
