//! The mixer's background thread and its Windows audio objects (Core Audio, through COM).
//!
//! Windows calls our callbacks on its own threads. They only post a message here; this thread
//! does the actual work, so the callbacks return immediately as Windows requires.

use std::collections::HashMap;
use std::ptr;
use std::sync::mpsc::{Receiver, Sender};

use tauri::{AppHandle, Manager};
use tauri_specta::Event;
use windows::Win32::Foundation::{PROPERTYKEY, S_OK};
use windows::Win32::Media::Audio::Endpoints::{
    IAudioEndpointVolume, IAudioEndpointVolumeCallback, IAudioEndpointVolumeCallback_Impl,
};
use windows::Win32::Media::Audio::{
    AUDIO_VOLUME_NOTIFICATION_DATA, AudioSessionDisconnectReason, AudioSessionState, AudioSessionStateExpired,
    DEVICE_STATE, EDataFlow, ERole, IAudioSessionControl, IAudioSessionControl2, IAudioSessionEvents,
    IAudioSessionEvents_Impl, IAudioSessionManager2, IAudioSessionNotification, IAudioSessionNotification_Impl,
    IMMDeviceEnumerator, IMMNotificationClient, IMMNotificationClient_Impl, ISimpleAudioVolume, MMDeviceEnumerator,
    eMultimedia, eRender,
};
use windows::Win32::System::Com::{
    CLSCTX_ALL, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx, CoTaskMemFree, CoUninitialize,
};
use windows::core::{BOOL, GUID, Interface, PCWSTR, PWSTR, Ref, Result, implement};

use super::app_info::{self, AppInfo};
use super::{AppAudio, Level, MixerChanged, MixerState, Msg, Shared, Target};

pub fn run(app: AppHandle, tx: Sender<Msg>, rx: Receiver<Msg>) {
    unsafe {
        if let Err(e) = CoInitializeEx(None, COINIT_MULTITHREADED).ok() {
            eprintln!("volume-mixer: COM init failed: {e}");
            return;
        }
    }
    if let Err(e) = serve(&app, &tx, &rx) {
        eprintln!("volume-mixer stopped on an error: {e}");
    }
    unsafe { CoUninitialize() };
}

fn serve(app: &AppHandle, tx: &Sender<Msg>, rx: &Receiver<Msg>) -> Result<()> {
    let enumerator: IMMDeviceEnumerator = unsafe { CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)? };
    let device_events: IMMNotificationClient = DeviceEvents(tx.clone()).into();
    unsafe { enumerator.RegisterEndpointNotificationCallback(&device_events)? };

    let mut names = HashMap::new();
    let mut device = Device::open(&enumerator, tx);
    publish(app, &mut device, &mut names);

    // Sleeps until a message arrives. Bursts (a slider drag, many callbacks) are handled together.
    'serve: while let Ok(first) = rx.recv() {
        let (mut refresh, mut new_device) = (false, false);
        for msg in std::iter::once(first).chain(rx.try_iter()) {
            match (msg, device.as_mut()) {
                (Msg::Stop, _) => break 'serve,
                (Msg::Refresh, _) => refresh = true,
                (Msg::DeviceChanged, _) => new_device = true,
                (Msg::SetVolume(target, volume), Some(d)) => report(d.set_volume(target, volume)),
                (Msg::SetMute(target, muted), Some(d)) => report(d.set_mute(target, muted)),
                (_, None) => {} // no output device: nothing to change
            }
        }
        if new_device {
            drop(device.take()); // unregisters from the old device first
            device = Device::open(&enumerator, tx);
        }
        if refresh || new_device {
            publish(app, &mut device, &mut names);
        }
    }

    drop(device);
    unsafe { enumerator.UnregisterEndpointNotificationCallback(&device_events)? };
    Ok(())
}

fn report(result: Result<()>) {
    if let Err(e) = result {
        eprintln!("volume-mixer: {e}");
    }
}

/// Windows' 0.0–1.0 scale to whole percent.
fn percent(scalar: f32) -> u8 {
    (scalar.clamp(0.0, 1.0) * 100.0).round() as u8
}

/// Re-reads everything and tells the UI, if anything changed.
fn publish(app: &AppHandle, device: &mut Option<Device>, names: &mut HashMap<u32, AppInfo>) {
    let state = match device.as_mut().map(|d| d.snapshot(names)) {
        Some(Ok(state)) => state,
        Some(Err(e)) => {
            eprintln!("volume-mixer: couldn't read the mixer: {e}");
            return;
        }
        None => MixerState::default(),
    };
    let shared = app.state::<Shared>();
    let mut current = shared.state.lock().unwrap();
    if *current != state {
        *current = state.clone();
        let _ = MixerChanged(state).emit(app);
    }
}

/// The default output device, with our callbacks registered on it.
struct Device {
    endpoint: IAudioEndpointVolume,
    endpoint_events: IAudioEndpointVolumeCallback,
    manager: IAudioSessionManager2,
    session_created: IAudioSessionNotification,
    /// Every session we're listening to, by its instance id.
    sessions: HashMap<String, Session>,
    tx: Sender<Msg>,
}

struct Session {
    control: IAudioSessionControl,
    events: IAudioSessionEvents,
    pid: u32,
    volume: ISimpleAudioVolume,
}

impl Device {
    /// None when there's no output device (or it can't be opened); a device change retries.
    fn open(enumerator: &IMMDeviceEnumerator, tx: &Sender<Msg>) -> Option<Self> {
        let open = || -> Result<Self> {
            unsafe {
                let device = enumerator.GetDefaultAudioEndpoint(eRender, eMultimedia)?;
                let endpoint: IAudioEndpointVolume = device.Activate(CLSCTX_ALL, None)?;
                let manager: IAudioSessionManager2 = device.Activate(CLSCTX_ALL, None)?;
                let endpoint_events: IAudioEndpointVolumeCallback = Notify(tx.clone()).into();
                endpoint.RegisterControlChangeNotify(&endpoint_events)?;
                let session_created: IAudioSessionNotification = Notify(tx.clone()).into();
                manager.RegisterSessionNotification(&session_created)?;
                Ok(Self { endpoint, endpoint_events, manager, session_created, sessions: HashMap::new(), tx: tx.clone() })
            }
        };
        open().map_err(|e| eprintln!("volume-mixer: no output device ({e})")).ok()
    }

    fn snapshot(&mut self, names: &mut HashMap<u32, AppInfo>) -> Result<MixerState> {
        let master = unsafe {
            Level {
                volume: percent(self.endpoint.GetMasterVolumeLevelScalar()?),
                muted: self.endpoint.GetMute()?.as_bool(),
            }
        };
        let mut apps: Vec<AppAudio> = Vec::new();
        let mut seen = Vec::new();
        // Also required before Windows sends new-session notifications at all.
        let list = unsafe { self.manager.GetSessionEnumerator()? };
        for i in 0..unsafe { list.GetCount()? } {
            let control = unsafe { list.GetSession(i)? };
            let control2: IAudioSessionControl2 = control.cast()?;
            let id = take_string(unsafe { control2.GetSessionInstanceIdentifier()? });
            seen.push(id.clone());
            if !self.sessions.contains_key(&id) {
                let events: IAudioSessionEvents = Notify(self.tx.clone()).into();
                unsafe { control.RegisterAudioSessionNotification(&events)? };
                let pid = unsafe { control2.GetProcessId()? };
                let volume = control.cast()?;
                self.sessions.insert(id.clone(), Session { control: control.clone(), events, pid, volume });
            }
            if unsafe { control.GetState()? } == AudioSessionStateExpired {
                continue;
            }
            let session = &self.sessions[&id];
            // An app can have several sessions; the first one stands for the app.
            if apps.iter().any(|a| a.pid == session.pid) {
                continue;
            }
            let system_sounds = unsafe { control2.IsSystemSoundsSession() } == S_OK;
            let info = names.entry(session.pid).or_insert_with(|| {
                if system_sounds {
                    AppInfo { name: "System sounds".into(), icon: None }
                } else {
                    app_info::lookup(session.pid)
                }
            });
            apps.push(AppAudio {
                pid: session.pid,
                system_sounds,
                name: info.name.clone(),
                icon: info.icon.clone(),
                volume: percent(unsafe { session.volume.GetMasterVolume()? }),
                muted: unsafe { session.volume.GetMute()?.as_bool() },
            });
        }
        // Stop listening to sessions that are gone.
        self.sessions.retain(|id, session| {
            let keep = seen.contains(id);
            if !keep {
                let _ = unsafe { session.control.UnregisterAudioSessionNotification(&session.events) };
            }
            keep
        });
        Ok(MixerState { master: Some(master), apps })
    }

    fn set_volume(&mut self, target: Target, percent: u8) -> Result<()> {
        let volume = f32::from(percent) / 100.0;
        match target {
            Target::Master => unsafe { self.endpoint.SetMasterVolumeLevelScalar(volume, ptr::null()) },
            Target::App(pid) => self.app_sessions(pid).try_for_each(|v| unsafe { v.SetMasterVolume(volume, ptr::null()) }),
        }
    }

    fn set_mute(&mut self, target: Target, muted: bool) -> Result<()> {
        match target {
            Target::Master => unsafe { self.endpoint.SetMute(muted, ptr::null()) },
            Target::App(pid) => self.app_sessions(pid).try_for_each(|v| unsafe { v.SetMute(muted, ptr::null()) }),
        }
    }

    fn app_sessions(&self, pid: u32) -> impl Iterator<Item = &ISimpleAudioVolume> {
        self.sessions.values().filter(move |s| s.pid == pid).map(|s| &s.volume)
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        unsafe {
            for session in self.sessions.values() {
                let _ = session.control.UnregisterAudioSessionNotification(&session.events);
            }
            let _ = self.manager.UnregisterSessionNotification(&self.session_created);
            let _ = self.endpoint.UnregisterControlChangeNotify(&self.endpoint_events);
        }
    }
}

/// Copies a string Windows allocated for us, then frees it.
fn take_string(s: PWSTR) -> String {
    let text = unsafe { s.to_string() }.unwrap_or_default();
    unsafe { CoTaskMemFree(Some(s.0 as *const _)) };
    text
}

// ---- Callbacks: each one just posts `Refresh` to the worker thread. ---------------------------

/// Master volume, a session's volume or state, or a new session: all mean "re-read".
#[implement(IAudioEndpointVolumeCallback, IAudioSessionNotification, IAudioSessionEvents)]
struct Notify(Sender<Msg>);

impl Notify_Impl {
    fn refresh(&self) -> Result<()> {
        let _ = self.0.send(Msg::Refresh);
        Ok(())
    }
}

impl IAudioEndpointVolumeCallback_Impl for Notify_Impl {
    fn OnNotify(&self, _: *mut AUDIO_VOLUME_NOTIFICATION_DATA) -> Result<()> {
        self.refresh()
    }
}

impl IAudioSessionNotification_Impl for Notify_Impl {
    fn OnSessionCreated(&self, _: Ref<IAudioSessionControl>) -> Result<()> {
        self.refresh()
    }
}

impl IAudioSessionEvents_Impl for Notify_Impl {
    fn OnDisplayNameChanged(&self, _: &PCWSTR, _: *const GUID) -> Result<()> {
        Ok(())
    }
    fn OnIconPathChanged(&self, _: &PCWSTR, _: *const GUID) -> Result<()> {
        Ok(())
    }
    fn OnSimpleVolumeChanged(&self, _: f32, _: BOOL, _: *const GUID) -> Result<()> {
        self.refresh()
    }
    fn OnChannelVolumeChanged(&self, _: u32, _: *const f32, _: u32, _: *const GUID) -> Result<()> {
        Ok(())
    }
    fn OnGroupingParamChanged(&self, _: *const GUID, _: *const GUID) -> Result<()> {
        Ok(())
    }
    fn OnStateChanged(&self, _: AudioSessionState) -> Result<()> {
        self.refresh()
    }
    fn OnSessionDisconnected(&self, _: AudioSessionDisconnectReason) -> Result<()> {
        self.refresh()
    }
}

/// Default output device changes (headphones plugged in, a different device picked in Windows).
#[implement(IMMNotificationClient)]
struct DeviceEvents(Sender<Msg>);

impl IMMNotificationClient_Impl for DeviceEvents_Impl {
    fn OnDefaultDeviceChanged(&self, flow: EDataFlow, role: ERole, _: &PCWSTR) -> Result<()> {
        if flow == eRender && role == eMultimedia {
            let _ = self.0.send(Msg::DeviceChanged);
        }
        Ok(())
    }
    fn OnDeviceStateChanged(&self, _: &PCWSTR, _: DEVICE_STATE) -> Result<()> {
        Ok(())
    }
    fn OnDeviceAdded(&self, _: &PCWSTR) -> Result<()> {
        Ok(())
    }
    fn OnDeviceRemoved(&self, _: &PCWSTR) -> Result<()> {
        Ok(())
    }
    fn OnPropertyValueChanged(&self, _: &PCWSTR, _: &PROPERTYKEY) -> Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn percent_rounds_and_clamps() {
        assert_eq!(super::percent(0.0), 0);
        assert_eq!(super::percent(0.064), 6); // Windows reports 6% as about 0.06
        assert_eq!(super::percent(0.995), 100);
        assert_eq!(super::percent(1.0), 100);
        assert_eq!(super::percent(-0.1), 0);
        assert_eq!(super::percent(1.5), 100);
        assert_eq!(super::percent(f32::NAN), 0);
    }
}
