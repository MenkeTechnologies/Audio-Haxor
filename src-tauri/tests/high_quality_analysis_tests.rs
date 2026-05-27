use app_lib::sample_analysis::{detect_manufacturer, extract_pack_name, match_category};
use app_lib::sample_filters::{BAD_GENRES, BAD_GENRES_TRANCE, is_excluded_genre};

#[test]
fn test_category_matching_normalization_edge_cases() {
    let m1 = match_category("TechKick.wav", "/").unwrap();
    assert_eq!(m1.name, "kick");
    let m2 = match_category("Bass_01_A_138.wav", "/").unwrap();
    assert_eq!(m2.name, "mid_bass");
    let m3 = match_category("Snare.wav", "/Samples/Kicks/").unwrap();
    assert_eq!(m3.name, "snare");
}

#[test]
fn test_manufacturer_detection_prioritization() {
    let path = "/Samples/Producer Loops/Ztekno - Hard Techno/Kicks";
    let m = detect_manufacturer(path).unwrap();
    assert_eq!(m.manufacturer_pattern, "Hard Techno");
    assert!(m.hardness_score > 0.5);

    let path2 = "/Samples/ Riemann / Techno";
    let m2 = detect_manufacturer(path2).unwrap();
    assert_eq!(m2.manufacturer_pattern, "Riemann");
}

#[test]
fn test_pack_name_extraction_heuristics() {
    assert_eq!(
        extract_pack_name("/Samples/My-Epic-Pack/Kicks"),
        Some("My-Epic-Pack".into())
    );
    assert_eq!(
        extract_pack_name("/Samples/Riemann Kollektion/Techno/Drums"),
        Some("Riemann Kollektion".into())
    );
    assert_eq!(
        extract_pack_name("/Samples/Vengeance Essential House/Claps"),
        Some("Vengeance Essential House".into())
    );
    assert!(extract_pack_name("/Samples/Kicks").is_none());
}

#[test]
fn test_negation_in_filename_matching() {
    assert!(match_category("No Kick.wav", "/").is_none());
    assert!(match_category("Without Bass.wav", "/").is_none());
    assert!(match_category("Non-Snare.wav", "/").is_none());
    assert!(match_category("Noah Kick.wav", "/").is_some());
}

#[test]
fn test_category_is_oneshot_property() {
    let m1 = match_category("Kick.wav", "/").unwrap();
    assert!(
        !m1.is_oneshot,
        "Kick currently is_oneshot=false in production patterns"
    );
    let m2 = match_category("SubBass.wav", "/").unwrap();
    assert!(m2.is_oneshot);
}

#[test]
fn test_is_excluded_genre_robustness() {
    assert!(is_excluded_genre("/Samples/Samba/Perc", BAD_GENRES));
    assert!(!is_excluded_genre("/Samples/Samba-Techno/Perc", BAD_GENRES));
    assert!(!is_excluded_genre("/Samples/TECHNO_Samba/Perc", BAD_GENRES));
    assert!(!is_excluded_genre("/Samples/Industrial/Drums", BAD_GENRES));
}

#[test]
fn test_is_excluded_genre_label_trust() {
    // "Riemann" is a trusted electronic label. Even if "Afro" (bad genre)
    // is in the path, we don't exclude if the label is recognized.
    // "Riemann" signal has genre -0.8 (Techno), so it's non-neutral.
    assert!(!is_excluded_genre(
        "/Samples/Riemann Afro Techno/Kicks",
        BAD_GENRES
    ));

    // Non-neutral manufacturer wins
    assert!(!is_excluded_genre(
        "/Samples/Freshly Squeezed Samba/Trance",
        BAD_GENRES
    ));
}

#[test]
fn test_bad_genres_trance_uplifting_exception() {
    // In BAD_GENRES_TRANCE, we don't have "uplifting" or "euphoric"
    assert!(!is_excluded_genre(
        "/Samples/Uplifting Trance/Lead",
        BAD_GENRES_TRANCE
    ));
}

#[test]
fn test_is_excluded_genre_case_insensitivity() {
    assert!(is_excluded_genre("/SAMBA/loops", BAD_GENRES));
    assert!(!is_excluded_genre("/TRANCE/samba", BAD_GENRES));
}
