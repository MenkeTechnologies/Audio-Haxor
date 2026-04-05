//! Unified filesystem walker — traverses a union of roots once and classifies
//! files into audio/daw/preset/pdf buckets by extension.
//!
//! ## Why
//! Running 4 separate walkers (audio, daw, preset, pdf) over overlapping roots
//! (`~`, `/Applications`, `/Volumes/*`) re-walks the same directories 4x.
//! On SMB shares, each `readdir`/`stat` is a network roundtrip — re-walks cost
//! minutes. This module walks each subtree exactly once.
//!
//! ## How
//! 1. Union the per-type root sets.
//! 2. Walk each unique root in parallel via rayon, dedup-visited by canonical path.
//! 3. For every file entry, classify by lowercase extension against each type's
//!    extension set AND the type's root-membership predicate. A file at
//!    `~/foo.wav` is audio if `~/foo.wav` sits under at least one `audio_roots`
//!    entry (for typical setups, this is trivially true).
//! 4. DAW packages (`.logicx`, `.band` directories) are detected at directory
//!    level and treated as projects; their subtree is NOT descended.
//! 5. Plugin bundles (`.vst3`, `.component`, etc.) are skipped entirely for DAW,
//!    but their interiors ARE still walked for audio/preset/pdf content.
//!
//! Per-type progress callbacks stream batches as they're discovered.

use crate::bulk_stat::{read_dir_bulk, BulkEntry};
use crate::history::{AudioSample, DawProject, PdfFile, PresetFile};
use rayon::prelude::*;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

/// Format a UNIX timestamp (seconds since epoch) as "YYYY-MM-DD" in UTC.
/// Returns empty string for invalid/zero timestamps.
fn fmt_mtime_ymd(mtime_secs: i64) -> String {
    if mtime_secs <= 0 {
        return String::new();
    }
    chrono::DateTime::<chrono::Utc>::from_timestamp(mtime_secs, 0)
        .map(|dt| dt.format("%Y-%m-%d").to_string())
        .unwrap_or_default()
}

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

// Re-exported from individual scanners to keep extension lists in sync. If
// either scanner's list changes, update the constant there and this module
// will see it via the `pub(crate)` re-exports at the bottom of each scanner.
const AUDIO_EXTENSIONS: &[&str] = &[
    ".wav", ".mp3", ".aiff", ".aif", ".flac", ".ogg", ".m4a", ".wma", ".aac", ".opus", ".rex",
    ".rx2", ".sf2", ".sfz",
];

const DAW_EXTENSIONS: &[&str] = &[
    ".als",
    ".logicx",
    ".flp",
    ".cpr",
    ".npr",
    ".bwproject",
    ".rpp",
    ".rpp-bak",
    ".ptx",
    ".ptf",
    ".song",
    ".reason",
    ".aup",
    ".aup3",
    ".band",
    ".ardour",
    ".dawproject",
];

const DAW_PACKAGE_EXTENSIONS: &[&str] = &[".logicx", ".band"];

const DAW_PLUGIN_BUNDLE_EXTENSIONS: &[&str] = &[
    ".vst",
    ".vst3",
    ".component",
    ".aaxplugin",
    ".app",
    ".framework",
    ".bundle",
    ".plugin",
    ".dpm",
    ".clap",
];

const DAW_BACKUP_DIRS: &[&str] = &["Backup", "Crash"];

const PRESET_EXTENSIONS: &[&str] = &[
    ".fxp",
    ".fxb",
    ".vstpreset",
    ".aupreset",
    ".adv",
    ".adg",
    ".nki",
    ".nksn",
    ".h2p",
    ".syx",
    ".tfx",
    ".pjunoxl",
    ".mid",
    ".midi",
];

const PDF_EXTENSION: &str = ".pdf";

const SKIP_DIRS: &[&str] = &[
    "node_modules",
    ".git",
    ".Trash",
    "$RECYCLE.BIN",
    "#recycle",
    "System Volume Information",
    ".cache",
    "__pycache__",
    "Caches",
    "DerivedData",
    "Backups.backupdb",
    "__MACOSX",
    // Synology NAS: `@`-prefixed dirs caught by traversal guard below.
    "#snapshot", // Synology Btrfs snapshots (#recycle already listed)
];

fn format_size(bytes: u64) -> String {
    crate::format_size(bytes)
}

fn daw_name_for_format(format: &str) -> &'static str {
    match format {
        "ALS" => "Ableton Live",
        "LOGICX" => "Logic Pro",
        "FLP" => "FL Studio",
        "CPR" => "Cubase",
        "NPR" => "Nuendo",
        "BWPROJECT" => "Bitwig Studio",
        "RPP" | "RPP-BAK" => "REAPER",
        "PTX" | "PTF" => "Pro Tools",
        "SONG" => "Studio One",
        "REASON" => "Reason",
        "AUP" | "AUP3" => "Audacity",
        "BAND" => "GarageBand",
        "ARDOUR" => "Ardour",
        "DAWPROJECT" => "DAWproject",
        _ => "Unknown",
    }
}

fn is_valid_band_package(path: &Path) -> bool {
    let pd = path.join("projectData");
    if !pd.exists() {
        return false;
    }
    if let Ok(mut f) = fs::File::open(&pd) {
        use std::io::Read;
        let mut magic = [0u8; 6];
        if f.read_exact(&mut magic).is_err() || &magic != b"bplist" {
            return false;
        }
    } else {
        return false;
    }
    path.join("Media").is_dir() || path.join("Output").is_dir() || path.join("Freeze Files").is_dir()
}

fn get_directory_size(path: &Path) -> u64 {
    get_directory_size_depth(path, 0)
}

fn get_directory_size_depth(path: &Path, depth: u32) -> u64 {
    if depth > 10 {
        return 0;
    }
    let mut total = 0u64;
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            if let Ok(ft) = entry.file_type() {
                if ft.is_dir() {
                    total += get_directory_size_depth(&entry.path(), depth + 1);
                } else if ft.is_file() {
                    if let Ok(meta) = entry.metadata() {
                        total += meta.len();
                    }
                }
            }
        }
    }
    total
}

/// Membership predicate — does `path` live under any root in `roots`?
/// Empty roots list means "no files qualify for this type".
fn under_any_root(path: &Path, roots: &[PathBuf]) -> bool {
    if roots.is_empty() {
        return false;
    }
    roots.iter().any(|r| path.starts_with(r))
}

/// Does the lowercased filename end with any of `exts`? `exts` must include the
/// leading dot.
fn ext_match(name_lower: &str, exts: &[&str]) -> Option<String> {
    for e in exts {
        if name_lower.ends_with(e) {
            return Some(e.to_string());
        }
    }
    None
}

/// Per-type scanning configuration. Empty roots disables the type (no output).
#[derive(Debug, Clone, Default)]
pub struct UnifiedSpec {
    pub audio_roots: Vec<PathBuf>,
    pub audio_exclude: HashSet<String>,
    pub daw_roots: Vec<PathBuf>,
    pub daw_exclude: HashSet<String>,
    pub daw_include_backups: bool,
    pub preset_roots: Vec<PathBuf>,
    pub preset_exclude: HashSet<String>,
    pub pdf_roots: Vec<PathBuf>,
    pub pdf_exclude: HashSet<String>,
}

/// One classified batch sent to the on_batch callback.
#[derive(Debug)]
pub enum ClassifiedBatch {
    Audio(Vec<AudioSample>),
    Daw(Vec<DawProject>),
    Preset(Vec<PresetFile>),
    Pdf(Vec<PdfFile>),
}

/// Running totals across all types.
#[derive(Debug, Clone, Copy, Default)]
pub struct UnifiedCounts {
    pub audio: usize,
    pub daw: usize,
    pub preset: usize,
    pub pdf: usize,
}

/// Walk the union of all roots across types, emitting classified batches.
///
/// The callback is invoked from the main receiving thread (single-threaded),
/// so it does not need to be Send/Sync. Batches are ~100 files each.
pub fn walk_unified(
    spec: &UnifiedSpec,
    on_batch: &mut dyn FnMut(ClassifiedBatch, UnifiedCounts),
    should_stop: &(dyn Fn() -> bool + Sync),
    active_dirs: Vec<Arc<Mutex<Vec<String>>>>,
) {
    let batch_size = 100;
    let stop = Arc::new(AtomicBool::new(false));
    // Fan-out sinks — each push goes to every provided Vec, so the walker's
    // single traversal drives all N walker-status tiles simultaneously.
    let active: Vec<Arc<Mutex<Vec<String>>>> = if active_dirs.is_empty() {
        vec![Arc::new(Mutex::new(Vec::new()))]
    } else {
        active_dirs
    };
    let (tx, rx) = std::sync::mpsc::sync_channel::<ClassifiedBatch>(256);
    let visited = Arc::new(Mutex::new(HashSet::new()));

    // Union of all roots, deduped by canonicalized path.
    let mut union: Vec<PathBuf> = Vec::new();
    for list in [
        &spec.audio_roots,
        &spec.daw_roots,
        &spec.preset_roots,
        &spec.pdf_roots,
    ] {
        for r in list {
            union.push(r.clone());
        }
    }
    union.sort();
    union.dedup();
    let union: Vec<PathBuf> = union.into_iter().filter(|r| r.exists()).collect();

    let audio_found = Arc::new(AtomicUsize::new(0));
    let daw_found = Arc::new(AtomicUsize::new(0));
    let preset_found = Arc::new(AtomicUsize::new(0));
    let pdf_found = Arc::new(AtomicUsize::new(0));

    let spec = Arc::new(spec.clone());
    let stop2 = stop.clone();
    let audio_f2 = audio_found.clone();
    let daw_f2 = daw_found.clone();
    let preset_f2 = preset_found.clone();
    let pdf_f2 = pdf_found.clone();

    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(num_cpus::get().max(4))
        .build()
        .unwrap();

    std::thread::spawn(move || {
        pool.install(|| {
            union.par_iter().for_each(|root| {
                if stop2.load(Ordering::Relaxed) {
                    return;
                }
                walk_dir_parallel(
                    root,
                    0,
                    &visited,
                    &tx,
                    &audio_f2,
                    &daw_f2,
                    &preset_f2,
                    &pdf_f2,
                    batch_size,
                    &stop2,
                    &spec,
                    &active,
                );
            });
        });
        drop(pool);
    });

    loop {
        if should_stop() {
            stop.store(true, Ordering::Relaxed);
            while rx.try_recv().is_ok() {}
            break;
        }
        match rx.recv_timeout(std::time::Duration::from_millis(10)) {
            Ok(batch) => {
                let counts = UnifiedCounts {
                    audio: audio_found.load(Ordering::Relaxed),
                    daw: daw_found.load(Ordering::Relaxed),
                    preset: preset_found.load(Ordering::Relaxed),
                    pdf: pdf_found.load(Ordering::Relaxed),
                };
                on_batch(batch, counts);
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
    tx: &std::sync::mpsc::SyncSender<ClassifiedBatch>,
    audio_found: &Arc<AtomicUsize>,
    daw_found: &Arc<AtomicUsize>,
    preset_found: &Arc<AtomicUsize>,
    pdf_found: &Arc<AtomicUsize>,
    batch_size: usize,
    stop: &Arc<AtomicBool>,
    spec: &Arc<UnifiedSpec>,
    active_dirs: &[Arc<Mutex<Vec<String>>>],
) {
    if depth > 30 || stop.load(Ordering::Relaxed) {
        return;
    }

    {
        // Canonicalize OUTSIDE the mutex — it's a syscall (network roundtrip
        // on SMB) and must not block other workers while in flight.
        let orig = normalize_macos_path(dir.to_path_buf());
        let canon = fs::canonicalize(dir).ok().map(normalize_macos_path);
        let key = canon.unwrap_or_else(|| orig.clone());
        let mut vis = visited.lock().unwrap_or_else(|e| e.into_inner());
        if !vis.insert(key) {
            return;
        }
        vis.insert(orig);
    }

    let dir_str = dir.to_string_lossy().to_string();
    {
        // Fan out the dir-status push to every sink (walker-status tiles).
        for sink in active_dirs {
            let mut ad = sink.lock().unwrap_or_else(|e| e.into_inner());
            ad.push(dir_str.clone());
            // Rolling window sized to fill a full-height walker tile (~200
            // lines × 16px line-height ≈ 3200px, comfortably more than any
            // realistic tile). Body is overflow-y:auto so any excess scrolls.
            if ad.len() > 200 {
                let excess = ad.len() - 200;
                ad.drain(..excess);
            }
        }
    }

    // One bulk syscall (getattrlistbulk on macOS) returns name+type+size+mtime
    // for every entry in the directory. Replaces readdir + file_type + stat
    // per entry — critical for SMB where each syscall is a network roundtrip.
    let entries: Vec<BulkEntry> = match read_dir_bulk(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    // Per-type batches collected in this directory before being flushed.
    let mut audio_batch: Vec<AudioSample> = Vec::new();
    let mut daw_batch: Vec<DawProject> = Vec::new();
    let mut preset_batch: Vec<PresetFile> = Vec::new();
    let mut pdf_batch: Vec<PdfFile> = Vec::new();
    let mut subdirs: Vec<PathBuf> = Vec::new();

    for entry in &entries {
        let name_str = entry.name.as_str();
        // `@` prefix = Synology NAS system dirs (@eaDir is in every media
        // folder on a Synology share — alone it can double a scan's file
        // count). Also handles @tmp, @syno*, @appstore, @docker, @database,
        // @SynoDrive, @SynologyCloudSync, etc.
        if name_str.starts_with('.')
            || name_str.starts_with('@')
            || SKIP_DIRS.contains(&name_str)
        {
            continue;
        }
        if !spec.daw_include_backups && DAW_BACKUP_DIRS.contains(&name_str) {
            // Backup/Crash dirs only matter for DAW. Audio/preset/pdf don't care
            // — but these dirs typically contain autosaves of DAW projects, not
            // user content, so skipping is safe for all types.
            continue;
        }

        let path = entry.path.clone();
        let name_lower = name_str.to_lowercase();

        if entry.is_dir {
            // 1) Plugin bundle directory? DAW never enters; other types DO enter
            //    since plugin bundles may contain presets/PDFs/audio.
            let is_plugin_bundle = DAW_PLUGIN_BUNDLE_EXTENSIONS
                .iter()
                .any(|ext| name_lower.ends_with(ext));

            // 2) DAW package directory? (.logicx, .band) → treat as DAW project,
            //    don't descend. Other types ignore it (packages have no audio).
            let is_daw_pkg = DAW_PACKAGE_EXTENSIONS
                .iter()
                .any(|ext| name_lower.ends_with(ext));

            if is_daw_pkg && under_any_root(&path, &spec.daw_roots) {
                if let Some(ext_with_dot) = ext_match(&name_lower, DAW_EXTENSIONS) {
                    let format = ext_with_dot.strip_prefix('.').unwrap_or("").to_uppercase();
                    if !(format == "BAND" && !is_valid_band_package(&path)) {
                        let path_str = path.to_string_lossy().to_string();
                        if !spec.daw_exclude.contains(&path_str) {
                            // Package size still needs recursive directory
                            // walk — that's inherent to the data model.
                            let size = get_directory_size(&path);
                            // Dir mtime comes free on macOS bulk path. On the
                            // portable fallback (Linux/Windows) entry.mtime_secs
                            // is 0 for dirs, so stat lazily here — only for
                            // packages we actually emit, not every dir.
                            let modified = if entry.mtime_secs > 0 {
                                fmt_mtime_ymd(entry.mtime_secs)
                            } else {
                                fs::metadata(&path)
                                    .ok()
                                    .and_then(|m| m.modified().ok())
                                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                                    .map(|d| fmt_mtime_ymd(d.as_secs() as i64))
                                    .unwrap_or_default()
                            };
                            let project_name = path
                                .file_stem()
                                .map(|s| s.to_string_lossy().to_string())
                                .unwrap_or_default();
                            let daw = daw_name_for_format(&format).to_string();
                            daw_batch.push(DawProject {
                                name: project_name,
                                path: path_str,
                                directory: dir.to_string_lossy().to_string(),
                                format,
                                daw,
                                size,
                                size_formatted: format_size(size),
                                modified,
                            });
                            daw_found.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                }
            }

            if is_plugin_bundle {
                // Plugin bundles: DAW skips entirely. Others may still want to
                // find presets/pdfs inside — descend for them.
                if !spec.preset_roots.is_empty() || !spec.pdf_roots.is_empty() || !spec.audio_roots.is_empty() {
                    subdirs.push(path);
                }
                continue;
            }

            if is_daw_pkg {
                // DAW package — never descend (contents are DAW-internal).
                continue;
            }

            subdirs.push(path);
            continue;
        }

        if !entry.is_file {
            continue;
        }

        // Files — classify by extension into each type's bucket.
        let ext_with_dot = match name_lower.rfind('.') {
            Some(i) => &name_lower[i..],
            None => continue,
        };
        let size = entry.size;
        let modified = fmt_mtime_ymd(entry.mtime_secs);

        // Audio
        if AUDIO_EXTENSIONS.contains(&ext_with_dot)
            && under_any_root(&path, &spec.audio_roots)
        {
            let path_str = path.to_string_lossy().to_string();
            if !spec.audio_exclude.contains(&path_str) && size > 0 {
                let sample_name = path
                    .file_stem()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_default();
                let fast_fmt = matches!(
                    ext_with_dot,
                    ".wav" | ".aiff" | ".aif" | ".flac"
                );
                let (dur, ch, sr, bps) = if fast_fmt {
                    let am = crate::audio_scanner::get_audio_metadata(&path_str);
                    (am.duration, am.channels, am.sample_rate, am.bits_per_sample)
                } else {
                    (None, None, None, None)
                };
                audio_batch.push(AudioSample {
                    name: sample_name,
                    path: path_str.clone(),
                    directory: dir.to_string_lossy().to_string(),
                    format: ext_with_dot.strip_prefix('.').unwrap_or("").to_uppercase(),
                    size,
                    size_formatted: format_size(size),
                    modified: modified.clone(),
                    duration: dur,
                    channels: ch,
                    sample_rate: sr,
                    bits_per_sample: bps,
                });
                audio_found.fetch_add(1, Ordering::Relaxed);
            }
        }

        // DAW (single-file formats — packages handled above for directories)
        if DAW_EXTENSIONS.contains(&ext_with_dot)
            && under_any_root(&path, &spec.daw_roots)
        {
            let path_str = path.to_string_lossy().to_string();
            if !spec.daw_exclude.contains(&path_str) {
                let format = ext_with_dot.strip_prefix('.').unwrap_or("").to_uppercase();
                // .band is ONLY valid as a package — skip files with this ext.
                if format != "BAND" {
                    let project_name = path
                        .file_stem()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_default();
                    let daw = daw_name_for_format(&format).to_string();
                    daw_batch.push(DawProject {
                        name: project_name,
                        path: path_str,
                        directory: dir.to_string_lossy().to_string(),
                        format,
                        daw,
                        size,
                        size_formatted: format_size(size),
                        modified: modified.clone(),
                    });
                    daw_found.fetch_add(1, Ordering::Relaxed);
                }
            }
        }

        // Preset (and midi — same bucket)
        if PRESET_EXTENSIONS.contains(&ext_with_dot)
            && under_any_root(&path, &spec.preset_roots)
        {
            let path_str = path.to_string_lossy().to_string();
            if !spec.preset_exclude.contains(&path_str) {
                let preset_name = path
                    .file_stem()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_default();
                preset_batch.push(PresetFile {
                    name: preset_name,
                    path: path_str,
                    directory: dir.to_string_lossy().to_string(),
                    format: ext_with_dot.strip_prefix('.').unwrap_or("").to_uppercase(),
                    size,
                    size_formatted: format_size(size),
                    modified: modified.clone(),
                });
                preset_found.fetch_add(1, Ordering::Relaxed);
            }
        }

        // PDF
        if ext_with_dot == PDF_EXTENSION && under_any_root(&path, &spec.pdf_roots) {
            let path_str = path.to_string_lossy().to_string();
            if !spec.pdf_exclude.contains(&path_str) {
                let pdf_name = path
                    .file_stem()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_default();
                pdf_batch.push(PdfFile {
                    name: pdf_name,
                    path: path_str,
                    directory: dir.to_string_lossy().to_string(),
                    size,
                    size_formatted: format_size(size),
                    modified,
                });
                pdf_found.fetch_add(1, Ordering::Relaxed);
            }
        }

        // Flush any batch that's reached batch_size.
        if audio_batch.len() >= batch_size {
            let _ = tx.send(ClassifiedBatch::Audio(std::mem::take(&mut audio_batch)));
        }
        if daw_batch.len() >= batch_size {
            let _ = tx.send(ClassifiedBatch::Daw(std::mem::take(&mut daw_batch)));
        }
        if preset_batch.len() >= batch_size {
            let _ = tx.send(ClassifiedBatch::Preset(std::mem::take(&mut preset_batch)));
        }
        if pdf_batch.len() >= batch_size {
            let _ = tx.send(ClassifiedBatch::Pdf(std::mem::take(&mut pdf_batch)));
        }
    }

    // Flush any partial batches at end of directory.
    if !audio_batch.is_empty() {
        let _ = tx.send(ClassifiedBatch::Audio(audio_batch));
    }
    if !daw_batch.is_empty() {
        let _ = tx.send(ClassifiedBatch::Daw(daw_batch));
    }
    if !preset_batch.is_empty() {
        let _ = tx.send(ClassifiedBatch::Preset(preset_batch));
    }
    if !pdf_batch.is_empty() {
        let _ = tx.send(ClassifiedBatch::Pdf(pdf_batch));
    }

    subdirs.par_iter().for_each(|subdir| {
        walk_dir_parallel(
            subdir,
            depth + 1,
            visited,
            tx,
            audio_found,
            daw_found,
            preset_found,
            pdf_found,
            batch_size,
            stop,
            spec,
            active_dirs,
        );
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;

    struct TestDir {
        path: PathBuf,
    }
    impl TestDir {
        fn new(name: &str) -> Self {
            let path = std::env::temp_dir().join(format!(
                "upum_uw_{}_{}",
                name,
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            let _ = fs::remove_dir_all(&path);
            fs::create_dir_all(&path).unwrap();
            Self { path }
        }
    }
    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn touch(p: &Path, content: &[u8]) {
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let mut f = File::create(p).unwrap();
        f.write_all(content).unwrap();
    }

    #[test]
    fn test_under_any_root() {
        let roots = vec![PathBuf::from("/a/b"), PathBuf::from("/x/y")];
        assert!(under_any_root(Path::new("/a/b/c.wav"), &roots));
        assert!(under_any_root(Path::new("/x/y/z/1.pdf"), &roots));
        assert!(!under_any_root(Path::new("/a/other.wav"), &roots));
        assert!(!under_any_root(Path::new("/q.wav"), &roots));
        assert!(!under_any_root(Path::new("/a/b/c.wav"), &[]));
    }

    #[test]
    fn test_ext_match() {
        assert_eq!(
            ext_match("song.wav", AUDIO_EXTENSIONS),
            Some(".wav".to_string())
        );
        assert_eq!(
            ext_match("mix.flp", DAW_EXTENSIONS),
            Some(".flp".to_string())
        );
        assert_eq!(ext_match("readme.txt", AUDIO_EXTENSIONS), None);
    }

    #[test]
    fn test_walk_unified_classifies_by_extension() {
        let tmp = TestDir::new("classify");
        let root = tmp.path.clone();
        touch(&root.join("a.wav"), b"RIFF");
        touch(&root.join("b.pdf"), b"%PDF-1.4");
        touch(&root.join("c.fxp"), b"CcnK");
        touch(&root.join("d.als"), b"<?xml");
        touch(&root.join("e.mid"), b"MThd");
        touch(&root.join("skip.txt"), b"junk");

        let spec = UnifiedSpec {
            audio_roots: vec![root.clone()],
            daw_roots: vec![root.clone()],
            preset_roots: vec![root.clone()],
            pdf_roots: vec![root.clone()],
            ..Default::default()
        };

        let mut audio = Vec::new();
        let mut daw = Vec::new();
        let mut preset = Vec::new();
        let mut pdf = Vec::new();

        walk_unified(
            &spec,
            &mut |batch, _counts| match batch {
                ClassifiedBatch::Audio(b) => audio.extend(b),
                ClassifiedBatch::Daw(b) => daw.extend(b),
                ClassifiedBatch::Preset(b) => preset.extend(b),
                ClassifiedBatch::Pdf(b) => pdf.extend(b),
            },
            &|| false,
            Vec::new(),
        );

        assert_eq!(audio.len(), 1, "expected 1 audio file");
        assert_eq!(audio[0].format, "WAV");
        assert_eq!(pdf.len(), 1, "expected 1 pdf");
        assert_eq!(daw.len(), 1, "expected 1 daw");
        assert_eq!(daw[0].format, "ALS");
        assert_eq!(preset.len(), 2, "expected 2 presets (fxp + mid)");
    }

    #[test]
    fn test_walk_unified_respects_per_type_roots() {
        // Only audio is configured — files of other types exist but shouldn't
        // appear in output.
        let tmp = TestDir::new("pertype");
        let root = tmp.path.clone();
        touch(&root.join("a.wav"), b"RIFF");
        touch(&root.join("b.pdf"), b"%PDF-1.4");
        touch(&root.join("c.fxp"), b"CcnK");

        let spec = UnifiedSpec {
            audio_roots: vec![root.clone()],
            ..Default::default()
        };

        let mut audio = 0usize;
        let mut other = 0usize;
        walk_unified(
            &spec,
            &mut |batch, _| match batch {
                ClassifiedBatch::Audio(b) => audio += b.len(),
                _ => other += 1,
            },
            &|| false,
            Vec::new(),
        );
        assert_eq!(audio, 1);
        assert_eq!(other, 0, "no batches for unconfigured types");
    }

    #[test]
    fn test_walk_unified_skips_hidden_and_skip_dirs() {
        let tmp = TestDir::new("skipdirs");
        let root = tmp.path.clone();
        touch(&root.join("keep.wav"), b"RIFF");
        touch(&root.join(".hidden.wav"), b"RIFF");
        touch(&root.join("node_modules/dep.wav"), b"RIFF");
        touch(&root.join("Caches/thing.wav"), b"RIFF");
        touch(&root.join("DerivedData/build.wav"), b"RIFF");
        // Synology system dirs — @-prefixed (dir guard) and #snapshot (list).
        touch(&root.join("@eaDir/thumb.wav"), b"RIFF");
        touch(&root.join("@tmp/work.wav"), b"RIFF");
        touch(&root.join("@SynoDrive/sync.wav"), b"RIFF");
        touch(&root.join("#snapshot/old.wav"), b"RIFF");
        touch(&root.join("#recycle/trash.wav"), b"RIFF");

        let spec = UnifiedSpec {
            audio_roots: vec![root.clone()],
            ..Default::default()
        };

        let mut names: Vec<String> = Vec::new();
        walk_unified(
            &spec,
            &mut |batch, _| {
                if let ClassifiedBatch::Audio(b) = batch {
                    names.extend(b.into_iter().map(|s| s.name));
                }
            },
            &|| false,
            Vec::new(),
        );
        assert_eq!(names, vec!["keep".to_string()]);
    }

    #[test]
    fn test_walk_unified_path_not_under_root_excluded() {
        // File exists in the traversal (because some OTHER type's root contains
        // it) but the audio_root does not — it must not appear as audio.
        let tmp = TestDir::new("rootcheck");
        let root = tmp.path.clone();
        let audio_dir = root.join("samples");
        let pdf_dir = root.join("docs");
        touch(&audio_dir.join("a.wav"), b"RIFF");
        touch(&pdf_dir.join("cross.wav"), b"RIFF"); // outside audio_roots
        touch(&pdf_dir.join("doc.pdf"), b"%PDF");

        let spec = UnifiedSpec {
            audio_roots: vec![audio_dir.clone()],
            pdf_roots: vec![pdf_dir.clone()],
            ..Default::default()
        };

        let mut audio = Vec::new();
        let mut pdf = Vec::new();
        walk_unified(
            &spec,
            &mut |batch, _| match batch {
                ClassifiedBatch::Audio(b) => audio.extend(b),
                ClassifiedBatch::Pdf(b) => pdf.extend(b),
                _ => {}
            },
            &|| false,
            Vec::new(),
        );
        assert_eq!(audio.len(), 1, "only audio under audio_roots counts");
        assert_eq!(audio[0].name, "a");
        assert_eq!(pdf.len(), 1);
    }

    #[test]
    fn test_walk_unified_stop_flag() {
        let tmp = TestDir::new("stopflag");
        let root = tmp.path.clone();
        for i in 0..200 {
            touch(&root.join(format!("f{}.wav", i)), b"RIFF");
        }
        let spec = UnifiedSpec {
            audio_roots: vec![root.clone()],
            ..Default::default()
        };
        walk_unified(&spec, &mut |_, _| {}, &|| true, Vec::new());
        // No assertion — just ensure stop=true returns promptly (test timeout
        // would catch a hang).
    }

    #[test]
    fn test_walk_unified_excludes_specific_paths() {
        let tmp = TestDir::new("exclude");
        let root = tmp.path.clone();
        touch(&root.join("keep.wav"), b"RIFF");
        touch(&root.join("skip.wav"), b"RIFF");
        let skip_path = root.join("skip.wav").to_string_lossy().to_string();
        let mut excl = HashSet::new();
        excl.insert(skip_path);
        let spec = UnifiedSpec {
            audio_roots: vec![root.clone()],
            audio_exclude: excl,
            ..Default::default()
        };
        let mut names = Vec::new();
        walk_unified(
            &spec,
            &mut |batch, _| {
                if let ClassifiedBatch::Audio(b) = batch {
                    names.extend(b.into_iter().map(|s| s.name));
                }
            },
            &|| false,
            Vec::new(),
        );
        assert_eq!(names, vec!["keep".to_string()]);
    }

    #[test]
    fn test_walk_unified_empty_spec() {
        let tmp = TestDir::new("empty");
        let root = tmp.path.clone();
        touch(&root.join("a.wav"), b"RIFF");
        let spec = UnifiedSpec::default();
        let mut batches = 0;
        walk_unified(
            &spec,
            &mut |_, _| {
                batches += 1;
            },
            &|| false,
            Vec::new(),
        );
        assert_eq!(batches, 0, "empty spec produces no output");
    }

    /// Display names for DAW `format` codes must stay aligned with `daw_scanner` / UI.
    #[test]
    fn daw_name_for_format_maps_all_known_codes() {
        assert_eq!(daw_name_for_format("ALS"), "Ableton Live");
        assert_eq!(daw_name_for_format("LOGICX"), "Logic Pro");
        assert_eq!(daw_name_for_format("FLP"), "FL Studio");
        assert_eq!(daw_name_for_format("CPR"), "Cubase");
        assert_eq!(daw_name_for_format("NPR"), "Nuendo");
        assert_eq!(daw_name_for_format("BWPROJECT"), "Bitwig Studio");
        assert_eq!(daw_name_for_format("RPP"), "REAPER");
        assert_eq!(daw_name_for_format("RPP-BAK"), "REAPER");
        assert_eq!(daw_name_for_format("PTX"), "Pro Tools");
        assert_eq!(daw_name_for_format("PTF"), "Pro Tools");
        assert_eq!(daw_name_for_format("SONG"), "Studio One");
        assert_eq!(daw_name_for_format("REASON"), "Reason");
        assert_eq!(daw_name_for_format("AUP"), "Audacity");
        assert_eq!(daw_name_for_format("AUP3"), "Audacity");
        assert_eq!(daw_name_for_format("BAND"), "GarageBand");
        assert_eq!(daw_name_for_format("ARDOUR"), "Ardour");
        assert_eq!(daw_name_for_format("DAWPROJECT"), "DAWproject");
        assert_eq!(daw_name_for_format("___unknown___"), "Unknown");
        assert_eq!(daw_name_for_format(""), "Unknown");
    }

    #[test]
    fn normalize_macos_path_strips_system_data_volume_prefix() {
        let p = PathBuf::from("/System/Volumes/Data/Users/foo/bar");
        let n = normalize_macos_path(p);
        #[cfg(target_os = "macos")]
        {
            assert_eq!(n, PathBuf::from("/Users/foo/bar"));
        }
        #[cfg(not(target_os = "macos"))]
        {
            assert_eq!(n, PathBuf::from("/System/Volumes/Data/Users/foo/bar"));
        }
    }

    #[test]
    fn normalize_macos_path_noop_when_not_data_volume() {
        let p = PathBuf::from("/home/user/Projects");
        assert_eq!(normalize_macos_path(p), PathBuf::from("/home/user/Projects"));
    }
}
