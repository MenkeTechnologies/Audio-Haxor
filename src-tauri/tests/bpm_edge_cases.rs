//! BPM helpers: `estimate_bpm` and PCM readers reject invalid inputs.

use app_lib::bpm::{decode_with_symphonia_pub, estimate_bpm, read_aiff_pcm_pub, read_wav_pcm_pub};

#[test]
fn estimate_bpm_missing_file_returns_none() {
    assert!(estimate_bpm("/nonexistent/audio_haxor/bpm_missing.wav").is_none());
}

#[test]
fn estimate_bpm_unsupported_extension_returns_none() {
    let p = std::env::temp_dir().join("audio_haxor_bpm_edge.txt");
    std::fs::write(&p, b"not audio").ok();
    let path = p.to_string_lossy().to_string();
    assert!(estimate_bpm(&path).is_none());
    let _ = std::fs::remove_file(&p);
}

#[test]
fn read_wav_pcm_truncated_returns_none() {
    let p = std::env::temp_dir().join("audio_haxor_trunc.wav");
    std::fs::write(&p, b"RIFF").unwrap();
    assert!(read_wav_pcm_pub(&p).is_none());
    let _ = std::fs::remove_file(&p);
}

#[test]
fn read_aiff_truncated_returns_none() {
    let p = std::env::temp_dir().join("audio_haxor_trunc.aif");
    std::fs::write(&p, b"FORM").unwrap();
    assert!(read_aiff_pcm_pub(&p).is_none());
    let _ = std::fs::remove_file(&p);
}

#[test]
fn decode_symphonia_non_audio_returns_none() {
    let p = std::env::temp_dir().join("audio_haxor_fake.mp3");
    std::fs::write(&p, b"not a real mp3").unwrap();
    assert!(decode_with_symphonia_pub(&p).is_none());
    let _ = std::fs::remove_file(&p);
}
