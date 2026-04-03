//! `PluginInfo` JSON round-trip (matches frontend / scanner contract).

#[test]
fn test_plugin_info_json_roundtrip() {
    use app_lib::scanner::PluginInfo;

    let original = PluginInfo {
        name: "FabFilter Pro-Q3".into(),
        path: "/Library/Audio/Plug-Ins/VST3/Pro-Q3.vst3".into(),
        plugin_type: "VST3".into(),
        version: "3.5.0".into(),
        manufacturer: "FabFilter".into(),
        manufacturer_url: Some("https://fabfilter.com".into()),
        size: "12 MB".into(),
        size_bytes: 12_000_000,
        modified: "2024-01-01T00:00:00Z".into(),
        architectures: vec!["arm64".into(), "x86_64".into()],
    };

    let json = serde_json::to_string(&original).unwrap();
    let deserialized: PluginInfo = serde_json::from_str(&json).unwrap();

    assert_eq!(original.name, deserialized.name);
    assert_eq!(original.plugin_type, deserialized.plugin_type);
    assert_eq!(original.path, deserialized.path);
    assert_eq!(original.manufacturer_url, deserialized.manufacturer_url);
}
