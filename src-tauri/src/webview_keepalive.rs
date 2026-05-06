//! Background WebView keep-alive: periodically posts a no-op `eval` to long-lived webviews so
//! macOS does not fully suspend their WebContent processes after extended hidden / unfocused
//! stretches (Cmd-H, minimized, on another Space, system sleep/wake).
//!
//! When a WKWebView's `WebContent` process is suspended, JS event listeners stop firing and the
//! webview's `setInterval` / `setTimeout` queue does not advance. From the user's perspective the
//! window opens but is dead — clicks don't register on the tray popover, autoplay-next via the
//! `audio-engine-rust-advanced` event listener doesn't cascade, and queued events all fire in a
//! burst the moment the window is shown again ("suddenly autoplays / suddenly responds"). Posting
//! `eval("void 0;")` from a Rust thread enqueues a script-runner task on the WebContent process,
//! which keeps it in the runnable set and prevents the WebKit suspension heuristic from firing.
//!
//! Frequency: 30 s. Tighter intervals waste battery; looser intervals risk the suspension
//! heuristic firing between ticks. The eval payload is intentionally trivial (`void 0;`) so the
//! cost per tick is negligible.

use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Manager, Wry};

const KEEPALIVE_INTERVAL_MS: u64 = 30_000;
const KEEPALIVE_LABELS: &[&str] = &["main", "tray-popover"];

static KEEPALIVE_ACTIVE: AtomicBool = AtomicBool::new(false);

/// Start the keep-alive thread (idempotent). Runs for the lifetime of the app.
pub fn start(app: AppHandle<Wry>) {
    if KEEPALIVE_ACTIVE.swap(true, Ordering::SeqCst) {
        return;
    }
    thread::spawn(move || {
        loop {
            thread::sleep(Duration::from_millis(KEEPALIVE_INTERVAL_MS));
            if !KEEPALIVE_ACTIVE.load(Ordering::SeqCst) {
                break;
            }
            for label in KEEPALIVE_LABELS {
                if let Some(win) = app.get_webview_window(label) {
                    /* `eval` errors only when the webview is gone or the runtime is shutting
                     * down — both fine to ignore here. The point is just to enqueue a task. */
                    let _ = win.eval("void 0;");
                }
            }
        }
    });
}
