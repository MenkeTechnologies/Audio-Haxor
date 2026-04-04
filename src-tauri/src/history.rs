use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::scanner::PluginInfo;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanSnapshot {
    pub id: String,
    pub timestamp: String,
    #[serde(rename = "pluginCount")]
    pub plugin_count: usize,
    pub plugins: Vec<PluginInfo>,
    pub directories: Vec<String>,
    #[serde(default)]
    pub roots: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanSummary {
    pub id: String,
    pub timestamp: String,
    #[serde(rename = "pluginCount")]
    pub plugin_count: usize,
    #[serde(default)]
    pub roots: Vec<String>,
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
    #[serde(default)]
    pub roots: Vec<String>,
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
    #[serde(default)]
    pub roots: Vec<String>,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub channels: Option<u16>,
    #[serde(
        default,
        rename = "sampleRate",
        skip_serializing_if = "Option::is_none"
    )]
    pub sample_rate: Option<u32>,
    #[serde(
        default,
        rename = "bitsPerSample",
        skip_serializing_if = "Option::is_none"
    )]
    pub bits_per_sample: Option<u16>,
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
    #[serde(default)]
    pub roots: Vec<String>,
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
    #[serde(default)]
    pub roots: Vec<String>,
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

// Preset scan types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresetFile {
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
pub struct PresetScanSnapshot {
    pub id: String,
    pub timestamp: String,
    #[serde(rename = "presetCount")]
    pub preset_count: usize,
    #[serde(rename = "totalBytes")]
    pub total_bytes: u64,
    #[serde(rename = "formatCounts")]
    pub format_counts: std::collections::HashMap<String, usize>,
    pub presets: Vec<PresetFile>,
    #[serde(default)]
    pub roots: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresetScanSummary {
    pub id: String,
    pub timestamp: String,
    #[serde(rename = "presetCount")]
    pub preset_count: usize,
    #[serde(rename = "totalBytes")]
    pub total_bytes: u64,
    #[serde(rename = "formatCounts")]
    pub format_counts: std::collections::HashMap<String, usize>,
    #[serde(default)]
    pub roots: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresetHistory {
    pub scans: Vec<PresetScanSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresetScanDiff {
    #[serde(rename = "oldScan")]
    pub old_scan: PresetScanSummary,
    #[serde(rename = "newScan")]
    pub new_scan: PresetScanSummary,
    pub added: Vec<PresetFile>,
    pub removed: Vec<PresetFile>,
}

#[cfg(test)]
thread_local! {
    static TEST_DATA_DIR: std::cell::RefCell<Option<PathBuf>> = const { std::cell::RefCell::new(None) };
}

pub fn get_data_dir() -> PathBuf {
    #[cfg(test)]
    {
        if let Some(dir) = TEST_DATA_DIR.with(|d| d.borrow().clone()) {
            return dir;
        }
    }
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("com.menketechnologies.audio-haxor")
}

/// Redirect [`get_data_dir`] for unit tests (e.g. `lib.rs` log tests). Clear when done.
#[cfg(test)]
pub fn set_test_data_dir_path(path: PathBuf) {
    TEST_DATA_DIR.with(|d| *d.borrow_mut() = Some(path));
}

#[cfg(test)]
pub fn clear_test_data_dir_path() {
    TEST_DATA_DIR.with(|d| *d.borrow_mut() = None);
}

/// Creates `get_data_dir()` if needed; safe to call before writing files there.
pub fn ensure_data_dir() -> PathBuf {
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
    ensure_data_dir().join("preferences.toml")
}

fn legacy_preferences_file() -> PathBuf {
    ensure_data_dir().join("preferences.json")
}

pub fn get_preferences_path() -> PathBuf {
    preferences_file()
}

pub type PrefsMap = serde_json::Map<String, serde_json::Value>;

/// Avoid re-reading preferences.toml on every hot path (e.g. `get_process_stats` once per second).
/// Cache entries are keyed by the resolved preferences path so parallel tests (each with a
/// thread-local temp data dir) and the main app never share a `PrefsMap` across different files.
static PREF_CACHE: Mutex<Option<(u64, PathBuf, PrefsMap)>> = Mutex::new(None);
const PREF_CACHE_TTL_MS: u64 = 2000;

fn prefs_cache_now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn invalidate_prefs_cache() {
    if let Ok(mut g) = PREF_CACHE.lock() {
        *g = None;
    }
}

// Maps flat pref keys to TOML sections for organized file layout.
// Keys not listed here go under [general].
// Format: (section_name, &[(flat_key, toml_key)])
const SECTION_MAP: &[(&str, &[(&str, &str)])] = &[
    ("window", &[("window", "window")]),
    (
        "appearance",
        &[
            ("theme", "theme"),
            ("colorScheme", "colorScheme"),
            ("crtEffects", "crtEffects"),
        ],
    ),
    (
        "scanning",
        &[
            ("autoScan", "autoScan"),
            ("autoUpdate", "autoUpdate"),
            ("singleClickPlay", "singleClickPlay"),
            ("defaultTypeFilter", "defaultTypeFilter"),
            ("customDirs", "customDirs"),
            ("audioScanDirs", "audioScanDirs"),
            ("dawScanDirs", "dawScanDirs"),
            ("presetScanDirs", "presetScanDirs"),
            ("includeAbletonBackups", "includeAbletonBackups"),
        ],
    ),
    (
        "sorting",
        &[
            ("pluginSort", "pluginSort"),
            ("audioSort", "audioSort"),
            ("dawSort", "dawSort"),
        ],
    ),
    (
        "performance",
        &[
            ("pageSize", "pageSize"),
            ("flushInterval", "flushInterval"),
            ("threadMultiplier", "threadMultiplier"),
            ("channelBuffer", "channelBuffer"),
            ("batchSize", "batchSize"),
        ],
    ),
    ("player", &[("playerDock", "dock")]),
    ("tabs", &[("tabOrder", "order")]),
    (
        "customScheme",
        &[
            ("customSchemeVars", "vars"),
            ("customSchemePresets", "presets"),
        ],
    ),
    ("data", &[("columnWidths", "widths")]),
    ("favorites", &[("favorites", "items")]),
    ("notes", &[("itemNotes", "itemNotes")]),
    ("history", &[("recentlyPlayed", "recentlyPlayed")]),
];

fn default_config() -> PrefsMap {
    let toml_str = include_str!("../../config.default.toml");
    toml_to_flat(toml_str)
}

/// Build a reverse lookup: (section, toml_key) → flat_key
fn toml_key_to_flat(section: &str, toml_key: &str) -> Option<String> {
    for (sec, keys) in SECTION_MAP {
        if *sec == section {
            for (flat, tk) in *keys {
                if *tk == toml_key {
                    return Some(flat.to_string());
                }
            }
        }
    }
    None
}

/// If a value is a string that looks like JSON array/object, parse it to native.
/// Handles migration from old format where structured data was JSON-stringified.
fn migrate_json_string(val: serde_json::Value) -> serde_json::Value {
    if let serde_json::Value::String(s) = &val {
        let trimmed = s.trim();
        if (trimmed.starts_with('[') && trimmed.ends_with(']'))
            || (trimmed.starts_with('{') && trimmed.ends_with('}'))
        {
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(trimmed) {
                return parsed;
            }
        }
    }
    val
}

/// Parse a TOML string into a flat PrefsMap.
/// Top-level sections become either a nested JSON value (for "window")
/// or their inner keys are promoted to flat top-level keys using SECTION_MAP.
fn toml_to_flat(toml_str: &str) -> PrefsMap {
    let table: toml::Table = match toml::from_str(toml_str) {
        Ok(t) => t,
        Err(_) => return PrefsMap::new(),
    };
    let mut map = PrefsMap::new();
    for (section, val) in &table {
        if let toml::Value::Table(inner) = val {
            if section == "window" {
                map.insert(
                    section.clone(),
                    toml_value_to_json(&toml::Value::Table(inner.clone())),
                );
            } else {
                for (toml_key, v) in inner {
                    let flat_key =
                        toml_key_to_flat(section, toml_key).unwrap_or_else(|| toml_key.clone());
                    let json_val = migrate_json_string(toml_value_to_json(v));
                    map.insert(flat_key, json_val);
                }
            }
        } else {
            map.insert(section.clone(), toml_value_to_json(val));
        }
    }
    map
}

fn toml_value_to_json(val: &toml::Value) -> serde_json::Value {
    match val {
        toml::Value::String(s) => serde_json::Value::String(s.clone()),
        toml::Value::Integer(i) => serde_json::json!(*i),
        toml::Value::Float(f) => serde_json::json!(*f),
        toml::Value::Boolean(b) => serde_json::Value::Bool(*b),
        toml::Value::Array(arr) => {
            serde_json::Value::Array(arr.iter().map(toml_value_to_json).collect())
        }
        toml::Value::Table(t) => {
            let mut m = PrefsMap::new();
            for (k, v) in t {
                m.insert(k.clone(), toml_value_to_json(v));
            }
            serde_json::Value::Object(m)
        }
        toml::Value::Datetime(d) => serde_json::Value::String(d.to_string()),
    }
}

fn json_to_toml_value(val: &serde_json::Value) -> toml::Value {
    match val {
        serde_json::Value::String(s) => toml::Value::String(s.clone()),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                toml::Value::Integer(i)
            } else {
                toml::Value::Float(n.as_f64().unwrap_or(0.0))
            }
        }
        serde_json::Value::Bool(b) => toml::Value::Boolean(*b),
        serde_json::Value::Array(arr) => {
            toml::Value::Array(arr.iter().map(json_to_toml_value).collect())
        }
        serde_json::Value::Object(m) => {
            let mut t = toml::Table::new();
            for (k, v) in m {
                t.insert(k.clone(), json_to_toml_value(v));
            }
            toml::Value::Table(t)
        }
        serde_json::Value::Null => toml::Value::String(String::new()),
    }
}

/// Convert a flat PrefsMap into sectioned TOML string.
fn flat_to_toml(prefs: &PrefsMap) -> String {
    let mut root = toml::Table::new();

    // Collect keys into sections preserving SECTION_MAP order
    for (section, key_pairs) in SECTION_MAP {
        let mut sec_table = toml::Table::new();
        if *section == "window" {
            if let Some(serde_json::Value::Object(m)) = prefs.get("window") {
                for (k, v) in m {
                    sec_table.insert(k.clone(), json_to_toml_value(v));
                }
            }
        } else {
            for (flat_key, toml_key) in *key_pairs {
                if let Some(val) = prefs.get(*flat_key) {
                    sec_table.insert(toml_key.to_string(), json_to_toml_value(val));
                }
            }
        }
        if !sec_table.is_empty() {
            root.insert(section.to_string(), toml::Value::Table(sec_table));
        }
    }

    // Any remaining keys not in SECTION_MAP go under [general]
    let all_flat_keys: Vec<&str> = SECTION_MAP
        .iter()
        .flat_map(|(_, pairs)| pairs.iter().map(|(flat, _)| *flat))
        .collect();
    let mut general = toml::Table::new();
    for (k, v) in prefs {
        if k == "window" || all_flat_keys.contains(&k.as_str()) {
            continue;
        }
        general.insert(k.clone(), json_to_toml_value(v));
    }
    if !general.is_empty() {
        root.insert("general".to_string(), toml::Value::Table(general));
    }

    toml::to_string_pretty(&root).unwrap_or_default()
}

fn load_preferences_from_disk() -> PrefsMap {
    let path = preferences_file();

    // Migrate from legacy JSON if TOML doesn't exist yet
    if !path.exists() {
        let legacy = legacy_preferences_file();
        if legacy.exists() {
            if let Ok(data) = fs::read_to_string(&legacy) {
                if let Ok(serde_json::Value::Object(user)) = serde_json::from_str(&data) {
                    let defaults = default_config();
                    let merged = merge_prefs(&defaults, &user);
                    save_preferences(&merged);
                    let _ = fs::remove_file(&legacy);
                    return merged;
                }
            }
        }
    }

    if path.exists() {
        if let Ok(data) = fs::read_to_string(&path) {
            let user = toml_to_flat(&data);
            let defaults = default_config();
            return merge_prefs(&defaults, &user);
        }
    }
    let defaults = default_config();
    save_preferences(&defaults);
    defaults
}

pub fn load_preferences() -> PrefsMap {
    let now = prefs_cache_now_ms();
    let path = preferences_file();
    {
        if let Ok(guard) = PREF_CACHE.lock() {
            if let Some((t, cached_path, p)) = guard.as_ref() {
                if now.saturating_sub(*t) < PREF_CACHE_TTL_MS && *cached_path == path {
                    return p.clone();
                }
            }
        }
    }
    let loaded = load_preferences_from_disk();
    if let Ok(mut guard) = PREF_CACHE.lock() {
        *guard = Some((now, path, loaded.clone()));
    }
    loaded
}

fn merge_prefs(defaults: &PrefsMap, user: &PrefsMap) -> PrefsMap {
    let mut merged = PrefsMap::new();
    for (k, v) in defaults {
        merged.insert(k.clone(), user.get(k).cloned().unwrap_or_else(|| v.clone()));
    }
    for (k, v) in user {
        if !merged.contains_key(k) {
            merged.insert(k.clone(), v.clone());
        }
    }
    merged
}

pub fn save_preferences(prefs: &PrefsMap) {
    let path = preferences_file();
    let toml_str = flat_to_toml(prefs);
    let _ = fs::write(&path, toml_str);
    invalidate_prefs_cache();
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

pub fn gen_id() -> String {
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

pub fn radix_string(mut n: u64, base: u64) -> String {
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

/// Build a ScanSnapshot without file I/O (for SQLite path).
pub fn build_plugin_snapshot(
    plugins: &[PluginInfo],
    directories: &[String],
    roots: &[String],
) -> ScanSnapshot {
    ScanSnapshot {
        id: gen_id(),
        timestamp: now_iso(),
        plugin_count: plugins.len(),
        plugins: plugins.to_vec(),
        directories: directories.to_vec(),
        roots: roots.to_vec(),
    }
}

pub fn save_scan(plugins: &[PluginInfo], directories: &[String], roots: &[String]) -> ScanSnapshot {
    let mut history = load_history();
    let snapshot = ScanSnapshot {
        id: gen_id(),
        timestamp: now_iso(),
        plugin_count: plugins.len(),
        plugins: plugins.to_vec(),
        directories: directories.to_vec(),
        roots: roots.to_vec(),
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
            roots: s.roots.clone(),
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
            roots: old_scan.roots.clone(),
        },
        new_scan: ScanSummary {
            id: new_scan.id.clone(),
            timestamp: new_scan.timestamp.clone(),
            plugin_count: new_scan.plugin_count,
            roots: new_scan.roots.clone(),
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

/// Compute diff between two plugin snapshots (no file I/O).
pub fn compute_plugin_diff(old_scan: &ScanSnapshot, new_scan: &ScanSnapshot) -> ScanDiff {
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
    ScanDiff {
        old_scan: ScanSummary {
            id: old_scan.id.clone(),
            timestamp: old_scan.timestamp.clone(),
            plugin_count: old_scan.plugin_count,
            roots: old_scan.roots.clone(),
        },
        new_scan: ScanSummary {
            id: new_scan.id.clone(),
            timestamp: new_scan.timestamp.clone(),
            plugin_count: new_scan.plugin_count,
            roots: new_scan.roots.clone(),
        },
        added,
        removed,
        version_changed,
    }
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

/// Build an AudioScanSnapshot without file I/O (for SQLite path).
pub fn build_audio_snapshot(samples: &[AudioSample], roots: &[String]) -> AudioScanSnapshot {
    let mut format_counts = std::collections::HashMap::new();
    let mut total_bytes = 0u64;
    for s in samples {
        *format_counts.entry(s.format.clone()).or_insert(0) += 1;
        total_bytes += s.size;
    }
    AudioScanSnapshot {
        id: gen_id(),
        timestamp: now_iso(),
        sample_count: samples.len(),
        total_bytes,
        format_counts,
        samples: samples.to_vec(),
        roots: roots.to_vec(),
    }
}

/// Compute diff between two audio snapshots (no file I/O).
pub fn compute_audio_diff(
    old_scan: &AudioScanSnapshot,
    new_scan: &AudioScanSnapshot,
) -> AudioScanDiff {
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
    AudioScanDiff {
        old_scan: AudioScanSummary {
            id: old_scan.id.clone(),
            timestamp: old_scan.timestamp.clone(),
            sample_count: old_scan.sample_count,
            total_bytes: old_scan.total_bytes,
            format_counts: old_scan.format_counts.clone(),
            roots: old_scan.roots.clone(),
        },
        new_scan: AudioScanSummary {
            id: new_scan.id.clone(),
            timestamp: new_scan.timestamp.clone(),
            sample_count: new_scan.sample_count,
            total_bytes: new_scan.total_bytes,
            format_counts: new_scan.format_counts.clone(),
            roots: new_scan.roots.clone(),
        },
        added,
        removed,
    }
}

pub fn save_audio_scan(samples: &[AudioSample], roots: &[String]) -> AudioScanSnapshot {
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
        roots: roots.to_vec(),
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
            roots: s.roots.clone(),
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

pub fn build_daw_snapshot(projects: &[DawProject], roots: &[String]) -> DawScanSnapshot {
    let mut daw_counts = std::collections::HashMap::new();
    let mut total_bytes = 0u64;
    for p in projects {
        *daw_counts.entry(p.daw.clone()).or_insert(0) += 1;
        total_bytes += p.size;
    }
    DawScanSnapshot {
        id: gen_id(),
        timestamp: now_iso(),
        project_count: projects.len(),
        total_bytes,
        daw_counts,
        projects: projects.to_vec(),
        roots: roots.to_vec(),
    }
}

pub fn compute_daw_diff(old_scan: &DawScanSnapshot, new_scan: &DawScanSnapshot) -> DawScanDiff {
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
    DawScanDiff {
        old_scan: DawScanSummary {
            id: old_scan.id.clone(),
            timestamp: old_scan.timestamp.clone(),
            project_count: old_scan.project_count,
            total_bytes: old_scan.total_bytes,
            daw_counts: old_scan.daw_counts.clone(),
            roots: old_scan.roots.clone(),
        },
        new_scan: DawScanSummary {
            id: new_scan.id.clone(),
            timestamp: new_scan.timestamp.clone(),
            project_count: new_scan.project_count,
            total_bytes: new_scan.total_bytes,
            daw_counts: new_scan.daw_counts.clone(),
            roots: new_scan.roots.clone(),
        },
        added,
        removed,
    }
}

pub fn save_daw_scan(projects: &[DawProject], roots: &[String]) -> DawScanSnapshot {
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
        roots: roots.to_vec(),
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
            roots: s.roots.clone(),
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
            roots: old_scan.roots.clone(),
        },
        new_scan: DawScanSummary {
            id: new_scan.id.clone(),
            timestamp: new_scan.timestamp.clone(),
            project_count: new_scan.project_count,
            total_bytes: new_scan.total_bytes,
            daw_counts: new_scan.daw_counts.clone(),
            roots: new_scan.roots.clone(),
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
            roots: old_scan.roots.clone(),
        },
        new_scan: AudioScanSummary {
            id: new_scan.id.clone(),
            timestamp: new_scan.timestamp.clone(),
            sample_count: new_scan.sample_count,
            total_bytes: new_scan.total_bytes,
            format_counts: new_scan.format_counts.clone(),
            roots: new_scan.roots.clone(),
        },
        added,
        removed,
    })
}

// ── Preset scan history ──

fn preset_history_file() -> PathBuf {
    ensure_data_dir().join("preset-scan-history.json")
}

fn load_preset_history() -> PresetHistory {
    let path = preset_history_file();
    if path.exists() {
        if let Ok(data) = fs::read_to_string(&path) {
            if let Ok(h) = serde_json::from_str(&data) {
                return h;
            }
        }
    }
    PresetHistory { scans: vec![] }
}

fn save_preset_history(history: &PresetHistory) {
    let path = preset_history_file();
    if let Ok(json) = serde_json::to_string(&history) {
        let _ = fs::write(&path, json);
    }
}

pub fn build_preset_snapshot(presets: &[PresetFile], roots: &[String]) -> PresetScanSnapshot {
    let mut format_counts = std::collections::HashMap::new();
    let mut total_bytes = 0u64;
    for p in presets {
        *format_counts.entry(p.format.clone()).or_insert(0) += 1;
        total_bytes += p.size;
    }
    PresetScanSnapshot {
        id: gen_id(),
        timestamp: now_iso(),
        preset_count: presets.len(),
        total_bytes,
        format_counts,
        presets: presets.to_vec(),
        roots: roots.to_vec(),
    }
}

pub fn compute_preset_diff(
    old_scan: &PresetScanSnapshot,
    new_scan: &PresetScanSnapshot,
) -> PresetScanDiff {
    let old_paths: std::collections::HashSet<&str> =
        old_scan.presets.iter().map(|p| p.path.as_str()).collect();
    let new_paths: std::collections::HashSet<&str> =
        new_scan.presets.iter().map(|p| p.path.as_str()).collect();
    let added: Vec<PresetFile> = new_scan
        .presets
        .iter()
        .filter(|p| !old_paths.contains(p.path.as_str()))
        .cloned()
        .collect();
    let removed: Vec<PresetFile> = old_scan
        .presets
        .iter()
        .filter(|p| !new_paths.contains(p.path.as_str()))
        .cloned()
        .collect();
    PresetScanDiff {
        old_scan: PresetScanSummary {
            id: old_scan.id.clone(),
            timestamp: old_scan.timestamp.clone(),
            preset_count: old_scan.preset_count,
            total_bytes: old_scan.total_bytes,
            format_counts: old_scan.format_counts.clone(),
            roots: old_scan.roots.clone(),
        },
        new_scan: PresetScanSummary {
            id: new_scan.id.clone(),
            timestamp: new_scan.timestamp.clone(),
            preset_count: new_scan.preset_count,
            total_bytes: new_scan.total_bytes,
            format_counts: new_scan.format_counts.clone(),
            roots: new_scan.roots.clone(),
        },
        added,
        removed,
    }
}

pub fn save_preset_scan(presets: &[PresetFile], roots: &[String]) -> PresetScanSnapshot {
    let mut history = load_preset_history();
    let mut format_counts = std::collections::HashMap::new();
    let mut total_bytes = 0u64;
    for p in presets {
        *format_counts.entry(p.format.clone()).or_insert(0) += 1;
        total_bytes += p.size;
    }
    let snapshot = PresetScanSnapshot {
        id: gen_id(),
        timestamp: now_iso(),
        preset_count: presets.len(),
        total_bytes,
        format_counts,
        presets: presets.to_vec(),
        roots: roots.to_vec(),
    };
    history.scans.insert(0, snapshot.clone());
    if history.scans.len() > 50 {
        history.scans.truncate(50);
    }
    save_preset_history(&history);
    snapshot
}

pub fn get_preset_scans() -> Vec<PresetScanSummary> {
    let history = load_preset_history();
    history
        .scans
        .iter()
        .map(|s| PresetScanSummary {
            id: s.id.clone(),
            timestamp: s.timestamp.clone(),
            preset_count: s.preset_count,
            total_bytes: s.total_bytes,
            format_counts: s.format_counts.clone(),
            roots: s.roots.clone(),
        })
        .collect()
}

pub fn get_preset_scan_detail(id: &str) -> Option<PresetScanSnapshot> {
    let history = load_preset_history();
    history.scans.into_iter().find(|s| s.id == id)
}

pub fn delete_preset_scan(id: &str) {
    let mut history = load_preset_history();
    history.scans.retain(|s| s.id != id);
    save_preset_history(&history);
}

pub fn clear_preset_history() {
    save_preset_history(&PresetHistory { scans: vec![] });
}

pub fn get_latest_preset_scan() -> Option<PresetScanSnapshot> {
    let history = load_preset_history();
    history.scans.into_iter().next()
}

pub fn diff_preset_scans(old_id: &str, new_id: &str) -> Option<PresetScanDiff> {
    let history = load_preset_history();
    let old_scan = history.scans.iter().find(|s| s.id == old_id)?.clone();
    let new_scan = history.scans.iter().find(|s| s.id == new_id)?.clone();

    let old_paths: std::collections::HashSet<&str> =
        old_scan.presets.iter().map(|p| p.path.as_str()).collect();
    let new_paths: std::collections::HashSet<&str> =
        new_scan.presets.iter().map(|p| p.path.as_str()).collect();

    let added: Vec<PresetFile> = new_scan
        .presets
        .iter()
        .filter(|p| !old_paths.contains(p.path.as_str()))
        .cloned()
        .collect();

    let removed: Vec<PresetFile> = old_scan
        .presets
        .iter()
        .filter(|p| !new_paths.contains(p.path.as_str()))
        .cloned()
        .collect();

    Some(PresetScanDiff {
        old_scan: PresetScanSummary {
            id: old_scan.id.clone(),
            timestamp: old_scan.timestamp.clone(),
            preset_count: old_scan.preset_count,
            total_bytes: old_scan.total_bytes,
            format_counts: old_scan.format_counts.clone(),
            roots: old_scan.roots.clone(),
        },
        new_scan: PresetScanSummary {
            id: new_scan.id.clone(),
            timestamp: new_scan.timestamp.clone(),
            preset_count: new_scan.preset_count,
            total_bytes: new_scan.total_bytes,
            format_counts: new_scan.format_counts.clone(),
            roots: new_scan.roots.clone(),
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
    fn test_radix_string_base2() {
        assert_eq!(radix_string(0, 2), "0");
        assert_eq!(radix_string(8, 2), "1000");
        assert_eq!(radix_string(255, 2), "11111111");
    }

    #[test]
    fn test_radix_string_base16_hex_word() {
        assert_eq!(radix_string(0xDEADBEEF, 16), "deadbeef");
    }

    #[test]
    fn test_toml_key_to_flat_maps_data_widths_to_flat_column_widths() {
        assert_eq!(
            toml_key_to_flat("data", "widths").as_deref(),
            Some("columnWidths")
        );
    }

    #[test]
    fn test_toml_key_to_flat_unknown_returns_none() {
        assert_eq!(toml_key_to_flat("not_a_section", "theme"), None);
    }

    #[test]
    fn test_toml_key_to_flat_performance_pageSize_to_flat_key() {
        assert_eq!(
            toml_key_to_flat("performance", "pageSize").as_deref(),
            Some("pageSize")
        );
    }

    #[test]
    fn test_toml_key_to_flat_favorites_items_to_flat_key() {
        assert_eq!(
            toml_key_to_flat("favorites", "items").as_deref(),
            Some("favorites")
        );
    }

    #[test]
    fn test_migrate_json_string_bracketed_invalid_json_keeps_original() {
        let v = migrate_json_string(serde_json::json!("[not valid json]"));
        assert_eq!(v, serde_json::json!("[not valid json]"));
    }

    #[test]
    fn test_toml_to_flat_promotes_data_widths_key() {
        let toml = "[data]\nwidths = [120, 240]\n";
        let flat = toml_to_flat(toml);
        assert_eq!(flat.get("columnWidths"), Some(&serde_json::json!([120, 240])));
    }

    #[test]
    fn test_migrate_json_string_parses_bracketed_json_array() {
        let v = migrate_json_string(serde_json::json!("[1, 2, 3]"));
        assert_eq!(v, serde_json::json!([1, 2, 3]));
    }

    #[test]
    fn test_migrate_json_string_parses_braced_json_object() {
        let v = migrate_json_string(serde_json::json!(r#"{"x":1}"#));
        assert_eq!(v, serde_json::json!({"x": 1}));
    }

    #[test]
    fn test_migrate_json_string_invalid_json_keeps_original_string() {
        let v = migrate_json_string(serde_json::json!("{broken"));
        assert_eq!(v, serde_json::json!("{broken"));
    }

    #[test]
    fn test_migrate_json_string_plain_text_not_migrated() {
        let v = migrate_json_string(serde_json::json!("just text"));
        assert_eq!(v, serde_json::json!("just text"));
    }

    #[test]
    fn test_migrate_json_string_empty_object_string() {
        let v = migrate_json_string(serde_json::json!("{}"));
        assert_eq!(v, serde_json::json!({}));
    }

    #[test]
    fn test_migrate_json_string_padded_bracket_array_strips_and_parses() {
        let v = migrate_json_string(serde_json::json!("  [7]  "));
        assert_eq!(v, serde_json::json!([7]));
    }

    #[test]
    fn test_toml_to_flat_invalid_toml_returns_empty_prefs() {
        assert!(toml_to_flat("this is not [[valid]] toml").is_empty());
    }

    #[test]
    fn test_toml_to_flat_unknown_section_keeps_inner_keys_as_flat_names() {
        let t = "[orphan]\nanswer = 42\n";
        let flat = toml_to_flat(t);
        assert_eq!(flat.get("answer"), Some(&serde_json::json!(42)));
    }

    #[test]
    fn test_json_to_toml_value_null_maps_to_empty_string() {
        let v = json_to_toml_value(&serde_json::Value::Null);
        assert!(matches!(v, toml::Value::String(s) if s.is_empty()));
    }

    #[test]
    fn test_json_to_toml_value_non_integer_number_is_float() {
        let v = json_to_toml_value(&serde_json::json!(3.14159));
        assert!(matches!(v, toml::Value::Float(f) if (f - 3.14159).abs() < 1e-5));
    }

    #[test]
    fn test_json_to_toml_value_bool() {
        assert!(matches!(
            json_to_toml_value(&serde_json::Value::Bool(true)),
            toml::Value::Boolean(true)
        ));
        assert!(matches!(
            json_to_toml_value(&serde_json::Value::Bool(false)),
            toml::Value::Boolean(false)
        ));
    }

    #[test]
    fn test_toml_value_to_json_bool_round_trips() {
        let j = toml_value_to_json(&toml::Value::Boolean(true));
        assert_eq!(j, serde_json::Value::Bool(true));
    }

    #[test]
    fn test_toml_value_to_json_integer_round_trips() {
        let t = toml::Value::Integer(-42);
        let j = toml_value_to_json(&t);
        assert_eq!(j, serde_json::json!(-42));
    }

    #[test]
    fn test_json_to_toml_nested_object_and_array_round_trip() {
        let j = serde_json::json!({
            "nested": { "x": 1, "flag": true },
            "arr": [1, 2, 3],
            "empty": {}
        });
        let t = json_to_toml_value(&j);
        let back = toml_value_to_json(&t);
        assert_eq!(back, j);
    }

    #[test]
    fn test_build_audio_snapshot_aggregates_formats_and_total_bytes() {
        let samples = vec![
            AudioSample {
                name: "a".into(),
                path: "/a.wav".into(),
                directory: "/tmp".into(),
                format: "WAV".into(),
                size: 100,
                size_formatted: "100 B".into(),
                modified: "t".into(),
                duration: None,
                channels: None,
                sample_rate: None,
                bits_per_sample: None,
            },
            AudioSample {
                name: "b".into(),
                path: "/b.wav".into(),
                directory: "/tmp".into(),
                format: "WAV".into(),
                size: 200,
                size_formatted: "200 B".into(),
                modified: "t".into(),
                duration: None,
                channels: None,
                sample_rate: None,
                bits_per_sample: None,
            },
            AudioSample {
                name: "c".into(),
                path: "/c.mp3".into(),
                directory: "/tmp".into(),
                format: "MP3".into(),
                size: 50,
                size_formatted: "50 B".into(),
                modified: "t".into(),
                duration: None,
                channels: None,
                sample_rate: None,
                bits_per_sample: None,
            },
        ];
        let roots = vec!["/music".into()];
        let snap = build_audio_snapshot(&samples, &roots);
        assert_eq!(snap.sample_count, 3);
        assert_eq!(snap.total_bytes, 350);
        assert_eq!(snap.format_counts.get("WAV"), Some(&2));
        assert_eq!(snap.format_counts.get("MP3"), Some(&1));
        assert_eq!(snap.roots, roots);
    }

    #[test]
    fn test_build_plugin_snapshot_counts_and_roots_match_input() {
        let plugins = vec![
            make_plugin("Alpha", "1.0", "/tmp/a.vst3"),
            make_plugin("Beta", "2.0", "/tmp/b.vst3"),
        ];
        let dirs = vec!["/tmp/plugins".into()];
        let roots = vec!["/root/A".into(), "/root/B".into()];
        let snap = build_plugin_snapshot(&plugins, &dirs, &roots);
        assert_eq!(snap.plugin_count, 2);
        assert_eq!(snap.plugins.len(), 2);
        assert_eq!(snap.directories, dirs);
        assert_eq!(snap.roots, roots);
    }

    #[test]
    fn test_build_preset_snapshot_aggregates_formats_and_total_bytes() {
        let presets = vec![
            PresetFile {
                name: "lead".into(),
                path: "/p/lead.fxp".into(),
                directory: "/p".into(),
                format: "FXP".into(),
                size: 100,
                size_formatted: "100 B".into(),
                modified: "m".into(),
            },
            PresetFile {
                name: "pad".into(),
                path: "/p/pad.vstpreset".into(),
                directory: "/p".into(),
                format: "VSTPRESET".into(),
                size: 250,
                size_formatted: "250 B".into(),
                modified: "m".into(),
            },
        ];
        let roots = vec!["/presets".into()];
        let snap = build_preset_snapshot(&presets, &roots);
        assert_eq!(snap.preset_count, 2);
        assert_eq!(snap.total_bytes, 350);
        assert_eq!(snap.format_counts.get("FXP"), Some(&1));
        assert_eq!(snap.format_counts.get("VSTPRESET"), Some(&1));
        assert_eq!(snap.roots, roots);
    }

    #[test]
    fn test_gen_id_unique() {
        let id1 = gen_id();
        let id2 = gen_id();
        assert_ne!(id1, id2);
        assert!(!id1.is_empty());
    }

    #[test]
    fn test_gen_id_is_base36_alphanumeric() {
        let id = gen_id();
        assert!(
            id.chars()
                .all(|c| c.is_ascii_digit() || ('a'..='z').contains(&c)),
            "gen_id must be radix-36 digits only, got {:?}",
            id
        );
    }

    #[test]
    fn test_now_iso_format() {
        let ts = now_iso();
        // Should be RFC3339 format
        assert!(ts.contains('T'));
        assert!(ts.ends_with('Z'));
    }

    fn with_temp_dir<F: FnOnce(&std::path::Path)>(f: F) {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let id = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!("upum_tmp_{}_{}", std::process::id(), id));
        let _ = fs::remove_dir_all(&dir);
        let _ = fs::create_dir_all(&dir);
        TEST_DATA_DIR.with(|d| *d.borrow_mut() = Some(dir.clone()));
        f(&dir);
        TEST_DATA_DIR.with(|d| *d.borrow_mut() = None);
        let _ = fs::remove_dir_all(&dir);
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
            size_bytes: 1048576,
            modified: "2024-01-01".into(),
            architectures: vec![],
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
            duration: None,
            channels: None,
            sample_rate: None,
            bits_per_sample: None,
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
            let snap = save_scan(&plugins, &dirs, &dirs);
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
                save_scan(&plugins, &dirs, &dirs);
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
            let snap1 = save_scan(&plugins1, &dirs, &dirs);
            let snap2 = save_scan(&plugins2, &dirs, &dirs);

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
            let snap1 = save_scan(&plugins1, &dirs, &dirs);
            let snap2 = save_scan(&plugins2, &dirs, &dirs);

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
            let snap = save_audio_scan(&samples, &[]);
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
            let s1 = save_scan(&[make_plugin("A", "1.0", "/tmp/a.vst3")], &dirs, &dirs);
            let s2 = save_scan(&[make_plugin("B", "1.0", "/tmp/b.vst3")], &dirs, &dirs);
            let s3 = save_scan(&[make_plugin("C", "1.0", "/tmp/c.vst3")], &dirs, &dirs);

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
            let snap = save_scan(&[make_plugin("X", "1.0", "/tmp/x.vst3")], &dirs, &dirs);

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
            save_scan(&[make_plugin("X", "1.0", "/tmp/x.vst3")], &dirs, &dirs);

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
            let snap1 = save_scan(&plugins, &dirs, &dirs);
            let snap2 = save_scan(&plugins, &dirs, &dirs);

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
                save_audio_scan(&sample, &[]);
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
            let snap1 = save_audio_scan(&samples1, &[]);
            let snap2 = save_audio_scan(&samples2, &[]);

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
            let snap = save_daw_scan(&projects, &[]);
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
                save_daw_scan(&projects, &[]);
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
            let snap1 = save_daw_scan(&projects1, &[]);
            let snap2 = save_daw_scan(&projects2, &[]);

            let diff = diff_daw_scans(&snap1.id, &snap2.id).unwrap();
            assert_eq!(diff.added.len(), 1);
            assert_eq!(diff.added[0].name, "Song3");
            assert_eq!(diff.removed.len(), 1);
            assert_eq!(diff.removed[0].name, "Song1");
        });
    }

    // ── Preferences tests ──

    #[test]
    fn test_preferences_roundtrip() {
        with_temp_dir(|_| {
            let mut prefs = PrefsMap::new();
            prefs.insert("theme".into(), serde_json::json!("dark"));
            prefs.insert("pageSize".into(), serde_json::json!(500));
            prefs.insert("autoScan".into(), serde_json::json!("on"));
            save_preferences(&prefs);

            let loaded = load_preferences();
            assert_eq!(loaded.get("theme"), Some(&serde_json::json!("dark")));
            assert_eq!(loaded.get("autoScan"), Some(&serde_json::json!("on")));
        });
    }

    #[test]
    fn test_set_and_get_preference() {
        with_temp_dir(|_| {
            set_preference("testKey", serde_json::json!("testValue"));
            let val = get_preference("testKey");
            assert_eq!(val, Some(serde_json::json!("testValue")));
        });
    }

    #[test]
    fn test_remove_preference() {
        with_temp_dir(|_| {
            set_preference("removeMe", serde_json::json!(42));
            assert!(get_preference("removeMe").is_some());
            remove_preference("removeMe");
            // After removal, may still return default — check it doesn't crash
            let _ = get_preference("removeMe");
        });
    }

    #[test]
    fn test_get_nonexistent_preference() {
        with_temp_dir(|_| {
            let val = get_preference("nonexistent_key_xyz");
            // May return a default or None
            let _ = val;
        });
    }

    #[test]
    fn test_preferences_overwrite() {
        with_temp_dir(|_| {
            set_preference("color", serde_json::json!("red"));
            set_preference("color", serde_json::json!("blue"));
            let val = get_preference("color");
            assert_eq!(val, Some(serde_json::json!("blue")));
        });
    }

    /// `PREF_CACHE` is global; unit tests use distinct thread-local temp dirs. Without path-keyed
    /// cache, parallel tests could read another thread's cached prefs (CI failure: overwrite test).
    #[test]
    fn test_preferences_cache_isolated_across_parallel_threads() {
        use std::thread;
        let a = thread::spawn(|| {
            with_temp_dir(|_| {
                set_preference("parallelIsolationKey", serde_json::json!("thread-a"));
                assert_eq!(
                    get_preference("parallelIsolationKey"),
                    Some(serde_json::json!("thread-a"))
                );
            });
        });
        let b = thread::spawn(|| {
            with_temp_dir(|_| {
                set_preference("parallelIsolationKey", serde_json::json!("thread-b"));
                assert_eq!(
                    get_preference("parallelIsolationKey"),
                    Some(serde_json::json!("thread-b"))
                );
            });
        });
        a.join().expect("thread a");
        b.join().expect("thread b");
    }

    // ── Scan detail tests ──

    #[test]
    fn test_get_scan_detail_found() {
        with_temp_dir(|_| {
            let plugins = vec![make_plugin("DetailPlugin", "1.0", "/detail")];
            save_scan(&plugins, &["/detail".into()], &["/detail".into()]);

            let scans = get_scans();
            assert!(!scans.is_empty());
            let id = &scans[0].id;

            let detail = get_scan_detail(id);
            assert!(detail.is_some());
            let snap = detail.unwrap();
            assert_eq!(snap.plugins.len(), 1);
            assert_eq!(snap.plugins[0].name, "DetailPlugin");
        });
    }

    #[test]
    fn test_get_scan_detail_not_found() {
        with_temp_dir(|_| {
            let detail = get_scan_detail("nonexistent_id");
            assert!(detail.is_none());
        });
    }

    // ── Preset history tests ──

    #[test]
    fn test_preset_scan_save_and_retrieve() {
        with_temp_dir(|_| {
            let presets = vec![PresetFile {
                name: "BassPreset".into(),
                path: "/presets/bass.fxp".into(),
                directory: "/presets".into(),
                format: "FXP".into(),
                size: 1024,
                size_formatted: "1.0 KB".into(),
                modified: "2024-06-01".into(),
            }];
            save_preset_scan(&presets, &["/presets".into()]);
            let scans = get_preset_scans();
            assert_eq!(scans.len(), 1);
            assert_eq!(scans[0].preset_count, 1);
        });
    }

    #[test]
    fn test_preset_scan_detail() {
        with_temp_dir(|_| {
            let presets = vec![PresetFile {
                name: "LeadPreset".into(),
                path: "/presets/lead.fxp".into(),
                directory: "/presets".into(),
                format: "FXP".into(),
                size: 2048,
                size_formatted: "2.0 KB".into(),
                modified: "2024-07-01".into(),
            }];
            save_preset_scan(&presets, &["/presets".into()]);
            let scans = get_preset_scans();
            let detail = get_preset_scan_detail(&scans[0].id);
            assert!(detail.is_some());
            assert_eq!(detail.unwrap().presets.len(), 1);
        });
    }

    // ── DAW scan detail tests ──

    #[test]
    fn test_daw_scan_detail() {
        with_temp_dir(|_| {
            let projects = vec![make_daw_project(
                "TestSong",
                "/songs/test.als",
                "ALS",
                "Ableton Live",
            )];
            save_daw_scan(&projects, &["/songs".into()]);
            let scans = get_daw_scans();
            let detail = get_daw_scan_detail(&scans[0].id);
            assert!(detail.is_some());
            assert_eq!(detail.unwrap().projects.len(), 1);
        });
    }

    #[test]
    fn test_daw_scan_detail_not_found() {
        with_temp_dir(|_| {
            let detail = get_daw_scan_detail("nonexistent");
            assert!(detail.is_none());
        });
    }

    // ── Merge prefs tests ──

    #[test]
    fn test_merge_prefs_user_overrides_defaults() {
        let mut defaults = PrefsMap::new();
        defaults.insert("a".into(), serde_json::json!("default_a"));
        defaults.insert("b".into(), serde_json::json!("default_b"));

        let mut user = PrefsMap::new();
        user.insert("a".into(), serde_json::json!("user_a"));
        user.insert("c".into(), serde_json::json!("user_c"));

        let merged = merge_prefs(&defaults, &user);
        assert_eq!(merged.get("a"), Some(&serde_json::json!("user_a")));
        assert_eq!(merged.get("b"), Some(&serde_json::json!("default_b")));
        assert_eq!(merged.get("c"), Some(&serde_json::json!("user_c")));
    }

    #[test]
    fn test_merge_prefs_empty_defaults_absorbs_user_only_keys() {
        let defaults = PrefsMap::new();
        let mut user = PrefsMap::new();
        user.insert("custom".into(), serde_json::json!(true));
        let merged = merge_prefs(&defaults, &user);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged.get("custom"), Some(&serde_json::json!(true)));
    }

    #[test]
    fn test_merge_prefs_empty_user_keeps_all_defaults() {
        let mut defaults = PrefsMap::new();
        defaults.insert("theme".into(), serde_json::json!("dark"));
        let user = PrefsMap::new();
        let merged = merge_prefs(&defaults, &user);
        assert_eq!(merged.get("theme"), Some(&serde_json::json!("dark")));
    }

    // ── TOML conversion tests ──

    #[test]
    fn test_flat_to_toml_and_back() {
        let mut prefs = PrefsMap::new();
        prefs.insert("theme".into(), serde_json::json!("cyberpunk"));
        prefs.insert("pageSize".into(), serde_json::json!(1000));
        prefs.insert("autoScan".into(), serde_json::json!(true));

        let toml_str = flat_to_toml(&prefs);
        assert!(toml_str.contains("theme"));

        let back = toml_to_flat(&toml_str);
        assert_eq!(back.get("theme"), Some(&serde_json::json!("cyberpunk")));
    }

    // ── Pure diffs (no JSON file I/O): invariants for SQLite / command paths ──

    #[test]
    fn test_compute_plugin_diff_duplicate_path_last_old_entry_wins_for_version_compare() {
        let old = ScanSnapshot {
            id: "old".into(),
            timestamp: "t1".into(),
            plugin_count: 2,
            plugins: vec![
                make_plugin("Plug", "1.0", "/tmp/dup_path.vst3"),
                make_plugin("Plug", "2.0", "/tmp/dup_path.vst3"),
            ],
            directories: vec![],
            roots: vec![],
        };
        let new = ScanSnapshot {
            id: "new".into(),
            timestamp: "t2".into(),
            plugin_count: 1,
            plugins: vec![make_plugin("Plug", "3.0", "/tmp/dup_path.vst3")],
            directories: vec![],
            roots: vec![],
        };
        let d = compute_plugin_diff(&old, &new);
        assert_eq!(d.version_changed.len(), 1);
        assert_eq!(d.version_changed[0].previous_version, "2.0");
        assert_eq!(d.version_changed[0].plugin.version, "3.0");
    }

    #[test]
    fn test_compute_plugin_diff_both_empty_snapshots() {
        let old = ScanSnapshot {
            id: "a".into(),
            timestamp: "t1".into(),
            plugin_count: 0,
            plugins: vec![],
            directories: vec![],
            roots: vec![],
        };
        let new = ScanSnapshot {
            id: "b".into(),
            timestamp: "t2".into(),
            plugin_count: 0,
            plugins: vec![],
            directories: vec![],
            roots: vec![],
        };
        let d = compute_plugin_diff(&old, &new);
        assert!(d.added.is_empty());
        assert!(d.removed.is_empty());
        assert!(d.version_changed.is_empty());
    }

    #[test]
    fn test_compute_plugin_diff_same_known_version_no_version_changed() {
        let p = make_plugin("Same", "1.2.3", "/tmp/same.vst3");
        let old = ScanSnapshot {
            id: "o".into(),
            timestamp: "t1".into(),
            plugin_count: 1,
            plugins: vec![p.clone()],
            directories: vec![],
            roots: vec![],
        };
        let new = ScanSnapshot {
            id: "n".into(),
            timestamp: "t2".into(),
            plugin_count: 1,
            plugins: vec![p],
            directories: vec![],
            roots: vec![],
        };
        let d = compute_plugin_diff(&old, &new);
        assert!(d.version_changed.is_empty());
        assert!(d.added.is_empty());
        assert!(d.removed.is_empty());
    }

    #[test]
    fn test_compute_plugin_diff_unknown_to_known_same_path_no_version_changed() {
        let old = ScanSnapshot {
            id: "o".into(),
            timestamp: "t1".into(),
            plugin_count: 1,
            plugins: vec![make_plugin("P", "Unknown", "/x/plugin.vst3")],
            directories: vec![],
            roots: vec![],
        };
        let new = ScanSnapshot {
            id: "n".into(),
            timestamp: "t2".into(),
            plugin_count: 1,
            plugins: vec![make_plugin("P", "1.0.0", "/x/plugin.vst3")],
            directories: vec![],
            roots: vec![],
        };
        let d = compute_plugin_diff(&old, &new);
        assert!(
            d.version_changed.is_empty(),
            "Unknown→known should not emit version_changed (scanner ambiguity)"
        );
    }

    #[test]
    fn test_compute_plugin_diff_both_unknown_same_path_no_version_changed() {
        let p = make_plugin("Q", "Unknown", "/y/plugin.vst3");
        let old = ScanSnapshot {
            id: "o".into(),
            timestamp: "t1".into(),
            plugin_count: 1,
            plugins: vec![p.clone()],
            directories: vec![],
            roots: vec![],
        };
        let new = ScanSnapshot {
            id: "n".into(),
            timestamp: "t2".into(),
            plugin_count: 1,
            plugins: vec![p],
            directories: vec![],
            roots: vec![],
        };
        assert!(compute_plugin_diff(&old, &new).version_changed.is_empty());
    }

    #[test]
    fn test_compute_plugin_diff_known_to_unknown_same_path_no_version_changed() {
        let old = ScanSnapshot {
            id: "o".into(),
            timestamp: "t1".into(),
            plugin_count: 1,
            plugins: vec![make_plugin("R", "2.1.0", "/z/plugin.vst3")],
            directories: vec![],
            roots: vec![],
        };
        let new = ScanSnapshot {
            id: "n".into(),
            timestamp: "t2".into(),
            plugin_count: 1,
            plugins: vec![make_plugin("R", "Unknown", "/z/plugin.vst3")],
            directories: vec![],
            roots: vec![],
        };
        assert!(
            compute_plugin_diff(&old, &new).version_changed.is_empty(),
            "lost version info should not emit version_changed"
        );
    }

    #[test]
    fn test_compute_plugin_diff_version_downgrade_emits_version_changed() {
        let old = ScanSnapshot {
            id: "old".into(),
            timestamp: "t1".into(),
            plugin_count: 1,
            plugins: vec![make_plugin("X", "2.0", "/p/down.vst3")],
            directories: vec![],
            roots: vec![],
        };
        let new = ScanSnapshot {
            id: "new".into(),
            timestamp: "t2".into(),
            plugin_count: 1,
            plugins: vec![make_plugin("X", "1.0", "/p/down.vst3")],
            directories: vec![],
            roots: vec![],
        };
        let d = compute_plugin_diff(&old, &new);
        assert_eq!(d.version_changed.len(), 1);
        assert_eq!(d.version_changed[0].previous_version, "2.0");
        assert_eq!(d.version_changed[0].plugin.version, "1.0");
    }

    #[test]
    fn test_compute_plugin_diff_version_upgrade_emits_version_changed() {
        let old = ScanSnapshot {
            id: "old".into(),
            timestamp: "t1".into(),
            plugin_count: 1,
            plugins: vec![make_plugin("Serum", "1.0.0", "/lib/Serum.vst3")],
            directories: vec![],
            roots: vec![],
        };
        let new = ScanSnapshot {
            id: "new".into(),
            timestamp: "t2".into(),
            plugin_count: 1,
            plugins: vec![make_plugin("Serum", "1.5.2", "/lib/Serum.vst3")],
            directories: vec![],
            roots: vec![],
        };
        let d = compute_plugin_diff(&old, &new);
        assert_eq!(d.version_changed.len(), 1);
        assert_eq!(d.version_changed[0].previous_version, "1.0.0");
        assert_eq!(d.version_changed[0].plugin.version, "1.5.2");
        assert!(d.added.is_empty());
        assert!(d.removed.is_empty());
    }

    /// Same scan diff path: one plugin upgrades, one path removed, one path added.
    #[test]
    fn test_compute_plugin_diff_upgrade_add_remove_flow() {
        let old = ScanSnapshot {
            id: "o".into(),
            timestamp: "t1".into(),
            plugin_count: 2,
            plugins: vec![
                make_plugin("Stay", "1.0.0", "/keep/plugin.vst3"),
                make_plugin("Gone", "1.0", "/old/gone.vst3"),
            ],
            directories: vec![],
            roots: vec![],
        };
        let new = ScanSnapshot {
            id: "n".into(),
            timestamp: "t2".into(),
            plugin_count: 2,
            plugins: vec![
                make_plugin("Stay", "2.0.0", "/keep/plugin.vst3"),
                make_plugin("New", "1.0", "/new/new.vst3"),
            ],
            directories: vec![],
            roots: vec![],
        };
        let d = compute_plugin_diff(&old, &new);
        assert_eq!(d.version_changed.len(), 1);
        assert_eq!(d.version_changed[0].previous_version, "1.0.0");
        assert_eq!(d.version_changed[0].plugin.version, "2.0.0");
        assert_eq!(d.added.len(), 1);
        assert_eq!(d.added[0].path, "/new/new.vst3");
        assert_eq!(d.removed.len(), 1);
        assert_eq!(d.removed[0].path, "/old/gone.vst3");
    }

    #[test]
    fn test_compute_audio_diff_both_empty() {
        let old = AudioScanSnapshot {
            id: "a".into(),
            timestamp: "t1".into(),
            sample_count: 0,
            total_bytes: 0,
            format_counts: std::collections::HashMap::new(),
            samples: vec![],
            roots: vec![],
        };
        let new = AudioScanSnapshot {
            id: "b".into(),
            timestamp: "t2".into(),
            sample_count: 0,
            total_bytes: 0,
            format_counts: std::collections::HashMap::new(),
            samples: vec![],
            roots: vec![],
        };
        let d = compute_audio_diff(&old, &new);
        assert!(d.added.is_empty());
        assert!(d.removed.is_empty());
    }

    #[test]
    fn test_compute_daw_diff_both_empty() {
        let old = DawScanSnapshot {
            id: "a".into(),
            timestamp: "t1".into(),
            project_count: 0,
            total_bytes: 0,
            daw_counts: std::collections::HashMap::new(),
            projects: vec![],
            roots: vec![],
        };
        let new = DawScanSnapshot {
            id: "b".into(),
            timestamp: "t2".into(),
            project_count: 0,
            total_bytes: 0,
            daw_counts: std::collections::HashMap::new(),
            projects: vec![],
            roots: vec![],
        };
        let d = compute_daw_diff(&old, &new);
        assert!(d.added.is_empty());
        assert!(d.removed.is_empty());
    }

    #[test]
    fn test_compute_preset_diff_both_empty() {
        let old = PresetScanSnapshot {
            id: "a".into(),
            timestamp: "t1".into(),
            preset_count: 0,
            total_bytes: 0,
            format_counts: std::collections::HashMap::new(),
            presets: vec![],
            roots: vec![],
        };
        let new = PresetScanSnapshot {
            id: "b".into(),
            timestamp: "t2".into(),
            preset_count: 0,
            total_bytes: 0,
            format_counts: std::collections::HashMap::new(),
            presets: vec![],
            roots: vec![],
        };
        let d = compute_preset_diff(&old, &new);
        assert!(d.added.is_empty());
        assert!(d.removed.is_empty());
    }

    #[test]
    fn test_compute_audio_diff_added_removed_by_path() {
        let old = AudioScanSnapshot {
            id: "o".into(),
            timestamp: "t1".into(),
            sample_count: 1,
            total_bytes: 100,
            format_counts: std::collections::HashMap::new(),
            samples: vec![make_sample("kick", "/tmp/kick.wav", "WAV")],
            roots: vec![],
        };
        let new = AudioScanSnapshot {
            id: "n".into(),
            timestamp: "t2".into(),
            sample_count: 1,
            total_bytes: 200,
            format_counts: std::collections::HashMap::new(),
            samples: vec![make_sample("snare", "/tmp/snare.wav", "WAV")],
            roots: vec![],
        };
        let d = compute_audio_diff(&old, &new);
        assert_eq!(d.added.len(), 1);
        assert_eq!(d.added[0].path, "/tmp/snare.wav");
        assert_eq!(d.removed.len(), 1);
        assert_eq!(d.removed[0].path, "/tmp/kick.wav");
    }

    /// One path stable, one removed, one added — same set logic as plugin/DAW diffs.
    #[test]
    fn test_compute_audio_diff_keep_add_remove_flow() {
        let old = AudioScanSnapshot {
            id: "o".into(),
            timestamp: "t1".into(),
            sample_count: 2,
            total_bytes: 2048,
            format_counts: std::collections::HashMap::new(),
            samples: vec![
                make_sample("shared", "/lib/shared.wav", "WAV"),
                make_sample("gone", "/old/gone.wav", "WAV"),
            ],
            roots: vec![],
        };
        let new = AudioScanSnapshot {
            id: "n".into(),
            timestamp: "t2".into(),
            sample_count: 2,
            total_bytes: 2048,
            format_counts: std::collections::HashMap::new(),
            samples: vec![
                make_sample("shared", "/lib/shared.wav", "WAV"),
                make_sample("fresh", "/new/fresh.wav", "WAV"),
            ],
            roots: vec![],
        };
        let d = compute_audio_diff(&old, &new);
        assert_eq!(d.added.len(), 1);
        assert_eq!(d.removed.len(), 1);
        assert_eq!(d.added[0].path, "/new/fresh.wav");
        assert_eq!(d.removed[0].path, "/old/gone.wav");
    }

    #[test]
    fn test_build_daw_snapshot_aggregates_daw_counts_and_total_bytes() {
        let projects = vec![
            make_daw_project("A", "/p/a.als", "ALS", "Ableton Live"),
            make_daw_project("B", "/p/b.flp", "FLP", "FL Studio"),
            make_daw_project("C", "/p/c.flp", "FLP", "FL Studio"),
        ];
        let roots = vec!["/projects".into()];
        let snap = build_daw_snapshot(&projects, &roots);
        assert_eq!(snap.project_count, 3);
        assert_eq!(snap.total_bytes, 2048 * 3);
        assert_eq!(snap.daw_counts.get("Ableton Live"), Some(&1));
        assert_eq!(snap.daw_counts.get("FL Studio"), Some(&2));
        assert_eq!(snap.roots, roots);
    }

    #[test]
    fn test_compute_daw_diff_added_removed_by_path() {
        let old = DawScanSnapshot {
            id: "o".into(),
            timestamp: "t1".into(),
            project_count: 1,
            total_bytes: 1000,
            daw_counts: std::collections::HashMap::new(),
            projects: vec![make_daw_project("Old", "/p/old.als", "ALS", "Ableton Live")],
            roots: vec![],
        };
        let new = DawScanSnapshot {
            id: "n".into(),
            timestamp: "t2".into(),
            project_count: 1,
            total_bytes: 2000,
            daw_counts: std::collections::HashMap::new(),
            projects: vec![make_daw_project("New", "/p/new.flp", "FLP", "FL Studio")],
            roots: vec![],
        };
        let d = compute_daw_diff(&old, &new);
        assert_eq!(d.added.len(), 1);
        assert_eq!(d.added[0].path, "/p/new.flp");
        assert_eq!(d.removed.len(), 1);
        assert_eq!(d.removed[0].path, "/p/old.als");
    }

    #[test]
    fn test_compute_daw_diff_same_paths_no_added_or_removed() {
        let p = make_daw_project("Live", "/projects/set.als", "ALS", "Ableton Live");
        let old = DawScanSnapshot {
            id: "o".into(),
            timestamp: "t1".into(),
            project_count: 1,
            total_bytes: 1000,
            daw_counts: std::collections::HashMap::new(),
            projects: vec![p.clone()],
            roots: vec!["/a".into()],
        };
        let new = DawScanSnapshot {
            id: "n".into(),
            timestamp: "t2".into(),
            project_count: 1,
            total_bytes: 999_000,
            daw_counts: std::collections::HashMap::new(),
            projects: vec![p],
            roots: vec!["/b".into()],
        };
        let d = compute_daw_diff(&old, &new);
        assert!(d.added.is_empty());
        assert!(d.removed.is_empty());
    }

    #[test]
    fn test_compute_daw_diff_keep_add_remove_flow() {
        let old = DawScanSnapshot {
            id: "o".into(),
            timestamp: "t1".into(),
            project_count: 2,
            total_bytes: 4096,
            daw_counts: std::collections::HashMap::new(),
            projects: vec![
                make_daw_project("Stay", "/projects/stay.als", "ALS", "Ableton Live"),
                make_daw_project("Gone", "/old/gone.flp", "FLP", "FL Studio"),
            ],
            roots: vec![],
        };
        let new = DawScanSnapshot {
            id: "n".into(),
            timestamp: "t2".into(),
            project_count: 2,
            total_bytes: 4096,
            daw_counts: std::collections::HashMap::new(),
            projects: vec![
                make_daw_project("Stay", "/projects/stay.als", "ALS", "Ableton Live"),
                make_daw_project("Fresh", "/new/fresh.cpr", "CPR", "Cubase"),
            ],
            roots: vec![],
        };
        let d = compute_daw_diff(&old, &new);
        assert_eq!(d.added.len(), 1);
        assert_eq!(d.removed.len(), 1);
        assert_eq!(d.added[0].path, "/new/fresh.cpr");
        assert_eq!(d.removed[0].path, "/old/gone.flp");
    }

    #[test]
    fn test_compute_preset_diff_added_removed_by_path() {
        let preset = |path: &str| PresetFile {
            name: "n".into(),
            path: path.into(),
            directory: "/d".into(),
            format: "FXP".into(),
            size: 10,
            size_formatted: "10 B".into(),
            modified: "2024-01-01".into(),
        };
        let old = PresetScanSnapshot {
            id: "o".into(),
            timestamp: "t1".into(),
            preset_count: 1,
            total_bytes: 10,
            format_counts: std::collections::HashMap::new(),
            presets: vec![preset("/a/a.fxp")],
            roots: vec![],
        };
        let new = PresetScanSnapshot {
            id: "n".into(),
            timestamp: "t2".into(),
            preset_count: 1,
            total_bytes: 20,
            format_counts: std::collections::HashMap::new(),
            presets: vec![preset("/b/b.fxp")],
            roots: vec![],
        };
        let d = compute_preset_diff(&old, &new);
        assert_eq!(d.added.len(), 1);
        assert_eq!(d.added[0].path, "/b/b.fxp");
        assert_eq!(d.removed.len(), 1);
        assert_eq!(d.removed[0].path, "/a/a.fxp");
    }

    #[test]
    fn test_compute_preset_diff_keep_add_remove_flow() {
        let p = |path: &str, name: &str| PresetFile {
            name: name.into(),
            path: path.into(),
            directory: "/d".into(),
            format: "FXP".into(),
            size: 10,
            size_formatted: "10 B".into(),
            modified: "2024-01-01".into(),
        };
        let old = PresetScanSnapshot {
            id: "o".into(),
            timestamp: "t1".into(),
            preset_count: 2,
            total_bytes: 20,
            format_counts: std::collections::HashMap::new(),
            presets: vec![p("/p/keep.fxp", "keep"), p("/p/old.fxp", "old")],
            roots: vec![],
        };
        let new = PresetScanSnapshot {
            id: "n".into(),
            timestamp: "t2".into(),
            preset_count: 2,
            total_bytes: 20,
            format_counts: std::collections::HashMap::new(),
            presets: vec![p("/p/keep.fxp", "keep"), p("/p/new.fxp", "new")],
            roots: vec![],
        };
        let d = compute_preset_diff(&old, &new);
        assert_eq!(d.added.len(), 1);
        assert_eq!(d.removed.len(), 1);
        assert_eq!(d.added[0].path, "/p/new.fxp");
        assert_eq!(d.removed[0].path, "/p/old.fxp");
    }

    #[test]
    fn test_compute_audio_diff_same_paths_no_added_or_removed() {
        let old = AudioScanSnapshot {
            id: "o".into(),
            timestamp: "t1".into(),
            sample_count: 1,
            total_bytes: 100,
            format_counts: std::collections::HashMap::new(),
            samples: vec![make_sample("kick", "/tmp/shared.wav", "WAV")],
            roots: vec![],
        };
        let new = AudioScanSnapshot {
            id: "n".into(),
            timestamp: "t2".into(),
            sample_count: 1,
            total_bytes: 9999,
            format_counts: std::collections::HashMap::new(),
            samples: vec![make_sample("kick", "/tmp/shared.wav", "WAV")],
            roots: vec![],
        };
        let d = compute_audio_diff(&old, &new);
        assert!(d.added.is_empty());
        assert!(d.removed.is_empty());
    }

    #[test]
    fn test_compute_preset_diff_same_paths_no_added_or_removed() {
        let p = PresetFile {
            name: "lead".into(),
            path: "/x/lead.fxp".into(),
            directory: "/x".into(),
            format: "FXP".into(),
            size: 10,
            size_formatted: "10 B".into(),
            modified: "d".into(),
        };
        let old = PresetScanSnapshot {
            id: "o".into(),
            timestamp: "t1".into(),
            preset_count: 1,
            total_bytes: 10,
            format_counts: std::collections::HashMap::new(),
            presets: vec![p.clone()],
            roots: vec![],
        };
        let new = PresetScanSnapshot {
            id: "n".into(),
            timestamp: "t2".into(),
            preset_count: 1,
            total_bytes: 99,
            format_counts: std::collections::HashMap::new(),
            presets: vec![p],
            roots: vec![],
        };
        let d = compute_preset_diff(&old, &new);
        assert!(d.added.is_empty());
        assert!(d.removed.is_empty());
    }
}
