//! On-disk `i18n/app_i18n_*.json` invariants for headless `cargo test` (no Node).
//!
//! - **Embed parity**: filenames referenced by `include_str!("../../i18n/…")` in
//!   `src/app_i18n.rs` must match exactly the set of `app_i18n_*.json` files in `i18n/`
//!   (add a locale in one place without the other → failure).
//! - **Key parity**: every locale map has the same keys as `app_i18n_en.json`.
//! - **ipc.js `appFmt`**: each `{…}` segment must be `\{\w+\}`; after stripping those,
//!   no `{` or `}` may remain (mirrors `test/i18n-placeholders.test.js`).
//! - **No UTF-8 BOM** at file start (mirrors `test/i18n-catalog-files.test.js`).

use std::collections::HashSet;
use std::fs::{read_dir, read_to_string};
use std::path::{Path, PathBuf};

use regex::Regex;
use serde_json::{Value, from_str};

fn i18n_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../i18n")
}

fn app_i18n_rs_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src/app_i18n.rs")
}

fn disk_app_i18n_json_names() -> Vec<String> {
    let dir = i18n_dir();
    let mut names = Vec::new();
    for ent in read_dir(&dir).unwrap_or_else(|e| panic!("read_dir {}: {e}", dir.display())) {
        let ent = ent.unwrap_or_else(|e| panic!("dir entry: {e}"));
        let n = ent.file_name().to_string_lossy().into_owned();
        if n.starts_with("app_i18n_") && n.ends_with(".json") {
            names.push(n);
        }
    }
    names.sort();
    names
}

fn include_str_seed_json_filenames() -> HashSet<String> {
    let src = read_to_string(app_i18n_rs_path())
        .unwrap_or_else(|e| panic!("read {}: {e}", app_i18n_rs_path().display()));
    let re = Regex::new(r#"include_str!\(\s*"\.\./\.\./i18n/(app_i18n_[^"]+\.json)"\s*\)"#)
        .expect("include_str regex");
    re.captures_iter(&src)
        .map(|c| c.get(1).expect("capture group 1").as_str().to_string())
        .collect()
}

fn assert_ipc_placeholders_ok(file_label: &str, key: &str, value: &str) {
    let brace_seg = Regex::new(r"\{[^}]+\}").expect("brace segment");
    let token_only = Regex::new(r"^\{\w+\}$").expect("token only");
    for m in brace_seg.find_iter(value) {
        let s = m.as_str();
        assert!(
            token_only.is_match(s),
            "{file_label} key {key}: segment {s:?} is not ipc.js appFmt-compatible (use {{word}} letters/digits/_ only)"
        );
    }
    let strip = Regex::new(r"\{\w+\}").expect("strip tokens");
    let rest = strip.replace_all(value, "").to_string();
    assert!(
        !rest.contains('{') && !rest.contains('}'),
        "{file_label} key {key}: stray {{ or }} after removing {{token}} placeholders — rest: {:?}",
        rest.chars().take(120).collect::<String>()
    );
}

#[test]
fn disk_app_i18n_json_set_matches_include_str_in_app_i18n_rs() {
    let from_rs = include_str_seed_json_filenames();
    let from_disk: HashSet<String> = disk_app_i18n_json_names().into_iter().collect();
    assert_eq!(
        from_rs, from_disk,
        "i18n/app_i18n_*.json on disk must match include_str!(../../i18n/…) in src/app_i18n.rs (and tests inside that file)"
    );
}

#[test]
fn all_app_i18n_locales_share_en_keys() {
    let dir = i18n_dir();
    let en_path = dir.join("app_i18n_en.json");
    let en: std::collections::HashMap<String, String> =
        from_str(&read_to_string(&en_path).expect("read en")).expect("parse en json");
    let keys_en: HashSet<_> = en.keys().cloned().collect();

    for name in disk_app_i18n_json_names() {
        if name == "app_i18n_en.json" {
            continue;
        }
        let p = dir.join(&name);
        let map: std::collections::HashMap<String, String> =
            from_str(&read_to_string(&p).unwrap_or_else(|e| panic!("read {}: {e}", p.display())))
                .unwrap_or_else(|e| panic!("parse {}: {e}", name));
        let keys: HashSet<_> = map.keys().cloned().collect();
        assert_eq!(
            keys_en, keys,
            "{name} keys must match app_i18n_en.json exactly"
        );
    }
}

#[test]
fn app_i18n_json_files_do_not_start_with_utf8_bom() {
    let dir = i18n_dir();
    for name in disk_app_i18n_json_names() {
        let p = dir.join(&name);
        let raw = std::fs::read(&p).unwrap_or_else(|e| panic!("read {}: {e}", p.display()));
        assert!(
            !raw.starts_with(&[0xef, 0xbb, 0xbf]),
            "{} must not start with UTF-8 BOM",
            name
        );
    }
}

#[test]
fn app_i18n_json_top_level_keys_are_lexicographically_sorted() {
    let dir = i18n_dir();
    for name in disk_app_i18n_json_names() {
        let p = dir.join(&name);
        let text = read_to_string(&p).unwrap_or_else(|e| panic!("read {}: {e}", p.display()));
        let v: Value = from_str(&text).unwrap_or_else(|e| panic!("parse {}: {e}", name));
        let obj = v
            .as_object()
            .unwrap_or_else(|| panic!("{name}: root must be a JSON object"));
        let keys: Vec<&str> = obj.keys().map(String::as_str).collect();
        for i in 1..keys.len() {
            assert!(
                keys[i] >= keys[i - 1],
                "{}: keys must be sorted — {:?} then {:?}",
                name,
                keys[i - 1],
                keys[i]
            );
        }
    }
}

#[test]
fn app_i18n_values_ipc_placeholder_rules_all_locales() {
    let dir = i18n_dir();
    for name in disk_app_i18n_json_names() {
        let p = dir.join(&name);
        let text = read_to_string(&p).unwrap_or_else(|e| panic!("read {}: {e}", p.display()));
        let map: std::collections::HashMap<String, String> =
            from_str(&text).unwrap_or_else(|e| panic!("parse {}: {e}", name));
        for (k, v) in &map {
            assert_ipc_placeholders_ok(&name, k, v);
        }
    }
}
