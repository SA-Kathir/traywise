//! Bluetooth: switch the radio, see paired devices and which are connected, and connect or
//! disconnect headphones and speakers.
//!
//! Nothing runs in the background: the state is read when the panel opens and after each change.
//! Windows has no public way for an app to connect a device. For audio devices, Traywise sends the
//! Bluetooth audio driver the same "reconnect" / "disconnect" request as the open-source ToothTray
//! (KSPROPSETID_BtAudio); other devices (mice, keyboards) only show their state.

use serde::Serialize;
use tauri::AppHandle;
use tauri_specta::Event;
use windows::Devices::Bluetooth::{BluetoothDevice, BluetoothLEDevice};
use windows::Devices::Enumeration::DeviceInformation;
use windows::Devices::Radios::RadioKind;
use windows_collections::IIterable;
use windows::Foundation::IReference;
use windows::Win32::Devices::FunctionDiscovery::PKEY_Device_ContainerId;
use windows::Win32::Media::Audio::{DEVICE_STATE, DEVICE_STATEMASK_ALL, IDeviceTopology, IMMDevice, IPart, eAll};
use windows::Win32::Media::KernelStreaming::{
    IKsControl, KSIDENTIFIER, KSIDENTIFIER_0, KSIDENTIFIER_0_0, KSPROPERTY_TYPE_GET, KSPROPSETID_BtAudio,
};
use windows::Win32::System::Com::StructuredStorage::{PropVariantClear, PropVariantToGUID};
use windows::Win32::System::Com::{CLSCTX_ALL, STGM_READ};
use windows::core::{GUID, HSTRING, Interface, Result};

use crate::audio;
use crate::module::{Module, ModuleId};
use crate::radios::{mta, radio_on, set_radio};

#[derive(Clone, Debug, Default, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct BluetoothState {
    /// The radio; None when this PC has no Bluetooth.
    pub on: Option<bool>,
    /// Paired devices, connected ones first.
    pub devices: Vec<BluetoothDeviceInfo>,
}

#[derive(Clone, Debug, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct BluetoothDeviceInfo {
    /// Windows' id for the device as a whole (its container), for the commands.
    pub id: String,
    pub name: String,
    pub connected: bool,
    /// Headphones or speakers: Traywise can connect and disconnect it.
    pub audio: bool,
}

/// Sent when the panel opens and after each change.
#[derive(Clone, Serialize, specta::Type, tauri_specta::Event)]
pub struct BluetoothChanged(pub BluetoothState);

pub struct Bluetooth {
    app: AppHandle,
}

impl Bluetooth {
    pub fn new(app: AppHandle) -> Self {
        Self { app }
    }
}

impl Module for Bluetooth {
    fn id(&self) -> ModuleId {
        ModuleId::Bluetooth
    }

    fn start(&mut self) -> std::result::Result<(), String> {
        Ok(()) // nothing to start: no thread, no subscriptions
    }

    fn stop(&mut self) {}

    fn panel_visible(&mut self, visible: bool) {
        if visible {
            let app = self.app.clone();
            std::thread::spawn(move || publish(&app));
        }
    }
}

fn publish(app: &AppHandle) {
    let _ = BluetoothChanged(read()).emit(app);
}

/// Waits on Windows: call off the UI's thread.
fn read() -> BluetoothState {
    let on = radio_on(RadioKind::Bluetooth);
    let devices = if on == Some(true) {
        devices().unwrap_or_else(|e| {
            eprintln!("bluetooth: couldn't list devices: {e}");
            Vec::new()
        })
    } else {
        Vec::new()
    };
    BluetoothState { on, devices }
}

const CONNECTED: &str = "System.Devices.Aep.IsConnected";
const CONTAINER: &str = "System.Devices.Aep.ContainerId";

/// Paired devices, classic and Bluetooth LE, one entry per physical device.
fn devices() -> Result<Vec<BluetoothDeviceInfo>> {
    mta();
    let audio = audio_containers()?;
    let mut devices: Vec<BluetoothDeviceInfo> = Vec::new();
    let properties = IIterable::<HSTRING>::from(vec![HSTRING::from(CONNECTED), HSTRING::from(CONTAINER)]);
    for selector in [BluetoothDevice::GetDeviceSelectorFromPairingState(true)?, BluetoothLEDevice::GetDeviceSelectorFromPairingState(true)?] {
        for info in DeviceInformation::FindAllAsyncAqsFilterAndAdditionalProperties(&selector, &properties)?.join()? {
            let props = info.Properties()?;
            let connected = props.Lookup(&HSTRING::from(CONNECTED)).and_then(|v| v.cast::<IReference<bool>>()?.Value()).unwrap_or(false);
            let Ok(container) = props.Lookup(&HSTRING::from(CONTAINER)).and_then(|v| v.cast::<IReference<GUID>>()?.Value()) else { continue };
            let id = format!("{container:?}");
            match devices.iter_mut().find(|d| d.id == id) {
                Some(d) => d.connected |= connected,
                None => devices.push(BluetoothDeviceInfo {
                    name: info.Name()?.to_string(),
                    connected,
                    audio: audio.iter().any(|(c, _)| *c == container),
                    id,
                }),
            }
        }
    }
    devices.sort_by_key(|d| !d.connected);
    Ok(devices)
}

/// Every audio endpoint (connected or not) with the device it belongs to.
fn audio_containers() -> Result<Vec<(GUID, IMMDevice)>> {
    let list = unsafe { audio::enumerator()?.EnumAudioEndpoints(eAll, DEVICE_STATE(DEVICE_STATEMASK_ALL))? };
    let mut out = Vec::new();
    for i in 0..unsafe { list.GetCount()? } {
        let device = unsafe { list.Item(i)? };
        let container = unsafe {
            let store = device.OpenPropertyStore(STGM_READ)?;
            let mut value = store.GetValue(&PKEY_Device_ContainerId)?;
            let guid = PropVariantToGUID(&value);
            let _ = PropVariantClear(&mut value);
            guid
        };
        if let Ok(container) = container {
            out.push((container, device));
        }
    }
    Ok(out)
}

/// Asks the Bluetooth audio driver behind one of the device's endpoints to connect or disconnect
/// it; the first endpoint that takes the request does it.
fn connect_audio(id: &str, connect: bool) -> Result<bool> {
    mta();
    for (container, device) in audio_containers()? {
        if format!("{container:?}") == id && ks_request(&device, connect).is_ok() {
            return Ok(true);
        }
    }
    Ok(false)
}

fn ks_request(device: &IMMDevice, connect: bool) -> Result<()> {
    unsafe {
        let topology: IDeviceTopology = device.Activate(CLSCTX_ALL, None)?;
        let part: IPart = topology.GetConnector(0)?.GetConnectedTo()?.cast()?;
        let mut ks: Option<IKsControl> = None;
        part.Activate(CLSCTX_ALL.0, &IKsControl::IID, Some(&mut ks as *mut _ as *mut _))?;
        let ks = ks.ok_or_else(|| windows::core::Error::from(windows::Win32::Foundation::E_NOINTERFACE))?;
        // KSPROPERTY_ONESHOT_RECONNECT is 0, KSPROPERTY_ONESHOT_DISCONNECT 1.
        let property = KSIDENTIFIER {
            Anonymous: KSIDENTIFIER_0 {
                Anonymous: KSIDENTIFIER_0_0 { Set: KSPROPSETID_BtAudio, Id: if connect { 0 } else { 1 }, Flags: KSPROPERTY_TYPE_GET },
            },
        };
        let mut returned = 0u32;
        ks.KsProperty(&property, size_of::<KSIDENTIFIER>() as u32, std::ptr::null_mut(), 0, &mut returned)
    }
}

#[tauri::command]
#[specta::specta]
pub async fn bluetooth_state() -> BluetoothState {
    read()
}

#[tauri::command]
#[specta::specta]
pub async fn bluetooth_set_on(app: AppHandle, on: bool) -> std::result::Result<(), String> {
    let result = set_radio(RadioKind::Bluetooth, on);
    publish(&app);
    result
}

/// Connects or disconnects a paired audio device (`id` from the state).
#[tauri::command]
#[specta::specta]
pub async fn bluetooth_connect(app: AppHandle, id: String, connect: bool) -> std::result::Result<(), String> {
    let result = match connect_audio(&id, connect) {
        Ok(true) => Ok(()),
        Ok(false) => Err("Windows didn't take the request; try Bluetooth settings".into()),
        Err(e) => Err(e.to_string()),
    };
    // The driver answers at once but connecting takes a moment: report what Windows says then.
    std::thread::sleep(std::time::Duration::from_secs(2));
    publish(&app);
    result
}

#[cfg(test)]
mod tests {
    /// Prints this PC's Bluetooth state, without changing anything. By hand:
    /// `cargo test bluetooth_live -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn bluetooth_live() {
        println!("{:#?}", super::read());
    }
}
