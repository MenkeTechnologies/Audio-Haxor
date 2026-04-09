//! Plugin preset file scanner.
//!
//! Discovers preset files (FXP, FXB, VSTPRESET, AUPRESET, etc.) across
//! the user home directory (`~`, resolved via [`dirs::home_dir`]) plus
//! system-wide locations outside `~` (e.g. `/Library/Audio/Presets` on
//! macOS, `Program Files\\Common Files\\VST3 Presets` on Windows).
//! Supports parallel traversal and stop signaling.
//! Symlinks are followed (`metadata` on the link) so file and directory
//! targets are scanned; broken links are skipped.

use crate::history::PresetFile;
use crate::scanner_skip_dirs::SCANNER_SKIP_DIRS as SKIP_DIRS;
use crate::unified_walker::IncrementalDirState;
use rayon::prelude::*;
use dashmap::DashSet;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

fn normalize_macos_path(p: PathBuf) -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        let s = p.to_string_lossy();
        if s.starts_with("/System/Volumes/Data/") {
            return PathBuf::from(&s["/System/Volumes/Data".len()..]);
        }
    }
    p
}

pub(crate) const PRESET_EXTENSIONS: &[&str] = &[
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
                  // .mid / .midi deliberately excluded — MIDI has its own dedicated scanner
                  // (midi_scanner.rs) and lives in the midi_files table. Including them here
                  // would double-count MIDI files into both presets and midi_files tables.
];

fn format_size(bytes: u64) -> String {
    crate::format_size(bytes)
}

/// `Path::extension()` lowercased, no dot — used by the file watcher to match [`PRESET_EXTENSIONS`].
#[inline]
pub(crate) fn is_preset_extension_lowercase(ext: &str) -> bool {
    PRESET_EXTENSIONS
        .iter()
        .any(|e| e.strip_prefix('.') == Some(ext))
}

pub fn get_preset_roots() -> Vec<PathBuf> {
    let home = dirs::home_dir().unwrap_or_default();
    let mut roots = Vec::new();

    if !home.as_os_str().is_empty() {
        roots.push(home.clone());
    }

    #[cfg(target_os = "macos")]
    {
        roots.push(PathBuf::from("/Library/Audio/Presets"));
    }

    #[cfg(target_os = "windows")]
    {
        if let Ok(pf) = std::env::var("ProgramFiles") {
            roots.push(PathBuf::from(&pf).join("Common Files").join("VST3 Presets"));
        }
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
    active_dirs: Option<Arc<Mutex<Vec<String>>>>,
    incremental: Option<Arc<IncrementalDirState>>,
) {
    let batch_size = 100;
    let stop = Arc::new(AtomicBool::new(false));
    let found = Arc::new(AtomicUsize::new(0));
    let active = active_dirs.unwrap_or_else(|| Arc::new(Mutex::new(Vec::new())));
    let (tx, rx) = std::sync::mpsc::sync_channel::<Vec<PresetFile>>(256);
    let visited = Arc::new(DashSet::new());
    let exclude = Arc::new(exclude.unwrap_or_default());

    let roots_owned: Vec<PathBuf> = roots.to_vec();
    let stop2 = stop.clone();
    let found2 = found.clone();
    let incremental = incremental.clone();
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(num_cpus::get().max(4))
        .build()
        .unwrap();
    std::thread::spawn(move || {
        pool.install(|| {
            roots_owned.par_iter().for_each(|root| {
                if stop2.load(Ordering::Relaxed) {
                    return;
                }
                walk_dir_parallel(
                    root,
                    0,
                    &visited,
                    &tx,
                    &found2,
                    batch_size,
                    &stop2,
                    &exclude,
                    &active,
                    incremental.clone(),
                );
            });
        });
        drop(pool);
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
    visited: &Arc<DashSet<PathBuf>>,
    tx: &std::sync::mpsc::SyncSender<Vec<PresetFile>>,
    found: &Arc<AtomicUsize>,
    batch_size: usize,
    stop: &Arc<AtomicBool>,
    exclude: &Arc<HashSet<String>>,
    active_dirs: &Arc<Mutex<Vec<String>>>,
    incremental: Option<Arc<IncrementalDirState>>,
) {
    if depth > 30 || stop.load(Ordering::Relaxed) {
        return;
    }

    {
        let orig = normalize_macos_path(dir.to_path_buf());
        let canon = fs::canonicalize(dir).ok().map(normalize_macos_path);
        let key = canon.unwrap_or_else(|| orig.clone());
        if !visited.insert(key) {
            return;
        }
        visited.insert(orig);
    }

    if let Some(ref inc) = incremental {
        if inc.should_skip(dir) {
            return;
        }
    }

    let dir_str = dir.to_string_lossy().to_string();
    {
        let mut ad = active_dirs.lock().unwrap_or_else(|e| e.into_inner());
        ad.push(dir_str.clone());
        if ad.len() > 200 {
            let excess = ad.len() - 200;
            ad.drain(..excess);
        }
    }

    let entries: Vec<_> = match fs::read_dir(dir) {
        Ok(e) => e.flatten().collect(),
        Err(_e) => {
            return;
        }
    };

    let mut files = Vec::new();
    let mut subdirs = Vec::new();

    for entry in &entries {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        // `@` prefix = Synology NAS system dirs (@eaDir, @tmp, @syno*, etc.).
        if name_str.starts_with('.')
            || name_str.starts_with('@')
            || SKIP_DIRS.contains(&name_str.as_ref())
            || exclude.contains(name_str.as_ref())
        {
            continue;
        }
        // Cached d_type from readdir — no extra stat() syscall per entry.
        let ft = match entry.file_type() {
            Ok(f) => f,
            Err(_) => continue,
        };
        let path = entry.path();
        if ft.is_dir() {
            subdirs.push(path);
        } else if ft.is_file() {
            files.push((path, dir.to_path_buf()));
        } else if ft.is_symlink() {
            match fs::metadata(&path) {
                Ok(m) if m.is_dir() => {
                    subdirs.push(path);
                }
                Ok(m) if m.is_file() => {
                    files.push((path, dir.to_path_buf()));
                }
                _ => {}
            }
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
            active_dirs,
            incremental.clone(),
        );
    });

    if let Some(ref inc) = incremental {
        inc.record_scanned_dir(dir);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::slice::from_ref;

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
    fn test_preset_extensions_excludes_midi() {
        // MIDI files belong to the dedicated midi_scanner/midi_files table —
        // listing them here would double-count them into both tables.
        for ext in &[".mid", ".midi"] {
            assert!(
                !PRESET_EXTENSIONS.contains(ext),
                "PRESET_EXTENSIONS must NOT list MIDI {} — use midi_scanner instead",
                ext
            );
        }
    }

    #[test]
    fn test_preset_extensions_includes_ableton_kontakt_extras() {
        for ext in &[".adv", ".adg", ".nksn", ".syx", ".pjunoxl"] {
            assert!(
                PRESET_EXTENSIONS.contains(ext),
                "PRESET_EXTENSIONS should contain {}",
                ext
            );
        }
    }

    #[test]
    fn test_normalize_macos_path() {
        let p = PathBuf::from("/System/Volumes/Data/Users/example");
        let n = normalize_macos_path(p);
        #[cfg(target_os = "macos")]
        assert_eq!(n, PathBuf::from("/Users/example"));
        #[cfg(not(target_os = "macos"))]
        assert_eq!(n, PathBuf::from("/System/Volumes/Data/Users/example"));
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
            from_ref(&tmp),
            &mut |batch, _count| {
                found.extend_from_slice(batch);
            },
            &|| false,
            None,
            None,
            None,
        );
        assert!(found.is_empty());
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_walk_for_presets_finds_tfx_and_h2p() {
        let tmp = std::env::temp_dir().join("upum_test_preset_tfx_h2p");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        fs::write(tmp.join("tone.tfx"), b"tone2").unwrap();
        fs::write(tmp.join("diva.h2p"), b"u-he").unwrap();

        let mut found = Vec::new();
        walk_for_presets(
            from_ref(&tmp),
            &mut |batch, _count| found.extend_from_slice(batch),
            &|| false,
            None,
            None,
            None,
        );
        let formats: Vec<&str> = found.iter().map(|p| p.format.as_str()).collect();
        assert!(formats.contains(&"TFX"), "expected TFX, got {:?}", formats);
        assert!(formats.contains(&"H2P"), "expected H2P, got {:?}", formats);
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_walk_for_presets_finds_nksn_kontakt_snapshot() {
        let tmp = std::env::temp_dir().join("upum_test_preset_nksn");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        fs::write(tmp.join("snap.nksn"), b"kontakt").unwrap();

        let mut found = Vec::new();
        walk_for_presets(
            from_ref(&tmp),
            &mut |batch, _count| found.extend_from_slice(batch),
            &|| false,
            None,
            None,
            None,
        );
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].format, "NKSN");
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
            from_ref(&tmp),
            &mut |batch, _count| {
                found.extend_from_slice(batch);
            },
            &|| false,
            None,
            None,
            None,
        );
        assert_eq!(found.len(), 3);
        let formats: Vec<&str> = found.iter().map(|p| p.format.as_str()).collect();
        assert!(formats.contains(&"FXP"));
        assert!(formats.contains(&"FXB"));
        assert!(formats.contains(&"VSTPRESET"));
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_walk_for_presets_stop_signal() {
        let tmp = std::env::temp_dir().join("upum_test_preset_stop");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        for i in 0..20 {
            fs::write(tmp.join(format!("preset{}.fxp", i)), b"fake").unwrap();
        }

        let counter = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let c2 = counter.clone();
        let stop_after = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let s2 = stop_after.clone();

        walk_for_presets(
            from_ref(&tmp),
            &mut |batch, _count| {
                c2.fetch_add(batch.len(), std::sync::atomic::Ordering::Relaxed);
                s2.store(true, std::sync::atomic::Ordering::Relaxed);
            },
            &|| stop_after.load(std::sync::atomic::Ordering::Relaxed),
            None,
            None,
            None,
        );
        // Should have stopped — may have found some but scan should terminate
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_walk_for_presets_exclude_set() {
        let tmp = std::env::temp_dir().join("upum_test_preset_exclude");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        let included = tmp.join("keep.fxp");
        let excluded = tmp.join("skip.fxp");
        fs::write(&included, b"keep").unwrap();
        fs::write(&excluded, b"skip").unwrap();

        let mut exclude = HashSet::new();
        exclude.insert(excluded.to_string_lossy().to_string());

        let mut found = Vec::new();
        walk_for_presets(
            from_ref(&tmp),
            &mut |batch, _count| {
                found.extend_from_slice(batch);
            },
            &|| false,
            Some(exclude),
            None,
            None,
        );
        assert_eq!(found.len(), 1);
        assert!(found[0].path.contains("keep.fxp"));
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_walk_for_presets_skips_hidden_and_blacklisted_dirs() {
        let tmp = std::env::temp_dir().join("upum_test_preset_skip");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(tmp.join(".hidden_dir")).unwrap();
        fs::create_dir_all(tmp.join("node_modules")).unwrap();
        fs::create_dir_all(tmp.join("normal")).unwrap();
        fs::write(tmp.join(".hidden_dir/a.fxp"), b"h").unwrap();
        fs::write(tmp.join("node_modules/b.fxp"), b"n").unwrap();
        fs::write(tmp.join("normal/c.fxp"), b"ok").unwrap();

        let mut found = Vec::new();
        walk_for_presets(
            from_ref(&tmp),
            &mut |batch, _count| found.extend_from_slice(batch),
            &|| false,
            None,
            None,
            None,
        );
        assert_eq!(found.len(), 1);
        assert!(found[0].path.contains("normal"));
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_walk_for_presets_deduplicates_symlinks() {
        let tmp = std::env::temp_dir().join("upum_test_preset_symlink");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(tmp.join("real")).unwrap();
        fs::write(tmp.join("real/a.fxp"), b"preset").unwrap();

        #[cfg(unix)]
        {
            let _ = std::os::unix::fs::symlink(tmp.join("real"), tmp.join("link"));
            let mut found = Vec::new();
            walk_for_presets(
                &[tmp.join("real"), tmp.join("link")],
                &mut |batch, _count| found.extend_from_slice(batch),
                &|| false,
                None,
                None,
                None,
            );
            assert_eq!(found.len(), 1, "Symlinked duplicate should be deduped");
        }
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_walk_for_presets_deduplicates_overlapping_roots() {
        let tmp = std::env::temp_dir().join("upum_test_preset_overlap");
        let _ = fs::remove_dir_all(&tmp);
        let child = tmp.join("sub");
        fs::create_dir_all(&child).unwrap();
        fs::write(child.join("overlap.fxp"), b"preset").unwrap();
        fs::write(tmp.join("top.fxp"), b"preset").unwrap();

        let mut found = Vec::new();
        walk_for_presets(
            &[tmp.clone(), child.clone()],
            &mut |batch, _| found.extend_from_slice(batch),
            &|| false,
            None,
            None,
            None,
        );
        let overlap_count = found.iter().filter(|p| p.name == "overlap").count();
        assert_eq!(
            overlap_count, 1,
            "overlap.fxp found {} times",
            overlap_count
        );
        assert!(found.iter().any(|p| p.name == "top"));
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_walk_for_presets_consistent_counts() {
        let tmp = std::env::temp_dir().join("upum_test_preset_consistent");
        let _ = fs::remove_dir_all(&tmp);
        for i in 0..5 {
            let d = tmp.join(format!("dir{}", i));
            fs::create_dir_all(&d).unwrap();
            fs::write(d.join(format!("p{}.fxp", i)), b"preset").unwrap();
        }
        let mut c1 = 0;
        walk_for_presets(
            &[tmp.clone()],
            &mut |b, _| c1 += b.len(),
            &|| false,
            None,
            None,
            None,
        );
        let mut c2 = 0;
        walk_for_presets(
            &[tmp.clone()],
            &mut |b, _| c2 += b.len(),
            &|| false,
            None,
            None,
            None,
        );
        assert_eq!(c1, c2, "two scans should match: {} vs {}", c1, c2);
        assert_eq!(c1, 5);
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_walk_for_presets_batching() {
        let tmp = std::env::temp_dir().join("upum_test_preset_batch");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        for i in 0..5 {
            fs::write(tmp.join(format!("p{}.fxp", i)), b"fake").unwrap();
        }

        let mut total = 0usize;
        walk_for_presets(
            from_ref(&tmp),
            &mut |batch, count| {
                assert!(!batch.is_empty());
                total = count;
            },
            &|| false,
            None,
            None,
            None,
        );
        assert_eq!(total, 5);
        let _ = fs::remove_dir_all(&tmp);
    }
}
