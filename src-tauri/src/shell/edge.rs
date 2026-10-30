//! Opening the panel from the screen's right edge (Settings › Open from the right edge).
//!
//! A tiny native window covers the middle third of the primary monitor's right edge: an invisible
//! 1 px strip, or a small pill when the handle is on. Windows tells it when the mouse rests on it,
//! so nothing watches the mouse anywhere else and it costs nothing while idle. It runs on its own
//! thread, which sleeps in Windows' message loop between those messages.

use std::sync::mpsc::{Sender, channel};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use std::thread::JoinHandle;

use tauri::{AppHandle, Manager};
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    CreateRoundRectRgn, CreateSolidBrush, GetMonitorInfoW, MONITOR_DEFAULTTOPRIMARY, MONITORINFO, MonitorFromPoint,
    SetWindowRgn,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, TME_HOVER, TRACKMOUSEEVENT, TrackMouseEvent, VK_LBUTTON, VK_RBUTTON};
use windows::Win32::UI::Shell::{QUNS_BUSY, QUNS_PRESENTATION_MODE, QUNS_RUNNING_D3D_FULL_SCREEN, SHQueryUserNotificationState};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW, IDC_ARROW, LWA_ALPHA, LoadCursorW, MSG,
    PostMessageW, PostQuitMessage, RegisterClassW, SW_SHOWNOACTIVATE, SWP_NOACTIVATE, SWP_NOZORDER, SetLayeredWindowAttributes,
    SetWindowPos, ShowWindow, TranslateMessage, WM_CLOSE, WM_DESTROY, WM_DISPLAYCHANGE, WM_MOUSEMOVE, WNDCLASSW,
    WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};
use windows::core::w;

use crate::settings::Settings;

/// Windows' "the mouse rested here" message (defined in its UI-controls headers, a feature not worth
/// pulling in for one number).
const WM_MOUSEHOVER: u32 = 0x02A1;
/// How long the mouse rests on the edge before the panel opens: passing by doesn't count.
const HOVER_MS: u32 = 250;
/// The handle: a pill this many physical pixels wide and tall.
const HANDLE: (i32, i32) = (8, 80);

/// The running edge window, and which settings it was made for.
#[derive(Default)]
pub struct EdgeState {
    running: Mutex<Option<Running>>,
}

struct Running {
    hwnd: isize,
    thread: JoinHandle<()>,
    handle: bool,
}

static APP: OnceLock<AppHandle> = OnceLock::new();
static SHOW_HANDLE: AtomicBool = AtomicBool::new(false);

/// Starts, changes or stops the edge window to match the settings (after each save, and at startup).
pub fn apply(app: &AppHandle, settings: &Settings) {
    let _ = APP.set(app.clone());
    let state = app.try_state::<EdgeState>().unwrap_or_else(|| {
        app.manage(EdgeState::default());
        app.state::<EdgeState>()
    });
    let mut running = state.running.lock().unwrap();
    let wanted = settings.edge_open.then_some(settings.edge_handle);
    if running.as_ref().map(|r| r.handle) == wanted {
        return; // unchanged, like most saves (a tile moved on the board)
    }
    if let Some(r) = running.take() {
        unsafe { PostMessageW(Some(HWND(r.hwnd as _)), WM_CLOSE, WPARAM(0), LPARAM(0)).ok() };
        let _ = r.thread.join();
    }
    if let Some(handle) = wanted {
        SHOW_HANDLE.store(handle, Ordering::Relaxed);
        let (tx, rx) = channel();
        if let Ok(thread) = std::thread::Builder::new().name("edge".into()).spawn(move || run(handle, tx))
            && let Ok(hwnd) = rx.recv()
        {
            *running = Some(Running { hwnd, thread, handle });
        }
    }
}

fn run(handle: bool, tx: Sender<isize>) {
    let made = unsafe {
        let instance = GetModuleHandleW(None).ok();
        let class = w!("TraywiseEdge");
        let wc = WNDCLASSW {
            lpfnWndProc: Some(window_proc),
            hInstance: instance.map(Into::into).unwrap_or_default(),
            lpszClassName: class,
            hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
            // The handle's grey; the strip is invisible anyway.
            hbrBackground: CreateSolidBrush(COLORREF(0x0090_9090)),
            ..Default::default()
        };
        RegisterClassW(&wc); // fails harmlessly when already registered by an earlier start
        let r = rect(monitor(), handle);
        CreateWindowExW(
            WS_EX_TOOLWINDOW | WS_EX_TOPMOST | WS_EX_NOACTIVATE | WS_EX_LAYERED,
            class,
            w!(""),
            WS_POPUP,
            r.left,
            r.top,
            r.right - r.left,
            r.bottom - r.top,
            None,
            None,
            instance.map(Into::into),
            None,
        )
    };
    let Ok(hwnd) = made else {
        eprintln!("edge: couldn't create the edge window");
        return;
    };
    style(hwnd, handle);
    let _ = tx.send(hwnd.0 as isize);
    let mut msg = MSG::default();
    while unsafe { GetMessageW(&mut msg, None, 0, 0) }.as_bool() {
        unsafe {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}

/// Nearly invisible (fully invisible windows get no mouse messages), or a translucent pill.
fn style(hwnd: HWND, handle: bool) {
    unsafe {
        let _ = SetLayeredWindowAttributes(hwnd, COLORREF(0), if handle { 140 } else { 1 }, LWA_ALPHA);
        if handle {
            let (w, h) = HANDLE;
            SetWindowRgn(hwnd, Some(CreateRoundRectRgn(0, 0, w + 1, h + 1, w, w)), true);
        }
        let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
    }
}

extern "system" fn window_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe {
        match msg {
            // Ask Windows to say when the mouse has rested here for HOVER_MS (once per arrival).
            WM_MOUSEMOVE => {
                let mut track = TRACKMOUSEEVENT {
                    cbSize: size_of::<TRACKMOUSEEVENT>() as u32,
                    dwFlags: TME_HOVER,
                    hwndTrack: hwnd,
                    dwHoverTime: HOVER_MS,
                };
                let _ = TrackMouseEvent(&mut track);
                LRESULT(0)
            }
            WM_MOUSEHOVER => {
                if !button_held() && !fullscreen() && let Some(app) = APP.get() {
                    let app2 = app.clone();
                    let _ = app.run_on_main_thread(move || super::panel::hover_open(&app2));
                }
                LRESULT(0)
            }
            // A monitor added, removed or rescaled: move to the new edge.
            WM_DISPLAYCHANGE => {
                let r = rect(monitor(), SHOW_HANDLE.load(Ordering::Relaxed));
                let _ = SetWindowPos(hwnd, None, r.left, r.top, r.right - r.left, r.bottom - r.top, SWP_NOZORDER | SWP_NOACTIVATE);
                LRESULT(0)
            }
            WM_CLOSE => {
                let _ = DestroyWindow(hwnd);
                LRESULT(0)
            }
            WM_DESTROY => {
                PostQuitMessage(0);
                LRESULT(0)
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }
}

/// Dragging (a scrollbar, say) along the edge shouldn't open the panel.
fn button_held() -> bool {
    unsafe { GetAsyncKeyState(VK_LBUTTON.0.into()) < 0 || GetAsyncKeyState(VK_RBUTTON.0.into()) < 0 }
}

/// A fullscreen game or video, or presentation mode: stay out of the way.
fn fullscreen() -> bool {
    unsafe { SHQueryUserNotificationState() }
        .is_ok_and(|s| s == QUNS_BUSY || s == QUNS_RUNNING_D3D_FULL_SCREEN || s == QUNS_PRESENTATION_MODE)
}

/// The primary monitor's whole area (the edge, not the work area: the taskbar may be on the right).
fn monitor() -> RECT {
    let mut info = MONITORINFO { cbSize: size_of::<MONITORINFO>() as u32, ..Default::default() };
    unsafe {
        let m = MonitorFromPoint(POINT { x: 0, y: 0 }, MONITOR_DEFAULTTOPRIMARY);
        let _ = GetMonitorInfoW(m, &mut info);
    }
    info.rcMonitor
}

/// The edge window: the middle third of the right edge, 1 px wide; or the handle, centred on it.
fn rect(monitor: RECT, handle: bool) -> RECT {
    let height = monitor.bottom - monitor.top;
    let (w, h) = if handle { HANDLE } else { (1, height / 3) };
    let top = monitor.top + (height - h) / 2;
    RECT { left: monitor.right - w, top, right: monitor.right, bottom: top + h }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edge_rect() {
        let screen = RECT { left: 0, top: 0, right: 2560, bottom: 1440 };
        let strip = rect(screen, false);
        assert_eq!((strip.left, strip.top, strip.right, strip.bottom), (2559, 480, 2560, 960));
        let pill = rect(screen, true);
        assert_eq!((pill.left, pill.top, pill.right, pill.bottom), (2552, 680, 2560, 760));
    }
}
