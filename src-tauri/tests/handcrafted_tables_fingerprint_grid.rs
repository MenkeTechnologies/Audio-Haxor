//! Grid tests for `similarity::fingerprint_distance` — expected values from reference Python (same formula as Rust).
#![allow(clippy::excessive_precision)]

use app_lib::similarity::{fingerprint_distance, AudioFingerprint};

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

#[test]
fn fp_rms_00_00() {
    let a = fp("a.wav", 0.00);
    let b = fp("b.wav", 0.00);
    let d = fingerprint_distance(&a, &b);
    let want = 0.00000000000000000e+00;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_00_01() {
    let a = fp("a.wav", 0.00);
    let b = fp("b.wav", 0.05);
    let d = fingerprint_distance(&a, &b);
    let want = 5.00000000000000028e-02;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_00_02() {
    let a = fp("a.wav", 0.00);
    let b = fp("b.wav", 0.10);
    let d = fingerprint_distance(&a, &b);
    let want = 1.00000000000000006e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_00_03() {
    let a = fp("a.wav", 0.00);
    let b = fp("b.wav", 0.15);
    let d = fingerprint_distance(&a, &b);
    let want = 1.50000000000000022e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_00_04() {
    let a = fp("a.wav", 0.00);
    let b = fp("b.wav", 0.20);
    let d = fingerprint_distance(&a, &b);
    let want = 2.00000000000000011e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_00_05() {
    let a = fp("a.wav", 0.00);
    let b = fp("b.wav", 0.25);
    let d = fingerprint_distance(&a, &b);
    let want = 2.50000000000000000e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_00_06() {
    let a = fp("a.wav", 0.00);
    let b = fp("b.wav", 0.30);
    let d = fingerprint_distance(&a, &b);
    let want = 3.00000000000000044e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_00_07() {
    let a = fp("a.wav", 0.00);
    let b = fp("b.wav", 0.35);
    let d = fingerprint_distance(&a, &b);
    let want = 3.50000000000000033e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_00_08() {
    let a = fp("a.wav", 0.00);
    let b = fp("b.wav", 0.40);
    let d = fingerprint_distance(&a, &b);
    let want = 4.00000000000000022e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_00_09() {
    let a = fp("a.wav", 0.00);
    let b = fp("b.wav", 0.45);
    let d = fingerprint_distance(&a, &b);
    let want = 4.50000000000000011e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_00_10() {
    let a = fp("a.wav", 0.00);
    let b = fp("b.wav", 0.50);
    let d = fingerprint_distance(&a, &b);
    let want = 5.00000000000000000e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_00_11() {
    let a = fp("a.wav", 0.00);
    let b = fp("b.wav", 0.55);
    let d = fingerprint_distance(&a, &b);
    let want = 5.50000000000000044e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_00_12() {
    let a = fp("a.wav", 0.00);
    let b = fp("b.wav", 0.60);
    let d = fingerprint_distance(&a, &b);
    let want = 6.00000000000000089e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_00_13() {
    let a = fp("a.wav", 0.00);
    let b = fp("b.wav", 0.65);
    let d = fingerprint_distance(&a, &b);
    let want = 6.50000000000000022e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_00_14() {
    let a = fp("a.wav", 0.00);
    let b = fp("b.wav", 0.70);
    let d = fingerprint_distance(&a, &b);
    let want = 7.00000000000000067e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_00_15() {
    let a = fp("a.wav", 0.00);
    let b = fp("b.wav", 0.75);
    let d = fingerprint_distance(&a, &b);
    let want = 7.50000000000000000e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_00_16() {
    let a = fp("a.wav", 0.00);
    let b = fp("b.wav", 0.80);
    let d = fingerprint_distance(&a, &b);
    let want = 8.00000000000000044e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_00_17() {
    let a = fp("a.wav", 0.00);
    let b = fp("b.wav", 0.85);
    let d = fingerprint_distance(&a, &b);
    let want = 8.50000000000000089e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_00_18() {
    let a = fp("a.wav", 0.00);
    let b = fp("b.wav", 0.90);
    let d = fingerprint_distance(&a, &b);
    let want = 9.00000000000000022e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_00_19() {
    let a = fp("a.wav", 0.00);
    let b = fp("b.wav", 0.95);
    let d = fingerprint_distance(&a, &b);
    let want = 9.50000000000000067e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_00_20() {
    let a = fp("a.wav", 0.00);
    let b = fp("b.wav", 1.00);
    let d = fingerprint_distance(&a, &b);
    let want = 1.00000000000000000e+00;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_01_01() {
    let a = fp("a.wav", 0.05);
    let b = fp("b.wav", 0.05);
    let d = fingerprint_distance(&a, &b);
    let want = 0.00000000000000000e+00;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_01_02() {
    let a = fp("a.wav", 0.05);
    let b = fp("b.wav", 0.10);
    let d = fingerprint_distance(&a, &b);
    let want = 5.00000000000000028e-02;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_01_03() {
    let a = fp("a.wav", 0.05);
    let b = fp("b.wav", 0.15);
    let d = fingerprint_distance(&a, &b);
    let want = 1.00000000000000019e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_01_04() {
    let a = fp("a.wav", 0.05);
    let b = fp("b.wav", 0.20);
    let d = fingerprint_distance(&a, &b);
    let want = 1.50000000000000022e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_01_05() {
    let a = fp("a.wav", 0.05);
    let b = fp("b.wav", 0.25);
    let d = fingerprint_distance(&a, &b);
    let want = 2.00000000000000011e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_01_06() {
    let a = fp("a.wav", 0.05);
    let b = fp("b.wav", 0.30);
    let d = fingerprint_distance(&a, &b);
    let want = 2.50000000000000056e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_01_07() {
    let a = fp("a.wav", 0.05);
    let b = fp("b.wav", 0.35);
    let d = fingerprint_distance(&a, &b);
    let want = 3.00000000000000044e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_01_08() {
    let a = fp("a.wav", 0.05);
    let b = fp("b.wav", 0.40);
    let d = fingerprint_distance(&a, &b);
    let want = 3.50000000000000033e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_01_09() {
    let a = fp("a.wav", 0.05);
    let b = fp("b.wav", 0.45);
    let d = fingerprint_distance(&a, &b);
    let want = 4.00000000000000022e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_01_10() {
    let a = fp("a.wav", 0.05);
    let b = fp("b.wav", 0.50);
    let d = fingerprint_distance(&a, &b);
    let want = 4.50000000000000011e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_01_11() {
    let a = fp("a.wav", 0.05);
    let b = fp("b.wav", 0.55);
    let d = fingerprint_distance(&a, &b);
    let want = 5.00000000000000000e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_01_12() {
    let a = fp("a.wav", 0.05);
    let b = fp("b.wav", 0.60);
    let d = fingerprint_distance(&a, &b);
    let want = 5.50000000000000044e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_01_13() {
    let a = fp("a.wav", 0.05);
    let b = fp("b.wav", 0.65);
    let d = fingerprint_distance(&a, &b);
    let want = 5.99999999999999978e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_01_14() {
    let a = fp("a.wav", 0.05);
    let b = fp("b.wav", 0.70);
    let d = fingerprint_distance(&a, &b);
    let want = 6.50000000000000022e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_01_15() {
    let a = fp("a.wav", 0.05);
    let b = fp("b.wav", 0.75);
    let d = fingerprint_distance(&a, &b);
    let want = 6.99999999999999956e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_01_16() {
    let a = fp("a.wav", 0.05);
    let b = fp("b.wav", 0.80);
    let d = fingerprint_distance(&a, &b);
    let want = 7.50000000000000000e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_01_17() {
    let a = fp("a.wav", 0.05);
    let b = fp("b.wav", 0.85);
    let d = fingerprint_distance(&a, &b);
    let want = 8.00000000000000044e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_01_18() {
    let a = fp("a.wav", 0.05);
    let b = fp("b.wav", 0.90);
    let d = fingerprint_distance(&a, &b);
    let want = 8.49999999999999978e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_01_19() {
    let a = fp("a.wav", 0.05);
    let b = fp("b.wav", 0.95);
    let d = fingerprint_distance(&a, &b);
    let want = 9.00000000000000022e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_01_20() {
    let a = fp("a.wav", 0.05);
    let b = fp("b.wav", 1.00);
    let d = fingerprint_distance(&a, &b);
    let want = 9.49999999999999956e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_02_02() {
    let a = fp("a.wav", 0.10);
    let b = fp("b.wav", 0.10);
    let d = fingerprint_distance(&a, &b);
    let want = 0.00000000000000000e+00;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_02_03() {
    let a = fp("a.wav", 0.10);
    let b = fp("b.wav", 0.15);
    let d = fingerprint_distance(&a, &b);
    let want = 5.00000000000000167e-02;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_02_04() {
    let a = fp("a.wav", 0.10);
    let b = fp("b.wav", 0.20);
    let d = fingerprint_distance(&a, &b);
    let want = 1.00000000000000006e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_02_05() {
    let a = fp("a.wav", 0.10);
    let b = fp("b.wav", 0.25);
    let d = fingerprint_distance(&a, &b);
    let want = 1.49999999999999994e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_02_06() {
    let a = fp("a.wav", 0.10);
    let b = fp("b.wav", 0.30);
    let d = fingerprint_distance(&a, &b);
    let want = 2.00000000000000039e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_02_07() {
    let a = fp("a.wav", 0.10);
    let b = fp("b.wav", 0.35);
    let d = fingerprint_distance(&a, &b);
    let want = 2.50000000000000000e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_02_08() {
    let a = fp("a.wav", 0.10);
    let b = fp("b.wav", 0.40);
    let d = fingerprint_distance(&a, &b);
    let want = 3.00000000000000044e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_02_09() {
    let a = fp("a.wav", 0.10);
    let b = fp("b.wav", 0.45);
    let d = fingerprint_distance(&a, &b);
    let want = 3.49999999999999978e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_02_10() {
    let a = fp("a.wav", 0.10);
    let b = fp("b.wav", 0.50);
    let d = fingerprint_distance(&a, &b);
    let want = 4.00000000000000022e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_02_11() {
    let a = fp("a.wav", 0.10);
    let b = fp("b.wav", 0.55);
    let d = fingerprint_distance(&a, &b);
    let want = 4.50000000000000067e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_02_12() {
    let a = fp("a.wav", 0.10);
    let b = fp("b.wav", 0.60);
    let d = fingerprint_distance(&a, &b);
    let want = 5.00000000000000111e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_02_13() {
    let a = fp("a.wav", 0.10);
    let b = fp("b.wav", 0.65);
    let d = fingerprint_distance(&a, &b);
    let want = 5.50000000000000044e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_02_14() {
    let a = fp("a.wav", 0.10);
    let b = fp("b.wav", 0.70);
    let d = fingerprint_distance(&a, &b);
    let want = 6.00000000000000089e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_02_15() {
    let a = fp("a.wav", 0.10);
    let b = fp("b.wav", 0.75);
    let d = fingerprint_distance(&a, &b);
    let want = 6.50000000000000022e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_02_16() {
    let a = fp("a.wav", 0.10);
    let b = fp("b.wav", 0.80);
    let d = fingerprint_distance(&a, &b);
    let want = 7.00000000000000067e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_02_17() {
    let a = fp("a.wav", 0.10);
    let b = fp("b.wav", 0.85);
    let d = fingerprint_distance(&a, &b);
    let want = 7.50000000000000111e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_02_18() {
    let a = fp("a.wav", 0.10);
    let b = fp("b.wav", 0.90);
    let d = fingerprint_distance(&a, &b);
    let want = 8.00000000000000044e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_02_19() {
    let a = fp("a.wav", 0.10);
    let b = fp("b.wav", 0.95);
    let d = fingerprint_distance(&a, &b);
    let want = 8.50000000000000089e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_02_20() {
    let a = fp("a.wav", 0.10);
    let b = fp("b.wav", 1.00);
    let d = fingerprint_distance(&a, &b);
    let want = 9.00000000000000022e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_03_03() {
    let a = fp("a.wav", 0.15);
    let b = fp("b.wav", 0.15);
    let d = fingerprint_distance(&a, &b);
    let want = 0.00000000000000000e+00;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_03_04() {
    let a = fp("a.wav", 0.15);
    let b = fp("b.wav", 0.20);
    let d = fingerprint_distance(&a, &b);
    let want = 4.99999999999999889e-02;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_03_05() {
    let a = fp("a.wav", 0.15);
    let b = fp("b.wav", 0.25);
    let d = fingerprint_distance(&a, &b);
    let want = 9.99999999999999778e-02;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_03_06() {
    let a = fp("a.wav", 0.15);
    let b = fp("b.wav", 0.30);
    let d = fingerprint_distance(&a, &b);
    let want = 1.50000000000000022e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_03_07() {
    let a = fp("a.wav", 0.15);
    let b = fp("b.wav", 0.35);
    let d = fingerprint_distance(&a, &b);
    let want = 2.00000000000000011e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_03_08() {
    let a = fp("a.wav", 0.15);
    let b = fp("b.wav", 0.40);
    let d = fingerprint_distance(&a, &b);
    let want = 2.50000000000000000e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_03_09() {
    let a = fp("a.wav", 0.15);
    let b = fp("b.wav", 0.45);
    let d = fingerprint_distance(&a, &b);
    let want = 2.99999999999999989e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_03_10() {
    let a = fp("a.wav", 0.15);
    let b = fp("b.wav", 0.50);
    let d = fingerprint_distance(&a, &b);
    let want = 3.49999999999999978e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_03_11() {
    let a = fp("a.wav", 0.15);
    let b = fp("b.wav", 0.55);
    let d = fingerprint_distance(&a, &b);
    let want = 4.00000000000000022e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_03_12() {
    let a = fp("a.wav", 0.15);
    let b = fp("b.wav", 0.60);
    let d = fingerprint_distance(&a, &b);
    let want = 4.50000000000000067e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_03_13() {
    let a = fp("a.wav", 0.15);
    let b = fp("b.wav", 0.65);
    let d = fingerprint_distance(&a, &b);
    let want = 5.00000000000000000e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_03_14() {
    let a = fp("a.wav", 0.15);
    let b = fp("b.wav", 0.70);
    let d = fingerprint_distance(&a, &b);
    let want = 5.50000000000000044e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_03_15() {
    let a = fp("a.wav", 0.15);
    let b = fp("b.wav", 0.75);
    let d = fingerprint_distance(&a, &b);
    let want = 5.99999999999999978e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_03_16() {
    let a = fp("a.wav", 0.15);
    let b = fp("b.wav", 0.80);
    let d = fingerprint_distance(&a, &b);
    let want = 6.50000000000000022e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_03_17() {
    let a = fp("a.wav", 0.15);
    let b = fp("b.wav", 0.85);
    let d = fingerprint_distance(&a, &b);
    let want = 7.00000000000000067e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_03_18() {
    let a = fp("a.wav", 0.15);
    let b = fp("b.wav", 0.90);
    let d = fingerprint_distance(&a, &b);
    let want = 7.50000000000000000e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_03_19() {
    let a = fp("a.wav", 0.15);
    let b = fp("b.wav", 0.95);
    let d = fingerprint_distance(&a, &b);
    let want = 8.00000000000000044e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_03_20() {
    let a = fp("a.wav", 0.15);
    let b = fp("b.wav", 1.00);
    let d = fingerprint_distance(&a, &b);
    let want = 8.49999999999999978e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_04_04() {
    let a = fp("a.wav", 0.20);
    let b = fp("b.wav", 0.20);
    let d = fingerprint_distance(&a, &b);
    let want = 0.00000000000000000e+00;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_04_05() {
    let a = fp("a.wav", 0.20);
    let b = fp("b.wav", 0.25);
    let d = fingerprint_distance(&a, &b);
    let want = 4.99999999999999889e-02;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_04_06() {
    let a = fp("a.wav", 0.20);
    let b = fp("b.wav", 0.30);
    let d = fingerprint_distance(&a, &b);
    let want = 1.00000000000000033e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_04_07() {
    let a = fp("a.wav", 0.20);
    let b = fp("b.wav", 0.35);
    let d = fingerprint_distance(&a, &b);
    let want = 1.50000000000000022e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_04_08() {
    let a = fp("a.wav", 0.20);
    let b = fp("b.wav", 0.40);
    let d = fingerprint_distance(&a, &b);
    let want = 2.00000000000000011e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_04_09() {
    let a = fp("a.wav", 0.20);
    let b = fp("b.wav", 0.45);
    let d = fingerprint_distance(&a, &b);
    let want = 2.50000000000000000e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_04_10() {
    let a = fp("a.wav", 0.20);
    let b = fp("b.wav", 0.50);
    let d = fingerprint_distance(&a, &b);
    let want = 2.99999999999999989e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_04_11() {
    let a = fp("a.wav", 0.20);
    let b = fp("b.wav", 0.55);
    let d = fingerprint_distance(&a, &b);
    let want = 3.50000000000000033e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_04_12() {
    let a = fp("a.wav", 0.20);
    let b = fp("b.wav", 0.60);
    let d = fingerprint_distance(&a, &b);
    let want = 4.00000000000000078e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_04_13() {
    let a = fp("a.wav", 0.20);
    let b = fp("b.wav", 0.65);
    let d = fingerprint_distance(&a, &b);
    let want = 4.50000000000000011e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_04_14() {
    let a = fp("a.wav", 0.20);
    let b = fp("b.wav", 0.70);
    let d = fingerprint_distance(&a, &b);
    let want = 5.00000000000000000e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_04_15() {
    let a = fp("a.wav", 0.20);
    let b = fp("b.wav", 0.75);
    let d = fingerprint_distance(&a, &b);
    let want = 5.50000000000000044e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_04_16() {
    let a = fp("a.wav", 0.20);
    let b = fp("b.wav", 0.80);
    let d = fingerprint_distance(&a, &b);
    let want = 6.00000000000000089e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_04_17() {
    let a = fp("a.wav", 0.20);
    let b = fp("b.wav", 0.85);
    let d = fingerprint_distance(&a, &b);
    let want = 6.50000000000000133e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_04_18() {
    let a = fp("a.wav", 0.20);
    let b = fp("b.wav", 0.90);
    let d = fingerprint_distance(&a, &b);
    let want = 6.99999999999999956e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_04_19() {
    let a = fp("a.wav", 0.20);
    let b = fp("b.wav", 0.95);
    let d = fingerprint_distance(&a, &b);
    let want = 7.50000000000000000e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_04_20() {
    let a = fp("a.wav", 0.20);
    let b = fp("b.wav", 1.00);
    let d = fingerprint_distance(&a, &b);
    let want = 8.00000000000000044e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_05_05() {
    let a = fp("a.wav", 0.25);
    let b = fp("b.wav", 0.25);
    let d = fingerprint_distance(&a, &b);
    let want = 0.00000000000000000e+00;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_05_06() {
    let a = fp("a.wav", 0.25);
    let b = fp("b.wav", 0.30);
    let d = fingerprint_distance(&a, &b);
    let want = 5.00000000000000444e-02;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_05_07() {
    let a = fp("a.wav", 0.25);
    let b = fp("b.wav", 0.35);
    let d = fingerprint_distance(&a, &b);
    let want = 1.00000000000000033e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_05_08() {
    let a = fp("a.wav", 0.25);
    let b = fp("b.wav", 0.40);
    let d = fingerprint_distance(&a, &b);
    let want = 1.50000000000000022e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_05_09() {
    let a = fp("a.wav", 0.25);
    let b = fp("b.wav", 0.45);
    let d = fingerprint_distance(&a, &b);
    let want = 2.00000000000000011e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_05_10() {
    let a = fp("a.wav", 0.25);
    let b = fp("b.wav", 0.50);
    let d = fingerprint_distance(&a, &b);
    let want = 2.50000000000000000e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_05_11() {
    let a = fp("a.wav", 0.25);
    let b = fp("b.wav", 0.55);
    let d = fingerprint_distance(&a, &b);
    let want = 3.00000000000000044e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_05_12() {
    let a = fp("a.wav", 0.25);
    let b = fp("b.wav", 0.60);
    let d = fingerprint_distance(&a, &b);
    let want = 3.50000000000000089e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_05_13() {
    let a = fp("a.wav", 0.25);
    let b = fp("b.wav", 0.65);
    let d = fingerprint_distance(&a, &b);
    let want = 4.00000000000000022e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_05_14() {
    let a = fp("a.wav", 0.25);
    let b = fp("b.wav", 0.70);
    let d = fingerprint_distance(&a, &b);
    let want = 4.50000000000000067e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_05_15() {
    let a = fp("a.wav", 0.25);
    let b = fp("b.wav", 0.75);
    let d = fingerprint_distance(&a, &b);
    let want = 5.00000000000000000e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_05_16() {
    let a = fp("a.wav", 0.25);
    let b = fp("b.wav", 0.80);
    let d = fingerprint_distance(&a, &b);
    let want = 5.50000000000000044e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_05_17() {
    let a = fp("a.wav", 0.25);
    let b = fp("b.wav", 0.85);
    let d = fingerprint_distance(&a, &b);
    let want = 6.00000000000000089e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_05_18() {
    let a = fp("a.wav", 0.25);
    let b = fp("b.wav", 0.90);
    let d = fingerprint_distance(&a, &b);
    let want = 6.50000000000000022e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_05_19() {
    let a = fp("a.wav", 0.25);
    let b = fp("b.wav", 0.95);
    let d = fingerprint_distance(&a, &b);
    let want = 7.00000000000000067e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_05_20() {
    let a = fp("a.wav", 0.25);
    let b = fp("b.wav", 1.00);
    let d = fingerprint_distance(&a, &b);
    let want = 7.50000000000000000e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_06_06() {
    let a = fp("a.wav", 0.30);
    let b = fp("b.wav", 0.30);
    let d = fingerprint_distance(&a, &b);
    let want = 0.00000000000000000e+00;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_06_07() {
    let a = fp("a.wav", 0.30);
    let b = fp("b.wav", 0.35);
    let d = fingerprint_distance(&a, &b);
    let want = 4.99999999999999889e-02;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_06_08() {
    let a = fp("a.wav", 0.30);
    let b = fp("b.wav", 0.40);
    let d = fingerprint_distance(&a, &b);
    let want = 9.99999999999999778e-02;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_06_09() {
    let a = fp("a.wav", 0.30);
    let b = fp("b.wav", 0.45);
    let d = fingerprint_distance(&a, &b);
    let want = 1.49999999999999967e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_06_10() {
    let a = fp("a.wav", 0.30);
    let b = fp("b.wav", 0.50);
    let d = fingerprint_distance(&a, &b);
    let want = 1.99999999999999956e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_06_11() {
    let a = fp("a.wav", 0.30);
    let b = fp("b.wav", 0.55);
    let d = fingerprint_distance(&a, &b);
    let want = 2.50000000000000000e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_06_12() {
    let a = fp("a.wav", 0.30);
    let b = fp("b.wav", 0.60);
    let d = fingerprint_distance(&a, &b);
    let want = 3.00000000000000044e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_06_13() {
    let a = fp("a.wav", 0.30);
    let b = fp("b.wav", 0.65);
    let d = fingerprint_distance(&a, &b);
    let want = 3.49999999999999978e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_06_14() {
    let a = fp("a.wav", 0.30);
    let b = fp("b.wav", 0.70);
    let d = fingerprint_distance(&a, &b);
    let want = 4.00000000000000022e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_06_15() {
    let a = fp("a.wav", 0.30);
    let b = fp("b.wav", 0.75);
    let d = fingerprint_distance(&a, &b);
    let want = 4.49999999999999956e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_06_16() {
    let a = fp("a.wav", 0.30);
    let b = fp("b.wav", 0.80);
    let d = fingerprint_distance(&a, &b);
    let want = 5.00000000000000000e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_06_17() {
    let a = fp("a.wav", 0.30);
    let b = fp("b.wav", 0.85);
    let d = fingerprint_distance(&a, &b);
    let want = 5.50000000000000044e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_06_18() {
    let a = fp("a.wav", 0.30);
    let b = fp("b.wav", 0.90);
    let d = fingerprint_distance(&a, &b);
    let want = 5.99999999999999978e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_06_19() {
    let a = fp("a.wav", 0.30);
    let b = fp("b.wav", 0.95);
    let d = fingerprint_distance(&a, &b);
    let want = 6.50000000000000022e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_06_20() {
    let a = fp("a.wav", 0.30);
    let b = fp("b.wav", 1.00);
    let d = fingerprint_distance(&a, &b);
    let want = 6.99999999999999956e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_07_07() {
    let a = fp("a.wav", 0.35);
    let b = fp("b.wav", 0.35);
    let d = fingerprint_distance(&a, &b);
    let want = 0.00000000000000000e+00;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_07_08() {
    let a = fp("a.wav", 0.35);
    let b = fp("b.wav", 0.40);
    let d = fingerprint_distance(&a, &b);
    let want = 4.99999999999999889e-02;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_07_09() {
    let a = fp("a.wav", 0.35);
    let b = fp("b.wav", 0.45);
    let d = fingerprint_distance(&a, &b);
    let want = 9.99999999999999778e-02;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_07_10() {
    let a = fp("a.wav", 0.35);
    let b = fp("b.wav", 0.50);
    let d = fingerprint_distance(&a, &b);
    let want = 1.49999999999999967e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_07_11() {
    let a = fp("a.wav", 0.35);
    let b = fp("b.wav", 0.55);
    let d = fingerprint_distance(&a, &b);
    let want = 2.00000000000000011e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_07_12() {
    let a = fp("a.wav", 0.35);
    let b = fp("b.wav", 0.60);
    let d = fingerprint_distance(&a, &b);
    let want = 2.50000000000000056e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_07_13() {
    let a = fp("a.wav", 0.35);
    let b = fp("b.wav", 0.65);
    let d = fingerprint_distance(&a, &b);
    let want = 2.99999999999999989e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_07_14() {
    let a = fp("a.wav", 0.35);
    let b = fp("b.wav", 0.70);
    let d = fingerprint_distance(&a, &b);
    let want = 3.50000000000000033e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_07_15() {
    let a = fp("a.wav", 0.35);
    let b = fp("b.wav", 0.75);
    let d = fingerprint_distance(&a, &b);
    let want = 3.99999999999999967e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_07_16() {
    let a = fp("a.wav", 0.35);
    let b = fp("b.wav", 0.80);
    let d = fingerprint_distance(&a, &b);
    let want = 4.50000000000000011e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_07_17() {
    let a = fp("a.wav", 0.35);
    let b = fp("b.wav", 0.85);
    let d = fingerprint_distance(&a, &b);
    let want = 5.00000000000000000e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_07_18() {
    let a = fp("a.wav", 0.35);
    let b = fp("b.wav", 0.90);
    let d = fingerprint_distance(&a, &b);
    let want = 5.50000000000000044e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_07_19() {
    let a = fp("a.wav", 0.35);
    let b = fp("b.wav", 0.95);
    let d = fingerprint_distance(&a, &b);
    let want = 6.00000000000000089e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_07_20() {
    let a = fp("a.wav", 0.35);
    let b = fp("b.wav", 1.00);
    let d = fingerprint_distance(&a, &b);
    let want = 6.49999999999999911e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_08_08() {
    let a = fp("a.wav", 0.40);
    let b = fp("b.wav", 0.40);
    let d = fingerprint_distance(&a, &b);
    let want = 0.00000000000000000e+00;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_08_09() {
    let a = fp("a.wav", 0.40);
    let b = fp("b.wav", 0.45);
    let d = fingerprint_distance(&a, &b);
    let want = 4.99999999999999889e-02;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_08_10() {
    let a = fp("a.wav", 0.40);
    let b = fp("b.wav", 0.50);
    let d = fingerprint_distance(&a, &b);
    let want = 9.99999999999999778e-02;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_08_11() {
    let a = fp("a.wav", 0.40);
    let b = fp("b.wav", 0.55);
    let d = fingerprint_distance(&a, &b);
    let want = 1.50000000000000022e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_08_12() {
    let a = fp("a.wav", 0.40);
    let b = fp("b.wav", 0.60);
    let d = fingerprint_distance(&a, &b);
    let want = 2.00000000000000067e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_08_13() {
    let a = fp("a.wav", 0.40);
    let b = fp("b.wav", 0.65);
    let d = fingerprint_distance(&a, &b);
    let want = 2.50000000000000000e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_08_14() {
    let a = fp("a.wav", 0.40);
    let b = fp("b.wav", 0.70);
    let d = fingerprint_distance(&a, &b);
    let want = 3.00000000000000044e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_08_15() {
    let a = fp("a.wav", 0.40);
    let b = fp("b.wav", 0.75);
    let d = fingerprint_distance(&a, &b);
    let want = 3.49999999999999978e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_08_16() {
    let a = fp("a.wav", 0.40);
    let b = fp("b.wav", 0.80);
    let d = fingerprint_distance(&a, &b);
    let want = 4.00000000000000022e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_08_17() {
    let a = fp("a.wav", 0.40);
    let b = fp("b.wav", 0.85);
    let d = fingerprint_distance(&a, &b);
    let want = 4.50000000000000067e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_08_18() {
    let a = fp("a.wav", 0.40);
    let b = fp("b.wav", 0.90);
    let d = fingerprint_distance(&a, &b);
    let want = 5.00000000000000000e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_08_19() {
    let a = fp("a.wav", 0.40);
    let b = fp("b.wav", 0.95);
    let d = fingerprint_distance(&a, &b);
    let want = 5.50000000000000044e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_08_20() {
    let a = fp("a.wav", 0.40);
    let b = fp("b.wav", 1.00);
    let d = fingerprint_distance(&a, &b);
    let want = 5.99999999999999978e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_09_09() {
    let a = fp("a.wav", 0.45);
    let b = fp("b.wav", 0.45);
    let d = fingerprint_distance(&a, &b);
    let want = 0.00000000000000000e+00;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_09_10() {
    let a = fp("a.wav", 0.45);
    let b = fp("b.wav", 0.50);
    let d = fingerprint_distance(&a, &b);
    let want = 4.99999999999999889e-02;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_09_11() {
    let a = fp("a.wav", 0.45);
    let b = fp("b.wav", 0.55);
    let d = fingerprint_distance(&a, &b);
    let want = 1.00000000000000033e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_09_12() {
    let a = fp("a.wav", 0.45);
    let b = fp("b.wav", 0.60);
    let d = fingerprint_distance(&a, &b);
    let want = 1.50000000000000078e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_09_13() {
    let a = fp("a.wav", 0.45);
    let b = fp("b.wav", 0.65);
    let d = fingerprint_distance(&a, &b);
    let want = 2.00000000000000011e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_09_14() {
    let a = fp("a.wav", 0.45);
    let b = fp("b.wav", 0.70);
    let d = fingerprint_distance(&a, &b);
    let want = 2.50000000000000056e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_09_15() {
    let a = fp("a.wav", 0.45);
    let b = fp("b.wav", 0.75);
    let d = fingerprint_distance(&a, &b);
    let want = 2.99999999999999989e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_09_16() {
    let a = fp("a.wav", 0.45);
    let b = fp("b.wav", 0.80);
    let d = fingerprint_distance(&a, &b);
    let want = 3.50000000000000033e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_09_17() {
    let a = fp("a.wav", 0.45);
    let b = fp("b.wav", 0.85);
    let d = fingerprint_distance(&a, &b);
    let want = 4.00000000000000078e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_09_18() {
    let a = fp("a.wav", 0.45);
    let b = fp("b.wav", 0.90);
    let d = fingerprint_distance(&a, &b);
    let want = 4.50000000000000011e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_09_19() {
    let a = fp("a.wav", 0.45);
    let b = fp("b.wav", 0.95);
    let d = fingerprint_distance(&a, &b);
    let want = 5.00000000000000000e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_09_20() {
    let a = fp("a.wav", 0.45);
    let b = fp("b.wav", 1.00);
    let d = fingerprint_distance(&a, &b);
    let want = 5.50000000000000044e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_10_10() {
    let a = fp("a.wav", 0.50);
    let b = fp("b.wav", 0.50);
    let d = fingerprint_distance(&a, &b);
    let want = 0.00000000000000000e+00;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_10_11() {
    let a = fp("a.wav", 0.50);
    let b = fp("b.wav", 0.55);
    let d = fingerprint_distance(&a, &b);
    let want = 5.00000000000000444e-02;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_10_12() {
    let a = fp("a.wav", 0.50);
    let b = fp("b.wav", 0.60);
    let d = fingerprint_distance(&a, &b);
    let want = 1.00000000000000089e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_10_13() {
    let a = fp("a.wav", 0.50);
    let b = fp("b.wav", 0.65);
    let d = fingerprint_distance(&a, &b);
    let want = 1.50000000000000022e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_10_14() {
    let a = fp("a.wav", 0.50);
    let b = fp("b.wav", 0.70);
    let d = fingerprint_distance(&a, &b);
    let want = 2.00000000000000067e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_10_15() {
    let a = fp("a.wav", 0.50);
    let b = fp("b.wav", 0.75);
    let d = fingerprint_distance(&a, &b);
    let want = 2.50000000000000000e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_10_16() {
    let a = fp("a.wav", 0.50);
    let b = fp("b.wav", 0.80);
    let d = fingerprint_distance(&a, &b);
    let want = 3.00000000000000044e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_10_17() {
    let a = fp("a.wav", 0.50);
    let b = fp("b.wav", 0.85);
    let d = fingerprint_distance(&a, &b);
    let want = 3.50000000000000089e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_10_18() {
    let a = fp("a.wav", 0.50);
    let b = fp("b.wav", 0.90);
    let d = fingerprint_distance(&a, &b);
    let want = 4.00000000000000022e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_10_19() {
    let a = fp("a.wav", 0.50);
    let b = fp("b.wav", 0.95);
    let d = fingerprint_distance(&a, &b);
    let want = 4.50000000000000067e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_10_20() {
    let a = fp("a.wav", 0.50);
    let b = fp("b.wav", 1.00);
    let d = fingerprint_distance(&a, &b);
    let want = 5.00000000000000000e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_11_11() {
    let a = fp("a.wav", 0.55);
    let b = fp("b.wav", 0.55);
    let d = fingerprint_distance(&a, &b);
    let want = 0.00000000000000000e+00;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_11_12() {
    let a = fp("a.wav", 0.55);
    let b = fp("b.wav", 0.60);
    let d = fingerprint_distance(&a, &b);
    let want = 5.00000000000000444e-02;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_11_13() {
    let a = fp("a.wav", 0.55);
    let b = fp("b.wav", 0.65);
    let d = fingerprint_distance(&a, &b);
    let want = 9.99999999999999778e-02;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_11_14() {
    let a = fp("a.wav", 0.55);
    let b = fp("b.wav", 0.70);
    let d = fingerprint_distance(&a, &b);
    let want = 1.50000000000000022e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_11_15() {
    let a = fp("a.wav", 0.55);
    let b = fp("b.wav", 0.75);
    let d = fingerprint_distance(&a, &b);
    let want = 1.99999999999999956e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_11_16() {
    let a = fp("a.wav", 0.55);
    let b = fp("b.wav", 0.80);
    let d = fingerprint_distance(&a, &b);
    let want = 2.50000000000000000e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_11_17() {
    let a = fp("a.wav", 0.55);
    let b = fp("b.wav", 0.85);
    let d = fingerprint_distance(&a, &b);
    let want = 3.00000000000000044e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_11_18() {
    let a = fp("a.wav", 0.55);
    let b = fp("b.wav", 0.90);
    let d = fingerprint_distance(&a, &b);
    let want = 3.49999999999999978e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_11_19() {
    let a = fp("a.wav", 0.55);
    let b = fp("b.wav", 0.95);
    let d = fingerprint_distance(&a, &b);
    let want = 4.00000000000000022e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_11_20() {
    let a = fp("a.wav", 0.55);
    let b = fp("b.wav", 1.00);
    let d = fingerprint_distance(&a, &b);
    let want = 4.49999999999999956e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_12_12() {
    let a = fp("a.wav", 0.60);
    let b = fp("b.wav", 0.60);
    let d = fingerprint_distance(&a, &b);
    let want = 0.00000000000000000e+00;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_12_13() {
    let a = fp("a.wav", 0.60);
    let b = fp("b.wav", 0.65);
    let d = fingerprint_distance(&a, &b);
    let want = 4.99999999999999334e-02;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_12_14() {
    let a = fp("a.wav", 0.60);
    let b = fp("b.wav", 0.70);
    let d = fingerprint_distance(&a, &b);
    let want = 9.99999999999999778e-02;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_12_15() {
    let a = fp("a.wav", 0.60);
    let b = fp("b.wav", 0.75);
    let d = fingerprint_distance(&a, &b);
    let want = 1.49999999999999911e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_12_16() {
    let a = fp("a.wav", 0.60);
    let b = fp("b.wav", 0.80);
    let d = fingerprint_distance(&a, &b);
    let want = 1.99999999999999956e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_12_17() {
    let a = fp("a.wav", 0.60);
    let b = fp("b.wav", 0.85);
    let d = fingerprint_distance(&a, &b);
    let want = 2.50000000000000000e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_12_18() {
    let a = fp("a.wav", 0.60);
    let b = fp("b.wav", 0.90);
    let d = fingerprint_distance(&a, &b);
    let want = 2.99999999999999933e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_12_19() {
    let a = fp("a.wav", 0.60);
    let b = fp("b.wav", 0.95);
    let d = fingerprint_distance(&a, &b);
    let want = 3.49999999999999978e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_12_20() {
    let a = fp("a.wav", 0.60);
    let b = fp("b.wav", 1.00);
    let d = fingerprint_distance(&a, &b);
    let want = 3.99999999999999911e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_13_13() {
    let a = fp("a.wav", 0.65);
    let b = fp("b.wav", 0.65);
    let d = fingerprint_distance(&a, &b);
    let want = 0.00000000000000000e+00;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_13_14() {
    let a = fp("a.wav", 0.65);
    let b = fp("b.wav", 0.70);
    let d = fingerprint_distance(&a, &b);
    let want = 5.00000000000000444e-02;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_13_15() {
    let a = fp("a.wav", 0.65);
    let b = fp("b.wav", 0.75);
    let d = fingerprint_distance(&a, &b);
    let want = 9.99999999999999778e-02;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_13_16() {
    let a = fp("a.wav", 0.65);
    let b = fp("b.wav", 0.80);
    let d = fingerprint_distance(&a, &b);
    let want = 1.50000000000000022e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_13_17() {
    let a = fp("a.wav", 0.65);
    let b = fp("b.wav", 0.85);
    let d = fingerprint_distance(&a, &b);
    let want = 2.00000000000000067e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_13_18() {
    let a = fp("a.wav", 0.65);
    let b = fp("b.wav", 0.90);
    let d = fingerprint_distance(&a, &b);
    let want = 2.50000000000000000e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_13_19() {
    let a = fp("a.wav", 0.65);
    let b = fp("b.wav", 0.95);
    let d = fingerprint_distance(&a, &b);
    let want = 3.00000000000000044e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_13_20() {
    let a = fp("a.wav", 0.65);
    let b = fp("b.wav", 1.00);
    let d = fingerprint_distance(&a, &b);
    let want = 3.49999999999999978e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_14_14() {
    let a = fp("a.wav", 0.70);
    let b = fp("b.wav", 0.70);
    let d = fingerprint_distance(&a, &b);
    let want = 0.00000000000000000e+00;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_14_15() {
    let a = fp("a.wav", 0.70);
    let b = fp("b.wav", 0.75);
    let d = fingerprint_distance(&a, &b);
    let want = 4.99999999999999334e-02;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_14_16() {
    let a = fp("a.wav", 0.70);
    let b = fp("b.wav", 0.80);
    let d = fingerprint_distance(&a, &b);
    let want = 9.99999999999999778e-02;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_14_17() {
    let a = fp("a.wav", 0.70);
    let b = fp("b.wav", 0.85);
    let d = fingerprint_distance(&a, &b);
    let want = 1.50000000000000022e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_14_18() {
    let a = fp("a.wav", 0.70);
    let b = fp("b.wav", 0.90);
    let d = fingerprint_distance(&a, &b);
    let want = 1.99999999999999956e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_14_19() {
    let a = fp("a.wav", 0.70);
    let b = fp("b.wav", 0.95);
    let d = fingerprint_distance(&a, &b);
    let want = 2.50000000000000000e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_14_20() {
    let a = fp("a.wav", 0.70);
    let b = fp("b.wav", 1.00);
    let d = fingerprint_distance(&a, &b);
    let want = 2.99999999999999933e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_15_15() {
    let a = fp("a.wav", 0.75);
    let b = fp("b.wav", 0.75);
    let d = fingerprint_distance(&a, &b);
    let want = 0.00000000000000000e+00;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_15_16() {
    let a = fp("a.wav", 0.75);
    let b = fp("b.wav", 0.80);
    let d = fingerprint_distance(&a, &b);
    let want = 5.00000000000000444e-02;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_15_17() {
    let a = fp("a.wav", 0.75);
    let b = fp("b.wav", 0.85);
    let d = fingerprint_distance(&a, &b);
    let want = 1.00000000000000089e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_15_18() {
    let a = fp("a.wav", 0.75);
    let b = fp("b.wav", 0.90);
    let d = fingerprint_distance(&a, &b);
    let want = 1.50000000000000022e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_15_19() {
    let a = fp("a.wav", 0.75);
    let b = fp("b.wav", 0.95);
    let d = fingerprint_distance(&a, &b);
    let want = 2.00000000000000067e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_15_20() {
    let a = fp("a.wav", 0.75);
    let b = fp("b.wav", 1.00);
    let d = fingerprint_distance(&a, &b);
    let want = 2.50000000000000000e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_16_16() {
    let a = fp("a.wav", 0.80);
    let b = fp("b.wav", 0.80);
    let d = fingerprint_distance(&a, &b);
    let want = 0.00000000000000000e+00;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_16_17() {
    let a = fp("a.wav", 0.80);
    let b = fp("b.wav", 0.85);
    let d = fingerprint_distance(&a, &b);
    let want = 5.00000000000000444e-02;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_16_18() {
    let a = fp("a.wav", 0.80);
    let b = fp("b.wav", 0.90);
    let d = fingerprint_distance(&a, &b);
    let want = 9.99999999999999778e-02;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_16_19() {
    let a = fp("a.wav", 0.80);
    let b = fp("b.wav", 0.95);
    let d = fingerprint_distance(&a, &b);
    let want = 1.50000000000000022e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_16_20() {
    let a = fp("a.wav", 0.80);
    let b = fp("b.wav", 1.00);
    let d = fingerprint_distance(&a, &b);
    let want = 1.99999999999999956e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_17_17() {
    let a = fp("a.wav", 0.85);
    let b = fp("b.wav", 0.85);
    let d = fingerprint_distance(&a, &b);
    let want = 0.00000000000000000e+00;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_17_18() {
    let a = fp("a.wav", 0.85);
    let b = fp("b.wav", 0.90);
    let d = fingerprint_distance(&a, &b);
    let want = 4.99999999999999334e-02;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_17_19() {
    let a = fp("a.wav", 0.85);
    let b = fp("b.wav", 0.95);
    let d = fingerprint_distance(&a, &b);
    let want = 9.99999999999999778e-02;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_17_20() {
    let a = fp("a.wav", 0.85);
    let b = fp("b.wav", 1.00);
    let d = fingerprint_distance(&a, &b);
    let want = 1.49999999999999911e-01;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_18_18() {
    let a = fp("a.wav", 0.90);
    let b = fp("b.wav", 0.90);
    let d = fingerprint_distance(&a, &b);
    let want = 0.00000000000000000e+00;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_18_19() {
    let a = fp("a.wav", 0.90);
    let b = fp("b.wav", 0.95);
    let d = fingerprint_distance(&a, &b);
    let want = 5.00000000000000444e-02;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_18_20() {
    let a = fp("a.wav", 0.90);
    let b = fp("b.wav", 1.00);
    let d = fingerprint_distance(&a, &b);
    let want = 9.99999999999999778e-02;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_19_19() {
    let a = fp("a.wav", 0.95);
    let b = fp("b.wav", 0.95);
    let d = fingerprint_distance(&a, &b);
    let want = 0.00000000000000000e+00;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_19_20() {
    let a = fp("a.wav", 0.95);
    let b = fp("b.wav", 1.00);
    let d = fingerprint_distance(&a, &b);
    let want = 4.99999999999999334e-02;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_rms_20_20() {
    let a = fp("a.wav", 1.00);
    let b = fp("b.wav", 1.00);
    let d = fingerprint_distance(&a, &b);
    let want = 0.00000000000000000e+00;
    assert!((d - want).abs() < 1e-9, "got {} want {}", d, want);
    assert!((fingerprint_distance(&b, &a) - d).abs() < 1e-9);
}

#[test]
fn fp_sc_00_00() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.00;
    b.spectral_centroid = 0.00;
    let d = fingerprint_distance(&a, &b);
    let want = 0.00000000000000000e+00;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_00_01() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.00;
    b.spectral_centroid = 0.05;
    let d = fingerprint_distance(&a, &b);
    let want = 1.00000000000000006e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_00_02() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.00;
    b.spectral_centroid = 0.10;
    let d = fingerprint_distance(&a, &b);
    let want = 2.00000000000000011e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_00_03() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.00;
    b.spectral_centroid = 0.15;
    let d = fingerprint_distance(&a, &b);
    let want = 3.00000000000000044e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_00_04() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.00;
    b.spectral_centroid = 0.20;
    let d = fingerprint_distance(&a, &b);
    let want = 4.00000000000000022e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_00_05() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.00;
    b.spectral_centroid = 0.25;
    let d = fingerprint_distance(&a, &b);
    let want = 5.00000000000000000e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_00_06() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.00;
    b.spectral_centroid = 0.30;
    let d = fingerprint_distance(&a, &b);
    let want = 6.00000000000000089e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_00_07() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.00;
    b.spectral_centroid = 0.35;
    let d = fingerprint_distance(&a, &b);
    let want = 7.00000000000000067e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_00_08() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.00;
    b.spectral_centroid = 0.40;
    let d = fingerprint_distance(&a, &b);
    let want = 8.00000000000000044e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_00_09() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.00;
    b.spectral_centroid = 0.45;
    let d = fingerprint_distance(&a, &b);
    let want = 9.00000000000000022e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_00_10() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.00;
    b.spectral_centroid = 0.50;
    let d = fingerprint_distance(&a, &b);
    let want = 1.00000000000000000e+00;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_00_11() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.00;
    b.spectral_centroid = 0.55;
    let d = fingerprint_distance(&a, &b);
    let want = 1.10000000000000009e+00;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_00_12() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.00;
    b.spectral_centroid = 0.60;
    let d = fingerprint_distance(&a, &b);
    let want = 1.20000000000000018e+00;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_00_13() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.00;
    b.spectral_centroid = 0.65;
    let d = fingerprint_distance(&a, &b);
    let want = 1.30000000000000004e+00;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_00_14() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.00;
    b.spectral_centroid = 0.70;
    let d = fingerprint_distance(&a, &b);
    let want = 1.40000000000000013e+00;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_00_15() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.00;
    b.spectral_centroid = 0.75;
    let d = fingerprint_distance(&a, &b);
    let want = 1.50000000000000000e+00;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_00_16() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.00;
    b.spectral_centroid = 0.80;
    let d = fingerprint_distance(&a, &b);
    let want = 1.60000000000000009e+00;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_00_17() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.00;
    b.spectral_centroid = 0.85;
    let d = fingerprint_distance(&a, &b);
    let want = 1.70000000000000018e+00;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_00_18() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.00;
    b.spectral_centroid = 0.90;
    let d = fingerprint_distance(&a, &b);
    let want = 1.80000000000000004e+00;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_00_19() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.00;
    b.spectral_centroid = 0.95;
    let d = fingerprint_distance(&a, &b);
    let want = 1.90000000000000013e+00;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_00_20() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.00;
    b.spectral_centroid = 1.00;
    let d = fingerprint_distance(&a, &b);
    let want = 2.00000000000000000e+00;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_01_01() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.05;
    b.spectral_centroid = 0.05;
    let d = fingerprint_distance(&a, &b);
    let want = 0.00000000000000000e+00;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_01_02() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.05;
    b.spectral_centroid = 0.10;
    let d = fingerprint_distance(&a, &b);
    let want = 1.00000000000000006e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_01_03() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.05;
    b.spectral_centroid = 0.15;
    let d = fingerprint_distance(&a, &b);
    let want = 2.00000000000000039e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_01_04() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.05;
    b.spectral_centroid = 0.20;
    let d = fingerprint_distance(&a, &b);
    let want = 3.00000000000000044e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_01_05() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.05;
    b.spectral_centroid = 0.25;
    let d = fingerprint_distance(&a, &b);
    let want = 4.00000000000000022e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_01_06() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.05;
    b.spectral_centroid = 0.30;
    let d = fingerprint_distance(&a, &b);
    let want = 5.00000000000000111e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_01_07() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.05;
    b.spectral_centroid = 0.35;
    let d = fingerprint_distance(&a, &b);
    let want = 6.00000000000000089e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_01_08() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.05;
    b.spectral_centroid = 0.40;
    let d = fingerprint_distance(&a, &b);
    let want = 7.00000000000000067e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_01_09() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.05;
    b.spectral_centroid = 0.45;
    let d = fingerprint_distance(&a, &b);
    let want = 8.00000000000000044e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_01_10() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.05;
    b.spectral_centroid = 0.50;
    let d = fingerprint_distance(&a, &b);
    let want = 9.00000000000000022e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_01_11() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.05;
    b.spectral_centroid = 0.55;
    let d = fingerprint_distance(&a, &b);
    let want = 1.00000000000000000e+00;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_01_12() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.05;
    b.spectral_centroid = 0.60;
    let d = fingerprint_distance(&a, &b);
    let want = 1.10000000000000009e+00;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_01_13() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.05;
    b.spectral_centroid = 0.65;
    let d = fingerprint_distance(&a, &b);
    let want = 1.19999999999999996e+00;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_01_14() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.05;
    b.spectral_centroid = 0.70;
    let d = fingerprint_distance(&a, &b);
    let want = 1.30000000000000004e+00;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_01_15() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.05;
    b.spectral_centroid = 0.75;
    let d = fingerprint_distance(&a, &b);
    let want = 1.39999999999999991e+00;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_01_16() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.05;
    b.spectral_centroid = 0.80;
    let d = fingerprint_distance(&a, &b);
    let want = 1.50000000000000000e+00;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_01_17() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.05;
    b.spectral_centroid = 0.85;
    let d = fingerprint_distance(&a, &b);
    let want = 1.60000000000000009e+00;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_01_18() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.05;
    b.spectral_centroid = 0.90;
    let d = fingerprint_distance(&a, &b);
    let want = 1.69999999999999996e+00;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_01_19() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.05;
    b.spectral_centroid = 0.95;
    let d = fingerprint_distance(&a, &b);
    let want = 1.80000000000000004e+00;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_01_20() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.05;
    b.spectral_centroid = 1.00;
    let d = fingerprint_distance(&a, &b);
    let want = 1.89999999999999991e+00;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_02_02() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.10;
    b.spectral_centroid = 0.10;
    let d = fingerprint_distance(&a, &b);
    let want = 0.00000000000000000e+00;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_02_03() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.10;
    b.spectral_centroid = 0.15;
    let d = fingerprint_distance(&a, &b);
    let want = 1.00000000000000033e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_02_04() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.10;
    b.spectral_centroid = 0.20;
    let d = fingerprint_distance(&a, &b);
    let want = 2.00000000000000011e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_02_05() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.10;
    b.spectral_centroid = 0.25;
    let d = fingerprint_distance(&a, &b);
    let want = 2.99999999999999989e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_02_06() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.10;
    b.spectral_centroid = 0.30;
    let d = fingerprint_distance(&a, &b);
    let want = 4.00000000000000078e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_02_07() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.10;
    b.spectral_centroid = 0.35;
    let d = fingerprint_distance(&a, &b);
    let want = 5.00000000000000000e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_02_08() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.10;
    b.spectral_centroid = 0.40;
    let d = fingerprint_distance(&a, &b);
    let want = 6.00000000000000089e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_02_09() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.10;
    b.spectral_centroid = 0.45;
    let d = fingerprint_distance(&a, &b);
    let want = 6.99999999999999956e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_02_10() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.10;
    b.spectral_centroid = 0.50;
    let d = fingerprint_distance(&a, &b);
    let want = 8.00000000000000044e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_02_11() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.10;
    b.spectral_centroid = 0.55;
    let d = fingerprint_distance(&a, &b);
    let want = 9.00000000000000133e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_02_12() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.10;
    b.spectral_centroid = 0.60;
    let d = fingerprint_distance(&a, &b);
    let want = 1.00000000000000022e+00;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_02_13() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.10;
    b.spectral_centroid = 0.65;
    let d = fingerprint_distance(&a, &b);
    let want = 1.10000000000000009e+00;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_02_14() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.10;
    b.spectral_centroid = 0.70;
    let d = fingerprint_distance(&a, &b);
    let want = 1.20000000000000018e+00;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_02_15() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.10;
    b.spectral_centroid = 0.75;
    let d = fingerprint_distance(&a, &b);
    let want = 1.30000000000000004e+00;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_02_16() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.10;
    b.spectral_centroid = 0.80;
    let d = fingerprint_distance(&a, &b);
    let want = 1.40000000000000013e+00;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_02_17() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.10;
    b.spectral_centroid = 0.85;
    let d = fingerprint_distance(&a, &b);
    let want = 1.50000000000000022e+00;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_02_18() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.10;
    b.spectral_centroid = 0.90;
    let d = fingerprint_distance(&a, &b);
    let want = 1.60000000000000009e+00;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_02_19() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.10;
    b.spectral_centroid = 0.95;
    let d = fingerprint_distance(&a, &b);
    let want = 1.70000000000000018e+00;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_02_20() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.10;
    b.spectral_centroid = 1.00;
    let d = fingerprint_distance(&a, &b);
    let want = 1.80000000000000004e+00;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_03_03() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.15;
    b.spectral_centroid = 0.15;
    let d = fingerprint_distance(&a, &b);
    let want = 0.00000000000000000e+00;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_03_04() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.15;
    b.spectral_centroid = 0.20;
    let d = fingerprint_distance(&a, &b);
    let want = 9.99999999999999778e-02;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_03_05() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.15;
    b.spectral_centroid = 0.25;
    let d = fingerprint_distance(&a, &b);
    let want = 1.99999999999999956e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_03_06() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.15;
    b.spectral_centroid = 0.30;
    let d = fingerprint_distance(&a, &b);
    let want = 3.00000000000000044e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_03_07() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.15;
    b.spectral_centroid = 0.35;
    let d = fingerprint_distance(&a, &b);
    let want = 4.00000000000000022e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_03_08() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.15;
    b.spectral_centroid = 0.40;
    let d = fingerprint_distance(&a, &b);
    let want = 5.00000000000000000e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_03_09() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.15;
    b.spectral_centroid = 0.45;
    let d = fingerprint_distance(&a, &b);
    let want = 5.99999999999999978e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_03_10() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.15;
    b.spectral_centroid = 0.50;
    let d = fingerprint_distance(&a, &b);
    let want = 6.99999999999999956e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_03_11() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.15;
    b.spectral_centroid = 0.55;
    let d = fingerprint_distance(&a, &b);
    let want = 8.00000000000000044e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_03_12() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.15;
    b.spectral_centroid = 0.60;
    let d = fingerprint_distance(&a, &b);
    let want = 9.00000000000000133e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_03_13() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.15;
    b.spectral_centroid = 0.65;
    let d = fingerprint_distance(&a, &b);
    let want = 1.00000000000000000e+00;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_03_14() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.15;
    b.spectral_centroid = 0.70;
    let d = fingerprint_distance(&a, &b);
    let want = 1.10000000000000009e+00;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_03_15() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.15;
    b.spectral_centroid = 0.75;
    let d = fingerprint_distance(&a, &b);
    let want = 1.19999999999999996e+00;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_03_16() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.15;
    b.spectral_centroid = 0.80;
    let d = fingerprint_distance(&a, &b);
    let want = 1.30000000000000004e+00;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_03_17() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.15;
    b.spectral_centroid = 0.85;
    let d = fingerprint_distance(&a, &b);
    let want = 1.40000000000000013e+00;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_03_18() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.15;
    b.spectral_centroid = 0.90;
    let d = fingerprint_distance(&a, &b);
    let want = 1.50000000000000000e+00;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_03_19() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.15;
    b.spectral_centroid = 0.95;
    let d = fingerprint_distance(&a, &b);
    let want = 1.60000000000000009e+00;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_03_20() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.15;
    b.spectral_centroid = 1.00;
    let d = fingerprint_distance(&a, &b);
    let want = 1.69999999999999996e+00;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_04_04() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.20;
    b.spectral_centroid = 0.20;
    let d = fingerprint_distance(&a, &b);
    let want = 0.00000000000000000e+00;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_04_05() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.20;
    b.spectral_centroid = 0.25;
    let d = fingerprint_distance(&a, &b);
    let want = 9.99999999999999778e-02;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_04_06() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.20;
    b.spectral_centroid = 0.30;
    let d = fingerprint_distance(&a, &b);
    let want = 2.00000000000000067e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_04_07() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.20;
    b.spectral_centroid = 0.35;
    let d = fingerprint_distance(&a, &b);
    let want = 3.00000000000000044e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_04_08() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.20;
    b.spectral_centroid = 0.40;
    let d = fingerprint_distance(&a, &b);
    let want = 4.00000000000000022e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_04_09() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.20;
    b.spectral_centroid = 0.45;
    let d = fingerprint_distance(&a, &b);
    let want = 5.00000000000000000e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_04_10() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.20;
    b.spectral_centroid = 0.50;
    let d = fingerprint_distance(&a, &b);
    let want = 5.99999999999999978e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_04_11() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.20;
    b.spectral_centroid = 0.55;
    let d = fingerprint_distance(&a, &b);
    let want = 7.00000000000000067e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_04_12() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.20;
    b.spectral_centroid = 0.60;
    let d = fingerprint_distance(&a, &b);
    let want = 8.00000000000000155e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_04_13() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.20;
    b.spectral_centroid = 0.65;
    let d = fingerprint_distance(&a, &b);
    let want = 9.00000000000000022e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_04_14() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.20;
    b.spectral_centroid = 0.70;
    let d = fingerprint_distance(&a, &b);
    let want = 1.00000000000000000e+00;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_04_15() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.20;
    b.spectral_centroid = 0.75;
    let d = fingerprint_distance(&a, &b);
    let want = 1.10000000000000009e+00;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_04_16() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.20;
    b.spectral_centroid = 0.80;
    let d = fingerprint_distance(&a, &b);
    let want = 1.20000000000000018e+00;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_04_17() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.20;
    b.spectral_centroid = 0.85;
    let d = fingerprint_distance(&a, &b);
    let want = 1.30000000000000027e+00;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_04_18() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.20;
    b.spectral_centroid = 0.90;
    let d = fingerprint_distance(&a, &b);
    let want = 1.39999999999999991e+00;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_04_19() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.20;
    b.spectral_centroid = 0.95;
    let d = fingerprint_distance(&a, &b);
    let want = 1.50000000000000000e+00;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_04_20() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.20;
    b.spectral_centroid = 1.00;
    let d = fingerprint_distance(&a, &b);
    let want = 1.60000000000000009e+00;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_05_05() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.25;
    b.spectral_centroid = 0.25;
    let d = fingerprint_distance(&a, &b);
    let want = 0.00000000000000000e+00;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_05_06() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.25;
    b.spectral_centroid = 0.30;
    let d = fingerprint_distance(&a, &b);
    let want = 1.00000000000000089e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_05_07() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.25;
    b.spectral_centroid = 0.35;
    let d = fingerprint_distance(&a, &b);
    let want = 2.00000000000000067e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_05_08() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.25;
    b.spectral_centroid = 0.40;
    let d = fingerprint_distance(&a, &b);
    let want = 3.00000000000000044e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_05_09() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.25;
    b.spectral_centroid = 0.45;
    let d = fingerprint_distance(&a, &b);
    let want = 4.00000000000000022e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_05_10() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.25;
    b.spectral_centroid = 0.50;
    let d = fingerprint_distance(&a, &b);
    let want = 5.00000000000000000e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_05_11() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.25;
    b.spectral_centroid = 0.55;
    let d = fingerprint_distance(&a, &b);
    let want = 6.00000000000000089e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_05_12() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.25;
    b.spectral_centroid = 0.60;
    let d = fingerprint_distance(&a, &b);
    let want = 7.00000000000000178e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_05_13() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.25;
    b.spectral_centroid = 0.65;
    let d = fingerprint_distance(&a, &b);
    let want = 8.00000000000000044e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_05_14() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.25;
    b.spectral_centroid = 0.70;
    let d = fingerprint_distance(&a, &b);
    let want = 9.00000000000000133e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_05_15() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.25;
    b.spectral_centroid = 0.75;
    let d = fingerprint_distance(&a, &b);
    let want = 1.00000000000000000e+00;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_05_16() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.25;
    b.spectral_centroid = 0.80;
    let d = fingerprint_distance(&a, &b);
    let want = 1.10000000000000009e+00;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_05_17() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.25;
    b.spectral_centroid = 0.85;
    let d = fingerprint_distance(&a, &b);
    let want = 1.20000000000000018e+00;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_05_18() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.25;
    b.spectral_centroid = 0.90;
    let d = fingerprint_distance(&a, &b);
    let want = 1.30000000000000004e+00;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_05_19() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.25;
    b.spectral_centroid = 0.95;
    let d = fingerprint_distance(&a, &b);
    let want = 1.40000000000000013e+00;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_05_20() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.25;
    b.spectral_centroid = 1.00;
    let d = fingerprint_distance(&a, &b);
    let want = 1.50000000000000000e+00;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_06_06() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.30;
    b.spectral_centroid = 0.30;
    let d = fingerprint_distance(&a, &b);
    let want = 0.00000000000000000e+00;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_06_07() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.30;
    b.spectral_centroid = 0.35;
    let d = fingerprint_distance(&a, &b);
    let want = 9.99999999999999778e-02;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_06_08() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.30;
    b.spectral_centroid = 0.40;
    let d = fingerprint_distance(&a, &b);
    let want = 1.99999999999999956e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_06_09() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.30;
    b.spectral_centroid = 0.45;
    let d = fingerprint_distance(&a, &b);
    let want = 2.99999999999999933e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_06_10() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.30;
    b.spectral_centroid = 0.50;
    let d = fingerprint_distance(&a, &b);
    let want = 3.99999999999999911e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_06_11() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.30;
    b.spectral_centroid = 0.55;
    let d = fingerprint_distance(&a, &b);
    let want = 5.00000000000000000e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_06_12() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.30;
    b.spectral_centroid = 0.60;
    let d = fingerprint_distance(&a, &b);
    let want = 6.00000000000000089e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_06_13() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.30;
    b.spectral_centroid = 0.65;
    let d = fingerprint_distance(&a, &b);
    let want = 6.99999999999999956e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_06_14() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.30;
    b.spectral_centroid = 0.70;
    let d = fingerprint_distance(&a, &b);
    let want = 8.00000000000000044e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_06_15() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.30;
    b.spectral_centroid = 0.75;
    let d = fingerprint_distance(&a, &b);
    let want = 8.99999999999999911e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_06_16() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.30;
    b.spectral_centroid = 0.80;
    let d = fingerprint_distance(&a, &b);
    let want = 1.00000000000000000e+00;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_06_17() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.30;
    b.spectral_centroid = 0.85;
    let d = fingerprint_distance(&a, &b);
    let want = 1.10000000000000009e+00;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_06_18() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.30;
    b.spectral_centroid = 0.90;
    let d = fingerprint_distance(&a, &b);
    let want = 1.19999999999999996e+00;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_06_19() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.30;
    b.spectral_centroid = 0.95;
    let d = fingerprint_distance(&a, &b);
    let want = 1.30000000000000004e+00;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_06_20() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.30;
    b.spectral_centroid = 1.00;
    let d = fingerprint_distance(&a, &b);
    let want = 1.39999999999999991e+00;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_07_07() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.35;
    b.spectral_centroid = 0.35;
    let d = fingerprint_distance(&a, &b);
    let want = 0.00000000000000000e+00;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_07_08() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.35;
    b.spectral_centroid = 0.40;
    let d = fingerprint_distance(&a, &b);
    let want = 9.99999999999999778e-02;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_07_09() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.35;
    b.spectral_centroid = 0.45;
    let d = fingerprint_distance(&a, &b);
    let want = 1.99999999999999956e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_07_10() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.35;
    b.spectral_centroid = 0.50;
    let d = fingerprint_distance(&a, &b);
    let want = 2.99999999999999933e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_07_11() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.35;
    b.spectral_centroid = 0.55;
    let d = fingerprint_distance(&a, &b);
    let want = 4.00000000000000022e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_07_12() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.35;
    b.spectral_centroid = 0.60;
    let d = fingerprint_distance(&a, &b);
    let want = 5.00000000000000111e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_07_13() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.35;
    b.spectral_centroid = 0.65;
    let d = fingerprint_distance(&a, &b);
    let want = 5.99999999999999978e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_07_14() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.35;
    b.spectral_centroid = 0.70;
    let d = fingerprint_distance(&a, &b);
    let want = 7.00000000000000067e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_07_15() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.35;
    b.spectral_centroid = 0.75;
    let d = fingerprint_distance(&a, &b);
    let want = 7.99999999999999933e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_07_16() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.35;
    b.spectral_centroid = 0.80;
    let d = fingerprint_distance(&a, &b);
    let want = 9.00000000000000022e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_07_17() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.35;
    b.spectral_centroid = 0.85;
    let d = fingerprint_distance(&a, &b);
    let want = 1.00000000000000000e+00;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_07_18() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.35;
    b.spectral_centroid = 0.90;
    let d = fingerprint_distance(&a, &b);
    let want = 1.10000000000000009e+00;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_07_19() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.35;
    b.spectral_centroid = 0.95;
    let d = fingerprint_distance(&a, &b);
    let want = 1.20000000000000018e+00;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_07_20() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.35;
    b.spectral_centroid = 1.00;
    let d = fingerprint_distance(&a, &b);
    let want = 1.29999999999999982e+00;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_08_08() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.40;
    b.spectral_centroid = 0.40;
    let d = fingerprint_distance(&a, &b);
    let want = 0.00000000000000000e+00;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_08_09() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.40;
    b.spectral_centroid = 0.45;
    let d = fingerprint_distance(&a, &b);
    let want = 9.99999999999999778e-02;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_08_10() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.40;
    b.spectral_centroid = 0.50;
    let d = fingerprint_distance(&a, &b);
    let want = 1.99999999999999956e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_08_11() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.40;
    b.spectral_centroid = 0.55;
    let d = fingerprint_distance(&a, &b);
    let want = 3.00000000000000044e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_08_12() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.40;
    b.spectral_centroid = 0.60;
    let d = fingerprint_distance(&a, &b);
    let want = 4.00000000000000133e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_08_13() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.40;
    b.spectral_centroid = 0.65;
    let d = fingerprint_distance(&a, &b);
    let want = 5.00000000000000000e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_08_14() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.40;
    b.spectral_centroid = 0.70;
    let d = fingerprint_distance(&a, &b);
    let want = 6.00000000000000089e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_08_15() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.40;
    b.spectral_centroid = 0.75;
    let d = fingerprint_distance(&a, &b);
    let want = 6.99999999999999956e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_08_16() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.40;
    b.spectral_centroid = 0.80;
    let d = fingerprint_distance(&a, &b);
    let want = 8.00000000000000044e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_08_17() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.40;
    b.spectral_centroid = 0.85;
    let d = fingerprint_distance(&a, &b);
    let want = 9.00000000000000133e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_08_18() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.40;
    b.spectral_centroid = 0.90;
    let d = fingerprint_distance(&a, &b);
    let want = 1.00000000000000000e+00;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_08_19() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.40;
    b.spectral_centroid = 0.95;
    let d = fingerprint_distance(&a, &b);
    let want = 1.10000000000000009e+00;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_08_20() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.40;
    b.spectral_centroid = 1.00;
    let d = fingerprint_distance(&a, &b);
    let want = 1.19999999999999996e+00;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_09_09() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.45;
    b.spectral_centroid = 0.45;
    let d = fingerprint_distance(&a, &b);
    let want = 0.00000000000000000e+00;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_09_10() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.45;
    b.spectral_centroid = 0.50;
    let d = fingerprint_distance(&a, &b);
    let want = 9.99999999999999778e-02;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_09_11() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.45;
    b.spectral_centroid = 0.55;
    let d = fingerprint_distance(&a, &b);
    let want = 2.00000000000000067e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_09_12() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.45;
    b.spectral_centroid = 0.60;
    let d = fingerprint_distance(&a, &b);
    let want = 3.00000000000000155e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_09_13() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.45;
    b.spectral_centroid = 0.65;
    let d = fingerprint_distance(&a, &b);
    let want = 4.00000000000000022e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_09_14() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.45;
    b.spectral_centroid = 0.70;
    let d = fingerprint_distance(&a, &b);
    let want = 5.00000000000000111e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_09_15() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.45;
    b.spectral_centroid = 0.75;
    let d = fingerprint_distance(&a, &b);
    let want = 5.99999999999999978e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_09_16() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.45;
    b.spectral_centroid = 0.80;
    let d = fingerprint_distance(&a, &b);
    let want = 7.00000000000000067e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_09_17() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.45;
    b.spectral_centroid = 0.85;
    let d = fingerprint_distance(&a, &b);
    let want = 8.00000000000000155e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_09_18() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.45;
    b.spectral_centroid = 0.90;
    let d = fingerprint_distance(&a, &b);
    let want = 9.00000000000000022e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_09_19() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.45;
    b.spectral_centroid = 0.95;
    let d = fingerprint_distance(&a, &b);
    let want = 1.00000000000000000e+00;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_09_20() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.45;
    b.spectral_centroid = 1.00;
    let d = fingerprint_distance(&a, &b);
    let want = 1.10000000000000009e+00;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_10_10() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.50;
    b.spectral_centroid = 0.50;
    let d = fingerprint_distance(&a, &b);
    let want = 0.00000000000000000e+00;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_10_11() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.50;
    b.spectral_centroid = 0.55;
    let d = fingerprint_distance(&a, &b);
    let want = 1.00000000000000089e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_10_12() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.50;
    b.spectral_centroid = 0.60;
    let d = fingerprint_distance(&a, &b);
    let want = 2.00000000000000178e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_10_13() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.50;
    b.spectral_centroid = 0.65;
    let d = fingerprint_distance(&a, &b);
    let want = 3.00000000000000044e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_10_14() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.50;
    b.spectral_centroid = 0.70;
    let d = fingerprint_distance(&a, &b);
    let want = 4.00000000000000133e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_10_15() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.50;
    b.spectral_centroid = 0.75;
    let d = fingerprint_distance(&a, &b);
    let want = 5.00000000000000000e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_10_16() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.50;
    b.spectral_centroid = 0.80;
    let d = fingerprint_distance(&a, &b);
    let want = 6.00000000000000089e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_10_17() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.50;
    b.spectral_centroid = 0.85;
    let d = fingerprint_distance(&a, &b);
    let want = 7.00000000000000178e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_10_18() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.50;
    b.spectral_centroid = 0.90;
    let d = fingerprint_distance(&a, &b);
    let want = 8.00000000000000044e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_10_19() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.50;
    b.spectral_centroid = 0.95;
    let d = fingerprint_distance(&a, &b);
    let want = 9.00000000000000133e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_10_20() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.50;
    b.spectral_centroid = 1.00;
    let d = fingerprint_distance(&a, &b);
    let want = 1.00000000000000000e+00;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_11_11() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.55;
    b.spectral_centroid = 0.55;
    let d = fingerprint_distance(&a, &b);
    let want = 0.00000000000000000e+00;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_11_12() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.55;
    b.spectral_centroid = 0.60;
    let d = fingerprint_distance(&a, &b);
    let want = 1.00000000000000089e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_11_13() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.55;
    b.spectral_centroid = 0.65;
    let d = fingerprint_distance(&a, &b);
    let want = 1.99999999999999956e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_11_14() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.55;
    b.spectral_centroid = 0.70;
    let d = fingerprint_distance(&a, &b);
    let want = 3.00000000000000044e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_11_15() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.55;
    b.spectral_centroid = 0.75;
    let d = fingerprint_distance(&a, &b);
    let want = 3.99999999999999911e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_11_16() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.55;
    b.spectral_centroid = 0.80;
    let d = fingerprint_distance(&a, &b);
    let want = 5.00000000000000000e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_11_17() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.55;
    b.spectral_centroid = 0.85;
    let d = fingerprint_distance(&a, &b);
    let want = 6.00000000000000089e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_11_18() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.55;
    b.spectral_centroid = 0.90;
    let d = fingerprint_distance(&a, &b);
    let want = 6.99999999999999956e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_11_19() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.55;
    b.spectral_centroid = 0.95;
    let d = fingerprint_distance(&a, &b);
    let want = 8.00000000000000044e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_11_20() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.55;
    b.spectral_centroid = 1.00;
    let d = fingerprint_distance(&a, &b);
    let want = 8.99999999999999911e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_12_12() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.60;
    b.spectral_centroid = 0.60;
    let d = fingerprint_distance(&a, &b);
    let want = 0.00000000000000000e+00;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_12_13() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.60;
    b.spectral_centroid = 0.65;
    let d = fingerprint_distance(&a, &b);
    let want = 9.99999999999998668e-02;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_12_14() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.60;
    b.spectral_centroid = 0.70;
    let d = fingerprint_distance(&a, &b);
    let want = 1.99999999999999956e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_12_15() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.60;
    b.spectral_centroid = 0.75;
    let d = fingerprint_distance(&a, &b);
    let want = 2.99999999999999822e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_12_16() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.60;
    b.spectral_centroid = 0.80;
    let d = fingerprint_distance(&a, &b);
    let want = 3.99999999999999911e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_12_17() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.60;
    b.spectral_centroid = 0.85;
    let d = fingerprint_distance(&a, &b);
    let want = 5.00000000000000000e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_12_18() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.60;
    b.spectral_centroid = 0.90;
    let d = fingerprint_distance(&a, &b);
    let want = 5.99999999999999867e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_12_19() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.60;
    b.spectral_centroid = 0.95;
    let d = fingerprint_distance(&a, &b);
    let want = 6.99999999999999956e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_12_20() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.60;
    b.spectral_centroid = 1.00;
    let d = fingerprint_distance(&a, &b);
    let want = 7.99999999999999822e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_13_13() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.65;
    b.spectral_centroid = 0.65;
    let d = fingerprint_distance(&a, &b);
    let want = 0.00000000000000000e+00;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_13_14() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.65;
    b.spectral_centroid = 0.70;
    let d = fingerprint_distance(&a, &b);
    let want = 1.00000000000000089e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_13_15() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.65;
    b.spectral_centroid = 0.75;
    let d = fingerprint_distance(&a, &b);
    let want = 1.99999999999999956e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_13_16() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.65;
    b.spectral_centroid = 0.80;
    let d = fingerprint_distance(&a, &b);
    let want = 3.00000000000000044e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_13_17() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.65;
    b.spectral_centroid = 0.85;
    let d = fingerprint_distance(&a, &b);
    let want = 4.00000000000000133e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_13_18() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.65;
    b.spectral_centroid = 0.90;
    let d = fingerprint_distance(&a, &b);
    let want = 5.00000000000000000e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_13_19() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.65;
    b.spectral_centroid = 0.95;
    let d = fingerprint_distance(&a, &b);
    let want = 6.00000000000000089e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_13_20() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.65;
    b.spectral_centroid = 1.00;
    let d = fingerprint_distance(&a, &b);
    let want = 6.99999999999999956e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_14_14() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.70;
    b.spectral_centroid = 0.70;
    let d = fingerprint_distance(&a, &b);
    let want = 0.00000000000000000e+00;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_14_15() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.70;
    b.spectral_centroid = 0.75;
    let d = fingerprint_distance(&a, &b);
    let want = 9.99999999999998668e-02;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_14_16() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.70;
    b.spectral_centroid = 0.80;
    let d = fingerprint_distance(&a, &b);
    let want = 1.99999999999999956e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_14_17() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.70;
    b.spectral_centroid = 0.85;
    let d = fingerprint_distance(&a, &b);
    let want = 3.00000000000000044e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_14_18() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.70;
    b.spectral_centroid = 0.90;
    let d = fingerprint_distance(&a, &b);
    let want = 3.99999999999999911e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_14_19() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.70;
    b.spectral_centroid = 0.95;
    let d = fingerprint_distance(&a, &b);
    let want = 5.00000000000000000e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_14_20() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.70;
    b.spectral_centroid = 1.00;
    let d = fingerprint_distance(&a, &b);
    let want = 5.99999999999999867e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_15_15() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.75;
    b.spectral_centroid = 0.75;
    let d = fingerprint_distance(&a, &b);
    let want = 0.00000000000000000e+00;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_15_16() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.75;
    b.spectral_centroid = 0.80;
    let d = fingerprint_distance(&a, &b);
    let want = 1.00000000000000089e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_15_17() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.75;
    b.spectral_centroid = 0.85;
    let d = fingerprint_distance(&a, &b);
    let want = 2.00000000000000178e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_15_18() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.75;
    b.spectral_centroid = 0.90;
    let d = fingerprint_distance(&a, &b);
    let want = 3.00000000000000044e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_15_19() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.75;
    b.spectral_centroid = 0.95;
    let d = fingerprint_distance(&a, &b);
    let want = 4.00000000000000133e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_15_20() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.75;
    b.spectral_centroid = 1.00;
    let d = fingerprint_distance(&a, &b);
    let want = 5.00000000000000000e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_16_16() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.80;
    b.spectral_centroid = 0.80;
    let d = fingerprint_distance(&a, &b);
    let want = 0.00000000000000000e+00;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_16_17() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.80;
    b.spectral_centroid = 0.85;
    let d = fingerprint_distance(&a, &b);
    let want = 1.00000000000000089e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_16_18() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.80;
    b.spectral_centroid = 0.90;
    let d = fingerprint_distance(&a, &b);
    let want = 1.99999999999999956e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_16_19() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.80;
    b.spectral_centroid = 0.95;
    let d = fingerprint_distance(&a, &b);
    let want = 3.00000000000000044e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_16_20() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.80;
    b.spectral_centroid = 1.00;
    let d = fingerprint_distance(&a, &b);
    let want = 3.99999999999999911e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_17_17() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.85;
    b.spectral_centroid = 0.85;
    let d = fingerprint_distance(&a, &b);
    let want = 0.00000000000000000e+00;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_17_18() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.85;
    b.spectral_centroid = 0.90;
    let d = fingerprint_distance(&a, &b);
    let want = 9.99999999999998668e-02;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_17_19() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.85;
    b.spectral_centroid = 0.95;
    let d = fingerprint_distance(&a, &b);
    let want = 1.99999999999999956e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_17_20() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.85;
    b.spectral_centroid = 1.00;
    let d = fingerprint_distance(&a, &b);
    let want = 2.99999999999999822e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_18_18() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.90;
    b.spectral_centroid = 0.90;
    let d = fingerprint_distance(&a, &b);
    let want = 0.00000000000000000e+00;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_18_19() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.90;
    b.spectral_centroid = 0.95;
    let d = fingerprint_distance(&a, &b);
    let want = 1.00000000000000089e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_18_20() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.90;
    b.spectral_centroid = 1.00;
    let d = fingerprint_distance(&a, &b);
    let want = 1.99999999999999956e-01;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_19_19() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.95;
    b.spectral_centroid = 0.95;
    let d = fingerprint_distance(&a, &b);
    let want = 0.00000000000000000e+00;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_19_20() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 0.95;
    b.spectral_centroid = 1.00;
    let d = fingerprint_distance(&a, &b);
    let want = 9.99999999999998668e-02;
    assert!((d - want).abs() < 1e-9);
}

#[test]
fn fp_sc_20_20() {
    let mut a = fp("a.wav", 0.5);
    let mut b = fp("b.wav", 0.5);
    a.spectral_centroid = 1.00;
    b.spectral_centroid = 1.00;
    let d = fingerprint_distance(&a, &b);
    let want = 0.00000000000000000e+00;
    assert!((d - want).abs() < 1e-9);
}
