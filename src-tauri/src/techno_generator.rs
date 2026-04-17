//! Techno arrangement generator — extracted from generate_true_techno example.
//! Produces valid ALS files using the embedded template approach.

use crate::als_generator::generate_empty_als;
use crate::write_app_log;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use rand::prelude::*;
use rand::rngs::StdRng;
use regex::Regex;
use std::cell::RefCell;
use std::collections::HashSet;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use std::sync::atomic::{AtomicU32, Ordering};

// Silence between songs (bars)
const GAP_BETWEEN_SONGS: u32 = 32;

// ---------------------------------------------------------------------------
// Seeded RNG for deterministic generation
// ---------------------------------------------------------------------------
//
// Every random decision the generator makes — sample shuffling, scatter
// placement, glitch edit positions, fill rotations, swoosh selection — reads
// from a single seeded `StdRng` stored in a thread-local `RefCell`.
//
// `generate()` calls `init_gen_rng(seed)` at entry and `clear_gen_rng()` at
// every exit path (success AND error), so a locked seed + identical config
// produces a bit-identical arrangement. Without this, each `rand::rng()` call
// drew from the thread's entropy source — two generations with the same
// wizard settings would produce different output, and there was no way to
// "regenerate the one I liked".
//
// Chose thread-local over explicit `&mut StdRng` threading because the
// generator has 10 helpers and 4 coordinators that currently pull from the
// global RNG — explicit threading would have touched every signature and
// every test caller. Thread-local scopes cleanly to a single `generate()`
// invocation (the Tauri command wraps the call in `spawn_blocking`, one task
// per generation) and keeps the diff localised to the actual randomness
// sources. Re-entrancy is not a concern: helpers never call each other
// recursively and all RNG access is inside `with_gen_rng` which holds a
// `RefCell` borrow only for the closure's lifetime.
thread_local! {
    static GEN_RNG: RefCell<Option<StdRng>> = const { RefCell::new(None) };
}

/// Seed the thread-local generation RNG. Called once at the top of `generate()`.
fn init_gen_rng(seed: u64) {
    GEN_RNG.with(|c| *c.borrow_mut() = Some(StdRng::seed_from_u64(seed)));
}

/// Drop the thread-local RNG. Called on every `generate()` exit path so a
/// later non-ALS use of this thread cannot observe leftover state (and so
/// tests that run helpers directly fall back to the wall-clock seed below).
fn clear_gen_rng() {
    GEN_RNG.with(|c| *c.borrow_mut() = None);
}

/// Run `f` with a mutable reference to the thread-local generation RNG.
///
/// When no generation is in progress (typically unit tests that invoke
/// helpers directly without going through `generate()`), lazily seed the
/// slot from the current wall-clock nanoseconds. This preserves the
/// pre-refactor behaviour (each helper call was seeded from system entropy)
/// while keeping the seeded path deterministic.
fn with_gen_rng<R>(f: impl FnOnce(&mut StdRng) -> R) -> R {
    GEN_RNG.with(|c| {
        let mut b = c.borrow_mut();
        if b.is_none() {
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos() as u64)
                .unwrap_or(0xA5A5_5A5A_A5A5_5A5A);
            *b = Some(StdRng::seed_from_u64(nanos));
        }
        f(b.as_mut().unwrap())
    })
}

// Canonical arrangement span — 224 bars (7 × 32-bar sections). This is the
// *reference* layout that every `TrackArrangement` template is written
// against; users can override per-section bar counts via
// `ProjectConfig.section_lengths`, and at generation time we remap the
// canonical ranges into the user's layout (see `remap_bar_range`).
// The authoritative per-section starts live in
// `SectionLengths::techno_default().starts()` / `canonical_section_starts()`;
// this const is retained because `apply_density_per_section` walks the
// canonical 8-bar block grid before remapping (dynamics are applied in
// canonical coordinates, then bar ranges projected to user coordinates).
const SONG_LENGTH_BARS: u32 = 224;

/// Canonical section starts — fixed, matches the values all templates were
/// written against. User layouts are derived from `SectionLengths::starts()`.
fn canonical_section_starts() -> crate::als_project::SectionStarts {
    crate::als_project::SectionLengths::techno_default().starts()
}

/// Project a canonical-layout bar range onto the user's section layout.
///
/// The canonical layout is 7 × 32 bars (see the `INTRO_START` family of
/// consts). Every `TrackArrangement` template is written with absolute bar
/// positions inside that layout — no template range crosses a section
/// boundary, which is what lets this remap stay a pure offset shift:
///
///   canonical_bar = canonical_section_start + offset_in_section
///   user_bar      = user_section_start      + offset_in_section
///
/// Ranges that extend past the user's (possibly shorter) section end are
/// clipped; ranges whose start is already past the user's section end are
/// dropped entirely (returns None). This gracefully handles the common user
/// edit ("shrink outro 48 → 32") without template rewrites.
///
/// For sections the user has *extended* beyond the canonical 32-bar length,
/// the remap produces bars only up to the original 32-bar span — the
/// extension remains silent (no template content fills it). Addressing that
/// is a future enhancement; for now the user's stated use case (shrinking
/// back to uniform 32) is handled correctly.
pub fn remap_bar_range(
    canon_start: f64,
    canon_end: f64,
    user: &crate::als_project::SectionStarts,
) -> Option<(f64, f64)> {
    let c = canonical_section_starts();
    let s_u32 = canon_start as u32;
    // Pick the canonical/user pair for the section containing `canon_start`.
    let (c_lo, c_hi, u_lo, u_hi) = if s_u32 < c.build.0 {
        (c.intro.0, c.intro.1, user.intro.0, user.intro.1)
    } else if s_u32 < c.breakdown.0 {
        (c.build.0, c.build.1, user.build.0, user.build.1)
    } else if s_u32 < c.drop1.0 {
        (c.breakdown.0, c.breakdown.1, user.breakdown.0, user.breakdown.1)
    } else if s_u32 < c.drop2.0 {
        (c.drop1.0, c.drop1.1, user.drop1.0, user.drop1.1)
    } else if s_u32 < c.fadedown.0 {
        (c.drop2.0, c.drop2.1, user.drop2.0, user.drop2.1)
    } else if s_u32 < c.outro.0 {
        (c.fadedown.0, c.fadedown.1, user.fadedown.0, user.fadedown.1)
    } else {
        (c.outro.0, c.outro.1, user.outro.0, user.outro.1)
    };

    // Sanity: if canon_start isn't inside the canonical section we picked
    // (shouldn't happen for well-formed templates), drop silently.
    if canon_start < c_lo as f64 || canon_start >= c_hi as f64 {
        return None;
    }

    let offset_start = canon_start - c_lo as f64;
    let offset_end = canon_end - c_lo as f64;
    let user_section_len = (u_hi - u_lo) as f64;

    // Range starts past the end of the user's (shorter) section → nothing.
    if offset_start >= user_section_len {
        return None;
    }

    let new_start = u_lo as f64 + offset_start;
    let new_end = (u_lo as f64 + offset_end).min(u_hi as f64);
    if new_end < new_start {
        None
    } else {
        // new_end == new_start is a valid one-shot placement (single hit)
        Some((new_start, new_end))
    }
}

// Element entry/exit positions (in bars, supports fractional for beat precision)
// 16.75 = bar 16, beat 4 (last beat of bar 16)
// 17.0 = bar 17, beat 1 (downbeat)
#[derive(Clone, Debug, PartialEq)]
struct TrackArrangement {
    name: String,
    sections: Vec<(f64, f64)>, // (start_bar, end_bar) pairs where element plays
}

impl TrackArrangement {
    fn new(name: &str, sections: Vec<(f64, f64)>) -> Self {
        Self { name: name.to_string(), sections }
    }
}

// All samples needed for one song
/// Song samples - each field is a Vec of tracks, each track has Vec<SampleInfo>
/// e.g., kicks[0] = KICK 1 samples, kicks[1] = KICK 2 samples, etc.
struct SongSamples {
    key: String,
    // Drums
    kicks: Vec<Vec<SampleInfo>>,
    claps: Vec<Vec<SampleInfo>>,
    snares: Vec<Vec<SampleInfo>>,
    hats: Vec<Vec<SampleInfo>>,
    percs: Vec<Vec<SampleInfo>>,
    rides: Vec<Vec<SampleInfo>>,
    fills: Vec<Vec<SampleInfo>>,
    // Bass
    basses: Vec<Vec<SampleInfo>>,
    subs: Vec<Vec<SampleInfo>>,
    // Melodics
    leads: Vec<Vec<SampleInfo>>,
    synths: Vec<Vec<SampleInfo>>,
    pads: Vec<Vec<SampleInfo>>,
    arps: Vec<Vec<SampleInfo>>,
    keyss: Vec<Vec<SampleInfo>>,
    // FX
    risers: Vec<Vec<SampleInfo>>,
    downlifters: Vec<Vec<SampleInfo>>,
    crashes: Vec<Vec<SampleInfo>>,
    impacts: Vec<Vec<SampleInfo>>,
    hits: Vec<Vec<SampleInfo>>,
    sweep_ups: Vec<Vec<SampleInfo>>,
    sweep_downs: Vec<Vec<SampleInfo>>,
    snare_rolls: Vec<Vec<SampleInfo>>,
    reverses: Vec<Vec<SampleInfo>>,
    sub_drops: Vec<Vec<SampleInfo>>,
    boom_kicks: Vec<Vec<SampleInfo>>,
    atmoses: Vec<Vec<SampleInfo>>,
    glitches: Vec<Vec<SampleInfo>>,
    scatters: Vec<Vec<SampleInfo>>,
    // Vocals
    voxes: Vec<Vec<SampleInfo>>,
}

/// Generate randomized swoosh (sweep up/down) arrangements.
/// 
/// - Sweeps hit every 16 bars
/// - Sweep UP ends at the grid (climax on the downbeat)
/// - Sweep DOWN starts at the grid
/// - SWEEP UP 1-4: risers leading into grid points
/// - SWEEP DOWN 1-4: falls following grid points
/// - Tracks rotate through grid positions
fn generate_swoosh_arrangements() -> Vec<TrackArrangement> {
    use rand::seq::SliceRandom;
    with_gen_rng(|rng| {
        // 16-bar grid positions throughout the track (224 bars total)
        let grid_positions: Vec<u32> = vec![16, 32, 48, 64, 80, 96, 112, 128, 144, 160, 176, 192, 208];

        // 4 tracks each for UP and DOWN
        let num_tracks = 4;

        // Default bar lengths for variety
        let bar_lengths: Vec<u32> = vec![2, 4, 4, 8];

        // Initialize track sections
        let mut up_tracks: Vec<Vec<(f64, f64)>> = (0..num_tracks).map(|_| Vec::new()).collect();
        let mut down_tracks: Vec<Vec<(f64, f64)>> = (0..num_tracks).map(|_| Vec::new()).collect();

        // Shuffle grid positions and distribute to tracks
        let mut shuffled_up = grid_positions.clone();
        let mut shuffled_down = grid_positions.clone();
        shuffled_up.shuffle(rng);
        shuffled_down.shuffle(rng);

        // Assign UP sweeps - round-robin across tracks
        for (i, &grid) in shuffled_up.iter().enumerate() {
            let track_idx = i % num_tracks;
            let bar_len = bar_lengths[track_idx];
            let start = (grid - bar_len) as f64;
            let end = grid as f64;
            up_tracks[track_idx].push((start, end));
        }

        // Assign DOWN sweeps - round-robin across tracks (all tracks get sections)
        for (i, &grid) in shuffled_down.iter().enumerate() {
            let track_idx = i % num_tracks;
            let bar_len = bar_lengths[track_idx];
            let start = grid as f64;
            let end = (grid + bar_len) as f64;
            down_tracks[track_idx].push((start, end));
        }

        // Sort each track's sections by start time
        for sections in up_tracks.iter_mut() {
            sections.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        }
        for sections in down_tracks.iter_mut() {
            sections.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        }

        // Build track arrangements
        let mut arrangements = Vec::new();

        // SWEEP UP 1, SWEEP UP 2, SWEEP UP 3, SWEEP UP 4
        for (i, sections) in up_tracks.into_iter().enumerate() {
            if !sections.is_empty() {
                let name = format!("SWEEP UP {}", i + 1);
                arrangements.push(TrackArrangement { name, sections });
            }
        }

        // SWEEP DOWN 1, SWEEP DOWN 2, SWEEP DOWN 3, SWEEP DOWN 4
        for (i, sections) in down_tracks.into_iter().enumerate() {
            if !sections.is_empty() {
                let name = format!("SWEEP DOWN {}", i + 1);
                arrangements.push(TrackArrangement { name, sections });
            }
        }

        arrangements
    })
}

/// Generate scattered one-shot hits on 1/16th grid.
/// 
/// Creates random hit patterns over 32-bar sections that repeat throughout the track.
/// Multiple SCATTER tracks with different samples fire at random 1/16th positions.
/// Per-section scatter values control density in each section (0.0 = none, 1.0 = dense).
/// 
/// Ableton supports fractional beat values (e.g., 480.5, 95.75) for 1/16th grid precision.
fn generate_scatter_hits(section_scatter: &SectionValues, global_scatter: f32, track_count: u32) -> Vec<TrackArrangement> {
    with_gen_rng(|rng| {
        const SIXTEENTHS_PER_BAR: u32 = 16;
        const BLOCK_BARS: u32 = 8;
        const SIXTEENTHS_PER_BLOCK: u32 = BLOCK_BARS * SIXTEENTHS_PER_BAR; // 128

        // Walk every 8-bar block in the canonical 224-bar layout, matching the
        // section-overrides grid granularity. Each block gets its own density
        // from `value_at_bar`, so painted blocks produce hits and unpainted ones
        // stay silent.
        let total_bars: u32 = 224;
        let mut blocks: Vec<(u32, u32, f32)> = Vec::new(); // (start, end_exclusive, density)
        let mut bar = 1u32;
        while bar <= total_bars {
            let block_end = (bar + BLOCK_BARS).min(total_bars + 1);
            let density = section_scatter.value_at_bar(bar, global_scatter);
            if density > 0.0 {
                blocks.push((bar, block_end, density));
            }
            bar = block_end;
        }

        if blocks.is_empty() {
            return vec![];
        }

        let mut results: Vec<TrackArrangement> = Vec::new();

        // Generate unique patterns for every scatter track. Each track
        // gets its own random 1/16th positions; higher tracks are sparser.
        let n = track_count.max(1);
        for track_num in 1..=n {
            let mut sections_out: Vec<(f64, f64)> = Vec::new();

            for &(block_start, block_end, density) in &blocks {
                let block_len = block_end - block_start;
                // Hits per 8-bar block scales quadratically for perceptible
                // density difference: 0.1 → ~1, 0.3 → ~3, 0.5 → ~8, 1.0 → ~32
                let max_hits_per_block = (block_len * SIXTEENTHS_PER_BAR / 4) as f32; // 32 for 8 bars
                let target_hits = ((density * density * max_hits_per_block).round() as u32).max(1);
                // Higher track numbers are sparser — track 1 full, track N minimal
                let track_hits = ((target_hits as f32 / track_num as f32).ceil() as u32).max(1);

                let sixteenths_in_block = block_len * SIXTEENTHS_PER_BAR;
                let mut pattern: Vec<u32> = Vec::new();
                let mut attempts = 0u32;
                while pattern.len() < track_hits as usize && attempts < 500 {
                    let s = rng.random_range(0..sixteenths_in_block);
                    let too_close = pattern.iter().any(|&p| s.abs_diff(p) < 4);
                    if !too_close {
                        pattern.push(s);
                    }
                    attempts += 1;
                }

                for &s in &pattern {
                    let bar_in_block = s / SIXTEENTHS_PER_BAR;
                    let sixteenth_in_bar = s % SIXTEENTHS_PER_BAR;
                    let abs_bar = block_start + bar_in_block;
                    if abs_bar >= block_end { continue; }
                    let abs_pos = abs_bar as f64 + (sixteenth_in_bar as f64 * 0.0625);
                    sections_out.push((abs_pos, abs_pos));
                }
            }

            if !sections_out.is_empty() {
                sections_out.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
                results.push(TrackArrangement::new(&format!("SCATTER {}", track_num), sections_out));
            }
        }

        results
    })
}

/// Generate randomized fill arrangements for variety.
/// 
/// Fill positions are at phrase boundaries (every 8 bars), but the LENGTH of each fill
/// (1-beat, 2-beat, or 4-beat) and which SAMPLE (A, B, C, D) is randomized.
/// This prevents the "machine gun" effect of predictable fill patterns.
fn generate_random_fills() -> Vec<TrackArrangement> {
    with_gen_rng(|rng| {
        // All possible fill positions (bar numbers where fills can occur)
        // These are the last bar of each 8-bar phrase
        let fill_positions: Vec<u32> = vec![
            16, 24, 32, 40, 48, 56, 64, 72, 80, 88, 96, 104, 112, 120, 128, 136, 144, 152, 160, 168, 176, 184, 192, 200, 208, 216
        ];

        // For each position, randomly choose fill length: 1, 2, or 4 beats
        // Weight towards variety - don't repeat same length too often
        let mut fill_assignments: Vec<(u32, u8, u8)> = Vec::new(); // (bar, length, sample_variant)
        let mut last_length: u8 = 0;

        for &bar in &fill_positions {
            // Weighted random: less likely to repeat same length twice
            let weights: Vec<u8> = vec![1, 2, 4];
            let length = loop {
                let choice = *weights.choose(rng).unwrap();
                // 70% chance to pick different length, 30% to repeat
                if choice != last_length || rng.random_bool(0.3) {
                    break choice;
                }
            };
            last_length = length;

            // Random sample variant (A=0, B=1, C=2, D=3 for 4-beat; A=0, B=1 for 1/2-beat)
            let max_variant = if length == 4 { 4 } else { 2 };
            let variant: u8 = rng.random_range(0..max_variant);

            fill_assignments.push((bar, length, variant));
        }

        // Distribute assignments to the 8 fill tracks
        let mut fill_1a: Vec<(f64, f64)> = Vec::new();
        let mut fill_1b: Vec<(f64, f64)> = Vec::new();
        let mut fill_2a: Vec<(f64, f64)> = Vec::new();
        let mut fill_2b: Vec<(f64, f64)> = Vec::new();
        let mut fill_4a: Vec<(f64, f64)> = Vec::new();
        let mut fill_4b: Vec<(f64, f64)> = Vec::new();
        let mut fill_4c: Vec<(f64, f64)> = Vec::new();
        let mut fill_4d: Vec<(f64, f64)> = Vec::new();

        for (bar, length, variant) in fill_assignments {
            let bar_f = bar as f64;
            let section = match length {
                1 => (bar_f + 0.75, bar_f + 1.0), // Last beat of bar
                2 => (bar_f + 0.5, bar_f + 1.0),  // Last 2 beats of bar
                4 => (bar_f, bar_f + 1.0),        // Full bar
                _ => continue,
            };

            match (length, variant) {
                (1, 0) => fill_1a.push(section),
                (1, 1) => fill_1b.push(section),
                (2, 0) => fill_2a.push(section),
                (2, 1) => fill_2b.push(section),
                (4, 0) => fill_4a.push(section),
                (4, 1) => fill_4b.push(section),
                (4, 2) => fill_4c.push(section),
                (4, 3) => fill_4d.push(section),
                _ => {}
            }
        }

        vec![
            TrackArrangement::new("FILL 1", fill_1a),
            TrackArrangement::new("FILL 2", fill_1b),
            TrackArrangement::new("FILL 3", fill_2a),
            TrackArrangement::new("FILL 4", fill_2b),
            TrackArrangement::new("FILL 5", fill_4a),
            TrackArrangement::new("FILL 6", fill_4b),
            TrackArrangement::new("FILL 7", fill_4c),
            TrackArrangement::new("FILL 8", fill_4d),
        ]
    })
}

/// Generate glitch arrangements at fill positions (same timing as fills).
/// Glitches add variety and are placed at phrase boundaries.
fn generate_glitch_arrangements() -> Vec<TrackArrangement> {
    use rand::seq::SliceRandom;
    with_gen_rng(|rng| {
        // Fill positions (every 8 bars)
        let mut positions: Vec<u32> = vec![
            16, 24, 32, 40, 48, 56, 64, 72, 80, 88, 96, 104, 112, 120, 128, 136, 144, 152, 160, 168, 176, 184, 192, 200, 208, 216
        ];
        positions.shuffle(rng);

        // Distribute positions across up to 8 glitch tracks (round-robin)
        let num_tracks = 8;
        let mut track_sections: Vec<Vec<(f64, f64)>> = (0..num_tracks).map(|_| Vec::new()).collect();

        for (i, &bar) in positions.iter().enumerate() {
            let track_idx = i % num_tracks;
            // Glitches are short bursts - 1-2 beats
            let bar_f = bar as f64;
            let section = if rng.random_bool(0.5) {
                (bar_f + 0.75, bar_f + 1.0) // 1 beat
            } else {
                (bar_f + 0.5, bar_f + 1.0)  // 2 beats
            };
            track_sections[track_idx].push(section);
        }

        // Sort each track's sections by time
        for sections in track_sections.iter_mut() {
            sections.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        }

        // Build arrangements (only for non-empty tracks)
        let mut arrangements = Vec::new();
        for (i, sections) in track_sections.into_iter().enumerate() {
            if !sections.is_empty() {
                arrangements.push(TrackArrangement::new(&format!("GLITCH {}", i + 1), sections));
            }
        }
        arrangements
    })
}

fn get_arrangement(chaos: f32) -> Vec<TrackArrangement> {
    // 8-BAR RULE: Every 8 bars, add something (intro/build) or drop something (fadedown)
    // 224 bars = 7 sections of 32 bars each
    //
    // INTRO:     1-32    (add elements)
    // BUILD:     33-64   (add elements)
    // BREAKDOWN: 65-96   (kick/bass out, melodic)
    // DROP 1:    97-128  (full energy)
    // DROP 2:    129-160 (full energy)
    // FADEDOWN:  161-192 (drop elements every 8 bars)
    // OUTRO:     193-224 (minimal, mirror intro)
    //
    // FILL RULE: Main elements (kick, clap, hat, bass) drop out 1 bar before
    // each 8-bar phrase boundary to make room for fills

    let mut base = vec![
        // === DRUMS ===
        // KICK: gaps for varied fill lengths
        // Gap is the LAST bar/beats before a phrase boundary, fill plays IN the gap
        // 1 beat gap: last beat of bar 16, 56, 104, 136, 168, 216 (beat 4)
        // 2 beat gap: last 2 beats of bar 24, 40, 72, 88, 120, 152, 184, 208 (beats 3-4)
        // 4 beat gap: full bar 32, 48, 64, 80, 96, 112, 128, 144, 160, 176, 192
        TrackArrangement::new("KICK", vec![
                // INTRO (1-32) - gap at bar 16 (1 beat), bar 24 (2 beats), bar 32 (4 beats)
                (1.0, 16.75),     // ends beat 4 of bar 16, gap is beat 4 (1 beat fill)
                (17.0, 24.5),     // ends beat 3 of bar 24, gap is beats 3-4 (2 beat fill)
                (25.0, 32.0),     // ends at bar 32, gap is bar 32 (4 beat fill)
                // BUILD (33-64) - gap at bar 40 (2 beats), bar 48 (4 beats), bar 56 (1 beat), bar 64 (4 beats)
                (33.0, 40.5),     // gap beats 3-4 of bar 40
                (41.0, 48.0),     // gap bar 48
                (49.0, 56.75),    // gap beat 4 of bar 56
                (57.0, 64.0),     // gap bar 64
                // BREAKDOWN: kick OUT (65-96)
                // DROP 1 (97-128) - gap at 104 (1 beat), 112 (4 beats), 120 (2 beats), 128 (4 beats)
                (97.0, 104.75),   // gap beat 4 of bar 104
                (105.0, 112.0),   // gap bar 112
                (113.0, 120.5),   // gap beats 3-4 of bar 120
                (121.0, 128.0),   // gap bar 128
                // DROP 2 (129-160)
                (129.0, 136.75),  // gap beat 4 of bar 136
                (137.0, 144.0),   // gap bar 144
                (145.0, 152.5),   // gap beats 3-4 of bar 152
                (153.0, 160.0),   // gap bar 160
                // FADEDOWN (161-192)
                (161.0, 168.75),  // gap beat 4 of bar 168
                (169.0, 176.0),   // gap bar 176
                (177.0, 184.5),   // gap beats 3-4 of bar 184
                (185.0, 192.0),   // gap bar 192
                // OUTRO (193-224)
                (193.0, 208.5),   // gap beats 3-4 of bar 208
                (209.0, 216.75),  // gap beat 4 of bar 216
                (217.0, 224.0),   // final phrase, no gap
            ]),
        // FADEDOWN (161-192) + OUTRO (193-224) drops every 8 bars:
        // Bar 161: start fadedown (full energy still)
        // Bar 169: -SYNTH 2, -SYNTH 3, -ARP, -ARP 2, -SUB
        // Bar 177: -SYNTH 1, -PAD, -PERC 2, -HAT 2, -RIDE
        // Bar 185: -PERC, -HAT
        // Bar 193: -CLAP (outro starts)
        // Bar 201: -BASS
        // Bar 209: (kick + atmos only)
        // Bar 217: (kick + atmos only)

        // CLAP: enters bar 9, gaps match KICK timing
        TrackArrangement::new("CLAP", vec![
                // INTRO - gaps at bar 16 (1 beat), 24 (2 beats), 32 (4 beats)
                (9.0, 16.75),     // ends beat 4 of bar 16
                (17.0, 24.5),     // ends beat 3 of bar 24
                (25.0, 32.0),     // ends at bar 32
                // BUILD - gaps at 40 (2 beats), 48 (4 beats), 56 (1 beat), 64 (4 beats)
                (33.0, 40.5),
                (41.0, 48.0),
                (49.0, 56.75),
                (57.0, 64.0),
                // Breakdown: out
                // DROP 1 - gaps at 104 (1 beat), 112 (4 beats), 120 (2 beats), 128 (4 beats)
                (97.0, 104.75),
                (105.0, 112.0),
                (113.0, 120.5),
                (121.0, 128.0),
                // DROP 2
                (129.0, 136.75),
                (137.0, 144.0),
                (145.0, 152.5),
                (153.0, 160.0),
                // FADEDOWN
                (161.0, 168.75),
                (169.0, 176.0),
                (177.0, 184.5),
                (185.0, 192.0),   // drops at 193
            ]),
        // SNARE: enters bar 33 (build), different timing than clap
        TrackArrangement::new("SNARE", vec![
                // BUILD - comes in later than clap
                (33.0, 40.5),
                (41.0, 48.0),
                (49.0, 56.75),
                (57.0, 64.0),
                // Breakdown: out
                // DROP 1
                (97.0, 104.75),
                (105.0, 112.0),
                (113.0, 120.5),
                (121.0, 128.0),
                // DROP 2
                (129.0, 136.75),
                (137.0, 144.0),
                (145.0, 152.5),
                (153.0, 160.0),
                // FADEDOWN - drops out earlier than clap
                (161.0, 168.75),
                (169.0, 176.0),
            ]),
        // HAT: enters bar 17, gaps match KICK
        TrackArrangement::new("HAT", vec![
                // INTRO
                (17.0, 24.5),
                (25.0, 32.0),
                // BUILD
                (33.0, 40.5),
                (41.0, 48.0),
                (49.0, 56.75),
                (57.0, 64.0),
                // Breakdown: out
                // DROP 1
                (97.0, 104.75),
                (105.0, 112.0),
                (113.0, 120.5),
                (121.0, 128.0),
                // DROP 2
                (129.0, 136.75),
                (137.0, 144.0),
                (145.0, 152.5),
                (153.0, 160.0),
                // FADEDOWN
                (161.0, 168.75),
                (169.0, 176.0),
                (177.0, 184.0),   // drops at 185
            ]),
        TrackArrangement::new("HAT 2", vec![
                (97.0, 104.75),
                (105.0, 112.0),
                (113.0, 120.5),
                (121.0, 128.0),
                (137.0, 144.0),
                (145.0, 152.5),
                (153.0, 160.0),
                (161.0, 168.75),
                (169.0, 176.0),   // drops at 177
            ]),
        TrackArrangement::new("PERC", vec![
                (25.0, 32.0),
                // BUILD
                (33.0, 40.5),
                (41.0, 48.0),
                (49.0, 56.75),
                (57.0, 64.0),
                // Breakdown: out
                // DROP 1
                (97.0, 104.75),
                (105.0, 112.0),
                (113.0, 120.5),
                (121.0, 128.0),
                // DROP 2
                (129.0, 136.75),
                (137.0, 144.0),
                (145.0, 152.5),
                (153.0, 160.0),
                // FADEDOWN
                (161.0, 168.75),
                (169.0, 176.0),
                (177.0, 184.0),   // drops at 185
            ]),
        TrackArrangement::new("PERC 2", vec![
                (41.0, 48.0),
                (49.0, 56.75),
                (57.0, 64.0),
                // Breakdown: out
                (113.0, 120.5),
                (121.0, 128.0),
                (129.0, 136.75),
                (137.0, 144.0),
                (145.0, 152.5),
                (153.0, 160.0),
                (161.0, 168.75),
                (169.0, 176.0),   // drops at 177
            ]),
        TrackArrangement::new("RIDE", vec![
                (33.0, 40.5),
                (41.0, 48.0),
                (49.0, 56.75),
                (57.0, 64.0),
                // Breakdown: out
                (97.0, 104.75),
                (105.0, 112.0),
                (113.0, 120.5),
                (121.0, 128.0),
                (129.0, 136.75),
                (137.0, 144.0),
                (145.0, 152.5),
                (153.0, 160.0),
                (161.0, 168.75),
                (169.0, 176.0),   // drops at 177
            ]),

        // === BASS ===
        // BASS: enters bar 33, gaps match drums
        TrackArrangement::new("BASS 1", vec![
                // BUILD
                (33.0, 40.5),
                (41.0, 48.0),
                (49.0, 56.75),
                (57.0, 64.0),
                // Breakdown: out
                // DROP 1
                (97.0, 104.75),
                (105.0, 112.0),
                (113.0, 120.5),
                (121.0, 128.0),
                // DROP 2
                (129.0, 136.75),
                (137.0, 144.0),
                (145.0, 152.5),
                (153.0, 160.0),
                // FADEDOWN
                (161.0, 168.75),
                (169.0, 176.0),
                (177.0, 184.5),
                (185.0, 192.0),
                (193.0, 200.0),   // drops at 201
            ]),
        // SUB: gaps match bass, plays through breakdown for low-end continuity
        TrackArrangement::new("SUB 1", vec![
                // BUILD (bars 33-64)
                (33.0, 40.0),
                (41.0, 48.0),
                (49.0, 56.75),
                (57.0, 64.0),
                // BREAKDOWN (bars 65-96) - sub continues for low-end
                (65.0, 72.5),
                (73.0, 80.0),
                (81.0, 88.5),
                (89.0, 96.0),
                // DROP 1
                (97.0, 104.75),
                (105.0, 112.0),
                (113.0, 120.5),
                (121.0, 128.0),
                // DROP 2
                (129.0, 136.75),
                (137.0, 144.0),
                (145.0, 152.5),
                (153.0, 160.0),
                (161.0, 168.75),  // drops at 169
            ]),

        // === MELODICS (all with fill gaps) ===
        // MAIN SYNTH - the lead, introduced mid-breakdown (bar 81), explodes in drop
        TrackArrangement::new("MAIN SYNTH", vec![
                (81.0, 88.5),     // mid-breakdown, gap at 88 (2 beats)
                (89.0, 96.0),     // gap at 96 (4 beats)
                // DROP 1
                (97.0, 104.75),
                (105.0, 112.0),
                (113.0, 120.5),
                (121.0, 128.0),
                // DROP 2
                (129.0, 136.75),
                (137.0, 144.0),
                (145.0, 152.5),
                (153.0, 160.0),
                // brief return in outro
                (185.0, 192.0),
            ]),
        TrackArrangement::new("SYNTH 1", vec![
                // BUILD
                (41.0, 48.0),
                (49.0, 56.75),
                (57.0, 64.0),
                // BREAKDOWN
                (73.0, 80.0),
                (81.0, 88.5),
                (89.0, 96.0),
                // DROPS
                (97.0, 104.75),
                (105.0, 112.0),
                (113.0, 120.5),
                (121.0, 128.0),
                (129.0, 136.75),
                (137.0, 144.0),
                (145.0, 152.5),
                (153.0, 160.0),
                (161.0, 168.75),
                (169.0, 176.0),   // drops at 177
            ]),
        TrackArrangement::new("PAD 1", vec![
                // BUILD
                (49.0, 56.75),
                (57.0, 64.0),
                // BREAKDOWN
                (65.0, 72.5),
                (73.0, 80.0),
                (81.0, 88.5),
                (89.0, 96.0),
                // DROPS
                (129.0, 136.75),
                (137.0, 144.0),
                (145.0, 152.5),
                (153.0, 160.0),
                (161.0, 168.75),
                (169.0, 176.0),   // drops at 177
            ]),
        TrackArrangement::new("PAD 2", vec![
                (81.0, 88.5),
                (89.0, 96.0),
            ]),
        // LEAD: similar to SYNTH 1 but more prominent in drops
        TrackArrangement::new("LEAD 1", vec![
                // BUILD (late entry)
                (49.0, 56.75),
                (57.0, 64.0),
                // BREAKDOWN
                (73.0, 80.0),
                (81.0, 88.5),
                (89.0, 96.0),
                // DROP 1
                (97.0, 104.75),
                (105.0, 112.0),
                (113.0, 120.5),
                (121.0, 128.0),
                // DROP 2
                (129.0, 136.75),
                (137.0, 144.0),
                (145.0, 152.5),
                (153.0, 160.0),
                // FADEDOWN
                (161.0, 168.75),
                (169.0, 176.0),
            ]),
        TrackArrangement::new("ARP 1", vec![
                (57.0, 64.0),
                (89.0, 96.0),
                // DROP 1
                (97.0, 104.75),
                (105.0, 112.0),
                (113.0, 120.5),
                // DROP 2
                (129.0, 136.75),
                (145.0, 152.5),
                (153.0, 160.0),
                (161.0, 168.75),  // drops at 169
            ]),
        TrackArrangement::new("ARP 2", vec![
                (121.0, 128.0),
                (137.0, 144.0),
                (145.0, 152.5),
                (153.0, 160.0),
                (161.0, 168.75),  // drops at 169
            ]),

        // === KEYS (piano/organ - similar to pads but with more rhythmic presence) ===
        TrackArrangement::new("KEYS 1", vec![
                // BREAKDOWN - keys shine here
                (65.0, 72.5),
                (73.0, 80.0),
                (81.0, 88.5),
                (89.0, 96.0),
                // DROP 2 - add texture
                (129.0, 136.75),
                (137.0, 144.0),
                (145.0, 152.5),
                (153.0, 160.0),
            ]),
        TrackArrangement::new("KEYS 2", vec![
                (73.0, 80.0),
                (81.0, 88.5),
                (145.0, 152.5),
                (153.0, 160.0),
            ]),

        // === FX - RISERS (CONTINUE THROUGH FILL GAPS for seamless tension) ===
        TrackArrangement::new("RISER 1", vec![
            (25.0, 33.0),     // pre-build (through fill gap into build)
            (57.0, 65.0),     // pre-breakdown (through fill gap)
            (89.0, 97.0),     // PRE-DROP 1 - the big one! (through to drop)
            (121.0, 129.0),   // mid drop 1 (through fill gap)
            (153.0, 161.0),   // pre-fadedown (through fill gap)
            (185.0, 193.0),   // pre-outro (through fill gap)
        ]),
        TrackArrangement::new("RISER 2", vec![
            (9.0, 17.0),      // early intro tension (through fill gap)
            (41.0, 49.0),     // mid build (through fill gap)
            (89.0, 97.0),     // PRE-DROP 1 - layer (through to drop)
            (137.0, 145.0),   // mid drop 2 (through fill gap)
            (177.0, 185.0),   // fadedown tension (through fill gap)
        ]),
        TrackArrangement::new("RISER 3", vec![
            (13.0, 17.0),     // intro accent (through fill gap)
            (29.0, 33.0),     // pre-build accent (through fill gap)
            (45.0, 49.0),     // build accent (through fill gap)
            (61.0, 65.0),     // pre-breakdown (through fill gap)
            (77.0, 81.0),     // breakdown tension (through fill gap)
            (93.0, 97.0),     // PRE-DROP final 4 (through to drop)
            (109.0, 113.0),   // drop 1 accent (through fill gap)
            (125.0, 129.0),   // end drop 1 (through fill gap)
            (141.0, 145.0),   // drop 2 accent (through fill gap)
            (157.0, 161.0),   // end drop 2 (through fill gap)
            (173.0, 177.0),   // fadedown accent (through fill gap)
            (189.0, 193.0),   // pre-outro (through fill gap)
        ]),
        TrackArrangement::new("RISER 4", vec![
            (5.0, 9.0),       // early intro
            (21.0, 25.0),     // intro mid
            (37.0, 41.0),     // intro end
            (53.0, 57.0),     // pre-breakdown
            (69.0, 73.0),     // breakdown mid
            (85.0, 89.0),     // pre-drop
            (101.0, 105.0),   // drop 1 early
            (117.0, 121.0),   // drop 1 mid
            (133.0, 137.0),   // drop 1 end / drop 2 start
            (149.0, 153.0),   // drop 2 mid
            (165.0, 169.0),   // drop 2 end
            (181.0, 185.0),   // fadedown mid
        ]),

        // === FX - SNARE ROLLS (critical for tension!) ===
        TrackArrangement::new("SNARE ROLL 1", vec![
            (29.0, 33.0),     // pre-build (through fill gap into build)
            (61.0, 65.0),     // pre-breakdown (through fill gap)
            (89.0, 97.0),     // PRE-DROP 1 - full roll into the drop!
            (125.0, 129.0),   // end drop 1 (through fill gap)
            (153.0, 161.0),   // pre-fadedown (through fill gap)
            (189.0, 193.0),   // pre-outro (through fill gap)
        ]),
        TrackArrangement::new("SNARE ROLL 2", vec![
            (61.0, 65.0),     // pre-breakdown
            (89.0, 97.0),     // PRE-DROP 1
            (153.0, 161.0),   // pre-fadedown
        ]),
        TrackArrangement::new("SNARE ROLL 3", vec![
            (89.0, 97.0),     // PRE-DROP 1 - the big one
            (153.0, 161.0),   // pre-fadedown
        ]),
        TrackArrangement::new("SNARE ROLL 4", vec![
            (89.0, 97.0),     // PRE-DROP 1 only - maximum impact
        ]),

        // === FX - DRUM FILLS (randomized per generation) ===
        // Generated by generate_random_fills() - see that function for logic

        // === FX - REVERSE CRASHES (2 samples alternating) ===
        TrackArrangement::new("REVERSE 1", vec![
                (16.0, 17.0),     // bar 16
                (48.0, 49.0),     // bar 48
                (80.0, 81.0),     // bar 80
                (112.0, 113.0),   // bar 112
                (144.0, 145.0),   // bar 144
                (176.0, 177.0),   // bar 176
            ]),
        TrackArrangement::new("REVERSE 2", vec![
                (32.0, 33.0),     // bar 32, into build
                (64.0, 65.0),     // bar 64, into breakdown
                (96.0, 97.0),     // bar 96, INTO DROP 1
                (128.0, 129.0),   // bar 128, into drop 2
                (160.0, 161.0),   // bar 160, into fadedown
                (192.0, 193.0),   // bar 192, into outro
            ]),

        // === FX - SUB DROP (layered in breakdown: 65, 73, 81, 89) ===
        TrackArrangement::new("SUB DROP 1", vec![
                (65.0, 65.0), (73.0, 73.0), (81.0, 81.0), (89.0, 89.0),
            ]),
        TrackArrangement::new("SUB DROP 2", vec![
                (73.0, 73.0), (81.0, 81.0), (89.0, 89.0),
            ]),
        TrackArrangement::new("SUB DROP 3", vec![
                (81.0, 81.0), (89.0, 89.0),
            ]),
        TrackArrangement::new("SUB DROP 4", vec![
                (89.0, 89.0),
            ]),

        // === FX - BOOM KICK (layered in breakdown: 65, 73, 81, 89) ===
        TrackArrangement::new("BOOM KICK 1", vec![
                (65.0, 65.0), (73.0, 73.0), (81.0, 81.0), (89.0, 89.0),
            ]),
        TrackArrangement::new("BOOM KICK 2", vec![
                (73.0, 73.0), (81.0, 81.0), (89.0, 89.0),
            ]),
        TrackArrangement::new("BOOM KICK 3", vec![
                (81.0, 81.0), (89.0, 89.0),
            ]),
        TrackArrangement::new("BOOM KICK 4", vec![
                (89.0, 89.0),
            ]),

        // === FX - DOWNLIFTERS (layered like risers) ===
        TrackArrangement::new("DOWNLIFTER 1", vec![
                (33.0, 40.0),     // build start (energy down then up)
                (65.0, 72.0),     // into breakdown
                (97.0, 104.0),    // post-drop settle
                (129.0, 136.0),   // post-drop 2
                (161.0, 168.0),   // into fadedown
                (193.0, 200.0),   // into outro
            ]),
        TrackArrangement::new("DOWNLIFTER 2", vec![
                (65.0, 72.0),     // into breakdown
                (97.0, 104.0),    // post-drop settle
                (129.0, 136.0),   // post-drop 2
                (161.0, 168.0),   // into fadedown
            ]),
        TrackArrangement::new("DOWNLIFTER 3", vec![
                (97.0, 104.0),    // post-drop settle
                (129.0, 136.0),   // post-drop 2
            ]),
        TrackArrangement::new("DOWNLIFTER 4", vec![
                (129.0, 136.0),   // post-drop 2
            ]),

        // === FX - CRASH (2 layered tracks) ===
        TrackArrangement::new("CRASH", vec![
                (1.0, 1.0), (9.0, 9.0), (17.0, 17.0), (25.0, 25.0),
                (33.0, 33.0), (41.0, 41.0), (49.0, 49.0), (57.0, 57.0),
                (65.0, 65.0), (73.0, 73.0), (81.0, 81.0), (89.0, 89.0),
                (97.0, 97.0), (105.0, 105.0), (113.0, 113.0), (121.0, 121.0),
                (129.0, 129.0), (137.0, 137.0), (145.0, 145.0), (153.0, 153.0),
                (161.0, 161.0), (169.0, 169.0), (177.0, 177.0), (185.0, 185.0),
                (193.0, 193.0), (201.0, 201.0), (209.0, 209.0), (217.0, 217.0),
            ]),
        TrackArrangement::new("CRASH 2", vec![
                (1.0, 1.0), (17.0, 17.0), (33.0, 33.0), (49.0, 49.0),
                (65.0, 65.0), (81.0, 81.0), (97.0, 97.0), (113.0, 113.0),
                (129.0, 129.0), (145.0, 145.0), (161.0, 161.0), (177.0, 177.0),
                (193.0, 193.0), (209.0, 209.0),
            ]),

        // === FX - IMPACT (2 layered tracks) ===
        TrackArrangement::new("IMPACT", vec![
                (1.0, 1.0), (33.0, 33.0), (65.0, 65.0), (97.0, 97.0),
                (129.0, 129.0), (161.0, 161.0), (193.0, 193.0),
            ]),
        TrackArrangement::new("IMPACT 2", vec![
                (1.0, 1.0), (33.0, 33.0), (65.0, 65.0), (97.0, 97.0),
                (129.0, 129.0), (161.0, 161.0), (193.0, 193.0),
            ]),

        // === FX - HIT (2 layered tracks, offbeat accents) ===
        TrackArrangement::new("HIT", vec![
                (5.0, 5.0), (13.0, 13.0), (21.0, 21.0), (29.0, 29.0),
                (37.0, 37.0), (45.0, 45.0), (53.0, 53.0), (61.0, 61.0),
                (69.0, 69.0), (77.0, 77.0), (85.0, 85.0), (93.0, 93.0),
                (101.0, 101.0), (109.0, 109.0), (117.0, 117.0), (125.0, 125.0),
                (133.0, 133.0), (141.0, 141.0), (149.0, 149.0), (157.0, 157.0),
                (165.0, 165.0), (173.0, 173.0), (181.0, 181.0), (189.0, 189.0),
                (197.0, 197.0), (205.0, 205.0), (213.0, 213.0), (221.0, 221.0),
            ]),
        TrackArrangement::new("HIT 2", vec![
            (5.0, 5.0), (21.0, 21.0), (37.0, 37.0), (53.0, 53.0),
            (69.0, 69.0), (85.0, 85.0), (101.0, 101.0), (117.0, 117.0),
            (133.0, 133.0), (149.0, 149.0), (165.0, 165.0), (181.0, 181.0),
            (197.0, 197.0), (213.0, 213.0),
        ]),

        // SWEEPS - generated by generate_swoosh_arrangements() for rotation and layering

        // === ATMOSPHERE ===
        TrackArrangement::new("ATMOS", vec![
            (1.0, 64.0),
            (65.0, 96.0),
            (97.0, 224.0),    // through outro
        ]),
        TrackArrangement::new("ATMOS 2", vec![
            (65.0, 96.0),
            (129.0, 160.0),
        ]),
        TrackArrangement::new("VOX 1", vec![
            (81.0, 96.0),
            (113.0, 128.0),
            (145.0, 160.0),
        ]),
    ];
    
    // Add randomized fill arrangements
    base.extend(generate_random_fills());
    // Add randomized glitch arrangements (same positions as fills)
    base.extend(generate_glitch_arrangements());
    // Add randomized swoosh arrangements (sweeps up/down at 16-bar grid)
    base.extend(generate_swoosh_arrangements());
    
    // Apply chaos to arrangements (bar-level gaps)
    if chaos > 0.0 {
        let uniform_chaos = SectionValues::default();
        base = apply_chaos_to_arrangements(base, &uniform_chaos, chaos);
    }
    
    base
}

fn get_arrangement_with_params(chaos: f32, glitch_intensity: f32, section_overrides: &SectionOverrides, density: f32, variation: f32, parallelism: f32, scatter: f32, scatter_track_count: u32) -> Vec<TrackArrangement> {
    let mut arrangements = get_arrangement(chaos);
    
    // Apply chaos per-section (bar-level gaps within sections)
    let has_any_chaos = chaos > 0.0 || section_overrides.chaos.any();
    if has_any_chaos {
        arrangements = apply_chaos_per_section(arrangements, &section_overrides.chaos, chaos);
    }
    
    // Apply parallelism per-section - limit how many tracks of same type play at once
    let has_any_parallelism = parallelism < 1.0 || section_overrides.parallelism.any();
    if has_any_parallelism {
        arrangements = apply_parallelism_per_section(arrangements, &section_overrides.parallelism, parallelism, &section_overrides.variation, variation);
    }
    
    // Apply variation per-section - elements drop in/out more frequently
    let has_any_variation = variation > 0.0 || section_overrides.variation.any();
    if has_any_variation {
        arrangements = apply_variation_per_section(arrangements, &section_overrides.variation, variation);
    }
    
    // Apply density per-section (extra clips in dense sections)
    let has_any_density = density > 0.0 || section_overrides.density.any();
    if has_any_density {
        arrangements = apply_density_per_section(arrangements, &section_overrides.density, density);
    }
    
    // Add scattered one-shot hits on 1/16 grid
    // Uses per-section scatter overrides, falling back to global scatter parameter
    let has_any_scatter = scatter > 0.0 || section_overrides.scatter.any();
    if has_any_scatter {
        arrangements.extend(generate_scatter_hits(&section_overrides.scatter, scatter, scatter_track_count));
    }
    
    // Apply glitch edits (beat-level micro-edits, stutters, dropouts)
    // Uses per-section intensity from section_overrides.glitch
    arrangements = apply_glitch_edits(arrangements, glitch_intensity, &section_overrides.glitch);
    
    arrangements
}

/// Apply parallelism - limit how many tracks of the same type play simultaneously
/// parallelism 0.0 = one track at a time, 1.0 = all tracks play together
/// variation controls switch interval: 0.0 = every 16 bars, 1.0 = every 4 bars
fn apply_parallelism(arrangements: Vec<TrackArrangement>, section_parallelism: &SectionValues, global_parallelism: f32, section_variation: &SectionValues, global_variation: f32) -> Vec<TrackArrangement> {
    use std::collections::HashMap;

    // Group tracks by their base type (strip trailing numbers)
    let get_base_type = |name: &str| -> String {
        let trimmed = name.trim_end_matches(|c: char| c.is_ascii_digit() || c == ' ');
        trimmed.trim().to_string()
    };

    // One-shot/FX tracks that shouldn't have parallelism applied
    let exempt = ["FILL", "IMPACT", "CRASH", "RISER", "DOWNLIFTER", "SUB DROP",
                  "BOOM KICK", "SNARE ROLL", "GLITCH", "REVERSE", "SWEEP", "SCATTER"];

    // Group arrangements by base type
    let mut groups: HashMap<String, Vec<usize>> = HashMap::new();
    for (idx, arr) in arrangements.iter().enumerate() {
        if exempt.iter().any(|e| arr.name.starts_with(e)) {
            continue;
        }
        let base = get_base_type(&arr.name);
        groups.entry(base).or_default().push(idx);
    }

    // Switch interval: use 8 bars to match the section-overrides block grid
    let switch_bars: u32 = 8;

    let mut result = arrangements;

    // For each group with multiple tracks, thin out based on parallelism
    for (_base_type, indices) in groups.iter() {
        let group_size = indices.len();
        if group_size <= 1 {
            continue; // Single track, nothing to thin
        }

        // Find the overall time range across all tracks in group
        let mut min_start = f64::MAX;
        let mut max_end = 0.0f64;
        for &idx in indices {
            for &(start, end) in &result[idx].sections {
                min_start = min_start.min(start);
                max_end = max_end.max(end);
            }
        }

        if min_start >= max_end {
            continue;
        }

        // Divide into time slots and randomly assign which tracks are active
        let total_bars = (max_end - min_start) as u32;
        let num_slots = (total_bars / switch_bars).max(1);

        // For each track, determine which slots it's active
        let mut track_active_slots: Vec<Vec<bool>> = vec![vec![false; num_slots as usize]; group_size];

        // Build the slot assignments with a single borrow of the seeded RNG.
        // Resolve parallelism per 8-bar slot from section overrides.
        with_gen_rng(|rng| {
            #[allow(clippy::needless_range_loop)]
            for slot in 0..num_slots as usize {
                let slot_bar = (min_start as u32) + (slot as u32 * switch_bars);
                let parallelism = section_parallelism.value_at_bar(slot_bar.max(1), global_parallelism);
                let max_concurrent = ((group_size as f32 * parallelism).ceil() as usize).max(1).min(group_size);

                if max_concurrent >= group_size {
                    // All tracks active this slot
                    for track_idx in 0..group_size {
                        track_active_slots[track_idx][slot] = true;
                    }
                    continue;
                }

                // Randomly pick which tracks are active this slot
                let mut candidates: Vec<usize> = (0..group_size).collect();

                // Shuffle and take max_concurrent
                for i in (1..candidates.len()).rev() {
                    let j = rng.random_range(0..=i as u32) as usize;
                    candidates.swap(i, j);
                }

                for &track_idx in candidates.iter().take(max_concurrent) {
                    track_active_slots[track_idx][slot] = true;
                }
            }
        });
        
        // Apply the active slots to each track's sections
        for (group_idx, &arr_idx) in indices.iter().enumerate() {
            let arr = &mut result[arr_idx];
            let mut new_sections: Vec<(f64, f64)> = Vec::new();
            
            for &(start, end) in &arr.sections {
                // Split this section by slots and keep only active portions
                let mut current = start;
                while current < end {
                    let slot_idx = ((current - min_start) / switch_bars as f64) as usize;
                    let slot_idx = slot_idx.min(num_slots as usize - 1);
                    
                    let slot_start = min_start + (slot_idx as f64 * switch_bars as f64);
                    let slot_end = (slot_start + switch_bars as f64).min(max_end);
                    
                    // Guard against infinite loop: if slot_end didn't advance past current, break
                    if slot_end <= current {
                        break;
                    }
                    
                    let section_start = current.max(start);
                    let section_end = slot_end.min(end);
                    
                    if track_active_slots[group_idx][slot_idx] && section_start < section_end {
                        // Merge with previous if contiguous
                        if let Some(last) = new_sections.last_mut() {
                            if (last.1 - section_start).abs() < 0.001 {
                                last.1 = section_end;
                            } else {
                                new_sections.push((section_start, section_end));
                            }
                        } else {
                            new_sections.push((section_start, section_end));
                        }
                    }
                    
                    current = slot_end;
                }
            }
            
            arr.sections = new_sections;
        }
    }
    
    result
}

/// Apply variation to arrangements - elements drop in/out within sections
/// variation 0.0 = elements play full sections, 1.0 = constant movement
fn apply_variation(mut arrangements: Vec<TrackArrangement>, section_variation: &SectionValues, global_variation: f32) -> Vec<TrackArrangement> {
    with_gen_rng(|rng| {
        // Tracks that should NOT have variation applied (core rhythm, fills, one-shots)
        let protected = ["KICK", "FILL", "IMPACT", "CRASH", "RISER", "DOWNLIFTER", "SUB DROP",
                         "BOOM KICK", "SNARE ROLL", "GLITCH", "REVERSE", "SWEEP", "SCATTER"];

        // Tracks that can have moderate variation (drums keep groove)
        let light_variation = ["CLAP", "SNARE", "BASS", "SUB"];

        // Snap to 4-bar boundaries for musical phrasing
        let snap_4bar = |v: f64| -> f64 { (v / 4.0).round() * 4.0 };

        for arr in arrangements.iter_mut() {
            // Skip protected tracks
            if protected.iter().any(|p| arr.name.starts_with(p)) {
                continue;
            }

            let is_light = light_variation.iter().any(|p| arr.name.starts_with(p));

            let mut new_sections: Vec<(f64, f64)> = Vec::new();

            for &(start, end) in &arr.sections {
                let section_len = end - start;

                // Skip short sections
                if section_len < 8.0 {
                    new_sections.push((start, end));
                    continue;
                }

                // Resolve variation for the 8-bar block at this section's midpoint
                let variation = section_variation.value_at_bar(((start + end) / 2.0) as u32, global_variation);
                let effective_variation = if is_light { variation * 0.3 } else { variation };

                // Probability of breaking up this section increases with variation
                if !rng.random_bool(effective_variation as f64 * 0.7) {
                    new_sections.push((start, end));
                    continue;
                }

                // Break section into 4 or 8 bar chunks with gaps
                let chunk_size = if rng.random_bool(0.5) { 4.0 } else { 8.0 };
                let mut pos = start;

                while pos < end {
                    let chunk_end = (pos + chunk_size).min(end);

                    // Guard against infinite loop: if chunk_end didn't advance, break
                    if chunk_end <= pos {
                        break;
                    }

                    // Randomly decide to play or skip this chunk
                    // Higher variation = more skips
                    let play_prob = 1.0 - (effective_variation as f64 * 0.5);

                    if rng.random_bool(play_prob) {
                        // Play this chunk, maybe with shortened duration
                        let actual_end = if rng.random_bool(effective_variation as f64 * 0.3) {
                            // Cut chunk short by 1-2 bars
                            let cut = if rng.random_bool(0.5) { 1.0 } else { 2.0 };
                            (chunk_end - cut).max(pos + 2.0)
                        } else {
                            chunk_end
                        };

                        if actual_end > pos {
                            new_sections.push((snap_4bar(pos).max(start), snap_4bar(actual_end).min(end)));
                        }
                    }
                    // else: skip this chunk (gap)

                    pos = chunk_end;
                }
            }

            // Filter out invalid/duplicate sections and sort
            let mut filtered: Vec<(f64, f64)> = new_sections
                .into_iter()
                .filter(|(s, e)| e > s && *e - *s >= 2.0)
                .collect();
            filtered.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

            // Merge overlapping sections
            let mut merged: Vec<(f64, f64)> = Vec::new();
            for (s, e) in filtered {
                if let Some(last) = merged.last_mut()
                    && s <= last.1 {
                        last.1 = last.1.max(e);
                        continue;
                    }
                merged.push((s, e));
            }

            if !merged.is_empty() {
                arr.sections = merged;
            }
        }

        arrangements
    })
}

/// Apply chaos to arrangements: random gaps + call-and-response patterns
/// chaos 0.0 = no changes, 1.0 = maximum randomization
fn apply_chaos_to_arrangements(mut arrangements: Vec<TrackArrangement>, section_chaos: &SectionValues, global_chaos: f32) -> Vec<TrackArrangement> {
    with_gen_rng(|rng| {
        // Tracks that should NOT be chaotified (fills, one-shots, FX impacts)
        let protected_prefixes = ["FILL", "IMPACT", "CRASH", "RISER", "DOWNLIFTER", "SUB DROP", "BOOM KICK", "SNARE ROLL", "GLITCH", "REVERSE", "SWEEP"];

        // Core rhythm tracks - can only have tiny gaps (1-2 beats max)
        let core_rhythm_prefixes = ["KICK", "CLAP", "SNARE", "HAT", "BASS", "SUB"];

        // Tracks that can use call-and-response (melodic/harmonic elements)
        let call_response_prefixes = ["SYNTH", "PAD", "LEAD", "ARP"];

        for arr in arrangements.iter_mut() {
            // Skip protected tracks entirely
            if protected_prefixes.iter().any(|p| arr.name.starts_with(p)) {
                continue;
            }

            // Skip if too few sections
            if arr.sections.len() < 2 {
                continue;
            }

            let is_core_rhythm = core_rhythm_prefixes.iter().any(|p| arr.name.starts_with(p));

            // Snap to beat grid (0.25 bar = 1 beat) - Ableton requires integer beat values
            let snap = |v: f64| -> f64 { (v * 4.0).round() / 4.0 };

            // 1. Micro-gaps: punch small holes in sections (1-2 bars max for core, 2-4 bars for others)
            // This creates variation without losing the groove
            let mut new_sections: Vec<(f64, f64)> = Vec::new();

            for section in arr.sections.iter() {
                let (start, end) = *section;
                let section_len = end - start;

                // Only apply micro-gaps to sections longer than 4 bars
                if section_len < 4.0 {
                    new_sections.push((start, end));
                    continue;
                }

                // Resolve chaos for the 8-bar block at this section's midpoint
                let chaos = section_chaos.value_at_bar(((start + end) / 2.0) as u32, global_chaos);

                // Chance to add a micro-gap in this section
                let gap_chance = chaos * 0.4;
                if !rng.random_bool(gap_chance as f64) {
                    new_sections.push((start, end));
                    continue;
                }

                // Gap size: 1-2 bars for core rhythm, 2-4 bars for melodics/perc
                let max_gap = if is_core_rhythm { 2.0 } else { 4.0 };
                let min_gap = if is_core_rhythm { 1.0 } else { 2.0 };
                let gap_size = snap(min_gap + rng.random::<f64>() * (max_gap - min_gap));

                // Gap position: somewhere in the middle (not first 2 or last 2 bars)
                let margin = 2.0;
                let gap_range = section_len - gap_size - (margin * 2.0);
                if gap_range <= 0.0 {
                    new_sections.push((start, end));
                    continue;
                }

                let gap_start = snap(start + margin + rng.random::<f64>() * gap_range);
                let gap_end = snap(gap_start + gap_size);

                // Split section around the gap
                if gap_start > start + 1.0 {
                    new_sections.push((snap(start), gap_start));
                }
                if end > gap_end + 1.0 {
                    new_sections.push((gap_end, snap(end)));
                }
            }

            // Representative chaos for track-level decisions (midpoint of first section)
            let track_chaos = arr.sections.first()
                .map(|(s, e)| section_chaos.value_at_bar(((s + e) / 2.0) as u32, global_chaos))
                .unwrap_or(global_chaos);

            // 2. Call-and-response: for melodic tracks, shift some sections by 2-4 bars
            if call_response_prefixes.iter().any(|p| arr.name.starts_with(p)) {
                let has_number = arr.name.chars().last().map(|c| c.is_ascii_digit()).unwrap_or(false);
                if has_number {
                    let shift_chance = track_chaos * 0.3;
                    new_sections = new_sections.iter().map(|(start, end)| {
                        if rng.random_bool(shift_chance as f64) && *start >= 8.0 {
                            let shift = if rng.random_bool(0.5) { 2.0 } else { 4.0 };
                            (*start + shift, *end + shift)
                        } else {
                            (*start, *end)
                        }
                    }).collect();
                }
            }

            // 3. Staggered entry: for non-primary tracks, slightly delay first section
            let has_number = arr.name.chars().last().map(|c| c.is_ascii_digit()).unwrap_or(false);
            if has_number && !new_sections.is_empty() && !is_core_rhythm {
                let stagger_chance = track_chaos * 0.25;
                if rng.random_bool(stagger_chance as f64) {
                    // Delay first section by 2-4 bars (not remove it entirely)
                    let delay = if rng.random_bool(0.5) { 2.0 } else { 4.0 };
                    if let Some((start, end)) = new_sections.first_mut()
                        && *end - *start > delay + 2.0 {
                            *start += delay;
                        }
                }
            }

            arr.sections = new_sections;
        }

        arrangements
    })
}

// ============================================================================
// Per-section wrapper functions for all 6 override parameters
// These functions apply effects using per-section intensity values
// ============================================================================

/// Apply chaos per-section - uses section-specific chaos values
fn apply_chaos_per_section(arrangements: Vec<TrackArrangement>, section_chaos: &SectionValues, global_chaos: f32) -> Vec<TrackArrangement> {
    let has_any = global_chaos > 0.0 || section_chaos.any();
    if has_any {
        apply_chaos_to_arrangements(arrangements, section_chaos, global_chaos)
    } else {
        arrangements
    }
}

/// Apply parallelism per-section - uses section-specific parallelism values
fn apply_parallelism_per_section(arrangements: Vec<TrackArrangement>, section_parallelism: &SectionValues, global_parallelism: f32, section_variation: &SectionValues, global_variation: f32) -> Vec<TrackArrangement> {
    let has_any = global_parallelism < 1.0 || section_parallelism.any();
    if has_any {
        apply_parallelism(arrangements, section_parallelism, global_parallelism, section_variation, global_variation)
    } else {
        arrangements
    }
}

/// Apply variation per-section - uses section-specific variation values
fn apply_variation_per_section(arrangements: Vec<TrackArrangement>, section_variation: &SectionValues, global_variation: f32) -> Vec<TrackArrangement> {
    let has_any = global_variation > 0.0 || section_variation.any();
    if has_any {
        apply_variation(arrangements, section_variation, global_variation)
    } else {
        arrangements
    }
}

/// Apply density per-block — walks the standard 224-bar Techno arrangement in
/// 8-bar blocks, resolving density from the block override (if any) or the
/// global scalar, and inserting micro-accent clips on densifiable tracks where
/// the block's density roll succeeds.
///
/// 8-bar blocks give ~28 dice rolls across the song instead of 7 — the finer
/// grid both matches the timeline UI's granularity and produces noticeably more
/// varied accenting without changing average density.
fn apply_density_per_section(mut arrangements: Vec<TrackArrangement>, section_density: &SectionValues, global_density: f32) -> Vec<TrackArrangement> {
    with_gen_rng(|rng| {
        // 28 blocks of 8 bars each, covering bars 1..=224 (Techno standard).
        // TODO: respect per-genre SectionBounds total_bars once plumbed through.
        let block_starts: Vec<u32> = (1..=SONG_LENGTH_BARS).step_by(8).collect();

        // Tracks that can have density-based doubling
        let densifiable = ["HAT", "PERC", "SYNTH", "ARP", "PAD"];

        for arr in arrangements.iter_mut() {
            if !densifiable.iter().any(|p| arr.name.starts_with(p)) {
                continue;
            }

            let mut new_sections: Vec<(f64, f64)> = Vec::new();

            for &(start, end) in &arr.sections {
                new_sections.push((start, end));

                // Walk every 8-bar block that overlaps this clip, pulling the
                // block-specific density (falling back to the global scalar).
                for &block_start in &block_starts {
                    let density = section_density.value_at_bar(block_start, global_density);
                    if density <= 0.0 { continue; }

                    let block_end = block_start + 8;
                    let block_start_f = block_start as f64;
                    let block_end_f = block_end as f64;

                    // Does the clip overlap this block?
                    if start < block_end_f && end > block_start_f
                        && rng.random_bool(density as f64 * 0.4)
                    {
                        // Add a 1- or 2-bar accent clip somewhere inside the
                        // overlap between the source clip and this block.
                        let clip_len = if rng.random_bool(0.5) { 1.0 } else { 2.0 };
                        let lo = start.max(block_start_f);
                        let hi = end.min(block_end_f);
                        let span = ((hi - lo) as u32).max(1);
                        let clip_start = lo + rng.random_range(0..span) as f64;
                        let clip_end = (clip_start + clip_len).min(end).min(block_end_f);
                        if clip_end > clip_start {
                            new_sections.push((clip_start, clip_end));
                        }
                    }
                }
            }

            // Sort and dedupe
            new_sections.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
            arr.sections = new_sections;
        }

        arrangements
    })
}

/// Apply glitch edits to arrangements - micro-stutters, beat dropouts, and ramping effects.
/// This creates the hand-crafted, detailed editing that makes tracks sound professionally produced.
/// 
/// Glitch intensity controls:
/// - 0.0 = clean, no glitches
/// - 0.3 = subtle glitches (occasional stutters, rare dropouts)
/// - 0.6 = moderate glitches (frequent stutters, beat-level edits)
/// - 1.0 = heavy glitches (constant micro-edits, IDM-style chaos)
/// 
/// IMPORTANT: All positions must be multiples of 0.25 bars (1 beat) to produce valid ALS XML.
/// Ableton expects integer beat values for CurrentStart/CurrentEnd.
fn apply_glitch_edits(mut arrangements: Vec<TrackArrangement>, glitch_intensity: f32, section_glitch: &SectionValues) -> Vec<TrackArrangement> {
    // Check if any glitch is enabled at all — either the global scalar is
    // non-trivial, or some per-block override pins a non-trivial value.
    let has_any_glitch = glitch_intensity > 0.05
        || section_glitch.values().any(|v| v > 0.05);
    
    if !has_any_glitch {
        return arrangements;
    }

    with_gen_rng(|rng| {
    // Snap to beat grid (0.25 bar = 1 beat)
    let snap = |v: f64| -> f64 { (v * 4.0).round() / 4.0 };

    // Tracks that get different glitch treatments
    let kick_tracks = ["KICK"];
    let drum_tracks = ["CLAP", "SNARE", "HAT", "PERC", "RIDE"];
    let bass_tracks = ["BASS", "SUB"];
    let melodic_tracks = ["SYNTH", "PAD", "LEAD", "ARP"];

    // Tracks that should NOT be glitched (one-shots, FX)
    let protected = ["FILL", "IMPACT", "CRASH", "RISER", "DOWNLIFTER", "SUB DROP", "BOOM KICK",
                     "SNARE ROLL", "GLITCH", "REVERSE", "SWEEP", "ATMOS", "VOX", "SCATTER"];
    
    for arr in arrangements.iter_mut() {
        // Skip protected tracks
        if protected.iter().any(|p| arr.name.starts_with(p)) {
            continue;
        }
        
        let is_kick = kick_tracks.iter().any(|p| arr.name.starts_with(p));
        let is_drum = drum_tracks.iter().any(|p| arr.name.starts_with(p));
        let is_bass = bass_tracks.iter().any(|p| arr.name.starts_with(p));
        let is_melodic = melodic_tracks.iter().any(|p| arr.name.starts_with(p));
        
        let mut new_sections: Vec<(f64, f64)> = Vec::new();
        
        for section in arr.sections.iter() {
            let (start, end) = *section;
            let section_len = end - start;
            
            // Skip very short sections
            if section_len < 1.0 {
                new_sections.push((snap(start), snap(end)));
                continue;
            }
            
            // Process the section bar by bar, adding glitch edits
            let mut current = snap(start);
            let end_snapped = snap(end);
            while current < end_snapped {
                let bar_end = snap((current + 1.0).min(end_snapped));
                // Guard against infinite loop: if bar_end didn't advance, force it forward
                if bar_end <= current {
                    current = end_snapped;
                    break;
                }
                let bar_num = current as u32;
                
                // Get section-aware glitch intensity for this bar
                // Bar numbers in arrangement are 1-based (INTRO starts at bar 1)
                let gi = section_glitch.value_at_bar(bar_num, glitch_intensity) as f64;
                
                // Skip this bar if no glitch intensity
                if gi < 0.05 {
                    new_sections.push((current, bar_end));
                    current = bar_end;
                    continue;
                }
                
                // === KICK GLITCHES ===
                if is_kick {
                    // Stutter before phrase boundaries (every 4 or 8 bars)
                    let is_pre_phrase = bar_num > 0 && (bar_num % 4 == 3 || bar_num % 8 == 7);
                    
                    // High intensity: stutter on pre-phrase bars
                    if is_pre_phrase && rng.random_bool(gi * 0.8) {
                        // Beats 1-2 normal, beats 3-4 stutter (on-off-on-off)
                        new_sections.push((current, current + 0.5));
                        new_sections.push((current + 0.5, current + 0.75));
                        // beat 4 gap
                        current = bar_end;
                        continue;
                    }
                    
                    // Random 1-beat dropouts throughout (more frequent at high intensity)
                    if rng.random_bool(gi * 0.25) {
                        // Drop a random beat (2, 3, or 4)
                        let drop_beat = rng.random_range(1..4) as f64 * 0.25;
                        new_sections.push((current, current + drop_beat));
                        if drop_beat + 0.25 < 1.0 {
                            new_sections.push((current + drop_beat + 0.25, bar_end));
                        }
                        current = bar_end;
                        continue;
                    }
                }
                
                // === DRUM STUTTERS (hats, percs, snares) ===
                if is_drum {
                    // Frequent beat dropouts
                    if rng.random_bool(gi * 0.35) {
                        // Multiple patterns:
                        let pattern = rng.random_range(0..4);
                        match pattern {
                            0 => {
                                // Drop beat 2
                                new_sections.push((current, current + 0.25));
                                new_sections.push((current + 0.5, bar_end));
                            }
                            1 => {
                                // Drop beat 3
                                new_sections.push((current, current + 0.5));
                                new_sections.push((current + 0.75, bar_end));
                            }
                            2 => {
                                // Drop beats 2-3 (stutter effect)
                                new_sections.push((current, current + 0.25));
                                new_sections.push((current + 0.75, bar_end));
                            }
                            _ => {
                                // Syncopated: only beats 1 and 3
                                new_sections.push((current, current + 0.25));
                                new_sections.push((current + 0.5, current + 0.75));
                            }
                        }
                        current = bar_end;
                        continue;
                    }
                }
                
                // === BASS GLITCHES ===
                if is_bass {
                    // Beat gaps and tail cuts
                    if rng.random_bool(gi * 0.3) {
                        let pattern = rng.random_range(0..3);
                        match pattern {
                            0 => {
                                // Gap at beat 2
                                new_sections.push((current, current + 0.25));
                                new_sections.push((current + 0.5, bar_end));
                            }
                            1 => {
                                // Tail cut - play 3 beats only
                                new_sections.push((current, current + 0.75));
                            }
                            _ => {
                                // Gap at beat 3
                                new_sections.push((current, current + 0.5));
                                new_sections.push((current + 0.75, bar_end));
                            }
                        }
                        current = bar_end;
                        continue;
                    }
                }
                
                // === MELODIC GLITCHES ===
                if is_melodic {
                    // Frequent stutters and dropouts
                    if rng.random_bool(gi * 0.4) {
                        let pattern = rng.random_range(0..4);
                        match pattern {
                            0 => {
                                // Beat 1 only (hard stutter)
                                new_sections.push((current, current + 0.25));
                            }
                            1 => {
                                // Beats 1 and 3 only (syncopated)
                                new_sections.push((current, current + 0.25));
                                new_sections.push((current + 0.5, current + 0.75));
                            }
                            2 => {
                                // Drop beat 2
                                new_sections.push((current, current + 0.25));
                                new_sections.push((current + 0.5, bar_end));
                            }
                            _ => {
                                // First half only
                                new_sections.push((current, current + 0.5));
                            }
                        }
                        current = bar_end;
                        continue;
                    }
                }
                
                // No glitch applied, add bar normally
                new_sections.push((current, bar_end));
                current = bar_end;
            }
        }
        
        // Filter out invalid sections (don't merge - we want the gaps!)
        // Keep one-shots (e == s) — they're valid single-hit placements.
        let filtered: Vec<(f64, f64)> = new_sections
            .into_iter()
            .map(|(s, e)| (snap(s), snap(e)))
            .filter(|(s, e)| e >= s)
            .collect();
        
        arr.sections = filtered;
    }

    arrangements
    })
}

const GROUP_TRACK_TEMPLATE: &str = include_str!("group_track_template.xml");
const MASTER_EQ8_HPF_TEMPLATE: &str = include_str!("master_eq8_hpf_template.xml");
const MASTER_LIMITER_TEMPLATE: &str = include_str!("master_limiter_template.xml");
const GROUP_SIDECHAIN_COMPRESSOR_TEMPLATE: &str =
    include_str!("group_sidechain_compressor_template.xml");

const DRUMS_COLOR: u32 = 69;
const BASS_COLOR: u32 = 13;
const MELODICS_COLOR: u32 = 26;
const FX_COLOR: u32 = 57;

struct IdAllocator {
    next_id: AtomicU32,
    used_ids: std::sync::Mutex<HashSet<u32>>,
}

impl IdAllocator {
    fn new(start: u32) -> Self {
        Self {
            next_id: AtomicU32::new(start),
            used_ids: std::sync::Mutex::new(HashSet::new()),
        }
    }

    fn alloc(&self) -> u32 {
        loop {
            let id = self.next_id.fetch_add(1, Ordering::SeqCst);
            let mut used = self.used_ids.lock().unwrap();
            if !used.contains(&id) {
                used.insert(id);
                return id;
            }
        }
    }

    fn reserve(&self, id: u32) {
        let mut used = self.used_ids.lock().unwrap();
        used.insert(id);
    }

    fn max_id(&self) -> u32 {
        self.next_id.load(Ordering::SeqCst)
    }
}

#[derive(Clone)]
struct SampleInfo {
    path: String,
    name: String,
    file_size: u64,
    duration_secs: f64,
    bpm: Option<f64>,
}

impl SampleInfo {
    fn from_db(
        path: &str,
        db_duration: f64,
        db_size: u64,
        db_bpm: Option<f64>,
        sample_rate: u32,
        channels: u16,
        bits_per_sample: u16,
    ) -> SampleInfo {
        let name = Path::new(path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("sample")
            .to_string();

        // Trust DB values - don't hit disk for every sample (network mounts are slow)
        let file_size = if db_size > 0 { db_size } else { 0 };
        
        // Prefer BPM from filename (most reliable) over DB metadata
        let filename_bpm = crate::sample_analysis::extract_bpm(&name).map(|b| b as f64);
        let bpm = filename_bpm.or(db_bpm);
        
        // Duration validation: must be at least 0.5s to be valid
        // Many samples in DB have incorrect durations (e.g., 0.002s when should be 3.8s)
        // because WAV header parsing failed for non-standard chunk layouts.
        // If duration looks invalid, estimate from file size using actual audio format.
        // 
        // Validate format values - DB can have garbage like sample_rate=1818386796
        let valid_sample_rate = sample_rate > 0 && sample_rate <= 192000;
        let valid_channels = channels > 0 && channels <= 8;
        let valid_bits = bits_per_sample >= 8 && bits_per_sample <= 64;
        
        let duration_secs = if db_duration >= 0.5 {
            db_duration
        } else if file_size > 0 && valid_sample_rate && valid_channels && valid_bits {
            // Calculate bytes per second using actual format info (use u64 to avoid overflow)
            let bytes_per_sample = ((bits_per_sample as u64) + 7) / 8;
            let bytes_per_second = (sample_rate as u64) * (channels as u64) * bytes_per_sample;
            let estimated_duration = file_size as f64 / bytes_per_second as f64;
            
            // For loops with BPM, quantize to standard bar lengths
            if let Some(sample_bpm) = bpm {
                if name.to_lowercase().contains("loop") {
                    let estimated_bars = (estimated_duration * sample_bpm) / (60.0 * 4.0);
                    let bars = if estimated_bars <= 1.5 { 1.0 }
                        else if estimated_bars <= 3.0 { 2.0 }
                        else if estimated_bars <= 6.0 { 4.0 }
                        else if estimated_bars <= 12.0 { 8.0 }
                        else { estimated_bars.min(32.0) };
                    (bars * 4.0 * 60.0) / sample_bpm
                } else {
                    // Has BPM but not a loop - use estimated duration directly
                    estimated_duration.max(0.1)
                }
            } else {
                // No BPM - use estimated duration directly (one-shots, etc.)
                estimated_duration.max(0.1)
            }
        } else {
            // No file size or format info - use 1 second as last resort fallback
            1.0
        };

        SampleInfo {
            path: path.to_string(),
            name,
            file_size,
            duration_secs,
            bpm,
        }
    }

    /// Detect if this sample is a loop vs one-shot.
    /// 
    /// Hierarchy (first match wins):
    /// 1. Path contains loop folder patterns → loop
    /// 2. Path contains one-shot folder patterns → one-shot
    /// 3. Filename contains "loop" → loop
    /// 4. Filename contains one-shot indicators → one-shot
    /// 5. Default: one-shot (safer assumption - won't stretch incorrectly)
    ///
    /// NOTE: We intentionally do NOT use BPM metadata for loop detection.
    /// Audio analysis can detect BPM from one-shots (transient patterns),
    /// so BPM presence is not a reliable indicator of loopiness.
    fn is_loop(&self, _project_bpm: f64) -> bool {
        let path_lower = self.path.to_lowercase();
        let name_lower = self.name.to_lowercase();
        
        // 1. Path contains one-shot folders or hit folders → one-shot (check FIRST)
        if path_lower.contains("/one-shots/") || path_lower.contains("\\one-shots\\") 
            || path_lower.contains("/oneshots/") || path_lower.contains("\\oneshots\\")
            || path_lower.contains("/one_shots/") || path_lower.contains("\\one_shots\\")
            || path_lower.contains("/one-shot/") || path_lower.contains("\\one-shot\\")
            || path_lower.contains("/hits/") || path_lower.contains("\\hits\\")
            || path_lower.contains("_hits/") || path_lower.contains("_hits\\")
            || path_lower.contains("/drum_hits/") || path_lower.contains("\\drum_hits\\")
            || path_lower.contains("unlooped") {
            return false;
        }
        
        // 2. Filename has one-shot indicators → one-shot
        if name_lower.contains("one_shot") || name_lower.contains("one-shot") || name_lower.contains("oneshot")
            || name_lower.contains("one shot")
            || name_lower.contains("_hit_") || name_lower.contains("_hit.")
            || name_lower.contains("_shot_") || name_lower.contains("_shot.")
            || name_lower.contains("_stab_") || name_lower.contains("_stab.") {
            return false;
        }
        
        // 3. Path contains loop folder patterns → loop
        // Be specific to avoid false positives like "loopmasters" brand name
        if path_lower.contains("/loops/") || path_lower.contains("\\loops\\") 
            || path_lower.contains("/loop/") || path_lower.contains("\\loop\\")
            || path_lower.contains("_loops/") || path_lower.contains("_loops\\")
            || path_lower.contains("_loop/") || path_lower.contains("_loop\\")
            || path_lower.contains(" loops/") || path_lower.contains(" loops\\")
            || path_lower.contains("/pads/") || path_lower.contains("\\pads\\")
            || path_lower.contains("/pad/") || path_lower.contains("\\pad\\")
            || path_lower.contains(" pads/") || path_lower.contains(" pads\\")
            || path_lower.contains("/synth pads/") || path_lower.contains("\\synth pads\\")
            || path_lower.contains("/leads/") || path_lower.contains("\\leads\\")
            || path_lower.contains("/lead/") || path_lower.contains("\\lead\\")
            || path_lower.contains("/arps/") || path_lower.contains("\\arps\\")
            || path_lower.contains("/arp/") || path_lower.contains("\\arp\\")
            || path_lower.contains("/synths/") || path_lower.contains("\\synths\\")
            || path_lower.contains("/synth/") || path_lower.contains("\\synth\\")
            || path_lower.contains("/bass/") || path_lower.contains("\\bass\\")
            || path_lower.contains("/basslines/") || path_lower.contains("\\basslines\\")
            || path_lower.contains("/melodic/") || path_lower.contains("\\melodic\\")
            || path_lower.contains("/music loops/") || path_lower.contains("\\music loops\\")
            || path_lower.contains("/atmosphere/") || path_lower.contains("\\atmosphere\\")
            || path_lower.contains("/atmospheres/") || path_lower.contains("\\atmospheres\\")
            || path_lower.contains("/drone/") || path_lower.contains("\\drone\\")
            || path_lower.contains("/drones/") || path_lower.contains("\\drones\\") {
            return true;
        }
        
        // 4. Filename has "loop" as word boundary → loop
        // Check for _loop, loop_, " loop", "loop " patterns, NOT "loopmasters" etc.
        if name_lower.contains("_loop") || name_lower.contains("loop_")
            || name_lower.contains(" loop") || name_lower.contains("loop ")
            || name_lower.starts_with("loop") || name_lower.ends_with("loop")
            || name_lower.contains("-loop") || name_lower.contains("loop-") {
            return true;
        }
        
        // 5. Default: one-shot (safer - won't stretch incorrectly)
        false
    }

    /// Calculate the loop length in bars based on the sample's actual duration and BPM.
    /// 
    /// Uses the sample's original BPM (from filename or metadata) to determine how many
    /// bars the sample represents. Quantizes to standard loop lengths: 1, 2, 4, 8, 16, 32.
    /// 
    /// The project_bpm is only used as a fallback when the sample has no BPM metadata,
    /// in which case we assume the sample was recorded at the project tempo.
    fn loop_bars(&self, project_bpm: f64) -> u32 {
        // Use the sample's original BPM to calculate bar length.
        // Only fall back to project BPM if sample BPM is unknown.
        let sample_bpm = self.bpm.unwrap_or(project_bpm);
        
        let duration = if self.duration_secs <= 0.0 || self.duration_secs > 300.0 {
            // Invalid duration - assume 4 bars at project tempo as fallback
            (4.0 * 60.0 * 4.0) / project_bpm
        } else {
            self.duration_secs
        };

        if sample_bpm <= 0.0 {
            return 4; // Fallback for invalid BPM
        }
        
        // Calculate actual bar count: duration_secs * bpm / (60 * beats_per_bar)
        // At 120 BPM, 1 bar (4 beats) = 2 seconds
        let bars = (duration * sample_bpm) / (60.0 * 4.0);
        
        // Quantize to standard loop lengths (1, 2, 4, 8, 16, 32 bars)
        // Using midpoint thresholds for rounding to nearest power of 2
        if bars <= 1.5 { 1 }
        else if bars <= 3.0 { 2 }
        else if bars <= 6.0 { 4 }
        else if bars <= 12.0 { 8 }
        else if bars <= 24.0 { 16 }
        else { 32 }
    }

    fn xml_path(&self) -> String {
        self.path
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&apos;")
    }

    fn xml_name(&self) -> String {
        self.name
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&apos;")
    }
}

fn open_dedicated_conn() -> Result<rusqlite::Connection, String> {
    let db_path = crate::history::get_data_dir().join("audio_haxor.db");
    let conn = rusqlite::Connection::open_with_flags(
        &db_path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    ).map_err(|e| format!("Failed to open DB: {}", e))?;
    conn.busy_timeout(std::time::Duration::from_secs(5)).ok();
    Ok(conn)
}

fn pick_random_key() -> String {
    write_app_log("[techno_generator] pick_random_key: start".into());
    let conn = match open_dedicated_conn() {
        Ok(c) => c,
        Err(e) => {
            write_app_log(format!("[techno_generator] pick_random_key: DB error: {}", e));
            return "A Minor".to_string();
        }
    };

    // Pick a key that has enough melodic samples (bass/synth/lead/pad/arp)
    // Focus on keys with actual melodic content, not just any samples
    let query = "SELECT s.key_name, COUNT(*) as cnt
                 FROM audio_library al
                 JOIN audio_samples s ON al.sample_id = s.id
                 WHERE s.key_name IS NOT NULL AND s.key_name != ''
                 AND (al.path LIKE '%bass%' OR al.path LIKE '%synth%' OR al.path LIKE '%lead%'
                      OR al.path LIKE '%pad%' OR al.path LIKE '%arp%' OR al.path LIKE '%melody%')
                 GROUP BY s.key_name
                 HAVING COUNT(*) >= 15
                 ORDER BY RANDOM() LIMIT 1";

    let key = conn.query_row(query, [], |row| row.get(0))
        .unwrap_or_else(|_| "A Minor".to_string());
    write_app_log(format!("[techno_generator] pick_random_key: selected '{}'", key));
    key
}

// Hardness patterns - samples with these in path are "harder"
const HARD_PATTERNS: &[&str] = &[
    "hard", "distort", "industrial", "schranz", "aggressive", "brutal", 
    "raw", "crushing", "pummel", "grind", "destroy", "destructive",
    "abrasive", "rave", "gabber", "hardcore", "acid", "drive", "gritty",
    "nasty", "dirty", "filthy", "intense", "heavy", "punish", "relentless",
];

// Soft patterns - samples with these in path are "softer"  
const SOFT_PATTERNS: &[&str] = &[
    "soft", "smooth", "mellow", "gentle", "ambient", "chill", "deep",
    "minimal", "subtle", "warm", "lush", "dreamy", "ethereal", "delicate",
    "clean", "pure", "light", "airy", "floating", "serene", "calm",
];

// Thread-local hardness for query functions
std::thread_local! {
    static CURRENT_HARDNESS: std::cell::Cell<f32> = const { std::cell::Cell::new(0.3) };
    static USED_SAMPLES: std::cell::RefCell<HashSet<String>> = std::cell::RefCell::new(HashSet::new());
}

// Persistent blacklist across generations - samples used in previous generations
// Persistent blacklist - stored in SQLite, cached in memory for fast lookups
// This ensures variety when generating multiple projects in a session AND across app restarts
use std::sync::Mutex;
use std::sync::atomic::AtomicBool;

static GENERATION_BLACKLIST: std::sync::LazyLock<Mutex<HashSet<String>>> = 
    std::sync::LazyLock::new(|| Mutex::new(HashSet::new()));
static BLACKLIST_LOADED: AtomicBool = AtomicBool::new(false);

// Directory whitelist - only use samples from these directories (if any are set)
static DIRECTORY_WHITELIST: std::sync::LazyLock<Mutex<HashSet<String>>> = 
    std::sync::LazyLock::new(|| Mutex::new(HashSet::new()));
static WHITELIST_LOADED: AtomicBool = AtomicBool::new(false);

/// Load blacklist from DB into memory cache (call once at startup or on first use)
fn ensure_blacklist_loaded() {
    if BLACKLIST_LOADED.swap(true, Ordering::SeqCst) {
        return; // Already loaded
    }
    if let Ok(entries) = crate::db::global().blacklist_list() {
        if let Ok(mut blacklist) = GENERATION_BLACKLIST.lock() {
            for entry in entries {
                blacklist.insert(entry);
            }
        }
        write_app_log(format!("[techno_generator] Loaded {} blacklist entries from DB", 
            GENERATION_BLACKLIST.lock().map(|b| b.len()).unwrap_or(0)));
    }
}

/// Load whitelist from DB into memory cache
fn ensure_whitelist_loaded() {
    if WHITELIST_LOADED.swap(true, Ordering::SeqCst) {
        return; // Already loaded
    }
    if let Ok(entries) = crate::db::global().whitelist_list() {
        if let Ok(mut whitelist) = DIRECTORY_WHITELIST.lock() {
            for entry in entries {
                whitelist.insert(entry);
            }
        }
        write_app_log(format!("[techno_generator] Loaded {} whitelist entries from DB", 
            DIRECTORY_WHITELIST.lock().map(|w| w.len()).unwrap_or(0)));
    }
}

/// Check if a sample path is allowed by the whitelist (empty whitelist = all allowed)
fn is_path_whitelisted(path: &str) -> bool {
    ensure_whitelist_loaded();
    if let Ok(whitelist) = DIRECTORY_WHITELIST.lock() {
        if whitelist.is_empty() {
            return true; // No whitelist = all allowed
        }
        // Check if path starts with any whitelisted directory
        for dir in whitelist.iter() {
            if path.starts_with(dir) {
                return true;
            }
        }
        // Log first rejection for debugging
        static LOGGED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
        if !LOGGED.swap(true, Ordering::Relaxed) {
            let dirs: Vec<_> = whitelist.iter().take(3).cloned().collect();
            write_app_log(format!("[whitelist] Rejected path: {} (whitelist dirs: {:?})", path, dirs));
        }
        false
    } else {
        true // On lock error, allow all
    }
}

fn set_hardness(h: f32) {
    CURRENT_HARDNESS.with(|c| c.set(h));
}

fn get_hardness() -> f32 {
    CURRENT_HARDNESS.with(|c| c.get())
}

fn clear_used_samples() {
    USED_SAMPLES.with(|s| s.borrow_mut().clear());
}

fn mark_sample_used(path: &str) {
    USED_SAMPLES.with(|s| s.borrow_mut().insert(path.to_string()));
    // Also add to persistent blacklist using key-stripped path
    // This prevents selecting the same sample in different keys
    let key_agnostic_path = crate::sample_analysis::strip_key_from_path(path);
    
    // Also extract and blacklist the filename (key-stripped)
    let filename = std::path::Path::new(path)
        .file_name()
        .and_then(|f| f.to_str())
        .unwrap_or("");
    let key_agnostic_filename = crate::sample_analysis::strip_key_from_path(filename);
    
    // Add both to in-memory cache
    if let Ok(mut blacklist) = GENERATION_BLACKLIST.lock() {
        blacklist.insert(key_agnostic_path.clone());
        if !key_agnostic_filename.is_empty() {
            blacklist.insert(key_agnostic_filename.clone());
        }
    }
    // Persist both to DB
    let _ = crate::db::global().blacklist_add(&key_agnostic_path);
    if !key_agnostic_filename.is_empty() {
        let _ = crate::db::global().blacklist_add(&key_agnostic_filename);
    }
}

fn is_sample_used(path: &str) -> bool {
    // Check current generation with exact path
    let in_current = USED_SAMPLES.with(|s| s.borrow().contains(path));
    if in_current {
        return true;
    }
    // Ensure blacklist is loaded from DB
    ensure_blacklist_loaded();
    
    // Check persistent blacklist with key-stripped path AND filename
    let key_agnostic_path = crate::sample_analysis::strip_key_from_path(path);
    let filename = std::path::Path::new(path)
        .file_name()
        .and_then(|f| f.to_str())
        .unwrap_or("");
    let key_agnostic_filename = crate::sample_analysis::strip_key_from_path(filename);
    
    if let Ok(blacklist) = GENERATION_BLACKLIST.lock() {
        // Match if either full path or just filename is blacklisted
        if blacklist.contains(&key_agnostic_path) {
            return true;
        }
        if !key_agnostic_filename.is_empty() && blacklist.contains(&key_agnostic_filename) {
            return true;
        }
    }
    false
}

/// Clear the persistent blacklist (call when user wants fresh samples)
pub fn clear_sample_blacklist() {
    // Clear in-memory cache
    if let Ok(mut blacklist) = GENERATION_BLACKLIST.lock() {
        let count = blacklist.len();
        blacklist.clear();
        write_app_log(format!("[techno_generator] Cleared sample blacklist ({} samples)", count));
    }
    // Clear from DB
    let _ = crate::db::global().blacklist_clear();
}

/// Get the number of samples in the blacklist
pub fn get_blacklist_count() -> usize {
    ensure_blacklist_loaded();
    GENERATION_BLACKLIST.lock().map(|b| b.len()).unwrap_or(0)
}

/// Get all blacklisted sample paths (key-stripped)
pub fn get_blacklist_entries() -> Vec<String> {
    ensure_blacklist_loaded();
    let mut entries: Vec<String> = GENERATION_BLACKLIST
        .lock()
        .map(|b| b.iter().cloned().collect())
        .unwrap_or_default();
    entries.sort();
    entries
}

/// Add a path or filename to the blacklist (key-stripped automatically)
/// If it looks like a full path, also blacklists the filename separately
pub fn add_to_blacklist(path: &str) {
    let key_agnostic = crate::sample_analysis::strip_key_from_path(path);
    
    // Add to in-memory cache
    if let Ok(mut blacklist) = GENERATION_BLACKLIST.lock() {
        blacklist.insert(key_agnostic.clone());
    }
    // Persist to DB
    let _ = crate::db::global().blacklist_add(&key_agnostic);
    
    // If it looks like a path (contains separator), also blacklist just the filename
    if path.contains('/') || path.contains('\\') {
        let filename = std::path::Path::new(path)
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or("");
        if !filename.is_empty() {
            let key_agnostic_filename = crate::sample_analysis::strip_key_from_path(filename);
            if let Ok(mut blacklist) = GENERATION_BLACKLIST.lock() {
                blacklist.insert(key_agnostic_filename.clone());
            }
            let _ = crate::db::global().blacklist_add(&key_agnostic_filename);
        }
    }
}

/// Remove a specific entry from the blacklist
pub fn remove_from_blacklist(entry: &str) -> bool {
    // Remove from in-memory cache
    let removed = if let Ok(mut blacklist) = GENERATION_BLACKLIST.lock() {
        blacklist.remove(entry)
    } else {
        false
    };
    // Remove from DB
    let _ = crate::db::global().blacklist_remove(entry);
    removed
}

// ── Directory Whitelist CRUD ──

/// Get the number of directories in the whitelist
pub fn get_whitelist_count() -> usize {
    ensure_whitelist_loaded();
    DIRECTORY_WHITELIST.lock().map(|w| w.len()).unwrap_or(0)
}

/// Get all whitelisted directories
pub fn get_whitelist_entries() -> Vec<String> {
    ensure_whitelist_loaded();
    let mut entries: Vec<String> = DIRECTORY_WHITELIST
        .lock()
        .map(|w| w.iter().cloned().collect())
        .unwrap_or_default();
    entries.sort();
    entries
}

/// Add a directory to the whitelist
pub fn add_to_whitelist(path: &str) {
    // Normalize path: remove trailing slashes, but preserve "/" (Unix root)
    let trimmed = path.trim();
    let normalized = if trimmed == "/" || trimmed == "\\" {
        trimmed
    } else {
        trimmed.trim_end_matches('/').trim_end_matches('\\')
    };
    if normalized.is_empty() {
        return;
    }
    
    // Add to in-memory cache
    if let Ok(mut whitelist) = DIRECTORY_WHITELIST.lock() {
        whitelist.insert(normalized.to_string());
    }
    // Persist to DB
    let _ = crate::db::global().whitelist_add(normalized);
    write_app_log(format!("[techno_generator] Added to whitelist: {}", normalized));
}

/// Remove a directory from the whitelist
pub fn remove_from_whitelist(path: &str) -> bool {
    // Remove from in-memory cache
    let removed = if let Ok(mut whitelist) = DIRECTORY_WHITELIST.lock() {
        whitelist.remove(path)
    } else {
        false
    };
    // Remove from DB
    let _ = crate::db::global().whitelist_remove(path);
    if removed {
        write_app_log(format!("[techno_generator] Removed from whitelist: {}", path));
    }
    removed
}

/// Clear the directory whitelist (allows all directories)
pub fn clear_whitelist() {
    // Clear in-memory cache
    if let Ok(mut whitelist) = DIRECTORY_WHITELIST.lock() {
        let count = whitelist.len();
        whitelist.clear();
        write_app_log(format!("[techno_generator] Cleared directory whitelist ({} directories)", count));
    }
    // Clear from DB
    let _ = crate::db::global().whitelist_clear();
}

fn query_samples_with_key(
    label: &str,
    include_patterns: &[&str],
    require_loop: bool,
    count: usize,
    key: Option<&str>,
) -> Vec<SampleInfo> {
    // Strict key filtering - no fallback to wrong keys
    let results = query_samples_internal(label, include_patterns, require_loop, count, key, false);

    if results.is_empty() && key.is_some() {
        write_app_log(format!("[techno_generator] {}: No samples with key in filename - track will be empty", label));
    }

    results
}

/// Like `query_samples_with_key` but also accepts samples with no detected key.
/// Use for types like scatter where tonal matching is preferred but not required.
fn query_samples_with_key_optional(
    label: &str,
    include_patterns: &[&str],
    require_loop: bool,
    count: usize,
    key: Option<&str>,
) -> Vec<SampleInfo> {
    let results = query_samples_internal(label, include_patterns, require_loop, count, key, true);

    if results.is_empty() {
        write_app_log(format!("[techno_generator] {}: No samples found", label));
    }

    results
}

/// Query samples from DB with smart loop/oneshot detection.
/// 
/// - `label`: track name/number for logging (e.g. "LEAD 3")
/// - `include_patterns`: path must contain at least one of these (case-insensitive)
/// - `require_loop`: if true, filter to samples that are loops (bar-aligned duration)
/// - `count`: max samples to return
/// - `key`: optional musical key filter (parsed from filename, not DB)
fn query_samples_internal(
    label: &str,
    include_patterns: &[&str],
    require_loop: bool,
    count: usize,
    key: Option<&str>,
    key_optional: bool,
) -> Vec<SampleInfo> {
    // Use 128 BPM as reference for loop detection (typical techno tempo)
    const REFERENCE_BPM: f64 = 128.0;

    let start = std::time::Instant::now();
    write_app_log(format!("[techno_generator] {}: patterns=[{}] key={:?} require_loop={}", label, include_patterns.join(","), key, require_loop));

    let conn = match open_dedicated_conn() {
        Ok(c) => c,
        Err(e) => {
            write_app_log(format!("[techno_generator] query_samples_internal: DB error: {}", e));
            return vec![];
        }
    };

    // Build FTS5 MATCH clause - use OR for multiple patterns
    // FTS5 trigram tokenizer requires quotes around phrases
    let fts_match: String = include_patterns
        .iter()
        .map(|p| format!("\"{}\"", p.replace('"', "\"\"")))
        .collect::<Vec<_>>()
        .join(" OR ");

    // Get compatible keys for filename matching (if key specified)
    let compatible_keys: Vec<String> = match key {
        Some(k) => {
            let parts: Vec<&str> = k.split_whitespace().collect();
            if parts.len() == 2 {
                let root = parts[0];
                let quality = parts[1];
                crate::als_project::get_compatible_keys(
                    root,
                    if quality.eq_ignore_ascii_case("minor") { "Aeolian" } else { "Ionian" }
                )
            } else {
                vec![k.to_string()]
            }
        }
        None => vec![],
    };

    // Use FTS5 for fast substring search via trigram index
    // Query FTS first, then join - FTS rowid = audio_samples.id
    let query = format!(
        "SELECT s.path, COALESCE(s.duration, 0), s.bpm, COALESCE(s.size, 0),
                COALESCE(s.sample_rate, 44100), COALESCE(s.channels, 2), COALESCE(s.bits_per_sample, 16)
         FROM audio_samples_fts fts
         JOIN audio_samples s ON s.id = fts.rowid
         WHERE s.format = 'WAV'
         AND fts.path MATCH '{}'",
        fts_match
    );

    let mut stmt = match conn.prepare(&query) {
        Ok(s) => s,
        Err(e) => {
            write_app_log(format!("[techno_generator] query_samples_internal: prepare error: {}", e));
            return vec![];
        }
    };

    let all_samples: Vec<SampleInfo> = stmt.query_map([], |row| {
        let path: String = row.get(0)?;
        let duration: f64 = row.get(1)?;
        let bpm: Option<f64> = row.get(2)?;
        let size: u64 = row.get::<_, i64>(3).map(|v| v as u64)?;
        let sample_rate: u32 = row.get(4)?;
        let channels: u16 = row.get(5)?;
        let bits_per_sample: u16 = row.get(6)?;
        Ok((path, duration, bpm, size, sample_rate, channels, bits_per_sample))
    })
    .ok()
    .map(|rows| {
        rows.filter_map(|r| r.ok())
            .map(|(path, duration, bpm, size, sample_rate, channels, bits_per_sample)| {
                SampleInfo::from_db(&path, duration, size, bpm, sample_rate, channels, bits_per_sample)
            })
            .collect()
    })
    .unwrap_or_default();

    // Filter out:
    // 1. Samples that don't actually contain any include_pattern (FTS5 can over-match)
    // 2. Reversed samples (files ending with -R.wav, _R.wav, etc.)
    // 3. Ableton project samples (frozen, consolidated, rendered from sessions)
    // 4. Bad genres (checked on directory path only, not filename)
    // 5. Construction kits/stems (not loopable, meant to be mixed not looped)
    // 6. Samples longer than 32 bars (too long for loop-based arrangement)
    use crate::sample_filters::{REVERSED_SUFFIXES, PROJECT_RENDER_KEYWORDS, CONSTRUCTION_KIT_KEYWORDS, BAD_GENRES, is_ableton_project_sample};
    
    // Max duration: 32 bars at the reference BPM (32 bars * 4 beats/bar * 60s/min / BPM)
    let max_duration_secs = (32.0 * 4.0 * 60.0) / REFERENCE_BPM;
    
    let all_samples: Vec<SampleInfo> = all_samples
        .into_iter()
        .filter(|s| {
            let path_lower = s.path.to_lowercase();
            
            // CRITICAL: Validate that at least one include_pattern actually appears in the path
            // FTS5 trigram can over-match, so we verify the pattern is really there
            let has_pattern = include_patterns.iter().any(|p| path_lower.contains(&p.to_lowercase()));
            if !has_pattern {
                return false;
            }
            
            // Skip reversed files
            if REVERSED_SUFFIXES.iter().any(|suffix| s.path.ends_with(suffix)) {
                return false;
            }
            // Skip frozen/consolidated/rendered files
            if PROJECT_RENDER_KEYWORDS.iter().any(|kw| path_lower.contains(kw)) {
                return false;
            }
            // Skip construction kits and stems (not loopable)
            if CONSTRUCTION_KIT_KEYWORDS.iter().any(|kw| path_lower.contains(kw)) {
                return false;
            }
            // Skip samples inside Ableton project directories
            if is_ableton_project_sample(&s.path) {
                return false;
            }
            // Skip samples longer than 32 bars (too long for loop-based arrangement)
            if s.duration_secs > max_duration_secs {
                return false;
            }
            // Skip bad genres - check directory path only (exclude filename)
            if let Some(last_slash) = s.path.rfind('/').or_else(|| s.path.rfind('\\')) {
                let dir_path = s.path[..last_slash].to_lowercase();
                if BAD_GENRES.iter().any(|genre| dir_path.contains(genre)) {
                    return false;
                }
            }
            true
        })
        .collect();

    // Filter by loop if required FIRST (cheaper than key extraction)
    let loop_filtered: Vec<SampleInfo> = if require_loop {
        all_samples
            .into_iter()
            .filter(|s| s.is_loop(REFERENCE_BPM))
            .collect()
    } else {
        all_samples
    };

    // Filter by key from filename (if key specified) - AFTER loop filter to reduce count
    let key_filtered: Vec<SampleInfo> = if !compatible_keys.is_empty() {
        loop_filtered
            .into_iter()
            .filter(|s| {
                // Extract key from filename/path
                if let Some(parsed_key) = crate::sample_analysis::extract_key(&s.path) {
                    // Check if parsed key matches any compatible key
                    compatible_keys.iter().any(|ck| ck.eq_ignore_ascii_case(&parsed_key))
                } else {
                    // No key in filename: include if key_optional, skip otherwise
                    key_optional
                }
            })
            .collect()
    } else {
        loop_filtered
    };

    // Score and sort by hardness preference
    let hardness = get_hardness();
    let mut scored: Vec<(SampleInfo, f32)> = key_filtered
        .into_iter()
        .map(|s| {
            let path_lower = s.path.to_lowercase();
            
            // Count hard pattern matches
            let hard_matches = HARD_PATTERNS.iter()
                .filter(|p| path_lower.contains(*p))
                .count() as f32;
            
            // Count soft pattern matches  
            let soft_matches = SOFT_PATTERNS.iter()
                .filter(|p| path_lower.contains(*p))
                .count() as f32;
            
            // Score: positive = hard, negative = soft
            // hardness 0.0 -> prefer soft (score * -1)
            // hardness 0.5 -> neutral (score * 0)
            // hardness 1.0 -> prefer hard (score * 1)
            let raw_score = hard_matches - soft_matches;
            let preference = (hardness - 0.5) * 2.0; // -1 to +1
            let final_score = raw_score * preference;
            
            (s, final_score)
        })
        .collect();
    
    // Shuffle first to randomize samples with similar scores, then stable sort by score.
    // Pull from the seeded generation RNG so two runs with the same seed pick the
    // same samples from each score bucket.
    use rand::seq::SliceRandom;
    with_gen_rng(|rng| scored.shuffle(rng));

    // Stable sort by score (higher = better match for current hardness)
    scored.sort_by(|a, b| {
        b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal)
    });
    
    // Filter out already-used samples and non-whitelisted paths, then take count
    let mut results: Vec<SampleInfo> = Vec::with_capacity(count);
    for (sample, _score) in scored {
        // Skip if not in whitelisted directories (if whitelist is set)
        if !is_path_whitelisted(&sample.path) {
            continue;
        }
        if !is_sample_used(&sample.path) {
            mark_sample_used(&sample.path);
            results.push(sample);
            if results.len() >= count {
                break;
            }
        }
    }

    let sample_names: Vec<&str> = results.iter().map(|s| s.name.as_str()).collect();
    write_app_log(format!("[techno_generator] {}: found {} in {:?}: {:?}", label, results.len(), start.elapsed(), sample_names));
    results
}

// Section locators for arrangement navigation (for one song). Built from the
// user's section layout so locators land on the actual section boundaries
// Ableton sees, not the canonical 32/32/32/32/32/32/32 grid.
fn get_song_locators(starts: &crate::als_project::SectionStarts) -> Vec<(&'static str, u32)> {
    vec![
        ("INTRO",     starts.intro.0),
        ("BUILD",     starts.build.0),
        ("BREAKDOWN", starts.breakdown.0),
        ("DROP 1",    starts.drop1.0),
        ("DROP 2",    starts.drop2.0),
        ("FADEDOWN",  starts.fadedown.0),
        ("OUTRO",     starts.outro.0),
    ]
}

fn create_locators_xml_multi(
    ids: &IdAllocator,
    num_songs: u32,
    song_keys: &[String],
    user_starts: &crate::als_project::SectionStarts,
) -> String {
    let mut locators: Vec<String> = Vec::new();
    let bars_per_song = user_starts.total_bars() + GAP_BETWEEN_SONGS;

    for song_idx in 0..num_songs {
        let offset = song_idx * bars_per_song;
        let key = song_keys.get(song_idx as usize).map(|s| s.as_str()).unwrap_or("?");

        // Add song start marker with key (only if multiple songs)
        if num_songs > 1 {
            let song_start_id = ids.alloc();
            let song_start_beat = offset * 4;
            locators.push(format!(
                r#"<Locator Id="{}">
					<LomId Value="0" />
					<Time Value="{}" />
					<Name Value="=== SONG {} ({}) ===" />
					<Annotation Value="" />
					<IsSongStart Value="false" />
				</Locator>"#,
                song_start_id, song_start_beat, song_idx + 1, key
            ));
        }

        // Add section markers for this song
        for (name, bar) in get_song_locators(user_starts) {
            let id = ids.alloc();
            let time_beats = (bar - 1 + offset) * 4; // bar 1 = beat 0
            // Only prefix with song number if multiple songs
            let label = if num_songs > 1 {
                format!("{} {}", song_idx + 1, name)
            } else {
                name.to_string()
            };
            locators.push(format!(
                r#"<Locator Id="{}">
					<LomId Value="0" />
					<Time Value="{}" />
					<Name Value="{}" />
					<Annotation Value="" />
					<IsSongStart Value="false" />
				</Locator>"#,
                id, time_beats, label
            ));
        }
    }

    // Output just the inner Locators content (locators wrapped in <Locators>)
    // The template has outer <Locators> wrapper with inner <Locators /> placeholder
    format!(
        "<Locators>\n\t\t\t\t{}\n\t\t\t</Locators>",
        locators.join("\n\t\t\t\t")
    )
}

fn load_song_samples(song_num: u32, target_key: Option<&str>, atonal: bool, hardness: f32, track_counts: &TrackCounts, type_atonal: &TypeAtonal, on_progress: Option<&dyn Fn(&str)>) -> SongSamples {
    let start = std::time::Instant::now();
    // Set thread-local hardness for query functions
    set_hardness(hardness);
    // NOTE: Don't clear used samples here - cleared once in generate() so songs don't reuse samples
    write_app_log(format!("[techno_generator] load_song_samples: song {} starting, target_key={:?}, atonal={}", song_num, target_key, atonal));
    // When global atonal is set, all types are atonal
    let track_key = target_key.map(|k| k.to_string()).unwrap_or_else(pick_random_key);
    let key_filter: Option<&str> = if atonal { None } else { Some(&track_key) };
    write_app_log(format!("[techno_generator] load_song_samples: song {} using key={}, key_filter={:?} (took {:?})", song_num, track_key, key_filter, start.elapsed()));
    
    let progress = |msg: &str| { if let Some(cb) = on_progress { cb(msg); } };
    
    // Helper: get key filter for a type - None if global atonal OR type-specific atonal
    let key_for = |type_is_atonal: bool| -> Option<&str> {
        if atonal || type_is_atonal { None } else { Some(&track_key) }
    };

    // Calculate total samples to search for progress tracking
    let total_samples: u32 = track_counts.kick + track_counts.clap + track_counts.snare + track_counts.hat
        + track_counts.perc + track_counts.ride + track_counts.fill
        + track_counts.bass + track_counts.sub
        + track_counts.lead + track_counts.synth + track_counts.pad + track_counts.arp
        + track_counts.riser + track_counts.downlifter + track_counts.crash + track_counts.impact
        + track_counts.hit + track_counts.sweep_up + track_counts.sweep_down + track_counts.snare_roll
        + track_counts.reverse + track_counts.sub_drop + track_counts.boom_kick + track_counts.atmos + track_counts.glitch + track_counts.scatter
        + track_counts.vox;
    let mut samples_searched: u32 = 0;

    // Helper to query N samples with optional key filtering
    // Use macro to avoid closure borrow issues with mutable counter
    macro_rules! query_n_keyed {
        ($label:expr, $inc:expr, $require_loop:expr, $count:expr, $key:expr) => {{
            let mut results = Vec::new();
            for i in 0..$count as usize {
                samples_searched += 1;
                progress(&format!("SAMPLE_PROGRESS:{}:{}", samples_searched, total_samples));
                let track_label = format!("{} {}", $label, i + 1);
                progress(&format!("Searching {}...", track_label));
                let samples = query_samples_with_key(&track_label, $inc, $require_loop, 1, $key);
                if !samples.is_empty() {
                    progress(&format!("Found {}: {}", track_label, samples[0].name));
                }
                results.push(samples);
            }
            results
        }};
    }

    // === DRUMS (typically no key, but respect per-type atonal toggle) ===
    let kick_inc = &["kick_loop", "kick loop", "drum_loops/kick", "drum loops/kick"];
    let kicks = query_n_keyed!("KICK", kick_inc, true, track_counts.kick, key_for(type_atonal.kick));

    let clap_inc = &["clap_loop", "clap loop", "claps/", "clap/"];
    let claps = query_n_keyed!("CLAP", clap_inc, true, track_counts.clap, key_for(type_atonal.clap));

    // Exclude snare_roll/snare_build - those go to SNARE_ROLL
    let snare_inc = &["snare_loop", "snare loop", "snares/", "snare/"];
    let snares = query_n_keyed!("SNARE", snare_inc, true, track_counts.snare, key_for(type_atonal.snare));

    let hat_inc = &["hat_loop", "hihat_loop", "closed_hat", "open_hat", "hats/", "/hats", "hihat/"];
    let hats = query_n_keyed!("HAT", hat_inc, true, track_counts.hat, key_for(type_atonal.hat));

    // top_loop removed - too ambiguous, could be hats or full drum tops
    let perc_inc = &["perc_loop", "percussion_loop", "percussion_&_top", "perc loop", "percussion loop", 
                     "shaker", "tom_loop", "conga", "bongo", "perc/", "percussion/"];
    let percs = query_n_keyed!("PERC", perc_inc, true, track_counts.perc, key_for(type_atonal.perc));

    let ride_inc = &["ride_loop", "ride loop", "cymbal_loop", "cymbal loop", "cymbals/"];
    let rides = query_n_keyed!("RIDE", ride_inc, true, track_counts.ride, key_for(type_atonal.ride));

    // "fill" alone is too broad (matches "filter"), use more specific patterns
    let fill_inc = &["drum_fill", "drum fill", "fills/", "fill/", "drum_break", "breaks/"];
    let fills = query_n_keyed!("FILL", fill_inc, false, track_counts.fill, key_for(type_atonal.fill));

    // === BASS (key matched unless atonal) ===
    let bass_inc = &["bass_loop", "bass loop", "bass_loops/", "bassline", "basslines/", "reeses_and_hoovers"];
    let basses = query_n_keyed!("BASS", bass_inc, true, track_counts.bass, key_for(type_atonal.bass));

    let sub_inc = &["sub_loop", "sub loop", "sub_bass", "808_loop", "808 loop"];
    let subs = query_n_keyed!("SUB", sub_inc, true, track_counts.sub, key_for(type_atonal.sub));

    // === MELODICS (key matched unless atonal) ===
    // Query pads FIRST so synth_pad/pad_synth samples go to pads, not synths
    let pad_inc = &["pad_loop", "pad loop", "pad_loops/", "pad/", "pads/", "drone_loop", "atmosphere_loop",
                    "synth_pad", "synth pad", "pad_synth", "pad synth"];
    let pads = query_n_keyed!("PAD", pad_inc, true, track_counts.pad, key_for(type_atonal.pad));

    let lead_inc = &["lead_loop", "lead loop", "synth_lead", "lead/"];
    let leads = query_n_keyed!("LEAD", lead_inc, true, track_counts.lead, key_for(type_atonal.lead));

    // Exclude pad/drone patterns - those go to PAD category
    let synth_inc = &["synth_loop", "synth loop", "synth_loops/", "music_loops/", "melody_loop", "acid_loop"];
    let synths = query_n_keyed!("SYNTH", synth_inc, true, track_counts.synth, key_for(type_atonal.synth));

    // pluck_loop stays but pluck/ alone is too broad (one-shots), piano/ might not be arps
    let arp_inc = &["arp_loop", "arp loop", "arpegg", "arpeggio", "arp/", "arps/", "pluck_loop", "sequence_loop"];
    let arps = query_n_keyed!("ARP", arp_inc, true, track_counts.arp, key_for(type_atonal.arp));

    let keys_inc = &["keys", "keys/", "piano", "piano/", "piano_loop", "keyboard", "keyboard/", 
                     "electric_piano", "rhodes", "wurlitzer", "organ_loop", "organ/"];
    let keyss = query_n_keyed!("KEYS", keys_inc, true, track_counts.keys, key_for(type_atonal.keys));

    // === FX (mixed - some tonal, some not) ===
    // "build" alone could match "buildup" for snare rolls - use more specific
    let riser_inc = &["riser", "risers___lifters", "uplifter", "riser/", "risers/", "tension", "build_up", "build_fx"];
    let risers = query_n_keyed!("RISER", riser_inc, false, track_counts.riser, key_for(type_atonal.riser));

    let downlifter_inc = &["downlifter", "falls___descenders", "fall", "descend"];
    let downlifters = query_n_keyed!("DOWNLIFTER", downlifter_inc, false, track_counts.downlifter, key_for(type_atonal.downlifter));

    let crash_inc = &["crash", "cymbal_crash", "crash___cymbals", "cymbal_hit"];
    let crashes = query_n_keyed!("CRASH", crash_inc, false, track_counts.crash, key_for(type_atonal.crash));

    // Remove "impacts___bombs" - also in BOOM_KICK. Remove "boom" - too broad
    let impact_inc = &["impact", "impacts/", "thud", "slam", "low_impact"];
    let impacts = query_n_keyed!("IMPACT", impact_inc, false, track_counts.impact, key_for(type_atonal.impact));

    let hit_inc = &["orchestral_hits", "fx_hit", "perc_shot", "rave_hit", "stab_hit"];
    let hits = query_n_keyed!("HIT", hit_inc, false, track_counts.hit, key_for(type_atonal.hit));

    let sweep_up_inc = &["sweep_up", "sweep up", "up_sweep", "up sweep", "upsweep", "noise sweep up", "noise_sweep_up"];
    let sweep_ups = query_n_keyed!("SWEEP UP", sweep_up_inc, false, track_counts.sweep_up, key_for(type_atonal.sweep_up));
    
    let sweep_down_inc = &["sweep_down", "sweep down", "down_sweep", "down sweep", "downsweep", "noise sweep down", "noise_sweep_down"];
    let sweep_downs = query_n_keyed!("SWEEP DOWN", sweep_down_inc, false, track_counts.sweep_down, key_for(type_atonal.sweep_down));

    let snare_roll_inc = &["snare_roll", "snare roll", "snare_build", "snare build", "buildup"];
    let snare_rolls = query_n_keyed!("SNARE_ROLL", snare_roll_inc, false, track_counts.snare_roll, key_for(type_atonal.snare_roll));

    let reverse_inc = &["reverse", "reverse_fx", "rev_cymbal", "rev_crash", "reversed"];
    let reverses = query_n_keyed!("REVERSE", reverse_inc, false, track_counts.reverse, key_for(type_atonal.reverse));

    let sub_drop_inc = &["sub drop", "sub_drop", "subboom", "sub_boom", "808_hit", "low_impact", "sine_sub"];
    let sub_drops = query_n_keyed!("SUB_DROP", sub_drop_inc, false, track_counts.sub_drop, key_for(type_atonal.sub_drop));

    let boom_kick_inc = &["kick fx", "kick_fx", "impact fx", "impact_fx", "boom kick", "boom_kick", "reverb kick", "reverb_kick", "impacts___bombs"];
    let boom_kicks = query_n_keyed!("BOOM_KICK", boom_kick_inc, false, track_counts.boom_kick, key_for(type_atonal.boom_kick));

    let atmos_inc = &["atmos", "atmosphere", "atmospheres/", "ambient", "texture", "drone", "soundscape",
                      "foley", "foley/", "synth_drone", "synth drone"];
    let atmoses = query_n_keyed!("ATMOS", atmos_inc, false, track_counts.atmos, key_for(type_atonal.atmos));

    let glitch_inc = &["glitch", "glitches/", "glitch_fx", "glitch fx", "stutter_fx", "stutter fx", "stutters/", "glitch_loop", "glitch loop"];
    let glitches = query_n_keyed!("GLITCH", glitch_inc, false, track_counts.glitch, key_for(type_atonal.glitch));

    // Scatter hits - short one-shots for random placement (any one-shots: perc, fx, melodic stabs)
    let scatter_inc = &[
        "one shot", "one_shot", "oneshot", "one-shot", "shots/",
        "stab", "stabs/", "hit", "hits/", "blip", "zap", "pluck",
        "perc shot", "perc_shot", "fx shot", "fx_shot", "fx hit", "fx_hit",
        "click", "tick", "snap", "pop", "chirp", "ping", "spike",
        "impact", "transient", "accent", "chop", "cut"
    ];
    let scatters = {
        let mut results = Vec::new();
        let scatter_key = key_for(type_atonal.scatter);
        for i in 0..track_counts.scatter as usize {
            samples_searched += 1;
            progress(&format!("SAMPLE_PROGRESS:{}:{}", samples_searched, total_samples));
            let track_label = format!("SCATTER {}", i + 1);
            progress(&format!("Searching {}...", track_label));
            let samples = query_samples_with_key_optional(&track_label, scatter_inc, false, 1, scatter_key);
            if !samples.is_empty() {
                progress(&format!("Found {}: {}", track_label, samples[0].name));
            }
            results.push(samples);
        }
        results
    };

    // === VOCALS ===
    let vox_inc = &["vox", "vocal", "voice", "vocals/", "vocal_cut", "vocal cut", "vocal_loop", "choir", "chant",
                    "vox_fx", "vox fx", "vocal_fx", "vocal fx"];
    let voxes = query_n_keyed!("VOX", vox_inc, false, track_counts.vox, key_for(type_atonal.vox));

    // Log non-empty counts for debugging
    let count_nonempty = |v: &[Vec<SampleInfo>]| v.iter().filter(|x| !x.is_empty()).count();
    write_app_log(format!(
        "[techno_generator] load_song_samples: song {} completed in {:?} - non-empty: bass={}/{} lead={}/{} pad={}/{}",
        song_num, start.elapsed(),
        count_nonempty(&basses), basses.len(),
        count_nonempty(&leads), leads.len(),
        count_nonempty(&pads), pads.len()
    ));

    SongSamples {
        key: track_key,
        kicks, claps, snares, hats, percs, rides, fills,
        basses, subs,
        leads, synths, pads, arps, keyss,
        risers, downlifters, crashes, impacts, hits, sweep_ups, sweep_downs, snare_rolls, reverses, sub_drops, boom_kicks, atmoses, glitches, scatters,
        voxes,
    }
}

pub struct GenerationResult {
    pub tracks: usize,
    pub clips: usize,
    pub bars: u32,
    pub warnings: Vec<String>,
    pub keys: Vec<String>,
}

/// Track counts from wizard UI - one slider per sample type
#[derive(Debug, Clone)]
pub struct TrackCounts {
    // Drums
    pub kick: u32,
    pub clap: u32,
    pub snare: u32,
    pub hat: u32,
    pub perc: u32,
    pub ride: u32,
    pub fill: u32,
    // Bass
    pub bass: u32,
    pub sub: u32,
    // Melodics
    pub lead: u32,
    pub synth: u32,
    pub pad: u32,
    pub arp: u32,
    pub keys: u32,
    // FX
    pub riser: u32,
    pub downlifter: u32,
    pub crash: u32,
    pub impact: u32,
    pub hit: u32,
    pub sweep_up: u32,
    pub sweep_down: u32,
    pub snare_roll: u32,
    pub reverse: u32,
    pub sub_drop: u32,
    pub boom_kick: u32,
    pub atmos: u32,
    pub glitch: u32,
    pub scatter: u32,
    // Vocals
    pub vox: u32,
}

impl Default for TrackCounts {
    fn default() -> Self {
        Self {
            kick: 1, clap: 1, snare: 1, hat: 2, perc: 2, ride: 1, fill: 4,
            bass: 1, sub: 1,
            lead: 1, synth: 3, pad: 2, arp: 2, keys: 2,
            riser: 3, downlifter: 1, crash: 2, impact: 2, hit: 2, sweep_up: 4, sweep_down: 4, snare_roll: 1, reverse: 2, sub_drop: 2, boom_kick: 2, atmos: 2, glitch: 2, scatter: 4,
            vox: 1,
        }
    }
}

/// Per-8-bar-block overrides for one dynamics parameter. Shared with the
/// IPC-facing type in `als_project` — both sides use the same `BTreeMap<String, f32>`
/// backing so no field-by-field mapping is needed at the IPC boundary.
///
/// Keys: the starting bar of an 8-bar block (1, 9, 17, …). Values: 0.0–1.0.
/// Missing keys fall back to the global scalar.
pub type SectionValues = crate::als_project::SectionValues;

/// Per-block overrides for all 6 dynamics params.
pub type SectionOverrides = crate::als_project::SectionOverridesConfig;

/// Legacy type alias for backwards compatibility
pub type SectionGlitch = SectionValues;

/// Per-type atonal flags - when true, skip key filtering for that sample type
#[derive(Debug, Clone, Default)]
pub struct TypeAtonal {
    // Drums (typically atonal by default)
    pub kick: bool,
    pub clap: bool,
    pub snare: bool,
    pub hat: bool,
    pub perc: bool,
    pub ride: bool,
    pub fill: bool,
    // Bass (tonal)
    pub bass: bool,
    pub sub: bool,
    // Melodics (tonal)
    pub lead: bool,
    pub synth: bool,
    pub pad: bool,
    pub arp: bool,
    pub keys: bool,
    // FX (mixed - some tonal like risers/sweeps, some atonal like crashes/hits)
    pub riser: bool,
    pub downlifter: bool,
    pub crash: bool,
    pub impact: bool,
    pub hit: bool,
    pub sweep_up: bool,
    pub sweep_down: bool,
    pub snare_roll: bool,
    pub reverse: bool,
    pub sub_drop: bool,
    pub boom_kick: bool,
    pub atmos: bool,
    pub glitch: bool,
    pub scatter: bool,
    // Vocals (can be either)
    pub vox: bool,
}

pub fn generate(
    output_path: &Path,
    bpm: f64,
    num_songs: u32,
    root_note: Option<&str>,
    mode: Option<&str>,
    genre: Option<&str>,
    hardness: f32,
    chaos: f32,
    glitch_intensity: f32,
    section_overrides: SectionOverrides,
    density: f32,
    variation: f32,
    parallelism: f32,
    scatter: f32,
    atonal: bool,
    track_counts: TrackCounts,
    type_atonal: TypeAtonal,
    section_lengths: crate::als_project::SectionLengths,
    // Generation seed — every random decision derives from this. Caller
    // (usually the Tauri command) must pass a concrete u64; `None` at the
    // wizard layer gets resolved to a fresh `rand::random()` before this fn
    // is called, so we can always echo the seed back in the result for
    // "regenerate with same seed".
    seed: u64,
    cancel: Option<&std::sync::atomic::AtomicBool>,
    on_progress: Option<&dyn Fn(&str)>,
) -> Result<GenerationResult, String> {
    // Seed the thread-local generation RNG up front — every helper below
    // reads from it via `with_gen_rng`. Wrap the rest of the body in a
    // closure so we can guarantee `clear_gen_rng()` runs on every exit path
    // (including the many `?` / `return Err` spots), without sprinkling
    // manual cleanup everywhere.
    init_gen_rng(seed);
    let r = generate_inner(
        output_path, bpm, num_songs, root_note, mode, genre, hardness, chaos,
        glitch_intensity, section_overrides, density, variation, parallelism,
        scatter, atonal, track_counts, type_atonal, section_lengths, cancel,
        on_progress,
    );
    clear_gen_rng();
    r
}

#[allow(clippy::too_many_arguments)]
fn generate_inner(
    output_path: &Path,
    bpm: f64,
    num_songs: u32,
    root_note: Option<&str>,
    mode: Option<&str>,
    genre: Option<&str>,
    hardness: f32,
    chaos: f32,
    glitch_intensity: f32,
    section_overrides: SectionOverrides,
    density: f32,
    variation: f32,
    parallelism: f32,
    scatter: f32,
    atonal: bool,
    track_counts: TrackCounts,
    type_atonal: TypeAtonal,
    section_lengths: crate::als_project::SectionLengths,
    cancel: Option<&std::sync::atomic::AtomicBool>,
    on_progress: Option<&dyn Fn(&str)>,
) -> Result<GenerationResult, String> {
    let gen_start = std::time::Instant::now();
    // Sanitize before use — if the frontend (or a stale IPC payload) ships
    // non-multiple-of-8 or sub-8 values, clamp them up to the grid so we
    // never produce a song with a 3-bar intro.
    let lengths = section_lengths.sanitize();
    let user_starts = lengths.starts();
    let song_length_bars = lengths.total_bars();
    write_app_log(format!(
        "[techno_generator] generate: INPUT PARAMS: output={:?}, bpm={}, num_songs={}, root_note={:?}, mode={:?}, genre={:?}, hardness={}, chaos={}, glitch_intensity={}, density={}, variation={}, parallelism={}, scatter={}, atonal={}, tracks={:?}, type_atonal={:?}, lengths={:?} ({} bars)",
        output_path, bpm, num_songs, root_note, mode, genre, hardness, chaos, glitch_intensity, density, variation, parallelism, scatter, atonal, track_counts, type_atonal, lengths, song_length_bars
    ));

    let cancelled = || cancel.is_some_and(|c| c.load(std::sync::atomic::Ordering::Relaxed));
    let progress = |msg: &str| { if let Some(cb) = on_progress { cb(msg); } };

    let ids = IdAllocator::new(1000000);
    let bars_per_song = song_length_bars + GAP_BETWEEN_SONGS;
    let total_bars = bars_per_song * num_songs;
    write_app_log(format!(
        "[techno_generator] generate: COMPUTED: bars_per_song={}, total_bars={}, song_length={} bars, gap={} bars",
        bars_per_song, total_bars, song_length_bars, GAP_BETWEEN_SONGS
    ));

    // Build target key from root_note + mode, or pick random
    let target_key = match (root_note, mode) {
        (Some(root), Some(m)) => {
            // Convert mode to minor/major for sample matching
            let suffix = match m.to_lowercase().as_str() {
                "aeolian" | "minor" | "dorian" | "phrygian" | "locrian" => "Minor",
                "ionian" | "major" | "lydian" | "mixolydian" => "Major",
                _ => "Minor", // default to minor for techno
            };
            Some(format!("{} {}", root, suffix))
        }
        _ => None,
    };
    write_app_log(format!("[techno_generator] generate: target_key={:?}", target_key));

    // Load samples for each song
    // Clear used samples once at the start so songs don't reuse samples
    clear_used_samples();
    write_app_log("[techno_generator] generate: starting sample loading loop".into());
    let mut all_songs: Vec<SongSamples> = Vec::new();
    for song_num in 1..=num_songs {
        if cancelled() {
            write_app_log("[techno_generator] generate: cancelled".into());
            return Err("Generation cancelled".into());
        }
        progress(&format!("Loading samples for song {}/{}...", song_num, num_songs));
        write_app_log(format!("[techno_generator] generate: calling load_song_samples({}) with hardness={}, track_counts={:?}", song_num, hardness, track_counts));
        let song_samples = load_song_samples(song_num, target_key.as_deref(), atonal, hardness, &track_counts, &type_atonal, on_progress);
        all_songs.push(song_samples);
        write_app_log(format!("[techno_generator] generate: load_song_samples({}) done", song_num));
    }
    write_app_log(format!("[techno_generator] generate: sample loading complete, elapsed {:?}", gen_start.elapsed()));

    // Collect keys for locators
    let song_keys: Vec<String> = all_songs.iter().map(|s| s.key.clone()).collect();

    // For track definitions, we use samples from first song to determine if track should be created
    let song1 = &all_songs[0];

    if cancelled() { return Err("Generation cancelled".into()); }
    progress("Generating base ALS template");
    generate_empty_als(output_path)?;

    let file = File::open(output_path).map_err(|e| e.to_string())?;
    let mut decoder = GzDecoder::new(file);
    let mut xml = String::new();
    decoder.read_to_string(&mut xml).map_err(|e| e.to_string())?;

    // Reserve template IDs
    let id_re = Regex::new(r#"Id="(\d+)""#).unwrap();
    for cap in id_re.captures_iter(&xml) {
        if let Ok(id) = cap[1].parse::<u32>() {
            ids.reserve(id);
        }
    }

    // Extract audio track template
    let track_start = xml.find("<AudioTrack").ok_or("No AudioTrack found")?;
    let track_end = xml.find("</AudioTrack>").ok_or("No AudioTrack end found")? + "</AudioTrack>".len();
    let original_audio_track = xml[track_start..track_end].to_string();

    // Allocate group IDs. KICKS is its own group (sibling of DRUMS) so the
    // kick pulse lives outside anything that will be sidechained to it — this
    // leaves the door open for a group-level sidechain on DRUMS/BASS without
    // the kick ducking itself.
    let kicks_group_id = ids.alloc();
    let drums_group_id = ids.alloc();
    let bass_group_id = ids.alloc();
    let bass_fx_group_id = ids.alloc();
    let melodics_group_id = ids.alloc();
    let fx_group_id = ids.alloc();
    let scatter_group_id = ids.alloc();

    // Create groups
    let kicks_group = create_group_track("KICKS", DRUMS_COLOR, kicks_group_id, &ids)?;
    let drums_group = create_group_track("DRUMS", DRUMS_COLOR, drums_group_id, &ids)?;
    let bass_group = create_group_track("BASS", BASS_COLOR, bass_group_id, &ids)?;
    let bass_fx_group = create_group_track("BASS FX", BASS_COLOR, bass_fx_group_id, &ids)?;
    let melodics_group = create_group_track("MELODICS", MELODICS_COLOR, melodics_group_id, &ids)?;
    let fx_group = create_group_track("FX", FX_COLOR, fx_group_id, &ids)?;
    let scatter_group = create_group_track("SCATTER", FX_COLOR, scatter_group_id, &ids)?;

    // Device injection (sidechain compressors, master EQ/limiter) is disabled
    // because Ableton's ALS parser crashes on programmatically injected device
    // XML — the user can add devices manually in Live.

    // Get arrangement structure with all section overrides applied. The
    // templates are in the canonical 224-bar layout; we remap onto the user's
    // `lengths` immediately after so everything downstream (clip placement,
    // find_arr fallback, full_arrangement) speaks in user bars.
    let mut arrangements = get_arrangement_with_params(chaos, glitch_intensity, &section_overrides, density, variation, parallelism, scatter, track_counts.scatter);
    for arr in arrangements.iter_mut() {
        let mut remapped: Vec<(f64, f64)> = Vec::with_capacity(arr.sections.len());
        for &(s, e) in arr.sections.iter() {
            if let Some(r) = remap_bar_range(s, e, &user_starts) {
                remapped.push(r);
            }
        }
        arr.sections = remapped;
    }

    // Default full-song arrangement for extra loop tracks — spans every
    // section of the user's layout, not the canonical one, so tracks using
    // this fallback fill the song regardless of section length customization.
    // Section bounds are (start_bar, end_bar_exclusive), e.g. intro=(1,33) for
    // 32 bars. Clips use the same exclusive end convention.
    let full_arrangement: Vec<(f64, f64)> = vec![
        (user_starts.intro.0     as f64, user_starts.intro.1     as f64),
        (user_starts.build.0     as f64, user_starts.build.1     as f64),
        (user_starts.breakdown.0 as f64, user_starts.breakdown.1 as f64),
        (user_starts.drop1.0     as f64, user_starts.drop1.1     as f64),
        (user_starts.drop2.0     as f64, user_starts.drop2.1     as f64),
        (user_starts.fadedown.0  as f64, user_starts.fadedown.1  as f64),
        (user_starts.outro.0     as f64, user_starts.outro.1     as f64),
    ];

    // Helper to find arrangement for a track
    // Supports dynamic 1-N for ALL track types
    let find_arr = |name: &str| -> Vec<(f64, f64)> {
        // First try exact match in predefined arrangements
        if let Some(arr) = arrangements.iter().find(|a| a.name == name) {
            return arr.sections.clone();
        }
        
        // All track types that support dynamic layering (1-N)
        // Maps prefix -> base arrangement name (the "1" version)
        let layer_patterns = [
            // Drums
            ("KICK ", "KICK"),
            ("CLAP ", "CLAP"),
            ("SNARE ", "SNARE"),
            ("HAT ", "HAT"),
            ("PERC ", "PERC"),
            ("RIDE ", "RIDE"),
            ("FILL ", "FILL 1"),
            // Bass
            ("BASS ", "BASS 1"),
            ("SUB ", "SUB 1"),
            // Bass FX
            ("SUB DROP ", "SUB DROP 1"),
            ("BOOM KICK ", "BOOM KICK 1"),
            // Melodics
            ("LEAD ", "LEAD 1"),
            ("SYNTH ", "SYNTH 1"),
            ("PAD ", "PAD 1"),
            ("ARP ", "ARP 1"),
            ("KEYS ", "KEYS 1"),
            ("ATMOS ", "ATMOS"),
            // FX
            ("RISER ", "RISER 1"),
            ("DOWNLIFTER ", "DOWNLIFTER 1"),
            ("CRASH ", "CRASH"),
            ("IMPACT ", "IMPACT"),
            ("HIT ", "HIT"),
            ("SNARE ROLL ", "SNARE ROLL 1"),
            ("REVERSE ", "REVERSE 1"),
            ("GLITCH ", "GLITCH 1"),
            ("SCATTER ", "SCATTER 1"),
            // Vocals
            ("VOX ", "VOX 1"),
        ];
        
        for (prefix, base_name) in layer_patterns {
            if let Some(num_str) = name.strip_prefix(prefix)
                && let Ok(layer_num) = num_str.parse::<usize>() {
                    // Get base arrangement
                    if let Some(base_arr) = arrangements.iter().find(|a| a.name == base_name) {
                        let base_sections = &base_arr.sections;
                        if base_sections.is_empty() {
                            return vec![];
                        }
                        
                        // Layer 1 = full arrangement
                        if layer_num == 1 {
                            return base_sections.clone();
                        }
                        
                        // Higher layers = trim from start and end (gradual build/breakdown)
                        // Layer 2 = trim 1 from each end, Layer 3 = trim 2, etc.
                        let trim = layer_num - 1;
                        let total = base_sections.len();
                        
                        // Need at least 1 section remaining
                        if trim * 2 >= total {
                            // Too many layers, just use middle section(s)
                            let mid = total / 2;
                            return vec![base_sections[mid]];
                        }
                        
                        // Trim from start (later entry) and end (earlier exit)
                        return base_sections[trim..total - trim].to_vec();
                    }
                }
        }
        
        // For dynamic tracks (DRUM LOOP N, BASS LOOP N, etc.), use full arrangement
        if name.starts_with("DRUM LOOP ") || name.starts_with("BASS LOOP ") 
            || name.starts_with("SYNTH LOOP ") || name.starts_with("PAD LOOP ") {
            return full_arrangement.clone();
        }
        vec![]
    };

    if cancelled() { return Err("Generation cancelled".into()); }

    // Count total tracks to create (for progress bar).
    // Always show ALL requested tracks even if empty - user wants to see what they asked for.
    // Groups are emitted when they have any requested children (not just non-empty samples).
    let kicks_group_emitted = !song1.kicks.is_empty();
    let drums_group_emitted = !song1.claps.is_empty() || !song1.snares.is_empty()
        || !song1.hats.is_empty() || !song1.percs.is_empty() || !song1.rides.is_empty()
        || !song1.fills.is_empty();
    let bass_group_emitted = !song1.basses.is_empty() || !song1.subs.is_empty();
    let bass_fx_group_emitted = !song1.sub_drops.is_empty() || !song1.boom_kicks.is_empty();
    let melodics_group_emitted = !song1.leads.is_empty() || !song1.synths.is_empty()
        || !song1.pads.is_empty() || !song1.arps.is_empty() || !song1.atmoses.is_empty();
    let fx_group_emitted = !song1.risers.is_empty() || !song1.downlifters.is_empty()
        || !song1.crashes.is_empty() || !song1.impacts.is_empty() || !song1.hits.is_empty()
        || !song1.sweep_ups.is_empty() || !song1.sweep_downs.is_empty()
        || !song1.snare_rolls.is_empty() || !song1.reverses.is_empty()
        || !song1.glitches.is_empty() || !song1.voxes.is_empty();
    let scatter_group_emitted = !song1.scatters.is_empty();
    let group_count = [
        kicks_group_emitted,
        drums_group_emitted,
        bass_group_emitted,
        bass_fx_group_emitted,
        melodics_group_emitted,
        fx_group_emitted,
        scatter_group_emitted,
    ].iter().filter(|b| **b).count();
    // Count ALL tracks (not just non-empty) since we show all requested tracks
    let total_tracks = group_count
        + song1.kicks.len()
        + song1.claps.len()
        + song1.snares.len()
        + song1.hats.len()
        + song1.percs.len()
        + song1.rides.len()
        + song1.fills.len()
        + song1.basses.len()
        + song1.subs.len()
        + song1.leads.len()
        + song1.synths.len()
        + song1.pads.len()
        + song1.arps.len()
        + song1.risers.len()
        + song1.downlifters.len()
        + song1.crashes.len()
        + song1.impacts.len()
        + song1.hits.len()
        + song1.sweep_ups.len()
        + song1.sweep_downs.len()
        + song1.snare_rolls.len()
        + song1.reverses.len()
        + song1.sub_drops.len()
        + song1.boom_kicks.len()
        + song1.atmoses.len()
        + song1.glitches.len()
        + song1.scatters.len()
        + song1.voxes.len();

    let mut tracks_created = 0usize;
    let report_progress = |created: usize, total: usize| {
        if let Some(cb) = on_progress {
            cb(&format!("TRACK_PROGRESS:{}:{}", created, total));
        }
    };

    report_progress(0, total_tracks);

    let mut warnings: Vec<String> = Vec::new();
    let mut all_tracks: Vec<String> = Vec::new();

    // Helper to check if any tracks were requested (regardless of whether samples were found)
    let has_requested = |samples: &[Vec<SampleInfo>]| -> bool {
        !samples.is_empty()
    };

    // Macro to reduce repetition in track creation
    // Always use numbered names (e.g., "HAT 1", "HAT 2") for consistent arrangement lookup
    // Returns the number of tracks created
    // IMPORTANT: Always creates tracks even if samples are empty (user wants to see all requested tracks)
    macro_rules! create_tracks {
        ($samples:expr, $base_name:expr, $color:expr, $group_id:expr) => {{
            let mut created = 0usize;
            for i in 0..$samples.len() {
                let name = format!("{} {}", $base_name, i + 1);
                // Always create the track even if empty - user requested this many tracks
                match create_arranged_track_multi(&original_audio_track, &name, $color, $group_id, &all_songs, &find_arr(&name), &ids, bpm, bars_per_song) {
                    Ok(track) => { all_tracks.push(track); created += 1; },
                    Err(e) => warnings.push(format!("{}: {}", name, e)),
                }
                tracks_created += 1;
                report_progress(tracks_created, total_tracks);
            }
            created
        }};
    }

    // === KICKS === (own group so its bus output can drive sidechain inputs
    // on every other group without the kick itself ducking to its own pulse).
    let kicks_has_children = has_requested(&song1.kicks);
    if kicks_has_children {
        all_tracks.push(kicks_group.clone());
        tracks_created += 1;
        report_progress(tracks_created, total_tracks);
    }
    create_tracks!(song1.kicks, "KICK", DRUMS_COLOR, if kicks_has_children { kicks_group_id as i32 } else { -1 });

    // === DRUMS === (non-kick drums: claps/snares/hats/percs/rides/fills).
    let drums_has_children = has_requested(&song1.claps) || has_requested(&song1.snares) ||
                             has_requested(&song1.hats) || has_requested(&song1.percs) || has_requested(&song1.rides) ||
                             has_requested(&song1.fills);
    if drums_has_children {
        all_tracks.push(drums_group.clone());
        tracks_created += 1;
        report_progress(tracks_created, total_tracks);
    }
    create_tracks!(song1.claps, "CLAP", DRUMS_COLOR, if drums_has_children { drums_group_id as i32 } else { -1 });
    create_tracks!(song1.snares, "SNARE", DRUMS_COLOR, if drums_has_children { drums_group_id as i32 } else { -1 });
    create_tracks!(song1.hats, "HAT", DRUMS_COLOR, if drums_has_children { drums_group_id as i32 } else { -1 });
    create_tracks!(song1.percs, "PERC", DRUMS_COLOR, if drums_has_children { drums_group_id as i32 } else { -1 });
    create_tracks!(song1.rides, "RIDE", DRUMS_COLOR, if drums_has_children { drums_group_id as i32 } else { -1 });
    create_tracks!(song1.fills, "FILL", DRUMS_COLOR, if drums_has_children { drums_group_id as i32 } else { -1 });

    // === BASS === (only add group if it will have children)
    let bass_has_children = has_requested(&song1.basses) || has_requested(&song1.subs);
    if bass_has_children {
        all_tracks.push(bass_group.clone());
        tracks_created += 1;
        report_progress(tracks_created, total_tracks);
    }
    create_tracks!(song1.basses, "BASS", BASS_COLOR, if bass_has_children { bass_group_id as i32 } else { -1 });
    create_tracks!(song1.subs, "SUB", BASS_COLOR, if bass_has_children { bass_group_id as i32 } else { -1 });

    // === BASS FX === (only add group if it will have children)
    let bass_fx_has_children = has_requested(&song1.sub_drops) || has_requested(&song1.boom_kicks);
    if bass_fx_has_children {
        all_tracks.push(bass_fx_group.clone());
        tracks_created += 1;
        report_progress(tracks_created, total_tracks);
    }
    create_tracks!(song1.sub_drops, "SUB DROP", BASS_COLOR, if bass_fx_has_children { bass_fx_group_id as i32 } else { -1 });
    create_tracks!(song1.boom_kicks, "BOOM KICK", BASS_COLOR, if bass_fx_has_children { bass_fx_group_id as i32 } else { -1 });

    // === MELODICS === (only add group if it will have children)
    let melodics_has_children = has_requested(&song1.leads) || has_requested(&song1.synths) || has_requested(&song1.pads) ||
                                has_requested(&song1.arps) || has_requested(&song1.keyss) || has_requested(&song1.atmoses);
    if melodics_has_children {
        all_tracks.push(melodics_group.clone());
        tracks_created += 1;
        report_progress(tracks_created, total_tracks);
    }
    create_tracks!(song1.leads, "LEAD", MELODICS_COLOR, if melodics_has_children { melodics_group_id as i32 } else { -1 });
    create_tracks!(song1.synths, "SYNTH", MELODICS_COLOR, if melodics_has_children { melodics_group_id as i32 } else { -1 });
    create_tracks!(song1.pads, "PAD", MELODICS_COLOR, if melodics_has_children { melodics_group_id as i32 } else { -1 });
    create_tracks!(song1.arps, "ARP", MELODICS_COLOR, if melodics_has_children { melodics_group_id as i32 } else { -1 });
    create_tracks!(song1.keyss, "KEYS", MELODICS_COLOR, if melodics_has_children { melodics_group_id as i32 } else { -1 });
    create_tracks!(song1.atmoses, "ATMOS", MELODICS_COLOR, if melodics_has_children { melodics_group_id as i32 } else { -1 });

    // === FX === (only add group if it will have children)
    let fx_has_children = has_requested(&song1.risers) || has_requested(&song1.downlifters) || has_requested(&song1.crashes) ||
                          has_requested(&song1.impacts) || has_requested(&song1.hits) || has_requested(&song1.sweep_ups) ||
                          has_requested(&song1.sweep_downs) || has_requested(&song1.snare_rolls) || has_requested(&song1.reverses) ||
                          has_requested(&song1.glitches) || has_requested(&song1.voxes);
    if fx_has_children {
        all_tracks.push(fx_group.clone());
        tracks_created += 1;
        report_progress(tracks_created, total_tracks);
    }
    create_tracks!(song1.risers, "RISER", FX_COLOR, if fx_has_children { fx_group_id as i32 } else { -1 });
    create_tracks!(song1.downlifters, "DOWNLIFTER", FX_COLOR, if fx_has_children { fx_group_id as i32 } else { -1 });
    create_tracks!(song1.crashes, "CRASH", FX_COLOR, if fx_has_children { fx_group_id as i32 } else { -1 });
    create_tracks!(song1.impacts, "IMPACT", FX_COLOR, if fx_has_children { fx_group_id as i32 } else { -1 });
    create_tracks!(song1.hits, "HIT", FX_COLOR, if fx_has_children { fx_group_id as i32 } else { -1 });
    create_tracks!(song1.sweep_ups, "SWEEP UP", FX_COLOR, if fx_has_children { fx_group_id as i32 } else { -1 });
    create_tracks!(song1.sweep_downs, "SWEEP DOWN", FX_COLOR, if fx_has_children { fx_group_id as i32 } else { -1 });
    create_tracks!(song1.snare_rolls, "SNARE ROLL", FX_COLOR, if fx_has_children { fx_group_id as i32 } else { -1 });
    create_tracks!(song1.reverses, "REVERSE", FX_COLOR, if fx_has_children { fx_group_id as i32 } else { -1 });
    create_tracks!(song1.glitches, "GLITCH", FX_COLOR, if fx_has_children { fx_group_id as i32 } else { -1 });

    // === VOCALS === (part of FX group)
    create_tracks!(song1.voxes, "VOX", FX_COLOR, if fx_has_children { fx_group_id as i32 } else { -1 });

    // === SCATTER === (own group for random one-shot hits)
    let scatter_has_children = has_requested(&song1.scatters);
    if scatter_has_children {
        all_tracks.push(scatter_group.clone());
        tracks_created += 1;
        report_progress(tracks_created, total_tracks);
    }
    create_tracks!(song1.scatters, "SCATTER", FX_COLOR, if scatter_has_children { scatter_group_id as i32 } else { -1 });

    // Log warnings
    for w in &warnings {
        write_app_log(format!("[techno_generator] WARNING: {}", w));
    }

    // Sidechain ducking is now applied at the GROUP BUS level (DRUMS / BASS /
    // BASS FX / MELODICS), keyed to the KICKS group bus. Per-track sidechain
    // on every audio track was redundant once we moved to group compressors
    // (stacking would double-compress). Groups are installed above during
    // group-track creation.

    progress("Assembling XML");
    // Build final XML - all tracks
    let before_track = &xml[..track_start];
    let after_track = &xml[track_end..];

    let track_count = all_tracks.len();
    let clip_count: usize = all_tracks.iter().map(|t| t.matches("<AudioClip").count()).sum();

    // Fail if no tracks were created (no samples found)
    if track_count == 0 {
        return Err("No samples found for any track type. Run sample analysis first or check your sample library paths.".into());
    }

    let all_tracks_xml = all_tracks.join("\n\t\t\t");

    let mut xml = format!("{}{}{}", before_track, all_tracks_xml, after_track);

    // Master chain device injection (Eq8 HPF + Limiter) is disabled — Ableton
    // crashes on programmatically injected device XML in the MainTrack.

    // Update NextPointeeId
    let next_id = ids.max_id() + 1000;
    let next_id_re = Regex::new(r#"<NextPointeeId Value="\d+" />"#).unwrap();
    xml = next_id_re.replace(&xml, format!(r#"<NextPointeeId Value="{}" />"#, next_id)).to_string();

    // Hide mixer
    xml = xml.replace(
        r#"<MixerInArrangement Value="1" />"#,
        r#"<MixerInArrangement Value="0" />"#,
    );

    // Add locators at section boundaries for ALL songs
    // Template has outer wrapper: <Locators>\n\t\t\t<Locators />\n\t\t</Locators>
    // We replace the inner <Locators /> with our populated <Locators>...</Locators>
    let locators_xml = create_locators_xml_multi(&ids, num_songs, &song_keys, &user_starts);
    let inner_locators_re = Regex::new(r#"<Locators\s*/>"#).unwrap();
    if inner_locators_re.is_match(&xml) {
        xml = inner_locators_re.replace(&xml, locators_xml.as_str()).to_string();
        write_app_log(format!("[techno_generator] Inserted {} locators", locators_xml.matches("<Locator ").count()));
    } else {
        write_app_log("[techno_generator] WARNING: Could not find inner <Locators /> placeholder in XML".into());
    }

    // Set tempo to specified BPM
    let bpm_str = format!("{}", bpm);
    let tempo_re = Regex::new(r#"<Tempo>\s*<LomId Value="0" />\s*<Manual Value="[^"]+" />"#).unwrap();
    xml = tempo_re.replace(&xml, format!(r#"<Tempo>
						<LomId Value="0" />
						<Manual Value="{}" />"#, bpm_str)).to_string();

    let tempo_event_re = Regex::new(r#"<FloatEvent Id="\d+" Time="-63072000" Value="[^"]+" />"#).unwrap();
    xml = tempo_event_re.replace(&xml, format!(r#"<FloatEvent Id="0" Time="-63072000" Value="{}" />"#, bpm_str)).to_string();

    let output_name = output_path.file_name().and_then(|n| n.to_str()).unwrap_or("project.als");
    progress(&format!("Writing {}", output_name));
    write_app_log(format!("[techno_generator] Writing output: {:?}", output_path));
    let output_file = File::create(output_path).map_err(|e| e.to_string())?;
    let mut encoder = GzEncoder::new(output_file, Compression::default());
    encoder.write_all(xml.as_bytes()).map_err(|e| e.to_string())?;
    encoder.finish().map_err(|e| e.to_string())?;
    write_app_log(format!("[techno_generator] Completed: {:?} ({} tracks, {} clips)", output_path, track_count, clip_count));

    Ok(GenerationResult {
        tracks: track_count,
        clips: clip_count,
        bars: bars_per_song * num_songs,
        warnings,
        keys: song_keys,
    })
}

fn create_audio_clip(sample: &SampleInfo, color: u32, clip_id: u32, start_bar: f64, end_bar: f64, bpm: f64) -> String {
    let beats_per_bar = 4.0;
    // Both bars are 1-indexed, so subtract 1 before converting to beats
    // Bar 1 = beat 0, bar 16 = beat 60, bar 16.25 = beat 61
    let start_beat = (start_bar - 1.0) * beats_per_bar;
    let end_beat = (end_bar - 1.0) * beats_per_bar;

    // Loop length is the sample's natural length - this doesn't change
    // Clip length (end_beat - start_beat) can be shorter if cut at a boundary
    let loop_bars = sample.loop_bars(bpm);
    let loop_beats = loop_bars as f64 * beats_per_bar;
    
    // DEBUG: Log loop calculation details
    write_app_log(format!(
        "[create_audio_clip] {} | duration={:.2}s | sample_bpm={:?} | project_bpm={} | loop_bars={} | loop_beats={}",
        sample.name, sample.duration_secs, sample.bpm, bpm, loop_bars, loop_beats
    ));

    // WarpMarker tells Ableton: "at SecTime seconds into the sample, we should be at BeatTime beats"
    // SecTime = actual duration of audio in the file
    // BeatTime = where that audio should align in the project timeline
    // Ableton uses these two points to calculate the stretch ratio
    //
    // The critical insight: we should use the ACTUAL sample duration for the warp marker,
    // not a derived value based on quantized bars and estimated BPM. This ensures:
    // 1. The loop point matches the actual audio content
    // 2. Warping works correctly even when BPM metadata is missing or wrong
    //
    // However, we cap it to the loop_beats duration at sample BPM (if known) to avoid
    // including silence or partial content beyond the loop boundary.
    let sample_bpm = sample.bpm.filter(|&b| b > 0.0);
    let warp_sec = if let Some(sbpm) = sample_bpm {
        // Known BPM: calculate exact duration for the loop length
        (loop_beats * 60.0) / sbpm
    } else {
        // Unknown BPM: use actual sample duration, capped to reasonable length
        // This preserves the original audio timing without artificial stretching
        let max_sec = (loop_beats * 60.0) / bpm.max(1.0);
        sample.duration_secs.min(max_sec).max(0.1)
    };

    format!(r#"<AudioClip Id="{clip_id}" Time="{start_beat}">
										<LomId Value="0" />
										<LomIdView Value="0" />
										<CurrentStart Value="{start_beat}" />
										<CurrentEnd Value="{end_beat}" />
										<Loop>
											<LoopStart Value="0" />
											<LoopEnd Value="{loop_beats}" />
											<StartRelative Value="0" />
											<LoopOn Value="true" />
											<OutMarker Value="{loop_beats}" />
											<HiddenLoopStart Value="0" />
											<HiddenLoopEnd Value="{loop_beats}" />
										</Loop>
										<Name Value="{name}" />
										<Annotation Value="" />
										<Color Value="{color}" />
										<LaunchMode Value="0" />
										<LaunchQuantisation Value="0" />
										<TimeSignature>
											<TimeSignatures>
												<RemoteableTimeSignature Id="0">
													<Numerator Value="4" />
													<Denominator Value="4" />
													<Time Value="0" />
												</RemoteableTimeSignature>
											</TimeSignatures>
										</TimeSignature>
										<Envelopes>
											<Envelopes />
										</Envelopes>
										<ScrollerTimePreserver>
											<LeftTime Value="0" />
											<RightTime Value="{end_beat}" />
										</ScrollerTimePreserver>
										<TimeSelection>
											<AnchorTime Value="0" />
											<OtherTime Value="0" />
										</TimeSelection>
										<Legato Value="false" />
										<Ram Value="false" />
										<GrooveSettings>
											<GrooveId Value="-1" />
										</GrooveSettings>
										<Disabled Value="false" />
										<VelocityAmount Value="0" />
										<FollowAction>
											<FollowTime Value="4" />
											<IsLinked Value="true" />
											<LoopIterations Value="1" />
											<FollowActionA Value="4" />
											<FollowActionB Value="0" />
											<FollowChanceA Value="100" />
											<FollowChanceB Value="0" />
											<JumpIndexA Value="1" />
											<JumpIndexB Value="1" />
											<FollowActionEnabled Value="false" />
										</FollowAction>
										<Grid>
											<FixedNumerator Value="1" />
											<FixedDenominator Value="16" />
											<GridIntervalPixel Value="20" />
											<Ntoles Value="2" />
											<SnapToGrid Value="true" />
											<Fixed Value="false" />
										</Grid>
										<FreezeStart Value="0" />
										<FreezeEnd Value="0" />
										<IsWarped Value="true" />
										<TakeId Value="1" />
										<SampleRef>
											<FileRef>
												<RelativePathType Value="0" />
												<RelativePath Value="" />
												<Path Value="{path}" />
												<Type Value="2" />
												<LivePackName Value="" />
												<LivePackId Value="" />
												<OriginalFileSize Value="{file_size}" />
												<OriginalCrc Value="0" />
											</FileRef>
											<LastModDate Value="0" />
											<SourceContext>
												<SourceContext Id="0">
													<OriginalFileRef>
														<FileRef Id="0">
															<RelativePathType Value="0" />
															<RelativePath Value="" />
															<Path Value="{path}" />
															<Type Value="2" />
															<LivePackName Value="" />
															<LivePackId Value="" />
															<OriginalFileSize Value="{file_size}" />
															<OriginalCrc Value="0" />
														</FileRef>
													</OriginalFileRef>
													<BrowserContentPath Value="" />
													<LocalFiltersJson Value="" />
												</SourceContext>
											</SourceContext>
											<SampleUsageHint Value="0" />
											<DefaultDuration Value="{loop_beats}" />
											<DefaultSampleRate Value="44100" />
										</SampleRef>
										<Onsets>
											<UserOnsets />
											<HasUserOnsets Value="false" />
										</Onsets>
										<WarpMode Value="0" />
										<GranularityTones Value="30" />
										<GranularityTexture Value="65" />
										<FluctuationTexture Value="25" />
										<TransientResolution Value="6" />
										<TransientLoopMode Value="2" />
										<TransientEnvelope Value="100" />
										<ComplexProFormants Value="100" />
										<ComplexProEnvelope Value="128" />
										<Sync Value="true" />
										<HiQ Value="true" />
										<Fade Value="true" />
										<Fades>
											<FadeInLength Value="0" />
											<FadeOutLength Value="0" />
											<ClipFadesAreInitialized Value="true" />
											<CrossfadeLength Value="0" />
											<FadeInCurveSkew Value="0" />
											<FadeInCurveSlope Value="0" />
											<FadeOutCurveSkew Value="0" />
											<FadeOutCurveSlope Value="0" />
											<IsDefaultFadeIn Value="true" />
											<IsDefaultFadeOut Value="true" />
										</Fades>
										<PitchCoarse Value="0" />
										<PitchFine Value="0" />
										<SampleVolume Value="1" />
										<MarkerDensity Value="2" />
										<AutoWarpTolerance Value="4" />
										<WarpMarkers>
											<WarpMarker Id="0" SecTime="0" BeatTime="0" />
											<WarpMarker Id="1" SecTime="{warp_sec}" BeatTime="{loop_beats}" />
										</WarpMarkers>
										<SavedWarpMarkersForStretched />
										<MarkersGenerated Value="true" />
										<IsSongTempoLeader Value="false" />
									</AudioClip>"#,
        clip_id = clip_id,
        start_beat = start_beat,
        end_beat = end_beat,
        loop_beats = loop_beats,
        name = sample.xml_name(),
        color = color,
        path = sample.xml_path(),
        file_size = sample.file_size,
        warp_sec = warp_sec
    )
}

/// Replace every `Id="N"` occurrence in a device XML template with freshly
/// allocated, unique IDs from the supplied allocator. This lets us clone a
/// device into a project without ID collisions.
fn reallocate_device_ids(template: &str, ids: &IdAllocator) -> String {
    let id_re = Regex::new(r#"Id="(\d+)""#).unwrap();
    let mut out = template.to_string();
    let mut replacements: Vec<(String, String)> = Vec::new();
    for cap in id_re.captures_iter(&out) {
        let old = format!(r#"Id="{}""#, &cap[1]);
        let new_id = ids.alloc();
        let new = format!(r#"Id="{}""#, new_id);
        replacements.push((old, new));
    }
    for (old, new) in replacements {
        out = out.replacen(&old, &new, 1);
    }
    out
}

/// Return (reverb_send_ratio, delay_send_ratio) as linear voltage ratios for a
/// track name. Ratio = 10^(dB/20). -∞ dB is encoded as the base template's
/// default `0.000316...` which reads as "silenced" in Ableton's UI.
///
/// Levels target a typical techno mix: bass stays dry, drums get a small kiss
/// of reverb, pads/atmos run wettest, leads add delay for rhythmic interest.
fn send_levels_for(name: &str) -> (f64, f64) {
    const OFF: f64 = 0.000316; // -∞ dB (same as template default)
    const DB_24: f64 = 0.06309573; // -24 dB
    const DB_18: f64 = 0.12589254; // -18 dB
    const DB_15: f64 = 0.17782794; // -15 dB
    const DB_12: f64 = 0.25118864; // -12 dB
    const DB_9: f64 = 0.35481339; // -9 dB

    let n = name.to_uppercase();
    let n = n.as_str();
    // Low-end stays dry — reverb/delay on sub frequencies muddies the mix.
    if n.starts_with("KICK")
        || n.starts_with("BOOM KICK")
        || n.starts_with("SUB DROP")
        || n.starts_with("SUB")
        || n.starts_with("BASS")
    {
        (OFF, OFF)
    } else if n.starts_with("CLAP") || n.starts_with("SNARE") {
        (DB_18, OFF)
    } else if n.starts_with("HAT") || n.starts_with("RIDE") || n.starts_with("PERC") || n.starts_with("FILL") {
        (DB_24, OFF)
    } else if n.starts_with("LEAD") || n.starts_with("ARP") {
        (DB_15, DB_12)
    } else if n.starts_with("SYNTH") {
        (DB_15, DB_18)
    } else if n.starts_with("PAD") || n.starts_with("ATMOS") {
        (DB_9, DB_18)
    } else if n.starts_with("VOX") {
        (DB_12, DB_15)
    } else if n.starts_with("RISER")
        || n.starts_with("DOWNLIFTER")
        || n.starts_with("SWEEP")
        || n.starts_with("CRASH")
        || n.starts_with("IMPACT")
        || n.starts_with("HIT")
        || n.starts_with("REVERSE")
    {
        (DB_12, DB_18)
    } else if n.starts_with("GLITCH") || n.starts_with("SCATTER") {
        (DB_15, DB_15)
    } else {
        (DB_24, OFF)
    }
}

/// Replace the two default `Manual Value="0.00031622..."` placeholders inside
/// the track's `<Sends>` block with the supplied linear ratios. Each value is
/// unique within one AudioTrack template so `replacen(.., 1)` targets the right
/// Send. Send A routes to Return A (Reverb) and Send B to Return B (Delay).
fn apply_sends(track: &str, reverb: f64, delay: f64) -> String {
    track
        .replacen(
            r#"<Manual Value="0.00031622799" />"#,
            &format!(r#"<Manual Value="{}" />"#, reverb),
            1,
        )
        .replacen(
            r#"<Manual Value="0.0003162277571" />"#,
            &format!(r#"<Manual Value="{}" />"#, delay),
            1,
        )
}

/// Build a sidechain Compressor2 device keyed to `source_track_id`, with fresh
/// unique IDs so multiple group buses can each own a copy. The template has
/// SideChain pre-enabled and a `__SC_SRC_ID__` placeholder which we substitute
/// for the real source track id.
fn build_group_sidechain_compressor(source_track_id: u32, ids: &IdAllocator) -> String {
    let device = reallocate_device_ids(GROUP_SIDECHAIN_COMPRESSOR_TEMPLATE, ids);
    device.replace("__SC_SRC_ID__", &source_track_id.to_string())
}

/// Inject a pre-built device into a group track's `<Devices />` block. The
/// base group template ships with an empty self-closing `<Devices />` — we
/// swap that for `<Devices>{device}</Devices>` on the first match so only
/// the group's own chain is populated (not nested freeze-sequencer slots).
fn inject_device_into_group_chain(group_xml: &str, device_xml: &str) -> String {
    // Re-indent the device body so it nests under the group's DeviceChain
    // tabs. The group_track_template places `<Devices />` at 6 tabs deep;
    // child devices should be at 7 tabs.
    let indent = "\t\t\t\t\t\t\t";
    let reindented: String = device_xml
        .lines()
        .map(|l| if l.is_empty() { l.to_string() } else { format!("{}{}", indent, l) })
        .collect::<Vec<_>>()
        .join("\n");
    let replacement = format!("<Devices>\n{}\n\t\t\t\t\t\t</Devices>", reindented);
    group_xml.replacen("<Devices />", &replacement, 1)
}

/// Inject `<Eq8>` (30 Hz high-pass) and `<Limiter>` (-0.3 dB ceiling) into the
/// MainTrack's device chain, in front of the existing `<StereoGain>` (Utility).
/// Order matters: HPF first to remove rumble, then Limiter to catch peaks.
fn inject_master_chain(xml: &str, ids: &IdAllocator) -> String {
    let eq8 = reallocate_device_ids(MASTER_EQ8_HPF_TEMPLATE, ids);
    let limiter = reallocate_device_ids(MASTER_LIMITER_TEMPLATE, ids);

    // Scope to the MainTrack element so we don't hit AudioTrack/ReturnTrack
    // Devices blocks. Real ALS templates open with `<MainTrack Selected...>`
    // but tests use `<MainTrack>` — accept both.
    let main_re = Regex::new(r"<MainTrack(?:\s|>)").unwrap();
    let Some(m) = main_re.find(xml) else { return xml.to_string() };
    let main_start = m.start();
    let Some(rel_main_end) = xml[main_start..].find("</MainTrack>") else { return xml.to_string() };
    let main_end = main_start + rel_main_end;

    let main = &xml[main_start..main_end];
    // First `<Devices>` with actual children (not `<Devices />`).
    let devices_open = "<Devices>";
    let Some(rel_dev) = main.find(devices_open) else { return xml.to_string() };
    let insert_pos = main_start + rel_dev + devices_open.len();

    // Existing first child indent is 7 tabs (matches <StereoGain Id=...>).
    // Our templates are at zero indent — we prefix their first line, and re-
    // indent subsequent lines with awk-style tab prepending done on the fly.
    let indent = "\t\t\t\t\t\t\t";
    let reindent = |body: &str| -> String {
        body.lines()
            .map(|l| if l.is_empty() { l.to_string() } else { format!("{}{}", indent, l) })
            .collect::<Vec<_>>()
            .join("\n")
    };

    let injected = format!(
        "\n{}\n{}",
        reindent(eq8.trim_end()),
        reindent(limiter.trim_end()),
    );

    format!("{}{}{}", &xml[..insert_pos], injected, &xml[insert_pos..])
}

fn create_group_track(name: &str, color: u32, group_id: u32, ids: &IdAllocator) -> Result<String, String> {
    let mut track = GROUP_TRACK_TEMPLATE.to_string();

    let id_re = Regex::new(r#"Id="(\d+)""#).unwrap();
    let mut replacements: Vec<(String, String)> = Vec::new();

    for cap in id_re.captures_iter(&track) {
        let old = format!(r#"Id="{}""#, &cap[1]);
        let new_id = ids.alloc();
        let new = format!(r#"Id="{}""#, new_id);
        replacements.push((old, new));
    }

    for (old, new) in replacements {
        track = track.replacen(&old, &new, 1);
    }

    let track_id_re = Regex::new(r#"<GroupTrack Id="\d+""#).unwrap();
    track = track_id_re.replace(&track, format!(r#"<GroupTrack Id="{}""#, group_id)).to_string();

    track = track.replace(
        r#"<EffectiveName Value="Drums" />"#,
        &format!(r#"<EffectiveName Value="{}" />"#, name),
    );
    track = track.replace(
        r#"<UserName Value="Drums" />"#,
        &format!(r#"<UserName Value="{}" />"#, name),
    );

    let color_re = Regex::new(r#"<Color Value="\d+" />"#).unwrap();
    track = color_re.replace_all(&track, format!(r#"<Color Value="{}" />"#, color)).to_string();

    Ok(track)
}

// Helper to get samples for a track from a SongSamples
// Parses track names like "KICK", "KICK 2", "SYNTH 3" etc. and returns samples from the appropriate vec
fn get_track_samples(song: &SongSamples, track_name: &str) -> Vec<SampleInfo> {
    // Parse track name to get type and optional index
    // "KICK" -> (kicks, 0), "KICK 2" -> (kicks, 1), etc.
    let parse_idx = |name: &str, prefix: &str| -> Option<usize> {
        if name == prefix {
            Some(0)
        } else if name.starts_with(prefix) && name.len() > prefix.len() {
            let suffix = name[prefix.len()..].trim();
            suffix.parse::<usize>().ok().map(|n| n.saturating_sub(1))
        } else {
            None
        }
    };

    // Try each type
    if let Some(idx) = parse_idx(track_name, "KICK") {
        return song.kicks.get(idx).cloned().unwrap_or_default();
    }
    if let Some(idx) = parse_idx(track_name, "CLAP") {
        return song.claps.get(idx).cloned().unwrap_or_default();
    }
    if let Some(idx) = parse_idx(track_name, "SNARE ROLL") {
        return song.snare_rolls.get(idx).cloned().unwrap_or_default();
    }
    if let Some(idx) = parse_idx(track_name, "SNARE") {
        return song.snares.get(idx).cloned().unwrap_or_default();
    }
    if let Some(idx) = parse_idx(track_name, "HAT") {
        return song.hats.get(idx).cloned().unwrap_or_default();
    }
    if let Some(idx) = parse_idx(track_name, "PERC") {
        return song.percs.get(idx).cloned().unwrap_or_default();
    }
    if let Some(idx) = parse_idx(track_name, "RIDE") {
        return song.rides.get(idx).cloned().unwrap_or_default();
    }
    if let Some(idx) = parse_idx(track_name, "FILL") {
        return song.fills.get(idx).cloned().unwrap_or_default();
    }
    if let Some(idx) = parse_idx(track_name, "SUB DROP") {
        return song.sub_drops.get(idx).cloned().unwrap_or_default();
    }
    if let Some(idx) = parse_idx(track_name, "SUB") {
        return song.subs.get(idx).cloned().unwrap_or_default();
    }
    if let Some(idx) = parse_idx(track_name, "BASS") {
        return song.basses.get(idx).cloned().unwrap_or_default();
    }
    if let Some(idx) = parse_idx(track_name, "LEAD") {
        return song.leads.get(idx).cloned().unwrap_or_default();
    }
    if let Some(idx) = parse_idx(track_name, "SYNTH") {
        return song.synths.get(idx).cloned().unwrap_or_default();
    }
    if let Some(idx) = parse_idx(track_name, "PAD") {
        return song.pads.get(idx).cloned().unwrap_or_default();
    }
    if let Some(idx) = parse_idx(track_name, "ARP") {
        return song.arps.get(idx).cloned().unwrap_or_default();
    }
    if let Some(idx) = parse_idx(track_name, "KEYS") {
        return song.keyss.get(idx).cloned().unwrap_or_default();
    }
    if let Some(idx) = parse_idx(track_name, "RISER") {
        return song.risers.get(idx).cloned().unwrap_or_default();
    }
    if let Some(idx) = parse_idx(track_name, "DOWNLIFTER") {
        return song.downlifters.get(idx).cloned().unwrap_or_default();
    }
    if let Some(idx) = parse_idx(track_name, "CRASH") {
        return song.crashes.get(idx).cloned().unwrap_or_default();
    }
    if let Some(idx) = parse_idx(track_name, "IMPACT") {
        return song.impacts.get(idx).cloned().unwrap_or_default();
    }
    if let Some(idx) = parse_idx(track_name, "BOOM KICK") {
        return song.boom_kicks.get(idx).cloned().unwrap_or_default();
    }
    if let Some(idx) = parse_idx(track_name, "HIT") {
        return song.hits.get(idx).cloned().unwrap_or_default();
    }
    if let Some(idx) = parse_idx(track_name, "SWEEP UP") {
        return song.sweep_ups.get(idx).cloned().unwrap_or_default();
    }
    if let Some(idx) = parse_idx(track_name, "SWEEP DOWN") {
        return song.sweep_downs.get(idx).cloned().unwrap_or_default();
    }
    if let Some(idx) = parse_idx(track_name, "REVERSE") {
        return song.reverses.get(idx).cloned().unwrap_or_default();
    }
    if let Some(idx) = parse_idx(track_name, "ATMOS") {
        return song.atmoses.get(idx).cloned().unwrap_or_default();
    }
    if let Some(idx) = parse_idx(track_name, "GLITCH") {
        return song.glitches.get(idx).cloned().unwrap_or_default();
    }
    if let Some(idx) = parse_idx(track_name, "SCATTER") {
        return song.scatters.get(idx).cloned().unwrap_or_default();
    }
    if let Some(idx) = parse_idx(track_name, "VOX") {
        return song.voxes.get(idx).cloned().unwrap_or_default();
    }
    
    vec![]
}

// Create a track with clips for multiple songs, each using different samples
fn create_arranged_track_multi(
    template: &str,
    name: &str,
    color: u32,
    group_id: i32,
    all_songs: &[SongSamples],
    sections: &[(f64, f64)],
    ids: &IdAllocator,
    bpm: f64,
    bars_per_song: u32,
) -> Result<String, String> {
    let mut track = template.to_string();

    // Replace all IDs
    let id_re = Regex::new(r#"Id="(\d+)""#).unwrap();
    let mut replacements: Vec<(String, String)> = Vec::new();

    for cap in id_re.captures_iter(&track) {
        let old = format!(r#"Id="{}""#, &cap[1]);
        let new_id = ids.alloc();
        let new = format!(r#"Id="{}""#, new_id);
        replacements.push((old, new));
    }

    for (old, new) in replacements {
        track = track.replacen(&old, &new, 1);
    }

    // Set name
    let name_re = Regex::new(r#"<EffectiveName Value="[^"]*" />"#).unwrap();
    track = name_re.replace(&track, format!(r#"<EffectiveName Value="{}" />"#, name)).to_string();

    let username_re = Regex::new(r#"(<EffectiveName Value="[^"]*" />\s*<UserName Value=")[^"]*(" />)"#).unwrap();
    track = username_re.replace(&track, format!(r#"${{1}}{}${{2}}"#, name)).to_string();

    // Set color
    let color_re = Regex::new(r#"<Color Value="\d+" />"#).unwrap();
    track = color_re.replace_all(&track, format!(r#"<Color Value="{}" />"#, color)).to_string();

    // Set group
    track = track.replacen(
        r#"<TrackGroupId Value="-1" />"#,
        &format!(r#"<TrackGroupId Value="{}" />"#, group_id),
        1,
    );

    // Route to group if in a group
    if group_id != -1 {
        track = track.replacen(
            r#"<Target Value="AudioOut/Main" />"#,
            r#"<Target Value="AudioOut/GroupTrack" />"#,
            1,
        );
        track = track.replacen(
            r#"<UpperDisplayString Value="Master" />"#,
            r#"<UpperDisplayString Value="Group" />"#,
            1,
        );
    }

    // Set volume to -12dB (except KICK which is 0dB)
    let volume_re = Regex::new(r#"(<Volume>\s*<LomId Value="0" />\s*<Manual Value=")[^"]+(" />)"#).unwrap();
    let volume_value = if name.starts_with("KICK") { "1" } else { "0.251188643" };
    track = volume_re.replace(&track, format!(r#"${{1}}{}${{2}}"#, volume_value)).to_string();

    // Create clips for each song, offset by song index * bars_per_song
    let mut clips: Vec<String> = Vec::new();

    for (song_idx, song) in all_songs.iter().enumerate() {
        let samples = get_track_samples(song, name);
        if samples.is_empty() {
            continue;
        }
        let sample = &samples[0];
        let offset = (song_idx as u32 * bars_per_song) as f64;
        
        // Get the sample's natural loop length in bars
        let loop_bars = sample.loop_bars(bpm) as f64;
        
        // Merge consecutive sections into continuous regions, then fill with loop-length clips
        // Example: sections [(33,35), (35,37), (37,39), (39,41)] with a gap at 41-42
        //          becomes regions [(33,41)] then fill with 4-bar clips: 33-37, 37-41
        
        // Step 1: Merge consecutive/overlapping sections into regions
        let mut regions: Vec<(f64, f64)> = Vec::new();
        for &(start, end) in sections.iter() {
            if let Some(last) = regions.last_mut() {
                // If this section is consecutive (within 0.5 bar tolerance), extend the region
                if start <= last.1 + 0.5 {
                    last.1 = last.1.max(end);
                    continue;
                }
            }
            regions.push((start, end));
        }
        
        // Step 2: Fill each region with full-length clips, last one cut at boundary
        for (region_start, region_end) in regions {
            let mut pos = region_start;
            while pos < region_end {
                let clip_end = (pos + loop_bars).min(region_end);
                let clip_id = ids.alloc();
                clips.push(create_audio_clip(sample, color, clip_id, pos + offset, clip_end + offset, bpm));
                pos += loop_bars;
            }
        }
    }

    let clips_xml = clips.join("\n");
    track = track.replacen(
        "<Events />",
        &format!("<Events>\n{}\n\t\t\t\t\t\t\t\t\t\t\t\t\t</Events>", clips_xml),
        1,
    );

    // Set reverb/delay send amounts based on track category. The template's
    // defaults are -∞ dB (silent); this makes the return busses actually
    // audible without any user intervention.
    let (reverb_send, delay_send) = send_levels_for(name);
    track = apply_sends(&track, reverb_send, delay_send);

    Ok(track)
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_section_values_at_bar() {
        let mut sv = SectionValues::default();
        let global = 0.3;
        
        // No overrides: always returns global
        assert_eq!(sv.value_at_bar(1, global), global);
        assert_eq!(sv.value_at_bar(100, global), global);
        assert_eq!(sv.value_at_bar(200, global), global);
        
        // Single section override (set for the whole block starting at 1)
        sv.set(1, 0.8);
        assert_eq!(sv.value_at_bar(1, global), 0.8);
        assert_eq!(sv.value_at_bar(8, global), 0.8);
        assert_eq!(sv.value_at_bar(9, global), global); // Next block
        
        // Multiple overrides (set at the start of each old 32-bar section)
        sv.set(33, 0.1);  // Build
        sv.set(65, 0.0);  // Breakdown
        sv.set(97, 1.0);  // Drop1
        sv.set(129, 0.9); // Drop2
        sv.set(161, 0.4); // Fadedown
        sv.set(193, 0.2); // Outro
        
        assert_eq!(sv.value_at_bar(1, global), 0.8);    // Intro start
        assert_eq!(sv.value_at_bar(40, global), 0.1);   // Build (bar 40 -> block 33)
        assert_eq!(sv.value_at_bar(70, global), 0.0);   // Breakdown (bar 70 -> block 65)
        assert_eq!(sv.value_at_bar(100, global), 1.0);  // Drop1 (bar 100 -> block 97)
        assert_eq!(sv.value_at_bar(130, global), 0.9);  // Drop2 (bar 130 -> block 129)
        // Let's use exact block starts for simplicity in tests
        assert_eq!(sv.value_at_bar(161, global), 0.4);
        assert_eq!(sv.value_at_bar(193, global), 0.2);
    }

    #[test]
    fn test_canonical_section_layout_is_stable() {
        // Every arrangement template is written in the canonical 7×32-bar
        // layout. If this ever drifts, `remap_bar_range` silently misroutes
        // every clip. Anchor on both the lengths struct and the derived
        // starts so a change to either surfaces here.
        let lengths = crate::als_project::SectionLengths::techno_default();
        assert_eq!(lengths.intro, 32);
        assert_eq!(lengths.build, 32);
        assert_eq!(lengths.breakdown, 32);
        assert_eq!(lengths.drop1, 32);
        assert_eq!(lengths.drop2, 32);
        assert_eq!(lengths.fadedown, 32);
        assert_eq!(lengths.outro, 32);
        assert_eq!(lengths.total_bars(), 224);
        assert_eq!(SONG_LENGTH_BARS, lengths.total_bars());

        let s = lengths.starts();
        assert_eq!(s.intro, (1, 33));
        assert_eq!(s.build, (33, 65));
        assert_eq!(s.breakdown, (65, 97));
        assert_eq!(s.drop1, (97, 129));
        assert_eq!(s.drop2, (129, 161));
        assert_eq!(s.fadedown, (161, 193));
        assert_eq!(s.outro, (193, 225));
    }

    #[test]
    fn test_apply_parallelism_per_section_effective_vals() {
        let mut p_vals = SectionValues::default();
        let v_vals = SectionValues::default();
        let global_p = 0.5;
        let global_v = 0.3;
        
        // Mocking behavior of average-based effective values
        p_vals.set(1, 1.0);
        p_vals.set(193, 1.0);
        
        // Effective parallelism = (1.0 + 1.0) / 2 = 1.0
        // Result should be arrangements unchanged if effective_p >= 1.0
        let arrangements = vec![TrackArrangement::new("KICK", vec![(1.0, 33.0)])];
        let result = apply_parallelism_per_section(arrangements.clone(), &p_vals, global_p, &v_vals, global_v);
        assert_eq!(result.len(), arrangements.len());
        assert_eq!(result[0].sections, arrangements[0].sections);
    }

    #[test]
    fn test_apply_variation_per_section_effective_vals() {
        let mut v_vals = SectionValues::default();
        let global_v = 0.0;
        
        v_vals.set(1, 0.0);
        v_vals.set(193, 0.0);
        
        // Effective variation = (0.0 + 0.0) / 2 = 0.0
        // Result should be arrangements unchanged if effective_v <= 0.0
        let arrangements = vec![TrackArrangement::new("KICK", vec![(1.0, 33.0)])];
        let result = apply_variation_per_section(arrangements.clone(), &v_vals, global_v);
        assert_eq!(result.len(), arrangements.len());
        assert_eq!(result[0].sections, arrangements[0].sections);
    }

    #[test]
    fn test_apply_glitch_edits_no_intensity() {
        let arrangements = vec![TrackArrangement::new("KICK", vec![(1.0, 33.0)])];
        let section_glitch = SectionValues::default();
        let result = apply_glitch_edits(arrangements.clone(), 0.0, &section_glitch);
        assert_eq!(result, arrangements);
    }

    #[test]
    fn test_apply_density_per_section_no_intensity() {
        let arrangements = vec![TrackArrangement::new("HAT", vec![(1.0, 33.0)])];
        let section_density = SectionValues::default();
        let result = apply_density_per_section(arrangements.clone(), &section_density, 0.0);
        assert_eq!(result, arrangements);
    }

    #[test]
    fn test_has_any_logic_in_arrangement_params() {
        let mut overrides = SectionOverrides::default();
        
        // Helper to check if any chaos is detected
        let has_any_chaos = |overrides: &SectionOverrides, chaos: f32| -> bool {
            chaos > 0.0 || overrides.chaos.any()
        };

        assert!(!has_any_chaos(&overrides, 0.0));
        assert!(has_any_chaos(&overrides, 0.1));
        
        overrides.chaos.set(65, 0.5); // breakdown
        assert!(has_any_chaos(&overrides, 0.0));
    }

    #[test]
    fn test_apply_glitch_edits_heavy_intensity() {
        // Use a track that is NOT protected (KICK)
        let arrangements = vec![TrackArrangement::new("KICK", vec![(1.0, 5.0)])];
        let mut section_glitch = SectionValues::default();
        section_glitch.set(1, 1.0); // Heavy glitch in intro
        
        let result = apply_glitch_edits(arrangements.clone(), 1.0, &section_glitch);
        
        // At 1.0 intensity, it should almost certainly modify the arrangement
        // (unless we get extremely unlucky with RNG)
        assert_ne!(result, arrangements, "Heavy glitch should have modified the arrangement");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "KICK");
        // It should have split the 4-bar section into many smaller segments
        assert!(result[0].sections.len() > 1);
    }

    #[test]
    fn test_apply_density_per_section_with_overrides() {
        // Use a densifiable track (HAT)
        let arrangements = vec![TrackArrangement::new("HAT", vec![(1.0, 5.0)])];
        let mut section_density = SectionValues::default();
        section_density.set(1, 1.0); // High density in intro
        
        let result = apply_density_per_section(arrangements.clone(), &section_density, 1.0);
        
        // At 1.0 density, it should likely add accent clips
        assert!(result[0].sections.len() >= arrangements[0].sections.len());
        assert_eq!(result[0].name, "HAT");
    }

    #[test]
    fn test_generate_scatter_hits_logic() {
        let mut section_scatter = SectionValues::default();
        let global_scatter = 0.0;
        
        // No scatter anywhere: should return empty
        let results = generate_scatter_hits(&section_scatter, global_scatter, 4);
        assert!(results.is_empty());
        
        // Global scatter enabled
        let results = generate_scatter_hits(&section_scatter, 0.5, 4);
        assert!(!results.is_empty());
        assert_eq!(results.len(), 4); // It always generates 4 tracks if active
        assert!(results[0].name.contains("SCATTER"));
        
        // Per-section scatter
        section_scatter.set(1, 1.0); // Intro
        let results = generate_scatter_hits(&section_scatter, 0.0, 4);
        assert!(!results.is_empty());
        // Check that hits are only in the intro (bars 1-32)
        for track in results {
            for (start, _) in track.sections {
                assert!(start >= 1.0 && start < 33.0, "Scatter hit at {} outside intro", start);
            }
        }
    }

    #[test]
    fn test_generate_random_fills_logic() {
        let fills = generate_random_fills();
        assert!(!fills.is_empty());
        assert_eq!(fills.len(), 8); // Always generates 8 tracks
        for (i, track) in fills.iter().enumerate() {
            assert_eq!(track.name, format!("FILL {}", i + 1));
            // Fills should only occur at the end of 8-bar phrases
            for (start, end) in &track.sections {
                // Fills are 1, 2, or 4 beats long.
                // 1 beat = 0.25 bars. 4 beats = 1.0 bar.
                assert!(*end > *start);
                assert!(*end - *start <= 1.0);
            }
        }
    }

    #[test]
    fn test_generate_glitch_arrangements_logic() {
        let glitches = generate_glitch_arrangements();
        assert!(!glitches.is_empty());
        assert_eq!(glitches.len(), 8);
        for (i, track) in glitches.iter().enumerate() {
            assert_eq!(track.name, format!("GLITCH {}", i + 1));
            for (start, end) in &track.sections {
                assert!(*end > *start);
            }
        }
    }
}

#[cfg(test)]
mod additional_tests {
    use super::*;

    /// Lock the thread-local RNG, run a helper twice, assert bit-identical
    /// output. `generate_swoosh_arrangements` is ideal for this — it's pure
    /// (no I/O, no filesystem, no DB), reads enough random state that any
    /// leak from uninitialized RNG would diverge, and returns a stable
    /// `Vec<TrackArrangement>` we can compare directly.
    #[test]
    fn test_seeded_helper_is_deterministic() {
        init_gen_rng(0xCAFE_BABE);
        let a = generate_swoosh_arrangements();
        clear_gen_rng();

        init_gen_rng(0xCAFE_BABE);
        let b = generate_swoosh_arrangements();
        clear_gen_rng();

        assert_eq!(a, b, "same seed → identical output");
        assert!(!a.is_empty(), "helper should have produced tracks");
    }

    /// Different seeds should produce different output — otherwise the seed
    /// is effectively ignored. We scan a small set so a fluke collision on
    /// any single pair doesn't flake the test. Uses Debug strings for set
    /// membership because `TrackArrangement` intentionally doesn't implement
    /// `Hash` (float fields).
    #[test]
    fn test_different_seeds_produce_different_output() {
        let runs: Vec<String> = [1u64, 2, 3, 4, 5]
            .iter()
            .map(|&s| {
                init_gen_rng(s);
                let out = generate_swoosh_arrangements();
                clear_gen_rng();
                format!("{:?}", out)
            })
            .collect();
        let unique: std::collections::HashSet<&String> = runs.iter().collect();
        assert!(unique.len() >= 2, "5 distinct seeds must produce ≥2 distinct outputs");
    }

    /// After `clear_gen_rng`, helpers should still work — the fallback path
    /// seeds from wall-clock nanos so tests that never call `generate()` keep
    /// working as they did before the refactor. Non-determinism is acceptable
    /// here; we only assert we get *some* output.
    #[test]
    fn test_helper_works_without_generate_init() {
        clear_gen_rng();
        let out = generate_swoosh_arrangements();
        assert!(!out.is_empty(), "helper should fall back when no seed is set");
    }

    #[test]
    fn test_get_arrangement_basics() {
        let arr = get_arrangement(0.0);
        assert!(!arr.is_empty());
        
        let kick = arr.iter().find(|t| t.name == "KICK").expect("KICK track missing");
        assert!(kick.sections.len() >= 10); // Lots of gaps for fills
        
        let clap = arr.iter().find(|t| t.name == "CLAP").expect("CLAP track missing");
        assert!(clap.sections.iter().all(|s| s.0 >= 9.0)); // Enters at bar 9
    }

    #[test]
    fn test_generate_swoosh_arrangements() {
        let swooshes = generate_swoosh_arrangements();
        assert!(!swooshes.is_empty());
        
        for s in swooshes {
            assert!(s.name.contains("SWEEP"));
            assert!(!s.sections.is_empty());
        }
    }

    #[test]
    fn test_apply_chaos_to_arrangements_protected() {
        let arrangements = vec![TrackArrangement::new("FILL 1", vec![(1.0, 10.0), (20.0, 30.0)])];
        let sv = SectionValues::default();
        let result = apply_chaos_to_arrangements(arrangements.clone(), &sv, 1.0);
        assert_eq!(result, arrangements, "Protected FILL track should not be modified by chaos");
    }

    #[test]
    fn test_apply_parallelism_exempt_tracks() {
        let arrangements = vec![
            TrackArrangement::new("IMPACT 1", vec![(1.0, 10.0)]),
            TrackArrangement::new("IMPACT 2", vec![(1.0, 10.0)]),
        ];
        // Parallelism 0.0 usually thins out to 1 track, but IMPACT is exempt
        let sv = SectionValues::default();
        let result = apply_parallelism(arrangements.clone(), &sv, 0.0, &sv, 0.0);
        assert_eq!(result.len(), 2, "Exempt IMPACT tracks should not be thinned by parallelism");
    }

    #[test]
    fn test_apply_variation_zero() {
        let arrangements = vec![TrackArrangement::new("SYNTH", vec![(1.0, 10.0), (20.0, 30.0), (40.0, 50.0)])];
        let sv = SectionValues::default();
        let result = apply_variation(arrangements.clone(), &sv, 0.0);
        assert_eq!(result, arrangements, "Zero variation should not modify arrangements");
    }

    // ---------- Tier 1 feature tests: sends, sidechain, master chain ----------

    /// Minimal fake of one AudioTrack's Mixer/Sends region — matches the shape
    /// the real template uses so `apply_sends` has the same anchors to find.
    fn fake_audio_track_sends() -> String {
        r#"<AudioTrack Id="99">
            <Name><EffectiveName Value="PAD 1" /></Name>
            <Sends>
                <TrackSendHolder Id="0">
                    <Send>
                        <LomId Value="0" />
                        <Manual Value="0.00031622799" />
                        <MidiControllerRange>
                            <Min Value="0.00031622799" />
                        </MidiControllerRange>
                    </Send>
                </TrackSendHolder>
                <TrackSendHolder Id="2">
                    <Send>
                        <LomId Value="0" />
                        <Manual Value="0.0003162277571" />
                        <MidiControllerRange>
                            <Min Value="0.0003162277571" />
                        </MidiControllerRange>
                    </Send>
                </TrackSendHolder>
            </Sends>
        </AudioTrack>"#.to_string()
    }

    #[test]
    fn send_levels_kick_is_dry() {
        // Low-end tracks must stay dry — reverb/delay on kick muddies the mix.
        let (rev, dly) = send_levels_for("KICK");
        assert!(rev < 0.001, "KICK reverb should be ~-∞ dB, got {}", rev);
        assert!(dly < 0.001, "KICK delay should be ~-∞ dB, got {}", dly);
        // Same for numbered variants
        let (rev, dly) = send_levels_for("KICK 3");
        assert!(rev < 0.001 && dly < 0.001);
    }

    #[test]
    fn send_levels_sub_and_bass_dry() {
        for name in &["SUB", "SUB 2", "BASS 1", "SUB DROP", "BOOM KICK"] {
            let (rev, dly) = send_levels_for(name);
            assert!(rev < 0.001, "{} reverb should be dry, got {}", name, rev);
            assert!(dly < 0.001, "{} delay should be dry, got {}", name, dly);
        }
    }

    #[test]
    fn send_levels_pad_is_wettest_category() {
        // Pads should have the loudest reverb of any category — they're the
        // atmosphere. Compare against drums/bass/leads.
        let (pad_rev, _) = send_levels_for("PAD 1");
        let (kick_rev, _) = send_levels_for("KICK");
        let (clap_rev, _) = send_levels_for("CLAP");
        let (lead_rev, _) = send_levels_for("LEAD 2");
        assert!(pad_rev > kick_rev, "pad reverb should exceed kick");
        assert!(pad_rev > clap_rev, "pad reverb should exceed clap");
        assert!(pad_rev > lead_rev, "pad reverb should exceed lead");
    }

    #[test]
    fn send_levels_lead_has_delay() {
        // Leads need delay for that techno/trance rhythmic ping.
        let (_, lead_dly) = send_levels_for("LEAD 1");
        let (_, pad_dly) = send_levels_for("PAD 1");
        assert!(lead_dly > pad_dly, "lead delay ({}) should exceed pad delay ({})", lead_dly, pad_dly);
    }

    #[test]
    fn apply_sends_replaces_only_the_two_send_manuals() {
        let track = fake_audio_track_sends();
        let out = apply_sends(&track, 0.25, 0.35);
        // Sent values must be present on the two Send<Manual> lines.
        assert!(out.contains(r#"<Manual Value="0.25" />"#));
        assert!(out.contains(r#"<Manual Value="0.35" />"#));
        // MidiControllerRange Min values (unrelated) must NOT have been touched —
        // they share the same numeric string but are Min, not Manual.
        assert!(out.contains(r#"<Min Value="0.00031622799" />"#));
        assert!(out.contains(r#"<Min Value="0.0003162277571" />"#));
        // The original Manual placeholders are gone.
        assert!(!out.contains(r#"<Manual Value="0.00031622799" />"#));
        assert!(!out.contains(r#"<Manual Value="0.0003162277571" />"#));
    }

    #[test]
    fn group_sidechain_compressor_template_is_preconfigured() {
        // Template integrity — if someone re-extracts it from a different
        // reference ALS and forgets to enable the sidechain block, the
        // DRUMS/BASS buses would emit but not actually duck. Guard against it.
        let t = GROUP_SIDECHAIN_COMPRESSOR_TEMPLATE;
        // OnOff must already be Manual="true" in the template (we never flip
        // it at runtime).
        let sc_idx = t.find("<SideChain>").expect("template missing SideChain");
        let onoff_slice = &t[sc_idx..sc_idx + 400];
        assert!(
            onoff_slice.contains(r#"<Manual Value="true" />"#),
            "SideChain OnOff must be pre-enabled"
        );
        // Source routing placeholder must be present for substitution.
        assert!(
            t.contains("__SC_SRC_ID__"),
            "template must include __SC_SRC_ID__ placeholder for source track"
        );
        // Ducking-appropriate threshold (≤ ~-10 dB in Ableton's 0.0005-1.9
        // scale, i.e. < 0.5).
        let thresh_idx = t.find("<Threshold>").unwrap();
        let thresh_slice = &t[thresh_idx..thresh_idx + 400];
        let re = regex::Regex::new(r#"<Manual Value="([0-9.]+)"#).unwrap();
        let cap = re.captures(thresh_slice).expect("Threshold Manual not found");
        let thresh: f64 = cap[1].parse().unwrap();
        assert!(thresh < 0.5, "Threshold {} is too loose for sidechain ducking", thresh);
    }

    #[test]
    fn build_group_sidechain_substitutes_source_id_and_allocates_ids() {
        let ids = IdAllocator::new(5_000_000);
        let out = build_group_sidechain_compressor(4242, &ids);
        assert!(
            out.contains("AudioIn/Track.4242/PostFxOut"),
            "source track id should be substituted"
        );
        assert!(!out.contains("__SC_SRC_ID__"), "placeholder should be gone");
        // Fresh IDs replaced the template's reserved ones.
        assert!(!out.contains(r#"Id="2""#), "template Id=2 should be reallocated");
    }

    #[test]
    fn build_group_sidechain_twice_has_disjoint_ids() {
        // Two groups (DRUMS and BASS) each get their own Compressor2 copy —
        // the IDs must not collide or Ableton will reject the file.
        let ids = IdAllocator::new(5_500_000);
        let a = build_group_sidechain_compressor(100, &ids);
        let b = build_group_sidechain_compressor(100, &ids);
        let id_re = regex::Regex::new(r#"Id="(\d+)""#).unwrap();
        let ids_a: Vec<_> = id_re.captures_iter(&a).map(|c| c[1].to_string()).collect();
        let ids_b: Vec<_> = id_re.captures_iter(&b).map(|c| c[1].to_string()).collect();
        for id in &ids_a {
            assert!(!ids_b.contains(id), "ID {} reused across two group compressors", id);
        }
    }

    #[test]
    fn inject_device_into_group_chain_replaces_empty_devices() {
        // The group template ships with a self-closing `<Devices />`. Inject
        // must expand it to `<Devices>…</Devices>` exactly once, preserving
        // the rest of the group XML.
        let group = r#"<GroupTrack Id="1">
            <DeviceChain>
                <Devices />
                <SignalModulations />
            </DeviceChain>
        </GroupTrack>"#;
        let out = inject_device_into_group_chain(group, "<Compressor2 Id=\"99\" />");
        assert!(!out.contains("<Devices />"), "empty Devices placeholder should be gone");
        assert!(out.contains("<Devices>"), "Devices should now be a container");
        assert!(out.contains("</Devices>"), "Devices container should close");
        assert!(out.contains(r#"<Compressor2 Id="99" />"#), "device content should be injected");
        // Preserved structure.
        assert!(out.contains("<SignalModulations />"));
        // Exactly one replacement happened (only the FIRST <Devices /> is targeted).
        assert_eq!(out.matches("<Compressor2 ").count(), 1);
    }

    #[test]
    fn inject_device_is_idempotent_when_no_empty_devices_present() {
        // If the group already has a populated Devices block, we must not
        // duplicate or mangle it.
        let group = r#"<GroupTrack Id="1">
            <DeviceChain>
                <Devices><EQ Id="5" /></Devices>
            </DeviceChain>
        </GroupTrack>"#;
        let out = inject_device_into_group_chain(group, "<Compressor2 Id=\"99\" />");
        assert_eq!(group, out, "no empty <Devices /> placeholder → no change");
    }

    #[test]
    fn reallocate_device_ids_gives_unique_fresh_ids() {
        let ids = IdAllocator::new(2_000_000);
        let out1 = reallocate_device_ids(r#"<A Id="1"><B Id="2" /><C Id="3" /></A>"#, &ids);
        let out2 = reallocate_device_ids(r#"<A Id="1"><B Id="2" /><C Id="3" /></A>"#, &ids);
        // Original IDs gone.
        for s in [&out1, &out2] {
            assert!(!s.contains(r#"Id="1""#), "template Id=1 should be replaced");
            assert!(!s.contains(r#"Id="2""#));
            assert!(!s.contains(r#"Id="3""#));
        }
        // Two calls produce disjoint ID sets.
        let id_re = regex::Regex::new(r#"Id="(\d+)""#).unwrap();
        let ids1: Vec<_> = id_re.captures_iter(&out1).map(|c| c[1].to_string()).collect();
        let ids2: Vec<_> = id_re.captures_iter(&out2).map(|c| c[1].to_string()).collect();
        for id in &ids1 {
            assert!(!ids2.contains(id), "ID {} reused across calls", id);
        }
    }

    #[test]
    fn inject_master_chain_places_devices_before_existing_stereogain() {
        let ids = IdAllocator::new(3_000_000);
        let xml = r#"<Ableton>
            <Tracks>
                <AudioTrack Id="1">
                    <DeviceChain>
                        <Devices>
                            <Compressor2 Id="10" />
                        </Devices>
                    </DeviceChain>
                </AudioTrack>
            </Tracks>
            <MainTrack>
                <DeviceChain>
                    <Devices>
                        <StereoGain Id="99" />
                    </Devices>
                </DeviceChain>
            </MainTrack>
        </Ableton>"#;
        let out = inject_master_chain(xml, &ids);
        // Master devices present.
        assert!(out.contains("<Eq8 "), "master Eq8 missing");
        assert!(out.contains("<Limiter "), "master Limiter missing");
        // They sit AFTER the MainTrack opening and BEFORE the StereoGain.
        let main_pos = out.find("<MainTrack").unwrap();
        let eq8_pos = out[main_pos..].find("<Eq8 ").expect("Eq8 should be inside MainTrack");
        let stereogain_pos = out[main_pos..].find("<StereoGain").unwrap();
        let limiter_pos = out[main_pos..].find("<Limiter ").expect("Limiter should be inside MainTrack");
        assert!(eq8_pos < stereogain_pos, "Eq8 must precede StereoGain");
        assert!(limiter_pos < stereogain_pos, "Limiter must precede StereoGain");
        assert!(eq8_pos < limiter_pos, "Eq8 must precede Limiter (HPF before peak catch)");
        // AudioTrack's existing Compressor2 must NOT get the master chain injected.
        let first_at = out.find("<AudioTrack").unwrap();
        let first_at_end = out.find("</AudioTrack>").unwrap();
        let at_slice = &out[first_at..first_at_end];
        assert!(!at_slice.contains("<Eq8 "), "Eq8 leaked into AudioTrack");
        assert!(!at_slice.contains("<Limiter "), "Limiter leaked into AudioTrack");
    }

    #[test]
    fn inject_master_chain_is_noop_without_maintrack() {
        let ids = IdAllocator::new(4_000_000);
        let xml = "<Ableton><Tracks></Tracks></Ableton>";
        let out = inject_master_chain(xml, &ids);
        assert_eq!(xml, out, "no MainTrack → no injection");
    }

    #[test]
    fn master_eq8_template_has_band0_hpf_at_30hz() {
        // Template integrity: Band.0 ParameterA must be an active HPF at 30Hz.
        // Regression guard — if someone re-extracts the template from a
        // different source and forgets to bump Freq, this catches it.
        let t = MASTER_EQ8_HPF_TEMPLATE;
        // Only one instance of Freq Manual="30" — the Band.0 HPF we tuned.
        let count = t.matches(r#"<Manual Value="30" />"#).count();
        assert!(count >= 1, "Eq8 template must have at least one Manual=30 (Band.0 Freq)");
        // The first <Freq> block's <Manual> must be "30", not the stock "10".
        let first_freq_block = t.find("<Freq>").expect("no <Freq>");
        let slice = &t[first_freq_block..first_freq_block + 500];
        assert!(slice.contains(r#"<Manual Value="30" />"#), "Band.0 Freq Manual should be 30");
        assert!(!slice.contains(r#"<Manual Value="10" />"#), "Band.0 Freq Manual should not be stock 10");
    }

    #[test]
    fn master_limiter_template_has_safe_ceiling() {
        // Limiter ceiling must be ≤ 0 dB; -0.3 dB is the standard mastering
        // safety margin for inter-sample peaks. If someone edits the template
        // and sets ceiling above 0, clipping returns.
        let t = MASTER_LIMITER_TEMPLATE;
        assert!(t.contains("<Ceiling>"), "limiter missing Ceiling param");
        let ceiling_pos = t.find("<Ceiling>").unwrap();
        let slice = &t[ceiling_pos..ceiling_pos + 400];
        let re = regex::Regex::new(r#"<Manual Value="(-?\d+\.?\d*)"#).unwrap();
        let cap = re.captures(slice).expect("Ceiling Manual not found");
        let val: f64 = cap[1].parse().unwrap();
        assert!(val <= 0.0, "Ceiling must be ≤ 0 dB, got {}", val);
        assert!(val >= -1.0, "Ceiling {} dB is too aggressive (< -1 dB)", val);
    }

    // ---------- 8-bar-block granularity tests (2026-04 refactor) ----------

    #[test]
    fn value_at_bar_resolves_block_by_bar_position_not_section() {
        // The whole point of the 8-bar-block refactor: two bars inside the
        // same section resolve to *different* block values when the blocks
        // differ. Previously this was impossible — a whole 32-bar section
        // shared one value.
        let mut sv = SectionValues::default();
        sv.set(1, 0.1);   // Intro block 1 (bars 1-8)
        sv.set(9, 0.5);   // Intro block 2 (bars 9-16)
        sv.set(17, 0.9);  // Intro block 3 (bars 17-24)
        sv.set(25, 0.2);  // Intro block 4 (bars 25-32)

        // Each bar inside a block resolves to that block's value.
        assert_eq!(sv.value_at_bar(1, 0.0), 0.1);
        assert_eq!(sv.value_at_bar(4, 0.0), 0.1);
        assert_eq!(sv.value_at_bar(8, 0.0), 0.1);
        assert_eq!(sv.value_at_bar(9, 0.0), 0.5);
        assert_eq!(sv.value_at_bar(12, 0.0), 0.5);
        assert_eq!(sv.value_at_bar(16, 0.0), 0.5);
        assert_eq!(sv.value_at_bar(17, 0.0), 0.9);
        assert_eq!(sv.value_at_bar(24, 0.0), 0.9);
        assert_eq!(sv.value_at_bar(25, 0.0), 0.2);
        assert_eq!(sv.value_at_bar(32, 0.0), 0.2);
    }

    #[test]
    fn value_at_bar_unpinned_block_falls_back_to_global() {
        // Only two blocks pinned out of ~28; every other bar must read the
        // global scalar verbatim.
        let mut sv = SectionValues::default();
        sv.set(65, 0.8);  // Breakdown block 1
        sv.set(129, 0.3); // Drop2 block 1

        let g = 0.42;
        assert_eq!(sv.value_at_bar(1, g), g,   "intro block 1 unpinned");
        assert_eq!(sv.value_at_bar(40, g), g,  "build block 2 unpinned");
        assert_eq!(sv.value_at_bar(100, g), g, "drop1 block 2 unpinned");
        assert_eq!(sv.value_at_bar(200, g), g, "outro block 2 unpinned");
        // And the two pinned blocks still return their pinned values.
        assert_eq!(sv.value_at_bar(65, g), 0.8);
        assert_eq!(sv.value_at_bar(129, g), 0.3);
    }

    #[test]
    fn set_snaps_non_block_start_bars_to_block_start() {
        // If a caller (or IPC payload) addresses a bar that isn't a block
        // boundary, we snap down to the containing block. Guards against
        // duplicate overrides from off-by-one callers.
        let mut sv = SectionValues::default();
        sv.set(5, 0.6);  // mid-block 1 → snaps to 1
        sv.set(14, 0.7); // mid-block 2 → snaps to 9
        sv.set(9, 0.4);  // exact block-start → overwrites 9

        assert_eq!(sv.blocks.len(), 2, "two distinct blocks after snapping");
        assert_eq!(sv.value_at_bar(1, 0.0), 0.6);
        assert_eq!(sv.value_at_bar(9, 0.0), 0.4, "overwritten");
    }

    #[test]
    fn set_clamps_value_to_unit_interval() {
        // Bad IPC payloads shouldn't poison the resolver — clamp extremes
        // rather than passing through out-of-range values.
        let mut sv = SectionValues::default();
        sv.set(1, -0.5);
        sv.set(9, 5.0);
        assert_eq!(sv.value_at_bar(1, 0.0), 0.0, "negative clamped to 0");
        assert_eq!(sv.value_at_bar(9, 0.0), 1.0, "over-1 clamped to 1");
    }

    #[test]
    fn section_values_serde_is_flat_bar_keyed_map() {
        // The IPC wire format must be a flat `{"1": 0.5, "9": 0.3}` object —
        // frontend depends on this shape. Serde's transparent on the struct
        // means no `{"blocks": {...}}` wrapper.
        let mut sv = SectionValues::default();
        sv.set(1, 0.5);
        sv.set(9, 0.25);
        let json = serde_json::to_string(&sv).unwrap();
        assert!(json.contains(r#""1":0.5"#), "missing flat bar-1 key: {}", json);
        assert!(json.contains(r#""9":0.25"#), "missing flat bar-9 key: {}", json);
        assert!(!json.contains(r#""blocks""#), "struct wrapper leaked: {}", json);

        // Round-trip: the same shape deserializes into an equivalent map.
        let back: SectionValues = serde_json::from_str(&json).unwrap();
        assert_eq!(back.value_at_bar(1, 0.0), 0.5);
        assert_eq!(back.value_at_bar(9, 0.0), 0.25);
    }

    #[test]
    fn values_iterator_averages_correctly() {
        // apply_*_per_section helpers average every pinned block value —
        // confirm the iterator returns them all (order doesn't matter for a
        // mean).
        let mut sv = SectionValues::default();
        sv.set(1, 0.2);
        sv.set(33, 0.4);
        sv.set(65, 0.6);
        let mean: f32 = sv.values().sum::<f32>() / sv.values().count() as f32;
        assert!((mean - 0.4).abs() < 1e-5, "mean should be 0.4, got {}", mean);
    }

    #[test]
    fn apply_glitch_edits_gates_on_per_block_threshold() {
        // Even when the global scalar is 0, a single pinned block above the
        // 0.05 gate must re-enable glitch editing. Catches the regression we
        // just fixed in the `has_any_glitch` computation.
        let mut sv = SectionValues::default();
        sv.set(97, 0.9); // Drop1 block 1 — heavy glitch
        let arr = vec![TrackArrangement::new("KICK", vec![(97.0, 100.0)])];
        let out = apply_glitch_edits(arr.clone(), 0.0, &sv);
        assert_ne!(out, arr, "a pinned block > 0.05 must trigger glitch even with global=0");
    }

    // ---------- Remap + user-chosen section length tests (2026-04-16) ----------

    fn canonical_starts() -> crate::als_project::SectionStarts {
        crate::als_project::SectionLengths::techno_default().starts()
    }

    #[test]
    fn remap_identity_leaves_canonical_ranges_untouched() {
        // A 32/32/32/32/32/32/32 user layout is the canonical layout — every
        // template range should remap to itself.
        let user = crate::als_project::SectionLengths::techno_default().starts();
        assert_eq!(remap_bar_range(1.0, 16.75, &user), Some((1.0, 16.75)));
        assert_eq!(remap_bar_range(97.0, 104.75, &user), Some((97.0, 104.75)));
        assert_eq!(remap_bar_range(193.0, 208.5, &user), Some((193.0, 208.5)));
        assert_eq!(remap_bar_range(217.0, 224.0, &user), Some((217.0, 224.0)));
    }

    #[test]
    fn remap_shrink_clips_ranges_past_section_end() {
        // Trance with user outro=32 (shrunk from the 48 default). Template
        // has KICK playing `(193.0, 208.5)` in canonical outro (bars 193-224).
        // User's outro in this layout starts at bar 209 (intro 32 + build 32 +
        // breakdown 48 + drop1 32 + drop2 32 + fadedown 32 = 208; outro starts
        // 209) and spans 32 bars (209..=240, exclusive 241).
        let user = crate::als_project::SectionLengths {
            intro: 32, build: 32, breakdown: 48, drop1: 32, drop2: 32, fadedown: 32, outro: 32,
        }.starts();
        assert_eq!(user.outro, (209, 241));

        // canonical (193.0, 208.5) → offset (0, 15.5) → user (209.0, 224.5)
        let remapped = remap_bar_range(193.0, 208.5, &user).expect("range should project");
        assert!((remapped.0 - 209.0).abs() < 1e-9);
        assert!((remapped.1 - 224.5).abs() < 1e-9);

        // A canonical range deep in the outro: (217.0, 224.0) = offsets 24-31
        // from canonical outro start 193. In the user's 32-bar outro starting
        // at bar 209, those offsets project to bars 233-240 — still inside the
        // user's outro (which ends exclusive at 241), no clipping.
        let remapped = remap_bar_range(217.0, 224.0, &user).expect("range should project");
        assert!((remapped.0 - 233.0).abs() < 1e-9, "got {}", remapped.0);
        assert!((remapped.1 - 240.0).abs() < 1e-9, "got {}", remapped.1);
    }

    #[test]
    fn remap_shrink_drops_ranges_starting_past_user_section_end() {
        // If user shrinks outro to 16 bars, canonical ranges starting past
        // bar 209 (offset 16 from canonical outro start) must be dropped —
        // they have no room to play.
        let user = crate::als_project::SectionLengths {
            intro: 32, build: 32, breakdown: 32, drop1: 32, drop2: 32, fadedown: 32, outro: 16,
        }.starts();
        // Canonical outro (193-225); user outro is 16 bars. Range (209.0, 216.75)
        // starts at offset 16 — already past the user's 16-bar outro end.
        assert_eq!(remap_bar_range(209.0, 216.75, &user), None);
        // But (193.0, 208.5) starts at offset 0 — fits in the first 15.5 bars.
        assert!(remap_bar_range(193.0, 208.5, &user).is_some());
    }

    #[test]
    fn remap_clips_range_that_straddles_shrunk_section_end() {
        // A canonical range partially inside a shrunk user section should
        // clip at the user's section end (not silently leak into the next
        // section, which would corrupt the arrangement).
        let user = crate::als_project::SectionLengths {
            intro: 32, build: 32, breakdown: 16, drop1: 32, drop2: 32, fadedown: 32, outro: 32,
        }.starts();
        // User breakdown = 16 bars (bars 65..=80, exclusive 81). Canonical
        // range (89.0, 96.0) is inside canonical breakdown (65-96) at offset
        // 24-31 — past the 16-bar user breakdown. Starts past end → None.
        assert_eq!(remap_bar_range(89.0, 96.0, &user), None);
        // Canonical range (65.0, 80.0) fits (offsets 0-15 of user breakdown).
        assert_eq!(remap_bar_range(65.0, 80.0, &user), Some((65.0, 80.0)));
        // (65.0, 96.0) straddles — clip at user breakdown end (bar 81).
        let r = remap_bar_range(65.0, 96.0, &user).expect("starts inside user section");
        assert_eq!(r.0, 65.0);
        assert!((r.1 - 81.0).abs() < 1e-9, "clipped to user breakdown end, got {}", r.1);
    }

    #[test]
    fn remap_extend_produces_silent_tail() {
        // User extends intro from 32 → 48 bars. Canonical template covers
        // bars 1-32 only (3 gap patterns), then produces no ranges beyond.
        // Remap just passes template ranges through unchanged since offset 0-32
        // fits in user intro (0-48). Bars 33-48 of user intro are silent because
        // no template range addresses them — that's the known extension limitation.
        let user = crate::als_project::SectionLengths {
            intro: 48, build: 32, breakdown: 32, drop1: 32, drop2: 32, fadedown: 32, outro: 32,
        }.starts();
        assert_eq!(user.intro, (1, 49));
        assert_eq!(user.build, (49, 81)); // shifted right by 16
        // Canonical intro ranges (1.0, 32.0) fit unchanged.
        assert_eq!(remap_bar_range(1.0, 32.0, &user), Some((1.0, 32.0)));
        // Canonical build ranges shift by 16.
        assert_eq!(remap_bar_range(33.0, 64.0, &user), Some((49.0, 80.0)));
    }

    #[test]
    fn section_lengths_defaults_match_historical_section_bounds() {
        // Regression guard: the canonical Techno layout stays at 32×7; trance
        // keeps 32/32/48/32/32/32/48 = 256; schranz keeps 32/32/16/32/48/32/16 = 208.
        // These values came from the old (now-deleted) SectionBounds struct
        // and represent shipped defaults users may rely on.
        let techno = crate::als_project::SectionLengths::techno_default();
        assert_eq!(techno.total_bars(), 224);

        let trance = crate::als_project::SectionLengths::trance_default();
        assert_eq!(trance.breakdown, 48);
        assert_eq!(trance.outro, 48);
        assert_eq!(trance.total_bars(), 256);

        let schranz = crate::als_project::SectionLengths::schranz_default();
        assert_eq!(schranz.breakdown, 16);
        assert_eq!(schranz.drop2, 48);
        assert_eq!(schranz.outro, 16);
        assert_eq!(schranz.total_bars(), 208);
    }

    #[test]
    fn section_lengths_sanitize_clamps_and_snaps_to_8bar_grid() {
        // Bad IPC payload: 3-bar intro would mangle every template range.
        // sanitize() snaps each field down to a multiple of 8 and enforces a
        // minimum of 8 (so zero/negative/sub-8 values don't silently pass).
        let bad = crate::als_project::SectionLengths {
            intro: 0, build: 3, breakdown: 15, drop1: 16, drop2: 31, fadedown: 40, outro: 100,
        }.sanitize();
        assert_eq!(bad.intro, 8, "0 → clamped to min 8");
        assert_eq!(bad.build, 8, "3 → snapped down to 0, clamped to min 8");
        assert_eq!(bad.breakdown, 8, "15 → snapped to 8");
        assert_eq!(bad.drop1, 16, "16 → unchanged (already multiple of 8)");
        assert_eq!(bad.drop2, 24, "31 → snapped down to 24");
        assert_eq!(bad.fadedown, 40);
        assert_eq!(bad.outro, 96, "100 → snapped down to 96");
    }

    #[test]
    fn section_starts_chain_correctly_from_lengths() {
        // The sum-chain invariant: outro.end - 1 == total_bars.
        let lengths = crate::als_project::SectionLengths {
            intro: 16, build: 32, breakdown: 48, drop1: 32, drop2: 32, fadedown: 32, outro: 32,
        };
        let s = lengths.starts();
        assert_eq!(s.intro, (1, 17));
        assert_eq!(s.build, (17, 49));
        assert_eq!(s.breakdown, (49, 97));
        assert_eq!(s.drop1, (97, 129));
        assert_eq!(s.drop2, (129, 161));
        assert_eq!(s.fadedown, (161, 193));
        assert_eq!(s.outro, (193, 225));
        assert_eq!(s.total_bars(), lengths.total_bars());
        assert_eq!(s.total_bars(), 224);
    }

    #[test]
    fn canonical_starts_matches_historical_consts() {
        // The "remap against" layout must stay at the 224-bar Techno grid —
        // every template is authored against it. Confirm the canonical
        // accessor returns the values that used to be compile-time consts.
        let c = canonical_starts();
        assert_eq!(c.intro, (1, 33));
        assert_eq!(c.build, (33, 65));
        assert_eq!(c.breakdown, (65, 97));
        assert_eq!(c.drop1, (97, 129));
        assert_eq!(c.drop2, (129, 161));
        assert_eq!(c.fadedown, (161, 193));
        assert_eq!(c.outro, (193, 225));
    }
}
