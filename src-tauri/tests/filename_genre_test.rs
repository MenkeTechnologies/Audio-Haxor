use app_lib::sample_analysis::{match_category, detect_manufacturer, analyze_sample};

#[test]
fn test_schranz_filename_parsing() {
    let m = match_category("SchranzKick_01.wav", "/Samples/").expect("Should match kick");
    println!("ACTUAL_MATCH: {}", m.name);
    assert_eq!(m.name, "kick");
}

#[test]
fn test_filename_genre_scoring_simulation() {
    let techno_keywords = vec!["techno", "tech", "warehouse", "berlin", "underground", "minimal", "industrial"];
    let trance_keywords = vec!["trance", "uplifting", "progressive", "euphoric", "psy", "melodic", "epic"];
    let schranz_keywords = vec!["schranz", "hardtechno", "hard techno", "industrial", "distorted", "aggressive", "rave"];

    let filename_techno = "Industrial_Techno_Kick_01.wav";
    let filename_trance = "Uplifting_Trance_Lead_Am.wav";
    let filename_schranz = "Relentless_Schranz_Drive.wav";

    let score_techno = techno_keywords.iter().filter(|kw| filename_techno.to_lowercase().contains(*kw)).count();
    assert!(score_techno >= 2);

    let score_trance = trance_keywords.iter().filter(|kw| filename_trance.to_lowercase().contains(*kw)).count();
    assert!(score_trance >= 2);

    let score_schranz = schranz_keywords.iter().filter(|kw| filename_schranz.to_lowercase().contains(*kw)).count();
    assert!(score_schranz >= 1);
}

#[test]
fn test_filename_vs_directory_genre_clash() {
    let directory = "/Samples/Trance/Armada/Pack1/";
    let filename = "Techno_Kick_01.wav";
    
    let m = detect_manufacturer(directory).expect("Armada detected");
    assert!(m.genre_score > 0.0);
    
    let a = analyze_sample(filename, directory);
    assert_eq!(a.manufacturer.unwrap().manufacturer_pattern, "Armada");
}

#[test]
fn test_genre_specific_exclusions_in_filenames() {
    use app_lib::sample_filters::BAD_GENRES;
    let filename = "Techno_Bossa_Nova_Loop.wav";
    let is_bad = BAD_GENRES.iter().any(|bad| filename.to_lowercase().contains(bad));
    assert!(is_bad);
}
