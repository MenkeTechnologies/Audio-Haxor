//! Format readers: invalid payloads return `None` from `bpm` PCM helpers.

use std::path::Path;

#[test]
fn test_read_wav_pcm_rejects_garbage() {
    let p = std::env::temp_dir().join("audio_haxor_garbage.wav");
    std::fs::write(&p, b"not a RIFF WAV").unwrap();
    assert!(app_lib::bpm::read_wav_pcm_pub(Path::new(&p)).is_none());
    let _ = std::fs::remove_file(&p);
}

#[test]
fn test_read_aiff_pcm_rejects_incomplete_form() {
    let p = std::env::temp_dir().join("audio_haxor_bad.aiff");
    std::fs::write(&p, b"FORM\x00\x00\x00\x00AIFF").unwrap();
    assert!(app_lib::bpm::read_aiff_pcm_pub(Path::new(&p)).is_none());
    let _ = std::fs::remove_file(&p);
}

#[test]
fn test_pcm_payload_bytes_stereo_24bit_three_minutes() {
    fn pcm_payload_bytes(channels: u16, sample_rate: u32, bits: u16, duration_sec: f32) -> usize {
        let bytes_per_sec =
            (channels as u64) * (sample_rate as u64) * (bits as u64 / 8);
        (bytes_per_sec as f64 * f64::from(duration_sec)) as usize
    }
    let size = pcm_payload_bytes(2, 44100, 24, 180.0);
    assert!((45_000_000..50_000_000).contains(&size), "size={size}");
}
