//! Single-directory, non-recursive filesystem watcher for the File Browser
//! tab.
//!
//! Distinct from `file_watcher.rs`, which is recursive + multi-dir +
//! extension-filtered for inventory auto-scan. This watcher is interactive
//! and exists purely to keep the visible file list in sync with disk: any
//! create / modify / remove / rename inside the current folder fires
//! `file-browser-change` to the frontend, which re-runs `list_directory`.
//!
//! Design notes:
//! - **Non-recursive.** The file browser only shows one level at a time;
//!   recursive events would be noise + cost.
//! - **300 ms debounce.** Bursty operations (save-as in TextEdit,
//!   `npm install`, file copies) generate many notify events per logical
//!   change; 300 ms is the upper bound of human-perceptible latency and
//!   collapses bursts into one reload.
//! - **No event-kind filter.** Anything that mutates the directory listing
//!   should trigger a reload — including renames (no per-row delta logic on
//!   the frontend).
//! - **Single-dir lifecycle.** Each `set(Some(dir))` replaces the previous
//!   watch entirely. `set(None)` stops watching (called on tab switch away
//!   from Files, or app shutdown).
//! - **Path canonicalization.** We canonicalize the requested path before
//!   watching AND canonicalize before deduping into the payload, so the
//!   frontend can compare with strict equality against its own canonical
//!   path (`_fileBrowserPath`).

use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

/// Single-watcher state — held as Tauri-managed state.
pub struct FbWatcherState {
    /// The active watcher (dropped to stop). `None` when nothing is watched.
    watcher: Mutex<Option<RecommendedWatcher>>,
    /// The directory currently being watched, in canonical form. Compared
    /// against the frontend's `_fileBrowserPath` so the JS handler can
    /// ignore stale events from a previous folder.
    current_dir: Mutex<Option<String>>,
}

impl Default for FbWatcherState {
    fn default() -> Self {
        Self::new()
    }
}

impl FbWatcherState {
    pub fn new() -> Self {
        Self {
            watcher: Mutex::new(None),
            current_dir: Mutex::new(None),
        }
    }

    pub fn current(&self) -> Option<String> {
        self.current_dir.lock().unwrap().clone()
    }
}

/// Stop the active watcher (if any). Idempotent.
pub fn stop(state: &FbWatcherState) {
    *state.watcher.lock().unwrap() = None;
    *state.current_dir.lock().unwrap() = None;
}

/// Swap the active watcher to `dir`. Stops any previous watch first.
/// Returns the canonicalized path that was actually watched (frontend
/// compares with strict equality, so we hand back the canonical form).
pub fn watch(app: &AppHandle, state: &FbWatcherState, dir: String) -> Result<String, String> {
    // Stop any existing watch — `notify` watchers don't support hot-swap;
    // dropping is the cleanup path.
    stop(state);

    let path = PathBuf::from(&dir);
    if !path.exists() || !path.is_dir() {
        return Err(format!("not a directory: {dir}"));
    }
    let canonical = path
        .canonicalize()
        .map_err(|e| format!("canonicalize failed: {e}"))?;
    let canonical_str = canonical.to_string_lossy().to_string();

    let app_handle = app.clone();
    let dir_for_event = canonical_str.clone();

    // Debounce — one thread, lazily spawned when an event arrives.
    let last_event = Arc::new(Mutex::new(Instant::now()));
    let last_event_cb = last_event.clone();
    let debounce_active = Arc::new(AtomicBool::new(false));
    let debounce_active_cb = debounce_active.clone();

    let mut watcher = RecommendedWatcher::new(
        move |result: Result<Event, notify::Error>| {
            if result.is_err() {
                return;
            }
            *last_event_cb.lock().unwrap() = Instant::now();
            if debounce_active_cb.swap(true, Ordering::SeqCst) {
                return;
            }
            let app_ref = app_handle.clone();
            let last_ref = last_event_cb.clone();
            let active_ref = debounce_active_cb.clone();
            let dir_ref = dir_for_event.clone();
            std::thread::spawn(move || {
                loop {
                    std::thread::sleep(Duration::from_millis(300));
                    let last = *last_ref.lock().unwrap();
                    if last.elapsed() < Duration::from_millis(250) {
                        // More events arrived during the sleep — wait another cycle
                        continue;
                    }
                    let _ = app_ref.emit(
                        "file-browser-change",
                        serde_json::json!({"dir": dir_ref}),
                    );
                    active_ref.store(false, Ordering::SeqCst);
                    return;
                }
            });
        },
        // Poll fallback at 5 s — same cadence as `file_watcher.rs`. Native
        // backends (FSEvents on macOS, inotify on Linux) deliver realtime;
        // poll is only used when those are unavailable.
        Config::default().with_poll_interval(Duration::from_secs(5)),
    )
    .map_err(|e| format!("create watcher: {e}"))?;

    watcher
        .watch(&canonical, RecursiveMode::NonRecursive)
        .map_err(|e| format!("watch {}: {e}", canonical.display()))?;

    *state.watcher.lock().unwrap() = Some(watcher);
    *state.current_dir.lock().unwrap() = Some(canonical_str.clone());
    Ok(canonical_str)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_starts_empty() {
        let s = FbWatcherState::new();
        assert!(s.current().is_none());
        assert!(s.watcher.lock().unwrap().is_none());
    }

    #[test]
    fn test_stop_is_idempotent_on_empty_state() {
        let s = FbWatcherState::new();
        stop(&s);
        stop(&s);
        assert!(s.current().is_none());
    }
}
