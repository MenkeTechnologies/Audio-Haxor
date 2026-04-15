//! SQLite database layer for scalable storage of audio samples, analysis caches,
//! and scan metadata. Replaces JSON file persistence for data that can grow to
//! millions of rows.

use crate::history::{
    self, AudioHistory, AudioSample, AudioScanSnapshot, DawHistory, DawProject, DawScanSnapshot,
    KvrCacheEntry, MidiFile, MidiScanSnapshot, PdfFile, PdfScanSnapshot, PresetFile, PresetHistory,
    PresetScanSnapshot, ScanHistory, ScanSnapshot, VideoFile,
};
use crate::path_norm::{normalize_path_for_db, path_strings_json_normalized};
use crate::scanner::PluginInfo;
use regex::{Regex, RegexBuilder};
use rusqlite::functions::{Context, FunctionFlags};
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

static GLOBAL_DB: OnceLock<Database> = OnceLock::new();
/// Serializes [`Database::open`] + migrations on the on-disk file. Without this, many threads can
/// pass the `GLOBAL_DB` empty check at once and run `migrate()` in parallel against the same path,
/// which triggers SQLite `database is locked` (seen on multi-core CI runners).
static INIT_GLOBAL_MUTEX: Mutex<()> = Mutex::new(());

/// Initialize the global database. Call once at startup.
///
/// Safe under parallel `cargo test`: [`OnceLock`] stores at most one handle; a mutex ensures only
/// one thread opens the DB file and runs migrations. Losers of the `set` race return `Ok` without
/// retaining a second connection.
pub fn init_global() -> Result<(), String> {
    if GLOBAL_DB.get().is_some() {
        return Ok(());
    }
    let _guard = INIT_GLOBAL_MUTEX
        .lock()
        .map_err(|e| format!("init_global mutex: {e}"))?;
    if GLOBAL_DB.get().is_some() {
        return Ok(());
    }
    let db = Database::open()?;
    match GLOBAL_DB.set(db) {
        Ok(()) => Ok(()),
        Err(_redundant) => Ok(()),
    }
}

/// Returns true after a successful [`init_global`] (including concurrent test runners).
pub fn global_initialized() -> bool {
    GLOBAL_DB.get().is_some()
}

/// Get the global database reference.
pub fn global() -> &'static Database {
    GLOBAL_DB.get().expect("Database not initialized")
}

/// One row for [`Database::batch_update_analysis`]: path, BPM, musical key, LUFS.
pub type AnalysisBatchRow = (String, Option<f64>, Option<String>, Option<f64>);

/// Row from `pdf_metadata` (pages plus PDF Info dictionary dates when extracted).
#[derive(Debug, Clone)]
pub struct PdfMetaDbRow {
    pub pages: Option<u32>,
    pub pdf_creation_date: Option<String>,
    pub pdf_mod_date: Option<String>,
}

/// SQLite with WAL: multiple connections can serve read-heavy queries concurrently.
/// `write` is the **only** handle used for schema migrations and code paths that need the writer
/// mutex; **never** mix it into the read round-robin — doing so serialized every `db_query_*` IPC
/// with every other read/write on that mutex (spinners across tabs when one query held the slot).
/// `read` is a pool of read-only file handles; [`Database::read_conn`] round-robins **only** here
/// (always at least one after [`Database::open`]).
pub struct Database {
    write: Mutex<Connection>,
    read: Vec<Mutex<Connection>>,
    /// Per-slot query-start timestamp (epoch ms) for the progress-handler timeout.
    read_deadlines: Vec<Arc<AtomicU64>>,
    read_idx: AtomicUsize,
    /// `SELECT COUNT(*) FROM midi_library` is O(rows); cache and invalidate on MIDI library writes.
    midi_library_total_cache: Mutex<Option<u64>>,
    /// Same pattern as [`Self::midi_library_total_rows`] for `video_library`.
    video_library_total_cache: Mutex<Option<u64>>,
    /// Same pattern as [`Self::midi_library_total_rows`] for `pdf_library`.
    pdf_library_total_cache: Mutex<Option<u64>>,
    /// `SELECT COUNT(*) FROM audio_library` — invalidated on audio library writes.
    audio_library_total_cache: Mutex<Option<u64>>,
    /// Preset tab header: non-MIDI rows in library (`query_presets` / `preset_filter_stats`).
    preset_inventory_total_cache: Mutex<Option<u64>>,
    /// `SELECT COUNT(*) FROM daw_library` — invalidated on DAW library writes.
    daw_library_total_cache: Mutex<Option<u64>>,
    /// `SELECT COUNT(*) FROM plugin_library` — invalidated on plugin library writes.
    plugin_library_total_cache: Mutex<Option<u64>>,
}

/// Parameters for paginated audio sample queries.
#[derive(Debug, Deserialize)]
pub struct AudioQueryParams {
    #[serde(default)]
    pub scan_id: Option<String>,
    #[serde(default)]
    pub search: Option<String>,
    /// When true, `search` is a Rust regex (case-insensitive, matches JS `RegExp` `i` flag).
    /// Uses SQLite `REGEXP` with a user-defined function — not FTS5 phrase search.
    #[serde(default)]
    pub search_regex: bool,
    #[serde(default)]
    pub format_filter: Option<String>,
    #[serde(default = "default_sort_key")]
    pub sort_key: String,
    #[serde(default = "default_true")]
    pub sort_asc: bool,
    #[serde(default)]
    pub offset: u64,
    #[serde(default = "default_limit")]
    pub limit: u64,
}

fn default_sort_key() -> String {
    "name".into()
}
fn default_true() -> bool {
    true
}
fn default_limit() -> u64 {
    200
}

/// Exact FTS match counts above this force a bounded count and skip heavy aggregates (GROUP BY /
/// size histogram over every hit) — common substrings like "loop" can match hundreds of
/// thousands of library rows. Shared by audio, presets, MIDI, and PDF inventory queries.
const FTS_INVENTORY_MATCH_COUNT_CAP: i64 = 100_000;

/// Convert a user search string into an FTS5 phrase query for the trigram
/// tokenizer. Returns `None` for empty/whitespace input. The result is wrapped
/// in double quotes (phrase match) with internal quotes doubled per FTS5 syntax.
/// Trigram tokenizer indexes substrings, so `"foo"` matches any row containing
/// "foo" as a substring in any indexed column.
/// Returns an FTS5 phrase for trigram MATCH, or None if the search is empty
/// or too short (trigram needs ≥3 chars). Callers must fall back to LIKE for
/// 1–2 char searches.
fn fts_phrase(search: &str) -> Option<String> {
    let trimmed = search.trim();
    if trimmed.len() < 3 {
        return None;
    }
    Some(format!("\"{}\"", trimmed.replace('"', "\"\"")))
}

/// Build a LIKE pattern for short searches (1–2 chars) where FTS5 trigram
/// can't help. Returns None for empty input.
fn short_like(search: &str) -> Option<String> {
    let trimmed = search.trim();
    if trimmed.is_empty() || trimmed.len() >= 3 {
        return None;
    }
    let escaped = trimmed
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_");
    Some(format!("%{escaped}%"))
}

/// FTS-backed tabs (name + path): returns `(fts_match, like_pat, regex_pat)` — at most one of the
/// three is `Some`. Mirrors [`AudioQueryParams::search_regex`] semantics.
fn classify_fts_name_path_search(
    search: Option<&str>,
    search_regex: bool,
) -> (Option<String>, Option<String>, Option<String>) {
    if search_regex {
        let mut like_pat = None;
        let mut regex_pat = None;
        if let Some(s) = search {
            let t = s.trim();
            if !t.is_empty() {
                if RegexBuilder::new(t).case_insensitive(true).build().is_ok() {
                    regex_pat = Some(t.to_string());
                } else {
                    let escaped = t
                        .replace('\\', "\\\\")
                        .replace('%', "\\%")
                        .replace('_', "\\_");
                    like_pat = Some(format!("%{escaped}%"));
                }
            }
        }
        (None, like_pat, regex_pat)
    } else {
        (
            search.and_then(fts_phrase),
            search.and_then(short_like),
            None,
        )
    }
}

/// Plugins tab (name, manufacturer, path): `(regex_pat, like_pat)` — when `regex_pat` is `Some`,
/// use `REGEXP` on all three columns; otherwise `like_pat` is fuzzy interleaved or invalid-regex
/// fallback (same binding shape).
fn classify_plugins_search(
    search: Option<&str>,
    search_regex: bool,
) -> (Option<String>, Option<String>) {
    let Some(s) = search else {
        return (None, None);
    };
    let t = s.trim();
    if t.is_empty() {
        return (None, None);
    }
    if search_regex {
        if RegexBuilder::new(t).case_insensitive(true).build().is_ok() {
            (Some(t.to_string()), None)
        } else {
            let escaped = t
                .replace('\\', "\\\\")
                .replace('%', "\\%")
                .replace('_', "\\_");
            (None, Some(format!("%{escaped}%")))
        }
    } else {
        let interleaved = format!(
            "%{}%",
            t.chars()
                .map(|c| c.to_string())
                .collect::<Vec<_>>()
                .join("%")
        );
        (None, Some(interleaved))
    }
}

static REGEXP_FUNC_CACHE: OnceLock<Mutex<HashMap<String, Regex>>> = OnceLock::new();

fn regexp_pattern_cache() -> &'static Mutex<HashMap<String, Regex>> {
    REGEXP_FUNC_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// SQLite `regexp(pattern, haystack)` — matches JS `new RegExp(pattern, 'i')` (case-insensitive).
fn regexp_user_matches(pattern: &str, haystack: &str) -> bool {
    let mut map = match regexp_pattern_cache().lock() {
        Ok(m) => m,
        Err(_) => return false,
    };
    if map.len() > 256 {
        map.clear();
    }
    let re = match map.entry(pattern.to_string()) {
        Entry::Occupied(e) => e.get().clone(),
        Entry::Vacant(v) => {
            let r = match RegexBuilder::new(pattern).case_insensitive(true).build() {
                Ok(r) => r,
                Err(_) => return false,
            };
            v.insert(r.clone());
            r
        }
    };
    re.is_match(haystack)
}

fn install_regexp_function(conn: &Connection) -> Result<(), String> {
    conn.create_scalar_function(
        "regexp",
        2,
        FunctionFlags::SQLITE_UTF8 | FunctionFlags::SQLITE_DETERMINISTIC,
        |ctx: &Context<'_>| -> std::result::Result<i64, rusqlite::Error> {
            let pattern: String = ctx.get(0)?;
            let haystack: String = ctx.get(1)?;
            Ok(if regexp_user_matches(&pattern, &haystack) {
                1
            } else {
                0
            })
        },
    )
    .map_err(|e| e.to_string())
}

/// Backfill FTS5 contentless shadow tables from primary tables for rows missing from FTS.
/// Migration v9 created empty FTS tables; existing `audio_samples` (etc.) rows were never
/// indexed, so `MATCH` returned no hits while the base tables still showed full library counts.
fn backfill_contentless_fts(conn: &rusqlite::Connection) -> Result<(), String> {
    let n_audio: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM audio_samples a WHERE NOT EXISTS (SELECT 1 FROM audio_samples_fts f WHERE f.rowid = a.id)",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);
    if n_audio > 0 {
        crate::append_log(format!(
            "DB migration v13: backfilling {n_audio} audio_samples rows into FTS"
        ));
        conn.execute(
            "INSERT INTO audio_samples_fts(rowid, name, path, scan_id)
             SELECT a.id, a.name, a.path, a.scan_id FROM audio_samples a
             WHERE NOT EXISTS (SELECT 1 FROM audio_samples_fts f WHERE f.rowid = a.id)",
            [],
        )
        .map_err(|e| e.to_string())?;
    }

    let n_daw: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM daw_projects p WHERE NOT EXISTS (SELECT 1 FROM daw_projects_fts f WHERE f.rowid = p.id)",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);
    if n_daw > 0 {
        crate::append_log(format!(
            "DB migration v13: backfilling {n_daw} daw_projects rows into FTS"
        ));
        conn.execute(
            "INSERT INTO daw_projects_fts(rowid, name, path, daw, scan_id)
             SELECT p.id, p.name, p.path, p.daw, p.scan_id FROM daw_projects p
             WHERE NOT EXISTS (SELECT 1 FROM daw_projects_fts f WHERE f.rowid = p.id)",
            [],
        )
        .map_err(|e| e.to_string())?;
    }

    let n_preset: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM presets p WHERE NOT EXISTS (SELECT 1 FROM presets_fts f WHERE f.rowid = p.id)",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);
    if n_preset > 0 {
        crate::append_log(format!(
            "DB migration v13: backfilling {n_preset} presets rows into FTS"
        ));
        conn.execute(
            "INSERT INTO presets_fts(rowid, name, path, format, scan_id)
             SELECT p.id, p.name, p.path, p.format, p.scan_id FROM presets p
             WHERE NOT EXISTS (SELECT 1 FROM presets_fts f WHERE f.rowid = p.id)",
            [],
        )
        .map_err(|e| e.to_string())?;
    }

    let n_midi: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM midi_files m WHERE NOT EXISTS (SELECT 1 FROM midi_files_fts f WHERE f.rowid = m.id)",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);
    if n_midi > 0 {
        crate::append_log(format!(
            "DB migration v13: backfilling {n_midi} midi_files rows into FTS"
        ));
        conn.execute(
            "INSERT INTO midi_files_fts(rowid, name, path, scan_id)
             SELECT m.id, m.name, m.path, m.scan_id FROM midi_files m
             WHERE NOT EXISTS (SELECT 1 FROM midi_files_fts f WHERE f.rowid = m.id)",
            [],
        )
        .map_err(|e| e.to_string())?;
    }

    let n_pdf: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM pdfs p WHERE NOT EXISTS (SELECT 1 FROM pdfs_fts f WHERE f.rowid = p.id)",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);
    if n_pdf > 0 {
        crate::append_log(format!(
            "DB migration v13: backfilling {n_pdf} pdfs rows into FTS"
        ));
        conn.execute(
            "INSERT INTO pdfs_fts(rowid, name, path, scan_id)
             SELECT p.id, p.name, p.path, p.scan_id FROM pdfs p
             WHERE NOT EXISTS (SELECT 1 FROM pdfs_fts f WHERE f.rowid = p.id)",
            [],
        )
        .map_err(|e| e.to_string())?;
    }

    Ok(())
}

/// A single row returned from a paginated query, with analysis data inline.
#[derive(Debug, Serialize)]
pub struct AudioSampleRow {
    pub name: String,
    pub path: String,
    pub directory: String,
    pub format: String,
    pub size: u64,
    #[serde(rename = "sizeFormatted")]
    pub size_formatted: String,
    pub modified: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channels: Option<u16>,
    #[serde(rename = "sampleRate", skip_serializing_if = "Option::is_none")]
    pub sample_rate: Option<u32>,
    #[serde(rename = "bitsPerSample", skip_serializing_if = "Option::is_none")]
    pub bits_per_sample: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bpm: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lufs: Option<f64>,
    /// True when BPM detection gave up (NULL BPM with key+LUFS filled) — UI shows N/A in the BPM column.
    #[serde(rename = "bpmExhausted", skip_serializing_if = "std::ops::Not::not")]
    pub bpm_exhausted: bool,
}

/// Result of a paginated query.
#[derive(Debug, Serialize)]
pub struct AudioQueryResult {
    pub samples: Vec<AudioSampleRow>,
    #[serde(rename = "totalCount")]
    pub total_count: u64,
    /// True when `total_count` is a floor (`FTS_INVENTORY_MATCH_COUNT_CAP`) — exact count not computed.
    #[serde(
        rename = "totalCountCapped",
        skip_serializing_if = "std::ops::Not::not"
    )]
    pub total_count_capped: bool,
    #[serde(rename = "totalUnfiltered")]
    pub total_unfiltered: u64,
}

/// Aggregate stats for a scan.
#[derive(Debug, Serialize)]
pub struct AudioStatsResult {
    #[serde(rename = "sampleCount")]
    pub sample_count: u64,
    #[serde(rename = "totalBytes")]
    pub total_bytes: u64,
    #[serde(rename = "formatCounts")]
    pub format_counts: HashMap<String, u64>,
    #[serde(rename = "analyzedCount")]
    pub analyzed_count: u64,
}

/// Aggregate DAW stats from [`Database::daw_stats`]: library totals (deduped by `path`) when
/// `scan_id` is omitted or empty; otherwise that scan only.
#[derive(Debug, Serialize)]
pub struct DawStatsResult {
    #[serde(rename = "projectCount")]
    pub project_count: u64,
    #[serde(rename = "totalBytes")]
    pub total_bytes: u64,
    #[serde(rename = "dawCounts")]
    pub daw_counts: HashMap<String, u64>,
}

/// Aggregate preset stats from [`Database::preset_stats`]: library totals (deduped by `path`, MIDI
/// formats excluded) when `scan_id` is omitted or empty; otherwise that scan only.
#[derive(Debug, Serialize)]
pub struct PresetStatsResult {
    #[serde(rename = "presetCount")]
    pub preset_count: u64,
    #[serde(rename = "totalBytes")]
    pub total_bytes: u64,
    #[serde(rename = "formatCounts")]
    pub format_counts: HashMap<String, u64>,
}

/// Scan metadata (no samples).
#[derive(Debug, Serialize)]
pub struct ScanInfo {
    pub id: String,
    pub timestamp: String,
    #[serde(rename = "sampleCount")]
    pub sample_count: u64,
    #[serde(rename = "totalBytes")]
    pub total_bytes: u64,
    #[serde(rename = "formatCounts")]
    pub format_counts: HashMap<String, u64>,
    pub roots: Vec<String>,
}

/// Stats for a single cache table.
#[derive(Debug, Serialize)]
pub struct CacheStat {
    pub key: String,
    pub label: String,
    pub count: u64,
    pub total: u64,
    #[serde(rename = "sizeBytes")]
    pub size_bytes: u64,
}

/// Approximate on-disk bytes for btree objects (table + indexes) for `scan_table` + `item_table`.
/// Uses SQLite [`dbstat`](https://www.sqlite.org/dbstat.html) when available.
/// Returns `None` if `dbstat` is not compiled in (caller splits DB file size by row count).
fn dbstat_bytes_for_scan_group(
    conn: &Connection,
    scan_table: &str,
    item_table: &str,
) -> Option<u64> {
    let mut stmt = conn
        .prepare(
            "SELECT name FROM sqlite_master WHERE type IN ('table','index') AND tbl_name IN (?1, ?2)",
        )
        .ok()?;
    let mut rows = stmt.query(rusqlite::params![scan_table, item_table]).ok()?;
    let mut names = Vec::new();
    loop {
        match rows.next() {
            Ok(Some(row)) => {
                names.push(row.get::<_, String>(0).ok()?);
            }
            Ok(None) => break,
            Err(_) => return None,
        }
    }
    if names.is_empty() {
        return Some(0);
    }
    let mut total: u64 = 0;
    for name in names {
        let v: i64 = conn
            .query_row(
                "SELECT COALESCE(SUM(pgsize), 0) FROM dbstat WHERE name = ?1",
                [&name],
                |r| r.get(0),
            )
            .ok()?;
        total = total.saturating_add(v.max(0) as u64);
    }
    Some(total)
}

/// Canonical inventory: one row per `path` (highest `id` wins — newest insert for that path).
/// Materialized in `audio_library` (migration v14) so library queries avoid `GROUP BY path` on hot paths.
const AUDIO_LIBRARY_IDS: &str = "id IN (SELECT sample_id FROM audio_library)";
/// Same semantics as `MAX(id) GROUP BY path`, materialized in `daw_library` (migration v16).
const DAW_LIBRARY_IDS: &str = "id IN (SELECT project_id FROM daw_library)";
/// Migration v15 — same semantics as `MAX(id) GROUP BY path`, maintained on insert and deletes.
const PRESET_LIBRARY_IDS: &str = "id IN (SELECT preset_id FROM preset_library)";
const PDF_LIBRARY_IDS: &str = "id IN (SELECT pdf_id FROM pdf_library)";
const MIDI_LIBRARY_IDS: &str = "id IN (SELECT midi_id FROM midi_library)";

const VIDEO_LIBRARY_IDS: &str = "id IN (SELECT video_id FROM video_library)";
/// Materialized in `plugin_library` (migration v17) — same semantics as other `*_library` tables.
const PLUGIN_LIBRARY_IDS: &str = "id IN (SELECT plugin_id FROM plugin_library)";
const PLUGIN_LIBRARY_IDS_QUALIFIED: &str = "plugins.id IN (SELECT plugin_id FROM plugin_library)";

/// Comma-separated `update`, `current`, `unknown` — matches `kvr_cache` + frontend `pluginStatusCategory`.
fn parse_plugin_status_filter(sf: Option<&str>) -> Option<Vec<&'static str>> {
    let s = sf?;
    let t = s.trim();
    if t.is_empty() || t == "all" {
        return None;
    }
    let mut v = Vec::new();
    for part in t.split(',') {
        match part.trim() {
            "update" => v.push("update"),
            "current" => v.push("current"),
            "unknown" => v.push("unknown"),
            _ => {}
        }
    }
    if v.is_empty() { None } else { Some(v) }
}

/// Latest **complete** DAW scan that has at least one `daw_projects` row. Empty scans remain in history but must not shadow prior results.
/// Uses child-row presence (not `project_count`) so streaming scans still resolve after finalize quirks.
const LATEST_DAW_SCAN_ID_SQL: &str = "\
    SELECT s.id FROM daw_scans s \
    WHERE s.scan_complete = 1 \
    AND EXISTS (SELECT 1 FROM daw_projects p WHERE p.scan_id = s.id) \
    ORDER BY s.timestamp DESC LIMIT 1";

// ── Generic paginated query result for plugins/DAW/presets ──

#[derive(Debug, Serialize)]
pub struct PluginRow {
    pub name: String,
    pub path: String,
    #[serde(rename = "type")]
    pub plugin_type: String,
    pub version: String,
    pub manufacturer: String,
    #[serde(rename = "manufacturerUrl", skip_serializing_if = "Option::is_none")]
    pub manufacturer_url: Option<String>,
    pub size: String,
    #[serde(rename = "sizeBytes")]
    pub size_bytes: u64,
    pub modified: String,
    pub architectures: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct PluginQueryResult {
    pub plugins: Vec<PluginRow>,
    #[serde(rename = "totalCount")]
    pub total_count: u64,
    #[serde(
        rename = "totalCountCapped",
        skip_serializing_if = "std::ops::Not::not"
    )]
    pub total_count_capped: bool,
    #[serde(rename = "totalUnfiltered")]
    pub total_unfiltered: u64,
}

#[derive(Debug, Serialize)]
pub struct DawRow {
    pub name: String,
    pub path: String,
    pub directory: String,
    pub format: String,
    pub daw: String,
    pub size: u64,
    #[serde(rename = "sizeFormatted")]
    pub size_formatted: String,
    pub modified: String,
}

#[derive(Debug, Serialize)]
pub struct DawQueryResult {
    pub projects: Vec<DawRow>,
    #[serde(rename = "totalCount")]
    pub total_count: u64,
    #[serde(
        rename = "totalCountCapped",
        skip_serializing_if = "std::ops::Not::not"
    )]
    pub total_count_capped: bool,
    #[serde(rename = "totalUnfiltered")]
    pub total_unfiltered: u64,
}

#[derive(Debug, Serialize)]
pub struct PresetRow {
    pub name: String,
    pub path: String,
    pub directory: String,
    pub format: String,
    pub size: u64,
    #[serde(rename = "sizeFormatted")]
    pub size_formatted: String,
    pub modified: String,
}

#[derive(Debug, Serialize)]
pub struct PresetQueryResult {
    pub presets: Vec<PresetRow>,
    #[serde(rename = "totalCount")]
    pub total_count: u64,
    #[serde(
        rename = "totalCountCapped",
        skip_serializing_if = "std::ops::Not::not"
    )]
    pub total_count_capped: bool,
    #[serde(rename = "totalUnfiltered")]
    pub total_unfiltered: u64,
}

#[derive(Debug, Serialize)]
pub struct MidiQueryResult {
    #[serde(rename = "midiFiles")]
    pub midi_files: Vec<MidiFile>,
    #[serde(rename = "totalCount")]
    pub total_count: u64,
    #[serde(
        rename = "totalCountCapped",
        skip_serializing_if = "std::ops::Not::not"
    )]
    pub total_count_capped: bool,
    #[serde(rename = "totalUnfiltered")]
    pub total_unfiltered: u64,
}

#[derive(Debug, Serialize)]
pub struct VideoQueryResult {
    #[serde(rename = "videoFiles")]
    pub video_files: Vec<VideoFile>,
    #[serde(rename = "totalCount")]
    pub total_count: u64,
    #[serde(
        rename = "totalCountCapped",
        skip_serializing_if = "std::ops::Not::not"
    )]
    pub total_count_capped: bool,
    #[serde(rename = "totalUnfiltered")]
    pub total_unfiltered: u64,
}

#[derive(Debug, Serialize)]
pub struct PdfRow {
    pub name: String,
    pub path: String,
    pub directory: String,
    pub size: u64,
    #[serde(rename = "sizeFormatted")]
    pub size_formatted: String,
    pub modified: String,
}

#[derive(Debug, Serialize)]
pub struct PdfQueryResult {
    pub pdfs: Vec<PdfRow>,
    #[serde(rename = "totalCount")]
    pub total_count: u64,
    #[serde(
        rename = "totalCountCapped",
        skip_serializing_if = "std::ops::Not::not"
    )]
    pub total_count_capped: bool,
    #[serde(rename = "totalUnfiltered")]
    pub total_unfiltered: u64,
}

#[derive(Debug, Serialize)]
pub struct PdfStatsResult {
    #[serde(rename = "pdfCount")]
    pub pdf_count: u64,
    #[serde(rename = "totalBytes")]
    pub total_bytes: u64,
}

/// One row for heatmap dashboard “top folders” (matches `heatmap-dashboard.js` folder keys).
#[derive(Debug, Clone, Serialize)]
pub struct TopFolderRow {
    pub path: String,
    pub count: u64,
}

/// Filtered aggregate stats — count + size + per-type breakdown reflecting
/// the active search/filter. One round-trip: COUNT + SUM + GROUP BY in SQL.
#[derive(Debug, Serialize)]
#[derive(Default)]
pub struct FilterStatsResult {
    pub count: u64,
    /// True when `count` is a floor — per-type breakdown and bytes were skipped.
    #[serde(rename = "countCapped", skip_serializing_if = "std::ops::Not::not")]
    pub count_capped: bool,
    #[serde(rename = "totalBytes")]
    pub total_bytes: u64,
    #[serde(rename = "byType")]
    pub by_type: HashMap<String, u64>,
    #[serde(rename = "bytesByType")]
    pub bytes_by_type: HashMap<String, u64>,
    #[serde(rename = "totalUnfiltered")]
    pub total_unfiltered: u64,
    /// Audio-only: file-size histogram (6 buckets, same thresholds as `heatmap-dashboard.js`).
    #[serde(default, rename = "sizeBuckets")]
    pub size_buckets: Vec<u64>,
    /// Audio-only: BPM histogram (34 bins, 50–220 step 5) for heatmap; empty for non-audio.
    #[serde(default, skip_serializing_if = "Vec::is_empty", rename = "bpmBuckets")]
    pub bpm_buckets: Vec<u64>,
    /// Audio-only: library rows with `bpm IS NOT NULL` in the filtered scope.
    #[serde(default, rename = "bpmAnalyzedCount")]
    pub bpm_analyzed_count: u64,
    /// Audio-only: key_name → count (top keys) for heatmap key card.
    #[serde(
        default,
        skip_serializing_if = "HashMap::is_empty",
        rename = "keyCounts"
    )]
    pub key_counts: HashMap<String, u64>,
    #[serde(default, rename = "keyAnalyzedCount")]
    pub key_analyzed_count: u64,
    /// Audio-only: top directory groups (first 3 path segments), sorted by count.
    /// Always serialized (even `[]`) so the frontend can tell DB aggregates from a missing field.
    #[serde(default, rename = "topFolders")]
    pub top_folders: Vec<TopFolderRow>,
}


/// One row persisted for the last unified home-tree scan (SQLite `unified_scan_run.id` is always 1).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UnifiedScanRunRow {
    pub run_id: String,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub outcome: String,
    pub audio_scan_id: Option<String>,
    pub daw_scan_id: Option<String>,
    pub preset_scan_id: Option<String>,
    pub pdf_scan_id: Option<String>,
    pub roots_json: String,
    pub last_directory_path: Option<String>,
    pub error_message: Option<String>,
}

/// Current schema version — bump when adding migrations.
#[allow(dead_code)]
const SCHEMA_VERSION: i64 = 4;

/// Cap on extra connections when [`sqliteReadPoolExtra`] pref is `"auto"`.
/// Keep modest: explicit `0`–`32` extra still opens `1 + extra` readers — RSS scales with
/// [`read_pool_cache_kib`] / [`read_pool_mmap_bytes`] (budget-split, not fixed per handle).
const SQLITE_READ_POOL_AUTO_CAP: usize = 8;

/// Max explicit extra connections from preferences (`0` = one reader + writer; see README).
const SQLITE_READ_POOL_EXTRA_MAX: usize = 32;

/// Primary SQLite handle (migrations, `write_conn`): large page cache + mmap.
const SQLITE_CACHE_KIB_PRIMARY: i32 = -262_144; // 256 MiB (negative `cache_size` = KiB)
const SQLITE_MMAP_BYTES_PRIMARY: i64 = 536_870_912; // 512 MiB

/// ~512 MiB total page-cache budget split across **all** read-pool connections (e.g. `extra = 32`
/// → 33 readers × ~15.5 MiB each). Caps each reader at 32 MiB when the pool is small.
fn read_pool_cache_kib(num_read_connections: usize) -> i32 {
    const TOTAL_KIB_BUDGET: i64 = 512 * 1024; // 512 MiB in KiB units for SQLite negative cache_size
    let n = num_read_connections.max(1) as i64;
    let per_kib = (TOTAL_KIB_BUDGET / n).clamp(256, 32 * 1024);
    -(per_kib as i32)
}

/// ~512 MiB total mmap cap split across read-pool connections; each reader at most 64 MiB.
fn read_pool_mmap_bytes(num_read_connections: usize) -> i64 {
    const BUDGET: i64 = 512 * 1024 * 1024;
    let n = num_read_connections.max(1) as i64;
    (BUDGET / n).clamp(256 * 1024, 64 * 1024 * 1024)
}

fn init_sqlite_connection_pragmas(
    conn: &Connection,
    cache_kib: i32,
    mmap_cap: i64,
) -> Result<(), String> {
    conn.execute_batch(&format!(
        "PRAGMA journal_mode=WAL;
         PRAGMA synchronous=NORMAL;
         PRAGMA cache_size={cache_kib};
         PRAGMA mmap_size={mmap_cap};
         PRAGMA foreign_keys=ON;
         PRAGMA temp_store=MEMORY;
         PRAGMA wal_autocheckpoint=1000;",
    ))
    .map_err(|e| format!("Failed to set pragmas: {e}"))?;
    Ok(())
}

/// Per-query timeout for read-pool connections. The SQLite progress handler fires every
/// [`SQLITE_PROGRESS_HANDLER_OPS`] VM opcodes; if wall-clock time since the query started
/// exceeds this limit the query is interrupted (`SQLITE_INTERRUPT`).
const SQLITE_QUERY_TIMEOUT_SECS: u64 = 30;

/// Check interval for the progress handler — every N virtual-machine opcodes.
/// 1000 is ~50µs on modern hardware; low enough for responsive cancellation,
/// high enough to avoid measurable overhead on normal queries.
const SQLITE_PROGRESS_HANDLER_OPS: u32 = 1000;

fn open_db_connection_with_pragmas(
    db_path: &std::path::Path,
    cache_kib: i32,
    mmap_cap: i64,
) -> Result<Connection, String> {
    let conn = Connection::open(db_path).map_err(|e| format!("Failed to open database: {e}"))?;
    conn.busy_timeout(std::time::Duration::from_secs(30))
        .map_err(|e| format!("Failed to set busy_timeout: {e}"))?;
    init_sqlite_connection_pragmas(&conn, cache_kib, mmap_cap)?;
    install_regexp_function(&conn)?;
    Ok(conn)
}

/// Open a read-pool connection with a progress-handler query timeout.
/// The returned `Arc<AtomicU64>` must be stored alongside the connection and reset
/// (via [`reset_query_deadline`]) each time the connection is acquired from the pool.
fn open_read_connection(
    db_path: &std::path::Path,
    cache_kib: i32,
    mmap_cap: i64,
) -> Result<(Connection, Arc<AtomicU64>), String> {
    let conn = open_db_connection_with_pragmas(db_path, cache_kib, mmap_cap)?;
    let start = Arc::new(AtomicU64::new(now_epoch_ms()));
    let start_for_handler = Arc::clone(&start);
    let timeout_ms = SQLITE_QUERY_TIMEOUT_SECS * 1000;
    conn.progress_handler(
        SQLITE_PROGRESS_HANDLER_OPS as i32,
        Some(move || {
            now_epoch_ms().saturating_sub(start_for_handler.load(Ordering::Relaxed)) > timeout_ms
        }),
    )
    .map_err(|e| format!("Failed to set progress_handler: {e}"))?;
    Ok((conn, start))
}

/// Milliseconds since [`std::time::UNIX_EPOCH`] (monotonic enough for timeout purposes).
fn now_epoch_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// Reset the query-start timestamp so the progress handler measures from *now*.
#[inline]
fn reset_query_deadline(start: &AtomicU64) {
    start.store(now_epoch_ms(), Ordering::Relaxed);
}

fn sqlite_read_pool_auto() -> usize {
    num_cpus::get().clamp(2, SQLITE_READ_POOL_AUTO_CAP)
}

/// Resolves [`performance.sqliteReadPoolExtra`]: `"auto"` → [`sqlite_read_pool_auto`], else `0`..=`32`.
fn parse_sqlite_read_pool_extra_pref() -> usize {
    let val = crate::history::get_preference("sqliteReadPoolExtra");
    match val {
        Some(serde_json::Value::String(s)) => {
            let t = s.trim();
            if t.eq_ignore_ascii_case("auto") || t.is_empty() {
                sqlite_read_pool_auto()
            } else if let Ok(n) = t.parse::<usize>() {
                n.min(SQLITE_READ_POOL_EXTRA_MAX)
            } else {
                sqlite_read_pool_auto()
            }
        }
        Some(serde_json::Value::Number(n)) => {
            if let Some(u) = n.as_u64() {
                (u as usize).min(SQLITE_READ_POOL_EXTRA_MAX)
            } else if let Some(i) = n.as_i64() {
                (i.max(0) as usize).min(SQLITE_READ_POOL_EXTRA_MAX)
            } else {
                sqlite_read_pool_auto()
            }
        }
        None => sqlite_read_pool_auto(),
        _ => sqlite_read_pool_auto(),
    }
}

impl Database {
    /// User pref: *additional* read pool slots beyond the mandatory first reader (`1 + this`, min 1).
    fn read_pool_extra() -> usize {
        parse_sqlite_read_pool_extra_pref()
    }

    /// Count of read-pool connections (each participates in [`read_conn`] round-robin only).
    pub fn sqlite_read_pool_extra_slots(&self) -> usize {
        self.read.len()
    }

    /// Primary writer + read pool (`1 + read.len()` open file handles).
    pub fn sqlite_read_pool_total_handles(&self) -> usize {
        1 + self.read.len()
    }

    /// Exclusive writer — migrations, cache writes, and anything that should not compete with
    /// [`Self::read_conn`] for the same `Mutex`.
    #[inline]
    fn write_conn(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.write.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Round-robin read pool only (never the primary `write` handle — see [`Database`] docs).
    /// Resets the per-slot query-timeout deadline so the progress handler measures from *now*.
    #[inline]
    pub(crate) fn read_conn(&self) -> std::sync::MutexGuard<'_, Connection> {
        let n = self.read.len();
        if n == 0 {
            panic!("Database read pool is empty — Database::open() must add at least one reader");
        }
        let i = self.read_idx.fetch_add(1, Ordering::Relaxed) % n;
        reset_query_deadline(&self.read_deadlines[i]);
        self.read[i].lock().unwrap_or_else(|e| e.into_inner())
    }

    fn midi_library_total_rows(&self, conn: &Connection) -> Result<u64, String> {
        if let Ok(g) = self.midi_library_total_cache.lock()
            && let Some(n) = *g {
                return Ok(n);
            }
        let n: u64 = conn
            .query_row("SELECT COUNT(*) FROM midi_library", [], |r| {
                r.get::<_, i64>(0).map(|v| v as u64)
            })
            .unwrap_or(0);
        if let Ok(mut g) = self.midi_library_total_cache.lock() {
            *g = Some(n);
        }
        Ok(n)
    }

    fn invalidate_midi_library_total_cache(&self) {
        if let Ok(mut g) = self.midi_library_total_cache.lock() {
            *g = None;
        }
    }

    /// `query_video` / stats scope rows through `video_library` (one canonical `video_id` per path).
    /// Rebuild when row counts diverge, or when `video_id` no longer points at the row for `path`
    /// (stale ids after deletes/re-scans — `COUNT(*)` on `video_library` can still match distinct paths).
    fn ensure_video_library_aligned(&self, conn: &Connection) -> Result<(), String> {
        let lib_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM video_library", [], |r| r.get(0))
            .unwrap_or(0);
        let distinct_paths: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM (SELECT DISTINCT path FROM video_files)",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);
        if lib_count == distinct_paths {
            let stale_links: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM video_library lib WHERE NOT EXISTS (\
                     SELECT 1 FROM video_files v WHERE v.id = lib.video_id AND v.path = lib.path)",
                    [],
                    |r| r.get(0),
                )
                .unwrap_or(0);
            if stale_links == 0 {
                return Ok(());
            }
        }
        conn.execute_batch(
            "DELETE FROM video_library;
             INSERT INTO video_library (path, video_id)
             SELECT path, MAX(id) FROM video_files GROUP BY path;",
        )
        .map_err(|e| e.to_string())?;
        self.invalidate_video_library_total_cache();
        Ok(())
    }

    fn video_library_total_rows(&self, conn: &Connection) -> Result<u64, String> {
        self.ensure_video_library_aligned(conn)?;
        if let Ok(g) = self.video_library_total_cache.lock()
            && let Some(n) = *g {
                return Ok(n);
            }
        let n: u64 = conn
            .query_row("SELECT COUNT(*) FROM video_library", [], |r| {
                r.get::<_, i64>(0).map(|v| v as u64)
            })
            .unwrap_or(0);
        if let Ok(mut g) = self.video_library_total_cache.lock() {
            *g = Some(n);
        }
        Ok(n)
    }

    fn invalidate_video_library_total_cache(&self) {
        if let Ok(mut g) = self.video_library_total_cache.lock() {
            *g = None;
        }
    }

    fn pdf_library_total_rows(&self, conn: &Connection) -> Result<u64, String> {
        if let Ok(g) = self.pdf_library_total_cache.lock()
            && let Some(n) = *g {
                return Ok(n);
            }
        let n: u64 = conn
            .query_row("SELECT COUNT(*) FROM pdf_library", [], |r| {
                r.get::<_, i64>(0).map(|v| v as u64)
            })
            .unwrap_or(0);
        if let Ok(mut g) = self.pdf_library_total_cache.lock() {
            *g = Some(n);
        }
        Ok(n)
    }

    fn invalidate_pdf_library_total_cache(&self) {
        if let Ok(mut g) = self.pdf_library_total_cache.lock() {
            *g = None;
        }
    }

    fn audio_library_total_rows(&self, conn: &Connection) -> Result<u64, String> {
        if let Ok(g) = self.audio_library_total_cache.lock()
            && let Some(n) = *g {
                return Ok(n);
            }
        let n: u64 = conn
            .query_row("SELECT COUNT(*) FROM audio_library", [], |r| {
                r.get::<_, i64>(0).map(|v| v as u64)
            })
            .unwrap_or(0);
        if let Ok(mut g) = self.audio_library_total_cache.lock() {
            *g = Some(n);
        }
        Ok(n)
    }

    fn invalidate_audio_library_total_cache(&self) {
        if let Ok(mut g) = self.audio_library_total_cache.lock() {
            *g = None;
        }
    }

    fn preset_inventory_total_rows(&self, conn: &Connection) -> Result<u64, String> {
        if let Ok(g) = self.preset_inventory_total_cache.lock()
            && let Some(n) = *g {
                return Ok(n);
            }
        let n: u64 = conn
            .query_row(
                "SELECT COUNT(*) FROM presets WHERE id IN (SELECT preset_id FROM preset_library) AND format NOT IN ('MID','MIDI')",
                [],
                |r| r.get::<_, i64>(0).map(|v| v as u64),
            )
            .unwrap_or(0);
        if let Ok(mut g) = self.preset_inventory_total_cache.lock() {
            *g = Some(n);
        }
        Ok(n)
    }

    fn invalidate_preset_inventory_total_cache(&self) {
        if let Ok(mut g) = self.preset_inventory_total_cache.lock() {
            *g = None;
        }
    }

    fn daw_library_total_rows(&self, conn: &Connection) -> Result<u64, String> {
        if let Ok(g) = self.daw_library_total_cache.lock()
            && let Some(n) = *g {
                return Ok(n);
            }
        let n: u64 = conn
            .query_row("SELECT COUNT(*) FROM daw_library", [], |r| {
                r.get::<_, i64>(0).map(|v| v as u64)
            })
            .unwrap_or(0);
        if let Ok(mut g) = self.daw_library_total_cache.lock() {
            *g = Some(n);
        }
        Ok(n)
    }

    fn invalidate_daw_library_total_cache(&self) {
        if let Ok(mut g) = self.daw_library_total_cache.lock() {
            *g = None;
        }
    }

    fn plugin_library_total_rows(&self, conn: &Connection) -> Result<u64, String> {
        if let Ok(g) = self.plugin_library_total_cache.lock()
            && let Some(n) = *g {
                return Ok(n);
            }
        let n: u64 = conn
            .query_row("SELECT COUNT(*) FROM plugin_library", [], |r| {
                r.get::<_, i64>(0).map(|v| v as u64)
            })
            .unwrap_or(0);
        if let Ok(mut g) = self.plugin_library_total_cache.lock() {
            *g = Some(n);
        }
        Ok(n)
    }

    fn invalidate_plugin_library_total_cache(&self) {
        if let Ok(mut g) = self.plugin_library_total_cache.lock() {
            *g = None;
        }
    }

    /// SQL bodies for [`sync_*_after_paths_refresh`] (standalone: wrapped in `BEGIN IMMEDIATE` by
    /// [`exec_sync_paths_refresh`]). For preset/midi/pdf flows already inside a
    /// [`Transaction`], use [`sync_preset_library_after_paths_refresh_tx`] /
    /// [`sync_midi_library_after_paths_refresh_tx`] /
    /// [`sync_pdf_library_after_paths_refresh_tx`] via [`exec_sync_paths_refresh_tx`] — do **not**
    /// nest another `BEGIN` (SQLite rejects it).
    const SYNC_AUDIO_LIBRARY_PATHS_SQL: &'static str = r#"DELETE FROM audio_library WHERE path IN (SELECT path FROM _al_refresh_paths) AND path NOT IN (SELECT DISTINCT path FROM audio_samples);
INSERT OR REPLACE INTO audio_library (path, sample_id)
 SELECT path, MAX(id) FROM audio_samples WHERE path IN (SELECT path FROM _al_refresh_paths) GROUP BY path;
DROP TABLE _al_refresh_paths;"#;

    const SYNC_PDF_LIBRARY_PATHS_SQL: &'static str = r#"DELETE FROM pdf_library WHERE path IN (SELECT path FROM _pdf_lib_refresh_paths) AND path NOT IN (SELECT DISTINCT path FROM pdfs);
INSERT OR REPLACE INTO pdf_library (path, pdf_id)
 SELECT path, MAX(id) FROM pdfs WHERE path IN (SELECT path FROM _pdf_lib_refresh_paths) GROUP BY path;
DROP TABLE _pdf_lib_refresh_paths;"#;

    const SYNC_MIDI_LIBRARY_PATHS_SQL: &'static str = r#"DELETE FROM midi_library WHERE path IN (SELECT path FROM _midi_lib_refresh_paths) AND path NOT IN (SELECT DISTINCT path FROM midi_files);
INSERT OR REPLACE INTO midi_library (path, midi_id)
 SELECT path, MAX(id) FROM midi_files WHERE path IN (SELECT path FROM _midi_lib_refresh_paths) GROUP BY path;
DROP TABLE _midi_lib_refresh_paths;"#;

    const SYNC_VIDEO_LIBRARY_PATHS_SQL: &'static str = r#"DELETE FROM video_library WHERE path IN (SELECT path FROM _video_lib_refresh_paths) AND path NOT IN (SELECT DISTINCT path FROM video_files);
INSERT OR REPLACE INTO video_library (path, video_id)
 SELECT path, MAX(id) FROM video_files WHERE path IN (SELECT path FROM _video_lib_refresh_paths) GROUP BY path;
DROP TABLE _video_lib_refresh_paths;"#;

    const SYNC_PRESET_LIBRARY_PATHS_SQL: &'static str = r#"DELETE FROM preset_library WHERE path IN (SELECT path FROM _preset_lib_refresh_paths) AND path NOT IN (SELECT DISTINCT path FROM presets);
INSERT OR REPLACE INTO preset_library (path, preset_id)
 SELECT path, MAX(id) FROM presets WHERE path IN (SELECT path FROM _preset_lib_refresh_paths) GROUP BY path;
DROP TABLE _preset_lib_refresh_paths;"#;

    const SYNC_DAW_LIBRARY_PATHS_SQL: &'static str = r#"DELETE FROM daw_library WHERE path IN (SELECT path FROM _dl_refresh_paths) AND path NOT IN (SELECT DISTINCT path FROM daw_projects);
INSERT OR REPLACE INTO daw_library (path, project_id)
 SELECT path, MAX(id) FROM daw_projects WHERE path IN (SELECT path FROM _dl_refresh_paths) GROUP BY path;
DROP TABLE _dl_refresh_paths;"#;

    const SYNC_PLUGIN_LIBRARY_PATHS_SQL: &'static str = r#"DELETE FROM plugin_library WHERE path IN (SELECT path FROM _pl_refresh_paths) AND path NOT IN (SELECT DISTINCT path FROM plugins);
INSERT OR REPLACE INTO plugin_library (path, plugin_id)
 SELECT path, MAX(id) FROM plugins WHERE path IN (SELECT path FROM _pl_refresh_paths) GROUP BY path;
DROP TABLE _pl_refresh_paths;"#;

    fn exec_sync_paths_refresh(conn: &Connection, sql: &str) -> Result<(), String> {
        conn.execute_batch(&format!("BEGIN IMMEDIATE;\n{sql}\nCOMMIT;"))
            .map_err(|e| e.to_string())
    }

    fn exec_sync_paths_refresh_tx(tx: &Transaction<'_>, sql: &str) -> Result<(), String> {
        tx.execute_batch(sql).map_err(|e| e.to_string())
    }

    /// `_al_refresh_paths` lists paths touched by removing `audio_samples` for a scan; those rows
    /// must already be deleted. Reconciles `audio_library` with remaining `audio_samples` rows.
    fn sync_audio_library_after_paths_refresh(conn: &Connection) -> Result<(), String> {
        Self::exec_sync_paths_refresh(conn, Self::SYNC_AUDIO_LIBRARY_PATHS_SQL)
    }

    fn sync_pdf_library_after_paths_refresh(conn: &Connection) -> Result<(), String> {
        Self::exec_sync_paths_refresh(conn, Self::SYNC_PDF_LIBRARY_PATHS_SQL)
    }

    fn sync_pdf_library_after_paths_refresh_tx(tx: &Transaction<'_>) -> Result<(), String> {
        Self::exec_sync_paths_refresh_tx(tx, Self::SYNC_PDF_LIBRARY_PATHS_SQL)
    }

    fn sync_midi_library_after_paths_refresh(conn: &Connection) -> Result<(), String> {
        Self::exec_sync_paths_refresh(conn, Self::SYNC_MIDI_LIBRARY_PATHS_SQL)
    }

    fn sync_midi_library_after_paths_refresh_tx(tx: &Transaction<'_>) -> Result<(), String> {
        Self::exec_sync_paths_refresh_tx(tx, Self::SYNC_MIDI_LIBRARY_PATHS_SQL)
    }

    fn sync_video_library_after_paths_refresh(conn: &Connection) -> Result<(), String> {
        Self::exec_sync_paths_refresh(conn, Self::SYNC_VIDEO_LIBRARY_PATHS_SQL)
    }

    fn sync_preset_library_after_paths_refresh(conn: &Connection) -> Result<(), String> {
        Self::exec_sync_paths_refresh(conn, Self::SYNC_PRESET_LIBRARY_PATHS_SQL)
    }

    fn sync_preset_library_after_paths_refresh_tx(tx: &Transaction<'_>) -> Result<(), String> {
        Self::exec_sync_paths_refresh_tx(tx, Self::SYNC_PRESET_LIBRARY_PATHS_SQL)
    }

    /// `_dl_refresh_paths` lists paths touched by removing `daw_projects` rows for a scan; those rows
    /// must already be deleted. Reconciles `daw_library` with remaining `daw_projects` rows.
    fn sync_daw_library_after_paths_refresh(conn: &Connection) -> Result<(), String> {
        Self::exec_sync_paths_refresh(conn, Self::SYNC_DAW_LIBRARY_PATHS_SQL)
    }

    fn rebuild_daw_library(conn: &Connection) -> Result<(), String> {
        conn.execute_batch(
            "BEGIN IMMEDIATE;
             DELETE FROM daw_library;
             INSERT INTO daw_library (path, project_id) SELECT path, MAX(id) FROM daw_projects GROUP BY path;
             COMMIT;",
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// `_pl_refresh_paths` lists paths touched by removing `plugins` rows for a scan; reconciles
    /// `plugin_library` with remaining `plugins` rows (same pattern as `sync_daw_library_after_paths_refresh`).
    fn sync_plugin_library_after_paths_refresh(conn: &Connection) -> Result<(), String> {
        Self::exec_sync_paths_refresh(conn, Self::SYNC_PLUGIN_LIBRARY_PATHS_SQL)
    }

    fn rebuild_plugin_library(conn: &Connection) -> Result<(), String> {
        conn.execute_batch(
            "BEGIN IMMEDIATE;
             DELETE FROM plugin_library;
             INSERT INTO plugin_library (path, plugin_id) SELECT path, MAX(id) FROM plugins GROUP BY path;
             COMMIT;",
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Full rebuild after bulk deletes (e.g. `prune_old_scans`) where per-path sync is impractical.
    ///
    /// One transaction: without it, each `DELETE`/`INSERT` autocommits separately — a crash or killed
    /// process after `DELETE FROM midi_library` but before `INSERT` could leave `*_library` empty
    /// while `midi_files` (etc.) still had rows.
    fn rebuild_pdf_midi_preset_daw_libraries(conn: &mut Connection) -> Result<(), String> {
        let tx = conn.transaction().map_err(|e| e.to_string())?;
        tx.execute_batch(
            "DELETE FROM pdf_library;
             INSERT INTO pdf_library (path, pdf_id) SELECT path, MAX(id) FROM pdfs GROUP BY path;
             DELETE FROM midi_library;
             INSERT INTO midi_library (path, midi_id) SELECT path, MAX(id) FROM midi_files GROUP BY path;
             DELETE FROM video_library;
             INSERT INTO video_library (path, video_id) SELECT path, MAX(id) FROM video_files GROUP BY path;
             DELETE FROM preset_library;
             INSERT INTO preset_library (path, preset_id) SELECT path, MAX(id) FROM presets GROUP BY path;
             DELETE FROM daw_library;
             INSERT INTO daw_library (path, project_id) SELECT path, MAX(id) FROM daw_projects GROUP BY path;
             DELETE FROM plugin_library;
             INSERT INTO plugin_library (path, plugin_id) SELECT path, MAX(id) FROM plugins GROUP BY path;",
        )
        .map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Open or create the database in the app data directory.
    pub fn open() -> Result<Self, String> {
        let db_path = history::get_data_dir().join("audio_haxor.db");
        let _ = std::fs::create_dir_all(db_path.parent().unwrap());
        // Parallel `cargo test` processes can open the same path; busy_timeout avoids
        // immediate `database is locked` during migrations.
        let write = open_db_connection_with_pragmas(
            &db_path,
            SQLITE_CACHE_KIB_PRIMARY,
            SQLITE_MMAP_BYTES_PRIMARY,
        )?;
        let mut db = Self {
            write: Mutex::new(write),
            read: Vec::new(),
            read_deadlines: Vec::new(),
            read_idx: AtomicUsize::new(0),
            midi_library_total_cache: Mutex::new(None),
            video_library_total_cache: Mutex::new(None),
            pdf_library_total_cache: Mutex::new(None),
            audio_library_total_cache: Mutex::new(None),
            preset_inventory_total_cache: Mutex::new(None),
            daw_library_total_cache: Mutex::new(None),
            plugin_library_total_cache: Mutex::new(None),
        };
        db.migrate()?;
        // At least one dedicated reader so `read_conn` never steals the primary handle (which
        // must remain available for migrations / writes without blocking every paginated query).
        // Pref `sqliteReadPoolExtra` is "extra" slots: total readers = 1 + extra (min 1).
        let n_read = (1 + Self::read_pool_extra()).max(1);
        let read_cache_kib = read_pool_cache_kib(n_read);
        let read_mmap = read_pool_mmap_bytes(n_read);
        for _ in 0..n_read {
            let (conn, deadline) = open_read_connection(&db_path, read_cache_kib, read_mmap)?;
            db.read.push(Mutex::new(conn));
            db.read_deadlines.push(deadline);
        }
        Ok(db)
    }

    /// Quick startup path: query planner refresh + cache touch. Safe from any thread; keep fast.
    pub fn housekeep_light(&self) {
        {
            let conn = self.read_conn();
            let _ = conn.execute_batch("PRAGMA optimize;");
        }
        self.prewarm();
    }

    /// Expensive path: prune old scans (DELETE + full `*_library` rebuild) and optional `VACUUM`.
    /// Run **well after** the window and `setup()` have finished so pooled `read_conn()` scopes are
    /// not held across first-frame IPC (long prune batches still contend on SQLite + pool slots).
    pub fn housekeep_heavy(&self) {
        if Self::prune_old_scans_pref_enabled() {
            self.prune_old_scans(Self::prune_old_scans_keep_pref());
        }
        self.vacuum_if_needed();
    }

    fn prune_old_scans_pref_enabled() -> bool {
        crate::history::get_preference("pruneOldScans")
            .and_then(|v| v.as_str().map(str::to_owned))
            .map(|s| s == "on" || s == "true")
            .unwrap_or(false)
    }

    /// `[performance]` → `pruneOldScansKeep` (default 3, clamped 1..=100).
    fn prune_old_scans_keep_pref() -> usize {
        const DEFAULT: usize = 3;
        const MIN: usize = 1;
        const MAX: usize = 100;
        let raw = crate::history::get_preference("pruneOldScansKeep")
            .and_then(|v| v.as_str().map(str::to_owned))
            .unwrap_or_else(|| DEFAULT.to_string());
        let n = raw.parse::<usize>().unwrap_or(DEFAULT);
        n.clamp(MIN, MAX)
    }

    /// Full sequence (manual / tests). Startup uses [`Self::housekeep_light`] + delayed [`Self::housekeep_heavy`].
    pub fn housekeep(&self) {
        self.housekeep_light();
        self.housekeep_heavy();
    }

    /// Prune old scans — keep only the N most recent **complete** scans per type. Incomplete
    /// (user-stopped) runs are retained until superseded or cleared so library rows stay addressable.
    pub fn prune_old_scans(&self, keep: usize) {
        let keep_i = keep as i64;
        for (scan_tbl, data_tbl, id_col) in [
            ("audio_scans", "audio_samples", "scan_id"),
            ("plugin_scans", "plugins", "scan_id"),
            ("daw_scans", "daw_projects", "scan_id"),
            ("preset_scans", "presets", "scan_id"),
            ("midi_scans", "midi_files", "scan_id"),
            ("video_scans", "video_files", "scan_id"),
            ("pdf_scans", "pdfs", "scan_id"),
        ] {
            // One `read_conn()` scope per domain so we do not hold a pooled handle across all
            // DELETE batches — other threads can use different handles (or the main thread during
            // startup can finish `setup` while prune runs).
            let conn = self.read_conn();
            let _ = conn.execute_batch(&format!(
                "DELETE FROM {data_tbl} WHERE {id_col} IN (\
                    SELECT id FROM {scan_tbl} WHERE scan_complete = 1 AND id NOT IN (\
                        SELECT id FROM {scan_tbl} WHERE scan_complete = 1 ORDER BY timestamp DESC LIMIT {keep_i}\
                    )\
                );\
                DELETE FROM {scan_tbl} WHERE scan_complete = 1 AND id NOT IN (\
                    SELECT id FROM {scan_tbl} WHERE scan_complete = 1 ORDER BY timestamp DESC LIMIT {keep_i}\
                );"
            ));
        }
        let mut conn = self.read_conn();
        if let Err(e) = Self::rebuild_pdf_midi_preset_daw_libraries(&mut conn) {
            crate::append_log(format!(
                "rebuild_pdf_midi_preset_daw_libraries after prune_old_scans failed: {e}"
            ));
        } else {
            self.invalidate_preset_inventory_total_cache();
            self.invalidate_midi_library_total_cache();
            self.invalidate_video_library_total_cache();
            self.invalidate_pdf_library_total_cache();
            self.invalidate_daw_library_total_cache();
            self.invalidate_plugin_library_total_cache();
        }
    }

    /// Mark whether a streaming scan finished normally (`complete`) or was user-stopped (partial).
    pub fn set_audio_scan_complete(&self, id: &str, complete: bool) -> Result<(), String> {
        let conn = self.read_conn();
        conn.execute(
            "UPDATE audio_scans SET scan_complete = ?2 WHERE id = ?1",
            params![id, if complete { 1 } else { 0 }],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn set_plugin_scan_complete(&self, id: &str, complete: bool) -> Result<(), String> {
        let conn = self.read_conn();
        conn.execute(
            "UPDATE plugin_scans SET scan_complete = ?2 WHERE id = ?1",
            params![id, if complete { 1 } else { 0 }],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn set_daw_scan_complete(&self, id: &str, complete: bool) -> Result<(), String> {
        let conn = self.read_conn();
        conn.execute(
            "UPDATE daw_scans SET scan_complete = ?2 WHERE id = ?1",
            params![id, if complete { 1 } else { 0 }],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn set_preset_scan_complete(&self, id: &str, complete: bool) -> Result<(), String> {
        let conn = self.read_conn();
        conn.execute(
            "UPDATE preset_scans SET scan_complete = ?2 WHERE id = ?1",
            params![id, if complete { 1 } else { 0 }],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn set_midi_scan_complete(&self, id: &str, complete: bool) -> Result<(), String> {
        let conn = self.read_conn();
        conn.execute(
            "UPDATE midi_scans SET scan_complete = ?2 WHERE id = ?1",
            params![id, if complete { 1 } else { 0 }],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn set_video_scan_complete(&self, id: &str, complete: bool) -> Result<(), String> {
        let conn = self.read_conn();
        conn.execute(
            "UPDATE video_scans SET scan_complete = ?2 WHERE id = ?1",
            params![id, if complete { 1 } else { 0 }],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn set_pdf_scan_complete(&self, id: &str, complete: bool) -> Result<(), String> {
        let conn = self.read_conn();
        conn.execute(
            "UPDATE pdf_scans SET scan_complete = ?2 WHERE id = ?1",
            params![id, if complete { 1 } else { 0 }],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Checkpoint WAL to merge it into the main DB file. Keeps WAL small.
    /// Warm the page cache by touching each table + FTS index root. First real
    /// query returns ~1 ms instead of the 50-200 ms cold-cache penalty.
    pub fn prewarm(&self) {
        let conn = self.read_conn();
        let _ = conn.execute_batch(
            "SELECT COUNT(*) FROM audio_samples WHERE id=1;
             SELECT COUNT(*) FROM daw_projects WHERE id=1;
             SELECT COUNT(*) FROM presets WHERE id=1;
             SELECT COUNT(*) FROM midi_files WHERE id=1;
             SELECT COUNT(*) FROM video_files WHERE id=1;
             SELECT COUNT(*) FROM pdfs WHERE id=1;
             SELECT COUNT(*) FROM plugins WHERE id=1;
             SELECT rowid FROM audio_samples_fts WHERE audio_samples_fts MATCH 'xzyq' LIMIT 1;
             SELECT rowid FROM daw_projects_fts WHERE daw_projects_fts MATCH 'xzyq' LIMIT 1;
             SELECT rowid FROM presets_fts WHERE presets_fts MATCH 'xzyq' LIMIT 1;
             SELECT rowid FROM midi_files_fts WHERE midi_files_fts MATCH 'xzyq' LIMIT 1;
             SELECT rowid FROM video_files_fts WHERE video_files_fts MATCH 'xzyq' LIMIT 1;
             SELECT rowid FROM pdfs_fts WHERE pdfs_fts MATCH 'xzyq' LIMIT 1;",
        );
    }

    pub fn checkpoint(&self) {
        let conn = self.read_conn();
        let _ = conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);");
    }

    /// Resolved app UI strings for the given locale (merged with English fallback).
    pub fn get_app_strings(&self, locale: &str) -> Result<HashMap<String, String>, String> {
        let conn = self.read_conn();
        crate::app_i18n::load_merged(&conn, locale)
    }

    /// Alias for [`Self::get_app_strings`] (legacy command name).
    pub fn get_toast_strings(&self, locale: &str) -> Result<HashMap<String, String>, String> {
        self.get_app_strings(locale)
    }

    /// VACUUM if >20% of pages are free (dead space from deleted rows).
    pub fn vacuum_if_needed(&self) {
        let conn = self.read_conn();
        let page_size: u64 = conn
            .query_row("PRAGMA page_size", [], |r| r.get::<_, i64>(0))
            .unwrap_or(4096) as u64;
        let page_count: u64 = conn
            .query_row("PRAGMA page_count", [], |r| r.get::<_, i64>(0))
            .unwrap_or(0) as u64;
        let free_count: u64 = conn
            .query_row("PRAGMA freelist_count", [], |r| r.get::<_, i64>(0))
            .unwrap_or(0) as u64;
        let pct = if page_count > 0 {
            free_count * 100 / page_count
        } else {
            0
        };
        if pct > 20 {
            let before = page_count * page_size;
            crate::append_log(format!(
                "DB VACUUM — {}% free ({} / {} pages) | before: {}",
                pct,
                free_count,
                page_count,
                crate::format_size(before),
            ));
            drop(conn);
            let conn = self.read_conn();
            let _ = conn.execute_batch("VACUUM;");
            let after: u64 = conn
                .query_row("PRAGMA page_count", [], |r| r.get::<_, i64>(0))
                .unwrap_or(0) as u64
                * page_size;
            crate::append_log(format!(
                "DB VACUUM DONE — after: {}",
                crate::format_size(after)
            ));
        }
    }

    /// One-time migration: normalize `plugins.path` and `plugin_scans` directories/roots JSON to
    /// [`normalize_path_for_db`] form; remove duplicate `(canonical path, scan_id)` rows (keep max `id`).
    fn migrate_plugin_paths_canonical(conn: &Connection) -> Result<(), String> {
        use std::collections::{HashMap, HashSet};

        let mut stmt = conn
            .prepare("SELECT id, path, scan_id FROM plugins")
            .map_err(|e| e.to_string())?;
        let rows: Vec<(i64, String, String)> = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;

        let mut by_key: HashMap<(String, String), Vec<i64>> = HashMap::new();
        for (id, path, scan_id) in &rows {
            let canon = normalize_path_for_db(path);
            by_key
                .entry((canon, scan_id.clone()))
                .or_default()
                .push(*id);
        }

        let mut to_delete = HashSet::new();
        for ids in by_key.values() {
            if ids.len() <= 1 {
                continue;
            }
            let mut v = ids.clone();
            v.sort_unstable();
            for id in &v[..v.len() - 1] {
                to_delete.insert(*id);
            }
        }

        let deleted = to_delete.len();
        if !to_delete.is_empty() {
            let mut ids: Vec<i64> = to_delete.iter().copied().collect();
            ids.sort_unstable();
            let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
            conn.execute(
                &format!("DELETE FROM plugins WHERE id IN ({})", placeholders),
                rusqlite::params_from_iter(ids.iter().copied()),
            )
            .map_err(|e| e.to_string())?;
        }

        let mut path_updates = 0usize;
        for (id, path, _) in &rows {
            if to_delete.contains(id) {
                continue;
            }
            let canon = normalize_path_for_db(path);
            if canon != *path {
                conn.execute(
                    "UPDATE plugins SET path = ?1 WHERE id = ?2",
                    params![canon, id],
                )
                .map_err(|e| e.to_string())?;
                path_updates += 1;
            }
        }

        let mut json_updates = 0usize;
        let mut stmt = conn
            .prepare("SELECT id, directories, roots FROM plugin_scans")
            .map_err(|e| e.to_string())?;
        let scan_rows: Vec<(String, String, String)> = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;

        for (id, dirs, roots) in scan_rows {
            let dirs_vec: Vec<String> = serde_json::from_str(&dirs).unwrap_or_default();
            let roots_vec: Vec<String> = serde_json::from_str(&roots).unwrap_or_default();
            let d2 = path_strings_json_normalized(&dirs_vec);
            let r2 = path_strings_json_normalized(&roots_vec);
            if d2 != dirs || r2 != roots {
                conn.execute(
                    "UPDATE plugin_scans SET directories = ?1, roots = ?2 WHERE id = ?3",
                    params![d2, r2, id],
                )
                .map_err(|e| e.to_string())?;
                json_updates += 1;
            }
        }

        if deleted > 0 || path_updates > 0 || json_updates > 0 {
            crate::append_log(format!(
                "DB migration v17: plugins deduped={deleted}, path rewrites={path_updates}, plugin_scans JSON rows={json_updates}"
            ));
        }

        Ok(())
    }

    /// Run schema migrations.
    fn migrate(&self) -> Result<(), String> {
        let conn = self.write_conn();

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS schema_version (
                version INTEGER NOT NULL
            );",
        )
        .map_err(|e| e.to_string())?;

        let current: i64 = conn
            .query_row(
                "SELECT COALESCE(MAX(version), 0) FROM schema_version",
                [],
                |row| row.get::<_, i64>(0),
            )
            .unwrap_or(0);

        if current < 1 {
            conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS audio_samples (
                    id              INTEGER PRIMARY KEY,
                    name            TEXT NOT NULL,
                    path            TEXT NOT NULL,
                    directory       TEXT NOT NULL,
                    format          TEXT NOT NULL,
                    size            INTEGER NOT NULL,
                    size_formatted  TEXT NOT NULL,
                    modified        TEXT NOT NULL,
                    duration        REAL,
                    channels        INTEGER,
                    sample_rate     INTEGER,
                    bits_per_sample INTEGER,
                    bpm             REAL,
                    key_name        TEXT,
                    lufs            REAL,
                    bpm_exhausted   INTEGER NOT NULL DEFAULT 0,
                    scan_id         TEXT NOT NULL,
                    created_at      TEXT NOT NULL DEFAULT (datetime('now'))
                );

                CREATE UNIQUE INDEX IF NOT EXISTS idx_samples_path_scan
                    ON audio_samples(path, scan_id);
                CREATE INDEX IF NOT EXISTS idx_samples_name
                    ON audio_samples(name COLLATE NOCASE);
                CREATE INDEX IF NOT EXISTS idx_samples_format
                    ON audio_samples(format);
                CREATE INDEX IF NOT EXISTS idx_samples_scan_id
                    ON audio_samples(scan_id);
                CREATE INDEX IF NOT EXISTS idx_samples_bpm
                    ON audio_samples(bpm);
                CREATE INDEX IF NOT EXISTS idx_samples_key
                    ON audio_samples(key_name);
                CREATE INDEX IF NOT EXISTS idx_samples_lufs
                    ON audio_samples(lufs);

                CREATE TABLE IF NOT EXISTS audio_scans (
                    id              TEXT PRIMARY KEY,
                    timestamp       TEXT NOT NULL,
                    sample_count    INTEGER NOT NULL,
                    total_bytes     INTEGER NOT NULL,
                    format_counts   TEXT NOT NULL,
                    roots           TEXT NOT NULL
                );

                CREATE TABLE IF NOT EXISTS waveform_cache (
                    path TEXT PRIMARY KEY,
                    data TEXT NOT NULL
                );

                CREATE TABLE IF NOT EXISTS spectrogram_cache (
                    path TEXT PRIMARY KEY,
                    data TEXT NOT NULL
                );

                INSERT INTO schema_version (version) VALUES (1);",
            )
            .map_err(|e| format!("Migration v1 failed: {e}"))?;
        }

        if current < 2 {
            conn.execute_batch(
                "-- Plugin scan history
                CREATE TABLE IF NOT EXISTS plugins (
                    id              INTEGER PRIMARY KEY,
                    name            TEXT NOT NULL,
                    path            TEXT NOT NULL,
                    plugin_type     TEXT NOT NULL,
                    version         TEXT NOT NULL,
                    manufacturer    TEXT NOT NULL,
                    manufacturer_url TEXT,
                    size            TEXT NOT NULL,
                    size_bytes      INTEGER NOT NULL DEFAULT 0,
                    modified        TEXT NOT NULL,
                    architectures   TEXT NOT NULL DEFAULT '[]',
                    scan_id         TEXT NOT NULL
                );
                CREATE UNIQUE INDEX IF NOT EXISTS idx_plugins_path_scan ON plugins(path, scan_id);
                CREATE INDEX IF NOT EXISTS idx_plugins_name ON plugins(name COLLATE NOCASE);
                CREATE INDEX IF NOT EXISTS idx_plugins_scan_id ON plugins(scan_id);

                CREATE TABLE IF NOT EXISTS plugin_scans (
                    id              TEXT PRIMARY KEY,
                    timestamp       TEXT NOT NULL,
                    plugin_count    INTEGER NOT NULL,
                    directories     TEXT NOT NULL,
                    roots           TEXT NOT NULL
                );

                -- DAW project history
                CREATE TABLE IF NOT EXISTS daw_projects (
                    id              INTEGER PRIMARY KEY,
                    name            TEXT NOT NULL,
                    path            TEXT NOT NULL,
                    directory       TEXT NOT NULL,
                    format          TEXT NOT NULL,
                    daw             TEXT NOT NULL,
                    size            INTEGER NOT NULL,
                    size_formatted  TEXT NOT NULL,
                    modified        TEXT NOT NULL,
                    scan_id         TEXT NOT NULL
                );
                CREATE UNIQUE INDEX IF NOT EXISTS idx_daw_path_scan ON daw_projects(path, scan_id);
                CREATE INDEX IF NOT EXISTS idx_daw_name ON daw_projects(name COLLATE NOCASE);
                CREATE INDEX IF NOT EXISTS idx_daw_scan_id ON daw_projects(scan_id);

                CREATE TABLE IF NOT EXISTS daw_scans (
                    id              TEXT PRIMARY KEY,
                    timestamp       TEXT NOT NULL,
                    project_count   INTEGER NOT NULL,
                    total_bytes     INTEGER NOT NULL,
                    daw_counts      TEXT NOT NULL,
                    roots           TEXT NOT NULL
                );

                -- Preset history
                CREATE TABLE IF NOT EXISTS presets (
                    id              INTEGER PRIMARY KEY,
                    name            TEXT NOT NULL,
                    path            TEXT NOT NULL,
                    directory       TEXT NOT NULL,
                    format          TEXT NOT NULL,
                    size            INTEGER NOT NULL,
                    size_formatted  TEXT NOT NULL,
                    modified        TEXT NOT NULL,
                    scan_id         TEXT NOT NULL
                );
                CREATE UNIQUE INDEX IF NOT EXISTS idx_presets_path_scan ON presets(path, scan_id);
                CREATE INDEX IF NOT EXISTS idx_presets_name ON presets(name COLLATE NOCASE);
                CREATE INDEX IF NOT EXISTS idx_presets_scan_id ON presets(scan_id);

                CREATE TABLE IF NOT EXISTS preset_scans (
                    id              TEXT PRIMARY KEY,
                    timestamp       TEXT NOT NULL,
                    preset_count    INTEGER NOT NULL,
                    total_bytes     INTEGER NOT NULL,
                    format_counts   TEXT NOT NULL,
                    roots           TEXT NOT NULL
                );

                -- KVR version cache
                CREATE TABLE IF NOT EXISTS kvr_cache (
                    plugin_key      TEXT PRIMARY KEY,
                    kvr_url         TEXT,
                    update_url      TEXT,
                    latest_version  TEXT,
                    has_update      INTEGER NOT NULL DEFAULT 0,
                    source          TEXT NOT NULL DEFAULT '',
                    timestamp       TEXT NOT NULL DEFAULT ''
                );

                -- Plugin cross-reference cache
                CREATE TABLE IF NOT EXISTS xref_cache (
                    project_path    TEXT PRIMARY KEY,
                    plugins_json    TEXT NOT NULL
                );

                -- Fingerprint cache
                CREATE TABLE IF NOT EXISTS fingerprint_cache (
                    path            TEXT PRIMARY KEY,
                    fingerprint     TEXT NOT NULL
                );

                INSERT INTO schema_version (version) VALUES (2);",
            )
            .map_err(|e| format!("Migration v2 failed: {e}"))?;
        }

        if current < 3 {
            conn.execute_batch(
                "-- Composite indexes for common query patterns
                CREATE INDEX IF NOT EXISTS idx_samples_scan_format
                    ON audio_samples(scan_id, format);
                CREATE INDEX IF NOT EXISTS idx_samples_scan_name
                    ON audio_samples(scan_id, name COLLATE NOCASE);
                CREATE INDEX IF NOT EXISTS idx_daw_scan_format
                    ON daw_projects(scan_id, format);
                CREATE INDEX IF NOT EXISTS idx_presets_scan_format
                    ON presets(scan_id, format);
                INSERT INTO schema_version (version) VALUES (3);",
            )
            .map_err(|e| format!("Migration v3 failed: {e}"))?;
        }

        if current < 4 {
            conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS toast_i18n (
                    key TEXT NOT NULL,
                    locale TEXT NOT NULL,
                    value TEXT NOT NULL,
                    PRIMARY KEY (key, locale)
                );
                CREATE INDEX IF NOT EXISTS idx_toast_i18n_locale ON toast_i18n(locale);
                INSERT INTO schema_version (version) VALUES (4);",
            )
            .map_err(|e| format!("Migration v4 failed: {e}"))?;
        }

        if current < 5 {
            let has_toast: bool = conn
                .query_row(
                    "SELECT 1 FROM sqlite_master WHERE type='table' AND name='toast_i18n'",
                    [],
                    |_| Ok(()),
                )
                .is_ok();
            if has_toast {
                conn.execute_batch(
                    "ALTER TABLE toast_i18n RENAME TO app_i18n;
                     DROP INDEX IF EXISTS idx_toast_i18n_locale;
                     CREATE INDEX IF NOT EXISTS idx_app_i18n_locale ON app_i18n(locale);",
                )
                .map_err(|e| format!("Migration v5 failed: {e}"))?;
            }
            conn.execute("INSERT INTO schema_version (version) VALUES (5)", [])
                .map_err(|e| format!("Migration v5 schema_version failed: {e}"))?;
        }

        if current < 6 {
            conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS pdfs (
                    id              INTEGER PRIMARY KEY,
                    name            TEXT NOT NULL,
                    path            TEXT NOT NULL,
                    directory       TEXT NOT NULL,
                    size            INTEGER NOT NULL,
                    size_formatted  TEXT NOT NULL,
                    modified        TEXT NOT NULL,
                    scan_id         TEXT NOT NULL
                );
                CREATE UNIQUE INDEX IF NOT EXISTS idx_pdfs_path_scan ON pdfs(path, scan_id);
                CREATE INDEX IF NOT EXISTS idx_pdfs_name ON pdfs(name COLLATE NOCASE);
                CREATE INDEX IF NOT EXISTS idx_pdfs_scan_id ON pdfs(scan_id);

                CREATE TABLE IF NOT EXISTS pdf_scans (
                    id              TEXT PRIMARY KEY,
                    timestamp       TEXT NOT NULL,
                    pdf_count       INTEGER NOT NULL,
                    total_bytes     INTEGER NOT NULL,
                    roots           TEXT NOT NULL
                );",
            )
            .map_err(|e| format!("Migration v6 (PDF tables) failed: {e}"))?;
            conn.execute("INSERT INTO schema_version (version) VALUES (6)", [])
                .map_err(|e| format!("Migration v6 schema_version failed: {e}"))?;
        }

        if current < 7 {
            conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS pdf_metadata (
                    path        TEXT PRIMARY KEY,
                    pages       INTEGER,
                    updated_at  TEXT NOT NULL
                );",
            )
            .map_err(|e| format!("Migration v7 (pdf_metadata) failed: {e}"))?;
            conn.execute("INSERT INTO schema_version (version) VALUES (7)", [])
                .map_err(|e| format!("Migration v7 schema_version failed: {e}"))?;
        }

        if current < 8 {
            conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS midi_files (
                    id              INTEGER PRIMARY KEY,
                    name            TEXT NOT NULL,
                    path            TEXT NOT NULL,
                    directory       TEXT NOT NULL,
                    format          TEXT NOT NULL,
                    size            INTEGER NOT NULL,
                    size_formatted  TEXT NOT NULL,
                    modified        TEXT NOT NULL,
                    scan_id         TEXT NOT NULL
                );
                CREATE UNIQUE INDEX IF NOT EXISTS idx_midi_files_path_scan ON midi_files(path, scan_id);
                CREATE INDEX IF NOT EXISTS idx_midi_files_name ON midi_files(name COLLATE NOCASE);
                CREATE INDEX IF NOT EXISTS idx_midi_files_scan_id ON midi_files(scan_id);
                CREATE INDEX IF NOT EXISTS idx_midi_files_format ON midi_files(format);

                CREATE TABLE IF NOT EXISTS midi_scans (
                    id              TEXT PRIMARY KEY,
                    timestamp       TEXT NOT NULL,
                    midi_count      INTEGER NOT NULL,
                    total_bytes     INTEGER NOT NULL,
                    format_counts   TEXT NOT NULL,
                    roots           TEXT NOT NULL
                );",
            )
            .map_err(|e| format!("Migration v8 (MIDI tables) failed: {e}"))?;
            conn.execute("INSERT INTO schema_version (version) VALUES (8)", [])
                .map_err(|e| format!("Migration v8 schema_version failed: {e}"))?;
        }

        if current < 9 {
            // Composite sort indexes: turn ORDER BY + LIMIT into an index range
            // scan instead of a full sort, plus FTS5 virtual tables with the
            // trigram tokenizer for fast substring search at millions of rows.
            conn.execute_batch(
                "-- audio_samples composite sort indexes
                 CREATE INDEX IF NOT EXISTS idx_samples_scan_name     ON audio_samples(scan_id, name COLLATE NOCASE, id);
                 CREATE INDEX IF NOT EXISTS idx_samples_scan_size     ON audio_samples(scan_id, size, id);
                 CREATE INDEX IF NOT EXISTS idx_samples_scan_modified ON audio_samples(scan_id, modified, id);
                 CREATE INDEX IF NOT EXISTS idx_samples_scan_format   ON audio_samples(scan_id, format, id);
                 CREATE INDEX IF NOT EXISTS idx_samples_scan_duration ON audio_samples(scan_id, duration, id);

                 -- daw_projects composite sort indexes
                 CREATE INDEX IF NOT EXISTS idx_daw_scan_name     ON daw_projects(scan_id, name COLLATE NOCASE, id);
                 CREATE INDEX IF NOT EXISTS idx_daw_scan_size     ON daw_projects(scan_id, size, id);
                 CREATE INDEX IF NOT EXISTS idx_daw_scan_modified ON daw_projects(scan_id, modified, id);
                 CREATE INDEX IF NOT EXISTS idx_daw_scan_daw      ON daw_projects(scan_id, daw, id);
                 CREATE INDEX IF NOT EXISTS idx_daw_scan_format   ON daw_projects(scan_id, format, id);

                 -- presets composite sort indexes
                 CREATE INDEX IF NOT EXISTS idx_presets_scan_name     ON presets(scan_id, name COLLATE NOCASE, id);
                 CREATE INDEX IF NOT EXISTS idx_presets_scan_size     ON presets(scan_id, size, id);
                 CREATE INDEX IF NOT EXISTS idx_presets_scan_modified ON presets(scan_id, modified, id);
                 CREATE INDEX IF NOT EXISTS idx_presets_scan_format   ON presets(scan_id, format, id);

                 -- midi_files composite sort indexes
                 CREATE INDEX IF NOT EXISTS idx_midi_scan_name     ON midi_files(scan_id, name COLLATE NOCASE, id);
                 CREATE INDEX IF NOT EXISTS idx_midi_scan_size     ON midi_files(scan_id, size, id);
                 CREATE INDEX IF NOT EXISTS idx_midi_scan_modified ON midi_files(scan_id, modified, id);
                 CREATE INDEX IF NOT EXISTS idx_midi_scan_format   ON midi_files(scan_id, format, id);

                 -- pdfs composite sort indexes
                 CREATE INDEX IF NOT EXISTS idx_pdfs_scan_name     ON pdfs(scan_id, name COLLATE NOCASE, id);
                 CREATE INDEX IF NOT EXISTS idx_pdfs_scan_size     ON pdfs(scan_id, size, id);
                 CREATE INDEX IF NOT EXISTS idx_pdfs_scan_modified ON pdfs(scan_id, modified, id);

                 -- FTS5 virtual tables with trigram tokenizer (substring search, O(log n)).
                 -- Contentless w/ scan_id so we can DELETE per-scan without scanning the whole FTS.
                 CREATE VIRTUAL TABLE IF NOT EXISTS audio_samples_fts USING fts5(
                    name, path, scan_id UNINDEXED, tokenize='trigram'
                 );
                 CREATE VIRTUAL TABLE IF NOT EXISTS daw_projects_fts USING fts5(
                    name, path, daw, scan_id UNINDEXED, tokenize='trigram'
                 );
                 CREATE VIRTUAL TABLE IF NOT EXISTS presets_fts USING fts5(
                    name, path, format, scan_id UNINDEXED, tokenize='trigram'
                 );
                 CREATE VIRTUAL TABLE IF NOT EXISTS midi_files_fts USING fts5(
                    name, path, scan_id UNINDEXED, tokenize='trigram'
                 );
                 CREATE VIRTUAL TABLE IF NOT EXISTS pdfs_fts USING fts5(
                    name, path, scan_id UNINDEXED, tokenize='trigram'
                 );",
            )
            .map_err(|e| format!("Migration v9 (indexes + FTS5) failed: {e}"))?;
            conn.execute("INSERT INTO schema_version (version) VALUES (9)", [])
                .map_err(|e| format!("Migration v9 schema_version failed: {e}"))?;
        }

        if current < 10 {
            conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS directory_scan_state (
                    domain          TEXT NOT NULL,
                    path            TEXT NOT NULL,
                    mtime_secs      INTEGER NOT NULL,
                    last_scan_id    TEXT,
                    PRIMARY KEY (domain, path)
                );
                CREATE INDEX IF NOT EXISTS idx_directory_scan_state_domain
                    ON directory_scan_state(domain);",
            )
            .map_err(|e| format!("Migration v10 (directory_scan_state) failed: {e}"))?;
            conn.execute("INSERT INTO schema_version (version) VALUES (10)", [])
                .map_err(|e| format!("Migration v10 schema_version failed: {e}"))?;
        }

        if current < 11 {
            conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS unified_scan_run (
                    id INTEGER PRIMARY KEY CHECK (id = 1),
                    run_id            TEXT NOT NULL DEFAULT '',
                    started_at        TEXT NOT NULL DEFAULT '',
                    finished_at       TEXT,
                    outcome           TEXT NOT NULL DEFAULT 'complete',
                    audio_scan_id     TEXT,
                    daw_scan_id       TEXT,
                    preset_scan_id    TEXT,
                    pdf_scan_id       TEXT,
                    roots_json        TEXT NOT NULL DEFAULT '{}',
                    last_directory_path TEXT,
                    error_message     TEXT
                );
                INSERT OR IGNORE INTO unified_scan_run (id, outcome, roots_json)
                    VALUES (1, 'complete', '{}');",
            )
            .map_err(|e| format!("Migration v11 (unified_scan_run) failed: {e}"))?;
            conn.execute("INSERT INTO schema_version (version) VALUES (11)", [])
                .map_err(|e| format!("Migration v11 schema_version failed: {e}"))?;
        }

        if current < 12 {
            // `scan_complete`: streaming scans start at 0; lib sets 1 when the run finishes without stop.
            // History / latest queries filter to complete rows so partial runs are not deletable "junk"
            // that still backs library aggregates.
            conn.execute_batch(
                "ALTER TABLE audio_scans ADD COLUMN scan_complete INTEGER NOT NULL DEFAULT 1;
                 ALTER TABLE plugin_scans ADD COLUMN scan_complete INTEGER NOT NULL DEFAULT 1;
                 ALTER TABLE daw_scans ADD COLUMN scan_complete INTEGER NOT NULL DEFAULT 1;
                 ALTER TABLE preset_scans ADD COLUMN scan_complete INTEGER NOT NULL DEFAULT 1;
                 ALTER TABLE midi_scans ADD COLUMN scan_complete INTEGER NOT NULL DEFAULT 1;
                 ALTER TABLE pdf_scans ADD COLUMN scan_complete INTEGER NOT NULL DEFAULT 1;",
            )
            .map_err(|e| format!("Migration v12 (scan_complete) failed: {e}"))?;
            conn.execute("INSERT INTO schema_version (version) VALUES (12)", [])
                .map_err(|e| format!("Migration v12 schema_version failed: {e}"))?;
        }

        if current < 13 {
            // FTS5 tables from v9 were never populated for rows that existed before FTS or for
            // restored/copied DBs — substring search used `MATCH` on empty FTS and found nothing.
            backfill_contentless_fts(&conn)?;
            conn.execute("INSERT INTO schema_version (version) VALUES (13)", [])
                .map_err(|e| format!("Migration v13 schema_version failed: {e}"))?;
        }

        if current < 14 {
            // One canonical `audio_samples` row id per filesystem path (same semantics as
            // `MAX(id) GROUP BY path`), maintained on insert and after scan deletes.
            conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS audio_library (
                    path TEXT PRIMARY KEY NOT NULL,
                    sample_id INTEGER NOT NULL
                );
                CREATE INDEX IF NOT EXISTS idx_audio_library_sample_id ON audio_library(sample_id);
                INSERT OR REPLACE INTO audio_library (path, sample_id)
                SELECT path, MAX(id) AS sample_id FROM audio_samples GROUP BY path;",
            )
            .map_err(|e| format!("Migration v14 (audio_library) failed: {e}"))?;
            conn.execute("INSERT INTO schema_version (version) VALUES (14)", [])
                .map_err(|e| format!("Migration v14 schema_version failed: {e}"))?;
        }

        if current < 15 {
            // Materialized library tables for PDF / MIDI / presets — same semantics as
            // `MAX(id) GROUP BY path`, maintained on insert and path-affecting deletes.
            conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS pdf_library (
                    path TEXT PRIMARY KEY NOT NULL,
                    pdf_id INTEGER NOT NULL
                );
                CREATE INDEX IF NOT EXISTS idx_pdf_library_pdf_id ON pdf_library(pdf_id);
                INSERT OR REPLACE INTO pdf_library (path, pdf_id)
                SELECT path, MAX(id) FROM pdfs GROUP BY path;

                CREATE TABLE IF NOT EXISTS midi_library (
                    path TEXT PRIMARY KEY NOT NULL,
                    midi_id INTEGER NOT NULL
                );
                CREATE INDEX IF NOT EXISTS idx_midi_library_midi_id ON midi_library(midi_id);
                INSERT OR REPLACE INTO midi_library (path, midi_id)
                SELECT path, MAX(id) FROM midi_files GROUP BY path;

                CREATE TABLE IF NOT EXISTS preset_library (
                    path TEXT PRIMARY KEY NOT NULL,
                    preset_id INTEGER NOT NULL
                );
                CREATE INDEX IF NOT EXISTS idx_preset_library_preset_id ON preset_library(preset_id);
                INSERT OR REPLACE INTO preset_library (path, preset_id)
                SELECT path, MAX(id) FROM presets GROUP BY path;",
            )
            .map_err(|e| format!("Migration v15 (pdf/midi/preset library tables) failed: {e}"))?;
            conn.execute("INSERT INTO schema_version (version) VALUES (15)", [])
                .map_err(|e| format!("Migration v15 schema_version failed: {e}"))?;
        }

        if current < 16 {
            // One canonical `daw_projects` row id per filesystem path (same semantics as
            // `MAX(id) GROUP BY path`), maintained on insert and after scan deletes.
            conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS daw_library (
                    path TEXT PRIMARY KEY NOT NULL,
                    project_id INTEGER NOT NULL
                );
                CREATE INDEX IF NOT EXISTS idx_daw_library_project_id ON daw_library(project_id);
                INSERT OR REPLACE INTO daw_library (path, project_id)
                SELECT path, MAX(id) AS project_id FROM daw_projects GROUP BY path;",
            )
            .map_err(|e| format!("Migration v16 (daw_library) failed: {e}"))?;
            conn.execute("INSERT INTO schema_version (version) VALUES (16)", [])
                .map_err(|e| format!("Migration v16 schema_version failed: {e}"))?;
        }

        if current < 17 {
            // Firmlink path backfill + `plugin_scans` JSON (same `normalize_path_for_db` as inserts),
            // then materialize `plugin_library` like v14–v16 for audio/DAW/PDF/MIDI/presets.
            Self::migrate_plugin_paths_canonical(&conn)
                .map_err(|e| format!("Migration v17 (plugin path canonicalization) failed: {e}"))?;
            conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS plugin_library (
                    path TEXT PRIMARY KEY NOT NULL,
                    plugin_id INTEGER NOT NULL
                );
                CREATE INDEX IF NOT EXISTS idx_plugin_library_plugin_id ON plugin_library(plugin_id);
                INSERT OR REPLACE INTO plugin_library (path, plugin_id)
                SELECT path, MAX(id) AS plugin_id FROM plugins GROUP BY path;",
            )
            .map_err(|e| format!("Migration v17 (plugin_library) failed: {e}"))?;
            conn.execute("INSERT INTO schema_version (version) VALUES (17)", [])
                .map_err(|e| format!("Migration v17 schema_version failed: {e}"))?;
        }

        if current < 18 {
            // Stops background analysis from re-queuing the same files forever when BPM
            // detection returns None but key + LUFS succeed (`unanalyzed_paths` used only `bpm IS NULL`).
            // Fresh installs may already have `bpm_exhausted` from v1 `CREATE TABLE audio_samples`.
            let has_bpm_exhausted: bool = conn
                .query_row(
                    "SELECT COUNT(*) FROM pragma_table_info('audio_samples') WHERE name = 'bpm_exhausted'",
                    [],
                    |row| row.get::<_, i64>(0).map(|n| n > 0),
                )
                .unwrap_or(false);
            if !has_bpm_exhausted {
                conn.execute(
                    "ALTER TABLE audio_samples ADD COLUMN bpm_exhausted INTEGER NOT NULL DEFAULT 0",
                    [],
                )
                .map_err(|e| format!("Migration v18 (bpm_exhausted) failed: {e}"))?;
            }
            conn.execute("INSERT INTO schema_version (version) VALUES (18)", [])
                .map_err(|e| format!("Migration v18 schema_version failed: {e}"))?;
        }

        if current < 19 {
            let has_pdf_creation: bool = conn
                .query_row(
                    "SELECT COUNT(*) FROM pragma_table_info('pdf_metadata') WHERE name = 'pdf_creation_date'",
                    [],
                    |row| row.get::<_, i64>(0).map(|n| n > 0),
                )
                .unwrap_or(false);
            if !has_pdf_creation {
                conn.execute(
                    "ALTER TABLE pdf_metadata ADD COLUMN pdf_creation_date TEXT",
                    [],
                )
                .map_err(|e| format!("Migration v19 (pdf_creation_date) failed: {e}"))?;
            }
            let has_pdf_mod: bool = conn
                .query_row(
                    "SELECT COUNT(*) FROM pragma_table_info('pdf_metadata') WHERE name = 'pdf_mod_date'",
                    [],
                    |row| row.get::<_, i64>(0).map(|n| n > 0),
                )
                .unwrap_or(false);
            if !has_pdf_mod {
                conn.execute("ALTER TABLE pdf_metadata ADD COLUMN pdf_mod_date TEXT", [],)
                    .map_err(|e| format!("Migration v19 (pdf_mod_date) failed: {e}"))?;
            }
            conn.execute("INSERT INTO schema_version (version) VALUES (19)", [])
                .map_err(|e| format!("Migration v19 schema_version failed: {e}"))?;
        }

        if current < 20 {
            conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS video_files (
                    id              INTEGER PRIMARY KEY,
                    name            TEXT NOT NULL,
                    path            TEXT NOT NULL,
                    directory       TEXT NOT NULL,
                    format          TEXT NOT NULL,
                    size            INTEGER NOT NULL,
                    size_formatted  TEXT NOT NULL,
                    modified        TEXT NOT NULL,
                    scan_id         TEXT NOT NULL
                );
                CREATE UNIQUE INDEX IF NOT EXISTS idx_video_files_path_scan ON video_files(path, scan_id);
                CREATE INDEX IF NOT EXISTS idx_video_files_name ON video_files(name COLLATE NOCASE);
                CREATE INDEX IF NOT EXISTS idx_video_files_scan_id ON video_files(scan_id);
                CREATE INDEX IF NOT EXISTS idx_video_files_format ON video_files(format);
                CREATE INDEX IF NOT EXISTS idx_video_scan_name     ON video_files(scan_id, name COLLATE NOCASE, id);
                CREATE INDEX IF NOT EXISTS idx_video_scan_size     ON video_files(scan_id, size, id);
                CREATE INDEX IF NOT EXISTS idx_video_scan_modified ON video_files(scan_id, modified, id);
                CREATE INDEX IF NOT EXISTS idx_video_scan_format   ON video_files(scan_id, format, id);

                CREATE TABLE IF NOT EXISTS video_scans (
                    id              TEXT PRIMARY KEY,
                    timestamp       TEXT NOT NULL,
                    video_count     INTEGER NOT NULL,
                    total_bytes     INTEGER NOT NULL,
                    format_counts   TEXT NOT NULL,
                    roots           TEXT NOT NULL,
                    scan_complete   INTEGER NOT NULL DEFAULT 1
                );

                CREATE VIRTUAL TABLE IF NOT EXISTS video_files_fts USING fts5(
                    name, path, scan_id UNINDEXED, tokenize='trigram'
                );

                CREATE TABLE IF NOT EXISTS video_library (
                    path TEXT PRIMARY KEY NOT NULL,
                    video_id INTEGER NOT NULL
                );
                CREATE INDEX IF NOT EXISTS idx_video_library_video_id ON video_library(video_id);
                INSERT OR REPLACE INTO video_library (path, video_id)
                SELECT path, MAX(id) FROM video_files GROUP BY path;",
            )
            .map_err(|e| format!("Migration v20 (video inventory) failed: {e}"))?;
            conn.execute("INSERT INTO schema_version (version) VALUES (20)", [])
                .map_err(|e| format!("Migration v20 schema_version failed: {e}"))?;
        }

        if current < 21 {
            conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS favorites (
                    id         INTEGER PRIMARY KEY,
                    type       TEXT NOT NULL,
                    path       TEXT NOT NULL UNIQUE,
                    name       TEXT NOT NULL,
                    format     TEXT NOT NULL DEFAULT '',
                    daw        TEXT NOT NULL DEFAULT '',
                    added_at   TEXT NOT NULL,
                    sort_order INTEGER NOT NULL DEFAULT 0
                );
                CREATE INDEX IF NOT EXISTS idx_favorites_path ON favorites(path);
                CREATE INDEX IF NOT EXISTS idx_favorites_sort ON favorites(sort_order);

                CREATE TABLE IF NOT EXISTS item_notes (
                    path       TEXT PRIMARY KEY NOT NULL,
                    note       TEXT NOT NULL DEFAULT '',
                    updated_at TEXT NOT NULL
                );

                CREATE TABLE IF NOT EXISTS item_tags (
                    path TEXT NOT NULL,
                    tag  TEXT NOT NULL,
                    PRIMARY KEY (path, tag)
                );
                CREATE INDEX IF NOT EXISTS idx_item_tags_tag ON item_tags(tag);

                CREATE TABLE IF NOT EXISTS standalone_tags (
                    tag TEXT PRIMARY KEY NOT NULL
                );",
            )
            .map_err(|e| format!("Migration v21 (favorites/notes/tags) failed: {e}"))?;
            conn.execute("INSERT INTO schema_version (version) VALUES (21)", [])
                .map_err(|e| format!("Migration v21 schema_version failed: {e}"))?;
        }

        if current < 22 {
            conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS player_history (
                    id         INTEGER PRIMARY KEY,
                    path       TEXT NOT NULL UNIQUE,
                    name       TEXT NOT NULL,
                    format     TEXT NOT NULL DEFAULT '',
                    size       TEXT NOT NULL DEFAULT '',
                    sort_order INTEGER NOT NULL DEFAULT 0
                );
                CREATE INDEX IF NOT EXISTS idx_player_history_sort ON player_history(sort_order);",
            )
            .map_err(|e| format!("Migration v22 (player_history) failed: {e}"))?;
            conn.execute("INSERT INTO schema_version (version) VALUES (22)", [])
                .map_err(|e| format!("Migration v22 schema_version failed: {e}"))?;
        }

        // v23: Add missing indexes for user data tables (favorites, player_history)
        // to handle thousands of items efficiently.
        if current < 23 {
            conn.execute_batch(
                "-- player_history: explicit path index (UNIQUE creates internal, but explicit is clearer)
                 CREATE INDEX IF NOT EXISTS idx_player_history_path ON player_history(path);
                 CREATE INDEX IF NOT EXISTS idx_player_history_name ON player_history(name COLLATE NOCASE);

                 -- favorites: add name and type indexes for search/filter
                 CREATE INDEX IF NOT EXISTS idx_favorites_name ON favorites(name COLLATE NOCASE);
                 CREATE INDEX IF NOT EXISTS idx_favorites_type ON favorites(type);

                 -- item_tags: add path index for fast tag lookups by item
                 CREATE INDEX IF NOT EXISTS idx_item_tags_path ON item_tags(path);",
            )
            .map_err(|e| format!("Migration v23 (user data indexes) failed: {e}"))?;
            conn.execute("INSERT INTO schema_version (version) VALUES (23)", [])
                .map_err(|e| format!("Migration v23 schema_version failed: {e}"))?;
        }

        if current < 24 {
            conn.execute_batch(
                "-- =======================================================================
                 -- ALS Generator: sample analysis lookup / cache tables
                 -- =======================================================================

                 -- Manufacturers / labels with genre & hardness scores
                 CREATE TABLE IF NOT EXISTS sample_pack_manufacturers (
                     id              INTEGER PRIMARY KEY AUTOINCREMENT,
                     name            TEXT UNIQUE NOT NULL,
                     genre_score     REAL NOT NULL DEFAULT 0.0,
                     hardness_score  REAL NOT NULL DEFAULT 0.0,
                     website         TEXT,
                     description     TEXT,
                     created_at      TEXT NOT NULL DEFAULT (datetime('now'))
                 );
                 CREATE INDEX IF NOT EXISTS idx_spm_name     ON sample_pack_manufacturers(name);
                 CREATE INDEX IF NOT EXISTS idx_spm_genre    ON sample_pack_manufacturers(genre_score);
                 CREATE INDEX IF NOT EXISTS idx_spm_hardness ON sample_pack_manufacturers(hardness_score);

                 -- Individual sample packs (FK to manufacturer)
                 CREATE TABLE IF NOT EXISTS sample_packs (
                     id              INTEGER PRIMARY KEY AUTOINCREMENT,
                     name            TEXT UNIQUE NOT NULL,
                     manufacturer_id INTEGER REFERENCES sample_pack_manufacturers(id),
                     genre_score     REAL,
                     hardness_score  REAL,
                     default_bpm     INTEGER,
                     website         TEXT,
                     description     TEXT,
                     created_at      TEXT NOT NULL DEFAULT (datetime('now'))
                 );
                 CREATE INDEX IF NOT EXISTS idx_sp_name ON sample_packs(name);
                 CREATE INDEX IF NOT EXISTS idx_sp_mfr  ON sample_packs(manufacturer_id);

                 -- Hierarchical sample categories with regex patterns
                 CREATE TABLE IF NOT EXISTS sample_categories (
                     id                INTEGER PRIMARY KEY AUTOINCREMENT,
                     name              TEXT UNIQUE NOT NULL,
                     parent_name       TEXT,
                     is_oneshot        INTEGER NOT NULL DEFAULT 0,
                     is_key_sensitive  INTEGER NOT NULL DEFAULT 0,
                     is_loop_preferred INTEGER NOT NULL DEFAULT 0,
                     pattern           TEXT NOT NULL
                 );
                 CREATE INDEX IF NOT EXISTS idx_sc_name ON sample_categories(name);

                 -- Per-sample parsed analysis (FK to audio_samples.id)
                 CREATE TABLE IF NOT EXISTS sample_analysis (
                     sample_id           INTEGER PRIMARY KEY REFERENCES audio_samples(id),
                     parsed_bpm          INTEGER,
                     parsed_key          TEXT,
                     category_id         INTEGER REFERENCES sample_categories(id),
                     pack_id             INTEGER REFERENCES sample_packs(id),
                     manufacturer_id     INTEGER REFERENCES sample_pack_manufacturers(id),
                     category_confidence REAL,
                     is_loop             INTEGER NOT NULL DEFAULT 0,
                     analyzed_at         TEXT NOT NULL DEFAULT (datetime('now'))
                 );
                 CREATE INDEX IF NOT EXISTS idx_sa_category     ON sample_analysis(category_id);
                 CREATE INDEX IF NOT EXISTS idx_sa_bpm          ON sample_analysis(parsed_bpm);
                 CREATE INDEX IF NOT EXISTS idx_sa_key          ON sample_analysis(parsed_key);
                 CREATE INDEX IF NOT EXISTS idx_sa_pack         ON sample_analysis(pack_id);
                 CREATE INDEX IF NOT EXISTS idx_sa_mfr          ON sample_analysis(manufacturer_id);
                 CREATE INDEX IF NOT EXISTS idx_sa_loop         ON sample_analysis(is_loop);
                 CREATE INDEX IF NOT EXISTS idx_sa_cat_bpm      ON sample_analysis(category_id, parsed_bpm);
                 CREATE INDEX IF NOT EXISTS idx_sa_cat_key      ON sample_analysis(category_id, parsed_key);
                 CREATE INDEX IF NOT EXISTS idx_sa_cat_bpm_key  ON sample_analysis(category_id, parsed_bpm, parsed_key);
                 ",
            )
            .map_err(|e| format!("Migration v24 (sample analysis tables) failed: {e}"))?;
            conn.execute("INSERT INTO schema_version (version) VALUES (24)", [])
                .map_err(|e| format!("Migration v24 schema_version failed: {e}"))?;
        }

        if conn
            .query_row(
                "SELECT 1 FROM sqlite_master WHERE type='table' AND name='app_i18n'",
                [],
                |_| Ok(()),
            )
            .is_ok()
        {
            crate::app_i18n::seed_defaults(&conn)?;
        }

        Ok(())
    }

    // ── Favorites ──

    pub fn favorites_list(&self) -> Result<Vec<serde_json::Value>, String> {
        let conn = self.read_conn();
        let mut stmt = conn
            .prepare("SELECT id, type, path, name, format, daw, added_at, sort_order FROM favorites ORDER BY sort_order ASC, id ASC")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| {
                Ok(serde_json::json!({
                    "type": row.get::<_, String>(1)?,
                    "path": row.get::<_, String>(2)?,
                    "name": row.get::<_, String>(3)?,
                    "format": row.get::<_, String>(4)?,
                    "daw": row.get::<_, String>(5)?,
                    "addedAt": row.get::<_, String>(6)?,
                }))
            })
            .map_err(|e| e.to_string())?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r.map_err(|e| e.to_string())?);
        }
        Ok(out)
    }

    pub fn favorites_add(
        &self,
        fav_type: &str,
        path: &str,
        name: &str,
        format: &str,
        daw: &str,
        added_at: &str,
    ) -> Result<bool, String> {
        let conn = self.write_conn();
        let min_order: i64 = conn
            .query_row("SELECT COALESCE(MIN(sort_order), 0) FROM favorites", [], |r| r.get(0))
            .unwrap_or(0);
        let res = conn.execute(
            "INSERT OR IGNORE INTO favorites (type, path, name, format, daw, added_at, sort_order) VALUES (?1,?2,?3,?4,?5,?6,?7)",
            params![fav_type, path, name, format, daw, added_at, min_order - 1],
        ).map_err(|e| e.to_string())?;
        Ok(res > 0)
    }

    pub fn favorites_remove(&self, path: &str) -> Result<bool, String> {
        let conn = self.write_conn();
        let n = conn
            .execute("DELETE FROM favorites WHERE path = ?1", params![path])
            .map_err(|e| e.to_string())?;
        Ok(n > 0)
    }

    pub fn favorites_clear(&self) -> Result<(), String> {
        let conn = self.write_conn();
        conn.execute("DELETE FROM favorites", [])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn favorites_is(&self, path: &str) -> Result<bool, String> {
        let conn = self.read_conn();
        let exists: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM favorites WHERE path = ?1)",
                params![path],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        Ok(exists)
    }

    /// Bulk-replace all favorites (for import).
    pub fn favorites_set_all(&self, favs: &[serde_json::Value]) -> Result<(), String> {
        let conn = self.write_conn();
        conn.execute("DELETE FROM favorites", [])
            .map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare("INSERT INTO favorites (type, path, name, format, daw, added_at, sort_order) VALUES (?1,?2,?3,?4,?5,?6,?7)")
            .map_err(|e| e.to_string())?;
        for (i, f) in favs.iter().enumerate() {
            let fav_type = f.get("type").and_then(|v| v.as_str()).unwrap_or("sample");
            let path = f.get("path").and_then(|v| v.as_str()).unwrap_or("");
            let name = f.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let format = f.get("format").and_then(|v| v.as_str()).unwrap_or("");
            let daw = f.get("daw").and_then(|v| v.as_str()).unwrap_or("");
            let added_at = f.get("addedAt").and_then(|v| v.as_str()).unwrap_or("");
            if path.is_empty() {
                continue;
            }
            stmt.execute(params![fav_type, path, name, format, daw, added_at, i as i64])
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    // ── Player History ──

    pub fn player_history_list(&self) -> Result<Vec<serde_json::Value>, String> {
        let conn = self.read_conn();
        let mut stmt = conn
            .prepare("SELECT path, name, format, size FROM player_history ORDER BY sort_order ASC")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| {
                Ok(serde_json::json!({
                    "path": row.get::<_, String>(0)?,
                    "name": row.get::<_, String>(1)?,
                    "format": row.get::<_, String>(2)?,
                    "size": row.get::<_, String>(3)?,
                }))
            })
            .map_err(|e| e.to_string())?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r.map_err(|e| e.to_string())?);
        }
        Ok(out)
    }

    /// Add or update an entry in player history. If `skip_reorder` is false, moves entry to top.
    pub fn player_history_add(
        &self,
        path: &str,
        name: &str,
        format: &str,
        size: &str,
        skip_reorder: bool,
    ) -> Result<(), String> {
        let conn = self.write_conn();
        if skip_reorder {
            // Update in place or append at end
            let exists: bool = conn
                .query_row(
                    "SELECT 1 FROM player_history WHERE path = ?1",
                    params![path],
                    |_| Ok(true),
                )
                .unwrap_or(false);
            if exists {
                conn.execute(
                    "UPDATE player_history SET name = ?2, format = ?3, size = ?4 WHERE path = ?1",
                    params![path, name, format, size],
                )
                .map_err(|e| e.to_string())?;
            } else {
                let max_order: i64 = conn
                    .query_row("SELECT COALESCE(MAX(sort_order), 0) FROM player_history", [], |r| r.get(0))
                    .unwrap_or(0);
                conn.execute(
                    "INSERT INTO player_history (path, name, format, size, sort_order) VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![path, name, format, size, max_order + 1],
                )
                .map_err(|e| e.to_string())?;
                // Enforce max 50 entries
                conn.execute(
                    "DELETE FROM player_history WHERE id IN (SELECT id FROM player_history ORDER BY sort_order DESC LIMIT -1 OFFSET 50)",
                    [],
                )
                .map_err(|e| e.to_string())?;
            }
        } else {
            // Move to top (lowest sort_order)
            conn.execute("DELETE FROM player_history WHERE path = ?1", params![path])
                .map_err(|e| e.to_string())?;
            let min_order: i64 = conn
                .query_row("SELECT COALESCE(MIN(sort_order), 0) FROM player_history", [], |r| r.get(0))
                .unwrap_or(0);
            conn.execute(
                "INSERT INTO player_history (path, name, format, size, sort_order) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![path, name, format, size, min_order - 1],
            )
            .map_err(|e| e.to_string())?;
            // Enforce max 50 entries
            conn.execute(
                "DELETE FROM player_history WHERE id IN (SELECT id FROM player_history ORDER BY sort_order DESC LIMIT -1 OFFSET 50)",
                [],
            )
            .map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    pub fn player_history_remove(&self, path: &str) -> Result<bool, String> {
        let conn = self.write_conn();
        let n = conn
            .execute("DELETE FROM player_history WHERE path = ?1", params![path])
            .map_err(|e| e.to_string())?;
        Ok(n > 0)
    }

    pub fn player_history_clear(&self) -> Result<(), String> {
        let conn = self.write_conn();
        conn.execute("DELETE FROM player_history", [])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Bulk reorder: receives paths in desired order, updates sort_order accordingly.
    pub fn player_history_reorder(&self, paths: &[String]) -> Result<(), String> {
        let conn = self.write_conn();
        let mut stmt = conn
            .prepare("UPDATE player_history SET sort_order = ?2 WHERE path = ?1")
            .map_err(|e| e.to_string())?;
        for (i, path) in paths.iter().enumerate() {
            stmt.execute(params![path, i as i64])
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    /// Import from prefs JSON array (one-time migration).
    pub fn player_history_import(&self, items: &[serde_json::Value]) -> Result<usize, String> {
        let conn = self.write_conn();
        let mut stmt = conn
            .prepare("INSERT OR IGNORE INTO player_history (path, name, format, size, sort_order) VALUES (?1, ?2, ?3, ?4, ?5)")
            .map_err(|e| e.to_string())?;
        let mut count = 0usize;
        for (i, item) in items.iter().enumerate() {
            let path = item.get("path").and_then(|v| v.as_str()).unwrap_or("");
            let name = item.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let format = item.get("format").and_then(|v| v.as_str()).unwrap_or("");
            let size = item.get("size").and_then(|v| v.as_str()).unwrap_or("");
            if path.is_empty() {
                continue;
            }
            if stmt.execute(params![path, name, format, size, i as i64]).is_ok() {
                count += 1;
            }
        }
        Ok(count)
    }

    /// Replace entire player history (for smart playlists).
    pub fn player_history_set_all(&self, items: &[serde_json::Value]) -> Result<(), String> {
        let conn = self.write_conn();
        conn.execute("DELETE FROM player_history", [])
            .map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare("INSERT INTO player_history (path, name, format, size, sort_order) VALUES (?1, ?2, ?3, ?4, ?5)")
            .map_err(|e| e.to_string())?;
        for (i, item) in items.iter().enumerate() {
            let path = item.get("path").and_then(|v| v.as_str()).unwrap_or("");
            let name = item.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let format = item.get("format").and_then(|v| v.as_str()).unwrap_or("");
            let size = item.get("size").and_then(|v| v.as_str()).unwrap_or("");
            if path.is_empty() {
                continue;
            }
            stmt.execute(params![path, name, format, size, i as i64])
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    // ── Notes ──

    pub fn note_get(&self, path: &str) -> Result<Option<serde_json::Value>, String> {
        let conn = self.read_conn();
        let note: Option<(String, String)> = conn
            .query_row(
                "SELECT note, updated_at FROM item_notes WHERE path = ?1",
                params![path],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()
            .map_err(|e| e.to_string())?;
        let Some((note_text, updated_at)) = note else {
            return Ok(None);
        };
        let mut tag_stmt = conn
            .prepare("SELECT tag FROM item_tags WHERE path = ?1 ORDER BY tag")
            .map_err(|e| e.to_string())?;
        let tags: Vec<String> = tag_stmt
            .query_map(params![path], |r| r.get(0))
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();
        Ok(Some(serde_json::json!({
            "note": note_text,
            "tags": tags,
            "updatedAt": updated_at,
        })))
    }

    pub fn note_set(&self, path: &str, note: &str, tags: &[String]) -> Result<(), String> {
        let conn = self.write_conn();
        let is_empty = note.trim().is_empty() && tags.is_empty();
        // If note is only whitespace but tags exist, keep the note text verbatim.
        if is_empty && note.is_empty() {
            conn.execute("DELETE FROM item_notes WHERE path = ?1", params![path])
                .map_err(|e| e.to_string())?;
            conn.execute("DELETE FROM item_tags WHERE path = ?1", params![path])
                .map_err(|e| e.to_string())?;
            return Ok(());
        }
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO item_notes (path, note, updated_at) VALUES (?1,?2,?3)
             ON CONFLICT(path) DO UPDATE SET note=excluded.note, updated_at=excluded.updated_at",
            params![path, note, now],
        )
        .map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM item_tags WHERE path = ?1", params![path])
            .map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare("INSERT OR IGNORE INTO item_tags (path, tag) VALUES (?1,?2)")
            .map_err(|e| e.to_string())?;
        for t in tags {
            stmt.execute(params![path, t]).map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    pub fn notes_get_all(&self) -> Result<serde_json::Value, String> {
        let conn = self.read_conn();
        let mut stmt = conn
            .prepare("SELECT path, note, updated_at FROM item_notes")
            .map_err(|e| e.to_string())?;
        let mut map = serde_json::Map::new();
        let rows: Vec<(String, String, String)> = stmt
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();
        for (path, note, updated_at) in &rows {
            map.insert(
                path.clone(),
                serde_json::json!({"note": note, "tags": Vec::<String>::new(), "updatedAt": updated_at}),
            );
        }
        // Fill in tags
        let mut tag_stmt = conn
            .prepare("SELECT path, tag FROM item_tags ORDER BY path, tag")
            .map_err(|e| e.to_string())?;
        let tag_rows: Vec<(String, String)> = tag_stmt
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();
        for (path, tag) in tag_rows {
            if let Some(entry) = map.get_mut(&path) {
                if let Some(tags) = entry.get_mut("tags").and_then(|v| v.as_array_mut()) {
                    tags.push(serde_json::Value::String(tag));
                }
            } else {
                // Tags for paths without a note row: create a stub entry
                map.insert(
                    path.clone(),
                    serde_json::json!({"note": "", "tags": [tag], "updatedAt": ""}),
                );
            }
        }
        Ok(serde_json::Value::Object(map))
    }

    // ── Tags ──

    pub fn tags_standalone_list(&self) -> Result<Vec<String>, String> {
        let conn = self.read_conn();
        let mut stmt = conn
            .prepare("SELECT tag FROM standalone_tags ORDER BY tag")
            .map_err(|e| e.to_string())?;
        let tags: Vec<String> = stmt
            .query_map([], |r| r.get(0))
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();
        Ok(tags)
    }

    pub fn tags_standalone_set(&self, tags: &[String]) -> Result<(), String> {
        let conn = self.write_conn();
        conn.execute("DELETE FROM standalone_tags", [])
            .map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare("INSERT OR IGNORE INTO standalone_tags (tag) VALUES (?1)")
            .map_err(|e| e.to_string())?;
        for t in tags {
            stmt.execute(params![t]).map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    pub fn tags_standalone_add(&self, tag: &str) -> Result<(), String> {
        let conn = self.write_conn();
        conn.execute(
            "INSERT OR IGNORE INTO standalone_tags (tag) VALUES (?1)",
            params![tag],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn tags_standalone_remove(&self, tag: &str) -> Result<(), String> {
        let conn = self.write_conn();
        conn.execute("DELETE FROM standalone_tags WHERE tag = ?1", params![tag])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn tags_all(&self) -> Result<Vec<String>, String> {
        let conn = self.read_conn();
        let mut stmt = conn
            .prepare(
                "SELECT tag FROM standalone_tags
                 UNION
                 SELECT tag FROM item_tags
                 ORDER BY tag",
            )
            .map_err(|e| e.to_string())?;
        let tags: Vec<String> = stmt
            .query_map([], |r| r.get(0))
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();
        Ok(tags)
    }

    pub fn tags_counts(&self) -> Result<serde_json::Value, String> {
        let conn = self.read_conn();
        let mut map = serde_json::Map::new();
        // Standalone tags seed at 0
        let mut stmt = conn
            .prepare("SELECT tag FROM standalone_tags")
            .map_err(|e| e.to_string())?;
        let standalone: Vec<String> = stmt
            .query_map([], |r| r.get(0))
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();
        for t in standalone {
            map.insert(t, serde_json::json!(0));
        }
        // Item tag counts
        let mut stmt2 = conn
            .prepare("SELECT tag, COUNT(*) FROM item_tags GROUP BY tag")
            .map_err(|e| e.to_string())?;
        let counts: Vec<(String, i64)> = stmt2
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();
        for (tag, count) in counts {
            map.insert(tag, serde_json::json!(count));
        }
        Ok(serde_json::Value::Object(map))
    }

    pub fn tags_items_with(&self, tag: &str) -> Result<Vec<serde_json::Value>, String> {
        let conn = self.read_conn();
        let mut stmt = conn
            .prepare(
                "SELECT it.path, COALESCE(n.note,'') AS note, COALESCE(n.updated_at,'') AS updated_at
                 FROM item_tags it
                 LEFT JOIN item_notes n ON n.path = it.path
                 WHERE it.tag = ?1",
            )
            .map_err(|e| e.to_string())?;
        let rows: Vec<serde_json::Value> = stmt
            .query_map(params![tag], |r| {
                Ok(serde_json::json!({
                    "path": r.get::<_, String>(0)?,
                    "note": r.get::<_, String>(1)?,
                    "updatedAt": r.get::<_, String>(2)?,
                }))
            })
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();
        Ok(rows)
    }

    pub fn tag_rename(&self, old_tag: &str, new_tag: &str) -> Result<i64, String> {
        let conn = self.write_conn();
        // Delete item_tags rows where the new_tag already exists for the same path (dedup)
        conn.execute(
            "DELETE FROM item_tags WHERE tag = ?1 AND path IN (SELECT path FROM item_tags WHERE tag = ?2)",
            params![old_tag, new_tag],
        )
        .map_err(|e| e.to_string())?;
        let changed = conn
            .execute(
                "UPDATE item_tags SET tag = ?2 WHERE tag = ?1",
                params![old_tag, new_tag],
            )
            .map_err(|e| e.to_string())? as i64;
        // Rename in standalone
        conn.execute(
            "DELETE FROM standalone_tags WHERE tag = ?1",
            params![old_tag],
        )
        .map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT OR IGNORE INTO standalone_tags (tag) VALUES (?1)",
            params![new_tag],
        )
        .map_err(|e| e.to_string())?;
        Ok(changed)
    }

    pub fn tag_delete(&self, tag: &str) -> Result<i64, String> {
        let conn = self.write_conn();
        let changed = conn
            .execute("DELETE FROM item_tags WHERE tag = ?1", params![tag])
            .map_err(|e| e.to_string())? as i64;
        conn.execute("DELETE FROM standalone_tags WHERE tag = ?1", params![tag])
            .map_err(|e| e.to_string())?;
        Ok(changed)
    }

    pub fn tag_add_to_item(&self, path: &str, tag: &str) -> Result<bool, String> {
        let conn = self.write_conn();
        // Ensure item_notes row exists
        conn.execute(
            "INSERT OR IGNORE INTO item_notes (path, note, updated_at) VALUES (?1, '', ?2)",
            params![path, chrono::Utc::now().to_rfc3339()],
        )
        .map_err(|e| e.to_string())?;
        let n = conn
            .execute(
                "INSERT OR IGNORE INTO item_tags (path, tag) VALUES (?1,?2)",
                params![path, tag],
            )
            .map_err(|e| e.to_string())?;
        Ok(n > 0)
    }

    pub fn tag_remove_from_item(&self, path: &str, tag: &str) -> Result<(), String> {
        let conn = self.write_conn();
        conn.execute(
            "DELETE FROM item_tags WHERE path = ?1 AND tag = ?2",
            params![path, tag],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn tag_has(&self, path: &str, tag: &str) -> Result<bool, String> {
        let conn = self.read_conn();
        let exists: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM item_tags WHERE path = ?1 AND tag = ?2)",
                params![path, tag],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        Ok(exists)
    }

    /// Insert a batch of audio samples in a single transaction.
    pub fn audio_scan_parent_create(
        &self,
        id: &str,
        timestamp: &str,
        roots: &[String],
    ) -> Result<(), String> {
        let conn = self.read_conn();
        let roots_json = path_strings_json_normalized(roots);
        conn.execute(
            "INSERT OR REPLACE INTO audio_scans (id, timestamp, sample_count, total_bytes, format_counts, roots, scan_complete) VALUES (?1,?2,0,0,'{}',?3,0)",
            params![id, timestamp, roots_json],
        ).map_err(|e| e.to_string())?;
        conn.execute(
            "CREATE TEMP TABLE _al_refresh_paths (path TEXT PRIMARY KEY)",
            [],
        )
        .map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT INTO _al_refresh_paths SELECT DISTINCT path FROM audio_samples WHERE scan_id = ?1",
            params![id],
        )
        .map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM audio_samples WHERE scan_id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        conn.execute(
            "DELETE FROM audio_samples_fts WHERE scan_id = ?1",
            params![id],
        )
        .map_err(|e| e.to_string())?;
        Self::sync_audio_library_after_paths_refresh(&conn)?;
        self.invalidate_audio_library_total_cache();
        Ok(())
    }

    /// After a file or bundle was removed on disk: delete matching rows from all inventory tables
    /// and reconcile `*_library` materializations (same pattern as scan-based path refresh).
    pub fn purge_inventory_path(&self, path: &str) -> Result<(), String> {
        let mut conn = self.write_conn();
        let tx = conn.transaction().map_err(|e| e.to_string())?;

        // ── Audio ──
        tx.execute(
            "CREATE TEMP TABLE _al_refresh_paths (path TEXT PRIMARY KEY)",
            [],
        )
        .map_err(|e| e.to_string())?;
        tx.execute(
            "INSERT OR REPLACE INTO _al_refresh_paths VALUES (?1)",
            params![path],
        )
        .map_err(|e| e.to_string())?;
        tx.execute(
            "DELETE FROM audio_samples_fts WHERE rowid IN (SELECT id FROM audio_samples WHERE path IN (SELECT path FROM _al_refresh_paths))",
            [],
        )
        .map_err(|e| e.to_string())?;
        tx.execute(
            "DELETE FROM audio_samples WHERE path IN (SELECT path FROM _al_refresh_paths)",
            [],
        )
        .map_err(|e| e.to_string())?;
        Self::exec_sync_paths_refresh_tx(&tx, Self::SYNC_AUDIO_LIBRARY_PATHS_SQL)?;

        // ── DAW ──
        tx.execute(
            "CREATE TEMP TABLE _dl_refresh_paths (path TEXT PRIMARY KEY)",
            [],
        )
        .map_err(|e| e.to_string())?;
        tx.execute(
            "INSERT OR REPLACE INTO _dl_refresh_paths VALUES (?1)",
            params![path],
        )
        .map_err(|e| e.to_string())?;
        tx.execute(
            "DELETE FROM daw_projects_fts WHERE rowid IN (SELECT id FROM daw_projects WHERE path IN (SELECT path FROM _dl_refresh_paths))",
            [],
        )
        .map_err(|e| e.to_string())?;
        tx.execute(
            "DELETE FROM daw_projects WHERE path IN (SELECT path FROM _dl_refresh_paths)",
            [],
        )
        .map_err(|e| e.to_string())?;
        Self::exec_sync_paths_refresh_tx(&tx, Self::SYNC_DAW_LIBRARY_PATHS_SQL)?;

        // ── Presets ──
        tx.execute(
            "CREATE TEMP TABLE _preset_lib_refresh_paths (path TEXT PRIMARY KEY)",
            [],
        )
        .map_err(|e| e.to_string())?;
        tx.execute(
            "INSERT OR REPLACE INTO _preset_lib_refresh_paths VALUES (?1)",
            params![path],
        )
        .map_err(|e| e.to_string())?;
        tx.execute(
            "DELETE FROM presets_fts WHERE rowid IN (SELECT id FROM presets WHERE path IN (SELECT path FROM _preset_lib_refresh_paths))",
            [],
        )
        .map_err(|e| e.to_string())?;
        tx.execute(
            "DELETE FROM presets WHERE path IN (SELECT path FROM _preset_lib_refresh_paths)",
            [],
        )
        .map_err(|e| e.to_string())?;
        Self::exec_sync_paths_refresh_tx(&tx, Self::SYNC_PRESET_LIBRARY_PATHS_SQL)?;

        // ── MIDI ──
        tx.execute(
            "CREATE TEMP TABLE _midi_lib_refresh_paths (path TEXT PRIMARY KEY)",
            [],
        )
        .map_err(|e| e.to_string())?;
        tx.execute(
            "INSERT OR REPLACE INTO _midi_lib_refresh_paths VALUES (?1)",
            params![path],
        )
        .map_err(|e| e.to_string())?;
        tx.execute(
            "DELETE FROM midi_files_fts WHERE rowid IN (SELECT id FROM midi_files WHERE path IN (SELECT path FROM _midi_lib_refresh_paths))",
            [],
        )
        .map_err(|e| e.to_string())?;
        tx.execute(
            "DELETE FROM midi_files WHERE path IN (SELECT path FROM _midi_lib_refresh_paths)",
            [],
        )
        .map_err(|e| e.to_string())?;
        Self::exec_sync_paths_refresh_tx(&tx, Self::SYNC_MIDI_LIBRARY_PATHS_SQL)?;

        // ── Video ──
        tx.execute(
            "CREATE TEMP TABLE _video_lib_refresh_paths (path TEXT PRIMARY KEY)",
            [],
        )
        .map_err(|e| e.to_string())?;
        tx.execute(
            "INSERT OR REPLACE INTO _video_lib_refresh_paths VALUES (?1)",
            params![path],
        )
        .map_err(|e| e.to_string())?;
        tx.execute(
            "DELETE FROM video_files_fts WHERE rowid IN (SELECT id FROM video_files WHERE path IN (SELECT path FROM _video_lib_refresh_paths))",
            [],
        )
        .map_err(|e| e.to_string())?;
        tx.execute(
            "DELETE FROM video_files WHERE path IN (SELECT path FROM _video_lib_refresh_paths)",
            [],
        )
        .map_err(|e| e.to_string())?;
        Self::exec_sync_paths_refresh_tx(&tx, Self::SYNC_VIDEO_LIBRARY_PATHS_SQL)?;

        // ── PDFs ──
        tx.execute(
            "CREATE TEMP TABLE _pdf_lib_refresh_paths (path TEXT PRIMARY KEY)",
            [],
        )
        .map_err(|e| e.to_string())?;
        tx.execute(
            "INSERT OR REPLACE INTO _pdf_lib_refresh_paths VALUES (?1)",
            params![path],
        )
        .map_err(|e| e.to_string())?;
        tx.execute(
            "DELETE FROM pdfs_fts WHERE rowid IN (SELECT id FROM pdfs WHERE path IN (SELECT path FROM _pdf_lib_refresh_paths))",
            [],
        )
        .map_err(|e| e.to_string())?;
        tx.execute(
            "DELETE FROM pdfs WHERE path IN (SELECT path FROM _pdf_lib_refresh_paths)",
            [],
        )
        .map_err(|e| e.to_string())?;
        Self::exec_sync_paths_refresh_tx(&tx, Self::SYNC_PDF_LIBRARY_PATHS_SQL)?;

        // ── Plugins (no FTS) ──
        tx.execute(
            "CREATE TEMP TABLE _pl_refresh_paths (path TEXT PRIMARY KEY)",
            [],
        )
        .map_err(|e| e.to_string())?;
        tx.execute(
            "INSERT OR REPLACE INTO _pl_refresh_paths VALUES (?1)",
            params![path],
        )
        .map_err(|e| e.to_string())?;
        tx.execute(
            "DELETE FROM plugins WHERE path IN (SELECT path FROM _pl_refresh_paths)",
            [],
        )
        .map_err(|e| e.to_string())?;
        Self::exec_sync_paths_refresh_tx(&tx, Self::SYNC_PLUGIN_LIBRARY_PATHS_SQL)?;

        tx.commit().map_err(|e| e.to_string())?;
        self.invalidate_audio_library_total_cache();
        self.invalidate_daw_library_total_cache();
        self.invalidate_preset_inventory_total_cache();
        self.invalidate_midi_library_total_cache();
        self.invalidate_video_library_total_cache();
        self.invalidate_pdf_library_total_cache();
        self.invalidate_plugin_library_total_cache();
        Ok(())
    }

    pub fn audio_scan_parent_finalize(
        &self,
        id: &str,
        _sample_count: u64,
        _total_bytes: u64,
        _format_counts: &HashMap<String, usize>,
    ) -> Result<(), String> {
        let conn = self.read_conn();
        let sample_count: i64 = conn
            .query_row("SELECT COUNT(DISTINCT path) FROM audio_samples", [], |r| {
                r.get(0)
            })
            .unwrap_or(0);
        let total_bytes: i64 = conn
            .query_row(
                "SELECT COALESCE(SUM(s.size), 0) FROM audio_samples s INNER JOIN audio_library lib ON s.id = lib.sample_id",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);
        let mut format_map: HashMap<String, usize> = HashMap::new();
        let mut stmt = conn
            .prepare(
                "SELECT s.format, COUNT(*) FROM audio_samples s INNER JOIN audio_library lib ON s.id = lib.sample_id GROUP BY s.format",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? as usize))
            })
            .map_err(|e| e.to_string())?;
        for (fmt, n) in rows.flatten() {
            format_map.insert(fmt, n);
        }
        let fc_json = serde_json::to_string(&format_map).unwrap_or_default();
        conn.execute(
            "UPDATE audio_scans SET sample_count = ?2, total_bytes = ?3, format_counts = ?4 WHERE id = ?1",
            params![id, sample_count, total_bytes, fc_json],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn insert_audio_batch(
        &self,
        scan_id: &str,
        samples: &[AudioSample],
    ) -> Result<u64, String> {
        let conn = self.read_conn();
        let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
        let mut inserted: u64 = 0;
        let mut batch_bytes: u64 = 0;
        {
            // INSERT OR IGNORE (not REPLACE) so auto-increment ids stay stable —
            // FTS5 rowid is linked to audio_samples.id and REPLACE would break that
            // link. parent_create clears rows per scan, so conflicts only occur
            // within a scan (same path emitted twice) — safe to ignore duplicates.
            let mut stmt = tx
                .prepare_cached(
                    "INSERT OR IGNORE INTO audio_samples
                     (name, path, directory, format, size, size_formatted, modified,
                      duration, channels, sample_rate, bits_per_sample, scan_id)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                )
                .map_err(|e| e.to_string())?;
            let mut fts_stmt = tx
                .prepare_cached(
                    "INSERT INTO audio_samples_fts(rowid, name, path, scan_id) VALUES (?1, ?2, ?3, ?4)",
                )
                .map_err(|e| e.to_string())?;
            let mut lib_stmt = tx
                .prepare_cached(
                    "INSERT INTO audio_library (path, sample_id) VALUES (?1, ?2)
                     ON CONFLICT(path) DO UPDATE SET sample_id = CASE
                       WHEN excluded.sample_id > audio_library.sample_id THEN excluded.sample_id
                       ELSE audio_library.sample_id END",
                )
                .map_err(|e| e.to_string())?;

            for s in samples {
                let path = normalize_path_for_db(&s.path);
                let directory = normalize_path_for_db(&s.directory);
                let changed = stmt
                    .execute(params![
                        s.name,
                        path,
                        directory,
                        s.format,
                        s.size as i64,
                        s.size_formatted,
                        s.modified,
                        s.duration,
                        s.channels,
                        s.sample_rate,
                        s.bits_per_sample,
                        scan_id,
                    ])
                    .map_err(|e| e.to_string())?;
                if changed > 0 {
                    let id = tx.last_insert_rowid();
                    fts_stmt
                        .execute(params![id, s.name, path, scan_id])
                        .map_err(|e| e.to_string())?;
                    lib_stmt
                        .execute(params![path, id])
                        .map_err(|e| e.to_string())?;
                    inserted += 1;
                    batch_bytes += s.size;
                }
            }
        }
        // Increment parent row counts so history is accurate mid-scan.
        if inserted > 0 {
            tx.execute(
                "UPDATE audio_scans SET sample_count = sample_count + ?2, total_bytes = total_bytes + ?3 WHERE id = ?1",
                params![scan_id, inserted as i64, batch_bytes as i64],
            ).map_err(|e| e.to_string())?;
        }
        tx.commit().map_err(|e| e.to_string())?;
        if inserted > 0 {
            self.invalidate_audio_library_total_cache();
        }
        Ok(inserted)
    }

    /// Save scan metadata.
    pub fn save_scan(
        &self,
        id: &str,
        timestamp: &str,
        sample_count: u64,
        total_bytes: u64,
        format_counts: &HashMap<String, usize>,
        roots: &[String],
    ) -> Result<(), String> {
        let conn = self.read_conn();
        let fc_json = serde_json::to_string(format_counts).unwrap_or_default();
        let roots_json = path_strings_json_normalized(roots);
        conn.execute(
            "INSERT OR REPLACE INTO audio_scans
             (id, timestamp, sample_count, total_bytes, format_counts, roots, scan_complete)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1)",
            params![
                id,
                timestamp,
                sample_count as i64,
                total_bytes as i64,
                fc_json,
                roots_json
            ],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Get the most recent scan ID.
    pub fn latest_scan_id(&self) -> Result<Option<String>, String> {
        let conn = self.read_conn();
        conn.query_row(
            "SELECT id FROM audio_scans WHERE scan_complete = 1 ORDER BY timestamp DESC LIMIT 1",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|e| e.to_string())
    }

    /// List all scans (metadata only).
    pub fn list_scans(&self) -> Result<Vec<ScanInfo>, String> {
        let conn = self.read_conn();
        let mut stmt = conn
            .prepare(
                "SELECT id, timestamp, sample_count, total_bytes, format_counts, roots
                 FROM audio_scans WHERE scan_complete = 1 ORDER BY timestamp DESC",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| {
                let fc_str: String = row.get(4)?;
                let roots_str: String = row.get(5)?;
                Ok(ScanInfo {
                    id: row.get(0)?,
                    timestamp: row.get(1)?,
                    sample_count: row.get::<_, i64>(2)? as u64,
                    total_bytes: row.get::<_, i64>(3)? as u64,
                    format_counts: serde_json::from_str(&fc_str).unwrap_or_default(),
                    roots: serde_json::from_str(&roots_str).unwrap_or_default(),
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    }

    /// Bounded FTS hit count for one scan (`audio_samples_fts.scan_id`).
    fn audio_fts_bounded_count_scan(
        conn: &Connection,
        fts_match: &str,
        scan_id: &str,
    ) -> Result<(u64, bool), String> {
        let cap = FTS_INVENTORY_MATCH_COUNT_CAP + 1;
        let raw: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM (
  SELECT 1 FROM audio_samples_fts
  WHERE audio_samples_fts MATCH ?1 AND scan_id = ?2
  LIMIT ?3)",
                params![fts_match, scan_id, cap],
                |row| row.get::<_, i64>(0),
            )
            .map_err(|e| e.to_string())?;
        let capped = raw > FTS_INVENTORY_MATCH_COUNT_CAP;
        let count = if capped {
            FTS_INVENTORY_MATCH_COUNT_CAP as u64
        } else {
            raw as u64
        };
        Ok((count, capped))
    }

    /// Bounded FTS hit count for library scope (same `format_filter` rules as [`Self::query_audio`]).
    fn audio_fts_bounded_count_library(
        conn: &Connection,
        fts_match: &str,
        format_filter: Option<&str>,
    ) -> Result<(u64, bool), String> {
        let cap = FTS_INVENTORY_MATCH_COUNT_CAP + 1;
        let raw: i64 = match format_filter {
            None => conn
                .query_row(
                    "SELECT COUNT(*) FROM (
  SELECT 1 FROM audio_samples_fts
  INNER JOIN audio_samples s ON s.id = audio_samples_fts.rowid
  INNER JOIN audio_library lib ON lib.sample_id = s.id
  WHERE audio_samples_fts MATCH ?1
  LIMIT ?2)",
                    params![fts_match, cap],
                    |row| row.get::<_, i64>(0),
                )
                .map_err(|e| e.to_string())?,
            Some(f) if f.trim().is_empty() || f == "all" => conn
                .query_row(
                    "SELECT COUNT(*) FROM (
  SELECT 1 FROM audio_samples_fts
  INNER JOIN audio_samples s ON s.id = audio_samples_fts.rowid
  INNER JOIN audio_library lib ON lib.sample_id = s.id
  WHERE audio_samples_fts MATCH ?1
  LIMIT ?2)",
                    params![fts_match, cap],
                    |row| row.get::<_, i64>(0),
                )
                .map_err(|e| e.to_string())?,
            Some(f) if f.contains(',') => {
                let in_list = Self::in_list_sql(f);
                let sql = format!(
                    "SELECT COUNT(*) FROM (
  SELECT 1 FROM audio_samples_fts
  INNER JOIN audio_samples s ON s.id = audio_samples_fts.rowid
  INNER JOIN audio_library lib ON lib.sample_id = s.id
  WHERE audio_samples_fts MATCH ?1
  AND s.format IN ({in_list})
  LIMIT ?2)"
                );
                conn.query_row(&sql, params![fts_match, cap], |row| row.get::<_, i64>(0))
                    .map_err(|e| e.to_string())?
            }
            Some(f) => conn
                .query_row(
                    "SELECT COUNT(*) FROM (
  SELECT 1 FROM audio_samples_fts
  INNER JOIN audio_samples s ON s.id = audio_samples_fts.rowid
  INNER JOIN audio_library lib ON lib.sample_id = s.id
  WHERE audio_samples_fts MATCH ?1
  AND s.format = ?2
  LIMIT ?3)",
                    params![fts_match, f, cap],
                    |row| row.get::<_, i64>(0),
                )
                .map_err(|e| e.to_string())?,
        };
        let capped = raw > FTS_INVENTORY_MATCH_COUNT_CAP;
        let count = if capped {
            FTS_INVENTORY_MATCH_COUNT_CAP as u64
        } else {
            raw as u64
        };
        Ok((count, capped))
    }

    /// Bounded FTS hit count for presets library (non-MIDI formats only; matches [`Self::query_presets`]).
    fn preset_fts_bounded_count_library(
        conn: &Connection,
        fts_match: &str,
        format_filter: Option<&str>,
    ) -> Result<(u64, bool), String> {
        let cap = FTS_INVENTORY_MATCH_COUNT_CAP + 1;
        let raw: i64 = match format_filter {
            None => conn
                .query_row(
                    "SELECT COUNT(*) FROM (
  SELECT 1 FROM presets_fts
  INNER JOIN presets p ON p.id = presets_fts.rowid
  INNER JOIN preset_library lib ON lib.preset_id = p.id
  WHERE presets_fts MATCH ?1
  AND p.format NOT IN ('MID','MIDI')
  LIMIT ?2)",
                    params![fts_match, cap],
                    |row| row.get::<_, i64>(0),
                )
                .map_err(|e| e.to_string())?,
            Some(f) if f.trim().is_empty() || f == "all" => conn
                .query_row(
                    "SELECT COUNT(*) FROM (
  SELECT 1 FROM presets_fts
  INNER JOIN presets p ON p.id = presets_fts.rowid
  INNER JOIN preset_library lib ON lib.preset_id = p.id
  WHERE presets_fts MATCH ?1
  AND p.format NOT IN ('MID','MIDI')
  LIMIT ?2)",
                    params![fts_match, cap],
                    |row| row.get::<_, i64>(0),
                )
                .map_err(|e| e.to_string())?,
            Some(f) if f.contains(',') => {
                let in_list = Self::in_list_sql(f);
                let sql = format!(
                    "SELECT COUNT(*) FROM (
  SELECT 1 FROM presets_fts
  INNER JOIN presets p ON p.id = presets_fts.rowid
  INNER JOIN preset_library lib ON lib.preset_id = p.id
  WHERE presets_fts MATCH ?1
  AND p.format NOT IN ('MID','MIDI')
  AND p.format IN ({in_list})
  LIMIT ?2)"
                );
                conn.query_row(&sql, params![fts_match, cap], |row| row.get::<_, i64>(0))
                    .map_err(|e| e.to_string())?
            }
            Some(f) => conn
                .query_row(
                    "SELECT COUNT(*) FROM (
  SELECT 1 FROM presets_fts
  INNER JOIN presets p ON p.id = presets_fts.rowid
  INNER JOIN preset_library lib ON lib.preset_id = p.id
  WHERE presets_fts MATCH ?1
  AND p.format NOT IN ('MID','MIDI')
  AND p.format = ?2
  LIMIT ?3)",
                    params![fts_match, f, cap],
                    |row| row.get::<_, i64>(0),
                )
                .map_err(|e| e.to_string())?,
        };
        let capped = raw > FTS_INVENTORY_MATCH_COUNT_CAP;
        let count = if capped {
            FTS_INVENTORY_MATCH_COUNT_CAP as u64
        } else {
            raw as u64
        };
        Ok((count, capped))
    }

    /// Bounded FTS hit count for MIDI library (same `format_filter` rules as [`Self::query_midi`]).
    fn midi_fts_bounded_count_library(
        conn: &Connection,
        fts_match: &str,
        format_filter: Option<&str>,
    ) -> Result<(u64, bool), String> {
        let cap = FTS_INVENTORY_MATCH_COUNT_CAP + 1;
        let raw: i64 = match format_filter {
            None => conn
                .query_row(
                    "SELECT COUNT(*) FROM (
  SELECT 1 FROM midi_files_fts
  INNER JOIN midi_library AS lib ON lib.midi_id = midi_files_fts.rowid
  WHERE midi_files_fts MATCH ?1
  LIMIT ?2)",
                    params![fts_match, cap],
                    |row| row.get::<_, i64>(0),
                )
                .map_err(|e| e.to_string())?,
            Some(f) if f.trim().is_empty() || f == "all" => conn
                .query_row(
                    "SELECT COUNT(*) FROM (
  SELECT 1 FROM midi_files_fts
  INNER JOIN midi_library AS lib ON lib.midi_id = midi_files_fts.rowid
  WHERE midi_files_fts MATCH ?1
  LIMIT ?2)",
                    params![fts_match, cap],
                    |row| row.get::<_, i64>(0),
                )
                .map_err(|e| e.to_string())?,
            Some(f) if f.contains(',') => {
                let in_list = Self::in_list_sql(f);
                let sql = format!(
                    "SELECT COUNT(*) FROM (
  SELECT 1 FROM midi_files_fts
  INNER JOIN midi_files AS m ON m.id = midi_files_fts.rowid
  INNER JOIN midi_library AS lib ON lib.midi_id = m.id
  WHERE midi_files_fts MATCH ?1 AND m.format IN ({in_list})
  LIMIT ?2)"
                );
                conn.query_row(&sql, params![fts_match, cap], |row| row.get::<_, i64>(0))
                    .map_err(|e| e.to_string())?
            }
            Some(f) => conn
                .query_row(
                    "SELECT COUNT(*) FROM (
  SELECT 1 FROM midi_files_fts
  INNER JOIN midi_files AS m ON m.id = midi_files_fts.rowid
  INNER JOIN midi_library AS lib ON lib.midi_id = m.id
  WHERE midi_files_fts MATCH ?1 AND m.format = ?2
  LIMIT ?3)",
                    params![fts_match, f, cap],
                    |row| row.get::<_, i64>(0),
                )
                .map_err(|e| e.to_string())?,
        };
        let capped = raw > FTS_INVENTORY_MATCH_COUNT_CAP;
        let count = if capped {
            FTS_INVENTORY_MATCH_COUNT_CAP as u64
        } else {
            raw as u64
        };
        Ok((count, capped))
    }

    /// Bounded FTS hit count for video library (same `format_filter` rules as [`Self::query_video`]).
    fn video_fts_bounded_count_library(
        conn: &Connection,
        fts_match: &str,
        format_filter: Option<&str>,
    ) -> Result<(u64, bool), String> {
        let cap = FTS_INVENTORY_MATCH_COUNT_CAP + 1;
        let raw: i64 = match format_filter {
            None => conn
                .query_row(
                    "SELECT COUNT(*) FROM (
  SELECT 1 FROM video_files_fts
  INNER JOIN video_library AS lib ON lib.video_id = video_files_fts.rowid
  WHERE video_files_fts MATCH ?1
  LIMIT ?2)",
                    params![fts_match, cap],
                    |row| row.get::<_, i64>(0),
                )
                .map_err(|e| e.to_string())?,
            Some(f) if f.trim().is_empty() || f == "all" => conn
                .query_row(
                    "SELECT COUNT(*) FROM (
  SELECT 1 FROM video_files_fts
  INNER JOIN video_library AS lib ON lib.video_id = video_files_fts.rowid
  WHERE video_files_fts MATCH ?1
  LIMIT ?2)",
                    params![fts_match, cap],
                    |row| row.get::<_, i64>(0),
                )
                .map_err(|e| e.to_string())?,
            Some(f) if f.contains(',') => {
                let in_list = Self::in_list_sql(f);
                let sql = format!(
                    "SELECT COUNT(*) FROM (
  SELECT 1 FROM video_files_fts
  INNER JOIN video_files AS v ON v.id = video_files_fts.rowid
  INNER JOIN video_library AS lib ON lib.video_id = v.id
  WHERE video_files_fts MATCH ?1 AND v.format IN ({in_list})
  LIMIT ?2)"
                );
                conn.query_row(&sql, params![fts_match, cap], |row| row.get::<_, i64>(0))
                    .map_err(|e| e.to_string())?
            }
            Some(f) => conn
                .query_row(
                    "SELECT COUNT(*) FROM (
  SELECT 1 FROM video_files_fts
  INNER JOIN video_files AS v ON v.id = video_files_fts.rowid
  INNER JOIN video_library AS lib ON lib.video_id = v.id
  WHERE video_files_fts MATCH ?1 AND v.format = ?2
  LIMIT ?3)",
                    params![fts_match, f, cap],
                    |row| row.get::<_, i64>(0),
                )
                .map_err(|e| e.to_string())?,
        };
        let capped = raw > FTS_INVENTORY_MATCH_COUNT_CAP;
        let count = if capped {
            FTS_INVENTORY_MATCH_COUNT_CAP as u64
        } else {
            raw as u64
        };
        Ok((count, capped))
    }

    /// Bounded FTS hit count for PDF library ([`Self::query_pdfs`] has no format filter).
    fn pdf_fts_bounded_count_library(
        conn: &Connection,
        fts_match: &str,
    ) -> Result<(u64, bool), String> {
        let cap = FTS_INVENTORY_MATCH_COUNT_CAP + 1;
        let raw: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM (
  SELECT 1 FROM pdfs_fts
  INNER JOIN pdf_library AS lib ON lib.pdf_id = pdfs_fts.rowid
  WHERE pdfs_fts MATCH ?1
  LIMIT ?2)",
                params![fts_match, cap],
                |row| row.get::<_, i64>(0),
            )
            .map_err(|e| e.to_string())?;
        let capped = raw > FTS_INVENTORY_MATCH_COUNT_CAP;
        let count = if capped {
            FTS_INVENTORY_MATCH_COUNT_CAP as u64
        } else {
            raw as u64
        };
        Ok((count, capped))
    }

    /// Bounded FTS hit count for DAW project library ([`Self::query_daw`] `daw_filter` rules).
    fn daw_fts_bounded_count_library(
        conn: &Connection,
        fts_match: &str,
        daw_filter: Option<&str>,
    ) -> Result<(u64, bool), String> {
        let cap = FTS_INVENTORY_MATCH_COUNT_CAP + 1;
        let raw: i64 = match daw_filter {
            None => conn
                .query_row(
                    "SELECT COUNT(*) FROM (
  SELECT 1 FROM daw_projects_fts
  INNER JOIN daw_projects d ON d.id = daw_projects_fts.rowid
  INNER JOIN daw_library lib ON lib.project_id = d.id
  WHERE daw_projects_fts MATCH ?1
  LIMIT ?2)",
                    params![fts_match, cap],
                    |row| row.get::<_, i64>(0),
                )
                .map_err(|e| e.to_string())?,
            Some(f) if f.trim().is_empty() || f == "all" => conn
                .query_row(
                    "SELECT COUNT(*) FROM (
  SELECT 1 FROM daw_projects_fts
  INNER JOIN daw_projects d ON d.id = daw_projects_fts.rowid
  INNER JOIN daw_library lib ON lib.project_id = d.id
  WHERE daw_projects_fts MATCH ?1
  LIMIT ?2)",
                    params![fts_match, cap],
                    |row| row.get::<_, i64>(0),
                )
                .map_err(|e| e.to_string())?,
            Some(f) if f.contains(',') => {
                let in_list = Self::in_list_sql(f);
                let sql = format!(
                    "SELECT COUNT(*) FROM (
  SELECT 1 FROM daw_projects_fts
  INNER JOIN daw_projects d ON d.id = daw_projects_fts.rowid
  INNER JOIN daw_library lib ON lib.project_id = d.id
  WHERE daw_projects_fts MATCH ?1
  AND d.daw IN ({in_list})
  LIMIT ?2)"
                );
                conn.query_row(&sql, params![fts_match, cap], |row| row.get::<_, i64>(0))
                    .map_err(|e| e.to_string())?
            }
            Some(f) => conn
                .query_row(
                    "SELECT COUNT(*) FROM (
  SELECT 1 FROM daw_projects_fts
  INNER JOIN daw_projects d ON d.id = daw_projects_fts.rowid
  INNER JOIN daw_library lib ON lib.project_id = d.id
  WHERE daw_projects_fts MATCH ?1
  AND d.daw = ?2
  LIMIT ?3)",
                    params![fts_match, f, cap],
                    |row| row.get::<_, i64>(0),
                )
                .map_err(|e| e.to_string())?,
        };
        let capped = raw > FTS_INVENTORY_MATCH_COUNT_CAP;
        let count = if capped {
            FTS_INVENTORY_MATCH_COUNT_CAP as u64
        } else {
            raw as u64
        };
        Ok((count, capped))
    }

    /// Bounded COUNT for plugin tab search (LIKE / REGEXP; no FTS). Same cap as inventory FTS paths.
    fn plugin_search_bounded_count_library(
        conn: &Connection,
        from_sql: &str,
        where_cl: &str,
        regex_pat: &Option<String>,
        like_pat: &Option<String>,
        type_filter: Option<&str>,
    ) -> Result<(u64, bool), String> {
        let cap = FTS_INVENTORY_MATCH_COUNT_CAP + 1;
        let mut cap_idx = 1usize;
        if regex_pat.is_some() || like_pat.is_some() {
            cap_idx += 1;
        }
        if type_filter
            .map(|t| !t.is_empty() && t != "all" && !t.contains(','))
            .unwrap_or(false)
        {
            cap_idx += 1;
        }
        let sql =
            format!("SELECT COUNT(*) FROM (SELECT 1 {from_sql} WHERE {where_cl} LIMIT ?{cap_idx})");
        let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
        let mut bi = 1usize;
        if let Some(ref p) = regex_pat.as_ref().or(like_pat.as_ref()) {
            stmt.raw_bind_parameter(bi, p).map_err(|e| e.to_string())?;
            bi += 1;
        }
        if let Some(tf) = type_filter
            && !tf.is_empty() && tf != "all" && !tf.contains(',') {
                stmt.raw_bind_parameter(bi, tf).map_err(|e| e.to_string())?;
                bi += 1;
            }
        stmt.raw_bind_parameter(bi, cap)
            .map_err(|e| e.to_string())?;
        let mut rows = stmt.raw_query();
        let raw: i64 = rows
            .next()
            .map_err(|e| e.to_string())?
            .map(|r| r.get::<_, i64>(0).unwrap_or(0))
            .unwrap_or(0);
        let capped = raw > FTS_INVENTORY_MATCH_COUNT_CAP;
        let count = if capped {
            FTS_INVENTORY_MATCH_COUNT_CAP as u64
        } else {
            raw as u64
        };
        Ok((count, capped))
    }

    /// Paginated, sortable, filterable query for audio samples.
    ///
    /// `scan_id` **Some(non-empty)** → rows for that scan only (history detail).
    /// `scan_id` **None** or empty → **library** mode: all rows across scans, deduped by `path`
    /// (`MAX(id)` per path).
    pub fn query_audio(&self, params: &AudioQueryParams) -> Result<AudioQueryResult, String> {
        let conn = self.read_conn();

        let single_scan = params
            .scan_id
            .as_ref()
            .map(|s| !s.is_empty())
            .unwrap_or(false);
        let scan_id = if single_scan {
            params.scan_id.as_ref().unwrap().clone()
        } else {
            String::new()
        };

        if single_scan && scan_id.is_empty() {
            return Ok(AudioQueryResult {
                samples: vec![],
                total_count: 0,
                total_count_capped: false,
                total_unfiltered: 0,
            });
        }

        // Build WHERE clause
        let mut conditions = if single_scan {
            vec!["scan_id = ?1".to_string()]
        } else {
            vec![AUDIO_LIBRARY_IDS.to_string()]
        };
        let mut bind_idx = if single_scan { 2 } else { 1 };

        // FTS5 trigram for ≥3 char searches; LIKE fallback for 1–2 chars.
        // Regex mode (UI `.*` toggle): real ECMA-style regex via SQLite `REGEXP`, not FTS phrase.
        let (fts_match, like_pat, regex_pat) =
            classify_fts_name_path_search(params.search.as_deref(), params.search_regex);
        if fts_match.is_some() {
            if single_scan {
                conditions.push(format!(
                    "id IN (SELECT rowid FROM audio_samples_fts WHERE audio_samples_fts MATCH ?{bind_idx} AND scan_id = ?{scan_idx})",
                    scan_idx = bind_idx + 1,
                ));
                bind_idx += 2;
            } else {
                // Library scope is already `AUDIO_LIBRARY_IDS` above; do not nest a second
                // `sample_id IN audio_library` inside the FTS subquery (same semantics, worse plan).
                conditions.push(format!(
                    "id IN (SELECT rowid FROM audio_samples_fts WHERE audio_samples_fts MATCH ?{bind_idx})",
                    bind_idx = bind_idx,
                ));
                bind_idx += 1;
            }
        } else if regex_pat.is_some() {
            conditions.push(format!(
                "((name REGEXP ?{bind_idx}) OR (path REGEXP ?{bind_idx}))"
            ));
            bind_idx += 1;
        } else if like_pat.is_some() {
            conditions.push(format!(
                "(name LIKE ?{bind_idx} ESCAPE '\\' OR path LIKE ?{bind_idx} ESCAPE '\\')"
            ));
            bind_idx += 1;
        }

        if let Some(fmt) = &params.format_filter
            && !fmt.is_empty() && fmt != "all" {
                if fmt.contains(',') {
                    let vals: Vec<String> = fmt
                        .split(',')
                        .map(|s| format!("'{}'", s.trim().replace('\'', "''")))
                        .collect();
                    conditions.push(format!("format IN ({})", vals.join(",")));
                } else {
                    conditions.push(format!("format = ?{bind_idx}"));
                    bind_idx += 1;
                }
            }

        let where_clause = conditions.join(" AND ");

        // Validate sort key
        let sort_col = match params.sort_key.as_str() {
            "name" => "name COLLATE NOCASE",
            "format" => "format",
            "size" => "size",
            "modified" => "modified",
            "directory" => "directory COLLATE NOCASE",
            "bpm" => "bpm",
            "key" => "key_name",
            "lufs" => "lufs",
            "duration" => "duration",
            "channels" => "channels",
            _ => "name COLLATE NOCASE",
        };
        let sort_dir = if params.sort_asc { "ASC" } else { "DESC" };
        let nulls = "NULLS LAST";

        // Total unfiltered count
        let total_unfiltered: u64 = if single_scan {
            conn.query_row(
                "SELECT COUNT(*) FROM audio_samples WHERE scan_id = ?1",
                params![scan_id],
                |row| row.get::<_, i64>(0).map(|v| v as u64),
            )
            .map_err(|e| e.to_string())?
        } else {
            self.audio_library_total_rows(&conn)?
        };

        // Filtered total: separate COUNT so the main SELECT can use LIMIT without
        // COUNT(*) OVER(), which SQLite evaluates before LIMIT (full scan / lockup at 200k+ rows).
        //
        // FTS: bounded COUNT (`FTS_INVENTORY_MATCH_COUNT_CAP`) — common substrings can match
        // hundreds of thousands of rows; exact `COUNT(*)` over that set dominated latency.
        let count_sql = format!("SELECT COUNT(*) FROM audio_samples WHERE {where_clause}");
        let (total_count, total_count_capped) = if let Some(m) = fts_match.as_ref() {
            if single_scan {
                Self::audio_fts_bounded_count_scan(&conn, m, &scan_id)?
            } else {
                Self::audio_fts_bounded_count_library(&conn, m, params.format_filter.as_deref())?
            }
        } else {
            let mut count_stmt = conn.prepare(&count_sql).map_err(|e| e.to_string())?;
            let mut idx = 1;
            if single_scan {
                count_stmt
                    .raw_bind_parameter(idx, &scan_id)
                    .map_err(|e| e.to_string())?;
                idx += 1;
            }
            if let Some(ref r) = regex_pat {
                count_stmt
                    .raw_bind_parameter(idx, r)
                    .map_err(|e| e.to_string())?;
                idx += 1;
            } else if let Some(ref pat) = like_pat {
                count_stmt
                    .raw_bind_parameter(idx, pat)
                    .map_err(|e| e.to_string())?;
                idx += 1;
            }
            if let Some(ref fmt) = params.format_filter
                && !fmt.is_empty() && fmt != "all" && !fmt.contains(',') {
                    count_stmt
                        .raw_bind_parameter(idx, fmt)
                        .map_err(|e| e.to_string())?;
                }
            let mut count_rows = count_stmt.raw_query();
            let row = count_rows
                .next()
                .map_err(|e| e.to_string())?
                .ok_or_else(|| "COUNT returned no rows".to_string())?;
            let n = row.get::<_, i64>(0).map_err(|e| e.to_string())? as u64;
            (n, false)
        };

        // FTS substring search: ORDER BY column sorts the entire match set before LIMIT (stalls).
        // Use bm25 + LIMIT like PDF/MIDI; frontend may re-rank with `searchScore`.
        let use_fts_bm25 = fts_match.is_some();
        let query_sql = if use_fts_bm25 {
            let (mut w, mut next_ph) = if single_scan {
                (
                    String::from(
                        "SELECT s.name, s.path, s.directory, s.format, s.size, s.size_formatted, s.modified,
                s.duration, s.channels, s.sample_rate, s.bits_per_sample, s.bpm, s.key_name, s.lufs, s.bpm_exhausted
                FROM audio_samples_fts
                INNER JOIN audio_samples s ON s.id = audio_samples_fts.rowid
                 WHERE audio_samples_fts MATCH ?1 AND s.scan_id = ?2",
                    ),
                    3usize,
                )
            } else {
                (
                    String::from(
                        "SELECT s.name, s.path, s.directory, s.format, s.size, s.size_formatted, s.modified,
                s.duration, s.channels, s.sample_rate, s.bits_per_sample, s.bpm, s.key_name, s.lufs, s.bpm_exhausted
                FROM audio_samples_fts
                INNER JOIN audio_samples s ON s.id = audio_samples_fts.rowid
                 INNER JOIN audio_library lib ON lib.sample_id = s.id
                 WHERE audio_samples_fts MATCH ?1",
                    ),
                    2usize,
                )
            };
            if let Some(fmt) = &params.format_filter
                && !fmt.is_empty() && fmt != "all" {
                    if fmt.contains(',') {
                        let vals: Vec<String> = fmt
                            .split(',')
                            .map(|s| format!("'{}'", s.trim().replace('\'', "''")))
                            .collect();
                        w.push_str(&format!(" AND s.format IN ({})", vals.join(",")));
                    } else {
                        w.push_str(&format!(" AND s.format = ?{next_ph}"));
                        next_ph += 1;
                    }
                }
            let plim = next_ph;
            let poff = next_ph + 1;
            w.push_str(&format!(
                " ORDER BY bm25(audio_samples_fts) LIMIT ?{plim} OFFSET ?{poff}"
            ));
            w
        } else {
            format!(
                "SELECT name, path, directory, format, size, size_formatted, modified,
                    duration, channels, sample_rate, bits_per_sample, bpm, key_name, lufs, bpm_exhausted
             FROM audio_samples
             WHERE {where_clause}
             ORDER BY {sort_col} {sort_dir} {nulls}
             LIMIT ?{limit_idx} OFFSET ?{offset_idx}",
                limit_idx = bind_idx,
                offset_idx = bind_idx + 1,
            )
        };

        let mut stmt = conn.prepare(&query_sql).map_err(|e| e.to_string())?;
        let mut idx = 1;
        if use_fts_bm25 {
            let m = fts_match.as_ref().expect("fts");
            stmt.raw_bind_parameter(idx, m).map_err(|e| e.to_string())?;
            idx += 1;
            if single_scan {
                stmt.raw_bind_parameter(idx, &scan_id)
                    .map_err(|e| e.to_string())?;
                idx += 1;
            }
            if let Some(ref fmt) = params.format_filter
                && !fmt.is_empty() && fmt != "all" && !fmt.contains(',') {
                    stmt.raw_bind_parameter(idx, fmt)
                        .map_err(|e| e.to_string())?;
                    idx += 1;
                }
            stmt.raw_bind_parameter(idx, params.limit as i64)
                .map_err(|e| e.to_string())?;
            stmt.raw_bind_parameter(idx + 1, params.offset as i64)
                .map_err(|e| e.to_string())?;
        } else {
            if single_scan {
                stmt.raw_bind_parameter(idx, &scan_id)
                    .map_err(|e| e.to_string())?;
                idx += 1;
            }
            if let Some(ref r) = regex_pat {
                stmt.raw_bind_parameter(idx, r).map_err(|e| e.to_string())?;
                idx += 1;
            } else if let Some(ref pat) = like_pat {
                stmt.raw_bind_parameter(idx, pat)
                    .map_err(|e| e.to_string())?;
                idx += 1;
            }
            if let Some(ref fmt) = params.format_filter
                && !fmt.is_empty() && fmt != "all" && !fmt.contains(',') {
                    stmt.raw_bind_parameter(idx, fmt)
                        .map_err(|e| e.to_string())?;
                    idx += 1;
                }
            stmt.raw_bind_parameter(idx, params.limit as i64)
                .map_err(|e| e.to_string())?;
            stmt.raw_bind_parameter(idx + 1, params.offset as i64)
                .map_err(|e| e.to_string())?;
        }

        let mut samples = Vec::new();
        let mut rows = stmt.raw_query();
        while let Some(row) = rows.next().map_err(|e| e.to_string())? {
            samples.push(AudioSampleRow {
                name: row.get(0).unwrap_or_default(),
                path: row.get(1).unwrap_or_default(),
                directory: row.get(2).unwrap_or_default(),
                format: row.get(3).unwrap_or_default(),
                size: row.get::<_, i64>(4).unwrap_or(0) as u64,
                size_formatted: row.get(5).unwrap_or_default(),
                modified: row.get(6).unwrap_or_default(),
                duration: row.get(7).ok(),
                channels: row
                    .get::<_, Option<i32>>(8)
                    .ok()
                    .flatten()
                    .map(|v| v as u16),
                sample_rate: row
                    .get::<_, Option<i32>>(9)
                    .ok()
                    .flatten()
                    .map(|v| v as u32),
                bits_per_sample: row
                    .get::<_, Option<i32>>(10)
                    .ok()
                    .flatten()
                    .map(|v| v as u16),
                bpm: row.get(11).ok(),
                key: row.get(12).ok(),
                lufs: row.get(13).ok(),
                bpm_exhausted: row.get::<_, i64>(14).ok().map(|v| v != 0).unwrap_or(false),
            });
        }

        Ok(AudioQueryResult {
            samples,
            total_count,
            total_count_capped,
            total_unfiltered,
        })
    }

    /// Get aggregate stats. `scan_id` None or empty → full library (deduped by path). Otherwise that scan only.
    pub fn audio_stats(&self, scan_id: Option<&str>) -> Result<AudioStatsResult, String> {
        let conn = self.read_conn();

        let library = scan_id.map(|s| s.is_empty()).unwrap_or(true);
        if library {
            let sample_count: u64 = conn
                .query_row("SELECT COUNT(*) FROM audio_library", [], |row| {
                    row.get::<_, i64>(0).map(|v| v as u64)
                })
                .unwrap_or(0);
            let total_bytes: u64 = conn
                .query_row(
                    "SELECT COALESCE(SUM(s.size), 0) FROM audio_samples s INNER JOIN audio_library lib ON s.id = lib.sample_id",
                    [],
                    |row| row.get::<_, i64>(0).map(|v| v as u64),
                )
                .unwrap_or(0);
            let mut format_counts = HashMap::new();
            let mut stmt = conn
                .prepare(
                    "SELECT s.format, COUNT(*) FROM audio_samples s INNER JOIN audio_library lib ON s.id = lib.sample_id GROUP BY s.format",
                )
                .map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map([], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? as u64))
                })
                .map_err(|e| e.to_string())?;
            for (fmt, count) in rows.flatten() {
                format_counts.insert(fmt, count);
            }
            let analyzed_count: u64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM audio_samples s INNER JOIN audio_library lib ON s.id = lib.sample_id WHERE s.bpm IS NOT NULL",
                    [],
                    |row| row.get::<_, i64>(0).map(|v| v as u64),
                )
                .unwrap_or(0);
            return Ok(AudioStatsResult {
                sample_count,
                total_bytes,
                format_counts,
                analyzed_count,
            });
        }

        let sid = scan_id.expect("scan_id").to_string();
        if sid.is_empty() {
            return Ok(AudioStatsResult {
                sample_count: 0,
                total_bytes: 0,
                format_counts: HashMap::new(),
                analyzed_count: 0,
            });
        }

        let sample_count: u64 = conn
            .query_row(
                "SELECT COUNT(*) FROM audio_samples WHERE scan_id = ?1",
                params![sid],
                |row| row.get::<_, i64>(0).map(|v| v as u64),
            )
            .map_err(|e| e.to_string())?;

        let total_bytes: u64 = conn
            .query_row(
                "SELECT COALESCE(SUM(size), 0) FROM audio_samples WHERE scan_id = ?1",
                params![sid],
                |row| row.get::<_, i64>(0).map(|v| v as u64),
            )
            .map_err(|e| e.to_string())?;

        let mut format_counts = HashMap::new();
        let mut stmt = conn
            .prepare(
                "SELECT format, COUNT(*) FROM audio_samples WHERE scan_id = ?1 GROUP BY format",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![sid], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? as u64))
            })
            .map_err(|e| e.to_string())?;
        for (fmt, count) in rows.flatten() {
            format_counts.insert(fmt, count);
        }

        let analyzed_count: u64 = conn
            .query_row(
                "SELECT COUNT(*) FROM audio_samples WHERE scan_id = ?1 AND bpm IS NOT NULL",
                params![sid],
                |row| row.get::<_, i64>(0).map(|v| v as u64),
            )
            .map_err(|e| e.to_string())?;

        Ok(AudioStatsResult {
            sample_count,
            total_bytes,
            format_counts,
            analyzed_count,
        })
    }

    /// DAW aggregate stats. `scan_id` None or empty → full library (deduped by path).
    pub fn daw_stats(&self, scan_id: Option<&str>) -> Result<DawStatsResult, String> {
        let conn = self.read_conn();
        let library = scan_id.map(|s| s.is_empty()).unwrap_or(true);
        if library {
            let project_count: u64 = self.daw_library_total_rows(&conn)?;
            let total_bytes: u64 = conn
                .query_row(
                    "SELECT COALESCE(SUM(s.size), 0) FROM daw_projects s INNER JOIN daw_library lib ON s.id = lib.project_id",
                    [],
                    |row| row.get::<_, i64>(0).map(|v| v as u64),
                )
                .unwrap_or(0);
            let mut daw_counts = HashMap::new();
            let mut stmt = conn
                .prepare(
                    "SELECT s.daw, COUNT(*) FROM daw_projects s INNER JOIN daw_library lib ON s.id = lib.project_id GROUP BY s.daw",
                )
                .map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map([], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? as u64))
                })
                .map_err(|e| e.to_string())?;
            for (daw, count) in rows.flatten() {
                daw_counts.insert(daw, count);
            }
            return Ok(DawStatsResult {
                project_count,
                total_bytes,
                daw_counts,
            });
        }

        let sid = scan_id.expect("scan_id").to_string();
        if sid.is_empty() {
            return Ok(DawStatsResult {
                project_count: 0,
                total_bytes: 0,
                daw_counts: HashMap::new(),
            });
        }
        let project_count: u64 = conn
            .query_row(
                "SELECT COUNT(*) FROM daw_projects WHERE scan_id = ?1",
                params![sid],
                |row| row.get::<_, i64>(0).map(|v| v as u64),
            )
            .map_err(|e| e.to_string())?;
        let total_bytes: u64 = conn
            .query_row(
                "SELECT COALESCE(SUM(size), 0) FROM daw_projects WHERE scan_id = ?1",
                params![sid],
                |row| row.get::<_, i64>(0).map(|v| v as u64),
            )
            .map_err(|e| e.to_string())?;
        let mut daw_counts = HashMap::new();
        let mut stmt = conn
            .prepare("SELECT daw, COUNT(*) FROM daw_projects WHERE scan_id = ?1 GROUP BY daw")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![sid], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? as u64))
            })
            .map_err(|e| e.to_string())?;
        for (daw, count) in rows.flatten() {
            daw_counts.insert(daw, count);
        }
        Ok(DawStatsResult {
            project_count,
            total_bytes,
            daw_counts,
        })
    }

    /// Preset aggregate stats. `scan_id` None or empty → full library (deduped by path). MIDI excluded.
    pub fn preset_stats(&self, scan_id: Option<&str>) -> Result<PresetStatsResult, String> {
        let conn = self.read_conn();
        let library = scan_id.map(|s| s.is_empty()).unwrap_or(true);
        if library {
            let preset_count: u64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM presets WHERE id IN (SELECT preset_id FROM preset_library) AND format NOT IN ('MID', 'MIDI')",
                    [],
                    |row| row.get::<_, i64>(0).map(|v| v as u64),
                )
                .unwrap_or(0);
            let total_bytes: u64 = conn
                .query_row(
                    "SELECT COALESCE(SUM(size), 0) FROM presets WHERE id IN (SELECT preset_id FROM preset_library) AND format NOT IN ('MID', 'MIDI')",
                    [],
                    |row| row.get::<_, i64>(0).map(|v| v as u64),
                )
                .unwrap_or(0);
            let mut format_counts = HashMap::new();
            let mut stmt = conn
                .prepare(
                    "SELECT format, COUNT(*) FROM presets WHERE id IN (SELECT preset_id FROM preset_library) AND format NOT IN ('MID', 'MIDI') GROUP BY format",
                )
                .map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map([], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? as u64))
                })
                .map_err(|e| e.to_string())?;
            for (fmt, count) in rows.flatten() {
                format_counts.insert(fmt, count);
            }
            return Ok(PresetStatsResult {
                preset_count,
                total_bytes,
                format_counts,
            });
        }

        let sid = scan_id.expect("scan_id").to_string();
        if sid.is_empty() {
            return Ok(PresetStatsResult {
                preset_count: 0,
                total_bytes: 0,
                format_counts: HashMap::new(),
            });
        }
        let preset_count: u64 = conn
            .query_row(
                "SELECT COUNT(*) FROM presets WHERE scan_id = ?1 AND format NOT IN ('MID', 'MIDI')",
                params![sid],
                |row| row.get::<_, i64>(0).map(|v| v as u64),
            )
            .map_err(|e| e.to_string())?;
        let total_bytes: u64 = conn
            .query_row(
                "SELECT COALESCE(SUM(size), 0) FROM presets WHERE scan_id = ?1 AND format NOT IN ('MID', 'MIDI')",
                params![sid],
                |row| row.get::<_, i64>(0).map(|v| v as u64),
            )
            .map_err(|e| e.to_string())?;
        let mut format_counts = HashMap::new();
        let mut stmt = conn
            .prepare(
                "SELECT format, COUNT(*) FROM presets WHERE scan_id = ?1 AND format NOT IN ('MID', 'MIDI') GROUP BY format",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![sid], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? as u64))
            })
            .map_err(|e| e.to_string())?;
        for (fmt, count) in rows.flatten() {
            format_counts.insert(fmt, count);
        }
        Ok(PresetStatsResult {
            preset_count,
            total_bytes,
            format_counts,
        })
    }

    /// Update BPM for a sample (all rows for that path).
    pub fn update_bpm(&self, path: &str, bpm: Option<f64>) -> Result<(), String> {
        let path = normalize_path_for_db(path);
        let conn = self.read_conn();
        conn.execute(
            "UPDATE audio_samples SET bpm = ?1, bpm_exhausted = 0 WHERE path = ?2",
            params![bpm, path],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Update musical key for a sample.
    pub fn update_key(&self, path: &str, key: Option<&str>) -> Result<(), String> {
        let path = normalize_path_for_db(path);
        let conn = self.read_conn();
        conn.execute(
            "UPDATE audio_samples SET key_name = ?1 WHERE path = ?2",
            params![key, path],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Update core audio metadata (duration, channels, sample_rate, bits_per_sample) for a sample.
    pub fn update_audio_meta(
        &self,
        path: &str,
        duration: Option<f64>,
        channels: Option<u16>,
        sample_rate: Option<u32>,
        bits_per_sample: Option<u16>,
    ) -> Result<(), String> {
        let path = normalize_path_for_db(path);
        let conn = self.read_conn();
        conn.execute(
            "UPDATE audio_samples SET duration = ?1, channels = ?2, sample_rate = ?3, bits_per_sample = ?4
             WHERE path = ?5",
            params![
                duration,
                channels.map(|v| v as i32),
                sample_rate.map(|v| v as i32),
                bits_per_sample.map(|v| v as i32),
                path
            ],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Get paths that are missing duration metadata (among the given paths).
    pub fn paths_missing_audio_meta(&self, paths: &[String]) -> Result<Vec<String>, String> {
        let conn = self.read_conn();
        if paths.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders: Vec<&str> = paths.iter().map(|_| "?").collect();
        let sql = format!(
            "SELECT path FROM audio_samples WHERE duration IS NULL AND path IN ({})",
            placeholders.join(",")
        );
        let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
        let mut idx = 1;
        for p in paths {
            let p = normalize_path_for_db(p);
            stmt.raw_bind_parameter(idx, p).map_err(|e| e.to_string())?;
            idx += 1;
        }
        let mut result = Vec::new();
        let mut rows = stmt.raw_query();
        while let Some(row) = rows.next().map_err(|e| e.to_string())? {
            result.push(row.get::<_, String>(0).unwrap_or_default());
        }
        Ok(result)
    }

    /// Update LUFS for a sample (all rows for that path).
    pub fn update_lufs(&self, path: &str, lufs: Option<f64>) -> Result<(), String> {
        let path = normalize_path_for_db(path);
        let conn = self.read_conn();
        conn.execute(
            "UPDATE audio_samples SET lufs = ?1 WHERE path = ?2",
            params![lufs, path],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Get analysis data for a single sample.
    pub fn get_analysis(&self, path: &str) -> Result<serde_json::Value, String> {
        let path = normalize_path_for_db(path);
        let conn = self.read_conn();
        let result = conn
            .query_row(
                &format!(
                    "SELECT bpm, key_name, lufs, duration, channels, sample_rate, bits_per_sample
                 FROM audio_samples WHERE path = ?1 AND ({AUDIO_LIBRARY_IDS})"
                ),
                params![path],
                |row| {
                    Ok(serde_json::json!({
                        "bpm": row.get::<_, Option<f64>>(0)?,
                        "key": row.get::<_, Option<String>>(1)?,
                        "lufs": row.get::<_, Option<f64>>(2)?,
                        "duration": row.get::<_, Option<f64>>(3)?,
                        "channels": row.get::<_, Option<i32>>(4)?,
                        "sampleRate": row.get::<_, Option<i32>>(5)?,
                        "bitsPerSample": row.get::<_, Option<i32>>(6)?,
                    }))
                },
            )
            .optional()
            .map_err(|e| e.to_string())?;
        Ok(result.unwrap_or(serde_json::json!({})))
    }

    /// Get paths of samples that still need BPM/Key/LUFS analysis (library rows).
    ///
    /// Rows where key and LUFS are filled but BPM is still NULL after a batch run are marked
    /// [`bpm_exhausted`](Self::batch_update_analysis) so we do not spin on the same paths forever
    /// when tempo detection fails. Clearing the BPM cache resets `bpm_exhausted` so analysis can retry.
    pub fn unanalyzed_paths(&self, limit: u64) -> Result<Vec<String>, String> {
        let conn = self.read_conn();
        let mut stmt = conn
            .prepare(&format!(
                "SELECT path FROM audio_samples
                 WHERE (
                     key_name IS NULL OR lufs IS NULL
                     OR (bpm IS NULL AND bpm_exhausted = 0)
                 )
                 AND ({AUDIO_LIBRARY_IDS})
                 LIMIT ?1"
            ))
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![limit as i64], |row| row.get(0))
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<String>, _>>()
            .map_err(|e| e.to_string())
    }

    /// All canonical `path` values in the audio library (one row per path via `audio_library`).
    pub fn audio_library_paths(&self) -> Result<Vec<String>, String> {
        let conn = self.read_conn();
        let mut stmt = conn
            .prepare("SELECT path FROM audio_library ORDER BY path")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<String>, _>>()
            .map_err(|e| e.to_string())
    }

    /// Delete a scan and its samples.
    pub fn delete_scan(&self, scan_id: &str) -> Result<(), String> {
        let conn = self.read_conn();
        conn.execute(
            "CREATE TEMP TABLE _al_refresh_paths (path TEXT PRIMARY KEY)",
            [],
        )
        .map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT INTO _al_refresh_paths SELECT DISTINCT path FROM audio_samples WHERE scan_id = ?1",
            params![scan_id],
        )
        .map_err(|e| e.to_string())?;
        conn.execute(
            "DELETE FROM audio_samples WHERE scan_id = ?1",
            params![scan_id],
        )
        .map_err(|e| e.to_string())?;
        conn.execute(
            "DELETE FROM audio_samples_fts WHERE scan_id = ?1",
            params![scan_id],
        )
        .map_err(|e| e.to_string())?;
        Self::sync_audio_library_after_paths_refresh(&conn)?;
        self.invalidate_audio_library_total_cache();
        conn.execute("DELETE FROM audio_scans WHERE id = ?1", params![scan_id])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    // ── Plugin scan CRUD ──

    // ── Paginated plugin query ──
    pub fn query_plugins(
        &self,
        search: Option<&str>,
        type_filter: Option<&str>,
        status_filter: Option<&str>,
        sort_key: &str,
        sort_asc: bool,
        search_regex: bool,
        offset: u64,
        limit: u64,
    ) -> Result<PluginQueryResult, String> {
        let conn = self.read_conn();
        let total_unfiltered: u64 = self.plugin_library_total_rows(&conn)?;
        if total_unfiltered == 0 {
            return Ok(PluginQueryResult {
                plugins: vec![],
                total_count: 0,
                total_count_capped: false,
                total_unfiltered: 0,
            });
        }

        let statuses = parse_plugin_status_filter(status_filter);
        let use_kvr_join = statuses.is_some();
        let q = if use_kvr_join { "plugins." } else { "" };
        let id_clause = if use_kvr_join {
            PLUGIN_LIBRARY_IDS_QUALIFIED
        } else {
            PLUGIN_LIBRARY_IDS
        };
        let from_sql = if use_kvr_join {
            "FROM plugins LEFT JOIN kvr_cache k ON k.plugin_key = (lower(coalesce(nullif(trim(plugins.manufacturer), ''), 'Unknown')) || '|||' || lower(plugins.name))"
        } else {
            "FROM plugins"
        };
        let select_cols = if use_kvr_join {
            "SELECT plugins.name, plugins.path, plugins.plugin_type, plugins.version, plugins.manufacturer, plugins.manufacturer_url, plugins.size, plugins.size_bytes, plugins.modified, plugins.architectures"
        } else {
            "SELECT name, path, plugin_type, version, manufacturer, manufacturer_url, size, size_bytes, modified, architectures"
        };

        let mut where_parts = vec![id_clause.to_string()];
        let mut bind_idx = 1usize;
        let (regex_pat, like_pat) = classify_plugins_search(search, search_regex);
        if regex_pat.is_some() {
            where_parts.push(format!(
                "({q}name REGEXP ?{bind_idx} OR {q}manufacturer REGEXP ?{bind_idx} OR {q}path REGEXP ?{bind_idx})"
            ));
            bind_idx += 1;
        } else if like_pat.is_some() {
            where_parts.push(format!(
                "({q}name LIKE ?{bind_idx} ESCAPE '\\' OR {q}manufacturer LIKE ?{bind_idx} ESCAPE '\\' OR {q}path LIKE ?{bind_idx} ESCAPE '\\')"
            ));
            bind_idx += 1;
        }
        if let Some(tf) = type_filter
            && !tf.is_empty() && tf != "all" {
                if tf.contains(',') {
                    let vals: Vec<String> = tf
                        .split(',')
                        .map(|s| format!("'{}'", s.trim().replace('\'', "''")))
                        .collect();
                    where_parts.push(format!("{q}plugin_type IN ({})", vals.join(",")));
                } else {
                    where_parts.push(format!("{q}plugin_type = ?{bind_idx}"));
                }
            }
        if let Some(ref st) = statuses {
            let parts: Vec<String> = st
                .iter()
                .map(|s| match *s {
                    "update" => "(k.has_update = 1)".to_string(),
                    "current" => "(k.plugin_key IS NOT NULL AND k.has_update = 0 AND COALESCE(k.source, '') != 'not-found')"
                        .to_string(),
                    "unknown" => "(k.plugin_key IS NULL OR (k.has_update = 0 AND k.source = 'not-found'))"
                        .to_string(),
                    _ => String::new(),
                })
                .filter(|s| !s.is_empty())
                .collect();
            if !parts.is_empty() {
                where_parts.push(format!("({})", parts.join(" OR ")));
            }
        }
        let where_cl = where_parts.join(" AND ");

        let sort_col = match sort_key {
            "name" => format!("{q}name COLLATE NOCASE"),
            "type" => format!("{q}plugin_type"),
            "version" => format!("{q}version"),
            "manufacturer" => format!("{q}manufacturer COLLATE NOCASE"),
            "size" => format!("{q}size_bytes"),
            "modified" => format!("{q}modified"),
            _ => format!("{q}name COLLATE NOCASE"),
        };
        let dir = if sort_asc { "ASC" } else { "DESC" };

        let (total_count, total_count_capped) = if regex_pat.is_some() || like_pat.is_some() {
            Self::plugin_search_bounded_count_library(
                &conn,
                from_sql,
                &where_cl,
                &regex_pat,
                &like_pat,
                type_filter,
            )?
        } else {
            let n: u64 = {
                let sql = format!("SELECT COUNT(*) {from_sql} WHERE {where_cl}");
                let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
                let mut bi = 1;
                if let Some(ref p) = regex_pat.as_ref().or(like_pat.as_ref()) {
                    stmt.raw_bind_parameter(bi, p).map_err(|e| e.to_string())?;
                    bi += 1;
                }
                if let Some(tf) = type_filter
                    && !tf.is_empty() && tf != "all" && !tf.contains(',') {
                        stmt.raw_bind_parameter(bi, tf).map_err(|e| e.to_string())?;
                    }
                let _ = bi;
                let mut rows = stmt.raw_query();
                rows.next()
                    .map_err(|e| e.to_string())?
                    .map(|r| r.get::<_, i64>(0).unwrap_or(0) as u64)
                    .unwrap_or(0)
            };
            (n, false)
        };

        let mut bind_offset = 1usize;
        if regex_pat.is_some() || like_pat.is_some() {
            bind_offset += 1;
        }
        if type_filter
            .map(|t| !t.is_empty() && t != "all" && !t.contains(','))
            .unwrap_or(false)
        {
            bind_offset += 1;
        }
        let sql = format!(
            "{select_cols} {from_sql} WHERE {where_cl} ORDER BY {sort_col} {dir} LIMIT ?{bind_offset} OFFSET ?{}",
            bind_offset + 1
        );
        let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
        let mut bi = 1;
        if let Some(ref p) = regex_pat.as_ref().or(like_pat.as_ref()) {
            stmt.raw_bind_parameter(bi, p).map_err(|e| e.to_string())?;
            bi += 1;
        }
        if let Some(tf) = type_filter
            && !tf.is_empty() && tf != "all" && !tf.contains(',') {
                stmt.raw_bind_parameter(bi, tf).map_err(|e| e.to_string())?;
                bi += 1;
            }
        stmt.raw_bind_parameter(bi, limit as i64)
            .map_err(|e| e.to_string())?;
        bi += 1;
        stmt.raw_bind_parameter(bi, offset as i64)
            .map_err(|e| e.to_string())?;

        let mut plugins = Vec::new();
        let mut rows = stmt.raw_query();
        while let Some(row) = rows.next().map_err(|e| e.to_string())? {
            let arch_str: String = row.get(9).unwrap_or_default();
            plugins.push(PluginRow {
                name: row.get(0).unwrap_or_default(),
                path: row.get(1).unwrap_or_default(),
                plugin_type: row.get(2).unwrap_or_default(),
                version: row.get(3).unwrap_or_default(),
                manufacturer: row.get(4).unwrap_or_default(),
                manufacturer_url: row.get(5).ok(),
                size: row.get(6).unwrap_or_default(),
                size_bytes: row.get::<_, i64>(7).unwrap_or(0) as u64,
                modified: row.get(8).unwrap_or_default(),
                architectures: serde_json::from_str(&arch_str).unwrap_or_default(),
            });
        }
        Ok(PluginQueryResult {
            plugins,
            total_count,
            total_count_capped,
            total_unfiltered,
        })
    }

    // ── Paginated DAW query ──
    /// Full library (deduped by path). Same pattern as `query_audio` without `scan_id`.
    pub fn query_daw(
        &self,
        search: Option<&str>,
        daw_filter: Option<&str>,
        sort_key: &str,
        sort_asc: bool,
        search_regex: bool,
        offset: u64,
        limit: u64,
    ) -> Result<DawQueryResult, String> {
        let conn = self.read_conn();
        let total_unfiltered: u64 = self.daw_library_total_rows(&conn)?;
        if total_unfiltered == 0 {
            return Ok(DawQueryResult {
                projects: vec![],
                total_count: 0,
                total_count_capped: false,
                total_unfiltered: 0,
            });
        }

        let mut where_parts = vec![DAW_LIBRARY_IDS.to_string()];
        let mut bind_idx = 1usize;
        let (fts_match, like_pat, regex_pat) = classify_fts_name_path_search(search, search_regex);
        if fts_match.is_some() {
            // Library scope is already `DAW_LIBRARY_IDS`; do not nest a second
            // `MAX(id) GROUP BY path` inside the FTS subquery (same semantics, worse plan).
            where_parts.push(format!(
                "id IN (SELECT rowid FROM daw_projects_fts WHERE daw_projects_fts MATCH ?{bind_idx})",
            ));
            bind_idx += 1;
        } else if regex_pat.is_some() {
            where_parts.push(format!(
                "((name REGEXP ?{bind_idx}) OR (path REGEXP ?{bind_idx}))"
            ));
            bind_idx += 1;
        } else if like_pat.is_some() {
            where_parts.push(format!(
                "(name LIKE ?{bind_idx} ESCAPE '\\' OR path LIKE ?{bind_idx} ESCAPE '\\')"
            ));
            bind_idx += 1;
        }
        if let Some(f) = daw_filter
            && !f.is_empty() && f != "all" {
                if f.contains(',') {
                    let vals: Vec<String> = f
                        .split(',')
                        .map(|s| format!("'{}'", s.trim().replace('\'', "''")))
                        .collect();
                    where_parts.push(format!("daw IN ({})", vals.join(",")));
                } else {
                    where_parts.push(format!("daw = ?{bind_idx}"));
                    bind_idx += 1;
                }
            }
        let where_cl = where_parts.join(" AND ");

        let sort_col = match sort_key {
            "name" => "name COLLATE NOCASE",
            "daw" => "daw",
            "format" => "format",
            "size" => "size",
            "modified" => "modified",
            "directory" => "directory COLLATE NOCASE",
            _ => "name COLLATE NOCASE",
        };
        let dir = if sort_asc { "ASC" } else { "DESC" };

        // FTS substring search: ORDER BY column sorts the entire match set before LIMIT (stalls).
        // Use bm25 + LIMIT like MIDI/PDF; frontend may re-rank with `searchScore`.
        let use_fts_rank_page = fts_match.is_some();

        let (total_count, total_count_capped) = if let Some(m) = fts_match.as_ref() {
            Self::daw_fts_bounded_count_library(&conn, m, daw_filter)?
        } else {
            let n: u64 = {
                let sql = format!("SELECT COUNT(*) FROM daw_projects WHERE {where_cl}");
                let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
                let mut bi = 1;
                if let Some(ref r) = regex_pat {
                    stmt.raw_bind_parameter(bi, r).map_err(|e| e.to_string())?;
                    bi += 1;
                } else if let Some(ref pat) = like_pat {
                    stmt.raw_bind_parameter(bi, pat)
                        .map_err(|e| e.to_string())?;
                    bi += 1;
                }
                if let Some(f) = daw_filter
                    && !f.is_empty() && f != "all" && !f.contains(',') {
                        stmt.raw_bind_parameter(bi, f).map_err(|e| e.to_string())?;
                    }
                let mut rows = stmt.raw_query();
                rows.next()
                    .map_err(|e| e.to_string())?
                    .map(|r| r.get::<_, i64>(0).unwrap_or(0) as u64)
                    .unwrap_or(0)
            };
            (n, false)
        };

        let sql = if use_fts_rank_page {
            let mut w = String::from(
                "SELECT d.name, d.path, d.directory, d.format, d.daw, d.size, d.size_formatted, d.modified
                 FROM daw_projects_fts
                 INNER JOIN daw_projects d ON d.id = daw_projects_fts.rowid
                 INNER JOIN daw_library lib ON lib.project_id = d.id
                 WHERE daw_projects_fts MATCH ?1",
            );
            let mut next_ph = 2usize;
            if let Some(f) = daw_filter
                && !f.is_empty() && f != "all" {
                    if f.contains(',') {
                        let vals: Vec<String> = f
                            .split(',')
                            .map(|s| format!("'{}'", s.trim().replace('\'', "''")))
                            .collect();
                        w.push_str(&format!(" AND d.daw IN ({})", vals.join(",")));
                    } else {
                        w.push_str(&format!(" AND d.daw = ?{next_ph}"));
                        next_ph += 1;
                    }
                }
            let li = next_ph;
            let oi = next_ph + 1;
            w.push_str(&format!(
                " ORDER BY bm25(daw_projects_fts) LIMIT ?{li} OFFSET ?{oi}"
            ));
            w
        } else {
            format!(
                "SELECT name, path, directory, format, daw, size, size_formatted, modified FROM daw_projects WHERE {where_cl} ORDER BY {sort_col} {dir} LIMIT ?{bind_idx} OFFSET ?{}",
                bind_idx + 1
            )
        };
        let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
        let mut bi = 1;
        if use_fts_rank_page {
            let m = fts_match.as_ref().expect("fts");
            stmt.raw_bind_parameter(bi, m).map_err(|e| e.to_string())?;
            bi += 1;
            if let Some(f) = daw_filter
                && !f.is_empty() && f != "all" && !f.contains(',') {
                    stmt.raw_bind_parameter(bi, f).map_err(|e| e.to_string())?;
                    bi += 1;
                }
            stmt.raw_bind_parameter(bi, limit as i64)
                .map_err(|e| e.to_string())?;
            stmt.raw_bind_parameter(bi + 1, offset as i64)
                .map_err(|e| e.to_string())?;
        } else if let Some(ref r) = regex_pat {
            stmt.raw_bind_parameter(bi, r).map_err(|e| e.to_string())?;
            bi += 1;
            if let Some(f) = daw_filter
                && !f.is_empty() && f != "all" && !f.contains(',') {
                    stmt.raw_bind_parameter(bi, f).map_err(|e| e.to_string())?;
                    bi += 1;
                }
            stmt.raw_bind_parameter(bi, limit as i64)
                .map_err(|e| e.to_string())?;
            stmt.raw_bind_parameter(bi + 1, offset as i64)
                .map_err(|e| e.to_string())?;
        } else if let Some(ref pat) = like_pat {
            stmt.raw_bind_parameter(bi, pat)
                .map_err(|e| e.to_string())?;
            bi += 1;
            if let Some(f) = daw_filter
                && !f.is_empty() && f != "all" && !f.contains(',') {
                    stmt.raw_bind_parameter(bi, f).map_err(|e| e.to_string())?;
                    bi += 1;
                }
            stmt.raw_bind_parameter(bi, limit as i64)
                .map_err(|e| e.to_string())?;
            stmt.raw_bind_parameter(bi + 1, offset as i64)
                .map_err(|e| e.to_string())?;
        } else {
            if let Some(f) = daw_filter
                && !f.is_empty() && f != "all" && !f.contains(',') {
                    stmt.raw_bind_parameter(bi, f).map_err(|e| e.to_string())?;
                    bi += 1;
                }
            stmt.raw_bind_parameter(bi, limit as i64)
                .map_err(|e| e.to_string())?;
            stmt.raw_bind_parameter(bi + 1, offset as i64)
                .map_err(|e| e.to_string())?;
        }

        let mut projects = Vec::new();
        let mut rows = stmt.raw_query();
        while let Some(row) = rows.next().map_err(|e| e.to_string())? {
            projects.push(DawRow {
                name: row.get(0).unwrap_or_default(),
                path: row.get(1).unwrap_or_default(),
                directory: row.get(2).unwrap_or_default(),
                format: row.get(3).unwrap_or_default(),
                daw: row.get(4).unwrap_or_default(),
                size: row.get::<_, i64>(5).unwrap_or(0) as u64,
                size_formatted: row.get(6).unwrap_or_default(),
                modified: row.get(7).unwrap_or_default(),
            });
        }
        Ok(DawQueryResult {
            projects,
            total_count,
            total_count_capped,
            total_unfiltered,
        })
    }

    // ── Paginated preset query ──
    pub fn query_presets(
        &self,
        search: Option<&str>,
        format_filter: Option<&str>,
        sort_key: &str,
        sort_asc: bool,
        search_regex: bool,
        offset: u64,
        limit: u64,
    ) -> Result<PresetQueryResult, String> {
        let conn = self.read_conn();
        let total_unfiltered: u64 = self.preset_inventory_total_rows(&conn)?;
        if total_unfiltered == 0 {
            return Ok(PresetQueryResult {
                presets: vec![],
                total_count: 0,
                total_count_capped: false,
                total_unfiltered: 0,
            });
        }

        let mut where_parts = vec![
            PRESET_LIBRARY_IDS.to_string(),
            "format NOT IN ('MID', 'MIDI')".to_string(),
        ];
        let mut bind_idx = 1usize;
        let (fts_match, like_pat, regex_pat) = classify_fts_name_path_search(search, search_regex);
        if fts_match.is_some() {
            // Library scope is already `PRESET_LIBRARY_IDS`; do not nest a second
            // `MAX(id) GROUP BY path` inside the FTS subquery (same semantics, worse plan).
            where_parts.push(format!(
                "id IN (SELECT rowid FROM presets_fts WHERE presets_fts MATCH ?{bind_idx})",
            ));
            bind_idx += 1;
        } else if regex_pat.is_some() {
            where_parts.push(format!(
                "((name REGEXP ?{bind_idx}) OR (path REGEXP ?{bind_idx}))"
            ));
            bind_idx += 1;
        } else if like_pat.is_some() {
            where_parts.push(format!(
                "(name LIKE ?{bind_idx} ESCAPE '\\' OR path LIKE ?{bind_idx} ESCAPE '\\')"
            ));
            bind_idx += 1;
        }
        if let Some(f) = format_filter
            && !f.is_empty() && f != "all" {
                if f.contains(',') {
                    let vals: Vec<String> = f
                        .split(',')
                        .map(|s| format!("'{}'", s.trim().replace('\'', "''")))
                        .collect();
                    where_parts.push(format!("format IN ({})", vals.join(",")));
                } else {
                    where_parts.push(format!("format = ?{bind_idx}"));
                    bind_idx += 1;
                }
            }
        let where_cl = where_parts.join(" AND ");

        let sort_col = match sort_key {
            "name" => "name COLLATE NOCASE",
            "format" => "format",
            "size" => "size",
            "modified" => "modified",
            "directory" => "directory COLLATE NOCASE",
            _ => "name COLLATE NOCASE",
        };
        let dir = if sort_asc { "ASC" } else { "DESC" };

        let (total_count, total_count_capped) = if let Some(m) = fts_match.as_ref() {
            Self::preset_fts_bounded_count_library(&conn, m, format_filter)?
        } else {
            let sql = format!("SELECT COUNT(*) FROM presets WHERE {where_cl}");
            let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
            let mut bi = 1;
            if let Some(ref r) = regex_pat {
                stmt.raw_bind_parameter(bi, r).map_err(|e| e.to_string())?;
                bi += 1;
            } else if let Some(ref pat) = like_pat {
                stmt.raw_bind_parameter(bi, pat)
                    .map_err(|e| e.to_string())?;
                bi += 1;
            }
            if let Some(f) = format_filter
                && !f.is_empty() && f != "all" && !f.contains(',') {
                    stmt.raw_bind_parameter(bi, f).map_err(|e| e.to_string())?;
                }
            let mut rows = stmt.raw_query();
            let n = rows
                .next()
                .map_err(|e| e.to_string())?
                .map(|r| r.get::<_, i64>(0).unwrap_or(0) as u64)
                .unwrap_or(0);
            (n, false)
        };

        // FTS substring search: ORDER BY column sorts the entire match set before LIMIT (stalls).
        // Use bm25 + LIMIT like PDF/MIDI; frontend may re-rank with `searchScore`.
        let use_fts_bm25 = fts_match.is_some();
        let sql = if use_fts_bm25 {
            let mut w = String::from(
                "SELECT p.name, p.path, p.directory, p.format, p.size, p.size_formatted, p.modified
                 FROM presets_fts
                 INNER JOIN presets p ON p.id = presets_fts.rowid
                 INNER JOIN preset_library lib ON lib.preset_id = p.id
                 WHERE presets_fts MATCH ?1
                 AND p.format NOT IN ('MID','MIDI')",
            );
            let mut next_ph = 2usize;
            if let Some(f) = format_filter
                && !f.is_empty() && f != "all" {
                    if f.contains(',') {
                        let vals: Vec<String> = f
                            .split(',')
                            .map(|s| format!("'{}'", s.trim().replace('\'', "''")))
                            .collect();
                        w.push_str(&format!(" AND p.format IN ({})", vals.join(",")));
                    } else {
                        w.push_str(&format!(" AND p.format = ?{next_ph}"));
                        next_ph += 1;
                    }
                }
            let plim = next_ph;
            let poff = next_ph + 1;
            w.push_str(&format!(
                " ORDER BY bm25(presets_fts) LIMIT ?{plim} OFFSET ?{poff}"
            ));
            w
        } else {
            format!(
                "SELECT name, path, directory, format, size, size_formatted, modified FROM presets WHERE {where_cl} ORDER BY {sort_col} {dir} LIMIT ?{bind_idx} OFFSET ?{}",
                bind_idx + 1
            )
        };
        let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
        let mut bi = 1;
        if use_fts_bm25 {
            let m = fts_match.as_ref().expect("fts");
            stmt.raw_bind_parameter(bi, m).map_err(|e| e.to_string())?;
            bi += 1;
            if let Some(f) = format_filter
                && !f.is_empty() && f != "all" && !f.contains(',') {
                    stmt.raw_bind_parameter(bi, f).map_err(|e| e.to_string())?;
                    bi += 1;
                }
            stmt.raw_bind_parameter(bi, limit as i64)
                .map_err(|e| e.to_string())?;
            stmt.raw_bind_parameter(bi + 1, offset as i64)
                .map_err(|e| e.to_string())?;
        } else if let Some(ref r) = regex_pat {
            stmt.raw_bind_parameter(bi, r).map_err(|e| e.to_string())?;
            bi += 1;
            if let Some(f) = format_filter
                && !f.is_empty() && f != "all" && !f.contains(',') {
                    stmt.raw_bind_parameter(bi, f).map_err(|e| e.to_string())?;
                    bi += 1;
                }
            stmt.raw_bind_parameter(bi, limit as i64)
                .map_err(|e| e.to_string())?;
            stmt.raw_bind_parameter(bi + 1, offset as i64)
                .map_err(|e| e.to_string())?;
        } else if let Some(ref pat) = like_pat {
            stmt.raw_bind_parameter(bi, pat)
                .map_err(|e| e.to_string())?;
            bi += 1;
            if let Some(f) = format_filter
                && !f.is_empty() && f != "all" && !f.contains(',') {
                    stmt.raw_bind_parameter(bi, f).map_err(|e| e.to_string())?;
                    bi += 1;
                }
            stmt.raw_bind_parameter(bi, limit as i64)
                .map_err(|e| e.to_string())?;
            stmt.raw_bind_parameter(bi + 1, offset as i64)
                .map_err(|e| e.to_string())?;
        } else {
            if let Some(f) = format_filter
                && !f.is_empty() && f != "all" && !f.contains(',') {
                    stmt.raw_bind_parameter(bi, f).map_err(|e| e.to_string())?;
                    bi += 1;
                }
            stmt.raw_bind_parameter(bi, limit as i64)
                .map_err(|e| e.to_string())?;
            stmt.raw_bind_parameter(bi + 1, offset as i64)
                .map_err(|e| e.to_string())?;
        }

        let mut presets = Vec::new();
        let mut rows = stmt.raw_query();
        while let Some(row) = rows.next().map_err(|e| e.to_string())? {
            presets.push(PresetRow {
                name: row.get(0).unwrap_or_default(),
                path: row.get(1).unwrap_or_default(),
                directory: row.get(2).unwrap_or_default(),
                format: row.get(3).unwrap_or_default(),
                size: row.get::<_, i64>(4).unwrap_or(0) as u64,
                size_formatted: row.get(5).unwrap_or_default(),
                modified: row.get(6).unwrap_or_default(),
            });
        }
        Ok(PresetQueryResult {
            presets,
            total_count,
            total_count_capped,
            total_unfiltered,
        })
    }

    pub fn save_plugin_scan(&self, snap: &ScanSnapshot) -> Result<(), String> {
        let conn = self.read_conn();
        let dirs_json = path_strings_json_normalized(&snap.directories);
        let roots_json = path_strings_json_normalized(&snap.roots);
        conn.execute(
            "INSERT OR REPLACE INTO plugin_scans (id, timestamp, plugin_count, directories, roots, scan_complete) VALUES (?1,?2,?3,?4,?5,1)",
            params![snap.id, snap.timestamp, snap.plugin_count as i64, dirs_json, roots_json],
        ).map_err(|e| e.to_string())?;
        let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
        {
            // Delete old plugins for this scan_id first
            tx.execute("DELETE FROM plugins WHERE scan_id = ?1", params![snap.id])
                .map_err(|e| e.to_string())?;
            let mut stmt = tx.prepare_cached(
                "INSERT OR REPLACE INTO plugins (name, path, plugin_type, version, manufacturer, manufacturer_url, size, size_bytes, modified, architectures, scan_id) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)"
            ).map_err(|e| e.to_string())?;
            for p in &snap.plugins {
                let arch_json = serde_json::to_string(&p.architectures).unwrap_or_default();
                let path = normalize_path_for_db(&p.path);
                stmt.execute(params![
                    p.name,
                    path,
                    p.plugin_type,
                    p.version,
                    p.manufacturer,
                    p.manufacturer_url,
                    p.size,
                    p.size_bytes as i64,
                    p.modified,
                    arch_json,
                    snap.id
                ])
                .map_err(|e| e.to_string())?;
            }
        }
        tx.commit().map_err(|e| e.to_string())?;
        Self::rebuild_plugin_library(&conn)?;
        self.invalidate_plugin_library_total_cache();
        Ok(())
    }

    /// Begin a streaming plugin scan: parent row + clear prior rows for this id.
    pub fn plugin_scan_parent_create(
        &self,
        id: &str,
        timestamp: &str,
        roots: &[String],
    ) -> Result<(), String> {
        let conn = self.read_conn();
        let dirs_json = "[]".to_string();
        let roots_json = path_strings_json_normalized(roots);
        conn.execute(
            "INSERT OR REPLACE INTO plugin_scans (id, timestamp, plugin_count, directories, roots, scan_complete) VALUES (?1,?2,0,?3,?4,0)",
            params![id, timestamp, dirs_json, roots_json],
        )
        .map_err(|e| e.to_string())?;
        conn.execute(
            "CREATE TEMP TABLE _pl_refresh_paths (path TEXT PRIMARY KEY)",
            [],
        )
        .map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT INTO _pl_refresh_paths SELECT DISTINCT path FROM plugins WHERE scan_id = ?1",
            params![id],
        )
        .map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM plugins WHERE scan_id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        Self::sync_plugin_library_after_paths_refresh(&conn)?;
        self.invalidate_plugin_library_total_cache();
        Ok(())
    }

    /// Append plugins for a streaming scan; updates `plugin_scans.plugin_count` incrementally.
    pub fn insert_plugin_batch(&self, scan_id: &str, batch: &[PluginInfo]) -> Result<u64, String> {
        let conn = self.read_conn();
        let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
        let mut inserted: u64 = 0;
        {
            let mut stmt = tx
                .prepare_cached(
                    "INSERT OR IGNORE INTO plugins (name, path, plugin_type, version, manufacturer, manufacturer_url, size, size_bytes, modified, architectures, scan_id) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
                )
                .map_err(|e| e.to_string())?;
            let mut lib_stmt = tx
                .prepare_cached(
                    "INSERT INTO plugin_library (path, plugin_id) VALUES (?1, ?2)
                     ON CONFLICT(path) DO UPDATE SET plugin_id = CASE
                       WHEN excluded.plugin_id > plugin_library.plugin_id THEN excluded.plugin_id
                       ELSE plugin_library.plugin_id END",
                )
                .map_err(|e| e.to_string())?;
            for p in batch {
                let arch_json = serde_json::to_string(&p.architectures).unwrap_or_default();
                let path = normalize_path_for_db(&p.path);
                let changed = stmt
                    .execute(params![
                        p.name,
                        path.clone(),
                        p.plugin_type,
                        p.version,
                        p.manufacturer,
                        p.manufacturer_url,
                        p.size,
                        p.size_bytes as i64,
                        p.modified,
                        arch_json,
                        scan_id
                    ])
                    .map_err(|e| e.to_string())?;
                if changed > 0 {
                    let row_id = tx.last_insert_rowid();
                    lib_stmt
                        .execute(params![path, row_id])
                        .map_err(|e| e.to_string())?;
                    inserted += 1;
                }
            }
        }
        if inserted > 0 {
            tx.execute(
                "UPDATE plugin_scans SET plugin_count = plugin_count + ?2 WHERE id = ?1",
                params![scan_id, inserted as i64],
            )
            .map_err(|e| e.to_string())?;
        }
        tx.commit().map_err(|e| e.to_string())?;
        if inserted > 0 {
            self.invalidate_plugin_library_total_cache();
        }
        Ok(inserted)
    }

    /// Finalize directory list and counts after streaming inserts (matches non-streaming snapshot shape).
    pub fn plugin_scan_parent_finalize(
        &self,
        id: &str,
        _plugin_count: usize,
        directories: &[String],
        roots: &[String],
    ) -> Result<(), String> {
        let conn = self.read_conn();
        let plugin_count: i64 = self
            .plugin_library_total_rows(&conn)?
            .try_into()
            .unwrap_or(i64::MAX);
        let dirs_json = path_strings_json_normalized(directories);
        let roots_json = path_strings_json_normalized(roots);
        conn.execute(
            "UPDATE plugin_scans SET plugin_count = ?2, directories = ?3, roots = ?4 WHERE id = ?1",
            params![id, plugin_count, dirs_json, roots_json],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn get_plugin_scans(&self) -> Result<Vec<serde_json::Value>, String> {
        let conn = self.read_conn();
        let mut stmt = conn.prepare(
            "SELECT s.id, s.timestamp, COALESCE(NULLIF(s.plugin_count,0),(SELECT COUNT(*) FROM plugins WHERE scan_id = s.id)), s.roots FROM plugin_scans s WHERE s.scan_complete = 1 ORDER BY s.timestamp DESC",
        )
        .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| {
                let roots_str: String = row.get(3)?;
                Ok(serde_json::json!({
                    "id": row.get::<_,String>(0)?,
                    "timestamp": row.get::<_,String>(1)?,
                    "pluginCount": row.get::<_, i64>(2)? as u64,
                    "roots": serde_json::from_str::<Vec<String>>(&roots_str).unwrap_or_default(),
                }))
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    }

    pub fn get_plugin_scan_detail(&self, id: &str) -> Result<ScanSnapshot, String> {
        let conn = self.read_conn();
        let (ts, pc, dirs_json, roots_json): (String, usize, String, String) = conn.query_row(
            "SELECT timestamp, plugin_count, directories, roots FROM plugin_scans WHERE id = ?1",
            params![id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get::<_, i64>(1)? as usize,
                    row.get(2)?,
                    row.get(3)?,
                ))
            },
        )
        .map_err(|e| e.to_string())?;
        let mut stmt = conn.prepare("SELECT name, path, plugin_type, version, manufacturer, manufacturer_url, size, size_bytes, modified, architectures FROM plugins WHERE scan_id = ?1").map_err(|e| e.to_string())?;
        let plugins = stmt
            .query_map(params![id], |row| {
                let arch_str: String = row.get(9)?;
                Ok(PluginInfo {
                    name: row.get(0)?,
                    path: row.get(1)?,
                    plugin_type: row.get(2)?,
                    version: row.get(3)?,
                    manufacturer: row.get(4)?,
                    manufacturer_url: row.get(5)?,
                    size: row.get(6)?,
                    size_bytes: row.get::<_, i64>(7).unwrap_or(0) as u64,
                    modified: row.get(8)?,
                    architectures: serde_json::from_str(&arch_str).unwrap_or_default(),
                })
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        Ok(ScanSnapshot {
            id: id.to_string(),
            timestamp: ts,
            plugin_count: pc,
            plugins,
            directories: serde_json::from_str(&dirs_json).unwrap_or_default(),
            roots: serde_json::from_str(&roots_json).unwrap_or_default(),
        })
    }

    pub fn get_latest_plugin_scan(&self) -> Result<Option<ScanSnapshot>, String> {
        let conn = self.read_conn();
        let id: Option<String> = conn
            .query_row(
                "SELECT id FROM plugin_scans WHERE scan_complete = 1 ORDER BY timestamp DESC LIMIT 1",
                [],
                |r| r.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())?;
        drop(conn);
        match id {
            Some(id) => self.get_plugin_scan_detail(&id).map(Some),
            None => Ok(None),
        }
    }

    pub fn delete_plugin_scan(&self, id: &str) -> Result<(), String> {
        let conn = self.read_conn();
        conn.execute(
            "CREATE TEMP TABLE _pl_refresh_paths (path TEXT PRIMARY KEY)",
            [],
        )
        .map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT INTO _pl_refresh_paths SELECT DISTINCT path FROM plugins WHERE scan_id = ?1",
            params![id],
        )
        .map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM plugins WHERE scan_id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM plugin_scans WHERE id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        Self::sync_plugin_library_after_paths_refresh(&conn)?;
        self.invalidate_plugin_library_total_cache();
        Ok(())
    }

    pub fn clear_plugin_history(&self) -> Result<(), String> {
        let conn = self.read_conn();
        conn.execute_batch(
            "BEGIN IMMEDIATE;
             DELETE FROM plugin_library;
             DELETE FROM plugins;
             DELETE FROM plugin_scans;
             COMMIT;",
        )
        .map_err(|e| e.to_string())?;
        self.invalidate_plugin_library_total_cache();
        Ok(())
    }

    // ── Audio scan full CRUD (using existing tables) ──

    pub fn save_audio_scan_full(&self, snap: &AudioScanSnapshot) -> Result<(), String> {
        // Write parent with 0 counts — insert_audio_batch increments live.
        // Finalize afterwards to set the authoritative totals (including format_counts).
        self.save_scan(
            &snap.id,
            &snap.timestamp,
            0,
            0,
            &snap.format_counts,
            &snap.roots,
        )?;
        self.insert_audio_batch(&snap.id, &snap.samples)?;
        self.audio_scan_parent_finalize(
            &snap.id,
            snap.sample_count as u64,
            snap.total_bytes,
            &snap.format_counts,
        )
    }

    pub fn get_audio_scans_list(&self) -> Result<Vec<serde_json::Value>, String> {
        let conn = self.read_conn();
        let mut stmt = conn.prepare(
            "SELECT s.id, s.timestamp, COALESCE(NULLIF(s.sample_count,0),(SELECT COUNT(*) FROM audio_samples WHERE scan_id = s.id)), COALESCE(NULLIF(s.total_bytes,0),(SELECT COALESCE(SUM(size),0) FROM audio_samples WHERE scan_id = s.id)), s.format_counts, s.roots FROM audio_scans s WHERE s.scan_complete = 1 ORDER BY s.timestamp DESC",
        )
        .map_err(|e| e.to_string())?;
        let rows = stmt.query_map([], |row| {
            let fc_str: String = row.get(4)?;
            let roots_str: String = row.get(5)?;
            Ok(serde_json::json!({
                "id": row.get::<_,String>(0)?,
                "timestamp": row.get::<_,String>(1)?,
                "sampleCount": row.get::<_, i64>(2)? as u64,
                "totalBytes": row.get::<_, i64>(3)? as u64,
                "formatCounts": serde_json::from_str::<HashMap<String,usize>>(&fc_str).unwrap_or_default(),
                "roots": serde_json::from_str::<Vec<String>>(&roots_str).unwrap_or_default(),
            }))
        }).map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    }

    pub fn get_audio_scan_detail(&self, id: &str) -> Result<AudioScanSnapshot, String> {
        let conn = self.read_conn();
        let (ts, sc, tb, fc_str, roots_str): (String, usize, u64, String, String) = conn.query_row(
            "SELECT timestamp, sample_count, total_bytes, format_counts, roots FROM audio_scans WHERE id = ?1",
            params![id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get::<_, i64>(1)? as usize,
                    row.get::<_, i64>(2)? as u64,
                    row.get(3)?,
                    row.get(4)?,
                ))
            },
        )
        .map_err(|e| e.to_string())?;
        let mut stmt = conn.prepare("SELECT name, path, directory, format, size, size_formatted, modified, duration, channels, sample_rate, bits_per_sample FROM audio_samples WHERE scan_id = ?1").map_err(|e| e.to_string())?;
        let samples = stmt
            .query_map(params![id], |row| {
                Ok(AudioSample {
                    name: row.get(0)?,
                    path: row.get(1)?,
                    directory: row.get(2)?,
                    format: row.get(3)?,
                    size: row.get::<_, i64>(4).unwrap_or(0) as u64,
                    size_formatted: row.get(5)?,
                    modified: row.get(6)?,
                    duration: row.get(7).ok(),
                    channels: row
                        .get::<_, Option<i32>>(8)
                        .ok()
                        .flatten()
                        .map(|v| v as u16),
                    sample_rate: row
                        .get::<_, Option<i32>>(9)
                        .ok()
                        .flatten()
                        .map(|v| v as u32),
                    bits_per_sample: row
                        .get::<_, Option<i32>>(10)
                        .ok()
                        .flatten()
                        .map(|v| v as u16),
                })
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        // Derive count + total_bytes from child rows so the detail view is
        // correct even when parent_finalize never ran (streaming scan stopped
        // or finalize silently failed).
        let live_count = samples.len();
        let live_bytes: u64 = samples.iter().map(|s| s.size).sum();
        Ok(AudioScanSnapshot {
            id: id.to_string(),
            timestamp: ts,
            sample_count: if sc > 0 { sc } else { live_count },
            total_bytes: if tb > 0 { tb } else { live_bytes },
            format_counts: serde_json::from_str(&fc_str).unwrap_or_default(),
            samples,
            roots: serde_json::from_str(&roots_str).unwrap_or_default(),
        })
    }

    pub fn get_latest_audio_scan(&self) -> Result<Option<AudioScanSnapshot>, String> {
        let conn = self.read_conn();
        let id: Option<String> = conn
            .query_row(
                "SELECT id FROM audio_scans WHERE scan_complete = 1 ORDER BY timestamp DESC LIMIT 1",
                [],
                |r| r.get::<_, String>(0),
            )
            .optional()
            .map_err(|e| e.to_string())?;
        drop(conn);
        match id {
            Some(id) => self.get_audio_scan_detail(&id).map(Some),
            None => Ok(None),
        }
    }

    pub fn delete_audio_scan(&self, id: &str) -> Result<(), String> {
        self.delete_scan(id)
    }

    pub fn clear_audio_history(&self) -> Result<(), String> {
        let conn = self.read_conn();
        conn.execute_batch(
            "BEGIN IMMEDIATE;
             DELETE FROM audio_library;
             DELETE FROM audio_samples_fts;
             DELETE FROM audio_samples;
             DELETE FROM audio_scans;
             COMMIT;",
        )
        .map_err(|e| e.to_string())?;
        self.invalidate_audio_library_total_cache();
        Ok(())
    }

    // ── DAW scan CRUD ──

    /// Create (or re-create) a parent daw_scans row with zero counts. Used by
    /// streaming scans that don't know totals up front.
    pub fn daw_scan_parent_create(
        &self,
        id: &str,
        timestamp: &str,
        roots: &[String],
    ) -> Result<(), String> {
        let conn = self.read_conn();
        let roots_json = path_strings_json_normalized(roots);
        conn.execute(
            "INSERT OR REPLACE INTO daw_scans (id, timestamp, project_count, total_bytes, daw_counts, roots, scan_complete) VALUES (?1,?2,0,0,'{}',?3,0)",
            params![id, timestamp, roots_json],
        ).map_err(|e| e.to_string())?;
        conn.execute(
            "CREATE TEMP TABLE _dl_refresh_paths (path TEXT PRIMARY KEY)",
            [],
        )
        .map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT INTO _dl_refresh_paths SELECT DISTINCT path FROM daw_projects WHERE scan_id = ?1",
            params![id],
        )
        .map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM daw_projects WHERE scan_id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        conn.execute(
            "DELETE FROM daw_projects_fts WHERE scan_id = ?1",
            params![id],
        )
        .map_err(|e| e.to_string())?;
        Self::sync_daw_library_after_paths_refresh(&conn)?;
        self.invalidate_daw_library_total_cache();
        Ok(())
    }

    /// Finalize a parent daw_scans row with aggregate counts after streaming is complete.
    pub fn daw_scan_parent_finalize(
        &self,
        id: &str,
        _project_count: usize,
        _total_bytes: u64,
        _daw_counts: &HashMap<String, usize>,
    ) -> Result<(), String> {
        let conn = self.read_conn();
        let project_count: i64 = self
            .daw_library_total_rows(&conn)?
            .try_into()
            .unwrap_or(i64::MAX);
        let total_bytes: i64 = conn
            .query_row(
                "SELECT COALESCE(SUM(s.size), 0) FROM daw_projects s INNER JOIN daw_library lib ON s.id = lib.project_id",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);
        let mut daw_map: HashMap<String, usize> = HashMap::new();
        let mut stmt = conn
            .prepare(
                "SELECT s.daw, COUNT(*) FROM daw_projects s INNER JOIN daw_library lib ON s.id = lib.project_id GROUP BY s.daw",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? as usize))
            })
            .map_err(|e| e.to_string())?;
        for (d, n) in rows.flatten() {
            daw_map.insert(d, n);
        }
        let dc_json = serde_json::to_string(&daw_map).unwrap_or_default();
        conn.execute(
            "UPDATE daw_scans SET project_count = ?2, total_bytes = ?3, daw_counts = ?4 WHERE id = ?1",
            params![id, project_count, total_bytes, dc_json],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Stream-insert a batch of DawProject rows under an existing scan_id.
    pub fn insert_daw_batch(
        &self,
        scan_id: &str,
        projects: &[DawProject],
    ) -> Result<Vec<usize>, String> {
        let conn = self.read_conn();
        let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
        let mut inserted_idx: Vec<usize> = Vec::new();
        let mut batch_bytes: u64 = 0;
        {
            let mut stmt = tx.prepare_cached("INSERT OR IGNORE INTO daw_projects (name, path, directory, format, daw, size, size_formatted, modified, scan_id) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)").map_err(|e| e.to_string())?;
            let mut fts_stmt = tx.prepare_cached("INSERT INTO daw_projects_fts(rowid, name, path, daw, scan_id) VALUES (?1,?2,?3,?4,?5)").map_err(|e| e.to_string())?;
            let mut lib_stmt = tx
                .prepare_cached(
                    "INSERT INTO daw_library (path, project_id) VALUES (?1, ?2)
                     ON CONFLICT(path) DO UPDATE SET project_id = CASE
                       WHEN excluded.project_id > daw_library.project_id THEN excluded.project_id
                       ELSE daw_library.project_id END",
                )
                .map_err(|e| e.to_string())?;
            for (i, p) in projects.iter().enumerate() {
                let path = normalize_path_for_db(&p.path);
                let directory = normalize_path_for_db(&p.directory);
                let changed = stmt
                    .execute(params![
                        p.name,
                        path,
                        directory,
                        p.format,
                        p.daw,
                        p.size as i64,
                        p.size_formatted,
                        p.modified,
                        scan_id
                    ])
                    .map_err(|e| e.to_string())?;
                if changed > 0 {
                    let id = tx.last_insert_rowid();
                    fts_stmt
                        .execute(params![id, p.name, path, p.daw, scan_id])
                        .map_err(|e| e.to_string())?;
                    lib_stmt
                        .execute(params![path, id])
                        .map_err(|e| e.to_string())?;
                    inserted_idx.push(i);
                    batch_bytes += p.size;
                }
            }
        }
        let inserted = inserted_idx.len() as u64;
        if inserted > 0 {
            tx.execute(
                "UPDATE daw_scans SET project_count = project_count + ?2, total_bytes = total_bytes + ?3 WHERE id = ?1",
                params![scan_id, inserted as i64, batch_bytes as i64],
            ).map_err(|e| e.to_string())?;
        }
        tx.commit().map_err(|e| e.to_string())?;
        if !inserted_idx.is_empty() {
            self.invalidate_daw_library_total_cache();
        }
        Ok(inserted_idx)
    }

    pub fn save_daw_scan(&self, snap: &DawScanSnapshot) -> Result<(), String> {
        let conn = self.read_conn();
        let daw_json = serde_json::to_string(&snap.daw_counts).unwrap_or_default();
        let roots_json = path_strings_json_normalized(&snap.roots);
        conn.execute(
            "INSERT OR REPLACE INTO daw_scans (id, timestamp, project_count, total_bytes, daw_counts, roots, scan_complete) VALUES (?1,?2,?3,?4,?5,?6,1)",
            params![snap.id, snap.timestamp, snap.project_count as i64, snap.total_bytes as i64, daw_json, roots_json],
        ).map_err(|e| e.to_string())?;
        let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
        {
            tx.execute(
                "DELETE FROM daw_projects WHERE scan_id = ?1",
                params![snap.id],
            )
            .map_err(|e| e.to_string())?;
            tx.execute(
                "DELETE FROM daw_projects_fts WHERE scan_id = ?1",
                params![snap.id],
            )
            .map_err(|e| e.to_string())?;
            let mut stmt = tx.prepare_cached("INSERT OR IGNORE INTO daw_projects (name, path, directory, format, daw, size, size_formatted, modified, scan_id) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)").map_err(|e| e.to_string())?;
            let mut fts_stmt = tx.prepare_cached("INSERT INTO daw_projects_fts(rowid, name, path, daw, scan_id) VALUES (?1,?2,?3,?4,?5)").map_err(|e| e.to_string())?;
            for p in &snap.projects {
                let path = normalize_path_for_db(&p.path);
                let directory = normalize_path_for_db(&p.directory);
                let changed = stmt
                    .execute(params![
                        p.name,
                        path,
                        directory,
                        p.format,
                        p.daw,
                        p.size as i64,
                        p.size_formatted,
                        p.modified,
                        snap.id
                    ])
                    .map_err(|e| e.to_string())?;
                if changed > 0 {
                    let id = tx.last_insert_rowid();
                    fts_stmt
                        .execute(params![id, p.name, path, p.daw, snap.id])
                        .map_err(|e| e.to_string())?;
                }
            }
        }
        tx.commit().map_err(|e| e.to_string())?;
        Self::rebuild_daw_library(&conn)?;
        self.invalidate_daw_library_total_cache();
        Ok(())
    }

    pub fn get_daw_scans(&self) -> Result<Vec<serde_json::Value>, String> {
        let conn = self.read_conn();
        // Count from child rows so the History tab stays correct even if parent totals
        // were never finalized (streaming scans) or finalize failed silently.
        let mut stmt = conn.prepare(
            "SELECT s.id, s.timestamp, COALESCE(NULLIF(s.project_count,0),(SELECT COUNT(*) FROM daw_projects WHERE scan_id = s.id)), COALESCE(NULLIF(s.total_bytes,0),(SELECT COALESCE(SUM(size),0) FROM daw_projects WHERE scan_id = s.id)), s.daw_counts, s.roots FROM daw_scans s WHERE s.scan_complete = 1 ORDER BY s.timestamp DESC",
        )
        .map_err(|e| e.to_string())?;
        let rows = stmt.query_map([], |row| {
            let dc_str: String = row.get(4)?;
            let roots_str: String = row.get(5)?;
            Ok(serde_json::json!({
                "id": row.get::<_,String>(0)?,
                "timestamp": row.get::<_,String>(1)?,
                "projectCount": row.get::<_, i64>(2)? as u64,
                "totalBytes": row.get::<_, i64>(3)? as u64,
                "dawCounts": serde_json::from_str::<HashMap<String,usize>>(&dc_str).unwrap_or_default(),
                "roots": serde_json::from_str::<Vec<String>>(&roots_str).unwrap_or_default(),
            }))
        }).map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    }

    pub fn get_daw_scan_detail(&self, id: &str) -> Result<DawScanSnapshot, String> {
        let conn = self.read_conn();
        let (ts, pc, tb, dc_str, roots_str): (String, usize, u64, String, String) = conn.query_row(
            "SELECT timestamp, project_count, total_bytes, daw_counts, roots FROM daw_scans WHERE id = ?1",
            params![id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get::<_, i64>(1)? as usize,
                    row.get::<_, i64>(2)? as u64,
                    row.get(3)?,
                    row.get(4)?,
                ))
            },
        )
        .map_err(|e| e.to_string())?;
        let mut stmt = conn.prepare("SELECT name, path, directory, format, daw, size, size_formatted, modified FROM daw_projects WHERE scan_id = ?1").map_err(|e| e.to_string())?;
        let projects = stmt
            .query_map(params![id], |row| {
                Ok(DawProject {
                    name: row.get(0)?,
                    path: row.get(1)?,
                    directory: row.get(2)?,
                    format: row.get(3)?,
                    daw: row.get(4)?,
                    size: row.get::<_, i64>(5).unwrap_or(0) as u64,
                    size_formatted: row.get(6)?,
                    modified: row.get(7)?,
                })
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        let live_count = projects.len();
        let live_bytes: u64 = projects.iter().map(|p| p.size).sum();
        Ok(DawScanSnapshot {
            id: id.to_string(),
            timestamp: ts,
            project_count: if pc > 0 { pc } else { live_count },
            total_bytes: if tb > 0 { tb } else { live_bytes },
            daw_counts: serde_json::from_str(&dc_str).unwrap_or_default(),
            projects,
            roots: serde_json::from_str(&roots_str).unwrap_or_default(),
        })
    }

    pub fn get_latest_daw_scan(&self) -> Result<Option<DawScanSnapshot>, String> {
        let conn = self.read_conn();
        let id: Option<String> = conn
            .query_row(LATEST_DAW_SCAN_ID_SQL, [], |r| r.get::<_, String>(0))
            .optional()
            .map_err(|e| e.to_string())?;
        drop(conn);
        match id {
            Some(id) => self.get_daw_scan_detail(&id).map(Some),
            None => Ok(None),
        }
    }

    pub fn delete_daw_scan(&self, id: &str) -> Result<(), String> {
        let conn = self.read_conn();
        conn.execute(
            "CREATE TEMP TABLE _dl_refresh_paths (path TEXT PRIMARY KEY)",
            [],
        )
        .map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT INTO _dl_refresh_paths SELECT DISTINCT path FROM daw_projects WHERE scan_id = ?1",
            params![id],
        )
        .map_err(|e| e.to_string())?;
        conn.execute(
            "DELETE FROM daw_projects_fts WHERE scan_id = ?1",
            params![id],
        )
        .map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM daw_projects WHERE scan_id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM daw_scans WHERE id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        Self::sync_daw_library_after_paths_refresh(&conn)?;
        self.invalidate_daw_library_total_cache();
        Ok(())
    }

    pub fn clear_daw_history(&self) -> Result<(), String> {
        let conn = self.read_conn();
        conn.execute_batch(
            "BEGIN IMMEDIATE;
             DELETE FROM daw_library;
             DELETE FROM daw_projects_fts;
             DELETE FROM daw_projects;
             DELETE FROM daw_scans;
             COMMIT;",
        )
        .map_err(|e| e.to_string())?;
        self.invalidate_daw_library_total_cache();
        Ok(())
    }

    // ── Preset scan CRUD ──

    pub fn preset_scan_parent_create(
        &self,
        id: &str,
        timestamp: &str,
        roots: &[String],
    ) -> Result<(), String> {
        let conn = self.read_conn();
        let roots_json = path_strings_json_normalized(roots);
        conn.execute(
            "INSERT OR REPLACE INTO preset_scans (id, timestamp, preset_count, total_bytes, format_counts, roots, scan_complete) VALUES (?1,?2,0,0,'{}',?3,0)",
            params![id, timestamp, roots_json],
        ).map_err(|e| e.to_string())?;
        conn.execute(
            "CREATE TEMP TABLE _preset_lib_refresh_paths (path TEXT PRIMARY KEY)",
            [],
        )
        .map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT INTO _preset_lib_refresh_paths SELECT DISTINCT path FROM presets WHERE scan_id = ?1",
            params![id],
        )
        .map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM presets WHERE scan_id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM presets_fts WHERE scan_id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        Self::sync_preset_library_after_paths_refresh(&conn)?;
        self.invalidate_preset_inventory_total_cache();
        Ok(())
    }

    pub fn preset_scan_parent_finalize(
        &self,
        id: &str,
        _preset_count: usize,
        _total_bytes: u64,
        _format_counts: &HashMap<String, usize>,
    ) -> Result<(), String> {
        let conn = self.read_conn();
        let preset_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM presets WHERE id IN (SELECT preset_id FROM preset_library) AND format NOT IN ('MID', 'MIDI')",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);
        let total_bytes: i64 = conn
            .query_row(
                "SELECT COALESCE(SUM(size), 0) FROM presets WHERE id IN (SELECT preset_id FROM preset_library) AND format NOT IN ('MID', 'MIDI')",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);
        let mut format_map: HashMap<String, usize> = HashMap::new();
        let mut stmt = conn
            .prepare(
                "SELECT format, COUNT(*) FROM presets WHERE id IN (SELECT preset_id FROM preset_library) AND format NOT IN ('MID', 'MIDI') GROUP BY format",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? as usize))
            })
            .map_err(|e| e.to_string())?;
        for (fmt, n) in rows.flatten() {
            format_map.insert(fmt, n);
        }
        let fc_json = serde_json::to_string(&format_map).unwrap_or_default();
        conn.execute(
            "UPDATE preset_scans SET preset_count = ?2, total_bytes = ?3, format_counts = ?4 WHERE id = ?1",
            params![id, preset_count, total_bytes, fc_json],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn insert_preset_batch(
        &self,
        scan_id: &str,
        presets: &[PresetFile],
    ) -> Result<u64, String> {
        let conn = self.read_conn();
        let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
        let mut inserted: u64 = 0;
        let mut batch_bytes: u64 = 0;
        {
            let mut stmt = tx.prepare_cached("INSERT OR IGNORE INTO presets (name, path, directory, format, size, size_formatted, modified, scan_id) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)").map_err(|e| e.to_string())?;
            let mut fts_stmt = tx.prepare_cached("INSERT INTO presets_fts(rowid, name, path, format, scan_id) VALUES (?1,?2,?3,?4,?5)").map_err(|e| e.to_string())?;
            let mut lib_stmt = tx
                .prepare_cached(
                    "INSERT INTO preset_library (path, preset_id) VALUES (?1, ?2)
                     ON CONFLICT(path) DO UPDATE SET preset_id = CASE
                       WHEN excluded.preset_id > preset_library.preset_id THEN excluded.preset_id
                       ELSE preset_library.preset_id END",
                )
                .map_err(|e| e.to_string())?;
            for p in presets {
                let path = normalize_path_for_db(&p.path);
                let directory = normalize_path_for_db(&p.directory);
                let changed = stmt
                    .execute(params![
                        p.name,
                        path,
                        directory,
                        p.format,
                        p.size as i64,
                        p.size_formatted,
                        p.modified,
                        scan_id
                    ])
                    .map_err(|e| e.to_string())?;
                if changed > 0 {
                    let id = tx.last_insert_rowid();
                    fts_stmt
                        .execute(params![id, p.name, path, p.format, scan_id])
                        .map_err(|e| e.to_string())?;
                    lib_stmt
                        .execute(params![path, id])
                        .map_err(|e| e.to_string())?;
                    inserted += 1;
                    batch_bytes += p.size;
                }
            }
        }
        if inserted > 0 {
            tx.execute(
                "UPDATE preset_scans SET preset_count = preset_count + ?2, total_bytes = total_bytes + ?3 WHERE id = ?1",
                params![scan_id, inserted as i64, batch_bytes as i64],
            ).map_err(|e| e.to_string())?;
        }
        tx.commit().map_err(|e| e.to_string())?;
        if inserted > 0 {
            self.invalidate_preset_inventory_total_cache();
        }
        Ok(inserted)
    }

    pub fn save_preset_scan(&self, snap: &PresetScanSnapshot) -> Result<(), String> {
        let conn = self.read_conn();
        let fc_json = serde_json::to_string(&snap.format_counts).unwrap_or_default();
        let roots_json = path_strings_json_normalized(&snap.roots);
        conn.execute(
            "INSERT OR REPLACE INTO preset_scans (id, timestamp, preset_count, total_bytes, format_counts, roots, scan_complete) VALUES (?1,?2,?3,?4,?5,?6,1)",
            params![snap.id, snap.timestamp, snap.preset_count as i64, snap.total_bytes as i64, fc_json, roots_json],
        ).map_err(|e| e.to_string())?;
        let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
        {
            tx.execute(
                "CREATE TEMP TABLE _preset_lib_refresh_paths (path TEXT PRIMARY KEY)",
                [],
            )
            .map_err(|e| e.to_string())?;
            tx.execute(
                "INSERT INTO _preset_lib_refresh_paths SELECT DISTINCT path FROM presets WHERE scan_id = ?1",
                params![snap.id],
            )
            .map_err(|e| e.to_string())?;
            for p in &snap.presets {
                let path = normalize_path_for_db(&p.path);
                tx.execute(
                    "INSERT OR IGNORE INTO _preset_lib_refresh_paths (path) VALUES (?1)",
                    params![path],
                )
                .map_err(|e| e.to_string())?;
            }
            tx.execute("DELETE FROM presets WHERE scan_id = ?1", params![snap.id])
                .map_err(|e| e.to_string())?;
            tx.execute(
                "DELETE FROM presets_fts WHERE scan_id = ?1",
                params![snap.id],
            )
            .map_err(|e| e.to_string())?;
            let mut stmt = tx.prepare_cached("INSERT OR IGNORE INTO presets (name, path, directory, format, size, size_formatted, modified, scan_id) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)").map_err(|e| e.to_string())?;
            let mut fts_stmt = tx.prepare_cached("INSERT INTO presets_fts(rowid, name, path, format, scan_id) VALUES (?1,?2,?3,?4,?5)").map_err(|e| e.to_string())?;
            for p in &snap.presets {
                let path = normalize_path_for_db(&p.path);
                let directory = normalize_path_for_db(&p.directory);
                let changed = stmt
                    .execute(params![
                        p.name,
                        path,
                        directory,
                        p.format,
                        p.size as i64,
                        p.size_formatted,
                        p.modified,
                        snap.id
                    ])
                    .map_err(|e| e.to_string())?;
                if changed > 0 {
                    let id = tx.last_insert_rowid();
                    fts_stmt
                        .execute(params![id, p.name, path, p.format, snap.id])
                        .map_err(|e| e.to_string())?;
                }
            }
            Self::sync_preset_library_after_paths_refresh_tx(&tx)?;
        }
        tx.commit().map_err(|e| e.to_string())?;
        self.invalidate_preset_inventory_total_cache();
        Ok(())
    }

    pub fn get_preset_scans(&self) -> Result<Vec<serde_json::Value>, String> {
        let conn = self.read_conn();
        let mut stmt = conn.prepare(
            "SELECT s.id, s.timestamp, COALESCE(NULLIF(s.preset_count,0),(SELECT COUNT(*) FROM presets WHERE scan_id = s.id)), COALESCE(NULLIF(s.total_bytes,0),(SELECT COALESCE(SUM(size),0) FROM presets WHERE scan_id = s.id)), s.format_counts, s.roots FROM preset_scans s WHERE s.scan_complete = 1 ORDER BY s.timestamp DESC",
        )
        .map_err(|e| e.to_string())?;
        let rows = stmt.query_map([], |row| {
            let fc_str: String = row.get(4)?;
            let roots_str: String = row.get(5)?;
            Ok(serde_json::json!({
                "id": row.get::<_,String>(0)?,
                "timestamp": row.get::<_,String>(1)?,
                "presetCount": row.get::<_, i64>(2)? as u64,
                "totalBytes": row.get::<_, i64>(3)? as u64,
                "formatCounts": serde_json::from_str::<HashMap<String,usize>>(&fc_str).unwrap_or_default(),
                "roots": serde_json::from_str::<Vec<String>>(&roots_str).unwrap_or_default(),
            }))
        }).map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    }

    pub fn get_preset_scan_detail(&self, id: &str) -> Result<PresetScanSnapshot, String> {
        let conn = self.read_conn();
        let (ts, pc, tb, fc_str, roots_str): (String, usize, u64, String, String) = conn.query_row(
            "SELECT timestamp, preset_count, total_bytes, format_counts, roots FROM preset_scans WHERE id = ?1",
            params![id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get::<_, i64>(1)? as usize,
                    row.get::<_, i64>(2)? as u64,
                    row.get(3)?,
                    row.get(4)?,
                ))
            },
        )
        .map_err(|e| e.to_string())?;
        let mut stmt = conn.prepare("SELECT name, path, directory, format, size, size_formatted, modified FROM presets WHERE scan_id = ?1").map_err(|e| e.to_string())?;
        let presets = stmt
            .query_map(params![id], |row| {
                Ok(PresetFile {
                    name: row.get(0)?,
                    path: row.get(1)?,
                    directory: row.get(2)?,
                    format: row.get(3)?,
                    size: row.get::<_, i64>(4).unwrap_or(0) as u64,
                    size_formatted: row.get(5)?,
                    modified: row.get(6)?,
                })
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        let live_count = presets.len();
        let live_bytes: u64 = presets.iter().map(|p| p.size).sum();
        Ok(PresetScanSnapshot {
            id: id.to_string(),
            timestamp: ts,
            preset_count: if pc > 0 { pc } else { live_count },
            total_bytes: if tb > 0 { tb } else { live_bytes },
            format_counts: serde_json::from_str(&fc_str).unwrap_or_default(),
            presets,
            roots: serde_json::from_str(&roots_str).unwrap_or_default(),
        })
    }

    pub fn get_latest_preset_scan(&self) -> Result<Option<PresetScanSnapshot>, String> {
        let conn = self.read_conn();
        let id: Option<String> = conn
            .query_row(
                "SELECT id FROM preset_scans WHERE scan_complete = 1 ORDER BY timestamp DESC LIMIT 1",
                [],
                |r| r.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())?;
        drop(conn);
        match id {
            Some(id) => self.get_preset_scan_detail(&id).map(Some),
            None => Ok(None),
        }
    }

    pub fn delete_preset_scan(&self, id: &str) -> Result<(), String> {
        let conn = self.read_conn();
        conn.execute(
            "CREATE TEMP TABLE _preset_lib_refresh_paths (path TEXT PRIMARY KEY)",
            [],
        )
        .map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT INTO _preset_lib_refresh_paths SELECT DISTINCT path FROM presets WHERE scan_id = ?1",
            params![id],
        )
        .map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM presets WHERE scan_id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM presets_fts WHERE scan_id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        Self::sync_preset_library_after_paths_refresh(&conn)?;
        self.invalidate_preset_inventory_total_cache();
        conn.execute("DELETE FROM preset_scans WHERE id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn clear_preset_history(&self) -> Result<(), String> {
        let conn = self.read_conn();
        conn.execute_batch(
            "BEGIN IMMEDIATE;
             DELETE FROM preset_library;
             DELETE FROM presets_fts;
             DELETE FROM presets;
             DELETE FROM preset_scans;
             COMMIT;",
        )
        .map_err(|e| e.to_string())?;
        self.invalidate_preset_inventory_total_cache();
        Ok(())
    }

    // ── MIDI scan CRUD ──

    pub fn midi_scan_parent_create(
        &self,
        id: &str,
        timestamp: &str,
        roots: &[String],
    ) -> Result<(), String> {
        let conn = self.read_conn();
        let roots_json = path_strings_json_normalized(roots);
        conn.execute(
            "INSERT OR REPLACE INTO midi_scans (id, timestamp, midi_count, total_bytes, format_counts, roots, scan_complete) VALUES (?1,?2,0,0,'{}',?3,0)",
            params![id, timestamp, roots_json],
        ).map_err(|e| e.to_string())?;
        conn.execute(
            "CREATE TEMP TABLE _midi_lib_refresh_paths (path TEXT PRIMARY KEY)",
            [],
        )
        .map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT INTO _midi_lib_refresh_paths SELECT DISTINCT path FROM midi_files WHERE scan_id = ?1",
            params![id],
        )
        .map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM midi_files WHERE scan_id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM midi_files_fts WHERE scan_id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        Self::sync_midi_library_after_paths_refresh(&conn)?;
        self.invalidate_midi_library_total_cache();
        Ok(())
    }

    pub fn midi_scan_parent_finalize(
        &self,
        id: &str,
        _midi_count: usize,
        _total_bytes: u64,
        _format_counts: &HashMap<String, usize>,
    ) -> Result<(), String> {
        let conn = self.read_conn();
        let midi_count: i64 = conn
            .query_row("SELECT COUNT(DISTINCT path) FROM midi_files", [], |r| {
                r.get(0)
            })
            .unwrap_or(0);
        let total_bytes: i64 = conn
            .query_row(
                "SELECT COALESCE(SUM(size), 0) FROM midi_files WHERE id IN (SELECT midi_id FROM midi_library)",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);
        let mut format_map: HashMap<String, usize> = HashMap::new();
        let mut stmt = conn
            .prepare(
                "SELECT format, COUNT(*) FROM midi_files WHERE id IN (SELECT midi_id FROM midi_library) GROUP BY format",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? as usize))
            })
            .map_err(|e| e.to_string())?;
        for (fmt, n) in rows.flatten() {
            format_map.insert(fmt, n);
        }
        let fc_json = serde_json::to_string(&format_map).unwrap_or_default();
        conn.execute(
            "UPDATE midi_scans SET midi_count = ?2, total_bytes = ?3, format_counts = ?4 WHERE id = ?1",
            params![id, midi_count, total_bytes, fc_json],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn insert_midi_batch(&self, scan_id: &str, midi_files: &[MidiFile]) -> Result<(), String> {
        let conn = self.read_conn();
        let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
        let mut inserted: u64 = 0;
        let mut batch_bytes: u64 = 0;
        {
            let mut stmt = tx.prepare_cached("INSERT OR IGNORE INTO midi_files (name, path, directory, format, size, size_formatted, modified, scan_id) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)").map_err(|e| e.to_string())?;
            let mut fts_stmt = tx
                .prepare_cached(
                    "INSERT INTO midi_files_fts(rowid, name, path, scan_id) VALUES (?1,?2,?3,?4)",
                )
                .map_err(|e| e.to_string())?;
            let mut lib_stmt = tx
                .prepare_cached(
                    "INSERT INTO midi_library (path, midi_id) VALUES (?1, ?2)
                     ON CONFLICT(path) DO UPDATE SET midi_id = CASE
                       WHEN excluded.midi_id > midi_library.midi_id THEN excluded.midi_id
                       ELSE midi_library.midi_id END",
                )
                .map_err(|e| e.to_string())?;
            for m in midi_files {
                let path = normalize_path_for_db(&m.path);
                let directory = normalize_path_for_db(&m.directory);
                let changed = stmt
                    .execute(params![
                        m.name,
                        path,
                        directory,
                        m.format,
                        m.size as i64,
                        m.size_formatted,
                        m.modified,
                        scan_id
                    ])
                    .map_err(|e| e.to_string())?;
                if changed > 0 {
                    let id = tx.last_insert_rowid();
                    fts_stmt
                        .execute(params![id, m.name, path, scan_id])
                        .map_err(|e| e.to_string())?;
                    lib_stmt
                        .execute(params![path, id])
                        .map_err(|e| e.to_string())?;
                    inserted += 1;
                    batch_bytes += m.size;
                }
            }
        }
        if inserted > 0 {
            tx.execute(
                "UPDATE midi_scans SET midi_count = midi_count + ?2, total_bytes = total_bytes + ?3 WHERE id = ?1",
                params![scan_id, inserted as i64, batch_bytes as i64],
            ).map_err(|e| e.to_string())?;
        }
        tx.commit().map_err(|e| e.to_string())?;
        if inserted > 0 {
            self.invalidate_midi_library_total_cache();
        }
        Ok(())
    }

    pub fn save_midi_scan(&self, snap: &MidiScanSnapshot) -> Result<(), String> {
        let conn = self.read_conn();
        let fc_json = serde_json::to_string(&snap.format_counts).unwrap_or_default();
        let roots_json = path_strings_json_normalized(&snap.roots);
        conn.execute(
            "INSERT OR REPLACE INTO midi_scans (id, timestamp, midi_count, total_bytes, format_counts, roots, scan_complete) VALUES (?1,?2,?3,?4,?5,?6,1)",
            params![snap.id, snap.timestamp, snap.midi_count as i64, snap.total_bytes as i64, fc_json, roots_json],
        ).map_err(|e| e.to_string())?;
        let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
        {
            tx.execute(
                "CREATE TEMP TABLE _midi_lib_refresh_paths (path TEXT PRIMARY KEY)",
                [],
            )
            .map_err(|e| e.to_string())?;
            tx.execute(
                "INSERT INTO _midi_lib_refresh_paths SELECT DISTINCT path FROM midi_files WHERE scan_id = ?1",
                params![snap.id],
            )
            .map_err(|e| e.to_string())?;
            for m in &snap.midi_files {
                let path = normalize_path_for_db(&m.path);
                tx.execute(
                    "INSERT OR IGNORE INTO _midi_lib_refresh_paths (path) VALUES (?1)",
                    params![path],
                )
                .map_err(|e| e.to_string())?;
            }
            tx.execute(
                "DELETE FROM midi_files WHERE scan_id = ?1",
                params![snap.id],
            )
            .map_err(|e| e.to_string())?;
            tx.execute(
                "DELETE FROM midi_files_fts WHERE scan_id = ?1",
                params![snap.id],
            )
            .map_err(|e| e.to_string())?;
            let mut stmt = tx.prepare_cached("INSERT OR IGNORE INTO midi_files (name, path, directory, format, size, size_formatted, modified, scan_id) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)").map_err(|e| e.to_string())?;
            let mut fts_stmt = tx
                .prepare_cached(
                    "INSERT INTO midi_files_fts(rowid, name, path, scan_id) VALUES (?1,?2,?3,?4)",
                )
                .map_err(|e| e.to_string())?;
            for m in &snap.midi_files {
                let path = normalize_path_for_db(&m.path);
                let directory = normalize_path_for_db(&m.directory);
                let changed = stmt
                    .execute(params![
                        m.name,
                        path,
                        directory,
                        m.format,
                        m.size as i64,
                        m.size_formatted,
                        m.modified,
                        snap.id
                    ])
                    .map_err(|e| e.to_string())?;
                if changed > 0 {
                    let id = tx.last_insert_rowid();
                    fts_stmt
                        .execute(params![id, m.name, path, snap.id])
                        .map_err(|e| e.to_string())?;
                }
            }
            Self::sync_midi_library_after_paths_refresh_tx(&tx)?;
        }
        tx.commit().map_err(|e| e.to_string())?;
        self.invalidate_midi_library_total_cache();
        Ok(())
    }

    pub fn get_midi_scans(&self) -> Result<Vec<serde_json::Value>, String> {
        let conn = self.read_conn();
        let mut stmt = conn.prepare(
            "SELECT s.id, s.timestamp, COALESCE(NULLIF(s.midi_count,0),(SELECT COUNT(*) FROM midi_files WHERE scan_id = s.id)), COALESCE(NULLIF(s.total_bytes,0),(SELECT COALESCE(SUM(size),0) FROM midi_files WHERE scan_id = s.id)), s.format_counts, s.roots FROM midi_scans s WHERE s.scan_complete = 1 ORDER BY s.timestamp DESC",
        )
        .map_err(|e| e.to_string())?;
        let rows = stmt.query_map([], |row| {
            let fc_str: String = row.get(4)?;
            let roots_str: String = row.get(5)?;
            Ok(serde_json::json!({
                "id": row.get::<_,String>(0)?,
                "timestamp": row.get::<_,String>(1)?,
                "midiCount": row.get::<_, i64>(2)? as u64,
                "totalBytes": row.get::<_, i64>(3)? as u64,
                "formatCounts": serde_json::from_str::<HashMap<String,usize>>(&fc_str).unwrap_or_default(),
                "roots": serde_json::from_str::<Vec<String>>(&roots_str).unwrap_or_default(),
            }))
        }).map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    }

    pub fn get_midi_scan_detail(&self, id: &str) -> Result<MidiScanSnapshot, String> {
        let conn = self.read_conn();
        let (ts, mc, tb, fc_str, roots_str): (String, usize, u64, String, String) = conn.query_row(
            "SELECT timestamp, midi_count, total_bytes, format_counts, roots FROM midi_scans WHERE id = ?1",
            params![id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get::<_, i64>(1)? as usize,
                    row.get::<_, i64>(2)? as u64,
                    row.get(3)?,
                    row.get(4)?,
                ))
            },
        )
        .map_err(|e| e.to_string())?;
        let mut stmt = conn.prepare("SELECT name, path, directory, format, size, size_formatted, modified FROM midi_files WHERE scan_id = ?1").map_err(|e| e.to_string())?;
        let midi_files = stmt
            .query_map(params![id], |row| {
                Ok(MidiFile {
                    name: row.get(0)?,
                    path: row.get(1)?,
                    directory: row.get(2)?,
                    format: row.get(3)?,
                    size: row.get::<_, i64>(4).unwrap_or(0) as u64,
                    size_formatted: row.get(5)?,
                    modified: row.get(6)?,
                })
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        let live_count = midi_files.len();
        let live_bytes: u64 = midi_files.iter().map(|m| m.size).sum();
        Ok(MidiScanSnapshot {
            id: id.to_string(),
            timestamp: ts,
            midi_count: if mc > 0 { mc } else { live_count },
            total_bytes: if tb > 0 { tb } else { live_bytes },
            format_counts: serde_json::from_str(&fc_str).unwrap_or_default(),
            midi_files,
            roots: serde_json::from_str(&roots_str).unwrap_or_default(),
        })
    }

    pub fn get_latest_midi_scan(&self) -> Result<Option<MidiScanSnapshot>, String> {
        let conn = self.read_conn();
        let id: Option<String> = conn
            .query_row(
                "SELECT id FROM midi_scans WHERE scan_complete = 1 ORDER BY timestamp DESC LIMIT 1",
                [],
                |r| r.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())?;
        drop(conn);
        match id {
            Some(id) => self.get_midi_scan_detail(&id).map(Some),
            None => Ok(None),
        }
    }

    pub fn delete_midi_scan(&self, id: &str) -> Result<(), String> {
        let conn = self.read_conn();
        conn.execute(
            "CREATE TEMP TABLE _midi_lib_refresh_paths (path TEXT PRIMARY KEY)",
            [],
        )
        .map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT INTO _midi_lib_refresh_paths SELECT DISTINCT path FROM midi_files WHERE scan_id = ?1",
            params![id],
        )
        .map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM midi_files WHERE scan_id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM midi_files_fts WHERE scan_id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        Self::sync_midi_library_after_paths_refresh(&conn)?;
        conn.execute("DELETE FROM midi_scans WHERE id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        self.invalidate_midi_library_total_cache();
        Ok(())
    }

    pub fn clear_midi_history(&self) -> Result<(), String> {
        let conn = self.read_conn();
        conn.execute_batch(
            "BEGIN IMMEDIATE;
             DELETE FROM midi_library;
             DELETE FROM midi_files_fts;
             DELETE FROM midi_files;
             DELETE FROM midi_scans;
             COMMIT;",
        )
        .map_err(|e| e.to_string())?;
        self.invalidate_midi_library_total_cache();
        Ok(())
    }

    pub fn query_midi(
        &self,
        search: Option<&str>,
        format_filter: Option<&str>,
        sort_key: &str,
        sort_asc: bool,
        search_regex: bool,
        offset: u64,
        limit: u64,
    ) -> Result<MidiQueryResult, String> {
        let conn = self.read_conn();
        // One row per library entry (same semantics as Samples tab / `query_audio` COUNT).
        let total_unfiltered: u64 = self.midi_library_total_rows(&conn)?;
        if total_unfiltered == 0 {
            return Ok(MidiQueryResult {
                midi_files: vec![],
                total_count: 0,
                total_count_capped: false,
                total_unfiltered: 0,
            });
        }

        let mut where_parts = vec![MIDI_LIBRARY_IDS.to_string()];
        let mut bind_idx = 1usize;
        let (fts_match, like_pat, regex_pat) = classify_fts_name_path_search(search, search_regex);
        if fts_match.is_some() {
            // Library scope is already `MIDI_LIBRARY_IDS`; do not nest a second
            // `MAX(id) GROUP BY path` inside the FTS subquery (same semantics, worse plan).
            where_parts.push(format!(
                "id IN (SELECT rowid FROM midi_files_fts WHERE midi_files_fts MATCH ?{bind_idx})",
            ));
            bind_idx += 1;
        } else if regex_pat.is_some() {
            where_parts.push(format!(
                "((name REGEXP ?{bind_idx}) OR (path REGEXP ?{bind_idx}))"
            ));
            bind_idx += 1;
        } else if like_pat.is_some() {
            where_parts.push(format!(
                "(name LIKE ?{bind_idx} ESCAPE '\\' OR path LIKE ?{bind_idx} ESCAPE '\\')"
            ));
            bind_idx += 1;
        }
        if let Some(f) = format_filter
            && !f.is_empty() && f != "all" {
                if f.contains(',') {
                    where_parts.push(format!(
                        "format IN ({})",
                        f.split(',')
                            .map(|s| format!("'{}'", s.trim().replace('\'', "''")))
                            .collect::<Vec<_>>()
                            .join(",")
                    ));
                } else {
                    where_parts.push(format!("format = ?{bind_idx}"));
                    bind_idx += 1;
                }
            }
        let where_cl = where_parts.join(" AND ");

        let sort_col = match sort_key {
            "name" => "name COLLATE NOCASE",
            "size" => "size",
            "modified" => "modified",
            "directory" => "directory COLLATE NOCASE",
            "format" => "format",
            _ => "name COLLATE NOCASE",
        };
        let dir = if sort_asc { "ASC" } else { "DESC" };

        // FTS + ORDER BY name/size/… over millions of substring hits forces SQLite to sort the
        // full match set before LIMIT — multi‑minute stalls. Always use bm25 + LIMIT for FTS
        // (column sort is ignored in SQL; JS may re-rank the page with `searchScore`).
        let use_fts_rank_page = fts_match.is_some();

        let (total_count, total_count_capped) = if let Some(m) = fts_match.as_ref() {
            Self::midi_fts_bounded_count_library(&conn, m, format_filter)?
        } else {
            let sql = format!("SELECT COUNT(*) FROM midi_files WHERE {where_cl}");
            let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
            let mut bi = 1;
            if let Some(ref r) = regex_pat {
                stmt.raw_bind_parameter(bi, r).map_err(|e| e.to_string())?;
                bi += 1;
            } else if let Some(ref pat) = like_pat {
                stmt.raw_bind_parameter(bi, pat)
                    .map_err(|e| e.to_string())?;
                bi += 1;
            }
            if let Some(f) = format_filter
                && !f.is_empty() && f != "all" && !f.contains(',') {
                    stmt.raw_bind_parameter(bi, f).map_err(|e| e.to_string())?;
                }
            let mut rows = stmt.raw_query();
            let n = rows
                .next()
                .map_err(|e| e.to_string())?
                .map(|r| r.get::<_, i64>(0).unwrap_or(0) as u64)
                .unwrap_or(0);
            (n, false)
        };

        let sql = if use_fts_rank_page {
            let mut w = String::from(
                "SELECT m.name, m.path, m.directory, m.format, m.size, m.size_formatted, m.modified
                 FROM midi_files_fts
                 INNER JOIN midi_files m ON m.id = midi_files_fts.rowid
                 INNER JOIN midi_library lib ON lib.midi_id = m.id
                 WHERE midi_files_fts MATCH ?1",
            );
            let mut next_ph = 2usize;
            if let Some(f) = format_filter
                && !f.is_empty() && f != "all" {
                    if f.contains(',') {
                        let vals: Vec<String> = f
                            .split(',')
                            .map(|s| format!("'{}'", s.trim().replace('\'', "''")))
                            .collect();
                        w.push_str(&format!(" AND m.format IN ({})", vals.join(",")));
                    } else {
                        w.push_str(&format!(" AND m.format = ?{next_ph}"));
                        next_ph += 1;
                    }
                }
            let li = next_ph;
            let oi = next_ph + 1;
            w.push_str(&format!(
                " ORDER BY bm25(midi_files_fts) LIMIT ?{li} OFFSET ?{oi}"
            ));
            w
        } else {
            format!(
                "SELECT name, path, directory, format, size, size_formatted, modified
                 FROM midi_files WHERE {where_cl}
                 ORDER BY {sort_col} {dir} LIMIT ?{limit_idx} OFFSET ?{off_idx}",
                limit_idx = bind_idx,
                off_idx = bind_idx + 1
            )
        };
        let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
        let mut bi = 1;
        if use_fts_rank_page {
            let m = fts_match.as_ref().expect("fts");
            stmt.raw_bind_parameter(bi, m).map_err(|e| e.to_string())?;
            bi += 1;
            if let Some(f) = format_filter
                && !f.is_empty() && f != "all" && !f.contains(',') {
                    stmt.raw_bind_parameter(bi, f).map_err(|e| e.to_string())?;
                    bi += 1;
                }
            stmt.raw_bind_parameter(bi, limit as i64)
                .map_err(|e| e.to_string())?;
            stmt.raw_bind_parameter(bi + 1, offset as i64)
                .map_err(|e| e.to_string())?;
        } else if let Some(ref m) = fts_match {
            stmt.raw_bind_parameter(bi, m).map_err(|e| e.to_string())?;
            bi += 1;
        } else if let Some(ref r) = regex_pat {
            stmt.raw_bind_parameter(bi, r).map_err(|e| e.to_string())?;
            bi += 1;
        } else if let Some(ref pat) = like_pat {
            stmt.raw_bind_parameter(bi, pat)
                .map_err(|e| e.to_string())?;
            bi += 1;
        }
        if !use_fts_rank_page {
            if let Some(f) = format_filter
                && !f.is_empty() && f != "all" && !f.contains(',') {
                    stmt.raw_bind_parameter(bi, f).map_err(|e| e.to_string())?;
                    bi += 1;
                }
            stmt.raw_bind_parameter(bi, limit as i64)
                .map_err(|e| e.to_string())?;
            stmt.raw_bind_parameter(bi + 1, offset as i64)
                .map_err(|e| e.to_string())?;
        }
        let mut rows = stmt.raw_query();
        let mut out = Vec::new();
        while let Some(row) = rows.next().map_err(|e| e.to_string())? {
            out.push(MidiFile {
                name: row.get(0).unwrap_or_default(),
                path: row.get(1).unwrap_or_default(),
                directory: row.get(2).unwrap_or_default(),
                format: row.get(3).unwrap_or_default(),
                size: row.get::<_, i64>(4).unwrap_or(0) as u64,
                size_formatted: row.get(5).unwrap_or_default(),
                modified: row.get(6).unwrap_or_default(),
            });
        }
        Ok(MidiQueryResult {
            midi_files: out,
            total_count,
            total_count_capped,
            total_unfiltered,
        })
    }

    pub fn midi_filter_stats(
        &self,
        search: Option<&str>,
        format_filter: Option<&str>,
        search_regex: bool,
    ) -> Result<FilterStatsResult, String> {
        let conn = self.read_conn();
        let total_unfiltered: u64 = self.midi_library_total_rows(&conn)?;
        let (fts_match, like_pat, regex_pat) = classify_fts_name_path_search(search, search_regex);
        let mut where_parts = vec![MIDI_LIBRARY_IDS.to_string()];
        let mut bind_idx = 1usize;
        if fts_match.is_some() {
            // Library scope is already `MIDI_LIBRARY_IDS`; do not nest a second
            // `MAX(id) GROUP BY path` inside the FTS subquery (same semantics, worse plan).
            where_parts.push(format!(
                "id IN (SELECT rowid FROM midi_files_fts WHERE midi_files_fts MATCH ?{bind_idx})",
            ));
            bind_idx += 1;
        } else if regex_pat.is_some() {
            where_parts.push(format!(
                "((name REGEXP ?{bind_idx}) OR (path REGEXP ?{bind_idx}))"
            ));
            bind_idx += 1;
        } else if like_pat.is_some() {
            where_parts.push(format!(
                "(name LIKE ?{bind_idx} ESCAPE '\\' OR path LIKE ?{bind_idx} ESCAPE '\\')"
            ));
            bind_idx += 1;
        }
        if let Some(f) = format_filter
            && !f.is_empty() && f != "all" {
                if f.contains(',') {
                    where_parts.push(format!("format IN ({})", Self::in_list_sql(f)));
                } else {
                    where_parts.push(format!("format = ?{bind_idx}"));
                }
            }
        let where_cl = where_parts.join(" AND ");
        if let Some(m) = fts_match.as_ref() {
            let (bc, capped) = Self::midi_fts_bounded_count_library(&conn, m, format_filter)?;
            if bc == 0 {
                return Ok(FilterStatsResult {
                    count: 0,
                    count_capped: false,
                    total_bytes: 0,
                    by_type: HashMap::new(),
                    bytes_by_type: HashMap::new(),
                    total_unfiltered,
                    size_buckets: vec![],
                    ..Default::default()
                });
            }
            if capped {
                return Ok(FilterStatsResult {
                    count: bc,
                    count_capped: true,
                    total_bytes: 0,
                    by_type: HashMap::new(),
                    bytes_by_type: HashMap::new(),
                    total_unfiltered,
                    size_buckets: vec![],
                    ..Default::default()
                });
            }
        }
        let sql = format!(
            "SELECT format, COUNT(*), COALESCE(SUM(size),0) FROM midi_files WHERE {where_cl} GROUP BY format"
        );
        let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
        let mut bi = 1;
        if let Some(ref m) = fts_match {
            stmt.raw_bind_parameter(bi, m).map_err(|e| e.to_string())?;
            bi += 1;
        } else if let Some(ref r) = regex_pat {
            stmt.raw_bind_parameter(bi, r).map_err(|e| e.to_string())?;
            bi += 1;
        } else if let Some(ref pat) = like_pat {
            stmt.raw_bind_parameter(bi, pat)
                .map_err(|e| e.to_string())?;
            bi += 1;
        }
        if let Some(f) = format_filter
            && !f.is_empty() && f != "all" && !f.contains(',') {
                stmt.raw_bind_parameter(bi, f).map_err(|e| e.to_string())?;
            }
        let _ = bi;
        let mut rows = stmt.raw_query();
        let mut count = 0u64;
        let mut total_bytes = 0u64;
        let mut by_type = HashMap::new();
        let mut bytes_by_type = HashMap::new();
        while let Some(row) = rows.next().map_err(|e| e.to_string())? {
            let fmt: String = row.get(0).unwrap_or_default();
            let n: u64 = row.get::<_, i64>(1).unwrap_or(0) as u64;
            let sz: u64 = row.get::<_, i64>(2).unwrap_or(0) as u64;
            count += n;
            total_bytes += sz;
            by_type.insert(fmt.clone(), n);
            bytes_by_type.insert(fmt, sz);
        }
        Ok(FilterStatsResult {
            count,
            count_capped: false,
            total_bytes,
            by_type,
            bytes_by_type,
            total_unfiltered,
            size_buckets: vec![],
            ..Default::default()
        })
    }

    // ── Video scan CRUD (library + FTS — same model as MIDI) ──

    pub fn video_scan_parent_create(
        &self,
        id: &str,
        timestamp: &str,
        roots: &[String],
    ) -> Result<(), String> {
        let conn = self.read_conn();
        let roots_json = path_strings_json_normalized(roots);
        conn.execute(
            "INSERT OR REPLACE INTO video_scans (id, timestamp, video_count, total_bytes, format_counts, roots, scan_complete) VALUES (?1,?2,0,0,'{}',?3,0)",
            params![id, timestamp, roots_json],
        ).map_err(|e| e.to_string())?;
        conn.execute(
            "CREATE TEMP TABLE _video_lib_refresh_paths (path TEXT PRIMARY KEY)",
            [],
        )
        .map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT INTO _video_lib_refresh_paths SELECT DISTINCT path FROM video_files WHERE scan_id = ?1",
            params![id],
        )
        .map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM video_files WHERE scan_id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM video_files_fts WHERE scan_id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        Self::sync_video_library_after_paths_refresh(&conn)?;
        self.invalidate_video_library_total_cache();
        Ok(())
    }

    pub fn video_scan_parent_finalize(
        &self,
        id: &str,
        _video_count: usize,
        _total_bytes: u64,
        _format_counts: &HashMap<String, usize>,
    ) -> Result<(), String> {
        let conn = self.read_conn();
        let video_count: i64 = conn
            .query_row("SELECT COUNT(DISTINCT path) FROM video_files", [], |r| {
                r.get(0)
            })
            .unwrap_or(0);
        let total_bytes: i64 = conn
            .query_row(
                "SELECT COALESCE(SUM(size), 0) FROM video_files WHERE id IN (SELECT video_id FROM video_library)",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);
        let mut format_map: HashMap<String, usize> = HashMap::new();
        let mut stmt = conn
            .prepare(
                "SELECT format, COUNT(*) FROM video_files WHERE id IN (SELECT video_id FROM video_library) GROUP BY format",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? as usize))
            })
            .map_err(|e| e.to_string())?;
        for (fmt, n) in rows.flatten() {
            format_map.insert(fmt, n);
        }
        let fc_json = serde_json::to_string(&format_map).unwrap_or_default();
        conn.execute(
            "UPDATE video_scans SET video_count = ?2, total_bytes = ?3, format_counts = ?4 WHERE id = ?1",
            params![id, video_count, total_bytes, fc_json],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn insert_video_batch(&self, scan_id: &str, video_files: &[VideoFile]) -> Result<(), String> {
        let conn = self.read_conn();
        let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
        let mut inserted: u64 = 0;
        let mut batch_bytes: u64 = 0;
        {
            let mut stmt = tx.prepare_cached("INSERT OR IGNORE INTO video_files (name, path, directory, format, size, size_formatted, modified, scan_id) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)").map_err(|e| e.to_string())?;
            let mut fts_stmt = tx
                .prepare_cached(
                    "INSERT INTO video_files_fts(rowid, name, path, scan_id) VALUES (?1,?2,?3,?4)",
                )
                .map_err(|e| e.to_string())?;
            let mut lib_stmt = tx
                .prepare_cached(
                    "INSERT INTO video_library (path, video_id) VALUES (?1, ?2)
                     ON CONFLICT(path) DO UPDATE SET video_id = CASE
                       WHEN excluded.video_id > video_library.video_id THEN excluded.video_id
                       ELSE video_library.video_id END",
                )
                .map_err(|e| e.to_string())?;
            for m in video_files {
                let path = normalize_path_for_db(&m.path);
                let directory = normalize_path_for_db(&m.directory);
                let changed = stmt
                    .execute(params![
                        m.name,
                        path,
                        directory,
                        m.format,
                        m.size as i64,
                        m.size_formatted,
                        m.modified,
                        scan_id
                    ])
                    .map_err(|e| e.to_string())?;
                if changed > 0 {
                    let id = tx.last_insert_rowid();
                    fts_stmt
                        .execute(params![id, m.name, path, scan_id])
                        .map_err(|e| e.to_string())?;
                    lib_stmt
                        .execute(params![path, id])
                        .map_err(|e| e.to_string())?;
                    inserted += 1;
                    batch_bytes += m.size;
                }
            }
        }
        if inserted > 0 {
            tx.execute(
                "UPDATE video_scans SET video_count = video_count + ?2, total_bytes = total_bytes + ?3 WHERE id = ?1",
                params![scan_id, inserted as i64, batch_bytes as i64],
            ).map_err(|e| e.to_string())?;
        }
        tx.commit().map_err(|e| e.to_string())?;
        if inserted > 0 {
            self.invalidate_video_library_total_cache();
        }
        Ok(())
    }

    pub fn query_video(
        &self,
        search: Option<&str>,
        format_filter: Option<&str>,
        sort_key: &str,
        sort_asc: bool,
        search_regex: bool,
        offset: u64,
        limit: u64,
    ) -> Result<VideoQueryResult, String> {
        let conn = self.read_conn();
        let total_unfiltered: u64 = self.video_library_total_rows(&conn)?;
        if total_unfiltered == 0 {
            return Ok(VideoQueryResult {
                video_files: vec![],
                total_count: 0,
                total_count_capped: false,
                total_unfiltered: 0,
            });
        }

        let mut where_parts = vec![VIDEO_LIBRARY_IDS.to_string()];
        let mut bind_idx = 1usize;
        let (fts_match, like_pat, regex_pat) = classify_fts_name_path_search(search, search_regex);
        if fts_match.is_some() {
            where_parts.push(format!(
                "id IN (SELECT rowid FROM video_files_fts WHERE video_files_fts MATCH ?{bind_idx})",
            ));
            bind_idx += 1;
        } else if regex_pat.is_some() {
            where_parts.push(format!(
                "((name REGEXP ?{bind_idx}) OR (path REGEXP ?{bind_idx}))"
            ));
            bind_idx += 1;
        } else if like_pat.is_some() {
            where_parts.push(format!(
                "(name LIKE ?{bind_idx} ESCAPE '\\' OR path LIKE ?{bind_idx} ESCAPE '\\')"
            ));
            bind_idx += 1;
        }
        if let Some(f) = format_filter
            && !f.is_empty() && f != "all" {
                if f.contains(',') {
                    where_parts.push(format!(
                        "format IN ({})",
                        f.split(',')
                            .map(|s| format!("'{}'", s.trim().replace('\'', "''")))
                            .collect::<Vec<_>>()
                            .join(",")
                    ));
                } else {
                    where_parts.push(format!("format = ?{bind_idx}"));
                    bind_idx += 1;
                }
            }
        let where_cl = where_parts.join(" AND ");

        let sort_col = match sort_key {
            "name" => "name COLLATE NOCASE",
            "size" => "size",
            "modified" => "modified",
            "directory" => "directory COLLATE NOCASE",
            "format" => "format",
            _ => "name COLLATE NOCASE",
        };
        let dir = if sort_asc { "ASC" } else { "DESC" };

        let use_fts_rank_page = fts_match.is_some();

        let (total_count, total_count_capped) = if let Some(m) = fts_match.as_ref() {
            Self::video_fts_bounded_count_library(&conn, m, format_filter)?
        } else {
            let sql = format!("SELECT COUNT(*) FROM video_files WHERE {where_cl}");
            let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
            let mut bi = 1;
            if let Some(ref r) = regex_pat {
                stmt.raw_bind_parameter(bi, r).map_err(|e| e.to_string())?;
                bi += 1;
            } else if let Some(ref pat) = like_pat {
                stmt.raw_bind_parameter(bi, pat)
                    .map_err(|e| e.to_string())?;
                bi += 1;
            }
            if let Some(f) = format_filter
                && !f.is_empty() && f != "all" && !f.contains(',') {
                    stmt.raw_bind_parameter(bi, f).map_err(|e| e.to_string())?;
                }
            let mut rows = stmt.raw_query();
            let n = rows
                .next()
                .map_err(|e| e.to_string())?
                .map(|r| r.get::<_, i64>(0).unwrap_or(0) as u64)
                .unwrap_or(0);
            (n, false)
        };

        let sql = if use_fts_rank_page {
            let mut w = String::from(
                "SELECT v.name, v.path, v.directory, v.format, v.size, v.size_formatted, v.modified
                 FROM video_files_fts
                 INNER JOIN video_files v ON v.id = video_files_fts.rowid
                 INNER JOIN video_library lib ON lib.video_id = v.id
                 WHERE video_files_fts MATCH ?1",
            );
            let mut next_ph = 2usize;
            if let Some(f) = format_filter
                && !f.is_empty() && f != "all" {
                    if f.contains(',') {
                        let vals: Vec<String> = f
                            .split(',')
                            .map(|s| format!("'{}'", s.trim().replace('\'', "''")))
                            .collect();
                        w.push_str(&format!(" AND v.format IN ({})", vals.join(",")));
                    } else {
                        w.push_str(&format!(" AND v.format = ?{next_ph}"));
                        next_ph += 1;
                    }
                }
            let li = next_ph;
            let oi = next_ph + 1;
            w.push_str(&format!(
                " ORDER BY bm25(video_files_fts) LIMIT ?{li} OFFSET ?{oi}"
            ));
            w
        } else {
            format!(
                "SELECT name, path, directory, format, size, size_formatted, modified
                 FROM video_files WHERE {where_cl}
                 ORDER BY {sort_col} {dir} LIMIT ?{limit_idx} OFFSET ?{off_idx}",
                limit_idx = bind_idx,
                off_idx = bind_idx + 1
            )
        };
        let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
        let mut bi = 1;
        if use_fts_rank_page {
            let m = fts_match.as_ref().expect("fts");
            stmt.raw_bind_parameter(bi, m).map_err(|e| e.to_string())?;
            bi += 1;
            if let Some(f) = format_filter
                && !f.is_empty() && f != "all" && !f.contains(',') {
                    stmt.raw_bind_parameter(bi, f).map_err(|e| e.to_string())?;
                    bi += 1;
                }
            stmt.raw_bind_parameter(bi, limit as i64)
                .map_err(|e| e.to_string())?;
            stmt.raw_bind_parameter(bi + 1, offset as i64)
                .map_err(|e| e.to_string())?;
        } else if let Some(ref m) = fts_match {
            stmt.raw_bind_parameter(bi, m).map_err(|e| e.to_string())?;
            bi += 1;
        } else if let Some(ref r) = regex_pat {
            stmt.raw_bind_parameter(bi, r).map_err(|e| e.to_string())?;
            bi += 1;
        } else if let Some(ref pat) = like_pat {
            stmt.raw_bind_parameter(bi, pat)
                .map_err(|e| e.to_string())?;
            bi += 1;
        }
        if !use_fts_rank_page {
            if let Some(f) = format_filter
                && !f.is_empty() && f != "all" && !f.contains(',') {
                    stmt.raw_bind_parameter(bi, f).map_err(|e| e.to_string())?;
                    bi += 1;
                }
            stmt.raw_bind_parameter(bi, limit as i64)
                .map_err(|e| e.to_string())?;
            stmt.raw_bind_parameter(bi + 1, offset as i64)
                .map_err(|e| e.to_string())?;
        }
        let mut rows = stmt.raw_query();
        let mut out = Vec::new();
        while let Some(row) = rows.next().map_err(|e| e.to_string())? {
            out.push(VideoFile {
                name: row.get(0).unwrap_or_default(),
                path: row.get(1).unwrap_or_default(),
                directory: row.get(2).unwrap_or_default(),
                format: row.get(3).unwrap_or_default(),
                size: row.get::<_, i64>(4).unwrap_or(0) as u64,
                size_formatted: row.get(5).unwrap_or_default(),
                modified: row.get(6).unwrap_or_default(),
            });
        }
        Ok(VideoQueryResult {
            video_files: out,
            total_count,
            total_count_capped,
            total_unfiltered,
        })
    }

    pub fn video_filter_stats(
        &self,
        search: Option<&str>,
        format_filter: Option<&str>,
        search_regex: bool,
    ) -> Result<FilterStatsResult, String> {
        let conn = self.read_conn();
        let total_unfiltered: u64 = self.video_library_total_rows(&conn)?;
        let (fts_match, like_pat, regex_pat) = classify_fts_name_path_search(search, search_regex);
        let mut where_parts = vec![VIDEO_LIBRARY_IDS.to_string()];
        let mut bind_idx = 1usize;
        if fts_match.is_some() {
            where_parts.push(format!(
                "id IN (SELECT rowid FROM video_files_fts WHERE video_files_fts MATCH ?{bind_idx})",
            ));
            bind_idx += 1;
        } else if regex_pat.is_some() {
            where_parts.push(format!(
                "((name REGEXP ?{bind_idx}) OR (path REGEXP ?{bind_idx}))"
            ));
            bind_idx += 1;
        } else if like_pat.is_some() {
            where_parts.push(format!(
                "(name LIKE ?{bind_idx} ESCAPE '\\' OR path LIKE ?{bind_idx} ESCAPE '\\')"
            ));
            bind_idx += 1;
        }
        if let Some(f) = format_filter
            && !f.is_empty() && f != "all" {
                if f.contains(',') {
                    where_parts.push(format!("format IN ({})", Self::in_list_sql(f)));
                } else {
                    where_parts.push(format!("format = ?{bind_idx}"));
                }
            }
        let where_cl = where_parts.join(" AND ");
        if let Some(m) = fts_match.as_ref() {
            let (bc, capped) = Self::video_fts_bounded_count_library(&conn, m, format_filter)?;
            if bc == 0 {
                return Ok(FilterStatsResult {
                    count: 0,
                    count_capped: false,
                    total_bytes: 0,
                    by_type: HashMap::new(),
                    bytes_by_type: HashMap::new(),
                    total_unfiltered,
                    size_buckets: vec![],
                    ..Default::default()
                });
            }
            if capped {
                return Ok(FilterStatsResult {
                    count: bc,
                    count_capped: true,
                    total_bytes: 0,
                    by_type: HashMap::new(),
                    bytes_by_type: HashMap::new(),
                    total_unfiltered,
                    size_buckets: vec![],
                    ..Default::default()
                });
            }
        }
        let sql = format!(
            "SELECT format, COUNT(*), COALESCE(SUM(size),0) FROM video_files WHERE {where_cl} GROUP BY format"
        );
        let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
        let mut bi = 1;
        if let Some(ref m) = fts_match {
            stmt.raw_bind_parameter(bi, m).map_err(|e| e.to_string())?;
            bi += 1;
        } else if let Some(ref r) = regex_pat {
            stmt.raw_bind_parameter(bi, r).map_err(|e| e.to_string())?;
            bi += 1;
        } else if let Some(ref pat) = like_pat {
            stmt.raw_bind_parameter(bi, pat)
                .map_err(|e| e.to_string())?;
            bi += 1;
        }
        if let Some(f) = format_filter
            && !f.is_empty() && f != "all" && !f.contains(',') {
                stmt.raw_bind_parameter(bi, f).map_err(|e| e.to_string())?;
            }
        let _ = bi;
        let mut rows = stmt.raw_query();
        let mut count = 0u64;
        let mut total_bytes = 0u64;
        let mut by_type = HashMap::new();
        let mut bytes_by_type = HashMap::new();
        while let Some(row) = rows.next().map_err(|e| e.to_string())? {
            let fmt: String = row.get(0).unwrap_or_default();
            let n: u64 = row.get::<_, i64>(1).unwrap_or(0) as u64;
            let sz: u64 = row.get::<_, i64>(2).unwrap_or(0) as u64;
            count += n;
            total_bytes += sz;
            by_type.insert(fmt.clone(), n);
            bytes_by_type.insert(fmt, sz);
        }
        Ok(FilterStatsResult {
            count,
            count_capped: false,
            total_bytes,
            by_type,
            bytes_by_type,
            total_unfiltered,
            size_buckets: vec![],
            ..Default::default()
        })
    }

    // ── PDF scan CRUD ──

    pub fn pdf_scan_parent_create(
        &self,
        id: &str,
        timestamp: &str,
        roots: &[String],
    ) -> Result<(), String> {
        let conn = self.read_conn();
        let roots_json = path_strings_json_normalized(roots);
        conn.execute(
            "INSERT OR REPLACE INTO pdf_scans (id, timestamp, pdf_count, total_bytes, roots, scan_complete) VALUES (?1,?2,0,0,?3,0)",
            params![id, timestamp, roots_json],
        ).map_err(|e| e.to_string())?;
        conn.execute(
            "CREATE TEMP TABLE _pdf_lib_refresh_paths (path TEXT PRIMARY KEY)",
            [],
        )
        .map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT INTO _pdf_lib_refresh_paths SELECT DISTINCT path FROM pdfs WHERE scan_id = ?1",
            params![id],
        )
        .map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM pdfs WHERE scan_id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM pdfs_fts WHERE scan_id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        Self::sync_pdf_library_after_paths_refresh(&conn)?;
        self.invalidate_pdf_library_total_cache();
        Ok(())
    }

    pub fn pdf_scan_parent_finalize(
        &self,
        id: &str,
        _pdf_count: usize,
        _total_bytes: u64,
    ) -> Result<(), String> {
        let conn = self.read_conn();
        let pdf_count: i64 = conn
            .query_row("SELECT COUNT(DISTINCT path) FROM pdfs", [], |r| r.get(0))
            .unwrap_or(0);
        let total_bytes: i64 = conn
            .query_row(
                "SELECT COALESCE(SUM(size), 0) FROM pdfs WHERE id IN (SELECT pdf_id FROM pdf_library)",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);
        conn.execute(
            "UPDATE pdf_scans SET pdf_count = ?2, total_bytes = ?3 WHERE id = ?1",
            params![id, pdf_count, total_bytes],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn insert_pdf_batch(&self, scan_id: &str, pdfs: &[PdfFile]) -> Result<u64, String> {
        let conn = self.read_conn();
        let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
        let mut inserted: u64 = 0;
        let mut batch_bytes: u64 = 0;
        {
            let mut stmt = tx.prepare_cached("INSERT OR IGNORE INTO pdfs (name, path, directory, size, size_formatted, modified, scan_id) VALUES (?1,?2,?3,?4,?5,?6,?7)").map_err(|e| e.to_string())?;
            let mut fts_stmt = tx
                .prepare_cached(
                    "INSERT INTO pdfs_fts(rowid, name, path, scan_id) VALUES (?1,?2,?3,?4)",
                )
                .map_err(|e| e.to_string())?;
            let mut lib_stmt = tx
                .prepare_cached(
                    "INSERT INTO pdf_library (path, pdf_id) VALUES (?1, ?2)
                     ON CONFLICT(path) DO UPDATE SET pdf_id = CASE
                       WHEN excluded.pdf_id > pdf_library.pdf_id THEN excluded.pdf_id
                       ELSE pdf_library.pdf_id END",
                )
                .map_err(|e| e.to_string())?;
            for p in pdfs {
                let path = normalize_path_for_db(&p.path);
                let directory = normalize_path_for_db(&p.directory);
                let changed = stmt
                    .execute(params![
                        p.name,
                        path,
                        directory,
                        p.size as i64,
                        p.size_formatted,
                        p.modified,
                        scan_id
                    ])
                    .map_err(|e| e.to_string())?;
                if changed > 0 {
                    let id = tx.last_insert_rowid();
                    fts_stmt
                        .execute(params![id, p.name, path, scan_id])
                        .map_err(|e| e.to_string())?;
                    lib_stmt
                        .execute(params![path, id])
                        .map_err(|e| e.to_string())?;
                    inserted += 1;
                    batch_bytes += p.size;
                }
            }
        }
        if inserted > 0 {
            tx.execute(
                "UPDATE pdf_scans SET pdf_count = pdf_count + ?2, total_bytes = total_bytes + ?3 WHERE id = ?1",
                params![scan_id, inserted as i64, batch_bytes as i64],
            ).map_err(|e| e.to_string())?;
        }
        tx.commit().map_err(|e| e.to_string())?;
        if inserted > 0 {
            self.invalidate_pdf_library_total_cache();
        }
        Ok(inserted)
    }

    // ── Directory scan state (incremental unified walker) ──

    /// Load stored directory mtimes for a domain (e.g. `"unified"`).
    pub fn load_directory_scan_snapshot(
        &self,
        domain: &str,
    ) -> Result<HashMap<String, i64>, String> {
        let conn = self.read_conn();
        let mut stmt = conn
            .prepare("SELECT path, mtime_secs FROM directory_scan_state WHERE domain = ?1")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![domain], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
            })
            .map_err(|e| e.to_string())?;
        let mut out = HashMap::new();
        for r in rows {
            let (p, m) = r.map_err(|e| e.to_string())?;
            out.insert(p, m);
        }
        Ok(out)
    }

    pub fn upsert_directory_scan_batch(
        &self,
        domain: &str,
        rows: &[(String, i64)],
        last_scan_id: Option<&str>,
    ) -> Result<(), String> {
        if rows.is_empty() {
            return Ok(());
        }
        let conn = self.read_conn();
        let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
        {
            let mut stmt = tx
                .prepare_cached(
                    "INSERT OR REPLACE INTO directory_scan_state (domain, path, mtime_secs, last_scan_id) VALUES (?1, ?2, ?3, ?4)",
                )
                .map_err(|e| e.to_string())?;
            for (path, mtime_secs) in rows {
                let path = normalize_path_for_db(path);
                stmt.execute(params![domain, path, mtime_secs, last_scan_id])
                    .map_err(|e| e.to_string())?;
            }
        }
        tx.commit().map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Remove all incremental directory rows for a domain (e.g. `"unified"`).
    pub fn delete_directory_scan_state_domain(&self, domain: &str) -> Result<u64, String> {
        let conn = self.read_conn();
        let n = conn
            .execute(
                "DELETE FROM directory_scan_state WHERE domain = ?1",
                params![domain],
            )
            .map_err(|e| e.to_string())?;
        Ok(n as u64)
    }

    /// True when the last persisted unified scan finished successfully; incremental mtime
    /// snapshots are only trusted in that case.
    pub fn unified_scan_incremental_snapshot_is_trusted(&self) -> Result<bool, String> {
        let conn = self.read_conn();
        let outcome: String = conn
            .query_row(
                "SELECT outcome FROM unified_scan_run WHERE id = 1",
                [],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;
        Ok(outcome == "complete")
    }

    pub fn unified_scan_run_start(
        &self,
        run_id: &str,
        started_at: &str,
        audio_scan_id: &str,
        daw_scan_id: &str,
        preset_scan_id: &str,
        pdf_scan_id: &str,
        roots_json: &str,
    ) -> Result<(), String> {
        let conn = self.read_conn();
        conn.execute(
            "INSERT INTO unified_scan_run (id, run_id, started_at, finished_at, outcome, audio_scan_id, daw_scan_id, preset_scan_id, pdf_scan_id, roots_json, last_directory_path, error_message)
             VALUES (1, ?1, ?2, NULL, 'in_progress', ?3, ?4, ?5, ?6, ?7, NULL, NULL)
             ON CONFLICT(id) DO UPDATE SET
               run_id = excluded.run_id,
               started_at = excluded.started_at,
               finished_at = NULL,
               outcome = 'in_progress',
               audio_scan_id = excluded.audio_scan_id,
               daw_scan_id = excluded.daw_scan_id,
               preset_scan_id = excluded.preset_scan_id,
               pdf_scan_id = excluded.pdf_scan_id,
               roots_json = excluded.roots_json,
               last_directory_path = NULL,
               error_message = NULL",
            params![
                run_id,
                started_at,
                audio_scan_id,
                daw_scan_id,
                preset_scan_id,
                pdf_scan_id,
                roots_json,
            ],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Persists final unified scan outcome. When `outcome` is not `complete`, clears incremental
    /// `directory_scan_state` rows for domain `"unified"` so partial walks are not reused.
    pub fn unified_scan_run_finish(
        &self,
        finished_at: &str,
        outcome: &str,
        error_message: Option<&str>,
        last_directory_path: Option<&str>,
    ) -> Result<(), String> {
        let conn = self.read_conn();
        conn.execute(
            "UPDATE unified_scan_run SET finished_at = ?1, outcome = ?2, error_message = ?3, last_directory_path = ?4 WHERE id = 1",
            params![
                finished_at,
                outcome,
                error_message,
                last_directory_path,
            ],
        )
        .map_err(|e| e.to_string())?;
        if outcome != "complete" {
            let _ = conn.execute(
                "DELETE FROM directory_scan_state WHERE domain = ?1",
                params![crate::DIRECTORY_SCAN_INCREMENTAL_DOMAIN],
            );
        }
        Ok(())
    }

    pub fn get_unified_scan_run(&self) -> Result<UnifiedScanRunRow, String> {
        let conn = self.read_conn();
        conn.query_row(
            "SELECT run_id, started_at, finished_at, outcome, audio_scan_id, daw_scan_id, preset_scan_id, pdf_scan_id, roots_json, last_directory_path, error_message FROM unified_scan_run WHERE id = 1",
            [],
            |row| {
                Ok(UnifiedScanRunRow {
                    run_id: row.get(0)?,
                    started_at: row.get(1)?,
                    finished_at: row.get(2)?,
                    outcome: row.get(3)?,
                    audio_scan_id: row.get(4)?,
                    daw_scan_id: row.get(5)?,
                    preset_scan_id: row.get(6)?,
                    pdf_scan_id: row.get(7)?,
                    roots_json: row.get(8)?,
                    last_directory_path: row.get(9)?,
                    error_message: row.get(10)?,
                })
            },
        )
        .map_err(|e| e.to_string())
    }

    pub fn save_pdf_scan(&self, snap: &PdfScanSnapshot) -> Result<(), String> {
        let conn = self.read_conn();
        let roots_json = path_strings_json_normalized(&snap.roots);
        conn.execute(
            "INSERT OR REPLACE INTO pdf_scans (id, timestamp, pdf_count, total_bytes, roots, scan_complete) VALUES (?1,?2,?3,?4,?5,1)",
            params![snap.id, snap.timestamp, snap.pdf_count as i64, snap.total_bytes as i64, roots_json],
        ).map_err(|e| e.to_string())?;
        let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
        {
            tx.execute(
                "CREATE TEMP TABLE _pdf_lib_refresh_paths (path TEXT PRIMARY KEY)",
                [],
            )
            .map_err(|e| e.to_string())?;
            tx.execute(
                "INSERT INTO _pdf_lib_refresh_paths SELECT DISTINCT path FROM pdfs WHERE scan_id = ?1",
                params![snap.id],
            )
            .map_err(|e| e.to_string())?;
            for p in &snap.pdfs {
                let path = normalize_path_for_db(&p.path);
                tx.execute(
                    "INSERT OR IGNORE INTO _pdf_lib_refresh_paths (path) VALUES (?1)",
                    params![path],
                )
                .map_err(|e| e.to_string())?;
            }
            tx.execute("DELETE FROM pdfs WHERE scan_id = ?1", params![snap.id])
                .map_err(|e| e.to_string())?;
            tx.execute("DELETE FROM pdfs_fts WHERE scan_id = ?1", params![snap.id])
                .map_err(|e| e.to_string())?;
            let mut stmt = tx.prepare_cached("INSERT OR IGNORE INTO pdfs (name, path, directory, size, size_formatted, modified, scan_id) VALUES (?1,?2,?3,?4,?5,?6,?7)").map_err(|e| e.to_string())?;
            let mut fts_stmt = tx
                .prepare_cached(
                    "INSERT INTO pdfs_fts(rowid, name, path, scan_id) VALUES (?1,?2,?3,?4)",
                )
                .map_err(|e| e.to_string())?;
            for p in &snap.pdfs {
                let path = normalize_path_for_db(&p.path);
                let directory = normalize_path_for_db(&p.directory);
                let changed = stmt
                    .execute(params![
                        p.name,
                        path,
                        directory,
                        p.size as i64,
                        p.size_formatted,
                        p.modified,
                        snap.id
                    ])
                    .map_err(|e| e.to_string())?;
                if changed > 0 {
                    let id = tx.last_insert_rowid();
                    fts_stmt
                        .execute(params![id, p.name, path, snap.id])
                        .map_err(|e| e.to_string())?;
                }
            }
            Self::sync_pdf_library_after_paths_refresh_tx(&tx)?;
        }
        tx.commit().map_err(|e| e.to_string())?;
        self.invalidate_pdf_library_total_cache();
        Ok(())
    }

    pub fn get_pdf_scans(&self) -> Result<Vec<serde_json::Value>, String> {
        let conn = self.read_conn();
        let mut stmt = conn.prepare(
            "SELECT s.id, s.timestamp, COALESCE(NULLIF(s.pdf_count,0),(SELECT COUNT(*) FROM pdfs WHERE scan_id = s.id)), COALESCE(NULLIF(s.total_bytes,0),(SELECT COALESCE(SUM(size),0) FROM pdfs WHERE scan_id = s.id)), s.roots FROM pdf_scans s WHERE s.scan_complete = 1 ORDER BY s.timestamp DESC",
        )
        .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| {
                let roots_str: String = row.get(4)?;
                Ok(serde_json::json!({
                    "id": row.get::<_,String>(0)?,
                    "timestamp": row.get::<_,String>(1)?,
                    "pdfCount": row.get::<_, i64>(2)? as u64,
                    "totalBytes": row.get::<_, i64>(3)? as u64,
                    "roots": serde_json::from_str::<Vec<String>>(&roots_str).unwrap_or_default(),
                }))
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    }

    pub fn get_pdf_scan_detail(&self, id: &str) -> Result<PdfScanSnapshot, String> {
        let conn = self.read_conn();
        let (ts, pc, tb, roots_str): (String, usize, u64, String) = conn
            .query_row(
                "SELECT timestamp, pdf_count, total_bytes, roots FROM pdf_scans WHERE id = ?1",
                params![id],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get::<_, i64>(1)? as usize,
                        row.get::<_, i64>(2)? as u64,
                        row.get(3)?,
                    ))
                },
            )
            .map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare("SELECT name, path, directory, size, size_formatted, modified FROM pdfs WHERE scan_id = ?1")
            .map_err(|e| e.to_string())?;
        let pdfs = stmt
            .query_map(params![id], |row| {
                Ok(PdfFile {
                    name: row.get(0)?,
                    path: row.get(1)?,
                    directory: row.get(2)?,
                    size: row.get::<_, i64>(3).unwrap_or(0) as u64,
                    size_formatted: row.get(4)?,
                    modified: row.get(5)?,
                    ..Default::default()
                })
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        let live_count = pdfs.len();
        let live_bytes: u64 = pdfs.iter().map(|p| p.size).sum();
        Ok(PdfScanSnapshot {
            id: id.to_string(),
            timestamp: ts,
            pdf_count: if pc > 0 { pc } else { live_count },
            total_bytes: if tb > 0 { tb } else { live_bytes },
            pdfs,
            roots: serde_json::from_str(&roots_str).unwrap_or_default(),
        })
    }

    pub fn get_latest_pdf_scan(&self) -> Result<Option<PdfScanSnapshot>, String> {
        let conn = self.read_conn();
        let id: Option<String> = conn
            .query_row(
                "SELECT id FROM pdf_scans WHERE scan_complete = 1 ORDER BY timestamp DESC LIMIT 1",
                [],
                |r| r.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())?;
        drop(conn);
        match id {
            Some(id) => self.get_pdf_scan_detail(&id).map(Some),
            None => Ok(None),
        }
    }

    pub fn delete_pdf_scan(&self, id: &str) -> Result<(), String> {
        let conn = self.read_conn();
        conn.execute(
            "CREATE TEMP TABLE _pdf_lib_refresh_paths (path TEXT PRIMARY KEY)",
            [],
        )
        .map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT INTO _pdf_lib_refresh_paths SELECT DISTINCT path FROM pdfs WHERE scan_id = ?1",
            params![id],
        )
        .map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM pdfs WHERE scan_id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM pdfs_fts WHERE scan_id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        Self::sync_pdf_library_after_paths_refresh(&conn)?;
        conn.execute("DELETE FROM pdf_scans WHERE id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        self.invalidate_pdf_library_total_cache();
        Ok(())
    }

    pub fn clear_pdf_history(&self) -> Result<(), String> {
        let conn = self.read_conn();
        conn.execute_batch(
            "BEGIN IMMEDIATE;
             DELETE FROM pdf_library;
             DELETE FROM pdfs_fts;
             DELETE FROM pdfs;
             DELETE FROM pdf_scans;
             COMMIT;",
        )
        .map_err(|e| e.to_string())?;
        self.invalidate_pdf_library_total_cache();
        Ok(())
    }

    // ── PDF metadata (page count) ──

    /// Paths in the materialized PDF **library** (newest row per filesystem path) that have no
    /// `pdf_metadata` row yet. Uses `pdf_library`, not “latest scan only”, so PDFs whose
    /// canonical `pdfs` row belongs to an older completed scan are still queued for page counts.
    pub fn unindexed_pdf_paths(&self, limit: u64) -> Result<Vec<String>, String> {
        let conn = self.read_conn();
        let mut stmt = conn
            .prepare(
                "SELECT p.path FROM pdfs p
             INNER JOIN pdf_library lib ON lib.pdf_id = p.id
             LEFT JOIN pdf_metadata m ON m.path = p.path
             WHERE m.path IS NULL
             LIMIT ?1",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![limit as i64], |row| row.get::<_, String>(0))
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    }

    /// Batch upsert PDF metadata. Entries with None page count are still
    /// inserted so we don't re-attempt broken files.
    pub fn save_pdf_metadata(
        &self,
        batch: &[(String, Option<u32>, Option<String>, Option<String>)],
    ) -> Result<(), String> {
        if batch.is_empty() {
            return Ok(());
        }
        let conn = self.read_conn();
        let now = chrono::Utc::now()
            .format("%Y-%m-%dT%H:%M:%S%.3fZ")
            .to_string();
        let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
        {
            let mut stmt = tx
                .prepare_cached(
                    "INSERT OR REPLACE INTO pdf_metadata (path, pages, pdf_creation_date, pdf_mod_date, updated_at) VALUES (?1, ?2, ?3, ?4, ?5)",
                )
                .map_err(|e| e.to_string())?;
            for (path, pages, pdf_creation_date, pdf_mod_date) in batch {
                let path = normalize_path_for_db(path);
                let pages_i: Option<i64> = pages.map(|n| n as i64);
                stmt.execute(params![
                    path,
                    pages_i,
                    pdf_creation_date.as_ref(),
                    pdf_mod_date.as_ref(),
                    now
                ])
                .map_err(|e| e.to_string())?;
            }
        }
        tx.commit().map_err(|e| e.to_string())
    }

    /// Get stored PDF metadata for a set of paths (returns only rows that exist).
    pub fn get_pdf_metadata(
        &self,
        paths: &[String],
    ) -> Result<std::collections::HashMap<String, PdfMetaDbRow>, String> {
        if paths.is_empty() {
            return Ok(std::collections::HashMap::new());
        }
        let conn = self.read_conn();
        let mut out = std::collections::HashMap::new();
        // SQLite IN clause with ~999 param limit — chunk to be safe.
        for chunk in paths.chunks(500) {
            let placeholders: Vec<String> = (1..=chunk.len()).map(|i| format!("?{i}")).collect();
            let sql = format!(
                "SELECT path, pages, pdf_creation_date, pdf_mod_date FROM pdf_metadata WHERE path IN ({})",
                placeholders.join(",")
            );
            let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
            for (i, p) in chunk.iter().enumerate() {
                let p = normalize_path_for_db(p);
                stmt.raw_bind_parameter(i + 1, p)
                    .map_err(|e| e.to_string())?;
            }
            let mut rows = stmt.raw_query();
            while let Some(row) = rows.next().map_err(|e| e.to_string())? {
                let path: String = row.get(0).unwrap_or_default();
                let pages: Option<i64> = row.get(1).ok();
                let pdf_creation_date: Option<String> = row.get(2).ok();
                let pdf_mod_date: Option<String> = row.get(3).ok();
                out.insert(
                    path,
                    PdfMetaDbRow {
                        pages: pages.and_then(|n| if n >= 0 { Some(n as u32) } else { None }),
                        pdf_creation_date,
                        pdf_mod_date,
                    },
                );
            }
        }
        Ok(out)
    }

    pub fn clear_pdf_metadata(&self) -> Result<(), String> {
        let conn = self.read_conn();
        conn.execute_batch("DELETE FROM pdf_metadata;")
            .map_err(|e| e.to_string())
    }

    // ── Paginated PDF query ──
    pub fn query_pdfs(
        &self,
        search: Option<&str>,
        sort_key: &str,
        sort_asc: bool,
        search_regex: bool,
        offset: u64,
        limit: u64,
    ) -> Result<PdfQueryResult, String> {
        let conn = self.read_conn();
        let total_unfiltered: u64 = self.pdf_library_total_rows(&conn)?;
        if total_unfiltered == 0 {
            return Ok(PdfQueryResult {
                pdfs: vec![],
                total_count: 0,
                total_count_capped: false,
                total_unfiltered: 0,
            });
        }

        let mut where_parts = vec![PDF_LIBRARY_IDS.to_string()];
        let mut bind_idx = 1usize;
        let (fts_match, like_pat, regex_pat) = classify_fts_name_path_search(search, search_regex);
        if fts_match.is_some() {
            // Library scope is already `PDF_LIBRARY_IDS`; do not nest a second
            // `MAX(id) GROUP BY path` inside the FTS subquery (same semantics, worse plan).
            where_parts.push(format!(
                "id IN (SELECT rowid FROM pdfs_fts WHERE pdfs_fts MATCH ?{bind_idx})",
            ));
            bind_idx += 1;
        } else if regex_pat.is_some() {
            where_parts.push(format!(
                "((name REGEXP ?{bind_idx}) OR (path REGEXP ?{bind_idx}))"
            ));
            bind_idx += 1;
        } else if like_pat.is_some() {
            where_parts.push(format!(
                "(name LIKE ?{bind_idx} ESCAPE '\\' OR path LIKE ?{bind_idx} ESCAPE '\\')"
            ));
            bind_idx += 1;
        }
        let where_cl = where_parts.join(" AND ");

        let sort_col = match sort_key {
            "name" => "name COLLATE NOCASE",
            "size" => "size",
            "modified" => "modified",
            "directory" => "directory COLLATE NOCASE",
            _ => "name COLLATE NOCASE",
        };
        let dir = if sort_asc { "ASC" } else { "DESC" };

        let (total_count, total_count_capped) = if let Some(m) = fts_match.as_ref() {
            Self::pdf_fts_bounded_count_library(&conn, m)?
        } else {
            let sql = format!("SELECT COUNT(*) FROM pdfs WHERE {where_cl}");
            let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
            let bi = 1;
            if let Some(ref r) = regex_pat {
                stmt.raw_bind_parameter(bi, r).map_err(|e| e.to_string())?;
            } else if let Some(ref pat) = like_pat {
                stmt.raw_bind_parameter(bi, pat)
                    .map_err(|e| e.to_string())?;
            }
            let _ = bi;
            let mut rows = stmt.raw_query();
            let n = rows
                .next()
                .map_err(|e| e.to_string())?
                .map(|r| r.get::<_, i64>(0).unwrap_or(0) as u64)
                .unwrap_or(0);
            (n, false)
        };

        // FTS substring search: ORDER BY column sorts the entire match set before LIMIT (stalls).
        // Use bm25 + LIMIT like MIDI; frontend may re-rank with `searchScore`.
        let use_fts_bm25 = fts_match.is_some();
        let sql = if use_fts_bm25 {
            String::from(
                "SELECT p.name, p.path, p.directory, p.size, p.size_formatted, p.modified
                 FROM pdfs_fts
                 INNER JOIN pdfs p ON p.id = pdfs_fts.rowid
                 INNER JOIN pdf_library lib ON lib.pdf_id = p.id
                 WHERE pdfs_fts MATCH ?1
                 ORDER BY bm25(pdfs_fts) LIMIT ?2 OFFSET ?3",
            )
        } else {
            format!(
                "SELECT name, path, directory, size, size_formatted, modified FROM pdfs WHERE {where_cl} ORDER BY {sort_col} {dir} LIMIT ?{bind_idx} OFFSET ?{}",
                bind_idx + 1
            )
        };
        let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
        let mut bi = 1;
        if use_fts_bm25 {
            let m = fts_match.as_ref().expect("fts");
            stmt.raw_bind_parameter(bi, m).map_err(|e| e.to_string())?;
            stmt.raw_bind_parameter(bi + 1, limit as i64)
                .map_err(|e| e.to_string())?;
            stmt.raw_bind_parameter(bi + 2, offset as i64)
                .map_err(|e| e.to_string())?;
        } else if let Some(ref r) = regex_pat {
            stmt.raw_bind_parameter(bi, r).map_err(|e| e.to_string())?;
            bi += 1;
        } else if let Some(ref pat) = like_pat {
            stmt.raw_bind_parameter(bi, pat)
                .map_err(|e| e.to_string())?;
            bi += 1;
        }
        if !use_fts_bm25 {
            stmt.raw_bind_parameter(bi, limit as i64)
                .map_err(|e| e.to_string())?;
            stmt.raw_bind_parameter(bi + 1, offset as i64)
                .map_err(|e| e.to_string())?;
        }

        let mut pdfs = Vec::new();
        let mut rows = stmt.raw_query();
        while let Some(row) = rows.next().map_err(|e| e.to_string())? {
            pdfs.push(PdfRow {
                name: row.get(0).unwrap_or_default(),
                path: row.get(1).unwrap_or_default(),
                directory: row.get(2).unwrap_or_default(),
                size: row.get::<_, i64>(3).unwrap_or(0) as u64,
                size_formatted: row.get(4).unwrap_or_default(),
                modified: row.get(5).unwrap_or_default(),
            });
        }
        Ok(PdfQueryResult {
            pdfs,
            total_count,
            total_count_capped,
            total_unfiltered,
        })
    }

    /// PDF aggregate stats. `scan_id` None or empty → full library (deduped by path).
    pub fn pdf_stats(&self, scan_id: Option<&str>) -> Result<PdfStatsResult, String> {
        let conn = self.read_conn();
        let library = scan_id.map(|s| s.is_empty()).unwrap_or(true);
        if library {
            let pdf_count: u64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM pdfs WHERE id IN (SELECT pdf_id FROM pdf_library)",
                    [],
                    |row| row.get::<_, i64>(0).map(|v| v as u64),
                )
                .unwrap_or(0);
            let total_bytes: u64 = conn
                .query_row(
                    "SELECT COALESCE(SUM(size), 0) FROM pdfs WHERE id IN (SELECT pdf_id FROM pdf_library)",
                    [],
                    |row| row.get::<_, i64>(0).map(|v| v as u64),
                )
                .unwrap_or(0);
            return Ok(PdfStatsResult {
                pdf_count,
                total_bytes,
            });
        }

        let sid = scan_id.expect("scan_id").to_string();
        if sid.is_empty() {
            return Ok(PdfStatsResult {
                pdf_count: 0,
                total_bytes: 0,
            });
        }
        let pdf_count: u64 = conn
            .query_row(
                "SELECT COUNT(*) FROM pdfs WHERE scan_id = ?1",
                params![sid],
                |row| row.get::<_, i64>(0).map(|v| v as u64),
            )
            .map_err(|e| e.to_string())?;
        let total_bytes: u64 = conn
            .query_row(
                "SELECT COALESCE(SUM(size), 0) FROM pdfs WHERE scan_id = ?1",
                params![sid],
                |row| row.get::<_, i64>(0).map(|v| v as u64),
            )
            .map_err(|e| e.to_string())?;
        Ok(PdfStatsResult {
            pdf_count,
            total_bytes,
        })
    }

    // ── Filter-aware aggregate stats ──
    // Each returns count + total_bytes + per-type breakdown reflecting the active
    // search/filter over library rows (deduped by path). Uses GROUP BY for the breakdown.

    fn in_list_sql(values: &str) -> String {
        values
            .split(',')
            .map(|s| format!("'{}'", s.trim().replace('\'', "''")))
            .collect::<Vec<_>>()
            .join(",")
    }

    /// First three path segments of `directory`, matching `heatmap-dashboard.js` `buildFolderCard`.
    fn heatmap_audio_folder_key(dir: &str) -> String {
        let parts: Vec<&str> = dir
            .split(['/', '\\'])
            .filter(|s| !s.is_empty())
            .collect();
        let n = parts.len().min(3);
        if n == 0 {
            return "/".to_string();
        }
        format!("/{}", parts[..n].join("/"))
    }

    /// BPM/key/folder aggregates for the heatmap dashboard (same `where_cl` as `audio_filter_stats`).
    fn audio_heatmap_aggregates(
        conn: &Connection,
        where_cl: &str,
        fts_match: &Option<String>,
        regex_pat: &Option<String>,
        like_pat: &Option<String>,
        format_filter: Option<&str>,
    ) -> Result<(Vec<u64>, u64, HashMap<String, u64>, u64, Vec<TopFolderRow>), String> {
        let mut bin_cases = Vec::with_capacity(34);
        for i in 0..34 {
            let lo = 50 + i * 5;
            let hi = lo + 5;
            bin_cases.push(format!(
                "COALESCE(SUM(CASE WHEN bpm IS NOT NULL AND bpm >= {lo} AND bpm < {hi} THEN 1 ELSE 0 END),0)"
            ));
        }
        let sql_bpm = format!(
            "SELECT COALESCE(SUM(CASE WHEN bpm IS NOT NULL THEN 1 ELSE 0 END),0), {} FROM audio_samples WHERE {}",
            bin_cases.join(", "),
            where_cl
        );
        let mut bpm_buckets = vec![0u64; 34];
        let mut bpm_analyzed_count = 0u64;
        let mut stmt = conn.prepare(&sql_bpm).map_err(|e| e.to_string())?;
        let mut bi = 1;
        if let Some(m) = fts_match {
            stmt.raw_bind_parameter(bi, m).map_err(|e| e.to_string())?;
            bi += 1;
        } else if let Some(r) = regex_pat {
            stmt.raw_bind_parameter(bi, r).map_err(|e| e.to_string())?;
            bi += 1;
        } else if let Some(pat) = like_pat {
            stmt.raw_bind_parameter(bi, pat)
                .map_err(|e| e.to_string())?;
            bi += 1;
        }
        if let Some(f) = format_filter
            && !f.is_empty() && f != "all" && !f.contains(',') {
                stmt.raw_bind_parameter(bi, f).map_err(|e| e.to_string())?;
            }
        let mut rows = stmt.raw_query();
        if let Some(row) = rows.next().map_err(|e| e.to_string())? {
            bpm_analyzed_count = row.get::<_, i64>(0).unwrap_or(0).max(0) as u64;
            for (i, bucket) in bpm_buckets.iter_mut().enumerate().take(34) {
                *bucket = row.get::<_, i64>(i + 1).unwrap_or(0).max(0) as u64;
            }
        }

        let sql_key = format!(
            "SELECT key_name, COUNT(*) AS c FROM audio_samples WHERE {} AND key_name IS NOT NULL AND TRIM(key_name) != '' GROUP BY key_name ORDER BY c DESC LIMIT 32",
            where_cl
        );
        let mut key_counts: HashMap<String, u64> = HashMap::new();
        let mut key_analyzed_count = 0u64;
        let mut stmt_k = conn.prepare(&sql_key).map_err(|e| e.to_string())?;
        let mut bi_k = 1;
        if let Some(m) = fts_match {
            stmt_k
                .raw_bind_parameter(bi_k, m)
                .map_err(|e| e.to_string())?;
            bi_k += 1;
        } else if let Some(r) = regex_pat {
            stmt_k
                .raw_bind_parameter(bi_k, r)
                .map_err(|e| e.to_string())?;
            bi_k += 1;
        } else if let Some(pat) = like_pat {
            stmt_k
                .raw_bind_parameter(bi_k, pat)
                .map_err(|e| e.to_string())?;
            bi_k += 1;
        }
        if let Some(f) = format_filter
            && !f.is_empty() && f != "all" && !f.contains(',') {
                stmt_k
                    .raw_bind_parameter(bi_k, f)
                    .map_err(|e| e.to_string())?;
            }
        let mut rows_k = stmt_k.raw_query();
        while let Some(row) = rows_k.next().map_err(|e| e.to_string())? {
            let k: String = row.get(0).unwrap_or_default();
            let c: u64 = row.get::<_, i64>(1).unwrap_or(0).max(0) as u64;
            if !k.is_empty() {
                key_counts.insert(k, c);
                key_analyzed_count += c;
            }
        }

        let sql_dir = format!(
            "SELECT directory FROM audio_samples WHERE {} AND TRIM(directory) != ''",
            where_cl
        );
        let mut folder_acc: HashMap<String, u64> = HashMap::new();
        let mut stmt_d = conn.prepare(&sql_dir).map_err(|e| e.to_string())?;
        let mut bi_d = 1;
        if let Some(m) = fts_match {
            stmt_d
                .raw_bind_parameter(bi_d, m)
                .map_err(|e| e.to_string())?;
            bi_d += 1;
        } else if let Some(r) = regex_pat {
            stmt_d
                .raw_bind_parameter(bi_d, r)
                .map_err(|e| e.to_string())?;
            bi_d += 1;
        } else if let Some(pat) = like_pat {
            stmt_d
                .raw_bind_parameter(bi_d, pat)
                .map_err(|e| e.to_string())?;
            bi_d += 1;
        }
        if let Some(f) = format_filter
            && !f.is_empty() && f != "all" && !f.contains(',') {
                stmt_d
                    .raw_bind_parameter(bi_d, f)
                    .map_err(|e| e.to_string())?;
            }
        let mut rows_d = stmt_d.raw_query();
        while let Some(row) = rows_d.next().map_err(|e| e.to_string())? {
            let dir: String = row.get(0).unwrap_or_default();
            let key = Self::heatmap_audio_folder_key(&dir);
            *folder_acc.entry(key).or_insert(0) += 1;
        }
        let mut top_pairs: Vec<(String, u64)> = folder_acc.into_iter().collect();
        top_pairs.sort_by(|a, b| b.1.cmp(&a.1));
        let top_folders: Vec<TopFolderRow> = top_pairs
            .into_iter()
            .take(12)
            .map(|(path, count)| TopFolderRow { path, count })
            .collect();

        Ok((
            bpm_buckets,
            bpm_analyzed_count,
            key_counts,
            key_analyzed_count,
            top_folders,
        ))
    }

    pub fn audio_filter_stats(
        &self,
        search: Option<&str>,
        format_filter: Option<&str>,
        search_regex: bool,
    ) -> Result<FilterStatsResult, String> {
        let conn = self.read_conn();
        let total_unfiltered: u64 = self.audio_library_total_rows(&conn)?;
        let (fts_match, like_pat, regex_pat) = classify_fts_name_path_search(search, search_regex);
        let mut where_parts = vec![AUDIO_LIBRARY_IDS.to_string()];
        let mut bind_idx = 1usize;
        if fts_match.is_some() {
            where_parts.push(format!(
                "id IN (SELECT rowid FROM audio_samples_fts WHERE audio_samples_fts MATCH ?{bind_idx})",
            ));
            bind_idx += 1;
        } else if regex_pat.is_some() {
            where_parts.push(format!(
                "((name REGEXP ?{bind_idx}) OR (path REGEXP ?{bind_idx}))"
            ));
            bind_idx += 1;
        } else if like_pat.is_some() {
            where_parts.push(format!(
                "(name LIKE ?{bind_idx} ESCAPE '\\' OR path LIKE ?{bind_idx} ESCAPE '\\')"
            ));
            bind_idx += 1;
        }
        if let Some(f) = format_filter
            && !f.is_empty() && f != "all" {
                if f.contains(',') {
                    where_parts.push(format!("format IN ({})", Self::in_list_sql(f)));
                } else {
                    where_parts.push(format!("format = ?{bind_idx}"));
                }
            }
        let where_cl = where_parts.join(" AND ");
        if let Some(m) = fts_match.as_ref() {
            let (bc, capped) = Self::audio_fts_bounded_count_library(&conn, m, format_filter)?;
            if bc == 0 {
                return Ok(FilterStatsResult {
                    count: 0,
                    count_capped: false,
                    total_bytes: 0,
                    by_type: HashMap::new(),
                    bytes_by_type: HashMap::new(),
                    total_unfiltered,
                    size_buckets: vec![0; 6],
                    ..Default::default()
                });
            }
            if capped {
                return Ok(FilterStatsResult {
                    count: bc,
                    count_capped: true,
                    total_bytes: 0,
                    by_type: HashMap::new(),
                    bytes_by_type: HashMap::new(),
                    total_unfiltered,
                    size_buckets: vec![0; 6],
                    ..Default::default()
                });
            }
        }
        let sql = format!(
            "SELECT format, COUNT(*), COALESCE(SUM(size),0) FROM audio_samples WHERE {where_cl} GROUP BY format"
        );
        let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
        let mut bi = 1;
        if let Some(ref m) = fts_match {
            stmt.raw_bind_parameter(bi, m).map_err(|e| e.to_string())?;
            bi += 1;
        } else if let Some(ref r) = regex_pat {
            stmt.raw_bind_parameter(bi, r).map_err(|e| e.to_string())?;
            bi += 1;
        } else if let Some(ref pat) = like_pat {
            stmt.raw_bind_parameter(bi, pat)
                .map_err(|e| e.to_string())?;
            bi += 1;
        }
        if let Some(f) = format_filter
            && !f.is_empty() && f != "all" && !f.contains(',') {
                stmt.raw_bind_parameter(bi, f).map_err(|e| e.to_string())?;
            }
        let _ = bi;
        let mut rows = stmt.raw_query();
        let mut count = 0u64;
        let mut total_bytes = 0u64;
        let mut by_type = HashMap::new();
        let mut bytes_by_type = HashMap::new();
        while let Some(row) = rows.next().map_err(|e| e.to_string())? {
            let fmt: String = row.get(0).unwrap_or_default();
            let n: u64 = row.get::<_, i64>(1).unwrap_or(0) as u64;
            let sz: u64 = row.get::<_, i64>(2).unwrap_or(0) as u64;
            count += n;
            total_bytes += sz;
            by_type.insert(fmt.clone(), n);
            bytes_by_type.insert(fmt, sz);
        }
        // Size histogram for heatmap (same 6 buckets as `heatmap-dashboard.js` — library scope + filters).
        let sql_buckets = format!(
            "SELECT \
             COALESCE(SUM(CASE WHEN COALESCE(size,0) < 102400 THEN 1 ELSE 0 END),0),\
             COALESCE(SUM(CASE WHEN COALESCE(size,0) >= 102400 AND COALESCE(size,0) < 1048576 THEN 1 ELSE 0 END),0),\
             COALESCE(SUM(CASE WHEN COALESCE(size,0) >= 1048576 AND COALESCE(size,0) < 10485760 THEN 1 ELSE 0 END),0),\
             COALESCE(SUM(CASE WHEN COALESCE(size,0) >= 10485760 AND COALESCE(size,0) < 52428800 THEN 1 ELSE 0 END),0),\
             COALESCE(SUM(CASE WHEN COALESCE(size,0) >= 52428800 AND COALESCE(size,0) < 104857600 THEN 1 ELSE 0 END),0),\
             COALESCE(SUM(CASE WHEN COALESCE(size,0) >= 104857600 THEN 1 ELSE 0 END),0)\
             FROM audio_samples WHERE {where_cl}"
        );
        let mut stmt_b = conn.prepare(&sql_buckets).map_err(|e| e.to_string())?;
        let mut bi_b = 1;
        if let Some(ref m) = fts_match {
            stmt_b
                .raw_bind_parameter(bi_b, m)
                .map_err(|e| e.to_string())?;
            bi_b += 1;
        } else if let Some(ref r) = regex_pat {
            stmt_b
                .raw_bind_parameter(bi_b, r)
                .map_err(|e| e.to_string())?;
            bi_b += 1;
        } else if let Some(ref pat) = like_pat {
            stmt_b
                .raw_bind_parameter(bi_b, pat)
                .map_err(|e| e.to_string())?;
            bi_b += 1;
        }
        if let Some(f) = format_filter
            && !f.is_empty() && f != "all" && !f.contains(',') {
                stmt_b
                    .raw_bind_parameter(bi_b, f)
                    .map_err(|e| e.to_string())?;
            }
        let size_buckets: Vec<u64> = {
            let mut rows_b = stmt_b.raw_query();
            if let Some(row) = rows_b.next().map_err(|e| e.to_string())? {
                (0..6)
                    .map(|i| row.get::<_, i64>(i).unwrap_or(0).max(0) as u64)
                    .collect()
            } else {
                vec![0; 6]
            }
        };
        let (bpm_buckets, bpm_analyzed_count, key_counts, key_analyzed_count, top_folders) =
            Self::audio_heatmap_aggregates(
                &conn,
                &where_cl,
                &fts_match,
                &regex_pat,
                &like_pat,
                format_filter,
            )?;
        Ok(FilterStatsResult {
            count,
            count_capped: false,
            total_bytes,
            by_type,
            bytes_by_type,
            total_unfiltered,
            size_buckets,
            bpm_buckets,
            bpm_analyzed_count,
            key_counts,
            key_analyzed_count,
            top_folders,
        })
    }

    pub fn daw_filter_stats(
        &self,
        search: Option<&str>,
        daw_filter: Option<&str>,
        search_regex: bool,
    ) -> Result<FilterStatsResult, String> {
        let conn = self.read_conn();
        let total_unfiltered: u64 = self.daw_library_total_rows(&conn)?;
        let (fts_match, like_pat, regex_pat) = classify_fts_name_path_search(search, search_regex);
        let mut where_parts = vec![DAW_LIBRARY_IDS.to_string()];
        let mut bind_idx = 1usize;
        if fts_match.is_some() {
            // Library scope is already `DAW_LIBRARY_IDS`; do not nest a second
            // `MAX(id) GROUP BY path` inside the FTS subquery (same semantics, worse plan).
            where_parts.push(format!(
                "id IN (SELECT rowid FROM daw_projects_fts WHERE daw_projects_fts MATCH ?{bind_idx})",
            ));
            bind_idx += 1;
        } else if regex_pat.is_some() {
            where_parts.push(format!(
                "((name REGEXP ?{bind_idx}) OR (path REGEXP ?{bind_idx}))"
            ));
            bind_idx += 1;
        } else if like_pat.is_some() {
            where_parts.push(format!(
                "(name LIKE ?{bind_idx} ESCAPE '\\' OR path LIKE ?{bind_idx} ESCAPE '\\')"
            ));
            bind_idx += 1;
        }
        if let Some(f) = daw_filter
            && !f.is_empty() && f != "all" {
                if f.contains(',') {
                    where_parts.push(format!("daw IN ({})", Self::in_list_sql(f)));
                } else {
                    where_parts.push(format!("daw = ?{bind_idx}"));
                }
            }
        let where_cl = where_parts.join(" AND ");
        if let Some(m) = fts_match.as_ref() {
            let (bc, capped) = Self::daw_fts_bounded_count_library(&conn, m, daw_filter)?;
            if bc == 0 {
                return Ok(FilterStatsResult {
                    count: 0,
                    count_capped: false,
                    total_bytes: 0,
                    by_type: HashMap::new(),
                    bytes_by_type: HashMap::new(),
                    total_unfiltered,
                    size_buckets: vec![],
                    ..Default::default()
                });
            }
            if capped {
                return Ok(FilterStatsResult {
                    count: bc,
                    count_capped: true,
                    total_bytes: 0,
                    by_type: HashMap::new(),
                    bytes_by_type: HashMap::new(),
                    total_unfiltered,
                    size_buckets: vec![],
                    ..Default::default()
                });
            }
        }
        let sql = format!(
            "SELECT daw, COUNT(*), COALESCE(SUM(size),0) FROM daw_projects WHERE {where_cl} GROUP BY daw"
        );
        let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
        let mut bi = 1;
        if let Some(ref m) = fts_match {
            stmt.raw_bind_parameter(bi, m).map_err(|e| e.to_string())?;
            bi += 1;
        } else if let Some(ref r) = regex_pat {
            stmt.raw_bind_parameter(bi, r).map_err(|e| e.to_string())?;
            bi += 1;
        } else if let Some(ref pat) = like_pat {
            stmt.raw_bind_parameter(bi, pat)
                .map_err(|e| e.to_string())?;
            bi += 1;
        }
        if let Some(f) = daw_filter
            && !f.is_empty() && f != "all" && !f.contains(',') {
                stmt.raw_bind_parameter(bi, f).map_err(|e| e.to_string())?;
            }
        let _ = bi;
        let mut rows = stmt.raw_query();
        let mut count = 0u64;
        let mut total_bytes = 0u64;
        let mut by_type = HashMap::new();
        let mut bytes_by_type = HashMap::new();
        while let Some(row) = rows.next().map_err(|e| e.to_string())? {
            let daw: String = row.get(0).unwrap_or_default();
            let n: u64 = row.get::<_, i64>(1).unwrap_or(0) as u64;
            let sz: u64 = row.get::<_, i64>(2).unwrap_or(0) as u64;
            count += n;
            total_bytes += sz;
            by_type.insert(daw.clone(), n);
            bytes_by_type.insert(daw, sz);
        }
        Ok(FilterStatsResult {
            count,
            count_capped: false,
            total_bytes,
            by_type,
            bytes_by_type,
            total_unfiltered,
            size_buckets: vec![],
            ..Default::default()
        })
    }

    pub fn preset_filter_stats(
        &self,
        search: Option<&str>,
        format_filter: Option<&str>,
        search_regex: bool,
    ) -> Result<FilterStatsResult, String> {
        let conn = self.read_conn();
        let total_unfiltered: u64 = self.preset_inventory_total_rows(&conn)?;
        let (fts_match, like_pat, regex_pat) = classify_fts_name_path_search(search, search_regex);
        let mut where_parts = vec![
            PRESET_LIBRARY_IDS.to_string(),
            "format NOT IN ('MID','MIDI')".to_string(),
        ];
        let mut bind_idx = 1usize;
        if fts_match.is_some() {
            // Library scope is already `PRESET_LIBRARY_IDS`; do not nest a second
            // `MAX(id) GROUP BY path` inside the FTS subquery (same semantics, worse plan).
            where_parts.push(format!(
                "id IN (SELECT rowid FROM presets_fts WHERE presets_fts MATCH ?{bind_idx})",
            ));
            bind_idx += 1;
        } else if regex_pat.is_some() {
            where_parts.push(format!(
                "((name REGEXP ?{bind_idx}) OR (path REGEXP ?{bind_idx}))"
            ));
            bind_idx += 1;
        } else if like_pat.is_some() {
            where_parts.push(format!(
                "(name LIKE ?{bind_idx} ESCAPE '\\' OR path LIKE ?{bind_idx} ESCAPE '\\')"
            ));
            bind_idx += 1;
        }
        if let Some(f) = format_filter
            && !f.is_empty() && f != "all" {
                if f.contains(',') {
                    where_parts.push(format!("format IN ({})", Self::in_list_sql(f)));
                } else {
                    where_parts.push(format!("format = ?{bind_idx}"));
                }
            }
        let where_cl = where_parts.join(" AND ");
        if let Some(m) = fts_match.as_ref() {
            let (bc, capped) = Self::preset_fts_bounded_count_library(&conn, m, format_filter)?;
            if bc == 0 {
                return Ok(FilterStatsResult {
                    count: 0,
                    count_capped: false,
                    total_bytes: 0,
                    by_type: HashMap::new(),
                    bytes_by_type: HashMap::new(),
                    total_unfiltered,
                    size_buckets: vec![],
                    ..Default::default()
                });
            }
            if capped {
                return Ok(FilterStatsResult {
                    count: bc,
                    count_capped: true,
                    total_bytes: 0,
                    by_type: HashMap::new(),
                    bytes_by_type: HashMap::new(),
                    total_unfiltered,
                    size_buckets: vec![],
                    ..Default::default()
                });
            }
        }
        let sql = format!(
            "SELECT format, COUNT(*), COALESCE(SUM(size),0) FROM presets WHERE {where_cl} GROUP BY format"
        );
        let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
        let mut bi = 1;
        if let Some(ref m) = fts_match {
            stmt.raw_bind_parameter(bi, m).map_err(|e| e.to_string())?;
            bi += 1;
        } else if let Some(ref r) = regex_pat {
            stmt.raw_bind_parameter(bi, r).map_err(|e| e.to_string())?;
            bi += 1;
        } else if let Some(ref pat) = like_pat {
            stmt.raw_bind_parameter(bi, pat)
                .map_err(|e| e.to_string())?;
            bi += 1;
        }
        if let Some(f) = format_filter
            && !f.is_empty() && f != "all" && !f.contains(',') {
                stmt.raw_bind_parameter(bi, f).map_err(|e| e.to_string())?;
            }
        let _ = bi;
        let mut rows = stmt.raw_query();
        let mut count = 0u64;
        let mut total_bytes = 0u64;
        let mut by_type = HashMap::new();
        let mut bytes_by_type = HashMap::new();
        while let Some(row) = rows.next().map_err(|e| e.to_string())? {
            let fmt: String = row.get(0).unwrap_or_default();
            let n: u64 = row.get::<_, i64>(1).unwrap_or(0) as u64;
            let sz: u64 = row.get::<_, i64>(2).unwrap_or(0) as u64;
            count += n;
            total_bytes += sz;
            by_type.insert(fmt.clone(), n);
            bytes_by_type.insert(fmt, sz);
        }
        Ok(FilterStatsResult {
            count,
            count_capped: false,
            total_bytes,
            by_type,
            bytes_by_type,
            total_unfiltered,
            size_buckets: vec![],
            ..Default::default()
        })
    }

    pub fn plugin_filter_stats(
        &self,
        search: Option<&str>,
        type_filter: Option<&str>,
        search_regex: bool,
    ) -> Result<FilterStatsResult, String> {
        let conn = self.read_conn();
        let total_unfiltered: u64 = self.plugin_library_total_rows(&conn)?;
        let (regex_pat, like_pat) = classify_plugins_search(search, search_regex);
        let mut where_parts = vec![PLUGIN_LIBRARY_IDS.to_string()];
        let mut bind_idx = 1usize;
        if regex_pat.is_some() {
            where_parts.push(format!("(name REGEXP ?{bind_idx} OR manufacturer REGEXP ?{bind_idx} OR path REGEXP ?{bind_idx})"));
            bind_idx += 1;
        } else if like_pat.is_some() {
            where_parts.push(format!("(name LIKE ?{bind_idx} ESCAPE '\\' OR manufacturer LIKE ?{bind_idx} ESCAPE '\\' OR path LIKE ?{bind_idx} ESCAPE '\\')"));
            bind_idx += 1;
        }
        if let Some(tf) = type_filter
            && !tf.is_empty() && tf != "all" {
                if tf.contains(',') {
                    where_parts.push(format!("plugin_type IN ({})", Self::in_list_sql(tf)));
                } else {
                    where_parts.push(format!("plugin_type = ?{bind_idx}"));
                }
            }
        let where_cl = where_parts.join(" AND ");
        if regex_pat.is_some() || like_pat.is_some() {
            let (bc, capped) = Self::plugin_search_bounded_count_library(
                &conn,
                "FROM plugins",
                &where_cl,
                &regex_pat,
                &like_pat,
                type_filter,
            )?;
            if bc == 0 {
                return Ok(FilterStatsResult {
                    count: 0,
                    count_capped: false,
                    total_bytes: 0,
                    by_type: HashMap::new(),
                    bytes_by_type: HashMap::new(),
                    total_unfiltered,
                    size_buckets: vec![],
                    ..Default::default()
                });
            }
            if capped {
                return Ok(FilterStatsResult {
                    count: bc,
                    count_capped: true,
                    total_bytes: 0,
                    by_type: HashMap::new(),
                    bytes_by_type: HashMap::new(),
                    total_unfiltered,
                    size_buckets: vec![],
                    ..Default::default()
                });
            }
        }
        let sql = format!(
            "SELECT plugin_type, COUNT(*), COALESCE(SUM(size_bytes),0) FROM plugins WHERE {where_cl} GROUP BY plugin_type"
        );
        let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
        let mut bi = 1;
        if let Some(ref p) = regex_pat.as_ref().or(like_pat.as_ref()) {
            stmt.raw_bind_parameter(bi, p).map_err(|e| e.to_string())?;
            bi += 1;
        }
        if let Some(tf) = type_filter
            && !tf.is_empty() && tf != "all" && !tf.contains(',') {
                stmt.raw_bind_parameter(bi, tf).map_err(|e| e.to_string())?;
            }
        let _ = bi;
        let mut rows = stmt.raw_query();
        let mut count = 0u64;
        let mut total_bytes = 0u64;
        let mut by_type = HashMap::new();
        let mut bytes_by_type = HashMap::new();
        while let Some(row) = rows.next().map_err(|e| e.to_string())? {
            let t: String = row.get(0).unwrap_or_default();
            let n: u64 = row.get::<_, i64>(1).unwrap_or(0) as u64;
            let sz: u64 = row.get::<_, i64>(2).unwrap_or(0) as u64;
            count += n;
            total_bytes += sz;
            by_type.insert(t.clone(), n);
            bytes_by_type.insert(t, sz);
        }
        Ok(FilterStatsResult {
            count,
            count_capped: false,
            total_bytes,
            by_type,
            bytes_by_type,
            total_unfiltered,
            size_buckets: vec![],
            ..Default::default()
        })
    }

    pub fn pdf_filter_stats(
        &self,
        search: Option<&str>,
        search_regex: bool,
    ) -> Result<FilterStatsResult, String> {
        let conn = self.read_conn();
        let total_unfiltered: u64 = self.pdf_library_total_rows(&conn)?;
        let (fts_match, like_pat, regex_pat) = classify_fts_name_path_search(search, search_regex);
        if let Some(m) = fts_match.as_ref() {
            let (bc, capped) = Self::pdf_fts_bounded_count_library(&conn, m)?;
            if bc == 0 {
                return Ok(FilterStatsResult {
                    count: 0,
                    count_capped: false,
                    total_bytes: 0,
                    by_type: HashMap::new(),
                    bytes_by_type: HashMap::new(),
                    total_unfiltered,
                    size_buckets: vec![],
                    ..Default::default()
                });
            }
            if capped {
                return Ok(FilterStatsResult {
                    count: bc,
                    count_capped: true,
                    total_bytes: 0,
                    by_type: HashMap::new(),
                    bytes_by_type: HashMap::new(),
                    total_unfiltered,
                    size_buckets: vec![],
                    ..Default::default()
                });
            }
        }
        let sql = if fts_match.is_some() {
            "SELECT COUNT(*), COALESCE(SUM(size),0) FROM pdfs WHERE id IN (SELECT pdf_id FROM pdf_library) AND id IN (SELECT rowid FROM pdfs_fts WHERE pdfs_fts MATCH ?1)"
        } else if regex_pat.is_some() {
            "SELECT COUNT(*), COALESCE(SUM(size),0) FROM pdfs WHERE id IN (SELECT pdf_id FROM pdf_library) AND ((name REGEXP ?1) OR (path REGEXP ?1))"
        } else if like_pat.is_some() {
            "SELECT COUNT(*), COALESCE(SUM(size),0) FROM pdfs WHERE id IN (SELECT pdf_id FROM pdf_library) AND (name LIKE ?1 ESCAPE '\\' OR path LIKE ?1 ESCAPE '\\')"
        } else {
            "SELECT COUNT(*), COALESCE(SUM(size),0) FROM pdfs WHERE id IN (SELECT pdf_id FROM pdf_library)"
        };
        let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;
        if let Some(ref m) = fts_match {
            stmt.raw_bind_parameter(1, m).map_err(|e| e.to_string())?;
        } else if let Some(ref r) = regex_pat {
            stmt.raw_bind_parameter(1, r).map_err(|e| e.to_string())?;
        } else if let Some(ref pat) = like_pat {
            stmt.raw_bind_parameter(1, pat).map_err(|e| e.to_string())?;
        }
        let mut rows = stmt.raw_query();
        let (count, total_bytes) = if let Some(row) = rows.next().map_err(|e| e.to_string())? {
            (
                row.get::<_, i64>(0).unwrap_or(0) as u64,
                row.get::<_, i64>(1).unwrap_or(0) as u64,
            )
        } else {
            (0, 0)
        };
        Ok(FilterStatsResult {
            count,
            count_capped: false,
            total_bytes,
            by_type: HashMap::new(),
            bytes_by_type: HashMap::new(),
            total_unfiltered,
            size_buckets: vec![],
            ..Default::default()
        })
    }

    // ── KVR cache ──

    pub fn load_kvr_cache(&self) -> Result<HashMap<String, KvrCacheEntry>, String> {
        let conn = self.read_conn();
        let mut stmt = conn.prepare("SELECT plugin_key, kvr_url, update_url, latest_version, has_update, source, timestamp FROM kvr_cache").map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    KvrCacheEntry {
                        kvr_url: row.get(1)?,
                        update_url: row.get(2)?,
                        latest_version: row.get(3)?,
                        has_update: row.get::<_, i32>(4).unwrap_or(0) != 0,
                        source: row.get(5)?,
                        timestamp: row.get(6)?,
                    },
                ))
            })
            .map_err(|e| e.to_string())?;
        let mut map = HashMap::new();
        for (k, v) in rows.flatten() {
            map.insert(k, v);
        }
        Ok(map)
    }

    pub fn update_kvr_cache(
        &self,
        entries: &[crate::history::KvrCacheUpdateEntry],
    ) -> Result<(), String> {
        let conn = self.read_conn();
        let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
        {
            let mut stmt = tx.prepare_cached(
                "INSERT OR REPLACE INTO kvr_cache (plugin_key, kvr_url, update_url, latest_version, has_update, source, timestamp) VALUES (?1, ?2, ?3, ?4, ?5, ?6, datetime('now'))"
            ).map_err(|e| e.to_string())?;
            for e in entries {
                stmt.execute(params![
                    e.key,
                    e.kvr_url,
                    e.update_url,
                    e.latest_version,
                    e.has_update.unwrap_or(false) as i32,
                    e.source.as_deref().unwrap_or("")
                ])
                .map_err(|e| e.to_string())?;
            }
        }
        tx.commit().map_err(|e| e.to_string())
    }

    // ── Generic cache read/write (replaces read_cache_file/write_cache_file) ──

    pub fn read_cache(&self, name: &str) -> Result<serde_json::Value, String> {
        match name {
            "bpm-cache.json" => self.read_analysis_as_cache("bpm"),
            "key-cache.json" => self.read_analysis_as_cache("key"),
            "lufs-cache.json" => self.read_analysis_as_cache("lufs"),
            _ => self.read_kv_cache(name),
        }
    }

    pub fn write_cache(&self, name: &str, data: &serde_json::Value) -> Result<(), String> {
        match name {
            "bpm-cache.json" => self.write_analysis_from_cache(data, "bpm"),
            "key-cache.json" => self.write_analysis_from_cache(data, "key"),
            "lufs-cache.json" => self.write_analysis_from_cache(data, "lufs"),
            _ => self.write_kv_cache(name, data),
        }
    }

    fn read_analysis_as_cache(&self, field: &str) -> Result<serde_json::Value, String> {
        let conn = self.read_conn();
        let col = match field {
            "bpm" => "bpm",
            "key" => "key_name",
            "lufs" => "lufs",
            _ => return Ok(serde_json::json!({})),
        };
        let sql = format!(
            "SELECT path, {col} FROM audio_samples WHERE {col} IS NOT NULL AND ({AUDIO_LIBRARY_IDS})"
        );
        let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
        let mut map = serde_json::Map::new();
        let mut rows = stmt.raw_query();
        while let Some(row) = rows.next().map_err(|e| e.to_string())? {
            let path: String = row.get(0).unwrap_or_default();
            let val: serde_json::Value = if field == "key" {
                serde_json::Value::String(row.get::<_, String>(1).unwrap_or_default())
            } else {
                serde_json::json!(row.get::<_, f64>(1).unwrap_or(0.0))
            };
            map.insert(path, val);
        }
        Ok(serde_json::Value::Object(map))
    }

    fn write_analysis_from_cache(
        &self,
        data: &serde_json::Value,
        field: &str,
    ) -> Result<(), String> {
        let obj = data.as_object().ok_or("expected object")?;
        if obj.is_empty() {
            return Ok(());
        }
        let conn = self.write_conn();
        let col = match field {
            "bpm" => "bpm",
            "key" => "key_name",
            "lufs" => "lufs",
            _ => return Ok(()),
        };
        let sql = if field == "bpm" {
            "UPDATE audio_samples SET bpm = ?1, bpm_exhausted = 0 WHERE path = ?2".to_string()
        } else {
            format!("UPDATE audio_samples SET {col} = ?1 WHERE path = ?2")
        };
        let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
        {
            let mut stmt = tx.prepare_cached(&sql).map_err(|e| e.to_string())?;
            for (path, val) in obj {
                let path = normalize_path_for_db(path);
                if field == "key" {
                    if let Some(s) = val.as_str() {
                        let _ = stmt.execute(params![s, path]);
                    }
                } else {
                    if let Some(v) = val.as_f64() {
                        let _ = stmt.execute(params![v, path]);
                    }
                }
            }
        }
        tx.commit().map_err(|e| e.to_string())
    }

    fn read_kv_cache(&self, name: &str) -> Result<serde_json::Value, String> {
        let (table, key_col, val_col) = self.cache_table_for(name);
        let conn = self.read_conn();
        let sql = format!("SELECT {key_col}, {val_col} FROM {table}");
        let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
        let mut map = serde_json::Map::new();
        let mut rows = stmt.raw_query();
        while let Some(row) = rows.next().map_err(|e| e.to_string())? {
            let k: String = row.get(0).unwrap_or_default();
            let v: String = row.get(1).unwrap_or_default();
            // Try to parse as JSON, fall back to string
            let val = serde_json::from_str(&v).unwrap_or(serde_json::Value::String(v));
            map.insert(k, val);
        }
        Ok(serde_json::Value::Object(map))
    }

    fn write_kv_cache(&self, name: &str, data: &serde_json::Value) -> Result<(), String> {
        let obj = data.as_object().ok_or("expected object")?;
        let (table, key_col, val_col) = self.cache_table_for(name);
        let conn = self.write_conn();
        let sql = format!("INSERT OR REPLACE INTO {table} ({key_col}, {val_col}) VALUES (?1, ?2)");
        let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
        {
            let mut stmt = tx.prepare_cached(&sql).map_err(|e| e.to_string())?;
            for (k, v) in obj {
                let k = normalize_path_for_db(k);
                let val_str = if v.is_string() {
                    v.as_str().unwrap_or("").to_string()
                } else {
                    v.to_string()
                };
                let _ = stmt.execute(params![k, val_str]);
            }
        }
        tx.commit().map_err(|e| e.to_string())
    }

    /// Single-row upsert so decoded waveform / spectrogram data hits SQLite immediately (not only on debounced full `write_cache_file`).
    pub fn upsert_waveform_cache_row(&self, path: &str, data: &serde_json::Value) -> Result<(), String> {
        let conn = self.write_conn();
        let k = normalize_path_for_db(path);
        let val_str = data.to_string();
        conn.execute(
            "INSERT OR REPLACE INTO waveform_cache (path, data) VALUES (?1, ?2)",
            params![k, val_str],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// One path lookup — avoids bulk `read_cache("waveform-cache.json")` on startup (large DBs).
    pub fn get_waveform_cache_row(&self, path: &str) -> Result<Option<serde_json::Value>, String> {
        let conn = self.read_conn();
        let k = normalize_path_for_db(path);
        let mut stmt = conn
            .prepare("SELECT data FROM waveform_cache WHERE path = ?1 LIMIT 1")
            .map_err(|e| e.to_string())?;
        let mut rows = stmt.query(params![k]).map_err(|e| e.to_string())?;
        if let Some(row) = rows.next().map_err(|e| e.to_string())? {
            let s: String = row.get(0).map_err(|e| e.to_string())?;
            let val: serde_json::Value = serde_json::from_str(&s).map_err(|e| e.to_string())?;
            Ok(Some(val))
        } else {
            Ok(None)
        }
    }

    pub fn upsert_spectrogram_cache_row(&self, path: &str, data: &serde_json::Value) -> Result<(), String> {
        let conn = self.write_conn();
        let k = normalize_path_for_db(path);
        let val_str = data.to_string();
        conn.execute(
            "INSERT OR REPLACE INTO spectrogram_cache (path, data) VALUES (?1, ?2)",
            params![k, val_str],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Get row counts for all tables.
    pub fn table_counts(&self) -> Result<serde_json::Value, String> {
        let conn = self.read_conn();
        let tables = [
            "audio_samples",
            "audio_scans",
            "plugins",
            "plugin_library",
            "plugin_scans",
            "daw_projects",
            "daw_library",
            "daw_scans",
            "presets",
            "preset_scans",
            "pdfs",
            "pdf_library",
            "midi_files",
            "midi_library",
            "video_files",
            "video_library",
            "video_scans",
            "pdf_scans",
            "pdf_metadata",
            "preset_library",
            "kvr_cache",
            "waveform_cache",
            "spectrogram_cache",
            "xref_cache",
            "fingerprint_cache",
        ];
        let mut map = serde_json::Map::new();
        for t in &tables {
            let count: u64 = conn
                .query_row(&format!("SELECT COUNT(*) FROM {t}"), [], |r| {
                    r.get::<_, i64>(0).map(|v| v as u64)
                })
                .unwrap_or(0);
            map.insert(t.to_string(), serde_json::json!(count));
        }

        // Library counts: one canonical row per `path` (matches Samples tab / `query_audio`).
        // Raw `audio_samples` rows can exceed this when the same path appears in multiple scans.
        let audio_lib: u64 = self.audio_library_total_rows(&conn)?;
        let plugins_lib: u64 = self.plugin_library_total_rows(&conn)?;
        let daw_lib: u64 = self.daw_library_total_rows(&conn)?;
        let presets_lib: u64 = conn
            .query_row(
                "SELECT COUNT(*) FROM presets WHERE id IN (SELECT preset_id FROM preset_library) AND format NOT IN ('MID','MIDI')",
                [],
                |r| r.get::<_, i64>(0).map(|v| v as u64),
            )
            .unwrap_or(0);
        let pdfs_lib: u64 = conn
            .query_row(
                "SELECT COUNT(*) FROM pdfs WHERE id IN (SELECT pdf_id FROM pdf_library)",
                [],
                |r| r.get::<_, i64>(0).map(|v| v as u64),
            )
            .unwrap_or(0);
        let midi_lib: u64 = conn
            .query_row(
                "SELECT COUNT(*) FROM midi_files WHERE id IN (SELECT midi_id FROM midi_library)",
                [],
                |r| r.get::<_, i64>(0).map(|v| v as u64),
            )
            .unwrap_or(0);
        let video_lib: u64 = conn
            .query_row(
                "SELECT COUNT(*) FROM video_files WHERE id IN (SELECT video_id FROM video_library)",
                [],
                |r| r.get::<_, i64>(0).map(|v| v as u64),
            )
            .unwrap_or(0);

        map.insert("audio_samples_library".into(), serde_json::json!(audio_lib));
        map.insert("plugins_library".into(), serde_json::json!(plugins_lib));
        map.insert("daw_projects_library".into(), serde_json::json!(daw_lib));
        map.insert("presets_library".into(), serde_json::json!(presets_lib));
        map.insert("pdfs_library".into(), serde_json::json!(pdfs_lib));
        map.insert("midi_files_library".into(), serde_json::json!(midi_lib));
        map.insert("video_files_library".into(), serde_json::json!(video_lib));

        Ok(serde_json::Value::Object(map))
    }

    /// Row counts per inventory category for the **library** view: one canonical row per `path`.
    /// Audio uses `audio_library` (v14); DAW uses `daw_library` (v16); PDF, MIDI, and presets use
    /// `pdf_library`, `midi_library`, and `preset_library` (v15); plugins use `plugin_library` (v17).
    /// Not scoped to a single `scan_id`. Presets exclude `MID`/`MIDI` (same tab rules as elsewhere).
    /// Matches default `scan_id` handling on paginated queries and `*_filter_stats`, not raw
    /// `COUNT(*)` on whole tables.
    pub fn active_scan_inventory_counts(&self) -> Result<serde_json::Value, String> {
        let conn = self.read_conn();
        let count_plugins: u64 = self.plugin_library_total_rows(&conn)?;
        let count_audio: u64 = self.audio_library_total_rows(&conn)?;
        let count_daw: u64 = self.daw_library_total_rows(&conn)?;
        let count_presets: u64 = conn
            .query_row(
                "SELECT COUNT(*) FROM presets WHERE id IN (SELECT preset_id FROM preset_library) AND format NOT IN ('MID','MIDI')",
                [],
                |r| r.get::<_, i64>(0).map(|v| v as u64),
            )
            .unwrap_or(0);
        let count_pdfs: u64 = conn
            .query_row(
                "SELECT COUNT(*) FROM pdfs WHERE id IN (SELECT pdf_id FROM pdf_library)",
                [],
                |r| r.get::<_, i64>(0).map(|v| v as u64),
            )
            .unwrap_or(0);
        let count_midi: u64 = conn
            .query_row(
                "SELECT COUNT(*) FROM midi_files WHERE id IN (SELECT midi_id FROM midi_library)",
                [],
                |r| r.get::<_, i64>(0).map(|v| v as u64),
            )
            .unwrap_or(0);
        let count_video: u64 = conn
            .query_row(
                "SELECT COUNT(*) FROM video_files WHERE id IN (SELECT video_id FROM video_library)",
                [],
                |r| r.get::<_, i64>(0).map(|v| v as u64),
            )
            .unwrap_or(0);

        Ok(serde_json::json!({
            "plugins": count_plugins,
            "audio_samples": count_audio,
            "daw_projects": count_daw,
            "presets": count_presets,
            "pdfs": count_pdfs,
            "midi_files": count_midi,
            "video_files": count_video,
        }))
    }

    /// All library paths with byte sizes for content-hash duplicate detection.
    /// Uses the same per-domain library rules as [`Database::active_scan_inventory_counts`].
    pub fn library_paths_for_content_hash(&self) -> Result<Vec<(String, u64, String)>, String> {
        let conn = self.read_conn();
        let mut out: Vec<(String, u64, String)> = Vec::new();

        let mut push_sql = |sql: &str, kind: &str| -> Result<(), String> {
            let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map([], |r| {
                    let path: String = r.get(0)?;
                    let sz: i64 = r.get(1)?;
                    Ok((path, sz.max(0) as u64, kind.to_string()))
                })
                .map_err(|e| e.to_string())?;
            for row in rows {
                out.push(row.map_err(|e| e.to_string())?);
            }
            Ok(())
        };

        push_sql(
            &format!("SELECT path, size_bytes FROM plugins WHERE {PLUGIN_LIBRARY_IDS}"),
            "plugins",
        )?;
        push_sql(
            &format!("SELECT path, size FROM audio_samples WHERE {AUDIO_LIBRARY_IDS}"),
            "audio",
        )?;
        push_sql(
            &format!("SELECT path, size FROM daw_projects WHERE {DAW_LIBRARY_IDS}"),
            "daw",
        )?;
        push_sql(
            "SELECT path, size FROM presets WHERE id IN (SELECT preset_id FROM preset_library) AND format NOT IN ('MID','MIDI')",
            "presets",
        )?;
        push_sql(
            "SELECT path, size FROM pdfs WHERE id IN (SELECT pdf_id FROM pdf_library)",
            "pdf",
        )?;
        push_sql(
            "SELECT path, size FROM midi_files WHERE id IN (SELECT midi_id FROM midi_library)",
            "midi",
        )?;
        push_sql(
            "SELECT path, size FROM video_files WHERE id IN (SELECT video_id FROM video_library)",
            "video",
        )?;

        Ok(out)
    }

    /// Get stats for all caches: item count and estimated size.
    ///
    /// Uses the primary **write** connection (not the read pool). Read-pool handles install a
    /// [`SQLITE_QUERY_TIMEOUT_SECS`] progress-handler budget per acquisition; `cache_stats` runs
    /// many sequential full-table `COUNT`s and `dbstat` passes — cumulative time exceeded 30s on
    /// large libraries and surfaced as an empty/error cache table in Settings.
    pub fn cache_stats(&self) -> Result<Vec<CacheStat>, String> {
        let conn = self.write_conn();
        let mut stats = Vec::new();

        // Analysis caches (columns on audio_samples — library rows only).
        // `total` stays 0 so Settings → Database Caches shows a plain row count (like KV caches),
        // not "cached / entire library", which misread the Items column as cache capacity.
        // Size: Key uses UTF-8 payload length; BPM/LUFS use an 8-byte float payload estimate.
        for (label, col, key) in [
            ("BPM", "bpm", "bpm"),
            ("Key", "key_name", "key"),
            ("LUFS", "lufs", "lufs"),
        ] {
            let count: u64 = conn.query_row(
                &format!("SELECT COUNT(*) FROM audio_samples WHERE {col} IS NOT NULL AND ({AUDIO_LIBRARY_IDS})"),
                [],
                |r| r.get::<_, i64>(0).map(|v| v as u64),
            )
            .unwrap_or(0);
            let size_bytes: u64 = if key == "key" {
                conn.query_row(
                    &format!(
                        "SELECT COALESCE(SUM(LENGTH(key_name)), 0) FROM audio_samples WHERE key_name IS NOT NULL AND ({AUDIO_LIBRARY_IDS})"
                    ),
                    [],
                    |r| r.get::<_, i64>(0).map(|v| v as u64),
                )
                .unwrap_or(0)
            } else {
                count.saturating_mul(8)
            };
            stats.push(CacheStat {
                key: key.into(),
                label: label.into(),
                count,
                total: 0,
                size_bytes,
            });
        }

        // KV caches — count rows and estimate size from data length
        for (label, table, _key_col, val_col, key) in [
            ("Waveform", "waveform_cache", "path", "data", "waveform"),
            (
                "Spectrogram",
                "spectrogram_cache",
                "path",
                "data",
                "spectrogram",
            ),
            ("Xref", "xref_cache", "project_path", "plugins_json", "xref"),
            (
                "Fingerprint",
                "fingerprint_cache",
                "path",
                "fingerprint",
                "fingerprint",
            ),
            ("KVR", "kvr_cache", "plugin_key", "kvr_url", "kvr"),
        ] {
            let (count, size): (u64, u64) = conn
                .query_row(
                    &format!("SELECT COUNT(*), COALESCE(SUM(LENGTH({val_col})), 0) FROM {table}"),
                    [],
                    |r| Ok((r.get::<_, i64>(0)? as u64, r.get::<_, i64>(1)? as u64)),
                )
                .unwrap_or((0, 0));
            stats.push(CacheStat {
                key: key.into(),
                label: label.into(),
                count,
                total: 0,
                size_bytes: size,
            });
        }

        // Scan histories — per-group on-disk size via `dbstat` (table + indexes), not
        // `(whole DB pages) / row_count` (that made every category ≈ full file size).
        let db_path = history::get_data_dir().join("audio_haxor.db");
        let db_size = std::fs::metadata(&db_path).map(|m| m.len()).unwrap_or(0);
        let total_inv_rows: u64 = [
            "plugins",
            "audio_samples",
            "daw_projects",
            "presets",
            "midi_files",
            "video_files",
            "pdfs",
        ]
        .iter()
        .map(|t| {
            conn.query_row(&format!("SELECT COUNT(*) FROM {t}"), [], |r| {
                r.get::<_, i64>(0).map(|v| v as u64)
            })
            .unwrap_or(0)
        })
        .sum();

        for (label, scan_table, item_table, key) in [
            ("Plugin Scans", "plugin_scans", "plugins", "plugin_scans"),
            ("Audio Scans", "audio_scans", "audio_samples", "audio_scans"),
            ("DAW Scans", "daw_scans", "daw_projects", "daw_scans"),
            ("Preset Scans", "preset_scans", "presets", "preset_scans"),
            ("MIDI Scans", "midi_scans", "midi_files", "midi_scans"),
            ("Video Scans", "video_scans", "video_files", "video_scans"),
            ("PDF Scans", "pdf_scans", "pdfs", "pdf_scans"),
        ] {
            let scan_count: u64 = conn
                .query_row(&format!("SELECT COUNT(*) FROM {scan_table}"), [], |r| {
                    r.get::<_, i64>(0).map(|v| v as u64)
                })
                .unwrap_or(0);
            let item_count: u64 = conn
                .query_row(&format!("SELECT COUNT(*) FROM {item_table}"), [], |r| {
                    r.get::<_, i64>(0).map(|v| v as u64)
                })
                .unwrap_or(0);
            let size_bytes =
                if let Some(b) = dbstat_bytes_for_scan_group(&conn, scan_table, item_table) {
                    b
                } else if total_inv_rows > 0 {
                    db_size.saturating_mul(item_count) / total_inv_rows.max(1)
                } else {
                    0
                };
            stats.push(CacheStat {
                key: key.into(),
                label: label.into(),
                count: item_count,
                total: scan_count,
                size_bytes,
            });
        }

        // Total DB file size
        stats.push(CacheStat {
            key: "database".into(),
            label: "Total Database".into(),
            count: 0,
            total: 0,
            size_bytes: db_size,
        });

        Ok(stats)
    }

    /// Batch update BPM/Key/LUFS for multiple files in a single transaction.
    ///
    /// Returns the **sum of `sqlite3_changes()`** for each `UPDATE` (rows modified), not the
    /// number of input paths — a path matching multiple `audio_samples` rows updates all of them.
    pub fn batch_update_analysis(&self, results: &[AnalysisBatchRow]) -> Result<u32, String> {
        let conn = self.read_conn();
        let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
        let mut rows_changed: u32 = 0;
        {
            let mut stmt = tx
                .prepare_cached(
                    "UPDATE audio_samples SET bpm = ?1, key_name = ?2, lufs = ?3, bpm_exhausted = ?4 WHERE path = ?5",
                )
                .map_err(|e| e.to_string())?;
            for (path, bpm, key, lufs) in results {
                let path = normalize_path_for_db(path);
                let exhausted: i32 = if bpm.is_none() && key.is_some() && lufs.is_some() {
                    1
                } else {
                    0
                };
                let n = stmt
                    .execute(params![bpm, key, lufs, exhausted, path])
                    .map_err(|e| e.to_string())?;
                rows_changed = rows_changed.saturating_add(n as u32);
            }
        }
        tx.commit().map_err(|e| e.to_string())?;
        Ok(rows_changed)
    }

    // ── ALS Generator: sample analysis ──

    /// Seed `sample_pack_manufacturers` from the hardcoded MANUFACTURER_SIGNALS list.
    /// Uses INSERT OR IGNORE so it is safe to call multiple times.
    pub fn seed_sample_manufacturers(&self) -> Result<u32, String> {
        let conn = self.read_conn();
        let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
        let mut count: u32 = 0;
        {
            let mut stmt = tx
                .prepare_cached(
                    "INSERT OR IGNORE INTO sample_pack_manufacturers (name, genre_score, hardness_score)
                     VALUES (?1, ?2, ?3)",
                )
                .map_err(|e| e.to_string())?;
            for &(name, genre, hardness) in crate::sample_analysis::MANUFACTURER_SIGNALS {
                let n = stmt
                    .execute(rusqlite::params![name, genre, hardness])
                    .map_err(|e| e.to_string())?;
                count = count.saturating_add(n as u32);
            }
        }
        tx.commit().map_err(|e| e.to_string())?;
        Ok(count)
    }

    /// Seed `sample_categories` from the hardcoded CATEGORY_PATTERNS list.
    /// Uses INSERT OR IGNORE so it is safe to call multiple times.
    pub fn seed_sample_categories(&self) -> Result<u32, String> {
        let conn = self.read_conn();
        let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
        let mut count: u32 = 0;
        {
            let mut stmt = tx
                .prepare_cached(
                    "INSERT OR IGNORE INTO sample_categories
                         (name, parent_name, is_oneshot, is_key_sensitive, is_loop_preferred, pattern)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                )
                .map_err(|e| e.to_string())?;
            for &(name, pattern, parent, oneshot, key_sens, loop_pref) in
                crate::sample_analysis::CATEGORY_PATTERNS
            {
                let n = stmt
                    .execute(rusqlite::params![
                        name,
                        parent,
                        oneshot as i32,
                        key_sens as i32,
                        loop_pref as i32,
                        pattern,
                    ])
                    .map_err(|e| e.to_string())?;
                count = count.saturating_add(n as u32);
            }
        }
        tx.commit().map_err(|e| e.to_string())?;
        Ok(count)
    }

    /// Get sample IDs + names + directories + paths + existing key that have not been analyzed yet.
    /// Only returns WAV samples from the canonical audio_library.
    /// Returns: (id, name, directory, path, key_name)
    pub fn unanalyzed_sample_ids(
        &self,
        limit: u64,
    ) -> Result<Vec<(i64, String, String, String, Option<String>)>, String> {
        let conn = self.read_conn();
        let mut stmt = conn
            .prepare(&format!(
                "SELECT s.id, s.name, s.directory, s.path, s.key_name
                 FROM audio_samples s
                 LEFT JOIN sample_analysis a ON s.id = a.sample_id
                 WHERE a.sample_id IS NULL
                   AND s.format = 'WAV'
                   AND ({AUDIO_LIBRARY_IDS})
                 LIMIT ?1"
            ))
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(rusqlite::params![limit as i64], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?))
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    }

    /// Count of WAV samples in audio_library that have not been analyzed.
    pub fn unanalyzed_sample_count(&self) -> Result<u64, String> {
        let conn = self.read_conn();
        let n: i64 = conn
            .query_row(
                &format!(
                    "SELECT COUNT(*)
                     FROM audio_samples s
                     LEFT JOIN sample_analysis a ON s.id = a.sample_id
                     WHERE a.sample_id IS NULL
                       AND s.format = 'WAV'
                       AND ({AUDIO_LIBRARY_IDS})"
                ),
                [],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;
        Ok(n as u64)
    }

    /// Count of WAV samples already analyzed.
    pub fn analyzed_sample_count(&self) -> Result<u64, String> {
        let conn = self.read_conn();
        let n: i64 = conn
            .query_row("SELECT COUNT(*) FROM sample_analysis", [], |row| row.get(0))
            .map_err(|e| e.to_string())?;
        Ok(n as u64)
    }

    /// Batch insert analysis results. Each row is:
    /// (sample_id, parsed_bpm, parsed_key, category_name, manufacturer_pattern, category_confidence, is_loop)
    pub fn batch_insert_sample_analysis(
        &self,
        rows: &[(i64, Option<u32>, Option<String>, Option<String>, Option<String>, f32, bool)],
    ) -> Result<u32, String> {
        let conn = self.read_conn();
        let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
        let mut count: u32 = 0;
        {
            let mut stmt = tx
                .prepare_cached(
                    "INSERT OR REPLACE INTO sample_analysis
                         (sample_id, parsed_bpm, parsed_key, category_id, manufacturer_id, category_confidence, is_loop)
                     VALUES (
                         ?1, ?2, ?3,
                         (SELECT id FROM sample_categories WHERE name = ?4),
                         (SELECT id FROM sample_pack_manufacturers WHERE name = ?5),
                         ?6, ?7
                     )",
                )
                .map_err(|e| e.to_string())?;
            for (sample_id, bpm, key, cat_name, mfr_pattern, confidence, is_loop) in rows {
                let n = stmt
                    .execute(rusqlite::params![
                        sample_id,
                        bpm,
                        key,
                        cat_name,
                        mfr_pattern,
                        confidence,
                        *is_loop as i32,
                    ])
                    .map_err(|e| e.to_string())?;
                count = count.saturating_add(n as u32);
            }
        }
        tx.commit().map_err(|e| e.to_string())?;
        Ok(count)
    }

    /// Execute a pre-built query for ALS sample selection.
    /// Returns `SelectedSample` rows matching the query.
    pub fn query_samples_for_als(
        &self,
        query: &str,
    ) -> Result<Vec<crate::als_project::SelectedSample>, String> {
        let conn = self.read_conn();
        let mut stmt = conn.prepare(query).map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| {
                Ok(crate::als_project::SelectedSample {
                    sample_id: row.get(0)?,
                    path: row.get(1)?,
                    name: row.get(2)?,
                    duration: row.get(3)?,
                    size: row.get(4)?,
                    parsed_bpm: row.get::<_, Option<i64>>(5)?.map(|v| v as u32),
                    parsed_key: row.get(6)?,
                    category: row.get(7)?,
                    is_loop: row.get::<_, i64>(8).unwrap_or(0) != 0,
                })
            })
            .map_err(|e| e.to_string())?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    /// Clear a specific cache table.
    pub fn clear_cache_table(&self, table: &str) -> Result<(), String> {
        let conn = self.read_conn();
        let sql = match table {
            "bpm" => "UPDATE audio_samples SET bpm = NULL, bpm_exhausted = 0",
            "key" => "UPDATE audio_samples SET key_name = NULL",
            "lufs" => "UPDATE audio_samples SET lufs = NULL",
            "waveform" => "DELETE FROM waveform_cache",
            "spectrogram" => "DELETE FROM spectrogram_cache",
            "xref" => "DELETE FROM xref_cache",
            "fingerprint" => "DELETE FROM fingerprint_cache",
            "kvr" => "DELETE FROM kvr_cache",
            _ => return Err(format!("Unknown cache: {table}")),
        };
        conn.execute_batch(sql).map_err(|e| e.to_string())
    }

    /// Clear all analysis and cache data from SQLite.
    pub fn clear_all_caches(&self) -> Result<(), String> {
        let conn = self.read_conn();
        conn.execute_batch(
            "UPDATE audio_samples SET bpm = NULL, key_name = NULL, lufs = NULL, bpm_exhausted = 0;
             DELETE FROM waveform_cache;
             DELETE FROM spectrogram_cache;
             DELETE FROM xref_cache;
             DELETE FROM fingerprint_cache;
             DELETE FROM kvr_cache;",
        )
        .map_err(|e| e.to_string())
    }

    fn cache_table_for(&self, name: &str) -> (&str, &str, &str) {
        match name {
            "waveform-cache.json" => ("waveform_cache", "path", "data"),
            "spectrogram-cache.json" => ("spectrogram_cache", "path", "data"),
            "xref-cache.json" => ("xref_cache", "project_path", "plugins_json"),
            "fingerprint-cache.json" => ("fingerprint_cache", "path", "fingerprint"),
            _ => ("waveform_cache", "path", "data"), // fallback
        }
    }

    /// One-time migration of ALL JSON history/cache files to SQLite.
    pub fn migrate_from_json(&self) -> Result<usize, String> {
        let data_dir = history::get_data_dir();
        let mut total = 0;

        // Check if already migrated (any scan table has data)
        {
            let conn = self.read_conn();
            let count: u64 = conn
                .query_row(
                    "SELECT (SELECT COUNT(*) FROM audio_scans) +
                            (SELECT COUNT(*) FROM plugin_scans) +
                            (SELECT COUNT(*) FROM daw_scans) +
                            (SELECT COUNT(*) FROM preset_scans)",
                    [],
                    |row| row.get::<_, i64>(0).map(|v| v as u64),
                )
                .unwrap_or(0);
            if count > 0 {
                return Ok(0);
            }
        }

        // ── Audio samples ──
        total += self.migrate_audio_json(&data_dir)?;

        // ── Plugin scans ──
        total += self.migrate_plugin_json(&data_dir)?;

        // ── DAW projects ──
        total += self.migrate_daw_json(&data_dir)?;

        // ── Presets ──
        total += self.migrate_preset_json(&data_dir)?;

        // ── KVR cache ──
        total += self.migrate_kvr_json(&data_dir)?;

        // ── Frontend caches (xref, waveform, spectrogram, fingerprint) ──
        total += self.migrate_kv_cache(
            &data_dir,
            "xref-cache.json",
            "xref_cache",
            "project_path",
            "plugins_json",
        )?;
        total += self.migrate_kv_cache(
            &data_dir,
            "waveform-cache.json",
            "waveform_cache",
            "path",
            "data",
        )?;
        total += self.migrate_kv_cache(
            &data_dir,
            "spectrogram-cache.json",
            "spectrogram_cache",
            "path",
            "data",
        )?;
        total += self.migrate_kv_cache(
            &data_dir,
            "fingerprint-cache.json",
            "fingerprint_cache",
            "path",
            "fingerprint",
        )?;

        // Rename all migrated JSON files to .bak
        for name in &[
            "audio-scan-history.json",
            "bpm-cache.json",
            "key-cache.json",
            "lufs-cache.json",
            "scan-history.json",
            "daw-scan-history.json",
            "preset-scan-history.json",
            "kvr-cache.json",
            "xref-cache.json",
            "waveform-cache.json",
            "spectrogram-cache.json",
            "fingerprint-cache.json",
        ] {
            let p = data_dir.join(name);
            if p.exists() {
                let _ = std::fs::rename(&p, data_dir.join(format!("{name}.bak")));
            }
        }

        Ok(total)
    }

    fn migrate_audio_json(&self, data_dir: &std::path::Path) -> Result<usize, String> {
        let path = data_dir.join("audio-scan-history.json");
        if !path.exists() {
            return Ok(0);
        }
        let data = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
        let history: AudioHistory =
            serde_json::from_str(&data).map_err(|e| format!("audio JSON: {e}"))?;
        let mut count = 0;
        for snap in &history.scans {
            self.save_scan(
                &snap.id,
                &snap.timestamp,
                snap.sample_count as u64,
                snap.total_bytes,
                &snap.format_counts,
                &snap.roots,
            )?;
            self.insert_audio_batch(&snap.id, &snap.samples)?;
            count += snap.samples.len();
        }
        self.migrate_analysis_cache(data_dir, "bpm-cache.json", "bpm")?;
        self.migrate_analysis_cache(data_dir, "key-cache.json", "key")?;
        self.migrate_analysis_cache(data_dir, "lufs-cache.json", "lufs")?;
        Ok(count)
    }

    fn migrate_plugin_json(&self, data_dir: &std::path::Path) -> Result<usize, String> {
        let path = data_dir.join("scan-history.json");
        if !path.exists() {
            return Ok(0);
        }
        let data = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
        let history: ScanHistory =
            serde_json::from_str(&data).map_err(|e| format!("plugin JSON: {e}"))?;
        let conn = self.read_conn();
        let mut count = 0;
        for snap in &history.scans {
            let dirs_json = path_strings_json_normalized(&snap.directories);
            let roots_json = path_strings_json_normalized(&snap.roots);
            conn.execute(
                "INSERT OR REPLACE INTO plugin_scans (id, timestamp, plugin_count, directories, roots, scan_complete) VALUES (?1,?2,?3,?4,?5,1)",
                params![snap.id, snap.timestamp, snap.plugin_count as i64, dirs_json, roots_json],
            ).map_err(|e| e.to_string())?;

            let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
            {
                let mut stmt = tx.prepare_cached(
                    "INSERT OR REPLACE INTO plugins (name, path, plugin_type, version, manufacturer, manufacturer_url, size, size_bytes, modified, architectures, scan_id) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)"
                ).map_err(|e| e.to_string())?;
                for p in &snap.plugins {
                    let arch_json = serde_json::to_string(&p.architectures).unwrap_or_default();
                    let path = normalize_path_for_db(&p.path);
                    stmt.execute(params![
                        p.name,
                        path,
                        p.plugin_type,
                        p.version,
                        p.manufacturer,
                        p.manufacturer_url,
                        p.size,
                        p.size_bytes as i64,
                        p.modified,
                        arch_json,
                        snap.id
                    ])
                    .map_err(|e| e.to_string())?;
                }
            }
            tx.commit().map_err(|e| e.to_string())?;
            count += snap.plugins.len();
        }
        Self::rebuild_plugin_library(&conn)?;
        self.invalidate_plugin_library_total_cache();
        Ok(count)
    }

    fn migrate_daw_json(&self, data_dir: &std::path::Path) -> Result<usize, String> {
        let path = data_dir.join("daw-scan-history.json");
        if !path.exists() {
            return Ok(0);
        }
        let data = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
        let history: DawHistory =
            serde_json::from_str(&data).map_err(|e| format!("daw JSON: {e}"))?;
        let conn = self.read_conn();
        let mut count = 0;
        for snap in &history.scans {
            let daw_json = serde_json::to_string(&snap.daw_counts).unwrap_or_default();
            let roots_json = path_strings_json_normalized(&snap.roots);
            conn.execute(
                "INSERT OR REPLACE INTO daw_scans (id, timestamp, project_count, total_bytes, daw_counts, roots, scan_complete) VALUES (?1,?2,?3,?4,?5,?6,1)",
                params![snap.id, snap.timestamp, snap.project_count as i64, snap.total_bytes as i64, daw_json, roots_json],
            ).map_err(|e| e.to_string())?;

            let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
            {
                let mut stmt = tx.prepare_cached(
                    "INSERT OR REPLACE INTO daw_projects (name, path, directory, format, daw, size, size_formatted, modified, scan_id) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)"
                ).map_err(|e| e.to_string())?;
                for p in &snap.projects {
                    let path = normalize_path_for_db(&p.path);
                    let directory = normalize_path_for_db(&p.directory);
                    stmt.execute(params![
                        p.name,
                        path,
                        directory,
                        p.format,
                        p.daw,
                        p.size as i64,
                        p.size_formatted,
                        p.modified,
                        snap.id
                    ])
                    .map_err(|e| e.to_string())?;
                }
            }
            tx.commit().map_err(|e| e.to_string())?;
            count += snap.projects.len();
        }
        Self::rebuild_daw_library(&conn)?;
        self.invalidate_daw_library_total_cache();
        Ok(count)
    }

    fn migrate_preset_json(&self, data_dir: &std::path::Path) -> Result<usize, String> {
        let path = data_dir.join("preset-scan-history.json");
        if !path.exists() {
            return Ok(0);
        }
        let data = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
        let history: PresetHistory =
            serde_json::from_str(&data).map_err(|e| format!("preset JSON: {e}"))?;
        let conn = self.read_conn();
        let mut count = 0;
        for snap in &history.scans {
            let fc_json = serde_json::to_string(&snap.format_counts).unwrap_or_default();
            let roots_json = path_strings_json_normalized(&snap.roots);
            conn.execute(
                "INSERT OR REPLACE INTO preset_scans (id, timestamp, preset_count, total_bytes, format_counts, roots, scan_complete) VALUES (?1,?2,?3,?4,?5,?6,1)",
                params![snap.id, snap.timestamp, snap.preset_count as i64, snap.total_bytes as i64, fc_json, roots_json],
            ).map_err(|e| e.to_string())?;

            let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
            {
                let mut stmt = tx.prepare_cached(
                    "INSERT OR REPLACE INTO presets (name, path, directory, format, size, size_formatted, modified, scan_id) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)"
                ).map_err(|e| e.to_string())?;
                for p in &snap.presets {
                    let path = normalize_path_for_db(&p.path);
                    let directory = normalize_path_for_db(&p.directory);
                    stmt.execute(params![
                        p.name,
                        path,
                        directory,
                        p.format,
                        p.size as i64,
                        p.size_formatted,
                        p.modified,
                        snap.id
                    ])
                    .map_err(|e| e.to_string())?;
                }
            }
            tx.commit().map_err(|e| e.to_string())?;
            count += snap.presets.len();
        }
        Ok(count)
    }

    fn migrate_kvr_json(&self, data_dir: &std::path::Path) -> Result<usize, String> {
        let path = data_dir.join("kvr-cache.json");
        if !path.exists() {
            return Ok(0);
        }
        let data = std::fs::read_to_string(&path).unwrap_or_default();
        let cache: HashMap<String, KvrCacheEntry> = serde_json::from_str(&data).unwrap_or_default();
        if cache.is_empty() {
            return Ok(0);
        }
        let conn = self.read_conn();
        let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
        let count = cache.len();
        {
            let mut stmt = tx.prepare_cached(
                "INSERT OR REPLACE INTO kvr_cache (plugin_key, kvr_url, update_url, latest_version, has_update, source, timestamp) VALUES (?1,?2,?3,?4,?5,?6,?7)"
            ).map_err(|e| e.to_string())?;
            for (key, entry) in &cache {
                stmt.execute(params![
                    key,
                    entry.kvr_url,
                    entry.update_url,
                    entry.latest_version,
                    entry.has_update as i32,
                    entry.source,
                    entry.timestamp
                ])
                .map_err(|e| e.to_string())?;
            }
        }
        tx.commit().map_err(|e| e.to_string())?;
        Ok(count)
    }

    /// Generic key→value JSON cache migration (xref, waveform, spectrogram, fingerprint).
    fn migrate_kv_cache(
        &self,
        data_dir: &std::path::Path,
        filename: &str,
        table: &str,
        key_col: &str,
        val_col: &str,
    ) -> Result<usize, String> {
        let path = data_dir.join(filename);
        if !path.exists() {
            return Ok(0);
        }
        let data = std::fs::read_to_string(&path).unwrap_or_default();
        let cache: HashMap<String, serde_json::Value> =
            serde_json::from_str(&data).unwrap_or_default();
        if cache.is_empty() {
            return Ok(0);
        }
        let conn = self.read_conn();
        let sql = format!("INSERT OR REPLACE INTO {table} ({key_col}, {val_col}) VALUES (?1, ?2)");
        let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
        let count = cache.len();
        {
            let mut stmt = tx.prepare_cached(&sql).map_err(|e| e.to_string())?;
            for (k, v) in &cache {
                let k = normalize_path_for_db(k);
                let val_str = if v.is_string() {
                    v.as_str().unwrap_or("").to_string()
                } else {
                    v.to_string()
                };
                stmt.execute(params![k, val_str])
                    .map_err(|e| e.to_string())?;
            }
        }
        tx.commit().map_err(|e| e.to_string())?;
        Ok(count)
    }

    fn migrate_analysis_cache(
        &self,
        data_dir: &std::path::Path,
        filename: &str,
        field: &str,
    ) -> Result<(), String> {
        let path = data_dir.join(filename);
        if !path.exists() {
            return Ok(());
        }
        let data = std::fs::read_to_string(&path).unwrap_or_default();
        let cache: HashMap<String, serde_json::Value> =
            serde_json::from_str(&data).unwrap_or_default();
        if cache.is_empty() {
            return Ok(());
        }

        let conn = self.read_conn();
        let sql = match field {
            "bpm" => "UPDATE audio_samples SET bpm = ?1 WHERE path = ?2",
            "key" => "UPDATE audio_samples SET key_name = ?1 WHERE path = ?2",
            "lufs" => "UPDATE audio_samples SET lufs = ?1 WHERE path = ?2",
            _ => return Ok(()),
        };
        let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
        {
            let mut stmt = tx.prepare_cached(sql).map_err(|e| e.to_string())?;
            for (sample_path, value) in &cache {
                let sample_path = normalize_path_for_db(sample_path);
                match field {
                    "bpm" | "lufs" => {
                        if let Some(v) = value.as_f64() {
                            let _ = stmt.execute(params![v, sample_path]);
                        }
                    }
                    "key" => {
                        if let Some(v) = value.as_str() {
                            let _ = stmt.execute(params![v, sample_path]);
                        }
                    }
                    _ => {}
                }
            }
        }
        tx.commit().map_err(|e| e.to_string())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    /// `history::set_test_data_dir_path` is process-global; serialize migrate JSON tests.
    static MIGRATE_JSON_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn read_pool_cache_splits_budget_when_extra_is_32_readers() {
        let one = read_pool_cache_kib(1);
        let thirty_three = read_pool_cache_kib(33);
        assert_eq!(one, -32_768, "single reader capped at 32 MiB");
        assert!(
            thirty_three.abs() < one.abs(),
            "33 readers must use smaller per-connection cache than a lone reader"
        );
        assert_eq!(thirty_three, -i32::try_from((512_i64 * 1024) / 33).unwrap());
    }

    #[test]
    fn read_pool_mmap_splits_budget() {
        let m = read_pool_mmap_bytes(33);
        assert_eq!(m, 536_870_912_i64 / 33);
    }

    #[test]
    fn test_audio_query_params_json_empty_object_uses_defaults() {
        let v = serde_json::json!({});
        let p: AudioQueryParams = serde_json::from_value(v).expect("deserialize");
        assert_eq!(p.sort_key, "name");
        assert!(p.sort_asc);
        assert_eq!(p.limit, 200);
        assert_eq!(p.offset, 0);
        assert!(p.scan_id.is_none());
        assert!(p.search.is_none());
        assert!(p.format_filter.is_none());
    }

    #[test]
    fn test_audio_query_params_json_partial_snake_case_overrides() {
        let v = serde_json::json!({
            "sort_key": "modified",
            "sort_asc": false,
            "limit": 50,
            "offset": 100,
            "search": "kick"
        });
        let p: AudioQueryParams = serde_json::from_value(v).expect("deserialize");
        assert_eq!(p.sort_key, "modified");
        assert!(!p.sort_asc);
        assert_eq!(p.limit, 50);
        assert_eq!(p.offset, 100);
        assert_eq!(p.search.as_deref(), Some("kick"));
        assert!(p.scan_id.is_none());
        assert!(p.format_filter.is_none());
    }

    #[test]
    fn test_audio_query_params_json_scan_id_and_format_filter() {
        let v = serde_json::json!({
            "scan_id": "scan-abc-123",
            "format_filter": "WAV,AIFF"
        });
        let p: AudioQueryParams = serde_json::from_value(v).expect("deserialize");
        assert_eq!(p.scan_id.as_deref(), Some("scan-abc-123"));
        assert_eq!(p.format_filter.as_deref(), Some("WAV,AIFF"));
        assert_eq!(p.sort_key, "name");
        assert!(p.sort_asc);
        assert_eq!(p.limit, 200);
    }

    #[test]
    fn test_audio_query_params_explicit_zero_offset_keeps_default_limit() {
        let v = serde_json::json!({ "offset": 0, "limit": 25 });
        let p: AudioQueryParams = serde_json::from_value(v).expect("deserialize");
        assert_eq!(p.offset, 0);
        assert_eq!(p.limit, 25);
        assert_eq!(p.sort_key, "name");
    }

    #[test]
    fn cache_stats_analysis_rows_use_count_only_not_library_total() {
        let db = test_db();
        let samples = vec![sample("a.wav", "/a.wav", "WAV", 100)];
        db.save_scan("s1", "2024-01-01T00:00:00", 1, 100, &HashMap::new(), &[])
            .unwrap();
        db.insert_audio_batch("s1", &samples).unwrap();
        db.update_lufs("/a.wav", Some(-12.0)).unwrap();
        let stats = db.cache_stats().unwrap();
        let bpm = stats.iter().find(|s| s.key == "bpm").unwrap();
        let key = stats.iter().find(|s| s.key == "key").unwrap();
        let lufs = stats.iter().find(|s| s.key == "lufs").unwrap();
        assert_eq!(
            bpm.total, 0,
            "BPM row must not use library row count as Items denominator"
        );
        assert_eq!(key.total, 0);
        assert_eq!(lufs.total, 0);
        assert_eq!(lufs.count, 1);
        assert_eq!(lufs.size_bytes, 8);
    }

    fn test_db() -> Database {
        // Shared-cache in-memory DB so writer + read pool see the same tables (matches on-disk open).
        let uri = format!(
            "file:memdb_{}?mode=memory&cache=shared",
            rand::random::<u128>()
        );
        let write = Connection::open(uri.as_str()).unwrap();
        write
            .execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")
            .unwrap();
        install_regexp_function(&write).unwrap();
        let mut db = Database {
            write: Mutex::new(write),
            read: Vec::new(),
            read_deadlines: Vec::new(),
            read_idx: AtomicUsize::new(0),
            midi_library_total_cache: Mutex::new(None),
            video_library_total_cache: Mutex::new(None),
            pdf_library_total_cache: Mutex::new(None),
            audio_library_total_cache: Mutex::new(None),
            preset_inventory_total_cache: Mutex::new(None),
            daw_library_total_cache: Mutex::new(None),
            plugin_library_total_cache: Mutex::new(None),
        };
        db.migrate().unwrap();
        let read = Connection::open(uri.as_str()).unwrap();
        read.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")
            .unwrap();
        install_regexp_function(&read).unwrap();
        let deadline = Arc::new(AtomicU64::new(now_epoch_ms()));
        db.read.push(Mutex::new(read));
        db.read_deadlines.push(deadline);
        db
    }

    fn sample(name: &str, path: &str, fmt: &str, size: u64) -> AudioSample {
        AudioSample {
            name: name.into(),
            path: path.into(),
            directory: "/test".into(),
            format: fmt.into(),
            size,
            size_formatted: crate::format_size(size),
            modified: "2024-01-01".into(),
            duration: None,
            channels: None,
            sample_rate: None,
            bits_per_sample: None,
        }
    }

    #[test]
    fn test_insert_and_query() {
        let db = test_db();
        let samples = vec![
            sample("kick.wav", "/test/kick.wav", "WAV", 1000),
            sample("snare.wav", "/test/snare.wav", "WAV", 2000),
            sample("hat.mp3", "/test/hat.mp3", "MP3", 500),
        ];
        db.save_scan(
            "scan1",
            "2024-01-01T00:00:00",
            3,
            3500,
            &HashMap::new(),
            &[],
        )
        .unwrap();
        db.insert_audio_batch("scan1", &samples).unwrap();

        let result = db
            .query_audio(&AudioQueryParams {
                scan_id: Some("scan1".into()),
                search: None,
                search_regex: false,
                format_filter: None,
                sort_key: "name".into(),
                sort_asc: true,
                offset: 0,
                limit: 100,
            })
            .unwrap();

        assert_eq!(result.total_count, 3);
        assert_eq!(result.samples.len(), 3);
        assert_eq!(result.samples[0].name, "hat.mp3");
    }

    #[test]
    fn test_search_subsequence() {
        let db = test_db();
        let samples = vec![
            sample("kick_hard.wav", "/test/kick_hard.wav", "WAV", 1000),
            sample("snare_soft.wav", "/test/snare_soft.wav", "WAV", 2000),
            sample("kick_808.wav", "/test/kick_808.wav", "WAV", 1500),
        ];
        db.save_scan("s1", "2024-01-01T00:00:00", 3, 4500, &HashMap::new(), &[])
            .unwrap();
        db.insert_audio_batch("s1", &samples).unwrap();

        // "kick" should match both "kick_hard.wav" and "kick_808.wav" via FTS5 substring.
        let result = db
            .query_audio(&AudioQueryParams {
                scan_id: Some("s1".into()),
                search: Some("kick".into()),
                search_regex: false,
                format_filter: None,
                sort_key: "name".into(),
                sort_asc: true,
                offset: 0,
                limit: 100,
            })
            .unwrap();

        assert_eq!(result.total_count, 2);
    }

    /// Regex mode (`search_regex`) uses SQLite `REGEXP` + Rust `regex` (case-insensitive), not FTS5
    /// phrase search — so `F[a][n]` matches `Fan` (JS `RegExp` semantics), not a literal `[` substring.
    #[test]
    fn test_query_audio_regex_mode_bracket_class_matches_name() {
        let db = test_db();
        let samples = vec![
            sample("Fan.wav", "/test/Fan.wav", "WAV", 1000),
            sample("Fox.wav", "/test/Fox.wav", "WAV", 2000),
        ];
        db.save_scan("s1", "2024-01-01T00:00:00", 2, 3000, &HashMap::new(), &[])
            .unwrap();
        db.insert_audio_batch("s1", &samples).unwrap();

        let result = db
            .query_audio(&AudioQueryParams {
                scan_id: Some("s1".into()),
                search: Some("F[a][n]".into()),
                search_regex: true,
                format_filter: None,
                sort_key: "name".into(),
                sort_asc: true,
                offset: 0,
                limit: 100,
            })
            .unwrap();

        assert_eq!(result.total_count, 1);
        assert_eq!(result.samples[0].name, "Fan.wav");
    }

    /// v13 backfill: rows in `audio_samples` without FTS shadow rows produced zero `MATCH` hits.
    #[test]
    fn test_backfill_contentless_fts_restores_audio_match() {
        let db = test_db();
        let samples = vec![sample("kick.wav", "/test/kick.wav", "WAV", 1000)];
        db.save_scan("s1", "2024-01-01T00:00:00", 1, 1000, &HashMap::new(), &[])
            .unwrap();
        db.insert_audio_batch("s1", &samples).unwrap();

        {
            let conn = db.read_conn();
            conn.execute("DELETE FROM audio_samples_fts", [])
                .expect("clear FTS");
        }

        let empty = db
            .query_audio(&AudioQueryParams {
                scan_id: Some("s1".into()),
                search: Some("kick".into()),
                search_regex: false,
                format_filter: None,
                sort_key: "name".into(),
                sort_asc: true,
                offset: 0,
                limit: 100,
            })
            .unwrap();
        assert_eq!(empty.total_count, 0);

        {
            let conn = db.read_conn();
            backfill_contentless_fts(&conn).expect("backfill");
        }

        let restored = db
            .query_audio(&AudioQueryParams {
                scan_id: Some("s1".into()),
                search: Some("kick".into()),
                search_regex: false,
                format_filter: None,
                sort_key: "name".into(),
                sort_asc: true,
                offset: 0,
                limit: 100,
            })
            .unwrap();
        assert_eq!(restored.total_count, 1);
    }

    /// Search (name subsequence) + pagination: verifies full query_audio path.
    /// With FTS active, row order is bm25 relevance (not column sort); UI may re-rank client-side.
    #[test]
    fn test_query_audio_search_subsequence_and_sort_size_desc() {
        use std::collections::HashSet;
        let db = test_db();
        let samples = vec![
            sample("small_kick.wav", "/test/small_kick.wav", "WAV", 100),
            sample("big_kick.wav", "/test/big_kick.wav", "WAV", 9_999),
            sample("snare.wav", "/test/snare.wav", "WAV", 500),
        ];
        db.save_scan("s1", "2024-01-01T00:00:00", 3, 10_599, &HashMap::new(), &[])
            .unwrap();
        db.insert_audio_batch("s1", &samples).unwrap();

        let result = db
            .query_audio(&AudioQueryParams {
                scan_id: Some("s1".into()),
                search: Some("kick".into()),
                search_regex: false,
                format_filter: None,
                sort_key: "size".into(),
                sort_asc: false,
                offset: 0,
                limit: 10,
            })
            .unwrap();

        assert_eq!(result.total_count, 2);
        let names: HashSet<&str> = result.samples.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains("big_kick.wav") && names.contains("small_kick.wav"));

        let by_size = db
            .query_audio(&AudioQueryParams {
                scan_id: Some("s1".into()),
                search: None,
                search_regex: false,
                format_filter: None,
                sort_key: "size".into(),
                sort_asc: false,
                offset: 0,
                limit: 10,
            })
            .unwrap();
        assert_eq!(by_size.samples[0].name, "big_kick.wav");
        assert_eq!(by_size.samples[0].size, 9_999);
    }

    #[test]
    fn test_format_filter() {
        let db = test_db();
        let samples = vec![
            sample("a.wav", "/a.wav", "WAV", 100),
            sample("b.mp3", "/b.mp3", "MP3", 200),
            sample("c.wav", "/c.wav", "WAV", 300),
        ];
        db.save_scan("s1", "2024-01-01T00:00:00", 3, 600, &HashMap::new(), &[])
            .unwrap();
        db.insert_audio_batch("s1", &samples).unwrap();

        let result = db
            .query_audio(&AudioQueryParams {
                scan_id: Some("s1".into()),
                search: None,
                search_regex: false,
                format_filter: Some("WAV".into()),
                sort_key: "name".into(),
                sort_asc: true,
                offset: 0,
                limit: 100,
            })
            .unwrap();

        assert_eq!(result.total_count, 2);
        assert!(result.samples.iter().all(|s| s.format == "WAV"));
    }

    /// User search uses SQL LIKE: `%` and `_` in the query string must be escaped (not wildcards).
    #[test]
    fn test_query_audio_search_escapes_percent_in_user_query() {
        let db = test_db();
        let samples = vec![
            sample("kick.wav", "/kick.wav", "WAV", 100),
            sample("100%_wet.wav", "/w.wav", "WAV", 200),
        ];
        db.save_scan("s1", "2024-01-01T00:00:00", 2, 300, &HashMap::new(), &[])
            .unwrap();
        db.insert_audio_batch("s1", &samples).unwrap();

        let result = db
            .query_audio(&AudioQueryParams {
                scan_id: Some("s1".into()),
                search: Some("100%".into()),
                search_regex: false,
                format_filter: None,
                sort_key: "name".into(),
                sort_asc: true,
                offset: 0,
                limit: 100,
            })
            .unwrap();

        assert_eq!(result.total_count, 1);
        assert_eq!(result.samples[0].name, "100%_wet.wav");
    }

    #[test]
    fn test_query_audio_search_escapes_underscore_in_user_query() {
        let db = test_db();
        let samples = vec![
            sample("ab.wav", "/ab.wav", "WAV", 100),
            sample("a_b.wav", "/a_b.wav", "WAV", 200),
        ];
        db.save_scan("s1", "2024-01-01T00:00:00", 2, 300, &HashMap::new(), &[])
            .unwrap();
        db.insert_audio_batch("s1", &samples).unwrap();

        let result = db
            .query_audio(&AudioQueryParams {
                scan_id: Some("s1".into()),
                search: Some("a_b".into()),
                search_regex: false,
                format_filter: None,
                sort_key: "name".into(),
                sort_asc: true,
                offset: 0,
                limit: 100,
            })
            .unwrap();

        assert_eq!(result.total_count, 1);
        assert_eq!(result.samples[0].name, "a_b.wav");
    }

    /// Unknown `sort_key` falls back to name (NOCASE), same as the default branch in `query_audio`.
    #[test]
    fn test_query_audio_unknown_sort_key_defaults_to_name() {
        let db = test_db();
        let samples = vec![
            sample("zebra.wav", "/z.wav", "WAV", 100),
            sample("Alpha.wav", "/a.wav", "WAV", 200),
        ];
        db.save_scan("s1", "2024-01-01T00:00:00", 2, 300, &HashMap::new(), &[])
            .unwrap();
        db.insert_audio_batch("s1", &samples).unwrap();

        let result = db
            .query_audio(&AudioQueryParams {
                scan_id: Some("s1".into()),
                search: None,
                search_regex: false,
                format_filter: None,
                sort_key: "not_a_supported_column".into(),
                sort_asc: true,
                offset: 0,
                limit: 100,
            })
            .unwrap();

        assert_eq!(result.samples[0].name, "Alpha.wav");
        assert_eq!(result.samples[1].name, "zebra.wav");
    }

    #[test]
    fn test_format_filter_all_does_not_restrict() {
        let db = test_db();
        let samples = vec![
            sample("a.wav", "/a.wav", "WAV", 100),
            sample("b.mp3", "/b.mp3", "MP3", 200),
        ];
        db.save_scan("s1", "2024-01-01T00:00:00", 2, 300, &HashMap::new(), &[])
            .unwrap();
        db.insert_audio_batch("s1", &samples).unwrap();

        let result = db
            .query_audio(&AudioQueryParams {
                scan_id: Some("s1".into()),
                search: None,
                search_regex: false,
                format_filter: Some("all".into()),
                sort_key: "name".into(),
                sort_asc: true,
                offset: 0,
                limit: 100,
            })
            .unwrap();

        assert_eq!(result.total_count, 2);
        assert_eq!(result.total_unfiltered, 2);
    }

    // ── Filter-aware aggregate stats (`*filter_stats` — disk bar / breakdown) ──

    #[test]
    fn test_audio_filter_stats_empty_db() {
        let db = test_db();
        let st = db.audio_filter_stats(None, None, false).unwrap();
        assert_eq!(st.count, 0);
        assert_eq!(st.total_bytes, 0);
        assert_eq!(st.total_unfiltered, 0);
        assert!(st.by_type.is_empty());
        assert_eq!(st.size_buckets, vec![0u64; 6]);
    }

    #[test]
    fn test_audio_filter_stats_unfiltered_breakdown() {
        let db = test_db();
        let samples = vec![
            sample("kick.wav", "/kick.wav", "WAV", 100),
            sample("snare.wav", "/snare.wav", "WAV", 200),
            sample("loop.mp3", "/loop.mp3", "MP3", 400),
        ];
        db.save_scan("s1", "2024-01-01T00:00:00", 3, 700, &HashMap::new(), &[])
            .unwrap();
        db.insert_audio_batch("s1", &samples).unwrap();

        let st = db.audio_filter_stats(None, None, false).unwrap();
        assert_eq!(st.total_unfiltered, 3);
        assert_eq!(st.count, 3);
        assert_eq!(st.total_bytes, 700);
        assert_eq!(st.by_type.get("WAV").copied().unwrap_or(0), 2);
        assert_eq!(st.by_type.get("MP3").copied().unwrap_or(0), 1);
        assert_eq!(st.bytes_by_type.get("WAV").copied().unwrap_or(0), 300);
        assert_eq!(st.bytes_by_type.get("MP3").copied().unwrap_or(0), 400);
        assert_eq!(st.size_buckets.len(), 6);
        assert_eq!(st.size_buckets.iter().sum::<u64>(), st.count);
        assert_eq!(st.size_buckets[0], 3); // all < 100 KiB
    }

    #[test]
    fn test_audio_filter_stats_search_subsequence() {
        let db = test_db();
        let samples = vec![
            sample("kick_hard.wav", "/k.wav", "WAV", 100),
            sample("snare.wav", "/s.wav", "WAV", 200),
            sample("kick_soft.wav", "/ks.wav", "WAV", 300),
        ];
        db.save_scan("s1", "2024-01-01T00:00:00", 3, 600, &HashMap::new(), &[])
            .unwrap();
        db.insert_audio_batch("s1", &samples).unwrap();

        let st = db.audio_filter_stats(Some("kick"), None, false).unwrap();
        assert_eq!(st.total_unfiltered, 3);
        assert_eq!(st.count, 2);
        assert_eq!(st.total_bytes, 400);
    }

    #[test]
    fn test_audio_filter_stats_format_single_and_multi() {
        let db = test_db();
        let samples = vec![
            sample("a.wav", "/a.wav", "WAV", 100),
            sample("b.aiff", "/b.aiff", "AIFF", 200),
            sample("c.mp3", "/c.mp3", "MP3", 400),
        ];
        db.save_scan("s1", "2024-01-01T00:00:00", 3, 700, &HashMap::new(), &[])
            .unwrap();
        db.insert_audio_batch("s1", &samples).unwrap();

        let w = db.audio_filter_stats(None, Some("WAV"), false).unwrap();
        assert_eq!(w.count, 1);
        assert_eq!(w.total_unfiltered, 3);
        assert_eq!(w.by_type.len(), 1);

        let wm = db
            .audio_filter_stats(None, Some("WAV,AIFF"), false)
            .unwrap();
        assert_eq!(wm.count, 2);
        assert_eq!(wm.total_bytes, 300);
    }

    #[test]
    fn test_audio_filter_stats_format_all_noop() {
        let db = test_db();
        let samples = vec![sample("x.wav", "/x.wav", "WAV", 10)];
        db.save_scan("s1", "2024-01-01T00:00:00", 1, 10, &HashMap::new(), &[])
            .unwrap();
        db.insert_audio_batch("s1", &samples).unwrap();

        let st = db.audio_filter_stats(None, Some("all"), false).unwrap();
        assert_eq!(st.count, 1);
        assert_eq!(st.total_unfiltered, 1);
    }

    #[test]
    fn test_daw_filter_stats_daw_filter_and_search() {
        let db = test_db();
        db.save_daw_scan(&daw_snap(
            "ds-fs1",
            "2024-06-01T00:00:00",
            vec![
                daw_project("a.als", "Ableton Live"),
                daw_project("b.als", "Ableton Live"),
                daw_project("c.logicx", "Logic Pro"),
            ],
        ))
        .unwrap();

        let unfiltered = db.daw_filter_stats(None, None, false).unwrap();
        assert_eq!(unfiltered.total_unfiltered, 3);
        assert_eq!(unfiltered.count, 3);
        assert_eq!(
            unfiltered.by_type.get("Ableton Live").copied().unwrap_or(0),
            2
        );
        assert_eq!(unfiltered.by_type.get("Logic Pro").copied().unwrap_or(0), 1);

        let abl = db
            .daw_filter_stats(None, Some("Ableton Live"), false)
            .unwrap();
        assert_eq!(abl.count, 2);
        assert_eq!(abl.total_unfiltered, 3);
        assert_eq!(abl.total_bytes, 2000);

        let search = db.daw_filter_stats(Some("a.als"), None, false).unwrap();
        assert_eq!(search.count, 1);
        assert_eq!(search.total_unfiltered, 3);
    }

    #[test]
    fn test_daw_filter_stats_empty_db() {
        let db = test_db();
        let st = db.daw_filter_stats(None, None, false).unwrap();
        assert_eq!(st.count, 0);
        assert_eq!(st.total_unfiltered, 0);
    }

    #[test]
    fn test_preset_filter_stats_respects_midi_exclusion() {
        let db = test_db();
        db.save_preset_scan(&preset_snap(
            "pr-fs",
            "2024-06-01T00:00:00",
            vec![
                preset_file("a.fxp", "FXP"),
                preset_file("b.fxp", "FXP"),
                preset_file("c.mid", "MID"),
            ],
        ))
        .unwrap();

        let st = db.preset_filter_stats(None, None, false).unwrap();
        assert_eq!(st.total_unfiltered, 2);
        assert_eq!(st.count, 2);
        assert_eq!(st.by_type.get("FXP").copied().unwrap_or(0), 2);

        let fx = db.preset_filter_stats(None, Some("FXP"), false).unwrap();
        assert_eq!(fx.count, 2);
        assert_eq!(fx.total_unfiltered, 2);
    }

    #[test]
    fn test_preset_filter_stats_search_subsequence() {
        let db = test_db();
        db.save_preset_scan(&preset_snap(
            "pr-fs2",
            "2024-06-01T00:00:00",
            vec![
                preset_file("lead_brass.fxp", "FXP"),
                preset_file("kick.wav", "WAV"),
            ],
        ))
        .unwrap();

        let st = db.preset_filter_stats(Some("brass"), None, false).unwrap();
        assert_eq!(st.count, 1);
        assert_eq!(st.total_unfiltered, 2);
    }

    #[test]
    fn test_plugin_filter_stats_type_and_search() {
        let db = test_db();
        db.save_plugin_scan(&plugin_snap(
            "ps-fs",
            "2024-06-01T00:00:00",
            vec![
                plugin_info("Serum", "VST3", "Xfer"),
                plugin_info("Diva", "AU", "u-he"),
                plugin_info("Vital", "VST3", "Matt"),
            ],
        ))
        .unwrap();

        let st = db.plugin_filter_stats(None, None, false).unwrap();
        assert_eq!(st.total_unfiltered, 3);
        assert_eq!(st.count, 3);
        assert_eq!(st.by_type.get("VST3").copied().unwrap_or(0), 2);

        let vst = db.plugin_filter_stats(None, Some("VST3"), false).unwrap();
        assert_eq!(vst.count, 2);
        assert_eq!(vst.total_bytes, 2_000_000);

        let xfer = db.plugin_filter_stats(Some("Xfer"), None, false).unwrap();
        assert_eq!(xfer.count, 1);
        assert_eq!(xfer.total_unfiltered, 3);
    }

    #[test]
    fn test_plugin_filter_stats_multi_type() {
        let db = test_db();
        db.save_plugin_scan(&plugin_snap(
            "ps-fs2",
            "2024-06-01T00:00:00",
            vec![plugin_info("A", "VST3", "X"), plugin_info("B", "AU", "X")],
        ))
        .unwrap();

        let st = db
            .plugin_filter_stats(None, Some("VST3,AU"), false)
            .unwrap();
        assert_eq!(st.count, 2);
        assert_eq!(st.total_unfiltered, 2);
    }

    #[test]
    fn test_plugin_filter_stats_empty_db() {
        let db = test_db();
        let st = db.plugin_filter_stats(None, None, false).unwrap();
        assert_eq!(st.count, 0);
        assert_eq!(st.total_unfiltered, 0);
    }

    #[test]
    fn test_pdf_filter_stats_search_and_totals() {
        let db = test_db();
        let snap = PdfScanSnapshot {
            id: "pdf-fs".into(),
            timestamp: "2024-07-01T00:00:00".into(),
            pdf_count: 2,
            total_bytes: 300,
            pdfs: vec![
                PdfFile {
                    name: "manual".into(),
                    path: "/docs/manual.pdf".into(),
                    directory: "/docs".into(),
                    size: 100,
                    size_formatted: "100 B".into(),
                    modified: "2024-06-01".into(),
                    ..Default::default()
                },
                PdfFile {
                    name: "readme_extra".into(),
                    path: "/docs/readme_extra.pdf".into(),
                    directory: "/docs".into(),
                    size: 200,
                    size_formatted: "200 B".into(),
                    modified: "2024-06-02".into(),
                    ..Default::default()
                },
            ],
            roots: vec!["/docs".into()],
        };
        db.save_pdf_scan(&snap).unwrap();

        let all = db.pdf_filter_stats(None, false).unwrap();
        assert_eq!(all.total_unfiltered, 2);
        assert_eq!(all.count, 2);
        assert_eq!(all.total_bytes, 300);
        assert!(all.by_type.is_empty());

        let sub = db.pdf_filter_stats(Some("readme"), false).unwrap();
        assert_eq!(sub.count, 1);
        assert_eq!(sub.total_bytes, 200);
        assert_eq!(sub.total_unfiltered, 2);
    }

    #[test]
    fn test_pdf_filter_stats_empty_db() {
        let db = test_db();
        let st = db.pdf_filter_stats(None, false).unwrap();
        assert_eq!(st.count, 0);
        assert_eq!(st.total_unfiltered, 0);
    }

    /// Whitespace-only `search` is treated as no search (same row set as `search: None`).
    #[test]
    fn test_query_audio_whitespace_only_search_is_noop() {
        let db = test_db();
        let samples = vec![
            sample("first.wav", "/first.wav", "WAV", 100),
            sample("second.wav", "/second.wav", "WAV", 200),
        ];
        db.save_scan("s1", "2024-01-01T00:00:00", 2, 300, &HashMap::new(), &[])
            .unwrap();
        db.insert_audio_batch("s1", &samples).unwrap();

        let with_spaces = db
            .query_audio(&AudioQueryParams {
                scan_id: Some("s1".into()),
                search: Some("   \t  ".into()),
                search_regex: false,
                format_filter: None,
                sort_key: "name".into(),
                sort_asc: true,
                offset: 0,
                limit: 100,
            })
            .unwrap();
        let no_search = db
            .query_audio(&AudioQueryParams {
                scan_id: Some("s1".into()),
                search: None,
                search_regex: false,
                format_filter: None,
                sort_key: "name".into(),
                sort_asc: true,
                offset: 0,
                limit: 100,
            })
            .unwrap();
        assert_eq!(with_spaces.total_count, no_search.total_count);
        assert_eq!(with_spaces.total_count, 2);
    }

    #[test]
    fn test_batch_update_analysis_empty_batch_returns_zero() {
        let db = test_db();
        assert_eq!(db.batch_update_analysis(&[]).unwrap(), 0);
    }

    #[test]
    fn test_get_analysis_unknown_path_returns_empty_object() {
        let db = test_db();
        let samples = vec![sample("a.wav", "/a.wav", "WAV", 100)];
        db.save_scan("s1", "2024-01-01T00:00:00", 1, 100, &HashMap::new(), &[])
            .unwrap();
        db.insert_audio_batch("s1", &samples).unwrap();

        let j = db.get_analysis("/no/such/file.wav").unwrap();
        assert!(j.is_object());
        assert!(j.as_object().unwrap().is_empty());
    }

    #[test]
    fn test_pagination() {
        let db = test_db();
        let samples: Vec<_> = (0..50)
            .map(|i| {
                sample(
                    &format!("s{i:03}.wav"),
                    &format!("/s{i:03}.wav"),
                    "WAV",
                    100,
                )
            })
            .collect();
        db.save_scan("s1", "2024-01-01T00:00:00", 50, 5000, &HashMap::new(), &[])
            .unwrap();
        db.insert_audio_batch("s1", &samples).unwrap();

        let page1 = db
            .query_audio(&AudioQueryParams {
                scan_id: Some("s1".into()),
                search: None,
                search_regex: false,
                format_filter: None,
                sort_key: "name".into(),
                sort_asc: true,
                offset: 0,
                limit: 10,
            })
            .unwrap();

        assert_eq!(page1.total_count, 50);
        assert_eq!(page1.samples.len(), 10);
        assert_eq!(page1.samples[0].name, "s000.wav");

        let page2 = db
            .query_audio(&AudioQueryParams {
                scan_id: Some("s1".into()),
                search: None,
                search_regex: false,
                format_filter: None,
                sort_key: "name".into(),
                sort_asc: true,
                offset: 10,
                limit: 10,
            })
            .unwrap();

        assert_eq!(page2.samples[0].name, "s010.wav");
    }

    #[test]
    fn test_update_analysis() {
        let db = test_db();
        let samples = vec![sample("kick.wav", "/kick.wav", "WAV", 1000)];
        db.save_scan("s1", "2024-01-01T00:00:00", 1, 1000, &HashMap::new(), &[])
            .unwrap();
        db.insert_audio_batch("s1", &samples).unwrap();

        db.update_bpm("/kick.wav", Some(120.0)).unwrap();
        db.update_key("/kick.wav", Some("C minor")).unwrap();
        db.update_lufs("/kick.wav", Some(-14.5)).unwrap();

        let analysis = db.get_analysis("/kick.wav").unwrap();
        assert_eq!(analysis["bpm"], 120.0);
        assert_eq!(analysis["key"], "C minor");
        assert_eq!(analysis["lufs"], -14.5);
    }

    #[test]
    fn test_batch_update_analysis_and_audio_stats() {
        let db = test_db();
        let samples = vec![
            sample("a.wav", "/a.wav", "WAV", 100),
            sample("b.wav", "/b.wav", "WAV", 200),
        ];
        db.save_scan("s1", "2024-01-01T00:00:00", 2, 300, &HashMap::new(), &[])
            .unwrap();
        db.insert_audio_batch("s1", &samples).unwrap();

        assert_eq!(db.audio_stats(Some("s1")).unwrap().analyzed_count, 0);

        let rows: Vec<AnalysisBatchRow> = vec![
            ("/a.wav".into(), Some(128.0), Some("D".into()), Some(-12.0)),
            (
                "/b.wav".into(),
                Some(90.0),
                Some("A minor".into()),
                Some(-15.5),
            ),
        ];
        assert_eq!(db.batch_update_analysis(&rows).unwrap(), 2);

        let stats = db.audio_stats(Some("s1")).unwrap();
        assert_eq!(stats.analyzed_count, 2);
        assert_eq!(stats.sample_count, 2);
        assert_eq!(stats.total_bytes, 300);

        let ja = db.get_analysis("/a.wav").unwrap();
        assert_eq!(ja["bpm"], 128.0);
        assert_eq!(ja["key"], "D");
        assert_eq!(ja["lufs"], -12.0);

        let jb = db.get_analysis("/b.wav").unwrap();
        assert_eq!(jb["bpm"], 90.0);
        assert_eq!(jb["key"], "A minor");
        assert_eq!(jb["lufs"], -15.5);
    }

    #[test]
    fn test_unanalyzed_paths() {
        let db = test_db();
        let samples = vec![
            sample("a.wav", "/a.wav", "WAV", 100),
            sample("b.wav", "/b.wav", "WAV", 200),
        ];
        db.save_scan("s1", "2024-01-01T00:00:00", 2, 300, &HashMap::new(), &[])
            .unwrap();
        db.insert_audio_batch("s1", &samples).unwrap();
        db.update_bpm("/a.wav", Some(120.0)).unwrap();
        db.update_key("/a.wav", Some("C")).unwrap();
        db.update_lufs("/a.wav", Some(-12.0)).unwrap();

        let unanalyzed = db.unanalyzed_paths(100).unwrap();
        assert_eq!(unanalyzed.len(), 1);
        assert_eq!(unanalyzed[0], "/b.wav");
    }

    #[test]
    fn test_unanalyzed_paths_skips_when_bpm_exhausted() {
        let db = test_db();
        let samples = vec![sample("a.wav", "/a.wav", "WAV", 100)];
        db.save_scan("s1", "2024-01-01T00:00:00", 1, 100, &HashMap::new(), &[])
            .unwrap();
        db.insert_audio_batch("s1", &samples).unwrap();
        let rows: Vec<AnalysisBatchRow> =
            vec![("/a.wav".into(), None, Some("D".into()), Some(-12.0))];
        assert_eq!(db.batch_update_analysis(&rows).unwrap(), 1);
        let unanalyzed = db.unanalyzed_paths(100).unwrap();
        assert!(
            unanalyzed.is_empty(),
            "key+lufs present but BPM None should set bpm_exhausted and leave the queue"
        );
    }

    #[test]
    fn test_audio_stats() {
        let db = test_db();
        let samples = vec![
            sample("a.wav", "/a.wav", "WAV", 100),
            sample("b.mp3", "/b.mp3", "MP3", 200),
            sample("c.wav", "/c.wav", "WAV", 300),
        ];
        db.save_scan("s1", "2024-01-01T00:00:00", 3, 600, &HashMap::new(), &[])
            .unwrap();
        db.insert_audio_batch("s1", &samples).unwrap();

        let stats = db.audio_stats(Some("s1")).unwrap();
        assert_eq!(stats.sample_count, 3);
        assert_eq!(stats.total_bytes, 600);
        assert_eq!(stats.format_counts["WAV"], 2);
        assert_eq!(stats.format_counts["MP3"], 1);
    }

    #[test]
    fn test_delete_scan() {
        let db = test_db();
        let samples = vec![sample("a.wav", "/a.wav", "WAV", 100)];
        db.save_scan("s1", "2024-01-01T00:00:00", 1, 100, &HashMap::new(), &[])
            .unwrap();
        db.insert_audio_batch("s1", &samples).unwrap();

        db.delete_scan("s1").unwrap();

        let scans = db.list_scans().unwrap();
        assert!(scans.is_empty());

        let stats = db.audio_stats(Some("s1")).unwrap();
        assert_eq!(stats.sample_count, 0);
    }

    #[test]
    fn test_sort_directions() {
        let db = test_db();
        let samples = vec![
            sample("z.wav", "/z.wav", "WAV", 300),
            sample("a.wav", "/a.wav", "WAV", 100),
            sample("m.wav", "/m.wav", "WAV", 200),
        ];
        db.save_scan("s1", "2024-01-01T00:00:00", 3, 600, &HashMap::new(), &[])
            .unwrap();
        db.insert_audio_batch("s1", &samples).unwrap();

        let asc = db
            .query_audio(&AudioQueryParams {
                scan_id: Some("s1".into()),
                search: None,
                search_regex: false,
                format_filter: None,
                sort_key: "size".into(),
                sort_asc: true,
                offset: 0,
                limit: 100,
            })
            .unwrap();
        assert_eq!(asc.samples[0].size, 100);
        assert_eq!(asc.samples[2].size, 300);

        let desc = db
            .query_audio(&AudioQueryParams {
                scan_id: Some("s1".into()),
                search: None,
                search_regex: false,
                format_filter: None,
                sort_key: "size".into(),
                sort_asc: false,
                offset: 0,
                limit: 100,
            })
            .unwrap();
        assert_eq!(desc.samples[0].size, 300);
    }

    #[test]
    fn test_plugin_scan_roundtrip() {
        let db = test_db();
        let snap = ScanSnapshot {
            id: "ps1".into(),
            timestamp: "2024-06-01T00:00:00".into(),
            plugin_count: 2,
            plugins: vec![
                PluginInfo {
                    name: "Serum".into(),
                    path: "/vst/Serum.vst3".into(),
                    plugin_type: "VST3".into(),
                    version: "1.3".into(),
                    manufacturer: "Xfer".into(),
                    manufacturer_url: None,
                    size: "10 MB".into(),
                    size_bytes: 10_000_000,
                    modified: "2024-01-01".into(),
                    architectures: vec!["arm64".into()],
                },
                PluginInfo {
                    name: "Vital".into(),
                    path: "/vst/Vital.vst3".into(),
                    plugin_type: "VST3".into(),
                    version: "1.5".into(),
                    manufacturer: "Matt Tytel".into(),
                    manufacturer_url: Some("https://vital.audio".into()),
                    size: "5 MB".into(),
                    size_bytes: 5_000_000,
                    modified: "2024-02-01".into(),
                    architectures: vec!["arm64".into(), "x86_64".into()],
                },
            ],
            directories: vec!["/vst".into()],
            roots: vec!["/vst".into()],
        };
        db.save_plugin_scan(&snap).unwrap();

        let scans = db.get_plugin_scans().unwrap();
        assert_eq!(scans.len(), 1);
        assert_eq!(scans[0]["id"], "ps1");
        assert_eq!(scans[0]["pluginCount"], 2);

        let detail = db.get_plugin_scan_detail("ps1").unwrap();
        assert_eq!(detail.plugins.len(), 2);
        assert_eq!(detail.plugins[0].name, "Serum");
        assert_eq!(detail.plugins[1].manufacturer, "Matt Tytel");
        assert_eq!(detail.plugins[1].architectures, vec!["arm64", "x86_64"]);
    }

    /// Subsequence search (name/manufacturer/path) + sort by `size_bytes` DESC.
    #[test]
    fn test_query_plugins_search_subsequence_and_sort_size_desc() {
        let db = test_db();
        let snap = ScanSnapshot {
            id: "pg-sort-1".into(),
            timestamp: "2024-06-01T00:00:00".into(),
            plugin_count: 3,
            plugins: vec![
                PluginInfo {
                    name: "small_serum_label".into(),
                    path: "/vst/small.vst3".into(),
                    plugin_type: "VST3".into(),
                    version: "1.0".into(),
                    manufacturer: "Xfer".into(),
                    manufacturer_url: None,
                    size: "100 B".into(),
                    size_bytes: 100,
                    modified: "2024-01-01".into(),
                    architectures: vec![],
                },
                PluginInfo {
                    name: "big_serum_bank".into(),
                    path: "/vst/big.vst3".into(),
                    plugin_type: "VST3".into(),
                    version: "1.0".into(),
                    manufacturer: "Xfer".into(),
                    manufacturer_url: None,
                    size: "10 KB".into(),
                    size_bytes: 10_000,
                    modified: "2024-01-01".into(),
                    architectures: vec![],
                },
                PluginInfo {
                    name: "Other".into(),
                    // Path must not contain "s…e…r" subsequence (e.g. `/vst/…` matches "ser").
                    path: "/plugin/other.clap".into(),
                    plugin_type: "CLAP".into(),
                    version: "1.0".into(),
                    manufacturer: "ACME".into(),
                    manufacturer_url: None,
                    size: "5 MB".into(),
                    size_bytes: 5_000_000,
                    modified: "2024-01-01".into(),
                    architectures: vec![],
                },
            ],
            directories: vec!["/vst".into()],
            roots: vec!["/vst".into()],
        };
        db.save_plugin_scan(&snap).unwrap();

        let res = db
            .query_plugins(Some("ser"), None, None, "size", false, false, 0, 100)
            .unwrap();
        assert_eq!(res.total_count, 2);
        assert_eq!(res.plugins[0].name, "big_serum_bank");
        assert_eq!(res.plugins[0].size_bytes, 10_000);
        assert_eq!(res.plugins[1].name, "small_serum_label");
    }

    #[test]
    fn test_daw_scan_roundtrip() {
        let db = test_db();
        let mut daw_counts = HashMap::new();
        daw_counts.insert("Ableton".into(), 2);
        let snap = DawScanSnapshot {
            id: "ds1".into(),
            timestamp: "2024-06-01T00:00:00".into(),
            project_count: 2,
            total_bytes: 50_000,
            daw_counts,
            projects: vec![
                DawProject {
                    name: "track1.als".into(),
                    path: "/music/track1.als".into(),
                    directory: "/music".into(),
                    format: "ALS".into(),
                    daw: "Ableton".into(),
                    size: 30_000,
                    size_formatted: "30 KB".into(),
                    modified: "2024-03-01".into(),
                },
                DawProject {
                    name: "track2.als".into(),
                    path: "/music/track2.als".into(),
                    directory: "/music".into(),
                    format: "ALS".into(),
                    daw: "Ableton".into(),
                    size: 20_000,
                    size_formatted: "20 KB".into(),
                    modified: "2024-04-01".into(),
                },
            ],
            roots: vec!["/music".into()],
        };
        db.save_daw_scan(&snap).unwrap();

        let scans = db.get_daw_scans().unwrap();
        assert_eq!(scans.len(), 1);
        assert_eq!(scans[0]["projectCount"], 2);
        assert_eq!(scans[0]["totalBytes"], 50_000);

        let detail = db.get_daw_scan_detail("ds1").unwrap();
        assert_eq!(detail.projects.len(), 2);
        assert_eq!(detail.projects[0].daw, "Ableton");
    }

    /// History list must show live row counts even if `daw_scan_parent_finalize` never ran
    /// (parent row still has project_count = 0).
    #[test]
    fn test_get_daw_scans_project_count_from_child_table() {
        let db = test_db();
        let id = "daw-unfinalized";
        let ts = "2024-06-01T00:00:00";
        db.daw_scan_parent_create(id, ts, &["/roots".into()])
            .unwrap();
        let p = DawProject {
            name: "track.als".into(),
            path: "/music/track.als".into(),
            directory: "/music".into(),
            format: "ALS".into(),
            daw: "Ableton".into(),
            size: 100,
            size_formatted: "100 B".into(),
            modified: "2024-01-01".into(),
        };
        db.insert_daw_batch(id, &[p]).unwrap();
        db.set_daw_scan_complete(id, true)
            .expect("mark scan complete so history lists it");
        let scans = db.get_daw_scans().unwrap();
        assert_eq!(scans.len(), 1);
        assert_eq!(scans[0]["projectCount"], 1);
    }

    /// User-stopped (or unfinalized-incomplete) scans stay in the DB for library aggregation but
    /// must not appear in deletable history lists.
    #[test]
    fn test_incomplete_scan_hidden_from_plugin_history() {
        let db = test_db();
        db.plugin_scan_parent_create("ps-partial", "2024-01-01T00:00:00", &["/vst".into()])
            .unwrap();
        let p = PluginInfo {
            name: "X".into(),
            path: "/x.vst3".into(),
            plugin_type: "VST3".into(),
            version: "1".into(),
            manufacturer: "M".into(),
            manufacturer_url: None,
            size: "1 B".into(),
            size_bytes: 1,
            modified: "2024-01-01".into(),
            architectures: vec![],
        };
        db.insert_plugin_batch("ps-partial", &[p]).unwrap();
        db.plugin_scan_parent_finalize("ps-partial", 1, &["/vst".into()], &["/vst".into()])
            .unwrap();
        assert!(db.get_plugin_scans().unwrap().is_empty());
        db.set_plugin_scan_complete("ps-partial", true).unwrap();
        assert_eq!(db.get_plugin_scans().unwrap().len(), 1);
    }

    /// Subsequence search on name/path + sort by file size DESC.
    #[test]
    fn test_query_daw_search_subsequence_and_sort_size_desc() {
        let db = test_db();
        let mut daw_counts = HashMap::new();
        daw_counts.insert("Ableton".into(), 3);
        let snap = DawScanSnapshot {
            id: "ds-sort-1".into(),
            timestamp: "2024-06-01T00:00:00".into(),
            project_count: 3,
            total_bytes: 10_600,
            daw_counts,
            projects: vec![
                DawProject {
                    name: "small_mix_down.als".into(),
                    path: "/music/small_mix_down.als".into(),
                    directory: "/music".into(),
                    format: "ALS".into(),
                    daw: "Ableton".into(),
                    size: 100,
                    size_formatted: "100 B".into(),
                    modified: "2024-01-01".into(),
                },
                DawProject {
                    name: "big_mix_master.als".into(),
                    path: "/music/big_mix_master.als".into(),
                    directory: "/music".into(),
                    format: "ALS".into(),
                    daw: "Ableton".into(),
                    size: 10_000,
                    size_formatted: "10 KB".into(),
                    modified: "2024-01-01".into(),
                },
                DawProject {
                    name: "vocal_take.als".into(),
                    path: "/music/vocal_take.als".into(),
                    directory: "/music".into(),
                    format: "ALS".into(),
                    daw: "Ableton".into(),
                    size: 500,
                    size_formatted: "500 B".into(),
                    modified: "2024-01-01".into(),
                },
            ],
            roots: vec!["/music".into()],
        };
        db.save_daw_scan(&snap).unwrap();

        let res = db
            .query_daw(Some("mix"), None, "size", false, false, 0, 100)
            .unwrap();
        assert_eq!(res.total_count, 2);
        let names: HashSet<_> = res.projects.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(
            names,
            HashSet::from(["big_mix_master.als", "small_mix_down.als"])
        );

        let by_size = db
            .query_daw(None, None, "size", false, false, 0, 100)
            .unwrap();
        assert_eq!(by_size.projects.len(), 3);
        assert_eq!(by_size.projects[0].name, "big_mix_master.als");
        assert_eq!(by_size.projects[0].size, 10_000);
        assert_eq!(by_size.projects[1].name, "vocal_take.als");
        assert_eq!(by_size.projects[2].name, "small_mix_down.als");
    }

    #[test]
    fn test_preset_scan_roundtrip() {
        let db = test_db();
        let mut format_counts = HashMap::new();
        format_counts.insert("FXP".into(), 1);
        let snap = PresetScanSnapshot {
            id: "pr1".into(),
            timestamp: "2024-06-01T00:00:00".into(),
            preset_count: 1,
            total_bytes: 8000,
            format_counts,
            presets: vec![PresetFile {
                name: "bass.fxp".into(),
                path: "/presets/bass.fxp".into(),
                directory: "/presets".into(),
                format: "FXP".into(),
                size: 8000,
                size_formatted: "8 KB".into(),
                modified: "2024-05-01".into(),
            }],
            roots: vec!["/presets".into()],
        };
        db.save_preset_scan(&snap).unwrap();

        let scans = db.get_preset_scans().unwrap();
        assert_eq!(scans.len(), 1);
        assert_eq!(scans[0]["presetCount"], 1);

        let detail = db.get_preset_scan_detail("pr1").unwrap();
        assert_eq!(detail.presets.len(), 1);
        assert_eq!(detail.presets[0].name, "bass.fxp");
    }

    #[test]
    fn test_pdf_scan_roundtrip() {
        let db = test_db();
        let snap = PdfScanSnapshot {
            id: "pdf1".into(),
            timestamp: "2024-07-01T00:00:00".into(),
            pdf_count: 1,
            total_bytes: 1024,
            pdfs: vec![PdfFile {
                name: "readme".into(),
                path: "/docs/readme.pdf".into(),
                directory: "/docs".into(),
                size: 1024,
                size_formatted: "1.0 KB".into(),
                modified: "2024-06-01".into(),
                ..Default::default()
            }],
            roots: vec!["/docs".into()],
        };
        db.save_pdf_scan(&snap).unwrap();
        let scans = db.get_pdf_scans().unwrap();
        assert_eq!(scans.len(), 1);
        assert_eq!(scans[0]["pdfCount"], 1);
        let detail = db.get_pdf_scan_detail("pdf1").unwrap();
        assert_eq!(detail.pdfs.len(), 1);
        assert_eq!(detail.pdfs[0].name, "readme");
    }

    #[test]
    fn test_query_pdfs_empty_without_scan() {
        let db = test_db();
        let res = db.query_pdfs(None, "name", true, false, 0, 100).unwrap();
        assert_eq!(res.total_count, 0);
        assert_eq!(res.total_unfiltered, 0);
        assert!(res.pdfs.is_empty());
    }

    #[test]
    fn test_query_pdfs_search_sort_and_pagination() {
        let db = test_db();
        let pdfs = vec![
            PdfFile {
                name: "zebra".into(),
                path: "/a/z.pdf".into(),
                directory: "/a".into(),
                size: 100,
                size_formatted: "100 B".into(),
                modified: "2024-01-03".into(),
                ..Default::default()
            },
            PdfFile {
                name: "alpha".into(),
                path: "/a/a.pdf".into(),
                directory: "/b".into(),
                size: 50,
                size_formatted: "50 B".into(),
                modified: "2024-01-01".into(),
                ..Default::default()
            },
            PdfFile {
                name: "alpha_notes".into(),
                path: "/a/notes.pdf".into(),
                directory: "/c".into(),
                size: 50,
                size_formatted: "50 B".into(),
                modified: "2024-01-02".into(),
                ..Default::default()
            },
        ];
        let total_bytes: u64 = pdfs.iter().map(|p| p.size).sum();
        let snap = PdfScanSnapshot {
            id: "pdfq".into(),
            timestamp: "2024-08-01T00:00:00".into(),
            pdf_count: pdfs.len(),
            total_bytes,
            pdfs,
            roots: vec![],
        };
        db.save_pdf_scan(&snap).unwrap();

        let filtered = db
            .query_pdfs(Some("alp"), "name", true, false, 0, 100)
            .unwrap();
        assert_eq!(filtered.total_unfiltered, 3);
        assert_eq!(filtered.total_count, 2);
        assert_eq!(filtered.pdfs.len(), 2);
        let mut got: Vec<&str> = filtered.pdfs.iter().map(|p| p.name.as_str()).collect();
        got.sort();
        assert_eq!(got, vec!["alpha", "alpha_notes"]);

        let by_size = db.query_pdfs(None, "size", false, false, 0, 10).unwrap();
        assert_eq!(by_size.pdfs[0].name, "zebra");

        let page = db.query_pdfs(None, "name", true, false, 1, 1).unwrap();
        assert_eq!(page.pdfs.len(), 1);
        assert_eq!(page.total_count, 3);
        assert_eq!(page.pdfs[0].name, "alpha_notes");
    }

    #[test]
    fn test_query_pdfs_library_unions_distinct_paths_across_scans() {
        let db = test_db();
        db.save_pdf_scan(&PdfScanSnapshot {
            id: "old-pdf".into(),
            timestamp: "2024-01-01T00:00:00".into(),
            pdf_count: 1,
            total_bytes: 10,
            pdfs: vec![PdfFile {
                name: "old".into(),
                path: "/a/old.pdf".into(),
                directory: "/a".into(),
                size: 10,
                size_formatted: "10 B".into(),
                modified: "d".into(),
                ..Default::default()
            }],
            roots: vec![],
        })
        .unwrap();
        db.save_pdf_scan(&PdfScanSnapshot {
            id: "new-pdf".into(),
            timestamp: "2024-02-01T00:00:00".into(),
            pdf_count: 1,
            total_bytes: 20,
            pdfs: vec![PdfFile {
                name: "new".into(),
                path: "/b/new.pdf".into(),
                directory: "/b".into(),
                size: 20,
                size_formatted: "20 B".into(),
                modified: "d".into(),
                ..Default::default()
            }],
            roots: vec![],
        })
        .unwrap();
        let res = db.query_pdfs(None, "name", true, false, 0, 100).unwrap();
        assert_eq!(res.total_unfiltered, 2);
        assert_eq!(res.pdfs.len(), 2);
        assert_eq!(res.pdfs[0].name, "new");
        assert_eq!(res.pdfs[1].name, "old");

        // Regression: unindexed paths must include every library path missing metadata, not only
        // PDFs whose `pdfs.scan_id` is the latest completed scan (otherwise /a/old.pdf would
        // never be queued for page-count extraction).
        let mut unindexed = db.unindexed_pdf_paths(100).unwrap();
        unindexed.sort();
        assert_eq!(
            unindexed,
            vec!["/a/old.pdf".to_string(), "/b/new.pdf".to_string()]
        );
    }

    #[test]
    fn test_pdf_stats_matches_rows() {
        let db = test_db();
        let snap = PdfScanSnapshot {
            id: "pdf-stat".into(),
            timestamp: "2024-09-01T00:00:00".into(),
            pdf_count: 2,
            total_bytes: 300,
            pdfs: vec![
                PdfFile {
                    name: "a".into(),
                    path: "/p/a.pdf".into(),
                    directory: "/p".into(),
                    size: 100,
                    size_formatted: "100 B".into(),
                    modified: "d".into(),
                    ..Default::default()
                },
                PdfFile {
                    name: "b".into(),
                    path: "/p/b.pdf".into(),
                    directory: "/p".into(),
                    size: 200,
                    size_formatted: "200 B".into(),
                    modified: "d".into(),
                    ..Default::default()
                },
            ],
            roots: vec![],
        };
        db.save_pdf_scan(&snap).unwrap();
        let st = db.pdf_stats(None).unwrap();
        assert_eq!(st.pdf_count, 2);
        assert_eq!(st.total_bytes, 300);
        let st2 = db.pdf_stats(Some("pdf-stat")).unwrap();
        assert_eq!(st2.pdf_count, 2);
        assert_eq!(st2.total_bytes, 300);
    }

    // ── Header-count regression tests ──
    //
    // These verify that query_plugins/query_daw/query_presets return a
    // `total_unfiltered` that reflects the *library* (one row per path) and is
    // independent of any search/filter arguments. This is what drives the
    // header counters and must NOT drop to 0 when a filter excludes all rows.

    fn plugin_info(name: &str, ptype: &str, manufacturer: &str) -> PluginInfo {
        PluginInfo {
            name: name.into(),
            path: format!("/vst/{name}.vst3"),
            plugin_type: ptype.into(),
            version: "1.0".into(),
            manufacturer: manufacturer.into(),
            manufacturer_url: None,
            size: "1 MB".into(),
            size_bytes: 1_000_000,
            modified: "2024-01-01".into(),
            architectures: vec!["arm64".into()],
        }
    }

    #[test]
    fn test_delete_plugin_scan_removes_rows_and_get_latest_falls_back() {
        let db = test_db();
        db.save_plugin_scan(&ScanSnapshot {
            id: "pl-old".into(),
            timestamp: "2024-01-01T00:00:00".into(),
            plugin_count: 1,
            plugins: vec![plugin_info("Old", "VST3", "Xfer")],
            directories: vec!["/vst".into()],
            roots: vec!["/vst".into()],
        })
        .unwrap();
        db.save_plugin_scan(&ScanSnapshot {
            id: "pl-new".into(),
            timestamp: "2024-06-01T00:00:00".into(),
            plugin_count: 1,
            plugins: vec![plugin_info("New", "VST3", "Y")],
            directories: vec!["/vst".into()],
            roots: vec!["/vst".into()],
        })
        .unwrap();
        assert_eq!(db.get_latest_plugin_scan().unwrap().unwrap().id, "pl-new");

        db.delete_plugin_scan("pl-new").unwrap();

        assert!(db.get_plugin_scan_detail("pl-new").is_err());
        let latest = db.get_latest_plugin_scan().unwrap().unwrap();
        assert_eq!(latest.id, "pl-old");
        assert_eq!(latest.plugins[0].name, "Old");
    }

    #[test]
    fn test_clear_plugin_history_removes_all_plugin_scans() {
        let db = test_db();
        db.save_plugin_scan(&ScanSnapshot {
            id: "pc1".into(),
            timestamp: "2024-06-01T00:00:00".into(),
            plugin_count: 1,
            plugins: vec![plugin_info("One", "VST3", "Z")],
            directories: vec![],
            roots: vec![],
        })
        .unwrap();
        db.clear_plugin_history().unwrap();
        assert!(db.get_latest_plugin_scan().unwrap().is_none());
        assert!(db.get_plugin_scans().unwrap().is_empty());
    }

    #[test]
    fn test_query_plugins_total_unfiltered_with_filter_match_none() {
        let db = test_db();
        let snap = ScanSnapshot {
            id: "ps-hdr-1".into(),
            timestamp: "2024-06-01T00:00:00".into(),
            plugin_count: 3,
            plugins: vec![
                plugin_info("Serum", "VST3", "Xfer"),
                plugin_info("Vital", "VST3", "Matt Tytel"),
                plugin_info("Massive", "VST3", "NI"),
            ],
            directories: vec!["/vst".into()],
            roots: vec!["/vst".into()],
        };
        db.save_plugin_scan(&snap).unwrap();

        // Filter that matches nothing → filtered count 0, unfiltered stays 3
        let res = db
            .query_plugins(
                Some("nonexistent_xyz"),
                None,
                None,
                "name",
                true,
                false,
                0,
                100,
            )
            .unwrap();
        assert_eq!(res.total_count, 0, "filtered count should be 0");
        assert_eq!(
            res.total_unfiltered, 3,
            "unfiltered header count must reflect full scan, not filter"
        );
        assert!(res.plugins.is_empty());
    }

    #[test]
    fn test_query_plugins_total_unfiltered_matches_total_count_no_filter() {
        let db = test_db();
        let snap = ScanSnapshot {
            id: "ps-hdr-2".into(),
            timestamp: "2024-06-01T00:00:00".into(),
            plugin_count: 2,
            plugins: vec![
                plugin_info("Serum", "VST3", "Xfer"),
                plugin_info("Vital", "VST3", "Matt Tytel"),
            ],
            directories: vec!["/vst".into()],
            roots: vec!["/vst".into()],
        };
        db.save_plugin_scan(&snap).unwrap();

        let res = db
            .query_plugins(None, None, None, "name", true, false, 0, 100)
            .unwrap();
        assert_eq!(res.total_count, 2);
        assert_eq!(res.total_unfiltered, 2);
    }

    #[test]
    fn test_query_plugins_total_unfiltered_empty_db() {
        let db = test_db();
        let res = db
            .query_plugins(None, None, None, "name", true, false, 0, 100)
            .unwrap();
        assert_eq!(res.total_count, 0);
        assert_eq!(res.total_unfiltered, 0);
        assert!(res.plugins.is_empty());
    }

    fn daw_project(name: &str, daw: &str) -> DawProject {
        DawProject {
            name: name.into(),
            path: format!("/music/{name}"),
            directory: "/music".into(),
            format: "ALS".into(),
            daw: daw.into(),
            size: 1000,
            size_formatted: "1 KB".into(),
            modified: "2024-01-01".into(),
        }
    }

    #[test]
    fn test_query_daw_total_unfiltered_with_filter_match_none() {
        let db = test_db();
        let mut daw_counts = HashMap::new();
        daw_counts.insert("Ableton".into(), 2);
        daw_counts.insert("Logic".into(), 1);
        let snap = DawScanSnapshot {
            id: "ds-hdr-1".into(),
            timestamp: "2024-06-01T00:00:00".into(),
            project_count: 3,
            total_bytes: 3000,
            daw_counts,
            projects: vec![
                daw_project("t1.als", "Ableton"),
                daw_project("t2.als", "Ableton"),
                daw_project("t3.logicx", "Logic"),
            ],
            roots: vec!["/music".into()],
        };
        db.save_daw_scan(&snap).unwrap();

        // daw_filter that doesn't match any existing daw — filtered=0, unfiltered=3
        let res = db
            .query_daw(None, Some("FL Studio"), "name", true, false, 0, 100)
            .unwrap();
        assert_eq!(res.total_count, 0);
        assert_eq!(
            res.total_unfiltered, 3,
            "unfiltered count must include all 3 projects in the library scope"
        );
    }

    #[test]
    fn test_query_daw_total_unfiltered_with_search_filter() {
        let db = test_db();
        let mut daw_counts = HashMap::new();
        daw_counts.insert("Ableton".into(), 2);
        let snap = DawScanSnapshot {
            id: "ds-hdr-2".into(),
            timestamp: "2024-06-01T00:00:00".into(),
            project_count: 2,
            total_bytes: 2000,
            daw_counts,
            projects: vec![
                daw_project("bassline.als", "Ableton"),
                daw_project("drums.als", "Ableton"),
            ],
            roots: vec!["/music".into()],
        };
        db.save_daw_scan(&snap).unwrap();

        // Search that only matches 1 — filtered=1, unfiltered=2
        let res = db
            .query_daw(Some("bass"), None, "name", true, false, 0, 100)
            .unwrap();
        assert_eq!(res.total_count, 1);
        assert_eq!(res.total_unfiltered, 2);
    }

    #[test]
    fn test_query_daw_total_unfiltered_empty_db() {
        let db = test_db();
        let res = db
            .query_daw(None, None, "name", true, false, 0, 100)
            .unwrap();
        assert_eq!(res.total_count, 0);
        assert_eq!(res.total_unfiltered, 0);
    }

    fn preset_file(name: &str, fmt: &str) -> PresetFile {
        PresetFile {
            name: name.into(),
            path: format!("/presets/{name}"),
            directory: "/presets".into(),
            format: fmt.into(),
            size: 1000,
            size_formatted: "1 KB".into(),
            modified: "2024-01-01".into(),
        }
    }

    #[test]
    fn test_delete_preset_scan_removes_rows_and_get_latest_falls_back() {
        let db = test_db();
        let mut fc = HashMap::new();
        fc.insert("FXP".into(), 1);
        db.save_preset_scan(&PresetScanSnapshot {
            id: "pr-old".into(),
            timestamp: "2024-01-01T00:00:00".into(),
            preset_count: 1,
            total_bytes: 1000,
            format_counts: fc.clone(),
            presets: vec![preset_file("old.fxp", "FXP")],
            roots: vec![],
        })
        .unwrap();
        db.save_preset_scan(&PresetScanSnapshot {
            id: "pr-new".into(),
            timestamp: "2024-06-01T00:00:00".into(),
            preset_count: 1,
            total_bytes: 2000,
            format_counts: fc,
            presets: vec![preset_file("new.fxp", "FXP")],
            roots: vec![],
        })
        .unwrap();
        assert_eq!(db.get_latest_preset_scan().unwrap().unwrap().id, "pr-new");

        db.delete_preset_scan("pr-new").unwrap();

        assert!(db.get_preset_scan_detail("pr-new").is_err());
        let latest = db.get_latest_preset_scan().unwrap().unwrap();
        assert_eq!(latest.id, "pr-old");
        assert_eq!(latest.presets[0].name, "old.fxp");
    }

    #[test]
    fn test_query_presets_total_unfiltered_with_filter_match_none() {
        let db = test_db();
        let mut format_counts = HashMap::new();
        format_counts.insert("FXP".into(), 2);
        let snap = PresetScanSnapshot {
            id: "pr-hdr-1".into(),
            timestamp: "2024-06-01T00:00:00".into(),
            preset_count: 2,
            total_bytes: 2000,
            format_counts,
            presets: vec![
                preset_file("lead.fxp", "FXP"),
                preset_file("pad.fxp", "FXP"),
            ],
            roots: vec!["/presets".into()],
        };
        db.save_preset_scan(&snap).unwrap();

        let res = db
            .query_presets(None, Some("H2P"), "name", true, false, 0, 100)
            .unwrap();
        assert_eq!(res.total_count, 0);
        assert_eq!(res.total_unfiltered, 2);
    }

    #[test]
    fn test_query_presets_total_unfiltered_excludes_midi() {
        // MIDI files live in the presets table but are shown in their own tab.
        // `total_unfiltered` for presets must exclude MID/MIDI so the preset
        // header count matches what the preset view actually shows.
        let db = test_db();
        let mut format_counts = HashMap::new();
        format_counts.insert("FXP".into(), 1);
        format_counts.insert("MID".into(), 2);
        let snap = PresetScanSnapshot {
            id: "pr-hdr-2".into(),
            timestamp: "2024-06-01T00:00:00".into(),
            preset_count: 3,
            total_bytes: 3000,
            format_counts,
            presets: vec![
                preset_file("lead.fxp", "FXP"),
                preset_file("song.mid", "MID"),
                preset_file("beat.midi", "MIDI"),
            ],
            roots: vec!["/presets".into()],
        };
        db.save_preset_scan(&snap).unwrap();

        let res = db
            .query_presets(None, None, "name", true, false, 0, 100)
            .unwrap();
        assert_eq!(
            res.total_unfiltered, 1,
            "MIDI files must be excluded from preset header count"
        );
        assert_eq!(res.total_count, 1);
        assert!(
            res.presets
                .iter()
                .all(|p| p.format != "MID" && p.format != "MIDI")
        );
    }

    #[test]
    fn test_query_presets_total_unfiltered_with_search() {
        let db = test_db();
        let mut format_counts = HashMap::new();
        format_counts.insert("FXP".into(), 3);
        let snap = PresetScanSnapshot {
            id: "pr-hdr-3".into(),
            timestamp: "2024-06-01T00:00:00".into(),
            preset_count: 3,
            total_bytes: 3000,
            format_counts,
            presets: vec![
                preset_file("bass_sub.fxp", "FXP"),
                preset_file("bass_808.fxp", "FXP"),
                preset_file("lead_saw.fxp", "FXP"),
            ],
            roots: vec!["/presets".into()],
        };
        db.save_preset_scan(&snap).unwrap();

        let res = db
            .query_presets(Some("bass"), None, "name", true, false, 0, 100)
            .unwrap();
        assert_eq!(res.total_count, 2);
        assert_eq!(res.total_unfiltered, 3);
    }

    /// Subsequence search on name + pagination (full `query_presets` path).
    /// With FTS active, row order is bm25 relevance (not column sort); UI may re-rank client-side.
    #[test]
    fn test_query_presets_search_subsequence_and_sort_size_desc() {
        use std::collections::HashSet;
        let db = test_db();
        let mut format_counts = HashMap::new();
        format_counts.insert("FXP".into(), 3);
        let snap = PresetScanSnapshot {
            id: "pr-sort-1".into(),
            timestamp: "2024-06-01T00:00:00".into(),
            preset_count: 3,
            total_bytes: 10_200,
            format_counts,
            presets: vec![
                PresetFile {
                    name: "small_lead.fxp".into(),
                    path: "/p/small_lead.fxp".into(),
                    directory: "/p".into(),
                    format: "FXP".into(),
                    size: 100,
                    size_formatted: "100 B".into(),
                    modified: "2024-01-01".into(),
                },
                PresetFile {
                    name: "big_lead.fxp".into(),
                    path: "/p/big_lead.fxp".into(),
                    directory: "/p".into(),
                    format: "FXP".into(),
                    size: 10_000,
                    size_formatted: "10 KB".into(),
                    modified: "2024-01-01".into(),
                },
                PresetFile {
                    name: "snare.fxp".into(),
                    path: "/p/snare.fxp".into(),
                    directory: "/p".into(),
                    format: "FXP".into(),
                    size: 5000,
                    size_formatted: "5 KB".into(),
                    modified: "2024-01-01".into(),
                },
            ],
            roots: vec!["/p".into()],
        };
        db.save_preset_scan(&snap).unwrap();

        let res = db
            .query_presets(Some("lead"), None, "size", false, false, 0, 100)
            .unwrap();
        assert_eq!(res.total_count, 2);
        let names: HashSet<&str> = res.presets.iter().map(|p| p.name.as_str()).collect();
        assert!(names.contains("big_lead.fxp") && names.contains("small_lead.fxp"));

        let by_size = db
            .query_presets(None, None, "size", false, false, 0, 100)
            .unwrap();
        assert_eq!(by_size.presets[0].name, "big_lead.fxp");
        assert_eq!(by_size.presets[0].size, 10_000);
    }

    #[test]
    fn test_query_presets_total_unfiltered_empty_db() {
        let db = test_db();
        let res = db
            .query_presets(None, None, "name", true, false, 0, 100)
            .unwrap();
        assert_eq!(res.total_count, 0);
        assert_eq!(res.total_unfiltered, 0);
    }

    // ── Multi-scan semantics ──
    //
    // Each new scan inserts rows with a fresh scan_id (tables accumulate rows across history).
    // Default UI queries use the **library** (one row per `path` — audio: `audio_library`; DAW:
    // `daw_library`; PDF/MIDI/presets: `pdf_library` / `midi_library` / `preset_library`; plugins:
    // `plugin_library`),
    // not “latest scan only” and not raw `COUNT(*)` of all rows.

    fn daw_snap(id: &str, ts: &str, projects: Vec<DawProject>) -> DawScanSnapshot {
        let mut daw_counts = HashMap::new();
        for p in &projects {
            *daw_counts.entry(p.daw.clone()).or_insert(0usize) += 1;
        }
        let total_bytes = projects.iter().map(|p| p.size).sum();
        DawScanSnapshot {
            id: id.into(),
            timestamp: ts.into(),
            project_count: projects.len(),
            total_bytes,
            daw_counts,
            projects,
            roots: vec!["/music".into()],
        }
    }

    /// `get_latest_*_scan` each run `ORDER BY timestamp DESC` then hydrate via `get_*_detail`.
    #[test]
    fn test_get_latest_plugin_audio_daw_preset_scan_return_newest_timestamp() {
        let db = test_db();

        db.save_plugin_scan(&ScanSnapshot {
            id: "pl-old".into(),
            timestamp: "2024-01-01T00:00:00".into(),
            plugin_count: 1,
            plugins: vec![plugin_info("OldPlug", "VST3", "Xfer")],
            directories: vec!["/vst".into()],
            roots: vec!["/vst".into()],
        })
        .unwrap();
        db.save_plugin_scan(&ScanSnapshot {
            id: "pl-new".into(),
            timestamp: "2024-06-01T00:00:00".into(),
            plugin_count: 1,
            plugins: vec![plugin_info("NewPlug", "VST3", "Xfer")],
            directories: vec!["/vst".into()],
            roots: vec!["/vst".into()],
        })
        .unwrap();
        assert_eq!(db.get_latest_plugin_scan().unwrap().unwrap().id, "pl-new");

        let mut fc = HashMap::new();
        fc.insert("WAV".into(), 1);
        db.save_scan("au-old", "2024-01-01T00:00:00", 1, 100, &fc, &[])
            .unwrap();
        db.insert_audio_batch("au-old", &[sample("a.wav", "/a.wav", "WAV", 100)])
            .unwrap();
        db.save_scan("au-new", "2024-06-01T00:00:00", 1, 200, &fc, &[])
            .unwrap();
        db.insert_audio_batch("au-new", &[sample("b.wav", "/b.wav", "WAV", 200)])
            .unwrap();
        assert_eq!(db.get_latest_audio_scan().unwrap().unwrap().id, "au-new");

        db.save_daw_scan(&daw_snap(
            "daw-old",
            "2024-01-01T00:00:00",
            vec![daw_project("old.als", "Ableton")],
        ))
        .unwrap();
        db.save_daw_scan(&daw_snap(
            "daw-new",
            "2024-06-01T00:00:00",
            vec![daw_project("new.als", "Ableton")],
        ))
        .unwrap();
        assert_eq!(db.get_latest_daw_scan().unwrap().unwrap().id, "daw-new");

        let mut pfc = HashMap::new();
        pfc.insert("FXP".into(), 1);
        db.save_preset_scan(&PresetScanSnapshot {
            id: "pr-old".into(),
            timestamp: "2024-01-01T00:00:00".into(),
            preset_count: 1,
            total_bytes: 10,
            format_counts: pfc.clone(),
            presets: vec![preset_file("old.fxp", "FXP")],
            roots: vec![],
        })
        .unwrap();
        db.save_preset_scan(&PresetScanSnapshot {
            id: "pr-new".into(),
            timestamp: "2024-06-01T00:00:00".into(),
            preset_count: 1,
            total_bytes: 20,
            format_counts: pfc,
            presets: vec![preset_file("new.fxp", "FXP")],
            roots: vec![],
        })
        .unwrap();
        assert_eq!(db.get_latest_preset_scan().unwrap().unwrap().id, "pr-new");
    }

    #[test]
    fn test_delete_daw_scan_removes_rows_and_get_latest_falls_back() {
        let db = test_db();
        db.save_daw_scan(&daw_snap(
            "daw-old",
            "2024-01-01T00:00:00",
            vec![daw_project("old.als", "Ableton")],
        ))
        .unwrap();
        db.save_daw_scan(&daw_snap(
            "daw-new",
            "2024-06-01T00:00:00",
            vec![daw_project("new.als", "Ableton")],
        ))
        .unwrap();
        assert_eq!(db.get_latest_daw_scan().unwrap().unwrap().id, "daw-new");

        db.delete_daw_scan("daw-new").unwrap();

        assert!(db.get_daw_scan_detail("daw-new").is_err());
        let latest = db.get_latest_daw_scan().unwrap().unwrap();
        assert_eq!(latest.id, "daw-old");
        assert_eq!(latest.projects[0].name, "old.als");
    }

    #[test]
    fn test_clear_daw_history_removes_all_daw_scans() {
        let db = test_db();
        db.save_daw_scan(&daw_snap(
            "daw1",
            "2024-06-01T00:00:00",
            vec![daw_project("x.als", "Ableton")],
        ))
        .unwrap();
        db.clear_daw_history().unwrap();
        assert!(db.get_latest_daw_scan().unwrap().is_none());
        assert!(db.get_daw_scans().unwrap().is_empty());
    }

    #[test]
    fn test_clear_preset_history_removes_all_preset_scans() {
        let db = test_db();
        let mut fc = HashMap::new();
        fc.insert("FXP".into(), 1);
        db.save_preset_scan(&PresetScanSnapshot {
            id: "pr-clear-1".into(),
            timestamp: "2024-06-01T00:00:00".into(),
            preset_count: 1,
            total_bytes: 1000,
            format_counts: fc,
            presets: vec![preset_file("x.fxp", "FXP")],
            roots: vec![],
        })
        .unwrap();
        db.clear_preset_history().unwrap();
        assert!(db.get_latest_preset_scan().unwrap().is_none());
        assert!(db.get_preset_scans().unwrap().is_empty());
    }

    #[test]
    fn test_delete_audio_scan_removes_samples_and_get_latest_falls_back() {
        let db = test_db();
        let mut fc = HashMap::new();
        fc.insert("WAV".into(), 1);
        db.save_scan("au-old", "2024-01-01T00:00:00", 1, 100, &fc, &[])
            .unwrap();
        db.insert_audio_batch("au-old", &[sample("a.wav", "/a.wav", "WAV", 100)])
            .unwrap();
        db.save_scan("au-new", "2024-06-01T00:00:00", 1, 200, &fc, &[])
            .unwrap();
        db.insert_audio_batch("au-new", &[sample("b.wav", "/b.wav", "WAV", 200)])
            .unwrap();
        assert_eq!(db.get_latest_audio_scan().unwrap().unwrap().id, "au-new");

        db.delete_audio_scan("au-new").unwrap();

        assert!(db.get_audio_scan_detail("au-new").is_err());
        let latest = db.get_latest_audio_scan().unwrap().unwrap();
        assert_eq!(latest.id, "au-old");
        assert_eq!(latest.samples[0].name, "a.wav");
    }

    #[test]
    fn test_clear_audio_history_removes_all_audio_scans() {
        let db = test_db();
        let mut fc = HashMap::new();
        fc.insert("WAV".into(), 1);
        db.save_scan("s1", "2024-06-01T00:00:00", 1, 100, &fc, &[])
            .unwrap();
        db.insert_audio_batch("s1", &[sample("x.wav", "/x.wav", "WAV", 100)])
            .unwrap();
        db.clear_audio_history().unwrap();
        assert!(db.get_latest_audio_scan().unwrap().is_none());
        assert!(db.list_scans().unwrap().is_empty());
    }

    #[test]
    fn test_latest_scan_id_returns_newest_audio_scan() {
        let db = test_db();
        let mut fc = HashMap::new();
        fc.insert("WAV".into(), 1);
        db.save_scan("a-old", "2024-01-01T00:00:00", 1, 100, &fc, &[])
            .unwrap();
        db.insert_audio_batch("a-old", &[sample("a.wav", "/a.wav", "WAV", 100)])
            .unwrap();
        db.save_scan("a-new", "2024-06-01T00:00:00", 1, 200, &fc, &[])
            .unwrap();
        db.insert_audio_batch("a-new", &[sample("b.wav", "/b.wav", "WAV", 200)])
            .unwrap();
        assert_eq!(db.latest_scan_id().unwrap().as_deref(), Some("a-new"));
    }

    #[test]
    fn test_prune_old_scans_drops_oldest_audio_beyond_keep() {
        let db = test_db();
        let mut fc = HashMap::new();
        fc.insert("WAV".into(), 1);
        for (id, ts, name) in [
            ("s1", "2024-01-01T00:00:00", "n1.wav"),
            ("s2", "2024-02-01T00:00:00", "n2.wav"),
            ("s3", "2024-03-01T00:00:00", "n3.wav"),
            ("s4", "2024-04-01T00:00:00", "n4.wav"),
        ] {
            db.save_scan(id, ts, 1, 100, &fc, &[]).unwrap();
            db.insert_audio_batch(id, &[sample(name, &format!("/{name}"), "WAV", 100)])
                .unwrap();
        }
        db.prune_old_scans(2);
        let scans = db.list_scans().unwrap();
        assert_eq!(scans.len(), 2);
        assert_eq!(scans[0].id, "s4");
        assert_eq!(scans[1].id, "s3");
        assert!(db.get_audio_scan_detail("s1").is_err());
        assert!(db.get_audio_scan_detail("s2").is_err());
        assert!(db.get_audio_scan_detail("s3").is_ok());
        assert!(db.get_audio_scan_detail("s4").is_ok());
    }

    #[test]
    fn test_save_audio_scan_full_roundtrip_and_get_audio_scans_list() {
        let db = test_db();
        let mut fc = HashMap::new();
        fc.insert("WAV".into(), 1);
        let roots = vec!["/Music/Samples".into()];
        let snap = AudioScanSnapshot {
            id: "full-1".into(),
            timestamp: "2024-05-01T12:00:00".into(),
            sample_count: 1,
            total_bytes: 100,
            format_counts: fc.clone(),
            samples: vec![sample("kick.wav", "/x/kick.wav", "WAV", 100)],
            roots: roots.clone(),
        };
        db.save_audio_scan_full(&snap).unwrap();

        let list = db.get_audio_scans_list().unwrap();
        assert_eq!(list.len(), 1);
        let row = &list[0];
        assert_eq!(row["id"].as_str(), Some("full-1"));
        assert_eq!(row["sampleCount"].as_u64(), Some(1));
        assert_eq!(row["totalBytes"].as_u64(), Some(100));

        let detail = db.get_audio_scan_detail("full-1").unwrap();
        assert_eq!(detail.id, "full-1");
        assert_eq!(detail.samples.len(), 1);
        assert_eq!(detail.samples[0].name, "kick.wav");
        assert_eq!(detail.roots, roots);
        assert_eq!(detail.format_counts.get("WAV"), Some(&1usize));
    }

    #[test]
    fn test_migrate_from_json_imports_audio_scan_when_no_prior_scans() {
        let _lock = MIGRATE_JSON_TEST_LOCK.lock().unwrap();
        let tmp = std::env::temp_dir().join(format!(
            "ah_db_migrate_json_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        history::set_test_data_dir_path(tmp.clone());

        let json = r#"{"scans":[{"id":"json-mig-1","timestamp":"2024-01-01T00:00:00","sampleCount":1,"totalBytes":100,"formatCounts":{"WAV":1},"samples":[{"name":"x.wav","path":"/a/x.wav","directory":"/a","format":"WAV","size":100,"sizeFormatted":"100 B","modified":"2024-01-01"}],"roots":["/root"]}]}"#;
        std::fs::write(tmp.join("audio-scan-history.json"), json).unwrap();

        let db = test_db();
        let migrated = db.migrate_from_json().expect("migrate");
        assert!(migrated >= 1, "expected migrated sample count >= 1");

        let latest = db.get_latest_audio_scan().unwrap().expect("scan");
        assert_eq!(latest.id, "json-mig-1");
        assert_eq!(latest.samples.len(), 1);
        assert_eq!(latest.samples[0].name, "x.wav");
        assert_eq!(latest.roots, vec!["/root".to_string()]);

        assert_eq!(
            db.migrate_from_json().unwrap(),
            0,
            "second call must no-op once any scan table has rows"
        );

        history::clear_test_data_dir_path();
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_migrate_from_json_imports_plugin_scan_when_no_prior_scans() {
        let _lock = MIGRATE_JSON_TEST_LOCK.lock().unwrap();
        let tmp = std::env::temp_dir().join(format!(
            "ah_db_migrate_plugin_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        history::set_test_data_dir_path(tmp.clone());

        let json = r#"{"scans":[{"id":"pl-mig-1","timestamp":"2024-01-01T00:00:00","pluginCount":1,"plugins":[{"name":"TestPlug","path":"/p/Test.vst3","type":"VST3","version":"1.0","manufacturer":"Co","manufacturerUrl":null,"size":"1 KB","sizeBytes":1024,"modified":"2024-01-01","architectures":["ARM64"]}],"directories":["/VST"],"roots":[]}]}"#;
        std::fs::write(tmp.join("scan-history.json"), json).unwrap();

        let db = test_db();
        let migrated = db.migrate_from_json().expect("migrate");
        assert!(migrated >= 1, "expected at least one migrated row");

        let latest = db.get_latest_plugin_scan().unwrap().expect("plugin scan");
        assert_eq!(latest.id, "pl-mig-1");
        assert_eq!(latest.plugins.len(), 1);
        assert_eq!(latest.plugins[0].name, "TestPlug");
        assert_eq!(latest.plugins[0].path, "/p/Test.vst3");

        assert_eq!(db.migrate_from_json().unwrap(), 0);

        history::clear_test_data_dir_path();
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_migrate_from_json_returns_zero_when_scans_already_exist() {
        let _lock = MIGRATE_JSON_TEST_LOCK.lock().unwrap();
        let tmp = std::env::temp_dir().join(format!(
            "ah_db_migrate_skip_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        history::set_test_data_dir_path(tmp.clone());

        let json = r#"{"scans":[{"id":"json-mig-2","timestamp":"2024-02-01T00:00:00","sampleCount":1,"totalBytes":50,"formatCounts":{"WAV":1},"samples":[{"name":"ignore.wav","path":"/i/ignore.wav","directory":"/i","format":"WAV","size":50,"sizeFormatted":"50 B","modified":"2024-01-01"}],"roots":[]}]}"#;
        std::fs::write(tmp.join("audio-scan-history.json"), json).unwrap();

        let db = test_db();
        let mut fc = HashMap::new();
        fc.insert("WAV".into(), 1);
        db.save_scan("existing", "2024-01-01T00:00:00", 1, 100, &fc, &[])
            .unwrap();
        db.insert_audio_batch("existing", &[sample("a.wav", "/a.wav", "WAV", 100)])
            .unwrap();

        assert_eq!(
            db.migrate_from_json().unwrap(),
            0,
            "must skip JSON import when DB already has scan rows"
        );

        let latest = db.get_latest_audio_scan().unwrap().expect("scan");
        assert_eq!(latest.id, "existing");
        assert_eq!(latest.samples[0].name, "a.wav");

        history::clear_test_data_dir_path();
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_query_daw_multi_scan_library_unions_distinct_paths() {
        let db = test_db();
        // First (older) scan with 3 projects
        db.save_daw_scan(&daw_snap(
            "ds-old",
            "2024-01-01T00:00:00",
            vec![
                daw_project("old1.als", "Ableton"),
                daw_project("old2.als", "Ableton"),
                daw_project("old3.als", "Ableton"),
            ],
        ))
        .unwrap();
        // Second (newer) scan with 2 projects
        db.save_daw_scan(&daw_snap(
            "ds-new",
            "2024-06-01T00:00:00",
            vec![
                daw_project("new1.als", "Ableton"),
                daw_project("new2.als", "Ableton"),
            ],
        ))
        .unwrap();

        let res = db
            .query_daw(None, None, "name", true, false, 0, 100)
            .unwrap();
        assert_eq!(res.total_unfiltered, 5, "library = union of distinct paths");
        assert_eq!(res.total_count, 5);
        assert_eq!(res.projects.len(), 5);
        let names: Vec<_> = res.projects.iter().map(|p| p.name.as_str()).collect();
        assert!(names.contains(&"old1.als"));
        assert!(names.contains(&"new2.als"));
    }

    #[test]
    fn test_query_daw_empty_scan_does_not_hide_library() {
        // A later empty DAW scan (no project rows) must not make the library query return zero
        // when prior scans already inserted projects (rows remain keyed by older scan_ids).
        let db = test_db();
        db.save_daw_scan(&daw_snap(
            "ds-real",
            "2024-01-01T00:00:00",
            vec![daw_project("only.als", "Ableton")],
        ))
        .unwrap();
        // A subsequent empty scan (user hit Stop immediately, or nothing found)
        db.save_daw_scan(&daw_snap("ds-empty", "2024-12-01T00:00:00", vec![]))
            .unwrap();

        let res = db
            .query_daw(None, None, "name", true, false, 0, 100)
            .unwrap();
        assert_eq!(
            res.total_unfiltered, 1,
            "empty scans with project_count=0 must not hide existing library projects"
        );
        assert_eq!(res.projects.len(), 1);
        assert_eq!(res.projects[0].name, "only.als");
    }

    #[test]
    fn test_query_daw_total_unfiltered_stable_across_pagination() {
        let db = test_db();
        let projects: Vec<_> = (0..25)
            .map(|i| daw_project(&format!("p{i:02}.als"), "Ableton"))
            .collect();
        db.save_daw_scan(&daw_snap("ds-page", "2024-06-01T00:00:00", projects))
            .unwrap();

        let p1 = db
            .query_daw(None, None, "name", true, false, 0, 10)
            .unwrap();
        let p2 = db
            .query_daw(None, None, "name", true, false, 10, 10)
            .unwrap();
        let p3 = db
            .query_daw(None, None, "name", true, false, 20, 10)
            .unwrap();

        assert_eq!(p1.total_unfiltered, 25);
        assert_eq!(p2.total_unfiltered, 25);
        assert_eq!(p3.total_unfiltered, 25);
        assert_eq!(p1.total_count, 25);
        assert_eq!(p1.projects.len(), 10);
        assert_eq!(p2.projects.len(), 10);
        assert_eq!(p3.projects.len(), 5);
    }

    #[test]
    fn test_query_daw_combined_search_and_filter() {
        let db = test_db();
        db.save_daw_scan(&daw_snap(
            "ds-combo",
            "2024-06-01T00:00:00",
            vec![
                daw_project("bass.als", "Ableton"),
                daw_project("drums.als", "Ableton"),
                daw_project("bass.logicx", "Logic"),
                daw_project("mix.logicx", "Logic"),
            ],
        ))
        .unwrap();

        // search="bass" + daw_filter="Ableton" → 1 match, unfiltered stays 4
        let res = db
            .query_daw(Some("bass"), Some("Ableton"), "name", true, false, 0, 100)
            .unwrap();
        assert_eq!(res.total_count, 1);
        assert_eq!(res.total_unfiltered, 4);
        assert_eq!(res.projects.len(), 1);
        assert_eq!(res.projects[0].name, "bass.als");
    }

    #[test]
    fn test_query_daw_comma_separated_filter_unfiltered_stable() {
        let db = test_db();
        db.save_daw_scan(&daw_snap(
            "ds-multi",
            "2024-06-01T00:00:00",
            vec![
                daw_project("a.als", "Ableton"),
                daw_project("b.logicx", "Logic"),
                daw_project("c.flp", "FL Studio"),
                daw_project("d.rpp", "REAPER"),
            ],
        ))
        .unwrap();

        let res = db
            .query_daw(None, Some("Ableton,Logic"), "name", true, false, 0, 100)
            .unwrap();
        assert_eq!(res.total_count, 2);
        assert_eq!(res.total_unfiltered, 4);
        assert_eq!(
            res.projects.len(),
            2,
            "main SELECT must return matching rows"
        );
        assert!(
            res.projects
                .iter()
                .all(|p| p.daw == "Ableton" || p.daw == "Logic")
        );
    }

    #[test]
    fn test_query_daw_comma_filter_with_pagination() {
        // Ensures LIMIT is respected when comma-separated filter is combined with offset/limit.
        let db = test_db();
        db.save_daw_scan(&daw_snap(
            "ds-comma-page",
            "2024-06-01T00:00:00",
            (0..12)
                .map(|i| {
                    let daw = if i % 2 == 0 { "Ableton" } else { "Logic" };
                    daw_project(&format!("p{i:02}.als"), daw)
                })
                .collect(),
        ))
        .unwrap();

        let res = db
            .query_daw(None, Some("Ableton,Logic"), "name", true, false, 0, 5)
            .unwrap();
        assert_eq!(res.total_count, 12);
        assert_eq!(res.projects.len(), 5, "LIMIT=5 must be respected");
    }

    fn preset_snap(id: &str, ts: &str, presets: Vec<PresetFile>) -> PresetScanSnapshot {
        let mut format_counts = HashMap::new();
        for p in &presets {
            *format_counts.entry(p.format.clone()).or_insert(0usize) += 1;
        }
        let total_bytes = presets.iter().map(|p| p.size).sum();
        PresetScanSnapshot {
            id: id.into(),
            timestamp: ts.into(),
            preset_count: presets.len(),
            total_bytes,
            format_counts,
            presets,
            roots: vec!["/presets".into()],
        }
    }

    #[test]
    fn test_query_presets_multi_scan_library_unions_distinct_paths() {
        let db = test_db();
        db.save_preset_scan(&preset_snap(
            "pr-old",
            "2024-01-01T00:00:00",
            vec![
                preset_file("a.fxp", "FXP"),
                preset_file("b.fxp", "FXP"),
                preset_file("c.fxp", "FXP"),
            ],
        ))
        .unwrap();
        db.save_preset_scan(&preset_snap(
            "pr-new",
            "2024-06-01T00:00:00",
            vec![preset_file("x.fxp", "FXP")],
        ))
        .unwrap();

        let res = db
            .query_presets(None, None, "name", true, false, 0, 100)
            .unwrap();
        assert_eq!(res.total_unfiltered, 4);
        assert_eq!(res.presets.len(), 4);
        assert_eq!(res.presets[0].name, "a.fxp");
        assert_eq!(res.presets[3].name, "x.fxp");
    }

    #[test]
    fn test_query_presets_midi_filter_still_excluded() {
        // Even if the user explicitly format-filters for MID, the `NOT IN ('MID','MIDI')`
        // clause must still exclude them — MIDI belongs in its own tab. The filtered
        // AND unfiltered counts for presets should both be 0 in this case.
        let db = test_db();
        db.save_preset_scan(&preset_snap(
            "pr-midi",
            "2024-06-01T00:00:00",
            vec![
                preset_file("song.mid", "MID"),
                preset_file("beat.midi", "MIDI"),
                preset_file("lead.fxp", "FXP"),
            ],
        ))
        .unwrap();

        // Explicit MID filter still returns 0 filtered results
        let res = db
            .query_presets(None, Some("MID"), "name", true, false, 0, 100)
            .unwrap();
        assert_eq!(res.total_count, 0);
        // Unfiltered excludes MIDI regardless of format_filter
        assert_eq!(res.total_unfiltered, 1);
    }

    #[test]
    fn test_query_presets_comma_separated_filter_unfiltered_stable() {
        // Regression: comma-separated format_filter was binding the raw string to the
        // LIMIT placeholder, causing "column index out of range" on the main SELECT.
        let db = test_db();
        db.save_preset_scan(&preset_snap(
            "pr-multi-fmt",
            "2024-06-01T00:00:00",
            vec![
                preset_file("a.fxp", "FXP"),
                preset_file("b.h2p", "H2P"),
                preset_file("c.nmsv", "NMSV"),
                preset_file("d.fxp", "FXP"),
            ],
        ))
        .unwrap();

        let res = db
            .query_presets(None, Some("FXP,H2P"), "name", true, false, 0, 100)
            .unwrap();
        assert_eq!(res.total_count, 3);
        assert_eq!(res.total_unfiltered, 4);
        assert_eq!(res.presets.len(), 3);
        assert!(
            res.presets
                .iter()
                .all(|p| p.format == "FXP" || p.format == "H2P")
        );
    }

    #[test]
    fn test_query_presets_total_unfiltered_stable_across_pagination() {
        let db = test_db();
        let presets: Vec<_> = (0..30)
            .map(|i| preset_file(&format!("p{i:02}.fxp"), "FXP"))
            .collect();
        db.save_preset_scan(&preset_snap("pr-page", "2024-06-01T00:00:00", presets))
            .unwrap();

        let p1 = db
            .query_presets(None, None, "name", true, false, 0, 10)
            .unwrap();
        let p2 = db
            .query_presets(None, None, "name", true, false, 10, 10)
            .unwrap();
        let p3 = db
            .query_presets(None, None, "name", true, false, 25, 10)
            .unwrap();

        assert_eq!(p1.total_unfiltered, 30);
        assert_eq!(p2.total_unfiltered, 30);
        assert_eq!(p3.total_unfiltered, 30);
        assert_eq!(p1.presets.len(), 10);
        assert_eq!(p2.presets.len(), 10);
        assert_eq!(p3.presets.len(), 5);
    }

    fn plugin_snap(id: &str, ts: &str, plugins: Vec<PluginInfo>) -> ScanSnapshot {
        ScanSnapshot {
            id: id.into(),
            timestamp: ts.into(),
            plugin_count: plugins.len(),
            plugins,
            directories: vec!["/vst".into()],
            roots: vec!["/vst".into()],
        }
    }

    #[test]
    fn test_query_plugins_multi_scan_library_unions_distinct_paths() {
        let db = test_db();
        db.save_plugin_scan(&plugin_snap(
            "ps-old",
            "2024-01-01T00:00:00",
            vec![
                plugin_info("Old1", "VST3", "Acme"),
                plugin_info("Old2", "VST3", "Acme"),
                plugin_info("Old3", "VST3", "Acme"),
            ],
        ))
        .unwrap();
        db.save_plugin_scan(&plugin_snap(
            "ps-new",
            "2024-06-01T00:00:00",
            vec![plugin_info("New1", "VST3", "Acme")],
        ))
        .unwrap();

        let res = db
            .query_plugins(None, None, None, "name", true, false, 0, 100)
            .unwrap();
        assert_eq!(res.total_unfiltered, 4);
        assert_eq!(res.plugins.len(), 4);
        assert_eq!(res.plugins[0].name, "New1");
        assert_eq!(res.plugins[3].name, "Old3");
    }

    #[test]
    fn test_query_plugins_multi_type_returns_rows_not_empty() {
        // Regression: comma-separated type_filter was over-incrementing bind_offset,
        // binding `limit` to a wrong placeholder slot so the real LIMIT slot was NULL.
        // Result: main SELECT returned 0 rows even though the IN clause had matches.
        let db = test_db();
        db.save_plugin_scan(&plugin_snap(
            "ps-multi-bind",
            "2024-06-01T00:00:00",
            vec![
                plugin_info("A", "VST3", "X"),
                plugin_info("B", "VST2", "X"),
                plugin_info("C", "AU", "X"),
                plugin_info("D", "VST3", "X"),
                plugin_info("E", "AU", "X"),
            ],
        ))
        .unwrap();

        let res = db
            .query_plugins(None, Some("VST3,AU"), None, "name", true, false, 0, 100)
            .unwrap();
        assert_eq!(res.total_count, 4);
        assert_eq!(
            res.plugins.len(),
            4,
            "main SELECT must return the 4 matching rows, not 0"
        );
        assert!(
            res.plugins
                .iter()
                .all(|p| p.plugin_type == "VST3" || p.plugin_type == "AU")
        );
    }

    #[test]
    fn test_query_plugins_multi_type_with_search_and_pagination() {
        // Compound scenario: search + comma-filter + offset — exercises all three
        // bind-offset branches simultaneously.
        let db = test_db();
        db.save_plugin_scan(&plugin_snap(
            "ps-compound",
            "2024-06-01T00:00:00",
            vec![
                plugin_info("alpha", "VST3", "X"),
                plugin_info("alpen", "VST3", "X"),
                plugin_info("alto", "AU", "X"),
                plugin_info("bravo", "VST3", "X"),
                plugin_info("alps", "AU", "X"),
            ],
        ))
        .unwrap();

        let res = db
            .query_plugins(Some("al"), Some("VST3,AU"), None, "name", true, false, 0, 2)
            .unwrap();
        assert_eq!(res.total_count, 4); // alpha, alpen, alto, alps
        assert_eq!(res.plugins.len(), 2, "LIMIT must be respected");
    }

    #[test]
    fn test_query_plugins_type_filter_multi_type() {
        let db = test_db();
        db.save_plugin_scan(&plugin_snap(
            "ps-types",
            "2024-06-01T00:00:00",
            vec![
                plugin_info("A", "VST3", "X"),
                plugin_info("B", "VST2", "X"),
                plugin_info("C", "AU", "X"),
                plugin_info("D", "VST3", "X"),
            ],
        ))
        .unwrap();

        let res = db
            .query_plugins(None, Some("VST3"), None, "name", true, false, 0, 100)
            .unwrap();
        assert_eq!(res.total_count, 2);
        assert_eq!(res.total_unfiltered, 4);

        let res = db
            .query_plugins(None, Some("VST3,AU"), None, "name", true, false, 0, 100)
            .unwrap();
        assert_eq!(res.total_count, 3);
        assert_eq!(res.total_unfiltered, 4);
    }

    #[test]
    fn test_query_plugins_status_filter_kvr_join() {
        use crate::history::KvrCacheUpdateEntry;
        let db = test_db();
        db.save_plugin_scan(&plugin_snap(
            "ps-kvr-status",
            "2024-06-01T00:00:00",
            vec![
                plugin_info("HasUpdate", "VST3", "Mfg"),
                plugin_info("Current", "VST3", "Mfg"),
                plugin_info("UnknownKvr", "VST3", "Mfg"),
                plugin_info("NoCache", "VST3", "Mfg"),
            ],
        ))
        .unwrap();
        db.update_kvr_cache(&[
            KvrCacheUpdateEntry {
                key: "mfg|||hasupdate".into(),
                kvr_url: Some("u".into()),
                update_url: None,
                latest_version: Some("2".into()),
                has_update: Some(true),
                source: Some("kvr".into()),
            },
            KvrCacheUpdateEntry {
                key: "mfg|||current".into(),
                kvr_url: Some("u".into()),
                update_url: None,
                latest_version: Some("1.0".into()),
                has_update: Some(false),
                source: Some("kvr".into()),
            },
            KvrCacheUpdateEntry {
                key: "mfg|||unknownkvr".into(),
                kvr_url: None,
                update_url: None,
                latest_version: None,
                has_update: Some(false),
                source: Some("not-found".into()),
            },
        ])
        .unwrap();

        let r_up = db
            .query_plugins(None, None, Some("update"), "name", true, false, 0, 100)
            .unwrap();
        assert_eq!(r_up.total_count, 1);
        assert_eq!(r_up.plugins[0].name, "HasUpdate");

        let r_cur = db
            .query_plugins(None, None, Some("current"), "name", true, false, 0, 100)
            .unwrap();
        assert_eq!(r_cur.total_count, 1);
        assert_eq!(r_cur.plugins[0].name, "Current");

        let r_unk = db
            .query_plugins(None, None, Some("unknown"), "name", true, false, 0, 100)
            .unwrap();
        assert_eq!(r_unk.total_count, 2);
        let names: Vec<_> = r_unk.plugins.iter().map(|p| p.name.as_str()).collect();
        assert!(names.contains(&"UnknownKvr"));
        assert!(names.contains(&"NoCache"));

        let r_all = db
            .query_plugins(
                None,
                None,
                Some("update,current,unknown"),
                "name",
                true,
                false,
                0,
                100,
            )
            .unwrap();
        assert_eq!(r_all.total_count, 4);
    }

    #[test]
    fn test_query_plugins_total_unfiltered_stable_across_pagination() {
        let db = test_db();
        let plugins: Vec<_> = (0..40)
            .map(|i| plugin_info(&format!("plug{i:02}"), "VST3", "X"))
            .collect();
        db.save_plugin_scan(&plugin_snap("ps-page", "2024-06-01T00:00:00", plugins))
            .unwrap();

        let p1 = db
            .query_plugins(None, None, None, "name", true, false, 0, 15)
            .unwrap();
        let p2 = db
            .query_plugins(None, None, None, "name", true, false, 15, 15)
            .unwrap();

        assert_eq!(p1.total_unfiltered, 40);
        assert_eq!(p2.total_unfiltered, 40);
        assert_eq!(p1.plugins.len(), 15);
        assert_eq!(p2.plugins.len(), 15);
    }

    #[test]
    fn test_query_plugins_search_by_manufacturer() {
        let db = test_db();
        db.save_plugin_scan(&plugin_snap(
            "ps-mfg",
            "2024-06-01T00:00:00",
            vec![
                plugin_info("Serum", "VST3", "Xfer"),
                plugin_info("Serum2", "VST3", "Xfer"),
                plugin_info("Vital", "VST3", "Matt"),
            ],
        ))
        .unwrap();

        let res = db
            .query_plugins(Some("Xfer"), None, None, "name", true, false, 0, 100)
            .unwrap();
        assert_eq!(res.total_count, 2);
        assert_eq!(res.total_unfiltered, 3);
    }

    // ── Unfiltered aggregate stats ──
    // These power the stats sections in the DAW/preset tabs and MUST be
    // independent of any table filter the user has applied.

    #[test]
    fn test_daw_stats_returns_library_aggregates() {
        let db = test_db();
        db.save_daw_scan(&daw_snap(
            "ds-stats",
            "2024-06-01T00:00:00",
            vec![
                daw_project("a.als", "Ableton Live"),
                daw_project("b.als", "Ableton Live"),
                daw_project("c.logicx", "Logic Pro"),
                daw_project("d.flp", "FL Studio"),
            ],
        ))
        .unwrap();

        let stats = db.daw_stats(None).unwrap();
        assert_eq!(stats.project_count, 4);
        assert_eq!(stats.total_bytes, 4000); // 4 × 1000 from daw_project helper
        assert_eq!(stats.daw_counts["Ableton Live"], 2);
        assert_eq!(stats.daw_counts["Logic Pro"], 1);
        assert_eq!(stats.daw_counts["FL Studio"], 1);
    }

    #[test]
    fn test_daw_stats_empty_db() {
        let db = test_db();
        let stats = db.daw_stats(None).unwrap();
        assert_eq!(stats.project_count, 0);
        assert_eq!(stats.total_bytes, 0);
        assert!(stats.daw_counts.is_empty());
    }

    #[test]
    fn test_daw_stats_multi_scan_library_unions_distinct_paths() {
        let db = test_db();
        db.save_daw_scan(&daw_snap(
            "ds-old",
            "2024-01-01T00:00:00",
            vec![
                daw_project("old1.als", "Ableton"),
                daw_project("old2.als", "Ableton"),
                daw_project("old3.als", "Ableton"),
            ],
        ))
        .unwrap();
        db.save_daw_scan(&daw_snap(
            "ds-new",
            "2024-06-01T00:00:00",
            vec![daw_project("new.logicx", "Logic")],
        ))
        .unwrap();

        let stats = db.daw_stats(None).unwrap();
        assert_eq!(stats.project_count, 4);
        assert_eq!(stats.daw_counts["Ableton"], 3);
        assert_eq!(stats.daw_counts["Logic"], 1);
    }

    #[test]
    fn test_daw_stats_empty_scan_ignored() {
        let db = test_db();
        db.save_daw_scan(&daw_snap(
            "ds-real",
            "2024-01-01T00:00:00",
            vec![daw_project("real.als", "Ableton")],
        ))
        .unwrap();
        db.save_daw_scan(&daw_snap("ds-empty", "2024-12-01T00:00:00", vec![]))
            .unwrap();

        let stats = db.daw_stats(None).unwrap();
        assert_eq!(
            stats.project_count, 1,
            "empty scan must not clobber real one"
        );
    }

    #[test]
    fn test_daw_stats_explicit_scan_id() {
        let db = test_db();
        db.save_daw_scan(&daw_snap(
            "ds-a",
            "2024-01-01T00:00:00",
            vec![
                daw_project("x.als", "Ableton"),
                daw_project("y.als", "Ableton"),
            ],
        ))
        .unwrap();
        db.save_daw_scan(&daw_snap(
            "ds-b",
            "2024-06-01T00:00:00",
            vec![daw_project("z.logicx", "Logic")],
        ))
        .unwrap();

        // Explicitly request older scan
        let stats = db.daw_stats(Some("ds-a")).unwrap();
        assert_eq!(stats.project_count, 2);
        assert_eq!(stats.daw_counts["Ableton"], 2);
    }

    #[test]
    fn test_preset_stats_returns_aggregates_excluding_midi() {
        let db = test_db();
        db.save_preset_scan(&preset_snap(
            "pr-stats",
            "2024-06-01T00:00:00",
            vec![
                preset_file("a.fxp", "FXP"),
                preset_file("b.fxp", "FXP"),
                preset_file("c.h2p", "H2P"),
                preset_file("song.mid", "MID"),
                preset_file("beat.midi", "MIDI"),
            ],
        ))
        .unwrap();

        let stats = db.preset_stats(None).unwrap();
        assert_eq!(stats.preset_count, 3, "MIDI must be excluded");
        assert_eq!(stats.total_bytes, 3000); // 3 × 1000, MIDI sizes excluded
        assert_eq!(stats.format_counts["FXP"], 2);
        assert_eq!(stats.format_counts["H2P"], 1);
        assert!(stats.format_counts.get("MID").is_none());
        assert!(stats.format_counts.get("MIDI").is_none());
    }

    #[test]
    fn test_preset_stats_empty_db() {
        let db = test_db();
        let stats = db.preset_stats(None).unwrap();
        assert_eq!(stats.preset_count, 0);
        assert_eq!(stats.total_bytes, 0);
        assert!(stats.format_counts.is_empty());
    }

    #[test]
    fn test_preset_stats_all_midi_returns_zero() {
        // Edge case: a scan with only MIDI files should report zero presets
        // for the presets tab (MIDI lives in its own tab).
        let db = test_db();
        db.save_preset_scan(&preset_snap(
            "pr-midi-only",
            "2024-06-01T00:00:00",
            vec![preset_file("a.mid", "MID"), preset_file("b.midi", "MIDI")],
        ))
        .unwrap();

        let stats = db.preset_stats(None).unwrap();
        assert_eq!(stats.preset_count, 0);
        assert_eq!(stats.total_bytes, 0);
        assert!(stats.format_counts.is_empty());
    }

    #[test]
    fn test_preset_stats_multi_scan_library_unions_distinct_paths() {
        let db = test_db();
        db.save_preset_scan(&preset_snap(
            "pr-old",
            "2024-01-01T00:00:00",
            vec![
                preset_file("x.fxp", "FXP"),
                preset_file("y.fxp", "FXP"),
                preset_file("z.fxp", "FXP"),
            ],
        ))
        .unwrap();
        db.save_preset_scan(&preset_snap(
            "pr-new",
            "2024-06-01T00:00:00",
            vec![preset_file("a.h2p", "H2P")],
        ))
        .unwrap();

        let stats = db.preset_stats(None).unwrap();
        assert_eq!(stats.preset_count, 4);
        assert_eq!(stats.format_counts["FXP"], 3);
        assert_eq!(stats.format_counts["H2P"], 1);
    }

    #[test]
    fn test_kvr_cache_roundtrip() {
        use crate::history::KvrCacheUpdateEntry;
        let db = test_db();

        let entries = vec![
            KvrCacheUpdateEntry {
                key: "serum".into(),
                kvr_url: Some("https://kvr.com/serum".into()),
                update_url: Some("https://xfer.com/update".into()),
                latest_version: Some("1.4".into()),
                has_update: Some(true),
                source: Some("kvr".into()),
            },
            KvrCacheUpdateEntry {
                key: "vital".into(),
                kvr_url: None,
                update_url: None,
                latest_version: Some("1.6".into()),
                has_update: Some(false),
                source: None,
            },
        ];
        db.update_kvr_cache(&entries).unwrap();

        let cache = db.load_kvr_cache().unwrap();
        assert_eq!(cache.len(), 2);
        assert_eq!(
            cache["serum"].kvr_url.as_deref(),
            Some("https://kvr.com/serum")
        );
        assert!(cache["serum"].has_update);
        assert!(!cache["vital"].has_update);
        assert_eq!(cache["vital"].latest_version.as_deref(), Some("1.6"));
    }

    #[test]
    fn test_clear_all_caches() {
        let db = test_db();
        let samples = vec![sample("kick.wav", "/kick.wav", "WAV", 1000)];
        db.save_scan("s1", "2024-01-01T00:00:00", 1, 1000, &HashMap::new(), &[])
            .unwrap();
        db.insert_audio_batch("s1", &samples).unwrap();
        db.update_bpm("/kick.wav", Some(120.0)).unwrap();
        db.update_key("/kick.wav", Some("A minor")).unwrap();
        db.update_lufs("/kick.wav", Some(-14.0)).unwrap();

        // Verify analysis is set
        let analysis = db.get_analysis("/kick.wav").unwrap();
        assert_eq!(analysis["bpm"], 120.0);

        db.clear_all_caches().unwrap();

        let analysis = db.get_analysis("/kick.wav").unwrap();
        assert!(analysis.get("bpm").and_then(|v| v.as_f64()).is_none());
        assert!(analysis.get("key").and_then(|v| v.as_str()).is_none());
        assert!(analysis.get("lufs").and_then(|v| v.as_f64()).is_none());
    }

    #[test]
    fn test_clear_cache_table_bpm() {
        let db = test_db();
        let samples = vec![sample("a.wav", "/a.wav", "WAV", 100)];
        db.save_scan("s1", "2024-01-01T00:00:00", 1, 100, &HashMap::new(), &[])
            .unwrap();
        db.insert_audio_batch("s1", &samples).unwrap();
        db.update_bpm("/a.wav", Some(140.0)).unwrap();

        db.clear_cache_table("bpm").unwrap();
        let analysis = db.get_analysis("/a.wav").unwrap();
        assert!(analysis.get("bpm").and_then(|v| v.as_f64()).is_none());
    }

    #[test]
    fn test_clear_cache_table_key() {
        let db = test_db();
        let samples = vec![sample("a.wav", "/a.wav", "WAV", 100)];
        db.save_scan("s1", "2024-01-01T00:00:00", 1, 100, &HashMap::new(), &[])
            .unwrap();
        db.insert_audio_batch("s1", &samples).unwrap();
        db.update_key("/a.wav", Some("D major")).unwrap();

        db.clear_cache_table("key").unwrap();
        let analysis = db.get_analysis("/a.wav").unwrap();
        assert!(analysis.get("key").and_then(|v| v.as_str()).is_none());
    }

    #[test]
    fn test_clear_cache_table_waveform() {
        let db = test_db();
        let data = serde_json::json!({"test_path": "some_waveform_data"});
        db.write_cache("waveform-cache.json", &data).unwrap();

        let cached = db.read_cache("waveform-cache.json").unwrap();
        assert!(cached.as_object().unwrap().contains_key("test_path"));

        db.clear_cache_table("waveform").unwrap();
        let cached = db.read_cache("waveform-cache.json").unwrap();
        assert!(cached.as_object().unwrap().is_empty());
    }

    #[test]
    fn test_clear_cache_table_xref() {
        let db = test_db();
        let data = serde_json::json!({"/project.als": "[\"Serum\",\"Vital\"]"});
        db.write_cache("xref-cache.json", &data).unwrap();

        db.clear_cache_table("xref").unwrap();
        let cached = db.read_cache("xref-cache.json").unwrap();
        assert!(cached.as_object().unwrap().is_empty());
    }

    #[test]
    fn test_clear_cache_table_spectrogram() {
        let db = test_db();
        let data = serde_json::json!({"/a.wav": "spectrogram_payload"});
        db.write_cache("spectrogram-cache.json", &data).unwrap();
        assert!(
            db.read_cache("spectrogram-cache.json")
                .unwrap()
                .as_object()
                .unwrap()
                .contains_key("/a.wav")
        );
        db.clear_cache_table("spectrogram").unwrap();
        assert!(
            db.read_cache("spectrogram-cache.json")
                .unwrap()
                .as_object()
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn test_clear_cache_table_fingerprint() {
        let db = test_db();
        let data = serde_json::json!({"/sample.wav": "fpabc"});
        db.write_cache("fingerprint-cache.json", &data).unwrap();
        db.clear_cache_table("fingerprint").unwrap();
        assert!(
            db.read_cache("fingerprint-cache.json")
                .unwrap()
                .as_object()
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn test_clear_cache_table_kvr() {
        let db = test_db();
        let entries = vec![crate::history::KvrCacheUpdateEntry {
            key: "test_plugin_key".into(),
            kvr_url: Some("https://www.kvraudio.com/product/test".into()),
            update_url: None,
            latest_version: Some("2.0".into()),
            has_update: Some(true),
            source: Some("test".into()),
        }];
        db.update_kvr_cache(&entries).unwrap();
        assert_eq!(db.load_kvr_cache().unwrap().len(), 1);
        db.clear_cache_table("kvr").unwrap();
        assert!(db.load_kvr_cache().unwrap().is_empty());
    }

    #[test]
    fn test_clear_cache_table_lufs() {
        let db = test_db();
        let samples = vec![sample("a.wav", "/a.wav", "WAV", 100)];
        db.save_scan("s1", "2024-01-01T00:00:00", 1, 100, &HashMap::new(), &[])
            .unwrap();
        db.insert_audio_batch("s1", &samples).unwrap();
        db.update_lufs("/a.wav", Some(-12.0)).unwrap();
        db.clear_cache_table("lufs").unwrap();
        let analysis = db.get_analysis("/a.wav").unwrap();
        assert!(analysis.get("lufs").and_then(|v| v.as_f64()).is_none());
    }

    #[test]
    fn test_clear_cache_table_unknown() {
        let db = test_db();
        let result = db.clear_cache_table("bogus");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unknown cache"));
    }

    #[test]
    fn test_read_write_cache_waveform() {
        let db = test_db();
        let data = serde_json::json!({"/path/to/file.wav": "base64waveformdata"});
        db.write_cache("waveform-cache.json", &data).unwrap();

        let result = db.read_cache("waveform-cache.json").unwrap();
        assert_eq!(result["/path/to/file.wav"], "base64waveformdata");
    }

    #[test]
    fn test_get_waveform_cache_row_single() {
        let db = test_db();
        let peaks = serde_json::json!([{"min": -0.5, "max": 0.5}]);
        db.upsert_waveform_cache_row("/videos/movie.mp4", &peaks).unwrap();
        assert_eq!(db.get_waveform_cache_row("/videos/movie.mp4").unwrap(), Some(peaks));
        assert!(db.get_waveform_cache_row("/nosuch.mp4").unwrap().is_none());
    }

    #[test]
    fn test_read_write_cache_xref() {
        let db = test_db();
        let data = serde_json::json!({"/project.flp": "[\"Serum\"]"});
        db.write_cache("xref-cache.json", &data).unwrap();

        let result = db.read_cache("xref-cache.json").unwrap();
        let obj = result.as_object().unwrap();
        assert!(obj.contains_key("/project.flp"));
    }

    #[test]
    fn test_table_counts() {
        let db = test_db();
        let counts = db.table_counts().unwrap();
        let obj = counts.as_object().unwrap();

        // Fresh DB should have all zeros
        assert_eq!(obj["audio_samples"], 0);
        assert_eq!(obj["plugins"], 0);
        assert_eq!(obj["daw_projects"], 0);
        assert_eq!(obj["presets"], 0);
        assert_eq!(obj["kvr_cache"], 0);

        // Insert some data and verify counts change
        let samples = vec![sample("a.wav", "/a.wav", "WAV", 100)];
        db.save_scan("s1", "2024-01-01T00:00:00", 1, 100, &HashMap::new(), &[])
            .unwrap();
        db.insert_audio_batch("s1", &samples).unwrap();

        let counts = db.table_counts().unwrap();
        let obj = counts.as_object().unwrap();
        assert_eq!(obj["audio_samples"], 1);
        assert_eq!(obj["audio_samples_library"], 1);
        assert_eq!(obj["audio_scans"], 1);
    }

    #[test]
    fn test_table_counts_raw_vs_library_when_same_path_rescanned() {
        let db = test_db();
        let s = sample("x.wav", "/same/x.wav", "WAV", 100);
        db.save_scan("s1", "2024-01-01T00:00:00", 1, 100, &HashMap::new(), &[])
            .unwrap();
        db.insert_audio_batch("s1", &[s.clone()]).unwrap();
        db.save_scan("s2", "2024-01-02T00:00:00", 1, 100, &HashMap::new(), &[])
            .unwrap();
        db.insert_audio_batch("s2", &[s]).unwrap();
        let obj = db.table_counts().unwrap();
        let obj = obj.as_object().unwrap();
        assert_eq!(obj["audio_samples"], 2);
        assert_eq!(obj["audio_samples_library"], 1);
    }

    #[test]
    fn test_audio_library_sample_id_is_max_id_per_path() {
        let db = test_db();
        let s = sample("x.wav", "/same/x.wav", "WAV", 100);
        db.save_scan("s1", "2024-01-01T00:00:00", 1, 100, &HashMap::new(), &[])
            .unwrap();
        db.insert_audio_batch("s1", &[s.clone()]).unwrap();
        db.save_scan("s2", "2024-01-02T00:00:00", 1, 100, &HashMap::new(), &[])
            .unwrap();
        db.insert_audio_batch("s2", &[s]).unwrap();
        let conn = db.read_conn();
        let n: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM audio_library lib
                 WHERE lib.sample_id = (SELECT MAX(id) FROM audio_samples a WHERE a.path = lib.path)",
                [],
                |r| r.get::<_, i64>(0),
            )
            .unwrap();
        assert_eq!(n, 1);
    }

    #[test]
    fn test_plugin_library_plugin_id_is_max_id_per_path() {
        let db = test_db();
        let p = |name: &str, path: &str| PluginInfo {
            name: name.into(),
            path: path.into(),
            plugin_type: "VST3".into(),
            version: "1".into(),
            manufacturer: "m".into(),
            manufacturer_url: None,
            size: "1 B".into(),
            size_bytes: 1,
            modified: "2024-01-01".into(),
            architectures: vec![],
        };
        db.plugin_scan_parent_create("ps1", "2024-01-01T00:00:00", &[])
            .unwrap();
        db.insert_plugin_batch("ps1", &[p("a", "/same/x.vst3")])
            .unwrap();
        db.plugin_scan_parent_create("ps2", "2024-06-01T00:00:00", &[])
            .unwrap();
        db.insert_plugin_batch("ps2", &[p("a", "/same/x.vst3")])
            .unwrap();
        let conn = db.read_conn();
        let n: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM plugin_library lib
                 WHERE lib.plugin_id = (SELECT MAX(id) FROM plugins p WHERE p.path = lib.path)",
                [],
                |r| r.get::<_, i64>(0),
            )
            .unwrap();
        assert_eq!(n, 1);
    }

    #[test]
    fn test_pdf_midi_preset_library_ids_match_max_id_per_path() {
        let db = test_db();
        let pdf = |path: &str| PdfFile {
            name: "a.pdf".into(),
            path: path.into(),
            directory: "/d".into(),
            size: 1,
            size_formatted: "1 B".into(),
            modified: "2024-01-01".into(),
            ..Default::default()
        };
        db.pdf_scan_parent_create("p1", "2024-01-01T00:00:00", &[])
            .unwrap();
        db.insert_pdf_batch("p1", &[pdf("/same/x.pdf")]).unwrap();
        db.pdf_scan_parent_create("p2", "2024-01-02T00:00:00", &[])
            .unwrap();
        db.insert_pdf_batch("p2", &[pdf("/same/x.pdf")]).unwrap();
        {
            let conn = db.read_conn();
            let n: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM pdf_library lib
                 WHERE lib.pdf_id = (SELECT MAX(id) FROM pdfs p WHERE p.path = lib.path)",
                    [],
                    |r| r.get::<_, i64>(0),
                )
                .unwrap();
            assert_eq!(n, 1);
        }

        let m = |path: &str| MidiFile {
            name: "a.mid".into(),
            path: path.into(),
            directory: "/d".into(),
            format: "MID".into(),
            size: 1,
            size_formatted: "1 B".into(),
            modified: "2024-01-01".into(),
        };
        db.midi_scan_parent_create("m1", "2024-01-01T00:00:00", &[])
            .unwrap();
        db.insert_midi_batch("m1", &[m("/same/y.mid")]).unwrap();
        db.midi_scan_parent_create("m2", "2024-01-02T00:00:00", &[])
            .unwrap();
        db.insert_midi_batch("m2", &[m("/same/y.mid")]).unwrap();
        {
            let conn = db.read_conn();
            let n: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM midi_library lib
                 WHERE lib.midi_id = (SELECT MAX(id) FROM midi_files f WHERE f.path = lib.path)",
                    [],
                    |r| r.get::<_, i64>(0),
                )
                .unwrap();
            assert_eq!(n, 1);
        }

        let pr = |path: &str| PresetFile {
            name: "a.fxp".into(),
            path: path.into(),
            directory: "/d".into(),
            format: "FXP".into(),
            size: 1,
            size_formatted: "1 B".into(),
            modified: "2024-01-01".into(),
        };
        db.preset_scan_parent_create("r1", "2024-01-01T00:00:00", &[])
            .unwrap();
        db.insert_preset_batch("r1", &[pr("/same/z.fxp")]).unwrap();
        db.preset_scan_parent_create("r2", "2024-01-02T00:00:00", &[])
            .unwrap();
        db.insert_preset_batch("r2", &[pr("/same/z.fxp")]).unwrap();
        {
            let conn = db.read_conn();
            let n: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM preset_library lib
                 WHERE lib.preset_id = (SELECT MAX(id) FROM presets p WHERE p.path = lib.path)",
                    [],
                    |r| r.get::<_, i64>(0),
                )
                .unwrap();
            assert_eq!(n, 1);
        }
    }

    #[test]
    fn test_table_counts_with_plugin_and_daw_data() {
        let db = test_db();
        let snap = ScanSnapshot {
            id: "ps1".into(),
            timestamp: "2024-01-01T00:00:00".into(),
            plugin_count: 1,
            plugins: vec![PluginInfo {
                name: "Test".into(),
                path: "/test.vst3".into(),
                plugin_type: "VST3".into(),
                version: "1.0".into(),
                manufacturer: "Test Co".into(),
                manufacturer_url: None,
                size: "1 MB".into(),
                size_bytes: 1_000_000,
                modified: "2024-01-01".into(),
                architectures: vec![],
            }],
            directories: vec![],
            roots: vec![],
        };
        db.save_plugin_scan(&snap).unwrap();

        let daw_snap = DawScanSnapshot {
            id: "ds1".into(),
            timestamp: "2024-01-01T00:00:00".into(),
            project_count: 1,
            total_bytes: 1000,
            daw_counts: HashMap::new(),
            projects: vec![DawProject {
                name: "t.als".into(),
                path: "/t.als".into(),
                directory: "/".into(),
                format: "ALS".into(),
                daw: "Ableton".into(),
                size: 1000,
                size_formatted: "1 KB".into(),
                modified: "2024-01-01".into(),
            }],
            roots: vec![],
        };
        db.save_daw_scan(&daw_snap).unwrap();

        let counts = db.table_counts().unwrap();
        let obj = counts.as_object().unwrap();
        assert_eq!(obj["plugins"], 1);
        assert_eq!(obj["plugin_scans"], 1);
        assert_eq!(obj["daw_projects"], 1);
        assert_eq!(obj["daw_scans"], 1);
    }

    #[test]
    fn test_active_scan_inventory_counts_empty() {
        let db = test_db();
        let v = db.active_scan_inventory_counts().unwrap();
        assert_eq!(v["plugins"], 0);
        assert_eq!(v["audio_samples"], 0);
        assert_eq!(v["daw_projects"], 0);
        assert_eq!(v["presets"], 0);
        assert_eq!(v["pdfs"], 0);
        assert_eq!(v["midi_files"], 0);
    }

    #[test]
    fn test_active_scan_inventory_counts_presets_exclude_midi_formats() {
        let db = test_db();
        db.save_preset_scan(&preset_snap(
            "pr-midi-only",
            "2024-06-01T00:00:00",
            vec![preset_file("a.mid", "MID"), preset_file("b.midi", "MIDI")],
        ))
        .unwrap();
        let v = db.active_scan_inventory_counts().unwrap();
        assert_eq!(v["presets"], 0);
    }

    #[test]
    fn test_plugin_streaming_insert_seen_in_active_scan_counts() {
        let db = test_db();
        db.plugin_scan_parent_create(
            "ps-stream",
            "2024-01-01T00:00:00",
            &["/Applications".into()],
        )
        .unwrap();
        let p = PluginInfo {
            name: "Test".into(),
            path: "/test.vst3".into(),
            plugin_type: "VST3".into(),
            version: "1.0".into(),
            manufacturer: "Test Co".into(),
            manufacturer_url: None,
            size: "1 MB".into(),
            size_bytes: 1_000_000,
            modified: "2024-01-01".into(),
            architectures: vec![],
        };
        assert_eq!(db.insert_plugin_batch("ps-stream", &[p]).unwrap(), 1);
        db.plugin_scan_parent_finalize("ps-stream", 1, &[], &["/Applications".into()])
            .unwrap();
        db.set_plugin_scan_complete("ps-stream", true).unwrap();
        let v = db.active_scan_inventory_counts().unwrap();
        assert_eq!(v["plugins"], 1);
    }

    /// Many lib tests call `init_global()` in parallel; migrations must not race on one file.
    /// Uses a temp [`history::get_data_dir`] so workers do not touch the real DB; the global
    /// test override is visible on all threads (see `TEST_DATA_DIR_GLOBAL` in `history.rs`).
    #[test]
    fn init_global_concurrent_ok() {
        let tmp = std::env::temp_dir().join(format!(
            "ah_init_global_conc_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        crate::history::set_test_data_dir_path(tmp.clone());

        let threads: Vec<_> = (0..32)
            .map(|_| {
                std::thread::spawn(|| {
                    init_global().expect("init_global");
                    assert!(global_initialized());
                    let _ = global().read_cache("concurrent-init-smoke.json");
                })
            })
            .collect();
        for t in threads {
            t.join().expect("thread join");
        }

        crate::history::clear_test_data_dir_path();
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn init_global_idempotent_same_thread() {
        let tmp = std::env::temp_dir().join(format!(
            "ah_init_global_idem_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        crate::history::set_test_data_dir_path(tmp.clone());

        for _ in 0..64 {
            init_global().expect("init_global");
        }
        assert!(global_initialized());

        crate::history::clear_test_data_dir_path();
        let _ = std::fs::remove_dir_all(&tmp);
    }

    /// Run this to migrate real JSON caches to SQLite.
    /// Not a real test — it's a one-shot migration runner.
    /// Run with: cargo test --manifest-path src-tauri/Cargo.toml "run_migration" -- --nocapture --ignored
    #[test]
    #[ignore]
    fn run_migration() {
        let db = Database::open().expect("Failed to open database");
        let count = db.migrate_from_json().expect("Migration failed");
        println!("Migrated {count} audio samples to SQLite");
        let scans = db.list_scans().expect("Failed to list scans");
        for s in &scans {
            println!(
                "  Scan {} — {} samples, {} bytes, {} roots",
                s.id,
                s.sample_count,
                s.total_bytes,
                s.roots.len()
            );
        }
        if let Ok(stats) = db.audio_stats(None) {
            println!(
                "Stats: {} samples, {} bytes, {} analyzed, {} formats",
                stats.sample_count,
                stats.total_bytes,
                stats.analyzed_count,
                stats.format_counts.len()
            );
        }
    }
}
