use app_lib::als_generator::{SampleInfo, TechnoConfig};

fn dummy_sample(name: &str) -> SampleInfo {
    SampleInfo {
        path: format!("/s/{}", name),
        name: name.into(),
        duration_secs: 1.0,
        sample_rate: 44100,
        file_size: 100,
        bpm: Some(130.0),
    }
}

#[test]
fn test_techno_arrangement_generation_structure() {
    let cfg = TechnoConfig {
        bpm: 130.0,
        kick: dummy_sample("kick"),
        clap: dummy_sample("clap"),
        hat: dummy_sample("hat"),
    };

    let tracks = cfg.generate_arrangement();

    assert!(tracks.len() >= 3);

    let track_names: Vec<_> = tracks.iter().map(|t| t.name.as_str()).collect();
    assert!(track_names.contains(&"Kick"));
    assert!(track_names.contains(&"Clap"));
    assert!(track_names.contains(&"Hat"));
}

#[test]
fn test_techno_arrangement_clip_offsets() {
    let cfg = TechnoConfig {
        bpm: 130.0,
        kick: dummy_sample("kick"),
        clap: dummy_sample("clap"),
        hat: dummy_sample("hat"),
    };

    let tracks = cfg.generate_arrangement();

    for track in tracks {
        if track.name == "Kick" {
            let has_drop_clip = track
                .clips
                .iter()
                .any(|c| c.start_beat >= 128.0 && c.start_beat < 256.0);
            assert!(has_drop_clip);
            let has_outro_clip = track.clips.iter().any(|c| c.start_beat >= 448.0);
            assert!(has_outro_clip);
        }
    }
}

#[test]
fn test_techno_arrangement_colors() {
    let cfg = TechnoConfig {
        bpm: 130.0,
        kick: dummy_sample("k"),
        clap: dummy_sample("c"),
        hat: dummy_sample("h"),
    };
    let tracks = cfg.generate_arrangement();
    for track in tracks {
        match track.name.as_str() {
            "Kick" => assert_eq!(track.color, 69),
            "Clap" => assert_eq!(track.color, 26),
            "Hat" => assert_eq!(track.color, 17),
            _ => {}
        }
    }
}
