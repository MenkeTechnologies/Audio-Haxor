//! Adversarial AIFF parser pins for [`app_lib::bpm::read_aiff_pcm_pub`].
//!
//! These exist to catch a specific bug class no existing test exercises: byte
//! values supplied by the file that drive arithmetic primitives (`exp`,
//! `chunk_size`) past their well-formed ranges. A robust decoder must either
//! reject the file or return what little it can — what it MUST NOT do is panic
//! on adversarial input. None of `tests/*.rs` constructs AIFF byte buffers, and
//! the in-crate tests in `src/bpm.rs` only assert the happy path
//! (`test_read_aiff_basic`) and missing-file (`test_read_aiff_nonexistent`) /
//! wrong-magic (`test_read_aiff_invalid_header`) negatives.
//!
//! Each test below explains the **byte-level invariant** it pins and why it is
//! not a mirror of an existing test.

use std::fs;
use std::path::{Path, PathBuf};

fn tmp_aiff(label: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!(
        "ah_aiff_corrupt_{}_{}.aiff",
        std::process::id(),
        label
    ));
    let _ = fs::remove_file(&p);
    p
}

/// Build a well-formed AIFF prefix (FORM + AIFF + COMM at 44.1kHz, 16-bit mono)
/// up to the start of where the SSND chunk would be written. Caller appends an
/// SSND chunk (or other content) and patches the FORM size in bytes 4..8.
fn build_aiff_prefix() -> Vec<u8> {
    let mut data = Vec::new();
    data.extend_from_slice(b"FORM");
    data.extend_from_slice(&[0u8; 4]); // FORM size placeholder
    data.extend_from_slice(b"AIFF");
    // COMM (18 bytes): channels(2) numFrames(4) bits(2) rate(10 = 80-bit extended)
    data.extend_from_slice(b"COMM");
    data.extend_from_slice(&18u32.to_be_bytes());
    data.extend_from_slice(&1u16.to_be_bytes()); // channels = 1
    data.extend_from_slice(&100u32.to_be_bytes()); // numSampleFrames (informational)
    data.extend_from_slice(&16u16.to_be_bytes()); // bits per sample
                                                  // 80-bit extended for 44100 Hz: exp=0x400E, mantissa hi32=0xAC440000.
    data.extend_from_slice(&[0x40, 0x0E, 0xAC, 0x44, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
    data
}

fn finalize_form_size(data: &mut [u8]) {
    let form_size = (data.len() - 8) as u32;
    data[4..8].copy_from_slice(&form_size.to_be_bytes());
}

/// REGRESSION (corrupt SSND chunk size < 8): the AIFF SSND chunk format requires
/// at least 8 bytes of header (offset:u32 + blockSize:u32) before any PCM data.
/// A file with `chunk_size == 4` describes an SSND chunk whose declared length
/// covers only half the mandatory header. The parser must NOT (a) panic on the
/// `(offset + 8 + chunk_size)` arithmetic, (b) panic on the implicit
/// `&data[start..end]` slice when `start > end`, or (c) read past the chunk's
/// declared end as if it contained PCM samples. Acceptable outcomes: return
/// `None` (correct — no usable samples found), or return `Some((empty, sr))`.
/// What is forbidden: panic, or returning fabricated samples not present in the
/// file.
///
/// Why this is not boilerplate: `test_read_aiff_basic` covers `chunk_size = 8 +
/// pcm_bytes` (the happy path). `test_read_aiff_invalid_header` covers the
/// FORM/AIFF magic check (returns None before the chunk loop). Neither test
/// reaches the SSND `start < end` branch with a malformed `chunk_size`. The
/// `start < end` guard at the SSND site is the exact thing this test exercises.
#[test]
fn read_aiff_ssnd_chunk_size_4_returns_none_without_panic() {
    let mut data = build_aiff_prefix();
    // SSND chunk_size = 4 (corrupt — less than the 8-byte minimum header).
    // We still write 4 bytes of data so the file isn't truncated mid-chunk.
    data.extend_from_slice(b"SSND");
    data.extend_from_slice(&4u32.to_be_bytes());
    data.extend_from_slice(&[0u8; 4]); // 4 bytes of "partial header"
    finalize_form_size(&mut data);

    let path = tmp_aiff("ssnd_size_4");
    fs::write(&path, &data).unwrap();
    let result = app_lib::bpm::read_aiff_pcm_pub(Path::new(&path));
    let _ = fs::remove_file(&path);

    // `start = offset + 16`, `end = offset + 8 + 4 = offset + 12`. `start > end`,
    // so `ssnd_data` stays None, then `let pcm = ssnd_data?` short-circuits ->
    // function returns None overall. The pin: this must NOT panic and must NOT
    // return Some with garbage.
    assert!(
        result.is_none(),
        "corrupt SSND (chunk_size=4) must yield None; got {result:?}"
    );
}

/// REGRESSION (SSND chunk_size = 8 — exactly the 8-byte header, zero PCM
/// bytes): the chunk is structurally valid but carries no samples. The decoder
/// must produce `None` (no decodable audio) rather than `Some((empty_vec, sr))`
/// which would propagate an "decoded 0 samples at 44.1kHz" lie to the BPM
/// detector. The internal guard is `if start < end { … }`: with `chunk_size =
/// 8`, `end = start`, so the guard is false and `ssnd_data` stays None.
///
/// Why this is not boilerplate: `test_read_aiff_basic` uses `ssnd_size = 8 +
/// pcm_bytes` (non-zero PCM). No existing test pins the exact `chunk_size == 8`
/// boundary, which is the smallest legal SSND according to the spec but
/// nonetheless represents an "empty body" the BPM pipeline must reject early.
#[test]
fn read_aiff_ssnd_chunk_size_exactly_8_yields_none() {
    let mut data = build_aiff_prefix();
    data.extend_from_slice(b"SSND");
    data.extend_from_slice(&8u32.to_be_bytes());
    data.extend_from_slice(&0u32.to_be_bytes()); // offset
    data.extend_from_slice(&0u32.to_be_bytes()); // blockSize
                                                 // No PCM bytes after the header — the SSND body length is exactly the 8-byte
                                                 // (offset + blockSize) header. The decoder must reject this as "no samples".
    finalize_form_size(&mut data);

    let path = tmp_aiff("ssnd_size_8");
    fs::write(&path, &data).unwrap();
    let result = app_lib::bpm::read_aiff_pcm_pub(Path::new(&path));
    let _ = fs::remove_file(&path);

    assert!(
        result.is_none(),
        "SSND with exactly 8 bytes (header only, no PCM) must yield None; got {result:?}"
    );
}

/// REGRESSION (AIFF with no SSND chunk at all): a file that's structurally
/// valid (FORM + AIFF + COMM) but lacks any SSND chunk must yield None instead
/// of panicking or returning fabricated empty samples. The chunk loop walks the
/// entire FORM payload, finds only COMM, and reaches the final
/// `let pcm = ssnd_data?;` which must short-circuit. Any other outcome (Some
/// with zero samples, panic, infinite loop) is a bug.
///
/// Why this is not boilerplate: `test_read_aiff_invalid_header` covers the
/// FORM magic-mismatch path (returns before the chunk loop). This test reaches
/// the chunk loop with valid magic but never finds an SSND, exercising the
/// `ssnd_data?` short-circuit. None of `tests/*.rs` builds such a file.
#[test]
fn read_aiff_no_ssnd_chunk_returns_none() {
    let mut data = build_aiff_prefix();
    // No SSND. Just COMM. Add a benign trailing chunk so the loop walks past it.
    data.extend_from_slice(b"ANNO"); // free-form text annotation chunk
    data.extend_from_slice(&4u32.to_be_bytes());
    data.extend_from_slice(b"hiya");
    finalize_form_size(&mut data);

    let path = tmp_aiff("no_ssnd");
    fs::write(&path, &data).unwrap();
    let result = app_lib::bpm::read_aiff_pcm_pub(Path::new(&path));
    let _ = fs::remove_file(&path);

    assert!(
        result.is_none(),
        "AIFF with no SSND chunk must yield None; got {result:?}"
    );
}
