//! Scanner: `get_plugin_type` edge cases and `PluginInfo` sanity checks.

use app_lib::scanner::{PluginInfo, get_plugin_type};

#[test]
fn test_get_plugin_type_comprehensive() {
    assert_eq!(get_plugin_type(".vst2"), "Unknown");
    assert_eq!(get_plugin_type(".vst3"), "VST3");
    assert_eq!(get_plugin_type(".component"), "AU");
    assert_eq!(get_plugin_type(".dll"), "VST2");
    assert_eq!(get_plugin_type(".vst"), "VST2");
    assert_eq!(get_plugin_type(".VST"), "Unknown");

    assert_eq!(get_plugin_type(".so"), "Unknown");
    assert_eq!(get_plugin_type(".dylib"), "Unknown");
    assert_eq!(get_plugin_type(".bundle"), "Unknown");
    assert_eq!(get_plugin_type(".preset"), "Unknown");
    assert_eq!(get_plugin_type(""), "Unknown");
    assert_eq!(get_plugin_type("."), "Unknown");
}

#[test]
fn test_plugin_type_table() {
    let test_cases = [
        (".vst2", "Unknown", true),
        (".vst3", "VST3", false),
        (".component", "AU", false),
        (".dll", "VST2", false),
        (".vst", "VST2", false),
        (".unknown_ext", "Unknown", true),
        (".wav", "Unknown", true),
        (".mp3", "Unknown", true),
    ];

    for (ext, expected_type, should_be_unknown) in test_cases {
        let result = get_plugin_type(ext);
        if should_be_unknown {
            assert_eq!(result, "Unknown", "Expected Unknown for {:?}", ext);
        } else {
            assert_eq!(result, expected_type, "Mismatch for {:?}", ext);
        }
    }
}

#[test]
fn test_plugin_info_constructed_fields() {
    let plugin = PluginInfo {
        name: "TestPlugin.x64".to_string(),
        path: "/test.vst".to_string(),
        plugin_type: "VST2".to_string(),
        version: "1.0".to_string(),
        manufacturer: "TestCo".to_string(),
        manufacturer_url: Some("https://testco.com".to_string()),
        size: "100 KB".to_string(),
        size_bytes: 100 * 1024,
        modified: "2024-01-01".to_string(),
        architectures: vec!["x86_64".to_string()],
    };

    assert_eq!(plugin.name, "TestPlugin.x64");
    assert!(plugin.path.contains(".vst"));
    assert!(plugin.architectures.contains(&"x86_64".to_string()));
}
