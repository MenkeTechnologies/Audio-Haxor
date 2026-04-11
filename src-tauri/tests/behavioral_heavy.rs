//! Additional large set of focused integration tests (one `#[test]` per case).

use std::cmp::Ordering;
use std::path::Path;

use app_lib::daw_scanner::{daw_name_for_format, ext_matches};
use app_lib::history::{
    KvrCacheEntry, build_audio_snapshot, build_daw_snapshot, build_plugin_snapshot,
    build_preset_snapshot, compute_audio_diff, compute_daw_diff, compute_plugin_diff,
    compute_preset_diff, radix_string,
};
use app_lib::scanner::{PluginInfo, get_plugin_type};
use app_lib::similarity::{AudioFingerprint, find_similar, fingerprint_distance};
use app_lib::xref::{PluginRef, extract_plugins, normalize_plugin_name};
use app_lib::{ExportPayload, ExportPlugin};

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

#[allow(clippy::too_many_arguments)]
fn fp(
    path: &str,
    rms: f64,
    sc: f64,
    zcr: f64,
    low: f64,
    mid: f64,
    high: f64,
    ler: f64,
    atk: f64,
) -> AudioFingerprint {
    AudioFingerprint {
        path: path.into(),
        rms,
        spectral_centroid: sc,
        zero_crossing_rate: zcr,
        low_band_energy: low,
        mid_band_energy: mid,
        high_band_energy: high,
        low_energy_ratio: ler,
        attack_time: atk,
    }
}

// ── KVR ───────────────────────────────────────────────────────────────────────

#[test]
fn kvr_cmp_9_9_vs_10_0() {
    assert_eq!(
        app_lib::kvr::compare_versions("9.9", "10.0"),
        Ordering::Less
    );
}

#[test]
fn kvr_cmp_equal_extra_trailing_zeros() {
    assert_eq!(
        app_lib::kvr::compare_versions("2.0.0.0.0", "2"),
        Ordering::Equal
    );
}

#[test]
fn kvr_parse_single_zero() {
    assert_eq!(app_lib::kvr::parse_version("0"), vec![0]);
}

#[test]
fn kvr_extract_version_dt_dd_table_row() {
    let html = "<table><tr><th>Version</th><dd>11.22.33</dd></tr></table>";
    assert_eq!(
        app_lib::kvr::extract_version(html).as_deref(),
        Some("11.22.33")
    );
}

#[test]
fn kvr_result_serde_none_download() {
    let k = app_lib::kvr::KvrResult {
        product_url: "https://kvraudio.com/p/x".into(),
        download_url: None,
    };
    let v = serde_json::to_value(&k).unwrap();
    assert!(v.get("downloadUrl").unwrap().is_null());
}

#[test]
fn kvr_url_re_skips_leading_non_http() {
    let t = "ftp://ignore http://keep.com/x";
    let m = app_lib::kvr::URL_RE.find(t).unwrap();
    assert!(m.as_str().starts_with("http://keep.com"));
}

// ── format_size / radix ─────────────────────────────────────────────────────

#[test]
fn format_size_just_below_one_kb() {
    let s = app_lib::format_size(1023);
    assert!(s.ends_with(" B"), "{s}");
}

#[test]
fn radix_hex_ff() {
    assert_eq!(radix_string(255, 16), "ff");
}

#[test]
fn radix_base_3_ten() {
    assert_eq!(radix_string(10, 3), "101");
}

// ── DAW ext / names ─────────────────────────────────────────────────────────

#[test]
fn ext_matches_ptf_suffix() {
    assert_eq!(ext_matches(Path::new("/s.ptf")).as_deref(), Some("PTF"));
}

#[test]
fn ext_matches_dawproject_long_name() {
    assert_eq!(
        ext_matches(Path::new("/MyProject.dawproject")).as_deref(),
        Some("DAWPROJECT")
    );
}

#[test]
fn daw_name_unknown_format_string() {
    assert_eq!(daw_name_for_format("NOTREAL"), "Unknown");
}

// ── similarity ───────────────────────────────────────────────────────────────

#[test]
fn fingerprint_mid_band_change_increases_distance() {
    let a = fp("/a.wav", 0.5, 0.2, 0.05, 0.1, 0.2, 0.05, 0.4, 0.01);
    let b = fp("/b.wav", 0.5, 0.2, 0.05, 0.1, 0.99, 0.05, 0.4, 0.01);
    assert!(fingerprint_distance(&a, &b) > 0.01);
}

#[test]
fn fingerprint_high_band_change_increases_distance() {
    let a = fp("/a.wav", 0.5, 0.2, 0.05, 0.1, 0.2, 0.05, 0.4, 0.01);
    let b = fp("/b.wav", 0.5, 0.2, 0.05, 0.1, 0.2, 0.88, 0.4, 0.01);
    assert!(fingerprint_distance(&a, &b) > 0.01);
}

#[test]
fn find_similar_single_candidate_returns_one() {
    let r = fp("/r.wav", 0.5, 0.3, 0.05, 0.1, 0.2, 0.05, 0.4, 0.01);
    let c = vec![fp("/only.wav", 0.51, 0.3, 0.05, 0.1, 0.2, 0.05, 0.4, 0.01)];
    let out = find_similar(&r, &c, 10);
    assert_eq!(out.len(), 1);
}

// ── history: build_* aggregates ───────────────────────────────────────────────

#[test]
fn build_daw_snapshot_counts_same_daw_twice() {
    use app_lib::history::DawProject;
    let p1 = DawProject {
        name: "a".into(),
        path: "/1.als".into(),
        directory: "/".into(),
        format: "ALS".into(),
        daw: "Ableton Live".into(),
        size: 10,
        size_formatted: "10 B".into(),
        modified: "m".into(),
    };
    let p2 = DawProject {
        name: "b".into(),
        path: "/2.als".into(),
        directory: "/".into(),
        format: "ALS".into(),
        daw: "Ableton Live".into(),
        size: 10,
        size_formatted: "10 B".into(),
        modified: "m".into(),
    };
    let s = build_daw_snapshot(&[p1, p2], &[]);
    assert_eq!(s.daw_counts.get("Ableton Live").copied(), Some(2));
    assert_eq!(s.total_bytes, 20);
}

#[test]
fn build_preset_snapshot_format_counts_two_kinds() {
    use app_lib::history::PresetFile;
    let a = PresetFile {
        name: "a".into(),
        path: "/a.fxp".into(),
        directory: "/".into(),
        format: "fxp".into(),
        size: 1,
        size_formatted: "1 B".into(),
        modified: "m".into(),
    };
    let b = PresetFile {
        name: "b".into(),
        path: "/b.h2p".into(),
        directory: "/".into(),
        format: "h2p".into(),
        size: 2,
        size_formatted: "2 B".into(),
        modified: "m".into(),
    };
    let s = build_preset_snapshot(&[a, b], &[]);
    assert_eq!(s.format_counts.get("fxp").copied(), Some(1));
    assert_eq!(s.format_counts.get("h2p").copied(), Some(1));
}

#[test]
fn compute_audio_diff_added_two_samples() {
    use app_lib::history::AudioSample;
    let s1 = AudioSample {
        name: "a".into(),
        path: "/a.wav".into(),
        directory: "/".into(),
        format: "WAV".into(),
        size: 1,
        size_formatted: "1 B".into(),
        modified: "m".into(),
        duration: None,
        channels: None,
        sample_rate: None,
        bits_per_sample: None,
    };
    let s2 = AudioSample {
        name: "b".into(),
        path: "/b.wav".into(),
        directory: "/".into(),
        format: "WAV".into(),
        size: 2,
        size_formatted: "2 B".into(),
        modified: "m".into(),
        duration: None,
        channels: None,
        sample_rate: None,
        bits_per_sample: None,
    };
    let old = build_audio_snapshot(&[], &[]);
    let new = build_audio_snapshot(&[s1, s2], &[]);
    let d = compute_audio_diff(&old, &new);
    assert_eq!(d.added.len(), 2);
}

#[test]
fn compute_daw_diff_no_overlap_two_each_side() {
    use app_lib::history::DawProject;
    let a = DawProject {
        name: "a".into(),
        path: "/a.als".into(),
        directory: "/".into(),
        format: "ALS".into(),
        daw: "Ableton Live".into(),
        size: 1,
        size_formatted: "1 B".into(),
        modified: "m".into(),
    };
    let b = DawProject {
        name: "b".into(),
        path: "/b.als".into(),
        directory: "/".into(),
        format: "ALS".into(),
        daw: "Ableton Live".into(),
        size: 1,
        size_formatted: "1 B".into(),
        modified: "m".into(),
    };
    let c = DawProject {
        name: "c".into(),
        path: "/c.als".into(),
        directory: "/".into(),
        format: "ALS".into(),
        daw: "Ableton Live".into(),
        size: 1,
        size_formatted: "1 B".into(),
        modified: "m".into(),
    };
    let d = DawProject {
        name: "d".into(),
        path: "/d.als".into(),
        directory: "/".into(),
        format: "ALS".into(),
        daw: "Ableton Live".into(),
        size: 1,
        size_formatted: "1 B".into(),
        modified: "m".into(),
    };
    let old = build_daw_snapshot(&[a, b], &[]);
    let new = build_daw_snapshot(&[c, d], &[]);
    let diff = compute_daw_diff(&old, &new);
    assert_eq!(diff.removed.len(), 2);
    assert_eq!(diff.added.len(), 2);
}

// ── xref / scanner ───────────────────────────────────────────────────────────

#[test]
fn normalize_au_suffix_in_brackets() {
    let n = normalize_plugin_name("Comp (AU)");
    assert!(!n.contains("(au)"));
}

#[test]
fn normalize_32_bit_bracket_suffix() {
    let n = normalize_plugin_name("Verb (32-bit)");
    assert!(!n.contains("32"));
}

#[test]
fn extract_plugins_markdown_returns_empty() {
    assert!(extract_plugins("/tmp/readme.md").is_empty());
}

#[test]
fn plugin_ref_json_roundtrip() {
    let p = PluginRef {
        name: "X".into(),
        normalized_name: "x".into(),
        manufacturer: "Y".into(),
        plugin_type: "VST2".into(),
    };
    let json = serde_json::to_string(&p).unwrap();
    let back: PluginRef = serde_json::from_str(&json).unwrap();
    assert_eq!(back.plugin_type, "VST2");
}

#[test]
fn get_plugin_type_bundle_unknown() {
    assert_eq!(get_plugin_type(".bundle"), "Unknown");
}

#[test]
fn get_plugin_type_vst3_exact() {
    assert_eq!(get_plugin_type(".vst3"), "VST3");
}

// ── export / cache serde ─────────────────────────────────────────────────────

#[test]
fn export_plugin_with_architectures_json() {
    let ep = ExportPlugin {
        name: "P".into(),
        plugin_type: "VST3".into(),
        version: "1".into(),
        manufacturer: "Co".into(),
        manufacturer_url: Some("https://co".into()),
        path: "/p.vst3".into(),
        size: "2 B".into(),
        size_bytes: 2,
        modified: "m".into(),
        architectures: vec!["arm64".into(), "x86_64".into()],
    };
    let v = serde_json::to_value(&ep).unwrap();
    assert_eq!(v["architectures"], serde_json::json!(["arm64", "x86_64"]));
}

#[test]
fn kvr_cache_entry_all_none_optional_strings() {
    let e = KvrCacheEntry {
        kvr_url: None,
        update_url: None,
        latest_version: None,
        has_update: false,
        source: "none".into(),
        timestamp: "t".into(),
    };
    let json = serde_json::to_string(&e).unwrap();
    let back: KvrCacheEntry = serde_json::from_str(&json).unwrap();
    assert!(back.kvr_url.is_none());
}

#[test]
fn export_payload_two_plugins_roundtrip() {
    let p = ExportPayload {
        version: "1".into(),
        exported_at: "t".into(),
        plugins: vec![
            ExportPlugin {
                name: "A".into(),
                plugin_type: "AU".into(),
                version: "1".into(),
                manufacturer: "M".into(),
                manufacturer_url: None,
                path: "/a.component".into(),
                size: "1 B".into(),
                size_bytes: 1,
                modified: "m".into(),
                architectures: vec![],
            },
            ExportPlugin {
                name: "B".into(),
                plugin_type: "VST3".into(),
                version: "2".into(),
                manufacturer: "M".into(),
                manufacturer_url: None,
                path: "/b.vst3".into(),
                size: "2 B".into(),
                size_bytes: 2,
                modified: "m".into(),
                architectures: vec![],
            },
        ],
    };
    let json = serde_json::to_string(&p).unwrap();
    let back: ExportPayload = serde_json::from_str(&json).unwrap();
    assert_eq!(back.plugins.len(), 2);
}

// ── plugin diff: version unchanged same string ──────────────────────────────

#[test]
fn compute_plugin_diff_no_version_change_when_same_version() {
    let old = build_plugin_snapshot(&[plug("/p.vst3", "1.5.0")], &[], &[]);
    let new = build_plugin_snapshot(&[plug("/p.vst3", "1.5.0")], &[], &[]);
    let d = compute_plugin_diff(&old, &new);
    assert!(d.version_changed.is_empty());
}

#[test]
fn compute_preset_diff_identical_presets_empty_delta() {
    use app_lib::history::PresetFile;
    let pf = PresetFile {
        name: "n".into(),
        path: "/p.fxp".into(),
        directory: "/".into(),
        format: "fxp".into(),
        size: 1,
        size_formatted: "1 B".into(),
        modified: "m".into(),
    };
    let s = build_preset_snapshot(&[pf], &[]);
    let d = compute_preset_diff(&s, &s);
    assert!(d.added.is_empty() && d.removed.is_empty());
}

#[test]
fn compute_plugin_diff_unknown_to_known_not_version_changed() {
    let old = build_plugin_snapshot(&[plug("/p.vst3", "Unknown")], &[], &[]);
    let new = build_plugin_snapshot(&[plug("/p.vst3", "2.0")], &[], &[]);
    let d = compute_plugin_diff(&old, &new);
    assert!(
        d.version_changed.is_empty(),
        "Unknown -> known should not populate version_changed"
    );
}

// ── decode / symphonia guard ─────────────────────────────────────────────────

#[test]
fn bpm_decode_symphonia_missing_none() {
    assert!(app_lib::bpm::decode_with_symphonia_pub(Path::new("/no/such.mp3")).is_none());
}

#[test]
fn fingerprint_low_energy_ratio_change() {
    let a = fp("/a.wav", 0.5, 0.2, 0.05, 0.1, 0.2, 0.05, 0.4, 0.01);
    let b = fp("/b.wav", 0.5, 0.2, 0.05, 0.1, 0.2, 0.05, 0.01, 0.01);
    assert!(fingerprint_distance(&a, &b) > 1e-6);
}

// ── Batch 2: more KVR / history / similarity ─────────────────────────────────

#[test]
fn kvr_cmp_less_first_component() {
    assert_eq!(
        app_lib::kvr::compare_versions("0.9.9", "1.0.0"),
        Ordering::Less
    );
}

#[test]
fn kvr_parse_many_trailing_dots() {
    assert_eq!(app_lib::kvr::parse_version("1..."), vec![1, 0, 0, 0]);
}

#[test]
fn format_size_two_tb_unit() {
    let s = app_lib::format_size(2 * 1024_u64.pow(4));
    assert!(s.contains("TB"), "{s}");
}

#[test]
fn radix_string_one_million_base_7() {
    let s = radix_string(1_000_000, 7);
    assert!(!s.is_empty());
}

#[test]
fn compute_plugin_diff_four_added_at_once() {
    let new = build_plugin_snapshot(
        &[
            plug("/a.vst3", "1"),
            plug("/b.vst3", "1"),
            plug("/c.vst3", "1"),
            plug("/d.vst3", "1"),
        ],
        &[],
        &[],
    );
    let old = build_plugin_snapshot(&[], &[], &[]);
    let d = compute_plugin_diff(&old, &new);
    assert_eq!(d.added.len(), 4);
}

#[test]
fn build_plugin_snapshot_roots_preserved() {
    let s = build_plugin_snapshot(&[plug("/x.vst3", "1")], &["/dir".into()], &["/root".into()]);
    assert_eq!(s.directories, vec!["/dir"]);
    assert_eq!(s.roots, vec!["/root"]);
}

#[test]
fn similarity_spectral_centroid_only_change() {
    let a = fp("/a.wav", 0.5, 0.1, 0.05, 0.1, 0.2, 0.05, 0.4, 0.01);
    let b = fp("/b.wav", 0.5, 0.45, 0.05, 0.1, 0.2, 0.05, 0.4, 0.01);
    assert!(fingerprint_distance(&a, &b) > 1e-6);
}

#[test]
fn find_similar_two_tied_distance_order_stable() {
    let r = fp("/r.wav", 0.5, 0.3, 0.05, 0.1, 0.2, 0.05, 0.4, 0.01);
    let c = vec![
        fp("/1.wav", 0.51, 0.3, 0.05, 0.1, 0.2, 0.05, 0.4, 0.01),
        fp("/2.wav", 0.51, 0.3, 0.05, 0.1, 0.2, 0.05, 0.4, 0.01),
    ];
    let out = find_similar(&r, &c, 2);
    assert_eq!(out.len(), 2);
}

#[test]
fn normalize_aax_in_brackets() {
    let n = normalize_plugin_name("Limiter (AAX)");
    assert!(!n.contains("aax"));
}

#[test]
fn extract_plugins_no_extension_empty() {
    assert!(extract_plugins("/tmp/noextension").is_empty());
}

#[test]
fn kvr_extract_version_latest_keyword_line() {
    let html = "<p>Latest version 8.1 here</p>";
    assert_eq!(app_lib::kvr::extract_version(html).as_deref(), Some("8.1"));
}

#[test]
fn plugin_info_manufacturer_url_some_json() {
    let p = PluginInfo {
        name: "n".into(),
        path: "/p.vst3".into(),
        plugin_type: "VST3".into(),
        version: "1".into(),
        manufacturer: "m".into(),
        manufacturer_url: Some("https://m.com".into()),
        size: "1 B".into(),
        size_bytes: 1,
        modified: "m".into(),
        architectures: vec![],
    };
    let v = serde_json::to_value(&p).unwrap();
    assert_eq!(v["manufacturerUrl"], "https://m.com");
}

#[test]
fn read_aiff_pub_missing_none() {
    assert!(app_lib::bpm::read_aiff_pcm_pub(Path::new("/no/such.aif")).is_none());
}

#[test]
fn kvr_compare_versions_reflexive_single_digit() {
    assert_eq!(app_lib::kvr::compare_versions("7", "7"), Ordering::Equal);
}
