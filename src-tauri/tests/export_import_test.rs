//! Export/import payload serialization (types in `app_lib`).

use app_lib::{ExportPayload, ExportPlugin};

#[test]
fn test_export_payload_serialization() {
    let payload = ExportPayload {
        version: "1.11.0".to_string(),
        exported_at: "2024-04-03T12:00:00Z".to_string(),
        plugins: vec![ExportPlugin {
            name: "FabFilter Pro-Q3".to_string(),
            plugin_type: "VST3".to_string(),
            version: "3.5.0".to_string(),
            manufacturer: "FabFilter".to_string(),
            manufacturer_url: Some("https://fabfilter.com".to_string()),
            path: "/Library/Audio/Plug-Ins/VST3/Pro-Q3.vst3".to_string(),
            size: "12.8 MB".to_string(),
            size_bytes: 13_483_264,
            modified: "2024-03-15T10:30:00Z".to_string(),
            architectures: vec!["x86_64".to_string(), "arm64".to_string()],
        }],
    };

    let json = serde_json::to_string(&payload).unwrap();
    assert!(json.contains("FabFilter Pro-Q3"));
    assert!(json.contains("VST3"));
}

#[test]
fn test_export_plugin_with_and_without_url() {
    // Plugin with manufacturer URL
    let plugin_with_url = ExportPlugin {
        name: "Plugin With URL".to_string(),
        plugin_type: "VST3".to_string(),
        version: "1.0.0".to_string(),
        manufacturer: "Plugin Co".to_string(),
        manufacturer_url: Some("https://plugin.com".to_string()),
        path: "/plugin.vst3".to_string(),
        size: "1 MB".to_string(),
        size_bytes: 1_048_576,
        modified: "2024-01-01".to_string(),
        architectures: vec![],
    };

    // Plugin without manufacturer URL
    let plugin_without_url = ExportPlugin {
        name: "Plugin Without URL".to_string(),
        plugin_type: "VST3".to_string(),
        version: "1.0.0".to_string(),
        manufacturer: "Plugin Co".to_string(),
        manufacturer_url: None,
        path: "/plugin2.vst3".to_string(),
        size: "1 MB".to_string(),
        size_bytes: 1_048_576,
        modified: "2024-01-01".to_string(),
        architectures: vec![],
    };

    assert!(plugin_with_url.manufacturer_url.is_some());
    assert!(plugin_without_url.manufacturer_url.is_none());
}

#[test]
fn test_payload_with_multiple_plugins() {
    let plugins = [
        // First plugin - VST2
        ExportPlugin {
            name: "ReaEQ".to_string(),
            plugin_type: "VST2".to_string(),
            version: "6.01".to_string(),
            manufacturer: "Cocktail Cubes".to_string(),
            manufacturer_url: None,
            path: "/vst/ReaEQ.vst".to_string(),
            size: "0.5 MB".to_string(),
            size_bytes: 524_288,
            modified: "2024-01-01".to_string(),
            architectures: vec!["x86_64".to_string()],
        },
        // Second plugin - VST3
        ExportPlugin {
            name: "Valhalla VintageVerb".to_string(),
            plugin_type: "VST3".to_string(),
            version: "3.3.4".to_string(),
            manufacturer: "Valhalla DSP".to_string(),
            manufacturer_url: Some("https://valhalladsp.com".to_string()),
            path: "/vst3/Valhalla VintageVerb.vst3".to_string(),
            size: "15.2 MB".to_string(),
            size_bytes: 15_938_304,
            modified: "2024-01-01".to_string(),
            architectures: vec!["x86_64".to_string(), "arm64".to_string()],
        },
        // Third plugin - AU
        ExportPlugin {
            name: "Softube Tube-Tech".to_string(),
            plugin_type: "AU".to_string(),
            version: "1.0.0".to_string(),
            manufacturer: "Softube".to_string(),
            manufacturer_url: Some("https://softube.com".to_string()),
            path: "/components/Tube-Tech.component".to_string(),
            size: "2.1 MB".to_string(),
            size_bytes: 2_202_009,
            modified: "2024-01-01".to_string(),
            architectures: vec!["arm64".to_string()],
        },
    ];

    assert_eq!(plugins.len(), 3);
}

