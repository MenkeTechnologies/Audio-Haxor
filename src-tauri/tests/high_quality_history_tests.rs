use app_lib::history::{ScanDiff, ScanSummary, VersionChangedPlugin as VCP};
use app_lib::scanner::PluginInfo;
use serde_json::json;

fn create_plugin(name: &str, version: &str) -> PluginInfo {
    PluginInfo {
        name: name.into(),
        path: "/p/1".into(),
        plugin_type: "VST".into(),
        version: version.into(),
        manufacturer: "Vendor".into(),
        manufacturer_url: None,
        size: "1 MB".into(),
        size_bytes: 1024,
        modified: "now".into(),
        architectures: vec!["x64".into()],
    }
}

#[test]
fn test_scan_diff_version_logic() {
    let old_summary = ScanSummary {
        id: "1".into(),
        timestamp: "1".into(),
        plugin_count: 1,
        roots: vec![],
    };
    let new_summary = ScanSummary {
        id: "2".into(),
        timestamp: "2".into(),
        plugin_count: 1,
        roots: vec![],
    };

    let p_v2 = create_plugin("Serum", "1.1");

    let diff = ScanDiff {
        old_scan: old_summary,
        new_scan: new_summary,
        added: vec![],
        removed: vec![],
        version_changed: vec![VCP {
            plugin: p_v2.clone(),
            previous_version: "1.0".into(),
        }],
    };

    assert_eq!(diff.version_changed[0].plugin.version, "1.1");
    assert_eq!(diff.version_changed[0].previous_version, "1.0");
}

#[test]
fn test_scan_snapshot_serialization() {
    use app_lib::history::ScanSnapshot;
    let s = ScanSnapshot {
        id: "abc".into(),
        timestamp: "2026".into(),
        plugin_count: 0,
        plugins: vec![],
        directories: vec![],
        roots: vec!["/a".into()],
    };
    let j = serde_json::to_string(&s).unwrap();
    assert!(j.contains("\"pluginCount\":0"));
}

#[test]
fn test_kvr_cache_entry_deser() {
    use app_lib::history::KvrCacheEntry;
    let j = json!({
        "kvrUrl": "http://kvr",
        "updateUrl": "http://update",
        "latestVersion": "2.0",
        "hasUpdate": true,
        "source": "api",
        "timestamp": "now"
    });
    let e: KvrCacheEntry = serde_json::from_value(j).unwrap();
    assert!(e.has_update);
}

#[test]
fn test_daw_project_struct() {
    use app_lib::history::DawProject;
    let p = DawProject {
        name: "Idea".into(),
        path: "/p/i".into(),
        directory: "/p".into(),
        format: "ALS".into(),
        daw: "Ableton".into(),
        size: 1024,
        size_formatted: "1.0 KB".into(),
        modified: "today".into(),
    };
    assert_eq!(p.size_formatted, "1.0 KB");
}

#[test]
fn test_history_roots_default() {
    use app_lib::history::ScanSnapshot;
    let j = json!({"id":"x","timestamp":"x","pluginCount":0,"plugins":[],"directories":[]});
    let s: ScanSnapshot = serde_json::from_value(j).unwrap();
    assert!(s.roots.is_empty());
}

#[test]
fn test_vcp_flatten() {
    let p = create_plugin("x", "2");
    let vcp = VCP {
        plugin: p,
        previous_version: "1".into(),
    };
    let j = serde_json::to_value(&vcp).unwrap();
    assert_eq!(j["name"], "x");
    assert_eq!(j["previousVersion"], "1");
}
