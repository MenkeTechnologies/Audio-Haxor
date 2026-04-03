//! Smoke: `format_size`, `PluginInfo`, audio path extensions.

#[test]
fn test_format_size_common_values() {
    assert_eq!(app_lib::format_size(0), "0 B");
    assert_eq!(app_lib::format_size(1), "1.0 B");
    assert_eq!(app_lib::format_size(1024), "1.0 KB");
    assert_eq!(app_lib::format_size(2048), "2.0 KB");
    assert_eq!(app_lib::format_size(1024 * 1024), "1.0 MB");
    assert_eq!(app_lib::format_size(2048 * 1024), "2.0 MB");
    assert_eq!(app_lib::format_size(1024 * 1024 * 1024), "1.0 GB");
}

#[test]
fn test_plugin_architectures_variations() {
    let plugin_no_arch = app_lib::scanner::PluginInfo {
        name: "Legacy Plugin".into(),
        path: "/test.vst".into(),
        plugin_type: "VST2".into(),
        version: "1.0".into(),
        manufacturer: "Legacy".into(),
        manufacturer_url: None,
        size: "1 MB".into(),
        size_bytes: 1_048_576,
        modified: "2024-01-01".into(),
        architectures: vec![],
    };
    assert!(plugin_no_arch.architectures.is_empty());

    let plugin_x64 = app_lib::scanner::PluginInfo {
        name: "x64 Plugin".into(),
        path: "/test.vst3".into(),
        plugin_type: "VST3".into(),
        version: "2.0".into(),
        manufacturer: "Co".into(),
        manufacturer_url: None,
        size: "1 MB".into(),
        size_bytes: 1_048_576,
        modified: "2024-01-01".into(),
        architectures: vec!["x86_64".into()],
    };
    assert_eq!(plugin_x64.architectures.len(), 1);
    assert_eq!(plugin_x64.architectures[0], "x86_64");
}
