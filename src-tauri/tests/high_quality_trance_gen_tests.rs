use app_lib::als_project::{MidiSettings, SectionLengths};
use app_lib::trance_generator::generate_midi_tracks_for_arrangement;

#[test]
fn test_trance_midi_generation_determinism() {
    let sl = SectionLengths::trance_default();
    let seed = 12345;

    let tracks1 =
        generate_midi_tracks_for_arrangement(Some("A"), Some("Aeolian"), &None, seed, 138, &sl)
            .unwrap();

    let tracks2 =
        generate_midi_tracks_for_arrangement(Some("A"), Some("Aeolian"), &None, seed, 138, &sl)
            .unwrap();

    assert_eq!(tracks1.len(), tracks2.len());
    for i in 0..tracks1.len() {
        assert_eq!(tracks1[i].name, tracks2[i].name);
        assert_eq!(tracks1[i].clips.len(), tracks2[i].clips.len());
    }
}

#[test]
fn test_trance_midi_generation_layer_presence() {
    let sl = SectionLengths::trance_default();
    let tracks =
        generate_midi_tracks_for_arrangement(Some("G"), Some("Phrygian"), &None, 999, 140, &sl)
            .unwrap();

    let track_names: Vec<_> = tracks.iter().map(|t| t.name.as_str()).collect();

    // Core trance layers should be present
    assert!(track_names.contains(&"PAD"));
    assert!(track_names.contains(&"SUB BASS"));
    assert!(track_names.contains(&"LEAD"));
}

#[test]
fn test_trance_midi_generation_with_overrides() {
    let sl = SectionLengths::trance_default();
    let overrides = MidiSettings {
        progression: vec!["C".into(), "G".into(), "Am".into(), "F".into()],
        bars_per_chord: 2,
        chromaticism: 10,
        length_bars: Some(8),
    };

    let tracks = generate_midi_tracks_for_arrangement(
        Some("C"),
        Some("Ionian"),
        &Some(&overrides),
        42,
        128,
        &sl,
    )
    .unwrap();

    assert!(!tracks.is_empty());
    // Verify that the PAD track (always present) has clips
    let pad_track = tracks.iter().find(|t| t.name == "PAD").unwrap();
    assert!(!pad_track.clips.is_empty());
}

#[test]
fn test_trance_midi_generation_section_bounds() {
    let sl = SectionLengths {
        intro: 8,
        build: 8,
        breakdown: 8,
        drop1: 8,
        drop2: 8,
        fadedown: 8,
        outro: 8,
    };
    let tracks =
        generate_midi_tracks_for_arrangement(Some("E"), Some("Aeolian"), &None, 777, 144, &sl)
            .unwrap();

    // Total bars = 56.
    for track in tracks {
        for clip in track.clips {
            // Clip start + length must be within total project length
            assert!(clip.start_bar + clip.length_bars <= 56);
        }
    }
}
