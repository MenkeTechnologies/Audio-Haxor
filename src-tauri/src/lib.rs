pub mod audio_scanner;
pub mod daw_scanner;
pub mod history;
pub mod kvr;
pub mod preset_scanner;
pub mod scanner;

use history::{AudioSample, DawProject, KvrCacheUpdateEntry, PresetFile};
use scanner::PluginInfo;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
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
    modified: String,
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
        let batch_size = 10;
        let (tx, rx) = std::sync::mpsc::sync_channel::<scanner::PluginInfo>(64);
        let stop_flag = std::sync::Arc::new(AtomicBool::new(false));
        let stop_flag2 = stop_flag.clone();

        std::thread::spawn(move || {
            unique_paths.par_iter().for_each(|p| {
                if stop_flag2.load(Ordering::Relaxed) {
                    return;
                }
                if let Some(info) = scanner::get_plugin_info(p) {
                    let _ = tx.send(info);
                }
            });
        });

        let mut all_plugins = Vec::new();
        let mut batch = Vec::new();
        let mut processed = 0usize;
        for info in rx {
            if scan_state.stop_scan.load(Ordering::Relaxed) {
                stop_flag.store(true, Ordering::Relaxed);
                break;
            }
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
    .await
    .map_err(|e| e.to_string())?;

    state.scanning.store(false, Ordering::SeqCst);
    Ok(result)
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
        );

        let root_strs: Vec<String> = roots
            .iter()
            .map(|r| r.to_string_lossy().to_string())
            .collect();
        let was_stopped = audio_state.stop_scan.load(Ordering::Relaxed);
        all_samples.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        serde_json::json!({ "samples": all_samples, "roots": root_strs, "stopped": was_stopped })
    })
    .await
    .map_err(|e| e.to_string())?;

    state.scanning.store(false, Ordering::SeqCst);
    Ok(result)
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
        );

        let root_strs: Vec<String> = roots
            .iter()
            .map(|r| r.to_string_lossy().to_string())
            .collect();
        let was_stopped = daw_state.stop_scan.load(Ordering::Relaxed);
        all_projects.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        serde_json::json!({ "projects": all_projects, "roots": root_strs, "stopped": was_stopped })
    })
    .await
    .map_err(|e| e.to_string())?;

    state.scanning.store(false, Ordering::SeqCst);
    Ok(result)
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
        );

        let was_stopped = preset_state.stop_scan.load(Ordering::Relaxed);
        let root_strs: Vec<String> = roots
            .iter()
            .map(|r| r.to_string_lossy().to_string())
            .collect();
        all_presets.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        serde_json::json!({ "presets": all_presets, "roots": root_strs, "stopped": was_stopped })
    })
    .await
    .map_err(|e| e.to_string())?;

    state.scanning.store(false, Ordering::SeqCst);
    Ok(result)
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
            modified: p.modified.clone(),
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
            modified: p.modified,
        })
        .collect())
}

// ── Process stats ──

#[tauri::command]
fn get_process_stats() -> serde_json::Value {
    let mem_bytes = get_rss_bytes();
    let threads = get_thread_count();
    serde_json::json!({
        "memBytes": mem_bytes,
        "threads": threads,
    })
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
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|s| {
            s.lines()
                .find(|l| l.starts_with("VmRSS:"))
                .and_then(|l| l.split_whitespace().nth(1))
                .and_then(|v| v.parse::<u64>().ok())
                .map(|kb| kb * 1024)
        })
        .unwrap_or(0)
}

#[cfg(target_os = "windows")]
fn get_rss_bytes() -> u64 {
    0
}

fn get_thread_count() -> u32 {
    #[cfg(target_os = "macos")]
    {
        let output = std::process::Command::new("ps")
            .args(["-M", "-p", &std::process::id().to_string()])
            .output();
        if let Ok(out) = output {
            let s = String::from_utf8_lossy(&out.stdout);
            return s.lines().count().saturating_sub(1) as u32;
        }
        0
    }
    #[cfg(target_os = "linux")]
    {
        std::fs::read_to_string("/proc/self/status")
            .ok()
            .and_then(|s| {
                s.lines()
                    .find(|l| l.starts_with("Threads:"))
                    .and_then(|l| l.split_whitespace().nth(1))
                    .and_then(|v| v.parse().ok())
            })
            .unwrap_or(0)
    }
    #[cfg(target_os = "windows")]
    {
        0
    }
}

// ── Preset export/import ──

#[tauri::command]
fn export_presets_json(presets: Vec<PresetFile>, file_path: String) -> Result<(), String> {
    let json = serde_json::to_string_pretty(&presets).map_err(|e| e.to_string())?;
    std::fs::write(&file_path, json).map_err(|e| e.to_string())
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
            modified: "2025-01-01".into(),
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
                modified: "2025-01-01".into(),
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
            modified: "2025-01-01".into(),
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
            modified: "2025-01-01".into(),
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
}

// ── App setup ──

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize rayon thread pool — sized for concurrent scans across all cores
    rayon::ThreadPoolBuilder::new()
        .num_threads(num_cpus::get())
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
        .invoke_handler(tauri::generate_handler![
            get_version,
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
            import_presets_json,
            import_audio_json,
            import_daw_json,
            get_process_stats,
            open_prefs_file,
            get_prefs_path,
        ])
        .setup(|app| {
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
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
