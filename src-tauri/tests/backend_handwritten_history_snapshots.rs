//! Handwritten contracts for `history` snapshot builders and pure diff helpers
//! (`compute_*_diff`, `compute_plugin_diff`). Validates path-set semantics, aggregate
//! counters, and version-change rules without touching SQLite or JSON history files.

use std::collections::HashMap;
use std::path::Path;

use app_lib::daw_scanner::{daw_name_for_format, ext_matches, is_package_ext};
use app_lib::history::{
    AudioSample, AudioScanSnapshot, DawProject, DawScanSnapshot, PresetFile, PresetScanSnapshot,
    ScanDiff, ScanSnapshot, build_audio_snapshot, build_daw_snapshot, build_preset_snapshot,
    compute_audio_diff, compute_daw_diff, compute_plugin_diff, compute_preset_diff,
};
use app_lib::scanner::PluginInfo;
use app_lib::{ExportPayload, ExportPlugin};

fn plugin(path: &str, ver: &str) -> PluginInfo {
    PluginInfo {
        name: "Plugin".into(),
        path: path.into(),
        plugin_type: "VST3".into(),
        version: ver.into(),
        manufacturer: "Mfr".into(),
        manufacturer_url: None,
        size: "1.0 KB".into(),
        size_bytes: 1024,
        modified: "2024-01-01".into(),
        architectures: vec![],
    }
}

fn scan_snap(id: &str, plugins: Vec<PluginInfo>) -> ScanSnapshot {
    ScanSnapshot {
        id: id.into(),
        timestamp: "ts".into(),
        plugin_count: plugins.len(),
        plugins,
        directories: vec!["/Plugins".into()],
        roots: vec!["/Plugins".into()],
    }
}

fn audio_sample(path: &str, format: &str, size: u64) -> AudioSample {
    AudioSample {
        name: path.rsplit('/').next().unwrap_or("x").into(),
        path: path.into(),
        directory: "/audio".into(),
        format: format.into(),
        size,
        size_formatted: app_lib::format_size(size),
        modified: "2024-01-01".into(),
        duration: None,
        channels: None,
        sample_rate: None,
        bits_per_sample: None,
    }
}

fn audio_snap(id: &str, samples: Vec<AudioSample>, roots: Vec<String>) -> AudioScanSnapshot {
    let mut format_counts = HashMap::new();
    let mut total_bytes = 0u64;
    for s in &samples {
        *format_counts.entry(s.format.clone()).or_insert(0) += 1;
        total_bytes += s.size;
    }
    AudioScanSnapshot {
        id: id.into(),
        timestamp: "ts".into(),
        sample_count: samples.len(),
        total_bytes,
        format_counts,
        samples,
        roots,
    }
}

fn daw_project(path: &str, daw: &str, size: u64) -> DawProject {
    DawProject {
        name: path.rsplit('/').next().unwrap_or("p").into(),
        path: path.into(),
        directory: "/daw".into(),
        format: "ALS".into(),
        daw: daw.into(),
        size,
        size_formatted: app_lib::format_size(size),
        modified: "2024-01-01".into(),
    }
}

fn daw_snap(id: &str, projects: Vec<DawProject>, roots: Vec<String>) -> DawScanSnapshot {
    let mut daw_counts = HashMap::new();
    let mut total_bytes = 0u64;
    for p in &projects {
        *daw_counts.entry(p.daw.clone()).or_insert(0) += 1;
        total_bytes += p.size;
    }
    DawScanSnapshot {
        id: id.into(),
        timestamp: "ts".into(),
        project_count: projects.len(),
        total_bytes,
        daw_counts,
        projects,
        roots,
    }
}

fn preset_file(path: &str, format: &str, size: u64) -> PresetFile {
    PresetFile {
        name: path.rsplit('/').next().unwrap_or("x").into(),
        path: path.into(),
        directory: "/presets".into(),
        format: format.into(),
        size,
        size_formatted: app_lib::format_size(size),
        modified: "2024-01-01".into(),
    }
}

fn preset_snap(id: &str, presets: Vec<PresetFile>, roots: Vec<String>) -> PresetScanSnapshot {
    let mut format_counts = HashMap::new();
    let mut total_bytes = 0u64;
    for p in &presets {
        *format_counts.entry(p.format.clone()).or_insert(0) += 1;
        total_bytes += p.size;
    }
    PresetScanSnapshot {
        id: id.into(),
        timestamp: "ts".into(),
        preset_count: presets.len(),
        total_bytes,
        format_counts,
        presets,
        roots,
    }
}

// ── `compute_plugin_diff` (version rules) ───────────────────────────────────────

#[test]
fn compute_plugin_diff_version_bump_same_path_emits_version_changed_only() {
    let old = scan_snap("a", vec![plugin("/a/X.vst3", "1.0.0")]);
    let new = scan_snap("b", vec![plugin("/a/X.vst3", "2.0.0")]);
    let d = compute_plugin_diff(&old, &new);
    assert!(d.added.is_empty());
    assert!(d.removed.is_empty());
    assert_eq!(d.version_changed.len(), 1);
    assert_eq!(d.version_changed[0].previous_version, "1.0.0");
    assert_eq!(d.version_changed[0].plugin.version, "2.0.0");
}

#[test]
fn compute_plugin_diff_unknown_to_known_same_path_skips_version_changed() {
    let old = scan_snap("a", vec![plugin("/a/X.vst3", "Unknown")]);
    let new = scan_snap("b", vec![plugin("/a/X.vst3", "1.0.0")]);
    let d = compute_plugin_diff(&old, &new);
    assert!(d.version_changed.is_empty());
}

#[test]
fn compute_plugin_diff_known_to_unknown_same_path_skips_version_changed() {
    let old = scan_snap("a", vec![plugin("/a/X.vst3", "1.0.0")]);
    let new = scan_snap("b", vec![plugin("/a/X.vst3", "Unknown")]);
    let d = compute_plugin_diff(&old, &new);
    assert!(d.version_changed.is_empty());
}

#[test]
fn compute_plugin_diff_both_unknown_same_path_no_version_changed() {
    let old = scan_snap("a", vec![plugin("/a/X.vst3", "Unknown")]);
    let new = scan_snap("b", vec![plugin("/a/X.vst3", "Unknown")]);
    let d = compute_plugin_diff(&old, &new);
    assert!(d.version_changed.is_empty());
}

#[test]
fn compute_plugin_diff_add_and_remove_distinct_paths_no_version_changed() {
    let old = scan_snap("a", vec![plugin("/old/Y.vst3", "1.0.0")]);
    let new = scan_snap("b", vec![plugin("/new/Z.vst3", "1.0.0")]);
    let d = compute_plugin_diff(&old, &new);
    assert_eq!(d.added.len(), 1);
    assert_eq!(d.removed.len(), 1);
    assert!(d.version_changed.is_empty());
}

#[test]
fn compute_plugin_diff_two_paths_both_version_bump() {
    let old = scan_snap(
        "a",
        vec![plugin("/p/a.vst3", "1.0.0"), plugin("/p/b.vst3", "2.0.0")],
    );
    let new = scan_snap(
        "b",
        vec![plugin("/p/a.vst3", "1.1.0"), plugin("/p/b.vst3", "3.0.0")],
    );
    let d = compute_plugin_diff(&old, &new);
    assert_eq!(d.version_changed.len(), 2);
}

#[test]
fn compute_plugin_diff_same_known_version_no_version_changed_entry() {
    let old = scan_snap("a", vec![plugin("/p/a.vst3", "1.0.0")]);
    let new = scan_snap("b", vec![plugin("/p/a.vst3", "1.0.0")]);
    let d = compute_plugin_diff(&old, &new);
    assert!(d.version_changed.is_empty());
}

// ── `compute_audio_diff` ────────────────────────────────────────────────────────

#[test]
fn compute_audio_diff_swap_reciprocates_added_removed() {
    let a = audio_snap("a", vec![audio_sample("/s/1.wav", "WAV", 100)], vec![]);
    let b = audio_snap("b", vec![audio_sample("/s/2.wav", "WAV", 200)], vec![]);
    let ab = compute_audio_diff(&a, &b);
    let ba = compute_audio_diff(&b, &a);
    assert_eq!(ab.added.len(), ba.removed.len());
    assert_eq!(ab.removed.len(), ba.added.len());
}

#[test]
fn compute_audio_diff_same_paths_no_delta() {
    let samples = vec![audio_sample("/k/a.flac", "FLAC", 999)];
    let old = audio_snap("o", samples.clone(), vec!["/k".into()]);
    let new = audio_snap("n", samples, vec!["/k".into()]);
    let d = compute_audio_diff(&old, &new);
    assert!(d.added.is_empty());
    assert!(d.removed.is_empty());
}

#[test]
fn compute_audio_diff_empty_to_nonempty_all_added() {
    let old = audio_snap("o", vec![], vec![]);
    let new = audio_snap(
        "n",
        vec![audio_sample("/x/t.wav", "WAV", 10)],
        vec!["/x".into()],
    );
    let d = compute_audio_diff(&old, &new);
    assert_eq!(d.added.len(), 1);
    assert!(d.removed.is_empty());
}

#[test]
fn compute_audio_diff_nonempty_to_empty_all_removed() {
    let old = audio_snap("o", vec![audio_sample("/gone.wav", "WAV", 10)], vec![]);
    let new = audio_snap("n", vec![], vec![]);
    let d = compute_audio_diff(&old, &new);
    assert_eq!(d.removed.len(), 1);
    assert!(d.added.is_empty());
}

#[test]
fn compute_audio_diff_net_two_added_one_removed() {
    let old = audio_snap(
        "o",
        vec![
            audio_sample("/keep.wav", "WAV", 10),
            audio_sample("/gone.wav", "WAV", 20),
        ],
        vec![],
    );
    let new = audio_snap(
        "n",
        vec![
            audio_sample("/keep.wav", "WAV", 10),
            audio_sample("/new1.wav", "WAV", 30),
            audio_sample("/new2.wav", "WAV", 40),
        ],
        vec![],
    );
    let d = compute_audio_diff(&old, &new);
    assert_eq!(d.added.len(), 2);
    assert_eq!(d.removed.len(), 1);
}

// ── `compute_daw_diff` ──────────────────────────────────────────────────────────

#[test]
fn compute_daw_diff_swap_reciprocates_added_removed() {
    let a = daw_snap(
        "a",
        vec![daw_project("/p/a.als", "Ableton Live", 1000)],
        vec![],
    );
    let b = daw_snap(
        "b",
        vec![daw_project("/p/b.als", "Ableton Live", 2000)],
        vec![],
    );
    let ab = compute_daw_diff(&a, &b);
    let ba = compute_daw_diff(&b, &a);
    assert_eq!(ab.added.len(), ba.removed.len());
    assert_eq!(ab.removed.len(), ba.added.len());
}

#[test]
fn compute_daw_diff_same_project_paths_stable() {
    let projects = vec![daw_project("/sessions/live.als", "Ableton Live", 500)];
    let old = daw_snap("o", projects.clone(), vec!["/sessions".into()]);
    let new = daw_snap("n", projects, vec!["/sessions".into()]);
    let d = compute_daw_diff(&old, &new);
    assert!(d.added.is_empty());
    assert!(d.removed.is_empty());
}

#[test]
fn compute_daw_diff_two_daw_types_in_one_diff() {
    let old = daw_snap("o", vec![daw_project("/a/rpp.RPP", "REAPER", 100)], vec![]);
    let new = daw_snap(
        "n",
        vec![
            daw_project("/a/rpp.RPP", "REAPER", 100),
            daw_project("/b/song.SONG", "Studio One", 200),
        ],
        vec![],
    );
    let d = compute_daw_diff(&old, &new);
    assert_eq!(d.added.len(), 1);
    assert!(d.removed.is_empty());
}

// ── `compute_preset_diff` ───────────────────────────────────────────────────────

#[test]
fn compute_preset_diff_swap_reciprocates_added_removed() {
    let a = preset_snap("a", vec![preset_file("/bank/a.fxp", "FXP", 50)], vec![]);
    let b = preset_snap("b", vec![preset_file("/bank/b.h2p", "H2P", 60)], vec![]);
    let ab = compute_preset_diff(&a, &b);
    let ba = compute_preset_diff(&b, &a);
    assert_eq!(ab.added.len(), ba.removed.len());
    assert_eq!(ab.removed.len(), ba.added.len());
}

#[test]
fn compute_preset_diff_identical_paths_empty_delta() {
    let presets = vec![preset_file("/p/x.aupreset", "AUPRESET", 12)];
    let old = preset_snap("o", presets.clone(), vec![]);
    let new = preset_snap("n", presets, vec![]);
    let d = compute_preset_diff(&old, &new);
    assert!(d.added.is_empty());
    assert!(d.removed.is_empty());
}

#[test]
fn compute_preset_diff_second_file_on_new_scan_is_added() {
    let a = preset_snap("a", vec![preset_file("/one/a.fxp", "FXP", 10)], vec![]);
    let b = preset_snap(
        "b",
        vec![
            preset_file("/one/a.fxp", "FXP", 10),
            preset_file("/two/b.fxp", "FXP", 20),
        ],
        vec![],
    );
    let d = compute_preset_diff(&a, &b);
    assert_eq!(d.added.len(), 1);
    assert_eq!(d.added[0].path, "/two/b.fxp");
}

// ── `build_*_snapshot` aggregates (no file I/O) ───────────────────────────────────

#[test]
fn build_audio_snapshot_format_counts_and_total_bytes() {
    let s = build_audio_snapshot(
        &[
            audio_sample("/a/1.wav", "WAV", 100),
            audio_sample("/a/2.wav", "WAV", 400),
            audio_sample("/b/3.flac", "FLAC", 500),
        ],
        &["/roots".into()],
    );
    assert_eq!(s.sample_count, 3);
    assert_eq!(s.total_bytes, 1000);
    assert_eq!(s.format_counts.get("WAV"), Some(&2));
    assert_eq!(s.format_counts.get("FLAC"), Some(&1));
    assert_eq!(s.roots, vec!["/roots"]);
}

#[test]
fn build_audio_snapshot_empty_samples() {
    let s = build_audio_snapshot(&[], &[]);
    assert_eq!(s.sample_count, 0);
    assert_eq!(s.total_bytes, 0);
    assert!(s.format_counts.is_empty());
}

#[test]
fn build_daw_snapshot_daw_counts_and_total_bytes() {
    let s = build_daw_snapshot(
        &[
            daw_project("/p/a.als", "Ableton Live", 100),
            daw_project("/p/b.als", "Ableton Live", 300),
            daw_project("/p/c.rpp", "REAPER", 50),
        ],
        &["/p".into()],
    );
    assert_eq!(s.project_count, 3);
    assert_eq!(s.total_bytes, 450);
    assert_eq!(s.daw_counts.get("Ableton Live"), Some(&2));
    assert_eq!(s.daw_counts.get("REAPER"), Some(&1));
}

#[test]
fn build_daw_snapshot_empty() {
    let s = build_daw_snapshot(&[], &[]);
    assert_eq!(s.project_count, 0);
    assert!(s.daw_counts.is_empty());
}

#[test]
fn build_preset_snapshot_format_counts_and_bytes() {
    let s = build_preset_snapshot(
        &[
            preset_file("/x/a.fxp", "FXP", 10),
            preset_file("/x/b.fxp", "FXP", 30),
            preset_file("/x/c.tfx", "TFX", 60),
        ],
        &["/x".into()],
    );
    assert_eq!(s.preset_count, 3);
    assert_eq!(s.total_bytes, 100);
    assert_eq!(s.format_counts.get("FXP"), Some(&2));
    assert_eq!(s.format_counts.get("TFX"), Some(&1));
}

#[test]
fn build_preset_snapshot_empty() {
    let s = build_preset_snapshot(&[], &[]);
    assert_eq!(s.preset_count, 0);
    assert!(s.format_counts.is_empty());
}

// ── Export payload serde (IPC / file interchange) ─────────────────────────────

#[test]
fn export_payload_roundtrip_preserves_plugins() {
    let ep = ExportPayload {
        version: "test-1".into(),
        exported_at: "2025-01-01T00:00:00Z".into(),
        plugins: vec![ExportPlugin {
            name: "Synth".into(),
            plugin_type: "VST3".into(),
            version: "3.2.1".into(),
            manufacturer: "Co".into(),
            manufacturer_url: None,
            path: "/P/Synth.vst3".into(),
            size: "2.0 MB".into(),
            size_bytes: 2 * 1024 * 1024,
            modified: "2025-01-01".into(),
            architectures: vec!["arm64".into()],
        }],
    };
    let json = serde_json::to_string(&ep).expect("to_string");
    let back: ExportPayload = serde_json::from_str(&json).expect("from_str");
    assert_eq!(back.plugins.len(), 1);
    assert_eq!(back.plugins[0].path, "/P/Synth.vst3");
    assert_eq!(back.plugins[0].size_bytes, 2 * 1024 * 1024);
}

#[test]
fn export_plugin_json_uses_camel_case_size_bytes_alias() {
    let json = r#"{
        "name": "A",
        "type": "AU",
        "version": "1",
        "manufacturer": "B",
        "path": "/A.component",
        "size": "0 B",
        "sizeBytes": 0,
        "modified": "x"
    }"#;
    let p: ExportPlugin = serde_json::from_str(json).expect("deserialize");
    assert_eq!(p.size_bytes, 0);
    assert_eq!(p.plugin_type, "AU");
}

#[test]
fn scan_snapshot_serde_roundtrip_plugin_count() {
    let s = scan_snap(
        "id1",
        vec![plugin("/a.vst3", "1.0.0"), plugin("/b.vst3", "1.0.0")],
    );
    let json = serde_json::to_string(&s).expect("ser");
    let t: ScanSnapshot = serde_json::from_str(&json).expect("de");
    assert_eq!(t.plugin_count, 2);
    assert_eq!(t.plugins.len(), 2);
}

#[test]
fn audio_scan_snapshot_serde_roundtrip() {
    let s = audio_snap(
        "sid",
        vec![audio_sample("/x/a.wav", "WAV", 100)],
        vec!["/x".into()],
    );
    let json = serde_json::to_string(&s).unwrap();
    let t: AudioScanSnapshot = serde_json::from_str(&json).unwrap();
    assert_eq!(t.sample_count, 1);
    assert_eq!(t.total_bytes, 100);
}

#[test]
fn daw_scan_snapshot_serde_roundtrip() {
    let s = daw_snap(
        "did",
        vec![daw_project("/p/x.als", "Ableton Live", 50)],
        vec![],
    );
    let json = serde_json::to_string(&s).unwrap();
    let t: DawScanSnapshot = serde_json::from_str(&json).unwrap();
    assert_eq!(t.project_count, 1);
}

#[test]
fn preset_scan_snapshot_serde_roundtrip() {
    let s = preset_snap("pid", vec![preset_file("/b/a.fxp", "FXP", 10)], vec![]);
    let json = serde_json::to_string(&s).unwrap();
    let t: PresetScanSnapshot = serde_json::from_str(&json).unwrap();
    assert_eq!(t.preset_count, 1);
}

#[test]
fn scan_diff_serde_roundtrip_from_plugin_diff() {
    let old = scan_snap("o", vec![plugin("/only.vst3", "1.0.0")]);
    let new = scan_snap("n", vec![plugin("/only.vst3", "2.0.0")]);
    let d = compute_plugin_diff(&old, &new);
    let json = serde_json::to_string(&d).unwrap();
    let back: ScanDiff = serde_json::from_str(&json).unwrap();
    assert_eq!(back.version_changed.len(), 1);
    assert_eq!(back.version_changed[0].plugin.path, "/only.vst3");
}

// ── `daw_scanner` pure helpers (extension table vs `daw_name_for_format`) ───────

#[test]
fn daw_ext_matches_dawproject_and_ardour_suffixes() {
    assert_eq!(
        ext_matches(Path::new("/proj/session.dawproject")).as_deref(),
        Some("DAWPROJECT")
    );
    assert_eq!(
        ext_matches(Path::new("/work/session.ardour")).as_deref(),
        Some("ARDOUR")
    );
}

#[test]
fn daw_ext_matches_reaper_backup_suffix() {
    assert_eq!(
        ext_matches(Path::new("/bak/track.RPP-BAK")).as_deref(),
        Some("RPP-BAK")
    );
}

#[test]
fn daw_is_package_ext_logicx_and_not_als() {
    assert!(is_package_ext(Path::new("/Music/Beat.logicx")));
    assert!(!is_package_ext(Path::new("/Projects/live.als")));
}

#[test]
fn daw_name_for_format_unknown_string() {
    assert_eq!(daw_name_for_format("NOT_A_REAL_CODE"), "Unknown");
}

// ── Export edge cases ───────────────────────────────────────────────────────────

#[test]
fn export_payload_empty_plugins_array_roundtrips() {
    let ep = ExportPayload {
        version: "1".into(),
        exported_at: "t".into(),
        plugins: vec![],
    };
    let json = serde_json::to_string(&ep).unwrap();
    let back: ExportPayload = serde_json::from_str(&json).unwrap();
    assert!(back.plugins.is_empty());
}

#[test]
fn export_payload_preserves_version_and_exported_at() {
    let ep = ExportPayload {
        version: "9.9.9".into(),
        exported_at: "2026-04-04T12:00:00Z".into(),
        plugins: vec![],
    };
    let json = serde_json::to_string(&ep).unwrap();
    let back: ExportPayload = serde_json::from_str(&json).unwrap();
    assert_eq!(back.version, "9.9.9");
    assert_eq!(back.exported_at, "2026-04-04T12:00:00Z");
}
