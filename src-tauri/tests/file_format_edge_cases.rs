//! Edge case tests for various file formats
//! Tests malformed files, truncated headers, and unusual encoding

use app_lib::audio_scanner::get_audio_metadata;

/// Test WAV file with truncated header
#[test]
fn test_wav_truncated_header() {
    let temp = std::env::temp_dir().join("truncated_wav.wav");
    // WAV header is 44 bytes, truncate to 10
    let _ = std::fs::write(&temp, vec![b'R'; 10]);

    let audio_meta = get_audio_metadata(temp.to_str().unwrap());
    // Should handle gracefully
    let _ = audio_meta;
}

/// Test WAV with invalid chunk size
#[test]
fn test_wav_invalid_chunk_size() {
    let temp = std::env::temp_dir().join("invalid_wav.wav");
    // RIFF + FF + zeros (invalid subchunk length)
    let _ = std::fs::write(&temp, b"RIFF\xFF\xFF\xFF\xFFWAVE");

    let audio_meta = get_audio_metadata(temp.to_str().unwrap());
    let _ = audio_meta;
}

/// Test FLAC with bad magic bytes
#[test]
fn test_flac_bad_magic() {
    let temp = std::env::temp_dir().join("bad_flac.flac");
    // Not FLAC magic
    let _ = std::fs::write(&temp, vec![b'X'; 44]);

    let audio_meta = get_audio_metadata(temp.to_str().unwrap());
    let _ = audio_meta;
}

/// Test MP3 truncation edge cases
#[test]
#[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
fn test_mp3_truncated() {
    let temp = std::env::temp_dir().join("truncated.mp3");
    
    // Valid MP3 header: ID3v1 or MPEG frame sync
    // Sync word: 0xFF
    let _ = std::fs::write(&temp, vec![
        0xFF, 0xF0, 0x00, 0x0E, // Frame header (14 bytes)
        0x28, 0x9E, 0x12, 0xAC, 0x52, 0x24, 0x25, 0xD2,
        0xC0, 0x21, 0x25, 0x00, 0x00, // Truncated mid-frame
    ]);

    let audio_meta = get_audio_metadata(temp.to_str().unwrap());
    // Should handle gracefully
    let _ = audio_meta;
}

/// Test OGG Vorbis with bad ID
#[test]
fn test_ogg_bad_vendor() {
    let temp = std::env::temp_dir().join("bad.ogg");
    
    // OGG magic bytes + invalid vendor string
    // Magic: FF FB 90 84
    let _ = std::fs::write(&temp, vec![
        0xFF, 0xFB, 0x90, 0x84,
        b'X', b'X', b'X', b'X', b'X', b'X', b'X', b'X',
        b'X', b'X', b'X', b'X', b'X', b'X', b'X', b'X',
    ]);

    let audio_meta = get_audio_metadata(temp.to_str().unwrap());
    let _ = audio_meta;
}

/// Test AIFF with truncated header
#[test]
fn test_aiff_truncated() {
    let temp = std::env::temp_dir().join("truncated.aif");

    // AIFF container starts with FORM + BE size; stop after magic so parser fails cleanly
    let _ = std::fs::write(&temp, b"FORM");

    let audio_meta = get_audio_metadata(temp.to_str().unwrap());
    let _ = audio_meta;
}