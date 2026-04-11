//! Handwritten KVR HTML parsers, serde contracts for update/KVR DTOs, `history` ID helpers,
//! and xref plugin-name normalization. Pure `app_lib` tests (no network, no DB).

use std::cmp::Ordering;
use std::collections::HashSet;

use app_lib::history::{gen_id, radix_string};
use app_lib::kvr::{
    KvrResult, UpdateResult, compare_versions, extract_download_url, extract_version, parse_version,
};
use app_lib::similarity::{AudioFingerprint, fingerprint_distance};
use app_lib::xref::{PluginRef, normalize_plugin_name};

// ── KVR: `extract_download_url` ─────────────────────────────────────────────────

#[test]
fn kvr_extract_download_url_none_when_no_candidate_hrefs() {
    let html = r#"<html><body><p>No download links here.</p></body></html>"#;
    assert!(extract_download_url(html).is_none());
}

#[test]
fn kvr_extract_download_url_fallback_uses_first_buy_get_release_href() {
    // `DOWNLOAD_LINK_RE` matches get|buy|release|download in URL path
    let html = r#"<a href="https://cdn.vendor.com/release/v2.1.0.zip">DL</a>"#;
    let (url, has_platform) = extract_download_url(html).expect("some url");
    assert!(url.contains("release"));
    assert!(!has_platform);
}

#[test]
fn kvr_extract_download_url_platform_keyword_in_url_sets_flag() {
    let html = if cfg!(target_os = "macos") {
        r#"<a href="https://files.example.com/get/osx-universal.dmg">x</a>"#
    } else if cfg!(target_os = "windows") {
        r#"<a href="https://files.example.com/get/windows-x64-installer.exe">x</a>"#
    } else {
        r#"<a href="https://files.example.com/get/linux-amd64.tar.gz">x</a>"#
    };
    let (_, has_platform) = extract_download_url(html).expect("match");
    assert!(has_platform);
}

// ── KVR: `extract_version` + date filter ──────────────────────────────────────

#[test]
fn kvr_extract_version_plain_version_colon_line() {
    let html = r#"<div class="specs">Version: 12.4.1</div>"#;
    assert_eq!(extract_version(html).as_deref(), Some("12.4.1"));
}

#[test]
fn kvr_extract_version_rejects_year_dot_month_looking_semver() {
    let html = r#"<p>Version: 2024.01.15</p>"#;
    assert!(extract_version(html).is_none());
}

#[test]
fn kvr_extract_version_software_version_keyword_snippet() {
    let html = r#"<meta name="softwareVersion" content="7.3.0" />"#;
    assert_eq!(extract_version(html).as_deref(), Some("7.3.0"));
}

// ── KVR: `parse_version` / `compare_versions` edge cases ───────────────────────

#[test]
fn kvr_compare_versions_shorter_vector_pads_with_zero() {
    assert_eq!(compare_versions("1", "1.0.0"), Ordering::Equal);
    assert_eq!(compare_versions("1.0.1", "1"), Ordering::Greater);
}

#[test]
fn kvr_parse_version_numeric_only_segment() {
    assert_eq!(parse_version("42"), vec![42]);
}

#[test]
fn kvr_compare_versions_prerelease_like_segment_zero() {
    // "1.0.0-rc" parses as 1, 0, 0, 0 (non-numeric chunk -> 0)
    let c = compare_versions("1.0.0", "1.0.0-rc");
    assert_eq!(c, Ordering::Equal);
}

// ── KVR / IPC DTO serde ─────────────────────────────────────────────────────────

#[test]
fn kvr_result_serde_roundtrip() {
    let k = KvrResult {
        product_url: "https://www.kvraudio.com/product/x".into(),
        download_url: Some("https://cdn.example.com/get/file.zip".into()),
    };
    let json = serde_json::to_string(&k).unwrap();
    let back: KvrResult = serde_json::from_str(&json).unwrap();
    assert_eq!(back.product_url, k.product_url);
    assert_eq!(back.download_url, k.download_url);
}

#[test]
fn kvr_update_result_serde_roundtrip() {
    let u = UpdateResult {
        latest_version: "2.1.0".into(),
        has_update: true,
        source: "kvr".into(),
        update_url: Some("https://example.com/dl".into()),
        kvr_url: Some("https://kvraudio.com/p/x".into()),
        has_platform_download: true,
    };
    let json = serde_json::to_string(&u).unwrap();
    let back: UpdateResult = serde_json::from_str(&json).unwrap();
    assert_eq!(back.latest_version, "2.1.0");
    assert!(back.has_update);
    assert!(back.has_platform_download);
}

// ── `history::radix_string` / `gen_id` ──────────────────────────────────────────

#[test]
fn radix_string_zero_in_bases() {
    assert_eq!(radix_string(0, 2), "0");
    assert_eq!(radix_string(0, 36), "0");
}

#[test]
fn radix_string_base36_alphabet_upper_bound() {
    assert_eq!(radix_string(35, 36), "z");
    assert_eq!(radix_string(36, 36), "10");
}

#[test]
fn radix_string_base16_lowercase() {
    assert_eq!(radix_string(255, 16), "ff");
}

#[test]
fn gen_id_unique_in_batch_and_base36_chars() {
    let mut set = HashSet::new();
    for _ in 0..80 {
        let id = gen_id();
        assert!(!id.is_empty());
        assert!(
            id.chars().all(|c| matches!(c, '0'..='9' | 'a'..='z')),
            "unexpected char in {id:?}"
        );
        assert!(set.insert(id));
    }
}

// ── Xref: `PluginRef` serde + `normalize_plugin_name` ──────────────────────────

#[test]
fn xref_plugin_ref_serde_roundtrip() {
    let p = PluginRef {
        name: "Pro-Q 4".into(),
        normalized_name: "pro-q 4".into(),
        manufacturer: "FabFilter".into(),
        plugin_type: "VST3".into(),
    };
    let json = serde_json::to_string(&p).unwrap();
    let q: PluginRef = serde_json::from_str(&json).unwrap();
    assert_eq!(p, q);
}

#[test]
fn normalize_plugin_name_strips_trailing_vst3_suffix() {
    // `vst2` is not matched by the `vst3?` token in `ARCH_SUFFIX_RE` (only `vst` / `vst3`).
    assert_eq!(normalize_plugin_name("Legacy (VST3)"), "legacy");
}

#[test]
fn normalize_plugin_name_preserves_hyphenated_product_name() {
    assert_eq!(
        normalize_plugin_name("FabFilter Pro-Q 3 (VST3)"),
        "fabfilter pro-q 3"
    );
}

#[test]
fn normalize_plugin_name_bracket_arm64_suffix() {
    assert_eq!(normalize_plugin_name("Synth [arm64]"), "synth");
}

#[test]
fn normalize_plugin_name_internal_digits_preserved() {
    assert_eq!(normalize_plugin_name("EQ Eight 2 (AU)"), "eq eight 2");
}

// ── Similarity: distance sanity ─────────────────────────────────────────────────

fn mk_fp(path: &str, rms: f64, zcr: f64) -> AudioFingerprint {
    AudioFingerprint {
        path: path.into(),
        rms,
        spectral_centroid: 0.2,
        zero_crossing_rate: zcr,
        low_band_energy: 0.25,
        mid_band_energy: 0.35,
        high_band_energy: 0.4,
        low_energy_ratio: 0.5,
        attack_time: 0.02,
    }
}

#[test]
fn fingerprint_distance_nonnegative_for_arbitrary_pair() {
    let a = mk_fp("/a.wav", 0.1, 0.01);
    let b = mk_fp("/b.wav", 0.9, 0.4);
    let d = fingerprint_distance(&a, &b);
    assert!(d >= 0.0);
    assert!(d.is_finite());
}

#[test]
fn fingerprint_distance_symmetric_off_diagonal() {
    let a = mk_fp("/x.wav", 0.33, 0.11);
    let b = mk_fp("/y.wav", 0.44, 0.22);
    let d_ab = fingerprint_distance(&a, &b);
    let d_ba = fingerprint_distance(&b, &a);
    assert!((d_ab - d_ba).abs() < 1e-12);
}

// ── `format_size` corner cases (crate root) ─────────────────────────────────────

#[test]
fn format_size_single_byte() {
    assert_eq!(app_lib::format_size(1), "1.0 B");
}

#[test]
fn format_size_1023_bytes_stays_byte_tier() {
    let s = app_lib::format_size(1023);
    assert!(s.ends_with(" B"), "{s}");
}

#[test]
fn format_size_exactly_one_kib() {
    assert_eq!(app_lib::format_size(1024), "1.0 KB");
}

#[test]
fn kvr_extract_download_url_first_platform_specific_link_wins_over_generic() {
    let html = if cfg!(target_os = "macos") {
        r#"<a href="https://cdn.example.com/get/mac-app.dmg"></a><a href="https://other.com/release/pkg.zip"></a>"#
    } else if cfg!(target_os = "windows") {
        r#"<a href="https://cdn.example.com/get/windows-setup.exe"></a><a href="https://other.com/release/pkg.zip"></a>"#
    } else {
        r#"<a href="https://cdn.example.com/get/linux.tar.gz"></a><a href="https://other.com/release/pkg.zip"></a>"#
    };
    let (url, has_platform) = extract_download_url(html).expect("url");
    assert!(has_platform);
    assert!(url.contains("cdn.example.com"));
}

#[test]
fn kvr_compare_versions_all_equal_chain() {
    let v = ["1.0.0", "1.0.0", "1.0.0"];
    for i in 0..v.len() {
        for j in 0..v.len() {
            assert_eq!(compare_versions(v[i], v[j]), Ordering::Equal);
        }
    }
}

#[test]
fn radix_string_base2_powers() {
    assert_eq!(radix_string(1, 2), "1");
    assert_eq!(radix_string(2, 2), "10");
    assert_eq!(radix_string(8, 2), "1000");
}

#[test]
fn radix_string_one_million_base36() {
    assert_eq!(radix_string(1_000_000, 36), "lfls");
}

#[test]
fn compare_versions_matches_lexicographic_vec_cmp_when_same_segment_count() {
    let a = "8.7.6";
    let b = "8.7.7";
    assert_eq!(compare_versions(a, b), Ordering::Less);
    assert_eq!(parse_version(a).cmp(&parse_version(b)), Ordering::Less);
}

#[test]
fn normalize_plugin_name_empty_input() {
    assert_eq!(normalize_plugin_name(""), "");
}

#[test]
fn normalize_plugin_name_only_whitespace() {
    assert_eq!(normalize_plugin_name("   \t\n  "), "");
}

#[test]
fn normalize_plugin_name_strips_intel_bracket_suffix() {
    assert_eq!(normalize_plugin_name("Bus [Intel]"), "bus");
}
