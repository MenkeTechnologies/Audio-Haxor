//! PDF file scanner.
//!
//! Discovers PDF files across user document directories. Supports parallel
//! traversal and stop signaling (mirrors preset_scanner.rs structure).

use crate::history::PdfFile;
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

const PDF_EXTENSION: &str = ".pdf";

fn format_size(bytes: u64) -> String {
    crate::format_size(bytes)
}

/// Default PDF scan roots: user home only (`~`). Add more via Settings → PDF scan directories.
pub fn get_pdf_roots() -> Vec<PathBuf> {
    let home = dirs::home_dir().unwrap_or_default();
    if home.as_os_str().is_empty() || !home.exists() {
        return Vec::new();
    }
    vec![home]
}

pub fn walk_for_pdfs(
    roots: &[PathBuf],
    on_batch: &mut dyn FnMut(&[PdfFile], usize),
    should_stop: &(dyn Fn() -> bool + Sync),
    exclude: Option<HashSet<String>>,
    active_dirs: Option<Arc<Mutex<Vec<String>>>>,
    incremental: Option<Arc<IncrementalDirState>>,
) {
    let batch_size = 100;
    let stop = Arc::new(AtomicBool::new(false));
    let found = Arc::new(AtomicUsize::new(0));
    let active = active_dirs.unwrap_or_else(|| Arc::new(Mutex::new(Vec::new())));
    let (tx, rx) = std::sync::mpsc::sync_channel::<Vec<PdfFile>>(256);
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
            Ok(pdfs) => {
                total_found += pdfs.len();
                on_batch(&pdfs, total_found);
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
    tx: &std::sync::mpsc::SyncSender<Vec<PdfFile>>,
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
        let mut vis = visited.lock().unwrap_or_else(|e| e.into_inner());
        let orig = normalize_macos_path(dir.to_path_buf());
        let canon = fs::canonicalize(dir).ok().map(normalize_macos_path);
        let key = canon.unwrap_or_else(|| orig.clone());
        if !vis.insert(key) {
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

        if ext == PDF_EXTENSION {
            let path_str = path.to_string_lossy().to_string();
            if exclude.contains(&path_str) {
                continue;
            }
            if let Ok(meta) = fs::metadata(&path) {
                let pdf_name = path
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

                batch.push(PdfFile {
                    name: pdf_name,
                    path: path_str,
                    directory: parent.to_string_lossy().to_string(),
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

    if let Some(ref inc) = incremental {
        inc.record_scanned_dir(dir);
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::slice::from_ref;

    #[test]
    fn test_pdf_extension_constant() {
        assert_eq!(PDF_EXTENSION, ".pdf");
    }

    #[test]
    fn test_get_pdf_roots_returns_existing_paths() {
        let roots = get_pdf_roots();
        for r in &roots {
            assert!(r.exists(), "returned root should exist: {:?}", r);
        }
    }

    #[test]
    fn test_walk_for_pdfs_empty_dir() {
        let tmp = std::env::temp_dir().join("upum_test_pdf_empty");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        let mut found = Vec::new();
        walk_for_pdfs(
            from_ref(&tmp),
            &mut |batch, _| found.extend_from_slice(batch),
            &|| false,
            None,
            None,
            None,
        );
        assert!(found.is_empty());
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_walk_for_pdfs_finds_files() {
        let tmp = std::env::temp_dir().join("upum_test_pdf_find");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        fs::write(tmp.join("manual.pdf"), b"%PDF-1.4").unwrap();
        fs::write(tmp.join("book.PDF"), b"%PDF-1.4").unwrap();
        fs::write(tmp.join("notes.txt"), b"nope").unwrap();

        let mut found = Vec::new();
        walk_for_pdfs(
            from_ref(&tmp),
            &mut |batch, _| found.extend_from_slice(batch),
            &|| false,
            None,
            None,
            None,
        );
        assert_eq!(found.len(), 2);
        assert!(found.iter().any(|p| p.name == "manual"));
        assert!(found.iter().any(|p| p.name == "book"));
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_walk_for_pdfs_skips_hidden_and_blacklisted() {
        let tmp = std::env::temp_dir().join("upum_test_pdf_skip");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(tmp.join(".hidden")).unwrap();
        fs::create_dir_all(tmp.join("node_modules")).unwrap();
        fs::create_dir_all(tmp.join("ok")).unwrap();
        fs::write(tmp.join(".hidden/a.pdf"), b"h").unwrap();
        fs::write(tmp.join("node_modules/b.pdf"), b"n").unwrap();
        fs::write(tmp.join("ok/c.pdf"), b"ok").unwrap();

        let mut found = Vec::new();
        walk_for_pdfs(
            from_ref(&tmp),
            &mut |batch, _| found.extend_from_slice(batch),
            &|| false,
            None,
            None,
            None,
        );
        assert_eq!(found.len(), 1);
        assert!(found[0].path.contains("/ok/"));
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_walk_for_pdfs_exclude_set() {
        let tmp = std::env::temp_dir().join("upum_test_pdf_exclude");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        fs::write(tmp.join("keep.pdf"), b"x").unwrap();
        let skip = tmp.join("skip.pdf");
        fs::write(&skip, b"x").unwrap();

        let mut exclude = HashSet::new();
        exclude.insert(skip.to_string_lossy().to_string());

        let mut found = Vec::new();
        walk_for_pdfs(
            from_ref(&tmp),
            &mut |batch, _| found.extend_from_slice(batch),
            &|| false,
            Some(exclude),
            None,
            None,
        );
        assert_eq!(found.len(), 1);
        assert!(found[0].path.ends_with("keep.pdf"));
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_walk_for_pdfs_deduplicates_overlapping_roots() {
        let tmp = std::env::temp_dir().join("upum_test_pdf_overlap");
        let _ = fs::remove_dir_all(&tmp);
        let child = tmp.join("sub");
        fs::create_dir_all(&child).unwrap();
        fs::write(child.join("overlap.pdf"), b"x").unwrap();
        fs::write(tmp.join("top.pdf"), b"x").unwrap();

        let mut found = Vec::new();
        walk_for_pdfs(
            &[tmp.clone(), child.clone()],
            &mut |batch, _| found.extend_from_slice(batch),
            &|| false,
            None,
            None,
            None,
        );
        let overlap = found.iter().filter(|p| p.name == "overlap").count();
        assert_eq!(overlap, 1);
        assert!(found.iter().any(|p| p.name == "top"));
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_walk_for_pdfs_consistent_counts() {
        let tmp = std::env::temp_dir().join("upum_test_pdf_consistent");
        let _ = fs::remove_dir_all(&tmp);
        for i in 0..5 {
            let d = tmp.join(format!("d{i}"));
            fs::create_dir_all(&d).unwrap();
            fs::write(d.join(format!("p{i}.pdf")), b"x").unwrap();
        }
        let mut a = 0;
        walk_for_pdfs(
            &[tmp.clone()],
            &mut |b, _| a += b.len(),
            &|| false,
            None,
            None,
            None,
        );
        let mut b = 0;
        walk_for_pdfs(
            &[tmp.clone()],
            &mut |b2, _| b += b2.len(),
            &|| false,
            None,
            None,
            None,
        );
        assert_eq!(a, b);
        assert_eq!(a, 5);
        let _ = fs::remove_dir_all(&tmp);
    }
}
