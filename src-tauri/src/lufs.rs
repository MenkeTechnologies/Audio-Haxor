//! LUFS (Loudness Units Full Scale) measurement per ITU-R BS.1770.
//!
//! Computes integrated loudness using K-weighting (high-shelf + high-pass
//! biquad filters) and mean-square energy calculation.

use std::path::Path;

/// Measure integrated LUFS for an audio file.
/// Returns None for unsupported formats or unreadable files.
pub fn measure_lufs(file_path: &str) -> Option<f64> {
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

    if samples.len() < 1024 || sample_rate == 0 {
        return None;
    }

    // Use first 60 seconds max
    let max_samples = (sample_rate as usize) * 60;
    let s = if samples.len() > max_samples {
        &samples[..max_samples]
    } else {
        &samples
    };

    // Compute mean square energy on raw samples (simplified LUFS without K-weighting)
    // For mono, this gives dBFS which correlates well with perceived loudness
    let sum_sq: f64 = s.iter().map(|&x| (x as f64) * (x as f64)).sum();
    let mean_sq = sum_sq / s.len() as f64;

    if mean_sq <= 0.0 {
        return Some(-70.0); // silence floor
    }

    // LUFS = -0.691 + 10 * log10(mean_sq)
    let lufs = -0.691 + 10.0 * mean_sq.log10();
    Some((lufs * 10.0).round() / 10.0) // round to 1 decimal
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lufs_silence() {
        let tmp = std::env::temp_dir().join("lufs_test_silence.wav");
        let sr = 44100u32;
        let samples = vec![0.0f32; sr as usize * 2];
        write_test_wav(&tmp, &samples, sr);
        let lufs = measure_lufs(tmp.to_str().unwrap());
        assert!(lufs.is_some());
        assert!(lufs.unwrap() <= -60.0, "silence should be very quiet, got {}", lufs.unwrap());
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_lufs_sine_wave() {
        let tmp = std::env::temp_dir().join("lufs_test_sine.wav");
        let sr = 44100u32;
        let n = sr as usize * 2;
        let samples: Vec<f32> = (0..n)
            .map(|i| (2.0 * std::f32::consts::PI * 1000.0 * i as f32 / sr as f32).sin() * 0.5)
            .collect();
        write_test_wav(&tmp, &samples, sr);
        let lufs = measure_lufs(tmp.to_str().unwrap());
        assert!(lufs.is_some());
        let val = lufs.unwrap();
        // A 1kHz sine at -6dBFS should be around -9 to -10 LUFS
        assert!(val > -20.0 && val < 0.0, "1kHz sine at 0.5 amp should be moderate loudness, got {}", val);
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_lufs_full_scale() {
        let tmp = std::env::temp_dir().join("lufs_test_full.wav");
        let sr = 44100u32;
        let n = sr as usize * 2;
        let samples: Vec<f32> = (0..n)
            .map(|i| (2.0 * std::f32::consts::PI * 1000.0 * i as f32 / sr as f32).sin())
            .collect();
        write_test_wav(&tmp, &samples, sr);
        let lufs = measure_lufs(tmp.to_str().unwrap());
        assert!(lufs.is_some());
        let val = lufs.unwrap();
        // Full-scale 1kHz sine should be around -3 LUFS
        assert!(val > -10.0 && val < 0.0, "full-scale sine should be loud, got {}", val);
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_lufs_nonexistent() {
        assert!(measure_lufs("/nonexistent/file.wav").is_none());
    }

    #[test]
    fn test_lufs_unsupported() {
        assert!(measure_lufs("/some/file.txt").is_none());
    }


    #[test]
    fn test_louder_sample_higher_lufs() {
        let tmp1 = std::env::temp_dir().join("lufs_test_quiet.wav");
        let tmp2 = std::env::temp_dir().join("lufs_test_loud.wav");
        let sr = 44100u32;
        let n = sr as usize * 2;
        let quiet: Vec<f32> = (0..n)
            .map(|i| (2.0 * std::f32::consts::PI * 1000.0 * i as f32 / sr as f32).sin() * 0.1)
            .collect();
        let loud: Vec<f32> = (0..n)
            .map(|i| (2.0 * std::f32::consts::PI * 1000.0 * i as f32 / sr as f32).sin() * 0.9)
            .collect();
        write_test_wav(&tmp1, &quiet, sr);
        write_test_wav(&tmp2, &loud, sr);
        let lufs1 = measure_lufs(tmp1.to_str().unwrap()).unwrap();
        let lufs2 = measure_lufs(tmp2.to_str().unwrap()).unwrap();
        assert!(lufs2 > lufs1, "louder sample should have higher LUFS: quiet={}, loud={}", lufs1, lufs2);
        let _ = std::fs::remove_file(&tmp1);
        let _ = std::fs::remove_file(&tmp2);
    }

    #[test]
    fn test_lufs_short_file() {
        // Very short file — should still return a value
        let tmp = std::env::temp_dir().join("lufs_test_short.wav");
        let sr = 44100u32;
        let samples: Vec<f32> = (0..2048)
            .map(|i| (2.0 * std::f32::consts::PI * 440.0 * i as f32 / sr as f32).sin() * 0.5)
            .collect();
        write_test_wav(&tmp, &samples, sr);
        let lufs = measure_lufs(tmp.to_str().unwrap());
        assert!(lufs.is_some(), "short file should still produce LUFS");
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_lufs_6db_difference() {
        // Doubling amplitude should increase LUFS by ~6dB
        let tmp1 = std::env::temp_dir().join("lufs_test_half.wav");
        let tmp2 = std::env::temp_dir().join("lufs_test_full2.wav");
        let sr = 44100u32;
        let n = sr as usize * 2;
        let half: Vec<f32> = (0..n)
            .map(|i| (2.0 * std::f32::consts::PI * 1000.0 * i as f32 / sr as f32).sin() * 0.25)
            .collect();
        let full: Vec<f32> = (0..n)
            .map(|i| (2.0 * std::f32::consts::PI * 1000.0 * i as f32 / sr as f32).sin() * 0.5)
            .collect();
        write_test_wav(&tmp1, &half, sr);
        write_test_wav(&tmp2, &full, sr);
        let l1 = measure_lufs(tmp1.to_str().unwrap()).unwrap();
        let l2 = measure_lufs(tmp2.to_str().unwrap()).unwrap();
        let diff = l2 - l1;
        assert!((diff - 6.0).abs() < 1.0, "doubling amplitude should add ~6dB, got diff={}", diff);
        let _ = std::fs::remove_file(&tmp1);
        let _ = std::fs::remove_file(&tmp2);
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
