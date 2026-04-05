//! App UI strings for i18n: seeded into SQLite (`app_i18n` table) from `i18n/app_i18n_en.json`
//! (toasts, menus, tray, HTML `data-i18n*`, dialogs). Locales `de`, `es`, `sv`, `fr`, `nl`, `pt`, `it`, `el`, `pl`, `ru`, `zh`, `ja` add rows with the same keys.

use rusqlite::{params, Connection};
use std::collections::HashMap;

static SEED_JSON_EN: &str = include_str!("../../i18n/app_i18n_en.json");
static SEED_JSON_DE: &str = include_str!("../../i18n/app_i18n_de.json");
static SEED_JSON_ES: &str = include_str!("../../i18n/app_i18n_es.json");
static SEED_JSON_SV: &str = include_str!("../../i18n/app_i18n_sv.json");
static SEED_JSON_FR: &str = include_str!("../../i18n/app_i18n_fr.json");
static SEED_JSON_PT: &str = include_str!("../../i18n/app_i18n_pt.json");
static SEED_JSON_NL: &str = include_str!("../../i18n/app_i18n_nl.json");
static SEED_JSON_IT: &str = include_str!("../../i18n/app_i18n_it.json");
static SEED_JSON_EL: &str = include_str!("../../i18n/app_i18n_el.json");
static SEED_JSON_PL: &str = include_str!("../../i18n/app_i18n_pl.json");
static SEED_JSON_RU: &str = include_str!("../../i18n/app_i18n_ru.json");
static SEED_JSON_ZH: &str = include_str!("../../i18n/app_i18n_zh.json");
static SEED_JSON_JA: &str = include_str!("../../i18n/app_i18n_ja.json");

/// Insert default locale rows (`INSERT OR REPLACE` on `(key, locale)` primary key) on every
/// migration so shipped `i18n/app_i18n_*.json` values stay current. There is no separate UI to
/// edit `app_i18n` rows; the catalog is the source of truth.
pub fn seed_defaults(conn: &Connection) -> Result<(), String> {
    seed_locale(conn, "en", SEED_JSON_EN)?;
    seed_locale(conn, "de", SEED_JSON_DE)?;
    seed_locale(conn, "es", SEED_JSON_ES)?;
    seed_locale(conn, "sv", SEED_JSON_SV)?;
    seed_locale(conn, "fr", SEED_JSON_FR)?;
    seed_locale(conn, "pt", SEED_JSON_PT)?;
    seed_locale(conn, "nl", SEED_JSON_NL)?;
    seed_locale(conn, "it", SEED_JSON_IT)?;
    seed_locale(conn, "el", SEED_JSON_EL)?;
    seed_locale(conn, "pl", SEED_JSON_PL)?;
    seed_locale(conn, "ru", SEED_JSON_RU)?;
    seed_locale(conn, "zh", SEED_JSON_ZH)?;
    seed_locale(conn, "ja", SEED_JSON_JA)?;
    Ok(())
}

fn seed_locale(conn: &Connection, locale: &str, json: &str) -> Result<(), String> {
    let map: HashMap<String, String> = serde_json::from_str(json).map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare_cached("INSERT OR REPLACE INTO app_i18n (key, locale, value) VALUES (?1, ?2, ?3)")
        .map_err(|e| e.to_string())?;
    for (k, v) in map {
        stmt.execute(params![k, locale, v]).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Merge English with `locale` (falls back to English for missing keys).
pub fn load_merged(conn: &Connection, locale: &str) -> Result<HashMap<String, String>, String> {
    let mut out: HashMap<String, String> = HashMap::new();
    let mut stmt = conn
        .prepare("SELECT key, value FROM app_i18n WHERE locale = 'en'")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))
        .map_err(|e| e.to_string())?;
    for row in rows {
        let (k, v) = row.map_err(|e| e.to_string())?;
        out.insert(k, v);
    }
    if locale == "en" || locale.is_empty() {
        return Ok(out);
    }
    let mut stmt = conn
        .prepare("SELECT key, value FROM app_i18n WHERE locale = ?1")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![locale], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|e| e.to_string())?;
    for row in rows {
        let (k, v) = row.map_err(|e| e.to_string())?;
        out.insert(k, v);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::{
        load_merged, SEED_JSON_DE, SEED_JSON_EL, SEED_JSON_EN, SEED_JSON_ES, SEED_JSON_FR, SEED_JSON_IT,
        SEED_JSON_JA, SEED_JSON_NL, SEED_JSON_PL, SEED_JSON_PT, SEED_JSON_RU, SEED_JSON_SV, SEED_JSON_ZH,
    };
    use regex::Regex;
    use rusqlite::Connection;
    use std::collections::{HashMap, HashSet};

    /// Keys passed to `t("…", …)` in `native_menu.rs` — English seed must define each.
    const NATIVE_MENU_BAR_KEYS: &[&str] = &[
        "menu.about",
        "menu.app",
        "menu.check_updates",
        "menu.clear_favorites",
        "menu.clear_history",
        "menu.clear_kvr",
        "menu.cmd_palette",
        "menu.data",
        "menu.dep_graph",
        "menu.docs",
        "menu.edit",
        "menu.expand_player",
        "menu.export_daw",
        "menu.export_plugins",
        "menu.export_presets",
        "menu.export_samples",
        "menu.file",
        "menu.find",
        "menu.find_duplicates",
        "menu.github",
        "menu.help",
        "menu.help_overlay",
        "menu.import_daw",
        "menu.import_plugins",
        "menu.import_presets",
        "menu.import_samples",
        "menu.next_track",
        "menu.play_pause",
        "menu.playback",
        "menu.preferences",
        "menu.prev_track",
        "menu.reset_all_scans",
        "menu.reset_columns",
        "menu.reset_tabs",
        "menu.scan",
        "menu.scan_all",
        "menu.scan_daw",
        "menu.scan_plugins",
        "menu.scan_presets",
        "menu.scan_samples",
        "menu.stop_all",
        "menu.stop_playback",
        "menu.tab_daw",
        "menu.tab_favorites",
        "menu.tab_files",
        "menu.tab_history",
        "menu.tab_notes",
        "menu.tab_plugins",
        "menu.tab_presets",
        "menu.tab_samples",
        "menu.tab_settings",
        "menu.toggle_crt",
        "menu.toggle_loop",
        "menu.toggle_mute",
        "menu.toggle_shuffle",
        "menu.toggle_theme",
        "menu.tools",
        "menu.view",
        "menu.window",
    ];

    /// Keys passed to `t("tray.…", …)` for the system tray in `lib.rs`.
    const TRAY_KEYS: &[&str] = &[
        "tray.show",
        "tray.scan_all",
        "tray.stop_all",
        "tray.play_pause",
        "tray.next_track",
        "tray.quit",
        "tray.tooltip",
    ];

    fn key_matches_catalog_prefix(k: &str) -> bool {
        [
            "menu.", "tray.", "confirm.", "toast.", "help.", "ui.",
        ]
        .iter()
        .any(|p| k.starts_with(p))
    }

    fn setup_minimal_i18n(conn: &Connection) {
        conn.execute_batch(
            "CREATE TABLE app_i18n (
                key TEXT NOT NULL,
                locale TEXT NOT NULL,
                value TEXT NOT NULL,
                PRIMARY KEY (key, locale)
            );",
        )
        .expect("create app_i18n");
        conn.execute(
            "INSERT INTO app_i18n (key, locale, value) VALUES ('k1', 'en', 'english')",
            [],
        )
        .expect("insert k1 en");
        conn.execute(
            "INSERT INTO app_i18n (key, locale, value) VALUES ('k2', 'en', 'only-en')",
            [],
        )
        .expect("insert k2 en");
        conn.execute(
            "INSERT INTO app_i18n (key, locale, value) VALUES ('k1', 'fr', 'french')",
            [],
        )
        .expect("insert k1 fr");
    }

    #[test]
    fn load_merged_fr_overrides_shared_key_keeps_en_only_keys() {
        let conn = Connection::open_in_memory().expect("in memory");
        setup_minimal_i18n(&conn);
        let m = load_merged(&conn, "fr").expect("merged fr");
        assert_eq!(m.get("k1").map(String::as_str), Some("french"));
        assert_eq!(m.get("k2").map(String::as_str), Some("only-en"));
    }

    #[test]
    fn load_merged_en_and_empty_locale_skip_overlay() {
        let conn = Connection::open_in_memory().expect("in memory");
        setup_minimal_i18n(&conn);
        for loc in ["en", ""] {
            let m = load_merged(&conn, loc).expect("merged");
            assert_eq!(m.get("k1").map(String::as_str), Some("english"));
            assert_eq!(m.get("k2").map(String::as_str), Some("only-en"));
        }
    }

    #[test]
    fn load_merged_unknown_locale_yields_english_base_only() {
        let conn = Connection::open_in_memory().expect("in memory");
        setup_minimal_i18n(&conn);
        let m = load_merged(&conn, "xx").expect("merged xx");
        assert_eq!(m.get("k1").map(String::as_str), Some("english"));
        assert_eq!(m.get("k2").map(String::as_str), Some("only-en"));
    }

    #[test]
    fn load_merged_overlay_empty_string_replaces_english_value() {
        let conn = Connection::open_in_memory().expect("in memory");
        conn.execute_batch(
            "CREATE TABLE app_i18n (
                key TEXT NOT NULL,
                locale TEXT NOT NULL,
                value TEXT NOT NULL,
                PRIMARY KEY (key, locale)
            );",
        )
        .expect("create app_i18n");
        conn.execute(
            "INSERT INTO app_i18n (key, locale, value) VALUES ('k1', 'en', 'english')",
            [],
        )
        .expect("insert k1 en");
        conn.execute(
            "INSERT INTO app_i18n (key, locale, value) VALUES ('k1', 'de', '')",
            [],
        )
        .expect("insert k1 de empty");
        let m = load_merged(&conn, "de").expect("merged de");
        assert_eq!(m.get("k1").map(String::as_str), Some(""));
    }

    #[test]
    fn seed_json_en_parses() {
        let map: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        assert!(
            map.len() > 100,
            "English seed should contain a large string table"
        );
        assert!(map.contains_key("menu.scan_all"));
    }

    #[test]
    fn seed_json_de_menu_scan_all_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let de: HashMap<String, String> = serde_json::from_str(SEED_JSON_DE).expect("de json");
        assert_ne!(
            en.get("menu.scan_all"),
            de.get("menu.scan_all"),
            "German seed should translate menu.scan_all (same key, different value)"
        );
    }

    #[test]
    fn seed_json_es_menu_scan_all_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let es: HashMap<String, String> = serde_json::from_str(SEED_JSON_ES).expect("es json");
        assert_ne!(
            en.get("menu.scan_all"),
            es.get("menu.scan_all"),
            "Spanish seed should translate menu.scan_all (same key, different value)"
        );
    }

    #[test]
    fn seed_json_sv_menu_scan_all_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let sv: HashMap<String, String> = serde_json::from_str(SEED_JSON_SV).expect("sv json");
        assert_ne!(
            en.get("menu.scan_all"),
            sv.get("menu.scan_all"),
            "Swedish seed should translate menu.scan_all (same key, different value)"
        );
    }

    #[test]
    fn seed_json_fr_menu_scan_all_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let fr: HashMap<String, String> = serde_json::from_str(include_str!(
            "../../i18n/app_i18n_fr.json"
        ))
        .expect("fr json");
        assert_ne!(
            en.get("menu.scan_all"),
            fr.get("menu.scan_all"),
            "French seed should translate menu.scan_all (same key, different value)"
        );
    }

    #[test]
    fn seed_json_pt_menu_scan_all_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let pt: HashMap<String, String> = serde_json::from_str(SEED_JSON_PT).expect("pt json");
        assert_ne!(
            en.get("menu.scan_all"),
            pt.get("menu.scan_all"),
            "Portuguese seed should translate menu.scan_all (same key, different value)"
        );
    }

    #[test]
    fn seed_json_nl_menu_scan_all_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let nl: HashMap<String, String> = serde_json::from_str(SEED_JSON_NL).expect("nl json");
        assert_ne!(
            en.get("menu.scan_all"),
            nl.get("menu.scan_all"),
            "Dutch seed should translate menu.scan_all (same key, different value)"
        );
    }

    #[test]
    fn seed_json_it_menu_scan_all_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let it: HashMap<String, String> = serde_json::from_str(SEED_JSON_IT).expect("it json");
        assert_ne!(
            en.get("menu.scan_all"),
            it.get("menu.scan_all"),
            "Italian seed should translate menu.scan_all (same key, different value)"
        );
    }

    #[test]
    fn seed_json_el_menu_scan_all_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let el: HashMap<String, String> = serde_json::from_str(SEED_JSON_EL).expect("el json");
        assert_ne!(
            en.get("menu.scan_all"),
            el.get("menu.scan_all"),
            "Greek seed should translate menu.scan_all (same key, different value)"
        );
    }

    #[test]
    fn seed_json_pl_menu_scan_all_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let pl: HashMap<String, String> = serde_json::from_str(SEED_JSON_PL).expect("pl json");
        assert_ne!(
            en.get("menu.scan_all"),
            pl.get("menu.scan_all"),
            "Polish seed should translate menu.scan_all (same key, different value)"
        );
    }

    #[test]
    fn seed_json_ru_menu_scan_all_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let ru: HashMap<String, String> = serde_json::from_str(SEED_JSON_RU).expect("ru json");
        assert_ne!(
            en.get("menu.scan_all"),
            ru.get("menu.scan_all"),
            "Russian seed should translate menu.scan_all (same key, different value)"
        );
    }

    #[test]
    fn seed_json_zh_menu_scan_all_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let zh: HashMap<String, String> = serde_json::from_str(SEED_JSON_ZH).expect("zh json");
        assert_ne!(
            en.get("menu.scan_all"),
            zh.get("menu.scan_all"),
            "Chinese seed should translate menu.scan_all (same key, different value)"
        );
    }

    #[test]
    fn seed_json_ja_menu_scan_all_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let ja: HashMap<String, String> = serde_json::from_str(SEED_JSON_JA).expect("ja json");
        assert_ne!(
            en.get("menu.scan_all"),
            ja.get("menu.scan_all"),
            "Japanese seed should translate menu.scan_all (same key, different value)"
        );
    }

    #[test]
    fn seed_json_de_tray_play_pause_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let de: HashMap<String, String> = serde_json::from_str(SEED_JSON_DE).expect("de json");
        assert_ne!(
            en.get("tray.play_pause"),
            de.get("tray.play_pause"),
            "German seed should translate tray.play_pause (same key, different value)"
        );
    }

    #[test]
    fn seed_json_es_tray_play_pause_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let es: HashMap<String, String> = serde_json::from_str(SEED_JSON_ES).expect("es json");
        assert_ne!(
            en.get("tray.play_pause"),
            es.get("tray.play_pause"),
            "Spanish seed should translate tray.play_pause (same key, different value)"
        );
    }

    #[test]
    fn seed_json_sv_tray_play_pause_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let sv: HashMap<String, String> = serde_json::from_str(SEED_JSON_SV).expect("sv json");
        assert_ne!(
            en.get("tray.play_pause"),
            sv.get("tray.play_pause"),
            "Swedish seed should translate tray.play_pause (same key, different value)"
        );
    }

    #[test]
    fn seed_json_fr_tray_play_pause_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let fr: HashMap<String, String> = serde_json::from_str(SEED_JSON_FR).expect("fr json");
        assert_ne!(
            en.get("tray.play_pause"),
            fr.get("tray.play_pause"),
            "French seed should translate tray.play_pause (same key, different value)"
        );
    }

    #[test]
    fn seed_json_pt_tray_play_pause_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let pt: HashMap<String, String> = serde_json::from_str(SEED_JSON_PT).expect("pt json");
        assert_ne!(
            en.get("tray.play_pause"),
            pt.get("tray.play_pause"),
            "Portuguese seed should translate tray.play_pause (same key, different value)"
        );
    }

    #[test]
    fn seed_json_nl_tray_play_pause_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let nl: HashMap<String, String> = serde_json::from_str(SEED_JSON_NL).expect("nl json");
        assert_ne!(
            en.get("tray.play_pause"),
            nl.get("tray.play_pause"),
            "Dutch seed should translate tray.play_pause (same key, different value)"
        );
    }

    #[test]
    fn seed_json_it_tray_play_pause_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let it: HashMap<String, String> = serde_json::from_str(SEED_JSON_IT).expect("it json");
        assert_ne!(
            en.get("tray.play_pause"),
            it.get("tray.play_pause"),
            "Italian seed should translate tray.play_pause (same key, different value)"
        );
    }

    #[test]
    fn seed_json_el_tray_play_pause_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let el: HashMap<String, String> = serde_json::from_str(SEED_JSON_EL).expect("el json");
        assert_ne!(
            en.get("tray.play_pause"),
            el.get("tray.play_pause"),
            "Greek seed should translate tray.play_pause (same key, different value)"
        );
    }

    #[test]
    fn seed_json_pl_tray_play_pause_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let pl: HashMap<String, String> = serde_json::from_str(SEED_JSON_PL).expect("pl json");
        assert_ne!(
            en.get("tray.play_pause"),
            pl.get("tray.play_pause"),
            "Polish seed should translate tray.play_pause (same key, different value)"
        );
    }

    #[test]
    fn seed_json_ru_tray_play_pause_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let ru: HashMap<String, String> = serde_json::from_str(SEED_JSON_RU).expect("ru json");
        assert_ne!(
            en.get("tray.play_pause"),
            ru.get("tray.play_pause"),
            "Russian seed should translate tray.play_pause (same key, different value)"
        );
    }

    #[test]
    fn seed_json_zh_tray_play_pause_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let zh: HashMap<String, String> = serde_json::from_str(SEED_JSON_ZH).expect("zh json");
        assert_ne!(
            en.get("tray.play_pause"),
            zh.get("tray.play_pause"),
            "Chinese seed should translate tray.play_pause (same key, different value)"
        );
    }

    #[test]
    fn seed_json_ja_tray_play_pause_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let ja: HashMap<String, String> = serde_json::from_str(SEED_JSON_JA).expect("ja json");
        assert_ne!(
            en.get("tray.play_pause"),
            ja.get("tray.play_pause"),
            "Japanese seed should translate tray.play_pause (same key, different value)"
        );
    }

    #[test]
    fn seed_json_de_tray_stop_all_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let de: HashMap<String, String> = serde_json::from_str(SEED_JSON_DE).expect("de json");
        assert_ne!(
            en.get("tray.stop_all"),
            de.get("tray.stop_all"),
            "German seed should translate tray.stop_all (same key, different value)"
        );
    }

    #[test]
    fn seed_json_es_tray_stop_all_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let es: HashMap<String, String> = serde_json::from_str(SEED_JSON_ES).expect("es json");
        assert_ne!(
            en.get("tray.stop_all"),
            es.get("tray.stop_all"),
            "Spanish seed should translate tray.stop_all (same key, different value)"
        );
    }

    #[test]
    fn seed_json_sv_tray_stop_all_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let sv: HashMap<String, String> = serde_json::from_str(SEED_JSON_SV).expect("sv json");
        assert_ne!(
            en.get("tray.stop_all"),
            sv.get("tray.stop_all"),
            "Swedish seed should translate tray.stop_all (same key, different value)"
        );
    }

    #[test]
    fn seed_json_fr_tray_stop_all_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let fr: HashMap<String, String> = serde_json::from_str(SEED_JSON_FR).expect("fr json");
        assert_ne!(
            en.get("tray.stop_all"),
            fr.get("tray.stop_all"),
            "French seed should translate tray.stop_all (same key, different value)"
        );
    }

    #[test]
    fn seed_json_pt_tray_stop_all_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let pt: HashMap<String, String> = serde_json::from_str(SEED_JSON_PT).expect("pt json");
        assert_ne!(
            en.get("tray.stop_all"),
            pt.get("tray.stop_all"),
            "Portuguese seed should translate tray.stop_all (same key, different value)"
        );
    }

    #[test]
    fn seed_json_nl_tray_stop_all_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let nl: HashMap<String, String> = serde_json::from_str(SEED_JSON_NL).expect("nl json");
        assert_ne!(
            en.get("tray.stop_all"),
            nl.get("tray.stop_all"),
            "Dutch seed should translate tray.stop_all (same key, different value)"
        );
    }

    #[test]
    fn seed_json_it_tray_stop_all_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let it: HashMap<String, String> = serde_json::from_str(SEED_JSON_IT).expect("it json");
        assert_ne!(
            en.get("tray.stop_all"),
            it.get("tray.stop_all"),
            "Italian seed should translate tray.stop_all (same key, different value)"
        );
    }

    #[test]
    fn seed_json_el_tray_stop_all_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let el: HashMap<String, String> = serde_json::from_str(SEED_JSON_EL).expect("el json");
        assert_ne!(
            en.get("tray.stop_all"),
            el.get("tray.stop_all"),
            "Greek seed should translate tray.stop_all (same key, different value)"
        );
    }

    #[test]
    fn seed_json_pl_tray_stop_all_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let pl: HashMap<String, String> = serde_json::from_str(SEED_JSON_PL).expect("pl json");
        assert_ne!(
            en.get("tray.stop_all"),
            pl.get("tray.stop_all"),
            "Polish seed should translate tray.stop_all (same key, different value)"
        );
    }

    #[test]
    fn seed_json_ru_tray_stop_all_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let ru: HashMap<String, String> = serde_json::from_str(SEED_JSON_RU).expect("ru json");
        assert_ne!(
            en.get("tray.stop_all"),
            ru.get("tray.stop_all"),
            "Russian seed should translate tray.stop_all (same key, different value)"
        );
    }

    #[test]
    fn seed_json_zh_tray_stop_all_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let zh: HashMap<String, String> = serde_json::from_str(SEED_JSON_ZH).expect("zh json");
        assert_ne!(
            en.get("tray.stop_all"),
            zh.get("tray.stop_all"),
            "Chinese seed should translate tray.stop_all (same key, different value)"
        );
    }

    #[test]
    fn seed_json_ja_tray_stop_all_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let ja: HashMap<String, String> = serde_json::from_str(SEED_JSON_JA).expect("ja json");
        assert_ne!(
            en.get("tray.stop_all"),
            ja.get("tray.stop_all"),
            "Japanese seed should translate tray.stop_all (same key, different value)"
        );
    }

    #[test]
    fn seed_json_de_tray_show_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let de: HashMap<String, String> = serde_json::from_str(SEED_JSON_DE).expect("de json");
        assert_ne!(
            en.get("tray.show"),
            de.get("tray.show"),
            "German seed should translate tray.show (same key, different value)"
        );
    }

    #[test]
    fn seed_json_es_tray_show_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let es: HashMap<String, String> = serde_json::from_str(SEED_JSON_ES).expect("es json");
        assert_ne!(
            en.get("tray.show"),
            es.get("tray.show"),
            "Spanish seed should translate tray.show (same key, different value)"
        );
    }

    #[test]
    fn seed_json_sv_tray_show_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let sv: HashMap<String, String> = serde_json::from_str(SEED_JSON_SV).expect("sv json");
        assert_ne!(
            en.get("tray.show"),
            sv.get("tray.show"),
            "Swedish seed should translate tray.show (same key, different value)"
        );
    }

    #[test]
    fn seed_json_fr_tray_show_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let fr: HashMap<String, String> = serde_json::from_str(SEED_JSON_FR).expect("fr json");
        assert_ne!(
            en.get("tray.show"),
            fr.get("tray.show"),
            "French seed should translate tray.show (same key, different value)"
        );
    }

    #[test]
    fn seed_json_pt_tray_show_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let pt: HashMap<String, String> = serde_json::from_str(SEED_JSON_PT).expect("pt json");
        assert_ne!(
            en.get("tray.show"),
            pt.get("tray.show"),
            "Portuguese seed should translate tray.show (same key, different value)"
        );
    }

    #[test]
    fn seed_json_nl_tray_show_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let nl: HashMap<String, String> = serde_json::from_str(SEED_JSON_NL).expect("nl json");
        assert_ne!(
            en.get("tray.show"),
            nl.get("tray.show"),
            "Dutch seed should translate tray.show (same key, different value)"
        );
    }

    #[test]
    fn seed_json_it_tray_show_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let it: HashMap<String, String> = serde_json::from_str(SEED_JSON_IT).expect("it json");
        assert_ne!(
            en.get("tray.show"),
            it.get("tray.show"),
            "Italian seed should translate tray.show (same key, different value)"
        );
    }

    #[test]
    fn seed_json_el_tray_show_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let el: HashMap<String, String> = serde_json::from_str(SEED_JSON_EL).expect("el json");
        assert_ne!(
            en.get("tray.show"),
            el.get("tray.show"),
            "Greek seed should translate tray.show (same key, different value)"
        );
    }

    #[test]
    fn seed_json_pl_tray_show_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let pl: HashMap<String, String> = serde_json::from_str(SEED_JSON_PL).expect("pl json");
        assert_ne!(
            en.get("tray.show"),
            pl.get("tray.show"),
            "Polish seed should translate tray.show (same key, different value)"
        );
    }

    #[test]
    fn seed_json_ru_tray_show_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let ru: HashMap<String, String> = serde_json::from_str(SEED_JSON_RU).expect("ru json");
        assert_ne!(
            en.get("tray.show"),
            ru.get("tray.show"),
            "Russian seed should translate tray.show (same key, different value)"
        );
    }

    #[test]
    fn seed_json_zh_tray_show_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let zh: HashMap<String, String> = serde_json::from_str(SEED_JSON_ZH).expect("zh json");
        assert_ne!(
            en.get("tray.show"),
            zh.get("tray.show"),
            "Chinese seed should translate tray.show (same key, different value)"
        );
    }

    #[test]
    fn seed_json_ja_tray_show_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let ja: HashMap<String, String> = serde_json::from_str(SEED_JSON_JA).expect("ja json");
        assert_ne!(
            en.get("tray.show"),
            ja.get("tray.show"),
            "Japanese seed should translate tray.show (same key, different value)"
        );
    }

    #[test]
    fn seed_json_de_menu_scan_daw_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let de: HashMap<String, String> = serde_json::from_str(SEED_JSON_DE).expect("de json");
        assert_ne!(
            en.get("menu.scan_daw"),
            de.get("menu.scan_daw"),
            "German seed should translate menu.scan_daw (same key, different value)"
        );
    }

    #[test]
    fn seed_json_es_menu_scan_daw_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let es: HashMap<String, String> = serde_json::from_str(SEED_JSON_ES).expect("es json");
        assert_ne!(
            en.get("menu.scan_daw"),
            es.get("menu.scan_daw"),
            "Spanish seed should translate menu.scan_daw (same key, different value)"
        );
    }

    #[test]
    fn seed_json_sv_menu_scan_daw_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let sv: HashMap<String, String> = serde_json::from_str(SEED_JSON_SV).expect("sv json");
        assert_ne!(
            en.get("menu.scan_daw"),
            sv.get("menu.scan_daw"),
            "Swedish seed should translate menu.scan_daw (same key, different value)"
        );
    }

    #[test]
    fn seed_json_fr_menu_scan_daw_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let fr: HashMap<String, String> = serde_json::from_str(SEED_JSON_FR).expect("fr json");
        assert_ne!(
            en.get("menu.scan_daw"),
            fr.get("menu.scan_daw"),
            "French seed should translate menu.scan_daw (same key, different value)"
        );
    }

    #[test]
    fn seed_json_pt_menu_scan_daw_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let pt: HashMap<String, String> = serde_json::from_str(SEED_JSON_PT).expect("pt json");
        assert_ne!(
            en.get("menu.scan_daw"),
            pt.get("menu.scan_daw"),
            "Portuguese seed should translate menu.scan_daw (same key, different value)"
        );
    }

    #[test]
    fn seed_json_nl_menu_scan_daw_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let nl: HashMap<String, String> = serde_json::from_str(SEED_JSON_NL).expect("nl json");
        assert_ne!(
            en.get("menu.scan_daw"),
            nl.get("menu.scan_daw"),
            "Dutch seed should translate menu.scan_daw (same key, different value)"
        );
    }

    #[test]
    fn seed_json_it_menu_scan_daw_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let it: HashMap<String, String> = serde_json::from_str(SEED_JSON_IT).expect("it json");
        assert_ne!(
            en.get("menu.scan_daw"),
            it.get("menu.scan_daw"),
            "Italian seed should translate menu.scan_daw (same key, different value)"
        );
    }

    #[test]
    fn seed_json_el_menu_scan_daw_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let el: HashMap<String, String> = serde_json::from_str(SEED_JSON_EL).expect("el json");
        assert_ne!(
            en.get("menu.scan_daw"),
            el.get("menu.scan_daw"),
            "Greek seed should translate menu.scan_daw (same key, different value)"
        );
    }

    #[test]
    fn seed_json_pl_menu_scan_daw_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let pl: HashMap<String, String> = serde_json::from_str(SEED_JSON_PL).expect("pl json");
        assert_ne!(
            en.get("menu.scan_daw"),
            pl.get("menu.scan_daw"),
            "Polish seed should translate menu.scan_daw (same key, different value)"
        );
    }

    #[test]
    fn seed_json_ru_menu_scan_daw_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let ru: HashMap<String, String> = serde_json::from_str(SEED_JSON_RU).expect("ru json");
        assert_ne!(
            en.get("menu.scan_daw"),
            ru.get("menu.scan_daw"),
            "Russian seed should translate menu.scan_daw (same key, different value)"
        );
    }

    #[test]
    fn seed_json_zh_menu_scan_daw_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let zh: HashMap<String, String> = serde_json::from_str(SEED_JSON_ZH).expect("zh json");
        assert_ne!(
            en.get("menu.scan_daw"),
            zh.get("menu.scan_daw"),
            "Chinese seed should translate menu.scan_daw (same key, different value)"
        );
    }

    #[test]
    fn seed_json_ja_menu_scan_daw_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let ja: HashMap<String, String> = serde_json::from_str(SEED_JSON_JA).expect("ja json");
        assert_ne!(
            en.get("menu.scan_daw"),
            ja.get("menu.scan_daw"),
            "Japanese seed should translate menu.scan_daw (same key, different value)"
        );
    }

    /// Mirrors `test/i18n-anchor-keys.test.js`: for every `menu.` / `tray.` / `confirm.` /
    /// `toast.` / `help.` / `ui.` key where **all** twelve non-English seeds differ from English, assert
    /// each locale row is not a verbatim copy of `en`.
    #[test]
    fn seed_json_safe_catalog_keys_all_locales_differ_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let locales: [(&str, &str); 12] = [
            ("de", SEED_JSON_DE),
            ("es", SEED_JSON_ES),
            ("sv", SEED_JSON_SV),
            ("fr", SEED_JSON_FR),
            ("nl", SEED_JSON_NL),
            ("pt", SEED_JSON_PT),
            ("it", SEED_JSON_IT),
            ("el", SEED_JSON_EL),
            ("pl", SEED_JSON_PL),
            ("ru", SEED_JSON_RU),
            ("zh", SEED_JSON_ZH),
            ("ja", SEED_JSON_JA),
        ];
        let maps: Vec<(&str, HashMap<String, String>)> = locales
            .iter()
            .map(|(loc, json)| (*loc, serde_json::from_str(json).expect(loc)))
            .collect();

        for (k, en_val) in &en {
            if !key_matches_catalog_prefix(k) {
                continue;
            }
            if en_val.trim().is_empty() {
                continue;
            }
            let all_differ = maps.iter().all(|(_, m)| match m.get(k) {
                Some(v) => v != en_val,
                None => false,
            });
            if !all_differ {
                continue;
            }
            for (loc, m) in &maps {
                assert_ne!(
                    m.get(k),
                    Some(en_val),
                    "locale {loc} key {k} must not copy English verbatim"
                );
            }
        }
    }

    #[test]
    fn seed_json_all_locales_share_exact_key_set() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let keys_en: HashSet<_> = en.keys().cloned().collect();
        for (loc, json) in [
            ("de", SEED_JSON_DE),
            ("es", SEED_JSON_ES),
            ("sv", SEED_JSON_SV),
            ("fr", SEED_JSON_FR),
            ("nl", SEED_JSON_NL),
            ("pt", SEED_JSON_PT),
            ("it", SEED_JSON_IT),
            ("el", SEED_JSON_EL),
            ("pl", SEED_JSON_PL),
            ("ru", SEED_JSON_RU),
            ("zh", SEED_JSON_ZH),
            ("ja", SEED_JSON_JA),
        ] {
            let m: HashMap<String, String> = serde_json::from_str(json).expect(loc);
            let keys: HashSet<_> = m.keys().cloned().collect();
            assert_eq!(
                keys_en, keys,
                "locale {loc} must define the same keys as en (missing or extra keys)"
            );
        }
    }

    #[test]
    fn seed_json_no_empty_values_any_locale() {
        for (loc, json) in [
            ("en", SEED_JSON_EN),
            ("de", SEED_JSON_DE),
            ("es", SEED_JSON_ES),
            ("sv", SEED_JSON_SV),
            ("fr", SEED_JSON_FR),
            ("nl", SEED_JSON_NL),
            ("pt", SEED_JSON_PT),
            ("it", SEED_JSON_IT),
            ("el", SEED_JSON_EL),
            ("pl", SEED_JSON_PL),
            ("ru", SEED_JSON_RU),
            ("zh", SEED_JSON_ZH),
            ("ja", SEED_JSON_JA),
        ] {
            let m: HashMap<String, String> = serde_json::from_str(json).expect(loc);
            for (k, v) in &m {
                assert!(
                    !v.trim().is_empty(),
                    "empty or whitespace-only value for key {k:?} in locale {loc}"
                );
            }
        }
    }

    /// `appFmt` / `toastFmt` replace `{token}` using the **English** token names passed from JS
    /// (`ipc.js`). `de`, `el`, `fr`, `it`, `nl`, `pl`, `pt`, `ru`, `sv`, `zh`, and `ja` seeds keep the same `{name}`, `{n}`, … substrings as English.
    /// Spanish (`es`) still has many legacy localized placeholder spellings in `toast.*` — covered
    /// separately via `seed_json_es_critical_prefixes_match_en_placeholders`.
    fn assert_seed_placeholders_match_en(en: &HashMap<String, String>, loc: &str, json: &str) {
        let re = Regex::new(r"\{[a-zA-Z_][a-zA-Z0-9_]*\}").expect("placeholder regex");
        let m: HashMap<String, String> = serde_json::from_str(json).expect(loc);
        for (k, en_val) in en {
            let placeholders: HashSet<String> = re
                .find_iter(en_val)
                .map(|x| x.as_str().to_string())
                .collect();
            if placeholders.is_empty() {
                continue;
            }
            let v = m.get(k).unwrap_or_else(|| panic!("key {k} missing in {loc}"));
            for p in &placeholders {
                assert!(
                    v.contains(p.as_str()),
                    "key {k} locale {loc}: value must contain placeholder {p} (English: {en_val:?})"
                );
            }
        }
    }

    #[test]
    fn seed_json_appfmt_placeholders_preserved_de_el_fr_it_nl_pl_pt_ru_sv_zh_ja() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        for (loc, json) in [
            ("de", SEED_JSON_DE),
            ("el", SEED_JSON_EL),
            ("fr", SEED_JSON_FR),
            ("it", SEED_JSON_IT),
            ("nl", SEED_JSON_NL),
            ("pl", SEED_JSON_PL),
            ("pt", SEED_JSON_PT),
            ("ru", SEED_JSON_RU),
            ("sv", SEED_JSON_SV),
            ("zh", SEED_JSON_ZH),
            ("ja", SEED_JSON_JA),
        ] {
            assert_seed_placeholders_match_en(&en, loc, json);
        }
    }

    #[test]
    fn seed_json_es_critical_prefixes_match_en_placeholders() {
        let re = Regex::new(r"\{[a-zA-Z_][a-zA-Z0-9_]*\}").expect("placeholder regex");
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let es: HashMap<String, String> = serde_json::from_str(SEED_JSON_ES).expect("es json");
        for (k, en_val) in &en {
            let is_critical = k.starts_with("menu.")
                || k.starts_with("ui.palette.")
                || k.starts_with("ui.sp_")
                || k.starts_with("confirm.");
            if !is_critical {
                continue;
            }
            let placeholders: HashSet<String> = re
                .find_iter(en_val)
                .map(|x| x.as_str().to_string())
                .collect();
            if placeholders.is_empty() {
                continue;
            }
            let v = es.get(k).expect("es must define same keys as en");
            for p in &placeholders {
                assert!(
                    v.contains(p.as_str()),
                    "key {k} locale es: value must contain placeholder {p} (English: {en_val:?})"
                );
            }
        }
    }

    #[test]
    fn seed_json_en_defines_all_native_menu_bar_keys() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        for key in NATIVE_MENU_BAR_KEYS {
            assert!(
                en.get(*key).map(|s| !s.trim().is_empty()).unwrap_or(false),
                "English seed missing native menu bar key {key} (sync with native_menu.rs)"
            );
        }
    }

    #[test]
    fn seed_json_en_defines_all_tray_keys() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        for key in TRAY_KEYS {
            assert!(
                en.get(*key).map(|s| !s.trim().is_empty()).unwrap_or(false),
                "English seed missing tray key {key} (sync with lib.rs tray menu)"
            );
        }
    }
}
