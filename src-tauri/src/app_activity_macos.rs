//! Disable macOS App Nap for the host process.
//!
//! After ~hours of user idle, macOS throttles CPU, coalesces timers, and (under memory
//! pressure) compresses or swaps the process's pages. Coming back to play the next sample
//! then takes seconds while the OS un-throttles and decompresses — visible as UI lag,
//! waveform slow to draw, and audio slow to start. Subsequent playbacks are fast because
//! everything is hot again. Classic App Nap symptom.
//!
//! Music apps (Logic, Music.app, Spotify, Audio Hijack) prevent this by holding an
//! `NSProcessInfo.beginActivityWithOptions:reason:` token for their lifetime. We do the
//! same — `NSActivityUserInitiatedAllowingIdleSystemSleep` keeps the host unthrottled
//! while still letting the laptop sleep when the user walks away (we don't want to be a
//! battery hog).
//!
//! Note: this only covers the host. The `audio-engine` JUCE subprocess is independent;
//! it stays hot via the periodic keep-alive ping fired from `lib.rs`'s HEALTH sampler.
//!
//! # Threading
//!
//! Earlier versions deferred `beginActivity` onto a worker thread that slept 500 ms. That
//! was unsafe: a foreign ObjC exception thrown by the call would unwind through the Rust
//! frame and trip `panic_cannot_unwind` (Rust cannot catch foreign exceptions across an
//! extern "C" boundary), aborting the host before the WebView could render — a
//! white-screen launch crash that varied by 5–15 s depending on how long
//! `applicationDidFinishLaunching:` was still running. The fix is to call from
//! `RunEvent::Ready` instead, which fires on the main thread *after* `did_finish_launching`
//! has returned, inside the normal Tauri runloop where there is no extern "C" boundary
//! and a panic propagates up the hook and is logged.

#![cfg(target_os = "macos")]

use std::ptr::NonNull;
use std::sync::atomic::{AtomicBool, Ordering};

use block2::RcBlock;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::msg_send;
use objc2_foundation::{
    NSActivityOptions, NSNotification, NSNotificationCenter, NSProcessInfo, NSString,
};

/// Acquire the activity assertion. Idempotent (no-op on subsequent calls). Must be called
/// on the main thread, after the app has fully launched (i.e. from `RunEvent::Ready`,
/// not from inside `setup` — see module docs).
///
/// Wrapped in [`objc2::exception::catch`] so that an unexpected ObjC throw from the
/// Foundation activity API is logged rather than aborting the host with "Rust cannot
/// catch foreign exceptions, aborting" (which is what `std::panic::catch_unwind` would
/// have done — it cannot catch foreign exception classes).
pub fn install_on_main() {
    static INSTALLED: AtomicBool = AtomicBool::new(false);
    if INSTALLED.swap(true, Ordering::SeqCst) {
        return;
    }
    if let Err(exc) = objc2::exception::catch(std::panic::AssertUnwindSafe(begin_activity)) {
        let desc = exc
            .as_ref()
            .map(|e| format!("{e:?}"))
            .unwrap_or_else(|| "<no exception object>".to_string());
        crate::write_app_log(format!("APP NAP install threw ObjC exception: {desc}"));
    }
    if let Err(exc) =
        objc2::exception::catch(std::panic::AssertUnwindSafe(install_bg_observers))
    {
        let desc = exc
            .as_ref()
            .map(|e| format!("{e:?}"))
            .unwrap_or_else(|| "<no exception object>".to_string());
        crate::write_app_log(format!("APP NAP observers threw ObjC exception: {desc}"));
    }
}

/// Install observers for app foreground/background transitions.
///
/// `NSApplicationDidResignActiveNotification` fires the moment the user clicks away from
/// the app — the point at which App Nap eligibility begins (after the OS-defined idle
/// window). `NSApplicationDidBecomeActiveNotification` fires on focus return. Logging
/// both makes it possible to correlate "first playback was sluggish" reports with the
/// actual background dwell time, and to confirm the activity assertion is in fact
/// holding (a process under a held assertion still resigns active, but never naps).
fn install_bg_observers() {
    let center = NSNotificationCenter::defaultCenter();
    let nil_object: *const AnyObject = std::ptr::null();
    let nil_queue: *const AnyObject = std::ptr::null();

    {
        let block = RcBlock::new(move |_n: NonNull<NSNotification>| {
            crate::write_app_log("APP BG: resignActive (entering background)".to_string());
        });
        let block_ref: &block2::DynBlock<dyn Fn(NonNull<NSNotification>)> = &block;
        let name = NSString::from_str("NSApplicationDidResignActiveNotification");
        let token: Retained<AnyObject> = unsafe {
            msg_send![&*center,
                addObserverForName: &*name,
                object: nil_object,
                queue: nil_queue,
                usingBlock: block_ref]
        };
        std::mem::forget(token);
    }

    {
        let block = RcBlock::new(move |_n: NonNull<NSNotification>| {
            crate::write_app_log("APP BG: becomeActive (returning to foreground)".to_string());
        });
        let block_ref: &block2::DynBlock<dyn Fn(NonNull<NSNotification>)> = &block;
        let name = NSString::from_str("NSApplicationDidBecomeActiveNotification");
        let token: Retained<AnyObject> = unsafe {
            msg_send![&*center,
                addObserverForName: &*name,
                object: nil_object,
                queue: nil_queue,
                usingBlock: block_ref]
        };
        std::mem::forget(token);
    }
}

fn begin_activity() {
    let pi = NSProcessInfo::processInfo();
    let reason = NSString::from_str("audio_haxor: keep audio engine + UI responsive across idle");
    // Same option set Music.app uses. Keeps the host unthrottled (no App Nap) while still
    // permitting display / system sleep when the user walks away.
    let token = pi.beginActivityWithOptions_reason(
        NSActivityOptions::UserInitiatedAllowingIdleSystemSleep,
        &reason,
    );
    // Token must outlive the process for the assertion to hold. `endActivity:` would
    // release the assertion; never call it.
    std::mem::forget(token);
}
