//! MIDI: `MidiInfo` matches `app_lib::midi` and `parse_midi` smoke tests.

#[test]
fn test_midi_info_serializes_to_json() {
    use app_lib::midi::MidiInfo;
    let midi = MidiInfo {
        format: 1,
        track_count: 2,
        ppqn: 480,
        tempo: 120.0,
        time_signature: "4/4".into(),
        key_signature: "C".into(),
        note_count: 100,
        duration: 10.5,
        track_names: vec!["Drums".into(), "Bass".into()],
        channels_used: 4,
    };
    let v = serde_json::to_value(&midi).unwrap();
    assert_eq!(v["format"], 1);
    assert_eq!(v["trackCount"], 2);
    assert_eq!(v["ppqn"], 480);
    assert!(v["trackNames"].as_array().unwrap().len() == 2);
}

#[test]
fn test_midi_info_default() {
    let d = app_lib::midi::MidiInfo::default();
    assert_eq!(d.format, 0);
    assert_eq!(d.track_count, 0);
}

#[test]
fn test_parse_midi_minimal_header() {
    let temp = std::env::temp_dir().join("audio_haxor_midi_comp.mid");
    let midi_header = vec![
        0x4D, 0x54, 0x68, 0x64, 0, 0, 0, 6, 0, 1, 0, 1, 0x01, 0xC0,
    ];
    std::fs::write(&temp, &midi_header).unwrap();
    let r = app_lib::midi::parse_midi(&temp);
    assert!(r.is_some());
    let info = r.unwrap();
    assert_eq!(info.format, 1);
    assert_eq!(info.track_count, 1);
    let _ = std::fs::remove_file(&temp);
}

#[test]
fn test_midi_tempo_meta_to_bpm() {
    // Standard MIDI file tempo meta: microseconds per quarter note.
    let tempo_us: f64 = 500_000.0;
    let bpm = 60_000_000.0 / tempo_us;
    assert!((bpm - 120.0).abs() < 0.01);
}
