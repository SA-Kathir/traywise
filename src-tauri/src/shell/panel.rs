//! The panel: one borderless, always-on-top window that the tray icon shows and hides.

use std::ffi::c_void;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use tauri_specta::Event;
use tauri::{AppHandle, Manager, State, WebviewUrl, WebviewWindow, WebviewWindowBuilder, WindowEvent};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
use windows::Win32::Graphics::Dwm::{
    DWMWA_BORDER_COLOR, DWMWA_COLOR_DEFAULT, DWMWA_COLOR_NONE, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_DONOTROUND,
    DWMWCP_ROUND, DwmSetWindowAttribute,
};
use windows::Win32::UI::WindowsAndMessaging::{SWP_NOACTIVATE, SWP_NOZORDER, SetWindowPos};

use super::webview;
use crate::module::Modules;
use crate::settings::{Look, Settings, SettingsState};

const LABEL: &str = "panel";
/// A tray click right after the panel lost focus is the same click that closed it, so ignore it.
const REOPEN_GUARD: Duration = Duration::from_millis(250);
/// Panel size in logical pixels. The height is fixed, so nothing moves when switching pages; what
/// doesn't fit scrolls.
const WIDTH: f64 = 360.0;
const HEIGHT: f64 = 640.0;

#[derive(Default)]
pub struct PanelState {
    /// When the panel was last hidden, for the reopen guard.
    last_hidden: Mutex<Option<Instant>>,
    /// When the panel was last asked to show, for timing how long it takes to appear.
    shown_at: Mutex<Option<Instant>>,
    /// The registered open/close shortcut.
    shortcut: Mutex<Option<String>>,
}

pub fn create(app: &tauri::App) -> tauri::Result<()> {
    app.manage(PanelState::default());
    let win = WebviewWindowBuilder::new(app, LABEL, WebviewUrl::default())
        .title("Traywise")
        .inner_size(WIDTH, HEIGHT)
        .visible(false)
        .decorations(false)
        .transparent(true)
        // Same setup the spike's numbers were measured with.
        .no_redirection_bitmap(true)
        .resizable(false)
        .always_on_top(true)
        .skip_taskbar(true)
        // Set per look by `frame`.
        .shadow(false)
        .build()?;
    frame(&win, framed(&app.state::<SettingsState>().get()));
    webview::set_active(&win, false);

    let handle = win.clone();
    win.on_window_event(move |event| match event {
        // Clicking anywhere else closes the panel, like the Windows volume and network flyouts.
        WindowEvent::Focused(false) if handle.is_visible().unwrap_or(false) => hide(&handle),
        // Alt+F4 hides instead of quitting; quit from the tray menu.
        WindowEvent::CloseRequested { api, .. } => {
            api.prevent_close();
            hide(&handle);
        }
        _ => {}
    });
    Ok(())
}

pub fn toggle(app: &AppHandle) {
    let Some(win) = app.get_webview_window(LABEL) else { return };
    let just_hidden = app
        .state::<PanelState>()
        .last_hidden
        .lock()
        .unwrap()
        .is_some_and(|t| t.elapsed() < REOPEN_GUARD);
    if win.is_visible().unwrap_or(false) {
        hide(&win);
    } else if !just_hidden {
        show(&win);
    }
}

/// Registers the shortcut that opens or closes the panel, replacing the previous one. Fails (keeping
/// the previous one) if Windows or another app already uses those keys.
pub fn set_shortcut(app: &AppHandle, keys: &str) -> Result<(), String> {
    let state = app.state::<PanelState>();
    let mut current = state.shortcut.lock().unwrap();
    if current.as_deref() == Some(keys) {
        return Ok(());
    }
    app.global_shortcut()
        .on_shortcut(keys, |app, _, event| {
            if event.state == ShortcutState::Pressed {
                toggle(app);
            }
        })
        .map_err(|e| format!("couldn't use {keys} to open Traywise: {e}"))?;
    if let Some(old) = current.replace(keys.to_owned()) {
        let _ = app.global_shortcut().unregister(old.as_str());
    }
    Ok(())
}

/// Opens the panel if it isn't already open. Used when Traywise is launched a second time.
pub fn open(app: &AppHandle) {
    if let Some(win) = app.get_webview_window(LABEL)
        && !win.is_visible().unwrap_or(false)
    {
        show(&win);
    }
}

fn show(win: &WebviewWindow) {
    *win.state::<PanelState>().shown_at.lock().unwrap() = Some(Instant::now());
    webview::set_active(win, true);
    place(win);
    let _ = win.show();
    let _ = win.set_focus();
    win.state::<Modules>().panel_visible(true);
    let _ = PanelShown.emit(win);
}

/// Sent to the UI each time the panel opens.
#[derive(Clone, serde::Serialize, specta::Type, tauri_specta::Event)]
pub struct PanelShown;

fn hide(win: &WebviewWindow) {
    let _ = win.hide();
    webview::set_active(win, false);
    win.state::<Modules>().panel_visible(false);
    *win.state::<PanelState>().last_hidden.lock().unwrap() = Some(Instant::now());
    crate::updater::check(win.app_handle());
}

/// The panel's Quit button, the same as the tray menu's.
#[tauri::command]
#[specta::specta]
pub fn quit(app: AppHandle) {
    app.exit(0);
}

/// Called by the UI once it has drawn after `panel-shown`. Prints the click-to-drawn time,
/// visible when traywise.exe is started from a terminal.
#[tauri::command]
#[specta::specta]
pub fn panel_painted(state: State<PanelState>) {
    if let Some(shown_at) = state.shown_at.lock().unwrap().take() {
        println!("panel opened in {:.1} ms", shown_at.elapsed().as_secs_f64() * 1000.0);
    }
}

/// Measurement builds only: opens and closes the panel 10 times on its own, so open time and
/// memory after real use can be measured without clicking the tray.
#[cfg(feature = "autotest")]
pub fn autotest(app: AppHandle) {
    std::thread::spawn(move || {
        let Some(win) = app.get_webview_window(LABEL) else { return };
        std::thread::sleep(Duration::from_secs(3));
        for _ in 0..10 {
            show(&win);
            std::thread::sleep(Duration::from_millis(400));
            hide(&win);
            std::thread::sleep(Duration::from_millis(200));
        }
    });
}

/// Sizes the panel (shorter if the screen is) and puts it in the bottom-right corner of the primary
/// monitor's work area (above the taskbar), 12 px margin.
fn place(win: &WebviewWindow) {
    let (Ok(Some(monitor)), Ok(hwnd)) = (win.primary_monitor(), win.hwnd()) else { return };
    let scale = monitor.scale_factor();
    let area = monitor.work_area();
    let margin = (12.0 * scale) as i32;
    let max_height = area.size.height as i32 - 2 * margin;
    let set = |width: i32, height: i32| {
        let height = height.min(max_height);
        let x = area.position.x + area.size.width as i32 - width - margin;
        let y = area.position.y + area.size.height as i32 - height - margin;
        let _ = unsafe { SetWindowPos(hwnd, None, x, y, width, height, SWP_NOZORDER | SWP_NOACTIVATE) };
    };
    let (width, height) = ((WIDTH * scale).round() as i32, (HEIGHT * scale).ceil() as i32);
    set(width, height);
    // A framed window's shadow takes a few pixels of it: add them back, so the page gets
    // WIDTH x HEIGHT either way. Unframed, there's nothing to add (and Windows briefly misreports
    // a frame right after resizing, so don't measure there).
    if framed(&win.state::<SettingsState>().get()) && let Ok(inner) = win.inner_size() {
        let (frame_w, frame_h) = (width - inner.width as i32, height.min(max_height) - inner.height as i32);
        if frame_w != 0 || frame_h != 0 {
            set(width + frame_w, height + frame_h);
        }
    }
}

/// Called after Settings are saved: the look or background may have changed.
pub fn refresh_frame(app: &AppHandle) {
    if let Some(win) = app.get_webview_window(LABEL) {
        frame(&win, framed(&app.state::<SettingsState>().get()));
        place(&win);
    }
}

/// Whether the window gets Windows 11's frame: every look except transparent Liquid, whose cards
/// float over the desktop, where a frame would outline empty space.
fn framed(settings: &Settings) -> bool {
    settings.look == Look::Windows || !settings.transparent
}

/// Windows 11's rounded corners, 1 px border and drop shadow, like Windows' own flyouts, or none
/// of them (rounded corners alone bring the system shadow and border, so they go together).
fn frame(win: &WebviewWindow, framed: bool) {
    let _ = win.set_shadow(framed);
    let Ok(hwnd) = win.hwnd() else { return };
    let corners = if framed { DWMWCP_ROUND } else { DWMWCP_DONOTROUND };
    let color = if framed { DWMWA_COLOR_DEFAULT } else { DWMWA_COLOR_NONE };
    unsafe {
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_WINDOW_CORNER_PREFERENCE,
            &corners as *const _ as *const c_void,
            size_of_val(&corners) as u32,
        );
        let _ =
            DwmSetWindowAttribute(hwnd, DWMWA_BORDER_COLOR, &color as *const _ as *const c_void, size_of_val(&color) as u32);
    }
}

