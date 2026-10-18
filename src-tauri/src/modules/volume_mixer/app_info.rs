//! A friendly name and an icon for a process, read from its .exe.

use std::ffi::c_void;
use std::path::Path;

use base64::Engine;
use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::Graphics::Gdi::{
    BI_RGB, BITMAP, BITMAPINFO, BITMAPINFOHEADER, CreateCompatibleDC, DIB_RGB_COLORS, DeleteDC, DeleteObject,
    GetDIBits, GetObjectW, HGDIOBJ,
};
use windows::Win32::Storage::FileSystem::{GetFileVersionInfoSizeW, GetFileVersionInfoW, VerQueryValueW};
use windows::Win32::System::Threading::{
    OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
};
use windows::Win32::UI::Shell::ExtractIconExW;
use windows::Win32::UI::WindowsAndMessaging::{DestroyIcon, GetIconInfo, HICON, ICONINFO};
use windows::core::{HSTRING, PWSTR, w};

pub struct AppInfo {
    pub name: String,
    pub icon: Option<String>,
}

pub fn lookup(pid: u32) -> AppInfo {
    let Some(path) = exe_path(pid) else {
        return AppInfo { name: format!("Process {pid}"), icon: None };
    };
    let name = description(&path).unwrap_or_else(|| fallback_name(&path));
    AppInfo { name, icon: icon_png(&path) }
}

/// "spotify.exe" → "Spotify", for apps without a description in their .exe.
fn fallback_name(path: &str) -> String {
    let stem = Path::new(path).file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let mut chars = stem.chars();
    chars.next().map(|c| c.to_uppercase().chain(chars).collect()).unwrap_or(stem)
}

fn exe_path(pid: u32) -> Option<String> {
    unsafe {
        let process: HANDLE = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buf = [0u16; 1024];
        let mut len = buf.len() as u32;
        let result = QueryFullProcessImageNameW(process, PROCESS_NAME_WIN32, PWSTR(buf.as_mut_ptr()), &mut len);
        let _ = CloseHandle(process);
        result.ok()?;
        Some(String::from_utf16_lossy(&buf[..len as usize]))
    }
}

/// The "File description" from the .exe's version info, e.g. "Google Chrome" for chrome.exe.
fn description(path: &str) -> Option<String> {
    unsafe {
        let path = HSTRING::from(path);
        let size = GetFileVersionInfoSizeW(&path, None);
        if size == 0 {
            return None;
        }
        let mut data = vec![0u8; size as usize];
        GetFileVersionInfoW(&path, None, size, data.as_mut_ptr() as *mut c_void).ok()?;

        let mut value: *mut c_void = std::ptr::null_mut();
        let mut len = 0u32;
        // The first language and code page the strings are stored under.
        VerQueryValueW(data.as_ptr() as *const c_void, w!("\\VarFileInfo\\Translation"), &mut value, &mut len)
            .as_bool()
            .then_some(())?;
        if len < 4 {
            return None;
        }
        let [lang, codepage] = *(value as *const [u16; 2]);
        let key = HSTRING::from(format!("\\StringFileInfo\\{lang:04x}{codepage:04x}\\FileDescription"));
        VerQueryValueW(data.as_ptr() as *const c_void, &key, &mut value, &mut len).as_bool().then_some(())?;
        let text = String::from_utf16_lossy(std::slice::from_raw_parts(value as *const u16, len as usize));
        let text = text.trim_end_matches('\0').trim();
        (!text.is_empty()).then(|| text.to_string())
    }
}

/// The .exe's main icon (32 × 32) as a PNG data URL.
fn icon_png(path: &str) -> Option<String> {
    unsafe {
        let mut icon = HICON::default();
        if ExtractIconExW(&HSTRING::from(path), 0, Some(&mut icon), None, 1) == 0 || icon.is_invalid() {
            return None;
        }
        let pixels = icon_pixels(icon);
        let _ = DestroyIcon(icon);
        let (width, height, rgba) = pixels?;

        let mut png = Vec::new();
        let mut encoder = png::Encoder::new(&mut png, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.write_header().ok()?.write_image_data(&rgba).ok()?;
        Some(format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(png)))
    }
}

/// Reads an icon's colour bitmap as RGBA pixels.
unsafe fn icon_pixels(icon: HICON) -> Option<(u32, u32, Vec<u8>)> {
    unsafe {
        let mut info = ICONINFO::default();
        GetIconInfo(icon, &mut info).ok()?;
        let color = HGDIOBJ(info.hbmColor.0);
        let mut bitmap = BITMAP::default();
        let read = GetObjectW(color, size_of::<BITMAP>() as i32, Some(&mut bitmap as *mut _ as *mut c_void));
        let (width, height) = (bitmap.bmWidth, bitmap.bmHeight);

        let mut bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -height, // negative: rows top to bottom
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut pixels = vec![0u8; (width * height * 4).max(0) as usize];
        let dc = CreateCompatibleDC(None);
        let rows = if read == 0 || width <= 0 || height <= 0 {
            0
        } else {
            GetDIBits(dc, info.hbmColor, 0, height as u32, Some(pixels.as_mut_ptr() as *mut c_void), &mut bmi, DIB_RGB_COLORS)
        };
        let _ = DeleteDC(dc);
        let _ = DeleteObject(color);
        let _ = DeleteObject(HGDIOBJ(info.hbmMask.0));
        if rows == 0 {
            return None;
        }

        let has_alpha = pixels.as_chunks::<4>().0.iter().any(|p| p[3] != 0);
        for p in pixels.as_chunks_mut::<4>().0 {
            p.swap(0, 2); // Windows gives BGRA
            if !has_alpha {
                // shortcut: old icons without an alpha channel get a solid square instead of using
                // their mask bitmap; read the mask if such icons show up looking boxy.
                p[3] = 255;
            }
        }
        Some((width as u32, height as u32, pixels))
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn fallback_name_capitalises_the_exe_name() {
        assert_eq!(super::fallback_name(r"C:\Apps\spotify.exe"), "Spotify");
        assert_eq!(super::fallback_name(r"C:\Apps\Discord.exe"), "Discord");
        assert_eq!(super::fallback_name(""), "");
    }
}
