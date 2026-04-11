//! Similarity: `compute_fingerprint` and `find_similar`.

#[test]
fn test_compute_fingerprint_short_invalid_wav_returns_none_or_some() {
    let temp = std::env::temp_dir().join("audio_haxor_sim_int.wav");
    std::fs::write(&temp, vec![0u8; 44]).unwrap();
    let r = app_lib::similarity::compute_fingerprint(&temp.to_string_lossy());
    if let Some(fp) = r {
        assert!(!fp.path.is_empty());
        assert!(fp.rms >= 0.0);
    }
    let _ = std::fs::remove_file(&temp);
}

#[test]
fn test_find_similar_empty_candidates() {
    use app_lib::similarity::{AudioFingerprint, find_similar};
    let reference = AudioFingerprint {
        path: "/ref.wav".into(),
        rms: 0.1,
        spectral_centroid: 1000.0,
        zero_crossing_rate: 0.05,
        low_band_energy: 0.1,
        mid_band_energy: 0.2,
        high_band_energy: 0.05,
        low_energy_ratio: 0.4,
        attack_time: 0.01,
    };
    let results = find_similar(&reference, &[], 10);
    assert!(results.is_empty());
}
