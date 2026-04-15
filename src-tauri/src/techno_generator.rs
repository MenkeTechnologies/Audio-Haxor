//! Techno arrangement generator — extracted from generate_true_techno example.
//! Produces valid ALS files using the embedded template approach.

use crate::als_generator::generate_empty_als;
use crate::write_app_log;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use rand::prelude::*;
use regex::Regex;
use std::collections::HashSet;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use std::sync::atomic::{AtomicU32, Ordering};

// Silence between songs (bars)
const GAP_BETWEEN_SONGS: u32 = 32;

// Arrangement structure (224 bars = 7 minutes at 128 BPM)
// All values in bars (1-indexed)
const SONG_LENGTH_BARS: u32 = 224;

// Section start positions (all 32 bars each)
const INTRO_START: u32 = 1;
const BUILD1_START: u32 = 33;
const BREAKDOWN_START: u32 = 65;
const DROP1_START: u32 = 97;
const DROP2_START: u32 = 129;
const FADEDOWN_START: u32 = 161;
const OUTRO_START: u32 = 193;

// Element entry/exit positions (in bars, supports fractional for beat precision)
// 16.75 = bar 16, beat 4 (last beat of bar 16)
// 17.0 = bar 17, beat 1 (downbeat)
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
    let mut rng = rand::rng();
    
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
    shuffled_up.shuffle(&mut rng);
    shuffled_down.shuffle(&mut rng);
    
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
}

/// Generate scattered one-shot hits on beat grid.
/// 
/// Creates random hit patterns over 32-bar sections that repeat throughout the track.
/// Multiple SCATTER tracks with different samples fire at random beat positions.
/// Density controls how many hits per 32 bars (0.0 = none, 1.0 = ~1 hit per bar average).
/// 
/// NOTE: Ableton XML requires integer beat values (bars * 4), so we use 1/4 bar (1 beat) grid.
fn generate_scatter_hits(density: f32) -> Vec<TrackArrangement> {
    if density <= 0.0 {
        return vec![];
    }
    
    let mut rng = rand::rng();
    
    // Work on beat grid (4 beats per bar) - Ableton requires integer beat values
    const BEATS_PER_BAR: u32 = 4;
    const PATTERN_BARS: u32 = 32;
    const BEATS_PER_PATTERN: u32 = PATTERN_BARS * BEATS_PER_BAR; // 128 beats
    
    // Song sections where scatter hits play (breakdown + drops mainly)
    let sections: Vec<(u32, u32)> = vec![
        (65, 96),   // breakdown
        (97, 128),  // drop 1
        (129, 160), // drop 2
        (161, 192), // fadedown
    ];
    
    // Generate 4 scatter tracks with different patterns
    let mut results: Vec<TrackArrangement> = Vec::new();
    
    for track_num in 1..=4u32 {
        // Hits per 32 bars based on density (density 1.0 = ~32 hits, density 0.5 = ~16 hits)
        let target_hits = ((density * 32.0) as u32).max(2);
        
        // Generate a 32-bar pattern of random beat positions (0-127)
        let mut pattern_beats: Vec<u32> = Vec::new();
        
        // Each track has different density - track 1 most dense, track 4 least
        let track_density = target_hits / track_num;
        
        // Pick random beat positions, avoiding consecutive hits
        let mut attempts = 0;
        while pattern_beats.len() < track_density as usize && attempts < 1000 {
            let beat: u32 = rng.random_range(0..BEATS_PER_PATTERN);
            
            // Avoid hits within 2 beats of each other for this track
            let too_close = pattern_beats.iter().any(|&b| {
                let diff = if beat > b { beat - b } else { b - beat };
                diff < 2
            });
            
            if !too_close {
                pattern_beats.push(beat);
            }
            attempts += 1;
        }
        
        pattern_beats.sort();
        
        // Convert pattern to actual clip positions for each section
        let mut sections_out: Vec<(f64, f64)> = Vec::new();
        
        for (section_start, section_end) in &sections {
            let section_bars = section_end - section_start;
            
            // Repeat the 32-bar pattern across the section
            let mut bar_offset = 0u32;
            while bar_offset < section_bars {
                for &beat in &pattern_beats {
                    // Convert beat to bar position
                    let beat_bar = beat / BEATS_PER_BAR;
                    let beat_within_bar = beat % BEATS_PER_BAR;
                    
                    if beat_bar >= PATTERN_BARS.min(section_bars - bar_offset) {
                        continue;
                    }
                    
                    let abs_bar = section_start + bar_offset + beat_bar;
                    
                    if abs_bar >= *section_end {
                        continue;
                    }
                    
                    // Position as bar + fraction (0.25 per beat)
                    let abs_pos: f64 = abs_bar as f64 + (beat_within_bar as f64 * 0.25);
                    
                    // One-shot = start and end at same position (single hit)
                    sections_out.push((abs_pos, abs_pos));
                }
                bar_offset += PATTERN_BARS;
            }
        }
        
        if !sections_out.is_empty() {
            results.push(TrackArrangement::new(&format!("SCATTER {}", track_num), sections_out));
        }
    }
    
    results
}

/// Generate randomized fill arrangements for variety.
/// 
/// Fill positions are at phrase boundaries (every 8 bars), but the LENGTH of each fill
/// (1-beat, 2-beat, or 4-beat) and which SAMPLE (A, B, C, D) is randomized.
/// This prevents the "machine gun" effect of predictable fill patterns.
fn generate_random_fills() -> Vec<TrackArrangement> {
    let mut rng = rand::rng();
    
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
            let choice = *weights.choose(&mut rng).unwrap();
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
}

/// Generate glitch arrangements at fill positions (same timing as fills).
/// Glitches add variety and are placed at phrase boundaries.
fn generate_glitch_arrangements() -> Vec<TrackArrangement> {
    use rand::seq::SliceRandom;
    let mut rng = rand::rng();
    
    // Fill positions (every 8 bars)
    let mut positions: Vec<u32> = vec![
        16, 24, 32, 40, 48, 56, 64, 72, 80, 88, 96, 104, 112, 120, 128, 136, 144, 152, 160, 168, 176, 184, 192, 200, 208, 216
    ];
    positions.shuffle(&mut rng);
    
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
        TrackArrangement::new("SUB DROP", vec![
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
        TrackArrangement::new("BOOM KICK", vec![
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
        base = apply_chaos_to_arrangements(base, chaos);
    }
    
    base
}

fn get_arrangement_with_params(chaos: f32, glitch_intensity: f32, density: f32) -> Vec<TrackArrangement> {
    let mut arrangements = get_arrangement(chaos);
    
    // Add scattered one-shot hits on 1/16 grid (density-controlled)
    if density > 0.0 {
        arrangements.extend(generate_scatter_hits(density));
    }
    
    // Apply glitch edits (beat-level micro-edits, stutters, dropouts)
    if glitch_intensity > 0.0 {
        arrangements = apply_glitch_edits(arrangements, glitch_intensity);
    }
    
    arrangements
}

/// Apply chaos to arrangements: random gaps + call-and-response patterns
/// chaos 0.0 = no changes, 1.0 = maximum randomization
fn apply_chaos_to_arrangements(mut arrangements: Vec<TrackArrangement>, chaos: f32) -> Vec<TrackArrangement> {
    let mut rng = rand::rng();
    
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
        
        // 2. Call-and-response: for melodic tracks, shift some sections by 2-4 bars
        if call_response_prefixes.iter().any(|p| arr.name.starts_with(p)) {
            let has_number = arr.name.chars().last().map(|c| c.is_ascii_digit()).unwrap_or(false);
            if has_number {
                let shift_chance = chaos * 0.3;
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
            let stagger_chance = chaos * 0.25;
            if rng.random_bool(stagger_chance as f64) {
                // Delay first section by 2-4 bars (not remove it entirely)
                let delay = if rng.random_bool(0.5) { 2.0 } else { 4.0 };
                if let Some((start, end)) = new_sections.first_mut() {
                    if *end - *start > delay + 2.0 {
                        *start += delay;
                    }
                }
            }
        }
        
        arr.sections = new_sections;
    }
    
    arrangements
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
fn apply_glitch_edits(mut arrangements: Vec<TrackArrangement>, glitch_intensity: f32) -> Vec<TrackArrangement> {
    if glitch_intensity < 0.05 {
        return arrangements;
    }
    
    let mut rng = rand::rng();
    
    // Snap to beat grid (0.25 bar = 1 beat)
    let snap = |v: f64| -> f64 { (v * 4.0).round() / 4.0 };
    
    // Tracks that get different glitch treatments
    let kick_tracks = ["KICK"];
    let drum_tracks = ["CLAP", "SNARE", "HAT", "PERC", "RIDE"];
    let bass_tracks = ["BASS", "SUB"];
    let melodic_tracks = ["SYNTH", "PAD", "LEAD", "ARP"];
    
    // Tracks that should NOT be glitched (one-shots, FX)
    let protected = ["FILL", "IMPACT", "CRASH", "RISER", "DOWNLIFTER", "SUB DROP", "BOOM KICK", 
                     "SNARE ROLL", "GLITCH", "REVERSE", "SWEEP", "ATMOS", "VOX"];
    
    // Scale probabilities based on intensity - at 1.0, we want LOTS of glitches
    let gi = glitch_intensity as f64;
    
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
                let bar_num = current as u32;
                
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
        let filtered: Vec<(f64, f64)> = new_sections
            .into_iter()
            .map(|(s, e)| (snap(s), snap(e)))
            .filter(|(s, e)| e > s)
            .collect();
        
        arr.sections = filtered;
    }
    
    arrangements
}

const GROUP_TRACK_TEMPLATE: &str = include_str!("group_track_template.xml");

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
    fn from_db(path: &str, db_duration: f64, db_size: u64, db_bpm: Option<f64>) -> SampleInfo {
        let name = Path::new(path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("sample")
            .to_string();

        // Trust DB values - don't hit disk for every sample (network mounts are slow)
        let file_size = if db_size > 0 { db_size } else { 0 };
        let duration_secs = if db_duration > 0.0 { db_duration } else { 1.0 };

        // Prefer BPM from filename (most reliable) over DB metadata
        let filename_bpm = crate::sample_analysis::extract_bpm(&name).map(|b| b as f64);
        let bpm = filename_bpm.or(db_bpm);

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

    fn loop_bars(&self, project_bpm: f64) -> u32 {
        let bpm = self.bpm.unwrap_or(project_bpm);
        let duration = if self.duration_secs <= 0.0 || self.duration_secs > 300.0 {
            (4.0 * 60.0 * 4.0) / project_bpm
        } else {
            self.duration_secs
        };

        if bpm <= 0.0 {
            return 4;
        }
        let bars = (duration * bpm) / (60.0 * 4.0);
        if bars <= 0.75 { 1 }
        else if bars <= 1.5 { 1 }
        else if bars <= 3.0 { 2 }
        else if bars <= 6.0 { 4 }
        else if bars <= 12.0 { 8 }
        else { 16 }
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
// This ensures variety when generating multiple projects in a session
use std::sync::Mutex;
static GENERATION_BLACKLIST: std::sync::LazyLock<Mutex<HashSet<String>>> = 
    std::sync::LazyLock::new(|| Mutex::new(HashSet::new()));

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
    // Also add to persistent blacklist
    if let Ok(mut blacklist) = GENERATION_BLACKLIST.lock() {
        blacklist.insert(path.to_string());
    }
}

fn is_sample_used(path: &str) -> bool {
    // Check both current generation and persistent blacklist
    let in_current = USED_SAMPLES.with(|s| s.borrow().contains(path));
    if in_current {
        return true;
    }
    // Check persistent blacklist
    if let Ok(blacklist) = GENERATION_BLACKLIST.lock() {
        return blacklist.contains(path);
    }
    false
}

/// Clear the persistent blacklist (call when user wants fresh samples)
pub fn clear_sample_blacklist() {
    if let Ok(mut blacklist) = GENERATION_BLACKLIST.lock() {
        let count = blacklist.len();
        blacklist.clear();
        write_app_log(format!("[techno_generator] Cleared sample blacklist ({} samples)", count));
    }
}

/// Get the number of samples in the blacklist
pub fn get_blacklist_count() -> usize {
    GENERATION_BLACKLIST.lock().map(|b| b.len()).unwrap_or(0)
}


fn query_samples_with_key(
    label: &str,
    include_patterns: &[&str],
    require_loop: bool,
    count: usize,
    key: Option<&str>,
) -> Vec<SampleInfo> {
    // Strict key filtering - no fallback to wrong keys
    let results = query_samples_internal(label, include_patterns, require_loop, count, key);

    if results.is_empty() && key.is_some() {
        write_app_log(format!("[techno_generator] {}: No samples with key in filename - track will be empty", label));
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
        "SELECT s.path, COALESCE(s.duration, 0), s.bpm, COALESCE(s.size, 0)
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
        Ok((path, duration, bpm, size))
    })
    .ok()
    .map(|rows| {
        rows.filter_map(|r| r.ok())
            .map(|(path, duration, bpm, size)| SampleInfo::from_db(&path, duration, size, bpm))
            .collect()
    })
    .unwrap_or_default();

    // Filter out:
    // 1. Samples that don't actually contain any include_pattern (FTS5 can over-match)
    // 2. Reversed samples (files ending with -R.wav, _R.wav, etc.)
    // 3. Ableton project samples (frozen, consolidated, rendered from sessions)
    // 4. Bad genres (checked on directory path only, not filename)
    use crate::sample_filters::{REVERSED_SUFFIXES, PROJECT_RENDER_KEYWORDS, BAD_GENRES, is_ableton_project_sample};
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
            // Skip samples inside Ableton project directories
            if is_ableton_project_sample(&s.path) {
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
                    false // No key in filename = skip when key filtering
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
    
    // Shuffle first to randomize samples with similar scores, then stable sort by score
    use rand::seq::SliceRandom;
    let mut rng = rand::rng();
    scored.shuffle(&mut rng);
    
    // Stable sort by score (higher = better match for current hardness)
    scored.sort_by(|a, b| {
        b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal)
    });
    
    // Filter out already-used samples and take count
    let mut results: Vec<SampleInfo> = Vec::with_capacity(count);
    for (sample, _score) in scored {
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

// Section locators for arrangement navigation (for one song)
fn get_song_locators() -> Vec<(&'static str, u32)> {
    vec![
        ("INTRO", INTRO_START),        // 1
        ("BUILD", BUILD1_START),       // 33
        ("BREAKDOWN", BREAKDOWN_START),// 65
        ("DROP 1", DROP1_START),       // 97
        ("DROP 2", DROP2_START),       // 129
        ("FADEDOWN", FADEDOWN_START),  // 161
        ("OUTRO", OUTRO_START),        // 193
    ]
}

fn create_locators_xml_multi(ids: &IdAllocator, num_songs: u32, song_keys: &[String]) -> String {
    let mut locators: Vec<String> = Vec::new();
    let bars_per_song = SONG_LENGTH_BARS + GAP_BETWEEN_SONGS;

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
        for (name, bar) in get_song_locators() {
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

    let clap_inc = &["clap_loop", "clap loop", "clap"];
    let claps = query_n_keyed!("CLAP", clap_inc, true, track_counts.clap, key_for(type_atonal.clap));

    let snare_inc = &["snare_loop", "snare loop", "snare"];
    let snares = query_n_keyed!("SNARE", snare_inc, true, track_counts.snare, key_for(type_atonal.snare));

    let hat_inc = &["hat_loop", "hihat_loop", "top_loop", "closed_hat", "open_hat", "hats/", "/hats"];
    let hats = query_n_keyed!("HAT", hat_inc, true, track_counts.hat, key_for(type_atonal.hat));

    let perc_inc = &["perc_loop", "percussion_loop", "percussion_&_top", "perc loop", "percussion loop", "top_loop", "shaker", "tom_loop", "conga", "bongo"];
    let percs = query_n_keyed!("PERC", perc_inc, true, track_counts.perc, key_for(type_atonal.perc));

    let ride_inc = &["ride_loop", "ride loop", "cymbal_loop", "cymbal loop", "cymbals/"];
    let rides = query_n_keyed!("RIDE", ride_inc, true, track_counts.ride, key_for(type_atonal.ride));

    let fill_inc = &["drum_fill", "drum fill", "fills/", "fill", "break", "breaks/"];
    let fills = query_n_keyed!("FILL", fill_inc, false, track_counts.fill, key_for(type_atonal.fill));

    // === BASS (key matched unless atonal) ===
    let bass_inc = &["bass_loop", "bass loop", "bass_loops/", "bassline", "basslines/", "reeses_and_hoovers"];
    let basses = query_n_keyed!("BASS", bass_inc, true, track_counts.bass, key_for(type_atonal.bass));

    let sub_inc = &["sub_loop", "sub loop", "sub_bass", "808_loop", "808 loop"];
    let subs = query_n_keyed!("SUB", sub_inc, true, track_counts.sub, key_for(type_atonal.sub));

    // === MELODICS (key matched unless atonal) ===
    let lead_inc = &["lead_loop", "lead loop", "synth_lead", "lead/"];
    let leads = query_n_keyed!("LEAD", lead_inc, true, track_counts.lead, key_for(type_atonal.lead));

    let synth_inc = &["synth_loop", "synth loop", "synth_loops/", "synth/", "music_loops/", "melody_loop", "acid_loop"];
    let synths = query_n_keyed!("SYNTH", synth_inc, true, track_counts.synth, key_for(type_atonal.synth));

    let pad_inc = &["pad_loop", "pad loop", "pad_loops/", "pad/", "pads/", "drone_loop", "atmosphere_loop"];
    let pads = query_n_keyed!("PAD", pad_inc, true, track_counts.pad, key_for(type_atonal.pad));

    let arp_inc = &["arp_loop", "arp loop", "arpegg", "arpeggio", "arp/", "arps/", "pluck_loop", "sequence_loop", "pluck/", "piano/"];
    let arps = query_n_keyed!("ARP", arp_inc, true, track_counts.arp, key_for(type_atonal.arp));

    // === FX (mixed - some tonal, some not) ===
    let riser_inc = &["riser", "risers___lifters", "uplifter", "riser/", "build", "tension"];
    let risers = query_n_keyed!("RISER", riser_inc, false, track_counts.riser, key_for(type_atonal.riser));

    let downlifter_inc = &["downlifter", "falls___descenders", "fall", "descend"];
    let downlifters = query_n_keyed!("DOWNLIFTER", downlifter_inc, false, track_counts.downlifter, key_for(type_atonal.downlifter));

    let crash_inc = &["crash", "cymbal_crash", "crash___cymbals", "cymbal_hit"];
    let crashes = query_n_keyed!("CRASH", crash_inc, false, track_counts.crash, key_for(type_atonal.crash));

    let impact_inc = &["impact", "impacts___bombs", "boom", "thud", "slam", "low impact"];
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

    let boom_kick_inc = &["kick fx", "kick_fx", "impact fx", "boom kick", "boom_kick", "reverb kick", "reverb_kick", "impacts___bombs"];
    let boom_kicks = query_n_keyed!("BOOM_KICK", boom_kick_inc, false, track_counts.boom_kick, key_for(type_atonal.boom_kick));

    let atmos_inc = &["atmos", "atmosphere", "atmospheres/", "ambient", "texture", "drone", "soundscape"];
    let atmoses = query_n_keyed!("ATMOS", atmos_inc, false, track_counts.atmos, key_for(type_atonal.atmos));

    let glitch_inc = &["glitch", "glitches/", "glitch_fx", "glitch fx", "stutter_fx", "stutter fx", "stutters/", "glitch_loop", "glitch loop"];
    let glitches = query_n_keyed!("GLITCH", glitch_inc, false, track_counts.glitch, key_for(type_atonal.glitch));

    // Scatter hits - short one-shots for random placement (perc stabs, hits, blips, zaps)
    let scatter_inc = &["perc shot", "perc_shot", "blip", "zap", "stab", "click", "tick", "one shot", "one_shot", "fx shot", "fx_shot"];
    let scatters = query_n_keyed!("SCATTER", scatter_inc, false, track_counts.scatter, key_for(type_atonal.scatter));

    // === VOCALS ===
    let vox_inc = &["vox", "vocal", "voice", "vocals/", "vocal_cut", "vocal cut", "vocal_loop", "choir", "chant"];
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
        leads, synths, pads, arps,
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
            lead: 1, synth: 3, pad: 2, arp: 2,
            riser: 3, downlifter: 1, crash: 2, impact: 2, hit: 2, sweep_up: 4, sweep_down: 4, snare_roll: 1, reverse: 2, sub_drop: 2, boom_kick: 2, atmos: 2, glitch: 2, scatter: 4,
            vox: 1,
        }
    }
}

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
    density: f32,
    atonal: bool,
    track_counts: TrackCounts,
    type_atonal: TypeAtonal,
    cancel: Option<&std::sync::atomic::AtomicBool>,
    on_progress: Option<&dyn Fn(&str)>,
) -> Result<GenerationResult, String> {
    let gen_start = std::time::Instant::now();
    write_app_log(format!(
        "[techno_generator] generate: INPUT PARAMS: output={:?}, bpm={}, num_songs={}, root_note={:?}, mode={:?}, genre={:?}, hardness={}, chaos={}, glitch_intensity={}, density={}, atonal={}, tracks={:?}, type_atonal={:?}",
        output_path, bpm, num_songs, root_note, mode, genre, hardness, chaos, glitch_intensity, density, atonal, track_counts, type_atonal
    ));

    let cancelled = || cancel.map_or(false, |c| c.load(std::sync::atomic::Ordering::Relaxed));
    let progress = |msg: &str| { if let Some(cb) = on_progress { cb(msg); } };

    let ids = IdAllocator::new(1000000);
    let bars_per_song = SONG_LENGTH_BARS + GAP_BETWEEN_SONGS;
    let total_bars = bars_per_song * num_songs;
    write_app_log(format!(
        "[techno_generator] generate: COMPUTED: bars_per_song={}, total_bars={}, song_length={} bars, gap={} bars",
        bars_per_song, total_bars, SONG_LENGTH_BARS, GAP_BETWEEN_SONGS
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

    // Allocate group IDs
    let drums_group_id = ids.alloc();
    let bass_group_id = ids.alloc();
    let bass_fx_group_id = ids.alloc();
    let melodics_group_id = ids.alloc();
    let fx_group_id = ids.alloc();

    // Create groups
    let drums_group = create_group_track("DRUMS", DRUMS_COLOR, drums_group_id, &ids)?;
    let bass_group = create_group_track("BASS", BASS_COLOR, bass_group_id, &ids)?;
    let bass_fx_group = create_group_track("BASS FX", BASS_COLOR, bass_fx_group_id, &ids)?;
    let melodics_group = create_group_track("MELODICS", MELODICS_COLOR, melodics_group_id, &ids)?;
    let fx_group = create_group_track("FX", FX_COLOR, fx_group_id, &ids)?;

    // Get arrangement structure with chaos, glitch, and density applied
    let arrangements = get_arrangement_with_params(chaos, glitch_intensity, density);
    
    // Default full-song arrangement for extra loop tracks (play throughout most of the song)
    let full_arrangement: Vec<(f64, f64)> = vec![
        (1.0, 32.0),     // Intro
        (33.0, 64.0),    // Build
        (65.0, 96.0),    // Breakdown
        (97.0, 128.0),   // Drop 1
        (129.0, 160.0),  // Drop 2
        (161.0, 192.0),  // Fadedown
        (193.0, 224.0),  // Outro
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
            ("SUB DROP ", "SUB DROP"),
            ("BOOM KICK ", "BOOM KICK"),
            // Melodics
            ("LEAD ", "LEAD 1"),
            ("SYNTH ", "SYNTH 1"),
            ("PAD ", "PAD 1"),
            ("ARP ", "ARP 1"),
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
            if let Some(num_str) = name.strip_prefix(prefix) {
                if let Ok(layer_num) = num_str.parse::<usize>() {
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
        }
        
        // For dynamic tracks (DRUM LOOP N, BASS LOOP N, etc.), use full arrangement
        if name.starts_with("DRUM LOOP ") || name.starts_with("BASS LOOP ") 
            || name.starts_with("SYNTH LOOP ") || name.starts_with("PAD LOOP ") {
            return full_arrangement.clone();
        }
        vec![]
    };

    if cancelled() { return Err("Generation cancelled".into()); }

    // Count total tracks to create (for progress bar)
    let total_tracks = 5 // group tracks (DRUMS, BASS, BASS FX, MELODICS, FX)
        + song1.kicks.iter().filter(|v| !v.is_empty()).count()
        + song1.claps.iter().filter(|v| !v.is_empty()).count()
        + song1.snares.iter().filter(|v| !v.is_empty()).count()
        + song1.hats.iter().filter(|v| !v.is_empty()).count()
        + song1.percs.iter().filter(|v| !v.is_empty()).count()
        + song1.rides.iter().filter(|v| !v.is_empty()).count()
        + song1.fills.iter().filter(|v| !v.is_empty()).count()
        + song1.basses.iter().filter(|v| !v.is_empty()).count()
        + song1.subs.iter().filter(|v| !v.is_empty()).count()
        + song1.leads.iter().filter(|v| !v.is_empty()).count()
        + song1.synths.iter().filter(|v| !v.is_empty()).count()
        + song1.pads.iter().filter(|v| !v.is_empty()).count()
        + song1.arps.iter().filter(|v| !v.is_empty()).count()
        + song1.risers.iter().filter(|v| !v.is_empty()).count()
        + song1.downlifters.iter().filter(|v| !v.is_empty()).count()
        + song1.crashes.iter().filter(|v| !v.is_empty()).count()
        + song1.impacts.iter().filter(|v| !v.is_empty()).count()
        + song1.hits.iter().filter(|v| !v.is_empty()).count()
        + song1.sweep_ups.iter().filter(|v| !v.is_empty()).count()
        + song1.sweep_downs.iter().filter(|v| !v.is_empty()).count()
        + song1.snare_rolls.iter().filter(|v| !v.is_empty()).count()
        + song1.reverses.iter().filter(|v| !v.is_empty()).count()
        + song1.sub_drops.iter().filter(|v| !v.is_empty()).count()
        + song1.boom_kicks.iter().filter(|v| !v.is_empty()).count()
        + song1.atmoses.iter().filter(|v| !v.is_empty()).count()
        + song1.glitches.iter().filter(|v| !v.is_empty()).count()
        + song1.scatters.iter().filter(|v| !v.is_empty()).count()
        + song1.voxes.iter().filter(|v| !v.is_empty()).count();

    let mut tracks_created = 0usize;
    let report_progress = |created: usize, total: usize| {
        if let Some(cb) = on_progress {
            cb(&format!("TRACK_PROGRESS:{}:{}", created, total));
        }
    };

    report_progress(0, total_tracks);

    let mut warnings: Vec<String> = Vec::new();
    let mut all_tracks: Vec<String> = Vec::new();

    // Macro to reduce repetition in track creation
    // Always use numbered names (e.g., "HAT 1", "HAT 2") for consistent arrangement lookup
    macro_rules! create_tracks {
        ($samples:expr, $base_name:expr, $color:expr, $group_id:expr) => {
            for i in 0..$samples.len() {
                let name = format!("{} {}", $base_name, i + 1);
                if !$samples[i].is_empty() {
                    match create_arranged_track_multi(&original_audio_track, &name, $color, $group_id, &all_songs, &find_arr(&name), &ids, bpm, bars_per_song) {
                        Ok(track) => all_tracks.push(track),
                        Err(e) => warnings.push(format!("{}: {}", name, e)),
                    }
                    tracks_created += 1;
                    report_progress(tracks_created, total_tracks);
                }
            }
        };
    }

    // Add group tracks first
    all_tracks.push(drums_group.clone());
    tracks_created += 1;
    report_progress(tracks_created, total_tracks);
    
    // === DRUMS ===
    create_tracks!(song1.kicks, "KICK", DRUMS_COLOR, drums_group_id as i32);
    create_tracks!(song1.claps, "CLAP", DRUMS_COLOR, drums_group_id as i32);
    create_tracks!(song1.snares, "SNARE", DRUMS_COLOR, drums_group_id as i32);
    create_tracks!(song1.hats, "HAT", DRUMS_COLOR, drums_group_id as i32);
    create_tracks!(song1.percs, "PERC", DRUMS_COLOR, drums_group_id as i32);
    create_tracks!(song1.rides, "RIDE", DRUMS_COLOR, drums_group_id as i32);
    create_tracks!(song1.fills, "FILL", DRUMS_COLOR, drums_group_id as i32);

    // === BASS ===
    all_tracks.push(bass_group.clone());
    tracks_created += 1;
    report_progress(tracks_created, total_tracks);
    
    create_tracks!(song1.basses, "BASS", BASS_COLOR, bass_group_id as i32);
    create_tracks!(song1.subs, "SUB", BASS_COLOR, bass_group_id as i32);

    // === BASS FX ===
    all_tracks.push(bass_fx_group.clone());
    tracks_created += 1;
    report_progress(tracks_created, total_tracks);
    
    create_tracks!(song1.sub_drops, "SUB DROP", BASS_COLOR, bass_fx_group_id as i32);
    create_tracks!(song1.boom_kicks, "BOOM KICK", BASS_COLOR, bass_fx_group_id as i32);

    // === MELODICS ===
    all_tracks.push(melodics_group.clone());
    tracks_created += 1;
    report_progress(tracks_created, total_tracks);
    
    create_tracks!(song1.leads, "LEAD", MELODICS_COLOR, melodics_group_id as i32);
    create_tracks!(song1.synths, "SYNTH", MELODICS_COLOR, melodics_group_id as i32);
    create_tracks!(song1.pads, "PAD", MELODICS_COLOR, melodics_group_id as i32);
    create_tracks!(song1.arps, "ARP", MELODICS_COLOR, melodics_group_id as i32);
    create_tracks!(song1.atmoses, "ATMOS", MELODICS_COLOR, melodics_group_id as i32);

    // === FX ===
    all_tracks.push(fx_group.clone());
    tracks_created += 1;
    report_progress(tracks_created, total_tracks);
    
    create_tracks!(song1.risers, "RISER", FX_COLOR, fx_group_id as i32);
    create_tracks!(song1.downlifters, "DOWNLIFTER", FX_COLOR, fx_group_id as i32);
    create_tracks!(song1.crashes, "CRASH", FX_COLOR, fx_group_id as i32);
    create_tracks!(song1.impacts, "IMPACT", FX_COLOR, fx_group_id as i32);
    create_tracks!(song1.hits, "HIT", FX_COLOR, fx_group_id as i32);
    create_tracks!(song1.sweep_ups, "SWEEP UP", FX_COLOR, fx_group_id as i32);
    create_tracks!(song1.sweep_downs, "SWEEP DOWN", FX_COLOR, fx_group_id as i32);
    create_tracks!(song1.snare_rolls, "SNARE ROLL", FX_COLOR, fx_group_id as i32);
    create_tracks!(song1.reverses, "REVERSE", FX_COLOR, fx_group_id as i32);
    create_tracks!(song1.glitches, "GLITCH", FX_COLOR, fx_group_id as i32);
    create_tracks!(song1.scatters, "SCATTER", FX_COLOR, fx_group_id as i32);

    // === VOCALS ===
    create_tracks!(song1.voxes, "VOX", FX_COLOR, fx_group_id as i32);

    // Log warnings
    for w in &warnings {
        write_app_log(format!("[techno_generator] WARNING: {}", w));
    }

    progress("Assembling XML");
    // Build final XML - all tracks
    let before_track = &xml[..track_start];
    let after_track = &xml[track_end..];

    let track_count = all_tracks.len();
    let clip_count: usize = all_tracks.iter().map(|t| t.matches("<AudioClip").count()).sum();
    let all_tracks_xml = all_tracks.join("\n\t\t\t");

    let mut xml = format!("{}{}{}", before_track, all_tracks_xml, after_track);

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
    let locators_xml = create_locators_xml_multi(&ids, num_songs, &song_keys);
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
        bars: (SONG_LENGTH_BARS + GAP_BETWEEN_SONGS) * num_songs,
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

    // Clip length in beats - if start == end (one-shot placement), use sample's natural duration
    let clip_length_beats = {
        let requested = end_beat - start_beat;
        if requested <= 0.0 {
            // One-shot: use sample's natural length (at least 1 bar)
            let loop_bars = sample.loop_bars(bpm);
            loop_bars as f64 * beats_per_bar
        } else {
            requested
        }
    };
    
    // Recalculate end_beat if we adjusted clip_length for one-shots
    let end_beat = start_beat + clip_length_beats;

    let loop_bars = sample.loop_bars(bpm);
    let sample_loop_beats = loop_bars as f64 * beats_per_bar;

    // Cap loop to clip length - don't let sample loop beyond the clip boundary
    let loop_beats = if clip_length_beats < sample_loop_beats {
        clip_length_beats
    } else {
        sample_loop_beats
    };

    // WarpMarker tells Ableton: "at SecTime seconds into the sample, we should be at BeatTime beats"
    // SecTime = actual duration of audio in the file (based on ORIGINAL sample BPM)
    // BeatTime = where that audio should align in the project timeline
    // Ableton uses these two points to calculate stretch ratio
    //
    // Example: 125 BPM loop, 4 beats = 1.92 sec actual audio
    // We set SecTime=1.92, BeatTime=4 → Ableton stretches to match project tempo
    let sample_bpm = sample.bpm.unwrap_or(bpm); // Fall back to project BPM if unknown
    let warp_sec = (loop_beats * 60.0) / sample_bpm;

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
    let volume_value = if name == "KICK" { "1" } else { "0.251188643" };
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
        
        for &(start_bar, end_bar) in sections.iter() {
            let clip_id = ids.alloc();
            clips.push(create_audio_clip(sample, color, clip_id, start_bar + offset, end_bar + offset, bpm));
        }
    }

    let clips_xml = clips.join("\n");
    track = track.replacen(
        "<Events />",
        &format!("<Events>\n{}\n\t\t\t\t\t\t\t\t\t\t\t\t\t</Events>", clips_xml),
        1,
    );

    Ok(track)
}
