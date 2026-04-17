//! Background waveform prefetch.
//!
//! Bulk-computes min/max peak envelopes for every audio sample in the library and
//! upserts them into the `waveform_cache` SQLite table. Uses symphonia (not the
//! BPM module's manual WAV parser) so 32-bit float WAV, ALAC M4A, and every other
//! symphonia-supported codec works — matching JUCE `waveform_preview` coverage
//! minus raw/obscure containers.
//!
//! Output shape: `Vec<Peak { max, min }>` → JSON → `waveform_cache.data`.

use serde::{Deserialize, Serialize};
use std::path::Path;

/// One peak bucket. Field names match the JS `{max, min}` objects consumed by
/// `audio.js::renderWaveformData`.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Peak {
    pub max: f32,
    pub min: f32,
}

/// Canonical peak resolution for the prefetch — high enough for 1200 px
/// canvases, compact enough (~16 KB JSON per sample).
pub const WAVEFORM_WIDTH_PX: usize = 800;

/// Compute a downsampled min/max peak envelope using symphonia.
///
/// Returns `None` for unsupported formats, unreadable files, or empty decodes.
/// Streams packets without holding the full PCM buffer in memory — safe for
/// multi-hour recordings.
pub fn compute_peaks(file_path: &str, width_px: usize) -> Option<Vec<Peak>> {
    use symphonia::core::audio::SampleBuffer;
    use symphonia::core::codecs::DecoderOptions;
    use symphonia::core::formats::FormatOptions;
    use symphonia::core::io::MediaSourceStream;
    use symphonia::core::meta::MetadataOptions;
    use symphonia::core::probe::Hint;

    let path = Path::new(file_path);
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    // Only attempt known audio extensions (skip videos, midis, etc.)
    if !matches!(
        ext.as_str(),
        "wav" | "aiff" | "aif" | "mp3" | "flac" | "ogg" | "m4a" | "aac" | "opus" | "wma"
    ) {
        return None;
    }

    let width = width_px.clamp(32, 8192);

    let _guard = crate::BgIoGuard::new();
    let file = std::fs::File::open(path).ok()?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    hint.with_extension(&ext);

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

    // Estimate total mono samples from duration hint (if available).
    // Fall back to a generous guess; the peak buckets resize-adapt below.
    let n_frames_hint = track
        .codec_params
        .n_frames
        .map(|n| n as usize)
        .unwrap_or(sample_rate as usize * 600); // default ≈10 min
    let total_mono_hint = n_frames_hint.max(1);
    let bucket_size = (total_mono_hint as f64 / width as f64).max(1.0);

    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .ok()?;

    // Streaming peak accumulation — never holds the full PCM buffer.
    let mut peaks: Vec<Peak> = Vec::with_capacity(width);
    let mut cur_min: f32 = f32::MAX;
    let mut cur_max: f32 = f32::MIN;
    let mut cur_count: usize = 0;
    let mut mono_idx: usize = 0; // running mono sample index

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

        // Mono-mix + bucket accumulation in one pass.
        let ch = channels.max(1);
        let inv = 1.0 / ch as f32;
        let mut i = 0;
        while i + ch <= buf.len() {
            let mono: f32 = if ch == 1 {
                buf[i]
            } else {
                buf[i..i + ch].iter().sum::<f32>() * inv
            };
            i += ch;

            if mono > cur_max {
                cur_max = mono;
            }
            if mono < cur_min {
                cur_min = mono;
            }
            cur_count += 1;
            mono_idx += 1;

            // Bucket full — flush.
            let target_bucket = ((mono_idx as f64) / bucket_size) as usize;
            if target_bucket > peaks.len() || cur_count >= bucket_size as usize {
                if cur_max == f32::MIN {
                    cur_max = 0.0;
                }
                if cur_min == f32::MAX {
                    cur_min = 0.0;
                }
                peaks.push(Peak {
                    max: cur_max.clamp(-1.0, 1.0),
                    min: cur_min.clamp(-1.0, 1.0),
                });
                cur_min = f32::MAX;
                cur_max = f32::MIN;
                cur_count = 0;
                // Stop if we've already filled the requested width.
                if peaks.len() >= width {
                    break;
                }
            }
        }
        if peaks.len() >= width {
            break;
        }
    }

    // Flush any remaining partial bucket.
    if cur_count > 0 && peaks.len() < width {
        if cur_max == f32::MIN {
            cur_max = 0.0;
        }
        if cur_min == f32::MAX {
            cur_min = 0.0;
        }
        peaks.push(Peak {
            max: cur_max.clamp(-1.0, 1.0),
            min: cur_min.clamp(-1.0, 1.0),
        });
    }

    if peaks.is_empty() {
        return None;
    }

    Some(peaks)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unsupported_ext_returns_none() {
        assert!(compute_peaks("/tmp/does_not_exist.txt", WAVEFORM_WIDTH_PX).is_none());
    }

    #[test]
    fn missing_file_returns_none() {
        assert!(compute_peaks("/tmp/absolutely_not_a_real_file_12345.wav", WAVEFORM_WIDTH_PX).is_none());
    }

    #[test]
    fn width_is_clamped() {
        assert!(compute_peaks("/tmp/nope.wav", 0).is_none());
        assert!(compute_peaks("/tmp/nope.wav", usize::MAX).is_none());
    }
}
