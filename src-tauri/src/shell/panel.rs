//! The panel: one borderless, always-on-top window that the tray icon shows and hides.

use std::ffi::c_void;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use tauri_specta::Event;
use tauri::{
    AppHandle, Manager, PhysicalPosition, State, WebviewUrl, WebviewWindow, WebviewWindowBuilder, WindowEvent,
};
use windows::Win32::Graphics::Dwm::{DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND, DwmSetWindowAttribute};

use super::webview;

const LABEL: &str = "panel";
/// A tray click right after the panel lost focus is the same click that closed it, so ignore it.
const REOPEN_GUARD: Duration = Duration::from_millis(250);

#[derive(Default)]
pub struct PanelState {
    /// When the panel was last hidden, for the reopen guard.
    last_hidden: Mutex<Option<Instant>>,
    /// When the panel was last asked to show, for timing how long it takes to appear.
    shown_at: Mutex<Option<Instant>>,
}

pub fn create(app: &tauri::App) -> tauri::Result<()> {
    app.manage(PanelState::default());
    let win = WebviewWindowBuilder::new(app, LABEL, WebviewUrl::default())
        .title("Traywise")
        .inner_size(360.0, 560.0)
        .visible(false)
        .decorations(false)
        .transparent(true)
        // Same setup the spike's numbers were measured with.
        .no_redirection_bitmap(true)
        .resizable(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .shadow(true)
        .build()?;
    round_corners(&win);
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

fn show(win: &WebviewWindow) {
    *win.state::<PanelState>().shown_at.lock().unwrap() = Some(Instant::now());
    webview::set_active(win, true);
    let _ = win.set_position(home_position(win));
    let _ = win.show();
    let _ = win.set_focus();
    let _ = PanelShown.emit(win);
}

/// Sent to the UI each time the panel opens.
#[derive(Clone, serde::Serialize, specta::Type, tauri_specta::Event)]
pub struct PanelShown;

fn hide(win: &WebviewWindow) {
    let _ = win.hide();
    webview::set_active(win, false);
    *win.state::<PanelState>().last_hidden.lock().unwrap() = Some(Instant::now());
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

/// Bottom-right corner of the primary monitor's work area (above the taskbar), 12 px margin.
fn home_position(win: &WebviewWindow) -> PhysicalPosition<i32> {
    let Ok(Some(monitor)) = win.primary_monitor() else {
        return PhysicalPosition { x: 100, y: 100 };
    };
    let area = monitor.work_area();
    let size = win.outer_size().unwrap_or_default();
    let margin = (12.0 * monitor.scale_factor()) as i32;
    PhysicalPosition {
        x: area.position.x + area.size.width as i32 - size.width as i32 - margin,
        y: area.position.y + area.size.height as i32 - size.height as i32 - margin,
    }
}

/// Windows 11 rounded corners. Borderless windows get square ones unless they ask.
fn round_corners(win: &WebviewWindow) {
    let Ok(hwnd) = win.hwnd() else { return };
    let _ = unsafe {
        DwmSetWindowAttribute(
            hwnd,
            DWMWA_WINDOW_CORNER_PREFERENCE,
            &DWMWCP_ROUND as *const _ as *const c_void,
            size_of_val(&DWMWCP_ROUND) as u32,
        )
    };
}
