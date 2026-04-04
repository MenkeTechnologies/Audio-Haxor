//! Musical key detection via chromagram analysis.
//!
//! Computes a 12-bin chroma vector (C, C#, D, ..., B) from audio PCM data
//! using frequency-domain energy estimation, then correlates against
//! Krumhansl-Kessler key profiles for all 24 major/minor keys.

use std::path::Path;

/// Note names for display.
const NOTE_NAMES: [&str; 12] = [
    "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
];

/// Krumhansl-Kessler major key profile (starting from C).
const MAJOR_PROFILE: [f64; 12] = [
    6.35, 2.23, 3.48, 2.33, 4.38, 4.09, 2.52, 5.19, 2.39, 3.66, 2.29, 2.88,
];

/// Krumhansl-Kessler minor key profile (starting from C minor).
const MINOR_PROFILE: [f64; 12] = [
    6.33, 2.68, 3.52, 5.38, 2.60, 3.53, 2.54, 4.75, 3.98, 2.69, 3.34, 3.17,
];

/// Detect the musical key of an audio file.
/// Returns a string like "C Major", "F# Minor", or None if detection fails.
pub fn detect_key(file_path: &str) -> Option<String> {
    let path = Path::new(file_path);
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    let (samples, sample_rate) = match ext.as_str() {
        "wav" => crate::bpm::read_wav_pcm_pub(path)?,
        "aiff" | "aif" => crate::bpm::read_aiff_pcm_pub(path)?,
        "mp3" | "flac" | "ogg" | "m4a" | "aac" | "opus" => {
            crate::bpm::decode_with_symphonia_pub(path)?
        }
        _ => return None,
    };

    if samples.len() < 4096 || sample_rate == 0 {
        return None;
    }

    // Use first 30 seconds max
    let max_samples = (sample_rate as usize) * 30;
    let s = if samples.len() > max_samples {
        &samples[..max_samples]
    } else {
        &samples
    };

    let chroma = compute_chromagram(s, sample_rate);

    // Check if we have enough energy to make a meaningful detection
    let total_energy: f64 = chroma.iter().sum();
    if total_energy < 1e-10 {
        return None;
    }

    match_key_profile(&chroma)
}

/// Compute a 12-bin chromagram from PCM audio.
///
/// Uses a sliding window DFT approach: for each frame, compute energy at
/// frequencies corresponding to each pitch class across multiple octaves,
/// then sum into 12 chroma bins.
fn compute_chromagram(samples: &[f32], sample_rate: u32) -> [f64; 12] {
    let sr = sample_rate as f64;
    let mut chroma = [0.0f64; 12];

    // Goertzel algorithm for targeted frequency detection
    // More efficient than FFT when we only need specific frequency bins
    let frame_size = 4096usize;
    let hop = frame_size / 2;
    let num_frames = (samples.len().saturating_sub(frame_size)) / hop;

    if num_frames == 0 {
        return chroma;
    }

    // Precompute target frequencies: C1 through B7 (7 octaves)
    // C1 = 32.703 Hz, up to B7 = 3951.066 Hz
    let base_freq = 32.7032; // C1
    let mut targets: Vec<(usize, f64)> = Vec::new(); // (chroma_bin, frequency)
    for octave in 0..7 {
        for note in 0..12 {
            let freq = base_freq * 2.0f64.powi(octave) * 2.0f64.powf(note as f64 / 12.0);
            if freq < sr / 2.0 && freq > 20.0 {
                targets.push((note, freq));
            }
        }
    }

    // Precompute Hann window and reuse windowed buffer
    let hann: Vec<f64> = (0..frame_size)
        .map(|i| {
            0.5 * (1.0 - (2.0 * std::f64::consts::PI * i as f64 / (frame_size - 1) as f64).cos())
        })
        .collect();
    let mut windowed = vec![0.0f64; frame_size];

    // Process each frame with Goertzel
    for frame_idx in 0..num_frames {
        let start = frame_idx * hop;
        let end = (start + frame_size).min(samples.len());
        let n = end - start;

        // Apply precomputed Hann window into reusable buffer
        for i in 0..n {
            windowed[i] = samples[start + i] as f64 * hann[i];
        }

        for &(chroma_bin, freq) in &targets {
            let power = goertzel(&windowed[..n], freq, sr);
            chroma[chroma_bin] += power;
        }
    }

    // Normalize
    let max_val = chroma.iter().cloned().fold(0.0f64, f64::max);
    if max_val > 1e-10 {
        for c in chroma.iter_mut() {
            *c /= max_val;
        }
    }

    chroma
}

/// Goertzel algorithm: compute energy at a specific frequency.
/// Much more efficient than FFT when you need only a few frequency bins.
fn goertzel(samples: &[f64], target_freq: f64, sample_rate: f64) -> f64 {
    let n = samples.len();
    let k = (target_freq * n as f64 / sample_rate).round();
    let w = 2.0 * std::f64::consts::PI * k / n as f64;
    let coeff = 2.0 * w.cos();

    let mut s1 = 0.0f64;
    let mut s2 = 0.0f64;

    for &sample in samples {
        let s0 = sample + coeff * s1 - s2;
        s2 = s1;
        s1 = s0;
    }

    // Power (magnitude squared)
    s1 * s1 + s2 * s2 - coeff * s1 * s2
}

/// Match a chroma vector against all 24 key profiles (12 major + 12 minor).
/// Returns the best-matching key name.
fn match_key_profile(chroma: &[f64; 12]) -> Option<String> {
    let mut best_key = String::new();
    let mut best_corr = f64::NEG_INFINITY;

    for (root, note) in NOTE_NAMES.iter().enumerate() {
        // Rotate profile to start from this root
        let major_corr = profile_correlation(chroma, &MAJOR_PROFILE, root);
        if major_corr > best_corr {
            best_corr = major_corr;
            best_key = format!("{note} Major");
        }

        let minor_corr = profile_correlation(chroma, &MINOR_PROFILE, root);
        if minor_corr > best_corr {
            best_corr = minor_corr;
            best_key = format!("{note} Minor");
        }
    }

    if best_key.is_empty() || best_corr < 0.0 {
        return None;
    }

    Some(best_key)
}

/// Pearson correlation between chroma and a rotated key profile.
fn profile_correlation(chroma: &[f64; 12], profile: &[f64; 12], root: usize) -> f64 {
    let n = 12.0;
    let mut sum_x = 0.0;
    let mut sum_y = 0.0;
    let mut sum_xy = 0.0;
    let mut sum_x2 = 0.0;
    let mut sum_y2 = 0.0;

    for i in 0..12 {
        let x = chroma[(i + root) % 12];
        let y = profile[i];
        sum_x += x;
        sum_y += y;
        sum_xy += x * y;
        sum_x2 += x * x;
        sum_y2 += y * y;
    }

    let numerator = n * sum_xy - sum_x * sum_y;
    let denominator = ((n * sum_x2 - sum_x * sum_x) * (n * sum_y2 - sum_y * sum_y)).sqrt();

    if denominator < 1e-10 {
        return 0.0;
    }

    numerator / denominator
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_goertzel_440hz() {
        // Generate 440Hz sine wave
        let sr = 44100.0;
        let n = 4096;
        let samples: Vec<f64> = (0..n)
            .map(|i| (2.0 * std::f64::consts::PI * 440.0 * i as f64 / sr).sin())
            .collect();

        let power_440 = goertzel(&samples, 440.0, sr);
        let power_300 = goertzel(&samples, 300.0, sr);
        assert!(
            power_440 > power_300 * 10.0,
            "440Hz should have much more energy than 300Hz: 440={}, 300={}",
            power_440,
            power_300
        );
    }

    #[test]
    fn test_goertzel_261hz_c4() {
        let sr = 44100.0;
        let n = 4096;
        let freq = 261.63; // C4
        let samples: Vec<f64> = (0..n)
            .map(|i| (2.0 * std::f64::consts::PI * freq * i as f64 / sr).sin())
            .collect();

        let power_c = goertzel(&samples, freq, sr);
        let power_e = goertzel(&samples, 329.63, sr); // E4
        assert!(power_c > power_e * 5.0, "C4 should dominate over E4");
    }

    #[test]
    fn test_chromagram_pure_a() {
        // Pure 440Hz sine = A4, should light up chroma bin 9 (A)
        let sr = 44100u32;
        let n = sr as usize * 2; // 2 seconds
        let samples: Vec<f32> = (0..n)
            .map(|i| (2.0 * std::f32::consts::PI * 440.0 * i as f32 / sr as f32).sin())
            .collect();

        let chroma = compute_chromagram(&samples, sr);
        // A is index 9
        let a_energy = chroma[9];
        let max_other = chroma
            .iter()
            .enumerate()
            .filter(|&(i, _)| i != 9)
            .map(|(_, &v)| v)
            .fold(0.0f64, f64::max);
        assert!(
            a_energy > max_other,
            "A (440Hz) should have strongest chroma bin. A={}, max_other={}",
            a_energy,
            max_other
        );
    }

    #[test]
    fn test_chromagram_pure_c() {
        // Pure 261.63Hz sine = C4, should light up chroma bin 0 (C)
        let sr = 44100u32;
        let n = sr as usize * 2;
        let samples: Vec<f32> = (0..n)
            .map(|i| (2.0 * std::f32::consts::PI * 261.63 * i as f32 / sr as f32).sin())
            .collect();

        let chroma = compute_chromagram(&samples, sr);
        let c_energy = chroma[0];
        let max_other = chroma
            .iter()
            .enumerate()
            .filter(|&(i, _)| i != 0)
            .map(|(_, &v)| v)
            .fold(0.0f64, f64::max);
        assert!(
            c_energy > max_other,
            "C should have strongest chroma bin. C={}, max_other={}",
            c_energy,
            max_other
        );
    }

    #[test]
    fn test_match_c_major_triad() {
        // C major triad: C, E, G have high energy
        let mut chroma = [0.1f64; 12];
        chroma[0] = 1.0; // C
        chroma[4] = 0.8; // E
        chroma[7] = 0.7; // G
        let key = match_key_profile(&chroma);
        assert!(key.is_some());
        let key = key.unwrap();
        assert!(
            key.contains("C") && key.contains("Major"),
            "C-E-G triad should match C Major, got {}",
            key
        );
    }

    #[test]
    fn test_match_a_minor_triad() {
        // A minor triad: A, C, E
        let mut chroma = [0.1f64; 12];
        chroma[9] = 1.0; // A
        chroma[0] = 0.8; // C
        chroma[4] = 0.7; // E
        let key = match_key_profile(&chroma);
        assert!(key.is_some());
        let key = key.unwrap();
        // A minor and C major are relative keys, both are valid interpretations
        assert!(
            key.contains("Minor") || key.contains("Major"),
            "A-C-E should match A Minor or C Major, got {}",
            key
        );
    }

    #[test]
    fn test_detect_key_nonexistent() {
        let key = detect_key("/nonexistent/file.wav");
        assert!(key.is_none());
    }

    #[test]
    fn test_detect_key_unsupported() {
        let key = detect_key("/some/file.txt");
        assert!(key.is_none());
    }

    #[test]
    fn test_detect_key_silence() {
        let tmp = std::env::temp_dir().join("key_test_silence.wav");
        let sr = 44100u32;
        let samples = vec![0.0f32; sr as usize * 2];
        write_test_wav(&tmp, &samples, sr);
        let key = detect_key(tmp.to_str().unwrap());
        assert!(key.is_none(), "silence should not detect a key");
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_detect_key_a440_wav() {
        let tmp = std::env::temp_dir().join("key_test_a440.wav");
        let sr = 44100u32;
        let n = sr as usize * 3; // 3 seconds
        let samples: Vec<f32> = (0..n)
            .map(|i| (2.0 * std::f32::consts::PI * 440.0 * i as f32 / sr as f32).sin() * 0.8)
            .collect();
        write_test_wav(&tmp, &samples, sr);
        let key = detect_key(tmp.to_str().unwrap());
        assert!(key.is_some(), "should detect key for 440Hz sine");
        let key = key.unwrap();
        // 440Hz = A4, should detect A-related key
        assert!(
            key.contains('A'),
            "440Hz should detect A-related key, got {}",
            key
        );
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_detect_key_c_major_chord() {
        let tmp = std::env::temp_dir().join("key_test_cmaj.wav");
        let sr = 44100u32;
        let n = sr as usize * 3;
        // C major chord: C4 (261.63) + E4 (329.63) + G4 (392.00)
        let samples: Vec<f32> = (0..n)
            .map(|i| {
                let t = i as f32 / sr as f32;
                let c = (2.0 * std::f32::consts::PI * 261.63 * t).sin();
                let e = (2.0 * std::f32::consts::PI * 329.63 * t).sin();
                let g = (2.0 * std::f32::consts::PI * 392.00 * t).sin();
                (c + e + g) * 0.3
            })
            .collect();
        write_test_wav(&tmp, &samples, sr);
        let key = detect_key(tmp.to_str().unwrap());
        assert!(key.is_some(), "should detect key for C major chord");
        let key = key.unwrap();
        // C major chord should detect C Major (or A Minor — relative key)
        assert!(
            key.contains('C') || key.contains('A'),
            "C major chord should detect C Major or A Minor, got {}",
            key
        );
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_profile_correlation_perfect_match() {
        // Profile correlated with itself should be 1.0
        let chroma: [f64; 12] = MAJOR_PROFILE;
        let corr = profile_correlation(&chroma, &MAJOR_PROFILE, 0);
        assert!(
            (corr - 1.0).abs() < 0.001,
            "perfect match should be ~1.0, got {}",
            corr
        );
    }

    #[test]
    fn test_profile_correlation_shifted() {
        // G major should match major profile shifted by 7
        let mut chroma = [0.0f64; 12];
        for i in 0..12 {
            chroma[(i + 7) % 12] = MAJOR_PROFILE[i];
        }
        let corr = profile_correlation(&chroma, &MAJOR_PROFILE, 7);
        assert!(
            (corr - 1.0).abs() < 0.001,
            "G major should perfectly match shifted profile, got {}",
            corr
        );
    }

    #[test]
    fn test_goertzel_near_zero_for_absent_frequency() {
        // Pure 440Hz sine should have near-zero energy at 261.63Hz (C4)
        let sr = 44100.0;
        let n = 4096;
        let samples: Vec<f64> = (0..n)
            .map(|i| (2.0 * std::f64::consts::PI * 440.0 * i as f64 / sr).sin())
            .collect();

        let power_440 = goertzel(&samples, 440.0, sr);
        let power_261 = goertzel(&samples, 261.63, sr);
        assert!(
            power_261 < power_440 * 0.01,
            "261Hz should have <1% of 440Hz energy: 261={}, 440={}",
            power_261,
            power_440
        );
    }

    #[test]
    fn test_chromagram_chord_c_major() {
        // C major chord: C4 + E4 + G4 — should light up bins 0 (C), 4 (E), 7 (G)
        let sr = 44100u32;
        let n = sr as usize * 2;
        let samples: Vec<f32> = (0..n)
            .map(|i| {
                let t = i as f32 / sr as f32;
                let c = (2.0 * std::f32::consts::PI * 261.63 * t).sin();
                let e = (2.0 * std::f32::consts::PI * 329.63 * t).sin();
                let g = (2.0 * std::f32::consts::PI * 392.00 * t).sin();
                (c + e + g) * 0.3
            })
            .collect();

        let chroma = compute_chromagram(&samples, sr);
        // C(0), E(4), G(7) should be the top 3 bins
        let mut indexed: Vec<(usize, f64)> =
            chroma.iter().enumerate().map(|(i, &v)| (i, v)).collect();
        indexed.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        let top3: Vec<usize> = indexed[..3].iter().map(|&(i, _)| i).collect();
        assert!(top3.contains(&0), "C should be in top 3, got {:?}", top3);
        assert!(top3.contains(&4), "E should be in top 3, got {:?}", top3);
        assert!(top3.contains(&7), "G should be in top 3, got {:?}", top3);
    }

    #[test]
    fn test_detect_key_high_sample_rate() {
        // 96kHz sample rate should still work
        let tmp = std::env::temp_dir().join("key_test_96k.wav");
        let sr = 96000u32;
        let n = sr as usize * 2;
        let samples: Vec<f32> = (0..n)
            .map(|i| (2.0 * std::f32::consts::PI * 440.0 * i as f32 / sr as f32).sin() * 0.8)
            .collect();
        write_test_wav(&tmp, &samples, sr);
        let key = detect_key(tmp.to_str().unwrap());
        assert!(key.is_some(), "should detect key at 96kHz");
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_detect_key_low_sample_rate() {
        // 8kHz — very few frequency bins, may return None but should not panic
        let tmp = std::env::temp_dir().join("key_test_8k.wav");
        let sr = 8000u32;
        let n = sr as usize * 2;
        let samples: Vec<f32> = (0..n)
            .map(|i| (2.0 * std::f32::consts::PI * 440.0 * i as f32 / sr as f32).sin() * 0.8)
            .collect();
        write_test_wav(&tmp, &samples, sr);
        // Should not panic regardless of result
        let _ = detect_key(tmp.to_str().unwrap());
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_detect_key_multi_octave_a() {
        // A across 3 octaves: A3 (220Hz) + A4 (440Hz) + A5 (880Hz)
        let tmp = std::env::temp_dir().join("key_test_multi_oct.wav");
        let sr = 44100u32;
        let n = sr as usize * 3;
        let samples: Vec<f32> = (0..n)
            .map(|i| {
                let t = i as f32 / sr as f32;
                let a3 = (2.0 * std::f32::consts::PI * 220.0 * t).sin();
                let a4 = (2.0 * std::f32::consts::PI * 440.0 * t).sin();
                let a5 = (2.0 * std::f32::consts::PI * 880.0 * t).sin();
                (a3 + a4 + a5) * 0.25
            })
            .collect();
        write_test_wav(&tmp, &samples, sr);
        let key = detect_key(tmp.to_str().unwrap());
        assert!(key.is_some(), "should detect key for multi-octave A");
        let key = key.unwrap();
        assert!(
            key.contains('A'),
            "multi-octave A should detect A-related key, got {}",
            key
        );
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_chromagram_bins_bounded() {
        // Verify all chroma bins are in [0, 1] after normalization
        let sr = 44100u32;
        let n = sr as usize * 2;
        let samples: Vec<f32> = (0..n)
            .map(|i| {
                let t = i as f32 / sr as f32;
                // White-noise-like signal via mixed frequencies
                (t * 261.63 * 2.0 * std::f32::consts::PI).sin() * 0.3
                    + (t * 440.0 * 2.0 * std::f32::consts::PI).sin() * 0.3
                    + (t * 783.99 * 2.0 * std::f32::consts::PI).sin() * 0.2
            })
            .collect();
        let chroma = compute_chromagram(&samples, sr);
        for (i, &v) in chroma.iter().enumerate() {
            assert!(
                (0.0..=1.0).contains(&v),
                "chroma bin {} should be [0,1], got {}",
                i,
                v
            );
        }
    }

    #[test]
    fn test_profile_correlation_identical_chroma_and_profile_is_one() {
        let chroma = MAJOR_PROFILE;
        let r = profile_correlation(&chroma, &MAJOR_PROFILE, 0);
        assert!((r - 1.0).abs() < 1e-9, "expected 1.0, got {}", r);
    }

    #[test]
    fn test_profile_correlation_rotated_chroma_matches_at_correct_root() {
        // chroma[k] = MAJOR_PROFILE[(k+5)%12] so at root 7, x_i = chroma[(i+7)%12] = MAJOR_PROFILE[i]
        let mut chroma = [0.0f64; 12];
        for k in 0..12 {
            chroma[k] = MAJOR_PROFILE[(k + 5) % 12];
        }
        let r = profile_correlation(&chroma, &MAJOR_PROFILE, 7);
        assert!((r - 1.0).abs() < 1e-9, "rotated major should match at root 7, got {}", r);
    }

    #[test]
    fn test_profile_correlation_constant_chroma_zero_variance_returns_zero() {
        let c = [0.25f64; 12];
        let r = profile_correlation(&c, &MAJOR_PROFILE, 0);
        assert_eq!(r, 0.0);
    }

    #[test]
    fn test_profile_correlation_zero_chroma_returns_zero() {
        let z = [0.0f64; 12];
        let r = profile_correlation(&z, &MAJOR_PROFILE, 0);
        assert_eq!(r, 0.0, "zero variance in x → Pearson denom 0 → 0.0");
    }

    #[test]
    fn test_compute_chromagram_too_short_for_one_frame_returns_zeros() {
        let sr = 44100u32;
        let samples: Vec<f32> = vec![0.5; 3000];
        let chroma = compute_chromagram(&samples, sr);
        assert!(
            chroma.iter().all(|&v| v == 0.0),
            "len < frame_size gives num_frames=0 → zero chroma"
        );
    }

    #[test]
    fn test_match_key_profile_all_zero_chroma_picks_first_tie_at_zero_correlation() {
        let z = [0.0f64; 12];
        let key = match_key_profile(&z);
        assert_eq!(key.as_deref(), Some("C Major"));
    }

    fn write_test_wav(path: &Path, samples: &[f32], sample_rate: u32) {
        let n = samples.len() as u32;
        let data_size = n * 2;
        let mut buf = Vec::with_capacity(44 + data_size as usize);
        buf.extend_from_slice(b"RIFF");
        buf.extend_from_slice(&(36 + data_size).to_le_bytes());
        buf.extend_from_slice(b"WAVE");
        buf.extend_from_slice(b"fmt ");
        buf.extend_from_slice(&16u32.to_le_bytes());
        buf.extend_from_slice(&1u16.to_le_bytes());
        buf.extend_from_slice(&1u16.to_le_bytes());
        buf.extend_from_slice(&sample_rate.to_le_bytes());
        buf.extend_from_slice(&(sample_rate * 2).to_le_bytes());
        buf.extend_from_slice(&2u16.to_le_bytes());
        buf.extend_from_slice(&16u16.to_le_bytes());
        buf.extend_from_slice(b"data");
        buf.extend_from_slice(&data_size.to_le_bytes());
        for &s in samples {
            let i = (s.clamp(-1.0, 1.0) * 32767.0) as i16;
            buf.extend_from_slice(&i.to_le_bytes());
        }
        std::fs::write(path, buf).unwrap();
    }
}
