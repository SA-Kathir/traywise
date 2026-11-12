//! The displays thread and its Windows calls.

use std::collections::HashMap;
use std::ptr;
use std::sync::mpsc::Receiver;
use std::time::Duration;

use tauri::{AppHandle, Manager};
use tauri_specta::Event;
use windows::Win32::Devices::Display::{
    DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME, DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME, DISPLAYCONFIG_DEVICE_INFO_HEADER,
    DISPLAYCONFIG_MODE_INFO, DISPLAYCONFIG_OUTPUT_TECHNOLOGY_DISPLAYPORT_EMBEDDED, DISPLAYCONFIG_OUTPUT_TECHNOLOGY_INTERNAL,
    DISPLAYCONFIG_OUTPUT_TECHNOLOGY_LVDS, DISPLAYCONFIG_OUTPUT_TECHNOLOGY_UDI_EMBEDDED, DISPLAYCONFIG_PATH_INFO,
    DISPLAYCONFIG_SOURCE_DEVICE_NAME, DISPLAYCONFIG_TARGET_DEVICE_NAME, DISPLAYCONFIG_TOPOLOGY_CLONE, DISPLAYCONFIG_TOPOLOGY_EXTEND,
    DISPLAYCONFIG_TOPOLOGY_EXTERNAL, DISPLAYCONFIG_TOPOLOGY_ID, DISPLAYCONFIG_TOPOLOGY_INTERNAL, DestroyPhysicalMonitors,
    DisplayConfigGetDeviceInfo, GetDisplayConfigBufferSizes, GetNumberOfPhysicalMonitorsFromHMONITOR, GetPhysicalMonitorsFromHMONITOR,
    GetVCPFeatureAndVCPFeatureReply, PHYSICAL_MONITOR, QDC_ALL_PATHS, QDC_DATABASE_CURRENT, QDC_ONLY_ACTIVE_PATHS,
    QUERY_DISPLAY_CONFIG_FLAGS, QueryDisplayConfig, SDC_APPLY, SDC_TOPOLOGY_CLONE, SDC_TOPOLOGY_EXTEND, SDC_TOPOLOGY_EXTERNAL,
    SDC_TOPOLOGY_INTERNAL, SetDisplayConfig, SetVCPFeature,
};
use windows::Win32::Foundation::{HANDLE, LPARAM, RECT};
use windows::Win32::Graphics::Gdi::{EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFO, MONITORINFOEXW};
use windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, CoCreateInstance, CoSetProxyBlanket, EOAC_NONE, RPC_C_AUTHN_LEVEL_CALL, RPC_C_IMP_LEVEL_IMPERSONATE,
};
use windows::Win32::System::Variant::VARIANT;
use windows::Win32::System::Wmi::{
    IWbemClassObject, IWbemLocator, IWbemServices, WBEM_FLAG_FORWARD_ONLY, WBEM_FLAG_RETURN_IMMEDIATELY,
    WBEM_FLAG_RETURN_WBEM_COMPLETE, WBEM_INFINITE, WbemLocator,
};
use windows::core::{BOOL, BSTR, PCWSTR, Result, w};

use super::{Display, DisplaysChanged, DisplaysState, Msg, Project, Shared, coalesce, from_percent, to_percent};
use crate::audio::with_com;

/// The VCP code for brightness (DDC/CI's "luminance").
const BRIGHTNESS: u8 = 0x10;

pub fn run(app: AppHandle, rx: Receiver<Msg>) {
    with_com("displays", || {
        // Open only while the panel is.
        let mut screens: Option<Screens> = None;
        while let Ok(first) = rx.recv() {
            for msg in coalesce(std::iter::once(first).chain(rx.try_iter()).collect()) {
                match msg {
                    Msg::PanelVisible(true) => screens = Some(refresh(&app)),
                    Msg::PanelVisible(false) => screens = None,
                    Msg::Brightness(id, percent) => {
                        if let Some(screens) = &screens {
                            screens.set(&id, percent);
                            remember(&app, &id, percent);
                        }
                    }
                    Msg::Project(project) => {
                        drop(screens.take()); // the handles are about to go stale
                        set_project(project);
                        // shortcut: screens take a moment to come up after a switch; one fixed wait
                        // instead of listening for display changes. Listen if it often reads too early.
                        std::thread::sleep(Duration::from_millis(1500));
                        screens = Some(refresh(&app));
                    }
                    Msg::Stop => return Ok(()),
                }
            }
        }
        Ok(())
    });
}

/// Reads the screens, publishes them, and keeps what's needed to change their brightness.
fn refresh(app: &AppHandle) -> Screens {
    let (screens, state) = Screens::read();
    *app.state::<Shared>().state.lock().unwrap() = state.clone();
    let _ = DisplaysChanged(state).emit(app);
    screens
}

/// Stores a new level and tells every tile showing that screen (the slider being dragged shows
/// its own position meanwhile, so the echo doesn't fight it).
fn remember(app: &AppHandle, id: &str, percent: u8) {
    let shared = app.state::<Shared>();
    let mut state = shared.state.lock().unwrap();
    if let Some(d) = state.displays.iter_mut().find(|d| d.id == id) {
        d.brightness = Some(percent);
        let _ = DisplaysChanged(state.clone()).emit(app);
    }
}

/// What's needed to change brightness: the laptop screen's WMI connection, and each external
/// monitor's DDC/CI handle with its maximum.
struct Screens {
    internal: Option<Wmi>,
    external: HashMap<String, (HANDLE, u32)>,
    /// Every handle Windows gave out, to give back.
    physical: Vec<PHYSICAL_MONITOR>,
}

impl Screens {
    fn read() -> (Self, DisplaysState) {
        let mut screens = Screens { internal: None, external: HashMap::new(), physical: Vec::new() };
        let mut displays = Vec::new();
        let monitors = monitors();
        // Targets (screens) grouped by source (what Windows draws): in Duplicate, two screens share one.
        let mut by_source: Vec<(String, Vec<Target>)> = Vec::new();
        for target in active_targets() {
            match by_source.iter_mut().find(|(s, _)| *s == target.source) {
                Some((_, targets)) => targets.push(target),
                None => by_source.push((target.source.clone(), vec![target])),
            }
        }
        for (source, targets) in by_source {
            // The source's DDC/CI-capable monitors, in Windows' order; the laptop's own screen never is one.
            let mut capable = Vec::new();
            if let Some(&hmonitor) = monitors.get(&source) {
                for m in physical_monitors(hmonitor) {
                    if let Some((value, max)) = read_vcp(m.hPhysicalMonitor) {
                        capable.push((m.hPhysicalMonitor, value, max));
                    }
                    screens.physical.push(m);
                }
            }
            let mut capable = capable.into_iter();
            for t in targets {
                let brightness = if t.internal {
                    screens.internal = Wmi::connect().inspect_err(|e| eprintln!("displays: WMI: {e}")).ok();
                    screens.internal.as_ref().and_then(|w| w.brightness().ok())
                } else {
                    // shortcut: in Duplicate with two external monitors, which handle is which screen is a
                    // guess (Windows' order); try with two real monitors if one slider moves the other.
                    capable.next().map(|(handle, value, max)| {
                        screens.external.insert(t.id.clone(), (handle, max));
                        to_percent(value, max)
                    })
                };
                // The built-in screen's own name is a model code, like "B160UAN07.K".
                let name = match (t.internal, t.name.is_empty()) {
                    (true, _) => "Built-in display".into(),
                    (false, false) => t.name,
                    (false, true) => format!("Display {}", displays.len() + 1),
                };
                displays.push(Display { id: t.id, name, internal: t.internal, brightness });
            }
        }
        displays.sort_by_key(|d| !d.internal);
        let project = (connected_screens() > 1).then(topology).flatten();
        (screens, DisplaysState { displays, project })
    }

    fn set(&self, id: &str, percent: u8) {
        if let Some(&(handle, max)) = self.external.get(id) {
            // Some monitors drop a command now and then (Twinkle Tray retries too).
            let sent = (0..2).any(|_| unsafe { SetVCPFeature(handle, BRIGHTNESS, from_percent(percent, max)) } != 0);
            if !sent {
                eprintln!("displays: monitor {id} didn't take the brightness");
            }
        } else if let Some(wmi) = &self.internal
            && let Err(e) = wmi.set_brightness(percent)
        {
            eprintln!("displays: couldn't set the laptop screen's brightness: {e}");
        }
    }
}

impl Drop for Screens {
    fn drop(&mut self) {
        if !self.physical.is_empty() {
            let _ = unsafe { DestroyPhysicalMonitors(&self.physical) };
        }
    }
}

/// (current, maximum) brightness over DDC/CI, or None if the monitor doesn't answer.
fn read_vcp(handle: HANDLE) -> Option<(u32, u32)> {
    let (mut value, mut max) = (0, 0);
    (0..2)
        .any(|_| unsafe { GetVCPFeatureAndVCPFeatureReply(handle, BRIGHTNESS, None, &mut value, Some(&mut max)) } != 0)
        .then_some((value, max))
}

/// A screen Windows is drawing on.
struct Target {
    id: String,
    /// The GDI name of what's drawn on it, like `\\.\DISPLAY1`.
    source: String,
    name: String,
    internal: bool,
}

fn paths(flags: QUERY_DISPLAY_CONFIG_FLAGS, topology: Option<*mut DISPLAYCONFIG_TOPOLOGY_ID>) -> Vec<DISPLAYCONFIG_PATH_INFO> {
    let (mut n_paths, mut n_modes) = (0, 0);
    if unsafe { GetDisplayConfigBufferSizes(flags, &mut n_paths, &mut n_modes) }.is_err() {
        return Vec::new();
    }
    let mut paths = vec![DISPLAYCONFIG_PATH_INFO::default(); n_paths as usize];
    let mut modes = vec![DISPLAYCONFIG_MODE_INFO::default(); n_modes as usize];
    let read = unsafe { QueryDisplayConfig(flags, &mut n_paths, paths.as_mut_ptr(), &mut n_modes, modes.as_mut_ptr(), topology) };
    if read.is_err() {
        return Vec::new();
    }
    paths.truncate(n_paths as usize);
    paths
}

/// Whether a screen that isn't the laptop's own is on.
pub(crate) fn external_connected() -> bool {
    active_targets().iter().any(|t| !t.internal)
}

fn active_targets() -> Vec<Target> {
    paths(QDC_ONLY_ACTIVE_PATHS, None)
        .iter()
        .map(|p| {
            let mut source = DISPLAYCONFIG_SOURCE_DEVICE_NAME {
                header: header(DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME, size_of::<DISPLAYCONFIG_SOURCE_DEVICE_NAME>()),
                ..Default::default()
            };
            source.header.adapterId = p.sourceInfo.adapterId;
            source.header.id = p.sourceInfo.id;
            let mut target = DISPLAYCONFIG_TARGET_DEVICE_NAME {
                header: header(DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME, size_of::<DISPLAYCONFIG_TARGET_DEVICE_NAME>()),
                ..Default::default()
            };
            target.header.adapterId = p.targetInfo.adapterId;
            target.header.id = p.targetInfo.id;
            unsafe {
                DisplayConfigGetDeviceInfo(&mut source.header);
                DisplayConfigGetDeviceInfo(&mut target.header);
            }
            let tech = target.outputTechnology;
            Target {
                id: format!("{:x}-{:x}-{}", p.targetInfo.adapterId.HighPart, p.targetInfo.adapterId.LowPart, p.targetInfo.id),
                source: text(&source.viewGdiDeviceName),
                name: text(&target.monitorFriendlyDeviceName).trim().to_string(),
                internal: [
                    DISPLAYCONFIG_OUTPUT_TECHNOLOGY_INTERNAL,
                    DISPLAYCONFIG_OUTPUT_TECHNOLOGY_DISPLAYPORT_EMBEDDED,
                    DISPLAYCONFIG_OUTPUT_TECHNOLOGY_LVDS,
                    DISPLAYCONFIG_OUTPUT_TECHNOLOGY_UDI_EMBEDDED,
                ]
                .contains(&tech),
            }
        })
        .collect()
}

fn header(kind: windows::Win32::Devices::Display::DISPLAYCONFIG_DEVICE_INFO_TYPE, size: usize) -> DISPLAYCONFIG_DEVICE_INFO_HEADER {
    DISPLAYCONFIG_DEVICE_INFO_HEADER { r#type: kind, size: size as u32, ..Default::default() }
}

fn text(wide: &[u16]) -> String {
    String::from_utf16_lossy(&wide[..wide.iter().position(|&c| c == 0).unwrap_or(wide.len())])
}

/// Screens plugged in, whether or not Windows is drawing on them.
fn connected_screens() -> usize {
    let mut seen: Vec<(i32, u32, u32)> = Vec::new();
    for p in paths(QDC_ALL_PATHS, None).iter().filter(|p| p.targetInfo.targetAvailable.as_bool()) {
        let key = (p.targetInfo.adapterId.HighPart, p.targetInfo.adapterId.LowPart, p.targetInfo.id);
        if !seen.contains(&key) {
            seen.push(key);
        }
    }
    seen.len()
}

fn topology() -> Option<Project> {
    let mut id = DISPLAYCONFIG_TOPOLOGY_ID::default();
    paths(QDC_DATABASE_CURRENT, Some(&mut id));
    match id {
        DISPLAYCONFIG_TOPOLOGY_INTERNAL => Some(Project::PcOnly),
        DISPLAYCONFIG_TOPOLOGY_CLONE => Some(Project::Duplicate),
        DISPLAYCONFIG_TOPOLOGY_EXTEND => Some(Project::Extend),
        DISPLAYCONFIG_TOPOLOGY_EXTERNAL => Some(Project::SecondOnly),
        _ => None,
    }
}

/// What Win+P does.
fn set_project(project: Project) {
    let topology = match project {
        Project::PcOnly => SDC_TOPOLOGY_INTERNAL,
        Project::Duplicate => SDC_TOPOLOGY_CLONE,
        Project::Extend => SDC_TOPOLOGY_EXTEND,
        Project::SecondOnly => SDC_TOPOLOGY_EXTERNAL,
    };
    let status = unsafe { SetDisplayConfig(None, None, SDC_APPLY | topology) };
    if status != 0 {
        eprintln!("displays: Windows didn't switch to {project:?} (error {status})");
    }
}

/// Each monitor's GDI name (like `\\.\DISPLAY1`) and handle.
fn monitors() -> HashMap<String, HMONITOR> {
    unsafe extern "system" fn found(monitor: HMONITOR, _: HDC, _: *mut RECT, list: LPARAM) -> BOOL {
        let list = unsafe { &mut *(list.0 as *mut HashMap<String, HMONITOR>) };
        let mut info = MONITORINFOEXW { monitorInfo: MONITORINFO { cbSize: size_of::<MONITORINFOEXW>() as u32, ..Default::default() }, ..Default::default() };
        if unsafe { GetMonitorInfoW(monitor, &mut info.monitorInfo) }.as_bool() {
            list.insert(text(&info.szDevice), monitor);
        }
        true.into()
    }
    let mut list = HashMap::new();
    let _ = unsafe { EnumDisplayMonitors(None, None, Some(found), LPARAM(&mut list as *mut _ as isize)) };
    list
}

fn physical_monitors(monitor: HMONITOR) -> Vec<PHYSICAL_MONITOR> {
    let mut count = 0;
    if unsafe { GetNumberOfPhysicalMonitorsFromHMONITOR(monitor, &mut count) }.is_err() || count == 0 {
        return Vec::new();
    }
    let mut list = vec![PHYSICAL_MONITOR::default(); count as usize];
    match unsafe { GetPhysicalMonitorsFromHMONITOR(monitor, &mut list) } {
        Ok(()) => list,
        Err(_) => Vec::new(),
    }
}

/// The laptop screen's brightness, through WMI's `root\wmi` classes.
struct Wmi {
    services: IWbemServices,
    /// The `WmiMonitorBrightnessMethods` instance, and `WmiSetBrightness`'s parameters.
    path: BSTR,
    params: IWbemClassObject,
}

impl Wmi {
    fn connect() -> Result<Self> {
        let locator: IWbemLocator = unsafe { CoCreateInstance(&WbemLocator, None, CLSCTX_INPROC_SERVER)? };
        let empty = BSTR::new();
        let services = unsafe { locator.ConnectServer(&BSTR::from(r"ROOT\WMI"), &empty, &empty, &empty, 0, &empty, None)? };
        // Without impersonation (an app's default is only "identify") WMI refuses with access denied.
        // 10 and 0: RPC_C_AUTHN_WINNT and RPC_C_AUTHZ_NONE.
        unsafe {
            CoSetProxyBlanket(&services, 10, 0, PCWSTR::null(), RPC_C_AUTHN_LEVEL_CALL, RPC_C_IMP_LEVEL_IMPERSONATE, None, EOAC_NONE)?;
        }
        let methods = first(&services, "SELECT __PATH FROM WmiMonitorBrightnessMethods WHERE Active=TRUE")?;
        let path = BSTR::try_from(&get(&methods, w!("__PATH"))?)?;
        let mut class = None;
        unsafe {
            services.GetObject(&BSTR::from("WmiMonitorBrightnessMethods"), WBEM_FLAG_RETURN_WBEM_COMPLETE, None, Some(&mut class), None)?;
        }
        let mut signature = None;
        unsafe { class.ok_or_else(missing)?.GetMethod(w!("WmiSetBrightness"), 0, &mut signature, ptr::null_mut())? };
        let params = unsafe { signature.ok_or_else(missing)?.SpawnInstance(0)? };
        Ok(Self { services, path, params })
    }

    fn brightness(&self) -> Result<u8> {
        let row = first(&self.services, "SELECT CurrentBrightness FROM WmiMonitorBrightness WHERE Active=TRUE")?;
        Ok(i32::try_from(&get(&row, w!("CurrentBrightness"))?)?.clamp(0, 100) as u8)
    }

    fn set_brightness(&self, percent: u8) -> Result<()> {
        unsafe {
            // Timeout is a uint32 and Brightness a uint8; WMI wants them as VT_I4 and VT_UI1.
            self.params.Put(w!("Timeout"), 0, &VARIANT::from(0i32), 0)?;
            self.params.Put(w!("Brightness"), 0, &VARIANT::from(percent.min(100)), 0)?;
            self.services.ExecMethod(&self.path, &BSTR::from("WmiSetBrightness"), Default::default(), None, &self.params, None, None)
        }
    }
}

fn first(services: &IWbemServices, query: &str) -> Result<IWbemClassObject> {
    let rows = unsafe {
        services.ExecQuery(&BSTR::from("WQL"), &BSTR::from(query), WBEM_FLAG_FORWARD_ONLY | WBEM_FLAG_RETURN_IMMEDIATELY, None)?
    };
    let mut row = [None];
    let mut returned = 0;
    unsafe { rows.Next(WBEM_INFINITE, &mut row, &mut returned).ok()? };
    row[0].take().ok_or_else(missing)
}

fn get(object: &IWbemClassObject, name: PCWSTR) -> Result<VARIANT> {
    let mut value = VARIANT::default();
    unsafe { object.Get(name, 0, &mut value, None, None)? };
    Ok(value)
}

/// No laptop screen in WMI: a desktop, or a screen whose brightness Windows can't set.
fn missing() -> windows::core::Error {
    windows::core::Error::from_hresult(windows::Win32::Foundation::E_NOTIMPL)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Reads (and sets back) this PC's screens: `cargo test displays_live -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn displays_live() {
        with_com("displays test", || {
            let (screens, state) = Screens::read();
            println!("{state:#?}");
            if let Some(d) = state.displays.iter().find(|d| d.brightness.is_some()) {
                screens.set(&d.id, d.brightness.unwrap()); // the same value: nothing visibly changes
            }
            Ok(())
        });
    }
}
