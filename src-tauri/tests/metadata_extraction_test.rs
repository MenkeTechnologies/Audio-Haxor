//! Metadata extraction: `AudioMetadata` from `get_audio_metadata`.

#[test]
fn test_get_audio_metadata_nonexistent() {
    let m = app_lib::audio_scanner::get_audio_metadata("/nonexistent/audio_haxor/missing.wav");
    assert!(m.error.is_some());
    assert_eq!(m.size_bytes, 0);
}

#[test]
fn test_get_audio_metadata_temp_wav_header_only() {
    let temp = std::env::temp_dir().join("audio_haxor_meta_short.wav");
    // Minimal 44-byte WAV header shell (may not decode duration)
    std::fs::write(&temp, &vec![0u8; 44]).unwrap();
    let m = app_lib::audio_scanner::get_audio_metadata(&temp.to_string_lossy());
    assert!(m.full_path.contains("audio_haxor_meta_short"));
    assert!(m.file_name.contains("audio_haxor_meta_short"));
    assert_eq!(m.format, "WAV");
    let _ = std::fs::remove_file(&temp);
}

#[test]
fn test_get_audio_metadata_directory_path() {
    let temp = std::env::temp_dir().join("audio_haxor_meta_dir");
    std::fs::create_dir_all(&temp).unwrap();
    let m = app_lib::audio_scanner::get_audio_metadata(&temp.to_string_lossy());
    assert!(m.error.is_none());
    assert!(m.format.is_empty());
    assert!(m.full_path.contains("audio_haxor_meta_dir"));
    let _ = std::fs::remove_dir_all(&temp);
}
