//! ALS project generation pipeline.
//!
//! Takes wizard inputs (genre, hardness, BPM, key, keywords, track counts)
//! and produces a complete Ableton Live Set file by:
//! 1. Querying sample_analysis tables for ranked samples per category
//! 2. Building an arrangement (element entry/exit per section)
//! 3. Generating ALS XML via the existing als_generator infrastructure

use crate::db;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

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
    /// User-chosen bar length for each of the 7 sections. Defaults to the
    /// Techno 32-bar-each canonical layout when absent; the frontend always
    /// sends the genre-specific defaults (or user overrides) via
    /// prefs `alsSectionLengthsByGenre[<genre>]`.
    #[serde(default)]
    pub section_lengths: SectionLengths,
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
    /// Optional seed for deterministic generation. `None` = a fresh random seed
    /// is drawn by the Tauri command and returned to the caller so it can be
    /// locked for a subsequent "regenerate with the same seed" run.
    ///
    /// When the user locks the seed, identical configs produce bit-identical
    /// arrangements and project names (the filename still gets a wall-clock
    /// timestamp suffix so back-to-back locked runs do not overwrite each
    /// other).
    ///
    /// Accepts either a JSON number or a string form — the wizard always
    /// sends a string so seeds above `Number.MAX_SAFE_INTEGER` round-trip
    /// through JS without precision loss.
    #[serde(default, deserialize_with = "deserialize_seed")]
    pub seed: Option<u64>,
}

/// Deserialize the wizard's `seed` field from either a u64 JSON number or a
/// decimal string. Empty strings, whitespace, and `null` map to `None` so the
/// Tauri command treats them as "pick a fresh random seed for this run".
fn deserialize_seed<'de, D>(d: D) -> Result<Option<u64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum SeedRepr {
        Num(u64),
        Str(String),
    }
    let raw: Option<SeedRepr> = Option::deserialize(d)?;
    match raw {
        None => Ok(None),
        Some(SeedRepr::Num(n)) => Ok(Some(n)),
        Some(SeedRepr::Str(s)) => {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                Ok(None)
            } else {
                trimmed
                    .parse::<u64>()
                    .map(Some)
                    .map_err(serde::de::Error::custom)
            }
        }
    }
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
    #[serde(default)] pub keys: Option<u32>,
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

/// Per-8-bar-block overrides for a single dynamics parameter.
///
/// Keys are the starting bar of each 8-bar block (1, 9, 17, 25, …), absolute
/// within the song arrangement. Values are 0.0–1.0. A missing key means
/// "use the global scalar for this param at that block".
///
/// Kept as a `BTreeMap<String, f32>` so JSON serialization is trivial (JSON
/// object keys are strings by spec). Helpers below parse keys back to `u32`
/// when resolving bar → block.
///
/// Replaced the old 7-named-section layout (intro/build/breakdown/drop1/…) on
/// 2026-04-16; the timeline UI now subdivides each section into 8-bar blocks
/// so users can shape dynamics at phrase granularity, not just section
/// granularity.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct SectionValues {
    pub blocks: BTreeMap<String, f32>,
}

impl SectionValues {
    /// Any override set? (used to skip resolver work when the param has no overrides)
    pub fn any(&self) -> bool {
        !self.blocks.is_empty()
    }

    /// Return the override value for the 8-bar block containing `bar` (1-indexed),
    /// falling back to `global` when nothing is pinned there. Blocks start on
    /// bars 1, 9, 17, … so `((bar-1)/8)*8 + 1` maps any bar to its block start.
    pub fn value_at_bar(&self, bar: u32, global: f32) -> f32 {
        let block_start = ((bar.saturating_sub(1)) / 8) * 8 + 1;
        self.blocks
            .get(&block_start.to_string())
            .copied()
            .unwrap_or(global)
    }

    /// Every pinned block value, for callers that want to average or sum
    /// overrides (e.g. the per-section `apply_*` helpers).
    pub fn values(&self) -> impl Iterator<Item = f32> + '_ {
        self.blocks.values().copied()
    }

    /// Insert a value at an explicit block-start bar. Out-of-range values are
    /// clamped to [0, 1]; non-block-start bars are snapped down to their block.
    pub fn set(&mut self, block_start_bar: u32, value: f32) {
        let snapped = ((block_start_bar.saturating_sub(1)) / 8) * 8 + 1;
        let clamped = value.clamp(0.0, 1.0);
        self.blocks.insert(snapped.to_string(), clamped);
    }
}

/// Bar length for each of the seven song sections. All values must be
/// multiples of 8 (the phrase grid) and at least 8 bars. Total song length =
/// sum of all seven fields. Per-genre defaults match the original genre
/// conventions that used to live in the dead `SectionBounds` struct.
///
/// Landed 2026-04-16 alongside the draggable-boundary timeline UI — the
/// frontend ships this in `ProjectConfig.section_lengths` keyed per-genre
/// under prefs `alsSectionLengthsByGenre`. The backend previously used the
/// compile-time consts `INTRO_START`, `BUILD1_START`, … from
/// `techno_generator` (all 32 bars); those still exist as the *canonical*
/// layout for arrangement templates, which remap onto user layouts at
/// generation time via `techno_generator::remap_bar_range`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct SectionLengths {
    pub intro: u32,
    pub build: u32,
    pub breakdown: u32,
    pub drop1: u32,
    pub drop2: u32,
    pub fadedown: u32,
    pub outro: u32,
}

impl SectionLengths {
    /// Canonical Techno layout: 32 × 7 = 224 bars. Also the serde default and
    /// the reference layout that arrangement templates are written against.
    pub const fn techno_default() -> Self {
        Self { intro: 32, build: 32, breakdown: 32, drop1: 32, drop2: 32, fadedown: 32, outro: 32 }
    }
    /// Trance: 48-bar breakdown (emotional development) and 48-bar outro
    /// (DJ-friendly mix-out). 32 / 32 / 48 / 32 / 32 / 32 / 48 = 256 bars.
    pub const fn trance_default() -> Self {
        Self { intro: 32, build: 32, breakdown: 48, drop1: 32, drop2: 32, fadedown: 32, outro: 48 }
    }
    /// Schranz: brief breakdown (16), extended Drop 2 (48), short outro (16).
    /// 32 / 32 / 16 / 32 / 48 / 32 / 16 = 208 bars.
    pub const fn schranz_default() -> Self {
        Self { intro: 32, build: 32, breakdown: 16, drop1: 32, drop2: 48, fadedown: 32, outro: 16 }
    }

    pub fn for_genre(g: Genre) -> Self {
        match g {
            Genre::Techno => Self::techno_default(),
            Genre::Trance => Self::trance_default(),
            Genre::Schranz => Self::schranz_default(),
        }
    }

    pub fn total_bars(&self) -> u32 {
        self.intro + self.build + self.breakdown + self.drop1 + self.drop2 + self.fadedown + self.outro
    }

    /// Expand to absolute (start, end_exclusive) tuples for each section.
    /// Section 1 is always bars 1.. since bars are 1-indexed in Ableton.
    pub fn starts(&self) -> SectionStarts {
        let mut s = 1u32;
        let intro = (s, s + self.intro); s += self.intro;
        let build = (s, s + self.build); s += self.build;
        let breakdown = (s, s + self.breakdown); s += self.breakdown;
        let drop1 = (s, s + self.drop1); s += self.drop1;
        let drop2 = (s, s + self.drop2); s += self.drop2;
        let fadedown = (s, s + self.fadedown); s += self.fadedown;
        let outro = (s, s + self.outro);
        SectionStarts { intro, build, breakdown, drop1, drop2, fadedown, outro }
    }

    /// Clamp every field to ≥ 8 bars and snap to an 8-bar multiple. Protects
    /// against bad IPC payloads silently corrupting the arrangement (e.g., a
    /// frontend bug sending intro=3 would otherwise mangle every template).
    pub fn sanitize(mut self) -> Self {
        let snap = |b: u32| -> u32 {
            let snapped = (b / 8) * 8;
            snapped.max(8)
        };
        self.intro = snap(self.intro);
        self.build = snap(self.build);
        self.breakdown = snap(self.breakdown);
        self.drop1 = snap(self.drop1);
        self.drop2 = snap(self.drop2);
        self.fadedown = snap(self.fadedown);
        self.outro = snap(self.outro);
        self
    }
}

impl Default for SectionLengths {
    fn default() -> Self { Self::techno_default() }
}

/// Concrete (start_bar, end_bar_exclusive) pairs per section, computed from
/// a `SectionLengths`. Used by the generator for locators, density block
/// iteration, and remapping canonical template ranges onto the user's layout.
#[derive(Debug, Clone, Copy)]
pub struct SectionStarts {
    pub intro: (u32, u32),
    pub build: (u32, u32),
    pub breakdown: (u32, u32),
    pub drop1: (u32, u32),
    pub drop2: (u32, u32),
    pub fadedown: (u32, u32),
    pub outro: (u32, u32),
}

impl SectionStarts {
    pub fn total_bars(&self) -> u32 {
        self.outro.1 - 1
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
            lead: 1, synth: 3, pad: 2, arp: 2, keys: None,
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
    #[serde(default)] pub keys: Option<bool>,
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
// Project name generation
// ---------------------------------------------------------------------------

/// Generate a project name from inputs. `seed` is the resolved generation
/// seed (see `ProjectConfig::seed`) — same seed + same config → same name.
/// Callers who want a non-deterministic name pass a fresh random u64.
pub fn generate_project_name(config: &ProjectConfig, seed: u64) -> String {
    // Use a seeded RNG for better distribution instead of modular arithmetic
    // which had collision issues (e.g. "Ocean Demolish Night" appearing repeatedly)
    use rand::prelude::*;
    use rand::rngs::StdRng;
    let mut rng = StdRng::seed_from_u64(seed);

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

    // Use RNG to pick words with better distribution than modular arithmetic
    let g = genre_words[rng.random_range(0..genre_words.len())];
    let m = mood_words[rng.random_range(0..mood_words.len())];
    let k = key_words[rng.random_range(0..key_words.len())];
    let g2 = genre_words[rng.random_range(0..genre_words.len())];
    let m2 = mood_words[rng.random_range(0..mood_words.len())];

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

    let name = &patterns[rng.random_range(0..patterns.len())];
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
            section_lengths: SectionLengths::default(),
            output_path: "/tmp/test.als".into(),
            project_name: None,
            num_songs: 1,
            seed: None,
        };
        let name = generate_project_name(&config, 0x4242_4242);
        assert!(!name.is_empty());
        assert!(name.contains(" - "), "Name should contain timestamp separator: {}", name);
    }

    #[test]
    fn test_genre_bpm() {
        assert_eq!(Genre::Techno.default_bpm(), 132);
        assert_eq!(Genre::Schranz.default_bpm(), 155);
        assert_eq!(Genre::Trance.default_bpm(), 140);
        
        assert_eq!(Genre::Techno.bpm_range(), (120, 140));
        assert_eq!(Genre::Schranz.bpm_range(), (145, 165));
        assert_eq!(Genre::Trance.bpm_range(), (130, 160));
    }

    #[test]
    fn test_section_values_any() {
        let mut sv = SectionValues::default();
        assert!(!sv.any());
        
        sv.set(1, 0.5);
        assert!(sv.any());
        
        sv.blocks.clear();
        sv.set(193, 0.1);
        assert!(sv.any());
    }

    #[test]
    fn test_is_key_sensitive() {
        // Tonal categories
        assert!(is_key_sensitive("sub_bass"));
        assert!(is_key_sensitive("mid_bass"));
        assert!(is_key_sensitive("lead"));
        assert!(is_key_sensitive("pad"));
        assert!(is_key_sensitive("arp"));
        assert!(is_key_sensitive("pluck"));
        assert!(is_key_sensitive("stab"));
        assert!(is_key_sensitive("acid"));
        assert!(is_key_sensitive("atmos"));
        assert!(is_key_sensitive("vocal"));
        assert!(is_key_sensitive("vocal_phrase"));
        assert!(is_key_sensitive("schranz_drive"));
        
        // Atonal categories
        assert!(!is_key_sensitive("kick"));
        assert!(!is_key_sensitive("clap"));
        assert!(!is_key_sensitive("closed_hat"));
        assert!(!is_key_sensitive("open_hat"));
        assert!(!is_key_sensitive("ride"));
        assert!(!is_key_sensitive("perc"));
        assert!(!is_key_sensitive("noise"));
        assert!(!is_key_sensitive("fx_riser"));
        assert!(!is_key_sensitive("fx_downer"));
        assert!(!is_key_sensitive("fx_crash"));
        assert!(!is_key_sensitive("fx_impact"));
    }

    #[test]
    fn test_category_to_like_pattern() {
        let kick_pattern = category_to_like_pattern("kick");
        assert!(kick_pattern.contains("kick"));
        assert!(kick_pattern.contains("kik"));
        assert!(kick_pattern.contains("bd"));
        
        let sub_pattern = category_to_like_pattern("sub_bass");
        assert!(sub_pattern.contains("sub"));
        assert!(sub_pattern.contains("808"));
        
        let unknown_pattern = category_to_like_pattern("some_unknown_cat");
        assert!(unknown_pattern.contains("fx"));
    }

    #[test]
    fn test_build_target_keys() {
        let mut config = ProjectConfig {
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
            root_note: Some("C".into()),
            mode: Some("Aeolian".into()),
            atonal: false,
            keywords: vec![],
            element_keywords: Default::default(),
            sample_source_path: None,
            tracks: TrackConfig::default(),
            track_counts: TrackCountsConfig::default(),
            type_atonal: TypeAtonalConfig::default(),
            section_lengths: SectionLengths::default(),
            output_path: "/tmp/test.als".into(),
            project_name: None,
            num_songs: 1,
            seed: None,
        };

        let keys = build_target_keys(&config);
        assert!(!keys.is_empty());
        assert!(keys.contains(&"D# Major".to_string()));
        
        config.atonal = true;
        let keys_atonal = build_target_keys(&config);
        assert!(keys_atonal.is_empty());
    }

    #[test]
    fn test_compatible_keys_edge_cases() {
        // Unknown root or mode should return a safe fallback
        let keys = get_compatible_keys("Z", "Aeolian");
        assert_eq!(keys, vec!["Z Minor".to_string()]);
        
        let keys2 = get_compatible_keys("C", "UnknownMode");
        // C Aeolian (offset -9) -> major_idx 3 (D#), minor_idx 0 (C)
        assert!(keys2.contains(&"D# Major".to_string()));
        assert!(keys2.contains(&"C Minor".to_string()));
        
        // Case insensitivity
        let keys3 = get_compatible_keys("c", "aeolian");
        assert!(keys3.contains(&"D# Major".to_string()));
        assert!(keys3.contains(&"C Minor".to_string()));
    }

    /// Lock the seed → same config + same seed must always produce the same
    /// word-pattern portion of the project name. The trailing `- YYYYMMDD_HHMMSS`
    /// stamp still varies per wall-clock call, so we compare only the prefix.
    #[test]
    fn test_generate_project_name_deterministic_with_seed() {
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
            section_lengths: SectionLengths::default(),
            output_path: "/tmp/test.als".into(),
            project_name: None,
            num_songs: 1,
            seed: None,
        };
        let prefix = |name: String| name.split(" - ").next().unwrap().to_string();
        let a = prefix(generate_project_name(&config, 0xDEAD_BEEF));
        let b = prefix(generate_project_name(&config, 0xDEAD_BEEF));
        assert_eq!(a, b, "same seed + same config → same name prefix");
    }

    /// Different seeds should produce different name prefixes — not guaranteed
    /// for every pair (the word pool is finite), but a handful of distinct
    /// seeds should yield at least two distinct names.
    #[test]
    fn test_generate_project_name_varies_with_seed() {
        let config = ProjectConfig {
            genre: Genre::Trance,
            hardness: 0.5,
            chaos: 0.3,
            glitch_intensity: 0.0,
            section_overrides: SectionOverridesConfig::default(),
            density: 0.0,
            variation: 0.0,
            parallelism: 0.4,
            scatter: 0.0,
            bpm: 140,
            root_note: Some("C".into()),
            mode: Some("Ionian".into()),
            atonal: false,
            keywords: vec![],
            element_keywords: Default::default(),
            sample_source_path: None,
            tracks: TrackConfig::default(),
            track_counts: TrackCountsConfig::default(),
            type_atonal: TypeAtonalConfig::default(),
            section_lengths: SectionLengths::default(),
            output_path: "/tmp/test.als".into(),
            project_name: None,
            num_songs: 1,
            seed: None,
        };
        let prefix = |s: u64| -> String {
            generate_project_name(&config, s).split(" - ").next().unwrap().to_string()
        };
        // Scan a handful of well-separated seeds and assert we see at least two
        // distinct prefixes. Using 10 seeds keeps the test fast and the chance
        // of a false negative astronomically small (given 32 genre × 28 mood
        // × 28 key words + 12 patterns).
        let names: std::collections::HashSet<String> =
            (0u64..10).map(|i| prefix(i.wrapping_mul(0x9E37_79B9_7F4A_7C15))).collect();
        assert!(names.len() >= 2, "10 distinct seeds should produce ≥2 distinct names, got {:?}", names);
    }

    /// The wizard sends `seed` as a string (to avoid JS Number precision loss
    /// for values above 2^53). The custom deserializer must accept both forms
    /// plus treat empty / whitespace-only strings as `None`.
    #[test]
    fn test_project_config_seed_accepts_string_and_number() {
        let make_json = |seed_val: &str| format!(
            r#"{{
                "genre": "techno", "hardness": 0.5, "bpm": 130, "atonal": false,
                "keywords": [], "element_keywords": {{}},
                "tracks": {{
                    "drums": {{"count": 3, "character": 0.5}},
                    "bass": {{"count": 2, "character": 0.5}},
                    "leads": {{"count": 2, "character": 0.5}},
                    "pads": {{"count": 2, "character": 0.5}},
                    "fx": {{"count": 6, "character": 0.5}},
                    "vocals": {{"count": 0, "character": 0.5}}
                }},
                "output_path": "/tmp/x.als", "project_name": null, "num_songs": 1,
                "seed": {}
            }}"#,
            seed_val
        );
        let parse = |json: String| -> ProjectConfig {
            serde_json::from_str(&json).expect("deserialize")
        };

        // JSON number
        assert_eq!(parse(make_json("12345")).seed, Some(12345));
        // JSON string (what the wizard actually sends)
        assert_eq!(parse(make_json("\"12345\"")).seed, Some(12345));
        // Large u64 — exceeds Number.MAX_SAFE_INTEGER (2^53 - 1)
        assert_eq!(parse(make_json("\"18446744073709551615\"")).seed, Some(u64::MAX));
        // Empty string → treat as unset
        assert_eq!(parse(make_json("\"\"")).seed, None);
        // Whitespace-only string → treat as unset
        assert_eq!(parse(make_json("\"   \"")).seed, None);
        // Explicit null → unset
        assert_eq!(parse(make_json("null")).seed, None);
        // Non-numeric string → hard error (surfaced to the user, not silently dropped)
        assert!(serde_json::from_str::<ProjectConfig>(&make_json("\"abc\"")).is_err());
    }

    /// Config-as-JSON without a `seed` field must deserialize with `seed = None`.
    /// This is the back-compat guarantee for any cached wizard payloads.
    #[test]
    fn test_project_config_seed_defaults_to_none() {
        let json = r#"{
            "genre": "techno",
            "hardness": 0.5,
            "bpm": 130,
            "atonal": false,
            "keywords": [],
            "element_keywords": {},
            "tracks": {
                "drums": {"count": 3, "character": 0.5},
                "bass": {"count": 2, "character": 0.5},
                "leads": {"count": 2, "character": 0.5},
                "pads": {"count": 2, "character": 0.5},
                "fx": {"count": 6, "character": 0.5},
                "vocals": {"count": 0, "character": 0.5}
            },
            "output_path": "/tmp/x.als",
            "project_name": null,
            "num_songs": 1
        }"#;
        let cfg: ProjectConfig = serde_json::from_str(json).expect("deserialize");
        assert!(cfg.seed.is_none(), "missing `seed` field must default to None");
    }
}
