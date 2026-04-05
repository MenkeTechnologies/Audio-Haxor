//! BPM estimation via onset-strength autocorrelation.
//!
//! Reads raw PCM data from WAV and AIFF files, computes an energy envelope,
//! derives onset strength, then uses autocorrelation to find the dominant
//! tempo in the 50–220 BPM range.

use std::fs;
use std::path::Path;

/// Estimate BPM for an audio file. Returns None for unsupported formats,
/// unreadable files, or when no clear tempo is detected.
pub fn estimate_bpm(file_path: &str) -> Option<f64> {
    let path = Path::new(file_path);
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    let (samples, sample_rate) = match ext.as_str() {
        "wav" => read_wav_pcm(path)?,
        "aiff" | "aif" => read_aiff_pcm(path)?,
        "mp3" | "flac" | "ogg" | "m4a" | "aac" | "opus" => decode_with_symphonia(path)?,
        _ => return None,
    };

    if samples.len() < 1024 || sample_rate == 0 {
        return None;
    }

    detect_tempo(&samples, sample_rate)
}

// Public wrappers for use by similarity module
pub fn read_wav_pcm_pub(path: &Path) -> Option<(Vec<f32>, u32)> {
    read_wav_pcm(path)
}
pub fn read_aiff_pcm_pub(path: &Path) -> Option<(Vec<f32>, u32)> {
    read_aiff_pcm(path)
}
pub fn decode_with_symphonia_pub(path: &Path) -> Option<(Vec<f32>, u32)> {
    decode_with_symphonia(path)
}

/// Read WAV file and return mono f32 samples + sample rate.
fn read_wav_pcm(path: &Path) -> Option<(Vec<f32>, u32)> {
    let data = fs::read(path).ok()?;
    if data.len() < 44 || &data[0..4] != b"RIFF" || &data[8..12] != b"WAVE" {
        return None;
    }

    let channels = u16::from_le_bytes([data[22], data[23]]) as usize;
    let sample_rate = u32::from_le_bytes([data[24], data[25], data[26], data[27]]);
    let bits = u16::from_le_bytes([data[34], data[35]]);

    // Find the data chunk (don't assume it starts at byte 44)
    let mut offset = 12;
    while offset + 8 < data.len() {
        let chunk_id = &data[offset..offset + 4];
        let chunk_size = u32::from_le_bytes([
            data[offset + 4],
            data[offset + 5],
            data[offset + 6],
            data[offset + 7],
        ]) as usize;
        if chunk_id == b"data" {
            let start = offset + 8;
            let end = (start + chunk_size).min(data.len());
            let pcm = &data[start..end];
            let samples = decode_pcm(pcm, bits, channels, true);
            return Some((samples, sample_rate));
        }
        offset += 8 + chunk_size;
        if !chunk_size.is_multiple_of(2) {
            offset += 1;
        }
    }
    None
}

/// Read AIFF file and return mono f32 samples + sample rate.
fn read_aiff_pcm(path: &Path) -> Option<(Vec<f32>, u32)> {
    let data = fs::read(path).ok()?;
    if data.len() < 12 || &data[0..4] != b"FORM" || &data[8..12] != b"AIFF" {
        return None;
    }

    let mut channels = 0u16;
    let mut bits = 0u16;
    let mut sample_rate = 0u32;
    let mut ssnd_data: Option<&[u8]> = None;

    let mut offset = 12;
    while offset + 8 < data.len() {
        let chunk_id = &data[offset..offset + 4];
        let chunk_size = u32::from_be_bytes([
            data[offset + 4],
            data[offset + 5],
            data[offset + 6],
            data[offset + 7],
        ]) as usize;

        if chunk_id == b"COMM" && offset + 26 <= data.len() {
            channels = u16::from_be_bytes([data[offset + 8], data[offset + 9]]);
            bits = u16::from_be_bytes([data[offset + 14], data[offset + 15]]);
            // 80-bit extended float sample rate
            let exp = u16::from_be_bytes([data[offset + 16], data[offset + 17]]) as i32;
            let mantissa = u32::from_be_bytes([
                data[offset + 18],
                data[offset + 19],
                data[offset + 20],
                data[offset + 21],
            ]);
            sample_rate = (mantissa as f64 * 2f64.powi(exp - 16383 - 31)).round() as u32;
        } else if chunk_id == b"SSND" {
            // SSND has 8 bytes of offset/blockSize before sample data
            let start = offset + 8 + 8;
            let end = (offset + 8 + chunk_size).min(data.len());
            if start < end {
                ssnd_data = Some(&data[start..end]);
            }
        }

        offset += 8 + chunk_size;
        if !chunk_size.is_multiple_of(2) {
            offset += 1;
        }
    }

    let pcm = ssnd_data?;
    if channels == 0 || sample_rate == 0 {
        return None;
    }
    let samples = decode_pcm(pcm, bits, channels as usize, false);
    Some((samples, sample_rate))
}

/// Decode raw PCM bytes to mono f32 samples, normalized to [-1, 1].
fn decode_pcm(data: &[u8], bits: u16, channels: usize, little_endian: bool) -> Vec<f32> {
    let bytes_per_sample = (bits / 8) as usize;
    let frame_size = bytes_per_sample * channels;
    if frame_size == 0 {
        return vec![];
    }

    let num_frames = data.len() / frame_size;
    // Cap at ~30 seconds at 44.1kHz for performance
    let max_frames = 44100 * 30;
    let frames = num_frames.min(max_frames);
    let mut samples = Vec::with_capacity(frames);

    for i in 0..frames {
        let offset = i * frame_size;
        // Read first channel only (mono mixdown)
        let sample = match bits {
            16 => {
                let raw = if little_endian {
                    i16::from_le_bytes([data[offset], data[offset + 1]])
                } else {
                    i16::from_be_bytes([data[offset], data[offset + 1]])
                };
                raw as f32 / 32768.0
            }
            24 => {
                let (b0, b1, b2) = if little_endian {
                    (data[offset], data[offset + 1], data[offset + 2])
                } else {
                    (data[offset + 2], data[offset + 1], data[offset])
                };
                let raw = ((b2 as i32) << 24 | (b1 as i32) << 16 | (b0 as i32) << 8) >> 8;
                raw as f32 / 8388608.0
            }
            32 => {
                let raw = if little_endian {
                    i32::from_le_bytes([
                        data[offset],
                        data[offset + 1],
                        data[offset + 2],
                        data[offset + 3],
                    ])
                } else {
                    i32::from_be_bytes([
                        data[offset],
                        data[offset + 1],
                        data[offset + 2],
                        data[offset + 3],
                    ])
                };
                raw as f32 / 2147483648.0
            }
            8 => (data[offset] as f32 - 128.0) / 128.0,
            _ => 0.0,
        };
        samples.push(sample);
    }

    samples
}

/// Decode compressed audio (MP3, FLAC, OGG, M4A, AAC) via symphonia.
/// Returns mono f32 samples + sample rate, or None on failure.
fn decode_with_symphonia(path: &Path) -> Option<(Vec<f32>, u32)> {
    use symphonia::core::audio::SampleBuffer;
    use symphonia::core::codecs::DecoderOptions;
    use symphonia::core::formats::FormatOptions;
    use symphonia::core::io::MediaSourceStream;
    use symphonia::core::meta::MetadataOptions;
    use symphonia::core::probe::Hint;

    let file = std::fs::File::open(path).ok()?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let probed = symphonia::default::get_probe()
        .format(
            &hint,
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .ok()?;

    let mut format = probed.format;
    let track = format.default_track()?;
    let sample_rate = track.codec_params.sample_rate?;
    let channels = track.codec_params.channels.map(|c| c.count()).unwrap_or(1);
    let track_id = track.id;

    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .ok()?;

    let mut all_samples: Vec<f32> = Vec::new();
    // Limit to ~30 seconds for BPM detection (avoid decoding huge files)
    let max_samples = sample_rate as usize * 30 * channels;

    while let Ok(packet) = format.next_packet() {
        if packet.track_id() != track_id {
            continue;
        }
        let decoded = match decoder.decode(&packet) {
            Ok(d) => d,
            Err(_) => continue,
        };

        let spec = *decoded.spec();
        let duration = decoded.capacity();
        let mut sample_buf = SampleBuffer::<f32>::new(duration as u64, spec);
        sample_buf.copy_interleaved_ref(decoded);

        let buf = sample_buf.samples();
        // Mix to mono
        if channels > 1 {
            for chunk in buf.chunks_exact(channels) {
                let mono: f32 = chunk.iter().sum::<f32>() / channels as f32;
                all_samples.push(mono);
            }
        } else {
            all_samples.extend_from_slice(buf);
        }

        if all_samples.len() >= max_samples {
            break;
        }
    }

    if all_samples.is_empty() {
        return None;
    }

    Some((all_samples, sample_rate))
}

/// Detect tempo using onset-strength autocorrelation.
fn detect_tempo(samples: &[f32], sample_rate: u32) -> Option<f64> {
    // Window size for energy computation (~23ms at 44.1kHz)
    let hop = (sample_rate as usize) / 43; // ~1024 at 44.1kHz
    if hop == 0 {
        return None;
    }
    let num_frames = samples.len() / hop;
    if num_frames < 4 {
        return None;
    }

    // Compute RMS energy per frame
    let mut energy = Vec::with_capacity(num_frames);
    for i in 0..num_frames {
        let start = i * hop;
        let end = (start + hop).min(samples.len());
        let rms: f32 =
            samples[start..end].iter().map(|s| s * s).sum::<f32>() / (end - start) as f32;
        energy.push(rms.sqrt());
    }

    // Onset strength: half-wave rectified first difference
    let mut onset = Vec::with_capacity(num_frames);
    onset.push(0.0f32);
    for i in 1..energy.len() {
        let diff = energy[i] - energy[i - 1];
        onset.push(diff.max(0.0));
    }

    // Normalize onset strength
    let max_onset = onset.iter().cloned().fold(0.0f32, f32::max);
    if max_onset < 1e-8 {
        return None; // silence
    }
    for v in onset.iter_mut() {
        *v /= max_onset;
    }

    // Autocorrelation over BPM range 50–220
    let frame_rate = sample_rate as f64 / hop as f64;
    let min_lag = (frame_rate * 60.0 / 220.0).floor() as usize; // 220 BPM
    let max_lag = (frame_rate * 60.0 / 50.0).ceil() as usize; // 50 BPM
    let max_lag = max_lag.min(onset.len() - 1);

    if min_lag >= max_lag || max_lag >= onset.len() {
        return None;
    }

    // Compute raw autocorrelation for all lags
    let mut corr_values = vec![0.0f64; max_lag + 1];
    for lag in min_lag..=max_lag {
        let n = onset.len() - lag;
        let mut c = 0.0f64;
        for i in 0..n {
            c += onset[i] as f64 * onset[i + lag] as f64;
        }
        corr_values[lag] = c / n as f64;
    }

    // Find raw best lag
    let mut best_lag = min_lag;
    let mut best_corr = f64::NEG_INFINITY;
    for (lag, &corr) in corr_values
        .iter()
        .enumerate()
        .skip(min_lag)
        .take(max_lag - min_lag + 1)
    {
        if corr > best_corr {
            best_corr = corr;
            best_lag = lag;
        }
    }

    if best_corr < 0.01 {
        return None;
    }

    // Collect candidate tempos: the raw peak + sub-harmonics (lag/2, lag/3)
    let mut candidates: Vec<(f64, f64)> = Vec::new(); // (bpm, correlation)

    for divisor in 1..=3 {
        let candidate_lag = best_lag / divisor;
        if candidate_lag >= min_lag && candidate_lag <= max_lag {
            let c = corr_values[candidate_lag];
            let bpm = frame_rate * 60.0 / candidate_lag as f64;
            candidates.push((bpm, c));
        }
    }

    // Also check the raw best
    let raw_bpm = frame_rate * 60.0 / best_lag as f64;
    candidates.push((raw_bpm, best_corr));

    // Select best candidate: if any candidate in the 80–170 BPM range has
    // reasonable correlation (>30% of best), prefer it over out-of-range peaks
    let mut final_bpm = raw_bpm;
    let mut best_in_range: Option<(f64, f64)> = None;

    for &(bpm, corr) in &candidates {
        if (80.0..=170.0).contains(&bpm)
            && corr > best_corr * 0.25
            && (best_in_range.is_none() || corr > best_in_range.unwrap().1)
        {
            best_in_range = Some((bpm, corr));
        }
    }

    if let Some((bpm, _)) = best_in_range {
        final_bpm = bpm;
    } else {
        // Fallback: pick highest-weighted candidate
        let mut best_score = f64::NEG_INFINITY;
        for &(bpm, corr) in &candidates {
            let weight = if (60.0..=220.0).contains(&bpm) {
                1.2
            } else {
                1.0
            };
            if corr * weight > best_score {
                best_score = corr * weight;
                final_bpm = bpm;
            }
        }
    }

    // Parabolic interpolation for sub-frame accuracy
    let final_lag = (frame_rate * 60.0 / final_bpm).round() as usize;
    let refined_bpm = if final_lag > min_lag && final_lag < max_lag {
        let prev = corr_values[final_lag - 1];
        let curr = corr_values[final_lag];
        let next = corr_values[final_lag + 1];
        let denom = 2.0 * (2.0 * curr - prev - next);
        if denom.abs() > 1e-12 {
            let refined_lag = final_lag as f64 + (prev - next) / denom;
            frame_rate * 60.0 / refined_lag
        } else {
            final_bpm
        }
    } else {
        final_bpm
    };

    // Round to nearest whole number if within 0.15, otherwise keep 1 decimal
    let rounded = (refined_bpm * 10.0).round() / 10.0;
    let nearest_int = rounded.round();
    if (rounded - nearest_int).abs() < 0.15 {
        Some(nearest_int)
    } else {
        Some(rounded)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    fn write_wav(path: &Path, samples: &[f32], sample_rate: u32) {
        let num_samples = samples.len() as u32;
        let bits: u16 = 16;
        let channels: u16 = 1;
        let byte_rate = sample_rate * (bits as u32 / 8) * channels as u32;
        let block_align = channels * (bits / 8);
        let data_size = num_samples * (bits as u32 / 8);
        let file_size = 36 + data_size;

        let mut buf = Vec::with_capacity(44 + data_size as usize);
        buf.extend_from_slice(b"RIFF");
        buf.extend_from_slice(&file_size.to_le_bytes());
        buf.extend_from_slice(b"WAVE");
        buf.extend_from_slice(b"fmt ");
        buf.extend_from_slice(&16u32.to_le_bytes()); // chunk size
        buf.extend_from_slice(&1u16.to_le_bytes()); // PCM
        buf.extend_from_slice(&channels.to_le_bytes());
        buf.extend_from_slice(&sample_rate.to_le_bytes());
        buf.extend_from_slice(&byte_rate.to_le_bytes());
        buf.extend_from_slice(&block_align.to_le_bytes());
        buf.extend_from_slice(&bits.to_le_bytes());
        buf.extend_from_slice(b"data");
        buf.extend_from_slice(&data_size.to_le_bytes());

        for &s in samples {
            let i = (s.clamp(-1.0, 1.0) * 32767.0) as i16;
            buf.extend_from_slice(&i.to_le_bytes());
        }

        fs::write(path, buf).unwrap();
    }

    /// Generate a click track at a specific BPM.
    fn click_track(bpm: f64, duration_secs: f64, sample_rate: u32) -> Vec<f32> {
        let num_samples = (duration_secs * sample_rate as f64) as usize;
        let samples_per_beat = (60.0 / bpm * sample_rate as f64) as usize;
        let click_len = (sample_rate as usize) / 100; // 10ms click

        let mut samples = vec![0.0f32; num_samples];
        let mut pos = 0;
        while pos < num_samples {
            for i in 0..click_len.min(num_samples - pos) {
                // Short sine burst
                let t = i as f32 / sample_rate as f32;
                let envelope = 1.0 - (i as f32 / click_len as f32);
                samples[pos + i] = (2.0 * PI * 1000.0 * t).sin() * envelope * 0.8;
            }
            pos += samples_per_beat;
        }
        samples
    }

    #[test]
    fn test_estimate_bpm_unsupported_format() {
        assert!(estimate_bpm("/some/file.mp3").is_none());
    }

    #[test]
    fn test_estimate_bpm_nonexistent() {
        assert!(estimate_bpm("/nonexistent/file.wav").is_none());
    }

    #[test]
    fn test_read_wav_pcm_pub_truncated_file_returns_none() {
        let tmp = std::env::temp_dir().join("test_bpm_trunc.wav");
        fs::write(&tmp, b"RIFF").unwrap();
        assert!(read_wav_pcm_pub(&tmp).is_none());
        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn test_read_wav_pcm_pub_not_riff_returns_none() {
        let tmp = std::env::temp_dir().join("test_bpm_not_riff.wav");
        fs::write(&tmp, b"XXXX0000WAVE").unwrap();
        assert!(read_wav_pcm_pub(&tmp).is_none());
        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn test_estimate_bpm_silence() {
        let tmp = std::env::temp_dir().join("test_bpm_silence.wav");
        let silence = vec![0.0f32; 44100 * 4];
        write_wav(&tmp, &silence, 44100);

        let result = estimate_bpm(tmp.to_str().unwrap());
        assert!(result.is_none(), "Silence should not produce a BPM");

        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn test_estimate_bpm_120() {
        let tmp = std::env::temp_dir().join("test_bpm_120.wav");
        let samples = click_track(120.0, 8.0, 44100);
        write_wav(&tmp, &samples, 44100);

        let bpm = estimate_bpm(tmp.to_str().unwrap());
        assert!(bpm.is_some(), "Should detect BPM");
        let bpm = bpm.unwrap();
        assert!((bpm - 120.0).abs() < 8.0, "Expected ~120 BPM, got {bpm}");

        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn test_estimate_bpm_140() {
        let tmp = std::env::temp_dir().join("test_bpm_140.wav");
        let samples = click_track(140.0, 8.0, 44100);
        write_wav(&tmp, &samples, 44100);

        let bpm = estimate_bpm(tmp.to_str().unwrap());
        assert!(bpm.is_some());
        let bpm = bpm.unwrap();
        assert!((bpm - 140.0).abs() < 8.0, "Expected ~140 BPM, got {bpm}");

        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn test_estimate_bpm_90() {
        let tmp = std::env::temp_dir().join("test_bpm_90.wav");
        let samples = click_track(90.0, 8.0, 44100);
        write_wav(&tmp, &samples, 44100);

        let bpm = estimate_bpm(tmp.to_str().unwrap());
        assert!(bpm.is_some());
        let bpm = bpm.unwrap();
        assert!((bpm - 90.0).abs() < 8.0, "Expected ~90 BPM, got {bpm}");

        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn test_estimate_bpm_short_file() {
        let tmp = std::env::temp_dir().join("test_bpm_short.wav");
        // Very short file — 0.1 seconds
        let samples = vec![0.5f32; 4410];
        write_wav(&tmp, &samples, 44100);

        // Should return None — too short to detect
        let result = estimate_bpm(tmp.to_str().unwrap());
        assert!(result.is_none());

        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn test_decode_pcm_16bit() {
        // Two 16-bit LE samples: 16384 (0.5) and -16384 (-0.5)
        let data = [0x00u8, 0x40, 0x00, 0xC0];
        let samples = decode_pcm(&data, 16, 1, true);
        assert_eq!(samples.len(), 2);
        assert!((samples[0] - 0.5).abs() < 0.001);
        assert!((samples[1] + 0.5).abs() < 0.001);
    }

    #[test]
    fn test_decode_pcm_16bit_be() {
        // Big-endian 0x4000 = 16384 → 0.5 normalized
        let data = [0x40u8, 0x00];
        let samples = decode_pcm(&data, 16, 1, false);
        assert_eq!(samples.len(), 1);
        assert!((samples[0] - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_decode_pcm_8bit() {
        let data = [128u8, 255, 0]; // 0.0, ~1.0, ~-1.0
        let samples = decode_pcm(&data, 8, 1, true);
        assert_eq!(samples.len(), 3);
        assert!((samples[0]).abs() < 0.01);
        assert!(samples[1] > 0.9);
        assert!(samples[2] < -0.9);
    }

    #[test]
    fn test_decode_pcm_stereo_takes_left() {
        // Two stereo frames, 16-bit LE: (L=0.5, R=-0.5), (L=-0.25, R=0.25)
        let data = [
            0x00u8, 0x40, 0x00, 0xC0, // frame 1
            0x00, 0xE0, 0x00, 0x20, // frame 2
        ];
        let samples = decode_pcm(&data, 16, 2, true);
        assert_eq!(samples.len(), 2);
        assert!((samples[0] - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_estimate_bpm_174() {
        let tmp = std::env::temp_dir().join("test_bpm_174.wav");
        let samples = click_track(174.0, 8.0, 44100);
        write_wav(&tmp, &samples, 44100);

        let bpm = estimate_bpm(tmp.to_str().unwrap());
        assert!(
            bpm.is_some(),
            "Should detect BPM for 174 drum-and-bass tempo"
        );
        let bpm = bpm.unwrap();
        assert!((bpm - 174.0).abs() < 8.0, "Expected ~174 BPM, got {bpm}");

        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn test_decode_pcm_32bit_le() {
        let mut data = vec![0u8; 4];
        let raw = 0x4000_0000i32;
        data.copy_from_slice(&raw.to_le_bytes());
        let samples = decode_pcm(&data, 32, 1, true);
        assert_eq!(samples.len(), 1);
        assert!(
            (samples[0] - 0.5).abs() < 1e-5,
            "32-bit LE 0x40000000 should normalize ~0.5, got {}",
            samples[0]
        );
    }

    #[test]
    fn test_decode_pcm_24bit() {
        // 24-bit LE sample: bytes [0x00, 0x00, 0x40] = 0x400000 as signed = 4194304
        // normalized: 4194304 / 8388608 = 0.5
        let data = [0x00u8, 0x00, 0x40];
        let samples = decode_pcm(&data, 24, 1, true);
        assert_eq!(samples.len(), 1);
        assert!(
            (samples[0] - 0.5).abs() < 0.001,
            "Expected ~0.5, got {}",
            samples[0]
        );
    }

    #[test]
    fn test_decode_pcm_empty() {
        let data: [u8; 0] = [];
        let samples = decode_pcm(&data, 16, 1, true);
        assert!(
            samples.is_empty(),
            "Empty input should produce empty output"
        );
    }

    #[test]
    fn test_decode_pcm_16bit_truncates_trailing_odd_byte() {
        let data = [0x00u8, 0x40, 0xFF];
        let samples = decode_pcm(&data, 16, 1, true);
        assert_eq!(samples.len(), 1);
        assert!((samples[0] - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_decode_pcm_32bit_big_endian() {
        let mut data = vec![0u8; 4];
        let raw = 0x4000_0000i32;
        data.copy_from_slice(&raw.to_be_bytes());
        let samples = decode_pcm(&data, 32, 1, false);
        assert_eq!(samples.len(), 1);
        assert!(
            (samples[0] - 0.5).abs() < 1e-5,
            "32-bit BE 0x40000000 → ~0.5, got {}",
            samples[0]
        );
    }

    #[test]
    fn test_decode_pcm_unsupported_bit_depth_yields_zero_sample() {
        let data = [0xABu8, 0xCD];
        let samples = decode_pcm(&data, 12, 1, true);
        assert_eq!(samples.len(), 2);
        assert_eq!(samples[0], 0.0);
        assert_eq!(samples[1], 0.0);
    }

    #[test]
    fn test_read_wav_invalid_header() {
        let tmp = std::env::temp_dir().join("test_bpm_invalid_header.wav");
        // Write data that is NOT a valid RIFF header
        fs::write(
            &tmp,
            b"NOT_RIFF_DATA_HERE_1234567890abcdefghijklmnopqrstuvwx",
        )
        .unwrap();

        let result = read_wav_pcm(&tmp);
        assert!(result.is_none(), "Non-RIFF data should return None");

        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn test_read_aiff_basic() {
        let tmp = std::env::temp_dir().join("test_bpm_aiff_basic.aiff");

        let mut data = Vec::new();
        data.extend_from_slice(b"FORM");
        // total size placeholder — filled after building
        data.extend_from_slice(&[0u8; 4]);
        data.extend_from_slice(b"AIFF");

        // COMM chunk: 18 bytes
        data.extend_from_slice(b"COMM");
        data.extend_from_slice(&18u32.to_be_bytes());
        data.extend_from_slice(&1u16.to_be_bytes()); // channels = 1
        data.extend_from_slice(&1000u32.to_be_bytes()); // num sample frames
        data.extend_from_slice(&16u16.to_be_bytes()); // bits per sample
                                                      // 80-bit extended for 44100 Hz:
                                                      // exponent = 16383 + 15 = 16398 = 0x400E
                                                      // mantissa high 32 bits = 44100 << 16 = 0xAC44_0000
        data.extend_from_slice(&[0x40, 0x0E, 0xAC, 0x44, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);

        // SSND chunk: 8 bytes header (offset + blockSize) + PCM data
        let pcm_bytes = 1000 * 2; // 1000 frames, 16-bit mono
        let ssnd_size = 8 + pcm_bytes;
        data.extend_from_slice(b"SSND");
        data.extend_from_slice(&(ssnd_size as u32).to_be_bytes());
        data.extend_from_slice(&0u32.to_be_bytes()); // offset
        data.extend_from_slice(&0u32.to_be_bytes()); // blockSize
                                                     // 1000 frames of silence (big-endian 16-bit zeros)
        data.extend_from_slice(&vec![0u8; pcm_bytes]);

        // Fix FORM size
        let form_size = (data.len() - 8) as u32;
        data[4..8].copy_from_slice(&form_size.to_be_bytes());

        fs::write(&tmp, &data).unwrap();

        let result = read_aiff_pcm(&tmp);
        assert!(result.is_some(), "Valid AIFF should parse successfully");
        let (samples, sr) = result.unwrap();
        assert_eq!(sr, 44100);
        assert_eq!(samples.len(), 1000);
        // All samples should be zero (silence)
        assert!(samples.iter().all(|&s| s.abs() < 0.001));

        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn test_read_wav_with_extra_chunks() {
        // WAV with a LIST chunk before data
        let tmp = std::env::temp_dir().join("test_bpm_extrachunk.wav");
        let pcm_data: Vec<u8> = (0..4410).flat_map(|_| 0i16.to_le_bytes()).collect();
        let list_chunk = b"LIST\x04\x00\x00\x00INFO";
        let data_size = pcm_data.len() as u32;
        let file_size = 4 + 24 + 8 + list_chunk.len() as u32 + 8 + data_size;

        let mut buf = Vec::new();
        buf.extend_from_slice(b"RIFF");
        buf.extend_from_slice(&file_size.to_le_bytes());
        buf.extend_from_slice(b"WAVE");
        buf.extend_from_slice(b"fmt ");
        buf.extend_from_slice(&16u32.to_le_bytes());
        buf.extend_from_slice(&1u16.to_le_bytes()); // PCM
        buf.extend_from_slice(&1u16.to_le_bytes()); // mono
        buf.extend_from_slice(&44100u32.to_le_bytes());
        buf.extend_from_slice(&88200u32.to_le_bytes()); // byte rate
        buf.extend_from_slice(&2u16.to_le_bytes()); // block align
        buf.extend_from_slice(&16u16.to_le_bytes()); // bits
        buf.extend_from_slice(list_chunk);
        buf.extend_from_slice(b"data");
        buf.extend_from_slice(&data_size.to_le_bytes());
        buf.extend_from_slice(&pcm_data);

        fs::write(&tmp, buf).unwrap();

        let result = read_wav_pcm(&tmp);
        assert!(result.is_some());
        let (samples, sr) = result.unwrap();
        assert_eq!(sr, 44100);
        assert_eq!(samples.len(), 4410);

        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn test_decode_symphonia_nonexistent() {
        let result = decode_with_symphonia(Path::new("/nonexistent/file.mp3"));
        assert!(result.is_none());
    }

    #[test]
    fn test_decode_symphonia_invalid_data() {
        let tmp = std::env::temp_dir().join("bpm_test_invalid.mp3");
        fs::write(&tmp, b"this is not an mp3 file").unwrap();
        let result = decode_with_symphonia(&tmp);
        assert!(result.is_none(), "garbage data should return None");
        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn test_decode_symphonia_wav_fallback() {
        // symphonia should be able to decode a valid WAV too
        let tmp = std::env::temp_dir().join("bpm_test_sym_wav.wav");
        let sr = 44100u32;
        let n = 4410usize; // 0.1s
        let mut buf = Vec::new();
        let data_size = (n * 2) as u32;
        buf.extend_from_slice(b"RIFF");
        buf.extend_from_slice(&(36 + data_size).to_le_bytes());
        buf.extend_from_slice(b"WAVE");
        buf.extend_from_slice(b"fmt ");
        buf.extend_from_slice(&16u32.to_le_bytes());
        buf.extend_from_slice(&1u16.to_le_bytes()); // PCM
        buf.extend_from_slice(&1u16.to_le_bytes()); // mono
        buf.extend_from_slice(&sr.to_le_bytes());
        buf.extend_from_slice(&(sr * 2).to_le_bytes());
        buf.extend_from_slice(&2u16.to_le_bytes());
        buf.extend_from_slice(&16u16.to_le_bytes());
        buf.extend_from_slice(b"data");
        buf.extend_from_slice(&data_size.to_le_bytes());
        for i in 0..n {
            let t = i as f64 / sr as f64;
            let s = ((t * 440.0 * 2.0 * std::f64::consts::PI).sin() * 16000.0) as i16;
            buf.extend_from_slice(&s.to_le_bytes());
        }
        fs::write(&tmp, &buf).unwrap();

        let result = decode_with_symphonia(&tmp);
        assert!(result.is_some(), "symphonia should decode valid WAV");
        let (samples, rate) = result.unwrap();
        assert_eq!(rate, 44100);
        assert!(!samples.is_empty());

        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn test_estimate_bpm_zero_length() {
        // WAV with zero data samples
        let tmp = std::env::temp_dir().join("bpm_test_zero.wav");
        let mut buf = Vec::new();
        buf.extend_from_slice(b"RIFF");
        buf.extend_from_slice(&36u32.to_le_bytes());
        buf.extend_from_slice(b"WAVE");
        buf.extend_from_slice(b"fmt ");
        buf.extend_from_slice(&16u32.to_le_bytes());
        buf.extend_from_slice(&1u16.to_le_bytes());
        buf.extend_from_slice(&1u16.to_le_bytes());
        buf.extend_from_slice(&44100u32.to_le_bytes());
        buf.extend_from_slice(&(44100u32 * 2).to_le_bytes());
        buf.extend_from_slice(&2u16.to_le_bytes());
        buf.extend_from_slice(&16u16.to_le_bytes());
        buf.extend_from_slice(b"data");
        buf.extend_from_slice(&0u32.to_le_bytes());
        fs::write(&tmp, &buf).unwrap();

        let bpm = estimate_bpm(tmp.to_str().unwrap());
        assert!(bpm.is_none(), "zero-length audio should not estimate BPM");
        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn test_read_aiff_nonexistent() {
        let result = read_aiff_pcm(Path::new("/nonexistent/file.aiff"));
        assert!(result.is_none());
    }

    #[test]
    fn test_read_aiff_invalid_header() {
        let tmp = std::env::temp_dir().join("bpm_test_bad_aiff.aiff");
        fs::write(&tmp, b"not an aiff file at all").unwrap();
        let result = read_aiff_pcm(&tmp);
        assert!(result.is_none());
        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn test_bpm_rounding_snaps_to_integer() {
        // Values within 0.15 of integer should snap
        let round = |v: f64| -> f64 {
            let rounded = (v * 10.0).round() / 10.0;
            let nearest = rounded.round();
            if (rounded - nearest).abs() < 0.15 {
                nearest
            } else {
                rounded
            }
        };
        assert_eq!(round(120.08), 120.0);
        assert_eq!(round(119.92), 120.0);
        assert_eq!(round(128.14), 128.0);
        assert_eq!(round(128.0), 128.0);
        // Values beyond 0.15 keep decimal
        assert_eq!(round(127.5), 127.5);
        assert_eq!(round(135.3), 135.3);
        assert_eq!(round(99.8), 99.8);
    }
}
