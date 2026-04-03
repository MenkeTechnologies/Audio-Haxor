use app_lib::scanner::{get_plugin_type, PluginInfo};

#[test]
fn test_get_plugin_type() {
    assert_eq!(get_plugin_type(".vst"), "VST2");
    assert_eq!(get_plugin_type(".vst3"), "VST3");
    assert_eq!(get_plugin_type(".component"), "AU");
    assert_eq!(get_plugin_type(".dll"), "VST2");
    assert_eq!(get_plugin_type(".unknown"), "Unknown");
}

#[test]
fn test_plugin_info_struct() {
    let info = PluginInfo {
        name: "Test Plugin".to_string(),
        path: "/Library/Audio/Plug-Ins/VST/test.vst".to_string(),
        plugin_type: "VST3".to_string(),
        version: "1.0.0".to_string(),
        manufacturer: "Test Co".to_string(),
        manufacturer_url: Some("https://test.com".to_string()),
        size: "1.2 MB".to_string(),
        size_bytes: 1_258_291,
        modified: "2024-01-01 12:00:00".to_string(),
        architectures: vec!["x86_64".to_string()],
    };

    assert_eq!(info.name, "Test Plugin");
    assert_eq!(info.plugin_type, "VST3");
    assert_eq!(info.architectures.len(), 1);
}

#[test]
fn test_plugin_info_default_architectures() {
    let info = PluginInfo {
        name: "Test".to_string(),
        path: "/test.vst".to_string(),
        plugin_type: "VST2".to_string(),
        version: "1.0".to_string(),
        manufacturer: "Test".to_string(),
        manufacturer_url: None,
        size: "1 MB".to_string(),
        size_bytes: 0,
        modified: "2024-01-01".to_string(),
        architectures: Vec::new(),
    };

    assert!(info.architectures.is_empty());
}

#[test]
fn test_plugin_info_serialize() {
    let info = PluginInfo {
        name: "MyVST".to_string(),
        path: "/path/to/vst.vst3".to_string(),
        plugin_type: "VST3".to_string(),
        version: "2.0".to_string(),
        manufacturer: "Vendor Inc".to_string(),
        manufacturer_url: Some("http://vendor.com".to_string()),
        size: "5.5 MB".to_string(),
        size_bytes: 5_767_168,
        modified: "2024-03-15 10:30:00".to_string(),
        architectures: vec!["x86_64".to_string(), "arm64".to_string()],
    };

    let json = serde_json::to_value(&info).unwrap();
    assert_eq!(json["name"], "MyVST");
    assert_eq!(json["version"], "2.0");
    assert_eq!(json["architectures"].as_array().unwrap().len(), 2);
}
