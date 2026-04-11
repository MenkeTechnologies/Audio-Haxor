//! Focused behavioral tests: explicit scenarios, no mass-generated grids.
//!
//! Covers diff logic, KVR parsing edge cases, similarity boundaries, and xref
//! behavior on missing files — complementary to `api_invariants` and table suites.

use std::cmp::Ordering;

use app_lib::history::{
    AudioSample, AudioScanDiff, AudioScanSummary, DawProject, DawScanDiff, DawScanSummary,
    KvrCacheEntry, KvrCacheUpdateEntry, PresetFile, PresetScanDiff, PresetScanSummary, ScanDiff,
    ScanSnapshot, ScanSummary, VersionChangedPlugin, build_audio_snapshot, build_daw_snapshot,
    build_plugin_snapshot, build_preset_snapshot, compute_audio_diff, compute_daw_diff,
    compute_plugin_diff, compute_preset_diff, radix_string,
};
use app_lib::scanner::PluginInfo;
use app_lib::similarity::{AudioFingerprint, find_similar, fingerprint_distance};
use app_lib::xref::{PluginRef, extract_plugins};
use app_lib::{ExportPayload, ExportPlugin};

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
    let new = build_plugin_snapshot(std::slice::from_ref(&p), &[], &[]);
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
    let full = build_plugin_snapshot(std::slice::from_ref(&p), &[], &[]);
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
    let old = build_daw_snapshot(&[sample_daw("/a.als", "A", "Ableton Live")], &["/r".into()]);
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
    assert!(
        !once.contains('('),
        "normalized name should drop arch parens: {once}"
    );
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
    let empty = build_audio_snapshot(&[], &[]);
    let one = build_audio_snapshot(&[sample_audio("/x.wav")], &[]);
    let d = compute_audio_diff(&empty, &one);
    assert_eq!(d.added.len(), 1);
    assert!(d.removed.is_empty());
}

#[test]
fn compute_preset_diff_empty_to_one() {
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

#[test]
fn kvr_parse_version_leading_zero_segments_are_numeric() {
    assert_eq!(app_lib::kvr::parse_version("01.02.03"), vec![1, 2, 3]);
}

#[test]
fn kvr_compare_versions_treats_leading_zeros_as_values() {
    assert_eq!(
        app_lib::kvr::compare_versions("01.0", "1.0"),
        Ordering::Equal
    );
}

#[test]
fn kvr_extract_version_plain_version_colon_line() {
    let html = "<html><body>Version: 9.8.7</body></html>";
    assert_eq!(
        app_lib::kvr::extract_version(html).as_deref(),
        Some("9.8.7")
    );
}

#[test]
fn kvr_extract_download_url_returns_none_without_candidate_links() {
    assert!(
        app_lib::kvr::extract_download_url("<html><body>no links here</body></html>").is_none()
    );
}

#[test]
fn kvr_update_result_json_roundtrip_keeps_rename_fields() {
    let u = app_lib::kvr::UpdateResult {
        latest_version: "2.1.0".into(),
        has_update: true,
        source: "kvr".into(),
        update_url: Some("https://example.com/dl".into()),
        kvr_url: Some("https://kvraudio.com/p/x".into()),
        has_platform_download: true,
    };
    let json = serde_json::to_value(&u).expect("serialize");
    assert!(json.get("latestVersion").is_some());
    assert!(json.get("hasUpdate").is_some());
    assert!(json.get("hasPlatformDownload").is_some());
    let back: app_lib::kvr::UpdateResult = serde_json::from_value(json).expect("deserialize");
    assert_eq!(back.latest_version, u.latest_version);
    assert_eq!(back.has_platform_download, u.has_platform_download);
}

#[test]
fn export_plugin_json_uses_type_key_for_plugin_type() {
    let ep = ExportPlugin {
        name: "N".into(),
        plugin_type: "VST3".into(),
        version: "1".into(),
        manufacturer: "M".into(),
        manufacturer_url: None,
        path: "/p".into(),
        size: "1 B".into(),
        size_bytes: 1,
        modified: "t".into(),
        architectures: vec![],
    };
    let v = serde_json::to_value(&ep).expect("to_value");
    assert_eq!(v.get("type"), Some(&serde_json::json!("VST3")));
    assert!(v.get("plugin_type").is_none());
    let back: ExportPlugin = serde_json::from_value(v).expect("from_value");
    assert_eq!(back.plugin_type, "VST3");
}

#[test]
fn export_payload_roundtrip_preserves_plugins_array() {
    let payload = ExportPayload {
        version: "1.0.0".into(),
        exported_at: "2020-01-01T00:00:00Z".into(),
        plugins: vec![ExportPlugin {
            name: "A".into(),
            plugin_type: "AU".into(),
            version: "3".into(),
            manufacturer: "Co".into(),
            manufacturer_url: Some("https://co".into()),
            path: "/a.component".into(),
            size: "2 B".into(),
            size_bytes: 2,
            modified: "m".into(),
            architectures: vec!["arm64".into()],
        }],
    };
    let json = serde_json::to_string(&payload).expect("serialize");
    let back: ExportPayload = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(back.plugins.len(), 1);
    assert_eq!(back.plugins[0].name, "A");
    assert_eq!(back.plugins[0].architectures, vec!["arm64"]);
}

#[test]
fn xref_plugin_ref_json_roundtrip_normalized_name_camel_case() {
    let p = PluginRef {
        name: "Foo (x64)".into(),
        normalized_name: "foo".into(),
        manufacturer: "Bar".into(),
        plugin_type: "VST3".into(),
    };
    let v = serde_json::to_value(&p).expect("to_value");
    assert!(v.get("normalizedName").is_some());
    assert!(v.get("pluginType").is_some());
    let back: PluginRef = serde_json::from_value(v).expect("from_value");
    assert_eq!(back, p);
}

#[test]
fn kvr_cache_entry_json_roundtrip() {
    let e = KvrCacheEntry {
        kvr_url: Some("https://k".into()),
        update_url: None,
        latest_version: Some("1.2".into()),
        has_update: false,
        source: "kvraudio".into(),
        timestamp: "ts".into(),
    };
    let json = serde_json::to_string(&e).expect("serialize");
    let back: KvrCacheEntry = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(back.kvr_url, e.kvr_url);
    assert_eq!(back.latest_version, e.latest_version);
}

#[test]
fn scan_diff_json_roundtrip_version_changed_shape() {
    let p = sample_plugin("/x.vst3", "2.0");
    let diff = ScanDiff {
        old_scan: ScanSummary {
            id: "old".into(),
            timestamp: "t1".into(),
            plugin_count: 1,
            roots: vec![],
        },
        new_scan: ScanSummary {
            id: "new".into(),
            timestamp: "t2".into(),
            plugin_count: 1,
            roots: vec![],
        },
        added: vec![],
        removed: vec![],
        version_changed: vec![VersionChangedPlugin {
            plugin: p.clone(),
            previous_version: "1.0".into(),
        }],
    };
    let v = serde_json::to_value(&diff).expect("to_value");
    assert!(v.get("oldScan").is_some());
    assert!(v.get("versionChanged").is_some());
    let back: ScanDiff = serde_json::from_value(v).expect("from_value");
    assert_eq!(back.version_changed[0].previous_version, "1.0");
    assert_eq!(back.version_changed[0].plugin.path, p.path);
}

#[test]
fn compute_plugin_diff_identical_snapshots_no_delta() {
    let snap = build_plugin_snapshot(&[sample_plugin("/a.vst3", "1.0")], &[], &[]);
    let d = compute_plugin_diff(&snap, &snap);
    assert!(d.added.is_empty());
    assert!(d.removed.is_empty());
    assert!(d.version_changed.is_empty());
}

#[test]
fn compute_daw_diff_identical_snapshots_no_delta() {
    let projects = vec![sample_daw("/p.als", "P", "Ableton Live")];
    let snap = build_daw_snapshot(&projects, &["/r".into()]);
    let d = compute_daw_diff(&snap, &snap);
    assert!(d.added.is_empty());
    assert!(d.removed.is_empty());
}

#[test]
fn compute_audio_diff_identical_snapshots_no_delta() {
    let samples = vec![sample_audio("/s.wav")];
    let snap = build_audio_snapshot(&samples, &[]);
    let d = compute_audio_diff(&snap, &snap);
    assert!(d.added.is_empty());
    assert!(d.removed.is_empty());
}

#[test]
fn compute_preset_diff_identical_snapshots_no_delta() {
    let pf = PresetFile {
        name: "n".into(),
        path: "/Presets/x.h2p".into(),
        directory: "/Presets".into(),
        format: "h2p".into(),
        size: 3,
        size_formatted: "3 B".into(),
        modified: "t".into(),
    };
    let snap = build_preset_snapshot(&[pf], &[]);
    let d = compute_preset_diff(&snap, &snap);
    assert!(d.added.is_empty());
    assert!(d.removed.is_empty());
}

#[test]
fn kvr_compare_versions_negative_numeric_components_order_correctly() {
    assert_eq!(
        app_lib::kvr::compare_versions("-1.0", "0.0"),
        Ordering::Less
    );
}

#[test]
fn kvr_result_json_roundtrip_product_and_download_urls() {
    let k = app_lib::kvr::KvrResult {
        product_url: "https://www.kvraudio.com/product/foo".into(),
        download_url: Some("https://vendor.com/get".into()),
    };
    let v = serde_json::to_value(&k).expect("serialize");
    assert!(v.get("productUrl").is_some());
    assert!(v.get("downloadUrl").is_some());
    let back: app_lib::kvr::KvrResult = serde_json::from_value(v).expect("deserialize");
    assert_eq!(back.product_url, k.product_url);
    assert_eq!(back.download_url, k.download_url);
}

#[test]
fn kvr_url_re_finds_first_http_url_in_text() {
    let text = r#"see https://example.com/path?q=1 and more"#;
    let m = app_lib::kvr::URL_RE.find(text).expect("match");
    assert!(m.as_str().starts_with("https://example.com/"));
}

#[test]
fn kvr_extract_version_returns_none_when_only_year_like_semver() {
    let html = "<html><body><p>Version: 2024.12.01</p></body></html>";
    assert!(app_lib::kvr::extract_version(html).is_none());
}

#[test]
fn kvr_extract_version_first_version_label_in_document_wins() {
    // Patterns match left-to-right; first successful non-date capture is returned.
    let html = concat!(
        "<html><body>",
        "<p>Version: 3.14.1</p>",
        "<p>Version: 2024.01.01</p>",
        "</body></html>"
    );
    assert_eq!(
        app_lib::kvr::extract_version(html).as_deref(),
        Some("3.14.1")
    );
}

#[test]
fn kvr_parse_version_accepts_arbitrary_depth() {
    assert_eq!(
        app_lib::kvr::parse_version("1.2.3.4.5"),
        vec![1, 2, 3, 4, 5]
    );
}

#[test]
fn kvr_cache_update_entry_json_optional_fields() {
    let e = KvrCacheUpdateEntry {
        key: "my-plugin".into(),
        kvr_url: None,
        update_url: Some("https://u".into()),
        latest_version: Some("9.0".into()),
        has_update: Some(true),
        source: Some("kvr".into()),
    };
    let json = serde_json::to_string(&e).expect("serialize");
    let back: KvrCacheUpdateEntry = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(back.key, e.key);
    assert_eq!(back.latest_version, e.latest_version);
}

#[test]
fn scanner_plugin_info_json_roundtrip_type_and_size_bytes_keys() {
    let p = PluginInfo {
        name: "N".into(),
        path: "/a.vst3".into(),
        plugin_type: "VST3".into(),
        version: "2".into(),
        manufacturer: "M".into(),
        manufacturer_url: None,
        size: "1 KB".into(),
        size_bytes: 1024,
        modified: "m".into(),
        architectures: vec!["x86_64".into()],
    };
    let v = serde_json::to_value(&p).expect("to_value");
    assert_eq!(v.get("type"), Some(&serde_json::json!("VST3")));
    assert_eq!(v.get("sizeBytes"), Some(&serde_json::json!(1024)));
    let back: PluginInfo = serde_json::from_value(v).expect("from_value");
    assert_eq!(back.path, p.path);
    assert_eq!(back.plugin_type, p.plugin_type);
    assert_eq!(back.size_bytes, p.size_bytes);
    assert_eq!(back.architectures, p.architectures);
}

#[test]
fn find_similar_excludes_all_candidates_sharing_reference_path() {
    let reference = AudioFingerprint {
        path: "/session/ref.wav".into(),
        rms: 0.5,
        spectral_centroid: 0.1,
        zero_crossing_rate: 0.1,
        low_band_energy: 0.2,
        mid_band_energy: 0.3,
        high_band_energy: 0.1,
        low_energy_ratio: 0.4,
        attack_time: 0.02,
    };
    let dup = AudioFingerprint {
        path: "/session/ref.wav".into(),
        rms: 0.9,
        spectral_centroid: 0.9,
        zero_crossing_rate: 0.9,
        low_band_energy: 0.9,
        mid_band_energy: 0.9,
        high_band_energy: 0.9,
        low_energy_ratio: 0.9,
        attack_time: 0.9,
    };
    let out = find_similar(&reference, &[dup], 10);
    assert!(out.is_empty(), "same path as reference must not be scored");
}

#[test]
fn audio_scan_diff_json_roundtrip_rename_keys() {
    let s = sample_audio("/a.wav");
    let diff = AudioScanDiff {
        old_scan: AudioScanSummary {
            id: "o".into(),
            timestamp: "t0".into(),
            sample_count: 0,
            total_bytes: 0,
            format_counts: std::collections::HashMap::new(),
            roots: vec![],
        },
        new_scan: AudioScanSummary {
            id: "n".into(),
            timestamp: "t1".into(),
            sample_count: 1,
            total_bytes: s.size,
            format_counts: [("WAV".into(), 1)].into_iter().collect(),
            roots: vec!["/r".into()],
        },
        added: vec![s.clone()],
        removed: vec![],
    };
    let v = serde_json::to_value(&diff).expect("to_value");
    assert!(v.get("oldScan").is_some());
    assert!(v.get("newScan").is_some());
    let back: AudioScanDiff = serde_json::from_value(v).expect("from_value");
    assert_eq!(back.added.len(), 1);
    assert_eq!(back.added[0].path, s.path);
}

#[test]
fn daw_scan_diff_json_roundtrip_rename_keys() {
    let dp = sample_daw("/proj.als", "Live", "Ableton Live");
    let diff = DawScanDiff {
        old_scan: DawScanSummary {
            id: "o".into(),
            timestamp: "t0".into(),
            project_count: 0,
            total_bytes: 0,
            daw_counts: std::collections::HashMap::new(),
            roots: vec![],
        },
        new_scan: DawScanSummary {
            id: "n".into(),
            timestamp: "t1".into(),
            project_count: 1,
            total_bytes: dp.size,
            daw_counts: [("Ableton Live".into(), 1)].into_iter().collect(),
            roots: vec![],
        },
        added: vec![dp],
        removed: vec![],
    };
    let v = serde_json::to_value(&diff).expect("to_value");
    assert!(v["oldScan"].get("dawCounts").is_some());
    let back: DawScanDiff = serde_json::from_value(v).expect("from_value");
    assert_eq!(back.added.len(), 1);
}

#[test]
fn preset_scan_diff_json_roundtrip_rename_keys() {
    let pf = PresetFile {
        name: "p".into(),
        path: "/Presets/a.h2p".into(),
        directory: "/Presets".into(),
        format: "h2p".into(),
        size: 8,
        size_formatted: "8 B".into(),
        modified: "m".into(),
    };
    let diff = PresetScanDiff {
        old_scan: PresetScanSummary {
            id: "o".into(),
            timestamp: "t0".into(),
            preset_count: 0,
            total_bytes: 0,
            format_counts: std::collections::HashMap::new(),
            roots: vec![],
        },
        new_scan: PresetScanSummary {
            id: "n".into(),
            timestamp: "t1".into(),
            preset_count: 1,
            total_bytes: pf.size,
            format_counts: [("h2p".into(), 1)].into_iter().collect(),
            roots: vec![],
        },
        added: vec![pf.clone()],
        removed: vec![],
    };
    let v = serde_json::to_value(&diff).expect("to_value");
    let back: PresetScanDiff = serde_json::from_value(v).expect("from_value");
    assert_eq!(back.new_scan.preset_count, 1);
    assert_eq!(back.added[0].path, pf.path);
}

#[test]
fn audio_scan_snapshot_json_keys_from_build_audio_snapshot() {
    let snap = build_audio_snapshot(&[sample_audio("/z.wav")], &["/root".into()]);
    let v: serde_json::Value = serde_json::to_value(&snap).expect("to_value");
    assert!(v.get("sampleCount").is_some());
    assert!(v.get("formatCounts").is_some());
    assert!(v.get("totalBytes").is_some());
}

#[test]
fn preset_scan_snapshot_json_keys_from_build_preset_snapshot() {
    let pf = PresetFile {
        name: "n".into(),
        path: "/p.h2p".into(),
        directory: "/".into(),
        format: "h2p".into(),
        size: 1,
        size_formatted: "1 B".into(),
        modified: "m".into(),
    };
    let snap = build_preset_snapshot(&[pf], &["/root".into()]);
    let v: serde_json::Value = serde_json::to_value(&snap).expect("to_value");
    assert!(v.get("presetCount").is_some());
    assert!(v.get("formatCounts").is_some());
}
