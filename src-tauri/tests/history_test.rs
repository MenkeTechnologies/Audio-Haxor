use app_lib::history::{KvrCacheEntry, ScanSnapshot, ScanSummary};
use app_lib::scanner::PluginInfo;

#[test]
fn test_scan_snapshot_serde() {
    let info = PluginInfo {
        name: "TestPlugin".to_string(),
        path: "/test.vst".to_string(),
        plugin_type: "VST3".to_string(),
        version: "1.0.0".to_string(),
        manufacturer: "Test Inc".to_string(),
        manufacturer_url: Some("https://test.com".to_string()),
        size: "1 MB".to_string(),
        size_bytes: 1_048_576,
        modified: "2024-01-01".to_string(),
        architectures: vec![],
    };

    let plugins = vec![info];
    let snapshot = ScanSnapshot {
        id: "test-001".to_string(),
        timestamp: "2024-01-01T00:00:00Z".to_string(),
        plugin_count: 1,
        plugins,
        directories: vec!["/Library/Audio/Plug-Ins/VST".to_string()],
        roots: vec![],
    };

    let json = serde_json::to_value(&snapshot).unwrap();
    assert_eq!(json["id"], "test-001");
    assert_eq!(json["pluginCount"], 1);
}

#[test]
fn test_scan_summary() {
    let summary = ScanSummary {
        id: "test-002".to_string(),
        timestamp: "2024-01-02T00:00:00Z".to_string(),
        plugin_count: 5,
        roots: vec!["/Library/Audio".to_string()],
    };

    assert_eq!(summary.plugin_count, 5);
    assert_eq!(summary.roots.len(), 1);
}

#[test]
fn test_kvr_cache_entry() {
    let entry = KvrCacheEntry {
        kvr_url: Some("https://kvraudio.com/test-plugin".to_string()),
        update_url: Some("https://kvraudio.com/download/test-plugin".to_string()),
        latest_version: Some("2.1.0".to_string()),
        has_update: true,
        source: "kvraudio".to_string(),
        timestamp: "2024-01-03T00:00:00Z".to_string(),
    };

    assert!(entry.has_update);
    assert!(entry.kvr_url.is_some());
    assert_eq!(entry.latest_version, Some("2.1.0".to_string()));
}

#[test]
fn test_kvr_cache_entry_defaults() {
    let entry = KvrCacheEntry {
        kvr_url: None,
        update_url: None,
        latest_version: None,
        has_update: false,
        source: "not-found".to_string(),
        timestamp: "2024-01-04T00:00:00Z".to_string(),
    };

    assert!(!entry.has_update);
    assert!(entry.kvr_url.is_none());
    assert_eq!(entry.source, "not-found");
}
