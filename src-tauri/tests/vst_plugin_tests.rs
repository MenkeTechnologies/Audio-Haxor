//! Smoke tests for VST plugin directory discovery

#[test]
fn test_vst_directories_returns_vec() {
    use app_lib::scanner::get_vst_directories;
    let dirs = get_vst_directories();
    // Paths are filtered to existing directories only; may be empty on minimal systems.
    for d in &dirs {
        assert!(!d.is_empty());
    }
}
