#[test]
fn test_key_detect_nonexistent_returns_none_or_key() {
    let p = "/nonexistent/audio_haxor/key.wav";
    let result = app_lib::key_detect::detect_key(p);
    if let Some(key) = result {
        assert!(!key.is_empty(), "key string should not be empty");
    }
}

#[test]
fn test_key_detect_rejects_non_audio_extension() {
    assert!(app_lib::key_detect::detect_key("/tmp/foo.txt").is_none());
}
