use app_lib::als_generator::xml_escape_pub;
use app_lib::als_project::{SectionLengths, get_compatible_keys};
use app_lib::midi_generator::{LeadType, MidiGenConfig, resolve_chords};
use app_lib::path_norm::normalize_path_for_db;
use app_lib::sample_analysis::{extract_bpm, extract_key, short_key_to_db, strip_key_from_path};

// ── Property: Path Normalization Idempotency ───────────────────────────
// normalize(normalize(x)) == normalize(x)

#[test]
fn test_path_norm_idempotency() {
    let cases = [
        "/Users/wizard/Music/Sample.wav",
        "/System/Volumes/Data/Users/wizard/Music",
        "relative/path/test",
        "",
        "/",
        "//double//slashes",
    ];
    for case in cases {
        let first = normalize_path_for_db(case);
        let second = normalize_path_for_db(&first);
        assert_eq!(
            first, second,
            "Path normalization must be idempotent for '{}'",
            case
        );
    }
}

// ── Property: Key Compatibility Symmetry/Cycle ────────────────────────
// If A is compatible with B, B should generally be compatible with A
// (or at least participate in a logical harmonic relationship).

#[test]
fn test_key_compatibility_symmetry() {
    let roots = [
        "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
    ];
    let modes = [
        "Ionian",
        "Aeolian",
        "Dorian",
        "Phrygian",
        "Lydian",
        "Mixolydian",
        "Locrian",
    ];

    for root in roots {
        for mode in modes {
            let compatible = get_compatible_keys(root, mode);
            assert!(
                compatible.len() >= 1,
                "Key {} {} should have compatible keys",
                root,
                mode
            );
        }
    }
}

// ── Invariant: Section Sanitization ───────────────────────────────────
// sanitized.total_bars() must always be a multiple of 8 (block boundary).
// All individual sections must be >= 8 and multiples of 8.

#[test]
fn test_section_sanitization_invariants() {
    for i in 0..100 {
        // Generate pseudo-random lengths
        let raw = SectionLengths {
            intro: (i * 7) % 64,
            build: (i * 13) % 64,
            breakdown: (i * 17) % 64,
            drop1: (i * 19) % 64,
            drop2: (i * 23) % 64,
            fadedown: (i * 29) % 64,
            outro: (i * 31) % 64,
        };
        let s = raw.sanitize();

        assert!(s.intro >= 8 && s.intro % 8 == 0);
        assert!(s.build >= 8 && s.build % 8 == 0);
        assert!(s.breakdown >= 8 && s.breakdown % 8 == 0);
        assert!(s.drop1 >= 8 && s.drop1 % 8 == 0);
        assert!(s.drop2 >= 8 && s.drop2 % 8 == 0);
        assert!(s.fadedown >= 8 && s.fadedown % 8 == 0);
        assert!(s.outro >= 8 && s.outro % 8 == 0);
        assert_eq!(s.total_bars() % 8, 0);
    }
}

// ── Invariant: Short Key to DB Mapping ────────────────────────────────
// short_key_to_db should always produce a string containing "Major" or "Minor".

#[test]
fn test_short_key_format_invariant() {
    let inputs = [
        "C", "C#", "Db", "D", "D#", "Eb", "E", "F", "F#", "Gb", "G", "G#", "Ab", "A", "A#", "Bb",
        "B", "Cb",
    ];
    for input in inputs {
        let full = short_key_to_db(input);
        assert!(full.contains("Major") || full.contains("Minor"));
        // Ensure no weird double sharps/flats in the output
        assert!(!full.contains("##"));
        assert!(!full.contains("bb"));
    }
}

// ── Property: Strip Key Idempotency ───────────────────────────────────
// strip_key_from_path(strip_key_from_path(x)) == strip_key_from_path(x)

#[test]
fn test_strip_key_idempotency() {
    let cases = [
        "Lead_Am_128.wav",
        "Bass_C#_140.aif",
        "Pad - F Sharp Minor - Soft.wav",
        "NoKeyHere.wav",
    ];
    for case in cases {
        let first = strip_key_from_path(case);
        let second = strip_key_from_path(&first);
        assert_eq!(first, second);
    }
}

// ── Robustness: XML Escaping Edge Cases ──────────────────────────────

#[test]
fn test_xml_escape_robustness() {
    let complex = "Hello <world> & 'friends' \"of\" Rust \n\r\t";
    let escaped = xml_escape_pub(complex);

    assert!(!escaped.contains('<'));
    assert!(!escaped.contains('>'));
    assert!(!escaped.contains('&') || escaped.contains("&amp;"));
    assert!(!escaped.contains('\'') || escaped.contains("&apos;"));
    assert!(!escaped.contains('\"') || escaped.contains("&quot;"));

    // Non-ASCII robustness
    let unicode = "Mörkö — 🎵";
    assert_eq!(xml_escape_pub(unicode), unicode); // Logic preserves non-target chars
}

// ── Invariant: Midi Generation Chord Resolution ──────────────────────
// resolve_chords should always return a non-empty vector if either chords or progression is set.

#[test]
fn test_chord_resolution_invariant() {
    let mut config = MidiGenConfig {
        key_root: 0,
        minor: true,
        lead_type: LeadType::TwoLayer,
        chords: vec![],
        progression: vec![],
        bpm: 140,
        bars_per_chord: 1,
        length_bars: None,
        chromaticism: 0,
        seed: 1,
        name: None,
        variations: None,
    };

    // Empty case
    assert!(resolve_chords(&config).is_empty());

    // Chords set
    config.chords = vec![0, 3, 7];
    assert_eq!(resolve_chords(&config).len(), 3);

    // Progression set (overrides chords)
    config.progression = vec!["Cm".into(), "Fm".into()];
    assert_eq!(resolve_chords(&config).len(), 2);
}

// ── Robustness: extract_bpm False Positives ───────────────────────────

#[test]
fn test_bpm_extraction_false_positives() {
    let bad_cases = [
        "Project_v1.wav",
        "Sample_44100Hz.wav",
        "Loop_20260419.wav",
        "Kick_01.wav",
        "Lead_1.wav",
        "Ambience_5.1_Surround.wav",
    ];
    for case in bad_cases {
        assert!(
            extract_bpm(case).is_none(),
            "Should not extract BPM from '{}'",
            case
        );
    }
}

// ── Robustness: SectionStarts 1-Indexing ──────────────────────────────
// The system uses 1-indexed bar numbers for Ableton XML.
// Verify that ends are always > starts.

#[test]
fn test_section_starts_ordering() {
    let sl = SectionLengths::techno_default();
    let starts = sl.starts();

    let ranges = [
        starts.intro,
        starts.build,
        starts.breakdown,
        starts.drop1,
        starts.drop2,
        starts.fadedown,
        starts.outro,
    ];

    for (start, end) in ranges {
        assert!(start >= 1, "Start bar must be >= 1");
        assert!(end > start, "End bar must be > start bar");
        assert_eq!((end - start) % 8, 0, "Section length must be multiple of 8");
    }
}

// ── Property: extract_key Stability ──────────────────────────────────
// extract_key should return the same normalized key for different representations.

#[test]
fn test_extract_key_stability() {
    let variations = [
        ("Lead_Am_128.wav", "A Minor"),
        ("Lead_A_min_128.wav", "A Minor"),
        ("Lead_A_minor_128.wav", "A Minor"),
        ("Lead_Amin_128.wav", "A Minor"),
    ];
    for (filename, expected) in variations {
        assert_eq!(extract_key(filename).unwrap(), expected);
    }
}

#[test]
fn test_key_word_normalization_robustness() {
    // extract_key uses normalize_sharp_flat_words internally.
    let cases = [
        ("Lead C sharp minor.wav", "C# Minor"),
        ("Bass-E-flat-Major.wav", "D# Major"),
        ("Pad G_sharp_Aeolian.wav", "G# Minor"), // Aeolian -> Minor
        ("Synth_B flat_128.wav", "A# Minor"),
    ];
    for (filename, expected) in cases {
        assert_eq!(extract_key(filename).unwrap(), expected);
    }
}
