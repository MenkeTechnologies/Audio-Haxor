use app_lib::sample_analysis::{detect_manufacturer, analyze_sample};
use app_lib::sample_filters::{is_ableton_project_sample, BAD_GENRES, BAD_GENRES_TRANCE, BAD_GENRES_SCHRANZ};

#[test]
fn test_genre_detection_from_manufacturer_signals() {
    // Trance leaning (positive genre_score)
    let m = detect_manufacturer("/Samples/Trance/Armada/Pack1/").expect("Armada should be detected");
    assert!(m.genre_score > 0.0, "Armada should lean toward trance");
    assert_eq!(m.manufacturer_pattern, "Armada");

    let m = detect_manufacturer("/Users/wizard/Samples/Anjuna/Sounds/").expect("Anjuna should be detected");
    assert!(m.genre_score > 0.0, "Anjuna should lean toward trance");

    // Techno leaning (negative genre_score)
    let m = detect_manufacturer("/Volumes/Storage/Drumcode/Essentials/").expect("Drumcode should be detected");
    assert!(m.genre_score < 0.0, "Drumcode should lean toward techno");
    assert_eq!(m.manufacturer_pattern, "Drumcode");

    let m = detect_manufacturer("/Samples/Techno/Mord/Loops/").expect("Mord should be detected");
    assert!(m.genre_score < 0.0, "Mord should lean toward techno");

    // Schranz leaning (strong negative score, high hardness)
    let m = detect_manufacturer("/Samples/Schranz/Schranz Total/").expect("Schranz Total should be detected");
    assert!(m.genre_score <= -0.9, "Schranz Total should lean heavily toward techno/schranz");
    assert!(m.hardness_score >= 0.9, "Schranz Total should be hard");
}

#[test]
fn test_genre_detection_priority() {
    // Longer match wins if scores are same, but non-neutral beats neutral
    // "Hard Techno" (-1.0, 1.0) vs "Loopmasters" (0.0, 0.0)
    let m = detect_manufacturer("/Samples/Loopmasters/Hard Techno Vol 1/").expect("Match expected");
    assert_eq!(m.manufacturer_pattern, "Hard Techno");
    assert_eq!(m.genre_score, -1.0);
}

#[test]
fn test_full_analysis_genre_lean() {
    // Test analyze_sample integration with genre-leaning manufacturers
    let a = analyze_sample(
        "Kick_01.wav",
        "/Samples/Filth on Acid/Reinier Zonneveld/Kicks/"
    );
    
    let manufacturer = a.manufacturer.expect("Manufacturer should be detected");
    // Reinier Zonneveld is -0.8 genre_score (techno)
    assert!(manufacturer.genre_score < 0.0);
    assert_eq!(manufacturer.manufacturer_pattern, "Reinier Zonneveld");
}

#[test]
fn test_neutral_manufacturers() {
    let m = detect_manufacturer("/Samples/Splice/").expect("Splice should be detected");
    assert_eq!(m.genre_score, 0.0);
    assert_eq!(m.hardness_score, 0.0);
    
    let m = detect_manufacturer("/Samples/Loopmasters/").expect("Loopmasters should be detected");
    assert_eq!(m.genre_score, 0.0);
}

#[test]
fn test_case_insensitivity() {
    let m = detect_manufacturer("/samples/drumcode/techno/").expect("drumcode should be detected");
    assert_eq!(m.manufacturer_pattern, "Drumcode");
}

#[test]
fn test_bad_genre_keywords() {
    // Ensure "samba" is in BAD_GENRES
    assert!(BAD_GENRES.contains(&"samba"));
    
    // Test a few keywords from the lists
    let path_samba = "/Samples/Samba_Drums/Loop.wav";
    assert!(BAD_GENRES.iter().any(|g| path_samba.to_lowercase().contains(g)));
    
    let path_hiphop = "/Samples/Hip_Hop_Beats/Kick.wav";
    assert!(BAD_GENRES.iter().any(|g| path_hiphop.to_lowercase().contains(g)));
    
    // Trance allows uplifting/euphoric
    assert!(!BAD_GENRES_TRANCE.contains(&"uplifting"));
    assert!(!BAD_GENRES_TRANCE.contains(&"euphoric"));
    assert!(BAD_GENRES.contains(&"uplifting"));
    assert!(BAD_GENRES.contains(&"euphoric"));
    
    // Schranz is very restrictive
    assert!(BAD_GENRES_SCHRANZ.contains(&"trance"));
    assert!(BAD_GENRES_SCHRANZ.contains(&"house"));
    assert!(BAD_GENRES_SCHRANZ.contains(&"soft"));
}

#[test]
fn test_ableton_project_sample_detection() {
    assert!(is_ableton_project_sample("/Users/wizard/Music/My Project/Samples/Imported/Kick.wav"));
    assert!(is_ableton_project_sample("C:\\Music\\Test Project\\Samples\\Processed\\Lead.wav"));
    assert!(!is_ableton_project_sample("/Users/wizard/Music/Samples/Techno/Kick.wav"));
}
