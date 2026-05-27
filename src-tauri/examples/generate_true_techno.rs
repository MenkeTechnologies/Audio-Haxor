//! Generate a TRUE techno arrangement with proper structure
//! - Pick 1-2 loops per track
//! - Place according to song structure (intro, build, breakdown, drop, outro)
//! - Elements enter/exit at correct bar positions

mod sample_filters;

use app_lib::als_generator::generate_empty_als;
use flate2::Compression;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use regex::Regex;
use rusqlite::Connection;
use sample_filters::BAD_GENRES;
use std::collections::HashSet;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use std::sync::atomic::{AtomicU32, Ordering};

const DB_PATH: &str =
    "/Users/wizard/Library/Application Support/com.menketechnologies.audio-haxor/audio_haxor.db";
const PROJECT_BPM: f64 = 128.0;

// Number of songs to generate in one ALS file
const NUM_SONGS: u32 = 4;
// Silence between songs (bars)
const GAP_BETWEEN_SONGS: u32 = 32;

// Arrangement structure (224 bars = 7 minutes at 128 BPM)
// All values in bars (1-indexed)
const SONG_LENGTH_BARS: u32 = 224;
const TOTAL_BARS: u32 = 224;

// Section boundaries (all 32 bars each)
const INTRO_START: u32 = 1;
const INTRO_END: u32 = 32; // 32 bars
const BUILD1_START: u32 = 33;
const BUILD1_END: u32 = 64; // 32 bars
const BREAKDOWN_START: u32 = 65;
const BREAKDOWN_END: u32 = 96; // 32 bars
const DROP1_START: u32 = 97;
const DROP1_END: u32 = 128; // 32 bars
const DROP2_START: u32 = 129;
const DROP2_END: u32 = 160; // 32 bars
const FADEDOWN_START: u32 = 161;
const FADEDOWN_END: u32 = 192; // 32 bars
const OUTRO_START: u32 = 193;
const OUTRO_END: u32 = 224; // 32 bars

// Element entry/exit positions (in bars, supports fractional for beat precision)
// 16.75 = bar 16, beat 4 (last beat of bar 16)
// 17.0 = bar 17, beat 1 (downbeat)
struct TrackArrangement {
    name: &'static str,
    sections: Vec<(f64, f64)>, // (start_bar, end_bar) pairs where element plays
}

// Offset all sections by a given number of bars (for multi-song generation)
fn offset_sections(sections: &[(f64, f64)], offset_bars: f64) -> Vec<(f64, f64)> {
    sections
        .iter()
        .map(|(start, end)| (start + offset_bars, end + offset_bars))
        .collect()
}

// All samples needed for one song
struct SongSamples {
    key: String,
    kick: Vec<SampleInfo>,
    clap: Vec<SampleInfo>,
    hat: Vec<SampleInfo>,
    hat2: Vec<SampleInfo>,
    perc: Vec<SampleInfo>,
    perc2: Vec<SampleInfo>,
    ride: Vec<SampleInfo>,
    bass: Vec<SampleInfo>,
    sub: Vec<SampleInfo>,
    main_synth: Vec<SampleInfo>,
    synth1: Vec<SampleInfo>,
    synth2: Vec<SampleInfo>,
    synth3: Vec<SampleInfo>,
    pad: Vec<SampleInfo>,
    pad2: Vec<SampleInfo>,
    arp: Vec<SampleInfo>,
    arp2: Vec<SampleInfo>,
    riser1: Vec<SampleInfo>,
    riser2: Vec<SampleInfo>,
    riser3: Vec<SampleInfo>,
    downlifter: Vec<SampleInfo>,
    crash: Vec<SampleInfo>,
    impact: Vec<SampleInfo>,
    hit: Vec<SampleInfo>,
    sweep_up: Vec<SampleInfo>,
    sweep_down: Vec<SampleInfo>,
    sweep_up2: Vec<SampleInfo>,
    sweep_down2: Vec<SampleInfo>,
    noise: Vec<SampleInfo>,
    noise2: Vec<SampleInfo>,
    snare_roll: Vec<SampleInfo>,
    fill_1a: Vec<SampleInfo>,
    fill_1b: Vec<SampleInfo>,
    fill_2a: Vec<SampleInfo>,
    fill_2b: Vec<SampleInfo>,
    fill_4a: Vec<SampleInfo>,
    fill_4b: Vec<SampleInfo>,
    fill_4c: Vec<SampleInfo>,
    fill_4d: Vec<SampleInfo>,
    reverse1: Vec<SampleInfo>,
    reverse2: Vec<SampleInfo>,
    sub_drop: Vec<SampleInfo>,
    atmos: Vec<SampleInfo>,
    atmos2: Vec<SampleInfo>,
    vox: Vec<SampleInfo>,
}

fn get_arrangement() -> Vec<TrackArrangement> {
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

    vec![
        // === DRUMS ===
        // KICK: gaps for varied fill lengths
        // Gap is the LAST bar/beats before a phrase boundary, fill plays IN the gap
        // 1 beat gap: last beat of bar 16, 56, 104, 136, 168, 216 (beat 4)
        // 2 beat gap: last 2 beats of bar 24, 40, 72, 88, 120, 152, 184, 208 (beats 3-4)
        // 4 beat gap: full bar 32, 48, 64, 80, 96, 112, 128, 144, 160, 176, 192
        TrackArrangement {
            name: "KICK",
            sections: vec![
                // INTRO (1-32) - gap at bar 16 (1 beat), bar 24 (2 beats), bar 32 (4 beats)
                (1.0, 16.75), // ends beat 4 of bar 16, gap is beat 4 (1 beat fill)
                (17.0, 24.5), // ends beat 3 of bar 24, gap is beats 3-4 (2 beat fill)
                (25.0, 32.0), // ends at bar 32, gap is bar 32 (4 beat fill)
                // BUILD (33-64) - gap at bar 40 (2 beats), bar 48 (4 beats), bar 56 (1 beat), bar 64 (4 beats)
                (33.0, 40.5),  // gap beats 3-4 of bar 40
                (41.0, 48.0),  // gap bar 48
                (49.0, 56.75), // gap beat 4 of bar 56
                (57.0, 64.0),  // gap bar 64
                // BREAKDOWN: kick OUT (65-96)
                // DROP 1 (97-128) - gap at 104 (1 beat), 112 (4 beats), 120 (2 beats), 128 (4 beats)
                (97.0, 104.75), // gap beat 4 of bar 104
                (105.0, 112.0), // gap bar 112
                (113.0, 120.5), // gap beats 3-4 of bar 120
                (121.0, 128.0), // gap bar 128
                // DROP 2 (129-160)
                (129.0, 136.75), // gap beat 4 of bar 136
                (137.0, 144.0),  // gap bar 144
                (145.0, 152.5),  // gap beats 3-4 of bar 152
                (153.0, 160.0),  // gap bar 160
                // FADEDOWN (161-192)
                (161.0, 168.75), // gap beat 4 of bar 168
                (169.0, 176.0),  // gap bar 176
                (177.0, 184.5),  // gap beats 3-4 of bar 184
                (185.0, 192.0),  // gap bar 192
                // OUTRO (193-224)
                (193.0, 208.5),  // gap beats 3-4 of bar 208
                (209.0, 216.75), // gap beat 4 of bar 216
                (217.0, 224.0),  // final phrase, no gap
            ],
        },
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
        TrackArrangement {
            name: "CLAP",
            sections: vec![
                // INTRO - gaps at bar 16 (1 beat), 24 (2 beats), 32 (4 beats)
                (9.0, 16.75), // ends beat 4 of bar 16
                (17.0, 24.5), // ends beat 3 of bar 24
                (25.0, 32.0), // ends at bar 32
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
                (185.0, 192.0), // drops at 193
            ],
        },
        // HAT: enters bar 17, gaps match KICK
        TrackArrangement {
            name: "HAT",
            sections: vec![
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
                (177.0, 184.0), // drops at 185
            ],
        },
        TrackArrangement {
            name: "HAT 2",
            sections: vec![
                (97.0, 104.75),
                (105.0, 112.0),
                (113.0, 120.5),
                (121.0, 128.0),
                (137.0, 144.0),
                (145.0, 152.5),
                (153.0, 160.0),
                (161.0, 168.75),
                (169.0, 176.0), // drops at 177
            ],
        },
        TrackArrangement {
            name: "PERC",
            sections: vec![
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
                (177.0, 184.0), // drops at 185
            ],
        },
        TrackArrangement {
            name: "PERC 2",
            sections: vec![
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
                (169.0, 176.0), // drops at 177
            ],
        },
        TrackArrangement {
            name: "RIDE",
            sections: vec![
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
                (169.0, 176.0), // drops at 177
            ],
        },
        // === BASS ===
        // BASS: enters bar 33, gaps match drums
        TrackArrangement {
            name: "BASS",
            sections: vec![
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
                (193.0, 200.0), // drops at 201
            ],
        },
        // SUB: gaps match bass
        TrackArrangement {
            name: "SUB",
            sections: vec![
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
                (161.0, 168.75), // drops at 169
            ],
        },
        // === MELODICS (all with fill gaps) ===
        // MAIN SYNTH - the lead, introduced mid-breakdown (bar 81), explodes in drop
        TrackArrangement {
            name: "MAIN SYNTH",
            sections: vec![
                (81.0, 88.5), // mid-breakdown, gap at 88 (2 beats)
                (89.0, 96.0), // gap at 96 (4 beats)
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
            ],
        },
        TrackArrangement {
            name: "SYNTH 1",
            sections: vec![
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
                (169.0, 176.0), // drops at 177
            ],
        },
        TrackArrangement {
            name: "SYNTH 2",
            sections: vec![
                (105.0, 112.0),
                (113.0, 120.5),
                (121.0, 128.0),
                (129.0, 136.75),
                (137.0, 144.0),
                (145.0, 152.5),
                (153.0, 160.0),
                (161.0, 168.75), // drops at 169
            ],
        },
        TrackArrangement {
            name: "SYNTH 3",
            sections: vec![
                (113.0, 120.5),
                (121.0, 128.0),
                (145.0, 152.5),
                (153.0, 160.0),
                (161.0, 168.75), // drops at 169
            ],
        },
        TrackArrangement {
            name: "PAD",
            sections: vec![
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
                (169.0, 176.0), // drops at 177
            ],
        },
        TrackArrangement {
            name: "PAD 2",
            sections: vec![(81.0, 88.5), (89.0, 96.0)],
        },
        TrackArrangement {
            name: "ARP",
            sections: vec![
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
                (161.0, 168.75), // drops at 169
            ],
        },
        TrackArrangement {
            name: "ARP 2",
            sections: vec![
                (121.0, 128.0),
                (137.0, 144.0),
                (145.0, 152.5),
                (153.0, 160.0),
                (161.0, 168.75), // drops at 169
            ],
        },
        // === FX - RISERS (CONTINUE THROUGH FILL GAPS for seamless tension) ===
        TrackArrangement {
            name: "RISER 1", // main long risers (8 bars) - through fill gaps
            sections: vec![
                (25.0, 33.0),   // pre-build (through fill gap into build)
                (57.0, 65.0),   // pre-breakdown (through fill gap)
                (89.0, 97.0),   // PRE-DROP 1 - the big one! (through to drop)
                (121.0, 129.0), // mid drop 1 (through fill gap)
                (153.0, 161.0), // pre-fadedown (through fill gap)
                (185.0, 193.0), // pre-outro (through fill gap)
            ],
        },
        TrackArrangement {
            name: "RISER 2", // secondary risers (different sample) - through fill gaps
            sections: vec![
                (9.0, 17.0),    // early intro tension (through fill gap)
                (41.0, 49.0),   // mid build (through fill gap)
                (89.0, 97.0),   // PRE-DROP 1 - layer (through to drop)
                (137.0, 145.0), // mid drop 2 (through fill gap)
                (177.0, 185.0), // fadedown tension (through fill gap)
            ],
        },
        TrackArrangement {
            name: "RISER 3", // short accent risers (4 bars) - through fill gaps
            sections: vec![
                (13.0, 17.0),   // intro accent (through fill gap)
                (29.0, 33.0),   // pre-build accent (through fill gap)
                (45.0, 49.0),   // build accent (through fill gap)
                (61.0, 65.0),   // pre-breakdown (through fill gap)
                (77.0, 81.0),   // breakdown tension (through fill gap)
                (93.0, 97.0),   // PRE-DROP final 4 (through to drop)
                (109.0, 113.0), // drop 1 accent (through fill gap)
                (125.0, 129.0), // end drop 1 (through fill gap)
                (141.0, 145.0), // drop 2 accent (through fill gap)
                (157.0, 161.0), // end drop 2 (through fill gap)
                (173.0, 177.0), // fadedown accent (through fill gap)
                (189.0, 193.0), // pre-outro (through fill gap)
            ],
        },
        // === FX - SNARE ROLLS (critical for tension!) ===
        TrackArrangement {
            name: "SNARE ROLL", // tension builder - CONTINUES THROUGH FILL GAPS
            sections: vec![
                (29.0, 33.0),   // pre-build (through fill gap into build)
                (61.0, 65.0),   // pre-breakdown (through fill gap)
                (89.0, 97.0),   // PRE-DROP 1 - full roll into the drop!
                (125.0, 129.0), // end drop 1 (through fill gap)
                (153.0, 161.0), // pre-fadedown (through fill gap)
                (189.0, 193.0), // pre-outro (through fill gap)
            ],
        },
        // === FX - DRUM FILLS (4 different samples, staggered) ===
        // Pattern: FILL A, B, C, D rotate through positions for variety
        // 1 BEAT fills - 2 different samples alternating
        TrackArrangement {
            name: "FILL 1A",
            sections: vec![
                (16.75, 17.0),   // bar 16
                (104.75, 105.0), // bar 104
                (168.75, 169.0), // bar 168
            ],
        },
        TrackArrangement {
            name: "FILL 1B",
            sections: vec![
                (56.75, 57.0),   // bar 56
                (136.75, 137.0), // bar 136
                (216.75, 217.0), // bar 216
            ],
        },
        // 2 BEAT fills - 2 different samples alternating
        TrackArrangement {
            name: "FILL 2A",
            sections: vec![
                (24.5, 25.0),   // bar 24
                (72.5, 73.0),   // bar 72
                (120.5, 121.0), // bar 120
                (184.5, 185.0), // bar 184
            ],
        },
        TrackArrangement {
            name: "FILL 2B",
            sections: vec![
                (40.5, 41.0),   // bar 40
                (88.5, 89.0),   // bar 88
                (152.5, 153.0), // bar 152
                (208.5, 209.0), // bar 208
            ],
        },
        // 4 BEAT fills - 4 different samples rotating: A, B, C, D, A, B, C, D...
        TrackArrangement {
            name: "FILL 4A",
            sections: vec![
                (32.0, 33.0),   // bar 32, into build
                (96.0, 97.0),   // bar 96, INTO DROP 1 - biggest!
                (160.0, 161.0), // bar 160, into fadedown
            ],
        },
        TrackArrangement {
            name: "FILL 4B",
            sections: vec![
                (48.0, 49.0),   // bar 48, mid build
                (112.0, 113.0), // bar 112, mid drop 1
                (176.0, 177.0), // bar 176, mid fadedown
            ],
        },
        TrackArrangement {
            name: "FILL 4C",
            sections: vec![
                (64.0, 65.0),   // bar 64, into breakdown
                (128.0, 129.0), // bar 128, into drop 2
                (192.0, 193.0), // bar 192, into outro
            ],
        },
        TrackArrangement {
            name: "FILL 4D",
            sections: vec![
                (80.0, 81.0),   // bar 80, mid breakdown
                (144.0, 145.0), // bar 144, mid drop 2
            ],
        },
        // === FX - REVERSE CRASHES (2 samples alternating) ===
        TrackArrangement {
            name: "REVERSE 1",
            sections: vec![
                (16.0, 17.0),   // bar 16
                (48.0, 49.0),   // bar 48
                (80.0, 81.0),   // bar 80
                (112.0, 113.0), // bar 112
                (144.0, 145.0), // bar 144
                (176.0, 177.0), // bar 176
            ],
        },
        TrackArrangement {
            name: "REVERSE 2",
            sections: vec![
                (32.0, 33.0),   // bar 32, into build
                (64.0, 65.0),   // bar 64, into breakdown
                (96.0, 97.0),   // bar 96, INTO DROP 1
                (128.0, 129.0), // bar 128, into drop 2
                (160.0, 161.0), // bar 160, into fadedown
                (192.0, 193.0), // bar 192, into outro
            ],
        },
        // === FX - SUB DROP (impact on big moments) ===
        TrackArrangement {
            name: "SUB DROP",
            sections: vec![
                (33.0, 33.0),   // build start
                (65.0, 65.0),   // breakdown start
                (97.0, 97.0),   // DROP 1 hit
                (129.0, 129.0), // DROP 2 hit
                (161.0, 161.0), // fadedown start
            ],
        },
        // === FX - DOWNLIFTERS ===
        TrackArrangement {
            name: "DOWNLIFTER",
            sections: vec![
                (33.0, 40.0),   // build start (energy down then up)
                (65.0, 72.0),   // into breakdown
                (97.0, 104.0),  // post-drop settle
                (129.0, 136.0), // post-drop 2
                (161.0, 168.0), // into fadedown
                (193.0, 200.0), // into outro
            ],
        },
        // === FX - IMPACTS/CRASHES (more frequent) ===
        TrackArrangement {
            name: "CRASH",
            sections: vec![
                // Every 8 bars for energy
                (1.0, 1.0),
                (9.0, 9.0),
                (17.0, 17.0),
                (25.0, 25.0),
                (33.0, 33.0),
                (41.0, 41.0),
                (49.0, 49.0),
                (57.0, 57.0),
                (65.0, 65.0),
                (73.0, 73.0),
                (81.0, 81.0),
                (89.0, 89.0),
                (97.0, 97.0),
                (105.0, 105.0),
                (113.0, 113.0),
                (121.0, 121.0),
                (129.0, 129.0),
                (137.0, 137.0),
                (145.0, 145.0),
                (153.0, 153.0),
                (161.0, 161.0),
                (169.0, 169.0),
                (177.0, 177.0),
                (185.0, 185.0),
                (193.0, 193.0),
                (201.0, 201.0),
                (209.0, 209.0),
                (217.0, 217.0),
            ],
        },
        TrackArrangement {
            name: "IMPACT",
            sections: vec![
                (1.0, 1.0),     // track start
                (33.0, 33.0),   // build start
                (65.0, 65.0),   // breakdown
                (97.0, 97.0),   // DROP 1
                (129.0, 129.0), // DROP 2
                (161.0, 161.0), // fadedown
                (193.0, 193.0), // outro
            ],
        },
        TrackArrangement {
            name: "HIT", // accent hits
            sections: vec![
                // Offbeat hits for groove
                (5.0, 5.0),
                (13.0, 13.0),
                (21.0, 21.0),
                (29.0, 29.0),
                (37.0, 37.0),
                (45.0, 45.0),
                (53.0, 53.0),
                (61.0, 61.0),
                (69.0, 69.0),
                (77.0, 77.0),
                (85.0, 85.0),
                (93.0, 93.0),
                (101.0, 101.0),
                (109.0, 109.0),
                (117.0, 117.0),
                (125.0, 125.0),
                (133.0, 133.0),
                (141.0, 141.0),
                (149.0, 149.0),
                (157.0, 157.0),
                (165.0, 165.0),
                (173.0, 173.0),
                (181.0, 181.0),
                (189.0, 189.0),
                (197.0, 197.0),
                (205.0, 205.0),
                (213.0, 213.0),
                (221.0, 221.0),
            ],
        },
        // === FX - SWEEPS (CONTINUE THROUGH FILL GAPS) ===
        TrackArrangement {
            name: "SWEEP UP",
            sections: vec![
                // Short sweeps (2 bars) - extend through gaps
                (7.0, 9.0), // intro
                (15.0, 17.0),
                (23.0, 25.0),
                // Medium sweeps (4 bars) - extend through gaps
                (29.0, 33.0), // pre-build (through fill gap)
                (45.0, 49.0),
                (61.0, 65.0), // pre-breakdown (through fill gap)
                (77.0, 81.0),
                // Long sweep before drop (through fill gap)
                (89.0, 97.0), // PRE-DROP 1 - 8 bar sweep!
                // More throughout (through fill gaps)
                (109.0, 113.0),
                (125.0, 129.0),
                (141.0, 145.0),
                (157.0, 161.0),
                (173.0, 177.0),
                (185.0, 193.0), // pre-outro (through fill gap)
            ],
        },
        TrackArrangement {
            name: "SWEEP DOWN",
            sections: vec![
                // After every major hit, sweep down - no gaps needed
                (1.0, 4.0), // track start
                (17.0, 20.0),
                (33.0, 40.0), // build start
                (49.0, 56.0),
                (65.0, 80.0), // breakdown start - long
                (81.0, 88.0),
                (97.0, 108.0), // post-drop 1
                (113.0, 120.0),
                (129.0, 140.0), // post-drop 2
                (145.0, 152.0),
                (161.0, 172.0), // fadedown
                (177.0, 184.0),
                (193.0, 204.0), // outro
                (209.0, 216.0),
            ],
        },
        TrackArrangement {
            name: "SWEEP UP 2", // second sweep layer - THROUGH FILL GAPS
            sections: vec![
                (13.0, 17.0),
                (29.0, 33.0),
                (53.0, 57.0),
                (61.0, 65.0),
                (89.0, 97.0), // layer on pre-drop (through fill gap)
                (121.0, 129.0),
                (153.0, 161.0),
                (185.0, 193.0),
            ],
        },
        TrackArrangement {
            name: "SWEEP DOWN 2", // second down sweep
            sections: vec![
                (17.0, 25.0),
                (65.0, 81.0),   // long breakdown sweep
                (97.0, 113.0),  // post-drop decay
                (161.0, 177.0), // fadedown atmosphere
            ],
        },
        // === FX - NOISE (white noise - CONTINUES THROUGH FILL GAPS) ===
        TrackArrangement {
            name: "NOISE",
            sections: vec![
                (9.0, 17.0),    // intro texture (through fill gap)
                (25.0, 33.0),   // pre-build (through fill gap)
                (41.0, 49.0),   // build tension (through fill gap)
                (57.0, 65.0),   // pre-breakdown (through fill gap)
                (73.0, 81.0),   // breakdown texture (through fill gap)
                (89.0, 97.0),   // PRE-DROP 1 - full noise (through fill gap)
                (105.0, 113.0), // drop 1 texture (through fill gap)
                (121.0, 129.0), // end drop 1 (through fill gap)
                (137.0, 145.0), // drop 2 texture (through fill gap)
                (153.0, 161.0), // pre-fadedown (through fill gap)
                (169.0, 177.0), // fadedown texture (through fill gap)
                (185.0, 193.0), // pre-outro (through fill gap)
            ],
        },
        TrackArrangement {
            name: "NOISE 2", // second noise layer - CONTINUES THROUGH FILL GAPS
            sections: vec![
                (29.0, 33.0),   // build accent (through fill gap)
                (61.0, 65.0),   // pre-breakdown accent (through fill gap)
                (89.0, 97.0),   // pre-drop layer (through fill gap)
                (125.0, 129.0), // end drop 1 (through fill gap)
                (157.0, 161.0), // end drop 2 (through fill gap)
                (189.0, 193.0), // pre-outro (through fill gap)
            ],
        },
        // === ATMOSPHERE ===
        TrackArrangement {
            name: "ATMOS",
            sections: vec![
                (1.0, 64.0),
                (65.0, 96.0),
                (97.0, 224.0), // through outro
            ],
        },
        TrackArrangement {
            name: "ATMOS 2",
            sections: vec![(65.0, 96.0), (129.0, 160.0)],
        },
        TrackArrangement {
            name: "VOX",
            sections: vec![(81.0, 96.0), (113.0, 128.0), (145.0, 160.0)],
        },
    ]
}

fn read_wav_duration(path: &str) -> Option<f64> {
    use std::io::{Read, Seek, SeekFrom};

    let mut file = File::open(path).ok()?;
    let mut header = [0u8; 44];
    file.read_exact(&mut header).ok()?;

    if &header[0..4] != b"RIFF" || &header[8..12] != b"WAVE" {
        return None;
    }

    file.seek(SeekFrom::Start(12)).ok()?;

    let mut chunk_header = [0u8; 8];
    let mut sample_rate: u32 = 0;
    let mut bits_per_sample: u16 = 0;
    let mut channels: u16 = 0;
    let mut data_size: u32 = 0;

    loop {
        if file.read_exact(&mut chunk_header).is_err() {
            break;
        }

        let chunk_id = &chunk_header[0..4];
        let chunk_size = u32::from_le_bytes([
            chunk_header[4],
            chunk_header[5],
            chunk_header[6],
            chunk_header[7],
        ]);

        if chunk_id == b"fmt " {
            let mut fmt_data = vec![0u8; chunk_size as usize];
            file.read_exact(&mut fmt_data).ok()?;

            channels = u16::from_le_bytes([fmt_data[2], fmt_data[3]]);
            sample_rate = u32::from_le_bytes([fmt_data[4], fmt_data[5], fmt_data[6], fmt_data[7]]);
            bits_per_sample = u16::from_le_bytes([fmt_data[14], fmt_data[15]]);
        } else if chunk_id == b"data" {
            data_size = chunk_size;
            break;
        } else {
            file.seek(SeekFrom::Current(chunk_size as i64)).ok()?;
        }
    }

    if sample_rate == 0 || channels == 0 || bits_per_sample == 0 || data_size == 0 {
        return None;
    }

    let bytes_per_sample = (bits_per_sample / 8) as u32;
    let total_samples = data_size / (bytes_per_sample * channels as u32);
    let duration = total_samples as f64 / sample_rate as f64;

    Some(duration)
}

const GROUP_TRACK_TEMPLATE: &str = include_str!("../src/group_track_template.xml");

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

fn main() {
    let output_path = Path::new("/Users/wizard/Desktop/True_Techno.als");

    match generate_true_techno(output_path) {
        Ok(()) => {
            println!("Generated: {}", output_path.display());
            println!("\nTrue Techno Arrangement (224 bars / 7 min at 128 BPM)");
            println!("28 TRACKS with full FX complement\n");
            println!("DRUMS: KICK, CLAP, HAT, HAT 2, PERC, PERC 2, RIDE");
            println!("BASS:  BASS, SUB");
            println!("MELODICS: SYNTH 1-3, PAD 1-2, ARP");
            println!("FX: RISER 1-3, DOWNLIFTER, CRASH, IMPACT, HIT");
            println!("    SWEEP UP, SWEEP DOWN, NOISE, ATMOS 1-2, VOX\n");
            println!("Crashes every 16 bars, hits every 8 bars");
            println!("Risers/sweeps at all transitions");
            println!("\nElement entry/exit:");
            for arr in get_arrangement() {
                let sections: String = arr
                    .sections
                    .iter()
                    .map(|(s, e)| {
                        if s == e {
                            format!("{}", s)
                        } else {
                            format!("{}-{}", s, e)
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                println!("  {:<12} bars: {}", arr.name, sections);
            }
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
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
    fn from_db(path: &str, db_duration: f64, bpm: Option<f64>) -> Result<Self, String> {
        let metadata =
            std::fs::metadata(path).map_err(|e| format!("Cannot read {}: {}", path, e))?;
        let name = Path::new(path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("sample")
            .to_string();

        let duration_secs = read_wav_duration(path).unwrap_or(db_duration);

        Ok(Self {
            path: path.to_string(),
            name,
            file_size: metadata.len(),
            duration_secs,
            bpm,
        })
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
        if bars <= 0.75 {
            1
        } else if bars <= 1.5 {
            1
        } else if bars <= 3.0 {
            2
        } else if bars <= 6.0 {
            4
        } else if bars <= 12.0 {
            8
        } else {
            16
        }
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

fn pick_random_key() -> String {
    let conn = match Connection::open(DB_PATH) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Warning: Cannot open DB for key selection: {}", e);
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

    conn.query_row(query, [], |row| row.get(0))
        .unwrap_or_else(|_| "A Minor".to_string())
}

fn query_samples(
    include_patterns: &[&str],
    exclude_patterns: Vec<&str>,
    require_loop: bool,
    count: usize,
) -> Vec<SampleInfo> {
    query_samples_with_key(
        include_patterns,
        exclude_patterns,
        require_loop,
        count,
        None,
    )
}

fn query_samples_with_key(
    include_patterns: &[&str],
    exclude_patterns: Vec<&str>,
    require_loop: bool,
    count: usize,
    key: Option<&str>,
) -> Vec<SampleInfo> {
    // Try with key first
    let results = query_samples_internal(
        include_patterns,
        exclude_patterns.clone(),
        require_loop,
        count,
        key,
    );

    // If no results with key, fallback to no key filter
    if results.is_empty() && key.is_some() {
        eprintln!("  (no samples in key, falling back to any key)");
        return query_samples_internal(
            include_patterns,
            exclude_patterns,
            require_loop,
            count,
            None,
        );
    }

    results
}

fn query_samples_internal(
    include_patterns: &[&str],
    exclude_patterns: Vec<&str>,
    require_loop: bool,
    count: usize,
    key: Option<&str>,
) -> Vec<SampleInfo> {
    let conn = match Connection::open(DB_PATH) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Warning: Cannot open DB: {}", e);
            return vec![];
        }
    };

    let include_clause: String = include_patterns
        .iter()
        .flat_map(|p| {
            vec![
                format!("al.path LIKE '%{}%'", p.to_lowercase()),
                format!("al.path LIKE '%{}%'", p),
            ]
        })
        .collect::<Vec<_>>()
        .join(" OR ");

    let exclude_clause: String = exclude_patterns
        .iter()
        .map(|p| {
            format!(
                "AND al.path NOT LIKE '%{}%' AND al.path NOT LIKE '%{}%'",
                p.to_lowercase(),
                p
            )
        })
        .collect::<Vec<_>>()
        .join(" ");

    let loop_clause = if require_loop {
        "AND al.path LIKE '%loop%'"
    } else {
        ""
    };

    let key_clause = match key {
        Some(k) => format!("AND s.key_name = '{}'", k),
        None => String::new(),
    };

    let query = format!(
        "SELECT al.path, COALESCE(s.duration, 0), s.bpm 
         FROM audio_library al 
         JOIN audio_samples s ON al.sample_id = s.id 
         WHERE s.format = 'WAV' 
         AND ({})
         {}
         {}
         AND al.path LIKE '%MusicProduction/Samples%'
         AND al.path NOT LIKE '%Splice%'
         {}
         ORDER BY RANDOM() 
         LIMIT {}",
        include_clause,
        loop_clause,
        key_clause,
        exclude_clause,
        count * 2
    );

    let mut stmt = match conn.prepare(&query) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Warning: Query error: {}", e);
            return vec![];
        }
    };

    stmt.query_map([], |row| {
        let path: String = row.get(0)?;
        let duration: f64 = row.get(1)?;
        let bpm: Option<f64> = row.get(2)?;
        Ok((path, duration, bpm))
    })
    .ok()
    .map(|rows| {
        rows.filter_map(|r| r.ok())
            .filter_map(|(path, duration, bpm)| {
                if !Path::new(&path).exists() {
                    return None;
                }
                SampleInfo::from_db(&path, duration, bpm).ok()
            })
            .take(count)
            .collect()
    })
    .unwrap_or_default()
}

// Section locators for arrangement navigation (for one song)
fn get_song_locators() -> Vec<(&'static str, u32)> {
    vec![
        ("INTRO", INTRO_START),         // 1
        ("BUILD", BUILD1_START),        // 33
        ("BREAKDOWN", BREAKDOWN_START), // 65
        ("DROP 1", DROP1_START),        // 97
        ("DROP 2", DROP2_START),        // 129
        ("FADEDOWN", FADEDOWN_START),   // 161
        ("OUTRO", OUTRO_START),         // 193
    ]
}

fn create_locators_xml_multi(ids: &IdAllocator, num_songs: u32, song_keys: &[String]) -> String {
    let mut locators: Vec<String> = Vec::new();
    let bars_per_song = SONG_LENGTH_BARS + GAP_BETWEEN_SONGS;

    for song_idx in 0..num_songs {
        let offset = song_idx * bars_per_song;
        let key = song_keys
            .get(song_idx as usize)
            .map(|s| s.as_str())
            .unwrap_or("?");

        // Add song start marker with key
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
            song_start_id,
            song_start_beat,
            song_idx + 1,
            key
        ));

        // Add section markers for this song
        for (name, bar) in get_song_locators() {
            let id = ids.alloc();
            let time_beats = (bar - 1 + offset) * 4; // bar 1 = beat 0
            locators.push(format!(
                r#"<Locator Id="{}">
					<LomId Value="0" />
					<Time Value="{}" />
					<Name Value="{} {}" />
					<Annotation Value="" />
					<IsSongStart Value="false" />
				</Locator>"#,
                id,
                time_beats,
                song_idx + 1,
                name
            ));
        }
    }

    format!(
        "<Locators>\n\t\t\t{}\n\t\t</Locators>",
        locators.join("\n\t\t\t")
    )
}

fn load_song_samples(song_num: u32) -> SongSamples {
    eprintln!("\n=== Loading samples for SONG {} ===", song_num);

    let track_key = pick_random_key();
    eprintln!("*** SONG {} KEY: {} ***\n", song_num, track_key);

    // DRUMS - no key needed
    eprintln!("KICK:");
    let kick = query_samples(
        &["kick_loop", "kick"],
        [
            BAD_GENRES,
            &[
                "no_kick",
                "no kick",
                "nokick",
                "without_kick",
                "without kick",
                "snare",
                "bass",
                "sub",
                "synth",
                "melody",
                "lead",
                "pad",
                "arp",
                "chord",
            ],
        ]
        .concat(),
        true,
        1,
    );
    eprintln!("CLAP:");
    let clap = query_samples(
        &["clap_loop", "clap", "snare_loop", "snare"],
        [
            BAD_GENRES,
            &[
                "kick", "bass", "sub", "synth", "melody", "lead", "pad", "arp", "chord", "roll",
                "fill",
            ],
        ]
        .concat(),
        true,
        1,
    );
    eprintln!("HAT:");
    let hat = query_samples(
        &["hat_loop", "hihat_loop", "hat", "hihat", "closed_hat"],
        [
            BAD_GENRES,
            &[
                "open", "ride", "bass", "sub", "synth", "melody", "lead", "pad", "arp", "chord",
                "kick",
            ],
        ]
        .concat(),
        true,
        1,
    );
    eprintln!("HAT 2:");
    let hat2 = query_samples(
        &["open_hat", "ohat", "hat_open"],
        [
            BAD_GENRES,
            &[
                "closed", "bass", "sub", "synth", "melody", "lead", "pad", "arp", "chord", "kick",
            ],
        ]
        .concat(),
        true,
        1,
    );
    eprintln!("PERC:");
    let perc = query_samples(
        &["perc_loop", "percussion_loop", "perc"],
        [
            BAD_GENRES,
            &[
                "kick",
                "snare",
                "hat",
                "bass",
                "sub",
                "synth",
                "melody",
                "lead",
                "pad",
                "arp",
                "chord",
                "full",
                "drums_full",
                "kit",
            ],
        ]
        .concat(),
        true,
        1,
    );
    eprintln!("PERC 2:");
    let perc2 = query_samples(
        &["tom_loop", "tom", "perc"],
        [
            BAD_GENRES,
            &[
                "kick", "snare", "bass", "sub", "synth", "melody", "lead", "pad", "arp", "chord",
                "full", "kit",
            ],
        ]
        .concat(),
        true,
        1,
    );
    eprintln!("RIDE:");
    let ride = query_samples(
        &["ride_loop", "ride", "cymbal_loop"],
        [
            BAD_GENRES,
            &[
                "crash", "hit", "bass", "sub", "synth", "melody", "lead", "pad", "arp", "chord",
                "kick",
            ],
        ]
        .concat(),
        true,
        1,
    );

    // BASS - MATCH KEY
    eprintln!("BASS (key: {}):", track_key);
    let bass = query_samples_with_key(
        &["bass_loop", "bassline", "bass"],
        [
            BAD_GENRES,
            &[
                "kick", "sub", "drum", "drums", "hat", "snare", "clap", "perc", "ride", "cymbal",
                "tom", "full", "kit", "synth", "lead", "pad", "arp", "melody",
            ],
        ]
        .concat(),
        true,
        1,
        Some(&track_key),
    );
    eprintln!("SUB (key: {}):", track_key);
    let sub = query_samples_with_key(
        &["sub_loop", "sub_bass", "808_loop", "sub"],
        [
            BAD_GENRES,
            &[
                "kick", "drum", "drums", "hat", "snare", "clap", "perc", "ride", "full", "kit",
                "synth", "lead", "pad", "arp", "melody",
            ],
        ]
        .concat(),
        true,
        1,
        Some(&track_key),
    );

    // MELODICS - MATCH KEY
    eprintln!("MAIN SYNTH (key: {}):", track_key);
    let main_synth = query_samples_with_key(
        &["lead_loop", "synth_lead", "lead"],
        [
            BAD_GENRES,
            &[
                "pad", "bass", "sub", "drum", "drums", "kick", "hat", "snare", "clap", "perc",
                "ride", "full", "kit", "arp",
            ],
        ]
        .concat(),
        true,
        1,
        Some(&track_key),
    );
    eprintln!("SYNTH 1 (key: {}):", track_key);
    let synth1 = query_samples_with_key(
        &["synth_loop", "acid_loop", "synth"],
        [
            BAD_GENRES,
            &[
                "pad", "lead", "bass", "sub", "drum", "drums", "kick", "hat", "snare", "clap",
                "perc", "ride", "full", "kit",
            ],
        ]
        .concat(),
        true,
        1,
        Some(&track_key),
    );
    eprintln!("SYNTH 2 (key: {}):", track_key);
    let synth2 = query_samples_with_key(
        &["melody_loop", "synth_melody", "melody"],
        [
            BAD_GENRES,
            &[
                "pad", "bass", "sub", "drum", "drums", "kick", "hat", "snare", "clap", "perc",
                "ride", "full", "kit",
            ],
        ]
        .concat(),
        true,
        1,
        Some(&track_key),
    );
    eprintln!("SYNTH 3 (stabs, key: {}):", track_key);
    let synth3 = query_samples_with_key(
        &["stab", "synth_shot", "chord_stab"],
        [
            BAD_GENRES,
            &[
                "pad", "bass", "sub", "drum", "drums", "kick", "hat", "snare", "clap", "perc",
                "ride", "full", "kit", "loop",
            ],
        ]
        .concat(),
        false,
        1,
        Some(&track_key),
    );
    eprintln!("PAD (key: {}):", track_key);
    let pad = query_samples_with_key(
        &["pad_loop", "pad"],
        [
            BAD_GENRES,
            &[
                "drum", "drums", "stab", "bass", "sub", "kick", "hat", "snare", "clap", "perc",
                "ride", "full", "kit", "lead", "arp",
            ],
        ]
        .concat(),
        true,
        1,
        Some(&track_key),
    );
    eprintln!("PAD 2 (key: {}):", track_key);
    let pad2 = query_samples_with_key(
        &["drone_loop", "atmosphere_loop", "drone", "atmosphere"],
        [
            BAD_GENRES,
            &[
                "drum", "drums", "kick", "stab", "bass", "sub", "hat", "snare", "clap", "perc",
                "ride", "full", "kit", "lead", "arp",
            ],
        ]
        .concat(),
        true,
        1,
        Some(&track_key),
    );
    eprintln!("ARP (key: {}):", track_key);
    let arp = query_samples_with_key(
        &["arp_loop", "arpegg", "arp"],
        [
            BAD_GENRES,
            &[
                "pad", "drum", "drums", "bass", "sub", "kick", "hat", "snare", "clap", "perc",
                "ride", "full", "kit", "lead",
            ],
        ]
        .concat(),
        true,
        1,
        Some(&track_key),
    );
    eprintln!("ARP 2 (key: {}):", track_key);
    let arp2 = query_samples_with_key(
        &["pluck_loop", "sequence_loop", "pluck"],
        [
            BAD_GENRES,
            &[
                "pad", "drum", "drums", "chord", "bass", "sub", "kick", "hat", "snare", "clap",
                "perc", "ride", "full", "kit", "lead",
            ],
        ]
        .concat(),
        true,
        1,
        Some(&track_key),
    );

    // FX - RISERS
    eprintln!("RISER 1:");
    let riser1 = query_samples(
        &["riser", "uplifter"],
        [BAD_GENRES, &["down", "impact"]].concat(),
        false,
        1,
    );
    eprintln!("RISER 2:");
    let riser2 = query_samples(
        &["build", "riser", "tension"],
        [BAD_GENRES, &["down", "impact"]].concat(),
        false,
        1,
    );
    eprintln!("RISER 3:");
    let riser3 = query_samples(
        &["whoosh", "sweep_up", "upsweep"],
        [BAD_GENRES, &["down"]].concat(),
        false,
        1,
    );

    // FX - DOWNLIFTERS
    eprintln!("DOWNLIFTER:");
    let downlifter = query_samples(
        &["downlifter", "downsweep", "down_sweep", "fall"],
        [BAD_GENRES, &["up", "riser"]].concat(),
        false,
        1,
    );

    // FX - IMPACTS
    eprintln!("CRASH:");
    let crash = query_samples(
        &["crash", "cymbal_crash"],
        [BAD_GENRES, &["loop", "ride"]].concat(),
        false,
        1,
    );
    eprintln!("IMPACT:");
    let impact = query_samples(
        &["impact", "boom", "thud"],
        [BAD_GENRES, &["loop", "riser"]].concat(),
        false,
        1,
    );
    eprintln!("HIT:");
    let hit = query_samples(
        &["hit", "fx_hit", "perc_shot"],
        [BAD_GENRES, &["loop", "riser", "crash"]].concat(),
        false,
        1,
    );

    // FX - SWEEPS
    eprintln!("SWEEP UP:");
    let sweep_up = query_samples(
        &["sweep_up", "upsweep", "white_noise_up"],
        [BAD_GENRES, &["down"]].concat(),
        false,
        1,
    );
    eprintln!("SWEEP DOWN:");
    let sweep_down = query_samples(
        &["sweep_down", "downsweep", "white_noise_down"],
        [BAD_GENRES, &["up"]].concat(),
        false,
        1,
    );
    eprintln!("SWEEP UP 2:");
    let sweep_up2 = query_samples(
        &["sweep", "riser", "build"],
        [BAD_GENRES, &["down", "impact"]].concat(),
        false,
        1,
    );
    eprintln!("SWEEP DOWN 2:");
    let sweep_down2 = query_samples(
        &["sweep", "down", "fall"],
        [BAD_GENRES, &["up", "riser"]].concat(),
        false,
        1,
    );

    // FX - NOISE
    eprintln!("NOISE:");
    let noise = query_samples(
        &["noise", "white_noise", "texture"],
        [BAD_GENRES, &["drum", "kick"]].concat(),
        false,
        1,
    );
    eprintln!("NOISE 2:");
    let noise2 = query_samples(
        &["hiss", "static", "noise"],
        [BAD_GENRES, &["drum", "kick"]].concat(),
        false,
        1,
    );

    // FX - SNARE ROLL
    eprintln!("SNARE ROLL:");
    let snare_roll = query_samples(
        &["snare_roll", "buildup", "roll"],
        [BAD_GENRES, &["bass", "synth", "pad", "melody"]].concat(),
        false,
        1,
    );

    // FX - FILLS (drum fills only)
    eprintln!("FILL 1A:");
    let fill_1a = query_samples(
        &["fill", "drum_fill", "perc_hit", "snare_hit"],
        [
            BAD_GENRES,
            &[
                "bass", "synth", "pad", "lead", "melody", "loop", "full", "8bar", "4bar", "chord",
            ],
        ]
        .concat(),
        false,
        1,
    );
    eprintln!("FILL 1B:");
    let fill_1b = query_samples(
        &["fill", "tom_hit", "drum_hit"],
        [
            BAD_GENRES,
            &[
                "bass", "synth", "pad", "lead", "melody", "loop", "full", "8bar", "4bar", "chord",
            ],
        ]
        .concat(),
        false,
        1,
    );
    eprintln!("FILL 2A:");
    let fill_2a = query_samples(
        &["fill", "drum_fill"],
        [
            BAD_GENRES,
            &[
                "bass", "synth", "pad", "lead", "melody", "loop", "full", "8bar", "4bar", "chord",
            ],
        ]
        .concat(),
        false,
        1,
    );
    eprintln!("FILL 2B:");
    let fill_2b = query_samples(
        &["fill", "break", "drum_break"],
        [
            BAD_GENRES,
            &[
                "bass", "synth", "pad", "lead", "melody", "loop", "full", "8bar", "4bar", "chord",
            ],
        ]
        .concat(),
        false,
        1,
    );
    eprintln!("FILL 4A:");
    let fill_4a = query_samples(
        &["fill", "drum_fill", "break"],
        [
            BAD_GENRES,
            &[
                "bass", "synth", "pad", "lead", "melody", "loop", "full", "8bar", "4bar", "chord",
            ],
        ]
        .concat(),
        false,
        1,
    );
    eprintln!("FILL 4B:");
    let fill_4b = query_samples(
        &["fill", "tom_fill"],
        [
            BAD_GENRES,
            &[
                "bass", "synth", "pad", "lead", "melody", "loop", "full", "8bar", "4bar", "chord",
            ],
        ]
        .concat(),
        false,
        1,
    );
    eprintln!("FILL 4C:");
    let fill_4c = query_samples(
        &["fill", "snare_fill"],
        [
            BAD_GENRES,
            &[
                "bass", "synth", "pad", "lead", "melody", "loop", "full", "8bar", "4bar", "chord",
            ],
        ]
        .concat(),
        false,
        1,
    );
    eprintln!("FILL 4D:");
    let fill_4d = query_samples(
        &["fill", "perc_fill"],
        [
            BAD_GENRES,
            &[
                "bass", "synth", "pad", "lead", "melody", "loop", "full", "8bar", "4bar", "chord",
            ],
        ]
        .concat(),
        false,
        1,
    );

    // FX - REVERSE
    eprintln!("REVERSE 1:");
    let reverse1 = query_samples(
        &["reverse", "rev_cymbal", "rev_crash"],
        [BAD_GENRES, &["loop"]].concat(),
        false,
        1,
    );
    eprintln!("REVERSE 2:");
    let reverse2 = query_samples(
        &["reverse", "rev_fx", "reversed"],
        [BAD_GENRES, &["loop"]].concat(),
        false,
        1,
    );

    // FX - SUB DROP
    eprintln!("SUB DROP:");
    let sub_drop = query_samples(
        &["sub_drop", "808_hit", "sub_boom", "low_impact"],
        [BAD_GENRES, &["loop"]].concat(),
        false,
        1,
    );

    // ATMOSPHERE - MATCH KEY
    eprintln!("ATMOS (key: {}):", track_key);
    let atmos = query_samples_with_key(
        &["atmos", "atmosphere", "ambient", "dark", "industrial"],
        [BAD_GENRES, &["drum", "kick"]].concat(),
        false,
        1,
        Some(&track_key),
    );
    eprintln!("ATMOS 2 (key: {}):", track_key);
    let atmos2 = query_samples_with_key(
        &["texture", "drone", "soundscape", "dark"],
        [BAD_GENRES, &["drum", "kick"]].concat(),
        false,
        1,
        Some(&track_key),
    );
    eprintln!("VOX (key: {}):", track_key);
    let vox = query_samples_with_key(
        &["vox", "vocal", "voice", "dark", "industrial"],
        [BAD_GENRES, &["drum"]].concat(),
        false,
        1,
        Some(&track_key),
    );

    SongSamples {
        key: track_key,
        kick,
        clap,
        hat,
        hat2,
        perc,
        perc2,
        ride,
        bass,
        sub,
        main_synth,
        synth1,
        synth2,
        synth3,
        pad,
        pad2,
        arp,
        arp2,
        riser1,
        riser2,
        riser3,
        downlifter,
        crash,
        impact,
        hit,
        sweep_up,
        sweep_down,
        sweep_up2,
        sweep_down2,
        noise,
        noise2,
        snare_roll,
        fill_1a,
        fill_1b,
        fill_2a,
        fill_2b,
        fill_4a,
        fill_4b,
        fill_4c,
        fill_4d,
        reverse1,
        reverse2,
        sub_drop,
        atmos,
        atmos2,
        vox,
    }
}

fn generate_true_techno(output_path: &Path) -> Result<(), String> {
    let ids = IdAllocator::new(1000000);
    let bars_per_song = (SONG_LENGTH_BARS + GAP_BETWEEN_SONGS) as f64;

    eprintln!("\n========================================");
    eprintln!("GENERATING {} SONGS IN ONE ALS FILE", NUM_SONGS);
    eprintln!(
        "Each song: {} bars + {} bar gap = {} bars total",
        SONG_LENGTH_BARS, GAP_BETWEEN_SONGS, bars_per_song as u32
    );
    eprintln!("Total length: {} bars", bars_per_song as u32 * NUM_SONGS);
    eprintln!("========================================\n");

    // Load samples for each song (each gets its own key)
    let mut all_songs: Vec<SongSamples> = Vec::new();
    for song_num in 1..=NUM_SONGS {
        all_songs.push(load_song_samples(song_num));
    }

    // Collect keys for locators
    let song_keys: Vec<String> = all_songs.iter().map(|s| s.key.clone()).collect();

    eprintln!("\n========================================");
    eprintln!("SONG KEYS:");
    for (i, key) in song_keys.iter().enumerate() {
        eprintln!("  Song {}: {}", i + 1, key);
    }
    eprintln!("========================================\n");

    // Use samples from each song for their respective sections
    // For now, use the first song's samples and repeat the arrangement for each song
    let song1 = &all_songs[0];
    // Use all samples from SongSamples - samples were already loaded by load_song_samples()
    let track_key = song1.key.clone();

    // DRUMS
    let kick_samples = song1.kick.clone();
    let clap_samples = song1.clap.clone();
    let hat_samples = song1.hat.clone();
    let hat2_samples = song1.hat2.clone();
    let perc_samples = song1.perc.clone();
    let perc2_samples = song1.perc2.clone();
    let ride_samples = song1.ride.clone();

    // BASS
    let bass_samples = song1.bass.clone();
    let sub_samples = song1.sub.clone();

    // MELODICS
    let main_synth_samples = song1.main_synth.clone();
    let synth1_samples = song1.synth1.clone();
    let synth2_samples = song1.synth2.clone();
    eprintln!("SYNTH 3 (stabs, key: {}):", track_key);
    let synth3_samples = query_samples_with_key(
        &["stab", "synth_shot", "chord_stab"],
        [
            BAD_GENRES,
            &[
                "pad", "bass", "sub", "drum", "drums", "kick", "hat", "snare", "clap", "perc",
                "ride", "full", "kit", "loop",
            ],
        ]
        .concat(),
        false,
        1,
        Some(&track_key),
    );
    eprintln!("PAD (key: {}):", track_key);
    let pad_samples = query_samples_with_key(
        &["pad_loop", "pad"],
        [
            BAD_GENRES,
            &[
                "drum", "drums", "stab", "bass", "sub", "kick", "hat", "snare", "clap", "perc",
                "ride", "full", "kit", "lead", "arp",
            ],
        ]
        .concat(),
        true,
        1,
        Some(&track_key),
    );
    eprintln!("PAD 2 (key: {}):", track_key);
    let pad2_samples = query_samples_with_key(
        &["drone_loop", "atmosphere_loop", "drone", "atmosphere"],
        [
            BAD_GENRES,
            &[
                "drum", "drums", "kick", "stab", "bass", "sub", "hat", "snare", "clap", "perc",
                "ride", "full", "kit", "lead", "arp",
            ],
        ]
        .concat(),
        true,
        1,
        Some(&track_key),
    );
    eprintln!("ARP (key: {}):", track_key);
    let arp_samples = query_samples_with_key(
        &["arp_loop", "arpegg", "arp"],
        [
            BAD_GENRES,
            &[
                "pad", "drum", "drums", "bass", "sub", "kick", "hat", "snare", "clap", "perc",
                "ride", "full", "kit", "lead",
            ],
        ]
        .concat(),
        true,
        1,
        Some(&track_key),
    );
    eprintln!("ARP 2 (key: {}):", track_key);
    let arp2_samples = query_samples_with_key(
        &["pluck_loop", "sequence_loop", "pluck"],
        [
            BAD_GENRES,
            &[
                "pad", "drum", "drums", "chord", "bass", "sub", "kick", "hat", "snare", "clap",
                "perc", "ride", "full", "kit", "lead",
            ],
        ]
        .concat(),
        true,
        1,
        Some(&track_key),
    );

    // FX - RISERS
    eprintln!("RISER 1:");
    let riser1_samples = query_samples(
        &["riser", "uplifter"],
        [BAD_GENRES, &["down", "impact"]].concat(),
        false,
        1,
    );
    eprintln!("RISER 2:");
    let riser2_samples = query_samples(
        &["build", "riser", "tension"],
        [BAD_GENRES, &["down", "impact"]].concat(),
        false,
        1,
    );
    eprintln!("RISER 3 (short):");
    let riser3_samples = query_samples(
        &["whoosh", "sweep_up", "upsweep"],
        [BAD_GENRES, &["down"]].concat(),
        false,
        1,
    );

    // FX - DOWNLIFTERS
    eprintln!("DOWNLIFTER:");
    let downlifter_samples = query_samples(
        &["downlifter", "downsweep", "down_sweep", "fall"],
        [BAD_GENRES, &["up", "riser"]].concat(),
        false,
        1,
    );

    // FX - IMPACTS
    eprintln!("CRASH:");
    let crash_samples = query_samples(
        &["crash", "cymbal_crash"],
        [BAD_GENRES, &["loop", "ride"]].concat(),
        false,
        1,
    );
    eprintln!("IMPACT:");
    let impact_samples = query_samples(
        &["impact", "boom", "thud"],
        [BAD_GENRES, &["loop", "riser"]].concat(),
        false,
        1,
    );
    eprintln!("HIT:");
    let hit_samples = query_samples(
        &["hit", "fx_hit", "perc_shot"],
        [BAD_GENRES, &["loop", "riser", "crash"]].concat(),
        false,
        1,
    );

    // FX - SWEEPS
    eprintln!("SWEEP UP:");
    let sweep_up_samples = query_samples(
        &["sweep_up", "upsweep", "white_noise_up"],
        [BAD_GENRES, &["down"]].concat(),
        false,
        1,
    );
    eprintln!("SWEEP DOWN:");
    let sweep_down_samples = query_samples(
        &["sweep_down", "downsweep", "white_noise_down"],
        [BAD_GENRES, &["up"]].concat(),
        false,
        1,
    );
    eprintln!("SWEEP UP 2:");
    let sweep_up2_samples = query_samples(
        &["sweep", "riser", "build"],
        [BAD_GENRES, &["down", "impact"]].concat(),
        false,
        1,
    );
    eprintln!("SWEEP DOWN 2:");
    let sweep_down2_samples = query_samples(
        &["fall", "drop", "down"],
        [BAD_GENRES, &["up", "sub"]].concat(),
        false,
        1,
    );

    // FX - NOISE
    eprintln!("NOISE:");
    let noise_samples = query_samples(
        &["noise", "white_noise", "hiss"],
        [BAD_GENRES, &["drum", "kick"]].concat(),
        false,
        1,
    );
    eprintln!("NOISE 2:");
    let noise2_samples = query_samples(
        &["texture", "noise", "static"],
        [BAD_GENRES, &["drum", "kick", "bass"]].concat(),
        false,
        1,
    );

    // FX - SNARE ROLLS
    eprintln!("SNARE ROLL:");
    let snare_roll_samples = query_samples(
        &["snare_roll", "snare_build", "snare_fill", "roll"],
        [BAD_GENRES, &["kick", "hat"]].concat(),
        false,
        1,
    );

    // FX - DRUM FILLS - multiple samples for variation (A/B alternating)
    let fill_exclude = [
        BAD_GENRES,
        &[
            "bass", "synth", "pad", "lead", "melody", "loop", "full", "8bar", "4bar", "chord",
        ],
    ]
    .concat();
    eprintln!("FILL 1A (1 beat):");
    let fill_1a_samples = query_samples(
        &["drum_fill", "fill", "perc_hit", "tom_hit"],
        fill_exclude.clone(),
        false,
        1,
    );
    eprintln!("FILL 1B (1 beat):");
    let fill_1b_samples = query_samples(
        &["snare_hit", "drum_hit", "hit"],
        fill_exclude.clone(),
        false,
        1,
    );
    eprintln!("FILL 2A (2 beats):");
    let fill_2a_samples = query_samples(
        &["drum_fill", "fill", "snare_roll"],
        fill_exclude.clone(),
        false,
        1,
    );
    eprintln!("FILL 2B (2 beats):");
    let fill_2b_samples = query_samples(
        &["tom_roll", "drum_roll", "roll"],
        fill_exclude.clone(),
        false,
        1,
    );
    eprintln!("FILL 4A (1 bar):");
    let fill_4a_samples = query_samples(
        &["drum_fill", "fill", "break"],
        fill_exclude.clone(),
        false,
        1,
    );
    eprintln!("FILL 4B (1 bar):");
    let fill_4b_samples = query_samples(
        &["drum_break", "tom_fill", "fill"],
        fill_exclude.clone(),
        false,
        1,
    );
    eprintln!("FILL 4C (1 bar):");
    let fill_4c_samples = query_samples(
        &["snare_fill", "perc_fill", "fill"],
        fill_exclude.clone(),
        false,
        1,
    );
    eprintln!("FILL 4D (1 bar):");
    let fill_4d_samples = query_samples(
        &["crash_fill", "cymbal_fill", "fill", "break"],
        fill_exclude,
        false,
        1,
    );

    // FX - REVERSE (2 different samples)
    eprintln!("REVERSE 1:");
    let reverse1_samples = query_samples(
        &["reverse", "rev_crash", "reversed"],
        [BAD_GENRES, &["loop"]].concat(),
        false,
        1,
    );
    eprintln!("REVERSE 2:");
    let reverse2_samples = query_samples(
        &["rev_cymbal", "reverse_hit", "reverse"],
        [BAD_GENRES, &["loop"]].concat(),
        false,
        1,
    );

    // FX - SUB DROP
    eprintln!("SUB DROP:");
    let sub_drop_samples = query_samples(
        &["sub_drop", "808_hit", "sub_boom", "low_impact"],
        [BAD_GENRES, &["loop"]].concat(),
        false,
        1,
    );

    // ATMOSPHERE - dark/industrial only
    eprintln!("ATMOS (key: {}):", track_key);
    let atmos_samples = query_samples_with_key(
        &["atmos", "atmosphere", "ambient", "dark", "industrial"],
        [BAD_GENRES, &["drum", "kick"]].concat(),
        false,
        1,
        Some(&track_key),
    );
    eprintln!("ATMOS 2 (key: {}):", track_key);
    let atmos2_samples = query_samples_with_key(
        &["texture", "drone", "soundscape", "dark"],
        [BAD_GENRES, &["drum", "kick"]].concat(),
        false,
        1,
        Some(&track_key),
    );
    eprintln!("VOX (key: {}):", track_key);
    let vox_samples = query_samples_with_key(
        &["vox", "vocal", "voice", "dark", "industrial"],
        [BAD_GENRES, &["drum"]].concat(),
        false,
        1,
        Some(&track_key),
    );

    // Generate base ALS
    generate_empty_als(output_path)?;

    let file = File::open(output_path).map_err(|e| e.to_string())?;
    let mut decoder = GzDecoder::new(file);
    let mut xml = String::new();
    decoder
        .read_to_string(&mut xml)
        .map_err(|e| e.to_string())?;

    // Reserve template IDs
    let id_re = Regex::new(r#"Id="(\d+)""#).unwrap();
    for cap in id_re.captures_iter(&xml) {
        if let Ok(id) = cap[1].parse::<u32>() {
            ids.reserve(id);
        }
    }

    // Extract audio track template
    let track_start = xml.find("<AudioTrack").ok_or("No AudioTrack found")?;
    let track_end =
        xml.find("</AudioTrack>").ok_or("No AudioTrack end found")? + "</AudioTrack>".len();
    let original_audio_track = xml[track_start..track_end].to_string();

    // Allocate group IDs
    let drums_group_id = ids.alloc();
    let melodics_group_id = ids.alloc();
    let fx_group_id = ids.alloc();

    // Create groups
    let drums_group = create_group_track("DRUMS", DRUMS_COLOR, drums_group_id, &ids)?;
    let melodics_group = create_group_track("MELODICS", MELODICS_COLOR, melodics_group_id, &ids)?;
    let fx_group = create_group_track("FX", FX_COLOR, fx_group_id, &ids)?;

    // Get arrangement structure
    let arrangements = get_arrangement();

    // Helper to find arrangement for a track
    let find_arr = |name: &str| -> Vec<(f64, f64)> {
        arrangements
            .iter()
            .find(|a| a.name == name)
            .map(|a| a.sections.clone())
            .unwrap_or_default()
    };

    // Create all tracks with TRUE arrangement
    // DRUMS
    let kick_track = create_arranged_track(
        &original_audio_track,
        "KICK",
        DRUMS_COLOR,
        drums_group_id as i32,
        &kick_samples,
        &find_arr("KICK"),
        &ids,
    )?;
    let clap_track = create_arranged_track(
        &original_audio_track,
        "CLAP",
        DRUMS_COLOR,
        drums_group_id as i32,
        &clap_samples,
        &find_arr("CLAP"),
        &ids,
    )?;
    let hat_track = create_arranged_track(
        &original_audio_track,
        "HAT",
        DRUMS_COLOR,
        drums_group_id as i32,
        &hat_samples,
        &find_arr("HAT"),
        &ids,
    )?;
    let hat2_track = create_arranged_track(
        &original_audio_track,
        "HAT 2",
        DRUMS_COLOR,
        drums_group_id as i32,
        &hat2_samples,
        &find_arr("HAT 2"),
        &ids,
    )?;
    let perc_track = create_arranged_track(
        &original_audio_track,
        "PERC",
        DRUMS_COLOR,
        drums_group_id as i32,
        &perc_samples,
        &find_arr("PERC"),
        &ids,
    )?;
    let perc2_track = create_arranged_track(
        &original_audio_track,
        "PERC 2",
        DRUMS_COLOR,
        drums_group_id as i32,
        &perc2_samples,
        &find_arr("PERC 2"),
        &ids,
    )?;
    let ride_track = create_arranged_track(
        &original_audio_track,
        "RIDE",
        DRUMS_COLOR,
        drums_group_id as i32,
        &ride_samples,
        &find_arr("RIDE"),
        &ids,
    )?;

    // BASS
    let bass_track = create_arranged_track(
        &original_audio_track,
        "BASS",
        BASS_COLOR,
        -1,
        &bass_samples,
        &find_arr("BASS"),
        &ids,
    )?;
    let sub_track = create_arranged_track(
        &original_audio_track,
        "SUB",
        BASS_COLOR,
        -1,
        &sub_samples,
        &find_arr("SUB"),
        &ids,
    )?;

    // MELODICS
    let main_synth_track = create_arranged_track(
        &original_audio_track,
        "MAIN SYNTH",
        MELODICS_COLOR,
        melodics_group_id as i32,
        &main_synth_samples,
        &find_arr("MAIN SYNTH"),
        &ids,
    )?;
    let synth1_track = create_arranged_track(
        &original_audio_track,
        "SYNTH 1",
        MELODICS_COLOR,
        melodics_group_id as i32,
        &synth1_samples,
        &find_arr("SYNTH 1"),
        &ids,
    )?;
    let synth2_track = create_arranged_track(
        &original_audio_track,
        "SYNTH 2",
        MELODICS_COLOR,
        melodics_group_id as i32,
        &synth2_samples,
        &find_arr("SYNTH 2"),
        &ids,
    )?;
    let synth3_track = create_arranged_track(
        &original_audio_track,
        "SYNTH 3",
        MELODICS_COLOR,
        melodics_group_id as i32,
        &synth3_samples,
        &find_arr("SYNTH 3"),
        &ids,
    )?;
    let pad_track = create_arranged_track(
        &original_audio_track,
        "PAD",
        MELODICS_COLOR,
        melodics_group_id as i32,
        &pad_samples,
        &find_arr("PAD"),
        &ids,
    )?;
    let pad2_track = create_arranged_track(
        &original_audio_track,
        "PAD 2",
        MELODICS_COLOR,
        melodics_group_id as i32,
        &pad2_samples,
        &find_arr("PAD 2"),
        &ids,
    )?;
    let arp_track = create_arranged_track(
        &original_audio_track,
        "ARP",
        MELODICS_COLOR,
        melodics_group_id as i32,
        &arp_samples,
        &find_arr("ARP"),
        &ids,
    )?;
    let arp2_track = create_arranged_track(
        &original_audio_track,
        "ARP 2",
        MELODICS_COLOR,
        melodics_group_id as i32,
        &arp2_samples,
        &find_arr("ARP 2"),
        &ids,
    )?;

    // FX
    let riser1_track = create_arranged_track(
        &original_audio_track,
        "RISER 1",
        FX_COLOR,
        fx_group_id as i32,
        &riser1_samples,
        &find_arr("RISER 1"),
        &ids,
    )?;
    let riser2_track = create_arranged_track(
        &original_audio_track,
        "RISER 2",
        FX_COLOR,
        fx_group_id as i32,
        &riser2_samples,
        &find_arr("RISER 2"),
        &ids,
    )?;
    let riser3_track = create_arranged_track(
        &original_audio_track,
        "RISER 3",
        FX_COLOR,
        fx_group_id as i32,
        &riser3_samples,
        &find_arr("RISER 3"),
        &ids,
    )?;
    let downlifter_track = create_arranged_track(
        &original_audio_track,
        "DOWNLIFTER",
        FX_COLOR,
        fx_group_id as i32,
        &downlifter_samples,
        &find_arr("DOWNLIFTER"),
        &ids,
    )?;
    let crash_track = create_arranged_track(
        &original_audio_track,
        "CRASH",
        FX_COLOR,
        fx_group_id as i32,
        &crash_samples,
        &find_arr("CRASH"),
        &ids,
    )?;
    let impact_track = create_arranged_track(
        &original_audio_track,
        "IMPACT",
        FX_COLOR,
        fx_group_id as i32,
        &impact_samples,
        &find_arr("IMPACT"),
        &ids,
    )?;
    let hit_track = create_arranged_track(
        &original_audio_track,
        "HIT",
        FX_COLOR,
        fx_group_id as i32,
        &hit_samples,
        &find_arr("HIT"),
        &ids,
    )?;
    let sweep_up_track = create_arranged_track(
        &original_audio_track,
        "SWEEP UP",
        FX_COLOR,
        fx_group_id as i32,
        &sweep_up_samples,
        &find_arr("SWEEP UP"),
        &ids,
    )?;
    let sweep_down_track = create_arranged_track(
        &original_audio_track,
        "SWEEP DOWN",
        FX_COLOR,
        fx_group_id as i32,
        &sweep_down_samples,
        &find_arr("SWEEP DOWN"),
        &ids,
    )?;
    let sweep_up2_track = create_arranged_track(
        &original_audio_track,
        "SWEEP UP 2",
        FX_COLOR,
        fx_group_id as i32,
        &sweep_up2_samples,
        &find_arr("SWEEP UP 2"),
        &ids,
    )?;
    let sweep_down2_track = create_arranged_track(
        &original_audio_track,
        "SWEEP DOWN 2",
        FX_COLOR,
        fx_group_id as i32,
        &sweep_down2_samples,
        &find_arr("SWEEP DOWN 2"),
        &ids,
    )?;
    let noise_track = create_arranged_track(
        &original_audio_track,
        "NOISE",
        FX_COLOR,
        fx_group_id as i32,
        &noise_samples,
        &find_arr("NOISE"),
        &ids,
    )?;
    let noise2_track = create_arranged_track(
        &original_audio_track,
        "NOISE 2",
        FX_COLOR,
        fx_group_id as i32,
        &noise2_samples,
        &find_arr("NOISE 2"),
        &ids,
    )?;
    let snare_roll_track = create_arranged_track(
        &original_audio_track,
        "SNARE ROLL",
        FX_COLOR,
        fx_group_id as i32,
        &snare_roll_samples,
        &find_arr("SNARE ROLL"),
        &ids,
    )?;
    // Fills - multiple samples for variation
    let fill_1a_track = create_arranged_track(
        &original_audio_track,
        "FILL 1A",
        DRUMS_COLOR,
        drums_group_id as i32,
        &fill_1a_samples,
        &find_arr("FILL 1A"),
        &ids,
    )?;
    let fill_1b_track = create_arranged_track(
        &original_audio_track,
        "FILL 1B",
        DRUMS_COLOR,
        drums_group_id as i32,
        &fill_1b_samples,
        &find_arr("FILL 1B"),
        &ids,
    )?;
    let fill_2a_track = create_arranged_track(
        &original_audio_track,
        "FILL 2A",
        DRUMS_COLOR,
        drums_group_id as i32,
        &fill_2a_samples,
        &find_arr("FILL 2A"),
        &ids,
    )?;
    let fill_2b_track = create_arranged_track(
        &original_audio_track,
        "FILL 2B",
        DRUMS_COLOR,
        drums_group_id as i32,
        &fill_2b_samples,
        &find_arr("FILL 2B"),
        &ids,
    )?;
    let fill_4a_track = create_arranged_track(
        &original_audio_track,
        "FILL 4A",
        DRUMS_COLOR,
        drums_group_id as i32,
        &fill_4a_samples,
        &find_arr("FILL 4A"),
        &ids,
    )?;
    let fill_4b_track = create_arranged_track(
        &original_audio_track,
        "FILL 4B",
        DRUMS_COLOR,
        drums_group_id as i32,
        &fill_4b_samples,
        &find_arr("FILL 4B"),
        &ids,
    )?;
    let fill_4c_track = create_arranged_track(
        &original_audio_track,
        "FILL 4C",
        DRUMS_COLOR,
        drums_group_id as i32,
        &fill_4c_samples,
        &find_arr("FILL 4C"),
        &ids,
    )?;
    let fill_4d_track = create_arranged_track(
        &original_audio_track,
        "FILL 4D",
        DRUMS_COLOR,
        drums_group_id as i32,
        &fill_4d_samples,
        &find_arr("FILL 4D"),
        &ids,
    )?;
    // Reverse - 2 samples alternating
    let reverse1_track = create_arranged_track(
        &original_audio_track,
        "REVERSE 1",
        FX_COLOR,
        fx_group_id as i32,
        &reverse1_samples,
        &find_arr("REVERSE 1"),
        &ids,
    )?;
    let reverse2_track = create_arranged_track(
        &original_audio_track,
        "REVERSE 2",
        FX_COLOR,
        fx_group_id as i32,
        &reverse2_samples,
        &find_arr("REVERSE 2"),
        &ids,
    )?;
    let sub_drop_track = create_arranged_track(
        &original_audio_track,
        "SUB DROP",
        FX_COLOR,
        fx_group_id as i32,
        &sub_drop_samples,
        &find_arr("SUB DROP"),
        &ids,
    )?;
    let atmos_track = create_arranged_track(
        &original_audio_track,
        "ATMOS",
        FX_COLOR,
        fx_group_id as i32,
        &atmos_samples,
        &find_arr("ATMOS"),
        &ids,
    )?;
    let atmos2_track = create_arranged_track(
        &original_audio_track,
        "ATMOS 2",
        FX_COLOR,
        fx_group_id as i32,
        &atmos2_samples,
        &find_arr("ATMOS 2"),
        &ids,
    )?;
    let vox_track = create_arranged_track(
        &original_audio_track,
        "VOX",
        FX_COLOR,
        fx_group_id as i32,
        &vox_samples,
        &find_arr("VOX"),
        &ids,
    )?;

    // Build final XML - all tracks
    let before_track = &xml[..track_start];
    let after_track = &xml[track_end..];

    let all_tracks = vec![
        // DRUMS group
        &drums_group,
        &kick_track,
        &clap_track,
        &hat_track,
        &hat2_track,
        &perc_track,
        &perc2_track,
        &ride_track,
        // Fills (8 tracks with different samples)
        &fill_1a_track,
        &fill_1b_track,
        &fill_2a_track,
        &fill_2b_track,
        &fill_4a_track,
        &fill_4b_track,
        &fill_4c_track,
        &fill_4d_track,
        // BASS (no group)
        &bass_track,
        &sub_track,
        // MELODICS group
        &melodics_group,
        &main_synth_track,
        &synth1_track,
        &synth2_track,
        &synth3_track,
        &pad_track,
        &pad2_track,
        &arp_track,
        &arp2_track,
        // FX group
        &fx_group,
        &riser1_track,
        &riser2_track,
        &riser3_track,
        &downlifter_track,
        &crash_track,
        &impact_track,
        &hit_track,
        &sweep_up_track,
        &sweep_down_track,
        &sweep_up2_track,
        &sweep_down2_track,
        &noise_track,
        &noise2_track,
        &snare_roll_track,
        &reverse1_track,
        &reverse2_track,
        &sub_drop_track,
        &atmos_track,
        &atmos2_track,
        &vox_track,
    ]
    .into_iter()
    .map(|s| s.as_str())
    .collect::<Vec<_>>()
    .join("\n\t\t\t");

    let mut xml = format!("{}{}{}", before_track, all_tracks, after_track);

    // Update NextPointeeId
    let next_id = ids.max_id() + 1000;
    let next_id_re = Regex::new(r#"<NextPointeeId Value="\d+" />"#).unwrap();
    xml = next_id_re
        .replace(&xml, format!(r#"<NextPointeeId Value="{}" />"#, next_id))
        .to_string();

    // Hide mixer
    xml = xml.replace(
        r#"<MixerInArrangement Value="1" />"#,
        r#"<MixerInArrangement Value="0" />"#,
    );

    // Add locators at section boundaries for ALL songs
    let locators_xml = create_locators_xml_multi(&ids, NUM_SONGS, &song_keys);
    xml = xml.replace(
        "<Locators>\n\t\t\t<Locators />\n\t\t</Locators>",
        &locators_xml,
    );
    // Also try alternate format
    xml = xml.replace("<Locators><Locators /></Locators>", &locators_xml);

    // Set tempo to 128 BPM
    let tempo_re =
        Regex::new(r#"<Tempo>\s*<LomId Value="0" />\s*<Manual Value="[^"]+" />"#).unwrap();
    xml = tempo_re
        .replace(
            &xml,
            r#"<Tempo>
						<LomId Value="0" />
						<Manual Value="128" />"#,
        )
        .to_string();

    let tempo_event_re =
        Regex::new(r#"<FloatEvent Id="\d+" Time="-63072000" Value="[^"]+" />"#).unwrap();
    xml = tempo_event_re
        .replace(
            &xml,
            r#"<FloatEvent Id="0" Time="-63072000" Value="128" />"#,
        )
        .to_string();

    // Write output
    let output_file = File::create(output_path).map_err(|e| e.to_string())?;
    let mut encoder = GzEncoder::new(output_file, Compression::default());
    encoder
        .write_all(xml.as_bytes())
        .map_err(|e| e.to_string())?;
    encoder.finish().map_err(|e| e.to_string())?;

    eprintln!("\nMax ID used: {}", ids.max_id());
    Ok(())
}

fn create_audio_clip(
    sample: &SampleInfo,
    color: u32,
    clip_id: u32,
    start_bar: f64,
    end_bar: f64,
) -> String {
    let beats_per_bar = 4.0;
    // Both bars are 1-indexed, so subtract 1 before converting to beats
    // Bar 1 = beat 0, bar 16 = beat 60, bar 16.25 = beat 61
    let start_beat = (start_bar - 1.0) * beats_per_bar;
    let end_beat = (end_bar - 1.0) * beats_per_bar;

    // Clip length in beats
    let clip_length_beats = end_beat - start_beat;

    let loop_bars = sample.loop_bars(PROJECT_BPM);
    let sample_loop_beats = loop_bars as f64 * beats_per_bar;

    // Cap loop to clip length - don't let sample loop beyond the clip boundary
    let loop_beats = if clip_length_beats < sample_loop_beats {
        clip_length_beats
    } else {
        sample_loop_beats
    };

    // WarpMarker tells Ableton: "at SecTime seconds into the sample, we should be at BeatTime beats"
    // To sync correctly at PROJECT_BPM, we calculate: how many seconds = loop_beats at PROJECT_BPM
    // This forces Ableton to stretch/compress the sample to match our target tempo
    //
    // Formula: warp_sec = (loop_beats * 60) / PROJECT_BPM
    // At 128 BPM: 4 beats (1 bar) = 1.875 sec, 16 beats (4 bars) = 7.5 sec
    let warp_sec = (loop_beats * 60.0) / PROJECT_BPM;

    format!(
        r#"<AudioClip Id="{clip_id}" Time="{start_beat}">
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

fn create_group_track(
    name: &str,
    color: u32,
    group_id: u32,
    ids: &IdAllocator,
) -> Result<String, String> {
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
    track = track_id_re
        .replace(&track, format!(r#"<GroupTrack Id="{}""#, group_id))
        .to_string();

    track = track.replace(
        r#"<EffectiveName Value="Drums" />"#,
        &format!(r#"<EffectiveName Value="{}" />"#, name),
    );
    track = track.replace(
        r#"<UserName Value="Drums" />"#,
        &format!(r#"<UserName Value="{}" />"#, name),
    );

    let color_re = Regex::new(r#"<Color Value="\d+" />"#).unwrap();
    track = color_re
        .replace_all(&track, format!(r#"<Color Value="{}" />"#, color))
        .to_string();

    eprintln!("GroupTrack {}: ID={}", name, group_id);
    Ok(track)
}

fn create_arranged_track(
    template: &str,
    name: &str,
    color: u32,
    group_id: i32,
    samples: &[SampleInfo],
    sections: &[(f64, f64)],
    ids: &IdAllocator,
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
    track = name_re
        .replace(&track, format!(r#"<EffectiveName Value="{}" />"#, name))
        .to_string();

    let username_re =
        Regex::new(r#"(<EffectiveName Value="[^"]*" />\s*<UserName Value=")[^"]*(" />)"#).unwrap();
    track = username_re
        .replace(&track, format!(r#"${{1}}{}${{2}}"#, name))
        .to_string();

    // Set color
    let color_re = Regex::new(r#"<Color Value="\d+" />"#).unwrap();
    track = color_re
        .replace_all(&track, format!(r#"<Color Value="{}" />"#, color))
        .to_string();

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
    let volume_re =
        Regex::new(r#"(<Volume>\s*<LomId Value="0" />\s*<Manual Value=")[^"]+(" />)"#).unwrap();
    let volume_value = if name == "KICK" { "1" } else { "0.251188643" };
    track = volume_re
        .replace(&track, format!(r#"${{1}}{}${{2}}"#, volume_value))
        .to_string();

    // Create clips for each section using the SAME sample (no switching)
    let mut clips: Vec<String> = Vec::new();

    if !samples.is_empty() {
        // Use only the first sample for consistency across the arrangement
        let sample = &samples[0];
        for &(start_bar, end_bar) in sections.iter() {
            let clip_id = ids.alloc();
            clips.push(create_audio_clip(
                sample, color, clip_id, start_bar, end_bar,
            ));
        }
    }

    let clips_xml = clips.join("\n");
    track = track.replacen(
        "<Events />",
        &format!(
            "<Events>\n{}\n\t\t\t\t\t\t\t\t\t\t\t\t\t</Events>",
            clips_xml
        ),
        1,
    );

    let section_str: String = sections
        .iter()
        .map(|&(s, e)| {
            if s == e {
                if s.fract() == 0.0 {
                    format!("{}", s as u32)
                } else {
                    format!("{:.2}", s)
                }
            } else if s.fract() == 0.0 && e.fract() == 0.0 {
                format!("{}-{}", s as u32, e as u32)
            } else {
                format!("{:.2}-{:.2}", s, e)
            }
        })
        .collect::<Vec<_>>()
        .join(", ");
    eprintln!(
        "Track {}: {} clips at bars [{}]",
        name,
        clips.len(),
        section_str
    );
    Ok(track)
}
