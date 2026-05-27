use app_lib::als_project::{Genre, SectionLengths, get_compatible_keys};

#[test]
fn test_circle_of_fifths_harmonic_compat() {
    // C Major is compatible with C Major and A Minor
    let c_maj = get_compatible_keys("C", "Ionian");
    assert!(c_maj.contains(&"C Major".to_string()));
    assert!(c_maj.contains(&"A Minor".to_string()));

    // G Major (one sharp) should be compatible with G Major and E Minor
    let g_maj = get_compatible_keys("G", "Ionian");
    assert!(g_maj.contains(&"G Major".to_string()));
    assert!(g_maj.contains(&"E Minor".to_string()));

    // F Major (one flat) should be compatible with F Major and D Minor
    let f_maj = get_compatible_keys("F", "Ionian");
    assert!(f_maj.contains(&"F Major".to_string()));
    assert!(f_maj.contains(&"D Minor".to_string()));
}

#[test]
fn test_all_modes_relative_keys() {
    let cases = [
        ("C", "Ionian", "C Major", "A Minor"),
        ("D", "Dorian", "C Major", "A Minor"),
        ("E", "Phrygian", "C Major", "A Minor"),
        ("F", "Lydian", "C Major", "A Minor"),
        ("G", "Mixolydian", "C Major", "A Minor"),
        ("A", "Aeolian", "C Major", "A Minor"),
        ("B", "Locrian", "C Major", "A Minor"),
    ];

    for (root, mode, expected_maj, expected_min) in cases {
        let compat = get_compatible_keys(root, mode);
        assert!(
            compat.contains(&expected_maj.to_string()),
            "Mode {} should match relative major {}",
            mode,
            expected_maj
        );
        assert!(
            compat.contains(&expected_min.to_string()),
            "Mode {} should match relative minor {}",
            mode,
            expected_min
        );
    }
}

#[test]
fn test_enharmonic_normalization_compat() {
    // Our system uses sharps internally for NOTES.
    // Roots not in NOTES get a fallback "X Minor" result.

    let db_maj = get_compatible_keys("C#", "Ionian");
    assert!(db_maj.contains(&"C# Major".to_string()));
    assert!(db_maj.contains(&"A# Minor".to_string()));

    // D# Minor (Aeolian)
    let eb_min = get_compatible_keys("D#", "Aeolian");
    assert!(eb_min.contains(&"F# Major".to_string()));
    assert!(eb_min.contains(&"D# Minor".to_string()));

    // Check fallback for flat which is not in NOTES
    let flat_fallback = get_compatible_keys("Db", "Ionian");
    assert_eq!(flat_fallback, vec!["Db Minor".to_string()]);
}

#[test]
fn test_section_lengths_sanitization_extremes() {
    // Massive values should still snap to 8
    let huge = SectionLengths {
        intro: 1000,
        build: 512,
        breakdown: 256,
        drop1: 128,
        drop2: 64,
        fadedown: 32,
        outro: 10000,
    };
    let s = huge.sanitize();
    assert_eq!(s.intro, (1000 / 8) * 8);
    assert_eq!(s.outro, (10000 / 8) * 8);
    assert!(s.total_bars() % 8 == 0);

    // Odd values just above 8
    let tiny_odd = SectionLengths {
        intro: 9,
        build: 15,
        breakdown: 17,
        drop1: 23,
        drop2: 25,
        fadedown: 31,
        outro: 33,
    };
    let s2 = tiny_odd.sanitize();
    assert_eq!(s2.intro, 8);
    assert_eq!(s2.build, 8);
    assert_eq!(s2.breakdown, 16);
    assert_eq!(s2.drop1, 16);
    assert_eq!(s2.drop2, 24);
    assert_eq!(s2.fadedown, 24);
    assert_eq!(s2.outro, 32);
}

#[test]
fn test_section_lengths_total_bars_consistency() {
    let genres = [Genre::Techno, Genre::Trance, Genre::Schranz];
    for g in genres {
        let sl = SectionLengths::for_genre(g);
        let starts = sl.starts();
        // The end of the last section (outro) should be total_bars + 1
        // because it is (start, end_exclusive) and bars are 1-indexed.
        assert_eq!(starts.outro.1, sl.total_bars() + 1);
    }
}
