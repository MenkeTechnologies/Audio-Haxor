//! Behavioral invariants: ordering, distances, and API contracts (not table grids).

use std::cmp::Ordering;

#[test]
fn kvr_compare_reflexive() {
    for v in ["0", "1", "1.0", "2.3.4", "10.20.30", "01.02.03"] {
        assert_eq!(app_lib::kvr::compare_versions(v, v), Ordering::Equal, "{v}");
    }
}

#[test]
fn kvr_compare_antisymmetric_pairs() {
    let pairs = [
        ("1.0", "2.0"),
        ("2.0", "1.0"),
        ("0.9", "1.0"),
        ("10.0", "9.99"),
        ("1.0.0", "1.0.1"),
    ];
    for (a, b) in pairs {
        let ab = app_lib::kvr::compare_versions(a, b);
        let ba = app_lib::kvr::compare_versions(b, a);
        match (ab, ba) {
            (Ordering::Equal, Ordering::Equal) => {}
            (Ordering::Less, Ordering::Greater) | (Ordering::Greater, Ordering::Less) => {}
            _ => panic!("compare({a},{b})={ab:?} compare({b},{a})={ba:?}"),
        }
    }
}

#[test]
fn kvr_parse_then_compare_matches_raw_compare() {
    use app_lib::kvr::{compare_versions, parse_version};
    let a = "3.14.15";
    let b = "3.14.16";
    assert_eq!(parse_version(a).len(), parse_version(b).len());
    assert_eq!(compare_versions(a, b), Ordering::Less);
}

#[test]
fn fingerprint_distance_nonnegative_and_symmetric() {
    use app_lib::similarity::{AudioFingerprint, fingerprint_distance};
    let mk = |path: &str, r: f64| AudioFingerprint {
        path: path.into(),
        rms: r,
        spectral_centroid: 1000.0,
        zero_crossing_rate: 0.05,
        low_band_energy: 0.1,
        mid_band_energy: 0.2,
        high_band_energy: 0.05,
        low_energy_ratio: 0.4,
        attack_time: 0.01,
    };
    let a = mk("a.wav", 0.3);
    let b = mk("b.wav", 0.7);
    let d_ab = fingerprint_distance(&a, &b);
    let d_ba = fingerprint_distance(&b, &a);
    assert!(d_ab >= 0.0);
    assert!((d_ab - d_ba).abs() < 1e-9);
    assert!(fingerprint_distance(&a, &a) < 1e-9);
}

#[test]
fn find_similar_orders_by_distance_and_excludes_self() {
    use app_lib::similarity::{AudioFingerprint, find_similar, fingerprint_distance};
    let mk = |path: &str, rms: f64| AudioFingerprint {
        path: path.into(),
        rms,
        spectral_centroid: 1000.0,
        zero_crossing_rate: 0.05,
        low_band_energy: 0.1,
        mid_band_energy: 0.2,
        high_band_energy: 0.05,
        low_energy_ratio: 0.4,
        attack_time: 0.01,
    };
    let reference = mk("/ref.wav", 0.5);
    let candidates = vec![
        mk("/ref.wav", 0.9),
        mk("/near.wav", 0.51),
        mk("/far.wav", 0.99),
    ];
    let out = find_similar(&reference, &candidates, 10);
    assert!(
        !out.iter().any(|(p, _)| p == "/ref.wav"),
        "should not score self"
    );
    assert!(!out.is_empty());
    for w in out.windows(2) {
        assert!(w[0].1 <= w[1].1 + 1e-9, "sorted by distance");
    }
    assert_eq!(out[0].0, "/near.wav");
    let d0 = fingerprint_distance(&reference, &mk("/near.wav", 0.51));
    assert!((out[0].1 - d0).abs() < 1e-9);
}

#[test]
fn normalize_plugin_name_idempotent_on_double_call() {
    let s = "Plugin (VST3) (x64)";
    let once = app_lib::xref::normalize_plugin_name(s);
    let twice = app_lib::xref::normalize_plugin_name(&once);
    assert_eq!(once, twice);
}

#[test]
fn format_size_nonzero_labels_for_large_powers_of_two() {
    for exp in [10u32, 15, 20, 30, 40] {
        let b = 1u64 << exp;
        let s = app_lib::format_size(b);
        assert_ne!(s, "0 B");
        assert!(
            s.ends_with(" B")
                || s.ends_with(" KB")
                || s.ends_with(" MB")
                || s.ends_with(" GB")
                || s.ends_with(" TB"),
            "{s}"
        );
    }
}

#[test]
fn history_gen_id_unique_in_batch() {
    let mut set = std::collections::HashSet::new();
    for _ in 0..50 {
        assert!(set.insert(app_lib::history::gen_id()));
    }
}
