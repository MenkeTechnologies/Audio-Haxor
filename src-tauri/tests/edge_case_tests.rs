//! Edge cases: `xref::normalize_plugin_name` and `PluginInfo` JSON with odd strings.

#[test]
fn test_normalize_plugin_name_strips_arch_suffixes() {
    let a = app_lib::xref::normalize_plugin_name("Serum (x64) (VST3)");
    let b = app_lib::xref::normalize_plugin_name("Serum");
    assert_eq!(a, b);
    assert!(a.contains("serum"));
    assert!(!a.contains("x64"));
}

#[test]
fn test_plugin_info_serde_with_special_characters_in_name() {
    let info = app_lib::scanner::PluginInfo {
        name: "Name\twith\nweird\u{00A0}chars".to_string(),
        path: "/tmp/test.vst3".to_string(),
        plugin_type: "VST3".to_string(),
        version: "1.0.0-rc.1".to_string(),
        manufacturer: "Co".to_string(),
        manufacturer_url: None,
        size: "1 B".to_string(),
        size_bytes: 1,
        modified: "2024-01-01".to_string(),
        architectures: vec![],
    };
    let json = serde_json::to_string(&info).unwrap();
    let back: app_lib::scanner::PluginInfo = serde_json::from_str(&json).unwrap();
    assert_eq!(back.name, info.name);
    assert_eq!(back.version, "1.0.0-rc.1");
}
