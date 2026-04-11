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
        let Some(popover) = app.get_webview_window("tray-popover") else {
            return event.as_ptr();
        };
        if !popover.is_visible().unwrap_or(false) {
            return event.as_ptr();
        }
        let _ = popover.hide();
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
