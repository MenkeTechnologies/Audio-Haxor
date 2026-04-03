//! Tests for Tauri command layer functions

#[test]
fn test_command_layer_format_size() {
    assert_eq!(app_lib::format_size(0), "0 B");
    assert_eq!(app_lib::format_size(100), "100.0 B");
    assert_eq!(app_lib::format_size(1024), "1.0 KB");
    assert_eq!(app_lib::format_size(1048576), "1.0 MB");
}

#[test]
fn test_command_layer_crate_version_is_semver() {
    let v = env!("CARGO_PKG_VERSION");
    assert!(
        v.chars().filter(|c| *c == '.').count() >= 1,
        "expected dotted version, got {v}"
    );
    assert!(v.chars().next().unwrap().is_ascii_digit());
}

#[test]
fn test_command_layer_export_payload() {
    use app_lib::{ExportPayload, ExportPlugin};
    let payload = ExportPayload {
        version: "1.0".to_string(),
        exported_at: "2024-01-01".to_string(),
        plugins: vec![ExportPlugin {
            name: "X".into(),
            plugin_type: "VST3".into(),
            version: "1".into(),
            manufacturer: "M".into(),
            manufacturer_url: None,
            path: "/p.vst3".into(),
            size: "1 B".into(),
            size_bytes: 1,
            modified: "t".into(),
            architectures: vec![],
        }],
    };
    let json = serde_json::to_value(&payload).unwrap();
    assert_eq!(json["version"], "1.0");
    assert!(json.get("plugins").unwrap().as_array().unwrap().len() == 1);
}

#[test]
fn test_command_layer_sort_plugins() {
    use std::collections::HashMap;
    
    let mut plugins = HashMap::new();
    plugins.insert("Zebra VST".to_string(), "/plugin.vst".to_string());
    plugins.insert("Apple VST".to_string(), "/plugin2.vst".to_string());
    plugins.insert("Banana VST".to_string(), "/plugin3.vst".to_string());
    
    let mut sorted: Vec<String> = plugins.keys().cloned().collect();
    sorted.sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()));
    
    // Check alphabetical order (case-insensitive sort, original casing preserved)
    assert!(sorted[0].to_lowercase().contains("apple"));
    assert!(sorted[1].to_lowercase().contains("banana"));
    assert!(sorted[2].to_lowercase().contains("zebra"));
}
