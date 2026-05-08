//! Tray popover: Escape closes the window.
//!
//! On macOS the popover is an undecorated `NSPanel` + `WKWebView`. The webview often never
//! becomes first responder, so JS `keydown` never runs. `NSEvent` local monitoring runs on the
//! main thread for the active app only (not a system-wide global shortcut).

#[cfg(target_os = "macos")]
pub fn install(app: tauri::AppHandle) {
    use block2::RcBlock;
    use objc2_app_kit::{NSEvent, NSEventMask};
    use std::ops::Deref;
    use std::ptr::NonNull;
    use tauri::Manager;

    let app = app.clone();
    let block = RcBlock::new(move |event: NonNull<NSEvent>| -> *mut NSEvent {
        let e = unsafe { event.as_ref() };
        if e.keyCode() != 53 {
            return event.as_ptr();
        }
        if app.get_webview_window("tray-popover").is_none() {
            return event.as_ptr();
        }
        /* Park off-screen rather than `hide()` so WebContent doesn't become a suspension
         * candidate. `park_tray_popover_offscreen` consults its own visibility flag, so calling
         * it when the popover is already parked is a cheap no-op — we still consume the Escape
         * keystroke (returning null) only if the user-facing visibility flag was on, otherwise
         * the keystroke falls through to whatever else handles Escape. */
        if !crate::tray_menu::tray_popover_user_visible() {
            return event.as_ptr();
        }
        crate::tray_menu::park_tray_popover_offscreen(&app);
        std::ptr::null_mut()
    });

    let handler: &block2::DynBlock<dyn Fn(NonNull<NSEvent>) -> *mut NSEvent> = block.deref();
    unsafe {
        if let Some(monitor) =
            NSEvent::addLocalMonitorForEventsMatchingMask_handler(NSEventMask::KeyDown, handler)
        {
            /* Monitor must stay alive; we never call `removeMonitor`. */
            std::mem::forget(monitor);
        }
    }
}

#[cfg(not(target_os = "macos"))]
pub fn install(_app: tauri::AppHandle) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_install_compiles() {
        // We cannot easily test macOS specific NSEvent monitoring,
        // but we can ensure this module is included in tests.
        assert!(true);
    }
}
