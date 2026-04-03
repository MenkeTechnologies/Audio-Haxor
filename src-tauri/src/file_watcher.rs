//! Filesystem watcher for auto-scanning new/changed audio files, DAW projects, and presets.
//!
//! Uses the `notify` crate (FSEvents on macOS, inotify on Linux) to watch
//! configured scan directories. When files matching known extensions are
//! created or modified, emits Tauri events so the frontend can trigger
//! incremental re-indexing.

use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashSet;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

/// Audio sample extensions (lowercase, no dot).
const AUDIO_EXTS: &[&str] = &[
    "wav", "mp3", "flac", "ogg", "aif", "aiff", "m4a", "wma", "opus", "ape",
];

/// DAW project extensions.
const DAW_EXTS: &[&str] = &[
    "als", "rpp", "flp", "cpr", "npr", "song", "dawproject", "bwproject",
    "logicx", "band", "ptx", "ptf", "reason",
];

/// Preset extensions.
const PRESET_EXTS: &[&str] = &[
    "fxp", "fxb", "vstpreset", "aupreset", "nmsv", "nkm", "nki", "adg",
    "adv", "agr", "als", "fst", "ksd", "pjunoxl", "bwpreset", "clap-preset",
    "tfx", "h2p", "tfx",
];

/// Plugin extensions.
const PLUGIN_EXTS: &[&str] = &["dll", "vst3", "component", "clap", "aaxplugin"];

/// Classify a file path into a change category.
fn classify(path: &Path) -> Option<&'static str> {
    let ext = path.extension()?.to_str()?.to_lowercase();
    if AUDIO_EXTS.contains(&ext.as_str()) {
        Some("audio")
    } else if DAW_EXTS.contains(&ext.as_str()) || path.is_dir() && ext == "logicx" {
        Some("daw")
    } else if PRESET_EXTS.contains(&ext.as_str()) {
        Some("preset")
    } else if PLUGIN_EXTS.contains(&ext.as_str()) {
        Some("plugin")
    } else {
        None
    }
}

/// State for the file watcher.
pub struct FileWatcherState {
    watcher: Mutex<Option<RecommendedWatcher>>,
    watching: AtomicBool,
    watched_dirs: Mutex<Vec<String>>,
}

impl FileWatcherState {
    pub fn new() -> Self {
        Self {
            watcher: Mutex::new(None),
            watching: AtomicBool::new(false),
            watched_dirs: Mutex::new(Vec::new()),
        }
    }
}

/// Start watching the given directories for file changes.
/// Debounces events and emits `file-watcher-change` to the frontend.
pub fn start_watching(
    app: &AppHandle,
    state: &FileWatcherState,
    dirs: Vec<String>,
) -> Result<(), String> {
    // Stop existing watcher first
    stop_watching(state);

    let app_handle = app.clone();

    // Debounce: collect changes for 2 seconds before emitting
    let pending: Arc<Mutex<HashSet<String>>> = Arc::new(Mutex::new(HashSet::new()));
    let pending_clone = pending.clone();
    let last_emit = Arc::new(Mutex::new(Instant::now()));
    let last_emit_clone = last_emit.clone();

    let mut watcher = RecommendedWatcher::new(
        move |result: Result<Event, notify::Error>| {
            let event = match result {
                Ok(e) => e,
                Err(_) => return,
            };

            // Only care about create/modify events
            match event.kind {
                EventKind::Create(_) | EventKind::Modify(_) => {}
                _ => return,
            }

            for path in &event.paths {
                if let Some(category) = classify(path) {
                    let mut p = pending_clone.lock().unwrap();
                    p.insert(category.to_string());
                }
            }

            // Debounce: emit after 2 seconds of quiet
            let mut last = last_emit_clone.lock().unwrap();
            *last = Instant::now();
            let pending_ref = pending_clone.clone();
            let app_ref = app_handle.clone();
            let last_ref = last_emit_clone.clone();

            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_secs(2));
                let last = last_ref.lock().unwrap();
                if last.elapsed() < Duration::from_millis(1900) {
                    return; // More events came in, skip
                }
                drop(last);

                let mut p = pending_ref.lock().unwrap();
                if p.is_empty() {
                    return;
                }
                let categories: Vec<String> = p.drain().collect();
                let _ = app_ref.emit("file-watcher-change", serde_json::json!({
                    "categories": categories,
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                }));
            });
        },
        Config::default()
            .with_poll_interval(Duration::from_secs(5)),
    )
    .map_err(|e| format!("Failed to create watcher: {e}"))?;

    // Watch each directory
    let mut watched = Vec::new();
    for dir in &dirs {
        let path = Path::new(dir);
        if path.exists() && path.is_dir() {
            if watcher.watch(path, RecursiveMode::Recursive).is_ok() {
                watched.push(dir.clone());
            }
        }
    }

    *state.watcher.lock().unwrap() = Some(watcher);
    *state.watched_dirs.lock().unwrap() = watched;
    state.watching.store(true, Ordering::SeqCst);

    Ok(())
}

/// Stop the file watcher.
pub fn stop_watching(state: &FileWatcherState) {
    let mut w = state.watcher.lock().unwrap();
    *w = None; // Dropping the watcher stops it
    state.watching.store(false, Ordering::SeqCst);
    state.watched_dirs.lock().unwrap().clear();
}

/// Check if the watcher is active.
pub fn is_watching(state: &FileWatcherState) -> bool {
    state.watching.load(Ordering::SeqCst)
}

/// Get the list of currently watched directories.
pub fn get_watched_dirs(state: &FileWatcherState) -> Vec<String> {
    state.watched_dirs.lock().unwrap().clone()
}
