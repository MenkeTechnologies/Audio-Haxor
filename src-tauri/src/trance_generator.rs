//! Trance arrangement generator — produces Ableton Live Sets with MIDI and audio tracks.
//!
//! All melodic layers (pad, lead, arp, bass, sub, pluck) are MIDI tracks.
//! Drums and FX (kick, clap, hat, riser, impact, crash) are audio tracks with
//! samples matched from the user's library.
//!
//! Section layout: intro(32) → build(32) → breakdown(48) → drop1(32) →
//!                 drop2(32) → fadedown(32) → outro(48) = 256 bars.
//!
//! Intro and outro MIDI clips use root chord only (no progression changes).

use crate::als_generator::{
    self, AbletonVersion, ClipPlacement, MidiClipPlacement, MidiTrackInfo, SampleInfo, TrackInfo,
};
use crate::als_project::{self, Genre, ProjectConfig, SectionLengths};
use crate::midi_generator::{self, LeadType, MidiGenConfig, NoteEvent};
use rand::rngs::StdRng;
use rand::{RngExt, SeedableRng};

/// Trance arrangement layer for MIDI tracks.
struct MidiLayer {
    name: &'static str,
    lead_type: LeadType,
    color: u8,
    /// Which sections this layer plays in (bar ranges computed from SectionLengths).
    /// Format: (start_bar_offset_from_section_start_ratio, plays_in_section)
    plays_in: SectionMask,
}

/// Which of the 7 sections a layer is active in.
#[derive(Clone, Copy)]
struct SectionMask {
    intro: bool,
    build: bool,
    breakdown: bool,
    drop1: bool,
    drop2: bool,
    fadedown: bool,
    outro: bool,
}

impl SectionMask {
    const FULL: Self = Self {
        intro: true,
        build: true,
        breakdown: true,
        drop1: true,
        drop2: true,
        fadedown: true,
        outro: true,
    };
    const DROPS: Self = Self {
        intro: false,
        build: false,
        breakdown: false,
        drop1: true,
        drop2: true,
        fadedown: false,
        outro: false,
    };
    const BUILD_TO_FADE: Self = Self {
        intro: false,
        build: true,
        breakdown: true,
        drop1: true,
        drop2: true,
        fadedown: true,
        outro: false,
    };
    const MAIN_BODY: Self = Self {
        intro: false,
        build: true,
        breakdown: true,
        drop1: true,
        drop2: true,
        fadedown: false,
        outro: false,
    };
    const INTRO_OUTRO: Self = Self {
        intro: true,
        build: false,
        breakdown: false,
        drop1: false,
        drop2: false,
        fadedown: false,
        outro: true,
    };
    const NO_BREAKDOWN: Self = Self {
        intro: true,
        build: true,
        breakdown: false,
        drop1: true,
        drop2: true,
        fadedown: true,
        outro: true,
    };
}

/// The MIDI layers that make up a trance arrangement.
fn midi_layers(rng: &mut StdRng) -> Vec<MidiLayer> {
    // Pick lead types with some randomness
    let lead_choices = [
        LeadType::TwoLayer,
        LeadType::Unison,
        LeadType::ChordArp,
        LeadType::Zigzag,
    ];
    let lead2_choices = [LeadType::SlowMelody, LeadType::Bounce, LeadType::Cell];

    vec![
        MidiLayer {
            name: "PAD",
            lead_type: LeadType::PadChord,
            color: 12, // blue
            plays_in: SectionMask::FULL,
        },
        MidiLayer {
            name: "LEAD",
            lead_type: lead_choices[rng.random_range(0..lead_choices.len())],
            color: 26, // orange
            plays_in: SectionMask::BUILD_TO_FADE,
        },
        MidiLayer {
            name: "LEAD 2",
            lead_type: lead2_choices[rng.random_range(0..lead2_choices.len())],
            color: 27, // light orange
            plays_in: SectionMask::DROPS,
        },
        MidiLayer {
            name: "ARP",
            lead_type: LeadType::Progressive,
            color: 14, // teal
            plays_in: SectionMask::NO_BREAKDOWN,
        },
        MidiLayer {
            name: "PLUCK",
            lead_type: LeadType::ChordPluck,
            color: 15, // cyan
            plays_in: SectionMask::MAIN_BODY,
        },
        MidiLayer {
            name: "DEEP BASS",
            lead_type: LeadType::DeepBass,
            color: 69, // dark red
            plays_in: SectionMask::NO_BREAKDOWN,
        },
        MidiLayer {
            name: "SUB BASS",
            lead_type: LeadType::SubBass,
            color: 70, // red
            plays_in: SectionMask::DROPS,
        },
        MidiLayer {
            name: "PIANO",
            lead_type: LeadType::PianoChord,
            color: 17, // purple
            plays_in: SectionMask {
                intro: false,
                build: false,
                breakdown: true,
                drop1: false,
                drop2: false,
                fadedown: true,
                outro: false,
            },
        },
        MidiLayer {
            name: "TRILL",
            lead_type: LeadType::Trill,
            color: 28, // yellow
            plays_in: SectionMask::DROPS,
        },
    ]
}

/// Section bar ranges computed from SectionLengths.
struct SectionBars {
    intro: (u32, u32),
    build: (u32, u32),
    breakdown: (u32, u32),
    drop1: (u32, u32),
    drop2: (u32, u32),
    fadedown: (u32, u32),
    outro: (u32, u32),
}

impl SectionBars {
    fn from_lengths(sl: &SectionLengths) -> Self {
        let mut pos = 0u32;
        let intro = (pos, pos + sl.intro);
        pos += sl.intro;
        let build = (pos, pos + sl.build);
        pos += sl.build;
        let breakdown = (pos, pos + sl.breakdown);
        pos += sl.breakdown;
        let drop1 = (pos, pos + sl.drop1);
        pos += sl.drop1;
        let drop2 = (pos, pos + sl.drop2);
        pos += sl.drop2;
        let fadedown = (pos, pos + sl.fadedown);
        pos += sl.fadedown;
        let outro = (pos, pos + sl.outro);
        Self {
            intro,
            build,
            breakdown,
            drop1,
            drop2,
            fadedown,
            outro,
        }
    }

    /// Get all (start, end) bar ranges where this mask is active.
    fn active_ranges(&self, mask: &SectionMask) -> Vec<(u32, u32)> {
        let mut ranges = Vec::new();
        if mask.intro {
            ranges.push(self.intro);
        }
        if mask.build {
            ranges.push(self.build);
        }
        if mask.breakdown {
            ranges.push(self.breakdown);
        }
        if mask.drop1 {
            ranges.push(self.drop1);
        }
        if mask.drop2 {
            ranges.push(self.drop2);
        }
        if mask.fadedown {
            ranges.push(self.fadedown);
        }
        if mask.outro {
            ranges.push(self.outro);
        }
        ranges
    }

    /// Is this bar range in the intro or outro? (for root-chord-only rule)
    fn is_intro_or_outro(&self, start: u32, end: u32) -> bool {
        (start >= self.intro.0 && end <= self.intro.1)
            || (start >= self.outro.0 && end <= self.outro.1)
    }
}

/// Generate just the MIDI tracks for an arrangement — called by track_generator
/// when midi_tracks is enabled. Returns MidiTrackInfo vec ready for ALS injection.
pub fn generate_midi_tracks_for_arrangement(
    root_note: Option<&str>,
    mode: Option<&str>,
    midi_settings: &Option<&crate::als_project::MidiSettings>,
    seed: u64,
    bpm: u16,
    section_lengths: &crate::als_project::SectionLengths,
) -> Result<Vec<MidiTrackInfo>, String> {
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    let mut rng = StdRng::seed_from_u64(seed);
    let sections = SectionBars::from_lengths(section_lengths);

    let key_root = root_note
        .and_then(|r| {
            [
                "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
            ]
            .iter()
            .position(|&n| n.eq_ignore_ascii_case(r))
        })
        .unwrap_or(9) as u8;
    let minor = mode
        .map(|m| !m.eq_ignore_ascii_case("Ionian"))
        .unwrap_or(true);

    let ms = midi_settings.map(|s| (*s).clone()).unwrap_or_default();
    let bars_per_chord = if ms.bars_per_chord > 0 {
        ms.bars_per_chord
    } else {
        2
    };
    let chromaticism = ms.chromaticism;

    // Build progression
    let progression: Vec<String> = if !ms.progression.is_empty() {
        ms.progression.clone()
    } else {
        let notes = [
            "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
        ];
        let root = notes[key_root as usize];
        if minor {
            vec![
                format!("{}m", root),
                notes[((key_root + 3) % 12) as usize].into(),
                notes[((key_root + 10) % 12) as usize].into(),
                notes[((key_root + 8) % 12) as usize].into(),
            ]
        } else {
            vec![
                root.into(),
                notes[((key_root + 5) % 12) as usize].into(),
                notes[((key_root + 7) % 12) as usize].into(),
                format!("{}m", notes[((key_root + 9) % 12) as usize]),
            ]
        }
    };
    let root_only: Vec<String> = vec![progression[0].clone()];

    let layers = midi_layers(&mut rng);
    let mut midi_tracks: Vec<MidiTrackInfo> = Vec::new();

    for layer in &layers {
        let ranges = sections.active_ranges(&layer.plays_in);
        if ranges.is_empty() {
            continue;
        }

        let mut clips: Vec<MidiClipPlacement> = Vec::new();
        for &(start_bar, end_bar) in &ranges {
            let length = end_bar - start_bar;
            if length == 0 {
                continue;
            }
            let prog = if sections.is_intro_or_outro(start_bar, end_bar) {
                &root_only
            } else {
                &progression
            };

            let midi_cfg = MidiGenConfig {
                key_root,
                minor,
                lead_type: layer.lead_type,
                chords: vec![],
                progression: prog.clone(),
                bpm,
                bars_per_chord,
                length_bars: Some(length),
                chromaticism,
                seed: seed.wrapping_add(start_bar as u64 * 100 + layer.color as u64),
                name: Some(layer.name.to_string()),
                variations: Some(1),
            };
            let events = midi_generator::generate_events(&midi_cfg)?;
            clips.push(MidiClipPlacement {
                events,
                start_bar,
                length_bars: length,
                name: layer.name.to_string(),
                color: layer.color,
            });
        }
        midi_tracks.push(MidiTrackInfo {
            name: layer.name.to_string(),
            color: layer.color,
            clips,
        });
    }

    Ok(midi_tracks)
}

/// Result from trance arrangement generation.
pub struct TranceResult {
    pub path: String,
    pub tracks: usize,
    pub clips: usize,
    pub bars: u32,
}

/// Generate a full trance arrangement ALS file.
pub fn generate(
    config: &ProjectConfig,
    output_path: &std::path::Path,
    seed: u64,
) -> Result<TranceResult, String> {
    let mut rng = StdRng::seed_from_u64(seed);
    let sl = config.section_lengths;
    let sections = SectionBars::from_lengths(&sl);
    let bpm = config.bpm as f64;

    // Resolve key
    let key_root = config
        .root_note
        .as_deref()
        .and_then(|r| {
            [
                "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
            ]
            .iter()
            .position(|&n| n.eq_ignore_ascii_case(r))
        })
        .unwrap_or(9) as u8; // default A
    let minor = config
        .mode
        .as_deref()
        .map(|m| !m.eq_ignore_ascii_case("Ionian"))
        .unwrap_or(true); // default minor

    // MIDI settings from the Trance Lead pane (or defaults)
    let ms = config.midi_settings.clone().unwrap_or_default();
    let bars_per_chord = if ms.bars_per_chord > 0 {
        ms.bars_per_chord
    } else {
        2
    };
    let chromaticism = ms.chromaticism;

    // Parse chord progression: midi_settings.progression > config.keywords > default
    let progression: Vec<String> = if !ms.progression.is_empty() {
        ms.progression.clone()
    } else if !config.keywords.is_empty() {
        config
            .keywords
            .iter()
            .filter(|k| !k.is_empty())
            .cloned()
            .collect()
    } else {
        // Default trance progression: i III VII VI
        let notes = [
            "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
        ];
        let root = notes[key_root as usize];
        if minor {
            let iii = notes[((key_root + 3) % 12) as usize];
            let vii = notes[((key_root + 10) % 12) as usize];
            let vi = notes[((key_root + 8) % 12) as usize];
            vec![
                format!("{}m", root),
                iii.to_string(),
                vii.to_string(),
                vi.to_string(),
            ]
        } else {
            let iv = notes[((key_root + 5) % 12) as usize];
            let v = notes[((key_root + 7) % 12) as usize];
            let vi = notes[((key_root + 9) % 12) as usize];
            vec![
                root.to_string(),
                iv.to_string(),
                v.to_string(),
                format!("{}m", vi),
            ]
        }
    };

    // Root-only progression for intro/outro
    let root_only: Vec<String> = vec![progression[0].clone()];

    let layers = midi_layers(&mut rng);

    // ── Generate MIDI tracks ─────────────────────────────────────────

    let mut midi_tracks: Vec<MidiTrackInfo> = Vec::new();

    if config.midi_tracks {
        for layer in &layers {
            let ranges = sections.active_ranges(&layer.plays_in);
            if ranges.is_empty() {
                continue;
            }

            let mut clips: Vec<MidiClipPlacement> = Vec::new();

            for &(start_bar, end_bar) in &ranges {
                let length = end_bar - start_bar;
                if length == 0 {
                    continue;
                }

                // Intro/outro: root chord only
                let prog = if sections.is_intro_or_outro(start_bar, end_bar) {
                    &root_only
                } else {
                    &progression
                };

                let midi_cfg = MidiGenConfig {
                    key_root,
                    minor,
                    lead_type: layer.lead_type,
                    chords: vec![],
                    progression: prog.clone(),
                    bpm: config.bpm as u16,
                    bars_per_chord,
                    length_bars: Some(length),
                    chromaticism,
                    seed: seed.wrapping_add(start_bar as u64 * 100 + layer.color as u64),
                    name: Some(layer.name.to_string()),
                    variations: Some(1),
                };

                let events = midi_generator::generate_events(&midi_cfg)?;

                clips.push(MidiClipPlacement {
                    events,
                    start_bar,
                    length_bars: length,
                    name: layer.name.to_string(),
                    color: layer.color,
                });
            }

            midi_tracks.push(MidiTrackInfo {
                name: layer.name.to_string(),
                color: layer.color,
                clips,
            });
        }
    } // if config.midi_tracks

    // ── Generate audio tracks (drums + FX) ───────────────────────────
    // Mirrors the techno generator's full track layout: KICKS, DRUMS, FX groups
    // with all sample types from the user's library.

    let mut audio_tracks: Vec<TrackInfo> = Vec::new();

    // Helper: query samples and build a track with clips at given bar ranges
    let build_audio_track = |name: &str,
                             category: &str,
                             color: u8,
                             mask: &SectionMask,
                             require_loop: bool,
                             limit: u32|
     -> Option<TrackInfo> {
        let ranges = sections.active_ranges(mask);
        if ranges.is_empty() {
            return None;
        }
        let samples =
            als_project::query_samples(category, config, require_loop, limit).unwrap_or_default();
        if samples.is_empty() {
            return None;
        }
        let sample = &samples[0];
        let sample_info = SampleInfo {
            path: sample.path.clone(),
            name: sample.name.clone(),
            duration_secs: sample.duration,
            sample_rate: 44100,
            file_size: sample.size as u64,
            bpm: sample.parsed_bpm.map(|b| b as f64),
        };
        let clips = ranges
            .iter()
            .map(|&(s, e)| ClipPlacement {
                sample: sample_info.clone(),
                start_beat: s as f64 * 4.0,
                duration_beats: (e - s) as f64 * 4.0,
            })
            .collect();
        Some(TrackInfo {
            name: name.to_string(),
            color,
            clips,
        })
    };

    // Helper for FX one-shots at specific bar positions
    let build_fx_track =
        |name: &str, category: &str, color: u8, positions: &[(u32, u32)]| -> Option<TrackInfo> {
            let samples =
                als_project::query_samples(category, config, false, 3).unwrap_or_default();
            if samples.is_empty() {
                return None;
            }
            let sample = &samples[0];
            let sample_info = SampleInfo {
                path: sample.path.clone(),
                name: sample.name.clone(),
                duration_secs: sample.duration,
                sample_rate: 44100,
                file_size: sample.size as u64,
                bpm: None,
            };
            let clips = positions
                .iter()
                .map(|&(s, e)| ClipPlacement {
                    sample: sample_info.clone(),
                    start_beat: s as f64 * 4.0,
                    duration_beats: (e - s) as f64 * 4.0,
                })
                .collect();
            Some(TrackInfo {
                name: name.to_string(),
                color,
                clips,
            })
        };

    let tc = &config.track_counts;

    // === DRUMS ===
    for i in 0..tc.kick {
        if let Some(t) = build_audio_track(
            &format!(
                "KICK{}",
                if tc.kick > 1 {
                    format!(" {}", i + 1)
                } else {
                    String::new()
                }
            ),
            "kick",
            1,
            &SectionMask::NO_BREAKDOWN,
            true,
            5,
        ) {
            audio_tracks.push(t);
        }
    }
    for i in 0..tc.clap {
        if let Some(t) = build_audio_track(
            &format!(
                "CLAP{}",
                if tc.clap > 1 {
                    format!(" {}", i + 1)
                } else {
                    String::new()
                }
            ),
            "clap",
            2,
            &SectionMask::NO_BREAKDOWN,
            true,
            5,
        ) {
            audio_tracks.push(t);
        }
    }
    for i in 0..tc.snare {
        if let Some(t) = build_audio_track(
            &format!(
                "SNARE{}",
                if tc.snare > 1 {
                    format!(" {}", i + 1)
                } else {
                    String::new()
                }
            ),
            "clap",
            2,
            &SectionMask::DROPS,
            true,
            5,
        ) {
            audio_tracks.push(t);
        }
    }
    for i in 0..tc.hat {
        if let Some(t) = build_audio_track(
            &format!(
                "HAT{}",
                if tc.hat > 1 {
                    format!(" {}", i + 1)
                } else {
                    String::new()
                }
            ),
            "closed_hat",
            3,
            &SectionMask::NO_BREAKDOWN,
            true,
            5,
        ) {
            audio_tracks.push(t);
        }
    }
    for i in 0..tc.perc {
        if let Some(t) = build_audio_track(
            &format!(
                "PERC{}",
                if tc.perc > 1 {
                    format!(" {}", i + 1)
                } else {
                    String::new()
                }
            ),
            "perc",
            4,
            &SectionMask::DROPS,
            true,
            5,
        ) {
            audio_tracks.push(t);
        }
    }
    for i in 0..tc.ride {
        if let Some(t) = build_audio_track(
            &format!(
                "RIDE{}",
                if tc.ride > 1 {
                    format!(" {}", i + 1)
                } else {
                    String::new()
                }
            ),
            "ride",
            5,
            &SectionMask::MAIN_BODY,
            true,
            5,
        ) {
            audio_tracks.push(t);
        }
    }
    for i in 0..tc.fill {
        if let Some(t) = build_audio_track(
            &format!(
                "FILL{}",
                if tc.fill > 1 {
                    format!(" {}", i + 1)
                } else {
                    String::new()
                }
            ),
            "fx_fill",
            6,
            &SectionMask::DROPS,
            false,
            5,
        ) {
            audio_tracks.push(t);
        }
    }

    // === FX ===
    let half_build = sections.build.0 + (sections.build.1 - sections.build.0) / 2;
    let half_breakdown = sections.breakdown.0 + (sections.breakdown.1 - sections.breakdown.0) / 2;

    for i in 0..tc.riser {
        if let Some(t) = build_fx_track(
            &format!(
                "RISER{}",
                if tc.riser > 1 {
                    format!(" {}", i + 1)
                } else {
                    String::new()
                }
            ),
            "fx_riser",
            8,
            &[
                (half_build, sections.build.1),
                (half_breakdown, sections.breakdown.1),
            ],
        ) {
            audio_tracks.push(t);
        }
    }
    for i in 0..tc.downlifter {
        if let Some(t) = build_fx_track(
            &format!(
                "DOWNLIFTER{}",
                if tc.downlifter > 1 {
                    format!(" {}", i + 1)
                } else {
                    String::new()
                }
            ),
            "fx_downer",
            8,
            &[
                (sections.drop1.0 + 16, sections.drop1.0 + 24),
                (sections.drop2.0 + 16, sections.drop2.0 + 24),
            ],
        ) {
            audio_tracks.push(t);
        }
    }
    for i in 0..tc.crash {
        if let Some(t) = build_fx_track(
            &format!(
                "CRASH{}",
                if tc.crash > 1 {
                    format!(" {}", i + 1)
                } else {
                    String::new()
                }
            ),
            "fx_crash",
            9,
            &[
                (sections.drop1.0, sections.drop1.0 + 2),
                (sections.drop2.0, sections.drop2.0 + 2),
            ],
        ) {
            audio_tracks.push(t);
        }
    }
    for i in 0..tc.impact {
        if let Some(t) = build_fx_track(
            &format!(
                "IMPACT{}",
                if tc.impact > 1 {
                    format!(" {}", i + 1)
                } else {
                    String::new()
                }
            ),
            "fx_impact",
            10,
            &[
                (sections.drop1.0, sections.drop1.0 + 1),
                (sections.drop2.0, sections.drop2.0 + 1),
            ],
        ) {
            audio_tracks.push(t);
        }
    }
    for i in 0..tc.hit {
        if let Some(t) = build_fx_track(
            &format!(
                "HIT{}",
                if tc.hit > 1 {
                    format!(" {}", i + 1)
                } else {
                    String::new()
                }
            ),
            "fx_misc",
            10,
            &[(sections.drop1.0, sections.drop1.0 + 1)],
        ) {
            audio_tracks.push(t);
        }
    }
    for i in 0..tc.sweep_up {
        if let Some(t) = build_fx_track(
            &format!(
                "SWEEP UP{}",
                if tc.sweep_up > 1 {
                    format!(" {}", i + 1)
                } else {
                    String::new()
                }
            ),
            "fx_riser",
            8,
            &[(half_breakdown, sections.breakdown.1)],
        ) {
            audio_tracks.push(t);
        }
    }
    for i in 0..tc.sweep_down {
        if let Some(t) = build_fx_track(
            &format!(
                "SWEEP DN{}",
                if tc.sweep_down > 1 {
                    format!(" {}", i + 1)
                } else {
                    String::new()
                }
            ),
            "fx_downer",
            8,
            &[(sections.fadedown.0, sections.fadedown.0 + 8)],
        ) {
            audio_tracks.push(t);
        }
    }
    for i in 0..tc.snare_roll {
        if let Some(t) = build_fx_track(
            &format!(
                "SNARE ROLL{}",
                if tc.snare_roll > 1 {
                    format!(" {}", i + 1)
                } else {
                    String::new()
                }
            ),
            "fx_fill",
            6,
            &[
                (sections.build.1 - 2, sections.build.1),
                (sections.breakdown.1 - 4, sections.breakdown.1),
            ],
        ) {
            audio_tracks.push(t);
        }
    }
    for i in 0..tc.reverse {
        if let Some(t) = build_fx_track(
            &format!(
                "REVERSE{}",
                if tc.reverse > 1 {
                    format!(" {}", i + 1)
                } else {
                    String::new()
                }
            ),
            "fx_reverse",
            10,
            &[
                (sections.drop1.0 - 1, sections.drop1.0),
                (sections.drop2.0 - 1, sections.drop2.0),
            ],
        ) {
            audio_tracks.push(t);
        }
    }
    for i in 0..tc.sub_drop {
        if let Some(t) = build_fx_track(
            &format!(
                "SUB DROP{}",
                if tc.sub_drop > 1 {
                    format!(" {}", i + 1)
                } else {
                    String::new()
                }
            ),
            "fx_sub_drop",
            10,
            &[(sections.breakdown.0, sections.breakdown.0 + 4)],
        ) {
            audio_tracks.push(t);
        }
    }
    for i in 0..tc.boom_kick {
        if let Some(t) = build_fx_track(
            &format!(
                "BOOM KICK{}",
                if tc.boom_kick > 1 {
                    format!(" {}", i + 1)
                } else {
                    String::new()
                }
            ),
            "fx_impact",
            10,
            &[(sections.drop1.0, sections.drop1.0 + 1)],
        ) {
            audio_tracks.push(t);
        }
    }
    // Vocals (audio, not MIDI)
    for i in 0..tc.vox {
        if let Some(t) = build_audio_track(
            &format!(
                "VOX{}",
                if tc.vox > 1 {
                    format!(" {}", i + 1)
                } else {
                    String::new()
                }
            ),
            "vocal",
            11,
            &SectionMask::MAIN_BODY,
            false,
            5,
        ) {
            audio_tracks.push(t);
        }
    }

    // ── Assemble ALS ─────────────────────────────────────────────────

    let total_tracks = audio_tracks.len() + midi_tracks.len();
    let total_clips: usize = audio_tracks.iter().map(|t| t.clips.len()).sum::<usize>()
        + midi_tracks.iter().map(|t| t.clips.len()).sum::<usize>();

    generate_trance_als(output_path, &audio_tracks, &midi_tracks, bpm)?;

    Ok(TranceResult {
        path: output_path.to_string_lossy().into(),
        tracks: total_tracks,
        clips: total_clips,
        bars: sl.total_bars(),
    })
}

/// Build the ALS XML with both audio and MIDI tracks.
fn generate_trance_als(
    output_path: &std::path::Path,
    audio_tracks: &[TrackInfo],
    midi_tracks: &[MidiTrackInfo],
    bpm: f64,
) -> Result<(), String> {
    use flate2::Compression;
    use flate2::write::GzEncoder;
    use std::io::Write;

    let version = AbletonVersion::detect();

    // Decompress the embedded template (has both AudioTrack and MidiTrack)
    let template_data = {
        use flate2::read::GzDecoder;
        use std::io::Read;
        let mut decoder = GzDecoder::new(als_generator::EMPTY_TEMPLATE_BYTES);
        let mut s = String::new();
        decoder.read_to_string(&mut s).map_err(|e| e.to_string())?;
        s
    };

    // Find the template's AudioTrack and MidiTrack — remove just those two,
    // keep everything else (ReturnTracks, GroupTracks, etc.) untouched.
    let tracks_start = template_data
        .find("<Tracks>")
        .ok_or("No <Tracks> in template")?
        + "<Tracks>".len();
    let tracks_end = template_data
        .find("</Tracks>")
        .ok_or("No </Tracks> in template")?;

    // Extract the AudioTrack template for cloning audio tracks
    let audio_track_template = {
        let at_start = template_data.find("<AudioTrack").ok_or("No AudioTrack")?;
        let at_end = template_data
            .find("</AudioTrack>")
            .ok_or("No AudioTrack end")?
            + "</AudioTrack>".len();
        template_data[at_start..at_end].to_string()
    };

    let id_re = regex::Regex::new(r#"Id="(\d+)""#).unwrap();
    let name_re = regex::Regex::new(r#"<EffectiveName Value="[^"]*" />"#).unwrap();
    let color_re = regex::Regex::new(r#"<Color Value="\d+" />"#).unwrap();

    let mut ids = crate::als_generator::IdAllocatorPub::new(100_000);
    let mut all_tracks_xml = Vec::new();

    // Audio tracks — clone from AudioTrack template
    for track in audio_tracks {
        let mut t = audio_track_template.clone();
        let mut replacements: Vec<(String, String)> = Vec::new();
        for cap in id_re.captures_iter(&t) {
            replacements.push((
                format!(r#"Id="{}""#, &cap[1]),
                format!(r#"Id="{}""#, ids.next()),
            ));
        }
        for (old, new) in replacements {
            t = t.replacen(&old, &new, 1);
        }
        t = name_re
            .replace(
                &t,
                format!(
                    r#"<EffectiveName Value="{}" />"#,
                    crate::als_generator::xml_escape_pub(&track.name)
                ),
            )
            .to_string();
        t = color_re
            .replace_all(&t, format!(r#"<Color Value="{}" />"#, track.color))
            .to_string();

        let mut clips_xml = Vec::new();
        for clip in &track.clips {
            clips_xml.push(crate::als_generator::generate_audio_clip_pub(
                clip, &mut ids,
            ));
        }
        if !clips_xml.is_empty() {
            let events_re = regex::Regex::new(r#"<Events />"#).unwrap();
            t = events_re
                .replace(&t, format!("<Events>\n{}\n</Events>", clips_xml.join("\n")))
                .to_string();
        }
        all_tracks_xml.push(t);
    }

    // MIDI tracks — uses embedded MIDI_TRACK_TEMPLATE internally
    for track in midi_tracks {
        let t = als_generator::generate_midi_track(&audio_track_template, track, &mut ids);
        all_tracks_xml.push(t);
    }

    // Strip template's AudioTrack and MidiTrack from the tracks section,
    // keep ReturnTracks and anything else intact.
    let tracks_section = &template_data[tracks_start..tracks_end];
    let strip_audio = regex::Regex::new(r"(?s)<AudioTrack\b.*?</AudioTrack>").unwrap();
    let strip_midi = regex::Regex::new(r"(?s)<MidiTrack\b.*?</MidiTrack>").unwrap();
    let kept = strip_audio.replace_all(tracks_section, "").to_string();
    let kept = strip_midi.replace_all(&kept, "").to_string();

    // Insert our generated tracks before the kept content (ReturnTracks etc.)
    let before = &template_data[..tracks_start];
    let after = &template_data[tracks_end..];
    let joined = all_tracks_xml.join("\n\t\t\t");
    let mut xml = format!("{}\n\t\t\t{}\n{}{}", before, joined, kept, after);

    // Update NextPointeeId
    let next_id = ids.max_val() + 1000;
    let next_id_re = regex::Regex::new(r#"<NextPointeeId Value="\d+" />"#).unwrap();
    xml = next_id_re
        .replace(&xml, format!(r#"<NextPointeeId Value="{}" />"#, next_id))
        .to_string();

    // Set tempo
    let tempo_re =
        regex::Regex::new(r#"<Tempo>\s*<LomId Value="0" />\s*<Manual Value="[^"]+" />"#).unwrap();
    xml = tempo_re
        .replace(
            &xml,
            format!(
                r#"<Tempo>
								<LomId Value="0" />
								<Manual Value="{}" />"#,
                bpm
            ),
        )
        .to_string();

    let tempo_event_re =
        regex::Regex::new(r#"<FloatEvent Id="\d+" Time="-63072000" Value="[^"]+" />"#).unwrap();
    xml = tempo_event_re
        .replace(
            &xml,
            format!(r#"<FloatEvent Id="0" Time="-63072000" Value="{}" />"#, bpm),
        )
        .to_string();

    // Set PhaseNudgeTempo and SessionTempo
    let pnt_re = regex::Regex::new(r#"<PhaseNudgeTempo Value="[^"]+" />"#).unwrap();
    xml = pnt_re
        .replace_all(&xml, format!(r#"<PhaseNudgeTempo Value="{}" />"#, bpm))
        .to_string();
    let st_re = regex::Regex::new(r#"<SessionTempo Value="[^"]+" />"#).unwrap();
    xml = st_re
        .replace_all(&xml, format!(r#"<SessionTempo Value="{}" />"#, bpm))
        .to_string();

    // Update Ableton version header
    let version_re = regex::Regex::new(r#"MinorVersion="[^"]*""#).unwrap();
    xml = version_re
        .replace(
            &xml,
            format!(r#"MinorVersion="{}""#, version.minor_version_string),
        )
        .to_string();
    let creator_re = regex::Regex::new(r#"Creator="[^"]*""#).unwrap();
    xml = creator_re
        .replace(&xml, format!(r#"Creator="{}""#, version.creator))
        .to_string();

    // Write compressed output
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let output_file =
        std::fs::File::create(output_path).map_err(|e| format!("Failed to create file: {}", e))?;
    let mut encoder = GzEncoder::new(output_file, Compression::default());
    encoder
        .write_all(xml.as_bytes())
        .map_err(|e| format!("Failed to write: {}", e))?;
    encoder
        .finish()
        .map_err(|e| format!("Failed to compress: {}", e))?;

    Ok(())
}
