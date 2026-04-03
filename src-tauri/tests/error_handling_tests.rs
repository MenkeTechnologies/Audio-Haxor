//! Error handling and edge case tests for audio_haxor
//! Tests failure scenarios, invalid inputs, and error recovery

use app_lib::format_size;
use app_lib::kvr::{compare_versions, URL_RE};
use app_lib::midi;
use app_lib::scanner::{self, PluginInfo};
use app_lib::similarity::{self, AudioFingerprint};
use std::path::Path;

/// Test scanner handles non-existent paths gracefully
#[test]
fn test_scanner_nonexistent_path() {
    let path = Path::new("/does/not/exist/Plugin.vst3");
    let info = scanner::get_plugin_info(path);
    assert!(info.is_none(), "Should return None for nonexistent path");
}

/// Test scanner rejects invalid extensions
#[test]
fn test_scanner_invalid_extension() {
    let path = Path::new("/fake/path/invalid.txt");
    let info = scanner::get_plugin_info(path);
    assert!(info.is_none(), "Should reject non-plugin extensions");
}

/// Test scanner handles permission errors
#[test]
#[ignore] // Can't easily test permission errors on macOS
fn test_scanner_permission_denied() {
    // This would test behavior when a file is read-only or inaccessible

    // Create a read-only file
    let temp = std::env::temp_dir().join("readonly.txt");
    let _ = std::fs::write(&temp, b"read-only");

    // Try to make it unreadable (doesn't work on macOS)
    // On Linux, we'd use: chmod 000 readonly.txt

    let path = Path::new("/nonexistent/Plugin.vst3"); // Fallback
    let info = scanner::get_plugin_info(path);
    assert!(info.is_none());

    let _ = std::fs::remove_file(&temp);
}

/// Test MIDI parser handles corrupted files
#[test]
fn test_midi_corrupted_header() {
    // Empty file
    let corrupted = std::env::temp_dir().join("corrupted.mid");
    let _ = std::fs::write(&corrupted, vec![]);

    let info = midi::parse_midi(corrupted.as_path());
    assert!(info.is_none(), "Should return None for empty MIDI");
}

/// Test MIDI parser handles partial headers
#[test]
fn test_midi_partial_header() {
    // Only first 4 bytes (MThd prefix without length)
    let partial = std::env::temp_dir().join("partial.mid");
    let _ = std::fs::write(&partial, b"MThd");

    let info = midi::parse_midi(partial.as_path());
    assert!(info.is_none(), "Should return None for partial header");
}

/// Test MIDI parser handles non-MIDI files
#[test]
fn test_midi_wrong_format() {
    // Just a text file
    let text = std::env::temp_dir().join("not_a_midi.txt");
    let _ = std::fs::write(&text, b"This is not MIDI data.");

    let info = midi::parse_midi(text.as_path());
    assert!(info.is_none(), "Should return None for non-MIDI files");
}

/// Test similarity computation on non-audio files
#[test]
fn test_similarity_non_audio_file() {
    let non_audio = std::env::temp_dir().join("document.txt");
    let _ = std::fs::write(&non_audio, b"PDF content");

    let result = non_audio
        .to_str()
        .and_then(|p| similarity::compute_fingerprint(p));
    assert!(result.is_none(), "Should return None for non-audio files");
}

/// `find_similar` with an empty candidate list returns no rows.
#[test]
fn test_similarity_no_candidates() {
    let reference = AudioFingerprint {
        path: "/tmp/ref.wav".to_string(),
        rms: 0.1,
        spectral_centroid: 0.2,
        zero_crossing_rate: 0.05,
        low_band_energy: 0.3,
        mid_band_energy: 0.4,
        high_band_energy: 0.3,
        low_energy_ratio: 0.5,
        attack_time: 0.01,
    };
    let candidates: Vec<AudioFingerprint> = vec![];
    let results = similarity::find_similar(&reference, &candidates, 10);
    assert!(results.is_empty());
}

/// Test parallel scan with cancelled task
#[test]
#[ignore]
fn test_parallel_scan_cancellation() {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    let cancel_flag = Arc::new(AtomicBool::new(false));
    let cancel_flag_clone = cancel_flag.clone();

    let handle = std::thread::spawn(move || {
        // Simulate scanning
        let mut count = 0;
        while count < 1000 {
            if cancel_flag_clone.load(Ordering::SeqCst) {
                break;
            }
            count += 1;
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        count
    });

    std::thread::sleep(std::time::Duration::from_millis(10)); // Let it start
    cancel_flag.store(true, Ordering::SeqCst);

    match handle.join() {
        Ok(count) => assert!(count < 1000, "Scan should have been cancelled"),
        Err(_) => panic!("Panic during scan"),
    }
}

/// Placeholder for `query_audio` + fixture-driven batch tests (DB API no longer has `get_audio`).
#[test]
#[ignore = "implement with AudioQueryParams + a scan fixture when DB integration tests are expanded"]
fn test_batch_with_mixed_paths() {}

/// Test export with very large plugin list
#[test]
#[ignore]
fn test_export_large_plugin_list() {
    use app_lib::history::ScanSnapshot;

    let large_plugins = (0..1000)
        .map(|i| {
            let name = format!("LargePlugin{}", i);
            let path = format!("/plugins/{}.vst3", name);
            PluginInfo {
                name,
                path,
                plugin_type: "VST3".to_string(),
                version: "1.0.0".to_string(),
                manufacturer: "Manufacturer".to_string(),
                manufacturer_url: None,
                size: "1 MB".to_string(),
                size_bytes: 1_000_000,
                modified: "2024-01-01".to_string(),
                architectures: vec!["x86_64".to_string()],
            }
        })
        .collect::<Vec<_>>();

    let snapshot = ScanSnapshot {
        id: "large_export_test".to_string(),
        timestamp: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        plugin_count: large_plugins.len(),
        directories: vec![],
        roots: vec![],
        plugins: large_plugins.clone(),
    };

    // Serialize to JSON
    let json = serde_json::to_string_pretty(&snapshot);
    assert!(json.is_ok(), "Should handle large plugin lists");
}

/// Test export with special characters in filenames
#[test]
fn test_export_special_chars() {
    let special_names = vec![
        "Plugin & Co",
        "VST <version>",
        "Plugin > 1.0",
        "Plugin \"quoted\"",
        "Plugin\ttabbed",
    ];

    for name in special_names {
        let name_escaped = serde_json::to_string(name);
        assert!(
            name_escaped.is_ok(),
            "Serialization should handle special chars"
        );
    }
}

/// Test KVR URL regex with edge cases
#[test]
fn test_kvr_url_extraction_edge_cases() {
    let test_cases = vec![
        ("Copyright © Copyright 2024, Some Plugin by PluginCo. Please download from https://www.example.com/plugins", Some("https://www.example.com/plugins")),
        ("Copyright 2024, Plugin Co. Visit http://example.org", Some("http://example.org")),
        ("No URL in copyright", None),
        ("Mixed: https://foo.com and http://bar.org", Some("https://foo.com")), // Should match first
        ("Multiple URLs: https://a.com, https://b.com", Some("https://a.com")),
    ];

    for (copyright, expected) in test_cases {
        let result = URL_RE.find(copyright).map(|m| m.as_str().to_string());
        assert_eq!(
            result,
            expected.map(|s| s.to_string()),
            "Failed for: {}",
            copyright
        );
    }
}

/// Test version string edge cases
#[test]
fn test_version_parsing_edge_cases() {
    let pairs = vec![
        // Equal versions (same or different formatting)
        (compare_versions("1.0", "1.0"), std::cmp::Ordering::Equal),
        (compare_versions("1.0.0", "1.0"), std::cmp::Ordering::Equal),
        (
            compare_versions("1.0.0.0", "1.0.0"),
            std::cmp::Ordering::Equal,
        ),
        // Greater
        (compare_versions("2.0", "1.0"), std::cmp::Ordering::Greater),
        (compare_versions("1.1", "1.0"), std::cmp::Ordering::Greater),
        (compare_versions("10.0", "9.0"), std::cmp::Ordering::Greater),
        // Less
        (compare_versions("0.9", "1.0"), std::cmp::Ordering::Less),
        (compare_versions("1.0", "2.0"), std::cmp::Ordering::Less),
        // Edge cases
        (compare_versions("1.0.0", "1.0.1"), std::cmp::Ordering::Less),
    ];

    for (result, expected) in pairs {
        assert_eq!(result, expected);
    }
}

/// Test plugin type detection edge cases
#[test]
fn test_plugin_type_detection() {
    assert_eq!(scanner::get_plugin_type(".vst"), "VST2");
    assert_eq!(scanner::get_plugin_type(".vst3"), "VST3");
    assert_eq!(scanner::get_plugin_type(".component"), "AU");
    assert_eq!(scanner::get_plugin_type(".dll"), "VST2");
    assert_eq!(scanner::get_plugin_type(".clap"), "Unknown");
    assert_eq!(scanner::get_plugin_type(".aaxplugin"), "Unknown");
    assert_eq!(scanner::get_plugin_type(".unknown"), "Unknown");
}

/// Test size formatting edge cases
#[test]
fn test_size_formatting_edge_cases() {
    assert_eq!(format_size(0), "0 B");
    assert_eq!(format_size(1), "1.0 B");
    assert_eq!(format_size(10), "10.0 B");
    assert_eq!(format_size(100), "100.0 B");
    assert_eq!(format_size(1023), "1023.0 B");
    assert_eq!(format_size(1024), "1.0 KB");
    assert_eq!(format_size(10239), "10.0 KB");
    assert_eq!(format_size(10240), "10.0 KB");
    assert_eq!(format_size(102400), "100.0 KB");
    // `format_size` uses IEC-style 1024^n tiers; use exact powers of 1024 for MB/GB labels.
    assert_eq!(format_size(1024 * 1024), "1.0 MB");
    assert_eq!(format_size(100 * 1024 * 1024), "100.0 MB");
    assert_eq!(format_size(1024 * 1024 * 1024), "1.0 GB");
    assert_eq!(format_size(10240000000), "9.5 GB");
}

/// Test path handling with unicode
#[test]
fn test_unicode_paths() {
    let temp = std::env::temp_dir().join("测试插件"); // Unicode Chinese
    let _ = std::fs::create_dir_all(&temp);

    let plugin = temp.join("插件.vst3");
    let _ = std::fs::create_dir_all(&plugin);

    let info = scanner::get_plugin_info(plugin.as_path());
    assert!(info.is_some());

    let _ = std::fs::remove_dir_all(temp);
}
