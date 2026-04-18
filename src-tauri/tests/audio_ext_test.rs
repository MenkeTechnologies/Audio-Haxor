use app_lib::audio_extensions::{is_audio_extension_lowercase, audio_format_tags_for_app_info, AUDIO_EXTENSIONS};

#[test]
fn test_is_audio_extension_lowercase() {
    assert!(is_audio_extension_lowercase("wav"));
    assert!(is_audio_extension_lowercase("mp3"));
    assert!(is_audio_extension_lowercase("flac"));
    
    assert!(!is_audio_extension_lowercase("txt"));
    assert!(!is_audio_extension_lowercase(".wav")); // Should not have a dot
    assert!(!is_audio_extension_lowercase("WAV")); // Should be lowercase
}

#[test]
fn test_audio_format_tags_for_app_info() {
    let tags = audio_format_tags_for_app_info();
    assert_eq!(tags.len(), AUDIO_EXTENSIONS.len());
    assert!(tags.contains(&"WAV".to_string()));
    assert!(tags.contains(&"MP3".to_string()));
}
