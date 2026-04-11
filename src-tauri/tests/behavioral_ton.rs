//! Large set of focused scenario tests (named `#[test]` each; not macro grids).

use std::cmp::Ordering;
use std::path::Path;

use app_lib::history::{
    AudioSample, DawProject, PresetFile, build_audio_snapshot, build_daw_snapshot,
    build_plugin_snapshot, build_preset_snapshot, compute_audio_diff, compute_daw_diff,
    compute_plugin_diff, compute_preset_diff, gen_id,
};
use app_lib::scanner::PluginInfo;
use app_lib::similarity::{AudioFingerprint, find_similar, fingerprint_distance};
use app_lib::xref::normalize_plugin_name;
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
fn fp_full(
    path: &str,
    rms: f64,
    sc: f64,
    zcr: f64,
    low: f64,
    mid: f64,
    high: f64,
    ratio: f64,
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
        low_energy_ratio: ratio,
        attack_time: atk,
    }
}

fn sample_audio(path: &str) -> AudioSample {
    AudioSample {
        name: "n".into(),
        path: path.into(),
        directory: "/d".into(),
        format: "WAV".into(),
        size: 10,
        size_formatted: "10 B".into(),
        modified: "m".into(),
        duration: None,
        channels: None,
        sample_rate: None,
        bits_per_sample: None,
    }
}

fn sample_daw(path: &str) -> DawProject {
    DawProject {
        name: "p".into(),
        path: path.into(),
        directory: "/d".into(),
        format: "ALS".into(),
        daw: "Ableton Live".into(),
        size: 50,
        size_formatted: "50 B".into(),
        modified: "m".into(),
    }
}

fn sample_preset(path: &str) -> PresetFile {
    PresetFile {
        name: "pr".into(),
        path: path.into(),
        directory: "/d".into(),
        format: "fxp".into(),
        size: 3,
        size_formatted: "3 B".into(),
        modified: "m".into(),
    }
}

// ── KVR compare_versions (extra pairwise cases) ─────────────────────────────

#[test]
fn kvr_cmp_100_0_gt_99_99() {
    assert_eq!(
        app_lib::kvr::compare_versions("100.0", "99.99"),
        Ordering::Greater
    );
}

#[test]
fn kvr_cmp_0_0_0_1_gt_0_0_0_0() {
    assert_eq!(
        app_lib::kvr::compare_versions("0.0.0.1", "0.0.0.0"),
        Ordering::Greater
    );
}

#[test]
fn kvr_cmp_shorter_left_shorter_implies_zero_pad() {
    assert_eq!(
        app_lib::kvr::compare_versions("5", "4.9.9"),
        Ordering::Greater
    );
}

#[test]
fn kvr_parse_does_not_trim_outer_whitespace() {
    // Segments are not `.trim()`'d; leading/trailing spaces make numeric parse fail → 0.
    let v = app_lib::kvr::parse_version(" 1.2 ");
    assert_eq!(v, vec![0, 0]);
}

#[test]
fn kvr_extract_version_current_keyword() {
    let html = "<span>current release v12.0.1 today</span>";
    assert_eq!(
        app_lib::kvr::extract_version(html).as_deref(),
        Some("12.0.1")
    );
}

#[test]
fn kvr_extract_download_url_buy_href() {
    let html = r#"<a href="https://shop.example.com/buy-plugin-v3">buy</a>"#;
    let r = app_lib::kvr::extract_download_url(html);
    assert!(
        r.is_some(),
        "buy in path should match download link pattern"
    );
}

// ── format_size (spot checks) ───────────────────────────────────────────────

#[test]
fn fmt_512_bytes() {
    assert!(app_lib::format_size(512).ends_with(" B"));
}

#[test]
fn fmt_1025_bytes_uses_kb() {
    let s = app_lib::format_size(1025);
    assert!(s.contains("KB"), "{s}");
}

#[test]
fn fmt_half_megabyte() {
    let s = app_lib::format_size(512 * 1024);
    assert!(s.contains("KB") || s.contains("MB"), "{s}");
}

// ── similarity ──────────────────────────────────────────────────────────────

#[test]
fn fingerprint_distance_increases_when_rms_differs() {
    let a = fp_full("/a.wav", 0.1, 0.2, 0.05, 0.1, 0.2, 0.05, 0.4, 0.01);
    let b = fp_full("/b.wav", 0.9, 0.2, 0.05, 0.1, 0.2, 0.05, 0.4, 0.01);
    let d = fingerprint_distance(&a, &b);
    assert!(d > 0.1);
}

#[test]
fn find_similar_three_candidates_picks_closest() {
    let r = fp_full("/r.wav", 0.5, 0.3, 0.05, 0.1, 0.2, 0.05, 0.4, 0.01);
    let c = vec![
        fp_full("/far.wav", 0.99, 0.3, 0.05, 0.1, 0.2, 0.05, 0.4, 0.01),
        fp_full("/mid.wav", 0.52, 0.3, 0.05, 0.1, 0.2, 0.05, 0.4, 0.01),
        fp_full("/near.wav", 0.501, 0.3, 0.05, 0.1, 0.2, 0.05, 0.4, 0.01),
    ];
    let out = find_similar(&r, &c, 3);
    assert_eq!(out[0].0, "/near.wav");
}

#[test]
fn fingerprint_distance_attack_time_contributes() {
    let base = fp_full("/x.wav", 0.5, 0.2, 0.05, 0.1, 0.2, 0.05, 0.4, 0.01);
    let diff_atk = fp_full("/y.wav", 0.5, 0.2, 0.05, 0.1, 0.2, 0.05, 0.4, 1.5);
    assert!(fingerprint_distance(&base, &diff_atk) > 1e-6);
}

// ── history diffs (audio / preset) ───────────────────────────────────────────

#[test]
fn compute_audio_diff_removed_only() {
    let old = build_audio_snapshot(&[sample_audio("/old.wav")], &[]);
    let new = build_audio_snapshot(&[], &[]);
    let d = compute_audio_diff(&old, &new);
    assert_eq!(d.removed.len(), 1);
    assert!(d.added.is_empty());
}

#[test]
fn compute_preset_diff_added_two() {
    let old = build_preset_snapshot(&[], &[]);
    let new = build_preset_snapshot(&[sample_preset("/a.fxp"), sample_preset("/b.fxp")], &[]);
    let d = compute_preset_diff(&old, &new);
    assert_eq!(d.added.len(), 2);
}

#[test]
fn compute_daw_diff_identical_projects_no_move() {
    let p = sample_daw("/proj.als");
    let s = build_daw_snapshot(&[p], &["/r".into()]);
    let d = compute_daw_diff(&s, &s);
    assert!(d.added.is_empty() && d.removed.is_empty());
}

#[test]
fn compute_plugin_diff_three_added() {
    let old = build_plugin_snapshot(&[], &[], &[]);
    let new = build_plugin_snapshot(
        &[
            plug("/a.vst3", "1"),
            plug("/b.vst3", "1"),
            plug("/c.vst3", "1"),
        ],
        &[],
        &[],
    );
    let d = compute_plugin_diff(&old, &new);
    assert_eq!(d.added.len(), 3);
}

// ── gen_id uniqueness (small batch) ────────────────────────────────────────

#[test]
fn gen_id_batch_no_duplicates_32() {
    use std::collections::HashSet;
    let mut set = HashSet::new();
    for _ in 0..32 {
        assert!(set.insert(gen_id()));
    }
}

// ── export / serde ──────────────────────────────────────────────────────────

#[test]
fn export_payload_empty_plugins_array() {
    let p = ExportPayload {
        version: "0.0.1".into(),
        exported_at: "t".into(),
        plugins: vec![],
    };
    let v = serde_json::to_value(&p).unwrap();
    assert_eq!(v["plugins"], serde_json::json!([]));
}

#[test]
fn export_plugin_skip_none_manufacturer_url_omits_key() {
    let ep = ExportPlugin {
        name: "n".into(),
        plugin_type: "VST3".into(),
        version: "1".into(),
        manufacturer: "m".into(),
        manufacturer_url: None,
        path: "/p".into(),
        size: "1 B".into(),
        size_bytes: 1,
        modified: "m".into(),
        architectures: vec![],
    };
    let v = serde_json::to_value(&ep).unwrap();
    assert!(v.get("manufacturerUrl").is_none());
}

#[test]
fn daw_project_json_roundtrip() {
    let d = sample_daw("/x.als");
    let json = serde_json::to_string(&d).unwrap();
    let back: DawProject = serde_json::from_str(&json).unwrap();
    assert_eq!(back.path, d.path);
}

// ── xref normalize (extra strings) ─────────────────────────────────────────

#[test]
fn norm_apple_silicon_suffix() {
    let n = normalize_plugin_name("Filter (Apple Silicon)");
    assert!(!n.contains("apple"));
}

#[test]
fn norm_intel_suffix() {
    let n = normalize_plugin_name("EQ (Intel)");
    assert!(!n.to_lowercase().contains("intel"));
}

#[test]
fn norm_preserves_inner_hyphen_plugin_name() {
    let n = normalize_plugin_name("Pro-Q 3");
    assert!(n.contains("pro-q") || n.contains("pro"));
}

// ── scanner / bpm read ───────────────────────────────────────────────────────

#[test]
fn bpm_read_wav_pub_missing_none() {
    assert!(app_lib::bpm::read_wav_pcm_pub(Path::new("/no/such/file.wav")).is_none());
}

#[test]
fn get_plugin_type_empty_string_unknown() {
    assert_eq!(app_lib::scanner::get_plugin_type(""), "Unknown");
}

#[test]
fn get_plugin_type_dot_only_unknown() {
    assert_eq!(app_lib::scanner::get_plugin_type("."), "Unknown");
}

// ── lufs unsupported ext ─────────────────────────────────────────────────────

#[test]
fn lufs_text_file_returns_none() {
    assert!(app_lib::lufs::measure_lufs("/tmp/not-a-real-audio-file.txt").is_none());
}

// ── kvr URL_RE multiple URLs takes first ─────────────────────────────────────

#[test]
fn url_re_first_match_is_leftmost() {
    let t = "a http://one.com/x http://two.com/y";
    let m = app_lib::kvr::URL_RE.find(t).unwrap();
    assert!(m.as_str().contains("one.com"));
}

// ── preset + audio snapshot serde keys ───────────────────────────────────────

#[test]
fn preset_scan_snapshot_build_has_format_counts() {
    let s = build_preset_snapshot(&[sample_preset("/p.fxp")], &[]);
    let v = serde_json::to_value(&s).unwrap();
    assert!(v.get("formatCounts").is_some());
}

#[test]
fn audio_scan_snapshot_build_has_sample_count() {
    let s = build_audio_snapshot(&[sample_audio("/z.wav")], &[]);
    assert_eq!(s.sample_count, 1);
}

// ── plugin diff: multiple removed ────────────────────────────────────────────

#[test]
fn compute_plugin_diff_removes_two_plugins() {
    let old = build_plugin_snapshot(&[plug("/a.vst3", "1"), plug("/b.vst3", "1")], &[], &[]);
    let new = build_plugin_snapshot(&[], &[], &[]);
    let d = compute_plugin_diff(&old, &new);
    assert_eq!(d.removed.len(), 2);
}

// ── kvr compare edge ────────────────────────────────────────────────────────

#[test]
fn kvr_cmp_equal_empty_vs_unknown_parse() {
    assert_eq!(
        app_lib::kvr::compare_versions("", "Unknown"),
        Ordering::Equal
    );
}

#[test]
fn kvr_parse_single_dot() {
    assert_eq!(app_lib::kvr::parse_version("."), vec![0, 0]);
}

#[test]
fn kvr_extract_version_none_for_plain_text() {
    assert!(app_lib::kvr::extract_version("no version here at all").is_none());
}

// ── Additional KVR / scanner / history (batch 2) ────────────────────────────

#[test]
fn kvr_cmp_less_for_all_zeros_vs_one() {
    assert_eq!(app_lib::kvr::compare_versions("0.0.0", "1"), Ordering::Less);
}

#[test]
fn kvr_parse_leading_dot() {
    assert_eq!(app_lib::kvr::parse_version(".5"), vec![0, 5]);
}

#[test]
fn format_size_exactly_one_mib() {
    let s = app_lib::format_size(1024 * 1024);
    assert!(s.contains("MB"), "{s}");
}

#[test]
fn similarity_zero_crossing_rate_affects_distance() {
    let a = fp_full("/a.wav", 0.5, 0.2, 0.01, 0.1, 0.2, 0.05, 0.4, 0.01);
    let b = fp_full("/b.wav", 0.5, 0.2, 0.40, 0.1, 0.2, 0.05, 0.4, 0.01);
    assert!(fingerprint_distance(&a, &b) > 1e-6);
}

#[test]
fn find_similar_limit_two_from_three() {
    let r = fp_full("/r.wav", 0.5, 0.3, 0.05, 0.1, 0.2, 0.05, 0.4, 0.01);
    let c = vec![
        fp_full("/1.wav", 0.51, 0.3, 0.05, 0.1, 0.2, 0.05, 0.4, 0.01),
        fp_full("/2.wav", 0.52, 0.3, 0.05, 0.1, 0.2, 0.05, 0.4, 0.01),
        fp_full("/3.wav", 0.53, 0.3, 0.05, 0.1, 0.2, 0.05, 0.4, 0.01),
    ];
    let out = find_similar(&r, &c, 2);
    assert_eq!(out.len(), 2);
}

#[test]
fn compute_audio_diff_both_populated_swap() {
    let old = build_audio_snapshot(&[sample_audio("/a.wav")], &[]);
    let new = build_audio_snapshot(&[sample_audio("/b.wav")], &[]);
    let d = compute_audio_diff(&old, &new);
    assert_eq!(d.removed.len(), 1);
    assert_eq!(d.added.len(), 1);
}

#[test]
fn compute_preset_diff_removed_when_cleared() {
    let old = build_preset_snapshot(&[sample_preset("/p.fxp")], &[]);
    let new = build_preset_snapshot(&[], &[]);
    let d = compute_preset_diff(&old, &new);
    assert_eq!(d.removed.len(), 1);
    assert!(d.added.is_empty());
}

#[test]
fn plugin_snapshot_json_plugin_count_matches_vec() {
    let s = build_plugin_snapshot(&[plug("/x.vst3", "1"), plug("/y.vst3", "2")], &[], &[]);
    assert_eq!(s.plugin_count, s.plugins.len());
}

#[test]
fn normalize_x86_suffix_in_brackets() {
    // `vst2` is not in the arch-suffix alternation; `x86` is.
    let n = normalize_plugin_name("Legacy (x86)");
    assert!(!n.contains("x86"));
}

#[test]
fn normalize_mono_suffix() {
    let n = normalize_plugin_name("Chorus (mono)");
    assert!(!n.contains("mono"));
}

#[test]
fn get_plugin_type_so_unknown() {
    assert_eq!(app_lib::scanner::get_plugin_type(".so"), "Unknown");
}

#[test]
fn get_plugin_type_dylib_unknown() {
    assert_eq!(app_lib::scanner::get_plugin_type(".dylib"), "Unknown");
}

#[test]
fn kvr_url_re_allows_trailing_paren_in_url() {
    let t = r#"href="https://x.com/a(b)""#;
    let m = app_lib::kvr::URL_RE.find(t);
    assert!(m.is_some());
}

#[test]
fn export_payload_version_roundtrips() {
    let p = ExportPayload {
        version: "1.2.3".into(),
        exported_at: "2020-01-01T00:00:00Z".into(),
        plugins: vec![ExportPlugin {
            name: "P".into(),
            plugin_type: "AU".into(),
            version: "3".into(),
            manufacturer: "Co".into(),
            manufacturer_url: None,
            path: "/p.component".into(),
            size: "1 B".into(),
            size_bytes: 1,
            modified: "m".into(),
            architectures: vec![],
        }],
    };
    let json = serde_json::to_string(&p).unwrap();
    let back: ExportPayload = serde_json::from_str(&json).unwrap();
    assert_eq!(back.version, "1.2.3");
    assert_eq!(back.plugins.len(), 1);
}

#[test]
fn preset_file_json_size_formatted_rename() {
    let pf = sample_preset("/a.fxp");
    let v = serde_json::to_value(&pf).unwrap();
    assert!(v.get("sizeFormatted").is_some());
}

#[test]
fn audio_sample_default_optional_fields_none_in_json() {
    let a = sample_audio("/f.wav");
    let v = serde_json::to_value(&a).unwrap();
    assert!(v.get("duration").is_none());
}

#[test]
fn kvr_cmp_max_intish_segments() {
    assert_eq!(
        app_lib::kvr::compare_versions("2147483647.0", "2147483646.999"),
        Ordering::Greater
    );
}
