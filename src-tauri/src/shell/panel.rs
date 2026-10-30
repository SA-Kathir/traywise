//! The panel: one borderless, always-on-top window that the tray icon shows and hides.

use std::ffi::c_void;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use tauri_specta::Event;
use tauri::{AppHandle, Manager, State, WebviewUrl, WebviewWindow, WebviewWindowBuilder, WindowEvent};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
use windows::Win32::Graphics::Dwm::{
    DWMWA_BORDER_COLOR, DWMWA_COLOR_DEFAULT, DWMWA_COLOR_NONE, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_DONOTROUND,
    DWMWCP_ROUND, DwmFlush, DwmSetWindowAttribute,
};
use windows::Win32::Foundation::{HWND, POINT, RECT};
use windows::Win32::Graphics::Gdi::{
    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, CreateCompatibleBitmap, CreateCompatibleDC, DIB_RGB_COLORS, DeleteDC,
    DeleteObject, GetDC, GetDIBits, HALFTONE, ReleaseDC, SRCCOPY, SelectObject, SetStretchBltMode, StretchBlt,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetCursorPos, GetWindowRect, IsIconic, IsWindowVisible, SPI_GETCLIENTAREAANIMATION, SW_HIDE, SW_SHOWNOACTIVATE, SWP_NOACTIVATE, SWP_NOSIZE, SWP_NOZORDER,
    SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, SetWindowPos, ShowWindow, SystemParametersInfoW,
};
use windows::core::BOOL;

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
    /// Opened by resting the mouse on the screen's edge (not the tray or the shortcut): it sits at
    /// the right, centred, doesn't take the keyboard, and closes when the mouse leaves.
    hover: AtomicBool,
    /// An edge-opened panel is sliding out: a second close request waits for it.
    closing: AtomicBool,
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
        WindowEvent::Focused(false) if is_open(&handle) => hide(&handle),
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
    if is_open(&win) {
        hide(&win);
    } else if !just_hidden {
        app.state::<PanelState>().hover.store(false, Ordering::Relaxed);
        show(&win);
    }
}

/// The mouse rested on the screen's edge (`shell::edge`): open the panel there, if it's closed.
pub fn hover_open(app: &AppHandle) {
    if let Some(win) = app.get_webview_window(LABEL)
        && !is_open(&win)
    {
        app.state::<PanelState>().hover.store(true, Ordering::Relaxed);
        show(&win);
    }
}

/// The UI closes an edge-opened panel when the mouse has left it (and it wasn't clicked into).
/// Not while the pointer is beside it, between the panel and the screen's edge (still on the edge
/// it was opened from, say): false then, and the UI asks again a moment later.
#[tauri::command]
#[specta::specta]
pub fn hide_panel(app: AppHandle) -> bool {
    let Some(win) = app.get_webview_window(LABEL) else { return true };
    let mut pointer = POINT::default();
    let mut r = RECT::default();
    if let Ok(hwnd) = win.hwnd()
        && unsafe { GetCursorPos(&mut pointer).is_ok() && GetWindowRect(hwnd, &mut r).is_ok() }
        && beside(&r, pointer)
    {
        return false;
    }
    hide(&win);
    true
}

/// Over the panel or to its right, level with it.
fn beside(panel: &RECT, p: POINT) -> bool {
    p.x >= panel.left && p.y >= panel.top && p.y < panel.bottom
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
        && !is_open(&win)
    {
        app.state::<PanelState>().hover.store(false, Ordering::Relaxed);
        show(&win);
    }
}

/// Shown and not minimised (a minimised window still counts as visible to Windows). Asked of
/// Windows itself: the edge shows the panel with ShowWindow, which Tauri doesn't track.
fn is_open(win: &WebviewWindow) -> bool {
    win.hwnd().is_ok_and(|h| unsafe { IsWindowVisible(h).as_bool() && !IsIconic(h).as_bool() })
}

fn show(win: &WebviewWindow) {
    win.state::<PanelState>().closing.store(false, Ordering::Relaxed);
    *win.state::<PanelState>().shown_at.lock().unwrap() = Some(Instant::now());
    webview::set_active(win, true);
    let hover = win.state::<PanelState>().hover.load(Ordering::Relaxed);
    place(win, hover);
    // Before showing it, so the sample is of what's behind the panel, not the panel.
    let settings = win.state::<SettingsState>().get();
    let backdrop = if framed(&settings) { None } else { win.hwnd().ok().and_then(backdrop_brightness) };
    if hover {
        // Shown without taking the keyboard, so typing elsewhere carries on; a click gives it focus.
        // It slides in from beyond the screen's edge (SW_SHOWNOACTIVATE also un-minimises).
        if let Ok(hwnd) = win.hwnd() {
            let mut r = RECT::default();
            let _ = unsafe { GetWindowRect(hwnd, &mut r) };
            let animate = animations_on();
            if animate {
                move_to(hwnd, offscreen(&r), r.top);
            }
            let _ = unsafe { ShowWindow(hwnd, SW_SHOWNOACTIVATE) };
            if animate {
                slide(hwnd, offscreen(&r), r.left, r.top, SLIDE_IN_MS, |t| 1.0 - (1.0 - t).powi(3), || {});
            }
        }
    } else {
        if win.is_minimized().unwrap_or(false) {
            let _ = win.unminimize();
        }
        let _ = win.show();
        let _ = win.set_focus();
    }
    win.state::<Modules>().panel_visible(true);
    let _ = PanelShown { backdrop, hover }.emit(win);
}

/// Sent to the UI each time the panel opens.
#[derive(Clone, serde::Serialize, specta::Type, tauri_specta::Event)]
pub struct PanelShown {
    /// For a transparent look, how bright the screen behind it is (0 black to 100 white), so the
    /// glass can be more solid over dark things.
    pub backdrop: Option<u8>,
    /// Opened from the screen's edge: the UI closes it when the mouse leaves.
    pub hover: bool,
}

/// How bright the screen is where the panel will appear: Windows shrinks that area to an 8 × 8
/// sample, which is averaged to one number and then deleted. Nothing is kept or sent anywhere.
/// (The owner allowed this for transparent looks only; see CLAUDE.md.)
fn backdrop_brightness(hwnd: HWND) -> Option<u8> {
    let mut r = RECT::default();
    unsafe { GetWindowRect(hwnd, &mut r) }.ok()?;
    let mut pixels = [0u8; 8 * 8 * 4];
    let copied = unsafe {
        let screen = GetDC(None);
        let memory = CreateCompatibleDC(Some(screen));
        let bitmap = CreateCompatibleBitmap(screen, 8, 8);
        let old = SelectObject(memory, bitmap.into());
        SetStretchBltMode(memory, HALFTONE);
        let drawn = StretchBlt(memory, 0, 0, 8, 8, Some(screen), r.left, r.top, r.right - r.left, r.bottom - r.top, SRCCOPY).as_bool();
        SelectObject(memory, old);
        let mut info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: 8,
                biHeight: -8,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let lines = GetDIBits(memory, bitmap, 0, 8, Some(pixels.as_mut_ptr().cast()), &mut info, DIB_RGB_COLORS);
        let _ = DeleteObject(bitmap.into());
        let _ = DeleteDC(memory);
        ReleaseDC(None, screen);
        drawn && lines == 8
    };
    copied.then(|| brightness(&pixels))
}

/// The average perceived brightness of BGRA pixels, 0 to 100.
fn brightness(bgra: &[u8]) -> u8 {
    let n = (bgra.len() / 4).max(1) as f64;
    let sum: f64 = bgra.as_chunks::<4>().0.iter().map(|p| 0.0722 * f64::from(p[0]) + 0.7152 * f64::from(p[1]) + 0.2126 * f64::from(p[2])).sum();
    (sum / n / 255.0 * 100.0).round() as u8
}

fn hide(win: &WebviewWindow) {
    let state = win.state::<PanelState>();
    // An edge-opened panel slides back out first.
    if state.hover.load(Ordering::Relaxed) && is_open(win) && animations_on() && let Ok(hwnd) = win.hwnd() {
        if state.closing.swap(true, Ordering::Relaxed) {
            return; // already sliding out
        }
        let mut r = RECT::default();
        let _ = unsafe { GetWindowRect(hwnd, &mut r) };
        let w = win.clone();
        slide(hwnd, r.left, offscreen(&r), r.top, SLIDE_OUT_MS, |t| t * t * t, move || {
            let w2 = w.clone();
            let _ = w.run_on_main_thread(move || hide_now(&w2));
        });
        return;
    }
    hide_now(win);
}

fn hide_now(win: &WebviewWindow) {
    win.state::<PanelState>().closing.store(false, Ordering::Relaxed);
    // Both: Tauri's hide does nothing when it didn't do the showing (the edge uses ShowWindow).
    let _ = win.hide();
    if let Ok(hwnd) = win.hwnd() {
        let _ = unsafe { ShowWindow(hwnd, SW_HIDE) };
    }
    webview::set_active(win, false);
    win.state::<Modules>().panel_visible(false);
    *win.state::<PanelState>().last_hidden.lock().unwrap() = Some(Instant::now());
    crate::updater::check(win.app_handle());
}

/// The edge drawer's slide: in, easing to a stop; out, speeding away.
const SLIDE_IN_MS: u64 = 200;
const SLIDE_OUT_MS: u64 = 150;

/// Where the panel sits just beyond the screen's right edge (it rests 12 px in from it).
fn offscreen(r: &RECT) -> i32 {
    r.right + 16
}

fn move_to(hwnd: HWND, x: i32, y: i32) {
    let _ = unsafe { SetWindowPos(hwnd, None, x, y, 0, 0, SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE) };
}

/// Moves the window from `from` to `to` (its left edge) over `ms`, once per screen refresh, on a
/// thread of its own; `ease` shapes the motion. Then runs `done`.
fn slide(hwnd: HWND, from: i32, to: i32, y: i32, ms: u64, ease: fn(f64) -> f64, done: impl FnOnce() + Send + 'static) {
    let hwnd = hwnd.0 as isize;
    std::thread::spawn(move || {
        let start = Instant::now();
        loop {
            // From the time elapsed, not a frame count: a late frame doesn't slow the slide.
            let t = (start.elapsed().as_secs_f64() * 1000.0 / ms as f64).min(1.0);
            move_to(HWND(hwnd as _), from + (f64::from(to - from) * ease(t)).round() as i32, y);
            if t >= 1.0 {
                break;
            }
            // Wait for the screen's next frame, so each step lands on one (Sleep is too coarse).
            if unsafe { DwmFlush() }.is_err() {
                std::thread::sleep(Duration::from_millis(8));
            }
        }
        done();
    });
}

/// Windows' "Animation effects" setting (Accessibility › Visual effects): off means no slide.
fn animations_on() -> bool {
    let mut on = BOOL(1);
    let _ = unsafe {
        SystemParametersInfoW(SPI_GETCLIENTAREAANIMATION, 0, Some(&mut on as *mut BOOL as *mut c_void), SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0))
    };
    on.as_bool()
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
/// monitor's work area (above the taskbar), 12 px margin; opened from the edge, at the right, centred.
fn place(win: &WebviewWindow, edge: bool) {
    let (Ok(Some(monitor)), Ok(hwnd)) = (win.primary_monitor(), win.hwnd()) else { return };
    let scale = monitor.scale_factor();
    let area = monitor.work_area();
    let margin = (12.0 * scale) as i32;
    let max_height = area.size.height as i32 - 2 * margin;
    let set = |width: i32, height: i32| {
        let height = height.min(max_height);
        let x = area.position.x + area.size.width as i32 - width - margin;
        let y = if edge {
            area.position.y + (area.size.height as i32 - height) / 2
        } else {
            area.position.y + area.size.height as i32 - height - margin
        };
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
        place(&win, app.state::<PanelState>().hover.load(Ordering::Relaxed));
    }
}

/// Whether the window gets Windows 11's frame: every look except transparent Liquid or Dot, whose cards
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


#[cfg(test)]
mod tests {
    #[test]
    fn beside_the_panel() {
        use windows::Win32::Foundation::{POINT, RECT};
        let panel = RECT { left: 1455, top: 227, right: 1905, bottom: 1027 };
        let at = |x, y| super::beside(&panel, POINT { x, y });
        assert!(at(1919, 600), "on the screen's edge, level with it");
        assert!(at(1600, 600), "over it");
        assert!(!at(1400, 600), "to its left");
        assert!(!at(1919, 100), "on the edge, above it");
    }

    #[test]
    fn brightness() {
        assert_eq!(super::brightness(&[0, 0, 0, 255].repeat(64)), 0);
        assert_eq!(super::brightness(&[255, 255, 255, 255].repeat(64)), 100);
        assert_eq!(super::brightness(&[0, 255, 0, 255, 0, 0, 0, 255].repeat(32)), 36); // half bright green
    }
}
