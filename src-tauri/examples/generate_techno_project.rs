//! Generate a full Techno project with proper group track structure
//! Uses an ID allocator to guarantee unique IDs throughout
//! Loads sample metadata (duration, BPM) from the audio_haxor database

use app_lib::als_generator::generate_empty_als;
use flate2::Compression;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use regex::Regex;
use rusqlite::Connection;
use std::collections::HashSet;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use std::sync::atomic::{AtomicU32, Ordering};

const DB_PATH: &str =
    "/Users/wizard/Library/Application Support/com.menketechnologies.audio-haxor/audio_haxor.db";
const PROJECT_BPM: f64 = 128.0;

/// Read WAV file duration directly from file header
fn read_wav_duration(path: &str) -> Option<f64> {
    use std::io::{Read, Seek, SeekFrom};

    let mut file = File::open(path).ok()?;
    let mut header = [0u8; 44];
    file.read_exact(&mut header).ok()?;

    // Check RIFF header
    if &header[0..4] != b"RIFF" || &header[8..12] != b"WAVE" {
        return None;
    }

    // Find fmt chunk - it might not be at offset 12
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
            // Skip this chunk
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
const CLIPS_PER_TRACK: usize = 48; // 48 x 4 bars = 192 bars = 6 minutes at 128 BPM

// Fresh techno LOOPS - enough variety for 6 minute arrangement
const KICK_SAMPLES: &[&str] = &[
    "/Users/wizard/mnt/production/MusicProduction/Samples/sounds.com/Tribal Techno Elements/Tribal Techno Elements/Drum Loops/TTE_01_Drum_130bpm_Kick.wav",
    "/Users/wizard/mnt/production/MusicProduction/Samples/freshly squeezed/Freshly Squeezed Samples - Dave Parkinson House Essentials/DPHE Construction Kits/DPHE Construction Kit - 007 - 126 BPM/Stems/DPHE Construction Kit - 007 - Kick Drum Loop.wav",
    "/Users/wizard/mnt/production/MusicProduction/Samples/datacode/Datacode & Code Sounds - Epic Cinematic Bundle/CODSND 005 - Code Sounds - Alone Dark Chill & Cinematic/Beat Loops/CS-ADC-80-Beat Loop 01b-Kick Loop.wav",
    "/Users/wizard/mnt/production/MusicProduction/Samples/Splice/sounds/packs/Awakening - Massive Trance/PLX_-_Awakening_-_Massive_Trance/loops/kick_loops/PLX_AMT_138_kick_scare.wav",
    "/Users/wizard/mnt/production/MusicProduction/Samples/riemann/10 Sequences Top Loops/Riemann Tripping Techno 1 WAV/Loops/Kick Loops/RK_TT1_Kick_Loop_05_125bpm.wav",
    "/Users/wizard/mnt/production/MusicProduction/Samples/datacode/DATSND-002-Datacode-FOCUS-Techno-Drums/DATSND 002 - Datacode - FOCUS Techno Drums/Drum Loops/Kick Loops 125 BPM/DF-TD-125-Kick Loop 03.wav",
    "/Users/wizard/mnt/production/MusicProduction/Samples/sounds.com/Techno Forces/Techno Forces/DRUM LOOP/02 Kick 129bpm G#m Analog Layered.wav",
    "/Users/wizard/mnt/production/MusicProduction/Samples/wa/free/WAProd_Free_Anniversary_Collection_3/DEMO Packs/Redhead Roman Exclusive EDM/Drum Loops/Drum Loop5 Kicks 128 BPM.wav",
];

const CLAP_SAMPLES: &[&str] = &[
    "/Users/wizard/mnt/production/MusicProduction/Samples/black octopus/Black Octopus Sound - Leviathan 4/Drums - Kick Snare Loops/Lev4_KickSnareLoop_140bpm_Layered_3.wav",
    "/Users/wizard/mnt/production/MusicProduction/Samples/Splice/sounds/packs/Astral - Dark Melodic Progressive 2/loops/drum_loops/PLX_ADMP2_123_drum_loop_nix_snare.wav",
    "/Users/wizard/mnt/production/MusicProduction/Samples/Splice/sounds/packs/Obsidian - Industrial Trance/loops/drum_loops/ff_it_140_drum_loop_aurora_snare.wav",
    "/Users/wizard/mnt/production/MusicProduction/Samples/riemann/FREE Riemann Techno Starter Sample Pack 2021 for FL Studio and Ableton/Loops (24bit WAV)/Clap Loops/RK_DUBT1_Clap_Loop_01_128bpm.wav",
    "/Users/wizard/mnt/production/MusicProduction/Samples/riemann/02 Berlin Dark Melodic/Riemann Dark Melodic Techno 1 WAV and MIDI/Loops/Snare Loops/RK_DMT1_Snare_Loop_02_127bpm.wav",
    "/Users/wizard/mnt/production/MusicProduction/Samples/sounds.com/Techno Code/Techno Code/DRUM LOOP/13 Clap 125bpm D#m Layered Analog.wav",
    "/Users/wizard/mnt/production/MusicProduction/Samples/loopmasters/loopcloud21/UnitySamplesVol.10_byD-Unity_DinoMaggiorana (a1ae58b9746d)/BEATS_LOOPS/US_12_PERCUSSION_CLAP_126BPM.wav",
    "/Users/wizard/mnt/production/MusicProduction/Samples/CM/musicradar-carnival-rave-samples/Rhythmic loops @ 130 bpm/Snared Bear/Snared Bear - chopped 1.wav",
];

const HIHAT_SAMPLES: &[&str] = &[
    "/Users/wizard/mnt/production/MusicProduction/Samples/noizz/WarehouseTechno_Wav_SP/WarehouseTechno_Wav_SP/Loops/Percussion/138_HatDance_SP_01.wav",
    "/Users/wizard/mnt/production/MusicProduction/Samples/freshly squeezed/Freshly Squeezed Samples - Dave Parkinson House Essentials Volume 2/DPHE2 Drum Loops/DPHE2 Hat Loops/DPHE2 Hat Loop - 125 BPM/DPHE2 Hat Loop - 006 - 125 BPM.wav",
    "/Users/wizard/mnt/production/MusicProduction/Samples/sounds.com/Warehouse Techno/Warehouse Techno/Hat Loops/WT Hat Loop 18 128.wav",
    "/Users/wizard/mnt/production/MusicProduction/Samples/Splice/sounds/packs/Ascension - Techno & Trance/PLX_-_Ascension_-_Techno_&_Trance/loops/drum_loops/PLX_ATT_140_drum_loco_hats.wav",
    "/Users/wizard/mnt/production/MusicProduction/Samples/ztekno/ZTEKNO - NEW TECHNO WORLD (ZIP MAIN)/ZTEKNO - NEW TECHNO WORLD (ZIP MAIN)/ZNT_WAV_LOOPS/ZNT_DRUM_PARTS/ZNT_128_Hats_11_V3.wav",
    "/Users/wizard/mnt/production/MusicProduction/Samples/black octopus/Chop+Shop+Samples-Techno+Top+Loops+Vol+02/Chop Shop Samples-Techno Top Loops Vol 02/Bonus Loops/Hi-Hat Loops/CSS006_TTL02_hihat_loop_29_128bpm.wav",
    "/Users/wizard/mnt/production/MusicProduction/Samples/Splice/sounds/packs/Trance Evolver/loops/drum_loops/PLX_TE_129_drum_loop_yellow_hats.wav",
    "/Users/wizard/mnt/production/MusicProduction/Samples/loopmasters/loopcloud21/Unity_Samples_Vol7_by_Dino_Maggiorana (9c8e2d1a88cb)/BEAT LOOPS/US_12_HIHAT_TOP_125_Fm.wav",
];

const PERC_SAMPLES: &[&str] = &[
    "/Users/wizard/mnt/production/MusicProduction/Samples/noizz/AfroLatinPercussion_Wav_SP/AfroLatinPercussion_Wav_SP/Loops/120bpm/Conga/120_CongaOrisa_SP_01.wav",
    "/Users/wizard/mnt/production/MusicProduction/Samples/functionloops/Function_Loops_-_Psytrance_Drums/Function Loops - Psytrance Drums/FL_PSD_140_Ethnic_Percussion_Loops/FL_PSD_140_Ethnic_Percussion_Loop_Wet_05_G#.wav",
    "/Users/wizard/mnt/production/MusicProduction/Samples/ghosthack/Ghosthack Ultimate Producer Bundle 2019 - Drum Packs/Ghosthack - Abstract Drums/Foley Perc Loops/Foley Loop_140BPM (89).wav",
    "/Users/wizard/mnt/production/MusicProduction/Samples/freshly squeezed/Dave Parkinson Trance Essentials/DPTE Drum Loops/DPTE Drum Loops - 138 BPM/DPTE Drum Loop - 036 - 138 BPM - Percussion Loop.wav",
    "/Users/wizard/mnt/production/MusicProduction/Samples/noizz/AfroLatinPercussion_Wav_SP/AfroLatinPercussion_Wav_SP/Loops/120bpm/Conga/120_CongaGuaguanco_SP_01.wav",
    "/Users/wizard/mnt/production/MusicProduction/Samples/kshmr/Sounds of KSHMR Vol 1-3 (Complete Collection)/Sounds of KSHMR Vol 3/KSHMR_Drums/KSHMR_Drum_Loops/KSHMR_Drum_Loops_Full/KSHMR_Percussion_Loops/KSHMR_Percussion_Loop_32_128_Momentum.wav",
    "/Users/wizard/mnt/production/MusicProduction/Samples/kshmr/Sounds of KSHMR Vol 1-3 (Complete Collection)/Sounds of KSHMR Vol 3/KSHMR_Drums/KSHMR_Drum_Loops/KSHMR_Drum_Loops_Full/KSHMR_Percussion_Loops/KSHMR_Percussion_Loop_26_123.wav",
    "/Users/wizard/mnt/production/MusicProduction/Samples/ghosthack/Ghosthack Ultimate Producer Bundle 2019 - Drum Packs/Ghosthack - Abstract Drums/Foley Perc Loops/Foley Loop_128BPM (96).wav",
];

const BASS_SAMPLES: &[&str] = &[
    "/Users/wizard/mnt/production/MusicProduction/Samples/ztekno/ZTEKNO - TECH HOUSE BOOK (WAVS)/ZTEKNO - TECH HOUSE BOOK (WAVS)/ZTTB_WAV_LOOPS/ZTTB_BASS_LOOPS/ZTTB_126_C_Bass_Loop_1.wav",
    "/Users/wizard/mnt/production/MusicProduction/Samples/black octopus/BSH146+-+Space+Techno+2/BSH146 - Space Techno 2/Space Techno 2-WAV/Bass Loops/ST2 Bass Loop 55 Gm 128.wav",
    "/Users/wizard/mnt/production/MusicProduction/Samples/sounds.com/Techno Sub Bass Loops vol.1/Techno Sub Bass Loops vol.1/LOOPS_125bpm/BASS LOOPS_125bpm/BFM_TSB_SubBassLoop_80_D_125bpm.wav",
    "/Users/wizard/mnt/production/MusicProduction/Samples/black octopus/Chop+Shop+Samples-Techno+Love/Chop Shop Samples-Techno Love/Bass Loops/CSS024_bass_loop_05_126bpm.wav",
    "/Users/wizard/mnt/production/MusicProduction/Samples/Producer loops/2021/Dark Techno 2 - Sample Tools by Cr2/Audio-MIDI-Presets/Bass Loops/14_DT2_Bass_Loop_126_A#.wav",
    "/Users/wizard/mnt/production/MusicProduction/Samples/ztekno/ZTEKNO - PRIME TECHNO (WAVS)/ZTPT_WAV_LOOPS/ZTPT_BASS_LOOPS/ZTPT_126_F#_Bass_Loop_2_SC.wav",
    "/Users/wizard/mnt/production/MusicProduction/Samples/riemann/02 Berlin Dark Melodic/Riemann Dark Melodic Techno 4 WAV/Loops/Bass Loops/RK_DMT4_Bass_Loop_11_128bpm_G.wav",
    "/Users/wizard/mnt/production/MusicProduction/Samples/CM/Blockbuster/Invisible System/bass fx loops etc/bassinfected120bpm_01.wav",
];

const SYNTH_SAMPLES: &[&str] = &[
    "/Users/wizard/mnt/production/MusicProduction/Samples/sounds.com/Mental Tech/Mental Tech/INSTRUMENT LOOP/02 Synth Lead 125bpm Gm Analog Lead.wav",
    "/Users/wizard/mnt/production/MusicProduction/Samples/Splice/sounds/packs/Ryan K Harder Side Of Dance/Freshly_Squeezed_Samples_-_Ryan_K_Harder_Side_Of_Dance/Loops/Synth_Loops/Trance_Synth_Loops/FSS_RKHSOD_140_synth_trance_loop_lushpluck_A#.wav",
    "/Users/wizard/mnt/production/MusicProduction/Samples/black octopus/Production_Master_-_Rezonance_-_Melodic_Techno/Production Master - Rezonance - Melodic Techno/Synth Loops/PMRZ_Synth_Loop_41_123_Fm.wav",
    "/Users/wizard/mnt/production/MusicProduction/Samples/ztekno/ZTEKNO - TECHNO RATTLE WAVS (38057eb74d1b)/ZTR_ANALOG_SYNTH_LOOPS/ZTR_130_A#_Analog_Synth_Loop_2_Wet.wav",
    "/Users/wizard/mnt/production/MusicProduction/Samples/resonance sound/SOR Minimal Techno Revolution Vol.5/SOR MTR5 Synth Loop 127/SOR MTR5 Synthloop 127 69 D#.wav",
    "/Users/wizard/mnt/production/MusicProduction/Samples/riemann/10 Sequences Top Loops/Riemann Techno Sequences 5 WAV/RK_TSEQ5_Synth_Loop_31_135bpm_Amin.wav",
    "/Users/wizard/mnt/production/MusicProduction/Samples/Splice/sounds/packs/Alloy - Melodic Techno/Alloy_-_Zenhiser/loops/synth_loops/lead_synth_loops/ZEN_ALL_125_floaty_lead_synth_loopl_pendulum_A.wav",
    "/Users/wizard/mnt/production/MusicProduction/Samples/ztekno/ZTEKNO - TECHNO ATOM (WAVS)/ZTEKNO - TECHNO ATOM (WAVS)/ZTTA_WAV_LOOPS/ZTTA_SYNTH_LOOPS/ZTTA_125_A#_Synth_Loop_2_Wet.wav",
];

const PAD_SAMPLES: &[&str] = &[
    "/Users/wizard/mnt/production/MusicProduction/Samples/loopmasters/Loopmasters - John 00 Fleming Presents Variations in Trance/WAV LOOPS/Synth_and_Pad_Loops/00DB_Pad_forever_lands_135_C.wav",
    "/Users/wizard/mnt/production/MusicProduction/Samples/Splice/sounds/packs/Raw Techno/SM88_-_Raw_Techno_-_Wav/pad_and_drone_loops/rt_pad130_creep_B.wav",
    "/Users/wizard/mnt/production/MusicProduction/Samples/Producer loops/2021/Sound of Techno/AA4POL~O/S6BXZ5~H/Synth Loops/09 SOT synth loop 127 atmospheric2 C.wav",
    "/Users/wizard/mnt/production/MusicProduction/Samples/Splice/sounds/packs/Ascension - Techno & Trance/PLX_-_Ascension_-_Techno_&_Trance/loops/atmosphere_loops/PLX_ATT_140_atmosphere_darkness_Gmin.wav",
    "/Users/wizard/mnt/production/MusicProduction/Samples/freshly squeezed/Freshly Squeezed Samples - Ad Brown Progressive House Essentials/Ad Brown Progressive House Essentials - Construction Kits (WAV)/ABPHE Construction Kits - 128 BPM/128 BPM - Northern Sun/Audio Stems/ABPHE Northern Sun - 128 BPM - Pad Loop.wav",
    "/Users/wizard/mnt/production/MusicProduction/Samples/ztekno/ZTEKNO - TECHNO WISH (WAVS) (be97b77d3e42)/ZTW_SYNTH_LOOPS/ZTW_130_F_Pad_Synth_Loop_3_Wet.wav",
    "/Users/wizard/mnt/production/MusicProduction/Samples/freshly squeezed/Dave Parkinson Trance Essentials 2/DPTE2 Music Loops - 140 BPM/DPTE2 Pad Loops - 140 BPM/DPTE2 Pad Loops - 140 BPM - D/DPTE2 Pad Loop - 014 - D Minor - 140 BPM.wav",
    "/Users/wizard/mnt/production/MusicProduction/Samples/ztekno/ZTEKNO - TECHNO PSYCHOMACHINE (WAVS)/ZTEKNO - TECHNO PSYCHOMACHINE (WAVS)/ZTP_SYNTH_LOOPS/ZTP_130_B_Pad_Synth_Loop_2_Dry.wav",
];

const FX_SAMPLES: &[&str] = &[
    "/Users/wizard/mnt/production/MusicProduction/Samples/resonance sound/SOR_FX_Revolution_Vol.1/SOR FXR1 Impacts/SOR FXR1 Impacts 032.wav",
    "/Users/wizard/mnt/production/MusicProduction/Samples/black octopus/Black Octopus Sound - Leviathan 3/FX - Risers/Lev3_Riser_4bars_128bpm_A#_SirenEsque.wav",
    "/Users/wizard/mnt/production/MusicProduction/Samples/ghosthack/Ghosthack - Ultimate Producer Bundle 2020 Part2/Ghosthack - Shockwave FX/SFX_Bass_GH/SFX_Impact_GH/SFX_Bass_Impact_FIltered_GH.wav",
    "/Users/wizard/mnt/production/MusicProduction/Samples/black octopus/Production+Master+-+Selektor+-+Berlin+Techno/Production Master - Selektor - Berlin Techno/FX/Impacts/PMSL_FX_Impact_10.wav",
    "/Users/wizard/mnt/production/MusicProduction/Samples/black octopus/Black Octopus Sound - Leviathan 4/FX - Impacts/Lev4_Impact_LowVerb_08.wav",
    "/Users/wizard/mnt/production/MusicProduction/Samples/functionloops/FunctionLoops-KingsofPsytrance/Function Loops - Kings of Psytrance/FL_PT_KIT02_138BPM_D#/FL_PT_KIT02_138BPM_STEMS/FL_PT_KIT02_STM_138_D#_Sweep 02.wav",
    "/Users/wizard/mnt/production/MusicProduction/Samples/ghosthack/Ghosthack - Cinematic SFX Volume 3/Ghosthack - Cinematic SFX Volume 3/Impacts/Ghosthack-CSFX3_Impacts_Trailer.wav",
    "/Users/wizard/mnt/production/MusicProduction/Samples/sounds.com/Techno Grooves/Techno Grooves/One Shots/FX/FX Sweep 19.wav",
];

const DRUMS_COLOR: u32 = 69;
const BASS_COLOR: u32 = 13;
const MELODICS_COLOR: u32 = 26;
const FX_COLOR: u32 = 57;

/// Global ID allocator - guarantees unique IDs
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
    let output_path = Path::new("/Users/wizard/Desktop/Techno_Project.als");

    match generate_techno_project(output_path) {
        Ok(()) => {
            println!("Generated: {}", output_path.display());
            println!("\nTechno Structure:");
            println!("├── DRUMS (Group)");
            println!("│   ├── KICK ({} clips)", CLIPS_PER_TRACK);
            println!("│   ├── CLAP ({} clips)", CLIPS_PER_TRACK);
            println!("│   ├── HAT ({} clips)", CLIPS_PER_TRACK);
            println!("│   └── PERC ({} clips)", CLIPS_PER_TRACK);
            println!("├── BASS ({} clips)", CLIPS_PER_TRACK);
            println!("├── MELODICS (Group)");
            println!("│   ├── SYNTH 1 ({} clips)", CLIPS_PER_TRACK);
            println!("│   ├── SYNTH 2 ({} clips)", CLIPS_PER_TRACK);
            println!("│   └── PAD ({} clips)", CLIPS_PER_TRACK);
            println!("├── FX (Group)");
            println!("│   ├── RISER ({} clips)", CLIPS_PER_TRACK);
            println!("│   ├── HITS ({} clips)", CLIPS_PER_TRACK);
            println!("│   └── ATMOS ({} clips)", CLIPS_PER_TRACK);
            println!("\nTotal: {} clips across 12 tracks", 12 * CLIPS_PER_TRACK);
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
    fn from_path(path: &str) -> Result<Self, String> {
        let metadata =
            std::fs::metadata(path).map_err(|e| format!("Cannot read {}: {}", path, e))?;
        let name = Path::new(path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("sample")
            .to_string();

        // Read actual duration from WAV file
        let duration_secs = read_wav_duration(path).unwrap_or(0.0);

        Ok(Self {
            path: path.to_string(),
            name,
            file_size: metadata.len(),
            duration_secs,
            bpm: None,
        })
    }

    fn from_db(path: &str, db_duration: f64, bpm: Option<f64>) -> Result<Self, String> {
        let metadata =
            std::fs::metadata(path).map_err(|e| format!("Cannot read {}: {}", path, e))?;
        let name = Path::new(path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("sample")
            .to_string();

        // Read actual duration from WAV file - don't trust DB duration
        let duration_secs = read_wav_duration(path).unwrap_or(db_duration);

        Ok(Self {
            path: path.to_string(),
            name,
            file_size: metadata.len(),
            duration_secs,
            bpm,
        })
    }

    /// Calculate loop length in bars based on duration and BPM
    /// Falls back to project BPM (128) if sample BPM unknown
    fn loop_bars(&self, project_bpm: f64) -> u32 {
        let bpm = self.bpm.unwrap_or(project_bpm);
        // Sanity check: duration should be reasonable (< 5 minutes for a loop)
        let duration = if self.duration_secs <= 0.0 || self.duration_secs > 300.0 {
            // Fallback: assume 4 bars at project BPM
            (4.0 * 60.0 * 4.0) / project_bpm
        } else {
            self.duration_secs
        };

        if bpm <= 0.0 {
            return 4; // fallback
        }
        // bars = (duration_secs * bpm) / (60 * beats_per_bar)
        let bars = (duration * bpm) / (60.0 * 4.0);
        // Round to nearest power of 2 or common bar length (1, 2, 4, 8, 16)
        let bars_rounded = if bars <= 0.75 {
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
        };
        bars_rounded
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

/// Load sample info from database, including duration and BPM
fn load_samples_from_db(paths: &[&str]) -> Vec<SampleInfo> {
    let conn = match Connection::open(DB_PATH) {
        Ok(c) => c,
        Err(e) => {
            eprintln!(
                "Warning: Cannot open DB: {}, falling back to file metadata",
                e
            );
            return paths
                .iter()
                .filter_map(|p| SampleInfo::from_path(p).ok())
                .collect();
        }
    };

    paths
        .iter()
        .filter_map(|path| {
            // Query via audio_library which has the full paths
            let result: Result<(f64, Option<f64>), _> = conn.query_row(
                "SELECT COALESCE(s.duration, 0), s.bpm 
             FROM audio_library al 
             JOIN audio_samples s ON al.sample_id = s.id 
             WHERE al.path = ?",
                [path],
                |row| Ok((row.get(0)?, row.get(1)?)),
            );

            match result {
                Ok((duration, bpm)) => match SampleInfo::from_db(path, duration, bpm) {
                    Ok(info) => {
                        eprintln!(
                            "  {} - {:.2}s, {:?} BPM -> {} bars",
                            info.name,
                            info.duration_secs,
                            info.bpm,
                            info.loop_bars(PROJECT_BPM)
                        );
                        Some(info)
                    }
                    Err(_) => None,
                },
                Err(_) => {
                    eprintln!(
                        "  {} - not in DB, using file metadata",
                        Path::new(path)
                            .file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                    );
                    SampleInfo::from_path(path).ok()
                }
            }
        })
        .collect()
}

/// Query DB for N unique random samples matching patterns
/// include_patterns: list of keywords to match (OR)
/// exclude_patterns: list of keywords to exclude (AND NOT)
/// require_loop: if true, path must contain "loop"
fn query_unique_samples_v2(
    include_patterns: &[&str],
    exclude_patterns: &[&str],
    require_loop: bool,
    count: usize,
) -> Vec<SampleInfo> {
    let conn = match Connection::open(DB_PATH) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Warning: Cannot open DB: {}", e);
            return vec![];
        }
    };

    // Build include clause: (path LIKE '%kick%' OR path LIKE '%Kick%' OR ...)
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

    // Build exclude clause: AND path NOT LIKE '%impact%' AND ...
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

    let query = format!(
        "SELECT al.path, COALESCE(s.duration, 0), s.bpm 
         FROM audio_library al 
         JOIN audio_samples s ON al.sample_id = s.id 
         WHERE s.format = 'WAV' 
         AND ({})
         {}
         AND al.path LIKE '%MusicProduction/Samples%'
         AND al.path NOT LIKE '%Splice%'
         {}
         ORDER BY RANDOM() 
         LIMIT {}",
        include_clause,
        loop_clause,
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

    let samples: Vec<SampleInfo> = stmt
        .query_map([], |row| {
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
        .unwrap_or_default();

    eprintln!(
        "  Found {} unique samples for {:?}",
        samples.len(),
        include_patterns
    );
    samples
}

/// Legacy wrapper for simple queries
fn query_unique_samples(
    pattern_name: &str,
    pattern_path: &str,
    exclude_pattern: Option<&str>,
    count: usize,
) -> Vec<SampleInfo> {
    let includes = if pattern_path.is_empty() {
        vec![pattern_name]
    } else {
        vec![pattern_name, pattern_path]
    };
    let excludes: Vec<&str> = exclude_pattern.map(|e| vec![e]).unwrap_or_default();
    query_unique_samples_v2(&includes, &excludes, !pattern_path.is_empty(), count)
}

fn generate_techno_project(output_path: &Path) -> Result<(), String> {
    let ids = IdAllocator::new(1000000);

    // Load unique samples directly from DB - NO REUSE
    eprintln!("Loading {} unique KICK samples:", CLIPS_PER_TRACK);
    let kick_samples = query_unique_samples("kick", "loop", Some("Splice"), CLIPS_PER_TRACK);

    eprintln!("Loading {} unique CLAP/SNARE samples:", CLIPS_PER_TRACK);
    let clap_samples = query_unique_samples("clap", "loop", Some("Splice"), CLIPS_PER_TRACK);

    eprintln!("Loading {} unique HAT samples:", CLIPS_PER_TRACK);
    let hihat_samples = query_unique_samples("hat", "loop", Some("Splice"), CLIPS_PER_TRACK);

    eprintln!("Loading {} unique PERC samples:", CLIPS_PER_TRACK);
    let perc_samples = query_unique_samples("perc", "loop", Some("Splice"), CLIPS_PER_TRACK);

    eprintln!("Loading {} unique BASS samples:", CLIPS_PER_TRACK);
    let bass_samples = query_unique_samples("bass", "loop", Some("Splice"), CLIPS_PER_TRACK);

    eprintln!("Loading {} unique SYNTH samples:", CLIPS_PER_TRACK);
    let synth1_samples = query_unique_samples("synth", "loop", Some("Splice"), CLIPS_PER_TRACK);
    let synth2_samples = query_unique_samples("lead", "loop", Some("Splice"), CLIPS_PER_TRACK);

    eprintln!("Loading {} unique PAD samples:", CLIPS_PER_TRACK);
    let pad_samples = query_unique_samples("pad", "loop", Some("Splice"), CLIPS_PER_TRACK);

    eprintln!("Loading {} unique FX samples:", CLIPS_PER_TRACK);
    let riser_samples = query_unique_samples_v2(
        &["riser", "sweep", "whoosh", "build"], // include
        &["impact", "crash", "hit"],            // exclude oneshots
        false,
        CLIPS_PER_TRACK,
    );
    let hits_samples = query_unique_samples_v2(
        &["impact", "crash", "hit", "downlifter"], // oneshots
        &["loop", "riser", "sweep"],               // exclude loops/risers
        false,
        CLIPS_PER_TRACK,
    );
    let atmos_samples = query_unique_samples_v2(
        &["atmos", "texture", "drone", "ambient"],
        &["drum", "kick", "snare"],
        false,
        CLIPS_PER_TRACK,
    );

    generate_empty_als(output_path)?;

    let file = File::open(output_path).map_err(|e| e.to_string())?;
    let mut decoder = GzDecoder::new(file);
    let mut xml = String::new();
    decoder
        .read_to_string(&mut xml)
        .map_err(|e| e.to_string())?;

    // Reserve all IDs already in the template
    let id_re = Regex::new(r#"Id="(\d+)""#).unwrap();
    for cap in id_re.captures_iter(&xml) {
        if let Ok(id) = cap[1].parse::<u32>() {
            ids.reserve(id);
        }
    }

    // Extract original AudioTrack as template
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

    // Create audio tracks
    let kick_refs: Vec<&SampleInfo> = kick_samples.iter().collect();
    let clap_refs: Vec<&SampleInfo> = clap_samples.iter().collect();
    let hat_refs: Vec<&SampleInfo> = hihat_samples.iter().collect();
    let perc_refs: Vec<&SampleInfo> = perc_samples.iter().collect();
    let bass_refs: Vec<&SampleInfo> = bass_samples.iter().collect();
    let synth1_refs: Vec<&SampleInfo> = synth1_samples.iter().collect();
    let synth2_refs: Vec<&SampleInfo> = synth2_samples.iter().collect();
    let pad_refs: Vec<&SampleInfo> = pad_samples.iter().collect();
    let riser_refs: Vec<&SampleInfo> = riser_samples.iter().collect();
    let hits_refs: Vec<&SampleInfo> = hits_samples.iter().collect();
    let atmos_refs: Vec<&SampleInfo> = atmos_samples.iter().collect();

    // Create tracks - loop_bars now calculated per-sample from duration/BPM
    let kick_track = create_audio_track(
        &original_audio_track,
        "KICK",
        DRUMS_COLOR,
        drums_group_id as i32,
        &kick_refs,
        &ids,
    )?;
    let clap_track = create_audio_track(
        &original_audio_track,
        "CLAP",
        DRUMS_COLOR,
        drums_group_id as i32,
        &clap_refs,
        &ids,
    )?;
    let hat_track = create_audio_track(
        &original_audio_track,
        "HAT",
        DRUMS_COLOR,
        drums_group_id as i32,
        &hat_refs,
        &ids,
    )?;
    let perc_track = create_audio_track(
        &original_audio_track,
        "PERC",
        DRUMS_COLOR,
        drums_group_id as i32,
        &perc_refs,
        &ids,
    )?;
    let bass_track = create_audio_track(
        &original_audio_track,
        "BASS",
        BASS_COLOR,
        -1,
        &bass_refs,
        &ids,
    )?;
    let synth1_track = create_audio_track(
        &original_audio_track,
        "SYNTH 1",
        MELODICS_COLOR,
        melodics_group_id as i32,
        &synth1_refs,
        &ids,
    )?;
    let synth2_track = create_audio_track(
        &original_audio_track,
        "SYNTH 2",
        MELODICS_COLOR,
        melodics_group_id as i32,
        &synth2_refs,
        &ids,
    )?;
    let pad_track = create_audio_track(
        &original_audio_track,
        "PAD",
        MELODICS_COLOR,
        melodics_group_id as i32,
        &pad_refs,
        &ids,
    )?;
    let riser_track = create_audio_track(
        &original_audio_track,
        "RISER",
        FX_COLOR,
        fx_group_id as i32,
        &riser_refs,
        &ids,
    )?;
    let hits_track = create_audio_track(
        &original_audio_track,
        "HITS",
        FX_COLOR,
        fx_group_id as i32,
        &hits_refs,
        &ids,
    )?;
    let atmos_track = create_audio_track(
        &original_audio_track,
        "ATMOS",
        FX_COLOR,
        fx_group_id as i32,
        &atmos_refs,
        &ids,
    )?;

    // Build final XML
    let before_track = &xml[..track_start];
    let after_track = &xml[track_end..];

    let all_tracks = format!(
        "{}\n\t\t\t{}\n\t\t\t{}\n\t\t\t{}\n\t\t\t{}\n\t\t\t{}\n\t\t\t{}\n\t\t\t{}\n\t\t\t{}\n\t\t\t{}\n\t\t\t{}\n\t\t\t{}\n\t\t\t{}\n\t\t\t{}",
        drums_group,
        kick_track,
        clap_track,
        hat_track,
        perc_track,
        bass_track,
        melodics_group,
        synth1_track,
        synth2_track,
        pad_track,
        fx_group,
        riser_track,
        hits_track,
        atmos_track
    );

    let mut xml = format!("{}{}{}", before_track, all_tracks, after_track);

    // Update NextPointeeId
    let next_id = ids.max_id() + 1000;
    let next_id_re = Regex::new(r#"<NextPointeeId Value="\d+" />"#).unwrap();
    xml = next_id_re
        .replace(&xml, format!(r#"<NextPointeeId Value="{}" />"#, next_id))
        .to_string();

    // Hide mixer in arrangement view
    xml = xml.replace(
        r#"<MixerInArrangement Value="1" />"#,
        r#"<MixerInArrangement Value="0" />"#,
    );

    // Set project tempo to 128 BPM
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

    // Set tempo automation to 128
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

    eprintln!("Max ID used: {}", ids.max_id());
    Ok(())
}

/// Create an audio clip with proper warping
/// - start_bar: 1-indexed bar position
/// - length_bars: total clip length in arrangement
/// - loop_bars: actual sample/loop length (for proper looping)
fn create_audio_clip(
    sample: &SampleInfo,
    color: u32,
    clip_id: u32,
    start_bar: u32,
    length_bars: u32,
    loop_bars: u32,
) -> String {
    let beats_per_bar = 4;
    let start_beat = (start_bar - 1) * beats_per_bar;
    let length_beats = length_bars * beats_per_bar;
    let loop_beats = loop_bars * beats_per_bar;
    let end_beat = start_beat + length_beats;

    // Calculate warp marker SecTime based on sample duration
    // If we know the duration, use it; otherwise estimate from loop_bars at project BPM
    let sample_duration = if sample.duration_secs > 0.0 && sample.duration_secs < 300.0 {
        sample.duration_secs
    } else {
        // Estimate: loop_bars at 128 BPM = (loop_bars * 4 beats) / (128 BPM / 60) seconds
        (loop_bars as f64 * 4.0 * 60.0) / PROJECT_BPM
    };

    // WarpMarker: map sample's full duration to loop_beats
    // This tells Ableton to stretch/compress the sample to fit the beat grid
    let warp_sec_time = sample_duration;
    let warp_beat_time = loop_beats;

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
											<RightTime Value="{length_beats}" />
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
											<DefaultDuration Value="{length_beats}" />
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
											<WarpMarker Id="1" SecTime="{warp_sec_time}" BeatTime="{warp_beat_time}" />
										</WarpMarkers>
										<SavedWarpMarkersForStretched />
										<MarkersGenerated Value="true" />
										<IsSongTempoLeader Value="false" />
									</AudioClip>"#,
        clip_id = clip_id,
        start_beat = start_beat,
        end_beat = end_beat,
        length_beats = length_beats,
        name = sample.xml_name(),
        color = color,
        path = sample.xml_path(),
        file_size = sample.file_size,
        warp_sec_time = warp_sec_time,
        warp_beat_time = warp_beat_time
    )
}

fn create_group_track(
    name: &str,
    color: u32,
    group_id: u32,
    ids: &IdAllocator,
) -> Result<String, String> {
    let mut track = GROUP_TRACK_TEMPLATE.to_string();

    // Replace all IDs with fresh unique ones (except the main group ID which we set explicitly)
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

    // Now set the main GroupTrack ID
    let track_id_re = Regex::new(r#"<GroupTrack Id="\d+""#).unwrap();
    track = track_id_re
        .replace(&track, format!(r#"<GroupTrack Id="{}""#, group_id))
        .to_string();

    // Set name
    track = track.replace(
        r#"<EffectiveName Value="Drums" />"#,
        &format!(r#"<EffectiveName Value="{}" />"#, name),
    );
    track = track.replace(
        r#"<UserName Value="Drums" />"#,
        &format!(r#"<UserName Value="{}" />"#, name),
    );

    // Set color
    let color_re = Regex::new(r#"<Color Value="\d+" />"#).unwrap();
    track = color_re
        .replace_all(&track, format!(r#"<Color Value="{}" />"#, color))
        .to_string();

    eprintln!("GroupTrack {}: ID={}", name, group_id);
    Ok(track)
}

fn create_audio_track(
    template: &str,
    name: &str,
    color: u32,
    group_id: i32,
    samples: &[&SampleInfo],
    ids: &IdAllocator,
) -> Result<String, String> {
    let mut track = template.to_string();

    // Replace all IDs with fresh unique ones
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

    // Set audio output routing - if in a group, route to GroupTrack
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

    // Set track volume to -12dB (linear: 10^(-12/20) ≈ 0.251188643)
    let volume_re =
        Regex::new(r#"(<Volume>\s*<LomId Value="0" />\s*<Manual Value=")[^"]+(" />)"#).unwrap();
    track = volume_re
        .replace(&track, r#"${1}0.251188643${2}"#)
        .to_string();

    // Create clips - each 4 bars in arrangement, loop_bars calculated per-sample
    let clips: Vec<String> = samples
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let clip_id = ids.alloc();
            let start_bar = (i * 4 + 1) as u32;
            let loop_bars = s.loop_bars(PROJECT_BPM);
            create_audio_clip(s, color, clip_id, start_bar, 4, loop_bars)
        })
        .collect();

    let clips_xml = clips.join("\n");
    track = track.replacen(
        "<Events />",
        &format!(
            "<Events>\n{}\n\t\t\t\t\t\t\t\t\t\t\t\t\t</Events>",
            clips_xml
        ),
        1,
    );

    eprintln!(
        "AudioTrack {}: group={}, {} clips",
        name,
        group_id,
        samples.len()
    );
    Ok(track)
}
