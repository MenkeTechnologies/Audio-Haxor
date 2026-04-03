//! Additional behavioral tests: binary MIDI fixtures, URL regex, scanner labels,
//! similarity invariants, and metadata error paths. Written explicitly — no codegen.

use std::cmp::Ordering;

use app_lib::similarity::{fingerprint_distance, find_similar, AudioFingerprint};

fn smf_single_track(tempo_micros: u32, include_note_on: bool) -> Vec<u8> {
    let t0 = ((tempo_micros >> 16) & 0xff) as u8;
    let t1 = ((tempo_micros >> 8) & 0xff) as u8;
    let t2 = (tempo_micros & 0xff) as u8;

    let mut track = vec![
        0x00, 0xFF, 0x51, 0x03, t0, t1, t2,
    ];
    if include_note_on {
        track.extend_from_slice(&[0x00, 0x90, 0x3C, 0x40]);
    }
    track.extend_from_slice(&[0x00, 0xFF, 0x2F, 0x00]);

    let trk_len = track.len() as u32;
    let mut mid = vec![
        b'M', b'T', b'h', b'd', 0, 0, 0, 6, 0, 0, 0, 1, 0x01, 0xE0,
    ];
    mid.extend_from_slice(b"MTrk");
    mid.extend_from_slice(&trk_len.to_be_bytes());
    mid.extend_from_slice(&track);
    mid
}

#[test]
fn midi_parse_tempo_120_bpm_from_fixture() {
    let dir = std::env::temp_dir().join("audio_haxor_deep_midi_120");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("t120.mid");
    std::fs::write(&path, smf_single_track(500_000, false)).unwrap();
    let info = app_lib::midi::parse_midi(&path).expect("valid SMF");
    assert_eq!(info.format, 0);
    assert_eq!(info.track_count, 1);
    assert_eq!(info.ppqn, 480);
    assert!((info.tempo - 120.0).abs() < 0.05, "got {}", info.tempo);
    assert_eq!(info.note_count, 0);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn midi_parse_tempo_60_bpm_from_fixture() {
    let dir = std::env::temp_dir().join("audio_haxor_deep_midi_60");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("t60.mid");
    std::fs::write(&path, smf_single_track(1_000_000, false)).unwrap();
    let info = app_lib::midi::parse_midi(&path).expect("valid SMF");
    assert!((info.tempo - 60.0).abs() < 0.05, "got {}", info.tempo);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn midi_parse_counts_note_on_events() {
    let dir = std::env::temp_dir().join("audio_haxor_deep_midi_notes");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("notes.mid");
    std::fs::write(&path, smf_single_track(500_000, true)).unwrap();
    let info = app_lib::midi::parse_midi(&path).expect("valid SMF");
    assert_eq!(info.note_count, 1);
    assert!(info.channels_used >= 1);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn midi_parse_rejects_non_mthd_magic() {
    let dir = std::env::temp_dir().join(format!(
        "audio_haxor_deep_midi_bad_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("bad.mid");
    std::fs::write(&path, b"XXXX\x00\x00\x00\x06").expect("write invalid header fixture");
    assert!(app_lib::midi::parse_midi(&path).is_none());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn kvr_parse_version_empty_and_unknown_are_zero_triples() {
    assert_eq!(app_lib::kvr::parse_version(""), vec![0, 0, 0]);
    assert_eq!(app_lib::kvr::parse_version("Unknown"), vec![0, 0, 0]);
}

#[test]
fn kvr_compare_unknown_orders_before_one_point_zero() {
    assert_eq!(
        app_lib::kvr::compare_versions("Unknown", "1.0"),
        Ordering::Less
    );
}

#[test]
fn kvr_compare_leading_zeros_in_segments() {
    assert_eq!(
        app_lib::kvr::compare_versions("01.02.00", "1.2.0"),
        Ordering::Equal
    );
}

#[test]
fn kvr_compare_double_dot_parses_empty_segment_as_zero() {
    assert_eq!(
        app_lib::kvr::compare_versions("1..3", "1.0.2"),
        Ordering::Greater
    );
}

#[test]
fn kvr_extract_version_returns_none_for_versionless_html() {
    let html = "<html><body><p>No version here</p></body></html>";
    assert!(app_lib::kvr::extract_version(html).is_none());
}

#[test]
fn kvr_extract_download_url_accepts_buy_and_release_hrefs() {
    let buy = r#"<a href="https://vendor.example.com/buy/product">Buy</a>"#;
    let (u, _) = app_lib::kvr::extract_download_url(buy).expect("buy href");
    assert!(u.contains("buy"));

    let rel = r#"<a href="https://cdn.example.com/release/v2/installer">Rel</a>"#;
    let (u2, _) = app_lib::kvr::extract_download_url(rel).expect("release href");
    assert!(u2.contains("release"));
}

#[test]
fn kvr_url_re_extracts_http_url_until_delimiter() {
    let text = r#"See https://www.kvraudio.com/product/foo for details."#;
    let m = app_lib::kvr::URL_RE.find(text).expect("match");
    assert_eq!(m.as_str(), "https://www.kvraudio.com/product/foo");
}

#[test]
fn scanner_get_plugin_type_maps_known_extensions() {
    assert_eq!(app_lib::scanner::get_plugin_type(".vst"), "VST2");
    assert_eq!(app_lib::scanner::get_plugin_type(".vst3"), "VST3");
    assert_eq!(app_lib::scanner::get_plugin_type(".component"), "AU");
    assert_eq!(app_lib::scanner::get_plugin_type(".dll"), "VST2");
}

#[test]
fn scanner_get_plugin_type_unknown_extension() {
    assert_eq!(app_lib::scanner::get_plugin_type(".exe"), "Unknown");
    assert_eq!(app_lib::scanner::get_plugin_type(""), "Unknown");
}

#[test]
fn daw_name_for_format_covers_major_formats() {
    assert_eq!(app_lib::daw_scanner::daw_name_for_format("ALS"), "Ableton Live");
    assert_eq!(app_lib::daw_scanner::daw_name_for_format("LOGICX"), "Logic Pro");
    assert_eq!(app_lib::daw_scanner::daw_name_for_format("DAWPROJECT"), "DAWproject");
    assert_eq!(app_lib::daw_scanner::daw_name_for_format("RPP-BAK"), "REAPER");
    assert_eq!(app_lib::daw_scanner::daw_name_for_format("BAND"), "GarageBand");
}

#[test]
fn daw_name_for_format_unknown_is_unknown() {
    assert_eq!(app_lib::daw_scanner::daw_name_for_format("XYZ"), "Unknown");
}

fn fp(path: &str, rms: f64) -> AudioFingerprint {
    AudioFingerprint {
        path: path.into(),
        rms,
        spectral_centroid: 0.5,
        zero_crossing_rate: 0.1,
        low_band_energy: 0.25,
        mid_band_energy: 0.35,
        high_band_energy: 0.15,
        low_energy_ratio: 0.4,
        attack_time: 0.02,
    }
}

#[test]
fn fingerprint_distance_is_zero_for_identical_vectors_even_if_paths_differ() {
    let a = fp("/a.wav", 0.42);
    let b = fp("/totally/different/b.wav", 0.42);
    let d = fingerprint_distance(&a, &b);
    assert!(d < 1e-12, "distance {}", d);
}

#[test]
fn fingerprint_distance_increases_when_rms_differs() {
    let a = fp("/x.wav", 0.1);
    let b = fp("/x.wav", 0.9);
    let d_small = fingerprint_distance(&fp("/1.wav", 0.5), &fp("/2.wav", 0.51));
    let d_large = fingerprint_distance(&a, &b);
    assert!(d_large > d_small);
}

#[test]
fn find_similar_truncates_to_zero_when_max_results_zero() {
    let reference = fp("/ref.wav", 0.5);
    let candidates = vec![fp("/a.wav", 0.51), fp("/b.wav", 0.6)];
    assert!(find_similar(&reference, &candidates, 0).is_empty());
}

#[test]
fn find_similar_sorts_by_distance_ties_use_partial_cmp() {
    let reference = fp("/ref.wav", 0.5);
    let c0 = fp("/a.wav", 0.6);
    let c1 = fp("/b.wav", 0.6);
    let out = find_similar(&reference, &[c0, c1], 2);
    assert_eq!(out.len(), 2);
    assert!(out[0].1 <= out[1].1);
}

#[test]
fn get_audio_metadata_nonexistent_file_sets_error() {
    let p = "/this/path/does/not/exist/audio_haxor_missing_file.wav";
    let m = app_lib::audio_scanner::get_audio_metadata(p);
    assert!(m.error.is_some(), "expected error for missing path");
    assert_eq!(m.full_path, p);
    assert_eq!(m.size_bytes, 0);
}

#[test]
fn normalize_plugin_name_strips_bare_x64_suffix() {
    assert_eq!(app_lib::xref::normalize_plugin_name("Serum x64"), "serum");
}

#[test]
fn normalize_plugin_name_collapses_internal_whitespace() {
    assert_eq!(
        app_lib::xref::normalize_plugin_name("  Foo   Bar  "),
        "foo bar"
    );
}

#[test]
fn radix_string_binary_and_hex() {
    assert_eq!(app_lib::history::radix_string(5, 2), "101");
    assert_eq!(app_lib::history::radix_string(255, 16), "ff");
}

#[test]
fn radix_string_large_values_non_empty() {
    let s = app_lib::history::radix_string(u64::MAX, 36);
    assert!(!s.is_empty());
    assert!(s.chars().all(|c| c.is_ascii_alphanumeric()));
}

#[test]
fn format_size_one_kib() {
    assert_eq!(app_lib::format_size(1024), "1.0 KB");
}

#[test]
fn format_size_one_mib() {
    assert_eq!(app_lib::format_size(1024 * 1024), "1.0 MB");
}

#[test]
fn format_size_one_tib() {
    assert_eq!(app_lib::format_size(1024u64.pow(4)), "1.0 TB");
}

#[test]
fn bpm_estimate_bpm_nonexistent_returns_none() {
    assert!(app_lib::bpm::estimate_bpm("/no/such/audio_haxor/file.wav").is_none());
}

#[test]
fn bpm_read_wav_pcm_nonexistent_returns_none() {
    use std::path::Path;
    assert!(app_lib::bpm::read_wav_pcm_pub(Path::new("/no/such.wav")).is_none());
}

#[test]
fn midi_info_serializes_expected_keys() {
    let info = app_lib::midi::MidiInfo {
        format: 1,
        track_count: 4,
        ppqn: 480,
        tempo: 140.0,
        time_signature: "6/8".into(),
        key_signature: "D minor".into(),
        note_count: 42,
        duration: 12.5,
        track_names: vec!["Drums".into()],
        channels_used: 8,
    };
    let v = serde_json::to_value(&info).unwrap();
    assert_eq!(v["format"], 1);
    assert_eq!(v["trackCount"], 4);
    assert_eq!(v["timeSignature"], "6/8");
    assert_eq!(v["keySignature"], "D minor");
    assert_eq!(v["noteCount"], 42);
}

#[test]
fn scanner_format_size_matches_lib() {
    let b = 999_999u64;
    assert_eq!(app_lib::scanner::format_size(b), app_lib::format_size(b));
}

#[test]
fn audio_scanner_format_size_matches_lib() {
    let b = 3_221u64;
    assert_eq!(
        app_lib::audio_scanner::format_size(b),
        app_lib::format_size(b)
    );
}

#[test]
fn daw_scanner_format_size_matches_lib() {
    let b = 77_777u64;
    assert_eq!(
        app_lib::daw_scanner::format_size(b),
        app_lib::format_size(b)
    );
}
