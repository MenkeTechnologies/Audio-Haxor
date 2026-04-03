//! Pure helpers from `daw_scanner` (extension matching, display names) — no filesystem walks.

use std::path::Path;

use app_lib::daw_scanner::{daw_name_for_format, ext_matches, is_package_ext};

#[test]
fn ext_matches_recognizes_common_daw_suffixes() {
    assert_eq!(
        ext_matches(Path::new("/p/MyProject.als")).as_deref(),
        Some("ALS")
    );
    assert_eq!(
        ext_matches(Path::new("/p/Session.bwproject")).as_deref(),
        Some("BWPROJECT")
    );
    assert_eq!(
        ext_matches(Path::new("/p/backup.RPP-BAK")).as_deref(),
        Some("RPP-BAK")
    );
    assert_eq!(ext_matches(Path::new("/p/foo.txt")), None);
}

#[test]
fn is_package_ext_logicx_and_band_only() {
    assert!(is_package_ext(Path::new("/Music/Beat.logicx")));
    assert!(is_package_ext(Path::new("/Music/Song.band")));
    assert!(!is_package_ext(Path::new("/p/x.als")));
}

#[test]
fn daw_name_for_format_maps_and_unknown_fallback() {
    assert_eq!(daw_name_for_format("ALS"), "Ableton Live");
    assert_eq!(daw_name_for_format("RPP"), "REAPER");
    assert_eq!(daw_name_for_format("RPP-BAK"), "REAPER");
    assert_eq!(daw_name_for_format("DAWPROJECT"), "DAWproject");
    assert_eq!(daw_name_for_format("NOT_A_FORMAT"), "Unknown");
}
