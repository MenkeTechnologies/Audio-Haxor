use app_lib::als_generator::{IdAllocatorPub, xml_escape_pub};
use app_lib::als_project::{
    Genre, ProjectConfig, SectionLengths, SectionStarts, SectionValues, generate_project_name,
    get_compatible_keys,
};
use app_lib::audio_extensions::{AUDIO_EXTENSIONS, is_audio_extension_lowercase};
use app_lib::audio_scanner::get_audio_metadata;
use app_lib::bpm::estimate_bpm;
use app_lib::content_hash::hash_file_sha256;
use app_lib::daw_scanner::{
    daw_name_for_format, ext_matches, is_package_ext, is_valid_pro_tools_session_file,
};
use app_lib::kvr::parse_version;
use app_lib::midi::parse_midi;
use app_lib::midi_generator::{
    LeadType, MidiGenConfig, build_base_name, build_filename, resolve_chords,
};
use app_lib::path_norm::normalize_path_for_db;
use app_lib::sample_analysis::{
    detect_manufacturer, extract_bpm, extract_key, extract_pack_name, match_category,
    short_key_to_db, strip_key_from_path,
};
use app_lib::sample_filters::{BAD_GENRES, is_ableton_project_sample, is_excluded_genre};
use app_lib::scanner::get_plugin_type;
use app_lib::scanner_skip_dirs::SCANNER_SKIP_DIRS;
use app_lib::similarity::{AudioFingerprint, fingerprint_distance};
use app_lib::track_generator::remap_bar_range;
use app_lib::trance_starter::TranceStarterConfig;
use app_lib::unified_walker::IncrementalDirState;
use app_lib::xref::normalize_plugin_name;
use app_lib::{format_size, path_needs_video_waveform_transcode};
use std::collections::HashMap;
use std::path::Path;

// ── 1-4. Genre Logic ──────────────────────────────────────────────────

#[test]
fn test_genre_bpm_techno() {
    assert_eq!(Genre::Techno.default_bpm(), 132);
}
#[test]
fn test_genre_bpm_schranz() {
    assert_eq!(Genre::Schranz.default_bpm(), 155);
}
#[test]
fn test_genre_bpm_trance() {
    assert_eq!(Genre::Trance.default_bpm(), 140);
}
#[test]
fn test_genre_range_techno() {
    assert_eq!(Genre::Techno.bpm_range(), (120, 140));
}

// ── 5-11. Section Lengths ─────────────────────────────────────────────

#[test]
fn test_sl_techno_total() {
    assert_eq!(SectionLengths::techno_default().total_bars(), 224);
}
#[test]
fn test_sl_trance_total() {
    assert_eq!(SectionLengths::trance_default().total_bars(), 256);
}
#[test]
fn test_sl_schranz_total() {
    assert_eq!(SectionLengths::schranz_default().total_bars(), 208);
}
#[test]
fn test_sl_sanitize_min() {
    let sl = SectionLengths {
        intro: 1,
        build: 1,
        breakdown: 1,
        drop1: 1,
        drop2: 1,
        fadedown: 1,
        outro: 1,
    };
    let s = sl.sanitize();
    assert_eq!(s.intro, 8);
    assert_eq!(s.outro, 8);
}
#[test]
fn test_sl_sanitize_snap() {
    let sl = SectionLengths {
        intro: 33,
        build: 32,
        breakdown: 32,
        drop1: 32,
        drop2: 32,
        fadedown: 32,
        outro: 32,
    };
    assert_eq!(sl.sanitize().intro, 32);
}
#[test]
fn test_sl_starts_1_indexed() {
    assert_eq!(SectionLengths::techno_default().starts().intro.0, 1);
}
#[test]
fn test_sl_starts_trance_breakdown() {
    assert_eq!(SectionLengths::trance_default().starts().breakdown.0, 65);
}

// ── 12-16. Section Values ─────────────────────────────────────────────

#[test]
fn test_sv_default_any() {
    assert!(!SectionValues::default().any());
}
#[test]
fn test_sv_set_any() {
    let mut v = SectionValues::default();
    v.set(1, 0.5);
    assert!(v.any());
}
#[test]
fn test_sv_value_at_bar_fallback() {
    assert_eq!(SectionValues::default().value_at_bar(1, 0.7), 0.7);
}
#[test]
fn test_sv_value_at_bar_pinned() {
    let mut v = SectionValues::default();
    v.set(1, 0.5);
    assert_eq!(v.value_at_bar(1, 0.7), 0.5);
    assert_eq!(v.value_at_bar(8, 0.7), 0.5);
    assert_eq!(v.value_at_bar(9, 0.7), 0.7);
}
#[test]
fn test_sv_clamping() {
    let mut v = SectionValues::default();
    v.set(1, 1.5);
    assert_eq!(v.value_at_bar(1, 0.0), 1.0);
    v.set(1, -0.5);
    assert_eq!(v.value_at_bar(1, 1.0), 0.0);
}

// ── 17-22. Path Normalization ────────────────────────────────────────

#[test]
fn test_path_norm_noop() {
    assert_eq!(normalize_path_for_db("/usr/bin/git"), "/usr/bin/git");
}
#[test]
fn test_path_norm_empty() {
    assert_eq!(normalize_path_for_db(""), "");
}
#[test]
fn test_path_norm_root() {
    assert_eq!(normalize_path_for_db("/"), "/");
}
#[test]
fn test_path_norm_firmlink() {
    #[cfg(target_os = "macos")]
    assert_eq!(
        normalize_path_for_db("/System/Volumes/Data/Users/x"),
        "/Users/x"
    );
}
#[test]
fn test_path_norm_firmlink_partial() {
    #[cfg(target_os = "macos")]
    assert_eq!(
        normalize_path_for_db("/System/Volumes/Data_Not/Users/x"),
        "/System/Volumes/Data_Not/Users/x"
    );
}
#[test]
fn test_path_norm_slashes() {
    assert_eq!(normalize_path_for_db("//a//b"), "//a//b");
}

// ── 23-27. Format Size ───────────────────────────────────────────────

#[test]
fn test_size_b() {
    assert_eq!(format_size(500), "500.0 B");
}
#[test]
fn test_size_kb() {
    assert_eq!(format_size(1024), "1.0 KB");
}
#[test]
fn test_size_mb() {
    assert_eq!(format_size(1024 * 1024), "1.0 MB");
}
#[test]
fn test_size_gb() {
    assert_eq!(format_size(1024 * 1024 * 1024), "1.0 GB");
}
#[test]
fn test_size_tb() {
    assert_eq!(format_size(1024 * 1024 * 1024 * 1024), "1.0 TB");
}

// ── 28-32. Audio Extensions ──────────────────────────────────────────

#[test]
fn test_ext_wav() {
    assert!(is_audio_extension_lowercase("wav"));
}
#[test]
fn test_ext_mp3() {
    assert!(is_audio_extension_lowercase("mp3"));
}
#[test]
fn test_ext_caps() {
    assert!(!is_audio_extension_lowercase("WAV"));
}
#[test]
fn test_ext_dot() {
    assert!(!is_audio_extension_lowercase(".wav"));
}
#[test]
fn test_ext_txt() {
    assert!(!is_audio_extension_lowercase("txt"));
}

// ── 33-37. Analysis Functions ────────────────────────────────────────

#[test]
fn test_bpm_none() {
    assert!(estimate_bpm("/tmp/nope.wav").is_none());
}
#[test]
fn test_midi_none() {
    assert!(parse_midi(Path::new("/tmp/nope.mid")).is_none());
}
#[test]
fn test_hash_none() {
    assert!(hash_file_sha256(Path::new("/tmp/nope.txt")).is_none());
}
#[test]
fn test_lufs_none() {
    assert!(app_lib::lufs::measure_lufs("/tmp/nope.wav").is_none());
}
#[test]
fn test_key_none() {
    assert!(app_lib::key_detect::detect_key("/tmp/nope.wav").is_none());
}

// ── 38-43. Key Compatibility ─────────────────────────────────────────

#[test]
fn test_keys_a_aeolian() {
    let keys = get_compatible_keys("A", "Aeolian");
    assert!(keys.contains(&"A Minor".into()));
    assert!(keys.contains(&"C Major".into()));
}
#[test]
fn test_keys_c_ionian() {
    let keys = get_compatible_keys("C", "Ionian");
    assert!(keys.contains(&"C Major".into()));
    assert!(keys.contains(&"A Minor".into()));
}
#[test]
fn test_keys_d_dorian() {
    assert!(get_compatible_keys("D", "Dorian").contains(&"C Major".into()));
}
#[test]
fn test_keys_fallback() {
    assert_eq!(get_compatible_keys("X", "Y"), vec!["X Minor".to_string()]);
}
#[test]
fn test_keys_g_sharp() {
    assert!(get_compatible_keys("G#", "Aeolian").contains(&"B Major".into()));
}
#[test]
fn test_keys_e_flat() {
    assert!(get_compatible_keys("D#", "Aeolian").contains(&"F# Major".into()));
}

// ── 44-50. Sample Analysis Strings ───────────────────────────────────

#[test]
fn test_short_key_am() {
    assert_eq!(short_key_to_db("Am"), "A Minor");
}
#[test]
fn test_short_key_c_sharp() {
    assert_eq!(short_key_to_db("C#"), "C# Minor");
}
#[test]
fn test_strip_key_am() {
    assert_eq!(strip_key_from_path("Kick_Am_128.wav"), "Kick__128.wav");
}
#[test]
fn test_strip_key_c_sharp_maj() {
    assert_eq!(strip_key_from_path("Lead_C#Maj_130.wav"), "Lead__130.wav");
}
#[test]
fn test_strip_key_g_sharp_word() {
    assert_eq!(
        strip_key_from_path("Bass G Sharp Minor 140.wav"),
        "Bass  140.wav"
    );
}
#[test]
fn test_extract_bpm_loop() {
    assert_eq!(extract_bpm("Loop_128_bpm.wav"), Some(128));
}
#[test]
fn test_extract_bpm_range() {
    assert_eq!(extract_bpm("Loop_220_bpm.wav"), None);
}

// ── 51-55. Manufacturer detection ────────────────────────────────────

#[test]
fn test_manuf_armada() {
    assert!(
        detect_manufacturer("/Labels/Armada/Trance")
            .unwrap()
            .genre_score
            > 0.0
    );
}
#[test]
fn test_manuf_drumcode() {
    assert!(
        detect_manufacturer("/Packs/Drumcode/Techno")
            .unwrap()
            .genre_score
            < 0.0
    );
}
#[test]
fn test_manuf_freshly() {
    assert!(
        detect_manufacturer("/Freshly Squeezed/Lax")
            .unwrap()
            .genre_score
            > 0.5
    );
}
#[test]
fn test_manuf_riemann() {
    assert!(
        detect_manufacturer("/Riemann/Kollektion")
            .unwrap()
            .genre_score
            < -0.5
    );
}
#[test]
fn test_manuf_neutral() {
    assert_eq!(
        detect_manufacturer("/Cymatics/Kicks").unwrap().genre_score,
        0.0
    );
}

// ── 56-60. Category Matching ─────────────────────────────────────────

#[test]
fn test_cat_schranz_kick() {
    assert_eq!(
        match_category("hard techno kick.wav", "/").unwrap().name,
        "schranz_kick"
    );
}
#[test]
fn test_cat_supersaw() {
    assert_eq!(
        match_category("Trance Lead Supersaw.wav", "/")
            .unwrap()
            .name,
        "supersaw"
    );
}
#[test]
fn test_cat_acid() {
    assert_eq!(
        match_category("Acid Bass 303.wav", "/").unwrap().name,
        "acid_bass"
    );
}
#[test]
fn test_cat_gate() {
    assert_eq!(
        match_category("Gated Trance Pad.wav", "/").unwrap().name,
        "pad"
    );
}
#[test]
fn test_cat_riser() {
    assert_eq!(
        match_category("Epic Riser.wav", "/").unwrap().name,
        "fx_riser"
    );
}

// ── 61-65. Sample Filters ────────────────────────────────────────────

#[test]
fn test_filter_ableton_sample() {
    assert!(is_ableton_project_sample(
        "/Project/Samples/Imported/kick.wav"
    ));
}
#[test]
fn test_filter_not_ableton() {
    assert!(!is_ableton_project_sample("/Samples/Kicks/kick.wav"));
}
#[test]
fn test_filter_excluded_genre() {
    assert!(is_excluded_genre("/Samba/loops", BAD_GENRES));
}
#[test]
fn test_filter_override_genre() {
    assert!(!is_excluded_genre("/Afro/Techno", BAD_GENRES));
}
#[test]
fn test_filter_unix_path() {
    assert!(is_ableton_project_sample(
        "/Samples/Project/Samples/Imported/x.wav"
    ));
}

// ── 66-70. MIDI Generator ────────────────────────────────────────────

#[test]
fn test_midi_resolve_c_maj() {
    let c = MidiGenConfig {
        key_root: 0,
        minor: false,
        lead_type: LeadType::TwoLayer,
        chords: vec![0, 5, 7],
        progression: vec![],
        bpm: 140,
        bars_per_chord: 2,
        length_bars: None,
        chromaticism: 0,
        seed: 1,
        name: None,
        variations: None,
    };
    assert_eq!(resolve_chords(&c), vec![0, 5, 7]);
}
#[test]
fn test_midi_resolve_prog() {
    let c = MidiGenConfig {
        key_root: 0,
        minor: false,
        lead_type: LeadType::TwoLayer,
        chords: vec![],
        progression: vec!["C".into(), "F".into()],
        bpm: 140,
        bars_per_chord: 2,
        length_bars: None,
        chromaticism: 0,
        seed: 1,
        name: None,
        variations: None,
    };
    assert_eq!(resolve_chords(&c), vec![0, 5]);
}
#[test]
fn test_midi_base_name() {
    let c = MidiGenConfig {
        key_root: 9,
        minor: true,
        lead_type: LeadType::TwoLayer,
        chords: vec![0, 3],
        progression: vec![],
        bpm: 138,
        bars_per_chord: 4,
        length_bars: None,
        chromaticism: 0,
        seed: 123,
        name: None,
        variations: None,
    };
    assert_eq!(build_base_name(&c), "Am_TwoLayer_8bars_138bpm_seed123");
}
#[test]
fn test_midi_filename() {
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
    assert_eq!(
        build_filename(&c, 0, 1),
        "Cm_TwoLayer_1bars_140bpm_seed1.mid"
    );
}
#[test]
fn test_midi_filename_padded() {
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
    assert_eq!(
        build_filename(&c, 1, 5),
        "Cm_TwoLayer_1bars_140bpm_seed1_02.mid"
    );
}

// ── 71-75. Scanner Logic ─────────────────────────────────────────────

#[test]
fn test_plugin_vst3() {
    assert_eq!(get_plugin_type(".vst3"), "VST3");
}
#[test]
fn test_plugin_comp() {
    assert_eq!(get_plugin_type(".component"), "AU");
}
#[test]
fn test_plugin_clap() {
    assert_eq!(get_plugin_type(".clap"), "CLAP");
}
#[test]
fn test_daw_name_als() {
    assert_eq!(daw_name_for_format("ALS"), "Ableton Live");
}
#[test]
fn test_daw_ext_logicx() {
    assert_eq!(ext_matches(Path::new("p.logicx")), Some("LOGICX".into()));
}

// ── 76-80. Similarity ────────────────────────────────────────────────

#[test]
fn test_sim_dist_zero() {
    let f = AudioFingerprint {
        path: "".into(),
        rms: 0.5,
        spectral_centroid: 1.0,
        zero_crossing_rate: 0.1,
        low_band_energy: 0.1,
        mid_band_energy: 0.1,
        high_band_energy: 0.1,
        low_energy_ratio: 0.1,
        attack_time: 0.1,
    };
    assert_eq!(fingerprint_distance(&f, &f), 0.0);
}
#[test]
fn test_sim_dist_pos() {
    let f1 = AudioFingerprint {
        path: "".into(),
        rms: 0.5,
        spectral_centroid: 1.0,
        zero_crossing_rate: 0.1,
        low_band_energy: 0.1,
        mid_band_energy: 0.1,
        high_band_energy: 0.1,
        low_energy_ratio: 0.1,
        attack_time: 0.1,
    };
    let mut f2 = f1.clone();
    f2.rms = 0.6;
    assert!(fingerprint_distance(&f1, &f2) > 0.0);
}
#[test]
fn test_sim_dist_larger() {
    let f1 = AudioFingerprint {
        path: "".into(),
        rms: 0.5,
        spectral_centroid: 1.0,
        zero_crossing_rate: 0.1,
        low_band_energy: 0.1,
        mid_band_energy: 0.1,
        high_band_energy: 0.1,
        low_energy_ratio: 0.1,
        attack_time: 0.1,
    };
    let mut f2 = f1.clone();
    f2.rms = 0.6;
    let mut f3 = f1.clone();
    f3.rms = 0.9;
    assert!(fingerprint_distance(&f1, &f3) > fingerprint_distance(&f1, &f2));
}
#[test]
fn test_sim_centroid_impact() {
    let f1 = AudioFingerprint {
        path: "".into(),
        rms: 0.5,
        spectral_centroid: 1000.0,
        zero_crossing_rate: 0.1,
        low_band_energy: 0.1,
        mid_band_energy: 0.1,
        high_band_energy: 0.1,
        low_energy_ratio: 0.1,
        attack_time: 0.1,
    };
    let mut f2 = f1.clone();
    f2.spectral_centroid = 2000.0;
    assert!(fingerprint_distance(&f1, &f2) > 0.0);
}
#[test]
fn test_sim_attack_impact() {
    let f1 = AudioFingerprint {
        path: "".into(),
        rms: 0.5,
        spectral_centroid: 1.0,
        zero_crossing_rate: 0.1,
        low_band_energy: 0.1,
        mid_band_energy: 0.1,
        high_band_energy: 0.1,
        low_energy_ratio: 0.1,
        attack_time: 0.01,
    };
    let mut f2 = f1.clone();
    f2.attack_time = 0.1;
    assert!(fingerprint_distance(&f1, &f2) > 0.0);
}

// ── 81-85. Misc Utils ────────────────────────────────────────────────

#[test]
fn test_xml_esc_amp() {
    assert_eq!(xml_escape_pub("&"), "&amp;");
}
#[test]
fn test_xml_esc_tags() {
    assert_eq!(xml_escape_pub("<>"), "&lt;&gt;");
}
#[test]
fn test_id_alloc_inc() {
    let mut a = IdAllocatorPub::new(10);
    assert_eq!(a.next(), 10);
    assert_eq!(a.next(), 11);
}
#[test]
fn test_id_alloc_max() {
    let mut a = IdAllocatorPub::new(10);
    a.next();
    assert_eq!(a.max_val(), 11);
}
#[test]
fn test_skip_git() {
    assert!(SCANNER_SKIP_DIRS.contains(&".git"));
}

// ── 86-90. ALS Project Name ─────────────────────────────────────────

#[test]
fn test_proj_name_not_empty() {
    let json = r#"{"genre":"techno","hardness":0.5,"bpm":130,"atonal":false,"keywords":[],"element_keywords":{},"tracks":{"drums":{"count":1,"character":0.5},"bass":{"count":1,"character":0.5},"leads":{"count":1,"character":0.5},"pads":{"count":1,"character":0.5},"fx":{"count":1,"character":0.5},"vocals":{"count":0,"character":0.5}},"output_path":"/tmp","num_songs":1}"#;
    let cfg: ProjectConfig = serde_json::from_str(json).unwrap();
    assert!(!generate_project_name(&cfg, 1).is_empty());
}
#[test]
fn test_proj_name_diff_seeds() {
    let json = r#"{"genre":"techno","hardness":0.5,"bpm":130,"atonal":false,"keywords":[],"element_keywords":{},"tracks":{"drums":{"count":1,"character":0.5},"bass":{"count":1,"character":0.5},"leads":{"count":1,"character":0.5},"pads":{"count":1,"character":0.5},"fx":{"count":1,"character":0.5},"vocals":{"count":0,"character":0.5}},"output_path":"/tmp","num_songs":1}"#;
    let cfg: ProjectConfig = serde_json::from_str(json).unwrap();
    assert_ne!(
        generate_project_name(&cfg, 1),
        generate_project_name(&cfg, 2)
    );
}
#[test]
fn test_proj_name_genre_impact() {
    let json_t = r#"{"genre":"techno","hardness":0.5,"bpm":130,"atonal":false,"keywords":[],"element_keywords":{},"tracks":{"drums":{"count":1,"character":0.5},"bass":{"count":1,"character":0.5},"leads":{"count":1,"character":0.5},"pads":{"count":1,"character":0.5},"fx":{"count":1,"character":0.5},"vocals":{"count":0,"character":0.5}},"output_path":"/tmp","num_songs":1}"#;
    let cfg_t: ProjectConfig = serde_json::from_str(json_t).unwrap();
    assert!(!generate_project_name(&cfg_t, 1).is_empty());
}
#[test]
fn test_proj_name_hardness_impact() {
    let json_h = r#"{"genre":"techno","hardness":0.9,"bpm":130,"atonal":false,"keywords":[],"element_keywords":{},"tracks":{"drums":{"count":1,"character":0.5},"bass":{"count":1,"character":0.5},"leads":{"count":1,"character":0.5},"pads":{"count":1,"character":0.5},"fx":{"count":1,"character":0.5},"vocals":{"count":0,"character":0.5}},"output_path":"/tmp","num_songs":1}"#;
    let cfg_h: ProjectConfig = serde_json::from_str(json_h).unwrap();
    assert!(!generate_project_name(&cfg_h, 1).is_empty());
}
#[test]
fn test_proj_name_atonal() {
    let json = r#"{"genre":"techno","hardness":0.5,"bpm":130,"atonal":true,"keywords":[],"element_keywords":{},"tracks":{"drums":{"count":1,"character":0.5},"bass":{"count":1,"character":0.5},"leads":{"count":1,"character":0.5},"pads":{"count":1,"character":0.5},"fx":{"count":1,"character":0.5},"vocals":{"count":0,"character":0.5}},"output_path":"/tmp","num_songs":1}"#;
    let cfg: ProjectConfig = serde_json::from_str(json).unwrap();
    assert!(!generate_project_name(&cfg, 1).is_empty());
}

// ── 91-95. Configuration Logic ──────────────────────────────────────

#[test]
fn test_starter_cfg_deser() {
    let json = r#"{"keyRoot":0,"minor":true,"perLayer":5}"#;
    let cfg: TranceStarterConfig = serde_json::from_str(json).unwrap();
    assert_eq!(cfg.per_layer, 5);
}
#[test]
fn test_starter_root_0_val() {
    let cfg = TranceStarterConfig {
        key_root: 0,
        minor: true,
        per_layer: 10,
        midi_config: None,
    };
    assert_eq!(cfg.key_root, 0);
}
#[test]
fn test_starter_root_11_val() {
    let cfg = TranceStarterConfig {
        key_root: 11,
        minor: false,
        per_layer: 10,
        midi_config: None,
    };
    assert_eq!(cfg.key_root, 11);
}
#[test]
fn test_genre_to_lowercase() {
    let g: Genre = serde_json::from_str("\"trance\"").unwrap();
    assert_eq!(g, Genre::Trance);
}
#[test]
fn test_genre_to_string() {
    let g = Genre::Schranz;
    assert_eq!(serde_json::to_string(&g).unwrap(), "\"schranz\"");
}

// ── 96-100. Robustness ───────────────────────────────────────────────

#[test]
fn test_remap_range_oob() {
    assert!(remap_bar_range(1000.0, 1004.0, &SectionLengths::techno_default().starts()).is_none());
}
#[test]
fn test_kvr_ver_strip_patch() {
    assert_eq!(parse_version("1.2.3.4"), vec![1, 2, 3, 4]);
}
#[test]
fn test_pack_extract_deep() {
    assert_eq!(
        extract_pack_name("/a/b/My-Big-Pack/c/d/e"),
        Some("My-Big-Pack".into())
    );
}
#[test]
fn test_ext_matches_ptx() {
    assert_eq!(ext_matches(Path::new("s.ptx")), Some("PTX".into()));
}
#[test]
fn test_is_excluded_genre_pos() {
    assert!(is_excluded_genre("Pop/Hits", BAD_GENRES));
}
