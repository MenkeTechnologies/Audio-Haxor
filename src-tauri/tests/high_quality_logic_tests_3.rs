use app_lib::als_generator::{ClipPlacement, IdAllocatorPub, SampleInfo, TrackInfo};
use app_lib::als_project::{ElementConfig, Genre, MidiSettings, SectionLengths, TrackConfig};
use app_lib::midi_generator::LeadType;

// ── 1-20. Genre Equality and Properties ─────────────────────────────

#[test]
fn test_genre_eq_1() {
    assert_eq!(Genre::Techno, Genre::Techno);
}
#[test]
fn test_genre_eq_2() {
    assert_eq!(Genre::Trance, Genre::Trance);
}
#[test]
fn test_genre_eq_3() {
    assert_eq!(Genre::Schranz, Genre::Schranz);
}
#[test]
fn test_genre_ne_1() {
    assert_ne!(Genre::Techno, Genre::Trance);
}
#[test]
fn test_genre_ne_2() {
    assert_ne!(Genre::Techno, Genre::Schranz);
}
#[test]
fn test_genre_ne_3() {
    assert_ne!(Genre::Trance, Genre::Schranz);
}
#[test]
fn test_genre_clone_1() {
    let g = Genre::Techno;
    assert_eq!(g.clone(), g);
}
#[test]
fn test_genre_clone_2() {
    let g = Genre::Trance;
    assert_eq!(g.clone(), g);
}
#[test]
fn test_genre_clone_3() {
    let g = Genre::Schranz;
    assert_eq!(g.clone(), g);
}
#[test]
fn test_genre_bpm_diff_1() {
    assert_ne!(Genre::Techno.default_bpm(), Genre::Schranz.default_bpm());
}
#[test]
fn test_genre_bpm_diff_2() {
    assert_ne!(Genre::Trance.default_bpm(), Genre::Schranz.default_bpm());
}
#[test]
fn test_genre_range_diff() {
    assert_ne!(Genre::Techno.bpm_range(), Genre::Schranz.bpm_range());
}
#[test]
fn test_genre_debug_1() {
    assert_eq!(format!("{:?}", Genre::Techno), "Techno");
}
#[test]
fn test_genre_debug_2() {
    assert_eq!(format!("{:?}", Genre::Trance), "Trance");
}
#[test]
fn test_genre_debug_3() {
    assert_eq!(format!("{:?}", Genre::Schranz), "Schranz");
}
#[test]
fn test_genre_copy_1() {
    let g1 = Genre::Techno;
    let g2 = g1;
    assert_eq!(g1, g2);
}
#[test]
fn test_genre_copy_2() {
    let g1 = Genre::Trance;
    let g2 = g1;
    assert_eq!(g1, g2);
}
#[test]
fn test_genre_copy_3() {
    let g1 = Genre::Schranz;
    let g2 = g1;
    assert_eq!(g1, g2);
}
#[test]
fn test_genre_bpm_order() {
    assert!(Genre::Schranz.default_bpm() > Genre::Techno.default_bpm());
}
#[test]
fn test_genre_range_contains_default() {
    let (min, max) = Genre::Techno.bpm_range();
    let d = Genre::Techno.default_bpm();
    assert!(d >= min && d <= max);
}

// ── 21-40. SectionLengths and SectionStarts ──────────────────────────

#[test]
fn test_sl_default_intro() {
    assert_eq!(SectionLengths::default().intro, 32);
}
#[test]
fn test_sl_default_build() {
    assert_eq!(SectionLengths::default().build, 32);
}
#[test]
fn test_sl_default_breakdown() {
    assert_eq!(SectionLengths::default().breakdown, 32);
}
#[test]
fn test_sl_default_drop1() {
    assert_eq!(SectionLengths::default().drop1, 32);
}
#[test]
fn test_sl_default_drop2() {
    assert_eq!(SectionLengths::default().drop2, 32);
}
#[test]
fn test_sl_default_fadedown() {
    assert_eq!(SectionLengths::default().fadedown, 32);
}
#[test]
fn test_sl_default_outro() {
    assert_eq!(SectionLengths::default().outro, 32);
}
#[test]
fn test_sl_clone() {
    let s = SectionLengths::default();
    assert_eq!(s.clone().intro, s.intro);
}
#[test]
fn test_sl_starts_intro() {
    let s = SectionLengths::default().starts();
    assert_eq!(s.intro, (1, 33));
}
#[test]
fn test_sl_starts_build() {
    let s = SectionLengths::default().starts();
    assert_eq!(s.build, (33, 65));
}
#[test]
fn test_sl_starts_breakdown() {
    let s = SectionLengths::default().starts();
    assert_eq!(s.breakdown, (65, 97));
}
#[test]
fn test_sl_starts_drop1() {
    let s = SectionLengths::default().starts();
    assert_eq!(s.drop1, (97, 129));
}
#[test]
fn test_sl_starts_drop2() {
    let s = SectionLengths::default().starts();
    assert_eq!(s.drop2, (129, 161));
}
#[test]
fn test_sl_starts_fadedown() {
    let s = SectionLengths::default().starts();
    assert_eq!(s.fadedown, (161, 193));
}
#[test]
fn test_sl_starts_outro() {
    let s = SectionLengths::default().starts();
    assert_eq!(s.outro, (193, 225));
}
#[test]
fn test_sl_total_bars_init() {
    assert_eq!(SectionLengths::default().total_bars(), 224);
}
#[test]
fn test_sl_total_bars_custom() {
    let s = SectionLengths {
        intro: 8,
        build: 8,
        breakdown: 8,
        drop1: 8,
        drop2: 8,
        fadedown: 8,
        outro: 8,
    };
    assert_eq!(s.total_bars(), 56);
}
#[test]
fn test_sl_starts_total_bars() {
    let s = SectionLengths::default().starts();
    assert_eq!(s.total_bars(), 224);
}
#[test]
fn test_sl_starts_copy() {
    let s1 = SectionLengths::default().starts();
    let s2 = s1;
    assert_eq!(s1.intro, s2.intro);
}
#[test]
fn test_sl_starts_debug() {
    let s = SectionLengths::default().starts();
    assert!(!format!("{:?}", s).is_empty());
}

// ── 41-60. ElementConfig and TrackConfig ─────────────────────────────

#[test]
fn test_ec_fields() {
    let e = ElementConfig {
        count: 1,
        character: 0.5,
    };
    assert_eq!(e.count, 1);
    assert_eq!(e.character, 0.5);
}
#[test]
fn test_tc_default_drums() {
    assert_eq!(TrackConfig::default().drums.count, 3);
}
#[test]
fn test_tc_default_bass() {
    assert_eq!(TrackConfig::default().bass.count, 2);
}
#[test]
fn test_tc_default_leads() {
    assert_eq!(TrackConfig::default().leads.count, 2);
}
#[test]
fn test_tc_default_pads() {
    assert_eq!(TrackConfig::default().pads.count, 2);
}
#[test]
fn test_tc_default_fx() {
    assert_eq!(TrackConfig::default().fx.count, 6);
}
#[test]
fn test_tc_default_vocals() {
    assert_eq!(TrackConfig::default().vocals.count, 0);
}
#[test]
fn test_ec_clone() {
    let e = ElementConfig {
        count: 5,
        character: 0.1,
    };
    assert_eq!(e.clone().count, e.count);
}
#[test]
fn test_tc_clone() {
    let t = TrackConfig::default();
    assert_eq!(t.clone().drums.count, t.drums.count);
}
#[test]
fn test_ec_debug() {
    assert!(
        !format!(
            "{:?}",
            ElementConfig {
                count: 1,
                character: 0.5
            }
        )
        .is_empty()
    );
}
#[test]
fn test_tc_debug() {
    assert!(!format!("{:?}", TrackConfig::default()).is_empty());
}
#[test]
fn test_tc_leads_count() {
    assert_eq!(TrackConfig::default().leads.count, 2);
}
#[test]
fn test_tc_pads_count() {
    assert_eq!(TrackConfig::default().pads.count, 2);
}
#[test]
fn test_tc_fx_count() {
    assert_eq!(TrackConfig::default().fx.count, 6);
}
#[test]
fn test_tc_vocals_count() {
    assert_eq!(TrackConfig::default().vocals.count, 0);
}
#[test]
fn test_tc_bass_char() {
    assert_eq!(TrackConfig::default().bass.character, 0.5);
}
#[test]
fn test_tc_leads_char() {
    assert_eq!(TrackConfig::default().leads.character, 0.5);
}
#[test]
fn test_tc_pads_char() {
    assert_eq!(TrackConfig::default().pads.character, 0.5);
}
#[test]
fn test_tc_fx_char() {
    assert_eq!(TrackConfig::default().fx.character, 0.5);
}
#[test]
fn test_ec_count_set() {
    let e = ElementConfig {
        count: 10,
        character: 0.5,
    };
    assert_eq!(e.count, 10);
}

// ── 61-80. SampleInfo and ClipPlacement ──────────────────────────────

#[test]
fn test_si_eq() {
    let s1 = SampleInfo {
        path: "a".into(),
        name: "a".into(),
        duration_secs: 1.0,
        sample_rate: 44100,
        file_size: 100,
        bpm: None,
    };
    let s2 = s1.clone();
    assert_eq!(s1, s2);
}
#[test]
fn test_si_ne() {
    let s1 = SampleInfo {
        path: "a".into(),
        name: "a".into(),
        duration_secs: 1.0,
        sample_rate: 44100,
        file_size: 100,
        bpm: None,
    };
    let mut s2 = s1.clone();
    s2.path = "b".into();
    assert_ne!(s1, s2);
}
#[test]
fn test_si_duration() {
    let s = SampleInfo {
        path: "a".into(),
        name: "a".into(),
        duration_secs: 2.5,
        sample_rate: 44100,
        file_size: 100,
        bpm: None,
    };
    assert_eq!(s.duration_secs, 2.5);
}
#[test]
fn test_si_rate() {
    let s = SampleInfo {
        path: "a".into(),
        name: "a".into(),
        duration_secs: 1.0,
        sample_rate: 48000,
        file_size: 100,
        bpm: None,
    };
    assert_eq!(s.sample_rate, 48000);
}
#[test]
fn test_si_size() {
    let s = SampleInfo {
        path: "a".into(),
        name: "a".into(),
        duration_secs: 1.0,
        sample_rate: 44100,
        file_size: 1234,
        bpm: None,
    };
    assert_eq!(s.file_size, 1234);
}
#[test]
fn test_si_bpm() {
    let s = SampleInfo {
        path: "a".into(),
        name: "a".into(),
        duration_secs: 1.0,
        sample_rate: 44100,
        file_size: 100,
        bpm: Some(120.0),
    };
    assert_eq!(s.bpm, Some(120.0));
}
#[test]
fn test_cp_eq() {
    let s = SampleInfo {
        path: "a".into(),
        name: "a".into(),
        duration_secs: 1.0,
        sample_rate: 44100,
        file_size: 100,
        bpm: None,
    };
    let c1 = ClipPlacement {
        sample: s.clone(),
        start_beat: 0.0,
        duration_beats: 4.0,
    };
    let c2 = c1.clone();
    assert_eq!(c1, c2);
}
#[test]
fn test_cp_ne() {
    let s = SampleInfo {
        path: "a".into(),
        name: "a".into(),
        duration_secs: 1.0,
        sample_rate: 44100,
        file_size: 100,
        bpm: None,
    };
    let c1 = ClipPlacement {
        sample: s.clone(),
        start_beat: 0.0,
        duration_beats: 4.0,
    };
    let mut c2 = c1.clone();
    c2.start_beat = 1.0;
    assert_ne!(c1, c2);
}
#[test]
fn test_cp_duration() {
    let s = SampleInfo {
        path: "a".into(),
        name: "a".into(),
        duration_secs: 1.0,
        sample_rate: 44100,
        file_size: 100,
        bpm: None,
    };
    let c = ClipPlacement {
        sample: s,
        start_beat: 0.0,
        duration_beats: 8.0,
    };
    assert_eq!(c.duration_beats, 8.0);
}
#[test]
fn test_ti_eq() {
    let t1 = TrackInfo {
        name: "T1".into(),
        color: 1,
        clips: vec![],
    };
    let t2 = t1.clone();
    assert_eq!(t1, t2);
}
#[test]
fn test_ti_ne_name() {
    let t1 = TrackInfo {
        name: "T1".into(),
        color: 1,
        clips: vec![],
    };
    let mut t2 = t1.clone();
    t2.name = "T2".into();
    assert_ne!(t1, t2);
}
#[test]
fn test_ti_ne_color() {
    let t1 = TrackInfo {
        name: "T1".into(),
        color: 1,
        clips: vec![],
    };
    let mut t2 = t1.clone();
    t2.color = 2;
    assert_ne!(t1, t2);
}
#[test]
fn test_ti_ne_clips() {
    let s = SampleInfo {
        path: "a".into(),
        name: "a".into(),
        duration_secs: 1.0,
        sample_rate: 44100,
        file_size: 100,
        bpm: None,
    };
    let c = ClipPlacement {
        sample: s,
        start_beat: 0.0,
        duration_beats: 4.0,
    };
    let t1 = TrackInfo {
        name: "T1".into(),
        color: 1,
        clips: vec![],
    };
    let mut t2 = t1.clone();
    t2.clips.push(c);
    assert_ne!(t1, t2);
}
#[test]
fn test_si_clone_identity() {
    let s = SampleInfo {
        path: "a".into(),
        name: "a".into(),
        duration_secs: 1.0,
        sample_rate: 44100,
        file_size: 100,
        bpm: None,
    };
    assert_eq!(s.clone(), s);
}
#[test]
fn test_cp_clone_identity() {
    let s = SampleInfo {
        path: "a".into(),
        name: "a".into(),
        duration_secs: 1.0,
        sample_rate: 44100,
        file_size: 100,
        bpm: None,
    };
    let c = ClipPlacement {
        sample: s,
        start_beat: 0.0,
        duration_beats: 4.0,
    };
    assert_eq!(c.clone(), c);
}
#[test]
fn test_ti_clone_identity() {
    let t = TrackInfo {
        name: "a".into(),
        color: 1,
        clips: vec![],
    };
    assert_eq!(t.clone(), t);
}
#[test]
fn test_si_debug() {
    let s = SampleInfo {
        path: "a".into(),
        name: "a".into(),
        duration_secs: 1.0,
        sample_rate: 44100,
        file_size: 100,
        bpm: None,
    };
    assert!(!format!("{:?}", s).is_empty());
}
#[test]
fn test_cp_debug() {
    let s = SampleInfo {
        path: "a".into(),
        name: "a".into(),
        duration_secs: 1.0,
        sample_rate: 44100,
        file_size: 100,
        bpm: None,
    };
    let c = ClipPlacement {
        sample: s,
        start_beat: 0.0,
        duration_beats: 4.0,
    };
    assert!(!format!("{:?}", c).is_empty());
}
#[test]
fn test_ti_debug() {
    let t = TrackInfo {
        name: "a".into(),
        color: 1,
        clips: vec![],
    };
    assert!(!format!("{:?}", t).is_empty());
}

// ── 81-90. LeadType Properties ───────────────────────────────────────

#[test]
fn test_lt_eq_1() {
    assert_eq!(LeadType::TwoLayer, LeadType::TwoLayer);
}
#[test]
fn test_lt_eq_2() {
    assert_eq!(LeadType::ChordArp, LeadType::ChordArp);
}
#[test]
fn test_lt_ne_1() {
    assert_ne!(LeadType::TwoLayer, LeadType::ChordArp);
}
#[test]
fn test_lt_ne_2() {
    assert_ne!(LeadType::DeepBass, LeadType::ChordArp);
}
#[test]
fn test_lt_clone() {
    let l = LeadType::TwoLayer;
    assert_eq!(l.clone(), l);
}
#[test]
fn test_lt_copy() {
    let l1 = LeadType::TwoLayer;
    let l2 = l1;
    assert_eq!(l1, l2);
}
#[test]
fn test_lt_debug() {
    assert!(!format!("{:?}", LeadType::TwoLayer).is_empty());
}
#[test]
fn test_lt_all_different() {
    assert_ne!(LeadType::TwoLayer, LeadType::DeepBass);
    assert_ne!(LeadType::TwoLayer, LeadType::ChordArp);
    assert_ne!(LeadType::DeepBass, LeadType::ChordArp);
}
#[test]
fn test_lt_chordarp_short() {
    let c = app_lib::midi_generator::MidiGenConfig {
        key_root: 0,
        minor: true,
        lead_type: LeadType::ChordArp,
        chords: vec![0],
        progression: vec![],
        bpm: 140,
        bars_per_chord: 1,
        length_bars: None,
        chromaticism: 0,
        seed: 1,
        name: None,
        variations: None,
    };
    assert!(app_lib::midi_generator::build_base_name(&c).contains("ChordArp"));
}
#[test]
fn test_lt_deepbass_short() {
    let c = app_lib::midi_generator::MidiGenConfig {
        key_root: 0,
        minor: true,
        lead_type: LeadType::DeepBass,
        chords: vec![0],
        progression: vec![],
        bpm: 140,
        bars_per_chord: 1,
        length_bars: None,
        chromaticism: 0,
        seed: 1,
        name: None,
        variations: None,
    };
    assert!(app_lib::midi_generator::build_base_name(&c).contains("DeepBass"));
}

// ── 91-100. Misc Structs ─────────────────────────────────────────────

#[test]
fn test_ms_default_prog() {
    assert!(MidiSettings::default().progression.is_empty());
}
#[test]
fn test_ms_default_bars() {
    assert_eq!(MidiSettings::default().bars_per_chord, 0);
}
#[test]
fn test_ms_clone() {
    let m = MidiSettings::default();
    assert_eq!(m.clone().bars_per_chord, m.bars_per_chord);
}
#[test]
fn test_ms_debug() {
    assert!(!format!("{:?}", MidiSettings::default()).is_empty());
}
#[test]
fn test_id_alloc_init_0() {
    let mut a = IdAllocatorPub::new(0);
    assert_eq!(a.next(), 0);
    assert_eq!(a.next(), 1);
}
#[test]
fn test_id_alloc_init_large() {
    let mut a = IdAllocatorPub::new(1000000);
    assert_eq!(a.next(), 1000000);
}
#[test]
fn test_id_alloc_monotonic_large() {
    let mut a = IdAllocatorPub::new(100);
    let v1 = a.next();
    let v2 = a.next();
    let v3 = a.next();
    assert!(v3 > v2 && v2 > v1);
}
#[test]
fn test_id_alloc_max_val_no_next() {
    let a = IdAllocatorPub::new(500);
    assert_eq!(a.max_val(), 500);
}
#[test]
fn test_id_alloc_max_val_after_next() {
    let mut a = IdAllocatorPub::new(500);
    a.next();
    assert_eq!(a.max_val(), 501);
}
#[test]
fn test_id_alloc_sequence() {
    let mut a = IdAllocatorPub::new(1);
    let mut sum = 0;
    for _ in 0..10 {
        sum += a.next();
    }
    assert_eq!(sum, 1 + 2 + 3 + 4 + 5 + 6 + 7 + 8 + 9 + 10);
}
#[test]
fn test_id_alloc_next_sequence() {
    let mut a = IdAllocatorPub::new(100);
    a.next();
    assert_eq!(a.next(), 101);
}
