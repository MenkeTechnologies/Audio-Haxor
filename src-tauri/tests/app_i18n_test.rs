//! Integration tests for SQLite-backed UI strings (`app_i18n` merge + `Database::get_app_strings`).
//! Uses the real on-disk DB via `init_global` (same pattern as `tests/db_test.rs`).
//!
//! Seed JSON parity (e.g. French vs English for the same key) is covered by unit tests in
//! `app_i18n::tests` — the on-disk DB can retain older `INSERT OR IGNORE` rows across upgrades.

use std::sync::Once;

static INIT_DB: Once = Once::new();

fn ensure_db() {
    INIT_DB.call_once(|| {
        app_lib::db::init_global().expect("init_global for app_i18n tests");
    });
}

#[test]
fn app_strings_en_contains_core_menu_keys() {
    ensure_db();
    let m = app_lib::db::global()
        .get_app_strings("en")
        .expect("get_app_strings en");
    for key in [
        "menu.scan_all",
        "menu.tab_plugins",
        "menu.scan_daw",
        "menu.about",
    ] {
        assert!(
            m.get(key).map(|s| !s.is_empty()).unwrap_or(false),
            "missing or empty key: {key}"
        );
    }
}

#[test]
fn app_strings_all_supported_locales_have_substantial_maps() {
    ensure_db();
    for loc in ["en", "de", "es", "sv", "fr"] {
        let m = app_lib::db::global()
            .get_app_strings(loc)
            .unwrap_or_else(|e| panic!("get_app_strings {loc}: {e}"));
        assert!(
            m.len() > 200,
            "locale {loc} should expose many keys, got {}",
            m.len()
        );
        assert!(
            m.contains_key("menu.scan_all"),
            "locale {loc} missing menu.scan_all"
        );
    }
}

#[test]
fn app_strings_unknown_locale_falls_back_to_english_values() {
    ensure_db();
    let en = app_lib::db::global()
        .get_app_strings("en")
        .expect("en");
    let pseudo = app_lib::db::global()
        .get_app_strings("zz")
        .expect("zz");
    assert_eq!(
        pseudo.get("menu.scan_all"),
        en.get("menu.scan_all"),
        "unknown locale should keep English for keys without zz rows"
    );
}
