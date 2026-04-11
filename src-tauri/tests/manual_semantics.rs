//! Hand-written behavioral tests only — no generated grids, no external scripts.
//! Covers ordering edges, snapshot builders, HTML fixtures, and similarity invariants.

use std::cmp::Ordering;
use std::path::Path;

use app_lib::history::{
    AudioSample, DawProject, PresetFile, build_audio_snapshot, build_daw_snapshot,
    build_preset_snapshot, compute_audio_diff, compute_daw_diff, compute_preset_diff,
};
use app_lib::similarity::{AudioFingerprint, find_similar, fingerprint_distance};

// ── KVR: `compare_versions` — explicit pairs (real semver-like strings) ──

#[test]
fn kvr_compare_patch_increments_less_than_next_minor() {
    assert_eq!(
        app_lib::kvr::compare_versions("1.0.9", "1.0.10"),
        Ordering::Less
    );
}

#[test]
fn kvr_compare_numeric_not_lexicographic_on_last_segment() {
    assert_eq!(
        app_lib::kvr::compare_versions("1.0.9", "1.0.10"),
        Ordering::Less
    );
    assert_eq!(
        app_lib::kvr::compare_versions("1.0.10", "1.0.9"),
        Ordering::Greater
    );
}

#[test]
fn kvr_compare_major_dominates_minor() {
    assert_eq!(
        app_lib::kvr::compare_versions("2.0.0", "1.99.99"),
        Ordering::Greater
    );
}

#[test]
fn kvr_compare_equal_with_trailing_zeros() {
    assert_eq!(
        app_lib::kvr::compare_versions("2.0", "2.0.0.0"),
        Ordering::Equal
    );
}

#[test]
fn kvr_compare_single_segment_vs_triple() {
    assert_eq!(app_lib::kvr::compare_versions("3", "3.0.1"), Ordering::Less);
}

#[test]
fn kvr_compare_four_segments() {
    assert_eq!(
        app_lib::kvr::compare_versions("1.0.0.1", "1.0.0.2"),
        Ordering::Less
    );
}

#[test]
fn kvr_compare_transitivity_sample() {
    let a = "1.5.0";
    let b = "1.6.0";
    let c = "1.7.0";
    assert_eq!(app_lib::kvr::compare_versions(a, b), Ordering::Less);
    assert_eq!(app_lib::kvr::compare_versions(b, c), Ordering::Less);
    assert_eq!(app_lib::kvr::compare_versions(a, c), Ordering::Less);
}

#[test]
fn kvr_parse_version_multidigit_segments() {
    assert_eq!(app_lib::kvr::parse_version("12.34.56"), vec![12, 34, 56]);
}

#[test]
fn kvr_parse_version_trailing_dot_yields_zero_patch() {
    // "1.0." → ["1","0",""] → [1, 0, 0]
    assert_eq!(app_lib::kvr::parse_version("1.0."), vec![1, 0, 0]);
}

#[test]
fn kvr_parse_version_leading_dot() {
    assert_eq!(app_lib::kvr::parse_version(".5.0"), vec![0, 5, 0]);
}

#[test]
fn kvr_extract_version_picks_label_before_dd_block() {
    let html = r#"<span>Version</span><dd>11.22.33</dd>"#;
    assert_eq!(
        app_lib::kvr::extract_version(html).as_deref(),
        Some("11.22.33")
    );
}

#[test]
fn kvr_extract_version_from_release_snippet() {
    let html = r#"Release notes: latest v2.4.0 is now available"#;
    assert_eq!(
        app_lib::kvr::extract_version(html).as_deref(),
        Some("2.4.0")
    );
}

#[test]
fn kvr_extract_download_prefers_first_matching_href_when_no_platform_match() {
    let html = r#"<a href="https://static.example.com/download/pkg">x</a>"#;
    let r = app_lib::kvr::extract_download_url(html);
    assert!(r.is_some());
    assert!(r.unwrap().0.contains("download"));
}

// ── History: `build_*_snapshot` aggregates bytes and format counts ────────

fn sample_audio(path: &str, format: &str, bytes: u64) -> AudioSample {
    AudioSample {
        name: "a".into(),
        path: path.into(),
        directory: "/d".into(),
        format: format.into(),
        size: bytes,
        size_formatted: "x".into(),
        modified: "t".into(),
        duration: None,
        channels: None,
        sample_rate: None,
        bits_per_sample: None,
    }
}

#[test]
fn build_audio_snapshot_counts_formats_and_total_bytes() {
    let s = vec![
        sample_audio("/a.wav", "wav", 100),
        sample_audio("/b.wav", "wav", 300),
        sample_audio("/c.flac", "flac", 50),
    ];
    let snap = build_audio_snapshot(&s, &["/root".into()]);
    assert_eq!(snap.sample_count, 3);
    assert_eq!(snap.total_bytes, 450);
    assert_eq!(snap.format_counts.get("wav").copied(), Some(2));
    assert_eq!(snap.format_counts.get("flac").copied(), Some(1));
    assert_eq!(snap.roots, vec!["/root".to_string()]);
}

fn proj(path: &str, daw: &str, sz: u64) -> DawProject {
    DawProject {
        name: "p".into(),
        path: path.into(),
        directory: "/d".into(),
        format: "als".into(),
        daw: daw.into(),
        size: sz,
        size_formatted: "x".into(),
        modified: "t".into(),
    }
}

#[test]
fn build_daw_snapshot_groups_daw_counts() {
    let p = vec![
        proj("/a.als", "ALS", 10),
        proj("/b.als", "ALS", 10),
        proj("/c.flp", "FLP", 5),
    ];
    let snap = build_daw_snapshot(&p, &[]);
    assert_eq!(snap.project_count, 3);
    assert_eq!(snap.total_bytes, 25);
    assert_eq!(snap.daw_counts.get("ALS").copied(), Some(2));
    assert_eq!(snap.daw_counts.get("FLP").copied(), Some(1));
}

fn preset(path: &str, fmt: &str, sz: u64) -> PresetFile {
    PresetFile {
        name: "n".into(),
        path: path.into(),
        directory: "/d".into(),
        format: fmt.into(),
        size: sz,
        size_formatted: "x".into(),
        modified: "t".into(),
    }
}

#[test]
fn build_preset_snapshot_counts_by_format() {
    let pr = vec![preset("/a.fxp", "fxp", 1), preset("/b.nmsv", "nmsv", 2)];
    let snap = build_preset_snapshot(&pr, &["/p".into()]);
    assert_eq!(snap.preset_count, 2);
    assert_eq!(snap.total_bytes, 3);
    assert_eq!(snap.format_counts.get("fxp").copied(), Some(1));
    assert_eq!(snap.format_counts.get("nmsv").copied(), Some(1));
}

#[test]
fn compute_audio_diff_both_empty_no_added_no_removed() {
    let old = build_audio_snapshot(&[], &[]);
    let new = build_audio_snapshot(&[], &[]);
    let d = compute_audio_diff(&old, &new);
    assert!(d.added.is_empty());
    assert!(d.removed.is_empty());
}

#[test]
fn compute_daw_diff_identical_snapshots_empty_diff() {
    let p = vec![proj("/x.als", "ALS", 1)];
    let a = build_daw_snapshot(&p, &[]);
    let b = build_daw_snapshot(&p, &[]);
    let d = compute_daw_diff(&a, &b);
    assert!(d.added.is_empty());
    assert!(d.removed.is_empty());
}

#[test]
fn compute_preset_diff_all_new_paths() {
    let old = build_preset_snapshot(&[], &[]);
    let new = build_preset_snapshot(&[preset("/only.fxp", "fxp", 1)], &[]);
    let d = compute_preset_diff(&old, &new);
    assert_eq!(d.added.len(), 1);
    assert_eq!(d.removed.len(), 0);
}

// ── Similarity: distance properties on constructed fingerprints ───────────

fn mk_fp(path: &str, rms: f64, zcr: f64) -> AudioFingerprint {
    AudioFingerprint {
        path: path.into(),
        rms,
        spectral_centroid: 0.4,
        zero_crossing_rate: zcr,
        low_band_energy: 0.2,
        mid_band_energy: 0.3,
        high_band_energy: 0.2,
        low_energy_ratio: 0.5,
        attack_time: 0.015,
    }
}

#[test]
fn fingerprint_distance_symmetric_for_asymmetric_paths() {
    let a = mk_fp("/left.wav", 0.3, 0.1);
    let b = mk_fp("/right.wav", 0.7, 0.05);
    let d1 = fingerprint_distance(&a, &b);
    let d2 = fingerprint_distance(&b, &a);
    assert!((d1 - d2).abs() < 1e-12);
}

#[test]
fn find_similar_orders_nearest_rms_first() {
    let reference = mk_fp("/ref.wav", 0.5, 0.1);
    let candidates = vec![
        mk_fp("/far.wav", 0.99, 0.1),
        mk_fp("/near.wav", 0.52, 0.1),
        mk_fp("/mid.wav", 0.7, 0.1),
    ];
    let out = find_similar(&reference, &candidates, 3);
    assert_eq!(out[0].0, "/near.wav");
}

#[test]
fn fingerprint_self_distance_zero() {
    let fp = mk_fp("/self.wav", 0.41, 0.02);
    assert!(fingerprint_distance(&fp, &fp) < 1e-12);
}

// ── `format_size` — hand-picked byte values ──────────────────────────────

#[test]
fn format_size_one_gib() {
    assert_eq!(app_lib::format_size(1024u64.pow(3)), "1.0 GB");
}

#[test]
fn format_size_sub_kib_shows_bytes() {
    assert_eq!(app_lib::format_size(512), "512.0 B");
}

#[test]
fn format_size_large_but_not_max_unit() {
    let b = 3 * 1024u64.pow(3) + 100;
    let s = app_lib::format_size(b);
    assert!(s.contains("GB") || s.contains("MB"));
}

// ── DAW scanner — path shape edge cases ─────────────────────────────────

#[test]
fn daw_ext_matches_case_insensitive_suffix() {
    assert_eq!(
        app_lib::daw_scanner::ext_matches(Path::new("Song.PTX")).as_deref(),
        Some("PTX")
    );
}

#[test]
fn daw_ext_matches_nested_suffix_rpp_bak() {
    assert_eq!(
        app_lib::daw_scanner::ext_matches(Path::new("backup/project.rpp-bak")).as_deref(),
        Some("RPP-BAK")
    );
}

#[test]
fn daw_is_package_ext_rejects_plain_als_file() {
    assert!(!app_lib::daw_scanner::is_package_ext(Path::new("live.als")));
}

// ── xref — normalization preserves semantic core ──────────────────────────

#[test]
fn xref_normalize_strips_vst3_then_x64() {
    assert_eq!(
        app_lib::xref::normalize_plugin_name("Pro-Q 3 (VST3) (x64)"),
        "pro-q 3"
    );
}

#[test]
fn xref_normalize_apple_silicon_bracket() {
    let s = app_lib::xref::normalize_plugin_name("Synth (Apple Silicon)");
    assert!(s.contains("synth"));
    assert!(!s.to_lowercase().contains("apple silicon"));
}

#[test]
fn xref_normalize_emptyish_suffix_falls_back() {
    let n = app_lib::xref::normalize_plugin_name(" (x64)");
    assert!(!n.is_empty());
}

// ── BPM / MIDI — failure modes without temp scripts ───────────────────────

#[test]
fn bpm_read_aiff_missing_returns_none() {
    use std::path::Path;
    assert!(app_lib::bpm::read_aiff_pcm_pub(Path::new("/no/such.aif")).is_none());
}

#[test]
fn midi_parse_short_file_returns_none() {
    let dir = std::env::temp_dir().join("audio_haxor_midi_short");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let p = dir.join("tiny.mid");
    std::fs::write(&p, [0x4Du8, 0x54, 0x68, 0x64]).unwrap();
    assert!(app_lib::midi::parse_midi(&p).is_none());
    let _ = std::fs::remove_dir_all(&dir);
}

// ── Scanner — plugin type remains stable for bundle types ────────────────

#[test]
fn scanner_plugin_type_vst3_lowercase_ext() {
    assert_eq!(app_lib::scanner::get_plugin_type(".VST3"), "Unknown");
}

#[test]
fn scanner_plugin_type_requires_leading_dot() {
    assert_eq!(app_lib::scanner::get_plugin_type("vst3"), "Unknown");
}
