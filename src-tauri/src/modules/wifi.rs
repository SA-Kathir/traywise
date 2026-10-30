//! Wi-Fi: switch the radio, see the network you're on, and pick another (Windows.Devices.WiFi).
//!
//! Nothing runs in the background. The state is read when the panel opens and after each change;
//! the network list is rescanned only when the Wi-Fi page asks. Since Windows 11 24H2, network
//! names count as location: if location is off for desktop apps, Windows hides them.

use serde::Serialize;
use tauri::AppHandle;
use tauri_specta::Event;
use windows::Devices::Radios::RadioKind;
use windows::Devices::WiFi::{WiFiAccessStatus, WiFiAdapter, WiFiAvailableNetwork, WiFiConnectionStatus, WiFiReconnectionKind};
use windows::Networking::Connectivity::{NetworkAuthenticationType, NetworkInformation};
use windows::Security::Credentials::PasswordCredential;
use windows::core::{HSTRING, Result};

use crate::module::{Module, ModuleId};
use crate::radios::{blocking, mta, radio_on, set_radio};

#[derive(Clone, Debug, Default, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct WifiState {
    /// The radio; None when this PC has no Wi-Fi.
    pub on: Option<bool>,
    /// Windows hides network names: location is off for desktop apps.
    pub names_hidden: bool,
    /// The network in use, if any.
    pub connected: Option<WifiNetwork>,
    /// Networks in range, strongest first (without the connected one).
    pub networks: Vec<WifiNetwork>,
}

#[derive(Clone, Debug, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct WifiNetwork {
    /// Empty when Windows hides names.
    pub ssid: String,
    /// Signal, 0 to 4 bars.
    pub bars: u8,
    /// Needs a password (or more) to join.
    pub secure: bool,
    /// Joined before: Windows has it saved, so no password is needed.
    pub saved: bool,
    /// "2.4 GHz", "5 GHz" or "6 GHz".
    pub band: Option<String>,
}

/// Sent when the panel opens and after each change.
#[derive(Clone, Serialize, specta::Type, tauri_specta::Event)]
pub struct WifiChanged(pub WifiState);

pub struct Wifi {
    app: AppHandle,
}

impl Wifi {
    pub fn new(app: AppHandle) -> Self {
        Self { app }
    }
}

impl Module for Wifi {
    fn id(&self) -> ModuleId {
        ModuleId::Wifi
    }

    fn start(&mut self) -> std::result::Result<(), String> {
        Ok(()) // nothing to start: no thread, no subscriptions
    }

    fn stop(&mut self) {}

    fn panel_visible(&mut self, visible: bool) {
        if visible {
            // Off the UI's thread: Windows can take a moment to answer.
            let app = self.app.clone();
            std::thread::spawn(move || publish(&app));
        }
    }
}

fn publish(app: &AppHandle) {
    let _ = WifiChanged(read()).emit(app);
}

/// Waits on Windows: call off the UI's thread.
fn read() -> WifiState {
    let on = radio_on(RadioKind::WiFi);
    if on != Some(true) {
        return WifiState { on, ..WifiState::default() };
    }
    match read_adapter() {
        Ok(state) => WifiState { on, ..state },
        Err(e) => {
            eprintln!("wifi: {e}");
            WifiState { on, ..WifiState::default() }
        }
    }
}

fn adapter() -> Result<Option<WiFiAdapter>> {
    mta();
    Ok(WiFiAdapter::FindAllAdaptersAsync()?.join()?.into_iter().next())
}

fn read_adapter() -> Result<WifiState> {
    let names_hidden = WiFiAdapter::RequestAccessAsync()?.join()? != WiFiAccessStatus::Allowed;
    let Some(adapter) = adapter()? else { return Ok(WifiState::default()) };
    let ssid = adapter
        .NetworkAdapter()?
        .GetConnectedProfileAsync()?
        .join()
        .ok()
        .and_then(|p| p.WlanConnectionProfileDetails().ok())
        .and_then(|d| d.GetConnectedSsid().ok())
        .map(|s| s.to_string());
    let saved = saved_networks();
    let mut networks: Vec<WifiNetwork> = Vec::new();
    for n in adapter.NetworkReport()?.AvailableNetworks()? {
        let network = describe(&n, &saved)?;
        // One entry per name (a network is often heard on 2.4 and 5 GHz): the strongest.
        match networks.iter_mut().find(|m| m.ssid == network.ssid && !network.ssid.is_empty()) {
            Some(m) if m.bars < network.bars => *m = network,
            Some(_) => {}
            None => networks.push(network),
        }
    }
    networks.sort_by_key(|n| std::cmp::Reverse(n.bars));
    let connected = ssid.map(|ssid| {
        let i = networks.iter().position(|n| n.ssid == ssid);
        let heard = i.map(|i| networks.remove(i));
        heard.unwrap_or(WifiNetwork { ssid, bars: 0, secure: true, saved: true, band: None })
    });
    networks.retain(|n| !n.ssid.is_empty());
    Ok(WifiState { on: Some(true), names_hidden, connected, networks })
}

fn describe(n: &WiFiAvailableNetwork, saved: &[String]) -> Result<WifiNetwork> {
    let ssid = n.Ssid()?.to_string();
    let auth = n.SecuritySettings()?.NetworkAuthenticationType()?;
    Ok(WifiNetwork {
        saved: saved.contains(&ssid),
        secure: auth != NetworkAuthenticationType::Open80211 && auth != NetworkAuthenticationType::None,
        bars: n.SignalBars()?,
        band: band(n.ChannelCenterFrequencyInKilohertz()?),
        ssid,
    })
}

fn band(khz: i32) -> Option<String> {
    match khz {
        1..3_000_000 => Some("2.4 GHz".into()),
        3_000_000..5_925_000 => Some("5 GHz".into()),
        5_925_000.. => Some("6 GHz".into()),
        _ => None,
    }
}

/// The names of the Wi-Fi networks Windows has saved.
fn saved_networks() -> Vec<String> {
    let Ok(profiles) = NetworkInformation::GetConnectionProfiles() else { return Vec::new() };
    profiles
        .into_iter()
        .filter(|p| p.IsWlanConnectionProfile().unwrap_or(false))
        .filter_map(|p| p.ProfileName().ok().map(|n| n.to_string()))
        .collect()
}

/// The current state, with the list as Windows last heard it (`wifi_scan` refreshes it).
#[tauri::command]
#[specta::specta]
pub async fn wifi_state() -> WifiState {
    blocking(read).await
}

/// Looks for networks again (a few seconds), then sends the new list.
#[tauri::command]
#[specta::specta]
pub async fn wifi_scan(app: AppHandle) -> std::result::Result<(), String> {
    blocking(move || {
        if let Some(adapter) = adapter().map_err(|e| e.to_string())? {
            adapter.ScanAsync().and_then(|scan| scan.join()).map_err(|e| e.to_string())?;
        }
        publish(&app);
        Ok(())
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn wifi_set_on(app: AppHandle, on: bool) -> std::result::Result<(), String> {
    blocking(move || {
        let result = set_radio(RadioKind::WiFi, on);
        publish(&app);
        result
    })
    .await
}

/// Joins a network. A secured network Windows hasn't saved needs `password`; Windows saves it
/// (as if joined in its own Settings) and Traywise keeps nothing.
#[tauri::command]
#[specta::specta]
pub async fn wifi_connect(app: AppHandle, ssid: String, password: Option<String>) -> std::result::Result<(), String> {
    blocking(move || {
        let result = connect(&ssid, password).unwrap_or_else(|e| Err(e.to_string()));
        publish(&app);
        result
    })
    .await
}

fn connect(ssid: &str, password: Option<String>) -> Result<std::result::Result<(), String>> {
    let Some(adapter) = adapter()? else { return Ok(Err("no Wi-Fi on this PC".into())) };
    let report = adapter.NetworkReport()?;
    // The strongest of the networks with this name.
    let mut best: Option<WiFiAvailableNetwork> = None;
    for n in report.AvailableNetworks()? {
        if n.Ssid()? == ssid && best.as_ref().is_none_or(|b| b.SignalBars().unwrap_or(0) < n.SignalBars().unwrap_or(0)) {
            best = Some(n);
        }
    }
    let Some(network) = best else { return Ok(Err(format!("{ssid} is out of range"))) };
    let result = match password {
        Some(p) => {
            let credential = PasswordCredential::new()?;
            credential.SetPassword(&HSTRING::from(p))?;
            adapter.ConnectWithPasswordCredentialAsync(&network, WiFiReconnectionKind::Automatic, &credential)?.join()?
        }
        None => adapter.ConnectAsync(&network, WiFiReconnectionKind::Automatic)?.join()?,
    };
    Ok(match result.ConnectionStatus()? {
        WiFiConnectionStatus::Success => Ok(()),
        WiFiConnectionStatus::InvalidCredential => Err("wrong password".into()),
        WiFiConnectionStatus::Timeout => Err("the network didn't answer".into()),
        WiFiConnectionStatus::NetworkNotAvailable => Err(format!("{ssid} is out of range")),
        WiFiConnectionStatus::UnsupportedAuthenticationProtocol => Err("this network needs Windows' own Wi-Fi settings".into()),
        status => Err(format!("couldn't connect ({status:?})")),
    })
}

#[tauri::command]
#[specta::specta]
pub async fn wifi_disconnect(app: AppHandle) -> std::result::Result<(), String> {
    blocking(move || {
        let result = adapter().map_err(|e| e.to_string()).and_then(|a| a.map_or(Ok(()), |a| a.Disconnect().map_err(|e| e.to_string())));
        publish(&app);
        result
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bands() {
        assert_eq!(band(2_437_000).as_deref(), Some("2.4 GHz"));
        assert_eq!(band(5_180_000).as_deref(), Some("5 GHz"));
        assert_eq!(band(5_955_000).as_deref(), Some("6 GHz"));
        assert_eq!(band(0), None);
    }

    /// Prints this PC's Wi-Fi state, without changing anything. By hand:
    /// `cargo test wifi_live -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn wifi_live() {
        let s = read();
        println!("on {:?}, names hidden {}, connected {:?}, {} networks", s.on, s.names_hidden, s.connected, s.networks.len());
    }
}
