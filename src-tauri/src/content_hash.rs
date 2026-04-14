//! Byte-level duplicate detection: SHA-256 over file contents.
//!
//! Groups paths by stored size first (from SQLite), then hashes only size buckets
//! with more than one path.

use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use rayon::prelude::*;
use tauri::{AppHandle, Emitter};

const READ_CHUNK: usize = 1024 * 1024;
/// Hash this many paths per Rayon batch so a stop request can land between chunks (same idea as BPM batches).
const HASH_CHUNK: usize = 256;

/// Hex-encoded SHA-256 of file bytes, or `None` if unreadable.
pub fn hash_file_sha256(path: &Path) -> Option<String> {
    let _guard = crate::BgIoGuard::new();
    let file = File::open(path).ok()?;
    let mut reader = BufReader::with_capacity(READ_CHUNK, file);
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; READ_CHUNK];
    loop {
        // Check yield between chunks so we can pause mid-hash for large files
        if crate::should_yield_for_playback() {
            drop(_guard);
            crate::yield_if_playback_active();
            return hash_file_sha256(path); // Restart after pause
        }
        let n = reader.read(&mut buf).ok()?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Some(format!("{:x}", hasher.finalize()))
}

#[derive(Debug, Clone, Serialize)]
pub struct ContentDupPath {
    pub path: String,
    /// Short domain tag: `plugins`, `audio`, `daw`, `presets`, `pdf`, `midi`.
    pub kind: String,
}

#[derive(Debug, Serialize)]
pub struct ContentDupGroup {
    pub hash_hex: String,
    pub size_bytes: u64,
    pub paths: Vec<ContentDupPath>,
}

#[derive(Debug, Serialize)]
pub struct ContentDupScanResult {
    pub groups: Vec<ContentDupGroup>,
    /// Successful SHA-256 reads (subset of same-size bucket paths).
    pub files_hashed: usize,
    /// Paths skipped (missing on disk or read error).
    pub skipped: usize,
    /// Entries with stored size 0 omitted from size bucketing (would otherwise merge the whole library).
    pub skipped_zero_stored_size: usize,
    /// Stopped early via `cancel_content_duplicate_scan` (after the current chunk).
    pub cancelled: bool,
    /// Count of same-size-collision paths to hash (progress denominator).
    pub candidates_total: usize,
}

/// `entries`: `(path, size_bytes, kind)` for the whole library.
/// When `cancel` is `Some`, loads `Ordering::Relaxed` between chunks; current chunk always finishes.
/// `hash_threads`: Rayon pool size for hashing (does not use the global scan pool).
pub fn find_byte_duplicate_groups(
    entries: Vec<(String, u64, String)>,
    progress: Option<(Arc<AppHandle>, usize)>,
    cancel: Option<&AtomicBool>,
    hash_threads: usize,
) -> ContentDupScanResult {
    let mut size_map: HashMap<u64, Vec<(String, String)>> = HashMap::new();
    let mut skipped_zero_stored_size = 0usize;
    for (path, sz, kind) in entries {
        if sz == 0 {
            skipped_zero_stored_size += 1;
            continue;
        }
        size_map.entry(sz).or_default().push((path, kind));
    }

    let mut to_hash: Vec<(String, String, u64)> = Vec::new();
    for (sz, paths) in size_map {
        if paths.len() < 2 {
            continue;
        }
        for (p, k) in paths {
            to_hash.push((p, k, sz));
        }
    }

    let candidates_total = to_hash.len();
    if candidates_total == 0 {
        return ContentDupScanResult {
            groups: vec![],
            files_hashed: 0,
            skipped: 0,
            skipped_zero_stored_size,
            cancelled: false,
            candidates_total: 0,
        };
    }

    let processed_ctr = AtomicUsize::new(0);
    let skipped_ctr = AtomicUsize::new(0);
    let mut by_hash: HashMap<String, Vec<(String, String, u64)>> = HashMap::new();
    let mut files_hashed: usize = 0;
    let mut cancelled = false;
    // Last `done` sent to the UI; emit only from this thread so `done` never decreases.
    let mut last_emitted_done: usize = 0;

    let threads = hash_threads.max(1);
    let pool = crate::build_low_priority_thread_pool(threads);

    for chunk in to_hash.chunks(HASH_CHUNK) {
        if cancel.is_some_and(|c| c.load(Ordering::Relaxed)) {
            cancelled = true;
            break;
        }
        let hashed_chunk: Vec<(String, String, u64, String)> = pool.install(|| {
            chunk
                .par_iter()
                .filter_map(|(path, kind, sz)| {
                    // Yield to audio playback — SMB shares can't prioritize I/O
                    crate::yield_if_playback_active();
                    let p = Path::new(path);
                    let h = match hash_file_sha256(p) {
                        Some(x) => x,
                        None => {
                            skipped_ctr.fetch_add(1, Ordering::Relaxed);
                            processed_ctr.fetch_add(1, Ordering::Relaxed);
                            return None;
                        }
                    };
                    processed_ctr.fetch_add(1, Ordering::Relaxed);
                    Some((path.clone(), kind.clone(), *sz, h))
                })
                .collect()
        });

        files_hashed += hashed_chunk.len();
        for (path, kind, sz, h) in hashed_chunk {
            by_hash.entry(h).or_default().push((path, kind, sz));
        }

        // Emit only here (never from Rayon workers): `done` is strictly increasing.
        if let Some((app, every)) = progress.as_ref() {
            if *every > 0 {
                let n = processed_ctr.load(Ordering::Relaxed);
                if n > last_emitted_done {
                    last_emitted_done = n;
                    let _ = app.emit(
                        "content-dup-progress",
                        serde_json::json!({ "done": n, "total": candidates_total }),
                    );
                }
            }
        }
    }

    let mut groups: Vec<ContentDupGroup> = by_hash
        .into_iter()
        .filter(|(_, paths)| paths.len() > 1)
        .map(|(hash_hex, mut paths)| {
            paths.sort_by(|a, b| a.0.cmp(&b.0));
            let size_bytes = paths[0].2;
            let paths = paths
                .into_iter()
                .map(|(path, kind, _)| ContentDupPath { path, kind })
                .collect();
            ContentDupGroup {
                hash_hex,
                size_bytes,
                paths,
            }
        })
        .collect();

    groups.sort_by(|a, b| a.hash_hex.cmp(&b.hash_hex));

    ContentDupScanResult {
        files_hashed,
        skipped: skipped_ctr.into_inner(),
        skipped_zero_stored_size,
        cancelled,
        candidates_total,
        groups,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicBool;

    fn test_dir(name: &str) -> std::path::PathBuf {
        let p =
            std::env::temp_dir().join(format!("ah_content_hash_{}_{}", std::process::id(), name));
        let _ = std::fs::create_dir_all(&p);
        p
    }

    #[test]
    fn identical_files_same_hash() {
        let dir = test_dir("same");
        let a = dir.join("a.bin");
        let b = dir.join("b.bin");
        std::fs::write(&a, b"hello").unwrap();
        std::fs::write(&b, b"hello").unwrap();
        assert_eq!(
            hash_file_sha256(&a),
            hash_file_sha256(&b),
            "same bytes => same SHA-256"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn find_groups_two_identical() {
        let dir = test_dir("dup");
        let a = dir.join("a.wav");
        let b = dir.join("b.wav");
        std::fs::write(&a, b"x").unwrap();
        std::fs::write(&b, b"x").unwrap();
        let entries = vec![
            (a.to_string_lossy().into_owned(), 1, "audio".into()),
            (b.to_string_lossy().into_owned(), 1, "audio".into()),
        ];
        let r = find_byte_duplicate_groups(entries, None, None, 4);
        assert_eq!(r.groups.len(), 1);
        assert_eq!(r.groups[0].paths.len(), 2);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn zero_stored_size_not_merged_into_one_bucket() {
        let entries = vec![
            ("/a/x".into(), 0, "audio".into()),
            ("/b/y".into(), 0, "audio".into()),
        ];
        let r = find_byte_duplicate_groups(entries, None, None, 4);
        assert!(r.groups.is_empty());
        assert_eq!(r.skipped_zero_stored_size, 2);
        assert_eq!(r.files_hashed, 0);
    }

    #[test]
    fn cancel_before_first_chunk_skips_hashing() {
        let dir = test_dir("cancel");
        let a = dir.join("a.wav");
        let b = dir.join("b.wav");
        std::fs::write(&a, b"x").unwrap();
        std::fs::write(&b, b"x").unwrap();
        let entries = vec![
            (a.to_string_lossy().into_owned(), 1, "audio".into()),
            (b.to_string_lossy().into_owned(), 1, "audio".into()),
        ];
        let cancel = AtomicBool::new(true);
        let r = find_byte_duplicate_groups(entries, None, Some(&cancel), 4);
        assert!(r.cancelled);
        assert_eq!(r.files_hashed, 0);
        assert!(r.groups.is_empty());
        assert_eq!(r.candidates_total, 2);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn hash_file_sha256_missing_file_returns_none() {
        assert!(hash_file_sha256(Path::new("/nonexistent/ah_sha256.bin")).is_none());
    }

    #[test]
    fn different_bytes_yield_different_hashes() {
        let dir = test_dir("diff");
        let a = dir.join("a.bin");
        let b = dir.join("b.bin");
        std::fs::write(&a, b"alpha").unwrap();
        std::fs::write(&b, b"beta").unwrap();
        let ha = hash_file_sha256(&a).expect("readable");
        let hb = hash_file_sha256(&b).expect("readable");
        assert_ne!(ha, hb);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn single_size_collision_path_is_not_hashed() {
        let entries = vec![("/only/one.wav".into(), 42, "audio".into())];
        let r = find_byte_duplicate_groups(entries, None, None, 2);
        assert!(r.groups.is_empty());
        assert_eq!(r.files_hashed, 0);
        assert_eq!(r.candidates_total, 0);
    }

    #[test]
    fn two_groups_and_paths_sorted_within_group() {
        let dir = test_dir("twogroups");
        let a1 = dir.join("z.bin");
        let a2 = dir.join("a.bin");
        let b1 = dir.join("m.bin");
        let b2 = dir.join("n.bin");
        std::fs::write(&a1, b"X").unwrap();
        std::fs::write(&a2, b"X").unwrap();
        std::fs::write(&b1, b"Y").unwrap();
        std::fs::write(&b2, b"Y").unwrap();
        let entries = vec![
            (a1.to_string_lossy().into_owned(), 1, "audio".into()),
            (a2.to_string_lossy().into_owned(), 1, "presets".into()),
            (b1.to_string_lossy().into_owned(), 2, "daw".into()),
            (b2.to_string_lossy().into_owned(), 2, "daw".into()),
        ];
        let r = find_byte_duplicate_groups(entries, None, None, 4);
        assert_eq!(r.groups.len(), 2);
        for g in &r.groups {
            assert_eq!(g.paths.len(), 2);
            let p0 = &g.paths[0].path;
            let p1 = &g.paths[1].path;
            assert!(
                p0 < p1,
                "paths should be sorted: {:?} before {:?}",
                p0,
                p1
            );
        }
        let mut hashes: Vec<_> = r.groups.iter().map(|g| g.hash_hex.as_str()).collect();
        hashes.sort();
        assert_eq!(hashes.len(), 2);
        assert!(hashes[0] < hashes[1], "group order by hash_hex");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn unreadable_path_in_bucket_counts_skipped_but_still_groups_readable_dupes() {
        let dir = test_dir("partial");
        let good_a = dir.join("ga.wav");
        let good_b = dir.join("gb.wav");
        std::fs::write(&good_a, b"same").unwrap();
        std::fs::write(&good_b, b"same").unwrap();
        let missing = dir.join("nope.wav");
        let entries = vec![
            (good_a.to_string_lossy().into_owned(), 9, "audio".into()),
            (good_b.to_string_lossy().into_owned(), 9, "audio".into()),
            (missing.to_string_lossy().into_owned(), 9, "audio".into()),
        ];
        let r = find_byte_duplicate_groups(entries, None, None, 2);
        assert_eq!(r.skipped, 1);
        assert_eq!(r.groups.len(), 1);
        assert_eq!(r.groups[0].paths.len(), 2);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
