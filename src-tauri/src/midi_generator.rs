//! Trance lead MIDI generator — creates MIDI files on the fly.
//!
//! Five lead types learned from professional trance MIDI patterns:
//! - **TwoLayer**: octave-cycling bass arp with melody overlay (poly 2-3)
//! - **Zigzag**: monophonic 4-note cells (root-root-melody-melody)
//! - **Bounce**: mixed 8th/16th with rhythmic gaps
//! - **Cell**: mixed-duration repeating cells
//! - **Shuffle**: swing-feel with non-standard durations (42t/18t)

use rand::rngs::StdRng;
use rand::{RngExt, SeedableRng};
use serde::{Deserialize, Serialize};

const PPQN: u32 = 96;
const S: u32 = 24; // sixteenth
const E: u32 = 48; // eighth
const DE: u32 = 72; // dotted eighth
const BAR: u32 = 384; // 4/4 bar
const LONG: u32 = 42; // swing long (0.438q)
const SHORT: u32 = 18; // swing short (0.188q)

const MINOR_SCALE: [u8; 7] = [0, 2, 3, 5, 7, 8, 10];
const MAJOR_SCALE: [u8; 7] = [0, 2, 4, 5, 7, 9, 11];

// ── Public types ─────────────────────────────────────────────────────

#[derive(Deserialize, Serialize, Clone, Copy, PartialEq, Debug)]
#[serde(rename_all = "snake_case")]
pub enum LeadType {
    TwoLayer,
    Zigzag,
    Bounce,
    Cell,
    Shuffle,
    /// Chord-tone arpeggiator: root→5th→melody cycling through triads (Airbase style).
    ChordArp,
    /// Full chord stabs with rhythmic gating — stuttered durations per hit.
    GatedStab,
    /// Sustained pad chords — wide voicings, whole/double-whole notes.
    PadChord,
    /// Sustained root notes — monophonic, whole/double-whole. The bass foundation.
    DeepBass,
    /// Fast monophonic root patterns — 16th/8th with octave jumps.
    SubBass,
    /// Full chord on every 8th note — the signature progressive trance sound.
    Progressive,
    /// Fast 32nd-note runs/trills — mono or duo, used for fills and energy.
    Trill,
    /// Slow melodic lines — quarter/half notes, vocal-like phrasing.
    SlowMelody,
    /// Full chords on every 16th note — thicker than Progressive.
    ChordPluck,
    /// Mixed-duration polyphonic — piano/orchestral feel, varied rhythms.
    PianoChord,
    /// Octave-doubled melody — same line at +12 and/or +24 simultaneously.
    Unison,
}

impl LeadType {
    fn short_name(self) -> &'static str {
        match self {
            Self::TwoLayer => "TwoLayer",
            Self::Zigzag => "Zigzag",
            Self::Bounce => "Bounce",
            Self::Cell => "Cell",
            Self::Shuffle => "Shuffle",
            Self::ChordArp => "ChordArp",
            Self::GatedStab => "GatedStab",
            Self::PadChord => "PadChord",
            Self::DeepBass => "DeepBass",
            Self::SubBass => "SubBass",
            Self::Progressive => "Progressive",
            Self::Trill => "Trill",
            Self::SlowMelody => "SlowMelody",
            Self::ChordPluck => "ChordPluck",
            Self::PianoChord => "PianoChord",
            Self::Unison => "Unison",
        }
    }
}

const NOTE_NAMES: [&str; 12] = ["C","Cs","D","Ds","E","F","Fs","G","Gs","A","As","B"];
const NOTE_DISPLAY: [&str; 12] = ["C","C#","D","D#","E","F","F#","G","G#","A","A#","B"];

/// Build a descriptive filename like `Am_TwoLayer_4chords_8bars_140bpm_seed42`.
/// Does NOT include the `.mid` extension or variation number.
pub fn build_base_name(config: &MidiGenConfig) -> String {
    let chords = resolve_chords(config);
    let bpc = config.bars_per_chord.max(1) as u32;
    let total_bars = if let Some(lb) = config.length_bars {
        lb
    } else {
        chords.len() as u32 * bpc
    };
    let key = NOTE_NAMES[config.key_root as usize % 12];
    let scale = if config.minor { "m" } else { "" };
    let lt = config.lead_type.short_name();
    format!("{key}{scale}_{lt}_{total_bars}bars_{bpm}bpm_seed{seed}",
        bpm = config.bpm, seed = config.seed)
}

/// Build the full filename for variation `i` of `n` total.
pub fn build_filename(config: &MidiGenConfig, i: usize, n: usize) -> String {
    let base = build_base_name(config);
    if n == 1 {
        format!("{base}.mid")
    } else {
        format!("{base}_{:02}.mid", i + 1)
    }
}

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MidiGenConfig {
    /// Key root pitch class: 0=C, 1=C#, 2=D … 11=B.
    pub key_root: u8,
    /// `true` = minor, `false` = major.
    pub minor: bool,
    /// Lead pattern type.
    pub lead_type: LeadType,
    /// Chord progression as semitone offsets from `key_root`.
    /// Ignored when `progression` is provided.
    #[serde(default)]
    pub chords: Vec<i8>,
    /// Chord progression as chord name strings: `["Am","Dm","Em","C"]`.
    /// Converted to semitone offsets internally. Takes precedence over `chords`.
    #[serde(default)]
    pub progression: Vec<String>,
    /// Tempo stored in the MIDI header (120 = DAW-template speed).
    pub bpm: u16,
    /// Number of bars per chord (typically 2). Total bars = chords.len() × bars_per_chord.
    /// For `Cell` type this is halved internally (1 bar per chord, repeated).
    pub bars_per_chord: u8,
    /// Total length in bars. When set, overrides the computed length
    /// (chords × bars_per_chord) — the progression repeats/truncates to fill.
    #[serde(default)]
    pub length_bars: Option<u32>,
    /// How much the melody goes outside the key. 0 = strictly in key,
    /// 100 = maximum chromatic passing tones. Default 15.
    #[serde(default = "default_chromaticism")]
    pub chromaticism: u8,
    /// RNG seed — identical seed + config = identical output.
    pub seed: u64,
    /// Optional track name embedded in the MIDI meta event.
    pub name: Option<String>,
    /// How many variations to generate (each gets seed + i). Default 1.
    pub variations: Option<u16>,
}

fn default_chromaticism() -> u8 {
    15
}

/// Parse a chord name like "Am", "C", "F#m", "Dm", "Bbm", "G" into a pitch class (0–11).
/// Returns `None` for unrecognized strings.
fn parse_chord_name(s: &str) -> Option<u8> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    // Strip trailing quality indicators: "m", "min", "minor", "maj", "major"
    let base = s
        .trim_end_matches("minor")
        .trim_end_matches("major")
        .trim_end_matches("min")
        .trim_end_matches("maj")
        .trim_end_matches('m')
        .trim();
    const NAMES: &[(&str, u8)] = &[
        ("C", 0), ("C#", 1), ("Db", 1), ("D", 2), ("D#", 3), ("Eb", 3),
        ("E", 4), ("F", 5), ("F#", 6), ("Gb", 6), ("G", 7), ("G#", 8),
        ("Ab", 8), ("A", 9), ("A#", 10), ("Bb", 10), ("B", 11), ("Cb", 11),
    ];
    // Try longest match first (C# before C)
    for &(name, pc) in NAMES.iter().rev() {
        if base.eq_ignore_ascii_case(name) {
            return Some(pc);
        }
    }
    None
}

/// Resolve the effective chord list: `progression` names → offsets, or raw `chords`.
pub fn resolve_chords(config: &MidiGenConfig) -> Vec<i8> {
    if !config.progression.is_empty() {
        config
            .progression
            .iter()
            .filter_map(|name| {
                parse_chord_name(name).map(|pc| {
                    ((pc as i16 - config.key_root as i16).rem_euclid(12)) as i8
                })
            })
            .collect()
    } else {
        config.chords.clone()
    }
}

// ── Public API ───────────────────────────────────────────────────────

/// Resolve progression names + length_bars into a concrete chord vec.
fn resolve_config(config: &MidiGenConfig) -> MidiGenConfig {
    let mut resolved = config.clone();
    let chords = resolve_chords(config);

    // If length_bars is set, repeat/truncate chords to fill.
    if let Some(total) = config.length_bars {
        let bpc = config.bars_per_chord.max(1) as u32;
        let needed = ((total + bpc - 1) / bpc) as usize; // ceil
        if !chords.is_empty() && needed > 0 {
            resolved.chords = chords.iter().copied().cycle().take(needed).collect();
        } else {
            resolved.chords = chords;
        }
    } else {
        resolved.chords = chords;
    }
    resolved.progression.clear(); // already resolved
    resolved
}

/// Build a descriptive track name embedded in the MIDI meta event.
fn build_track_name(config: &MidiGenConfig, i: u64, n: u64) -> String {
    let key = NOTE_DISPLAY[config.key_root as usize % 12];
    let scale = if config.minor { "m" } else { "" };
    let lt = config.lead_type.short_name();
    if n == 1 {
        format!("{key}{scale} {lt}")
    } else {
        format!("{key}{scale} {lt} {:02}", i + 1)
    }
}

/// Generate one MIDI file and return its bytes.
pub fn generate(config: &MidiGenConfig) -> Result<Vec<u8>, String> {
    let config = resolve_config(config);
    validate(&config)?;
    let mut rng = StdRng::seed_from_u64(config.seed);
    let scale_pcs = build_scale_pcs(config.key_root, config.minor);
    let events = dispatch(&config, &scale_pcs, &mut rng);
    let name = build_track_name(&config, 0, 1);
    Ok(build_midi_file(&name, &config, &events))
}

/// Generate N variations, each with `seed + i`. Returns vec of MIDI byte arrays.
pub fn generate_batch(config: &MidiGenConfig) -> Result<Vec<Vec<u8>>, String> {
    let config = resolve_config(config);
    validate(&config)?;
    let n = config.variations.unwrap_or(1).max(1) as u64;
    let scale_pcs = build_scale_pcs(config.key_root, config.minor);
    let mut out = Vec::with_capacity(n as usize);
    for i in 0..n {
        let mut rng = StdRng::seed_from_u64(config.seed.wrapping_add(i));
        let events = dispatch(&config, &scale_pcs, &mut rng);
        let name = build_track_name(&config, i, n);
        out.push(build_midi_file(&name, &config, &events));
    }
    Ok(out)
}

// ── Kit generation ──────────────────────────────────────────────────

/// Which layers to include in a kit.
const KIT_LAYERS: &[(LeadType, &str)] = &[
    (LeadType::TwoLayer, "Lead TwoLayer"),
    (LeadType::ChordArp, "Lead ChordArp"),
    (LeadType::Unison, "Lead Unison"),
    (LeadType::Progressive, "Progressive"),
    (LeadType::ChordPluck, "Chord Pluck"),
    (LeadType::PadChord, "Pad"),
    (LeadType::PianoChord, "Piano"),
    (LeadType::GatedStab, "Chord Stab"),
    (LeadType::SlowMelody, "Melody"),
    (LeadType::Trill, "Trill"),
    (LeadType::DeepBass, "Deep Bass"),
    (LeadType::SubBass, "Sub Bass"),
];

/// Configuration for generating full kits (multiple layers per kit).
#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct KitGenConfig {
    /// Key root pitch class: 0=C … 11=B.
    pub key_root: u8,
    /// true = minor, false = major.
    pub minor: bool,
    /// Chord progression as chord name strings.
    #[serde(default)]
    pub progression: Vec<String>,
    /// Fallback: semitone offsets.
    #[serde(default)]
    pub chords: Vec<i8>,
    /// Tempo.
    pub bpm: u16,
    /// Bars per chord.
    pub bars_per_chord: u8,
    /// Total length override.
    #[serde(default)]
    pub length_bars: Option<u32>,
    /// Chromaticism: 0 = strictly in key, 100 = maximum out-of-key notes.
    #[serde(default = "default_chromaticism")]
    pub chromaticism: u8,
    /// Base seed — each kit gets seed + kit_index.
    pub seed: u64,
    /// Number of kits to generate.
    pub num_kits: u16,
    /// Which layer types to include. Empty = all from KIT_LAYERS.
    #[serde(default)]
    pub layers: Vec<LeadType>,
}

/// Info about one generated file.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KitFileInfo {
    pub path: String,
    pub layer: String,
    pub size: usize,
}

/// Info about one generated kit.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KitInfo {
    pub dir: String,
    pub name: String,
    pub files: Vec<KitFileInfo>,
}

/// Generate N kits, each in its own subdirectory with all layer MIDI files.
///
/// Directory structure:
/// ```text
/// output_dir/
///   Kit 1 (Am) (140 BPM)/
///     Lead TwoLayer.mid
///     Lead ChordArp.mid
///     Progressive.mid
///     Pad.mid
///     Chord Stab.mid
///     Deep Bass.mid
///     Sub Bass.mid
///   Kit 2 (Am) (140 BPM)/
///     ...
/// ```
pub fn generate_kits(
    config: &KitGenConfig,
    output_dir: &std::path::Path,
) -> Result<Vec<KitInfo>, String> {
    if config.key_root > 11 {
        return Err("key_root must be 0–11".into());
    }
    if config.chords.is_empty() && config.progression.is_empty() {
        return Err("chords or progression must not be empty".into());
    }
    if config.num_kits == 0 {
        return Err("num_kits must be >= 1".into());
    }

    let layers: Vec<(LeadType, &str)> = if config.layers.is_empty() {
        KIT_LAYERS.to_vec()
    } else {
        config
            .layers
            .iter()
            .map(|lt| {
                KIT_LAYERS
                    .iter()
                    .find(|(t, _)| t == lt)
                    .copied()
                    .unwrap_or((*lt, lt.short_name()))
            })
            .collect()
    };

    let key_display = NOTE_DISPLAY[config.key_root as usize % 12];
    let scale_suffix = if config.minor { "m" } else { "" };

    // Pack directory: "Trance MIDI Pack Am 140BPM 2026-04-17"
    let ts = chrono::Local::now().format("%Y-%m-%d_%H%M%S").to_string();
    let pack_name = format!(
        "Trance MIDI Pack {}{} {}BPM {}",
        key_display, scale_suffix, config.bpm, ts
    );
    let pack_dir = output_dir.join(&pack_name);
    std::fs::create_dir_all(&pack_dir).map_err(|e| e.to_string())?;

    let mut kits = Vec::with_capacity(config.num_kits as usize);

    for kit_i in 0..config.num_kits {
        let kit_seed = config.seed.wrapping_add(kit_i as u64 * 1000);
        let kit_name = format!(
            "Kit {} ({}{}) ({} BPM)",
            kit_i + 1,
            key_display,
            scale_suffix,
            config.bpm
        );
        let kit_dir = pack_dir.join(&kit_name);
        std::fs::create_dir_all(&kit_dir).map_err(|e| e.to_string())?;

        let mut files = Vec::new();

        for &(lead_type, layer_name) in &layers {
            let midi_cfg = MidiGenConfig {
                key_root: config.key_root,
                minor: config.minor,
                lead_type,
                chords: config.chords.clone(),
                progression: config.progression.clone(),
                bpm: config.bpm,
                bars_per_chord: config.bars_per_chord,
                length_bars: config.length_bars,
                chromaticism: config.chromaticism,
                seed: kit_seed,
                name: Some(layer_name.to_string()),
                variations: Some(1),
            };

            let bytes = generate(&midi_cfg)?;
            let filename = format!("{layer_name}.mid");
            let path = kit_dir.join(&filename);
            std::fs::write(&path, &bytes).map_err(|e| e.to_string())?;

            files.push(KitFileInfo {
                path: path.to_string_lossy().into(),
                layer: layer_name.to_string(),
                size: bytes.len(),
            });
        }

        kits.push(KitInfo {
            dir: kit_dir.to_string_lossy().into(),
            name: kit_name,
            files,
        });
    }

    Ok(kits)
}

// ── Validation ───────────────────────────────────────────────────────

fn validate(config: &MidiGenConfig) -> Result<(), String> {
    if config.key_root > 11 {
        return Err("key_root must be 0–11".into());
    }
    if config.chords.is_empty() && config.progression.is_empty() {
        return Err("chords or progression must not be empty".into());
    }
    if config.bpm == 0 || config.bpm > 300 {
        return Err("bpm must be 1–300".into());
    }
    if config.bars_per_chord == 0 {
        return Err("bars_per_chord must be >= 1".into());
    }
    Ok(())
}

// ── Dispatch ─────────────────────────────────────────────────────────

fn dispatch(config: &MidiGenConfig, pcs: &[u8], rng: &mut StdRng) -> Vec<NoteEvent> {
    match config.lead_type {
        LeadType::TwoLayer => gen_two_layer(config, pcs, rng),
        LeadType::Zigzag => gen_zigzag(config, pcs, rng),
        LeadType::Bounce => gen_bounce(config, pcs, rng),
        LeadType::Cell => gen_cell(config, pcs, rng),
        LeadType::Shuffle => gen_shuffle(config, pcs, rng),
        LeadType::ChordArp => gen_chord_arp(config, pcs, rng),
        LeadType::GatedStab => gen_gated_stab(config, pcs, rng),
        LeadType::PadChord => gen_pad_chord(config, pcs, rng),
        LeadType::DeepBass => gen_deep_bass(config, pcs, rng),
        LeadType::SubBass => gen_sub_bass(config, pcs, rng),
        LeadType::Progressive => gen_progressive(config, pcs, rng),
        LeadType::Trill => gen_trill(config, pcs, rng),
        LeadType::SlowMelody => gen_slow_melody(config, pcs, rng),
        LeadType::ChordPluck => gen_chord_pluck(config, pcs, rng),
        LeadType::PianoChord => gen_piano_chord(config, pcs, rng),
        LeadType::Unison => gen_unison(config, pcs, rng),
    }
}

// ── Note event ───────────────────────────────────────────────────────

struct NoteEvent {
    tick: u32,
    pitch: u8,
    vel: u8,
    dur: u32,
}

// ── Scale / chord helpers ────────────────────────────────────────────

fn build_scale_pcs(root: u8, minor: bool) -> Vec<u8> {
    let intervals = if minor { &MINOR_SCALE } else { &MAJOR_SCALE };
    intervals.iter().map(|&i| (root + i) % 12).collect()
}

/// All MIDI notes whose pitch class is in `pcs`, between `lo` and `hi` inclusive.
fn scale_notes_in_range(pcs: &[u8], lo: u8, hi: u8) -> Vec<u8> {
    (lo..=hi)
        .filter(|&n| pcs.contains(&(n % 12)))
        .collect()
}

/// Chord tones (root + diatonic 3rd + 5th) in a MIDI range.
fn chord_tones_in_range(pcs: &[u8], chord_pc: u8, lo: u8, hi: u8) -> Vec<u8> {
    let triad_pcs: Vec<u8> = if let Some(ri) = pcs.iter().position(|&p| p == chord_pc) {
        vec![pcs[ri], pcs[(ri + 2) % pcs.len()], pcs[(ri + 4) % pcs.len()]]
    } else {
        // Root outside scale — allow root, m3, M3, P5.
        vec![
            chord_pc,
            (chord_pc + 3) % 12,
            (chord_pc + 4) % 12,
            (chord_pc + 7) % 12,
        ]
    };
    (lo..=hi)
        .filter(|&n| triad_pcs.contains(&(n % 12)))
        .collect()
}

fn chord_root_pc(key_root: u8, offset: i8) -> u8 {
    ((key_root as i16 + offset as i16).rem_euclid(12)) as u8
}

/// Lowest MIDI note for the bass arp (octave 2 = MIDI 36–47).
fn bass_midi(pc: u8) -> u8 {
    pc + 36
}

/// Move `steps` scale degrees through `scale_notes`. Clamps at boundaries.
fn step_in_scale(notes: &[u8], pitch: u8, steps: i8) -> u8 {
    if notes.is_empty() {
        return pitch;
    }
    let idx = nearest_idx(notes, pitch);
    let new = (idx as i32 + steps as i32).clamp(0, notes.len() as i32 - 1) as usize;
    notes[new]
}

fn nearest_idx(notes: &[u8], pitch: u8) -> usize {
    notes
        .iter()
        .enumerate()
        .min_by_key(|&(_, n)| (*n as i16 - pitch as i16).abs())
        .map(|(i, _)| i)
        .unwrap_or(0)
}

fn nearest_in_set(target: u8, set: &[u8]) -> u8 {
    set.iter()
        .copied()
        .min_by_key(|&n| (n as i16 - target as i16).abs())
        .unwrap_or(target)
}

/// Return a chromatic neighbor that is NOT in the scale (for tension).
fn chromatic_neighbor(pitch: u8, pcs: &[u8], rng: &mut StdRng) -> u8 {
    let up = pitch.saturating_add(1).min(127);
    let dn = pitch.saturating_sub(1);
    let up_outside = !pcs.contains(&(up % 12));
    let dn_outside = !pcs.contains(&(dn % 12));
    match (up_outside, dn_outside) {
        (true, false) => up,
        (false, true) => dn,
        _ => {
            if rng.random_bool(0.5) {
                up
            } else {
                dn
            }
        }
    }
}

/// MIDI key-signature sf value (negative = flats, positive = sharps).
fn key_sig_sf(root: u8, minor: bool) -> i8 {
    const MAJOR_SF: [i8; 12] = [
        0,  // C
        7,  // C# / Db → 7 sharps (or -5 flats; MIDI allows either)
        2,  // D
        -3, // Eb
        4,  // E
        -1, // F
        -6, // F# / Gb → -6 flats
        1,  // G
        -4, // Ab
        3,  // A
        -2, // Bb
        5,  // B
    ];
    let major_root = if minor { (root + 3) % 12 } else { root };
    MAJOR_SF[major_root as usize]
}

// ── Generators ───────────────────────────────────────────────────────

// ----- TwoLayer --------------------------------------------------

/// Pick a bass octave pattern — varies per variation via RNG.
fn random_bass_pattern(rng: &mut StdRng) -> [u8; 16] {
    const PATTERNS: [[u8; 16]; 5] = [
        [0,12,24,0, 12,0,12,24, 0,12,0,12, 0,12,24,0],
        [0,12,0,24, 12,24,0,12, 0,12,0,24, 12,0,12,24],
        [0,24,12,0, 12,0,24,12, 0,24,12,0, 12,0,24,0],
        [0,12,0,12, 24,0,12,0, 12,24,0,12, 0,12,0,24],
        [0,12,24,12, 0,12,0,24, 12,0,12,24, 0,24,12,0],
    ];
    PATTERNS[rng.random_range(0..PATTERNS.len())]
}

fn gen_two_layer(cfg: &MidiGenConfig, pcs: &[u8], rng: &mut StdRng) -> Vec<NoteEvent> {
    let vel = 104u8;
    let mel_lo = 65u8;
    let mel_hi = 87u8;
    let scale_mel = scale_notes_in_range(pcs, mel_lo, mel_hi);
    let bpc = cfg.bars_per_chord as u32;
    let bass_pat = random_bass_pattern(rng);

    let mut events = Vec::new();
    let mut prev: Option<u8> = None;
    let n = cfg.chords.len();

    for (ci, &off) in cfg.chords.iter().enumerate() {
        let cpc = chord_root_pc(cfg.key_root, off);
        let bass = bass_midi(cpc);
        let ct = chord_tones_in_range(pcs, cpc, mel_lo, mel_hi);
        let ascending = ci < n / 2 || n == 1;

        let melody = two_layer_melody(rng, &scale_mel, &ct, pcs, prev, ascending, cfg.chromaticism);

        for bar in 0..bpc {
            let t = (ci as u32 * bpc + bar) * BAR;
            for i in 0..16u32 {
                events.push(NoteEvent {
                    tick: t + i * S,
                    pitch: bass + bass_pat[i as usize],
                    vel,
                    dur: S,
                });
            }
            for &(pos, pitch, dur) in &melody {
                events.push(NoteEvent {
                    tick: t + pos as u32 * S,
                    pitch,
                    vel,
                    dur,
                });
            }
        }
        prev = melody.last().map(|m| m.1);
    }
    events
}

fn two_layer_melody(
    rng: &mut StdRng,
    scale: &[u8],
    chord_t: &[u8],
    pcs: &[u8],
    prev: Option<u8>,
    ascending: bool,
    chromaticism: u8,
) -> Vec<(u8, u8, u32)> {
    let ct = if chord_t.is_empty() { scale } else { chord_t };
    if ct.is_empty() {
        return Vec::new();
    }

    // Randomize starting pitch within chord tones (not always the same one)
    let start = match prev {
        Some(p) => {
            // Pick a chord tone near prev, but with some randomness
            let near = nearest_in_set(p, ct);
            let jitter = rng.random_range(-2..=2_i8);
            step_in_scale(scale, near, jitter)
        }
        None => {
            // Random chord tone, not always the middle
            let idx = rng.random_range(0..ct.len());
            ct[idx]
        }
    };

    // Randomize direction — mostly follow `ascending` but sometimes flip
    let dir: i8 = if rng.random_ratio(75, 100) {
        if ascending { 1 } else { -1 }
    } else {
        if ascending { -1 } else { 1 }
    };

    // Long notes: varied intervals
    let p0 = start;
    let p1 = step_in_scale(scale, p0, dir * rng.random_range(1..=4_i8));
    let p2 = step_in_scale(scale, p1, -dir * rng.random_range(1..=5_i8));

    // Short notes: mix of directions, not just straight ascending
    let mut p = p2;
    let mut shorts = Vec::with_capacity(5);
    for i in 0..5 {
        // Vary direction within the run
        let step = match rng.random_range(0..10_u8) {
            0..=4 => dir,           // 50% continue in main direction
            5..=7 => -dir,          // 30% reverse
            _ => dir * 2,           // 20% leap in main direction
        };
        p = step_in_scale(scale, p, step);
        if chromaticism > 0 && rng.random_ratio(chromaticism as u32, 100) {
            p = chromatic_neighbor(p, pcs, rng);
        }
        // Occasionally land on a chord tone for stability
        if i == 2 && rng.random_ratio(40, 100) {
            p = nearest_in_set(p, ct);
        }
        shorts.push(p);
    }

    // Randomize the rhythmic template — vary which positions get melody
    // and which durations they use
    let templates: &[&[(u8, u32)]] = &[
        // Original: 3 long + 5 short
        &[(0,DE),(3,DE),(6,E),(8,S),(9,S),(10,S),(11,S),(13,S)],
        // Variant: 2 long + 4 short, different positions
        &[(0,DE),(4,DE),(8,S),(9,S),(10,S),(11,S),(13,S),(14,S)],
        // Variant: syncopated longs
        &[(1,DE),(3,E),(6,DE),(9,S),(10,S),(11,S),(12,S),(14,S)],
        // Variant: more short notes, punchier
        &[(0,E),(2,S),(3,S),(5,E),(8,S),(9,S),(10,S),(12,S)],
        // Variant: heavy on beat 3-4
        &[(0,DE),(3,DE),(8,S),(9,S),(10,S),(11,S),(12,S),(13,S)],
    ];
    let tmpl = templates[rng.random_range(0..templates.len())];

    let pitches = [p0, p1, p2, shorts[0], shorts[1], shorts[2], shorts[3], shorts[4]];

    tmpl.iter()
        .enumerate()
        .map(|(i, &(pos, dur))| (pos, pitches[i.min(pitches.len() - 1)], dur))
        .collect()
}

// ----- Zigzag ----------------------------------------------------

fn gen_zigzag(cfg: &MidiGenConfig, pcs: &[u8], rng: &mut StdRng) -> Vec<NoteEvent> {
    let vel = 122u8;
    let mel_lo = 72u8;
    let mel_hi = 89u8;
    let scale_mel = scale_notes_in_range(pcs, mel_lo, mel_hi);
    let bpc = cfg.bars_per_chord as u32;

    let mut events = Vec::new();

    for (ci, &off) in cfg.chords.iter().enumerate() {
        let cpc = chord_root_pc(cfg.key_root, off);
        let root_lo = bass_midi(cpc);

        // Pick 4 melody pairs per chord — randomize interval and direction
        let ct = chord_tones_in_range(pcs, cpc, mel_lo, mel_hi);
        let start = if !ct.is_empty() {
            ct[rng.random_range(0..ct.len())]
        } else if !scale_mel.is_empty() {
            scale_mel[rng.random_range(0..scale_mel.len())]
        } else {
            mel_lo
        };
        let mut cursor = start;
        let pairs: Vec<(u8, u8)> = (0..4)
            .map(|_| {
                let m1 = cursor;
                let step = rng.random_range(-2..=3_i8);
                let m2 = step_in_scale(&scale_mel, m1, step);
                // Move cursor for next cell — varied jumps
                cursor = step_in_scale(&scale_mel, m2, rng.random_range(-1..=2_i8));
                (m1, m2)
            })
            .collect();

        // Randomize which octave the root bass notes use
        let lo_oct = if rng.random_ratio(30, 100) { 12u8 } else { 0 };
        let hi_oct = lo_oct + 12;

        for bar in 0..bpc {
            let t = (ci as u32 * bpc + bar) * BAR;
            for cell in 0..4u32 {
                let (m1, m2) = pairs[cell as usize];
                let base = t + cell * 4 * S;
                events.push(NoteEvent { tick: base, pitch: root_lo + lo_oct, vel, dur: S });
                events.push(NoteEvent { tick: base + S, pitch: root_lo + hi_oct, vel, dur: S });
                events.push(NoteEvent { tick: base + 2 * S, pitch: m1, vel, dur: S });
                events.push(NoteEvent { tick: base + 3 * S, pitch: m2, vel, dur: S });
            }
        }
    }
    events
}

// ----- Bounce ----------------------------------------------------

fn gen_bounce(cfg: &MidiGenConfig, pcs: &[u8], rng: &mut StdRng) -> Vec<NoteEvent> {
    let vel = 122u8;
    let mel_lo = 72u8;
    let mel_hi = 89u8;
    let scale_mel = scale_notes_in_range(pcs, mel_lo, mel_hi);
    let bpc = cfg.bars_per_chord as u32;

    let mut events = Vec::new();

    for (ci, &off) in cfg.chords.iter().enumerate() {
        let cpc = chord_root_pc(cfg.key_root, off);
        let b3 = cpc + 48;
        let b2 = cpc + 36;
        let ct = chord_tones_in_range(pcs, cpc, mel_lo, mel_hi);
        // Randomize melody notes per chord — pick from chord tones with jitter
        let ma = if !ct.is_empty() {
            ct[rng.random_range(0..ct.len())]
        } else {
            scale_mel.get(rng.random_range(0..scale_mel.len().max(1))).copied().unwrap_or(mel_lo + 6)
        };
        let mb = step_in_scale(&scale_mel, ma, rng.random_range(-3..=-1_i8));
        let mc = step_in_scale(&scale_mel, ma, rng.random_range(2..=6_i8));
        // Occasional extra melody note for variation
        let md = step_in_scale(&scale_mel, mc, rng.random_range(-2..=1_i8));

        for bar in 0..bpc {
            let t = (ci as u32 * bpc + bar) * BAR;
            events.push(NoteEvent { tick: t, pitch: b3, vel, dur: E });
            events.push(NoteEvent { tick: t, pitch: ma, vel, dur: E });
            events.push(NoteEvent { tick: t + 2 * S, pitch: mb, vel, dur: S });
            events.push(NoteEvent { tick: t + 3 * S, pitch: b3, vel, dur: E });
            events.push(NoteEvent { tick: t + 5 * S, pitch: if rng.random_ratio(60, 100) { ma } else { md }, vel, dur: S });
            events.push(NoteEvent { tick: t + 6 * S, pitch: b2, vel, dur: E });
            events.push(NoteEvent { tick: t + 7 * S, pitch: if rng.random_ratio(70, 100) { mb } else { ma }, vel, dur: S });
            events.push(NoteEvent { tick: t + 8 * S, pitch: b3, vel, dur: E });
            events.push(NoteEvent { tick: t + 10 * S, pitch: mc, vel, dur: S });
            events.push(NoteEvent { tick: t + 11 * S, pitch: b3, vel, dur: E });
            events.push(NoteEvent { tick: t + 14 * S, pitch: b2, vel, dur: E });
            events.push(NoteEvent { tick: t + 14 * S, pitch: if rng.random_ratio(50, 100) { mc } else { md }, vel, dur: S });
        }
    }
    events
}

// ----- Cell ------------------------------------------------------

fn gen_cell(cfg: &MidiGenConfig, pcs: &[u8], rng: &mut StdRng) -> Vec<NoteEvent> {
    let vel = 122u8;
    let mel_lo = 65u8;
    let mel_hi = 85u8;
    let scale_mel = scale_notes_in_range(pcs, mel_lo, mel_hi);
    // Cell uses 1 bar per chord internally, repeated `bars_per_chord` times.
    let repeats = cfg.bars_per_chord.max(1) as u32;

    let mut events = Vec::new();
    let mut prev_start: Option<u8> = None;

    for rep in 0..repeats {
        for (ci, &off) in cfg.chords.iter().enumerate() {
            let cpc = chord_root_pc(cfg.key_root, off);
            let r2 = bass_midi(cpc);
            let r3 = r2 + 12;
            let ct = chord_tones_in_range(pcs, cpc, mel_lo, mel_hi);

            let start = match prev_start {
                Some(p) => nearest_in_set(p, if ct.is_empty() { &scale_mel } else { &ct }),
                None => ct.get(ct.len() / 3).copied().unwrap_or(mel_lo + 5),
            };

            // 8 melody pitches for 2 cells
            let mut mel = Vec::with_capacity(8);
            let mut p = start;
            for j in 0..8 {
                mel.push(p);
                let s = if j % 3 == 2 {
                    rng.random_range(-2..=0_i8)
                } else {
                    rng.random_range(1..=2_i8)
                };
                p = step_in_scale(&scale_mel, p, s);
            }
            prev_start = Some(mel[0]);

            let t_bar = (rep * cfg.chords.len() as u32 + ci as u32) * BAR;
            for cell in 0..2u32 {
                let t = t_bar + cell * 8 * S;
                let mi = (cell * 4) as usize;
                events.push(NoteEvent { tick: t, pitch: mel[mi], vel, dur: E });
                events.push(NoteEvent { tick: t + S, pitch: r3, vel, dur: S });
                events.push(NoteEvent { tick: t + 2 * S, pitch: r2, vel, dur: S });
                events.push(NoteEvent { tick: t + 3 * S, pitch: mel[mi + 1], vel, dur: E });
                events.push(NoteEvent { tick: t + 4 * S, pitch: r3, vel, dur: S });
                events.push(NoteEvent { tick: t + 5 * S, pitch: mel[mi + 2], vel, dur: S });
                events.push(NoteEvent { tick: t + 6 * S, pitch: mel[mi + 3], vel, dur: S });
                // Vary last note on repeats
                let last = if rep > 0 {
                    step_in_scale(&scale_mel, mel[mi + 2], rng.random_range(1..=3_i8))
                } else {
                    mel[mi + 2]
                };
                events.push(NoteEvent { tick: t + 7 * S, pitch: last, vel, dur: S });
            }
        }
    }
    events
}

// ----- Shuffle ---------------------------------------------------

fn gen_shuffle(cfg: &MidiGenConfig, pcs: &[u8], rng: &mut StdRng) -> Vec<NoteEvent> {
    let vel = 122u8;
    let mel_lo = 60u8;
    let mel_hi = 80u8;
    let scale_mel = scale_notes_in_range(pcs, mel_lo, mel_hi);
    let bpc = cfg.bars_per_chord as u32;

    let mut events = Vec::new();

    for (ci, &off) in cfg.chords.iter().enumerate() {
        let cpc = chord_root_pc(cfg.key_root, off);
        let b1 = bass_midi(cpc);
        let b2 = b1 + 12;
        let ct = chord_tones_in_range(pcs, cpc, mel_lo, mel_hi);
        let ma = nearest_in_set(ct.get(ct.len() / 2).copied().unwrap_or(mel_lo + 6), &scale_mel);
        let mb = step_in_scale(&scale_mel, ma, rng.random_range(1..=2_i8));
        let ml = step_in_scale(&scale_mel, ma, rng.random_range(-3..=-1_i8));

        for bar in 0..bpc {
            let t = (ci as u32 * bpc + bar) * BAR;
            // 16 sixteenth positions, bass alternating b1/b2, melody overlays
            events.push(NoteEvent { tick: t, pitch: b1, vel, dur: LONG });
            events.push(NoteEvent { tick: t, pitch: ma, vel, dur: E });
            events.push(NoteEvent { tick: t + S, pitch: b2, vel, dur: SHORT });
            events.push(NoteEvent { tick: t + 2 * S, pitch: b1, vel, dur: SHORT });
            events.push(NoteEvent { tick: t + 2 * S, pitch: mb, vel, dur: S });
            events.push(NoteEvent { tick: t + 3 * S, pitch: b2, vel, dur: LONG });
            events.push(NoteEvent { tick: t + 4 * S, pitch: ma, vel, dur: S });
            events.push(NoteEvent { tick: t + 5 * S, pitch: b1, vel, dur: SHORT });
            events.push(NoteEvent { tick: t + 5 * S, pitch: ml, vel, dur: S });
            events.push(NoteEvent { tick: t + 6 * S, pitch: b2, vel, dur: LONG });
            events.push(NoteEvent { tick: t + 7 * S, pitch: ml, vel, dur: S });
            events.push(NoteEvent { tick: t + 8 * S, pitch: b1, vel, dur: LONG });
            events.push(NoteEvent { tick: t + 9 * S, pitch: b2, vel, dur: SHORT });
            events.push(NoteEvent { tick: t + 9 * S, pitch: ma, vel, dur: S });
            events.push(NoteEvent { tick: t + 10 * S, pitch: b1, vel, dur: SHORT });
            events.push(NoteEvent { tick: t + 10 * S, pitch: mb, vel, dur: S });
            events.push(NoteEvent { tick: t + 11 * S, pitch: b2, vel, dur: LONG });
            events.push(NoteEvent { tick: t + 12 * S, pitch: ma, vel, dur: S });
            events.push(NoteEvent { tick: t + 13 * S, pitch: b1, vel, dur: SHORT });
            events.push(NoteEvent { tick: t + 14 * S, pitch: b2, vel, dur: LONG });
            events.push(NoteEvent { tick: t + 15 * S, pitch: ml, vel, dur: S });
        }
    }
    events
}

// ----- ChordArp --------------------------------------------------
// Airbase style: 3-voice chord arpeggio cycling root→5th→melody at 16ths.
// Each 16th plays one voice, swing durations (short/long alternating).
// Melody voice picks from chord tones. Root changes per chord.

fn gen_chord_arp(cfg: &MidiGenConfig, pcs: &[u8], rng: &mut StdRng) -> Vec<NoteEvent> {
    let vel = 100u8;
    let bpc = cfg.bars_per_chord as u32;

    let mut events = Vec::new();

    for (ci, &off) in cfg.chords.iter().enumerate() {
        let cpc = chord_root_pc(cfg.key_root, off);
        let root2 = cpc + 36; // octave 2
        let ct = chord_tones_in_range(pcs, cpc, 48, 72); // octave 3-5 for 5th/melody
        let fifth = if ct.len() >= 2 { ct[1] } else { root2 + 7 };
        let mel_notes = chord_tones_in_range(pcs, cpc, 60, 81); // melody range C4-A5

        // Pick 2-3 melody notes for this chord, varying per variation
        let mel_a = if !mel_notes.is_empty() {
            mel_notes[rng.random_range(0..mel_notes.len())]
        } else {
            cpc + 60
        };
        let mel_b = step_in_scale(
            &scale_notes_in_range(pcs, 60, 81),
            mel_a,
            rng.random_range(-2..=2_i8),
        );

        // Arp pattern per bar: 16 sixteenths cycling through root/5th/melody
        // with swing durations (short=18t, long=42t alternating)
        for bar in 0..bpc {
            let t = (ci as u32 * bpc + bar) * BAR;
            for i in 0..16u32 {
                let tick = t + i * S;
                let dur = if i % 2 == 0 { SHORT } else { LONG };
                // Cycle: root, 5th, melody_a, root, root_oct, 5th, melody_b, root...
                let pitch = match i % 8 {
                    0 => root2,
                    1 => fifth,
                    2 => mel_a,
                    3 => root2,
                    4 => root2 + 12,
                    5 => fifth,
                    6 => mel_b,
                    7 => root2,
                    _ => root2,
                };
                // Occasional melody variation
                let pitch = if i >= 12 && cfg.chromaticism > 0 && rng.random_ratio(cfg.chromaticism as u32, 100) {
                    step_in_scale(
                        &scale_notes_in_range(pcs, 55, 81),
                        pitch,
                        rng.random_range(-1..=1_i8),
                    )
                } else {
                    pitch
                };
                events.push(NoteEvent { tick, pitch, vel, dur });
            }
        }
    }
    events
}

// ----- GatedStab -------------------------------------------------
// Full chord stabs with rhythmic gating — all notes hit simultaneously
// but with varied durations (32nd/16th/8th/dotted-8th) per hit,
// creating a stuttered gate effect. Like Airbase Arp Pattern 05.

fn gen_gated_stab(cfg: &MidiGenConfig, pcs: &[u8], rng: &mut StdRng) -> Vec<NoteEvent> {
    let vel = 120u8;
    let bpc = cfg.bars_per_chord as u32;

    // Gate rhythm templates — each is 8 entries (half bar),
    // values are durations in ticks for the chord stab at that position
    const T: u32 = 12; // 32nd
    const GATE_TEMPLATES: [[u32; 8]; 5] = [
        // Template A: short-short-med-med-long-skip-short-short
        [T, T, S, S, DE, 0, T, T],
        // Template B: hit-hit-long-skip-hit-16th-8th-skip
        [T, T, DE, 0, T, S, E, 0],
        // Template C: stutter then sustain
        [T, T, T, T, T, 0, DE, E],
        // Template D: syncopated
        [S, 0, S, S, 0, DE, T, T],
        // Template E: heavy downbeat
        [E, T, T, S, DE, 0, T, T],
    ];

    let mut events = Vec::new();

    for (ci, &off) in cfg.chords.iter().enumerate() {
        let cpc = chord_root_pc(cfg.key_root, off);

        // Build a 4-5 note chord voicing spanning 2+ octaves
        let root2 = cpc + 36;
        let root3 = cpc + 48;
        let ct_mid = chord_tones_in_range(pcs, cpc, 55, 72);
        let ct_hi = chord_tones_in_range(pcs, cpc, 60, 81);

        let mut voicing: Vec<u8> = vec![root2, root3];
        if let Some(&p) = ct_mid.get(1) {
            voicing.push(p);
        }
        if let Some(&p) = ct_hi.first() {
            voicing.push(p);
        }
        if let Some(&p) = ct_hi.get(ct_hi.len().saturating_sub(1).max(1).min(ct_hi.len() - 1)) {
            if !voicing.contains(&p) {
                voicing.push(p);
            }
        }
        voicing.sort();
        voicing.dedup();

        // Pick gate template (randomized per chord)
        let tmpl = GATE_TEMPLATES[rng.random_range(0..GATE_TEMPLATES.len())];

        for bar in 0..bpc {
            let t = (ci as u32 * bpc + bar) * BAR;
            // Apply template twice per bar (first half, second half)
            for half in 0..2u32 {
                let base = t + half * 8 * S;
                for (pos, &dur) in tmpl.iter().enumerate() {
                    if dur == 0 {
                        continue; // skip = rest
                    }
                    let tick = base + pos as u32 * S;
                    // All chord notes hit simultaneously
                    for &pitch in &voicing {
                        events.push(NoteEvent { tick, pitch, vel, dur });
                    }
                }
            }
        }
    }
    events
}

// ----- PadChord --------------------------------------------------
// Sustained pad chords — wide voicings (root + octave + 3rd + 5th + extensions).
// Whole notes to double-whole, poly 4-6. Two sub-modes:
// - "short": clean whole-note chords, one per bar
// - "long": some notes sustain across bars (ties/suspensions)
// Chosen randomly per variation.

fn gen_pad_chord(cfg: &MidiGenConfig, pcs: &[u8], rng: &mut StdRng) -> Vec<NoteEvent> {
    let vel = 100u8;
    let bpc = cfg.bars_per_chord as u32;
    let use_long = rng.random_bool(0.5); // 50% long pads, 50% short pads

    let mut events = Vec::new();

    for (ci, &off) in cfg.chords.iter().enumerate() {
        let cpc = chord_root_pc(cfg.key_root, off);

        // Wide voicing: root in octave 2, then stack chord tones up to octave 5
        let root2 = cpc + 36;
        let root3 = cpc + 48;
        let scale_notes = scale_notes_in_range(pcs, 55, 79);
        let ct = chord_tones_in_range(pcs, cpc, 55, 79);

        // Build 4-6 note voicing
        let mut voicing: Vec<u8> = vec![root2, root3];

        // Add 5th
        let fifth = step_in_scale(&scale_notes, root3, 4);
        if !voicing.contains(&fifth) {
            voicing.push(fifth);
        }

        // Add chord tones in upper range
        for &p in &ct {
            if !voicing.contains(&p) && voicing.len() < 6 {
                voicing.push(p);
            }
        }

        // Occasionally add a scale tone for color (9th, 11th, etc.)
        if rng.random_ratio(30, 100) && !scale_notes.is_empty() {
            let color = scale_notes[rng.random_range(0..scale_notes.len())];
            if !voicing.contains(&color) && voicing.len() < 7 {
                voicing.push(color);
            }
        }

        voicing.sort();

        for bar in 0..bpc {
            let t = (ci as u32 * bpc + bar) * BAR;

            for &pitch in &voicing {
                let dur = if use_long {
                    // Long mode: some notes sustain 2-4 bars, some just 1
                    match rng.random_range(0..4_u8) {
                        0 => BAR * 2, // double-whole (sustain across next bar)
                        1 => BAR * bpc.min(4), // sustain the whole chord section
                        _ => BAR, // whole note
                    }
                } else {
                    // Short mode: clean whole notes, one per bar
                    BAR
                };
                // Clamp duration so it doesn't extend past the end of this chord's section
                let remaining = (bpc - bar) * BAR;
                let dur = dur.min(remaining);
                events.push(NoteEvent { tick: t, pitch, vel, dur });
            }

            // In short mode, only emit one bar of notes per chord
            // (they're all whole notes, no need to repeat the same chord)
            if !use_long {
                // But we DO want each bar to have the chord, so continue
            }
        }
    }
    events
}

// ----- DeepBass --------------------------------------------------
// Sustained monophonic root notes — whole or double-whole per chord.
// 90 kits in Trance Obsession use this: 54% whole, 28% double-whole.
// The simplest layer but essential for the low-end foundation.

fn gen_deep_bass(cfg: &MidiGenConfig, pcs: &[u8], rng: &mut StdRng) -> Vec<NoteEvent> {
    let vel = 100u8;
    let bpc = cfg.bars_per_chord as u32;
    let mut events = Vec::new();

    for (ci, &off) in cfg.chords.iter().enumerate() {
        let cpc = chord_root_pc(cfg.key_root, off);
        // Deep bass sits in octave 2 (MIDI 36-47)
        let root = cpc + 36;

        for bar in 0..bpc {
            let t = (ci as u32 * bpc + bar) * BAR;
            // Choose duration: whole note, or sustain across multiple bars
            let dur = if bpc > 1 && bar == 0 && rng.random_ratio(40, 100) {
                // 40% chance: sustain across all bars of this chord
                BAR * bpc
            } else if bar == 0 || !rng.random_ratio(40, 100) {
                BAR // whole note
            } else {
                continue; // skip — previous note is sustaining
            };
            let remaining = (bpc - bar) * BAR;
            events.push(NoteEvent {
                tick: t,
                pitch: root,
                vel,
                dur: dur.min(remaining),
            });
            // Occasionally add the 5th as a passing tone on beat 3
            if rng.random_ratio(20, 100) {
                let fifth = step_in_scale(&scale_notes_in_range(pcs, 36, 55), root, 4);
                events.push(NoteEvent {
                    tick: t + 2 * Q,
                    pitch: fifth,
                    vel: vel - 10,
                    dur: Q * 2, // half note
                });
            }
        }
    }
    events
}

// ----- SubBass ---------------------------------------------------
// Fast monophonic root patterns — 16ths with octave jumps and occasional
// 5th. Like the "Hi Bass" from Trance Obsession (97.5% 16ths).

fn gen_sub_bass(cfg: &MidiGenConfig, pcs: &[u8], rng: &mut StdRng) -> Vec<NoteEvent> {
    let vel = 110u8;
    let bpc = cfg.bars_per_chord as u32;
    let mut events = Vec::new();

    // Pick a rhythmic pattern for this variation
    // Each pattern is 16 entries: 0=root, 1=root+oct, 2=5th, 3=rest
    const PATTERNS: [[u8; 16]; 5] = [
        [0,0,1,0, 0,1,0,0, 1,0,0,1, 0,0,1,0],   // standard octave bounce
        [0,0,0,1, 0,0,1,0, 0,0,0,1, 0,1,0,0],   // sparse octave
        [0,1,0,1, 0,0,1,0, 1,0,1,0, 0,1,0,1],   // heavy octave
        [0,0,2,0, 0,0,1,0, 0,2,0,0, 1,0,0,2],   // with 5th
        [0,3,0,0, 1,3,0,0, 0,3,1,0, 0,3,0,1],   // gated (rests)
    ];
    let pat = PATTERNS[rng.random_range(0..PATTERNS.len())];

    for (ci, &off) in cfg.chords.iter().enumerate() {
        let cpc = chord_root_pc(cfg.key_root, off);
        let root = cpc + 36; // octave 2
        let root_hi = root + 12; // octave 3
        let scale = scale_notes_in_range(pcs, 36, 60);
        let fifth = step_in_scale(&scale, root, 4);

        for bar in 0..bpc {
            let t = (ci as u32 * bpc + bar) * BAR;
            for i in 0..16u32 {
                let voice = pat[i as usize];
                if voice == 3 { continue; } // rest
                let pitch = match voice {
                    0 => root,
                    1 => root_hi,
                    2 => fifth,
                    _ => root,
                };
                events.push(NoteEvent {
                    tick: t + i * S,
                    pitch,
                    vel,
                    dur: S,
                });
            }
        }
    }
    events
}

// ----- Progressive -----------------------------------------------
// Full chord on EVERY 8th note — the signature progressive trance sound.
// 80 kits in Trance Obsession, 99.8% 8ths, poly 4-8 (avg 5.3).
// Chord tones span octaves 2-5, chord changes per bar following progression.

fn gen_progressive(cfg: &MidiGenConfig, pcs: &[u8], rng: &mut StdRng) -> Vec<NoteEvent> {
    let vel = 105u8;
    let bpc = cfg.bars_per_chord as u32;
    let mut events = Vec::new();

    for (ci, &off) in cfg.chords.iter().enumerate() {
        let cpc = chord_root_pc(cfg.key_root, off);
        let root2 = cpc + 36;
        let root3 = cpc + 48;

        // Build a 4-6 note voicing for the full chord
        let ct_mid = chord_tones_in_range(pcs, cpc, 48, 67);
        let ct_hi = chord_tones_in_range(pcs, cpc, 60, 79);

        let mut voicing: Vec<u8> = vec![root2, root3];
        // Add 5th in octave 3
        let scale = scale_notes_in_range(pcs, 48, 72);
        let fifth = step_in_scale(&scale, root3, 4);
        if !voicing.contains(&fifth) { voicing.push(fifth); }
        // Add remaining chord tones
        for &p in ct_mid.iter().chain(ct_hi.iter()) {
            if !voicing.contains(&p) && voicing.len() < 6 { voicing.push(p); }
        }
        // Sometimes add a color tone for variation
        if rng.random_ratio(25, 100) {
            let sn = scale_notes_in_range(pcs, 60, 75);
            if !sn.is_empty() {
                let color = sn[rng.random_range(0..sn.len())];
                if !voicing.contains(&color) && voicing.len() < 7 {
                    voicing.push(color);
                }
            }
        }
        voicing.sort();

        for bar in 0..bpc {
            let t = (ci as u32 * bpc + bar) * BAR;
            // 8 eighth notes per bar, full chord on each
            for beat in 0..8u32 {
                let tick = t + beat * E;
                for &pitch in &voicing {
                    events.push(NoteEvent { tick, pitch, vel, dur: E });
                }
            }
        }
    }
    events
}

const Q: u32 = 96; // quarter note
const H: u32 = 192; // half note
const W: u32 = 384; // whole note (= BAR)
const T: u32 = 12; // 32nd note

// ----- Trill -----------------------------------------------------
// Fast 32nd-note runs/trills — mono or duo, used for fills and energy.

fn gen_trill(cfg: &MidiGenConfig, pcs: &[u8], rng: &mut StdRng) -> Vec<NoteEvent> {
    let vel = 110u8;
    let mel_lo = 60u8;
    let mel_hi = 84u8;
    let scale_mel = scale_notes_in_range(pcs, mel_lo, mel_hi);
    let bpc = cfg.bars_per_chord as u32;

    let mut events = Vec::new();
    let mut prev_pitch: Option<u8> = None;

    for (ci, &off) in cfg.chords.iter().enumerate() {
        let cpc = chord_root_pc(cfg.key_root, off);
        let ct = chord_tones_in_range(pcs, cpc, mel_lo, mel_hi);

        let start = match prev_pitch {
            Some(p) => step_in_scale(&scale_mel, p, rng.random_range(-2..=2_i8)),
            None => if !ct.is_empty() { ct[rng.random_range(0..ct.len())] } else { 72 },
        };

        for bar in 0..bpc {
            let t_bar = (ci as u32 * bpc + bar) * BAR;
            let mut p = start;

            match rng.random_range(0..3_u8) {
                0 => {
                    // Ascending run then descending
                    for i in 0..32u32 {
                        let step = if i < 16 { 1i8 } else { -1 };
                        p = step_in_scale(&scale_mel, p, step);
                        events.push(NoteEvent { tick: t_bar + i * T, pitch: p, vel, dur: T });
                    }
                }
                1 => {
                    // Trill: alternate two adjacent notes
                    let p2 = step_in_scale(&scale_mel, p, 1);
                    for i in 0..32u32 {
                        let pitch = if i % 2 == 0 { p } else { p2 };
                        events.push(NoteEvent { tick: t_bar + i * T, pitch, vel, dur: T });
                    }
                    p = step_in_scale(&scale_mel, p, rng.random_range(1..=3_i8));
                }
                _ => {
                    // Mixed: groups of 4 (run up 3, trill 1)
                    for group in 0..8u32 {
                        let base = t_bar + group * 4 * T;
                        for j in 0..3u32 {
                            p = step_in_scale(&scale_mel, p, 1);
                            events.push(NoteEvent { tick: base + j * T, pitch: p, vel, dur: T });
                        }
                        let tp = step_in_scale(&scale_mel, p, if rng.random_bool(0.5) { -1 } else { 0 });
                        events.push(NoteEvent { tick: base + 3 * T, pitch: tp, vel, dur: T });
                    }
                }
            }
            prev_pitch = Some(p);
        }
    }
    events
}

// ----- SlowMelody ------------------------------------------------
// Slow vocal-like melodic lines — quarter/half notes, stepwise.

fn gen_slow_melody(cfg: &MidiGenConfig, pcs: &[u8], rng: &mut StdRng) -> Vec<NoteEvent> {
    let vel = 100u8;
    let mel_lo = 60u8;
    let mel_hi = 81u8;
    let scale_mel = scale_notes_in_range(pcs, mel_lo, mel_hi);
    let bpc = cfg.bars_per_chord as u32;

    let mut events = Vec::new();
    let mut prev: Option<u8> = None;

    const RHYTHM_TEMPLATES: [[u32; 4]; 5] = [
        [2, 2, 2, 2],
        [1, 1, 3, 3],
        [3, 1, 1, 3],
        [4, 2, 1, 1],
        [1, 3, 1, 3],
    ];

    for (ci, &off) in cfg.chords.iter().enumerate() {
        let cpc = chord_root_pc(cfg.key_root, off);
        let ct = chord_tones_in_range(pcs, cpc, mel_lo, mel_hi);
        let ascending = ci < cfg.chords.len() / 2;

        let start = match prev {
            Some(p) => {
                let near = nearest_in_set(p, if ct.is_empty() { &scale_mel } else { &ct });
                step_in_scale(&scale_mel, near, rng.random_range(-1..=1_i8))
            }
            None => if !ct.is_empty() { ct[rng.random_range(0..ct.len())] } else { 69 },
        };

        let tmpl = RHYTHM_TEMPLATES[rng.random_range(0..RHYTHM_TEMPLATES.len())];
        let dir: i8 = if ascending { 1 } else { -1 };

        for bar in 0..bpc {
            let t_bar = (ci as u32 * bpc + bar) * BAR;
            let mut p = if bar == 0 { start } else { prev.unwrap_or(start) };
            let mut tick_offset = 0u32;

            for &dur_q in &tmpl {
                let dur = dur_q * Q;
                if tick_offset + dur > BAR { break; }
                events.push(NoteEvent { tick: t_bar + tick_offset, pitch: p, vel, dur });
                let step = match rng.random_range(0..10_u8) {
                    0..=4 => dir,
                    5..=7 => dir * 2,
                    8 => -dir,
                    _ => 0,
                };
                p = step_in_scale(&scale_mel, p, step);
                if cfg.chromaticism > 0 && rng.random_ratio(cfg.chromaticism as u32, 100) {
                    p = chromatic_neighbor(p, pcs, rng);
                }
                tick_offset += dur;
                prev = Some(p);
            }
        }
    }
    events
}

// ----- ChordPluck ------------------------------------------------
// Full chords on every 16th note — thicker/faster than Progressive.
// The "supersaw wall" sound. Poly 7-12.

fn gen_chord_pluck(cfg: &MidiGenConfig, pcs: &[u8], rng: &mut StdRng) -> Vec<NoteEvent> {
    let vel = 108u8;
    let bpc = cfg.bars_per_chord as u32;

    let mut events = Vec::new();

    for (ci, &off) in cfg.chords.iter().enumerate() {
        let cpc = chord_root_pc(cfg.key_root, off);
        let root2 = cpc + 36;
        let root3 = cpc + 48;

        let ct_lo = chord_tones_in_range(pcs, cpc, 36, 55);
        let ct_mid = chord_tones_in_range(pcs, cpc, 48, 67);
        let ct_hi = chord_tones_in_range(pcs, cpc, 60, 79);

        let mut voicing: Vec<u8> = vec![root2, root3];
        for &p in ct_lo.iter().chain(ct_mid.iter()).chain(ct_hi.iter()) {
            if !voicing.contains(&p) { voicing.push(p); }
        }
        let scale = scale_notes_in_range(pcs, 36, 79);
        let fifth = step_in_scale(&scale, root3, 4);
        if !voicing.contains(&fifth) { voicing.push(fifth); }
        for _ in 0..2 {
            if rng.random_ratio(30, 100) && !scale.is_empty() {
                let c = scale[rng.random_range(0..scale.len())];
                if !voicing.contains(&c) { voicing.push(c); }
            }
        }
        voicing.sort();

        for bar in 0..bpc {
            let t = (ci as u32 * bpc + bar) * BAR;
            for i in 0..16u32 {
                let tick = t + i * S;
                for &pitch in &voicing {
                    events.push(NoteEvent { tick, pitch, vel, dur: S });
                }
            }
        }
    }
    events
}

// ----- PianoChord ------------------------------------------------
// Mixed-duration polyphonic — piano/orchestral feel. Sustained bass
// notes with rhythmic chord hits and melodic fragments on top.

fn gen_piano_chord(cfg: &MidiGenConfig, pcs: &[u8], rng: &mut StdRng) -> Vec<NoteEvent> {
    let vel = 95u8;
    let bpc = cfg.bars_per_chord as u32;

    let mut events = Vec::new();

    for (ci, &off) in cfg.chords.iter().enumerate() {
        let cpc = chord_root_pc(cfg.key_root, off);
        let root3 = cpc + 48;
        let ct = chord_tones_in_range(pcs, cpc, 48, 79);
        let scale = scale_notes_in_range(pcs, 55, 84);

        let mut chord: Vec<u8> = vec![root3];
        for &p in &ct {
            if !chord.contains(&p) && chord.len() < 6 { chord.push(p); }
        }
        chord.sort();

        for bar in 0..bpc {
            let t = (ci as u32 * bpc + bar) * BAR;

            // Sustained bass root
            events.push(NoteEvent { tick: t, pitch: cpc + 36, vel, dur: BAR });

            match rng.random_range(0..4_u8) {
                0 => {
                    // Two half-note chord hits
                    for &p in &chord {
                        events.push(NoteEvent { tick: t, pitch: p, vel, dur: H });
                        events.push(NoteEvent { tick: t + H, pitch: p, vel: vel - 5, dur: H });
                    }
                }
                1 => {
                    // Syncopated chord hits
                    for &p in &chord {
                        events.push(NoteEvent { tick: t, pitch: p, vel, dur: Q });
                    }
                    for &p in &chord {
                        events.push(NoteEvent { tick: t + Q + E, pitch: p, vel: vel - 8, dur: E });
                    }
                    for &p in &chord {
                        events.push(NoteEvent { tick: t + 2*Q, pitch: p, vel: vel - 3, dur: Q });
                    }
                    for &p in &chord {
                        events.push(NoteEvent { tick: t + 3*Q, pitch: p, vel: vel - 5, dur: Q });
                    }
                }
                2 => {
                    // Whole chord + melodic fragment on top
                    for &p in &chord {
                        events.push(NoteEvent { tick: t, pitch: p, vel, dur: W });
                    }
                    let mut mp = chord.last().copied().unwrap_or(72);
                    for beat in 0..4u32 {
                        mp = step_in_scale(&scale, mp, rng.random_range(-2..=2_i8));
                        events.push(NoteEvent { tick: t + beat * Q, pitch: mp, vel: vel + 5, dur: Q });
                    }
                }
                _ => {
                    // 8th note chord stabs
                    for beat in 0..8u32 {
                        let v = if beat % 2 == 0 { vel } else { vel - 10 };
                        for &p in &chord {
                            events.push(NoteEvent { tick: t + beat * E, pitch: p, vel: v, dur: E });
                        }
                    }
                }
            }
        }
    }
    events
}

// ----- Unison ----------------------------------------------------
// Octave-doubled melody — same line at +12 and optionally +24.
// Creates the thick "supersaw lead" sound. Uses TwoLayer melody
// algorithm but doubles every note at octave intervals.

fn gen_unison(cfg: &MidiGenConfig, pcs: &[u8], rng: &mut StdRng) -> Vec<NoteEvent> {
    let vel = 105u8;
    let mel_lo = 55u8;
    let mel_hi = 79u8;
    let scale_mel = scale_notes_in_range(pcs, mel_lo, mel_hi);
    let bpc = cfg.bars_per_chord as u32;
    let bass_pat = random_bass_pattern(rng);

    let mut events = Vec::new();
    let mut prev: Option<u8> = None;
    let n = cfg.chords.len();
    let double_24 = rng.random_bool(0.4);

    for (ci, &off) in cfg.chords.iter().enumerate() {
        let cpc = chord_root_pc(cfg.key_root, off);
        let bass = bass_midi(cpc);
        let ct = chord_tones_in_range(pcs, cpc, mel_lo, mel_hi);
        let ascending = ci < n / 2 || n == 1;

        let melody = two_layer_melody(rng, &scale_mel, &ct, pcs, prev, ascending, cfg.chromaticism);

        for bar in 0..bpc {
            let t = (ci as u32 * bpc + bar) * BAR;

            for i in 0..16u32 {
                events.push(NoteEvent {
                    tick: t + i * S,
                    pitch: bass + bass_pat[i as usize],
                    vel,
                    dur: S,
                });
            }

            for &(pos, pitch, dur) in &melody {
                let tick = t + pos as u32 * S;
                events.push(NoteEvent { tick, pitch, vel, dur });
                if pitch + 12 <= 127 {
                    events.push(NoteEvent { tick, pitch: pitch + 12, vel: vel - 5, dur });
                }
                if double_24 && pitch + 24 <= 127 {
                    events.push(NoteEvent { tick, pitch: pitch + 24, vel: vel - 10, dur });
                }
            }
        }
        prev = melody.last().map(|m| m.1);
    }
    events
}

// ── MIDI file building ──────────────────────────────────────────────

fn build_midi_file(name: &str, cfg: &MidiGenConfig, events: &[NoteEvent]) -> Vec<u8> {
    let track = build_track(name, cfg, events);
    let mut buf = Vec::with_capacity(14 + track.len());
    // MThd header
    buf.extend_from_slice(b"MThd");
    buf.extend_from_slice(&6u32.to_be_bytes());
    buf.extend_from_slice(&0u16.to_be_bytes()); // format 0
    buf.extend_from_slice(&1u16.to_be_bytes()); // 1 track
    buf.extend_from_slice(&(PPQN as u16).to_be_bytes());
    buf.extend_from_slice(&track);
    buf
}

fn build_track(name: &str, cfg: &MidiGenConfig, events: &[NoteEvent]) -> Vec<u8> {
    let mut d = Vec::new();

    // ── Meta events ──
    // Track name
    write_vlq(&mut d, 0);
    d.extend_from_slice(&[0xFF, 0x03]);
    let nb = name.as_bytes();
    write_vlq(&mut d, nb.len() as u32);
    d.extend_from_slice(nb);

    // Tempo
    write_vlq(&mut d, 0);
    d.extend_from_slice(&[0xFF, 0x51, 0x03]);
    let us = 60_000_000u32 / cfg.bpm.max(1) as u32;
    d.push((us >> 16) as u8);
    d.push((us >> 8) as u8);
    d.push(us as u8);

    // Time signature 4/4
    write_vlq(&mut d, 0);
    d.extend_from_slice(&[0xFF, 0x58, 0x04, 4, 2, 24, 8]);

    // Key signature
    write_vlq(&mut d, 0);
    let sf = key_sig_sf(cfg.key_root, cfg.minor);
    d.extend_from_slice(&[0xFF, 0x59, 0x02, sf as u8, u8::from(cfg.minor)]);

    // ── Note events — interleave on/off sorted by tick ──
    let mut on_off: Vec<(u32, u8, u8, u8)> = Vec::with_capacity(events.len() * 2);
    for ev in events {
        on_off.push((ev.tick, 0, ev.pitch, ev.vel)); // 0 = note-on
        on_off.push((ev.tick + ev.dur.max(1), 1, ev.pitch, 0)); // 1 = note-off
    }
    on_off.sort_by_key(|e| (e.0, e.1, e.2));

    let mut prev_tick = 0u32;
    for &(tick, is_off, pitch, vel) in &on_off {
        let delta = tick.saturating_sub(prev_tick);
        write_vlq(&mut d, delta);
        d.push(if is_off == 1 { 0x80 } else { 0x90 });
        d.push(pitch.min(127));
        d.push(vel.min(127));
        prev_tick = tick;
    }

    // End of track
    write_vlq(&mut d, 0);
    d.extend_from_slice(&[0xFF, 0x2F, 0x00]);

    // Wrap in MTrk chunk
    let mut track = Vec::with_capacity(8 + d.len());
    track.extend_from_slice(b"MTrk");
    track.extend_from_slice(&(d.len() as u32).to_be_bytes());
    track.extend_from_slice(&d);
    track
}

fn write_vlq(buf: &mut Vec<u8>, val: u32) {
    if val == 0 {
        buf.push(0);
        return;
    }
    let mut bytes = [0u8; 4];
    let mut n = 0;
    let mut v = val;
    bytes[0] = (v & 0x7F) as u8;
    v >>= 7;
    n += 1;
    while v > 0 {
        bytes[n] = (v & 0x7F) as u8 | 0x80;
        v >>= 7;
        n += 1;
    }
    for i in (0..n).rev() {
        buf.push(bytes[i]);
    }
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(lead_type: LeadType) -> MidiGenConfig {
        MidiGenConfig {
            key_root: 9,  // A
            minor: true,  // A minor
            lead_type,
            chords: vec![0, 5, 7, 3], // Am Dm Em C
            progression: vec![],
            bpm: 120,
            bars_per_chord: 2,
            length_bars: None,
            chromaticism: 15,
            seed: 42,
            name: Some("Test".into()),
            variations: None,
        }
    }

    fn roundtrip(lead_type: LeadType) -> crate::midi::MidiInfo {
        let bytes = generate(&cfg(lead_type)).unwrap();
        let tmp = std::env::temp_dir().join(format!("test_gen_{lead_type:?}.mid"));
        std::fs::write(&tmp, &bytes).unwrap();
        let info = crate::midi::parse_midi(&tmp).unwrap();
        let _ = std::fs::remove_file(&tmp);
        info
    }

    #[test]
    fn test_two_layer_roundtrip() {
        let info = roundtrip(LeadType::TwoLayer);
        assert!(info.note_count > 100, "got {}", info.note_count);
        assert_eq!(info.ppqn, PPQN as u16);
        assert!((info.tempo - 120.0).abs() < 0.5);
        assert_eq!(info.key_signature, "A minor");
    }

    #[test]
    fn test_zigzag_roundtrip() {
        let info = roundtrip(LeadType::Zigzag);
        assert!(info.note_count > 100);
        assert_eq!(info.key_signature, "A minor");
    }

    #[test]
    fn test_bounce_roundtrip() {
        let info = roundtrip(LeadType::Bounce);
        assert!(info.note_count > 50);
        assert_eq!(info.key_signature, "A minor");
    }

    #[test]
    fn test_cell_roundtrip() {
        let info = roundtrip(LeadType::Cell);
        assert!(info.note_count > 50);
        assert_eq!(info.key_signature, "A minor");
    }

    #[test]
    fn test_shuffle_roundtrip() {
        let info = roundtrip(LeadType::Shuffle);
        assert!(info.note_count > 100);
        assert_eq!(info.key_signature, "A minor");
    }

    #[test]
    fn test_chord_arp_roundtrip() {
        let info = roundtrip(LeadType::ChordArp);
        assert!(info.note_count > 100, "ChordArp got {}", info.note_count);
        assert_eq!(info.key_signature, "A minor");
    }

    #[test]
    fn test_gated_stab_roundtrip() {
        let info = roundtrip(LeadType::GatedStab);
        assert!(info.note_count > 50, "GatedStab got {}", info.note_count);
        assert_eq!(info.key_signature, "A minor");
    }

    #[test]
    fn test_pad_chord_roundtrip() {
        let info = roundtrip(LeadType::PadChord);
        assert!(info.note_count > 15, "PadChord got {}", info.note_count);
        assert_eq!(info.key_signature, "A minor");
    }

    #[test]
    fn test_deep_bass_roundtrip() {
        let info = roundtrip(LeadType::DeepBass);
        assert!(info.note_count >= 4, "DeepBass got {}", info.note_count);
        assert_eq!(info.key_signature, "A minor");
    }

    #[test]
    fn test_sub_bass_roundtrip() {
        let info = roundtrip(LeadType::SubBass);
        assert!(info.note_count > 50, "SubBass got {}", info.note_count);
        assert_eq!(info.key_signature, "A minor");
    }

    #[test]
    fn test_progressive_roundtrip() {
        let info = roundtrip(LeadType::Progressive);
        assert!(info.note_count > 100, "Progressive got {}", info.note_count);
        assert_eq!(info.key_signature, "A minor");
    }

    #[test]
    fn test_trill_roundtrip() {
        let info = roundtrip(LeadType::Trill);
        assert!(info.note_count > 50, "Trill got {}", info.note_count);
    }

    #[test]
    fn test_slow_melody_roundtrip() {
        let info = roundtrip(LeadType::SlowMelody);
        assert!(info.note_count >= 8, "SlowMelody got {}", info.note_count);
    }

    #[test]
    fn test_chord_pluck_roundtrip() {
        let info = roundtrip(LeadType::ChordPluck);
        assert!(info.note_count > 100, "ChordPluck got {}", info.note_count);
    }

    #[test]
    fn test_piano_chord_roundtrip() {
        let info = roundtrip(LeadType::PianoChord);
        assert!(info.note_count > 20, "PianoChord got {}", info.note_count);
    }

    #[test]
    fn test_unison_roundtrip() {
        let info = roundtrip(LeadType::Unison);
        assert!(info.note_count > 100, "Unison got {}", info.note_count);
    }

    #[test]
    fn test_deterministic_same_seed() {
        let a = generate(&cfg(LeadType::TwoLayer)).unwrap();
        let b = generate(&cfg(LeadType::TwoLayer)).unwrap();
        assert_eq!(a, b, "same seed must produce identical output");
    }

    #[test]
    fn test_different_seeds_differ() {
        let mut c1 = cfg(LeadType::TwoLayer);
        let mut c2 = cfg(LeadType::TwoLayer);
        c1.seed = 1;
        c2.seed = 2;
        assert_ne!(generate(&c1).unwrap(), generate(&c2).unwrap());
    }

    #[test]
    fn test_batch_produces_variations() {
        let mut c = cfg(LeadType::TwoLayer);
        c.variations = Some(5);
        let batch = generate_batch(&c).unwrap();
        assert_eq!(batch.len(), 5);
        // Each variation should differ
        for i in 1..batch.len() {
            assert_ne!(batch[0], batch[i], "variation {i} should differ from 0");
        }
    }

    #[test]
    fn test_validation_bad_key_root() {
        let mut c = cfg(LeadType::TwoLayer);
        c.key_root = 12;
        assert!(generate(&c).is_err());
    }

    #[test]
    fn test_validation_empty_chords() {
        let mut c = cfg(LeadType::TwoLayer);
        c.chords = vec![];
        assert!(generate(&c).is_err());
    }

    #[test]
    fn test_validation_zero_bpm() {
        let mut c = cfg(LeadType::TwoLayer);
        c.bpm = 0;
        assert!(generate(&c).is_err());
    }

    #[test]
    fn test_major_key() {
        let mut c = cfg(LeadType::TwoLayer);
        c.minor = false;
        c.key_root = 0; // C major
        let bytes = generate(&c).unwrap();
        let tmp = std::env::temp_dir().join("test_gen_major.mid");
        std::fs::write(&tmp, &bytes).unwrap();
        let info = crate::midi::parse_midi(&tmp).unwrap();
        assert_eq!(info.key_signature, "C major");
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_single_bar_per_chord() {
        let mut c = cfg(LeadType::TwoLayer);
        c.bars_per_chord = 1;
        let info_1 = {
            let bytes = generate(&c).unwrap();
            let tmp = std::env::temp_dir().join("test_gen_1bpc.mid");
            std::fs::write(&tmp, &bytes).unwrap();
            let i = crate::midi::parse_midi(&tmp).unwrap();
            let _ = std::fs::remove_file(&tmp);
            i
        };
        c.bars_per_chord = 4;
        let info_4 = {
            let bytes = generate(&c).unwrap();
            let tmp = std::env::temp_dir().join("test_gen_4bpc.mid");
            std::fs::write(&tmp, &bytes).unwrap();
            let i = crate::midi::parse_midi(&tmp).unwrap();
            let _ = std::fs::remove_file(&tmp);
            i
        };
        assert!(
            info_4.note_count > info_1.note_count,
            "4 bpc ({}) should have more notes than 1 bpc ({})",
            info_4.note_count,
            info_1.note_count
        );
    }

    #[test]
    fn test_vlq_encoding() {
        let cases: [(u32, &[u8]); 5] = [
            (0, &[0x00]),
            (0x7F, &[0x7F]),
            (0x80, &[0x81, 0x00]),
            (480, &[0x83, 0x60]),
            (268_435_455, &[0xFF, 0xFF, 0xFF, 0x7F]),
        ];
        for (val, expected) in &cases {
            let mut buf = Vec::new();
            write_vlq(&mut buf, *val);
            assert_eq!(&buf, expected, "VLQ for {val}");
        }
    }

    #[test]
    fn test_key_sig_sf_values() {
        assert_eq!(key_sig_sf(0, false), 0); // C major
        assert_eq!(key_sig_sf(9, true), 0); // A minor (rel. C)
        assert_eq!(key_sig_sf(7, false), 1); // G major
        assert_eq!(key_sig_sf(3, false), -3); // Eb major
        assert_eq!(key_sig_sf(4, true), 1); // E minor (rel. G)
    }

    #[test]
    fn test_all_12_keys_generate() {
        for root in 0..12u8 {
            for minor in [true, false] {
                let c = MidiGenConfig {
                    key_root: root,
                    minor,
                    lead_type: LeadType::TwoLayer,
                    chords: vec![0, 5, 7],
                    progression: vec![],
                    bpm: 120,
                    bars_per_chord: 1,
                    length_bars: None,
                    chromaticism: 15,
                    seed: 99,
                    name: None,
                    variations: None,
                };
                let bytes = generate(&c).unwrap();
                assert!(bytes.len() > 14, "key {root} minor={minor} produced too few bytes");
            }
        }
    }

    #[test]
    fn test_all_lead_types_all_bar_lengths() {
        for lt in [
            LeadType::TwoLayer,
            LeadType::Zigzag,
            LeadType::Bounce,
            LeadType::Cell,
            LeadType::Shuffle,
            LeadType::ChordArp,
            LeadType::GatedStab,
            LeadType::PadChord,
            LeadType::DeepBass,
            LeadType::SubBass,
            LeadType::Progressive,
            LeadType::Trill,
            LeadType::SlowMelody,
            LeadType::ChordPluck,
            LeadType::PianoChord,
            LeadType::Unison,
        ] {
            for bpc in [1, 2, 4, 8] {
                let c = MidiGenConfig {
                    key_root: 2, // D
                    minor: true,
                    lead_type: lt,
                    chords: vec![0, 5, 7, 3],
                    progression: vec![],
                    bpm: 140,
                    bars_per_chord: bpc,
                    length_bars: None,
                    chromaticism: 15,
                    seed: 123,
                    name: None,
                    variations: None,
                };
                let bytes = generate(&c).unwrap();
                let tmp = std::env::temp_dir().join(format!("test_gen_{lt:?}_{bpc}bpc.mid"));
                std::fs::write(&tmp, &bytes).unwrap();
                let info = crate::midi::parse_midi(&tmp);
                assert!(
                    info.is_some(),
                    "{lt:?} with {bpc} bars_per_chord produced unparseable MIDI"
                );
                let _ = std::fs::remove_file(&tmp);
            }
        }
    }

    #[test]
    fn test_parse_chord_names() {
        assert_eq!(parse_chord_name("Am"), Some(9));
        assert_eq!(parse_chord_name("C"), Some(0));
        assert_eq!(parse_chord_name("F#m"), Some(6));
        assert_eq!(parse_chord_name("Bbm"), Some(10));
        assert_eq!(parse_chord_name("Dm"), Some(2));
        assert_eq!(parse_chord_name("G"), Some(7));
        assert_eq!(parse_chord_name("Eb"), Some(3));
        assert_eq!(parse_chord_name("Dbm"), Some(1));
        assert_eq!(parse_chord_name(""), None);
    }

    #[test]
    fn test_progression_names_resolve() {
        let mut c = cfg(LeadType::TwoLayer);
        c.chords = vec![];
        c.progression = vec!["Am".into(), "Dm".into(), "Em".into(), "C".into()];
        // key_root = 9 (A), so Am=0, Dm=5, Em=7, C=3
        let resolved = resolve_chords(&c);
        assert_eq!(resolved, vec![0, 5, 7, 3]);
    }

    #[test]
    fn test_progression_takes_precedence() {
        let mut c = cfg(LeadType::TwoLayer);
        c.chords = vec![0, 0, 0, 0]; // would be boring
        c.progression = vec!["Am".into(), "F".into(), "G".into()];
        let resolved = resolve_chords(&c);
        // A=9, F=5 → offset (5-9)%12=8, G=7 → offset (7-9)%12=10
        assert_eq!(resolved, vec![0, 8, 10]);
    }

    #[test]
    fn test_length_bars_repeats_progression() {
        let mut c = cfg(LeadType::TwoLayer);
        c.progression = vec!["Am".into(), "Dm".into()]; // 2 chords
        c.bars_per_chord = 2;
        c.length_bars = Some(16); // needs 8 chords to fill 16 bars at 2 bpc
        let resolved = resolve_config(&c);
        assert_eq!(resolved.chords.len(), 8);
        // Should cycle: Am, Dm, Am, Dm, Am, Dm, Am, Dm
        assert_eq!(resolved.chords[0], resolved.chords[2]);
        assert_eq!(resolved.chords[1], resolved.chords[3]);
    }

    #[test]
    fn test_progression_generates_valid_midi() {
        let mut c = cfg(LeadType::TwoLayer);
        c.chords = vec![];
        c.progression = vec!["Am".into(), "F".into(), "C".into(), "G".into()];
        let bytes = generate(&c).unwrap();
        let tmp = std::env::temp_dir().join("test_gen_progression.mid");
        std::fs::write(&tmp, &bytes).unwrap();
        let info = crate::midi::parse_midi(&tmp).unwrap();
        assert!(info.note_count > 100);
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_length_bars_generates_correct_duration() {
        let mut c = cfg(LeadType::Zigzag);
        c.progression = vec!["Am".into(), "Dm".into()];
        c.bars_per_chord = 2;
        c.length_bars = Some(8); // 8 bars = 4 chord slots
        let bytes = generate(&c).unwrap();
        let tmp = std::env::temp_dir().join("test_gen_length.mid");
        std::fs::write(&tmp, &bytes).unwrap();
        let info = crate::midi::parse_midi(&tmp).unwrap();
        // 8 bars at 120 BPM, ppqn=96 → 8 × 4 beats × 0.5s/beat = 16s
        assert!(info.duration > 10.0 && info.duration < 20.0,
            "expected ~16s duration, got {}", info.duration);
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_generate_kits_creates_directory_structure() {
        let tmp_dir = std::env::temp_dir().join("test_trance_kits");
        let _ = std::fs::remove_dir_all(&tmp_dir);

        let config = KitGenConfig {
            key_root: 9, // A
            minor: true,
            progression: vec!["Am".into(), "Dm".into(), "Em".into(), "C".into()],
            chords: vec![],
            bpm: 140,
            bars_per_chord: 2,
            length_bars: None,
            chromaticism: 15,
            seed: 42,
            num_kits: 3,
            layers: vec![],
        };

        let kits = generate_kits(&config, &tmp_dir).unwrap();
        assert_eq!(kits.len(), 3);

        for (i, kit) in kits.iter().enumerate() {
            assert!(kit.name.contains("Am"), "kit name should contain key");
            assert!(kit.name.contains("140 BPM"), "kit name should contain BPM");
            assert!(kit.name.starts_with(&format!("Kit {}", i + 1)));
            assert_eq!(kit.files.len(), KIT_LAYERS.len(), "kit should have all default layers");

            // Verify each file exists on disk
            for f in &kit.files {
                let path = std::path::Path::new(&f.path);
                assert!(path.exists(), "file should exist: {}", f.path);
                assert!(f.size > 14, "MIDI file should be non-trivial");
                assert!(f.path.ends_with(".mid"));
            }

            // Verify directory exists
            let dir = std::path::Path::new(&kit.dir);
            assert!(dir.is_dir());
        }

        let _ = std::fs::remove_dir_all(&tmp_dir);
    }

    #[test]
    fn test_generate_kits_custom_layers() {
        let tmp_dir = std::env::temp_dir().join("test_trance_kits_custom");
        let _ = std::fs::remove_dir_all(&tmp_dir);

        let config = KitGenConfig {
            key_root: 2, // D
            minor: true,
            progression: vec!["Dm".into(), "Am".into()],
            chords: vec![],
            bpm: 138,
            bars_per_chord: 2,
            length_bars: None,
            chromaticism: 15,
            seed: 99,
            num_kits: 1,
            layers: vec![LeadType::TwoLayer, LeadType::PadChord, LeadType::DeepBass],
        };

        let kits = generate_kits(&config, &tmp_dir).unwrap();
        assert_eq!(kits.len(), 1);
        assert_eq!(kits[0].files.len(), 3, "should have 3 custom layers");

        let layer_names: Vec<&str> = kits[0].files.iter().map(|f| f.layer.as_str()).collect();
        assert!(layer_names.contains(&"Lead TwoLayer"));
        assert!(layer_names.contains(&"Pad"));
        assert!(layer_names.contains(&"Deep Bass"));

        let _ = std::fs::remove_dir_all(&tmp_dir);
    }
}
