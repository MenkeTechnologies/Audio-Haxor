//! Mission Control / Spaces window-preview snapshot fix (macOS).
//!
//! WKWebView renders out-of-process via the WebContent + GPU XPC services. The frames it
//! delivers live in IOSurfaces attached to a `CARemoteLayer`, which the WindowServer's
//! snapshot mechanism (used by Mission Control / Spaces / cmd-tab thumbnails when the
//! window is in another Space) does not capture reliably — the preview shows a solid
//! gray/dark rectangle. WebKit bug 146877, open since 2015.
//!
//! Workaround: keep an `NSImageView` overlay layered **above** the WKWebView inside the
//! window's content view. Periodically refresh it via `WKWebView.takeSnapshot:` while the
//! window is the key window. Toggle visible only on **occlusion-state change** (window
//! genuinely off-screen / behind Mission Control / on another Space) so the system's
//! window snapshot captures the bitmap-backed image view's `CALayer` instead of the empty
//! `CARemoteLayer`. Plain key-focus loss (e.g. the tray popover stealing focus, Cmd-Tab
//! to an app that does not fully occlude us) does NOT show the overlay — pinning a static
//! bitmap over a still-on-screen live WebView looks like the app froze. The resign-key
//! observer is kept only to refresh the cached snapshot so it's current the moment
//! occlusion does fire. Live WebView resumes when occlusion clears.
//!
//! Public API only. WKWebView snapshot has been available since macOS 10.13.

#![cfg(target_os = "macos")]

use std::ptr::NonNull;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use block2::RcBlock;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::msg_send;
use objc2_app_kit::{
    NSAutoresizingMaskOptions, NSImage, NSImageScaling, NSImageView, NSView, NSWindow,
    NSWindowOcclusionState, NSWindowOrderingMode,
};
use objc2_foundation::{
    MainThreadMarker, NSNotification, NSNotificationCenter, NSString,
};
use objc2_web_kit::{WKSnapshotConfiguration, WKWebView};
use tauri::Manager;

/// Periodic refresh interval. Was 1500 ms originally; that was the cause of an overnight
/// kernel panic via `userspace watchdog timeout: no successful checkins from WindowServer
/// in 122 seconds`. ~33 000 snapshots over 14 hours of overnight idle accumulated enough
/// IOSurface / GPU compositor pressure to make WindowServer miss its 122-second watchdog
/// window, forcing a SoC-wide reset.
///
/// The periodic refresh is in fact mostly redundant — the `NSWindowDidResignKey` and
/// `NSWindowDidChangeOcclusionState` notification handlers already call
/// `capture_snapshot_into` synchronously *at the moment* the window loses focus or
/// becomes occluded, so the bitmap that Mission Control / Spaces / Cmd-Tab capture is
/// always fresh. The periodic loop is kept only as a defense against pathological cases
/// where the window changes Space without firing either notification, but the cadence is
/// now a much gentler 30 s.
const SNAPSHOT_REFRESH_MS: u64 = 30_000;

/// Install once. Idempotent. Schedules `install_main` onto the main thread *after a
/// short delay* so it cannot run during `applicationDidFinishLaunching:` — that callback
/// is invoked via objc-runtime FFI and any unwind inside would `abort()` the process
/// (cannot unwind through C). The delay also gives wry's WKWebView a chance to be
/// inserted into the window's contentView before we go looking for it.
///
/// The install body is wrapped in [`objc2::exception::catch`], not
/// [`std::panic::catch_unwind`]. The latter aborts with "Rust cannot catch foreign
/// exceptions, aborting" when an ObjC exception (e.g. `addSubview:positioned:relativeTo:`
/// rejecting a non-sibling, or `takeSnapshot…` failing on an unloaded WebView) reaches
/// it; the former is built for ObjC throws and reports them without aborting.
pub fn install(app: tauri::AppHandle) {
    static INSTALLED: AtomicBool = AtomicBool::new(false);
    if INSTALLED.swap(true, Ordering::SeqCst) {
        return;
    }
    std::thread::Builder::new()
        .name("ah-space-preview-init".to_string())
        .spawn(move || {
            std::thread::sleep(Duration::from_millis(750));
            let app2 = app.clone();
            let _ = app.run_on_main_thread(move || {
                if let Err(exc) = objc2::exception::catch(std::panic::AssertUnwindSafe(|| {
                    install_main(app2);
                })) {
                    let desc = exc
                        .as_ref()
                        .map(|e| format!("{e:?}"))
                        .unwrap_or_else(|| "<no exception object>".to_string());
                    crate::write_app_log(format!("SPACE PREVIEW install threw ObjC exception: {desc}"));
                }
            });
        })
        .ok();
}

fn install_main(app: tauri::AppHandle) {
    let mtm = match MainThreadMarker::new() {
        Some(m) => m,
        None => return,
    };
    let win = match app.get_webview_window("main") {
        Some(w) => w,
        None => return,
    };
    let ns_window_ptr = match win.ns_window() {
        Ok(p) => p as *mut NSWindow,
        Err(_) => return,
    };
    if ns_window_ptr.is_null() {
        return;
    }
    let ns_window: &NSWindow = unsafe { &*ns_window_ptr };
    let content_view = match ns_window.contentView() {
        Some(v) => v,
        None => return,
    };
    let webview = match find_wkwebview_subview(&content_view) {
        Some(v) => v,
        None => return,
    };

    // Build the overlay NSImageView pinned to the WebView's frame. Autoresizing keeps it
    // matched on resize / DPI change.
    let overlay = NSImageView::new(mtm);
    let frame = webview.frame();
    overlay.setFrame(frame);
    overlay.setAutoresizingMask(
        NSAutoresizingMaskOptions::ViewWidthSizable | NSAutoresizingMaskOptions::ViewHeightSizable,
    );
    overlay.setImageScaling(NSImageScaling::ScaleAxesIndependently);
    overlay.setHidden(true);

    // Insert above the WKWebView under the same parent view.
    unsafe {
        let parent: Retained<NSView> = match webview.superview() {
            Some(p) => p,
            None => Retained::from(content_view.clone()),
        };
        parent.addSubview_positioned_relativeTo(
            &overlay,
            NSWindowOrderingMode::Above,
            Some(&webview),
        );
    }

    // Capture an initial snapshot ASAP. WebContent is usually live by app-setup time.
    capture_snapshot_into(&webview, &overlay);

    install_window_observers(ns_window, webview.clone(), overlay.clone());

    // Periodic refresh — keeps the cached snapshot recent so the moment the window
    // resigns key (Mission Control invocation) we already have a current bitmap. Skip
    // when the overlay is already showing (window not the key window — takeSnapshot can
    // return blank on occluded webview, and we have the last-good frame anyway).
    static REFRESH_THREAD_STARTED: AtomicBool = AtomicBool::new(false);
    if !REFRESH_THREAD_STARTED.swap(true, Ordering::SeqCst) {
        let app_for_loop = app.clone();
        std::thread::Builder::new()
            .name("ah-space-preview-refresh".to_string())
            .spawn(move || loop {
                std::thread::sleep(Duration::from_millis(SNAPSHOT_REFRESH_MS));
                let _ = app_for_loop.run_on_main_thread(|| {
                    let mtm = match MainThreadMarker::new() {
                        Some(m) => m,
                        None => return,
                    };
                    if let Some((wv, ov)) = main_window_views(mtm) {
                        if !ov.isHidden() {
                            return;
                        }
                        capture_snapshot_into(&wv, &ov);
                    }
                });
            })
            .ok();
    }
}

fn main_window_views(
    mtm: MainThreadMarker,
) -> Option<(Retained<WKWebView>, Retained<NSImageView>)> {
    let app = objc2_app_kit::NSApplication::sharedApplication(mtm);
    let main_win = app.mainWindow()?;
    let content = main_win.contentView()?;
    let wv = find_wkwebview_subview(&content)?;
    let ov = find_overlay_subview(&content)?;
    Some((wv, ov))
}

/// Recursive subview walk for the first `WKWebView` (Tauri/wry adds it as a deep subview
/// of the window's `contentView`). wry registers its own subclass — current name is
/// `"wry::wkwebview::class::wry_web_view::WryWebView<ver>"` — so a direct equality compare
/// against `"WKWebView"` never matches. Walk the superclass chain via objc2's
/// [`AnyClass::superclass`] until we hit `WKWebView` (or run out). This avoids both a
/// hard-coded subclass name (would break on any wry rename) and the typed `is_kind_of`
/// path (which depends on `objc2-web-kit` version-pinned `ClassType` impls).
fn find_wkwebview_subview(view: &NSView) -> Option<Retained<WKWebView>> {
    let subviews = view.subviews();
    for i in 0..subviews.len() {
        let sub = subviews.objectAtIndex(i);
        if subview_inherits(&sub, "WKWebView") {
            // Re-message the same object as a WKWebView. `subview` already holds a +1
            // retain via the array accessor; transmute the smart pointer to the typed
            // class. Safe because the runtime class hierarchy includes WKWebView.
            return Some(unsafe { Retained::cast_unchecked::<WKWebView>(sub) });
        }
        if let Some(found) = find_wkwebview_subview(&sub) {
            return Some(found);
        }
    }
    None
}

fn find_overlay_subview(view: &NSView) -> Option<Retained<NSImageView>> {
    let subviews = view.subviews();
    for i in 0..subviews.len() {
        let sub = subviews.objectAtIndex(i);
        if subview_inherits(&sub, "NSImageView") {
            return Some(unsafe { Retained::cast_unchecked::<NSImageView>(sub) });
        }
        if let Some(found) = find_overlay_subview(&sub) {
            return Some(found);
        }
    }
    None
}

/// True if `view`'s class is `target_name` or any of its ancestors is. Walks the
/// superclass chain via [`AnyClass::superclass`] (i.e. `class_getSuperclass`).
///
/// Used instead of an instance-class equality check because wry subclasses `WKWebView`
/// with its own runtime class (currently `"wry::wkwebview::class::wry_web_view::WryWebView<ver>"`).
/// A name compare against `"WKWebView"` would never match. Class names are read via
/// [`AnyClass::name`] (`class_getName`); sending the `name` selector with `msg_send!`
/// throws `NSInvalidArgumentException` on classes whose metaclass does not implement
/// `+name`, which includes wry's class.
fn subview_inherits(view: &NSView, target_name: &str) -> bool {
    unsafe {
        let cls_ptr: *const objc2::runtime::AnyClass = msg_send![view, class];
        if cls_ptr.is_null() {
            return false;
        }
        let mut cls: Option<&objc2::runtime::AnyClass> = Some(&*cls_ptr);
        while let Some(c) = cls {
            if c.name().to_str().ok() == Some(target_name) {
                return true;
            }
            cls = c.superclass();
        }
        false
    }
}

fn capture_snapshot_into(webview: &WKWebView, overlay: &NSImageView) {
    // Clone the ref-counted handle so the completion block can move it.
    let overlay_for_block: Retained<NSImageView> =
        unsafe { Retained::retain(overlay as *const NSImageView as *mut NSImageView) }
            .expect("overlay retain");
    let block = RcBlock::new(
        move |image: *mut NSImage, _error: *mut AnyObject| {
            if image.is_null() {
                return;
            }
            let img: &NSImage = unsafe { &*image };
            overlay_for_block.setImage(Some(img));
        },
    );
    let block_ref: &block2::DynBlock<dyn Fn(*mut NSImage, *mut AnyObject)> = &block;
    unsafe {
        let nil_cfg: *mut WKSnapshotConfiguration = std::ptr::null_mut();
        let _: () = msg_send![webview,
            takeSnapshotWithConfiguration: nil_cfg,
            completionHandler: block_ref];
    }
}

fn install_window_observers(
    window: &NSWindow,
    webview: Retained<WKWebView>,
    overlay: Retained<NSImageView>,
) {
    let center = NSNotificationCenter::defaultCenter();
    let window_obj: *const NSWindow = window;
    let nil_queue: *const AnyObject = std::ptr::null();

    // NSWindowDidResignKeyNotification — about to lose focus (Mission Control invoke,
    // Space switch, app switch, OR tray-popover stealing key focus while main window
    // is fully on screen). Refresh the cached snapshot so it's current for the moment
    // the window actually becomes occluded — but do NOT show the overlay here. Resign
    // key fires for plain focus loss too (clicking the tray popover, clicking another
    // app while ours stays visible), and pinning a static bitmap over the live WKWebView
    // in that case looks like the app froze. Let `NSWindowDidChangeOcclusionStateNotification`
    // alone decide when to actually swap to the overlay; that one fires only when the
    // window is truly off-screen / occluded, which is the only case where Mission
    // Control / Spaces previews need the snapshot.
    {
        let webview = webview.clone();
        let overlay = overlay.clone();
        let block = RcBlock::new(move |_n: NonNull<NSNotification>| {
            capture_snapshot_into(&webview, &overlay);
        });
        let block_ref: &block2::DynBlock<dyn Fn(NonNull<NSNotification>)> = &block;
        let name = NSString::from_str("NSWindowDidResignKeyNotification");
        let token: Retained<AnyObject> = unsafe {
            msg_send![&*center,
                addObserverForName: &*name,
                object: window_obj as *const AnyObject,
                queue: nil_queue,
                usingBlock: block_ref]
        };
        std::mem::forget(token);
    }

    // NSWindowDidBecomeKeyNotification — focus regained, hide overlay so live WebView
    // shows.
    {
        let overlay = overlay.clone();
        let block = RcBlock::new(move |_n: NonNull<NSNotification>| {
            overlay.setHidden(true);
        });
        let block_ref: &block2::DynBlock<dyn Fn(NonNull<NSNotification>)> = &block;
        let name = NSString::from_str("NSWindowDidBecomeKeyNotification");
        let token: Retained<AnyObject> = unsafe {
            msg_send![&*center,
                addObserverForName: &*name,
                object: window_obj as *const AnyObject,
                queue: nil_queue,
                usingBlock: block_ref]
        };
        std::mem::forget(token);
    }

    // NSWindowDidChangeOcclusionStateNotification — fires both directions; inspect the
    // current occlusion state inside the handler.
    {
        let webview = webview.clone();
        let overlay = overlay.clone();
        let window_obj: *const NSWindow = window_obj;
        let block = RcBlock::new(move |_n: NonNull<NSNotification>| {
            let win = unsafe { &*window_obj };
            let visible = win
                .occlusionState()
                .contains(NSWindowOcclusionState::Visible);
            if visible {
                overlay.setHidden(true);
            } else {
                capture_snapshot_into(&webview, &overlay);
                overlay.setHidden(false);
            }
        });
        let block_ref: &block2::DynBlock<dyn Fn(NonNull<NSNotification>)> = &block;
        let name = NSString::from_str("NSWindowDidChangeOcclusionStateNotification");
        let token: Retained<AnyObject> = unsafe {
            msg_send![&*center,
                addObserverForName: &*name,
                object: window_obj as *const AnyObject,
                queue: nil_queue,
                usingBlock: block_ref]
        };
        std::mem::forget(token);
    }
}
