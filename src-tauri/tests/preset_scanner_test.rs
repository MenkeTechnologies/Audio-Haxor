//! Preset roots and `PresetFile` shape.

#[test]
fn test_preset_roots_nonempty_path_components() {
    let roots = app_lib::preset_scanner::get_preset_roots();
    for p in roots {
        assert!(!p.as_os_str().is_empty());
    }
}

#[test]
fn test_preset_file_serde_roundtrip() {
    use app_lib::history::PresetFile;
    let preset = PresetFile {
        name: "Factory Preset".into(),
        path: "/tmp/factory.fxp".into(),
        directory: "/tmp".into(),
        format: "FXP".into(),
        size: 1024,
        size_formatted: "1.0 KB".into(),
        modified: "2024-01-01".into(),
    };
    let json = serde_json::to_string(&preset).unwrap();
    let back: PresetFile = serde_json::from_str(&json).unwrap();
    assert_eq!(back.name, preset.name);
    assert!(back.size_formatted.contains("KB"));
}
