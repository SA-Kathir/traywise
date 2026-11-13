//! Opening the panel from the screen's left or right edge (Settings › Open from the screen edge).
//!
//! A tiny native window covers the middle third of the chosen edge of each monitor that has nothing
//! beside it on that side: an invisible 1 px strip, or a small pill when the handle is on. Windows tells it when the mouse rests on it,
//! so nothing watches the mouse anywhere else and it costs nothing while idle. It runs on its own
//! thread, which sleeps in Windows' message loop between those messages.

use std::cell::{Cell, RefCell};
use std::sync::mpsc::{Sender, channel};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use std::thread::JoinHandle;
use std::time::Instant;

use tauri::{AppHandle, Manager};
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    CreateRoundRectRgn, CreateSolidBrush, EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITOR_DEFAULTTONEAREST, MONITORINFO,
    MonitorFromWindow, SetWindowRgn,
};
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, TME_HOVER, TME_LEAVE, TRACKMOUSEEVENT, TrackMouseEvent, VK_LBUTTON, VK_RBUTTON};
use windows::Win32::UI::Shell::{QUNS_BUSY, QUNS_PRESENTATION_MODE, QUNS_RUNNING_D3D_FULL_SCREEN, SHQueryUserNotificationState};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetClassNameW, GetForegroundWindow, GetMessageW, IDC_ARROW, KillTimer, SetTimer, WM_TIMER, LWA_ALPHA, LoadCursorW, MSG,
    PostThreadMessageW, RegisterClassW, SW_SHOWNOACTIVATE, SWP_NOACTIVATE, SWP_NOZORDER, SetLayeredWindowAttributes,
    SetWindowPos, ShowWindow, TranslateMessage, UnregisterClassW, WM_APP, WM_DISPLAYCHANGE, WM_QUIT, WM_MOUSEMOVE, WNDCLASSW,
    WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TRANSPARENT, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};
use windows::core::{BOOL, w};

use crate::settings::{Accent, EdgeSide, Look, Preset, Settings};

/// Windows' "the mouse rested here" message (defined in its UI-controls headers, a feature not worth
/// pulling in for one number).
const WM_MOUSEHOVER: u32 = 0x02A1;
/// How long the mouse rests on the edge before the panel opens: passing by doesn't count.
const HOVER_MS: u32 = 250;
/// The handle: a pill this many physical pixels wide and tall.
const HANDLE: (i32, i32) = (8, 80);
/// While the mouse rests on the handle it swells to this size and brightens, so you can see the
/// panel is about to open (like a Dynamic Island stretching before it opens).
const SWOLLEN: (i32, i32) = (14, 104);
/// How long the swell takes: longer than the hover, so the handle is still moving while the panel appears and
/// hands over to it without a pause.
const SWELL_MS: u32 = HOVER_MS + 250;
const SWOLLEN_ALPHA: u8 = 255;
const HANDLE_ALPHA: u8 = 140;
/// Windows' "the mouse left" message, and the animation's timer (about 60 frames a second).
const WM_MOUSELEAVE: u32 = 0x02A3;
const ANIMATION_TIMER: usize = 1;
/// Warming the panel up starts this long after the mouse lands (so passing by costs nothing), and its timer.
const WARM_MS: u32 = 120;
const WARM_TIMER: usize = 2;
const FRAME_MS: u32 = 16;

/// Posted to the edge thread when the monitors changed: the strips are made again.
const WM_REBUILD: u32 = WM_APP;

thread_local! {
    /// When the mouse arrived on the handle, while it swells.
    static ARRIVED: Cell<Option<Instant>> = const { Cell::new(None) };
    /// The window you see (the handle). The edge window itself never changes size, so the pointer's target
    /// holds still while the handle swells (resizing the target under the pointer made the handle flicker).
    /// The panel has been warmed up for this visit (and must be put back if the mouse leaves).
    static WARMED: Cell<bool> = const { Cell::new(false) };
    /// Each strip's window and, when the handle is on, the handle window drawn over it.
    static STRIPS: RefCell<Vec<(isize, isize)>> = const { RefCell::new(Vec::new()) };
    /// A rebuild has been asked for and not yet done (every strip hears of a display change).
    static REBUILD: Cell<bool> = const { Cell::new(false) };
}

/// The running edge window, and which settings it was made for.
#[derive(Default)]
pub struct EdgeState {
    running: Mutex<Option<Running>>,
}

struct Running {
    /// The edge thread's id, to tell it to stop.
    thread_id: u32,
    thread: JoinHandle<()>,
    /// What the window was made for: with a handle, which side, and the handle's colour (Windows' BGR).
    key: Key,
}

type Key = (bool, EdgeSide, u32);

static APP: OnceLock<AppHandle> = OnceLock::new();
static SHOW_HANDLE: AtomicBool = AtomicBool::new(false);
static LEFT: AtomicBool = AtomicBool::new(false);

/// Starts, changes or stops the edge window to match the settings (after each save, and at startup).
pub fn apply(app: &AppHandle, settings: &Settings) {
    let _ = APP.set(app.clone());
    let state = app.try_state::<EdgeState>().unwrap_or_else(|| {
        app.manage(EdgeState::default());
        app.state::<EdgeState>()
    });
    let mut running = state.running.lock().unwrap();
    let wanted: Option<Key> = settings.edge_open.then(|| (settings.edge_handle, settings.edge_side, handle_colour(settings)));
    if running.as_ref().map(|r| r.key) == wanted {
        return; // unchanged, like most saves (a tile moved on the board)
    }
    if let Some(r) = running.take() {
        unsafe { PostThreadMessageW(r.thread_id, WM_QUIT, WPARAM(0), LPARAM(0)).ok() };
        let _ = r.thread.join();
    }
    if let Some(key @ (handle, side, colour)) = wanted {
        SHOW_HANDLE.store(handle, Ordering::Relaxed);
        LEFT.store(side == EdgeSide::Left, Ordering::Relaxed);
        let (tx, rx) = channel();
        if let Ok(thread) = std::thread::Builder::new().name("edge".into()).spawn(move || run(handle, side, colour, tx))
            && let Ok(thread_id) = rx.recv()
        {
            *running = Some(Running { thread_id, thread, key });
        }
    }
}

fn run(handle: bool, side: EdgeSide, colour: u32, tx: Sender<u32>) {
    let instance = unsafe { GetModuleHandleW(None).ok() };
    let class = w!("TraywiseEdge");
    let wc = WNDCLASSW {
        lpfnWndProc: Some(window_proc),
        hInstance: instance.map(Into::into).unwrap_or_default(),
        lpszClassName: class,
        hCursor: unsafe { LoadCursorW(None, IDC_ARROW) }.unwrap_or_default(),
        // The handle's colour (grey, or the accent); the strip is invisible anyway.
        hbrBackground: unsafe { CreateSolidBrush(COLORREF(colour)) },
        ..Default::default()
    };
    unsafe { RegisterClassW(&wc) }; // (the class is removed when the thread ends, so a new start gets its own colour)
    let _ = tx.send(unsafe { GetCurrentThreadId() });
    let mut msg = MSG::default();
    'strips: loop {
        REBUILD.set(false);
        let monitors = monitors();
        let strips: Vec<(isize, isize)> = (0..monitors.len())
            .filter(|&i| outer_edge(&monitors, i, side))
            .filter_map(|i| make_strip(instance, class, monitors[i], handle, side))
            .collect();
        STRIPS.with_borrow_mut(|s| *s = strips.clone());
        // Sleeps in Windows' message loop; a display change posts WM_REBUILD, and WM_QUIT ends it.
        loop {
            if !unsafe { GetMessageW(&mut msg, None, 0, 0) }.as_bool() {
                break;
            }
            if msg.hwnd.0.is_null() && msg.message == WM_REBUILD {
                for (hit, visible) in strips {
                    unsafe {
                        let _ = DestroyWindow(HWND(hit as _));
                        if visible != 0 {
                            let _ = DestroyWindow(HWND(visible as _));
                        }
                    }
                }
                continue 'strips;
            }
            unsafe {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
        for (hit, visible) in strips {
            unsafe {
                let _ = DestroyWindow(HWND(hit as _));
                if visible != 0 {
                    let _ = DestroyWindow(HWND(visible as _));
                }
            }
        }
        break;
    }
    STRIPS.with_borrow_mut(Vec::clear);
    // The class holds the handle's colour, so it goes with the strips: the next start registers its own.
    let _ = unsafe { UnregisterClassW(w!("TraywiseEdge"), instance.map(Into::into)) };
}

/// One edge strip on `monitor` (and its handle), as (strip window, handle window or 0).
fn make_strip(instance: Option<windows::Win32::Foundation::HMODULE>, class: windows::core::PCWSTR, monitor: RECT, handle: bool, side: EdgeSide) -> Option<(isize, isize)> {
    let create = |extra, r: RECT| unsafe {
        CreateWindowExW(
            WS_EX_TOOLWINDOW | WS_EX_TOPMOST | WS_EX_NOACTIVATE | WS_EX_LAYERED | extra,
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
    let hit = match create(Default::default(), rect(monitor, handle, side)) {
        Ok(hit) => hit,
        Err(e) => {
            eprintln!("edge: couldn't create an edge window: {e}");
            return None;
        }
    };
    style(hit);
    let visible = if handle
        && let Ok(visible) = create(WS_EX_TRANSPARENT, rect(monitor, handle, side))
    {
        // Click-through: the mouse messages go to the window under it.
        let strip = (hit.0 as isize, visible.0 as isize);
        pose(strip.0, 0.0);
        let _ = unsafe { ShowWindow(visible, SW_SHOWNOACTIVATE) };
        strip.1
    } else {
        0
    };
    Some((hit.0 as isize, visible))
}

/// Nearly invisible (fully invisible windows get no mouse messages). With a handle, it's the handle's
/// largest size, so the target is steady.
fn style(hwnd: HWND) {
    unsafe {
        let _ = SetLayeredWindowAttributes(hwnd, COLORREF(0), 1, LWA_ALPHA);
        if SHOW_HANDLE.load(Ordering::Relaxed) {
            let (w, h) = SWOLLEN;
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
                // The handle starts swelling as soon as the mouse arrives (the strip has nothing to show).
                if ARRIVED.get().is_none() {
                    ARRIVED.set(Some(Instant::now()));
                    if SHOW_HANDLE.load(Ordering::Relaxed) {
                        SetTimer(Some(hwnd), ANIMATION_TIMER, FRAME_MS, None);
                    }
                    SetTimer(Some(hwnd), WARM_TIMER, WARM_MS, None);
                }
                let mut track = TRACKMOUSEEVENT {
                    cbSize: size_of::<TRACKMOUSEEVENT>() as u32,
                    dwFlags: TME_HOVER | TME_LEAVE,
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
            WM_TIMER if wparam.0 == WARM_TIMER => {
                // Resting here: the panel is likely to open, so start getting its data ready.
                let _ = KillTimer(Some(hwnd), WARM_TIMER);
                if let Some(app) = APP.get() {
                    WARMED.set(true);
                    super::panel::warm_up(app, true);
                }
                LRESULT(0)
            }
            WM_TIMER => {
                let t = ARRIVED.get().map_or(0.0, |at| (at.elapsed().as_millis() as f32 / SWELL_MS as f32).min(1.0));
                pose(hwnd.0 as isize, t);
                if t >= 1.0 {
                    let _ = KillTimer(Some(hwnd), ANIMATION_TIMER); // it stays swollen until the mouse leaves
                }
                LRESULT(0)
            }
            // The mouse left: back to the resting handle.
            WM_MOUSELEAVE => {
                let _ = KillTimer(Some(hwnd), ANIMATION_TIMER);
                // (Does nothing if the panel did open.)
                let _ = KillTimer(Some(hwnd), WARM_TIMER);
                ARRIVED.set(None);
                if WARMED.replace(false) && let Some(app) = APP.get() {
                    super::panel::warm_up(app, false);
                }
                pose(hwnd.0 as isize, 0.0);
                LRESULT(0)
            }
            // A monitor added, removed or rescaled: the strips are made again for the new layout (once,
            // though every strip hears of it).
            WM_DISPLAYCHANGE => {
                if !REBUILD.replace(true) {
                    let _ = PostThreadMessageW(GetCurrentThreadId(), WM_REBUILD, WPARAM(0), LPARAM(0));
                }
                LRESULT(0)
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }
}

/// The handle of the strip `hit` `t` of the way (0 to 1) from resting to swollen: its size, shape and brightness.
fn pose(hit: isize, t: f32) {
    let Some(visible) = STRIPS.with_borrow(|s| s.iter().find(|(h, _)| *h == hit).map(|&(_, v)| v)).filter(|&v| v != 0) else { return };
    let hwnd = HWND(visible as _);
    let side = if LEFT.load(Ordering::Relaxed) { EdgeSide::Left } else { EdgeSide::Right };
    let (r, alpha) = swell(monitor_of(HWND(hit as _)), side, t);
    let (w, h) = (r.right - r.left, r.bottom - r.top);
    unsafe {
        let _ = SetWindowPos(hwnd, None, r.left, r.top, w, h, SWP_NOZORDER | SWP_NOACTIVATE);
        SetWindowRgn(hwnd, Some(CreateRoundRectRgn(0, 0, w + 1, h + 1, w, w)), true);
        let _ = SetLayeredWindowAttributes(hwnd, COLORREF(0), alpha, LWA_ALPHA);
    }
}

/// The handle's rectangle and opacity at `t`, easing in: slow at first, gathering speed, so it's at full
/// stretch only as the panel arrives (an ease-out looked finished halfway through).
fn swell(monitor: RECT, side: EdgeSide, t: f32) -> (RECT, u8) {
    let e = t * t;
    let mix = |a: i32, b: i32| a + ((b - a) as f32 * e).round() as i32;
    let (w, h) = (mix(HANDLE.0, SWOLLEN.0), mix(HANDLE.1, SWOLLEN.1));
    let top = monitor.top + (monitor.bottom - monitor.top - h) / 2;
    let left = if side == EdgeSide::Left { monitor.left } else { monitor.right - w };
    (RECT { left, top, right: left + w, bottom: top + h }, mix(HANDLE_ALPHA as i32, SWOLLEN_ALPHA as i32) as u8)
}

/// Dragging (a scrollbar, say) along the edge shouldn't open the panel.
fn button_held() -> bool {
    unsafe { GetAsyncKeyState(VK_LBUTTON.0.into()) < 0 || GetAsyncKeyState(VK_RBUTTON.0.into()) < 0 }
}

/// A fullscreen game or video, or presentation mode: stay out of the way. (Not the desktop itself:
/// with nothing else open, Windows answers "busy" because the desktop covers the whole screen.)
fn fullscreen() -> bool {
    !desktop_in_front()
        && unsafe { SHQueryUserNotificationState() }
            .is_ok_and(|s| s == QUNS_BUSY || s == QUNS_RUNNING_D3D_FULL_SCREEN || s == QUNS_PRESENTATION_MODE)
}

/// The desktop is the window in front (nothing else is open, or it was just clicked).
fn desktop_in_front() -> bool {
    let mut class = [0u16; 32];
    let len = unsafe { GetClassNameW(GetForegroundWindow(), &mut class) } as usize;
    matches!(String::from_utf16_lossy(&class[..len]).as_str(), "Progman" | "WorkerW")
}

/// The whole area of every monitor (the edge, not the work area: the taskbar may be on that side).
fn monitors() -> Vec<RECT> {
    unsafe extern "system" fn add(_: HMONITOR, _: HDC, area: *mut RECT, list: LPARAM) -> BOOL {
        unsafe { (*(list.0 as *mut Vec<RECT>)).push(*area) };
        true.into()
    }
    let mut list: Vec<RECT> = Vec::new();
    let _ = unsafe { EnumDisplayMonitors(None, None, Some(add), LPARAM(&mut list as *mut Vec<RECT> as isize)) };
    list
}

/// The whole area of the monitor a strip is on.
fn monitor_of(strip: HWND) -> RECT {
    let mut info = MONITORINFO { cbSize: size_of::<MONITORINFO>() as u32, ..Default::default() };
    unsafe {
        let _ = GetMonitorInfoW(MonitorFromWindow(strip, MONITOR_DEFAULTTONEAREST), &mut info);
    }
    info.rcMonitor
}

/// Whether monitor `i` has no other monitor beside it on `side`, level with its middle third: only
/// those edges get a strip, so the mouse crossing from one monitor to the next doesn't open the panel.
fn outer_edge(monitors: &[RECT], i: usize, side: EdgeSide) -> bool {
    let m = monitors[i];
    let third = (m.bottom - m.top) / 3;
    let (top, bottom) = (m.top + third, m.bottom - third);
    let edge = if side == EdgeSide::Left { m.left } else { m.right };
    !monitors.iter().enumerate().any(|(j, o)| {
        let beside = if side == EdgeSide::Left { o.right >= edge && o.left < edge } else { o.left <= edge && o.right > edge };
        j != i && beside && o.top < bottom && o.bottom > top
    })
}

/// The edge window: the middle third of the chosen edge, 1 px wide; or the handle, centred on it.
fn rect(monitor: RECT, handle: bool, side: EdgeSide) -> RECT {
    let height = monitor.bottom - monitor.top;
    let (w, h) = if handle { SWOLLEN } else { (1, height / 3) };
    let top = monitor.top + (height - h) / 2;
    let left = if side == EdgeSide::Left { monitor.left } else { monitor.right - w };
    RECT { left, top, right: left + w, bottom: top + h }
}

/// The accent of the current look as 0xRRGGBB: Dot's accent presets, Tint's preset accents and Windows'
/// blue; the same colours as in the CSS.
pub fn accent_rgb(settings: &Settings) -> u32 {
    match settings.look {
        Look::Dot => match settings.accent {
            Accent::Red => 0xd9_2b_24,
            Accent::Amber => 0xf2_a3_3a,
            Accent::Violet => 0xa9_8b_ff,
            Accent::Blue => 0x5a_a9_ff,
            Accent::Green => 0x34_c7_7b,
        },
        Look::Liquid => match settings.preset {
            Preset::Ocean => 0xa9_c8_ee,
            Preset::Graphite => 0xd7_dc_e4,
            Preset::Forest => 0xa8_d8_bd,
            Preset::Plum => 0xd4_bd_e8,
            Preset::Ember => 0xf0_c3_a6,
            Preset::Rose => 0xf0_bf_d4,
        },
        Look::Windows => 0x4c_c2_ff,
    }
}

/// The handle's colour as Windows wants it (0x00BBGGRR): the accent, or grey.
fn handle_colour(settings: &Settings) -> u32 {
    let rgb = if settings.edge_handle_accent { accent_rgb(settings) } else { 0x90_90_90 };
    let (r, g, b) = (rgb >> 16, (rgb >> 8) & 0xff, rgb & 0xff);
    (b << 16) | (g << 8) | r
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_only_on_outer_edges() {
        let r = |l, t, rt, b| RECT { left: l, top: t, right: rt, bottom: b };
        // A laptop screen with a monitor to its right.
        let two = [r(0, 0, 1920, 1080), r(1920, 0, 4480, 1440)];
        assert!(outer_edge(&two, 0, EdgeSide::Left), "the laptop's left edge");
        assert!(!outer_edge(&two, 0, EdgeSide::Right), "the laptop's right edge touches the monitor");
        assert!(!outer_edge(&two, 1, EdgeSide::Left));
        assert!(outer_edge(&two, 1, EdgeSide::Right));
        // One screen: both edges are outer.
        assert!(outer_edge(&two[..1], 0, EdgeSide::Right));
        // A monitor that only reaches the top corner doesn't cover the middle third of the other.
        let corner = [r(0, 0, 1920, 1080), r(1920, -900, 3840, 100)];
        assert!(outer_edge(&corner, 0, EdgeSide::Right));
        // A monitor above, not beside: still outer.
        let above = [r(0, 0, 1920, 1080), r(0, -1080, 1920, 0)];
        assert!(outer_edge(&above, 0, EdgeSide::Right) && outer_edge(&above, 0, EdgeSide::Left));
    }

    #[test]
    fn edge_rect() {
        let screen = RECT { left: 0, top: 0, right: 2560, bottom: 1440 };
        let strip = rect(screen, false, EdgeSide::Right);
        assert_eq!((strip.left, strip.top, strip.right, strip.bottom), (2559, 480, 2560, 960));
        let left = rect(screen, false, EdgeSide::Left);
        assert_eq!((left.left, left.top, left.right, left.bottom), (0, 480, 1, 960));
        let pill = rect(screen, true, EdgeSide::Right);
        assert_eq!((pill.left, pill.top, pill.right, pill.bottom), (2546, 668, 2560, 772));
        // Swelling starts from the resting handle, ends bigger, and stays on the edge, centred.
        let (start, a0) = swell(screen, EdgeSide::Right, 0.0);
        assert_eq!((start.left, start.top, start.right, start.bottom, a0), (2552, 680, 2560, 760, HANDLE_ALPHA));
        let (end, a1) = swell(screen, EdgeSide::Right, 1.0);
        assert_eq!((end.left, end.top, end.right, end.bottom, a1), (2546, 668, 2560, 772, SWOLLEN_ALPHA));
    }
}
