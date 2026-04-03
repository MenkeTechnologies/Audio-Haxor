#[test]
fn test_scanner_get_plugin_type() {
    assert_eq!(app_lib::scanner::get_plugin_type(".vst3"), "VST3");
}

#[test]
fn test_scanner_format_size() {
    assert!(!app_lib::scanner::format_size(2048).is_empty());
}
