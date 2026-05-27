use app_lib::als_generator::{ClipPlacement, IdAllocatorPub, SampleInfo, generate_audio_clip_pub};

#[test]
fn test_audio_clip_xml_generation_basic() {
    let mut ids = IdAllocatorPub::new(100);
    let sample = SampleInfo {
        path: "/Samples/Kick.wav".into(),
        name: "Kick 01".into(),
        duration_secs: 1.0,
        sample_rate: 44100,
        file_size: 1024,
        bpm: Some(132.0),
    };
    let clip = ClipPlacement {
        sample,
        start_beat: 0.0,
        duration_beats: 4.0,
    };

    let xml = generate_audio_clip_pub(&clip, &mut ids);

    // Check key elements based on actual implementation
    assert!(xml.contains("<AudioClip"), "Should contain AudioClip tag");
    assert!(
        xml.contains("<Name Value=\"Kick 01\" />"),
        "Should contain correct clip name in Value attribute"
    );
    assert!(
        xml.contains("Id=\"100\""),
        "Should use the first allocated ID"
    );
}

#[test]
fn test_audio_clip_xml_escaping() {
    let mut ids = IdAllocatorPub::new(1000);
    let sample = SampleInfo {
        path: "/Samples/A & B < C > D.wav".into(),
        name: "Clip & Tag".into(),
        duration_secs: 2.0,
        sample_rate: 48000,
        file_size: 2048,
        bpm: None,
    };
    let clip = ClipPlacement {
        sample,
        start_beat: 16.0,
        duration_beats: 8.0,
    };

    let xml = generate_audio_clip_pub(&clip, &mut ids);

    // Escaped entities should be present in Value attributes
    assert!(
        xml.contains("<Name Value=\"Clip &amp; Tag\" />"),
        "Clip name should be escaped"
    );
    assert!(
        xml.contains("Value=\"/Samples/A &amp; B &lt; C &gt; D.wav\""),
        "Path should be escaped in the XML"
    );
}
