//! Snapshot diff logic for audio / DAW / preset scans, plus a few sharp edge cases.

use std::collections::HashMap;

use app_lib::history::{
    compute_audio_diff, compute_daw_diff, compute_preset_diff, AudioSample, AudioScanSnapshot,
    DawProject, DawScanSnapshot, PresetFile, PresetScanSnapshot,
};

fn audio_sample(path: &str) -> AudioSample {
    AudioSample {
        name: "sample".into(),
        path: path.into(),
        directory: "/audio".into(),
        format: "wav".into(),
        size: 100,
        size_formatted: "100 B".into(),
        modified: "2024-01-01".into(),
        duration: None,
        channels: None,
        sample_rate: None,
        bits_per_sample: None,
    }
}

fn audio_snapshot(id: &str, samples: Vec<AudioSample>) -> AudioScanSnapshot {
    let mut format_counts = HashMap::new();
    if !samples.is_empty() {
        format_counts.insert("wav".into(), samples.len());
    }
    let total_bytes: u64 = samples.iter().map(|s| s.size).sum();
    AudioScanSnapshot {
        id: id.into(),
        timestamp: "t0".into(),
        sample_count: samples.len(),
        total_bytes,
        format_counts,
        samples,
        roots: vec!["/roots/a".into()],
    }
}

#[test]
fn compute_audio_diff_added_and_removed_by_path() {
    let old = audio_snapshot(
        "old",
        vec![audio_sample("/keep/x.wav"), audio_sample("/gone/y.wav")],
    );
    let new = audio_snapshot(
        "new",
        vec![audio_sample("/keep/x.wav"), audio_sample("/new/z.wav")],
    );
    let d = compute_audio_diff(&old, &new);
    assert_eq!(d.added.len(), 1);
    assert_eq!(d.added[0].path, "/new/z.wav");
    assert_eq!(d.removed.len(), 1);
    assert_eq!(d.removed[0].path, "/gone/y.wav");
    assert_eq!(d.old_scan.id, "old");
    assert_eq!(d.new_scan.sample_count, 2);
}

fn daw_project(path: &str, label: &str) -> DawProject {
    DawProject {
        name: "proj".into(),
        path: path.into(),
        directory: "/daw".into(),
        format: "als".into(),
        daw: label.into(),
        size: 200,
        size_formatted: "200 B".into(),
        modified: "2024-01-01".into(),
    }
}

fn daw_snapshot(id: &str, projects: Vec<DawProject>) -> DawScanSnapshot {
    let mut daw_counts = HashMap::new();
    for p in &projects {
        *daw_counts.entry(p.daw.clone()).or_insert(0) += 1;
    }
    let total_bytes: u64 = projects.iter().map(|p| p.size).sum();
    DawScanSnapshot {
        id: id.into(),
        timestamp: "t1".into(),
        project_count: projects.len(),
        total_bytes,
        daw_counts,
        projects,
        roots: vec![],
    }
}

#[test]
fn compute_daw_diff_added_and_removed_by_path() {
    let old = daw_snapshot(
        "o",
        vec![
            daw_project("/p/a.als", "ALS"),
            daw_project("/p/b.als", "ALS"),
        ],
    );
    let new = daw_snapshot(
        "n",
        vec![
            daw_project("/p/a.als", "ALS"),
            daw_project("/p/c.als", "ALS"),
        ],
    );
    let d = compute_daw_diff(&old, &new);
    assert_eq!(d.added.len(), 1);
    assert_eq!(d.added[0].path, "/p/c.als");
    assert_eq!(d.removed.len(), 1);
    assert_eq!(d.removed[0].path, "/p/b.als");
}

fn preset_file(path: &str) -> PresetFile {
    PresetFile {
        name: "preset".into(),
        path: path.into(),
        directory: "/pre".into(),
        format: "fxp".into(),
        size: 50,
        size_formatted: "50 B".into(),
        modified: "2024-01-01".into(),
    }
}

fn preset_snapshot(id: &str, presets: Vec<PresetFile>) -> PresetScanSnapshot {
    let mut format_counts = HashMap::new();
    if !presets.is_empty() {
        format_counts.insert("fxp".into(), presets.len());
    }
    let total_bytes: u64 = presets.iter().map(|p| p.size).sum();
    PresetScanSnapshot {
        id: id.into(),
        timestamp: "t2".into(),
        preset_count: presets.len(),
        total_bytes,
        format_counts,
        presets,
        roots: vec![],
    }
}

#[test]
fn compute_preset_diff_added_and_removed_by_path() {
    let old = preset_snapshot("old", vec![preset_file("/a.fxp"), preset_file("/b.fxp")]);
    let new = preset_snapshot("new", vec![preset_file("/b.fxp"), preset_file("/c.fxp")]);
    let d = compute_preset_diff(&old, &new);
    assert_eq!(d.added.len(), 1);
    assert_eq!(d.added[0].path, "/c.fxp");
    assert_eq!(d.removed.len(), 1);
    assert_eq!(d.removed[0].path, "/a.fxp");
}

#[test]
fn compute_audio_diff_no_overlap_means_full_replace() {
    let old = audio_snapshot("a", vec![audio_sample("/only/old.wav")]);
    let new = audio_snapshot("b", vec![audio_sample("/only/new.wav")]);
    let d = compute_audio_diff(&old, &new);
    assert_eq!(d.added.len(), 1);
    assert_eq!(d.removed.len(), 1);
}

// ── Key / LUFS: reject non-audio paths early ─────────────────────────────

#[test]
fn detect_key_unsupported_extension_returns_none() {
    assert!(app_lib::key_detect::detect_key("/tmp/readme.txt").is_none());
}

#[test]
fn measure_lufs_nonexistent_returns_none() {
    assert!(app_lib::lufs::measure_lufs("/nonexistent/audio_haxor/missing.wav").is_none());
}

// ── KVR: `get` token in download URL (regex alternation) ─────────────────

#[test]
fn kvr_extract_download_url_accepts_get_href() {
    let html = r#"<a href="https://files.example.com/get/installer-v3">DL</a>"#;
    let r = app_lib::kvr::extract_download_url(html).expect("get href");
    assert!(r.0.contains("get"));
}

// ── Similarity: self-path excluded even if features identical ────────────

#[test]
fn find_similar_skips_candidate_with_same_path_as_reference() {
    use app_lib::similarity::{find_similar, AudioFingerprint};
    let fp = AudioFingerprint {
        path: "/same.wav".into(),
        rms: 0.5,
        spectral_centroid: 0.5,
        zero_crossing_rate: 0.1,
        low_band_energy: 0.2,
        mid_band_energy: 0.3,
        high_band_energy: 0.2,
        low_energy_ratio: 0.4,
        attack_time: 0.01,
    };
    let out = find_similar(&fp, std::slice::from_ref(&fp), 5);
    assert!(out.is_empty());
}
