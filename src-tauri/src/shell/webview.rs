//! WebView2 settings that Tauri doesn't expose, reached through WebView2's COM interfaces.

use tauri::WebviewWindow;
use webview2_com::Microsoft::Web::WebView2::Win32::{
    COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL, COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW,
    COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_NORMAL, ICoreWebView2_19,
};
use windows_core::Interface;

/// Puts WebView2 to sleep while the panel is hidden and wakes it before it's shown.
/// Hiding the window alone isn't enough: WebView2 keeps rendering behind it (~39% of a core
/// in the spike). Asleep, it also trims its memory, though much of it comes back within minutes.
pub fn set_active(win: &WebviewWindow, active: bool) {
    let _ = win.with_webview(move |webview| {
        let controller = webview.controller();
        let set_memory = |level: COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL| unsafe {
            if let Ok(core) = controller.CoreWebView2().and_then(|c| c.cast::<ICoreWebView2_19>()) {
                let _ = core.SetMemoryUsageTargetLevel(level);
            }
        };
        // Memory goes back to normal before drawing resumes, and is trimmed only once drawing stops.
        if active {
            set_memory(COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_NORMAL);
            let _ = unsafe { controller.SetIsVisible(true) };
        } else {
            let _ = unsafe { controller.SetIsVisible(false) };
            set_memory(COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW);
        }
    });
}
