#[test]
fn test_preset_get_roots_paths_nonempty_strings() {
    let roots = app_lib::preset_scanner::get_preset_roots();
    for p in &roots {
        assert!(
            !p.as_os_str().is_empty(),
            "preset root path should not be empty"
        );
    }
}
