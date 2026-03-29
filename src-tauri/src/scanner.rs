use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInfo {
    pub name: String,
    pub path: String,
    #[serde(rename = "type")]
    pub plugin_type: String,
    pub version: String,
    pub manufacturer: String,
    #[serde(rename = "manufacturerUrl")]
    pub manufacturer_url: Option<String>,
    pub size: String,
    pub modified: String,
}

pub fn get_vst_directories() -> Vec<String> {
    let home = dirs::home_dir().unwrap_or_default();
    let mut dirs_list: Vec<PathBuf> = Vec::new();

    #[cfg(target_os = "macos")]
    {
        dirs_list.extend([
            PathBuf::from("/Library/Audio/Plug-Ins/VST"),
            PathBuf::from("/Library/Audio/Plug-Ins/VST3"),
            PathBuf::from("/Library/Audio/Plug-Ins/Components"),
            home.join("Library/Audio/Plug-Ins/VST"),
            home.join("Library/Audio/Plug-Ins/VST3"),
            home.join("Library/Audio/Plug-Ins/Components"),
        ]);
    }

    #[cfg(target_os = "windows")]
    {
        let pf = std::env::var("ProgramFiles").unwrap_or_else(|_| "C:\\Program Files".into());
        let pf86 =
            std::env::var("ProgramFiles(x86)").unwrap_or_else(|_| "C:\\Program Files (x86)".into());
        dirs_list.extend([
            PathBuf::from(&pf).join("Common Files").join("VST3"),
            PathBuf::from(&pf).join("VSTPlugins"),
            PathBuf::from(&pf).join("Steinberg").join("VSTPlugins"),
            PathBuf::from(&pf86).join("Common Files").join("VST3"),
            PathBuf::from(&pf86).join("VSTPlugins"),
            PathBuf::from(&pf86)
                .join("Steinberg")
                .join("VSTPlugins"),
        ]);
    }

    #[cfg(target_os = "linux")]
    {
        dirs_list.extend([
            PathBuf::from("/usr/lib/vst"),
            PathBuf::from("/usr/lib/vst3"),
            PathBuf::from("/usr/local/lib/vst"),
            PathBuf::from("/usr/local/lib/vst3"),
            home.join(".vst"),
            home.join(".vst3"),
        ]);
    }

    dirs_list
        .into_iter()
        .filter(|d| d.exists())
        .map(|d| d.to_string_lossy().to_string())
        .collect()
}

fn get_plugin_type(ext: &str) -> &str {
    match ext {
        ".vst" => "VST2",
        ".vst3" => "VST3",
        ".component" => "AU",
        ".dll" => "VST2",
        _ => "Unknown",
    }
}

fn get_directory_size(dir: &Path) -> u64 {
    let mut size = 0u64;
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                size += get_directory_size(&path);
            } else if let Ok(meta) = fs::metadata(&path) {
                size += meta.len();
            }
        }
    }
    size
}

fn format_size(bytes: u64) -> String {
    if bytes == 0 {
        return "0 B".into();
    }
    let units = ["B", "KB", "MB", "GB"];
    let i = (bytes as f64).log(1024.0).floor() as usize;
    let i = i.min(units.len() - 1);
    format!("{:.1} {}", bytes as f64 / 1024f64.powi(i as i32), units[i])
}

#[cfg(target_os = "macos")]
fn read_plist_info(plugin_path: &Path) -> (Option<String>, Option<String>, Option<String>) {
    let plist_path = plugin_path.join("Contents").join("Info.plist");
    if !plist_path.exists() {
        return (None, None, None);
    }

    let plist_val = match plist::Value::from_file(&plist_path) {
        Ok(v) => v,
        Err(_) => return (None, None, None),
    };

    let dict = match plist_val.as_dictionary() {
        Some(d) => d,
        None => return (None, None, None),
    };

    let version = dict
        .get("CFBundleShortVersionString")
        .and_then(|v| v.as_string())
        .map(|s| s.to_string());

    let mut manufacturer: Option<String> = None;
    let mut manufacturer_url: Option<String> = None;

    if let Some(bundle_id) = dict.get("CFBundleIdentifier").and_then(|v| v.as_string()) {
        let parts: Vec<&str> = bundle_id.split('.').collect();
        if parts.len() >= 2 {
            let domain = parts[1];
            let mut mfg = domain.to_string();
            if let Some(first) = mfg.get_mut(0..1) {
                first.make_ascii_uppercase();
            }
            manufacturer = Some(mfg);

            let lower = domain.to_lowercase();
            if lower != "apple" && lower.len() > 1 {
                manufacturer_url = Some(format!("https://www.{}.com", lower));
            }
        }
    }

    if manufacturer_url.is_none() {
        if let Some(copyright) = dict
            .get("NSHumanReadableCopyright")
            .and_then(|v| v.as_string())
        {
            let url_re = regex::Regex::new(r#"https?://[^\s)"',]+"#).unwrap();
            if let Some(m) = url_re.find(copyright) {
                manufacturer_url = Some(m.as_str().to_string());
            }
        }
    }

    (version, manufacturer, manufacturer_url)
}

#[cfg(not(target_os = "macos"))]
fn read_plist_info(_plugin_path: &Path) -> (Option<String>, Option<String>, Option<String>) {
    (None, None, None)
}

pub fn get_plugin_info(file_path: &Path) -> Option<PluginInfo> {
    let ext = file_path
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy().to_lowercase()))
        .unwrap_or_default();

    let name = file_path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();

    let meta = fs::metadata(file_path).ok()?;

    let (version, manufacturer, manufacturer_url) = read_plist_info(file_path);

    let size = if meta.is_dir() {
        get_directory_size(file_path)
    } else {
        meta.len()
    };

    let modified = meta
        .modified()
        .ok()
        .map(|t| {
            let datetime: chrono::DateTime<chrono::Utc> = t.into();
            datetime.format("%Y-%m-%d").to_string()
        })
        .unwrap_or_default();

    Some(PluginInfo {
        name,
        path: file_path.to_string_lossy().to_string(),
        plugin_type: get_plugin_type(&ext).to_string(),
        version: version.unwrap_or_else(|| "Unknown".into()),
        manufacturer: manufacturer.unwrap_or_else(|| "Unknown".into()),
        manufacturer_url,
        size: format_size(size),
        modified,
    })
}

pub fn discover_plugins(directories: &[String]) -> Vec<PathBuf> {
    let valid_extensions = [".vst", ".vst3", ".component", ".dll"];
    let mut plugin_paths = Vec::new();

    for dir in directories {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                let ext = path
                    .extension()
                    .map(|e| format!(".{}", e.to_string_lossy().to_lowercase()))
                    .unwrap_or_default();
                if valid_extensions.contains(&ext.as_str()) {
                    plugin_paths.push(path);
                }
            }
        }
    }

    plugin_paths
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_get_plugin_type() {
        assert_eq!(get_plugin_type(".vst"), "VST2");
        assert_eq!(get_plugin_type(".vst3"), "VST3");
        assert_eq!(get_plugin_type(".component"), "AU");
        assert_eq!(get_plugin_type(".dll"), "VST2");
        assert_eq!(get_plugin_type(".exe"), "Unknown");
    }

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(512), "512.0 B");
        assert_eq!(format_size(1024), "1.0 KB");
        assert_eq!(format_size(1536), "1.5 KB");
        assert_eq!(format_size(1048576), "1.0 MB");
        assert_eq!(format_size(1073741824), "1.0 GB");
    }

    #[test]
    fn test_discover_plugins_empty_dir() {
        let tmp = std::env::temp_dir().join("upum_test_discover_empty");
        let _ = fs::create_dir_all(&tmp);
        let dirs = vec![tmp.to_string_lossy().to_string()];
        let result = discover_plugins(&dirs);
        assert!(result.is_empty());
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_discover_plugins_finds_vst() {
        let tmp = std::env::temp_dir().join("upum_test_discover_vst");
        let _ = fs::remove_dir_all(&tmp);
        let _ = fs::create_dir_all(&tmp);

        // Create fake plugin bundles (directories with plugin extensions)
        let vst2 = tmp.join("TestPlugin.vst");
        let vst3 = tmp.join("TestPlugin.vst3");
        let au = tmp.join("TestPlugin.component");
        let txt = tmp.join("readme.txt");
        let _ = fs::create_dir_all(&vst2);
        let _ = fs::create_dir_all(&vst3);
        let _ = fs::create_dir_all(&au);
        let _ = fs::write(&txt, "not a plugin");

        let dirs = vec![tmp.to_string_lossy().to_string()];
        let mut result = discover_plugins(&dirs);
        result.sort();

        assert_eq!(result.len(), 3);
        assert!(result.iter().any(|p| p.extension().unwrap() == "vst"));
        assert!(result.iter().any(|p| p.extension().unwrap() == "vst3"));
        assert!(result.iter().any(|p| p.extension().unwrap() == "component"));

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_discover_plugins_nonexistent_dir() {
        let dirs = vec!["/nonexistent/path/that/does/not/exist".to_string()];
        let result = discover_plugins(&dirs);
        assert!(result.is_empty());
    }

    #[test]
    fn test_get_directory_size() {
        let tmp = std::env::temp_dir().join("upum_test_dir_size");
        let _ = fs::remove_dir_all(&tmp);
        let _ = fs::create_dir_all(tmp.join("sub"));
        let _ = fs::write(tmp.join("a.txt"), "hello"); // 5 bytes
        let _ = fs::write(tmp.join("sub").join("b.txt"), "world!"); // 6 bytes
        assert_eq!(get_directory_size(&tmp), 11);
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_get_vst_directories_returns_existing_only() {
        let dirs = get_vst_directories();
        for d in &dirs {
            assert!(Path::new(d).exists(), "Directory {} should exist", d);
        }
    }

    #[test]
    fn test_format_size_edge_cases() {
        assert_eq!(format_size(1), "1.0 B");
        assert_eq!(format_size(1023), "1023.0 B");
        assert_eq!(format_size(1024), "1.0 KB");
        // Large value: 5 GB
        assert_eq!(format_size(5 * 1024 * 1024 * 1024), "5.0 GB");
    }

    #[test]
    fn test_get_plugin_info_on_real_dir() {
        let tmp = std::env::temp_dir().join("upum_test_plugin_info");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        let plugin_dir = tmp.join("FakePlugin.vst3");
        fs::create_dir_all(&plugin_dir).unwrap();

        let info = get_plugin_info(&plugin_dir);
        assert!(info.is_some());
        let info = info.unwrap();
        assert_eq!(info.name, "FakePlugin");
        assert_eq!(info.plugin_type, "VST3");
        assert!(info.path.contains("FakePlugin.vst3"));
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_format_size_exact_boundaries() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(1), "1.0 B");
        assert_eq!(format_size(1024), "1.0 KB");
        assert_eq!(format_size(1048576), "1.0 MB");
        assert_eq!(format_size(1073741824), "1.0 GB");
    }

    #[test]
    fn test_get_plugin_info_returns_none_for_nonexistent() {
        let path = Path::new("/nonexistent/path/that/does/not/exist/Plugin.vst3");
        let result = get_plugin_info(path);
        assert!(
            result.is_none(),
            "get_plugin_info should return None for nonexistent path"
        );
    }

    #[test]
    fn test_get_plugin_info_file_not_dir() {
        let tmp = std::env::temp_dir().join("upum_test_plugin_info_file");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        // Create a regular file with .vst3 extension (not a directory bundle)
        let plugin_file = tmp.join("FakeFile.vst3");
        fs::write(&plugin_file, b"not a real plugin").unwrap();

        let info = get_plugin_info(&plugin_file);
        assert!(info.is_some(), "Should return Some even for a file with .vst3 ext");
        let info = info.unwrap();
        assert_eq!(info.name, "FakeFile");
        assert_eq!(info.plugin_type, "VST3");
        // Size should reflect the file size (not 0)
        assert_ne!(info.size, "0 B");
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_discover_plugins_multiple_dirs() {
        let tmp1 = std::env::temp_dir().join("upum_test_discover_multi_1");
        let tmp2 = std::env::temp_dir().join("upum_test_discover_multi_2");
        let _ = fs::remove_dir_all(&tmp1);
        let _ = fs::remove_dir_all(&tmp2);
        fs::create_dir_all(&tmp1).unwrap();
        fs::create_dir_all(&tmp2).unwrap();

        fs::create_dir_all(tmp1.join("PlugA.vst3")).unwrap();
        fs::create_dir_all(tmp1.join("PlugB.vst")).unwrap();
        fs::create_dir_all(tmp2.join("PlugC.component")).unwrap();

        let dirs = vec![
            tmp1.to_string_lossy().to_string(),
            tmp2.to_string_lossy().to_string(),
        ];
        let result = discover_plugins(&dirs);
        assert_eq!(result.len(), 3, "Should find all plugins across both dirs");

        let _ = fs::remove_dir_all(&tmp1);
        let _ = fs::remove_dir_all(&tmp2);
    }

    #[test]
    fn test_discover_plugins_mixed_extensions() {
        let tmp = std::env::temp_dir().join("upum_test_discover_mixed");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        // Valid plugin extensions
        fs::create_dir_all(tmp.join("A.vst")).unwrap();
        fs::create_dir_all(tmp.join("B.vst3")).unwrap();
        fs::create_dir_all(tmp.join("C.component")).unwrap();
        fs::write(tmp.join("D.dll"), b"fake dll").unwrap();

        // Invalid extensions
        fs::write(tmp.join("readme.txt"), b"text").unwrap();
        fs::create_dir_all(tmp.join("Something.app")).unwrap();

        let dirs = vec![tmp.to_string_lossy().to_string()];
        let result = discover_plugins(&dirs);
        assert_eq!(
            result.len(),
            4,
            "Should find exactly 4 valid plugins (.vst, .vst3, .component, .dll), found: {:?}",
            result
        );

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_discover_plugins_ignores_subdirs() {
        let tmp = std::env::temp_dir().join("upum_test_discover_subdirs");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        // Create a subdir, and put a .vst3 inside it (nested, not top-level)
        let subdir = tmp.join("subdir");
        fs::create_dir_all(&subdir).unwrap();
        let nested_plugin = subdir.join("Nested.vst3");
        fs::create_dir_all(&nested_plugin).unwrap();

        // discover_plugins should only scan one level deep from the given directories
        let dirs = vec![tmp.to_string_lossy().to_string()];
        let result = discover_plugins(&dirs);
        // "subdir" has no plugin extension, and Nested.vst3 is inside subdir, not at top level of tmp
        assert!(
            result.is_empty(),
            "Should not find plugins nested inside subdirs, found: {:?}",
            result
        );
        let _ = fs::remove_dir_all(&tmp);
    }
}
