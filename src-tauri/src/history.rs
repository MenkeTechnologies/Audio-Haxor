use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::scanner::PluginInfo;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanSnapshot {
    pub id: String,
    pub timestamp: String,
    #[serde(rename = "pluginCount")]
    pub plugin_count: usize,
    pub plugins: Vec<PluginInfo>,
    pub directories: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanSummary {
    pub id: String,
    pub timestamp: String,
    #[serde(rename = "pluginCount")]
    pub plugin_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanHistory {
    pub scans: Vec<ScanSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionChangedPlugin {
    #[serde(flatten)]
    pub plugin: PluginInfo,
    #[serde(rename = "previousVersion")]
    pub previous_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanDiff {
    #[serde(rename = "oldScan")]
    pub old_scan: ScanSummary,
    #[serde(rename = "newScan")]
    pub new_scan: ScanSummary,
    pub added: Vec<PluginInfo>,
    pub removed: Vec<PluginInfo>,
    #[serde(rename = "versionChanged")]
    pub version_changed: Vec<VersionChangedPlugin>,
}

// KVR Cache
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KvrCacheEntry {
    #[serde(rename = "kvrUrl")]
    pub kvr_url: Option<String>,
    #[serde(rename = "updateUrl")]
    pub update_url: Option<String>,
    #[serde(rename = "latestVersion")]
    pub latest_version: Option<String>,
    #[serde(rename = "hasUpdate")]
    pub has_update: bool,
    pub source: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KvrCacheUpdateEntry {
    pub key: String,
    #[serde(rename = "kvrUrl")]
    pub kvr_url: Option<String>,
    #[serde(rename = "updateUrl")]
    pub update_url: Option<String>,
    #[serde(rename = "latestVersion")]
    pub latest_version: Option<String>,
    #[serde(rename = "hasUpdate")]
    pub has_update: Option<bool>,
    pub source: Option<String>,
}

// DAW project types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DawProject {
    pub name: String,
    pub path: String,
    pub directory: String,
    pub format: String,
    pub daw: String,
    pub size: u64,
    #[serde(rename = "sizeFormatted")]
    pub size_formatted: String,
    pub modified: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DawScanSnapshot {
    pub id: String,
    pub timestamp: String,
    #[serde(rename = "projectCount")]
    pub project_count: usize,
    #[serde(rename = "totalBytes")]
    pub total_bytes: u64,
    #[serde(rename = "dawCounts")]
    pub daw_counts: std::collections::HashMap<String, usize>,
    pub projects: Vec<DawProject>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DawScanSummary {
    pub id: String,
    pub timestamp: String,
    #[serde(rename = "projectCount")]
    pub project_count: usize,
    #[serde(rename = "totalBytes")]
    pub total_bytes: u64,
    #[serde(rename = "dawCounts")]
    pub daw_counts: std::collections::HashMap<String, usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DawHistory {
    pub scans: Vec<DawScanSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DawScanDiff {
    #[serde(rename = "oldScan")]
    pub old_scan: DawScanSummary,
    #[serde(rename = "newScan")]
    pub new_scan: DawScanSummary,
    pub added: Vec<DawProject>,
    pub removed: Vec<DawProject>,
}

// Audio scan types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioSample {
    pub name: String,
    pub path: String,
    pub directory: String,
    pub format: String,
    pub size: u64,
    #[serde(rename = "sizeFormatted")]
    pub size_formatted: String,
    pub modified: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioScanSnapshot {
    pub id: String,
    pub timestamp: String,
    #[serde(rename = "sampleCount")]
    pub sample_count: usize,
    #[serde(rename = "totalBytes")]
    pub total_bytes: u64,
    #[serde(rename = "formatCounts")]
    pub format_counts: std::collections::HashMap<String, usize>,
    pub samples: Vec<AudioSample>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioScanSummary {
    pub id: String,
    pub timestamp: String,
    #[serde(rename = "sampleCount")]
    pub sample_count: usize,
    #[serde(rename = "totalBytes")]
    pub total_bytes: u64,
    #[serde(rename = "formatCounts")]
    pub format_counts: std::collections::HashMap<String, usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioHistory {
    pub scans: Vec<AudioScanSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioScanDiff {
    #[serde(rename = "oldScan")]
    pub old_scan: AudioScanSummary,
    #[serde(rename = "newScan")]
    pub new_scan: AudioScanSummary,
    pub added: Vec<AudioSample>,
    pub removed: Vec<AudioSample>,
}

#[cfg(test)]
thread_local! {
    static TEST_DATA_DIR: std::cell::RefCell<Option<PathBuf>> = const { std::cell::RefCell::new(None) };
}

fn get_data_dir() -> PathBuf {
    #[cfg(test)]
    {
        if let Some(dir) = TEST_DATA_DIR.with(|d| d.borrow().clone()) {
            return dir;
        }
    }
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("com.menketechnologies.universal-plugin-update-manager")
}

fn ensure_data_dir() -> PathBuf {
    let dir = get_data_dir();
    let _ = fs::create_dir_all(&dir);
    dir
}

fn history_file() -> PathBuf {
    ensure_data_dir().join("scan-history.json")
}

fn kvr_cache_file() -> PathBuf {
    ensure_data_dir().join("kvr-cache.json")
}

fn audio_history_file() -> PathBuf {
    ensure_data_dir().join("audio-scan-history.json")
}

fn daw_history_file() -> PathBuf {
    ensure_data_dir().join("daw-scan-history.json")
}

fn preferences_file() -> PathBuf {
    ensure_data_dir().join("preferences.json")
}

fn default_config() -> std::collections::HashMap<String, serde_json::Value> {
    let bytes = include_str!("../../config.default.json");
    serde_json::from_str(bytes).unwrap_or_default()
}

pub fn load_preferences() -> std::collections::HashMap<String, serde_json::Value> {
    let path = preferences_file();
    if path.exists() {
        if let Ok(data) = fs::read_to_string(&path) {
            if let Ok(prefs) =
                serde_json::from_str::<std::collections::HashMap<String, serde_json::Value>>(&data)
            {
                // Merge defaults under user prefs so new keys are picked up
                let mut merged = default_config();
                for (k, v) in prefs {
                    merged.insert(k, v);
                }
                return merged;
            }
        }
    }
    let defaults = default_config();
    save_preferences(&defaults);
    defaults
}

pub fn save_preferences(prefs: &std::collections::HashMap<String, serde_json::Value>) {
    let path = preferences_file();
    if let Ok(json) = serde_json::to_string_pretty(prefs) {
        let _ = fs::write(&path, json);
    }
}

pub fn set_preference(key: &str, value: serde_json::Value) {
    let mut prefs = load_preferences();
    prefs.insert(key.to_string(), value);
    save_preferences(&prefs);
}

pub fn get_preference(key: &str) -> Option<serde_json::Value> {
    let prefs = load_preferences();
    prefs.get(key).cloned()
}

pub fn remove_preference(key: &str) {
    let mut prefs = load_preferences();
    prefs.remove(key);
    save_preferences(&prefs);
}

fn gen_id() -> String {
    use rand::Rng;
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let mut rng = rand::thread_rng();
    let rand_part: u32 = rng.gen();
    format!(
        "{}{}",
        radix_string(ts as u64, 36),
        radix_string(rand_part as u64, 36)
    )
}

fn radix_string(mut n: u64, base: u64) -> String {
    if n == 0 {
        return "0".into();
    }
    let chars: Vec<char> = "0123456789abcdefghijklmnopqrstuvwxyz".chars().collect();
    let mut result = Vec::new();
    while n > 0 {
        result.push(chars[(n % base) as usize]);
        n /= base;
    }
    result.reverse();
    result.into_iter().collect()
}

fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

// ── Plugin scan history ──

fn load_history() -> ScanHistory {
    let path = history_file();
    if path.exists() {
        if let Ok(data) = fs::read_to_string(&path) {
            if let Ok(h) = serde_json::from_str(&data) {
                return h;
            }
        }
    }
    ScanHistory { scans: vec![] }
}

fn save_history(history: &ScanHistory) {
    let path = history_file();
    if let Ok(data) = serde_json::to_string_pretty(history) {
        let _ = fs::write(path, data);
    }
}

pub fn save_scan(plugins: &[PluginInfo], directories: &[String]) -> ScanSnapshot {
    let mut history = load_history();
    let snapshot = ScanSnapshot {
        id: gen_id(),
        timestamp: now_iso(),
        plugin_count: plugins.len(),
        plugins: plugins.to_vec(),
        directories: directories.to_vec(),
    };
    history.scans.insert(0, snapshot.clone());
    if history.scans.len() > 50 {
        history.scans.truncate(50);
    }
    save_history(&history);
    snapshot
}

pub fn get_scans() -> Vec<ScanSummary> {
    let history = load_history();
    history
        .scans
        .iter()
        .map(|s| ScanSummary {
            id: s.id.clone(),
            timestamp: s.timestamp.clone(),
            plugin_count: s.plugin_count,
        })
        .collect()
}

pub fn get_scan_detail(id: &str) -> Option<ScanSnapshot> {
    let history = load_history();
    history.scans.into_iter().find(|s| s.id == id)
}

pub fn delete_scan(id: &str) {
    let mut history = load_history();
    history.scans.retain(|s| s.id != id);
    save_history(&history);
}

pub fn clear_history() {
    save_history(&ScanHistory { scans: vec![] });
}

pub fn diff_scans(old_id: &str, new_id: &str) -> Option<ScanDiff> {
    let history = load_history();
    let old_scan = history.scans.iter().find(|s| s.id == old_id)?;
    let new_scan = history.scans.iter().find(|s| s.id == new_id)?;

    let old_paths: std::collections::HashSet<&str> =
        old_scan.plugins.iter().map(|p| p.path.as_str()).collect();
    let new_paths: std::collections::HashSet<&str> =
        new_scan.plugins.iter().map(|p| p.path.as_str()).collect();

    let old_by_path: std::collections::HashMap<&str, &PluginInfo> = old_scan
        .plugins
        .iter()
        .map(|p| (p.path.as_str(), p))
        .collect();

    let added: Vec<PluginInfo> = new_scan
        .plugins
        .iter()
        .filter(|p| !old_paths.contains(p.path.as_str()))
        .cloned()
        .collect();

    let removed: Vec<PluginInfo> = old_scan
        .plugins
        .iter()
        .filter(|p| !new_paths.contains(p.path.as_str()))
        .cloned()
        .collect();

    let version_changed: Vec<VersionChangedPlugin> = new_scan
        .plugins
        .iter()
        .filter_map(|p| {
            let old = old_by_path.get(p.path.as_str())?;
            if old.version != p.version && p.version != "Unknown" && old.version != "Unknown" {
                Some(VersionChangedPlugin {
                    plugin: p.clone(),
                    previous_version: old.version.clone(),
                })
            } else {
                None
            }
        })
        .collect();

    Some(ScanDiff {
        old_scan: ScanSummary {
            id: old_scan.id.clone(),
            timestamp: old_scan.timestamp.clone(),
            plugin_count: old_scan.plugin_count,
        },
        new_scan: ScanSummary {
            id: new_scan.id.clone(),
            timestamp: new_scan.timestamp.clone(),
            plugin_count: new_scan.plugin_count,
        },
        added,
        removed,
        version_changed,
    })
}

pub fn get_latest_scan() -> Option<ScanSnapshot> {
    let history = load_history();
    history.scans.into_iter().next()
}

// ── KVR Cache ──

pub fn load_kvr_cache() -> std::collections::HashMap<String, KvrCacheEntry> {
    let path = kvr_cache_file();
    if path.exists() {
        if let Ok(data) = fs::read_to_string(&path) {
            if let Ok(cache) = serde_json::from_str(&data) {
                return cache;
            }
        }
    }
    std::collections::HashMap::new()
}

fn save_kvr_cache(cache: &std::collections::HashMap<String, KvrCacheEntry>) {
    let path = kvr_cache_file();
    if let Ok(data) = serde_json::to_string_pretty(cache) {
        let _ = fs::write(path, data);
    }
}

pub fn update_kvr_cache(entries: &[KvrCacheUpdateEntry]) {
    let mut cache = load_kvr_cache();
    for entry in entries {
        cache.insert(
            entry.key.clone(),
            KvrCacheEntry {
                kvr_url: entry.kvr_url.clone(),
                update_url: entry.update_url.clone(),
                latest_version: entry.latest_version.clone(),
                has_update: entry.has_update.unwrap_or(false),
                source: entry.source.clone().unwrap_or_else(|| "kvr".into()),
                timestamp: now_iso(),
            },
        );
    }
    save_kvr_cache(&cache);
}

// ── Audio scan history ──

fn load_audio_history() -> AudioHistory {
    let path = audio_history_file();
    if path.exists() {
        if let Ok(data) = fs::read_to_string(&path) {
            if let Ok(h) = serde_json::from_str(&data) {
                return h;
            }
        }
    }
    AudioHistory { scans: vec![] }
}

fn save_audio_history(history: &AudioHistory) {
    let path = audio_history_file();
    if let Ok(data) = serde_json::to_string_pretty(history) {
        let _ = fs::write(path, data);
    }
}

pub fn save_audio_scan(samples: &[AudioSample]) -> AudioScanSnapshot {
    let mut history = load_audio_history();
    let mut format_counts = std::collections::HashMap::new();
    let mut total_bytes = 0u64;
    for s in samples {
        *format_counts.entry(s.format.clone()).or_insert(0) += 1;
        total_bytes += s.size;
    }
    let snapshot = AudioScanSnapshot {
        id: gen_id(),
        timestamp: now_iso(),
        sample_count: samples.len(),
        total_bytes,
        format_counts,
        samples: samples.to_vec(),
    };
    history.scans.insert(0, snapshot.clone());
    if history.scans.len() > 50 {
        history.scans.truncate(50);
    }
    save_audio_history(&history);
    snapshot
}

pub fn get_audio_scans() -> Vec<AudioScanSummary> {
    let history = load_audio_history();
    history
        .scans
        .iter()
        .map(|s| AudioScanSummary {
            id: s.id.clone(),
            timestamp: s.timestamp.clone(),
            sample_count: s.sample_count,
            total_bytes: s.total_bytes,
            format_counts: s.format_counts.clone(),
        })
        .collect()
}

pub fn get_audio_scan_detail(id: &str) -> Option<AudioScanSnapshot> {
    let history = load_audio_history();
    history.scans.into_iter().find(|s| s.id == id)
}

pub fn delete_audio_scan(id: &str) {
    let mut history = load_audio_history();
    history.scans.retain(|s| s.id != id);
    save_audio_history(&history);
}

pub fn clear_audio_history() {
    save_audio_history(&AudioHistory { scans: vec![] });
}

pub fn get_latest_audio_scan() -> Option<AudioScanSnapshot> {
    let history = load_audio_history();
    history.scans.into_iter().next()
}

// ── DAW scan history ──

fn load_daw_history() -> DawHistory {
    let path = daw_history_file();
    if path.exists() {
        if let Ok(data) = fs::read_to_string(&path) {
            if let Ok(h) = serde_json::from_str(&data) {
                return h;
            }
        }
    }
    DawHistory { scans: vec![] }
}

fn save_daw_history(history: &DawHistory) {
    let path = daw_history_file();
    if let Ok(data) = serde_json::to_string_pretty(history) {
        let _ = fs::write(path, data);
    }
}

pub fn save_daw_scan(projects: &[DawProject]) -> DawScanSnapshot {
    let mut history = load_daw_history();
    let mut daw_counts = std::collections::HashMap::new();
    let mut total_bytes = 0u64;
    for p in projects {
        *daw_counts.entry(p.daw.clone()).or_insert(0) += 1;
        total_bytes += p.size;
    }
    let snapshot = DawScanSnapshot {
        id: gen_id(),
        timestamp: now_iso(),
        project_count: projects.len(),
        total_bytes,
        daw_counts,
        projects: projects.to_vec(),
    };
    history.scans.insert(0, snapshot.clone());
    if history.scans.len() > 50 {
        history.scans.truncate(50);
    }
    save_daw_history(&history);
    snapshot
}

pub fn get_daw_scans() -> Vec<DawScanSummary> {
    let history = load_daw_history();
    history
        .scans
        .iter()
        .map(|s| DawScanSummary {
            id: s.id.clone(),
            timestamp: s.timestamp.clone(),
            project_count: s.project_count,
            total_bytes: s.total_bytes,
            daw_counts: s.daw_counts.clone(),
        })
        .collect()
}

pub fn get_daw_scan_detail(id: &str) -> Option<DawScanSnapshot> {
    let history = load_daw_history();
    history.scans.into_iter().find(|s| s.id == id)
}

pub fn delete_daw_scan(id: &str) {
    let mut history = load_daw_history();
    history.scans.retain(|s| s.id != id);
    save_daw_history(&history);
}

pub fn clear_daw_history() {
    save_daw_history(&DawHistory { scans: vec![] });
}

pub fn get_latest_daw_scan() -> Option<DawScanSnapshot> {
    let history = load_daw_history();
    history.scans.into_iter().next()
}

pub fn diff_daw_scans(old_id: &str, new_id: &str) -> Option<DawScanDiff> {
    let history = load_daw_history();
    let old_scan = history.scans.iter().find(|s| s.id == old_id)?;
    let new_scan = history.scans.iter().find(|s| s.id == new_id)?;

    let old_paths: std::collections::HashSet<&str> =
        old_scan.projects.iter().map(|p| p.path.as_str()).collect();
    let new_paths: std::collections::HashSet<&str> =
        new_scan.projects.iter().map(|p| p.path.as_str()).collect();

    let added: Vec<DawProject> = new_scan
        .projects
        .iter()
        .filter(|p| !old_paths.contains(p.path.as_str()))
        .cloned()
        .collect();

    let removed: Vec<DawProject> = old_scan
        .projects
        .iter()
        .filter(|p| !new_paths.contains(p.path.as_str()))
        .cloned()
        .collect();

    Some(DawScanDiff {
        old_scan: DawScanSummary {
            id: old_scan.id.clone(),
            timestamp: old_scan.timestamp.clone(),
            project_count: old_scan.project_count,
            total_bytes: old_scan.total_bytes,
            daw_counts: old_scan.daw_counts.clone(),
        },
        new_scan: DawScanSummary {
            id: new_scan.id.clone(),
            timestamp: new_scan.timestamp.clone(),
            project_count: new_scan.project_count,
            total_bytes: new_scan.total_bytes,
            daw_counts: new_scan.daw_counts.clone(),
        },
        added,
        removed,
    })
}

pub fn diff_audio_scans(old_id: &str, new_id: &str) -> Option<AudioScanDiff> {
    let history = load_audio_history();
    let old_scan = history.scans.iter().find(|s| s.id == old_id)?;
    let new_scan = history.scans.iter().find(|s| s.id == new_id)?;

    let old_paths: std::collections::HashSet<&str> =
        old_scan.samples.iter().map(|s| s.path.as_str()).collect();
    let new_paths: std::collections::HashSet<&str> =
        new_scan.samples.iter().map(|s| s.path.as_str()).collect();

    let added: Vec<AudioSample> = new_scan
        .samples
        .iter()
        .filter(|s| !old_paths.contains(s.path.as_str()))
        .cloned()
        .collect();

    let removed: Vec<AudioSample> = old_scan
        .samples
        .iter()
        .filter(|s| !new_paths.contains(s.path.as_str()))
        .cloned()
        .collect();

    Some(AudioScanDiff {
        old_scan: AudioScanSummary {
            id: old_scan.id.clone(),
            timestamp: old_scan.timestamp.clone(),
            sample_count: old_scan.sample_count,
            total_bytes: old_scan.total_bytes,
            format_counts: old_scan.format_counts.clone(),
        },
        new_scan: AudioScanSummary {
            id: new_scan.id.clone(),
            timestamp: new_scan.timestamp.clone(),
            sample_count: new_scan.sample_count,
            total_bytes: new_scan.total_bytes,
            format_counts: new_scan.format_counts.clone(),
        },
        added,
        removed,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_radix_string_base36() {
        assert_eq!(radix_string(0, 36), "0");
        assert_eq!(radix_string(35, 36), "z");
        assert_eq!(radix_string(36, 36), "10");
        assert_eq!(radix_string(100, 36), "2s");
    }

    #[test]
    fn test_radix_string_base10() {
        assert_eq!(radix_string(0, 10), "0");
        assert_eq!(radix_string(42, 10), "42");
        assert_eq!(radix_string(999, 10), "999");
    }

    #[test]
    fn test_gen_id_unique() {
        let id1 = gen_id();
        let id2 = gen_id();
        assert_ne!(id1, id2);
        assert!(!id1.is_empty());
    }

    #[test]
    fn test_now_iso_format() {
        let ts = now_iso();
        // Should be RFC3339 format
        assert!(ts.contains('T'));
        assert!(ts.ends_with('Z'));
    }

    fn with_test_dir<F: FnOnce()>(name: &str, f: F) {
        let dir = std::env::temp_dir().join(format!("upum_test_{}", name));
        let _ = fs::remove_dir_all(&dir);
        let _ = fs::create_dir_all(&dir);
        TEST_DATA_DIR.with(|d| *d.borrow_mut() = Some(dir.clone()));
        f();
        TEST_DATA_DIR.with(|d| *d.borrow_mut() = None);
        let _ = fs::remove_dir_all(&dir);
    }

    fn make_plugin(name: &str, version: &str, path: &str) -> PluginInfo {
        PluginInfo {
            name: name.into(),
            path: path.into(),
            plugin_type: "VST3".into(),
            version: version.into(),
            manufacturer: "TestMfg".into(),
            manufacturer_url: None,
            size: "1.0 MB".into(),
            modified: "2024-01-01".into(),
        }
    }

    fn make_sample(name: &str, path: &str, format: &str) -> AudioSample {
        AudioSample {
            name: name.into(),
            path: path.into(),
            directory: "/tmp".into(),
            format: format.into(),
            size: 1024,
            size_formatted: "1.0 KB".into(),
            modified: "2024-01-01".into(),
        }
    }

    #[test]
    fn test_scan_history_crud() {
        with_test_dir("scan_crud", || {
            let plugins = vec![
                make_plugin("PlugA", "1.0", "/tmp/a.vst3"),
                make_plugin("PlugB", "2.0", "/tmp/b.vst3"),
            ];
            let dirs = vec!["/tmp".to_string()];
            let snap = save_scan(&plugins, &dirs);
            assert_eq!(snap.plugin_count, 2);

            let scans = get_scans();
            assert!(scans.iter().any(|s| s.id == snap.id));

            let detail = get_scan_detail(&snap.id);
            assert!(detail.is_some());
            assert_eq!(detail.unwrap().plugins.len(), 2);

            let latest = get_latest_scan();
            assert!(latest.is_some());

            delete_scan(&snap.id);
            assert!(get_scan_detail(&snap.id).is_none());
        });
    }

    #[test]
    fn test_scan_history_limit_50() {
        with_test_dir("scan_limit", || {
            let plugins = vec![make_plugin("P", "1.0", "/tmp/p.vst3")];
            let dirs = vec!["/tmp".to_string()];
            for _ in 0..55 {
                save_scan(&plugins, &dirs);
            }
            let scans = get_scans();
            assert!(scans.len() <= 50);
        });
    }

    #[test]
    fn test_diff_scans_added_removed() {
        with_test_dir("diff_added_removed", || {
            let plugins1 = vec![
                make_plugin("PlugA", "1.0", "/tmp/a.vst3"),
                make_plugin("PlugB", "1.0", "/tmp/b.vst3"),
            ];
            let plugins2 = vec![
                make_plugin("PlugB", "1.0", "/tmp/b.vst3"),
                make_plugin("PlugC", "1.0", "/tmp/c.vst3"),
            ];
            let dirs = vec!["/tmp".to_string()];
            let snap1 = save_scan(&plugins1, &dirs);
            let snap2 = save_scan(&plugins2, &dirs);

            let diff = diff_scans(&snap1.id, &snap2.id).unwrap();
            assert_eq!(diff.added.len(), 1);
            assert_eq!(diff.added[0].name, "PlugC");
            assert_eq!(diff.removed.len(), 1);
            assert_eq!(diff.removed[0].name, "PlugA");
        });
    }

    #[test]
    fn test_diff_scans_version_changed() {
        with_test_dir("diff_version", || {
            let plugins1 = vec![make_plugin("PlugA", "1.0", "/tmp/vc_a.vst3")];
            let plugins2 = vec![make_plugin("PlugA", "2.0", "/tmp/vc_a.vst3")];
            let dirs = vec!["/tmp".to_string()];
            let snap1 = save_scan(&plugins1, &dirs);
            let snap2 = save_scan(&plugins2, &dirs);

            let diff = diff_scans(&snap1.id, &snap2.id).unwrap();
            assert_eq!(diff.version_changed.len(), 1);
            assert_eq!(diff.version_changed[0].previous_version, "1.0");
            assert_eq!(diff.version_changed[0].plugin.version, "2.0");
        });
    }

    #[test]
    fn test_kvr_cache_crud() {
        with_test_dir("kvr_cache", || {
            let entries = vec![KvrCacheUpdateEntry {
                key: "test-plugin".into(),
                kvr_url: Some("https://kvr.com/test".into()),
                update_url: None,
                latest_version: Some("1.5".into()),
                has_update: Some(true),
                source: Some("kvr".into()),
            }];
            update_kvr_cache(&entries);

            let cache = load_kvr_cache();
            assert!(cache.contains_key("test-plugin"));
            let entry = &cache["test-plugin"];
            assert_eq!(entry.latest_version, Some("1.5".into()));
            assert!(entry.has_update);
        });
    }

    #[test]
    fn test_audio_history_crud() {
        with_test_dir("audio_crud", || {
            let samples = vec![
                make_sample("kick", "/tmp/kick.wav", "WAV"),
                make_sample("snare", "/tmp/snare.mp3", "MP3"),
            ];
            let snap = save_audio_scan(&samples);
            assert_eq!(snap.sample_count, 2);
            assert_eq!(snap.total_bytes, 2048);
            assert_eq!(snap.format_counts.get("WAV"), Some(&1));
            assert_eq!(snap.format_counts.get("MP3"), Some(&1));

            let scans = get_audio_scans();
            assert!(scans.iter().any(|s| s.id == snap.id));

            let detail = get_audio_scan_detail(&snap.id).unwrap();
            assert_eq!(detail.samples.len(), 2);

            let latest = get_latest_audio_scan().unwrap();
            assert_eq!(latest.id, snap.id);

            delete_audio_scan(&snap.id);
            assert!(get_audio_scan_detail(&snap.id).is_none());
        });
    }

    #[test]
    fn test_save_scan_preserves_order() {
        with_test_dir("scan_order", || {
            let dirs = vec!["/tmp".to_string()];
            let s1 = save_scan(&[make_plugin("A", "1.0", "/tmp/a.vst3")], &dirs);
            let s2 = save_scan(&[make_plugin("B", "1.0", "/tmp/b.vst3")], &dirs);
            let s3 = save_scan(&[make_plugin("C", "1.0", "/tmp/c.vst3")], &dirs);

            let scans = get_scans();
            // Newest first
            assert_eq!(scans[0].id, s3.id);
            assert_eq!(scans[1].id, s2.id);
            assert_eq!(scans[2].id, s1.id);
        });
    }

    #[test]
    fn test_delete_nonexistent_scan() {
        with_test_dir("delete_nonexistent", || {
            let dirs = vec!["/tmp".to_string()];
            let snap = save_scan(&[make_plugin("X", "1.0", "/tmp/x.vst3")], &dirs);

            // Delete a fake id - should not crash
            delete_scan("totally-fake-id-12345");

            // Original scan should still exist
            let detail = get_scan_detail(&snap.id);
            assert!(detail.is_some());
        });
    }

    #[test]
    fn test_clear_history_idempotent() {
        with_test_dir("clear_idempotent", || {
            let dirs = vec!["/tmp".to_string()];
            save_scan(&[make_plugin("X", "1.0", "/tmp/x.vst3")], &dirs);

            clear_history();
            assert!(get_scans().is_empty());

            // Second clear should not crash
            clear_history();
            assert!(get_scans().is_empty());
        });
    }

    #[test]
    fn test_diff_scans_no_changes() {
        with_test_dir("diff_no_changes", || {
            let plugins = vec![
                make_plugin("PlugA", "1.0", "/tmp/a.vst3"),
                make_plugin("PlugB", "2.0", "/tmp/b.vst3"),
            ];
            let dirs = vec!["/tmp".to_string()];
            let snap1 = save_scan(&plugins, &dirs);
            let snap2 = save_scan(&plugins, &dirs);

            let diff = diff_scans(&snap1.id, &snap2.id).unwrap();
            assert!(diff.added.is_empty(), "added should be empty");
            assert!(diff.removed.is_empty(), "removed should be empty");
            assert!(
                diff.version_changed.is_empty(),
                "version_changed should be empty"
            );
        });
    }

    #[test]
    fn test_diff_scans_nonexistent_ids() {
        with_test_dir("diff_nonexistent", || {
            let result = diff_scans("fake-id-1", "fake-id-2");
            assert!(
                result.is_none(),
                "diff_scans with fake ids should return None"
            );
        });
    }

    #[test]
    fn test_audio_history_limit_50() {
        with_test_dir("audio_limit_50", || {
            let sample = vec![make_sample("kick", "/tmp/kick.wav", "WAV")];
            for _ in 0..55 {
                save_audio_scan(&sample);
            }
            let scans = get_audio_scans();
            assert!(
                scans.len() <= 50,
                "Audio history should be limited to 50, got {}",
                scans.len()
            );
        });
    }

    #[test]
    fn test_kvr_cache_update_overwrites() {
        with_test_dir("kvr_overwrite", || {
            let entries_v1 = vec![KvrCacheUpdateEntry {
                key: "my-plugin".into(),
                kvr_url: Some("https://kvr.com/my".into()),
                update_url: None,
                latest_version: Some("1.0".into()),
                has_update: Some(false),
                source: Some("kvr".into()),
            }];
            update_kvr_cache(&entries_v1);

            let entries_v2 = vec![KvrCacheUpdateEntry {
                key: "my-plugin".into(),
                kvr_url: Some("https://kvr.com/my".into()),
                update_url: Some("https://example.com/dl".into()),
                latest_version: Some("2.0".into()),
                has_update: Some(true),
                source: Some("kvr".into()),
            }];
            update_kvr_cache(&entries_v2);

            let cache = load_kvr_cache();
            let entry = &cache["my-plugin"];
            assert_eq!(entry.latest_version, Some("2.0".into()));
            assert!(entry.has_update);
            assert_eq!(entry.update_url, Some("https://example.com/dl".into()));
        });
    }

    #[test]
    fn test_kvr_cache_multiple_entries() {
        with_test_dir("kvr_multiple", || {
            let entries = vec![
                KvrCacheUpdateEntry {
                    key: "plugin-a".into(),
                    kvr_url: Some("https://kvr.com/a".into()),
                    update_url: None,
                    latest_version: Some("1.0".into()),
                    has_update: Some(false),
                    source: Some("kvr".into()),
                },
                KvrCacheUpdateEntry {
                    key: "plugin-b".into(),
                    kvr_url: Some("https://kvr.com/b".into()),
                    update_url: None,
                    latest_version: Some("2.0".into()),
                    has_update: Some(true),
                    source: Some("kvr".into()),
                },
                KvrCacheUpdateEntry {
                    key: "plugin-c".into(),
                    kvr_url: Some("https://kvr.com/c".into()),
                    update_url: None,
                    latest_version: Some("3.0".into()),
                    has_update: Some(false),
                    source: Some("kvr".into()),
                },
            ];
            update_kvr_cache(&entries);

            let cache = load_kvr_cache();
            assert!(cache.contains_key("plugin-a"));
            assert!(cache.contains_key("plugin-b"));
            assert!(cache.contains_key("plugin-c"));
            assert_eq!(cache["plugin-a"].latest_version, Some("1.0".into()));
            assert_eq!(cache["plugin-b"].latest_version, Some("2.0".into()));
            assert_eq!(cache["plugin-c"].latest_version, Some("3.0".into()));
        });
    }

    #[test]
    fn test_audio_diff() {
        with_test_dir("audio_diff", || {
            let samples1 = vec![
                make_sample("kick", "/tmp/kick.wav", "WAV"),
                make_sample("snare", "/tmp/snare.wav", "WAV"),
            ];
            let samples2 = vec![
                make_sample("snare", "/tmp/snare.wav", "WAV"),
                make_sample("hihat", "/tmp/hihat.wav", "WAV"),
            ];
            let snap1 = save_audio_scan(&samples1);
            let snap2 = save_audio_scan(&samples2);

            let diff = diff_audio_scans(&snap1.id, &snap2.id).unwrap();
            assert_eq!(diff.added.len(), 1);
            assert_eq!(diff.added[0].name, "hihat");
            assert_eq!(diff.removed.len(), 1);
            assert_eq!(diff.removed[0].name, "kick");
        });
    }

    fn make_daw_project(name: &str, path: &str, format: &str, daw: &str) -> DawProject {
        DawProject {
            name: name.into(),
            path: path.into(),
            directory: "/tmp".into(),
            format: format.into(),
            daw: daw.into(),
            size: 2048,
            size_formatted: "2.0 KB".into(),
            modified: "2024-01-01".into(),
        }
    }

    #[test]
    fn test_daw_history_crud() {
        with_test_dir("daw_crud", || {
            let projects = vec![
                make_daw_project("Song1", "/tmp/song1.als", "ALS", "Ableton Live"),
                make_daw_project("Song2", "/tmp/song2.flp", "FLP", "FL Studio"),
            ];
            let snap = save_daw_scan(&projects);
            assert_eq!(snap.project_count, 2);
            assert_eq!(snap.total_bytes, 4096);
            assert_eq!(snap.daw_counts.get("Ableton Live"), Some(&1));
            assert_eq!(snap.daw_counts.get("FL Studio"), Some(&1));

            let scans = get_daw_scans();
            assert!(scans.iter().any(|s| s.id == snap.id));

            let detail = get_daw_scan_detail(&snap.id).unwrap();
            assert_eq!(detail.projects.len(), 2);

            let latest = get_latest_daw_scan().unwrap();
            assert_eq!(latest.id, snap.id);

            delete_daw_scan(&snap.id);
            assert!(get_daw_scan_detail(&snap.id).is_none());
        });
    }

    #[test]
    fn test_daw_history_limit_50() {
        with_test_dir("daw_limit_50", || {
            let projects = vec![make_daw_project(
                "Song",
                "/tmp/song.als",
                "ALS",
                "Ableton Live",
            )];
            for _ in 0..55 {
                save_daw_scan(&projects);
            }
            let scans = get_daw_scans();
            assert!(
                scans.len() <= 50,
                "DAW history should be limited to 50, got {}",
                scans.len()
            );
        });
    }

    #[test]
    fn test_daw_diff() {
        with_test_dir("daw_diff", || {
            let projects1 = vec![
                make_daw_project("Song1", "/tmp/song1.als", "ALS", "Ableton Live"),
                make_daw_project("Song2", "/tmp/song2.flp", "FLP", "FL Studio"),
            ];
            let projects2 = vec![
                make_daw_project("Song2", "/tmp/song2.flp", "FLP", "FL Studio"),
                make_daw_project("Song3", "/tmp/song3.rpp", "RPP", "REAPER"),
            ];
            let snap1 = save_daw_scan(&projects1);
            let snap2 = save_daw_scan(&projects2);

            let diff = diff_daw_scans(&snap1.id, &snap2.id).unwrap();
            assert_eq!(diff.added.len(), 1);
            assert_eq!(diff.added[0].name, "Song3");
            assert_eq!(diff.removed.len(), 1);
            assert_eq!(diff.removed[0].name, "Song1");
        });
    }
}
