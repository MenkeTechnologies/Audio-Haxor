//! Large handcrafted tables: `kvr::parse_version`, `kvr::compare_versions`, `format_size`.
//! String literals and expected values were generated with the same algorithms as the Rust code.

use std::cmp::Ordering;

// ── parse_version ──────────────────────────────────────────────────

#[test]
fn pv_empty() {
    assert_eq!(app_lib::kvr::parse_version(""), vec![0, 0, 0]);
}

#[test]
fn pv_unknown() {
    assert_eq!(app_lib::kvr::parse_version("Unknown"), vec![0, 0, 0]);
}

#[test]
fn pv_00_00_00() {
    assert_eq!(app_lib::kvr::parse_version("0.0.0"), vec![0, 0, 0]);
}

#[test]
fn pv_00_00_01() {
    assert_eq!(app_lib::kvr::parse_version("0.0.1"), vec![0, 0, 1]);
}

#[test]
fn pv_00_00_02() {
    assert_eq!(app_lib::kvr::parse_version("0.0.2"), vec![0, 0, 2]);
}

#[test]
fn pv_00_00_03() {
    assert_eq!(app_lib::kvr::parse_version("0.0.3"), vec![0, 0, 3]);
}

#[test]
fn pv_00_00_04() {
    assert_eq!(app_lib::kvr::parse_version("0.0.4"), vec![0, 0, 4]);
}

#[test]
fn pv_00_00_05() {
    assert_eq!(app_lib::kvr::parse_version("0.0.5"), vec![0, 0, 5]);
}

#[test]
fn pv_00_00_06() {
    assert_eq!(app_lib::kvr::parse_version("0.0.6"), vec![0, 0, 6]);
}

#[test]
fn pv_00_00_07() {
    assert_eq!(app_lib::kvr::parse_version("0.0.7"), vec![0, 0, 7]);
}

#[test]
fn pv_00_01_00() {
    assert_eq!(app_lib::kvr::parse_version("0.1.0"), vec![0, 1, 0]);
}

#[test]
fn pv_00_01_01() {
    assert_eq!(app_lib::kvr::parse_version("0.1.1"), vec![0, 1, 1]);
}

#[test]
fn pv_00_01_02() {
    assert_eq!(app_lib::kvr::parse_version("0.1.2"), vec![0, 1, 2]);
}

#[test]
fn pv_00_01_03() {
    assert_eq!(app_lib::kvr::parse_version("0.1.3"), vec![0, 1, 3]);
}

#[test]
fn pv_00_01_04() {
    assert_eq!(app_lib::kvr::parse_version("0.1.4"), vec![0, 1, 4]);
}

#[test]
fn pv_00_01_05() {
    assert_eq!(app_lib::kvr::parse_version("0.1.5"), vec![0, 1, 5]);
}

#[test]
fn pv_00_01_06() {
    assert_eq!(app_lib::kvr::parse_version("0.1.6"), vec![0, 1, 6]);
}

#[test]
fn pv_00_01_07() {
    assert_eq!(app_lib::kvr::parse_version("0.1.7"), vec![0, 1, 7]);
}

#[test]
fn pv_00_02_00() {
    assert_eq!(app_lib::kvr::parse_version("0.2.0"), vec![0, 2, 0]);
}

#[test]
fn pv_00_02_01() {
    assert_eq!(app_lib::kvr::parse_version("0.2.1"), vec![0, 2, 1]);
}

#[test]
fn pv_00_02_02() {
    assert_eq!(app_lib::kvr::parse_version("0.2.2"), vec![0, 2, 2]);
}

#[test]
fn pv_00_02_03() {
    assert_eq!(app_lib::kvr::parse_version("0.2.3"), vec![0, 2, 3]);
}

#[test]
fn pv_00_02_04() {
    assert_eq!(app_lib::kvr::parse_version("0.2.4"), vec![0, 2, 4]);
}

#[test]
fn pv_00_02_05() {
    assert_eq!(app_lib::kvr::parse_version("0.2.5"), vec![0, 2, 5]);
}

#[test]
fn pv_00_02_06() {
    assert_eq!(app_lib::kvr::parse_version("0.2.6"), vec![0, 2, 6]);
}

#[test]
fn pv_00_02_07() {
    assert_eq!(app_lib::kvr::parse_version("0.2.7"), vec![0, 2, 7]);
}

#[test]
fn pv_00_03_00() {
    assert_eq!(app_lib::kvr::parse_version("0.3.0"), vec![0, 3, 0]);
}

#[test]
fn pv_00_03_01() {
    assert_eq!(app_lib::kvr::parse_version("0.3.1"), vec![0, 3, 1]);
}

#[test]
fn pv_00_03_02() {
    assert_eq!(app_lib::kvr::parse_version("0.3.2"), vec![0, 3, 2]);
}

#[test]
fn pv_00_03_03() {
    assert_eq!(app_lib::kvr::parse_version("0.3.3"), vec![0, 3, 3]);
}

#[test]
fn pv_00_03_04() {
    assert_eq!(app_lib::kvr::parse_version("0.3.4"), vec![0, 3, 4]);
}

#[test]
fn pv_00_03_05() {
    assert_eq!(app_lib::kvr::parse_version("0.3.5"), vec![0, 3, 5]);
}

#[test]
fn pv_00_03_06() {
    assert_eq!(app_lib::kvr::parse_version("0.3.6"), vec![0, 3, 6]);
}

#[test]
fn pv_00_03_07() {
    assert_eq!(app_lib::kvr::parse_version("0.3.7"), vec![0, 3, 7]);
}

#[test]
fn pv_00_04_00() {
    assert_eq!(app_lib::kvr::parse_version("0.4.0"), vec![0, 4, 0]);
}

#[test]
fn pv_00_04_01() {
    assert_eq!(app_lib::kvr::parse_version("0.4.1"), vec![0, 4, 1]);
}

#[test]
fn pv_00_04_02() {
    assert_eq!(app_lib::kvr::parse_version("0.4.2"), vec![0, 4, 2]);
}

#[test]
fn pv_00_04_03() {
    assert_eq!(app_lib::kvr::parse_version("0.4.3"), vec![0, 4, 3]);
}

#[test]
fn pv_00_04_04() {
    assert_eq!(app_lib::kvr::parse_version("0.4.4"), vec![0, 4, 4]);
}

#[test]
fn pv_00_04_05() {
    assert_eq!(app_lib::kvr::parse_version("0.4.5"), vec![0, 4, 5]);
}

#[test]
fn pv_00_04_06() {
    assert_eq!(app_lib::kvr::parse_version("0.4.6"), vec![0, 4, 6]);
}

#[test]
fn pv_00_04_07() {
    assert_eq!(app_lib::kvr::parse_version("0.4.7"), vec![0, 4, 7]);
}

#[test]
fn pv_00_05_00() {
    assert_eq!(app_lib::kvr::parse_version("0.5.0"), vec![0, 5, 0]);
}

#[test]
fn pv_00_05_01() {
    assert_eq!(app_lib::kvr::parse_version("0.5.1"), vec![0, 5, 1]);
}

#[test]
fn pv_00_05_02() {
    assert_eq!(app_lib::kvr::parse_version("0.5.2"), vec![0, 5, 2]);
}

#[test]
fn pv_00_05_03() {
    assert_eq!(app_lib::kvr::parse_version("0.5.3"), vec![0, 5, 3]);
}

#[test]
fn pv_00_05_04() {
    assert_eq!(app_lib::kvr::parse_version("0.5.4"), vec![0, 5, 4]);
}

#[test]
fn pv_00_05_05() {
    assert_eq!(app_lib::kvr::parse_version("0.5.5"), vec![0, 5, 5]);
}

#[test]
fn pv_00_05_06() {
    assert_eq!(app_lib::kvr::parse_version("0.5.6"), vec![0, 5, 6]);
}

#[test]
fn pv_00_05_07() {
    assert_eq!(app_lib::kvr::parse_version("0.5.7"), vec![0, 5, 7]);
}

#[test]
fn pv_00_06_00() {
    assert_eq!(app_lib::kvr::parse_version("0.6.0"), vec![0, 6, 0]);
}

#[test]
fn pv_00_06_01() {
    assert_eq!(app_lib::kvr::parse_version("0.6.1"), vec![0, 6, 1]);
}

#[test]
fn pv_00_06_02() {
    assert_eq!(app_lib::kvr::parse_version("0.6.2"), vec![0, 6, 2]);
}

#[test]
fn pv_00_06_03() {
    assert_eq!(app_lib::kvr::parse_version("0.6.3"), vec![0, 6, 3]);
}

#[test]
fn pv_00_06_04() {
    assert_eq!(app_lib::kvr::parse_version("0.6.4"), vec![0, 6, 4]);
}

#[test]
fn pv_00_06_05() {
    assert_eq!(app_lib::kvr::parse_version("0.6.5"), vec![0, 6, 5]);
}

#[test]
fn pv_00_06_06() {
    assert_eq!(app_lib::kvr::parse_version("0.6.6"), vec![0, 6, 6]);
}

#[test]
fn pv_00_06_07() {
    assert_eq!(app_lib::kvr::parse_version("0.6.7"), vec![0, 6, 7]);
}

#[test]
fn pv_00_07_00() {
    assert_eq!(app_lib::kvr::parse_version("0.7.0"), vec![0, 7, 0]);
}

#[test]
fn pv_00_07_01() {
    assert_eq!(app_lib::kvr::parse_version("0.7.1"), vec![0, 7, 1]);
}

#[test]
fn pv_00_07_02() {
    assert_eq!(app_lib::kvr::parse_version("0.7.2"), vec![0, 7, 2]);
}

#[test]
fn pv_00_07_03() {
    assert_eq!(app_lib::kvr::parse_version("0.7.3"), vec![0, 7, 3]);
}

#[test]
fn pv_00_07_04() {
    assert_eq!(app_lib::kvr::parse_version("0.7.4"), vec![0, 7, 4]);
}

#[test]
fn pv_00_07_05() {
    assert_eq!(app_lib::kvr::parse_version("0.7.5"), vec![0, 7, 5]);
}

#[test]
fn pv_00_07_06() {
    assert_eq!(app_lib::kvr::parse_version("0.7.6"), vec![0, 7, 6]);
}

#[test]
fn pv_00_07_07() {
    assert_eq!(app_lib::kvr::parse_version("0.7.7"), vec![0, 7, 7]);
}

#[test]
fn pv_01_00_00() {
    assert_eq!(app_lib::kvr::parse_version("1.0.0"), vec![1, 0, 0]);
}

#[test]
fn pv_01_00_01() {
    assert_eq!(app_lib::kvr::parse_version("1.0.1"), vec![1, 0, 1]);
}

#[test]
fn pv_01_00_02() {
    assert_eq!(app_lib::kvr::parse_version("1.0.2"), vec![1, 0, 2]);
}

#[test]
fn pv_01_00_03() {
    assert_eq!(app_lib::kvr::parse_version("1.0.3"), vec![1, 0, 3]);
}

#[test]
fn pv_01_00_04() {
    assert_eq!(app_lib::kvr::parse_version("1.0.4"), vec![1, 0, 4]);
}

#[test]
fn pv_01_00_05() {
    assert_eq!(app_lib::kvr::parse_version("1.0.5"), vec![1, 0, 5]);
}

#[test]
fn pv_01_00_06() {
    assert_eq!(app_lib::kvr::parse_version("1.0.6"), vec![1, 0, 6]);
}

#[test]
fn pv_01_00_07() {
    assert_eq!(app_lib::kvr::parse_version("1.0.7"), vec![1, 0, 7]);
}

#[test]
fn pv_01_01_00() {
    assert_eq!(app_lib::kvr::parse_version("1.1.0"), vec![1, 1, 0]);
}

#[test]
fn pv_01_01_01() {
    assert_eq!(app_lib::kvr::parse_version("1.1.1"), vec![1, 1, 1]);
}

#[test]
fn pv_01_01_02() {
    assert_eq!(app_lib::kvr::parse_version("1.1.2"), vec![1, 1, 2]);
}

#[test]
fn pv_01_01_03() {
    assert_eq!(app_lib::kvr::parse_version("1.1.3"), vec![1, 1, 3]);
}

#[test]
fn pv_01_01_04() {
    assert_eq!(app_lib::kvr::parse_version("1.1.4"), vec![1, 1, 4]);
}

#[test]
fn pv_01_01_05() {
    assert_eq!(app_lib::kvr::parse_version("1.1.5"), vec![1, 1, 5]);
}

#[test]
fn pv_01_01_06() {
    assert_eq!(app_lib::kvr::parse_version("1.1.6"), vec![1, 1, 6]);
}

#[test]
fn pv_01_01_07() {
    assert_eq!(app_lib::kvr::parse_version("1.1.7"), vec![1, 1, 7]);
}

#[test]
fn pv_01_02_00() {
    assert_eq!(app_lib::kvr::parse_version("1.2.0"), vec![1, 2, 0]);
}

#[test]
fn pv_01_02_01() {
    assert_eq!(app_lib::kvr::parse_version("1.2.1"), vec![1, 2, 1]);
}

#[test]
fn pv_01_02_02() {
    assert_eq!(app_lib::kvr::parse_version("1.2.2"), vec![1, 2, 2]);
}

#[test]
fn pv_01_02_03() {
    assert_eq!(app_lib::kvr::parse_version("1.2.3"), vec![1, 2, 3]);
}

#[test]
fn pv_01_02_04() {
    assert_eq!(app_lib::kvr::parse_version("1.2.4"), vec![1, 2, 4]);
}

#[test]
fn pv_01_02_05() {
    assert_eq!(app_lib::kvr::parse_version("1.2.5"), vec![1, 2, 5]);
}

#[test]
fn pv_01_02_06() {
    assert_eq!(app_lib::kvr::parse_version("1.2.6"), vec![1, 2, 6]);
}

#[test]
fn pv_01_02_07() {
    assert_eq!(app_lib::kvr::parse_version("1.2.7"), vec![1, 2, 7]);
}

#[test]
fn pv_01_03_00() {
    assert_eq!(app_lib::kvr::parse_version("1.3.0"), vec![1, 3, 0]);
}

#[test]
fn pv_01_03_01() {
    assert_eq!(app_lib::kvr::parse_version("1.3.1"), vec![1, 3, 1]);
}

#[test]
fn pv_01_03_02() {
    assert_eq!(app_lib::kvr::parse_version("1.3.2"), vec![1, 3, 2]);
}

#[test]
fn pv_01_03_03() {
    assert_eq!(app_lib::kvr::parse_version("1.3.3"), vec![1, 3, 3]);
}

#[test]
fn pv_01_03_04() {
    assert_eq!(app_lib::kvr::parse_version("1.3.4"), vec![1, 3, 4]);
}

#[test]
fn pv_01_03_05() {
    assert_eq!(app_lib::kvr::parse_version("1.3.5"), vec![1, 3, 5]);
}

#[test]
fn pv_01_03_06() {
    assert_eq!(app_lib::kvr::parse_version("1.3.6"), vec![1, 3, 6]);
}

#[test]
fn pv_01_03_07() {
    assert_eq!(app_lib::kvr::parse_version("1.3.7"), vec![1, 3, 7]);
}

#[test]
fn pv_01_04_00() {
    assert_eq!(app_lib::kvr::parse_version("1.4.0"), vec![1, 4, 0]);
}

#[test]
fn pv_01_04_01() {
    assert_eq!(app_lib::kvr::parse_version("1.4.1"), vec![1, 4, 1]);
}

#[test]
fn pv_01_04_02() {
    assert_eq!(app_lib::kvr::parse_version("1.4.2"), vec![1, 4, 2]);
}

#[test]
fn pv_01_04_03() {
    assert_eq!(app_lib::kvr::parse_version("1.4.3"), vec![1, 4, 3]);
}

#[test]
fn pv_01_04_04() {
    assert_eq!(app_lib::kvr::parse_version("1.4.4"), vec![1, 4, 4]);
}

#[test]
fn pv_01_04_05() {
    assert_eq!(app_lib::kvr::parse_version("1.4.5"), vec![1, 4, 5]);
}

#[test]
fn pv_01_04_06() {
    assert_eq!(app_lib::kvr::parse_version("1.4.6"), vec![1, 4, 6]);
}

#[test]
fn pv_01_04_07() {
    assert_eq!(app_lib::kvr::parse_version("1.4.7"), vec![1, 4, 7]);
}

#[test]
fn pv_01_05_00() {
    assert_eq!(app_lib::kvr::parse_version("1.5.0"), vec![1, 5, 0]);
}

#[test]
fn pv_01_05_01() {
    assert_eq!(app_lib::kvr::parse_version("1.5.1"), vec![1, 5, 1]);
}

#[test]
fn pv_01_05_02() {
    assert_eq!(app_lib::kvr::parse_version("1.5.2"), vec![1, 5, 2]);
}

#[test]
fn pv_01_05_03() {
    assert_eq!(app_lib::kvr::parse_version("1.5.3"), vec![1, 5, 3]);
}

#[test]
fn pv_01_05_04() {
    assert_eq!(app_lib::kvr::parse_version("1.5.4"), vec![1, 5, 4]);
}

#[test]
fn pv_01_05_05() {
    assert_eq!(app_lib::kvr::parse_version("1.5.5"), vec![1, 5, 5]);
}

#[test]
fn pv_01_05_06() {
    assert_eq!(app_lib::kvr::parse_version("1.5.6"), vec![1, 5, 6]);
}

#[test]
fn pv_01_05_07() {
    assert_eq!(app_lib::kvr::parse_version("1.5.7"), vec![1, 5, 7]);
}

#[test]
fn pv_01_06_00() {
    assert_eq!(app_lib::kvr::parse_version("1.6.0"), vec![1, 6, 0]);
}

#[test]
fn pv_01_06_01() {
    assert_eq!(app_lib::kvr::parse_version("1.6.1"), vec![1, 6, 1]);
}

#[test]
fn pv_01_06_02() {
    assert_eq!(app_lib::kvr::parse_version("1.6.2"), vec![1, 6, 2]);
}

#[test]
fn pv_01_06_03() {
    assert_eq!(app_lib::kvr::parse_version("1.6.3"), vec![1, 6, 3]);
}

#[test]
fn pv_01_06_04() {
    assert_eq!(app_lib::kvr::parse_version("1.6.4"), vec![1, 6, 4]);
}

#[test]
fn pv_01_06_05() {
    assert_eq!(app_lib::kvr::parse_version("1.6.5"), vec![1, 6, 5]);
}

#[test]
fn pv_01_06_06() {
    assert_eq!(app_lib::kvr::parse_version("1.6.6"), vec![1, 6, 6]);
}

#[test]
fn pv_01_06_07() {
    assert_eq!(app_lib::kvr::parse_version("1.6.7"), vec![1, 6, 7]);
}

#[test]
fn pv_01_07_00() {
    assert_eq!(app_lib::kvr::parse_version("1.7.0"), vec![1, 7, 0]);
}

#[test]
fn pv_01_07_01() {
    assert_eq!(app_lib::kvr::parse_version("1.7.1"), vec![1, 7, 1]);
}

#[test]
fn pv_01_07_02() {
    assert_eq!(app_lib::kvr::parse_version("1.7.2"), vec![1, 7, 2]);
}

#[test]
fn pv_01_07_03() {
    assert_eq!(app_lib::kvr::parse_version("1.7.3"), vec![1, 7, 3]);
}

#[test]
fn pv_01_07_04() {
    assert_eq!(app_lib::kvr::parse_version("1.7.4"), vec![1, 7, 4]);
}

#[test]
fn pv_01_07_05() {
    assert_eq!(app_lib::kvr::parse_version("1.7.5"), vec![1, 7, 5]);
}

#[test]
fn pv_01_07_06() {
    assert_eq!(app_lib::kvr::parse_version("1.7.6"), vec![1, 7, 6]);
}

#[test]
fn pv_01_07_07() {
    assert_eq!(app_lib::kvr::parse_version("1.7.7"), vec![1, 7, 7]);
}

#[test]
fn pv_02_00_00() {
    assert_eq!(app_lib::kvr::parse_version("2.0.0"), vec![2, 0, 0]);
}

#[test]
fn pv_02_00_01() {
    assert_eq!(app_lib::kvr::parse_version("2.0.1"), vec![2, 0, 1]);
}

#[test]
fn pv_02_00_02() {
    assert_eq!(app_lib::kvr::parse_version("2.0.2"), vec![2, 0, 2]);
}

#[test]
fn pv_02_00_03() {
    assert_eq!(app_lib::kvr::parse_version("2.0.3"), vec![2, 0, 3]);
}

#[test]
fn pv_02_00_04() {
    assert_eq!(app_lib::kvr::parse_version("2.0.4"), vec![2, 0, 4]);
}

#[test]
fn pv_02_00_05() {
    assert_eq!(app_lib::kvr::parse_version("2.0.5"), vec![2, 0, 5]);
}

#[test]
fn pv_02_00_06() {
    assert_eq!(app_lib::kvr::parse_version("2.0.6"), vec![2, 0, 6]);
}

#[test]
fn pv_02_00_07() {
    assert_eq!(app_lib::kvr::parse_version("2.0.7"), vec![2, 0, 7]);
}

#[test]
fn pv_02_01_00() {
    assert_eq!(app_lib::kvr::parse_version("2.1.0"), vec![2, 1, 0]);
}

#[test]
fn pv_02_01_01() {
    assert_eq!(app_lib::kvr::parse_version("2.1.1"), vec![2, 1, 1]);
}

#[test]
fn pv_02_01_02() {
    assert_eq!(app_lib::kvr::parse_version("2.1.2"), vec![2, 1, 2]);
}

#[test]
fn pv_02_01_03() {
    assert_eq!(app_lib::kvr::parse_version("2.1.3"), vec![2, 1, 3]);
}

#[test]
fn pv_02_01_04() {
    assert_eq!(app_lib::kvr::parse_version("2.1.4"), vec![2, 1, 4]);
}

#[test]
fn pv_02_01_05() {
    assert_eq!(app_lib::kvr::parse_version("2.1.5"), vec![2, 1, 5]);
}

#[test]
fn pv_02_01_06() {
    assert_eq!(app_lib::kvr::parse_version("2.1.6"), vec![2, 1, 6]);
}

#[test]
fn pv_02_01_07() {
    assert_eq!(app_lib::kvr::parse_version("2.1.7"), vec![2, 1, 7]);
}

#[test]
fn pv_02_02_00() {
    assert_eq!(app_lib::kvr::parse_version("2.2.0"), vec![2, 2, 0]);
}

#[test]
fn pv_02_02_01() {
    assert_eq!(app_lib::kvr::parse_version("2.2.1"), vec![2, 2, 1]);
}

#[test]
fn pv_02_02_02() {
    assert_eq!(app_lib::kvr::parse_version("2.2.2"), vec![2, 2, 2]);
}

#[test]
fn pv_02_02_03() {
    assert_eq!(app_lib::kvr::parse_version("2.2.3"), vec![2, 2, 3]);
}

#[test]
fn pv_02_02_04() {
    assert_eq!(app_lib::kvr::parse_version("2.2.4"), vec![2, 2, 4]);
}

#[test]
fn pv_02_02_05() {
    assert_eq!(app_lib::kvr::parse_version("2.2.5"), vec![2, 2, 5]);
}

#[test]
fn pv_02_02_06() {
    assert_eq!(app_lib::kvr::parse_version("2.2.6"), vec![2, 2, 6]);
}

#[test]
fn pv_02_02_07() {
    assert_eq!(app_lib::kvr::parse_version("2.2.7"), vec![2, 2, 7]);
}

#[test]
fn pv_02_03_00() {
    assert_eq!(app_lib::kvr::parse_version("2.3.0"), vec![2, 3, 0]);
}

#[test]
fn pv_02_03_01() {
    assert_eq!(app_lib::kvr::parse_version("2.3.1"), vec![2, 3, 1]);
}

#[test]
fn pv_02_03_02() {
    assert_eq!(app_lib::kvr::parse_version("2.3.2"), vec![2, 3, 2]);
}

#[test]
fn pv_02_03_03() {
    assert_eq!(app_lib::kvr::parse_version("2.3.3"), vec![2, 3, 3]);
}

#[test]
fn pv_02_03_04() {
    assert_eq!(app_lib::kvr::parse_version("2.3.4"), vec![2, 3, 4]);
}

#[test]
fn pv_02_03_05() {
    assert_eq!(app_lib::kvr::parse_version("2.3.5"), vec![2, 3, 5]);
}

#[test]
fn pv_02_03_06() {
    assert_eq!(app_lib::kvr::parse_version("2.3.6"), vec![2, 3, 6]);
}

#[test]
fn pv_02_03_07() {
    assert_eq!(app_lib::kvr::parse_version("2.3.7"), vec![2, 3, 7]);
}

#[test]
fn pv_02_04_00() {
    assert_eq!(app_lib::kvr::parse_version("2.4.0"), vec![2, 4, 0]);
}

#[test]
fn pv_02_04_01() {
    assert_eq!(app_lib::kvr::parse_version("2.4.1"), vec![2, 4, 1]);
}

#[test]
fn pv_02_04_02() {
    assert_eq!(app_lib::kvr::parse_version("2.4.2"), vec![2, 4, 2]);
}

#[test]
fn pv_02_04_03() {
    assert_eq!(app_lib::kvr::parse_version("2.4.3"), vec![2, 4, 3]);
}

#[test]
fn pv_02_04_04() {
    assert_eq!(app_lib::kvr::parse_version("2.4.4"), vec![2, 4, 4]);
}

#[test]
fn pv_02_04_05() {
    assert_eq!(app_lib::kvr::parse_version("2.4.5"), vec![2, 4, 5]);
}

#[test]
fn pv_02_04_06() {
    assert_eq!(app_lib::kvr::parse_version("2.4.6"), vec![2, 4, 6]);
}

#[test]
fn pv_02_04_07() {
    assert_eq!(app_lib::kvr::parse_version("2.4.7"), vec![2, 4, 7]);
}

#[test]
fn pv_02_05_00() {
    assert_eq!(app_lib::kvr::parse_version("2.5.0"), vec![2, 5, 0]);
}

#[test]
fn pv_02_05_01() {
    assert_eq!(app_lib::kvr::parse_version("2.5.1"), vec![2, 5, 1]);
}

#[test]
fn pv_02_05_02() {
    assert_eq!(app_lib::kvr::parse_version("2.5.2"), vec![2, 5, 2]);
}

#[test]
fn pv_02_05_03() {
    assert_eq!(app_lib::kvr::parse_version("2.5.3"), vec![2, 5, 3]);
}

#[test]
fn pv_02_05_04() {
    assert_eq!(app_lib::kvr::parse_version("2.5.4"), vec![2, 5, 4]);
}

#[test]
fn pv_02_05_05() {
    assert_eq!(app_lib::kvr::parse_version("2.5.5"), vec![2, 5, 5]);
}

#[test]
fn pv_02_05_06() {
    assert_eq!(app_lib::kvr::parse_version("2.5.6"), vec![2, 5, 6]);
}

#[test]
fn pv_02_05_07() {
    assert_eq!(app_lib::kvr::parse_version("2.5.7"), vec![2, 5, 7]);
}

#[test]
fn pv_02_06_00() {
    assert_eq!(app_lib::kvr::parse_version("2.6.0"), vec![2, 6, 0]);
}

#[test]
fn pv_02_06_01() {
    assert_eq!(app_lib::kvr::parse_version("2.6.1"), vec![2, 6, 1]);
}

#[test]
fn pv_02_06_02() {
    assert_eq!(app_lib::kvr::parse_version("2.6.2"), vec![2, 6, 2]);
}

#[test]
fn pv_02_06_03() {
    assert_eq!(app_lib::kvr::parse_version("2.6.3"), vec![2, 6, 3]);
}

#[test]
fn pv_02_06_04() {
    assert_eq!(app_lib::kvr::parse_version("2.6.4"), vec![2, 6, 4]);
}

#[test]
fn pv_02_06_05() {
    assert_eq!(app_lib::kvr::parse_version("2.6.5"), vec![2, 6, 5]);
}

#[test]
fn pv_02_06_06() {
    assert_eq!(app_lib::kvr::parse_version("2.6.6"), vec![2, 6, 6]);
}

#[test]
fn pv_02_06_07() {
    assert_eq!(app_lib::kvr::parse_version("2.6.7"), vec![2, 6, 7]);
}

#[test]
fn pv_02_07_00() {
    assert_eq!(app_lib::kvr::parse_version("2.7.0"), vec![2, 7, 0]);
}

#[test]
fn pv_02_07_01() {
    assert_eq!(app_lib::kvr::parse_version("2.7.1"), vec![2, 7, 1]);
}

#[test]
fn pv_02_07_02() {
    assert_eq!(app_lib::kvr::parse_version("2.7.2"), vec![2, 7, 2]);
}

#[test]
fn pv_02_07_03() {
    assert_eq!(app_lib::kvr::parse_version("2.7.3"), vec![2, 7, 3]);
}

#[test]
fn pv_02_07_04() {
    assert_eq!(app_lib::kvr::parse_version("2.7.4"), vec![2, 7, 4]);
}

#[test]
fn pv_02_07_05() {
    assert_eq!(app_lib::kvr::parse_version("2.7.5"), vec![2, 7, 5]);
}

#[test]
fn pv_02_07_06() {
    assert_eq!(app_lib::kvr::parse_version("2.7.6"), vec![2, 7, 6]);
}

#[test]
fn pv_02_07_07() {
    assert_eq!(app_lib::kvr::parse_version("2.7.7"), vec![2, 7, 7]);
}

#[test]
fn pv_03_00_00() {
    assert_eq!(app_lib::kvr::parse_version("3.0.0"), vec![3, 0, 0]);
}

#[test]
fn pv_03_00_01() {
    assert_eq!(app_lib::kvr::parse_version("3.0.1"), vec![3, 0, 1]);
}

#[test]
fn pv_03_00_02() {
    assert_eq!(app_lib::kvr::parse_version("3.0.2"), vec![3, 0, 2]);
}

#[test]
fn pv_03_00_03() {
    assert_eq!(app_lib::kvr::parse_version("3.0.3"), vec![3, 0, 3]);
}

#[test]
fn pv_03_00_04() {
    assert_eq!(app_lib::kvr::parse_version("3.0.4"), vec![3, 0, 4]);
}

#[test]
fn pv_03_00_05() {
    assert_eq!(app_lib::kvr::parse_version("3.0.5"), vec![3, 0, 5]);
}

#[test]
fn pv_03_00_06() {
    assert_eq!(app_lib::kvr::parse_version("3.0.6"), vec![3, 0, 6]);
}

#[test]
fn pv_03_00_07() {
    assert_eq!(app_lib::kvr::parse_version("3.0.7"), vec![3, 0, 7]);
}

#[test]
fn pv_03_01_00() {
    assert_eq!(app_lib::kvr::parse_version("3.1.0"), vec![3, 1, 0]);
}

#[test]
fn pv_03_01_01() {
    assert_eq!(app_lib::kvr::parse_version("3.1.1"), vec![3, 1, 1]);
}

#[test]
fn pv_03_01_02() {
    assert_eq!(app_lib::kvr::parse_version("3.1.2"), vec![3, 1, 2]);
}

#[test]
fn pv_03_01_03() {
    assert_eq!(app_lib::kvr::parse_version("3.1.3"), vec![3, 1, 3]);
}

#[test]
fn pv_03_01_04() {
    assert_eq!(app_lib::kvr::parse_version("3.1.4"), vec![3, 1, 4]);
}

#[test]
fn pv_03_01_05() {
    assert_eq!(app_lib::kvr::parse_version("3.1.5"), vec![3, 1, 5]);
}

#[test]
fn pv_03_01_06() {
    assert_eq!(app_lib::kvr::parse_version("3.1.6"), vec![3, 1, 6]);
}

#[test]
fn pv_03_01_07() {
    assert_eq!(app_lib::kvr::parse_version("3.1.7"), vec![3, 1, 7]);
}

#[test]
fn pv_03_02_00() {
    assert_eq!(app_lib::kvr::parse_version("3.2.0"), vec![3, 2, 0]);
}

#[test]
fn pv_03_02_01() {
    assert_eq!(app_lib::kvr::parse_version("3.2.1"), vec![3, 2, 1]);
}

#[test]
fn pv_03_02_02() {
    assert_eq!(app_lib::kvr::parse_version("3.2.2"), vec![3, 2, 2]);
}

#[test]
fn pv_03_02_03() {
    assert_eq!(app_lib::kvr::parse_version("3.2.3"), vec![3, 2, 3]);
}

#[test]
fn pv_03_02_04() {
    assert_eq!(app_lib::kvr::parse_version("3.2.4"), vec![3, 2, 4]);
}

#[test]
fn pv_03_02_05() {
    assert_eq!(app_lib::kvr::parse_version("3.2.5"), vec![3, 2, 5]);
}

#[test]
fn pv_03_02_06() {
    assert_eq!(app_lib::kvr::parse_version("3.2.6"), vec![3, 2, 6]);
}

#[test]
fn pv_03_02_07() {
    assert_eq!(app_lib::kvr::parse_version("3.2.7"), vec![3, 2, 7]);
}

#[test]
fn pv_03_03_00() {
    assert_eq!(app_lib::kvr::parse_version("3.3.0"), vec![3, 3, 0]);
}

#[test]
fn pv_03_03_01() {
    assert_eq!(app_lib::kvr::parse_version("3.3.1"), vec![3, 3, 1]);
}

#[test]
fn pv_03_03_02() {
    assert_eq!(app_lib::kvr::parse_version("3.3.2"), vec![3, 3, 2]);
}

#[test]
fn pv_03_03_03() {
    assert_eq!(app_lib::kvr::parse_version("3.3.3"), vec![3, 3, 3]);
}

#[test]
fn pv_03_03_04() {
    assert_eq!(app_lib::kvr::parse_version("3.3.4"), vec![3, 3, 4]);
}

#[test]
fn pv_03_03_05() {
    assert_eq!(app_lib::kvr::parse_version("3.3.5"), vec![3, 3, 5]);
}

#[test]
fn pv_03_03_06() {
    assert_eq!(app_lib::kvr::parse_version("3.3.6"), vec![3, 3, 6]);
}

#[test]
fn pv_03_03_07() {
    assert_eq!(app_lib::kvr::parse_version("3.3.7"), vec![3, 3, 7]);
}

#[test]
fn pv_03_04_00() {
    assert_eq!(app_lib::kvr::parse_version("3.4.0"), vec![3, 4, 0]);
}

#[test]
fn pv_03_04_01() {
    assert_eq!(app_lib::kvr::parse_version("3.4.1"), vec![3, 4, 1]);
}

#[test]
fn pv_03_04_02() {
    assert_eq!(app_lib::kvr::parse_version("3.4.2"), vec![3, 4, 2]);
}

#[test]
fn pv_03_04_03() {
    assert_eq!(app_lib::kvr::parse_version("3.4.3"), vec![3, 4, 3]);
}

#[test]
fn pv_03_04_04() {
    assert_eq!(app_lib::kvr::parse_version("3.4.4"), vec![3, 4, 4]);
}

#[test]
fn pv_03_04_05() {
    assert_eq!(app_lib::kvr::parse_version("3.4.5"), vec![3, 4, 5]);
}

#[test]
fn pv_03_04_06() {
    assert_eq!(app_lib::kvr::parse_version("3.4.6"), vec![3, 4, 6]);
}

#[test]
fn pv_03_04_07() {
    assert_eq!(app_lib::kvr::parse_version("3.4.7"), vec![3, 4, 7]);
}

#[test]
fn pv_03_05_00() {
    assert_eq!(app_lib::kvr::parse_version("3.5.0"), vec![3, 5, 0]);
}

#[test]
fn pv_03_05_01() {
    assert_eq!(app_lib::kvr::parse_version("3.5.1"), vec![3, 5, 1]);
}

#[test]
fn pv_03_05_02() {
    assert_eq!(app_lib::kvr::parse_version("3.5.2"), vec![3, 5, 2]);
}

#[test]
fn pv_03_05_03() {
    assert_eq!(app_lib::kvr::parse_version("3.5.3"), vec![3, 5, 3]);
}

#[test]
fn pv_03_05_04() {
    assert_eq!(app_lib::kvr::parse_version("3.5.4"), vec![3, 5, 4]);
}

#[test]
fn pv_03_05_05() {
    assert_eq!(app_lib::kvr::parse_version("3.5.5"), vec![3, 5, 5]);
}

#[test]
fn pv_03_05_06() {
    assert_eq!(app_lib::kvr::parse_version("3.5.6"), vec![3, 5, 6]);
}

#[test]
fn pv_03_05_07() {
    assert_eq!(app_lib::kvr::parse_version("3.5.7"), vec![3, 5, 7]);
}

#[test]
fn pv_03_06_00() {
    assert_eq!(app_lib::kvr::parse_version("3.6.0"), vec![3, 6, 0]);
}

#[test]
fn pv_03_06_01() {
    assert_eq!(app_lib::kvr::parse_version("3.6.1"), vec![3, 6, 1]);
}

#[test]
fn pv_03_06_02() {
    assert_eq!(app_lib::kvr::parse_version("3.6.2"), vec![3, 6, 2]);
}

#[test]
fn pv_03_06_03() {
    assert_eq!(app_lib::kvr::parse_version("3.6.3"), vec![3, 6, 3]);
}

#[test]
fn pv_03_06_04() {
    assert_eq!(app_lib::kvr::parse_version("3.6.4"), vec![3, 6, 4]);
}

#[test]
fn pv_03_06_05() {
    assert_eq!(app_lib::kvr::parse_version("3.6.5"), vec![3, 6, 5]);
}

#[test]
fn pv_03_06_06() {
    assert_eq!(app_lib::kvr::parse_version("3.6.6"), vec![3, 6, 6]);
}

#[test]
fn pv_03_06_07() {
    assert_eq!(app_lib::kvr::parse_version("3.6.7"), vec![3, 6, 7]);
}

#[test]
fn pv_03_07_00() {
    assert_eq!(app_lib::kvr::parse_version("3.7.0"), vec![3, 7, 0]);
}

#[test]
fn pv_03_07_01() {
    assert_eq!(app_lib::kvr::parse_version("3.7.1"), vec![3, 7, 1]);
}

#[test]
fn pv_03_07_02() {
    assert_eq!(app_lib::kvr::parse_version("3.7.2"), vec![3, 7, 2]);
}

#[test]
fn pv_03_07_03() {
    assert_eq!(app_lib::kvr::parse_version("3.7.3"), vec![3, 7, 3]);
}

#[test]
fn pv_03_07_04() {
    assert_eq!(app_lib::kvr::parse_version("3.7.4"), vec![3, 7, 4]);
}

#[test]
fn pv_03_07_05() {
    assert_eq!(app_lib::kvr::parse_version("3.7.5"), vec![3, 7, 5]);
}

#[test]
fn pv_03_07_06() {
    assert_eq!(app_lib::kvr::parse_version("3.7.6"), vec![3, 7, 6]);
}

#[test]
fn pv_03_07_07() {
    assert_eq!(app_lib::kvr::parse_version("3.7.7"), vec![3, 7, 7]);
}

#[test]
fn pv_04_00_00() {
    assert_eq!(app_lib::kvr::parse_version("4.0.0"), vec![4, 0, 0]);
}

#[test]
fn pv_04_00_01() {
    assert_eq!(app_lib::kvr::parse_version("4.0.1"), vec![4, 0, 1]);
}

#[test]
fn pv_04_00_02() {
    assert_eq!(app_lib::kvr::parse_version("4.0.2"), vec![4, 0, 2]);
}

#[test]
fn pv_04_00_03() {
    assert_eq!(app_lib::kvr::parse_version("4.0.3"), vec![4, 0, 3]);
}

#[test]
fn pv_04_00_04() {
    assert_eq!(app_lib::kvr::parse_version("4.0.4"), vec![4, 0, 4]);
}

#[test]
fn pv_04_00_05() {
    assert_eq!(app_lib::kvr::parse_version("4.0.5"), vec![4, 0, 5]);
}

#[test]
fn pv_04_00_06() {
    assert_eq!(app_lib::kvr::parse_version("4.0.6"), vec![4, 0, 6]);
}

#[test]
fn pv_04_00_07() {
    assert_eq!(app_lib::kvr::parse_version("4.0.7"), vec![4, 0, 7]);
}

#[test]
fn pv_04_01_00() {
    assert_eq!(app_lib::kvr::parse_version("4.1.0"), vec![4, 1, 0]);
}

#[test]
fn pv_04_01_01() {
    assert_eq!(app_lib::kvr::parse_version("4.1.1"), vec![4, 1, 1]);
}

#[test]
fn pv_04_01_02() {
    assert_eq!(app_lib::kvr::parse_version("4.1.2"), vec![4, 1, 2]);
}

#[test]
fn pv_04_01_03() {
    assert_eq!(app_lib::kvr::parse_version("4.1.3"), vec![4, 1, 3]);
}

#[test]
fn pv_04_01_04() {
    assert_eq!(app_lib::kvr::parse_version("4.1.4"), vec![4, 1, 4]);
}

#[test]
fn pv_04_01_05() {
    assert_eq!(app_lib::kvr::parse_version("4.1.5"), vec![4, 1, 5]);
}

#[test]
fn pv_04_01_06() {
    assert_eq!(app_lib::kvr::parse_version("4.1.6"), vec![4, 1, 6]);
}

#[test]
fn pv_04_01_07() {
    assert_eq!(app_lib::kvr::parse_version("4.1.7"), vec![4, 1, 7]);
}

#[test]
fn pv_04_02_00() {
    assert_eq!(app_lib::kvr::parse_version("4.2.0"), vec![4, 2, 0]);
}

#[test]
fn pv_04_02_01() {
    assert_eq!(app_lib::kvr::parse_version("4.2.1"), vec![4, 2, 1]);
}

#[test]
fn pv_04_02_02() {
    assert_eq!(app_lib::kvr::parse_version("4.2.2"), vec![4, 2, 2]);
}

#[test]
fn pv_04_02_03() {
    assert_eq!(app_lib::kvr::parse_version("4.2.3"), vec![4, 2, 3]);
}

#[test]
fn pv_04_02_04() {
    assert_eq!(app_lib::kvr::parse_version("4.2.4"), vec![4, 2, 4]);
}

#[test]
fn pv_04_02_05() {
    assert_eq!(app_lib::kvr::parse_version("4.2.5"), vec![4, 2, 5]);
}

#[test]
fn pv_04_02_06() {
    assert_eq!(app_lib::kvr::parse_version("4.2.6"), vec![4, 2, 6]);
}

#[test]
fn pv_04_02_07() {
    assert_eq!(app_lib::kvr::parse_version("4.2.7"), vec![4, 2, 7]);
}

#[test]
fn pv_04_03_00() {
    assert_eq!(app_lib::kvr::parse_version("4.3.0"), vec![4, 3, 0]);
}

#[test]
fn pv_04_03_01() {
    assert_eq!(app_lib::kvr::parse_version("4.3.1"), vec![4, 3, 1]);
}

#[test]
fn pv_04_03_02() {
    assert_eq!(app_lib::kvr::parse_version("4.3.2"), vec![4, 3, 2]);
}

#[test]
fn pv_04_03_03() {
    assert_eq!(app_lib::kvr::parse_version("4.3.3"), vec![4, 3, 3]);
}

#[test]
fn pv_04_03_04() {
    assert_eq!(app_lib::kvr::parse_version("4.3.4"), vec![4, 3, 4]);
}

#[test]
fn pv_04_03_05() {
    assert_eq!(app_lib::kvr::parse_version("4.3.5"), vec![4, 3, 5]);
}

#[test]
fn pv_04_03_06() {
    assert_eq!(app_lib::kvr::parse_version("4.3.6"), vec![4, 3, 6]);
}

#[test]
fn pv_04_03_07() {
    assert_eq!(app_lib::kvr::parse_version("4.3.7"), vec![4, 3, 7]);
}

#[test]
fn pv_04_04_00() {
    assert_eq!(app_lib::kvr::parse_version("4.4.0"), vec![4, 4, 0]);
}

#[test]
fn pv_04_04_01() {
    assert_eq!(app_lib::kvr::parse_version("4.4.1"), vec![4, 4, 1]);
}

#[test]
fn pv_04_04_02() {
    assert_eq!(app_lib::kvr::parse_version("4.4.2"), vec![4, 4, 2]);
}

#[test]
fn pv_04_04_03() {
    assert_eq!(app_lib::kvr::parse_version("4.4.3"), vec![4, 4, 3]);
}

#[test]
fn pv_04_04_04() {
    assert_eq!(app_lib::kvr::parse_version("4.4.4"), vec![4, 4, 4]);
}

#[test]
fn pv_04_04_05() {
    assert_eq!(app_lib::kvr::parse_version("4.4.5"), vec![4, 4, 5]);
}

#[test]
fn pv_04_04_06() {
    assert_eq!(app_lib::kvr::parse_version("4.4.6"), vec![4, 4, 6]);
}

#[test]
fn pv_04_04_07() {
    assert_eq!(app_lib::kvr::parse_version("4.4.7"), vec![4, 4, 7]);
}

#[test]
fn pv_04_05_00() {
    assert_eq!(app_lib::kvr::parse_version("4.5.0"), vec![4, 5, 0]);
}

#[test]
fn pv_04_05_01() {
    assert_eq!(app_lib::kvr::parse_version("4.5.1"), vec![4, 5, 1]);
}

#[test]
fn pv_04_05_02() {
    assert_eq!(app_lib::kvr::parse_version("4.5.2"), vec![4, 5, 2]);
}

#[test]
fn pv_04_05_03() {
    assert_eq!(app_lib::kvr::parse_version("4.5.3"), vec![4, 5, 3]);
}

#[test]
fn pv_04_05_04() {
    assert_eq!(app_lib::kvr::parse_version("4.5.4"), vec![4, 5, 4]);
}

#[test]
fn pv_04_05_05() {
    assert_eq!(app_lib::kvr::parse_version("4.5.5"), vec![4, 5, 5]);
}

#[test]
fn pv_04_05_06() {
    assert_eq!(app_lib::kvr::parse_version("4.5.6"), vec![4, 5, 6]);
}

#[test]
fn pv_04_05_07() {
    assert_eq!(app_lib::kvr::parse_version("4.5.7"), vec![4, 5, 7]);
}

#[test]
fn pv_04_06_00() {
    assert_eq!(app_lib::kvr::parse_version("4.6.0"), vec![4, 6, 0]);
}

#[test]
fn pv_04_06_01() {
    assert_eq!(app_lib::kvr::parse_version("4.6.1"), vec![4, 6, 1]);
}

#[test]
fn pv_04_06_02() {
    assert_eq!(app_lib::kvr::parse_version("4.6.2"), vec![4, 6, 2]);
}

#[test]
fn pv_04_06_03() {
    assert_eq!(app_lib::kvr::parse_version("4.6.3"), vec![4, 6, 3]);
}

#[test]
fn pv_04_06_04() {
    assert_eq!(app_lib::kvr::parse_version("4.6.4"), vec![4, 6, 4]);
}

#[test]
fn pv_04_06_05() {
    assert_eq!(app_lib::kvr::parse_version("4.6.5"), vec![4, 6, 5]);
}

#[test]
fn pv_04_06_06() {
    assert_eq!(app_lib::kvr::parse_version("4.6.6"), vec![4, 6, 6]);
}

#[test]
fn pv_04_06_07() {
    assert_eq!(app_lib::kvr::parse_version("4.6.7"), vec![4, 6, 7]);
}

#[test]
fn pv_04_07_00() {
    assert_eq!(app_lib::kvr::parse_version("4.7.0"), vec![4, 7, 0]);
}

#[test]
fn pv_04_07_01() {
    assert_eq!(app_lib::kvr::parse_version("4.7.1"), vec![4, 7, 1]);
}

#[test]
fn pv_04_07_02() {
    assert_eq!(app_lib::kvr::parse_version("4.7.2"), vec![4, 7, 2]);
}

#[test]
fn pv_04_07_03() {
    assert_eq!(app_lib::kvr::parse_version("4.7.3"), vec![4, 7, 3]);
}

#[test]
fn pv_04_07_04() {
    assert_eq!(app_lib::kvr::parse_version("4.7.4"), vec![4, 7, 4]);
}

#[test]
fn pv_04_07_05() {
    assert_eq!(app_lib::kvr::parse_version("4.7.5"), vec![4, 7, 5]);
}

#[test]
fn pv_04_07_06() {
    assert_eq!(app_lib::kvr::parse_version("4.7.6"), vec![4, 7, 6]);
}

#[test]
fn pv_04_07_07() {
    assert_eq!(app_lib::kvr::parse_version("4.7.7"), vec![4, 7, 7]);
}

#[test]
fn pv_05_00_00() {
    assert_eq!(app_lib::kvr::parse_version("5.0.0"), vec![5, 0, 0]);
}

#[test]
fn pv_05_00_01() {
    assert_eq!(app_lib::kvr::parse_version("5.0.1"), vec![5, 0, 1]);
}

#[test]
fn pv_05_00_02() {
    assert_eq!(app_lib::kvr::parse_version("5.0.2"), vec![5, 0, 2]);
}

#[test]
fn pv_05_00_03() {
    assert_eq!(app_lib::kvr::parse_version("5.0.3"), vec![5, 0, 3]);
}

#[test]
fn pv_05_00_04() {
    assert_eq!(app_lib::kvr::parse_version("5.0.4"), vec![5, 0, 4]);
}

#[test]
fn pv_05_00_05() {
    assert_eq!(app_lib::kvr::parse_version("5.0.5"), vec![5, 0, 5]);
}

#[test]
fn pv_05_00_06() {
    assert_eq!(app_lib::kvr::parse_version("5.0.6"), vec![5, 0, 6]);
}

#[test]
fn pv_05_00_07() {
    assert_eq!(app_lib::kvr::parse_version("5.0.7"), vec![5, 0, 7]);
}

#[test]
fn pv_05_01_00() {
    assert_eq!(app_lib::kvr::parse_version("5.1.0"), vec![5, 1, 0]);
}

#[test]
fn pv_05_01_01() {
    assert_eq!(app_lib::kvr::parse_version("5.1.1"), vec![5, 1, 1]);
}

#[test]
fn pv_05_01_02() {
    assert_eq!(app_lib::kvr::parse_version("5.1.2"), vec![5, 1, 2]);
}

#[test]
fn pv_05_01_03() {
    assert_eq!(app_lib::kvr::parse_version("5.1.3"), vec![5, 1, 3]);
}

#[test]
fn pv_05_01_04() {
    assert_eq!(app_lib::kvr::parse_version("5.1.4"), vec![5, 1, 4]);
}

#[test]
fn pv_05_01_05() {
    assert_eq!(app_lib::kvr::parse_version("5.1.5"), vec![5, 1, 5]);
}

#[test]
fn pv_05_01_06() {
    assert_eq!(app_lib::kvr::parse_version("5.1.6"), vec![5, 1, 6]);
}

#[test]
fn pv_05_01_07() {
    assert_eq!(app_lib::kvr::parse_version("5.1.7"), vec![5, 1, 7]);
}

#[test]
fn pv_05_02_00() {
    assert_eq!(app_lib::kvr::parse_version("5.2.0"), vec![5, 2, 0]);
}

#[test]
fn pv_05_02_01() {
    assert_eq!(app_lib::kvr::parse_version("5.2.1"), vec![5, 2, 1]);
}

#[test]
fn pv_05_02_02() {
    assert_eq!(app_lib::kvr::parse_version("5.2.2"), vec![5, 2, 2]);
}

#[test]
fn pv_05_02_03() {
    assert_eq!(app_lib::kvr::parse_version("5.2.3"), vec![5, 2, 3]);
}

#[test]
fn pv_05_02_04() {
    assert_eq!(app_lib::kvr::parse_version("5.2.4"), vec![5, 2, 4]);
}

#[test]
fn pv_05_02_05() {
    assert_eq!(app_lib::kvr::parse_version("5.2.5"), vec![5, 2, 5]);
}

#[test]
fn pv_05_02_06() {
    assert_eq!(app_lib::kvr::parse_version("5.2.6"), vec![5, 2, 6]);
}

#[test]
fn pv_05_02_07() {
    assert_eq!(app_lib::kvr::parse_version("5.2.7"), vec![5, 2, 7]);
}

#[test]
fn pv_05_03_00() {
    assert_eq!(app_lib::kvr::parse_version("5.3.0"), vec![5, 3, 0]);
}

#[test]
fn pv_05_03_01() {
    assert_eq!(app_lib::kvr::parse_version("5.3.1"), vec![5, 3, 1]);
}

#[test]
fn pv_05_03_02() {
    assert_eq!(app_lib::kvr::parse_version("5.3.2"), vec![5, 3, 2]);
}

#[test]
fn pv_05_03_03() {
    assert_eq!(app_lib::kvr::parse_version("5.3.3"), vec![5, 3, 3]);
}

#[test]
fn pv_05_03_04() {
    assert_eq!(app_lib::kvr::parse_version("5.3.4"), vec![5, 3, 4]);
}

#[test]
fn pv_05_03_05() {
    assert_eq!(app_lib::kvr::parse_version("5.3.5"), vec![5, 3, 5]);
}

#[test]
fn pv_05_03_06() {
    assert_eq!(app_lib::kvr::parse_version("5.3.6"), vec![5, 3, 6]);
}

#[test]
fn pv_05_03_07() {
    assert_eq!(app_lib::kvr::parse_version("5.3.7"), vec![5, 3, 7]);
}

#[test]
fn pv_05_04_00() {
    assert_eq!(app_lib::kvr::parse_version("5.4.0"), vec![5, 4, 0]);
}

#[test]
fn pv_05_04_01() {
    assert_eq!(app_lib::kvr::parse_version("5.4.1"), vec![5, 4, 1]);
}

#[test]
fn pv_05_04_02() {
    assert_eq!(app_lib::kvr::parse_version("5.4.2"), vec![5, 4, 2]);
}

#[test]
fn pv_05_04_03() {
    assert_eq!(app_lib::kvr::parse_version("5.4.3"), vec![5, 4, 3]);
}

#[test]
fn pv_05_04_04() {
    assert_eq!(app_lib::kvr::parse_version("5.4.4"), vec![5, 4, 4]);
}

#[test]
fn pv_05_04_05() {
    assert_eq!(app_lib::kvr::parse_version("5.4.5"), vec![5, 4, 5]);
}

#[test]
fn pv_05_04_06() {
    assert_eq!(app_lib::kvr::parse_version("5.4.6"), vec![5, 4, 6]);
}

#[test]
fn pv_05_04_07() {
    assert_eq!(app_lib::kvr::parse_version("5.4.7"), vec![5, 4, 7]);
}

#[test]
fn pv_05_05_00() {
    assert_eq!(app_lib::kvr::parse_version("5.5.0"), vec![5, 5, 0]);
}

#[test]
fn pv_05_05_01() {
    assert_eq!(app_lib::kvr::parse_version("5.5.1"), vec![5, 5, 1]);
}

#[test]
fn pv_05_05_02() {
    assert_eq!(app_lib::kvr::parse_version("5.5.2"), vec![5, 5, 2]);
}

#[test]
fn pv_05_05_03() {
    assert_eq!(app_lib::kvr::parse_version("5.5.3"), vec![5, 5, 3]);
}

#[test]
fn pv_05_05_04() {
    assert_eq!(app_lib::kvr::parse_version("5.5.4"), vec![5, 5, 4]);
}

#[test]
fn pv_05_05_05() {
    assert_eq!(app_lib::kvr::parse_version("5.5.5"), vec![5, 5, 5]);
}

#[test]
fn pv_05_05_06() {
    assert_eq!(app_lib::kvr::parse_version("5.5.6"), vec![5, 5, 6]);
}

#[test]
fn pv_05_05_07() {
    assert_eq!(app_lib::kvr::parse_version("5.5.7"), vec![5, 5, 7]);
}

#[test]
fn pv_05_06_00() {
    assert_eq!(app_lib::kvr::parse_version("5.6.0"), vec![5, 6, 0]);
}

#[test]
fn pv_05_06_01() {
    assert_eq!(app_lib::kvr::parse_version("5.6.1"), vec![5, 6, 1]);
}

#[test]
fn pv_05_06_02() {
    assert_eq!(app_lib::kvr::parse_version("5.6.2"), vec![5, 6, 2]);
}

#[test]
fn pv_05_06_03() {
    assert_eq!(app_lib::kvr::parse_version("5.6.3"), vec![5, 6, 3]);
}

#[test]
fn pv_05_06_04() {
    assert_eq!(app_lib::kvr::parse_version("5.6.4"), vec![5, 6, 4]);
}

#[test]
fn pv_05_06_05() {
    assert_eq!(app_lib::kvr::parse_version("5.6.5"), vec![5, 6, 5]);
}

#[test]
fn pv_05_06_06() {
    assert_eq!(app_lib::kvr::parse_version("5.6.6"), vec![5, 6, 6]);
}

#[test]
fn pv_05_06_07() {
    assert_eq!(app_lib::kvr::parse_version("5.6.7"), vec![5, 6, 7]);
}

#[test]
fn pv_05_07_00() {
    assert_eq!(app_lib::kvr::parse_version("5.7.0"), vec![5, 7, 0]);
}

#[test]
fn pv_05_07_01() {
    assert_eq!(app_lib::kvr::parse_version("5.7.1"), vec![5, 7, 1]);
}

#[test]
fn pv_05_07_02() {
    assert_eq!(app_lib::kvr::parse_version("5.7.2"), vec![5, 7, 2]);
}

#[test]
fn pv_05_07_03() {
    assert_eq!(app_lib::kvr::parse_version("5.7.3"), vec![5, 7, 3]);
}

#[test]
fn pv_05_07_04() {
    assert_eq!(app_lib::kvr::parse_version("5.7.4"), vec![5, 7, 4]);
}

#[test]
fn pv_05_07_05() {
    assert_eq!(app_lib::kvr::parse_version("5.7.5"), vec![5, 7, 5]);
}

#[test]
fn pv_05_07_06() {
    assert_eq!(app_lib::kvr::parse_version("5.7.6"), vec![5, 7, 6]);
}

#[test]
fn pv_05_07_07() {
    assert_eq!(app_lib::kvr::parse_version("5.7.7"), vec![5, 7, 7]);
}

#[test]
fn pv_06_00_00() {
    assert_eq!(app_lib::kvr::parse_version("6.0.0"), vec![6, 0, 0]);
}

#[test]
fn pv_06_00_01() {
    assert_eq!(app_lib::kvr::parse_version("6.0.1"), vec![6, 0, 1]);
}

#[test]
fn pv_06_00_02() {
    assert_eq!(app_lib::kvr::parse_version("6.0.2"), vec![6, 0, 2]);
}

#[test]
fn pv_06_00_03() {
    assert_eq!(app_lib::kvr::parse_version("6.0.3"), vec![6, 0, 3]);
}

#[test]
fn pv_06_00_04() {
    assert_eq!(app_lib::kvr::parse_version("6.0.4"), vec![6, 0, 4]);
}

#[test]
fn pv_06_00_05() {
    assert_eq!(app_lib::kvr::parse_version("6.0.5"), vec![6, 0, 5]);
}

#[test]
fn pv_06_00_06() {
    assert_eq!(app_lib::kvr::parse_version("6.0.6"), vec![6, 0, 6]);
}

#[test]
fn pv_06_00_07() {
    assert_eq!(app_lib::kvr::parse_version("6.0.7"), vec![6, 0, 7]);
}

#[test]
fn pv_06_01_00() {
    assert_eq!(app_lib::kvr::parse_version("6.1.0"), vec![6, 1, 0]);
}

#[test]
fn pv_06_01_01() {
    assert_eq!(app_lib::kvr::parse_version("6.1.1"), vec![6, 1, 1]);
}

#[test]
fn pv_06_01_02() {
    assert_eq!(app_lib::kvr::parse_version("6.1.2"), vec![6, 1, 2]);
}

#[test]
fn pv_06_01_03() {
    assert_eq!(app_lib::kvr::parse_version("6.1.3"), vec![6, 1, 3]);
}

#[test]
fn pv_06_01_04() {
    assert_eq!(app_lib::kvr::parse_version("6.1.4"), vec![6, 1, 4]);
}

#[test]
fn pv_06_01_05() {
    assert_eq!(app_lib::kvr::parse_version("6.1.5"), vec![6, 1, 5]);
}

#[test]
fn pv_06_01_06() {
    assert_eq!(app_lib::kvr::parse_version("6.1.6"), vec![6, 1, 6]);
}

#[test]
fn pv_06_01_07() {
    assert_eq!(app_lib::kvr::parse_version("6.1.7"), vec![6, 1, 7]);
}

#[test]
fn pv_06_02_00() {
    assert_eq!(app_lib::kvr::parse_version("6.2.0"), vec![6, 2, 0]);
}

#[test]
fn pv_06_02_01() {
    assert_eq!(app_lib::kvr::parse_version("6.2.1"), vec![6, 2, 1]);
}

#[test]
fn pv_06_02_02() {
    assert_eq!(app_lib::kvr::parse_version("6.2.2"), vec![6, 2, 2]);
}

#[test]
fn pv_06_02_03() {
    assert_eq!(app_lib::kvr::parse_version("6.2.3"), vec![6, 2, 3]);
}

#[test]
fn pv_06_02_04() {
    assert_eq!(app_lib::kvr::parse_version("6.2.4"), vec![6, 2, 4]);
}

#[test]
fn pv_06_02_05() {
    assert_eq!(app_lib::kvr::parse_version("6.2.5"), vec![6, 2, 5]);
}

#[test]
fn pv_06_02_06() {
    assert_eq!(app_lib::kvr::parse_version("6.2.6"), vec![6, 2, 6]);
}

#[test]
fn pv_06_02_07() {
    assert_eq!(app_lib::kvr::parse_version("6.2.7"), vec![6, 2, 7]);
}

#[test]
fn pv_06_03_00() {
    assert_eq!(app_lib::kvr::parse_version("6.3.0"), vec![6, 3, 0]);
}

#[test]
fn pv_06_03_01() {
    assert_eq!(app_lib::kvr::parse_version("6.3.1"), vec![6, 3, 1]);
}

#[test]
fn pv_06_03_02() {
    assert_eq!(app_lib::kvr::parse_version("6.3.2"), vec![6, 3, 2]);
}

#[test]
fn pv_06_03_03() {
    assert_eq!(app_lib::kvr::parse_version("6.3.3"), vec![6, 3, 3]);
}

#[test]
fn pv_06_03_04() {
    assert_eq!(app_lib::kvr::parse_version("6.3.4"), vec![6, 3, 4]);
}

#[test]
fn pv_06_03_05() {
    assert_eq!(app_lib::kvr::parse_version("6.3.5"), vec![6, 3, 5]);
}

#[test]
fn pv_06_03_06() {
    assert_eq!(app_lib::kvr::parse_version("6.3.6"), vec![6, 3, 6]);
}

#[test]
fn pv_06_03_07() {
    assert_eq!(app_lib::kvr::parse_version("6.3.7"), vec![6, 3, 7]);
}

#[test]
fn pv_06_04_00() {
    assert_eq!(app_lib::kvr::parse_version("6.4.0"), vec![6, 4, 0]);
}

#[test]
fn pv_06_04_01() {
    assert_eq!(app_lib::kvr::parse_version("6.4.1"), vec![6, 4, 1]);
}

#[test]
fn pv_06_04_02() {
    assert_eq!(app_lib::kvr::parse_version("6.4.2"), vec![6, 4, 2]);
}

#[test]
fn pv_06_04_03() {
    assert_eq!(app_lib::kvr::parse_version("6.4.3"), vec![6, 4, 3]);
}

#[test]
fn pv_06_04_04() {
    assert_eq!(app_lib::kvr::parse_version("6.4.4"), vec![6, 4, 4]);
}

#[test]
fn pv_06_04_05() {
    assert_eq!(app_lib::kvr::parse_version("6.4.5"), vec![6, 4, 5]);
}

#[test]
fn pv_06_04_06() {
    assert_eq!(app_lib::kvr::parse_version("6.4.6"), vec![6, 4, 6]);
}

#[test]
fn pv_06_04_07() {
    assert_eq!(app_lib::kvr::parse_version("6.4.7"), vec![6, 4, 7]);
}

#[test]
fn pv_06_05_00() {
    assert_eq!(app_lib::kvr::parse_version("6.5.0"), vec![6, 5, 0]);
}

#[test]
fn pv_06_05_01() {
    assert_eq!(app_lib::kvr::parse_version("6.5.1"), vec![6, 5, 1]);
}

#[test]
fn pv_06_05_02() {
    assert_eq!(app_lib::kvr::parse_version("6.5.2"), vec![6, 5, 2]);
}

#[test]
fn pv_06_05_03() {
    assert_eq!(app_lib::kvr::parse_version("6.5.3"), vec![6, 5, 3]);
}

#[test]
fn pv_06_05_04() {
    assert_eq!(app_lib::kvr::parse_version("6.5.4"), vec![6, 5, 4]);
}

#[test]
fn pv_06_05_05() {
    assert_eq!(app_lib::kvr::parse_version("6.5.5"), vec![6, 5, 5]);
}

#[test]
fn pv_06_05_06() {
    assert_eq!(app_lib::kvr::parse_version("6.5.6"), vec![6, 5, 6]);
}

#[test]
fn pv_06_05_07() {
    assert_eq!(app_lib::kvr::parse_version("6.5.7"), vec![6, 5, 7]);
}

#[test]
fn pv_06_06_00() {
    assert_eq!(app_lib::kvr::parse_version("6.6.0"), vec![6, 6, 0]);
}

#[test]
fn pv_06_06_01() {
    assert_eq!(app_lib::kvr::parse_version("6.6.1"), vec![6, 6, 1]);
}

#[test]
fn pv_06_06_02() {
    assert_eq!(app_lib::kvr::parse_version("6.6.2"), vec![6, 6, 2]);
}

#[test]
fn pv_06_06_03() {
    assert_eq!(app_lib::kvr::parse_version("6.6.3"), vec![6, 6, 3]);
}

#[test]
fn pv_06_06_04() {
    assert_eq!(app_lib::kvr::parse_version("6.6.4"), vec![6, 6, 4]);
}

#[test]
fn pv_06_06_05() {
    assert_eq!(app_lib::kvr::parse_version("6.6.5"), vec![6, 6, 5]);
}

#[test]
fn pv_06_06_06() {
    assert_eq!(app_lib::kvr::parse_version("6.6.6"), vec![6, 6, 6]);
}

#[test]
fn pv_06_06_07() {
    assert_eq!(app_lib::kvr::parse_version("6.6.7"), vec![6, 6, 7]);
}

#[test]
fn pv_06_07_00() {
    assert_eq!(app_lib::kvr::parse_version("6.7.0"), vec![6, 7, 0]);
}

#[test]
fn pv_06_07_01() {
    assert_eq!(app_lib::kvr::parse_version("6.7.1"), vec![6, 7, 1]);
}

#[test]
fn pv_06_07_02() {
    assert_eq!(app_lib::kvr::parse_version("6.7.2"), vec![6, 7, 2]);
}

#[test]
fn pv_06_07_03() {
    assert_eq!(app_lib::kvr::parse_version("6.7.3"), vec![6, 7, 3]);
}

#[test]
fn pv_06_07_04() {
    assert_eq!(app_lib::kvr::parse_version("6.7.4"), vec![6, 7, 4]);
}

#[test]
fn pv_06_07_05() {
    assert_eq!(app_lib::kvr::parse_version("6.7.5"), vec![6, 7, 5]);
}

#[test]
fn pv_06_07_06() {
    assert_eq!(app_lib::kvr::parse_version("6.7.6"), vec![6, 7, 6]);
}

#[test]
fn pv_06_07_07() {
    assert_eq!(app_lib::kvr::parse_version("6.7.7"), vec![6, 7, 7]);
}

#[test]
fn pv_07_00_00() {
    assert_eq!(app_lib::kvr::parse_version("7.0.0"), vec![7, 0, 0]);
}

#[test]
fn pv_07_00_01() {
    assert_eq!(app_lib::kvr::parse_version("7.0.1"), vec![7, 0, 1]);
}

#[test]
fn pv_07_00_02() {
    assert_eq!(app_lib::kvr::parse_version("7.0.2"), vec![7, 0, 2]);
}

#[test]
fn pv_07_00_03() {
    assert_eq!(app_lib::kvr::parse_version("7.0.3"), vec![7, 0, 3]);
}

#[test]
fn pv_07_00_04() {
    assert_eq!(app_lib::kvr::parse_version("7.0.4"), vec![7, 0, 4]);
}

#[test]
fn pv_07_00_05() {
    assert_eq!(app_lib::kvr::parse_version("7.0.5"), vec![7, 0, 5]);
}

#[test]
fn pv_07_00_06() {
    assert_eq!(app_lib::kvr::parse_version("7.0.6"), vec![7, 0, 6]);
}

#[test]
fn pv_07_00_07() {
    assert_eq!(app_lib::kvr::parse_version("7.0.7"), vec![7, 0, 7]);
}

#[test]
fn pv_07_01_00() {
    assert_eq!(app_lib::kvr::parse_version("7.1.0"), vec![7, 1, 0]);
}

#[test]
fn pv_07_01_01() {
    assert_eq!(app_lib::kvr::parse_version("7.1.1"), vec![7, 1, 1]);
}

#[test]
fn pv_07_01_02() {
    assert_eq!(app_lib::kvr::parse_version("7.1.2"), vec![7, 1, 2]);
}

#[test]
fn pv_07_01_03() {
    assert_eq!(app_lib::kvr::parse_version("7.1.3"), vec![7, 1, 3]);
}

#[test]
fn pv_07_01_04() {
    assert_eq!(app_lib::kvr::parse_version("7.1.4"), vec![7, 1, 4]);
}

#[test]
fn pv_07_01_05() {
    assert_eq!(app_lib::kvr::parse_version("7.1.5"), vec![7, 1, 5]);
}

#[test]
fn pv_07_01_06() {
    assert_eq!(app_lib::kvr::parse_version("7.1.6"), vec![7, 1, 6]);
}

#[test]
fn pv_07_01_07() {
    assert_eq!(app_lib::kvr::parse_version("7.1.7"), vec![7, 1, 7]);
}

#[test]
fn pv_07_02_00() {
    assert_eq!(app_lib::kvr::parse_version("7.2.0"), vec![7, 2, 0]);
}

#[test]
fn pv_07_02_01() {
    assert_eq!(app_lib::kvr::parse_version("7.2.1"), vec![7, 2, 1]);
}

#[test]
fn pv_07_02_02() {
    assert_eq!(app_lib::kvr::parse_version("7.2.2"), vec![7, 2, 2]);
}

#[test]
fn pv_07_02_03() {
    assert_eq!(app_lib::kvr::parse_version("7.2.3"), vec![7, 2, 3]);
}

#[test]
fn pv_07_02_04() {
    assert_eq!(app_lib::kvr::parse_version("7.2.4"), vec![7, 2, 4]);
}

#[test]
fn pv_07_02_05() {
    assert_eq!(app_lib::kvr::parse_version("7.2.5"), vec![7, 2, 5]);
}

#[test]
fn pv_07_02_06() {
    assert_eq!(app_lib::kvr::parse_version("7.2.6"), vec![7, 2, 6]);
}

#[test]
fn pv_07_02_07() {
    assert_eq!(app_lib::kvr::parse_version("7.2.7"), vec![7, 2, 7]);
}

#[test]
fn pv_07_03_00() {
    assert_eq!(app_lib::kvr::parse_version("7.3.0"), vec![7, 3, 0]);
}

#[test]
fn pv_07_03_01() {
    assert_eq!(app_lib::kvr::parse_version("7.3.1"), vec![7, 3, 1]);
}

#[test]
fn pv_07_03_02() {
    assert_eq!(app_lib::kvr::parse_version("7.3.2"), vec![7, 3, 2]);
}

#[test]
fn pv_07_03_03() {
    assert_eq!(app_lib::kvr::parse_version("7.3.3"), vec![7, 3, 3]);
}

#[test]
fn pv_07_03_04() {
    assert_eq!(app_lib::kvr::parse_version("7.3.4"), vec![7, 3, 4]);
}

#[test]
fn pv_07_03_05() {
    assert_eq!(app_lib::kvr::parse_version("7.3.5"), vec![7, 3, 5]);
}

#[test]
fn pv_07_03_06() {
    assert_eq!(app_lib::kvr::parse_version("7.3.6"), vec![7, 3, 6]);
}

#[test]
fn pv_07_03_07() {
    assert_eq!(app_lib::kvr::parse_version("7.3.7"), vec![7, 3, 7]);
}

#[test]
fn pv_07_04_00() {
    assert_eq!(app_lib::kvr::parse_version("7.4.0"), vec![7, 4, 0]);
}

#[test]
fn pv_07_04_01() {
    assert_eq!(app_lib::kvr::parse_version("7.4.1"), vec![7, 4, 1]);
}

#[test]
fn pv_07_04_02() {
    assert_eq!(app_lib::kvr::parse_version("7.4.2"), vec![7, 4, 2]);
}

#[test]
fn pv_07_04_03() {
    assert_eq!(app_lib::kvr::parse_version("7.4.3"), vec![7, 4, 3]);
}

#[test]
fn pv_07_04_04() {
    assert_eq!(app_lib::kvr::parse_version("7.4.4"), vec![7, 4, 4]);
}

#[test]
fn pv_07_04_05() {
    assert_eq!(app_lib::kvr::parse_version("7.4.5"), vec![7, 4, 5]);
}

#[test]
fn pv_07_04_06() {
    assert_eq!(app_lib::kvr::parse_version("7.4.6"), vec![7, 4, 6]);
}

#[test]
fn pv_07_04_07() {
    assert_eq!(app_lib::kvr::parse_version("7.4.7"), vec![7, 4, 7]);
}

#[test]
fn pv_07_05_00() {
    assert_eq!(app_lib::kvr::parse_version("7.5.0"), vec![7, 5, 0]);
}

#[test]
fn pv_07_05_01() {
    assert_eq!(app_lib::kvr::parse_version("7.5.1"), vec![7, 5, 1]);
}

#[test]
fn pv_07_05_02() {
    assert_eq!(app_lib::kvr::parse_version("7.5.2"), vec![7, 5, 2]);
}

#[test]
fn pv_07_05_03() {
    assert_eq!(app_lib::kvr::parse_version("7.5.3"), vec![7, 5, 3]);
}

#[test]
fn pv_07_05_04() {
    assert_eq!(app_lib::kvr::parse_version("7.5.4"), vec![7, 5, 4]);
}

#[test]
fn pv_07_05_05() {
    assert_eq!(app_lib::kvr::parse_version("7.5.5"), vec![7, 5, 5]);
}

#[test]
fn pv_07_05_06() {
    assert_eq!(app_lib::kvr::parse_version("7.5.6"), vec![7, 5, 6]);
}

#[test]
fn pv_07_05_07() {
    assert_eq!(app_lib::kvr::parse_version("7.5.7"), vec![7, 5, 7]);
}

#[test]
fn pv_07_06_00() {
    assert_eq!(app_lib::kvr::parse_version("7.6.0"), vec![7, 6, 0]);
}

#[test]
fn pv_07_06_01() {
    assert_eq!(app_lib::kvr::parse_version("7.6.1"), vec![7, 6, 1]);
}

#[test]
fn pv_07_06_02() {
    assert_eq!(app_lib::kvr::parse_version("7.6.2"), vec![7, 6, 2]);
}

#[test]
fn pv_07_06_03() {
    assert_eq!(app_lib::kvr::parse_version("7.6.3"), vec![7, 6, 3]);
}

#[test]
fn pv_07_06_04() {
    assert_eq!(app_lib::kvr::parse_version("7.6.4"), vec![7, 6, 4]);
}

#[test]
fn pv_07_06_05() {
    assert_eq!(app_lib::kvr::parse_version("7.6.5"), vec![7, 6, 5]);
}

#[test]
fn pv_07_06_06() {
    assert_eq!(app_lib::kvr::parse_version("7.6.6"), vec![7, 6, 6]);
}

#[test]
fn pv_07_06_07() {
    assert_eq!(app_lib::kvr::parse_version("7.6.7"), vec![7, 6, 7]);
}

#[test]
fn pv_07_07_00() {
    assert_eq!(app_lib::kvr::parse_version("7.7.0"), vec![7, 7, 0]);
}

#[test]
fn pv_07_07_01() {
    assert_eq!(app_lib::kvr::parse_version("7.7.1"), vec![7, 7, 1]);
}

#[test]
fn pv_07_07_02() {
    assert_eq!(app_lib::kvr::parse_version("7.7.2"), vec![7, 7, 2]);
}

#[test]
fn pv_07_07_03() {
    assert_eq!(app_lib::kvr::parse_version("7.7.3"), vec![7, 7, 3]);
}

#[test]
fn pv_07_07_04() {
    assert_eq!(app_lib::kvr::parse_version("7.7.4"), vec![7, 7, 4]);
}

#[test]
fn pv_07_07_05() {
    assert_eq!(app_lib::kvr::parse_version("7.7.5"), vec![7, 7, 5]);
}

#[test]
fn pv_07_07_06() {
    assert_eq!(app_lib::kvr::parse_version("7.7.6"), vec![7, 7, 6]);
}

#[test]
fn pv_07_07_07() {
    assert_eq!(app_lib::kvr::parse_version("7.7.7"), vec![7, 7, 7]);
}

#[test]
fn pv_edge_00() {
    assert_eq!(app_lib::kvr::parse_version("0.0.1"), vec![0, 0, 1]);
}

#[test]
fn pv_edge_01() {
    assert_eq!(app_lib::kvr::parse_version("0.1.0"), vec![0, 1, 0]);
}

#[test]
fn pv_edge_02() {
    assert_eq!(app_lib::kvr::parse_version("1.0.10"), vec![1, 0, 10]);
}

#[test]
fn pv_edge_03() {
    assert_eq!(app_lib::kvr::parse_version("1.0.9"), vec![1, 0, 9]);
}

#[test]
fn pv_edge_04() {
    assert_eq!(app_lib::kvr::parse_version("10.20.30"), vec![10, 20, 30]);
}

#[test]
fn pv_edge_05() {
    assert_eq!(app_lib::kvr::parse_version("99.99.99"), vec![99, 99, 99]);
}

#[test]
fn pv_edge_06() {
    assert_eq!(app_lib::kvr::parse_version("1.0."), vec![1, 0, 0]);
}

#[test]
fn pv_edge_07() {
    assert_eq!(app_lib::kvr::parse_version(".5.0"), vec![0, 5, 0]);
}

#[test]
fn pv_edge_08() {
    assert_eq!(app_lib::kvr::parse_version("1.x.3"), vec![1, 0, 3]);
}

#[test]
fn pv_edge_09() {
    assert_eq!(app_lib::kvr::parse_version("1..2"), vec![1, 0, 2]);
}

#[test]
fn pv_edge_10() {
    assert_eq!(app_lib::kvr::parse_version("..3"), vec![0, 0, 3]);
}

#[test]
fn pv_edge_11() {
    assert_eq!(app_lib::kvr::parse_version("2147483647.0"), vec![2147483647, 0]);
}

#[test]
fn pv_edge_12() {
    assert_eq!(app_lib::kvr::parse_version("01.02.03"), vec![1, 2, 3]);
}

#[test]
fn pv_edge_13() {
    assert_eq!(app_lib::kvr::parse_version("1.0.beta"), vec![1, 0, 0]);
}

#[test]
fn pv_edge_14() {
    assert_eq!(app_lib::kvr::parse_version("1.-1.0"), vec![1, -1, 0]);
}

#[test]
fn pv_edge_15() {
    assert_eq!(app_lib::kvr::parse_version("2.0.0.0.1"), vec![2, 0, 0, 0, 1]);
}

// ── compare_versions ───────────────────────────────────────────────

#[test]
fn cmp_lt_0000() {
    assert_eq!(app_lib::kvr::compare_versions("0.0.0", "0.0.1"), Ordering::Less);
}

#[test]
fn cmp_lt_0001() {
    assert_eq!(app_lib::kvr::compare_versions("0.0.1", "0.0.2"), Ordering::Less);
}

#[test]
fn cmp_lt_0002() {
    assert_eq!(app_lib::kvr::compare_versions("0.0.2", "0.0.3"), Ordering::Less);
}

#[test]
fn cmp_lt_0003() {
    assert_eq!(app_lib::kvr::compare_versions("0.0.3", "0.0.4"), Ordering::Less);
}

#[test]
fn cmp_lt_0004() {
    assert_eq!(app_lib::kvr::compare_versions("0.0.4", "0.0.5"), Ordering::Less);
}

#[test]
fn cmp_lt_0005() {
    assert_eq!(app_lib::kvr::compare_versions("0.0.5", "0.0.6"), Ordering::Less);
}

#[test]
fn cmp_lt_0006() {
    assert_eq!(app_lib::kvr::compare_versions("0.0.6", "0.0.7"), Ordering::Less);
}

#[test]
fn cmp_lt_0007() {
    assert_eq!(app_lib::kvr::compare_versions("0.0.7", "0.1.0"), Ordering::Less);
}

#[test]
fn cmp_lt_0008() {
    assert_eq!(app_lib::kvr::compare_versions("0.1.0", "0.1.1"), Ordering::Less);
}

#[test]
fn cmp_lt_0009() {
    assert_eq!(app_lib::kvr::compare_versions("0.1.1", "0.1.2"), Ordering::Less);
}

#[test]
fn cmp_lt_0010() {
    assert_eq!(app_lib::kvr::compare_versions("0.1.2", "0.1.3"), Ordering::Less);
}

#[test]
fn cmp_lt_0011() {
    assert_eq!(app_lib::kvr::compare_versions("0.1.3", "0.1.4"), Ordering::Less);
}

#[test]
fn cmp_lt_0012() {
    assert_eq!(app_lib::kvr::compare_versions("0.1.4", "0.1.5"), Ordering::Less);
}

#[test]
fn cmp_lt_0013() {
    assert_eq!(app_lib::kvr::compare_versions("0.1.5", "0.1.6"), Ordering::Less);
}

#[test]
fn cmp_lt_0014() {
    assert_eq!(app_lib::kvr::compare_versions("0.1.6", "0.1.7"), Ordering::Less);
}

#[test]
fn cmp_lt_0015() {
    assert_eq!(app_lib::kvr::compare_versions("0.1.7", "0.2.0"), Ordering::Less);
}

#[test]
fn cmp_lt_0016() {
    assert_eq!(app_lib::kvr::compare_versions("0.2.0", "0.2.1"), Ordering::Less);
}

#[test]
fn cmp_lt_0017() {
    assert_eq!(app_lib::kvr::compare_versions("0.2.1", "0.2.2"), Ordering::Less);
}

#[test]
fn cmp_lt_0018() {
    assert_eq!(app_lib::kvr::compare_versions("0.2.2", "0.2.3"), Ordering::Less);
}

#[test]
fn cmp_lt_0019() {
    assert_eq!(app_lib::kvr::compare_versions("0.2.3", "0.2.4"), Ordering::Less);
}

#[test]
fn cmp_lt_0020() {
    assert_eq!(app_lib::kvr::compare_versions("0.2.4", "0.2.5"), Ordering::Less);
}

#[test]
fn cmp_lt_0021() {
    assert_eq!(app_lib::kvr::compare_versions("0.2.5", "0.2.6"), Ordering::Less);
}

#[test]
fn cmp_lt_0022() {
    assert_eq!(app_lib::kvr::compare_versions("0.2.6", "0.2.7"), Ordering::Less);
}

#[test]
fn cmp_lt_0023() {
    assert_eq!(app_lib::kvr::compare_versions("0.2.7", "0.3.0"), Ordering::Less);
}

#[test]
fn cmp_lt_0024() {
    assert_eq!(app_lib::kvr::compare_versions("0.3.0", "0.3.1"), Ordering::Less);
}

#[test]
fn cmp_lt_0025() {
    assert_eq!(app_lib::kvr::compare_versions("0.3.1", "0.3.2"), Ordering::Less);
}

#[test]
fn cmp_lt_0026() {
    assert_eq!(app_lib::kvr::compare_versions("0.3.2", "0.3.3"), Ordering::Less);
}

#[test]
fn cmp_lt_0027() {
    assert_eq!(app_lib::kvr::compare_versions("0.3.3", "0.3.4"), Ordering::Less);
}

#[test]
fn cmp_lt_0028() {
    assert_eq!(app_lib::kvr::compare_versions("0.3.4", "0.3.5"), Ordering::Less);
}

#[test]
fn cmp_lt_0029() {
    assert_eq!(app_lib::kvr::compare_versions("0.3.5", "0.3.6"), Ordering::Less);
}

#[test]
fn cmp_lt_0030() {
    assert_eq!(app_lib::kvr::compare_versions("0.3.6", "0.3.7"), Ordering::Less);
}

#[test]
fn cmp_lt_0031() {
    assert_eq!(app_lib::kvr::compare_versions("0.3.7", "0.4.0"), Ordering::Less);
}

#[test]
fn cmp_lt_0032() {
    assert_eq!(app_lib::kvr::compare_versions("0.4.0", "0.4.1"), Ordering::Less);
}

#[test]
fn cmp_lt_0033() {
    assert_eq!(app_lib::kvr::compare_versions("0.4.1", "0.4.2"), Ordering::Less);
}

#[test]
fn cmp_lt_0034() {
    assert_eq!(app_lib::kvr::compare_versions("0.4.2", "0.4.3"), Ordering::Less);
}

#[test]
fn cmp_lt_0035() {
    assert_eq!(app_lib::kvr::compare_versions("0.4.3", "0.4.4"), Ordering::Less);
}

#[test]
fn cmp_lt_0036() {
    assert_eq!(app_lib::kvr::compare_versions("0.4.4", "0.4.5"), Ordering::Less);
}

#[test]
fn cmp_lt_0037() {
    assert_eq!(app_lib::kvr::compare_versions("0.4.5", "0.4.6"), Ordering::Less);
}

#[test]
fn cmp_lt_0038() {
    assert_eq!(app_lib::kvr::compare_versions("0.4.6", "0.4.7"), Ordering::Less);
}

#[test]
fn cmp_lt_0039() {
    assert_eq!(app_lib::kvr::compare_versions("0.4.7", "0.5.0"), Ordering::Less);
}

#[test]
fn cmp_lt_0040() {
    assert_eq!(app_lib::kvr::compare_versions("0.5.0", "0.5.1"), Ordering::Less);
}

#[test]
fn cmp_lt_0041() {
    assert_eq!(app_lib::kvr::compare_versions("0.5.1", "0.5.2"), Ordering::Less);
}

#[test]
fn cmp_lt_0042() {
    assert_eq!(app_lib::kvr::compare_versions("0.5.2", "0.5.3"), Ordering::Less);
}

#[test]
fn cmp_lt_0043() {
    assert_eq!(app_lib::kvr::compare_versions("0.5.3", "0.5.4"), Ordering::Less);
}

#[test]
fn cmp_lt_0044() {
    assert_eq!(app_lib::kvr::compare_versions("0.5.4", "0.5.5"), Ordering::Less);
}

#[test]
fn cmp_lt_0045() {
    assert_eq!(app_lib::kvr::compare_versions("0.5.5", "0.5.6"), Ordering::Less);
}

#[test]
fn cmp_lt_0046() {
    assert_eq!(app_lib::kvr::compare_versions("0.5.6", "0.5.7"), Ordering::Less);
}

#[test]
fn cmp_lt_0047() {
    assert_eq!(app_lib::kvr::compare_versions("0.5.7", "0.6.0"), Ordering::Less);
}

#[test]
fn cmp_lt_0048() {
    assert_eq!(app_lib::kvr::compare_versions("0.6.0", "0.6.1"), Ordering::Less);
}

#[test]
fn cmp_lt_0049() {
    assert_eq!(app_lib::kvr::compare_versions("0.6.1", "0.6.2"), Ordering::Less);
}

#[test]
fn cmp_lt_0050() {
    assert_eq!(app_lib::kvr::compare_versions("0.6.2", "0.6.3"), Ordering::Less);
}

#[test]
fn cmp_lt_0051() {
    assert_eq!(app_lib::kvr::compare_versions("0.6.3", "0.6.4"), Ordering::Less);
}

#[test]
fn cmp_lt_0052() {
    assert_eq!(app_lib::kvr::compare_versions("0.6.4", "0.6.5"), Ordering::Less);
}

#[test]
fn cmp_lt_0053() {
    assert_eq!(app_lib::kvr::compare_versions("0.6.5", "0.6.6"), Ordering::Less);
}

#[test]
fn cmp_lt_0054() {
    assert_eq!(app_lib::kvr::compare_versions("0.6.6", "0.6.7"), Ordering::Less);
}

#[test]
fn cmp_lt_0055() {
    assert_eq!(app_lib::kvr::compare_versions("0.6.7", "0.7.0"), Ordering::Less);
}

#[test]
fn cmp_lt_0056() {
    assert_eq!(app_lib::kvr::compare_versions("0.7.0", "0.7.1"), Ordering::Less);
}

#[test]
fn cmp_lt_0057() {
    assert_eq!(app_lib::kvr::compare_versions("0.7.1", "0.7.2"), Ordering::Less);
}

#[test]
fn cmp_lt_0058() {
    assert_eq!(app_lib::kvr::compare_versions("0.7.2", "0.7.3"), Ordering::Less);
}

#[test]
fn cmp_lt_0059() {
    assert_eq!(app_lib::kvr::compare_versions("0.7.3", "0.7.4"), Ordering::Less);
}

#[test]
fn cmp_lt_0060() {
    assert_eq!(app_lib::kvr::compare_versions("0.7.4", "0.7.5"), Ordering::Less);
}

#[test]
fn cmp_lt_0061() {
    assert_eq!(app_lib::kvr::compare_versions("0.7.5", "0.7.6"), Ordering::Less);
}

#[test]
fn cmp_lt_0062() {
    assert_eq!(app_lib::kvr::compare_versions("0.7.6", "0.7.7"), Ordering::Less);
}

#[test]
fn cmp_lt_0063() {
    assert_eq!(app_lib::kvr::compare_versions("0.7.7", "1.0.0"), Ordering::Less);
}

#[test]
fn cmp_lt_0064() {
    assert_eq!(app_lib::kvr::compare_versions("1.0.0", "1.0.1"), Ordering::Less);
}

#[test]
fn cmp_lt_0065() {
    assert_eq!(app_lib::kvr::compare_versions("1.0.1", "1.0.2"), Ordering::Less);
}

#[test]
fn cmp_lt_0066() {
    assert_eq!(app_lib::kvr::compare_versions("1.0.2", "1.0.3"), Ordering::Less);
}

#[test]
fn cmp_lt_0067() {
    assert_eq!(app_lib::kvr::compare_versions("1.0.3", "1.0.4"), Ordering::Less);
}

#[test]
fn cmp_lt_0068() {
    assert_eq!(app_lib::kvr::compare_versions("1.0.4", "1.0.5"), Ordering::Less);
}

#[test]
fn cmp_lt_0069() {
    assert_eq!(app_lib::kvr::compare_versions("1.0.5", "1.0.6"), Ordering::Less);
}

#[test]
fn cmp_lt_0070() {
    assert_eq!(app_lib::kvr::compare_versions("1.0.6", "1.0.7"), Ordering::Less);
}

#[test]
fn cmp_lt_0071() {
    assert_eq!(app_lib::kvr::compare_versions("1.0.7", "1.1.0"), Ordering::Less);
}

#[test]
fn cmp_lt_0072() {
    assert_eq!(app_lib::kvr::compare_versions("1.1.0", "1.1.1"), Ordering::Less);
}

#[test]
fn cmp_lt_0073() {
    assert_eq!(app_lib::kvr::compare_versions("1.1.1", "1.1.2"), Ordering::Less);
}

#[test]
fn cmp_lt_0074() {
    assert_eq!(app_lib::kvr::compare_versions("1.1.2", "1.1.3"), Ordering::Less);
}

#[test]
fn cmp_lt_0075() {
    assert_eq!(app_lib::kvr::compare_versions("1.1.3", "1.1.4"), Ordering::Less);
}

#[test]
fn cmp_lt_0076() {
    assert_eq!(app_lib::kvr::compare_versions("1.1.4", "1.1.5"), Ordering::Less);
}

#[test]
fn cmp_lt_0077() {
    assert_eq!(app_lib::kvr::compare_versions("1.1.5", "1.1.6"), Ordering::Less);
}

#[test]
fn cmp_lt_0078() {
    assert_eq!(app_lib::kvr::compare_versions("1.1.6", "1.1.7"), Ordering::Less);
}

#[test]
fn cmp_lt_0079() {
    assert_eq!(app_lib::kvr::compare_versions("1.1.7", "1.2.0"), Ordering::Less);
}

#[test]
fn cmp_lt_0080() {
    assert_eq!(app_lib::kvr::compare_versions("1.2.0", "1.2.1"), Ordering::Less);
}

#[test]
fn cmp_lt_0081() {
    assert_eq!(app_lib::kvr::compare_versions("1.2.1", "1.2.2"), Ordering::Less);
}

#[test]
fn cmp_lt_0082() {
    assert_eq!(app_lib::kvr::compare_versions("1.2.2", "1.2.3"), Ordering::Less);
}

#[test]
fn cmp_lt_0083() {
    assert_eq!(app_lib::kvr::compare_versions("1.2.3", "1.2.4"), Ordering::Less);
}

#[test]
fn cmp_lt_0084() {
    assert_eq!(app_lib::kvr::compare_versions("1.2.4", "1.2.5"), Ordering::Less);
}

#[test]
fn cmp_lt_0085() {
    assert_eq!(app_lib::kvr::compare_versions("1.2.5", "1.2.6"), Ordering::Less);
}

#[test]
fn cmp_lt_0086() {
    assert_eq!(app_lib::kvr::compare_versions("1.2.6", "1.2.7"), Ordering::Less);
}

#[test]
fn cmp_lt_0087() {
    assert_eq!(app_lib::kvr::compare_versions("1.2.7", "1.3.0"), Ordering::Less);
}

#[test]
fn cmp_lt_0088() {
    assert_eq!(app_lib::kvr::compare_versions("1.3.0", "1.3.1"), Ordering::Less);
}

#[test]
fn cmp_lt_0089() {
    assert_eq!(app_lib::kvr::compare_versions("1.3.1", "1.3.2"), Ordering::Less);
}

#[test]
fn cmp_lt_0090() {
    assert_eq!(app_lib::kvr::compare_versions("1.3.2", "1.3.3"), Ordering::Less);
}

#[test]
fn cmp_lt_0091() {
    assert_eq!(app_lib::kvr::compare_versions("1.3.3", "1.3.4"), Ordering::Less);
}

#[test]
fn cmp_lt_0092() {
    assert_eq!(app_lib::kvr::compare_versions("1.3.4", "1.3.5"), Ordering::Less);
}

#[test]
fn cmp_lt_0093() {
    assert_eq!(app_lib::kvr::compare_versions("1.3.5", "1.3.6"), Ordering::Less);
}

#[test]
fn cmp_lt_0094() {
    assert_eq!(app_lib::kvr::compare_versions("1.3.6", "1.3.7"), Ordering::Less);
}

#[test]
fn cmp_lt_0095() {
    assert_eq!(app_lib::kvr::compare_versions("1.3.7", "1.4.0"), Ordering::Less);
}

#[test]
fn cmp_lt_0096() {
    assert_eq!(app_lib::kvr::compare_versions("1.4.0", "1.4.1"), Ordering::Less);
}

#[test]
fn cmp_lt_0097() {
    assert_eq!(app_lib::kvr::compare_versions("1.4.1", "1.4.2"), Ordering::Less);
}

#[test]
fn cmp_lt_0098() {
    assert_eq!(app_lib::kvr::compare_versions("1.4.2", "1.4.3"), Ordering::Less);
}

#[test]
fn cmp_lt_0099() {
    assert_eq!(app_lib::kvr::compare_versions("1.4.3", "1.4.4"), Ordering::Less);
}

#[test]
fn cmp_lt_0100() {
    assert_eq!(app_lib::kvr::compare_versions("1.4.4", "1.4.5"), Ordering::Less);
}

#[test]
fn cmp_lt_0101() {
    assert_eq!(app_lib::kvr::compare_versions("1.4.5", "1.4.6"), Ordering::Less);
}

#[test]
fn cmp_lt_0102() {
    assert_eq!(app_lib::kvr::compare_versions("1.4.6", "1.4.7"), Ordering::Less);
}

#[test]
fn cmp_lt_0103() {
    assert_eq!(app_lib::kvr::compare_versions("1.4.7", "1.5.0"), Ordering::Less);
}

#[test]
fn cmp_lt_0104() {
    assert_eq!(app_lib::kvr::compare_versions("1.5.0", "1.5.1"), Ordering::Less);
}

#[test]
fn cmp_lt_0105() {
    assert_eq!(app_lib::kvr::compare_versions("1.5.1", "1.5.2"), Ordering::Less);
}

#[test]
fn cmp_lt_0106() {
    assert_eq!(app_lib::kvr::compare_versions("1.5.2", "1.5.3"), Ordering::Less);
}

#[test]
fn cmp_lt_0107() {
    assert_eq!(app_lib::kvr::compare_versions("1.5.3", "1.5.4"), Ordering::Less);
}

#[test]
fn cmp_lt_0108() {
    assert_eq!(app_lib::kvr::compare_versions("1.5.4", "1.5.5"), Ordering::Less);
}

#[test]
fn cmp_lt_0109() {
    assert_eq!(app_lib::kvr::compare_versions("1.5.5", "1.5.6"), Ordering::Less);
}

#[test]
fn cmp_lt_0110() {
    assert_eq!(app_lib::kvr::compare_versions("1.5.6", "1.5.7"), Ordering::Less);
}

#[test]
fn cmp_lt_0111() {
    assert_eq!(app_lib::kvr::compare_versions("1.5.7", "1.6.0"), Ordering::Less);
}

#[test]
fn cmp_lt_0112() {
    assert_eq!(app_lib::kvr::compare_versions("1.6.0", "1.6.1"), Ordering::Less);
}

#[test]
fn cmp_lt_0113() {
    assert_eq!(app_lib::kvr::compare_versions("1.6.1", "1.6.2"), Ordering::Less);
}

#[test]
fn cmp_lt_0114() {
    assert_eq!(app_lib::kvr::compare_versions("1.6.2", "1.6.3"), Ordering::Less);
}

#[test]
fn cmp_lt_0115() {
    assert_eq!(app_lib::kvr::compare_versions("1.6.3", "1.6.4"), Ordering::Less);
}

#[test]
fn cmp_lt_0116() {
    assert_eq!(app_lib::kvr::compare_versions("1.6.4", "1.6.5"), Ordering::Less);
}

#[test]
fn cmp_lt_0117() {
    assert_eq!(app_lib::kvr::compare_versions("1.6.5", "1.6.6"), Ordering::Less);
}

#[test]
fn cmp_lt_0118() {
    assert_eq!(app_lib::kvr::compare_versions("1.6.6", "1.6.7"), Ordering::Less);
}

#[test]
fn cmp_lt_0119() {
    assert_eq!(app_lib::kvr::compare_versions("1.6.7", "1.7.0"), Ordering::Less);
}

#[test]
fn cmp_lt_0120() {
    assert_eq!(app_lib::kvr::compare_versions("1.7.0", "1.7.1"), Ordering::Less);
}

#[test]
fn cmp_lt_0121() {
    assert_eq!(app_lib::kvr::compare_versions("1.7.1", "1.7.2"), Ordering::Less);
}

#[test]
fn cmp_lt_0122() {
    assert_eq!(app_lib::kvr::compare_versions("1.7.2", "1.7.3"), Ordering::Less);
}

#[test]
fn cmp_lt_0123() {
    assert_eq!(app_lib::kvr::compare_versions("1.7.3", "1.7.4"), Ordering::Less);
}

#[test]
fn cmp_lt_0124() {
    assert_eq!(app_lib::kvr::compare_versions("1.7.4", "1.7.5"), Ordering::Less);
}

#[test]
fn cmp_lt_0125() {
    assert_eq!(app_lib::kvr::compare_versions("1.7.5", "1.7.6"), Ordering::Less);
}

#[test]
fn cmp_lt_0126() {
    assert_eq!(app_lib::kvr::compare_versions("1.7.6", "1.7.7"), Ordering::Less);
}

#[test]
fn cmp_lt_0127() {
    assert_eq!(app_lib::kvr::compare_versions("1.7.7", "2.0.0"), Ordering::Less);
}

#[test]
fn cmp_lt_0128() {
    assert_eq!(app_lib::kvr::compare_versions("2.0.0", "2.0.1"), Ordering::Less);
}

#[test]
fn cmp_lt_0129() {
    assert_eq!(app_lib::kvr::compare_versions("2.0.1", "2.0.2"), Ordering::Less);
}

#[test]
fn cmp_lt_0130() {
    assert_eq!(app_lib::kvr::compare_versions("2.0.2", "2.0.3"), Ordering::Less);
}

#[test]
fn cmp_lt_0131() {
    assert_eq!(app_lib::kvr::compare_versions("2.0.3", "2.0.4"), Ordering::Less);
}

#[test]
fn cmp_lt_0132() {
    assert_eq!(app_lib::kvr::compare_versions("2.0.4", "2.0.5"), Ordering::Less);
}

#[test]
fn cmp_lt_0133() {
    assert_eq!(app_lib::kvr::compare_versions("2.0.5", "2.0.6"), Ordering::Less);
}

#[test]
fn cmp_lt_0134() {
    assert_eq!(app_lib::kvr::compare_versions("2.0.6", "2.0.7"), Ordering::Less);
}

#[test]
fn cmp_lt_0135() {
    assert_eq!(app_lib::kvr::compare_versions("2.0.7", "2.1.0"), Ordering::Less);
}

#[test]
fn cmp_lt_0136() {
    assert_eq!(app_lib::kvr::compare_versions("2.1.0", "2.1.1"), Ordering::Less);
}

#[test]
fn cmp_lt_0137() {
    assert_eq!(app_lib::kvr::compare_versions("2.1.1", "2.1.2"), Ordering::Less);
}

#[test]
fn cmp_lt_0138() {
    assert_eq!(app_lib::kvr::compare_versions("2.1.2", "2.1.3"), Ordering::Less);
}

#[test]
fn cmp_lt_0139() {
    assert_eq!(app_lib::kvr::compare_versions("2.1.3", "2.1.4"), Ordering::Less);
}

#[test]
fn cmp_lt_0140() {
    assert_eq!(app_lib::kvr::compare_versions("2.1.4", "2.1.5"), Ordering::Less);
}

#[test]
fn cmp_lt_0141() {
    assert_eq!(app_lib::kvr::compare_versions("2.1.5", "2.1.6"), Ordering::Less);
}

#[test]
fn cmp_lt_0142() {
    assert_eq!(app_lib::kvr::compare_versions("2.1.6", "2.1.7"), Ordering::Less);
}

#[test]
fn cmp_lt_0143() {
    assert_eq!(app_lib::kvr::compare_versions("2.1.7", "2.2.0"), Ordering::Less);
}

#[test]
fn cmp_lt_0144() {
    assert_eq!(app_lib::kvr::compare_versions("2.2.0", "2.2.1"), Ordering::Less);
}

#[test]
fn cmp_lt_0145() {
    assert_eq!(app_lib::kvr::compare_versions("2.2.1", "2.2.2"), Ordering::Less);
}

#[test]
fn cmp_lt_0146() {
    assert_eq!(app_lib::kvr::compare_versions("2.2.2", "2.2.3"), Ordering::Less);
}

#[test]
fn cmp_lt_0147() {
    assert_eq!(app_lib::kvr::compare_versions("2.2.3", "2.2.4"), Ordering::Less);
}

#[test]
fn cmp_lt_0148() {
    assert_eq!(app_lib::kvr::compare_versions("2.2.4", "2.2.5"), Ordering::Less);
}

#[test]
fn cmp_lt_0149() {
    assert_eq!(app_lib::kvr::compare_versions("2.2.5", "2.2.6"), Ordering::Less);
}

#[test]
fn cmp_lt_0150() {
    assert_eq!(app_lib::kvr::compare_versions("2.2.6", "2.2.7"), Ordering::Less);
}

#[test]
fn cmp_lt_0151() {
    assert_eq!(app_lib::kvr::compare_versions("2.2.7", "2.3.0"), Ordering::Less);
}

#[test]
fn cmp_lt_0152() {
    assert_eq!(app_lib::kvr::compare_versions("2.3.0", "2.3.1"), Ordering::Less);
}

#[test]
fn cmp_lt_0153() {
    assert_eq!(app_lib::kvr::compare_versions("2.3.1", "2.3.2"), Ordering::Less);
}

#[test]
fn cmp_lt_0154() {
    assert_eq!(app_lib::kvr::compare_versions("2.3.2", "2.3.3"), Ordering::Less);
}

#[test]
fn cmp_lt_0155() {
    assert_eq!(app_lib::kvr::compare_versions("2.3.3", "2.3.4"), Ordering::Less);
}

#[test]
fn cmp_lt_0156() {
    assert_eq!(app_lib::kvr::compare_versions("2.3.4", "2.3.5"), Ordering::Less);
}

#[test]
fn cmp_lt_0157() {
    assert_eq!(app_lib::kvr::compare_versions("2.3.5", "2.3.6"), Ordering::Less);
}

#[test]
fn cmp_lt_0158() {
    assert_eq!(app_lib::kvr::compare_versions("2.3.6", "2.3.7"), Ordering::Less);
}

#[test]
fn cmp_lt_0159() {
    assert_eq!(app_lib::kvr::compare_versions("2.3.7", "2.4.0"), Ordering::Less);
}

#[test]
fn cmp_lt_0160() {
    assert_eq!(app_lib::kvr::compare_versions("2.4.0", "2.4.1"), Ordering::Less);
}

#[test]
fn cmp_lt_0161() {
    assert_eq!(app_lib::kvr::compare_versions("2.4.1", "2.4.2"), Ordering::Less);
}

#[test]
fn cmp_lt_0162() {
    assert_eq!(app_lib::kvr::compare_versions("2.4.2", "2.4.3"), Ordering::Less);
}

#[test]
fn cmp_lt_0163() {
    assert_eq!(app_lib::kvr::compare_versions("2.4.3", "2.4.4"), Ordering::Less);
}

#[test]
fn cmp_lt_0164() {
    assert_eq!(app_lib::kvr::compare_versions("2.4.4", "2.4.5"), Ordering::Less);
}

#[test]
fn cmp_lt_0165() {
    assert_eq!(app_lib::kvr::compare_versions("2.4.5", "2.4.6"), Ordering::Less);
}

#[test]
fn cmp_lt_0166() {
    assert_eq!(app_lib::kvr::compare_versions("2.4.6", "2.4.7"), Ordering::Less);
}

#[test]
fn cmp_lt_0167() {
    assert_eq!(app_lib::kvr::compare_versions("2.4.7", "2.5.0"), Ordering::Less);
}

#[test]
fn cmp_lt_0168() {
    assert_eq!(app_lib::kvr::compare_versions("2.5.0", "2.5.1"), Ordering::Less);
}

#[test]
fn cmp_lt_0169() {
    assert_eq!(app_lib::kvr::compare_versions("2.5.1", "2.5.2"), Ordering::Less);
}

#[test]
fn cmp_lt_0170() {
    assert_eq!(app_lib::kvr::compare_versions("2.5.2", "2.5.3"), Ordering::Less);
}

#[test]
fn cmp_lt_0171() {
    assert_eq!(app_lib::kvr::compare_versions("2.5.3", "2.5.4"), Ordering::Less);
}

#[test]
fn cmp_lt_0172() {
    assert_eq!(app_lib::kvr::compare_versions("2.5.4", "2.5.5"), Ordering::Less);
}

#[test]
fn cmp_lt_0173() {
    assert_eq!(app_lib::kvr::compare_versions("2.5.5", "2.5.6"), Ordering::Less);
}

#[test]
fn cmp_lt_0174() {
    assert_eq!(app_lib::kvr::compare_versions("2.5.6", "2.5.7"), Ordering::Less);
}

#[test]
fn cmp_lt_0175() {
    assert_eq!(app_lib::kvr::compare_versions("2.5.7", "2.6.0"), Ordering::Less);
}

#[test]
fn cmp_lt_0176() {
    assert_eq!(app_lib::kvr::compare_versions("2.6.0", "2.6.1"), Ordering::Less);
}

#[test]
fn cmp_lt_0177() {
    assert_eq!(app_lib::kvr::compare_versions("2.6.1", "2.6.2"), Ordering::Less);
}

#[test]
fn cmp_lt_0178() {
    assert_eq!(app_lib::kvr::compare_versions("2.6.2", "2.6.3"), Ordering::Less);
}

#[test]
fn cmp_lt_0179() {
    assert_eq!(app_lib::kvr::compare_versions("2.6.3", "2.6.4"), Ordering::Less);
}

#[test]
fn cmp_lt_0180() {
    assert_eq!(app_lib::kvr::compare_versions("2.6.4", "2.6.5"), Ordering::Less);
}

#[test]
fn cmp_lt_0181() {
    assert_eq!(app_lib::kvr::compare_versions("2.6.5", "2.6.6"), Ordering::Less);
}

#[test]
fn cmp_lt_0182() {
    assert_eq!(app_lib::kvr::compare_versions("2.6.6", "2.6.7"), Ordering::Less);
}

#[test]
fn cmp_lt_0183() {
    assert_eq!(app_lib::kvr::compare_versions("2.6.7", "2.7.0"), Ordering::Less);
}

#[test]
fn cmp_lt_0184() {
    assert_eq!(app_lib::kvr::compare_versions("2.7.0", "2.7.1"), Ordering::Less);
}

#[test]
fn cmp_lt_0185() {
    assert_eq!(app_lib::kvr::compare_versions("2.7.1", "2.7.2"), Ordering::Less);
}

#[test]
fn cmp_lt_0186() {
    assert_eq!(app_lib::kvr::compare_versions("2.7.2", "2.7.3"), Ordering::Less);
}

#[test]
fn cmp_lt_0187() {
    assert_eq!(app_lib::kvr::compare_versions("2.7.3", "2.7.4"), Ordering::Less);
}

#[test]
fn cmp_lt_0188() {
    assert_eq!(app_lib::kvr::compare_versions("2.7.4", "2.7.5"), Ordering::Less);
}

#[test]
fn cmp_lt_0189() {
    assert_eq!(app_lib::kvr::compare_versions("2.7.5", "2.7.6"), Ordering::Less);
}

#[test]
fn cmp_lt_0190() {
    assert_eq!(app_lib::kvr::compare_versions("2.7.6", "2.7.7"), Ordering::Less);
}

#[test]
fn cmp_lt_0191() {
    assert_eq!(app_lib::kvr::compare_versions("2.7.7", "3.0.0"), Ordering::Less);
}

#[test]
fn cmp_lt_0192() {
    assert_eq!(app_lib::kvr::compare_versions("3.0.0", "3.0.1"), Ordering::Less);
}

#[test]
fn cmp_lt_0193() {
    assert_eq!(app_lib::kvr::compare_versions("3.0.1", "3.0.2"), Ordering::Less);
}

#[test]
fn cmp_lt_0194() {
    assert_eq!(app_lib::kvr::compare_versions("3.0.2", "3.0.3"), Ordering::Less);
}

#[test]
fn cmp_lt_0195() {
    assert_eq!(app_lib::kvr::compare_versions("3.0.3", "3.0.4"), Ordering::Less);
}

#[test]
fn cmp_lt_0196() {
    assert_eq!(app_lib::kvr::compare_versions("3.0.4", "3.0.5"), Ordering::Less);
}

#[test]
fn cmp_lt_0197() {
    assert_eq!(app_lib::kvr::compare_versions("3.0.5", "3.0.6"), Ordering::Less);
}

#[test]
fn cmp_lt_0198() {
    assert_eq!(app_lib::kvr::compare_versions("3.0.6", "3.0.7"), Ordering::Less);
}

#[test]
fn cmp_lt_0199() {
    assert_eq!(app_lib::kvr::compare_versions("3.0.7", "3.1.0"), Ordering::Less);
}

#[test]
fn cmp_lt_0200() {
    assert_eq!(app_lib::kvr::compare_versions("3.1.0", "3.1.1"), Ordering::Less);
}

#[test]
fn cmp_lt_0201() {
    assert_eq!(app_lib::kvr::compare_versions("3.1.1", "3.1.2"), Ordering::Less);
}

#[test]
fn cmp_lt_0202() {
    assert_eq!(app_lib::kvr::compare_versions("3.1.2", "3.1.3"), Ordering::Less);
}

#[test]
fn cmp_lt_0203() {
    assert_eq!(app_lib::kvr::compare_versions("3.1.3", "3.1.4"), Ordering::Less);
}

#[test]
fn cmp_lt_0204() {
    assert_eq!(app_lib::kvr::compare_versions("3.1.4", "3.1.5"), Ordering::Less);
}

#[test]
fn cmp_lt_0205() {
    assert_eq!(app_lib::kvr::compare_versions("3.1.5", "3.1.6"), Ordering::Less);
}

#[test]
fn cmp_lt_0206() {
    assert_eq!(app_lib::kvr::compare_versions("3.1.6", "3.1.7"), Ordering::Less);
}

#[test]
fn cmp_lt_0207() {
    assert_eq!(app_lib::kvr::compare_versions("3.1.7", "3.2.0"), Ordering::Less);
}

#[test]
fn cmp_lt_0208() {
    assert_eq!(app_lib::kvr::compare_versions("3.2.0", "3.2.1"), Ordering::Less);
}

#[test]
fn cmp_lt_0209() {
    assert_eq!(app_lib::kvr::compare_versions("3.2.1", "3.2.2"), Ordering::Less);
}

#[test]
fn cmp_lt_0210() {
    assert_eq!(app_lib::kvr::compare_versions("3.2.2", "3.2.3"), Ordering::Less);
}

#[test]
fn cmp_lt_0211() {
    assert_eq!(app_lib::kvr::compare_versions("3.2.3", "3.2.4"), Ordering::Less);
}

#[test]
fn cmp_lt_0212() {
    assert_eq!(app_lib::kvr::compare_versions("3.2.4", "3.2.5"), Ordering::Less);
}

#[test]
fn cmp_lt_0213() {
    assert_eq!(app_lib::kvr::compare_versions("3.2.5", "3.2.6"), Ordering::Less);
}

#[test]
fn cmp_lt_0214() {
    assert_eq!(app_lib::kvr::compare_versions("3.2.6", "3.2.7"), Ordering::Less);
}

#[test]
fn cmp_lt_0215() {
    assert_eq!(app_lib::kvr::compare_versions("3.2.7", "3.3.0"), Ordering::Less);
}

#[test]
fn cmp_lt_0216() {
    assert_eq!(app_lib::kvr::compare_versions("3.3.0", "3.3.1"), Ordering::Less);
}

#[test]
fn cmp_lt_0217() {
    assert_eq!(app_lib::kvr::compare_versions("3.3.1", "3.3.2"), Ordering::Less);
}

#[test]
fn cmp_lt_0218() {
    assert_eq!(app_lib::kvr::compare_versions("3.3.2", "3.3.3"), Ordering::Less);
}

#[test]
fn cmp_lt_0219() {
    assert_eq!(app_lib::kvr::compare_versions("3.3.3", "3.3.4"), Ordering::Less);
}

#[test]
fn cmp_lt_0220() {
    assert_eq!(app_lib::kvr::compare_versions("3.3.4", "3.3.5"), Ordering::Less);
}

#[test]
fn cmp_lt_0221() {
    assert_eq!(app_lib::kvr::compare_versions("3.3.5", "3.3.6"), Ordering::Less);
}

#[test]
fn cmp_lt_0222() {
    assert_eq!(app_lib::kvr::compare_versions("3.3.6", "3.3.7"), Ordering::Less);
}

#[test]
fn cmp_lt_0223() {
    assert_eq!(app_lib::kvr::compare_versions("3.3.7", "3.4.0"), Ordering::Less);
}

#[test]
fn cmp_lt_0224() {
    assert_eq!(app_lib::kvr::compare_versions("3.4.0", "3.4.1"), Ordering::Less);
}

#[test]
fn cmp_lt_0225() {
    assert_eq!(app_lib::kvr::compare_versions("3.4.1", "3.4.2"), Ordering::Less);
}

#[test]
fn cmp_lt_0226() {
    assert_eq!(app_lib::kvr::compare_versions("3.4.2", "3.4.3"), Ordering::Less);
}

#[test]
fn cmp_lt_0227() {
    assert_eq!(app_lib::kvr::compare_versions("3.4.3", "3.4.4"), Ordering::Less);
}

#[test]
fn cmp_lt_0228() {
    assert_eq!(app_lib::kvr::compare_versions("3.4.4", "3.4.5"), Ordering::Less);
}

#[test]
fn cmp_lt_0229() {
    assert_eq!(app_lib::kvr::compare_versions("3.4.5", "3.4.6"), Ordering::Less);
}

#[test]
fn cmp_lt_0230() {
    assert_eq!(app_lib::kvr::compare_versions("3.4.6", "3.4.7"), Ordering::Less);
}

#[test]
fn cmp_lt_0231() {
    assert_eq!(app_lib::kvr::compare_versions("3.4.7", "3.5.0"), Ordering::Less);
}

#[test]
fn cmp_lt_0232() {
    assert_eq!(app_lib::kvr::compare_versions("3.5.0", "3.5.1"), Ordering::Less);
}

#[test]
fn cmp_lt_0233() {
    assert_eq!(app_lib::kvr::compare_versions("3.5.1", "3.5.2"), Ordering::Less);
}

#[test]
fn cmp_lt_0234() {
    assert_eq!(app_lib::kvr::compare_versions("3.5.2", "3.5.3"), Ordering::Less);
}

#[test]
fn cmp_lt_0235() {
    assert_eq!(app_lib::kvr::compare_versions("3.5.3", "3.5.4"), Ordering::Less);
}

#[test]
fn cmp_lt_0236() {
    assert_eq!(app_lib::kvr::compare_versions("3.5.4", "3.5.5"), Ordering::Less);
}

#[test]
fn cmp_lt_0237() {
    assert_eq!(app_lib::kvr::compare_versions("3.5.5", "3.5.6"), Ordering::Less);
}

#[test]
fn cmp_lt_0238() {
    assert_eq!(app_lib::kvr::compare_versions("3.5.6", "3.5.7"), Ordering::Less);
}

#[test]
fn cmp_lt_0239() {
    assert_eq!(app_lib::kvr::compare_versions("3.5.7", "3.6.0"), Ordering::Less);
}

#[test]
fn cmp_lt_0240() {
    assert_eq!(app_lib::kvr::compare_versions("3.6.0", "3.6.1"), Ordering::Less);
}

#[test]
fn cmp_lt_0241() {
    assert_eq!(app_lib::kvr::compare_versions("3.6.1", "3.6.2"), Ordering::Less);
}

#[test]
fn cmp_lt_0242() {
    assert_eq!(app_lib::kvr::compare_versions("3.6.2", "3.6.3"), Ordering::Less);
}

#[test]
fn cmp_lt_0243() {
    assert_eq!(app_lib::kvr::compare_versions("3.6.3", "3.6.4"), Ordering::Less);
}

#[test]
fn cmp_lt_0244() {
    assert_eq!(app_lib::kvr::compare_versions("3.6.4", "3.6.5"), Ordering::Less);
}

#[test]
fn cmp_lt_0245() {
    assert_eq!(app_lib::kvr::compare_versions("3.6.5", "3.6.6"), Ordering::Less);
}

#[test]
fn cmp_lt_0246() {
    assert_eq!(app_lib::kvr::compare_versions("3.6.6", "3.6.7"), Ordering::Less);
}

#[test]
fn cmp_lt_0247() {
    assert_eq!(app_lib::kvr::compare_versions("3.6.7", "3.7.0"), Ordering::Less);
}

#[test]
fn cmp_lt_0248() {
    assert_eq!(app_lib::kvr::compare_versions("3.7.0", "3.7.1"), Ordering::Less);
}

#[test]
fn cmp_lt_0249() {
    assert_eq!(app_lib::kvr::compare_versions("3.7.1", "3.7.2"), Ordering::Less);
}

#[test]
fn cmp_lt_0250() {
    assert_eq!(app_lib::kvr::compare_versions("3.7.2", "3.7.3"), Ordering::Less);
}

#[test]
fn cmp_lt_0251() {
    assert_eq!(app_lib::kvr::compare_versions("3.7.3", "3.7.4"), Ordering::Less);
}

#[test]
fn cmp_lt_0252() {
    assert_eq!(app_lib::kvr::compare_versions("3.7.4", "3.7.5"), Ordering::Less);
}

#[test]
fn cmp_lt_0253() {
    assert_eq!(app_lib::kvr::compare_versions("3.7.5", "3.7.6"), Ordering::Less);
}

#[test]
fn cmp_lt_0254() {
    assert_eq!(app_lib::kvr::compare_versions("3.7.6", "3.7.7"), Ordering::Less);
}

#[test]
fn cmp_lt_0255() {
    assert_eq!(app_lib::kvr::compare_versions("3.7.7", "4.0.0"), Ordering::Less);
}

#[test]
fn cmp_lt_0256() {
    assert_eq!(app_lib::kvr::compare_versions("4.0.0", "4.0.1"), Ordering::Less);
}

#[test]
fn cmp_lt_0257() {
    assert_eq!(app_lib::kvr::compare_versions("4.0.1", "4.0.2"), Ordering::Less);
}

#[test]
fn cmp_lt_0258() {
    assert_eq!(app_lib::kvr::compare_versions("4.0.2", "4.0.3"), Ordering::Less);
}

#[test]
fn cmp_lt_0259() {
    assert_eq!(app_lib::kvr::compare_versions("4.0.3", "4.0.4"), Ordering::Less);
}

#[test]
fn cmp_lt_0260() {
    assert_eq!(app_lib::kvr::compare_versions("4.0.4", "4.0.5"), Ordering::Less);
}

#[test]
fn cmp_lt_0261() {
    assert_eq!(app_lib::kvr::compare_versions("4.0.5", "4.0.6"), Ordering::Less);
}

#[test]
fn cmp_lt_0262() {
    assert_eq!(app_lib::kvr::compare_versions("4.0.6", "4.0.7"), Ordering::Less);
}

#[test]
fn cmp_lt_0263() {
    assert_eq!(app_lib::kvr::compare_versions("4.0.7", "4.1.0"), Ordering::Less);
}

#[test]
fn cmp_lt_0264() {
    assert_eq!(app_lib::kvr::compare_versions("4.1.0", "4.1.1"), Ordering::Less);
}

#[test]
fn cmp_lt_0265() {
    assert_eq!(app_lib::kvr::compare_versions("4.1.1", "4.1.2"), Ordering::Less);
}

#[test]
fn cmp_lt_0266() {
    assert_eq!(app_lib::kvr::compare_versions("4.1.2", "4.1.3"), Ordering::Less);
}

#[test]
fn cmp_lt_0267() {
    assert_eq!(app_lib::kvr::compare_versions("4.1.3", "4.1.4"), Ordering::Less);
}

#[test]
fn cmp_lt_0268() {
    assert_eq!(app_lib::kvr::compare_versions("4.1.4", "4.1.5"), Ordering::Less);
}

#[test]
fn cmp_lt_0269() {
    assert_eq!(app_lib::kvr::compare_versions("4.1.5", "4.1.6"), Ordering::Less);
}

#[test]
fn cmp_lt_0270() {
    assert_eq!(app_lib::kvr::compare_versions("4.1.6", "4.1.7"), Ordering::Less);
}

#[test]
fn cmp_lt_0271() {
    assert_eq!(app_lib::kvr::compare_versions("4.1.7", "4.2.0"), Ordering::Less);
}

#[test]
fn cmp_lt_0272() {
    assert_eq!(app_lib::kvr::compare_versions("4.2.0", "4.2.1"), Ordering::Less);
}

#[test]
fn cmp_lt_0273() {
    assert_eq!(app_lib::kvr::compare_versions("4.2.1", "4.2.2"), Ordering::Less);
}

#[test]
fn cmp_lt_0274() {
    assert_eq!(app_lib::kvr::compare_versions("4.2.2", "4.2.3"), Ordering::Less);
}

#[test]
fn cmp_lt_0275() {
    assert_eq!(app_lib::kvr::compare_versions("4.2.3", "4.2.4"), Ordering::Less);
}

#[test]
fn cmp_lt_0276() {
    assert_eq!(app_lib::kvr::compare_versions("4.2.4", "4.2.5"), Ordering::Less);
}

#[test]
fn cmp_lt_0277() {
    assert_eq!(app_lib::kvr::compare_versions("4.2.5", "4.2.6"), Ordering::Less);
}

#[test]
fn cmp_lt_0278() {
    assert_eq!(app_lib::kvr::compare_versions("4.2.6", "4.2.7"), Ordering::Less);
}

#[test]
fn cmp_lt_0279() {
    assert_eq!(app_lib::kvr::compare_versions("4.2.7", "4.3.0"), Ordering::Less);
}

#[test]
fn cmp_lt_0280() {
    assert_eq!(app_lib::kvr::compare_versions("4.3.0", "4.3.1"), Ordering::Less);
}

#[test]
fn cmp_lt_0281() {
    assert_eq!(app_lib::kvr::compare_versions("4.3.1", "4.3.2"), Ordering::Less);
}

#[test]
fn cmp_lt_0282() {
    assert_eq!(app_lib::kvr::compare_versions("4.3.2", "4.3.3"), Ordering::Less);
}

#[test]
fn cmp_lt_0283() {
    assert_eq!(app_lib::kvr::compare_versions("4.3.3", "4.3.4"), Ordering::Less);
}

#[test]
fn cmp_lt_0284() {
    assert_eq!(app_lib::kvr::compare_versions("4.3.4", "4.3.5"), Ordering::Less);
}

#[test]
fn cmp_lt_0285() {
    assert_eq!(app_lib::kvr::compare_versions("4.3.5", "4.3.6"), Ordering::Less);
}

#[test]
fn cmp_lt_0286() {
    assert_eq!(app_lib::kvr::compare_versions("4.3.6", "4.3.7"), Ordering::Less);
}

#[test]
fn cmp_lt_0287() {
    assert_eq!(app_lib::kvr::compare_versions("4.3.7", "4.4.0"), Ordering::Less);
}

#[test]
fn cmp_lt_0288() {
    assert_eq!(app_lib::kvr::compare_versions("4.4.0", "4.4.1"), Ordering::Less);
}

#[test]
fn cmp_lt_0289() {
    assert_eq!(app_lib::kvr::compare_versions("4.4.1", "4.4.2"), Ordering::Less);
}

#[test]
fn cmp_lt_0290() {
    assert_eq!(app_lib::kvr::compare_versions("4.4.2", "4.4.3"), Ordering::Less);
}

#[test]
fn cmp_lt_0291() {
    assert_eq!(app_lib::kvr::compare_versions("4.4.3", "4.4.4"), Ordering::Less);
}

#[test]
fn cmp_lt_0292() {
    assert_eq!(app_lib::kvr::compare_versions("4.4.4", "4.4.5"), Ordering::Less);
}

#[test]
fn cmp_lt_0293() {
    assert_eq!(app_lib::kvr::compare_versions("4.4.5", "4.4.6"), Ordering::Less);
}

#[test]
fn cmp_lt_0294() {
    assert_eq!(app_lib::kvr::compare_versions("4.4.6", "4.4.7"), Ordering::Less);
}

#[test]
fn cmp_lt_0295() {
    assert_eq!(app_lib::kvr::compare_versions("4.4.7", "4.5.0"), Ordering::Less);
}

#[test]
fn cmp_lt_0296() {
    assert_eq!(app_lib::kvr::compare_versions("4.5.0", "4.5.1"), Ordering::Less);
}

#[test]
fn cmp_lt_0297() {
    assert_eq!(app_lib::kvr::compare_versions("4.5.1", "4.5.2"), Ordering::Less);
}

#[test]
fn cmp_lt_0298() {
    assert_eq!(app_lib::kvr::compare_versions("4.5.2", "4.5.3"), Ordering::Less);
}

#[test]
fn cmp_lt_0299() {
    assert_eq!(app_lib::kvr::compare_versions("4.5.3", "4.5.4"), Ordering::Less);
}

#[test]
fn cmp_lt_0300() {
    assert_eq!(app_lib::kvr::compare_versions("4.5.4", "4.5.5"), Ordering::Less);
}

#[test]
fn cmp_lt_0301() {
    assert_eq!(app_lib::kvr::compare_versions("4.5.5", "4.5.6"), Ordering::Less);
}

#[test]
fn cmp_lt_0302() {
    assert_eq!(app_lib::kvr::compare_versions("4.5.6", "4.5.7"), Ordering::Less);
}

#[test]
fn cmp_lt_0303() {
    assert_eq!(app_lib::kvr::compare_versions("4.5.7", "4.6.0"), Ordering::Less);
}

#[test]
fn cmp_lt_0304() {
    assert_eq!(app_lib::kvr::compare_versions("4.6.0", "4.6.1"), Ordering::Less);
}

#[test]
fn cmp_lt_0305() {
    assert_eq!(app_lib::kvr::compare_versions("4.6.1", "4.6.2"), Ordering::Less);
}

#[test]
fn cmp_lt_0306() {
    assert_eq!(app_lib::kvr::compare_versions("4.6.2", "4.6.3"), Ordering::Less);
}

#[test]
fn cmp_lt_0307() {
    assert_eq!(app_lib::kvr::compare_versions("4.6.3", "4.6.4"), Ordering::Less);
}

#[test]
fn cmp_lt_0308() {
    assert_eq!(app_lib::kvr::compare_versions("4.6.4", "4.6.5"), Ordering::Less);
}

#[test]
fn cmp_lt_0309() {
    assert_eq!(app_lib::kvr::compare_versions("4.6.5", "4.6.6"), Ordering::Less);
}

#[test]
fn cmp_lt_0310() {
    assert_eq!(app_lib::kvr::compare_versions("4.6.6", "4.6.7"), Ordering::Less);
}

#[test]
fn cmp_lt_0311() {
    assert_eq!(app_lib::kvr::compare_versions("4.6.7", "4.7.0"), Ordering::Less);
}

#[test]
fn cmp_lt_0312() {
    assert_eq!(app_lib::kvr::compare_versions("4.7.0", "4.7.1"), Ordering::Less);
}

#[test]
fn cmp_lt_0313() {
    assert_eq!(app_lib::kvr::compare_versions("4.7.1", "4.7.2"), Ordering::Less);
}

#[test]
fn cmp_lt_0314() {
    assert_eq!(app_lib::kvr::compare_versions("4.7.2", "4.7.3"), Ordering::Less);
}

#[test]
fn cmp_lt_0315() {
    assert_eq!(app_lib::kvr::compare_versions("4.7.3", "4.7.4"), Ordering::Less);
}

#[test]
fn cmp_lt_0316() {
    assert_eq!(app_lib::kvr::compare_versions("4.7.4", "4.7.5"), Ordering::Less);
}

#[test]
fn cmp_lt_0317() {
    assert_eq!(app_lib::kvr::compare_versions("4.7.5", "4.7.6"), Ordering::Less);
}

#[test]
fn cmp_lt_0318() {
    assert_eq!(app_lib::kvr::compare_versions("4.7.6", "4.7.7"), Ordering::Less);
}

#[test]
fn cmp_lt_0319() {
    assert_eq!(app_lib::kvr::compare_versions("4.7.7", "5.0.0"), Ordering::Less);
}

#[test]
fn cmp_lt_0320() {
    assert_eq!(app_lib::kvr::compare_versions("5.0.0", "5.0.1"), Ordering::Less);
}

#[test]
fn cmp_lt_0321() {
    assert_eq!(app_lib::kvr::compare_versions("5.0.1", "5.0.2"), Ordering::Less);
}

#[test]
fn cmp_lt_0322() {
    assert_eq!(app_lib::kvr::compare_versions("5.0.2", "5.0.3"), Ordering::Less);
}

#[test]
fn cmp_lt_0323() {
    assert_eq!(app_lib::kvr::compare_versions("5.0.3", "5.0.4"), Ordering::Less);
}

#[test]
fn cmp_lt_0324() {
    assert_eq!(app_lib::kvr::compare_versions("5.0.4", "5.0.5"), Ordering::Less);
}

#[test]
fn cmp_lt_0325() {
    assert_eq!(app_lib::kvr::compare_versions("5.0.5", "5.0.6"), Ordering::Less);
}

#[test]
fn cmp_lt_0326() {
    assert_eq!(app_lib::kvr::compare_versions("5.0.6", "5.0.7"), Ordering::Less);
}

#[test]
fn cmp_lt_0327() {
    assert_eq!(app_lib::kvr::compare_versions("5.0.7", "5.1.0"), Ordering::Less);
}

#[test]
fn cmp_lt_0328() {
    assert_eq!(app_lib::kvr::compare_versions("5.1.0", "5.1.1"), Ordering::Less);
}

#[test]
fn cmp_lt_0329() {
    assert_eq!(app_lib::kvr::compare_versions("5.1.1", "5.1.2"), Ordering::Less);
}

#[test]
fn cmp_lt_0330() {
    assert_eq!(app_lib::kvr::compare_versions("5.1.2", "5.1.3"), Ordering::Less);
}

#[test]
fn cmp_lt_0331() {
    assert_eq!(app_lib::kvr::compare_versions("5.1.3", "5.1.4"), Ordering::Less);
}

#[test]
fn cmp_lt_0332() {
    assert_eq!(app_lib::kvr::compare_versions("5.1.4", "5.1.5"), Ordering::Less);
}

#[test]
fn cmp_lt_0333() {
    assert_eq!(app_lib::kvr::compare_versions("5.1.5", "5.1.6"), Ordering::Less);
}

#[test]
fn cmp_lt_0334() {
    assert_eq!(app_lib::kvr::compare_versions("5.1.6", "5.1.7"), Ordering::Less);
}

#[test]
fn cmp_lt_0335() {
    assert_eq!(app_lib::kvr::compare_versions("5.1.7", "5.2.0"), Ordering::Less);
}

#[test]
fn cmp_lt_0336() {
    assert_eq!(app_lib::kvr::compare_versions("5.2.0", "5.2.1"), Ordering::Less);
}

#[test]
fn cmp_lt_0337() {
    assert_eq!(app_lib::kvr::compare_versions("5.2.1", "5.2.2"), Ordering::Less);
}

#[test]
fn cmp_lt_0338() {
    assert_eq!(app_lib::kvr::compare_versions("5.2.2", "5.2.3"), Ordering::Less);
}

#[test]
fn cmp_lt_0339() {
    assert_eq!(app_lib::kvr::compare_versions("5.2.3", "5.2.4"), Ordering::Less);
}

#[test]
fn cmp_lt_0340() {
    assert_eq!(app_lib::kvr::compare_versions("5.2.4", "5.2.5"), Ordering::Less);
}

#[test]
fn cmp_lt_0341() {
    assert_eq!(app_lib::kvr::compare_versions("5.2.5", "5.2.6"), Ordering::Less);
}

#[test]
fn cmp_lt_0342() {
    assert_eq!(app_lib::kvr::compare_versions("5.2.6", "5.2.7"), Ordering::Less);
}

#[test]
fn cmp_lt_0343() {
    assert_eq!(app_lib::kvr::compare_versions("5.2.7", "5.3.0"), Ordering::Less);
}

#[test]
fn cmp_lt_0344() {
    assert_eq!(app_lib::kvr::compare_versions("5.3.0", "5.3.1"), Ordering::Less);
}

#[test]
fn cmp_lt_0345() {
    assert_eq!(app_lib::kvr::compare_versions("5.3.1", "5.3.2"), Ordering::Less);
}

#[test]
fn cmp_lt_0346() {
    assert_eq!(app_lib::kvr::compare_versions("5.3.2", "5.3.3"), Ordering::Less);
}

#[test]
fn cmp_lt_0347() {
    assert_eq!(app_lib::kvr::compare_versions("5.3.3", "5.3.4"), Ordering::Less);
}

#[test]
fn cmp_lt_0348() {
    assert_eq!(app_lib::kvr::compare_versions("5.3.4", "5.3.5"), Ordering::Less);
}

#[test]
fn cmp_lt_0349() {
    assert_eq!(app_lib::kvr::compare_versions("5.3.5", "5.3.6"), Ordering::Less);
}

#[test]
fn cmp_lt_0350() {
    assert_eq!(app_lib::kvr::compare_versions("5.3.6", "5.3.7"), Ordering::Less);
}

#[test]
fn cmp_lt_0351() {
    assert_eq!(app_lib::kvr::compare_versions("5.3.7", "5.4.0"), Ordering::Less);
}

#[test]
fn cmp_lt_0352() {
    assert_eq!(app_lib::kvr::compare_versions("5.4.0", "5.4.1"), Ordering::Less);
}

#[test]
fn cmp_lt_0353() {
    assert_eq!(app_lib::kvr::compare_versions("5.4.1", "5.4.2"), Ordering::Less);
}

#[test]
fn cmp_lt_0354() {
    assert_eq!(app_lib::kvr::compare_versions("5.4.2", "5.4.3"), Ordering::Less);
}

#[test]
fn cmp_lt_0355() {
    assert_eq!(app_lib::kvr::compare_versions("5.4.3", "5.4.4"), Ordering::Less);
}

#[test]
fn cmp_lt_0356() {
    assert_eq!(app_lib::kvr::compare_versions("5.4.4", "5.4.5"), Ordering::Less);
}

#[test]
fn cmp_lt_0357() {
    assert_eq!(app_lib::kvr::compare_versions("5.4.5", "5.4.6"), Ordering::Less);
}

#[test]
fn cmp_lt_0358() {
    assert_eq!(app_lib::kvr::compare_versions("5.4.6", "5.4.7"), Ordering::Less);
}

#[test]
fn cmp_lt_0359() {
    assert_eq!(app_lib::kvr::compare_versions("5.4.7", "5.5.0"), Ordering::Less);
}

#[test]
fn cmp_lt_0360() {
    assert_eq!(app_lib::kvr::compare_versions("5.5.0", "5.5.1"), Ordering::Less);
}

#[test]
fn cmp_lt_0361() {
    assert_eq!(app_lib::kvr::compare_versions("5.5.1", "5.5.2"), Ordering::Less);
}

#[test]
fn cmp_lt_0362() {
    assert_eq!(app_lib::kvr::compare_versions("5.5.2", "5.5.3"), Ordering::Less);
}

#[test]
fn cmp_lt_0363() {
    assert_eq!(app_lib::kvr::compare_versions("5.5.3", "5.5.4"), Ordering::Less);
}

#[test]
fn cmp_lt_0364() {
    assert_eq!(app_lib::kvr::compare_versions("5.5.4", "5.5.5"), Ordering::Less);
}

#[test]
fn cmp_lt_0365() {
    assert_eq!(app_lib::kvr::compare_versions("5.5.5", "5.5.6"), Ordering::Less);
}

#[test]
fn cmp_lt_0366() {
    assert_eq!(app_lib::kvr::compare_versions("5.5.6", "5.5.7"), Ordering::Less);
}

#[test]
fn cmp_lt_0367() {
    assert_eq!(app_lib::kvr::compare_versions("5.5.7", "5.6.0"), Ordering::Less);
}

#[test]
fn cmp_lt_0368() {
    assert_eq!(app_lib::kvr::compare_versions("5.6.0", "5.6.1"), Ordering::Less);
}

#[test]
fn cmp_lt_0369() {
    assert_eq!(app_lib::kvr::compare_versions("5.6.1", "5.6.2"), Ordering::Less);
}

#[test]
fn cmp_lt_0370() {
    assert_eq!(app_lib::kvr::compare_versions("5.6.2", "5.6.3"), Ordering::Less);
}

#[test]
fn cmp_lt_0371() {
    assert_eq!(app_lib::kvr::compare_versions("5.6.3", "5.6.4"), Ordering::Less);
}

#[test]
fn cmp_lt_0372() {
    assert_eq!(app_lib::kvr::compare_versions("5.6.4", "5.6.5"), Ordering::Less);
}

#[test]
fn cmp_lt_0373() {
    assert_eq!(app_lib::kvr::compare_versions("5.6.5", "5.6.6"), Ordering::Less);
}

#[test]
fn cmp_lt_0374() {
    assert_eq!(app_lib::kvr::compare_versions("5.6.6", "5.6.7"), Ordering::Less);
}

#[test]
fn cmp_lt_0375() {
    assert_eq!(app_lib::kvr::compare_versions("5.6.7", "5.7.0"), Ordering::Less);
}

#[test]
fn cmp_lt_0376() {
    assert_eq!(app_lib::kvr::compare_versions("5.7.0", "5.7.1"), Ordering::Less);
}

#[test]
fn cmp_lt_0377() {
    assert_eq!(app_lib::kvr::compare_versions("5.7.1", "5.7.2"), Ordering::Less);
}

#[test]
fn cmp_lt_0378() {
    assert_eq!(app_lib::kvr::compare_versions("5.7.2", "5.7.3"), Ordering::Less);
}

#[test]
fn cmp_lt_0379() {
    assert_eq!(app_lib::kvr::compare_versions("5.7.3", "5.7.4"), Ordering::Less);
}

#[test]
fn cmp_lt_0380() {
    assert_eq!(app_lib::kvr::compare_versions("5.7.4", "5.7.5"), Ordering::Less);
}

#[test]
fn cmp_lt_0381() {
    assert_eq!(app_lib::kvr::compare_versions("5.7.5", "5.7.6"), Ordering::Less);
}

#[test]
fn cmp_lt_0382() {
    assert_eq!(app_lib::kvr::compare_versions("5.7.6", "5.7.7"), Ordering::Less);
}

#[test]
fn cmp_lt_0383() {
    assert_eq!(app_lib::kvr::compare_versions("5.7.7", "6.0.0"), Ordering::Less);
}

#[test]
fn cmp_lt_0384() {
    assert_eq!(app_lib::kvr::compare_versions("6.0.0", "6.0.1"), Ordering::Less);
}

#[test]
fn cmp_lt_0385() {
    assert_eq!(app_lib::kvr::compare_versions("6.0.1", "6.0.2"), Ordering::Less);
}

#[test]
fn cmp_lt_0386() {
    assert_eq!(app_lib::kvr::compare_versions("6.0.2", "6.0.3"), Ordering::Less);
}

#[test]
fn cmp_lt_0387() {
    assert_eq!(app_lib::kvr::compare_versions("6.0.3", "6.0.4"), Ordering::Less);
}

#[test]
fn cmp_lt_0388() {
    assert_eq!(app_lib::kvr::compare_versions("6.0.4", "6.0.5"), Ordering::Less);
}

#[test]
fn cmp_lt_0389() {
    assert_eq!(app_lib::kvr::compare_versions("6.0.5", "6.0.6"), Ordering::Less);
}

#[test]
fn cmp_lt_0390() {
    assert_eq!(app_lib::kvr::compare_versions("6.0.6", "6.0.7"), Ordering::Less);
}

#[test]
fn cmp_lt_0391() {
    assert_eq!(app_lib::kvr::compare_versions("6.0.7", "6.1.0"), Ordering::Less);
}

#[test]
fn cmp_lt_0392() {
    assert_eq!(app_lib::kvr::compare_versions("6.1.0", "6.1.1"), Ordering::Less);
}

#[test]
fn cmp_lt_0393() {
    assert_eq!(app_lib::kvr::compare_versions("6.1.1", "6.1.2"), Ordering::Less);
}

#[test]
fn cmp_lt_0394() {
    assert_eq!(app_lib::kvr::compare_versions("6.1.2", "6.1.3"), Ordering::Less);
}

#[test]
fn cmp_lt_0395() {
    assert_eq!(app_lib::kvr::compare_versions("6.1.3", "6.1.4"), Ordering::Less);
}

#[test]
fn cmp_lt_0396() {
    assert_eq!(app_lib::kvr::compare_versions("6.1.4", "6.1.5"), Ordering::Less);
}

#[test]
fn cmp_lt_0397() {
    assert_eq!(app_lib::kvr::compare_versions("6.1.5", "6.1.6"), Ordering::Less);
}

#[test]
fn cmp_lt_0398() {
    assert_eq!(app_lib::kvr::compare_versions("6.1.6", "6.1.7"), Ordering::Less);
}

#[test]
fn cmp_lt_0399() {
    assert_eq!(app_lib::kvr::compare_versions("6.1.7", "6.2.0"), Ordering::Less);
}

#[test]
fn cmp_lt_0400() {
    assert_eq!(app_lib::kvr::compare_versions("6.2.0", "6.2.1"), Ordering::Less);
}

#[test]
fn cmp_lt_0401() {
    assert_eq!(app_lib::kvr::compare_versions("6.2.1", "6.2.2"), Ordering::Less);
}

#[test]
fn cmp_lt_0402() {
    assert_eq!(app_lib::kvr::compare_versions("6.2.2", "6.2.3"), Ordering::Less);
}

#[test]
fn cmp_lt_0403() {
    assert_eq!(app_lib::kvr::compare_versions("6.2.3", "6.2.4"), Ordering::Less);
}

#[test]
fn cmp_lt_0404() {
    assert_eq!(app_lib::kvr::compare_versions("6.2.4", "6.2.5"), Ordering::Less);
}

#[test]
fn cmp_lt_0405() {
    assert_eq!(app_lib::kvr::compare_versions("6.2.5", "6.2.6"), Ordering::Less);
}

#[test]
fn cmp_lt_0406() {
    assert_eq!(app_lib::kvr::compare_versions("6.2.6", "6.2.7"), Ordering::Less);
}

#[test]
fn cmp_lt_0407() {
    assert_eq!(app_lib::kvr::compare_versions("6.2.7", "6.3.0"), Ordering::Less);
}

#[test]
fn cmp_lt_0408() {
    assert_eq!(app_lib::kvr::compare_versions("6.3.0", "6.3.1"), Ordering::Less);
}

#[test]
fn cmp_lt_0409() {
    assert_eq!(app_lib::kvr::compare_versions("6.3.1", "6.3.2"), Ordering::Less);
}

#[test]
fn cmp_lt_0410() {
    assert_eq!(app_lib::kvr::compare_versions("6.3.2", "6.3.3"), Ordering::Less);
}

#[test]
fn cmp_lt_0411() {
    assert_eq!(app_lib::kvr::compare_versions("6.3.3", "6.3.4"), Ordering::Less);
}

#[test]
fn cmp_lt_0412() {
    assert_eq!(app_lib::kvr::compare_versions("6.3.4", "6.3.5"), Ordering::Less);
}

#[test]
fn cmp_lt_0413() {
    assert_eq!(app_lib::kvr::compare_versions("6.3.5", "6.3.6"), Ordering::Less);
}

#[test]
fn cmp_lt_0414() {
    assert_eq!(app_lib::kvr::compare_versions("6.3.6", "6.3.7"), Ordering::Less);
}

#[test]
fn cmp_lt_0415() {
    assert_eq!(app_lib::kvr::compare_versions("6.3.7", "6.4.0"), Ordering::Less);
}

#[test]
fn cmp_lt_0416() {
    assert_eq!(app_lib::kvr::compare_versions("6.4.0", "6.4.1"), Ordering::Less);
}

#[test]
fn cmp_lt_0417() {
    assert_eq!(app_lib::kvr::compare_versions("6.4.1", "6.4.2"), Ordering::Less);
}

#[test]
fn cmp_lt_0418() {
    assert_eq!(app_lib::kvr::compare_versions("6.4.2", "6.4.3"), Ordering::Less);
}

#[test]
fn cmp_lt_0419() {
    assert_eq!(app_lib::kvr::compare_versions("6.4.3", "6.4.4"), Ordering::Less);
}

#[test]
fn cmp_lt_0420() {
    assert_eq!(app_lib::kvr::compare_versions("6.4.4", "6.4.5"), Ordering::Less);
}

#[test]
fn cmp_lt_0421() {
    assert_eq!(app_lib::kvr::compare_versions("6.4.5", "6.4.6"), Ordering::Less);
}

#[test]
fn cmp_lt_0422() {
    assert_eq!(app_lib::kvr::compare_versions("6.4.6", "6.4.7"), Ordering::Less);
}

#[test]
fn cmp_lt_0423() {
    assert_eq!(app_lib::kvr::compare_versions("6.4.7", "6.5.0"), Ordering::Less);
}

#[test]
fn cmp_lt_0424() {
    assert_eq!(app_lib::kvr::compare_versions("6.5.0", "6.5.1"), Ordering::Less);
}

#[test]
fn cmp_lt_0425() {
    assert_eq!(app_lib::kvr::compare_versions("6.5.1", "6.5.2"), Ordering::Less);
}

#[test]
fn cmp_lt_0426() {
    assert_eq!(app_lib::kvr::compare_versions("6.5.2", "6.5.3"), Ordering::Less);
}

#[test]
fn cmp_lt_0427() {
    assert_eq!(app_lib::kvr::compare_versions("6.5.3", "6.5.4"), Ordering::Less);
}

#[test]
fn cmp_lt_0428() {
    assert_eq!(app_lib::kvr::compare_versions("6.5.4", "6.5.5"), Ordering::Less);
}

#[test]
fn cmp_lt_0429() {
    assert_eq!(app_lib::kvr::compare_versions("6.5.5", "6.5.6"), Ordering::Less);
}

#[test]
fn cmp_lt_0430() {
    assert_eq!(app_lib::kvr::compare_versions("6.5.6", "6.5.7"), Ordering::Less);
}

#[test]
fn cmp_lt_0431() {
    assert_eq!(app_lib::kvr::compare_versions("6.5.7", "6.6.0"), Ordering::Less);
}

#[test]
fn cmp_lt_0432() {
    assert_eq!(app_lib::kvr::compare_versions("6.6.0", "6.6.1"), Ordering::Less);
}

#[test]
fn cmp_lt_0433() {
    assert_eq!(app_lib::kvr::compare_versions("6.6.1", "6.6.2"), Ordering::Less);
}

#[test]
fn cmp_lt_0434() {
    assert_eq!(app_lib::kvr::compare_versions("6.6.2", "6.6.3"), Ordering::Less);
}

#[test]
fn cmp_lt_0435() {
    assert_eq!(app_lib::kvr::compare_versions("6.6.3", "6.6.4"), Ordering::Less);
}

#[test]
fn cmp_lt_0436() {
    assert_eq!(app_lib::kvr::compare_versions("6.6.4", "6.6.5"), Ordering::Less);
}

#[test]
fn cmp_lt_0437() {
    assert_eq!(app_lib::kvr::compare_versions("6.6.5", "6.6.6"), Ordering::Less);
}

#[test]
fn cmp_lt_0438() {
    assert_eq!(app_lib::kvr::compare_versions("6.6.6", "6.6.7"), Ordering::Less);
}

#[test]
fn cmp_lt_0439() {
    assert_eq!(app_lib::kvr::compare_versions("6.6.7", "6.7.0"), Ordering::Less);
}

#[test]
fn cmp_lt_0440() {
    assert_eq!(app_lib::kvr::compare_versions("6.7.0", "6.7.1"), Ordering::Less);
}

#[test]
fn cmp_lt_0441() {
    assert_eq!(app_lib::kvr::compare_versions("6.7.1", "6.7.2"), Ordering::Less);
}

#[test]
fn cmp_lt_0442() {
    assert_eq!(app_lib::kvr::compare_versions("6.7.2", "6.7.3"), Ordering::Less);
}

#[test]
fn cmp_lt_0443() {
    assert_eq!(app_lib::kvr::compare_versions("6.7.3", "6.7.4"), Ordering::Less);
}

#[test]
fn cmp_lt_0444() {
    assert_eq!(app_lib::kvr::compare_versions("6.7.4", "6.7.5"), Ordering::Less);
}

#[test]
fn cmp_lt_0445() {
    assert_eq!(app_lib::kvr::compare_versions("6.7.5", "6.7.6"), Ordering::Less);
}

#[test]
fn cmp_lt_0446() {
    assert_eq!(app_lib::kvr::compare_versions("6.7.6", "6.7.7"), Ordering::Less);
}

#[test]
fn cmp_lt_0447() {
    assert_eq!(app_lib::kvr::compare_versions("6.7.7", "7.0.0"), Ordering::Less);
}

#[test]
fn cmp_lt_0448() {
    assert_eq!(app_lib::kvr::compare_versions("7.0.0", "7.0.1"), Ordering::Less);
}

#[test]
fn cmp_lt_0449() {
    assert_eq!(app_lib::kvr::compare_versions("7.0.1", "7.0.2"), Ordering::Less);
}

#[test]
fn cmp_lt_0450() {
    assert_eq!(app_lib::kvr::compare_versions("7.0.2", "7.0.3"), Ordering::Less);
}

#[test]
fn cmp_lt_0451() {
    assert_eq!(app_lib::kvr::compare_versions("7.0.3", "7.0.4"), Ordering::Less);
}

#[test]
fn cmp_lt_0452() {
    assert_eq!(app_lib::kvr::compare_versions("7.0.4", "7.0.5"), Ordering::Less);
}

#[test]
fn cmp_lt_0453() {
    assert_eq!(app_lib::kvr::compare_versions("7.0.5", "7.0.6"), Ordering::Less);
}

#[test]
fn cmp_lt_0454() {
    assert_eq!(app_lib::kvr::compare_versions("7.0.6", "7.0.7"), Ordering::Less);
}

#[test]
fn cmp_lt_0455() {
    assert_eq!(app_lib::kvr::compare_versions("7.0.7", "7.1.0"), Ordering::Less);
}

#[test]
fn cmp_lt_0456() {
    assert_eq!(app_lib::kvr::compare_versions("7.1.0", "7.1.1"), Ordering::Less);
}

#[test]
fn cmp_lt_0457() {
    assert_eq!(app_lib::kvr::compare_versions("7.1.1", "7.1.2"), Ordering::Less);
}

#[test]
fn cmp_lt_0458() {
    assert_eq!(app_lib::kvr::compare_versions("7.1.2", "7.1.3"), Ordering::Less);
}

#[test]
fn cmp_lt_0459() {
    assert_eq!(app_lib::kvr::compare_versions("7.1.3", "7.1.4"), Ordering::Less);
}

#[test]
fn cmp_lt_0460() {
    assert_eq!(app_lib::kvr::compare_versions("7.1.4", "7.1.5"), Ordering::Less);
}

#[test]
fn cmp_lt_0461() {
    assert_eq!(app_lib::kvr::compare_versions("7.1.5", "7.1.6"), Ordering::Less);
}

#[test]
fn cmp_lt_0462() {
    assert_eq!(app_lib::kvr::compare_versions("7.1.6", "7.1.7"), Ordering::Less);
}

#[test]
fn cmp_lt_0463() {
    assert_eq!(app_lib::kvr::compare_versions("7.1.7", "7.2.0"), Ordering::Less);
}

#[test]
fn cmp_lt_0464() {
    assert_eq!(app_lib::kvr::compare_versions("7.2.0", "7.2.1"), Ordering::Less);
}

#[test]
fn cmp_lt_0465() {
    assert_eq!(app_lib::kvr::compare_versions("7.2.1", "7.2.2"), Ordering::Less);
}

#[test]
fn cmp_lt_0466() {
    assert_eq!(app_lib::kvr::compare_versions("7.2.2", "7.2.3"), Ordering::Less);
}

#[test]
fn cmp_lt_0467() {
    assert_eq!(app_lib::kvr::compare_versions("7.2.3", "7.2.4"), Ordering::Less);
}

#[test]
fn cmp_lt_0468() {
    assert_eq!(app_lib::kvr::compare_versions("7.2.4", "7.2.5"), Ordering::Less);
}

#[test]
fn cmp_lt_0469() {
    assert_eq!(app_lib::kvr::compare_versions("7.2.5", "7.2.6"), Ordering::Less);
}

#[test]
fn cmp_lt_0470() {
    assert_eq!(app_lib::kvr::compare_versions("7.2.6", "7.2.7"), Ordering::Less);
}

#[test]
fn cmp_lt_0471() {
    assert_eq!(app_lib::kvr::compare_versions("7.2.7", "7.3.0"), Ordering::Less);
}

#[test]
fn cmp_lt_0472() {
    assert_eq!(app_lib::kvr::compare_versions("7.3.0", "7.3.1"), Ordering::Less);
}

#[test]
fn cmp_lt_0473() {
    assert_eq!(app_lib::kvr::compare_versions("7.3.1", "7.3.2"), Ordering::Less);
}

#[test]
fn cmp_lt_0474() {
    assert_eq!(app_lib::kvr::compare_versions("7.3.2", "7.3.3"), Ordering::Less);
}

#[test]
fn cmp_lt_0475() {
    assert_eq!(app_lib::kvr::compare_versions("7.3.3", "7.3.4"), Ordering::Less);
}

#[test]
fn cmp_lt_0476() {
    assert_eq!(app_lib::kvr::compare_versions("7.3.4", "7.3.5"), Ordering::Less);
}

#[test]
fn cmp_lt_0477() {
    assert_eq!(app_lib::kvr::compare_versions("7.3.5", "7.3.6"), Ordering::Less);
}

#[test]
fn cmp_lt_0478() {
    assert_eq!(app_lib::kvr::compare_versions("7.3.6", "7.3.7"), Ordering::Less);
}

#[test]
fn cmp_lt_0479() {
    assert_eq!(app_lib::kvr::compare_versions("7.3.7", "7.4.0"), Ordering::Less);
}

#[test]
fn cmp_lt_0480() {
    assert_eq!(app_lib::kvr::compare_versions("7.4.0", "7.4.1"), Ordering::Less);
}

#[test]
fn cmp_lt_0481() {
    assert_eq!(app_lib::kvr::compare_versions("7.4.1", "7.4.2"), Ordering::Less);
}

#[test]
fn cmp_lt_0482() {
    assert_eq!(app_lib::kvr::compare_versions("7.4.2", "7.4.3"), Ordering::Less);
}

#[test]
fn cmp_lt_0483() {
    assert_eq!(app_lib::kvr::compare_versions("7.4.3", "7.4.4"), Ordering::Less);
}

#[test]
fn cmp_lt_0484() {
    assert_eq!(app_lib::kvr::compare_versions("7.4.4", "7.4.5"), Ordering::Less);
}

#[test]
fn cmp_lt_0485() {
    assert_eq!(app_lib::kvr::compare_versions("7.4.5", "7.4.6"), Ordering::Less);
}

#[test]
fn cmp_lt_0486() {
    assert_eq!(app_lib::kvr::compare_versions("7.4.6", "7.4.7"), Ordering::Less);
}

#[test]
fn cmp_lt_0487() {
    assert_eq!(app_lib::kvr::compare_versions("7.4.7", "7.5.0"), Ordering::Less);
}

#[test]
fn cmp_lt_0488() {
    assert_eq!(app_lib::kvr::compare_versions("7.5.0", "7.5.1"), Ordering::Less);
}

#[test]
fn cmp_lt_0489() {
    assert_eq!(app_lib::kvr::compare_versions("7.5.1", "7.5.2"), Ordering::Less);
}

#[test]
fn cmp_lt_0490() {
    assert_eq!(app_lib::kvr::compare_versions("7.5.2", "7.5.3"), Ordering::Less);
}

#[test]
fn cmp_lt_0491() {
    assert_eq!(app_lib::kvr::compare_versions("7.5.3", "7.5.4"), Ordering::Less);
}

#[test]
fn cmp_lt_0492() {
    assert_eq!(app_lib::kvr::compare_versions("7.5.4", "7.5.5"), Ordering::Less);
}

#[test]
fn cmp_lt_0493() {
    assert_eq!(app_lib::kvr::compare_versions("7.5.5", "7.5.6"), Ordering::Less);
}

#[test]
fn cmp_lt_0494() {
    assert_eq!(app_lib::kvr::compare_versions("7.5.6", "7.5.7"), Ordering::Less);
}

#[test]
fn cmp_lt_0495() {
    assert_eq!(app_lib::kvr::compare_versions("7.5.7", "7.6.0"), Ordering::Less);
}

#[test]
fn cmp_lt_0496() {
    assert_eq!(app_lib::kvr::compare_versions("7.6.0", "7.6.1"), Ordering::Less);
}

#[test]
fn cmp_lt_0497() {
    assert_eq!(app_lib::kvr::compare_versions("7.6.1", "7.6.2"), Ordering::Less);
}

#[test]
fn cmp_lt_0498() {
    assert_eq!(app_lib::kvr::compare_versions("7.6.2", "7.6.3"), Ordering::Less);
}

#[test]
fn cmp_lt_0499() {
    assert_eq!(app_lib::kvr::compare_versions("7.6.3", "7.6.4"), Ordering::Less);
}

#[test]
fn cmp_lt_0500() {
    assert_eq!(app_lib::kvr::compare_versions("7.6.4", "7.6.5"), Ordering::Less);
}

#[test]
fn cmp_lt_0501() {
    assert_eq!(app_lib::kvr::compare_versions("7.6.5", "7.6.6"), Ordering::Less);
}

#[test]
fn cmp_lt_0502() {
    assert_eq!(app_lib::kvr::compare_versions("7.6.6", "7.6.7"), Ordering::Less);
}

#[test]
fn cmp_lt_0503() {
    assert_eq!(app_lib::kvr::compare_versions("7.6.7", "7.7.0"), Ordering::Less);
}

#[test]
fn cmp_lt_0504() {
    assert_eq!(app_lib::kvr::compare_versions("7.7.0", "7.7.1"), Ordering::Less);
}

#[test]
fn cmp_lt_0505() {
    assert_eq!(app_lib::kvr::compare_versions("7.7.1", "7.7.2"), Ordering::Less);
}

#[test]
fn cmp_lt_0506() {
    assert_eq!(app_lib::kvr::compare_versions("7.7.2", "7.7.3"), Ordering::Less);
}

#[test]
fn cmp_lt_0507() {
    assert_eq!(app_lib::kvr::compare_versions("7.7.3", "7.7.4"), Ordering::Less);
}

#[test]
fn cmp_lt_0508() {
    assert_eq!(app_lib::kvr::compare_versions("7.7.4", "7.7.5"), Ordering::Less);
}

#[test]
fn cmp_lt_0509() {
    assert_eq!(app_lib::kvr::compare_versions("7.7.5", "7.7.6"), Ordering::Less);
}

#[test]
fn cmp_lt_0510() {
    assert_eq!(app_lib::kvr::compare_versions("7.7.6", "7.7.7"), Ordering::Less);
}

#[test]
fn cmp_gt_0000() {
    assert_eq!(app_lib::kvr::compare_versions("0.0.1", "0.0.0"), Ordering::Greater);
}

#[test]
fn cmp_gt_0001() {
    assert_eq!(app_lib::kvr::compare_versions("0.0.2", "0.0.1"), Ordering::Greater);
}

#[test]
fn cmp_gt_0002() {
    assert_eq!(app_lib::kvr::compare_versions("0.0.3", "0.0.2"), Ordering::Greater);
}

#[test]
fn cmp_gt_0003() {
    assert_eq!(app_lib::kvr::compare_versions("0.0.4", "0.0.3"), Ordering::Greater);
}

#[test]
fn cmp_gt_0004() {
    assert_eq!(app_lib::kvr::compare_versions("0.0.5", "0.0.4"), Ordering::Greater);
}

#[test]
fn cmp_gt_0005() {
    assert_eq!(app_lib::kvr::compare_versions("0.0.6", "0.0.5"), Ordering::Greater);
}

#[test]
fn cmp_gt_0006() {
    assert_eq!(app_lib::kvr::compare_versions("0.0.7", "0.0.6"), Ordering::Greater);
}

#[test]
fn cmp_gt_0007() {
    assert_eq!(app_lib::kvr::compare_versions("0.1.0", "0.0.7"), Ordering::Greater);
}

#[test]
fn cmp_gt_0008() {
    assert_eq!(app_lib::kvr::compare_versions("0.1.1", "0.1.0"), Ordering::Greater);
}

#[test]
fn cmp_gt_0009() {
    assert_eq!(app_lib::kvr::compare_versions("0.1.2", "0.1.1"), Ordering::Greater);
}

#[test]
fn cmp_gt_0010() {
    assert_eq!(app_lib::kvr::compare_versions("0.1.3", "0.1.2"), Ordering::Greater);
}

#[test]
fn cmp_gt_0011() {
    assert_eq!(app_lib::kvr::compare_versions("0.1.4", "0.1.3"), Ordering::Greater);
}

#[test]
fn cmp_gt_0012() {
    assert_eq!(app_lib::kvr::compare_versions("0.1.5", "0.1.4"), Ordering::Greater);
}

#[test]
fn cmp_gt_0013() {
    assert_eq!(app_lib::kvr::compare_versions("0.1.6", "0.1.5"), Ordering::Greater);
}

#[test]
fn cmp_gt_0014() {
    assert_eq!(app_lib::kvr::compare_versions("0.1.7", "0.1.6"), Ordering::Greater);
}

#[test]
fn cmp_gt_0015() {
    assert_eq!(app_lib::kvr::compare_versions("0.2.0", "0.1.7"), Ordering::Greater);
}

#[test]
fn cmp_gt_0016() {
    assert_eq!(app_lib::kvr::compare_versions("0.2.1", "0.2.0"), Ordering::Greater);
}

#[test]
fn cmp_gt_0017() {
    assert_eq!(app_lib::kvr::compare_versions("0.2.2", "0.2.1"), Ordering::Greater);
}

#[test]
fn cmp_gt_0018() {
    assert_eq!(app_lib::kvr::compare_versions("0.2.3", "0.2.2"), Ordering::Greater);
}

#[test]
fn cmp_gt_0019() {
    assert_eq!(app_lib::kvr::compare_versions("0.2.4", "0.2.3"), Ordering::Greater);
}

#[test]
fn cmp_gt_0020() {
    assert_eq!(app_lib::kvr::compare_versions("0.2.5", "0.2.4"), Ordering::Greater);
}

#[test]
fn cmp_gt_0021() {
    assert_eq!(app_lib::kvr::compare_versions("0.2.6", "0.2.5"), Ordering::Greater);
}

#[test]
fn cmp_gt_0022() {
    assert_eq!(app_lib::kvr::compare_versions("0.2.7", "0.2.6"), Ordering::Greater);
}

#[test]
fn cmp_gt_0023() {
    assert_eq!(app_lib::kvr::compare_versions("0.3.0", "0.2.7"), Ordering::Greater);
}

#[test]
fn cmp_gt_0024() {
    assert_eq!(app_lib::kvr::compare_versions("0.3.1", "0.3.0"), Ordering::Greater);
}

#[test]
fn cmp_gt_0025() {
    assert_eq!(app_lib::kvr::compare_versions("0.3.2", "0.3.1"), Ordering::Greater);
}

#[test]
fn cmp_gt_0026() {
    assert_eq!(app_lib::kvr::compare_versions("0.3.3", "0.3.2"), Ordering::Greater);
}

#[test]
fn cmp_gt_0027() {
    assert_eq!(app_lib::kvr::compare_versions("0.3.4", "0.3.3"), Ordering::Greater);
}

#[test]
fn cmp_gt_0028() {
    assert_eq!(app_lib::kvr::compare_versions("0.3.5", "0.3.4"), Ordering::Greater);
}

#[test]
fn cmp_gt_0029() {
    assert_eq!(app_lib::kvr::compare_versions("0.3.6", "0.3.5"), Ordering::Greater);
}

#[test]
fn cmp_gt_0030() {
    assert_eq!(app_lib::kvr::compare_versions("0.3.7", "0.3.6"), Ordering::Greater);
}

#[test]
fn cmp_gt_0031() {
    assert_eq!(app_lib::kvr::compare_versions("0.4.0", "0.3.7"), Ordering::Greater);
}

#[test]
fn cmp_gt_0032() {
    assert_eq!(app_lib::kvr::compare_versions("0.4.1", "0.4.0"), Ordering::Greater);
}

#[test]
fn cmp_gt_0033() {
    assert_eq!(app_lib::kvr::compare_versions("0.4.2", "0.4.1"), Ordering::Greater);
}

#[test]
fn cmp_gt_0034() {
    assert_eq!(app_lib::kvr::compare_versions("0.4.3", "0.4.2"), Ordering::Greater);
}

#[test]
fn cmp_gt_0035() {
    assert_eq!(app_lib::kvr::compare_versions("0.4.4", "0.4.3"), Ordering::Greater);
}

#[test]
fn cmp_gt_0036() {
    assert_eq!(app_lib::kvr::compare_versions("0.4.5", "0.4.4"), Ordering::Greater);
}

#[test]
fn cmp_gt_0037() {
    assert_eq!(app_lib::kvr::compare_versions("0.4.6", "0.4.5"), Ordering::Greater);
}

#[test]
fn cmp_gt_0038() {
    assert_eq!(app_lib::kvr::compare_versions("0.4.7", "0.4.6"), Ordering::Greater);
}

#[test]
fn cmp_gt_0039() {
    assert_eq!(app_lib::kvr::compare_versions("0.5.0", "0.4.7"), Ordering::Greater);
}

#[test]
fn cmp_gt_0040() {
    assert_eq!(app_lib::kvr::compare_versions("0.5.1", "0.5.0"), Ordering::Greater);
}

#[test]
fn cmp_gt_0041() {
    assert_eq!(app_lib::kvr::compare_versions("0.5.2", "0.5.1"), Ordering::Greater);
}

#[test]
fn cmp_gt_0042() {
    assert_eq!(app_lib::kvr::compare_versions("0.5.3", "0.5.2"), Ordering::Greater);
}

#[test]
fn cmp_gt_0043() {
    assert_eq!(app_lib::kvr::compare_versions("0.5.4", "0.5.3"), Ordering::Greater);
}

#[test]
fn cmp_gt_0044() {
    assert_eq!(app_lib::kvr::compare_versions("0.5.5", "0.5.4"), Ordering::Greater);
}

#[test]
fn cmp_gt_0045() {
    assert_eq!(app_lib::kvr::compare_versions("0.5.6", "0.5.5"), Ordering::Greater);
}

#[test]
fn cmp_gt_0046() {
    assert_eq!(app_lib::kvr::compare_versions("0.5.7", "0.5.6"), Ordering::Greater);
}

#[test]
fn cmp_gt_0047() {
    assert_eq!(app_lib::kvr::compare_versions("0.6.0", "0.5.7"), Ordering::Greater);
}

#[test]
fn cmp_gt_0048() {
    assert_eq!(app_lib::kvr::compare_versions("0.6.1", "0.6.0"), Ordering::Greater);
}

#[test]
fn cmp_gt_0049() {
    assert_eq!(app_lib::kvr::compare_versions("0.6.2", "0.6.1"), Ordering::Greater);
}

#[test]
fn cmp_gt_0050() {
    assert_eq!(app_lib::kvr::compare_versions("0.6.3", "0.6.2"), Ordering::Greater);
}

#[test]
fn cmp_gt_0051() {
    assert_eq!(app_lib::kvr::compare_versions("0.6.4", "0.6.3"), Ordering::Greater);
}

#[test]
fn cmp_gt_0052() {
    assert_eq!(app_lib::kvr::compare_versions("0.6.5", "0.6.4"), Ordering::Greater);
}

#[test]
fn cmp_gt_0053() {
    assert_eq!(app_lib::kvr::compare_versions("0.6.6", "0.6.5"), Ordering::Greater);
}

#[test]
fn cmp_gt_0054() {
    assert_eq!(app_lib::kvr::compare_versions("0.6.7", "0.6.6"), Ordering::Greater);
}

#[test]
fn cmp_gt_0055() {
    assert_eq!(app_lib::kvr::compare_versions("0.7.0", "0.6.7"), Ordering::Greater);
}

#[test]
fn cmp_gt_0056() {
    assert_eq!(app_lib::kvr::compare_versions("0.7.1", "0.7.0"), Ordering::Greater);
}

#[test]
fn cmp_gt_0057() {
    assert_eq!(app_lib::kvr::compare_versions("0.7.2", "0.7.1"), Ordering::Greater);
}

#[test]
fn cmp_gt_0058() {
    assert_eq!(app_lib::kvr::compare_versions("0.7.3", "0.7.2"), Ordering::Greater);
}

#[test]
fn cmp_gt_0059() {
    assert_eq!(app_lib::kvr::compare_versions("0.7.4", "0.7.3"), Ordering::Greater);
}

#[test]
fn cmp_gt_0060() {
    assert_eq!(app_lib::kvr::compare_versions("0.7.5", "0.7.4"), Ordering::Greater);
}

#[test]
fn cmp_gt_0061() {
    assert_eq!(app_lib::kvr::compare_versions("0.7.6", "0.7.5"), Ordering::Greater);
}

#[test]
fn cmp_gt_0062() {
    assert_eq!(app_lib::kvr::compare_versions("0.7.7", "0.7.6"), Ordering::Greater);
}

#[test]
fn cmp_gt_0063() {
    assert_eq!(app_lib::kvr::compare_versions("1.0.0", "0.7.7"), Ordering::Greater);
}

#[test]
fn cmp_gt_0064() {
    assert_eq!(app_lib::kvr::compare_versions("1.0.1", "1.0.0"), Ordering::Greater);
}

#[test]
fn cmp_gt_0065() {
    assert_eq!(app_lib::kvr::compare_versions("1.0.2", "1.0.1"), Ordering::Greater);
}

#[test]
fn cmp_gt_0066() {
    assert_eq!(app_lib::kvr::compare_versions("1.0.3", "1.0.2"), Ordering::Greater);
}

#[test]
fn cmp_gt_0067() {
    assert_eq!(app_lib::kvr::compare_versions("1.0.4", "1.0.3"), Ordering::Greater);
}

#[test]
fn cmp_gt_0068() {
    assert_eq!(app_lib::kvr::compare_versions("1.0.5", "1.0.4"), Ordering::Greater);
}

#[test]
fn cmp_gt_0069() {
    assert_eq!(app_lib::kvr::compare_versions("1.0.6", "1.0.5"), Ordering::Greater);
}

#[test]
fn cmp_gt_0070() {
    assert_eq!(app_lib::kvr::compare_versions("1.0.7", "1.0.6"), Ordering::Greater);
}

#[test]
fn cmp_gt_0071() {
    assert_eq!(app_lib::kvr::compare_versions("1.1.0", "1.0.7"), Ordering::Greater);
}

#[test]
fn cmp_gt_0072() {
    assert_eq!(app_lib::kvr::compare_versions("1.1.1", "1.1.0"), Ordering::Greater);
}

#[test]
fn cmp_gt_0073() {
    assert_eq!(app_lib::kvr::compare_versions("1.1.2", "1.1.1"), Ordering::Greater);
}

#[test]
fn cmp_gt_0074() {
    assert_eq!(app_lib::kvr::compare_versions("1.1.3", "1.1.2"), Ordering::Greater);
}

#[test]
fn cmp_gt_0075() {
    assert_eq!(app_lib::kvr::compare_versions("1.1.4", "1.1.3"), Ordering::Greater);
}

#[test]
fn cmp_gt_0076() {
    assert_eq!(app_lib::kvr::compare_versions("1.1.5", "1.1.4"), Ordering::Greater);
}

#[test]
fn cmp_gt_0077() {
    assert_eq!(app_lib::kvr::compare_versions("1.1.6", "1.1.5"), Ordering::Greater);
}

#[test]
fn cmp_gt_0078() {
    assert_eq!(app_lib::kvr::compare_versions("1.1.7", "1.1.6"), Ordering::Greater);
}

#[test]
fn cmp_gt_0079() {
    assert_eq!(app_lib::kvr::compare_versions("1.2.0", "1.1.7"), Ordering::Greater);
}

#[test]
fn cmp_gt_0080() {
    assert_eq!(app_lib::kvr::compare_versions("1.2.1", "1.2.0"), Ordering::Greater);
}

#[test]
fn cmp_gt_0081() {
    assert_eq!(app_lib::kvr::compare_versions("1.2.2", "1.2.1"), Ordering::Greater);
}

#[test]
fn cmp_gt_0082() {
    assert_eq!(app_lib::kvr::compare_versions("1.2.3", "1.2.2"), Ordering::Greater);
}

#[test]
fn cmp_gt_0083() {
    assert_eq!(app_lib::kvr::compare_versions("1.2.4", "1.2.3"), Ordering::Greater);
}

#[test]
fn cmp_gt_0084() {
    assert_eq!(app_lib::kvr::compare_versions("1.2.5", "1.2.4"), Ordering::Greater);
}

#[test]
fn cmp_gt_0085() {
    assert_eq!(app_lib::kvr::compare_versions("1.2.6", "1.2.5"), Ordering::Greater);
}

#[test]
fn cmp_gt_0086() {
    assert_eq!(app_lib::kvr::compare_versions("1.2.7", "1.2.6"), Ordering::Greater);
}

#[test]
fn cmp_gt_0087() {
    assert_eq!(app_lib::kvr::compare_versions("1.3.0", "1.2.7"), Ordering::Greater);
}

#[test]
fn cmp_gt_0088() {
    assert_eq!(app_lib::kvr::compare_versions("1.3.1", "1.3.0"), Ordering::Greater);
}

#[test]
fn cmp_gt_0089() {
    assert_eq!(app_lib::kvr::compare_versions("1.3.2", "1.3.1"), Ordering::Greater);
}

#[test]
fn cmp_gt_0090() {
    assert_eq!(app_lib::kvr::compare_versions("1.3.3", "1.3.2"), Ordering::Greater);
}

#[test]
fn cmp_gt_0091() {
    assert_eq!(app_lib::kvr::compare_versions("1.3.4", "1.3.3"), Ordering::Greater);
}

#[test]
fn cmp_gt_0092() {
    assert_eq!(app_lib::kvr::compare_versions("1.3.5", "1.3.4"), Ordering::Greater);
}

#[test]
fn cmp_gt_0093() {
    assert_eq!(app_lib::kvr::compare_versions("1.3.6", "1.3.5"), Ordering::Greater);
}

#[test]
fn cmp_gt_0094() {
    assert_eq!(app_lib::kvr::compare_versions("1.3.7", "1.3.6"), Ordering::Greater);
}

#[test]
fn cmp_gt_0095() {
    assert_eq!(app_lib::kvr::compare_versions("1.4.0", "1.3.7"), Ordering::Greater);
}

#[test]
fn cmp_gt_0096() {
    assert_eq!(app_lib::kvr::compare_versions("1.4.1", "1.4.0"), Ordering::Greater);
}

#[test]
fn cmp_gt_0097() {
    assert_eq!(app_lib::kvr::compare_versions("1.4.2", "1.4.1"), Ordering::Greater);
}

#[test]
fn cmp_gt_0098() {
    assert_eq!(app_lib::kvr::compare_versions("1.4.3", "1.4.2"), Ordering::Greater);
}

#[test]
fn cmp_gt_0099() {
    assert_eq!(app_lib::kvr::compare_versions("1.4.4", "1.4.3"), Ordering::Greater);
}

#[test]
fn cmp_gt_0100() {
    assert_eq!(app_lib::kvr::compare_versions("1.4.5", "1.4.4"), Ordering::Greater);
}

#[test]
fn cmp_gt_0101() {
    assert_eq!(app_lib::kvr::compare_versions("1.4.6", "1.4.5"), Ordering::Greater);
}

#[test]
fn cmp_gt_0102() {
    assert_eq!(app_lib::kvr::compare_versions("1.4.7", "1.4.6"), Ordering::Greater);
}

#[test]
fn cmp_gt_0103() {
    assert_eq!(app_lib::kvr::compare_versions("1.5.0", "1.4.7"), Ordering::Greater);
}

#[test]
fn cmp_gt_0104() {
    assert_eq!(app_lib::kvr::compare_versions("1.5.1", "1.5.0"), Ordering::Greater);
}

#[test]
fn cmp_gt_0105() {
    assert_eq!(app_lib::kvr::compare_versions("1.5.2", "1.5.1"), Ordering::Greater);
}

#[test]
fn cmp_gt_0106() {
    assert_eq!(app_lib::kvr::compare_versions("1.5.3", "1.5.2"), Ordering::Greater);
}

#[test]
fn cmp_gt_0107() {
    assert_eq!(app_lib::kvr::compare_versions("1.5.4", "1.5.3"), Ordering::Greater);
}

#[test]
fn cmp_gt_0108() {
    assert_eq!(app_lib::kvr::compare_versions("1.5.5", "1.5.4"), Ordering::Greater);
}

#[test]
fn cmp_gt_0109() {
    assert_eq!(app_lib::kvr::compare_versions("1.5.6", "1.5.5"), Ordering::Greater);
}

#[test]
fn cmp_gt_0110() {
    assert_eq!(app_lib::kvr::compare_versions("1.5.7", "1.5.6"), Ordering::Greater);
}

#[test]
fn cmp_gt_0111() {
    assert_eq!(app_lib::kvr::compare_versions("1.6.0", "1.5.7"), Ordering::Greater);
}

#[test]
fn cmp_gt_0112() {
    assert_eq!(app_lib::kvr::compare_versions("1.6.1", "1.6.0"), Ordering::Greater);
}

#[test]
fn cmp_gt_0113() {
    assert_eq!(app_lib::kvr::compare_versions("1.6.2", "1.6.1"), Ordering::Greater);
}

#[test]
fn cmp_gt_0114() {
    assert_eq!(app_lib::kvr::compare_versions("1.6.3", "1.6.2"), Ordering::Greater);
}

#[test]
fn cmp_gt_0115() {
    assert_eq!(app_lib::kvr::compare_versions("1.6.4", "1.6.3"), Ordering::Greater);
}

#[test]
fn cmp_gt_0116() {
    assert_eq!(app_lib::kvr::compare_versions("1.6.5", "1.6.4"), Ordering::Greater);
}

#[test]
fn cmp_gt_0117() {
    assert_eq!(app_lib::kvr::compare_versions("1.6.6", "1.6.5"), Ordering::Greater);
}

#[test]
fn cmp_gt_0118() {
    assert_eq!(app_lib::kvr::compare_versions("1.6.7", "1.6.6"), Ordering::Greater);
}

#[test]
fn cmp_gt_0119() {
    assert_eq!(app_lib::kvr::compare_versions("1.7.0", "1.6.7"), Ordering::Greater);
}

#[test]
fn cmp_gt_0120() {
    assert_eq!(app_lib::kvr::compare_versions("1.7.1", "1.7.0"), Ordering::Greater);
}

#[test]
fn cmp_gt_0121() {
    assert_eq!(app_lib::kvr::compare_versions("1.7.2", "1.7.1"), Ordering::Greater);
}

#[test]
fn cmp_gt_0122() {
    assert_eq!(app_lib::kvr::compare_versions("1.7.3", "1.7.2"), Ordering::Greater);
}

#[test]
fn cmp_gt_0123() {
    assert_eq!(app_lib::kvr::compare_versions("1.7.4", "1.7.3"), Ordering::Greater);
}

#[test]
fn cmp_gt_0124() {
    assert_eq!(app_lib::kvr::compare_versions("1.7.5", "1.7.4"), Ordering::Greater);
}

#[test]
fn cmp_gt_0125() {
    assert_eq!(app_lib::kvr::compare_versions("1.7.6", "1.7.5"), Ordering::Greater);
}

#[test]
fn cmp_gt_0126() {
    assert_eq!(app_lib::kvr::compare_versions("1.7.7", "1.7.6"), Ordering::Greater);
}

#[test]
fn cmp_gt_0127() {
    assert_eq!(app_lib::kvr::compare_versions("2.0.0", "1.7.7"), Ordering::Greater);
}

#[test]
fn cmp_gt_0128() {
    assert_eq!(app_lib::kvr::compare_versions("2.0.1", "2.0.0"), Ordering::Greater);
}

#[test]
fn cmp_gt_0129() {
    assert_eq!(app_lib::kvr::compare_versions("2.0.2", "2.0.1"), Ordering::Greater);
}

#[test]
fn cmp_gt_0130() {
    assert_eq!(app_lib::kvr::compare_versions("2.0.3", "2.0.2"), Ordering::Greater);
}

#[test]
fn cmp_gt_0131() {
    assert_eq!(app_lib::kvr::compare_versions("2.0.4", "2.0.3"), Ordering::Greater);
}

#[test]
fn cmp_gt_0132() {
    assert_eq!(app_lib::kvr::compare_versions("2.0.5", "2.0.4"), Ordering::Greater);
}

#[test]
fn cmp_gt_0133() {
    assert_eq!(app_lib::kvr::compare_versions("2.0.6", "2.0.5"), Ordering::Greater);
}

#[test]
fn cmp_gt_0134() {
    assert_eq!(app_lib::kvr::compare_versions("2.0.7", "2.0.6"), Ordering::Greater);
}

#[test]
fn cmp_gt_0135() {
    assert_eq!(app_lib::kvr::compare_versions("2.1.0", "2.0.7"), Ordering::Greater);
}

#[test]
fn cmp_gt_0136() {
    assert_eq!(app_lib::kvr::compare_versions("2.1.1", "2.1.0"), Ordering::Greater);
}

#[test]
fn cmp_gt_0137() {
    assert_eq!(app_lib::kvr::compare_versions("2.1.2", "2.1.1"), Ordering::Greater);
}

#[test]
fn cmp_gt_0138() {
    assert_eq!(app_lib::kvr::compare_versions("2.1.3", "2.1.2"), Ordering::Greater);
}

#[test]
fn cmp_gt_0139() {
    assert_eq!(app_lib::kvr::compare_versions("2.1.4", "2.1.3"), Ordering::Greater);
}

#[test]
fn cmp_gt_0140() {
    assert_eq!(app_lib::kvr::compare_versions("2.1.5", "2.1.4"), Ordering::Greater);
}

#[test]
fn cmp_gt_0141() {
    assert_eq!(app_lib::kvr::compare_versions("2.1.6", "2.1.5"), Ordering::Greater);
}

#[test]
fn cmp_gt_0142() {
    assert_eq!(app_lib::kvr::compare_versions("2.1.7", "2.1.6"), Ordering::Greater);
}

#[test]
fn cmp_gt_0143() {
    assert_eq!(app_lib::kvr::compare_versions("2.2.0", "2.1.7"), Ordering::Greater);
}

#[test]
fn cmp_gt_0144() {
    assert_eq!(app_lib::kvr::compare_versions("2.2.1", "2.2.0"), Ordering::Greater);
}

#[test]
fn cmp_gt_0145() {
    assert_eq!(app_lib::kvr::compare_versions("2.2.2", "2.2.1"), Ordering::Greater);
}

#[test]
fn cmp_gt_0146() {
    assert_eq!(app_lib::kvr::compare_versions("2.2.3", "2.2.2"), Ordering::Greater);
}

#[test]
fn cmp_gt_0147() {
    assert_eq!(app_lib::kvr::compare_versions("2.2.4", "2.2.3"), Ordering::Greater);
}

#[test]
fn cmp_gt_0148() {
    assert_eq!(app_lib::kvr::compare_versions("2.2.5", "2.2.4"), Ordering::Greater);
}

#[test]
fn cmp_gt_0149() {
    assert_eq!(app_lib::kvr::compare_versions("2.2.6", "2.2.5"), Ordering::Greater);
}

#[test]
fn cmp_gt_0150() {
    assert_eq!(app_lib::kvr::compare_versions("2.2.7", "2.2.6"), Ordering::Greater);
}

#[test]
fn cmp_gt_0151() {
    assert_eq!(app_lib::kvr::compare_versions("2.3.0", "2.2.7"), Ordering::Greater);
}

#[test]
fn cmp_gt_0152() {
    assert_eq!(app_lib::kvr::compare_versions("2.3.1", "2.3.0"), Ordering::Greater);
}

#[test]
fn cmp_gt_0153() {
    assert_eq!(app_lib::kvr::compare_versions("2.3.2", "2.3.1"), Ordering::Greater);
}

#[test]
fn cmp_gt_0154() {
    assert_eq!(app_lib::kvr::compare_versions("2.3.3", "2.3.2"), Ordering::Greater);
}

#[test]
fn cmp_gt_0155() {
    assert_eq!(app_lib::kvr::compare_versions("2.3.4", "2.3.3"), Ordering::Greater);
}

#[test]
fn cmp_gt_0156() {
    assert_eq!(app_lib::kvr::compare_versions("2.3.5", "2.3.4"), Ordering::Greater);
}

#[test]
fn cmp_gt_0157() {
    assert_eq!(app_lib::kvr::compare_versions("2.3.6", "2.3.5"), Ordering::Greater);
}

#[test]
fn cmp_gt_0158() {
    assert_eq!(app_lib::kvr::compare_versions("2.3.7", "2.3.6"), Ordering::Greater);
}

#[test]
fn cmp_gt_0159() {
    assert_eq!(app_lib::kvr::compare_versions("2.4.0", "2.3.7"), Ordering::Greater);
}

#[test]
fn cmp_gt_0160() {
    assert_eq!(app_lib::kvr::compare_versions("2.4.1", "2.4.0"), Ordering::Greater);
}

#[test]
fn cmp_gt_0161() {
    assert_eq!(app_lib::kvr::compare_versions("2.4.2", "2.4.1"), Ordering::Greater);
}

#[test]
fn cmp_gt_0162() {
    assert_eq!(app_lib::kvr::compare_versions("2.4.3", "2.4.2"), Ordering::Greater);
}

#[test]
fn cmp_gt_0163() {
    assert_eq!(app_lib::kvr::compare_versions("2.4.4", "2.4.3"), Ordering::Greater);
}

#[test]
fn cmp_gt_0164() {
    assert_eq!(app_lib::kvr::compare_versions("2.4.5", "2.4.4"), Ordering::Greater);
}

#[test]
fn cmp_gt_0165() {
    assert_eq!(app_lib::kvr::compare_versions("2.4.6", "2.4.5"), Ordering::Greater);
}

#[test]
fn cmp_gt_0166() {
    assert_eq!(app_lib::kvr::compare_versions("2.4.7", "2.4.6"), Ordering::Greater);
}

#[test]
fn cmp_gt_0167() {
    assert_eq!(app_lib::kvr::compare_versions("2.5.0", "2.4.7"), Ordering::Greater);
}

#[test]
fn cmp_gt_0168() {
    assert_eq!(app_lib::kvr::compare_versions("2.5.1", "2.5.0"), Ordering::Greater);
}

#[test]
fn cmp_gt_0169() {
    assert_eq!(app_lib::kvr::compare_versions("2.5.2", "2.5.1"), Ordering::Greater);
}

#[test]
fn cmp_gt_0170() {
    assert_eq!(app_lib::kvr::compare_versions("2.5.3", "2.5.2"), Ordering::Greater);
}

#[test]
fn cmp_gt_0171() {
    assert_eq!(app_lib::kvr::compare_versions("2.5.4", "2.5.3"), Ordering::Greater);
}

#[test]
fn cmp_gt_0172() {
    assert_eq!(app_lib::kvr::compare_versions("2.5.5", "2.5.4"), Ordering::Greater);
}

#[test]
fn cmp_gt_0173() {
    assert_eq!(app_lib::kvr::compare_versions("2.5.6", "2.5.5"), Ordering::Greater);
}

#[test]
fn cmp_gt_0174() {
    assert_eq!(app_lib::kvr::compare_versions("2.5.7", "2.5.6"), Ordering::Greater);
}

#[test]
fn cmp_gt_0175() {
    assert_eq!(app_lib::kvr::compare_versions("2.6.0", "2.5.7"), Ordering::Greater);
}

#[test]
fn cmp_gt_0176() {
    assert_eq!(app_lib::kvr::compare_versions("2.6.1", "2.6.0"), Ordering::Greater);
}

#[test]
fn cmp_gt_0177() {
    assert_eq!(app_lib::kvr::compare_versions("2.6.2", "2.6.1"), Ordering::Greater);
}

#[test]
fn cmp_gt_0178() {
    assert_eq!(app_lib::kvr::compare_versions("2.6.3", "2.6.2"), Ordering::Greater);
}

#[test]
fn cmp_gt_0179() {
    assert_eq!(app_lib::kvr::compare_versions("2.6.4", "2.6.3"), Ordering::Greater);
}

#[test]
fn cmp_gt_0180() {
    assert_eq!(app_lib::kvr::compare_versions("2.6.5", "2.6.4"), Ordering::Greater);
}

#[test]
fn cmp_gt_0181() {
    assert_eq!(app_lib::kvr::compare_versions("2.6.6", "2.6.5"), Ordering::Greater);
}

#[test]
fn cmp_gt_0182() {
    assert_eq!(app_lib::kvr::compare_versions("2.6.7", "2.6.6"), Ordering::Greater);
}

#[test]
fn cmp_gt_0183() {
    assert_eq!(app_lib::kvr::compare_versions("2.7.0", "2.6.7"), Ordering::Greater);
}

#[test]
fn cmp_gt_0184() {
    assert_eq!(app_lib::kvr::compare_versions("2.7.1", "2.7.0"), Ordering::Greater);
}

#[test]
fn cmp_gt_0185() {
    assert_eq!(app_lib::kvr::compare_versions("2.7.2", "2.7.1"), Ordering::Greater);
}

#[test]
fn cmp_gt_0186() {
    assert_eq!(app_lib::kvr::compare_versions("2.7.3", "2.7.2"), Ordering::Greater);
}

#[test]
fn cmp_gt_0187() {
    assert_eq!(app_lib::kvr::compare_versions("2.7.4", "2.7.3"), Ordering::Greater);
}

#[test]
fn cmp_gt_0188() {
    assert_eq!(app_lib::kvr::compare_versions("2.7.5", "2.7.4"), Ordering::Greater);
}

#[test]
fn cmp_gt_0189() {
    assert_eq!(app_lib::kvr::compare_versions("2.7.6", "2.7.5"), Ordering::Greater);
}

#[test]
fn cmp_gt_0190() {
    assert_eq!(app_lib::kvr::compare_versions("2.7.7", "2.7.6"), Ordering::Greater);
}

#[test]
fn cmp_gt_0191() {
    assert_eq!(app_lib::kvr::compare_versions("3.0.0", "2.7.7"), Ordering::Greater);
}

#[test]
fn cmp_gt_0192() {
    assert_eq!(app_lib::kvr::compare_versions("3.0.1", "3.0.0"), Ordering::Greater);
}

#[test]
fn cmp_gt_0193() {
    assert_eq!(app_lib::kvr::compare_versions("3.0.2", "3.0.1"), Ordering::Greater);
}

#[test]
fn cmp_gt_0194() {
    assert_eq!(app_lib::kvr::compare_versions("3.0.3", "3.0.2"), Ordering::Greater);
}

#[test]
fn cmp_gt_0195() {
    assert_eq!(app_lib::kvr::compare_versions("3.0.4", "3.0.3"), Ordering::Greater);
}

#[test]
fn cmp_gt_0196() {
    assert_eq!(app_lib::kvr::compare_versions("3.0.5", "3.0.4"), Ordering::Greater);
}

#[test]
fn cmp_gt_0197() {
    assert_eq!(app_lib::kvr::compare_versions("3.0.6", "3.0.5"), Ordering::Greater);
}

#[test]
fn cmp_gt_0198() {
    assert_eq!(app_lib::kvr::compare_versions("3.0.7", "3.0.6"), Ordering::Greater);
}

#[test]
fn cmp_gt_0199() {
    assert_eq!(app_lib::kvr::compare_versions("3.1.0", "3.0.7"), Ordering::Greater);
}

#[test]
fn cmp_gt_0200() {
    assert_eq!(app_lib::kvr::compare_versions("3.1.1", "3.1.0"), Ordering::Greater);
}

#[test]
fn cmp_gt_0201() {
    assert_eq!(app_lib::kvr::compare_versions("3.1.2", "3.1.1"), Ordering::Greater);
}

#[test]
fn cmp_gt_0202() {
    assert_eq!(app_lib::kvr::compare_versions("3.1.3", "3.1.2"), Ordering::Greater);
}

#[test]
fn cmp_gt_0203() {
    assert_eq!(app_lib::kvr::compare_versions("3.1.4", "3.1.3"), Ordering::Greater);
}

#[test]
fn cmp_gt_0204() {
    assert_eq!(app_lib::kvr::compare_versions("3.1.5", "3.1.4"), Ordering::Greater);
}

#[test]
fn cmp_gt_0205() {
    assert_eq!(app_lib::kvr::compare_versions("3.1.6", "3.1.5"), Ordering::Greater);
}

#[test]
fn cmp_gt_0206() {
    assert_eq!(app_lib::kvr::compare_versions("3.1.7", "3.1.6"), Ordering::Greater);
}

#[test]
fn cmp_gt_0207() {
    assert_eq!(app_lib::kvr::compare_versions("3.2.0", "3.1.7"), Ordering::Greater);
}

#[test]
fn cmp_gt_0208() {
    assert_eq!(app_lib::kvr::compare_versions("3.2.1", "3.2.0"), Ordering::Greater);
}

#[test]
fn cmp_gt_0209() {
    assert_eq!(app_lib::kvr::compare_versions("3.2.2", "3.2.1"), Ordering::Greater);
}

#[test]
fn cmp_gt_0210() {
    assert_eq!(app_lib::kvr::compare_versions("3.2.3", "3.2.2"), Ordering::Greater);
}

#[test]
fn cmp_gt_0211() {
    assert_eq!(app_lib::kvr::compare_versions("3.2.4", "3.2.3"), Ordering::Greater);
}

#[test]
fn cmp_gt_0212() {
    assert_eq!(app_lib::kvr::compare_versions("3.2.5", "3.2.4"), Ordering::Greater);
}

#[test]
fn cmp_gt_0213() {
    assert_eq!(app_lib::kvr::compare_versions("3.2.6", "3.2.5"), Ordering::Greater);
}

#[test]
fn cmp_gt_0214() {
    assert_eq!(app_lib::kvr::compare_versions("3.2.7", "3.2.6"), Ordering::Greater);
}

#[test]
fn cmp_gt_0215() {
    assert_eq!(app_lib::kvr::compare_versions("3.3.0", "3.2.7"), Ordering::Greater);
}

#[test]
fn cmp_gt_0216() {
    assert_eq!(app_lib::kvr::compare_versions("3.3.1", "3.3.0"), Ordering::Greater);
}

#[test]
fn cmp_gt_0217() {
    assert_eq!(app_lib::kvr::compare_versions("3.3.2", "3.3.1"), Ordering::Greater);
}

#[test]
fn cmp_gt_0218() {
    assert_eq!(app_lib::kvr::compare_versions("3.3.3", "3.3.2"), Ordering::Greater);
}

#[test]
fn cmp_gt_0219() {
    assert_eq!(app_lib::kvr::compare_versions("3.3.4", "3.3.3"), Ordering::Greater);
}

#[test]
fn cmp_gt_0220() {
    assert_eq!(app_lib::kvr::compare_versions("3.3.5", "3.3.4"), Ordering::Greater);
}

#[test]
fn cmp_gt_0221() {
    assert_eq!(app_lib::kvr::compare_versions("3.3.6", "3.3.5"), Ordering::Greater);
}

#[test]
fn cmp_gt_0222() {
    assert_eq!(app_lib::kvr::compare_versions("3.3.7", "3.3.6"), Ordering::Greater);
}

#[test]
fn cmp_gt_0223() {
    assert_eq!(app_lib::kvr::compare_versions("3.4.0", "3.3.7"), Ordering::Greater);
}

#[test]
fn cmp_gt_0224() {
    assert_eq!(app_lib::kvr::compare_versions("3.4.1", "3.4.0"), Ordering::Greater);
}

#[test]
fn cmp_gt_0225() {
    assert_eq!(app_lib::kvr::compare_versions("3.4.2", "3.4.1"), Ordering::Greater);
}

#[test]
fn cmp_gt_0226() {
    assert_eq!(app_lib::kvr::compare_versions("3.4.3", "3.4.2"), Ordering::Greater);
}

#[test]
fn cmp_gt_0227() {
    assert_eq!(app_lib::kvr::compare_versions("3.4.4", "3.4.3"), Ordering::Greater);
}

#[test]
fn cmp_gt_0228() {
    assert_eq!(app_lib::kvr::compare_versions("3.4.5", "3.4.4"), Ordering::Greater);
}

#[test]
fn cmp_gt_0229() {
    assert_eq!(app_lib::kvr::compare_versions("3.4.6", "3.4.5"), Ordering::Greater);
}

#[test]
fn cmp_gt_0230() {
    assert_eq!(app_lib::kvr::compare_versions("3.4.7", "3.4.6"), Ordering::Greater);
}

#[test]
fn cmp_gt_0231() {
    assert_eq!(app_lib::kvr::compare_versions("3.5.0", "3.4.7"), Ordering::Greater);
}

#[test]
fn cmp_gt_0232() {
    assert_eq!(app_lib::kvr::compare_versions("3.5.1", "3.5.0"), Ordering::Greater);
}

#[test]
fn cmp_gt_0233() {
    assert_eq!(app_lib::kvr::compare_versions("3.5.2", "3.5.1"), Ordering::Greater);
}

#[test]
fn cmp_gt_0234() {
    assert_eq!(app_lib::kvr::compare_versions("3.5.3", "3.5.2"), Ordering::Greater);
}

#[test]
fn cmp_gt_0235() {
    assert_eq!(app_lib::kvr::compare_versions("3.5.4", "3.5.3"), Ordering::Greater);
}

#[test]
fn cmp_gt_0236() {
    assert_eq!(app_lib::kvr::compare_versions("3.5.5", "3.5.4"), Ordering::Greater);
}

#[test]
fn cmp_gt_0237() {
    assert_eq!(app_lib::kvr::compare_versions("3.5.6", "3.5.5"), Ordering::Greater);
}

#[test]
fn cmp_gt_0238() {
    assert_eq!(app_lib::kvr::compare_versions("3.5.7", "3.5.6"), Ordering::Greater);
}

#[test]
fn cmp_gt_0239() {
    assert_eq!(app_lib::kvr::compare_versions("3.6.0", "3.5.7"), Ordering::Greater);
}

#[test]
fn cmp_gt_0240() {
    assert_eq!(app_lib::kvr::compare_versions("3.6.1", "3.6.0"), Ordering::Greater);
}

#[test]
fn cmp_gt_0241() {
    assert_eq!(app_lib::kvr::compare_versions("3.6.2", "3.6.1"), Ordering::Greater);
}

#[test]
fn cmp_gt_0242() {
    assert_eq!(app_lib::kvr::compare_versions("3.6.3", "3.6.2"), Ordering::Greater);
}

#[test]
fn cmp_gt_0243() {
    assert_eq!(app_lib::kvr::compare_versions("3.6.4", "3.6.3"), Ordering::Greater);
}

#[test]
fn cmp_gt_0244() {
    assert_eq!(app_lib::kvr::compare_versions("3.6.5", "3.6.4"), Ordering::Greater);
}

#[test]
fn cmp_gt_0245() {
    assert_eq!(app_lib::kvr::compare_versions("3.6.6", "3.6.5"), Ordering::Greater);
}

#[test]
fn cmp_gt_0246() {
    assert_eq!(app_lib::kvr::compare_versions("3.6.7", "3.6.6"), Ordering::Greater);
}

#[test]
fn cmp_gt_0247() {
    assert_eq!(app_lib::kvr::compare_versions("3.7.0", "3.6.7"), Ordering::Greater);
}

#[test]
fn cmp_gt_0248() {
    assert_eq!(app_lib::kvr::compare_versions("3.7.1", "3.7.0"), Ordering::Greater);
}

#[test]
fn cmp_gt_0249() {
    assert_eq!(app_lib::kvr::compare_versions("3.7.2", "3.7.1"), Ordering::Greater);
}

#[test]
fn cmp_gt_0250() {
    assert_eq!(app_lib::kvr::compare_versions("3.7.3", "3.7.2"), Ordering::Greater);
}

#[test]
fn cmp_gt_0251() {
    assert_eq!(app_lib::kvr::compare_versions("3.7.4", "3.7.3"), Ordering::Greater);
}

#[test]
fn cmp_gt_0252() {
    assert_eq!(app_lib::kvr::compare_versions("3.7.5", "3.7.4"), Ordering::Greater);
}

#[test]
fn cmp_gt_0253() {
    assert_eq!(app_lib::kvr::compare_versions("3.7.6", "3.7.5"), Ordering::Greater);
}

#[test]
fn cmp_gt_0254() {
    assert_eq!(app_lib::kvr::compare_versions("3.7.7", "3.7.6"), Ordering::Greater);
}

#[test]
fn cmp_gt_0255() {
    assert_eq!(app_lib::kvr::compare_versions("4.0.0", "3.7.7"), Ordering::Greater);
}

#[test]
fn cmp_gt_0256() {
    assert_eq!(app_lib::kvr::compare_versions("4.0.1", "4.0.0"), Ordering::Greater);
}

#[test]
fn cmp_gt_0257() {
    assert_eq!(app_lib::kvr::compare_versions("4.0.2", "4.0.1"), Ordering::Greater);
}

#[test]
fn cmp_gt_0258() {
    assert_eq!(app_lib::kvr::compare_versions("4.0.3", "4.0.2"), Ordering::Greater);
}

#[test]
fn cmp_gt_0259() {
    assert_eq!(app_lib::kvr::compare_versions("4.0.4", "4.0.3"), Ordering::Greater);
}

#[test]
fn cmp_gt_0260() {
    assert_eq!(app_lib::kvr::compare_versions("4.0.5", "4.0.4"), Ordering::Greater);
}

#[test]
fn cmp_gt_0261() {
    assert_eq!(app_lib::kvr::compare_versions("4.0.6", "4.0.5"), Ordering::Greater);
}

#[test]
fn cmp_gt_0262() {
    assert_eq!(app_lib::kvr::compare_versions("4.0.7", "4.0.6"), Ordering::Greater);
}

#[test]
fn cmp_gt_0263() {
    assert_eq!(app_lib::kvr::compare_versions("4.1.0", "4.0.7"), Ordering::Greater);
}

#[test]
fn cmp_gt_0264() {
    assert_eq!(app_lib::kvr::compare_versions("4.1.1", "4.1.0"), Ordering::Greater);
}

#[test]
fn cmp_gt_0265() {
    assert_eq!(app_lib::kvr::compare_versions("4.1.2", "4.1.1"), Ordering::Greater);
}

#[test]
fn cmp_gt_0266() {
    assert_eq!(app_lib::kvr::compare_versions("4.1.3", "4.1.2"), Ordering::Greater);
}

#[test]
fn cmp_gt_0267() {
    assert_eq!(app_lib::kvr::compare_versions("4.1.4", "4.1.3"), Ordering::Greater);
}

#[test]
fn cmp_gt_0268() {
    assert_eq!(app_lib::kvr::compare_versions("4.1.5", "4.1.4"), Ordering::Greater);
}

#[test]
fn cmp_gt_0269() {
    assert_eq!(app_lib::kvr::compare_versions("4.1.6", "4.1.5"), Ordering::Greater);
}

#[test]
fn cmp_gt_0270() {
    assert_eq!(app_lib::kvr::compare_versions("4.1.7", "4.1.6"), Ordering::Greater);
}

#[test]
fn cmp_gt_0271() {
    assert_eq!(app_lib::kvr::compare_versions("4.2.0", "4.1.7"), Ordering::Greater);
}

#[test]
fn cmp_gt_0272() {
    assert_eq!(app_lib::kvr::compare_versions("4.2.1", "4.2.0"), Ordering::Greater);
}

#[test]
fn cmp_gt_0273() {
    assert_eq!(app_lib::kvr::compare_versions("4.2.2", "4.2.1"), Ordering::Greater);
}

#[test]
fn cmp_gt_0274() {
    assert_eq!(app_lib::kvr::compare_versions("4.2.3", "4.2.2"), Ordering::Greater);
}

#[test]
fn cmp_gt_0275() {
    assert_eq!(app_lib::kvr::compare_versions("4.2.4", "4.2.3"), Ordering::Greater);
}

#[test]
fn cmp_gt_0276() {
    assert_eq!(app_lib::kvr::compare_versions("4.2.5", "4.2.4"), Ordering::Greater);
}

#[test]
fn cmp_gt_0277() {
    assert_eq!(app_lib::kvr::compare_versions("4.2.6", "4.2.5"), Ordering::Greater);
}

#[test]
fn cmp_gt_0278() {
    assert_eq!(app_lib::kvr::compare_versions("4.2.7", "4.2.6"), Ordering::Greater);
}

#[test]
fn cmp_gt_0279() {
    assert_eq!(app_lib::kvr::compare_versions("4.3.0", "4.2.7"), Ordering::Greater);
}

#[test]
fn cmp_gt_0280() {
    assert_eq!(app_lib::kvr::compare_versions("4.3.1", "4.3.0"), Ordering::Greater);
}

#[test]
fn cmp_gt_0281() {
    assert_eq!(app_lib::kvr::compare_versions("4.3.2", "4.3.1"), Ordering::Greater);
}

#[test]
fn cmp_gt_0282() {
    assert_eq!(app_lib::kvr::compare_versions("4.3.3", "4.3.2"), Ordering::Greater);
}

#[test]
fn cmp_gt_0283() {
    assert_eq!(app_lib::kvr::compare_versions("4.3.4", "4.3.3"), Ordering::Greater);
}

#[test]
fn cmp_gt_0284() {
    assert_eq!(app_lib::kvr::compare_versions("4.3.5", "4.3.4"), Ordering::Greater);
}

#[test]
fn cmp_gt_0285() {
    assert_eq!(app_lib::kvr::compare_versions("4.3.6", "4.3.5"), Ordering::Greater);
}

#[test]
fn cmp_gt_0286() {
    assert_eq!(app_lib::kvr::compare_versions("4.3.7", "4.3.6"), Ordering::Greater);
}

#[test]
fn cmp_gt_0287() {
    assert_eq!(app_lib::kvr::compare_versions("4.4.0", "4.3.7"), Ordering::Greater);
}

#[test]
fn cmp_gt_0288() {
    assert_eq!(app_lib::kvr::compare_versions("4.4.1", "4.4.0"), Ordering::Greater);
}

#[test]
fn cmp_gt_0289() {
    assert_eq!(app_lib::kvr::compare_versions("4.4.2", "4.4.1"), Ordering::Greater);
}

#[test]
fn cmp_gt_0290() {
    assert_eq!(app_lib::kvr::compare_versions("4.4.3", "4.4.2"), Ordering::Greater);
}

#[test]
fn cmp_gt_0291() {
    assert_eq!(app_lib::kvr::compare_versions("4.4.4", "4.4.3"), Ordering::Greater);
}

#[test]
fn cmp_gt_0292() {
    assert_eq!(app_lib::kvr::compare_versions("4.4.5", "4.4.4"), Ordering::Greater);
}

#[test]
fn cmp_gt_0293() {
    assert_eq!(app_lib::kvr::compare_versions("4.4.6", "4.4.5"), Ordering::Greater);
}

#[test]
fn cmp_gt_0294() {
    assert_eq!(app_lib::kvr::compare_versions("4.4.7", "4.4.6"), Ordering::Greater);
}

#[test]
fn cmp_gt_0295() {
    assert_eq!(app_lib::kvr::compare_versions("4.5.0", "4.4.7"), Ordering::Greater);
}

#[test]
fn cmp_gt_0296() {
    assert_eq!(app_lib::kvr::compare_versions("4.5.1", "4.5.0"), Ordering::Greater);
}

#[test]
fn cmp_gt_0297() {
    assert_eq!(app_lib::kvr::compare_versions("4.5.2", "4.5.1"), Ordering::Greater);
}

#[test]
fn cmp_gt_0298() {
    assert_eq!(app_lib::kvr::compare_versions("4.5.3", "4.5.2"), Ordering::Greater);
}

#[test]
fn cmp_gt_0299() {
    assert_eq!(app_lib::kvr::compare_versions("4.5.4", "4.5.3"), Ordering::Greater);
}

#[test]
fn cmp_gt_0300() {
    assert_eq!(app_lib::kvr::compare_versions("4.5.5", "4.5.4"), Ordering::Greater);
}

#[test]
fn cmp_gt_0301() {
    assert_eq!(app_lib::kvr::compare_versions("4.5.6", "4.5.5"), Ordering::Greater);
}

#[test]
fn cmp_gt_0302() {
    assert_eq!(app_lib::kvr::compare_versions("4.5.7", "4.5.6"), Ordering::Greater);
}

#[test]
fn cmp_gt_0303() {
    assert_eq!(app_lib::kvr::compare_versions("4.6.0", "4.5.7"), Ordering::Greater);
}

#[test]
fn cmp_gt_0304() {
    assert_eq!(app_lib::kvr::compare_versions("4.6.1", "4.6.0"), Ordering::Greater);
}

#[test]
fn cmp_gt_0305() {
    assert_eq!(app_lib::kvr::compare_versions("4.6.2", "4.6.1"), Ordering::Greater);
}

#[test]
fn cmp_gt_0306() {
    assert_eq!(app_lib::kvr::compare_versions("4.6.3", "4.6.2"), Ordering::Greater);
}

#[test]
fn cmp_gt_0307() {
    assert_eq!(app_lib::kvr::compare_versions("4.6.4", "4.6.3"), Ordering::Greater);
}

#[test]
fn cmp_gt_0308() {
    assert_eq!(app_lib::kvr::compare_versions("4.6.5", "4.6.4"), Ordering::Greater);
}

#[test]
fn cmp_gt_0309() {
    assert_eq!(app_lib::kvr::compare_versions("4.6.6", "4.6.5"), Ordering::Greater);
}

#[test]
fn cmp_gt_0310() {
    assert_eq!(app_lib::kvr::compare_versions("4.6.7", "4.6.6"), Ordering::Greater);
}

#[test]
fn cmp_gt_0311() {
    assert_eq!(app_lib::kvr::compare_versions("4.7.0", "4.6.7"), Ordering::Greater);
}

#[test]
fn cmp_gt_0312() {
    assert_eq!(app_lib::kvr::compare_versions("4.7.1", "4.7.0"), Ordering::Greater);
}

#[test]
fn cmp_gt_0313() {
    assert_eq!(app_lib::kvr::compare_versions("4.7.2", "4.7.1"), Ordering::Greater);
}

#[test]
fn cmp_gt_0314() {
    assert_eq!(app_lib::kvr::compare_versions("4.7.3", "4.7.2"), Ordering::Greater);
}

#[test]
fn cmp_gt_0315() {
    assert_eq!(app_lib::kvr::compare_versions("4.7.4", "4.7.3"), Ordering::Greater);
}

#[test]
fn cmp_gt_0316() {
    assert_eq!(app_lib::kvr::compare_versions("4.7.5", "4.7.4"), Ordering::Greater);
}

#[test]
fn cmp_gt_0317() {
    assert_eq!(app_lib::kvr::compare_versions("4.7.6", "4.7.5"), Ordering::Greater);
}

#[test]
fn cmp_gt_0318() {
    assert_eq!(app_lib::kvr::compare_versions("4.7.7", "4.7.6"), Ordering::Greater);
}

#[test]
fn cmp_gt_0319() {
    assert_eq!(app_lib::kvr::compare_versions("5.0.0", "4.7.7"), Ordering::Greater);
}

#[test]
fn cmp_gt_0320() {
    assert_eq!(app_lib::kvr::compare_versions("5.0.1", "5.0.0"), Ordering::Greater);
}

#[test]
fn cmp_gt_0321() {
    assert_eq!(app_lib::kvr::compare_versions("5.0.2", "5.0.1"), Ordering::Greater);
}

#[test]
fn cmp_gt_0322() {
    assert_eq!(app_lib::kvr::compare_versions("5.0.3", "5.0.2"), Ordering::Greater);
}

#[test]
fn cmp_gt_0323() {
    assert_eq!(app_lib::kvr::compare_versions("5.0.4", "5.0.3"), Ordering::Greater);
}

#[test]
fn cmp_gt_0324() {
    assert_eq!(app_lib::kvr::compare_versions("5.0.5", "5.0.4"), Ordering::Greater);
}

#[test]
fn cmp_gt_0325() {
    assert_eq!(app_lib::kvr::compare_versions("5.0.6", "5.0.5"), Ordering::Greater);
}

#[test]
fn cmp_gt_0326() {
    assert_eq!(app_lib::kvr::compare_versions("5.0.7", "5.0.6"), Ordering::Greater);
}

#[test]
fn cmp_gt_0327() {
    assert_eq!(app_lib::kvr::compare_versions("5.1.0", "5.0.7"), Ordering::Greater);
}

#[test]
fn cmp_gt_0328() {
    assert_eq!(app_lib::kvr::compare_versions("5.1.1", "5.1.0"), Ordering::Greater);
}

#[test]
fn cmp_gt_0329() {
    assert_eq!(app_lib::kvr::compare_versions("5.1.2", "5.1.1"), Ordering::Greater);
}

#[test]
fn cmp_gt_0330() {
    assert_eq!(app_lib::kvr::compare_versions("5.1.3", "5.1.2"), Ordering::Greater);
}

#[test]
fn cmp_gt_0331() {
    assert_eq!(app_lib::kvr::compare_versions("5.1.4", "5.1.3"), Ordering::Greater);
}

#[test]
fn cmp_gt_0332() {
    assert_eq!(app_lib::kvr::compare_versions("5.1.5", "5.1.4"), Ordering::Greater);
}

#[test]
fn cmp_gt_0333() {
    assert_eq!(app_lib::kvr::compare_versions("5.1.6", "5.1.5"), Ordering::Greater);
}

#[test]
fn cmp_gt_0334() {
    assert_eq!(app_lib::kvr::compare_versions("5.1.7", "5.1.6"), Ordering::Greater);
}

#[test]
fn cmp_gt_0335() {
    assert_eq!(app_lib::kvr::compare_versions("5.2.0", "5.1.7"), Ordering::Greater);
}

#[test]
fn cmp_gt_0336() {
    assert_eq!(app_lib::kvr::compare_versions("5.2.1", "5.2.0"), Ordering::Greater);
}

#[test]
fn cmp_gt_0337() {
    assert_eq!(app_lib::kvr::compare_versions("5.2.2", "5.2.1"), Ordering::Greater);
}

#[test]
fn cmp_gt_0338() {
    assert_eq!(app_lib::kvr::compare_versions("5.2.3", "5.2.2"), Ordering::Greater);
}

#[test]
fn cmp_gt_0339() {
    assert_eq!(app_lib::kvr::compare_versions("5.2.4", "5.2.3"), Ordering::Greater);
}

#[test]
fn cmp_gt_0340() {
    assert_eq!(app_lib::kvr::compare_versions("5.2.5", "5.2.4"), Ordering::Greater);
}

#[test]
fn cmp_gt_0341() {
    assert_eq!(app_lib::kvr::compare_versions("5.2.6", "5.2.5"), Ordering::Greater);
}

#[test]
fn cmp_gt_0342() {
    assert_eq!(app_lib::kvr::compare_versions("5.2.7", "5.2.6"), Ordering::Greater);
}

#[test]
fn cmp_gt_0343() {
    assert_eq!(app_lib::kvr::compare_versions("5.3.0", "5.2.7"), Ordering::Greater);
}

#[test]
fn cmp_gt_0344() {
    assert_eq!(app_lib::kvr::compare_versions("5.3.1", "5.3.0"), Ordering::Greater);
}

#[test]
fn cmp_gt_0345() {
    assert_eq!(app_lib::kvr::compare_versions("5.3.2", "5.3.1"), Ordering::Greater);
}

#[test]
fn cmp_gt_0346() {
    assert_eq!(app_lib::kvr::compare_versions("5.3.3", "5.3.2"), Ordering::Greater);
}

#[test]
fn cmp_gt_0347() {
    assert_eq!(app_lib::kvr::compare_versions("5.3.4", "5.3.3"), Ordering::Greater);
}

#[test]
fn cmp_gt_0348() {
    assert_eq!(app_lib::kvr::compare_versions("5.3.5", "5.3.4"), Ordering::Greater);
}

#[test]
fn cmp_gt_0349() {
    assert_eq!(app_lib::kvr::compare_versions("5.3.6", "5.3.5"), Ordering::Greater);
}

#[test]
fn cmp_gt_0350() {
    assert_eq!(app_lib::kvr::compare_versions("5.3.7", "5.3.6"), Ordering::Greater);
}

#[test]
fn cmp_gt_0351() {
    assert_eq!(app_lib::kvr::compare_versions("5.4.0", "5.3.7"), Ordering::Greater);
}

#[test]
fn cmp_gt_0352() {
    assert_eq!(app_lib::kvr::compare_versions("5.4.1", "5.4.0"), Ordering::Greater);
}

#[test]
fn cmp_gt_0353() {
    assert_eq!(app_lib::kvr::compare_versions("5.4.2", "5.4.1"), Ordering::Greater);
}

#[test]
fn cmp_gt_0354() {
    assert_eq!(app_lib::kvr::compare_versions("5.4.3", "5.4.2"), Ordering::Greater);
}

#[test]
fn cmp_gt_0355() {
    assert_eq!(app_lib::kvr::compare_versions("5.4.4", "5.4.3"), Ordering::Greater);
}

#[test]
fn cmp_gt_0356() {
    assert_eq!(app_lib::kvr::compare_versions("5.4.5", "5.4.4"), Ordering::Greater);
}

#[test]
fn cmp_gt_0357() {
    assert_eq!(app_lib::kvr::compare_versions("5.4.6", "5.4.5"), Ordering::Greater);
}

#[test]
fn cmp_gt_0358() {
    assert_eq!(app_lib::kvr::compare_versions("5.4.7", "5.4.6"), Ordering::Greater);
}

#[test]
fn cmp_gt_0359() {
    assert_eq!(app_lib::kvr::compare_versions("5.5.0", "5.4.7"), Ordering::Greater);
}

#[test]
fn cmp_gt_0360() {
    assert_eq!(app_lib::kvr::compare_versions("5.5.1", "5.5.0"), Ordering::Greater);
}

#[test]
fn cmp_gt_0361() {
    assert_eq!(app_lib::kvr::compare_versions("5.5.2", "5.5.1"), Ordering::Greater);
}

#[test]
fn cmp_gt_0362() {
    assert_eq!(app_lib::kvr::compare_versions("5.5.3", "5.5.2"), Ordering::Greater);
}

#[test]
fn cmp_gt_0363() {
    assert_eq!(app_lib::kvr::compare_versions("5.5.4", "5.5.3"), Ordering::Greater);
}

#[test]
fn cmp_gt_0364() {
    assert_eq!(app_lib::kvr::compare_versions("5.5.5", "5.5.4"), Ordering::Greater);
}

#[test]
fn cmp_gt_0365() {
    assert_eq!(app_lib::kvr::compare_versions("5.5.6", "5.5.5"), Ordering::Greater);
}

#[test]
fn cmp_gt_0366() {
    assert_eq!(app_lib::kvr::compare_versions("5.5.7", "5.5.6"), Ordering::Greater);
}

#[test]
fn cmp_gt_0367() {
    assert_eq!(app_lib::kvr::compare_versions("5.6.0", "5.5.7"), Ordering::Greater);
}

#[test]
fn cmp_gt_0368() {
    assert_eq!(app_lib::kvr::compare_versions("5.6.1", "5.6.0"), Ordering::Greater);
}

#[test]
fn cmp_gt_0369() {
    assert_eq!(app_lib::kvr::compare_versions("5.6.2", "5.6.1"), Ordering::Greater);
}

#[test]
fn cmp_gt_0370() {
    assert_eq!(app_lib::kvr::compare_versions("5.6.3", "5.6.2"), Ordering::Greater);
}

#[test]
fn cmp_gt_0371() {
    assert_eq!(app_lib::kvr::compare_versions("5.6.4", "5.6.3"), Ordering::Greater);
}

#[test]
fn cmp_gt_0372() {
    assert_eq!(app_lib::kvr::compare_versions("5.6.5", "5.6.4"), Ordering::Greater);
}

#[test]
fn cmp_gt_0373() {
    assert_eq!(app_lib::kvr::compare_versions("5.6.6", "5.6.5"), Ordering::Greater);
}

#[test]
fn cmp_gt_0374() {
    assert_eq!(app_lib::kvr::compare_versions("5.6.7", "5.6.6"), Ordering::Greater);
}

#[test]
fn cmp_gt_0375() {
    assert_eq!(app_lib::kvr::compare_versions("5.7.0", "5.6.7"), Ordering::Greater);
}

#[test]
fn cmp_gt_0376() {
    assert_eq!(app_lib::kvr::compare_versions("5.7.1", "5.7.0"), Ordering::Greater);
}

#[test]
fn cmp_gt_0377() {
    assert_eq!(app_lib::kvr::compare_versions("5.7.2", "5.7.1"), Ordering::Greater);
}

#[test]
fn cmp_gt_0378() {
    assert_eq!(app_lib::kvr::compare_versions("5.7.3", "5.7.2"), Ordering::Greater);
}

#[test]
fn cmp_gt_0379() {
    assert_eq!(app_lib::kvr::compare_versions("5.7.4", "5.7.3"), Ordering::Greater);
}

#[test]
fn cmp_gt_0380() {
    assert_eq!(app_lib::kvr::compare_versions("5.7.5", "5.7.4"), Ordering::Greater);
}

#[test]
fn cmp_gt_0381() {
    assert_eq!(app_lib::kvr::compare_versions("5.7.6", "5.7.5"), Ordering::Greater);
}

#[test]
fn cmp_gt_0382() {
    assert_eq!(app_lib::kvr::compare_versions("5.7.7", "5.7.6"), Ordering::Greater);
}

#[test]
fn cmp_gt_0383() {
    assert_eq!(app_lib::kvr::compare_versions("6.0.0", "5.7.7"), Ordering::Greater);
}

#[test]
fn cmp_gt_0384() {
    assert_eq!(app_lib::kvr::compare_versions("6.0.1", "6.0.0"), Ordering::Greater);
}

#[test]
fn cmp_gt_0385() {
    assert_eq!(app_lib::kvr::compare_versions("6.0.2", "6.0.1"), Ordering::Greater);
}

#[test]
fn cmp_gt_0386() {
    assert_eq!(app_lib::kvr::compare_versions("6.0.3", "6.0.2"), Ordering::Greater);
}

#[test]
fn cmp_gt_0387() {
    assert_eq!(app_lib::kvr::compare_versions("6.0.4", "6.0.3"), Ordering::Greater);
}

#[test]
fn cmp_gt_0388() {
    assert_eq!(app_lib::kvr::compare_versions("6.0.5", "6.0.4"), Ordering::Greater);
}

#[test]
fn cmp_gt_0389() {
    assert_eq!(app_lib::kvr::compare_versions("6.0.6", "6.0.5"), Ordering::Greater);
}

#[test]
fn cmp_gt_0390() {
    assert_eq!(app_lib::kvr::compare_versions("6.0.7", "6.0.6"), Ordering::Greater);
}

#[test]
fn cmp_gt_0391() {
    assert_eq!(app_lib::kvr::compare_versions("6.1.0", "6.0.7"), Ordering::Greater);
}

#[test]
fn cmp_gt_0392() {
    assert_eq!(app_lib::kvr::compare_versions("6.1.1", "6.1.0"), Ordering::Greater);
}

#[test]
fn cmp_gt_0393() {
    assert_eq!(app_lib::kvr::compare_versions("6.1.2", "6.1.1"), Ordering::Greater);
}

#[test]
fn cmp_gt_0394() {
    assert_eq!(app_lib::kvr::compare_versions("6.1.3", "6.1.2"), Ordering::Greater);
}

#[test]
fn cmp_gt_0395() {
    assert_eq!(app_lib::kvr::compare_versions("6.1.4", "6.1.3"), Ordering::Greater);
}

#[test]
fn cmp_gt_0396() {
    assert_eq!(app_lib::kvr::compare_versions("6.1.5", "6.1.4"), Ordering::Greater);
}

#[test]
fn cmp_gt_0397() {
    assert_eq!(app_lib::kvr::compare_versions("6.1.6", "6.1.5"), Ordering::Greater);
}

#[test]
fn cmp_gt_0398() {
    assert_eq!(app_lib::kvr::compare_versions("6.1.7", "6.1.6"), Ordering::Greater);
}

#[test]
fn cmp_gt_0399() {
    assert_eq!(app_lib::kvr::compare_versions("6.2.0", "6.1.7"), Ordering::Greater);
}

#[test]
fn cmp_gt_0400() {
    assert_eq!(app_lib::kvr::compare_versions("6.2.1", "6.2.0"), Ordering::Greater);
}

#[test]
fn cmp_gt_0401() {
    assert_eq!(app_lib::kvr::compare_versions("6.2.2", "6.2.1"), Ordering::Greater);
}

#[test]
fn cmp_gt_0402() {
    assert_eq!(app_lib::kvr::compare_versions("6.2.3", "6.2.2"), Ordering::Greater);
}

#[test]
fn cmp_gt_0403() {
    assert_eq!(app_lib::kvr::compare_versions("6.2.4", "6.2.3"), Ordering::Greater);
}

#[test]
fn cmp_gt_0404() {
    assert_eq!(app_lib::kvr::compare_versions("6.2.5", "6.2.4"), Ordering::Greater);
}

#[test]
fn cmp_gt_0405() {
    assert_eq!(app_lib::kvr::compare_versions("6.2.6", "6.2.5"), Ordering::Greater);
}

#[test]
fn cmp_gt_0406() {
    assert_eq!(app_lib::kvr::compare_versions("6.2.7", "6.2.6"), Ordering::Greater);
}

#[test]
fn cmp_gt_0407() {
    assert_eq!(app_lib::kvr::compare_versions("6.3.0", "6.2.7"), Ordering::Greater);
}

#[test]
fn cmp_gt_0408() {
    assert_eq!(app_lib::kvr::compare_versions("6.3.1", "6.3.0"), Ordering::Greater);
}

#[test]
fn cmp_gt_0409() {
    assert_eq!(app_lib::kvr::compare_versions("6.3.2", "6.3.1"), Ordering::Greater);
}

#[test]
fn cmp_gt_0410() {
    assert_eq!(app_lib::kvr::compare_versions("6.3.3", "6.3.2"), Ordering::Greater);
}

#[test]
fn cmp_gt_0411() {
    assert_eq!(app_lib::kvr::compare_versions("6.3.4", "6.3.3"), Ordering::Greater);
}

#[test]
fn cmp_gt_0412() {
    assert_eq!(app_lib::kvr::compare_versions("6.3.5", "6.3.4"), Ordering::Greater);
}

#[test]
fn cmp_gt_0413() {
    assert_eq!(app_lib::kvr::compare_versions("6.3.6", "6.3.5"), Ordering::Greater);
}

#[test]
fn cmp_gt_0414() {
    assert_eq!(app_lib::kvr::compare_versions("6.3.7", "6.3.6"), Ordering::Greater);
}

#[test]
fn cmp_gt_0415() {
    assert_eq!(app_lib::kvr::compare_versions("6.4.0", "6.3.7"), Ordering::Greater);
}

#[test]
fn cmp_gt_0416() {
    assert_eq!(app_lib::kvr::compare_versions("6.4.1", "6.4.0"), Ordering::Greater);
}

#[test]
fn cmp_gt_0417() {
    assert_eq!(app_lib::kvr::compare_versions("6.4.2", "6.4.1"), Ordering::Greater);
}

#[test]
fn cmp_gt_0418() {
    assert_eq!(app_lib::kvr::compare_versions("6.4.3", "6.4.2"), Ordering::Greater);
}

#[test]
fn cmp_gt_0419() {
    assert_eq!(app_lib::kvr::compare_versions("6.4.4", "6.4.3"), Ordering::Greater);
}

#[test]
fn cmp_gt_0420() {
    assert_eq!(app_lib::kvr::compare_versions("6.4.5", "6.4.4"), Ordering::Greater);
}

#[test]
fn cmp_gt_0421() {
    assert_eq!(app_lib::kvr::compare_versions("6.4.6", "6.4.5"), Ordering::Greater);
}

#[test]
fn cmp_gt_0422() {
    assert_eq!(app_lib::kvr::compare_versions("6.4.7", "6.4.6"), Ordering::Greater);
}

#[test]
fn cmp_gt_0423() {
    assert_eq!(app_lib::kvr::compare_versions("6.5.0", "6.4.7"), Ordering::Greater);
}

#[test]
fn cmp_gt_0424() {
    assert_eq!(app_lib::kvr::compare_versions("6.5.1", "6.5.0"), Ordering::Greater);
}

#[test]
fn cmp_gt_0425() {
    assert_eq!(app_lib::kvr::compare_versions("6.5.2", "6.5.1"), Ordering::Greater);
}

#[test]
fn cmp_gt_0426() {
    assert_eq!(app_lib::kvr::compare_versions("6.5.3", "6.5.2"), Ordering::Greater);
}

#[test]
fn cmp_gt_0427() {
    assert_eq!(app_lib::kvr::compare_versions("6.5.4", "6.5.3"), Ordering::Greater);
}

#[test]
fn cmp_gt_0428() {
    assert_eq!(app_lib::kvr::compare_versions("6.5.5", "6.5.4"), Ordering::Greater);
}

#[test]
fn cmp_gt_0429() {
    assert_eq!(app_lib::kvr::compare_versions("6.5.6", "6.5.5"), Ordering::Greater);
}

#[test]
fn cmp_gt_0430() {
    assert_eq!(app_lib::kvr::compare_versions("6.5.7", "6.5.6"), Ordering::Greater);
}

#[test]
fn cmp_gt_0431() {
    assert_eq!(app_lib::kvr::compare_versions("6.6.0", "6.5.7"), Ordering::Greater);
}

#[test]
fn cmp_gt_0432() {
    assert_eq!(app_lib::kvr::compare_versions("6.6.1", "6.6.0"), Ordering::Greater);
}

#[test]
fn cmp_gt_0433() {
    assert_eq!(app_lib::kvr::compare_versions("6.6.2", "6.6.1"), Ordering::Greater);
}

#[test]
fn cmp_gt_0434() {
    assert_eq!(app_lib::kvr::compare_versions("6.6.3", "6.6.2"), Ordering::Greater);
}

#[test]
fn cmp_gt_0435() {
    assert_eq!(app_lib::kvr::compare_versions("6.6.4", "6.6.3"), Ordering::Greater);
}

#[test]
fn cmp_gt_0436() {
    assert_eq!(app_lib::kvr::compare_versions("6.6.5", "6.6.4"), Ordering::Greater);
}

#[test]
fn cmp_gt_0437() {
    assert_eq!(app_lib::kvr::compare_versions("6.6.6", "6.6.5"), Ordering::Greater);
}

#[test]
fn cmp_gt_0438() {
    assert_eq!(app_lib::kvr::compare_versions("6.6.7", "6.6.6"), Ordering::Greater);
}

#[test]
fn cmp_gt_0439() {
    assert_eq!(app_lib::kvr::compare_versions("6.7.0", "6.6.7"), Ordering::Greater);
}

#[test]
fn cmp_gt_0440() {
    assert_eq!(app_lib::kvr::compare_versions("6.7.1", "6.7.0"), Ordering::Greater);
}

#[test]
fn cmp_gt_0441() {
    assert_eq!(app_lib::kvr::compare_versions("6.7.2", "6.7.1"), Ordering::Greater);
}

#[test]
fn cmp_gt_0442() {
    assert_eq!(app_lib::kvr::compare_versions("6.7.3", "6.7.2"), Ordering::Greater);
}

#[test]
fn cmp_gt_0443() {
    assert_eq!(app_lib::kvr::compare_versions("6.7.4", "6.7.3"), Ordering::Greater);
}

#[test]
fn cmp_gt_0444() {
    assert_eq!(app_lib::kvr::compare_versions("6.7.5", "6.7.4"), Ordering::Greater);
}

#[test]
fn cmp_gt_0445() {
    assert_eq!(app_lib::kvr::compare_versions("6.7.6", "6.7.5"), Ordering::Greater);
}

#[test]
fn cmp_gt_0446() {
    assert_eq!(app_lib::kvr::compare_versions("6.7.7", "6.7.6"), Ordering::Greater);
}

#[test]
fn cmp_gt_0447() {
    assert_eq!(app_lib::kvr::compare_versions("7.0.0", "6.7.7"), Ordering::Greater);
}

#[test]
fn cmp_gt_0448() {
    assert_eq!(app_lib::kvr::compare_versions("7.0.1", "7.0.0"), Ordering::Greater);
}

#[test]
fn cmp_gt_0449() {
    assert_eq!(app_lib::kvr::compare_versions("7.0.2", "7.0.1"), Ordering::Greater);
}

#[test]
fn cmp_gt_0450() {
    assert_eq!(app_lib::kvr::compare_versions("7.0.3", "7.0.2"), Ordering::Greater);
}

#[test]
fn cmp_gt_0451() {
    assert_eq!(app_lib::kvr::compare_versions("7.0.4", "7.0.3"), Ordering::Greater);
}

#[test]
fn cmp_gt_0452() {
    assert_eq!(app_lib::kvr::compare_versions("7.0.5", "7.0.4"), Ordering::Greater);
}

#[test]
fn cmp_gt_0453() {
    assert_eq!(app_lib::kvr::compare_versions("7.0.6", "7.0.5"), Ordering::Greater);
}

#[test]
fn cmp_gt_0454() {
    assert_eq!(app_lib::kvr::compare_versions("7.0.7", "7.0.6"), Ordering::Greater);
}

#[test]
fn cmp_gt_0455() {
    assert_eq!(app_lib::kvr::compare_versions("7.1.0", "7.0.7"), Ordering::Greater);
}

#[test]
fn cmp_gt_0456() {
    assert_eq!(app_lib::kvr::compare_versions("7.1.1", "7.1.0"), Ordering::Greater);
}

#[test]
fn cmp_gt_0457() {
    assert_eq!(app_lib::kvr::compare_versions("7.1.2", "7.1.1"), Ordering::Greater);
}

#[test]
fn cmp_gt_0458() {
    assert_eq!(app_lib::kvr::compare_versions("7.1.3", "7.1.2"), Ordering::Greater);
}

#[test]
fn cmp_gt_0459() {
    assert_eq!(app_lib::kvr::compare_versions("7.1.4", "7.1.3"), Ordering::Greater);
}

#[test]
fn cmp_gt_0460() {
    assert_eq!(app_lib::kvr::compare_versions("7.1.5", "7.1.4"), Ordering::Greater);
}

#[test]
fn cmp_gt_0461() {
    assert_eq!(app_lib::kvr::compare_versions("7.1.6", "7.1.5"), Ordering::Greater);
}

#[test]
fn cmp_gt_0462() {
    assert_eq!(app_lib::kvr::compare_versions("7.1.7", "7.1.6"), Ordering::Greater);
}

#[test]
fn cmp_gt_0463() {
    assert_eq!(app_lib::kvr::compare_versions("7.2.0", "7.1.7"), Ordering::Greater);
}

#[test]
fn cmp_gt_0464() {
    assert_eq!(app_lib::kvr::compare_versions("7.2.1", "7.2.0"), Ordering::Greater);
}

#[test]
fn cmp_gt_0465() {
    assert_eq!(app_lib::kvr::compare_versions("7.2.2", "7.2.1"), Ordering::Greater);
}

#[test]
fn cmp_gt_0466() {
    assert_eq!(app_lib::kvr::compare_versions("7.2.3", "7.2.2"), Ordering::Greater);
}

#[test]
fn cmp_gt_0467() {
    assert_eq!(app_lib::kvr::compare_versions("7.2.4", "7.2.3"), Ordering::Greater);
}

#[test]
fn cmp_gt_0468() {
    assert_eq!(app_lib::kvr::compare_versions("7.2.5", "7.2.4"), Ordering::Greater);
}

#[test]
fn cmp_gt_0469() {
    assert_eq!(app_lib::kvr::compare_versions("7.2.6", "7.2.5"), Ordering::Greater);
}

#[test]
fn cmp_gt_0470() {
    assert_eq!(app_lib::kvr::compare_versions("7.2.7", "7.2.6"), Ordering::Greater);
}

#[test]
fn cmp_gt_0471() {
    assert_eq!(app_lib::kvr::compare_versions("7.3.0", "7.2.7"), Ordering::Greater);
}

#[test]
fn cmp_gt_0472() {
    assert_eq!(app_lib::kvr::compare_versions("7.3.1", "7.3.0"), Ordering::Greater);
}

#[test]
fn cmp_gt_0473() {
    assert_eq!(app_lib::kvr::compare_versions("7.3.2", "7.3.1"), Ordering::Greater);
}

#[test]
fn cmp_gt_0474() {
    assert_eq!(app_lib::kvr::compare_versions("7.3.3", "7.3.2"), Ordering::Greater);
}

#[test]
fn cmp_gt_0475() {
    assert_eq!(app_lib::kvr::compare_versions("7.3.4", "7.3.3"), Ordering::Greater);
}

#[test]
fn cmp_gt_0476() {
    assert_eq!(app_lib::kvr::compare_versions("7.3.5", "7.3.4"), Ordering::Greater);
}

#[test]
fn cmp_gt_0477() {
    assert_eq!(app_lib::kvr::compare_versions("7.3.6", "7.3.5"), Ordering::Greater);
}

#[test]
fn cmp_gt_0478() {
    assert_eq!(app_lib::kvr::compare_versions("7.3.7", "7.3.6"), Ordering::Greater);
}

#[test]
fn cmp_gt_0479() {
    assert_eq!(app_lib::kvr::compare_versions("7.4.0", "7.3.7"), Ordering::Greater);
}

#[test]
fn cmp_gt_0480() {
    assert_eq!(app_lib::kvr::compare_versions("7.4.1", "7.4.0"), Ordering::Greater);
}

#[test]
fn cmp_gt_0481() {
    assert_eq!(app_lib::kvr::compare_versions("7.4.2", "7.4.1"), Ordering::Greater);
}

#[test]
fn cmp_gt_0482() {
    assert_eq!(app_lib::kvr::compare_versions("7.4.3", "7.4.2"), Ordering::Greater);
}

#[test]
fn cmp_gt_0483() {
    assert_eq!(app_lib::kvr::compare_versions("7.4.4", "7.4.3"), Ordering::Greater);
}

#[test]
fn cmp_gt_0484() {
    assert_eq!(app_lib::kvr::compare_versions("7.4.5", "7.4.4"), Ordering::Greater);
}

#[test]
fn cmp_gt_0485() {
    assert_eq!(app_lib::kvr::compare_versions("7.4.6", "7.4.5"), Ordering::Greater);
}

#[test]
fn cmp_gt_0486() {
    assert_eq!(app_lib::kvr::compare_versions("7.4.7", "7.4.6"), Ordering::Greater);
}

#[test]
fn cmp_gt_0487() {
    assert_eq!(app_lib::kvr::compare_versions("7.5.0", "7.4.7"), Ordering::Greater);
}

#[test]
fn cmp_gt_0488() {
    assert_eq!(app_lib::kvr::compare_versions("7.5.1", "7.5.0"), Ordering::Greater);
}

#[test]
fn cmp_gt_0489() {
    assert_eq!(app_lib::kvr::compare_versions("7.5.2", "7.5.1"), Ordering::Greater);
}

#[test]
fn cmp_gt_0490() {
    assert_eq!(app_lib::kvr::compare_versions("7.5.3", "7.5.2"), Ordering::Greater);
}

#[test]
fn cmp_gt_0491() {
    assert_eq!(app_lib::kvr::compare_versions("7.5.4", "7.5.3"), Ordering::Greater);
}

#[test]
fn cmp_gt_0492() {
    assert_eq!(app_lib::kvr::compare_versions("7.5.5", "7.5.4"), Ordering::Greater);
}

#[test]
fn cmp_gt_0493() {
    assert_eq!(app_lib::kvr::compare_versions("7.5.6", "7.5.5"), Ordering::Greater);
}

#[test]
fn cmp_gt_0494() {
    assert_eq!(app_lib::kvr::compare_versions("7.5.7", "7.5.6"), Ordering::Greater);
}

#[test]
fn cmp_gt_0495() {
    assert_eq!(app_lib::kvr::compare_versions("7.6.0", "7.5.7"), Ordering::Greater);
}

#[test]
fn cmp_gt_0496() {
    assert_eq!(app_lib::kvr::compare_versions("7.6.1", "7.6.0"), Ordering::Greater);
}

#[test]
fn cmp_gt_0497() {
    assert_eq!(app_lib::kvr::compare_versions("7.6.2", "7.6.1"), Ordering::Greater);
}

#[test]
fn cmp_gt_0498() {
    assert_eq!(app_lib::kvr::compare_versions("7.6.3", "7.6.2"), Ordering::Greater);
}

#[test]
fn cmp_gt_0499() {
    assert_eq!(app_lib::kvr::compare_versions("7.6.4", "7.6.3"), Ordering::Greater);
}

#[test]
fn cmp_gt_0500() {
    assert_eq!(app_lib::kvr::compare_versions("7.6.5", "7.6.4"), Ordering::Greater);
}

#[test]
fn cmp_gt_0501() {
    assert_eq!(app_lib::kvr::compare_versions("7.6.6", "7.6.5"), Ordering::Greater);
}

#[test]
fn cmp_gt_0502() {
    assert_eq!(app_lib::kvr::compare_versions("7.6.7", "7.6.6"), Ordering::Greater);
}

#[test]
fn cmp_gt_0503() {
    assert_eq!(app_lib::kvr::compare_versions("7.7.0", "7.6.7"), Ordering::Greater);
}

#[test]
fn cmp_gt_0504() {
    assert_eq!(app_lib::kvr::compare_versions("7.7.1", "7.7.0"), Ordering::Greater);
}

#[test]
fn cmp_gt_0505() {
    assert_eq!(app_lib::kvr::compare_versions("7.7.2", "7.7.1"), Ordering::Greater);
}

#[test]
fn cmp_gt_0506() {
    assert_eq!(app_lib::kvr::compare_versions("7.7.3", "7.7.2"), Ordering::Greater);
}

#[test]
fn cmp_gt_0507() {
    assert_eq!(app_lib::kvr::compare_versions("7.7.4", "7.7.3"), Ordering::Greater);
}

#[test]
fn cmp_gt_0508() {
    assert_eq!(app_lib::kvr::compare_versions("7.7.5", "7.7.4"), Ordering::Greater);
}

#[test]
fn cmp_gt_0509() {
    assert_eq!(app_lib::kvr::compare_versions("7.7.6", "7.7.5"), Ordering::Greater);
}

#[test]
fn cmp_gt_0510() {
    assert_eq!(app_lib::kvr::compare_versions("7.7.7", "7.7.6"), Ordering::Greater);
}

#[test]
fn cmp_eq_0000() {
    assert_eq!(app_lib::kvr::compare_versions("0.0.0", "0.0.0"), Ordering::Equal);
}

#[test]
fn cmp_eq_0001() {
    assert_eq!(app_lib::kvr::compare_versions("0.0.1", "0.0.1"), Ordering::Equal);
}

#[test]
fn cmp_eq_0002() {
    assert_eq!(app_lib::kvr::compare_versions("0.0.2", "0.0.2"), Ordering::Equal);
}

#[test]
fn cmp_eq_0003() {
    assert_eq!(app_lib::kvr::compare_versions("0.0.3", "0.0.3"), Ordering::Equal);
}

#[test]
fn cmp_eq_0004() {
    assert_eq!(app_lib::kvr::compare_versions("0.0.4", "0.0.4"), Ordering::Equal);
}

#[test]
fn cmp_eq_0005() {
    assert_eq!(app_lib::kvr::compare_versions("0.0.5", "0.0.5"), Ordering::Equal);
}

#[test]
fn cmp_eq_0006() {
    assert_eq!(app_lib::kvr::compare_versions("0.0.6", "0.0.6"), Ordering::Equal);
}

#[test]
fn cmp_eq_0007() {
    assert_eq!(app_lib::kvr::compare_versions("0.0.7", "0.0.7"), Ordering::Equal);
}

#[test]
fn cmp_eq_0008() {
    assert_eq!(app_lib::kvr::compare_versions("0.1.0", "0.1.0"), Ordering::Equal);
}

#[test]
fn cmp_eq_0009() {
    assert_eq!(app_lib::kvr::compare_versions("0.1.1", "0.1.1"), Ordering::Equal);
}

#[test]
fn cmp_eq_0010() {
    assert_eq!(app_lib::kvr::compare_versions("0.1.2", "0.1.2"), Ordering::Equal);
}

#[test]
fn cmp_eq_0011() {
    assert_eq!(app_lib::kvr::compare_versions("0.1.3", "0.1.3"), Ordering::Equal);
}

#[test]
fn cmp_eq_0012() {
    assert_eq!(app_lib::kvr::compare_versions("0.1.4", "0.1.4"), Ordering::Equal);
}

#[test]
fn cmp_eq_0013() {
    assert_eq!(app_lib::kvr::compare_versions("0.1.5", "0.1.5"), Ordering::Equal);
}

#[test]
fn cmp_eq_0014() {
    assert_eq!(app_lib::kvr::compare_versions("0.1.6", "0.1.6"), Ordering::Equal);
}

#[test]
fn cmp_eq_0015() {
    assert_eq!(app_lib::kvr::compare_versions("0.1.7", "0.1.7"), Ordering::Equal);
}

#[test]
fn cmp_eq_0016() {
    assert_eq!(app_lib::kvr::compare_versions("0.2.0", "0.2.0"), Ordering::Equal);
}

#[test]
fn cmp_eq_0017() {
    assert_eq!(app_lib::kvr::compare_versions("0.2.1", "0.2.1"), Ordering::Equal);
}

#[test]
fn cmp_eq_0018() {
    assert_eq!(app_lib::kvr::compare_versions("0.2.2", "0.2.2"), Ordering::Equal);
}

#[test]
fn cmp_eq_0019() {
    assert_eq!(app_lib::kvr::compare_versions("0.2.3", "0.2.3"), Ordering::Equal);
}

#[test]
fn cmp_eq_0020() {
    assert_eq!(app_lib::kvr::compare_versions("0.2.4", "0.2.4"), Ordering::Equal);
}

#[test]
fn cmp_eq_0021() {
    assert_eq!(app_lib::kvr::compare_versions("0.2.5", "0.2.5"), Ordering::Equal);
}

#[test]
fn cmp_eq_0022() {
    assert_eq!(app_lib::kvr::compare_versions("0.2.6", "0.2.6"), Ordering::Equal);
}

#[test]
fn cmp_eq_0023() {
    assert_eq!(app_lib::kvr::compare_versions("0.2.7", "0.2.7"), Ordering::Equal);
}

#[test]
fn cmp_eq_0024() {
    assert_eq!(app_lib::kvr::compare_versions("0.3.0", "0.3.0"), Ordering::Equal);
}

#[test]
fn cmp_eq_0025() {
    assert_eq!(app_lib::kvr::compare_versions("0.3.1", "0.3.1"), Ordering::Equal);
}

#[test]
fn cmp_eq_0026() {
    assert_eq!(app_lib::kvr::compare_versions("0.3.2", "0.3.2"), Ordering::Equal);
}

#[test]
fn cmp_eq_0027() {
    assert_eq!(app_lib::kvr::compare_versions("0.3.3", "0.3.3"), Ordering::Equal);
}

#[test]
fn cmp_eq_0028() {
    assert_eq!(app_lib::kvr::compare_versions("0.3.4", "0.3.4"), Ordering::Equal);
}

#[test]
fn cmp_eq_0029() {
    assert_eq!(app_lib::kvr::compare_versions("0.3.5", "0.3.5"), Ordering::Equal);
}

#[test]
fn cmp_eq_0030() {
    assert_eq!(app_lib::kvr::compare_versions("0.3.6", "0.3.6"), Ordering::Equal);
}

#[test]
fn cmp_eq_0031() {
    assert_eq!(app_lib::kvr::compare_versions("0.3.7", "0.3.7"), Ordering::Equal);
}

#[test]
fn cmp_eq_0032() {
    assert_eq!(app_lib::kvr::compare_versions("0.4.0", "0.4.0"), Ordering::Equal);
}

#[test]
fn cmp_eq_0033() {
    assert_eq!(app_lib::kvr::compare_versions("0.4.1", "0.4.1"), Ordering::Equal);
}

#[test]
fn cmp_eq_0034() {
    assert_eq!(app_lib::kvr::compare_versions("0.4.2", "0.4.2"), Ordering::Equal);
}

#[test]
fn cmp_eq_0035() {
    assert_eq!(app_lib::kvr::compare_versions("0.4.3", "0.4.3"), Ordering::Equal);
}

#[test]
fn cmp_eq_0036() {
    assert_eq!(app_lib::kvr::compare_versions("0.4.4", "0.4.4"), Ordering::Equal);
}

#[test]
fn cmp_eq_0037() {
    assert_eq!(app_lib::kvr::compare_versions("0.4.5", "0.4.5"), Ordering::Equal);
}

#[test]
fn cmp_eq_0038() {
    assert_eq!(app_lib::kvr::compare_versions("0.4.6", "0.4.6"), Ordering::Equal);
}

#[test]
fn cmp_eq_0039() {
    assert_eq!(app_lib::kvr::compare_versions("0.4.7", "0.4.7"), Ordering::Equal);
}

#[test]
fn cmp_eq_0040() {
    assert_eq!(app_lib::kvr::compare_versions("0.5.0", "0.5.0"), Ordering::Equal);
}

#[test]
fn cmp_eq_0041() {
    assert_eq!(app_lib::kvr::compare_versions("0.5.1", "0.5.1"), Ordering::Equal);
}

#[test]
fn cmp_eq_0042() {
    assert_eq!(app_lib::kvr::compare_versions("0.5.2", "0.5.2"), Ordering::Equal);
}

#[test]
fn cmp_eq_0043() {
    assert_eq!(app_lib::kvr::compare_versions("0.5.3", "0.5.3"), Ordering::Equal);
}

#[test]
fn cmp_eq_0044() {
    assert_eq!(app_lib::kvr::compare_versions("0.5.4", "0.5.4"), Ordering::Equal);
}

#[test]
fn cmp_eq_0045() {
    assert_eq!(app_lib::kvr::compare_versions("0.5.5", "0.5.5"), Ordering::Equal);
}

#[test]
fn cmp_eq_0046() {
    assert_eq!(app_lib::kvr::compare_versions("0.5.6", "0.5.6"), Ordering::Equal);
}

#[test]
fn cmp_eq_0047() {
    assert_eq!(app_lib::kvr::compare_versions("0.5.7", "0.5.7"), Ordering::Equal);
}

#[test]
fn cmp_eq_0048() {
    assert_eq!(app_lib::kvr::compare_versions("0.6.0", "0.6.0"), Ordering::Equal);
}

#[test]
fn cmp_eq_0049() {
    assert_eq!(app_lib::kvr::compare_versions("0.6.1", "0.6.1"), Ordering::Equal);
}

#[test]
fn cmp_eq_0050() {
    assert_eq!(app_lib::kvr::compare_versions("0.6.2", "0.6.2"), Ordering::Equal);
}

#[test]
fn cmp_eq_0051() {
    assert_eq!(app_lib::kvr::compare_versions("0.6.3", "0.6.3"), Ordering::Equal);
}

#[test]
fn cmp_eq_0052() {
    assert_eq!(app_lib::kvr::compare_versions("0.6.4", "0.6.4"), Ordering::Equal);
}

#[test]
fn cmp_eq_0053() {
    assert_eq!(app_lib::kvr::compare_versions("0.6.5", "0.6.5"), Ordering::Equal);
}

#[test]
fn cmp_eq_0054() {
    assert_eq!(app_lib::kvr::compare_versions("0.6.6", "0.6.6"), Ordering::Equal);
}

#[test]
fn cmp_eq_0055() {
    assert_eq!(app_lib::kvr::compare_versions("0.6.7", "0.6.7"), Ordering::Equal);
}

#[test]
fn cmp_eq_0056() {
    assert_eq!(app_lib::kvr::compare_versions("0.7.0", "0.7.0"), Ordering::Equal);
}

#[test]
fn cmp_eq_0057() {
    assert_eq!(app_lib::kvr::compare_versions("0.7.1", "0.7.1"), Ordering::Equal);
}

#[test]
fn cmp_eq_0058() {
    assert_eq!(app_lib::kvr::compare_versions("0.7.2", "0.7.2"), Ordering::Equal);
}

#[test]
fn cmp_eq_0059() {
    assert_eq!(app_lib::kvr::compare_versions("0.7.3", "0.7.3"), Ordering::Equal);
}

#[test]
fn cmp_eq_0060() {
    assert_eq!(app_lib::kvr::compare_versions("0.7.4", "0.7.4"), Ordering::Equal);
}

#[test]
fn cmp_eq_0061() {
    assert_eq!(app_lib::kvr::compare_versions("0.7.5", "0.7.5"), Ordering::Equal);
}

#[test]
fn cmp_eq_0062() {
    assert_eq!(app_lib::kvr::compare_versions("0.7.6", "0.7.6"), Ordering::Equal);
}

#[test]
fn cmp_eq_0063() {
    assert_eq!(app_lib::kvr::compare_versions("0.7.7", "0.7.7"), Ordering::Equal);
}

#[test]
fn cmp_eq_0064() {
    assert_eq!(app_lib::kvr::compare_versions("1.0.0", "1.0.0"), Ordering::Equal);
}

#[test]
fn cmp_eq_0065() {
    assert_eq!(app_lib::kvr::compare_versions("1.0.1", "1.0.1"), Ordering::Equal);
}

#[test]
fn cmp_eq_0066() {
    assert_eq!(app_lib::kvr::compare_versions("1.0.2", "1.0.2"), Ordering::Equal);
}

#[test]
fn cmp_eq_0067() {
    assert_eq!(app_lib::kvr::compare_versions("1.0.3", "1.0.3"), Ordering::Equal);
}

#[test]
fn cmp_eq_0068() {
    assert_eq!(app_lib::kvr::compare_versions("1.0.4", "1.0.4"), Ordering::Equal);
}

#[test]
fn cmp_eq_0069() {
    assert_eq!(app_lib::kvr::compare_versions("1.0.5", "1.0.5"), Ordering::Equal);
}

#[test]
fn cmp_eq_0070() {
    assert_eq!(app_lib::kvr::compare_versions("1.0.6", "1.0.6"), Ordering::Equal);
}

#[test]
fn cmp_eq_0071() {
    assert_eq!(app_lib::kvr::compare_versions("1.0.7", "1.0.7"), Ordering::Equal);
}

#[test]
fn cmp_eq_0072() {
    assert_eq!(app_lib::kvr::compare_versions("1.1.0", "1.1.0"), Ordering::Equal);
}

#[test]
fn cmp_eq_0073() {
    assert_eq!(app_lib::kvr::compare_versions("1.1.1", "1.1.1"), Ordering::Equal);
}

#[test]
fn cmp_eq_0074() {
    assert_eq!(app_lib::kvr::compare_versions("1.1.2", "1.1.2"), Ordering::Equal);
}

#[test]
fn cmp_eq_0075() {
    assert_eq!(app_lib::kvr::compare_versions("1.1.3", "1.1.3"), Ordering::Equal);
}

#[test]
fn cmp_eq_0076() {
    assert_eq!(app_lib::kvr::compare_versions("1.1.4", "1.1.4"), Ordering::Equal);
}

#[test]
fn cmp_eq_0077() {
    assert_eq!(app_lib::kvr::compare_versions("1.1.5", "1.1.5"), Ordering::Equal);
}

#[test]
fn cmp_eq_0078() {
    assert_eq!(app_lib::kvr::compare_versions("1.1.6", "1.1.6"), Ordering::Equal);
}

#[test]
fn cmp_eq_0079() {
    assert_eq!(app_lib::kvr::compare_versions("1.1.7", "1.1.7"), Ordering::Equal);
}

#[test]
fn cmp_eq_0080() {
    assert_eq!(app_lib::kvr::compare_versions("1.2.0", "1.2.0"), Ordering::Equal);
}

#[test]
fn cmp_eq_0081() {
    assert_eq!(app_lib::kvr::compare_versions("1.2.1", "1.2.1"), Ordering::Equal);
}

#[test]
fn cmp_eq_0082() {
    assert_eq!(app_lib::kvr::compare_versions("1.2.2", "1.2.2"), Ordering::Equal);
}

#[test]
fn cmp_eq_0083() {
    assert_eq!(app_lib::kvr::compare_versions("1.2.3", "1.2.3"), Ordering::Equal);
}

#[test]
fn cmp_eq_0084() {
    assert_eq!(app_lib::kvr::compare_versions("1.2.4", "1.2.4"), Ordering::Equal);
}

#[test]
fn cmp_eq_0085() {
    assert_eq!(app_lib::kvr::compare_versions("1.2.5", "1.2.5"), Ordering::Equal);
}

#[test]
fn cmp_eq_0086() {
    assert_eq!(app_lib::kvr::compare_versions("1.2.6", "1.2.6"), Ordering::Equal);
}

#[test]
fn cmp_eq_0087() {
    assert_eq!(app_lib::kvr::compare_versions("1.2.7", "1.2.7"), Ordering::Equal);
}

#[test]
fn cmp_eq_0088() {
    assert_eq!(app_lib::kvr::compare_versions("1.3.0", "1.3.0"), Ordering::Equal);
}

#[test]
fn cmp_eq_0089() {
    assert_eq!(app_lib::kvr::compare_versions("1.3.1", "1.3.1"), Ordering::Equal);
}

#[test]
fn cmp_eq_0090() {
    assert_eq!(app_lib::kvr::compare_versions("1.3.2", "1.3.2"), Ordering::Equal);
}

#[test]
fn cmp_eq_0091() {
    assert_eq!(app_lib::kvr::compare_versions("1.3.3", "1.3.3"), Ordering::Equal);
}

#[test]
fn cmp_eq_0092() {
    assert_eq!(app_lib::kvr::compare_versions("1.3.4", "1.3.4"), Ordering::Equal);
}

#[test]
fn cmp_eq_0093() {
    assert_eq!(app_lib::kvr::compare_versions("1.3.5", "1.3.5"), Ordering::Equal);
}

#[test]
fn cmp_eq_0094() {
    assert_eq!(app_lib::kvr::compare_versions("1.3.6", "1.3.6"), Ordering::Equal);
}

#[test]
fn cmp_eq_0095() {
    assert_eq!(app_lib::kvr::compare_versions("1.3.7", "1.3.7"), Ordering::Equal);
}

#[test]
fn cmp_eq_0096() {
    assert_eq!(app_lib::kvr::compare_versions("1.4.0", "1.4.0"), Ordering::Equal);
}

#[test]
fn cmp_eq_0097() {
    assert_eq!(app_lib::kvr::compare_versions("1.4.1", "1.4.1"), Ordering::Equal);
}

#[test]
fn cmp_eq_0098() {
    assert_eq!(app_lib::kvr::compare_versions("1.4.2", "1.4.2"), Ordering::Equal);
}

#[test]
fn cmp_eq_0099() {
    assert_eq!(app_lib::kvr::compare_versions("1.4.3", "1.4.3"), Ordering::Equal);
}

#[test]
fn cmp_eq_0100() {
    assert_eq!(app_lib::kvr::compare_versions("1.4.4", "1.4.4"), Ordering::Equal);
}

#[test]
fn cmp_eq_0101() {
    assert_eq!(app_lib::kvr::compare_versions("1.4.5", "1.4.5"), Ordering::Equal);
}

#[test]
fn cmp_eq_0102() {
    assert_eq!(app_lib::kvr::compare_versions("1.4.6", "1.4.6"), Ordering::Equal);
}

#[test]
fn cmp_eq_0103() {
    assert_eq!(app_lib::kvr::compare_versions("1.4.7", "1.4.7"), Ordering::Equal);
}

#[test]
fn cmp_eq_0104() {
    assert_eq!(app_lib::kvr::compare_versions("1.5.0", "1.5.0"), Ordering::Equal);
}

#[test]
fn cmp_eq_0105() {
    assert_eq!(app_lib::kvr::compare_versions("1.5.1", "1.5.1"), Ordering::Equal);
}

#[test]
fn cmp_eq_0106() {
    assert_eq!(app_lib::kvr::compare_versions("1.5.2", "1.5.2"), Ordering::Equal);
}

#[test]
fn cmp_eq_0107() {
    assert_eq!(app_lib::kvr::compare_versions("1.5.3", "1.5.3"), Ordering::Equal);
}

#[test]
fn cmp_eq_0108() {
    assert_eq!(app_lib::kvr::compare_versions("1.5.4", "1.5.4"), Ordering::Equal);
}

#[test]
fn cmp_eq_0109() {
    assert_eq!(app_lib::kvr::compare_versions("1.5.5", "1.5.5"), Ordering::Equal);
}

#[test]
fn cmp_eq_0110() {
    assert_eq!(app_lib::kvr::compare_versions("1.5.6", "1.5.6"), Ordering::Equal);
}

#[test]
fn cmp_eq_0111() {
    assert_eq!(app_lib::kvr::compare_versions("1.5.7", "1.5.7"), Ordering::Equal);
}

#[test]
fn cmp_eq_0112() {
    assert_eq!(app_lib::kvr::compare_versions("1.6.0", "1.6.0"), Ordering::Equal);
}

#[test]
fn cmp_eq_0113() {
    assert_eq!(app_lib::kvr::compare_versions("1.6.1", "1.6.1"), Ordering::Equal);
}

#[test]
fn cmp_eq_0114() {
    assert_eq!(app_lib::kvr::compare_versions("1.6.2", "1.6.2"), Ordering::Equal);
}

#[test]
fn cmp_eq_0115() {
    assert_eq!(app_lib::kvr::compare_versions("1.6.3", "1.6.3"), Ordering::Equal);
}

#[test]
fn cmp_eq_0116() {
    assert_eq!(app_lib::kvr::compare_versions("1.6.4", "1.6.4"), Ordering::Equal);
}

#[test]
fn cmp_eq_0117() {
    assert_eq!(app_lib::kvr::compare_versions("1.6.5", "1.6.5"), Ordering::Equal);
}

#[test]
fn cmp_eq_0118() {
    assert_eq!(app_lib::kvr::compare_versions("1.6.6", "1.6.6"), Ordering::Equal);
}

#[test]
fn cmp_eq_0119() {
    assert_eq!(app_lib::kvr::compare_versions("1.6.7", "1.6.7"), Ordering::Equal);
}

#[test]
fn cmp_eq_0120() {
    assert_eq!(app_lib::kvr::compare_versions("1.7.0", "1.7.0"), Ordering::Equal);
}

#[test]
fn cmp_eq_0121() {
    assert_eq!(app_lib::kvr::compare_versions("1.7.1", "1.7.1"), Ordering::Equal);
}

#[test]
fn cmp_eq_0122() {
    assert_eq!(app_lib::kvr::compare_versions("1.7.2", "1.7.2"), Ordering::Equal);
}

#[test]
fn cmp_eq_0123() {
    assert_eq!(app_lib::kvr::compare_versions("1.7.3", "1.7.3"), Ordering::Equal);
}

#[test]
fn cmp_eq_0124() {
    assert_eq!(app_lib::kvr::compare_versions("1.7.4", "1.7.4"), Ordering::Equal);
}

#[test]
fn cmp_eq_0125() {
    assert_eq!(app_lib::kvr::compare_versions("1.7.5", "1.7.5"), Ordering::Equal);
}

#[test]
fn cmp_eq_0126() {
    assert_eq!(app_lib::kvr::compare_versions("1.7.6", "1.7.6"), Ordering::Equal);
}

#[test]
fn cmp_eq_0127() {
    assert_eq!(app_lib::kvr::compare_versions("1.7.7", "1.7.7"), Ordering::Equal);
}

#[test]
fn cmp_eq_0128() {
    assert_eq!(app_lib::kvr::compare_versions("2.0.0", "2.0.0"), Ordering::Equal);
}

#[test]
fn cmp_eq_0129() {
    assert_eq!(app_lib::kvr::compare_versions("2.0.1", "2.0.1"), Ordering::Equal);
}

#[test]
fn cmp_eq_0130() {
    assert_eq!(app_lib::kvr::compare_versions("2.0.2", "2.0.2"), Ordering::Equal);
}

#[test]
fn cmp_eq_0131() {
    assert_eq!(app_lib::kvr::compare_versions("2.0.3", "2.0.3"), Ordering::Equal);
}

#[test]
fn cmp_eq_0132() {
    assert_eq!(app_lib::kvr::compare_versions("2.0.4", "2.0.4"), Ordering::Equal);
}

#[test]
fn cmp_eq_0133() {
    assert_eq!(app_lib::kvr::compare_versions("2.0.5", "2.0.5"), Ordering::Equal);
}

#[test]
fn cmp_eq_0134() {
    assert_eq!(app_lib::kvr::compare_versions("2.0.6", "2.0.6"), Ordering::Equal);
}

#[test]
fn cmp_eq_0135() {
    assert_eq!(app_lib::kvr::compare_versions("2.0.7", "2.0.7"), Ordering::Equal);
}

#[test]
fn cmp_eq_0136() {
    assert_eq!(app_lib::kvr::compare_versions("2.1.0", "2.1.0"), Ordering::Equal);
}

#[test]
fn cmp_eq_0137() {
    assert_eq!(app_lib::kvr::compare_versions("2.1.1", "2.1.1"), Ordering::Equal);
}

#[test]
fn cmp_eq_0138() {
    assert_eq!(app_lib::kvr::compare_versions("2.1.2", "2.1.2"), Ordering::Equal);
}

#[test]
fn cmp_eq_0139() {
    assert_eq!(app_lib::kvr::compare_versions("2.1.3", "2.1.3"), Ordering::Equal);
}

#[test]
fn cmp_eq_0140() {
    assert_eq!(app_lib::kvr::compare_versions("2.1.4", "2.1.4"), Ordering::Equal);
}

#[test]
fn cmp_eq_0141() {
    assert_eq!(app_lib::kvr::compare_versions("2.1.5", "2.1.5"), Ordering::Equal);
}

#[test]
fn cmp_eq_0142() {
    assert_eq!(app_lib::kvr::compare_versions("2.1.6", "2.1.6"), Ordering::Equal);
}

#[test]
fn cmp_eq_0143() {
    assert_eq!(app_lib::kvr::compare_versions("2.1.7", "2.1.7"), Ordering::Equal);
}

#[test]
fn cmp_eq_0144() {
    assert_eq!(app_lib::kvr::compare_versions("2.2.0", "2.2.0"), Ordering::Equal);
}

#[test]
fn cmp_eq_0145() {
    assert_eq!(app_lib::kvr::compare_versions("2.2.1", "2.2.1"), Ordering::Equal);
}

#[test]
fn cmp_eq_0146() {
    assert_eq!(app_lib::kvr::compare_versions("2.2.2", "2.2.2"), Ordering::Equal);
}

#[test]
fn cmp_eq_0147() {
    assert_eq!(app_lib::kvr::compare_versions("2.2.3", "2.2.3"), Ordering::Equal);
}

#[test]
fn cmp_eq_0148() {
    assert_eq!(app_lib::kvr::compare_versions("2.2.4", "2.2.4"), Ordering::Equal);
}

#[test]
fn cmp_eq_0149() {
    assert_eq!(app_lib::kvr::compare_versions("2.2.5", "2.2.5"), Ordering::Equal);
}

#[test]
fn cmp_eq_0150() {
    assert_eq!(app_lib::kvr::compare_versions("2.2.6", "2.2.6"), Ordering::Equal);
}

#[test]
fn cmp_eq_0151() {
    assert_eq!(app_lib::kvr::compare_versions("2.2.7", "2.2.7"), Ordering::Equal);
}

#[test]
fn cmp_eq_0152() {
    assert_eq!(app_lib::kvr::compare_versions("2.3.0", "2.3.0"), Ordering::Equal);
}

#[test]
fn cmp_eq_0153() {
    assert_eq!(app_lib::kvr::compare_versions("2.3.1", "2.3.1"), Ordering::Equal);
}

#[test]
fn cmp_eq_0154() {
    assert_eq!(app_lib::kvr::compare_versions("2.3.2", "2.3.2"), Ordering::Equal);
}

#[test]
fn cmp_eq_0155() {
    assert_eq!(app_lib::kvr::compare_versions("2.3.3", "2.3.3"), Ordering::Equal);
}

#[test]
fn cmp_eq_0156() {
    assert_eq!(app_lib::kvr::compare_versions("2.3.4", "2.3.4"), Ordering::Equal);
}

#[test]
fn cmp_eq_0157() {
    assert_eq!(app_lib::kvr::compare_versions("2.3.5", "2.3.5"), Ordering::Equal);
}

#[test]
fn cmp_eq_0158() {
    assert_eq!(app_lib::kvr::compare_versions("2.3.6", "2.3.6"), Ordering::Equal);
}

#[test]
fn cmp_eq_0159() {
    assert_eq!(app_lib::kvr::compare_versions("2.3.7", "2.3.7"), Ordering::Equal);
}

#[test]
fn cmp_eq_0160() {
    assert_eq!(app_lib::kvr::compare_versions("2.4.0", "2.4.0"), Ordering::Equal);
}

#[test]
fn cmp_eq_0161() {
    assert_eq!(app_lib::kvr::compare_versions("2.4.1", "2.4.1"), Ordering::Equal);
}

#[test]
fn cmp_eq_0162() {
    assert_eq!(app_lib::kvr::compare_versions("2.4.2", "2.4.2"), Ordering::Equal);
}

#[test]
fn cmp_eq_0163() {
    assert_eq!(app_lib::kvr::compare_versions("2.4.3", "2.4.3"), Ordering::Equal);
}

#[test]
fn cmp_eq_0164() {
    assert_eq!(app_lib::kvr::compare_versions("2.4.4", "2.4.4"), Ordering::Equal);
}

#[test]
fn cmp_eq_0165() {
    assert_eq!(app_lib::kvr::compare_versions("2.4.5", "2.4.5"), Ordering::Equal);
}

#[test]
fn cmp_eq_0166() {
    assert_eq!(app_lib::kvr::compare_versions("2.4.6", "2.4.6"), Ordering::Equal);
}

#[test]
fn cmp_eq_0167() {
    assert_eq!(app_lib::kvr::compare_versions("2.4.7", "2.4.7"), Ordering::Equal);
}

#[test]
fn cmp_eq_0168() {
    assert_eq!(app_lib::kvr::compare_versions("2.5.0", "2.5.0"), Ordering::Equal);
}

#[test]
fn cmp_eq_0169() {
    assert_eq!(app_lib::kvr::compare_versions("2.5.1", "2.5.1"), Ordering::Equal);
}

#[test]
fn cmp_eq_0170() {
    assert_eq!(app_lib::kvr::compare_versions("2.5.2", "2.5.2"), Ordering::Equal);
}

#[test]
fn cmp_eq_0171() {
    assert_eq!(app_lib::kvr::compare_versions("2.5.3", "2.5.3"), Ordering::Equal);
}

#[test]
fn cmp_eq_0172() {
    assert_eq!(app_lib::kvr::compare_versions("2.5.4", "2.5.4"), Ordering::Equal);
}

#[test]
fn cmp_eq_0173() {
    assert_eq!(app_lib::kvr::compare_versions("2.5.5", "2.5.5"), Ordering::Equal);
}

#[test]
fn cmp_eq_0174() {
    assert_eq!(app_lib::kvr::compare_versions("2.5.6", "2.5.6"), Ordering::Equal);
}

#[test]
fn cmp_eq_0175() {
    assert_eq!(app_lib::kvr::compare_versions("2.5.7", "2.5.7"), Ordering::Equal);
}

#[test]
fn cmp_eq_0176() {
    assert_eq!(app_lib::kvr::compare_versions("2.6.0", "2.6.0"), Ordering::Equal);
}

#[test]
fn cmp_eq_0177() {
    assert_eq!(app_lib::kvr::compare_versions("2.6.1", "2.6.1"), Ordering::Equal);
}

#[test]
fn cmp_eq_0178() {
    assert_eq!(app_lib::kvr::compare_versions("2.6.2", "2.6.2"), Ordering::Equal);
}

#[test]
fn cmp_eq_0179() {
    assert_eq!(app_lib::kvr::compare_versions("2.6.3", "2.6.3"), Ordering::Equal);
}

#[test]
fn cmp_eq_0180() {
    assert_eq!(app_lib::kvr::compare_versions("2.6.4", "2.6.4"), Ordering::Equal);
}

#[test]
fn cmp_eq_0181() {
    assert_eq!(app_lib::kvr::compare_versions("2.6.5", "2.6.5"), Ordering::Equal);
}

#[test]
fn cmp_eq_0182() {
    assert_eq!(app_lib::kvr::compare_versions("2.6.6", "2.6.6"), Ordering::Equal);
}

#[test]
fn cmp_eq_0183() {
    assert_eq!(app_lib::kvr::compare_versions("2.6.7", "2.6.7"), Ordering::Equal);
}

#[test]
fn cmp_eq_0184() {
    assert_eq!(app_lib::kvr::compare_versions("2.7.0", "2.7.0"), Ordering::Equal);
}

#[test]
fn cmp_eq_0185() {
    assert_eq!(app_lib::kvr::compare_versions("2.7.1", "2.7.1"), Ordering::Equal);
}

#[test]
fn cmp_eq_0186() {
    assert_eq!(app_lib::kvr::compare_versions("2.7.2", "2.7.2"), Ordering::Equal);
}

#[test]
fn cmp_eq_0187() {
    assert_eq!(app_lib::kvr::compare_versions("2.7.3", "2.7.3"), Ordering::Equal);
}

#[test]
fn cmp_eq_0188() {
    assert_eq!(app_lib::kvr::compare_versions("2.7.4", "2.7.4"), Ordering::Equal);
}

#[test]
fn cmp_eq_0189() {
    assert_eq!(app_lib::kvr::compare_versions("2.7.5", "2.7.5"), Ordering::Equal);
}

#[test]
fn cmp_eq_0190() {
    assert_eq!(app_lib::kvr::compare_versions("2.7.6", "2.7.6"), Ordering::Equal);
}

#[test]
fn cmp_eq_0191() {
    assert_eq!(app_lib::kvr::compare_versions("2.7.7", "2.7.7"), Ordering::Equal);
}

#[test]
fn cmp_eq_0192() {
    assert_eq!(app_lib::kvr::compare_versions("3.0.0", "3.0.0"), Ordering::Equal);
}

#[test]
fn cmp_eq_0193() {
    assert_eq!(app_lib::kvr::compare_versions("3.0.1", "3.0.1"), Ordering::Equal);
}

#[test]
fn cmp_eq_0194() {
    assert_eq!(app_lib::kvr::compare_versions("3.0.2", "3.0.2"), Ordering::Equal);
}

#[test]
fn cmp_eq_0195() {
    assert_eq!(app_lib::kvr::compare_versions("3.0.3", "3.0.3"), Ordering::Equal);
}

#[test]
fn cmp_eq_0196() {
    assert_eq!(app_lib::kvr::compare_versions("3.0.4", "3.0.4"), Ordering::Equal);
}

#[test]
fn cmp_eq_0197() {
    assert_eq!(app_lib::kvr::compare_versions("3.0.5", "3.0.5"), Ordering::Equal);
}

#[test]
fn cmp_eq_0198() {
    assert_eq!(app_lib::kvr::compare_versions("3.0.6", "3.0.6"), Ordering::Equal);
}

#[test]
fn cmp_eq_0199() {
    assert_eq!(app_lib::kvr::compare_versions("3.0.7", "3.0.7"), Ordering::Equal);
}

#[test]
fn cmp_eq_0200() {
    assert_eq!(app_lib::kvr::compare_versions("3.1.0", "3.1.0"), Ordering::Equal);
}

#[test]
fn cmp_eq_0201() {
    assert_eq!(app_lib::kvr::compare_versions("3.1.1", "3.1.1"), Ordering::Equal);
}

#[test]
fn cmp_eq_0202() {
    assert_eq!(app_lib::kvr::compare_versions("3.1.2", "3.1.2"), Ordering::Equal);
}

#[test]
fn cmp_eq_0203() {
    assert_eq!(app_lib::kvr::compare_versions("3.1.3", "3.1.3"), Ordering::Equal);
}

#[test]
fn cmp_eq_0204() {
    assert_eq!(app_lib::kvr::compare_versions("3.1.4", "3.1.4"), Ordering::Equal);
}

#[test]
fn cmp_eq_0205() {
    assert_eq!(app_lib::kvr::compare_versions("3.1.5", "3.1.5"), Ordering::Equal);
}

#[test]
fn cmp_eq_0206() {
    assert_eq!(app_lib::kvr::compare_versions("3.1.6", "3.1.6"), Ordering::Equal);
}

#[test]
fn cmp_eq_0207() {
    assert_eq!(app_lib::kvr::compare_versions("3.1.7", "3.1.7"), Ordering::Equal);
}

#[test]
fn cmp_eq_0208() {
    assert_eq!(app_lib::kvr::compare_versions("3.2.0", "3.2.0"), Ordering::Equal);
}

#[test]
fn cmp_eq_0209() {
    assert_eq!(app_lib::kvr::compare_versions("3.2.1", "3.2.1"), Ordering::Equal);
}

#[test]
fn cmp_eq_0210() {
    assert_eq!(app_lib::kvr::compare_versions("3.2.2", "3.2.2"), Ordering::Equal);
}

#[test]
fn cmp_eq_0211() {
    assert_eq!(app_lib::kvr::compare_versions("3.2.3", "3.2.3"), Ordering::Equal);
}

#[test]
fn cmp_eq_0212() {
    assert_eq!(app_lib::kvr::compare_versions("3.2.4", "3.2.4"), Ordering::Equal);
}

#[test]
fn cmp_eq_0213() {
    assert_eq!(app_lib::kvr::compare_versions("3.2.5", "3.2.5"), Ordering::Equal);
}

#[test]
fn cmp_eq_0214() {
    assert_eq!(app_lib::kvr::compare_versions("3.2.6", "3.2.6"), Ordering::Equal);
}

#[test]
fn cmp_eq_0215() {
    assert_eq!(app_lib::kvr::compare_versions("3.2.7", "3.2.7"), Ordering::Equal);
}

#[test]
fn cmp_eq_0216() {
    assert_eq!(app_lib::kvr::compare_versions("3.3.0", "3.3.0"), Ordering::Equal);
}

#[test]
fn cmp_eq_0217() {
    assert_eq!(app_lib::kvr::compare_versions("3.3.1", "3.3.1"), Ordering::Equal);
}

#[test]
fn cmp_eq_0218() {
    assert_eq!(app_lib::kvr::compare_versions("3.3.2", "3.3.2"), Ordering::Equal);
}

#[test]
fn cmp_eq_0219() {
    assert_eq!(app_lib::kvr::compare_versions("3.3.3", "3.3.3"), Ordering::Equal);
}

#[test]
fn cmp_eq_0220() {
    assert_eq!(app_lib::kvr::compare_versions("3.3.4", "3.3.4"), Ordering::Equal);
}

#[test]
fn cmp_eq_0221() {
    assert_eq!(app_lib::kvr::compare_versions("3.3.5", "3.3.5"), Ordering::Equal);
}

#[test]
fn cmp_eq_0222() {
    assert_eq!(app_lib::kvr::compare_versions("3.3.6", "3.3.6"), Ordering::Equal);
}

#[test]
fn cmp_eq_0223() {
    assert_eq!(app_lib::kvr::compare_versions("3.3.7", "3.3.7"), Ordering::Equal);
}

#[test]
fn cmp_eq_0224() {
    assert_eq!(app_lib::kvr::compare_versions("3.4.0", "3.4.0"), Ordering::Equal);
}

#[test]
fn cmp_eq_0225() {
    assert_eq!(app_lib::kvr::compare_versions("3.4.1", "3.4.1"), Ordering::Equal);
}

#[test]
fn cmp_eq_0226() {
    assert_eq!(app_lib::kvr::compare_versions("3.4.2", "3.4.2"), Ordering::Equal);
}

#[test]
fn cmp_eq_0227() {
    assert_eq!(app_lib::kvr::compare_versions("3.4.3", "3.4.3"), Ordering::Equal);
}

#[test]
fn cmp_eq_0228() {
    assert_eq!(app_lib::kvr::compare_versions("3.4.4", "3.4.4"), Ordering::Equal);
}

#[test]
fn cmp_eq_0229() {
    assert_eq!(app_lib::kvr::compare_versions("3.4.5", "3.4.5"), Ordering::Equal);
}

#[test]
fn cmp_eq_0230() {
    assert_eq!(app_lib::kvr::compare_versions("3.4.6", "3.4.6"), Ordering::Equal);
}

#[test]
fn cmp_eq_0231() {
    assert_eq!(app_lib::kvr::compare_versions("3.4.7", "3.4.7"), Ordering::Equal);
}

#[test]
fn cmp_eq_0232() {
    assert_eq!(app_lib::kvr::compare_versions("3.5.0", "3.5.0"), Ordering::Equal);
}

#[test]
fn cmp_eq_0233() {
    assert_eq!(app_lib::kvr::compare_versions("3.5.1", "3.5.1"), Ordering::Equal);
}

#[test]
fn cmp_eq_0234() {
    assert_eq!(app_lib::kvr::compare_versions("3.5.2", "3.5.2"), Ordering::Equal);
}

#[test]
fn cmp_eq_0235() {
    assert_eq!(app_lib::kvr::compare_versions("3.5.3", "3.5.3"), Ordering::Equal);
}

#[test]
fn cmp_eq_0236() {
    assert_eq!(app_lib::kvr::compare_versions("3.5.4", "3.5.4"), Ordering::Equal);
}

#[test]
fn cmp_eq_0237() {
    assert_eq!(app_lib::kvr::compare_versions("3.5.5", "3.5.5"), Ordering::Equal);
}

#[test]
fn cmp_eq_0238() {
    assert_eq!(app_lib::kvr::compare_versions("3.5.6", "3.5.6"), Ordering::Equal);
}

#[test]
fn cmp_eq_0239() {
    assert_eq!(app_lib::kvr::compare_versions("3.5.7", "3.5.7"), Ordering::Equal);
}

#[test]
fn cmp_eq_0240() {
    assert_eq!(app_lib::kvr::compare_versions("3.6.0", "3.6.0"), Ordering::Equal);
}

#[test]
fn cmp_eq_0241() {
    assert_eq!(app_lib::kvr::compare_versions("3.6.1", "3.6.1"), Ordering::Equal);
}

#[test]
fn cmp_eq_0242() {
    assert_eq!(app_lib::kvr::compare_versions("3.6.2", "3.6.2"), Ordering::Equal);
}

#[test]
fn cmp_eq_0243() {
    assert_eq!(app_lib::kvr::compare_versions("3.6.3", "3.6.3"), Ordering::Equal);
}

#[test]
fn cmp_eq_0244() {
    assert_eq!(app_lib::kvr::compare_versions("3.6.4", "3.6.4"), Ordering::Equal);
}

#[test]
fn cmp_eq_0245() {
    assert_eq!(app_lib::kvr::compare_versions("3.6.5", "3.6.5"), Ordering::Equal);
}

#[test]
fn cmp_eq_0246() {
    assert_eq!(app_lib::kvr::compare_versions("3.6.6", "3.6.6"), Ordering::Equal);
}

#[test]
fn cmp_eq_0247() {
    assert_eq!(app_lib::kvr::compare_versions("3.6.7", "3.6.7"), Ordering::Equal);
}

#[test]
fn cmp_eq_0248() {
    assert_eq!(app_lib::kvr::compare_versions("3.7.0", "3.7.0"), Ordering::Equal);
}

#[test]
fn cmp_eq_0249() {
    assert_eq!(app_lib::kvr::compare_versions("3.7.1", "3.7.1"), Ordering::Equal);
}

#[test]
fn cmp_eq_0250() {
    assert_eq!(app_lib::kvr::compare_versions("3.7.2", "3.7.2"), Ordering::Equal);
}

#[test]
fn cmp_eq_0251() {
    assert_eq!(app_lib::kvr::compare_versions("3.7.3", "3.7.3"), Ordering::Equal);
}

#[test]
fn cmp_eq_0252() {
    assert_eq!(app_lib::kvr::compare_versions("3.7.4", "3.7.4"), Ordering::Equal);
}

#[test]
fn cmp_eq_0253() {
    assert_eq!(app_lib::kvr::compare_versions("3.7.5", "3.7.5"), Ordering::Equal);
}

#[test]
fn cmp_eq_0254() {
    assert_eq!(app_lib::kvr::compare_versions("3.7.6", "3.7.6"), Ordering::Equal);
}

#[test]
fn cmp_eq_0255() {
    assert_eq!(app_lib::kvr::compare_versions("3.7.7", "3.7.7"), Ordering::Equal);
}

#[test]
fn cmp_eq_0256() {
    assert_eq!(app_lib::kvr::compare_versions("4.0.0", "4.0.0"), Ordering::Equal);
}

#[test]
fn cmp_eq_0257() {
    assert_eq!(app_lib::kvr::compare_versions("4.0.1", "4.0.1"), Ordering::Equal);
}

#[test]
fn cmp_eq_0258() {
    assert_eq!(app_lib::kvr::compare_versions("4.0.2", "4.0.2"), Ordering::Equal);
}

#[test]
fn cmp_eq_0259() {
    assert_eq!(app_lib::kvr::compare_versions("4.0.3", "4.0.3"), Ordering::Equal);
}

#[test]
fn cmp_eq_0260() {
    assert_eq!(app_lib::kvr::compare_versions("4.0.4", "4.0.4"), Ordering::Equal);
}

#[test]
fn cmp_eq_0261() {
    assert_eq!(app_lib::kvr::compare_versions("4.0.5", "4.0.5"), Ordering::Equal);
}

#[test]
fn cmp_eq_0262() {
    assert_eq!(app_lib::kvr::compare_versions("4.0.6", "4.0.6"), Ordering::Equal);
}

#[test]
fn cmp_eq_0263() {
    assert_eq!(app_lib::kvr::compare_versions("4.0.7", "4.0.7"), Ordering::Equal);
}

#[test]
fn cmp_eq_0264() {
    assert_eq!(app_lib::kvr::compare_versions("4.1.0", "4.1.0"), Ordering::Equal);
}

#[test]
fn cmp_eq_0265() {
    assert_eq!(app_lib::kvr::compare_versions("4.1.1", "4.1.1"), Ordering::Equal);
}

#[test]
fn cmp_eq_0266() {
    assert_eq!(app_lib::kvr::compare_versions("4.1.2", "4.1.2"), Ordering::Equal);
}

#[test]
fn cmp_eq_0267() {
    assert_eq!(app_lib::kvr::compare_versions("4.1.3", "4.1.3"), Ordering::Equal);
}

#[test]
fn cmp_eq_0268() {
    assert_eq!(app_lib::kvr::compare_versions("4.1.4", "4.1.4"), Ordering::Equal);
}

#[test]
fn cmp_eq_0269() {
    assert_eq!(app_lib::kvr::compare_versions("4.1.5", "4.1.5"), Ordering::Equal);
}

#[test]
fn cmp_eq_0270() {
    assert_eq!(app_lib::kvr::compare_versions("4.1.6", "4.1.6"), Ordering::Equal);
}

#[test]
fn cmp_eq_0271() {
    assert_eq!(app_lib::kvr::compare_versions("4.1.7", "4.1.7"), Ordering::Equal);
}

#[test]
fn cmp_eq_0272() {
    assert_eq!(app_lib::kvr::compare_versions("4.2.0", "4.2.0"), Ordering::Equal);
}

#[test]
fn cmp_eq_0273() {
    assert_eq!(app_lib::kvr::compare_versions("4.2.1", "4.2.1"), Ordering::Equal);
}

#[test]
fn cmp_eq_0274() {
    assert_eq!(app_lib::kvr::compare_versions("4.2.2", "4.2.2"), Ordering::Equal);
}

#[test]
fn cmp_eq_0275() {
    assert_eq!(app_lib::kvr::compare_versions("4.2.3", "4.2.3"), Ordering::Equal);
}

#[test]
fn cmp_eq_0276() {
    assert_eq!(app_lib::kvr::compare_versions("4.2.4", "4.2.4"), Ordering::Equal);
}

#[test]
fn cmp_eq_0277() {
    assert_eq!(app_lib::kvr::compare_versions("4.2.5", "4.2.5"), Ordering::Equal);
}

#[test]
fn cmp_eq_0278() {
    assert_eq!(app_lib::kvr::compare_versions("4.2.6", "4.2.6"), Ordering::Equal);
}

#[test]
fn cmp_eq_0279() {
    assert_eq!(app_lib::kvr::compare_versions("4.2.7", "4.2.7"), Ordering::Equal);
}

#[test]
fn cmp_eq_0280() {
    assert_eq!(app_lib::kvr::compare_versions("4.3.0", "4.3.0"), Ordering::Equal);
}

#[test]
fn cmp_eq_0281() {
    assert_eq!(app_lib::kvr::compare_versions("4.3.1", "4.3.1"), Ordering::Equal);
}

#[test]
fn cmp_eq_0282() {
    assert_eq!(app_lib::kvr::compare_versions("4.3.2", "4.3.2"), Ordering::Equal);
}

#[test]
fn cmp_eq_0283() {
    assert_eq!(app_lib::kvr::compare_versions("4.3.3", "4.3.3"), Ordering::Equal);
}

#[test]
fn cmp_eq_0284() {
    assert_eq!(app_lib::kvr::compare_versions("4.3.4", "4.3.4"), Ordering::Equal);
}

#[test]
fn cmp_eq_0285() {
    assert_eq!(app_lib::kvr::compare_versions("4.3.5", "4.3.5"), Ordering::Equal);
}

#[test]
fn cmp_eq_0286() {
    assert_eq!(app_lib::kvr::compare_versions("4.3.6", "4.3.6"), Ordering::Equal);
}

#[test]
fn cmp_eq_0287() {
    assert_eq!(app_lib::kvr::compare_versions("4.3.7", "4.3.7"), Ordering::Equal);
}

#[test]
fn cmp_eq_0288() {
    assert_eq!(app_lib::kvr::compare_versions("4.4.0", "4.4.0"), Ordering::Equal);
}

#[test]
fn cmp_eq_0289() {
    assert_eq!(app_lib::kvr::compare_versions("4.4.1", "4.4.1"), Ordering::Equal);
}

#[test]
fn cmp_eq_0290() {
    assert_eq!(app_lib::kvr::compare_versions("4.4.2", "4.4.2"), Ordering::Equal);
}

#[test]
fn cmp_eq_0291() {
    assert_eq!(app_lib::kvr::compare_versions("4.4.3", "4.4.3"), Ordering::Equal);
}

#[test]
fn cmp_eq_0292() {
    assert_eq!(app_lib::kvr::compare_versions("4.4.4", "4.4.4"), Ordering::Equal);
}

#[test]
fn cmp_eq_0293() {
    assert_eq!(app_lib::kvr::compare_versions("4.4.5", "4.4.5"), Ordering::Equal);
}

#[test]
fn cmp_eq_0294() {
    assert_eq!(app_lib::kvr::compare_versions("4.4.6", "4.4.6"), Ordering::Equal);
}

#[test]
fn cmp_eq_0295() {
    assert_eq!(app_lib::kvr::compare_versions("4.4.7", "4.4.7"), Ordering::Equal);
}

#[test]
fn cmp_eq_0296() {
    assert_eq!(app_lib::kvr::compare_versions("4.5.0", "4.5.0"), Ordering::Equal);
}

#[test]
fn cmp_eq_0297() {
    assert_eq!(app_lib::kvr::compare_versions("4.5.1", "4.5.1"), Ordering::Equal);
}

#[test]
fn cmp_eq_0298() {
    assert_eq!(app_lib::kvr::compare_versions("4.5.2", "4.5.2"), Ordering::Equal);
}

#[test]
fn cmp_eq_0299() {
    assert_eq!(app_lib::kvr::compare_versions("4.5.3", "4.5.3"), Ordering::Equal);
}

#[test]
fn cmp_eq_0300() {
    assert_eq!(app_lib::kvr::compare_versions("4.5.4", "4.5.4"), Ordering::Equal);
}

#[test]
fn cmp_eq_0301() {
    assert_eq!(app_lib::kvr::compare_versions("4.5.5", "4.5.5"), Ordering::Equal);
}

#[test]
fn cmp_eq_0302() {
    assert_eq!(app_lib::kvr::compare_versions("4.5.6", "4.5.6"), Ordering::Equal);
}

#[test]
fn cmp_eq_0303() {
    assert_eq!(app_lib::kvr::compare_versions("4.5.7", "4.5.7"), Ordering::Equal);
}

#[test]
fn cmp_eq_0304() {
    assert_eq!(app_lib::kvr::compare_versions("4.6.0", "4.6.0"), Ordering::Equal);
}

#[test]
fn cmp_eq_0305() {
    assert_eq!(app_lib::kvr::compare_versions("4.6.1", "4.6.1"), Ordering::Equal);
}

#[test]
fn cmp_eq_0306() {
    assert_eq!(app_lib::kvr::compare_versions("4.6.2", "4.6.2"), Ordering::Equal);
}

#[test]
fn cmp_eq_0307() {
    assert_eq!(app_lib::kvr::compare_versions("4.6.3", "4.6.3"), Ordering::Equal);
}

#[test]
fn cmp_eq_0308() {
    assert_eq!(app_lib::kvr::compare_versions("4.6.4", "4.6.4"), Ordering::Equal);
}

#[test]
fn cmp_eq_0309() {
    assert_eq!(app_lib::kvr::compare_versions("4.6.5", "4.6.5"), Ordering::Equal);
}

#[test]
fn cmp_eq_0310() {
    assert_eq!(app_lib::kvr::compare_versions("4.6.6", "4.6.6"), Ordering::Equal);
}

#[test]
fn cmp_eq_0311() {
    assert_eq!(app_lib::kvr::compare_versions("4.6.7", "4.6.7"), Ordering::Equal);
}

#[test]
fn cmp_eq_0312() {
    assert_eq!(app_lib::kvr::compare_versions("4.7.0", "4.7.0"), Ordering::Equal);
}

#[test]
fn cmp_eq_0313() {
    assert_eq!(app_lib::kvr::compare_versions("4.7.1", "4.7.1"), Ordering::Equal);
}

#[test]
fn cmp_eq_0314() {
    assert_eq!(app_lib::kvr::compare_versions("4.7.2", "4.7.2"), Ordering::Equal);
}

#[test]
fn cmp_eq_0315() {
    assert_eq!(app_lib::kvr::compare_versions("4.7.3", "4.7.3"), Ordering::Equal);
}

#[test]
fn cmp_eq_0316() {
    assert_eq!(app_lib::kvr::compare_versions("4.7.4", "4.7.4"), Ordering::Equal);
}

#[test]
fn cmp_eq_0317() {
    assert_eq!(app_lib::kvr::compare_versions("4.7.5", "4.7.5"), Ordering::Equal);
}

#[test]
fn cmp_eq_0318() {
    assert_eq!(app_lib::kvr::compare_versions("4.7.6", "4.7.6"), Ordering::Equal);
}

#[test]
fn cmp_eq_0319() {
    assert_eq!(app_lib::kvr::compare_versions("4.7.7", "4.7.7"), Ordering::Equal);
}

#[test]
fn cmp_eq_0320() {
    assert_eq!(app_lib::kvr::compare_versions("5.0.0", "5.0.0"), Ordering::Equal);
}

#[test]
fn cmp_eq_0321() {
    assert_eq!(app_lib::kvr::compare_versions("5.0.1", "5.0.1"), Ordering::Equal);
}

#[test]
fn cmp_eq_0322() {
    assert_eq!(app_lib::kvr::compare_versions("5.0.2", "5.0.2"), Ordering::Equal);
}

#[test]
fn cmp_eq_0323() {
    assert_eq!(app_lib::kvr::compare_versions("5.0.3", "5.0.3"), Ordering::Equal);
}

#[test]
fn cmp_eq_0324() {
    assert_eq!(app_lib::kvr::compare_versions("5.0.4", "5.0.4"), Ordering::Equal);
}

#[test]
fn cmp_eq_0325() {
    assert_eq!(app_lib::kvr::compare_versions("5.0.5", "5.0.5"), Ordering::Equal);
}

#[test]
fn cmp_eq_0326() {
    assert_eq!(app_lib::kvr::compare_versions("5.0.6", "5.0.6"), Ordering::Equal);
}

#[test]
fn cmp_eq_0327() {
    assert_eq!(app_lib::kvr::compare_versions("5.0.7", "5.0.7"), Ordering::Equal);
}

#[test]
fn cmp_eq_0328() {
    assert_eq!(app_lib::kvr::compare_versions("5.1.0", "5.1.0"), Ordering::Equal);
}

#[test]
fn cmp_eq_0329() {
    assert_eq!(app_lib::kvr::compare_versions("5.1.1", "5.1.1"), Ordering::Equal);
}

#[test]
fn cmp_eq_0330() {
    assert_eq!(app_lib::kvr::compare_versions("5.1.2", "5.1.2"), Ordering::Equal);
}

#[test]
fn cmp_eq_0331() {
    assert_eq!(app_lib::kvr::compare_versions("5.1.3", "5.1.3"), Ordering::Equal);
}

#[test]
fn cmp_eq_0332() {
    assert_eq!(app_lib::kvr::compare_versions("5.1.4", "5.1.4"), Ordering::Equal);
}

#[test]
fn cmp_eq_0333() {
    assert_eq!(app_lib::kvr::compare_versions("5.1.5", "5.1.5"), Ordering::Equal);
}

#[test]
fn cmp_eq_0334() {
    assert_eq!(app_lib::kvr::compare_versions("5.1.6", "5.1.6"), Ordering::Equal);
}

#[test]
fn cmp_eq_0335() {
    assert_eq!(app_lib::kvr::compare_versions("5.1.7", "5.1.7"), Ordering::Equal);
}

#[test]
fn cmp_eq_0336() {
    assert_eq!(app_lib::kvr::compare_versions("5.2.0", "5.2.0"), Ordering::Equal);
}

#[test]
fn cmp_eq_0337() {
    assert_eq!(app_lib::kvr::compare_versions("5.2.1", "5.2.1"), Ordering::Equal);
}

#[test]
fn cmp_eq_0338() {
    assert_eq!(app_lib::kvr::compare_versions("5.2.2", "5.2.2"), Ordering::Equal);
}

#[test]
fn cmp_eq_0339() {
    assert_eq!(app_lib::kvr::compare_versions("5.2.3", "5.2.3"), Ordering::Equal);
}

#[test]
fn cmp_eq_0340() {
    assert_eq!(app_lib::kvr::compare_versions("5.2.4", "5.2.4"), Ordering::Equal);
}

#[test]
fn cmp_eq_0341() {
    assert_eq!(app_lib::kvr::compare_versions("5.2.5", "5.2.5"), Ordering::Equal);
}

#[test]
fn cmp_eq_0342() {
    assert_eq!(app_lib::kvr::compare_versions("5.2.6", "5.2.6"), Ordering::Equal);
}

#[test]
fn cmp_eq_0343() {
    assert_eq!(app_lib::kvr::compare_versions("5.2.7", "5.2.7"), Ordering::Equal);
}

#[test]
fn cmp_eq_0344() {
    assert_eq!(app_lib::kvr::compare_versions("5.3.0", "5.3.0"), Ordering::Equal);
}

#[test]
fn cmp_eq_0345() {
    assert_eq!(app_lib::kvr::compare_versions("5.3.1", "5.3.1"), Ordering::Equal);
}

#[test]
fn cmp_eq_0346() {
    assert_eq!(app_lib::kvr::compare_versions("5.3.2", "5.3.2"), Ordering::Equal);
}

#[test]
fn cmp_eq_0347() {
    assert_eq!(app_lib::kvr::compare_versions("5.3.3", "5.3.3"), Ordering::Equal);
}

#[test]
fn cmp_eq_0348() {
    assert_eq!(app_lib::kvr::compare_versions("5.3.4", "5.3.4"), Ordering::Equal);
}

#[test]
fn cmp_eq_0349() {
    assert_eq!(app_lib::kvr::compare_versions("5.3.5", "5.3.5"), Ordering::Equal);
}

#[test]
fn cmp_eq_0350() {
    assert_eq!(app_lib::kvr::compare_versions("5.3.6", "5.3.6"), Ordering::Equal);
}

#[test]
fn cmp_eq_0351() {
    assert_eq!(app_lib::kvr::compare_versions("5.3.7", "5.3.7"), Ordering::Equal);
}

#[test]
fn cmp_eq_0352() {
    assert_eq!(app_lib::kvr::compare_versions("5.4.0", "5.4.0"), Ordering::Equal);
}

#[test]
fn cmp_eq_0353() {
    assert_eq!(app_lib::kvr::compare_versions("5.4.1", "5.4.1"), Ordering::Equal);
}

#[test]
fn cmp_eq_0354() {
    assert_eq!(app_lib::kvr::compare_versions("5.4.2", "5.4.2"), Ordering::Equal);
}

#[test]
fn cmp_eq_0355() {
    assert_eq!(app_lib::kvr::compare_versions("5.4.3", "5.4.3"), Ordering::Equal);
}

#[test]
fn cmp_eq_0356() {
    assert_eq!(app_lib::kvr::compare_versions("5.4.4", "5.4.4"), Ordering::Equal);
}

#[test]
fn cmp_eq_0357() {
    assert_eq!(app_lib::kvr::compare_versions("5.4.5", "5.4.5"), Ordering::Equal);
}

#[test]
fn cmp_eq_0358() {
    assert_eq!(app_lib::kvr::compare_versions("5.4.6", "5.4.6"), Ordering::Equal);
}

#[test]
fn cmp_eq_0359() {
    assert_eq!(app_lib::kvr::compare_versions("5.4.7", "5.4.7"), Ordering::Equal);
}

#[test]
fn cmp_eq_0360() {
    assert_eq!(app_lib::kvr::compare_versions("5.5.0", "5.5.0"), Ordering::Equal);
}

#[test]
fn cmp_eq_0361() {
    assert_eq!(app_lib::kvr::compare_versions("5.5.1", "5.5.1"), Ordering::Equal);
}

#[test]
fn cmp_eq_0362() {
    assert_eq!(app_lib::kvr::compare_versions("5.5.2", "5.5.2"), Ordering::Equal);
}

#[test]
fn cmp_eq_0363() {
    assert_eq!(app_lib::kvr::compare_versions("5.5.3", "5.5.3"), Ordering::Equal);
}

#[test]
fn cmp_eq_0364() {
    assert_eq!(app_lib::kvr::compare_versions("5.5.4", "5.5.4"), Ordering::Equal);
}

#[test]
fn cmp_eq_0365() {
    assert_eq!(app_lib::kvr::compare_versions("5.5.5", "5.5.5"), Ordering::Equal);
}

#[test]
fn cmp_eq_0366() {
    assert_eq!(app_lib::kvr::compare_versions("5.5.6", "5.5.6"), Ordering::Equal);
}

#[test]
fn cmp_eq_0367() {
    assert_eq!(app_lib::kvr::compare_versions("5.5.7", "5.5.7"), Ordering::Equal);
}

#[test]
fn cmp_eq_0368() {
    assert_eq!(app_lib::kvr::compare_versions("5.6.0", "5.6.0"), Ordering::Equal);
}

#[test]
fn cmp_eq_0369() {
    assert_eq!(app_lib::kvr::compare_versions("5.6.1", "5.6.1"), Ordering::Equal);
}

#[test]
fn cmp_eq_0370() {
    assert_eq!(app_lib::kvr::compare_versions("5.6.2", "5.6.2"), Ordering::Equal);
}

#[test]
fn cmp_eq_0371() {
    assert_eq!(app_lib::kvr::compare_versions("5.6.3", "5.6.3"), Ordering::Equal);
}

#[test]
fn cmp_eq_0372() {
    assert_eq!(app_lib::kvr::compare_versions("5.6.4", "5.6.4"), Ordering::Equal);
}

#[test]
fn cmp_eq_0373() {
    assert_eq!(app_lib::kvr::compare_versions("5.6.5", "5.6.5"), Ordering::Equal);
}

#[test]
fn cmp_eq_0374() {
    assert_eq!(app_lib::kvr::compare_versions("5.6.6", "5.6.6"), Ordering::Equal);
}

#[test]
fn cmp_eq_0375() {
    assert_eq!(app_lib::kvr::compare_versions("5.6.7", "5.6.7"), Ordering::Equal);
}

#[test]
fn cmp_eq_0376() {
    assert_eq!(app_lib::kvr::compare_versions("5.7.0", "5.7.0"), Ordering::Equal);
}

#[test]
fn cmp_eq_0377() {
    assert_eq!(app_lib::kvr::compare_versions("5.7.1", "5.7.1"), Ordering::Equal);
}

#[test]
fn cmp_eq_0378() {
    assert_eq!(app_lib::kvr::compare_versions("5.7.2", "5.7.2"), Ordering::Equal);
}

#[test]
fn cmp_eq_0379() {
    assert_eq!(app_lib::kvr::compare_versions("5.7.3", "5.7.3"), Ordering::Equal);
}

#[test]
fn cmp_eq_0380() {
    assert_eq!(app_lib::kvr::compare_versions("5.7.4", "5.7.4"), Ordering::Equal);
}

#[test]
fn cmp_eq_0381() {
    assert_eq!(app_lib::kvr::compare_versions("5.7.5", "5.7.5"), Ordering::Equal);
}

#[test]
fn cmp_eq_0382() {
    assert_eq!(app_lib::kvr::compare_versions("5.7.6", "5.7.6"), Ordering::Equal);
}

#[test]
fn cmp_eq_0383() {
    assert_eq!(app_lib::kvr::compare_versions("5.7.7", "5.7.7"), Ordering::Equal);
}

#[test]
fn cmp_eq_0384() {
    assert_eq!(app_lib::kvr::compare_versions("6.0.0", "6.0.0"), Ordering::Equal);
}

#[test]
fn cmp_eq_0385() {
    assert_eq!(app_lib::kvr::compare_versions("6.0.1", "6.0.1"), Ordering::Equal);
}

#[test]
fn cmp_eq_0386() {
    assert_eq!(app_lib::kvr::compare_versions("6.0.2", "6.0.2"), Ordering::Equal);
}

#[test]
fn cmp_eq_0387() {
    assert_eq!(app_lib::kvr::compare_versions("6.0.3", "6.0.3"), Ordering::Equal);
}

#[test]
fn cmp_eq_0388() {
    assert_eq!(app_lib::kvr::compare_versions("6.0.4", "6.0.4"), Ordering::Equal);
}

#[test]
fn cmp_eq_0389() {
    assert_eq!(app_lib::kvr::compare_versions("6.0.5", "6.0.5"), Ordering::Equal);
}

#[test]
fn cmp_eq_0390() {
    assert_eq!(app_lib::kvr::compare_versions("6.0.6", "6.0.6"), Ordering::Equal);
}

#[test]
fn cmp_eq_0391() {
    assert_eq!(app_lib::kvr::compare_versions("6.0.7", "6.0.7"), Ordering::Equal);
}

#[test]
fn cmp_eq_0392() {
    assert_eq!(app_lib::kvr::compare_versions("6.1.0", "6.1.0"), Ordering::Equal);
}

#[test]
fn cmp_eq_0393() {
    assert_eq!(app_lib::kvr::compare_versions("6.1.1", "6.1.1"), Ordering::Equal);
}

#[test]
fn cmp_eq_0394() {
    assert_eq!(app_lib::kvr::compare_versions("6.1.2", "6.1.2"), Ordering::Equal);
}

#[test]
fn cmp_eq_0395() {
    assert_eq!(app_lib::kvr::compare_versions("6.1.3", "6.1.3"), Ordering::Equal);
}

#[test]
fn cmp_eq_0396() {
    assert_eq!(app_lib::kvr::compare_versions("6.1.4", "6.1.4"), Ordering::Equal);
}

#[test]
fn cmp_eq_0397() {
    assert_eq!(app_lib::kvr::compare_versions("6.1.5", "6.1.5"), Ordering::Equal);
}

#[test]
fn cmp_eq_0398() {
    assert_eq!(app_lib::kvr::compare_versions("6.1.6", "6.1.6"), Ordering::Equal);
}

#[test]
fn cmp_eq_0399() {
    assert_eq!(app_lib::kvr::compare_versions("6.1.7", "6.1.7"), Ordering::Equal);
}

#[test]
fn cmp_eq_0400() {
    assert_eq!(app_lib::kvr::compare_versions("6.2.0", "6.2.0"), Ordering::Equal);
}

#[test]
fn cmp_eq_0401() {
    assert_eq!(app_lib::kvr::compare_versions("6.2.1", "6.2.1"), Ordering::Equal);
}

#[test]
fn cmp_eq_0402() {
    assert_eq!(app_lib::kvr::compare_versions("6.2.2", "6.2.2"), Ordering::Equal);
}

#[test]
fn cmp_eq_0403() {
    assert_eq!(app_lib::kvr::compare_versions("6.2.3", "6.2.3"), Ordering::Equal);
}

#[test]
fn cmp_eq_0404() {
    assert_eq!(app_lib::kvr::compare_versions("6.2.4", "6.2.4"), Ordering::Equal);
}

#[test]
fn cmp_eq_0405() {
    assert_eq!(app_lib::kvr::compare_versions("6.2.5", "6.2.5"), Ordering::Equal);
}

#[test]
fn cmp_eq_0406() {
    assert_eq!(app_lib::kvr::compare_versions("6.2.6", "6.2.6"), Ordering::Equal);
}

#[test]
fn cmp_eq_0407() {
    assert_eq!(app_lib::kvr::compare_versions("6.2.7", "6.2.7"), Ordering::Equal);
}

#[test]
fn cmp_eq_0408() {
    assert_eq!(app_lib::kvr::compare_versions("6.3.0", "6.3.0"), Ordering::Equal);
}

#[test]
fn cmp_eq_0409() {
    assert_eq!(app_lib::kvr::compare_versions("6.3.1", "6.3.1"), Ordering::Equal);
}

#[test]
fn cmp_eq_0410() {
    assert_eq!(app_lib::kvr::compare_versions("6.3.2", "6.3.2"), Ordering::Equal);
}

#[test]
fn cmp_eq_0411() {
    assert_eq!(app_lib::kvr::compare_versions("6.3.3", "6.3.3"), Ordering::Equal);
}

#[test]
fn cmp_eq_0412() {
    assert_eq!(app_lib::kvr::compare_versions("6.3.4", "6.3.4"), Ordering::Equal);
}

#[test]
fn cmp_eq_0413() {
    assert_eq!(app_lib::kvr::compare_versions("6.3.5", "6.3.5"), Ordering::Equal);
}

#[test]
fn cmp_eq_0414() {
    assert_eq!(app_lib::kvr::compare_versions("6.3.6", "6.3.6"), Ordering::Equal);
}

#[test]
fn cmp_eq_0415() {
    assert_eq!(app_lib::kvr::compare_versions("6.3.7", "6.3.7"), Ordering::Equal);
}

#[test]
fn cmp_eq_0416() {
    assert_eq!(app_lib::kvr::compare_versions("6.4.0", "6.4.0"), Ordering::Equal);
}

#[test]
fn cmp_eq_0417() {
    assert_eq!(app_lib::kvr::compare_versions("6.4.1", "6.4.1"), Ordering::Equal);
}

#[test]
fn cmp_eq_0418() {
    assert_eq!(app_lib::kvr::compare_versions("6.4.2", "6.4.2"), Ordering::Equal);
}

#[test]
fn cmp_eq_0419() {
    assert_eq!(app_lib::kvr::compare_versions("6.4.3", "6.4.3"), Ordering::Equal);
}

#[test]
fn cmp_eq_0420() {
    assert_eq!(app_lib::kvr::compare_versions("6.4.4", "6.4.4"), Ordering::Equal);
}

#[test]
fn cmp_eq_0421() {
    assert_eq!(app_lib::kvr::compare_versions("6.4.5", "6.4.5"), Ordering::Equal);
}

#[test]
fn cmp_eq_0422() {
    assert_eq!(app_lib::kvr::compare_versions("6.4.6", "6.4.6"), Ordering::Equal);
}

#[test]
fn cmp_eq_0423() {
    assert_eq!(app_lib::kvr::compare_versions("6.4.7", "6.4.7"), Ordering::Equal);
}

#[test]
fn cmp_eq_0424() {
    assert_eq!(app_lib::kvr::compare_versions("6.5.0", "6.5.0"), Ordering::Equal);
}

#[test]
fn cmp_eq_0425() {
    assert_eq!(app_lib::kvr::compare_versions("6.5.1", "6.5.1"), Ordering::Equal);
}

#[test]
fn cmp_eq_0426() {
    assert_eq!(app_lib::kvr::compare_versions("6.5.2", "6.5.2"), Ordering::Equal);
}

#[test]
fn cmp_eq_0427() {
    assert_eq!(app_lib::kvr::compare_versions("6.5.3", "6.5.3"), Ordering::Equal);
}

#[test]
fn cmp_eq_0428() {
    assert_eq!(app_lib::kvr::compare_versions("6.5.4", "6.5.4"), Ordering::Equal);
}

#[test]
fn cmp_eq_0429() {
    assert_eq!(app_lib::kvr::compare_versions("6.5.5", "6.5.5"), Ordering::Equal);
}

#[test]
fn cmp_eq_0430() {
    assert_eq!(app_lib::kvr::compare_versions("6.5.6", "6.5.6"), Ordering::Equal);
}

#[test]
fn cmp_eq_0431() {
    assert_eq!(app_lib::kvr::compare_versions("6.5.7", "6.5.7"), Ordering::Equal);
}

#[test]
fn cmp_eq_0432() {
    assert_eq!(app_lib::kvr::compare_versions("6.6.0", "6.6.0"), Ordering::Equal);
}

#[test]
fn cmp_eq_0433() {
    assert_eq!(app_lib::kvr::compare_versions("6.6.1", "6.6.1"), Ordering::Equal);
}

#[test]
fn cmp_eq_0434() {
    assert_eq!(app_lib::kvr::compare_versions("6.6.2", "6.6.2"), Ordering::Equal);
}

#[test]
fn cmp_eq_0435() {
    assert_eq!(app_lib::kvr::compare_versions("6.6.3", "6.6.3"), Ordering::Equal);
}

#[test]
fn cmp_eq_0436() {
    assert_eq!(app_lib::kvr::compare_versions("6.6.4", "6.6.4"), Ordering::Equal);
}

#[test]
fn cmp_eq_0437() {
    assert_eq!(app_lib::kvr::compare_versions("6.6.5", "6.6.5"), Ordering::Equal);
}

#[test]
fn cmp_eq_0438() {
    assert_eq!(app_lib::kvr::compare_versions("6.6.6", "6.6.6"), Ordering::Equal);
}

#[test]
fn cmp_eq_0439() {
    assert_eq!(app_lib::kvr::compare_versions("6.6.7", "6.6.7"), Ordering::Equal);
}

#[test]
fn cmp_eq_0440() {
    assert_eq!(app_lib::kvr::compare_versions("6.7.0", "6.7.0"), Ordering::Equal);
}

#[test]
fn cmp_eq_0441() {
    assert_eq!(app_lib::kvr::compare_versions("6.7.1", "6.7.1"), Ordering::Equal);
}

#[test]
fn cmp_eq_0442() {
    assert_eq!(app_lib::kvr::compare_versions("6.7.2", "6.7.2"), Ordering::Equal);
}

#[test]
fn cmp_eq_0443() {
    assert_eq!(app_lib::kvr::compare_versions("6.7.3", "6.7.3"), Ordering::Equal);
}

#[test]
fn cmp_eq_0444() {
    assert_eq!(app_lib::kvr::compare_versions("6.7.4", "6.7.4"), Ordering::Equal);
}

#[test]
fn cmp_eq_0445() {
    assert_eq!(app_lib::kvr::compare_versions("6.7.5", "6.7.5"), Ordering::Equal);
}

#[test]
fn cmp_eq_0446() {
    assert_eq!(app_lib::kvr::compare_versions("6.7.6", "6.7.6"), Ordering::Equal);
}

#[test]
fn cmp_eq_0447() {
    assert_eq!(app_lib::kvr::compare_versions("6.7.7", "6.7.7"), Ordering::Equal);
}

#[test]
fn cmp_eq_0448() {
    assert_eq!(app_lib::kvr::compare_versions("7.0.0", "7.0.0"), Ordering::Equal);
}

#[test]
fn cmp_eq_0449() {
    assert_eq!(app_lib::kvr::compare_versions("7.0.1", "7.0.1"), Ordering::Equal);
}

#[test]
fn cmp_eq_0450() {
    assert_eq!(app_lib::kvr::compare_versions("7.0.2", "7.0.2"), Ordering::Equal);
}

#[test]
fn cmp_eq_0451() {
    assert_eq!(app_lib::kvr::compare_versions("7.0.3", "7.0.3"), Ordering::Equal);
}

#[test]
fn cmp_eq_0452() {
    assert_eq!(app_lib::kvr::compare_versions("7.0.4", "7.0.4"), Ordering::Equal);
}

#[test]
fn cmp_eq_0453() {
    assert_eq!(app_lib::kvr::compare_versions("7.0.5", "7.0.5"), Ordering::Equal);
}

#[test]
fn cmp_eq_0454() {
    assert_eq!(app_lib::kvr::compare_versions("7.0.6", "7.0.6"), Ordering::Equal);
}

#[test]
fn cmp_eq_0455() {
    assert_eq!(app_lib::kvr::compare_versions("7.0.7", "7.0.7"), Ordering::Equal);
}

#[test]
fn cmp_eq_0456() {
    assert_eq!(app_lib::kvr::compare_versions("7.1.0", "7.1.0"), Ordering::Equal);
}

#[test]
fn cmp_eq_0457() {
    assert_eq!(app_lib::kvr::compare_versions("7.1.1", "7.1.1"), Ordering::Equal);
}

#[test]
fn cmp_eq_0458() {
    assert_eq!(app_lib::kvr::compare_versions("7.1.2", "7.1.2"), Ordering::Equal);
}

#[test]
fn cmp_eq_0459() {
    assert_eq!(app_lib::kvr::compare_versions("7.1.3", "7.1.3"), Ordering::Equal);
}

#[test]
fn cmp_eq_0460() {
    assert_eq!(app_lib::kvr::compare_versions("7.1.4", "7.1.4"), Ordering::Equal);
}

#[test]
fn cmp_eq_0461() {
    assert_eq!(app_lib::kvr::compare_versions("7.1.5", "7.1.5"), Ordering::Equal);
}

#[test]
fn cmp_eq_0462() {
    assert_eq!(app_lib::kvr::compare_versions("7.1.6", "7.1.6"), Ordering::Equal);
}

#[test]
fn cmp_eq_0463() {
    assert_eq!(app_lib::kvr::compare_versions("7.1.7", "7.1.7"), Ordering::Equal);
}

#[test]
fn cmp_eq_0464() {
    assert_eq!(app_lib::kvr::compare_versions("7.2.0", "7.2.0"), Ordering::Equal);
}

#[test]
fn cmp_eq_0465() {
    assert_eq!(app_lib::kvr::compare_versions("7.2.1", "7.2.1"), Ordering::Equal);
}

#[test]
fn cmp_eq_0466() {
    assert_eq!(app_lib::kvr::compare_versions("7.2.2", "7.2.2"), Ordering::Equal);
}

#[test]
fn cmp_eq_0467() {
    assert_eq!(app_lib::kvr::compare_versions("7.2.3", "7.2.3"), Ordering::Equal);
}

#[test]
fn cmp_eq_0468() {
    assert_eq!(app_lib::kvr::compare_versions("7.2.4", "7.2.4"), Ordering::Equal);
}

#[test]
fn cmp_eq_0469() {
    assert_eq!(app_lib::kvr::compare_versions("7.2.5", "7.2.5"), Ordering::Equal);
}

#[test]
fn cmp_eq_0470() {
    assert_eq!(app_lib::kvr::compare_versions("7.2.6", "7.2.6"), Ordering::Equal);
}

#[test]
fn cmp_eq_0471() {
    assert_eq!(app_lib::kvr::compare_versions("7.2.7", "7.2.7"), Ordering::Equal);
}

#[test]
fn cmp_eq_0472() {
    assert_eq!(app_lib::kvr::compare_versions("7.3.0", "7.3.0"), Ordering::Equal);
}

#[test]
fn cmp_eq_0473() {
    assert_eq!(app_lib::kvr::compare_versions("7.3.1", "7.3.1"), Ordering::Equal);
}

#[test]
fn cmp_eq_0474() {
    assert_eq!(app_lib::kvr::compare_versions("7.3.2", "7.3.2"), Ordering::Equal);
}

#[test]
fn cmp_eq_0475() {
    assert_eq!(app_lib::kvr::compare_versions("7.3.3", "7.3.3"), Ordering::Equal);
}

#[test]
fn cmp_eq_0476() {
    assert_eq!(app_lib::kvr::compare_versions("7.3.4", "7.3.4"), Ordering::Equal);
}

#[test]
fn cmp_eq_0477() {
    assert_eq!(app_lib::kvr::compare_versions("7.3.5", "7.3.5"), Ordering::Equal);
}

#[test]
fn cmp_eq_0478() {
    assert_eq!(app_lib::kvr::compare_versions("7.3.6", "7.3.6"), Ordering::Equal);
}

#[test]
fn cmp_eq_0479() {
    assert_eq!(app_lib::kvr::compare_versions("7.3.7", "7.3.7"), Ordering::Equal);
}

#[test]
fn cmp_eq_0480() {
    assert_eq!(app_lib::kvr::compare_versions("7.4.0", "7.4.0"), Ordering::Equal);
}

#[test]
fn cmp_eq_0481() {
    assert_eq!(app_lib::kvr::compare_versions("7.4.1", "7.4.1"), Ordering::Equal);
}

#[test]
fn cmp_eq_0482() {
    assert_eq!(app_lib::kvr::compare_versions("7.4.2", "7.4.2"), Ordering::Equal);
}

#[test]
fn cmp_eq_0483() {
    assert_eq!(app_lib::kvr::compare_versions("7.4.3", "7.4.3"), Ordering::Equal);
}

#[test]
fn cmp_eq_0484() {
    assert_eq!(app_lib::kvr::compare_versions("7.4.4", "7.4.4"), Ordering::Equal);
}

#[test]
fn cmp_eq_0485() {
    assert_eq!(app_lib::kvr::compare_versions("7.4.5", "7.4.5"), Ordering::Equal);
}

#[test]
fn cmp_eq_0486() {
    assert_eq!(app_lib::kvr::compare_versions("7.4.6", "7.4.6"), Ordering::Equal);
}

#[test]
fn cmp_eq_0487() {
    assert_eq!(app_lib::kvr::compare_versions("7.4.7", "7.4.7"), Ordering::Equal);
}

#[test]
fn cmp_eq_0488() {
    assert_eq!(app_lib::kvr::compare_versions("7.5.0", "7.5.0"), Ordering::Equal);
}

#[test]
fn cmp_eq_0489() {
    assert_eq!(app_lib::kvr::compare_versions("7.5.1", "7.5.1"), Ordering::Equal);
}

#[test]
fn cmp_eq_0490() {
    assert_eq!(app_lib::kvr::compare_versions("7.5.2", "7.5.2"), Ordering::Equal);
}

#[test]
fn cmp_eq_0491() {
    assert_eq!(app_lib::kvr::compare_versions("7.5.3", "7.5.3"), Ordering::Equal);
}

#[test]
fn cmp_eq_0492() {
    assert_eq!(app_lib::kvr::compare_versions("7.5.4", "7.5.4"), Ordering::Equal);
}

#[test]
fn cmp_eq_0493() {
    assert_eq!(app_lib::kvr::compare_versions("7.5.5", "7.5.5"), Ordering::Equal);
}

#[test]
fn cmp_eq_0494() {
    assert_eq!(app_lib::kvr::compare_versions("7.5.6", "7.5.6"), Ordering::Equal);
}

#[test]
fn cmp_eq_0495() {
    assert_eq!(app_lib::kvr::compare_versions("7.5.7", "7.5.7"), Ordering::Equal);
}

#[test]
fn cmp_eq_0496() {
    assert_eq!(app_lib::kvr::compare_versions("7.6.0", "7.6.0"), Ordering::Equal);
}

#[test]
fn cmp_eq_0497() {
    assert_eq!(app_lib::kvr::compare_versions("7.6.1", "7.6.1"), Ordering::Equal);
}

#[test]
fn cmp_eq_0498() {
    assert_eq!(app_lib::kvr::compare_versions("7.6.2", "7.6.2"), Ordering::Equal);
}

#[test]
fn cmp_eq_0499() {
    assert_eq!(app_lib::kvr::compare_versions("7.6.3", "7.6.3"), Ordering::Equal);
}

#[test]
fn cmp_eq_0500() {
    assert_eq!(app_lib::kvr::compare_versions("7.6.4", "7.6.4"), Ordering::Equal);
}

#[test]
fn cmp_eq_0501() {
    assert_eq!(app_lib::kvr::compare_versions("7.6.5", "7.6.5"), Ordering::Equal);
}

#[test]
fn cmp_eq_0502() {
    assert_eq!(app_lib::kvr::compare_versions("7.6.6", "7.6.6"), Ordering::Equal);
}

#[test]
fn cmp_eq_0503() {
    assert_eq!(app_lib::kvr::compare_versions("7.6.7", "7.6.7"), Ordering::Equal);
}

#[test]
fn cmp_eq_0504() {
    assert_eq!(app_lib::kvr::compare_versions("7.7.0", "7.7.0"), Ordering::Equal);
}

#[test]
fn cmp_eq_0505() {
    assert_eq!(app_lib::kvr::compare_versions("7.7.1", "7.7.1"), Ordering::Equal);
}

#[test]
fn cmp_eq_0506() {
    assert_eq!(app_lib::kvr::compare_versions("7.7.2", "7.7.2"), Ordering::Equal);
}

#[test]
fn cmp_eq_0507() {
    assert_eq!(app_lib::kvr::compare_versions("7.7.3", "7.7.3"), Ordering::Equal);
}

#[test]
fn cmp_eq_0508() {
    assert_eq!(app_lib::kvr::compare_versions("7.7.4", "7.7.4"), Ordering::Equal);
}

#[test]
fn cmp_eq_0509() {
    assert_eq!(app_lib::kvr::compare_versions("7.7.5", "7.7.5"), Ordering::Equal);
}

#[test]
fn cmp_eq_0510() {
    assert_eq!(app_lib::kvr::compare_versions("7.7.6", "7.7.6"), Ordering::Equal);
}

#[test]
fn cmp_eq_0511() {
    assert_eq!(app_lib::kvr::compare_versions("7.7.7", "7.7.7"), Ordering::Equal);
}

#[test]
fn cmp_sp_1p0p9_1p0p10() {
    assert_eq!(app_lib::kvr::compare_versions("1.0.9", "1.0.10"), Ordering::Less);
}

#[test]
fn cmp_sp_1p0p10_1p0p9() {
    assert_eq!(app_lib::kvr::compare_versions("1.0.10", "1.0.9"), Ordering::Greater);
}

#[test]
fn cmp_sp_2p0p0_1p99p99() {
    assert_eq!(app_lib::kvr::compare_versions("2.0.0", "1.99.99"), Ordering::Greater);
}

#[test]
fn cmp_sp_2p0_2p0p0p0() {
    assert_eq!(app_lib::kvr::compare_versions("2.0", "2.0.0.0"), Ordering::Equal);
}

#[test]
fn cmp_sp_10p0_9p0() {
    assert_eq!(app_lib::kvr::compare_versions("10.0", "9.0"), Ordering::Greater);
}

// ── format_size ────────────────────────────────────────────────────

#[test]
fn fmt_zero() {
    assert_eq!(app_lib::format_size(0u64), "0 B");
}

#[test]
fn fmt_pow2_0() {
    assert_eq!(app_lib::format_size(1u64), "1.0 B");
}

#[test]
fn fmt_pow2_1() {
    assert_eq!(app_lib::format_size(2u64), "2.0 B");
}

#[test]
fn fmt_pow2_2() {
    assert_eq!(app_lib::format_size(4u64), "4.0 B");
}

#[test]
fn fmt_pow2_3() {
    assert_eq!(app_lib::format_size(8u64), "8.0 B");
}

#[test]
fn fmt_pow2_4() {
    assert_eq!(app_lib::format_size(16u64), "16.0 B");
}

#[test]
fn fmt_pow2_5() {
    assert_eq!(app_lib::format_size(32u64), "32.0 B");
}

#[test]
fn fmt_pow2_6() {
    assert_eq!(app_lib::format_size(64u64), "64.0 B");
}

#[test]
fn fmt_pow2_7() {
    assert_eq!(app_lib::format_size(128u64), "128.0 B");
}

#[test]
fn fmt_pow2_8() {
    assert_eq!(app_lib::format_size(256u64), "256.0 B");
}

#[test]
fn fmt_pow2_9() {
    assert_eq!(app_lib::format_size(512u64), "512.0 B");
}

#[test]
fn fmt_pow2_10() {
    assert_eq!(app_lib::format_size(1024u64), "1.0 KB");
}

#[test]
fn fmt_pow2_11() {
    assert_eq!(app_lib::format_size(2048u64), "2.0 KB");
}

#[test]
fn fmt_pow2_12() {
    assert_eq!(app_lib::format_size(4096u64), "4.0 KB");
}

#[test]
fn fmt_pow2_13() {
    assert_eq!(app_lib::format_size(8192u64), "8.0 KB");
}

#[test]
fn fmt_pow2_14() {
    assert_eq!(app_lib::format_size(16384u64), "16.0 KB");
}

#[test]
fn fmt_pow2_15() {
    assert_eq!(app_lib::format_size(32768u64), "32.0 KB");
}

#[test]
fn fmt_pow2_16() {
    assert_eq!(app_lib::format_size(65536u64), "64.0 KB");
}

#[test]
fn fmt_pow2_17() {
    assert_eq!(app_lib::format_size(131072u64), "128.0 KB");
}

#[test]
fn fmt_pow2_18() {
    assert_eq!(app_lib::format_size(262144u64), "256.0 KB");
}

#[test]
fn fmt_pow2_19() {
    assert_eq!(app_lib::format_size(524288u64), "512.0 KB");
}

#[test]
fn fmt_pow2_20() {
    assert_eq!(app_lib::format_size(1048576u64), "1.0 MB");
}

#[test]
fn fmt_pow2_21() {
    assert_eq!(app_lib::format_size(2097152u64), "2.0 MB");
}

#[test]
fn fmt_pow2_22() {
    assert_eq!(app_lib::format_size(4194304u64), "4.0 MB");
}

#[test]
fn fmt_pow2_23() {
    assert_eq!(app_lib::format_size(8388608u64), "8.0 MB");
}

#[test]
fn fmt_pow2_24() {
    assert_eq!(app_lib::format_size(16777216u64), "16.0 MB");
}

#[test]
fn fmt_pow2_25() {
    assert_eq!(app_lib::format_size(33554432u64), "32.0 MB");
}

#[test]
fn fmt_pow2_26() {
    assert_eq!(app_lib::format_size(67108864u64), "64.0 MB");
}

#[test]
fn fmt_pow2_27() {
    assert_eq!(app_lib::format_size(134217728u64), "128.0 MB");
}

#[test]
fn fmt_pow2_28() {
    assert_eq!(app_lib::format_size(268435456u64), "256.0 MB");
}

#[test]
fn fmt_pow2_29() {
    assert_eq!(app_lib::format_size(536870912u64), "512.0 MB");
}

#[test]
fn fmt_pow2_30() {
    assert_eq!(app_lib::format_size(1073741824u64), "1.0 GB");
}

#[test]
fn fmt_pow2_31() {
    assert_eq!(app_lib::format_size(2147483648u64), "2.0 GB");
}

#[test]
fn fmt_pow2_32() {
    assert_eq!(app_lib::format_size(4294967296u64), "4.0 GB");
}

#[test]
fn fmt_pow2_33() {
    assert_eq!(app_lib::format_size(8589934592u64), "8.0 GB");
}

#[test]
fn fmt_pow2_34() {
    assert_eq!(app_lib::format_size(17179869184u64), "16.0 GB");
}

#[test]
fn fmt_pow2_35() {
    assert_eq!(app_lib::format_size(34359738368u64), "32.0 GB");
}

#[test]
fn fmt_pow2_36() {
    assert_eq!(app_lib::format_size(68719476736u64), "64.0 GB");
}

#[test]
fn fmt_pow2_37() {
    assert_eq!(app_lib::format_size(137438953472u64), "128.0 GB");
}

#[test]
fn fmt_pow2_38() {
    assert_eq!(app_lib::format_size(274877906944u64), "256.0 GB");
}

#[test]
fn fmt_pow2_39() {
    assert_eq!(app_lib::format_size(549755813888u64), "512.0 GB");
}

#[test]
fn fmt_pow2_40() {
    assert_eq!(app_lib::format_size(1099511627776u64), "1.0 TB");
}

#[test]
fn fmt_u_0001() {
    assert_eq!(app_lib::format_size(1u64), "1.0 B");
}

#[test]
fn fmt_u_0002() {
    assert_eq!(app_lib::format_size(2u64), "2.0 B");
}

#[test]
fn fmt_u_0003() {
    assert_eq!(app_lib::format_size(3u64), "3.0 B");
}

#[test]
fn fmt_u_0004() {
    assert_eq!(app_lib::format_size(4u64), "4.0 B");
}

#[test]
fn fmt_u_0005() {
    assert_eq!(app_lib::format_size(5u64), "5.0 B");
}

#[test]
fn fmt_u_0006() {
    assert_eq!(app_lib::format_size(6u64), "6.0 B");
}

#[test]
fn fmt_u_0007() {
    assert_eq!(app_lib::format_size(7u64), "7.0 B");
}

#[test]
fn fmt_u_0008() {
    assert_eq!(app_lib::format_size(8u64), "8.0 B");
}

#[test]
fn fmt_u_0009() {
    assert_eq!(app_lib::format_size(9u64), "9.0 B");
}

#[test]
fn fmt_u_0010() {
    assert_eq!(app_lib::format_size(10u64), "10.0 B");
}

#[test]
fn fmt_u_0011() {
    assert_eq!(app_lib::format_size(11u64), "11.0 B");
}

#[test]
fn fmt_u_0012() {
    assert_eq!(app_lib::format_size(12u64), "12.0 B");
}

#[test]
fn fmt_u_0013() {
    assert_eq!(app_lib::format_size(13u64), "13.0 B");
}

#[test]
fn fmt_u_0014() {
    assert_eq!(app_lib::format_size(14u64), "14.0 B");
}

#[test]
fn fmt_u_0015() {
    assert_eq!(app_lib::format_size(15u64), "15.0 B");
}

#[test]
fn fmt_u_0016() {
    assert_eq!(app_lib::format_size(16u64), "16.0 B");
}

#[test]
fn fmt_u_0017() {
    assert_eq!(app_lib::format_size(17u64), "17.0 B");
}

#[test]
fn fmt_u_0018() {
    assert_eq!(app_lib::format_size(18u64), "18.0 B");
}

#[test]
fn fmt_u_0019() {
    assert_eq!(app_lib::format_size(19u64), "19.0 B");
}

#[test]
fn fmt_u_0020() {
    assert_eq!(app_lib::format_size(20u64), "20.0 B");
}

#[test]
fn fmt_u_0021() {
    assert_eq!(app_lib::format_size(21u64), "21.0 B");
}

#[test]
fn fmt_u_0022() {
    assert_eq!(app_lib::format_size(22u64), "22.0 B");
}

#[test]
fn fmt_u_0023() {
    assert_eq!(app_lib::format_size(23u64), "23.0 B");
}

#[test]
fn fmt_u_0024() {
    assert_eq!(app_lib::format_size(24u64), "24.0 B");
}

#[test]
fn fmt_u_0025() {
    assert_eq!(app_lib::format_size(25u64), "25.0 B");
}

#[test]
fn fmt_u_0026() {
    assert_eq!(app_lib::format_size(26u64), "26.0 B");
}

#[test]
fn fmt_u_0027() {
    assert_eq!(app_lib::format_size(27u64), "27.0 B");
}

#[test]
fn fmt_u_0028() {
    assert_eq!(app_lib::format_size(28u64), "28.0 B");
}

#[test]
fn fmt_u_0029() {
    assert_eq!(app_lib::format_size(29u64), "29.0 B");
}

#[test]
fn fmt_u_0030() {
    assert_eq!(app_lib::format_size(30u64), "30.0 B");
}

#[test]
fn fmt_u_0031() {
    assert_eq!(app_lib::format_size(31u64), "31.0 B");
}

#[test]
fn fmt_u_0032() {
    assert_eq!(app_lib::format_size(32u64), "32.0 B");
}

#[test]
fn fmt_u_0033() {
    assert_eq!(app_lib::format_size(33u64), "33.0 B");
}

#[test]
fn fmt_u_0034() {
    assert_eq!(app_lib::format_size(34u64), "34.0 B");
}

#[test]
fn fmt_u_0035() {
    assert_eq!(app_lib::format_size(35u64), "35.0 B");
}

#[test]
fn fmt_u_0036() {
    assert_eq!(app_lib::format_size(36u64), "36.0 B");
}

#[test]
fn fmt_u_0037() {
    assert_eq!(app_lib::format_size(37u64), "37.0 B");
}

#[test]
fn fmt_u_0038() {
    assert_eq!(app_lib::format_size(38u64), "38.0 B");
}

#[test]
fn fmt_u_0039() {
    assert_eq!(app_lib::format_size(39u64), "39.0 B");
}

#[test]
fn fmt_u_0040() {
    assert_eq!(app_lib::format_size(40u64), "40.0 B");
}

#[test]
fn fmt_u_0041() {
    assert_eq!(app_lib::format_size(41u64), "41.0 B");
}

#[test]
fn fmt_u_0042() {
    assert_eq!(app_lib::format_size(42u64), "42.0 B");
}

#[test]
fn fmt_u_0043() {
    assert_eq!(app_lib::format_size(43u64), "43.0 B");
}

#[test]
fn fmt_u_0044() {
    assert_eq!(app_lib::format_size(44u64), "44.0 B");
}

#[test]
fn fmt_u_0045() {
    assert_eq!(app_lib::format_size(45u64), "45.0 B");
}

#[test]
fn fmt_u_0046() {
    assert_eq!(app_lib::format_size(46u64), "46.0 B");
}

#[test]
fn fmt_u_0047() {
    assert_eq!(app_lib::format_size(47u64), "47.0 B");
}

#[test]
fn fmt_u_0048() {
    assert_eq!(app_lib::format_size(48u64), "48.0 B");
}

#[test]
fn fmt_u_0049() {
    assert_eq!(app_lib::format_size(49u64), "49.0 B");
}

#[test]
fn fmt_u_0050() {
    assert_eq!(app_lib::format_size(50u64), "50.0 B");
}

#[test]
fn fmt_u_0051() {
    assert_eq!(app_lib::format_size(51u64), "51.0 B");
}

#[test]
fn fmt_u_0052() {
    assert_eq!(app_lib::format_size(52u64), "52.0 B");
}

#[test]
fn fmt_u_0053() {
    assert_eq!(app_lib::format_size(53u64), "53.0 B");
}

#[test]
fn fmt_u_0054() {
    assert_eq!(app_lib::format_size(54u64), "54.0 B");
}

#[test]
fn fmt_u_0055() {
    assert_eq!(app_lib::format_size(55u64), "55.0 B");
}

#[test]
fn fmt_u_0056() {
    assert_eq!(app_lib::format_size(56u64), "56.0 B");
}

#[test]
fn fmt_u_0057() {
    assert_eq!(app_lib::format_size(57u64), "57.0 B");
}

#[test]
fn fmt_u_0058() {
    assert_eq!(app_lib::format_size(58u64), "58.0 B");
}

#[test]
fn fmt_u_0059() {
    assert_eq!(app_lib::format_size(59u64), "59.0 B");
}

#[test]
fn fmt_u_0060() {
    assert_eq!(app_lib::format_size(60u64), "60.0 B");
}

#[test]
fn fmt_u_0061() {
    assert_eq!(app_lib::format_size(61u64), "61.0 B");
}

#[test]
fn fmt_u_0062() {
    assert_eq!(app_lib::format_size(62u64), "62.0 B");
}

#[test]
fn fmt_u_0063() {
    assert_eq!(app_lib::format_size(63u64), "63.0 B");
}

#[test]
fn fmt_u_0064() {
    assert_eq!(app_lib::format_size(64u64), "64.0 B");
}

#[test]
fn fmt_u_0065() {
    assert_eq!(app_lib::format_size(65u64), "65.0 B");
}

#[test]
fn fmt_u_0066() {
    assert_eq!(app_lib::format_size(66u64), "66.0 B");
}

#[test]
fn fmt_u_0067() {
    assert_eq!(app_lib::format_size(67u64), "67.0 B");
}

#[test]
fn fmt_u_0068() {
    assert_eq!(app_lib::format_size(68u64), "68.0 B");
}

#[test]
fn fmt_u_0069() {
    assert_eq!(app_lib::format_size(69u64), "69.0 B");
}

#[test]
fn fmt_u_0070() {
    assert_eq!(app_lib::format_size(70u64), "70.0 B");
}

#[test]
fn fmt_u_0071() {
    assert_eq!(app_lib::format_size(71u64), "71.0 B");
}

#[test]
fn fmt_u_0072() {
    assert_eq!(app_lib::format_size(72u64), "72.0 B");
}

#[test]
fn fmt_u_0073() {
    assert_eq!(app_lib::format_size(73u64), "73.0 B");
}

#[test]
fn fmt_u_0074() {
    assert_eq!(app_lib::format_size(74u64), "74.0 B");
}

#[test]
fn fmt_u_0075() {
    assert_eq!(app_lib::format_size(75u64), "75.0 B");
}

#[test]
fn fmt_u_0076() {
    assert_eq!(app_lib::format_size(76u64), "76.0 B");
}

#[test]
fn fmt_u_0077() {
    assert_eq!(app_lib::format_size(77u64), "77.0 B");
}

#[test]
fn fmt_u_0078() {
    assert_eq!(app_lib::format_size(78u64), "78.0 B");
}

#[test]
fn fmt_u_0079() {
    assert_eq!(app_lib::format_size(79u64), "79.0 B");
}

#[test]
fn fmt_u_0080() {
    assert_eq!(app_lib::format_size(80u64), "80.0 B");
}

#[test]
fn fmt_u_0081() {
    assert_eq!(app_lib::format_size(81u64), "81.0 B");
}

#[test]
fn fmt_u_0082() {
    assert_eq!(app_lib::format_size(82u64), "82.0 B");
}

#[test]
fn fmt_u_0083() {
    assert_eq!(app_lib::format_size(83u64), "83.0 B");
}

#[test]
fn fmt_u_0084() {
    assert_eq!(app_lib::format_size(84u64), "84.0 B");
}

#[test]
fn fmt_u_0085() {
    assert_eq!(app_lib::format_size(85u64), "85.0 B");
}

#[test]
fn fmt_u_0086() {
    assert_eq!(app_lib::format_size(86u64), "86.0 B");
}

#[test]
fn fmt_u_0087() {
    assert_eq!(app_lib::format_size(87u64), "87.0 B");
}

#[test]
fn fmt_u_0088() {
    assert_eq!(app_lib::format_size(88u64), "88.0 B");
}

#[test]
fn fmt_u_0089() {
    assert_eq!(app_lib::format_size(89u64), "89.0 B");
}

#[test]
fn fmt_u_0090() {
    assert_eq!(app_lib::format_size(90u64), "90.0 B");
}

#[test]
fn fmt_u_0091() {
    assert_eq!(app_lib::format_size(91u64), "91.0 B");
}

#[test]
fn fmt_u_0092() {
    assert_eq!(app_lib::format_size(92u64), "92.0 B");
}

#[test]
fn fmt_u_0093() {
    assert_eq!(app_lib::format_size(93u64), "93.0 B");
}

#[test]
fn fmt_u_0094() {
    assert_eq!(app_lib::format_size(94u64), "94.0 B");
}

#[test]
fn fmt_u_0095() {
    assert_eq!(app_lib::format_size(95u64), "95.0 B");
}

#[test]
fn fmt_u_0096() {
    assert_eq!(app_lib::format_size(96u64), "96.0 B");
}

#[test]
fn fmt_u_0097() {
    assert_eq!(app_lib::format_size(97u64), "97.0 B");
}

#[test]
fn fmt_u_0098() {
    assert_eq!(app_lib::format_size(98u64), "98.0 B");
}

#[test]
fn fmt_u_0099() {
    assert_eq!(app_lib::format_size(99u64), "99.0 B");
}

#[test]
fn fmt_u_0100() {
    assert_eq!(app_lib::format_size(100u64), "100.0 B");
}

#[test]
fn fmt_u_0101() {
    assert_eq!(app_lib::format_size(101u64), "101.0 B");
}

#[test]
fn fmt_u_0102() {
    assert_eq!(app_lib::format_size(102u64), "102.0 B");
}

#[test]
fn fmt_u_0103() {
    assert_eq!(app_lib::format_size(103u64), "103.0 B");
}

#[test]
fn fmt_u_0104() {
    assert_eq!(app_lib::format_size(104u64), "104.0 B");
}

#[test]
fn fmt_u_0105() {
    assert_eq!(app_lib::format_size(105u64), "105.0 B");
}

#[test]
fn fmt_u_0106() {
    assert_eq!(app_lib::format_size(106u64), "106.0 B");
}

#[test]
fn fmt_u_0107() {
    assert_eq!(app_lib::format_size(107u64), "107.0 B");
}

#[test]
fn fmt_u_0108() {
    assert_eq!(app_lib::format_size(108u64), "108.0 B");
}

#[test]
fn fmt_u_0109() {
    assert_eq!(app_lib::format_size(109u64), "109.0 B");
}

#[test]
fn fmt_u_0110() {
    assert_eq!(app_lib::format_size(110u64), "110.0 B");
}

#[test]
fn fmt_u_0111() {
    assert_eq!(app_lib::format_size(111u64), "111.0 B");
}

#[test]
fn fmt_u_0112() {
    assert_eq!(app_lib::format_size(112u64), "112.0 B");
}

#[test]
fn fmt_u_0113() {
    assert_eq!(app_lib::format_size(113u64), "113.0 B");
}

#[test]
fn fmt_u_0114() {
    assert_eq!(app_lib::format_size(114u64), "114.0 B");
}

#[test]
fn fmt_u_0115() {
    assert_eq!(app_lib::format_size(115u64), "115.0 B");
}

#[test]
fn fmt_u_0116() {
    assert_eq!(app_lib::format_size(116u64), "116.0 B");
}

#[test]
fn fmt_u_0117() {
    assert_eq!(app_lib::format_size(117u64), "117.0 B");
}

#[test]
fn fmt_u_0118() {
    assert_eq!(app_lib::format_size(118u64), "118.0 B");
}

#[test]
fn fmt_u_0119() {
    assert_eq!(app_lib::format_size(119u64), "119.0 B");
}

#[test]
fn fmt_u_0120() {
    assert_eq!(app_lib::format_size(120u64), "120.0 B");
}

#[test]
fn fmt_u_0121() {
    assert_eq!(app_lib::format_size(121u64), "121.0 B");
}

#[test]
fn fmt_u_0122() {
    assert_eq!(app_lib::format_size(122u64), "122.0 B");
}

#[test]
fn fmt_u_0123() {
    assert_eq!(app_lib::format_size(123u64), "123.0 B");
}

#[test]
fn fmt_u_0124() {
    assert_eq!(app_lib::format_size(124u64), "124.0 B");
}

#[test]
fn fmt_u_0125() {
    assert_eq!(app_lib::format_size(125u64), "125.0 B");
}

#[test]
fn fmt_u_0126() {
    assert_eq!(app_lib::format_size(126u64), "126.0 B");
}

#[test]
fn fmt_u_0127() {
    assert_eq!(app_lib::format_size(127u64), "127.0 B");
}

#[test]
fn fmt_u_0128() {
    assert_eq!(app_lib::format_size(128u64), "128.0 B");
}

#[test]
fn fmt_u_0129() {
    assert_eq!(app_lib::format_size(129u64), "129.0 B");
}

#[test]
fn fmt_u_0130() {
    assert_eq!(app_lib::format_size(130u64), "130.0 B");
}

#[test]
fn fmt_u_0131() {
    assert_eq!(app_lib::format_size(131u64), "131.0 B");
}

#[test]
fn fmt_u_0132() {
    assert_eq!(app_lib::format_size(132u64), "132.0 B");
}

#[test]
fn fmt_u_0133() {
    assert_eq!(app_lib::format_size(133u64), "133.0 B");
}

#[test]
fn fmt_u_0134() {
    assert_eq!(app_lib::format_size(134u64), "134.0 B");
}

#[test]
fn fmt_u_0135() {
    assert_eq!(app_lib::format_size(135u64), "135.0 B");
}

#[test]
fn fmt_u_0136() {
    assert_eq!(app_lib::format_size(136u64), "136.0 B");
}

#[test]
fn fmt_u_0137() {
    assert_eq!(app_lib::format_size(137u64), "137.0 B");
}

#[test]
fn fmt_u_0138() {
    assert_eq!(app_lib::format_size(138u64), "138.0 B");
}

#[test]
fn fmt_u_0139() {
    assert_eq!(app_lib::format_size(139u64), "139.0 B");
}

#[test]
fn fmt_u_0140() {
    assert_eq!(app_lib::format_size(140u64), "140.0 B");
}

#[test]
fn fmt_u_0141() {
    assert_eq!(app_lib::format_size(141u64), "141.0 B");
}

#[test]
fn fmt_u_0142() {
    assert_eq!(app_lib::format_size(142u64), "142.0 B");
}

#[test]
fn fmt_u_0143() {
    assert_eq!(app_lib::format_size(143u64), "143.0 B");
}

#[test]
fn fmt_u_0144() {
    assert_eq!(app_lib::format_size(144u64), "144.0 B");
}

#[test]
fn fmt_u_0145() {
    assert_eq!(app_lib::format_size(145u64), "145.0 B");
}

#[test]
fn fmt_u_0146() {
    assert_eq!(app_lib::format_size(146u64), "146.0 B");
}

#[test]
fn fmt_u_0147() {
    assert_eq!(app_lib::format_size(147u64), "147.0 B");
}

#[test]
fn fmt_u_0148() {
    assert_eq!(app_lib::format_size(148u64), "148.0 B");
}

#[test]
fn fmt_u_0149() {
    assert_eq!(app_lib::format_size(149u64), "149.0 B");
}

#[test]
fn fmt_u_0150() {
    assert_eq!(app_lib::format_size(150u64), "150.0 B");
}

#[test]
fn fmt_u_0151() {
    assert_eq!(app_lib::format_size(151u64), "151.0 B");
}

#[test]
fn fmt_u_0152() {
    assert_eq!(app_lib::format_size(152u64), "152.0 B");
}

#[test]
fn fmt_u_0153() {
    assert_eq!(app_lib::format_size(153u64), "153.0 B");
}

#[test]
fn fmt_u_0154() {
    assert_eq!(app_lib::format_size(154u64), "154.0 B");
}

#[test]
fn fmt_u_0155() {
    assert_eq!(app_lib::format_size(155u64), "155.0 B");
}

#[test]
fn fmt_u_0156() {
    assert_eq!(app_lib::format_size(156u64), "156.0 B");
}

#[test]
fn fmt_u_0157() {
    assert_eq!(app_lib::format_size(157u64), "157.0 B");
}

#[test]
fn fmt_u_0158() {
    assert_eq!(app_lib::format_size(158u64), "158.0 B");
}

#[test]
fn fmt_u_0159() {
    assert_eq!(app_lib::format_size(159u64), "159.0 B");
}

#[test]
fn fmt_u_0160() {
    assert_eq!(app_lib::format_size(160u64), "160.0 B");
}

#[test]
fn fmt_u_0161() {
    assert_eq!(app_lib::format_size(161u64), "161.0 B");
}

#[test]
fn fmt_u_0162() {
    assert_eq!(app_lib::format_size(162u64), "162.0 B");
}

#[test]
fn fmt_u_0163() {
    assert_eq!(app_lib::format_size(163u64), "163.0 B");
}

#[test]
fn fmt_u_0164() {
    assert_eq!(app_lib::format_size(164u64), "164.0 B");
}

#[test]
fn fmt_u_0165() {
    assert_eq!(app_lib::format_size(165u64), "165.0 B");
}

#[test]
fn fmt_u_0166() {
    assert_eq!(app_lib::format_size(166u64), "166.0 B");
}

#[test]
fn fmt_u_0167() {
    assert_eq!(app_lib::format_size(167u64), "167.0 B");
}

#[test]
fn fmt_u_0168() {
    assert_eq!(app_lib::format_size(168u64), "168.0 B");
}

#[test]
fn fmt_u_0169() {
    assert_eq!(app_lib::format_size(169u64), "169.0 B");
}

#[test]
fn fmt_u_0170() {
    assert_eq!(app_lib::format_size(170u64), "170.0 B");
}

#[test]
fn fmt_u_0171() {
    assert_eq!(app_lib::format_size(171u64), "171.0 B");
}

#[test]
fn fmt_u_0172() {
    assert_eq!(app_lib::format_size(172u64), "172.0 B");
}

#[test]
fn fmt_u_0173() {
    assert_eq!(app_lib::format_size(173u64), "173.0 B");
}

#[test]
fn fmt_u_0174() {
    assert_eq!(app_lib::format_size(174u64), "174.0 B");
}

#[test]
fn fmt_u_0175() {
    assert_eq!(app_lib::format_size(175u64), "175.0 B");
}

#[test]
fn fmt_u_0176() {
    assert_eq!(app_lib::format_size(176u64), "176.0 B");
}

#[test]
fn fmt_u_0177() {
    assert_eq!(app_lib::format_size(177u64), "177.0 B");
}

#[test]
fn fmt_u_0178() {
    assert_eq!(app_lib::format_size(178u64), "178.0 B");
}

#[test]
fn fmt_u_0179() {
    assert_eq!(app_lib::format_size(179u64), "179.0 B");
}

#[test]
fn fmt_u_0180() {
    assert_eq!(app_lib::format_size(180u64), "180.0 B");
}

#[test]
fn fmt_u_0181() {
    assert_eq!(app_lib::format_size(181u64), "181.0 B");
}

#[test]
fn fmt_u_0182() {
    assert_eq!(app_lib::format_size(182u64), "182.0 B");
}

#[test]
fn fmt_u_0183() {
    assert_eq!(app_lib::format_size(183u64), "183.0 B");
}

#[test]
fn fmt_u_0184() {
    assert_eq!(app_lib::format_size(184u64), "184.0 B");
}

#[test]
fn fmt_u_0185() {
    assert_eq!(app_lib::format_size(185u64), "185.0 B");
}

#[test]
fn fmt_u_0186() {
    assert_eq!(app_lib::format_size(186u64), "186.0 B");
}

#[test]
fn fmt_u_0187() {
    assert_eq!(app_lib::format_size(187u64), "187.0 B");
}

#[test]
fn fmt_u_0188() {
    assert_eq!(app_lib::format_size(188u64), "188.0 B");
}

#[test]
fn fmt_u_0189() {
    assert_eq!(app_lib::format_size(189u64), "189.0 B");
}

#[test]
fn fmt_u_0190() {
    assert_eq!(app_lib::format_size(190u64), "190.0 B");
}

#[test]
fn fmt_u_0191() {
    assert_eq!(app_lib::format_size(191u64), "191.0 B");
}

#[test]
fn fmt_u_0192() {
    assert_eq!(app_lib::format_size(192u64), "192.0 B");
}

#[test]
fn fmt_u_0193() {
    assert_eq!(app_lib::format_size(193u64), "193.0 B");
}

#[test]
fn fmt_u_0194() {
    assert_eq!(app_lib::format_size(194u64), "194.0 B");
}

#[test]
fn fmt_u_0195() {
    assert_eq!(app_lib::format_size(195u64), "195.0 B");
}

#[test]
fn fmt_u_0196() {
    assert_eq!(app_lib::format_size(196u64), "196.0 B");
}

#[test]
fn fmt_u_0197() {
    assert_eq!(app_lib::format_size(197u64), "197.0 B");
}

#[test]
fn fmt_u_0198() {
    assert_eq!(app_lib::format_size(198u64), "198.0 B");
}

#[test]
fn fmt_u_0199() {
    assert_eq!(app_lib::format_size(199u64), "199.0 B");
}

#[test]
fn fmt_u_0200() {
    assert_eq!(app_lib::format_size(200u64), "200.0 B");
}

#[test]
fn fmt_u_0201() {
    assert_eq!(app_lib::format_size(201u64), "201.0 B");
}

#[test]
fn fmt_u_0202() {
    assert_eq!(app_lib::format_size(202u64), "202.0 B");
}

#[test]
fn fmt_u_0203() {
    assert_eq!(app_lib::format_size(203u64), "203.0 B");
}

#[test]
fn fmt_u_0204() {
    assert_eq!(app_lib::format_size(204u64), "204.0 B");
}

#[test]
fn fmt_u_0205() {
    assert_eq!(app_lib::format_size(205u64), "205.0 B");
}

#[test]
fn fmt_u_0206() {
    assert_eq!(app_lib::format_size(206u64), "206.0 B");
}

#[test]
fn fmt_u_0207() {
    assert_eq!(app_lib::format_size(207u64), "207.0 B");
}

#[test]
fn fmt_u_0208() {
    assert_eq!(app_lib::format_size(208u64), "208.0 B");
}

#[test]
fn fmt_u_0209() {
    assert_eq!(app_lib::format_size(209u64), "209.0 B");
}

#[test]
fn fmt_u_0210() {
    assert_eq!(app_lib::format_size(210u64), "210.0 B");
}

#[test]
fn fmt_u_0211() {
    assert_eq!(app_lib::format_size(211u64), "211.0 B");
}

#[test]
fn fmt_u_0212() {
    assert_eq!(app_lib::format_size(212u64), "212.0 B");
}

#[test]
fn fmt_u_0213() {
    assert_eq!(app_lib::format_size(213u64), "213.0 B");
}

#[test]
fn fmt_u_0214() {
    assert_eq!(app_lib::format_size(214u64), "214.0 B");
}

#[test]
fn fmt_u_0215() {
    assert_eq!(app_lib::format_size(215u64), "215.0 B");
}

#[test]
fn fmt_u_0216() {
    assert_eq!(app_lib::format_size(216u64), "216.0 B");
}

#[test]
fn fmt_u_0217() {
    assert_eq!(app_lib::format_size(217u64), "217.0 B");
}

#[test]
fn fmt_u_0218() {
    assert_eq!(app_lib::format_size(218u64), "218.0 B");
}

#[test]
fn fmt_u_0219() {
    assert_eq!(app_lib::format_size(219u64), "219.0 B");
}

#[test]
fn fmt_u_0220() {
    assert_eq!(app_lib::format_size(220u64), "220.0 B");
}

#[test]
fn fmt_u_0221() {
    assert_eq!(app_lib::format_size(221u64), "221.0 B");
}

#[test]
fn fmt_u_0222() {
    assert_eq!(app_lib::format_size(222u64), "222.0 B");
}

#[test]
fn fmt_u_0223() {
    assert_eq!(app_lib::format_size(223u64), "223.0 B");
}

#[test]
fn fmt_u_0224() {
    assert_eq!(app_lib::format_size(224u64), "224.0 B");
}

#[test]
fn fmt_u_0225() {
    assert_eq!(app_lib::format_size(225u64), "225.0 B");
}

#[test]
fn fmt_u_0226() {
    assert_eq!(app_lib::format_size(226u64), "226.0 B");
}

#[test]
fn fmt_u_0227() {
    assert_eq!(app_lib::format_size(227u64), "227.0 B");
}

#[test]
fn fmt_u_0228() {
    assert_eq!(app_lib::format_size(228u64), "228.0 B");
}

#[test]
fn fmt_u_0229() {
    assert_eq!(app_lib::format_size(229u64), "229.0 B");
}

#[test]
fn fmt_u_0230() {
    assert_eq!(app_lib::format_size(230u64), "230.0 B");
}

#[test]
fn fmt_u_0231() {
    assert_eq!(app_lib::format_size(231u64), "231.0 B");
}

#[test]
fn fmt_u_0232() {
    assert_eq!(app_lib::format_size(232u64), "232.0 B");
}

#[test]
fn fmt_u_0233() {
    assert_eq!(app_lib::format_size(233u64), "233.0 B");
}

#[test]
fn fmt_u_0234() {
    assert_eq!(app_lib::format_size(234u64), "234.0 B");
}

#[test]
fn fmt_u_0235() {
    assert_eq!(app_lib::format_size(235u64), "235.0 B");
}

#[test]
fn fmt_u_0236() {
    assert_eq!(app_lib::format_size(236u64), "236.0 B");
}

#[test]
fn fmt_u_0237() {
    assert_eq!(app_lib::format_size(237u64), "237.0 B");
}

#[test]
fn fmt_u_0238() {
    assert_eq!(app_lib::format_size(238u64), "238.0 B");
}

#[test]
fn fmt_u_0239() {
    assert_eq!(app_lib::format_size(239u64), "239.0 B");
}

#[test]
fn fmt_u_0240() {
    assert_eq!(app_lib::format_size(240u64), "240.0 B");
}

#[test]
fn fmt_u_0241() {
    assert_eq!(app_lib::format_size(241u64), "241.0 B");
}

#[test]
fn fmt_u_0242() {
    assert_eq!(app_lib::format_size(242u64), "242.0 B");
}

#[test]
fn fmt_u_0243() {
    assert_eq!(app_lib::format_size(243u64), "243.0 B");
}

#[test]
fn fmt_u_0244() {
    assert_eq!(app_lib::format_size(244u64), "244.0 B");
}

#[test]
fn fmt_u_0245() {
    assert_eq!(app_lib::format_size(245u64), "245.0 B");
}

#[test]
fn fmt_u_0246() {
    assert_eq!(app_lib::format_size(246u64), "246.0 B");
}

#[test]
fn fmt_u_0247() {
    assert_eq!(app_lib::format_size(247u64), "247.0 B");
}

#[test]
fn fmt_u_0248() {
    assert_eq!(app_lib::format_size(248u64), "248.0 B");
}

#[test]
fn fmt_u_0249() {
    assert_eq!(app_lib::format_size(249u64), "249.0 B");
}

#[test]
fn fmt_u_0250() {
    assert_eq!(app_lib::format_size(250u64), "250.0 B");
}

#[test]
fn fmt_u_0251() {
    assert_eq!(app_lib::format_size(251u64), "251.0 B");
}

#[test]
fn fmt_u_0252() {
    assert_eq!(app_lib::format_size(252u64), "252.0 B");
}

#[test]
fn fmt_u_0253() {
    assert_eq!(app_lib::format_size(253u64), "253.0 B");
}

#[test]
fn fmt_u_0254() {
    assert_eq!(app_lib::format_size(254u64), "254.0 B");
}

#[test]
fn fmt_u_0255() {
    assert_eq!(app_lib::format_size(255u64), "255.0 B");
}

#[test]
fn fmt_u_0256() {
    assert_eq!(app_lib::format_size(256u64), "256.0 B");
}

#[test]
fn fmt_u_0257() {
    assert_eq!(app_lib::format_size(257u64), "257.0 B");
}

#[test]
fn fmt_u_0258() {
    assert_eq!(app_lib::format_size(258u64), "258.0 B");
}

#[test]
fn fmt_u_0259() {
    assert_eq!(app_lib::format_size(259u64), "259.0 B");
}

#[test]
fn fmt_u_0260() {
    assert_eq!(app_lib::format_size(260u64), "260.0 B");
}

#[test]
fn fmt_u_0261() {
    assert_eq!(app_lib::format_size(261u64), "261.0 B");
}

#[test]
fn fmt_u_0262() {
    assert_eq!(app_lib::format_size(262u64), "262.0 B");
}

#[test]
fn fmt_u_0263() {
    assert_eq!(app_lib::format_size(263u64), "263.0 B");
}

#[test]
fn fmt_u_0264() {
    assert_eq!(app_lib::format_size(264u64), "264.0 B");
}

#[test]
fn fmt_u_0265() {
    assert_eq!(app_lib::format_size(265u64), "265.0 B");
}

#[test]
fn fmt_u_0266() {
    assert_eq!(app_lib::format_size(266u64), "266.0 B");
}

#[test]
fn fmt_u_0267() {
    assert_eq!(app_lib::format_size(267u64), "267.0 B");
}

#[test]
fn fmt_u_0268() {
    assert_eq!(app_lib::format_size(268u64), "268.0 B");
}

#[test]
fn fmt_u_0269() {
    assert_eq!(app_lib::format_size(269u64), "269.0 B");
}

#[test]
fn fmt_u_0270() {
    assert_eq!(app_lib::format_size(270u64), "270.0 B");
}

#[test]
fn fmt_u_0271() {
    assert_eq!(app_lib::format_size(271u64), "271.0 B");
}

#[test]
fn fmt_u_0272() {
    assert_eq!(app_lib::format_size(272u64), "272.0 B");
}

#[test]
fn fmt_u_0273() {
    assert_eq!(app_lib::format_size(273u64), "273.0 B");
}

#[test]
fn fmt_u_0274() {
    assert_eq!(app_lib::format_size(274u64), "274.0 B");
}

#[test]
fn fmt_u_0275() {
    assert_eq!(app_lib::format_size(275u64), "275.0 B");
}

#[test]
fn fmt_u_0276() {
    assert_eq!(app_lib::format_size(276u64), "276.0 B");
}

#[test]
fn fmt_u_0277() {
    assert_eq!(app_lib::format_size(277u64), "277.0 B");
}

#[test]
fn fmt_u_0278() {
    assert_eq!(app_lib::format_size(278u64), "278.0 B");
}

#[test]
fn fmt_u_0279() {
    assert_eq!(app_lib::format_size(279u64), "279.0 B");
}

#[test]
fn fmt_u_0280() {
    assert_eq!(app_lib::format_size(280u64), "280.0 B");
}

#[test]
fn fmt_u_0281() {
    assert_eq!(app_lib::format_size(281u64), "281.0 B");
}

#[test]
fn fmt_u_0282() {
    assert_eq!(app_lib::format_size(282u64), "282.0 B");
}

#[test]
fn fmt_u_0283() {
    assert_eq!(app_lib::format_size(283u64), "283.0 B");
}

#[test]
fn fmt_u_0284() {
    assert_eq!(app_lib::format_size(284u64), "284.0 B");
}

#[test]
fn fmt_u_0285() {
    assert_eq!(app_lib::format_size(285u64), "285.0 B");
}

#[test]
fn fmt_u_0286() {
    assert_eq!(app_lib::format_size(286u64), "286.0 B");
}

#[test]
fn fmt_u_0287() {
    assert_eq!(app_lib::format_size(287u64), "287.0 B");
}

#[test]
fn fmt_u_0288() {
    assert_eq!(app_lib::format_size(288u64), "288.0 B");
}

#[test]
fn fmt_u_0289() {
    assert_eq!(app_lib::format_size(289u64), "289.0 B");
}

#[test]
fn fmt_u_0290() {
    assert_eq!(app_lib::format_size(290u64), "290.0 B");
}

#[test]
fn fmt_u_0291() {
    assert_eq!(app_lib::format_size(291u64), "291.0 B");
}

#[test]
fn fmt_u_0292() {
    assert_eq!(app_lib::format_size(292u64), "292.0 B");
}

#[test]
fn fmt_u_0293() {
    assert_eq!(app_lib::format_size(293u64), "293.0 B");
}

#[test]
fn fmt_u_0294() {
    assert_eq!(app_lib::format_size(294u64), "294.0 B");
}

#[test]
fn fmt_u_0295() {
    assert_eq!(app_lib::format_size(295u64), "295.0 B");
}

#[test]
fn fmt_u_0296() {
    assert_eq!(app_lib::format_size(296u64), "296.0 B");
}

#[test]
fn fmt_u_0297() {
    assert_eq!(app_lib::format_size(297u64), "297.0 B");
}

#[test]
fn fmt_u_0298() {
    assert_eq!(app_lib::format_size(298u64), "298.0 B");
}

#[test]
fn fmt_u_0299() {
    assert_eq!(app_lib::format_size(299u64), "299.0 B");
}

#[test]
fn fmt_u_0300() {
    assert_eq!(app_lib::format_size(300u64), "300.0 B");
}

#[test]
fn fmt_u_0301() {
    assert_eq!(app_lib::format_size(301u64), "301.0 B");
}

#[test]
fn fmt_u_0302() {
    assert_eq!(app_lib::format_size(302u64), "302.0 B");
}

#[test]
fn fmt_u_0303() {
    assert_eq!(app_lib::format_size(303u64), "303.0 B");
}

#[test]
fn fmt_u_0304() {
    assert_eq!(app_lib::format_size(304u64), "304.0 B");
}

#[test]
fn fmt_u_0305() {
    assert_eq!(app_lib::format_size(305u64), "305.0 B");
}

#[test]
fn fmt_u_0306() {
    assert_eq!(app_lib::format_size(306u64), "306.0 B");
}

#[test]
fn fmt_u_0307() {
    assert_eq!(app_lib::format_size(307u64), "307.0 B");
}

#[test]
fn fmt_u_0308() {
    assert_eq!(app_lib::format_size(308u64), "308.0 B");
}

#[test]
fn fmt_u_0309() {
    assert_eq!(app_lib::format_size(309u64), "309.0 B");
}

#[test]
fn fmt_u_0310() {
    assert_eq!(app_lib::format_size(310u64), "310.0 B");
}

#[test]
fn fmt_u_0311() {
    assert_eq!(app_lib::format_size(311u64), "311.0 B");
}

#[test]
fn fmt_u_0312() {
    assert_eq!(app_lib::format_size(312u64), "312.0 B");
}

#[test]
fn fmt_u_0313() {
    assert_eq!(app_lib::format_size(313u64), "313.0 B");
}

#[test]
fn fmt_u_0314() {
    assert_eq!(app_lib::format_size(314u64), "314.0 B");
}

#[test]
fn fmt_u_0315() {
    assert_eq!(app_lib::format_size(315u64), "315.0 B");
}

#[test]
fn fmt_u_0316() {
    assert_eq!(app_lib::format_size(316u64), "316.0 B");
}

#[test]
fn fmt_u_0317() {
    assert_eq!(app_lib::format_size(317u64), "317.0 B");
}

#[test]
fn fmt_u_0318() {
    assert_eq!(app_lib::format_size(318u64), "318.0 B");
}

#[test]
fn fmt_u_0319() {
    assert_eq!(app_lib::format_size(319u64), "319.0 B");
}

#[test]
fn fmt_u_0320() {
    assert_eq!(app_lib::format_size(320u64), "320.0 B");
}

#[test]
fn fmt_u_0321() {
    assert_eq!(app_lib::format_size(321u64), "321.0 B");
}

#[test]
fn fmt_u_0322() {
    assert_eq!(app_lib::format_size(322u64), "322.0 B");
}

#[test]
fn fmt_u_0323() {
    assert_eq!(app_lib::format_size(323u64), "323.0 B");
}

#[test]
fn fmt_u_0324() {
    assert_eq!(app_lib::format_size(324u64), "324.0 B");
}

#[test]
fn fmt_u_0325() {
    assert_eq!(app_lib::format_size(325u64), "325.0 B");
}

#[test]
fn fmt_u_0326() {
    assert_eq!(app_lib::format_size(326u64), "326.0 B");
}

#[test]
fn fmt_u_0327() {
    assert_eq!(app_lib::format_size(327u64), "327.0 B");
}

#[test]
fn fmt_u_0328() {
    assert_eq!(app_lib::format_size(328u64), "328.0 B");
}

#[test]
fn fmt_u_0329() {
    assert_eq!(app_lib::format_size(329u64), "329.0 B");
}

#[test]
fn fmt_u_0330() {
    assert_eq!(app_lib::format_size(330u64), "330.0 B");
}

#[test]
fn fmt_u_0331() {
    assert_eq!(app_lib::format_size(331u64), "331.0 B");
}

#[test]
fn fmt_u_0332() {
    assert_eq!(app_lib::format_size(332u64), "332.0 B");
}

#[test]
fn fmt_u_0333() {
    assert_eq!(app_lib::format_size(333u64), "333.0 B");
}

#[test]
fn fmt_u_0334() {
    assert_eq!(app_lib::format_size(334u64), "334.0 B");
}

#[test]
fn fmt_u_0335() {
    assert_eq!(app_lib::format_size(335u64), "335.0 B");
}

#[test]
fn fmt_u_0336() {
    assert_eq!(app_lib::format_size(336u64), "336.0 B");
}

#[test]
fn fmt_u_0337() {
    assert_eq!(app_lib::format_size(337u64), "337.0 B");
}

#[test]
fn fmt_u_0338() {
    assert_eq!(app_lib::format_size(338u64), "338.0 B");
}

#[test]
fn fmt_u_0339() {
    assert_eq!(app_lib::format_size(339u64), "339.0 B");
}

#[test]
fn fmt_u_0340() {
    assert_eq!(app_lib::format_size(340u64), "340.0 B");
}

#[test]
fn fmt_u_0341() {
    assert_eq!(app_lib::format_size(341u64), "341.0 B");
}

#[test]
fn fmt_u_0342() {
    assert_eq!(app_lib::format_size(342u64), "342.0 B");
}

#[test]
fn fmt_u_0343() {
    assert_eq!(app_lib::format_size(343u64), "343.0 B");
}

#[test]
fn fmt_u_0344() {
    assert_eq!(app_lib::format_size(344u64), "344.0 B");
}

#[test]
fn fmt_u_0345() {
    assert_eq!(app_lib::format_size(345u64), "345.0 B");
}

#[test]
fn fmt_u_0346() {
    assert_eq!(app_lib::format_size(346u64), "346.0 B");
}

#[test]
fn fmt_u_0347() {
    assert_eq!(app_lib::format_size(347u64), "347.0 B");
}

#[test]
fn fmt_u_0348() {
    assert_eq!(app_lib::format_size(348u64), "348.0 B");
}

#[test]
fn fmt_u_0349() {
    assert_eq!(app_lib::format_size(349u64), "349.0 B");
}

#[test]
fn fmt_u_0350() {
    assert_eq!(app_lib::format_size(350u64), "350.0 B");
}

#[test]
fn fmt_u_0351() {
    assert_eq!(app_lib::format_size(351u64), "351.0 B");
}

#[test]
fn fmt_u_0352() {
    assert_eq!(app_lib::format_size(352u64), "352.0 B");
}

#[test]
fn fmt_u_0353() {
    assert_eq!(app_lib::format_size(353u64), "353.0 B");
}

#[test]
fn fmt_u_0354() {
    assert_eq!(app_lib::format_size(354u64), "354.0 B");
}

#[test]
fn fmt_u_0355() {
    assert_eq!(app_lib::format_size(355u64), "355.0 B");
}

#[test]
fn fmt_u_0356() {
    assert_eq!(app_lib::format_size(356u64), "356.0 B");
}

#[test]
fn fmt_u_0357() {
    assert_eq!(app_lib::format_size(357u64), "357.0 B");
}

#[test]
fn fmt_u_0358() {
    assert_eq!(app_lib::format_size(358u64), "358.0 B");
}

#[test]
fn fmt_u_0359() {
    assert_eq!(app_lib::format_size(359u64), "359.0 B");
}

#[test]
fn fmt_u_0360() {
    assert_eq!(app_lib::format_size(360u64), "360.0 B");
}

#[test]
fn fmt_u_0361() {
    assert_eq!(app_lib::format_size(361u64), "361.0 B");
}

#[test]
fn fmt_u_0362() {
    assert_eq!(app_lib::format_size(362u64), "362.0 B");
}

#[test]
fn fmt_u_0363() {
    assert_eq!(app_lib::format_size(363u64), "363.0 B");
}

#[test]
fn fmt_u_0364() {
    assert_eq!(app_lib::format_size(364u64), "364.0 B");
}

#[test]
fn fmt_u_0365() {
    assert_eq!(app_lib::format_size(365u64), "365.0 B");
}

#[test]
fn fmt_u_0366() {
    assert_eq!(app_lib::format_size(366u64), "366.0 B");
}

#[test]
fn fmt_u_0367() {
    assert_eq!(app_lib::format_size(367u64), "367.0 B");
}

#[test]
fn fmt_u_0368() {
    assert_eq!(app_lib::format_size(368u64), "368.0 B");
}

#[test]
fn fmt_u_0369() {
    assert_eq!(app_lib::format_size(369u64), "369.0 B");
}

#[test]
fn fmt_u_0370() {
    assert_eq!(app_lib::format_size(370u64), "370.0 B");
}

#[test]
fn fmt_u_0371() {
    assert_eq!(app_lib::format_size(371u64), "371.0 B");
}

#[test]
fn fmt_u_0372() {
    assert_eq!(app_lib::format_size(372u64), "372.0 B");
}

#[test]
fn fmt_u_0373() {
    assert_eq!(app_lib::format_size(373u64), "373.0 B");
}

#[test]
fn fmt_u_0374() {
    assert_eq!(app_lib::format_size(374u64), "374.0 B");
}

#[test]
fn fmt_u_0375() {
    assert_eq!(app_lib::format_size(375u64), "375.0 B");
}

#[test]
fn fmt_u_0376() {
    assert_eq!(app_lib::format_size(376u64), "376.0 B");
}

#[test]
fn fmt_u_0377() {
    assert_eq!(app_lib::format_size(377u64), "377.0 B");
}

#[test]
fn fmt_u_0378() {
    assert_eq!(app_lib::format_size(378u64), "378.0 B");
}

#[test]
fn fmt_u_0379() {
    assert_eq!(app_lib::format_size(379u64), "379.0 B");
}

#[test]
fn fmt_u_0380() {
    assert_eq!(app_lib::format_size(380u64), "380.0 B");
}

#[test]
fn fmt_u_0381() {
    assert_eq!(app_lib::format_size(381u64), "381.0 B");
}

#[test]
fn fmt_u_0382() {
    assert_eq!(app_lib::format_size(382u64), "382.0 B");
}

#[test]
fn fmt_u_0383() {
    assert_eq!(app_lib::format_size(383u64), "383.0 B");
}

#[test]
fn fmt_u_0384() {
    assert_eq!(app_lib::format_size(384u64), "384.0 B");
}

#[test]
fn fmt_u_0385() {
    assert_eq!(app_lib::format_size(385u64), "385.0 B");
}

#[test]
fn fmt_u_0386() {
    assert_eq!(app_lib::format_size(386u64), "386.0 B");
}

#[test]
fn fmt_u_0387() {
    assert_eq!(app_lib::format_size(387u64), "387.0 B");
}

#[test]
fn fmt_u_0388() {
    assert_eq!(app_lib::format_size(388u64), "388.0 B");
}

#[test]
fn fmt_u_0389() {
    assert_eq!(app_lib::format_size(389u64), "389.0 B");
}

#[test]
fn fmt_u_0390() {
    assert_eq!(app_lib::format_size(390u64), "390.0 B");
}

#[test]
fn fmt_u_0391() {
    assert_eq!(app_lib::format_size(391u64), "391.0 B");
}

#[test]
fn fmt_u_0392() {
    assert_eq!(app_lib::format_size(392u64), "392.0 B");
}

#[test]
fn fmt_u_0393() {
    assert_eq!(app_lib::format_size(393u64), "393.0 B");
}

#[test]
fn fmt_u_0394() {
    assert_eq!(app_lib::format_size(394u64), "394.0 B");
}

#[test]
fn fmt_u_0395() {
    assert_eq!(app_lib::format_size(395u64), "395.0 B");
}

#[test]
fn fmt_u_0396() {
    assert_eq!(app_lib::format_size(396u64), "396.0 B");
}

#[test]
fn fmt_u_0397() {
    assert_eq!(app_lib::format_size(397u64), "397.0 B");
}

#[test]
fn fmt_u_0398() {
    assert_eq!(app_lib::format_size(398u64), "398.0 B");
}

#[test]
fn fmt_u_0399() {
    assert_eq!(app_lib::format_size(399u64), "399.0 B");
}

#[test]
fn fmt_u_0400() {
    assert_eq!(app_lib::format_size(400u64), "400.0 B");
}

#[test]
fn fmt_u_0401() {
    assert_eq!(app_lib::format_size(401u64), "401.0 B");
}

#[test]
fn fmt_u_0402() {
    assert_eq!(app_lib::format_size(402u64), "402.0 B");
}

#[test]
fn fmt_u_0403() {
    assert_eq!(app_lib::format_size(403u64), "403.0 B");
}

#[test]
fn fmt_u_0404() {
    assert_eq!(app_lib::format_size(404u64), "404.0 B");
}

#[test]
fn fmt_u_0405() {
    assert_eq!(app_lib::format_size(405u64), "405.0 B");
}

#[test]
fn fmt_u_0406() {
    assert_eq!(app_lib::format_size(406u64), "406.0 B");
}

#[test]
fn fmt_u_0407() {
    assert_eq!(app_lib::format_size(407u64), "407.0 B");
}

#[test]
fn fmt_u_0408() {
    assert_eq!(app_lib::format_size(408u64), "408.0 B");
}

#[test]
fn fmt_u_0409() {
    assert_eq!(app_lib::format_size(409u64), "409.0 B");
}

#[test]
fn fmt_u_0410() {
    assert_eq!(app_lib::format_size(410u64), "410.0 B");
}

#[test]
fn fmt_u_0411() {
    assert_eq!(app_lib::format_size(411u64), "411.0 B");
}

#[test]
fn fmt_u_0412() {
    assert_eq!(app_lib::format_size(412u64), "412.0 B");
}

#[test]
fn fmt_u_0413() {
    assert_eq!(app_lib::format_size(413u64), "413.0 B");
}

#[test]
fn fmt_u_0414() {
    assert_eq!(app_lib::format_size(414u64), "414.0 B");
}

#[test]
fn fmt_u_0415() {
    assert_eq!(app_lib::format_size(415u64), "415.0 B");
}

#[test]
fn fmt_u_0416() {
    assert_eq!(app_lib::format_size(416u64), "416.0 B");
}

#[test]
fn fmt_u_0417() {
    assert_eq!(app_lib::format_size(417u64), "417.0 B");
}

#[test]
fn fmt_u_0418() {
    assert_eq!(app_lib::format_size(418u64), "418.0 B");
}

#[test]
fn fmt_u_0419() {
    assert_eq!(app_lib::format_size(419u64), "419.0 B");
}

#[test]
fn fmt_u_0420() {
    assert_eq!(app_lib::format_size(420u64), "420.0 B");
}

#[test]
fn fmt_u_0421() {
    assert_eq!(app_lib::format_size(421u64), "421.0 B");
}

#[test]
fn fmt_u_0422() {
    assert_eq!(app_lib::format_size(422u64), "422.0 B");
}

#[test]
fn fmt_u_0423() {
    assert_eq!(app_lib::format_size(423u64), "423.0 B");
}

#[test]
fn fmt_u_0424() {
    assert_eq!(app_lib::format_size(424u64), "424.0 B");
}

#[test]
fn fmt_u_0425() {
    assert_eq!(app_lib::format_size(425u64), "425.0 B");
}

#[test]
fn fmt_u_0426() {
    assert_eq!(app_lib::format_size(426u64), "426.0 B");
}

#[test]
fn fmt_u_0427() {
    assert_eq!(app_lib::format_size(427u64), "427.0 B");
}

#[test]
fn fmt_u_0428() {
    assert_eq!(app_lib::format_size(428u64), "428.0 B");
}

#[test]
fn fmt_u_0429() {
    assert_eq!(app_lib::format_size(429u64), "429.0 B");
}

#[test]
fn fmt_u_0430() {
    assert_eq!(app_lib::format_size(430u64), "430.0 B");
}

#[test]
fn fmt_u_0431() {
    assert_eq!(app_lib::format_size(431u64), "431.0 B");
}

#[test]
fn fmt_u_0432() {
    assert_eq!(app_lib::format_size(432u64), "432.0 B");
}

#[test]
fn fmt_u_0433() {
    assert_eq!(app_lib::format_size(433u64), "433.0 B");
}

#[test]
fn fmt_u_0434() {
    assert_eq!(app_lib::format_size(434u64), "434.0 B");
}

#[test]
fn fmt_u_0435() {
    assert_eq!(app_lib::format_size(435u64), "435.0 B");
}

#[test]
fn fmt_u_0436() {
    assert_eq!(app_lib::format_size(436u64), "436.0 B");
}

#[test]
fn fmt_u_0437() {
    assert_eq!(app_lib::format_size(437u64), "437.0 B");
}

#[test]
fn fmt_u_0438() {
    assert_eq!(app_lib::format_size(438u64), "438.0 B");
}

#[test]
fn fmt_u_0439() {
    assert_eq!(app_lib::format_size(439u64), "439.0 B");
}

#[test]
fn fmt_u_0440() {
    assert_eq!(app_lib::format_size(440u64), "440.0 B");
}

#[test]
fn fmt_u_0441() {
    assert_eq!(app_lib::format_size(441u64), "441.0 B");
}

#[test]
fn fmt_u_0442() {
    assert_eq!(app_lib::format_size(442u64), "442.0 B");
}

#[test]
fn fmt_u_0443() {
    assert_eq!(app_lib::format_size(443u64), "443.0 B");
}

#[test]
fn fmt_u_0444() {
    assert_eq!(app_lib::format_size(444u64), "444.0 B");
}

#[test]
fn fmt_u_0445() {
    assert_eq!(app_lib::format_size(445u64), "445.0 B");
}

#[test]
fn fmt_u_0446() {
    assert_eq!(app_lib::format_size(446u64), "446.0 B");
}

#[test]
fn fmt_u_0447() {
    assert_eq!(app_lib::format_size(447u64), "447.0 B");
}

#[test]
fn fmt_u_0448() {
    assert_eq!(app_lib::format_size(448u64), "448.0 B");
}

#[test]
fn fmt_u_0449() {
    assert_eq!(app_lib::format_size(449u64), "449.0 B");
}

#[test]
fn fmt_u_0450() {
    assert_eq!(app_lib::format_size(450u64), "450.0 B");
}

#[test]
fn fmt_u_0451() {
    assert_eq!(app_lib::format_size(451u64), "451.0 B");
}

#[test]
fn fmt_u_0452() {
    assert_eq!(app_lib::format_size(452u64), "452.0 B");
}

#[test]
fn fmt_u_0453() {
    assert_eq!(app_lib::format_size(453u64), "453.0 B");
}

#[test]
fn fmt_u_0454() {
    assert_eq!(app_lib::format_size(454u64), "454.0 B");
}

#[test]
fn fmt_u_0455() {
    assert_eq!(app_lib::format_size(455u64), "455.0 B");
}

#[test]
fn fmt_u_0456() {
    assert_eq!(app_lib::format_size(456u64), "456.0 B");
}

#[test]
fn fmt_u_0457() {
    assert_eq!(app_lib::format_size(457u64), "457.0 B");
}

#[test]
fn fmt_u_0458() {
    assert_eq!(app_lib::format_size(458u64), "458.0 B");
}

#[test]
fn fmt_u_0459() {
    assert_eq!(app_lib::format_size(459u64), "459.0 B");
}

#[test]
fn fmt_u_0460() {
    assert_eq!(app_lib::format_size(460u64), "460.0 B");
}

#[test]
fn fmt_u_0461() {
    assert_eq!(app_lib::format_size(461u64), "461.0 B");
}

#[test]
fn fmt_u_0462() {
    assert_eq!(app_lib::format_size(462u64), "462.0 B");
}

#[test]
fn fmt_u_0463() {
    assert_eq!(app_lib::format_size(463u64), "463.0 B");
}

#[test]
fn fmt_u_0464() {
    assert_eq!(app_lib::format_size(464u64), "464.0 B");
}

#[test]
fn fmt_u_0465() {
    assert_eq!(app_lib::format_size(465u64), "465.0 B");
}

#[test]
fn fmt_u_0466() {
    assert_eq!(app_lib::format_size(466u64), "466.0 B");
}

#[test]
fn fmt_u_0467() {
    assert_eq!(app_lib::format_size(467u64), "467.0 B");
}

#[test]
fn fmt_u_0468() {
    assert_eq!(app_lib::format_size(468u64), "468.0 B");
}

#[test]
fn fmt_u_0469() {
    assert_eq!(app_lib::format_size(469u64), "469.0 B");
}

#[test]
fn fmt_u_0470() {
    assert_eq!(app_lib::format_size(470u64), "470.0 B");
}

#[test]
fn fmt_u_0471() {
    assert_eq!(app_lib::format_size(471u64), "471.0 B");
}

#[test]
fn fmt_u_0472() {
    assert_eq!(app_lib::format_size(472u64), "472.0 B");
}

#[test]
fn fmt_u_0473() {
    assert_eq!(app_lib::format_size(473u64), "473.0 B");
}

#[test]
fn fmt_u_0474() {
    assert_eq!(app_lib::format_size(474u64), "474.0 B");
}

#[test]
fn fmt_u_0475() {
    assert_eq!(app_lib::format_size(475u64), "475.0 B");
}

#[test]
fn fmt_u_0476() {
    assert_eq!(app_lib::format_size(476u64), "476.0 B");
}

#[test]
fn fmt_u_0477() {
    assert_eq!(app_lib::format_size(477u64), "477.0 B");
}

#[test]
fn fmt_u_0478() {
    assert_eq!(app_lib::format_size(478u64), "478.0 B");
}

#[test]
fn fmt_u_0479() {
    assert_eq!(app_lib::format_size(479u64), "479.0 B");
}

#[test]
fn fmt_u_0480() {
    assert_eq!(app_lib::format_size(480u64), "480.0 B");
}

#[test]
fn fmt_u_0481() {
    assert_eq!(app_lib::format_size(481u64), "481.0 B");
}

#[test]
fn fmt_u_0482() {
    assert_eq!(app_lib::format_size(482u64), "482.0 B");
}

#[test]
fn fmt_u_0483() {
    assert_eq!(app_lib::format_size(483u64), "483.0 B");
}

#[test]
fn fmt_u_0484() {
    assert_eq!(app_lib::format_size(484u64), "484.0 B");
}

#[test]
fn fmt_u_0485() {
    assert_eq!(app_lib::format_size(485u64), "485.0 B");
}

#[test]
fn fmt_u_0486() {
    assert_eq!(app_lib::format_size(486u64), "486.0 B");
}

#[test]
fn fmt_u_0487() {
    assert_eq!(app_lib::format_size(487u64), "487.0 B");
}

#[test]
fn fmt_u_0488() {
    assert_eq!(app_lib::format_size(488u64), "488.0 B");
}

#[test]
fn fmt_u_0489() {
    assert_eq!(app_lib::format_size(489u64), "489.0 B");
}

#[test]
fn fmt_u_0490() {
    assert_eq!(app_lib::format_size(490u64), "490.0 B");
}

#[test]
fn fmt_u_0491() {
    assert_eq!(app_lib::format_size(491u64), "491.0 B");
}

#[test]
fn fmt_u_0492() {
    assert_eq!(app_lib::format_size(492u64), "492.0 B");
}

#[test]
fn fmt_u_0493() {
    assert_eq!(app_lib::format_size(493u64), "493.0 B");
}

#[test]
fn fmt_u_0494() {
    assert_eq!(app_lib::format_size(494u64), "494.0 B");
}

#[test]
fn fmt_u_0495() {
    assert_eq!(app_lib::format_size(495u64), "495.0 B");
}

#[test]
fn fmt_u_0496() {
    assert_eq!(app_lib::format_size(496u64), "496.0 B");
}

#[test]
fn fmt_u_0497() {
    assert_eq!(app_lib::format_size(497u64), "497.0 B");
}

#[test]
fn fmt_u_0498() {
    assert_eq!(app_lib::format_size(498u64), "498.0 B");
}

#[test]
fn fmt_u_0499() {
    assert_eq!(app_lib::format_size(499u64), "499.0 B");
}

#[test]
fn fmt_u_0500() {
    assert_eq!(app_lib::format_size(500u64), "500.0 B");
}

#[test]
fn fmt_u_0501() {
    assert_eq!(app_lib::format_size(501u64), "501.0 B");
}

#[test]
fn fmt_u_0502() {
    assert_eq!(app_lib::format_size(502u64), "502.0 B");
}

#[test]
fn fmt_u_0503() {
    assert_eq!(app_lib::format_size(503u64), "503.0 B");
}

#[test]
fn fmt_u_0504() {
    assert_eq!(app_lib::format_size(504u64), "504.0 B");
}

#[test]
fn fmt_u_0505() {
    assert_eq!(app_lib::format_size(505u64), "505.0 B");
}

#[test]
fn fmt_u_0506() {
    assert_eq!(app_lib::format_size(506u64), "506.0 B");
}

#[test]
fn fmt_u_0507() {
    assert_eq!(app_lib::format_size(507u64), "507.0 B");
}

#[test]
fn fmt_u_0508() {
    assert_eq!(app_lib::format_size(508u64), "508.0 B");
}

#[test]
fn fmt_u_0509() {
    assert_eq!(app_lib::format_size(509u64), "509.0 B");
}

#[test]
fn fmt_u_0510() {
    assert_eq!(app_lib::format_size(510u64), "510.0 B");
}

#[test]
fn fmt_u_0511() {
    assert_eq!(app_lib::format_size(511u64), "511.0 B");
}

#[test]
fn fmt_u_0512() {
    assert_eq!(app_lib::format_size(512u64), "512.0 B");
}

#[test]
fn fmt_mb_1047552() {
    assert_eq!(app_lib::format_size(1047552u64), "1023.0 KB");
}

#[test]
fn fmt_mb_1047553() {
    assert_eq!(app_lib::format_size(1047553u64), "1023.0 KB");
}

#[test]
fn fmt_mb_1047554() {
    assert_eq!(app_lib::format_size(1047554u64), "1023.0 KB");
}

#[test]
fn fmt_mb_1047555() {
    assert_eq!(app_lib::format_size(1047555u64), "1023.0 KB");
}

#[test]
fn fmt_mb_1047556() {
    assert_eq!(app_lib::format_size(1047556u64), "1023.0 KB");
}

#[test]
fn fmt_mb_1047557() {
    assert_eq!(app_lib::format_size(1047557u64), "1023.0 KB");
}

#[test]
fn fmt_mb_1047558() {
    assert_eq!(app_lib::format_size(1047558u64), "1023.0 KB");
}

#[test]
fn fmt_mb_1047559() {
    assert_eq!(app_lib::format_size(1047559u64), "1023.0 KB");
}

#[test]
fn fmt_mb_1047560() {
    assert_eq!(app_lib::format_size(1047560u64), "1023.0 KB");
}

#[test]
fn fmt_mb_1047561() {
    assert_eq!(app_lib::format_size(1047561u64), "1023.0 KB");
}

#[test]
fn fmt_mb_1047562() {
    assert_eq!(app_lib::format_size(1047562u64), "1023.0 KB");
}

#[test]
fn fmt_mb_1047563() {
    assert_eq!(app_lib::format_size(1047563u64), "1023.0 KB");
}

#[test]
fn fmt_mb_1047564() {
    assert_eq!(app_lib::format_size(1047564u64), "1023.0 KB");
}

#[test]
fn fmt_mb_1047565() {
    assert_eq!(app_lib::format_size(1047565u64), "1023.0 KB");
}

#[test]
fn fmt_mb_1047566() {
    assert_eq!(app_lib::format_size(1047566u64), "1023.0 KB");
}

#[test]
fn fmt_mb_1047567() {
    assert_eq!(app_lib::format_size(1047567u64), "1023.0 KB");
}

#[test]
fn fmt_mb_1047568() {
    assert_eq!(app_lib::format_size(1047568u64), "1023.0 KB");
}

#[test]
fn fmt_mb_1047569() {
    assert_eq!(app_lib::format_size(1047569u64), "1023.0 KB");
}

#[test]
fn fmt_mb_1047570() {
    assert_eq!(app_lib::format_size(1047570u64), "1023.0 KB");
}

#[test]
fn fmt_mb_1047571() {
    assert_eq!(app_lib::format_size(1047571u64), "1023.0 KB");
}

#[test]
fn fmt_mb_1047572() {
    assert_eq!(app_lib::format_size(1047572u64), "1023.0 KB");
}

#[test]
fn fmt_mb_1047573() {
    assert_eq!(app_lib::format_size(1047573u64), "1023.0 KB");
}

#[test]
fn fmt_mb_1047574() {
    assert_eq!(app_lib::format_size(1047574u64), "1023.0 KB");
}

#[test]
fn fmt_mb_1047575() {
    assert_eq!(app_lib::format_size(1047575u64), "1023.0 KB");
}

#[test]
fn fmt_mb_1047576() {
    assert_eq!(app_lib::format_size(1047576u64), "1023.0 KB");
}

#[test]
fn fmt_mb_1047577() {
    assert_eq!(app_lib::format_size(1047577u64), "1023.0 KB");
}

#[test]
fn fmt_mb_1047578() {
    assert_eq!(app_lib::format_size(1047578u64), "1023.0 KB");
}

#[test]
fn fmt_mb_1047579() {
    assert_eq!(app_lib::format_size(1047579u64), "1023.0 KB");
}

#[test]
fn fmt_mb_1047580() {
    assert_eq!(app_lib::format_size(1047580u64), "1023.0 KB");
}

#[test]
fn fmt_mb_1047581() {
    assert_eq!(app_lib::format_size(1047581u64), "1023.0 KB");
}

#[test]
fn fmt_mb_1047582() {
    assert_eq!(app_lib::format_size(1047582u64), "1023.0 KB");
}

#[test]
fn fmt_mb_1047583() {
    assert_eq!(app_lib::format_size(1047583u64), "1023.0 KB");
}

#[test]
fn fmt_mb_1047584() {
    assert_eq!(app_lib::format_size(1047584u64), "1023.0 KB");
}

#[test]
fn fmt_mb_1047585() {
    assert_eq!(app_lib::format_size(1047585u64), "1023.0 KB");
}

#[test]
fn fmt_mb_1047586() {
    assert_eq!(app_lib::format_size(1047586u64), "1023.0 KB");
}

#[test]
fn fmt_mb_1047587() {
    assert_eq!(app_lib::format_size(1047587u64), "1023.0 KB");
}

#[test]
fn fmt_mb_1047588() {
    assert_eq!(app_lib::format_size(1047588u64), "1023.0 KB");
}

#[test]
fn fmt_mb_1047589() {
    assert_eq!(app_lib::format_size(1047589u64), "1023.0 KB");
}

#[test]
fn fmt_mb_1047590() {
    assert_eq!(app_lib::format_size(1047590u64), "1023.0 KB");
}

#[test]
fn fmt_mb_1047591() {
    assert_eq!(app_lib::format_size(1047591u64), "1023.0 KB");
}

#[test]
fn fmt_mb_1047592() {
    assert_eq!(app_lib::format_size(1047592u64), "1023.0 KB");
}

#[test]
fn fmt_mb_1047593() {
    assert_eq!(app_lib::format_size(1047593u64), "1023.0 KB");
}

#[test]
fn fmt_mb_1047594() {
    assert_eq!(app_lib::format_size(1047594u64), "1023.0 KB");
}

#[test]
fn fmt_mb_1047595() {
    assert_eq!(app_lib::format_size(1047595u64), "1023.0 KB");
}

#[test]
fn fmt_mb_1047596() {
    assert_eq!(app_lib::format_size(1047596u64), "1023.0 KB");
}

#[test]
fn fmt_mb_1047597() {
    assert_eq!(app_lib::format_size(1047597u64), "1023.0 KB");
}

#[test]
fn fmt_mb_1047598() {
    assert_eq!(app_lib::format_size(1047598u64), "1023.0 KB");
}

#[test]
fn fmt_mb_1047599() {
    assert_eq!(app_lib::format_size(1047599u64), "1023.0 KB");
}

#[test]
fn fmt_mb_1047600() {
    assert_eq!(app_lib::format_size(1047600u64), "1023.0 KB");
}

#[test]
fn fmt_mb_1047601() {
    assert_eq!(app_lib::format_size(1047601u64), "1023.0 KB");
}

#[test]
fn fmt_mb_1047602() {
    assert_eq!(app_lib::format_size(1047602u64), "1023.0 KB");
}

#[test]
fn fmt_mb_1047603() {
    assert_eq!(app_lib::format_size(1047603u64), "1023.0 KB");
}

#[test]
fn fmt_mb_1047604() {
    assert_eq!(app_lib::format_size(1047604u64), "1023.1 KB");
}

#[test]
fn fmt_mb_1047605() {
    assert_eq!(app_lib::format_size(1047605u64), "1023.1 KB");
}

#[test]
fn fmt_mb_1047606() {
    assert_eq!(app_lib::format_size(1047606u64), "1023.1 KB");
}

#[test]
fn fmt_mb_1047607() {
    assert_eq!(app_lib::format_size(1047607u64), "1023.1 KB");
}

#[test]
fn fmt_mb_1047608() {
    assert_eq!(app_lib::format_size(1047608u64), "1023.1 KB");
}

#[test]
fn fmt_mb_1047609() {
    assert_eq!(app_lib::format_size(1047609u64), "1023.1 KB");
}

#[test]
fn fmt_mb_1047610() {
    assert_eq!(app_lib::format_size(1047610u64), "1023.1 KB");
}

#[test]
fn fmt_mb_1047611() {
    assert_eq!(app_lib::format_size(1047611u64), "1023.1 KB");
}

#[test]
fn fmt_mb_1047612() {
    assert_eq!(app_lib::format_size(1047612u64), "1023.1 KB");
}

#[test]
fn fmt_mb_1047613() {
    assert_eq!(app_lib::format_size(1047613u64), "1023.1 KB");
}

#[test]
fn fmt_mb_1047614() {
    assert_eq!(app_lib::format_size(1047614u64), "1023.1 KB");
}

#[test]
fn fmt_mb_1047615() {
    assert_eq!(app_lib::format_size(1047615u64), "1023.1 KB");
}

#[test]
fn fmt_mb_1047616() {
    assert_eq!(app_lib::format_size(1047616u64), "1023.1 KB");
}

#[test]
fn fmt_mb_1047617() {
    assert_eq!(app_lib::format_size(1047617u64), "1023.1 KB");
}

#[test]
fn fmt_mb_1047618() {
    assert_eq!(app_lib::format_size(1047618u64), "1023.1 KB");
}

#[test]
fn fmt_mb_1047619() {
    assert_eq!(app_lib::format_size(1047619u64), "1023.1 KB");
}

#[test]
fn fmt_mb_1047620() {
    assert_eq!(app_lib::format_size(1047620u64), "1023.1 KB");
}

#[test]
fn fmt_mb_1047621() {
    assert_eq!(app_lib::format_size(1047621u64), "1023.1 KB");
}

#[test]
fn fmt_mb_1047622() {
    assert_eq!(app_lib::format_size(1047622u64), "1023.1 KB");
}

#[test]
fn fmt_mb_1047623() {
    assert_eq!(app_lib::format_size(1047623u64), "1023.1 KB");
}

#[test]
fn fmt_mb_1047624() {
    assert_eq!(app_lib::format_size(1047624u64), "1023.1 KB");
}

#[test]
fn fmt_mb_1047625() {
    assert_eq!(app_lib::format_size(1047625u64), "1023.1 KB");
}

#[test]
fn fmt_mb_1047626() {
    assert_eq!(app_lib::format_size(1047626u64), "1023.1 KB");
}

#[test]
fn fmt_mb_1047627() {
    assert_eq!(app_lib::format_size(1047627u64), "1023.1 KB");
}

#[test]
fn fmt_mb_1047628() {
    assert_eq!(app_lib::format_size(1047628u64), "1023.1 KB");
}

#[test]
fn fmt_mb_1047629() {
    assert_eq!(app_lib::format_size(1047629u64), "1023.1 KB");
}

#[test]
fn fmt_mb_1047630() {
    assert_eq!(app_lib::format_size(1047630u64), "1023.1 KB");
}

#[test]
fn fmt_mb_1047631() {
    assert_eq!(app_lib::format_size(1047631u64), "1023.1 KB");
}

#[test]
fn fmt_mb_1047632() {
    assert_eq!(app_lib::format_size(1047632u64), "1023.1 KB");
}

#[test]
fn fmt_mb_1047633() {
    assert_eq!(app_lib::format_size(1047633u64), "1023.1 KB");
}

#[test]
fn fmt_mb_1047634() {
    assert_eq!(app_lib::format_size(1047634u64), "1023.1 KB");
}

#[test]
fn fmt_mb_1047635() {
    assert_eq!(app_lib::format_size(1047635u64), "1023.1 KB");
}

#[test]
fn fmt_mb_1047636() {
    assert_eq!(app_lib::format_size(1047636u64), "1023.1 KB");
}

#[test]
fn fmt_mb_1047637() {
    assert_eq!(app_lib::format_size(1047637u64), "1023.1 KB");
}

#[test]
fn fmt_mb_1047638() {
    assert_eq!(app_lib::format_size(1047638u64), "1023.1 KB");
}

#[test]
fn fmt_mb_1047639() {
    assert_eq!(app_lib::format_size(1047639u64), "1023.1 KB");
}

#[test]
fn fmt_mb_1047640() {
    assert_eq!(app_lib::format_size(1047640u64), "1023.1 KB");
}

#[test]
fn fmt_mb_1047641() {
    assert_eq!(app_lib::format_size(1047641u64), "1023.1 KB");
}

#[test]
fn fmt_mb_1047642() {
    assert_eq!(app_lib::format_size(1047642u64), "1023.1 KB");
}

#[test]
fn fmt_mb_1047643() {
    assert_eq!(app_lib::format_size(1047643u64), "1023.1 KB");
}

#[test]
fn fmt_mb_1047644() {
    assert_eq!(app_lib::format_size(1047644u64), "1023.1 KB");
}

#[test]
fn fmt_mb_1047645() {
    assert_eq!(app_lib::format_size(1047645u64), "1023.1 KB");
}

#[test]
fn fmt_mb_1047646() {
    assert_eq!(app_lib::format_size(1047646u64), "1023.1 KB");
}

#[test]
fn fmt_mb_1047647() {
    assert_eq!(app_lib::format_size(1047647u64), "1023.1 KB");
}

#[test]
fn fmt_mb_1047648() {
    assert_eq!(app_lib::format_size(1047648u64), "1023.1 KB");
}

#[test]
fn fmt_mb_1047649() {
    assert_eq!(app_lib::format_size(1047649u64), "1023.1 KB");
}

#[test]
fn fmt_mb_1047650() {
    assert_eq!(app_lib::format_size(1047650u64), "1023.1 KB");
}

#[test]
fn fmt_mb_1047651() {
    assert_eq!(app_lib::format_size(1047651u64), "1023.1 KB");
}

#[test]
fn fmt_mb_1047652() {
    assert_eq!(app_lib::format_size(1047652u64), "1023.1 KB");
}

#[test]
fn fmt_mb_1047653() {
    assert_eq!(app_lib::format_size(1047653u64), "1023.1 KB");
}

#[test]
fn fmt_mb_1047654() {
    assert_eq!(app_lib::format_size(1047654u64), "1023.1 KB");
}

#[test]
fn fmt_mb_1047655() {
    assert_eq!(app_lib::format_size(1047655u64), "1023.1 KB");
}

#[test]
fn fmt_mb_1047656() {
    assert_eq!(app_lib::format_size(1047656u64), "1023.1 KB");
}

#[test]
fn fmt_mb_1047657() {
    assert_eq!(app_lib::format_size(1047657u64), "1023.1 KB");
}

#[test]
fn fmt_mb_1047658() {
    assert_eq!(app_lib::format_size(1047658u64), "1023.1 KB");
}

#[test]
fn fmt_mb_1047659() {
    assert_eq!(app_lib::format_size(1047659u64), "1023.1 KB");
}

#[test]
fn fmt_mb_1047660() {
    assert_eq!(app_lib::format_size(1047660u64), "1023.1 KB");
}

#[test]
fn fmt_mb_1047661() {
    assert_eq!(app_lib::format_size(1047661u64), "1023.1 KB");
}

#[test]
fn fmt_mb_1047662() {
    assert_eq!(app_lib::format_size(1047662u64), "1023.1 KB");
}

#[test]
fn fmt_mb_1047663() {
    assert_eq!(app_lib::format_size(1047663u64), "1023.1 KB");
}

#[test]
fn fmt_mb_1047664() {
    assert_eq!(app_lib::format_size(1047664u64), "1023.1 KB");
}

#[test]
fn fmt_mb_1047665() {
    assert_eq!(app_lib::format_size(1047665u64), "1023.1 KB");
}

#[test]
fn fmt_mb_1047666() {
    assert_eq!(app_lib::format_size(1047666u64), "1023.1 KB");
}

#[test]
fn fmt_mb_1047667() {
    assert_eq!(app_lib::format_size(1047667u64), "1023.1 KB");
}

#[test]
fn fmt_mb_1047668() {
    assert_eq!(app_lib::format_size(1047668u64), "1023.1 KB");
}

#[test]
fn fmt_mb_1047669() {
    assert_eq!(app_lib::format_size(1047669u64), "1023.1 KB");
}

#[test]
fn fmt_mb_1047670() {
    assert_eq!(app_lib::format_size(1047670u64), "1023.1 KB");
}

#[test]
fn fmt_mb_1047671() {
    assert_eq!(app_lib::format_size(1047671u64), "1023.1 KB");
}

#[test]
fn fmt_mb_1047672() {
    assert_eq!(app_lib::format_size(1047672u64), "1023.1 KB");
}

#[test]
fn fmt_mb_1047673() {
    assert_eq!(app_lib::format_size(1047673u64), "1023.1 KB");
}

#[test]
fn fmt_mb_1047674() {
    assert_eq!(app_lib::format_size(1047674u64), "1023.1 KB");
}

#[test]
fn fmt_mb_1047675() {
    assert_eq!(app_lib::format_size(1047675u64), "1023.1 KB");
}

#[test]
fn fmt_mb_1047676() {
    assert_eq!(app_lib::format_size(1047676u64), "1023.1 KB");
}

#[test]
fn fmt_mb_1047677() {
    assert_eq!(app_lib::format_size(1047677u64), "1023.1 KB");
}

#[test]
fn fmt_mb_1047678() {
    assert_eq!(app_lib::format_size(1047678u64), "1023.1 KB");
}

#[test]
fn fmt_mb_1047679() {
    assert_eq!(app_lib::format_size(1047679u64), "1023.1 KB");
}

#[test]
fn fmt_mb_1047680() {
    assert_eq!(app_lib::format_size(1047680u64), "1023.1 KB");
}

#[test]
fn fmt_mb_1047681() {
    assert_eq!(app_lib::format_size(1047681u64), "1023.1 KB");
}

#[test]
fn fmt_mb_1047682() {
    assert_eq!(app_lib::format_size(1047682u64), "1023.1 KB");
}

#[test]
fn fmt_mb_1047683() {
    assert_eq!(app_lib::format_size(1047683u64), "1023.1 KB");
}

#[test]
fn fmt_mb_1047684() {
    assert_eq!(app_lib::format_size(1047684u64), "1023.1 KB");
}

#[test]
fn fmt_mb_1047685() {
    assert_eq!(app_lib::format_size(1047685u64), "1023.1 KB");
}

#[test]
fn fmt_mb_1047686() {
    assert_eq!(app_lib::format_size(1047686u64), "1023.1 KB");
}

#[test]
fn fmt_mb_1047687() {
    assert_eq!(app_lib::format_size(1047687u64), "1023.1 KB");
}

#[test]
fn fmt_mb_1047688() {
    assert_eq!(app_lib::format_size(1047688u64), "1023.1 KB");
}

#[test]
fn fmt_mb_1047689() {
    assert_eq!(app_lib::format_size(1047689u64), "1023.1 KB");
}

#[test]
fn fmt_mb_1047690() {
    assert_eq!(app_lib::format_size(1047690u64), "1023.1 KB");
}

#[test]
fn fmt_mb_1047691() {
    assert_eq!(app_lib::format_size(1047691u64), "1023.1 KB");
}

#[test]
fn fmt_mb_1047692() {
    assert_eq!(app_lib::format_size(1047692u64), "1023.1 KB");
}

#[test]
fn fmt_mb_1047693() {
    assert_eq!(app_lib::format_size(1047693u64), "1023.1 KB");
}

#[test]
fn fmt_mb_1047694() {
    assert_eq!(app_lib::format_size(1047694u64), "1023.1 KB");
}

#[test]
fn fmt_mb_1047695() {
    assert_eq!(app_lib::format_size(1047695u64), "1023.1 KB");
}

#[test]
fn fmt_mb_1047696() {
    assert_eq!(app_lib::format_size(1047696u64), "1023.1 KB");
}

#[test]
fn fmt_mb_1047697() {
    assert_eq!(app_lib::format_size(1047697u64), "1023.1 KB");
}

#[test]
fn fmt_mb_1047698() {
    assert_eq!(app_lib::format_size(1047698u64), "1023.1 KB");
}

#[test]
fn fmt_mb_1047699() {
    assert_eq!(app_lib::format_size(1047699u64), "1023.1 KB");
}

#[test]
fn fmt_mb_1047700() {
    assert_eq!(app_lib::format_size(1047700u64), "1023.1 KB");
}

#[test]
fn fmt_mb_1047701() {
    assert_eq!(app_lib::format_size(1047701u64), "1023.1 KB");
}

#[test]
fn fmt_mb_1047702() {
    assert_eq!(app_lib::format_size(1047702u64), "1023.1 KB");
}

#[test]
fn fmt_mb_1047703() {
    assert_eq!(app_lib::format_size(1047703u64), "1023.1 KB");
}

#[test]
fn fmt_mb_1047704() {
    assert_eq!(app_lib::format_size(1047704u64), "1023.1 KB");
}

#[test]
fn fmt_mb_1047705() {
    assert_eq!(app_lib::format_size(1047705u64), "1023.1 KB");
}

#[test]
fn fmt_mb_1047706() {
    assert_eq!(app_lib::format_size(1047706u64), "1023.2 KB");
}

#[test]
fn fmt_mb_1047707() {
    assert_eq!(app_lib::format_size(1047707u64), "1023.2 KB");
}

#[test]
fn fmt_mb_1047708() {
    assert_eq!(app_lib::format_size(1047708u64), "1023.2 KB");
}

#[test]
fn fmt_mb_1047709() {
    assert_eq!(app_lib::format_size(1047709u64), "1023.2 KB");
}

#[test]
fn fmt_mb_1047710() {
    assert_eq!(app_lib::format_size(1047710u64), "1023.2 KB");
}

#[test]
fn fmt_mb_1047711() {
    assert_eq!(app_lib::format_size(1047711u64), "1023.2 KB");
}

#[test]
fn fmt_mb_1047712() {
    assert_eq!(app_lib::format_size(1047712u64), "1023.2 KB");
}

#[test]
fn fmt_mb_1047713() {
    assert_eq!(app_lib::format_size(1047713u64), "1023.2 KB");
}

#[test]
fn fmt_mb_1047714() {
    assert_eq!(app_lib::format_size(1047714u64), "1023.2 KB");
}

#[test]
fn fmt_mb_1047715() {
    assert_eq!(app_lib::format_size(1047715u64), "1023.2 KB");
}

#[test]
fn fmt_mb_1047716() {
    assert_eq!(app_lib::format_size(1047716u64), "1023.2 KB");
}

#[test]
fn fmt_mb_1047717() {
    assert_eq!(app_lib::format_size(1047717u64), "1023.2 KB");
}

#[test]
fn fmt_mb_1047718() {
    assert_eq!(app_lib::format_size(1047718u64), "1023.2 KB");
}

#[test]
fn fmt_mb_1047719() {
    assert_eq!(app_lib::format_size(1047719u64), "1023.2 KB");
}

#[test]
fn fmt_mb_1047720() {
    assert_eq!(app_lib::format_size(1047720u64), "1023.2 KB");
}

#[test]
fn fmt_mb_1047721() {
    assert_eq!(app_lib::format_size(1047721u64), "1023.2 KB");
}

#[test]
fn fmt_mb_1047722() {
    assert_eq!(app_lib::format_size(1047722u64), "1023.2 KB");
}

#[test]
fn fmt_mb_1047723() {
    assert_eq!(app_lib::format_size(1047723u64), "1023.2 KB");
}

#[test]
fn fmt_mb_1047724() {
    assert_eq!(app_lib::format_size(1047724u64), "1023.2 KB");
}

#[test]
fn fmt_mb_1047725() {
    assert_eq!(app_lib::format_size(1047725u64), "1023.2 KB");
}

#[test]
fn fmt_mb_1047726() {
    assert_eq!(app_lib::format_size(1047726u64), "1023.2 KB");
}

#[test]
fn fmt_mb_1047727() {
    assert_eq!(app_lib::format_size(1047727u64), "1023.2 KB");
}

#[test]
fn fmt_mb_1047728() {
    assert_eq!(app_lib::format_size(1047728u64), "1023.2 KB");
}

#[test]
fn fmt_mb_1047729() {
    assert_eq!(app_lib::format_size(1047729u64), "1023.2 KB");
}

#[test]
fn fmt_mb_1047730() {
    assert_eq!(app_lib::format_size(1047730u64), "1023.2 KB");
}

#[test]
fn fmt_mb_1047731() {
    assert_eq!(app_lib::format_size(1047731u64), "1023.2 KB");
}

#[test]
fn fmt_mb_1047732() {
    assert_eq!(app_lib::format_size(1047732u64), "1023.2 KB");
}

#[test]
fn fmt_mb_1047733() {
    assert_eq!(app_lib::format_size(1047733u64), "1023.2 KB");
}

#[test]
fn fmt_mb_1047734() {
    assert_eq!(app_lib::format_size(1047734u64), "1023.2 KB");
}

#[test]
fn fmt_mb_1047735() {
    assert_eq!(app_lib::format_size(1047735u64), "1023.2 KB");
}

#[test]
fn fmt_mb_1047736() {
    assert_eq!(app_lib::format_size(1047736u64), "1023.2 KB");
}

#[test]
fn fmt_mb_1047737() {
    assert_eq!(app_lib::format_size(1047737u64), "1023.2 KB");
}

#[test]
fn fmt_mb_1047738() {
    assert_eq!(app_lib::format_size(1047738u64), "1023.2 KB");
}

#[test]
fn fmt_mb_1047739() {
    assert_eq!(app_lib::format_size(1047739u64), "1023.2 KB");
}

#[test]
fn fmt_mb_1047740() {
    assert_eq!(app_lib::format_size(1047740u64), "1023.2 KB");
}

#[test]
fn fmt_mb_1047741() {
    assert_eq!(app_lib::format_size(1047741u64), "1023.2 KB");
}

#[test]
fn fmt_mb_1047742() {
    assert_eq!(app_lib::format_size(1047742u64), "1023.2 KB");
}

#[test]
fn fmt_mb_1047743() {
    assert_eq!(app_lib::format_size(1047743u64), "1023.2 KB");
}

#[test]
fn fmt_mb_1047744() {
    assert_eq!(app_lib::format_size(1047744u64), "1023.2 KB");
}

#[test]
fn fmt_mb_1047745() {
    assert_eq!(app_lib::format_size(1047745u64), "1023.2 KB");
}

#[test]
fn fmt_mb_1047746() {
    assert_eq!(app_lib::format_size(1047746u64), "1023.2 KB");
}

#[test]
fn fmt_mb_1047747() {
    assert_eq!(app_lib::format_size(1047747u64), "1023.2 KB");
}

#[test]
fn fmt_mb_1047748() {
    assert_eq!(app_lib::format_size(1047748u64), "1023.2 KB");
}

#[test]
fn fmt_mb_1047749() {
    assert_eq!(app_lib::format_size(1047749u64), "1023.2 KB");
}

#[test]
fn fmt_mb_1047750() {
    assert_eq!(app_lib::format_size(1047750u64), "1023.2 KB");
}

#[test]
fn fmt_mb_1047751() {
    assert_eq!(app_lib::format_size(1047751u64), "1023.2 KB");
}

#[test]
fn fmt_mb_1047752() {
    assert_eq!(app_lib::format_size(1047752u64), "1023.2 KB");
}

#[test]
fn fmt_mb_1047753() {
    assert_eq!(app_lib::format_size(1047753u64), "1023.2 KB");
}

#[test]
fn fmt_mb_1047754() {
    assert_eq!(app_lib::format_size(1047754u64), "1023.2 KB");
}

#[test]
fn fmt_mb_1047755() {
    assert_eq!(app_lib::format_size(1047755u64), "1023.2 KB");
}

#[test]
fn fmt_mb_1047756() {
    assert_eq!(app_lib::format_size(1047756u64), "1023.2 KB");
}

#[test]
fn fmt_mb_1047757() {
    assert_eq!(app_lib::format_size(1047757u64), "1023.2 KB");
}

#[test]
fn fmt_mb_1047758() {
    assert_eq!(app_lib::format_size(1047758u64), "1023.2 KB");
}

#[test]
fn fmt_mb_1047759() {
    assert_eq!(app_lib::format_size(1047759u64), "1023.2 KB");
}

#[test]
fn fmt_mb_1047760() {
    assert_eq!(app_lib::format_size(1047760u64), "1023.2 KB");
}

#[test]
fn fmt_mb_1047761() {
    assert_eq!(app_lib::format_size(1047761u64), "1023.2 KB");
}

#[test]
fn fmt_mb_1047762() {
    assert_eq!(app_lib::format_size(1047762u64), "1023.2 KB");
}

#[test]
fn fmt_mb_1047763() {
    assert_eq!(app_lib::format_size(1047763u64), "1023.2 KB");
}

#[test]
fn fmt_mb_1047764() {
    assert_eq!(app_lib::format_size(1047764u64), "1023.2 KB");
}

#[test]
fn fmt_mb_1047765() {
    assert_eq!(app_lib::format_size(1047765u64), "1023.2 KB");
}

#[test]
fn fmt_mb_1047766() {
    assert_eq!(app_lib::format_size(1047766u64), "1023.2 KB");
}

#[test]
fn fmt_mb_1047767() {
    assert_eq!(app_lib::format_size(1047767u64), "1023.2 KB");
}

#[test]
fn fmt_mb_1047768() {
    assert_eq!(app_lib::format_size(1047768u64), "1023.2 KB");
}

#[test]
fn fmt_mb_1047769() {
    assert_eq!(app_lib::format_size(1047769u64), "1023.2 KB");
}

#[test]
fn fmt_mb_1047770() {
    assert_eq!(app_lib::format_size(1047770u64), "1023.2 KB");
}

#[test]
fn fmt_mb_1047771() {
    assert_eq!(app_lib::format_size(1047771u64), "1023.2 KB");
}

#[test]
fn fmt_mb_1047772() {
    assert_eq!(app_lib::format_size(1047772u64), "1023.2 KB");
}

#[test]
fn fmt_mb_1047773() {
    assert_eq!(app_lib::format_size(1047773u64), "1023.2 KB");
}

#[test]
fn fmt_mb_1047774() {
    assert_eq!(app_lib::format_size(1047774u64), "1023.2 KB");
}

#[test]
fn fmt_mb_1047775() {
    assert_eq!(app_lib::format_size(1047775u64), "1023.2 KB");
}

#[test]
fn fmt_mb_1047776() {
    assert_eq!(app_lib::format_size(1047776u64), "1023.2 KB");
}

#[test]
fn fmt_mb_1047777() {
    assert_eq!(app_lib::format_size(1047777u64), "1023.2 KB");
}

#[test]
fn fmt_mb_1047778() {
    assert_eq!(app_lib::format_size(1047778u64), "1023.2 KB");
}

#[test]
fn fmt_mb_1047779() {
    assert_eq!(app_lib::format_size(1047779u64), "1023.2 KB");
}

#[test]
fn fmt_mb_1047780() {
    assert_eq!(app_lib::format_size(1047780u64), "1023.2 KB");
}

#[test]
fn fmt_mb_1047781() {
    assert_eq!(app_lib::format_size(1047781u64), "1023.2 KB");
}

#[test]
fn fmt_mb_1047782() {
    assert_eq!(app_lib::format_size(1047782u64), "1023.2 KB");
}

#[test]
fn fmt_mb_1047783() {
    assert_eq!(app_lib::format_size(1047783u64), "1023.2 KB");
}

#[test]
fn fmt_mb_1047784() {
    assert_eq!(app_lib::format_size(1047784u64), "1023.2 KB");
}

#[test]
fn fmt_mb_1047785() {
    assert_eq!(app_lib::format_size(1047785u64), "1023.2 KB");
}

#[test]
fn fmt_mb_1047786() {
    assert_eq!(app_lib::format_size(1047786u64), "1023.2 KB");
}

#[test]
fn fmt_mb_1047787() {
    assert_eq!(app_lib::format_size(1047787u64), "1023.2 KB");
}

#[test]
fn fmt_mb_1047788() {
    assert_eq!(app_lib::format_size(1047788u64), "1023.2 KB");
}

#[test]
fn fmt_mb_1047789() {
    assert_eq!(app_lib::format_size(1047789u64), "1023.2 KB");
}

#[test]
fn fmt_mb_1047790() {
    assert_eq!(app_lib::format_size(1047790u64), "1023.2 KB");
}

#[test]
fn fmt_mb_1047791() {
    assert_eq!(app_lib::format_size(1047791u64), "1023.2 KB");
}

#[test]
fn fmt_mb_1047792() {
    assert_eq!(app_lib::format_size(1047792u64), "1023.2 KB");
}

#[test]
fn fmt_mb_1047793() {
    assert_eq!(app_lib::format_size(1047793u64), "1023.2 KB");
}

#[test]
fn fmt_mb_1047794() {
    assert_eq!(app_lib::format_size(1047794u64), "1023.2 KB");
}

#[test]
fn fmt_mb_1047795() {
    assert_eq!(app_lib::format_size(1047795u64), "1023.2 KB");
}

#[test]
fn fmt_mb_1047796() {
    assert_eq!(app_lib::format_size(1047796u64), "1023.2 KB");
}

#[test]
fn fmt_mb_1047797() {
    assert_eq!(app_lib::format_size(1047797u64), "1023.2 KB");
}

#[test]
fn fmt_mb_1047798() {
    assert_eq!(app_lib::format_size(1047798u64), "1023.2 KB");
}

#[test]
fn fmt_mb_1047799() {
    assert_eq!(app_lib::format_size(1047799u64), "1023.2 KB");
}

#[test]
fn fmt_mb_1047800() {
    assert_eq!(app_lib::format_size(1047800u64), "1023.2 KB");
}

#[test]
fn fmt_mb_1047801() {
    assert_eq!(app_lib::format_size(1047801u64), "1023.2 KB");
}

#[test]
fn fmt_mb_1047802() {
    assert_eq!(app_lib::format_size(1047802u64), "1023.2 KB");
}

#[test]
fn fmt_mb_1047803() {
    assert_eq!(app_lib::format_size(1047803u64), "1023.2 KB");
}

#[test]
fn fmt_mb_1047804() {
    assert_eq!(app_lib::format_size(1047804u64), "1023.2 KB");
}

#[test]
fn fmt_mb_1047805() {
    assert_eq!(app_lib::format_size(1047805u64), "1023.2 KB");
}

#[test]
fn fmt_mb_1047806() {
    assert_eq!(app_lib::format_size(1047806u64), "1023.2 KB");
}

#[test]
fn fmt_mb_1047807() {
    assert_eq!(app_lib::format_size(1047807u64), "1023.2 KB");
}

#[test]
fn fmt_mb_1047808() {
    assert_eq!(app_lib::format_size(1047808u64), "1023.2 KB");
}

#[test]
fn fmt_mb_1047809() {
    assert_eq!(app_lib::format_size(1047809u64), "1023.3 KB");
}

#[test]
fn fmt_mb_1047810() {
    assert_eq!(app_lib::format_size(1047810u64), "1023.3 KB");
}

#[test]
fn fmt_mb_1047811() {
    assert_eq!(app_lib::format_size(1047811u64), "1023.3 KB");
}

#[test]
fn fmt_mb_1047812() {
    assert_eq!(app_lib::format_size(1047812u64), "1023.3 KB");
}

#[test]
fn fmt_mb_1047813() {
    assert_eq!(app_lib::format_size(1047813u64), "1023.3 KB");
}

#[test]
fn fmt_mb_1047814() {
    assert_eq!(app_lib::format_size(1047814u64), "1023.3 KB");
}

#[test]
fn fmt_mb_1047815() {
    assert_eq!(app_lib::format_size(1047815u64), "1023.3 KB");
}

#[test]
fn fmt_mb_1047816() {
    assert_eq!(app_lib::format_size(1047816u64), "1023.3 KB");
}

#[test]
fn fmt_mb_1047817() {
    assert_eq!(app_lib::format_size(1047817u64), "1023.3 KB");
}

#[test]
fn fmt_mb_1047818() {
    assert_eq!(app_lib::format_size(1047818u64), "1023.3 KB");
}

#[test]
fn fmt_mb_1047819() {
    assert_eq!(app_lib::format_size(1047819u64), "1023.3 KB");
}

#[test]
fn fmt_mb_1047820() {
    assert_eq!(app_lib::format_size(1047820u64), "1023.3 KB");
}

#[test]
fn fmt_mb_1047821() {
    assert_eq!(app_lib::format_size(1047821u64), "1023.3 KB");
}

#[test]
fn fmt_mb_1047822() {
    assert_eq!(app_lib::format_size(1047822u64), "1023.3 KB");
}

#[test]
fn fmt_mb_1047823() {
    assert_eq!(app_lib::format_size(1047823u64), "1023.3 KB");
}

#[test]
fn fmt_mb_1047824() {
    assert_eq!(app_lib::format_size(1047824u64), "1023.3 KB");
}

#[test]
fn fmt_mb_1047825() {
    assert_eq!(app_lib::format_size(1047825u64), "1023.3 KB");
}

#[test]
fn fmt_mb_1047826() {
    assert_eq!(app_lib::format_size(1047826u64), "1023.3 KB");
}

#[test]
fn fmt_mb_1047827() {
    assert_eq!(app_lib::format_size(1047827u64), "1023.3 KB");
}

#[test]
fn fmt_mb_1047828() {
    assert_eq!(app_lib::format_size(1047828u64), "1023.3 KB");
}

#[test]
fn fmt_mb_1047829() {
    assert_eq!(app_lib::format_size(1047829u64), "1023.3 KB");
}

#[test]
fn fmt_mb_1047830() {
    assert_eq!(app_lib::format_size(1047830u64), "1023.3 KB");
}

#[test]
fn fmt_mb_1047831() {
    assert_eq!(app_lib::format_size(1047831u64), "1023.3 KB");
}

#[test]
fn fmt_mb_1047832() {
    assert_eq!(app_lib::format_size(1047832u64), "1023.3 KB");
}

#[test]
fn fmt_mb_1047833() {
    assert_eq!(app_lib::format_size(1047833u64), "1023.3 KB");
}

#[test]
fn fmt_mb_1047834() {
    assert_eq!(app_lib::format_size(1047834u64), "1023.3 KB");
}

#[test]
fn fmt_mb_1047835() {
    assert_eq!(app_lib::format_size(1047835u64), "1023.3 KB");
}

#[test]
fn fmt_mb_1047836() {
    assert_eq!(app_lib::format_size(1047836u64), "1023.3 KB");
}

#[test]
fn fmt_mb_1047837() {
    assert_eq!(app_lib::format_size(1047837u64), "1023.3 KB");
}

#[test]
fn fmt_mb_1047838() {
    assert_eq!(app_lib::format_size(1047838u64), "1023.3 KB");
}

#[test]
fn fmt_mb_1047839() {
    assert_eq!(app_lib::format_size(1047839u64), "1023.3 KB");
}

#[test]
fn fmt_mb_1047840() {
    assert_eq!(app_lib::format_size(1047840u64), "1023.3 KB");
}

#[test]
fn fmt_mb_1047841() {
    assert_eq!(app_lib::format_size(1047841u64), "1023.3 KB");
}

#[test]
fn fmt_mb_1047842() {
    assert_eq!(app_lib::format_size(1047842u64), "1023.3 KB");
}

#[test]
fn fmt_mb_1047843() {
    assert_eq!(app_lib::format_size(1047843u64), "1023.3 KB");
}

#[test]
fn fmt_mb_1047844() {
    assert_eq!(app_lib::format_size(1047844u64), "1023.3 KB");
}

#[test]
fn fmt_mb_1047845() {
    assert_eq!(app_lib::format_size(1047845u64), "1023.3 KB");
}

#[test]
fn fmt_mb_1047846() {
    assert_eq!(app_lib::format_size(1047846u64), "1023.3 KB");
}

#[test]
fn fmt_mb_1047847() {
    assert_eq!(app_lib::format_size(1047847u64), "1023.3 KB");
}

#[test]
fn fmt_mb_1047848() {
    assert_eq!(app_lib::format_size(1047848u64), "1023.3 KB");
}

#[test]
fn fmt_mb_1047849() {
    assert_eq!(app_lib::format_size(1047849u64), "1023.3 KB");
}

#[test]
fn fmt_mb_1047850() {
    assert_eq!(app_lib::format_size(1047850u64), "1023.3 KB");
}

#[test]
fn fmt_mb_1047851() {
    assert_eq!(app_lib::format_size(1047851u64), "1023.3 KB");
}

#[test]
fn fmt_mb_1047852() {
    assert_eq!(app_lib::format_size(1047852u64), "1023.3 KB");
}

#[test]
fn fmt_mb_1047853() {
    assert_eq!(app_lib::format_size(1047853u64), "1023.3 KB");
}

#[test]
fn fmt_mb_1047854() {
    assert_eq!(app_lib::format_size(1047854u64), "1023.3 KB");
}

#[test]
fn fmt_mb_1047855() {
    assert_eq!(app_lib::format_size(1047855u64), "1023.3 KB");
}

#[test]
fn fmt_mb_1047856() {
    assert_eq!(app_lib::format_size(1047856u64), "1023.3 KB");
}

#[test]
fn fmt_mb_1047857() {
    assert_eq!(app_lib::format_size(1047857u64), "1023.3 KB");
}

#[test]
fn fmt_mb_1047858() {
    assert_eq!(app_lib::format_size(1047858u64), "1023.3 KB");
}

#[test]
fn fmt_mb_1047859() {
    assert_eq!(app_lib::format_size(1047859u64), "1023.3 KB");
}

#[test]
fn fmt_mb_1047860() {
    assert_eq!(app_lib::format_size(1047860u64), "1023.3 KB");
}

#[test]
fn fmt_mb_1047861() {
    assert_eq!(app_lib::format_size(1047861u64), "1023.3 KB");
}

#[test]
fn fmt_mb_1047862() {
    assert_eq!(app_lib::format_size(1047862u64), "1023.3 KB");
}

#[test]
fn fmt_mb_1047863() {
    assert_eq!(app_lib::format_size(1047863u64), "1023.3 KB");
}

#[test]
fn fmt_mb_1047864() {
    assert_eq!(app_lib::format_size(1047864u64), "1023.3 KB");
}

#[test]
fn fmt_mb_1047865() {
    assert_eq!(app_lib::format_size(1047865u64), "1023.3 KB");
}

#[test]
fn fmt_mb_1047866() {
    assert_eq!(app_lib::format_size(1047866u64), "1023.3 KB");
}

#[test]
fn fmt_mb_1047867() {
    assert_eq!(app_lib::format_size(1047867u64), "1023.3 KB");
}

#[test]
fn fmt_mb_1047868() {
    assert_eq!(app_lib::format_size(1047868u64), "1023.3 KB");
}

#[test]
fn fmt_mb_1047869() {
    assert_eq!(app_lib::format_size(1047869u64), "1023.3 KB");
}

#[test]
fn fmt_mb_1047870() {
    assert_eq!(app_lib::format_size(1047870u64), "1023.3 KB");
}

#[test]
fn fmt_mb_1047871() {
    assert_eq!(app_lib::format_size(1047871u64), "1023.3 KB");
}

#[test]
fn fmt_mb_1047872() {
    assert_eq!(app_lib::format_size(1047872u64), "1023.3 KB");
}

#[test]
fn fmt_mb_1047873() {
    assert_eq!(app_lib::format_size(1047873u64), "1023.3 KB");
}

#[test]
fn fmt_mb_1047874() {
    assert_eq!(app_lib::format_size(1047874u64), "1023.3 KB");
}

#[test]
fn fmt_mb_1047875() {
    assert_eq!(app_lib::format_size(1047875u64), "1023.3 KB");
}

#[test]
fn fmt_mb_1047876() {
    assert_eq!(app_lib::format_size(1047876u64), "1023.3 KB");
}

#[test]
fn fmt_mb_1047877() {
    assert_eq!(app_lib::format_size(1047877u64), "1023.3 KB");
}

#[test]
fn fmt_mb_1047878() {
    assert_eq!(app_lib::format_size(1047878u64), "1023.3 KB");
}

#[test]
fn fmt_mb_1047879() {
    assert_eq!(app_lib::format_size(1047879u64), "1023.3 KB");
}

#[test]
fn fmt_mb_1047880() {
    assert_eq!(app_lib::format_size(1047880u64), "1023.3 KB");
}

#[test]
fn fmt_mb_1047881() {
    assert_eq!(app_lib::format_size(1047881u64), "1023.3 KB");
}

#[test]
fn fmt_mb_1047882() {
    assert_eq!(app_lib::format_size(1047882u64), "1023.3 KB");
}

#[test]
fn fmt_mb_1047883() {
    assert_eq!(app_lib::format_size(1047883u64), "1023.3 KB");
}

#[test]
fn fmt_mb_1047884() {
    assert_eq!(app_lib::format_size(1047884u64), "1023.3 KB");
}

#[test]
fn fmt_mb_1047885() {
    assert_eq!(app_lib::format_size(1047885u64), "1023.3 KB");
}

#[test]
fn fmt_mb_1047886() {
    assert_eq!(app_lib::format_size(1047886u64), "1023.3 KB");
}

#[test]
fn fmt_mb_1047887() {
    assert_eq!(app_lib::format_size(1047887u64), "1023.3 KB");
}

#[test]
fn fmt_mb_1047888() {
    assert_eq!(app_lib::format_size(1047888u64), "1023.3 KB");
}

#[test]
fn fmt_mb_1047889() {
    assert_eq!(app_lib::format_size(1047889u64), "1023.3 KB");
}

#[test]
fn fmt_mb_1047890() {
    assert_eq!(app_lib::format_size(1047890u64), "1023.3 KB");
}

#[test]
fn fmt_mb_1047891() {
    assert_eq!(app_lib::format_size(1047891u64), "1023.3 KB");
}

#[test]
fn fmt_mb_1047892() {
    assert_eq!(app_lib::format_size(1047892u64), "1023.3 KB");
}

#[test]
fn fmt_mb_1047893() {
    assert_eq!(app_lib::format_size(1047893u64), "1023.3 KB");
}

#[test]
fn fmt_mb_1047894() {
    assert_eq!(app_lib::format_size(1047894u64), "1023.3 KB");
}

#[test]
fn fmt_mb_1047895() {
    assert_eq!(app_lib::format_size(1047895u64), "1023.3 KB");
}

#[test]
fn fmt_mb_1047896() {
    assert_eq!(app_lib::format_size(1047896u64), "1023.3 KB");
}

#[test]
fn fmt_mb_1047897() {
    assert_eq!(app_lib::format_size(1047897u64), "1023.3 KB");
}

#[test]
fn fmt_mb_1047898() {
    assert_eq!(app_lib::format_size(1047898u64), "1023.3 KB");
}

#[test]
fn fmt_mb_1047899() {
    assert_eq!(app_lib::format_size(1047899u64), "1023.3 KB");
}

#[test]
fn fmt_mb_1047900() {
    assert_eq!(app_lib::format_size(1047900u64), "1023.3 KB");
}

#[test]
fn fmt_mb_1047901() {
    assert_eq!(app_lib::format_size(1047901u64), "1023.3 KB");
}

#[test]
fn fmt_mb_1047902() {
    assert_eq!(app_lib::format_size(1047902u64), "1023.3 KB");
}

#[test]
fn fmt_mb_1047903() {
    assert_eq!(app_lib::format_size(1047903u64), "1023.3 KB");
}

#[test]
fn fmt_mb_1047904() {
    assert_eq!(app_lib::format_size(1047904u64), "1023.3 KB");
}

#[test]
fn fmt_mb_1047905() {
    assert_eq!(app_lib::format_size(1047905u64), "1023.3 KB");
}

#[test]
fn fmt_mb_1047906() {
    assert_eq!(app_lib::format_size(1047906u64), "1023.3 KB");
}

#[test]
fn fmt_mb_1047907() {
    assert_eq!(app_lib::format_size(1047907u64), "1023.3 KB");
}

#[test]
fn fmt_mb_1047908() {
    assert_eq!(app_lib::format_size(1047908u64), "1023.3 KB");
}

#[test]
fn fmt_mb_1047909() {
    assert_eq!(app_lib::format_size(1047909u64), "1023.3 KB");
}

#[test]
fn fmt_mb_1047910() {
    assert_eq!(app_lib::format_size(1047910u64), "1023.3 KB");
}

#[test]
fn fmt_mb_1047911() {
    assert_eq!(app_lib::format_size(1047911u64), "1023.4 KB");
}

#[test]
fn fmt_mb_1047912() {
    assert_eq!(app_lib::format_size(1047912u64), "1023.4 KB");
}

#[test]
fn fmt_mb_1047913() {
    assert_eq!(app_lib::format_size(1047913u64), "1023.4 KB");
}

#[test]
fn fmt_mb_1047914() {
    assert_eq!(app_lib::format_size(1047914u64), "1023.4 KB");
}

#[test]
fn fmt_mb_1047915() {
    assert_eq!(app_lib::format_size(1047915u64), "1023.4 KB");
}

#[test]
fn fmt_mb_1047916() {
    assert_eq!(app_lib::format_size(1047916u64), "1023.4 KB");
}

#[test]
fn fmt_mb_1047917() {
    assert_eq!(app_lib::format_size(1047917u64), "1023.4 KB");
}

#[test]
fn fmt_mb_1047918() {
    assert_eq!(app_lib::format_size(1047918u64), "1023.4 KB");
}

#[test]
fn fmt_mb_1047919() {
    assert_eq!(app_lib::format_size(1047919u64), "1023.4 KB");
}

#[test]
fn fmt_mb_1047920() {
    assert_eq!(app_lib::format_size(1047920u64), "1023.4 KB");
}

#[test]
fn fmt_mb_1047921() {
    assert_eq!(app_lib::format_size(1047921u64), "1023.4 KB");
}

#[test]
fn fmt_mb_1047922() {
    assert_eq!(app_lib::format_size(1047922u64), "1023.4 KB");
}

#[test]
fn fmt_mb_1047923() {
    assert_eq!(app_lib::format_size(1047923u64), "1023.4 KB");
}

#[test]
fn fmt_mb_1047924() {
    assert_eq!(app_lib::format_size(1047924u64), "1023.4 KB");
}

#[test]
fn fmt_mb_1047925() {
    assert_eq!(app_lib::format_size(1047925u64), "1023.4 KB");
}

#[test]
fn fmt_mb_1047926() {
    assert_eq!(app_lib::format_size(1047926u64), "1023.4 KB");
}

#[test]
fn fmt_mb_1047927() {
    assert_eq!(app_lib::format_size(1047927u64), "1023.4 KB");
}

#[test]
fn fmt_mb_1047928() {
    assert_eq!(app_lib::format_size(1047928u64), "1023.4 KB");
}

#[test]
fn fmt_mb_1047929() {
    assert_eq!(app_lib::format_size(1047929u64), "1023.4 KB");
}

#[test]
fn fmt_mb_1047930() {
    assert_eq!(app_lib::format_size(1047930u64), "1023.4 KB");
}

#[test]
fn fmt_mb_1047931() {
    assert_eq!(app_lib::format_size(1047931u64), "1023.4 KB");
}

#[test]
fn fmt_mb_1047932() {
    assert_eq!(app_lib::format_size(1047932u64), "1023.4 KB");
}

#[test]
fn fmt_mb_1047933() {
    assert_eq!(app_lib::format_size(1047933u64), "1023.4 KB");
}

#[test]
fn fmt_mb_1047934() {
    assert_eq!(app_lib::format_size(1047934u64), "1023.4 KB");
}

#[test]
fn fmt_mb_1047935() {
    assert_eq!(app_lib::format_size(1047935u64), "1023.4 KB");
}

#[test]
fn fmt_mb_1047936() {
    assert_eq!(app_lib::format_size(1047936u64), "1023.4 KB");
}

#[test]
fn fmt_mb_1047937() {
    assert_eq!(app_lib::format_size(1047937u64), "1023.4 KB");
}

#[test]
fn fmt_mb_1047938() {
    assert_eq!(app_lib::format_size(1047938u64), "1023.4 KB");
}

#[test]
fn fmt_mb_1047939() {
    assert_eq!(app_lib::format_size(1047939u64), "1023.4 KB");
}

#[test]
fn fmt_mb_1047940() {
    assert_eq!(app_lib::format_size(1047940u64), "1023.4 KB");
}

#[test]
fn fmt_mb_1047941() {
    assert_eq!(app_lib::format_size(1047941u64), "1023.4 KB");
}

#[test]
fn fmt_mb_1047942() {
    assert_eq!(app_lib::format_size(1047942u64), "1023.4 KB");
}

#[test]
fn fmt_mb_1047943() {
    assert_eq!(app_lib::format_size(1047943u64), "1023.4 KB");
}

#[test]
fn fmt_mb_1047944() {
    assert_eq!(app_lib::format_size(1047944u64), "1023.4 KB");
}

#[test]
fn fmt_mb_1047945() {
    assert_eq!(app_lib::format_size(1047945u64), "1023.4 KB");
}

#[test]
fn fmt_mb_1047946() {
    assert_eq!(app_lib::format_size(1047946u64), "1023.4 KB");
}

#[test]
fn fmt_mb_1047947() {
    assert_eq!(app_lib::format_size(1047947u64), "1023.4 KB");
}

#[test]
fn fmt_mb_1047948() {
    assert_eq!(app_lib::format_size(1047948u64), "1023.4 KB");
}

#[test]
fn fmt_mb_1047949() {
    assert_eq!(app_lib::format_size(1047949u64), "1023.4 KB");
}

#[test]
fn fmt_mb_1047950() {
    assert_eq!(app_lib::format_size(1047950u64), "1023.4 KB");
}

#[test]
fn fmt_mb_1047951() {
    assert_eq!(app_lib::format_size(1047951u64), "1023.4 KB");
}

#[test]
fn fmt_mb_1047952() {
    assert_eq!(app_lib::format_size(1047952u64), "1023.4 KB");
}

#[test]
fn fmt_mb_1047953() {
    assert_eq!(app_lib::format_size(1047953u64), "1023.4 KB");
}

#[test]
fn fmt_mb_1047954() {
    assert_eq!(app_lib::format_size(1047954u64), "1023.4 KB");
}

#[test]
fn fmt_mb_1047955() {
    assert_eq!(app_lib::format_size(1047955u64), "1023.4 KB");
}

#[test]
fn fmt_mb_1047956() {
    assert_eq!(app_lib::format_size(1047956u64), "1023.4 KB");
}

#[test]
fn fmt_mb_1047957() {
    assert_eq!(app_lib::format_size(1047957u64), "1023.4 KB");
}

#[test]
fn fmt_mb_1047958() {
    assert_eq!(app_lib::format_size(1047958u64), "1023.4 KB");
}

#[test]
fn fmt_mb_1047959() {
    assert_eq!(app_lib::format_size(1047959u64), "1023.4 KB");
}

#[test]
fn fmt_mb_1047960() {
    assert_eq!(app_lib::format_size(1047960u64), "1023.4 KB");
}

#[test]
fn fmt_mb_1047961() {
    assert_eq!(app_lib::format_size(1047961u64), "1023.4 KB");
}

#[test]
fn fmt_mb_1047962() {
    assert_eq!(app_lib::format_size(1047962u64), "1023.4 KB");
}

#[test]
fn fmt_mb_1047963() {
    assert_eq!(app_lib::format_size(1047963u64), "1023.4 KB");
}

#[test]
fn fmt_mb_1047964() {
    assert_eq!(app_lib::format_size(1047964u64), "1023.4 KB");
}

#[test]
fn fmt_mb_1047965() {
    assert_eq!(app_lib::format_size(1047965u64), "1023.4 KB");
}

#[test]
fn fmt_mb_1047966() {
    assert_eq!(app_lib::format_size(1047966u64), "1023.4 KB");
}

#[test]
fn fmt_mb_1047967() {
    assert_eq!(app_lib::format_size(1047967u64), "1023.4 KB");
}

#[test]
fn fmt_mb_1047968() {
    assert_eq!(app_lib::format_size(1047968u64), "1023.4 KB");
}

#[test]
fn fmt_mb_1047969() {
    assert_eq!(app_lib::format_size(1047969u64), "1023.4 KB");
}

#[test]
fn fmt_mb_1047970() {
    assert_eq!(app_lib::format_size(1047970u64), "1023.4 KB");
}

#[test]
fn fmt_mb_1047971() {
    assert_eq!(app_lib::format_size(1047971u64), "1023.4 KB");
}

#[test]
fn fmt_mb_1047972() {
    assert_eq!(app_lib::format_size(1047972u64), "1023.4 KB");
}

#[test]
fn fmt_mb_1047973() {
    assert_eq!(app_lib::format_size(1047973u64), "1023.4 KB");
}

#[test]
fn fmt_mb_1047974() {
    assert_eq!(app_lib::format_size(1047974u64), "1023.4 KB");
}

#[test]
fn fmt_mb_1047975() {
    assert_eq!(app_lib::format_size(1047975u64), "1023.4 KB");
}

#[test]
fn fmt_mb_1047976() {
    assert_eq!(app_lib::format_size(1047976u64), "1023.4 KB");
}

#[test]
fn fmt_mb_1047977() {
    assert_eq!(app_lib::format_size(1047977u64), "1023.4 KB");
}

#[test]
fn fmt_mb_1047978() {
    assert_eq!(app_lib::format_size(1047978u64), "1023.4 KB");
}

#[test]
fn fmt_mb_1047979() {
    assert_eq!(app_lib::format_size(1047979u64), "1023.4 KB");
}

#[test]
fn fmt_mb_1047980() {
    assert_eq!(app_lib::format_size(1047980u64), "1023.4 KB");
}

#[test]
fn fmt_mb_1047981() {
    assert_eq!(app_lib::format_size(1047981u64), "1023.4 KB");
}

#[test]
fn fmt_mb_1047982() {
    assert_eq!(app_lib::format_size(1047982u64), "1023.4 KB");
}

#[test]
fn fmt_mb_1047983() {
    assert_eq!(app_lib::format_size(1047983u64), "1023.4 KB");
}

#[test]
fn fmt_mb_1047984() {
    assert_eq!(app_lib::format_size(1047984u64), "1023.4 KB");
}

#[test]
fn fmt_mb_1047985() {
    assert_eq!(app_lib::format_size(1047985u64), "1023.4 KB");
}

#[test]
fn fmt_mb_1047986() {
    assert_eq!(app_lib::format_size(1047986u64), "1023.4 KB");
}

#[test]
fn fmt_mb_1047987() {
    assert_eq!(app_lib::format_size(1047987u64), "1023.4 KB");
}

#[test]
fn fmt_mb_1047988() {
    assert_eq!(app_lib::format_size(1047988u64), "1023.4 KB");
}

#[test]
fn fmt_mb_1047989() {
    assert_eq!(app_lib::format_size(1047989u64), "1023.4 KB");
}

#[test]
fn fmt_mb_1047990() {
    assert_eq!(app_lib::format_size(1047990u64), "1023.4 KB");
}

#[test]
fn fmt_mb_1047991() {
    assert_eq!(app_lib::format_size(1047991u64), "1023.4 KB");
}

#[test]
fn fmt_mb_1047992() {
    assert_eq!(app_lib::format_size(1047992u64), "1023.4 KB");
}

#[test]
fn fmt_mb_1047993() {
    assert_eq!(app_lib::format_size(1047993u64), "1023.4 KB");
}

#[test]
fn fmt_mb_1047994() {
    assert_eq!(app_lib::format_size(1047994u64), "1023.4 KB");
}

#[test]
fn fmt_mb_1047995() {
    assert_eq!(app_lib::format_size(1047995u64), "1023.4 KB");
}

#[test]
fn fmt_mb_1047996() {
    assert_eq!(app_lib::format_size(1047996u64), "1023.4 KB");
}

#[test]
fn fmt_mb_1047997() {
    assert_eq!(app_lib::format_size(1047997u64), "1023.4 KB");
}

#[test]
fn fmt_mb_1047998() {
    assert_eq!(app_lib::format_size(1047998u64), "1023.4 KB");
}

#[test]
fn fmt_mb_1047999() {
    assert_eq!(app_lib::format_size(1047999u64), "1023.4 KB");
}

#[test]
fn fmt_mb_1048000() {
    assert_eq!(app_lib::format_size(1048000u64), "1023.4 KB");
}

#[test]
fn fmt_mb_1048001() {
    assert_eq!(app_lib::format_size(1048001u64), "1023.4 KB");
}

#[test]
fn fmt_mb_1048002() {
    assert_eq!(app_lib::format_size(1048002u64), "1023.4 KB");
}

#[test]
fn fmt_mb_1048003() {
    assert_eq!(app_lib::format_size(1048003u64), "1023.4 KB");
}

#[test]
fn fmt_mb_1048004() {
    assert_eq!(app_lib::format_size(1048004u64), "1023.4 KB");
}

#[test]
fn fmt_mb_1048005() {
    assert_eq!(app_lib::format_size(1048005u64), "1023.4 KB");
}

#[test]
fn fmt_mb_1048006() {
    assert_eq!(app_lib::format_size(1048006u64), "1023.4 KB");
}

#[test]
fn fmt_mb_1048007() {
    assert_eq!(app_lib::format_size(1048007u64), "1023.4 KB");
}

#[test]
fn fmt_mb_1048008() {
    assert_eq!(app_lib::format_size(1048008u64), "1023.4 KB");
}

#[test]
fn fmt_mb_1048009() {
    assert_eq!(app_lib::format_size(1048009u64), "1023.4 KB");
}

#[test]
fn fmt_mb_1048010() {
    assert_eq!(app_lib::format_size(1048010u64), "1023.4 KB");
}

#[test]
fn fmt_mb_1048011() {
    assert_eq!(app_lib::format_size(1048011u64), "1023.4 KB");
}

#[test]
fn fmt_mb_1048012() {
    assert_eq!(app_lib::format_size(1048012u64), "1023.4 KB");
}

#[test]
fn fmt_mb_1048013() {
    assert_eq!(app_lib::format_size(1048013u64), "1023.5 KB");
}

#[test]
fn fmt_mb_1048014() {
    assert_eq!(app_lib::format_size(1048014u64), "1023.5 KB");
}

#[test]
fn fmt_mb_1048015() {
    assert_eq!(app_lib::format_size(1048015u64), "1023.5 KB");
}

#[test]
fn fmt_mb_1048016() {
    assert_eq!(app_lib::format_size(1048016u64), "1023.5 KB");
}

#[test]
fn fmt_mb_1048017() {
    assert_eq!(app_lib::format_size(1048017u64), "1023.5 KB");
}

#[test]
fn fmt_mb_1048018() {
    assert_eq!(app_lib::format_size(1048018u64), "1023.5 KB");
}

#[test]
fn fmt_mb_1048019() {
    assert_eq!(app_lib::format_size(1048019u64), "1023.5 KB");
}

#[test]
fn fmt_mb_1048020() {
    assert_eq!(app_lib::format_size(1048020u64), "1023.5 KB");
}

#[test]
fn fmt_mb_1048021() {
    assert_eq!(app_lib::format_size(1048021u64), "1023.5 KB");
}

#[test]
fn fmt_mb_1048022() {
    assert_eq!(app_lib::format_size(1048022u64), "1023.5 KB");
}

#[test]
fn fmt_mb_1048023() {
    assert_eq!(app_lib::format_size(1048023u64), "1023.5 KB");
}

#[test]
fn fmt_mb_1048024() {
    assert_eq!(app_lib::format_size(1048024u64), "1023.5 KB");
}

#[test]
fn fmt_mb_1048025() {
    assert_eq!(app_lib::format_size(1048025u64), "1023.5 KB");
}

#[test]
fn fmt_mb_1048026() {
    assert_eq!(app_lib::format_size(1048026u64), "1023.5 KB");
}

#[test]
fn fmt_mb_1048027() {
    assert_eq!(app_lib::format_size(1048027u64), "1023.5 KB");
}

#[test]
fn fmt_mb_1048028() {
    assert_eq!(app_lib::format_size(1048028u64), "1023.5 KB");
}

#[test]
fn fmt_mb_1048029() {
    assert_eq!(app_lib::format_size(1048029u64), "1023.5 KB");
}

#[test]
fn fmt_mb_1048030() {
    assert_eq!(app_lib::format_size(1048030u64), "1023.5 KB");
}

#[test]
fn fmt_mb_1048031() {
    assert_eq!(app_lib::format_size(1048031u64), "1023.5 KB");
}

#[test]
fn fmt_mb_1048032() {
    assert_eq!(app_lib::format_size(1048032u64), "1023.5 KB");
}

#[test]
fn fmt_mb_1048033() {
    assert_eq!(app_lib::format_size(1048033u64), "1023.5 KB");
}

#[test]
fn fmt_mb_1048034() {
    assert_eq!(app_lib::format_size(1048034u64), "1023.5 KB");
}

#[test]
fn fmt_mb_1048035() {
    assert_eq!(app_lib::format_size(1048035u64), "1023.5 KB");
}

#[test]
fn fmt_mb_1048036() {
    assert_eq!(app_lib::format_size(1048036u64), "1023.5 KB");
}

#[test]
fn fmt_mb_1048037() {
    assert_eq!(app_lib::format_size(1048037u64), "1023.5 KB");
}

#[test]
fn fmt_mb_1048038() {
    assert_eq!(app_lib::format_size(1048038u64), "1023.5 KB");
}

#[test]
fn fmt_mb_1048039() {
    assert_eq!(app_lib::format_size(1048039u64), "1023.5 KB");
}

#[test]
fn fmt_mb_1048040() {
    assert_eq!(app_lib::format_size(1048040u64), "1023.5 KB");
}

#[test]
fn fmt_mb_1048041() {
    assert_eq!(app_lib::format_size(1048041u64), "1023.5 KB");
}

#[test]
fn fmt_mb_1048042() {
    assert_eq!(app_lib::format_size(1048042u64), "1023.5 KB");
}

#[test]
fn fmt_mb_1048043() {
    assert_eq!(app_lib::format_size(1048043u64), "1023.5 KB");
}

#[test]
fn fmt_mb_1048044() {
    assert_eq!(app_lib::format_size(1048044u64), "1023.5 KB");
}

#[test]
fn fmt_mb_1048045() {
    assert_eq!(app_lib::format_size(1048045u64), "1023.5 KB");
}

#[test]
fn fmt_mb_1048046() {
    assert_eq!(app_lib::format_size(1048046u64), "1023.5 KB");
}

#[test]
fn fmt_mb_1048047() {
    assert_eq!(app_lib::format_size(1048047u64), "1023.5 KB");
}

#[test]
fn fmt_mb_1048048() {
    assert_eq!(app_lib::format_size(1048048u64), "1023.5 KB");
}

#[test]
fn fmt_mb_1048049() {
    assert_eq!(app_lib::format_size(1048049u64), "1023.5 KB");
}

#[test]
fn fmt_mb_1048050() {
    assert_eq!(app_lib::format_size(1048050u64), "1023.5 KB");
}

#[test]
fn fmt_mb_1048051() {
    assert_eq!(app_lib::format_size(1048051u64), "1023.5 KB");
}

#[test]
fn fmt_mb_1048052() {
    assert_eq!(app_lib::format_size(1048052u64), "1023.5 KB");
}

#[test]
fn fmt_mb_1048053() {
    assert_eq!(app_lib::format_size(1048053u64), "1023.5 KB");
}

#[test]
fn fmt_mb_1048054() {
    assert_eq!(app_lib::format_size(1048054u64), "1023.5 KB");
}

#[test]
fn fmt_mb_1048055() {
    assert_eq!(app_lib::format_size(1048055u64), "1023.5 KB");
}

#[test]
fn fmt_mb_1048056() {
    assert_eq!(app_lib::format_size(1048056u64), "1023.5 KB");
}

#[test]
fn fmt_mb_1048057() {
    assert_eq!(app_lib::format_size(1048057u64), "1023.5 KB");
}

#[test]
fn fmt_mb_1048058() {
    assert_eq!(app_lib::format_size(1048058u64), "1023.5 KB");
}

#[test]
fn fmt_mb_1048059() {
    assert_eq!(app_lib::format_size(1048059u64), "1023.5 KB");
}

#[test]
fn fmt_mb_1048060() {
    assert_eq!(app_lib::format_size(1048060u64), "1023.5 KB");
}

#[test]
fn fmt_mb_1048061() {
    assert_eq!(app_lib::format_size(1048061u64), "1023.5 KB");
}

#[test]
fn fmt_mb_1048062() {
    assert_eq!(app_lib::format_size(1048062u64), "1023.5 KB");
}

#[test]
fn fmt_mb_1048063() {
    assert_eq!(app_lib::format_size(1048063u64), "1023.5 KB");
}

#[test]
fn fmt_mb_1048064() {
    assert_eq!(app_lib::format_size(1048064u64), "1023.5 KB");
}

#[test]
fn fmt_mb_1048065() {
    assert_eq!(app_lib::format_size(1048065u64), "1023.5 KB");
}

#[test]
fn fmt_mb_1048066() {
    assert_eq!(app_lib::format_size(1048066u64), "1023.5 KB");
}

#[test]
fn fmt_mb_1048067() {
    assert_eq!(app_lib::format_size(1048067u64), "1023.5 KB");
}

#[test]
fn fmt_mb_1048068() {
    assert_eq!(app_lib::format_size(1048068u64), "1023.5 KB");
}

#[test]
fn fmt_mb_1048069() {
    assert_eq!(app_lib::format_size(1048069u64), "1023.5 KB");
}

#[test]
fn fmt_mb_1048070() {
    assert_eq!(app_lib::format_size(1048070u64), "1023.5 KB");
}

#[test]
fn fmt_mb_1048071() {
    assert_eq!(app_lib::format_size(1048071u64), "1023.5 KB");
}

#[test]
fn fmt_mb_1048072() {
    assert_eq!(app_lib::format_size(1048072u64), "1023.5 KB");
}

#[test]
fn fmt_mb_1048073() {
    assert_eq!(app_lib::format_size(1048073u64), "1023.5 KB");
}

#[test]
fn fmt_mb_1048074() {
    assert_eq!(app_lib::format_size(1048074u64), "1023.5 KB");
}

#[test]
fn fmt_mb_1048075() {
    assert_eq!(app_lib::format_size(1048075u64), "1023.5 KB");
}

#[test]
fn fmt_mb_1048076() {
    assert_eq!(app_lib::format_size(1048076u64), "1023.5 KB");
}

#[test]
fn fmt_mb_1048077() {
    assert_eq!(app_lib::format_size(1048077u64), "1023.5 KB");
}

#[test]
fn fmt_mb_1048078() {
    assert_eq!(app_lib::format_size(1048078u64), "1023.5 KB");
}

#[test]
fn fmt_mb_1048079() {
    assert_eq!(app_lib::format_size(1048079u64), "1023.5 KB");
}

#[test]
fn fmt_mb_1048080() {
    assert_eq!(app_lib::format_size(1048080u64), "1023.5 KB");
}

#[test]
fn fmt_mb_1048081() {
    assert_eq!(app_lib::format_size(1048081u64), "1023.5 KB");
}

#[test]
fn fmt_mb_1048082() {
    assert_eq!(app_lib::format_size(1048082u64), "1023.5 KB");
}

#[test]
fn fmt_mb_1048083() {
    assert_eq!(app_lib::format_size(1048083u64), "1023.5 KB");
}

#[test]
fn fmt_mb_1048084() {
    assert_eq!(app_lib::format_size(1048084u64), "1023.5 KB");
}

#[test]
fn fmt_mb_1048085() {
    assert_eq!(app_lib::format_size(1048085u64), "1023.5 KB");
}

#[test]
fn fmt_mb_1048086() {
    assert_eq!(app_lib::format_size(1048086u64), "1023.5 KB");
}

#[test]
fn fmt_mb_1048087() {
    assert_eq!(app_lib::format_size(1048087u64), "1023.5 KB");
}

#[test]
fn fmt_mb_1048088() {
    assert_eq!(app_lib::format_size(1048088u64), "1023.5 KB");
}

#[test]
fn fmt_mb_1048089() {
    assert_eq!(app_lib::format_size(1048089u64), "1023.5 KB");
}

#[test]
fn fmt_mb_1048090() {
    assert_eq!(app_lib::format_size(1048090u64), "1023.5 KB");
}

#[test]
fn fmt_mb_1048091() {
    assert_eq!(app_lib::format_size(1048091u64), "1023.5 KB");
}

#[test]
fn fmt_mb_1048092() {
    assert_eq!(app_lib::format_size(1048092u64), "1023.5 KB");
}

#[test]
fn fmt_mb_1048093() {
    assert_eq!(app_lib::format_size(1048093u64), "1023.5 KB");
}

#[test]
fn fmt_mb_1048094() {
    assert_eq!(app_lib::format_size(1048094u64), "1023.5 KB");
}

#[test]
fn fmt_mb_1048095() {
    assert_eq!(app_lib::format_size(1048095u64), "1023.5 KB");
}

#[test]
fn fmt_mb_1048096() {
    assert_eq!(app_lib::format_size(1048096u64), "1023.5 KB");
}

#[test]
fn fmt_mb_1048097() {
    assert_eq!(app_lib::format_size(1048097u64), "1023.5 KB");
}

#[test]
fn fmt_mb_1048098() {
    assert_eq!(app_lib::format_size(1048098u64), "1023.5 KB");
}

#[test]
fn fmt_mb_1048099() {
    assert_eq!(app_lib::format_size(1048099u64), "1023.5 KB");
}

#[test]
fn fmt_mb_1048100() {
    assert_eq!(app_lib::format_size(1048100u64), "1023.5 KB");
}

#[test]
fn fmt_mb_1048101() {
    assert_eq!(app_lib::format_size(1048101u64), "1023.5 KB");
}

#[test]
fn fmt_mb_1048102() {
    assert_eq!(app_lib::format_size(1048102u64), "1023.5 KB");
}

#[test]
fn fmt_mb_1048103() {
    assert_eq!(app_lib::format_size(1048103u64), "1023.5 KB");
}

#[test]
fn fmt_mb_1048104() {
    assert_eq!(app_lib::format_size(1048104u64), "1023.5 KB");
}

#[test]
fn fmt_mb_1048105() {
    assert_eq!(app_lib::format_size(1048105u64), "1023.5 KB");
}

#[test]
fn fmt_mb_1048106() {
    assert_eq!(app_lib::format_size(1048106u64), "1023.5 KB");
}

#[test]
fn fmt_mb_1048107() {
    assert_eq!(app_lib::format_size(1048107u64), "1023.5 KB");
}

#[test]
fn fmt_mb_1048108() {
    assert_eq!(app_lib::format_size(1048108u64), "1023.5 KB");
}

#[test]
fn fmt_mb_1048109() {
    assert_eq!(app_lib::format_size(1048109u64), "1023.5 KB");
}

#[test]
fn fmt_mb_1048110() {
    assert_eq!(app_lib::format_size(1048110u64), "1023.5 KB");
}

#[test]
fn fmt_mb_1048111() {
    assert_eq!(app_lib::format_size(1048111u64), "1023.5 KB");
}

#[test]
fn fmt_mb_1048112() {
    assert_eq!(app_lib::format_size(1048112u64), "1023.5 KB");
}

#[test]
fn fmt_mb_1048113() {
    assert_eq!(app_lib::format_size(1048113u64), "1023.5 KB");
}

#[test]
fn fmt_mb_1048114() {
    assert_eq!(app_lib::format_size(1048114u64), "1023.5 KB");
}

#[test]
fn fmt_mb_1048115() {
    assert_eq!(app_lib::format_size(1048115u64), "1023.5 KB");
}

#[test]
fn fmt_mb_1048116() {
    assert_eq!(app_lib::format_size(1048116u64), "1023.6 KB");
}

#[test]
fn fmt_mb_1048117() {
    assert_eq!(app_lib::format_size(1048117u64), "1023.6 KB");
}

#[test]
fn fmt_mb_1048118() {
    assert_eq!(app_lib::format_size(1048118u64), "1023.6 KB");
}

#[test]
fn fmt_mb_1048119() {
    assert_eq!(app_lib::format_size(1048119u64), "1023.6 KB");
}

#[test]
fn fmt_mb_1048120() {
    assert_eq!(app_lib::format_size(1048120u64), "1023.6 KB");
}

#[test]
fn fmt_mb_1048121() {
    assert_eq!(app_lib::format_size(1048121u64), "1023.6 KB");
}

#[test]
fn fmt_mb_1048122() {
    assert_eq!(app_lib::format_size(1048122u64), "1023.6 KB");
}

#[test]
fn fmt_mb_1048123() {
    assert_eq!(app_lib::format_size(1048123u64), "1023.6 KB");
}

#[test]
fn fmt_mb_1048124() {
    assert_eq!(app_lib::format_size(1048124u64), "1023.6 KB");
}

#[test]
fn fmt_mb_1048125() {
    assert_eq!(app_lib::format_size(1048125u64), "1023.6 KB");
}

#[test]
fn fmt_mb_1048126() {
    assert_eq!(app_lib::format_size(1048126u64), "1023.6 KB");
}

#[test]
fn fmt_mb_1048127() {
    assert_eq!(app_lib::format_size(1048127u64), "1023.6 KB");
}

#[test]
fn fmt_mb_1048128() {
    assert_eq!(app_lib::format_size(1048128u64), "1023.6 KB");
}

#[test]
fn fmt_mb_1048129() {
    assert_eq!(app_lib::format_size(1048129u64), "1023.6 KB");
}

#[test]
fn fmt_mb_1048130() {
    assert_eq!(app_lib::format_size(1048130u64), "1023.6 KB");
}

#[test]
fn fmt_mb_1048131() {
    assert_eq!(app_lib::format_size(1048131u64), "1023.6 KB");
}

#[test]
fn fmt_mb_1048132() {
    assert_eq!(app_lib::format_size(1048132u64), "1023.6 KB");
}

#[test]
fn fmt_mb_1048133() {
    assert_eq!(app_lib::format_size(1048133u64), "1023.6 KB");
}

#[test]
fn fmt_mb_1048134() {
    assert_eq!(app_lib::format_size(1048134u64), "1023.6 KB");
}

#[test]
fn fmt_mb_1048135() {
    assert_eq!(app_lib::format_size(1048135u64), "1023.6 KB");
}

#[test]
fn fmt_mb_1048136() {
    assert_eq!(app_lib::format_size(1048136u64), "1023.6 KB");
}

#[test]
fn fmt_mb_1048137() {
    assert_eq!(app_lib::format_size(1048137u64), "1023.6 KB");
}

#[test]
fn fmt_mb_1048138() {
    assert_eq!(app_lib::format_size(1048138u64), "1023.6 KB");
}

#[test]
fn fmt_mb_1048139() {
    assert_eq!(app_lib::format_size(1048139u64), "1023.6 KB");
}

#[test]
fn fmt_mb_1048140() {
    assert_eq!(app_lib::format_size(1048140u64), "1023.6 KB");
}

#[test]
fn fmt_mb_1048141() {
    assert_eq!(app_lib::format_size(1048141u64), "1023.6 KB");
}

#[test]
fn fmt_mb_1048142() {
    assert_eq!(app_lib::format_size(1048142u64), "1023.6 KB");
}

#[test]
fn fmt_mb_1048143() {
    assert_eq!(app_lib::format_size(1048143u64), "1023.6 KB");
}

#[test]
fn fmt_mb_1048144() {
    assert_eq!(app_lib::format_size(1048144u64), "1023.6 KB");
}

#[test]
fn fmt_mb_1048145() {
    assert_eq!(app_lib::format_size(1048145u64), "1023.6 KB");
}

#[test]
fn fmt_mb_1048146() {
    assert_eq!(app_lib::format_size(1048146u64), "1023.6 KB");
}

#[test]
fn fmt_mb_1048147() {
    assert_eq!(app_lib::format_size(1048147u64), "1023.6 KB");
}

#[test]
fn fmt_mb_1048148() {
    assert_eq!(app_lib::format_size(1048148u64), "1023.6 KB");
}

#[test]
fn fmt_mb_1048149() {
    assert_eq!(app_lib::format_size(1048149u64), "1023.6 KB");
}

#[test]
fn fmt_mb_1048150() {
    assert_eq!(app_lib::format_size(1048150u64), "1023.6 KB");
}

#[test]
fn fmt_mb_1048151() {
    assert_eq!(app_lib::format_size(1048151u64), "1023.6 KB");
}

#[test]
fn fmt_mb_1048152() {
    assert_eq!(app_lib::format_size(1048152u64), "1023.6 KB");
}

#[test]
fn fmt_mb_1048153() {
    assert_eq!(app_lib::format_size(1048153u64), "1023.6 KB");
}

#[test]
fn fmt_mb_1048154() {
    assert_eq!(app_lib::format_size(1048154u64), "1023.6 KB");
}

#[test]
fn fmt_mb_1048155() {
    assert_eq!(app_lib::format_size(1048155u64), "1023.6 KB");
}

#[test]
fn fmt_mb_1048156() {
    assert_eq!(app_lib::format_size(1048156u64), "1023.6 KB");
}

#[test]
fn fmt_mb_1048157() {
    assert_eq!(app_lib::format_size(1048157u64), "1023.6 KB");
}

#[test]
fn fmt_mb_1048158() {
    assert_eq!(app_lib::format_size(1048158u64), "1023.6 KB");
}

#[test]
fn fmt_mb_1048159() {
    assert_eq!(app_lib::format_size(1048159u64), "1023.6 KB");
}

#[test]
fn fmt_mb_1048160() {
    assert_eq!(app_lib::format_size(1048160u64), "1023.6 KB");
}

#[test]
fn fmt_mb_1048161() {
    assert_eq!(app_lib::format_size(1048161u64), "1023.6 KB");
}

#[test]
fn fmt_mb_1048162() {
    assert_eq!(app_lib::format_size(1048162u64), "1023.6 KB");
}

#[test]
fn fmt_mb_1048163() {
    assert_eq!(app_lib::format_size(1048163u64), "1023.6 KB");
}

#[test]
fn fmt_mb_1048164() {
    assert_eq!(app_lib::format_size(1048164u64), "1023.6 KB");
}

#[test]
fn fmt_mb_1048165() {
    assert_eq!(app_lib::format_size(1048165u64), "1023.6 KB");
}

#[test]
fn fmt_mb_1048166() {
    assert_eq!(app_lib::format_size(1048166u64), "1023.6 KB");
}

#[test]
fn fmt_mb_1048167() {
    assert_eq!(app_lib::format_size(1048167u64), "1023.6 KB");
}

#[test]
fn fmt_mb_1048168() {
    assert_eq!(app_lib::format_size(1048168u64), "1023.6 KB");
}

#[test]
fn fmt_mb_1048169() {
    assert_eq!(app_lib::format_size(1048169u64), "1023.6 KB");
}

#[test]
fn fmt_mb_1048170() {
    assert_eq!(app_lib::format_size(1048170u64), "1023.6 KB");
}

#[test]
fn fmt_mb_1048171() {
    assert_eq!(app_lib::format_size(1048171u64), "1023.6 KB");
}

#[test]
fn fmt_mb_1048172() {
    assert_eq!(app_lib::format_size(1048172u64), "1023.6 KB");
}

#[test]
fn fmt_mb_1048173() {
    assert_eq!(app_lib::format_size(1048173u64), "1023.6 KB");
}

#[test]
fn fmt_mb_1048174() {
    assert_eq!(app_lib::format_size(1048174u64), "1023.6 KB");
}

#[test]
fn fmt_mb_1048175() {
    assert_eq!(app_lib::format_size(1048175u64), "1023.6 KB");
}

#[test]
fn fmt_mb_1048176() {
    assert_eq!(app_lib::format_size(1048176u64), "1023.6 KB");
}

#[test]
fn fmt_mb_1048177() {
    assert_eq!(app_lib::format_size(1048177u64), "1023.6 KB");
}

#[test]
fn fmt_mb_1048178() {
    assert_eq!(app_lib::format_size(1048178u64), "1023.6 KB");
}

#[test]
fn fmt_mb_1048179() {
    assert_eq!(app_lib::format_size(1048179u64), "1023.6 KB");
}

#[test]
fn fmt_mb_1048180() {
    assert_eq!(app_lib::format_size(1048180u64), "1023.6 KB");
}

#[test]
fn fmt_mb_1048181() {
    assert_eq!(app_lib::format_size(1048181u64), "1023.6 KB");
}

#[test]
fn fmt_mb_1048182() {
    assert_eq!(app_lib::format_size(1048182u64), "1023.6 KB");
}

#[test]
fn fmt_mb_1048183() {
    assert_eq!(app_lib::format_size(1048183u64), "1023.6 KB");
}

#[test]
fn fmt_mb_1048184() {
    assert_eq!(app_lib::format_size(1048184u64), "1023.6 KB");
}

#[test]
fn fmt_mb_1048185() {
    assert_eq!(app_lib::format_size(1048185u64), "1023.6 KB");
}

#[test]
fn fmt_mb_1048186() {
    assert_eq!(app_lib::format_size(1048186u64), "1023.6 KB");
}

#[test]
fn fmt_mb_1048187() {
    assert_eq!(app_lib::format_size(1048187u64), "1023.6 KB");
}

#[test]
fn fmt_mb_1048188() {
    assert_eq!(app_lib::format_size(1048188u64), "1023.6 KB");
}

#[test]
fn fmt_mb_1048189() {
    assert_eq!(app_lib::format_size(1048189u64), "1023.6 KB");
}

#[test]
fn fmt_mb_1048190() {
    assert_eq!(app_lib::format_size(1048190u64), "1023.6 KB");
}

#[test]
fn fmt_mb_1048191() {
    assert_eq!(app_lib::format_size(1048191u64), "1023.6 KB");
}

#[test]
fn fmt_mb_1048192() {
    assert_eq!(app_lib::format_size(1048192u64), "1023.6 KB");
}

#[test]
fn fmt_mb_1048193() {
    assert_eq!(app_lib::format_size(1048193u64), "1023.6 KB");
}

#[test]
fn fmt_mb_1048194() {
    assert_eq!(app_lib::format_size(1048194u64), "1023.6 KB");
}

#[test]
fn fmt_mb_1048195() {
    assert_eq!(app_lib::format_size(1048195u64), "1023.6 KB");
}

#[test]
fn fmt_mb_1048196() {
    assert_eq!(app_lib::format_size(1048196u64), "1023.6 KB");
}

#[test]
fn fmt_mb_1048197() {
    assert_eq!(app_lib::format_size(1048197u64), "1023.6 KB");
}

#[test]
fn fmt_mb_1048198() {
    assert_eq!(app_lib::format_size(1048198u64), "1023.6 KB");
}

#[test]
fn fmt_mb_1048199() {
    assert_eq!(app_lib::format_size(1048199u64), "1023.6 KB");
}

#[test]
fn fmt_mb_1048200() {
    assert_eq!(app_lib::format_size(1048200u64), "1023.6 KB");
}

#[test]
fn fmt_mb_1048201() {
    assert_eq!(app_lib::format_size(1048201u64), "1023.6 KB");
}

#[test]
fn fmt_mb_1048202() {
    assert_eq!(app_lib::format_size(1048202u64), "1023.6 KB");
}

#[test]
fn fmt_mb_1048203() {
    assert_eq!(app_lib::format_size(1048203u64), "1023.6 KB");
}

#[test]
fn fmt_mb_1048204() {
    assert_eq!(app_lib::format_size(1048204u64), "1023.6 KB");
}

#[test]
fn fmt_mb_1048205() {
    assert_eq!(app_lib::format_size(1048205u64), "1023.6 KB");
}

#[test]
fn fmt_mb_1048206() {
    assert_eq!(app_lib::format_size(1048206u64), "1023.6 KB");
}

#[test]
fn fmt_mb_1048207() {
    assert_eq!(app_lib::format_size(1048207u64), "1023.6 KB");
}

#[test]
fn fmt_mb_1048208() {
    assert_eq!(app_lib::format_size(1048208u64), "1023.6 KB");
}

#[test]
fn fmt_mb_1048209() {
    assert_eq!(app_lib::format_size(1048209u64), "1023.6 KB");
}

#[test]
fn fmt_mb_1048210() {
    assert_eq!(app_lib::format_size(1048210u64), "1023.6 KB");
}

#[test]
fn fmt_mb_1048211() {
    assert_eq!(app_lib::format_size(1048211u64), "1023.6 KB");
}

#[test]
fn fmt_mb_1048212() {
    assert_eq!(app_lib::format_size(1048212u64), "1023.6 KB");
}

#[test]
fn fmt_mb_1048213() {
    assert_eq!(app_lib::format_size(1048213u64), "1023.6 KB");
}

#[test]
fn fmt_mb_1048214() {
    assert_eq!(app_lib::format_size(1048214u64), "1023.6 KB");
}

#[test]
fn fmt_mb_1048215() {
    assert_eq!(app_lib::format_size(1048215u64), "1023.6 KB");
}

#[test]
fn fmt_mb_1048216() {
    assert_eq!(app_lib::format_size(1048216u64), "1023.6 KB");
}

#[test]
fn fmt_mb_1048217() {
    assert_eq!(app_lib::format_size(1048217u64), "1023.6 KB");
}

#[test]
fn fmt_mb_1048218() {
    assert_eq!(app_lib::format_size(1048218u64), "1023.7 KB");
}

#[test]
fn fmt_mb_1048219() {
    assert_eq!(app_lib::format_size(1048219u64), "1023.7 KB");
}

#[test]
fn fmt_mb_1048220() {
    assert_eq!(app_lib::format_size(1048220u64), "1023.7 KB");
}

#[test]
fn fmt_mb_1048221() {
    assert_eq!(app_lib::format_size(1048221u64), "1023.7 KB");
}

#[test]
fn fmt_mb_1048222() {
    assert_eq!(app_lib::format_size(1048222u64), "1023.7 KB");
}

#[test]
fn fmt_mb_1048223() {
    assert_eq!(app_lib::format_size(1048223u64), "1023.7 KB");
}

#[test]
fn fmt_mb_1048224() {
    assert_eq!(app_lib::format_size(1048224u64), "1023.7 KB");
}

#[test]
fn fmt_mb_1048225() {
    assert_eq!(app_lib::format_size(1048225u64), "1023.7 KB");
}

#[test]
fn fmt_mb_1048226() {
    assert_eq!(app_lib::format_size(1048226u64), "1023.7 KB");
}

#[test]
fn fmt_mb_1048227() {
    assert_eq!(app_lib::format_size(1048227u64), "1023.7 KB");
}

#[test]
fn fmt_mb_1048228() {
    assert_eq!(app_lib::format_size(1048228u64), "1023.7 KB");
}

#[test]
fn fmt_mb_1048229() {
    assert_eq!(app_lib::format_size(1048229u64), "1023.7 KB");
}

#[test]
fn fmt_mb_1048230() {
    assert_eq!(app_lib::format_size(1048230u64), "1023.7 KB");
}

#[test]
fn fmt_mb_1048231() {
    assert_eq!(app_lib::format_size(1048231u64), "1023.7 KB");
}

#[test]
fn fmt_mb_1048232() {
    assert_eq!(app_lib::format_size(1048232u64), "1023.7 KB");
}

#[test]
fn fmt_mb_1048233() {
    assert_eq!(app_lib::format_size(1048233u64), "1023.7 KB");
}

#[test]
fn fmt_mb_1048234() {
    assert_eq!(app_lib::format_size(1048234u64), "1023.7 KB");
}

#[test]
fn fmt_mb_1048235() {
    assert_eq!(app_lib::format_size(1048235u64), "1023.7 KB");
}

#[test]
fn fmt_mb_1048236() {
    assert_eq!(app_lib::format_size(1048236u64), "1023.7 KB");
}

#[test]
fn fmt_mb_1048237() {
    assert_eq!(app_lib::format_size(1048237u64), "1023.7 KB");
}

#[test]
fn fmt_mb_1048238() {
    assert_eq!(app_lib::format_size(1048238u64), "1023.7 KB");
}

#[test]
fn fmt_mb_1048239() {
    assert_eq!(app_lib::format_size(1048239u64), "1023.7 KB");
}

#[test]
fn fmt_mb_1048240() {
    assert_eq!(app_lib::format_size(1048240u64), "1023.7 KB");
}

#[test]
fn fmt_mb_1048241() {
    assert_eq!(app_lib::format_size(1048241u64), "1023.7 KB");
}

#[test]
fn fmt_mb_1048242() {
    assert_eq!(app_lib::format_size(1048242u64), "1023.7 KB");
}

#[test]
fn fmt_mb_1048243() {
    assert_eq!(app_lib::format_size(1048243u64), "1023.7 KB");
}

#[test]
fn fmt_mb_1048244() {
    assert_eq!(app_lib::format_size(1048244u64), "1023.7 KB");
}

#[test]
fn fmt_mb_1048245() {
    assert_eq!(app_lib::format_size(1048245u64), "1023.7 KB");
}

#[test]
fn fmt_mb_1048246() {
    assert_eq!(app_lib::format_size(1048246u64), "1023.7 KB");
}

#[test]
fn fmt_mb_1048247() {
    assert_eq!(app_lib::format_size(1048247u64), "1023.7 KB");
}

#[test]
fn fmt_mb_1048248() {
    assert_eq!(app_lib::format_size(1048248u64), "1023.7 KB");
}

#[test]
fn fmt_mb_1048249() {
    assert_eq!(app_lib::format_size(1048249u64), "1023.7 KB");
}

#[test]
fn fmt_mb_1048250() {
    assert_eq!(app_lib::format_size(1048250u64), "1023.7 KB");
}

#[test]
fn fmt_mb_1048251() {
    assert_eq!(app_lib::format_size(1048251u64), "1023.7 KB");
}

#[test]
fn fmt_mb_1048252() {
    assert_eq!(app_lib::format_size(1048252u64), "1023.7 KB");
}

#[test]
fn fmt_mb_1048253() {
    assert_eq!(app_lib::format_size(1048253u64), "1023.7 KB");
}

#[test]
fn fmt_mb_1048254() {
    assert_eq!(app_lib::format_size(1048254u64), "1023.7 KB");
}

#[test]
fn fmt_mb_1048255() {
    assert_eq!(app_lib::format_size(1048255u64), "1023.7 KB");
}

#[test]
fn fmt_mb_1048256() {
    assert_eq!(app_lib::format_size(1048256u64), "1023.7 KB");
}

#[test]
fn fmt_mb_1048257() {
    assert_eq!(app_lib::format_size(1048257u64), "1023.7 KB");
}

#[test]
fn fmt_mb_1048258() {
    assert_eq!(app_lib::format_size(1048258u64), "1023.7 KB");
}

#[test]
fn fmt_mb_1048259() {
    assert_eq!(app_lib::format_size(1048259u64), "1023.7 KB");
}

#[test]
fn fmt_mb_1048260() {
    assert_eq!(app_lib::format_size(1048260u64), "1023.7 KB");
}

#[test]
fn fmt_mb_1048261() {
    assert_eq!(app_lib::format_size(1048261u64), "1023.7 KB");
}

#[test]
fn fmt_mb_1048262() {
    assert_eq!(app_lib::format_size(1048262u64), "1023.7 KB");
}

#[test]
fn fmt_mb_1048263() {
    assert_eq!(app_lib::format_size(1048263u64), "1023.7 KB");
}

#[test]
fn fmt_mb_1048264() {
    assert_eq!(app_lib::format_size(1048264u64), "1023.7 KB");
}

#[test]
fn fmt_mb_1048265() {
    assert_eq!(app_lib::format_size(1048265u64), "1023.7 KB");
}

#[test]
fn fmt_mb_1048266() {
    assert_eq!(app_lib::format_size(1048266u64), "1023.7 KB");
}

#[test]
fn fmt_mb_1048267() {
    assert_eq!(app_lib::format_size(1048267u64), "1023.7 KB");
}

#[test]
fn fmt_mb_1048268() {
    assert_eq!(app_lib::format_size(1048268u64), "1023.7 KB");
}

#[test]
fn fmt_mb_1048269() {
    assert_eq!(app_lib::format_size(1048269u64), "1023.7 KB");
}

#[test]
fn fmt_mb_1048270() {
    assert_eq!(app_lib::format_size(1048270u64), "1023.7 KB");
}

#[test]
fn fmt_mb_1048271() {
    assert_eq!(app_lib::format_size(1048271u64), "1023.7 KB");
}

#[test]
fn fmt_mb_1048272() {
    assert_eq!(app_lib::format_size(1048272u64), "1023.7 KB");
}

#[test]
fn fmt_mb_1048273() {
    assert_eq!(app_lib::format_size(1048273u64), "1023.7 KB");
}

#[test]
fn fmt_mb_1048274() {
    assert_eq!(app_lib::format_size(1048274u64), "1023.7 KB");
}

#[test]
fn fmt_mb_1048275() {
    assert_eq!(app_lib::format_size(1048275u64), "1023.7 KB");
}

#[test]
fn fmt_mb_1048276() {
    assert_eq!(app_lib::format_size(1048276u64), "1023.7 KB");
}

#[test]
fn fmt_mb_1048277() {
    assert_eq!(app_lib::format_size(1048277u64), "1023.7 KB");
}

#[test]
fn fmt_mb_1048278() {
    assert_eq!(app_lib::format_size(1048278u64), "1023.7 KB");
}

#[test]
fn fmt_mb_1048279() {
    assert_eq!(app_lib::format_size(1048279u64), "1023.7 KB");
}

#[test]
fn fmt_mb_1048280() {
    assert_eq!(app_lib::format_size(1048280u64), "1023.7 KB");
}

#[test]
fn fmt_mb_1048281() {
    assert_eq!(app_lib::format_size(1048281u64), "1023.7 KB");
}

#[test]
fn fmt_mb_1048282() {
    assert_eq!(app_lib::format_size(1048282u64), "1023.7 KB");
}

#[test]
fn fmt_mb_1048283() {
    assert_eq!(app_lib::format_size(1048283u64), "1023.7 KB");
}

#[test]
fn fmt_mb_1048284() {
    assert_eq!(app_lib::format_size(1048284u64), "1023.7 KB");
}

#[test]
fn fmt_mb_1048285() {
    assert_eq!(app_lib::format_size(1048285u64), "1023.7 KB");
}

#[test]
fn fmt_mb_1048286() {
    assert_eq!(app_lib::format_size(1048286u64), "1023.7 KB");
}

#[test]
fn fmt_mb_1048287() {
    assert_eq!(app_lib::format_size(1048287u64), "1023.7 KB");
}

#[test]
fn fmt_mb_1048288() {
    assert_eq!(app_lib::format_size(1048288u64), "1023.7 KB");
}

#[test]
fn fmt_mb_1048289() {
    assert_eq!(app_lib::format_size(1048289u64), "1023.7 KB");
}

#[test]
fn fmt_mb_1048290() {
    assert_eq!(app_lib::format_size(1048290u64), "1023.7 KB");
}

#[test]
fn fmt_mb_1048291() {
    assert_eq!(app_lib::format_size(1048291u64), "1023.7 KB");
}

#[test]
fn fmt_mb_1048292() {
    assert_eq!(app_lib::format_size(1048292u64), "1023.7 KB");
}

#[test]
fn fmt_mb_1048293() {
    assert_eq!(app_lib::format_size(1048293u64), "1023.7 KB");
}

#[test]
fn fmt_mb_1048294() {
    assert_eq!(app_lib::format_size(1048294u64), "1023.7 KB");
}

#[test]
fn fmt_mb_1048295() {
    assert_eq!(app_lib::format_size(1048295u64), "1023.7 KB");
}

#[test]
fn fmt_mb_1048296() {
    assert_eq!(app_lib::format_size(1048296u64), "1023.7 KB");
}

#[test]
fn fmt_mb_1048297() {
    assert_eq!(app_lib::format_size(1048297u64), "1023.7 KB");
}

#[test]
fn fmt_mb_1048298() {
    assert_eq!(app_lib::format_size(1048298u64), "1023.7 KB");
}

#[test]
fn fmt_mb_1048299() {
    assert_eq!(app_lib::format_size(1048299u64), "1023.7 KB");
}

#[test]
fn fmt_mb_1048300() {
    assert_eq!(app_lib::format_size(1048300u64), "1023.7 KB");
}

#[test]
fn fmt_mb_1048301() {
    assert_eq!(app_lib::format_size(1048301u64), "1023.7 KB");
}

#[test]
fn fmt_mb_1048302() {
    assert_eq!(app_lib::format_size(1048302u64), "1023.7 KB");
}

#[test]
fn fmt_mb_1048303() {
    assert_eq!(app_lib::format_size(1048303u64), "1023.7 KB");
}

#[test]
fn fmt_mb_1048304() {
    assert_eq!(app_lib::format_size(1048304u64), "1023.7 KB");
}

#[test]
fn fmt_mb_1048305() {
    assert_eq!(app_lib::format_size(1048305u64), "1023.7 KB");
}

#[test]
fn fmt_mb_1048306() {
    assert_eq!(app_lib::format_size(1048306u64), "1023.7 KB");
}

#[test]
fn fmt_mb_1048307() {
    assert_eq!(app_lib::format_size(1048307u64), "1023.7 KB");
}

#[test]
fn fmt_mb_1048308() {
    assert_eq!(app_lib::format_size(1048308u64), "1023.7 KB");
}

#[test]
fn fmt_mb_1048309() {
    assert_eq!(app_lib::format_size(1048309u64), "1023.7 KB");
}

#[test]
fn fmt_mb_1048310() {
    assert_eq!(app_lib::format_size(1048310u64), "1023.7 KB");
}

#[test]
fn fmt_mb_1048311() {
    assert_eq!(app_lib::format_size(1048311u64), "1023.7 KB");
}

#[test]
fn fmt_mb_1048312() {
    assert_eq!(app_lib::format_size(1048312u64), "1023.7 KB");
}

#[test]
fn fmt_mb_1048313() {
    assert_eq!(app_lib::format_size(1048313u64), "1023.7 KB");
}

#[test]
fn fmt_mb_1048314() {
    assert_eq!(app_lib::format_size(1048314u64), "1023.7 KB");
}

#[test]
fn fmt_mb_1048315() {
    assert_eq!(app_lib::format_size(1048315u64), "1023.7 KB");
}

#[test]
fn fmt_mb_1048316() {
    assert_eq!(app_lib::format_size(1048316u64), "1023.7 KB");
}

#[test]
fn fmt_mb_1048317() {
    assert_eq!(app_lib::format_size(1048317u64), "1023.7 KB");
}

#[test]
fn fmt_mb_1048318() {
    assert_eq!(app_lib::format_size(1048318u64), "1023.7 KB");
}

#[test]
fn fmt_mb_1048319() {
    assert_eq!(app_lib::format_size(1048319u64), "1023.7 KB");
}

#[test]
fn fmt_mb_1048320() {
    assert_eq!(app_lib::format_size(1048320u64), "1023.8 KB");
}

#[test]
fn fmt_mb_1048321() {
    assert_eq!(app_lib::format_size(1048321u64), "1023.8 KB");
}

#[test]
fn fmt_mb_1048322() {
    assert_eq!(app_lib::format_size(1048322u64), "1023.8 KB");
}

#[test]
fn fmt_mb_1048323() {
    assert_eq!(app_lib::format_size(1048323u64), "1023.8 KB");
}

#[test]
fn fmt_mb_1048324() {
    assert_eq!(app_lib::format_size(1048324u64), "1023.8 KB");
}

#[test]
fn fmt_mb_1048325() {
    assert_eq!(app_lib::format_size(1048325u64), "1023.8 KB");
}

#[test]
fn fmt_mb_1048326() {
    assert_eq!(app_lib::format_size(1048326u64), "1023.8 KB");
}

#[test]
fn fmt_mb_1048327() {
    assert_eq!(app_lib::format_size(1048327u64), "1023.8 KB");
}

#[test]
fn fmt_mb_1048328() {
    assert_eq!(app_lib::format_size(1048328u64), "1023.8 KB");
}

#[test]
fn fmt_mb_1048329() {
    assert_eq!(app_lib::format_size(1048329u64), "1023.8 KB");
}

#[test]
fn fmt_mb_1048330() {
    assert_eq!(app_lib::format_size(1048330u64), "1023.8 KB");
}

#[test]
fn fmt_mb_1048331() {
    assert_eq!(app_lib::format_size(1048331u64), "1023.8 KB");
}

#[test]
fn fmt_mb_1048332() {
    assert_eq!(app_lib::format_size(1048332u64), "1023.8 KB");
}

#[test]
fn fmt_mb_1048333() {
    assert_eq!(app_lib::format_size(1048333u64), "1023.8 KB");
}

#[test]
fn fmt_mb_1048334() {
    assert_eq!(app_lib::format_size(1048334u64), "1023.8 KB");
}

#[test]
fn fmt_mb_1048335() {
    assert_eq!(app_lib::format_size(1048335u64), "1023.8 KB");
}

#[test]
fn fmt_mb_1048336() {
    assert_eq!(app_lib::format_size(1048336u64), "1023.8 KB");
}

#[test]
fn fmt_mb_1048337() {
    assert_eq!(app_lib::format_size(1048337u64), "1023.8 KB");
}

#[test]
fn fmt_mb_1048338() {
    assert_eq!(app_lib::format_size(1048338u64), "1023.8 KB");
}

#[test]
fn fmt_mb_1048339() {
    assert_eq!(app_lib::format_size(1048339u64), "1023.8 KB");
}

#[test]
fn fmt_mb_1048340() {
    assert_eq!(app_lib::format_size(1048340u64), "1023.8 KB");
}

#[test]
fn fmt_mb_1048341() {
    assert_eq!(app_lib::format_size(1048341u64), "1023.8 KB");
}

#[test]
fn fmt_mb_1048342() {
    assert_eq!(app_lib::format_size(1048342u64), "1023.8 KB");
}

#[test]
fn fmt_mb_1048343() {
    assert_eq!(app_lib::format_size(1048343u64), "1023.8 KB");
}

#[test]
fn fmt_mb_1048344() {
    assert_eq!(app_lib::format_size(1048344u64), "1023.8 KB");
}

#[test]
fn fmt_mb_1048345() {
    assert_eq!(app_lib::format_size(1048345u64), "1023.8 KB");
}

#[test]
fn fmt_mb_1048346() {
    assert_eq!(app_lib::format_size(1048346u64), "1023.8 KB");
}

#[test]
fn fmt_mb_1048347() {
    assert_eq!(app_lib::format_size(1048347u64), "1023.8 KB");
}

#[test]
fn fmt_mb_1048348() {
    assert_eq!(app_lib::format_size(1048348u64), "1023.8 KB");
}

#[test]
fn fmt_mb_1048349() {
    assert_eq!(app_lib::format_size(1048349u64), "1023.8 KB");
}

#[test]
fn fmt_mb_1048350() {
    assert_eq!(app_lib::format_size(1048350u64), "1023.8 KB");
}

#[test]
fn fmt_mb_1048351() {
    assert_eq!(app_lib::format_size(1048351u64), "1023.8 KB");
}

#[test]
fn fmt_mb_1048352() {
    assert_eq!(app_lib::format_size(1048352u64), "1023.8 KB");
}

#[test]
fn fmt_mb_1048353() {
    assert_eq!(app_lib::format_size(1048353u64), "1023.8 KB");
}

#[test]
fn fmt_mb_1048354() {
    assert_eq!(app_lib::format_size(1048354u64), "1023.8 KB");
}

#[test]
fn fmt_mb_1048355() {
    assert_eq!(app_lib::format_size(1048355u64), "1023.8 KB");
}

#[test]
fn fmt_mb_1048356() {
    assert_eq!(app_lib::format_size(1048356u64), "1023.8 KB");
}

#[test]
fn fmt_mb_1048357() {
    assert_eq!(app_lib::format_size(1048357u64), "1023.8 KB");
}

#[test]
fn fmt_mb_1048358() {
    assert_eq!(app_lib::format_size(1048358u64), "1023.8 KB");
}

#[test]
fn fmt_mb_1048359() {
    assert_eq!(app_lib::format_size(1048359u64), "1023.8 KB");
}

#[test]
fn fmt_mb_1048360() {
    assert_eq!(app_lib::format_size(1048360u64), "1023.8 KB");
}

#[test]
fn fmt_mb_1048361() {
    assert_eq!(app_lib::format_size(1048361u64), "1023.8 KB");
}

#[test]
fn fmt_mb_1048362() {
    assert_eq!(app_lib::format_size(1048362u64), "1023.8 KB");
}

#[test]
fn fmt_mb_1048363() {
    assert_eq!(app_lib::format_size(1048363u64), "1023.8 KB");
}

#[test]
fn fmt_mb_1048364() {
    assert_eq!(app_lib::format_size(1048364u64), "1023.8 KB");
}

#[test]
fn fmt_mb_1048365() {
    assert_eq!(app_lib::format_size(1048365u64), "1023.8 KB");
}

#[test]
fn fmt_mb_1048366() {
    assert_eq!(app_lib::format_size(1048366u64), "1023.8 KB");
}

#[test]
fn fmt_mb_1048367() {
    assert_eq!(app_lib::format_size(1048367u64), "1023.8 KB");
}

#[test]
fn fmt_mb_1048368() {
    assert_eq!(app_lib::format_size(1048368u64), "1023.8 KB");
}

#[test]
fn fmt_mb_1048369() {
    assert_eq!(app_lib::format_size(1048369u64), "1023.8 KB");
}

#[test]
fn fmt_mb_1048370() {
    assert_eq!(app_lib::format_size(1048370u64), "1023.8 KB");
}

#[test]
fn fmt_mb_1048371() {
    assert_eq!(app_lib::format_size(1048371u64), "1023.8 KB");
}

#[test]
fn fmt_mb_1048372() {
    assert_eq!(app_lib::format_size(1048372u64), "1023.8 KB");
}

#[test]
fn fmt_mb_1048373() {
    assert_eq!(app_lib::format_size(1048373u64), "1023.8 KB");
}

#[test]
fn fmt_mb_1048374() {
    assert_eq!(app_lib::format_size(1048374u64), "1023.8 KB");
}

#[test]
fn fmt_mb_1048375() {
    assert_eq!(app_lib::format_size(1048375u64), "1023.8 KB");
}

#[test]
fn fmt_mb_1048376() {
    assert_eq!(app_lib::format_size(1048376u64), "1023.8 KB");
}

#[test]
fn fmt_mb_1048377() {
    assert_eq!(app_lib::format_size(1048377u64), "1023.8 KB");
}

#[test]
fn fmt_mb_1048378() {
    assert_eq!(app_lib::format_size(1048378u64), "1023.8 KB");
}

#[test]
fn fmt_mb_1048379() {
    assert_eq!(app_lib::format_size(1048379u64), "1023.8 KB");
}

#[test]
fn fmt_mb_1048380() {
    assert_eq!(app_lib::format_size(1048380u64), "1023.8 KB");
}

#[test]
fn fmt_mb_1048381() {
    assert_eq!(app_lib::format_size(1048381u64), "1023.8 KB");
}

#[test]
fn fmt_mb_1048382() {
    assert_eq!(app_lib::format_size(1048382u64), "1023.8 KB");
}

#[test]
fn fmt_mb_1048383() {
    assert_eq!(app_lib::format_size(1048383u64), "1023.8 KB");
}

#[test]
fn fmt_mb_1048384() {
    assert_eq!(app_lib::format_size(1048384u64), "1023.8 KB");
}

#[test]
fn fmt_mb_1048385() {
    assert_eq!(app_lib::format_size(1048385u64), "1023.8 KB");
}

#[test]
fn fmt_mb_1048386() {
    assert_eq!(app_lib::format_size(1048386u64), "1023.8 KB");
}

#[test]
fn fmt_mb_1048387() {
    assert_eq!(app_lib::format_size(1048387u64), "1023.8 KB");
}

#[test]
fn fmt_mb_1048388() {
    assert_eq!(app_lib::format_size(1048388u64), "1023.8 KB");
}

#[test]
fn fmt_mb_1048389() {
    assert_eq!(app_lib::format_size(1048389u64), "1023.8 KB");
}

#[test]
fn fmt_mb_1048390() {
    assert_eq!(app_lib::format_size(1048390u64), "1023.8 KB");
}

#[test]
fn fmt_mb_1048391() {
    assert_eq!(app_lib::format_size(1048391u64), "1023.8 KB");
}

#[test]
fn fmt_mb_1048392() {
    assert_eq!(app_lib::format_size(1048392u64), "1023.8 KB");
}

#[test]
fn fmt_mb_1048393() {
    assert_eq!(app_lib::format_size(1048393u64), "1023.8 KB");
}

#[test]
fn fmt_mb_1048394() {
    assert_eq!(app_lib::format_size(1048394u64), "1023.8 KB");
}

#[test]
fn fmt_mb_1048395() {
    assert_eq!(app_lib::format_size(1048395u64), "1023.8 KB");
}

#[test]
fn fmt_mb_1048396() {
    assert_eq!(app_lib::format_size(1048396u64), "1023.8 KB");
}

#[test]
fn fmt_mb_1048397() {
    assert_eq!(app_lib::format_size(1048397u64), "1023.8 KB");
}

#[test]
fn fmt_mb_1048398() {
    assert_eq!(app_lib::format_size(1048398u64), "1023.8 KB");
}

#[test]
fn fmt_mb_1048399() {
    assert_eq!(app_lib::format_size(1048399u64), "1023.8 KB");
}

#[test]
fn fmt_mb_1048400() {
    assert_eq!(app_lib::format_size(1048400u64), "1023.8 KB");
}

#[test]
fn fmt_mb_1048401() {
    assert_eq!(app_lib::format_size(1048401u64), "1023.8 KB");
}

#[test]
fn fmt_mb_1048402() {
    assert_eq!(app_lib::format_size(1048402u64), "1023.8 KB");
}

#[test]
fn fmt_mb_1048403() {
    assert_eq!(app_lib::format_size(1048403u64), "1023.8 KB");
}

#[test]
fn fmt_mb_1048404() {
    assert_eq!(app_lib::format_size(1048404u64), "1023.8 KB");
}

#[test]
fn fmt_mb_1048405() {
    assert_eq!(app_lib::format_size(1048405u64), "1023.8 KB");
}

#[test]
fn fmt_mb_1048406() {
    assert_eq!(app_lib::format_size(1048406u64), "1023.8 KB");
}

#[test]
fn fmt_mb_1048407() {
    assert_eq!(app_lib::format_size(1048407u64), "1023.8 KB");
}

#[test]
fn fmt_mb_1048408() {
    assert_eq!(app_lib::format_size(1048408u64), "1023.8 KB");
}

#[test]
fn fmt_mb_1048409() {
    assert_eq!(app_lib::format_size(1048409u64), "1023.8 KB");
}

#[test]
fn fmt_mb_1048410() {
    assert_eq!(app_lib::format_size(1048410u64), "1023.8 KB");
}

#[test]
fn fmt_mb_1048411() {
    assert_eq!(app_lib::format_size(1048411u64), "1023.8 KB");
}

#[test]
fn fmt_mb_1048412() {
    assert_eq!(app_lib::format_size(1048412u64), "1023.8 KB");
}

#[test]
fn fmt_mb_1048413() {
    assert_eq!(app_lib::format_size(1048413u64), "1023.8 KB");
}

#[test]
fn fmt_mb_1048414() {
    assert_eq!(app_lib::format_size(1048414u64), "1023.8 KB");
}

#[test]
fn fmt_mb_1048415() {
    assert_eq!(app_lib::format_size(1048415u64), "1023.8 KB");
}

#[test]
fn fmt_mb_1048416() {
    assert_eq!(app_lib::format_size(1048416u64), "1023.8 KB");
}

#[test]
fn fmt_mb_1048417() {
    assert_eq!(app_lib::format_size(1048417u64), "1023.8 KB");
}

#[test]
fn fmt_mb_1048418() {
    assert_eq!(app_lib::format_size(1048418u64), "1023.8 KB");
}

#[test]
fn fmt_mb_1048419() {
    assert_eq!(app_lib::format_size(1048419u64), "1023.8 KB");
}

#[test]
fn fmt_mb_1048420() {
    assert_eq!(app_lib::format_size(1048420u64), "1023.8 KB");
}

#[test]
fn fmt_mb_1048421() {
    assert_eq!(app_lib::format_size(1048421u64), "1023.8 KB");
}

#[test]
fn fmt_mb_1048422() {
    assert_eq!(app_lib::format_size(1048422u64), "1023.8 KB");
}

#[test]
fn fmt_mb_1048423() {
    assert_eq!(app_lib::format_size(1048423u64), "1023.9 KB");
}

#[test]
fn fmt_mb_1048424() {
    assert_eq!(app_lib::format_size(1048424u64), "1023.9 KB");
}

#[test]
fn fmt_mb_1048425() {
    assert_eq!(app_lib::format_size(1048425u64), "1023.9 KB");
}

#[test]
fn fmt_mb_1048426() {
    assert_eq!(app_lib::format_size(1048426u64), "1023.9 KB");
}

#[test]
fn fmt_mb_1048427() {
    assert_eq!(app_lib::format_size(1048427u64), "1023.9 KB");
}

#[test]
fn fmt_mb_1048428() {
    assert_eq!(app_lib::format_size(1048428u64), "1023.9 KB");
}

#[test]
fn fmt_mb_1048429() {
    assert_eq!(app_lib::format_size(1048429u64), "1023.9 KB");
}

#[test]
fn fmt_mb_1048430() {
    assert_eq!(app_lib::format_size(1048430u64), "1023.9 KB");
}

#[test]
fn fmt_mb_1048431() {
    assert_eq!(app_lib::format_size(1048431u64), "1023.9 KB");
}

#[test]
fn fmt_mb_1048432() {
    assert_eq!(app_lib::format_size(1048432u64), "1023.9 KB");
}

#[test]
fn fmt_mb_1048433() {
    assert_eq!(app_lib::format_size(1048433u64), "1023.9 KB");
}

#[test]
fn fmt_mb_1048434() {
    assert_eq!(app_lib::format_size(1048434u64), "1023.9 KB");
}

#[test]
fn fmt_mb_1048435() {
    assert_eq!(app_lib::format_size(1048435u64), "1023.9 KB");
}

#[test]
fn fmt_mb_1048436() {
    assert_eq!(app_lib::format_size(1048436u64), "1023.9 KB");
}

#[test]
fn fmt_mb_1048437() {
    assert_eq!(app_lib::format_size(1048437u64), "1023.9 KB");
}

#[test]
fn fmt_mb_1048438() {
    assert_eq!(app_lib::format_size(1048438u64), "1023.9 KB");
}

#[test]
fn fmt_mb_1048439() {
    assert_eq!(app_lib::format_size(1048439u64), "1023.9 KB");
}

#[test]
fn fmt_mb_1048440() {
    assert_eq!(app_lib::format_size(1048440u64), "1023.9 KB");
}

#[test]
fn fmt_mb_1048441() {
    assert_eq!(app_lib::format_size(1048441u64), "1023.9 KB");
}

#[test]
fn fmt_mb_1048442() {
    assert_eq!(app_lib::format_size(1048442u64), "1023.9 KB");
}

#[test]
fn fmt_mb_1048443() {
    assert_eq!(app_lib::format_size(1048443u64), "1023.9 KB");
}

#[test]
fn fmt_mb_1048444() {
    assert_eq!(app_lib::format_size(1048444u64), "1023.9 KB");
}

#[test]
fn fmt_mb_1048445() {
    assert_eq!(app_lib::format_size(1048445u64), "1023.9 KB");
}

#[test]
fn fmt_mb_1048446() {
    assert_eq!(app_lib::format_size(1048446u64), "1023.9 KB");
}

#[test]
fn fmt_mb_1048447() {
    assert_eq!(app_lib::format_size(1048447u64), "1023.9 KB");
}

#[test]
fn fmt_mb_1048448() {
    assert_eq!(app_lib::format_size(1048448u64), "1023.9 KB");
}

#[test]
fn fmt_mb_1048449() {
    assert_eq!(app_lib::format_size(1048449u64), "1023.9 KB");
}

#[test]
fn fmt_mb_1048450() {
    assert_eq!(app_lib::format_size(1048450u64), "1023.9 KB");
}

#[test]
fn fmt_mb_1048451() {
    assert_eq!(app_lib::format_size(1048451u64), "1023.9 KB");
}

#[test]
fn fmt_mb_1048452() {
    assert_eq!(app_lib::format_size(1048452u64), "1023.9 KB");
}

#[test]
fn fmt_mb_1048453() {
    assert_eq!(app_lib::format_size(1048453u64), "1023.9 KB");
}

#[test]
fn fmt_mb_1048454() {
    assert_eq!(app_lib::format_size(1048454u64), "1023.9 KB");
}

#[test]
fn fmt_mb_1048455() {
    assert_eq!(app_lib::format_size(1048455u64), "1023.9 KB");
}

#[test]
fn fmt_mb_1048456() {
    assert_eq!(app_lib::format_size(1048456u64), "1023.9 KB");
}

#[test]
fn fmt_mb_1048457() {
    assert_eq!(app_lib::format_size(1048457u64), "1023.9 KB");
}

#[test]
fn fmt_mb_1048458() {
    assert_eq!(app_lib::format_size(1048458u64), "1023.9 KB");
}

#[test]
fn fmt_mb_1048459() {
    assert_eq!(app_lib::format_size(1048459u64), "1023.9 KB");
}

#[test]
fn fmt_mb_1048460() {
    assert_eq!(app_lib::format_size(1048460u64), "1023.9 KB");
}

#[test]
fn fmt_mb_1048461() {
    assert_eq!(app_lib::format_size(1048461u64), "1023.9 KB");
}

#[test]
fn fmt_mb_1048462() {
    assert_eq!(app_lib::format_size(1048462u64), "1023.9 KB");
}

#[test]
fn fmt_mb_1048463() {
    assert_eq!(app_lib::format_size(1048463u64), "1023.9 KB");
}

#[test]
fn fmt_mb_1048464() {
    assert_eq!(app_lib::format_size(1048464u64), "1023.9 KB");
}

#[test]
fn fmt_mb_1048465() {
    assert_eq!(app_lib::format_size(1048465u64), "1023.9 KB");
}

#[test]
fn fmt_mb_1048466() {
    assert_eq!(app_lib::format_size(1048466u64), "1023.9 KB");
}

#[test]
fn fmt_mb_1048467() {
    assert_eq!(app_lib::format_size(1048467u64), "1023.9 KB");
}

#[test]
fn fmt_mb_1048468() {
    assert_eq!(app_lib::format_size(1048468u64), "1023.9 KB");
}

#[test]
fn fmt_mb_1048469() {
    assert_eq!(app_lib::format_size(1048469u64), "1023.9 KB");
}

#[test]
fn fmt_mb_1048470() {
    assert_eq!(app_lib::format_size(1048470u64), "1023.9 KB");
}

#[test]
fn fmt_mb_1048471() {
    assert_eq!(app_lib::format_size(1048471u64), "1023.9 KB");
}

#[test]
fn fmt_mb_1048472() {
    assert_eq!(app_lib::format_size(1048472u64), "1023.9 KB");
}

#[test]
fn fmt_mb_1048473() {
    assert_eq!(app_lib::format_size(1048473u64), "1023.9 KB");
}

#[test]
fn fmt_mb_1048474() {
    assert_eq!(app_lib::format_size(1048474u64), "1023.9 KB");
}

#[test]
fn fmt_mb_1048475() {
    assert_eq!(app_lib::format_size(1048475u64), "1023.9 KB");
}

#[test]
fn fmt_mb_1048476() {
    assert_eq!(app_lib::format_size(1048476u64), "1023.9 KB");
}

#[test]
fn fmt_mb_1048477() {
    assert_eq!(app_lib::format_size(1048477u64), "1023.9 KB");
}

#[test]
fn fmt_mb_1048478() {
    assert_eq!(app_lib::format_size(1048478u64), "1023.9 KB");
}

#[test]
fn fmt_mb_1048479() {
    assert_eq!(app_lib::format_size(1048479u64), "1023.9 KB");
}

#[test]
fn fmt_mb_1048480() {
    assert_eq!(app_lib::format_size(1048480u64), "1023.9 KB");
}

#[test]
fn fmt_mb_1048481() {
    assert_eq!(app_lib::format_size(1048481u64), "1023.9 KB");
}

#[test]
fn fmt_mb_1048482() {
    assert_eq!(app_lib::format_size(1048482u64), "1023.9 KB");
}

#[test]
fn fmt_mb_1048483() {
    assert_eq!(app_lib::format_size(1048483u64), "1023.9 KB");
}

#[test]
fn fmt_mb_1048484() {
    assert_eq!(app_lib::format_size(1048484u64), "1023.9 KB");
}

#[test]
fn fmt_mb_1048485() {
    assert_eq!(app_lib::format_size(1048485u64), "1023.9 KB");
}

#[test]
fn fmt_mb_1048486() {
    assert_eq!(app_lib::format_size(1048486u64), "1023.9 KB");
}

#[test]
fn fmt_mb_1048487() {
    assert_eq!(app_lib::format_size(1048487u64), "1023.9 KB");
}

#[test]
fn fmt_mb_1048488() {
    assert_eq!(app_lib::format_size(1048488u64), "1023.9 KB");
}

#[test]
fn fmt_mb_1048489() {
    assert_eq!(app_lib::format_size(1048489u64), "1023.9 KB");
}

#[test]
fn fmt_mb_1048490() {
    assert_eq!(app_lib::format_size(1048490u64), "1023.9 KB");
}

#[test]
fn fmt_mb_1048491() {
    assert_eq!(app_lib::format_size(1048491u64), "1023.9 KB");
}

#[test]
fn fmt_mb_1048492() {
    assert_eq!(app_lib::format_size(1048492u64), "1023.9 KB");
}

#[test]
fn fmt_mb_1048493() {
    assert_eq!(app_lib::format_size(1048493u64), "1023.9 KB");
}

#[test]
fn fmt_mb_1048494() {
    assert_eq!(app_lib::format_size(1048494u64), "1023.9 KB");
}

#[test]
fn fmt_mb_1048495() {
    assert_eq!(app_lib::format_size(1048495u64), "1023.9 KB");
}

#[test]
fn fmt_mb_1048496() {
    assert_eq!(app_lib::format_size(1048496u64), "1023.9 KB");
}

#[test]
fn fmt_mb_1048497() {
    assert_eq!(app_lib::format_size(1048497u64), "1023.9 KB");
}

#[test]
fn fmt_mb_1048498() {
    assert_eq!(app_lib::format_size(1048498u64), "1023.9 KB");
}

#[test]
fn fmt_mb_1048499() {
    assert_eq!(app_lib::format_size(1048499u64), "1023.9 KB");
}

#[test]
fn fmt_mb_1048500() {
    assert_eq!(app_lib::format_size(1048500u64), "1023.9 KB");
}

#[test]
fn fmt_mb_1048501() {
    assert_eq!(app_lib::format_size(1048501u64), "1023.9 KB");
}

#[test]
fn fmt_mb_1048502() {
    assert_eq!(app_lib::format_size(1048502u64), "1023.9 KB");
}

#[test]
fn fmt_mb_1048503() {
    assert_eq!(app_lib::format_size(1048503u64), "1023.9 KB");
}

#[test]
fn fmt_mb_1048504() {
    assert_eq!(app_lib::format_size(1048504u64), "1023.9 KB");
}

#[test]
fn fmt_mb_1048505() {
    assert_eq!(app_lib::format_size(1048505u64), "1023.9 KB");
}

#[test]
fn fmt_mb_1048506() {
    assert_eq!(app_lib::format_size(1048506u64), "1023.9 KB");
}

#[test]
fn fmt_mb_1048507() {
    assert_eq!(app_lib::format_size(1048507u64), "1023.9 KB");
}

#[test]
fn fmt_mb_1048508() {
    assert_eq!(app_lib::format_size(1048508u64), "1023.9 KB");
}

#[test]
fn fmt_mb_1048509() {
    assert_eq!(app_lib::format_size(1048509u64), "1023.9 KB");
}

#[test]
fn fmt_mb_1048510() {
    assert_eq!(app_lib::format_size(1048510u64), "1023.9 KB");
}

#[test]
fn fmt_mb_1048511() {
    assert_eq!(app_lib::format_size(1048511u64), "1023.9 KB");
}

#[test]
fn fmt_mb_1048512() {
    assert_eq!(app_lib::format_size(1048512u64), "1023.9 KB");
}

#[test]
fn fmt_mb_1048513() {
    assert_eq!(app_lib::format_size(1048513u64), "1023.9 KB");
}

#[test]
fn fmt_mb_1048514() {
    assert_eq!(app_lib::format_size(1048514u64), "1023.9 KB");
}

#[test]
fn fmt_mb_1048515() {
    assert_eq!(app_lib::format_size(1048515u64), "1023.9 KB");
}

#[test]
fn fmt_mb_1048516() {
    assert_eq!(app_lib::format_size(1048516u64), "1023.9 KB");
}

#[test]
fn fmt_mb_1048517() {
    assert_eq!(app_lib::format_size(1048517u64), "1023.9 KB");
}

#[test]
fn fmt_mb_1048518() {
    assert_eq!(app_lib::format_size(1048518u64), "1023.9 KB");
}

#[test]
fn fmt_mb_1048519() {
    assert_eq!(app_lib::format_size(1048519u64), "1023.9 KB");
}

#[test]
fn fmt_mb_1048520() {
    assert_eq!(app_lib::format_size(1048520u64), "1023.9 KB");
}

#[test]
fn fmt_mb_1048521() {
    assert_eq!(app_lib::format_size(1048521u64), "1023.9 KB");
}

#[test]
fn fmt_mb_1048522() {
    assert_eq!(app_lib::format_size(1048522u64), "1023.9 KB");
}

#[test]
fn fmt_mb_1048523() {
    assert_eq!(app_lib::format_size(1048523u64), "1023.9 KB");
}

#[test]
fn fmt_mb_1048524() {
    assert_eq!(app_lib::format_size(1048524u64), "1023.9 KB");
}

#[test]
fn fmt_mb_1048525() {
    assert_eq!(app_lib::format_size(1048525u64), "1024.0 KB");
}

#[test]
fn fmt_mb_1048526() {
    assert_eq!(app_lib::format_size(1048526u64), "1024.0 KB");
}

#[test]
fn fmt_mb_1048527() {
    assert_eq!(app_lib::format_size(1048527u64), "1024.0 KB");
}

#[test]
fn fmt_mb_1048528() {
    assert_eq!(app_lib::format_size(1048528u64), "1024.0 KB");
}

#[test]
fn fmt_mb_1048529() {
    assert_eq!(app_lib::format_size(1048529u64), "1024.0 KB");
}

#[test]
fn fmt_mb_1048530() {
    assert_eq!(app_lib::format_size(1048530u64), "1024.0 KB");
}

#[test]
fn fmt_mb_1048531() {
    assert_eq!(app_lib::format_size(1048531u64), "1024.0 KB");
}

#[test]
fn fmt_mb_1048532() {
    assert_eq!(app_lib::format_size(1048532u64), "1024.0 KB");
}

#[test]
fn fmt_mb_1048533() {
    assert_eq!(app_lib::format_size(1048533u64), "1024.0 KB");
}

#[test]
fn fmt_mb_1048534() {
    assert_eq!(app_lib::format_size(1048534u64), "1024.0 KB");
}

#[test]
fn fmt_mb_1048535() {
    assert_eq!(app_lib::format_size(1048535u64), "1024.0 KB");
}

#[test]
fn fmt_mb_1048536() {
    assert_eq!(app_lib::format_size(1048536u64), "1024.0 KB");
}

#[test]
fn fmt_mb_1048537() {
    assert_eq!(app_lib::format_size(1048537u64), "1024.0 KB");
}

#[test]
fn fmt_mb_1048538() {
    assert_eq!(app_lib::format_size(1048538u64), "1024.0 KB");
}

#[test]
fn fmt_mb_1048539() {
    assert_eq!(app_lib::format_size(1048539u64), "1024.0 KB");
}

#[test]
fn fmt_mb_1048540() {
    assert_eq!(app_lib::format_size(1048540u64), "1024.0 KB");
}

#[test]
fn fmt_mb_1048541() {
    assert_eq!(app_lib::format_size(1048541u64), "1024.0 KB");
}

#[test]
fn fmt_mb_1048542() {
    assert_eq!(app_lib::format_size(1048542u64), "1024.0 KB");
}

#[test]
fn fmt_mb_1048543() {
    assert_eq!(app_lib::format_size(1048543u64), "1024.0 KB");
}

#[test]
fn fmt_mb_1048544() {
    assert_eq!(app_lib::format_size(1048544u64), "1024.0 KB");
}

#[test]
fn fmt_mb_1048545() {
    assert_eq!(app_lib::format_size(1048545u64), "1024.0 KB");
}

#[test]
fn fmt_mb_1048546() {
    assert_eq!(app_lib::format_size(1048546u64), "1024.0 KB");
}

#[test]
fn fmt_mb_1048547() {
    assert_eq!(app_lib::format_size(1048547u64), "1024.0 KB");
}

#[test]
fn fmt_mb_1048548() {
    assert_eq!(app_lib::format_size(1048548u64), "1024.0 KB");
}

#[test]
fn fmt_mb_1048549() {
    assert_eq!(app_lib::format_size(1048549u64), "1024.0 KB");
}

#[test]
fn fmt_mb_1048550() {
    assert_eq!(app_lib::format_size(1048550u64), "1024.0 KB");
}

#[test]
fn fmt_mb_1048551() {
    assert_eq!(app_lib::format_size(1048551u64), "1024.0 KB");
}

#[test]
fn fmt_mb_1048552() {
    assert_eq!(app_lib::format_size(1048552u64), "1024.0 KB");
}

#[test]
fn fmt_mb_1048553() {
    assert_eq!(app_lib::format_size(1048553u64), "1024.0 KB");
}

#[test]
fn fmt_mb_1048554() {
    assert_eq!(app_lib::format_size(1048554u64), "1024.0 KB");
}

#[test]
fn fmt_mb_1048555() {
    assert_eq!(app_lib::format_size(1048555u64), "1024.0 KB");
}

#[test]
fn fmt_mb_1048556() {
    assert_eq!(app_lib::format_size(1048556u64), "1024.0 KB");
}

#[test]
fn fmt_mb_1048557() {
    assert_eq!(app_lib::format_size(1048557u64), "1024.0 KB");
}

#[test]
fn fmt_mb_1048558() {
    assert_eq!(app_lib::format_size(1048558u64), "1024.0 KB");
}

#[test]
fn fmt_mb_1048559() {
    assert_eq!(app_lib::format_size(1048559u64), "1024.0 KB");
}

#[test]
fn fmt_mb_1048560() {
    assert_eq!(app_lib::format_size(1048560u64), "1024.0 KB");
}

#[test]
fn fmt_mb_1048561() {
    assert_eq!(app_lib::format_size(1048561u64), "1024.0 KB");
}

#[test]
fn fmt_mb_1048562() {
    assert_eq!(app_lib::format_size(1048562u64), "1024.0 KB");
}

#[test]
fn fmt_mb_1048563() {
    assert_eq!(app_lib::format_size(1048563u64), "1024.0 KB");
}

#[test]
fn fmt_mb_1048564() {
    assert_eq!(app_lib::format_size(1048564u64), "1024.0 KB");
}

#[test]
fn fmt_mb_1048565() {
    assert_eq!(app_lib::format_size(1048565u64), "1024.0 KB");
}

#[test]
fn fmt_mb_1048566() {
    assert_eq!(app_lib::format_size(1048566u64), "1024.0 KB");
}

#[test]
fn fmt_mb_1048567() {
    assert_eq!(app_lib::format_size(1048567u64), "1024.0 KB");
}

#[test]
fn fmt_mb_1048568() {
    assert_eq!(app_lib::format_size(1048568u64), "1024.0 KB");
}

#[test]
fn fmt_mb_1048569() {
    assert_eq!(app_lib::format_size(1048569u64), "1024.0 KB");
}

#[test]
fn fmt_mb_1048570() {
    assert_eq!(app_lib::format_size(1048570u64), "1024.0 KB");
}

#[test]
fn fmt_mb_1048571() {
    assert_eq!(app_lib::format_size(1048571u64), "1024.0 KB");
}

#[test]
fn fmt_mb_1048572() {
    assert_eq!(app_lib::format_size(1048572u64), "1024.0 KB");
}

#[test]
fn fmt_mb_1048573() {
    assert_eq!(app_lib::format_size(1048573u64), "1024.0 KB");
}

#[test]
fn fmt_mb_1048574() {
    assert_eq!(app_lib::format_size(1048574u64), "1024.0 KB");
}

#[test]
fn fmt_mb_1048575() {
    assert_eq!(app_lib::format_size(1048575u64), "1024.0 KB");
}

#[test]
fn fmt_mb_1048576() {
    assert_eq!(app_lib::format_size(1048576u64), "1.0 MB");
}

#[test]
fn fmt_pib() {
    assert_eq!(app_lib::format_size(1125899906842624u64), "1024.0 TB");
}

