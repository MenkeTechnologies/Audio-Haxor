//! JSON roundtrips for `history` diff/summary/sample types (IPC + on-disk JSON history) and
//! small pure checks for `pdf_meta`, `scanner`, `similarity`, and format_size aliases.

use std::collections::HashMap;
use std::path::Path;

use app_lib::history::{
    AudioHistory, AudioSample, AudioScanDiff, AudioScanSummary, DawHistory, DawProject,
    DawScanDiff, DawScanSnapshot, DawScanSummary, PdfFile, PdfScanDiff, PdfScanSnapshot,
    PdfScanSummary, PresetFile, PresetHistory, PresetScanDiff, PresetScanSnapshot,
    PresetScanSummary, ScanDiff, ScanHistory, ScanSnapshot, ScanSummary, VersionChangedPlugin,
};
use app_lib::scanner::PluginInfo;

fn mk_plugin(path: &str, ver: &str) -> PluginInfo {
    PluginInfo {
        name: "P".into(),
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

fn scan_snap_json_roundtrip() -> ScanSnapshot {
    ScanSnapshot {
        id: "id".into(),
        timestamp: "ts".into(),
        plugin_count: 1,
        plugins: vec![mk_plugin("/a.vst3", "1.0")],
        directories: vec!["/d".into()],
        roots: vec!["/r".into()],
    }
}

// ── Core plugin scan / diff ─────────────────────────────────────────────────────

#[test]
fn scan_snapshot_json_roundtrip() {
    let s = scan_snap_json_roundtrip();
    let json = serde_json::to_string(&s).unwrap();
    let t: ScanSnapshot = serde_json::from_str(&json).unwrap();
    assert_eq!(t.plugin_count, s.plugin_count);
    assert_eq!(t.plugins[0].path, s.plugins[0].path);
}

#[test]
fn scan_history_empty_roundtrip() {
    let h = ScanHistory { scans: vec![] };
    let json = serde_json::to_string(&h).unwrap();
    let back: ScanHistory = serde_json::from_str(&json).unwrap();
    assert!(back.scans.is_empty());
}

#[test]
fn version_changed_plugin_json_roundtrip() {
    let v = VersionChangedPlugin {
        plugin: mk_plugin("/p.vst3", "2.0"),
        previous_version: "1.0".into(),
    };
    let json = serde_json::to_string(&v).unwrap();
    assert!(json.contains("previousVersion"));
    let back: VersionChangedPlugin = serde_json::from_str(&json).unwrap();
    assert_eq!(back.previous_version, "1.0");
    assert_eq!(back.plugin.version, "2.0");
}

#[test]
fn scan_diff_json_roundtrip() {
    let old = ScanSummary {
        id: "o".into(),
        timestamp: "t".into(),
        plugin_count: 0,
        roots: vec![],
    };
    let new = ScanSummary {
        id: "n".into(),
        timestamp: "t2".into(),
        plugin_count: 1,
        roots: vec!["/r".into()],
    };
    let d = ScanDiff {
        old_scan: old,
        new_scan: new,
        added: vec![mk_plugin("/new.vst3", "1.0")],
        removed: vec![],
        version_changed: vec![VersionChangedPlugin {
            plugin: mk_plugin("/same.vst3", "2.0"),
            previous_version: "1.0".into(),
        }],
    };
    let json = serde_json::to_string(&d).unwrap();
    let back: ScanDiff = serde_json::from_str(&json).unwrap();
    assert_eq!(back.added.len(), 1);
    assert_eq!(back.version_changed.len(), 1);
}

// ── Audio / DAW / preset / PDF diffs ───────────────────────────────────────────

#[test]
fn audio_scan_diff_roundtrip() {
    let sum = || AudioScanSummary {
        id: "i".into(),
        timestamp: "t".into(),
        sample_count: 0,
        total_bytes: 0,
        format_counts: HashMap::new(),
        roots: vec![],
    };
    let s = AudioSample {
        name: "a".into(),
        path: "/a.wav".into(),
        directory: "/".into(),
        format: "WAV".into(),
        size: 10,
        size_formatted: "10 B".into(),
        modified: "m".into(),
        duration: None,
        channels: None,
        sample_rate: None,
        bits_per_sample: None,
    };
    let d = AudioScanDiff {
        old_scan: sum(),
        new_scan: sum(),
        added: vec![s.clone()],
        removed: vec![],
    };
    let json = serde_json::to_string(&d).unwrap();
    let back: AudioScanDiff = serde_json::from_str(&json).unwrap();
    assert_eq!(back.added.len(), 1);
}

#[test]
fn daw_scan_diff_roundtrip() {
    let sum = || DawScanSummary {
        id: "i".into(),
        timestamp: "t".into(),
        project_count: 0,
        total_bytes: 0,
        daw_counts: HashMap::new(),
        roots: vec![],
    };
    let p = DawProject {
        name: "p".into(),
        path: "/p.als".into(),
        directory: "/".into(),
        format: "ALS".into(),
        daw: "Ableton Live".into(),
        size: 1,
        size_formatted: "1 B".into(),
        modified: "m".into(),
    };
    let d = DawScanDiff {
        old_scan: sum(),
        new_scan: sum(),
        added: vec![p],
        removed: vec![],
    };
    let json = serde_json::to_string(&d).unwrap();
    let back: DawScanDiff = serde_json::from_str(&json).unwrap();
    assert_eq!(back.added[0].daw, "Ableton Live");
}

#[test]
fn preset_scan_diff_roundtrip() {
    let sum = || PresetScanSummary {
        id: "i".into(),
        timestamp: "t".into(),
        preset_count: 0,
        total_bytes: 0,
        format_counts: HashMap::new(),
        roots: vec![],
    };
    let p = PresetFile {
        name: "x".into(),
        path: "/x.fxp".into(),
        directory: "/".into(),
        format: "FXP".into(),
        size: 1,
        size_formatted: "1 B".into(),
        modified: "m".into(),
    };
    let d = PresetScanDiff {
        old_scan: sum(),
        new_scan: sum(),
        added: vec![p],
        removed: vec![],
    };
    let json = serde_json::to_string(&d).unwrap();
    let back: PresetScanDiff = serde_json::from_str(&json).unwrap();
    assert_eq!(back.added.len(), 1);
}

#[test]
fn pdf_scan_diff_roundtrip() {
    let sum = || PdfScanSummary {
        id: "i".into(),
        timestamp: "t".into(),
        pdf_count: 0,
        total_bytes: 0,
        roots: vec![],
    };
    let p = PdfFile {
        name: "r".into(),
        path: "/r.pdf".into(),
        directory: "/".into(),
        size: 100,
        size_formatted: "100 B".into(),
        modified: "m".into(),
    };
    let d = PdfScanDiff {
        old_scan: sum(),
        new_scan: sum(),
        added: vec![p],
        removed: vec![],
    };
    let json = serde_json::to_string(&d).unwrap();
    let back: PdfScanDiff = serde_json::from_str(&json).unwrap();
    assert_eq!(back.added[0].path, "/r.pdf");
}

// ── Container histories ─────────────────────────────────────────────────────────

#[test]
fn daw_audio_preset_histories_empty_roundtrip() {
    let dh = DawHistory { scans: vec![] };
    let ah = AudioHistory { scans: vec![] };
    let ph = PresetHistory { scans: vec![] };
    for (name, json) in [
        ("daw", serde_json::to_string(&dh).unwrap()),
        ("audio", serde_json::to_string(&ah).unwrap()),
        ("preset", serde_json::to_string(&ph).unwrap()),
    ] {
        let _: serde_json::Value = serde_json::from_str(&json).expect(name);
    }
}

#[test]
fn daw_scan_snapshot_roundtrip() {
    let s = DawScanSnapshot {
        id: "d".into(),
        timestamp: "t".into(),
        project_count: 0,
        total_bytes: 0,
        daw_counts: HashMap::new(),
        projects: vec![],
        roots: vec![],
    };
    let json = serde_json::to_string(&s).unwrap();
    let t: DawScanSnapshot = serde_json::from_str(&json).unwrap();
    assert_eq!(t.project_count, 0);
}

#[test]
fn preset_scan_snapshot_roundtrip() {
    let s = PresetScanSnapshot {
        id: "p".into(),
        timestamp: "t".into(),
        preset_count: 0,
        total_bytes: 0,
        format_counts: HashMap::new(),
        presets: vec![],
        roots: vec![],
    };
    let json = serde_json::to_string(&s).unwrap();
    let t: PresetScanSnapshot = serde_json::from_str(&json).unwrap();
    assert_eq!(t.preset_count, 0);
}

#[test]
fn pdf_scan_snapshot_roundtrip() {
    let s = PdfScanSnapshot {
        id: "f".into(),
        timestamp: "t".into(),
        pdf_count: 0,
        total_bytes: 0,
        pdfs: vec![],
        roots: vec![],
    };
    let json = serde_json::to_string(&s).unwrap();
    let t: PdfScanSnapshot = serde_json::from_str(&json).unwrap();
    assert_eq!(t.pdf_count, 0);
}

#[test]
fn audio_sample_optional_fields_omit_when_none_in_json() {
    let s = AudioSample {
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
    let v = serde_json::to_value(&s).unwrap();
    let o = v.as_object().unwrap();
    assert!(o.get("duration").is_none());
    assert!(o.get("sampleRate").is_none());
}

#[test]
fn audio_sample_optional_fields_present_when_some() {
    let s = AudioSample {
        name: "a".into(),
        path: "/a.wav".into(),
        directory: "/".into(),
        format: "WAV".into(),
        size: 1,
        size_formatted: "1 B".into(),
        modified: "m".into(),
        duration: Some(1.5),
        channels: Some(2),
        sample_rate: Some(44100),
        bits_per_sample: Some(16),
    };
    let v = serde_json::to_value(&s).unwrap();
    let o = v.as_object().unwrap();
    assert_eq!(o.get("sampleRate"), Some(&serde_json::json!(44100)));
    assert_eq!(o.get("bitsPerSample"), Some(&serde_json::json!(16)));
}

// ── `pdf_meta` (batch + missing file) ─────────────────────────────────────────

#[test]
fn pdf_meta_extract_pages_batch_empty_input() {
    assert!(app_lib::pdf_meta::extract_pages_batch(&[]).is_empty());
}

#[test]
fn pdf_meta_extract_page_count_missing_file() {
    assert!(app_lib::pdf_meta::extract_page_count("/no/such/path/ah_pdf_test.pdf").is_none());
}

// ── Scanner: roots + similarity failure path ────────────────────────────────────

#[test]
fn scanner_get_vst_directories_only_lists_existing_paths() {
    for d in app_lib::scanner::get_vst_directories() {
        assert!(
            Path::new(&d).exists(),
            "get_vst_directories filters with exists(); got stale path: {d}"
        );
    }
}

#[test]
fn similarity_compute_fingerprint_non_audio_returns_none() {
    let p = std::env::temp_dir().join("ah_not_audio.txt");
    std::fs::write(&p, b"hello").unwrap();
    let got = p
        .to_str()
        .and_then(|s| app_lib::similarity::compute_fingerprint(s));
    let _ = std::fs::remove_file(&p);
    assert!(got.is_none());
}

#[test]
fn similarity_compute_fingerprint_missing_file_returns_none() {
    assert!(app_lib::similarity::compute_fingerprint("/no/such/ah_fp_test.wav").is_none());
}

// ── `format_size` aliases match crate root ─────────────────────────────────────

#[test]
fn audio_scanner_format_size_matches_lib() {
    let b = 1024 * 1024u64 * 3;
    assert_eq!(
        app_lib::audio_scanner::format_size(b),
        app_lib::format_size(b)
    );
}

#[test]
fn daw_scanner_format_size_matches_lib() {
    let b = 999_999u64;
    assert_eq!(
        app_lib::daw_scanner::format_size(b),
        app_lib::format_size(b)
    );
}

#[test]
fn scanner_module_format_size_matches_lib() {
    assert_eq!(
        app_lib::scanner::format_size(512),
        app_lib::format_size(512)
    );
}

// ── `discover_plugins` empty dir ───────────────────────────────────────────────

#[test]
fn scanner_discover_plugins_empty_directory() {
    let tmp = std::env::temp_dir().join(format!("ah_disc_empty_{}", std::process::id()));
    std::fs::create_dir_all(&tmp).unwrap();
    let out = app_lib::scanner::discover_plugins(&[tmp.to_string_lossy().into_owned()], None);
    let _ = std::fs::remove_dir_all(&tmp);
    assert!(out.is_empty());
}

// ── `read_wav_pcm_pub` invalid file ────────────────────────────────────────────

#[test]
fn bpm_read_wav_pcm_pub_empty_file_returns_none() {
    let tmp = std::env::temp_dir().join(format!("ah_empty_{}.wav", std::process::id()));
    std::fs::write(&tmp, []).unwrap();
    let got = app_lib::bpm::read_wav_pcm_pub(Path::new(&tmp));
    let _ = std::fs::remove_file(&tmp);
    assert!(got.is_none());
}
