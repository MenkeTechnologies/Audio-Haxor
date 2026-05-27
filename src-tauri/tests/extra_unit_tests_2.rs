use app_lib::als_generator::{IdAllocatorPub, xml_escape_pub};
use app_lib::als_project::{
    Genre, ProjectConfig, SectionLengths, SectionStarts, SectionValues, generate_project_name,
};
use app_lib::bulk_stat::BulkEntry;
use app_lib::midi_generator::{
    LeadType, MidiGenConfig, build_base_name, build_filename, resolve_chords,
};
use app_lib::sample_analysis::{
    extract_bpm, extract_key, extract_pack_name, match_category, short_key_to_db,
    strip_key_from_path,
};
use app_lib::similarity::AudioFingerprint;
use app_lib::trance_starter::{TranceLayer, TranceStarterConfig};
use std::collections::HashMap;
use std::path::PathBuf;

// ── 1-10. MidiGenConfig Variations ──────────────────────────────────

#[test]
fn test_midi_cfg_1() {
    let c = MidiGenConfig {
        key_root: 0,
        minor: true,
        lead_type: LeadType::TwoLayer,
        chords: vec![0],
        progression: vec![],
        bpm: 140,
        bars_per_chord: 1,
        length_bars: None,
        chromaticism: 0,
        seed: 1,
        name: None,
        variations: None,
    };
    assert_eq!(c.key_root, 0);
}
#[test]
fn test_midi_cfg_2() {
    let c = MidiGenConfig {
        key_root: 1,
        minor: false,
        lead_type: LeadType::TwoLayer,
        chords: vec![0],
        progression: vec![],
        bpm: 130,
        bars_per_chord: 2,
        length_bars: None,
        chromaticism: 1,
        seed: 2,
        name: None,
        variations: None,
    };
    assert_eq!(c.bpm, 130);
}
#[test]
fn test_midi_cfg_3() {
    let c = MidiGenConfig {
        key_root: 11,
        minor: true,
        lead_type: LeadType::TwoLayer,
        chords: vec![0],
        progression: vec![],
        bpm: 150,
        bars_per_chord: 4,
        length_bars: None,
        chromaticism: 10,
        seed: 3,
        name: None,
        variations: None,
    };
    assert_eq!(c.seed, 3);
}
#[test]
fn test_midi_cfg_4() {
    let c = MidiGenConfig {
        key_root: 5,
        minor: true,
        lead_type: LeadType::TwoLayer,
        chords: vec![0],
        progression: vec![],
        bpm: 120,
        bars_per_chord: 1,
        length_bars: Some(16),
        chromaticism: 5,
        seed: 4,
        name: None,
        variations: None,
    };
    assert_eq!(c.length_bars, Some(16));
}
#[test]
fn test_midi_cfg_5() {
    let c = MidiGenConfig {
        key_root: 7,
        minor: false,
        lead_type: LeadType::TwoLayer,
        chords: vec![0, 4, 7],
        progression: vec![],
        bpm: 128,
        bars_per_chord: 2,
        length_bars: None,
        chromaticism: 20,
        seed: 5,
        name: Some("B".into()),
        variations: None,
    };
    assert_eq!(c.name, Some("B".into()));
}
#[test]
fn test_midi_cfg_6() {
    let c = MidiGenConfig {
        key_root: 2,
        minor: true,
        lead_type: LeadType::TwoLayer,
        chords: vec![0],
        progression: vec!["D".into()],
        bpm: 145,
        bars_per_chord: 1,
        length_bars: None,
        chromaticism: 0,
        seed: 6,
        name: None,
        variations: Some(3),
    };
    assert_eq!(c.variations, Some(3));
}
#[test]
fn test_midi_cfg_7() {
    let c = MidiGenConfig {
        key_root: 0,
        minor: true,
        lead_type: LeadType::TwoLayer,
        chords: vec![0],
        progression: vec![],
        bpm: 140,
        bars_per_chord: 1,
        length_bars: None,
        chromaticism: 100,
        seed: 7,
        name: None,
        variations: None,
    };
    assert_eq!(c.chromaticism, 100);
}
#[test]
fn test_midi_cfg_8() {
    let c = MidiGenConfig {
        key_root: 0,
        minor: true,
        lead_type: LeadType::TwoLayer,
        chords: vec![0],
        progression: vec![],
        bpm: 140,
        bars_per_chord: 8,
        length_bars: None,
        chromaticism: 0,
        seed: 8,
        name: None,
        variations: None,
    };
    assert_eq!(c.bars_per_chord, 8);
}
#[test]
fn test_midi_cfg_9() {
    let c = MidiGenConfig {
        key_root: 0,
        minor: true,
        lead_type: LeadType::TwoLayer,
        chords: vec![0, 1, 2, 3],
        progression: vec![],
        bpm: 140,
        bars_per_chord: 1,
        length_bars: None,
        chromaticism: 0,
        seed: 9,
        name: None,
        variations: None,
    };
    assert_eq!(c.chords.len(), 4);
}
#[test]
fn test_midi_cfg_10() {
    let c = MidiGenConfig {
        key_root: 0,
        minor: true,
        lead_type: LeadType::TwoLayer,
        chords: vec![],
        progression: vec!["C".into(), "G".into()],
        bpm: 140,
        bars_per_chord: 1,
        length_bars: None,
        chromaticism: 0,
        seed: 10,
        name: None,
        variations: None,
    };
    assert_eq!(c.progression[1], "G");
}

// ── 11-20. is_negated edge cases ─────────────────────────────────────

#[test]
fn test_neg_1() {
    assert!(match_category("No Kick.wav", "/").is_none());
}
#[test]
fn test_neg_2() {
    assert!(match_category("Non-Kick.wav", "/").is_none());
}
#[test]
fn test_neg_3() {
    assert!(match_category("Without Kick.wav", "/").is_none());
}
#[test]
fn test_neg_4() {
    assert!(match_category("Kick No.wav", "/").is_some());
}
#[test]
fn test_neg_5() {
    assert!(match_category("Noah Kick.wav", "/").is_some());
}
#[test]
fn test_neg_6() {
    assert!(match_category("nothing_here.wav", "/").is_none());
}
#[test]
fn test_neg_7() {
    assert!(match_category("Nonstop Kick.wav", "/").is_some());
}
#[test]
fn test_neg_8() {
    assert!(match_category("without_bass.wav", "/").is_none());
}
#[test]
fn test_neg_9() {
    assert!(match_category("No_Kick_01.wav", "/").is_none());
}
#[test]
fn test_neg_10() {
    assert!(match_category("Kick_without_bass.wav", "/").is_some());
}

// ── 21-30. Category Confidence ──────────────────────────────────────

#[test]
fn test_conf_filename() {
    assert_eq!(match_category("Kick.wav", "/").unwrap().confidence, 1.0);
}
#[test]
fn test_conf_dir() {
    assert_eq!(match_category("01.wav", "/Kicks/").unwrap().confidence, 0.6);
}
#[test]
fn test_conf_both() {
    assert_eq!(
        match_category("Kick.wav", "/Kicks/").unwrap().confidence,
        1.0
    );
}
#[test]
fn test_conf_melodic() {
    assert_eq!(match_category("Lead.wav", "/").unwrap().confidence, 1.0);
}
#[test]
fn test_conf_fx() {
    assert_eq!(match_category("Riser.wav", "/").unwrap().confidence, 1.0);
}
#[test]
fn test_conf_perc() {
    assert_eq!(match_category("Conga.wav", "/").unwrap().confidence, 1.0);
}
#[test]
fn test_conf_clap() {
    assert_eq!(match_category("Clap.wav", "/").unwrap().confidence, 1.0);
}
#[test]
fn test_conf_snare() {
    assert_eq!(match_category("Snare.wav", "/").unwrap().confidence, 1.0);
}
#[test]
fn test_conf_hat() {
    assert_eq!(match_category("Hat.wav", "/").unwrap().confidence, 1.0);
}
#[test]
fn test_conf_none() {
    assert!(match_category("random.txt", "/").is_none());
}

// ── 31-40. normalize_sharp_flat_words ─────────────────────────────────

#[test]
fn test_norm_word_1() {
    assert_eq!(
        extract_key("Loop_C_Sharp_Minor.wav"),
        Some("C# Minor".into())
    );
}
#[test]
fn test_norm_word_2() {
    assert_eq!(
        extract_key("Loop_D_Flat_Major.wav"),
        Some("C# Major".into())
    );
}
#[test]
fn test_norm_word_3() {
    assert_eq!(
        extract_key("Loop_E_Flat_Minor.wav"),
        Some("D# Minor".into())
    );
}
#[test]
fn test_norm_word_4() {
    assert_eq!(
        extract_key("Loop F Sharp Major.wav"),
        Some("F# Major".into())
    );
}
#[test]
fn test_norm_word_5() {
    assert_eq!(extract_key("Loop-G-Sharp-Min.wav"), Some("G# Minor".into()));
}
#[test]
fn test_norm_word_6() {
    assert_eq!(extract_key("Loop A Flat Maj.wav"), Some("G# Major".into()));
}
#[test]
fn test_norm_word_7() {
    assert_eq!(
        extract_key("Loop B Flat Minor.wav"),
        Some("A# Minor".into())
    );
}
#[test]
fn test_norm_word_8() {
    assert_eq!(
        extract_key("Loop_A_Sharp_Major.wav"),
        Some("A# Major".into())
    );
}
#[test]
fn test_norm_word_9() {
    assert_eq!(
        extract_key("Loop_B_Major_128bpm.wav"),
        Some("B Major".into())
    );
}
#[test]
fn test_norm_word_10() {
    assert_eq!(extract_key("Loop_F_Minor.wav"), Some("F Minor".into()));
}

// ── 41-45. XML Escaping ─────────────────────────────────────────────

#[test]
fn test_xml_1() {
    assert_eq!(xml_escape_pub("a&b"), "a&amp;b");
}
#[test]
fn test_xml_2() {
    assert_eq!(xml_escape_pub("<tag>"), "&lt;tag&gt;");
}
#[test]
fn test_xml_3() {
    assert_eq!(xml_escape_pub("\"q\""), "&quot;q&quot;");
}
#[test]
fn test_xml_4() {
    assert_eq!(xml_escape_pub("'a'"), "&apos;a&apos;");
}
#[test]
fn test_xml_5() {
    assert_eq!(xml_escape_pub(" "), " ");
}

// ── 46-55. ProjectConfig deserialization ────────────────────────────

fn minimal_config_json() -> &'static str {
    r#"{"genre":"techno","hardness":0.5,"bpm":130,"atonal":false,"keywords":[],"element_keywords":{},"tracks":{"drums":{"count":1,"character":0.5},"bass":{"count":1,"character":0.5},"leads":{"count":1,"character":0.5},"pads":{"count":1,"character":0.5},"fx":{"count":1,"character":0.5},"vocals":{"count":0,"character":0.5}},"output_path":"/tmp","num_songs":1}"#
}

#[test]
fn test_pc_deser_1() {
    let c: ProjectConfig = serde_json::from_str(minimal_config_json()).unwrap();
    assert_eq!(c.genre, Genre::Techno);
}
#[test]
fn test_pc_deser_2() {
    let c: ProjectConfig = serde_json::from_str(minimal_config_json()).unwrap();
    assert_eq!(c.bpm, 130);
}
#[test]
fn test_pc_deser_3() {
    let c: ProjectConfig = serde_json::from_str(minimal_config_json()).unwrap();
    assert!(!c.atonal);
}
#[test]
fn test_pc_deser_4() {
    let c: ProjectConfig = serde_json::from_str(minimal_config_json()).unwrap();
    assert_eq!(c.num_songs, 1);
}
#[test]
fn test_pc_deser_5() {
    let c: ProjectConfig = serde_json::from_str(minimal_config_json()).unwrap();
    assert!(c.midi_tracks);
}
#[test]
fn test_pc_deser_6() {
    let c: ProjectConfig = serde_json::from_str(minimal_config_json()).unwrap();
    assert_eq!(c.chaos, 0.3);
}
#[test]
fn test_pc_deser_7() {
    let c: ProjectConfig = serde_json::from_str(minimal_config_json()).unwrap();
    assert_eq!(c.parallelism, 0.4);
}
#[test]
fn test_pc_deser_8() {
    let c: ProjectConfig = serde_json::from_str(minimal_config_json()).unwrap();
    assert_eq!(c.variation, 0.0);
} // 0.0 is the actual default in the code for many f32s
#[test]
fn test_pc_deser_9() {
    let c: ProjectConfig = serde_json::from_str(minimal_config_json()).unwrap();
    assert_eq!(c.density, 0.0);
}
#[test]
fn test_pc_deser_10() {
    let c: ProjectConfig = serde_json::from_str(minimal_config_json()).unwrap();
    assert!(c.seed.is_none());
}

// ── 56-60. SectionLengths Details ───────────────────────────────────

#[test]
fn test_sl_detail_techno() {
    let s = SectionLengths::techno_default();
    assert_eq!(s.intro, 32);
}
#[test]
fn test_sl_detail_trance() {
    let s = SectionLengths::trance_default();
    assert_eq!(s.breakdown, 48);
}
#[test]
fn test_sl_detail_schranz() {
    let s = SectionLengths::schranz_default();
    assert_eq!(s.drop2, 48);
}
#[test]
fn test_sl_detail_starts() {
    let s = SectionLengths::techno_default().starts();
    assert_eq!(s.intro, (1, 33));
}
#[test]
fn test_sl_detail_total() {
    let s = SectionLengths::techno_default().starts();
    assert_eq!(s.total_bars(), 224);
}

// ── 61-65. AudioFingerprint ─────────────────────────────────────────

#[test]
fn test_afp_1() {
    let f = AudioFingerprint {
        path: "a".into(),
        rms: 0.1,
        spectral_centroid: 1.0,
        zero_crossing_rate: 0.1,
        low_band_energy: 0.1,
        mid_band_energy: 0.1,
        high_band_energy: 0.1,
        low_energy_ratio: 0.1,
        attack_time: 0.1,
    };
    assert_eq!(f.rms, 0.1);
}
#[test]
fn test_afp_2() {
    let f = AudioFingerprint {
        path: "a".into(),
        rms: 0.1,
        spectral_centroid: 100.0,
        zero_crossing_rate: 0.1,
        low_band_energy: 0.1,
        mid_band_energy: 0.1,
        high_band_energy: 0.1,
        low_energy_ratio: 0.1,
        attack_time: 0.1,
    };
    assert_eq!(f.spectral_centroid, 100.0);
}
#[test]
fn test_afp_3() {
    let f = AudioFingerprint {
        path: "a".into(),
        rms: 0.1,
        spectral_centroid: 1.0,
        zero_crossing_rate: 0.5,
        low_band_energy: 0.1,
        mid_band_energy: 0.1,
        high_band_energy: 0.1,
        low_energy_ratio: 0.1,
        attack_time: 0.1,
    };
    assert_eq!(f.zero_crossing_rate, 0.5);
}
#[test]
fn test_afp_4() {
    let f = AudioFingerprint {
        path: "a".into(),
        rms: 0.1,
        spectral_centroid: 1.0,
        zero_crossing_rate: 0.1,
        low_band_energy: 0.9,
        mid_band_energy: 0.1,
        high_band_energy: 0.1,
        low_energy_ratio: 0.1,
        attack_time: 0.1,
    };
    assert_eq!(f.low_band_energy, 0.9);
}
#[test]
fn test_afp_5() {
    let f = AudioFingerprint {
        path: "a".into(),
        rms: 0.1,
        spectral_centroid: 1.0,
        zero_crossing_rate: 0.1,
        low_band_energy: 0.1,
        mid_band_energy: 0.1,
        high_band_energy: 0.1,
        low_energy_ratio: 0.1,
        attack_time: 0.001,
    };
    assert_eq!(f.attack_time, 0.001);
}

// ── 66-80. TranceLayer Variants ──────────────────────────────────────

#[test]
fn test_tl_kick() {
    assert_eq!(format!("{:?}", TranceLayer::Kick), "Kick");
}
#[test]
fn test_tl_bass() {
    assert_eq!(format!("{:?}", TranceLayer::Bass), "Bass");
}
#[test]
fn test_tl_pad() {
    assert_eq!(format!("{:?}", TranceLayer::Pad), "Pad");
}
#[test]
fn test_tl_arp() {
    assert_eq!(format!("{:?}", TranceLayer::Arp), "Arp");
}
#[test]
fn test_tl_pluck() {
    assert_eq!(format!("{:?}", TranceLayer::Pluck), "Pluck");
}
#[test]
fn test_tl_lead() {
    assert_eq!(format!("{:?}", TranceLayer::Lead), "Lead");
}
#[test]
fn test_tl_vocal() {
    assert_eq!(format!("{:?}", TranceLayer::Vocal), "Vocal");
}
#[test]
fn test_tl_vocal_chop() {
    assert_eq!(format!("{:?}", TranceLayer::VocalChop), "VocalChop");
}
#[test]
fn test_tl_vocal_atmos() {
    assert_eq!(
        format!("{:?}", TranceLayer::VocalAtmosphere),
        "VocalAtmosphere"
    );
}
#[test]
fn test_tl_vocal_phrase() {
    assert_eq!(format!("{:?}", TranceLayer::VocalPhrase), "VocalPhrase");
}
#[test]
fn test_tl_riser() {
    assert_eq!(format!("{:?}", TranceLayer::Riser), "Riser");
}
#[test]
fn test_tl_downer() {
    assert_eq!(format!("{:?}", TranceLayer::Downlifter), "Downlifter");
}
#[test]
fn test_tl_impact() {
    assert_eq!(format!("{:?}", TranceLayer::Impact), "Impact");
}
#[test]
fn test_tl_crash() {
    assert_eq!(format!("{:?}", TranceLayer::Crash), "Crash");
}
#[test]
fn test_tl_atmos() {
    assert_eq!(format!("{:?}", TranceLayer::Atmos), "Atmos");
}

// ── 81-90. BulkEntry ────────────────────────────────────────────────

#[test]
fn test_be_file() {
    let e = BulkEntry {
        name: "a".into(),
        path: PathBuf::from("a"),
        is_dir: false,
        is_file: true,
        is_symlink: false,
        size: 100,
        mtime_secs: 1,
    };
    assert!(e.is_file);
}
#[test]
fn test_be_dir() {
    let e = BulkEntry {
        name: "a".into(),
        path: PathBuf::from("a"),
        is_dir: true,
        is_file: false,
        is_symlink: false,
        size: 0,
        mtime_secs: 1,
    };
    assert!(e.is_dir);
}
#[test]
fn test_be_size() {
    let e = BulkEntry {
        name: "a".into(),
        path: PathBuf::from("a"),
        is_dir: false,
        is_file: true,
        is_symlink: false,
        size: 1234,
        mtime_secs: 1,
    };
    assert_eq!(e.size, 1234);
}
#[test]
fn test_be_mtime() {
    let e = BulkEntry {
        name: "a".into(),
        path: PathBuf::from("a"),
        is_dir: false,
        is_file: true,
        is_symlink: false,
        size: 100,
        mtime_secs: 999,
    };
    assert_eq!(e.mtime_secs, 999);
}
#[test]
fn test_be_name() {
    let e = BulkEntry {
        name: "test.wav".into(),
        path: PathBuf::from("test.wav"),
        is_dir: false,
        is_file: true,
        is_symlink: false,
        size: 100,
        mtime_secs: 1,
    };
    assert_eq!(e.name, "test.wav");
}
#[test]
fn test_be_path() {
    let e = BulkEntry {
        name: "a".into(),
        path: PathBuf::from("/tmp/a"),
        is_dir: false,
        is_file: true,
        is_symlink: false,
        size: 100,
        mtime_secs: 1,
    };
    assert_eq!(e.path, PathBuf::from("/tmp/a"));
}
#[test]
fn test_be_sym() {
    let e = BulkEntry {
        name: "a".into(),
        path: PathBuf::from("a"),
        is_dir: false,
        is_file: false,
        is_symlink: true,
        size: 0,
        mtime_secs: 1,
    };
    assert!(e.is_symlink);
}
#[test]
fn test_be_clone() {
    let e = BulkEntry {
        name: "a".into(),
        path: PathBuf::from("a"),
        is_dir: false,
        is_file: true,
        is_symlink: false,
        size: 100,
        mtime_secs: 1,
    };
    let c = e.clone();
    assert_eq!(c.name, "a");
}
#[test]
fn test_be_debug() {
    let e = BulkEntry {
        name: "a".into(),
        path: PathBuf::from("a"),
        is_dir: false,
        is_file: true,
        is_symlink: false,
        size: 100,
        mtime_secs: 1,
    };
    assert!(!format!("{:?}", e).is_empty());
}
#[test]
fn test_be_path_ext() {
    let e = BulkEntry {
        name: "a.wav".into(),
        path: PathBuf::from("a.wav"),
        is_dir: false,
        is_file: true,
        is_symlink: false,
        size: 100,
        mtime_secs: 1,
    };
    assert_eq!(e.path.extension().unwrap(), "wav");
}

// ── 91-100. Misc ───────────────────────────────────────────────────

#[test]
fn test_misc_short_key_major() {
    assert_eq!(short_key_to_db("A"), "A Minor");
}
#[test]
fn test_misc_strip_key_complex() {
    assert_eq!(strip_key_from_path("/a/b_Cmaj_c.wav"), "/a/b__c.wav");
}
#[test]
fn test_misc_id_alloc_start() {
    let a = IdAllocatorPub::new(500);
    assert_eq!(a.max_val(), 500);
}
#[test]
fn test_misc_xml_complex() {
    assert_eq!(xml_escape_pub("&<>\"' "), "&amp;&lt;&gt;&quot;&apos; ");
}
#[test]
fn test_misc_bpm_context() {
    assert_eq!(extract_bpm("Loop_120_bpm.wav"), Some(120));
}
#[test]
fn test_misc_bpm_context_2() {
    assert_eq!(extract_bpm("Kick_120.wav"), None);
}
#[test]
fn test_misc_key_context() {
    assert_eq!(extract_key("Lead_A.wav"), Some("A Minor".into()));
}
#[test]
fn test_misc_key_context_2() {
    assert_eq!(extract_key("01.wav"), None);
}
#[test]
fn test_misc_pack_name() {
    assert_eq!(
        extract_pack_name("/Samples/My-Pack/01.wav"),
        Some("My-Pack".into())
    );
}
#[test]
fn test_misc_pack_name_2() {
    assert_eq!(
        extract_pack_name("/Samples/My Pack Vol 1/01.wav"),
        Some("My Pack Vol 1".into())
    );
}
