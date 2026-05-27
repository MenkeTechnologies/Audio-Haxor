use app_lib::midi_generator::{
    LeadType, MidiGenConfig, build_base_name, build_filename, resolve_chords,
};

#[test]
fn test_chord_resolution_to_pitch_classes() {
    let mut config = MidiGenConfig {
        key_root: 0, // C
        minor: true,
        lead_type: LeadType::TwoLayer,
        chords: vec![],
        progression: vec!["C".into(), "Eb".into(), "G".into()],
        bpm: 140,
        bars_per_chord: 1,
        length_bars: None,
        chromaticism: 0,
        seed: 1,
        name: None,
        variations: None,
    };

    let resolved = resolve_chords(&config);
    assert_eq!(resolved, vec![0, 3, 7]);

    config.key_root = 2;
    config.progression = vec!["D".into(), "F".into(), "A".into()];
    let resolved_d = resolve_chords(&config);
    assert_eq!(resolved_d, vec![0, 3, 7]);
}

#[test]
fn test_filename_construction_invariants() {
    let config = MidiGenConfig {
        key_root: 9, // A
        minor: true,
        lead_type: LeadType::TwoLayer,
        chords: vec![0, 5], // A, D
        progression: vec![],
        bpm: 138,
        bars_per_chord: 4,
        length_bars: None,
        chromaticism: 0,
        seed: 42,
        name: Some("MyLead".into()),
        variations: Some(3),
    };

    let base = build_base_name(&config);
    assert!(
        base.contains("Am"),
        "Base name '{}' should contain key",
        base
    );
    assert!(base.contains("138bpm"), "Base name should contain BPM");
    assert!(
        base.contains("8bars"),
        "Base name should contain total bars"
    );
    assert!(base.contains("seed42"), "Base name should contain seed");

    let f1 = build_filename(&config, 0, 3);
    assert!(
        f1.ends_with("_01.mid"),
        "First variation should have _01 suffix"
    );
}

#[test]
fn test_chord_resolution_case_insensitivity() {
    let config = MidiGenConfig {
        key_root: 0,
        minor: true,
        lead_type: LeadType::TwoLayer,
        chords: vec![],
        progression: vec!["c".into(), "f".into()], // trim_end_matches is case sensitive
        bpm: 140,
        bars_per_chord: 1,
        length_bars: None,
        chromaticism: 0,
        seed: 1,
        name: None,
        variations: None,
    };

    let resolved = resolve_chords(&config);
    assert_eq!(resolved, vec![0, 5]);
}

#[test]
fn test_lead_type_base_names() {
    let mut config = MidiGenConfig {
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

    assert!(build_base_name(&config).contains("TwoLayer"));

    config.lead_type = LeadType::ChordArp;
    assert!(build_base_name(&config).contains("ChordArp"));

    config.lead_type = LeadType::DeepBass;
    assert!(build_base_name(&config).contains("DeepBass"));
}

#[test]
fn test_midi_gen_config_deserialization_full() {
    let json = r#"{
        "keyRoot": 2,
        "minor": false,
        "leadType": "chord_arp",
        "chords": [0, 4, 7],
        "progression": ["D", "G"],
        "bpm": 125,
        "barsPerChord": 2,
        "chromaticism": 25,
        "seed": 999
    }"#;

    let c: MidiGenConfig = serde_json::from_str(json).expect("valid json");
    assert_eq!(c.key_root, 2);
    assert_eq!(c.bpm, 125);
    assert_eq!(c.seed, 999);
    assert!(matches!(c.lead_type, LeadType::ChordArp));
}
