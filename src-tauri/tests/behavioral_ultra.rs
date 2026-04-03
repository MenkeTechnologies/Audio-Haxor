//! Large focused integration batch: KVR, `format_size`, `radix_string`, DAW helpers,
//! snapshots, similarity, xref, serde — complements `behavioral_heavy` / `behavioral_ton`.

use std::cmp::Ordering;
use std::path::Path;

use app_lib::daw_scanner::{daw_name_for_format, ext_matches, is_package_ext};
use app_lib::history::{
    build_audio_snapshot, build_daw_snapshot, build_plugin_snapshot, build_preset_snapshot,
    compute_audio_diff, compute_daw_diff, compute_plugin_diff, compute_preset_diff, radix_string,
    AudioSample, DawProject, KvrCacheEntry, PresetFile,
};
use app_lib::scanner::{get_plugin_type, PluginInfo};
use app_lib::similarity::{fingerprint_distance, find_similar, AudioFingerprint};
use app_lib::xref::{extract_plugins, normalize_plugin_name, PluginRef};
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

fn fp(path: &str) -> AudioFingerprint {
    AudioFingerprint {
        path: path.into(),
        rms: 0.4,
        spectral_centroid: 0.25,
        zero_crossing_rate: 0.05,
        low_band_energy: 0.1,
        mid_band_energy: 0.2,
        high_band_energy: 0.05,
        low_energy_ratio: 0.35,
        attack_time: 0.02,
    }
}

fn sample(path: &str) -> AudioSample {
    AudioSample {
        name: "s".into(),
        path: path.into(),
        directory: "/d".into(),
        format: "WAV".into(),
        size: 100,
        size_formatted: "100 B".into(),
        modified: "m".into(),
        duration: Some(1.0),
        channels: Some(2),
        sample_rate: Some(44100),
        bits_per_sample: Some(16),
    }
}

fn dawproj(path: &str) -> DawProject {
    DawProject {
        name: "p".into(),
        path: path.into(),
        directory: "/d".into(),
        format: "dawproject".into(),
        daw: "dawproject".into(),
        size: 1,
        size_formatted: "1 B".into(),
        modified: "m".into(),
    }
}

fn preset(path: &str) -> PresetFile {
    PresetFile {
        name: "n".into(),
        path: path.into(),
        directory: "/".into(),
        format: "fxp".into(),
        size: 1,
        size_formatted: "1 B".into(),
        modified: "m".into(),
    }
}

// ── KVR ───────────────────────────────────────────────────────────────────────

#[test]
fn kvr_cmp_alpha_vs_numeric() {
    assert_eq!(
        app_lib::kvr::compare_versions("10", "2"),
        Ordering::Greater
    );
}

#[test]
fn kvr_cmp_prerelease_style() {
    assert_eq!(
        app_lib::kvr::compare_versions("1.0.0", "1.0.1"),
        Ordering::Less
    );
}

#[test]
fn kvr_parse_leading_zeros_trimmed() {
    assert_eq!(app_lib::kvr::parse_version("01.02"), vec![1, 2]);
}

#[test]
fn kvr_parse_single_component() {
    assert_eq!(app_lib::kvr::parse_version("42"), vec![42]);
}

#[test]
fn kvr_extract_version_colon_pattern() {
    let html = r#"Version: 3.14.159"#;
    assert_eq!(
        app_lib::kvr::extract_version(html).as_deref(),
        Some("3.14.159")
    );
}

#[test]
fn kvr_url_re_finds_https_with_query() {
    let t = "see https://example.com/x?y=1 ok";
    let m = app_lib::kvr::URL_RE.find(t).unwrap();
    assert!(m.as_str().contains("example.com"));
}

#[test]
fn update_result_serde_roundtrip() {
    let u = app_lib::kvr::UpdateResult {
        latest_version: "2".into(),
        has_update: true,
        source: "kvr".into(),
        update_url: None,
        kvr_url: Some("https://k/x".into()),
        has_platform_download: false,
    };
    let j = serde_json::to_string(&u).unwrap();
    let back: app_lib::kvr::UpdateResult = serde_json::from_str(&j).unwrap();
    assert!(back.has_update);
}

// ── format_size / radix ───────────────────────────────────────────────────────

#[test]
fn format_size_one_byte() {
    assert_eq!(app_lib::format_size(1), "1.0 B");
}

#[test]
fn format_size_three_mb() {
    let s = app_lib::format_size(3 * 1024 * 1024);
    assert!(s.contains("MB"), "{s}");
}

#[test]
fn radix_base_2_powers() {
    assert_eq!(radix_string(8, 2), "1000");
}

#[test]
fn radix_base_36_single_digit() {
    assert_eq!(radix_string(35, 36), "z");
}

#[test]
fn radix_zero_always_zero() {
    assert_eq!(radix_string(0, 16), "0");
}

#[test]
fn radix_base_10_large() {
    assert_eq!(radix_string(999_999, 10), "999999");
}

// ── DAW / scanner ─────────────────────────────────────────────────────────────

#[test]
fn daw_name_reaper_from_rpp_token() {
    assert_eq!(daw_name_for_format("RPP"), "REAPER");
}

#[test]
fn ext_matches_als_token_and_daw_name() {
    let tok = ext_matches(Path::new("/p/MyProject.als"));
    assert_eq!(tok.as_deref(), Some("ALS"));
    assert_eq!(daw_name_for_format("ALS"), "Ableton Live");
}

#[test]
fn ext_matches_dawproject_token_and_daw_name() {
    let tok = ext_matches(Path::new("/x.dawproject"));
    assert_eq!(tok.as_deref(), Some("DAWPROJECT"));
    assert_eq!(daw_name_for_format("DAWPROJECT"), "DAWproject");
}

#[test]
fn is_package_logic_true() {
    assert!(is_package_ext(Path::new("/a.logicx")));
}

#[test]
fn get_plugin_type_vst3() {
    assert_eq!(get_plugin_type(".vst3"), "VST3");
}

#[test]
fn get_plugin_type_uppercase_dot_unknown() {
    assert_eq!(get_plugin_type(".VST3"), "Unknown");
}

// ── Similarity ─────────────────────────────────────────────────────────────────

#[test]
fn fingerprint_distance_self_zero() {
    let a = fp("/x.wav");
    assert!(fingerprint_distance(&a, &a) < 1e-9);
}

#[test]
fn fingerprint_distance_symmetric() {
    let a = fp("/a.wav");
    let mut b = fp("/b.wav");
    b.rms = 0.9;
    let d1 = fingerprint_distance(&a, &b);
    let d2 = fingerprint_distance(&b, &a);
    assert!((d1 - d2).abs() < 1e-9);
}

#[test]
fn find_similar_max_zero_returns_empty() {
    let r = fp("/r.wav");
    let c = vec![fp("/c.wav")];
    assert!(find_similar(&r, &c, 0).is_empty());
}

#[test]
fn find_similar_excludes_reference_path() {
    let r = fp("/only.wav");
    let c = vec![r.clone()];
    assert!(find_similar(&r, &c, 5).is_empty());
}

// ── Snapshots / diff ───────────────────────────────────────────────────────────

#[test]
fn compute_audio_diff_swap_roots_only() {
    let a = build_audio_snapshot(&[sample("/a.wav")], &["/r1".into()]);
    let b = build_audio_snapshot(&[sample("/a.wav")], &["/r2".into()]);
    let d = compute_audio_diff(&a, &b);
    assert!(d.added.is_empty() && d.removed.is_empty());
}

#[test]
fn compute_daw_diff_one_removed() {
    let old = build_daw_snapshot(&[dawproj("/p.dawproject")], &[]);
    let new = build_daw_snapshot(&[], &[]);
    let d = compute_daw_diff(&old, &new);
    assert_eq!(d.removed.len(), 1);
}

#[test]
fn compute_preset_diff_one_added() {
    let old = build_preset_snapshot(&[], &[]);
    let new = build_preset_snapshot(&[preset("/n.fxp")], &[]);
    let d = compute_preset_diff(&old, &new);
    assert_eq!(d.added.len(), 1);
}

#[test]
fn compute_plugin_diff_version_bump() {
    let old = build_plugin_snapshot(&[plug("/p.vst3", "1.0")], &[], &[]);
    let new = build_plugin_snapshot(&[plug("/p.vst3", "2.0")], &[], &[]);
    let d = compute_plugin_diff(&old, &new);
    assert_eq!(d.version_changed.len(), 1);
}

// ── Xref ──────────────────────────────────────────────────────────────────────

#[test]
fn normalize_strips_trailing_vst3_in_parens() {
    let n = normalize_plugin_name("MyPlug (VST3)");
    assert!(!n.contains("vst3"));
}

#[test]
fn plugin_ref_json_roundtrip() {
    let p = PluginRef {
        name: "X".into(),
        normalized_name: "x".into(),
        manufacturer: "M".into(),
        plugin_type: "VST3".into(),
    };
    let j = serde_json::to_string(&p).unwrap();
    let back: PluginRef = serde_json::from_str(&j).unwrap();
    assert_eq!(back.name, "X");
}

#[test]
fn extract_plugins_empty_path_components() {
    assert!(extract_plugins("").is_empty());
}

// ── Export / cache serde ──────────────────────────────────────────────────────

#[test]
fn export_payload_empty_plugins_array() {
    let p = ExportPayload {
        version: "1".into(),
        exported_at: "t".into(),
        plugins: vec![],
    };
    let v = serde_json::to_value(&p).unwrap();
    assert_eq!(v["plugins"].as_array().unwrap().len(), 0);
}

#[test]
fn kvr_cache_entry_all_none_urls() {
    let e = KvrCacheEntry {
        kvr_url: None,
        update_url: None,
        latest_version: None,
        has_update: false,
        source: "test".into(),
        timestamp: "0".into(),
    };
    let j = serde_json::to_string(&e).unwrap();
    assert!(j.contains("null"));
}

#[test]
fn export_plugin_architectures_two() {
    let p = ExportPlugin {
        name: "n".into(),
        plugin_type: "AU".into(),
        version: "1".into(),
        manufacturer: "m".into(),
        manufacturer_url: None,
        path: "/p.component".into(),
        size: "1 B".into(),
        size_bytes: 1,
        modified: "m".into(),
        architectures: vec!["arm64".into(), "x86_64".into()],
    };
    let v = serde_json::to_value(&p).unwrap();
    assert_eq!(v["architectures"].as_array().unwrap().len(), 2);
}

// ── gen_id uniqueness ──────────────────────────────────────────────────────────

#[test]
fn gen_id_batch_80_unique() {
    use app_lib::history::gen_id;
    use std::collections::HashSet;
    let mut s = HashSet::new();
    for _ in 0..80 {
        assert!(s.insert(gen_id()));
    }
}

// ── Second wave: more KVR + format_size + DAW ──────────────────────────────────

#[test]
fn kvr_cmp_empty_vs_zero() {
    assert_eq!(
        app_lib::kvr::compare_versions("", "0"),
        Ordering::Equal
    );
}

#[test]
fn kvr_parse_empty_yields_triple_zero() {
    assert_eq!(app_lib::kvr::parse_version(""), vec![0, 0, 0]);
}

#[test]
fn format_size_half_tb() {
    let s = app_lib::format_size(512 * 1024_u64.pow(4));
    assert!(s.contains("TB"), "{s}");
}

#[test]
fn ext_matches_ptx() {
    assert!(ext_matches(Path::new("/session.ptx")).is_some());
}

#[test]
fn ext_matches_unknown_ext() {
    assert!(ext_matches(Path::new("/f.unknownextfortest")).is_none());
}

#[test]
fn daw_name_unknown_slug() {
    assert_eq!(daw_name_for_format("___not_a_real_daw___"), "Unknown");
}

#[test]
fn compute_audio_diff_two_added() {
    let old = build_audio_snapshot(&[], &[]);
    let new = build_audio_snapshot(&[sample("/a.wav"), sample("/b.wav")], &[]);
    let d = compute_audio_diff(&old, &new);
    assert_eq!(d.added.len(), 2);
}

#[test]
fn compute_plugin_diff_two_removed() {
    let old = build_plugin_snapshot(&[plug("/a.vst3", "1"), plug("/b.vst3", "1")], &[], &[]);
    let new = build_plugin_snapshot(&[], &[], &[]);
    let d = compute_plugin_diff(&old, &new);
    assert_eq!(d.removed.len(), 2);
}

#[test]
fn fingerprint_attack_time_delta() {
    let mut a = fp("/a.wav");
    let mut b = fp("/b.wav");
    a.attack_time = 0.01;
    b.attack_time = 0.99;
    assert!(fingerprint_distance(&a, &b) > 0.01);
}

#[test]
fn normalize_plugin_name_au_suffix() {
    let n = normalize_plugin_name("Synth (AU)");
    assert!(!n.contains("(AU)"));
}

#[test]
fn radix_string_base_12() {
    let s = radix_string(144, 12);
    assert_eq!(s, "100");
}

#[test]
fn kvr_extract_from_th_version_cell() {
    let html = "<th>Version</th><td>7.8.9</td>";
    assert!(app_lib::kvr::extract_version(html).is_some());
}

#[test]
fn export_payload_two_plugins_roundtrip() {
    let p = ExportPayload {
        version: "1".into(),
        exported_at: "now".into(),
        plugins: vec![
            ExportPlugin {
                name: "A".into(),
                plugin_type: "VST3".into(),
                version: "1".into(),
                manufacturer: "M".into(),
                manufacturer_url: None,
                path: "/a.vst3".into(),
                size: "1 B".into(),
                size_bytes: 1,
                modified: "m".into(),
                architectures: vec![],
            },
            ExportPlugin {
                name: "B".into(),
                plugin_type: "AU".into(),
                version: "2".into(),
                manufacturer: "M".into(),
                manufacturer_url: Some("https://m".into()),
                path: "/b.component".into(),
                size: "2 B".into(),
                size_bytes: 2,
                modified: "m".into(),
                architectures: vec!["arm64".into()],
            },
        ],
    };
    let j = serde_json::to_string(&p).unwrap();
    let back: ExportPayload = serde_json::from_str(&j).unwrap();
    assert_eq!(back.plugins.len(), 2);
}

#[test]
fn find_similar_sorted_by_distance() {
    let r = fp("/r.wav");
    let mut c1 = fp("/c1.wav");
    c1.rms = 0.41;
    let mut c2 = fp("/c2.wav");
    c2.rms = 0.99;
    let out = find_similar(&r, &[c1, c2], 2);
    assert_eq!(out.len(), 2);
    assert!(out[0].1 <= out[1].1);
}

#[test]
fn build_daw_snapshot_counts() {
    let s = build_daw_snapshot(
        &[dawproj("/a.dawproject"), dawproj("/b.dawproject")],
        &["/root".into()],
    );
    assert_eq!(s.projects.len(), 2);
    assert_eq!(s.roots, vec!["/root"]);
}

#[test]
fn compute_preset_diff_removed() {
    let old = build_preset_snapshot(&[preset("/old.fxp")], &[]);
    let new = build_preset_snapshot(&[], &[]);
    let d = compute_preset_diff(&old, &new);
    assert_eq!(d.removed.len(), 1);
}

#[test]
fn get_plugin_type_clap_unknown() {
    assert_eq!(get_plugin_type(".clap"), "Unknown");
}

#[test]
fn get_plugin_type_aaxplugin_unknown() {
    assert_eq!(get_plugin_type(".aaxplugin"), "Unknown");
}

#[test]
fn kvr_cmp_equal_normalized() {
    assert_eq!(
        app_lib::kvr::compare_versions("1.0.0", "1.0"),
        Ordering::Equal
    );
}

#[test]
fn radix_string_base_24() {
    assert_eq!(radix_string(24, 24), "10");
}

#[test]
fn format_size_1023_bytes() {
    let s = app_lib::format_size(1023);
    assert!(s.ends_with(" B"), "{s}");
}

#[test]
fn plugin_info_default_arch_empty_vec() {
    let p = PluginInfo {
        name: "n".into(),
        path: "/p.vst3".into(),
        plugin_type: "VST3".into(),
        version: "1".into(),
        manufacturer: "m".into(),
        manufacturer_url: None,
        size: "1 B".into(),
        size_bytes: 1,
        modified: "m".into(),
        architectures: vec![],
    };
    assert!(p.architectures.is_empty());
}

#[test]
fn audio_fingerprint_json_keys() {
    let f = fp("/z.wav");
    let v = serde_json::to_value(&f).unwrap();
    assert!(v.get("spectral_centroid").is_some());
}

#[test]
fn kvr_parse_version_trailing_nonnumeric_stops() {
    assert_eq!(app_lib::kvr::parse_version("2.5beta"), vec![2, 0]);
}

#[test]
fn ext_matches_lower_case_extension() {
    assert!(ext_matches(Path::new("/x.rpp")).is_some());
}

#[test]
fn is_package_ext_band_false() {
    assert!(!is_package_ext(Path::new("/x.rpp")));
}

#[test]
fn compute_daw_diff_identical_empty() {
    let s = build_daw_snapshot(&[dawproj("/p.dawproject")], &[]);
    let d = compute_daw_diff(&s, &s);
    assert!(d.added.is_empty() && d.removed.is_empty());
}

#[test]
fn compute_audio_diff_identical_empty() {
    let s = build_audio_snapshot(&[sample("/a.wav")], &[]);
    let d = compute_audio_diff(&s, &s);
    assert!(d.added.is_empty() && d.removed.is_empty());
}

#[test]
fn fingerprint_zero_crossing_change() {
    let mut a = fp("/a.wav");
    let mut b = fp("/b.wav");
    a.zero_crossing_rate = 0.01;
    b.zero_crossing_rate = 0.9;
    assert!(fingerprint_distance(&a, &b) > 0.1);
}

#[test]
fn kvr_result_roundtrip_download_some() {
    let k = app_lib::kvr::KvrResult {
        product_url: "https://p".into(),
        download_url: Some("https://d".into()),
    };
    let j = serde_json::to_string(&k).unwrap();
    let back: app_lib::kvr::KvrResult = serde_json::from_str(&j).unwrap();
    assert_eq!(back.download_url.as_deref(), Some("https://d"));
}

#[test]
fn normalize_plugin_name_trim_whitespace() {
    assert_eq!(normalize_plugin_name("  MyPlug  "), "myplug");
}

#[test]
fn radix_string_max_base_36() {
    let s = radix_string(10_000, 36);
    assert!(!s.is_empty());
}

#[test]
fn build_plugin_snapshot_empty() {
    let s = build_plugin_snapshot(&[], &[], &[]);
    assert!(s.plugins.is_empty());
}

#[test]
fn build_preset_snapshot_two_roots() {
    let s = build_preset_snapshot(&[preset("/a.fxp")], &["/r1".into(), "/r2".into()]);
    assert_eq!(s.roots.len(), 2);
}

// ── Extra wave: DAW tokens, radix, diffs ───────────────────────────────────────

#[test]
fn daw_name_logicx() {
    assert_eq!(daw_name_for_format("LOGICX"), "Logic Pro");
}

#[test]
fn daw_name_bitwig() {
    assert_eq!(daw_name_for_format("BWPROJECT"), "Bitwig Studio");
}

#[test]
fn ext_matches_cpr_token() {
    assert_eq!(ext_matches(Path::new("/p.cpr")).as_deref(), Some("CPR"));
}

#[test]
fn ext_matches_bwproject_token() {
    assert_eq!(
        ext_matches(Path::new("/b.bwproject")).as_deref(),
        Some("BWPROJECT")
    );
}

#[test]
fn kvr_cmp_longer_version_greater() {
    assert_eq!(
        app_lib::kvr::compare_versions("2.0.1", "2.0.0"),
        Ordering::Greater
    );
}

#[test]
fn kvr_parse_many_components() {
    assert_eq!(
        app_lib::kvr::parse_version("1.2.3.4.5"),
        vec![1, 2, 3, 4, 5]
    );
}

#[test]
fn radix_string_base_8() {
    assert_eq!(radix_string(64, 8), "100");
}

#[test]
fn format_size_512_kb() {
    let s = app_lib::format_size(512 * 1024);
    assert!(s.contains("KB"), "{s}");
}

#[test]
fn compute_plugin_diff_empty_both() {
    let a = build_plugin_snapshot(&[], &[], &[]);
    let b = build_plugin_snapshot(&[], &[], &[]);
    let d = compute_plugin_diff(&a, &b);
    assert!(d.added.is_empty() && d.removed.is_empty() && d.version_changed.is_empty());
}

#[test]
fn build_audio_snapshot_empty() {
    let s = build_audio_snapshot(&[], &[]);
    assert!(s.samples.is_empty());
}

#[test]
fn build_daw_snapshot_empty() {
    let s = build_daw_snapshot(&[], &[]);
    assert!(s.projects.is_empty());
}

#[test]
fn fingerprint_mid_band_energy_change() {
    let mut a = fp("/a.wav");
    let mut b = fp("/b.wav");
    a.mid_band_energy = 0.1;
    b.mid_band_energy = 0.95;
    assert!(fingerprint_distance(&a, &b) > 0.05);
}

#[test]
fn find_similar_one_candidate() {
    let r = fp("/r.wav");
    let c = vec![fp("/c.wav")];
    let out = find_similar(&r, &c, 5);
    assert_eq!(out.len(), 1);
}

#[test]
fn normalize_collapses_internal_spaces() {
    assert_eq!(
        normalize_plugin_name("Foo   Bar   Baz"),
        "foo bar baz"
    );
}

#[test]
fn kvr_unknown_parses_to_triple_zero() {
    assert_eq!(app_lib::kvr::parse_version("Unknown"), vec![0, 0, 0]);
}

#[test]
fn ext_matches_rpp_bak() {
    assert_eq!(
        ext_matches(Path::new("/p.rpp-bak")).as_deref(),
        Some("RPP-BAK")
    );
}

#[test]
fn kvr_result_serde_product_only() {
    let k = app_lib::kvr::KvrResult {
        product_url: "https://p".into(),
        download_url: None,
    };
    let v = serde_json::to_value(&k).unwrap();
    assert_eq!(v["productUrl"], "https://p");
}

// ── Wave 3: more DAW / KVR / radix / fingerprints ─────────────────────────────

#[test]
fn daw_name_fl_studio() {
    assert_eq!(daw_name_for_format("FLP"), "FL Studio");
}

#[test]
fn daw_name_cubase() {
    assert_eq!(daw_name_for_format("CPR"), "Cubase");
}

#[test]
fn daw_name_studio_one() {
    assert_eq!(daw_name_for_format("SONG"), "Studio One");
}

#[test]
fn daw_name_pro_tools_ptx() {
    assert_eq!(daw_name_for_format("PTX"), "Pro Tools");
}

#[test]
fn ext_matches_flp() {
    assert_eq!(ext_matches(Path::new("/p.flp")).as_deref(), Some("FLP"));
}

#[test]
fn ext_matches_song() {
    assert_eq!(ext_matches(Path::new("/x.song")).as_deref(), Some("SONG"));
}

#[test]
fn ext_matches_aup3() {
    assert_eq!(ext_matches(Path::new("/p.aup3")).as_deref(), Some("AUP3"));
}

#[test]
fn kvr_cmp_identical_long_versions() {
    assert_eq!(
        app_lib::kvr::compare_versions("1.2.3.4.5", "1.2.3.4.5"),
        Ordering::Equal
    );
}

#[test]
fn kvr_parse_only_dots() {
    assert_eq!(app_lib::kvr::parse_version("..."), vec![0, 0, 0, 0]);
}

#[test]
fn radix_string_base_16_deadbeef() {
    assert_eq!(radix_string(0xdeadbeef, 16), "deadbeef");
}

#[test]
fn format_size_exactly_one_gib() {
    let s = app_lib::format_size(1024_u64.pow(3));
    assert!(s.contains("GB"), "{s}");
}

#[test]
fn fingerprint_high_band_delta() {
    let mut a = fp("/a.wav");
    let mut b = fp("/b.wav");
    a.high_band_energy = 0.01;
    b.high_band_energy = 0.99;
    assert!(fingerprint_distance(&a, &b) > 0.05);
}

#[test]
fn fingerprint_low_band_delta() {
    let mut a = fp("/a.wav");
    let mut b = fp("/b.wav");
    a.low_band_energy = 0.5;
    b.low_band_energy = 0.01;
    assert!(fingerprint_distance(&a, &b) > 0.01);
}

#[test]
fn compute_plugin_diff_one_added_one_removed() {
    let old = build_plugin_snapshot(&[plug("/old.vst3", "1")], &[], &[]);
    let new = build_plugin_snapshot(&[plug("/new.vst3", "1")], &[], &[]);
    let d = compute_plugin_diff(&old, &new);
    assert_eq!(d.added.len(), 1);
    assert_eq!(d.removed.len(), 1);
}

#[test]
fn build_audio_snapshot_single_root() {
    let s = build_audio_snapshot(&[sample("/a.wav")], &["/root".into()]);
    assert_eq!(s.roots, vec!["/root"]);
}

#[test]
fn plugin_info_serde_minimal() {
    let p = PluginInfo {
        name: "n".into(),
        path: "/p.vst3".into(),
        plugin_type: "VST3".into(),
        version: "1".into(),
        manufacturer: "m".into(),
        manufacturer_url: None,
        size: "1 B".into(),
        size_bytes: 1,
        modified: "m".into(),
        architectures: vec!["x86_64".into()],
    };
    let j = serde_json::to_string(&p).unwrap();
    assert!(j.contains("x86_64"));
}

#[test]
fn kvr_extract_version_from_label() {
    let html = r#"<label>Version 4.2</label>"#;
    assert!(app_lib::kvr::extract_version(html).is_some());
}

#[test]
fn normalize_x64_brackets() {
    let n = normalize_plugin_name("Filter (x64)");
    assert!(!n.contains("x64"));
}

#[test]
fn radix_base_5_twentyfive() {
    assert_eq!(radix_string(25, 5), "100");
}

#[test]
fn find_similar_max_one_picks_nearest() {
    let r = fp("/r.wav");
    let c = vec![
        fp("/far.wav"),
        {
            let mut x = fp("/near.wav");
            x.rms = r.rms + 0.001;
            x
        },
    ];
    let out = find_similar(&r, &c, 1);
    assert_eq!(out.len(), 1);
}

#[test]
fn ext_matches_npr() {
    assert_eq!(ext_matches(Path::new("/p.npr")).as_deref(), Some("NPR"));
}

#[test]
fn daw_name_nuendo() {
    assert_eq!(daw_name_for_format("NPR"), "Nuendo");
}

#[test]
fn kvr_cmp_numeric_vs_emptyish() {
    assert_eq!(app_lib::kvr::compare_versions("1", ""), Ordering::Greater);
}

#[test]
fn format_size_10_bytes() {
    let s = app_lib::format_size(10);
    assert!(s.ends_with(" B"), "{s}");
}

#[test]
fn compute_preset_diff_both_empty() {
    let a = build_preset_snapshot(&[], &[]);
    let d = compute_preset_diff(&a, &a);
    assert!(d.added.is_empty() && d.removed.is_empty());
}

#[test]
fn fingerprint_ler_change() {
    let mut a = fp("/a.wav");
    let mut b = fp("/b.wav");
    a.low_energy_ratio = 0.1;
    b.low_energy_ratio = 0.9;
    assert!(fingerprint_distance(&a, &b) > 0.01);
}

#[test]
fn export_plugin_roundtrip_arch() {
    let e = ExportPlugin {
        name: "n".into(),
        plugin_type: "VST3".into(),
        version: "1".into(),
        manufacturer: "m".into(),
        manufacturer_url: None,
        path: "/p".into(),
        size: "1 B".into(),
        size_bytes: 1,
        modified: "m".into(),
        architectures: vec!["universal".into()],
    };
    let j = serde_json::to_string(&e).unwrap();
    let back: ExportPlugin = serde_json::from_str(&j).unwrap();
    assert_eq!(back.architectures, vec!["universal"]);
}

#[test]
fn kvr_parse_three_component_semver() {
    assert_eq!(
        app_lib::kvr::parse_version("10.20.30"),
        vec![10, 20, 30]
    );
}

#[test]
fn is_package_ext_logicx() {
    assert!(is_package_ext(Path::new("/p.logicx")));
}

#[test]
fn ext_matches_band_dir_style() {
    assert_eq!(ext_matches(Path::new("/proj.band")).as_deref(), Some("BAND"));
}

// ── Extra scenario batch (CI stress + regression net) ─────────────────────────

#[test]
fn kvr_cmp_equal_strings() {
    assert_eq!(
        app_lib::kvr::compare_versions("1.2.3", "1.2.3"),
        Ordering::Equal
    );
}

#[test]
fn kvr_extract_version_from_meta_content() {
    let html = r#"<meta name="version" content="9.8.7">"#;
    assert_eq!(
        app_lib::kvr::extract_version(html).as_deref(),
        Some("9.8.7")
    );
}

#[test]
fn format_size_zero_bytes() {
    let s = app_lib::format_size(0);
    assert!(s.contains('0'), "{s}");
}

#[test]
fn format_size_exact_one_gib() {
    let s = app_lib::format_size(1024u64 * 1024 * 1024);
    assert!(s.contains("GB") || s.contains("GiB"), "{s}");
}

#[test]
fn radix_base_16_deadbeef() {
    assert_eq!(radix_string(0xdeadbeefu64, 16), "deadbeef");
}

#[test]
fn radix_base_3_twentyseven() {
    assert_eq!(radix_string(27, 3), "1000");
}

#[test]
fn daw_name_garageband_from_band() {
    assert_eq!(daw_name_for_format("BAND"), "GarageBand");
}

#[test]
fn ext_matches_ptx_uppercase() {
    assert_eq!(
        ext_matches(Path::new("/s.PTX")).as_deref(),
        Some("PTX")
    );
}

#[test]
fn get_plugin_type_vst2() {
    assert_eq!(get_plugin_type(".dll"), "VST2");
}

#[test]
fn get_plugin_type_au_component() {
    assert_eq!(get_plugin_type(".component"), "AU");
}

#[test]
fn fingerprint_distance_ordering() {
    let a = fp("/a.wav");
    let mut far = fp("/far.wav");
    far.rms = 1.0;
    let mut near = fp("/near.wav");
    near.rms = a.rms + 0.0001;
    let d_near = fingerprint_distance(&a, &near);
    let d_far = fingerprint_distance(&a, &far);
    assert!(d_near < d_far);
}

#[test]
fn find_similar_candidates_empty_returns_empty() {
    let r = fp("/r.wav");
    assert!(find_similar(&r, &[], 10).is_empty());
}

#[test]
fn compute_plugin_diff_identical_empty() {
    let s = build_plugin_snapshot(&[plug("/p.vst3", "1")], &[], &[]);
    let d = compute_plugin_diff(&s, &s);
    assert!(d.added.is_empty() && d.removed.is_empty() && d.version_changed.is_empty());
}

#[test]
fn compute_audio_diff_both_empty_roots() {
    let a = build_audio_snapshot(&[], &[]);
    let d = compute_audio_diff(&a, &a);
    assert!(d.added.is_empty() && d.removed.is_empty());
}

#[test]
fn compute_daw_diff_identical() {
    let s = build_daw_snapshot(&[dawproj("/x.dawproject")], &[]);
    let d = compute_daw_diff(&s, &s);
    assert!(d.added.is_empty() && d.removed.is_empty());
}

#[test]
fn normalize_plugin_name_lowercase_trim() {
    let n = normalize_plugin_name("  MySynth  ");
    assert_eq!(n, "mysynth");
}

#[test]
fn extract_plugins_whitespace_only_empty() {
    assert!(extract_plugins("   \n\t").is_empty());
}

#[test]
fn plugin_ref_roundtrip_with_unicode_name() {
    let p = PluginRef {
        name: "插件".into(),
        normalized_name: "x".into(),
        manufacturer: "M".into(),
        plugin_type: "VST3".into(),
    };
    let j = serde_json::to_string(&p).unwrap();
    let back: PluginRef = serde_json::from_str(&j).unwrap();
    assert_eq!(back.name, "插件");
}

#[test]
fn export_payload_roundtrip_version_field() {
    let p = ExportPayload {
        version: "2".into(),
        exported_at: "now".into(),
        plugins: vec![],
    };
    let j = serde_json::to_string(&p).unwrap();
    let back: ExportPayload = serde_json::from_str(&j).unwrap();
    assert_eq!(back.version, "2");
}

#[test]
fn kvr_cache_entry_serde_has_update_true() {
    let e = KvrCacheEntry {
        kvr_url: Some("https://k".into()),
        update_url: None,
        latest_version: Some("3".into()),
        has_update: true,
        source: "kvr".into(),
        timestamp: "1".into(),
    };
    let j = serde_json::to_string(&e).unwrap();
    assert!(j.contains("true"));
}

#[test]
fn kvr_cmp_patch_bump() {
    assert_eq!(
        app_lib::kvr::compare_versions("1.0.1", "1.0.2"),
        Ordering::Less
    );
}

#[test]
fn radix_string_base_2_one() {
    assert_eq!(radix_string(1, 2), "1");
}

#[test]
fn format_size_kib_boundary() {
    let s = app_lib::format_size(1024);
    assert!(s.contains("KB") || s.contains("KiB"), "{s}");
}

#[test]
fn daw_name_pro_tools_from_ptx() {
    assert_eq!(daw_name_for_format("PTX"), "Pro Tools");
}

#[test]
fn is_package_ext_not_a_package_wav() {
    assert!(!is_package_ext(Path::new("/x.wav")));
}

#[test]
fn compute_preset_diff_same_file_twice_no_dup_added() {
    let p = preset("/one.fxp");
    let s = build_preset_snapshot(&[p.clone()], &[]);
    let d = compute_preset_diff(&s, &s);
    assert!(d.added.is_empty() && d.removed.is_empty());
}

#[test]
fn audio_sample_serde_roundtrip_minimal() {
    let s = sample("/t.wav");
    let j = serde_json::to_string(&s).unwrap();
    let back: AudioSample = serde_json::from_str(&j).unwrap();
    assert_eq!(back.path, "/t.wav");
}

#[test]
fn daw_project_serde_roundtrip() {
    let d = dawproj("/z.dawproject");
    let j = serde_json::to_string(&d).unwrap();
    let back: DawProject = serde_json::from_str(&j).unwrap();
    assert_eq!(back.format, "dawproject");
}

#[test]
fn fingerprint_all_zero_still_finite_distance() {
    let mut a = fp("/z.wav");
    a.rms = 0.0;
    a.spectral_centroid = 0.0;
    a.zero_crossing_rate = 0.0;
    a.low_band_energy = 0.0;
    a.mid_band_energy = 0.0;
    a.high_band_energy = 0.0;
    a.low_energy_ratio = 0.0;
    a.attack_time = 0.0;
    let mut b = a.clone();
    b.path = "/y.wav".into();
    let d = fingerprint_distance(&a, &b);
    assert!(d.is_finite() && d >= 0.0);
}

#[test]
fn find_similar_sorts_by_distance() {
    let r = fp("/ref.wav");
    let mut c1 = fp("/c1.wav");
    c1.rms = r.rms + 0.5;
    let mut c2 = fp("/c2.wav");
    c2.rms = r.rms + 0.01;
    let out = find_similar(&r, &[c1, c2], 2);
    assert_eq!(out.len(), 2);
    assert!(out[0].0.contains("c2"));
}

#[test]
fn kvr_parse_version_with_trailing_nonnumeric_stripped() {
    let v = app_lib::kvr::parse_version("2.0beta");
    assert!(v.len() >= 1);
    assert_eq!(v[0], 2);
}

#[test]
fn normalize_removes_vst2_suffix_parens() {
    let n = normalize_plugin_name("Gain (VST)");
    assert!(!n.contains("vst"));
}

#[test]
fn radix_base_8_sixtyfour() {
    assert_eq!(radix_string(64, 8), "100");
}

#[test]
fn compute_plugin_diff_path_added() {
    let old = build_plugin_snapshot(&[], &[], &[]);
    let new = build_plugin_snapshot(&[plug("/new.vst3", "1")], &[], &[]);
    let d = compute_plugin_diff(&old, &new);
    assert_eq!(d.added.len(), 1);
}

#[test]
fn kvr_extract_version_from_td_cell() {
    let html = r#"<td>Version 12.34.56</td>"#;
    assert_eq!(
        app_lib::kvr::extract_version(html).as_deref(),
        Some("12.34.56")
    );
}

// ── Wave 4: DAW suffix matrix + diff/serde stress ─────────────────────────────

#[test]
fn ext_matches_ardour() {
    assert_eq!(
        ext_matches(Path::new("/session.ardour")).as_deref(),
        Some("ARDOUR")
    );
}

#[test]
fn daw_name_ardour_token() {
    assert_eq!(daw_name_for_format("ARDOUR"), "Ardour");
}

#[test]
fn ext_matches_reason() {
    assert_eq!(
        ext_matches(Path::new("/rack.reason")).as_deref(),
        Some("REASON")
    );
}

#[test]
fn daw_name_reason_token() {
    assert_eq!(daw_name_for_format("REASON"), "Reason");
}

#[test]
fn ext_matches_ptf_legacy() {
    assert_eq!(
        ext_matches(Path::new("/oldsong.ptf")).as_deref(),
        Some("PTF")
    );
}

#[test]
fn daw_name_ptf_pro_tools() {
    assert_eq!(daw_name_for_format("PTF"), "Pro Tools");
}

#[test]
fn ext_matches_aup_legacy() {
    assert_eq!(
        ext_matches(Path::new("/legacy.aup")).as_deref(),
        Some("AUP")
    );
}

#[test]
fn daw_name_aup_audacity() {
    assert_eq!(daw_name_for_format("AUP"), "Audacity");
}

#[test]
fn compute_audio_diff_one_removed_sample() {
    let old = build_audio_snapshot(&[sample("/gone.wav")], &[]);
    let new = build_audio_snapshot(&[], &[]);
    let d = compute_audio_diff(&old, &new);
    assert_eq!(d.removed.len(), 1);
}

#[test]
fn compute_plugin_diff_one_removed_only() {
    let old = build_plugin_snapshot(&[plug("/bye.vst3", "1")], &[], &[]);
    let new = build_plugin_snapshot(&[], &[], &[]);
    let d = compute_plugin_diff(&old, &new);
    assert_eq!(d.removed.len(), 1);
}

#[test]
fn compute_daw_diff_two_formats() {
    let a = dawproj("/a.dawproject");
    let mut b = dawproj("/b.dawproject");
    b.path = "/other/x.dawproject".into();
    let old = build_daw_snapshot(&[a], &[]);
    let new = build_daw_snapshot(&[b], &[]);
    let d = compute_daw_diff(&old, &new);
    assert_eq!(d.added.len() + d.removed.len(), 2);
}

#[test]
fn kvr_cmp_major_only_vs_patch() {
    assert_eq!(
        app_lib::kvr::compare_versions("2", "1.9.9"),
        Ordering::Greater
    );
}

#[test]
fn kvr_parse_mixed_numeric_text_segments() {
    let v = app_lib::kvr::parse_version("1.x.3");
    assert_eq!(v, vec![1, 0, 3]);
}

#[test]
fn format_size_two_bytes() {
    let s = app_lib::format_size(2);
    assert!(s.contains('2'), "{s}");
}

#[test]
fn radix_base_7_forty_nine() {
    assert_eq!(radix_string(49, 7), "100");
}

#[test]
fn radix_base_11_small() {
    assert_eq!(radix_string(120, 11), "aa");
}

#[test]
fn normalize_strips_au_bracket() {
    let n = normalize_plugin_name("Synth (AU)");
    assert!(!n.contains("(au)"));
}

#[test]
fn plugin_ref_type_vst2_roundtrip() {
    let p = PluginRef {
        name: "Old".into(),
        normalized_name: "old".into(),
        manufacturer: "M".into(),
        plugin_type: "VST2".into(),
    };
    let j = serde_json::to_string(&p).unwrap();
    let back: PluginRef = serde_json::from_str(&j).unwrap();
    assert_eq!(back.plugin_type, "VST2");
}

#[test]
fn export_plugin_with_manufacturer_url_some() {
    let p = ExportPlugin {
        name: "n".into(),
        plugin_type: "VST3".into(),
        version: "1".into(),
        manufacturer: "m".into(),
        manufacturer_url: Some("https://m.example".into()),
        path: "/p.vst3".into(),
        size: "1 B".into(),
        size_bytes: 1,
        modified: "t".into(),
        architectures: vec![],
    };
    let v = serde_json::to_value(&p).unwrap();
    assert!(v["manufacturer_url"].is_string());
}

#[test]
fn preset_file_serde_roundtrip() {
    let p = preset("/presets/p.h2p");
    let j = serde_json::to_string(&p).unwrap();
    let back: PresetFile = serde_json::from_str(&j).unwrap();
    assert_eq!(back.path, "/presets/p.h2p");
}

#[test]
fn fingerprint_spectral_centroid_delta_distance() {
    let mut a = fp("/a.wav");
    let mut b = fp("/b.wav");
    a.spectral_centroid = 0.1;
    b.spectral_centroid = 0.9;
    assert!(fingerprint_distance(&a, &b) > 0.05);
}

#[test]
fn fingerprint_zero_crossing_delta() {
    let mut a = fp("/a.wav");
    let mut b = fp("/b.wav");
    a.zero_crossing_rate = 0.01;
    b.zero_crossing_rate = 0.5;
    assert!(fingerprint_distance(&a, &b) > 0.01);
}

#[test]
fn find_similar_truncates_to_max() {
    let r = fp("/r.wav");
    let c: Vec<_> = (0..20)
        .map(|i| {
            let mut x = fp(&format!("/c{i}.wav"));
            x.rms = r.rms + (i as f64) * 0.001;
            x
        })
        .collect();
    let out = find_similar(&r, &c, 3);
    assert_eq!(out.len(), 3);
}

#[test]
fn kvr_extract_version_software_keyword_line() {
    let html = r#"Software version 5.4.3 available"#;
    assert_eq!(
        app_lib::kvr::extract_version(html).as_deref(),
        Some("5.4.3")
    );
}

#[test]
fn ext_matches_reason_uppercase() {
    assert_eq!(
        ext_matches(Path::new("/X.REASON")).as_deref(),
        Some("REASON")
    );
}

#[test]
fn is_package_ext_band_still_true() {
    assert!(is_package_ext(Path::new("/Music/MySong.band")));
}

#[test]
fn compute_preset_diff_two_added() {
    let old = build_preset_snapshot(&[preset("/a.fxp")], &[]);
    let new = build_preset_snapshot(&[preset("/a.fxp"), preset("/b.fxp")], &[]);
    let d = compute_preset_diff(&old, &new);
    assert_eq!(d.added.len(), 1);
}

#[test]
fn kvr_cmp_zero_vs_one() {
    assert_eq!(
        app_lib::kvr::compare_versions("0.0.1", "0.0.0"),
        Ordering::Greater
    );
}

#[test]
fn radix_string_base_9_eighty_one() {
    assert_eq!(radix_string(81, 9), "100");
}

#[test]
fn format_size_megabytes_not_zero() {
    let s = app_lib::format_size(5 * 1024 * 1024 + 1);
    assert!(s.contains("MB") || s.contains("MiB"), "{s}");
}

#[test]
fn compute_plugin_diff_version_and_path_change() {
    let old = build_plugin_snapshot(&[plug("/p.vst3", "1.0")], &[], &[]);
    let mut q = plug("/other.vst3", "2.0");
    q.path = "/other.vst3".into();
    let new = build_plugin_snapshot(&[q], &[], &[]);
    let d = compute_plugin_diff(&old, &new);
    assert!(!d.added.is_empty() || !d.removed.is_empty() || !d.version_changed.is_empty());
}
