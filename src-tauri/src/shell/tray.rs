//! The tray icon: left-click toggles the panel, right-click opens a menu with Quit. While microphones
//! are muted or Keep awake is on, it gets a dot in a corner and a line in its tooltip.

use std::sync::Mutex;

use tauri::image::Image;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager};

use super::panel;

const ID: &str = "main";
const MUTED: [u8; 3] = [0xe5, 0x48, 0x4d]; // red, bottom-right
const AWAKE: [u8; 3] = [0x3d, 0xd6, 0x8c]; // green, top-right

/// What the icon shows besides Traywise itself. Modules set their part, and clear it when they stop.
#[derive(Default, PartialEq)]
struct Status {
    mics_muted: bool,
    /// The tooltip line, like "Keeping awake until 3:40 PM". None when Keep awake is off.
    awake: Option<String>,
}

pub fn create(app: &tauri::App) -> tauri::Result<()> {
    app.manage(Mutex::new(Status::default()));
    let quit = MenuItem::with_id(app, "quit", "Quit Traywise", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&quit])?;
    TrayIconBuilder::with_id(ID)
        .icon(app.default_window_icon().cloned().expect("app icon is set in tauri.conf.json"))
        .tooltip("Traywise")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| {
            if event.id == "quit" {
                app.exit(0);
            }
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
    let before = Status { mics_muted: status.mics_muted, awake: status.awake.clone() };
    change(&mut status);
    if *status == before {
        return;
    }
    let mut tooltip = String::from("Traywise");
    if status.mics_muted {
        tooltip += "\nMicrophones muted";
    }
    if let Some(line) = &status.awake {
        tooltip += "\n";
        tooltip += line;
    }
    let _ = tray.set_tooltip(Some(tooltip));
    let icon = app.default_window_icon().expect("app icon is set in tauri.conf.json");
    let _ = tray.set_icon(Some(badged(icon, status.mics_muted, status.awake.is_some())));
}

/// The Traywise icon with a dot per state, each cut out with a clear ring so it reads as a mark on
/// Traywise's icon rather than a separate (Windows) mute icon.
fn badged(icon: &Image, muted: bool, awake: bool) -> Image<'static> {
    let (width, height) = (icon.width(), icon.height());
    let mut rgba = icon.rgba().to_vec();
    let size = width as f32;
    let radius = size * 0.2;
    let ring = size * 0.07;
    let centre = size - radius - ring;
    let dots = [(muted, (centre, centre), MUTED), (awake, (centre, radius + ring), AWAKE)];
    for (_, (cx, cy), colour) in dots.into_iter().filter(|dot| dot.0) {
        for y in 0..height {
            for x in 0..width {
                let distance = (x as f32 + 0.5 - cx).hypot(y as f32 + 0.5 - cy);
                let pixel = &mut rgba[((y * width + x) * 4) as usize..][..4];
                if distance <= radius {
                    pixel.copy_from_slice(&[colour[0], colour[1], colour[2], 255]);
                } else if distance <= radius + ring {
                    pixel[3] = 0;
                }
            }
        }
    }
    Image::new_owned(rgba, width, height)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn badges_sit_in_their_corners_with_a_clear_ring() {
        let white = Image::new_owned(vec![255; 64 * 64 * 4], 64, 64);
        let pixel = |image: &Image, x: u32, y: u32| image.rgba()[((y * 64 + x) * 4) as usize..][..4].to_vec();
        let both = badged(&white, true, true);
        assert_eq!(pixel(&both, 51, 51), [MUTED[0], MUTED[1], MUTED[2], 255]);
        assert_eq!(pixel(&both, 51, 12), [AWAKE[0], AWAKE[1], AWAKE[2], 255]);
        assert_eq!(pixel(&both, 46, 32)[3], 0, "ring above the muted dot is see-through");
        assert_eq!(pixel(&both, 10, 10), [255; 4], "the icon itself is untouched");
        let muted = badged(&white, true, false);
        assert_eq!(pixel(&muted, 51, 12), [255; 4], "no keep-awake dot when it's off");
    }
}
