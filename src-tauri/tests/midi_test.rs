#[test]
fn test_midi_parse_truncated_header_returns_none() {
    let temp = std::env::temp_dir().join("audio_haxor_midi_truncated.mid");
    // Too short for MThd + length
    std::fs::write(&temp, b"MThd").unwrap();
    assert!(app_lib::midi::parse_midi(&temp).is_none());
    let _ = std::fs::remove_file(&temp);
}

#[test]
fn test_midi_parse_basic_header() {
    let temp = std::env::temp_dir().join("audio_haxor_midi_basic.mid");
    std::fs::create_dir_all(temp.parent().unwrap()).ok();
    let midi_header = vec![
        0x4D, 0x54, 0x68, 0x64, // 'MThd'
        0, 0, 0, 6,            // header chunk length
        0, 1,                  // format 1
        0, 1,                  // 1 track
        0x01, 0xC0,            // 448 PPQN
    ];
    std::fs::write(&temp, &midi_header).unwrap();
    let result = app_lib::midi::parse_midi(&temp);
    assert!(
        result.is_some(),
        "minimal valid MThd should parse: {:?}",
        result
    );
    let info = result.unwrap();
    assert_eq!(info.format, 1);
    assert_eq!(info.track_count, 1);
    let _ = std::fs::remove_file(&temp);
}

#[test]
fn test_midi_info_default() {
    use app_lib::midi::MidiInfo;
    let d = MidiInfo::default();
    assert_eq!(d.format, 0);
    assert_eq!(d.track_count, 0);
    assert!(d.track_names.is_empty());
}
