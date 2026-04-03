//! Batch-style use of analysis helpers (same building blocks as the UI batch flow).

#[test]
fn test_batch_estimate_bpm_short_wavs() {
    let temp = std::env::temp_dir().join("audio_haxor_batch_bpm");
    let _ = std::fs::create_dir_all(&temp);
    for i in 0..4 {
        let p = temp.join(format!("track_{i}.wav"));
        std::fs::write(&p, vec![0u8; 44]).unwrap();
        let r = app_lib::bpm::estimate_bpm(&p.to_string_lossy());
        if let Some(bpm) = r {
            assert!(bpm > 0.0 && bpm < 1000.0, "bpm={bpm}");
        }
    }
    let _ = std::fs::remove_dir_all(&temp);
}

#[test]
fn test_batch_fingerprint_missing_files_all_none() {
    for i in 0..8 {
        let p = format!("/nonexistent/audio_haxor/missing_{i}.wav");
        assert!(
            app_lib::similarity::compute_fingerprint(&p).is_none(),
            "path {p}"
        );
    }
}

#[test]
fn test_batch_key_detect_unsupported_extensions_none() {
    for ext in ["txt", "pdf", "rs"] {
        let p = std::env::temp_dir()
            .join(format!("audio_haxor_batch_key.{ext}"));
        std::fs::write(&p, b"x").unwrap();
        assert!(
            app_lib::key_detect::detect_key(&p.to_string_lossy()).is_none(),
            "ext {ext}"
        );
        let _ = std::fs::remove_file(&p);
    }
}
