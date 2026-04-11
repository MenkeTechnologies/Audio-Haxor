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
use std::sync::atomic::{AtomicUsize, Ordering};

use rayon::prelude::*;
use tauri::{AppHandle, Emitter};

const READ_CHUNK: usize = 1024 * 1024;

/// Hex-encoded SHA-256 of file bytes, or `None` if unreadable.
pub fn hash_file_sha256(path: &Path) -> Option<String> {
    let file = File::open(path).ok()?;
    let mut reader = BufReader::with_capacity(READ_CHUNK, file);
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; READ_CHUNK];
    loop {
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
    /// Files that were hashed (only candidates in multi-path size buckets).
    pub files_hashed: usize,
    /// Paths skipped (missing on disk or read error).
    pub skipped: usize,
}

/// `entries`: `(path, size_bytes, kind)` for the whole library.
pub fn find_byte_duplicate_groups(
    entries: Vec<(String, u64, String)>,
    progress: Option<(Arc<AppHandle>, usize)>,
) -> ContentDupScanResult {
    let mut size_map: HashMap<u64, Vec<(String, String)>> = HashMap::new();
    for (path, sz, kind) in entries {
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

    let total = to_hash.len();
    if total == 0 {
        return ContentDupScanResult {
            groups: vec![],
            files_hashed: 0,
            skipped: 0,
        };
    }

    let done_ctr = AtomicUsize::new(0);
    let skipped_ctr = AtomicUsize::new(0);

    let hashed: Vec<(String, String, u64, String)> = to_hash
        .into_par_iter()
        .filter_map(|(path, kind, sz)| {
            let p = Path::new(&path);
            let h = match hash_file_sha256(p) {
                Some(x) => x,
                None => {
                    skipped_ctr.fetch_add(1, Ordering::Relaxed);
                    return None;
                }
            };
            if let Some((app, every)) = progress.as_ref() {
                let n = done_ctr.fetch_add(1, Ordering::Relaxed) + 1;
                if *every > 0 && (n % *every == 0 || n == total) {
                    let _ = app.emit(
                        "content-dup-progress",
                        serde_json::json!({ "done": n, "total": total }),
                    );
                }
            } else {
                done_ctr.fetch_add(1, Ordering::Relaxed);
            }
            Some((path, kind, sz, h))
        })
        .collect();

    let mut by_hash: HashMap<String, Vec<(String, String, u64)>> = HashMap::new();
    for (path, kind, sz, h) in hashed {
        by_hash.entry(h).or_default().push((path, kind, sz));
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
        files_hashed: total,
        skipped: skipped_ctr.into_inner(),
        groups,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let r = find_byte_duplicate_groups(entries, None);
        assert_eq!(r.groups.len(), 1);
        assert_eq!(r.groups[0].paths.len(), 2);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
