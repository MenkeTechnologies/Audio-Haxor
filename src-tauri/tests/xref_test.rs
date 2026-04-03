#[test]
fn test_xref_extract_plugins_nonexistent_returns_empty() {
    let plugins = app_lib::xref::extract_plugins("/nonexistent/audio_haxor/project.flp");
    assert!(
        plugins.is_empty(),
        "nonexistent project should yield no plugins, got {}",
        plugins.len()
    );
}

#[test]
fn test_xref_plugin_ref_struct() {
    let ref_info = app_lib::xref::PluginRef {
        name: "Test Plugin".to_string(),
        normalized_name: "test plugin".to_string(),
        manufacturer: "Co".to_string(),
        plugin_type: "VST3".to_string(),
    };
    assert_eq!(ref_info.plugin_type, "VST3");
    assert!(!ref_info.normalized_name.is_empty());
}
