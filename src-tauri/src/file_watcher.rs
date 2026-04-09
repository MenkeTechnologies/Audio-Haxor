//! Filesystem watcher for auto-scanning new/changed audio files, DAW projects, presets, plugins, PDFs, and MIDI.
//!
//! Uses the `notify` crate (FSEvents on macOS, inotify on Linux, ReadDirectoryChangesW on Windows) to watch
//! configured scan directories. On create/modify, classifies paths by extension, maps each path to a **scan root**
//! (parent dir for files; bundle dirs as-is), debounces 2s, collapses nested roots, then emits `file-watcher-change`
//! with `roots_by_category` so the UI runs each scanner **only on those subtrees**.

use crate::audio_extensions::is_audio_extension_lowercase;
use crate::daw_scanner::is_daw_extension_lowercase;
use crate::preset_scanner::is_preset_extension_lowercase;
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

/// Plugin extensions.
const PLUGIN_EXTS: &[&str] = &["dll", "vst3", "component", "clap", "aaxplugin"];

/// Directory to pass to scanners: bundle dirs (`.vst3`, `.logicx`, …) as-is; files use their parent folder.
fn scan_root_for_changed_path(path: &Path) -> Option<PathBuf> {
    if path.is_dir() {
        Some(path.to_path_buf())
    } else {
        path.parent().map(Path::to_path_buf)
    }
}

/// Drop redundant roots: if `/a` and `/a/b` both changed, keep only `/a`.
fn minimize_scan_roots(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    if paths.is_empty() {
        return Vec::new();
    }
    let mut paths: Vec<PathBuf> = paths.into_iter().collect();
    paths.sort_by_key(|p| p.components().count());
    let mut out: Vec<PathBuf> = Vec::new();
    for p in paths {
        if out.iter().any(|r| p.starts_with(r)) {
            continue;
        }
        out.push(p);
    }
    out
}

/// Classify a file path into a change category.
fn classify(path: &Path) -> Option<&'static str> {
    let ext = path.extension()?.to_str()?.to_lowercase();
    if is_audio_extension_lowercase(ext.as_str()) {
        Some("audio")
    } else if is_daw_extension_lowercase(ext.as_str()) {
        Some("daw")
    } else if is_preset_extension_lowercase(ext.as_str()) {
        Some("preset")
    } else if PLUGIN_EXTS.contains(&ext.as_str()) {
        Some("plugin")
    } else if ext == "pdf" {
        Some("pdf")
    } else if ext == "mid" || ext == "midi" {
        Some("midi")
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

impl Default for FileWatcherState {
    fn default() -> Self {
        Self::new()
    }
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

    // Debounce: collect per-category scan roots for 2 seconds before emitting
    let pending: Arc<Mutex<HashMap<String, HashSet<String>>>> =
        Arc::new(Mutex::new(HashMap::new()));
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
                let Some(category) = classify(path) else {
                    continue;
                };
                let Some(mut root) = scan_root_for_changed_path(path) else {
                    continue;
                };
                if let Ok(canonical) = root.canonicalize() {
                    root = canonical;
                }
                let mut p = pending_clone.lock().unwrap();
                p.entry(category.to_string())
                    .or_insert_with(HashSet::new)
                    .insert(root.to_string_lossy().to_string());
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

                let mut map = pending_ref.lock().unwrap();
                if map.is_empty() {
                    return;
                }
                let categories: Vec<String> = map.keys().cloned().collect();
                let mut roots_by_category = serde_json::Map::new();
                for (cat, path_strs) in map.drain() {
                    let paths: Vec<PathBuf> = path_strs.into_iter().map(PathBuf::from).collect();
                    let minimized = minimize_scan_roots(paths);
                    let arr: Vec<String> = minimized
                        .into_iter()
                        .map(|p| p.to_string_lossy().to_string())
                        .collect();
                    roots_by_category.insert(cat, serde_json::json!(arr));
                }
                let _ = app_ref.emit(
                    "file-watcher-change",
                    serde_json::json!({
                        "categories": categories,
                        "roots_by_category": roots_by_category,
                        "timestamp": chrono::Utc::now().to_rfc3339(),
                    }),
                );
            });
        },
        Config::default().with_poll_interval(Duration::from_secs(5)),
    )
    .map_err(|e| format!("Failed to create watcher: {e}"))?;

    // Watch each directory
    let mut watched = Vec::new();
    for dir in &dirs {
        let path = Path::new(dir);
        if path.exists() && path.is_dir() && watcher.watch(path, RecursiveMode::Recursive).is_ok() {
            watched.push(dir.clone());
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_classify_audio() {
        for ext in &[
            "wav",
            "mp3",
            "flac",
            "ogg",
            "aif",
            "aiff",
            "m4a",
            "wma",
            "opus",
            "aac",
            "rex",
            "rx2",
            "sf2",
            "sfz",
        ] {
            let name = format!("test.{ext}");
            assert_eq!(
                classify(Path::new(&name)),
                Some("audio"),
                "expected audio for .{ext}"
            );
        }
    }

    #[test]
    fn test_classify_daw() {
        for ext in &[
            "als",
            "logicx",
            "flp",
            "cpr",
            "npr",
            "bwproject",
            "rpp",
            "rpp-bak",
            "ptx",
            "ptf",
            "song",
            "reason",
            "aup",
            "aup3",
            "band",
            "ardour",
            "dawproject",
        ] {
            let name = format!("project.{ext}");
            assert_eq!(
                classify(Path::new(&name)),
                Some("daw"),
                "expected daw for .{ext}"
            );
        }
    }

    #[test]
    fn test_classify_preset() {
        for ext in &[
            "fxp",
            "fxb",
            "vstpreset",
            "aupreset",
            "adv",
            "adg",
            "nki",
            "nksn",
            "h2p",
            "syx",
            "tfx",
            "pjunoxl",
        ] {
            let name = format!("preset.{ext}");
            assert_eq!(
                classify(Path::new(&name)),
                Some("preset"),
                "expected preset for .{ext}"
            );
        }
    }

    #[test]
    fn test_classify_plugin() {
        for ext in &["dll", "vst3", "component", "clap", "aaxplugin"] {
            let name = format!("plugin.{ext}");
            assert_eq!(
                classify(Path::new(&name)),
                Some("plugin"),
                "expected plugin for .{ext}"
            );
        }
    }

    #[test]
    fn test_classify_vst2_bundle_ext_not_watched_as_plugin() {
        // Legacy `.vst` dirs are plugins but watcher only lists modern bundle extensions
        assert_eq!(classify(Path::new("LegacySynth.vst")), None);
    }

    #[test]
    fn test_classify_unknown_returns_none() {
        assert_eq!(classify(Path::new("readme.txt")), None);
        assert_eq!(classify(Path::new("photo.png")), None);
        assert_eq!(classify(Path::new("data.json")), None);
        assert_eq!(classify(Path::new("noext")), None);
    }

    #[test]
    fn test_classify_pdf_and_midi() {
        assert_eq!(classify(Path::new("manual.pdf")), Some("pdf"));
        assert_eq!(classify(Path::new("x.PDF")), Some("pdf"));
        assert_eq!(classify(Path::new("song.mid")), Some("midi"));
        assert_eq!(classify(Path::new("track.midi")), Some("midi"));
    }

    #[test]
    fn test_minimize_scan_roots_drops_nested() {
        let a = PathBuf::from("music");
        let b = PathBuf::from("music/sub");
        let out = minimize_scan_roots(vec![b, a.clone()]);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0], a);
    }

    #[test]
    fn test_minimize_scan_roots_keeps_siblings() {
        let a = PathBuf::from("a/x");
        let b = PathBuf::from("a/y");
        let out = minimize_scan_roots(vec![a.clone(), b.clone()]);
        assert_eq!(out.len(), 2);
        assert!(out.contains(&a));
        assert!(out.contains(&b));
    }

    #[test]
    fn test_scan_root_file_is_parent() {
        let p = Path::new("folder/track.wav");
        assert_eq!(
            scan_root_for_changed_path(p),
            Some(PathBuf::from("folder"))
        );
    }

    #[test]
    fn test_scan_root_dir_is_self() {
        let tmp = std::env::temp_dir().join("audio_haxor_fw_test_logicx");
        let _ = std::fs::remove_dir_all(&tmp);
        let bundle = tmp.join("Proj.logicx");
        std::fs::create_dir_all(&bundle).unwrap();
        assert!(bundle.is_dir());
        assert_eq!(scan_root_for_changed_path(&bundle), Some(bundle.clone()));
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_classify_archive_double_extension_is_last_segment() {
        // `extension()` is only the final segment — `.gz` is not audio
        assert_eq!(classify(Path::new("backup.tar.gz")), None);
    }

    #[test]
    fn test_classify_preset_nmsv_not_indexed() {
        assert_eq!(
            classify(Path::new("preset.nmsv")),
            None,
            ".nmsv is not in preset_scanner::PRESET_EXTENSIONS — watcher must not flag preset scans"
        );
    }

    #[test]
    fn test_classify_preset_clap_hyphen_not_indexed() {
        assert_eq!(
            classify(Path::new("Analog.clap-preset")),
            None,
            ".clap-preset is not in PRESET_EXTENSIONS"
        );
    }

    #[test]
    fn test_classify_audio_opus() {
        assert_eq!(classify(Path::new("track.opus")), Some("audio"));
    }

    #[test]
    fn test_classify_daw_bwproject() {
        assert_eq!(classify(Path::new("song.bwproject")), Some("daw"));
    }

    #[test]
    fn test_classify_daw_reaper_backup_rpp_bak() {
        assert_eq!(
            classify(Path::new("session.rpp-bak")),
            Some("daw"),
            "REAPER backups must match DAW scanner .rpp-bak"
        );
    }

    #[test]
    fn test_classify_preset_nkm_not_indexed() {
        assert_eq!(classify(Path::new("Bank.nkm")), None);
    }

    #[test]
    fn test_classify_preset_bwpreset_not_indexed() {
        assert_eq!(classify(Path::new("Analog.bwpreset")), None);
    }

    #[test]
    fn test_classify_preset_agr_not_indexed() {
        assert_eq!(classify(Path::new("Swing.agr")), None);
    }

    #[test]
    fn test_classify_case_insensitive() {
        assert_eq!(classify(Path::new("test.WAV")), Some("audio"));
        assert_eq!(classify(Path::new("test.Flp")), Some("daw"));
        assert_eq!(classify(Path::new("track.RPP")), Some("daw"));
        assert_eq!(classify(Path::new("test.FXP")), Some("preset"));
        assert_eq!(classify(Path::new("test.DLL")), Some("plugin"));
    }

    #[test]
    fn test_file_watcher_state_new() {
        let state = FileWatcherState::new();
        assert!(!state.watching.load(Ordering::SeqCst));
        assert!(state.watcher.lock().unwrap().is_none());
        assert!(state.watched_dirs.lock().unwrap().is_empty());
    }

    #[test]
    fn test_is_watching_default_false() {
        let state = FileWatcherState::new();
        assert!(!is_watching(&state));
    }

    #[test]
    fn test_get_watched_dirs_default_empty() {
        let state = FileWatcherState::new();
        assert!(get_watched_dirs(&state).is_empty());
    }

    #[test]
    fn test_stop_watching_noop_on_fresh_state() {
        let state = FileWatcherState::new();
        stop_watching(&state);
        assert!(!is_watching(&state));
        assert!(get_watched_dirs(&state).is_empty());
    }
}
