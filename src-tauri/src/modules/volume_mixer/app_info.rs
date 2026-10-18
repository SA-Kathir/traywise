//! A friendly name and an icon for a process: the name from its .exe, the icon from the
//! Windows shell (which also knows the icons of packaged "Store" apps like Arc).

use std::ffi::c_void;
use std::path::Path;

use base64::Engine;
use windows::Win32::Foundation::{CloseHandle, ERROR_SUCCESS, HANDLE, SIZE};
use windows::Win32::Graphics::Gdi::{
    BI_RGB, BITMAP, BITMAPINFO, BITMAPINFOHEADER, CreateCompatibleDC, DIB_RGB_COLORS, DeleteDC, DeleteObject,
    GetDIBits, GetObjectW, HBITMAP, HGDIOBJ,
};
use windows::Win32::Storage::FileSystem::{GetFileVersionInfoSizeW, GetFileVersionInfoW, VerQueryValueW};
use windows::Win32::Storage::Packaging::Appx::GetApplicationUserModelId;
use windows::Win32::System::Threading::{
    OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
};
use windows::Win32::UI::Shell::{IShellItemImageFactory, SHCreateItemFromParsingName, SIIGBF_ICONONLY};
use windows::core::{HSTRING, PWSTR, w};

pub struct AppInfo {
    pub name: String,
    pub icon: Option<String>,
}

pub fn lookup(pid: u32) -> AppInfo {
    let Some((path, app_id)) = process_ids(pid) else {
        return AppInfo { name: format!("Process {pid}"), icon: None };
    };
    let name = description(&path).unwrap_or_else(|| fallback_name(&path));
    // Packaged apps keep their icons in the package, not the .exe: ask the shell by app id.
    let shell_name = app_id.map_or(path, |id| format!("shell:AppsFolder{}{id}", std::path::MAIN_SEPARATOR));
    AppInfo { name, icon: icon_png(&shell_name) }
}

/// "spotify.exe" → "Spotify", for apps without a description in their .exe.
fn fallback_name(path: &str) -> String {
    let stem = Path::new(path).file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let mut chars = stem.chars();
    chars.next().map(|c| c.to_uppercase().chain(chars).collect()).unwrap_or(stem)
}

/// The process's .exe path, and its app id if it's a packaged app.
fn process_ids(pid: u32) -> Option<(String, Option<String>)> {
    unsafe {
        let process: HANDLE = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buf = [0u16; 1024];
        let mut len = buf.len() as u32;
        let path = QueryFullProcessImageNameW(process, PROCESS_NAME_WIN32, PWSTR(buf.as_mut_ptr()), &mut len)
            .ok()
            .map(|()| String::from_utf16_lossy(&buf[..len as usize]));
        let mut len = buf.len() as u32;
        // Fails with "no application" for ordinary, non-packaged programs.
        let app_id = (GetApplicationUserModelId(process, &mut len, Some(PWSTR(buf.as_mut_ptr()))) == ERROR_SUCCESS)
            .then(|| String::from_utf16_lossy(&buf[..len.saturating_sub(1) as usize])); // len counts the final NUL
        let _ = CloseHandle(process);
        Some((path?, app_id))
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


/// The icon the shell shows for a file or app (32 × 32), as a PNG data URL.
fn icon_png(shell_name: &str) -> Option<String> {
    unsafe {
        let item: IShellItemImageFactory = SHCreateItemFromParsingName(&HSTRING::from(shell_name), None).ok()?;
        let bitmap = item.GetImage(SIZE { cx: 32, cy: 32 }, SIIGBF_ICONONLY).ok()?;
        let pixels = bitmap_pixels(bitmap);
        let _ = DeleteObject(HGDIOBJ(bitmap.0));
        let (width, height, rgba) = pixels?;

        let mut png = Vec::new();
        let mut encoder = png::Encoder::new(&mut png, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.write_header().ok()?.write_image_data(&rgba).ok()?;
        Some(format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(png)))
    }
}

/// Reads a 32-bit bitmap as plain (not premultiplied) RGBA pixels.
unsafe fn bitmap_pixels(bitmap: HBITMAP) -> Option<(u32, u32, Vec<u8>)> {
    unsafe {
        let mut info = BITMAP::default();
        let read = GetObjectW(HGDIOBJ(bitmap.0), size_of::<BITMAP>() as i32, Some(&mut info as *mut _ as *mut c_void));
        let (width, height) = (info.bmWidth, info.bmHeight);
        if read == 0 || width <= 0 || height <= 0 {
            return None;
        }
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
        let mut pixels = vec![0u8; (width * height * 4) as usize];
        let dc = CreateCompatibleDC(None);
        let rows =
            GetDIBits(dc, bitmap, 0, height as u32, Some(pixels.as_mut_ptr() as *mut c_void), &mut bmi, DIB_RGB_COLORS);
        let _ = DeleteDC(dc);
        if rows == 0 {
            return None;
        }
        to_rgba(&mut pixels);
        Some((width as u32, height as u32, pixels))
    }
}

/// Windows' premultiplied BGRA → the straight RGBA a PNG holds.
fn to_rgba(pixels: &mut [u8]) {
    let has_alpha = pixels.as_chunks::<4>().0.iter().any(|p| p[3] != 0);
    for p in pixels.as_chunks_mut::<4>().0 {
        p.swap(0, 2);
        if !has_alpha {
            p[3] = 255; // a bitmap with no alpha channel at all is fully opaque
        } else if let alpha @ 1..=254 = u16::from(p[3]) {
            for c in &mut p[..3] {
                *c = (u16::from(*c) * 255 / alpha).min(255) as u8;
            }
        }
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

    #[test]
    fn to_rgba_swaps_channels_and_undoes_premultiplying() {
        // BGRA: opaque blue, half-transparent premultiplied red, fully transparent.
        let mut pixels = [255, 0, 0, 255, 0, 0, 128, 128, 0, 0, 0, 0];
        super::to_rgba(&mut pixels);
        assert_eq!(pixels, [0, 0, 255, 255, 255, 0, 0, 128, 0, 0, 0, 0]);

        let mut no_alpha = [10, 20, 30, 0];
        super::to_rgba(&mut no_alpha);
        assert_eq!(no_alpha, [30, 20, 10, 255], "no alpha channel means opaque");
    }
}
