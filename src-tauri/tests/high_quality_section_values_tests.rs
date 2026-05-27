use app_lib::als_project::SectionValues;

#[test]
fn test_section_values_snapping_logic() {
    let mut sv = SectionValues::default();

    // First block: 1..8
    sv.set(1, 0.1);
    assert_eq!(sv.value_at_bar(1, 0.0), 0.1);
    assert_eq!(sv.value_at_bar(8, 0.0), 0.1);
    assert_eq!(sv.value_at_bar(9, 0.0), 0.0); // Next block

    // Mid-block set (should snap down to 1)
    sv.set(5, 0.2);
    assert_eq!(
        sv.value_at_bar(1, 0.0),
        0.2,
        "Bar 5 should have snapped to 1, overwriting previous value"
    );

    // Second block: 9..16
    sv.set(9, 0.9);
    assert_eq!(sv.value_at_bar(9, 0.0), 0.9);
    assert_eq!(sv.value_at_bar(16, 0.0), 0.9);

    // Boundary of second block (should snap to 9)
    sv.set(16, 0.8);
    assert_eq!(sv.value_at_bar(9, 0.0), 0.8);

    // Third block starts at 17
    assert_eq!(sv.value_at_bar(17, 0.5), 0.5);
    sv.set(17, 0.7);
    assert_eq!(sv.value_at_bar(17, 0.5), 0.7);
}

#[test]
fn test_section_values_saturating_sub_zero() {
    let mut sv = SectionValues::default();
    // Bar 0 is technically invalid in the UI but let's test robust code
    sv.set(0, 1.0);
    // (0-1).saturating_sub(1) -> 0. 0/8*8 + 1 = 1.
    assert_eq!(sv.value_at_bar(1, 0.0), 1.0);
}

#[test]
fn test_section_values_clamping() {
    let mut sv = SectionValues::default();
    sv.set(1, 5.0);
    assert_eq!(sv.value_at_bar(1, 0.0), 1.0);
    sv.set(9, -1.0);
    assert_eq!(sv.value_at_bar(9, 0.5), 0.0);
}

#[test]
fn test_section_values_many_blocks() {
    let mut sv = SectionValues::default();
    for i in (1..100).step_by(8) {
        sv.set(i, i as f32 / 100.0);
    }

    for i in (1..100).step_by(8) {
        let expected = i as f32 / 100.0;
        assert_eq!(sv.value_at_bar(i, 0.0), expected);
        assert_eq!(sv.value_at_bar(i + 7, 0.0), expected);
    }
}
