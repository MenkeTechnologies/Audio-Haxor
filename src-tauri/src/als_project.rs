//! ALS project generation pipeline.
//!
//! Takes wizard inputs (genre, hardness, BPM, key, keywords, track counts)
//! and produces a complete Ableton Live Set file by:
//! 1. Querying sample_analysis tables for ranked samples per category
//! 2. Building an arrangement (element entry/exit per section)
//! 3. Generating ALS XML via the existing als_generator infrastructure

use crate::db;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Genre & configuration types
// ---------------------------------------------------------------------------

/// Genre enum matching the spec: Techno, Schranz, Trance.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Genre {
    Techno,
    Schranz,
    Trance,
}

impl Genre {
    pub fn default_bpm(self) -> u32 {
        match self {
            Genre::Techno => 132,
            Genre::Schranz => 155,
            Genre::Trance => 140,
        }
    }

    pub fn bpm_range(self) -> (u32, u32) {
        match self {
            Genre::Techno => (120, 140),
            Genre::Schranz => (145, 165),
            Genre::Trance => (130, 160),
        }
    }
}

/// Per-element track configuration from the wizard.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElementConfig {
    pub count: u32,
    /// 0.0 = clean/smooth/subtle, 1.0 = distorted/aggressive/intense
    pub character: f32,
}

/// Track count configuration from the wizard.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackConfig {
    pub drums: ElementConfig,
    pub bass: ElementConfig,
    pub leads: ElementConfig,
    pub pads: ElementConfig,
    pub fx: ElementConfig,
    pub vocals: ElementConfig,
}

impl Default for TrackConfig {
    fn default() -> Self {
        Self {
            drums: ElementConfig { count: 3, character: 0.5 },
            bass: ElementConfig { count: 2, character: 0.5 },
            leads: ElementConfig { count: 2, character: 0.5 },
            pads: ElementConfig { count: 2, character: 0.5 },
            fx: ElementConfig { count: 6, character: 0.5 },
            vocals: ElementConfig { count: 0, character: 0.5 },
        }
    }
}

/// Full project configuration from the wizard.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    pub genre: Genre,
    /// 0.0 = regular, 1.0 = hard
    pub hardness: f32,
    /// 0.0 = predictable, 1.0 = chaotic (controls randomized gaps and call-and-response)
    #[serde(default = "default_chaos")]
    pub chaos: f32,
    /// 0.0 = clean, 1.0 = heavily glitched (micro-edits, stutters, beat dropouts)
    #[serde(default)]
    pub glitch_intensity: f32,
    /// Per-section overrides for all 5 dynamics params (chaos, glitch, density, variation, parallelism).
    /// Each per-section value is `Option<f32>` — `None` falls back to the global scalar above.
    /// Replaces the legacy single-param `section_glitch: SectionGlitchConfig` (removed).
    #[serde(default)]
    pub section_overrides: SectionOverridesConfig,
    /// 0.0 = none, 1.0 = dense scattered one-shot hits on 1/16 grid
    #[serde(default)]
    pub density: f32,
    /// 0.0 = static (elements play full sections), 1.0 = dynamic (elements constantly in/out)
    #[serde(default)]
    pub variation: f32,
    /// 0.0 = one track at a time per group, 1.0 = all tracks play together
    #[serde(default = "default_parallelism")]
    pub parallelism: f32,
    /// 0.0 = no scatter hits, 1.0 = dense scattered one-shot hits on 1/16 grid
    #[serde(default)]
    pub scatter: f32,
    pub bpm: u32,
    /// e.g. "A" — root note
    pub root_note: Option<String>,
    /// e.g. "Aeolian" — mode
    pub mode: Option<String>,
    pub atonal: bool,
    pub keywords: Vec<String>,
    pub element_keywords: std::collections::HashMap<String, String>,
    /// Optional base path to filter samples - only use samples under this directory
    #[serde(default)]
    pub sample_source_path: Option<String>,
    /// Legacy category-based track counts (kept for backwards compat)
    pub tracks: TrackConfig,
    pub output_path: String,
    pub project_name: Option<String>,
    /// Number of songs to generate in one ALS file (1-10)
    pub num_songs: u32,
    /// Per-type atonal toggles (overrides global atonal for specific types)
    #[serde(default)]
    pub type_atonal: TypeAtonalConfig,
    /// Per-type track counts (new - takes precedence over category-based tracks)
    #[serde(default)]
    pub track_counts: TrackCountsConfig,
}

/// Per-type track counts from frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackCountsConfig {
    #[serde(default = "default_1")] pub kick: u32,
    #[serde(default = "default_1")] pub clap: u32,
    #[serde(default = "default_1")] pub snare: u32,
    #[serde(default = "default_2")] pub hat: u32,
    #[serde(default = "default_2")] pub perc: u32,
    #[serde(default = "default_1")] pub ride: u32,
    #[serde(default = "default_4")] pub fill: u32,
    #[serde(default = "default_1")] pub bass: u32,
    #[serde(default = "default_1")] pub sub: u32,
    #[serde(default = "default_1")] pub lead: u32,
    #[serde(default = "default_3")] pub synth: u32,
    #[serde(default = "default_2")] pub pad: u32,
    #[serde(default = "default_2")] pub arp: u32,
    #[serde(default = "default_3")] pub riser: u32,
    #[serde(default = "default_1")] pub downlifter: u32,
    #[serde(default = "default_2")] pub crash: u32,
    #[serde(default = "default_2")] pub impact: u32,
    #[serde(default = "default_2")] pub hit: u32,
    #[serde(default = "default_4")] pub sweep_up: u32,
    #[serde(default = "default_4")] pub sweep_down: u32,
    #[serde(default = "default_1")] pub snare_roll: u32,
    #[serde(default = "default_2")] pub reverse: u32,
    #[serde(default = "default_2")] pub sub_drop: u32,
    #[serde(default = "default_2")] pub boom_kick: u32,
    #[serde(default = "default_2")] pub atmos: u32,
    #[serde(default = "default_2")] pub glitch: u32,
    #[serde(default = "default_4")] pub scatter: u32,
    #[serde(default = "default_1")] pub vox: u32,
}

fn default_1() -> u32 { 1 }
fn default_2() -> u32 { 2 }
fn default_3() -> u32 { 3 }
fn default_4() -> u32 { 4 }
fn default_chaos() -> f32 { 0.3 }
fn default_parallelism() -> f32 { 0.4 }

/// Per-section values for a single dynamics parameter.
/// Each value is 0.0-1.0, with None meaning "use the global scalar for this param".
/// Bar ranges are genre-specific (see `get_sections_for_genre`); the 7 names are fixed.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SectionValues {
    #[serde(default)]
    pub intro: Option<f32>,
    #[serde(default)]
    pub build: Option<f32>,
    #[serde(default)]
    pub breakdown: Option<f32>,
    #[serde(default)]
    pub drop1: Option<f32>,
    #[serde(default)]
    pub drop2: Option<f32>,
    #[serde(default)]
    pub fadedown: Option<f32>,
    #[serde(default)]
    pub outro: Option<f32>,
}

impl SectionValues {
    /// Any override set? (used to skip resolver work when the param has no overrides)
    pub fn any(&self) -> bool {
        self.intro.is_some()
            || self.build.is_some()
            || self.breakdown.is_some()
            || self.drop1.is_some()
            || self.drop2.is_some()
            || self.fadedown.is_some()
            || self.outro.is_some()
    }
}

/// Per-section overrides for all 6 dynamics params. Drives the ALS Generator timeline editor.
/// Replaces legacy `SectionGlitchConfig` (which only covered glitch). An empty config means
/// "use the global scalar for everything" — the same behavior as before for users who don't
/// touch the timeline.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SectionOverridesConfig {
    #[serde(default)]
    pub chaos: SectionValues,
    #[serde(default)]
    pub glitch: SectionValues,
    #[serde(default)]
    pub density: SectionValues,
    #[serde(default)]
    pub variation: SectionValues,
    #[serde(default)]
    pub parallelism: SectionValues,
    #[serde(default)]
    pub scatter: SectionValues,
}

impl Default for TrackCountsConfig {
    fn default() -> Self {
        Self {
            kick: 1, clap: 1, snare: 1, hat: 2, perc: 2, ride: 1, fill: 4,
            bass: 1, sub: 1,
            lead: 1, synth: 3, pad: 2, arp: 2,
            riser: 3, downlifter: 1, crash: 2, impact: 2, hit: 2, sweep_up: 4, sweep_down: 4, snare_roll: 1, reverse: 2, sub_drop: 2, boom_kick: 2, atmos: 2, glitch: 2, scatter: 4,
            vox: 1,
        }
    }
}

/// Per-type atonal configuration from frontend
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TypeAtonalConfig {
    #[serde(default)] pub kick: bool,
    #[serde(default)] pub clap: bool,
    #[serde(default)] pub snare: bool,
    #[serde(default)] pub hat: bool,
    #[serde(default)] pub perc: bool,
    #[serde(default)] pub ride: bool,
    #[serde(default)] pub fill: bool,
    #[serde(default)] pub bass: bool,
    #[serde(default)] pub sub: bool,
    #[serde(default)] pub lead: bool,
    #[serde(default)] pub synth: bool,
    #[serde(default)] pub pad: bool,
    #[serde(default)] pub arp: bool,
    #[serde(default)] pub riser: bool,
    #[serde(default)] pub downlifter: bool,
    #[serde(default)] pub crash: bool,
    #[serde(default)] pub impact: bool,
    #[serde(default)] pub hit: bool,
    #[serde(default)] pub sweep_up: bool,
    #[serde(default)] pub sweep_down: bool,
    #[serde(default)] pub snare_roll: bool,
    #[serde(default)] pub reverse: bool,
    #[serde(default)] pub sub_drop: bool,
    #[serde(default)] pub boom_kick: bool,
    #[serde(default)] pub atmos: bool,
    #[serde(default)] pub glitch: bool,
    #[serde(default)] pub scatter: bool,
    #[serde(default)] pub vox: bool,
}

// ---------------------------------------------------------------------------
// Key / mode utilities
// ---------------------------------------------------------------------------

const NOTES: &[&str] = &["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];

/// Semitone offsets from mode root to relative major root.
const MODE_TO_RELATIVE_MAJOR: &[(&str, i32)] = &[
    ("Ionian", 0),
    ("Dorian", -2),
    ("Phrygian", -4),
    ("Lydian", -5),
    ("Mixolydian", -7),
    ("Aeolian", -9),
    ("Locrian", -11),
];

/// Convert root+mode to compatible keys for sample query.
/// Returns both relative major and relative minor (same notes).
pub fn get_compatible_keys(root: &str, mode: &str) -> Vec<String> {
    let root_idx = match NOTES.iter().position(|&n| n.eq_ignore_ascii_case(root)) {
        Some(i) => i as i32,
        None => return vec![format!("{} Minor", root)], // fallback
    };

    let offset = MODE_TO_RELATIVE_MAJOR
        .iter()
        .find(|(m, _)| m.eq_ignore_ascii_case(mode))
        .map(|(_, o)| *o)
        .unwrap_or(-9); // default to Aeolian

    let major_idx = ((root_idx + offset) % 12 + 12) % 12;
    let minor_idx = (major_idx + 9) % 12;

    vec![
        format!("{} Major", NOTES[major_idx as usize]),
        format!("{} Minor", NOTES[minor_idx as usize]),
    ]
}

// ---------------------------------------------------------------------------
// Sample query engine
// ---------------------------------------------------------------------------

/// A sample selected for the arrangement.
#[derive(Debug, Clone, Serialize)]
pub struct SelectedSample {
    pub sample_id: i64,
    pub path: String,
    pub name: String,
    pub duration: f64,
    pub size: i64,
    pub parsed_bpm: Option<u32>,
    pub parsed_key: Option<String>,
    pub category: Option<String>,
    pub is_loop: bool,
}

/// Query the sample_analysis tables for samples matching the given criteria.
/// Returns ranked results using multi-factor scoring.
/// Query samples with a fallback chain:
/// 1. Try sample_analysis tables (fast indexed queries)
/// 2. Fallback to direct audio_samples query (works before analysis job runs)
///
/// Key enforcement:
/// - Atonal ON → skip key matching
/// - Atonal OFF + key-sensitive category → REQUIRE key match (hard WHERE filter)
/// - Non-key-sensitive category → ignore key
///
/// Fallback for key: strict key match → compatible keys → any key (with warning)
pub fn query_samples(
    category: &str,
    config: &ProjectConfig,
    require_loop: bool,
    limit: u32,
) -> Result<Vec<SelectedSample>, String> {
    // Try analyzed path first
    let results = query_samples_analyzed(category, config, require_loop, limit);
    if let Ok(ref r) = results
        && !r.is_empty() {
            return results;
        }

    // Fallback: direct query against audio_samples (before analysis runs)
    query_samples_direct(category, config, require_loop, limit)
}

/// Query via sample_analysis tables (requires analysis job to have run).
fn query_samples_analyzed(
    category: &str,
    config: &ProjectConfig,
    require_loop: bool,
    limit: u32,
) -> Result<Vec<SelectedSample>, String> {
    let target_keys = build_target_keys(config);
    let key_sensitive = is_key_sensitive(category);

    let bpm_lo = config.bpm.saturating_sub(5);
    let bpm_hi = config.bpm + 5;

    let genre_score_direction = match config.genre {
        Genre::Techno | Genre::Schranz => "ASC",
        Genre::Trance => "DESC",
    };
    let hardness_direction = if config.hardness >= 0.5 { "DESC" } else { "ASC" };

    // Key filter: hard WHERE when not atonal AND key-sensitive
    let key_where = if key_sensitive && !target_keys.is_empty() {
        let key_list: String = target_keys
            .iter()
            .map(|k| format!("'{}'", k.replace('\'', "''")))
            .collect::<Vec<_>>()
            .join(", ");
        // Strict: only samples WITH a detected key that matches
        format!("AND a.parsed_key IN ({})", key_list)
    } else {
        String::new()
    };

    let loop_clause = if require_loop { "AND a.is_loop = 1" } else { "" };
    let bpm_clause = if require_loop {
        format!("AND (a.parsed_bpm IS NULL OR a.parsed_bpm BETWEEN {} AND {})", bpm_lo, bpm_hi)
    } else {
        String::new()
    };

    let query = format!(
        "SELECT s.id, s.path, s.name, COALESCE(s.duration, 0.0), COALESCE(s.size, 0),
                a.parsed_bpm, a.parsed_key, c.name AS cat_name, a.is_loop
         FROM audio_samples s
         JOIN sample_analysis a ON s.id = a.sample_id
         LEFT JOIN sample_categories c ON a.category_id = c.id
         LEFT JOIN sample_pack_manufacturers m ON a.manufacturer_id = m.id
         WHERE s.format = 'WAV'
           AND s.id IN (SELECT sample_id FROM audio_library)
           AND a.category_id = (SELECT id FROM sample_categories WHERE name = '{category}')
           {loop_clause}
           {bpm_clause}
           {key_where}
         ORDER BY
           COALESCE(m.genre_score, 0) {genre_score_direction},
           COALESCE(m.hardness_score, 0) {hardness_direction},
           a.category_confidence DESC,
           RANDOM()
         LIMIT {limit}",
        category = category.replace('\'', "''"),
    );

    let results = db::global().query_samples_for_als(&query)?;

    // If strict key match returned nothing for a key-sensitive category,
    // return empty rather than wrong-key samples that clash harmonically.
    Ok(results)
}

/// Fallback: direct query against audio_samples when sample_analysis hasn't been populated.
/// Uses audio_samples.key_name (from audio key detection) and filename pattern matching.
fn query_samples_direct(
    category: &str,
    config: &ProjectConfig,
    require_loop: bool,
    limit: u32,
) -> Result<Vec<SelectedSample>, String> {
    let target_keys = build_target_keys(config);
    let key_sensitive = is_key_sensitive(category);

    let bpm_lo = config.bpm.saturating_sub(5);
    let bpm_hi = config.bpm + 5;

    // Category → filename patterns for direct matching
    let name_pattern = category_to_like_pattern(category);

    // Genre keyword patterns for scoring
    let genre_keywords: &[&str] = match config.genre {
        Genre::Techno => &["techno", "tech", "warehouse", "berlin", "underground", "minimal", "industrial"],
        Genre::Schranz => &["schranz", "hardtechno", "hard techno", "industrial", "distorted", "aggressive", "rave"],
        Genre::Trance => &["trance", "uplifting", "progressive", "euphoric", "psy", "melodic", "epic"],
    };

    // Key filter using audio_samples.key_name
    let key_where = if key_sensitive && !target_keys.is_empty() {
        let key_list: String = target_keys
            .iter()
            .map(|k| format!("'{}'", k.replace('\'', "''")))
            .collect::<Vec<_>>()
            .join(", ");
        format!("AND s.key_name IN ({})", key_list)
    } else {
        String::new()
    };

    let loop_clause = if require_loop {
        "AND LOWER(s.name) LIKE '%loop%'"
    } else {
        ""
    };

    let bpm_clause = if require_loop {
        format!("AND (s.bpm IS NULL OR s.bpm BETWEEN {} AND {})", bpm_lo, bpm_hi)
    } else {
        String::new()
    };

    // Genre scoring in ORDER BY (cloned because used in fallback query too)
    let genre_score: String = genre_keywords
        .iter()
        .map(|kw| format!("(CASE WHEN LOWER(s.path) LIKE '%{}%' THEN 1 ELSE 0 END)", kw))
        .collect::<Vec<_>>()
        .join(" + ");

    let query = format!(
        "SELECT s.id, s.path, s.name, COALESCE(s.duration, 0.0), COALESCE(s.size, 0),
                CAST(s.bpm AS INTEGER), s.key_name, NULL AS cat_name,
                CASE WHEN LOWER(s.name) LIKE '%loop%' THEN 1 ELSE 0 END AS is_loop
         FROM audio_samples s
         WHERE s.format = 'WAV'
           AND s.id IN (SELECT sample_id FROM audio_library)
           AND ({name_pattern})
           {loop_clause}
           {bpm_clause}
           {key_where}
         ORDER BY
           ({genre_score}) DESC,
           RANDOM()
         LIMIT {limit}",
        name_pattern = name_pattern,
        genre_score = if genre_score.is_empty() { "0" } else { &genre_score },
    );

    let results = db::global().query_samples_for_als(&query)?;

    // If strict key match returned nothing for a key-sensitive category,
    // return empty rather than wrong-key samples that clash harmonically.
    Ok(results)
}

// ---------------------------------------------------------------------------
// Query helpers
// ---------------------------------------------------------------------------

fn build_target_keys(config: &ProjectConfig) -> Vec<String> {
    if config.atonal {
        vec![]
    } else if let (Some(root), Some(mode)) = (&config.root_note, &config.mode) {
        get_compatible_keys(root, mode)
    } else {
        vec![]
    }
}

fn is_key_sensitive(category: &str) -> bool {
    matches!(
        category,
        "sub_bass" | "mid_bass" | "lead" | "pad" | "arp" | "pluck" | "stab" | "acid"
            | "atmos" | "vocal" | "vocal_phrase" | "schranz_drive"
    )
}

/// Convert a category name to SQL LIKE patterns for direct filename matching.
fn category_to_like_pattern(category: &str) -> String {
    let patterns: &[&str] = match category {
        "kick" => &["kick", "kik", "bd"],
        "clap" => &["clap", "snare", "snr"],
        "closed_hat" => &["closed hat", "closed_hat", "chh"],
        "open_hat" => &["open hat", "open_hat", "ohh"],
        "ride" => &["ride"],
        "perc" => &["perc", "shaker", "tambourine", "conga", "bongo"],
        "sub_bass" => &["sub", "808"],
        "mid_bass" => &["bass"],
        "lead" => &["lead", "synth lead"],
        "pad" => &["pad", "string", "chord"],
        "arp" => &["arp", "sequence"],
        "pluck" => &["pluck", "pizz"],
        "stab" => &["stab", "brass"],
        "acid" => &["acid", "303"],
        "atmos" => &["atmos", "ambient", "drone", "soundscape"],
        "noise" => &["noise", "texture", "static"],
        "fx_riser" => &["riser", "rise", "sweep up", "uplifter", "build"],
        "fx_downer" => &["downer", "downlifter", "sweep down", "fall"],
        "fx_crash" => &["crash", "cymbal"],
        "fx_impact" => &["impact", "boom", "hit"],
        "fx_fill" => &["fill", "roll", "snare roll"],
        "fx_reverse" => &["reverse", "rev"],
        "fx_sub_drop" => &["sub drop", "bass drop"],
        "fx_misc" => &["fx", "sfx", "effect"],
        "vocal" => &["vox", "vocal", "voice"],
        "vocal_chop" => &["vocal chop", "vox chop"],
        "schranz_drive" => &["drive", "rumble"],
        "schranz_kick" => &["schranz kick", "hard techno kick"],
        _ => &["fx"],
    };
    patterns
        .iter()
        .flat_map(|p| {
            vec![
                format!("LOWER(s.name) LIKE '%{}%'", p),
                format!("LOWER(s.directory) LIKE '%{}%'", p),
            ]
        })
        .collect::<Vec<_>>()
        .join(" OR ")
}

// ---------------------------------------------------------------------------
// Arrangement engine
// ---------------------------------------------------------------------------

/// A track's arrangement: when it plays within the song.
#[derive(Debug, Clone)]
pub struct TrackArrangement {
    pub name: String,
    pub category: String,
    pub group: String,
    pub color: u32,
    /// (start_bar, end_bar) pairs — 1-indexed, fractional for beat precision
    pub sections: Vec<(f64, f64)>,
    pub require_loop: bool,
    pub key_sensitive: bool,
}

// Ableton color palette indices
const DRUMS_COLOR: u32 = 69;  // Orange
const BASS_COLOR: u32 = 13;   // Blue
const LEADS_COLOR: u32 = 26;  // Purple
const PADS_COLOR: u32 = 17;   // Yellow
const FX_COLOR: u32 = 57;     // Cyan
const VOX_COLOR: u32 = 4;     // Pink
const ATMOS_COLOR: u32 = 41;  // Gray

/// Section boundaries for a 224-bar arrangement (7 sections × 32 bars).
#[derive(Debug, Clone, Copy)]
pub struct SectionBounds {
    pub intro: (u32, u32),
    pub build: (u32, u32),
    pub breakdown: (u32, u32),
    pub drop1: (u32, u32),
    pub drop2: (u32, u32),
    pub fadedown: (u32, u32),
    pub outro: (u32, u32),
    pub total_bars: u32,
}

impl SectionBounds {
    /// Standard 224-bar arrangement: 7 × 32 bars.
    pub fn standard() -> Self {
        Self {
            intro: (1, 32),
            build: (33, 64),
            breakdown: (65, 96),
            drop1: (97, 128),
            drop2: (129, 160),
            fadedown: (161, 192),
            outro: (193, 224),
            total_bars: 224,
        }
    }

    /// Trance arrangement: longer breakdowns (256 bars).
    pub fn trance() -> Self {
        Self {
            intro: (1, 32),
            build: (33, 64),
            breakdown: (65, 112),  // 48 bars (longer emotional section)
            drop1: (113, 144),
            drop2: (145, 176),
            fadedown: (177, 208),
            outro: (209, 256),     // 48 bars (longer DJ-friendly outro)
            total_bars: 256,
        }
    }

    /// Schranz arrangement: minimal breakdowns (208 bars).
    pub fn schranz() -> Self {
        Self {
            intro: (1, 32),
            build: (33, 64),
            breakdown: (65, 80),   // 16 bars (brief, not emotional)
            drop1: (81, 112),
            drop2: (113, 160),     // 48 bars (extended peak)
            fadedown: (161, 192),
            outro: (193, 208),     // 16 bars (short exit)
            total_bars: 208,
        }
    }

    pub fn for_genre(genre: Genre) -> Self {
        match genre {
            Genre::Techno => Self::standard(),
            Genre::Trance => Self::trance(),
            Genre::Schranz => Self::schranz(),
        }
    }
}

/// Generate fill gap pattern: varied gap lengths at 8-bar phrase boundaries.
/// Returns sections with gaps for fills (1-beat, 2-beat, 4-beat gaps rotating).
fn sections_with_fill_gaps(start: u32, end: u32) -> Vec<(f64, f64)> {
    let mut sections = Vec::new();
    let mut bar = start;
    let mut gap_idx = 0u32;

    while bar < end {
        let phrase_end = (bar + 7).min(end);
        // Rotate gap sizes: 1-beat (0.25), 2-beat (0.5), 4-beat (1.0)
        let gap = match gap_idx % 3 {
            0 => 0.25,  // 1 beat
            1 => 0.5,   // 2 beats
            _ => 1.0,   // 4 beats (full bar)
        };
        let section_end = phrase_end as f64 + 1.0 - gap;
        sections.push((bar as f64, section_end));
        bar = phrase_end + 1;
        gap_idx += 1;
    }
    sections
}

/// Generate sections without gaps (for FX, atmos — they play through transitions).
fn sections_continuous(start: u32, end: u32) -> Vec<(f64, f64)> {
    vec![(start as f64, (end + 1) as f64)]
}

/// Build the full arrangement for a given genre and track configuration.
pub fn build_arrangement(config: &ProjectConfig) -> Vec<TrackArrangement> {
    let s = SectionBounds::for_genre(config.genre);
    let mut tracks: Vec<TrackArrangement> = Vec::new();

    // === DRUMS ===
    let drum_dist = distribute_drums(config.tracks.drums.count);

    // KICK: intro through outro, out during breakdown
    if drum_dist.kicks >= 1 {
        let mut kick_sections = sections_with_fill_gaps(s.intro.0, s.build.1);
        kick_sections.extend(sections_with_fill_gaps(s.drop1.0, s.fadedown.1));
        kick_sections.extend(sections_with_fill_gaps(s.outro.0, s.outro.1));
        tracks.push(TrackArrangement {
            name: "Kick".into(), category: "kick".into(), group: "Drums".into(),
            color: DRUMS_COLOR, sections: kick_sections, require_loop: true, key_sensitive: false,
        });
    }

    // CLAP: enters bar 9
    if drum_dist.claps >= 1 {
        let mut clap_sections = sections_with_fill_gaps(s.intro.0 + 8, s.build.1);
        clap_sections.extend(sections_with_fill_gaps(s.drop1.0, s.fadedown.1));
        tracks.push(TrackArrangement {
            name: "Clap".into(), category: "clap".into(), group: "Drums".into(),
            color: DRUMS_COLOR, sections: clap_sections, require_loop: true, key_sensitive: false,
        });
    }

    // CLOSED HAT: enters bar 17
    if drum_dist.hats >= 1 {
        let mut hat_sections = sections_with_fill_gaps(s.intro.0 + 16, s.build.1);
        hat_sections.extend(sections_with_fill_gaps(s.drop1.0, s.fadedown.0 + 16));
        tracks.push(TrackArrangement {
            name: "Hat".into(), category: "closed_hat".into(), group: "Drums".into(),
            color: DRUMS_COLOR, sections: hat_sections, require_loop: true, key_sensitive: false,
        });
    }

    // OPEN HAT: drops only
    if drum_dist.hats >= 2 {
        let hat2_sections = sections_with_fill_gaps(s.drop1.0, s.fadedown.0 + 8);
        tracks.push(TrackArrangement {
            name: "Hat 2".into(), category: "open_hat".into(), group: "Drums".into(),
            color: DRUMS_COLOR, sections: hat2_sections, require_loop: true, key_sensitive: false,
        });
    }

    // RIDE: build through fadedown
    if drum_dist.rides >= 1 {
        let mut ride_sections = sections_with_fill_gaps(s.build.0, s.build.1);
        ride_sections.extend(sections_with_fill_gaps(s.drop1.0, s.fadedown.0 + 8));
        tracks.push(TrackArrangement {
            name: "Ride".into(), category: "ride".into(), group: "Drums".into(),
            color: DRUMS_COLOR, sections: ride_sections, require_loop: true, key_sensitive: false,
        });
    }

    // PERC: enters bar 25
    if drum_dist.percs >= 1 {
        let mut perc_sections = sections_with_fill_gaps(s.intro.0 + 24, s.build.1);
        perc_sections.extend(sections_with_fill_gaps(s.drop1.0, s.fadedown.0 + 16));
        tracks.push(TrackArrangement {
            name: "Perc".into(), category: "perc".into(), group: "Drums".into(),
            color: DRUMS_COLOR, sections: perc_sections, require_loop: true, key_sensitive: false,
        });
    }

    // PERC 2: drops only
    if drum_dist.percs >= 2 {
        let perc2_sections = sections_with_fill_gaps(s.drop1.0 + 16, s.fadedown.0 + 8);
        tracks.push(TrackArrangement {
            name: "Perc 2".into(), category: "perc".into(), group: "Drums".into(),
            color: DRUMS_COLOR, sections: perc2_sections, require_loop: true, key_sensitive: false,
        });
    }

    // === BASS ===
    let bass_dist = distribute_bass(config.tracks.bass.count);

    if bass_dist.sub >= 1 {
        let mut sub_sections = sections_with_fill_gaps(s.drop1.0, s.drop2.1);
        if config.genre != Genre::Schranz {
            // Schranz sub drops earlier
            sub_sections.extend(sections_with_fill_gaps(s.fadedown.0, s.fadedown.0 + 8));
        }
        tracks.push(TrackArrangement {
            name: "Sub".into(), category: "sub_bass".into(), group: "Bass".into(),
            color: BASS_COLOR, sections: sub_sections, require_loop: true, key_sensitive: true,
        });
    }

    if bass_dist.mid >= 1 {
        let mut bass_sections = sections_with_fill_gaps(s.build.0, s.build.1);
        bass_sections.extend(sections_with_fill_gaps(s.drop1.0, s.fadedown.1));
        bass_sections.extend(sections_with_fill_gaps(s.outro.0, s.outro.0 + 8));
        tracks.push(TrackArrangement {
            name: "Bass".into(), category: "mid_bass".into(), group: "Bass".into(),
            color: BASS_COLOR, sections: bass_sections, require_loop: true, key_sensitive: true,
        });
    }

    // Schranz-specific: drive tracks
    if config.genre == Genre::Schranz && bass_dist.mid >= 2 {
        let drive_sections = sections_with_fill_gaps(s.build.0, s.fadedown.1);
        tracks.push(TrackArrangement {
            name: "Drive".into(), category: "schranz_drive".into(), group: "Bass".into(),
            color: BASS_COLOR, sections: drive_sections, require_loop: true, key_sensitive: false,
        });
    }

    // === LEADS / MELODICS ===
    for i in 0..config.tracks.leads.count {
        let (name, cat) = match i {
            0 => ("Main Synth".to_string(), "lead"),
            1 => ("Synth 1".to_string(), "lead"),
            2 => ("Stab".to_string(), "stab"),
            3 => ("Acid".to_string(), "acid"),
            4 => ("Arp".to_string(), "arp"),
            _ => (format!("Synth {}", i), "lead"),
        };

        // Main synth: breakdown through drops
        // Others: progressively later entry
        let entry_bar = match i {
            0 => s.breakdown.0 + 16, // mid-breakdown
            1 => s.build.0 + 8,
            2 => s.drop1.0 + 8,
            3 => s.drop1.0 + 16,
            _ => s.drop2.0,
        };

        let exit_bar = match i {
            0 => s.fadedown.1,
            1 => s.fadedown.0 + 8,
            _ => s.fadedown.0,
        };

        if entry_bar < exit_bar {
            let sections = sections_with_fill_gaps(entry_bar, exit_bar);
            tracks.push(TrackArrangement {
                name, category: cat.into(), group: "Leads".into(),
                color: LEADS_COLOR, sections, require_loop: true, key_sensitive: true,
            });
        }
    }

    // === PADS ===
    for i in 0..config.tracks.pads.count {
        let name = if i == 0 { "Pad".to_string() } else { format!("Pad {}", i + 1) };
        let entry_bar = match i {
            0 => s.build.0 + 16,
            _ => s.breakdown.0,
        };
        let exit_bar = match i {
            0 => s.fadedown.0 + 8,
            _ => s.breakdown.1,
        };

        if entry_bar < exit_bar {
            let mut sections = sections_with_fill_gaps(entry_bar, s.build.1.min(exit_bar));
            // Pads also play through breakdown (continuous, no gaps)
            if s.breakdown.0 >= entry_bar {
                sections.extend(sections_continuous(s.breakdown.0, s.breakdown.1.min(exit_bar)));
            }
            if s.drop1.0 < exit_bar {
                sections.extend(sections_with_fill_gaps(
                    s.drop2.0.max(entry_bar),
                    exit_bar.min(s.fadedown.0 + 8),
                ));
            }
            tracks.push(TrackArrangement {
                name, category: "pad".into(), group: "Pads".into(),
                color: PADS_COLOR, sections, require_loop: true, key_sensitive: true,
            });
        }
    }

    // === FX ===
    let fx_dist = distribute_fx(config.tracks.fx.count);

    // Crashes: every 8 bars
    if fx_dist.crashes >= 1 {
        let mut crash_sections = Vec::new();
        let mut bar = s.intro.0;
        while bar <= s.outro.1 {
            crash_sections.push((bar as f64, bar as f64));
            bar += 8;
        }
        tracks.push(TrackArrangement {
            name: "Crash".into(), category: "fx_crash".into(), group: "FX".into(),
            color: FX_COLOR, sections: crash_sections, require_loop: false, key_sensitive: false,
        });
    }

    // Risers: 8 bars before each major transition
    if fx_dist.risers >= 1 {
        let riser_sections = vec![
            (s.build.1.saturating_sub(7) as f64, (s.breakdown.0 + 1) as f64),
            (s.breakdown.1.saturating_sub(7) as f64, (s.drop1.0 + 1) as f64), // THE big one
            (s.drop1.1.saturating_sub(7) as f64, (s.drop2.0 + 1) as f64),
            (s.drop2.1.saturating_sub(7) as f64, (s.fadedown.0 + 1) as f64),
        ];
        tracks.push(TrackArrangement {
            name: "Riser 1".into(), category: "fx_riser".into(), group: "FX".into(),
            color: FX_COLOR, sections: riser_sections, require_loop: false, key_sensitive: false,
        });
    }
    if fx_dist.risers >= 2 {
        let riser2_sections = vec![
            ((s.intro.0 + 8) as f64, (s.intro.0 + 17) as f64),
            ((s.build.0 + 8) as f64, (s.build.0 + 17) as f64),
            (s.breakdown.1.saturating_sub(7) as f64, (s.drop1.0 + 1) as f64), // layer
            ((s.drop2.0 + 8) as f64, (s.drop2.0 + 17) as f64),
        ];
        tracks.push(TrackArrangement {
            name: "Riser 2".into(), category: "fx_riser".into(), group: "FX".into(),
            color: FX_COLOR, sections: riser2_sections, require_loop: false, key_sensitive: false,
        });
    }

    // Impacts: on beat 1 of major sections
    if fx_dist.impacts >= 1 {
        let impact_sections = vec![
            (s.intro.0 as f64, s.intro.0 as f64),
            (s.build.0 as f64, s.build.0 as f64),
            (s.breakdown.0 as f64, s.breakdown.0 as f64),
            (s.drop1.0 as f64, s.drop1.0 as f64),
            (s.drop2.0 as f64, s.drop2.0 as f64),
            (s.fadedown.0 as f64, s.fadedown.0 as f64),
            (s.outro.0 as f64, s.outro.0 as f64),
        ];
        tracks.push(TrackArrangement {
            name: "Impact".into(), category: "fx_impact".into(), group: "FX".into(),
            color: FX_COLOR, sections: impact_sections, require_loop: false, key_sensitive: false,
        });
    }

    // Downers: after drops
    if fx_dist.downers >= 1 {
        let downer_sections = vec![
            (s.build.0 as f64, (s.build.0 + 8) as f64),
            (s.breakdown.0 as f64, (s.breakdown.0 + 8) as f64),
            (s.fadedown.0 as f64, (s.fadedown.0 + 8) as f64),
            (s.outro.0 as f64, (s.outro.0 + 8) as f64),
        ];
        tracks.push(TrackArrangement {
            name: "Downlifter".into(), category: "fx_downer".into(), group: "FX".into(),
            color: FX_COLOR, sections: downer_sections, require_loop: false, key_sensitive: false,
        });
    }

    // Snare roll: before drops
    if fx_dist.fills >= 1 {
        let snare_sections = vec![
            ((s.build.1 - 3) as f64, (s.breakdown.0 + 1) as f64),
            ((s.breakdown.1 - 7) as f64, (s.drop1.0 + 1) as f64),
            ((s.drop1.1 - 3) as f64, (s.drop2.0 + 1) as f64),
            ((s.drop2.1 - 7) as f64, (s.fadedown.0 + 1) as f64),
        ];
        tracks.push(TrackArrangement {
            name: "Snare Roll".into(), category: "fx_fill".into(), group: "FX".into(),
            color: FX_COLOR, sections: snare_sections, require_loop: false, key_sensitive: false,
        });
    }

    // Sweeps
    if fx_dist.crashes >= 2 {
        // Sweep up before transitions
        let sweep_up_sections = vec![
            ((s.build.1 - 3) as f64, (s.breakdown.0 + 1) as f64),
            ((s.breakdown.1 - 7) as f64, (s.drop1.0 + 1) as f64),
            ((s.drop1.1 - 3) as f64, (s.drop2.0 + 1) as f64),
            ((s.fadedown.1 - 7) as f64, (s.outro.0 + 1) as f64),
        ];
        tracks.push(TrackArrangement {
            name: "Sweep Up".into(), category: "fx_riser".into(), group: "FX".into(),
            color: FX_COLOR, sections: sweep_up_sections, require_loop: false, key_sensitive: false,
        });

        // Sweep down after drops
        let sweep_down_sections = vec![
            (s.intro.0 as f64, (s.intro.0 + 4) as f64),
            (s.build.0 as f64, (s.build.0 + 8) as f64),
            (s.breakdown.0 as f64, (s.breakdown.0 + 16) as f64),
            (s.drop1.0 as f64, (s.drop1.0 + 12) as f64),
            (s.drop2.0 as f64, (s.drop2.0 + 12) as f64),
            (s.fadedown.0 as f64, (s.fadedown.0 + 12) as f64),
            (s.outro.0 as f64, (s.outro.0 + 12) as f64),
        ];
        tracks.push(TrackArrangement {
            name: "Sweep Down".into(), category: "fx_downer".into(), group: "FX".into(),
            color: FX_COLOR, sections: sweep_down_sections, require_loop: false, key_sensitive: false,
        });
    }

    // Noise texture
    if fx_dist.glitches >= 1 {
        let mut noise_sections = Vec::new();
        let mut bar = s.intro.0 + 8;
        while bar < s.outro.1 {
            noise_sections.push((bar as f64, (bar + 9) as f64));
            bar += 16;
        }
        tracks.push(TrackArrangement {
            name: "Noise".into(), category: "noise".into(), group: "FX".into(),
            color: FX_COLOR, sections: noise_sections, require_loop: false, key_sensitive: false,
        });
    }

    // === ATMOSPHERE ===
    // Always present — continuous
    tracks.push(TrackArrangement {
        name: "Atmos".into(), category: "atmos".into(), group: "Atmosphere".into(),
        color: ATMOS_COLOR,
        sections: vec![
            (s.intro.0 as f64, s.build.1 as f64 + 1.0),
            (s.breakdown.0 as f64, s.breakdown.1 as f64 + 1.0),
            (s.drop1.0 as f64, s.outro.1 as f64 + 1.0),
        ],
        require_loop: false, key_sensitive: true,
    });

    // === VOCALS ===
    for i in 0..config.tracks.vocals.count {
        let name = if i == 0 { "Vox".to_string() } else { format!("Vox {}", i + 1) };
        let sections = vec![
            ((s.breakdown.0 + 16) as f64, s.breakdown.1 as f64 + 1.0),
            ((s.drop1.0 + 16) as f64, s.drop1.1 as f64 + 1.0),
            ((s.drop2.0 + 16) as f64, s.drop2.1 as f64 + 1.0),
        ];
        tracks.push(TrackArrangement {
            name, category: "vocal".into(), group: "Atmosphere".into(),
            color: VOX_COLOR, sections, require_loop: false, key_sensitive: true,
        });
    }

    tracks
}

// ---------------------------------------------------------------------------
// Distribution helpers (from spec)
// ---------------------------------------------------------------------------

struct DrumDistribution { kicks: u32, claps: u32, hats: u32, rides: u32, percs: u32 }
struct BassDistribution { sub: u32, mid: u32 }
struct FxDistribution { crashes: u32, risers: u32, impacts: u32, downers: u32, fills: u32, glitches: u32 }

fn distribute_drums(count: u32) -> DrumDistribution {
    match count {
        0 | 1 => DrumDistribution { kicks: 1, claps: 0, hats: 0, rides: 0, percs: 0 },
        2 => DrumDistribution { kicks: 1, claps: 0, hats: 1, rides: 0, percs: 0 },
        3 => DrumDistribution { kicks: 1, claps: 1, hats: 1, rides: 0, percs: 0 },
        4 => DrumDistribution { kicks: 1, claps: 1, hats: 1, rides: 0, percs: 1 },
        5 => DrumDistribution { kicks: 1, claps: 1, hats: 2, rides: 0, percs: 1 },
        6 => DrumDistribution { kicks: 1, claps: 1, hats: 2, rides: 1, percs: 1 },
        7 => DrumDistribution { kicks: 1, claps: 1, hats: 2, rides: 1, percs: 2 },
        _ => DrumDistribution { kicks: 1, claps: 1, hats: 2, rides: 1, percs: 2 },
    }
}

fn distribute_bass(count: u32) -> BassDistribution {
    match count {
        0 | 1 => BassDistribution { sub: 1, mid: 0 },
        2 => BassDistribution { sub: 1, mid: 1 },
        3 => BassDistribution { sub: 1, mid: 2 },
        _ => BassDistribution { sub: 1, mid: count - 1 },
    }
}

fn distribute_fx(count: u32) -> FxDistribution {
    FxDistribution {
        crashes: (count / 4).max(1),
        risers: (count / 3).max(1),
        impacts: (count / 5).max(1),
        downers: count / 6,
        fills: (count / 4).max(1),
        glitches: count / 8,
    }
}

// ---------------------------------------------------------------------------
// Project name generation
// ---------------------------------------------------------------------------

/// Generate a project name from inputs.
pub fn generate_project_name(config: &ProjectConfig) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    // Use a deterministic-but-varied selection based on current time (millis for uniqueness)
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let mut hasher = DefaultHasher::new();
    now.hash(&mut hasher);
    let seed = hasher.finish() as usize;

    let genre_words: &[&str] = match config.genre {
        Genre::Techno => &[
            "Industrial", "Warehouse", "Underground", "Dark", "Driving", "Hypnotic", "Pulse", "Nocturnal",
            "Bunker", "Concrete", "Steel", "Machine", "Circuit", "Reactor", "Turbine", "Generator",
            "Voltage", "Current", "Analog", "Digital", "Binary", "Cipher", "Protocol", "Sector",
            "Terminal", "Module", "Sequence", "Pattern", "Loop", "Grid", "Mesh", "Core",
        ],
        Genre::Schranz => &[
            "Relentless", "Pounding", "Crushing", "Raw", "Abrasive", "Grain", "Distortion", "Assault",
            "Havoc", "Fracture", "Shatter", "Grind", "Pummel", "Smash", "Wreck", "Demolish",
            "Savage", "Brutal", "Fierce", "Vicious", "Ruthless", "Merciless", "Punish", "Torment",
            "Rampage", "Onslaught", "Barrage", "Storm", "Blitz", "Chaos", "Mayhem", "Carnage",
        ],
        Genre::Trance => &[
            "Euphoria", "Aurora", "Celestial", "Ascend", "Ethereal", "Eclipse", "Horizon", "Nebula",
            "Cosmos", "Galaxy", "Stellar", "Astral", "Lunar", "Solar", "Radiant", "Luminous",
            "Serenity", "Tranquil", "Bliss", "Nirvana", "Paradise", "Utopia", "Elysium", "Zenith",
            "Cascade", "Crystal", "Prism", "Shimmer", "Glow", "Aura", "Spirit", "Essence",
        ],
    };

    let mood_words: &[&str] = if config.hardness >= 0.5 {
        &[
            "Acid", "Rave", "Peak", "Intense", "Raw", "Fury", "Void",
            "Frenzy", "Surge", "Blast", "Burn", "Ignite", "Explode", "Detonate",
            "Warp", "Twist", "Distort", "Corrupt", "Infect", "Mutate", "Override",
            "Annihilate", "Obliterate", "Decimate", "Eradicate", "Purge", "Cleanse", "Reset",
        ]
    } else {
        &[
            "Deep", "Smooth", "Flow", "Drift", "Wave", "Dream", "Signal",
            "Glide", "Float", "Hover", "Suspend", "Linger", "Breathe", "Exhale",
            "Ripple", "Tide", "Current", "Stream", "River", "Ocean", "Abyss",
            "Whisper", "Echo", "Murmur", "Hum", "Pulse", "Throb", "Heartbeat",
        ]
    };

    let key_words: &[&str] = if config.atonal {
        &[
            "Abstract", "System", "Code", "Matrix", "Grid",
            "Algorithm", "Function", "Vector", "Scalar", "Tensor", "Quantum", "Entropy",
            "Fractal", "Recursion", "Iteration", "Parallel", "Serial", "Async", "Sync",
            "Node", "Edge", "Graph", "Tree", "Stack", "Queue", "Buffer",
        ]
    } else {
        &[
            "Shadow", "Descent", "Abyss", "Night", "Rise", "Dawn", "Horizon",
            "Twilight", "Dusk", "Midnight", "Daybreak", "Sunrise", "Sunset", "Equinox",
            "Solstice", "Crescent", "Waning", "Waxing", "Zenith", "Nadir", "Apex",
            "Depth", "Height", "Summit", "Peak", "Valley", "Chasm", "Ravine",
        ]
    };

    let g = genre_words[seed % genre_words.len()];
    let m = mood_words[(seed / 7) % mood_words.len()];
    let k = key_words[(seed / 13) % key_words.len()];
    let g2 = genre_words[(seed / 17) % genre_words.len()];
    let m2 = mood_words[(seed / 23) % mood_words.len()];

    let patterns = [
        format!("{} {}", g, k),
        format!("{} {}", m, g),
        format!("{} {}", k, config.bpm),
        format!("{} {} {}", g, m, k),
        format!("{} {}", g, m),
        format!("{} {}", k, g),
        format!("{} {}", m, k),
        format!("{} {} {}", m, g, k),
        format!("{} {} {}", k, m, g),
        format!("{} {}", g, g2),
        format!("{} {}", m, m2),
        format!("{} {} {}", g, k, m2),
    ];

    let name = &patterns[seed % patterns.len()];
    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    format!("{} - {}", name, timestamp)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compatible_keys() {
        let keys = get_compatible_keys("A", "Aeolian");
        assert!(keys.contains(&"C Major".to_string()));
        assert!(keys.contains(&"A Minor".to_string()));

        let keys = get_compatible_keys("D", "Dorian");
        assert!(keys.contains(&"C Major".to_string()));
        assert!(keys.contains(&"A Minor".to_string()));

        let keys = get_compatible_keys("F#", "Phrygian");
        assert!(keys.contains(&"D Major".to_string()));
        assert!(keys.contains(&"B Minor".to_string()));
    }

    #[test]
    fn test_section_bounds() {
        let s = SectionBounds::standard();
        assert_eq!(s.total_bars, 224);
        assert_eq!(s.intro, (1, 32));
        assert_eq!(s.outro, (193, 224));

        let t = SectionBounds::trance();
        assert_eq!(t.total_bars, 256);
        assert!(t.breakdown.1 - t.breakdown.0 > s.breakdown.1 - s.breakdown.0); // longer breakdown

        let sc = SectionBounds::schranz();
        assert_eq!(sc.total_bars, 208);
        assert!(sc.breakdown.1 - sc.breakdown.0 < s.breakdown.1 - s.breakdown.0); // shorter breakdown
    }

    #[test]
    fn test_fill_gap_pattern() {
        let sections = sections_with_fill_gaps(1, 32);
        assert!(!sections.is_empty());
        // Each section should end before the next phrase boundary
        for (start, end) in &sections {
            assert!(start < end, "start {} should be < end {}", start, end);
        }
    }

    #[test]
    fn test_arrangement_track_count() {
        let config = ProjectConfig {
            genre: Genre::Techno,
            hardness: 0.3,
            chaos: 0.3,
            glitch_intensity: 0.0,
            section_overrides: SectionOverridesConfig::default(),
            density: 0.0,
            variation: 0.0,
            parallelism: 0.4,
            scatter: 0.0,
            bpm: 130,
            root_note: Some("A".into()),
            mode: Some("Aeolian".into()),
            atonal: false,
            keywords: vec![],
            element_keywords: Default::default(),
            sample_source_path: None,
            tracks: TrackConfig::default(),
            track_counts: TrackCountsConfig::default(),
            type_atonal: TypeAtonalConfig::default(),
            output_path: "/tmp/test.als".into(),
            project_name: None,
            num_songs: 1,
        };
        let arrangement = build_arrangement(&config);
        assert!(arrangement.len() >= 10, "Expected at least 10 tracks, got {}", arrangement.len());

        // Verify all tracks have sections
        for track in &arrangement {
            assert!(!track.sections.is_empty(), "Track {} has no sections", track.name);
        }
    }

    #[test]
    fn test_arrangement_schranz() {
        let config = ProjectConfig {
            genre: Genre::Schranz,
            hardness: 0.8,
            chaos: 0.3,
            glitch_intensity: 0.0,
            section_overrides: SectionOverridesConfig::default(),
            density: 0.0,
            variation: 0.0,
            parallelism: 0.4,
            scatter: 0.0,
            bpm: 155,
            root_note: None,
            mode: None,
            atonal: true,
            keywords: vec![],
            element_keywords: Default::default(),
            sample_source_path: None,
            tracks: TrackConfig {
                drums: ElementConfig { count: 6, character: 0.8 },
                bass: ElementConfig { count: 3, character: 0.9 },
                leads: ElementConfig { count: 2, character: 0.7 },
                pads: ElementConfig { count: 1, character: 0.6 },
                fx: ElementConfig { count: 8, character: 0.8 },
                vocals: ElementConfig { count: 0, character: 0.0 },
            },
            track_counts: TrackCountsConfig::default(),
            type_atonal: TypeAtonalConfig::default(),
            output_path: "/tmp/test.als".into(),
            project_name: None,
            num_songs: 1,
        };
        let arrangement = build_arrangement(&config);
        // Schranz should have a Drive track
        assert!(arrangement.iter().any(|t| t.name == "Drive"), "Schranz should have Drive track");
    }

    #[test]
    fn test_project_name_generation() {
        let config = ProjectConfig {
            genre: Genre::Techno,
            hardness: 0.5,
            chaos: 0.3,
            glitch_intensity: 0.0,
            section_overrides: SectionOverridesConfig::default(),
            density: 0.0,
            variation: 0.0,
            parallelism: 0.4,
            scatter: 0.0,
            bpm: 130,
            root_note: Some("A".into()),
            mode: Some("Aeolian".into()),
            atonal: false,
            keywords: vec![],
            element_keywords: Default::default(),
            sample_source_path: None,
            tracks: TrackConfig::default(),
            track_counts: TrackCountsConfig::default(),
            type_atonal: TypeAtonalConfig::default(),
            output_path: "/tmp/test.als".into(),
            project_name: None,
            num_songs: 1,
        };
        let name = generate_project_name(&config);
        assert!(!name.is_empty());
        assert!(name.contains(" - "), "Name should contain timestamp separator: {}", name);
    }
}
