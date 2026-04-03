//! Hand-written fixtures: HTML snippets, path edge cases, and snapshot diffs.
//! No generated tables — each case documents a real behavior we care about.

use std::cmp::Ordering;
use std::path::Path;

use app_lib::history::{compute_plugin_diff, radix_string, ScanSnapshot};
use app_lib::scanner::PluginInfo;

fn sample_plugin(path: &str, version: &str) -> PluginInfo {
    PluginInfo {
        name: "TestPlugin".into(),
        path: path.into(),
        plugin_type: "VST3".into(),
        version: version.into(),
        manufacturer: "Co".into(),
        manufacturer_url: None,
        size: "1 B".into(),
        size_bytes: 1,
        modified: "2024-01-01".into(),
        architectures: vec![],
    }
}

// ── KVR: parse / compare (non-numeric segments, unequal-length vectors) ──

#[test]
fn kvr_parse_version_non_numeric_segment_becomes_zero() {
    assert_eq!(app_lib::kvr::parse_version("1.x.3"), vec![1, 0, 3]);
}

#[test]
fn kvr_compare_versions_pads_zero_for_missing_trailing_components() {
    assert_eq!(
        app_lib::kvr::compare_versions("1.0", "1.0.0"),
        Ordering::Equal
    );
    assert_eq!(
        app_lib::kvr::compare_versions("1.0.1", "1.0"),
        Ordering::Greater
    );
}

#[test]
fn kvr_extract_version_from_plain_label() {
    let html = r#"<p>Version: 3.5.2</p>"#;
    assert_eq!(
        app_lib::kvr::extract_version(html).as_deref(),
        Some("3.5.2")
    );
}

#[test]
fn kvr_extract_version_from_dt_dd_block() {
    let html = r#"<dl><dt>Version</dt><dd>2.1.0</dd></dl>"#;
    assert_eq!(
        app_lib::kvr::extract_version(html).as_deref(),
        Some("2.1.0")
    );
}

#[test]
fn kvr_extract_version_from_json_ld_style_software_version() {
    let html = r#""softwareVersion": "4.0.1""#;
    assert_eq!(
        app_lib::kvr::extract_version(html).as_deref(),
        Some("4.0.1")
    );
}

#[test]
fn kvr_extract_download_url_finds_href_with_download_token() {
    let html = r#"<a href="https://files.example.com/download/installer">DL</a>"#;
    let r = app_lib::kvr::extract_download_url(html);
    assert!(r.is_some());
    let (url, _) = r.unwrap();
    assert!(url.contains("download"));
}

#[cfg(target_os = "macos")]
#[test]
fn kvr_extract_download_url_marks_platform_when_url_contains_mac_keyword() {
    let html = r#"<a href="https://cdn.example.com/download/mac">Mac</a>"#;
    let r = app_lib::kvr::extract_download_url(html).expect("expected link");
    assert!(r.1, "mac in URL should count as platform-specific on macOS");
}

#[cfg(target_os = "linux")]
#[test]
fn kvr_extract_download_url_marks_platform_when_url_contains_linux_keyword() {
    let html = r#"<a href="https://cdn.example.com/download/linux">Linux</a>"#;
    let r = app_lib::kvr::extract_download_url(html).expect("expected link");
    assert!(r.1, "linux in URL should count as platform-specific on Linux");
}

#[cfg(target_os = "windows")]
#[test]
fn kvr_extract_download_url_marks_platform_when_url_contains_windows_keyword() {
    let html = r#"<a href="https://cdn.example.com/download/windows">Win</a>"#;
    let r = app_lib::kvr::extract_download_url(html).expect("expected link");
    assert!(r.1, "windows in URL should count as platform-specific on Windows");
}

// ── Similarity: truncation and empty input ──

#[test]
fn find_similar_respects_max_results() {
    use app_lib::similarity::{find_similar, AudioFingerprint};

    let mk = |path: &str, rms: f64| AudioFingerprint {
        path: path.into(),
        rms,
        spectral_centroid: 1000.0,
        zero_crossing_rate: 0.05,
        low_band_energy: 0.1,
        mid_band_energy: 0.2,
        high_band_energy: 0.05,
        low_energy_ratio: 0.4,
        attack_time: 0.01,
    };
    let reference = mk("/ref.wav", 0.5);
    let candidates = vec![mk("/a.wav", 0.51), mk("/b.wav", 0.7), mk("/c.wav", 0.9)];
    let out = find_similar(&reference, &candidates, 1);
    assert_eq!(out.len(), 1);
}

#[test]
fn find_similar_empty_candidates_returns_empty() {
    use app_lib::similarity::{find_similar, AudioFingerprint};

    let reference = AudioFingerprint {
        path: "/ref.wav".into(),
        rms: 0.5,
        spectral_centroid: 1000.0,
        zero_crossing_rate: 0.05,
        low_band_energy: 0.1,
        mid_band_energy: 0.2,
        high_band_energy: 0.05,
        low_energy_ratio: 0.4,
        attack_time: 0.01,
    };
    let out = find_similar(&reference, &[], 10);
    assert!(out.is_empty());
}

// ── xref: normalize stacked suffixes, unsupported path ──

#[test]
fn normalize_plugin_name_strips_multiple_arch_suffixes_in_order() {
    let s = "Serum (x64) (VST3)";
    assert_eq!(
        app_lib::xref::normalize_plugin_name(s),
        "serum"
    );
}

#[test]
fn extract_plugins_missing_file_returns_empty() {
    let p = "/nonexistent/path/does/not/exist.cpr";
    assert!(app_lib::xref::extract_plugins(p).is_empty());
}

#[test]
fn extract_plugins_unknown_extension_returns_empty() {
    let p = "/tmp/foo.not_a_daw_project";
    assert!(app_lib::xref::extract_plugins(p).is_empty());
}

// ── format_size ──

#[test]
fn format_size_zero_and_sub_kib_boundary() {
    assert_eq!(app_lib::format_size(0), "0 B");
    assert_eq!(app_lib::format_size(1023), "1023.0 B");
}

// ── History: radix ids and plugin diff ──

#[test]
fn radix_string_base36_matches_known_encoding() {
    assert_eq!(radix_string(0, 36), "0");
    assert_eq!(radix_string(35, 36), "z");
    assert_eq!(radix_string(36, 36), "10");
}

#[test]
fn compute_plugin_diff_detects_added_removed_and_version_change() {
    let old = ScanSnapshot {
        id: "old".into(),
        timestamp: "t0".into(),
        plugin_count: 2,
        plugins: vec![
            sample_plugin("/a.vst3", "1.0.0"),
            sample_plugin("/b.vst3", "2.0.0"),
        ],
        directories: vec![],
        roots: vec![],
    };
    let new = ScanSnapshot {
        id: "new".into(),
        timestamp: "t1".into(),
        plugin_count: 2,
        plugins: vec![
            sample_plugin("/b.vst3", "2.1.0"),
            sample_plugin("/c.vst3", "1.0.0"),
        ],
        directories: vec![],
        roots: vec![],
    };
    let d = compute_plugin_diff(&old, &new);
    assert_eq!(d.added.len(), 1);
    assert_eq!(d.added[0].path, "/c.vst3");
    assert_eq!(d.removed.len(), 1);
    assert_eq!(d.removed[0].path, "/a.vst3");
    assert_eq!(d.version_changed.len(), 1);
    assert_eq!(d.version_changed[0].plugin.path, "/b.vst3");
    assert_eq!(d.version_changed[0].previous_version, "2.0.0");
}

#[test]
fn compute_plugin_diff_skips_version_change_when_either_side_unknown() {
    let old = ScanSnapshot {
        id: "o".into(),
        timestamp: "t".into(),
        plugin_count: 1,
        plugins: vec![sample_plugin("/x.vst3", "Unknown")],
        directories: vec![],
        roots: vec![],
    };
    let new = ScanSnapshot {
        id: "n".into(),
        timestamp: "t".into(),
        plugin_count: 1,
        plugins: vec![sample_plugin("/x.vst3", "2.0.0")],
        directories: vec![],
        roots: vec![],
    };
    let d = compute_plugin_diff(&old, &new);
    assert!(d.version_changed.is_empty());
}

// ── DAW scanner: extensions and package detection ──

#[test]
fn daw_ext_matches_longest_suffix_wins() {
    assert_eq!(
        app_lib::daw_scanner::ext_matches(Path::new("backup.rpp-bak")).as_deref(),
        Some("RPP-BAK")
    );
}

#[test]
fn daw_ext_matches_dawproject() {
    assert_eq!(
        app_lib::daw_scanner::ext_matches(Path::new("project.dawproject")).as_deref(),
        Some("DAWPROJECT")
    );
}

#[test]
fn daw_is_package_ext_logicx_and_band() {
    assert!(app_lib::daw_scanner::is_package_ext(Path::new("song.logicx")));
    assert!(app_lib::daw_scanner::is_package_ext(Path::new("my.band")));
    assert!(!app_lib::daw_scanner::is_package_ext(Path::new("song.als")));
}
