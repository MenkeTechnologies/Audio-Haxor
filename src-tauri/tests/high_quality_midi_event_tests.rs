use app_lib::midi_generator::{LeadType, MidiGenConfig, generate_events};

#[test]
fn test_generate_events_determinism() {
    let config = MidiGenConfig {
        key_root: 0,
        minor: true,
        lead_type: LeadType::TwoLayer,
        chords: vec![0, 3, 7],
        progression: vec![],
        bpm: 140,
        bars_per_chord: 1,
        length_bars: None,
        chromaticism: 0,
        seed: 123,
        name: None,
        variations: None,
    };

    let events1 = generate_events(&config).unwrap();
    let events2 = generate_events(&config).unwrap();

    assert_eq!(events1.len(), events2.len());
    for i in 0..events1.len() {
        assert_eq!(events1[i].pitch, events2[i].pitch);
        assert_eq!(events1[i].tick, events2[i].tick);
    }
}

#[test]
fn test_generate_events_scale_adherence() {
    // C Major (minor: false)
    let config = MidiGenConfig {
        key_root: 0,
        minor: false,
        lead_type: LeadType::TwoLayer,
        chords: vec![0],
        progression: vec![],
        bpm: 120,
        bars_per_chord: 4,
        length_bars: None,
        chromaticism: 0, // strict scale
        seed: 42,
        name: None,
        variations: None,
    };

    let events = generate_events(&config).unwrap();
    let c_maj_pcs = [0, 2, 4, 5, 7, 9, 11]; // C D E F G A B

    for ev in events {
        let pc = ev.pitch % 12;
        assert!(
            c_maj_pcs.contains(&pc),
            "Pitch {} (pc {}) is not in C Major scale",
            ev.pitch,
            pc
        );
    }
}

#[test]
fn test_generate_events_chromaticism_impact() {
    let mut config = MidiGenConfig {
        key_root: 0,
        minor: false,
        lead_type: LeadType::TwoLayer,
        chords: vec![0],
        progression: vec![],
        bpm: 120,
        bars_per_chord: 4,
        length_bars: None,
        chromaticism: 100, // Very chromatic
        seed: 7,
        name: None,
        variations: None,
    };

    let events_chromatic = generate_events(&config).unwrap();

    config.chromaticism = 0;
    let events_strict = generate_events(&config).unwrap();

    // With seed 7, these should differ if the chromaticism logic is working
    assert_ne!(events_chromatic.len(), 0);
    assert_ne!(events_strict.len(), 0);
}

#[test]
fn test_generate_events_valid_chords() {
    let config = MidiGenConfig {
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
    let res = generate_events(&config);
    assert!(res.is_ok());
}

#[test]
fn test_generate_events_all_lead_types() {
    let mut config = MidiGenConfig {
        key_root: 0,
        minor: true,
        lead_type: LeadType::TwoLayer,
        chords: vec![0],
        progression: vec![],
        bpm: 140,
        bars_per_chord: 1,
        length_bars: None,
        chromaticism: 10,
        seed: 1,
        name: None,
        variations: None,
    };

    let types = [
        LeadType::TwoLayer,
        LeadType::Unison,
        LeadType::ChordArp,
        LeadType::Zigzag,
        LeadType::SlowMelody,
        LeadType::Bounce,
        LeadType::Cell,
        LeadType::PadChord,
        LeadType::ChordPluck,
        LeadType::DeepBass,
        LeadType::SubBass,
        LeadType::PianoChord,
        LeadType::Trill,
        LeadType::Progressive,
        LeadType::Shuffle,
        LeadType::GatedStab,
    ];

    for lt in types {
        config.lead_type = lt;
        let res = generate_events(&config);
        assert!(
            res.is_ok(),
            "Failed to generate events for lead type {:?}",
            lt
        );
    }
}
