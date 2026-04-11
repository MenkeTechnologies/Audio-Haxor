//! More focused scenario tests (named cases, not macro-generated grids).

use std::cmp::Ordering;
use std::path::Path;

use app_lib::audio_scanner::get_audio_metadata;
use app_lib::daw_scanner::{daw_name_for_format, ext_matches, is_package_ext};
use app_lib::history::{compute_daw_diff, compute_plugin_diff, radix_string};
use app_lib::scanner::PluginInfo;
use app_lib::xref::{extract_plugins, normalize_plugin_name};

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

// ── Missing-file smoke (public APIs return None / error safely) ─────────────

#[test]
fn bpm_estimate_bpm_nonexistent_returns_none() {
    assert!(app_lib::bpm::estimate_bpm("/no/such/path/file.wav").is_none());
}

#[test]
fn lufs_measure_nonexistent_returns_none() {
    assert!(app_lib::lufs::measure_lufs("/no/such/path/file.wav").is_none());
}

#[test]
fn key_detect_nonexistent_returns_none() {
    assert!(app_lib::key_detect::detect_key("/no/such/path/file.wav").is_none());
}

#[test]
fn midi_parse_nonexistent_returns_none() {
    assert!(app_lib::midi::parse_midi(Path::new("/no/such/path/file.mid")).is_none());
}

#[test]
fn audio_metadata_nonexistent_has_error_field() {
    let m = get_audio_metadata("/no/such/file.wav");
    assert!(m.error.is_some(), "expected io error in metadata");
    assert_eq!(m.size_bytes, 0);
}

#[test]
fn bpm_unsupported_extension_returns_none() {
    assert!(app_lib::bpm::estimate_bpm("/tmp/x.xyz").is_none());
}

#[test]
fn key_detect_unsupported_extension_returns_none() {
    assert!(app_lib::key_detect::detect_key("/tmp/x.xyz").is_none());
}

// ── DAW ext_matches + display name (one suffix per test) ───────────────────

fn assert_daw_ext(path_suffix: &str, code: &str) {
    let p = Path::new("/tmp").join(path_suffix);
    assert_eq!(ext_matches(&p).as_deref(), Some(code));
    assert_ne!(daw_name_for_format(code), "Unknown");
}

#[test]
fn daw_ext_als() {
    assert_daw_ext("live.als", "ALS");
}

#[test]
fn daw_ext_logicx() {
    assert_daw_ext("p.logicx", "LOGICX");
}

#[test]
fn daw_ext_flp() {
    assert_daw_ext("beat.flp", "FLP");
}

#[test]
fn daw_ext_cpr() {
    assert_daw_ext("song.cpr", "CPR");
}

#[test]
fn daw_ext_npr() {
    assert_daw_ext("nuendo.npr", "NPR");
}

#[test]
fn daw_ext_bwproject() {
    assert_daw_ext("proj.bwproject", "BWPROJECT");
}

#[test]
fn daw_ext_rpp() {
    assert_daw_ext("tr.rpp", "RPP");
}

#[test]
fn daw_ext_rpp_bak() {
    assert_daw_ext("bak.rpp-bak", "RPP-BAK");
}

#[test]
fn daw_ext_ptx() {
    assert_daw_ext("s.ptx", "PTX");
}

#[test]
fn daw_ext_song() {
    assert_daw_ext("x.song", "SONG");
}

#[test]
fn daw_ext_reason() {
    assert_daw_ext("r.reason", "REASON");
}

#[test]
fn daw_ext_aup() {
    assert_daw_ext("old.aup", "AUP");
}

#[test]
fn daw_ext_band() {
    assert_daw_ext("gb.band", "BAND");
}

#[test]
fn is_package_ext_band_and_logicx() {
    assert!(is_package_ext(Path::new("/p/My.band")));
    assert!(is_package_ext(Path::new("/p/Proj.logicx")));
}

// ── KVR compare / parse ──────────────────────────────────────────────────────

#[test]
fn kvr_cmp_3_0_0_vs_2_9_9() {
    assert_eq!(
        app_lib::kvr::compare_versions("3.0.0", "2.9.9"),
        Ordering::Greater
    );
}

#[test]
fn kvr_cmp_identical_multi_segment() {
    assert_eq!(
        app_lib::kvr::compare_versions("1.2.3.4", "1.2.3.4"),
        Ordering::Equal
    );
}

#[test]
fn kvr_parse_only_dots_yield_zeros() {
    assert_eq!(app_lib::kvr::parse_version("..."), vec![0, 0, 0, 0]);
}

#[test]
fn kvr_extract_version_release_keyword_snippet() {
    let html = "<div>latest release version 2.3.0 here</div>";
    assert_eq!(
        app_lib::kvr::extract_version(html).as_deref(),
        Some("2.3.0")
    );
}

#[test]
fn kvr_url_re_finds_https_with_query() {
    let t = "x https://cdn.io/dl?v=1&x=y z";
    let m = app_lib::kvr::URL_RE.find(t).unwrap();
    assert!(m.as_str().contains("cdn.io"));
}

// ── history radix_string ─────────────────────────────────────────────────────

#[test]
fn radix_base_2_eight() {
    assert_eq!(radix_string(8, 2), "1000");
}

#[test]
fn radix_base_8_sixty_four() {
    assert_eq!(radix_string(64, 8), "100");
}

#[test]
fn radix_base_36_ten() {
    assert_eq!(radix_string(10, 36), "a");
}

#[test]
fn radix_large_base_10() {
    assert_eq!(radix_string(999_999, 10), "999999");
}

// ── xref ───────────────────────────────────────────────────────────────────

#[test]
fn normalize_strips_stereo_mono_suffix() {
    let n = normalize_plugin_name("Delay (stereo)");
    assert!(!n.contains("stereo"));
}

#[test]
fn normalize_strips_universal_suffix() {
    let n = normalize_plugin_name("Synth [Universal]");
    assert!(!n.contains("universal"));
}

#[test]
fn extract_plugins_txt_extension_returns_empty() {
    assert!(extract_plugins("/tmp/x.txt").is_empty());
}

#[test]
fn extract_plugins_vst3_extension_returns_empty() {
    assert!(extract_plugins("/tmp/x.vst3").is_empty());
}

// ── history DAW diff ─────────────────────────────────────────────────────────

#[test]
fn compute_daw_diff_swap_projects() {
    use app_lib::history::{DawProject, build_daw_snapshot};
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
        size: 2,
        size_formatted: "2 B".into(),
        modified: "m".into(),
    };
    let old = build_daw_snapshot(std::slice::from_ref(&a), &[]);
    let new = build_daw_snapshot(std::slice::from_ref(&b), &[]);
    let d = compute_daw_diff(&old, &new);
    assert_eq!(d.removed.len(), 1);
    assert_eq!(d.added.len(), 1);
    assert_eq!(d.removed[0].path, "/a.als");
    assert_eq!(d.added[0].path, "/b.als");
}

// ── plugin diff version bump both known ─────────────────────────────────────

#[test]
fn compute_plugin_diff_version_bump_reports_previous() {
    use app_lib::history::build_plugin_snapshot;
    let old = build_plugin_snapshot(&[plug("/p.vst3", "1.0.0")], &[], &[]);
    let new = build_plugin_snapshot(&[plug("/p.vst3", "2.0.0")], &[], &[]);
    let d = compute_plugin_diff(&old, &new);
    assert_eq!(d.version_changed.len(), 1);
    assert_eq!(d.version_changed[0].previous_version, "1.0.0");
    assert_eq!(d.version_changed[0].plugin.version, "2.0.0");
}

// ── format_size edge cases ───────────────────────────────────────────────────

#[test]
fn format_size_mb_range() {
    let s = app_lib::format_size(5 * 1024 * 1024);
    assert!(s.contains("MB"), "{s}");
}

#[test]
fn format_size_pb_saturates_to_tb() {
    let huge = 1024_u64.pow(5);
    let s = app_lib::format_size(huge);
    assert!(s.contains("TB"), "large values use top unit: {s}");
}

// ── MidiInfo JSON (Serialize only — no Deserialize on struct) ───────────────

#[test]
fn midi_info_json_has_camel_case_keys() {
    let i = app_lib::midi::MidiInfo {
        format: 1,
        track_count: 2,
        ppqn: 480,
        tempo: 120.0,
        time_signature: "4/4".into(),
        key_signature: "C".into(),
        note_count: 10,
        duration: 4.0,
        track_names: vec!["T".into()],
        channels_used: 4,
    };
    let v = serde_json::to_value(&i).unwrap();
    assert_eq!(v["trackCount"], 2);
    assert_eq!(v["timeSignature"], "4/4");
    assert_eq!(v["channelsUsed"], 4);
}

// ── scanner get_plugin_type ─────────────────────────────────────────────────

#[test]
fn get_plugin_type_aaxplugin_unknown() {
    assert_eq!(app_lib::scanner::get_plugin_type(".aaxplugin"), "Unknown");
}

#[test]
fn get_plugin_type_component_au() {
    assert_eq!(app_lib::scanner::get_plugin_type(".component"), "AU");
}
