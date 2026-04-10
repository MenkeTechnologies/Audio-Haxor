//! AUDIO_HAXOR — Tauri v2 desktop app for audio plugin management.
//!
//! This crate provides the Rust backend for scanning audio plugins (VST2/VST3/AU/CLAP),
//! audio samples, DAW project files, and presets. It includes KVR Audio version
//! checking, scan history with diffing, and export to JSON/TOML/CSV/TSV/PDF.
//!
//! # Modules
//!
//! - [`scanner`] — Plugin filesystem scanner with architecture detection
//! - [`scanner_skip_dirs`] — Shared directory-name blocklist for recursive scans
//! - [`audio_extensions`] — Canonical audio sample extension list (scanner, walker, App Info)
//! - [`audio_scanner`] — Audio sample discovery and metadata extraction
//! - [`daw_scanner`] — DAW project scanner (14+ formats)
//! - [`preset_scanner`] — Plugin preset discovery
//! - [`audio_engine`] — Spawns the `audio-engine` AudioEngine (JUCE: devices, playback, VST3/AU scan) via stdin/stdout JSON
//! - [`kvr`] — KVR Audio scraper and version checker
//! - [`history`] — Scan history persistence, diffing, and preferences
//! - [`content_hash`] — SHA-256 file hashing for byte-identical duplicate detection

pub mod app_i18n;
pub mod audio_engine;
pub mod audio_extensions;
pub mod audio_scanner;
pub mod bpm;
pub mod bulk_stat;
pub mod content_hash;
pub mod daw_scanner;
pub mod db;
pub mod file_watcher;
pub mod history;
pub mod key_detect;
pub mod kvr;
pub mod lufs;
pub mod midi;
pub mod midi_scanner;
pub mod native_menu;
mod open_with_app;
pub mod pdf_meta;
pub mod path_norm;
pub mod pdf_scanner;
pub mod preset_scanner;
pub mod scanner;
pub mod scanner_skip_dirs;
pub mod similarity;
pub mod tray_menu;
pub mod unified_walker;
pub mod xref;

/// Shared utility: format bytes to human-readable string.
pub fn format_size(bytes: u64) -> String {
    if bytes == 0 {
        return "0 B".into();
    }
    let units = ["B", "KB", "MB", "GB", "TB"];
    let i = (bytes as f64).log(1024.0).floor() as usize;
    let i = i.min(units.len() - 1);
    format!("{:.1} {}", bytes as f64 / 1024f64.powi(i as i32), units[i])
}

use history::{AudioSample, DawProject, KvrCacheUpdateEntry, PdfFile, PresetFile};
use path_norm::normalize_path_for_db;
use scanner::PluginInfo;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager};

/// Domain string for SQLite `directory_scan_state` — shared by unified and standalone walkers.
pub const DIRECTORY_SCAN_INCREMENTAL_DOMAIN: &str = "unified";

/// Cached `app.log` verbosity: `0` = quiet (suppress selected normal-level chatter), `1` = normal, `2` = verbose (extra scan/KVR diagnostics).
static LOG_VERBOSITY_LEVEL: AtomicU8 = AtomicU8::new(1);

/// Set by `pdf_metadata_extract_abort`; checked between PDF page-count extraction chunks so the UI can stop CPU-heavy work when the PDF tab is hidden or the window is idle.
static PDF_META_EXTRACT_ABORT: AtomicBool = AtomicBool::new(false);

#[inline]
pub fn log_verbosity_level() -> u8 {
    LOG_VERBOSITY_LEVEL.load(Ordering::Relaxed)
}

fn refresh_log_verbosity_from_prefs() {
    let level = history::get_preference("logVerbosity")
        .and_then(|v| v.as_str().map(std::string::ToString::to_string))
        .unwrap_or_else(|| "normal".to_string());
    let n = match level.as_str() {
        "quiet" => 0u8,
        "verbose" => 2u8,
        _ => 1u8,
    };
    LOG_VERBOSITY_LEVEL.store(n, Ordering::Relaxed);
}

/// Fingerprint cache keys in SQLite are written with [`normalize_path_for_db`]; align in-memory
/// lookups and inserts so `contains_key` matches paths from the UI (`allAudioSamples`).
fn normalize_fingerprint_cache_map(
    cache: HashMap<String, similarity::AudioFingerprint>,
) -> HashMap<String, similarity::AudioFingerprint> {
    cache
        .into_iter()
        .map(|(k, mut v)| {
            let nk = normalize_path_for_db(&k);
            v.path = nk.clone();
            (nk, v)
        })
        .collect()
}

fn should_suppress_app_log_line(msg: &str) -> bool {
    if LOG_VERBOSITY_LEVEL.load(Ordering::Relaxed) != 0 {
        return false;
    }
    // Optional normal-level prefixes to hide in Quiet (high-volume `write_app_log` only).
    const PREFIXES: &[&str] = &[];
    if PREFIXES.is_empty() {
        return false;
    }
    let m = msg.trim_start();
    PREFIXES.iter().any(|p| m.starts_with(p))
}

fn incremental_directory_scan_enabled() -> bool {
    let prefs = history::load_preferences();
    prefs
        .get("incrementalDirectoryScan")
        .and_then(|v| v.as_str())
        .map(|s| s != "off")
        .unwrap_or(true)
}

fn load_incremental_dir_state_for_walk() -> Option<Arc<unified_walker::IncrementalDirState>> {
    if !incremental_directory_scan_enabled() {
        return None;
    }
    match db::global().unified_scan_incremental_snapshot_is_trusted() {
        Ok(false) => {
            crate::write_app_log(
                "SCAN INCREMENTAL — last unified scan did not finish successfully; full walk"
                    .into(),
            );
            None
        }
        Err(e) => {
            crate::write_app_log(format!(
                "SCAN INCREMENTAL — could not read unified scan outcome ({e}); full walk",
            ));
            None
        }
        Ok(true) => match db::global().load_directory_scan_snapshot(DIRECTORY_SCAN_INCREMENTAL_DOMAIN)
        {
            Ok(m) => {
                let n = m.len();
                crate::app_log_verbose(move || {
                    format!("SCAN VERBOSE — incremental snapshot loaded: {n} directory keys")
                });
                Some(Arc::new(unified_walker::IncrementalDirState::new(m)))
            }
            Err(e) => {
                crate::write_app_log(format!(
                    "SCAN INCREMENTAL — load directory snapshot failed ({e}); full walk",
                ));
                None
            }
        },
    }
}

fn persist_incremental_dir_state_after_walk(
    inc: Option<&Arc<unified_walker::IncrementalDirState>>,
    scan_id_for_audit: &str,
) {
    let Some(inc) = inc else {
        return;
    };
    let pending = inc.take_pending();
    if pending.is_empty() {
        return;
    }
    let _ = db::global().upsert_directory_scan_batch(
        DIRECTORY_SCAN_INCREMENTAL_DOMAIN,
        &pending,
        Some(scan_id_for_audit),
    );
}

// ── Export / Import types ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportPayload {
    pub version: String,
    pub exported_at: String,
    pub plugins: Vec<ExportPlugin>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportPlugin {
    pub name: String,
    #[serde(rename = "type")]
    pub plugin_type: String,
    pub version: String,
    pub manufacturer: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manufacturer_url: Option<String>,
    pub path: String,
    pub size: String,
    #[serde(rename = "sizeBytes", default)]
    pub size_bytes: u64,
    pub modified: String,
    #[serde(default)]
    pub architectures: Vec<String>,
}

// Shared state for cancellation
struct ScanState {
    scanning: AtomicBool,
    stop_scan: AtomicBool,
}

struct UpdateState {
    checking: AtomicBool,
    stop_updates: AtomicBool,
}

struct AudioScanState {
    scanning: AtomicBool,
    stop_scan: AtomicBool,
}

struct DawScanState {
    scanning: AtomicBool,
    stop_scan: AtomicBool,
}

struct PresetScanState {
    scanning: AtomicBool,
    stop_scan: AtomicBool,
}

struct MidiScanState {
    scanning: AtomicBool,
    stop_scan: AtomicBool,
}

struct PdfScanState {
    scanning: AtomicBool,
    stop_scan: AtomicBool,
}

/// Tracks active directory paths being walked by each scanner for live status display.
struct WalkerStatus {
    plugin_dirs: Arc<std::sync::Mutex<Vec<String>>>,
    audio_dirs: Arc<std::sync::Mutex<Vec<String>>>,
    daw_dirs: Arc<std::sync::Mutex<Vec<String>>>,
    preset_dirs: Arc<std::sync::Mutex<Vec<String>>>,
    midi_dirs: Arc<std::sync::Mutex<Vec<String>>>,
    pdf_dirs: Arc<std::sync::Mutex<Vec<String>>>,
    /// True while `scan_unified` is active. Frontend walker-status tiles
    /// collapse 4 → 1 display when this is true (the single walker fans its
    /// dir-push out to all 4 `*_dirs` lists; showing all 4 would be redundant).
    unified_scanning: AtomicBool,
}

// ── Plugin update types ──

#[derive(Debug, Clone, Serialize, Deserialize)]
struct UpdatedPlugin {
    #[serde(flatten)]
    plugin: PluginInfo,
    #[serde(rename = "currentVersion")]
    current_version: String,
    #[serde(rename = "latestVersion")]
    latest_version: String,
    #[serde(rename = "hasUpdate")]
    has_update: bool,
    #[serde(rename = "updateUrl")]
    update_url: Option<String>,
    #[serde(rename = "kvrUrl")]
    kvr_url: Option<String>,
    #[serde(rename = "hasPlatformDownload")]
    has_platform_download: bool,
    source: String,
}

// ── IPC: offload blocking work to Tokio's blocking pool (keeps async runtime + window responsive)

#[inline]
async fn blocking<T, F>(f: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
{
    tokio::task::spawn_blocking(f)
        .await
        .map_err(|e| format!("spawn_blocking: {e}"))
}

#[inline]
async fn blocking_res<T, F>(f: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, String> + Send + 'static,
{
    tokio::task::spawn_blocking(f)
        .await
        .map_err(|e| format!("spawn_blocking: {e}"))?
}

// ── Tauri commands ──

#[tauri::command]
fn get_version(app: AppHandle) -> String {
    app.package_info().version.to_string()
}

#[tauri::command]
fn get_walker_status(app: AppHandle) -> serde_json::Value {
    let ws = app.state::<WalkerStatus>();
    let plugin = ws
        .plugin_dirs
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone();
    let audio = ws
        .audio_dirs
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone();
    let daw = ws
        .daw_dirs
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone();
    let preset = ws
        .preset_dirs
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone();
    let midi = ws
        .midi_dirs
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone();
    let pdf = ws
        .pdf_dirs
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone();
    let pool_threads = num_cpus::get().max(4);
    let plugin_scanning = app.state::<ScanState>().scanning.load(Ordering::Relaxed);
    let audio_scanning = app
        .state::<AudioScanState>()
        .scanning
        .load(Ordering::Relaxed);
    let daw_scanning = app.state::<DawScanState>().scanning.load(Ordering::Relaxed);
    let preset_scanning = app
        .state::<PresetScanState>()
        .scanning
        .load(Ordering::Relaxed);
    let pdf_scanning = app.state::<PdfScanState>().scanning.load(Ordering::Relaxed);
    let midi_scanning = app
        .state::<MidiScanState>()
        .scanning
        .load(Ordering::Relaxed);
    let unified_scanning = ws.unified_scanning.load(Ordering::Relaxed);
    serde_json::json!({
        "plugin": plugin,
        "audio": audio,
        "daw": daw,
        "preset": preset,
        "midi": midi,
        "pdf": pdf,
        "poolThreads": pool_threads,
        "pluginScanning": plugin_scanning,
        "audioScanning": audio_scanning,
        "dawScanning": daw_scanning,
        "presetScanning": preset_scanning,
        "midiScanning": midi_scanning,
        "pdfScanning": pdf_scanning,
        "unifiedScanning": unified_scanning,
    })
}

#[tauri::command]
async fn scan_plugins(
    app: AppHandle,
    custom_roots: Option<Vec<String>>,
    exclude_paths: Option<Vec<String>>,
) -> Result<serde_json::Value, String> {
    let state = app.state::<ScanState>();

    if state.scanning.swap(true, Ordering::SeqCst) {
        return Err("Scan already in progress".into());
    }
    state.stop_scan.store(false, Ordering::SeqCst);
    let scan_start = Instant::now();
    append_log(format!(
        "SCAN START — plugins | roots: {:?}",
        custom_roots.as_deref().unwrap_or(&[])
    ));

    let app_handle = app.clone();
    let result = tokio::task::spawn_blocking(move || {
        let scan_state = app_handle.state::<ScanState>();
        let directories = if let Some(ref extra) = custom_roots {
            let custom: Vec<String> = extra
                .iter()
                .filter(|r| std::path::Path::new(r).exists())
                .cloned()
                .collect();
            if custom.is_empty() {
                scanner::get_vst_directories()
            } else {
                custom
            }
        } else {
            scanner::get_vst_directories()
        };
        let plugin_scan_id = history::gen_id();
        let now_iso = history::now_iso();
        let db = db::global();
        // Plugin discovery must not use the shared unified incremental map: `record_scanned_dir`
        // would mark each VST root as "already scanned", and `should_skip` would skip the entire
        // root on the next plugin run (or skip immediately if a unified walk already recorded it).
        let plugin_paths = scanner::discover_plugins(&directories, None);
        let total = plugin_paths.len();

        let _ = db.plugin_scan_parent_create(&plugin_scan_id, &now_iso, &directories);

        let _ = app_handle.emit(
            "scan-progress",
            serde_json::json!({
                "phase": "start",
                "total": total,
                "processed": 0
            }),
        );

        // Deduplicate and exclude already-scanned paths
        let exclude_set: HashSet<String> = exclude_paths.unwrap_or_default().into_iter().collect();
        let mut seen = HashSet::new();
        let unique_paths: Vec<_> = plugin_paths
            .into_iter()
            .filter(|p| {
                let s = p.to_string_lossy().to_string();
                !exclude_set.contains(&s) && seen.insert(s)
            })
            .collect();

        // Process plugins in parallel, streaming results to UI via channel
        use rayon::prelude::*;
        let prefs = history::load_preferences();
        let batch_size = prefs
            .get("batchSize")
            .and_then(|v| {
                v.as_str()
                    .and_then(|s| s.parse::<usize>().ok())
                    .or(v.as_u64().map(|n| n as usize))
            })
            .unwrap_or(100)
            .clamp(10, 200);
        let chan_buf = prefs
            .get("channelBuffer")
            .and_then(|v| {
                v.as_str()
                    .and_then(|s| s.parse::<usize>().ok())
                    .or(v.as_u64().map(|n| n as usize))
            })
            .unwrap_or(256)
            .clamp(64, 512);
        let (tx, rx) = std::sync::mpsc::sync_channel::<scanner::PluginInfo>(chan_buf);
        // Share stop flag directly with rayon workers for immediate cancellation
        let stop_flag = std::sync::Arc::new(AtomicBool::new(false));
        let stop_flag2 = stop_flag.clone();
        let plugin_dirs = Arc::clone(&app_handle.state::<WalkerStatus>().plugin_dirs);

        // Dedicated thread pool so plugin scanning doesn't starve other scanners
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(num_cpus::get().max(4))
            .build()
            .unwrap_or_else(|e| {
                let msg = format!("Thread pool creation failed ({e}), retrying with 2 threads");
                eprintln!("{msg}");
                append_log(msg);
                rayon::ThreadPoolBuilder::new()
                    .num_threads(2)
                    .build()
                    .expect("fallback 2-thread pool")
            });
        std::thread::spawn(move || {
            pool.install(|| {
                unique_paths.par_iter().for_each(|p| {
                    if stop_flag2.load(Ordering::Relaxed) {
                        return;
                    }
                    // Track plugin path
                    {
                        let mut ad = plugin_dirs.lock().unwrap_or_else(|e| e.into_inner());
                        ad.push(p.to_string_lossy().to_string());
                        if ad.len() > 30 {
                            let excess = ad.len() - 30;
                            ad.drain(..excess);
                        }
                    }
                    if let Some(info) = scanner::get_plugin_info(p) {
                        if stop_flag2.load(Ordering::Relaxed) {
                            return;
                        }
                        let _ = tx.send(info);
                    }
                });
            });
        });

        let mut all_plugins = Vec::new();
        let mut batch = Vec::new();
        let mut processed = 0usize;

        // Use try_recv with short timeout so stop signal is checked frequently
        loop {
            if scan_state.stop_scan.load(Ordering::Relaxed) {
                stop_flag.store(true, Ordering::Relaxed);
                // Drain channel to unblock workers
                while rx.try_recv().is_ok() {}
                break;
            }
            let info = match rx.recv_timeout(std::time::Duration::from_millis(10)) {
                Ok(info) => info,
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => continue,
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
            };
            batch.push(info);
            processed += 1;
            if batch.len() >= batch_size || processed == total {
                let _ = db.insert_plugin_batch(&plugin_scan_id, &batch);
                all_plugins.extend(batch.clone());
                let _ = app_handle.emit(
                    "scan-progress",
                    serde_json::json!({
                        "phase": "scanning",
                        "plugins": batch,
                        "processed": processed,
                        "total": total
                    }),
                );
                batch.clear();
            }
        }
        if !batch.is_empty() {
            let _ = db.insert_plugin_batch(&plugin_scan_id, &batch);
            all_plugins.extend(batch.clone());
            let _ = app_handle.emit(
                "scan-progress",
                serde_json::json!({
                    "phase": "scanning",
                    "plugins": batch,
                    "processed": processed,
                    "total": total
                }),
            );
        }

        let was_stopped = scan_state.stop_scan.load(Ordering::Relaxed);
        all_plugins.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        let roots: Vec<String> = directories.clone();
        let _ = db.plugin_scan_parent_finalize(
            &plugin_scan_id,
            all_plugins.len(),
            &directories,
            &roots,
        );
        let _ = db.set_plugin_scan_complete(&plugin_scan_id, !was_stopped);
        db.checkpoint();

        serde_json::json!({
            "plugins": all_plugins,
            "directories": directories,
            "snapshotId": plugin_scan_id,
            "stopped": was_stopped
        })
    })
    .await;

    state.scanning.store(false, Ordering::SeqCst);
    {
        let ws = app.state::<WalkerStatus>();
        let mut ad = ws.plugin_dirs.lock().unwrap_or_else(|e| e.into_inner());
        ad.clear();
    }
    let elapsed = scan_start.elapsed();
    match &result {
        Ok(v) => append_log(format!(
            "SCAN END — plugins | {}s | {} found",
            elapsed.as_secs(),
            v.get("plugins")
                .and_then(|p| p.as_array())
                .map(|a| a.len())
                .unwrap_or(0)
        )),
        Err(e) => append_log(format!(
            "SCAN ERROR — plugins | {}s | {}",
            elapsed.as_secs(),
            e
        )),
    }
    result.map_err(|e| e.to_string())
}

#[tauri::command]
async fn stop_scan(app: AppHandle) -> Result<(), String> {
    append_log("SCAN STOP — plugins (user requested)".into());
    let state = app.state::<ScanState>();
    state.stop_scan.store(true, Ordering::SeqCst);
    Ok(())
}

#[tauri::command]
async fn check_updates(
    app: AppHandle,
    plugins: Vec<PluginInfo>,
) -> Result<Vec<UpdatedPlugin>, String> {
    let state = app.state::<UpdateState>();
    if state.checking.swap(true, Ordering::SeqCst) {
        return Err("Update check already in progress".into());
    }
    state.stop_updates.store(false, Ordering::SeqCst);

    // Load KVR cache to skip already-checked plugins (resume from previous run)
    let kvr_cache = history::load_kvr_cache();

    let total = plugins.len();
    #[cfg(not(test))]
    append_log(format!("UPDATE CHECK — {} plugins", total));
    let _ = app.emit(
        "update-progress",
        serde_json::json!({
            "phase": "start",
            "total": total,
            "processed": 0
        }),
    );

    // Deduplicate by manufacturer+name
    let mut search_groups: std::collections::HashMap<String, (PluginInfo, Vec<PluginInfo>)> =
        std::collections::HashMap::new();
    for plugin in &plugins {
        let key = format!("{}|||{}", plugin.manufacturer, plugin.name).to_lowercase();
        search_groups
            .entry(key)
            .or_insert_with(|| (plugin.clone(), Vec::new()))
            .1
            .push(plugin.clone());
    }

    let groups: Vec<(PluginInfo, Vec<PluginInfo>)> = search_groups.into_values().collect();
    let mut results: std::collections::HashMap<String, UpdatedPlugin> =
        std::collections::HashMap::new();
    let mut processed = 0usize;

    for (representative, siblings) in &groups {
        if state.stop_updates.load(Ordering::SeqCst) {
            break;
        }

        let cache_key =
            format!("{}|||{}", representative.manufacturer, representative.name).to_lowercase();

        // Use cached result if available
        let update_result = if let Some(cached) = kvr_cache.get(&cache_key) {
            Some(kvr::UpdateResult {
                latest_version: cached
                    .latest_version
                    .clone()
                    .unwrap_or_else(|| representative.version.clone()),
                has_update: cached.has_update,
                update_url: cached.update_url.clone(),
                kvr_url: cached.kvr_url.clone(),
                has_platform_download: cached.update_url.is_some(),
                source: cached.source.clone(),
            })
        } else {
            kvr::find_latest_version(
                &representative.name,
                &representative.manufacturer,
                &representative.version,
            )
            .await
        };

        let mut batch_plugins = Vec::new();
        for sibling in siblings {
            let current_version = sibling.version.clone();
            let updated = if let Some(ref result) = update_result {
                let has_update = kvr::compare_versions(&result.latest_version, &current_version)
                    == std::cmp::Ordering::Greater
                    && current_version != "Unknown";
                UpdatedPlugin {
                    plugin: sibling.clone(),
                    current_version,
                    latest_version: result.latest_version.clone(),
                    has_update,
                    update_url: result.update_url.clone(),
                    kvr_url: result.kvr_url.clone(),
                    has_platform_download: result.has_platform_download,
                    source: result.source.clone(),
                }
            } else {
                UpdatedPlugin {
                    plugin: sibling.clone(),
                    current_version: current_version.clone(),
                    latest_version: current_version,
                    has_update: false,
                    update_url: None,
                    kvr_url: None,
                    has_platform_download: false,
                    source: "not-found".into(),
                }
            };

            results.insert(sibling.path.clone(), updated.clone());
            batch_plugins.push(updated);
            processed += 1;
        }

        let _ = app.emit(
            "update-progress",
            serde_json::json!({
                "phase": "checking",
                "plugins": batch_plugins,
                "processed": processed,
                "total": total
            }),
        );

        // Only rate-limit when we actually hit the network
        if !kvr_cache.contains_key(&cache_key) {
            crate::app_log_verbose(|| {
                format!(
                    "UPDATE VERBOSE — KVR network fetch | {} | {}",
                    representative.name, representative.manufacturer
                )
            });
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }
    }

    state.checking.store(false, Ordering::SeqCst);

    let final_plugins: Vec<UpdatedPlugin> = plugins
        .iter()
        .map(|p| {
            results.remove(&p.path).unwrap_or_else(|| UpdatedPlugin {
                plugin: p.clone(),
                current_version: p.version.clone(),
                latest_version: p.version.clone(),
                has_update: false,
                update_url: None,
                kvr_url: None,
                has_platform_download: false,
                source: "not-found".into(),
            })
        })
        .collect();

    Ok(final_plugins)
}

#[tauri::command]
async fn stop_updates(app: AppHandle) -> Result<(), String> {
    append_log("UPDATE STOP — user cancelled update check".into());
    let state = app.state::<UpdateState>();
    state.stop_updates.store(true, Ordering::SeqCst);
    Ok(())
}

#[tauri::command]
async fn resolve_kvr(direct_url: String, plugin_name: String) -> Result<kvr::KvrResult, String> {
    Ok(kvr::resolve_kvr(&direct_url, &plugin_name).await)
}

// History commands — all backed by SQLite via db::global()
#[tauri::command]
async fn history_get_scans() -> Result<Vec<serde_json::Value>, String> {
    blocking_res(|| db::global().get_plugin_scans()).await
}

#[tauri::command]
async fn history_get_detail(id: String) -> Result<history::ScanSnapshot, String> {
    blocking_res(move || db::global().get_plugin_scan_detail(&id)).await
}

#[tauri::command]
async fn history_delete(id: String) -> Result<(), String> {
    blocking_res(move || db::global().delete_plugin_scan(&id)).await
}

#[tauri::command]
async fn history_clear() -> Result<(), String> {
    #[cfg(not(test))]
    append_log("HISTORY CLEAR — plugins (all scan history deleted)".into());
    blocking_res(|| db::global().clear_plugin_history()).await
}

#[tauri::command]
async fn history_diff(old_id: String, new_id: String) -> Option<history::ScanDiff> {
    tokio::task::spawn_blocking(move || {
        let old = db::global().get_plugin_scan_detail(&old_id).ok()?;
        let new = db::global().get_plugin_scan_detail(&new_id).ok()?;
        Some(history::compute_plugin_diff(&old, &new))
    })
    .await
    .ok()
    .flatten()
}

#[tauri::command]
async fn history_latest() -> Result<Option<history::ScanSnapshot>, String> {
    blocking_res(|| db::global().get_latest_plugin_scan()).await
}

#[tauri::command]
async fn kvr_cache_get(
) -> Result<std::collections::HashMap<String, history::KvrCacheEntry>, String> {
    blocking_res(|| db::global().load_kvr_cache()).await
}

#[tauri::command]
async fn kvr_cache_update(entries: Vec<KvrCacheUpdateEntry>) -> Result<(), String> {
    blocking_res(move || db::global().update_kvr_cache(&entries)).await
}

// Audio scanner commands
#[tauri::command]
async fn scan_audio_samples(
    app: AppHandle,
    custom_roots: Option<Vec<String>>,
    exclude_paths: Option<Vec<String>>,
) -> Result<serde_json::Value, String> {
    let state = app.state::<AudioScanState>();
    let scan_start = Instant::now();
    append_log(format!(
        "SCAN START — audio | roots: {:?}",
        custom_roots.as_deref().unwrap_or(&[])
    ));
    if state.scanning.swap(true, Ordering::SeqCst) {
        return Err("Audio scan already in progress".into());
    }
    state.stop_scan.store(false, Ordering::SeqCst);

    let _ = app.emit(
        "audio-scan-progress",
        serde_json::json!({
            "phase": "status",
            "message": "Walking filesystem directories parallelized for audio files..."
        }),
    );

    let app_handle = app.clone();
    let result = tokio::task::spawn_blocking(move || {
        let audio_state = app_handle.state::<AudioScanState>();
        let roots = if let Some(ref extra) = custom_roots {
            let custom: Vec<std::path::PathBuf> = extra
                .iter()
                .map(std::path::PathBuf::from)
                .filter(|p| p.exists())
                .collect();
            if custom.is_empty() {
                audio_scanner::get_audio_roots()
            } else {
                custom
            }
        } else {
            audio_scanner::get_audio_roots()
        };
        let root_strs: Vec<String> = roots
            .iter()
            .map(|r| r.to_string_lossy().to_string())
            .collect();
        let now_iso = history::now_iso();
        let audio_scan_id = history::gen_id();
        let db = db::global();
        let _ = db.audio_scan_parent_create(&audio_scan_id, &now_iso, &root_strs);

        let mut audio_count: u64 = 0;
        let mut audio_bytes: u64 = 0;
        let mut audio_format_counts: HashMap<String, usize> = HashMap::new();
        let exclude_set = exclude_paths.map(|v| v.into_iter().collect::<HashSet<String>>());
        let incremental_state = load_incremental_dir_state_for_walk();

        audio_scanner::walk_for_audio(
            &roots,
            &mut |batch, _found| {
                for s in batch.iter() {
                    audio_bytes += s.size;
                    *audio_format_counts.entry(s.format.clone()).or_insert(0) += 1;
                }
                let inserted = db.insert_audio_batch(&audio_scan_id, batch).unwrap_or(0);
                audio_count += inserted;
                let _ = app_handle.emit(
                    "audio-scan-progress",
                    serde_json::json!({
                        "phase": "scanning",
                        "samples": batch,
                        "found": audio_count,
                    }),
                );
            },
            &|| audio_state.stop_scan.load(Ordering::SeqCst),
            exclude_set,
            Some(Arc::clone(&app_handle.state::<WalkerStatus>().audio_dirs)),
            incremental_state.clone(),
        );

        persist_incremental_dir_state_after_walk(incremental_state.as_ref(), &audio_scan_id);

        // Clear walker status
        {
            let ws = app_handle.state::<WalkerStatus>();
            let mut ad = ws.audio_dirs.lock().unwrap_or_else(|e| e.into_inner());
            ad.clear();
        }

        let was_stopped = audio_state.stop_scan.load(Ordering::Relaxed);
        let _ = db.audio_scan_parent_finalize(
            &audio_scan_id,
            audio_count,
            audio_bytes,
            &audio_format_counts,
        );
        let _ = db.set_audio_scan_complete(&audio_scan_id, !was_stopped);
        db.checkpoint();
        serde_json::json!({
            "samples": [],
            "roots": root_strs,
            "stopped": was_stopped,
            "streamed": true,
            "audioScanId": audio_scan_id,
            "audioCount": audio_count,
        })
    })
    .await;

    state.scanning.store(false, Ordering::SeqCst);
    let elapsed = scan_start.elapsed();
    match &result {
        Ok(v) => append_log(format!(
            "SCAN END — audio | {}s | {} found",
            elapsed.as_secs(),
            v.get("audioCount")
                .and_then(|n| n.as_u64())
                .or_else(|| {
                    v.get("samples")
                        .and_then(|p| p.as_array())
                        .map(|a| a.len() as u64)
                })
                .unwrap_or(0)
        )),
        Err(e) => append_log(format!(
            "SCAN ERROR — audio | {}s | {}",
            elapsed.as_secs(),
            e
        )),
    }
    result.map_err(|e| e.to_string())
}

#[tauri::command]
async fn stop_audio_scan(app: AppHandle) -> Result<(), String> {
    append_log("SCAN STOP — audio (user requested)".into());
    let state = app.state::<AudioScanState>();
    state.stop_scan.store(true, Ordering::SeqCst);
    Ok(())
}

#[tauri::command]
async fn get_audio_metadata(file_path: String) -> audio_scanner::AudioMetadata {
    let fallback_path = file_path.clone();
    tokio::task::spawn_blocking(move || audio_scanner::get_audio_metadata(&file_path))
        .await
        .unwrap_or_else(|_| audio_scanner::get_audio_metadata(&fallback_path))
}

// Audio history commands — SQLite backed
#[tauri::command]
async fn audio_history_save(
    samples: Vec<AudioSample>,
    roots: Option<Vec<String>>,
) -> Result<history::AudioScanSnapshot, String> {
    let roots = roots.unwrap_or_default();
    blocking_res(move || {
        let snap = history::build_audio_snapshot(&samples, &roots);
        db::global().save_audio_scan_full(&snap)?;
        db::global().checkpoint();
        Ok(snap)
    })
    .await
}

#[tauri::command]
async fn audio_history_get_scans() -> Result<Vec<serde_json::Value>, String> {
    blocking_res(|| db::global().get_audio_scans_list()).await
}

#[tauri::command]
async fn audio_history_get_detail(id: String) -> Result<history::AudioScanSnapshot, String> {
    blocking_res(move || db::global().get_audio_scan_detail(&id)).await
}

#[tauri::command]
async fn audio_history_delete(id: String) -> Result<(), String> {
    blocking_res(move || db::global().delete_audio_scan(&id)).await
}

#[tauri::command]
async fn audio_history_clear() -> Result<(), String> {
    #[cfg(not(test))]
    append_log("HISTORY CLEAR — audio samples (all scan history deleted)".into());
    blocking_res(|| db::global().clear_audio_history()).await
}

#[tauri::command]
async fn audio_history_latest() -> Result<Option<history::AudioScanSnapshot>, String> {
    blocking_res(|| db::global().get_latest_audio_scan()).await
}

#[tauri::command]
async fn audio_history_diff(old_id: String, new_id: String) -> Option<history::AudioScanDiff> {
    tokio::task::spawn_blocking(move || {
        let old = db::global().get_audio_scan_detail(&old_id).ok()?;
        let new = db::global().get_audio_scan_detail(&new_id).ok()?;
        Some(history::compute_audio_diff(&old, &new))
    })
    .await
    .ok()
    .flatten()
}

// DAW scanner commands
#[tauri::command]
async fn scan_daw_projects(
    app: AppHandle,
    custom_roots: Option<Vec<String>>,
    exclude_paths: Option<Vec<String>>,
) -> Result<serde_json::Value, String> {
    let state = app.state::<DawScanState>();
    let scan_start = Instant::now();
    append_log(format!(
        "SCAN START — daw | roots: {:?}",
        custom_roots.as_deref().unwrap_or(&[])
    ));
    if state.scanning.swap(true, Ordering::SeqCst) {
        return Err("DAW scan already in progress".into());
    }
    state.stop_scan.store(false, Ordering::SeqCst);

    let _ = app.emit(
        "daw-scan-progress",
        serde_json::json!({
            "phase": "status",
            "message": "Walking filesystem directories parallelized for DAW project files..."
        }),
    );

    let app_handle = app.clone();
    let result = tokio::task::spawn_blocking(move || {
        let daw_state = app_handle.state::<DawScanState>();
        let roots = if let Some(ref extra) = custom_roots {
            let custom: Vec<std::path::PathBuf> = extra
                .iter()
                .map(std::path::PathBuf::from)
                .filter(|p| p.exists())
                .collect();
            if custom.is_empty() {
                daw_scanner::get_daw_roots()
            } else {
                custom
            }
        } else {
            daw_scanner::get_daw_roots()
        };
        let root_strs: Vec<String> = roots
            .iter()
            .map(|r| r.to_string_lossy().to_string())
            .collect();
        let now_iso = history::now_iso();
        let daw_scan_id = history::gen_id();
        let db = db::global();
        let _ = db.daw_scan_parent_create(&daw_scan_id, &now_iso, &root_strs);

        let mut daw_count: u64 = 0;
        let mut daw_bytes: u64 = 0;
        let mut daw_daw_counts: HashMap<String, usize> = HashMap::new();
        let exclude_set = exclude_paths.map(|v| v.into_iter().collect::<HashSet<String>>());
        let incremental_state = load_incremental_dir_state_for_walk();

        daw_scanner::walk_for_daw(
            &roots,
            &mut |batch, _found| {
                let inserted_idx = db.insert_daw_batch(&daw_scan_id, batch).unwrap_or_default();
                let deduped: Vec<&DawProject> = inserted_idx.iter().map(|&i| &batch[i]).collect();
                for p in &deduped {
                    daw_bytes += p.size;
                    *daw_daw_counts.entry(p.daw.clone()).or_insert(0) += 1;
                }
                daw_count += deduped.len() as u64;
                let _ = app_handle.emit(
                    "daw-scan-progress",
                    serde_json::json!({
                        "phase": "scanning",
                        "projects": deduped,
                        "found": daw_count,
                    }),
                );
            },
            &|| daw_state.stop_scan.load(Ordering::SeqCst),
            exclude_set,
            {
                let prefs = history::load_preferences();
                prefs
                    .get("includeAbletonBackups")
                    .and_then(|v| v.as_str())
                    .map(|s| s == "on")
                    .unwrap_or(false)
            },
            Some(Arc::clone(&app_handle.state::<WalkerStatus>().daw_dirs)),
            incremental_state.clone(),
        );

        persist_incremental_dir_state_after_walk(incremental_state.as_ref(), &daw_scan_id);

        {
            let ws = app_handle.state::<WalkerStatus>();
            let mut ad = ws.daw_dirs.lock().unwrap_or_else(|e| e.into_inner());
            ad.clear();
        }
        let was_stopped = daw_state.stop_scan.load(Ordering::Relaxed);
        let _ = db.daw_scan_parent_finalize(
            &daw_scan_id,
            daw_count as usize,
            daw_bytes,
            &daw_daw_counts,
        );
        let _ = db.set_daw_scan_complete(&daw_scan_id, !was_stopped);
        db.checkpoint();
        serde_json::json!({
            "projects": [],
            "roots": root_strs,
            "stopped": was_stopped,
            "streamed": true,
            "dawScanId": daw_scan_id,
            "dawCount": daw_count,
        })
    })
    .await;

    state.scanning.store(false, Ordering::SeqCst);
    let elapsed = scan_start.elapsed();
    match &result {
        Ok(v) => append_log(format!(
            "SCAN END — daw | {}s | {} found",
            elapsed.as_secs(),
            v.get("dawCount")
                .and_then(|n| n.as_u64())
                .or_else(|| {
                    v.get("projects")
                        .and_then(|p| p.as_array())
                        .map(|a| a.len() as u64)
                })
                .unwrap_or(0)
        )),
        Err(e) => append_log(format!("SCAN ERROR — daw | {}s | {}", elapsed.as_secs(), e)),
    }
    result.map_err(|e| e.to_string())
}

#[tauri::command]
async fn stop_daw_scan(app: AppHandle) -> Result<(), String> {
    append_log("SCAN STOP — daw (user requested)".into());
    let state = app.state::<DawScanState>();
    state.stop_scan.store(true, Ordering::SeqCst);
    Ok(())
}

// DAW history commands — SQLite backed
#[tauri::command]
async fn daw_history_save(
    projects: Vec<DawProject>,
    roots: Option<Vec<String>>,
) -> Result<history::DawScanSnapshot, String> {
    let roots = roots.unwrap_or_default();
    blocking_res(move || {
        let snap = history::build_daw_snapshot(&projects, &roots);
        db::global().save_daw_scan(&snap)?;
        db::global().checkpoint();
        Ok(snap)
    })
    .await
}

#[tauri::command]
async fn daw_history_get_scans() -> Result<Vec<serde_json::Value>, String> {
    blocking_res(|| db::global().get_daw_scans()).await
}

#[tauri::command]
async fn daw_history_get_detail(id: String) -> Result<history::DawScanSnapshot, String> {
    blocking_res(move || db::global().get_daw_scan_detail(&id)).await
}

#[tauri::command]
async fn daw_history_delete(id: String) -> Result<(), String> {
    blocking_res(move || db::global().delete_daw_scan(&id)).await
}

#[tauri::command]
async fn daw_history_clear() -> Result<(), String> {
    #[cfg(not(test))]
    append_log("HISTORY CLEAR — DAW projects".into());
    blocking_res(|| db::global().clear_daw_history()).await
}

#[tauri::command]
async fn daw_history_latest() -> Result<Option<history::DawScanSnapshot>, String> {
    blocking_res(|| db::global().get_latest_daw_scan()).await
}

#[tauri::command]
async fn daw_history_diff(old_id: String, new_id: String) -> Option<history::DawScanDiff> {
    tokio::task::spawn_blocking(move || {
        let old = db::global().get_daw_scan_detail(&old_id).ok()?;
        let new = db::global().get_daw_scan_detail(&new_id).ok()?;
        Some(history::compute_daw_diff(&old, &new))
    })
    .await
    .ok()
    .flatten()
}

// Preset scanner commands
#[tauri::command]
async fn scan_presets(
    app: AppHandle,
    custom_roots: Option<Vec<String>>,
    exclude_paths: Option<Vec<String>>,
) -> Result<serde_json::Value, String> {
    let state = app.state::<PresetScanState>();
    let scan_start = Instant::now();
    append_log(format!(
        "SCAN START — presets | roots: {:?}",
        custom_roots.as_deref().unwrap_or(&[])
    ));
    if state.scanning.swap(true, Ordering::SeqCst) {
        return Err("Preset scan already in progress".into());
    }
    state.stop_scan.store(false, Ordering::SeqCst);

    let _ = app.emit(
        "preset-scan-progress",
        serde_json::json!({
            "phase": "status",
            "message": "Walking filesystem directories parallelized for preset files..."
        }),
    );

    let app_handle = app.clone();
    let result = tokio::task::spawn_blocking(move || {
        let preset_state = app_handle.state::<PresetScanState>();
        let roots = if let Some(ref extra) = custom_roots {
            let custom: Vec<std::path::PathBuf> = extra
                .iter()
                .map(std::path::PathBuf::from)
                .filter(|p| p.exists())
                .collect();
            if custom.is_empty() {
                preset_scanner::get_preset_roots()
            } else {
                custom
            }
        } else {
            preset_scanner::get_preset_roots()
        };
        let root_strs: Vec<String> = roots
            .iter()
            .map(|r| r.to_string_lossy().to_string())
            .collect();
        let now_iso = history::now_iso();
        let preset_scan_id = history::gen_id();
        let db = db::global();
        let _ = db.preset_scan_parent_create(&preset_scan_id, &now_iso, &root_strs);

        let mut preset_count: u64 = 0;
        let mut preset_bytes: u64 = 0;
        let mut preset_format_counts: HashMap<String, usize> = HashMap::new();
        let exclude_set = exclude_paths.map(|v| v.into_iter().collect::<HashSet<String>>());
        let incremental_state = load_incremental_dir_state_for_walk();

        preset_scanner::walk_for_presets(
            &roots,
            &mut |batch, _found| {
                for p in batch.iter() {
                    preset_bytes += p.size;
                    *preset_format_counts.entry(p.format.clone()).or_insert(0) += 1;
                }
                let inserted = db.insert_preset_batch(&preset_scan_id, batch).unwrap_or(0);
                preset_count += inserted;
                let _ = app_handle.emit(
                    "preset-scan-progress",
                    serde_json::json!({
                        "phase": "scanning",
                        "presets": batch,
                        "found": preset_count,
                    }),
                );
            },
            &|| preset_state.stop_scan.load(Ordering::SeqCst),
            exclude_set,
            Some(Arc::clone(&app_handle.state::<WalkerStatus>().preset_dirs)),
            incremental_state.clone(),
        );

        persist_incremental_dir_state_after_walk(incremental_state.as_ref(), &preset_scan_id);

        {
            let ws = app_handle.state::<WalkerStatus>();
            let mut ad = ws.preset_dirs.lock().unwrap_or_else(|e| e.into_inner());
            ad.clear();
        }
        let was_stopped = preset_state.stop_scan.load(Ordering::Relaxed);
        let _ = db.preset_scan_parent_finalize(
            &preset_scan_id,
            preset_count as usize,
            preset_bytes,
            &preset_format_counts,
        );
        let _ = db.set_preset_scan_complete(&preset_scan_id, !was_stopped);
        db.checkpoint();
        serde_json::json!({
            "presets": [],
            "roots": root_strs,
            "stopped": was_stopped,
            "streamed": true,
            "presetScanId": preset_scan_id,
            "presetCount": preset_count,
        })
    })
    .await;

    state.scanning.store(false, Ordering::SeqCst);
    let elapsed = scan_start.elapsed();
    match &result {
        Ok(v) => append_log(format!(
            "SCAN END — presets | {}s | {} found",
            elapsed.as_secs(),
            v.get("presetCount")
                .and_then(|n| n.as_u64())
                .or_else(|| {
                    v.get("presets")
                        .and_then(|p| p.as_array())
                        .map(|a| a.len() as u64)
                })
                .unwrap_or(0)
        )),
        Err(e) => append_log(format!(
            "SCAN ERROR — presets | {}s | {}",
            elapsed.as_secs(),
            e
        )),
    }
    result.map_err(|e| e.to_string())
}

#[tauri::command]
async fn stop_preset_scan(app: AppHandle) -> Result<(), String> {
    append_log("SCAN STOP — presets (user requested)".into());
    let state = app.state::<PresetScanState>();
    state.stop_scan.store(true, Ordering::SeqCst);
    Ok(())
}

// Preset history commands — SQLite backed
#[tauri::command]
async fn preset_history_save(
    presets: Vec<PresetFile>,
    roots: Option<Vec<String>>,
) -> Result<history::PresetScanSnapshot, String> {
    let roots = roots.unwrap_or_default();
    blocking_res(move || {
        let snap = history::build_preset_snapshot(&presets, &roots);
        db::global().save_preset_scan(&snap)?;
        db::global().checkpoint();
        Ok(snap)
    })
    .await
}

#[tauri::command]
async fn preset_history_get_scans() -> Result<Vec<serde_json::Value>, String> {
    blocking_res(|| db::global().get_preset_scans()).await
}

#[tauri::command]
async fn preset_history_get_detail(id: String) -> Result<history::PresetScanSnapshot, String> {
    blocking_res(move || db::global().get_preset_scan_detail(&id)).await
}

#[tauri::command]
async fn preset_history_delete(id: String) -> Result<(), String> {
    blocking_res(move || db::global().delete_preset_scan(&id)).await
}

#[tauri::command]
async fn preset_history_clear() -> Result<(), String> {
    #[cfg(not(test))]
    append_log("HISTORY CLEAR — presets".into());
    blocking_res(|| db::global().clear_preset_history()).await
}

#[tauri::command]
async fn preset_history_latest() -> Result<Option<history::PresetScanSnapshot>, String> {
    blocking_res(|| db::global().get_latest_preset_scan()).await
}

#[tauri::command]
async fn preset_history_diff(old_id: String, new_id: String) -> Option<history::PresetScanDiff> {
    tokio::task::spawn_blocking(move || {
        let old = db::global().get_preset_scan_detail(&old_id).ok()?;
        let new = db::global().get_preset_scan_detail(&new_id).ok()?;
        Some(history::compute_preset_diff(&old, &new))
    })
    .await
    .ok()
    .flatten()
}

// MIDI scanner commands — dedicated MIDI walker, fully independent of preset scan.
#[tauri::command]
async fn scan_midi_files(
    app: AppHandle,
    custom_roots: Option<Vec<String>>,
    exclude_paths: Option<Vec<String>>,
) -> Result<serde_json::Value, String> {
    let state = app.state::<MidiScanState>();
    let scan_start = Instant::now();
    append_log(format!(
        "SCAN START — midi | roots: {:?}",
        custom_roots.as_deref().unwrap_or(&[])
    ));
    if state.scanning.swap(true, Ordering::SeqCst) {
        return Err("MIDI scan already in progress".into());
    }
    state.stop_scan.store(false, Ordering::SeqCst);

    let _ = app.emit(
        "midi-scan-progress",
        serde_json::json!({
            "phase": "status",
            "message": "Walking filesystem directories parallelized for MIDI files..."
        }),
    );

    let app_handle = app.clone();
    let result = tokio::task::spawn_blocking(move || {
        let midi_state = app_handle.state::<MidiScanState>();
        let roots = if let Some(ref extra) = custom_roots {
            let custom: Vec<std::path::PathBuf> = extra
                .iter()
                .map(std::path::PathBuf::from)
                .filter(|p| p.exists())
                .collect();
            if custom.is_empty() {
                midi_scanner::get_midi_roots()
            } else {
                custom
            }
        } else {
            midi_scanner::get_midi_roots()
        };
        let exclude_set = exclude_paths.map(|v| v.into_iter().collect::<HashSet<String>>());
        let root_strs: Vec<String> = roots
            .iter()
            .map(|r| r.to_string_lossy().to_string())
            .collect();

        // Streaming save: create parent row upfront, insert each batch directly
        // to the DB, finalize totals at end. Keeps memory bounded at 6M+ scale.
        let now_iso = history::now_iso();
        let midi_scan_id = history::gen_id();
        let db = db::global();
        let _ = db.midi_scan_parent_create(&midi_scan_id, &now_iso, &root_strs);

        let mut midi_count: u64 = 0;
        let mut midi_bytes: u64 = 0;
        let mut midi_format_counts: HashMap<String, usize> = HashMap::new();
        let incremental_state = load_incremental_dir_state_for_walk();

        midi_scanner::walk_for_midi(
            &roots,
            &mut |batch, found| {
                for m in batch {
                    midi_bytes += m.size;
                    *midi_format_counts.entry(m.format.clone()).or_insert(0) += 1;
                }
                midi_count += batch.len() as u64;
                let _ = db.insert_midi_batch(&midi_scan_id, batch);
                let _ = app_handle.emit(
                    "midi-scan-progress",
                    serde_json::json!({
                        "phase": "scanning",
                        "midiFiles": batch,
                        "found": found
                    }),
                );
            },
            &|| midi_state.stop_scan.load(Ordering::SeqCst),
            exclude_set,
            Some(Arc::clone(&app_handle.state::<WalkerStatus>().midi_dirs)),
            incremental_state.clone(),
        );

        persist_incremental_dir_state_after_walk(incremental_state.as_ref(), &midi_scan_id);

        {
            let ws = app_handle.state::<WalkerStatus>();
            let mut ad = ws.midi_dirs.lock().unwrap_or_else(|e| e.into_inner());
            ad.clear();
        }
        let was_stopped = midi_state.stop_scan.load(Ordering::Relaxed);
        let _ = db.midi_scan_parent_finalize(
            &midi_scan_id,
            midi_count as usize,
            midi_bytes,
            &midi_format_counts,
        );
        let _ = db.set_midi_scan_complete(&midi_scan_id, !was_stopped);
        db.checkpoint();
        serde_json::json!({
            "midiCount": midi_count,
            "roots": root_strs,
            "stopped": was_stopped,
            "midiScanId": midi_scan_id,
            "streamed": true
        })
    })
    .await;

    state.scanning.store(false, Ordering::SeqCst);
    let elapsed = scan_start.elapsed();
    match &result {
        Ok(v) => append_log(format!(
            "SCAN END — midi | {}s | {} found",
            elapsed.as_secs(),
            v.get("midiCount").and_then(|x| x.as_u64()).unwrap_or(0)
        )),
        Err(e) => append_log(format!(
            "SCAN ERROR — midi | {}s | {}",
            elapsed.as_secs(),
            e
        )),
    }
    result.map_err(|e| e.to_string())
}

#[tauri::command]
async fn stop_midi_scan(app: AppHandle) -> Result<(), String> {
    append_log("SCAN STOP — midi (user requested)".into());
    let state = app.state::<MidiScanState>();
    state.stop_scan.store(true, Ordering::SeqCst);
    Ok(())
}

#[tauri::command]
async fn midi_history_save(
    midi_files: Vec<history::MidiFile>,
    roots: Option<Vec<String>>,
) -> Result<history::MidiScanSnapshot, String> {
    let roots = roots.unwrap_or_default();
    blocking_res(move || {
        let snap = history::build_midi_snapshot(&midi_files, &roots);
        db::global().save_midi_scan(&snap)?;
        db::global().checkpoint();
        Ok(snap)
    })
    .await
}

#[tauri::command]
async fn midi_history_get_scans() -> Result<Vec<serde_json::Value>, String> {
    blocking_res(|| db::global().get_midi_scans()).await
}

#[tauri::command]
async fn midi_history_get_detail(id: String) -> Result<history::MidiScanSnapshot, String> {
    blocking_res(move || db::global().get_midi_scan_detail(&id)).await
}

#[tauri::command]
async fn midi_history_delete(id: String) -> Result<(), String> {
    blocking_res(move || db::global().delete_midi_scan(&id)).await
}

#[tauri::command]
async fn midi_history_clear() -> Result<(), String> {
    #[cfg(not(test))]
    append_log("HISTORY CLEAR — midi".into());
    blocking_res(|| db::global().clear_midi_history()).await
}

#[tauri::command]
async fn midi_history_latest() -> Result<Option<history::MidiScanSnapshot>, String> {
    blocking_res(|| db::global().get_latest_midi_scan()).await
}

#[tauri::command]
async fn midi_history_diff(old_id: String, new_id: String) -> Option<history::MidiScanDiff> {
    tokio::task::spawn_blocking(move || {
        let old = db::global().get_midi_scan_detail(&old_id).ok()?;
        let new = db::global().get_midi_scan_detail(&new_id).ok()?;
        Some(history::compute_midi_diff(&old, &new))
    })
    .await
    .ok()
    .flatten()
}

#[tauri::command(rename_all = "snake_case")]
async fn db_query_midi(
    search: Option<String>,
    format_filter: Option<String>,
    sort_key: Option<String>,
    sort_asc: Option<bool>,
    search_regex: Option<bool>,
    offset: Option<u64>,
    limit: Option<u64>,
) -> Result<db::MidiQueryResult, String> {
    let search_regex = search_regex.unwrap_or(false);
    tokio::task::spawn_blocking(move || {
        db::global().query_midi(
            search.as_deref(),
            format_filter.as_deref(),
            sort_key.as_deref().unwrap_or("name"),
            sort_asc.unwrap_or(true),
            search_regex,
            offset.unwrap_or(0),
            limit.unwrap_or(500),
        )
    })
    .await
    .map_err(|e| format!("db_query_midi task: {e}"))?
}

#[tauri::command(rename_all = "snake_case")]
async fn db_midi_filter_stats(
    search: Option<String>,
    format_filter: Option<String>,
    search_regex: Option<bool>,
) -> Result<db::FilterStatsResult, String> {
    let search_regex = search_regex.unwrap_or(false);
    tokio::task::spawn_blocking(move || {
        db::global().midi_filter_stats(
            search.as_deref(),
            format_filter.as_deref(),
            search_regex,
        )
    })
    .await
    .map_err(|e| format!("db_midi_filter_stats task: {e}"))?
}

// PDF scanner commands
#[tauri::command]
async fn scan_pdfs(
    app: AppHandle,
    custom_roots: Option<Vec<String>>,
    exclude_paths: Option<Vec<String>>,
) -> Result<serde_json::Value, String> {
    let state = app.state::<PdfScanState>();
    let scan_start = Instant::now();
    append_log(format!(
        "SCAN START — pdfs | roots: {:?}",
        custom_roots.as_deref().unwrap_or(&[])
    ));
    if state.scanning.swap(true, Ordering::SeqCst) {
        return Err("PDF scan already in progress".into());
    }
    state.stop_scan.store(false, Ordering::SeqCst);

    let _ = app.emit(
        "pdf-scan-progress",
        serde_json::json!({
            "phase": "status",
            "message": "Walking filesystem directories parallelized for PDF files..."
        }),
    );

    let app_handle = app.clone();
    let result = tokio::task::spawn_blocking(move || {
        let pdf_state = app_handle.state::<PdfScanState>();
        let roots = if let Some(ref extra) = custom_roots {
            let custom: Vec<std::path::PathBuf> = extra
                .iter()
                .map(std::path::PathBuf::from)
                .filter(|p| p.exists())
                .collect();
            if custom.is_empty() {
                pdf_scanner::get_pdf_roots()
            } else {
                custom
            }
        } else {
            pdf_scanner::get_pdf_roots()
        };
        let root_strs: Vec<String> = roots
            .iter()
            .map(|r| r.to_string_lossy().to_string())
            .collect();
        let now_iso = history::now_iso();
        let pdf_scan_id = history::gen_id();
        let db = db::global();
        let _ = db.pdf_scan_parent_create(&pdf_scan_id, &now_iso, &root_strs);

        let mut pdf_count: u64 = 0;
        let mut pdf_bytes: u64 = 0;
        let exclude_set = exclude_paths.map(|v| v.into_iter().collect::<HashSet<String>>());
        let incremental_state = load_incremental_dir_state_for_walk();

        pdf_scanner::walk_for_pdfs(
            &roots,
            &mut |batch, _found| {
                for p in batch.iter() {
                    pdf_bytes += p.size;
                }
                let inserted = db.insert_pdf_batch(&pdf_scan_id, batch).unwrap_or(0);
                pdf_count += inserted;
                let _ = app_handle.emit(
                    "pdf-scan-progress",
                    serde_json::json!({
                        "phase": "scanning",
                        "pdfs": batch,
                        "found": pdf_count,
                    }),
                );
            },
            &|| pdf_state.stop_scan.load(Ordering::SeqCst),
            exclude_set,
            Some(Arc::clone(&app_handle.state::<WalkerStatus>().pdf_dirs)),
            incremental_state.clone(),
        );

        persist_incremental_dir_state_after_walk(incremental_state.as_ref(), &pdf_scan_id);

        {
            let ws = app_handle.state::<WalkerStatus>();
            let mut ad = ws.pdf_dirs.lock().unwrap_or_else(|e| e.into_inner());
            ad.clear();
        }
        let was_stopped = pdf_state.stop_scan.load(Ordering::Relaxed);
        let _ = db.pdf_scan_parent_finalize(&pdf_scan_id, pdf_count as usize, pdf_bytes);
        let _ = db.set_pdf_scan_complete(&pdf_scan_id, !was_stopped);
        db.checkpoint();
        serde_json::json!({
            "pdfs": [],
            "roots": root_strs,
            "stopped": was_stopped,
            "streamed": true,
            "pdfScanId": pdf_scan_id,
            "pdfCount": pdf_count,
        })
    })
    .await;

    state.scanning.store(false, Ordering::SeqCst);
    let elapsed = scan_start.elapsed();
    match &result {
        Ok(v) => append_log(format!(
            "SCAN END — pdfs | {}s | {} found",
            elapsed.as_secs(),
            v.get("pdfCount")
                .and_then(|n| n.as_u64())
                .or_else(|| {
                    v.get("pdfs")
                        .and_then(|p| p.as_array())
                        .map(|a| a.len() as u64)
                })
                .unwrap_or(0)
        )),
        Err(e) => append_log(format!(
            "SCAN ERROR — pdfs | {}s | {}",
            elapsed.as_secs(),
            e
        )),
    }
    result.map_err(|e| e.to_string())
}

#[tauri::command]
async fn stop_pdf_scan(app: AppHandle) -> Result<(), String> {
    append_log("SCAN STOP — pdfs (user requested)".into());
    let state = app.state::<PdfScanState>();
    state.stop_scan.store(true, Ordering::SeqCst);
    Ok(())
}

// ── Unified home-tree scan ──
// Walks the union of audio/daw/preset/pdf roots ONCE and classifies files in
// place, emitting the same per-type events (`audio-scan-progress`,
// `daw-scan-progress`, `preset-scan-progress`, `pdf-scan-progress`) so
// frontend listeners work unchanged. Saves 4x filesystem traversals on
// overlapping roots (especially valuable on SMB shares where every readdir
// is a network roundtrip).
#[tauri::command]
async fn scan_unified(
    app: AppHandle,
    audio_custom_roots: Option<Vec<String>>,
    audio_exclude_paths: Option<Vec<String>>,
    daw_custom_roots: Option<Vec<String>>,
    daw_exclude_paths: Option<Vec<String>>,
    daw_include_backups: Option<bool>,
    preset_custom_roots: Option<Vec<String>>,
    preset_exclude_paths: Option<Vec<String>>,
    pdf_custom_roots: Option<Vec<String>>,
    pdf_exclude_paths: Option<Vec<String>>,
) -> Result<serde_json::Value, String> {
    let scan_start = Instant::now();
    append_log("SCAN START — unified (audio+daw+preset+pdf)".into());

    // Acquire all 4 scanning flags atomically; rollback if any is taken.
    let audio_state = app.state::<AudioScanState>();
    let daw_state = app.state::<DawScanState>();
    let preset_state = app.state::<PresetScanState>();
    let pdf_state = app.state::<PdfScanState>();

    if audio_state.scanning.swap(true, Ordering::SeqCst) {
        return Err("Audio scan already in progress".into());
    }
    if daw_state.scanning.swap(true, Ordering::SeqCst) {
        audio_state.scanning.store(false, Ordering::SeqCst);
        return Err("DAW scan already in progress".into());
    }
    if preset_state.scanning.swap(true, Ordering::SeqCst) {
        audio_state.scanning.store(false, Ordering::SeqCst);
        daw_state.scanning.store(false, Ordering::SeqCst);
        return Err("Preset scan already in progress".into());
    }
    if pdf_state.scanning.swap(true, Ordering::SeqCst) {
        audio_state.scanning.store(false, Ordering::SeqCst);
        daw_state.scanning.store(false, Ordering::SeqCst);
        preset_state.scanning.store(false, Ordering::SeqCst);
        return Err("PDF scan already in progress".into());
    }
    // Do NOT clear stop flags here — `prepare_unified_scan` clears stale flags
    // when Scan All begins; if the user hit Stop during the frontend delay, flags
    // stay true and we honour that below.
    if audio_state.stop_scan.load(Ordering::SeqCst)
        || daw_state.stop_scan.load(Ordering::SeqCst)
        || preset_state.stop_scan.load(Ordering::SeqCst)
        || pdf_state.stop_scan.load(Ordering::SeqCst)
    {
        audio_state.scanning.store(false, Ordering::SeqCst);
        daw_state.scanning.store(false, Ordering::SeqCst);
        preset_state.scanning.store(false, Ordering::SeqCst);
        pdf_state.scanning.store(false, Ordering::SeqCst);
        app.state::<WalkerStatus>()
            .unified_scanning
            .store(false, Ordering::SeqCst);
        append_log("SCAN CANCELLED — unified (stop before walk)".into());
        return Ok(serde_json::json!({
            "audioCount": 0u64,
            "dawCount": 0u64,
            "presetCount": 0u64,
            "pdfCount": 0u64,
            "audioRoots": serde_json::json!([]),
            "dawRoots": serde_json::json!([]),
            "presetRoots": serde_json::json!([]),
            "pdfRoots": serde_json::json!([]),
            "audioScanId": "",
            "dawScanId": "",
            "presetScanId": "",
            "pdfScanId": "",
            "unifiedRunId": "",
            "stopped": true,
            "streamed": true,
        }));
    }
    // Signal walker-status tiles to collapse 4 → 1 while we hold the walker.
    app.state::<WalkerStatus>()
        .unified_scanning
        .store(true, Ordering::SeqCst);

    // Kick off four status messages on the same event streams so the tabs
    // show "scanning" immediately.
    for ev in [
        "audio-scan-progress",
        "daw-scan-progress",
        "preset-scan-progress",
        "pdf-scan-progress",
    ] {
        let _ = app.emit(
            ev,
            serde_json::json!({
                "phase": "status",
                "message": "Walking filesystem (unified) — single traversal classifying all types..."
            }),
        );
    }

    let app_handle = app.clone();
    let result = tokio::task::spawn_blocking(move || -> Result<serde_json::Value, String> {
        let resolve = |custom: Option<Vec<String>>,
                       default: &dyn Fn() -> Vec<std::path::PathBuf>|
         -> Vec<std::path::PathBuf> {
            if let Some(extra) = custom {
                let v: Vec<std::path::PathBuf> = extra
                    .into_iter()
                    .map(std::path::PathBuf::from)
                    .filter(|p| p.exists())
                    .collect();
                if v.is_empty() {
                    default()
                } else {
                    v
                }
            } else {
                default()
            }
        };
        let audio_roots = resolve(audio_custom_roots, &audio_scanner::get_audio_roots);
        let daw_roots = resolve(daw_custom_roots, &daw_scanner::get_daw_roots);
        let preset_roots = resolve(preset_custom_roots, &preset_scanner::get_preset_roots);
        let pdf_roots = resolve(pdf_custom_roots, &pdf_scanner::get_pdf_roots);

        let spec = unified_walker::UnifiedSpec {
            audio_roots: audio_roots.clone(),
            audio_exclude: audio_exclude_paths.into_iter().flatten().collect(),
            daw_roots: daw_roots.clone(),
            daw_exclude: daw_exclude_paths.into_iter().flatten().collect(),
            daw_include_backups: daw_include_backups.unwrap_or(false),
            preset_roots: preset_roots.clone(),
            preset_exclude: preset_exclude_paths.into_iter().flatten().collect(),
            pdf_roots: pdf_roots.clone(),
            pdf_exclude: pdf_exclude_paths.into_iter().flatten().collect(),
        };

        // Streaming architecture: create 4 parent scan rows upfront, batch-insert
        // rows into the DB during the walker callback, and finalize totals at end.
        // This keeps memory O(batch_size) regardless of total file count.
        let now_iso = history::now_iso();
        let audio_scan_id = history::gen_id();
        let daw_scan_id = history::gen_id();
        let preset_scan_id = history::gen_id();
        let pdf_scan_id = history::gen_id();

        let to_strs = |v: &[std::path::PathBuf]| -> Vec<String> {
            v.iter().map(|r| r.to_string_lossy().to_string()).collect()
        };
        let audio_roots_strs = to_strs(&audio_roots);
        let daw_roots_strs = to_strs(&daw_roots);
        let preset_roots_strs = to_strs(&preset_roots);
        let pdf_roots_strs = to_strs(&pdf_roots);

        let unified_run_id = history::gen_id();
        let roots_json = serde_json::json!({
            "audio": &audio_roots_strs,
            "daw": &daw_roots_strs,
            "preset": &preset_roots_strs,
            "pdf": &pdf_roots_strs,
        })
        .to_string();

        let incremental_state = load_incremental_dir_state_for_walk();
        let db = db::global();
        let _ = db.unified_scan_run_start(
            &unified_run_id,
            &now_iso,
            &audio_scan_id,
            &daw_scan_id,
            &preset_scan_id,
            &pdf_scan_id,
            &roots_json,
        );

        let _ = db.audio_scan_parent_create(&audio_scan_id, &now_iso, &audio_roots_strs);
        let _ = db.daw_scan_parent_create(&daw_scan_id, &now_iso, &daw_roots_strs);
        let _ = db.preset_scan_parent_create(&preset_scan_id, &now_iso, &preset_roots_strs);
        let _ = db.pdf_scan_parent_create(&pdf_scan_id, &now_iso, &pdf_roots_strs);

        let mut audio_count: u64 = 0;
        let mut daw_count: u64 = 0;
        let mut preset_count: u64 = 0;
        let mut pdf_count: u64 = 0;
        let mut audio_bytes: u64 = 0;
        let mut daw_bytes: u64 = 0;
        let mut preset_bytes: u64 = 0;
        let mut pdf_bytes: u64 = 0;
        let mut audio_format_counts: HashMap<String, usize> = HashMap::new();
        let mut daw_daw_counts: HashMap<String, usize> = HashMap::new();
        let mut preset_format_counts: HashMap<String, usize> = HashMap::new();

        let audio_state2 = app_handle.state::<AudioScanState>();
        let daw_state2 = app_handle.state::<DawScanState>();
        let preset_state2 = app_handle.state::<PresetScanState>();
        let pdf_state2 = app_handle.state::<PdfScanState>();

        let closure_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            unified_walker::walk_unified(
                &spec,
                &mut |batch, _counts| {
                    use unified_walker::ClassifiedBatch;
                    match batch {
                        ClassifiedBatch::Audio(b) => {
                            for s in &b {
                                audio_bytes += s.size;
                                *audio_format_counts.entry(s.format.clone()).or_insert(0) += 1;
                            }
                            let inserted = db.insert_audio_batch(&audio_scan_id, &b).unwrap_or(0);
                            audio_count += inserted;
                            let _ = app_handle.emit(
                                "audio-scan-progress",
                                serde_json::json!({
                                    "phase": "scanning",
                                    "samples": &b,
                                    "found": audio_count,
                                }),
                            );
                        }
                        ClassifiedBatch::Daw(b) => {
                            let inserted_idx =
                                db.insert_daw_batch(&daw_scan_id, &b).unwrap_or_default();
                            let deduped: Vec<&DawProject> =
                                inserted_idx.iter().map(|&i| &b[i]).collect();
                            for p in &deduped {
                                daw_bytes += p.size;
                                *daw_daw_counts.entry(p.daw.clone()).or_insert(0) += 1;
                            }
                            daw_count += deduped.len() as u64;
                            let _ = app_handle.emit(
                                "daw-scan-progress",
                                serde_json::json!({
                                    "phase": "scanning",
                                    "projects": &deduped,
                                    "found": daw_count,
                                }),
                            );
                        }
                        ClassifiedBatch::Preset(b) => {
                            for p in &b {
                                preset_bytes += p.size;
                                *preset_format_counts.entry(p.format.clone()).or_insert(0) += 1;
                            }
                            let inserted = db.insert_preset_batch(&preset_scan_id, &b).unwrap_or(0);
                            preset_count += inserted;
                            let _ = app_handle.emit(
                                "preset-scan-progress",
                                serde_json::json!({
                                    "phase": "scanning",
                                    "presets": &b,
                                    "found": preset_count,
                                }),
                            );
                        }
                        ClassifiedBatch::Pdf(b) => {
                            for p in &b {
                                pdf_bytes += p.size;
                            }
                            let inserted = db.insert_pdf_batch(&pdf_scan_id, &b).unwrap_or(0);
                            pdf_count += inserted;
                            let _ = app_handle.emit(
                                "pdf-scan-progress",
                                serde_json::json!({
                                    "phase": "scanning",
                                    "pdfs": &b,
                                    "found": pdf_count,
                                }),
                            );
                        }
                    }
                },
                &|| {
                    // Any individual stop_* command cancels the unified scan.
                    audio_state2.stop_scan.load(Ordering::SeqCst)
                        || daw_state2.stop_scan.load(Ordering::SeqCst)
                        || preset_state2.stop_scan.load(Ordering::SeqCst)
                        || pdf_state2.stop_scan.load(Ordering::SeqCst)
                },
                // Fan the walker's current-dir updates into all 4 WalkerStatus
                // lists so each walker-status tile shows live progress.
                {
                    let ws = app_handle.state::<WalkerStatus>();
                    vec![
                        Arc::clone(&ws.audio_dirs),
                        Arc::clone(&ws.daw_dirs),
                        Arc::clone(&ws.preset_dirs),
                        Arc::clone(&ws.pdf_dirs),
                    ]
                },
                incremental_state.clone(),
            );

            let stopped = audio_state2.stop_scan.load(Ordering::Relaxed)
                || daw_state2.stop_scan.load(Ordering::Relaxed)
                || preset_state2.stop_scan.load(Ordering::Relaxed)
                || pdf_state2.stop_scan.load(Ordering::Relaxed);

            if !stopped {
                persist_incremental_dir_state_after_walk(incremental_state.as_ref(), &audio_scan_id);
            }

            // Clear WalkerStatus dir lists so tiles return to idle state.
            {
                let ws = app_handle.state::<WalkerStatus>();
                for sink in [&ws.audio_dirs, &ws.daw_dirs, &ws.preset_dirs, &ws.pdf_dirs] {
                    sink.lock().unwrap_or_else(|e| e.into_inner()).clear();
                }
            }

            // Finalize parent scan rows with real totals now that streaming is done.
            let _ = db.audio_scan_parent_finalize(
                &audio_scan_id,
                audio_count,
                audio_bytes,
                &audio_format_counts,
            );
            let _ = db.daw_scan_parent_finalize(
                &daw_scan_id,
                daw_count as usize,
                daw_bytes,
                &daw_daw_counts,
            );
            let _ = db.preset_scan_parent_finalize(
                &preset_scan_id,
                preset_count as usize,
                preset_bytes,
                &preset_format_counts,
            );
            let _ = db.pdf_scan_parent_finalize(&pdf_scan_id, pdf_count as usize, pdf_bytes);
            let complete = !stopped;
            let _ = db.set_audio_scan_complete(&audio_scan_id, complete);
            let _ = db.set_daw_scan_complete(&daw_scan_id, complete);
            let _ = db.set_preset_scan_complete(&preset_scan_id, complete);
            let _ = db.set_pdf_scan_complete(&pdf_scan_id, complete);
            db.checkpoint();

            let finished_at = history::now_iso();
            if stopped {
                let _ = db.unified_scan_run_finish(&finished_at, "stopped", None, None);
            } else {
                let _ = db.unified_scan_run_finish(&finished_at, "complete", None, None);
            }

            serde_json::json!({
                "audioCount": audio_count,
                "dawCount": daw_count,
                "presetCount": preset_count,
                "pdfCount": pdf_count,
                "audioRoots": audio_roots_strs,
                "dawRoots": daw_roots_strs,
                "presetRoots": preset_roots_strs,
                "pdfRoots": pdf_roots_strs,
                "audioScanId": audio_scan_id,
                "dawScanId": daw_scan_id,
                "presetScanId": preset_scan_id,
                "pdfScanId": pdf_scan_id,
                "unifiedRunId": unified_run_id,
                "stopped": stopped,
                "streamed": true,
            })
        }));

        match closure_result {
            Ok(v) => Ok(v),
            Err(_) => {
                let _ = db.unified_scan_run_finish(
                    &history::now_iso(),
                    "error",
                    Some("panic"),
                    None,
                );
                Err("unified scan panicked".into())
            }
        }
    })
    .await;

    let result: Result<serde_json::Value, String> = match result {
        Ok(inner) => inner,
        Err(e) => Err(e.to_string()),
    };

    audio_state.scanning.store(false, Ordering::SeqCst);
    daw_state.scanning.store(false, Ordering::SeqCst);
    preset_state.scanning.store(false, Ordering::SeqCst);
    pdf_state.scanning.store(false, Ordering::SeqCst);
    app.state::<WalkerStatus>()
        .unified_scanning
        .store(false, Ordering::SeqCst);

    let elapsed = scan_start.elapsed();
    match &result {
        Ok(v) => append_log(format!(
            "SCAN END — unified | {}s | audio:{} daw:{} preset:{} pdf:{}",
            elapsed.as_secs(),
            v.get("audioCount").and_then(|x| x.as_u64()).unwrap_or(0),
            v.get("dawCount").and_then(|x| x.as_u64()).unwrap_or(0),
            v.get("presetCount").and_then(|x| x.as_u64()).unwrap_or(0),
            v.get("pdfCount").and_then(|x| x.as_u64()).unwrap_or(0),
        )),
        Err(e) => append_log(format!(
            "SCAN ERROR — unified | {}s | {}",
            elapsed.as_secs(),
            e
        )),
    }
    result
}

#[tauri::command]
async fn get_unified_scan_run() -> Result<db::UnifiedScanRunRow, String> {
    blocking_res(|| db::global().get_unified_scan_run()).await
}

/// Clears unified stop flags **before** the `scan_unified` invoke (after the
/// frontend's listener-registration delay). Without this, `scan_unified` would
/// reset `stop_scan` to false at entry and erase a Stop All that happened during
/// that delay — scans looked like they "could not stop".
#[tauri::command]
async fn prepare_unified_scan(app: AppHandle) -> Result<(), String> {
    app.state::<AudioScanState>()
        .stop_scan
        .store(false, Ordering::SeqCst);
    app.state::<DawScanState>()
        .stop_scan
        .store(false, Ordering::SeqCst);
    app.state::<PresetScanState>()
        .stop_scan
        .store(false, Ordering::SeqCst);
    app.state::<PdfScanState>()
        .stop_scan
        .store(false, Ordering::SeqCst);
    Ok(())
}

// Stops a running unified scan by setting stop flags on all four per-type
// scan states. The scan loop checks these each iteration and breaks out.
#[tauri::command]
async fn stop_unified_scan(app: AppHandle) -> Result<(), String> {
    append_log("SCAN STOP — unified (user requested)".into());
    app.state::<AudioScanState>()
        .stop_scan
        .store(true, Ordering::SeqCst);
    app.state::<DawScanState>()
        .stop_scan
        .store(true, Ordering::SeqCst);
    app.state::<PresetScanState>()
        .stop_scan
        .store(true, Ordering::SeqCst);
    app.state::<PdfScanState>()
        .stop_scan
        .store(true, Ordering::SeqCst);
    Ok(())
}

#[tauri::command]
async fn pdf_history_save(
    pdfs: Vec<PdfFile>,
    roots: Option<Vec<String>>,
) -> Result<history::PdfScanSnapshot, String> {
    let roots = roots.unwrap_or_default();
    blocking_res(move || {
        let snap = history::build_pdf_snapshot(&pdfs, &roots);
        db::global().save_pdf_scan(&snap)?;
        db::global().checkpoint();
        Ok(snap)
    })
    .await
}

#[tauri::command]
async fn pdf_history_get_scans() -> Result<Vec<serde_json::Value>, String> {
    blocking_res(|| db::global().get_pdf_scans()).await
}

#[tauri::command]
async fn pdf_history_get_detail(id: String) -> Result<history::PdfScanSnapshot, String> {
    blocking_res(move || db::global().get_pdf_scan_detail(&id)).await
}

#[tauri::command]
async fn pdf_history_delete(id: String) -> Result<(), String> {
    blocking_res(move || db::global().delete_pdf_scan(&id)).await
}

#[tauri::command]
async fn pdf_history_clear() -> Result<(), String> {
    #[cfg(not(test))]
    append_log("HISTORY CLEAR — pdfs".into());
    blocking_res(|| db::global().clear_pdf_history()).await
}

#[tauri::command]
async fn pdf_history_latest() -> Result<Option<history::PdfScanSnapshot>, String> {
    blocking_res(|| db::global().get_latest_pdf_scan()).await
}

#[tauri::command]
async fn pdf_history_diff(old_id: String, new_id: String) -> Option<history::PdfScanDiff> {
    tokio::task::spawn_blocking(move || {
        let old = db::global().get_pdf_scan_detail(&old_id).ok()?;
        let new = db::global().get_pdf_scan_detail(&new_id).ok()?;
        Some(history::compute_pdf_diff(&old, &new))
    })
    .await
    .ok()
    .flatten()
}

#[tauri::command]
async fn open_pdf_file(file_path: String) -> Result<(), String> {
    open_plugin_folder(file_path).await
}

#[tauri::command]
async fn pdf_metadata_get(paths: Vec<String>) -> Result<serde_json::Value, String> {
    tokio::task::spawn_blocking(move || {
        let map = db::global().get_pdf_metadata(&paths)?;
        let mut out = serde_json::Map::new();
        for (k, v) in map {
            out.insert(
                k,
                match v {
                    Some(n) => serde_json::json!(n),
                    None => serde_json::Value::Null,
                },
            );
        }
        Ok::<serde_json::Value, String>(serde_json::Value::Object(out))
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
fn pdf_metadata_extract_abort() {
    PDF_META_EXTRACT_ABORT.store(true, Ordering::Relaxed);
}

#[tauri::command]
async fn pdf_metadata_extract_batch(
    app: AppHandle,
    paths: Vec<String>,
) -> Result<serde_json::Value, String> {
    tokio::task::spawn_blocking(move || {
        PDF_META_EXTRACT_ABORT.store(false, Ordering::Relaxed);
        let total = paths.len();
        if total == 0 {
            return serde_json::json!({ "extracted": 0, "total": 0, "aborted": false });
        }
        let _ = app.emit(
            "pdf-metadata-progress",
            serde_json::json!({ "phase": "start", "total": total }),
        );
        // Chunk so we can emit progress + persist incrementally
        const CHUNK: usize = 100;
        let mut done = 0usize;
        let mut extracted = 0usize;
        for chunk in paths.chunks(CHUNK) {
            if PDF_META_EXTRACT_ABORT.load(Ordering::Relaxed) {
                let _ = app.emit(
                    "pdf-metadata-progress",
                    serde_json::json!({
                        "phase": "aborted", "done": done, "total": total, "extracted": extracted
                    }),
                );
                let _ = app.emit(
                    "pdf-metadata-progress",
                    serde_json::json!({
                        "phase": "done", "extracted": extracted, "total": total, "aborted": true
                    }),
                );
                return serde_json::json!({ "extracted": extracted, "total": total, "aborted": true });
            }
            let pairs = pdf_meta::extract_pages_batch(chunk);
            // Build a HashMap for O(1) lookup instead of O(n) linear scan per element
            let pairs_map: std::collections::HashMap<&String, u32> =
                pairs.iter().map(|(p, n)| (p, *n)).collect();
            let mut rows: Vec<(String, Option<u32>)> = Vec::with_capacity(chunk.len());
            for p in chunk {
                rows.push((p.clone(), pairs_map.get(p).copied()));
            }
            let _ = db::global().save_pdf_metadata(&rows);
            extracted += pairs.len();
            done += chunk.len();
            let _ = app.emit(
                "pdf-metadata-progress",
                serde_json::json!({
                    "phase": "progress", "done": done, "total": total, "extracted": extracted
                }),
            );
        }
        let _ = app.emit(
            "pdf-metadata-progress",
            serde_json::json!({ "phase": "done", "extracted": extracted, "total": total, "aborted": false }),
        );
        serde_json::json!({ "extracted": extracted, "total": total, "aborted": false })
    })
    .await
    .map_err(|e| e.to_string())
}

/// Paths in the PDF library (`pdf_library`) with no `pdf_metadata` row yet — used to kick off
/// background page-count extraction for the whole inventory, not only the latest scan.
#[tauri::command]
async fn pdf_metadata_unindexed(limit: Option<u64>) -> Result<Vec<String>, String> {
    let lim = limit.unwrap_or(100000);
    blocking_res(move || db::global().unindexed_pdf_paths(lim)).await
}

#[tauri::command]
async fn open_preset_folder(file_path: String) -> Result<(), String> {
    open_plugin_folder(file_path).await
}

#[tauri::command]
async fn open_daw_folder(file_path: String) -> Result<(), String> {
    open_plugin_folder(file_path).await
}

#[tauri::command]
async fn open_daw_project(file_path: String) -> Result<(), String> {
    let path = std::path::Path::new(&file_path);
    if !path.exists() {
        return Err(format!("File not found: {}", file_path));
    }

    #[cfg(target_os = "macos")]
    {
        let output = std::process::Command::new("open")
            .arg(&file_path)
            .output()
            .map_err(|e| e.to_string())?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("No application can open") || stderr.contains("no application set") {
                return Err("No application installed to open this project file".to_string());
            }
            return Err(format!("Failed to open project: {}", stderr.trim()));
        }
    }

    #[cfg(target_os = "windows")]
    {
        let output = std::process::Command::new("cmd")
            .args(["/C", "start", "", &file_path])
            .output()
            .map_err(|e| e.to_string())?;
        if !output.status.success() {
            return Err("No application installed to open this project file".to_string());
        }
    }

    #[cfg(target_os = "linux")]
    {
        let output = std::process::Command::new("xdg-open")
            .arg(&file_path)
            .output()
            .map_err(|e| format!("No application installed to open this project file: {}", e))?;
        if !output.status.success() {
            return Err("No application installed to open this project file".to_string());
        }
    }

    Ok(())
}

#[tauri::command]
async fn extract_project_plugins(file_path: String) -> Result<Vec<xref::PluginRef>, String> {
    let mut result = xref::extract_plugins(&file_path);
    // Enrich empty manufacturers from scanned plugin database
    if result.iter().any(|p| p.manufacturer.is_empty()) {
        if let Ok(all) = db::global().query_plugins(None, None, None, "name", true, false, 0, 100000) {
            let mfg_map: std::collections::HashMap<String, String> = all
                .plugins
                .iter()
                .filter(|p| !p.manufacturer.is_empty())
                .map(|p| (p.name.to_lowercase(), p.manufacturer.clone()))
                .collect();
            for p in &mut result {
                if p.manufacturer.is_empty() {
                    if let Some(mfg) = mfg_map.get(&p.name.to_lowercase()) {
                        p.manufacturer = mfg.clone();
                    }
                }
            }
        }
    }
    #[cfg(not(test))]
    append_log(format!(
        "XREF EXTRACT — {} | {} plugins found",
        file_path,
        result.len()
    ));
    Ok(result)
}

fn read_als_xml_impl(file_path: &str) -> Result<String, String> {
    use flate2::read::GzDecoder;
    use std::io::Read;
    let data = std::fs::read(file_path).map_err(|e| e.to_string())?;
    let mut decoder = GzDecoder::new(&data[..]);
    const MAX_XML_SIZE: usize = 20_000_000; // 20MB cap to prevent WebView OOM
    let mut xml = String::new();
    decoder
        .read_to_string(&mut xml)
        .map_err(|e| format!("Not a valid gzip file: {}", e))?;
    if xml.len() > MAX_XML_SIZE {
        xml.truncate(MAX_XML_SIZE);
        xml.push_str("\n<!-- TRUNCATED: file too large for viewer -->");
    }
    Ok(xml)
}

#[tauri::command]
async fn read_als_xml(file_path: String) -> Result<String, String> {
    blocking_res(move || read_als_xml_impl(&file_path)).await
}

#[tauri::command]
async fn estimate_bpm(file_path: String) -> Result<Option<f64>, String> {
    Ok(bpm::estimate_bpm(&file_path))
}

#[tauri::command]
async fn detect_audio_key(file_path: String) -> Result<Option<String>, String> {
    tokio::task::spawn_blocking(move || key_detect::detect_key(&file_path))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn measure_lufs(file_path: String) -> Result<Option<f64>, String> {
    tokio::task::spawn_blocking(move || lufs::measure_lufs(&file_path))
        .await
        .map_err(|e| e.to_string())
}

/// Batch analyze: BPM + Key + LUFS for multiple files in parallel, save to SQLite.
/// Analyzes files in parallel (rayon), batch-writes to DB, returns results
/// directly so the frontend can update visible rows without extra IPC.
///
/// Uses a **small dedicated rayon pool** (at most 4 threads) so a full batch does not claim every
/// CPU core during a library scan — unbounded default rayon was starving other work (including
/// audio I/O in the separate engine process on the same machine).
#[tauri::command]
async fn batch_analyze(paths: Vec<String>) -> Result<serde_json::Value, String> {
    let inner = tokio::task::spawn_blocking(move || {
        use rayon::prelude::*;
        if paths.is_empty() {
            return Ok(serde_json::json!({ "count": 0, "results": [] }));
        }
        const MAX_BATCH_ANALYSIS_THREADS: usize = 4;
        let num_threads = std::cmp::min(paths.len(), MAX_BATCH_ANALYSIS_THREADS).max(1);
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(num_threads)
            .build()
            .expect("batch_analyze rayon pool");
        let results: Vec<db::AnalysisBatchRow> = pool.install(|| {
            paths
                .par_iter()
                .map(|path| {
                    let bpm_val = bpm::estimate_bpm(path);
                    let key_val = key_detect::detect_key(path);
                    let lufs_val = lufs::measure_lufs(path);
                    (path.clone(), bpm_val, key_val, lufs_val)
                })
                .collect()
        });
        // Batch all DB writes in a single transaction
        let count = db::global().batch_update_analysis(&results)?;
        // Return results so frontend skips N individual dbGetAnalysis IPC calls
        let items: Vec<serde_json::Value> = results
            .iter()
            .map(|(path, bpm, key, lufs)| {
                let bpm_exhausted = bpm.is_none() && key.is_some() && lufs.is_some();
                serde_json::json!({
                    "path": path,
                    "bpm": bpm,
                    "key": key,
                    "lufs": lufs,
                    "bpmExhausted": bpm_exhausted,
                })
            })
            .collect();
        Ok(serde_json::json!({ "count": count, "results": items }))
    })
    .await
    .map_err(|e| e.to_string())?;
    inner
}

#[tauri::command]
async fn compute_fingerprint(
    file_path: String,
) -> Result<Option<similarity::AudioFingerprint>, String> {
    tokio::task::spawn_blocking(move || similarity::compute_fingerprint(&file_path))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn build_fingerprint_cache(
    app: AppHandle,
    candidate_paths: Vec<String>,
) -> Result<serde_json::Value, String> {
    tokio::task::spawn_blocking(move || {
        let fp_json = db::global()
            .read_cache("fingerprint-cache.json")
            .unwrap_or_default();
        let raw: HashMap<String, similarity::AudioFingerprint> =
            serde_json::from_value(fp_json).unwrap_or_default();
        let mut cache = normalize_fingerprint_cache_map(raw);
        use rayon::prelude::*;
        let uncached: Vec<&String> = candidate_paths
            .iter()
            .filter(|p| !cache.contains_key(&normalize_path_for_db(p.as_str())))
            .collect();
        let total = uncached.len();
        if total == 0 {
            return serde_json::json!({ "built": 0, "cached": cache.len() });
        }
        let _ = app.emit(
            "fingerprint-build-progress",
            serde_json::json!({
                "phase": "start", "total": total, "cached": cache.len()
            }),
        );
        const CHUNK: usize = 500;
        let mut done = 0usize;
        for chunk in uncached.chunks(CHUNK) {
            let new_fps: Vec<similarity::AudioFingerprint> = chunk
                .par_iter()
                .filter_map(|p| similarity::compute_fingerprint(p))
                .collect();
            for mut fp in new_fps {
                let k = normalize_path_for_db(&fp.path);
                fp.path = k.clone();
                cache.insert(k, fp);
            }
            done += chunk.len();
            let _ = app.emit(
                "fingerprint-build-progress",
                serde_json::json!({
                    "phase": "progress", "done": done, "total": total
                }),
            );
            if let Ok(val) = serde_json::to_value(&cache) {
                let _ = db::global().write_cache("fingerprint-cache.json", &val);
            }
        }
        let _ = app.emit(
            "fingerprint-build-progress",
            serde_json::json!({ "phase": "done", "built": done, "cached": cache.len() }),
        );
        serde_json::json!({ "built": done, "cached": cache.len() })
    })
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
async fn find_similar_samples(
    app: AppHandle,
    file_path: String,
    candidate_paths: Vec<String>,
    max_results: usize,
) -> Result<Vec<serde_json::Value>, String> {
    tokio::task::spawn_blocking(move || {
        // Load cached fingerprints from SQLite
        let fp_json = db::global()
            .read_cache("fingerprint-cache.json")
            .unwrap_or_default();
        let raw: HashMap<String, similarity::AudioFingerprint> =
            serde_json::from_value(fp_json).unwrap_or_default();
        let mut cache = normalize_fingerprint_cache_map(raw);

        let file_key = normalize_path_for_db(&file_path);
        // Compute reference fingerprint (use cache if available)
        let reference = if let Some(fp) = cache.get(&file_key) {
            fp.clone()
        } else {
            match similarity::compute_fingerprint(&file_path) {
                Some(mut fp) => {
                    let k = normalize_path_for_db(&fp.path);
                    fp.path = k.clone();
                    cache.insert(k.clone(), fp.clone());
                    fp
                }
                None => return vec![],
            }
        };

        // Compute missing fingerprints in parallel
        use rayon::prelude::*;
        let uncached: Vec<&String> = candidate_paths
            .iter()
            .filter(|p| !cache.contains_key(&normalize_path_for_db(p.as_str())))
            .collect();

        if !uncached.is_empty() {
            // Emit progress (explicit counts — `total` alone was uncached-only and confused the UI)
            let uncached_count = uncached.len();
            let candidate_count = candidate_paths.len();
            let cached_count = candidate_count.saturating_sub(uncached_count);
            let _ = app.emit(
                "similarity-progress",
                serde_json::json!({
                    "phase": "computing",
                    "candidate_count": candidate_count,
                    "uncached_count": uncached_count,
                    "cached_count": cached_count,
                    "total": uncached_count,
                    "cached": cached_count
                }),
            );

            let new_fps: Vec<similarity::AudioFingerprint> = uncached
                .par_iter()
                .filter_map(|p| similarity::compute_fingerprint(p))
                .collect();

            for mut fp in new_fps {
                let k = normalize_path_for_db(&fp.path);
                fp.path = k.clone();
                cache.insert(k, fp);
            }

            // Save cache to SQLite
            if let Ok(val) = serde_json::to_value(&cache) {
                let _ = db::global().write_cache("fingerprint-cache.json", &val);
            }
        }

        // Collect cached fingerprints for candidates
        let candidates: Vec<similarity::AudioFingerprint> = candidate_paths
            .iter()
            .filter_map(|p| cache.get(&normalize_path_for_db(p.as_str())).cloned())
            .collect();

        similarity::find_similar(&reference, &candidates, max_results)
            .into_iter()
            .map(|(path, distance)| {
                serde_json::json!({
                    "path": path,
                    "distance": distance,
                    "similarity": (1.0 - distance.min(1.0)) * 100.0
                })
            })
            .collect()
    })
    .await
    .map_err(|e| e.to_string())
}

/// Byte-identical files across the scanned library (SHA-256 after grouping by stored size).
#[tauri::command]
async fn find_content_duplicates(app: AppHandle) -> Result<serde_json::Value, String> {
    let app_pb = Arc::new(app);
    tokio::task::spawn_blocking(move || {
        let entries = db::global().library_paths_for_content_hash()?;
        let progress = Some((app_pb, 25usize));
        let r = content_hash::find_byte_duplicate_groups(entries, progress);
        serde_json::to_value(&r).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn open_file_default(file_path: String) -> Result<(), String> {
    blocking_res(move || {
        let path = std::path::Path::new(&file_path);
        if !path.exists() {
            return Err(format!("File not found: {}", file_path));
        }
        #[cfg(target_os = "macos")]
        {
            let output = std::process::Command::new("open")
                .arg(&file_path)
                .output()
                .map_err(|e| e.to_string())?;
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(format!("open failed: {}", stderr.trim()));
            }
        }
        #[cfg(target_os = "windows")]
        {
            let output = std::process::Command::new("cmd")
                .args(["/C", "start", "", &file_path])
                .output()
                .map_err(|e| e.to_string())?;
            if !output.status.success() {
                return Err("start failed".into());
            }
        }
        #[cfg(target_os = "linux")]
        {
            let output = std::process::Command::new("xdg-open")
                .arg(&file_path)
                .output()
                .map_err(|e| e.to_string())?;
            if !output.status.success() {
                return Err("xdg-open failed".into());
            }
        }
        Ok(())
    })
    .await
}

#[tauri::command]
async fn open_with_app(file_path: String, app_name: String) -> Result<(), String> {
    blocking_res(move || {
        let path = std::path::Path::new(&file_path);
        open_with_app::open_with_application(path, &app_name)
    })
    .await
}

#[tauri::command]
async fn open_update_url(url: String) -> Result<(), String> {
    opener::open(&url).map_err(|e| e.to_string())
}

#[tauri::command]
async fn open_plugin_folder(plugin_path: String) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg("-R")
            .arg(&plugin_path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(format!("/select,{}", plugin_path))
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "linux")]
    {
        let parent = std::path::Path::new(&plugin_path)
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        opener::open(&parent).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
async fn open_audio_folder(file_path: String) -> Result<(), String> {
    open_plugin_folder(file_path).await
}

// ── Preferences commands ──

#[tauri::command]
async fn prefs_get_all() -> history::PrefsMap {
    blocking(|| history::load_preferences())
        .await
        .unwrap_or_default()
}

#[tauri::command]
async fn prefs_set(key: String, value: serde_json::Value) {
    let refresh_log = key == "logVerbosity";
    let _ = blocking_res(move || {
        history::set_preference(&key, value);
        Ok(())
    })
    .await;
    if refresh_log {
        refresh_log_verbosity_from_prefs();
    }
}

#[tauri::command]
async fn prefs_remove(key: String) {
    let _ = blocking_res(move || {
        history::remove_preference(&key);
        Ok(())
    })
    .await;
}

#[tauri::command]
async fn prefs_save_all(prefs: history::PrefsMap) {
    let _ = blocking_res(move || {
        history::save_preferences(&prefs);
        Ok(())
    })
    .await;
    refresh_log_verbosity_from_prefs();
}

#[tauri::command]
async fn open_prefs_file() -> Result<(), String> {
    let path = history::get_preferences_path();
    opener::open(&path).map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_prefs_path() -> String {
    blocking(|| {
        history::get_preferences_path()
            .to_string_lossy()
            .to_string()
    })
    .await
    .unwrap_or_default()
}

// Cache file read/write — backed by SQLite
#[tauri::command]
async fn read_cache_file(name: String) -> Result<serde_json::Value, String> {
    blocking_res(move || db::global().read_cache(&name)).await
}

#[tauri::command]
async fn write_cache_file(name: String, data: serde_json::Value) -> Result<(), String> {
    blocking_res(move || db::global().write_cache(&name, &data)).await
}

#[tauri::command]
async fn audio_engine_invoke(request: serde_json::Value) -> Result<serde_json::Value, String> {
    let payload = audio_engine::normalize_ipc_request_payload(&request);
    let v = tokio::task::spawn_blocking({
        let payload = payload.clone();
        move || audio_engine::spawn_audio_engine_request(&payload)
    })
    .await
    .map_err(|e| format!("audio-engine spawn_blocking: {e}"))??;
    if v.get("ok") == Some(&serde_json::Value::Bool(false)) {
        let cmd = payload
            .get("cmd")
            .and_then(|c| c.as_str())
            .unwrap_or("?");
        let err = v
            .get("error")
            .and_then(|e| e.as_str())
            .unwrap_or("?");
        write_app_log(format!("audio-engine [{cmd}] {err}"));
    }
    Ok(v)
}

#[tauri::command]
fn audio_engine_restart() -> Result<(), String> {
    audio_engine::restart_audio_engine_child()
}

#[tauri::command]
fn audio_engine_eof_watchdog_start(app: AppHandle) -> Result<(), String> {
    audio_engine::audio_engine_eof_watchdog_start(app);
    Ok(())
}

#[tauri::command]
fn audio_engine_eof_watchdog_stop() -> Result<(), String> {
    audio_engine::audio_engine_eof_watchdog_stop();
    Ok(())
}

#[tauri::command]
fn append_log(msg: String) {
    write_app_log(msg);
}

fn write_app_log_line(msg: &str) {
    let path = history::get_data_dir().join("app.log");
    // Ensure dir exists
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    // Rotate if > 5MB — rename to app.log.1 (drop prior backup); if rename fails, truncate in place
    const MAX_LOG_SIZE: u64 = 5 * 1024 * 1024;
    if let Ok(meta) = std::fs::metadata(&path) {
        if meta.len() > MAX_LOG_SIZE {
            let backup = path.with_extension("log.1");
            let _ = std::fs::remove_file(&backup);
            if std::fs::rename(&path, &backup).is_err() {
                let _ = std::fs::write(&path, "");
            }
        }
    }
    let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let line = format!("[{}] {}\n", timestamp, msg);
    let _ = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .and_then(|mut f| {
            use std::io::Write;
            f.write_all(line.as_bytes())
        });
}

/// Extra diagnostics when `logVerbosity` is `verbose`. `f` runs only if verbose (no `format!` cost otherwise).
pub fn app_log_verbose<F: FnOnce() -> String>(f: F) {
    if LOG_VERBOSITY_LEVEL.load(Ordering::Relaxed) < 2 {
        return;
    }
    write_app_log_line(&f());
}

/// Like [`app_log_verbose`] when the message is already a `String`.
pub fn write_app_log_verbose(msg: String) {
    if LOG_VERBOSITY_LEVEL.load(Ordering::Relaxed) < 2 {
        return;
    }
    write_app_log_line(&msg);
}

/// Public log-append entry point callable from any module. Writes a
/// timestamped line to `<data-dir>/app.log`, rotating to `.log.1` at 5MB.
/// The `append_log` Tauri command delegates to this.
pub fn write_app_log(msg: String) {
    if should_suppress_app_log_line(&msg) {
        return;
    }
    write_app_log_line(&msg);
}

#[tauri::command]
async fn read_log() -> Result<String, String> {
    blocking_res(|| {
        let path = history::get_data_dir().join("app.log");
        match std::fs::read_to_string(&path) {
            Ok(s) => Ok(s),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
            Err(e) => Err(e.to_string()),
        }
    })
    .await
}

#[tauri::command]
async fn clear_log() -> Result<(), String> {
    blocking_res(|| {
        let path = history::ensure_data_dir().join("app.log");
        std::fs::write(&path, "").map_err(|e| e.to_string())
    })
    .await
}

/// Generic project file reader: returns {type: "xml"|"tree", content: ...}
/// XML formats get raw XML string, binary formats get structured JSON tree.
#[tauri::command]
async fn read_project_file(file_path: String) -> Result<serde_json::Value, String> {
    blocking_res(move || {
        let path = std::path::Path::new(&file_path);
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        match ext.as_str() {
            "als" => {
                let xml = read_als_xml_impl(&file_path)?;
                Ok(serde_json::json!({"type": "xml", "format": "Ableton Live Set", "content": xml, "path": file_path}))
            }
            "song" => {
                let xml = read_zip_xml(&file_path, &["song.xml", "Song/song.xml", "metainfo.xml"])?;
                Ok(serde_json::json!({"type": "xml", "format": "Studio One Song", "content": xml, "path": file_path}))
            }
            "dawproject" => {
                let xml = read_zip_xml(&file_path, &["project.xml", "metadata.xml"])?;
                Ok(serde_json::json!({"type": "xml", "format": "DAWproject", "content": xml, "path": file_path}))
            }
            "rpp" | "rpp-bak" => {
                let content = std::fs::read_to_string(&file_path).map_err(|e| e.to_string())?;
                Ok(serde_json::json!({"type": "text", "format": "REAPER Project", "content": content, "path": file_path}))
            }
            _ => read_binary_project(file_path, &ext),
        }
    })
    .await
}

/// Read XML from a ZIP archive (Studio One, DAWproject).
fn read_zip_xml(file_path: &str, names: &[&str]) -> Result<String, String> {
    use std::io::Read;
    let file = std::fs::File::open(file_path).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| format!("Not a valid ZIP: {e}"))?;
    for name in names {
        if let Ok(mut entry) = archive.by_name(name) {
            let mut s = String::new();
            entry.read_to_string(&mut s).map_err(|e| e.to_string())?;
            if !s.is_empty() {
                return Ok(s);
            }
        }
    }
    // List all files and return the first XML found
    let mut xml_name = None;
    for i in 0..archive.len() {
        if let Ok(entry) = archive.by_index(i) {
            if entry.name().ends_with(".xml") {
                xml_name = Some(entry.name().to_string());
                break;
            }
        }
    }
    if let Some(name) = xml_name {
        let mut entry = archive.by_name(&name).map_err(|e| e.to_string())?;
        let mut s = String::new();
        entry.read_to_string(&mut s).map_err(|e| e.to_string())?;
        return Ok(s);
    }
    Err("No XML found in archive".into())
}

/// Read any binary DAW project file and return a structured JSON tree.
fn read_binary_project(file_path: String, ext: &str) -> Result<serde_json::Value, String> {
    let format_name = match ext {
        "bwproject" => "Bitwig Studio Project (.bwproject)",
        "flp" => "FL Studio Project (.flp)",
        "logicx" => "Logic Pro Project (.logicx)",
        "cpr" => "Cubase Project (.cpr)",
        "npr" => "Nuendo Project (.npr)",
        "ptx" => "Pro Tools Session (.ptx)",
        "ptf" => "Pro Tools Session (.ptf)",
        "reason" => "Reason Song (.reason)",
        "band" => "GarageBand Project (.band)",
        _ => "Binary DAW Project",
    };
    let mut result = read_binary_project_inner(&file_path)?;
    if let Some(obj) = result.as_object_mut() {
        obj.insert(
            "_format".into(),
            serde_json::Value::String(format_name.into()),
        );
    }
    Ok(result)
}

fn read_binary_project_inner(file_path: &str) -> Result<serde_json::Value, String> {
    let path = std::path::Path::new(file_path);
    // Handle macOS package directories (e.g. .bwproject, .logicx)
    let data = if path.is_dir() {
        // Read all files in the package and concatenate
        let mut buf = Vec::new();
        fn collect_dir(dir: &std::path::Path, buf: &mut Vec<u8>, limit: usize) {
            if buf.len() > limit {
                return;
            }
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let p = entry.path();
                    if p.is_file() {
                        if let Ok(data) = std::fs::read(&p) {
                            buf.extend_from_slice(&data);
                            if buf.len() > limit {
                                return;
                            }
                        }
                    } else if p.is_dir() {
                        collect_dir(&p, buf, limit);
                    }
                }
            }
        }
        collect_dir(path, &mut buf, 50_000_000); // cap at 50MB
        buf
    } else {
        std::fs::read(file_path).map_err(|e| format!("Failed to read: {e}"))?
    };

    let mut metadata = serde_json::Map::new();
    let mut strings_found = Vec::new();
    let mut plugins = Vec::new();

    // Parse header metadata (key-value pairs encoded as printable strings)
    let mut i = 0;
    while i + 4 < data.len() && i < 10000 {
        if data[i] >= 0x20 && data[i] <= 0x7E {
            let start = i;
            while i < data.len() && data[i] >= 0x20 && data[i] <= 0x7E {
                i += 1;
            }
            if i - start >= 3 {
                let s = String::from_utf8_lossy(&data[start..i]).to_string();
                strings_found.push(s);
            }
        } else {
            i += 1;
        }
    }

    let meta_keys = [
        "album",
        "application_version_name",
        "artist",
        "branch",
        "comment",
        "copyright",
        "creator",
        "genre",
        "orig_artist",
        "producer",
        "title",
        "version",
    ];
    let mut idx = 0;
    while idx + 1 < strings_found.len() {
        let key = &strings_found[idx];
        if meta_keys.contains(&key.as_str()) && idx + 1 < strings_found.len() {
            let val = &strings_found[idx + 1];
            if !val.is_empty() && !meta_keys.contains(&val.as_str()) {
                metadata.insert(key.clone(), serde_json::Value::String(val.clone()));
                idx += 2;
                continue;
            }
        }
        idx += 1;
    }

    // Extract plugin paths from full binary
    let mut current = Vec::new();
    for &byte in &data {
        if (0x20..=0x7E).contains(&byte) {
            current.push(byte);
        } else {
            if current.len() >= 6 {
                let s = String::from_utf8_lossy(&current).to_string();
                if s.ends_with(".dll")
                    || s.ends_with(".vst3")
                    || s.ends_with(".component")
                    || s.ends_with(".clap")
                    || s.ends_with(".aaxplugin")
                {
                    plugins.push(s);
                }
            }
            current.clear();
        }
    }
    plugins.sort();
    plugins.dedup();

    let mut tree = serde_json::Map::new();
    tree.insert(
        "_path".into(),
        serde_json::Value::String(file_path.to_string()),
    );
    tree.insert(
        "_size".into(),
        serde_json::Value::String(format_size(data.len() as u64)),
    );
    tree.insert("metadata".into(), serde_json::Value::Object(metadata));
    tree.insert(
        "plugins".into(),
        serde_json::Value::Array(plugins.into_iter().map(serde_json::Value::String).collect()),
    );

    let mut fxb_count = 0usize;
    for window in data.windows(4) {
        if window == b".fxb" {
            fxb_count += 1;
        }
    }
    if fxb_count > 0 {
        tree.insert(
            "pluginStateCount".into(),
            serde_json::Value::Number(fxb_count.into()),
        );
    }

    Ok(serde_json::Value::Object(tree))
}

#[tauri::command]
async fn read_bwproject(file_path: String) -> Result<serde_json::Value, String> {
    blocking_res(move || read_binary_project(file_path, "bwproject")).await
}

// ── MIDI metadata ──

#[tauri::command]
async fn get_midi_info(file_path: String) -> Result<Option<midi::MidiInfo>, String> {
    blocking_res(move || Ok(midi::parse_midi(std::path::Path::new(&file_path)))).await
}

// ── Export / Import commands ──

fn plugins_to_export(plugins: &[PluginInfo]) -> Vec<ExportPlugin> {
    plugins
        .iter()
        .map(|p| ExportPlugin {
            name: p.name.clone(),
            plugin_type: p.plugin_type.clone(),
            version: p.version.clone(),
            manufacturer: p.manufacturer.clone(),
            manufacturer_url: p.manufacturer_url.clone(),
            path: p.path.clone(),
            size: p.size.clone(),
            size_bytes: p.size_bytes,
            modified: p.modified.clone(),
            architectures: p.architectures.clone(),
        })
        .collect()
}

#[tauri::command]
async fn export_plugins_json(plugins: Vec<PluginInfo>, file_path: String) -> Result<(), String> {
    blocking_res(move || {
        #[cfg(not(test))]
        append_log(format!(
            "EXPORT — {} plugins → {}",
            plugins.len(),
            file_path
        ));
        let payload = ExportPayload {
            version: env!("CARGO_PKG_VERSION").into(),
            exported_at: chrono::Utc::now().to_rfc3339(),
            plugins: plugins_to_export(&plugins),
        };
        let json = serde_json::to_string_pretty(&payload).map_err(|e| e.to_string())?;
        std::fs::write(&file_path, json).map_err(|e| e.to_string())
    })
    .await
}

#[tauri::command]
async fn export_plugins_csv(plugins: Vec<PluginInfo>, file_path: String) -> Result<(), String> {
    blocking_res(move || {
        #[cfg(not(test))]
        append_log(format!(
            "EXPORT — {} plugins → {}",
            plugins.len(),
            file_path
        ));
        let sep = detect_separator(&file_path);
        let mut out = format!(
            "Name{s}Type{s}Version{s}Manufacturer{s}Manufacturer URL{s}Path{s}Size{s}Modified\n",
            s = sep
        );
        for p in &plugins {
            out.push_str(&format!(
                "{}{sep}{}{sep}{}{sep}{}{sep}{}{sep}{}{sep}{}{sep}{}\n",
                dsv_escape(&p.name, sep),
                dsv_escape(&p.plugin_type, sep),
                dsv_escape(&p.version, sep),
                dsv_escape(&p.manufacturer, sep),
                dsv_escape(p.manufacturer_url.as_deref().unwrap_or(""), sep),
                dsv_escape(&p.path, sep),
                dsv_escape(&p.size, sep),
                dsv_escape(&p.modified, sep),
            ));
        }
        std::fs::write(&file_path, out).map_err(|e| e.to_string())
    })
    .await
}

#[cfg(test)]
fn csv_escape(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

fn dsv_escape(s: &str, sep: char) -> String {
    if s.contains(sep) || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

fn detect_separator(file_path: &str) -> char {
    if file_path.ends_with(".tsv") {
        '\t'
    } else {
        ','
    }
}

// ── Audio export ──

#[tauri::command]
async fn export_audio_json(
    samples: Vec<history::AudioSample>,
    file_path: String,
) -> Result<(), String> {
    blocking_res(move || {
        let payload = serde_json::json!({
            "version": env!("CARGO_PKG_VERSION"),
            "exported_at": chrono::Utc::now().to_rfc3339(),
            "samples": samples,
        });
        let json = serde_json::to_string_pretty(&payload).map_err(|e| e.to_string())?;
        std::fs::write(&file_path, json).map_err(|e| e.to_string())
    })
    .await
}

#[tauri::command]
async fn export_audio_dsv(
    samples: Vec<history::AudioSample>,
    file_path: String,
) -> Result<(), String> {
    blocking_res(move || {
        let sep = detect_separator(&file_path);
        let mut out = format!(
            "Name{s}Format{s}Path{s}Directory{s}Size{s}Modified\n",
            s = sep
        );
        for s in &samples {
            out.push_str(&format!(
                "{}{sep}{}{sep}{}{sep}{}{sep}{}{sep}{}\n",
                dsv_escape(&s.name, sep),
                dsv_escape(&s.format, sep),
                dsv_escape(&s.path, sep),
                dsv_escape(&s.directory, sep),
                dsv_escape(&s.size_formatted, sep),
                dsv_escape(&s.modified, sep),
            ));
        }
        std::fs::write(&file_path, out).map_err(|e| e.to_string())
    })
    .await
}

// ── DAW export ──

#[tauri::command]
async fn export_daw_json(
    projects: Vec<history::DawProject>,
    file_path: String,
) -> Result<(), String> {
    blocking_res(move || {
        let payload = serde_json::json!({
            "version": env!("CARGO_PKG_VERSION"),
            "exported_at": chrono::Utc::now().to_rfc3339(),
            "projects": projects,
        });
        let json = serde_json::to_string_pretty(&payload).map_err(|e| e.to_string())?;
        std::fs::write(&file_path, json).map_err(|e| e.to_string())
    })
    .await
}

#[tauri::command]
async fn export_daw_dsv(
    projects: Vec<history::DawProject>,
    file_path: String,
) -> Result<(), String> {
    blocking_res(move || {
        let sep = detect_separator(&file_path);
        let mut out = format!(
            "Name{s}DAW{s}Format{s}Path{s}Directory{s}Size{s}Modified\n",
            s = sep
        );
        for p in &projects {
            out.push_str(&format!(
                "{}{sep}{}{sep}{}{sep}{}{sep}{}{sep}{}{sep}{}\n",
                dsv_escape(&p.name, sep),
                dsv_escape(&p.daw, sep),
                dsv_escape(&p.format, sep),
                dsv_escape(&p.path, sep),
                dsv_escape(&p.directory, sep),
                dsv_escape(&p.size_formatted, sep),
                dsv_escape(&p.modified, sep),
            ));
        }
        std::fs::write(&file_path, out).map_err(|e| e.to_string())
    })
    .await
}

#[tauri::command]
async fn import_plugins_json(file_path: String) -> Result<Vec<PluginInfo>, String> {
    blocking_res(move || {
        #[cfg(not(test))]
        append_log(format!("IMPORT — plugins ← {}", file_path));
        let data = std::fs::read_to_string(&file_path).map_err(|e| e.to_string())?;
        let payload: ExportPayload = serde_json::from_str(&data).map_err(|e| e.to_string())?;
        Ok(payload
            .plugins
            .into_iter()
            .map(|p| PluginInfo {
                name: p.name,
                path: p.path,
                plugin_type: p.plugin_type,
                version: p.version,
                manufacturer: p.manufacturer,
                manufacturer_url: p.manufacturer_url,
                size: p.size,
                size_bytes: p.size_bytes,
                modified: p.modified,
                architectures: p.architectures,
            })
            .collect())
    })
    .await
}

// ── Process stats ──

use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Disk + DB file sizes + `table_counts` are expensive; the UI polls ~1 Hz.
struct SlowStatsSnapshot {
    at: Instant,
    dir_key: String,
    disk_total: u64,
    disk_free: u64,
    db_bytes: u64,
    prefs_bytes: u64,
    table_counts: serde_json::Value,
}

static SLOW_STATS_CACHE: Mutex<Option<SlowStatsSnapshot>> = Mutex::new(None);
const SLOW_STATS_TTL: Duration = Duration::from_secs(4);

fn compute_slow_stats(data_dir: &std::path::Path) -> (u64, u64, u64, u64, serde_json::Value) {
    let file_size = |name: &str| -> u64 {
        std::fs::metadata(data_dir.join(name))
            .map(|m| m.len())
            .unwrap_or(0)
    };
    let (disk_total, disk_free) = {
        use sysinfo::Disks;
        let disks = Disks::new_with_refreshed_list();
        let data_str = data_dir.to_string_lossy().to_string();
        let data_path = std::path::Path::new(&data_str);
        disks
            .iter()
            .filter(|d| data_path.starts_with(d.mount_point()))
            .max_by_key(|d| d.mount_point().as_os_str().len())
            .map(|d| (d.total_space(), d.available_space()))
            .unwrap_or((0, 0))
    };
    let db_bytes = file_size("audio_haxor.db")
        + file_size("audio_haxor.db-wal")
        + file_size("audio_haxor.db-shm");
    let prefs_bytes = file_size("preferences.toml");
    let table_counts = db::global().table_counts().unwrap_or_default();
    (disk_total, disk_free, db_bytes, prefs_bytes, table_counts)
}

fn cached_slow_stats(data_dir: &std::path::Path) -> (u64, u64, u64, u64, serde_json::Value) {
    let now = Instant::now();
    let dir_key = data_dir.to_string_lossy().to_string();
    if let Ok(guard) = SLOW_STATS_CACHE.lock() {
        if let Some(s) = guard.as_ref() {
            if s.dir_key == dir_key && now.saturating_duration_since(s.at) < SLOW_STATS_TTL {
                return (
                    s.disk_total,
                    s.disk_free,
                    s.db_bytes,
                    s.prefs_bytes,
                    s.table_counts.clone(),
                );
            }
        }
    }
    let (disk_total, disk_free, db_bytes, prefs_bytes, table_counts) = compute_slow_stats(data_dir);
    if let Ok(mut guard) = SLOW_STATS_CACHE.lock() {
        *guard = Some(SlowStatsSnapshot {
            at: now,
            dir_key,
            disk_total,
            disk_free,
            db_bytes,
            prefs_bytes,
            table_counts: table_counts.clone(),
        });
    }
    (disk_total, disk_free, db_bytes, prefs_bytes, table_counts)
}

fn dotted_extensions_to_upper_tags(exts: &[&str]) -> Vec<String> {
    exts.iter()
        .map(|e| e.strip_prefix('.').unwrap_or(e).to_ascii_uppercase())
        .collect()
}

fn build_process_stats(app: AppHandle) -> serde_json::Value {
    let rss = get_rss_bytes();
    let virt = get_virtual_bytes();
    let threads = get_thread_count();
    let cpu_pct = get_cpu_percent();
    let rayon_threads = rayon::current_num_threads();
    let uptime_secs = get_uptime_secs();
    let pid = std::process::id();
    let open_fds = get_open_fd_count();
    let ncpus = num_cpus::get();

    // Scanner states
    let scan_state = app.state::<ScanState>();
    let update_state = app.state::<UpdateState>();
    let audio_state = app.state::<AudioScanState>();
    let daw_state = app.state::<DawScanState>();
    let preset_state = app.state::<PresetScanState>();
    let pdf_state = app.state::<PdfScanState>();
    let midi_state = app.state::<MidiScanState>();

    // Preferences for scanner config
    let prefs = history::load_preferences();
    let thread_mult = prefs
        .get("threadMultiplier")
        .and_then(|v| {
            v.as_str()
                .and_then(|s| s.parse::<usize>().ok())
                .or(v.as_u64().map(|n| n as usize))
        })
        .unwrap_or(4);
    let batch_size = prefs
        .get("batchSize")
        .and_then(|v| {
            v.as_str()
                .and_then(|s| s.parse::<usize>().ok())
                .or(v.as_u64().map(|n| n as usize))
        })
        .unwrap_or(100);
    let chan_buf = prefs
        .get("channelBuffer")
        .and_then(|v| {
            v.as_str()
                .and_then(|s| s.parse::<usize>().ok())
                .or(v.as_u64().map(|n| n as usize))
        })
        .unwrap_or(512);
    let flush_interval = prefs
        .get("flushInterval")
        .and_then(|v| {
            v.as_str()
                .and_then(|s| s.parse::<usize>().ok())
                .or(v.as_u64().map(|n| n as usize))
        })
        .unwrap_or(100);
    let page_size = prefs
        .get("pageSize")
        .and_then(|v| {
            v.as_str()
                .and_then(|s| s.parse::<usize>().ok())
                .or(v.as_u64().map(|n| n as usize))
        })
        .unwrap_or(200);

    let sqlite_read_pool_pref = prefs
        .get("sqliteReadPoolExtra")
        .map(|v| {
            v.as_str()
                .map(std::string::ToString::to_string)
                .unwrap_or_else(|| v.to_string())
        })
        .unwrap_or_else(|| "auto".to_string());

    let (sqlite_read_pool_extra, sqlite_read_pool_total) = if db::global_initialized() {
        let db = db::global();
        (
            db.sqlite_read_pool_extra_slots(),
            db.sqlite_read_pool_total_handles(),
        )
    } else {
        (0, 0)
    };

    let data_dir = history::get_data_dir();
    let (disk_total, disk_free, db_bytes, prefs_bytes, db_table_counts) =
        cached_slow_stats(&data_dir);

    // OS info
    let os_name = std::env::consts::OS;
    let os_arch = std::env::consts::ARCH;
    let hostname = gethostname();

    // FD limits
    #[cfg(unix)]
    let (fd_soft, fd_hard) = {
        let mut rlim = libc::rlimit {
            rlim_cur: 0,
            rlim_max: 0,
        };
        if unsafe { libc::getrlimit(libc::RLIMIT_NOFILE, &mut rlim) } == 0 {
            (rlim.rlim_cur, rlim.rlim_max)
        } else {
            (0, 0)
        }
    };
    #[cfg(not(unix))]
    let (fd_soft, fd_hard) = (0u64, 0u64);

    // Supported formats: audio → `audio_extensions`; DAW/preset → scanners; xref → `xref::XREF_SUPPORTED_EXTENSIONS`
    let plugin_formats = ["VST2", "VST3", "AU", "CLAP", "AAX"];
    let daw_formats = dotted_extensions_to_upper_tags(crate::daw_scanner::DAW_EXTENSIONS);
    let preset_formats = dotted_extensions_to_upper_tags(crate::preset_scanner::PRESET_EXTENSIONS);
    let xref_formats = dotted_extensions_to_upper_tags(crate::xref::XREF_SUPPORTED_EXTENSIONS);
    let midi_formats = ["MID", "MIDI"];
    let pdf_formats = ["PDF"];

    serde_json::json!({
        "pid": pid,
        "rssBytes": rss,
        "virtualBytes": virt,
        "threads": threads,
        "cpuPercent": cpu_pct,
        "rayonThreads": rayon_threads,
        "numCpus": ncpus,
        "uptimeSecs": uptime_secs,
        "openFds": open_fds,
        "fdSoftLimit": fd_soft,
        "fdHardLimit": fd_hard,
        "os": os_name,
        "arch": os_arch,
        "hostname": hostname,
        "appVersion": env!("CARGO_PKG_VERSION"),
        "tauriVersion": tauri::VERSION,
        "rustcTarget": option_env!("AUDIO_HAXOR_TARGET_TRIPLE").unwrap_or("unknown"),
        "buildProfile": if cfg!(debug_assertions) { "debug" } else { "release" },
        "diskTotalBytes": disk_total,
        "diskFreeBytes": disk_free,
        "app": {
            "audioFormats": crate::audio_extensions::audio_format_tags_for_app_info(),
            "pluginFormats": plugin_formats,
            "dawFormats": daw_formats,
            "presetFormats": preset_formats,
            "xrefFormats": xref_formats,
            "midiFormats": midi_formats,
            "pdfFormats": pdf_formats,
            "analysisEngines": ["BPM (autocorrelation)", "Key (Goertzel chromagram)", "LUFS (RMS dBFS)", "Fingerprint (spectral)"],
            "visualizers": ["FFT spectrum", "Waveform", "Spectrogram", "Stereo Lissajous", "Level meters", "Frequency bands"],
            "exportFormats": ["JSON", "TOML", "CSV", "TSV", "PDF"],
            "storageBackend": "SQLite (WAL mode)",
            "uiFramework": "Tauri v2 + vanilla JS",
            "searchEngine": "fzf-style fuzzy matching",
        },
        "scanner": {
            "pluginScanning": scan_state.scanning.load(Ordering::Relaxed),
            "pluginStopped": scan_state.stop_scan.load(Ordering::Relaxed),
            "updateChecking": update_state.checking.load(Ordering::Relaxed),
            "updateStopped": update_state.stop_updates.load(Ordering::Relaxed),
            "audioScanning": audio_state.scanning.load(Ordering::Relaxed),
            "audioStopped": audio_state.stop_scan.load(Ordering::Relaxed),
            "dawScanning": daw_state.scanning.load(Ordering::Relaxed),
            "dawStopped": daw_state.stop_scan.load(Ordering::Relaxed),
            "presetScanning": preset_state.scanning.load(Ordering::Relaxed),
            "presetStopped": preset_state.stop_scan.load(Ordering::Relaxed),
            "pdfScanning": pdf_state.scanning.load(Ordering::Relaxed),
            "pdfStopped": pdf_state.stop_scan.load(Ordering::Relaxed),
            "midiScanning": midi_state.scanning.load(Ordering::Relaxed),
            "midiStopped": midi_state.stop_scan.load(Ordering::Relaxed),
        },
        "config": {
            "threadMultiplier": thread_mult,
            "globalPoolSize": ncpus * thread_mult,
            "perScannerThreads": ncpus * 2,
            "batchSize": batch_size,
            "channelBuffer": chan_buf,
            "walkerChannelBuffer": 2048,
            "walkerBatchSize": 100,
            "flushInterval": flush_interval,
            "pageSize": page_size,
            "stackSize": "8 MB",
            "depthLimit": 50,
            "pluginChannelMin": 64,
            "pluginChannelMax": 8192,
        },
        "database": {
            "sizeBytes": db_bytes,
            "tables": db_table_counts,
            "sqliteReadPoolExtra": sqlite_read_pool_extra,
            "sqliteReadPoolTotal": sqlite_read_pool_total,
            "sqliteReadPoolExtraPref": sqlite_read_pool_pref,
        },
        "dataFiles": {
            "preferencesBytes": prefs_bytes,
        },
        "dataDir": data_dir.to_string_lossy(),
    })
}

#[tauri::command]
async fn get_process_stats(app: AppHandle) -> serde_json::Value {
    let app = app.clone();
    blocking(move || build_process_stats(app))
        .await
        .unwrap_or_else(|_| serde_json::json!({}))
}

#[tauri::command]
async fn list_data_files() -> Vec<serde_json::Value> {
    blocking(move || {
        let data_dir = history::get_data_dir();
        let mut files = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&data_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_file() {
                    continue;
                }
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                let meta = std::fs::metadata(&path).ok();
                let size = meta.as_ref().map(|m| m.len()).unwrap_or(0);
                let modified = meta
                    .and_then(|m| m.modified().ok())
                    .map(|t| {
                        let dt: chrono::DateTime<chrono::Utc> = t.into();
                        dt.format("%Y-%m-%d %H:%M:%S").to_string()
                    })
                    .unwrap_or_default();
                files.push(serde_json::json!({
                    "name": name,
                    "path": path.to_string_lossy(),
                    "size": size,
                    "sizeFormatted": format_size(size),
                    "modified": modified,
                }));
            }
        }
        files.sort_by(|a, b| {
            a["name"]
                .as_str()
                .unwrap_or("")
                .cmp(b["name"].as_str().unwrap_or(""))
        });
        files
    })
    .await
    .unwrap_or_default()
}

#[tauri::command]
async fn delete_data_file(name: String) -> Result<(), String> {
    blocking_res(move || {
        let path = history::get_data_dir().join(&name);
        if !path.exists() {
            return Ok(());
        }
        std::fs::remove_file(&path).map_err(|e| e.to_string())
    })
    .await
}

static APP_START: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();

fn get_uptime_secs() -> u64 {
    APP_START.get_or_init(Instant::now).elapsed().as_secs()
}

// ── Cross-platform process stats via sysinfo ──

fn get_process_info() -> (u64, u64, f32) {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Mutex, OnceLock};
    use sysinfo::{Pid, System};
    static SYS: OnceLock<Mutex<System>> = OnceLock::new();
    static PRIMED: AtomicBool = AtomicBool::new(false);
    let sys_mutex = SYS.get_or_init(|| Mutex::new(System::new()));
    let mut sys = sys_mutex.lock().unwrap();
    let pid = Pid::from_u32(std::process::id());
    // First call: prime with an initial refresh so cpu_usage() has a baseline
    if !PRIMED.swap(true, Ordering::Relaxed) {
        sys.refresh_processes(sysinfo::ProcessesToUpdate::Some(&[pid]), true);
        std::thread::sleep(std::time::Duration::from_millis(200));
    }
    sys.refresh_processes(sysinfo::ProcessesToUpdate::Some(&[pid]), true);
    if let Some(proc_info) = sys.process(pid) {
        (
            proc_info.memory(),
            proc_info.virtual_memory(),
            proc_info.cpu_usage(),
        )
    } else {
        (0, 0, 0.0)
    }
}

fn get_rss_bytes() -> u64 {
    get_process_info().0
}

fn get_virtual_bytes() -> u64 {
    get_process_info().1
}

fn get_thread_count() -> u32 {
    // Linux: `Process::tasks()` (per-thread PIDs). Never use `cpu_usage()` here (`f32`) — that
    // mismatch only surfaces on Linux targets. Other OSes use fallbacks below.
    #[cfg(target_os = "linux")]
    {
        use std::sync::{Mutex, OnceLock};
        use sysinfo::{Pid, System};
        static SYS: OnceLock<Mutex<System>> = OnceLock::new();
        let mut sys = SYS
            .get_or_init(|| Mutex::new(System::new()))
            .lock()
            .unwrap();
        let pid = Pid::from_u32(std::process::id());
        sys.refresh_processes(sysinfo::ProcessesToUpdate::Some(&[pid]), true);
        if let Some(p) = sys.process(pid) {
            if let Some(tasks) = p.tasks() {
                return (tasks.len() as u32).saturating_add(1);
            }
        }
    }
    #[cfg(target_os = "macos")]
    {
        let pid = std::process::id();
        if let Ok(out) = std::process::Command::new("ps")
            .args(["-M", "-p", &pid.to_string()])
            .output()
        {
            return String::from_utf8_lossy(&out.stdout)
                .lines()
                .count()
                .saturating_sub(1) as u32;
        }
    }
    0
}

/// Per-PID user+system CPU time in microseconds (same units as `libc::rusage` tv_sec/tv_usec combined).
/// Used so AudioEngine CPU% matches the header formula: `(Δuser + Δsys) / Δwall * 100`.
fn foreign_process_cpu_times_us(pid: u32) -> Option<(i64, i64)> {
    if pid == 0 {
        return None;
    }
    #[cfg(target_os = "linux")]
    {
        let path = format!("/proc/{pid}/stat");
        let line = std::fs::read_to_string(&path).ok()?;
        let idx = line.rfind(')')?;
        let rest = line[idx + 1..].trim_start();
        let fields: Vec<&str> = rest.split_whitespace().collect();
        if fields.len() < 13 {
            return None;
        }
        let utime_ticks: i64 = fields[11].parse().ok()?;
        let stime_ticks: i64 = fields[12].parse().ok()?;
        let clk = unsafe { libc::sysconf(libc::_SC_CLK_TCK) } as i64;
        if clk <= 0 {
            return None;
        }
        let user_us = utime_ticks * 1_000_000 / clk;
        let sys_us = stime_ticks * 1_000_000 / clk;
        return Some((user_us, sys_us));
    }
    #[cfg(target_os = "macos")]
    {
        // Must match `<sys/proc_info.h>` `struct proc_taskinfo` size for `proc_pidinfo`.
        #[repr(C)]
        struct ProcTaskInfo {
            virtual_size: u64,
            resident_size: u64,
            total_user: u64,
            total_system: u64,
            threads_user: u64,
            threads_system: u64,
            policy: u64,
            ssugg: u64,
            flags: u64,
        }
        #[link(name = "proc", kind = "dylib")]
        unsafe extern "C" {
            fn proc_pidinfo(
                pid: i32,
                flavor: i32,
                arg: u64,
                buffer: *mut ProcTaskInfo,
                buffersize: i32,
            ) -> i32;
        }
        const PROC_PIDTASKINFO: i32 = 4;
        let mut info: ProcTaskInfo = unsafe { std::mem::zeroed() };
        let n = unsafe {
            proc_pidinfo(
                pid as i32,
                PROC_PIDTASKINFO,
                0,
                &mut info,
                std::mem::size_of::<ProcTaskInfo>() as i32,
            )
        };
        if n <= 0 {
            return None;
        }
        // Nanoseconds → microseconds (same scale as `getrusage` path in `get_cpu_percent`).
        let user_us = (info.total_user / 1000) as i64;
        let sys_us = (info.total_system / 1000) as i64;
        return Some((user_us, sys_us));
    }
    #[cfg(target_os = "windows")]
    {
        use std::ffi::c_void;
        use std::mem::MaybeUninit;
        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn OpenProcess(dwDesiredAccess: u32, bInheritHandle: i32, dwProcessId: u32) -> *mut c_void;
            fn CloseHandle(h: *mut c_void) -> i32;
            fn GetProcessTimes(
                h: *mut c_void,
                creation: *mut [u32; 2],
                exit: *mut [u32; 2],
                kernel: *mut [u32; 2],
                user: *mut [u32; 2],
            ) -> i32;
        }
        const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;
        unsafe {
            let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
            if h.is_null() {
                return None;
            }
            let mut creation = MaybeUninit::<[u32; 2]>::uninit();
            let mut exit = MaybeUninit::<[u32; 2]>::uninit();
            let mut kernel = MaybeUninit::<[u32; 2]>::uninit();
            let mut user = MaybeUninit::<[u32; 2]>::uninit();
            let ok = GetProcessTimes(
                h,
                creation.as_mut_ptr(),
                exit.as_mut_ptr(),
                kernel.as_mut_ptr(),
                user.as_mut_ptr(),
            );
            let _ = CloseHandle(h);
            if ok == 0 {
                return None;
            }
            let ft_to_us = |ft: [u32; 2]| -> i64 {
                let ticks = (ft[1] as i64) << 32 | ft[0] as i64;
                ticks / 10
            };
            let user_us = ft_to_us(user.assume_init());
            let sys_us = ft_to_us(kernel.assume_init());
            return Some((user_us, sys_us));
        }
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        let _ = pid;
        None
    }
}

/// Same formula as [`get_cpu_percent`] (`getrusage` user+sys deltas vs wall clock), for another PID.
fn get_cpu_percent_like_rusage_for_pid(pid: u32) -> f64 {
    use std::sync::{Mutex, OnceLock};
    use std::time::Instant;

    struct CpuSample {
        wall: Instant,
        user_us: i64,
        sys_us: i64,
    }

    static PREV: OnceLock<Mutex<Option<(u32, CpuSample)>>> = OnceLock::new();
    let prev_lock = PREV.get_or_init(|| Mutex::new(None));

    if pid == 0 {
        let mut prev = prev_lock.lock().unwrap();
        *prev = None;
        return 0.0;
    }

    let Some((user_us, sys_us)) = foreign_process_cpu_times_us(pid) else {
        let mut prev = prev_lock.lock().unwrap();
        *prev = None;
        return 0.0;
    };

    let now = Instant::now();
    let mut prev_guard = prev_lock.lock().unwrap();
    let pct = match *prev_guard {
        Some((prev_pid, ref p)) if prev_pid == pid => {
            let wall_us = now.duration_since(p.wall).as_micros() as f64;
            if wall_us > 0.0 {
                let cpu_us = ((user_us - p.user_us) + (sys_us - p.sys_us)) as f64;
                (cpu_us / wall_us) * 100.0
            } else {
                0.0
            }
        }
        _ => 0.0,
    };
    *prev_guard = Some((
        pid,
        CpuSample {
            wall: now,
            user_us,
            sys_us,
        },
    ));
    pct
}

fn get_cpu_percent() -> f64 {
    use std::sync::{Mutex, OnceLock};
    use std::time::Instant;

    struct CpuSample {
        wall: Instant,
        user_us: i64,
        sys_us: i64,
    }

    static PREV: OnceLock<Mutex<Option<CpuSample>>> = OnceLock::new();
    let prev_lock = PREV.get_or_init(|| Mutex::new(None));

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        let mut usage: libc::rusage = unsafe { std::mem::zeroed() };
        let ret = unsafe { libc::getrusage(libc::RUSAGE_SELF, &mut usage) };
        if ret != 0 {
            return get_process_info().2 as f64;
        }

        let now = Instant::now();
        let user_us = usage.ru_utime.tv_sec as i64 * 1_000_000 + usage.ru_utime.tv_usec as i64;
        let sys_us = usage.ru_stime.tv_sec as i64 * 1_000_000 + usage.ru_stime.tv_usec as i64;

        let mut prev = prev_lock.lock().unwrap();
        let pct = if let Some(ref p) = *prev {
            let wall_us = now.duration_since(p.wall).as_micros() as f64;
            if wall_us > 0.0 {
                let cpu_us = ((user_us - p.user_us) + (sys_us - p.sys_us)) as f64;
                (cpu_us / wall_us) * 100.0
            } else {
                0.0
            }
        } else {
            0.0
        };
        *prev = Some(CpuSample {
            wall: now,
            user_us,
            sys_us,
        });
        pct
    }
    #[cfg(target_os = "windows")]
    {
        use std::ffi::c_void;
        use std::mem::MaybeUninit;
        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn GetCurrentProcess() -> *mut c_void;
            fn GetProcessTimes(
                h: *mut c_void,
                creation: *mut [u32; 2],
                exit: *mut [u32; 2],
                kernel: *mut [u32; 2],
                user: *mut [u32; 2],
            ) -> i32;
        }
        let mut creation = MaybeUninit::<[u32; 2]>::uninit();
        let mut exit = MaybeUninit::<[u32; 2]>::uninit();
        let mut kernel = MaybeUninit::<[u32; 2]>::uninit();
        let mut user = MaybeUninit::<[u32; 2]>::uninit();
        let ok = unsafe {
            GetProcessTimes(
                GetCurrentProcess(),
                creation.as_mut_ptr(),
                exit.as_mut_ptr(),
                kernel.as_mut_ptr(),
                user.as_mut_ptr(),
            )
        };
        if ok == 0 {
            return get_process_info().2 as f64;
        }
        let ft_to_us = |ft: [u32; 2]| -> i64 {
            let ticks = (ft[1] as i64) << 32 | ft[0] as i64; // 100ns ticks
            ticks / 10 // to microseconds
        };
        let now = Instant::now();
        let user_us = ft_to_us(unsafe { user.assume_init() });
        let sys_us = ft_to_us(unsafe { kernel.assume_init() });

        let mut prev = prev_lock.lock().unwrap();
        let pct = if let Some(ref p) = *prev {
            let wall_us = now.duration_since(p.wall).as_micros() as f64;
            if wall_us > 0.0 {
                let cpu_us = ((user_us - p.user_us) + (sys_us - p.sys_us)) as f64;
                (cpu_us / wall_us) * 100.0
            } else {
                0.0
            }
        } else {
            0.0
        };
        *prev = Some(CpuSample {
            wall: now,
            user_us,
            sys_us,
        });
        pct
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        get_process_info().2 as f64
    }
}

fn get_open_fd_count() -> u32 {
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        // /dev/fd on macOS, /proc/self/fd on Linux
        for dir in &["/dev/fd", "/proc/self/fd"] {
            if let Ok(entries) = std::fs::read_dir(dir) {
                return entries.count() as u32;
            }
        }
    }
    #[cfg(target_os = "windows")]
    {
        use std::ffi::c_void;
        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn GetCurrentProcess() -> *mut c_void;
            fn GetProcessHandleCount(h_process: *mut c_void, p_count: *mut u32) -> i32;
        }
        unsafe {
            let mut count = 0u32;
            if GetProcessHandleCount(GetCurrentProcess(), &mut count) != 0 {
                return count;
            }
        }
    }
    0
}

// ── AudioEngine subprocess stats (same probes as [`build_process_stats`] / header: sysinfo RSS/VIRT/CPU, FD count, threads) ──

#[cfg(not(target_os = "linux"))]
fn thread_count_for_pid_non_sysinfo(pid: u32) -> u32 {
    if pid == 0 {
        return 0;
    }
    #[cfg(target_os = "macos")]
    {
        if let Ok(out) = std::process::Command::new("ps")
            .args(["-M", "-p", &pid.to_string()])
            .output()
        {
            return String::from_utf8_lossy(&out.stdout)
                .lines()
                .count()
                .saturating_sub(1) as u32;
        }
    }
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        if let Ok(out) = std::process::Command::new("powershell")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                &format!("(Get-Process -Id {pid} -ErrorAction SilentlyContinue).Threads.Count"),
            ])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
        {
            if out.status.success() {
                if let Ok(n) = String::from_utf8_lossy(&out.stdout).trim().parse::<u32>() {
                    return n;
                }
            }
        }
    }
    0
}

fn open_fd_count_for_pid(pid: u32) -> u32 {
    if pid == 0 {
        return 0;
    }
    #[cfg(target_os = "linux")]
    {
        let path = format!("/proc/{pid}/fd");
        if let Ok(entries) = std::fs::read_dir(&path) {
            return entries.count() as u32;
        }
    }
    #[cfg(target_os = "macos")]
    {
        if let Ok(out) = std::process::Command::new("lsof")
            .args(["-w", "-p", &pid.to_string()])
            .output()
        {
            if !out.status.success() {
                return 0;
            }
            let stdout = String::from_utf8_lossy(&out.stdout);
            let lines: Vec<&str> = stdout
                .lines()
                .filter(|l| !l.trim().is_empty())
                .collect();
            if lines.is_empty() {
                return 0;
            }
            return lines.len().saturating_sub(1) as u32;
        }
    }
    #[cfg(target_os = "windows")]
    {
        use std::ffi::c_void;
        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn OpenProcess(dwDesiredAccess: u32, bInheritHandle: i32, dwProcessId: u32) -> *mut c_void;
            fn CloseHandle(h: *mut c_void) -> i32;
            fn GetProcessHandleCount(hProcess: *mut c_void, lpdwHandleCount: *mut u32) -> i32;
        }
        const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;
        unsafe {
            let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
            if h.is_null() {
                return 0;
            }
            let mut count = 0u32;
            let ok = GetProcessHandleCount(h, &mut count);
            let _ = CloseHandle(h);
            if ok != 0 {
                return count;
            }
        }
    }
    0
}

fn collect_audio_engine_process_metrics(pid: u32) -> (u64, u64, u64, u32) {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Mutex, OnceLock};
    use sysinfo::{Pid, ProcessesToUpdate, System};

    static SYS: OnceLock<Mutex<System>> = OnceLock::new();
    static PRIMED: AtomicBool = AtomicBool::new(false);
    static LAST_PID: Mutex<Option<u32>> = Mutex::new(None);

    if pid == 0 {
        return (0, 0, 0, 0);
    }

    let sys_mutex = SYS.get_or_init(|| Mutex::new(System::new()));
    let mut sys = sys_mutex.lock().unwrap();
    let spid = Pid::from_u32(pid);

    {
        let mut last = LAST_PID.lock().unwrap();
        if *last != Some(pid) {
            PRIMED.store(false, Ordering::Relaxed);
            *last = Some(pid);
        }
    }

    if !PRIMED.swap(true, Ordering::Relaxed) {
        sys.refresh_processes(ProcessesToUpdate::Some(&[spid]), true);
        std::thread::sleep(std::time::Duration::from_millis(200));
    }
    sys.refresh_processes(ProcessesToUpdate::Some(&[spid]), true);

    let Some(proc_info) = sys.process(spid) else {
        return (0, 0, 0, 0);
    };

    let rss = proc_info.memory();
    let virt = proc_info.virtual_memory();
    let run_time = proc_info.run_time();

    #[cfg(target_os = "linux")]
    {
        let threads = proc_info
            .tasks()
            .map(|t| (t.len() as u32).saturating_add(1))
            .unwrap_or(0);
        (rss, virt, run_time, threads)
    }

    #[cfg(not(target_os = "linux"))]
    {
        drop(sys);
        let threads = thread_count_for_pid_non_sysinfo(pid);
        (rss, virt, run_time, threads)
    }
}

fn build_audio_engine_process_stats() -> serde_json::Value {
    let pid = audio_engine::audio_engine_child_pid();
    let ncpus = num_cpus::get() as u32;
    if pid == 0 {
        return serde_json::json!({
            "running": false,
            "pid": 0u32,
            "numCpus": ncpus,
            "rssBytes": 0u64,
            "virtualBytes": 0u64,
            "cpuPercent": 0.0,
            "threads": 0u32,
            "openFds": 0u32,
            "uptimeSecs": 0u64,
        });
    }
    let (rss, virt, run_time, threads) = collect_audio_engine_process_metrics(pid);
    let fds = open_fd_count_for_pid(pid);
    let cpu_pct = get_cpu_percent_like_rusage_for_pid(pid);
    serde_json::json!({
        "running": true,
        "pid": pid,
        "numCpus": ncpus,
        "rssBytes": rss,
        "virtualBytes": virt,
        "cpuPercent": cpu_pct,
        "threads": threads,
        "openFds": fds,
        "uptimeSecs": run_time,
    })
}

#[tauri::command]
async fn get_audio_engine_process_stats() -> serde_json::Value {
    blocking(move || build_audio_engine_process_stats())
        .await
        .unwrap_or_else(|_| serde_json::json!({}))
}

fn gethostname() -> String {
    sysinfo::System::host_name().unwrap_or_default()
}

// ── PDF export/import ──

#[tauri::command]
async fn export_pdfs_json(pdfs: Vec<PdfFile>, file_path: String) -> Result<(), String> {
    blocking_res(move || {
        let json = serde_json::to_string_pretty(&pdfs).map_err(|e| e.to_string())?;
        std::fs::write(&file_path, json).map_err(|e| e.to_string())
    })
    .await
}

#[tauri::command]
async fn export_pdfs_dsv(pdfs: Vec<PdfFile>, file_path: String) -> Result<(), String> {
    blocking_res(move || {
        let sep = detect_separator(&file_path);
        let mut out = format!("Name{s}Path{s}Directory{s}Size{s}Modified\n", s = sep);
        for p in &pdfs {
            out.push_str(&format!(
                "{}{sep}{}{sep}{}{sep}{}{sep}{}\n",
                dsv_escape(&p.name, sep),
                dsv_escape(&p.path, sep),
                dsv_escape(&p.directory, sep),
                dsv_escape(&p.size_formatted, sep),
                dsv_escape(&p.modified, sep),
            ));
        }
        std::fs::write(&file_path, out).map_err(|e| e.to_string())
    })
    .await
}

// ── Preset export/import ──

#[tauri::command]
async fn export_presets_json(presets: Vec<PresetFile>, file_path: String) -> Result<(), String> {
    blocking_res(move || {
        let json = serde_json::to_string_pretty(&presets).map_err(|e| e.to_string())?;
        std::fs::write(&file_path, json).map_err(|e| e.to_string())
    })
    .await
}

#[tauri::command]
async fn export_presets_dsv(presets: Vec<PresetFile>, file_path: String) -> Result<(), String> {
    blocking_res(move || {
        let sep = detect_separator(&file_path);
        let mut out = format!(
            "Name{s}Format{s}Path{s}Directory{s}Size{s}Modified\n",
            s = sep
        );
        for p in &presets {
            out.push_str(&format!(
                "{}{sep}{}{sep}{}{sep}{}{sep}{}{sep}{}\n",
                dsv_escape(&p.name, sep),
                dsv_escape(&p.format, sep),
                dsv_escape(&p.path, sep),
                dsv_escape(&p.directory, sep),
                dsv_escape(&p.size_formatted, sep),
                dsv_escape(&p.modified, sep),
            ));
        }
        std::fs::write(&file_path, out).map_err(|e| e.to_string())
    })
    .await
}

// ── TOML export (generic — works for all types via serde) ──

#[tauri::command]
async fn export_toml(data: serde_json::Value, file_path: String) -> Result<(), String> {
    blocking_res(move || {
        let toml_str = toml::to_string_pretty(&data).map_err(|e| e.to_string())?;
        std::fs::write(&file_path, toml_str).map_err(|e| e.to_string())
    })
    .await
}

#[tauri::command]
async fn import_toml(file_path: String) -> Result<serde_json::Value, String> {
    blocking_res(move || {
        let data = std::fs::read_to_string(&file_path).map_err(|e| e.to_string())?;
        let val: toml::Value = toml::from_str(&data).map_err(|e| e.to_string())?;
        let json_str = serde_json::to_string(&val).map_err(|e| e.to_string())?;
        serde_json::from_str(&json_str).map_err(|e| e.to_string())
    })
    .await
}

// ── PDF export (printpdf 0.9 — Op stream + PdfPage) ──

fn export_pdf_impl(
    title: String,
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
    file_path: String,
) -> Result<(), String> {
    #[cfg(not(test))]
    append_log(format!(
        "EXPORT PDF — \"{}\" | {} rows | {} columns → {}",
        title,
        rows.len(),
        headers.len(),
        file_path
    ));
    use printpdf::*;

    let icon_bytes: &[u8] = include_bytes!("../icons/32x32.png");

    let page_w_mm = Mm(297.0); // A4 landscape
    let page_h_mm = Mm(210.0);
    let page_w = page_w_mm.0;
    let page_h = page_h_mm.0;
    let margin_x = 10.0_f32;
    let margin_bottom = 12.0_f32;
    let row_height = 4.5_f32;
    let header_row_h = 7.0_f32;
    let col_count = headers.len();
    let usable_w = page_w - margin_x * 2.0;

    const MAX_PDF_ROWS: usize = 10_000;
    let total_row_count = rows.len();
    let capped = total_row_count > MAX_PDF_ROWS;
    let export_rows = if capped {
        &rows[..MAX_PDF_ROWS]
    } else {
        &rows[..]
    };

    let col_widths: Vec<f32> = if col_count > 0 {
        let sample_step = (export_rows.len() / 500).max(1);
        let mut col_maxes: Vec<usize> = headers.iter().map(|h| h.len()).collect();
        let mut col_sums: Vec<usize> = vec![0; col_count];
        let mut sample_count = 0_usize;
        for (idx, row) in export_rows.iter().enumerate() {
            if idx % sample_step != 0 {
                continue;
            }
            sample_count += 1;
            for (i, cell) in row.iter().enumerate() {
                if i < col_count {
                    let l = cell.len().min(120);
                    if l > col_maxes[i] {
                        col_maxes[i] = l;
                    }
                    col_sums[i] += l;
                }
            }
        }
        let effective: Vec<usize> = col_sums
            .iter()
            .enumerate()
            .map(|(i, &s)| {
                let avg = if sample_count > 0 {
                    s / sample_count
                } else {
                    6
                };
                let p90_approx = (avg as f32 * 1.3) as usize;
                p90_approx
                    .max(headers[i].len() * 2)
                    .max(6)
                    .min(col_maxes[i])
            })
            .collect();
        let total_len: usize = effective.iter().sum::<usize>().max(1);
        let min_col = 12.0_f32;
        let mut widths: Vec<f32> = effective
            .iter()
            .map(|&l| (l as f32 / total_len as f32 * usable_w).max(min_col))
            .collect();
        let sum: f32 = widths.iter().sum();
        let scale = usable_w / sum;
        for w in &mut widths {
            *w *= scale;
        }
        widths
    } else {
        vec![usable_w]
    };
    let version = env!("CARGO_PKG_VERSION");

    fn rgb_color(r: f32, g: f32, b: f32) -> Color {
        Color::Rgb(Rgb::new(r, g, b, None))
    }

    fn push_fill_rect(
        ops: &mut Vec<Op>,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        r: f32,
        g: f32,
        b: f32,
    ) {
        ops.push(Op::SetFillColor {
            col: rgb_color(r, g, b),
        });
        let lp = |x: f32, y: f32| LinePoint {
            p: Point::new(Mm(x), Mm(y)),
            bezier: false,
        };
        ops.push(Op::DrawPolygon {
            polygon: Polygon {
                rings: vec![PolygonRing {
                    points: vec![
                        lp(x, y),
                        lp(x + w, y),
                        lp(x + w, y + h),
                        lp(x, y + h),
                    ],
                }],
                mode: PaintMode::Fill,
                winding_order: WindingOrder::NonZero,
            },
        });
    }

    fn push_stroke_line(
        ops: &mut Vec<Op>,
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        r: f32,
        g: f32,
        b: f32,
        thick_pt: f32,
    ) {
        ops.push(Op::SetOutlineColor {
            col: rgb_color(r, g, b),
        });
        ops.push(Op::SetOutlineThickness {
            pt: Pt(thick_pt),
        });
        ops.push(Op::DrawLine {
            line: Line {
                points: vec![
                    LinePoint {
                        p: Point::new(Mm(x1), Mm(y1)),
                        bezier: false,
                    },
                    LinePoint {
                        p: Point::new(Mm(x2), Mm(y2)),
                        bezier: false,
                    },
                ],
                is_closed: false,
            },
        });
    }

    fn push_text(
        ops: &mut Vec<Op>,
        text: String,
        font: BuiltinFont,
        size_pt: f32,
        x_mm: f32,
        y_mm: f32,
        color: Color,
    ) {
        ops.push(Op::StartTextSection);
        ops.push(Op::SetTextCursor {
            pos: Point::new(Mm(x_mm), Mm(y_mm)),
        });
        ops.push(Op::SetFont {
            font: PdfFontHandle::Builtin(font),
            size: Pt(size_pt),
        });
        ops.push(Op::SetLineHeight { lh: Pt(size_pt) });
        ops.push(Op::SetFillColor { col: color });
        ops.push(Op::ShowText {
            items: vec![TextItem::Text(text)],
        });
        ops.push(Op::EndTextSection);
    }

    let mut doc = PdfDocument::new(title.as_str());
    let mut decode_warnings = Vec::new();
    let icon_info: Option<(XObjectId, f32, f32)> =
        match RawImage::decode_from_bytes(icon_bytes, &mut decode_warnings) {
            Ok(img) => {
                let iw = img.width as f32;
                let ih = img.height as f32;
                let id = doc.add_image(&img);
                Some((id, iw, ih))
            }
            Err(_) => None,
        };

    let mut pages: Vec<PdfPage> = Vec::new();
    let mut ops: Vec<Op> = Vec::new();

    let render_header = |ops: &mut Vec<Op>, y: &mut f32, page: usize| {
        push_fill_rect(ops, 0.0, page_h - 22.0, page_w, 22.0, 0.02, 0.02, 0.04);

        let mut icon_offset = 0.0_f32;
        if let Some((ref id, iw, ih)) = icon_info {
            let icon_size = 6.0_f32;
            ops.push(Op::UseXobject {
                id: id.clone(),
                transform: XObjectTransform {
                    translate_x: Some(Mm(margin_x).into_pt()),
                    translate_y: Some(Mm(page_h - 19.0).into_pt()),
                    scale_x: Some(icon_size / iw),
                    scale_y: Some(icon_size / ih),
                    dpi: Some(300.0),
                    ..Default::default()
                },
            });
            icon_offset = icon_size + 2.0;
        }

        push_text(
            ops,
            "AUDIO_HAXOR".to_string(),
            BuiltinFont::HelveticaBold,
            14.0,
            margin_x + icon_offset,
            page_h - 14.0,
            rgb_color(0.02, 0.85, 0.91),
        );
        push_text(
            ops,
            format!("v{}", version),
            BuiltinFont::Helvetica,
            8.0,
            margin_x + icon_offset + 58.0,
            page_h - 14.0,
            rgb_color(1.0, 1.0, 1.0),
        );
        push_text(
            ops,
            title.clone(),
            BuiltinFont::HelveticaBold,
            12.0,
            page_w - margin_x - 80.0,
            page_h - 14.0,
            rgb_color(1.0, 1.0, 1.0),
        );

        push_stroke_line(
            ops,
            0.0,
            page_h - 22.0,
            page_w,
            page_h - 22.0,
            0.02,
            0.85,
            0.91,
            1.5,
        );

        *y = page_h - 28.0;

        if page == 1 {
            let sub = if capped {
                format!(
                    "Showing {} of {} items (capped)  |  Exported {}  |  by MenkeTechnologies",
                    export_rows.len(),
                    total_row_count,
                    chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
                )
            } else {
                format!(
                    "{} items  |  Exported {}  |  by MenkeTechnologies",
                    total_row_count,
                    chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
                )
            };
            push_text(
                ops,
                sub,
                BuiltinFont::HelveticaOblique,
                8.0,
                margin_x,
                *y,
                rgb_color(0.4, 0.4, 0.45),
            );
            *y -= 6.0;
        }
    };

    let render_col_headers = |ops: &mut Vec<Op>, y: &mut f32| {
        push_fill_rect(
            ops,
            margin_x - 1.0,
            *y - 1.5,
            usable_w + 2.0,
            header_row_h,
            0.04,
            0.04,
            0.08,
        );
        push_stroke_line(
            ops,
            margin_x - 1.0,
            *y - 1.5,
            margin_x + usable_w + 1.0,
            *y - 1.5,
            0.02,
            0.85,
            0.91,
            0.5,
        );

        let mut x = margin_x + 1.0;
        for (i, h) in headers.iter().enumerate() {
            push_text(
                ops,
                h.clone(),
                BuiltinFont::HelveticaBold,
                9.0,
                x,
                *y,
                rgb_color(0.02, 0.85, 0.91),
            );
            x += col_widths[i];
        }
        *y -= header_row_h;
    };

    let render_footer = |ops: &mut Vec<Op>, page: usize| {
        let footer_y = 8.0;
        push_fill_rect(
            ops,
            0.0,
            0.0,
            page_w,
            footer_y + 4.0,
            0.02,
            0.02,
            0.04,
        );
        push_stroke_line(
            ops,
            margin_x,
            footer_y + 3.0,
            page_w - margin_x,
            footer_y + 3.0,
            0.02,
            0.85,
            0.91,
            0.5,
        );
        push_text(
            ops,
            format!("AUDIO_HAXOR v{} — {}", version, title),
            BuiltinFont::Helvetica,
            7.0,
            margin_x,
            footer_y,
            rgb_color(0.4, 0.4, 0.45),
        );
        push_text(
            ops,
            format!("Page {}", page),
            BuiltinFont::Helvetica,
            7.0,
            page_w - margin_x - 25.0,
            footer_y,
            rgb_color(0.4, 0.4, 0.45),
        );
    };

    let mut y = 0.0_f32;
    let mut page_num = 1_usize;
    let mut row_idx = 0_usize;

    render_header(&mut ops, &mut y, page_num);
    render_col_headers(&mut ops, &mut y);
    y -= 1.0;

    for row in export_rows {
        if y < margin_bottom + 5.0 {
            render_footer(&mut ops, page_num);
            pages.push(PdfPage::new(page_w_mm, page_h_mm, std::mem::take(&mut ops)));
            page_num += 1;
            y = 0.0;
            render_header(&mut ops, &mut y, page_num);
            render_col_headers(&mut ops, &mut y);
            y -= 1.0;
            row_idx = 0;
        }

        if row_idx == 0 {
            push_fill_rect(&mut ops, 0.0, 0.0, page_w, y + 2.0, 0.03, 0.03, 0.06);
        }
        if row_idx % 2 == 1 {
            push_fill_rect(
                &mut ops,
                margin_x - 1.0,
                y - 1.2,
                usable_w + 2.0,
                row_height,
                0.06,
                0.06,
                0.10,
            );
        } else {
            push_fill_rect(
                &mut ops,
                margin_x - 1.0,
                y - 1.2,
                usable_w + 2.0,
                row_height,
                0.04,
                0.04,
                0.08,
            );
        }

        let mut x = margin_x + 0.5;
        for (i, cell) in row.iter().enumerate() {
            let w = if i < col_widths.len() {
                col_widths[i]
            } else {
                30.0
            };
            let max_chars = (w / 1.2) as usize;
            let cell_text = if cell.len() > max_chars && max_chars > 3 {
                format!("{}...", &cell[..max_chars - 3])
            } else {
                cell.clone()
            };
            push_text(
                &mut ops,
                cell_text,
                BuiltinFont::Helvetica,
                7.0,
                x,
                y,
                rgb_color(0.85, 0.85, 0.90),
            );
            x += w;
        }

        y -= row_height;
        row_idx += 1;
    }

    if capped {
        y -= 3.0;
        push_text(
            &mut ops,
            format!(
                "Export capped at {} of {} rows. Use CSV/JSON for the full dataset.",
                MAX_PDF_ROWS, total_row_count
            ),
            BuiltinFont::HelveticaBold,
            8.0,
            margin_x,
            y,
            rgb_color(0.83, 0.0, 0.77),
        );
    }

    render_footer(&mut ops, page_num);
    pages.push(PdfPage::new(page_w_mm, page_h_mm, ops));

    doc.with_pages(pages);
    let bytes = doc.save(&PdfSaveOptions::default(), &mut decode_warnings);
    std::fs::write(&file_path, bytes).map_err(|e| e.to_string())
}

#[tauri::command]
async fn export_pdf(
    title: String,
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
    file_path: String,
) -> Result<(), String> {
    blocking_res(move || export_pdf_impl(title, headers, rows, file_path)).await
}

// ── File browser ──

#[tauri::command]
async fn fs_list_dir(dir_path: String) -> Result<serde_json::Value, String> {
    blocking_res(move || {
        let path = std::path::Path::new(&dir_path);
        if !path.exists() {
            return Err(format!("Directory not found: {}", dir_path));
        }
        if !path.is_dir() {
            return Err(format!("Not a directory: {}", dir_path));
        }

        let mut entries = Vec::new();
        let read = std::fs::read_dir(path).map_err(|e| e.to_string())?;
        for entry in read.flatten() {
            let ep = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') {
                continue;
            }
            let is_dir = ep.is_dir();
            let meta = std::fs::metadata(&ep).ok();
            let size = meta.as_ref().map(|m| m.len()).unwrap_or(0);
            let modified = meta
                .and_then(|m| m.modified().ok())
                .map(|t| {
                    let dt: chrono::DateTime<chrono::Utc> = t.into();
                    dt.format("%Y-%m-%d %H:%M").to_string()
                })
                .unwrap_or_default();
            let ext = ep
                .extension()
                .map(|e| e.to_string_lossy().to_lowercase())
                .unwrap_or_default();
            entries.push(serde_json::json!({
                "name": name,
                "path": ep.to_string_lossy(),
                "isDir": is_dir,
                "size": size,
                "sizeFormatted": scanner::format_size(size),
                "modified": modified,
                "ext": ext,
            }));
        }
        entries.sort_by(|a, b| {
            let a_dir = a["isDir"].as_bool().unwrap_or(false);
            let b_dir = b["isDir"].as_bool().unwrap_or(false);
            b_dir.cmp(&a_dir).then_with(|| {
                a["name"]
                    .as_str()
                    .unwrap_or("")
                    .to_lowercase()
                    .cmp(&b["name"].as_str().unwrap_or("").to_lowercase())
            })
        });
        Ok(serde_json::json!({ "entries": entries, "path": dir_path }))
    })
    .await
}

#[tauri::command]
async fn delete_file(file_path: String) -> Result<(), String> {
    blocking_res(move || {
        #[cfg(not(test))]
        append_log(format!("FILE DELETE — {}", file_path));
        let path = std::path::Path::new(&file_path);
        if !path.exists() {
            return Err("File not found".into());
        }
        if path.is_dir() {
            std::fs::remove_dir_all(path).map_err(|e| e.to_string())
        } else {
            std::fs::remove_file(path).map_err(|e| e.to_string())
        }
    })
    .await
}

#[tauri::command]
async fn rename_file(old_path: String, new_path: String) -> Result<(), String> {
    blocking_res(move || {
        #[cfg(not(test))]
        append_log(format!("FILE RENAME — {} → {}", old_path, new_path));
        std::fs::rename(&old_path, &new_path).map_err(|e| e.to_string())
    })
    .await
}

#[tauri::command]
async fn write_text_file(file_path: String, contents: String) -> Result<(), String> {
    blocking_res(move || std::fs::write(&file_path, &contents).map_err(|e| e.to_string())).await
}

#[tauri::command]
async fn read_text_file(file_path: String) -> Result<String, String> {
    blocking_res(move || std::fs::read_to_string(&file_path).map_err(|e| e.to_string())).await
}

#[tauri::command]
async fn get_home_dir() -> Result<String, String> {
    blocking_res(|| {
        dirs::home_dir()
            .map(|p| p.to_string_lossy().to_string())
            .ok_or_else(|| "Could not determine home directory".into())
    })
    .await
}

#[tauri::command]
async fn import_presets_json(file_path: String) -> Result<Vec<PresetFile>, String> {
    blocking_res(move || {
        let data = std::fs::read_to_string(&file_path).map_err(|e| e.to_string())?;
        if let Ok(arr) = serde_json::from_str::<Vec<PresetFile>>(&data) {
            return Ok(arr);
        }
        let val: serde_json::Value = serde_json::from_str(&data).map_err(|e| e.to_string())?;
        if let Some(arr) = val.get("presets") {
            return serde_json::from_value(arr.clone()).map_err(|e| e.to_string());
        }
        Err("Expected a JSON array of presets or { \"presets\": [...] }".into())
    })
    .await
}

#[tauri::command]
async fn import_pdfs_json(file_path: String) -> Result<Vec<PdfFile>, String> {
    blocking_res(move || {
        let data = std::fs::read_to_string(&file_path).map_err(|e| e.to_string())?;
        if let Ok(arr) = serde_json::from_str::<Vec<PdfFile>>(&data) {
            return Ok(arr);
        }
        let val: serde_json::Value = serde_json::from_str(&data).map_err(|e| e.to_string())?;
        if let Some(arr) = val.get("pdfs") {
            return serde_json::from_value(arr.clone()).map_err(|e| e.to_string());
        }
        Err("Expected a JSON array of PDFs or { \"pdfs\": [...] }".into())
    })
    .await
}

#[tauri::command]
async fn import_audio_json(file_path: String) -> Result<Vec<AudioSample>, String> {
    blocking_res(move || {
        let data = std::fs::read_to_string(&file_path).map_err(|e| e.to_string())?;
        if let Ok(arr) = serde_json::from_str::<Vec<AudioSample>>(&data) {
            return Ok(arr);
        }
        let val: serde_json::Value = serde_json::from_str(&data).map_err(|e| e.to_string())?;
        if let Some(arr) = val.get("samples") {
            return serde_json::from_value(arr.clone()).map_err(|e| e.to_string());
        }
        Err("Expected a JSON array of samples or { \"samples\": [...] }".into())
    })
    .await
}

#[tauri::command]
async fn import_daw_json(file_path: String) -> Result<Vec<DawProject>, String> {
    blocking_res(move || {
        let data = std::fs::read_to_string(&file_path).map_err(|e| e.to_string())?;
        if let Ok(arr) = serde_json::from_str::<Vec<DawProject>>(&data) {
            return Ok(arr);
        }
        let val: serde_json::Value = serde_json::from_str(&data).map_err(|e| e.to_string())?;
        if let Some(arr) = val.get("projects") {
            return serde_json::from_value(arr.clone()).map_err(|e| e.to_string());
        }
        Err("Expected a JSON array of projects or { \"projects\": [...] }".into())
    })
    .await
}

// ── Tests ──

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::Mutex;

    /// Serialize tests that read/write `app.log` (parallel test runs would race otherwise).
    static APP_LOG_TEST_LOCK: Mutex<()> = Mutex::new(());

    /// `history::set_test_data_dir_path` uses a process-global override; parallel tests would
    /// overwrite each other. Also `read_log` uses `spawn_blocking` — worker threads do not see
    /// thread-local test dirs, so only one test may set the global override at a time.
    static TEST_DATA_DIR_SERIAL: Mutex<()> = Mutex::new(());

    fn app_log_lock() -> std::sync::MutexGuard<'static, ()> {
        APP_LOG_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Run async `#[tauri::command]` handlers from sync `#[test]` (Tokio runtime).
    fn rt_block_on<F: std::future::Future>(f: F) -> F::Output {
        tokio::runtime::Runtime::new()
            .expect("tokio runtime for lib.rs tests")
            .block_on(f)
    }

    /// Isolated temp data dir for tests that call `set_test_data_dir_path`; cleared on drop.
    /// Holds [`TEST_DATA_DIR_SERIAL`] so no other test can clobber the global override mid-case.
    struct TestDataDirGuard {
        path: std::path::PathBuf,
        _serial: std::sync::MutexGuard<'static, ()>,
    }
    impl Drop for TestDataDirGuard {
        fn drop(&mut self) {
            history::clear_test_data_dir_path();
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn test_data_dir() -> TestDataDirGuard {
        let serial = TEST_DATA_DIR_SERIAL
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let tmp = std::env::temp_dir().join(format!(
            "ah_data_test_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        history::set_test_data_dir_path(tmp.clone());
        TestDataDirGuard {
            path: tmp,
            _serial: serial,
        }
    }

    fn make_plugin(name: &str, plugin_type: &str) -> PluginInfo {
        PluginInfo {
            name: name.into(),
            path: format!("/lib/{}.vst3", name),
            plugin_type: plugin_type.into(),
            version: "1.0.0".into(),
            manufacturer: "TestCo".into(),
            manufacturer_url: Some("https://testco.com".into()),
            size: "2.5 MB".into(),
            size_bytes: 2621440,
            modified: "2025-01-01".into(),
            architectures: vec!["ARM64".into(), "x86_64".into()],
        }
    }

    #[test]
    fn test_csv_escape_plain() {
        assert_eq!(csv_escape("hello"), "hello");
    }

    #[test]
    fn test_csv_escape_comma() {
        assert_eq!(csv_escape("a,b"), "\"a,b\"");
    }

    #[test]
    fn test_csv_escape_quotes() {
        assert_eq!(csv_escape("say \"hi\""), "\"say \"\"hi\"\"\"");
    }

    #[test]
    fn test_csv_escape_newline() {
        assert_eq!(csv_escape("line1\nline2"), "\"line1\nline2\"");
    }

    #[test]
    fn test_csv_escape_empty() {
        assert_eq!(csv_escape(""), "");
    }

    #[test]
    fn test_csv_escape_comma_and_quotes() {
        assert_eq!(csv_escape("a,\"b\""), "\"a,\"\"b\"\"\"");
    }

    #[test]
    fn test_format_size_shared_tb() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(1024_u64.pow(4)), "1.0 TB");
        // Above max unit index: clamp to TB (e.g. 1 PiB → 1024.0 TB)
        assert_eq!(format_size(1024_u64.pow(5)), "1024.0 TB");
    }

    #[test]
    fn test_format_size_fractional_kb() {
        assert_eq!(format_size(2048 + 512), "2.5 KB");
    }

    #[test]
    fn test_format_size_single_byte_and_sub_kb() {
        assert_eq!(format_size(1), "1.0 B");
        assert_eq!(format_size(1023), "1023.0 B");
    }

    #[test]
    fn test_format_size_mb_boundary() {
        assert_eq!(format_size(1024 * 1024), "1.0 MB");
        assert_eq!(format_size(1024 * 1024 + 512 * 1024), "1.5 MB");
    }

    #[test]
    fn test_dsv_escape_tab_in_field() {
        assert_eq!(dsv_escape("a\tb", ','), "a\tb");
        assert_eq!(dsv_escape("a\tb", '\t'), "\"a\tb\"");
    }

    #[test]
    fn test_dsv_escape_semicolon_field_when_sep_is_semicolon() {
        assert_eq!(dsv_escape("a;b", ';'), "\"a;b\"");
        assert_eq!(dsv_escape("plain", ';'), "plain");
    }

    #[test]
    fn test_dsv_escape_quote_only() {
        assert_eq!(dsv_escape("\"", ','), "\"\"\"\"");
    }

    #[test]
    fn test_dsv_escape_newline_requires_quoting() {
        assert_eq!(
            dsv_escape("a\nb", ','),
            "\"a\nb\"",
            "embedded newline must quote for CSV/DSV"
        );
        assert_eq!(dsv_escape("line1\nline2", '\t'), "\"line1\nline2\"");
    }

    #[test]
    fn test_detect_separator() {
        assert_eq!(detect_separator("x.csv"), ',');
        assert_eq!(detect_separator("/path/to/out.tsv"), '\t');
        assert_eq!(detect_separator("nested/dir/report.csv"), ',');
        assert_eq!(detect_separator("sheet.tsv"), '\t');
    }

    #[test]
    fn test_read_zip_xml_returns_named_entry() {
        use std::io::Write;
        let tmp = std::env::temp_dir().join("upum_test_lib_read_zip_named.zip");
        let _ = fs::remove_file(&tmp);
        let file = fs::File::create(&tmp).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        zip.start_file::<_, ()>("notes.txt", Default::default())
            .unwrap();
        zip.write_all(b"noise").unwrap();
        zip.start_file::<_, ()>("project.xml", Default::default())
            .unwrap();
        zip.write_all(b"<Project>ok</Project>").unwrap();
        zip.finish().unwrap();

        let xml = read_zip_xml(tmp.to_str().unwrap(), &["project.xml"]).unwrap();
        assert_eq!(xml, "<Project>ok</Project>");
        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn test_read_zip_xml_fallback_scans_first_xml_member() {
        use std::io::Write;
        let tmp = std::env::temp_dir().join("upum_test_lib_read_zip_fallback.zip");
        let _ = fs::remove_file(&tmp);
        let file = fs::File::create(&tmp).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        zip.start_file::<_, ()>("nested/session.xml", Default::default())
            .unwrap();
        zip.write_all(b"<Session/>").unwrap();
        zip.finish().unwrap();

        let xml = read_zip_xml(tmp.to_str().unwrap(), &["project.xml"]).unwrap();
        assert_eq!(xml, "<Session/>");
        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn test_read_zip_xml_invalid_file_errors() {
        let tmp = std::env::temp_dir().join("upum_test_lib_not_zip.bin");
        let _ = fs::remove_file(&tmp);
        fs::write(&tmp, b"plain text not zip").unwrap();
        let err = read_zip_xml(tmp.to_str().unwrap(), &["a.xml"]).unwrap_err();
        assert!(
            err.contains("Not a valid ZIP") || err.contains("zip"),
            "unexpected err: {err}"
        );
        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn test_read_zip_xml_no_xml_member_errors() {
        use std::io::Write;
        let tmp = std::env::temp_dir().join("upum_test_lib_zip_no_xml.zip");
        let _ = fs::remove_file(&tmp);
        let file = fs::File::create(&tmp).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        zip.start_file::<_, ()>("readme.txt", Default::default())
            .unwrap();
        zip.write_all(b"hello").unwrap();
        zip.finish().unwrap();

        let err = read_zip_xml(tmp.to_str().unwrap(), &["missing.xml"]).unwrap_err();
        assert_eq!(err, "No XML found in archive");
        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn test_read_binary_project_inner_missing_file_errors() {
        assert!(read_binary_project_inner("/nonexistent/audio_haxor_binary_probe.bin").is_err());
    }

    #[test]
    fn test_read_binary_project_inner_extracts_printable_plugin_paths() {
        let tmp = std::env::temp_dir().join("upum_test_read_bin_inner.flp");
        let _ = fs::remove_file(&tmp);
        let mut blob = vec![0u8, 0x01, 0x02, 0x03];
        blob.extend_from_slice(b"/Library/Audio/Plug-Ins/VST3/PluginA.vst3");
        blob.push(0);
        blob.extend_from_slice(b"C:\\VSTPlugins\\PluginB.dll");
        blob.push(0);
        fs::write(&tmp, &blob).unwrap();
        let v = read_binary_project_inner(tmp.to_str().unwrap()).unwrap();
        let plugins: Vec<&str> = v["plugins"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|x| x.as_str())
            .collect();
        assert!(
            plugins.contains(&"/Library/Audio/Plug-Ins/VST3/PluginA.vst3"),
            "plugins={plugins:?}"
        );
        assert!(
            plugins.contains(&"C:\\VSTPlugins\\PluginB.dll"),
            "plugins={plugins:?}"
        );
        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn test_read_binary_project_adds_format_display_name() {
        let tmp = std::env::temp_dir().join("upum_test_read_bin_fmt.cpr");
        let _ = fs::remove_file(&tmp);
        fs::write(&tmp, b"x").unwrap();
        let v = read_binary_project(tmp.to_string_lossy().to_string(), "cpr").unwrap();
        assert_eq!(
            v.get("_format").and_then(|x| x.as_str()),
            Some("Cubase Project (.cpr)")
        );
        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn test_plugins_to_export_empty() {
        let result = plugins_to_export(&[]);
        assert!(result.is_empty());
    }

    #[test]
    fn test_plugins_to_export_preserves_fields() {
        let plugins = vec![make_plugin("Serum", "VST3")];
        let exported = plugins_to_export(&plugins);
        assert_eq!(exported.len(), 1);
        assert_eq!(exported[0].name, "Serum");
        assert_eq!(exported[0].plugin_type, "VST3");
        assert_eq!(exported[0].version, "1.0.0");
        assert_eq!(exported[0].manufacturer, "TestCo");
        assert_eq!(
            exported[0].manufacturer_url,
            Some("https://testco.com".into())
        );
    }

    #[test]
    fn test_plugins_to_export_no_url() {
        let mut p = make_plugin("NoUrl", "AU");
        p.manufacturer_url = None;
        let exported = plugins_to_export(&[p]);
        assert_eq!(exported[0].manufacturer_url, None);
    }

    #[test]
    fn test_export_import_json_roundtrip() {
        let tmp = std::env::temp_dir().join("upum_test_export_json.json");
        let _ = fs::remove_file(&tmp);

        let plugins = vec![make_plugin("PluginA", "VST3"), make_plugin("PluginB", "AU")];

        rt_block_on(export_plugins_json(
            plugins.clone(),
            tmp.to_string_lossy().to_string(),
        ))
        .unwrap();
        let imported = rt_block_on(import_plugins_json(tmp.to_string_lossy().to_string())).unwrap();

        assert_eq!(imported.len(), 2);
        assert_eq!(imported[0].name, "PluginA");
        assert_eq!(imported[0].plugin_type, "VST3");
        assert_eq!(imported[1].name, "PluginB");
        assert_eq!(imported[1].plugin_type, "AU");
        assert_eq!(imported[1].manufacturer, "TestCo");

        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn test_export_json_contains_metadata() {
        let tmp = std::env::temp_dir().join("upum_test_export_meta.json");
        let _ = fs::remove_file(&tmp);

        let plugins = vec![make_plugin("Test", "VST2")];
        rt_block_on(export_plugins_json(plugins, tmp.to_string_lossy().to_string())).unwrap();

        let content = fs::read_to_string(&tmp).unwrap();
        let payload: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert_eq!(payload["version"], env!("CARGO_PKG_VERSION"));
        assert!(payload["exported_at"].as_str().unwrap().contains("T"));
        assert_eq!(payload["plugins"].as_array().unwrap().len(), 1);

        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn test_export_csv_format() {
        let tmp = std::env::temp_dir().join("upum_test_export.csv");
        let _ = fs::remove_file(&tmp);

        let plugins = vec![make_plugin("Serum", "VST3")];
        rt_block_on(export_plugins_csv(plugins, tmp.to_string_lossy().to_string())).unwrap();

        let content = fs::read_to_string(&tmp).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(
            lines[0],
            "Name,Type,Version,Manufacturer,Manufacturer URL,Path,Size,Modified"
        );
        assert!(lines[1].starts_with("Serum,VST3,1.0.0,TestCo,"));

        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn test_export_csv_escapes_commas() {
        let tmp = std::env::temp_dir().join("upum_test_export_escape.csv");
        let _ = fs::remove_file(&tmp);

        let mut p = make_plugin("Plugin, With Comma", "VST3");
        p.manufacturer = "Company, Inc.".into();
        rt_block_on(export_plugins_csv(vec![p], tmp.to_string_lossy().to_string())).unwrap();

        let content = fs::read_to_string(&tmp).unwrap();
        assert!(content.contains("\"Plugin, With Comma\""));
        assert!(content.contains("\"Company, Inc.\""));

        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn test_export_plugins_tsv_uses_tab_separator_and_header() {
        let tmp = std::env::temp_dir().join("upum_test_export_plugins.tsv");
        let _ = fs::remove_file(&tmp);

        let plugins = vec![make_plugin("Serum", "VST3")];
        rt_block_on(export_plugins_csv(plugins, tmp.to_string_lossy().to_string())).unwrap();

        let content = fs::read_to_string(&tmp).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(
            lines[0],
            "Name\tType\tVersion\tManufacturer\tManufacturer URL\tPath\tSize\tModified"
        );
        assert!(
            !lines[1].contains(','),
            "TSV data row should use tabs, not commas: {}",
            lines[1]
        );
        assert!(lines[1].contains('\t'));

        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn test_import_plugins_json_errors_on_malformed_json() {
        let tmp = std::env::temp_dir().join("upum_test_import_plugins_bad.json");
        let _ = fs::remove_file(&tmp);
        fs::write(&tmp, "{ not json").unwrap();
        let err = rt_block_on(import_plugins_json(tmp.to_string_lossy().to_string())).unwrap_err();
        assert!(!err.is_empty());
        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn test_import_json_invalid_file() {
        let result = rt_block_on(import_plugins_json("/nonexistent/path.json".into()));
        assert!(result.is_err());
    }

    #[test]
    fn test_import_json_invalid_format() {
        let tmp = std::env::temp_dir().join("upum_test_import_bad.json");
        fs::write(&tmp, "not valid json").unwrap();

        let result = rt_block_on(import_plugins_json(tmp.to_string_lossy().to_string()));
        assert!(result.is_err());

        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn test_import_json_empty_plugins() {
        let tmp = std::env::temp_dir().join("upum_test_import_empty.json");
        let content = r#"{"version":"1.0","exported_at":"2025-01-01T00:00:00Z","plugins":[]}"#;
        fs::write(&tmp, content).unwrap();

        let result = rt_block_on(import_plugins_json(tmp.to_string_lossy().to_string())).unwrap();
        assert!(result.is_empty());

        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn test_import_plugins_json_errors_when_plugins_is_not_array() {
        let tmp = std::env::temp_dir().join("upum_test_import_plugins_wrong_type.json");
        let _ = fs::remove_file(&tmp);
        fs::write(
            &tmp,
            r#"{"version":"1.0","exported_at":"2025-01-01T00:00:00Z","plugins":"not-an-array"}"#,
        )
        .unwrap();
        let err = rt_block_on(import_plugins_json(tmp.to_string_lossy().to_string())).unwrap_err();
        assert!(!err.is_empty());
        let _ = fs::remove_file(&tmp);
    }

    /// Forward-compatible imports: serde ignores unknown keys on plugin objects.
    #[test]
    fn test_import_plugins_json_extra_keys_on_plugin_ignored() {
        let tmp = std::env::temp_dir().join("upum_test_import_plugins_extra_keys.json");
        let _ = fs::remove_file(&tmp);
        let content = r#"{
        "version":"1.0",
        "exported_at":"2025-01-01T00:00:00Z",
        "plugins":[{
            "name":"Extra",
            "type":"VST3",
            "version":"1",
            "manufacturer":"M",
            "path":"/p.vst3",
            "size":"1 B",
            "sizeBytes":1,
            "modified":"t",
            "architectures":[],
            "futureProofField":true
        }]
    }"#;
        fs::write(&tmp, content).unwrap();
        let imported = rt_block_on(import_plugins_json(tmp.to_string_lossy().to_string())).unwrap();
        assert_eq!(imported.len(), 1);
        assert_eq!(imported[0].name, "Extra");
        assert_eq!(imported[0].plugin_type, "VST3");
        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn test_export_csv_empty_plugins() {
        let tmp = std::env::temp_dir().join("upum_test_export_empty.csv");
        let _ = fs::remove_file(&tmp);

        rt_block_on(export_plugins_csv(vec![], tmp.to_string_lossy().to_string())).unwrap();
        let content = fs::read_to_string(&tmp).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 1); // header only
        assert!(lines[0].starts_with("Name,"));

        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn test_plugins_to_export_multiple() {
        let plugins = vec![
            make_plugin("A", "VST2"),
            make_plugin("B", "VST3"),
            make_plugin("C", "AU"),
        ];
        let exported = plugins_to_export(&plugins);
        assert_eq!(exported.len(), 3);
        assert_eq!(exported[0].name, "A");
        assert_eq!(exported[2].plugin_type, "AU");
    }

    #[test]
    fn test_export_payload_serde() {
        let payload = ExportPayload {
            version: "1.0".into(),
            exported_at: "2025-01-01T00:00:00Z".into(),
            plugins: vec![ExportPlugin {
                name: "Test".into(),
                plugin_type: "VST3".into(),
                version: "2.0".into(),
                manufacturer: "Co".into(),
                manufacturer_url: None,
                path: "/test".into(),
                size: "1 MB".into(),
                size_bytes: 1048576,
                modified: "2025-01-01".into(),
                architectures: vec![],
            }],
        };

        let json = serde_json::to_string(&payload).unwrap();
        let deserialized: ExportPayload = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.version, "1.0");
        assert_eq!(deserialized.plugins.len(), 1);
        assert_eq!(deserialized.plugins[0].name, "Test");
        assert!(deserialized.plugins[0].manufacturer_url.is_none());
    }

    #[test]
    fn test_export_plugin_skips_none_url_in_json() {
        let plugin = ExportPlugin {
            name: "Test".into(),
            plugin_type: "VST3".into(),
            version: "1.0".into(),
            manufacturer: "Co".into(),
            manufacturer_url: None,
            path: "/test".into(),
            size: "1 MB".into(),
            size_bytes: 0,
            modified: "2025-01-01".into(),
            architectures: vec![],
        };
        let json = serde_json::to_string(&plugin).unwrap();
        assert!(!json.contains("manufacturer_url"));
    }

    #[test]
    fn test_export_plugin_includes_url_in_json() {
        let plugin = ExportPlugin {
            name: "Test".into(),
            plugin_type: "VST3".into(),
            version: "1.0".into(),
            manufacturer: "Co".into(),
            manufacturer_url: Some("https://co.com".into()),
            path: "/test".into(),
            size: "1 MB".into(),
            size_bytes: 0,
            modified: "2025-01-01".into(),
            architectures: vec![],
        };
        let json = serde_json::to_string(&plugin).unwrap();
        assert!(json.contains("manufacturer_url"));
        assert!(json.contains("https://co.com"));
    }

    // ── Import/Export tests for all scan types ──

    fn make_audio_sample(name: &str, format: &str) -> AudioSample {
        AudioSample {
            name: name.into(),
            path: format!("/tmp/{}.{}", name, format.to_lowercase()),
            directory: "/tmp".into(),
            format: format.into(),
            size: 1024,
            size_formatted: "1.0 KB".into(),
            modified: "2025-01-01".into(),
            duration: None,
            channels: None,
            sample_rate: None,
            bits_per_sample: None,
        }
    }

    fn make_daw_project(name: &str, format: &str, daw: &str) -> DawProject {
        DawProject {
            name: name.into(),
            path: format!("/tmp/{}.{}", name, format.to_lowercase()),
            directory: "/tmp".into(),
            format: format.into(),
            daw: daw.into(),
            size: 2048,
            size_formatted: "2.0 KB".into(),
            modified: "2025-01-01".into(),
        }
    }

    fn make_preset(name: &str, format: &str) -> PresetFile {
        PresetFile {
            name: name.into(),
            path: format!("/tmp/{}.{}", name, format.to_lowercase()),
            directory: "/tmp".into(),
            format: format.into(),
            size: 512,
            size_formatted: "512 B".into(),
            modified: "2025-01-01".into(),
        }
    }

    #[test]
    fn test_import_audio_json_valid() {
        let tmp = std::env::temp_dir().join("upum_test_import_audio.json");
        let samples = vec![
            make_audio_sample("kick", "WAV"),
            make_audio_sample("snare", "FLAC"),
        ];
        let json = serde_json::to_string_pretty(&samples).unwrap();
        fs::write(&tmp, &json).unwrap();

        let result = rt_block_on(import_audio_json(tmp.to_string_lossy().to_string()));
        assert!(result.is_ok());
        let imported = result.unwrap();
        assert_eq!(imported.len(), 2);
        assert_eq!(imported[0].name, "kick");
        assert_eq!(imported[1].format, "FLAC");

        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn test_import_audio_json_extra_field_on_sample_ignored() {
        let tmp = std::env::temp_dir().join("upum_test_import_audio_extra_field.json");
        let _ = fs::remove_file(&tmp);
        let content = r#"{
        "version":"1.0",
        "exported_at":"2025-01-01T00:00:00Z",
        "samples":[{
            "name":"Extra",
            "path":"/a.wav",
            "directory":"/d",
            "format":"WAV",
            "size":100,
            "sizeFormatted":"100 B",
            "modified":"t",
            "futureProof":true
        }]
    }"#;
        fs::write(&tmp, content).unwrap();
        let imported = rt_block_on(import_audio_json(tmp.to_string_lossy().to_string())).unwrap();
        assert_eq!(imported.len(), 1);
        assert_eq!(imported[0].name, "Extra");
        assert_eq!(imported[0].format, "WAV");
        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn test_import_audio_json_invalid_format() {
        let tmp = std::env::temp_dir().join("upum_test_import_audio_bad.json");
        fs::write(&tmp, r#"{"not": "an array"}"#).unwrap();

        let result = rt_block_on(import_audio_json(tmp.to_string_lossy().to_string()));
        assert!(result.is_err());

        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn test_import_audio_json_nonexistent() {
        let result = rt_block_on(import_audio_json("/tmp/nonexistent_audio_file.json".into()));
        assert!(result.is_err());
    }

    #[test]
    fn test_import_daw_json_valid() {
        let tmp = std::env::temp_dir().join("upum_test_import_daw.json");
        let projects = vec![
            make_daw_project("Song1", "ALS", "Ableton Live"),
            make_daw_project("Song2", "FLP", "FL Studio"),
        ];
        let json = serde_json::to_string_pretty(&projects).unwrap();
        fs::write(&tmp, &json).unwrap();

        let result = rt_block_on(import_daw_json(tmp.to_string_lossy().to_string()));
        assert!(result.is_ok());
        let imported = result.unwrap();
        assert_eq!(imported.len(), 2);
        assert_eq!(imported[0].daw, "Ableton Live");
        assert_eq!(imported[1].format, "FLP");

        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn test_import_daw_json_extra_field_on_project_ignored() {
        let tmp = std::env::temp_dir().join("upum_test_import_daw_extra_field.json");
        let _ = fs::remove_file(&tmp);
        let content = r#"{
        "version":"1.0",
        "exported_at":"2025-01-01T00:00:00Z",
        "projects":[{
            "name":"Extra",
            "path":"/p.als",
            "directory":"/tmp",
            "format":"ALS",
            "daw":"Ableton Live",
            "size":2048,
            "sizeFormatted":"2.0 KB",
            "modified":"t",
            "futureProof":true
        }]
    }"#;
        fs::write(&tmp, content).unwrap();
        let imported = rt_block_on(import_daw_json(tmp.to_string_lossy().to_string())).unwrap();
        assert_eq!(imported.len(), 1);
        assert_eq!(imported[0].name, "Extra");
        assert_eq!(imported[0].daw, "Ableton Live");
        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn test_import_daw_json_invalid_format() {
        let tmp = std::env::temp_dir().join("upum_test_import_daw_bad.json");
        fs::write(&tmp, "not json at all").unwrap();

        let result = rt_block_on(import_daw_json(tmp.to_string_lossy().to_string()));
        assert!(result.is_err());

        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn test_import_presets_json_valid() {
        let tmp = std::env::temp_dir().join("upum_test_import_presets.json");
        let presets = vec![make_preset("Lead", "FXP"), make_preset("Pad", "VSTPRESET")];
        let json = serde_json::to_string_pretty(&presets).unwrap();
        fs::write(&tmp, &json).unwrap();

        let result = rt_block_on(import_presets_json(tmp.to_string_lossy().to_string()));
        assert!(result.is_ok());
        let imported = result.unwrap();
        assert_eq!(imported.len(), 2);
        assert_eq!(imported[0].name, "Lead");
        assert_eq!(imported[1].format, "VSTPRESET");

        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn test_import_presets_json_extra_field_on_preset_ignored() {
        let tmp = std::env::temp_dir().join("upum_test_import_presets_extra_field.json");
        let _ = fs::remove_file(&tmp);
        let content = r#"{
        "version":"1.0",
        "exported_at":"2025-01-01T00:00:00Z",
        "presets":[{
            "name":"Extra",
            "path":"/p.fxp",
            "directory":"/d",
            "format":"FXP",
            "size":100,
            "sizeFormatted":"100 B",
            "modified":"t",
            "futureProof":true
        }]
    }"#;
        fs::write(&tmp, content).unwrap();
        let imported = rt_block_on(import_presets_json(tmp.to_string_lossy().to_string())).unwrap();
        assert_eq!(imported.len(), 1);
        assert_eq!(imported[0].name, "Extra");
        assert_eq!(imported[0].format, "FXP");
        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn test_import_presets_json_invalid_format() {
        let tmp = std::env::temp_dir().join("upum_test_import_presets_bad.json");
        fs::write(&tmp, r#"[{"wrong": "fields"}]"#).unwrap();

        let result = rt_block_on(import_presets_json(tmp.to_string_lossy().to_string()));
        assert!(result.is_err());

        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn test_export_import_presets_roundtrip() {
        let tmp = std::env::temp_dir().join("upum_test_preset_roundtrip.json");
        let presets = vec![
            make_preset("Bass", "FXB"),
            make_preset("Keys", "AUPRESET"),
            make_preset("Strings", "H2P"),
        ];

        rt_block_on(export_presets_json(
            presets.clone(),
            tmp.to_string_lossy().to_string(),
        ))
        .unwrap();
        let imported = rt_block_on(import_presets_json(tmp.to_string_lossy().to_string())).unwrap();

        assert_eq!(imported.len(), 3);
        assert_eq!(imported[0].name, presets[0].name);
        assert_eq!(imported[1].format, presets[1].format);
        assert_eq!(imported[2].size, presets[2].size);

        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn test_export_import_audio_roundtrip() {
        let tmp = std::env::temp_dir().join("upum_test_audio_roundtrip.json");
        let samples = vec![
            make_audio_sample("hi-hat", "WAV"),
            make_audio_sample("pad", "FLAC"),
        ];

        rt_block_on(export_audio_json(
            samples.clone(),
            tmp.to_string_lossy().to_string(),
        ))
        .unwrap();
        let imported = rt_block_on(import_audio_json(tmp.to_string_lossy().to_string())).unwrap();

        assert_eq!(imported.len(), 2);
        assert_eq!(imported[0].name, "hi-hat");
        assert_eq!(imported[1].format, "FLAC");

        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn test_export_import_daw_roundtrip() {
        let tmp = std::env::temp_dir().join("upum_test_daw_roundtrip.json");
        let projects = vec![
            make_daw_project("Track1", "LOGICX", "Logic Pro"),
            make_daw_project("Track2", "RPP", "REAPER"),
        ];

        rt_block_on(export_daw_json(
            projects.clone(),
            tmp.to_string_lossy().to_string(),
        ))
        .unwrap();
        let imported = rt_block_on(import_daw_json(tmp.to_string_lossy().to_string())).unwrap();

        assert_eq!(imported.len(), 2);
        assert_eq!(imported[0].daw, "Logic Pro");
        assert_eq!(imported[1].format, "RPP");

        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn test_import_presets_json_nonexistent() {
        let result = rt_block_on(import_presets_json("/tmp/nonexistent_preset_file.json".into()));
        assert!(result.is_err());
    }

    #[test]
    fn test_import_daw_json_nonexistent() {
        let result = rt_block_on(import_daw_json("/tmp/nonexistent_daw_file.json".into()));
        assert!(result.is_err());
    }

    #[test]
    fn test_import_audio_json_empty_array() {
        let tmp = std::env::temp_dir().join("upum_test_import_audio_empty.json");
        fs::write(&tmp, "[]").unwrap();

        let result = rt_block_on(import_audio_json(tmp.to_string_lossy().to_string()));
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);

        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn test_import_presets_json_empty_array() {
        let tmp = std::env::temp_dir().join("upum_test_import_presets_empty.json");
        fs::write(&tmp, "[]").unwrap();

        let result = rt_block_on(import_presets_json(tmp.to_string_lossy().to_string()));
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);

        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn test_import_audio_json_errors_when_object_has_no_samples_key() {
        let tmp = std::env::temp_dir().join("upum_test_import_audio_no_samples.json");
        fs::write(
            &tmp,
            r#"{"version":"1.0","exported_at":"2025-01-01T00:00:00Z"}"#,
        )
        .unwrap();
        let err = rt_block_on(import_audio_json(tmp.to_string_lossy().to_string())).unwrap_err();
        assert!(err.contains("samples"), "unexpected error: {err}");
        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn test_import_daw_json_empty_array() {
        let tmp = std::env::temp_dir().join("upum_test_import_daw_empty.json");
        fs::write(&tmp, "[]").unwrap();
        let result = rt_block_on(import_daw_json(tmp.to_string_lossy().to_string()));
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);
        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn test_import_daw_json_errors_when_object_has_no_projects_key() {
        let tmp = std::env::temp_dir().join("upum_test_import_daw_no_projects.json");
        fs::write(&tmp, r#"{"version":"1.0","samples":[]}"#).unwrap();
        let err = rt_block_on(import_daw_json(tmp.to_string_lossy().to_string())).unwrap_err();
        assert!(err.contains("projects"), "unexpected error: {err}");
        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn test_import_audio_json_errors_when_envelope_uses_projects_key() {
        let tmp = std::env::temp_dir().join("upum_test_import_audio_wrong_envelope.json");
        fs::write(&tmp, r#"{"projects":[]}"#).unwrap();
        let err = rt_block_on(import_audio_json(tmp.to_string_lossy().to_string())).unwrap_err();
        assert!(err.contains("samples"), "unexpected error: {err}");
        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn test_import_presets_json_envelope_without_bare_array() {
        let tmp = std::env::temp_dir().join("upum_test_import_presets_envelope_only.json");
        let preset = make_preset("OnlyEnvelope", "FXP");
        let json = serde_json::json!({ "presets": [preset] });
        fs::write(&tmp, serde_json::to_string(&json).unwrap()).unwrap();
        let imported = rt_block_on(import_presets_json(tmp.to_string_lossy().to_string())).unwrap();
        assert_eq!(imported.len(), 1);
        assert_eq!(imported[0].name, "OnlyEnvelope");
        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn test_import_audio_json_samples_not_array_returns_error() {
        let tmp = std::env::temp_dir().join("upum_test_import_audio_samples_bad_type.json");
        fs::write(&tmp, r#"{"samples":"nope"}"#).unwrap();
        assert!(rt_block_on(import_audio_json(tmp.to_string_lossy().to_string())).is_err());
        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn test_import_daw_json_projects_not_array_returns_error() {
        let tmp = std::env::temp_dir().join("upum_test_import_daw_projects_bad_type.json");
        fs::write(&tmp, r#"{"projects":{}}"#).unwrap();
        assert!(rt_block_on(import_daw_json(tmp.to_string_lossy().to_string())).is_err());
        let _ = fs::remove_file(&tmp);
    }

    // ── File browser tests ──

    #[test]
    fn test_fs_list_dir_valid() {
        let tmp = std::env::temp_dir().join("upum_test_fs_list");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        fs::write(tmp.join("file1.txt"), "hello").unwrap();
        fs::write(tmp.join("file2.wav"), "audio").unwrap();
        fs::create_dir(tmp.join("subdir")).unwrap();
        fs::write(tmp.join(".hidden"), "skip").unwrap();

        let result = rt_block_on(fs_list_dir(tmp.to_string_lossy().to_string())).unwrap();
        let entries = result["entries"].as_array().unwrap();
        // Should have 3 entries (subdir, file1.txt, file2.wav) — .hidden is skipped
        assert_eq!(entries.len(), 3);
        // Dirs first
        assert!(entries[0]["isDir"].as_bool().unwrap());
        assert_eq!(entries[0]["name"].as_str().unwrap(), "subdir");
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_fs_list_dir_nonexistent() {
        let result = rt_block_on(fs_list_dir("/nonexistent/upum_dir_xyz".into()));
        assert!(result.is_err());
    }

    #[test]
    fn test_fs_list_dir_not_a_dir() {
        let tmp = std::env::temp_dir().join("upum_test_fs_notdir.txt");
        fs::write(&tmp, "data").unwrap();
        let result = rt_block_on(fs_list_dir(tmp.to_string_lossy().to_string()));
        assert!(result.is_err());
        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn test_delete_file_regular() {
        let tmp = std::env::temp_dir().join("upum_test_delete.txt");
        fs::write(&tmp, "delete me").unwrap();
        assert!(tmp.exists());
        rt_block_on(delete_file(tmp.to_string_lossy().to_string())).unwrap();
        assert!(!tmp.exists());
    }

    #[test]
    fn test_delete_file_directory() {
        let tmp = std::env::temp_dir().join("upum_test_delete_dir");
        fs::create_dir_all(tmp.join("inner")).unwrap();
        fs::write(tmp.join("inner").join("file.txt"), "data").unwrap();
        rt_block_on(delete_file(tmp.to_string_lossy().to_string())).unwrap();
        assert!(!tmp.exists());
    }

    #[test]
    fn test_delete_file_nonexistent() {
        let result = rt_block_on(delete_file("/nonexistent/upum_file_xyz.txt".into()));
        assert!(result.is_err());
    }

    #[test]
    fn test_rename_file() {
        let tmp1 = std::env::temp_dir().join("upum_test_rename_old.txt");
        let tmp2 = std::env::temp_dir().join("upum_test_rename_new.txt");
        let _ = fs::remove_file(&tmp2);
        fs::write(&tmp1, "content").unwrap();
        rt_block_on(rename_file(
            tmp1.to_string_lossy().to_string(),
            tmp2.to_string_lossy().to_string(),
        ))
        .unwrap();
        assert!(!tmp1.exists());
        assert!(tmp2.exists());
        assert_eq!(fs::read_to_string(&tmp2).unwrap(), "content");
        let _ = fs::remove_file(&tmp2);
    }

    #[test]
    fn test_get_home_dir() {
        let result = rt_block_on(get_home_dir());
        assert!(result.is_ok());
        let home = result.unwrap();
        assert!(!home.is_empty());
        assert!(std::path::Path::new(&home).exists());
    }

    // ── Cache file tests ──

    #[test]
    fn test_cache_file_roundtrip() {
        let _guard = test_data_dir();
        let db = db::Database::open().expect("open db for cache roundtrip");
        let data = serde_json::json!({"hello": "world", "count": 42});
        db.write_cache("test-cache-roundtrip.json", &data).unwrap();
        let result = db.read_cache("test-cache-roundtrip.json").unwrap();
        assert_eq!(result["hello"], "world");
        assert_eq!(result["count"], 42);
    }

    #[test]
    fn test_cache_file_nonexistent() {
        let _guard = test_data_dir();
        let db = db::Database::open().expect("open db for cache read");
        let result = db.read_cache("nonexistent-cache-xyz.json").unwrap();
        // Falls back to waveform_cache table — result is valid JSON (may be empty or populated)
        assert!(result.is_object());
    }

    #[test]
    fn test_append_and_read_log() {
        let _guard = app_log_lock();
        let _tmp = test_data_dir();
        rt_block_on(clear_log()).unwrap();
        let token = format!(
            "log-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        );
        append_log(format!("{token} entry1"));
        append_log(format!("{token} entry2"));
        let log = rt_block_on(read_log()).unwrap();
        assert!(
            log.contains(&format!("{token} entry1")),
            "missing first line in log (len {})",
            log.len()
        );
        assert!(
            log.contains(&format!("{token} entry2")),
            "missing second line in log (len {})",
            log.len()
        );
    }

    #[test]
    fn test_clear_log() {
        let _guard = app_log_lock();
        let _tmp = test_data_dir();
        rt_block_on(clear_log()).unwrap();
        append_log("before clear".into());
        rt_block_on(clear_log()).unwrap();
        let log = rt_block_on(read_log()).unwrap();
        assert!(!log.contains("before clear"));
    }

    #[test]
    fn test_read_log_missing_file_returns_empty() {
        let _guard = app_log_lock();
        let tmp = test_data_dir();
        let _ = fs::remove_file(tmp.path.join("app.log"));
        assert_eq!(rt_block_on(read_log()).unwrap(), "");
    }

    #[test]
    fn test_log_entries_have_timestamp() {
        let _guard = app_log_lock();
        let _tmp = test_data_dir();
        rt_block_on(clear_log()).unwrap();
        append_log("timestamp-check".into());
        let log = rt_block_on(read_log()).unwrap();
        // Timestamp format: [YYYY-MM-DD HH:MM:SS]
        let re =
            regex::Regex::new(r"\[\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}\] timestamp-check").unwrap();
        assert!(re.is_match(&log), "log entry missing timestamp: {}", log);
    }

    #[test]
    fn test_log_appends_not_overwrites() {
        let _guard = app_log_lock();
        let _tmp = test_data_dir();
        rt_block_on(clear_log()).unwrap();
        append_log("first".into());
        append_log("second".into());
        append_log("third".into());
        let log = rt_block_on(read_log()).unwrap();
        let lines: Vec<&str> = log.lines().collect();
        assert!(
            lines.len() >= 3,
            "expected at least 3 lines, got {}",
            lines.len()
        );
        assert!(lines.iter().any(|l| l.contains("first")));
        assert!(lines.iter().any(|l| l.contains("second")));
        assert!(lines.iter().any(|l| l.contains("third")));
        // Verify order: first appears before second
        let first_pos = log.find("first").unwrap();
        let second_pos = log.find("second").unwrap();
        let third_pos = log.find("third").unwrap();
        assert!(
            first_pos < second_pos && second_pos < third_pos,
            "log entries out of order"
        );
    }

    #[test]
    fn test_log_handles_special_characters() {
        let _guard = app_log_lock();
        let _tmp = test_data_dir();
        rt_block_on(clear_log()).unwrap();
        append_log("unicode: 日本語テスト 🎵 emoji".into());
        append_log("newlines: line1\nline2".into());
        append_log("path: /Users/test/my file (1).vst3".into());
        let log = rt_block_on(read_log()).unwrap();
        assert!(log.contains("日本語テスト"));
        assert!(log.contains("🎵"));
        assert!(log.contains("my file (1).vst3"));
    }

    #[test]
    fn test_log_concurrent_appends() {
        let _guard = app_log_lock();
        let tmp = test_data_dir();
        rt_block_on(clear_log()).unwrap();
        let path = tmp.path.clone();
        let handles: Vec<_> = (0..10)
            .map(|i| {
                let path = path.clone();
                std::thread::spawn(move || {
                    history::set_test_data_dir_path(path);
                    append_log(format!("concurrent-{i}"));
                })
            })
            .collect();
        for h in handles {
            h.join().unwrap();
        }
        let log = rt_block_on(read_log()).unwrap();
        for i in 0..10 {
            assert!(
                log.contains(&format!("concurrent-{i}")),
                "missing concurrent-{i}"
            );
        }
    }

    #[test]
    fn test_clear_log_then_append_works() {
        let _guard = app_log_lock();
        let _tmp = test_data_dir();
        rt_block_on(clear_log()).unwrap();
        append_log("before".into());
        rt_block_on(clear_log()).unwrap();
        append_log("after".into());
        let log = rt_block_on(read_log()).unwrap();
        assert!(!log.contains("before"), "cleared content should be gone");
        assert!(log.contains("after"), "new content should be present");
    }

    // ── TOML export/import tests ──

    #[test]
    fn test_export_import_toml_roundtrip() {
        let tmp = std::env::temp_dir().join("upum_test_export.toml");
        let data = serde_json::json!({
            "plugins": [{"name": "Test", "version": "1.0"}]
        });
        rt_block_on(export_toml(data.clone(), tmp.to_string_lossy().to_string())).unwrap();
        let imported = rt_block_on(import_toml(tmp.to_string_lossy().to_string())).unwrap();
        assert!(imported["plugins"].is_array());
        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn test_import_toml_nonexistent() {
        let result = rt_block_on(import_toml("/nonexistent/file.toml".into()));
        assert!(result.is_err());
    }

    #[test]
    fn test_import_toml_invalid() {
        let tmp = std::env::temp_dir().join("upum_test_invalid.toml");
        fs::write(&tmp, "this is not valid toml [[[").unwrap();
        let result = rt_block_on(import_toml(tmp.to_string_lossy().to_string()));
        assert!(result.is_err());
        let _ = fs::remove_file(&tmp);
    }

    // ── Preset DSV export tests ──

    #[test]
    fn test_export_presets_dsv_csv() {
        let tmp = std::env::temp_dir().join("upum_test_presets.csv");
        let presets = vec![PresetFile {
            name: "Lead".into(),
            path: "/presets/lead.fxp".into(),
            directory: "/presets".into(),
            format: "FXP".into(),
            size: 1024,
            size_formatted: "1.0 KB".into(),
            modified: "2024-01-01".into(),
        }];
        rt_block_on(export_presets_dsv(presets, tmp.to_string_lossy().to_string())).unwrap();
        let content = fs::read_to_string(&tmp).unwrap();
        assert!(content.contains("Lead"));
        assert!(content.contains("FXP"));
        assert!(content.contains(","));
        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn test_export_presets_dsv_tsv() {
        let tmp = std::env::temp_dir().join("upum_test_presets.tsv");
        let presets = vec![PresetFile {
            name: "Bass".into(),
            path: "/presets/bass.fxp".into(),
            directory: "/presets".into(),
            format: "FXP".into(),
            size: 2048,
            size_formatted: "2.0 KB".into(),
            modified: "2024-02-01".into(),
        }];
        rt_block_on(export_presets_dsv(presets, tmp.to_string_lossy().to_string())).unwrap();
        let content = fs::read_to_string(&tmp).unwrap();
        assert!(content.contains("Bass"));
        assert!(content.contains("\t"));
        let _ = fs::remove_file(&tmp);
    }

    // ── .band validation tests ──

    #[test]
    fn test_band_validation_valid() {
        let tmp = std::env::temp_dir().join("upum_test_valid.band");
        fs::create_dir_all(tmp.join("Media")).unwrap();
        fs::write(tmp.join("projectData"), b"bplist00fake").unwrap();
        assert!(daw_scanner::is_package_ext(&tmp));
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_band_validation_no_bplist() {
        let tmp = std::env::temp_dir().join("upum_test_nobplist.band");
        fs::create_dir_all(tmp.join("Media")).unwrap();
        fs::write(tmp.join("projectData"), b"not a plist").unwrap();
        // is_package_ext returns true (it's a .band dir) but the internal
        // validation in walk_for_daw would reject it
        assert!(daw_scanner::is_package_ext(&tmp));
        let _ = fs::remove_dir_all(&tmp);
    }

    // ── open_daw_project tests ──

    #[test]
    fn test_open_daw_project_nonexistent() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(open_daw_project("/nonexistent/project.als".into()));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[test]
    fn bulk_format_size_non_empty() {
        for i in 0..12_000u32 {
            let b = i as u64 * 17 + (i as u64 % 1024);
            let s = format_size(b);
            assert!(!s.is_empty(), "format_size({b})");
        }
    }

    #[test]
    fn test_format_size_one_gb() {
        assert_eq!(format_size(1024_u64.pow(3)), "1.0 GB");
    }

    #[test]
    fn test_format_size_one_byte_below_one_gib_stays_in_mb_tier() {
        let b = 1024_u64.pow(3) - 1;
        let s = format_size(b);
        assert!(
            s.ends_with(" MB"),
            "just under 1 GiB should use MB unit, got {s}"
        );
    }

    #[test]
    fn test_detect_separator_unknown_extension_defaults_csv() {
        assert_eq!(detect_separator("export.data"), ',');
        assert_eq!(detect_separator("/tmp/no_extension"), ',');
    }

    #[test]
    fn test_export_pdf_writes_pdf_magic_bytes() {
        let tmp =
            std::env::temp_dir().join(format!("ah_export_pdf_test_{}.pdf", std::process::id()));
        let _ = fs::remove_file(&tmp);
        rt_block_on(export_pdf(
            "Unit test".into(),
            vec!["Col A".into(), "Col B".into()],
            vec![vec!["cell-a".into(), "cell-b".into()]],
            tmp.to_string_lossy().to_string(),
        ))
        .unwrap();
        let bytes = fs::read(&tmp).unwrap();
        assert!(
            bytes.starts_with(b"%PDF-"),
            "expected PDF header, got {:?}",
            &bytes[..bytes.len().min(16)]
        );
        let _ = fs::remove_file(&tmp);
    }
}

// ── Database IPC commands ──

#[tauri::command]
async fn db_query_audio(params: db::AudioQueryParams) -> Result<db::AudioQueryResult, String> {
    tokio::task::spawn_blocking(move || db::global().query_audio(&params))
        .await
        .map_err(|e| format!("db_query_audio task: {e}"))?
}

#[tauri::command(rename_all = "snake_case")]
async fn db_query_plugins(
    search: Option<String>,
    type_filter: Option<String>,
    status_filter: Option<String>,
    sort_key: Option<String>,
    sort_asc: Option<bool>,
    search_regex: Option<bool>,
    offset: Option<u64>,
    limit: Option<u64>,
) -> Result<db::PluginQueryResult, String> {
    let search_regex = search_regex.unwrap_or(false);
    tokio::task::spawn_blocking(move || {
        db::global().query_plugins(
            search.as_deref(),
            type_filter.as_deref(),
            status_filter.as_deref(),
            &sort_key.unwrap_or("name".into()),
            sort_asc.unwrap_or(true),
            search_regex,
            offset.unwrap_or(0),
            limit.unwrap_or(200),
        )
    })
    .await
    .map_err(|e| format!("db_query_plugins task: {e}"))?
}

#[tauri::command(rename_all = "snake_case")]
async fn db_query_daw(
    search: Option<String>,
    daw_filter: Option<String>,
    sort_key: Option<String>,
    sort_asc: Option<bool>,
    search_regex: Option<bool>,
    offset: Option<u64>,
    limit: Option<u64>,
) -> Result<db::DawQueryResult, String> {
    let search_regex = search_regex.unwrap_or(false);
    tokio::task::spawn_blocking(move || {
        db::global().query_daw(
            search.as_deref(),
            daw_filter.as_deref(),
            &sort_key.unwrap_or("name".into()),
            sort_asc.unwrap_or(true),
            search_regex,
            offset.unwrap_or(0),
            limit.unwrap_or(200),
        )
    })
    .await
    .map_err(|e| format!("db_query_daw task: {e}"))?
}

#[tauri::command(rename_all = "snake_case")]
async fn db_query_presets(
    search: Option<String>,
    format_filter: Option<String>,
    sort_key: Option<String>,
    sort_asc: Option<bool>,
    search_regex: Option<bool>,
    offset: Option<u64>,
    limit: Option<u64>,
) -> Result<db::PresetQueryResult, String> {
    let search_regex = search_regex.unwrap_or(false);
    tokio::task::spawn_blocking(move || {
        db::global().query_presets(
            search.as_deref(),
            format_filter.as_deref(),
            &sort_key.unwrap_or("name".into()),
            sort_asc.unwrap_or(true),
            search_regex,
            offset.unwrap_or(0),
            limit.unwrap_or(200),
        )
    })
    .await
    .map_err(|e| format!("db_query_presets task: {e}"))?
}

#[tauri::command]
async fn db_audio_stats(scan_id: Option<String>) -> Result<db::AudioStatsResult, String> {
    blocking_res(move || db::global().audio_stats(scan_id.as_deref())).await
}

#[tauri::command]
async fn db_daw_stats(scan_id: Option<String>) -> Result<db::DawStatsResult, String> {
    blocking_res(move || db::global().daw_stats(scan_id.as_deref())).await
}

#[tauri::command]
async fn db_preset_stats(scan_id: Option<String>) -> Result<db::PresetStatsResult, String> {
    blocking_res(move || db::global().preset_stats(scan_id.as_deref())).await
}

#[tauri::command(rename_all = "snake_case")]
async fn db_query_pdfs(
    search: Option<String>,
    sort_key: Option<String>,
    sort_asc: Option<bool>,
    search_regex: Option<bool>,
    offset: Option<u64>,
    limit: Option<u64>,
) -> Result<db::PdfQueryResult, String> {
    let search_regex = search_regex.unwrap_or(false);
    tokio::task::spawn_blocking(move || {
        db::global().query_pdfs(
            search.as_deref(),
            &sort_key.unwrap_or("name".into()),
            sort_asc.unwrap_or(true),
            search_regex,
            offset.unwrap_or(0),
            limit.unwrap_or(200),
        )
    })
    .await
    .map_err(|e| format!("db_query_pdfs task: {e}"))?
}

/// Single IPC round-trip for Cmd+K inventory preview (same limits as six separate `db_query_*` calls).
#[derive(Debug, Serialize)]
pub struct PalettePreviewResult {
    pub plugins: db::PluginQueryResult,
    pub audio: db::AudioQueryResult,
    pub daw: db::DawQueryResult,
    pub presets: db::PresetQueryResult,
    pub pdfs: db::PdfQueryResult,
    pub midi: db::MidiQueryResult,
}

fn palette_preview_empty() -> PalettePreviewResult {
    PalettePreviewResult {
        plugins: db::PluginQueryResult {
            plugins: vec![],
            total_count: 0,
            total_count_capped: false,
            total_unfiltered: 0,
        },
        audio: db::AudioQueryResult {
            samples: vec![],
            total_count: 0,
            total_count_capped: false,
            total_unfiltered: 0,
        },
        daw: db::DawQueryResult {
            projects: vec![],
            total_count: 0,
            total_count_capped: false,
            total_unfiltered: 0,
        },
        presets: db::PresetQueryResult {
            presets: vec![],
            total_count: 0,
            total_count_capped: false,
            total_unfiltered: 0,
        },
        pdfs: db::PdfQueryResult {
            pdfs: vec![],
            total_count: 0,
            total_count_capped: false,
            total_unfiltered: 0,
        },
        midi: db::MidiQueryResult {
            midi_files: vec![],
            total_count: 0,
            total_count_capped: false,
            total_unfiltered: 0,
        },
    }
}

#[tauri::command(rename_all = "snake_case")]
async fn db_query_palette_preview(search: String) -> Result<PalettePreviewResult, String> {
    let search = search.trim().to_string();
    if search.len() < 2 {
        return Ok(palette_preview_empty());
    }
    tokio::task::spawn_blocking(move || {
        let db = db::global();
        let plugins = db.query_plugins(Some(&search), None, None, "name", true, false, 0, 6)?;
        let audio = db.query_audio(&db::AudioQueryParams {
            scan_id: None,
            search: Some(search.clone()),
            search_regex: false,
            format_filter: None,
            sort_key: "name".into(),
            sort_asc: true,
            offset: 0,
            limit: 6,
        })?;
        let daw = db.query_daw(Some(&search), None, "name", true, false, 0, 6)?;
        let presets = db.query_presets(Some(&search), None, "name", true, false, 0, 6)?;
        let pdfs = db.query_pdfs(Some(&search), "name", true, false, 0, 6)?;
        let midi = db.query_midi(Some(&search), None, "name", true, false, 0, 6)?;
        Ok(PalettePreviewResult {
            plugins,
            audio,
            daw,
            presets,
            pdfs,
            midi,
        })
    })
    .await
    .map_err(|e| format!("db_query_palette_preview task: {e}"))?
}

#[tauri::command]
async fn db_pdf_stats(scan_id: Option<String>) -> Result<db::PdfStatsResult, String> {
    blocking_res(move || db::global().pdf_stats(scan_id.as_deref())).await
}

#[tauri::command(rename_all = "snake_case")]
async fn db_audio_filter_stats(
    search: Option<String>,
    format_filter: Option<String>,
    search_regex: Option<bool>,
) -> Result<db::FilterStatsResult, String> {
    let search_regex = search_regex.unwrap_or(false);
    tokio::task::spawn_blocking(move || {
        db::global().audio_filter_stats(
            search.as_deref(),
            format_filter.as_deref(),
            search_regex,
        )
    })
    .await
    .map_err(|e| format!("db_audio_filter_stats task: {e}"))?
}

#[tauri::command(rename_all = "snake_case")]
async fn db_daw_filter_stats(
    search: Option<String>,
    daw_filter: Option<String>,
    search_regex: Option<bool>,
) -> Result<db::FilterStatsResult, String> {
    let search_regex = search_regex.unwrap_or(false);
    tokio::task::spawn_blocking(move || {
        db::global().daw_filter_stats(
            search.as_deref(),
            daw_filter.as_deref(),
            search_regex,
        )
    })
    .await
    .map_err(|e| format!("db_daw_filter_stats task: {e}"))?
}

#[tauri::command(rename_all = "snake_case")]
async fn db_preset_filter_stats(
    search: Option<String>,
    format_filter: Option<String>,
    search_regex: Option<bool>,
) -> Result<db::FilterStatsResult, String> {
    let search_regex = search_regex.unwrap_or(false);
    tokio::task::spawn_blocking(move || {
        db::global().preset_filter_stats(
            search.as_deref(),
            format_filter.as_deref(),
            search_regex,
        )
    })
    .await
    .map_err(|e| format!("db_preset_filter_stats task: {e}"))?
}

#[tauri::command(rename_all = "snake_case")]
async fn db_plugin_filter_stats(
    search: Option<String>,
    type_filter: Option<String>,
    search_regex: Option<bool>,
) -> Result<db::FilterStatsResult, String> {
    let search_regex = search_regex.unwrap_or(false);
    tokio::task::spawn_blocking(move || {
        db::global().plugin_filter_stats(
            search.as_deref(),
            type_filter.as_deref(),
            search_regex,
        )
    })
    .await
    .map_err(|e| format!("db_plugin_filter_stats task: {e}"))?
}

#[tauri::command(rename_all = "snake_case")]
async fn db_pdf_filter_stats(
    search: Option<String>,
    search_regex: Option<bool>,
) -> Result<db::FilterStatsResult, String> {
    let search_regex = search_regex.unwrap_or(false);
    tokio::task::spawn_blocking(move || {
        db::global().pdf_filter_stats(search.as_deref(), search_regex)
    })
    .await
    .map_err(|e| format!("db_pdf_filter_stats task: {e}"))?
}

/// Per-category row counts for the header strip — **library** scope (one row per `path`), not the
/// current in-progress `scan_id`. See [`db::Database::active_scan_inventory_counts`].
#[tauri::command]
async fn get_active_scan_inventory_counts() -> Result<serde_json::Value, String> {
    blocking_res(|| db::global().active_scan_inventory_counts()).await
}

#[tauri::command]
async fn db_list_scans() -> Result<Vec<db::ScanInfo>, String> {
    blocking_res(|| db::global().list_scans()).await
}

#[tauri::command]
async fn db_update_bpm(path: String, bpm: Option<f64>) -> Result<(), String> {
    blocking_res(move || db::global().update_bpm(&path, bpm)).await
}

#[tauri::command]
async fn db_update_key(path: String, key: Option<String>) -> Result<(), String> {
    blocking_res(move || db::global().update_key(&path, key.as_deref())).await
}

#[tauri::command]
async fn db_update_lufs(path: String, lufs: Option<f64>) -> Result<(), String> {
    blocking_res(move || db::global().update_lufs(&path, lufs)).await
}

/// Persist BPM, key, and LUFS together (same transaction and `bpm_exhausted` rules as batch analysis).
#[tauri::command]
async fn db_update_analysis(
    path: String,
    bpm: Option<f64>,
    key: Option<String>,
    lufs: Option<f64>,
) -> Result<(), String> {
    let row = vec![(path, bpm, key, lufs)];
    blocking_res(move || db::global().batch_update_analysis(&row).map(|_| ())).await
}

#[tauri::command]
async fn db_backfill_audio_meta(paths: Vec<String>) -> Result<serde_json::Value, String> {
    blocking_res(move || {
        let missing = db::global().paths_missing_audio_meta(&paths)?;
        if missing.is_empty() {
            return Ok(serde_json::json!({}));
        }
        let mut updated = serde_json::Map::new();
        for p in &missing {
            let am = audio_scanner::get_audio_metadata(p);
            if am.duration.is_some() || am.channels.is_some() {
                db::global().update_audio_meta(
                    p,
                    am.duration,
                    am.channels,
                    am.sample_rate,
                    am.bits_per_sample,
                )?;
                let mut obj = serde_json::Map::new();
                if let Some(d) = am.duration {
                    obj.insert("duration".into(), serde_json::json!(d));
                }
                if let Some(c) = am.channels {
                    obj.insert("channels".into(), serde_json::json!(c));
                }
                if let Some(sr) = am.sample_rate {
                    obj.insert("sampleRate".into(), serde_json::json!(sr));
                }
                if let Some(bps) = am.bits_per_sample {
                    obj.insert("bitsPerSample".into(), serde_json::json!(bps));
                }
                updated.insert(p.clone(), serde_json::Value::Object(obj));
            }
        }
        Ok(serde_json::Value::Object(updated))
    })
    .await
}

#[tauri::command]
async fn db_get_analysis(path: String) -> Result<serde_json::Value, String> {
    blocking_res(move || db::global().get_analysis(&path)).await
}

#[tauri::command]
async fn db_unanalyzed_paths(limit: Option<u64>) -> Result<Vec<String>, String> {
    blocking_res(move || db::global().unanalyzed_paths(limit.unwrap_or(100))).await
}

#[tauri::command]
async fn db_audio_library_paths() -> Result<Vec<String>, String> {
    blocking_res(move || db::global().audio_library_paths()).await
}

#[tauri::command]
async fn db_migrate_json() -> Result<usize, String> {
    blocking_res(|| db::global().migrate_from_json()).await
}

#[tauri::command]
async fn db_cache_stats() -> Result<Vec<db::CacheStat>, String> {
    blocking_res(|| db::global().cache_stats()).await
}

#[tauri::command]
async fn db_clear_caches() -> Result<(), String> {
    append_log("DB CLEAR — all caches (waveform, spectrogram, xref, fingerprint, kvr)".into());
    blocking_res(|| db::global().clear_all_caches()).await
}

#[tauri::command]
async fn db_clear_cache_table(table: String) -> Result<(), String> {
    append_log(format!("DB CLEAR — cache table: {}", table));
    blocking_res(move || db::global().clear_cache_table(&table)).await
}

fn resolve_ui_locale(locale: Option<String>) -> String {
    locale.unwrap_or_else(|| {
        history::load_preferences()
            .get("uiLocale")
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_else(|| "en".to_string())
    })
}

#[tauri::command]
async fn get_app_strings(
    locale: Option<String>,
) -> Result<std::collections::HashMap<String, String>, String> {
    let loc = resolve_ui_locale(locale);
    blocking_res(move || db::global().get_app_strings(&loc)).await
}

#[tauri::command]
async fn get_toast_strings(
    locale: Option<String>,
) -> Result<std::collections::HashMap<String, String>, String> {
    get_app_strings(locale).await
}

/// Rebuild the native menu bar from SQLite `app_i18n` for the current UI locale (after changing language in Settings).
#[tauri::command]
fn refresh_native_menu(app: AppHandle) -> Result<(), String> {
    let ui_locale = resolve_ui_locale(None);
    let strings = db::global().get_app_strings(&ui_locale).unwrap_or_default();
    let menu = native_menu::build_native_menu_bar(&app, &strings).map_err(|e| e.to_string())?;
    app.set_menu(menu).map_err(|e| e.to_string())?;
    let state = app.state::<tray_menu::TrayState>();
    if let Ok(guard) = state.0.lock() {
        if let Some(ref tray) = *guard {
            tray_menu::refresh_tray_popup_menu(&app, tray, &strings)?;
        }
    }
    Ok(())
}

// ── File watcher commands ──

#[tauri::command]
fn start_file_watcher(app: AppHandle, dirs: Vec<String>) -> Result<(), String> {
    append_log(format!(
        "FILE WATCHER START — {} directories: {:?}",
        dirs.len(),
        dirs
    ));
    let state = app.state::<file_watcher::FileWatcherState>();
    file_watcher::start_watching(&app, &state, dirs)
}

#[tauri::command]
fn stop_file_watcher(app: AppHandle) -> Result<(), String> {
    append_log("FILE WATCHER STOP".into());
    let state = app.state::<file_watcher::FileWatcherState>();
    file_watcher::stop_watching(&state);
    Ok(())
}

#[tauri::command]
fn get_file_watcher_status(app: AppHandle) -> serde_json::Value {
    let state = app.state::<file_watcher::FileWatcherState>();
    serde_json::json!({
        "watching": file_watcher::is_watching(&state),
        "dirs": file_watcher::get_watched_dirs(&state),
    })
}

// ── App setup ──

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Panic hook — write crash info to app.log before dying
    std::panic::set_hook(Box::new(|info| {
        let path = history::ensure_data_dir().join("app.log");
        let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let location = info
            .location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
            .unwrap_or_default();
        let payload = if let Some(s) = info.payload().downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "unknown panic".into()
        };
        let backtrace = std::backtrace::Backtrace::force_capture();
        let msg = format!("[{timestamp}] PANIC at {location}: {payload}\n{backtrace}\n");
        eprintln!("{msg}");
        let _ = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .and_then(|mut f| {
                use std::io::Write;
                f.write_all(msg.as_bytes())
            });
    }));

    // Initialize app start time for uptime tracking
    APP_START.get_or_init(Instant::now);

    // Register atexit handler: terminate the AudioEngine, then shutdown logging (Cmd+Q, SIGTERM, etc.)
    extern "C" fn on_exit() {
        let _ = audio_engine::shutdown_audio_engine_child();
        log_shutdown();
    }
    unsafe {
        libc::atexit(on_exit);
    }

    // Load preferences once for all startup config
    let prefs = history::load_preferences();
    refresh_log_verbosity_from_prefs();

    // Log startup with system info
    let rss = get_rss_bytes();
    let db_path = history::ensure_data_dir().join("audio_haxor.db");
    let db_size = std::fs::metadata(&db_path).map(|m| m.len()).unwrap_or(0);
    let rayon_threads = rayon::current_num_threads();
    let hostname = sysinfo::System::host_name().unwrap_or_default();
    // Raise file descriptor limit for intensive directory walking
    #[cfg(unix)]
    let fd_target: u64 = prefs
        .get("fdLimit")
        .and_then(|v| v.as_str().and_then(|s| s.parse().ok()).or(v.as_u64()))
        .unwrap_or(10240)
        .clamp(256, 65536);
    #[cfg(not(unix))]
    let fd_target: u64 = 0;

    let batch_size = prefs
        .get("batchSize")
        .and_then(|v| v.as_str())
        .unwrap_or("100");
    let channel_buffer = prefs
        .get("channelBuffer")
        .and_then(|v| v.as_str())
        .unwrap_or("512");
    let flush_interval = prefs
        .get("flushInterval")
        .and_then(|v| v.as_str())
        .unwrap_or("100");
    let analysis_pause = prefs
        .get("analysisPause")
        .and_then(|v| v.as_str())
        .unwrap_or("100");
    let page_size = prefs
        .get("pageSize")
        .and_then(|v| v.as_str())
        .unwrap_or("200");
    let auto_scan = prefs
        .get("autoScan")
        .and_then(|v| v.as_str())
        .unwrap_or("off");
    let folder_watch = prefs
        .get("folderWatch")
        .and_then(|v| v.as_str())
        .unwrap_or("off");
    let log_verbosity = prefs
        .get("logVerbosity")
        .and_then(|v| v.as_str())
        .unwrap_or("normal");

    append_log(format!(
        "APP START — v{} | {} {} | {} | {} cores | {} rayon threads | pid {} | RSS {} | DB {}",
        env!("CARGO_PKG_VERSION"),
        std::env::consts::OS,
        std::env::consts::ARCH,
        hostname,
        num_cpus::get(),
        rayon_threads,
        std::process::id(),
        format_size(rss),
        format_size(db_size),
    ));
    append_log(format!(
        "CONFIG — fd_limit: {} | batch_size: {} | channel_buffer: {} | flush_interval: {}ms | analysis_pause: {}ms | page_size: {} | auto_scan: {} | folder_watch: {} | log_verbosity: {}",
        fd_target, batch_size, channel_buffer, flush_interval, analysis_pause, page_size, auto_scan, folder_watch, log_verbosity,
    ));

    #[cfg(unix)]
    {
        let mut rlim = libc::rlimit {
            rlim_cur: 0,
            rlim_max: 0,
        };
        unsafe {
            if libc::getrlimit(libc::RLIMIT_NOFILE, &mut rlim) == 0 {
                let target = (rlim.rlim_max).min(fd_target);
                if rlim.rlim_cur < target {
                    rlim.rlim_cur = target;
                    libc::setrlimit(libc::RLIMIT_NOFILE, &rlim);
                }
            }
        }
    }

    // Initialize rayon thread pool — multiplier read from config (default 4x).
    // Filesystem scanning is heavily I/O-bound: threads spend most time waiting
    // on disk reads, stat calls, and plist parsing. Oversubscription ensures
    // there are always runnable threads when others are blocked on I/O.
    let multiplier = prefs
        .get("threadMultiplier")
        .and_then(|v| {
            v.as_str()
                .or_else(|| v.as_u64().map(|_| ""))
                .and_then(|s| s.parse::<usize>().ok())
        })
        .or_else(|| {
            prefs
                .get("threadMultiplier")
                .and_then(|v| v.as_u64().map(|n| n as usize))
        })
        .unwrap_or(8)
        .clamp(1, 16);
    let pool_size = num_cpus::get() * multiplier;
    append_log(format!(
        "THREAD POOL — {}x multiplier | {} threads | 8MB stack",
        multiplier, pool_size,
    ));
    rayon::ThreadPoolBuilder::new()
        .num_threads(pool_size)
        .stack_size(8 * 1024 * 1024)
        .panic_handler(|panic_info| {
            let msg = format!("Rayon thread panicked: {:?}", panic_info);
            eprintln!("{msg}");
            append_log(msg);
        })
        .build_global()
        .ok();

    // Initialize global SQLite database (open + migrate only — fast)
    db::init_global().expect("Failed to initialize database");

    // Two-phase DB housekeeping (off main thread). `prune_old_scans` + `rebuild_*_libraries` can
    // hold a pooled handle for a long time; `setup()` + first IPC need `read_conn()` without waiting.
    // Light: `PRAGMA optimize` + prewarm soon after launch. Heavy: prune + `VACUUM` many seconds later.
    const STARTUP_DB_LIGHT_DELAY_MS: u64 = 750;
    const STARTUP_DB_HEAVY_DELAY_SECS: u64 = 12;
    std::thread::spawn(|| {
        std::thread::sleep(std::time::Duration::from_millis(STARTUP_DB_LIGHT_DELAY_MS));
        db::global().housekeep_light();
    });
    std::thread::spawn(|| {
        std::thread::sleep(std::time::Duration::from_secs(STARTUP_DB_HEAVY_DELAY_SECS));
        db::global().housekeep_heavy();
        if let Ok(counts) = db::global().table_counts() {
            let m = counts.as_object().unwrap();
            let get = |k: &str| m.get(k).and_then(|v| v.as_u64()).unwrap_or(0);
            append_log(format!(
                "DB STATS — {} plugins | {} samples | {} DAW projects | {} presets | {} KVR cache | {} waveforms | {} spectrograms | {} xref | {} fingerprints",
                get("plugins"), get("audio_samples"), get("daw_projects"), get("presets"),
                get("kvr_cache"), get("waveform_cache"), get("spectrogram_cache"), get("xref_cache"), get("fingerprint_cache"),
            ));
        }
    });

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_drag::init())
        .manage(ScanState {
            scanning: AtomicBool::new(false),
            stop_scan: AtomicBool::new(false),
        })
        .manage(UpdateState {
            checking: AtomicBool::new(false),
            stop_updates: AtomicBool::new(false),
        })
        .manage(AudioScanState {
            scanning: AtomicBool::new(false),
            stop_scan: AtomicBool::new(false),
        })
        .manage(DawScanState {
            scanning: AtomicBool::new(false),
            stop_scan: AtomicBool::new(false),
        })
        .manage(PresetScanState {
            scanning: AtomicBool::new(false),
            stop_scan: AtomicBool::new(false),
        })
        .manage(MidiScanState {
            scanning: AtomicBool::new(false),
            stop_scan: AtomicBool::new(false),
        })
        .manage(PdfScanState {
            scanning: AtomicBool::new(false),
            stop_scan: AtomicBool::new(false),
        })
        .manage(WalkerStatus {
            plugin_dirs: Arc::new(std::sync::Mutex::new(Vec::new())),
            audio_dirs: Arc::new(std::sync::Mutex::new(Vec::new())),
            daw_dirs: Arc::new(std::sync::Mutex::new(Vec::new())),
            preset_dirs: Arc::new(std::sync::Mutex::new(Vec::new())),
            midi_dirs: Arc::new(std::sync::Mutex::new(Vec::new())),
            pdf_dirs: Arc::new(std::sync::Mutex::new(Vec::new())),
            unified_scanning: AtomicBool::new(false),
        })
        .manage(file_watcher::FileWatcherState::new())
        .manage(tray_menu::TrayState(std::sync::Mutex::new(None)))
        .invoke_handler(tauri::generate_handler![
            get_version,
            get_walker_status,
            scan_plugins,
            stop_scan,
            check_updates,
            stop_updates,
            resolve_kvr,
            history_get_scans,
            history_get_detail,
            history_delete,
            history_clear,
            history_diff,
            history_latest,
            kvr_cache_get,
            kvr_cache_update,
            scan_audio_samples,
            stop_audio_scan,
            get_audio_metadata,
            audio_history_save,
            audio_history_get_scans,
            audio_history_get_detail,
            audio_history_delete,
            audio_history_clear,
            audio_history_latest,
            audio_history_diff,
            scan_daw_projects,
            stop_daw_scan,
            daw_history_save,
            daw_history_get_scans,
            daw_history_get_detail,
            daw_history_delete,
            daw_history_clear,
            daw_history_latest,
            daw_history_diff,
            open_daw_folder,
            open_daw_project,
            extract_project_plugins,
            read_als_xml,
            estimate_bpm,
            detect_audio_key,
            measure_lufs,
            batch_analyze,
            read_cache_file,
            write_cache_file,
            audio_engine_invoke,
            audio_engine_restart,
            audio_engine_eof_watchdog_start,
            audio_engine_eof_watchdog_stop,
            get_audio_engine_process_stats,
            append_log,
            read_log,
            clear_log,
            list_data_files,
            delete_data_file,
            read_bwproject,
            read_project_file,
            compute_fingerprint,
            find_similar_samples,
            build_fingerprint_cache,
            find_content_duplicates,
            open_update_url,
            open_plugin_folder,
            open_audio_folder,
            export_plugins_json,
            export_plugins_csv,
            import_plugins_json,
            export_audio_json,
            export_audio_dsv,
            export_daw_json,
            export_daw_dsv,
            prefs_get_all,
            prefs_set,
            prefs_remove,
            prefs_save_all,
            scan_presets,
            stop_preset_scan,
            preset_history_save,
            preset_history_get_scans,
            preset_history_get_detail,
            preset_history_delete,
            preset_history_clear,
            preset_history_latest,
            preset_history_diff,
            open_preset_folder,
            scan_midi_files,
            stop_midi_scan,
            midi_history_save,
            midi_history_get_scans,
            midi_history_get_detail,
            midi_history_delete,
            midi_history_clear,
            midi_history_latest,
            midi_history_diff,
            db_query_midi,
            db_midi_filter_stats,
            scan_pdfs,
            stop_pdf_scan,
            scan_unified,
            get_unified_scan_run,
            prepare_unified_scan,
            stop_unified_scan,
            pdf_history_save,
            pdf_history_get_scans,
            pdf_history_get_detail,
            pdf_history_delete,
            pdf_history_clear,
            pdf_history_latest,
            pdf_history_diff,
            open_pdf_file,
            pdf_metadata_get,
            pdf_metadata_extract_abort,
            pdf_metadata_extract_batch,
            pdf_metadata_unindexed,
            open_file_default,
            export_presets_json,
            export_presets_dsv,
            export_pdfs_json,
            export_pdfs_dsv,
            import_pdfs_json,
            export_toml,
            import_toml,
            export_pdf,
            import_presets_json,
            import_audio_json,
            import_daw_json,
            open_with_app,
            fs_list_dir,
            delete_file,
            rename_file,
            write_text_file,
            read_text_file,
            get_home_dir,
            get_process_stats,
            open_prefs_file,
            get_prefs_path,
            db_query_audio,
            db_query_plugins,
            db_query_daw,
            db_query_presets,
            db_audio_stats,
            db_daw_stats,
            db_preset_stats,
            db_query_pdfs,
            db_query_palette_preview,
            db_pdf_stats,
            db_audio_filter_stats,
            db_daw_filter_stats,
            db_preset_filter_stats,
            db_plugin_filter_stats,
            db_pdf_filter_stats,
            get_active_scan_inventory_counts,
            db_list_scans,
            db_update_bpm,
            db_update_key,
            db_update_lufs,
            db_update_analysis,
            db_backfill_audio_meta,
            db_get_analysis,
            db_unanalyzed_paths,
            db_audio_library_paths,
            db_migrate_json,
            db_cache_stats,
            db_clear_caches,
            db_clear_cache_table,
            get_app_strings,
            get_toast_strings,
            refresh_native_menu,
            tray_menu::update_tray_now_playing,
            start_file_watcher,
            stop_file_watcher,
            get_file_watcher_status,
            get_midi_info,
        ])
        .setup(|app| {
            // Restore window size/position
            let prefs = history::load_preferences();
            if let Some(win_val) = prefs.get("window") {
                if let Some(win) = app.get_webview_window("main") {
                    if let Some(w) = win_val.get("width").and_then(|v| v.as_u64()) {
                        if let Some(h) = win_val.get("height").and_then(|v| v.as_u64()) {
                            let size = tauri::PhysicalSize::new(w as u32, h as u32);
                            let _ = win.set_size(tauri::Size::Physical(size));
                        }
                    }
                    if let Some(x) = win_val.get("x").and_then(|v| v.as_i64()) {
                        if let Some(y) = win_val.get("y").and_then(|v| v.as_i64()) {
                            let pos = tauri::PhysicalPosition::new(x as i32, y as i32);
                            let _ = win.set_position(tauri::Position::Physical(pos));
                        }
                    }
                }
            }

            // Build menu bar
            let handle = app.handle();
            let ui_locale = prefs
                .get("uiLocale")
                .and_then(|v| v.as_str().map(|s| s.to_string()))
                .unwrap_or_else(|| "en".to_string());
            let strings = db::global().get_app_strings(&ui_locale).unwrap_or_default();
            let menu =
                native_menu::build_native_menu_bar(handle, &strings).map_err(|e| e.to_string())?;
            app.set_menu(menu).map_err(|e| e.to_string())?;

            // Handle menu events — emit to frontend JS
            let handle2 = app.handle().clone();
            app.on_menu_event(move |_app, event| {
                let id = event.id().0.as_str();
                if let Some(win) = handle2.get_webview_window("main") {
                    let _ = win.emit("menu-action", id);
                }
            });

            let tray = tray_menu::create_tray(app, &strings)?;
            {
                let state = app.state::<tray_menu::TrayState>();
                let mut guard = state
                    .0
                    .lock()
                    .map_err(|_| "tray state mutex poisoned".to_string())?;
                *guard = Some(tray);
            }

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|_app, event| match event {
            tauri::RunEvent::ExitRequested { .. } | tauri::RunEvent::Exit => {
                let _ = audio_engine::shutdown_audio_engine_child();
                log_shutdown();
            }
            _ => {}
        });
}

#[cfg(test)]
mod log_verbosity_tests {
    use super::{app_log_verbose, log_verbosity_level, should_suppress_app_log_line, LOG_VERBOSITY_LEVEL};
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn quiet_filter_is_opt_in_prefix_list() {
        LOG_VERBOSITY_LEVEL.store(0, Ordering::Relaxed);
        assert!(!should_suppress_app_log_line("SCAN ERROR — daw | x"));
        assert!(!should_suppress_app_log_line("SCAN TCC DENIED — unified | x"));
        LOG_VERBOSITY_LEVEL.store(1, Ordering::Relaxed);
    }

    #[test]
    fn app_log_verbose_skips_closure_when_not_verbose() {
        let calls = AtomicUsize::new(0);
        LOG_VERBOSITY_LEVEL.store(1, Ordering::Relaxed);
        app_log_verbose(|| {
            calls.fetch_add(1, Ordering::Relaxed);
            "should not run".to_string()
        });
        assert_eq!(calls.load(Ordering::Relaxed), 0);
        LOG_VERBOSITY_LEVEL.store(2, Ordering::Relaxed);
        app_log_verbose(|| {
            calls.fetch_add(1, Ordering::Relaxed);
            "should run".to_string()
        });
        assert_eq!(calls.load(Ordering::Relaxed), 1);
        LOG_VERBOSITY_LEVEL.store(1, Ordering::Relaxed);
    }

    #[test]
    fn log_verbosity_level_tracks_atomic() {
        LOG_VERBOSITY_LEVEL.store(2, Ordering::Relaxed);
        assert_eq!(log_verbosity_level(), 2);
        LOG_VERBOSITY_LEVEL.store(1, Ordering::Relaxed);
    }
}

fn log_shutdown() {
    use std::sync::atomic::{AtomicBool, Ordering};
    static LOGGED: AtomicBool = AtomicBool::new(false);
    if LOGGED.swap(true, Ordering::Relaxed) {
        return;
    } // only log once
    let uptime = APP_START.get().map(|s| s.elapsed().as_secs()).unwrap_or(0);
    append_log(format!(
        "APP SHUTDOWN — uptime {}m {}s",
        uptime / 60,
        uptime % 60
    ));
}
