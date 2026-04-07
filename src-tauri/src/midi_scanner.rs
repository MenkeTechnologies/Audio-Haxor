//! MIDI file scanner — dedicated walker independent of the preset scanner.
//!
//! Discovers `.mid` / `.midi` files across the user home directory (`~`,
//! resolved via [`dirs::home_dir`]) plus system-wide locations. Supports
//! parallel traversal and stop signaling.

use crate::history::MidiFile;
use crate::scanner_skip_dirs::SCANNER_SKIP_DIRS as SKIP_DIRS;
use crate::unified_walker::IncrementalDirState;
use rayon::prelude::*;
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

const MIDI_EXTENSIONS: &[&str] = &[".mid", ".midi"];

fn format_size(bytes: u64) -> String {
    crate::format_size(bytes)
}

pub fn get_midi_roots() -> Vec<PathBuf> {
    let home = dirs::home_dir().unwrap_or_default();
    let mut roots = Vec::new();

    if !home.as_os_str().is_empty() {
        roots.push(home.clone());
    }

    #[cfg(target_os = "macos")]
    {
        roots.push(PathBuf::from("/Library/Audio/Presets"));
    }

    roots.sort();
    roots.dedup();
    roots.into_iter().filter(|r| r.exists()).collect()
}

pub fn walk_for_midi(
    roots: &[PathBuf],
    on_batch: &mut dyn FnMut(&[MidiFile], usize),
    should_stop: &(dyn Fn() -> bool + Sync),
    exclude: Option<HashSet<String>>,
    active_dirs: Option<Arc<Mutex<Vec<String>>>>,
    incremental: Option<Arc<IncrementalDirState>>,
) {
    let batch_size = 100;
    let stop = Arc::new(AtomicBool::new(false));
    let found = Arc::new(AtomicUsize::new(0));
    let active = active_dirs.unwrap_or_else(|| Arc::new(Mutex::new(Vec::new())));
    let (tx, rx) = std::sync::mpsc::sync_channel::<Vec<MidiFile>>(256);
    let visited = Arc::new(Mutex::new(HashSet::new()));
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
                    None,
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
            Ok(midi_files) => {
                total_found += midi_files.len();
                on_batch(&midi_files, total_found);
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
    parent_dev: Option<u64>,
    visited: &Arc<Mutex<HashSet<PathBuf>>>,
    tx: &std::sync::mpsc::SyncSender<Vec<MidiFile>>,
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

    // Mount-point detection — on Unix, a dir whose st_dev differs from its
    // parent's sits on a different filesystem (network mount, external drive,
    // overlayfs, etc.). Log the boundary so the user can see which mounts
    // the walker actually entered.
    #[cfg(unix)]
    let current_dev: Option<u64> = {
        use std::os::unix::fs::MetadataExt;
        match fs::metadata(dir) {
            Ok(m) => {
                let d = m.dev();
                if let Some(pd) = parent_dev {
                    if pd != d {
                        crate::write_app_log_verbose(format!(
                            "SCAN MOUNT — midi | {} | parent_dev={} current_dev={}",
                            dir.display(),
                            pd,
                            d
                        ));
                    }
                }
                Some(d)
            }
            Err(_) => None,
        }
    };
    #[cfg(not(unix))]
    let current_dev: Option<u64> = None;
    let _ = parent_dev;

    // Canonicalize OUTSIDE the mutex — it's a syscall (network roundtrip on
    // SMB) and must not block other workers while in flight.
    {
        let orig = normalize_macos_path(dir.to_path_buf());
        let canon = fs::canonicalize(dir).ok().map(normalize_macos_path);
        let key = canon.clone().unwrap_or_else(|| orig.clone());
        let mut vis = visited.lock().unwrap_or_else(|e| e.into_inner());
        if !vis.insert(key.clone()) {
            // Dedup hit — already visited via another path. Log if this is
            // something the user might care about (network mounts, /mnt).
            let s = dir.to_string_lossy();
            if s.contains("/mnt/") || s.ends_with("/mnt") {
                crate::write_app_log_verbose(format!(
                    "SCAN DEDUP SKIP — midi | orig={} | canon={} | key={}",
                    orig.display(),
                    canon
                        .as_ref()
                        .map(|p| p.display().to_string())
                        .unwrap_or_else(|| "<canonicalize failed>".into()),
                    key.display(),
                ));
            }
            return;
        }
        vis.insert(orig);
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

    // Diagnostic: log when we enter /mnt/ paths (SMB mounts typically).
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
        }
    }

    let mut batch = Vec::new();
    for (path, parent) in files {
        let ext = path
            .extension()
            .map(|e| format!(".{}", e.to_string_lossy().to_lowercase()))
            .unwrap_or_default();

        if MIDI_EXTENSIONS.contains(&ext.as_str()) {
            let path_str = path.to_string_lossy().to_string();
            if exclude.contains(&path_str) {
                continue;
            }
            if let Ok(meta) = fs::metadata(&path) {
                let midi_name = path
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

                batch.push(MidiFile {
                    name: midi_name,
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
            current_dev,
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

    #[test]
    fn test_midi_extensions_complete() {
        assert!(MIDI_EXTENSIONS.contains(&".mid"));
        assert!(MIDI_EXTENSIONS.contains(&".midi"));
    }

    #[test]
    fn test_get_midi_roots_returns_existing_paths() {
        let roots = get_midi_roots();
        for r in &roots {
            assert!(r.exists(), "returned root {:?} should exist", r);
        }
    }

    fn test_dir(name: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!(
            "upum_midi_scan_{}_{}",
            name,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        p
    }

    fn touch(p: &Path, content: &[u8]) {
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(p, content).unwrap();
    }

    #[test]
    fn test_walk_for_midi_finds_only_midi_files() {
        let root = test_dir("finds_only_midi");
        touch(&root.join("song.mid"), b"MThd");
        touch(&root.join("another.MIDI"), b"MThd");
        touch(&root.join("preset.fxp"), b"nope");
        touch(&root.join("audio.wav"), b"RIFF");
        touch(&root.join("doc.pdf"), b"%PDF");

        let mut found_names: Vec<String> = Vec::new();
        walk_for_midi(
            &[root.clone()],
            &mut |batch, _total| {
                for m in batch {
                    found_names.push(m.name.clone());
                }
            },
            &|| false,
            None,
            None,
            None,
        );
        found_names.sort();
        assert_eq!(found_names, vec!["another".to_string(), "song".to_string()]);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn test_walk_for_midi_exclude_set() {
        let root = test_dir("exclude_set");
        touch(&root.join("keep.mid"), b"MThd");
        touch(&root.join("drop.mid"), b"MThd");
        let mut excl = HashSet::new();
        excl.insert(root.join("drop.mid").to_string_lossy().to_string());

        let mut names: Vec<String> = Vec::new();
        walk_for_midi(
            &[root.clone()],
            &mut |batch, _| {
                for m in batch {
                    names.push(m.name.clone());
                }
            },
            &|| false,
            Some(excl),
            None,
            None,
        );
        assert_eq!(names, vec!["keep".to_string()]);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn test_walk_for_midi_skips_hidden_and_node_modules() {
        let root = test_dir("skip_hidden");
        touch(&root.join("visible.mid"), b"MThd");
        touch(&root.join(".hidden.mid"), b"MThd");
        touch(&root.join("node_modules/dep.mid"), b"MThd");
        touch(&root.join("@eaDir/thumb.mid"), b"MThd");

        let mut names: Vec<String> = Vec::new();
        walk_for_midi(
            &[root.clone()],
            &mut |batch, _| {
                for m in batch {
                    names.push(m.name.clone());
                }
            },
            &|| false,
            None,
            None,
            None,
        );
        assert_eq!(names, vec!["visible".to_string()]);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn test_walk_for_midi_populates_metadata() {
        let root = test_dir("metadata");
        touch(&root.join("song.mid"), b"MThd\x00\x00\x00\x06");

        let mut files: Vec<MidiFile> = Vec::new();
        walk_for_midi(
            &[root.clone()],
            &mut |batch, _| files.extend_from_slice(batch),
            &|| false,
            None,
            None,
            None,
        );
        assert_eq!(files.len(), 1);
        let f = &files[0];
        assert_eq!(f.name, "song");
        assert_eq!(f.format, "MID");
        assert_eq!(f.size, 8);
        assert!(!f.modified.is_empty());
        let _ = fs::remove_dir_all(&root);
    }
}
