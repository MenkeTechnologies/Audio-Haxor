//! Shared sample filtering constants for ALS generators
//! 
//! These exclusions apply to techno, trance, and schranz generation.
//! They filter out genres/styles that don't fit dark electronic music.

/// Reversed sample suffixes - files ending with these are reversed versions
/// Filter these out unless explicitly looking for reversed samples
pub const REVERSED_SUFFIXES: &[&str] = &[
    "-R.wav", " R.wav", "_R.wav", "-R.aif", " R.aif", "_R.aif",
];

/// Keywords indicating frozen/consolidated/rendered project files
pub const PROJECT_RENDER_KEYWORDS: &[&str] = &[
    "frozen", "consolidated", "flattened", "bounced", "rendered",
];

/// Keywords indicating construction kits/stems - not usable as loops
/// Construction kits are full song parts meant to be mixed together, not looped individually
/// Stems are isolated track bounces from full productions
pub const CONSTRUCTION_KIT_KEYWORDS: &[&str] = &[
    "construction kit", "construction_kit", "constructionkit",
    "/stems/", "\\stems\\", "/stem/", "\\stem\\",
    "_stem_", "_stem.", " stem.", " stem ",
    "full mix", "full_mix", "fullmix",
    "song starter", "song_starter", "songstarter",
    "track starter", "track_starter",
    "production kit", "production_kit",
    "demo track", "demo_track",
];

/// Check if a sample path is inside an Ableton Live project directory.
/// Ableton projects have structure: "Something Project/Samples/..." with an .als file nearby.
pub fn is_ableton_project_sample(path: &str) -> bool {
    // Pattern 1: path contains " Project/Samples/" (Ableton's default naming)
    // e.g., "Zforce-Alert Project/Samples/Imported/..."
    if path.contains(" Project/Samples/") || path.contains(" Project/Samples\\")
        || path.contains(" Project\\Samples/") || path.contains(" Project\\Samples\\") {
        return true;
    }
    
    // Pattern 2: path contains "/Samples/Processed/" or "/Samples/Imported/" or "/Samples/Recorded/"
    // These are Ableton-specific subdirectories inside project folders
    if path.contains("/Samples/Processed/") || path.contains("/Samples/Imported/") 
        || path.contains("/Samples/Recorded/") || path.contains("/Samples/Consolidated/")
        || path.contains("\\Samples\\Processed\\") || path.contains("\\Samples\\Imported\\")
        || path.contains("\\Samples\\Recorded\\") || path.contains("\\Samples\\Consolidated\\") {
        return true;
    }
    
    false
}

/// Global genre exclusions - samples containing these keywords are filtered out
/// 
/// Apply to ALL sample queries when generating techno/trance/schranz
pub const BAD_GENRES: &[&str] = &[
    // World/ethnic - wrong vibe entirely
    "samba", "latin", "bossa", "salsa", "reggae", "reggaeton", "afro", "african",
    "world", "ethnic", "tribal", "oriental", "arabic", "indian", "asian", "celtic",
    "flamenco", "cumbia", "bachata", "merengue", "calypso", "caribbean",
    
    // Pop/commercial - too bright/happy
    "disco", "nudisco", "nu_disco", "nu-disco", "funky", "funk", "soul", "motown",
    "pop", "chart", "commercial", "radio", "mainstream",
    
    // House subgenres (keep big_room, EDM, festival, hardstyle - those are fine for electronic)
    "deep_house", "tropical", "future_house",
    "progressive_house", "electro_house", "dutch", "bounce",
    
    // Chill/downtempo - too relaxed
    "lounge", "chillout", "chill", "downtempo", "ambient_pop", "easy_listening",
    "lo-fi", "lofi", "bedroom", "indie",
    
    // Hip-hop/R&B - different groove
    "hip_hop", "hiphop", "hip-hop", "trap", "rnb", "r&b", "rap", "boom_bap",
    
    // Rock/band - acoustic/organic
    "rock", "guitar", "acoustic", "folk", "country", "blues", "jazz",
    
    // Cinematic/orchestral
    "cinematic", "film", "movie", "orchestral", "classical", "epic",
    
    // Wrong character
    "organic", "natural", "live", "vintage", "retro", "80s", "70s", "60s",
    "happy", "uplifting", "euphoric", "cheerful", "bright", "sunny",
    
    // Sample pack brands known for non-electronic content
    "ghosthack", "cymatics", "splice_top", "beatport_top",
];

/// Trance-specific exclusions - same as BAD_GENRES but allows uplifting/euphoric
/// since those are valid trance subgenres
pub const BAD_GENRES_TRANCE: &[&str] = &[
    // World/ethnic
    "samba", "latin", "bossa", "salsa", "reggae", "reggaeton", "afro", "african",
    "world", "ethnic", "tribal", "oriental", "arabic", "indian", "asian", "celtic",
    "flamenco", "cumbia", "bachata", "merengue", "calypso", "caribbean",
    
    // Pop/commercial
    "disco", "nudisco", "nu_disco", "nu-disco", "funky", "funk", "soul", "motown",
    "pop", "chart", "commercial", "radio", "mainstream",
    
    // EDM/festival (but keep progressive for prog trance)
    "deep_house", "tropical", "future_house", "big_room", "festival",
    "electro_house", "dutch", "bounce", "hardstyle",
    
    // Chill/downtempo
    "lounge", "chillout", "chill", "downtempo", "ambient_pop", "easy_listening",
    "lo-fi", "lofi", "bedroom", "indie",
    
    // Hip-hop/R&B
    "hip_hop", "hiphop", "hip-hop", "trap", "rnb", "r&b", "rap", "boom_bap",
    
    // Rock/band
    "rock", "guitar", "acoustic", "folk", "country", "blues", "jazz",
    
    // Cinematic/orchestral
    "cinematic", "film", "movie", "orchestral", "classical", "epic",
    
    // Wrong character (NOTE: uplifting/euphoric allowed for trance)
    "organic", "natural", "live", "vintage", "retro", "80s", "70s", "60s",
    "happy", "cheerful", "bright", "sunny",
    
    // Sample pack brands
    "ghosthack", "cymatics", "splice_top", "beatport_top",
];

/// Schranz-specific exclusions - most restrictive, only industrial/hard sounds
pub const BAD_GENRES_SCHRANZ: &[&str] = &[
    // Everything from BAD_GENRES plus:
    
    // World/ethnic
    "samba", "latin", "bossa", "salsa", "reggae", "reggaeton", "afro", "african",
    "world", "ethnic", "tribal", "oriental", "arabic", "indian", "asian", "celtic",
    "flamenco", "cumbia", "bachata", "merengue", "calypso", "caribbean",
    
    // Pop/commercial
    "disco", "nudisco", "nu_disco", "nu-disco", "funky", "funk", "soul", "motown",
    "pop", "chart", "commercial", "radio", "mainstream",
    
    // EDM/festival
    "house", "deep_house", "tropical", "future_house", "big_room", "festival",
    "progressive_house", "electro_house", "dutch", "bounce",
    // Note: hardstyle may overlap with schranz, so not excluded
    
    // Chill/downtempo
    "lounge", "chillout", "chill", "downtempo", "ambient_pop", "easy_listening",
    "lo-fi", "lofi", "bedroom", "indie",
    
    // Hip-hop/R&B
    "hip_hop", "hiphop", "hip-hop", "trap", "rnb", "r&b", "rap", "boom_bap",
    
    // Rock/band
    "rock", "guitar", "acoustic", "folk", "country", "blues", "jazz",
    
    // Cinematic/orchestral
    "cinematic", "film", "movie", "orchestral", "classical", "epic",
    
    // Wrong character - schranz is dark/industrial only
    "organic", "natural", "live", "vintage", "retro", "80s", "70s", "60s",
    "happy", "uplifting", "euphoric", "cheerful", "bright", "sunny",
    "soft", "gentle", "smooth", "mellow", "warm",
    
    // Trance (wrong genre for schranz)
    "trance", "psytrance", "goa",
    
    // Sample pack brands
    "ghosthack", "cymatics", "splice_top", "beatport_top",
];

/// Helper to combine BAD_GENRES with additional exclusions
pub fn exclude_with<'a>(base: &[&'a str], extras: &[&'a str]) -> Vec<&'a str> {
    let mut v: Vec<&'a str> = base.to_vec();
    v.extend_from_slice(extras);
    v
}

/// Cross-category exclusions to prevent sample misclassification
/// Each category should exclude terms from other categories
pub mod cross_exclude {
    pub const DRUMS_EXCLUDE: &[&str] = &[
        "bass", "sub", "synth", "melody", "lead", "pad", "arp", "chord",
    ];
    
    pub const BASS_EXCLUDE: &[&str] = &[
        "kick", "drum", "drums", "hat", "snare", "clap", "perc", "ride", 
        "cymbal", "tom", "full", "kit", "synth", "lead", "pad", "arp", "melody",
    ];
    
    pub const MELODIC_EXCLUDE: &[&str] = &[
        "drum", "drums", "kick", "hat", "snare", "clap", "perc", "ride",
        "full", "kit", "bass", "sub",
    ];
    
    pub const FILL_EXCLUDE: &[&str] = &[
        "bass", "synth", "pad", "lead", "melody", "loop", "full", "8bar", "4bar", "chord",
    ];
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_bad_genres_not_empty() {
        assert!(!BAD_GENRES.is_empty());
        assert!(!BAD_GENRES_TRANCE.is_empty());
        assert!(!BAD_GENRES_SCHRANZ.is_empty());
    }
    
    #[test]
    fn test_exclude_with() {
        let result = exclude_with(BAD_GENRES, &["extra1", "extra2"]);
        assert!(result.contains(&"samba"));
        assert!(result.contains(&"extra1"));
        assert!(result.contains(&"extra2"));
    }
}
