//! Sample analysis engine for ALS generation.
//!
//! Parses BPM, key, and category from sample **filenames and directory paths**
//! (not audio content). Designed to index 1.5M+ WAV files quickly so that
//! ALS generator queries hit pre-computed indexes instead of runtime regex.

use regex::Regex;
use std::sync::LazyLock;

// ---------------------------------------------------------------------------
// BPM extraction
// ---------------------------------------------------------------------------

/// Regex: 2–3 digit number (80–180) next to a BPM marker or delimited.
///
/// Matches patterns like:
///   `Loop_138_Am.wav`        — delimited by underscores
///   `Kick[140]_hard.wav`     — bracket-delimited
///   `Lead 145 bpm.wav`       — explicit BPM marker
///   `- 140 BPM - Bm`        — dash-delimited with BPM marker
///   `132bpm`                 — no delimiter, explicit marker
static BPM_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?x)
        # Explicit BPM marker (highest confidence)
        (\d{2,3})\s*[Bb][Pp][Mm]
        |
        # Bracket-delimited: [140]
        \[(\d{2,3})\]
        |
        # Delimiter-bounded: _138_, -140-, ' 145 '
        (?:^|[_\s\-])(\d{2,3})(?:[_\s\-]|$)
        ",
    )
    .unwrap()
});

/// Extract BPM from a sample filename. Returns `None` if no valid BPM found.
///
/// Only returns values in the 80–180 range (covers downtempo through hardstyle).
/// When multiple candidates exist, prefers explicit markers (`132bpm`) over
/// bare delimited numbers.
///
/// For delimiter-bounded numbers (e.g., `_140_`), requires context to avoid
/// false positives from variant numbers (e.g., `Clap_140.wav` is variant 140, not 140 BPM).
/// Context includes: "loop" in filename, key indicator after number, or "bpm" nearby.
pub fn extract_bpm(name: &str) -> Option<u32> {
    let name_lower = name.to_ascii_lowercase();
    let has_loop_context = name_lower.contains("loop");
    
    let mut best: Option<(u32, u8)> = None; // (bpm, priority) — lower priority wins

    for caps in BPM_RE.captures_iter(name) {
        // Groups: 1 = explicit marker, 2 = bracket, 3 = delimiter
        let (val_str, priority) = if let Some(m) = caps.get(1) {
            (m.as_str(), 0) // explicit "132bpm" — highest confidence
        } else if let Some(m) = caps.get(2) {
            (m.as_str(), 1) // bracket "[140]"
        } else if let Some(m) = caps.get(3) {
            // Delimiter-bounded numbers need context to avoid false positives
            // like "Clap_140.wav" where 140 is a variant number
            let match_end = caps.get(0).map(|m| m.end()).unwrap_or(0);
            let after_match = &name[match_end..];
            
            // Check for context that indicates this is actually BPM:
            // 1. Filename contains "loop"
            // 2. Number is followed by a key indicator (A-G, with optional # or b)
            // 3. Number is at start and followed by key (e.g., "120_C_HousyBass")
            let has_key_after = after_match.chars().next()
                .map(|c| c.is_ascii_alphabetic() && "ABCDEFG".contains(c.to_ascii_uppercase()))
                .unwrap_or(false);
            
            if !has_loop_context && !has_key_after {
                continue; // Skip this match - likely a variant number
            }
            
            (m.as_str(), 2) // delimiter "_138_"
        } else {
            continue;
        };

        if let Ok(bpm) = val_str.parse::<u32>() {
            if (80..=180).contains(&bpm) {
                match best {
                    Some((_, p)) if priority < p => best = Some((bpm, priority)),
                    None => best = Some((bpm, priority)),
                    _ => {}
                }
            }
        }
    }

    best.map(|(bpm, _)| bpm)
}

// ---------------------------------------------------------------------------
// Key extraction
// ---------------------------------------------------------------------------

/// All chromatic note names used in filenames.
const NOTES: &[&str] = &[
    "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
];

/// Flat equivalents for enharmonic matching.
const FLAT_NOTES: &[(&str, &str)] = &[
    ("Db", "C#"),
    ("Eb", "D#"),
    ("Gb", "F#"),
    ("Ab", "G#"),
    ("Bb", "A#"),
];

/// Regex for explicit minor key patterns: Am, Amin, A_min, A-min, A Minor, Am7, Amin7
static MINOR_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)(?:^|[_\s\-\[(/])([A-G][#b]?)(?:m(?:in(?:or)?)?|[_\-\s]min(?:or)?)(?:\d+)?(?:[_\s\-\])/.,]|$)",
    )
    .unwrap()
});

/// Regex for explicit major key patterns: Cmaj, C_maj, C Major, CM (uppercase M), CMaj9
static MAJOR_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)(?:^|[_\s\-\[(/])([A-G][#b]?)(?:maj(?:or)?(?:\d+)?|[_\-\s]maj(?:or)?|M(?:aj)?(?:\d+)?)(?:[_\s\-\])/.,]|$)",
    )
    .unwrap()
});

/// Regex for bare note names delimited by underscores, spaces, dashes, or brackets.
/// These default to MINOR in electronic music.
/// Must NOT match inside words like "Crash", "Drive", "Bass", "Fade", "Gold", etc.
static BARE_NOTE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?:^|[_\s\-\[(/])([A-G][#b]?)(?:[_\s\-\])/.,]|$)",
    )
    .unwrap()
});

/// Normalize a note name: resolve flats to sharps, validate against NOTES.
fn normalize_note(raw: &str) -> Option<&'static str> {
    // Try direct match first
    let upper = if raw.len() == 1 {
        raw.to_uppercase()
    } else {
        // e.g., "f#" -> "F#", "bb" -> "Bb"
        let mut chars = raw.chars();
        let first = chars.next()?.to_uppercase().to_string();
        let rest: String = chars.collect();
        format!("{}{}", first, rest)
    };

    // Direct match
    if let Some(&note) = NOTES.iter().find(|&&n| n == upper) {
        return Some(note);
    }

    // Flat → sharp
    for &(flat, sharp) in FLAT_NOTES {
        if upper.eq_ignore_ascii_case(flat) {
            return Some(sharp);
        }
    }

    None
}

/// Static regexes for sharp/flat word normalization - compiled once
static SHARP_WORD_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)([A-G])[ _\-]?sharp").unwrap()
});
static FLAT_WORD_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)([A-G])[ _\-]?flat").unwrap()
});

/// Normalize "G Sharp" → "G#", "B Flat" → "Bb", etc. before regex matching.
/// This allows standard regexes to handle word-form accidentals.
fn normalize_sharp_flat_words(name: &str) -> String {
    let result = SHARP_WORD_RE.replace_all(name, "${1}#");
    let result = FLAT_WORD_RE.replace_all(&result, "${1}b");
    result.into_owned()
}

/// Extract musical key from a sample filename.
///
/// Priority:
/// 1. Explicit minor: `Am`, `Amin`, `A_min`, `A Minor`
/// 2. Explicit major: `Amaj`, `A_maj`, `A Major`, `AM`
/// 3. Bare note (defaults to minor): `_A_`, `- F# -`, `[C]`
///
/// Also handles word forms: "G Sharp Minor" → "G# Minor", "B Flat" → "Bb" → "A#"
///
/// Bare notes like `_F_` or `C#` default to **minor** because most
/// electronic music (techno/trance/schranz) is in minor keys.
pub fn extract_key(name: &str) -> Option<String> {
    // Preprocess: convert "G Sharp" → "G#", "B Flat" → "Bb"
    let normalized_name = normalize_sharp_flat_words(name);
    
    // 1. Try explicit minor
    if let Some(caps) = MINOR_RE.captures(&normalized_name) {
        if let Some(note) = normalize_note(caps.get(1)?.as_str()) {
            return Some(format!("{} Minor", note));
        }
    }

    // 2. Try explicit major
    if let Some(caps) = MAJOR_RE.captures(&normalized_name) {
        if let Some(note) = normalize_note(caps.get(1)?.as_str()) {
            return Some(format!("{} Major", note));
        }
    }

    // 3. Bare note → minor
    if let Some(caps) = BARE_NOTE_RE.captures(&normalized_name) {
        let raw = caps.get(1)?.as_str();
        if let Some(note) = normalize_note(raw) {
            // Reject false positives: single-char notes that are likely version/variant markers
            // If the note is a single letter and is surrounded by digits, skip it
            // e.g. "Vol_3_A_01" — "A" here is likely a variant, not a key
            // But "Lead Loop 8 C# 140 BPM" — "C#" IS a key
            if raw.len() == 1 {
                // Check if the character before the match is a digit (variant marker)
                let match_start = caps.get(0)?.start();
                let bytes = name.as_bytes();
                if match_start > 0 {
                    let _prev = bytes[match_start] as char; // delimiter char
                    // Look one more back for a digit
                    if match_start > 1 && bytes[match_start - 1].is_ascii_digit() {
                        // Could be "Vol3_A_" — check if next after note is also digit
                        let match_end = caps.get(0)?.end();
                        if match_end < bytes.len() && bytes[match_end - 1].is_ascii_digit() {
                            return None; // Likely a variant marker, not a key
                        }
                    }
                }
            }
            return Some(format!("{} Minor", note));
        }
    }

    None
}

/// Regex to strip minor key indicators, capturing surrounding delimiters
static STRIP_MINOR_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)([_\s\-\[(/])[A-G][#b]?(?:m(?:in(?:or)?)?|[_\-\s]min(?:or)?)(?:\d+)?([_\s\-\])/.,])"
    ).unwrap()
});

/// Regex to strip major key indicators, capturing surrounding delimiters
static STRIP_MAJOR_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)([_\s\-\[(/])[A-G][#b]?(?:maj(?:or)?(?:\d+)?|[_\-\s]maj(?:or)?|M(?:aj)?(?:\d+)?)([_\s\-\])/.,])"
    ).unwrap()
});

/// Regex to strip bare note at end of filename before extension: Loop-121-C.wav → Loop-121-.wav
static STRIP_BARE_NOTE_END_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"([_\s\-\[(/])[A-G][#b]?(\.[^.]+$)"
    ).unwrap()
});

/// Regex to strip bare note followed by variant: _C-02 → _-02
static STRIP_BARE_NOTE_VARIANT_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"([_\s\[(/])[A-G][#b]?(\-\d)"
    ).unwrap()
});

/// Regex to strip bare note names between delimiters: _A_, [C#], -F#-
static STRIP_BARE_NOTE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"([_\s\-\[(/])[A-G][#b]?([_\s\-\])/.,])"
    ).unwrap()
});

/// Strip key indicators from a sample path to create a key-agnostic identifier.
/// This allows blacklisting samples regardless of their key variant.
///
/// Examples:
///   `/samples/Lead_Am_120.wav` → `/samples/Lead__120.wav`
///   `/samples/Bass_C#_Hard.wav` → `/samples/Bass__Hard.wav`
///   `/samples/Pad - F# Minor - Soft.wav` → `/samples/Pad -  - Soft.wav`
///   `/samples/AtmosLoop_C-02.wav` → `/samples/AtmosLoop_-02.wav`
///   `/samples/Loop-121-C.wav` → `/samples/Loop-121-.wav`
pub fn strip_key_from_path(path: &str) -> String {
    // Normalize sharp/flat words first: "G Sharp" → "G#"
    let normalized = normalize_sharp_flat_words(path);
    
    // Strip keys in order of specificity
    let result = STRIP_MINOR_RE.replace_all(&normalized, "$1$2");
    let result = STRIP_MAJOR_RE.replace_all(&result, "$1$2");
    // Handle note at end before extension: Loop-121-C.wav
    let result = STRIP_BARE_NOTE_END_RE.replace_all(&result, "$1$2");
    // Handle note followed by variant number: _C-02
    let result = STRIP_BARE_NOTE_VARIANT_RE.replace_all(&result, "$1$2");
    // Handle note between delimiters: _A_
    let result = STRIP_BARE_NOTE_RE.replace_all(&result, "$1$2");
    
    result.into_owned()
}

// ---------------------------------------------------------------------------
// Category matching
// ---------------------------------------------------------------------------

/// A sample category with its regex pattern for filename matching.
pub struct CategoryPattern {
    pub name: &'static str,
    pub parent: Option<&'static str>,
    pub pattern: LazyLock<Regex>,
    pub is_oneshot: bool,
    pub is_key_sensitive: bool,
    pub is_loop_preferred: bool,
}

/// All category patterns ordered by specificity (most specific first).
/// When matching, the first match wins, so specific patterns must come before
/// general ones (e.g., `fx_riser` before `fx_misc`).
pub static CATEGORY_PATTERNS: &[(&str, &str, Option<&str>, bool, bool, bool)] = &[
    // (name, regex_pattern, parent, is_oneshot, is_key_sensitive, is_loop_preferred)

    // === DRUMS (specific first) ===
    ("kick",        r"(?i)kick|kik|(?:^|[\s_\-])bd(?:[\s_\-]|$)|(?:^|[\s_\-])kck(?:[\s_\-]|$)", Some("drums"), false, false, true),
    ("clap",        r"(?i)clap|snare|(?:^|[\s_\-])(?:clp|snr|sd)(?:[\s_\-]|$)", Some("drums"), false, false, true),
    ("closed_hat",  r"(?i)closed.?hat|closed.?hh|(?:^|[\s_\-])chh(?:[\s_\-]|$)", Some("drums"), false, false, true),
    ("open_hat",    r"(?i)open.?hat|open.?hh|(?:^|[\s_\-])ohh(?:[\s_\-]|$)", Some("drums"), false, false, true),
    ("ride",        r"(?i)(?:^|[\s_\-])ride(?:[\s_\-.]|$)", Some("drums"), false, false, true),
    ("perc",        r"(?i)perc|(?:^|[\s_\-])(?:tom|conga|bongo|shaker|rim|tambourine)(?:[\s_\-]|$)", Some("drums"), false, false, true),

    // === SCHRANZ-SPECIFIC ===
    ("schranz_kick",  r"(?i)schranz.*kick|kick.*schranz|hard[\s_]*techno.*kick", Some("drums"), false, false, true),
    ("schranz_drive", r"(?i)(?:^|[\s_\-])drive(?:[\s_\-]|$)|rumble[\s_]*(?:bass|loop)|schranz.*bass", Some("bass"), false, false, true),
    ("schranz_roll",  r"(?i)kick[\s_]*roll|roll[\s_]*kick|schranz[\s_]*roll", Some("drums"), false, false, true),

    // === BASS ===
    ("sub_bass",    r"(?i)(?:^|[\s_\-])sub(?:[\s_\-]|$)|(?:^|[\s_\-])808(?:[\s_\-]|$)|bass[\s_]*sub|low[\s_]*end", Some("bass"), true, true, false),
    ("mid_bass",    r"(?i)(?:^|[\s_\-])bass(?:[\s_\-]|$)|reese|hoover|wobble", Some("bass"), false, true, true),

    // === MELODIC ===
    ("lead",        r"(?i)(?:^|[\s_\-])lead(?:[\s_\-]|$)|(?:^|[\s_\-])ld[_\-]|synth[\s_]*lead|(?:^|[\s_\-])riff(?:[\s_\-]|$)", Some("melodic"), false, true, true),
    ("pad",         r"(?i)(?:^|[\s_\-])pad(?:[\s_\-]|$)|(?:^|[\s_\-])string(?:s?)(?:[\s_\-]|$)|(?:^|[\s_\-])chord(?:[\s_\-]|$)|evolve", Some("melodic"), false, true, true),
    ("arp",         r"(?i)(?:^|[\s_\-])arp(?:[\s_\-]|$)|sequence|(?:^|[\s_\-])seq[_\-]", Some("melodic"), false, true, true),
    ("pluck",       r"(?i)pluck|pizz|picked|marimba", Some("melodic"), true, true, false),
    ("stab",        r"(?i)(?:^|[\s_\-])stab(?:[\s_\-]|$)|(?:^|[\s_\-])brass(?:[\s_\-]|$)", Some("melodic"), true, true, false),
    ("acid",        r"(?i)(?:^|[\s_\-])acid(?:[\s_\-]|$)|(?:^|[\s_\-])303(?:[\s_\-]|$)|squelch", Some("melodic"), false, true, true),

    // === ATMOS ===
    ("atmos",       r"(?i)atmos|ambient|drone|soundscape|background", Some("atmos"), false, true, false),
    ("texture",     r"(?i)texture|foley|field[\s_]*rec", Some("atmos"), false, false, false),
    ("noise",       r"(?i)(?:^|[\s_\-])noise(?:[\s_\-]|$)|(?:^|[\s_\-])static(?:[\s_\-]|$)|hiss|crackle", Some("atmos"), false, false, false),
    ("tape",        r"(?i)(?:^|[\s_\-])tape(?:[\s_\-]|$)|vinyl|lo-?fi|cassette", Some("atmos"), false, false, false),

    // === FX — Transitions ===
    ("fx_riser",    r"(?i)riser|(?:^|[\s_\-])rise(?:[\s_\-]|$)|sweep[\s_]*up|uplifter|build[\s_]*up|(?:^|[\s_\-])tension(?:[\s_\-]|$)|ascend", Some("fx"), false, false, false),
    ("fx_downer",   r"(?i)downer|downlifter|sweep[\s_]*down|down[\s_]*sweep|descend", Some("fx"), false, false, false),
    ("fx_swell",    r"(?i)swell|bloom|expand", Some("fx"), false, false, false),

    // === FX — Impacts ===
    ("fx_crash",    r"(?i)crash|(?:^|[\s_\-])china(?:[\s_\-]|$)|splash", Some("fx"), true, false, false),
    ("fx_impact",   r"(?i)impact|(?:^|[\s_\-])slam(?:[\s_\-]|$)|(?:^|[\s_\-])boom(?:[\s_\-]|$)|thud", Some("fx"), true, false, false),
    ("fx_explosion",r"(?i)explo|burst|detonate|blast", Some("fx"), true, false, false),

    // === FX — Rhythmic ===
    ("fx_fill",     r"(?i)(?:^|[\s_\-])fill(?:[\s_\-]|$)|snare[\s_]*roll|drum[\s_]*break|buildup", Some("fx"), false, false, true),
    ("fx_glitch",   r"(?i)glitch|stutter|(?:^|[\s_\-])chop(?:[\s_\-]|$)|(?:^|[\s_\-])slice(?:[\s_\-]|$)|granular|buffer", Some("fx"), false, false, true),

    // === FX — Tonal ===
    ("fx_whoosh",   r"(?i)whoosh|swish|swoosh", Some("fx"), true, false, false),
    ("fx_laser",    r"(?i)laser|(?:^|[\s_\-])zap(?:[\s_\-]|$)|(?:^|[\s_\-])beam(?:[\s_\-]|$)|sci-?fi|blaster", Some("fx"), true, false, false),
    ("fx_reverse",  r"(?i)reverse|(?:^|[\s_\-])rev[_\-]|backwards|reversed", Some("fx"), true, false, false),

    // === FX — Misc ===
    ("fx_sub_drop", r"(?i)sub[\s_]*drop|808[\s_]*drop|bass[\s_]*drop", Some("fx"), true, false, false),
    ("fx_white_noise", r"(?i)white[\s_]*noise|noise[\s_]*sweep|filtered[\s_]*noise", Some("fx"), false, false, false),
    ("fx_vocal",    r"(?i)fx[\s_]*vox|vocal[\s_]*fx|processed[\s_]*vocal|vocal[\s_]*chop", Some("fx"), false, false, true),
    ("fx_misc",     r"(?i)(?:^|[\s_\-])fx(?:[\s_\-]|$)|(?:^|[\s_\-])sfx(?:[\s_\-]|$)|transition|cinematic", Some("fx"), false, false, false),

    // === VOCAL ===
    ("vocal_chop",  r"(?i)vocal[\s_]*chop|chop[\s_]*vocal|vox[\s_]*chop", Some("vocal"), false, false, true),
    ("vocal_phrase", r"(?i)vocal[\s_]*phrase|(?:^|[\s_\-])phrase(?:[\s_\-]|$)|spoken|speech", Some("vocal"), false, true, false),
    ("vocal_adlib", r"(?i)adlib|(?:^|[\s_\-])shout(?:[\s_\-]|$)|(?:^|[\s_\-])scream(?:[\s_\-]|$)", Some("vocal"), true, false, false),
    ("vocal",       r"(?i)(?:^|[\s_\-])vox(?:[\s_\-]|$)|vocal|voice|chant|acapella", Some("vocal"), false, true, false),
];

/// Compiled category regex cache.
static COMPILED_PATTERNS: LazyLock<Vec<(&str, Regex, Option<&str>, bool, bool, bool)>> =
    LazyLock::new(|| {
        CATEGORY_PATTERNS
            .iter()
            .map(|(name, pat, parent, oneshot, key_sens, loop_pref)| {
                (
                    *name,
                    Regex::new(pat).unwrap(),
                    *parent,
                    *oneshot,
                    *key_sens,
                    *loop_pref,
                )
            })
            .collect()
    });

/// Result of category matching.
#[derive(Debug, Clone)]
pub struct CategoryMatch {
    pub name: String,
    pub parent: Option<String>,
    pub confidence: f32,
    pub is_oneshot: bool,
    pub is_key_sensitive: bool,
    pub is_loop_preferred: bool,
}

/// Match a sample to its category using filename first, directory as fallback.
///
/// Returns the best match with a confidence score:
/// - 1.0 = matched in filename (high confidence)
/// - 0.6 = matched in directory only (lower confidence)
/// - None = no match
pub fn match_category(name: &str, directory: &str) -> Option<CategoryMatch> {
    // Try filename first (higher confidence)
    for (cat_name, re, parent, oneshot, key_sens, loop_pref) in COMPILED_PATTERNS.iter() {
        if re.is_match(name) {
            return Some(CategoryMatch {
                name: cat_name.to_string(),
                parent: parent.map(|s| s.to_string()),
                confidence: 1.0,
                is_oneshot: *oneshot,
                is_key_sensitive: *key_sens,
                is_loop_preferred: *loop_pref,
            });
        }
    }

    // Fallback to directory (lower confidence)
    for (cat_name, re, parent, oneshot, key_sens, loop_pref) in COMPILED_PATTERNS.iter() {
        if re.is_match(directory) {
            return Some(CategoryMatch {
                name: cat_name.to_string(),
                parent: parent.map(|s| s.to_string()),
                confidence: 0.6,
                is_oneshot: *oneshot,
                is_key_sensitive: *key_sens,
                is_loop_preferred: *loop_pref,
            });
        }
    }

    None
}

// ---------------------------------------------------------------------------
// Pack / manufacturer detection
// ---------------------------------------------------------------------------

/// Known manufacturer/label signals matched against directory paths.
/// Format: (pattern, genre_score, hardness_score)
///   genre_score:    -1.0 = techno, +1.0 = trance, 0.0 = neutral
///   hardness_score: -1.0 = soft,   +1.0 = hard,   0.0 = neutral
pub const MANUFACTURER_SIGNALS: &[(&str, f32, f32)] = &[
    // === HARD DANCE / HARD TRANCE LABELS ===
    ("Tidy",              0.7,  0.9),
    ("Full On",           0.9,  0.5),
    ("Vandit",            0.9,  0.3),
    ("Armada",            0.7,  0.0),
    ("Anjuna",            0.8,  0.0),
    ("FSOE",              0.9,  0.3),
    ("Blackhole",         0.8,  0.2),
    ("Grotesque",         0.8,  0.4),
    ("WAO138",            0.9,  0.6),
    ("Kearnage",          0.9,  0.7),
    ("Subculture",        0.8,  0.5),
    ("Outburst",          0.8,  0.6),
    ("VII",               0.8,  0.4),
    ("Pure Trance",       0.9,  0.3),
    ("Damaged",           0.7,  0.8),

    // === SCHRANZ / HARD TECHNO LABELS ===
    ("Definition of Hard Techno", -1.0, 1.0),
    ("definitionofhardtechno",    -1.0, 1.0),
    ("Chris Liebing",    -1.0,  0.95),
    ("Stigmata",         -1.0,  0.95),
    ("Arkus P",          -1.0,  1.0),
    ("Robert Natus",     -1.0,  1.0),
    ("Viper XXL",        -1.0,  1.0),
    ("Leo Laker",        -1.0,  1.0),
    ("Sven Wittekind",   -1.0,  1.0),
    ("DJ Rush",          -1.0,  0.9),
    ("Boris S",          -1.0,  1.0),
    ("Frank Kvitta",     -1.0,  1.0),
    ("O.B.I.",           -1.0,  1.0),
    ("Noise Not War",    -1.0,  1.0),
    ("Klangkuenstler",   -1.0,  0.95),
    ("Pet Duo",          -1.0,  0.95),
    ("A.N.I",            -1.0,  1.0),
    ("Nikolina",         -1.0,  0.95),
    ("TRIPTYKH",         -1.0,  1.0),
    ("Elektrabel",       -1.0,  1.0),
    ("Schranz Total",    -1.0,  1.0),
    ("Schranz",          -1.0,  1.0),
    ("Hardtechno",       -1.0,  1.0),
    ("Hard Techno",      -1.0,  1.0),
    ("Amok",             -1.0,  1.0),
    ("Nachtstrom",       -1.0,  0.95),
    ("MB Elektronics",   -1.0,  0.9),

    // === TECHNO LABELS ===
    ("Drumcode",         -0.9,  0.5),
    ("Filth on Acid",    -0.8,  0.7),
    ("Exhale",           -0.7,  0.7),
    ("KNTXT",            -0.8,  0.6),
    ("Possession",       -0.7,  0.85),
    ("Perc Trax",        -0.8,  0.8),
    ("Mord",             -0.9,  0.85),
    ("Planet Rhythm",    -0.8,  0.5),
    ("Soma",             -0.7,  0.3),
    ("Tresor",           -0.8,  0.3),
    ("Ostgut Ton",       -0.9,  0.4),
    ("CLR",              -0.9,  0.7),
    ("Tronic",           -0.7,  0.4),
    ("Bedrock",          -0.6,  0.2),
    ("Cocoon",           -0.7,  0.3),
    ("Minus",            -0.8,  0.2),
    ("M_nus",            -0.8,  0.2),

    // === SAMPLE PACK COMPANIES ===
    ("Loopmasters",       0.0,  0.0),
    ("Splice",            0.0,  0.0),
    ("Sample Magic",      0.0,  0.0),
    ("Vengeance",         0.0,  0.3),
    ("Black Octopus",     0.0,  0.0),
    ("Ghosthack",         0.0,  0.2),
    ("Industrial Strength", -0.5, 0.9),
    ("Singomakers",       0.0,  0.0),
    ("Function Loops",    0.0,  0.0),
    ("Producer Loops",    0.0,  0.0),
    ("Zenhiser",         -0.3,  0.3),
    ("Freshly Squeezed",  0.5,  0.5),
    ("Mutekki",          -0.6,  0.4),
    ("Toolroom",         -0.4,  0.2),
    ("Revealed",          0.3,  0.4),
    ("Spinnin",           0.2,  0.2),

    // === ARTIST PACKS ===
    ("Allen Watts",       0.9,  0.5),
    ("Bryan Kearney",     0.9,  0.7),
    ("Simon Patterson",   0.8,  0.5),
    ("John Askew",        0.8,  0.7),
    ("Sean Tyas",         0.8,  0.5),
    ("Will Atkinson",     0.8,  0.6),
    ("Adam Ellis",        0.9,  0.4),
    ("ReOrder",           0.9,  0.4),
    ("Sneijder",          0.8,  0.6),
    ("Factor B",          0.9,  0.3),
    ("Adam Beyer",       -0.9,  0.5),
    ("Charlotte de Witte",-0.8,  0.7),
    ("Amelie Lens",      -0.7,  0.7),
    ("Reinier Zonneveld", -0.8, 0.7),
    ("UMEK",             -0.7,  0.5),
    ("Enrico Sangiuliano",-0.7, 0.5),
    ("Spartaque",        -0.8,  0.6),
    ("Alignment",        -0.8,  0.8),
    ("DYEN",             -0.7,  0.6),
    ("Afterlife",        -0.5,  0.2),
    ("Tale of Us",       -0.5,  0.1),
    ("Metta & Glyde",     0.9,  0.5),
    ("Metta Glyde",       0.9,  0.5),
];

/// Result of manufacturer/pack detection from a directory path.
#[derive(Debug, Clone)]
pub struct PackMatch {
    pub manufacturer_pattern: String,
    pub genre_score: f32,
    pub hardness_score: f32,
}

/// Detect manufacturer/label from a directory path.
///
/// Scans the full path for known manufacturer names and returns the
/// genre and hardness scores for the best match.
///
/// Ranking: non-neutral (genre/hardness != 0) beats neutral, then longer match wins.
/// This ensures "Tidy" (trance, hard) beats "Producer Loops" (neutral) even though
/// "Producer Loops" is a longer string.
pub fn detect_manufacturer(directory: &str) -> Option<PackMatch> {
    let dir_lower = directory.to_lowercase();

    // Collect all matches
    let mut matches: Vec<(&str, f32, f32)> = Vec::new();
    for &(pattern, genre, hardness) in MANUFACTURER_SIGNALS {
        let pat_lower = pattern.to_lowercase();
        if dir_lower.contains(&pat_lower) {
            matches.push((pattern, genre, hardness));
        }
    }

    if matches.is_empty() {
        return None;
    }

    // Sort: non-neutral first, then by pattern length descending
    matches.sort_by(|a, b| {
        let a_neutral = a.1 == 0.0 && a.2 == 0.0;
        let b_neutral = b.1 == 0.0 && b.2 == 0.0;
        // Non-neutral beats neutral
        a_neutral.cmp(&b_neutral).then_with(||
            // Longer match wins
            b.0.len().cmp(&a.0.len())
        )
    });

    let (pattern, genre, hardness) = matches[0];
    Some(PackMatch {
        manufacturer_pattern: pattern.to_string(),
        genre_score: genre,
        hardness_score: hardness,
    })
}

/// Extract the likely sample pack name from a directory path.
///
/// Looks for the deepest directory component that resembles a pack name
/// (contains spaces, dashes, volume numbers, etc.) rather than a generic
/// subfolder like "Kicks" or "Loops".
///
/// Example: `/Users/wizard/Samples/Producer loops/Tidy - Bits & Pieces Vol 1/Leads/`
///   → `"Tidy - Bits & Pieces Vol 1"`
pub fn extract_pack_name(directory: &str) -> Option<String> {
    // Generic subfolder names that are NOT pack names
    static GENERIC_DIRS: LazyLock<regex::RegexSet> = LazyLock::new(|| {
        regex::RegexSet::new([
            r"(?i)^(kicks?|snares?|claps?|hats?|rides?|percs?|percussion|cymbals?)$",
            r"(?i)^(bass|basses|leads?|pads?|synths?|strings?|keys?)$",
            r"(?i)^(fx|effects?|sfx|risers?|impacts?|sweeps?|transitions?)$",
            r"(?i)^(vocals?|vox|atmosphere|atmos|ambient|drones?)$",
            r"(?i)^(loops?|oneshots?|one[_\-]shots?|samples?|midi|audio|wav|presets?)$",
            r"(?i)^(drums?|melodic|tonal|rhythmic|percussive|fills?)$",
            r"(?i)^(Imported|Processed|Bounced|Rendered|Recorded|Raw|Original)$",
        ])
        .unwrap()
    });

    let components: Vec<&str> = directory
        .split(['/', '\\'])
        .filter(|c| !c.is_empty())
        .collect();

    // Walk from deepest to shallowest, skip generic names
    for &component in components.iter().rev() {
        if GENERIC_DIRS.is_match(component) {
            continue;
        }
        // A pack name typically has multiple words, dashes, or "Vol"
        if component.contains(' ') || component.contains('-') || component.contains("Vol") {
            return Some(component.to_string());
        }
    }

    // Fallback: return deepest non-generic component
    for &component in components.iter().rev() {
        if !GENERIC_DIRS.is_match(component) {
            return Some(component.to_string());
        }
    }

    None
}

// ---------------------------------------------------------------------------
// Full sample analysis
// ---------------------------------------------------------------------------

/// Complete analysis result for a single sample.
#[derive(Debug, Clone)]
pub struct SampleAnalysis {
    pub parsed_bpm: Option<u32>,
    pub parsed_key: Option<String>,
    pub category: Option<CategoryMatch>,
    pub pack_name: Option<String>,
    pub manufacturer: Option<PackMatch>,
    pub is_loop: bool,
}

/// Analyze a sample from its filename and directory path.
/// No audio decoding — purely string parsing.
pub fn analyze_sample(name: &str, directory: &str) -> SampleAnalysis {
    let parsed_bpm = extract_bpm(name);
    let parsed_key = extract_key(name);
    let category = match_category(name, directory);
    let pack_name = extract_pack_name(directory);
    let manufacturer = detect_manufacturer(directory);
    let is_loop = name.to_ascii_lowercase().contains("loop");

    SampleAnalysis {
        parsed_bpm,
        parsed_key,
        category,
        pack_name,
        manufacturer,
        is_loop,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- BPM ---

    #[test]
    fn bpm_explicit_marker() {
        assert_eq!(extract_bpm("Lead 145 bpm.wav"), Some(145));
        assert_eq!(extract_bpm("132bpm"), Some(132));
        assert_eq!(extract_bpm("RY_ELYSIUM_LEAD LOOP_002_F_132bpm"), Some(132));
        assert_eq!(extract_bpm("140BPM_Kick.wav"), Some(140));
    }

    #[test]
    fn bpm_bracket_delimited() {
        assert_eq!(extract_bpm("Kick[140]_hard.wav"), Some(140));
        assert_eq!(extract_bpm("Bass[128].wav"), Some(128));
    }

    #[test]
    fn bpm_underscore_delimited() {
        assert_eq!(extract_bpm("Loop_138_Am.wav"), Some(138));
        assert_eq!(extract_bpm("Pad_120_Cm.wav"), Some(120));
    }

    #[test]
    fn bpm_dash_delimited() {
        assert_eq!(extract_bpm("Tidy1 - Lead Loop 10 - PT3 - 140 BPM - Bm"), Some(140));
    }

    #[test]
    fn bpm_out_of_range() {
        assert_eq!(extract_bpm("Sample_44100_Hz.wav"), None); // 44100 not in 80-180
        assert_eq!(extract_bpm("Track_01.wav"), None);        // 01 not in range
    }

    #[test]
    fn bpm_prefers_explicit_marker() {
        // If both explicit and delimited exist, prefer explicit
        assert_eq!(extract_bpm("Loop_100_128bpm.wav"), Some(128));
    }

    // --- Key ---

    #[test]
    fn key_explicit_minor() {
        assert_eq!(extract_key("BassLoop_Reeeeze_142_Cm_PL"), Some("C Minor".into()));
        assert_eq!(extract_key("Tidy1 - Lead Loop 10 - PT3 - 140 BPM - Bm"), Some("B Minor".into()));
        assert_eq!(extract_key("Synth Chord Pad Cosmic Fmin"), Some("F Minor".into()));
        assert_eq!(extract_key("Loop_Am_128.wav"), Some("A Minor".into()));
    }

    #[test]
    fn key_explicit_major() {
        assert_eq!(extract_key("E-Piano Motion CMaj9"), Some("C Major".into()));
        assert_eq!(extract_key("Pad_Gmaj_120.wav"), Some("G Major".into()));
    }

    #[test]
    fn key_bare_note_defaults_minor() {
        assert_eq!(extract_key("RY_ELYSIUM_LEAD LOOP_002_F_132bpm"), Some("F Minor".into()));
        assert_eq!(extract_key("Full On Lead Loop 8 C# 140 BPM"), Some("C# Minor".into()));
    }

    #[test]
    fn key_sharps_and_flats() {
        assert_eq!(extract_key("Lead_F#m_140.wav"), Some("F# Minor".into()));
        assert_eq!(extract_key("Pad_Bbm_120.wav"), Some("A# Minor".into())); // Bb → A#
        assert_eq!(extract_key("Bass_Ebmin.wav"), Some("D# Minor".into()));   // Eb → D#
    }
    
    #[test]
    fn key_sharp_flat_words() {
        // "G Sharp" == "G#" (NOT folded to F#!)
        assert_eq!(extract_key("DPTE2 Bass Loop - 001 - G Sharp - 140 BPM.wav"), Some("G# Minor".into()));
        assert_eq!(extract_key("DPTE2 Pluck Loop - 016 - G Sharp Minor - 140 BPM.wav"), Some("G# Minor".into()));
        assert_eq!(extract_key("Propht - C Sharp Minor.wav"), Some("C# Minor".into()));
        assert_eq!(extract_key("DPTE2 Vocal Stab - 006 - C Sharp.wav"), Some("C# Minor".into()));
        // "B Flat" == "Bb" == "A#"
        assert_eq!(extract_key("Synth_B Flat Minor_120.wav"), Some("A# Minor".into()));
        assert_eq!(extract_key("Lead - E Flat - 128 BPM.wav"), Some("D# Minor".into()));
        // All sharp word forms → sharp notation (stays as sharp)
        assert_eq!(extract_key("Loop_A Sharp_140.wav"), Some("A# Minor".into()));
        assert_eq!(extract_key("Loop_C Sharp_140.wav"), Some("C# Minor".into()));
        assert_eq!(extract_key("Loop_D Sharp_140.wav"), Some("D# Minor".into()));
        assert_eq!(extract_key("Loop_F Sharp_140.wav"), Some("F# Minor".into()));
        assert_eq!(extract_key("Loop_G Sharp_140.wav"), Some("G# Minor".into()));
    }
    
    #[test]
    fn key_all_flats_fold_to_sharps() {
        // All flats must fold to their enharmonic sharp equivalent
        // Db → C#
        assert_eq!(extract_key("Loop_Dbm_140.wav"), Some("C# Minor".into()));
        assert_eq!(extract_key("Loop_D Flat_140.wav"), Some("C# Minor".into()));
        assert_eq!(extract_key("Loop - D Flat Minor.wav"), Some("C# Minor".into()));
        // Eb → D#
        assert_eq!(extract_key("Loop_Ebm_140.wav"), Some("D# Minor".into()));
        assert_eq!(extract_key("Loop_E Flat_140.wav"), Some("D# Minor".into()));
        assert_eq!(extract_key("Loop - E Flat Minor.wav"), Some("D# Minor".into()));
        // Gb → F#
        assert_eq!(extract_key("Loop_Gbm_140.wav"), Some("F# Minor".into()));
        assert_eq!(extract_key("Loop_G Flat_140.wav"), Some("F# Minor".into()));
        assert_eq!(extract_key("Loop - G Flat Minor.wav"), Some("F# Minor".into()));
        // Ab → G#
        assert_eq!(extract_key("Loop_Abm_140.wav"), Some("G# Minor".into()));
        assert_eq!(extract_key("Loop_A Flat_140.wav"), Some("G# Minor".into()));
        assert_eq!(extract_key("Loop - A Flat Minor.wav"), Some("G# Minor".into()));
        // Bb → A#
        assert_eq!(extract_key("Loop_Bbm_140.wav"), Some("A# Minor".into()));
        assert_eq!(extract_key("Loop_B Flat_140.wav"), Some("A# Minor".into()));
        assert_eq!(extract_key("Loop - B Flat Minor.wav"), Some("A# Minor".into()));
    }
    
    #[test]
    fn key_sharps_stay_as_sharps() {
        // Sharps must NOT be folded - G# stays G#, not folded to Ab or anything else
        assert_eq!(extract_key("Loop_C#m_140.wav"), Some("C# Minor".into()));
        assert_eq!(extract_key("Loop_D#m_140.wav"), Some("D# Minor".into()));
        assert_eq!(extract_key("Loop_F#m_140.wav"), Some("F# Minor".into()));
        assert_eq!(extract_key("Loop_G#m_140.wav"), Some("G# Minor".into()));
        assert_eq!(extract_key("Loop_A#m_140.wav"), Some("A# Minor".into()));
    }

    #[test]
    fn key_no_match() {
        assert_eq!(extract_key("Kick_Hard_01.wav"), None);
        assert_eq!(extract_key("Crash_Big.wav"), None);
    }

    // --- Category ---

    #[test]
    fn category_from_filename() {
        let m = match_category("TechKick_Rumble_128_01.wav", "/Samples/Kicks/").unwrap();
        assert_eq!(m.name, "kick");
        assert_eq!(m.confidence, 1.0);

        let m = match_category("Deep_Sub_A_138BPM.wav", "/Samples/Bass/").unwrap();
        assert_eq!(m.name, "sub_bass");

        let m = match_category("Riser_8bar_Up.wav", "/FX/").unwrap();
        assert_eq!(m.name, "fx_riser");
    }

    #[test]
    fn category_from_directory_fallback() {
        let m = match_category("TK_01.wav", "/Samples/Kicks/Techno/").unwrap();
        assert_eq!(m.name, "kick");
        assert_eq!(m.confidence, 0.6); // directory match = lower confidence
    }

    // --- Manufacturer ---

    #[test]
    fn manufacturer_detection() {
        let m = detect_manufacturer("/Users/wizard/Samples/Drumcode/Techno Vol 3/Kicks/").unwrap();
        assert_eq!(m.manufacturer_pattern, "Drumcode");
        assert!(m.genre_score < 0.0);  // techno-leaning

        let m = detect_manufacturer("/Samples/Tidy - Bits & Pieces Vol 1/Leads/").unwrap();
        assert_eq!(m.manufacturer_pattern, "Tidy");
        assert!(m.genre_score > 0.0);  // trance-leaning
        assert!(m.hardness_score > 0.5); // hard
    }

    #[test]
    fn manufacturer_prefers_longer_match() {
        // "Hard Techno" should match over just "Hard" if both existed
        let m = detect_manufacturer("/Definition of Hard Techno/Kicks/").unwrap();
        assert_eq!(m.manufacturer_pattern, "Definition of Hard Techno");
    }

    // --- Pack name ---

    #[test]
    fn pack_name_extraction() {
        assert_eq!(
            extract_pack_name("/Users/wizard/Samples/Producer loops/Tidy - Bits & Pieces Vol 1/Leads/"),
            Some("Tidy - Bits & Pieces Vol 1".into())
        );
        assert_eq!(
            extract_pack_name("/Samples/Loopmasters/Techno Essentials/Kicks/"),
            Some("Techno Essentials".into())
        );
    }

    // --- Full analysis ---

    #[test]
    fn full_analysis() {
        let a = analyze_sample(
            "RY_TRIPTYKH_VOL1_RUMBLE_KICK_LOOP_004_154bpm.wav",
            "/Samples/Definition of Hard Techno/Vol1/",
        );
        assert_eq!(a.parsed_bpm, Some(154));
        assert!(a.is_loop);
        assert!(a.category.is_some());
        assert_eq!(a.category.as_ref().unwrap().name, "kick");
        assert!(a.manufacturer.is_some());
        assert_eq!(a.manufacturer.as_ref().unwrap().manufacturer_pattern, "Definition of Hard Techno");
    }

    // --- Real-world filenames from the DB ---

    #[test]
    fn real_bpm_parsing() {
        assert_eq!(extract_bpm("DPTE2 Techno Loop - 046 - 140 BPM"), Some(140));
        assert_eq!(extract_bpm("Loop 19 (120BPM)"), Some(120));
        assert_eq!(extract_bpm("42 Minimal_Techno_Beat 124BPM Full Layered"), Some(124));
        assert_eq!(extract_bpm("BFM_DT1_DarkTechnoLoop_60_Ebm_125bpm"), Some(125));
        assert_eq!(extract_bpm("RK_MT1_Bass_Loop_19_127bpm_Bmin"), Some(127));
        assert_eq!(extract_bpm("RK_RT3_Background_Loop_04_130bpm"), Some(130));
        assert_eq!(extract_bpm("AOG_RST_SchoolOfWisdom_Drumloop_94bpm_Em_60"), Some(94));
        assert_eq!(extract_bpm("TA_TECHNO_SYNTH_25_125bpm_C"), Some(125));
    }

    #[test]
    fn real_key_parsing() {
        assert_eq!(extract_key("Tidy1 - Lead Loop  06 - PT3 - 140 BPM - G#m"), Some("G# Minor".into()));
        assert_eq!(extract_key("Tidy1 - Bass Loop 02 - PT3 - 140 BPM - Dm"), Some("D Minor".into()));
        assert_eq!(extract_key("PLX_ELT_128_kit_always_synth_Ebmin"), Some("D# Minor".into()));
        assert_eq!(extract_key("GH_EP_Vocal Loop_17_145_C#m_Dry"), Some("C# Minor".into()));
        assert_eq!(extract_key("RK_MT1_Bass_Loop_19_127bpm_Bmin"), Some("B Minor".into()));
        assert_eq!(extract_key("BFM_DT1_DarkTechnoLoop_60_Ebm_125bpm"), Some("D# Minor".into()));
        assert_eq!(extract_key("ZTTP_126_D#_Bass_Loop_1_SC"), Some("D# Minor".into()));
        assert_eq!(extract_key("RK_DT3_Synth_Seq_22_128bpm_Cmin"), Some("C Minor".into()));
        assert_eq!(extract_key("BEEFE_PSY_Trance_Kick6_A#"), Some("A# Minor".into()));
        assert_eq!(extract_key("002_a__Synth_Loop_1_128bpm_G_-_IGNITETECHNO_Zenhiser"), Some("G Minor".into()));
    }

    #[test]
    fn real_category_matching() {
        // Kick from filename
        let m = match_category("Kick Drum Tight 02", "/Drums/").unwrap();
        assert_eq!(m.name, "kick");

        // Clap/snare
        let m = match_category("snare-02", "/Drums/Snares/").unwrap();
        assert_eq!(m.name, "clap"); // snare maps to clap category

        // Hi-hat
        let m = match_category("Tr8 Closed Hat 03", "/Drums/Hats/").unwrap();
        assert_eq!(m.name, "closed_hat");

        // Bass from filename
        let m = match_category("RK_MT1_Bass_Loop_19_127bpm_Bmin", "/Bass/").unwrap();
        assert_eq!(m.name, "mid_bass");

        // Pad
        let m = match_category("Synth Chord Pad Cosmic Fmin", "/Synths/Pads/").unwrap();
        assert_eq!(m.name, "pad");

        // Vocal
        let m = match_category("GH_EP_Vocal Loop_17_145_C#m_Dry", "/Vocals/").unwrap();
        assert_eq!(m.name, "vocal");

        // FX from directory
        let m = match_category("23-Fx Down Filter - ElmntTranceVol2", "/FX/").unwrap();
        assert_eq!(m.name, "fx_misc"); // "Fx" matches fx_misc

        // Perc
        let m = match_category("ElectroHouse-Perc-EH Perc 10", "/Drums/").unwrap();
        assert_eq!(m.name, "perc");

        // Ride
        let m = match_category("Perc-Cymbal-Ride 4 Bell 1", "/Drums/Cymbals/").unwrap();
        assert_eq!(m.name, "ride");
    }

    #[test]
    fn real_manufacturer_from_directory() {
        let m = detect_manufacturer(
            "/Users/wizard/mnt/production/MusicProduction/Samples/Producer loops/Tidy - Bits & Pieces Vol 1 (complete)/Tidy1 - Leads/Loops/Normal Loops"
        ).unwrap();
        assert_eq!(m.manufacturer_pattern, "Tidy");

        let m = detect_manufacturer(
            "/Users/wizard/mnt/production/MusicProduction/Samples/Splice/sounds/packs/Essential Techno/TOOLROOM_-_TOOLROOM_ACADEMY_ESSENTIAL_TECHNO/SYNTH_LOOPS_125BPM"
        ).unwrap();
        assert_eq!(m.manufacturer_pattern, "Toolroom");

        let m = detect_manufacturer(
            "/Users/wizard/mnt/production/MusicProduction/Samples/freshly squeezed/Sunny Lax Kick Essentials Volume 1 - Sample Content/SLKEV1 Trance Kicks"
        );
        // "Freshly Squeezed" should match (case-insensitive)
        assert!(m.is_some());
    }

    #[test]
    fn real_full_pipeline() {
        // Tidy trance lead
        let a = analyze_sample(
            "Tidy1 - Lead Loop  06 - PT3 - 140 BPM - G#m",
            "/Users/wizard/Samples/Producer loops/Tidy - Bits & Pieces Vol 1 (complete)/Tidy1 - Leads/Loops/Normal Loops",
        );
        assert_eq!(a.parsed_bpm, Some(140));
        assert_eq!(a.parsed_key, Some("G# Minor".into()));
        assert!(a.is_loop);
        assert_eq!(a.category.as_ref().unwrap().name, "lead");
        assert_eq!(a.manufacturer.as_ref().unwrap().manufacturer_pattern, "Tidy");
        assert!(a.manufacturer.as_ref().unwrap().genre_score > 0.0); // trance

        // Zenhiser techno synth — Zenhiser is in the filename, Splice in the directory
        let a = analyze_sample(
            "002_a__Synth_Loop_1_128bpm_G_-_IGNITETECHNO_Zenhiser",
            "/Samples/Splice/sounds/packs/Ignite - Techno/loops/synth",
        );
        assert_eq!(a.parsed_bpm, Some(128));
        assert_eq!(a.parsed_key, Some("G Minor".into()));
        assert!(a.is_loop);
        // detect_manufacturer only looks at directory, not filename — Splice is neutral
        assert_eq!(a.manufacturer.as_ref().unwrap().manufacturer_pattern, "Splice");

        // Dark techno bass loop
        let a = analyze_sample(
            "BFM_DT1_DarkTechnoLoop_60_Ebm_125bpm",
            "/Samples/sounds.com/Dark Techno Loops vol.1/LOOPS_125bpm",
        );
        assert_eq!(a.parsed_bpm, Some(125));
        assert_eq!(a.parsed_key, Some("D# Minor".into()));
        assert!(a.is_loop);
    }

    // =========================================================================
    // Splice sample packs - comprehensive BPM and key detection tests
    // =========================================================================

    #[test]
    fn splice_bpm_prefix_format() {
        // Format: PREFIX_PACKCODE_BPM_type_name_KEY.wav
        assert_eq!(extract_bpm("PLX_ACT_140_fx_loop_distract_C.wav"), Some(140));
        assert_eq!(extract_bpm("PLX_ACT_140_fx_loop_top_Eb.wav"), Some(140));
        assert_eq!(extract_bpm("PLX_ACT_140_fx_loop_wow_Amin.wav"), Some(140));
        assert_eq!(extract_bpm("FF_LFT2_127_bass_synth_loop_how_Dmaj.wav"), Some(127));
        assert_eq!(extract_bpm("FF_LFT2_127_drum_loop_exit_percussion.wav"), Some(127));
        assert_eq!(extract_bpm("PLX_NFP_126_atmosphere_reflect_Bmin.wav"), Some(126));
        assert_eq!(extract_bpm("PLX_NFP_128_atmosphere_desert_Bbmin.wav"), Some(128));
        assert_eq!(extract_bpm("PLX_NFP_136_atmosphere_darkstar_Bmin.wav"), Some(136));
        assert_eq!(extract_bpm("PLX_NFP_130_atmosphere_oceans_Cmin.wav"), Some(130));
    }

    #[test]
    fn splice_key_suffix_format() {
        // Bare note at end defaults to minor
        assert_eq!(extract_key("PLX_ACT_140_fx_loop_distract_C.wav"), Some("C Minor".into()));
        assert_eq!(extract_key("PLX_ACT_140_fx_loop_top_Eb.wav"), Some("D# Minor".into()));
        assert_eq!(extract_key("PLX_ACT_140_fx_loop_mod_G.wav"), Some("G Minor".into()));
        assert_eq!(extract_key("PLX_ACT_140_fx_loop_dog_F.wav"), Some("F Minor".into()));
        assert_eq!(extract_key("PLX_ACT_140_fx_loop_moves_E.wav"), Some("E Minor".into()));
        assert_eq!(extract_key("PLX_ACT_140_fx_loop_cold_D.wav"), Some("D Minor".into()));
        
        // Explicit minor suffix
        assert_eq!(extract_key("PLX_ACT_140_fx_loop_wow_Amin.wav"), Some("A Minor".into()));
        assert_eq!(extract_key("PLX_NFP_126_atmosphere_reflect_Bmin.wav"), Some("B Minor".into()));
        assert_eq!(extract_key("PLX_NFP_128_atmosphere_desert_Bbmin.wav"), Some("A# Minor".into()));
        assert_eq!(extract_key("PLX_NFP_128_atmosphere_forgot_C#min.wav"), Some("C# Minor".into()));
        assert_eq!(extract_key("PLX_NFP_130_atmosphere_oceans_Cmin.wav"), Some("C Minor".into()));
        assert_eq!(extract_key("PLX_NFP_136_atmosphere_rediscover_Gmin.wav"), Some("G Minor".into()));
        
        // 7th chord notation (should still extract root as minor)
        assert_eq!(extract_key("PLX_NFP_130_atmosphere_primal_Emin7.wav"), Some("E Minor".into()));
        assert_eq!(extract_key("PLX_NFP_136_atmosphere_follow_Emin7.wav"), Some("E Minor".into()));
        
        // Explicit major
        assert_eq!(extract_key("FF_LFT2_127_bass_synth_loop_how_Dmaj.wav"), Some("D Major".into()));
    }

    #[test]
    fn splice_oneshot_no_bpm() {
        // One-shots typically don't have BPM in filename
        assert_eq!(extract_bpm("PLX_ACT_kick_mid_short.wav"), None);
        assert_eq!(extract_bpm("PLX_ACT_kick_low_heavy.wav"), None);
        assert_eq!(extract_bpm("PLX_ACT_kick_low_techno.wav"), None);
    }

    #[test]
    fn splice_path_detection() {
        // Loop detection from path
        let a = analyze_sample(
            "PLX_ACT_140_fx_loop_distract_C.wav",
            "/Samples/Splice/sounds/packs/Acid Trance/PLX_-_Acid_Trance/loops/fx_loops/",
        );
        assert_eq!(a.parsed_bpm, Some(140));
        assert_eq!(a.parsed_key, Some("C Minor".into()));
        assert!(a.is_loop);

        // One-shot detection from path
        let a = analyze_sample(
            "PLX_ACT_kick_mid_short.wav",
            "/Samples/Splice/sounds/packs/Acid Trance/PLX_-_Acid_Trance/one-shots/drum_one-shots/kick/",
        );
        assert!(!a.is_loop);
    }

    #[test]
    fn splice_various_packs() {
        // Left Field Techno 2
        assert_eq!(extract_bpm("FF_LFT2_127_bass_synth_loop_how_Dmaj.wav"), Some(127));
        assert_eq!(extract_key("FF_LFT2_127_bass_synth_loop_how_Dmaj.wav"), Some("D Major".into()));

        // Nightfall - Future Progressive & Trance  
        assert_eq!(extract_bpm("PLX_NFP_126_atmosphere_reflect_Bmin.wav"), Some(126));
        assert_eq!(extract_key("PLX_NFP_126_atmosphere_reflect_Bmin.wav"), Some("B Minor".into()));
        
        assert_eq!(extract_bpm("PLX_NFP_136_atmosphere_touched_Gmin.wav"), Some(136));
        assert_eq!(extract_key("PLX_NFP_136_atmosphere_touched_Gmin.wav"), Some("G Minor".into()));
    }

    #[test]
    fn bpm_edge_cases() {
        // BPM at start of filename
        assert_eq!(extract_bpm("120_C_HousyBass_SP_01.wav"), Some(120));
        
        // BPM with underscore delimiters
        assert_eq!(extract_bpm("Loop_128_Am.wav"), Some(128));
        assert_eq!(extract_bpm("Bass_140_Fm.wav"), Some(140));
        
        // BPM in middle of complex filename
        assert_eq!(extract_bpm("SYNTH_LOOPS_125BPM"), Some(125));
        
        // Should NOT match version numbers, sample rates, etc.
        assert_eq!(extract_bpm("Sample_v2_44100hz.wav"), None);
        assert_eq!(extract_bpm("Track_01_Final.wav"), None);
        assert_eq!(extract_bpm("Kick_808_03.wav"), None); // 808 is out of range but 03 shouldn't match
    }

    #[test]
    fn key_edge_cases() {
        // Key at end after underscore
        assert_eq!(extract_key("Bass_Loop_01_Am.wav"), Some("A Minor".into()));
        assert_eq!(extract_key("Synth_Pad_F#m.wav"), Some("F# Minor".into()));
        
        // Key with flat notation (should convert to sharp)
        assert_eq!(extract_key("Lead_Bbmin.wav"), Some("A# Minor".into()));
        assert_eq!(extract_key("Pad_Ebm.wav"), Some("D# Minor".into()));
        assert_eq!(extract_key("Bass_Abmin.wav"), Some("G# Minor".into()));
        
        // Key should NOT match inside words
        assert_eq!(extract_key("Crash_Big_01.wav"), None); // "C" inside "Crash"
        assert_eq!(extract_key("Fade_Out.wav"), None);     // "F" and "A" inside words
        assert_eq!(extract_key("Drum_Loop.wav"), None);    // "D" inside "Drum"
    }

    #[test]
    fn combined_bpm_and_key() {
        // Both BPM and key in same filename
        let a = analyze_sample("Lead_128_Am.wav", "/Samples/Leads/");
        assert_eq!(a.parsed_bpm, Some(128));
        assert_eq!(a.parsed_key, Some("A Minor".into()));

        let a = analyze_sample("PLX_NFP_130_atmosphere_oceans_Cmin.wav", "/loops/");
        assert_eq!(a.parsed_bpm, Some(130));
        assert_eq!(a.parsed_key, Some("C Minor".into()));

        let a = analyze_sample("FF_LFT2_127_bass_synth_loop_how_Dmaj.wav", "/loops/");
        assert_eq!(a.parsed_bpm, Some(127));
        assert_eq!(a.parsed_key, Some("D Major".into()));
    }

    // =========================================================================
    // ZTEKNO sample packs - BPM and key detection tests
    // =========================================================================

    #[test]
    fn ztekno_bpm_key_format() {
        // Format: PREFIX_BPM_KEY_Type_Loop_N.wav
        // DRIVING TECHNO pack
        assert_eq!(extract_bpm("ZDT_132_A#_Bass_Loop_1.wav"), Some(132));
        assert_eq!(extract_key("ZDT_132_A#_Bass_Loop_1.wav"), Some("A# Minor".into()));
        
        assert_eq!(extract_bpm("ZDT_132_C#_Bass_Loop_2.wav"), Some(132));
        assert_eq!(extract_key("ZDT_132_C#_Bass_Loop_2.wav"), Some("C# Minor".into()));
        
        assert_eq!(extract_bpm("ZDT_132_D#_Bass_Loop_3.wav"), Some(132));
        assert_eq!(extract_key("ZDT_132_D#_Bass_Loop_3.wav"), Some("D# Minor".into()));
        
        assert_eq!(extract_bpm("ZDT_132_F#_Bass_Loop_1.wav"), Some(132));
        assert_eq!(extract_key("ZDT_132_F#_Bass_Loop_1.wav"), Some("F# Minor".into()));
        
        assert_eq!(extract_bpm("ZDT_132_G#_Bass_Loop_2.wav"), Some(132));
        assert_eq!(extract_key("ZDT_132_G#_Bass_Loop_2.wav"), Some("G# Minor".into()));
        
        // Natural notes
        assert_eq!(extract_bpm("ZDT_132_A_Bass_Loop_1.wav"), Some(132));
        assert_eq!(extract_key("ZDT_132_A_Bass_Loop_1.wav"), Some("A Minor".into()));
        
        assert_eq!(extract_bpm("ZDT_132_E_Bass_Loop_3.wav"), Some(132));
        assert_eq!(extract_key("ZDT_132_E_Bass_Loop_3.wav"), Some("E Minor".into()));
    }

    #[test]
    fn ztekno_techno_freaks_format() {
        // TECHNO FREAKS pack - Format: PREFIX_Kit_N_BPM_Bpm_KEY_Type_Loop.wav
        assert_eq!(extract_bpm("ZTTF_Kit_1_126_Bpm_A_Bass_Loop.wav"), Some(126));
        assert_eq!(extract_key("ZTTF_Kit_1_126_Bpm_A_Bass_Loop.wav"), Some("A Minor".into()));
        
        assert_eq!(extract_bpm("ZTTF_Kit_1_126_Bpm_F#_Melodic_Synth_Loop.wav"), Some(126));
        assert_eq!(extract_key("ZTTF_Kit_1_126_Bpm_F#_Melodic_Synth_Loop.wav"), Some("F# Minor".into()));
        
        assert_eq!(extract_bpm("ZTTF_Kit_2_126_Bpm_C_Bass_Loop.wav"), Some(126));
        assert_eq!(extract_key("ZTTF_Kit_2_126_Bpm_C_Bass_Loop.wav"), Some("C Minor".into()));
        
        assert_eq!(extract_bpm("ZTTF_Kit_4_126_Bpm_D#_Bass_Loop.wav"), Some(126));
        assert_eq!(extract_key("ZTTF_Kit_4_126_Bpm_D#_Bass_Loop.wav"), Some("D# Minor".into()));
        
        assert_eq!(extract_bpm("ZTTF_Kit_5_126_Bpm_F_Acid_Loop.wav"), Some(126));
        assert_eq!(extract_key("ZTTF_Kit_5_126_Bpm_F_Acid_Loop.wav"), Some("F Minor".into()));
    }

    #[test]
    fn ztekno_no_key_samples() {
        // Samples without key info - BPM only
        assert_eq!(extract_bpm("ZTTF_Kit_1_126_Bpm_Kick_Loop.wav"), Some(126));
        assert_eq!(extract_key("ZTTF_Kit_1_126_Bpm_Kick_Loop.wav"), None);
        
        assert_eq!(extract_bpm("ZTTF_Kit_1_126_Bpm_Top_Loop.wav"), Some(126));
        assert_eq!(extract_key("ZTTF_Kit_1_126_Bpm_Top_Loop.wav"), None);
        
        assert_eq!(extract_bpm("ZTTF_Kit_1_126_Bpm_Fx.wav"), Some(126));
        assert_eq!(extract_key("ZTTF_Kit_1_126_Bpm_Fx.wav"), None);
        
        assert_eq!(extract_bpm("ZTTF_Kit_1_126_Bpm_Vox.wav"), Some(126));
        assert_eq!(extract_key("ZTTF_Kit_1_126_Bpm_Vox.wav"), None);
    }

    #[test]
    fn ztekno_oneshot_samples() {
        // One-shot samples - no BPM expected (numbers are variant IDs, not BPM)
        assert_eq!(extract_bpm("ZAT_Clap_1.wav"), None);
        assert_eq!(extract_bpm("ZAT_Clap_22.wav"), None);
        assert_eq!(extract_bpm("ZAT_Clap_140.wav"), None);  // 140 is variant, not BPM
        assert_eq!(extract_bpm("ZAT_Kick_128.wav"), None);  // 128 is variant, not BPM
        assert_eq!(extract_key("ZAT_Clap_1.wav"), None);
        
        // "Kick 23 G.wav" - 23 is variant, G is key
        assert_eq!(extract_bpm("Kick 23 G.wav"), None);     // 23 is variant, not BPM
        assert_eq!(extract_key("Kick 23 G.wav"), Some("G Minor".into()));
        
        // "FL_RNB_Clap.wav" - no BPM, no key (RNB is not a note)
        assert_eq!(extract_bpm("FL_RNB_Clap.wav"), None);
        assert_eq!(extract_key("FL_RNB_Clap.wav"), None);
    }

    #[test]
    fn ztekno_sc_suffix() {
        // Sidechain versions (_SC suffix)
        assert_eq!(extract_bpm("ZDT_132_A#_Bass_Loop_1_SC.wav"), Some(132));
        assert_eq!(extract_key("ZDT_132_A#_Bass_Loop_1_SC.wav"), Some("A# Minor".into()));
        
        assert_eq!(extract_bpm("ZDT_132_C_Bass_Loop_3_SC.wav"), Some(132));
        assert_eq!(extract_key("ZDT_132_C_Bass_Loop_3_SC.wav"), Some("C Minor".into()));
    }

    #[test]
    fn ztekno_full_analysis() {
        // Full pipeline test
        let a = analyze_sample(
            "ZDT_132_A#_Bass_Loop_1.wav",
            "/Samples/ztekno/ZTEKNO - DRIVING TECHNO/ZDT_BASS_LOOPS/",
        );
        assert_eq!(a.parsed_bpm, Some(132));
        assert_eq!(a.parsed_key, Some("A# Minor".into()));
        assert!(a.is_loop);
        
        let a = analyze_sample(
            "ZTTF_Kit_1_126_Bpm_F#_Melodic_Synth_Loop.wav",
            "/Samples/ztekno/ZTEKNO - TECHNO FREAKS/ZTTF_LIVE_KITS/",
        );
        assert_eq!(a.parsed_bpm, Some(126));
        assert_eq!(a.parsed_key, Some("F# Minor".into()));
        assert!(a.is_loop);
    }
}
