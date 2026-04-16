//! Shared sample filtering constants for ALS generators
//! 
//! These exclusions apply to techno, trance, and schranz generation.
//! They filter out genres/styles that don't fit dark electronic music.

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

fn main() {
    println!("This example is a container for shared constants and tests.");
}
