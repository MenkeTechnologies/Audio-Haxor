use app_lib::als_generator::{
    IdAllocatorPub, MidiClipPlacement, MidiTrackInfo, generate_midi_track,
};
use app_lib::midi_generator::NoteEvent;

#[test]
fn test_midi_track_xml_generation_basic() {
    let mut ids = IdAllocatorPub::new(500);

    let event = NoteEvent {
        pitch: 60, // C3
        vel: 100,
        tick: 0,
        dur: 96,
    };

    let clip = MidiClipPlacement {
        events: vec![event],
        start_bar: 0, // 0 bars offset = beat 0
        length_bars: 4,
        name: "Test MIDI".into(),
        color: 3,
    };

    let info = MidiTrackInfo {
        name: "Piano".into(),
        color: 12,
        clips: vec![clip],
    };

    let xml = generate_midi_track("", &info, &mut ids);

    assert!(xml.contains("EffectiveName Value=\"Piano\""));
    assert!(xml.contains("<MidiClip"));
    assert!(xml.contains("<Name Value=\"Test MIDI\" />"));

    // Check note data
    assert!(
        xml.contains("<MidiKey Value=\"60\" />"),
        "Should contain the correct MIDI note number in MidiKey tag"
    );
}

#[test]
fn test_midi_track_xml_timing_conversion() {
    let mut ids = IdAllocatorPub::new(1000);

    let event = NoteEvent {
        pitch: 72,
        vel: 127,
        tick: 192,
        dur: 48,
    };

    let clip = MidiClipPlacement {
        events: vec![event],
        start_bar: 1, // 1 bar offset = 4 beats offset
        length_bars: 1,
        name: "Beat 3 Note".into(),
        color: 1,
    };

    let info = MidiTrackInfo {
        name: "Synth".into(),
        color: 1,
        clips: vec![clip],
    };

    let xml = generate_midi_track("", &info, &mut ids);

    // 1 bar * 4 beats/bar = 4 beats.
    assert!(
        xml.contains("Time=\"4\""),
        "Clip starting at bar 1 should be at beat 4"
    );

    // Note time 2.0 relative to clip start.
    assert!(
        xml.contains("Time=\"2\""),
        "Note at tick 192 should be at relative time 2.0"
    );
    assert!(
        xml.contains("Duration=\"0.5\""),
        "48 ticks should be 0.5 beats"
    );
}

#[test]
fn test_midi_track_xml_escaping() {
    let mut ids = IdAllocatorPub::new(1);
    let clip = MidiClipPlacement {
        events: vec![],
        start_bar: 0,
        length_bars: 4,
        name: "Midi & Logic < 1".into(),
        color: 5,
    };

    let info = MidiTrackInfo {
        name: "A & B".into(),
        color: 10,
        clips: vec![clip],
    };

    let xml = generate_midi_track("", &info, &mut ids);

    assert!(xml.contains("Value=\"A &amp; B\""));
    assert!(xml.contains("Value=\"Midi &amp; Logic &lt; 1\""));
}
