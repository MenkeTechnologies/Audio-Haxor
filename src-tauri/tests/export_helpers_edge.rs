use app_lib::{ExportPayload, ExportPlugin, format_size};

#[test]
fn test_format_size() {
    // Zero bytes
    assert_eq!(format_size(0), "0 B");

    // Small sizes
    assert_eq!(format_size(1024), "1.0 KB");
    assert_eq!(format_size(1024 * 1024), "1.0 MB");
    assert_eq!(format_size(1024 * 1024 * 1024), "1.0 GB");
    assert_eq!(format_size(1024 * 1024 * 1024 * 1024), "1.0 TB");

    // Fractional values
    assert_eq!(format_size(1536), "1.5 KB");
    assert_eq!(format_size(512 * 1024), "512.0 KB");

    // Large files
    assert_eq!(format_size(5 * 1024 * 1024 * 1024), "5.0 GB");

    // Near boundary
    assert_eq!(format_size(1024 * 1024 * 1024 - 1), "1024.0 MB");
}

#[test]
fn test_export_payload() {
    let payload = ExportPayload {
        version: "1.11.0".to_string(),
        exported_at: "2024-01-01T12:00:00Z".to_string(),
        plugins: vec![ExportPlugin {
            name: "Test Plugin".to_string(),
            plugin_type: "VST2".to_string(),
            version: "1.0.0".to_string(),
            manufacturer: "TestCo".to_string(),
            manufacturer_url: Some("https://testco.com".to_string()),
            path: "/test.vst".to_string(),
            size: "1.2 MB".to_string(),
            size_bytes: 1_258_291,
            modified: "2024-01-01".to_string(),
            architectures: vec!["x86_64".to_string()],
        }],
    };

    assert_eq!(payload.version, "1.11.0");
    assert_eq!(payload.plugins.len(), 1);
}

#[test]
fn test_export_payload_empty_plugins() {
    let payload = ExportPayload {
        version: "1.0.0".to_string(),
        exported_at: "2024-01-01T12:00:00Z".to_string(),
        plugins: vec![],
    };

    assert!(payload.plugins.is_empty());
}

#[test]
fn test_export_plugin_roundtrip() {
    let plugin = ExportPlugin {
        name: "MyPlugin".to_string(),
        plugin_type: "VST3".to_string(),
        version: "2.0.0".to_string(),
        manufacturer: "Vendor Inc".to_string(),
        manufacturer_url: Some("http://vendor.com".to_string()),
        path: "/MyPlugin.vst3".to_string(),
        size: "2.5 MB".to_string(),
        size_bytes: 2_621_440,
        modified: "2024-03-15".to_string(),
        architectures: vec!["x86_64".to_string(), "arm64".to_string()],
    };

    let json = serde_json::to_value(&plugin).unwrap();
    assert_eq!(json["name"], "MyPlugin");
    assert_eq!(json["type"], "VST3");
    assert_eq!(json["architectures"].as_array().unwrap().len(), 2);
}

#[test]
fn test_export_plugin_no_optional_fields() {
    let plugin = ExportPlugin {
        name: "SimplePlugin".to_string(),
        plugin_type: "AU".to_string(),
        version: "1.0".to_string(),
        manufacturer: "Simple Co".to_string(),
        manufacturer_url: None,
        path: "/simple.component".to_string(),
        size: "500 KB".to_string(),
        size_bytes: 512_000,
        modified: "2024-01-01".to_string(),
        architectures: Vec::new(),
    };

    assert!(plugin.manufacturer_url.is_none());
    assert!(plugin.architectures.is_empty());
}

#[test]
fn test_multiple_plugins_export() {
    let payload = ExportPayload {
        version: "1.0.0".to_string(),
        exported_at: "2024-01-01T12:00:00Z".to_string(),
        plugins: vec![
            ExportPlugin {
                name: "Plugin1".to_string(),
                plugin_type: "VST2".to_string(),
                version: "1.0".to_string(),
                manufacturer: "Co1".to_string(),
                manufacturer_url: None,
                path: "/plugin1.vst".to_string(),
                size: "1.0 MB".to_string(),
                size_bytes: 1_048_576,
                modified: "2024-01-01".to_string(),
                architectures: vec![],
            },
            ExportPlugin {
                name: "Plugin2".to_string(),
                plugin_type: "VST3".to_string(),
                version: "2.0".to_string(),
                manufacturer: "Co2".to_string(),
                manufacturer_url: None,
                path: "/plugin2.vst3".to_string(),
                size: "2.0 MB".to_string(),
                size_bytes: 2_097_152,
                modified: "2024-01-02".to_string(),
                architectures: vec![],
            },
            ExportPlugin {
                name: "Plugin3".to_string(),
                plugin_type: "AU".to_string(),
                version: "3.0".to_string(),
                manufacturer: "Co3".to_string(),
                manufacturer_url: None,
                path: "/plugin3.component".to_string(),
                size: "3.0 MB".to_string(),
                size_bytes: 3_145_728,
                modified: "2024-01-03".to_string(),
                architectures: vec![],
            },
        ],
    };

    assert_eq!(payload.plugins.len(), 3);
    assert_eq!(payload.plugins[0].name, "Plugin1");
    assert_eq!(payload.plugins[2].plugin_type, "AU");
}

#[test]
fn test_export_version_formatting() {
    let a = ExportPayload {
        version: "1.11.0".to_string(),
        exported_at: "2024-01-01T12:00:00Z".to_string(),
        plugins: vec![],
    };
    let b = ExportPayload {
        version: "1.11.0-test".to_string(),
        exported_at: "2024-01-01T12:00:00Z".to_string(),
        plugins: vec![],
    };
    assert_eq!(a.version, "1.11.0");
    assert!(b.version.contains("test"));
}

#[test]
fn test_export_payload_deserialize() {
    let json = r#"{
        "version": "1.0.0",
        "exported_at": "2024-01-01T12:00:00Z",
        "plugins": []
    }"#;

    let payload: ExportPayload = serde_json::from_str(json).unwrap();
    assert_eq!(payload.version, "1.0.0");
    assert!(payload.plugins.is_empty());
}

#[test]
fn test_export_payload_full_json_roundtrip() {
    let payload = ExportPayload {
        version: "1.0.0".to_string(),
        exported_at: "2024-01-01T12:00:00Z".to_string(),
        plugins: vec![ExportPlugin {
            name: "Test".to_string(),
            plugin_type: "VST2".to_string(),
            version: "1.0".to_string(),
            manufacturer: "Test".to_string(),
            manufacturer_url: None,
            path: "/test.vst".to_string(),
            size: "1.0 MB".to_string(),
            size_bytes: 1_048_576,
            modified: "2024-01-01".to_string(),
            architectures: vec![],
        }],
    };

    let json = serde_json::to_string(&payload).unwrap();
    let deserialized: ExportPayload = serde_json::from_str(&json).unwrap();
    let v1 = serde_json::to_value(&payload).unwrap();
    let v2 = serde_json::to_value(&deserialized).unwrap();
    assert_eq!(v1, v2);
}
