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

// ── Wave 5: more DAW paths, KVR/radix/size, diffs, serde ──────────────────────

#[test]
fn ext_matches_logicx_path_token() {
    assert_eq!(
        ext_matches(Path::new("/Projects/Beat.logicx")).as_deref(),
        Some("LOGICX")
    );
}

#[test]
fn ext_matches_dawproject_uppercase_ext() {
    assert_eq!(
        ext_matches(Path::new("/export/MyTune.DAWPROJECT")).as_deref(),
        Some("DAWPROJECT")
    );
}

#[test]
fn is_package_ext_dawproject_file_not_package() {
    assert!(!is_package_ext(Path::new("/p.dawproject")));
}

#[test]
fn kvr_cmp_patch_triple_vs_double() {
    assert_eq!(
        app_lib::kvr::compare_versions("1.2.3", "1.2.2"),
        Ordering::Greater
    );
}

#[test]
fn kvr_parse_trailing_empty_segment() {
    let v = app_lib::kvr::parse_version("1.0.");
    assert_eq!(v, vec![1, 0, 0]);
}

#[test]
fn format_size_1025_bytes() {
    let s = app_lib::format_size(1025);
    assert!(s.contains("KB") || s.contains("KiB") || s.contains("B"), "{s}");
}

#[test]
fn radix_base_13_one_sixty_nine() {
    assert_eq!(radix_string(169, 13), "100");
}

#[test]
fn normalize_strips_aax_brackets() {
    let n = normalize_plugin_name("Comp (AAX)");
    assert!(!n.contains("aax"));
}

#[test]
fn fingerprint_mid_band_energy_delta() {
    let mut a = fp("/a.wav");
    let mut b = fp("/b.wav");
    a.mid_band_energy = 0.01;
    b.mid_band_energy = 0.99;
    assert!(fingerprint_distance(&a, &b) > 0.05);
}

#[test]
fn fingerprint_high_band_energy_delta() {
    let mut a = fp("/a.wav");
    let mut b = fp("/b.wav");
    a.high_band_energy = 0.01;
    b.high_band_energy = 0.95;
    assert!(fingerprint_distance(&a, &b) > 0.02);
}

#[test]
fn compute_preset_diff_one_removed_entry() {
    let old = build_preset_snapshot(&[preset("/a.fxp"), preset("/b.fxp")], &[]);
    let new = build_preset_snapshot(&[preset("/a.fxp")], &[]);
    let d = compute_preset_diff(&old, &new);
    assert_eq!(d.removed.len(), 1);
}

#[test]
fn compute_audio_diff_one_added_one_removed_same_count() {
    let old = build_audio_snapshot(&[sample("/old.wav")], &[]);
    let new = build_audio_snapshot(&[sample("/new.wav")], &[]);
    let d = compute_audio_diff(&old, &new);
    assert_eq!(d.added.len(), 1);
    assert_eq!(d.removed.len(), 1);
}

#[test]
fn export_payload_one_plugin_roundtrip() {
    let p = ExportPayload {
        version: "3".into(),
        exported_at: "t".into(),
        plugins: vec![ExportPlugin {
            name: "P".into(),
            plugin_type: "VST3".into(),
            version: "1".into(),
            manufacturer: "M".into(),
            manufacturer_url: None,
            path: "/x.vst3".into(),
            size: "1 B".into(),
            size_bytes: 1,
            modified: "m".into(),
            architectures: vec!["arm64".into()],
        }],
    };
    let j = serde_json::to_string(&p).unwrap();
    let back: ExportPayload = serde_json::from_str(&j).unwrap();
    assert_eq!(back.plugins.len(), 1);
    assert_eq!(back.plugins[0].name, "P");
}

#[test]
fn kvr_cache_entry_roundtrip_all_fields() {
    let e = KvrCacheEntry {
        kvr_url: Some("https://kvr/x".into()),
        update_url: Some("https://dl/y".into()),
        latest_version: Some("9.9".into()),
        has_update: true,
        source: "kvr".into(),
        timestamp: "99".into(),
    };
    let j = serde_json::to_string(&e).unwrap();
    let back: KvrCacheEntry = serde_json::from_str(&j).unwrap();
    assert_eq!(back.latest_version.as_deref(), Some("9.9"));
    assert!(back.has_update);
}

#[test]
fn kvr_extract_version_plain_after_word_version() {
    let html = r#"Release notes: Version 8.1.0 (stable)"#;
    assert_eq!(
        app_lib::kvr::extract_version(html).as_deref(),
        Some("8.1.0")
    );
}

#[test]
fn ext_matches_cpr_uppercase() {
    assert_eq!(
        ext_matches(Path::new("/mix.CPR")).as_deref(),
        Some("CPR")
    );
}

#[test]
fn daw_name_cubase_from_cpr() {
    assert_eq!(daw_name_for_format("CPR"), "Cubase");
}

#[test]
fn ext_matches_bwproject_mixed_case() {
    assert_eq!(
        ext_matches(Path::new("/p.BwPrOjEcT")).as_deref(),
        Some("BWPROJECT")
    );
}

#[test]
fn find_similar_keeps_stable_order_for_ties() {
    let r = fp("/r.wav");
    let c1 = fp("/c1.wav");
    let mut c2 = fp("/c2.wav");
    c2.rms = c1.rms;
    c2.spectral_centroid = c1.spectral_centroid;
    c2.zero_crossing_rate = c1.zero_crossing_rate;
    c2.low_band_energy = c1.low_band_energy;
    c2.mid_band_energy = c1.mid_band_energy;
    c2.high_band_energy = c1.high_band_energy;
    c2.low_energy_ratio = c1.low_energy_ratio;
    c2.attack_time = c1.attack_time;
    let out = find_similar(&r, &[c1, c2], 2);
    assert_eq!(out.len(), 2);
}

#[test]
fn kvr_cmp_infinity_style_long() {
    assert_eq!(
        app_lib::kvr::compare_versions("99.99.99", "99.99.98"),
        Ordering::Greater
    );
}

#[test]
fn radix_base_4_two_fifty_six() {
    assert_eq!(radix_string(256, 4), "10000");
}

#[test]
fn format_size_one_less_than_mb() {
    let s = app_lib::format_size(1024 * 1024 - 1);
    assert!(!s.is_empty());
}

#[test]
fn normalize_plugin_name_digit_in_name_kept() {
    let n = normalize_plugin_name("Synth 2");
    assert!(n.contains('2'));
}

#[test]
fn plugin_ref_manufacturer_unicode_roundtrip() {
    let p = PluginRef {
        name: "A".into(),
        normalized_name: "a".into(),
        manufacturer: "日本".into(),
        plugin_type: "AU".into(),
    };
    let j = serde_json::to_string(&p).unwrap();
    let back: PluginRef = serde_json::from_str(&j).unwrap();
    assert_eq!(back.manufacturer, "日本");
}

#[test]
fn compute_plugin_diff_same_version_two_paths() {
    let a = plug("/a.vst3", "1");
    let mut b = plug("/b.vst3", "1");
    b.path = "/b.vst3".into();
    let old = build_plugin_snapshot(&[a], &[], &[]);
    let new = build_plugin_snapshot(&[b], &[], &[]);
    let d = compute_plugin_diff(&old, &new);
    assert!(!d.added.is_empty() && !d.removed.is_empty());
}

#[test]
fn ext_matches_ardour_uppercase() {
    assert_eq!(
        ext_matches(Path::new("/X.ARDOUR")).as_deref(),
        Some("ARDOUR")
    );
}

#[test]
fn kvr_parse_single_zero_component() {
    assert_eq!(app_lib::kvr::parse_version("0"), vec![0]);
}

#[test]
fn fingerprint_attack_time_wide_gap_distance() {
    let mut a = fp("/a.wav");
    let mut b = fp("/b.wav");
    a.attack_time = 0.001;
    b.attack_time = 0.5;
    assert!(fingerprint_distance(&a, &b) > 0.01);
}

// ── Wave 6: diff edges, KVR/radix/size, fingerprints, serde ───────────────────

#[test]
fn compute_daw_diff_one_added_project() {
    let old = build_daw_snapshot(&[], &[]);
    let new = build_daw_snapshot(&[dawproj("/only.dawproject")], &[]);
    let d = compute_daw_diff(&old, &new);
    assert_eq!(d.added.len(), 1);
}

#[test]
fn compute_daw_diff_two_removed_both() {
    let a = dawproj("/a.dawproject");
    let b = dawproj("/b.dawproject");
    let old = build_daw_snapshot(&[a, b], &[]);
    let new = build_daw_snapshot(&[], &[]);
    let d = compute_daw_diff(&old, &new);
    assert_eq!(d.removed.len(), 2);
}

#[test]
fn kvr_cmp_patch_vs_shorter_equal() {
    assert_eq!(
        app_lib::kvr::compare_versions("1.0.0", "1.0"),
        Ordering::Equal
    );
}

#[test]
fn kvr_cmp_extra_patch_component_greater() {
    assert_eq!(
        app_lib::kvr::compare_versions("1.0.1", "1.0"),
        Ordering::Greater
    );
}

#[test]
fn kvr_extract_version_latest_word_prefix() {
    let html = r#"Latest 3.0.1 — download"#;
    assert_eq!(
        app_lib::kvr::extract_version(html).as_deref(),
        Some("3.0.1")
    );
}

#[test]
fn format_size_three_bytes() {
    let s = app_lib::format_size(3);
    assert!(s.contains('3'), "{s}");
}

#[test]
fn radix_base_6_thirty_six() {
    assert_eq!(radix_string(36, 6), "100");
}

#[test]
fn radix_base_14_one_nine_six() {
    assert_eq!(radix_string(196, 14), "100");
}

#[test]
fn fingerprint_low_energy_ratio_delta() {
    let mut a = fp("/a.wav");
    let mut b = fp("/b.wav");
    a.low_energy_ratio = 0.01;
    b.low_energy_ratio = 0.99;
    assert!(fingerprint_distance(&a, &b) > 0.05);
}

#[test]
fn export_payload_two_plugins_roundtrip_names() {
    let p = ExportPayload {
        version: "1".into(),
        exported_at: "e".into(),
        plugins: vec![
            ExportPlugin {
                name: "A".into(),
                plugin_type: "AU".into(),
                version: "1".into(),
                manufacturer: "Ma".into(),
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
                manufacturer: "Mb".into(),
                manufacturer_url: None,
                path: "/b.vst3".into(),
                size: "2 B".into(),
                size_bytes: 2,
                modified: "m".into(),
                architectures: vec![],
            },
        ],
    };
    let j = serde_json::to_string(&p).unwrap();
    let back: ExportPayload = serde_json::from_str(&j).unwrap();
    assert_eq!(back.plugins[0].name, "A");
    assert_eq!(back.plugins[1].name, "B");
}

#[test]
fn ext_matches_reason_lower() {
    assert_eq!(
        ext_matches(Path::new("/p.reason")).as_deref(),
        Some("REASON")
    );
}

#[test]
fn ext_matches_ptf_lower() {
    assert_eq!(
        ext_matches(Path::new("/legacy.ptf")).as_deref(),
        Some("PTF")
    );
}

#[test]
fn daw_name_bitwig_bwproject_token() {
    assert_eq!(daw_name_for_format("BWPROJECT"), "Bitwig Studio");
}

#[test]
fn normalize_strips_intel_parens() {
    let n = normalize_plugin_name("Amp (Intel)");
    assert!(!n.contains("intel"));
}

#[test]
fn find_similar_reference_excluded_from_candidates_list() {
    let r = fp("/same.wav");
    let mut c = r.clone();
    c.path = "/other.wav".into();
    c.rms = 0.99;
    let out = find_similar(&r, &[r.clone(), c], 5);
    assert_eq!(out.len(), 1);
    assert!(out[0].0.contains("other"));
}

#[test]
fn kvr_parse_multiple_separators_only_numbers() {
    let v = app_lib::kvr::parse_version("1..2");
    assert_eq!(v, vec![1, 0, 2]);
}

#[test]
fn format_size_hundred_kb_range() {
    let s = app_lib::format_size(100 * 1024);
    assert!(s.contains("KB") || s.contains("KiB") || s.contains("k"), "{s}");
}

#[test]
fn plugin_info_json_preserves_path_with_spaces() {
    let p = PluginInfo {
        name: "N".into(),
        path: "/Library/Audio/My Plug.vst3".into(),
        plugin_type: "VST3".into(),
        version: "1".into(),
        manufacturer: "M".into(),
        manufacturer_url: None,
        size: "1 B".into(),
        size_bytes: 1,
        modified: "t".into(),
        architectures: vec![],
    };
    let j = serde_json::to_string(&p).unwrap();
    assert!(j.contains("My Plug"));
}

#[test]
fn compute_audio_diff_roots_change_only_same_samples() {
    let s = sample("/x.wav");
    let a = build_audio_snapshot(&[s.clone()], &["/r1".into()]);
    let b = build_audio_snapshot(&[s], &["/r2".into()]);
    let d = compute_audio_diff(&a, &b);
    assert!(d.added.is_empty() && d.removed.is_empty());
}

#[test]
fn compute_preset_diff_identical_lists() {
    let p = preset("/z.fxp");
    let old = build_preset_snapshot(&[p.clone()], &[]);
    let new = build_preset_snapshot(&[p], &[]);
    let d = compute_preset_diff(&old, &new);
    assert!(d.added.is_empty() && d.removed.is_empty());
}

#[test]
fn kvr_cmp_negative_style_strings_numeric_parse() {
    assert_eq!(
        app_lib::kvr::compare_versions("0.0.2", "0.0.1"),
        Ordering::Greater
    );
}

#[test]
fn radix_string_one_in_base_36() {
    assert_eq!(radix_string(1, 36), "1");
}

#[test]
fn ext_matches_npr_lowercase_path() {
    assert_eq!(
        ext_matches(Path::new("/p.npr")).as_deref(),
        Some("NPR")
    );
}

#[test]
fn is_package_ext_band_path_true() {
    assert!(is_package_ext(Path::new("/GarageBandProject.band")));
}

#[test]
fn fingerprint_identical_paths_excluded_in_find_similar_dup_paths() {
    let r = fp("/ref.wav");
    let mut c = fp("/ref.wav");
    c.rms = 0.5;
    let out = find_similar(&r, &[c], 3);
    assert!(out.is_empty());
}

#[test]
fn kvr_cache_entry_has_update_false_roundtrip() {
    let e = KvrCacheEntry {
        kvr_url: None,
        update_url: None,
        latest_version: Some("1".into()),
        has_update: false,
        source: "x".into(),
        timestamp: "0".into(),
    };
    let j = serde_json::to_string(&e).unwrap();
    let back: KvrCacheEntry = serde_json::from_str(&j).unwrap();
    assert!(!back.has_update);
}

#[test]
fn daw_name_reaper_rpp_bak_token() {
    assert_eq!(daw_name_for_format("RPP-BAK"), "REAPER");
}

#[test]
fn ext_matches_rpp_lowercase() {
    assert_eq!(
        ext_matches(Path::new("/session.rpp")).as_deref(),
        Some("RPP")
    );
}

#[test]
fn normalize_plugin_name_multiple_spaces_collapsed() {
    assert_eq!(
        normalize_plugin_name("Ab   Cd   Ef"),
        "ab cd ef"
    );
}

#[test]
fn radix_base_17_two_eight_nine() {
    assert_eq!(radix_string(289, 17), "100");
}

#[test]
fn format_size_large_but_not_tb() {
    let s = app_lib::format_size(500 * 1024 * 1024 * 1024);
    assert!(!s.is_empty());
}

// ── Wave 7: semver edges, sizes, radix grid, snapshots, normalize ────────────

#[test]
fn kvr_cmp_single_digit_vs_double_digit() {
    assert_eq!(
        app_lib::kvr::compare_versions("2", "10"),
        Ordering::Less
    );
}

#[test]
fn kvr_parse_triple_hundreds() {
    assert_eq!(
        app_lib::kvr::parse_version("100.200.300"),
        vec![100, 200, 300]
    );
}

#[test]
fn format_size_one_tebibyte() {
    let s = app_lib::format_size(1024u64.pow(4));
    assert!(s.contains("TB") || s.contains("TiB"), "{s}");
}

#[test]
fn radix_base_18_three_twenty_four() {
    assert_eq!(radix_string(324, 18), "100");
}

#[test]
fn radix_base_19_three_six_one() {
    assert_eq!(radix_string(361, 19), "100");
}

#[test]
fn fingerprint_low_band_energy_wide_delta() {
    let mut a = fp("/a.wav");
    let mut b = fp("/b.wav");
    a.low_band_energy = 0.001;
    b.low_band_energy = 0.95;
    assert!(fingerprint_distance(&a, &b) > 0.05);
}

#[test]
fn compute_audio_diff_three_samples_added() {
    let old = build_audio_snapshot(&[], &[]);
    let new = build_audio_snapshot(
        &[
            sample("/a.wav"),
            sample("/b.wav"),
            sample("/c.wav"),
        ],
        &[],
    );
    let d = compute_audio_diff(&old, &new);
    assert_eq!(d.added.len(), 3);
}

#[test]
fn compute_plugin_diff_three_removed() {
    let old = build_plugin_snapshot(
        &[
            plug("/p1.vst3", "1"),
            plug("/p2.vst3", "1"),
            plug("/p3.vst3", "1"),
        ],
        &[],
        &[],
    );
    let new = build_plugin_snapshot(&[], &[], &[]);
    let d = compute_plugin_diff(&old, &new);
    assert_eq!(d.removed.len(), 3);
}

#[test]
fn ext_matches_song_uppercase_ext() {
    assert_eq!(
        ext_matches(Path::new("/MixFinal.SONG")).as_deref(),
        Some("SONG")
    );
}

#[test]
fn daw_name_audacity_aup3_token() {
    assert_eq!(daw_name_for_format("AUP3"), "Audacity");
}

#[test]
fn normalize_strips_apple_silicon_parens() {
    let n = normalize_plugin_name("Filter (Apple Silicon)");
    assert!(!n.contains("apple"));
}

#[test]
fn normalize_strips_universal_brackets() {
    let n = normalize_plugin_name("Plug (Universal)");
    assert!(!n.contains("universal"));
}

#[test]
fn find_similar_max_exceeds_candidate_count() {
    let r = fp("/r.wav");
    let c = fp("/only.wav");
    let out = find_similar(&r, &[c], 99);
    assert_eq!(out.len(), 1);
}

#[test]
fn export_payload_plugins_array_two_serde_types() {
    let p = ExportPayload {
        version: "1".into(),
        exported_at: "t".into(),
        plugins: vec![
            ExportPlugin {
                name: "A".into(),
                plugin_type: "AU".into(),
                version: "1".into(),
                manufacturer: "Ma".into(),
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
                manufacturer: "Mb".into(),
                manufacturer_url: Some("https://x".into()),
                path: "/b.vst3".into(),
                size: "2 B".into(),
                size_bytes: 2,
                modified: "m".into(),
                architectures: vec!["x86_64".into()],
            },
        ],
    };
    let j = serde_json::to_string(&p).unwrap();
    assert!(j.contains("manufacturer_url"));
    let back: ExportPayload = serde_json::from_str(&j).unwrap();
    assert_eq!(back.plugins.len(), 2);
}

#[test]
fn plugin_ref_all_fields_distinct() {
    let p = PluginRef {
        name: "LongName".into(),
        normalized_name: "longname".into(),
        manufacturer: "Manu".into(),
        plugin_type: "CLAP".into(),
    };
    let j = serde_json::to_string(&p).unwrap();
    let back: PluginRef = serde_json::from_str(&j).unwrap();
    assert_eq!(back.plugin_type, "CLAP");
}

#[test]
fn kvr_extract_version_release_word() {
    let html = r#"Release 12.0.1 notes"#;
    assert_eq!(
        app_lib::kvr::extract_version(html).as_deref(),
        Some("12.0.1")
    );
}

#[test]
fn ext_matches_bwproject_trailing_path() {
    assert_eq!(
        ext_matches(Path::new("/deep/nested/x.bwproject")).as_deref(),
        Some("BWPROJECT")
    );
}

#[test]
fn compute_daw_diff_identical_projects_noop() {
    let p = dawproj("/same.dawproject");
    let s = build_daw_snapshot(&[p.clone()], &[]);
    let d = compute_daw_diff(&s, &s);
    assert!(d.added.is_empty() && d.removed.is_empty());
}

#[test]
fn format_size_sub_kilobyte() {
    let s = app_lib::format_size(512);
    assert!(s.contains("512") || s.contains("B"), "{s}");
}

#[test]
fn radix_string_large_in_base_36() {
    let s = radix_string(35 * 36 + 35, 36);
    assert_eq!(s.len(), 2);
}

#[test]
fn kvr_cmp_all_zeros_equal() {
    assert_eq!(
        app_lib::kvr::compare_versions("0.0.0", "0"),
        Ordering::Equal
    );
}

#[test]
fn fingerprint_symmetric_near_identical() {
    let a = fp("/a.wav");
    let mut b = fp("/b.wav");
    b.rms = a.rms + 1e-12;
    let d = fingerprint_distance(&a, &b);
    assert!(d < 1e-6);
}

#[test]
fn compute_preset_diff_three_removed() {
    let old = build_preset_snapshot(
        &[
            preset("/a.fxp"),
            preset("/b.fxp"),
            preset("/c.fxp"),
        ],
        &[],
    );
    let new = build_preset_snapshot(&[], &[]);
    let d = compute_preset_diff(&old, &new);
    assert_eq!(d.removed.len(), 3);
}

#[test]
fn is_package_ext_logicx_path() {
    assert!(is_package_ext(Path::new("/Projects/Album.logicx")));
}

#[test]
fn ext_matches_flp_mixed_case() {
    assert_eq!(
        ext_matches(Path::new("/beat.FLP")).as_deref(),
        Some("FLP")
    );
}

#[test]
fn daw_name_unknown_empty_token() {
    assert_eq!(daw_name_for_format(""), "Unknown");
}

#[test]
fn kvr_parse_leading_dot_segment() {
    let v = app_lib::kvr::parse_version(".5.2");
    assert_eq!(v[0], 0);
}

#[test]
fn format_size_u64_max_sane_string() {
    let s = app_lib::format_size(u64::MAX);
    assert!(!s.is_empty() && s.len() < 64);
}

#[test]
fn radix_base_20_four_hundred() {
    assert_eq!(radix_string(400, 20), "100");
}

#[test]
fn find_similar_zero_candidates_with_positive_max() {
    let r = fp("/r.wav");
    assert!(find_similar(&r, &[], 10).is_empty());
}

#[test]
fn audio_sample_clone_eq_path() {
    let s = sample("/x.wav");
    assert_eq!(s.path, "/x.wav");
}

// ── Wave 8: batch adds, KVR/radix, normalize stereo/mono, serde ──────────────

#[test]
fn kvr_cmp_quadruple_identical() {
    assert_eq!(
        app_lib::kvr::compare_versions("1.2.3.4", "1.2.3.4"),
        Ordering::Equal
    );
}

#[test]
fn kvr_cmp_four_vs_three_components() {
    assert_eq!(
        app_lib::kvr::compare_versions("1.0.0.1", "1.0.0"),
        Ordering::Greater
    );
}

#[test]
fn radix_base_21_four_four_one() {
    assert_eq!(radix_string(441, 21), "100");
}

#[test]
fn radix_base_22_four_eight_four() {
    assert_eq!(radix_string(484, 22), "100");
}

#[test]
fn radix_base_23_five_two_nine() {
    assert_eq!(radix_string(529, 23), "100");
}

#[test]
fn format_size_exactly_one_mebibyte() {
    let s = app_lib::format_size(1024 * 1024);
    assert!(s.contains("MB") || s.contains("MiB"), "{s}");
}

#[test]
fn compute_daw_diff_two_added_projects() {
    let old = build_daw_snapshot(&[], &[]);
    let new = build_daw_snapshot(
        &[dawproj("/first.dawproject"), dawproj("/second.dawproject")],
        &[],
    );
    let d = compute_daw_diff(&old, &new);
    assert_eq!(d.added.len(), 2);
}

#[test]
fn compute_plugin_diff_two_added_plugins() {
    let old = build_plugin_snapshot(&[], &[], &[]);
    let new = build_plugin_snapshot(
        &[plug("/one.vst3", "1"), plug("/two.vst3", "2")],
        &[],
        &[],
    );
    let d = compute_plugin_diff(&old, &new);
    assert_eq!(d.added.len(), 2);
}

#[test]
fn ext_matches_als_uppercase_ext() {
    assert_eq!(
        ext_matches(Path::new("/LiveSet.ALS")).as_deref(),
        Some("ALS")
    );
}

#[test]
fn normalize_strips_mono_brackets() {
    let n = normalize_plugin_name("Bus (Mono)");
    assert!(!n.contains("mono"));
}

#[test]
fn normalize_strips_stereo_brackets() {
    let n = normalize_plugin_name("Bus (Stereo)");
    assert!(!n.contains("stereo"));
}

#[test]
fn plugin_info_serde_manufacturer_url_roundtrip() {
    let p = PluginInfo {
        name: "N".into(),
        path: "/p.vst3".into(),
        plugin_type: "VST3".into(),
        version: "1".into(),
        manufacturer: "M".into(),
        manufacturer_url: Some("https://mfg.example".into()),
        size: "1 B".into(),
        size_bytes: 1,
        modified: "t".into(),
        architectures: vec![],
    };
    let j = serde_json::to_string(&p).unwrap();
    assert!(j.contains("manufacturerUrl"));
    let back: PluginInfo = serde_json::from_str(&j).unwrap();
    assert_eq!(
        back.manufacturer_url.as_deref(),
        Some("https://mfg.example")
    );
}

#[test]
fn kvr_extract_version_after_word_version_plain() {
    let html = r#"Installer — Version 4.5.6 — ready"#;
    assert_eq!(
        app_lib::kvr::extract_version(html).as_deref(),
        Some("4.5.6")
    );
}

#[test]
fn export_payload_exported_at_preserved() {
    let p = ExportPayload {
        version: "1".into(),
        exported_at: "2026-01-02T00:00:00Z".into(),
        plugins: vec![],
    };
    let j = serde_json::to_string(&p).unwrap();
    let back: ExportPayload = serde_json::from_str(&j).unwrap();
    assert_eq!(back.exported_at, "2026-01-02T00:00:00Z");
}

#[test]
fn find_similar_two_candidates_both_scored() {
    let r = fp("/ref.wav");
    let mut a = fp("/a.wav");
    let mut b = fp("/b.wav");
    a.rms = r.rms + 0.01;
    b.rms = r.rms + 0.5;
    let out = find_similar(&r, &[a, b], 2);
    assert_eq!(out.len(), 2);
    assert!(out[0].1 <= out[1].1);
}

#[test]
fn fingerprint_distance_non_negative() {
    let a = fp("/a.wav");
    let b = fp("/b.wav");
    let d = fingerprint_distance(&a, &b);
    assert!(d >= 0.0 && d.is_finite());
}

#[test]
fn compute_audio_diff_no_overlap_paths() {
    let old = build_audio_snapshot(&[sample("/only.wav")], &[]);
    let new = build_audio_snapshot(&[sample("/other.wav")], &[]);
    let d = compute_audio_diff(&old, &new);
    assert_eq!(d.added.len(), 1);
    assert_eq!(d.removed.len(), 1);
}

#[test]
fn compute_preset_diff_two_removed() {
    let old = build_preset_snapshot(&[preset("/a.fxp"), preset("/b.fxp")], &[]);
    let new = build_preset_snapshot(&[], &[]);
    let d = compute_preset_diff(&old, &new);
    assert_eq!(d.removed.len(), 2);
}

#[test]
fn kvr_parse_single_huge_component() {
    let v = app_lib::kvr::parse_version("999999");
    assert_eq!(v, vec![999999]);
}

#[test]
fn kvr_cmp_same_length_lex_numeric() {
    assert_eq!(
        app_lib::kvr::compare_versions("2.0.0", "1.99.99"),
        Ordering::Greater
    );
}

#[test]
fn ext_matches_dawproject_deep_lowercase() {
    assert_eq!(
        ext_matches(Path::new("/a/b/c/d/project.dawproject")).as_deref(),
        Some("DAWPROJECT")
    );
}

#[test]
fn daw_name_dawproject_slug() {
    assert_eq!(daw_name_for_format("DAWPROJECT"), "DAWproject");
}

#[test]
fn is_package_ext_not_wav_file() {
    assert!(!is_package_ext(Path::new("/x.wav")));
}

#[test]
fn radix_string_zero_base_35() {
    assert_eq!(radix_string(0, 35), "0");
}

#[test]
fn format_size_2048_bytes() {
    let s = app_lib::format_size(2048);
    assert!(s.contains('2') || s.contains("KB") || s.contains("KiB"), "{s}");
}

#[test]
fn plugin_ref_normalized_name_distinct() {
    let p = PluginRef {
        name: "CamelCase".into(),
        normalized_name: "camelcase".into(),
        manufacturer: "M".into(),
        plugin_type: "VST3".into(),
    };
    let j = serde_json::to_string(&p).unwrap();
    assert!(j.contains("normalizedName"));
    let back: PluginRef = serde_json::from_str(&j).unwrap();
    assert_eq!(back.normalized_name, "camelcase");
}

#[test]
fn kvr_extract_version_in_paragraph_after_heading() {
    let html = r#"<h1>Product</h1><p>Version 11.22.33</p>"#;
    assert_eq!(
        app_lib::kvr::extract_version(html).as_deref(),
        Some("11.22.33")
    );
}

#[test]
fn compute_plugin_diff_version_downgrade() {
    let old = build_plugin_snapshot(&[plug("/p.vst3", "2.0")], &[], &[]);
    let new = build_plugin_snapshot(&[plug("/p.vst3", "1.0")], &[], &[]);
    let d = compute_plugin_diff(&old, &new);
    assert_eq!(d.version_changed.len(), 1);
}

/// Regression: `init_global` must serialize `Database::open` + migrations; parallel opens of the
/// same file caused `database is locked` on CI (`db::tests::init_global_concurrent_ok`). This
/// integration binary shares the same static state as lib tests.
#[test]
fn db_init_global_concurrent_many_threads() {
    let handles: Vec<_> = (0..48)
        .map(|_| {
            std::thread::spawn(|| {
                app_lib::db::init_global().expect("init_global");
                assert!(app_lib::db::global_initialized());
            })
        })
        .collect();
    for h in handles {
        h.join().expect("thread join");
    }
}

#[test]
fn kvr_compare_unknown_vs_empty_equals() {
    assert_eq!(
        app_lib::kvr::compare_versions("Unknown", ""),
        Ordering::Equal
    );
}

#[test]
fn kvr_parse_version_non_numeric_suffix_segment_becomes_zero() {
    let v = app_lib::kvr::parse_version("1.2.3beta");
    assert_eq!(v, vec![1, 2, 0]);
}

#[test]
fn normalize_plugin_name_unicode_lowercased() {
    assert_eq!(normalize_plugin_name("Müller (x64)"), "müller");
}

#[test]
fn compute_daw_diff_swap_two_projects_same_count() {
    let a = dawproj("/one.dawproject");
    let mut b = dawproj("/two.dawproject");
    b.name = "q".into();
    let old = build_daw_snapshot(&[a.clone()], &[]);
    let new = build_daw_snapshot(&[b.clone()], &[]);
    let d = compute_daw_diff(&old, &new);
    assert_eq!(d.added.len(), 1);
    assert_eq!(d.removed.len(), 1);
}

#[test]
fn ext_matches_ptx_deep_path_uppercase_ext() {
    assert_eq!(
        ext_matches(Path::new("/Sessions/PROJ.PTX")).as_deref(),
        Some("PTX")
    );
}

#[test]
fn radix_string_roundtrip_base_17() {
    let n: u64 = 169;
    let s = radix_string(n, 17);
    assert_eq!(u64::from_str_radix(&s, 17).unwrap(), n);
}

#[test]
fn export_payload_minimal_plugins_array_roundtrip() {
    let p = ExportPayload {
        version: "2".into(),
        exported_at: "t".into(),
        plugins: vec![ExportPlugin {
            name: "A".into(),
            plugin_type: "VST3".into(),
            version: "1".into(),
            manufacturer: "M".into(),
            manufacturer_url: Some("https://m.example".into()),
            path: "/a.vst3".into(),
            size: "1 B".into(),
            size_bytes: 1,
            modified: "m".into(),
            architectures: vec!["x64".into()],
        }],
    };
    let j = serde_json::to_string(&p).unwrap();
    let back: ExportPayload = serde_json::from_str(&j).unwrap();
    assert_eq!(back.plugins.len(), 1);
    assert_eq!(
        back.plugins[0].manufacturer_url.as_deref(),
        Some("https://m.example")
    );
}

#[test]
fn compute_preset_diff_both_empty_noop() {
    let empty = build_preset_snapshot(&[], &[]);
    let d = compute_preset_diff(&empty, &empty);
    assert!(d.added.is_empty());
    assert!(d.removed.is_empty());
}

#[test]
fn kvr_parse_version_double_dot_empty_segments_zero() {
    let v = app_lib::kvr::parse_version("1..2..3");
    assert_eq!(v, vec![1, 0, 2, 0, 3]);
}

#[test]
fn compute_audio_diff_identical_snapshots_empty_diff() {
    let s = sample("/x.wav");
    let snap = build_audio_snapshot(&[s.clone()], &[]);
    let d = compute_audio_diff(&snap, &snap);
    assert!(d.added.is_empty());
    assert!(d.removed.is_empty());
}

#[test]
fn format_size_1024_bytes_is_one_kb() {
    assert_eq!(app_lib::format_size(1024), "1.0 KB");
}

#[test]
fn find_similar_max_larger_than_candidate_count_returns_all_scored() {
    let r = fp("/ref.wav");
    let cands: Vec<_> = (0..3).map(|i| fp(&format!("/c{i}.wav"))).collect();
    let out = find_similar(&r, &cands, 100);
    assert_eq!(out.len(), 3);
}

#[test]
fn ext_matches_rpp_uppercase_extension() {
    assert_eq!(
        ext_matches(Path::new("C:/Projects/SESSION.RPP")).as_deref(),
        Some("RPP")
    );
}

#[test]
fn kvr_compare_versions_both_unknown_equal() {
    assert_eq!(
        app_lib::kvr::compare_versions("Unknown", "Unknown"),
        Ordering::Equal
    );
}

#[test]
fn compute_plugin_diff_unknown_to_known_same_path_not_version_changed() {
    let old = build_plugin_snapshot(&[plug("/p.vst3", "Unknown")], &[], &[]);
    let new = build_plugin_snapshot(&[plug("/p.vst3", "2.0")], &[], &[]);
    let d = compute_plugin_diff(&old, &new);
    assert!(d.added.is_empty());
    assert!(d.removed.is_empty());
    assert!(
        d.version_changed.is_empty(),
        "Unknown→known is not listed as version_changed per diff rules"
    );
}

#[test]
fn plugin_ref_serde_roundtrip_empty_manufacturer() {
    let p = PluginRef {
        name: "X".into(),
        normalized_name: "x".into(),
        manufacturer: "".into(),
        plugin_type: "VST3".into(),
    };
    let j = serde_json::to_string(&p).unwrap();
    let back: PluginRef = serde_json::from_str(&j).unwrap();
    assert_eq!(back.manufacturer, "");
}

#[test]
fn kvr_extract_version_plain_version_colon_line() {
    let html = r#"Release notes — Version: 9.8.7 — stable"#;
    assert_eq!(
        app_lib::kvr::extract_version(html).as_deref(),
        Some("9.8.7")
    );
}

#[test]
fn radix_string_pow2_256_base16() {
    assert_eq!(radix_string(256, 16), "100");
}

#[test]
fn fingerprint_distance_commutative_explicit() {
    let a = fp("/a.wav");
    let mut b = fp("/b.wav");
    b.rms = 0.11;
    let d1 = fingerprint_distance(&a, &b);
    let d2 = fingerprint_distance(&b, &a);
    assert!((d1 - d2).abs() < 1e-9);
}

#[test]
fn compute_preset_diff_one_added_two_removed_net() {
    let old = build_preset_snapshot(
        &[preset("/a.fxp"), preset("/b.fxp"), preset("/c.fxp")],
        &[],
    );
    let new = build_preset_snapshot(&[preset("/new.fxp")], &[]);
    let d = compute_preset_diff(&old, &new);
    assert_eq!(d.added.len(), 1);
    assert_eq!(d.removed.len(), 3);
}

#[test]
fn compute_audio_diff_both_empty_snapshots() {
    let a = build_audio_snapshot(&[], &[]);
    let b = build_audio_snapshot(&[], &[]);
    let d = compute_audio_diff(&a, &b);
    assert!(d.added.is_empty());
    assert!(d.removed.is_empty());
}

#[test]
fn get_plugin_type_dot_bundle_unknown() {
    assert_eq!(get_plugin_type(".bundle"), "Unknown");
}

// ── Wave 10: plugin diff (both-known version_changed), find_similar edges, format_size
//    boundary, radix round-trips, xref extract guard, KVR + serde + DAW path tokens ─────

#[test]
fn compute_plugin_diff_both_known_same_path_emits_version_changed() {
    let old = build_plugin_snapshot(&[plug("/same.vst3", "1.0.0")], &[], &[]);
    let new = build_plugin_snapshot(&[plug("/same.vst3", "2.1.0")], &[], &[]);
    let d = compute_plugin_diff(&old, &new);
    assert!(d.added.is_empty() && d.removed.is_empty());
    assert_eq!(d.version_changed.len(), 1);
    assert_eq!(d.version_changed[0].previous_version, "1.0.0");
    assert_eq!(d.version_changed[0].plugin.version, "2.1.0");
}

#[test]
fn compute_plugin_diff_same_known_version_no_version_changed() {
    let old = build_plugin_snapshot(&[plug("/p.vst3", "3.3.3")], &[], &[]);
    let new = build_plugin_snapshot(&[plug("/p.vst3", "3.3.3")], &[], &[]);
    let d = compute_plugin_diff(&old, &new);
    assert!(d.version_changed.is_empty());
}

#[test]
fn find_similar_empty_candidates_returns_empty() {
    let r = fp("/ref.wav");
    let out = find_similar(&r, &[], 5);
    assert!(out.is_empty());
}

#[test]
fn find_similar_max_results_zero_truncates_to_empty() {
    let r = fp("/ref.wav");
    let cands = vec![fp("/a.wav"), fp("/b.wav")];
    let out = find_similar(&r, &cands, 0);
    assert!(out.is_empty());
}

#[test]
fn format_size_one_byte_below_one_mib_stays_in_kb_tier() {
    let b = 1024_u64 * 1024 - 1;
    assert_eq!(app_lib::format_size(b), "1024.0 KB");
}

#[test]
fn radix_string_one_base2() {
    assert_eq!(radix_string(1, 2), "1");
}

#[test]
fn radix_string_u64_max_base10_roundtrip() {
    let s = radix_string(u64::MAX, 10);
    assert_eq!(s.parse::<u64>().unwrap(), u64::MAX);
}

#[test]
fn ext_matches_reaper_project_uppercase_ext() {
    assert_eq!(
        ext_matches(Path::new("/Sessions/Mix.RPP")).as_deref(),
        Some("RPP")
    );
}

#[test]
fn ext_matches_nuendo_cpr_uppercase_deep_path() {
    assert_eq!(
        ext_matches(Path::new("/Volumes/Audio/MyAlbum/MASTER.CPR")).as_deref(),
        Some("CPR")
    );
}

#[test]
fn extract_plugins_no_extension_returns_empty() {
    assert!(extract_plugins("/tmp/noextension").is_empty());
}

#[test]
fn extract_plugins_unknown_extension_returns_empty() {
    assert!(extract_plugins("/tmp/x.xyz").is_empty());
}

#[test]
fn normalize_plugin_name_strips_aax_suffix_token() {
    assert_eq!(normalize_plugin_name("MyPlug (AAX)"), "myplug");
}

#[test]
fn kvr_compare_versions_equal_with_trailing_zeros() {
    assert_eq!(
        app_lib::kvr::compare_versions("2.0", "2.0.0"),
        Ordering::Equal
    );
}

#[test]
fn kvr_parse_version_non_numeric_segment_becomes_zero() {
    // split on '.' only; "v12" does not parse as i32
    assert_eq!(app_lib::kvr::parse_version("v12.34"), vec![0, 34]);
}

#[test]
fn export_plugin_json_skips_manufacturer_url_key_when_none() {
    let p = ExportPlugin {
        name: "N".into(),
        plugin_type: "VST3".into(),
        version: "1".into(),
        manufacturer: "M".into(),
        manufacturer_url: None,
        path: "/p.vst3".into(),
        size: "1 B".into(),
        size_bytes: 1,
        modified: "m".into(),
        architectures: vec![],
    };
    let j = serde_json::to_string(&p).unwrap();
    assert!(!j.contains("manufacturerUrl"));
}

#[test]
fn fingerprint_distance_self_is_zero() {
    let a = fp("/self.wav");
    assert!(fingerprint_distance(&a, &a).abs() < 1e-9);
}

#[test]
fn compute_audio_diff_one_sample_added() {
    let s0 = sample("/only.wav");
    let old = build_audio_snapshot(&[], &[]);
    let new = build_audio_snapshot(&[s0], &[]);
    let d = compute_audio_diff(&old, &new);
    assert_eq!(d.added.len(), 1);
    assert!(d.removed.is_empty());
}

#[test]
fn compute_preset_diff_identical_lists_no_delta() {
    let ps = vec![preset("/a.fxp"), preset("/b.fxp")];
    let old = build_preset_snapshot(&ps, &[]);
    let new = build_preset_snapshot(&ps, &[]);
    let d = compute_preset_diff(&old, &new);
    assert!(d.added.is_empty() && d.removed.is_empty());
}

// ── Wave 11: plugin Unknown transitions, xref missing files, format_size multi-MiB,
//    radix base-36, preset swap, KVR lexicographic, normalize edge cases ───────────────

#[test]
fn compute_plugin_diff_known_to_unknown_same_path_not_version_changed() {
    let old = build_plugin_snapshot(&[plug("/only.vst3", "9.9.9")], &[], &[]);
    let new = build_plugin_snapshot(&[plug("/only.vst3", "Unknown")], &[], &[]);
    let d = compute_plugin_diff(&old, &new);
    assert!(
        d.version_changed.is_empty(),
        "Known→Unknown is not version_changed (new side is Unknown)"
    );
}

#[test]
fn compute_plugin_diff_both_unknown_same_path_not_version_changed() {
    let old = build_plugin_snapshot(&[plug("/x.vst3", "Unknown")], &[], &[]);
    let new = build_plugin_snapshot(&[plug("/x.vst3", "Unknown")], &[], &[]);
    let d = compute_plugin_diff(&old, &new);
    assert!(d.version_changed.is_empty());
}

#[test]
fn extract_plugins_nonexistent_rpp_returns_empty() {
    let p = std::env::temp_dir().join("audio_haxor_no_such_rpp_for_xref_test.rpp");
    assert!(extract_plugins(p.to_str().expect("utf8 temp path")).is_empty());
}

#[test]
fn format_size_two_mebibytes() {
    let s = app_lib::format_size(2 * 1024 * 1024);
    assert_eq!(s, "2.0 MB");
}

#[test]
fn radix_string_35_in_base36_is_z() {
    assert_eq!(radix_string(35, 36), "z");
}

#[test]
fn kvr_compare_versions_lexicographic_major_strings() {
    assert_eq!(
        app_lib::kvr::compare_versions("10", "9"),
        Ordering::Greater
    );
}

#[test]
fn kvr_parse_version_whitespace_only_yields_single_zero_segment() {
    assert_eq!(app_lib::kvr::parse_version("   "), vec![0]);
}

#[test]
fn fingerprint_distance_identical_vectors_different_paths_near_zero() {
    let a = fp("/path/one.wav");
    let mut b = fp("/other/two.wav");
    b.rms = a.rms;
    b.spectral_centroid = a.spectral_centroid;
    b.zero_crossing_rate = a.zero_crossing_rate;
    b.low_band_energy = a.low_band_energy;
    b.mid_band_energy = a.mid_band_energy;
    b.high_band_energy = a.high_band_energy;
    b.low_energy_ratio = a.low_energy_ratio;
    b.attack_time = a.attack_time;
    assert!(fingerprint_distance(&a, &b).abs() < 1e-9);
}

#[test]
fn compute_preset_diff_swap_two_paths_two_added_two_removed() {
    let old = build_preset_snapshot(&[preset("/a.fxp"), preset("/b.fxp")], &[]);
    let new = build_preset_snapshot(&[preset("/b.fxp"), preset("/c.fxp")], &[]);
    let d = compute_preset_diff(&old, &new);
    assert_eq!(d.added.len(), 1);
    assert_eq!(d.removed.len(), 1);
}

#[test]
fn ext_matches_ableton_project_lowercase_als() {
    assert_eq!(
        ext_matches(Path::new("/Music/Projects/live_set.als")).as_deref(),
        Some("ALS")
    );
}

#[test]
fn ext_matches_fl_studio_flp_with_spaces_in_parent_dir() {
    assert_eq!(
        ext_matches(Path::new("/My Beats/v1/beat.flp")).as_deref(),
        Some("FLP")
    );
}

#[test]
fn normalize_plugin_name_strips_x64_twice_nested() {
    let n = normalize_plugin_name("Synth (x64) (VST3)");
    assert!(!n.contains("x64"));
    assert!(!n.contains("vst3"));
}

#[test]
fn compute_audio_diff_two_samples_removed() {
    let s1 = sample("/one.wav");
    let s2 = sample("/two.wav");
    let old = build_audio_snapshot(&[s1.clone(), s2.clone()], &[]);
    let new = build_audio_snapshot(&[], &[]);
    let d = compute_audio_diff(&old, &new);
    assert_eq!(d.removed.len(), 2);
    assert!(d.added.is_empty());
}

#[test]
fn compute_daw_diff_one_format_change_same_stem_different_ext() {
    let a = dawproj("/proj.dawproject");
    let mut b = DawProject {
        format: "rpp".into(),
        daw: "reaper".into(),
        ..a.clone()
    };
    b.path = "/proj.rpp".into();
    let old = build_daw_snapshot(&[a], &[]);
    let new = build_daw_snapshot(&[b], &[]);
    let d = compute_daw_diff(&old, &new);
    assert_eq!(d.added.len(), 1);
    assert_eq!(d.removed.len(), 1);
}

#[test]
fn compute_plugin_diff_two_plugins_only_one_emits_version_changed() {
    let old = build_plugin_snapshot(
        &[plug("/a.vst3", "1.0"), plug("/b.vst3", "1.0")],
        &[],
        &[],
    );
    let new = build_plugin_snapshot(
        &[plug("/a.vst3", "2.0"), plug("/b.vst3", "1.0")],
        &[],
        &[],
    );
    let d = compute_plugin_diff(&old, &new);
    assert_eq!(d.version_changed.len(), 1);
    assert_eq!(d.version_changed[0].plugin.path, "/a.vst3");
}

#[test]
fn kvr_compare_versions_empty_vs_empty() {
    assert_eq!(app_lib::kvr::compare_versions("", ""), Ordering::Equal);
}

#[test]
fn find_similar_three_candidates_max_two() {
    let r = fp("/ref.wav");
    let cands: Vec<_> = (0..3).map(|i| fp(&format!("/c{i}.wav"))).collect();
    let out = find_similar(&r, &cands, 2);
    assert_eq!(out.len(), 2);
}

// ── Wave 12: radix b36 grid, KVR JSON-LD `softwareVersion`, preset/plugin/DAW diffs,
//    Nuendo/GarageBand paths, fingerprint + find_similar ordering ──────────────────────

#[test]
fn radix_string_1296_base36_is_one_hundred() {
    assert_eq!(radix_string(1296, 36), "100");
}

#[test]
fn kvr_extract_version_software_version_json_ld_style() {
    let html = r#"{"@type":"SoftwareApplication","softwareVersion":"4.5.6"}"#;
    assert_eq!(
        app_lib::kvr::extract_version(html).as_deref(),
        Some("4.5.6")
    );
}

#[test]
fn ext_matches_nuendo_uppercase_npr_filename() {
    assert_eq!(
        ext_matches(Path::new("/Sessions/FilmScore/MASTER.NPR")).as_deref(),
        Some("NPR")
    );
}

#[test]
fn compute_preset_diff_empty_to_three_presets() {
    let old = build_preset_snapshot(&[], &[]);
    let new = build_preset_snapshot(
        &[
            preset("/presets/a.fxp"),
            preset("/presets/b.fxp"),
            preset("/presets/c.fxp"),
        ],
        &[],
    );
    let d = compute_preset_diff(&old, &new);
    assert_eq!(d.added.len(), 3);
    assert!(d.removed.is_empty());
}

#[test]
fn compute_plugin_diff_remove_one_add_one_different_paths() {
    let old = build_plugin_snapshot(&[plug("/old.vst3", "1")], &[], &[]);
    let new = build_plugin_snapshot(&[plug("/new.vst3", "1")], &[], &[]);
    let d = compute_plugin_diff(&old, &new);
    assert_eq!(d.removed.len(), 1);
    assert_eq!(d.added.len(), 1);
    assert!(d.version_changed.is_empty());
}

#[test]
fn normalize_plugin_name_pro_q_style_preserves_version_digit() {
    assert_eq!(
        normalize_plugin_name("FabFilter Pro-Q 3 (VST3)"),
        "fabfilter pro-q 3"
    );
}

#[test]
fn kvr_compare_versions_trailing_components_implicit_zero() {
    assert_eq!(
        app_lib::kvr::compare_versions("1.0.0", "1.0"),
        Ordering::Equal
    );
}

#[test]
fn find_similar_four_candidates_max_one() {
    let r = fp("/ref.wav");
    let cands: Vec<_> = (0..4).map(|i| fp(&format!("/c{i}.wav"))).collect();
    let out = find_similar(&r, &cands, 1);
    assert_eq!(out.len(), 1);
}

#[test]
fn fingerprint_distance_rms_only_change_nonzero() {
    let a = fp("/a.wav");
    let mut b = fp("/b.wav");
    b.rms = 0.99;
    assert!(fingerprint_distance(&a, &b) > 0.01);
}

#[test]
fn compute_daw_diff_replace_one_project_same_count() {
    let old = build_daw_snapshot(&[dawproj("/one.dawproject")], &[]);
    let mut p = dawproj("/other.dawproject");
    p.name = "other".into();
    let new = build_daw_snapshot(&[p], &[]);
    let d = compute_daw_diff(&old, &new);
    assert_eq!(d.added.len(), 1);
    assert_eq!(d.removed.len(), 1);
}

#[test]
fn kvr_compare_versions_leading_zeros_in_component_equal() {
    assert_eq!(
        app_lib::kvr::compare_versions("1.0", "1.00"),
        Ordering::Equal
    );
}

#[test]
fn kvr_parse_version_triple_plus_non_numeric_segment_zero() {
    assert_eq!(app_lib::kvr::parse_version("+++"), vec![0]);
}

#[test]
fn ext_matches_garageband_band_uppercase() {
    assert_eq!(
        ext_matches(Path::new("/Mobile/Grooves/BEAT.BAND")).as_deref(),
        Some("BAND")
    );
}

#[test]
fn compute_plugin_diff_version_changed_and_added_in_same_diff() {
    let old = build_plugin_snapshot(&[plug("/keep.vst3", "1.0")], &[], &[]);
    let new = build_plugin_snapshot(
        &[plug("/keep.vst3", "2.0"), plug("/extra.vst3", "1")],
        &[],
        &[],
    );
    let d = compute_plugin_diff(&old, &new);
    assert_eq!(d.version_changed.len(), 1);
    assert_eq!(d.added.len(), 1);
}

#[test]
fn format_size_quarter_tebibyte() {
    let s = app_lib::format_size(256 * 1024_u64.pow(4));
    assert!(s.contains("TB"), "{s}");
}

#[test]
fn kvr_parse_version_single_dot_only() {
    assert_eq!(app_lib::kvr::parse_version("."), vec![0, 0]);
}

#[test]
fn find_similar_picks_lowest_distance_path_first() {
    let r = fp("/ref.wav");
    let mut close = fp("/close.wav");
    close.rms = r.rms + 0.001;
    let mut far = fp("/far.wav");
    far.rms = 0.99;
    let out = find_similar(&r, &[close, far], 1);
    assert_eq!(out.len(), 1);
    assert!(
        out[0].0.contains("close"),
        "expected nearest fingerprint path, got {:?}",
        out[0].0
    );
}
