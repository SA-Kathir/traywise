//! The panel: one borderless, always-on-top window that the tray icon shows and hides.

use std::ffi::c_void;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::time::{Duration, Instant};

use tauri_specta::Event;
use tauri::{AppHandle, Manager, State, WebviewUrl, WebviewWindow, WebviewWindowBuilder, WindowEvent};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
use windows::Win32::Graphics::Dwm::{
    DWMWA_BORDER_COLOR, DWMWA_COLOR_DEFAULT, DWMWA_COLOR_NONE, DWMWA_TRANSITIONS_FORCEDISABLED, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_DONOTROUND,
    DWMWCP_ROUND, DwmSetWindowAttribute,
};
use windows::Win32::Foundation::{HWND, POINT, RECT};
use windows::Win32::Graphics::Gdi::{
    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, CreateCompatibleBitmap, CreateCompatibleDC, DIB_RGB_COLORS, DeleteDC,
    DeleteObject, GetDC, GetDIBits, HALFTONE, ReleaseDC, SRCCOPY, SelectObject, SetStretchBltMode, StretchBlt,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetCursorPos, GetForegroundWindow, GetWindowRect, IsIconic, IsWindowVisible, SPI_GETCLIENTAREAANIMATION, SW_HIDE, SW_SHOWNOACTIVATE, SWP_NOACTIVATE, SWP_NOZORDER,
    SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, SetWindowPos, ShowWindow, SystemParametersInfoW,
};
use windows::core::BOOL;

use super::webview;
use crate::module::Modules;
use crate::settings::{EdgeSide, Look, Settings, SettingsState};

const LABEL: &str = "panel";
/// A tray click right after the panel lost focus is the same click that closed it, so ignore it.
const REOPEN_GUARD: Duration = Duration::from_millis(250);
/// How long after a frame change the focus loss Windows reports is ignored.
const REFRAME_GUARD: Duration = Duration::from_millis(400);
/// Panel size in logical pixels. The height is fixed, so nothing moves when switching pages; what
/// doesn't fit scrolls.
const WIDTH: f64 = 360.0;
const HEIGHT: f64 = 648.0;

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
    /// When the frame was last changed: Windows reports the focus lost a moment later (after the change
    /// has returned), which isn't a click elsewhere.
    reframed_at: Mutex<Option<Instant>>,
    /// Counts open/close animations: an animation stops when a newer one has started.
    anim: AtomicU32,
    /// While the page waits to come in: the UI says (`panel_ready`) once its tiles have their data.
    ready: Mutex<Option<std::sync::mpsc::Sender<()>>>,
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
    // Windows fades a window in and out when it's shown or hidden: that fade would play on the empty
    // frame around the tiny bar (it lingered for about half a second after closing). The panel
    // animates itself.
    if let Ok(hwnd) = win.hwnd() {
        let off = BOOL(1);
        let _ = unsafe { DwmSetWindowAttribute(hwnd, DWMWA_TRANSITIONS_FORCEDISABLED, &off as *const _ as *const c_void, size_of_val(&off) as u32) };
    }
    webview::set_active(&win, false);

    let handle = win.clone();
    win.on_window_event(move |event| match event {
        // Clicking anywhere else closes the panel, like the Windows volume and network flyouts.
        WindowEvent::Focused(false) if is_open(&handle) => {
            if !just_reframed(&handle) {
                hide(&handle);
            }
        }
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
    if just_reframed(&win) {
        return false;
    }
    let mut pointer = POINT::default();
    let mut r = RECT::default();
    if let Ok(hwnd) = win.hwnd()
        && unsafe { GetCursorPos(&mut pointer).is_ok() && GetWindowRect(hwnd, &mut r).is_ok() }
        && beside(&r, pointer, app.state::<SettingsState>().get().edge_side)
    {
        return false;
    }
    hide(&win);
    true
}

/// Over the panel or between it and the screen's edge it opened from, level with it.
fn beside(panel: &RECT, p: POINT, side: EdgeSide) -> bool {
    let across = if side == EdgeSide::Left { p.x <= panel.right } else { p.x >= panel.left };
    across && p.y >= panel.top && p.y < panel.bottom
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
        // (SW_SHOWNOACTIVATE also un-minimises.)
        if let Ok(hwnd) = win.hwnd() {
            let _ = unsafe { ShowWindow(hwnd, SW_SHOWNOACTIVATE) };
        }
    } else {
        if win.is_minimized().unwrap_or(false) {
            let _ = win.unminimize();
        }
        let _ = win.show();
        let _ = win.set_focus();
    }
    // The page's tiles come in one after another (shell/island.css). Until now the page has been
    // waiting with nothing showing; once they're all in, it goes back to normal.
    let id = win.state::<PanelState>().anim.fetch_add(1, Ordering::Relaxed) + 1;
    if animations_on() {
        // The page keeps waiting ("pre") until its tiles have their data, so it never comes in half
        // built, but not longer than HOLD_MS (a tile that never answers mustn't keep it blank).
        let (tx, rx) = std::sync::mpsc::channel();
        *win.state::<PanelState>().ready.lock().unwrap() = Some(tx);
        let w = win.clone();
        std::thread::spawn(move || {
            let _ = rx.recv_timeout(HOLD_MS);
            if w.state::<PanelState>().anim.load(Ordering::Relaxed) != id {
                return;
            }
            page_state(&w, Some("in"));
            std::thread::sleep(IN_MS);
            if w.state::<PanelState>().anim.load(Ordering::Relaxed) == id {
                page_state(&w, None);
            }
        });
    } else {
        page_state(win, None);
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
    // The tiles blur away first (shell/island.css).
    if is_open(win) && animations_on() {
        if state.closing.swap(true, Ordering::Relaxed) {
            return; // already closing
        }
        *state.last_hidden.lock().unwrap() = Some(Instant::now());
        let id = state.anim.fetch_add(1, Ordering::Relaxed) + 1;
        page_state(win, Some("out"));
        let w = win.clone();
        std::thread::spawn(move || {
            std::thread::sleep(OUT_MS);
            if w.state::<PanelState>().anim.load(Ordering::Relaxed) == id {
                let w2 = w.clone();
                let _ = w.run_on_main_thread(move || hide_now(&w2));
            }
        });
        return;
    }
    hide_now(win);
}

fn hide_now(win: &WebviewWindow) {
    win.state::<PanelState>().closing.store(false, Ordering::Relaxed);
    win.state::<PanelState>().anim.fetch_add(1, Ordering::Relaxed);
    // Both: Tauri's hide does nothing when it didn't do the showing (the edge uses ShowWindow).
    let _ = win.hide();
    if let Ok(hwnd) = win.hwnd() {
        let _ = unsafe { ShowWindow(hwnd, SW_HIDE) };
    }
    // Ready for the next open: the page waits with nothing showing ("pre"), because a message sent as
    // the panel shows can reach the page a few frames late, and the tiles would flash first.
    page_state(win, animations_on().then_some("pre"));
    webview::set_active(win, false);
    win.state::<Modules>().panel_visible(false);
    *win.state::<PanelState>().last_hidden.lock().unwrap() = Some(Instant::now());
    crate::updater::check(win.app_handle());
}

/// How long the tiles take to come in (the last one starts about 0.5 s in), and to blur away.
/// Shell/island.css has the same numbers.
const IN_MS: Duration = Duration::from_millis(1000);
const OUT_MS: Duration = Duration::from_millis(390);
/// The longest the page waits for its tiles' data before coming in anyway.
const HOLD_MS: Duration = Duration::from_millis(400);

/// Sets which part of the open/close animation the page is in (`html[data-island]`: "pre", "in" or
/// "out"), or none.
fn page_state(win: &WebviewWindow, state: Option<&str>) {
    let _ = match state {
        Some(s) => win.eval(format!("document.documentElement.dataset.island='{s}'")),
        None => win.eval("delete document.documentElement.dataset.island"),
    };
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

/// Called by the UI when every tile on the page has its data, so the tiles can come in.
#[tauri::command]
#[specta::specta]
pub fn panel_ready(state: State<PanelState>) {
    if let Some(tx) = state.ready.lock().unwrap().take() {
        let _ = tx.send(());
    }
}

/// The mouse has landed on the screen's edge (`shell::edge`) and the panel may open in a moment: wake the
/// modules and the page now, so their data is there by the time it does. `on: false` undoes it if the
/// mouse left without the panel opening.
pub fn warm_up(app: &AppHandle, on: bool) {
    let app2 = app.clone();
    let _ = app.run_on_main_thread(move || {
        if let Some(win) = app2.get_webview_window(LABEL)
            && !is_open(&win)
        {
            webview::set_active(&win, on);
            win.state::<Modules>().panel_visible(on);
        }
    });
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
        // An edge-opened panel sits at the edge it opened from.
        let x = if edge && win.state::<SettingsState>().get().edge_side == EdgeSide::Left {
            area.position.x + margin
        } else {
            area.position.x + area.size.width as i32 - width - margin
        };
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
        let state = app.state::<PanelState>();
        // Changing the frame makes Windows drop the focus, which would hide the panel: the focus-loss
        // handler ignores it for a moment (`just_reframed`), then we take the focus back (an
        // edge-opened panel only had it if the user had clicked in; without it, clicking elsewhere
        // wouldn't close the panel any more).
        *state.reframed_at.lock().unwrap() = Some(Instant::now());
        let was_open = is_open(&win);
        let had_focus = win.is_focused().unwrap_or(false) || win.hwnd().is_ok_and(|h| unsafe { GetForegroundWindow() } == h);
        frame(&win, framed(&app.state::<SettingsState>().get()));
        place(&win, state.hover.load(Ordering::Relaxed));
        // Tauri's frame change hides a window it didn't show itself (the edge shows it with
        // ShowWindow, so Tauri thinks it's hidden): show it again, still without taking the keyboard.
        if was_open && !is_open(&win) && let Ok(hwnd) = win.hwnd() {
            let _ = unsafe { ShowWindow(hwnd, SW_SHOWNOACTIVATE) };
        }
        if is_open(&win) && (had_focus || !state.hover.load(Ordering::Relaxed)) {
            let _ = win.set_focus();
            // The page inside needs it too, or clicking elsewhere may not register as leaving.
            let _ = tauri::Webview::set_focus(win.as_ref());
        }
        *state.reframed_at.lock().unwrap() = Some(Instant::now());
    }
}

fn just_reframed(win: &WebviewWindow) -> bool {
    win.state::<PanelState>().reframed_at.lock().unwrap().is_some_and(|t| t.elapsed() < REFRAME_GUARD)
}

/// Whether the window gets Windows 11's frame: every look except transparent Dot, whose cards
/// float over the desktop, where a frame would outline empty space. (Only Dot can be transparent;
/// an old Tint file may still say so.)
fn framed(settings: &Settings) -> bool {
    settings.look != Look::Dot || !settings.transparent
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
        use crate::settings::EdgeSide;
        let at = |x, y| super::beside(&panel, POINT { x, y }, EdgeSide::Right);
        assert!(at(1919, 600), "on the screen's edge, level with it");
        assert!(at(1600, 600), "over it");
        assert!(!at(1400, 600), "to its left");
        assert!(!at(1919, 100), "on the edge, above it");
        let at_left = |x, y| super::beside(&panel, POINT { x, y }, EdgeSide::Left);
        assert!(at_left(0, 600), "on the left edge, level with it");
        assert!(at_left(1600, 600), "over it");
        assert!(!at_left(1919, 600), "to its right"); 
    }

    #[test]
    fn brightness() {
        assert_eq!(super::brightness(&[0, 0, 0, 255].repeat(64)), 0);
        assert_eq!(super::brightness(&[255, 255, 255, 255].repeat(64)), 100);
        assert_eq!(super::brightness(&[0, 255, 0, 255, 0, 0, 0, 255].repeat(32)), 36); // half bright green
    }
}

