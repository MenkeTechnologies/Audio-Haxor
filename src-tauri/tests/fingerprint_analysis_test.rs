//! Fingerprint & similarity: align with `app_lib::similarity::AudioFingerprint`.

#[test]
fn test_audio_fingerprint_fields_construct() {
    use app_lib::similarity::AudioFingerprint;
    let fp = AudioFingerprint {
        path: "/tmp/silence.wav".to_string(),
        rms: 0.0,
        spectral_centroid: 1000.0,
        zero_crossing_rate: 0.05,
        low_band_energy: 0.1,
        mid_band_energy: 0.2,
        high_band_energy: 0.05,
        low_energy_ratio: 0.4,
        attack_time: 0.01,
    };
    assert_eq!(fp.path, "/tmp/silence.wav");
    assert!(fp.rms >= 0.0);
}

#[test]
fn test_compute_fingerprint_missing_file() {
    assert!(
        app_lib::similarity::compute_fingerprint("/nonexistent/audio_haxor/no_such.wav").is_none()
    );
}

#[test]
fn test_compute_fingerprint_non_audio_extension() {
    let temp = std::env::temp_dir().join("audio_haxor_fp_txt");
    std::fs::write(&temp, b"not audio").unwrap();
    assert!(app_lib::similarity::compute_fingerprint(&temp.to_string_lossy()).is_none());
    let _ = std::fs::remove_file(&temp);
}
