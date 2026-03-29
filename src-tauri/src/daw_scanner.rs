use crate::history::DawProject;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

/// File extensions for DAW project files.
/// Includes both single-file formats and macOS bundle/package formats.
const DAW_EXTENSIONS: &[&str] = &[
    ".als",        // Ableton Live Set
    ".alp",        // Ableton Live Pack
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

fn get_directory_size(path: &Path) -> u64 {
    let mut total = 0u64;
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                total += get_directory_size(&p);
            } else if let Ok(meta) = fs::metadata(&p) {
                total += meta.len();
            }
        }
    }
    total
}

fn ext_matches(path: &Path) -> Option<String> {
    let name = path.file_name()?.to_string_lossy().to_lowercase();
    for ext in DAW_EXTENSIONS {
        if name.ends_with(ext) {
            return Some(ext[1..].to_uppercase());
        }
    }
    None
}

fn is_package_ext(path: &Path) -> bool {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    PACKAGE_EXTENSIONS.iter().any(|ext| name.ends_with(ext))
}

fn daw_name_for_format(format: &str) -> &'static str {
    match format {
        "ALS" | "ALP" => "Ableton Live",
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
    should_stop: &dyn Fn() -> bool,
) {
    let mut visited = HashSet::new();
    let mut batch = Vec::new();
    let mut found = 0usize;
    let batch_size = 50;

    for root in roots {
        if should_stop() {
            break;
        }
        walk_dir(
            root,
            0,
            &mut visited,
            &mut batch,
            &mut found,
            batch_size,
            on_batch,
            should_stop,
        );
    }

    if !batch.is_empty() {
        on_batch(&batch, found);
    }
}

#[allow(clippy::too_many_arguments)]
fn walk_dir(
    dir: &Path,
    depth: u32,
    visited: &mut HashSet<PathBuf>,
    batch: &mut Vec<DawProject>,
    found: &mut usize,
    batch_size: usize,
    on_batch: &mut dyn FnMut(&[DawProject], usize),
    should_stop: &dyn Fn() -> bool,
) {
    if depth > 30 || should_stop() {
        return;
    }

    let real_dir = match fs::canonicalize(dir) {
        Ok(p) => p,
        Err(_) => return,
    };
    if !visited.insert(real_dir) {
        return;
    }

    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        if should_stop() {
            return;
        }

        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if name_str.starts_with('.') || SKIP_DIRS.contains(&name_str.as_ref()) {
            continue;
        }

        let path = entry.path();

        if path.is_dir() {
            // Check if this directory is a macOS package (e.g. .logicx, .band)
            if is_package_ext(&path) {
                if let Some(format) = ext_matches(&path) {
                    let size = get_directory_size(&path);
                    let modified = fs::metadata(&path)
                        .ok()
                        .and_then(|m| m.modified().ok())
                        .map(|t| {
                            let dt: chrono::DateTime<chrono::Utc> = t.into();
                            dt.format("%Y-%m-%d").to_string()
                        })
                        .unwrap_or_default();

                    let project_name = path
                        .file_stem()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_default();

                    let daw = daw_name_for_format(&format).to_string();

                    batch.push(DawProject {
                        name: project_name,
                        path: path.to_string_lossy().to_string(),
                        directory: dir.to_string_lossy().to_string(),
                        format,
                        daw,
                        size,
                        size_formatted: format_size(size),
                        modified,
                    });
                    *found += 1;

                    if batch.len() >= batch_size {
                        on_batch(batch, *found);
                        batch.clear();
                    }
                }
                // Don't recurse into package directories
                continue;
            }

            walk_dir(
                &path,
                depth + 1,
                visited,
                batch,
                found,
                batch_size,
                on_batch,
                should_stop,
            );
        } else if path.is_file() {
            if let Some(format) = ext_matches(&path) {
                if let Ok(meta) = fs::metadata(&path) {
                    let project_name = path
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

                    let daw = daw_name_for_format(&format).to_string();

                    batch.push(DawProject {
                        name: project_name,
                        path: path.to_string_lossy().to_string(),
                        directory: dir.to_string_lossy().to_string(),
                        format,
                        daw,
                        size: meta.len(),
                        size_formatted: format_size(meta.len()),
                        modified,
                    });
                    *found += 1;

                    if batch.len() >= batch_size {
                        on_batch(batch, *found);
                        batch.clear();
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

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
            ".bwproject",
            ".rpp",
            ".ptx",
            ".song",
        ] {
            assert!(
                DAW_EXTENSIONS.contains(ext),
                "DAW_EXTENSIONS should contain {}",
                ext
            );
        }
    }

    #[test]
    fn test_skip_dirs_complete() {
        for dir in &["node_modules", ".git", ".Trash"] {
            assert!(SKIP_DIRS.contains(dir), "SKIP_DIRS should contain {}", dir);
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
    fn test_is_package_ext() {
        assert!(is_package_ext(Path::new("MySong.logicx")));
        assert!(is_package_ext(Path::new("MySong.band")));
        assert!(!is_package_ext(Path::new("MySong.als")));
        assert!(!is_package_ext(Path::new("MySong.flp")));
    }

    #[test]
    fn test_daw_name_for_format() {
        assert_eq!(daw_name_for_format("ALS"), "Ableton Live");
        assert_eq!(daw_name_for_format("LOGICX"), "Logic Pro");
        assert_eq!(daw_name_for_format("FLP"), "FL Studio");
        assert_eq!(daw_name_for_format("CPR"), "Cubase");
        assert_eq!(daw_name_for_format("RPP"), "REAPER");
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
            &[tmp.clone()],
            &mut |_batch, count| {
                total = count;
            },
            &|| false,
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
            &[tmp.clone()],
            &mut |batch, _count| {
                found.extend_from_slice(batch);
            },
            &|| false,
        );
        assert_eq!(found.len(), 1);
        assert!(found[0].path.contains("mysong.als"));
        assert_eq!(found[0].format, "ALS");
        assert_eq!(found[0].daw, "Ableton Live");
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
            &[tmp.clone()],
            &mut |batch, _count| {
                found.extend_from_slice(batch);
            },
            &|| false,
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
            &[tmp.clone()],
            &mut |batch, _count| {
                found.extend_from_slice(batch);
            },
            &|| true,
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
            &[tmp.clone()],
            &mut |batch, _count| {
                found.extend_from_slice(batch);
            },
            &|| false,
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

        // Create a .band package directory
        let band = tmp.join("MyBand.band");
        fs::create_dir_all(&band).unwrap();
        fs::write(band.join("projectdata"), b"band data").unwrap();

        let mut found = Vec::new();
        walk_for_daw(
            &[tmp.clone()],
            &mut |batch, _count| {
                found.extend_from_slice(batch);
            },
            &|| false,
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

        for i in 0..60 {
            fs::write(tmp.join(format!("song_{}.als", i)), b"data").unwrap();
        }

        let mut batch_call_count = 0usize;
        walk_for_daw(
            &[tmp.clone()],
            &mut |_batch, _count| {
                batch_call_count += 1;
            },
            &|| false,
        );
        assert!(
            batch_call_count >= 2,
            "Expected on_batch called at least twice for 60 files with batch_size=50, got {}",
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
                &[tmp.clone()],
                &mut |batch, _count| {
                    found.extend_from_slice(batch);
                },
                &|| false,
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
}
