//! BPM estimation: `estimate_bpm` returns `Option<f64>`.

#[test]
fn test_bpm_detect_silent_file() {
    #[cfg(target_os = "macos")]
    {
        let temp = std::env::temp_dir().join("audio_haxor_bpm_silent.wav");
        let _ = std::fs::write(&temp, vec![0u8; 44]);

        let result = app_lib::bpm::estimate_bpm(&temp.to_string_lossy());
        if let Some(bpm) = result {
            assert!((20.0..400.0).contains(&bpm), "bpm={bpm}");
        }

        let _ = std::fs::remove_file(&temp);
    }
}

#[test]
fn test_bpm_detect_nonexistent_paths_no_panic() {
    #[cfg(target_os = "macos")]
    {
        for path in [
            "/nonexistent/audio_haxor/test.wav",
            "/nonexistent/audio_haxor/test.aiff",
            "/nonexistent/audio_haxor/test.mp3",
        ] {
            let _ = app_lib::bpm::estimate_bpm(path);
        }
    }
}
