#[test]
fn test_audio_metadata_smoke() {
    let meta = app_lib::audio_scanner::get_audio_metadata("/nonexistent/sample.wav");
    assert!(!meta.full_path.is_empty());
}

#[test]
fn test_audio_format_size() {
    assert!(!app_lib::audio_scanner::format_size(1024).is_empty());
}
