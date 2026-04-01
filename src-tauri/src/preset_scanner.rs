//! Plugin preset file scanner.
//!
//! Discovers preset files (FXP, FXB, VSTPRESET, AUPRESET, etc.) across
//! platform-specific preset directories. Supports parallel traversal
//! and stop signaling.

use crate::history::PresetFile;
use rayon::prelude::*;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

const PRESET_EXTENSIONS: &[&str] = &[
    ".fxp",       // VST2 preset
    ".fxb",       // VST2 bank
    ".vstpreset", // VST3 preset
    ".aupreset",  // Audio Unit preset
    ".adv",       // Ableton device preset
    ".adg",       // Ableton rack preset
    ".nki",       // Kontakt instrument
    ".nksn",      // Kontakt snapshot
    ".h2p",       // u-he preset
    ".syx",       // MIDI SysEx dump
    ".tfx",       // Tone2 preset
    ".pjunoxl",   // TAL preset
];

const SKIP_DIRS: &[&str] = &[
    "node_modules",
    ".git",
    ".Trash",
    "$RECYCLE.BIN",
    "System Volume Information",
    ".cache",
    "__pycache__",
];

fn format_size(bytes: u64) -> String {
    if bytes == 0 {
        return "0 B".into();
    }
    let units = ["B", "KB", "MB", "GB"];
    let i = (bytes as f64).log(1024.0).floor() as usize;
    let i = i.min(units.len() - 1);
    format!("{:.1} {}", bytes as f64 / 1024f64.powi(i as i32), units[i])
}

pub fn get_preset_roots() -> Vec<PathBuf> {
    let home = dirs::home_dir().unwrap_or_default();
    let mut roots = Vec::new();

    #[cfg(target_os = "macos")]
    {
        roots.push(home.join("Library/Audio/Presets"));
        roots.push(PathBuf::from("/Library/Audio/Presets"));
        roots.push(home.join("Music"));
        roots.push(home.join("Documents"));
    }

    #[cfg(target_os = "windows")]
    {
        roots.push(home.join("Documents"));
        if let Ok(pf) = std::env::var("ProgramFiles") {
            roots.push(PathBuf::from(&pf).join("Common Files").join("VST3 Presets"));
        }
        if let Ok(appdata) = std::env::var("APPDATA") {
            roots.push(PathBuf::from(&appdata));
        }
        if let Ok(local) = std::env::var("LOCALAPPDATA") {
            roots.push(PathBuf::from(&local));
        }
    }

    #[cfg(target_os = "linux")]
    {
        roots.push(home.join(".vst3/presets"));
        roots.push(home.join(".local/share"));
        roots.push(home.join("Documents"));
    }

    roots.sort();
    roots.dedup();
    roots.into_iter().filter(|r| r.exists()).collect()
}

pub fn walk_for_presets(
    roots: &[PathBuf],
    on_batch: &mut dyn FnMut(&[PresetFile], usize),
    should_stop: &(dyn Fn() -> bool + Sync),
    exclude: Option<HashSet<String>>,
) {
    let batch_size = 100;
    let stop = Arc::new(AtomicBool::new(false));
    let found = Arc::new(AtomicUsize::new(0));
    let (tx, rx) = std::sync::mpsc::sync_channel::<Vec<PresetFile>>(512);
    let visited = Arc::new(Mutex::new(HashSet::new()));
    let exclude = Arc::new(exclude.unwrap_or_default());

    let roots_owned: Vec<PathBuf> = roots.to_vec();
    let stop2 = stop.clone();
    let found2 = found.clone();
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(num_cpus::get().max(2))
        .build()
        .unwrap();
    std::thread::spawn(move || {
        pool.install(|| {
            roots_owned.par_iter().for_each(|root| {
                if stop2.load(Ordering::Relaxed) {
                    return;
                }
                walk_dir_parallel(
                    root, 0, &visited, &tx, &found2, batch_size, &stop2, &exclude,
                );
            });
        });
    });

    let mut total_found = 0usize;
    loop {
        if should_stop() {
            stop.store(true, Ordering::Relaxed);
            while rx.try_recv().is_ok() {}
            break;
        }
        match rx.recv_timeout(std::time::Duration::from_millis(10)) {
            Ok(presets) => {
                total_found += presets.len();
                on_batch(&presets, total_found);
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => continue,
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn walk_dir_parallel(
    dir: &Path,
    depth: u32,
    visited: &Arc<Mutex<HashSet<PathBuf>>>,
    tx: &std::sync::mpsc::SyncSender<Vec<PresetFile>>,
    found: &Arc<AtomicUsize>,
    batch_size: usize,
    stop: &Arc<AtomicBool>,
    exclude: &Arc<HashSet<String>>,
) {
    if depth > 30 || stop.load(Ordering::Relaxed) {
        return;
    }

    let real_dir = match fs::canonicalize(dir) {
        Ok(p) => p,
        Err(_) => return,
    };
    {
        let mut vis = visited.lock().unwrap();
        if !vis.insert(real_dir) {
            return;
        }
    }

    let entries: Vec<_> = match fs::read_dir(dir) {
        Ok(e) => e.flatten().collect(),
        Err(_) => return,
    };

    let mut files = Vec::new();
    let mut subdirs = Vec::new();

    for entry in &entries {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if name_str.starts_with('.') || SKIP_DIRS.contains(&name_str.as_ref()) {
            continue;
        }
        let path = entry.path();
        if path.is_dir() {
            subdirs.push(path);
        } else if path.is_file() {
            files.push((path, dir.to_path_buf()));
        }
    }

    let mut batch = Vec::new();
    for (path, parent) in files {
        let ext = path
            .extension()
            .map(|e| format!(".{}", e.to_string_lossy().to_lowercase()))
            .unwrap_or_default();

        if PRESET_EXTENSIONS.contains(&ext.as_str()) {
            let path_str = path.to_string_lossy().to_string();
            if exclude.contains(&path_str) {
                continue;
            }
            if let Ok(meta) = fs::metadata(&path) {
                let preset_name = path
                    .file_stem()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_default();
                let modified = meta
                    .modified()
                    .ok()
                    .map(|t| {
                        let dt: chrono::DateTime<chrono::Utc> = t.into();
                        dt.format("%Y-%m-%d").to_string()
                    })
                    .unwrap_or_default();

                batch.push(PresetFile {
                    name: preset_name,
                    path: path_str,
                    directory: parent.to_string_lossy().to_string(),
                    format: ext[1..].to_uppercase(),
                    size: meta.len(),
                    size_formatted: format_size(meta.len()),
                    modified,
                });
                found.fetch_add(1, Ordering::Relaxed);

                if batch.len() >= batch_size {
                    let _ = tx.send(batch);
                    batch = Vec::new();
                }
            }
        }
    }
    if !batch.is_empty() {
        let _ = tx.send(batch);
    }

    subdirs.par_iter().for_each(|subdir| {
        walk_dir_parallel(
            subdir,
            depth + 1,
            visited,
            tx,
            found,
            batch_size,
            stop,
            exclude,
        );
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preset_extensions_complete() {
        for ext in &[".fxp", ".fxb", ".vstpreset", ".aupreset"] {
            assert!(
                PRESET_EXTENSIONS.contains(ext),
                "PRESET_EXTENSIONS should contain {}",
                ext
            );
        }
    }

    #[test]
    fn test_get_preset_roots_not_empty() {
        let roots = get_preset_roots();
        assert!(!roots.is_empty());
    }

    #[test]
    fn test_preset_extensions_includes_common() {
        for ext in &[".fxp", ".fxb", ".vstpreset"] {
            assert!(
                PRESET_EXTENSIONS.contains(ext),
                "PRESET_EXTENSIONS must contain {}",
                ext
            );
        }
    }

    #[test]
    fn test_preset_roots_exist() {
        let roots = get_preset_roots();
        assert!(
            !roots.is_empty(),
            "At least one preset root directory should exist on this system"
        );
        for root in &roots {
            assert!(root.exists(), "Returned root should exist: {:?}", root);
        }
    }

    #[test]
    fn test_walk_for_presets_empty_dir() {
        let tmp = std::env::temp_dir().join("upum_test_preset_empty");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        let mut found = Vec::new();
        walk_for_presets(
            &[tmp.clone()],
            &mut |batch, _count| {
                found.extend_from_slice(batch);
            },
            &|| false,
            None,
        );
        assert!(found.is_empty());
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_walk_for_presets_finds_files() {
        let tmp = std::env::temp_dir().join("upum_test_preset_find");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        fs::write(tmp.join("lead.fxp"), b"fake preset").unwrap();
        fs::write(tmp.join("bank.fxb"), b"fake bank").unwrap();
        fs::write(tmp.join("pad.vstpreset"), b"fake vstpreset").unwrap();
        fs::write(tmp.join("not_a_preset.txt"), b"nope").unwrap();

        let mut found = Vec::new();
        walk_for_presets(
            &[tmp.clone()],
            &mut |batch, _count| {
                found.extend_from_slice(batch);
            },
            &|| false,
            None,
        );
        assert_eq!(found.len(), 3);
        let formats: Vec<&str> = found.iter().map(|p| p.format.as_str()).collect();
        assert!(formats.contains(&"FXP"));
        assert!(formats.contains(&"FXB"));
        assert!(formats.contains(&"VSTPRESET"));
        let _ = fs::remove_dir_all(&tmp);
    }
}
