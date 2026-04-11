//! `similarity::fingerprint_distance` symmetry and self-distance — fixed feature vectors, varying RMS only.

use app_lib::similarity::{AudioFingerprint, fingerprint_distance};

fn fp(path: &str, rms: f64) -> AudioFingerprint {
    AudioFingerprint {
        path: path.into(),
        rms,
        spectral_centroid: 0.25,
        zero_crossing_rate: 0.04,
        low_band_energy: 0.33,
        mid_band_energy: 0.34,
        high_band_energy: 0.33,
        low_energy_ratio: 0.45,
        attack_time: 0.02,
    }
}

macro_rules! fp_symmetric_pair {
    ($($name:ident: $a:expr, $b:expr)*) => {
        $(
            #[test]
            fn $name() {
                let p = fp("a.wav", $a);
                let q = fp("b.wav", $b);
                let d_ab = fingerprint_distance(&p, &q);
                let d_ba = fingerprint_distance(&q, &p);
                assert!((d_ab - d_ba).abs() < 1e-9, "symmetry {} vs {}", d_ab, d_ba);
            }
        )*
    };
}

fp_symmetric_pair! {
    fp_sym_01: 0.0, 1.0
    fp_sym_02: 0.1, 0.9
    fp_sym_03: 0.25, 0.75
    fp_sym_04: 0.5, 0.5
    fp_sym_05: 0.01, 0.99
    fp_sym_06: 0.33, 0.66
    fp_sym_07: 0.001, 0.999
    fp_sym_08: 0.2, 0.8
    fp_sym_09: 0.4, 0.6
    fp_sym_10: 0.11, 0.89
}

#[test]
fn handcrafted_fingerprint_self_distance_near_zero() {
    let p = fp("self.wav", 0.42);
    assert!(fingerprint_distance(&p, &p) < 1e-9);
}

#[test]
fn handcrafted_fingerprint_different_paths_same_features_same_distance() {
    let p = fp("/a/x.wav", 0.5);
    let q = fp("/b/y.wav", 0.5);
    let d = fingerprint_distance(&p, &q);
    assert!(
        d < 1e-9,
        "distance should ignore path when features match: {}",
        d
    );
}

macro_rules! fp_symmetric_pair_sc {
    ($($name:ident: $sc_a:expr, $sc_b:expr)*) => {
        $(
            #[test]
            fn $name() {
                let mut p = fp("p.wav", 0.5);
                let mut q = fp("q.wav", 0.5);
                p.spectral_centroid = $sc_a;
                q.spectral_centroid = $sc_b;
                let d1 = fingerprint_distance(&p, &q);
                let d2 = fingerprint_distance(&q, &p);
                assert!((d1 - d2).abs() < 1e-9);
            }
        )*
    };
}

fp_symmetric_pair_sc! {
    fp_sc_01: 0.0, 1.0
    fp_sc_02: 0.1, 0.2
    fp_sc_03: 0.33, 0.77
    fp_sc_04: 0.5, 0.5
}
