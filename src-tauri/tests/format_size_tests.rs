#[cfg(test)]
mod format_size_tests {
    #[test]
    fn test_format_size_zero() {
        assert_eq!(app_lib::format_size(0), "0 B");
    }

    #[test]
    fn test_format_size_bytes() {
        assert_eq!(app_lib::format_size(512), "512.0 B");
        assert_eq!(app_lib::format_size(1023), "1023.0 B");
    }

    #[test]
    fn test_format_size_kb() {
        assert_eq!(app_lib::format_size(1024), "1.0 KB");
        let kb = 1234;
        let expected = "1.2 KB";
        assert_eq!(app_lib::format_size(kb), expected);
    }

    #[test]
    fn test_format_size_mb() {
        assert_eq!(app_lib::format_size(1024 * 1024), "1.0 MB");
        let mb = 1234567;
        let expected = "1.2 MB";
        assert_eq!(app_lib::format_size(mb), expected);
    }

    #[test]
    fn test_format_size_gb() {
        assert_eq!(app_lib::format_size(1024 * 1024 * 1024), "1.0 GB");
        let gb = 1234567890;
        let expected = "1.1 GB";
        assert_eq!(app_lib::format_size(gb), expected);
    }

    #[test]
    fn test_format_size_tb() {
        assert_eq!(app_lib::format_size(1024 * 1024 * 1024 * 1024), "1.0 TB");
        let tb = 1234567890123;
        let expected = "1.1 TB";
        assert_eq!(app_lib::format_size(tb), expected);
    }

    #[test]
    fn test_format_size_precision() {
        assert!(app_lib::format_size(0).starts_with("0"));
        assert!(app_lib::format_size(1024).contains("KB"));
        assert!(app_lib::format_size(1024 * 1024).contains("MB"));
        assert!(app_lib::format_size(1024 * 1024 * 1024).contains("GB"));
        assert!(app_lib::format_size(1024 * 1024 * 1024 * 1024).contains("TB"));
    }

    #[test]
    fn test_format_size_large_exceeds_tb() {
        // Values larger than TB should be handled gracefully
        let large = 100 * 1024 * 1024 * 1024 * 1024;
        let formatted = app_lib::format_size(large);
        // Should not panic, even if display gets truncated
        assert!(!formatted.is_empty());
    }
}

#[cfg(test)]
mod export_payload_tests {
    use chrono::Utc;

    #[test]
    fn test_export_payload_simple() {
        let plugin = app_lib::ExportPlugin {
            name: "Test Plugin".to_string(),
            plugin_type: "VST3".to_string(),
            version: "1.0.0".to_string(),
            manufacturer: "Test Co".to_string(),
            manufacturer_url: None,
            path: "/test/path".to_string(),
            size: "1.0 MB".to_string(),
            size_bytes: 1048576,
            modified: "2024-01-01".to_string(),
            architectures: Vec::new(),
        };

        let payload = app_lib::ExportPayload {
            version: "1.0".to_string(),
            exported_at: Utc::now().to_rfc3339(),
            plugins: vec![plugin],
        };

        let json = serde_json::to_string(&payload).expect("should serialize");
        assert!(json.contains("\"name\""));
        assert!(json.contains("\"plugins\""));
    }

    #[test]
    fn test_export_payload_roundtrip() {
        let plugin = app_lib::ExportPlugin {
            name: "MyPlugin".to_string(),
            plugin_type: "VST3".to_string(),
            version: "2.0".to_string(),
            manufacturer: "Vendor".to_string(),
            manufacturer_url: Some("https://example.com".to_string()),
            path: "/path/to/plugin".to_string(),
            size: "500.0 KB".to_string(),
            size_bytes: 512000,
            modified: "2024-12-25".to_string(),
            architectures: vec!["x64".to_string(), "arm64".to_string()],
        };

        let payload = app_lib::ExportPayload {
            version: "1.5".to_string(),
            exported_at: Utc::now().to_rfc3339(),
            plugins: vec![plugin.clone()],
        };

        let json = serde_json::to_string(&payload).expect("should serialize");
        let payload2: app_lib::ExportPayload =
            serde_json::from_str(&json).expect("should deserialize");

        assert_eq!(payload.version, payload2.version);
        assert_eq!(payload.plugins.len(), payload2.plugins.len());
        assert_eq!(payload.plugins[0].name, payload2.plugins[0].name);
        assert_eq!(
            payload.plugins[0].size_bytes,
            payload2.plugins[0].size_bytes
        );
    }

    #[test]
    fn test_export_payload_missing_optional_fields() {
        let json = r#"{
            "name": "Simple",
            "type": "AU",
            "version": "1.0.0",
            "manufacturer": "Simple Co",
            "path": "/tmp/Simple.component",
            "size": "0 B",
            "sizeBytes": 0,
            "modified": "2024-01-01"
        }"#;
        let plugin: app_lib::ExportPlugin = serde_json::from_str(json).expect("should deserialize");
        assert_eq!(plugin.plugin_type, "AU");
        assert!(plugin.manufacturer_url.is_none());
        assert!(plugin.architectures.is_empty());
    }

    #[test]
    fn test_export_payload_empty_plugins() {
        let payload = app_lib::ExportPayload {
            version: "1.0".to_string(),
            exported_at: Utc::now().to_rfc3339(),
            plugins: Vec::new(),
        };

        let json = serde_json::to_string(&payload).expect("should serialize");
        let payload2: app_lib::ExportPayload =
            serde_json::from_str(&json).expect("should deserialize");
        assert!(payload2.plugins.is_empty());
    }
}

#[cfg(test)]
mod snapshot_serialization_tests {
    use std::collections::HashMap;

    #[test]
    fn test_scan_snapshot_serialize() {
        let snap = app_lib::history::ScanSnapshot {
            id: String::new(),
            timestamp: String::new(),
            plugin_count: 5,
            plugins: Vec::new(),
            directories: Vec::new(),
            roots: Vec::new(),
        };

        let json = serde_json::to_value(&snap).expect("should serialize");
        assert!(json.get("pluginCount").is_some());
        assert!(json.get("plugins").is_some());
    }

    #[test]
    fn test_scan_snapshot_zero_values() {
        let snap = app_lib::history::ScanSnapshot {
            id: String::new(),
            timestamp: String::new(),
            plugin_count: 0,
            plugins: Vec::new(),
            directories: Vec::new(),
            roots: Vec::new(),
        };

        let json = serde_json::to_value(&snap).expect("should serialize");
        assert_eq!(json.get("pluginCount").and_then(|v| v.as_i64()), Some(0));
    }

    #[test]
    fn test_daw_snapshot_serialize() {
        let snap = app_lib::history::DawScanSnapshot {
            id: String::new(),
            timestamp: String::new(),
            project_count: 10,
            total_bytes: 0,
            daw_counts: HashMap::new(),
            projects: Vec::new(),
            roots: Vec::new(),
        };

        let json = serde_json::to_value(&snap).expect("should serialize");
        assert!(json.get("projectCount").is_some());
    }

    #[test]
    fn test_preset_snapshot_serialize() {
        let snap = app_lib::history::PresetScanSnapshot {
            id: String::new(),
            timestamp: String::new(),
            preset_count: 20,
            total_bytes: 0,
            format_counts: HashMap::new(),
            presets: Vec::new(),
            roots: Vec::new(),
        };

        let json = serde_json::to_value(&snap).expect("should serialize");
        assert!(json.get("presetCount").is_some());
    }

    #[test]
    fn test_audio_snapshot_serialize() {
        let snap = app_lib::history::AudioScanSnapshot {
            id: String::new(),
            timestamp: String::new(),
            sample_count: 100,
            total_bytes: 0,
            format_counts: HashMap::new(),
            samples: Vec::new(),
            roots: Vec::new(),
        };

        let json = serde_json::to_value(&snap).expect("should serialize");
        assert!(json.get("sampleCount").is_some());
    }

    #[test]
    fn test_preset_snapshot_zero_values() {
        let snap = app_lib::history::PresetScanSnapshot {
            id: String::new(),
            timestamp: String::new(),
            preset_count: 0,
            total_bytes: 0,
            format_counts: HashMap::new(),
            presets: Vec::new(),
            roots: Vec::new(),
        };

        let json = serde_json::to_value(&snap).expect("should serialize");
        assert_eq!(json.get("presetCount").and_then(|v| v.as_i64()), Some(0));
    }

    #[test]
    fn test_audio_snapshot_zero_values() {
        let snap = app_lib::history::AudioScanSnapshot {
            id: String::new(),
            timestamp: String::new(),
            sample_count: 0,
            total_bytes: 0,
            format_counts: HashMap::new(),
            samples: Vec::new(),
            roots: Vec::new(),
        };

        let json = serde_json::to_value(&snap).expect("should serialize");
        assert_eq!(json.get("sampleCount").and_then(|v| v.as_i64()), Some(0));
    }
}

#[cfg(test)]
mod plugin_info_tests {
    #[test]
    fn test_plugin_info_construction() {
        let plugin = app_lib::scanner::PluginInfo {
            name: "Test Plugin".to_string(),
            manufacturer: "Test Manufacturer".to_string(),
            version: "1.0.0".to_string(),
            plugin_type: "VST3".to_string(),
            path: "/test/plugin".to_string(),
            size: "1.0 MB".to_string(),
            size_bytes: 1048576,
            modified: "2024-01-01".to_string(),
            architectures: vec!["x64".to_string()],
            manufacturer_url: None,
        };

        assert_eq!(plugin.name, "Test Plugin");
        assert_eq!(plugin.plugin_type, "VST3");
        assert!(plugin.architectures.contains(&"x64".to_string()));
    }

    #[test]
    fn test_plugin_info_roundtrip() {
        let original = app_lib::scanner::PluginInfo {
            name: "Original".to_string(),
            manufacturer: "Mfr".to_string(),
            version: "1.2.0".to_string(),
            plugin_type: "AU".to_string(),
            path: "/audio/plugin".to_string(),
            size: "512.0 KB".to_string(),
            size_bytes: 524288,
            modified: "2024-06-15".to_string(),
            architectures: vec!["arm64".to_string()],
            manufacturer_url: None,
        };

        let json = serde_json::to_string(&original).expect("should serialize");
        let deserialized: app_lib::scanner::PluginInfo =
            serde_json::from_str(&json).expect("should deserialize");

        assert_eq!(original.name, deserialized.name);
        assert_eq!(original.version, deserialized.version);
        assert_eq!(original.plugin_type, deserialized.plugin_type);
        assert_eq!(original.size_bytes, deserialized.size_bytes);
    }

    #[test]
    fn test_plugin_info_empty_fields() {
        let plugin = app_lib::scanner::PluginInfo {
            name: "".to_string(),
            manufacturer: "".to_string(),
            version: "".to_string(),
            plugin_type: "".to_string(),
            path: "".to_string(),
            size: "".to_string(),
            size_bytes: 0,
            modified: "".to_string(),
            architectures: Vec::new(),
            manufacturer_url: None,
        };

        // Should not panic on empty strings
        assert_eq!(plugin.name, "");
        assert!(plugin.architectures.is_empty());
    }

    #[test]
    fn test_plugin_info_serialization_stability() {
        let plugin1 = app_lib::scanner::PluginInfo {
            name: "Stable".to_string(),
            manufacturer: "Stable Mfr".to_string(),
            version: "1.0.0".to_string(),
            plugin_type: "VST3".to_string(),
            path: "/stable".to_string(),
            size: "1.0".to_string(),
            size_bytes: 1024,
            modified: "2024-01-01".to_string(),
            architectures: vec!["x64".to_string()],
            manufacturer_url: None,
        };

        let json1 = serde_json::to_string(&plugin1).expect("should serialize");
        let plugin2: app_lib::scanner::PluginInfo =
            serde_json::from_str(&json1).expect("should deserialize");

        assert_eq!(plugin1.name, plugin2.name);
        assert_eq!(plugin1.manufacturer, plugin2.manufacturer);
        assert_eq!(plugin1.version, plugin2.version);
    }
}

#[cfg(test)]
mod format_size_stability_tests {
    #[test]
    fn test_format_size_consistency() {
        // Run multiple times and ensure consistent results
        let test_values = [
            0u64,
            1,
            1024,
            1024 * 1024,
            1024 * 1024 * 1024,
            1024 * 1024 * 1024 * 1024,
        ];
        let results: Vec<String> = test_values
            .iter()
            .map(|&v| app_lib::format_size(v))
            .collect();

        // Run again
        let results2: Vec<String> = test_values
            .iter()
            .map(|&v| app_lib::format_size(v))
            .collect();

        assert_eq!(results, results2);
    }

    #[test]
    fn test_format_size_deterministic() {
        let want = app_lib::format_size(123456789);
        for _ in 0..10 {
            assert_eq!(app_lib::format_size(123456789), want);
        }
        assert_eq!(want, "117.7 MB");
    }
}
