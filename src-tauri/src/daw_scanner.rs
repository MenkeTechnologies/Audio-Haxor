//! DAW project file scanner supporting 14+ DAW formats.
//!
//! Discovers Ableton, Logic, FL Studio, REAPER, Cubase, Pro Tools,
//! Bitwig, Studio One, Reason, Audacity, GarageBand, Ardour, and
//! DAWproject files. Handles macOS package directories (.logicx, .band)
//! and validates GarageBand bundles by internal structure.

use crate::history::DawProject;
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

/// File extensions for DAW project files.
/// Includes both single-file formats and macOS bundle/package formats.
const DAW_EXTENSIONS: &[&str] = &[
    ".als",        // Ableton Live Set
    ".logicx",     // Logic Pro X (macOS package)
    ".flp",        // FL Studio
    ".cpr",        // Cubase Project
    ".npr",        // Nuendo Project
    ".bwproject",  // Bitwig Studio
    ".rpp",        // REAPER
    ".rpp-bak",    // REAPER backup
    ".ptx",        // Pro Tools (v10+)
    ".ptf",        // Pro Tools (legacy)
    ".song",       // Studio One
    ".reason",     // Reason
    ".aup",        // Audacity (legacy)
    ".aup3",       // Audacity 3
    ".band",       // GarageBand (macOS package)
    ".ardour",     // Ardour
    ".dawproject", // DAWproject (open standard)
];

/// Extensions that are macOS packages (directories with these extensions should
/// be treated as files, not recursed into).
const PACKAGE_EXTENSIONS: &[&str] = &[".logicx", ".band"];

/// Plugin bundle extensions — directories with these extensions should never
/// be recursed into by the DAW scanner. They contain plugin code and presets,
/// not DAW projects.
const PLUGIN_BUNDLE_EXTENSIONS: &[&str] = &[
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

/// Additional directories to skip when not including Ableton backups/crashes.
/// Ableton stores auto-saved backup Live Sets in a "Backup" folder
/// and crash recovery sets in a "Crash" folder inside each project directory.
const BACKUP_DIRS: &[&str] = &["Backup", "Crash"];

pub fn format_size(bytes: u64) -> String {
    crate::format_size(bytes)
}

fn get_directory_size(path: &Path) -> u64 {
    get_directory_size_depth(path, 0)
}

fn get_directory_size_depth(path: &Path, depth: u32) -> u64 {
    if depth > 10 {
        return 0;
    } // cap recursion to limit FD usage
    let mut total = 0u64;
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                total += get_directory_size_depth(&p, depth + 1);
            } else if let Ok(meta) = fs::metadata(&p) {
                total += meta.len();
            }
        }
    }
    total
}

pub fn ext_matches(path: &Path) -> Option<String> {
    let name = path.file_name()?.to_string_lossy().to_lowercase();
    for ext in DAW_EXTENSIONS {
        if name.ends_with(ext) {
            return Some(ext[1..].to_uppercase());
        }
    }
    None
}

pub fn is_package_ext(path: &Path) -> bool {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    PACKAGE_EXTENSIONS.iter().any(|ext| name.ends_with(ext))
}

/// Validate that a .band directory is actually a GarageBand project.
/// Checks for `projectData` binary plist (must start with "bplist") AND
/// requires at least one other GarageBand-specific marker to eliminate
/// false positives from other macOS bundles that happen to use .band extension.
fn is_valid_band_package(path: &Path) -> bool {
    let pd = path.join("projectData");
    if !pd.exists() {
        return false;
    }
    // Verify projectData is a binary plist (starts with "bplist")
    if let Ok(mut f) = fs::File::open(&pd) {
        use std::io::Read;
        let mut magic = [0u8; 6];
        if f.read_exact(&mut magic).is_err() || &magic != b"bplist" {
            return false;
        }
    } else {
        return false;
    }
    // Require at least one GarageBand-specific subdirectory
    path.join("Media").is_dir()
        || path.join("Output").is_dir()
        || path.join("Freeze Files").is_dir()
}

pub fn daw_name_for_format(format: &str) -> &'static str {
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

pub fn get_daw_roots() -> Vec<PathBuf> {
    let home = dirs::home_dir().unwrap_or_default();
    let mut roots = Vec::new();

    #[cfg(target_os = "macos")]
    {
        roots.push(home.clone());
        roots.push(PathBuf::from("/Applications"));
        if let Ok(vols) = fs::read_dir("/Volumes") {
            for entry in vols.flatten() {
                let path = entry.path();
                if path.is_dir() || path.is_symlink() {
                    roots.push(path);
                }
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        roots.push(home.clone());
        roots.push(PathBuf::from(
            std::env::var("ProgramFiles").unwrap_or_else(|_| "C:\\Program Files".into()),
        ));
        roots.push(PathBuf::from(
            std::env::var("ProgramFiles(x86)").unwrap_or_else(|_| "C:\\Program Files (x86)".into()),
        ));
        for c in b'C'..=b'Z' {
            let drive = format!("{}:\\", c as char);
            if Path::new(&drive).exists() {
                roots.push(PathBuf::from(drive));
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        roots.push(home.clone());
        roots.push(PathBuf::from("/usr/share"));
        roots.push(PathBuf::from("/usr/local/share"));
    }

    roots.sort();
    roots.dedup();
    roots.into_iter().filter(|r| r.exists()).collect()
}

pub fn walk_for_daw(
    roots: &[PathBuf],
    on_batch: &mut dyn FnMut(&[DawProject], usize),
    should_stop: &(dyn Fn() -> bool + Sync),
    exclude: Option<HashSet<String>>,
    include_backups: bool,
    active_dirs: Option<Arc<Mutex<Vec<String>>>>,
    incremental: Option<Arc<IncrementalDirState>>,
) {
    let batch_size = 100;
    let stop = Arc::new(AtomicBool::new(false));
    let found = Arc::new(AtomicUsize::new(0));
    let active = active_dirs.unwrap_or_else(|| Arc::new(Mutex::new(Vec::new())));
    let (tx, rx) = std::sync::mpsc::sync_channel::<Vec<DawProject>>(256);
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
                    include_backups,
                    &active,
                    incremental.clone(),
                );
            });
        });
        drop(pool);
    });

    // Stream results to callback as they arrive, checking stop frequently
    let mut total_found = 0usize;
    loop {
        if should_stop() {
            stop.store(true, Ordering::Relaxed);
            while rx.try_recv().is_ok() {}
            break;
        }
        let projects = match rx.recv_timeout(std::time::Duration::from_millis(10)) {
            Ok(p) => p,
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => continue,
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
        };
        total_found += projects.len();
        on_batch(&projects, total_found);
    }
}

#[allow(clippy::too_many_arguments)]
fn walk_dir_parallel(
    dir: &Path,
    depth: u32,
    visited: &Arc<Mutex<HashSet<PathBuf>>>,
    tx: &std::sync::mpsc::SyncSender<Vec<DawProject>>,
    found: &Arc<AtomicUsize>,
    batch_size: usize,
    stop: &Arc<AtomicBool>,
    exclude: &Arc<HashSet<String>>,
    include_backups: bool,
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
        // Dedup on canonical path (resolves symlinks), fall back to original
        let key = canon.unwrap_or_else(|| orig.clone());
        if !vis.insert(key) {
            return;
        }
        vis.insert(orig); // also mark original to prevent re-entry via different route
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

    let mut files_and_packages = Vec::new();
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
        if !include_backups && BACKUP_DIRS.contains(&name_str.as_ref()) {
            continue;
        }
        // Cached d_type from readdir — no extra stat() syscall per entry.
        let ft = match entry.file_type() {
            Ok(f) => f,
            Err(_) => continue,
        };
        let path = entry.path();
        if ft.is_dir() {
            // Skip plugin bundles entirely — they contain presets, not DAW projects
            let name_lower = name_str.to_lowercase();
            if PLUGIN_BUNDLE_EXTENSIONS
                .iter()
                .any(|ext| name_lower.ends_with(ext))
            {
                continue;
            }
            if is_package_ext(&path) {
                files_and_packages.push((path, dir.to_path_buf(), true));
            } else {
                subdirs.push(path);
            }
        } else if ft.is_file() {
            files_and_packages.push((path, dir.to_path_buf(), false));
        }
    }

    let mut batch = Vec::new();
    for (path, parent, is_pkg) in files_and_packages {
        if exclude.contains(&path.to_string_lossy().to_string()) {
            continue;
        }
        if let Some(format) = ext_matches(&path) {
            // .band is ONLY valid as a GarageBand package directory, never as a plain file
            if format == "BAND" && (!is_pkg || !is_valid_band_package(&path)) {
                continue;
            }
            let (size, modified) = if is_pkg {
                let sz = get_directory_size(&path);
                let mod_str = fs::metadata(&path)
                    .ok()
                    .and_then(|m| m.modified().ok())
                    .map(|t| {
                        let dt: chrono::DateTime<chrono::Utc> = t.into();
                        dt.format("%Y-%m-%d").to_string()
                    })
                    .unwrap_or_default();
                (sz, mod_str)
            } else if let Ok(meta) = fs::metadata(&path) {
                let mod_str = meta
                    .modified()
                    .ok()
                    .map(|t| {
                        let dt: chrono::DateTime<chrono::Utc> = t.into();
                        dt.format("%Y-%m-%d").to_string()
                    })
                    .unwrap_or_default();
                (meta.len(), mod_str)
            } else {
                continue;
            };

            let project_name = path
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            let daw = daw_name_for_format(&format).to_string();

            batch.push(DawProject {
                name: project_name,
                path: path.to_string_lossy().to_string(),
                directory: parent.to_string_lossy().to_string(),
                format,
                daw,
                size,
                size_formatted: format_size(size),
                modified,
            });
            found.fetch_add(1, Ordering::Relaxed);

            if batch.len() >= batch_size {
                let _ = tx.send(batch);
                batch = Vec::new();
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
            include_backups,
            active_dirs,
            incremental.clone(),
        );
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use std::fs;
    use std::slice::from_ref;

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(500), "500.0 B");
        assert_eq!(format_size(1024), "1.0 KB");
        assert_eq!(format_size(1_048_576), "1.0 MB");
        assert_eq!(format_size(1_073_741_824), "1.0 GB");
    }

    #[test]
    fn test_daw_extensions_complete() {
        for ext in &[
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
        ] {
            assert!(
                DAW_EXTENSIONS.contains(ext),
                "DAW_EXTENSIONS should contain {}",
                ext
            );
        }
    }

    #[test]
    fn test_ext_matches() {
        assert_eq!(ext_matches(Path::new("song.als")), Some("ALS".into()));
        assert_eq!(ext_matches(Path::new("song.flp")), Some("FLP".into()));
        assert_eq!(
            ext_matches(Path::new("project.logicx")),
            Some("LOGICX".into())
        );
        assert_eq!(ext_matches(Path::new("not_a_daw.txt")), None);
    }

    #[test]
    fn test_ext_matches_ardour_and_dawproject() {
        assert_eq!(
            ext_matches(Path::new("session.ardour")),
            Some("ARDOUR".into())
        );
        assert_eq!(
            ext_matches(Path::new("openstd.dawproject")),
            Some("DAWPROJECT".into())
        );
        assert_eq!(daw_name_for_format("ARDOUR"), "Ardour");
        assert_eq!(daw_name_for_format("DAWPROJECT"), "DAWproject");
    }

    #[test]
    fn test_is_package_ext() {
        assert!(is_package_ext(Path::new("MySong.logicx")));
        assert!(is_package_ext(Path::new("MySong.band")));
        assert!(!is_package_ext(Path::new("MySong.als")));
        assert!(!is_package_ext(Path::new("MySong.flp")));
    }

    #[test]
    fn test_is_package_ext_dawproject_is_file_not_bundle() {
        assert!(
            !is_package_ext(Path::new("project.dawproject")),
            ".dawproject is a zip file, not a macOS package dir"
        );
    }

    #[test]
    fn test_ext_matches_reaper_backup_suffix_before_plain_rpp() {
        assert_eq!(
            ext_matches(Path::new("Mixdown.rpp-bak")),
            Some("RPP-BAK".into())
        );
        assert_eq!(ext_matches(Path::new("Mixdown.rpp")), Some("RPP".into()));
    }

    #[test]
    fn test_ext_matches_case_insensitive_file_name() {
        assert_eq!(ext_matches(Path::new("LIVE.SET.ALS")), Some("ALS".into()));
    }

    #[test]
    fn test_ext_matches_audacity_aup3_suffix() {
        assert_eq!(ext_matches(Path::new("podcast.aup3")), Some("AUP3".into()));
    }

    #[test]
    fn test_ext_matches_aup3_not_matched_as_aup_prefix() {
        // `.aup` appears before `.aup3` in `DAW_EXTENSIONS`; `ends_with` must still pick the longer suffix.
        assert_eq!(ext_matches(Path::new("session.aup3")), Some("AUP3".into()));
        assert_eq!(ext_matches(Path::new("legacy.aup")), Some("AUP".into()));
    }

    #[test]
    fn test_ext_matches_pro_tools_ptf_vs_ptx() {
        assert_eq!(ext_matches(Path::new("session.ptf")), Some("PTF".into()));
        assert_eq!(ext_matches(Path::new("session.ptx")), Some("PTX".into()));
    }

    #[test]
    fn test_daw_name_for_format_all_known_tokens() {
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
        assert_eq!(daw_name_for_format("XYZ"), "Unknown");
    }

    #[test]
    fn test_get_daw_roots_not_empty() {
        let roots = get_daw_roots();
        assert!(
            roots.iter().any(|r| r.exists()),
            "get_daw_roots should return at least one existing path"
        );
    }

    #[test]
    fn test_walk_for_daw_empty_dir() {
        let tmp = std::env::temp_dir().join("upum_test_daw_walk_empty");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        let mut total = 0usize;
        walk_for_daw(
            from_ref(&tmp),
            &mut |_batch, count| {
                total = count;
            },
            &|| false,
            None,
            false,
            None,
            None,
        );
        assert_eq!(total, 0);
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_walk_for_daw_finds_files() {
        let tmp = std::env::temp_dir().join("upum_test_daw_walk_finds");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        fs::write(tmp.join("mysong.als"), b"fake ableton data").unwrap();
        fs::write(tmp.join("test.txt"), b"not a daw project").unwrap();

        let mut found = Vec::new();
        walk_for_daw(
            from_ref(&tmp),
            &mut |batch, _count| {
                found.extend_from_slice(batch);
            },
            &|| false,
            None,
            false,
            None,
            None,
        );
        assert_eq!(found.len(), 1);
        assert!(found[0].path.contains("mysong.als"));
        assert_eq!(found[0].format, "ALS");
        assert_eq!(found[0].daw, "Ableton Live");
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_walk_for_daw_exclude_full_path_skips_file() {
        let tmp = std::env::temp_dir().join("upum_test_daw_walk_exclude_path");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        fs::write(tmp.join("keep.als"), b"fake").unwrap();
        fs::write(tmp.join("skip.als"), b"fake").unwrap();
        let mut ex = HashSet::new();
        ex.insert(tmp.join("skip.als").to_string_lossy().into_owned());

        let mut found = Vec::new();
        walk_for_daw(
            from_ref(&tmp),
            &mut |batch, _count| {
                found.extend_from_slice(batch);
            },
            &|| false,
            Some(ex),
            false,
            None,
            None,
        );
        assert_eq!(found.len(), 1);
        assert!(found[0].path.contains("keep.als"));
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_walk_for_daw_finds_multiple_formats() {
        let tmp = std::env::temp_dir().join("upum_test_daw_walk_multi");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        fs::write(tmp.join("song1.als"), b"ableton").unwrap();
        fs::write(tmp.join("song2.flp"), b"fl studio").unwrap();
        fs::write(tmp.join("song3.rpp"), b"reaper").unwrap();

        let mut found = Vec::new();
        walk_for_daw(
            from_ref(&tmp),
            &mut |batch, _count| {
                found.extend_from_slice(batch);
            },
            &|| false,
            None,
            false,
            None,
            None,
        );
        assert_eq!(found.len(), 3);
        let formats: Vec<&str> = found.iter().map(|d| d.format.as_str()).collect();
        assert!(formats.contains(&"ALS"));
        assert!(formats.contains(&"FLP"));
        assert!(formats.contains(&"RPP"));
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_walk_for_daw_stop() {
        let tmp = std::env::temp_dir().join("upum_test_daw_walk_stop");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        fs::write(tmp.join("test.als"), b"fake data").unwrap();

        let mut found = Vec::new();
        walk_for_daw(
            from_ref(&tmp),
            &mut |batch, _count| {
                found.extend_from_slice(batch);
            },
            &|| true,
            None,
            false,
            None,
            None,
        );
        assert_eq!(found.len(), 0);
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_walk_for_daw_skips_dotdirs() {
        let tmp = std::env::temp_dir().join("upum_test_daw_walk_dotdirs");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(tmp.join(".hidden")).unwrap();
        fs::create_dir_all(tmp.join("visible")).unwrap();
        fs::write(tmp.join(".hidden").join("test.als"), b"hidden").unwrap();
        fs::write(tmp.join("visible").join("test.als"), b"visible").unwrap();

        let mut found = Vec::new();
        walk_for_daw(
            from_ref(&tmp),
            &mut |batch, _count| {
                found.extend_from_slice(batch);
            },
            &|| false,
            None,
            false,
            None,
            None,
        );
        assert_eq!(found.len(), 1);
        assert!(found[0].path.contains("visible"));
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_walk_for_daw_package_dirs() {
        let tmp = std::env::temp_dir().join("upum_test_daw_walk_pkg");
        let _ = fs::remove_dir_all(&tmp);

        // Create a .logicx package directory
        let logicx = tmp.join("MySong.logicx");
        fs::create_dir_all(&logicx).unwrap();
        fs::write(logicx.join("projectdata"), b"logic data").unwrap();

        // Create a .band package directory (must have bplist projectData + Media dir)
        let band = tmp.join("MyBand.band");
        fs::create_dir_all(band.join("Media")).unwrap();
        fs::write(band.join("projectData"), b"bplist00fake").unwrap();

        let mut found = Vec::new();
        walk_for_daw(
            from_ref(&tmp),
            &mut |batch, _count| {
                found.extend_from_slice(batch);
            },
            &|| false,
            None,
            false,
            None,
            None,
        );
        assert_eq!(found.len(), 2);
        let formats: Vec<&str> = found.iter().map(|d| d.format.as_str()).collect();
        assert!(formats.contains(&"LOGICX"));
        assert!(formats.contains(&"BAND"));
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_walk_for_daw_batching() {
        let tmp = std::env::temp_dir().join("upum_test_daw_walk_batching");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        for i in 0..120 {
            fs::write(tmp.join(format!("song_{}.als", i)), b"data").unwrap();
        }

        let mut batch_call_count = 0usize;
        walk_for_daw(
            from_ref(&tmp),
            &mut |_batch, _count| {
                batch_call_count += 1;
            },
            &|| false,
            None,
            false,
            None,
            None,
        );
        assert!(
            batch_call_count >= 2,
            "Expected on_batch called at least twice for 120 files with batch_size=100, got {}",
            batch_call_count
        );
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_walk_for_daw_deduplicates_symlinks() {
        let tmp = std::env::temp_dir().join("upum_test_daw_walk_symlinks");
        let _ = fs::remove_dir_all(&tmp);
        let subdir = tmp.join("originals");
        fs::create_dir_all(&subdir).unwrap();
        fs::write(subdir.join("test.als"), b"data").unwrap();

        #[cfg(unix)]
        {
            let link = tmp.join("linked");
            std::os::unix::fs::symlink(&subdir, &link).unwrap();

            let mut found = Vec::new();
            walk_for_daw(
                from_ref(&tmp),
                &mut |batch, _count| {
                    found.extend_from_slice(batch);
                },
                &|| false,
                None,
                false,
                None,
                None,
            );
            let count = found.iter().filter(|d| d.name == "test").count();
            assert_eq!(
                count, 1,
                "test.als should be found exactly once despite symlink, found {}",
                count
            );
        }
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_walk_for_daw_deduplicates_overlapping_roots() {
        // Parent and child dir as separate roots — files in child should be found once
        let tmp = std::env::temp_dir().join("upum_test_daw_overlap");
        let _ = fs::remove_dir_all(&tmp);
        let child = tmp.join("sub");
        fs::create_dir_all(&child).unwrap();
        fs::write(child.join("overlap.rpp"), b"data").unwrap();
        fs::write(tmp.join("top.als"), b"data").unwrap();

        let mut found = Vec::new();
        walk_for_daw(
            &[tmp.clone(), child.clone()],
            &mut |batch, _| found.extend_from_slice(batch),
            &|| false,
            None,
            false,
            None,
            None,
        );
        let overlap_count = found.iter().filter(|d| d.name == "overlap").count();
        assert_eq!(
            overlap_count, 1,
            "overlap.rpp should be found once despite overlapping roots, got {}",
            overlap_count
        );
        assert!(
            found.iter().any(|d| d.name == "top"),
            "top.als should be found"
        );
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_walk_for_daw_consistent_counts() {
        // Run the same scan twice — should get identical results
        let tmp = std::env::temp_dir().join("upum_test_daw_consistent");
        let _ = fs::remove_dir_all(&tmp);
        for i in 0..5 {
            let d = tmp.join(format!("dir{}", i));
            fs::create_dir_all(&d).unwrap();
            fs::write(d.join(format!("project{}.als", i)), b"data").unwrap();
            fs::write(d.join(format!("project{}.rpp", i)), b"data").unwrap();
        }

        let mut count1 = 0;
        walk_for_daw(
            &[tmp.clone()],
            &mut |batch, _| count1 += batch.len(),
            &|| false,
            None,
            false,
            None,
            None,
        );
        let mut count2 = 0;
        walk_for_daw(
            &[tmp.clone()],
            &mut |batch, _| count2 += batch.len(),
            &|| false,
            None,
            false,
            None,
            None,
        );

        assert_eq!(
            count1, count2,
            "two scans of same dirs should find same count: {} vs {}",
            count1, count2
        );
        assert_eq!(count1, 10, "should find 10 projects");
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_walk_for_daw_skips_backup_dirs() {
        let tmp = std::env::temp_dir().join("upum_test_daw_backup");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(tmp.join("Backup")).unwrap();
        fs::create_dir_all(tmp.join("Crash")).unwrap();
        fs::write(tmp.join("main.als"), b"data").unwrap();
        fs::write(tmp.join("Backup").join("backup.als"), b"data").unwrap();
        fs::write(tmp.join("Crash").join("crash.als"), b"data").unwrap();

        let mut found = Vec::new();
        walk_for_daw(
            from_ref(&tmp),
            &mut |batch, _| {
                found.extend_from_slice(batch);
            },
            &|| false,
            None,
            false, // include_backups = false
            None,
            None,
        );
        // Should only find main.als, not backup or crash
        assert_eq!(
            found.len(),
            1,
            "Expected 1 (main.als), found {}: {:?}",
            found.len(),
            found.iter().map(|f| &f.name).collect::<Vec<_>>()
        );
        assert_eq!(found[0].name, "main");
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_walk_for_daw_includes_backup_when_enabled() {
        let tmp = std::env::temp_dir().join("upum_test_daw_backup_on");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(tmp.join("Backup")).unwrap();
        fs::write(tmp.join("main.als"), b"data").unwrap();
        fs::write(tmp.join("Backup").join("backup.als"), b"data").unwrap();

        let mut found = Vec::new();
        walk_for_daw(
            from_ref(&tmp),
            &mut |batch, _| {
                found.extend_from_slice(batch);
            },
            &|| false,
            None,
            true, // include_backups = true
            None,
            None,
        );
        assert_eq!(found.len(), 2);
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_walk_for_daw_skips_plugin_bundles() {
        let tmp = std::env::temp_dir().join("upum_test_daw_plugin_skip");
        let _ = fs::remove_dir_all(&tmp);
        // Create a .vst3 directory with .als inside — should NOT be found
        fs::create_dir_all(tmp.join("Plugin.vst3").join("Contents")).unwrap();
        fs::write(
            tmp.join("Plugin.vst3").join("Contents").join("fake.als"),
            b"data",
        )
        .unwrap();
        // Create a real .als outside
        fs::write(tmp.join("real.als"), b"data").unwrap();

        let mut found = Vec::new();
        walk_for_daw(
            from_ref(&tmp),
            &mut |batch, _| {
                found.extend_from_slice(batch);
            },
            &|| false,
            None,
            false,
            None,
            None,
        );
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "real");
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_band_plain_file_rejected() {
        let tmp = std::env::temp_dir().join("upum_test_band_file");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        // .band as a plain file should be rejected
        fs::write(tmp.join("preset.band"), b"not a garageband project").unwrap();

        let mut found = Vec::new();
        walk_for_daw(
            from_ref(&tmp),
            &mut |batch, _| {
                found.extend_from_slice(batch);
            },
            &|| false,
            None,
            false,
            None,
            None,
        );
        assert_eq!(found.len(), 0, "Plain .band file should be rejected");
        let _ = fs::remove_dir_all(&tmp);
    }
}
