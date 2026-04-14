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
//! 6. Symlinks: `readdir` does not mark symlink targets as files/dirs; each
//!    symlink is `stat`ed and classified by its target (broken symlinks skipped).
//!
//! Per-type progress callbacks stream batches as they're discovered.

use crate::audio_extensions::AUDIO_EXTENSIONS;
use crate::bulk_stat::{BulkEntry, read_dir_bulk};
use crate::history::{AudioSample, DawProject, PdfFile, PresetFile};
use crate::scanner_skip_dirs::SCANNER_SKIP_DIRS as SKIP_DIRS;
use dashmap::DashSet;
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

/// Same path key string as visit deduplication: `canonicalize` when possible, else normalized original.
fn directory_incremental_key(dir: &Path) -> String {
    let orig = normalize_macos_path(dir.to_path_buf());
    let canon = fs::canonicalize(dir).ok().map(normalize_macos_path);
    let key = canon.unwrap_or(orig);
    key.to_string_lossy().to_string()
}

fn dir_mtime_secs(dir: &Path) -> i64 {
    fs::metadata(dir)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Snapshot + in-scan updates for incremental directory skipping (`scan_unified` and
/// standalone per-type walkers). Persisted in SQLite under domain `"unified"` so all scan modes
/// share one mtime map.
pub struct IncrementalDirState {
    mtimes: Arc<Mutex<HashMap<String, i64>>>,
    pending: Arc<Mutex<Vec<(String, i64)>>>,
}

impl IncrementalDirState {
    pub fn new(snapshot: HashMap<String, i64>) -> Self {
        Self {
            mtimes: Arc::new(Mutex::new(snapshot)),
            pending: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Skip this directory (and its subtree) when stored mtime ≥ current — same rule as
    /// `scan_unified`.
    pub fn should_skip(&self, dir: &Path) -> bool {
        let key = directory_incremental_key(dir);
        let cur = dir_mtime_secs(dir);
        if cur <= 0 {
            return false;
        }
        let map = self.mtimes.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(&stored) = map.get(&key) {
            if cur <= stored {
                return true;
            }
        }
        false
    }

    /// Call after successfully listing/processing a directory so the next run can skip it if
    /// unchanged.
    pub fn record_scanned_dir(&self, dir: &Path) {
        let key = directory_incremental_key(dir);
        let cur = dir_mtime_secs(dir);
        if cur <= 0 {
            return;
        }
        let mut map = self.mtimes.lock().unwrap_or_else(|e| e.into_inner());
        map.insert(key.clone(), cur);
        drop(map);
        self.pending
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push((key, cur));
    }

    pub fn take_pending(&self) -> Vec<(String, i64)> {
        let mut p = self.pending.lock().unwrap_or_else(|e| e.into_inner());
        std::mem::take(&mut *p)
    }
}

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

// ── Network filesystem detection ─────────────────────────────────────────
// Returns the filesystem type name (e.g. "smbfs", "nfs", "afpfs") if `path`
// lives on a network mount, or None for local filesystems.  Uses statfs(2)
// on macOS; always returns None on other platforms.
#[cfg(target_os = "macos")]
fn network_fs_type(path: &Path) -> Option<String> {
    use std::ffi::CString;
    let c_path = CString::new(path.as_os_str().to_string_lossy().as_bytes()).ok()?;
    let mut stat: libc::statfs = unsafe { std::mem::zeroed() };
    let rc = unsafe { libc::statfs(c_path.as_ptr(), &mut stat) };
    if rc != 0 {
        return None;
    }
    let fstype = unsafe {
        std::ffi::CStr::from_ptr(stat.f_fstypename.as_ptr())
            .to_string_lossy()
            .to_string()
    };
    // MNT_LOCAL is clear on network mounts (smbfs, nfs, afpfs, webdav).
    const MNT_LOCAL: u32 = 0x00001000;
    if stat.f_flags & MNT_LOCAL == 0 {
        Some(fstype)
    } else {
        None
    }
}

#[cfg(not(target_os = "macos"))]
fn network_fs_type(_path: &Path) -> Option<String> {
    None
}

/// Quick check: is this path likely on a network share?
/// Network mounts can be anywhere (not just /Volumes/ or /mnt/), so fall back
/// to statfs(2) which is authoritative on macOS.
fn is_network_path(path: &Path) -> bool {
    network_fs_type(path).is_some()
}

// Audio: `crate::audio_extensions::AUDIO_EXTENSIONS`. DAW/preset lists are local
// to this walker (keep aligned with `daw_scanner` / `preset_scanner` / file watcher).
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
    // .mid / .midi live in midi_files (separate walker/table) — NEVER here.
];

const PDF_EXTENSION: &str = ".pdf";

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
    path.join("Media").is_dir()
        || path.join("Output").is_dir()
        || path.join("Freeze Files").is_dir()
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
            let p = entry.path();
            // Match `daw_scanner` / `scanner`: `file_type().is_file()` skips symlink
            // children; `path.is_dir()` + `metadata` follows symlinks for size.
            if p.is_dir() {
                total += get_directory_size_depth(&p, depth + 1);
            } else if let Ok(meta) = fs::metadata(&p) {
                total += meta.len();
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

/// Per-tab stop flags for [`walk_unified`] — any `true` cancels the walk (mirrors `stop_*_scan` commands).
#[derive(Clone)]
pub struct UnifiedStopArms {
    pub audio: Arc<AtomicBool>,
    pub daw: Arc<AtomicBool>,
    pub preset: Arc<AtomicBool>,
    pub pdf: Arc<AtomicBool>,
}

impl UnifiedStopArms {
    #[inline]
    pub fn any(&self) -> bool {
        self.audio.load(Ordering::Relaxed)
            || self.daw.load(Ordering::Relaxed)
            || self.preset.load(Ordering::Relaxed)
            || self.pdf.load(Ordering::Relaxed)
    }

    pub fn all_false() -> Self {
        Self {
            audio: Arc::new(AtomicBool::new(false)),
            daw: Arc::new(AtomicBool::new(false)),
            preset: Arc::new(AtomicBool::new(false)),
            pdf: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn all_true() -> Self {
        Self {
            audio: Arc::new(AtomicBool::new(true)),
            daw: Arc::new(AtomicBool::new(true)),
            preset: Arc::new(AtomicBool::new(true)),
            pdf: Arc::new(AtomicBool::new(true)),
        }
    }
}

/// Walk the union of all roots across types, emitting classified batches.
///
/// The callback is invoked from the main receiving thread (single-threaded),
/// so it does not need to be Send/Sync. Batches are ~100 files each.
pub fn walk_unified(
    spec: &UnifiedSpec,
    on_batch: &mut dyn FnMut(ClassifiedBatch, UnifiedCounts),
    stops: UnifiedStopArms,
    active_dirs: Vec<Arc<Mutex<Vec<String>>>>,
    incremental: Option<Arc<IncrementalDirState>>,
) {
    let batch_size = 100;
    // Fan-out sinks — each push goes to every provided Vec, so the walker's
    // single traversal drives all N walker-status tiles simultaneously.
    let active: Vec<Arc<Mutex<Vec<String>>>> = if active_dirs.is_empty() {
        vec![Arc::new(Mutex::new(Vec::new()))]
    } else {
        active_dirs
    };
    let (tx, rx) = std::sync::mpsc::sync_channel::<ClassifiedBatch>(256);
    let visited = Arc::new(DashSet::new());

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

    // Log root set with network mount annotations so the user can verify
    // their SMB/NFS shares are included in the walk.
    for r in &union {
        if let Some(fs) = network_fs_type(r) {
            let p = r.display().to_string();
            crate::write_app_log_verbose(format!("SCAN ROOT — unified | {} [NETWORK: {}]", p, fs,));
        }
    }

    let audio_found = Arc::new(AtomicUsize::new(0));
    let daw_found = Arc::new(AtomicUsize::new(0));
    let preset_found = Arc::new(AtomicUsize::new(0));
    let pdf_found = Arc::new(AtomicUsize::new(0));

    let spec = Arc::new(spec.clone());
    let stops_w = stops.clone();
    let audio_f2 = audio_found.clone();
    let daw_f2 = daw_found.clone();
    let preset_f2 = preset_found.clone();
    let pdf_f2 = pdf_found.clone();

    let pool = crate::build_low_priority_thread_pool(num_cpus::get().max(4));

    let tcc_denied: Arc<DashSet<PathBuf>> = Arc::new(DashSet::new());
    let tcc_summary = tcc_denied.clone();
    std::thread::spawn(move || {
        pool.install(|| {
            union.par_iter().for_each(|root| {
                if stops_w.any() {
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
                    stops_w.clone(),
                    &spec,
                    &active,
                    &tcc_denied,
                    incremental.clone(),
                );
            });
        });
        drop(pool);
    });

    loop {
        if stops.any() {
            while rx.try_recv().is_ok() {}
            break;
        }
        match rx.recv_timeout(std::time::Duration::from_millis(10)) {
            Ok(batch) => {
                if stops.any() {
                    while rx.try_recv().is_ok() {}
                    break;
                }
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

    if !tcc_summary.is_empty() {
        let paths: Vec<_> = tcc_summary
            .iter()
            .map(|p| p.key().display().to_string())
            .collect();
        crate::write_app_log(format!(
            "SCAN TCC SUMMARY — {} path(s) denied by macOS TCC: {} \
             (grant access in System Settings → Privacy & Security → Files and Folders)",
            paths.len(),
            paths.join(", "),
        ));
    }
}

#[allow(clippy::too_many_arguments)]
fn walk_dir_parallel(
    dir: &Path,
    depth: u32,
    visited: &Arc<DashSet<PathBuf>>,
    tx: &std::sync::mpsc::SyncSender<ClassifiedBatch>,
    audio_found: &Arc<AtomicUsize>,
    daw_found: &Arc<AtomicUsize>,
    preset_found: &Arc<AtomicUsize>,
    pdf_found: &Arc<AtomicUsize>,
    batch_size: usize,
    stops: UnifiedStopArms,
    spec: &Arc<UnifiedSpec>,
    active_dirs: &[Arc<Mutex<Vec<String>>>],
    tcc_denied: &Arc<DashSet<PathBuf>>,
    incremental: Option<Arc<IncrementalDirState>>,
) {
    if depth > 30 || stops.any() {
        return;
    }

    // Skip paths under a previously TCC-denied ancestor — no syscalls needed.
    if tcc_denied.iter().any(|d| dir.starts_with(d.key())) {
        return;
    }

    {
        // Canonicalize OUTSIDE the mutex — it's a syscall (network roundtrip
        // on SMB) and must not block other workers while in flight.
        let orig = normalize_macos_path(dir.to_path_buf());
        let canon_result = fs::canonicalize(dir);
        if is_network_path(dir) {
            if let Err(ref e) = canon_result {
                crate::write_app_log_verbose(format!(
                    "SCAN NETWORK CANONICALIZE FAIL — unified | {} | {} (using original path as dedup key)",
                    dir.display(),
                    e,
                ));
            }
        }
        let canon = canon_result.ok().map(normalize_macos_path);
        let key = canon.clone().unwrap_or_else(|| orig.clone());
        if !visited.insert(key.clone()) {
            if is_network_path(dir) {
                crate::write_app_log_verbose(format!(
                    "SCAN DEDUP SKIP — unified | orig={} | canon={} | key={}",
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
        visited.insert(orig);
    }

    if let Some(ref inc) = incremental {
        if inc.should_skip(dir) {
            return;
        }
    }

    let dir_str = dir.to_string_lossy().to_string();

    // Log when entering a network share root (depth 0-2) so the user can
    // verify their mounts are actually being traversed.
    if depth <= 2 {
        if let Some(fstype) = network_fs_type(dir) {
            crate::write_app_log_verbose(format!(
                "SCAN NETWORK ENTER — unified | {} | fs={} | depth={}",
                dir.display(),
                fstype,
                depth,
            ));
        }
    }
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
    // Retry once on transient errors (common on SMB/NFS mounts).
    let is_net = is_network_path(dir);
    let entries: Vec<BulkEntry> = match read_dir_bulk(dir) {
        Ok(e) => e,
        Err(first_err) => {
            // EPERM (os error 1) on macOS = TCC denial — retrying is futile,
            // the user must grant access in System Settings → Privacy & Security.
            // Log once per denied root, silently skip descendants.
            if first_err.raw_os_error() == Some(1) {
                if !tcc_denied.iter().any(|d| dir.starts_with(d.key())) {
                    tcc_denied.insert(dir.to_path_buf());
                    crate::write_app_log(format!(
                        "SCAN TCC DENIED — unified | {} | {} \
                         (grant Full Disk Access or Files and Folders permission)",
                        dir.display(),
                        first_err,
                    ));
                }
                return;
            }
            // Local filesystem errors are not transient — don't retry.
            if !is_net {
                return;
            }
            // Network shares (SMB/NFS/AFP) return transient ETIMEDOUT / EIO /
            // ENOENT on first access after idle, wake-from-sleep, or auto-mount.
            // Retry up to 3 times with exponential backoff (50ms, 100ms, 200ms).
            const MAX_RETRIES: u32 = 3;
            const BASE_DELAY_MS: u64 = 50;
            crate::write_app_log_verbose(format!(
                "SCAN NETWORK RETRY — unified | {} | first error: {} | up to {} retries",
                dir.display(),
                first_err,
                MAX_RETRIES,
            ));
            let mut last_err = first_err;
            let mut recovered = None;
            for attempt in 0..MAX_RETRIES {
                let delay = BASE_DELAY_MS * (1 << attempt); // 50, 100, 200
                std::thread::sleep(std::time::Duration::from_millis(delay));
                match read_dir_bulk(dir) {
                    Ok(e) => {
                        crate::write_app_log_verbose(format!(
                            "SCAN NETWORK RECOVERED — unified | {} | succeeded on retry {}",
                            dir.display(),
                            attempt + 1,
                        ));
                        recovered = Some(e);
                        break;
                    }
                    Err(e) => {
                        last_err = e;
                    }
                }
            }
            match recovered {
                Some(e) => e,
                None => {
                    let fsinfo = network_fs_type(dir)
                        .map(|fs| format!(" (fs={})", fs))
                        .unwrap_or_default();
                    crate::write_app_log(format!(
                        "SCAN READDIR ERROR — unified | {}{} | {} retries exhausted | last: {}",
                        dir.display(),
                        fsinfo,
                        MAX_RETRIES,
                        last_err,
                    ));
                    return;
                }
            }
        }
    };

    // Verbose: entry counts near roots only (avoids multi-million-line logs on deep trees).
    if depth <= 2 {
        let n = entries.len();
        let d = dir.to_path_buf();
        crate::app_log_verbose(move || {
            format!(
                "SCAN VERBOSE — unified | depth={} | dir={} | entries={}",
                depth,
                d.display(),
                n
            )
        });
    }

    // Per-type batches collected in this directory before being flushed.
    let mut audio_batch: Vec<AudioSample> = Vec::new();
    let mut daw_batch: Vec<DawProject> = Vec::new();
    let mut preset_batch: Vec<PresetFile> = Vec::new();
    let mut pdf_batch: Vec<PdfFile> = Vec::new();
    let mut subdirs: Vec<PathBuf> = Vec::new();

    for (i, entry) in entries.iter().enumerate() {
        if i != 0 && i % 256 == 0 && stops.any() {
            return;
        }
        let name_str = entry.name.as_str();
        // `@` prefix = Synology NAS system dirs (@eaDir is in every media
        // folder on a Synology share — alone it can double a scan's file
        // count). Also handles @tmp, @syno*, @appstore, @docker, @database,
        // @SynoDrive, @SynologyCloudSync, etc.
        if name_str.starts_with('.') || name_str.starts_with('@') || SKIP_DIRS.contains(&name_str) {
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

        // Bulk `d_type` lists symlinks as neither file nor dir — follow target.
        let (is_dir, is_file, size, mtime_secs) = if entry.is_symlink {
            match fs::metadata(&path) {
                Ok(m) => {
                    let is_d = m.is_dir();
                    let is_f = m.is_file();
                    let sz = if is_f { m.len() } else { 0 };
                    let mt = m
                        .modified()
                        .ok()
                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                        .map(|d| d.as_secs() as i64)
                        .unwrap_or(0);
                    (is_d, is_f, sz, mt)
                }
                Err(_) => continue,
            }
        } else {
            (entry.is_dir, entry.is_file, entry.size, entry.mtime_secs)
        };

        if is_dir {
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
                            if stops.any() {
                                return;
                            }
                            // Package size still needs recursive directory
                            // walk — that's inherent to the data model.
                            let size = get_directory_size(&path);
                            // Dir mtime comes free on macOS bulk path. On the
                            // portable fallback (Linux/Windows) entry.mtime_secs
                            // is 0 for dirs, so stat lazily here — only for
                            // packages we actually emit, not every dir.
                            let modified = if mtime_secs > 0 {
                                fmt_mtime_ymd(mtime_secs)
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
                if !spec.preset_roots.is_empty()
                    || !spec.pdf_roots.is_empty()
                    || !spec.audio_roots.is_empty()
                {
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

        if !is_file {
            continue;
        }

        // Files — classify by extension into each type's bucket.
        let ext_with_dot = match name_lower.rfind('.') {
            Some(i) => &name_lower[i..],
            None => continue,
        };
        let modified = fmt_mtime_ymd(mtime_secs);

        // Audio
        if AUDIO_EXTENSIONS.contains(&ext_with_dot) && under_any_root(&path, &spec.audio_roots) {
            let path_str = path.to_string_lossy().to_string();
            if !spec.audio_exclude.contains(&path_str) && size > 0 {
                if stops.any() {
                    return;
                }
                let sample_name = path
                    .file_stem()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_default();
                let am = crate::audio_scanner::get_audio_metadata(&path_str);
                let (dur, ch, sr, bps) =
                    (am.duration, am.channels, am.sample_rate, am.bits_per_sample);
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
        if DAW_EXTENSIONS.contains(&ext_with_dot) && under_any_root(&path, &spec.daw_roots) {
            let path_str = path.to_string_lossy().to_string();
            if !spec.daw_exclude.contains(&path_str) {
                let format = ext_with_dot.strip_prefix('.').unwrap_or("").to_uppercase();
                // .band is ONLY valid as a package — skip files with this ext.
                // .ptx/.ptf must match Pro Tools session BOF (filters CUDA .ptx, etc.).
                if format != "BAND"
                    && ((format != "PTX" && format != "PTF")
                        || crate::daw_scanner::is_valid_pro_tools_session_file(&path))
                {
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
        if PRESET_EXTENSIONS.contains(&ext_with_dot) && under_any_root(&path, &spec.preset_roots) {
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
                    ..Default::default()
                });
                pdf_found.fetch_add(1, Ordering::Relaxed);
            }
        }

        // Flush any batch that's reached batch_size.
        if audio_batch.len() >= batch_size {
            if stops.any() {
                return;
            }
            let _ = tx.send(ClassifiedBatch::Audio(std::mem::take(&mut audio_batch)));
        }
        if daw_batch.len() >= batch_size {
            if stops.any() {
                return;
            }
            let _ = tx.send(ClassifiedBatch::Daw(std::mem::take(&mut daw_batch)));
        }
        if preset_batch.len() >= batch_size {
            if stops.any() {
                return;
            }
            let _ = tx.send(ClassifiedBatch::Preset(std::mem::take(&mut preset_batch)));
        }
        if pdf_batch.len() >= batch_size {
            if stops.any() {
                return;
            }
            let _ = tx.send(ClassifiedBatch::Pdf(std::mem::take(&mut pdf_batch)));
        }
    }

    // Flush any partial batches at end of directory.
    if stops.any() {
        return;
    }
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
            stops.clone(),
            spec,
            active_dirs,
            tcc_denied,
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
    use std::collections::HashMap;
    use std::fs::{self, File};
    use std::io::Write;
    use std::path::Path;
    use std::sync::Arc;

    fn try_symlink(target: &Path, link: &Path) -> bool {
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(target, link).is_ok()
        }
        #[cfg(windows)]
        {
            std::os::windows::fs::symlink_file(target, link).is_ok()
        }
        #[cfg(not(any(unix, windows)))]
        {
            let _ = (target, link);
            false
        }
    }

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
            UnifiedStopArms::all_false(),
            Vec::new(),
            None,
        );

        assert_eq!(audio.len(), 1, "expected 1 audio file");
        assert_eq!(audio[0].format, "WAV");
        assert_eq!(pdf.len(), 1, "expected 1 pdf");
        assert_eq!(daw.len(), 1, "expected 1 daw");
        assert_eq!(daw[0].format, "ALS");
        assert_eq!(
            preset.len(),
            1,
            "expected 1 preset (fxp) — .mid routes to MIDI bucket"
        );
    }

    #[test]
    fn test_walk_unified_follows_symlink_to_als() {
        let tmp = TestDir::new("symlink_als");
        let root = tmp.path.clone();
        touch(&root.join("real.als"), b"<?xml");
        let link = root.join("link.als");
        if !try_symlink(&root.join("real.als"), &link) {
            return;
        }

        let spec = UnifiedSpec {
            daw_roots: vec![root.clone()],
            ..Default::default()
        };

        let mut daw = Vec::new();
        walk_unified(
            &spec,
            &mut |batch, _counts| {
                if let ClassifiedBatch::Daw(b) = batch {
                    daw.extend(b);
                }
            },
            UnifiedStopArms::all_false(),
            Vec::new(),
            None,
        );

        assert_eq!(
            daw.len(),
            2,
            "real file + symlink path both classify as DAW"
        );
        assert!(daw.iter().any(|d| d.path.ends_with("link.als")));
        assert!(daw.iter().any(|d| d.path.ends_with("real.als")));
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
            UnifiedStopArms::all_false(),
            Vec::new(),
            None,
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
        touch(&root.join("bower_components/legacy.wav"), b"RIFF");
        touch(&root.join("target/debug.wav"), b"RIFF");
        touch(&root.join("htmlcov/cov.wav"), b"RIFF");
        touch(&root.join("coverage/lcov.wav"), b"RIFF");
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
            UnifiedStopArms::all_false(),
            Vec::new(),
            None,
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
            UnifiedStopArms::all_false(),
            Vec::new(),
            None,
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
        walk_unified(
            &spec,
            &mut |_, _| {},
            UnifiedStopArms::all_true(),
            Vec::new(),
            None,
        );
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
            UnifiedStopArms::all_false(),
            Vec::new(),
            None,
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
            UnifiedStopArms::all_false(),
            Vec::new(),
            None,
        );
        assert_eq!(batches, 0, "empty spec produces no output");
    }

    #[test]
    fn test_incremental_skips_unchanged_directory_tree() {
        let tmp = TestDir::new("incskip");
        let root = tmp.path.clone();
        touch(&root.join("a.wav"), b"RIFF");
        let key = directory_incremental_key(&root);
        let m = dir_mtime_secs(&root);
        let mut snap = HashMap::new();
        snap.insert(key, m);
        let spec = UnifiedSpec {
            audio_roots: vec![root.clone()],
            ..Default::default()
        };
        let mut count = 0usize;
        walk_unified(
            &spec,
            &mut |batch, _| {
                if let ClassifiedBatch::Audio(b) = batch {
                    count += b.len();
                }
            },
            UnifiedStopArms::all_false(),
            Vec::new(),
            Some(Arc::new(IncrementalDirState::new(snap))),
        );
        assert_eq!(
            count, 0,
            "snapshot matching current dir mtime skips subtree"
        );
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
        assert_eq!(
            normalize_macos_path(p),
            PathBuf::from("/home/user/Projects")
        );
    }
}
