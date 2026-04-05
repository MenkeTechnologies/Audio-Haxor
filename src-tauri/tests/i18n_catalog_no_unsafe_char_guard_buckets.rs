//! Bucketed guards: no C0 controls (U+0000–U+001F), DEL (U+007F), or U+2028/U+2029 in any
//! `i18n/app_i18n_*.json` value — matches `test/i18n-value-safety.test.js` `UNSAFE_CHAR` at the
//! Rust `include_str!` seed boundary (NUL is a subset of C0; still asserted per char).

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

#[inline]
fn is_unsafe_catalog_char(c: char) -> bool {
    matches!(c as u32, 0x0000..=0x001F | 0x007F | 0x2028 | 0x2029)
}

fn guard_value(v: &str, locale: &str, key: &str) {
    for c in v.chars() {
        assert!(
            !is_unsafe_catalog_char(c),
            "locale `{locale}` key `{key}` must not contain U+{:04X}",
            c as u32
        );
    }
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
locale_map!(CELL_ZH, map_zh, "app_i18n_zh.json");
locale_map!(CELL_JA, map_ja, "app_i18n_ja.json");

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
    #[test]
    fn zh_bucket~N() {
        check_bucket(map_zh(), "zh", N);
    }
    #[test]
    fn ja_bucket~N() {
        check_bucket(map_ja(), "ja", N);
    }
});
