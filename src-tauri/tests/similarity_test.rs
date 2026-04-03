#[test]
fn test_similarity_compute_fingerprint_nonexistent() {
    assert!(
        app_lib::similarity::compute_fingerprint("/nonexistent/audio_haxor_fp.wav").is_none()
    );
}

#[test]
fn test_similarity_find_similar_empty_candidates() {
    let fp = app_lib::similarity::AudioFingerprint {
        path: "/a.wav".to_string(),
        rms: 0.0,
        spectral_centroid: 0.0,
        zero_crossing_rate: 0.0,
        low_band_energy: 0.0,
        mid_band_energy: 0.0,
        high_band_energy: 0.0,
        low_energy_ratio: 0.0,
        attack_time: 0.0,
    };
    let results = app_lib::similarity::find_similar(&fp, &[], 10);
    assert!(results.is_empty());
}

#[test]
fn test_similarity_audio_fingerprint_struct_fields() {
    let fp = app_lib::similarity::AudioFingerprint {
        path: "/test.wav".to_string(),
        rms: 0.1,
        spectral_centroid: 0.5,
        zero_crossing_rate: 0.0,
        low_band_energy: 0.2,
        mid_band_energy: 0.3,
        high_band_energy: 0.5,
        low_energy_ratio: 0.4,
        attack_time: 0.01,
    };
    assert_eq!(fp.path, "/test.wav");
    assert!(fp.rms >= 0.0 && fp.spectral_centroid <= 1.0);
}
