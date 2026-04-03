#[test]
fn test_bpm_short_wav_returns_option_in_range_or_none() {
    let temp = std::env::temp_dir()
        .join("audio_haxor_bpm")
        .join("test.wav");
    let _ = std::fs::create_dir_all(temp.parent().unwrap());
    std::fs::write(&temp, vec![0u8; 44]).unwrap();

    let result = app_lib::bpm::estimate_bpm(&temp.to_string_lossy());
    if let Some(bpm) = result {
        assert!(
            bpm > 0.0 && bpm < 1000.0,
            "BPM in plausible range, got {bpm}"
        );
    }
    let _ = std::fs::remove_dir_all(temp.parent().unwrap());
}

#[test]
fn test_bpm_nonexistent_and_non_audio_no_panic() {
    assert!(app_lib::bpm::estimate_bpm("/nonexistent/audio_haxor/missing.wav").is_none());
    let p = std::env::temp_dir().join("audio_haxor_bpm_note.txt");
    std::fs::write(&p, b"x").unwrap();
    assert!(app_lib::bpm::estimate_bpm(&p.to_string_lossy()).is_none());
    let _ = std::fs::remove_file(&p);
}
