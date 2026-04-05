//! Handwritten backend contracts: PDF scan diff/snapshot (SQLite path), paginated query
//! defaults, semver ordering traps, similarity truncation, and cheap failure modes for
//! analysis helpers. Complements `behavioral_*` grids with explicit intent-heavy cases.

use std::cmp::Ordering;
use std::path::Path;

use app_lib::db::AudioQueryParams;
use app_lib::xref::normalize_plugin_name;
use app_lib::history::{
    build_pdf_snapshot, compute_pdf_diff, PdfFile, PdfScanSnapshot,
};
use app_lib::kvr::{compare_versions, extract_version, parse_version};
use app_lib::similarity::{find_similar, fingerprint_distance, AudioFingerprint};
use app_lib::scanner::PluginInfo;

fn pdf_file(path: &str, size: u64) -> PdfFile {
    PdfFile {
        name: Path::new(path)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("x.pdf")
            .to_string(),
        path: path.into(),
        directory: "/tmp".into(),
        size,
        size_formatted: app_lib::format_size(size),
        modified: "2024-01-01T00:00:00Z".into(),
    }
}

fn pdf_snap(id: &str, pdfs: Vec<PdfFile>, roots: Vec<String>) -> PdfScanSnapshot {
    let total_bytes: u64 = pdfs.iter().map(|p| p.size).sum();
    PdfScanSnapshot {
        id: id.into(),
        timestamp: "t0".into(),
        pdf_count: pdfs.len(),
        total_bytes,
        pdfs,
        roots,
    }
}

// ── PDF: `build_pdf_snapshot` / `compute_pdf_diff` (used by DB-backed PDF scans) ──

#[test]
fn build_pdf_snapshot_aggregates_count_bytes_and_roots() {
    let pdfs = vec![
        pdf_file("/vault/a.pdf", 100),
        pdf_file("/vault/b.pdf", 400),
    ];
    let s = build_pdf_snapshot(&pdfs, &["/vault".into()]);
    assert_eq!(s.pdf_count, 2);
    assert_eq!(s.total_bytes, 500);
    assert_eq!(s.roots, vec!["/vault"]);
}

#[test]
fn compute_pdf_diff_identical_snapshots_empty_delta() {
    let a = pdf_snap("old", vec![pdf_file("/x/1.pdf", 10)], vec![]);
    let d = compute_pdf_diff(&a, &a);
    assert!(d.added.is_empty());
    assert!(d.removed.is_empty());
}

#[test]
fn compute_pdf_diff_same_paths_both_sides_no_added_or_removed() {
    let pdfs = vec![pdf_file("/shared/report.pdf", 500)];
    let old = pdf_snap("a", pdfs.clone(), vec!["/root".into()]);
    let new = pdf_snap("b", pdfs, vec!["/root".into()]);
    let d = compute_pdf_diff(&old, &new);
    assert!(d.added.is_empty());
    assert!(d.removed.is_empty());
}

#[test]
fn compute_pdf_diff_detects_added_and_removed_paths() {
    let old = pdf_snap(
        "o",
        vec![pdf_file("/only/old.pdf", 1)],
        vec![],
    );
    let new = pdf_snap(
        "n",
        vec![pdf_file("/only/new.pdf", 2)],
        vec![],
    );
    let d = compute_pdf_diff(&old, &new);
    assert_eq!(d.added.len(), 1);
    assert_eq!(d.removed.len(), 1);
    assert_eq!(d.added[0].path, "/only/new.pdf");
    assert_eq!(d.removed[0].path, "/only/old.pdf");
}

#[test]
fn compute_pdf_diff_swap_swaps_added_and_removed() {
    let left = pdf_snap("L", vec![pdf_file("/p/a.pdf", 3)], vec![]);
    let right = pdf_snap("R", vec![pdf_file("/p/b.pdf", 4)], vec![]);
    let lr = compute_pdf_diff(&left, &right);
    let rl = compute_pdf_diff(&right, &left);
    assert_eq!(lr.added.len(), rl.removed.len());
    assert_eq!(lr.removed.len(), rl.added.len());
    assert_eq!(lr.added[0].path, rl.removed[0].path);
    assert_eq!(lr.removed[0].path, rl.added[0].path);
}

#[test]
fn compute_pdf_diff_union_path_stable_when_one_side_empty() {
    let old = pdf_snap("o", vec![], vec![]);
    let new = pdf_snap(
        "n",
        vec![pdf_file("/inbox/manual.pdf", 99)],
        vec!["/inbox".into()],
    );
    let d = compute_pdf_diff(&old, &new);
    assert_eq!(d.added.len(), 1);
    assert!(d.removed.is_empty());
    let d2 = compute_pdf_diff(&new, &old);
    assert_eq!(d2.removed.len(), 1);
    assert!(d2.added.is_empty());
}

#[test]
fn build_pdf_snapshot_empty_pdfs_zero_count_and_bytes() {
    let s = build_pdf_snapshot(&[], &[]);
    assert_eq!(s.pdf_count, 0);
    assert_eq!(s.total_bytes, 0);
    assert!(s.pdfs.is_empty());
}

// ── DB: `AudioQueryParams` — integration-level JSON (snake_case matches Tauri IPC) ─
// (defaults + partial overrides are also unit-tested in `db.rs`.)

#[test]
fn audio_query_params_snake_case_roundtrip_via_serde_json_value() {
    let v = serde_json::json!({
        "scan_id": "s1",
        "search": "snare",
        "format_filter": "WAV",
        "sort_key": "bpm",
        "sort_asc": false,
        "offset": 40,
        "limit": 15
    });
    let p: AudioQueryParams = serde_json::from_value(v.clone()).expect("from_value");
    assert_eq!(p.scan_id.as_deref(), Some("s1"));
    assert_eq!(p.search.as_deref(), Some("snare"));
    assert_eq!(p.format_filter.as_deref(), Some("WAV"));
    assert_eq!(p.sort_key, "bpm");
    assert!(!p.sort_asc);
    assert_eq!(p.offset, 40);
    assert_eq!(p.limit, 15);
}

// ── KVR: numeric semver semantics (not lexicographic string compare) ────────────

#[test]
fn kvr_compare_versions_numeric_segments_not_strings() {
    assert_eq!(compare_versions("1.10", "1.9"), Ordering::Greater);
    assert_eq!(compare_versions("2.0", "1.99"), Ordering::Greater);
}

#[test]
fn kvr_compare_versions_transitive_on_sorted_chain() {
    let chain = ["0.0.1", "0.1.0", "1.0.0", "1.2.0", "1.10.0", "10.0.0"];
    for i in 0..chain.len() {
        for j in (i + 1)..chain.len() {
            assert_eq!(
                compare_versions(chain[i], chain[j]),
                Ordering::Less,
                "{} should be < {}",
                chain[i],
                chain[j]
            );
        }
    }
}

#[test]
fn kvr_parse_version_splits_numeric_segments_for_compare() {
    let a = parse_version("1.10");
    let b = parse_version("1.9");
    assert_eq!(a, vec![1, 10]);
    assert_eq!(b, vec![1, 9]);
}

#[test]
fn kvr_compare_versions_ignores_leading_zero_segments() {
    assert_eq!(compare_versions("01.02.03", "1.2.3"), Ordering::Equal);
    assert_eq!(compare_versions("007.0.0", "7.0.0"), Ordering::Equal);
}

#[test]
fn kvr_compare_versions_empty_strings_equal() {
    assert_eq!(compare_versions("", ""), Ordering::Equal);
}

#[test]
fn kvr_extract_version_finds_label_after_version_colon() {
    let html = r#"<p>Version: 4.8.2</p><footer>footer</footer>"#;
    assert_eq!(extract_version(html).as_deref(), Some("4.8.2"));
}

// ── Similarity: `find_similar` respects `max_results` ───────────────────────────

fn fp(path: &str, rms: f64) -> AudioFingerprint {
    AudioFingerprint {
        path: path.into(),
        rms,
        spectral_centroid: 0.1,
        zero_crossing_rate: 0.05,
        low_band_energy: 0.3,
        mid_band_energy: 0.4,
        high_band_energy: 0.3,
        low_energy_ratio: 0.2,
        attack_time: 0.05,
    }
}

#[test]
fn find_similar_truncates_to_max_results() {
    let reference = fp("/ref.wav", 0.5);
    let candidates: Vec<AudioFingerprint> = (0..20)
        .map(|i| fp(&format!("/c{i}.wav"), 0.5 + (i as f64) * 0.01))
        .collect();
    let out = find_similar(&reference, &candidates, 3);
    assert_eq!(out.len(), 3);
    for w in out.windows(2) {
        assert!(w[0].1 <= w[1].1 + 1e-9);
    }
}

#[test]
fn find_similar_max_results_zero_is_empty() {
    let reference = fp("/r.wav", 0.1);
    let candidates = vec![fp("/a.wav", 0.2), fp("/b.wav", 0.3)];
    assert!(find_similar(&reference, &candidates, 0).is_empty());
}

#[test]
fn find_similar_empty_candidates_yields_empty() {
    let reference = fp("/r.wav", 0.5);
    assert!(find_similar(&reference, &[], 5).is_empty());
}

#[test]
fn find_similar_all_candidates_same_path_as_reference_yields_empty() {
    let reference = fp("/only.wav", 0.5);
    let candidates = vec![fp("/only.wav", 0.9)];
    assert!(find_similar(&reference, &candidates, 5).is_empty());
}

#[test]
fn fingerprint_distance_self_is_zero() {
    let a = fp("/same.wav", 0.42);
    let d = fingerprint_distance(&a, &a);
    assert!(d < 1e-9);
}

// ── `format_size` unit boundary just below each scale ───────────────────────────

#[test]
fn format_size_one_byte_below_megabyte_shows_kilobytes() {
    let b = 1024u64 * 1024 - 1;
    let s = app_lib::format_size(b);
    assert!(
        s.contains("KB"),
        "expected KB tier just under 1 MiB, got {s:?}"
    );
    assert!(!s.contains("MB"), "{s:?}");
}

#[test]
fn format_size_exact_power_of_terabyte_label() {
    let tb = 1024u64.pow(4);
    let s = app_lib::format_size(tb);
    assert_eq!(s, "1.0 TB");
}

#[test]
fn format_size_u64_max_does_not_panic() {
    let s = app_lib::format_size(u64::MAX);
    assert!(!s.is_empty());
    assert!(s.contains(' '));
}

// ── Scanner: JSON roundtrip preserves non-ASCII plugin metadata ─────────────────

#[test]
fn plugin_info_serde_roundtrip_unicode_name() {
    let p = PluginInfo {
        name: "音響プラグイン".into(),
        manufacturer: "メーカー".into(),
        version: "1.0.0".into(),
        plugin_type: "VST3".into(),
        path: "/Plugins/X.vst3".into(),
        size: "1.0 KB".into(),
        size_bytes: 1024,
        modified: "2025-06-01".into(),
        architectures: vec!["arm64".into()],
        manufacturer_url: Some("https://例.jp".into()),
    };
    let json = serde_json::to_string(&p).expect("serialize");
    let q: PluginInfo = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(p.name, q.name);
    assert_eq!(p.manufacturer, q.manufacturer);
    assert_eq!(p.manufacturer_url, q.manufacturer_url);
}

// ── Cheap failure modes (no heavy decode path) ──────────────────────────────────

#[test]
fn key_detect_unsupported_extension_returns_none() {
    assert!(app_lib::key_detect::detect_key("/tmp/not_audio.txt").is_none());
}

#[test]
fn bpm_estimate_missing_file_returns_none() {
    assert!(app_lib::bpm::estimate_bpm("/no/such/file_ah_test.wav").is_none());
}

#[test]
fn lufs_measure_missing_file_returns_none() {
    assert!(app_lib::lufs::measure_lufs("/no/such/file_ah_test.wav").is_none());
}

#[test]
fn midi_parse_short_file_returns_none() {
    let tmp = std::env::temp_dir().join(format!(
        "ah_contracts_bad_midi_{}.mid",
        std::process::id()
    ));
    std::fs::write(&tmp, b"xx").expect("write temp");
    let got = app_lib::midi::parse_midi(&tmp);
    let _ = std::fs::remove_file(&tmp);
    assert!(got.is_none());
}

// ── Xref: `normalize_plugin_name` bracket form + bare suffix (regressions for DAW strings) ─

#[test]
fn normalize_plugin_name_strips_bracket_vst3_suffix() {
    assert_eq!(
        normalize_plugin_name("ChannelStrip [VST3]"),
        "channelstrip"
    );
}

#[test]
fn normalize_plugin_name_strips_bare_x64_without_parens() {
    assert_eq!(normalize_plugin_name("AnalogSynth x64"), "analogsynth");
}

#[test]
fn normalize_plugin_name_triple_stacked_arch_suffixes() {
    assert_eq!(
        normalize_plugin_name("Tape  x64  (VST3)  (AU)"),
        "tape"
    );
}
