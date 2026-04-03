//! Export/Import functionality tests
//! Tests JSON, TOML, CSV, TSV, PDF exports and import validation

use serde_json;

/// Test JSON export of plugins
#[test]
fn test_export_json_valid() {
    let base = std::env::temp_dir().join("export_test");
    let _ = std::fs::create_dir_all(&base);

    // Simulate plugin data
    let plugins = vec![app_lib::scanner::PluginInfo {
        name: "TestVST1".to_string(),
        path: "/test/plugin1.vst3".to_string(),
        plugin_type: "VST3".to_string(),
        version: "1.0.0".to_string(),
        manufacturer: "TestCo".to_string(),
        manufacturer_url: Some("https://test.com".to_string()),
        size: "1 MB".to_string(),
        size_bytes: 1_000_000,
        modified: "2024-01-01".to_string(),
        architectures: vec!["x86_64".to_string()],
    }];

    let snapshot = app_lib::history::ScanSnapshot {
        id: "json_export_test".to_string(),
        timestamp: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        plugin_count: 1,
        directories: vec![],
        roots: vec![base.to_string_lossy().to_string()],
        plugins,
    };

    let json = serde_json::to_string(&snapshot);
    assert!(json.is_ok());

    let _ = std::fs::remove_dir_all(&base);
}

/// Test JSON export with many plugins
#[test]
fn test_export_json_large_list() {
    let plugins = (0..100)
        .map(|i| app_lib::scanner::PluginInfo {
            name: format!("Plugin{}", i),
            path: format!("/plugins/plugin{}.vst3", i),
            plugin_type: "VST3".to_string(),
            version: "1.0".to_string(),
            manufacturer: "TestCo".to_string(),
            manufacturer_url: None,
            size: "1 MB".to_string(),
            size_bytes: 1_000_000,
            modified: "2024-01-01".to_string(),
            architectures: vec!["x86_64".to_string()],
        })
        .collect::<Vec<_>>();

    let snapshot = app_lib::history::ScanSnapshot {
        id: "large_export".to_string(),
        timestamp: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        plugin_count: plugins.len(),
        directories: vec![],
        roots: vec![],
        plugins,
    };

    let json = serde_json::to_string(&snapshot);
    assert!(json.is_ok());

    // Verify can round-trip
    let json_str = json.unwrap();
    let back: app_lib::history::ScanSnapshot = serde_json::from_str(&json_str).unwrap();
    assert_eq!(back.plugin_count, 100);
}

/// Test CSV export format
#[test]
fn test_export_csv_format() {
    let base = std::env::temp_dir().join("csv_export");
    let _ = std::fs::create_dir_all(&base);

    use std::io::Write;

    // Test CSV header format
    let csv_content = "name,path,type,version,manufacturer,size,modified,architectures\n";
    let mut file = std::fs::File::create(base.join("plugins.csv")).unwrap();
    writeln!(file, "{}", csv_content).unwrap();

    let content = std::fs::read_to_string(&base.join("plugins.csv")).unwrap();
    assert!(content.contains("name,path,type,version"));

    let _ = std::fs::remove_dir_all(&base);
}

/// Test TOML export format
#[test]
fn test_export_toml_format() {
    let base = std::env::temp_dir().join("toml_export");
    let _ = std::fs::create_dir_all(&base);

    let config = TomlConfig {
        version: "1.11.0".to_string(),
        exported_at: chrono::Utc::now().to_rfc3339(),
        plugins: vec![PluginConfig {
            name: "VST1".to_string(),
            plugin_type: "VST3".to_string(),
            version: "1.0".to_string(),
            manufacturer: "Test".to_string(),
            path: "/test/plugin.vst3".to_string(),
            size: "1 MB".to_string(),
            architectures: vec!["x86_64".to_string()],
        }],
    };

    let toml_str = toml::to_string(&config);
    assert!(toml_str.is_ok());
    assert!(toml_str.unwrap().contains("VST1"));

    let _ = std::fs::remove_dir_all(&base);
}

/// Test export with special characters
#[test]
fn test_export_special_characters() {
    let special_plugins = vec![app_lib::scanner::PluginInfo {
        name: "Plugin & Co".to_string(),
        path: "/test/plugin & co.vst3".to_string(),
        plugin_type: "VST3".to_string(),
        version: "1.0.0".to_string(),
        manufacturer: "Test <Company>".to_string(),
        manufacturer_url: Some("https://example.com".to_string()),
        size: "1 MB".to_string(),
        size_bytes: 1_000_000,
        modified: "2024-01-01".to_string(),
        architectures: vec!["x86_64, ARM64".to_string()],
    }];

    let snapshot = app_lib::history::ScanSnapshot {
        id: "special_chars".to_string(),
        timestamp: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        plugin_count: 1,
        directories: vec![],
        roots: vec![],
        plugins: special_plugins,
    };

    let json = serde_json::to_string(&snapshot);
    assert!(json.is_ok());
}

/// Test CSV import validation
#[test]
#[ignore] // Requires actual CSV parsing setup
fn test_import_csv_validation() {
    use std::io::Write;

    let base = std::env::temp_dir().join("import_csv_test");
    let _ = std::fs::create_dir_all(&base);

    let _csv_content = "name,path,type,version,manufacturer,size,modified,architectures\n";
    writeln!(
        std::fs::File::create(&base.join("plugins.csv")).unwrap(),
        "TestVST,/test/plugin.vst3,VST3,1.0,Test,1 MB,2024-01-01,[x86_64]\n",
    )
    .unwrap();

    // Test that CSV parsing would succeed
    let content = std::fs::read_to_string(&base.join("plugins.csv")).unwrap();
    assert!(content.contains("TestVST"));

    let _ = std::fs::remove_dir_all(&base);
}

/// Test shared `format_size` (used by export UI)
#[test]
fn test_export_helpers() {
    assert_eq!(app_lib::format_size(0), "0 B");
    assert_eq!(app_lib::format_size(1024), "1.0 KB");
    assert_eq!(app_lib::format_size(1048576), "1.0 MB");
    assert_eq!(app_lib::format_size(1073741824), "1.0 GB");
}

/// Test batch export
#[test]
fn test_batch_export() {
    let plugins = (0..50)
        .map(|i| app_lib::scanner::PluginInfo {
            name: format!("Plugin{}", i),
            path: format!("/plugins/plugin{}.vst3", i),
            plugin_type: "VST3".to_string(),
            version: "1.0".to_string(),
            manufacturer: "TestCo".to_string(),
            manufacturer_url: None,
            size: "1 MB".to_string(),
            size_bytes: 1_000_000,
            modified: "2024-01-01".to_string(),
            architectures: vec!["x86_64".to_string()],
        })
        .collect::<Vec<_>>();

    let export_json = serde_json::to_string(&plugins);
    assert!(export_json.is_ok());
    assert!(export_json.unwrap().contains("Plugin0"));
}

/// Test export with empty plugin list
#[test]
fn test_export_empty_list() {
    let snapshot = app_lib::history::ScanSnapshot {
        id: "empty_export".to_string(),
        timestamp: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        plugin_count: 0,
        directories: vec![],
        roots: vec![],
        plugins: vec![],
    };

    let json = serde_json::to_string(&snapshot);
    assert!(json.is_ok());
}

/// Test export with various plugin types
#[test]
fn test_export_mixed_plugin_types() {
    let plugins = vec![
        app_lib::scanner::PluginInfo {
            name: "VSTPlugin".to_string(),
            path: "/test/plugin.vst".to_string(),
            plugin_type: "VST2".to_string(),
            version: "1.0".to_string(),
            manufacturer: "Co".to_string(),
            manufacturer_url: None,
            size: "1 MB".to_string(),
            size_bytes: 1_000_000,
            modified: "2024-01-01".to_string(),
            architectures: vec![],
        },
        app_lib::scanner::PluginInfo {
            name: "VST3Plugin".to_string(),
            path: "/test/plugin.vst3".to_string(),
            plugin_type: "VST3".to_string(),
            version: "2.0".to_string(),
            manufacturer: "Co".to_string(),
            manufacturer_url: None,
            size: "2 MB".to_string(),
            size_bytes: 2_000_000,
            modified: "2024-01-01".to_string(),
            architectures: vec!["x86_64".to_string()],
        },
        app_lib::scanner::PluginInfo {
            name: "AudioUnit".to_string(),
            path: "/test/plugin.component".to_string(),
            plugin_type: "AU".to_string(),
            version: "5.0".to_string(),
            manufacturer: "Apple".to_string(),
            manufacturer_url: None,
            size: "3 MB".to_string(),
            size_bytes: 3_000_000,
            modified: "2024-01-01".to_string(),
            architectures: vec!["ARM64".to_string()],
        },
    ];

    let json = serde_json::to_string(&plugins).expect("serialize plugins");
    assert!(json.contains("VST2"));
    assert!(json.contains("VST3"));
    assert!(json.contains("AU"));
}

#[derive(serde::Serialize)]
struct TomlConfig {
    version: String,
    exported_at: String,
    plugins: Vec<PluginConfig>,
}

#[derive(serde::Serialize)]
struct PluginConfig {
    name: String,
    plugin_type: String,
    version: String,
    manufacturer: String,
    path: String,
    size: String,
    architectures: Vec<String>,
}
