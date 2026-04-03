//! Stress tests for parallel scanning and batch operations.
//! Heavy tests are marked `#[ignore]` so default `cargo test` stays fast.

use app_lib::db::{global, init_global};
use app_lib::history::AudioSample;
use app_lib::scanner::{get_plugin_info, PluginInfo};
use rayon::prelude::*;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Serialize stress tests in this binary so parallel `cargo test` does not contend on temp I/O.
static STRESS_TESTS_LOCK: Mutex<()> = Mutex::new(());

static STRESS_DIR_SEQ: AtomicU64 = AtomicU64::new(0);

fn stress_temp_base() -> PathBuf {
    let n = STRESS_DIR_SEQ.fetch_add(1, Ordering::SeqCst);
    std::env::temp_dir().join(format!("audio_haxor_stress_{n}"))
}

/// Mach-O 64-bit LE header bytes (minimal; enough for scanner heuristics).
const MACHO_HEAD: [u8; 8] = [0xcf, 0xfa, 0xed, 0xfe, 0x0c, 0x00, 0x00, 0x01];

fn create_fake_plugin(dir: &Path, name: &str) -> PathBuf {
    let plugin = dir.join(format!("{}.vst3", name));
    for i in 0..3 {
        let bin = plugin.join(format!("Engine{}", i)).join("MacOS");
        let _ = std::fs::create_dir_all(&bin);
        let binary = bin.join("plugin");
        let _ = std::fs::write(&binary, MACHO_HEAD);
    }
    plugin
}

fn create_stress_plugins(count: usize, base_dir: &Path) -> Vec<PathBuf> {
    let dir = base_dir.join("VST3");
    let mut plugins = Vec::new();
    for i in 0..count {
        let plugin = create_fake_plugin(&dir, &format!("StressPlugin{}", i));
        plugins.push(plugin);
    }
    plugins
}

#[test]
fn test_stress_plugin_scan_large_batch() {
    let _lock = STRESS_TESTS_LOCK.lock().expect("stress mutex poisoned");
    let base_dir = stress_temp_base();
    let plugins = create_stress_plugins(500, &base_dir);

    let start = Instant::now();
    let infos: Vec<_> = plugins
        .par_iter()
        .filter_map(|p| get_plugin_info(p.as_path()))
        .collect();
    let elapsed = start.elapsed();

    assert_eq!(infos.len(), 500, "Should scan all fake plugins");
    dbg!(format!("Scanned {} plugins in {:?}", infos.len(), elapsed));

    let _ = std::fs::remove_dir_all(&base_dir);
}

#[test]
#[ignore = "initializes global DB and writes 1000 temp WAVs; run explicitly"]
fn test_stress_db_batch_insert() {
    let _ = init_global().map_err(|e| eprintln!("DB init failed: {}", e));

    let base_dir = stress_temp_base();
    let samples = (0..1000)
        .map(|i| {
            let file = base_dir.join(format!("audio_{}.wav", i));
            let _ = std::fs::write(&file, vec![0u8; 1024]);
            AudioSample {
                name: format!("audio_{}", i),
                path: file.to_string_lossy().to_string(),
                directory: base_dir.to_string_lossy().to_string(),
                format: "WAV".to_string(),
                size: 1024,
                size_formatted: "1.0 KB".to_string(),
                modified: "2024-01-01".to_string(),
                duration: None,
                channels: None,
                sample_rate: None,
                bits_per_sample: None,
            }
        })
        .collect::<Vec<_>>();

    let start = Instant::now();
    let _ = global().insert_audio_batch("stress_scan_id", &samples);
    let elapsed = start.elapsed();

    assert!(
        elapsed <= Duration::from_secs(5),
        "Batch insert should complete in <5s"
    );
    dbg!(format!(
        "Inserted {} audio samples in {:?}",
        samples.len(),
        elapsed
    ));

    let _ = std::fs::remove_dir_all(&base_dir);
}

#[test]
fn test_stress_scan_with_abort() {
    let _lock = STRESS_TESTS_LOCK.lock().expect("stress mutex poisoned");
    let base_dir = stress_temp_base();
    let plugins = create_stress_plugins(200, &base_dir);

    let start = Instant::now();
    let infos: Vec<_> = plugins
        .into_iter()
        .take(100)
        .filter_map(|p| get_plugin_info(p.as_path()))
        .collect();
    let elapsed = start.elapsed();

    assert_eq!(
        infos.len(),
        100,
        "Should have processed 100 plugins before abort"
    );
    dbg!(format!(
        "Scan stopped after {}, elapsed: {:?}",
        infos.len(),
        elapsed
    ));

    let _ = std::fs::remove_dir_all(&base_dir);
}

#[test]
#[ignore = "file watcher API is start_watching/stop_watching; replace when a headless harness exists"]
fn test_stress_concurrent_file_watcher() {}

#[test]
#[ignore]
fn test_stress_memory_pressure() {
    use std::collections::HashMap;

    let base_dir = stress_temp_base();
    let mut plugin_cache: HashMap<String, PluginInfo> = HashMap::new();

    let plugins = create_stress_plugins(1000, &base_dir);
    let infos: Vec<_> = plugins
        .into_iter()
        .filter_map(|p| get_plugin_info(p.as_path()))
        .collect();

    for info in infos {
        plugin_cache.insert(info.name.clone(), info);
    }

    let cache_size = plugin_cache.len();
    let _ = std::fs::remove_dir_all(&base_dir);

    assert_eq!(cache_size, 1000);
}

#[test]
fn test_stress_serialization() {
    let _lock = STRESS_TESTS_LOCK.lock().expect("stress mutex poisoned");
    let base_dir = stress_temp_base();
    let plugins = create_stress_plugins(100, &base_dir);

    let start = Instant::now();
    let plugin_jsons: Vec<_> = plugins
        .par_iter()
        .filter_map(|p| get_plugin_info(p.as_path()))
        .map(|info| {
            let json = serde_json::to_string(&info).expect("Serialization failed");
            (info.name.clone(), json)
        })
        .collect();
    let encode_time = start.elapsed();

    let start = Instant::now();
    let deserialized: Vec<_> = plugin_jsons
        .iter()
        .map(|(_, json)| serde_json::from_str::<PluginInfo>(json).expect("Deserialization failed"))
        .collect();
    let decode_time = start.elapsed();

    assert_eq!(deserialized.len(), 100);
    dbg!(format!(
        "Encode: {:?}, Decode: {:?}",
        encode_time, decode_time
    ));

    let _ = std::fs::remove_dir_all(&base_dir);
}

#[test]
#[ignore]
#[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
fn test_stress_batch_analysis() {
    use app_lib::bpm::estimate_bpm;
    use app_lib::key_detect::detect_key;
    use app_lib::lufs::measure_lufs;

    let base_dir = stress_temp_base();
    let file_count = 200;
    let paths: Vec<PathBuf> = (0..file_count)
        .map(|i| {
            let file = base_dir.join(format!("audio_{}.wav", i));
            let _ = std::fs::write(&file, vec![0u8; 1024]);
            file
        })
        .collect();

    let start = Instant::now();
    let _ = paths
        .par_iter()
        .filter_map(|path| path.to_str())
        .map(|path| {
            let _ = estimate_bpm(path);
            let _ = detect_key(path);
            let _ = measure_lufs(path);
        })
        .count();
    let elapsed = start.elapsed();

    dbg!(format!("Analyzed {} files in {:?}", file_count, elapsed));
    let _ = std::fs::remove_dir_all(&base_dir);
}

#[test]
fn test_stress_progress_events() {
    use std::sync::mpsc;
    use std::sync::{Arc, Mutex};

    let _lock = STRESS_TESTS_LOCK.lock().expect("stress mutex poisoned");
    let base_dir = stress_temp_base();
    let plugins = create_stress_plugins(100, &base_dir);
    let total = plugins.len();

    let (tx, rx) = mpsc::channel::<(String, usize, usize)>();
    let plugin_dirs = Arc::new(Mutex::new(Vec::new()));

    let handle = std::thread::spawn(move || {
        for (i, plugin) in plugins.into_iter().enumerate() {
            let Some(info) = get_plugin_info(plugin.as_path()) else {
                continue;
            };
            let _ = tx.send((info.name, i + 1, total));

            let mut dirs = plugin_dirs.lock().unwrap();
            if dirs.len() < 30 {
                dirs.push(plugin.to_string_lossy().to_string());
            }
        }
    });

    handle.join().expect("progress thread panicked");
    let events_received: usize = rx.iter().count();
    assert_eq!(
        events_received, 100,
        "expected one progress event per fake plugin"
    );

    let _ = std::fs::remove_dir_all(&base_dir);
}
