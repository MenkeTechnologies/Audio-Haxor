//! App UI strings for i18n: seeded into SQLite (`app_i18n` table) from `i18n/app_i18n_en.json`
//! (toasts, menus, tray, HTML `data-i18n*`, dialogs). Locales `cs`, `da`, `de`, `es`, `es-419`, `sv`, `fr`, `nl`, `pt`, `pt-BR`, `it`, `el`, `pl`, `ru`, `zh`, `ja`, `ko`, `fi`, `nb`, `tr`, `hu`, `ro`, `uk`, `vi`, `id`, `hi` add rows with the same keys.

use rusqlite::{params, Connection};
use std::collections::HashMap;

static SEED_JSON_EN: &str = include_str!("../../i18n/app_i18n_en.json");
static SEED_JSON_DE: &str = include_str!("../../i18n/app_i18n_de.json");
static SEED_JSON_ES: &str = include_str!("../../i18n/app_i18n_es.json");
static SEED_JSON_ES_419: &str = include_str!("../../i18n/app_i18n_es_419.json");
static SEED_JSON_SV: &str = include_str!("../../i18n/app_i18n_sv.json");
static SEED_JSON_FR: &str = include_str!("../../i18n/app_i18n_fr.json");
static SEED_JSON_PT: &str = include_str!("../../i18n/app_i18n_pt.json");
static SEED_JSON_PT_BR: &str = include_str!("../../i18n/app_i18n_pt_br.json");
static SEED_JSON_NL: &str = include_str!("../../i18n/app_i18n_nl.json");
static SEED_JSON_IT: &str = include_str!("../../i18n/app_i18n_it.json");
static SEED_JSON_EL: &str = include_str!("../../i18n/app_i18n_el.json");
static SEED_JSON_PL: &str = include_str!("../../i18n/app_i18n_pl.json");
static SEED_JSON_RU: &str = include_str!("../../i18n/app_i18n_ru.json");
static SEED_JSON_ZH: &str = include_str!("../../i18n/app_i18n_zh.json");
static SEED_JSON_JA: &str = include_str!("../../i18n/app_i18n_ja.json");
static SEED_JSON_KO: &str = include_str!("../../i18n/app_i18n_ko.json");
static SEED_JSON_FI: &str = include_str!("../../i18n/app_i18n_fi.json");
static SEED_JSON_DA: &str = include_str!("../../i18n/app_i18n_da.json");
static SEED_JSON_NB: &str = include_str!("../../i18n/app_i18n_nb.json");
static SEED_JSON_TR: &str = include_str!("../../i18n/app_i18n_tr.json");
static SEED_JSON_CS: &str = include_str!("../../i18n/app_i18n_cs.json");
static SEED_JSON_HU: &str = include_str!("../../i18n/app_i18n_hu.json");
static SEED_JSON_RO: &str = include_str!("../../i18n/app_i18n_ro.json");
static SEED_JSON_UK: &str = include_str!("../../i18n/app_i18n_uk.json");
static SEED_JSON_VI: &str = include_str!("../../i18n/app_i18n_vi.json");
static SEED_JSON_ID: &str = include_str!("../../i18n/app_i18n_id.json");
static SEED_JSON_HI: &str = include_str!("../../i18n/app_i18n_hi.json");

/// Insert default locale rows (`INSERT OR REPLACE` on `(key, locale)` primary key) on every
/// migration so shipped `i18n/app_i18n_*.json` values stay current. There is no separate UI to
/// edit `app_i18n` rows; the catalog is the source of truth.
pub fn seed_defaults(conn: &Connection) -> Result<(), String> {
    seed_locale(conn, "en", SEED_JSON_EN)?;
    seed_locale(conn, "de", SEED_JSON_DE)?;
    seed_locale(conn, "es", SEED_JSON_ES)?;
    seed_locale(conn, "es-419", SEED_JSON_ES_419)?;
    seed_locale(conn, "sv", SEED_JSON_SV)?;
    seed_locale(conn, "fr", SEED_JSON_FR)?;
    seed_locale(conn, "pt", SEED_JSON_PT)?;
    seed_locale(conn, "pt-BR", SEED_JSON_PT_BR)?;
    seed_locale(conn, "nl", SEED_JSON_NL)?;
    seed_locale(conn, "it", SEED_JSON_IT)?;
    seed_locale(conn, "el", SEED_JSON_EL)?;
    seed_locale(conn, "pl", SEED_JSON_PL)?;
    seed_locale(conn, "ru", SEED_JSON_RU)?;
    seed_locale(conn, "zh", SEED_JSON_ZH)?;
    seed_locale(conn, "ja", SEED_JSON_JA)?;
    seed_locale(conn, "ko", SEED_JSON_KO)?;
    seed_locale(conn, "fi", SEED_JSON_FI)?;
    seed_locale(conn, "da", SEED_JSON_DA)?;
    seed_locale(conn, "nb", SEED_JSON_NB)?;
    seed_locale(conn, "tr", SEED_JSON_TR)?;
    seed_locale(conn, "cs", SEED_JSON_CS)?;
    seed_locale(conn, "hu", SEED_JSON_HU)?;
    seed_locale(conn, "ro", SEED_JSON_RO)?;
    seed_locale(conn, "uk", SEED_JSON_UK)?;
    seed_locale(conn, "vi", SEED_JSON_VI)?;
    seed_locale(conn, "id", SEED_JSON_ID)?;
    seed_locale(conn, "hi", SEED_JSON_HI)?;
    Ok(())
}

fn seed_locale(conn: &Connection, locale: &str, json: &str) -> Result<(), String> {
    let map: HashMap<String, String> = serde_json::from_str(json).map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare_cached("INSERT OR REPLACE INTO app_i18n (key, locale, value) VALUES (?1, ?2, ?3)")
        .map_err(|e| e.to_string())?;
    for (k, v) in map {
        stmt.execute(params![k, locale, v])
            .map_err(|e| e.to_string())?;
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
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
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
        load_merged, SEED_JSON_CS, SEED_JSON_DA, SEED_JSON_DE, SEED_JSON_EL, SEED_JSON_EN,
        SEED_JSON_ES, SEED_JSON_ES_419, SEED_JSON_FI, SEED_JSON_FR, SEED_JSON_HU, SEED_JSON_IT,
        SEED_JSON_JA, SEED_JSON_KO, SEED_JSON_NB, SEED_JSON_NL, SEED_JSON_PL, SEED_JSON_PT,
        SEED_JSON_PT_BR, SEED_JSON_RO, SEED_JSON_RU, SEED_JSON_SV, SEED_JSON_TR, SEED_JSON_UK,
        SEED_JSON_VI, SEED_JSON_ID, SEED_JSON_HI, SEED_JSON_ZH,
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
        "menu.tab_audio_engine",
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

    /// Keys passed to `t("tray.…", …)` for the system tray in `tray_menu.rs` (+ dynamic strings from `audio.js`).
    const TRAY_KEYS: &[&str] = &[
        "tray.show",
        "tray.scan_all",
        "tray.stop_all",
        "tray.previous_track",
        "tray.play_pause",
        "tray.next_track",
        "tray.quit",
        "tray.tooltip",
        "tray.status_playing",
        "tray.status_paused",
    ];

    fn key_matches_catalog_prefix(k: &str) -> bool {
        ["menu.", "tray.", "confirm.", "toast.", "help.", "ui."]
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
    fn seed_json_es_419_menu_scan_all_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let es_419: HashMap<String, String> =
            serde_json::from_str(SEED_JSON_ES_419).expect("es-419 json");
        assert_ne!(
            en.get("menu.scan_all"),
            es_419.get("menu.scan_all"),
            "Latin American Spanish seed should translate menu.scan_all (same key, different value)"
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
        let fr: HashMap<String, String> =
            serde_json::from_str(include_str!("../../i18n/app_i18n_fr.json")).expect("fr json");
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
    fn seed_json_pt_br_menu_scan_all_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let pt_br: HashMap<String, String> =
            serde_json::from_str(SEED_JSON_PT_BR).expect("pt-BR json");
        assert_ne!(
            en.get("menu.scan_all"),
            pt_br.get("menu.scan_all"),
            "Brazilian Portuguese seed should translate menu.scan_all (same key, different value)"
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
    fn seed_json_ko_menu_scan_all_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let ko: HashMap<String, String> = serde_json::from_str(SEED_JSON_KO).expect("ko json");
        assert_ne!(
            en.get("menu.scan_all"),
            ko.get("menu.scan_all"),
            "Korean seed should translate menu.scan_all (same key, different value)"
        );
    }

    #[test]
    fn seed_json_fi_menu_scan_all_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let fi: HashMap<String, String> = serde_json::from_str(SEED_JSON_FI).expect("fi json");
        assert_ne!(
            en.get("menu.scan_all"),
            fi.get("menu.scan_all"),
            "Finnish seed should translate menu.scan_all (same key, different value)"
        );
    }

    #[test]
    fn seed_json_da_menu_scan_all_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let da: HashMap<String, String> = serde_json::from_str(SEED_JSON_DA).expect("da json");
        assert_ne!(
            en.get("menu.scan_all"),
            da.get("menu.scan_all"),
            "Danish seed should translate menu.scan_all (same key, different value)"
        );
    }

    #[test]
    fn seed_json_nb_menu_scan_all_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let nb: HashMap<String, String> = serde_json::from_str(SEED_JSON_NB).expect("nb json");
        assert_ne!(
            en.get("menu.scan_all"),
            nb.get("menu.scan_all"),
            "Norwegian seed should translate menu.scan_all (same key, different value)"
        );
    }

    #[test]
    fn seed_json_tr_menu_scan_all_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let tr: HashMap<String, String> = serde_json::from_str(SEED_JSON_TR).expect("tr json");
        assert_ne!(
            en.get("menu.scan_all"),
            tr.get("menu.scan_all"),
            "Turkish seed should translate menu.scan_all (same key, different value)"
        );
    }

    #[test]
    fn seed_json_cs_menu_scan_all_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let cs: HashMap<String, String> = serde_json::from_str(SEED_JSON_CS).expect("cs json");
        assert_ne!(
            en.get("menu.scan_all"),
            cs.get("menu.scan_all"),
            "Czech seed should translate menu.scan_all (same key, different value)"
        );
    }

    #[test]
    fn seed_json_hu_menu_scan_all_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let hu: HashMap<String, String> = serde_json::from_str(SEED_JSON_HU).expect("hu json");
        assert_ne!(
            en.get("menu.scan_all"),
            hu.get("menu.scan_all"),
            "Hungarian seed should translate menu.scan_all (same key, different value)"
        );
    }

    #[test]
    fn seed_json_ro_menu_scan_all_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let ro: HashMap<String, String> = serde_json::from_str(SEED_JSON_RO).expect("ro json");
        assert_ne!(
            en.get("menu.scan_all"),
            ro.get("menu.scan_all"),
            "Romanian seed should translate menu.scan_all (same key, different value)"
        );
    }

    #[test]
    fn seed_json_uk_menu_scan_all_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let uk: HashMap<String, String> = serde_json::from_str(SEED_JSON_UK).expect("uk json");
        assert_ne!(
            en.get("menu.scan_all"),
            uk.get("menu.scan_all"),
            "Ukrainian seed should translate menu.scan_all (same key, different value)"
        );
    }

    #[test]
    fn seed_json_vi_menu_scan_all_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let vi: HashMap<String, String> = serde_json::from_str(SEED_JSON_VI).expect("vi json");
        assert_ne!(
            en.get("menu.scan_all"),
            vi.get("menu.scan_all"),
            "Vietnamese seed should translate menu.scan_all (same key, different value)"
        );
    }

    #[test]
    fn seed_json_id_menu_scan_all_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let id: HashMap<String, String> = serde_json::from_str(SEED_JSON_ID).expect("id json");
        assert_ne!(
            en.get("menu.scan_all"),
            id.get("menu.scan_all"),
            "Indonesian seed should translate menu.scan_all (same key, different value)"
        );
    }

    #[test]
    fn seed_json_hi_menu_scan_all_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let hi: HashMap<String, String> = serde_json::from_str(SEED_JSON_HI).expect("hi json");
        assert_ne!(
            en.get("menu.scan_all"),
            hi.get("menu.scan_all"),
            "Hindi seed should translate menu.scan_all (same key, different value)"
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
    fn seed_json_es_419_tray_play_pause_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let es_419: HashMap<String, String> =
            serde_json::from_str(SEED_JSON_ES_419).expect("es-419 json");
        assert_ne!(
            en.get("tray.play_pause"),
            es_419.get("tray.play_pause"),
            "Latin American Spanish seed should translate tray.play_pause (same key, different value)"
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
    fn seed_json_pt_br_tray_play_pause_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let pt_br: HashMap<String, String> =
            serde_json::from_str(SEED_JSON_PT_BR).expect("pt-BR json");
        assert_ne!(
            en.get("tray.play_pause"),
            pt_br.get("tray.play_pause"),
            "Brazilian Portuguese seed should translate tray.play_pause (same key, different value)"
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
    fn seed_json_ko_tray_play_pause_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let ko: HashMap<String, String> = serde_json::from_str(SEED_JSON_KO).expect("ko json");
        assert_ne!(
            en.get("tray.play_pause"),
            ko.get("tray.play_pause"),
            "Korean seed should translate tray.play_pause (same key, different value)"
        );
    }

    #[test]
    fn seed_json_fi_tray_play_pause_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let fi: HashMap<String, String> = serde_json::from_str(SEED_JSON_FI).expect("fi json");
        assert_ne!(
            en.get("tray.play_pause"),
            fi.get("tray.play_pause"),
            "Finnish seed should translate tray.play_pause (same key, different value)"
        );
    }

    #[test]
    fn seed_json_da_tray_play_pause_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let da: HashMap<String, String> = serde_json::from_str(SEED_JSON_DA).expect("da json");
        assert_ne!(
            en.get("tray.play_pause"),
            da.get("tray.play_pause"),
            "Danish seed should translate tray.play_pause (same key, different value)"
        );
    }

    #[test]
    fn seed_json_nb_tray_play_pause_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let nb: HashMap<String, String> = serde_json::from_str(SEED_JSON_NB).expect("nb json");
        assert_ne!(
            en.get("tray.play_pause"),
            nb.get("tray.play_pause"),
            "Norwegian seed should translate tray.play_pause (same key, different value)"
        );
    }

    #[test]
    fn seed_json_tr_tray_play_pause_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let tr: HashMap<String, String> = serde_json::from_str(SEED_JSON_TR).expect("tr json");
        assert_ne!(
            en.get("tray.play_pause"),
            tr.get("tray.play_pause"),
            "Turkish seed should translate tray.play_pause (same key, different value)"
        );
    }

    #[test]
    fn seed_json_cs_tray_play_pause_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let cs: HashMap<String, String> = serde_json::from_str(SEED_JSON_CS).expect("cs json");
        assert_ne!(
            en.get("tray.play_pause"),
            cs.get("tray.play_pause"),
            "Czech seed should translate tray.play_pause (same key, different value)"
        );
    }

    #[test]
    fn seed_json_hu_tray_play_pause_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let hu: HashMap<String, String> = serde_json::from_str(SEED_JSON_HU).expect("hu json");
        assert_ne!(
            en.get("tray.play_pause"),
            hu.get("tray.play_pause"),
            "Hungarian seed should translate tray.play_pause (same key, different value)"
        );
    }

    #[test]
    fn seed_json_ro_tray_play_pause_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let ro: HashMap<String, String> = serde_json::from_str(SEED_JSON_RO).expect("ro json");
        assert_ne!(
            en.get("tray.play_pause"),
            ro.get("tray.play_pause"),
            "Romanian seed should translate tray.play_pause (same key, different value)"
        );
    }

    #[test]
    fn seed_json_uk_tray_play_pause_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let uk: HashMap<String, String> = serde_json::from_str(SEED_JSON_UK).expect("uk json");
        assert_ne!(
            en.get("tray.play_pause"),
            uk.get("tray.play_pause"),
            "Ukrainian seed should translate tray.play_pause (same key, different value)"
        );
    }

    #[test]
    fn seed_json_vi_tray_play_pause_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let vi: HashMap<String, String> = serde_json::from_str(SEED_JSON_VI).expect("vi json");
        assert_ne!(
            en.get("tray.play_pause"),
            vi.get("tray.play_pause"),
            "Vietnamese seed should translate tray.play_pause (same key, different value)"
        );
    }

    #[test]
    fn seed_json_id_tray_play_pause_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let id: HashMap<String, String> = serde_json::from_str(SEED_JSON_ID).expect("id json");
        assert_ne!(
            en.get("tray.play_pause"),
            id.get("tray.play_pause"),
            "Indonesian seed should translate tray.play_pause (same key, different value)"
        );
    }

    #[test]
    fn seed_json_hi_tray_play_pause_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let hi: HashMap<String, String> = serde_json::from_str(SEED_JSON_HI).expect("hi json");
        assert_ne!(
            en.get("tray.play_pause"),
            hi.get("tray.play_pause"),
            "Hindi seed should translate tray.play_pause (same key, different value)"
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
    fn seed_json_es_419_tray_stop_all_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let es_419: HashMap<String, String> =
            serde_json::from_str(SEED_JSON_ES_419).expect("es-419 json");
        assert_ne!(
            en.get("tray.stop_all"),
            es_419.get("tray.stop_all"),
            "Latin American Spanish seed should translate tray.stop_all (same key, different value)"
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
    fn seed_json_pt_br_tray_stop_all_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let pt_br: HashMap<String, String> =
            serde_json::from_str(SEED_JSON_PT_BR).expect("pt-BR json");
        assert_ne!(
            en.get("tray.stop_all"),
            pt_br.get("tray.stop_all"),
            "Brazilian Portuguese seed should translate tray.stop_all (same key, different value)"
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
    fn seed_json_ko_tray_stop_all_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let ko: HashMap<String, String> = serde_json::from_str(SEED_JSON_KO).expect("ko json");
        assert_ne!(
            en.get("tray.stop_all"),
            ko.get("tray.stop_all"),
            "Korean seed should translate tray.stop_all (same key, different value)"
        );
    }

    #[test]
    fn seed_json_fi_tray_stop_all_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let fi: HashMap<String, String> = serde_json::from_str(SEED_JSON_FI).expect("fi json");
        assert_ne!(
            en.get("tray.stop_all"),
            fi.get("tray.stop_all"),
            "Finnish seed should translate tray.stop_all (same key, different value)"
        );
    }

    #[test]
    fn seed_json_da_tray_stop_all_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let da: HashMap<String, String> = serde_json::from_str(SEED_JSON_DA).expect("da json");
        assert_ne!(
            en.get("tray.stop_all"),
            da.get("tray.stop_all"),
            "Danish seed should translate tray.stop_all (same key, different value)"
        );
    }

    #[test]
    fn seed_json_nb_tray_stop_all_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let nb: HashMap<String, String> = serde_json::from_str(SEED_JSON_NB).expect("nb json");
        assert_ne!(
            en.get("tray.stop_all"),
            nb.get("tray.stop_all"),
            "Norwegian seed should translate tray.stop_all (same key, different value)"
        );
    }

    #[test]
    fn seed_json_tr_tray_stop_all_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let tr: HashMap<String, String> = serde_json::from_str(SEED_JSON_TR).expect("tr json");
        assert_ne!(
            en.get("tray.stop_all"),
            tr.get("tray.stop_all"),
            "Turkish seed should translate tray.stop_all (same key, different value)"
        );
    }

    #[test]
    fn seed_json_cs_tray_stop_all_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let cs: HashMap<String, String> = serde_json::from_str(SEED_JSON_CS).expect("cs json");
        assert_ne!(
            en.get("tray.stop_all"),
            cs.get("tray.stop_all"),
            "Czech seed should translate tray.stop_all (same key, different value)"
        );
    }

    #[test]
    fn seed_json_hu_tray_stop_all_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let hu: HashMap<String, String> = serde_json::from_str(SEED_JSON_HU).expect("hu json");
        assert_ne!(
            en.get("tray.stop_all"),
            hu.get("tray.stop_all"),
            "Hungarian seed should translate tray.stop_all (same key, different value)"
        );
    }

    #[test]
    fn seed_json_ro_tray_stop_all_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let ro: HashMap<String, String> = serde_json::from_str(SEED_JSON_RO).expect("ro json");
        assert_ne!(
            en.get("tray.stop_all"),
            ro.get("tray.stop_all"),
            "Romanian seed should translate tray.stop_all (same key, different value)"
        );
    }

    #[test]
    fn seed_json_uk_tray_stop_all_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let uk: HashMap<String, String> = serde_json::from_str(SEED_JSON_UK).expect("uk json");
        assert_ne!(
            en.get("tray.stop_all"),
            uk.get("tray.stop_all"),
            "Ukrainian seed should translate tray.stop_all (same key, different value)"
        );
    }

    #[test]
    fn seed_json_vi_tray_stop_all_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let vi: HashMap<String, String> = serde_json::from_str(SEED_JSON_VI).expect("vi json");
        assert_ne!(
            en.get("tray.stop_all"),
            vi.get("tray.stop_all"),
            "Vietnamese seed should translate tray.stop_all (same key, different value)"
        );
    }

    #[test]
    fn seed_json_id_tray_stop_all_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let id: HashMap<String, String> = serde_json::from_str(SEED_JSON_ID).expect("id json");
        assert_ne!(
            en.get("tray.stop_all"),
            id.get("tray.stop_all"),
            "Indonesian seed should translate tray.stop_all (same key, different value)"
        );
    }

    #[test]
    fn seed_json_hi_tray_stop_all_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let hi: HashMap<String, String> = serde_json::from_str(SEED_JSON_HI).expect("hi json");
        assert_ne!(
            en.get("tray.stop_all"),
            hi.get("tray.stop_all"),
            "Hindi seed should translate tray.stop_all (same key, different value)"
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
    fn seed_json_es_419_tray_show_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let es_419: HashMap<String, String> =
            serde_json::from_str(SEED_JSON_ES_419).expect("es-419 json");
        assert_ne!(
            en.get("tray.show"),
            es_419.get("tray.show"),
            "Latin American Spanish seed should translate tray.show (same key, different value)"
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
    fn seed_json_pt_br_tray_show_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let pt_br: HashMap<String, String> =
            serde_json::from_str(SEED_JSON_PT_BR).expect("pt-BR json");
        assert_ne!(
            en.get("tray.show"),
            pt_br.get("tray.show"),
            "Brazilian Portuguese seed should translate tray.show (same key, different value)"
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
    fn seed_json_ko_tray_show_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let ko: HashMap<String, String> = serde_json::from_str(SEED_JSON_KO).expect("ko json");
        assert_ne!(
            en.get("tray.show"),
            ko.get("tray.show"),
            "Korean seed should translate tray.show (same key, different value)"
        );
    }

    #[test]
    fn seed_json_fi_tray_show_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let fi: HashMap<String, String> = serde_json::from_str(SEED_JSON_FI).expect("fi json");
        assert_ne!(
            en.get("tray.show"),
            fi.get("tray.show"),
            "Finnish seed should translate tray.show (same key, different value)"
        );
    }

    #[test]
    fn seed_json_da_tray_show_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let da: HashMap<String, String> = serde_json::from_str(SEED_JSON_DA).expect("da json");
        assert_ne!(
            en.get("tray.show"),
            da.get("tray.show"),
            "Danish seed should translate tray.show (same key, different value)"
        );
    }

    #[test]
    fn seed_json_nb_tray_show_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let nb: HashMap<String, String> = serde_json::from_str(SEED_JSON_NB).expect("nb json");
        assert_ne!(
            en.get("tray.show"),
            nb.get("tray.show"),
            "Norwegian seed should translate tray.show (same key, different value)"
        );
    }

    #[test]
    fn seed_json_tr_tray_show_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let tr: HashMap<String, String> = serde_json::from_str(SEED_JSON_TR).expect("tr json");
        assert_ne!(
            en.get("tray.show"),
            tr.get("tray.show"),
            "Turkish seed should translate tray.show (same key, different value)"
        );
    }

    #[test]
    fn seed_json_cs_tray_show_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let cs: HashMap<String, String> = serde_json::from_str(SEED_JSON_CS).expect("cs json");
        assert_ne!(
            en.get("tray.show"),
            cs.get("tray.show"),
            "Czech seed should translate tray.show (same key, different value)"
        );
    }

    #[test]
    fn seed_json_hu_tray_show_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let hu: HashMap<String, String> = serde_json::from_str(SEED_JSON_HU).expect("hu json");
        assert_ne!(
            en.get("tray.show"),
            hu.get("tray.show"),
            "Hungarian seed should translate tray.show (same key, different value)"
        );
    }

    #[test]
    fn seed_json_ro_tray_show_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let ro: HashMap<String, String> = serde_json::from_str(SEED_JSON_RO).expect("ro json");
        assert_ne!(
            en.get("tray.show"),
            ro.get("tray.show"),
            "Romanian seed should translate tray.show (same key, different value)"
        );
    }

    #[test]
    fn seed_json_uk_tray_show_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let uk: HashMap<String, String> = serde_json::from_str(SEED_JSON_UK).expect("uk json");
        assert_ne!(
            en.get("tray.show"),
            uk.get("tray.show"),
            "Ukrainian seed should translate tray.show (same key, different value)"
        );
    }

    #[test]
    fn seed_json_vi_tray_show_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let vi: HashMap<String, String> = serde_json::from_str(SEED_JSON_VI).expect("vi json");
        assert_ne!(
            en.get("tray.show"),
            vi.get("tray.show"),
            "Vietnamese seed should translate tray.show (same key, different value)"
        );
    }

    #[test]
    fn seed_json_id_tray_show_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let id: HashMap<String, String> = serde_json::from_str(SEED_JSON_ID).expect("id json");
        assert_ne!(
            en.get("tray.show"),
            id.get("tray.show"),
            "Indonesian seed should translate tray.show (same key, different value)"
        );
    }

    #[test]
    fn seed_json_hi_tray_show_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let hi: HashMap<String, String> = serde_json::from_str(SEED_JSON_HI).expect("hi json");
        assert_ne!(
            en.get("tray.show"),
            hi.get("tray.show"),
            "Hindi seed should translate tray.show (same key, different value)"
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
    fn seed_json_es_419_menu_scan_daw_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let es_419: HashMap<String, String> =
            serde_json::from_str(SEED_JSON_ES_419).expect("es-419 json");
        assert_ne!(
            en.get("menu.scan_daw"),
            es_419.get("menu.scan_daw"),
            "Latin American Spanish seed should translate menu.scan_daw (same key, different value)"
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
    fn seed_json_pt_br_menu_scan_daw_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let pt_br: HashMap<String, String> =
            serde_json::from_str(SEED_JSON_PT_BR).expect("pt-BR json");
        assert_ne!(
            en.get("menu.scan_daw"),
            pt_br.get("menu.scan_daw"),
            "Brazilian Portuguese seed should translate menu.scan_daw (same key, different value)"
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

    #[test]
    fn seed_json_ko_menu_scan_daw_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let ko: HashMap<String, String> = serde_json::from_str(SEED_JSON_KO).expect("ko json");
        assert_ne!(
            en.get("menu.scan_daw"),
            ko.get("menu.scan_daw"),
            "Korean seed should translate menu.scan_daw (same key, different value)"
        );
    }

    #[test]
    fn seed_json_fi_menu_scan_daw_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let fi: HashMap<String, String> = serde_json::from_str(SEED_JSON_FI).expect("fi json");
        assert_ne!(
            en.get("menu.scan_daw"),
            fi.get("menu.scan_daw"),
            "Finnish seed should translate menu.scan_daw (same key, different value)"
        );
    }

    #[test]
    fn seed_json_da_menu_scan_daw_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let da: HashMap<String, String> = serde_json::from_str(SEED_JSON_DA).expect("da json");
        assert_ne!(
            en.get("menu.scan_daw"),
            da.get("menu.scan_daw"),
            "Danish seed should translate menu.scan_daw (same key, different value)"
        );
    }

    #[test]
    fn seed_json_nb_menu_scan_daw_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let nb: HashMap<String, String> = serde_json::from_str(SEED_JSON_NB).expect("nb json");
        assert_ne!(
            en.get("menu.scan_daw"),
            nb.get("menu.scan_daw"),
            "Norwegian seed should translate menu.scan_daw (same key, different value)"
        );
    }

    #[test]
    fn seed_json_tr_menu_scan_daw_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let tr: HashMap<String, String> = serde_json::from_str(SEED_JSON_TR).expect("tr json");
        assert_ne!(
            en.get("menu.scan_daw"),
            tr.get("menu.scan_daw"),
            "Turkish seed should translate menu.scan_daw (same key, different value)"
        );
    }

    #[test]
    fn seed_json_cs_menu_scan_daw_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let cs: HashMap<String, String> = serde_json::from_str(SEED_JSON_CS).expect("cs json");
        assert_ne!(
            en.get("menu.scan_daw"),
            cs.get("menu.scan_daw"),
            "Czech seed should translate menu.scan_daw (same key, different value)"
        );
    }

    #[test]
    fn seed_json_hu_menu_scan_daw_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let hu: HashMap<String, String> = serde_json::from_str(SEED_JSON_HU).expect("hu json");
        assert_ne!(
            en.get("menu.scan_daw"),
            hu.get("menu.scan_daw"),
            "Hungarian seed should translate menu.scan_daw (same key, different value)"
        );
    }

    #[test]
    fn seed_json_ro_menu_scan_daw_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let ro: HashMap<String, String> = serde_json::from_str(SEED_JSON_RO).expect("ro json");
        assert_ne!(
            en.get("menu.scan_daw"),
            ro.get("menu.scan_daw"),
            "Romanian seed should translate menu.scan_daw (same key, different value)"
        );
    }

    #[test]
    fn seed_json_uk_menu_scan_daw_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let uk: HashMap<String, String> = serde_json::from_str(SEED_JSON_UK).expect("uk json");
        assert_ne!(
            en.get("menu.scan_daw"),
            uk.get("menu.scan_daw"),
            "Ukrainian seed should translate menu.scan_daw (same key, different value)"
        );
    }

    #[test]
    fn seed_json_vi_menu_scan_daw_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let vi: HashMap<String, String> = serde_json::from_str(SEED_JSON_VI).expect("vi json");
        assert_ne!(
            en.get("menu.scan_daw"),
            vi.get("menu.scan_daw"),
            "Vietnamese seed should translate menu.scan_daw (same key, different value)"
        );
    }

    #[test]
    fn seed_json_id_menu_scan_daw_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let id: HashMap<String, String> = serde_json::from_str(SEED_JSON_ID).expect("id json");
        assert_ne!(
            en.get("menu.scan_daw"),
            id.get("menu.scan_daw"),
            "Indonesian seed should translate menu.scan_daw (same key, different value)"
        );
    }

    #[test]
    fn seed_json_hi_menu_scan_daw_differs_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let hi: HashMap<String, String> = serde_json::from_str(SEED_JSON_HI).expect("hi json");
        assert_ne!(
            en.get("menu.scan_daw"),
            hi.get("menu.scan_daw"),
            "Hindi seed should translate menu.scan_daw (same key, different value)"
        );
    }

    /// Mirrors `test/i18n-anchor-keys.test.js`: for every `menu.` / `tray.` / `confirm.` /
    /// `toast.` / `help.` / `ui.` key where **all** non-English seeds differ from English, assert
    /// each locale row is not a verbatim copy of `en`.
    #[test]
    fn seed_json_safe_catalog_keys_all_locales_differ_from_en() {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let locales: [(&str, &str); 26] = [
            ("de", SEED_JSON_DE),
            ("es", SEED_JSON_ES),
            ("es-419", SEED_JSON_ES_419),
            ("sv", SEED_JSON_SV),
            ("fr", SEED_JSON_FR),
            ("nl", SEED_JSON_NL),
            ("pt", SEED_JSON_PT),
            ("pt-BR", SEED_JSON_PT_BR),
            ("it", SEED_JSON_IT),
            ("el", SEED_JSON_EL),
            ("pl", SEED_JSON_PL),
            ("ru", SEED_JSON_RU),
            ("zh", SEED_JSON_ZH),
            ("ja", SEED_JSON_JA),
            ("ko", SEED_JSON_KO),
            ("fi", SEED_JSON_FI),
            ("da", SEED_JSON_DA),
            ("nb", SEED_JSON_NB),
            ("tr", SEED_JSON_TR),
            ("cs", SEED_JSON_CS),
            ("hu", SEED_JSON_HU),
            ("ro", SEED_JSON_RO),
            ("uk", SEED_JSON_UK),
            ("vi", SEED_JSON_VI),
            ("id", SEED_JSON_ID),
            ("hi", SEED_JSON_HI),
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
            ("es-419", SEED_JSON_ES_419),
            ("sv", SEED_JSON_SV),
            ("fr", SEED_JSON_FR),
            ("nl", SEED_JSON_NL),
            ("pt", SEED_JSON_PT),
            ("pt-BR", SEED_JSON_PT_BR),
            ("it", SEED_JSON_IT),
            ("el", SEED_JSON_EL),
            ("pl", SEED_JSON_PL),
            ("ru", SEED_JSON_RU),
            ("zh", SEED_JSON_ZH),
            ("ja", SEED_JSON_JA),
            ("ko", SEED_JSON_KO),
            ("fi", SEED_JSON_FI),
            ("da", SEED_JSON_DA),
            ("nb", SEED_JSON_NB),
            ("tr", SEED_JSON_TR),
            ("cs", SEED_JSON_CS),
            ("hu", SEED_JSON_HU),
            ("ro", SEED_JSON_RO),
            ("uk", SEED_JSON_UK),
            ("vi", SEED_JSON_VI),
            ("id", SEED_JSON_ID),
            ("hi", SEED_JSON_HI),
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
            ("es-419", SEED_JSON_ES_419),
            ("sv", SEED_JSON_SV),
            ("fr", SEED_JSON_FR),
            ("nl", SEED_JSON_NL),
            ("pt", SEED_JSON_PT),
            ("pt-BR", SEED_JSON_PT_BR),
            ("it", SEED_JSON_IT),
            ("el", SEED_JSON_EL),
            ("pl", SEED_JSON_PL),
            ("ru", SEED_JSON_RU),
            ("zh", SEED_JSON_ZH),
            ("ja", SEED_JSON_JA),
            ("ko", SEED_JSON_KO),
            ("fi", SEED_JSON_FI),
            ("da", SEED_JSON_DA),
            ("nb", SEED_JSON_NB),
            ("tr", SEED_JSON_TR),
            ("cs", SEED_JSON_CS),
            ("hu", SEED_JSON_HU),
            ("ro", SEED_JSON_RO),
            ("uk", SEED_JSON_UK),
            ("vi", SEED_JSON_VI),
            ("id", SEED_JSON_ID),
            ("hi", SEED_JSON_HI),
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
    /// (`ipc.js`). `de`, `el`, `fi`, `fr`, `it`, `nl`, `pl`, `pt`, `pt-BR`, `ru`, `sv`, `zh`, `ja`, `ko`, and `hi` seeds keep the same `{name}`, `{n}`, … substrings as English.
    /// Spanish (`es` / `es-419`) still has many legacy localized placeholder spellings in `toast.*`
    /// — covered separately via `seed_json_es_critical_prefixes_match_en_placeholders` and
    /// `seed_json_es_419_critical_prefixes_match_en_placeholders`.
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
            let v = m
                .get(k)
                .unwrap_or_else(|| panic!("key {k} missing in {loc}"));
            for p in &placeholders {
                assert!(
                    v.contains(p.as_str()),
                    "key {k} locale {loc}: value must contain placeholder {p} (English: {en_val:?})"
                );
            }
        }
    }

    #[test]
    fn seed_json_appfmt_placeholders_preserved_de_el_es_es_419_fi_fr_it_nl_pl_pt_pt_br_ru_sv_zh_ja_ko_da_nb_tr_cs_hu_ro_uk_vi_id_hi(
    ) {
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        for (loc, json) in [
            ("de", SEED_JSON_DE),
            ("el", SEED_JSON_EL),
            ("es", SEED_JSON_ES),
            ("es-419", SEED_JSON_ES_419),
            ("fr", SEED_JSON_FR),
            ("it", SEED_JSON_IT),
            ("nl", SEED_JSON_NL),
            ("pl", SEED_JSON_PL),
            ("pt", SEED_JSON_PT),
            ("pt-BR", SEED_JSON_PT_BR),
            ("ru", SEED_JSON_RU),
            ("sv", SEED_JSON_SV),
            ("zh", SEED_JSON_ZH),
            ("ja", SEED_JSON_JA),
            ("ko", SEED_JSON_KO),
            ("fi", SEED_JSON_FI),
            ("da", SEED_JSON_DA),
            ("nb", SEED_JSON_NB),
            ("tr", SEED_JSON_TR),
            ("cs", SEED_JSON_CS),
            ("hu", SEED_JSON_HU),
            ("ro", SEED_JSON_RO),
            ("uk", SEED_JSON_UK),
            ("vi", SEED_JSON_VI),
            ("id", SEED_JSON_ID),
            ("hi", SEED_JSON_HI),
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
    fn seed_json_es_419_critical_prefixes_match_en_placeholders() {
        let re = Regex::new(r"\{[a-zA-Z_][a-zA-Z0-9_]*\}").expect("placeholder regex");
        let en: HashMap<String, String> = serde_json::from_str(SEED_JSON_EN).expect("en json");
        let es_419: HashMap<String, String> =
            serde_json::from_str(SEED_JSON_ES_419).expect("es-419 json");
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
            let v = es_419.get(k).expect("es-419 must define same keys as en");
            for p in &placeholders {
                assert!(
                    v.contains(p.as_str()),
                    "key {k} locale es-419: value must contain placeholder {p} (English: {en_val:?})"
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
                "English seed missing tray key {key} (sync with tray_menu.rs)"
            );
        }
    }
}
