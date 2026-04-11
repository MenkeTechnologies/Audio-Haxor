//! Cross-cutting integration tests: history diff-by-id, serde shapes, KVR HTML filters.
//! Written manually in Rust — no generators, no scripts.

use std::cmp::Ordering;

// ── History: `diff_*_scans` returns None when IDs are not in on-disk history ──

#[test]
fn diff_scans_unknown_ids_returns_none() {
    assert!(app_lib::history::diff_scans("no_such_scan_id_old", "no_such_scan_id_new").is_none());
}

#[test]
fn diff_daw_scans_unknown_ids_returns_none() {
    assert!(app_lib::history::diff_daw_scans("missing_daw_old", "missing_daw_new").is_none());
}

#[test]
fn diff_audio_scans_unknown_ids_returns_none() {
    assert!(app_lib::history::diff_audio_scans("missing_audio_old", "missing_audio_new").is_none());
}

#[test]
fn diff_preset_scans_unknown_ids_returns_none() {
    assert!(
        app_lib::history::diff_preset_scans("missing_preset_old", "missing_preset_new").is_none()
    );
}

// ── KVR: date-like strings filtered from `extract_version` (avoid changelog years) ──

#[test]
fn kvr_extract_version_skips_year_dot_month_looking_token() {
    let html = r#"<p>Version: 2024.12.31</p>"#;
    assert!(app_lib::kvr::extract_version(html).is_none());
}

#[test]
fn kvr_extract_version_accepts_non_date_semver_next_to_text() {
    let html = r#"<meta name="version" content="3.14.159" />"#;
    let v = app_lib::kvr::extract_version(html);
    assert_eq!(v.as_deref(), Some("3.14.159"));
}

// ── KVR: `compare_versions` with wide segment values ──────────────────────

#[test]
fn kvr_compare_patch_99_vs_100() {
    assert_eq!(
        app_lib::kvr::compare_versions("1.0.99", "1.0.100"),
        Ordering::Less
    );
}

#[test]
fn kvr_parse_version_large_numeric_segment() {
    assert_eq!(app_lib::kvr::parse_version("2025.11.3"), vec![2025, 11, 3]);
}

// ── Similarity: candidate list with duplicate distances still yields sorted output ──

#[test]
fn find_similar_stable_ordering_when_two_candidates_equidistant() {
    use app_lib::similarity::{AudioFingerprint, find_similar};
    let mk = |path: &str| AudioFingerprint {
        path: path.into(),
        rms: 0.5,
        spectral_centroid: 0.5,
        zero_crossing_rate: 0.1,
        low_band_energy: 0.2,
        mid_band_energy: 0.3,
        high_band_energy: 0.2,
        low_energy_ratio: 0.4,
        attack_time: 0.02,
    };
    let reference = mk("/ref.wav");
    let a = mk("/a.wav");
    let b = mk("/b.wav");
    let out = find_similar(&reference, &[a, b], 2);
    assert_eq!(out.len(), 2);
    assert!(out[0].1 <= out[1].1 + 1e-12);
}

// ── Scanner: `PluginInfo` with optional URL round-trips JSON ──────────────

#[test]
fn plugin_info_serde_preserves_manufacturer_url_some() {
    let info = app_lib::scanner::PluginInfo {
        name: "P".into(),
        path: "/p.vst3".into(),
        plugin_type: "VST3".into(),
        version: "1".into(),
        manufacturer: "M".into(),
        manufacturer_url: Some("https://vendor.example/plugin".into()),
        size: "1 B".into(),
        size_bytes: 1,
        modified: "t".into(),
        architectures: vec!["x64".into()],
    };
    let json = serde_json::to_string(&info).unwrap();
    let back: app_lib::scanner::PluginInfo = serde_json::from_str(&json).unwrap();
    assert_eq!(
        back.manufacturer_url,
        Some("https://vendor.example/plugin".into())
    );
    assert_eq!(back.architectures, vec!["x64".to_string()]);
}

// ── History: `AudioSample` + `DawProject` serde (IPC shape) ─────────────

#[test]
fn audio_sample_json_roundtrip_keeps_optional_fields() {
    let s = app_lib::history::AudioSample {
        name: "s".into(),
        path: "/a.wav".into(),
        directory: "/d".into(),
        format: "wav".into(),
        size: 100,
        size_formatted: "100 B".into(),
        modified: "t".into(),
        duration: Some(12.5),
        channels: Some(2),
        sample_rate: Some(48_000),
        bits_per_sample: Some(24),
    };
    let v = serde_json::to_value(&s).unwrap();
    assert_eq!(v["duration"], 12.5);
    assert_eq!(v["channels"], 2);
    let back: app_lib::history::AudioSample = serde_json::from_value(v).unwrap();
    assert_eq!(back.duration, Some(12.5));
    assert_eq!(back.sample_rate, Some(48_000));
}

#[test]
fn daw_project_json_includes_daw_field() {
    let p = app_lib::history::DawProject {
        name: "song".into(),
        path: "/p.als".into(),
        directory: "/dir".into(),
        format: "als".into(),
        daw: "Ableton Live".into(),
        size: 500,
        size_formatted: "500 B".into(),
        modified: "t".into(),
    };
    let v = serde_json::to_value(&p).unwrap();
    assert_eq!(v["daw"], "Ableton Live");
}

// ── xref: normalization with unicode and mixed case ───────────────────────

#[test]
fn xref_normalize_lowercases_unicode_letters() {
    let n = app_lib::xref::normalize_plugin_name("Réverb (AU)");
    assert!(n.contains("réverb") || n.contains("reverb"));
    assert!(!n.to_lowercase().contains("(au)"));
}

// ── `format_size` — exact power-of-two boundaries ───────────────────────

#[test]
fn format_size_1023_bytes_not_one_kb() {
    assert_eq!(app_lib::format_size(1023), "1023.0 B");
}

#[test]
fn format_size_1025_bytes_shows_kb_not_exactly_one() {
    let s = app_lib::format_size(1025);
    assert!(s.contains("KB"), "{s}");
}

// ── MIDI: `MidiInfo` channels_used serializes ───────────────────────────

#[test]
fn midi_info_channels_used_in_json() {
    let m = app_lib::midi::MidiInfo {
        channels_used: 16,
        ..Default::default()
    };
    let v = serde_json::to_value(&m).unwrap();
    assert_eq!(v["channelsUsed"], 16);
}

// ── File watcher: fresh state ─────────────────────────────────────────────

#[test]
fn file_watcher_state_starts_not_watching() {
    let st = app_lib::file_watcher::FileWatcherState::new();
    assert!(!app_lib::file_watcher::is_watching(&st));
}

// ── KVR: `URL_RE` matches http and https ─────────────────────────────────

#[test]
fn kvr_url_re_matches_https_host() {
    let t = "link=https://cdn.example.com/x/y/z";
    assert_eq!(
        app_lib::kvr::URL_RE.find(t).unwrap().as_str(),
        "https://cdn.example.com/x/y/z"
    );
}

#[test]
fn kvr_url_re_matches_http_host() {
    let t = "see http://legacy.example/a";
    assert_eq!(
        app_lib::kvr::URL_RE.find(t).unwrap().as_str(),
        "http://legacy.example/a"
    );
}

// ── Scanner: `get_plugin_type` is case-sensitive on extension string ────

#[test]
fn scanner_get_plugin_type_dot_uppercase_unknown() {
    assert_eq!(app_lib::scanner::get_plugin_type(".VST3"), "Unknown");
}

// ── Similarity: excluding all candidates leaves empty ───────────────────

#[test]
fn find_similar_all_candidates_share_reference_path_returns_empty() {
    use app_lib::similarity::{AudioFingerprint, find_similar};
    let fp = AudioFingerprint {
        path: "/same.wav".into(),
        rms: 0.5,
        spectral_centroid: 0.5,
        zero_crossing_rate: 0.1,
        low_band_energy: 0.2,
        mid_band_energy: 0.3,
        high_band_energy: 0.2,
        low_energy_ratio: 0.4,
        attack_time: 0.02,
    };
    assert!(find_similar(&fp, &[fp.clone(), fp.clone()], 5).is_empty());
}

// ── History: `PresetFile` serde roundtrip ───────────────────────────────

#[test]
fn preset_file_json_roundtrip() {
    let p = app_lib::history::PresetFile {
        name: "Lead".into(),
        path: "/p/Bank/Lead.fxp".into(),
        directory: "/p/Bank".into(),
        format: "fxp".into(),
        size: 4096,
        size_formatted: "4 KB".into(),
        modified: "2024-01-01".into(),
    };
    let json = serde_json::to_string(&p).unwrap();
    let back: app_lib::history::PresetFile = serde_json::from_str(&json).unwrap();
    assert_eq!(back.path, p.path);
    assert_eq!(back.format, "fxp");
}

// ── `format_size` — zero handled at crate root ───────────────────────────

#[test]
fn format_size_zero_bytes_label() {
    assert_eq!(app_lib::format_size(0), "0 B");
}

// ── KVR: antisymmetry spot check ──────────────────────────────────────────

#[test]
fn kvr_compare_antisymmetric_three_digit_versions() {
    let a = "10.11.12";
    let b = "10.11.13";
    assert_eq!(app_lib::kvr::compare_versions(a, b), Ordering::Less);
    assert_eq!(app_lib::kvr::compare_versions(b, a), Ordering::Greater);
}
