//! Focused behavioral tests: explicit scenarios, no mass-generated grids.
//!
//! Covers diff logic, KVR parsing edge cases, similarity boundaries, and xref
//! behavior on missing files — complementary to `api_invariants` and table suites.

use std::cmp::Ordering;

use app_lib::history::{
    build_daw_snapshot, build_plugin_snapshot, compute_daw_diff, compute_plugin_diff,
    radix_string, AudioSample, DawProject, ScanSnapshot,
};
use app_lib::scanner::PluginInfo;
use app_lib::similarity::{find_similar, fingerprint_distance, AudioFingerprint};
use app_lib::xref::extract_plugins;

fn sample_plugin(path: &str, version: &str) -> PluginInfo {
    PluginInfo {
        name: "P".into(),
        path: path.into(),
        plugin_type: "VST3".into(),
        version: version.into(),
        manufacturer: "M".into(),
        manufacturer_url: None,
        size: "1.0 KB".into(),
        size_bytes: 1024,
        modified: "t".into(),
        architectures: vec![],
    }
}

fn sample_daw(path: &str, name: &str, daw: &str) -> DawProject {
    DawProject {
        name: name.into(),
        path: path.into(),
        directory: "/d".into(),
        format: "ALS".into(),
        daw: daw.into(),
        size: 100,
        size_formatted: "100 B".into(),
        modified: "t".into(),
    }
}

fn sample_audio(path: &str) -> AudioSample {
    AudioSample {
        name: "a".into(),
        path: path.into(),
        directory: "/d".into(),
        format: "WAV".into(),
        size: 10,
        size_formatted: "10 B".into(),
        modified: "t".into(),
        duration: None,
        channels: None,
        sample_rate: None,
        bits_per_sample: None,
    }
}

#[test]
fn kvr_parse_version_unknown_and_empty_are_zero_triples() {
    assert_eq!(app_lib::kvr::parse_version("Unknown"), vec![0, 0, 0]);
    assert_eq!(app_lib::kvr::parse_version(""), vec![0, 0, 0]);
}

#[test]
fn kvr_parse_version_non_numeric_segment_becomes_zero() {
    assert_eq!(app_lib::kvr::parse_version("2.x.3"), vec![2, 0, 3]);
}

#[test]
fn kvr_compare_versions_pads_missing_components_with_zero() {
    assert_eq!(
        app_lib::kvr::compare_versions("1", "1.0.0"),
        Ordering::Equal
    );
    assert_eq!(
        app_lib::kvr::compare_versions("1.0", "1.0.0"),
        Ordering::Equal
    );
    assert_eq!(
        app_lib::kvr::compare_versions("2", "1.99.99"),
        Ordering::Greater
    );
}

#[test]
fn kvr_compare_versions_numeric_not_lexicographic() {
    assert_eq!(
        app_lib::kvr::compare_versions("2.0", "10.0"),
        Ordering::Less
    );
}

#[test]
fn radix_string_zero_and_single_digit_bases() {
    assert_eq!(radix_string(0, 16), "0");
    assert_eq!(radix_string(15, 16), "f");
    assert_eq!(radix_string(16, 16), "10");
    assert_eq!(radix_string(255, 16), "ff");
}

#[test]
fn radix_string_base_36_uses_lowercase_digits_and_letters() {
    assert_eq!(radix_string(35, 36), "z");
    assert_eq!(radix_string(36, 36), "10");
}

#[test]
fn build_plugin_snapshot_empty_has_zero_count_and_roots() {
    let snap = build_plugin_snapshot(&[], &["/a".into()], &["/root".into()]);
    assert_eq!(snap.plugin_count, 0);
    assert!(snap.plugins.is_empty());
    assert_eq!(snap.directories, vec!["/a"]);
    assert_eq!(snap.roots, vec!["/root"]);
    assert!(!snap.id.is_empty());
}

#[test]
fn compute_plugin_diff_empty_to_one_marks_added_only() {
    let old = build_plugin_snapshot(&[], &[], &[]);
    let p = sample_plugin("/Plugins/X.vst3", "1.0");
    let new = build_plugin_snapshot(&[p.clone()], &[], &[]);
    let d = compute_plugin_diff(&old, &new);
    assert_eq!(d.added.len(), 1);
    assert_eq!(d.added[0].path, p.path);
    assert!(d.removed.is_empty());
    assert!(d.version_changed.is_empty());
}

#[test]
fn compute_plugin_diff_version_change_requires_both_non_unknown() {
    let a = sample_plugin("/p/a.vst3", "Unknown");
    let b = sample_plugin("/p/a.vst3", "2.0");
    let old = build_plugin_snapshot(&[a], &[], &[]);
    let new = build_plugin_snapshot(&[b], &[], &[]);
    let d = compute_plugin_diff(&old, &new);
    assert!(
        d.version_changed.is_empty(),
        "transition from Unknown should not count as version change"
    );

    let v1 = sample_plugin("/p/b.vst3", "1.0");
    let v2 = sample_plugin("/p/b.vst3", "2.0");
    let old2 = build_plugin_snapshot(&[v1], &[], &[]);
    let new2 = build_plugin_snapshot(&[v2], &[], &[]);
    let d2 = compute_plugin_diff(&old2, &new2);
    assert_eq!(d2.version_changed.len(), 1);
    assert_eq!(d2.version_changed[0].previous_version, "1.0");
}

#[test]
fn compute_plugin_diff_swap_old_new_produces_added_removed_swap() {
    let p = sample_plugin("/only.vst3", "1.0");
    let full = build_plugin_snapshot(&[p.clone()], &[], &[]);
    let empty = build_plugin_snapshot(&[], &[], &[]);
    let d = compute_plugin_diff(&full, &empty);
    assert_eq!(d.removed.len(), 1);
    assert!(d.added.is_empty());
    let d2 = compute_plugin_diff(&empty, &full);
    assert_eq!(d2.added.len(), 1);
    assert!(d2.removed.is_empty());
}

#[test]
fn compute_daw_diff_detects_added_and_removed_paths() {
    let old = build_daw_snapshot(
        &[sample_daw("/a.als", "A", "Ableton Live")],
        &["/r".into()],
    );
    let new = build_daw_snapshot(
        &[
            sample_daw("/a.als", "A", "Ableton Live"),
            sample_daw("/b.als", "B", "Ableton Live"),
        ],
        &["/r".into()],
    );
    let d = compute_daw_diff(&old, &new);
    assert_eq!(d.added.len(), 1);
    assert_eq!(d.added[0].path, "/b.als");
    assert!(d.removed.is_empty());

    let d2 = compute_daw_diff(&new, &old);
    assert_eq!(d2.removed.len(), 1);
    assert_eq!(d2.removed[0].path, "/b.als");
}

#[test]
fn build_daw_snapshot_aggregates_counts_and_bytes() {
    let projects = vec![
        sample_daw("/1.als", "P1", "Ableton Live"),
        sample_daw("/2.als", "P2", "Ableton Live"),
    ];
    let snap = build_daw_snapshot(&projects, &["/home".into()]);
    assert_eq!(snap.project_count, 2);
    assert_eq!(snap.total_bytes, 200);
    assert_eq!(snap.daw_counts.get("Ableton Live").copied(), Some(2));
}

#[test]
fn find_similar_empty_candidates_returns_empty() {
    let reference = AudioFingerprint {
        path: "/ref.wav".into(),
        rms: 0.5,
        spectral_centroid: 0.1,
        zero_crossing_rate: 0.1,
        low_band_energy: 0.2,
        mid_band_energy: 0.3,
        high_band_energy: 0.1,
        low_energy_ratio: 0.4,
        attack_time: 0.02,
    };
    let out = find_similar(&reference, &[], 5);
    assert!(out.is_empty());
}

#[test]
fn find_similar_max_results_zero_truncates_to_empty() {
    let mk = |path: &str, rms: f64| AudioFingerprint {
        path: path.into(),
        rms,
        spectral_centroid: 0.1,
        zero_crossing_rate: 0.1,
        low_band_energy: 0.2,
        mid_band_energy: 0.3,
        high_band_energy: 0.1,
        low_energy_ratio: 0.4,
        attack_time: 0.02,
    };
    let reference = mk("/ref.wav", 0.5);
    let candidates = vec![mk("/a.wav", 0.6), mk("/b.wav", 0.7)];
    let out = find_similar(&reference, &candidates, 0);
    assert!(out.is_empty());
}

#[test]
fn fingerprint_distance_self_match_is_zero_for_identical_paths_allowed() {
    let fp = AudioFingerprint {
        path: "/same.wav".into(),
        rms: 0.4,
        spectral_centroid: 0.2,
        zero_crossing_rate: 0.05,
        low_band_energy: 0.1,
        mid_band_energy: 0.2,
        high_band_energy: 0.05,
        low_energy_ratio: 0.5,
        attack_time: 0.01,
    };
    let d = fingerprint_distance(&fp, &fp);
    assert!(d < 1e-9);
}

#[test]
fn xref_extract_plugins_missing_file_returns_empty() {
    assert!(extract_plugins("/no/such/path/project.flp").is_empty());
    assert!(extract_plugins("/no/such/file.als").is_empty());
}

#[test]
fn xref_normalize_plugin_name_strips_nested_suffixes() {
    let s = "MySynth (x64) (VST3) (AU)";
    let once = app_lib::xref::normalize_plugin_name(s);
    let twice = app_lib::xref::normalize_plugin_name(&once);
    assert_eq!(once, twice);
    assert!(!once.contains('('), "normalized name should drop arch parens: {once}");
}

#[test]
fn format_size_one_byte_and_kib_boundary() {
    assert_eq!(app_lib::format_size(0), "0 B");
    assert_eq!(app_lib::format_size(1), "1.0 B");
    let kb = app_lib::format_size(1024);
    assert!(
        kb.contains("KB"),
        "expected 1024 bytes to use KB label, got {kb}"
    );
}

#[test]
fn compute_audio_diff_empty_scans_from_history_helpers() {
    use app_lib::history::{build_audio_snapshot, compute_audio_diff};
    let empty = build_audio_snapshot(&[], &[]);
    let one = build_audio_snapshot(&[sample_audio("/x.wav")], &[]);
    let d = compute_audio_diff(&empty, &one);
    assert_eq!(d.added.len(), 1);
    assert!(d.removed.is_empty());
}

#[test]
fn compute_preset_diff_empty_to_one() {
    use app_lib::history::{build_preset_snapshot, compute_preset_diff, PresetFile};
    let pf = PresetFile {
        name: "p".into(),
        path: "/Presets/p.h2p".into(),
        directory: "/Presets".into(),
        format: "h2p".into(),
        size: 5,
        size_formatted: "5 B".into(),
        modified: "t".into(),
    };
    let old = build_preset_snapshot(&[], &[]);
    let new = build_preset_snapshot(&[pf], &[]);
    let d = compute_preset_diff(&old, &new);
    assert_eq!(d.added.len(), 1);
    assert!(d.removed.is_empty());
}

#[test]
fn scan_snapshot_serde_roundtrip_preserves_plugin_count() {
    let snap = build_plugin_snapshot(
        &[sample_plugin("/z.vst3", "3.2.1")],
        &["/d".into()],
        &["/r".into()],
    );
    let json = serde_json::to_string(&snap).expect("serialize");
    let back: ScanSnapshot = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(back.plugin_count, snap.plugin_count);
    assert_eq!(back.plugins[0].path, snap.plugins[0].path);
}

#[test]
fn daw_scan_snapshot_json_uses_expected_rename_keys() {
    let snap = build_daw_snapshot(&[sample_daw("/p.als", "Proj", "Ableton Live")], &[]);
    let v: serde_json::Value = serde_json::to_value(&snap).expect("to_value");
    assert!(v.get("projectCount").is_some());
    assert!(v.get("totalBytes").is_some());
    assert!(v.get("dawCounts").is_some());
}

#[test]
fn plugin_diff_preserves_scan_summaries_ids() {
    let old = build_plugin_snapshot(&[], &[], &[]);
    let new = build_plugin_snapshot(&[sample_plugin("/only.vst3", "1")], &[], &[]);
    let d = compute_plugin_diff(&old, &new);
    assert_eq!(d.old_scan.id, old.id);
    assert_eq!(d.new_scan.id, new.id);
}

#[test]
fn radix_string_large_value_is_stable_representation() {
    let s = radix_string(1_000_000, 10);
    assert_eq!(s, "1000000");
    let hex = radix_string(4_294_967_295, 16);
    assert_eq!(hex, "ffffffff");
}
