pub mod audio_scanner;
pub mod history;
pub mod kvr;
pub mod scanner;

use history::{AudioSample, KvrCacheUpdateEntry};
use scanner::PluginInfo;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Emitter, Manager};

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
async fn scan_plugins(app: AppHandle) -> Result<serde_json::Value, String> {
    let state = app.state::<ScanState>();

    if state.scanning.swap(true, Ordering::SeqCst) {
        return Err("Scan already in progress".into());
    }
    state.stop_scan.store(false, Ordering::SeqCst);

    let app_handle = app.clone();
    let result = tokio::task::spawn_blocking(move || {
        let scan_state = app_handle.state::<ScanState>();
        let directories = scanner::get_vst_directories();
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

        let mut all_plugins = Vec::new();
        let mut seen = HashSet::new();
        let mut processed = 0usize;
        let batch_size = 10;
        let mut batch = Vec::new();

        for plugin_path in &plugin_paths {
            if scan_state.stop_scan.load(Ordering::SeqCst) {
                break;
            }

            let path_str = plugin_path.to_string_lossy().to_string();
            if seen.contains(&path_str) {
                processed += 1;
                continue;
            }
            seen.insert(path_str);

            if let Some(info) = scanner::get_plugin_info(plugin_path) {
                batch.push(info);
            }
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

        all_plugins.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        let snapshot = history::save_scan(&all_plugins, &directories);

        serde_json::json!({
            "plugins": all_plugins,
            "directories": directories,
            "snapshotId": snapshot.id
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

        let update_result = kvr::find_latest_version(
            &representative.name,
            &representative.manufacturer,
            &representative.version,
        )
        .await;

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

        // Rate limit
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
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
async fn scan_audio_samples(app: AppHandle) -> Result<serde_json::Value, String> {
    let state = app.state::<AudioScanState>();
    if state.scanning.swap(true, Ordering::SeqCst) {
        return Err("Audio scan already in progress".into());
    }
    state.stop_scan.store(false, Ordering::SeqCst);

    let _ = app.emit(
        "audio-scan-progress",
        serde_json::json!({
            "phase": "status",
            "message": "Walking filesystem for audio files..."
        }),
    );

    let app_handle = app.clone();
    let result = tokio::task::spawn_blocking(move || {
        let audio_state = app_handle.state::<AudioScanState>();
        let roots = audio_scanner::get_audio_roots();
        let mut all_samples: Vec<AudioSample> = Vec::new();

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
        );

        all_samples.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        serde_json::json!({ "samples": all_samples })
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
fn audio_history_save(samples: Vec<AudioSample>) -> history::AudioScanSnapshot {
    history::save_audio_scan(&samples)
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
fn audio_history_diff(
    old_id: String,
    new_id: String,
) -> Option<history::AudioScanDiff> {
    history::diff_audio_scans(&old_id, &new_id)
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

// ── App setup ──

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
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
            open_update_url,
            open_plugin_folder,
            open_audio_folder,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
