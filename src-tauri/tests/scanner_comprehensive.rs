//! Comprehensive scanner: `PluginInfo` serde and `get_plugin_type`.

#[test]
fn test_plugin_info_complete_validation() {
    use app_lib::scanner::PluginInfo;
    let plugin = PluginInfo {
        name: "FabFilter Pro-Q3".into(),
        path: "/Library/Audio/Plug-Ins/VST3/Pro-Q3.vst3".into(),
        plugin_type: "VST3".into(),
        version: "3.5.0".into(),
        manufacturer: "FabFilter".into(),
        manufacturer_url: Some("https://www.fabfilter.com".into()),
        size: "12.8 MB".into(),
        size_bytes: 13_483_264,
        modified: "2024-03-15T10:30:00Z".into(),
        architectures: vec!["x86_64".into(), "arm64".into()],
    };
    let json = serde_json::to_string(&plugin).unwrap();
    let deserialized: PluginInfo = serde_json::from_str(&json).unwrap();
    assert_eq!(plugin.name, deserialized.name);
    assert_eq!(plugin.plugin_type, deserialized.plugin_type);
    assert_eq!(plugin.version, deserialized.version);
    assert_eq!(plugin.manufacturer, deserialized.manufacturer);
    assert_eq!(plugin.architectures, deserialized.architectures);
}

#[test]
fn test_plugin_info_edge_names_roundtrip_json() {
    use app_lib::scanner::PluginInfo;
    for (name, mfg) in [
        ("", ""),
        (
            "Super Ultra Mega Plugin With Many Many Many Many Many Many Many Many Words",
            "Manufacturer",
        ),
    ] {
        let p = PluginInfo {
            name: name.into(),
            path: "/test.vst".into(),
            plugin_type: "VST3".into(),
            version: "1.0".into(),
            manufacturer: mfg.into(),
            manufacturer_url: None,
            size: "1 MB".into(),
            size_bytes: 0,
            modified: "2024-01-01".into(),
            architectures: vec![],
        };
        let json = serde_json::to_string(&p).unwrap();
        let back: PluginInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(back.name, name);
    }
}

#[test]
fn test_scanner_type_detection() {
    use app_lib::scanner::get_plugin_type;
    assert_eq!(get_plugin_type(".vst"), "VST2");
    assert_eq!(get_plugin_type(".vst3"), "VST3");
    assert_eq!(get_plugin_type(".component"), "AU");
    assert_eq!(get_plugin_type(".dll"), "VST2");
    assert_eq!(get_plugin_type(".unknown"), "Unknown");
    assert_eq!(get_plugin_type(""), "Unknown");
    assert_eq!(get_plugin_type("wav"), "Unknown");
}
