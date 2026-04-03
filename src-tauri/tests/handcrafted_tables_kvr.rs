//! Hand-authored KVR tables: `parse_version` vectors and `compare_versions` ordering.
//! Pairwise checks use a strictly increasing version chain (verified against Rust semantics).

use std::cmp::Ordering;

#[test]
fn handcrafted_parse_version_empty_and_unknown() {
    for (s, exp) in [
        ("", vec![0, 0, 0]),
        ("Unknown", vec![0, 0, 0]),
    ] {
        assert_eq!(app_lib::kvr::parse_version(s), exp, "{s:?}");
    }
}

#[test]
fn handcrafted_parse_version_numeric_segments() {
    let cases: &[(&str, Vec<i32>)] = &[
        ("0", vec![0]),
        ("1", vec![1]),
        ("01", vec![1]),
        ("10", vec![10]),
        ("2147483647", vec![2147483647]),
        ("1.0", vec![1, 0]),
        ("1.0.0", vec![1, 0, 0]),
        ("1.0.0.0", vec![1, 0, 0, 0]),
        ("12.34.56", vec![12, 34, 56]),
        ("10.20.30", vec![10, 20, 30]),
        ("99.99.99", vec![99, 99, 99]),
        ("100.0.0", vec![100, 0, 0]),
    ];
    for (s, exp) in cases {
        assert_eq!(&app_lib::kvr::parse_version(s), exp, "{s:?}");
    }
}

#[test]
fn handcrafted_parse_version_dots_and_non_numeric() {
    let cases: &[(&str, Vec<i32>)] = &[
        ("1.0.", vec![1, 0, 0]),
        (".5.0", vec![0, 5, 0]),
        ("1.x.3", vec![1, 0, 3]),
        ("1.0.beta", vec![1, 0, 0]),
        ("v1", vec![0]),
        ("1..2", vec![1, 0, 2]),
        ("..3", vec![0, 0, 3]),
        ("2147483647.0", vec![2147483647, 0]),
    ];
    for (s, exp) in cases {
        assert_eq!(&app_lib::kvr::parse_version(s), exp, "{s:?}");
    }
}

#[test]
fn handcrafted_compare_versions_reflexive_chain() {
    const CHAIN: &[&str] = &[
        "0", "0.0.1", "0.1", "0.9", "1.0.0.1", "1.0.0.2", "1.0.1", "1.0.9", "1.0.10", "1.1",
        "1.2.3", "2.0.0.1", "2.1", "3.0.1", "10.0", "10.0.1", "10.20.30", "11.0.0", "99.99.99",
        "100.0.0", "100.0.1", "101.0", "200.5.6", "200.5.7", "201.0.0", "300.0", "400.1.2",
        "500.0.0.1", "600.0", "700.10.20", "800.0.0.0.1", "900.0.0.1", "999.999.999", "1000.0",
        "1001.2.3", "1234.0.0", "5000.0", "9999.0", "10000.0", "10001.1", "20000.0.0", "30000.1",
    ];
    for a in CHAIN {
        assert_eq!(
            app_lib::kvr::compare_versions(a, a),
            Ordering::Equal,
            "{a}"
        );
    }
}

#[test]
fn handcrafted_compare_versions_strict_total_order_on_chain() {
    const CHAIN: &[&str] = &[
        "0", "0.0.1", "0.1", "0.9", "1.0.0.1", "1.0.0.2", "1.0.1", "1.0.9", "1.0.10", "1.1",
        "1.2.3", "2.0.0.1", "2.1", "3.0.1", "10.0", "10.0.1", "10.20.30", "11.0.0", "99.99.99",
        "100.0.0", "100.0.1", "101.0", "200.5.6", "200.5.7", "201.0.0", "300.0", "400.1.2",
        "500.0.0.1", "600.0", "700.10.20", "800.0.0.0.1", "900.0.0.1", "999.999.999", "1000.0",
        "1001.2.3", "1234.0.0", "5000.0", "9999.0", "10000.0", "10001.1", "20000.0.0", "30000.1",
    ];
    for i in 0..CHAIN.len() {
        for j in (i + 1)..CHAIN.len() {
            assert_eq!(
                app_lib::kvr::compare_versions(CHAIN[i], CHAIN[j]),
                Ordering::Less,
                "{} < {}",
                CHAIN[i],
                CHAIN[j]
            );
            assert_eq!(
                app_lib::kvr::compare_versions(CHAIN[j], CHAIN[i]),
                Ordering::Greater,
                "{} > {}",
                CHAIN[j],
                CHAIN[i]
            );
        }
    }
}

#[test]
fn handcrafted_compare_versions_explicit_pairs() {
    let pairs: &[(&str, &str, Ordering)] = &[
        ("1.0.9", "1.0.10", Ordering::Less),
        ("1.0.10", "1.0.9", Ordering::Greater),
        ("2.0.0", "1.99.99", Ordering::Greater),
        ("2.0", "2.0.0.0", Ordering::Equal),
        ("3", "3.0.1", Ordering::Less),
        ("1.0.0.1", "1.0.0.2", Ordering::Less),
        ("0.9", "1.0", Ordering::Less),
        ("10.0", "9.0", Ordering::Greater),
        ("1.0.1", "1.0.0", Ordering::Greater),
        ("1.0", "1.0.1", Ordering::Less),
    ];
    for (a, b, want) in pairs {
        assert_eq!(
            app_lib::kvr::compare_versions(a, b),
            *want,
            "compare({a}, {b})"
        );
    }
}
