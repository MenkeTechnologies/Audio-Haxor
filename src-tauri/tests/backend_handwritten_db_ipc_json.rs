//! JSON serialization contracts for `db` row/result types (Tauri → WebView IPC) and related
//! DTOs. Guards `#[serde(rename)]` field names and `skip_serializing_if` behavior.

use std::collections::HashMap;

use app_lib::audio_scanner::AudioMetadata;
use app_lib::db::{
    AudioQueryResult, AudioSampleRow, AudioStatsResult, CacheStat, DawQueryResult, DawRow,
    DawStatsResult, FilterStatsResult, PdfQueryResult, PdfRow, PdfStatsResult, PluginQueryResult,
    PluginRow, PresetQueryResult, PresetRow, PresetStatsResult, ScanInfo,
};
use app_lib::file_watcher::FileWatcherState;
use app_lib::history::{KvrCacheEntry, KvrCacheUpdateEntry};
use app_lib::midi::MidiInfo;

fn as_obj(v: &serde_json::Value) -> &serde_json::Map<String, serde_json::Value> {
    v.as_object().expect("JSON object")
}

// ── Plugin list (`query_plugins` shape) ───────────────────────────────────────────

#[test]
fn plugin_row_serializes_type_and_size_bytes_camel_case() {
    let row = PluginRow {
        name: "X".into(),
        path: "/p/X.vst3".into(),
        plugin_type: "VST3".into(),
        version: "1".into(),
        manufacturer: "M".into(),
        manufacturer_url: Some("https://m.example".into()),
        size: "1.0 KB".into(),
        size_bytes: 1024,
        modified: "t".into(),
        architectures: vec!["arm64".into()],
    };
    let v = serde_json::to_value(&row).unwrap();
    let o = as_obj(&v);
    assert_eq!(o.get("type"), Some(&serde_json::json!("VST3")));
    assert_eq!(o.get("sizeBytes"), Some(&serde_json::json!(1024)));
    assert_eq!(
        o.get("manufacturerUrl"),
        Some(&serde_json::json!("https://m.example"))
    );
}

#[test]
fn plugin_row_omits_none_manufacturer_url() {
    let row = PluginRow {
        name: "Y".into(),
        path: "/p/Y.vst3".into(),
        plugin_type: "VST3".into(),
        version: "1".into(),
        manufacturer: "M".into(),
        manufacturer_url: None,
        size: "0 B".into(),
        size_bytes: 0,
        modified: "t".into(),
        architectures: vec![],
    };
    let v = serde_json::to_value(&row).unwrap();
    assert!(as_obj(&v).get("manufacturerUrl").is_none());
}

#[test]
fn plugin_query_result_has_total_count_keys() {
    let q = PluginQueryResult {
        plugins: vec![],
        total_count: 12,
        total_count_capped: false,
        total_unfiltered: 100,
    };
    let v = serde_json::to_value(&q).unwrap();
    let o = as_obj(&v);
    assert_eq!(o.get("totalCount"), Some(&serde_json::json!(12)));
    assert_eq!(o.get("totalUnfiltered"), Some(&serde_json::json!(100)));
}

// ── Audio samples (`query_audio` shape) ─────────────────────────────────────────

#[test]
fn audio_sample_row_serializes_analysis_fields_when_some() {
    let row = AudioSampleRow {
        name: "a".into(),
        path: "/a.wav".into(),
        directory: "/".into(),
        format: "WAV".into(),
        size: 100,
        size_formatted: "100.0 B".into(),
        modified: "t".into(),
        duration: Some(1.5),
        channels: Some(2),
        sample_rate: Some(44100),
        bits_per_sample: Some(16),
        bpm: Some(120.0),
        key: Some("C Major".into()),
        lufs: Some(-14.2),
    };
    let v = serde_json::to_value(&row).unwrap();
    let o = as_obj(&v);
    assert_eq!(o.get("sampleRate"), Some(&serde_json::json!(44100)));
    assert_eq!(o.get("bitsPerSample"), Some(&serde_json::json!(16)));
    assert_eq!(o.get("bpm"), Some(&serde_json::json!(120.0)));
    assert_eq!(o.get("key"), Some(&serde_json::json!("C Major")));
    assert_eq!(o.get("lufs"), Some(&serde_json::json!(-14.2)));
}

#[test]
fn audio_sample_row_omits_none_optional_analysis_fields() {
    let row = AudioSampleRow {
        name: "a".into(),
        path: "/a.wav".into(),
        directory: "/".into(),
        format: "WAV".into(),
        size: 10,
        size_formatted: "10.0 B".into(),
        modified: "t".into(),
        duration: None,
        channels: None,
        sample_rate: None,
        bits_per_sample: None,
        bpm: None,
        key: None,
        lufs: None,
    };
    let v = serde_json::to_value(&row).unwrap();
    let o = as_obj(&v);
    assert!(o.get("bpm").is_none());
    assert!(o.get("sampleRate").is_none());
    assert!(o.get("lufs").is_none());
}

#[test]
fn audio_query_result_shape() {
    let q = AudioQueryResult {
        samples: vec![],
        total_count: 3,
        total_count_capped: false,
        total_unfiltered: 50,
    };
    let v = serde_json::to_value(&q).unwrap();
    let o = as_obj(&v);
    assert!(o.get("samples").is_some());
    assert_eq!(o.get("totalCount"), Some(&serde_json::json!(3)));
}

#[test]
fn audio_stats_result_shape() {
    let mut fc = HashMap::new();
    fc.insert("WAV".into(), 5u64);
    let s = AudioStatsResult {
        sample_count: 5,
        total_bytes: 1000,
        format_counts: fc,
        analyzed_count: 2,
    };
    let v = serde_json::to_value(&s).unwrap();
    let o = as_obj(&v);
    assert_eq!(o.get("sampleCount"), Some(&serde_json::json!(5)));
    assert_eq!(
        o.get("formatCounts").unwrap().get("WAV"),
        Some(&serde_json::json!(5))
    );
    assert_eq!(o.get("analyzedCount"), Some(&serde_json::json!(2)));
}

// ── DAW / preset / PDF rows ─────────────────────────────────────────────────────

#[test]
fn daw_row_and_query_result_shape() {
    let row = DawRow {
        name: "p".into(),
        path: "/p.als".into(),
        directory: "/d".into(),
        format: "ALS".into(),
        daw: "Ableton Live".into(),
        size: 99,
        size_formatted: "99.0 B".into(),
        modified: "t".into(),
    };
    let v = serde_json::to_value(&row).unwrap();
    assert_eq!(
        as_obj(&v).get("sizeFormatted"),
        Some(&serde_json::json!("99.0 B"))
    );

    let q = DawQueryResult {
        projects: vec![row],
        total_count: 1,
        total_count_capped: false,
        total_unfiltered: 10,
    };
    let vq = serde_json::to_value(&q).unwrap();
    assert!(as_obj(&vq).get("projects").unwrap().is_array());
}

#[test]
fn preset_row_and_query_result_shape() {
    let row = PresetRow {
        name: "z".into(),
        path: "/z.fxp".into(),
        directory: "/".into(),
        format: "FXP".into(),
        size: 1,
        size_formatted: "1.0 B".into(),
        modified: "t".into(),
    };
    let q = PresetQueryResult {
        presets: vec![row],
        total_count: 1,
        total_count_capped: false,
        total_unfiltered: 2,
    };
    let v = serde_json::to_value(&q).unwrap();
    let o = as_obj(&v);
    assert_eq!(o.get("totalCount"), Some(&serde_json::json!(1)));
}

#[test]
fn pdf_row_query_and_stats_shape() {
    let row = PdfRow {
        name: "r".into(),
        path: "/r.pdf".into(),
        directory: "/".into(),
        size: 400,
        size_formatted: "400.0 B".into(),
        modified: "t".into(),
    };
    let q = PdfQueryResult {
        pdfs: vec![row],
        total_count: 1,
        total_count_capped: false,
        total_unfiltered: 9,
    };
    let v = serde_json::to_value(&q).unwrap();
    assert!(as_obj(&v).get("pdfs").unwrap().is_array());

    let ps = PdfStatsResult {
        pdf_count: 3,
        total_bytes: 900,
    };
    let vs = serde_json::to_value(&ps).unwrap();
    let o = as_obj(&vs);
    assert_eq!(o.get("pdfCount"), Some(&serde_json::json!(3)));
    assert_eq!(o.get("totalBytes"), Some(&serde_json::json!(900)));
}

// ── Aggregates & scan info ──────────────────────────────────────────────────────

#[test]
fn daw_stats_preset_stats_scan_info_shapes() {
    let mut dc = HashMap::new();
    dc.insert("ALS".into(), 2u64);
    let ds = DawStatsResult {
        project_count: 2,
        total_bytes: 100,
        daw_counts: dc,
    };
    let v = serde_json::to_value(&ds).unwrap();
    assert!(as_obj(&v).get("dawCounts").is_some());

    let mut pfc = HashMap::new();
    pfc.insert("FXP".into(), 4u64);
    let ps = PresetStatsResult {
        preset_count: 4,
        total_bytes: 200,
        format_counts: pfc,
    };
    let v = serde_json::to_value(&ps).unwrap();
    assert_eq!(as_obj(&v).get("presetCount"), Some(&serde_json::json!(4)));

    let mut sfc = HashMap::new();
    sfc.insert("WAV".into(), 1u64);
    let si = ScanInfo {
        id: "id".into(),
        timestamp: "ts".into(),
        sample_count: 1,
        total_bytes: 10,
        format_counts: sfc,
        roots: vec!["/r".into()],
    };
    let v = serde_json::to_value(&si).unwrap();
    assert_eq!(
        as_obj(&v).get("roots").unwrap().as_array().unwrap().len(),
        1
    );
}

#[test]
fn cache_stat_serializes_size_bytes() {
    let c = CacheStat {
        key: "k".into(),
        label: "L".into(),
        count: 10,
        total: 10,
        size_bytes: 4096,
    };
    let v = serde_json::to_value(&c).unwrap();
    assert_eq!(as_obj(&v).get("sizeBytes"), Some(&serde_json::json!(4096)));
}

#[test]
fn filter_stats_result_shape() {
    let mut bt = HashMap::new();
    bt.insert("WAV".into(), 3u64);
    let mut bbt = HashMap::new();
    bbt.insert("WAV".into(), 999u64);
    let f = FilterStatsResult {
        count: 3,
        count_capped: false,
        total_bytes: 999,
        by_type: bt,
        bytes_by_type: bbt,
        total_unfiltered: 100,
        size_buckets: vec![1, 2, 0, 0, 0, 0],
    };
    let v = serde_json::to_value(&f).unwrap();
    let o = as_obj(&v);
    assert!(o.get("byType").is_some());
    assert!(o.get("bytesByType").is_some());
    assert_eq!(o.get("totalUnfiltered"), Some(&serde_json::json!(100)));
    assert_eq!(
        o.get("sizeBuckets"),
        Some(&serde_json::json!([1, 2, 0, 0, 0, 0]))
    );
}

// ── `FileWatcherState` (initial, no Tauri app) ──────────────────────────────────

#[test]
fn file_watcher_state_new_not_watching_empty_dirs() {
    let s = FileWatcherState::new();
    assert!(!app_lib::file_watcher::is_watching(&s));
    assert!(app_lib::file_watcher::get_watched_dirs(&s).is_empty());
}

#[test]
fn file_watcher_state_default_matches_new() {
    let a = FileWatcherState::new();
    let b = FileWatcherState::default();
    assert_eq!(
        app_lib::file_watcher::is_watching(&a),
        app_lib::file_watcher::is_watching(&b)
    );
}

// ── KVR cache DTOs (`history`) ──────────────────────────────────────────────────

#[test]
fn kvr_cache_entry_json_roundtrip() {
    let e = KvrCacheEntry {
        kvr_url: Some("https://kvraudio.com/p/x".into()),
        update_url: Some("https://get.example/dl".into()),
        latest_version: Some("2.0".into()),
        has_update: true,
        source: "kvr".into(),
        timestamp: "2026-01-01".into(),
    };
    let json = serde_json::to_string(&e).unwrap();
    let back: KvrCacheEntry = serde_json::from_str(&json).unwrap();
    assert_eq!(back.latest_version, e.latest_version);
    assert!(back.has_update);
}

#[test]
fn kvr_cache_update_entry_json_roundtrip() {
    let u = KvrCacheUpdateEntry {
        key: "plugin:foo".into(),
        kvr_url: Some("https://kvr/x".into()),
        update_url: None,
        latest_version: Some("1.2".into()),
        has_update: Some(true),
        source: Some("resolver".into()),
    };
    let json = serde_json::to_string(&u).unwrap();
    let back: KvrCacheUpdateEntry = serde_json::from_str(&json).unwrap();
    assert_eq!(back.key, "plugin:foo");
    assert_eq!(back.has_update, Some(true));
}

// ── MIDI info (player / metadata panel) ─────────────────────────────────────────

#[test]
fn midi_info_serializes_rename_fields() {
    let mut m = MidiInfo::default();
    m.format = 1;
    m.track_count = 2;
    m.ppqn = 480;
    m.tempo = 120.0;
    m.time_signature = "4/4".into();
    m.key_signature = "C".into();
    m.note_count = 10;
    m.duration = 5.5;
    m.track_names = vec!["Drums".into()];
    m.channels_used = 4;

    let v = serde_json::to_value(&m).unwrap();
    let o = as_obj(&v);
    assert!(o.get("trackCount").is_some());
    assert!(o.get("timeSignature").is_some());
    assert!(o.get("keySignature").is_some());
    assert!(o.get("noteCount").is_some());
    assert!(o.get("trackNames").is_some());
    assert!(o.get("channelsUsed").is_some());
}

// ── `AudioMetadata` (audio scanner → UI) ───────────────────────────────────────

#[test]
fn audio_metadata_serializes_camel_case_paths_and_size_bytes() {
    let m = AudioMetadata {
        full_path: "/Music/kick.wav".into(),
        file_name: "kick.wav".into(),
        directory: "/Music".into(),
        format: "WAV".into(),
        size_bytes: 2048,
        created: "c".into(),
        modified: "m".into(),
        accessed: "a".into(),
        permissions: "0644".into(),
        channels: Some(2),
        sample_rate: Some(48000),
        bits_per_sample: Some(24),
        duration: Some(2.25),
        error: None,
    };
    let v = serde_json::to_value(&m).unwrap();
    let o = as_obj(&v);
    assert_eq!(
        o.get("fullPath"),
        Some(&serde_json::json!("/Music/kick.wav"))
    );
    assert_eq!(o.get("fileName"), Some(&serde_json::json!("kick.wav")));
    assert_eq!(o.get("sizeBytes"), Some(&serde_json::json!(2048)));
    assert_eq!(o.get("sampleRate"), Some(&serde_json::json!(48000)));
}

#[test]
fn audio_metadata_error_field_serializes_when_present() {
    let m = AudioMetadata {
        full_path: "/x.wav".into(),
        file_name: "x.wav".into(),
        directory: "/".into(),
        format: "".into(),
        size_bytes: 0,
        created: "".into(),
        modified: "".into(),
        accessed: "".into(),
        permissions: "".into(),
        channels: None,
        sample_rate: None,
        bits_per_sample: None,
        duration: None,
        error: Some("read failed".into()),
    };
    let v = serde_json::to_value(&m).unwrap();
    assert_eq!(
        as_obj(&v).get("error"),
        Some(&serde_json::json!("read failed"))
    );
}

#[test]
fn plugin_query_result_with_row_serializes_nested_plugins() {
    let row = PluginRow {
        name: "P".into(),
        path: "/P.vst3".into(),
        plugin_type: "VST3".into(),
        version: "1".into(),
        manufacturer: "M".into(),
        manufacturer_url: None,
        size: "1 B".into(),
        size_bytes: 1,
        modified: "t".into(),
        architectures: vec![],
    };
    let q = PluginQueryResult {
        plugins: vec![row],
        total_count: 1,
        total_count_capped: false,
        total_unfiltered: 1,
    };
    let v = serde_json::to_value(&q).unwrap();
    let plugins = as_obj(&v).get("plugins").unwrap().as_array().unwrap();
    assert_eq!(plugins.len(), 1);
    assert_eq!(plugins[0].get("path"), Some(&serde_json::json!("/P.vst3")));
}

#[test]
fn pdf_stats_result_zero_counts() {
    let p = PdfStatsResult {
        pdf_count: 0,
        total_bytes: 0,
    };
    let v = serde_json::to_value(&p).unwrap();
    let o = as_obj(&v);
    assert_eq!(o.get("pdfCount"), Some(&serde_json::json!(0)));
}

#[test]
fn filter_stats_empty_breakdown_maps() {
    let f = FilterStatsResult {
        count: 0,
        count_capped: false,
        total_bytes: 0,
        by_type: HashMap::new(),
        bytes_by_type: HashMap::new(),
        total_unfiltered: 0,
        size_buckets: vec![],
    };
    let v = serde_json::to_value(&f).unwrap();
    let o = as_obj(&v);
    assert_eq!(o.get("byType").unwrap().as_object().unwrap().len(), 0);
    assert_eq!(o.get("bytesByType").unwrap().as_object().unwrap().len(), 0);
}

#[test]
fn audio_stats_empty_format_counts() {
    let s = AudioStatsResult {
        sample_count: 0,
        total_bytes: 0,
        format_counts: HashMap::new(),
        analyzed_count: 0,
    };
    let v = serde_json::to_value(&s).unwrap();
    assert_eq!(
        as_obj(&v)
            .get("formatCounts")
            .unwrap()
            .as_object()
            .unwrap()
            .len(),
        0
    );
}

#[test]
fn plugin_row_unicode_paths_roundtrip_json() {
    let row = PluginRow {
        name: "音響".into(),
        path: "/音響/プラグイン.vst3".into(),
        plugin_type: "VST3".into(),
        version: "1".into(),
        manufacturer: "メーカー".into(),
        manufacturer_url: None,
        size: "0 B".into(),
        size_bytes: 0,
        modified: "t".into(),
        architectures: vec![],
    };
    let json = serde_json::to_string(&row).unwrap();
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(
        v.get("path"),
        Some(&serde_json::json!("/音響/プラグイン.vst3"))
    );
}
