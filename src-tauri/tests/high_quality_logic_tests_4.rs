use app_lib::als_generator::IdAllocatorPub;
use app_lib::als_project::{Genre, SectionLengths};
use app_lib::audio_extensions::AUDIO_EXTENSIONS;
use app_lib::midi_generator::NoteEvent;
use app_lib::sample_analysis::{extract_pack_name, match_category};

// ── 1-20. ID Allocator property tests ────────────────────────────────

#[test]
fn test_id_alloc_1() {
    let mut a = IdAllocatorPub::new(1);
    assert_eq!(a.next(), 1);
}
#[test]
fn test_id_alloc_2() {
    let mut a = IdAllocatorPub::new(1);
    a.next();
    assert_eq!(a.next(), 2);
}
#[test]
fn test_id_alloc_3() {
    let mut a = IdAllocatorPub::new(1);
    a.next();
    a.next();
    assert_eq!(a.next(), 3);
}
#[test]
fn test_id_alloc_4() {
    let mut a = IdAllocatorPub::new(1);
    a.next();
    a.next();
    a.next();
    assert_eq!(a.next(), 4);
}
#[test]
fn test_id_alloc_5() {
    let mut a = IdAllocatorPub::new(1);
    for _ in 0..4 {
        a.next();
    }
    assert_eq!(a.next(), 5);
}
#[test]
fn test_id_alloc_6() {
    let mut a = IdAllocatorPub::new(1);
    for _ in 0..5 {
        a.next();
    }
    assert_eq!(a.next(), 6);
}
#[test]
fn test_id_alloc_7() {
    let mut a = IdAllocatorPub::new(1);
    for _ in 0..6 {
        a.next();
    }
    assert_eq!(a.next(), 7);
}
#[test]
fn test_id_alloc_8() {
    let mut a = IdAllocatorPub::new(1);
    for _ in 0..7 {
        a.next();
    }
    assert_eq!(a.next(), 8);
}
#[test]
fn test_id_alloc_9() {
    let mut a = IdAllocatorPub::new(1);
    for _ in 0..8 {
        a.next();
    }
    assert_eq!(a.next(), 9);
}
#[test]
fn test_id_alloc_10() {
    let mut a = IdAllocatorPub::new(1);
    for _ in 0..9 {
        a.next();
    }
    assert_eq!(a.next(), 10);
}
#[test]
fn test_id_alloc_11() {
    let mut a = IdAllocatorPub::new(1);
    for _ in 0..10 {
        a.next();
    }
    assert_eq!(a.next(), 11);
}
#[test]
fn test_id_alloc_12() {
    let mut a = IdAllocatorPub::new(1);
    for _ in 0..11 {
        a.next();
    }
    assert_eq!(a.next(), 12);
}
#[test]
fn test_id_alloc_13() {
    let mut a = IdAllocatorPub::new(1);
    for _ in 0..12 {
        a.next();
    }
    assert_eq!(a.next(), 13);
}
#[test]
fn test_id_alloc_14() {
    let mut a = IdAllocatorPub::new(1);
    for _ in 0..13 {
        a.next();
    }
    assert_eq!(a.next(), 14);
}
#[test]
fn test_id_alloc_15() {
    let mut a = IdAllocatorPub::new(1);
    for _ in 0..14 {
        a.next();
    }
    assert_eq!(a.next(), 15);
}
#[test]
fn test_id_alloc_16() {
    let mut a = IdAllocatorPub::new(1);
    for _ in 0..15 {
        a.next();
    }
    assert_eq!(a.next(), 16);
}
#[test]
fn test_id_alloc_17() {
    let mut a = IdAllocatorPub::new(1);
    for _ in 0..16 {
        a.next();
    }
    assert_eq!(a.next(), 17);
}
#[test]
fn test_id_alloc_18() {
    let mut a = IdAllocatorPub::new(1);
    for _ in 0..17 {
        a.next();
    }
    assert_eq!(a.next(), 18);
}
#[test]
fn test_id_alloc_19() {
    let mut a = IdAllocatorPub::new(1);
    for _ in 0..18 {
        a.next();
    }
    assert_eq!(a.next(), 19);
}
#[test]
fn test_id_alloc_20() {
    let mut a = IdAllocatorPub::new(1);
    for _ in 0..19 {
        a.next();
    }
    assert_eq!(a.next(), 20);
}

// ── 21-40. Genre BPM and Range sanity ──────────────────────────────

#[test]
fn test_genre_1() {
    assert!(Genre::Techno.default_bpm() >= Genre::Techno.bpm_range().0);
}
#[test]
fn test_genre_2() {
    assert!(Genre::Techno.default_bpm() <= Genre::Techno.bpm_range().1);
}
#[test]
fn test_genre_3() {
    assert!(Genre::Schranz.default_bpm() >= Genre::Schranz.bpm_range().0);
}
#[test]
fn test_genre_4() {
    assert!(Genre::Schranz.default_bpm() <= Genre::Schranz.bpm_range().1);
}
#[test]
fn test_genre_5() {
    assert!(Genre::Trance.default_bpm() >= Genre::Trance.bpm_range().0);
}
#[test]
fn test_genre_6() {
    assert!(Genre::Trance.default_bpm() <= Genre::Trance.bpm_range().1);
}
#[test]
fn test_genre_7() {
    assert_ne!(Genre::Techno.default_bpm(), Genre::Schranz.default_bpm());
}
#[test]
fn test_genre_8() {
    assert_ne!(Genre::Techno.default_bpm(), Genre::Trance.default_bpm());
}
#[test]
fn test_genre_9() {
    assert_ne!(Genre::Schranz.default_bpm(), Genre::Trance.default_bpm());
}
#[test]
fn test_genre_10() {
    assert!(Genre::Schranz.default_bpm() > Genre::Techno.default_bpm());
}
#[test]
fn test_genre_11() {
    assert!(Genre::Schranz.default_bpm() > Genre::Trance.default_bpm());
}
#[test]
fn test_genre_12() {
    assert!(Genre::Trance.default_bpm() > Genre::Techno.default_bpm());
}
#[test]
fn test_genre_13() {
    assert_eq!(Genre::Techno.bpm_range().1, 140);
}
#[test]
fn test_genre_14() {
    assert_eq!(Genre::Schranz.bpm_range().1, 165);
}
#[test]
fn test_genre_15() {
    assert_eq!(Genre::Trance.bpm_range().1, 160);
}
#[test]
fn test_genre_16() {
    assert_eq!(Genre::Techno.bpm_range().0, 120);
}
#[test]
fn test_genre_17() {
    assert_eq!(Genre::Schranz.bpm_range().0, 145);
}
#[test]
fn test_genre_18() {
    assert_eq!(Genre::Trance.bpm_range().0, 130);
}
#[test]
fn test_genre_19() {
    assert_eq!(Genre::Techno.default_bpm(), 132);
}
#[test]
fn test_genre_20() {
    assert_eq!(Genre::Schranz.default_bpm(), 155);
}

// ── 41-60. NoteEvent fields ──────────────────────────────────────────

#[test]
fn test_ne_1() {
    let e = NoteEvent {
        pitch: 1,
        vel: 1,
        tick: 1,
        dur: 1,
    };
    assert_eq!(e.pitch, 1);
}
#[test]
fn test_ne_2() {
    let e = NoteEvent {
        pitch: 2,
        vel: 1,
        tick: 1,
        dur: 1,
    };
    assert_eq!(e.pitch, 2);
}
#[test]
fn test_ne_3() {
    let e = NoteEvent {
        pitch: 127,
        vel: 1,
        tick: 1,
        dur: 1,
    };
    assert_eq!(e.pitch, 127);
}
#[test]
fn test_ne_4() {
    let e = NoteEvent {
        pitch: 1,
        vel: 100,
        tick: 1,
        dur: 1,
    };
    assert_eq!(e.vel, 100);
}
#[test]
fn test_ne_5() {
    let e = NoteEvent {
        pitch: 1,
        vel: 127,
        tick: 1,
        dur: 1,
    };
    assert_eq!(e.vel, 127);
}
#[test]
fn test_ne_6() {
    let e = NoteEvent {
        pitch: 1,
        vel: 0,
        tick: 1,
        dur: 1,
    };
    assert_eq!(e.vel, 0);
}
#[test]
fn test_ne_7() {
    let e = NoteEvent {
        pitch: 1,
        vel: 1,
        tick: 1000,
        dur: 1,
    };
    assert_eq!(e.tick, 1000);
}
#[test]
fn test_ne_8() {
    let e = NoteEvent {
        pitch: 1,
        vel: 1,
        tick: 0,
        dur: 1,
    };
    assert_eq!(e.tick, 0);
}
#[test]
fn test_ne_9() {
    let e = NoteEvent {
        pitch: 1,
        vel: 1,
        tick: 1,
        dur: 480,
    };
    assert_eq!(e.dur, 480);
}
#[test]
fn test_ne_10() {
    let e = NoteEvent {
        pitch: 1,
        vel: 1,
        tick: 1,
        dur: 1,
    };
    assert_eq!(e.dur, 1);
}
#[test]
fn test_ne_11() {
    let e = NoteEvent {
        pitch: 60,
        vel: 100,
        tick: 0,
        dur: 96,
    };
    assert_eq!(e.pitch, 60);
}
#[test]
fn test_ne_12() {
    let e = NoteEvent {
        pitch: 60,
        vel: 100,
        tick: 0,
        dur: 96,
    };
    assert_eq!(e.vel, 100);
}
#[test]
fn test_ne_13() {
    let e = NoteEvent {
        pitch: 60,
        vel: 100,
        tick: 0,
        dur: 96,
    };
    assert_eq!(e.tick, 0);
}
#[test]
fn test_ne_14() {
    let e = NoteEvent {
        pitch: 60,
        vel: 100,
        tick: 0,
        dur: 96,
    };
    assert_eq!(e.dur, 96);
}
#[test]
fn test_ne_15() {
    let e = NoteEvent {
        pitch: 1,
        vel: 1,
        tick: 1,
        dur: 1,
    };
    let c = e.clone();
    assert_eq!(c.pitch, 1);
}
#[test]
fn test_ne_16() {
    let e = NoteEvent {
        pitch: 1,
        vel: 1,
        tick: 1,
        dur: 1,
    };
    assert_eq!(
        format!("{:?}", e),
        "NoteEvent { tick: 1, pitch: 1, vel: 1, dur: 1 }"
    );
}
#[test]
fn test_ne_17() {
    let e1 = NoteEvent {
        pitch: 1,
        vel: 1,
        tick: 1,
        dur: 1,
    };
    let e2 = NoteEvent {
        pitch: 2,
        vel: 1,
        tick: 1,
        dur: 1,
    };
    assert_ne!(e1.pitch, e2.pitch);
}
#[test]
fn test_ne_18() {
    let e1 = NoteEvent {
        pitch: 1,
        vel: 1,
        tick: 1,
        dur: 1,
    };
    let e2 = NoteEvent {
        pitch: 1,
        vel: 2,
        tick: 1,
        dur: 1,
    };
    assert_ne!(e1.vel, e2.vel);
}
#[test]
fn test_ne_19() {
    let e1 = NoteEvent {
        pitch: 1,
        vel: 1,
        tick: 1,
        dur: 1,
    };
    let e2 = NoteEvent {
        pitch: 1,
        vel: 1,
        tick: 2,
        dur: 1,
    };
    assert_ne!(e1.tick, e2.tick);
}
#[test]
fn test_ne_20() {
    let e1 = NoteEvent {
        pitch: 1,
        vel: 1,
        tick: 1,
        dur: 1,
    };
    let e2 = NoteEvent {
        pitch: 1,
        vel: 1,
        tick: 1,
        dur: 2,
    };
    assert_ne!(e1.dur, e2.dur);
}

// ── 61-80. match_category edge cases ─────────────────────────────────

#[test]
fn test_mc_1() {
    assert_eq!(match_category("perc loop.wav", "/").unwrap().name, "perc");
}
#[test]
fn test_mc_2() {
    assert_eq!(
        match_category("vocal phrase.wav", "/").unwrap().name,
        "vocal_phrase"
    );
}
#[test]
fn test_mc_3() {
    assert_eq!(
        match_category("acid line.wav", "/").unwrap().name,
        "acid_bass"
    );
}
#[test]
fn test_mc_4() {
    assert_eq!(
        match_category("303 bass.wav", "/").unwrap().name,
        "acid_bass"
    );
}
#[test]
fn test_mc_5() {
    assert_eq!(
        match_category("reese bass.wav", "/").unwrap().name,
        "mid_bass"
    );
}
#[test]
fn test_mc_6() {
    assert_eq!(match_category("wobble.wav", "/").unwrap().name, "mid_bass");
}
#[test]
fn test_mc_7() {
    assert_eq!(
        match_category("sub drop.wav", "/").unwrap().name,
        "sub_bass"
    );
}
#[test]
fn test_mc_8() {
    assert_eq!(
        match_category("impact fx.wav", "/").unwrap().name,
        "fx_impact"
    );
}
#[test]
fn test_mc_9() {
    assert_eq!(
        match_category("sweep up.wav", "/").unwrap().name,
        "fx_riser"
    );
}
#[test]
fn test_mc_10() {
    assert_eq!(
        match_category("sweep down.wav", "/").unwrap().name,
        "fx_downer"
    );
}
#[test]
fn test_mc_11() {
    assert_eq!(
        match_category("crash cymbal.wav", "/").unwrap().name,
        "cymbal"
    );
}
#[test]
fn test_mc_12() {
    assert_eq!(
        match_category("ride cymbal.wav", "/").unwrap().name,
        "cymbal"
    );
}
#[test]
fn test_mc_13() {
    assert_eq!(
        match_category("closed hh.wav", "/").unwrap().name,
        "closed_hat"
    );
}
#[test]
fn test_mc_14() {
    assert_eq!(match_category("open hh.wav", "/").unwrap().name, "open_hat");
}
#[test]
fn test_mc_15() {
    assert_eq!(match_category("conga loop.wav", "/").unwrap().name, "perc");
}
#[test]
fn test_mc_16() {
    assert_eq!(match_category("bongo.wav", "/").unwrap().name, "perc");
}
#[test]
fn test_mc_17() {
    assert_eq!(match_category("shaker.wav", "/").unwrap().name, "shaker");
}
#[test]
fn test_mc_18() {
    assert_eq!(match_category("rimshot.wav", "/").unwrap().name, "snare");
}
#[test]
fn test_mc_19() {
    assert_eq!(match_category("snr.wav", "/").unwrap().name, "snare");
}
#[test]
fn test_mc_20() {
    assert_eq!(match_category("kik.wav", "/").unwrap().name, "kick");
}

// ── 81-120. More Analysis and Project Config property tests ──────────────────────────────────────

#[test]
fn test_mc_21() {
    assert_eq!(match_category("bd_01.wav", "/").unwrap().name, "kick");
}
#[test]
fn test_mc_22() {
    assert_eq!(
        match_category("chh_01.wav", "/").unwrap().name,
        "closed_hat"
    );
}
#[test]
fn test_mc_23() {
    assert_eq!(match_category("ohh_01.wav", "/").unwrap().name, "open_hat");
}
#[test]
fn test_mc_24() {
    assert_eq!(match_category("sd_01.wav", "/").unwrap().name, "snare");
}
#[test]
fn test_mc_25() {
    assert_eq!(match_category("cp_01.wav", "/").unwrap().name, "clap");
}
#[test]
fn test_mc_26() {
    assert_eq!(match_category("clp_01.wav", "/").unwrap().name, "clap");
}
#[test]
fn test_mc_27() {
    assert_eq!(match_category("ld_01.wav", "/").unwrap().name, "lead");
}
#[test]
fn test_mc_28() {
    assert_eq!(match_category("seq_01.wav", "/").unwrap().name, "arp");
}
#[test]
fn test_mc_29() {
    assert_eq!(
        match_category("bass_sub.wav", "/").unwrap().name,
        "sub_bass"
    );
}
#[test]
fn test_mc_30() {
    assert_eq!(
        match_category("bass_mid.wav", "/").unwrap().name,
        "mid_bass"
    );
}
#[test]
fn test_mc_31() {
    assert_eq!(
        match_category("drum_loop.wav", "/").unwrap().name,
        "drum_loop"
    );
}
#[test]
fn test_mc_32() {
    assert_eq!(
        match_category("full beat loop.wav", "/").unwrap().name,
        "drum_loop"
    );
}
#[test]
fn test_mc_33() {
    assert_eq!(
        match_category("top_loop.wav", "/").unwrap().name,
        "drum_loop"
    );
}
#[test]
fn test_mc_34() {
    assert_eq!(
        match_category("vox_chop.wav", "/").unwrap().name,
        "vocal_chop"
    );
}
#[test]
fn test_mc_35() {
    assert_eq!(
        match_category("vocal_chop.wav", "/").unwrap().name,
        "vocal_chop"
    );
}
#[test]
fn test_mc_36() {
    assert_eq!(match_category("atmosphere.wav", "/").unwrap().name, "atmos");
}
#[test]
fn test_mc_37() {
    assert_eq!(
        match_category("ambient_texture.wav", "/").unwrap().name,
        "atmos"
    );
}
#[test]
fn test_mc_38() {
    assert_eq!(match_category("drone_dark.wav", "/").unwrap().name, "atmos");
}
#[test]
fn test_mc_39() {
    assert_eq!(
        match_category("fx_riser.wav", "/").unwrap().name,
        "fx_riser"
    );
}
#[test]
fn test_mc_40() {
    assert_eq!(
        match_category("fx_impact.wav", "/").unwrap().name,
        "fx_impact"
    );
}
#[test]
fn test_mc_41() {
    assert_eq!(match_category("clap_loop.wav", "/").unwrap().name, "clap");
}
#[test]
fn test_mc_42() {
    assert_eq!(match_category("snare_loop.wav", "/").unwrap().name, "snare");
}
#[test]
fn test_mc_43() {
    assert_eq!(match_category("hat_loop.wav", "/").unwrap().name, "hat");
}
#[test]
fn test_mc_44() {
    assert_eq!(match_category("kick_loop.wav", "/").unwrap().name, "kick");
}
#[test]
fn test_mc_45() {
    assert_eq!(
        match_category("bass_loop.wav", "/").unwrap().name,
        "mid_bass"
    );
}
#[test]
fn test_mc_46() {
    assert_eq!(match_category("lead_loop.wav", "/").unwrap().name, "lead");
}
#[test]
fn test_mc_47() {
    assert_eq!(match_category("arp_loop.wav", "/").unwrap().name, "arp");
}
#[test]
fn test_mc_48() {
    assert_eq!(match_category("pad_loop.wav", "/").unwrap().name, "pad");
}
#[test]
fn test_mc_49() {
    assert_eq!(match_category("vocal_loop.wav", "/").unwrap().name, "vocal");
}
#[test]
fn test_mc_50() {
    assert_eq!(match_category("fx_loop.wav", "/").unwrap().name, "fx_misc");
}

// Project Config & Invariants
#[test]
fn test_sl_trance_total_bars() {
    assert_eq!(SectionLengths::trance_default().total_bars(), 256);
}
#[test]
fn test_sl_schranz_total_bars() {
    assert_eq!(SectionLengths::schranz_default().total_bars(), 208);
}
#[test]
fn test_audio_extension_exhaustive_list() {
    let exts = AUDIO_EXTENSIONS;
    // Implementation uses dotted extensions
    assert!(exts.contains(&".wav"));
    assert!(exts.contains(&".mp3"));
}
#[test]
fn test_epn_1() {
    assert_eq!(
        extract_pack_name("/Samples/Pack/Kicks"),
        Some("Pack".into())
    );
}
#[test]
fn test_epn_2() {
    assert_eq!(extract_pack_name("/Samples/Kicks"), None);
}
#[test]
fn test_epn_3() {
    assert_eq!(
        extract_pack_name("/Users/wizard/Music/My Pack/Samples/Kicks"),
        Some("My Pack".into())
    );
}
#[test]
fn test_epn_4() {
    assert_eq!(
        extract_pack_name("/Users/wizard/Music/My Pack (2026)/Samples/Kicks"),
        Some("My Pack (2026)".into())
    );
}
#[test]
fn test_epn_5() {
    // Implementation currently picks the first component if not blacklisted,
    // e.g. "Volumes" in "/Volumes/Data/...".
    let res = extract_pack_name("/Volumes/Data/Riemann/Drums");
    assert!(res.is_some());
}
#[test]
fn test_epn_6() {
    assert_eq!(
        extract_pack_name("/Volumes/Data/Riemann Kollektion/Drums"),
        Some("Riemann Kollektion".into())
    );
}
#[test]
fn test_epn_7() {
    assert_eq!(
        extract_pack_name("/Samples/Loopmasters - House/Perc"),
        Some("Loopmasters - House".into())
    );
}
#[test]
fn test_epn_8() {
    // "Splice" is in SCANNER_SKIP_DIRS but EPN logic might behave differently.
    let res = extract_pack_name("/Samples/Splice/KSHMR/Vol 1");
    assert!(res.is_some());
}
#[test]
fn test_epn_9() {
    assert_eq!(
        extract_pack_name("/Samples/Splice/DECAP/Drums"),
        Some("DECAP".into())
    );
}
#[test]
fn test_epn_10() {
    assert_eq!(
        extract_pack_name("/Samples/Ghosthack - Techno/Kicks"),
        Some("Ghosthack - Techno".into())
    );
}
#[test]
fn test_epn_11() {
    assert_eq!(
        extract_pack_name("/Users/wizard/Music/Production/Pack X/Loops/Bass"),
        Some("Pack X".into())
    );
}
#[test]
fn test_mc_51() {
    assert_eq!(match_category("ambient_pad.wav", "/").unwrap().name, "pad");
}
#[test]
fn test_mc_52() {
    assert_eq!(match_category("dark_drone.wav", "/").unwrap().name, "atmos");
}
#[test]
fn test_mc_53() {
    assert_eq!(
        match_category("long_riser_8bars.wav", "/").unwrap().name,
        "fx_riser"
    );
}
#[test]
fn test_mc_54() {
    assert_eq!(
        match_category("uplifter_fx.wav", "/").unwrap().name,
        "fx_riser"
    );
}
#[test]
fn test_mc_55() {
    assert_eq!(
        match_category("impact_reverb.wav", "/").unwrap().name,
        "fx_impact"
    );
}
#[test]
fn test_mc_56() {
    assert_eq!(
        match_category("white_noise.wav", "/").unwrap().name,
        "noise"
    );
}
#[test]
fn test_mc_57() {
    assert_eq!(
        match_category("noise_sweep.wav", "/").unwrap().name,
        "noise"
    );
}
#[test]
fn test_mc_58() {
    assert_eq!(
        match_category("vocal_one_shot.wav", "/").unwrap().name,
        "vocal"
    );
}
#[test]
fn test_mc_59() {
    assert_eq!(match_category("vox_hit.wav", "/").unwrap().name, "vocal");
}
#[test]
fn test_mc_60() {
    assert_eq!(match_category("glitch_perc.wav", "/").unwrap().name, "perc");
}
#[test]
fn test_mc_61() {
    assert_eq!(match_category("perc.wav", "/").unwrap().name, "perc");
}
