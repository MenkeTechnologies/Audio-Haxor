use app_lib::midi::{MidiInfo, parse_midi};
use std::fs;
use std::path::Path;

#[test]
fn test_midi_info_struct_default() {
    let info = MidiInfo::default();
    assert_eq!(info.format, 0);
    assert_eq!(info.track_count, 0);
    assert_eq!(info.tempo, 0.0);
}

#[test]
fn test_midi_header_parsing_failures() {
    let tmp = std::env::temp_dir().join("fail.mid");

    // Too short
    fs::write(&tmp, b"MThd").unwrap();
    assert!(parse_midi(&tmp).is_none());

    // Wrong magic
    fs::write(&tmp, b"MTxt000000000000000000").unwrap();
    assert!(parse_midi(&tmp).is_none());

    let _ = fs::remove_file(&tmp);
}

#[test]
fn test_midi_info_serialization() {
    let info = MidiInfo {
        format: 1,
        track_count: 5,
        ppqn: 480,
        tempo: 128.0,
        time_signature: "4/4".into(),
        key_signature: "C Major".into(),
        note_count: 100,
        duration: 60.5,
        track_names: vec!["Lead".into(), "Bass".into()],
        channels_used: 1,
    };

    let j = serde_json::to_value(&info).unwrap();
    assert_eq!(j["format"], 1);
    assert_eq!(j["trackCount"], 5);
    assert_eq!(j["noteCount"], 100);
    assert_eq!(j["trackNames"][0], "Lead");
}

#[test]
fn test_midi_channel_mask_logic() {
    // Simulated logic verification
    let mut channel_mask = 0u16;
    channel_mask |= 1 << 0; // Channel 1
    channel_mask |= 1 << 9; // Channel 10

    let channels_used = channel_mask.count_ones() as u16;
    assert_eq!(channels_used, 2);
}

#[test]
fn test_midi_none_invalid_path() {
    assert!(parse_midi(Path::new("/tmp/nonexistent_midi_file_123")).is_none());
}
