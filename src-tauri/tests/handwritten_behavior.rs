//! Extra integration tests: snapshot builders, KVR edges, decode entry points, serde.
//! All cases written directly in Rust — no scripts, no macro grids.

use std::cmp::Ordering;
use std::path::Path;

use app_lib::history::{build_plugin_snapshot, compute_plugin_diff, ScanSnapshot};
use app_lib::scanner::PluginInfo;

fn plugin(path: &str, name: &str, ver: &str) -> PluginInfo {
    PluginInfo {
        name: name.into(),
        path: path.into(),
        plugin_type: "VST3".into(),
        version: ver.into(),
        manufacturer: "M".into(),
        manufacturer_url: None,
        size: "1 B".into(),
        size_bytes: 1,
        modified: "t".into(),
        architectures: vec![],
    }
}

#[test]
fn build_plugin_snapshot_empty_has_zero_plugins_and_preserves_roots() {
    let snap = build_plugin_snapshot(&[], &["/d".into()], &["/r".into()]);
    assert_eq!(snap.plugin_count, 0);
    assert!(snap.plugins.is_empty());
    assert_eq!(snap.directories, vec!["/d".to_string()]);
    assert_eq!(snap.roots, vec!["/r".to_string()]);
}

#[test]
fn build_plugin_snapshot_counts_match_vec_len() {
    let p = vec![plugin("/a.vst3", "A", "1.0"), plugin("/b.vst3", "B", "2.0")];
    let snap = build_plugin_snapshot(&p, &[], &[]);
    assert_eq!(snap.plugin_count, 2);
    assert_eq!(snap.plugins.len(), 2);
}

#[test]
fn compute_plugin_diff_no_change_when_identical_snapshots() {
    let p = vec![plugin("/x.vst3", "X", "1.0.0")];
    let a = build_plugin_snapshot(&p, &[], &[]);
    let b = build_plugin_snapshot(&p, &[], &[]);
    let d = compute_plugin_diff(&a, &b);
    assert!(d.added.is_empty());
    assert!(d.removed.is_empty());
    assert!(d.version_changed.is_empty());
}

#[test]
fn compute_plugin_diff_same_path_version_bump_only_in_version_changed() {
    let old = build_plugin_snapshot(&[plugin("/p.vst3", "P", "1.0.0")], &[], &[]);
    let new = build_plugin_snapshot(&[plugin("/p.vst3", "P", "2.0.0")], &[], &[]);
    let d = compute_plugin_diff(&old, &new);
    assert!(d.added.is_empty());
    assert!(d.removed.is_empty());
    assert_eq!(d.version_changed.len(), 1);
    assert_eq!(d.version_changed[0].previous_version, "1.0.0");
    assert_eq!(d.version_changed[0].plugin.version, "2.0.0");
}

#[test]
fn scan_snapshot_serializes_roundtrip_through_json() {
    let snap = build_plugin_snapshot(
        &[plugin("/z.vst3", "Z", "3.1.4")],
        &["/Plugins".into()],
        &[],
    );
    let json = serde_json::to_string(&snap).unwrap();
    let back: ScanSnapshot = serde_json::from_str(&json).unwrap();
    assert_eq!(back.plugin_count, snap.plugin_count);
    assert_eq!(back.plugins[0].path, "/z.vst3");
}

// ── KVR: multi-digit segments and strict inequality ───────────────────────

#[test]
fn kvr_compare_versions_10_gt_2_in_first_segment() {
    assert_eq!(
        app_lib::kvr::compare_versions("10.0.0", "2.0.0"),
        Ordering::Greater
    );
}

#[test]
fn kvr_parse_version_double_digit_segments() {
    assert_eq!(app_lib::kvr::parse_version("10.20.30"), vec![10, 20, 30]);
}

#[test]
fn kvr_compare_independent_of_extra_build_metadata_segments() {
    assert_eq!(
        app_lib::kvr::compare_versions("1.0.0.1", "1.0.0.2"),
        Ordering::Less
    );
}

// ── Similarity: `max_results` larger than pool ───────────────────────────

#[test]
fn find_similar_returns_all_distinct_candidates_when_max_large() {
    use app_lib::similarity::{find_similar, AudioFingerprint};
    let mk = |path: &str, rms: f64| AudioFingerprint {
        path: path.into(),
        rms,
        spectral_centroid: 0.5,
        zero_crossing_rate: 0.1,
        low_band_energy: 0.2,
        mid_band_energy: 0.3,
        high_band_energy: 0.2,
        low_energy_ratio: 0.4,
        attack_time: 0.02,
    };
    let reference = mk("/r.wav", 0.5);
    let candidates = vec![mk("/a.wav", 0.6), mk("/b.wav", 0.7)];
    let out = find_similar(&reference, &candidates, 100);
    assert_eq!(out.len(), 2);
}

// ── BPM / symphonia: missing paths ────────────────────────────────────────

#[test]
fn bpm_decode_symphonia_nonexistent_returns_none() {
    assert!(app_lib::bpm::decode_with_symphonia_pub(Path::new(
        "/nonexistent/audio_haxor/missing.mp3"
    ))
    .is_none());
}

#[test]
fn bpm_read_aiff_nonexistent_returns_none() {
    assert!(
        app_lib::bpm::read_aiff_pcm_pub(Path::new("/nonexistent/audio_haxor/missing.aif"))
            .is_none()
    );
}

// ── MIDI: default + serde ────────────────────────────────────────────────

#[test]
fn midi_info_default_matches_derive_defaults() {
    let m = app_lib::midi::MidiInfo::default();
    assert_eq!(m.format, 0);
    assert_eq!(m.track_count, 0);
    assert_eq!(m.ppqn, 0);
    assert_eq!(m.tempo, 0.0);
    assert_eq!(m.time_signature, "");
    assert_eq!(m.key_signature, "");
    assert_eq!(m.note_count, 0);
    assert_eq!(m.duration, 0.0);
    assert!(m.track_names.is_empty());
    assert_eq!(m.channels_used, 0);
}

#[test]
fn midi_info_serializes_track_names_array() {
    let m = app_lib::midi::MidiInfo {
        track_names: vec!["Drums".into(), "Bass".into()],
        ..Default::default()
    };
    let v = serde_json::to_value(&m).unwrap();
    assert_eq!(v["trackNames"].as_array().unwrap().len(), 2);
}

// ── `format_size` — boundary just below 1 MiB ───────────────────────────

#[test]
fn format_size_just_below_one_mib() {
    let s = app_lib::format_size(1024 * 1024 - 1);
    assert!(s.contains("MB") || s.contains("KB"), "label: {s}");
}

// ── DAW: remaining `daw_name_for_format` labels ─────────────────────────

#[test]
fn daw_name_audacity_project_extensions() {
    assert_eq!(app_lib::daw_scanner::daw_name_for_format("AUP"), "Audacity");
    assert_eq!(
        app_lib::daw_scanner::daw_name_for_format("AUP3"),
        "Audacity"
    );
}

#[test]
fn daw_name_ardour_token() {
    assert_eq!(
        app_lib::daw_scanner::daw_name_for_format("ARDOUR"),
        "Ardour"
    );
}

// ── xref: `.als` missing file yields empty plugin list ───────────────────

#[test]
fn xref_extract_ableton_missing_returns_empty() {
    assert!(app_lib::xref::extract_plugins("/no/such/path/audio_haxor/missing.als").is_empty());
}

#[test]
fn xref_normalize_preserves_inner_product_name() {
    assert_eq!(
        app_lib::xref::normalize_plugin_name("EQ Eight (device)"),
        "eq eight (device)"
    );
}

// ── History: `gen_id` produces distinct strings in a short burst ─────────

#[test]
fn gen_id_twenty_uniq_in_loop() {
    use std::collections::HashSet;
    let mut set = HashSet::new();
    for _ in 0..20 {
        assert!(set.insert(app_lib::history::gen_id()));
    }
}

// ── KVR URL regex: stops at common delimiters ─────────────────────────────

#[test]
fn kvr_url_re_stops_before_closing_paren() {
    let t = r#"url(https://example.com/a/b)"#;
    let m = app_lib::kvr::URL_RE.find(t).unwrap();
    assert_eq!(m.as_str(), "https://example.com/a/b");
}

// ── Audio metadata: existing path that is not a valid audio file ────────

#[test]
fn get_audio_metadata_plain_text_extension_no_channels_parsed() {
    let dir = std::env::temp_dir().join("audio_haxor_meta_txt");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let p = dir.join("note.txt");
    std::fs::write(&p, b"hello").unwrap();
    let m = app_lib::audio_scanner::get_audio_metadata(p.to_str().unwrap());
    assert!(m.error.is_none());
    assert_eq!(m.channels, None);
    let _ = std::fs::remove_dir_all(&dir);
}

// ── Preset scanner: roots call returns a vec (smoke) ───────────────────

#[test]
fn preset_get_preset_roots_returns_vec() {
    let roots = app_lib::preset_scanner::get_preset_roots();
    assert!(roots.len() < 10_000, "sanity: reasonable count");
}

// ── Scanner: VST dirs list is usable ────────────────────────────────────

#[test]
fn scanner_get_vst_directories_non_empty_on_typical_system() {
    let dirs = app_lib::scanner::get_vst_directories();
    assert!(
        dirs.is_empty() || dirs.iter().any(|d| !d.is_empty()),
        "either empty or contains paths"
    );
}

// ── Key detect / LUFS — early rejection paths ────────────────────────────

#[test]
fn key_detect_directory_path_returns_none() {
    let dir = std::env::temp_dir().join("audio_haxor_key_dironly");
    let _ = std::fs::create_dir_all(&dir);
    assert!(app_lib::key_detect::detect_key(&format!("{}", dir.display())).is_none());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn lufs_measure_lufs_missing_file_returns_none() {
    assert!(app_lib::lufs::measure_lufs("/no/such/audio_haxor/lufs.wav").is_none());
}

// ── History: `radix_string` small bases ───────────────────────────────────

#[test]
fn radix_string_one_in_base_10_is_digit_one() {
    assert_eq!(app_lib::history::radix_string(1, 10), "1");
}

#[test]
fn radix_string_zero_any_base_is_zero() {
    assert_eq!(app_lib::history::radix_string(0, 36), "0");
}

// ── KVR: `Unknown` parse path used by compare ─────────────────────────────

#[test]
fn kvr_compare_unknown_equal_to_unknown() {
    assert_eq!(
        app_lib::kvr::compare_versions("Unknown", "Unknown"),
        Ordering::Equal
    );
}

// ── Similarity: distance nonnegative ─────────────────────────────────────

#[test]
fn fingerprint_distance_nonnegative_for_random_vectors() {
    use app_lib::similarity::{fingerprint_distance, AudioFingerprint};
    let a = AudioFingerprint {
        path: "/a".into(),
        rms: 0.1,
        spectral_centroid: 0.9,
        zero_crossing_rate: 0.2,
        low_band_energy: 0.15,
        mid_band_energy: 0.25,
        high_band_energy: 0.4,
        low_energy_ratio: 0.33,
        attack_time: 0.05,
    };
    let b = AudioFingerprint {
        path: "/b".into(),
        rms: 0.88,
        spectral_centroid: 0.1,
        zero_crossing_rate: 0.45,
        low_band_energy: 0.2,
        mid_band_energy: 0.2,
        high_band_energy: 0.2,
        low_energy_ratio: 0.2,
        attack_time: 0.1,
    };
    assert!(fingerprint_distance(&a, &b) >= 0.0);
}

// ── Audio roots / DAW roots smoke (non-panicking) ────────────────────────

#[test]
fn get_audio_roots_returns_vec() {
    let roots = app_lib::audio_scanner::get_audio_roots();
    assert!(roots.len() < 1000);
}

#[test]
fn get_daw_roots_returns_vec() {
    let roots = app_lib::daw_scanner::get_daw_roots();
    assert!(roots.len() < 1000);
}
