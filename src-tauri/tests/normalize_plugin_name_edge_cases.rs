//! Edge-case pins for `normalize_plugin_name` in `xref.rs`.
//!
//! Complements existing pin coverage in `backend_handwritten_contracts.rs`
//! (`*_strips_bracket_vst3_suffix`, `*_bare_x64_without_parens`,
//! `*_triple_stacked_arch_suffixes`) and `backend_handwritten_parsers_kvr.rs`
//! (`*_strips_trailing_vst3_suffix`, `*_preserves_hyphenated_product_name`).
//!
//! Surfaces pinned here:
//!   - all individual ARCH_SUFFIX_RE tokens (every alternation branch)
//!   - bracket-form variants (`[...]`)
//!   - mixed bracket + paren stacking
//!   - whitespace-collapse interaction with suffix stripping
//!   - empty-after-strip fallback to original lowercased name
//!   - dash/underscore preservation
//!   - mid-string suffix tokens (must NOT be stripped — `$` anchor)
//!   - empty / whitespace-only input

use app_lib::xref::normalize_plugin_name;

// ── Every individual ARCH_SUFFIX_RE token (paren form) ────────────

#[test]
fn strips_paren_x64() {
    assert_eq!(normalize_plugin_name("Serum (x64)"), "serum");
}

#[test]
fn strips_paren_x86_64() {
    assert_eq!(normalize_plugin_name("Diva (x86_64)"), "diva");
}

#[test]
fn strips_paren_x86() {
    assert_eq!(normalize_plugin_name("Massive (x86)"), "massive");
}

#[test]
fn strips_paren_arm64() {
    assert_eq!(normalize_plugin_name("Vital (arm64)"), "vital");
}

#[test]
fn strips_paren_aarch64() {
    assert_eq!(normalize_plugin_name("Helm (aarch64)"), "helm");
}

#[test]
fn strips_paren_64_dash_bit() {
    assert_eq!(normalize_plugin_name("Battery 4 (64-bit)"), "battery 4");
}

#[test]
fn strips_paren_32_space_bit() {
    assert_eq!(normalize_plugin_name("Kontakt 5 (32 bit)"), "kontakt 5");
}

#[test]
fn strips_paren_intel() {
    assert_eq!(normalize_plugin_name("Pro-Q 3 (intel)"), "pro-q 3");
}

#[test]
fn strips_paren_apple_silicon() {
    assert_eq!(normalize_plugin_name("Helm (apple silicon)"), "helm");
}

#[test]
fn strips_paren_universal() {
    assert_eq!(normalize_plugin_name("Phoenix (Universal)"), "phoenix");
}

#[test]
fn strips_paren_stereo() {
    assert_eq!(normalize_plugin_name("Reverb (Stereo)"), "reverb");
}

#[test]
fn strips_paren_mono() {
    assert_eq!(normalize_plugin_name("Bass (Mono)"), "bass");
}

#[test]
fn strips_paren_aax() {
    assert_eq!(normalize_plugin_name("Limiter (AAX)"), "limiter");
}

#[test]
fn strips_paren_au() {
    assert_eq!(
        normalize_plugin_name("MeldaProduction (AU)"),
        "meldaproduction"
    );
}

#[test]
fn strips_paren_vst() {
    assert_eq!(normalize_plugin_name("Pro-Q 3 (VST)"), "pro-q 3");
}

// ── Bracket-form alternation ──────────────────────────────────────

#[test]
fn strips_bracket_au() {
    assert_eq!(normalize_plugin_name("MultiCap [AU]"), "multicap");
}

#[test]
fn strips_bracket_arm64() {
    assert_eq!(normalize_plugin_name("Knife [arm64]"), "knife");
}

#[test]
fn strips_bracket_universal() {
    assert_eq!(normalize_plugin_name("RC-20 [Universal]"), "rc-20");
}

// ── Mixed bracket + paren stacking (regex loops) ──────────────────

#[test]
fn strips_mixed_bracket_then_paren() {
    assert_eq!(normalize_plugin_name("FilterX [VST3] (x64)"), "filterx");
}

#[test]
fn strips_mixed_paren_then_bracket() {
    assert_eq!(normalize_plugin_name("FilterX (x64) [AU]"), "filterx");
}

// ── Whitespace collapse + suffix stripping interaction ────────────

#[test]
fn collapses_internal_whitespace_after_strip() {
    assert_eq!(
        normalize_plugin_name("  FabFilter   Pro-Q   3   (VST3)  "),
        "fabfilter pro-q 3"
    );
}

#[test]
fn collapses_tabs_and_mixed_whitespace() {
    assert_eq!(
        normalize_plugin_name("Massive\t  X  \t (Apple Silicon)"),
        "massive x"
    );
}

// ── Empty-after-strip fallback ────────────────────────────────────

/// If the suffix consumes the entire input, fall back to original
/// lowercased name (no spurious empty string returned).
#[test]
fn empty_after_strip_falls_back_to_lowercased_original() {
    // " (VST3)" — only the suffix remains after trim, but no name body.
    // Implementation: strip yields "" so result is the trimmed lowercased input.
    let r = normalize_plugin_name("(VST3)");
    assert_eq!(r, "(vst3)");
}

// ── Hyphen / underscore / digit preservation ──────────────────────

#[test]
fn preserves_underscored_product_name() {
    assert_eq!(normalize_plugin_name("Massive_X (VST3)"), "massive_x");
}

#[test]
fn preserves_digits_in_product_name() {
    assert_eq!(
        normalize_plugin_name("U-He Diva 1.4.5 (x64)"),
        "u-he diva 1.4.5"
    );
}

// ── Bare suffix without parens (BARE_SUFFIX_RE) ────────────────────

#[test]
fn strips_bare_x86_64_no_parens() {
    assert_eq!(normalize_plugin_name("Sylenth1 x86_64"), "sylenth1");
}

#[test]
fn strips_bare_64bit_no_parens() {
    assert_eq!(normalize_plugin_name("Spire 64bit"), "spire");
}

#[test]
fn strips_bare_32bit_no_parens() {
    assert_eq!(normalize_plugin_name("Old Plugin 32bit"), "old plugin");
}

// ── Mid-string suffix tokens MUST NOT be stripped (`$` anchor) ────

/// `(x64)` mid-string is part of the actual plugin name, not a suffix.
#[test]
fn preserves_mid_string_arch_token() {
    // "Test (x64) Plugin" — the `(x64)` is not at end, so no strip.
    // Whitespace collapses to single spaces.
    let r = normalize_plugin_name("Test (x64) Plugin");
    assert_eq!(r, "test (x64) plugin");
}

#[test]
fn preserves_arch_word_inside_name() {
    // "Intel Synth Editor" — "intel" appears mid-string, not suffix.
    assert_eq!(
        normalize_plugin_name("Intel Synth Editor"),
        "intel synth editor"
    );
}

// ── Already-normalized input is idempotent ────────────────────────

#[test]
fn already_normalized_is_idempotent() {
    let once = normalize_plugin_name("FabFilter Pro-Q 3 (VST3)");
    let twice = normalize_plugin_name(&once);
    assert_eq!(once, twice);
    assert_eq!(once, "fabfilter pro-q 3");
}

// ── Empty / whitespace-only input ─────────────────────────────────

#[test]
fn empty_input_yields_empty() {
    assert_eq!(normalize_plugin_name(""), "");
}

#[test]
fn whitespace_only_input_yields_empty() {
    assert_eq!(normalize_plugin_name("   \t  "), "");
}

// ── Case folding is unconditional ─────────────────────────────────

#[test]
fn unicode_case_folded_to_ascii_lowercase() {
    // "ÜBERMODE" → "übermode" (Rust's to_lowercase handles Unicode).
    assert_eq!(normalize_plugin_name("ÜBERMODE"), "übermode");
}
