//! The audio-devices thread and its Windows audio objects (Core Audio, through COM).
//!
//! Windows calls the device callback on its own threads. It only posts a message here; this
//! thread does the actual work, so the callback returns immediately as Windows requires.

use std::sync::mpsc::{Receiver, Sender};

use tauri::{AppHandle, Manager};
use tauri_specta::Event;
use windows::Win32::Foundation::PROPERTYKEY;
use windows::Win32::Media::Audio::{
    DEVICE_STATE, DEVICE_STATE_ACTIVE, EDataFlow, ERole, IMMDeviceEnumerator, IMMNotificationClient,
    IMMNotificationClient_Impl, eMultimedia, eRender,
};
use windows::core::{PCWSTR, Result, implement};

use super::{AudioDevicesChanged, AudioDevicesState, Msg, OutputDevice, Shared, policy_config};
use crate::audio;

pub fn run(app: AppHandle, tx: Sender<Msg>, rx: Receiver<Msg>) {
    audio::with_com("audio-devices", || serve(&app, &tx, &rx));
}

fn serve(app: &AppHandle, tx: &Sender<Msg>, rx: &Receiver<Msg>) -> Result<()> {
    let enumerator = audio::enumerator()?;
    let device_events: IMMNotificationClient = DeviceEvents(tx.clone()).into();
    unsafe { enumerator.RegisterEndpointNotificationCallback(&device_events)? };
    publish(app, &enumerator);

    // Sleeps until a message arrives. Bursts (one default change sends one per role) are handled together.
    'serve: while let Ok(first) = rx.recv() {
        let mut refresh = false;
        for msg in std::iter::once(first).chain(rx.try_iter()) {
            match msg {
                Msg::Stop => break 'serve,
                Msg::Refresh => refresh = true,
                // Windows then reports the new default, which refreshes.
                Msg::SetOutput(id) => report(policy_config::set_default(&id)),
            }
        }
        if refresh {
            publish(app, &enumerator);
        }
    }

    unsafe { enumerator.UnregisterEndpointNotificationCallback(&device_events)? };
    Ok(())
}

fn report(result: Result<()>) {
    if let Err(e) = result {
        eprintln!("audio-devices: {e}");
    }
}

/// Re-reads the devices and tells the UI, if anything changed.
fn publish(app: &AppHandle, enumerator: &IMMDeviceEnumerator) {
    let state = match outputs(enumerator) {
        Ok(outputs) => AudioDevicesState { outputs },
        Err(e) => return eprintln!("audio-devices: couldn't list devices: {e}"),
    };
    let shared = app.state::<Shared>();
    let mut current = shared.state.lock().unwrap();
    if *current != state {
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

/// Devices plugged in, removed, enabled, disabled, or made the default.
#[implement(IMMNotificationClient)]
struct DeviceEvents(Sender<Msg>);

impl DeviceEvents_Impl {
    fn refresh(&self) -> Result<()> {
        let _ = self.0.send(Msg::Refresh);
        Ok(())
    }
}

impl IMMNotificationClient_Impl for DeviceEvents_Impl {
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
