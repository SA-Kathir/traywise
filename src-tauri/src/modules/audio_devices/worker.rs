//! The audio-devices thread and its Windows audio objects (Core Audio, through COM).
//!
//! Windows calls our callbacks on its own threads. They only post a message here; this thread
//! does the actual work, so the callbacks return immediately as Windows requires.

use std::collections::HashMap;
use std::ptr;
use std::sync::mpsc::{Receiver, Sender};

use tauri::{AppHandle, Manager};
use tauri_specta::Event;
use windows::Win32::Foundation::PROPERTYKEY;
use windows::Win32::Media::Audio::Endpoints::{
    IAudioEndpointVolume, IAudioEndpointVolumeCallback, IAudioEndpointVolumeCallback_Impl,
};
use windows::Win32::Media::Audio::{
    AUDIO_VOLUME_NOTIFICATION_DATA, DEVICE_STATE, DEVICE_STATE_ACTIVE, EDataFlow, ERole, IMMDeviceEnumerator,
    IMMNotificationClient, IMMNotificationClient_Impl, eCapture, eMultimedia, eRender,
};
use windows::Win32::System::Com::CLSCTX_ALL;
use windows::core::{PCWSTR, Result, implement};

use super::{AudioDevicesChanged, AudioDevicesState, Msg, OutputDevice, Shared, ShortcutStatus, policy_config};
use crate::audio;
use crate::shell::tray;

pub fn run(app: AppHandle, tx: Sender<Msg>, rx: Receiver<Msg>) {
    audio::with_com("audio-devices", || serve(&app, &tx, &rx));
    tray::set_mics_muted(&app, false);
}

fn serve(app: &AppHandle, tx: &Sender<Msg>, rx: &Receiver<Msg>) -> Result<()> {
    let enumerator = audio::enumerator()?;
    let device_events: IMMNotificationClient = Notify(tx.clone()).into();
    unsafe { enumerator.RegisterEndpointNotificationCallback(&device_events)? };
    let mut mics = Mics { list: HashMap::new(), follow: false, tx: tx.clone() };
    let mut shortcut: Option<ShortcutStatus> = None;
    publish(app, &enumerator, &mut mics, &shortcut);

    // Sleeps until a message arrives. Bursts (one default change sends one per role) are handled together.
    'serve: while let Ok(first) = rx.recv() {
        let mut refresh = false;
        for msg in std::iter::once(first).chain(rx.try_iter()) {
            match msg {
                Msg::Stop => break 'serve,
                Msg::Refresh => refresh = true,
                // Windows then reports the change, which refreshes.
                Msg::SetOutput(id) => report(policy_config::set_default(&id)),
                Msg::MuteMics(muted) => report(mics.set_muted(muted)),
                Msg::ToggleMics => {
                    let muted = mics.all_muted().unwrap_or(false);
                    report(mics.set_muted(!muted));
                }
                Msg::Shortcut(status) => {
                    shortcut = Some(status);
                    refresh = true;
                }
            }
        }
        if refresh {
            publish(app, &enumerator, &mut mics, &shortcut);
        }
    }

    drop(mics); // unregisters from every microphone
    unsafe { enumerator.UnregisterEndpointNotificationCallback(&device_events)? };
    Ok(())
}

fn report(result: Result<()>) {
    if let Err(e) = result {
        eprintln!("audio-devices: {e}");
    }
}

/// Re-reads the devices and tells the UI, if anything changed.
fn publish(app: &AppHandle, enumerator: &IMMDeviceEnumerator, mics: &mut Mics, shortcut: &Option<ShortcutStatus>) {
    let outputs = match outputs(enumerator) {
        Ok(outputs) => outputs,
        Err(e) => return eprintln!("audio-devices: couldn't list outputs: {e}"),
    };
    if let Err(e) = mics.refresh(enumerator) {
        eprintln!("audio-devices: couldn't list microphones: {e}");
    }
    let state = AudioDevicesState { outputs, mics_muted: mics.all_muted(), shortcut: shortcut.clone() };
    let shared = app.state::<Shared>();
    let mut current = shared.state.lock().unwrap();
    if *current != state {
        tray::set_mics_muted(app, state.mics_muted == Some(true));
        *current = state.clone();
        let _ = AudioDevicesChanged(state).emit(app);
    }
}

/// Every plugged-in, enabled output device, by name.
fn outputs(enumerator: &IMMDeviceEnumerator) -> Result<Vec<OutputDevice>> {
    // No default when there's no output device at all.
    let default = unsafe { enumerator.GetDefaultAudioEndpoint(eRender, eMultimedia) }.and_then(|d| audio::device_id(&d)).ok();
    let list = unsafe { enumerator.EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE)? };
    let mut outputs = Vec::new();
    for i in 0..unsafe { list.GetCount()? } {
        let device = unsafe { list.Item(i)? };
        let id = audio::device_id(&device)?;
        let name = audio::friendly_name(&device).unwrap_or_else(|_| "Unknown device".into());
        outputs.push(OutputDevice { is_default: default.as_ref() == Some(&id), id, name });
    }
    outputs.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(outputs)
}

/// Every plugged-in, enabled microphone, with a callback on its mute switch.
struct Mics {
    list: HashMap<String, Mic>,
    /// Whether all were muted when last checked. Newly plugged-in microphones follow it.
    follow: bool,
    tx: Sender<Msg>,
}

struct Mic {
    endpoint: IAudioEndpointVolume,
    events: IAudioEndpointVolumeCallback,
}

impl Mics {
    fn refresh(&mut self, enumerator: &IMMDeviceEnumerator) -> Result<()> {
        let list = unsafe { enumerator.EnumAudioEndpoints(eCapture, DEVICE_STATE_ACTIVE)? };
        let mut seen = Vec::new();
        for i in 0..unsafe { list.GetCount()? } {
            let device = unsafe { list.Item(i)? };
            let id = audio::device_id(&device)?;
            if !self.list.contains_key(&id) {
                let endpoint: IAudioEndpointVolume = unsafe { device.Activate(CLSCTX_ALL, None)? };
                let events: IAudioEndpointVolumeCallback = Notify(self.tx.clone()).into();
                unsafe { endpoint.RegisterControlChangeNotify(&events)? };
                if self.follow {
                    unsafe { endpoint.SetMute(true, ptr::null())? };
                }
                self.list.insert(id.clone(), Mic { endpoint, events });
            }
            seen.push(id);
        }
        self.list.retain(|id, _| seen.contains(id)); // dropping one unregisters it
        Ok(())
    }

    /// None when there are no microphones.
    fn all_muted(&mut self) -> Option<bool> {
        if self.list.is_empty() {
            return None;
        }
        let muted = self.list.values().all(|m| unsafe { m.endpoint.GetMute() }.is_ok_and(|b| b.as_bool()));
        self.follow = muted;
        Some(muted)
    }

    /// Windows then reports each change, which refreshes.
    fn set_muted(&self, muted: bool) -> Result<()> {
        self.list.values().try_for_each(|m| unsafe { m.endpoint.SetMute(muted, ptr::null()) })
    }
}

impl Drop for Mic {
    fn drop(&mut self) {
        let _ = unsafe { self.endpoint.UnregisterControlChangeNotify(&self.events) };
    }
}

// ---- Callbacks: each one just posts `Refresh` to the worker thread. ---------------------------

/// Devices plugged in, removed, enabled, disabled or made the default, and a microphone (un)muted.
#[implement(IMMNotificationClient, IAudioEndpointVolumeCallback)]
struct Notify(Sender<Msg>);

impl Notify_Impl {
    fn refresh(&self) -> Result<()> {
        let _ = self.0.send(Msg::Refresh);
        Ok(())
    }
}

impl IMMNotificationClient_Impl for Notify_Impl {
    fn OnDefaultDeviceChanged(&self, _: EDataFlow, _: ERole, _: &PCWSTR) -> Result<()> {
        self.refresh()
    }
    fn OnDeviceStateChanged(&self, _: &PCWSTR, _: DEVICE_STATE) -> Result<()> {
        self.refresh()
    }
    fn OnDeviceAdded(&self, _: &PCWSTR) -> Result<()> {
        self.refresh()
    }
    fn OnDeviceRemoved(&self, _: &PCWSTR) -> Result<()> {
        self.refresh()
    }
    // Fires often (levels, formats); names rarely change, so it's ignored.
    fn OnPropertyValueChanged(&self, _: &PCWSTR, _: &PROPERTYKEY) -> Result<()> {
        Ok(())
    }
}

impl IAudioEndpointVolumeCallback_Impl for Notify_Impl {
    fn OnNotify(&self, _: *mut AUDIO_VOLUME_NOTIFICATION_DATA) -> Result<()> {
        self.refresh()
    }
}
