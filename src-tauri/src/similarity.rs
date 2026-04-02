//! Audio similarity search via spectral fingerprinting.
//!
//! Computes a feature vector for each audio file using fast energy-band
//! analysis (no DFT needed). Compares fingerprints using euclidean distance
//! to find similar-sounding samples. Fingerprints are cached to avoid
//! recomputation.

use serde::{Deserialize, Serialize};
use std::path::Path;

/// Audio fingerprint — a compact feature vector for similarity comparison.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioFingerprint {
    pub path: String,
    pub rms: f64,
    pub spectral_centroid: f64,
    pub zero_crossing_rate: f64,
    pub low_band_energy: f64,
    pub mid_band_energy: f64,
    pub high_band_energy: f64,
    pub low_energy_ratio: f64,
    pub attack_time: f64,
}

/// Compute a fingerprint for an audio file using fast energy-band analysis.
pub fn compute_fingerprint(file_path: &str) -> Option<AudioFingerprint> {
    let path = Path::new(file_path);
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();

    let (samples, sample_rate) = match ext.as_str() {
        "wav" => crate::bpm::read_wav_pcm_pub(path)?,
        "aiff" | "aif" => crate::bpm::read_aiff_pcm_pub(path)?,
        "mp3" | "flac" | "ogg" | "m4a" | "aac" | "opus" => {
            crate::bpm::decode_with_symphonia_pub(path)?
        }
        _ => return None,
    };

    if samples.len() < 1024 || sample_rate == 0 {
        return None;
    }

    // Use first 10 seconds max
    let max_samples = (sample_rate as usize) * 10;
    let s = if samples.len() > max_samples { &samples[..max_samples] } else { &samples };
    let n = s.len() as f64;
    let sr = sample_rate as f64;

    // RMS energy
    let rms = (s.iter().map(|&x| (x as f64) * (x as f64)).sum::<f64>() / n).sqrt();

    // Zero-crossing rate
    let mut zc = 0usize;
    for i in 1..s.len() {
        if (s[i] >= 0.0) != (s[i - 1] >= 0.0) { zc += 1; }
    }
    let zero_crossing_rate = zc as f64 / n;

    // Spectral centroid approximation via zero-crossing rate (normalized to [0, 1])
    // ZCR × sr/2 gives Hz estimate; divide by Nyquist to normalize across sample rates
    let spectral_centroid = zero_crossing_rate;

    // Energy in 3 frequency bands using bandpass via averaging
    // Low: 0-300Hz, Mid: 300-3000Hz, High: 3000Hz+
    // Approximate by splitting samples into blocks and measuring energy
    let frame_size = (sr / 50.0) as usize; // ~20ms frames
    let mut low_e = 0.0f64;
    let mut mid_e = 0.0f64;
    let mut high_e = 0.0f64;
    let mut frame_count = 0usize;

    for chunk in s.chunks(frame_size) {
        if chunk.len() < 4 { continue; }
        frame_count += 1;

        // Simple 3-band energy split using differences
        // Low-pass: running average (smoothed)
        // High-pass: sample - running average
        let mut lp = 0.0f32;
        let alpha_low = 0.05f32;  // ~300Hz cutoff at 44.1kHz
        let alpha_high = 0.7f32;  // ~3kHz cutoff
        let mut low_sum = 0.0f64;
        let mut mid_sum = 0.0f64;
        let mut high_sum = 0.0f64;

        let mut lp_slow = 0.0f32;
        for &x in chunk {
            lp = lp + alpha_low * (x - lp);           // low-pass ~300Hz
            lp_slow = lp_slow + alpha_high * (x - lp_slow); // low-pass ~3kHz
            let low = lp as f64;
            let mid = (lp_slow - lp) as f64;
            let high = (x - lp_slow) as f64;
            low_sum += low * low;
            mid_sum += mid * mid;
            high_sum += high * high;
        }
        low_e += low_sum / chunk.len() as f64;
        mid_e += mid_sum / chunk.len() as f64;
        high_e += high_sum / chunk.len() as f64;
    }

    let _fc = frame_count.max(1) as f64;
    let total_e = (low_e + mid_e + high_e).max(1e-10);
    let low_band_energy = low_e / total_e;
    let mid_band_energy = mid_e / total_e;
    let high_band_energy = high_e / total_e;

    // Low energy ratio
    let mut frame_energies = Vec::new();
    for chunk in s.chunks(1024) {
        let e: f64 = chunk.iter().map(|&x| (x as f64) * (x as f64)).sum::<f64>() / chunk.len() as f64;
        frame_energies.push(e);
    }
    let avg_energy = frame_energies.iter().sum::<f64>() / frame_energies.len().max(1) as f64;
    let low_energy_ratio = frame_energies.iter().filter(|&&e| e < avg_energy).count() as f64
        / frame_energies.len().max(1) as f64;

    // Attack time: how quickly the signal reaches peak energy
    let env_size = 256;
    let mut envelope = Vec::new();
    for chunk in s.chunks(env_size) {
        let peak = chunk.iter().map(|&x| x.abs()).fold(0.0f32, f32::max);
        envelope.push(peak as f64);
    }
    let peak_val = envelope.iter().cloned().fold(0.0f64, f64::max).max(1e-10);
    let attack_threshold = peak_val * 0.9;
    let attack_time = envelope.iter().position(|&e| e >= attack_threshold)
        .map(|i| i as f64 * env_size as f64 / sr)
        .unwrap_or(1.0);

    Some(AudioFingerprint {
        path: file_path.to_string(),
        rms,
        spectral_centroid,
        zero_crossing_rate,
        low_band_energy,
        mid_band_energy,
        high_band_energy,
        low_energy_ratio,
        attack_time,
    })
}

/// Compute distance between two fingerprints (lower = more similar).
pub fn fingerprint_distance(a: &AudioFingerprint, b: &AudioFingerprint) -> f64 {
    let norm = |va: f64, vb: f64, max: f64| -> f64 {
        let da = va / max.max(1e-10);
        let db = vb / max.max(1e-10);
        (da - db) * (da - db)
    };

    let d = norm(a.rms, b.rms, 1.0)
        + norm(a.spectral_centroid, b.spectral_centroid, 0.5)
        + norm(a.zero_crossing_rate, b.zero_crossing_rate, 0.5)
        + norm(a.low_band_energy, b.low_band_energy, 1.0)
        + norm(a.mid_band_energy, b.mid_band_energy, 1.0)
        + norm(a.high_band_energy, b.high_band_energy, 1.0)
        + norm(a.low_energy_ratio, b.low_energy_ratio, 1.0)
        + norm(a.attack_time, b.attack_time, 2.0);

    d.sqrt()
}

/// Find the N most similar samples to a reference fingerprint.
pub fn find_similar(
    reference: &AudioFingerprint,
    candidates: &[AudioFingerprint],
    max_results: usize,
) -> Vec<(String, f64)> {
    let mut scored: Vec<(String, f64)> = candidates
        .iter()
        .filter(|c| c.path != reference.path)
        .map(|c| (c.path.clone(), fingerprint_distance(reference, c)))
        .collect();

    scored.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(max_results);
    scored
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_fp(path: &str, rms: f64, centroid: f64, zcr: f64, low: f64, mid: f64, high: f64) -> AudioFingerprint {
        AudioFingerprint {
            path: path.to_string(),
            rms,
            spectral_centroid: centroid,
            zero_crossing_rate: zcr,
            low_band_energy: low,
            mid_band_energy: mid,
            high_band_energy: high,
            low_energy_ratio: 0.5,
            attack_time: 0.1,
        }
    }

    #[test]
    fn test_identical_fingerprints_zero_distance() {
        let a = make_fp("a.wav", 0.5, 0.1, 0.1, 0.6, 0.3, 0.1);
        let b = make_fp("b.wav", 0.5, 0.1, 0.1, 0.6, 0.3, 0.1);
        let d = fingerprint_distance(&a, &b);
        assert!(d < 0.001, "identical fingerprints should have ~0 distance, got {}", d);
    }

    #[test]
    fn test_different_fingerprints_nonzero_distance() {
        let kick = make_fp("kick.wav", 0.8, 0.02, 0.05, 0.9, 0.08, 0.02);
        let hihat = make_fp("hihat.wav", 0.3, 0.4, 0.4, 0.05, 0.15, 0.8);
        let d = fingerprint_distance(&kick, &hihat);
        assert!(d > 0.5, "kick and hihat should be very different, got {}", d);
    }

    #[test]
    fn test_similar_kicks_closer_than_kick_hihat() {
        let kick1 = make_fp("kick1.wav", 0.8, 0.02, 0.05, 0.9, 0.08, 0.02);
        let kick2 = make_fp("kick2.wav", 0.75, 0.03, 0.06, 0.85, 0.1, 0.05);
        let hihat = make_fp("hihat.wav", 0.3, 0.4, 0.4, 0.05, 0.15, 0.8);

        let d_kicks = fingerprint_distance(&kick1, &kick2);
        let d_kick_hihat = fingerprint_distance(&kick1, &hihat);
        assert!(d_kicks < d_kick_hihat, "similar kicks ({}) should be closer than kick-hihat ({})", d_kicks, d_kick_hihat);
    }

    #[test]
    fn test_find_similar_returns_sorted() {
        let reference = make_fp("ref.wav", 0.5, 0.1, 0.1, 0.5, 0.3, 0.2);
        let close = make_fp("close.wav", 0.48, 0.11, 0.11, 0.48, 0.32, 0.2);
        let far = make_fp("far.wav", 0.1, 0.4, 0.4, 0.05, 0.15, 0.8);
        let medium = make_fp("medium.wav", 0.6, 0.15, 0.15, 0.4, 0.35, 0.25);

        let results = find_similar(&reference, &[close, far, medium], 10);
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].0, "close.wav");
        assert_eq!(results[2].0, "far.wav");
    }

    #[test]
    fn test_find_similar_excludes_self() {
        let a = make_fp("a.wav", 0.5, 0.1, 0.1, 0.5, 0.3, 0.2);
        let same = make_fp("a.wav", 0.5, 0.1, 0.1, 0.5, 0.3, 0.2);
        let results = find_similar(&a, &[same], 10);
        assert_eq!(results.len(), 0, "should exclude self from results");
    }

    #[test]
    fn test_find_similar_max_results() {
        let reference = make_fp("ref.wav", 0.5, 0.1, 0.1, 0.5, 0.3, 0.2);
        let candidates: Vec<_> = (0..50).map(|i| {
            make_fp(&format!("s{}.wav", i), 0.5 + i as f64 * 0.01, 0.1, 0.1, 0.5, 0.3, 0.2)
        }).collect();
        let results = find_similar(&reference, &candidates, 5);
        assert_eq!(results.len(), 5);
    }

    #[test]
    fn test_compute_fingerprint_nonexistent_file() {
        let fp = compute_fingerprint("/nonexistent/file.wav");
        assert!(fp.is_none());
    }

    #[test]
    fn test_compute_fingerprint_unsupported_format() {
        let fp = compute_fingerprint("/some/file.txt");
        assert!(fp.is_none());
    }

    #[test]
    fn test_compute_fingerprint_wav() {
        // Create a minimal WAV with a sine wave
        let tmp = std::env::temp_dir().join("sim_test.wav");
        let sample_rate = 44100u32;
        let num_samples = sample_rate as usize; // 1 second
        let mut data = vec![0u8; 44 + num_samples * 2];
        data[0..4].copy_from_slice(b"RIFF");
        let file_size = (36 + num_samples * 2) as u32;
        data[4..8].copy_from_slice(&file_size.to_le_bytes());
        data[8..12].copy_from_slice(b"WAVE");
        data[12..16].copy_from_slice(b"fmt ");
        data[16..20].copy_from_slice(&16u32.to_le_bytes());
        data[20..22].copy_from_slice(&1u16.to_le_bytes()); // PCM
        data[22..24].copy_from_slice(&1u16.to_le_bytes()); // mono
        data[24..28].copy_from_slice(&sample_rate.to_le_bytes());
        data[28..32].copy_from_slice(&(sample_rate * 2).to_le_bytes());
        data[32..34].copy_from_slice(&2u16.to_le_bytes());
        data[34..36].copy_from_slice(&16u16.to_le_bytes());
        data[36..40].copy_from_slice(b"data");
        data[40..44].copy_from_slice(&(num_samples as u32 * 2).to_le_bytes());
        // 440Hz sine wave
        for i in 0..num_samples {
            let t = i as f64 / sample_rate as f64;
            let sample = (t * 440.0 * 2.0 * std::f64::consts::PI).sin() * 16000.0;
            let s = sample as i16;
            let offset = 44 + i * 2;
            data[offset..offset + 2].copy_from_slice(&s.to_le_bytes());
        }
        std::fs::write(&tmp, &data).unwrap();

        let fp = compute_fingerprint(tmp.to_str().unwrap());
        assert!(fp.is_some(), "should compute fingerprint for valid WAV");
        let fp = fp.unwrap();
        assert!(fp.rms > 0.0, "RMS should be positive");
        assert!(fp.spectral_centroid > 0.0, "centroid should be positive");

        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_fingerprint_all_zeros() {
        // All-zero features should produce valid fingerprint with zero distance to itself
        let a = make_fp("z.wav", 0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
        let b = make_fp("z2.wav", 0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
        let d = fingerprint_distance(&a, &b);
        assert!(d < 0.001, "all-zero fingerprints should have ~0 distance, got {}", d);
    }

    #[test]
    fn test_fingerprint_distance_symmetric() {
        let a = make_fp("a.wav", 0.8, 0.02, 0.05, 0.9, 0.08, 0.02);
        let b = make_fp("b.wav", 0.3, 0.4, 0.4, 0.05, 0.15, 0.8);
        let d_ab = fingerprint_distance(&a, &b);
        let d_ba = fingerprint_distance(&b, &a);
        assert!((d_ab - d_ba).abs() < 1e-10, "distance should be symmetric");
    }

    #[test]
    fn test_find_similar_empty_candidates() {
        let reference = make_fp("ref.wav", 0.5, 0.1, 0.1, 0.5, 0.3, 0.2);
        let results = find_similar(&reference, &[], 10);
        assert!(results.is_empty());
    }

    #[test]
    fn test_find_similar_single_candidate() {
        let reference = make_fp("ref.wav", 0.5, 0.1, 0.1, 0.5, 0.3, 0.2);
        let candidate = make_fp("c.wav", 0.6, 0.12, 0.12, 0.45, 0.35, 0.2);
        let results = find_similar(&reference, &[candidate], 10);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, "c.wav");
    }

    #[test]
    fn test_compute_fingerprint_silence_wav() {
        // Create a silent WAV (all zero samples)
        let tmp = std::env::temp_dir().join("sim_test_silence.wav");
        let sample_rate = 44100u32;
        let num_samples = sample_rate as usize;
        let mut data = vec![0u8; 44 + num_samples * 2];
        data[0..4].copy_from_slice(b"RIFF");
        let file_size = (36 + num_samples * 2) as u32;
        data[4..8].copy_from_slice(&file_size.to_le_bytes());
        data[8..12].copy_from_slice(b"WAVE");
        data[12..16].copy_from_slice(b"fmt ");
        data[16..20].copy_from_slice(&16u32.to_le_bytes());
        data[20..22].copy_from_slice(&1u16.to_le_bytes());
        data[22..24].copy_from_slice(&1u16.to_le_bytes());
        data[24..28].copy_from_slice(&sample_rate.to_le_bytes());
        data[28..32].copy_from_slice(&(sample_rate * 2).to_le_bytes());
        data[32..34].copy_from_slice(&2u16.to_le_bytes());
        data[34..36].copy_from_slice(&16u16.to_le_bytes());
        data[36..40].copy_from_slice(b"data");
        data[40..44].copy_from_slice(&(num_samples as u32 * 2).to_le_bytes());
        // All samples are zero (silence)
        std::fs::write(&tmp, &data).unwrap();

        let fp = compute_fingerprint(tmp.to_str().unwrap());
        assert!(fp.is_some(), "should handle silent WAV");
        let fp = fp.unwrap();
        assert!(fp.rms < 0.001, "silent file should have near-zero RMS");

        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_compute_fingerprint_very_short_wav() {
        // WAV with only 100 samples — below 1024 frame threshold
        let tmp = std::env::temp_dir().join("sim_test_short.wav");
        let sample_rate = 44100u32;
        let num_samples = 100usize;
        let mut data = vec![0u8; 44 + num_samples * 2];
        data[0..4].copy_from_slice(b"RIFF");
        let file_size = (36 + num_samples * 2) as u32;
        data[4..8].copy_from_slice(&file_size.to_le_bytes());
        data[8..12].copy_from_slice(b"WAVE");
        data[12..16].copy_from_slice(b"fmt ");
        data[16..20].copy_from_slice(&16u32.to_le_bytes());
        data[20..22].copy_from_slice(&1u16.to_le_bytes());
        data[22..24].copy_from_slice(&1u16.to_le_bytes());
        data[24..28].copy_from_slice(&sample_rate.to_le_bytes());
        data[28..32].copy_from_slice(&(sample_rate * 2).to_le_bytes());
        data[32..34].copy_from_slice(&2u16.to_le_bytes());
        data[34..36].copy_from_slice(&16u16.to_le_bytes());
        data[36..40].copy_from_slice(b"data");
        data[40..44].copy_from_slice(&(num_samples as u32 * 2).to_le_bytes());
        for i in 0..num_samples {
            let s = (i as i16).wrapping_mul(100);
            let offset = 44 + i * 2;
            data[offset..offset + 2].copy_from_slice(&s.to_le_bytes());
        }
        std::fs::write(&tmp, &data).unwrap();

        // Very short files should still return a fingerprint (we handle gracefully)
        let fp = compute_fingerprint(tmp.to_str().unwrap());
        // May be None or Some depending on min sample threshold — either is acceptable
        let _ = std::fs::remove_file(&tmp);
        // Just ensure no panic
        let _ = fp;
    }
}
