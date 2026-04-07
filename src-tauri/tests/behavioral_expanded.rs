//! Additional focused integration tests (explicit scenarios, not generated grids).

use std::cmp::Ordering;
use std::path::Path;

use app_lib::daw_scanner::{daw_name_for_format, ext_matches, is_package_ext};
use app_lib::history::{build_plugin_snapshot, compute_plugin_diff};
use app_lib::scanner::{get_plugin_type, PluginInfo};
use app_lib::similarity::{find_similar, fingerprint_distance, AudioFingerprint};
use app_lib::xref::normalize_plugin_name;

fn fp(path: &str, rms: f64, sc: f64) -> AudioFingerprint {
    AudioFingerprint {
        path: path.into(),
        rms,
        spectral_centroid: sc,
        zero_crossing_rate: 0.05,
        low_band_energy: 0.1,
        mid_band_energy: 0.2,
        high_band_energy: 0.05,
        low_energy_ratio: 0.4,
        attack_time: 0.01,
    }
}

fn plug(path: &str, ver: &str) -> PluginInfo {
    PluginInfo {
        name: "N".into(),
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

// ── KVR compare_versions (one scenario per test) ─────────────────────────────

#[test]
fn kvr_cmp_2_0_gt_1_99() {
    assert_eq!(
        app_lib::kvr::compare_versions("2.0", "1.99"),
        Ordering::Greater
    );
}

#[test]
fn kvr_cmp_0_0_1_gt_0_0_0() {
    assert_eq!(
        app_lib::kvr::compare_versions("0.0.1", "0.0.0"),
        Ordering::Greater
    );
}

#[test]
fn kvr_cmp_equal_with_trailing_zeros() {
    assert_eq!(
        app_lib::kvr::compare_versions("2.0.0.0", "2"),
        Ordering::Equal
    );
}

#[test]
fn kvr_cmp_patch_bump() {
    assert_eq!(
        app_lib::kvr::compare_versions("1.0.1", "1.0.0"),
        Ordering::Greater
    );
}

#[test]
fn kvr_cmp_fourth_segment_matters() {
    assert_eq!(
        app_lib::kvr::compare_versions("1.0.0.2", "1.0.0.1"),
        Ordering::Greater
    );
}

#[test]
fn kvr_cmp_prerelease_suffix_after_dot_becomes_zero_component() {
    // "0-beta" does not parse as an integer, so the segment becomes 0 — same as "1.0.0".
    assert_eq!(
        app_lib::kvr::compare_versions("1.0.0-beta", "1.0.0"),
        Ordering::Equal
    );
}

#[test]
fn kvr_parse_single_segment() {
    assert_eq!(app_lib::kvr::parse_version("42"), vec![42]);
}

#[test]
fn kvr_parse_trailing_dot_empty_segment() {
    assert_eq!(app_lib::kvr::parse_version("1.2."), vec![1, 2, 0]);
}

#[test]
fn kvr_extract_version_software_version_json_attr() {
    let html = r#"<meta name="softwareVersion" content="4.5.6" />"#;
    assert_eq!(
        app_lib::kvr::extract_version(html).as_deref(),
        Some("4.5.6")
    );
}

#[test]
fn kvr_extract_download_url_first_generic_link() {
    let html = r#"<html><body><a href="https://cdn.vendor.com/releases/get-installer-v2">download</a></body></html>"#;
    let r = app_lib::kvr::extract_download_url(html).expect("expected a link");
    assert!(r.0.contains("get-installer"));
}

#[cfg(target_os = "macos")]
#[test]
fn kvr_extract_download_url_platform_hint_macos_url() {
    let html = r#"<a href="https://files.example.com/download-mac-universal.dmg">get</a>"#;
    let (_, platform) = app_lib::kvr::extract_download_url(html).expect("link");
    assert!(platform, "mac keyword should set platform hint on macOS");
}

#[cfg(target_os = "windows")]
#[test]
fn kvr_extract_download_url_platform_hint_windows_url() {
    let html = r#"<a href="https://files.example.com/download/windows-x64-setup.exe">get</a>"#;
    let (_, platform) = app_lib::kvr::extract_download_url(html).expect("link");
    assert!(platform);
}

#[cfg(target_os = "linux")]
#[test]
fn kvr_extract_download_url_platform_hint_linux_url() {
    let html = r#"<a href="https://files.example.com/get-linux-amd64.deb">download</a>"#;
    let (_, platform) = app_lib::kvr::extract_download_url(html).expect("link");
    assert!(platform);
}

// ── format_size ─────────────────────────────────────────────────────────────

#[test]
fn format_size_zero() {
    assert_eq!(app_lib::format_size(0), "0 B");
}

#[test]
fn format_size_one_tb() {
    let s = app_lib::format_size(1024_u64.pow(4));
    assert!(s.contains("TB"), "got {s}");
}

#[test]
fn format_size_one_gb() {
    let s = app_lib::format_size(1024_u64.pow(3));
    assert!(s.contains("GB"), "got {s}");
}

#[test]
fn format_size_slightly_below_kib() {
    let s = app_lib::format_size(1023);
    assert!(s.ends_with(" B"), "got {s}");
}

// ── xref normalize_plugin_name ──────────────────────────────────────────────

#[test]
fn norm_strips_vst3_in_parens() {
    assert_eq!(
        normalize_plugin_name("Serum (VST3)"),
        normalize_plugin_name("Serum")
    );
}

#[test]
fn norm_strips_x64_brackets() {
    assert_eq!(normalize_plugin_name("Bass [x64]"), "bass");
}

#[test]
fn norm_collapses_internal_whitespace() {
    assert_eq!(normalize_plugin_name("Foo   Bar"), "foo bar");
}

#[test]
fn norm_unicode_name_lowercased() {
    assert_eq!(normalize_plugin_name("Café AU"), "café au");
}

#[test]
fn norm_bare_x64_suffix_no_parens() {
    assert_eq!(normalize_plugin_name("PluginName x64"), "pluginname");
}

#[test]
fn norm_empty_after_strip_falls_back() {
    let s = "(x64)";
    let out = normalize_plugin_name(s);
    assert!(!out.is_empty());
}

// ── DAW scanner pure helpers ────────────────────────────────────────────────

#[test]
fn ext_matches_dawproject_suffix() {
    assert_eq!(
        ext_matches(Path::new("/p/MySong.dawproject")).as_deref(),
        Some("DAWPROJECT")
    );
}

#[test]
fn ext_matches_ptf_uppercase_name() {
    assert_eq!(
        ext_matches(Path::new("/sessions/Old.ptf")).as_deref(),
        Some("PTF")
    );
}

#[test]
fn ext_matches_not_confused_by_inner_dot_before_ext() {
    assert_eq!(
        ext_matches(Path::new("/a/foo.backup.als")).as_deref(),
        Some("ALS")
    );
}

#[test]
fn is_package_ext_false_for_dawproject_file_ext() {
    assert!(!is_package_ext(Path::new("/x.dawproject")));
}

#[test]
fn daw_name_for_format_ptf_and_ptx() {
    assert_eq!(daw_name_for_format("PTF"), "Pro Tools");
    assert_eq!(daw_name_for_format("PTX"), "Pro Tools");
}

#[test]
fn daw_name_for_format_rpp_bak_same_as_rpp() {
    assert_eq!(daw_name_for_format("RPP-BAK"), daw_name_for_format("RPP"));
}

// ── similarity ──────────────────────────────────────────────────────────────

#[test]
fn fingerprint_distance_symmetric_mismatch() {
    let a = fp("/a.wav", 0.1, 0.2);
    let b = fp("/b.wav", 0.9, 0.8);
    let d1 = fingerprint_distance(&a, &b);
    let d2 = fingerprint_distance(&b, &a);
    assert!((d1 - d2).abs() < 1e-9);
    assert!(d1 > 0.01);
}

#[test]
fn find_similar_respects_max_results_one() {
    let ref_fp = fp("/ref.wav", 0.5, 0.5);
    let cands = vec![
        fp("/1.wav", 0.51, 0.5),
        fp("/2.wav", 0.52, 0.5),
        fp("/3.wav", 0.99, 0.5),
    ];
    let out = find_similar(&ref_fp, &cands, 1);
    assert_eq!(out.len(), 1);
}

#[test]
fn find_similar_orders_nearest_first() {
    let ref_fp = fp("/r.wav", 0.5, 0.5);
    let cands = vec![fp("/far.wav", 0.99, 0.5), fp("/near.wav", 0.51, 0.5)];
    let out = find_similar(&ref_fp, &cands, 2);
    assert_eq!(out[0].0, "/near.wav");
}

// ── scanner plugin type ──────────────────────────────────────────────────────

#[test]
fn get_plugin_type_vst3_and_au() {
    assert_eq!(get_plugin_type(".vst3"), "VST3");
    assert_eq!(get_plugin_type(".component"), "AU");
}

#[test]
fn get_plugin_type_clap_unknown() {
    assert_eq!(get_plugin_type(".clap"), "CLAP");
}

#[test]
fn get_plugin_type_dll_is_vst2() {
    assert_eq!(get_plugin_type(".dll"), "VST2");
}

// ── history compute_plugin_diff ─────────────────────────────────────────────

#[test]
fn compute_diff_removed_when_path_dropped() {
    let old = build_plugin_snapshot(&[plug("/gone.vst3", "1.0")], &[], &[]);
    let new = build_plugin_snapshot(&[], &[], &[]);
    let d = compute_plugin_diff(&old, &new);
    assert_eq!(d.removed.len(), 1);
    assert_eq!(d.removed[0].path, "/gone.vst3");
    assert!(d.added.is_empty());
}

#[test]
fn compute_diff_added_and_removed_disjoint() {
    let old = build_plugin_snapshot(&[plug("/a.vst3", "1.0")], &[], &[]);
    let new = build_plugin_snapshot(&[plug("/b.vst3", "1.0")], &[], &[]);
    let d = compute_plugin_diff(&old, &new);
    assert_eq!(d.removed.len(), 1);
    assert_eq!(d.added.len(), 1);
}

#[test]
fn compute_diff_same_paths_both_unknown_no_version_changed() {
    let a = plug("/x.vst3", "Unknown");
    let b = plug("/x.vst3", "Unknown");
    let old = build_plugin_snapshot(&[a], &[], &[]);
    let new = build_plugin_snapshot(&[b], &[], &[]);
    let d = compute_plugin_diff(&old, &new);
    assert!(d.version_changed.is_empty());
}

// ── serde: AudioSample optional metadata ────────────────────────────────────

#[test]
fn audio_sample_json_optional_fields_omit_when_none() {
    use app_lib::history::AudioSample;
    let s = AudioSample {
        name: "s".into(),
        path: "/p.wav".into(),
        directory: "/".into(),
        format: "WAV".into(),
        size: 4,
        size_formatted: "4 B".into(),
        modified: "m".into(),
        duration: None,
        channels: None,
        sample_rate: None,
        bits_per_sample: None,
    };
    let v = serde_json::to_value(&s).expect("to_value");
    assert!(v.get("duration").is_none());
    assert!(v.get("sampleRate").is_none());
}

#[test]
fn audio_sample_json_roundtrip_with_metadata() {
    use app_lib::history::AudioSample;
    let s = AudioSample {
        name: "s".into(),
        path: "/p.wav".into(),
        directory: "/".into(),
        format: "WAV".into(),
        size: 4,
        size_formatted: "4 B".into(),
        modified: "m".into(),
        duration: Some(1.25),
        channels: Some(2),
        sample_rate: Some(48_000),
        bits_per_sample: Some(24),
    };
    let json = serde_json::to_string(&s).expect("ser");
    let back: AudioSample = serde_json::from_str(&json).expect("de");
    assert_eq!(back.sample_rate, Some(48_000));
    assert_eq!(back.bits_per_sample, Some(24));
}

// ── More KVR / URL / DAW / normalize (explicit cases) ───────────────────────

#[test]
fn kvr_cmp_very_long_patch_run() {
    assert_eq!(
        app_lib::kvr::compare_versions("1.0.0.0.0.1", "1.0.0.0.0.0"),
        Ordering::Greater
    );
}

#[test]
fn kvr_parse_version_non_numeric_middle_segment_zero() {
    assert_eq!(app_lib::kvr::parse_version("1.x.3"), vec![1, 0, 3]);
}

#[test]
fn url_re_matches_http_without_www() {
    let t = "link http://example.org/foo";
    let m = app_lib::kvr::URL_RE.find(t).unwrap();
    assert!(m.as_str().starts_with("http://example.org"));
}

#[test]
fn ext_matches_ardour_lower() {
    assert_eq!(
        ext_matches(Path::new("/p/session.ardour")).as_deref(),
        Some("ARDOUR")
    );
}

#[test]
fn ext_matches_aup3() {
    assert_eq!(
        ext_matches(Path::new("/p/jingle.aup3")).as_deref(),
        Some("AUP3")
    );
}

#[test]
fn daw_name_for_format_aup_vs_aup3_both_audacity() {
    assert_eq!(daw_name_for_format("AUP"), daw_name_for_format("AUP3"));
}

#[test]
fn norm_strips_arm64_suffix() {
    assert!(!normalize_plugin_name("X (arm64)").contains("arm64"));
}

#[test]
fn norm_strips_aax_in_brackets() {
    let n = normalize_plugin_name("Pro-Q (AAX)");
    assert!(!n.contains('('));
}

#[test]
fn history_scan_history_empty_json_array() {
    use app_lib::history::ScanHistory;
    let h = ScanHistory { scans: vec![] };
    let v = serde_json::to_value(&h).unwrap();
    assert_eq!(v["scans"], serde_json::json!([]));
}

#[test]
fn plugin_info_manufacturer_url_roundtrips_json() {
    let p = PluginInfo {
        name: "n".into(),
        path: "/p.vst3".into(),
        plugin_type: "VST3".into(),
        version: "1".into(),
        manufacturer: "m".into(),
        manufacturer_url: Some("https://m.example".into()),
        size: "1 B".into(),
        size_bytes: 1,
        modified: "t".into(),
        architectures: vec![],
    };
    let v = serde_json::to_value(&p).unwrap();
    assert_eq!(v["manufacturerUrl"], serde_json::json!("https://m.example"));
}

#[test]
fn fingerprint_identical_features_zero_distance() {
    let a = fp("/1.wav", 0.25, 0.33);
    let b = fp("/2.wav", 0.25, 0.33);
    assert!(fingerprint_distance(&a, &b) < 1e-9);
}

#[test]
fn find_similar_max_results_exceeds_candidates_len() {
    let r = fp("/r.wav", 0.5, 0.5);
    let c = vec![fp("/1.wav", 0.6, 0.5)];
    let out = find_similar(&r, &c, 100);
    assert_eq!(out.len(), 1);
}

#[test]
fn get_plugin_type_vst2_extension() {
    assert_eq!(get_plugin_type(".vst"), "VST2");
}

#[test]
fn ext_matches_rejects_plain_wav() {
    assert!(ext_matches(Path::new("/a/b.wav")).is_none());
}

#[test]
fn is_package_ext_case_insensitive_logicx() {
    assert!(is_package_ext(Path::new("/P.LOGICX")));
}

#[test]
fn kvr_extract_version_latest_version_dt_dd_style() {
    let html = concat!(
        "<html><body>",
        "Version</th><td>7.8.9</td>",
        "</body></html>"
    );
    assert_eq!(
        app_lib::kvr::extract_version(html).as_deref(),
        Some("7.8.9")
    );
}
