//! Explicit `compare_versions` pairs (expected ordering precomputed against `parse_version` semantics).

use std::cmp::Ordering;

macro_rules! kvr_cmp {
    ($($name:ident: ($a:expr, $b:expr) => $ord:expr)*) => {
        $(
            #[test]
            fn $name() {
                assert_eq!(
                    app_lib::kvr::compare_versions($a, $b),
                    $ord,
                    "compare({}, {})",
                    $a,
                    $b
                );
            }
        )*
    };
}

kvr_cmp! {
    cmp_001: ("163.7.0", "189.17.7") => Ordering::Less
    cmp_002: ("57.8.23", "26.43.23") => Ordering::Greater
    cmp_003: ("139.5.18", "108.2.0") => Ordering::Greater
    cmp_004: ("23.13.7", "129.38.0") => Ordering::Less
    cmp_005: ("143.12.22", "166.44.17") => Ordering::Less
    cmp_006: ("107.14.14", "150.17.25") => Ordering::Less
    cmp_007: ("1.48.25", "40.44.13") => Ordering::Less
    cmp_008: ("87.17.4", "55.48.10") => Ordering::Greater
    cmp_009: ("26.5.12", "24.22.27") => Ordering::Greater
    cmp_010: ("88.38.8", "11.46.14") => Ordering::Greater
    cmp_011: ("137.7.29", "96.5.17") => Ordering::Greater
    cmp_012: ("75.40.19", "92.36.6") => Ordering::Less
    cmp_013: ("180.4.1", "169.14.24") => Ordering::Greater
    cmp_014: ("74.5.27", "59.6.12") => Ordering::Greater
    cmp_015: ("71.29.20", "93.10.11") => Ordering::Less
    cmp_016: ("90.13.21", "68.44.29") => Ordering::Greater
    cmp_017: ("174.41.2", "155.40.5") => Ordering::Greater
    cmp_018: ("136.46.7", "41.29.12") => Ordering::Greater
    cmp_019: ("69.40.22", "142.14.21") => Ordering::Less
    cmp_020: ("83.49.24", "14.14.26") => Ordering::Greater
    cmp_021: ("8.20.12", "68.4.6") => Ordering::Less
    cmp_022: ("145.45.10", "54.41.15") => Ordering::Greater
    cmp_023: ("101.41.14", "36.16.4") => Ordering::Greater
    cmp_024: ("63.47.17", "137.16.23") => Ordering::Less
    cmp_025: ("149.27.28", "149.25.11") => Ordering::Greater
    cmp_026: ("56.8.16", "126.5.24") => Ordering::Less
    cmp_027: ("12.7.4", "160.10.25") => Ordering::Less
    cmp_028: ("174.27.19", "16.24.12") => Ordering::Greater
    cmp_029: ("152.29.16", "64.35.27") => Ordering::Greater
    cmp_030: ("2.43.23", "29.43.28") => Ordering::Less
    cmp_031: ("137.48.8", "196.41.10") => Ordering::Less
    cmp_032: ("28.18.13", "40.29.0") => Ordering::Less
    cmp_033: ("184.46.8", "128.48.5") => Ordering::Greater
    cmp_034: ("129.6.27", "160.19.26") => Ordering::Less
    cmp_035: ("163.32.19", "50.9.11") => Ordering::Greater
    cmp_036: ("195.10.17", "199.33.29") => Ordering::Less
    cmp_037: ("0.38.10", "125.1.3") => Ordering::Less
    cmp_038: ("92.19.7", "14.15.28") => Ordering::Greater
    cmp_039: ("145.5.2", "187.31.26") => Ordering::Less
    cmp_040: ("17.48.17", "196.8.4") => Ordering::Less
    cmp_041: ("168.30.30", "140.10.8") => Ordering::Greater
    cmp_042: ("135.38.13", "54.34.24") => Ordering::Greater
    cmp_043: ("186.44.6", "182.19.12") => Ordering::Greater
    cmp_044: ("171.41.11", "112.33.14") => Ordering::Greater
    cmp_045: ("30.15.7", "16.21.0") => Ordering::Greater
    cmp_046: ("150.35.7", "150.14.0") => Ordering::Greater
    cmp_047: ("18.45.20", "15.14.2") => Ordering::Greater
    cmp_048: ("8.21.2", "131.15.8") => Ordering::Less
    cmp_049: ("171.31.6", "138.8.23") => Ordering::Greater
    cmp_050: ("146.36.15", "62.50.15") => Ordering::Greater
}
