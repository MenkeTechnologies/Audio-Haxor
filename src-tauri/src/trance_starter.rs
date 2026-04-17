//! Trance song starter — finds matching samples for a generated MIDI lead.
//!
//! Given a key and genre, queries the sample library for compatible samples
//! across all trance layers: kick, bass, pad, arp, pluck, vocal, FX.
//! Uses key compatibility (relative major/minor + circle-of-fifths neighbors),
//! genre scoring, and category classification from the existing analysis pipeline.

use crate::als_project::{get_compatible_keys, SelectedSample};
use crate::db;
use crate::midi_generator::MidiGenConfig;
use serde::{Deserialize, Serialize};

// ── Public types ─────────────────────────────────────────────────────

/// Layer categories for a trance arrangement.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TranceLayer {
    Kick,
    Bass,
    Pad,
    Arp,
    Pluck,
    Lead,
    Vocal,
    VocalChop,
    /// Ethereal vocal atmospheres — long, reverb-heavy, sustained vowels/textures.
    VocalAtmosphere,
    /// Vocal phrases — melodic sung lines.
    VocalPhrase,
    Riser,
    Downlifter,
    Impact,
    Crash,
    Atmos,
}

/// Query parameters for a layer.
struct LayerQuery {
    category: &'static str,
    tonal: bool,
    prefer_loop: bool,
    /// Extra SQL WHERE clauses (name/path keyword matching, duration filters).
    extra_where: &'static str,
    /// Sort bias: extra ORDER BY clause prepended to the default scoring.
    extra_order: &'static str,
}

impl TranceLayer {
    fn query(self) -> LayerQuery {
        match self {
            Self::Kick => LayerQuery {
                category: "kick", tonal: false, prefer_loop: false,
                extra_where: "", extra_order: "",
            },
            Self::Bass => LayerQuery {
                category: "mid_bass", tonal: true, prefer_loop: true,
                extra_where: "", extra_order: "",
            },
            Self::Pad => LayerQuery {
                category: "pad", tonal: true, prefer_loop: true,
                extra_where: "", extra_order: "",
            },
            Self::Arp => LayerQuery {
                category: "arp", tonal: true, prefer_loop: true,
                extra_where: "", extra_order: "",
            },
            Self::Pluck => LayerQuery {
                category: "pluck", tonal: true, prefer_loop: false,
                extra_where: "", extra_order: "",
            },
            Self::Lead => LayerQuery {
                category: "lead", tonal: true, prefer_loop: true,
                extra_where: "", extra_order: "",
            },
            Self::Vocal => LayerQuery {
                category: "vocal", tonal: true, prefer_loop: false,
                extra_where: "", extra_order: "",
            },
            Self::VocalChop => LayerQuery {
                category: "vocal_chop", tonal: false, prefer_loop: false,
                extra_where: "", extra_order: "",
            },
            Self::VocalAtmosphere => LayerQuery {
                // Ethereal vocals: long duration (>3s), keywords in name/path
                category: "vocal", tonal: true, prefer_loop: false,
                extra_where: "AND s.duration > 3.0 \
                    AND (LOWER(s.name) LIKE '%atmosphere%' \
                      OR LOWER(s.name) LIKE '%ethereal%' \
                      OR LOWER(s.name) LIKE '%evolving%' \
                      OR LOWER(s.name) LIKE '%texture%' \
                      OR LOWER(s.name) LIKE '%swell%' \
                      OR LOWER(s.name) LIKE '%long%' \
                      OR LOWER(s.name) LIKE '%wet%' \
                      OR LOWER(s.name) LIKE '%ahh%' \
                      OR LOWER(s.name) LIKE '%ohh%' \
                      OR LOWER(s.name) LIKE '%choir%' \
                      OR LOWER(s.name) LIKE '%pad%' \
                      OR LOWER(s.path) LIKE '%atmosphere%' \
                      OR LOWER(s.path) LIKE '%ethereal%' \
                      OR LOWER(s.path) LIKE '%evolving%' \
                      OR LOWER(s.path) LIKE '%texture%' \
                      OR LOWER(s.path) LIKE '%swell%' \
                      OR LOWER(s.path) LIKE '%choir%')",
                extra_order: "s.duration DESC,", // prefer longer samples
            },
            Self::VocalPhrase => LayerQuery {
                category: "vocal_phrase", tonal: true, prefer_loop: false,
                extra_where: "", extra_order: "",
            },
            Self::Riser => LayerQuery {
                category: "fx_riser", tonal: false, prefer_loop: false,
                extra_where: "", extra_order: "",
            },
            Self::Downlifter => LayerQuery {
                category: "fx_downer", tonal: false, prefer_loop: false,
                extra_where: "", extra_order: "",
            },
            Self::Impact => LayerQuery {
                category: "fx_impact", tonal: false, prefer_loop: false,
                extra_where: "", extra_order: "",
            },
            Self::Crash => LayerQuery {
                category: "fx_crash", tonal: false, prefer_loop: false,
                extra_where: "", extra_order: "",
            },
            Self::Atmos => LayerQuery {
                category: "atmos", tonal: true, prefer_loop: true,
                extra_where: "", extra_order: "",
            },
        }
    }

    /// All layers in arrangement order.
    fn all() -> &'static [Self] {
        &[
            Self::Kick,
            Self::Bass,
            Self::Pad,
            Self::Arp,
            Self::Pluck,
            Self::Lead,
            Self::VocalAtmosphere,
            Self::VocalPhrase,
            Self::Vocal,
            Self::VocalChop,
            Self::Riser,
            Self::Downlifter,
            Self::Impact,
            Self::Crash,
            Self::Atmos,
        ]
    }
}

/// Configuration for finding matching samples.
#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TranceStarterConfig {
    /// Key root pitch class: 0=C … 11=B.
    pub key_root: u8,
    /// true = minor, false = major.
    pub minor: bool,
    /// Max samples to return per layer.
    #[serde(default = "default_per_layer")]
    pub per_layer: u32,
    /// Optional: also generate MIDI leads as part of the starter.
    pub midi_config: Option<MidiGenConfig>,
}

fn default_per_layer() -> u32 {
    10
}

/// A group of matched samples for one layer.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LayerResult {
    pub layer: String,
    pub is_tonal: bool,
    pub key_matched: bool,
    pub samples: Vec<SelectedSample>,
}

/// Full result of a trance starter query.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranceStarterResult {
    pub key: String,
    pub compatible_keys: Vec<String>,
    pub layers: Vec<LayerResult>,
    /// Generated MIDI files (if midi_config was provided).
    pub midi_files: Vec<MidiFileInfo>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MidiFileInfo {
    pub path: String,
    pub size: usize,
}

// ── Note name mapping ────────────────────────────────────────────────

const NOTES: [&str; 12] = [
    "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
];

fn root_to_name(root: u8) -> &'static str {
    NOTES.get(root as usize).unwrap_or(&"C")
}

fn mode_name(minor: bool) -> &'static str {
    if minor { "Aeolian" } else { "Ionian" }
}

// ── Core query ───────────────────────────────────────────────────────

/// Find matching samples for all trance layers.
pub fn find_matching_samples(config: &TranceStarterConfig) -> Result<TranceStarterResult, String> {
    if config.key_root > 11 {
        return Err("key_root must be 0–11".into());
    }

    let root = root_to_name(config.key_root);
    let mode = mode_name(config.minor);
    let compatible = get_compatible_keys(root, mode);
    let key_display = format!(
        "{} {}",
        root,
        if config.minor { "Minor" } else { "Major" }
    );

    // Circle-of-fifths neighbors: one step each direction
    let fifth_up = NOTES[((config.key_root + 7) % 12) as usize];
    let fifth_dn = NOTES[((config.key_root + 5) % 12) as usize];
    let mut all_keys = compatible.clone();
    // Add fifths neighbors (both minor and major forms)
    for neighbor in [fifth_up, fifth_dn] {
        let neighbor_keys = get_compatible_keys(neighbor, mode);
        for k in neighbor_keys {
            if !all_keys.contains(&k) {
                all_keys.push(k);
            }
        }
    }

    let mut layers = Vec::new();

    for &layer in TranceLayer::all() {
        let result = query_layer(layer, &compatible, &all_keys, config.per_layer);
        layers.push(result);
    }

    Ok(TranceStarterResult {
        key: key_display,
        compatible_keys: all_keys,
        layers,
        midi_files: Vec::new(),
    })
}

/// Query samples for a single layer.
fn query_layer(
    layer: TranceLayer,
    strict_keys: &[String],
    extended_keys: &[String],
    limit: u32,
) -> LayerResult {
    let lq = layer.query();

    // Try strict key match first (relative major/minor only)
    if lq.tonal && !strict_keys.is_empty() {
        if let Ok(samples) = query_category(&lq, Some(strict_keys), limit) {
            if !samples.is_empty() {
                return LayerResult {
                    layer: lq.category.into(),
                    is_tonal: true,
                    key_matched: true,
                    samples,
                };
            }
        }
        // Fallback: extended keys (circle of fifths)
        if let Ok(samples) = query_category(&lq, Some(extended_keys), limit) {
            if !samples.is_empty() {
                return LayerResult {
                    layer: lq.category.into(),
                    is_tonal: true,
                    key_matched: true,
                    samples,
                };
            }
        }
    }

    // Atonal or no key match: query without key filter
    let samples = query_category(&lq, None, limit).unwrap_or_default();

    LayerResult {
        layer: lq.category.into(),
        is_tonal: lq.tonal,
        key_matched: false,
        samples,
    }
}

/// Run a SQL query against the sample DB for one category.
fn query_category(
    lq: &LayerQuery,
    keys: Option<&[String]>,
    limit: u32,
) -> Result<Vec<SelectedSample>, String> {
    let key_clause = if let Some(ks) = keys {
        if ks.is_empty() {
            String::new()
        } else {
            let list: String = ks
                .iter()
                .map(|k| format!("'{}'", k.replace('\'', "''")))
                .collect::<Vec<_>>()
                .join(", ");
            format!("AND a.parsed_key IN ({})", list)
        }
    } else {
        String::new()
    };

    let loop_clause = if lq.prefer_loop {
        "AND a.is_loop = 1"
    } else {
        ""
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
           AND a.category_id = (SELECT id FROM sample_categories WHERE name = '{cat}')
           {loop_clause}
           {key_clause}
           {extra_where}
         ORDER BY
           {extra_order}
           COALESCE(m.genre_score, 0) DESC,
           COALESCE(m.hardness_score, 0) ASC,
           a.category_confidence DESC,
           RANDOM()
         LIMIT {limit}",
        cat = lq.category.replace('\'', "''"),
        extra_where = lq.extra_where,
        extra_order = lq.extra_order,
    );

    db::global().query_samples_for_als(&query)
}

// ── Generate + match (combined workflow) ─────────────────────────────

/// Generate MIDI leads AND find matching samples in one call.
pub fn generate_and_match(
    config: &TranceStarterConfig,
    output_dir: &std::path::Path,
) -> Result<TranceStarterResult, String> {
    let mut result = find_matching_samples(config)?;

    if let Some(ref midi_cfg) = config.midi_config {
        let files = crate::midi_generator::generate_batch(midi_cfg)?;
        let n = files.len();

        if !output_dir.exists() {
            std::fs::create_dir_all(output_dir).map_err(|e| e.to_string())?;
        }

        for (i, bytes) in files.iter().enumerate() {
            let name = crate::midi_generator::build_filename(midi_cfg, i, n);
            let path = output_dir.join(&name);
            std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
            result.midi_files.push(MidiFileInfo {
                path: path.to_string_lossy().into(),
                size: bytes.len(),
            });
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_root_to_name() {
        assert_eq!(root_to_name(0), "C");
        assert_eq!(root_to_name(9), "A");
        assert_eq!(root_to_name(11), "B");
    }

    #[test]
    fn test_layer_categories() {
        assert_eq!(TranceLayer::Kick.query().category, "kick");
        assert_eq!(TranceLayer::Bass.query().category, "mid_bass");
        assert_eq!(TranceLayer::Riser.query().category, "fx_riser");
        assert_eq!(TranceLayer::VocalAtmosphere.query().category, "vocal");
        assert_eq!(TranceLayer::VocalPhrase.query().category, "vocal_phrase");
    }

    #[test]
    fn test_tonal_layers() {
        assert!(TranceLayer::Bass.query().tonal);
        assert!(TranceLayer::Pad.query().tonal);
        assert!(TranceLayer::VocalAtmosphere.query().tonal);
        assert!(!TranceLayer::Kick.query().tonal);
        assert!(!TranceLayer::Crash.query().tonal);
    }

    #[test]
    fn test_vocal_atmosphere_has_duration_filter() {
        let lq = TranceLayer::VocalAtmosphere.query();
        assert!(lq.extra_where.contains("duration > 3.0"));
        assert!(lq.extra_where.contains("atmosphere"));
        assert!(lq.extra_where.contains("ethereal"));
        assert!(lq.extra_order.contains("duration DESC"));
    }

    #[test]
    fn test_circle_of_fifths_keys() {
        // A minor: compatible = A Minor + C Major
        // Fifth up from A = E, fifth down = D
        // So extended keys should include E minor/G major and D minor/F major
        let config = TranceStarterConfig {
            key_root: 9, // A
            minor: true,
            per_layer: 5,
            midi_config: None,
        };
        let root = root_to_name(config.key_root);
        let mode = mode_name(config.minor);
        let compatible = get_compatible_keys(root, mode);
        assert!(compatible.contains(&"A Minor".to_string()));
        assert!(compatible.contains(&"C Major".to_string()));

        // Extended with fifths
        let fifth_up = NOTES[((config.key_root + 7) % 12) as usize]; // E
        let fifth_dn = NOTES[((config.key_root + 5) % 12) as usize]; // D
        assert_eq!(fifth_up, "E");
        assert_eq!(fifth_dn, "D");
    }

    #[test]
    fn test_all_layers_covered() {
        assert_eq!(TranceLayer::all().len(), 15);
    }
}
