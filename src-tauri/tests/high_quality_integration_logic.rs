use app_lib::als_project::{Genre, ProjectConfig, generate_project_name, get_compatible_keys};

// ── Integrated Test: Key Sensitivity ──────────────────────────────────

#[test]
fn test_category_key_sensitivity_integration() {
    let tonal = [
        "sub_bass",
        "mid_bass",
        "lead",
        "pad",
        "arp",
        "pluck",
        "stab",
        "acid",
        "atmos",
        "vocal",
        "vocal_phrase",
    ];
    let atonal = [
        "kick",
        "clap",
        "snare",
        "hat",
        "perc",
        "fx_riser",
        "fx_impact",
        "schranz_drive",
    ];

    use app_lib::sample_analysis::match_category;

    for cat in tonal {
        let m = match_category("test.wav", &format!("/Samples/{}/", cat)).unwrap();
        assert!(m.is_key_sensitive, "{} must be key sensitive", cat);
    }

    for cat in atonal {
        let m = match_category("test.wav", &format!("/Samples/{}/", cat)).unwrap();
        assert!(!m.is_key_sensitive, "{} must NOT be key sensitive", cat);
    }
}

// ── Integrated Test: Harmonic Target Expansion ────────────────────────

#[test]
fn test_harmonic_expansion_trance_aeolian() {
    let targets = get_compatible_keys("A", "Aeolian");
    assert!(targets.contains(&"A Minor".to_string()));
    assert!(targets.contains(&"C Major".to_string()));
}

// ── Integrated Test: ProjectConfig Validation ─────────────────────────

#[test]
fn test_complex_project_config_mapping() {
    let json = r#"{
        "genre": "schranz",
        "hardness": 0.95,
        "bpm": 160,
        "atonal": false,
        "root_note": "F#",
        "mode": "Phrygian",
        "keywords": ["hard", "industrial"],
        "element_keywords": {"kick": "pounding"},
        "type_atonal": {"kick": true},
        "tracks": {
            "drums": {"count": 4, "character": 0.9},
            "bass": {"count": 2, "character": 0.8},
            "leads": {"count": 1, "character": 0.5},
            "pads": {"count": 1, "character": 0.5},
            "fx": {"count": 1, "character": 0.5},
            "vocals": {"count": 0, "character": 0.5}
        },
        "output_path": "/tmp/schranz",
        "num_songs": 3
    }"#;

    let cfg: ProjectConfig = serde_json::from_str(json).expect("valid complex config");
    assert_eq!(cfg.genre, Genre::Schranz);
    assert_eq!(cfg.bpm, 160);
    assert_eq!(cfg.root_note, Some("F#".into()));
    assert_eq!(cfg.keywords.len(), 2);
    assert_eq!(cfg.element_keywords.get("kick").unwrap(), "pounding");
}

// ── Integrated Test: Filename + Directory Classification ──────────────

#[test]
fn test_deep_path_classification() {
    use app_lib::sample_analysis::match_category;

    let m = match_category("Snare_01.wav", "/Samples/Kicks").unwrap();
    assert_eq!(m.name, "snare");

    let m2 = match_category("01.wav", "/Samples/Hard_Techno_Kicks").unwrap();
    assert_eq!(m2.name, "schranz_kick");
}

// ── Integrated Test: Deterministic Project Naming ─────────────────────

#[test]
fn test_deterministic_project_naming() {
    let json = r#"{
        "genre": "techno",
        "hardness": 0.5,
        "bpm": 132,
        "atonal": false,
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
        "output_path": "/tmp",
        "num_songs": 1
    }"#;
    let cfg: ProjectConfig = serde_json::from_str(json).unwrap();

    let name1 = generate_project_name(&cfg, 12345);
    let name2 = generate_project_name(&cfg, 12345);
    assert_eq!(
        name1, name2,
        "Naming must be deterministic with identical seeds"
    );
}
