//! The tray icon: left-click toggles the panel, right-click opens a menu (Settings, the microphone mute and
//! Keep awake when those modules run, Quit). While microphones
//! are muted or Keep awake is on, it gets a dot in a corner and a line in its tooltip.

use std::sync::Mutex;

use tauri::image::Image;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager};

use super::panel;
use crate::modules::{audio_devices, keep_awake};
use crate::settings::Settings;
use crate::module::ModuleId;

const ID: &str = "main";
const MUTED: [u8; 3] = [0xe5, 0x48, 0x4d]; // the muted badge's red disc, bottom right
const AWAKE: [u8; 3] = [0xff, 0xff, 0xff]; // the Keep awake cup, top right

/// The tray's own image: the small-size variant of the app icon (no glass plate, bolder glyph),
/// which stays legible at tray size. Made from icons/source/app-icon-small.svg.
fn tray_icon() -> Image<'static> {
    Image::from_bytes(include_bytes!("../../icons/tray.png")).expect("icons/tray.png is a valid PNG")
}

/// What the icon shows besides Traywise itself. Modules set their part, and clear it when they stop.
#[derive(Clone, Default, PartialEq)]
struct Status {
    /// Which modules are switched on, so the menu only offers what works.
    mics_available: bool,
    awake_available: bool,
    /// The icon's colour (0xRRGGBB): the look's accent.
    accent: u32,
    mics_muted: bool,
    /// The tooltip line, like "Keeping awake until 3:40 PM". None when Keep awake is off.
    awake: Option<String>,
}

pub fn create(app: &tauri::App) -> tauri::Result<()> {
    app.manage(Mutex::new(Status::default()));
    TrayIconBuilder::with_id(ID)
        .icon(tray_icon())
        .tooltip("Traywise")
        .menu(&menu(app.handle(), &Status::default())?)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "quit" => app.exit(0),
            "settings" => panel::open_settings(app),
            "mics" => {
                let muted = app.state::<Mutex<Status>>().lock().unwrap().mics_muted;
                let _ = audio_devices::audio_devices_mute_mics(app.state(), !muted);
            }
            "awake" => {
                let on = app.state::<Mutex<Status>>().lock().unwrap().awake.is_some();
                let _ = keep_awake::keep_awake_set_mode(app.state(), if on { keep_awake::Mode::Off } else { keep_awake::Mode::On });
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                panel::toggle(tray.app_handle());
            }
        })
        .build(app)?;
    Ok(())
}

/// The right-click menu for the current state: the mute and Keep awake lines only when their modules run.
fn menu(app: &AppHandle, status: &Status) -> tauri::Result<Menu<tauri::Wry>> {
    let menu = Menu::new(app)?;
    menu.append(&MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?)?;
    if status.mics_available || status.awake_available {
        menu.append(&PredefinedMenuItem::separator(app)?)?;
    }
    if status.mics_available {
        menu.append(&CheckMenuItem::with_id(app, "mics", "Mute microphones", true, status.mics_muted, None::<&str>)?)?;
    }
    if status.awake_available {
        menu.append(&CheckMenuItem::with_id(app, "awake", "Keep awake", true, status.awake.is_some(), None::<&str>)?)?;
    }
    menu.append(&PredefinedMenuItem::separator(app)?)?;
    menu.append(&MenuItem::with_id(app, "quit", "Quit Traywise", true, None::<&str>)?)?;
    Ok(menu)
}

/// Which of the menu's lines apply: after each settings save, and at startup.
pub fn apply_settings(app: &AppHandle, settings: &Settings) {
    update(app, |status| {
        status.mics_available = settings.enabled_modules.contains(&ModuleId::AudioDevices);
        status.awake_available = settings.enabled_modules.contains(&ModuleId::KeepAwake);
        status.accent = super::edge::accent_rgb(settings);
    });
}

pub fn set_mics_muted(app: &AppHandle, muted: bool) {
    update(app, |status| status.mics_muted = muted);
}

pub fn set_keep_awake(app: &AppHandle, line: Option<String>) {
    update(app, |status| status.awake = line);
}

/// Redraws the icon and tooltip, only when something changed. Called from the modules' threads.
fn update(app: &AppHandle, change: impl FnOnce(&mut Status)) {
    let (Some(state), Some(tray)) = (app.try_state::<Mutex<Status>>(), app.tray_by_id(ID)) else { return };
    let mut status = state.lock().unwrap();
    let before = status.clone();
    change(&mut status);
    if *status == before {
        return;
    }
    let _ = tray.set_menu(menu(app, &status).ok());
    let mut tooltip = String::from("Traywise");
    if status.mics_muted {
        tooltip += "\nMicrophones muted";
    }
    if let Some(line) = &status.awake {
        tooltip += "\n";
        tooltip += line;
    }
    let _ = tray.set_tooltip(Some(tooltip));
    let _ = tray.set_icon(Some(badged(&tinted(&tray_icon(), status.accent), status.mics_muted, status.awake.is_some())));
}

/// The icon with its bright tile (the top left one) in `accent` (0xRRGGBB), like the website's logo: each
/// pixel there is mixed between the dark plate's colour and the accent by how light it is. The other three
/// tiles and the plate stay as they are. A zero accent (not set yet) leaves the icon as it is.
fn tinted(icon: &Image, accent: u32) -> Image<'static> {
    const PLATE: [f32; 3] = [11.0, 11.0, 12.0];
    const TILE: f32 = 244.0;
    let (width, height) = (icon.width() as usize, icon.height() as usize);
    let mut rgba = icon.rgba().to_vec();
    if accent != 0 {
        let target = [(accent >> 16 & 0xff) as f32, (accent >> 8 & 0xff) as f32, (accent & 0xff) as f32];
        for y in 0..height / 2 {
            for x in 0..width / 2 {
                let pixel = &mut rgba[(y * width + x) * 4..][..3];
                let light = 0.299 * pixel[0] as f32 + 0.587 * pixel[1] as f32 + 0.114 * pixel[2] as f32;
                let t = ((light - PLATE[1]) / (TILE - PLATE[1])).clamp(0.0, 1.0);
                for c in 0..3 {
                    pixel[c] = (PLATE[c] + (target[c] - PLATE[c]) * t).round() as u8;
                }
            }
        }
    }
    Image::new_owned(rgba, icon.width(), icon.height())
}

/// Distance from (px, py) to the segment a-b.
fn segment(px: f32, py: f32, a: (f32, f32), b: (f32, f32)) -> f32 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let t = (((px - a.0) * dx + (py - a.1) * dy) / (dx * dx + dy * dy)).clamp(0.0, 1.0);
    (px - a.0 - t * dx).hypot(py - a.1 - t * dy)
}

/// Anti-aliased coverage of a pixel at signed distance `d` (negative inside the shape).
fn cover(d: f32) -> f32 {
    (0.5 - d).clamp(0.0, 1.0)
}

/// Signed distance to a rounded box with corners (x0, y0)-(x1, y1) and corner radius `rd`.
fn rounded_box(u: f32, v: f32, (x0, y0, x1, y1): (f32, f32, f32, f32), rd: f32) -> f32 {
    let (qx, qy) = ((u - (x0 + x1) / 2.0).abs() - (x1 - x0) / 2.0 + rd, (v - (y0 + y1) / 2.0).abs() - (y1 - y0) / 2.0 + rd);
    qx.max(0.0).hypot(qy.max(0.0)) + qx.max(qy).min(0.0) - rd
}

/// Signed distance to a coffee cup (body, handle on the right, two steam strokes), like the Keep awake
/// tile's icon. `r` is the glyph's size and the origin its centre.
fn cup(u: f32, v: f32, r: f32) -> f32 {
    let (u, v) = (u / r, v / r);
    let handle = if u > 0.2 { ((u - 0.3).hypot(v - 0.33) - 0.3).abs() - 0.09 } else { f32::MAX };
    let steam = segment(u, v, (-0.5, -0.62), (-0.5, -0.38)).min(segment(u, v, (-0.1, -0.62), (-0.1, -0.38))) - 0.1;
    rounded_box(u, v, (-0.78, -0.05, 0.32, 0.72), 0.2).min(handle).min(steam) * r
}

/// How much of a white microphone with a slash through it covers (u, v), inside a disc of radius `r`.
fn muted_mic(u: f32, v: f32, r: f32) -> f32 {
    let mic = cover(segment(u, v, (0.0, -0.3 * r), (0.0, 0.2 * r)) - 0.3 * r);
    let slash = segment(u, v, (-0.5 * r, -0.55 * r), (0.5 * r, 0.55 * r));
    // The slash is cut out of the microphone with a thin gap, so the two stay apart at 16 px.
    (mic * (1.0 - cover(slash - 0.16 * r))).max(cover(slash - 0.07 * r))
}

/// Mixes `colour` into the pixel by `amount` and keeps the pixel's alpha at least that opaque.
fn paint(pixel: &mut [u8], colour: [u8; 3], amount: f32) {
    for c in 0..3 {
        pixel[c] = (pixel[c] as f32 + (colour[c] as f32 - pixel[c] as f32) * amount).round() as u8;
    }
    pixel[3] = (pixel[3] as f32 + (255.0 - pixel[3] as f32) * amount).round() as u8;
}

/// The Traywise icon with a badge per state, each cut out with a clear ring so it reads as a mark on
/// Traywise's icon rather than a separate (Windows) mute icon: a red disc with a slashed microphone
/// (bottom right) for muted microphones, and a white coffee cup (top right) for Keep awake. Shapes are sized
/// from the icon's width, drawn from distances to circles and line segments.
fn badged(icon: &Image, muted: bool, awake: bool) -> Image<'static> {
    let (width, height) = (icon.width(), icon.height());
    let mut rgba = icon.rgba().to_vec();
    let size = width as f32;
    let disc = size * 0.234; // the muted disc's radius
    let mug = size * 0.306; // the cup's size
    let ring = size * 0.047;
    let disc_at = size - disc - ring;
    let (mug_x, mug_y) = (size * 0.7125, size * 0.319);
    for y in 0..height {
        for x in 0..width {
            let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
            let pixel = &mut rgba[((y * width + x) * 4) as usize..][..4];
            if awake {
                let d = cup(px - mug_x, py - mug_y, mug);
                pixel[3] = (pixel[3] as f32 * (1.0 - cover(d - size * 0.031))).round() as u8;
                paint(pixel, AWAKE, cover(d));
            }
            if muted {
                let d = (px - disc_at).hypot(py - disc_at);
                pixel[3] = (pixel[3] as f32 * (1.0 - cover(d - disc - ring))).round() as u8;
                let colour = MUTED.map(|c| c as f32 + (255.0 - c as f32) * muted_mic(px - disc_at, py - disc_at, disc));
                paint(pixel, colour.map(|c| c as u8), cover(d - disc));
            }
        }
    }
    Image::new_owned(rgba, width, height)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_top_left_tile_takes_the_accent() {
        // 2 x 2 pixels: the top left is the bright tile, the others a dimmed tile and the plate.
        let icon = Image::new_owned([[244, 244, 242, 255], [104, 104, 104, 255], [104, 104, 104, 255], [11, 11, 12, 255]].concat(), 2, 2);
        let out = tinted(&icon, 0xd9_2b_24);
        assert_eq!(&out.rgba()[..4], [0xd9, 0x2b, 0x24, 255]);
        assert_eq!(&out.rgba()[4..], &icon.rgba()[4..], "the rest is untouched");
        assert_eq!(tinted(&icon, 0).rgba(), icon.rgba(), "no accent yet: unchanged");
    }

    #[test]
    fn badges_sit_in_their_corners_with_a_clear_ring() {
        // A black 64 x 64 icon, so white badge parts show up against it.
        let black = Image::new_owned([0, 0, 0, 255].repeat(64 * 64), 64, 64);
        let pixel = |image: &Image, x: u32, y: u32| image.rgba()[((y * 64 + x) * 4) as usize..][..4].to_vec();
        let (red, white, black_px) = ([MUTED[0], MUTED[1], MUTED[2], 255], [255; 4], [0, 0, 0, 255]);
        let both = badged(&black, true, true);
        assert_eq!(pixel(&both, 53, 46), red, "red disc, bottom right");
        assert_eq!(pixel(&both, 45, 40), white, "the microphone is white");
        assert_eq!(pixel(&both, 45, 24), white, "the cup's body, top right");
        assert_eq!(pixel(&both, 46, 30)[3], 0, "ring above the disc is see-through");
        assert_eq!(pixel(&both, 10, 10), black_px, "the icon itself is untouched");
        let muted = badged(&black, true, false);
        assert_eq!(pixel(&muted, 53, 46), red);
        assert_eq!(pixel(&muted, 45, 24), black_px, "no cup when Keep awake is off");
        let awake = badged(&black, false, true);
        assert_eq!(pixel(&awake, 45, 24), white);
        assert_eq!(pixel(&awake, 45, 35)[3], 0, "ring below the cup is see-through");
        assert_eq!(pixel(&awake, 53, 46), black_px, "no disc when mics are live");
        assert_eq!(badged(&black, false, false).rgba(), black.rgba(), "no badges: unchanged");
    }
}
