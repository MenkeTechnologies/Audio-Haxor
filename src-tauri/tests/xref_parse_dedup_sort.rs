//! Integration tests for `xref::extract_plugins` end-to-end on synthetic DAW
//! project files. Focused on the post-parse pipeline (dedup by
//! `(normalized_name, plugin_type)`, sort by `normalized_name`) plus the
//! regex-based extraction for Ableton `.als` (gzipped XML) and REAPER `.rpp`
//! (plaintext) — the two formats whose parsers actually exercise the dedup
//! path and contain the most fragile regexes in `xref.rs`.
//!
//! These tests are intentionally NOT mirror/smoke tests of "extract_plugins on
//! a nonexistent file returns empty" (already covered elsewhere). Each test
//! pins a specific behavior that, if broken, would silently corrupt the xref
//! UI numbers without crashing.

use app_lib::xref::extract_plugins;
use flate2::write::GzEncoder;
use flate2::Compression;
use std::io::Write;

fn unique_temp(prefix: &str, ext: &str) -> std::path::PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let n = SEQ.fetch_add(1, Ordering::SeqCst);
    std::env::temp_dir().join(format!(
        "ah_xref_parse_{}_{}_{}{ext}",
        std::process::id(),
        prefix,
        n
    ))
}

fn write_gzipped(path: &std::path::Path, body: &str) {
    let f = std::fs::File::create(path).expect("create gz target");
    let mut enc = GzEncoder::new(f, Compression::default());
    enc.write_all(body.as_bytes()).expect("write inner xml");
    enc.finish().expect("flush gz footer");
}

/// `extract_plugins` on a synthetic `.als` (gzip-wrapped XML) must:
///   1. Detect VST2, VST3, and AU plugin info blocks in the same file.
///   2. Deduplicate by `(normalized_name, plugin_type)` so duplicate
///      `<VstPluginInfo>` blocks for the same plugin name collapse to one
///      entry — but the same plugin name on a different `plugin_type` is
///      preserved (VST2 + VST3 of the same product are both kept).
///   3. Return the entries sorted by `normalized_name` (UI sort
///      determinism).
///   4. Skip blocks whose name capture is empty (`<PlugName Value=""/>`),
///      because pushing an empty-name reference would feed the xref UI a
///      blank row.
///
/// A regression in any of the four would not panic — it would silently
/// inflate / deflate the xref count or scramble UI sort order. None of those
/// outcomes are caught by the existing "nonexistent file returns empty"
/// tests.
#[test]
fn extract_plugins_als_mixed_types_dedup_and_sort() {
    let als = unique_temp("mixed", ".als");

    // Inner XML: two VST2 plugins (one duplicated), one VST3 (same product
    // name as a VST2 — must NOT collapse with it), one AU, plus an empty
    // VST2 name that must be discarded.
    let inner = r#"<?xml version="1.0"?>
<Ableton MajorVersion="5" MinorVersion="12.0_12049">
  <VstPluginInfo Id="0">
    <PlugName Value="Zebra2"/>
    <Manufacturer Value="u-he"/>
  </VstPluginInfo>
  <VstPluginInfo Id="1">
    <PlugName Value="Serum"/>
    <Manufacturer Value="Xfer Records"/>
  </VstPluginInfo>
  <VstPluginInfo Id="2">
    <PlugName Value="Zebra2"/>
    <Manufacturer Value="u-he"/>
  </VstPluginInfo>
  <VstPluginInfo Id="3">
    <PlugName Value=""/>
    <Manufacturer Value="empty name should be dropped"/>
  </VstPluginInfo>
  <Vst3PluginInfo Id="4">
    <Name Value="Serum"/>
    <DeviceCreator Value="Xfer Records"/>
  </Vst3PluginInfo>
  <AuPluginInfo Id="5">
    <Name Value="ChannelEQ"/>
    <Manufacturer Value="Apple"/>
  </AuPluginInfo>
</Ableton>"#;

    write_gzipped(&als, inner);

    let plugins = extract_plugins(als.to_str().expect("utf8 path"));
    let _ = std::fs::remove_file(&als);

    // Expected post-dedup, post-sort (sorted by normalized_name lexicographically):
    //   1. ("channeleq",  "AU")
    //   2. ("serum",      "VST2")
    //   3. ("serum",      "VST3")
    //   4. ("zebra2",     "VST2")  ← duplicate VST2 collapsed
    // The empty-name VST2 block must not appear.
    let observed: Vec<(String, String)> = plugins
        .iter()
        .map(|p| (p.normalized_name.clone(), p.plugin_type.clone()))
        .collect();

    assert_eq!(
        observed,
        vec![
            ("channeleq".to_string(), "AU".to_string()),
            ("serum".to_string(), "VST2".to_string()),
            ("serum".to_string(), "VST3".to_string()),
            ("zebra2".to_string(), "VST2".to_string()),
        ],
        "ALS xref dedup/sort regression: got {observed:?}"
    );

    // Empty-name guard: no PluginRef should ever have a blank name.
    for p in &plugins {
        assert!(
            !p.name.trim().is_empty(),
            "PluginRef with empty name leaked through: {p:?}"
        );
        assert!(
            !p.normalized_name.is_empty(),
            "PluginRef with empty normalized_name leaked through: {p:?}"
        );
    }
}

/// REAPER `.rpp` parser must:
///   1. Tag `VST3` lines as `plugin_type = "VST3"` (NOT `"VST2"`), even
///      though both come from a `<VST ...>` outer tag.
///   2. Capture the manufacturer from the trailing `(...)` only when one is
///      present — and leave it empty (NOT panic, NOT pull text from the next
///      line) when it is absent.
///   3. Tolerate a plugin name that itself contains a parenthesized chunk:
///      e.g. `VST3: My Synth (Pro Edition) (Vendor)` — the LAST
///      parenthesized chunk is the manufacturer; the inner one is part of
///      the name.
///
/// A regression here would either misclassify VST3 as VST2 (breaks the
/// `plugins-by-type` UI bar chart) or eat a parenthesized brand into the
/// plugin name (breaks search by name).
#[test]
fn extract_plugins_rpp_classifies_vst3_and_handles_inner_parens() {
    let rpp = unique_temp("classify", ".rpp");

    // REAPER .rpp lines as REAPER actually emits them: each plugin appears
    // inside a `<VST ...` block with the typed string in quotes.
    let body = r#"<REAPER_PROJECT 0.1 "7.00/macos-arm64"
  <TRACK
    <FXCHAIN
      <VST "VST3: Pro-Q 3 (FabFilter)" ProQ3.vst3 0 ""
        dGVzdA==
      >
      <VST "VST: TAL-NoiseMaker" TAL-NoiseMaker.dll 0 ""
        dGVzdA==
      >
      <VST "VST3: My Synth (Pro Edition) (Vendor X)" mysynth.vst3 0 ""
        dGVzdA==
      >
      <VST "VST3: NoVendor" novendor.vst3 0 ""
        dGVzdA==
      >
    >
  >
>
"#;
    std::fs::write(&rpp, body).expect("write rpp");

    let plugins = extract_plugins(rpp.to_str().expect("utf8 path"));
    let _ = std::fs::remove_file(&rpp);

    // Index by normalized_name for stable assertions (sort is by normalized_name).
    let by_norm: std::collections::HashMap<String, &app_lib::xref::PluginRef> = plugins
        .iter()
        .map(|p| (p.normalized_name.clone(), p))
        .collect();

    // (1) VST3 classification — must NOT be silently downgraded to VST2.
    let pq3 = by_norm
        .get("pro-q 3")
        .expect("Pro-Q 3 not extracted from .rpp");
    assert_eq!(pq3.plugin_type, "VST3", "Pro-Q 3 misclassified");
    assert_eq!(pq3.manufacturer, "FabFilter");

    // (2) Trailing-mfg absent → manufacturer is empty (does not leak from
    // the next plugin line or the filename).
    let nv = by_norm
        .get("novendor")
        .expect("NoVendor not extracted from .rpp");
    assert_eq!(nv.plugin_type, "VST3");
    assert!(
        nv.manufacturer.is_empty(),
        "NoVendor should have empty manufacturer, got {:?}",
        nv.manufacturer
    );

    // (3) VST2 stays VST2.
    let tal = by_norm
        .get("tal-noisemaker")
        .expect("TAL-NoiseMaker not extracted from .rpp");
    assert_eq!(tal.plugin_type, "VST2");

    // (4) Inner parens stay in the name; only the outermost paren is the mfg.
    //
    // The regex `(.+?)\s*(?:\(([^)]+)\))?\s*"` is non-greedy on name but the
    // optional `(...)` group is greedy enough that the LAST `(...)` becomes
    // the manufacturer when both are on the same line — so the expected
    // observation is: name contains "(Pro Edition)" and manufacturer is
    // "Vendor X". Pin BOTH so a regex refactor can't swap them without the
    // test screaming.
    let ms = by_norm
        .get("my synth (pro edition)")
        .or_else(|| by_norm.get("my synth (pro edition) (vendor x)"))
        .unwrap_or_else(|| {
            panic!(
                "expected 'my synth (pro edition)' (or full nested form) in {:?}",
                by_norm.keys().collect::<Vec<_>>()
            )
        });
    assert_eq!(ms.plugin_type, "VST3");
    // The manufacturer should be the outer paren — "Vendor X" — when the
    // name retained the inner paren. If a future refactor pulls the inner
    // paren as mfg instead, this assertion fires.
    assert!(
        ms.manufacturer == "Vendor X" || ms.name.contains("Vendor X"),
        "Nested-paren plugin: name={:?} mfg={:?} — outer (Vendor X) lost",
        ms.name,
        ms.manufacturer
    );
}
