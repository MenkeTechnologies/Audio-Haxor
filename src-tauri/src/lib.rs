//! AUDIO_HAXOR — Tauri v2 desktop app for audio plugin management.
//!
//! This crate provides the Rust backend for scanning audio plugins (VST2/VST3/AU),
//! audio samples, DAW project files, and presets. It includes KVR Audio version
//! checking, scan history with diffing, and export to JSON/TOML/CSV/TSV/PDF.
//!
//! # Modules
//!
//! - [`scanner`] — Plugin filesystem scanner with architecture detection
//! - [`audio_scanner`] — Audio sample discovery and metadata extraction
//! - [`daw_scanner`] — DAW project scanner (14+ formats)
//! - [`preset_scanner`] — Plugin preset discovery
//! - [`kvr`] — KVR Audio scraper and version checker
//! - [`history`] — Scan history persistence, diffing, and preferences

pub mod audio_scanner;
pub mod bpm;
pub mod daw_scanner;
pub mod similarity;
pub mod history;
pub mod key_detect;
pub mod lufs;
pub mod kvr;
pub mod preset_scanner;
pub mod scanner;
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

use history::{AudioSample, DawProject, KvrCacheUpdateEntry, PresetFile};
use scanner::PluginInfo;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager};

// ── Export / Import types ──

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ExportPayload {
    version: String,
    exported_at: String,
    plugins: Vec<ExportPlugin>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ExportPlugin {
    name: String,
    #[serde(rename = "type")]
    plugin_type: String,
    version: String,
    manufacturer: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    manufacturer_url: Option<String>,
    path: String,
    size: String,
    #[serde(rename = "sizeBytes", default)]
    size_bytes: u64,
    modified: String,
    #[serde(default)]
    architectures: Vec<String>,
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

/// Tracks active directory paths being walked by each scanner for live status display.
struct WalkerStatus {
    plugin_dirs: Arc<std::sync::Mutex<Vec<String>>>,
    audio_dirs: Arc<std::sync::Mutex<Vec<String>>>,
    daw_dirs: Arc<std::sync::Mutex<Vec<String>>>,
    preset_dirs: Arc<std::sync::Mutex<Vec<String>>>,
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

// ── Tauri commands ──

#[tauri::command]
fn get_version(app: AppHandle) -> String {
    app.package_info().version.to_string()
}

#[tauri::command]
fn get_walker_status(app: AppHandle) -> serde_json::Value {
    let ws = app.state::<WalkerStatus>();
    let plugin = ws.plugin_dirs.lock().unwrap_or_else(|e| e.into_inner()).clone();
    let audio = ws.audio_dirs.lock().unwrap_or_else(|e| e.into_inner()).clone();
    let daw = ws.daw_dirs.lock().unwrap_or_else(|e| e.into_inner()).clone();
    let preset = ws.preset_dirs.lock().unwrap_or_else(|e| e.into_inner()).clone();
    let pool_threads = num_cpus::get().max(4);
    let plugin_scanning = app.state::<ScanState>().scanning.load(Ordering::Relaxed);
    let audio_scanning = app.state::<AudioScanState>().scanning.load(Ordering::Relaxed);
    let daw_scanning = app.state::<DawScanState>().scanning.load(Ordering::Relaxed);
    let preset_scanning = app.state::<PresetScanState>().scanning.load(Ordering::Relaxed);
    serde_json::json!({
        "plugin": plugin,
        "audio": audio,
        "daw": daw,
        "preset": preset,
        "poolThreads": pool_threads,
        "pluginScanning": plugin_scanning,
        "audioScanning": audio_scanning,
        "dawScanning": daw_scanning,
        "presetScanning": preset_scanning,
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
        let plugin_paths = scanner::discover_plugins(&directories);
        let total = plugin_paths.len();

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
                eprintln!("Thread pool creation failed ({e}), retrying with 2 threads");
                rayon::ThreadPoolBuilder::new().num_threads(2).build()
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
                        if ad.len() > 30 { let excess = ad.len() - 30; ad.drain(..excess); }
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
        let snapshot = history::save_scan(&all_plugins, &directories, &roots);

        serde_json::json!({
            "plugins": all_plugins,
            "directories": directories,
            "snapshotId": snapshot.id,
            "stopped": was_stopped
        })
    })
    .await;

    state.scanning.store(false, Ordering::SeqCst);
    { let ws = app.state::<WalkerStatus>(); let mut ad = ws.plugin_dirs.lock().unwrap_or_else(|e| e.into_inner()); ad.clear(); }
    result.map_err(|e| e.to_string())
}

#[tauri::command]
async fn stop_scan(app: AppHandle) -> Result<(), String> {
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
    let state = app.state::<UpdateState>();
    state.stop_updates.store(true, Ordering::SeqCst);
    Ok(())
}

#[tauri::command]
async fn resolve_kvr(direct_url: String, plugin_name: String) -> Result<kvr::KvrResult, String> {
    Ok(kvr::resolve_kvr(&direct_url, &plugin_name).await)
}

// History commands
#[tauri::command]
fn history_get_scans() -> Vec<history::ScanSummary> {
    history::get_scans()
}

#[tauri::command]
fn history_get_detail(id: String) -> Option<history::ScanSnapshot> {
    history::get_scan_detail(&id)
}

#[tauri::command]
fn history_delete(id: String) {
    history::delete_scan(&id);
}

#[tauri::command]
fn history_clear() {
    history::clear_history();
}

#[tauri::command]
fn history_diff(old_id: String, new_id: String) -> Option<history::ScanDiff> {
    history::diff_scans(&old_id, &new_id)
}

#[tauri::command]
fn history_latest() -> Option<history::ScanSnapshot> {
    history::get_latest_scan()
}

#[tauri::command]
fn kvr_cache_get() -> std::collections::HashMap<String, history::KvrCacheEntry> {
    history::load_kvr_cache()
}

#[tauri::command]
fn kvr_cache_update(entries: Vec<KvrCacheUpdateEntry>) {
    history::update_kvr_cache(&entries);
}

// Audio scanner commands
#[tauri::command]
async fn scan_audio_samples(
    app: AppHandle,
    custom_roots: Option<Vec<String>>,
    exclude_paths: Option<Vec<String>>,
) -> Result<serde_json::Value, String> {
    let state = app.state::<AudioScanState>();
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
        let mut all_samples: Vec<AudioSample> = Vec::new();
        let exclude_set = exclude_paths.map(|v| v.into_iter().collect::<HashSet<String>>());

        audio_scanner::walk_for_audio(
            &roots,
            &mut |batch, found| {
                all_samples.extend_from_slice(batch);
                let _ = app_handle.emit(
                    "audio-scan-progress",
                    serde_json::json!({
                        "phase": "scanning",
                        "samples": batch,
                        "found": found
                    }),
                );
            },
            &|| audio_state.stop_scan.load(Ordering::SeqCst),
            exclude_set,
            Some(Arc::clone(&app_handle.state::<WalkerStatus>().audio_dirs)),
        );

        // Clear walker status
        { let ws = app_handle.state::<WalkerStatus>(); let mut ad = ws.audio_dirs.lock().unwrap_or_else(|e| e.into_inner()); ad.clear(); }

        let root_strs: Vec<String> = roots
            .iter()
            .map(|r| r.to_string_lossy().to_string())
            .collect();
        let was_stopped = audio_state.stop_scan.load(Ordering::Relaxed);
        all_samples.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        serde_json::json!({ "samples": all_samples, "roots": root_strs, "stopped": was_stopped })
    })
    .await;

    state.scanning.store(false, Ordering::SeqCst);
    result.map_err(|e| e.to_string())
}

#[tauri::command]
async fn stop_audio_scan(app: AppHandle) -> Result<(), String> {
    let state = app.state::<AudioScanState>();
    state.stop_scan.store(true, Ordering::SeqCst);
    Ok(())
}

#[tauri::command]
fn get_audio_metadata(file_path: String) -> audio_scanner::AudioMetadata {
    audio_scanner::get_audio_metadata(&file_path)
}

// Audio history commands
#[tauri::command]
fn audio_history_save(
    samples: Vec<AudioSample>,
    roots: Option<Vec<String>>,
) -> history::AudioScanSnapshot {
    history::save_audio_scan(&samples, &roots.unwrap_or_default())
}

#[tauri::command]
fn audio_history_get_scans() -> Vec<history::AudioScanSummary> {
    history::get_audio_scans()
}

#[tauri::command]
fn audio_history_get_detail(id: String) -> Option<history::AudioScanSnapshot> {
    history::get_audio_scan_detail(&id)
}

#[tauri::command]
fn audio_history_delete(id: String) {
    history::delete_audio_scan(&id);
}

#[tauri::command]
fn audio_history_clear() {
    history::clear_audio_history();
}

#[tauri::command]
fn audio_history_latest() -> Option<history::AudioScanSnapshot> {
    history::get_latest_audio_scan()
}

#[tauri::command]
fn audio_history_diff(old_id: String, new_id: String) -> Option<history::AudioScanDiff> {
    history::diff_audio_scans(&old_id, &new_id)
}

// DAW scanner commands
#[tauri::command]
async fn scan_daw_projects(
    app: AppHandle,
    custom_roots: Option<Vec<String>>,
    exclude_paths: Option<Vec<String>>,
) -> Result<serde_json::Value, String> {
    let state = app.state::<DawScanState>();
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
        let mut all_projects: Vec<DawProject> = Vec::new();
        let exclude_set = exclude_paths.map(|v| v.into_iter().collect::<HashSet<String>>());

        daw_scanner::walk_for_daw(
            &roots,
            &mut |batch, found| {
                all_projects.extend_from_slice(batch);
                let _ = app_handle.emit(
                    "daw-scan-progress",
                    serde_json::json!({
                        "phase": "scanning",
                        "projects": batch,
                        "found": found
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
        );

        { let ws = app_handle.state::<WalkerStatus>(); let mut ad = ws.daw_dirs.lock().unwrap_or_else(|e| e.into_inner()); ad.clear(); }
        let root_strs: Vec<String> = roots
            .iter()
            .map(|r| r.to_string_lossy().to_string())
            .collect();
        let was_stopped = daw_state.stop_scan.load(Ordering::Relaxed);
        all_projects.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        serde_json::json!({ "projects": all_projects, "roots": root_strs, "stopped": was_stopped })
    })
    .await;

    state.scanning.store(false, Ordering::SeqCst);
    result.map_err(|e| e.to_string())
}

#[tauri::command]
async fn stop_daw_scan(app: AppHandle) -> Result<(), String> {
    let state = app.state::<DawScanState>();
    state.stop_scan.store(true, Ordering::SeqCst);
    Ok(())
}

// DAW history commands
#[tauri::command]
fn daw_history_save(
    projects: Vec<DawProject>,
    roots: Option<Vec<String>>,
) -> history::DawScanSnapshot {
    history::save_daw_scan(&projects, &roots.unwrap_or_default())
}

#[tauri::command]
fn daw_history_get_scans() -> Vec<history::DawScanSummary> {
    history::get_daw_scans()
}

#[tauri::command]
fn daw_history_get_detail(id: String) -> Option<history::DawScanSnapshot> {
    history::get_daw_scan_detail(&id)
}

#[tauri::command]
fn daw_history_delete(id: String) {
    history::delete_daw_scan(&id);
}

#[tauri::command]
fn daw_history_clear() {
    history::clear_daw_history();
}

#[tauri::command]
fn daw_history_latest() -> Option<history::DawScanSnapshot> {
    history::get_latest_daw_scan()
}

#[tauri::command]
fn daw_history_diff(old_id: String, new_id: String) -> Option<history::DawScanDiff> {
    history::diff_daw_scans(&old_id, &new_id)
}

// Preset scanner commands
#[tauri::command]
async fn scan_presets(
    app: AppHandle,
    custom_roots: Option<Vec<String>>,
    exclude_paths: Option<Vec<String>>,
) -> Result<serde_json::Value, String> {
    let state = app.state::<PresetScanState>();
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
        let mut all_presets: Vec<PresetFile> = Vec::new();
        let exclude_set = exclude_paths.map(|v| v.into_iter().collect::<HashSet<String>>());

        preset_scanner::walk_for_presets(
            &roots,
            &mut |batch, found| {
                all_presets.extend_from_slice(batch);
                let _ = app_handle.emit(
                    "preset-scan-progress",
                    serde_json::json!({
                        "phase": "scanning",
                        "presets": batch,
                        "found": found
                    }),
                );
            },
            &|| preset_state.stop_scan.load(Ordering::SeqCst),
            exclude_set,
            Some(Arc::clone(&app_handle.state::<WalkerStatus>().preset_dirs)),
        );

        { let ws = app_handle.state::<WalkerStatus>(); let mut ad = ws.preset_dirs.lock().unwrap_or_else(|e| e.into_inner()); ad.clear(); }
        let was_stopped = preset_state.stop_scan.load(Ordering::Relaxed);
        let root_strs: Vec<String> = roots
            .iter()
            .map(|r| r.to_string_lossy().to_string())
            .collect();
        all_presets.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        serde_json::json!({ "presets": all_presets, "roots": root_strs, "stopped": was_stopped })
    })
    .await;

    state.scanning.store(false, Ordering::SeqCst);
    result.map_err(|e| e.to_string())
}

#[tauri::command]
async fn stop_preset_scan(app: AppHandle) -> Result<(), String> {
    let state = app.state::<PresetScanState>();
    state.stop_scan.store(true, Ordering::SeqCst);
    Ok(())
}

// Preset history commands
#[tauri::command]
fn preset_history_save(
    presets: Vec<PresetFile>,
    roots: Option<Vec<String>>,
) -> history::PresetScanSnapshot {
    history::save_preset_scan(&presets, &roots.unwrap_or_default())
}

#[tauri::command]
fn preset_history_get_scans() -> Vec<history::PresetScanSummary> {
    history::get_preset_scans()
}

#[tauri::command]
fn preset_history_get_detail(id: String) -> Option<history::PresetScanSnapshot> {
    history::get_preset_scan_detail(&id)
}

#[tauri::command]
fn preset_history_delete(id: String) {
    history::delete_preset_scan(&id);
}

#[tauri::command]
fn preset_history_clear() {
    history::clear_preset_history();
}

#[tauri::command]
fn preset_history_latest() -> Option<history::PresetScanSnapshot> {
    history::get_latest_preset_scan()
}

#[tauri::command]
fn preset_history_diff(old_id: String, new_id: String) -> Option<history::PresetScanDiff> {
    history::diff_preset_scans(&old_id, &new_id)
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
    Ok(xref::extract_plugins(&file_path))
}

#[tauri::command]
fn read_als_xml(file_path: String) -> Result<String, String> {
    use flate2::read::GzDecoder;
    use std::io::Read;
    let data = std::fs::read(&file_path).map_err(|e| e.to_string())?;
    let mut decoder = GzDecoder::new(&data[..]);
    let mut xml = String::new();
    decoder
        .read_to_string(&mut xml)
        .map_err(|e| format!("Not a valid gzip file: {}", e))?;
    Ok(xml)
}

#[tauri::command]
async fn estimate_bpm(file_path: String) -> Result<Option<f64>, String> {
    Ok(bpm::estimate_bpm(&file_path))
}

#[tauri::command]
async fn detect_audio_key(file_path: String) -> Result<Option<String>, String> {
    Ok(tokio::task::spawn_blocking(move || {
        key_detect::detect_key(&file_path)
    }).await.map_err(|e| e.to_string())?)
}

#[tauri::command]
async fn measure_lufs(file_path: String) -> Result<Option<f64>, String> {
    Ok(tokio::task::spawn_blocking(move || {
        lufs::measure_lufs(&file_path)
    }).await.map_err(|e| e.to_string())?)
}

#[tauri::command]
async fn compute_fingerprint(file_path: String) -> Result<Option<similarity::AudioFingerprint>, String> {
    Ok(tokio::task::spawn_blocking(move || {
        similarity::compute_fingerprint(&file_path)
    }).await.map_err(|e| e.to_string())?)
}

#[tauri::command]
async fn find_similar_samples(
    app: AppHandle,
    file_path: String,
    candidate_paths: Vec<String>,
    max_results: usize,
) -> Result<Vec<serde_json::Value>, String> {
    Ok(tokio::task::spawn_blocking(move || {
        // Load cached fingerprints
        let cache_file = history::get_data_dir().join("fingerprint-cache.json");
        let mut cache: std::collections::HashMap<String, similarity::AudioFingerprint> =
            std::fs::read_to_string(&cache_file)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default();

        // Compute reference fingerprint (use cache if available)
        let reference = if let Some(fp) = cache.get(&file_path) {
            fp.clone()
        } else {
            match similarity::compute_fingerprint(&file_path) {
                Some(fp) => { cache.insert(file_path.clone(), fp.clone()); fp }
                None => return vec![],
            }
        };

        // Compute missing fingerprints in parallel
        use rayon::prelude::*;
        let uncached: Vec<&String> = candidate_paths.iter()
            .filter(|p| !cache.contains_key(p.as_str()))
            .collect();

        if !uncached.is_empty() {
            // Emit progress
            let total = uncached.len();
            let _ = app.emit("similarity-progress", serde_json::json!({
                "phase": "computing", "total": total, "cached": candidate_paths.len() - total
            }));

            let new_fps: Vec<similarity::AudioFingerprint> = uncached
                .par_iter()
                .filter_map(|p| similarity::compute_fingerprint(p))
                .collect();

            for fp in new_fps {
                cache.insert(fp.path.clone(), fp);
            }

            // Save cache to disk
            if let Ok(json) = serde_json::to_string(&cache) {
                let _ = std::fs::write(&cache_file, json);
            }
        }

        // Collect cached fingerprints for candidates
        let candidates: Vec<similarity::AudioFingerprint> = candidate_paths
            .iter()
            .filter_map(|p| cache.get(p).cloned())
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
    }).await.map_err(|e| e.to_string())?)
}

#[tauri::command]
fn open_with_app(file_path: String, app_name: String) -> Result<(), String> {
    let path = std::path::Path::new(&file_path);
    if !path.exists() {
        return Err(format!("File not found: {}", file_path));
    }

    #[cfg(target_os = "macos")]
    {
        let output = std::process::Command::new("open")
            .args(["-a", &app_name, &file_path])
            .output()
            .map_err(|e| e.to_string())?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!(
                "Could not open with {}: {}",
                app_name,
                stderr.trim()
            ));
        }
    }

    #[cfg(target_os = "windows")]
    {
        let output = std::process::Command::new("cmd")
            .args(["/C", "start", "", &file_path])
            .output()
            .map_err(|e| e.to_string())?;
        if !output.status.success() {
            return Err(format!("Could not open with {}", app_name));
        }
    }

    #[cfg(target_os = "linux")]
    {
        let output = std::process::Command::new("xdg-open")
            .arg(&file_path)
            .output()
            .map_err(|e| e.to_string())?;
        if !output.status.success() {
            return Err(format!("Could not open with {}", app_name));
        }
    }

    Ok(())
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
fn prefs_get_all() -> history::PrefsMap {
    history::load_preferences()
}

#[tauri::command]
fn prefs_set(key: String, value: serde_json::Value) {
    history::set_preference(&key, value);
}

#[tauri::command]
fn prefs_remove(key: String) {
    history::remove_preference(&key);
}

#[tauri::command]
fn prefs_save_all(prefs: history::PrefsMap) {
    history::save_preferences(&prefs);
}

#[tauri::command]
async fn open_prefs_file() -> Result<(), String> {
    let path = history::get_preferences_path();
    opener::open(&path).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_prefs_path() -> String {
    history::get_preferences_path()
        .to_string_lossy()
        .to_string()
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
fn export_plugins_json(plugins: Vec<PluginInfo>, file_path: String) -> Result<(), String> {
    let payload = ExportPayload {
        version: env!("CARGO_PKG_VERSION").into(),
        exported_at: chrono::Utc::now().to_rfc3339(),
        plugins: plugins_to_export(&plugins),
    };
    let json = serde_json::to_string_pretty(&payload).map_err(|e| e.to_string())?;
    std::fs::write(&file_path, json).map_err(|e| e.to_string())
}

#[tauri::command]
fn export_plugins_csv(plugins: Vec<PluginInfo>, file_path: String) -> Result<(), String> {
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
fn export_audio_json(samples: Vec<history::AudioSample>, file_path: String) -> Result<(), String> {
    let payload = serde_json::json!({
        "version": env!("CARGO_PKG_VERSION"),
        "exported_at": chrono::Utc::now().to_rfc3339(),
        "samples": samples,
    });
    let json = serde_json::to_string_pretty(&payload).map_err(|e| e.to_string())?;
    std::fs::write(&file_path, json).map_err(|e| e.to_string())
}

#[tauri::command]
fn export_audio_dsv(samples: Vec<history::AudioSample>, file_path: String) -> Result<(), String> {
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
}

// ── DAW export ──

#[tauri::command]
fn export_daw_json(projects: Vec<history::DawProject>, file_path: String) -> Result<(), String> {
    let payload = serde_json::json!({
        "version": env!("CARGO_PKG_VERSION"),
        "exported_at": chrono::Utc::now().to_rfc3339(),
        "projects": projects,
    });
    let json = serde_json::to_string_pretty(&payload).map_err(|e| e.to_string())?;
    std::fs::write(&file_path, json).map_err(|e| e.to_string())
}

#[tauri::command]
fn export_daw_dsv(projects: Vec<history::DawProject>, file_path: String) -> Result<(), String> {
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
}

#[tauri::command]
fn import_plugins_json(file_path: String) -> Result<Vec<PluginInfo>, String> {
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
}

// ── Process stats ──

use std::sync::atomic::AtomicU64;
use std::time::Instant;

static LAST_CPU_TIME: AtomicU64 = AtomicU64::new(0);
static LAST_WALL_TIME: AtomicU64 = AtomicU64::new(0);

#[tauri::command]
fn get_process_stats(app: AppHandle) -> serde_json::Value {
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
        .unwrap_or(500);

    // Data file sizes
    let data_dir = history::get_data_dir();
    let file_size = |name: &str| -> u64 {
        std::fs::metadata(data_dir.join(name))
            .map(|m| m.len())
            .unwrap_or(0)
    };

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
        "dataFiles": {
            "preferencesBytes": file_size("preferences.toml"),
            "scanHistoryBytes": file_size("scan-history.json"),
            "audioHistoryBytes": file_size("audio-scan-history.json"),
            "dawHistoryBytes": file_size("daw-scan-history.json"),
            "presetHistoryBytes": file_size("preset-scan-history.json"),
            "kvrCacheBytes": file_size("kvr-cache.json"),
        },
        "dataDir": data_dir.to_string_lossy(),
    })
}

static APP_START: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();

fn get_uptime_secs() -> u64 {
    APP_START.get_or_init(Instant::now).elapsed().as_secs()
}

#[cfg(target_os = "macos")]
#[allow(deprecated)]
fn get_rss_bytes() -> u64 {
    unsafe {
        let mut info: libc::mach_task_basic_info_data_t = std::mem::zeroed();
        let mut count = (std::mem::size_of::<libc::mach_task_basic_info_data_t>()
            / std::mem::size_of::<libc::natural_t>()) as u32;
        let kr = libc::task_info(
            libc::mach_task_self(),
            libc::MACH_TASK_BASIC_INFO,
            &mut info as *mut _ as *mut i32,
            &mut count,
        );
        if kr == libc::KERN_SUCCESS {
            info.resident_size as u64
        } else {
            0
        }
    }
}

#[cfg(target_os = "linux")]
fn get_rss_bytes() -> u64 {
    read_proc_field("VmRSS:").map(|kb| kb * 1024).unwrap_or(0)
}

#[cfg(target_os = "windows")]
fn get_rss_bytes() -> u64 {
    0
}

#[cfg(target_os = "macos")]
#[allow(deprecated)]
fn get_virtual_bytes() -> u64 {
    unsafe {
        let mut info: libc::mach_task_basic_info_data_t = std::mem::zeroed();
        let mut count = (std::mem::size_of::<libc::mach_task_basic_info_data_t>()
            / std::mem::size_of::<libc::natural_t>()) as u32;
        let kr = libc::task_info(
            libc::mach_task_self(),
            libc::MACH_TASK_BASIC_INFO,
            &mut info as *mut _ as *mut i32,
            &mut count,
        );
        if kr == libc::KERN_SUCCESS {
            info.virtual_size as u64
        } else {
            0
        }
    }
}

#[cfg(target_os = "linux")]
fn get_virtual_bytes() -> u64 {
    read_proc_field("VmSize:").map(|kb| kb * 1024).unwrap_or(0)
}

#[cfg(target_os = "windows")]
fn get_virtual_bytes() -> u64 {
    0
}

fn get_thread_count() -> u32 {
    #[cfg(target_os = "macos")]
    {
        // Read thread count from proc_pidinfo
        let pid = std::process::id();
        let output = std::process::Command::new("ps")
            .args(["-M", "-p", &pid.to_string()])
            .output();
        if let Ok(out) = output {
            let s = String::from_utf8_lossy(&out.stdout);
            return s.lines().count().saturating_sub(1) as u32;
        }
        0
    }
    #[cfg(target_os = "linux")]
    {
        read_proc_field("Threads:").map(|v| v as u32).unwrap_or(0)
    }
    #[cfg(target_os = "windows")]
    {
        0
    }
}

#[cfg(target_os = "macos")]
#[allow(deprecated)]
fn get_cpu_percent() -> f64 {
    unsafe {
        let mut info: libc::task_thread_times_info_data_t = std::mem::zeroed();
        let mut count = (std::mem::size_of::<libc::task_thread_times_info_data_t>()
            / std::mem::size_of::<libc::natural_t>()) as u32;
        let kr = libc::task_info(
            libc::mach_task_self(),
            libc::TASK_THREAD_TIMES_INFO,
            &mut info as *mut _ as *mut i32,
            &mut count,
        );
        if kr != libc::KERN_SUCCESS {
            return 0.0;
        }
        let user_us =
            info.user_time.seconds as u64 * 1_000_000 + info.user_time.microseconds as u64;
        let sys_us =
            info.system_time.seconds as u64 * 1_000_000 + info.system_time.microseconds as u64;
        let total_us = user_us + sys_us;

        let prev = LAST_CPU_TIME.swap(total_us, Ordering::Relaxed);
        let now_ns = Instant::now()
            .duration_since(*APP_START.get_or_init(Instant::now))
            .as_micros() as u64;
        let prev_wall = LAST_WALL_TIME.swap(now_ns, Ordering::Relaxed);

        let wall_delta = now_ns.saturating_sub(prev_wall);
        let cpu_delta = total_us.saturating_sub(prev);

        if wall_delta == 0 {
            return 0.0;
        }
        let ncpus = num_cpus::get() as f64;
        let pct = (cpu_delta as f64 / wall_delta as f64) * 100.0 / ncpus;
        (pct * 10.0).round() / 10.0
    }
}

#[cfg(target_os = "linux")]
fn get_cpu_percent() -> f64 {
    // Read from /proc/self/stat: utime + stime in clock ticks
    if let Ok(stat) = std::fs::read_to_string("/proc/self/stat") {
        let parts: Vec<&str> = stat.split_whitespace().collect();
        if parts.len() > 14 {
            let utime = parts[13].parse::<u64>().unwrap_or(0);
            let stime = parts[14].parse::<u64>().unwrap_or(0);
            let ticks_per_sec = unsafe { libc::sysconf(libc::_SC_CLK_TCK) } as u64;
            let total_us = (utime + stime) * 1_000_000 / ticks_per_sec.max(1);

            let prev = LAST_CPU_TIME.swap(total_us, Ordering::Relaxed);
            let now_ns = Instant::now()
                .duration_since(*APP_START.get_or_init(Instant::now))
                .as_micros() as u64;
            let prev_wall = LAST_WALL_TIME.swap(now_ns, Ordering::Relaxed);

            let wall_delta = now_ns.saturating_sub(prev_wall);
            let cpu_delta = total_us.saturating_sub(prev);

            if wall_delta > 0 {
                let ncpus = num_cpus::get() as f64;
                let pct = (cpu_delta as f64 / wall_delta as f64) * 100.0 / ncpus;
                return (pct * 10.0).round() / 10.0;
            }
        }
    }
    0.0
}

#[cfg(target_os = "windows")]
fn get_cpu_percent() -> f64 {
    0.0
}

fn get_open_fd_count() -> u32 {
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        if let Ok(entries) = std::fs::read_dir("/dev/fd") {
            return entries.count() as u32;
        }
        0
    }
    #[cfg(target_os = "windows")]
    {
        0
    }
}

#[cfg(target_os = "linux")]
fn read_proc_field(field: &str) -> Option<u64> {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|s| {
            s.lines()
                .find(|l| l.starts_with(field))
                .and_then(|l| l.split_whitespace().nth(1))
                .and_then(|v| v.parse().ok())
        })
}

// ── Preset export/import ──

#[tauri::command]
fn export_presets_json(presets: Vec<PresetFile>, file_path: String) -> Result<(), String> {
    let json = serde_json::to_string_pretty(&presets).map_err(|e| e.to_string())?;
    std::fs::write(&file_path, json).map_err(|e| e.to_string())
}

#[tauri::command]
fn export_presets_dsv(presets: Vec<PresetFile>, file_path: String) -> Result<(), String> {
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
}

// ── TOML export (generic — works for all types via serde) ──

#[tauri::command]
fn export_toml(data: serde_json::Value, file_path: String) -> Result<(), String> {
    let toml_str = toml::to_string_pretty(&data).map_err(|e| e.to_string())?;
    std::fs::write(&file_path, toml_str).map_err(|e| e.to_string())
}

#[tauri::command]
fn import_toml(file_path: String) -> Result<serde_json::Value, String> {
    let data = std::fs::read_to_string(&file_path).map_err(|e| e.to_string())?;
    let val: toml::Value = toml::from_str(&data).map_err(|e| e.to_string())?;
    // Convert toml::Value to serde_json::Value
    let json_str = serde_json::to_string(&val).map_err(|e| e.to_string())?;
    serde_json::from_str(&json_str).map_err(|e| e.to_string())
}

// ── PDF export ──

#[tauri::command]
fn export_pdf(
    title: String,
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
    file_path: String,
) -> Result<(), String> {
    use printpdf::path::{PaintMode, WindingOrder};
    use printpdf::*;

    let page_w = Mm(297.0); // A4 landscape
    let page_h = Mm(210.0);
    let margin_x = 10.0_f32;
    let _margin_top = 12.0_f32;
    let margin_bottom = 12.0_f32;
    let row_height = 4.5_f32;
    let header_row_h = 7.0_f32;
    let col_count = headers.len();
    let usable_w = page_w.0 - margin_x * 2.0;

    // Calculate column widths based on 90th percentile content length
    let col_widths: Vec<f32> = if col_count > 0 {
        // Collect all cell lengths per column
        let mut col_lens: Vec<Vec<usize>> = headers.iter().map(|h| vec![h.len()]).collect();
        for row in &rows {
            for (i, cell) in row.iter().enumerate() {
                if i < col_lens.len() {
                    col_lens[i].push(cell.len());
                }
            }
        }
        // Use 90th percentile length (not max) to avoid outlier skew
        let p90_lens: Vec<usize> = col_lens
            .iter()
            .map(|lens| {
                let mut sorted = lens.clone();
                sorted.sort();
                let idx = (sorted.len() as f32 * 0.9).ceil() as usize;
                sorted[idx.min(sorted.len() - 1)].min(120)
            })
            .collect();
        // Ensure minimum representation for short columns (header len * 2)
        let effective: Vec<usize> = p90_lens
            .iter()
            .enumerate()
            .map(|(i, &l)| l.max(headers[i].len() * 2).max(6))
            .collect();
        let total_len: usize = effective.iter().sum::<usize>().max(1);
        let min_col = 12.0_f32;
        let mut widths: Vec<f32> = effective
            .iter()
            .map(|&l| (l as f32 / total_len as f32 * usable_w).max(min_col))
            .collect();
        // Normalize to fit usable_w exactly
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

    let (doc, page1, layer1) = PdfDocument::new(&title, page_w, page_h, "Layer 1");
    let font = doc
        .add_builtin_font(BuiltinFont::Helvetica)
        .map_err(|e| e.to_string())?;
    let font_bold = doc
        .add_builtin_font(BuiltinFont::HelveticaBold)
        .map_err(|e| e.to_string())?;
    let font_italic = doc
        .add_builtin_font(BuiltinFont::HelveticaOblique)
        .map_err(|e| e.to_string())?;

    let mut current_page = page1;
    let mut current_layer = layer1;
    let mut y: f32;
    let mut page_num = 1_usize;
    let mut row_idx = 0_usize;

    macro_rules! layer {
        () => {
            doc.get_page(current_page).get_layer(current_layer)
        };
    }

    // Color helper (Color doesn't impl Clone)
    fn rgb(r: f32, g: f32, b: f32) -> Color {
        Color::Rgb(Rgb::new(r, g, b, None))
    }

    #[allow(clippy::too_many_arguments)]
    fn fill_rect(
        layer: &PdfLayerReference,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        r: f32,
        g: f32,
        b: f32,
    ) {
        layer.set_fill_color(rgb(r, g, b));
        layer.set_outline_color(rgb(r, g, b));
        layer.set_outline_thickness(0.0);
        let points = vec![
            (Point::new(Mm(x), Mm(y)), false),
            (Point::new(Mm(x + w), Mm(y)), false),
            (Point::new(Mm(x + w), Mm(y + h)), false),
            (Point::new(Mm(x), Mm(y + h)), false),
        ];
        layer.add_polygon(Polygon {
            rings: vec![points],
            mode: PaintMode::FillStroke,
            winding_order: WindingOrder::NonZero,
        });
    }

    #[allow(clippy::too_many_arguments)]
    fn stroke_line(
        layer: &PdfLayerReference,
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        r: f32,
        g: f32,
        b: f32,
        thickness: f32,
    ) {
        layer.set_outline_color(rgb(r, g, b));
        layer.set_outline_thickness(thickness);
        let points = vec![
            (Point::new(Mm(x1), Mm(y1)), false),
            (Point::new(Mm(x2), Mm(y2)), false),
        ];
        layer.add_line(Line {
            points,
            is_closed: false,
        });
    }

    // ── Render page header ──
    let render_header = |layer_ref: &PdfLayerReference, y: &mut f32, page: usize| {
        // Dark header bar
        fill_rect(
            layer_ref,
            0.0,
            page_h.0 - 22.0,
            page_w.0,
            22.0,
            0.02,
            0.02,
            0.04,
        );

        // App name + music note (cyan)
        layer_ref.set_fill_color(rgb(0.02, 0.85, 0.91));
        layer_ref.use_text(
            "\u{266B} AUDIO_HAXOR",
            14.0,
            Mm(margin_x),
            Mm(page_h.0 - 14.0),
            &font_bold,
        );

        // Version (white)
        layer_ref.set_fill_color(rgb(1.0, 1.0, 1.0));
        layer_ref.use_text(
            format!("v{}", version),
            8.0,
            Mm(margin_x + 68.0),
            Mm(page_h.0 - 14.0),
            &font,
        );

        // Title on the right
        layer_ref.use_text(
            &title,
            12.0,
            Mm(page_w.0 - margin_x - 80.0),
            Mm(page_h.0 - 14.0),
            &font_bold,
        );

        // Cyan accent line under header
        stroke_line(
            layer_ref,
            0.0,
            page_h.0 - 22.0,
            page_w.0,
            page_h.0 - 22.0,
            0.02,
            0.85,
            0.91,
            1.5,
        );

        *y = page_h.0 - 28.0;

        // Subtitle (only on first page)
        if page == 1 {
            layer_ref.set_fill_color(rgb(0.55, 0.55, 0.55));
            let sub = format!(
                "{} items  |  Exported {}  |  by MenkeTechnologies",
                rows.len(),
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
            );
            layer_ref.use_text(&sub, 8.0, Mm(margin_x), Mm(*y), &font_italic);
            *y -= 6.0;
        }
    };

    // ── Render column headers ──
    let render_col_headers = |layer_ref: &PdfLayerReference, y: &mut f32| {
        // Header background
        fill_rect(
            layer_ref,
            margin_x - 1.0,
            *y - 1.5,
            usable_w + 2.0,
            header_row_h,
            0.93,
            0.93,
            0.93,
        );

        layer_ref.set_fill_color(rgb(0.15, 0.15, 0.15));
        let mut x = margin_x + 1.0;
        for (i, h) in headers.iter().enumerate() {
            layer_ref.use_text(h, 9.0, Mm(x), Mm(*y), &font_bold);
            x += col_widths[i];
        }
        *y -= header_row_h;
    };

    // ── Render footer ──
    let render_footer = |layer_ref: &PdfLayerReference, page: usize| {
        let footer_y = 8.0;
        // Thin gray line
        stroke_line(
            layer_ref,
            margin_x,
            footer_y + 3.0,
            page_w.0 - margin_x,
            footer_y + 3.0,
            0.8,
            0.8,
            0.8,
            0.3,
        );

        layer_ref.set_fill_color(rgb(0.55, 0.55, 0.55));
        layer_ref.use_text(
            format!("AUDIO_HAXOR v{} — {}", version, title),
            7.0,
            Mm(margin_x),
            Mm(footer_y),
            &font,
        );

        let page_str = format!("Page {}", page);
        layer_ref.use_text(
            &page_str,
            7.0,
            Mm(page_w.0 - margin_x - 25.0),
            Mm(footer_y),
            &font,
        );
    };

    // ── First page ──
    y = 0.0;
    render_header(&layer!(), &mut y, page_num);
    render_col_headers(&layer!(), &mut y);
    y -= 1.0;

    // ── Data rows ──
    for row in &rows {
        if y < margin_bottom + 5.0 {
            render_footer(&layer!(), page_num);
            let (new_page, new_layer) = doc.add_page(page_w, page_h, "Layer 1");
            current_page = new_page;
            current_layer = new_layer;
            page_num += 1;
            y = 0.0;
            render_header(&layer!(), &mut y, page_num);
            render_col_headers(&layer!(), &mut y);
            y -= 1.0;
            row_idx = 0;
        }

        // Alternating row stripe
        if row_idx % 2 == 1 {
            fill_rect(
                &layer!(),
                margin_x - 1.0,
                y - 1.2,
                usable_w + 2.0,
                row_height,
                0.96,
                0.96,
                0.98,
            );
        }

        layer!().set_fill_color(rgb(0.12, 0.12, 0.12));
        let mut x = margin_x + 0.5;
        for (i, cell) in row.iter().enumerate() {
            let w = if i < col_widths.len() {
                col_widths[i]
            } else {
                30.0
            };
            // At 7pt Helvetica, avg char width ~1.2mm
            let max_chars = (w / 1.2) as usize;
            let text = if cell.len() > max_chars && max_chars > 3 {
                format!("{}...", &cell[..max_chars - 3])
            } else {
                cell.clone()
            };
            layer!().use_text(&text, 7.0, Mm(x), Mm(y), &font);
            x += w;
        }

        y -= row_height;
        row_idx += 1;
    }

    // Final page footer
    render_footer(&layer!(), page_num);

    doc.save(&mut std::io::BufWriter::new(
        std::fs::File::create(&file_path).map_err(|e| e.to_string())?,
    ))
    .map_err(|e| e.to_string())
}

// ── File browser ──

#[tauri::command]
fn fs_list_dir(dir_path: String) -> Result<serde_json::Value, String> {
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
        } // skip hidden
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
    // Sort: dirs first, then by name
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
}

#[tauri::command]
fn delete_file(file_path: String) -> Result<(), String> {
    let path = std::path::Path::new(&file_path);
    if !path.exists() {
        return Err("File not found".into());
    }
    if path.is_dir() {
        std::fs::remove_dir_all(path).map_err(|e| e.to_string())
    } else {
        std::fs::remove_file(path).map_err(|e| e.to_string())
    }
}

#[tauri::command]
fn rename_file(old_path: String, new_path: String) -> Result<(), String> {
    std::fs::rename(&old_path, &new_path).map_err(|e| e.to_string())
}

#[tauri::command]
fn write_text_file(file_path: String, contents: String) -> Result<(), String> {
    std::fs::write(&file_path, &contents).map_err(|e| e.to_string())
}

#[tauri::command]
fn read_text_file(file_path: String) -> Result<String, String> {
    std::fs::read_to_string(&file_path).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_home_dir() -> Result<String, String> {
    dirs::home_dir()
        .map(|p| p.to_string_lossy().to_string())
        .ok_or_else(|| "Could not determine home directory".into())
}

#[tauri::command]
fn import_presets_json(file_path: String) -> Result<Vec<PresetFile>, String> {
    let data = std::fs::read_to_string(&file_path).map_err(|e| e.to_string())?;
    if let Ok(arr) = serde_json::from_str::<Vec<PresetFile>>(&data) {
        return Ok(arr);
    }
    let val: serde_json::Value = serde_json::from_str(&data).map_err(|e| e.to_string())?;
    if let Some(arr) = val.get("presets") {
        return serde_json::from_value(arr.clone()).map_err(|e| e.to_string());
    }
    Err("Expected a JSON array of presets or { \"presets\": [...] }".into())
}

#[tauri::command]
fn import_audio_json(file_path: String) -> Result<Vec<AudioSample>, String> {
    let data = std::fs::read_to_string(&file_path).map_err(|e| e.to_string())?;
    // Accept bare array or { "samples": [...] } envelope
    if let Ok(arr) = serde_json::from_str::<Vec<AudioSample>>(&data) {
        return Ok(arr);
    }
    let val: serde_json::Value = serde_json::from_str(&data).map_err(|e| e.to_string())?;
    if let Some(arr) = val.get("samples") {
        return serde_json::from_value(arr.clone()).map_err(|e| e.to_string());
    }
    Err("Expected a JSON array of samples or { \"samples\": [...] }".into())
}

#[tauri::command]
fn import_daw_json(file_path: String) -> Result<Vec<DawProject>, String> {
    let data = std::fs::read_to_string(&file_path).map_err(|e| e.to_string())?;
    // Accept bare array or { "projects": [...] } envelope
    if let Ok(arr) = serde_json::from_str::<Vec<DawProject>>(&data) {
        return Ok(arr);
    }
    let val: serde_json::Value = serde_json::from_str(&data).map_err(|e| e.to_string())?;
    if let Some(arr) = val.get("projects") {
        return serde_json::from_value(arr.clone()).map_err(|e| e.to_string());
    }
    Err("Expected a JSON array of projects or { \"projects\": [...] }".into())
}

// ── Tests ──

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

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

        export_plugins_json(plugins.clone(), tmp.to_string_lossy().to_string()).unwrap();
        let imported = import_plugins_json(tmp.to_string_lossy().to_string()).unwrap();

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
        export_plugins_json(plugins, tmp.to_string_lossy().to_string()).unwrap();

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
        export_plugins_csv(plugins, tmp.to_string_lossy().to_string()).unwrap();

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
        export_plugins_csv(vec![p], tmp.to_string_lossy().to_string()).unwrap();

        let content = fs::read_to_string(&tmp).unwrap();
        assert!(content.contains("\"Plugin, With Comma\""));
        assert!(content.contains("\"Company, Inc.\""));

        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn test_import_json_invalid_file() {
        let result = import_plugins_json("/nonexistent/path.json".into());
        assert!(result.is_err());
    }

    #[test]
    fn test_import_json_invalid_format() {
        let tmp = std::env::temp_dir().join("upum_test_import_bad.json");
        fs::write(&tmp, "not valid json").unwrap();

        let result = import_plugins_json(tmp.to_string_lossy().to_string());
        assert!(result.is_err());

        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn test_import_json_empty_plugins() {
        let tmp = std::env::temp_dir().join("upum_test_import_empty.json");
        let content = r#"{"version":"1.0","exported_at":"2025-01-01T00:00:00Z","plugins":[]}"#;
        fs::write(&tmp, content).unwrap();

        let result = import_plugins_json(tmp.to_string_lossy().to_string()).unwrap();
        assert!(result.is_empty());

        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn test_export_csv_empty_plugins() {
        let tmp = std::env::temp_dir().join("upum_test_export_empty.csv");
        let _ = fs::remove_file(&tmp);

        export_plugins_csv(vec![], tmp.to_string_lossy().to_string()).unwrap();
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

        let result = import_audio_json(tmp.to_string_lossy().to_string());
        assert!(result.is_ok());
        let imported = result.unwrap();
        assert_eq!(imported.len(), 2);
        assert_eq!(imported[0].name, "kick");
        assert_eq!(imported[1].format, "FLAC");

        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn test_import_audio_json_invalid_format() {
        let tmp = std::env::temp_dir().join("upum_test_import_audio_bad.json");
        fs::write(&tmp, r#"{"not": "an array"}"#).unwrap();

        let result = import_audio_json(tmp.to_string_lossy().to_string());
        assert!(result.is_err());

        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn test_import_audio_json_nonexistent() {
        let result = import_audio_json("/tmp/nonexistent_audio_file.json".into());
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

        let result = import_daw_json(tmp.to_string_lossy().to_string());
        assert!(result.is_ok());
        let imported = result.unwrap();
        assert_eq!(imported.len(), 2);
        assert_eq!(imported[0].daw, "Ableton Live");
        assert_eq!(imported[1].format, "FLP");

        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn test_import_daw_json_invalid_format() {
        let tmp = std::env::temp_dir().join("upum_test_import_daw_bad.json");
        fs::write(&tmp, "not json at all").unwrap();

        let result = import_daw_json(tmp.to_string_lossy().to_string());
        assert!(result.is_err());

        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn test_import_presets_json_valid() {
        let tmp = std::env::temp_dir().join("upum_test_import_presets.json");
        let presets = vec![make_preset("Lead", "FXP"), make_preset("Pad", "VSTPRESET")];
        let json = serde_json::to_string_pretty(&presets).unwrap();
        fs::write(&tmp, &json).unwrap();

        let result = import_presets_json(tmp.to_string_lossy().to_string());
        assert!(result.is_ok());
        let imported = result.unwrap();
        assert_eq!(imported.len(), 2);
        assert_eq!(imported[0].name, "Lead");
        assert_eq!(imported[1].format, "VSTPRESET");

        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn test_import_presets_json_invalid_format() {
        let tmp = std::env::temp_dir().join("upum_test_import_presets_bad.json");
        fs::write(&tmp, r#"[{"wrong": "fields"}]"#).unwrap();

        let result = import_presets_json(tmp.to_string_lossy().to_string());
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

        export_presets_json(presets.clone(), tmp.to_string_lossy().to_string()).unwrap();
        let imported = import_presets_json(tmp.to_string_lossy().to_string()).unwrap();

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

        export_audio_json(samples.clone(), tmp.to_string_lossy().to_string()).unwrap();
        let imported = import_audio_json(tmp.to_string_lossy().to_string()).unwrap();

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

        export_daw_json(projects.clone(), tmp.to_string_lossy().to_string()).unwrap();
        let imported = import_daw_json(tmp.to_string_lossy().to_string()).unwrap();

        assert_eq!(imported.len(), 2);
        assert_eq!(imported[0].daw, "Logic Pro");
        assert_eq!(imported[1].format, "RPP");

        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn test_import_presets_json_nonexistent() {
        let result = import_presets_json("/tmp/nonexistent_preset_file.json".into());
        assert!(result.is_err());
    }

    #[test]
    fn test_import_daw_json_nonexistent() {
        let result = import_daw_json("/tmp/nonexistent_daw_file.json".into());
        assert!(result.is_err());
    }

    #[test]
    fn test_import_audio_json_empty_array() {
        let tmp = std::env::temp_dir().join("upum_test_import_audio_empty.json");
        fs::write(&tmp, "[]").unwrap();

        let result = import_audio_json(tmp.to_string_lossy().to_string());
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);

        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn test_import_presets_json_empty_array() {
        let tmp = std::env::temp_dir().join("upum_test_import_presets_empty.json");
        fs::write(&tmp, "[]").unwrap();

        let result = import_presets_json(tmp.to_string_lossy().to_string());
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);

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

        let result = fs_list_dir(tmp.to_string_lossy().to_string()).unwrap();
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
        let result = fs_list_dir("/nonexistent/upum_dir_xyz".into());
        assert!(result.is_err());
    }

    #[test]
    fn test_fs_list_dir_not_a_dir() {
        let tmp = std::env::temp_dir().join("upum_test_fs_notdir.txt");
        fs::write(&tmp, "data").unwrap();
        let result = fs_list_dir(tmp.to_string_lossy().to_string());
        assert!(result.is_err());
        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn test_delete_file_regular() {
        let tmp = std::env::temp_dir().join("upum_test_delete.txt");
        fs::write(&tmp, "delete me").unwrap();
        assert!(tmp.exists());
        delete_file(tmp.to_string_lossy().to_string()).unwrap();
        assert!(!tmp.exists());
    }

    #[test]
    fn test_delete_file_directory() {
        let tmp = std::env::temp_dir().join("upum_test_delete_dir");
        fs::create_dir_all(tmp.join("inner")).unwrap();
        fs::write(tmp.join("inner").join("file.txt"), "data").unwrap();
        delete_file(tmp.to_string_lossy().to_string()).unwrap();
        assert!(!tmp.exists());
    }

    #[test]
    fn test_delete_file_nonexistent() {
        let result = delete_file("/nonexistent/upum_file_xyz.txt".into());
        assert!(result.is_err());
    }

    #[test]
    fn test_rename_file() {
        let tmp1 = std::env::temp_dir().join("upum_test_rename_old.txt");
        let tmp2 = std::env::temp_dir().join("upum_test_rename_new.txt");
        let _ = fs::remove_file(&tmp2);
        fs::write(&tmp1, "content").unwrap();
        rename_file(
            tmp1.to_string_lossy().to_string(),
            tmp2.to_string_lossy().to_string(),
        )
        .unwrap();
        assert!(!tmp1.exists());
        assert!(tmp2.exists());
        assert_eq!(fs::read_to_string(&tmp2).unwrap(), "content");
        let _ = fs::remove_file(&tmp2);
    }

    #[test]
    fn test_get_home_dir() {
        let result = get_home_dir();
        assert!(result.is_ok());
        let home = result.unwrap();
        assert!(!home.is_empty());
        assert!(std::path::Path::new(&home).exists());
    }

    // ── TOML export/import tests ──

    #[test]
    fn test_export_import_toml_roundtrip() {
        let tmp = std::env::temp_dir().join("upum_test_export.toml");
        let data = serde_json::json!({
            "plugins": [{"name": "Test", "version": "1.0"}]
        });
        export_toml(data.clone(), tmp.to_string_lossy().to_string()).unwrap();
        let imported = import_toml(tmp.to_string_lossy().to_string()).unwrap();
        assert!(imported["plugins"].is_array());
        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn test_import_toml_nonexistent() {
        let result = import_toml("/nonexistent/file.toml".into());
        assert!(result.is_err());
    }

    #[test]
    fn test_import_toml_invalid() {
        let tmp = std::env::temp_dir().join("upum_test_invalid.toml");
        fs::write(&tmp, "this is not valid toml [[[").unwrap();
        let result = import_toml(tmp.to_string_lossy().to_string());
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
        export_presets_dsv(presets, tmp.to_string_lossy().to_string()).unwrap();
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
        export_presets_dsv(presets, tmp.to_string_lossy().to_string()).unwrap();
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
}

// ── App setup ──

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize app start time for uptime tracking
    APP_START.get_or_init(Instant::now);

    // Load preferences once for all startup config
    let prefs = history::load_preferences();

    // Raise file descriptor limit for intensive directory walking
    #[cfg(unix)]
    {
        let fd_target: u64 = prefs
            .get("fdLimit")
            .and_then(|v| v.as_str().and_then(|s| s.parse().ok()).or(v.as_u64()))
            .unwrap_or(10240)
            .clamp(256, 65536);
        let mut rlim = libc::rlimit { rlim_cur: 0, rlim_max: 0 };
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
    rayon::ThreadPoolBuilder::new()
        .num_threads(pool_size)
        .stack_size(8 * 1024 * 1024)
        .panic_handler(|panic_info| {
            eprintln!("Rayon thread panicked: {:?}", panic_info);
        })
        .build_global()
        .ok();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
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
        .manage(WalkerStatus {
            plugin_dirs: Arc::new(std::sync::Mutex::new(Vec::new())),
            audio_dirs: Arc::new(std::sync::Mutex::new(Vec::new())),
            daw_dirs: Arc::new(std::sync::Mutex::new(Vec::new())),
            preset_dirs: Arc::new(std::sync::Mutex::new(Vec::new())),
        })
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
            compute_fingerprint,
            find_similar_samples,
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
            export_presets_json,
            export_presets_dsv,
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
            use tauri::menu::*;

            let handle = app.handle();

            // App menu (macOS convention — first menu shows app name)
            let app_about = PredefinedMenuItem::about(handle, Some("About AUDIO_HAXOR"), None)?;
            let app_sep1 = PredefinedMenuItem::separator(handle)?;
            let app_prefs = MenuItem::with_id(
                handle,
                "open_prefs_app",
                "Preferences...",
                true,
                Some("CmdOrCtrl+,"),
            )?;
            let app_sep2 = PredefinedMenuItem::separator(handle)?;
            let app_services = PredefinedMenuItem::services(handle, None)?;
            let app_sep3 = PredefinedMenuItem::separator(handle)?;
            let app_hide = PredefinedMenuItem::hide(handle, None)?;
            let app_hide_others = PredefinedMenuItem::hide_others(handle, None)?;
            let app_show_all = PredefinedMenuItem::show_all(handle, None)?;
            let app_sep4 = PredefinedMenuItem::separator(handle)?;
            let app_quit = PredefinedMenuItem::quit(handle, None)?;

            let app_menu = Submenu::with_id_and_items(
                handle,
                "app",
                "AUDIO_HAXOR",
                true,
                &[
                    &app_about,
                    &app_sep1,
                    &app_prefs,
                    &app_sep2,
                    &app_services,
                    &app_sep3,
                    &app_hide,
                    &app_hide_others,
                    &app_show_all,
                    &app_sep4,
                    &app_quit,
                ],
            )?;

            // File menu
            let scan_all = MenuItem::with_id(
                handle,
                "scan_all",
                "Scan All",
                true,
                Some("CmdOrCtrl+Shift+S"),
            )?;
            let stop_all =
                MenuItem::with_id(handle, "stop_all", "Stop All", true, Some("CmdOrCtrl+."))?;
            let sep1 = PredefinedMenuItem::separator(handle)?;
            let export_plugins = MenuItem::with_id(
                handle,
                "export_plugins",
                "Export Plugins...",
                true,
                Some("CmdOrCtrl+E"),
            )?;
            let import_plugins = MenuItem::with_id(
                handle,
                "import_plugins",
                "Import Plugins...",
                true,
                Some("CmdOrCtrl+I"),
            )?;
            let sep2 = PredefinedMenuItem::separator(handle)?;
            let export_audio = MenuItem::with_id(
                handle,
                "export_audio",
                "Export Samples...",
                true,
                None::<&str>,
            )?;
            let import_audio = MenuItem::with_id(
                handle,
                "import_audio",
                "Import Samples...",
                true,
                None::<&str>,
            )?;
            let sep3 = PredefinedMenuItem::separator(handle)?;
            let export_daw = MenuItem::with_id(
                handle,
                "export_daw",
                "Export DAW Projects...",
                true,
                None::<&str>,
            )?;
            let import_daw = MenuItem::with_id(
                handle,
                "import_daw",
                "Import DAW Projects...",
                true,
                None::<&str>,
            )?;
            let sep4 = PredefinedMenuItem::separator(handle)?;
            let export_presets = MenuItem::with_id(
                handle,
                "export_presets",
                "Export Presets...",
                true,
                None::<&str>,
            )?;
            let import_presets = MenuItem::with_id(
                handle,
                "import_presets",
                "Import Presets...",
                true,
                None::<&str>,
            )?;
            let file_menu = Submenu::with_id_and_items(
                handle,
                "file",
                "File",
                true,
                &[
                    &scan_all,
                    &stop_all,
                    &sep1,
                    &export_plugins,
                    &import_plugins,
                    &sep2,
                    &export_audio,
                    &import_audio,
                    &sep3,
                    &export_daw,
                    &import_daw,
                    &sep4,
                    &export_presets,
                    &import_presets,
                ],
            )?;

            // Edit menu
            let edit_undo = PredefinedMenuItem::undo(handle, None)?;
            let edit_redo = PredefinedMenuItem::redo(handle, None)?;
            let edit_sep1 = PredefinedMenuItem::separator(handle)?;
            let edit_cut = PredefinedMenuItem::cut(handle, None)?;
            let edit_copy = PredefinedMenuItem::copy(handle, None)?;
            let edit_paste = PredefinedMenuItem::paste(handle, None)?;
            let edit_select_all = PredefinedMenuItem::select_all(handle, None)?;
            let edit_sep2 = PredefinedMenuItem::separator(handle)?;
            let find = MenuItem::with_id(handle, "find", "Find...", true, Some("CmdOrCtrl+F"))?;

            let edit_menu = Submenu::with_id_and_items(
                handle,
                "edit",
                "Edit",
                true,
                &[
                    &edit_undo,
                    &edit_redo,
                    &edit_sep1,
                    &edit_cut,
                    &edit_copy,
                    &edit_paste,
                    &edit_select_all,
                    &edit_sep2,
                    &find,
                ],
            )?;

            // Scan menu
            let scan_plugins = MenuItem::with_id(
                handle,
                "scan_plugins",
                "Scan Plugins",
                true,
                Some("CmdOrCtrl+Shift+P"),
            )?;
            let scan_audio = MenuItem::with_id(
                handle,
                "scan_audio",
                "Scan Samples",
                true,
                Some("CmdOrCtrl+Shift+A"),
            )?;
            let scan_daw = MenuItem::with_id(
                handle,
                "scan_daw",
                "Scan DAW Projects",
                true,
                Some("CmdOrCtrl+Shift+D"),
            )?;
            let scan_presets = MenuItem::with_id(
                handle,
                "scan_presets",
                "Scan Presets",
                true,
                Some("CmdOrCtrl+Shift+R"),
            )?;
            let scan_sep = PredefinedMenuItem::separator(handle)?;
            let check_updates = MenuItem::with_id(
                handle,
                "check_updates",
                "Check Updates",
                true,
                Some("CmdOrCtrl+U"),
            )?;

            let scan_menu = Submenu::with_id_and_items(
                handle,
                "scan",
                "Scan",
                true,
                &[
                    &scan_plugins,
                    &scan_audio,
                    &scan_daw,
                    &scan_presets,
                    &scan_sep,
                    &check_updates,
                ],
            )?;

            // View menu
            let tab_plugins =
                MenuItem::with_id(handle, "tab_plugins", "Plugins", true, Some("CmdOrCtrl+1"))?;
            let tab_samples =
                MenuItem::with_id(handle, "tab_samples", "Samples", true, Some("CmdOrCtrl+2"))?;
            let tab_daw =
                MenuItem::with_id(handle, "tab_daw", "DAW Projects", true, Some("CmdOrCtrl+3"))?;
            let tab_presets =
                MenuItem::with_id(handle, "tab_presets", "Presets", true, Some("CmdOrCtrl+4"))?;
            let tab_favorites = MenuItem::with_id(
                handle,
                "tab_favorites",
                "Favorites",
                true,
                Some("CmdOrCtrl+5"),
            )?;
            let tab_notes =
                MenuItem::with_id(handle, "tab_notes", "Notes", true, Some("CmdOrCtrl+6"))?;
            let tab_history =
                MenuItem::with_id(handle, "tab_history", "History", true, Some("CmdOrCtrl+7"))?;
            let tab_settings = MenuItem::with_id(
                handle,
                "tab_settings",
                "Settings",
                true,
                Some("CmdOrCtrl+8"),
            )?;
            let tab_files =
                MenuItem::with_id(handle, "tab_files", "Files", true, Some("CmdOrCtrl+9"))?;
            let view_sep = PredefinedMenuItem::separator(handle)?;
            let toggle_theme = MenuItem::with_id(
                handle,
                "toggle_theme",
                "Toggle Light/Dark",
                true,
                Some("CmdOrCtrl+T"),
            )?;
            let toggle_crt = MenuItem::with_id(
                handle,
                "toggle_crt",
                "Toggle CRT Effects",
                true,
                None::<&str>,
            )?;
            let view_sep2 = PredefinedMenuItem::separator(handle)?;
            let reset_columns = MenuItem::with_id(
                handle,
                "reset_columns",
                "Reset Column Widths",
                true,
                None::<&str>,
            )?;
            let reset_tabs =
                MenuItem::with_id(handle, "reset_tabs", "Reset Tab Order", true, None::<&str>)?;

            let view_menu = Submenu::with_id_and_items(
                handle,
                "view",
                "View",
                true,
                &[
                    &tab_plugins,
                    &tab_samples,
                    &tab_daw,
                    &tab_presets,
                    &tab_favorites,
                    &tab_notes,
                    &tab_history,
                    &tab_settings,
                    &tab_files,
                    &view_sep,
                    &toggle_theme,
                    &toggle_crt,
                    &view_sep2,
                    &reset_columns,
                    &reset_tabs,
                ],
            )?;

            // Playback menu
            let play_pause =
                MenuItem::with_id(handle, "play_pause", "Play / Pause", true, Some("Space"))?;
            let toggle_loop = MenuItem::with_id(
                handle,
                "toggle_loop",
                "Toggle Loop",
                true,
                Some("CmdOrCtrl+L"),
            )?;
            let stop_playback = MenuItem::with_id(
                handle,
                "stop_playback",
                "Stop Playback",
                true,
                Some("CmdOrCtrl+Shift+."),
            )?;
            let expand_player = MenuItem::with_id(
                handle,
                "expand_player",
                "Expand / Collapse Player",
                true,
                Some("CmdOrCtrl+Shift+M"),
            )?;

            let next_track = MenuItem::with_id(
                handle,
                "next_track",
                "Next Track",
                true,
                Some("CmdOrCtrl+Right"),
            )?;
            let prev_track = MenuItem::with_id(
                handle,
                "prev_track",
                "Previous Track",
                true,
                Some("CmdOrCtrl+Left"),
            )?;
            let toggle_shuffle = MenuItem::with_id(
                handle,
                "toggle_shuffle",
                "Toggle Shuffle",
                true,
                None::<&str>,
            )?;
            let toggle_mute =
                MenuItem::with_id(handle, "toggle_mute", "Mute / Unmute", true, None::<&str>)?;
            let playback_sep = PredefinedMenuItem::separator(handle)?;

            let playback_menu = Submenu::with_id_and_items(
                handle,
                "playback",
                "Playback",
                true,
                &[
                    &play_pause,
                    &stop_playback,
                    &playback_sep,
                    &next_track,
                    &prev_track,
                    &toggle_loop,
                    &toggle_shuffle,
                    &toggle_mute,
                    &playback_sep,
                    &expand_player,
                ],
            )?;

            // Data menu
            let clear_history = MenuItem::with_id(
                handle,
                "clear_history",
                "Clear All History...",
                true,
                Some("CmdOrCtrl+Shift+Delete"),
            )?;
            let clear_kvr = MenuItem::with_id(
                handle,
                "clear_kvr",
                "Clear KVR Cache...",
                true,
                None::<&str>,
            )?;
            let clear_favorites = MenuItem::with_id(
                handle,
                "clear_favorites",
                "Clear Favorites...",
                true,
                None::<&str>,
            )?;

            let reset_all = MenuItem::with_id(
                handle,
                "reset_all",
                "Reset All Scans...",
                true,
                Some("CmdOrCtrl+Shift+Backspace"),
            )?;
            let data_sep = PredefinedMenuItem::separator(handle)?;
            let find_duplicates = MenuItem::with_id(
                handle,
                "find_duplicates",
                "Find Duplicates",
                true,
                Some("CmdOrCtrl+D"),
            )?;
            let dep_graph = MenuItem::with_id(
                handle,
                "dep_graph",
                "Dependency Graph",
                true,
                Some("CmdOrCtrl+G"),
            )?;
            let cmd_palette = MenuItem::with_id(
                handle,
                "cmd_palette",
                "Command Palette",
                true,
                Some("CmdOrCtrl+K"),
            )?;
            let help_overlay = MenuItem::with_id(
                handle,
                "help_overlay",
                "Keyboard Shortcuts",
                true,
                None::<&str>,
            )?;

            let data_menu = Submenu::with_id_and_items(
                handle,
                "data",
                "Data",
                true,
                &[
                    &clear_history,
                    &clear_kvr,
                    &clear_favorites,
                    &data_sep,
                    &reset_all,
                ],
            )?;

            let tools_menu = Submenu::with_id_and_items(
                handle,
                "tools",
                "Tools",
                true,
                &[
                    &find_duplicates,
                    &dep_graph,
                    &data_sep,
                    &cmd_palette,
                    &help_overlay,
                ],
            )?;

            // Window menu
            let minimize = PredefinedMenuItem::minimize(handle, None)?;
            let zoom = PredefinedMenuItem::maximize(handle, None)?;
            let win_sep = PredefinedMenuItem::separator(handle)?;
            let close_win = PredefinedMenuItem::close_window(handle, None)?;

            let window_menu = Submenu::with_id_and_items(
                handle,
                "window",
                "Window",
                true,
                &[&minimize, &zoom, &win_sep, &close_win],
            )?;

            // Help menu
            let github =
                MenuItem::with_id(handle, "github", "GitHub Repository", true, None::<&str>)?;
            let docs = MenuItem::with_id(handle, "docs", "Documentation", true, None::<&str>)?;

            let help_menu =
                Submenu::with_id_and_items(handle, "help", "Help", true, &[&github, &docs])?;

            let menu = Menu::with_items(
                handle,
                &[
                    &app_menu,
                    &file_menu,
                    &edit_menu,
                    &scan_menu,
                    &view_menu,
                    &playback_menu,
                    &data_menu,
                    &tools_menu,
                    &window_menu,
                    &help_menu,
                ],
            )?;
            app.set_menu(menu)?;

            // Handle menu events — emit to frontend JS
            let handle2 = app.handle().clone();
            app.on_menu_event(move |_app, event| {
                let id = event.id().0.as_str();
                if let Some(win) = handle2.get_webview_window("main") {
                    let _ = win.emit("menu-action", id);
                }
            });

            // System tray
            use tauri::tray::*;
            let tray_menu = MenuBuilder::new(app)
                .text("tray_show", "Show AUDIO_HAXOR")
                .separator()
                .text("tray_scan_all", "Scan All")
                .text("tray_stop_all", "Stop All")
                .separator()
                .text("tray_play_pause", "Play / Pause")
                .text("tray_next", "Next Track")
                .separator()
                .text("tray_quit", "Quit")
                .build()?;

            let _tray = TrayIconBuilder::new()
                .menu(&tray_menu)
                .tooltip("AUDIO_HAXOR")
                .on_menu_event(move |app_handle, event| {
                    let id = event.id().as_ref();
                    if id == "tray_quit" {
                        app_handle.exit(0);
                    } else if id == "tray_show" {
                        if let Some(win) = app_handle.get_webview_window("main") {
                            let _ = win.show();
                            let _ = win.set_focus();
                        }
                    } else if let Some(win) = app_handle.get_webview_window("main") {
                        let action = match id {
                            "tray_scan_all" => "scan_all",
                            "tray_stop_all" => "stop_all",
                            "tray_play_pause" => "play_pause",
                            "tray_next" => "next_track",
                            _ => return,
                        };
                        let _ = win.emit("menu-action", action);
                    }
                })
                .build(app)?;

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
