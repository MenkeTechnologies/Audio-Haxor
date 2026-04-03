//! History: `gen_id`, `radix_string`, `ScanSnapshot`, `compute_plugin_diff`.

#[test]
fn test_snapshot_id_and_radix() {
    assert_eq!(app_lib::history::radix_string(0, 35), "0");
    assert_eq!(app_lib::history::radix_string(34, 35), "y");
    assert_eq!(app_lib::history::radix_string(35, 36), "z");

    let ids: Vec<String> = (0..100u32).map(|_| app_lib::history::gen_id()).collect();
    let unique: std::collections::HashSet<&String> = ids.iter().collect();
    assert_eq!(ids.len(), unique.len());

    let plugin = app_lib::scanner::PluginInfo {
        name: "TestVST".into(),
        path: "/test.vst3".into(),
        plugin_type: "VST3".into(),
        version: "1.0.0".into(),
        manufacturer: "Test".into(),
        manufacturer_url: None,
        size: "1 MB".into(),
        size_bytes: 1_048_576,
        modified: "2024-04-03".into(),
        architectures: vec![],
    };
    let snapshot = app_lib::history::ScanSnapshot {
        id: app_lib::history::gen_id(),
        timestamp: "2024-04-03T00:00:00Z".into(),
        plugin_count: 1,
        plugins: vec![plugin],
        directories: vec![],
        roots: vec![],
    };
    assert!(!snapshot.id.is_empty());
    assert!(snapshot.id.chars().all(|c: char| c.is_alphanumeric()));
}

#[test]
fn test_diff_compute_same_paths_version_change() {
    let old_plugin = app_lib::scanner::PluginInfo {
        name: "Plugin A".into(),
        path: "/vst/a.vst3".into(),
        plugin_type: "VST3".into(),
        version: "1.0.0".into(),
        manufacturer: "M".into(),
        manufacturer_url: None,
        size: "1 MB".into(),
        size_bytes: 1_048_576,
        modified: "2024-01-01".into(),
        architectures: vec![],
    };
    let new_plugin = app_lib::scanner::PluginInfo {
        name: "Plugin A".into(),
        path: "/vst/a.vst3".into(),
        plugin_type: "VST3".into(),
        version: "2.0.0".into(),
        manufacturer: "M".into(),
        manufacturer_url: None,
        size: "2 MB".into(),
        size_bytes: 2_097_152,
        modified: "2024-04-03".into(),
        architectures: vec![],
    };
    let old_snap = app_lib::history::build_plugin_snapshot(&[old_plugin], &[], &[]);
    let new_snap = app_lib::history::build_plugin_snapshot(&[new_plugin], &[], &[]);
    let diff = app_lib::history::compute_plugin_diff(&old_snap, &new_snap);
    assert_eq!(diff.version_changed.len(), 1);
    assert_eq!(diff.added.len(), 0);
    assert_eq!(diff.removed.len(), 0);
}

#[test]
fn test_rfc3339_millis_z_shape() {
    let ts = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    assert!(ts.contains('T'));
    assert!(ts.ends_with('Z'));
}
