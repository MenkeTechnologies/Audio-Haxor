use app_lib::als_project::SectionLengths;
use app_lib::track_generator::remap_bar_range;

#[test]
fn test_remap_bar_range_identity() {
    // identity mapping: user layout matches techno_default (32 bars per section)
    let user = SectionLengths::techno_default().starts();

    // Intro canonical is 1..33
    assert_eq!(remap_bar_range(1.0, 33.0, &user), Some((1.0, 33.0)));
    assert_eq!(remap_bar_range(16.5, 20.0, &user), Some((16.5, 20.0)));

    // Build canonical is 33..65
    assert_eq!(remap_bar_range(33.0, 65.0, &user), Some((33.0, 65.0)));
}

#[test]
fn test_remap_bar_range_shift() {
    // User breakdown is longer (48 bars) than techno_default (32 bars)
    let mut sl = SectionLengths::techno_default();
    sl.breakdown = 48;
    let user = sl.starts();

    // Breakdown canonical starts at 65 (32+32+1)
    // Shift is unchanged for sections BEFORE breakdown
    assert_eq!(remap_bar_range(1.0, 9.0, &user), Some((1.0, 9.0)));

    // Breakdown itself starts at 65. offset 0 should map to 65.
    assert_eq!(remap_bar_range(65.0, 69.0, &user), Some((65.0, 69.0)));

    // Sections AFTER breakdown are pushed later
    // Drop 1 canonical starts at 97 (32*3 + 1)
    // User Drop 1 starts at 113 (32+32+48 + 1)
    assert_eq!(remap_bar_range(97.0, 101.0, &user), Some((113.0, 117.0)));
}

#[test]
fn test_remap_bar_range_truncation() {
    // User breakdown is shorter (8 bars) than techno_default (32 bars)
    let mut sl = SectionLengths::techno_default();
    sl.breakdown = 8;
    let user = sl.starts();

    // Range (65..97) in canonical (32 bars)
    // User range is only 8 bars (65..73)

    // Starts at 65, ends at 73 in user
    assert_eq!(remap_bar_range(65.0, 97.0, &user), Some((65.0, 73.0)));

    // Starts at offset 4 (69.0)
    assert_eq!(remap_bar_range(69.0, 97.0, &user), Some((69.0, 73.0)));

    // Starts at offset 10 (75.0) -> past end of user section (8 bars)
    assert_eq!(remap_bar_range(75.0, 80.0, &user), None);
}

#[test]
fn test_remap_bar_range_out_of_bounds() {
    let user = SectionLengths::techno_default().starts();

    // Canonical only goes up to 224 bars (7 * 32)
    // Outro starts at 193, ends at 225.

    assert!(remap_bar_range(225.0, 226.0, &user).is_none());
    assert!(remap_bar_range(0.0, 1.0, &user).is_none());
}
