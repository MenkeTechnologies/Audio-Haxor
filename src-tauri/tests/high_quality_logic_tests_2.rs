use app_lib::als_generator::{IdAllocatorPub, xml_escape_pub};
use app_lib::als_project::{Genre, ProjectConfig, SectionLengths, TrackConfig};
use app_lib::sample_analysis::{detect_manufacturer, match_category};

// ── Invariant: ProjectConfig JSON Schema Stability ───────────────────

#[test]
fn test_project_config_full_json_deserialization() {
    let json = r#"{
        "genre": "trance",
        "hardness": 0.5,
        "bpm": 140,
        "atonal": false,
        "root_note": "A#",
        "mode": "Aeolian",
        "keywords": ["trance", "acid"],
        "element_keywords": {"kick": "hard", "bass": "rolling"},
        "tracks": {
            "drums": {"count": 2, "character": 0.8},
            "bass": {"count": 1, "character": 0.7},
            "leads": {"count": 1, "character": 0.5},
            "pads": {"count": 1, "character": 0.5},
            "fx": {"count": 1, "character": 0.5},
            "vocals": {"count": 0, "character": 0.5}
        },
        "output_path": "/tmp/test",
        "num_songs": 1
    }"#;

    let c: ProjectConfig = serde_json::from_str(json).expect("should parse full json");
    assert_eq!(c.genre, Genre::Trance);
    assert_eq!(c.root_note, Some("A#".into()));
}

#[test]
fn test_project_config_minimal_json_deserialization() {
    let json = r#"{
        "genre": "techno",
        "hardness": 0.5,
        "bpm": 132,
        "atonal": true,
        "keywords": [],
        "element_keywords": {},
        "tracks": {
            "drums": {"count": 1, "character": 0.5},
            "bass": {"count": 1, "character": 0.5},
            "leads": {"count": 1, "character": 0.5},
            "pads": {"count": 1, "character": 0.5},
            "fx": {"count": 1, "character": 0.5},
            "vocals": {"count": 0, "character": 0.5}
        },
        "output_path": "/tmp/test",
        "num_songs": 1
    }"#;

    let c: ProjectConfig = serde_json::from_str(json).expect("should parse minimal json");
    assert_eq!(c.genre, Genre::Techno);
    assert!(c.atonal);
}

// ── Invariant: IdAllocator Monotonicity ──────────────────────────────

#[test]
fn test_id_allocator_monotonicity() {
    let mut alloc = IdAllocatorPub::new(1000);
    let mut last = 999;
    for _ in 0..100 {
        let current = alloc.next();
        assert!(current > last);
        last = current;
    }
    assert_eq!(alloc.max_val(), 1100);
}

// ── Robustness: XML Escape Character Coverage ────────────────────────

#[test]
fn test_xml_escape_all_entities() {
    assert_eq!(xml_escape_pub("&"), "&amp;");
    assert_eq!(xml_escape_pub("<"), "&lt;");
    assert_eq!(xml_escape_pub(">"), "&gt;");
    assert_eq!(xml_escape_pub("\""), "&quot;");
    assert_eq!(xml_escape_pub("'"), "&apos;");
}

// ── Property: Category Match Confidence Stability ────────────────────

#[test]
fn test_category_match_confidence_stability() {
    let m1 = match_category("Kick_01.wav", "/").unwrap();
    assert_eq!(m1.confidence, 1.0);

    let m2 = match_category("01.wav", "/Samples/Kicks/").unwrap();
    assert_eq!(m2.confidence, 0.6);
}

// ── Robustness: Manufacturer Signal Prioritization ───────────────────

#[test]
fn test_manufacturer_prioritization_logic() {
    let path = "/Samples/Producer Loops/Riemann Kollektion Techno";
    let m = detect_manufacturer(path).unwrap();
    assert_eq!(m.manufacturer_pattern, "Riemann Kollektion");
}

// ── Invariant: SectionLengths Summation ──────────────────────────────

#[test]
fn test_section_lengths_summation_invariant() {
    let sl = SectionLengths::techno_default();
    assert_eq!(sl.total_bars(), 224);
}

// ── Property: Path Normalization Identity ────────────────────────────

#[test]
fn test_path_norm_identity() {
    use app_lib::path_norm::normalize_path_for_db;
    let p = "/Users/wizard/Music/kick.wav";
    assert_eq!(normalize_path_for_db(p), p);
}

// ── Robustness: Short Key to DB Full Word Forms ──────────────────────

#[test]
fn test_short_key_to_db_comprehensive() {
    use app_lib::sample_analysis::short_key_to_db;
    assert_eq!(short_key_to_db("Am"), "A Minor");
    // "Gb" is not currently folded in short_key_to_db, but it is in extract_key.
    // We test the direct mapping here.
    assert_eq!(short_key_to_db("Gb"), "Gb Minor");
    assert_eq!(short_key_to_db("F#"), "F# Minor");
}

// ── Invariant: TrackConfig Defaults ──────────────────────────────────

#[test]
fn test_track_config_defaults() {
    let tc = TrackConfig::default();
    assert_eq!(tc.drums.count, 3);
}

// ── Invariant: SectionLengths Defaults and Sequencing ────────────────

#[test]
fn test_section_lengths_defaults() {
    let d = SectionLengths::default();
    assert_eq!(d.intro, 32);
    assert_eq!(d.total_bars(), 224);
}

#[test]
fn test_section_starts_ordering_invariants() {
    let sl = SectionLengths::techno_default();
    let s = sl.starts();
    assert_eq!(s.build.0, s.intro.1);
    assert_eq!(s.breakdown.0, s.build.1);
    assert_eq!(s.drop1.0, s.breakdown.1);
    assert_eq!(s.drop2.0, s.drop1.1);
    assert_eq!(s.fadedown.0, s.drop2.1);
    assert_eq!(s.outro.0, s.fadedown.1);
}

// ── Property: Audio Extension Filtering Logic ────────────────────────

#[test]
fn test_audio_extension_filtering_properties() {
    use app_lib::audio_extensions::is_audio_extension_lowercase;
    // The filter is case-sensitive and dot-exclusive by design
    assert!(is_audio_extension_lowercase("wav"));
    assert!(!is_audio_extension_lowercase(".wav"));
    assert!(!is_audio_extension_lowercase("WAV"));
}

// ── Invariant: MidiSettings Default Field Values ────────────────────

#[test]
fn test_midi_settings_default_values() {
    use app_lib::als_project::MidiSettings;
    let ms = MidiSettings::default();
    assert!(ms.progression.is_empty());
    assert_eq!(ms.bars_per_chord, 0); // derive(Default) for u8
}

// ── Property: XML Escape preserves alphanumeric ─────────────────────

#[test]
fn test_xml_escape_preservation() {
    let input = "AbletonLive12";
    assert_eq!(xml_escape_pub(input), input);
}
