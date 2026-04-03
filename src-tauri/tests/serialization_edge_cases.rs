//! Serde edge cases for `PluginInfo` and `MidiInfo`.

#[test]
fn test_plugin_info_empty_manufacturer_serializes() {
    let info = app_lib::scanner::PluginInfo {
        name: "Test Plugin".into(),
        path: "/test.vst3".into(),
        plugin_type: "VST3".into(),
        version: "1.0.0".into(),
        manufacturer: "".into(),
        manufacturer_url: Some("https://unknown.com".into()),
        size: "1 MB".into(),
        size_bytes: 1_048_576,
        modified: "2024-01-01".into(),
        architectures: vec![],
    };
    let json = serde_json::to_string(&info).unwrap();
    assert!(json.contains("Test Plugin"));
    let back: app_lib::scanner::PluginInfo = serde_json::from_str(&json).unwrap();
    assert_eq!(back.manufacturer, "");
}

#[test]
fn test_plugin_info_roundtrip_special_names() {
    for name in [
        "My Plugin v1.0",
        "Plugin (Custom)",
        "Plugin - Pro",
        "Plugin_3",
    ] {
        let info = app_lib::scanner::PluginInfo {
            name: name.into(),
            path: "/test.vst3".into(),
            plugin_type: "VST3".into(),
            version: "1.0.0".into(),
            manufacturer: "Test".into(),
            manufacturer_url: None,
            size: "1 MB".into(),
            size_bytes: 0,
            modified: "2024-01-01".into(),
            architectures: vec![],
        };
        let json = serde_json::to_string(&info).unwrap();
        let back: app_lib::scanner::PluginInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(back.name, name);
    }
}

#[test]
fn test_midi_info_default_serializes() {
    let midi = app_lib::midi::MidiInfo::default();
    let json = serde_json::to_value(&midi).unwrap();
    assert!(json.get("format").is_some());
}
