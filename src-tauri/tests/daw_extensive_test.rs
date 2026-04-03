use std::path::Path;

#[test]
fn test_daw_ext_matches() {
    assert_eq!(
        app_lib::daw_scanner::ext_matches(Path::new("project.flp")).as_deref(),
        Some("FLP")
    );
}

#[test]
fn test_daw_name_for_format() {
    assert!(!app_lib::daw_scanner::daw_name_for_format("flp").is_empty());
}
