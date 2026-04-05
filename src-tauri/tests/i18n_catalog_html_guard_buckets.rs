//! Bucketed HTML-injection guards for every `i18n/app_i18n_*.json` row (mirrors
//! `test/i18n-html-injection-guard.test.js`). Each locale map is parsed once (`OnceLock`);
//! `bucket_id(key)` assigns keys to `BUCKETS` shards so failures name a small slice.
//!
//! `seq-macro` expands to `BUCKETS × 11` separate `#[test]` functions (parallel-friendly).

use seq_macro::seq;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::OnceLock;

const BUCKETS: usize = 1024;

fn bucket_id(k: &str) -> usize {
    let mut h = DefaultHasher::new();
    k.hash(&mut h);
    (h.finish() as usize) % BUCKETS
}

fn guard_value(v: &str, locale: &str, key: &str) {
    let lower = v.to_ascii_lowercase();
    assert!(
        !lower.contains("<script"),
        "locale `{locale}` key `{key}` must not contain `<script`"
    );
    assert!(
        !lower.contains("<iframe"),
        "locale `{locale}` key `{key}` must not contain `<iframe`"
    );
}

fn check_bucket(m: &HashMap<String, String>, locale: &str, bucket: usize) {
    for (k, v) in m {
        if bucket_id(k) != bucket {
            continue;
        }
        guard_value(v, locale, k);
    }
}

macro_rules! locale_map {
    ($cell:ident, $getter:ident, $file:literal) => {
        static $cell: OnceLock<HashMap<String, String>> = OnceLock::new();
        fn $getter() -> &'static HashMap<String, String> {
            $cell.get_or_init(|| {
                serde_json::from_str(include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../i18n/",
                    $file
                )))
                .expect(concat!("parse ", $file))
            })
        }
    };
}

locale_map!(CELL_EN, map_en, "app_i18n_en.json");
locale_map!(CELL_DE, map_de, "app_i18n_de.json");
locale_map!(CELL_ES, map_es, "app_i18n_es.json");
locale_map!(CELL_SV, map_sv, "app_i18n_sv.json");
locale_map!(CELL_FR, map_fr, "app_i18n_fr.json");
locale_map!(CELL_NL, map_nl, "app_i18n_nl.json");
locale_map!(CELL_PT, map_pt, "app_i18n_pt.json");
locale_map!(CELL_IT, map_it, "app_i18n_it.json");
locale_map!(CELL_EL, map_el, "app_i18n_el.json");
locale_map!(CELL_PL, map_pl, "app_i18n_pl.json");
locale_map!(CELL_RU, map_ru, "app_i18n_ru.json");

seq!(N in 0..1024 {
    #[test]
    fn en_bucket~N() {
        check_bucket(map_en(), "en", N);
    }
    #[test]
    fn de_bucket~N() {
        check_bucket(map_de(), "de", N);
    }
    #[test]
    fn es_bucket~N() {
        check_bucket(map_es(), "es", N);
    }
    #[test]
    fn sv_bucket~N() {
        check_bucket(map_sv(), "sv", N);
    }
    #[test]
    fn fr_bucket~N() {
        check_bucket(map_fr(), "fr", N);
    }
    #[test]
    fn nl_bucket~N() {
        check_bucket(map_nl(), "nl", N);
    }
    #[test]
    fn pt_bucket~N() {
        check_bucket(map_pt(), "pt", N);
    }
    #[test]
    fn it_bucket~N() {
        check_bucket(map_it(), "it", N);
    }
    #[test]
    fn el_bucket~N() {
        check_bucket(map_el(), "el", N);
    }
    #[test]
    fn pl_bucket~N() {
        check_bucket(map_pl(), "pl", N);
    }
    #[test]
    fn ru_bucket~N() {
        check_bucket(map_ru(), "ru", N);
    }
});
