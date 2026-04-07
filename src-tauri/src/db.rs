//! SQLite database layer for scalable storage of audio samples, analysis caches,
//! and scan metadata. Replaces JSON file persistence for data that can grow to
//! millions of rows.

use crate::history::{
    self, AudioHistory, AudioSample, AudioScanSnapshot, DawHistory, DawProject, DawScanSnapshot,
    KvrCacheEntry, MidiFile, MidiScanSnapshot, PdfFile, PdfScanSnapshot, PresetFile, PresetHistory,
    PresetScanSnapshot, ScanHistory, ScanSnapshot,
};
use crate::scanner::PluginInfo;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

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

/// Wraps a SQLite connection with WAL mode for concurrent reads.
pub struct Database {
    conn: Mutex<Connection>,
}

/// Parameters for paginated audio sample queries.
#[derive(Debug, Deserialize)]
pub struct AudioQueryParams {
    #[serde(default)]
    pub scan_id: Option<String>,
    #[serde(default)]
    pub search: Option<String>,
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
    let escaped = trimmed.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_");
    Some(format!("%{escaped}%"))
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
}

/// Result of a paginated query.
#[derive(Debug, Serialize)]
pub struct AudioQueryResult {
    pub samples: Vec<AudioSampleRow>,
    #[serde(rename = "totalCount")]
    pub total_count: u64,
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

/// Aggregate stats for a DAW scan — latest scan, unfiltered.
#[derive(Debug, Serialize)]
pub struct DawStatsResult {
    #[serde(rename = "projectCount")]
    pub project_count: u64,
    #[serde(rename = "totalBytes")]
    pub total_bytes: u64,
    #[serde(rename = "dawCounts")]
    pub daw_counts: HashMap<String, u64>,
}

/// Aggregate stats for a preset scan — latest scan, unfiltered, excluding MIDI.
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

/// Canonical inventory: one row per `path` (highest `id` wins — newest insert for that path).
const AUDIO_LIBRARY_IDS: &str = "id IN (SELECT MAX(id) FROM audio_samples GROUP BY path)";
const DAW_LIBRARY_IDS: &str = "id IN (SELECT MAX(id) FROM daw_projects GROUP BY path)";
const PRESET_LIBRARY_IDS: &str =
    "id IN (SELECT MAX(id) FROM presets GROUP BY path)";
const PDF_LIBRARY_IDS: &str = "id IN (SELECT MAX(id) FROM pdfs GROUP BY path)";
const MIDI_LIBRARY_IDS: &str = "id IN (SELECT MAX(id) FROM midi_files GROUP BY path)";
const PLUGIN_LIBRARY_IDS: &str = "id IN (SELECT MAX(id) FROM plugins GROUP BY path)";

/// Latest DAW scan that has at least one `daw_projects` row. Empty scans remain in history but must not shadow prior results.
/// Uses child-row presence (not `project_count`) so streaming scans still resolve after finalize quirks.
const LATEST_DAW_SCAN_ID_SQL: &str = "\
    SELECT s.id FROM daw_scans s \
    WHERE EXISTS (SELECT 1 FROM daw_projects p WHERE p.scan_id = s.id) \
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
    #[serde(rename = "totalUnfiltered")]
    pub total_unfiltered: u64,
}

#[derive(Debug, Serialize)]
pub struct MidiQueryResult {
    #[serde(rename = "midiFiles")]
    pub midi_files: Vec<MidiFile>,
    #[serde(rename = "totalCount")]
    pub total_count: u64,
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

/// Filtered aggregate stats — count + size + per-type breakdown reflecting
/// the active search/filter. One round-trip: COUNT + SUM + GROUP BY in SQL.
#[derive(Debug, Serialize)]
pub struct FilterStatsResult {
    pub count: u64,
    #[serde(rename = "totalBytes")]
    pub total_bytes: u64,
    #[serde(rename = "byType")]
    pub by_type: HashMap<String, u64>,
    #[serde(rename = "bytesByType")]
    pub bytes_by_type: HashMap<String, u64>,
    #[serde(rename = "totalUnfiltered")]
    pub total_unfiltered: u64,
}

/// Current schema version — bump when adding migrations.
#[allow(dead_code)]
const SCHEMA_VERSION: i64 = 4;

impl Database {
    /// Open or create the database in the app data directory.
    pub fn open() -> Result<Self, String> {
        let db_path = history::get_data_dir().join("audio_haxor.db");
        let _ = std::fs::create_dir_all(db_path.parent().unwrap());
        let conn =
            Connection::open(&db_path).map_err(|e| format!("Failed to open database: {e}"))?;
        // Parallel `cargo test` processes can open the same path; without a busy timeout,
        // migrations hit `database is locked` immediately.
        conn.busy_timeout(std::time::Duration::from_secs(30))
            .map_err(|e| format!("Failed to set busy_timeout: {e}"))?;

        // Performance pragmas
        //   cache_size=-262144  — 256 MB page cache (negative = KB). Keeps hot
        //                         indexes resident at millions-of-rows scale.
        //   mmap_size=536870912 — 512 MB memory-mapped reads. SQLite reads pages
        //                         directly from the OS page cache without a read()
        //                         syscall — big win for browse-heavy workloads.
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA synchronous=NORMAL;
             PRAGMA cache_size=-262144;
             PRAGMA mmap_size=536870912;
             PRAGMA foreign_keys=ON;
             PRAGMA temp_store=MEMORY;
             PRAGMA wal_autocheckpoint=1000;",
        )
        .map_err(|e| format!("Failed to set pragmas: {e}"))?;

        let db = Self {
            conn: Mutex::new(conn),
        };
        db.migrate()?;
        Ok(db)
    }

    /// Background housekeeping — checkpoint WAL, refresh query planner stats,
    /// prune old scans, and vacuum if needed. Safe to call from any thread;
    /// must NOT run on the main thread (blocks for seconds on large DBs).
    pub fn housekeep(&self) {
        {
            let conn = self.conn.lock().unwrap();
            let _ = conn.execute_batch("PRAGMA optimize;");
        }
        self.prune_old_scans(3);
        self.vacuum_if_needed();
        self.prewarm();
    }

    /// Prune old scans — keep only the N most recent per type. Reduces DB bloat.
    pub fn prune_old_scans(&self, keep: usize) {
        let conn = self.conn.lock().unwrap();
        let keep_i = keep as i64;
        for (scan_tbl, data_tbl, id_col) in [
            ("audio_scans", "audio_samples", "scan_id"),
            ("plugin_scans", "plugins", "scan_id"),
            ("daw_scans", "daw_projects", "scan_id"),
            ("preset_scans", "presets", "scan_id"),
            ("midi_scans", "midi_files", "scan_id"),
            ("pdf_scans", "pdfs", "scan_id"),
        ] {
            let _ = conn.execute_batch(&format!(
                "DELETE FROM {data_tbl} WHERE {id_col} NOT IN (SELECT id FROM {scan_tbl} ORDER BY timestamp DESC LIMIT {keep_i});
                 DELETE FROM {scan_tbl} WHERE id NOT IN (SELECT id FROM {scan_tbl} ORDER BY timestamp DESC LIMIT {keep_i});"
            ));
        }
    }

    /// Checkpoint WAL to merge it into the main DB file. Keeps WAL small.
    /// Warm the page cache by touching each table + FTS index root. First real
    /// query returns ~1 ms instead of the 50-200 ms cold-cache penalty.
    pub fn prewarm(&self) {
        let conn = self.conn.lock().unwrap();
        let _ = conn.execute_batch(
            "SELECT COUNT(*) FROM audio_samples WHERE id=1;
             SELECT COUNT(*) FROM daw_projects WHERE id=1;
             SELECT COUNT(*) FROM presets WHERE id=1;
             SELECT COUNT(*) FROM midi_files WHERE id=1;
             SELECT COUNT(*) FROM pdfs WHERE id=1;
             SELECT COUNT(*) FROM plugins WHERE id=1;
             SELECT rowid FROM audio_samples_fts WHERE audio_samples_fts MATCH 'xzyq' LIMIT 1;
             SELECT rowid FROM daw_projects_fts WHERE daw_projects_fts MATCH 'xzyq' LIMIT 1;
             SELECT rowid FROM presets_fts WHERE presets_fts MATCH 'xzyq' LIMIT 1;
             SELECT rowid FROM midi_files_fts WHERE midi_files_fts MATCH 'xzyq' LIMIT 1;
             SELECT rowid FROM pdfs_fts WHERE pdfs_fts MATCH 'xzyq' LIMIT 1;",
        );
    }

    pub fn checkpoint(&self) {
        let conn = self.conn.lock().unwrap();
        let _ = conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);");
    }

    /// Resolved app UI strings for the given locale (merged with English fallback).
    pub fn get_app_strings(&self, locale: &str) -> Result<HashMap<String, String>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        crate::app_i18n::load_merged(&conn, locale)
    }

    /// Alias for [`Self::get_app_strings`] (legacy command name).
    pub fn get_toast_strings(&self, locale: &str) -> Result<HashMap<String, String>, String> {
        self.get_app_strings(locale)
    }

    /// VACUUM if >20% of pages are free (dead space from deleted rows).
    pub fn vacuum_if_needed(&self) {
        let conn = self.conn.lock().unwrap();
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
            let conn = self.conn.lock().unwrap();
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

    /// Run schema migrations.
    fn migrate(&self) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();

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

    /// Insert a batch of audio samples in a single transaction.
    pub fn audio_scan_parent_create(
        &self,
        id: &str,
        timestamp: &str,
        roots: &[String],
    ) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        let roots_json = serde_json::to_string(roots).unwrap_or_default();
        conn.execute(
            "INSERT OR REPLACE INTO audio_scans (id, timestamp, sample_count, total_bytes, format_counts, roots) VALUES (?1,?2,0,0,'{}',?3)",
            params![id, timestamp, roots_json],
        ).map_err(|e| e.to_string())?;
        conn.execute(
            "DELETE FROM audio_samples WHERE scan_id = ?1",
            params![id],
        )
        .map_err(|e| e.to_string())?;
        conn.execute(
            "DELETE FROM audio_samples_fts WHERE scan_id = ?1",
            params![id],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn audio_scan_parent_finalize(
        &self,
        id: &str,
        _sample_count: u64,
        _total_bytes: u64,
        _format_counts: &HashMap<String, usize>,
    ) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        let sample_count: i64 = conn
            .query_row(
                "SELECT COUNT(DISTINCT path) FROM audio_samples",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);
        let total_bytes: i64 = conn
            .query_row(
                "SELECT COALESCE(SUM(size), 0) FROM audio_samples WHERE id IN (SELECT MAX(id) FROM audio_samples GROUP BY path)",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);
        let mut format_map: HashMap<String, usize> = HashMap::new();
        let mut stmt = conn
            .prepare(
                "SELECT format, COUNT(*) FROM audio_samples WHERE id IN (SELECT MAX(id) FROM audio_samples GROUP BY path) GROUP BY format",
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

    pub fn insert_audio_batch(&self, scan_id: &str, samples: &[AudioSample]) -> Result<u64, String> {
        let conn = self.conn.lock().unwrap();
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

            for s in samples {
                let changed = stmt
                    .execute(params![
                        s.name,
                        s.path,
                        s.directory,
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
                        .execute(params![id, s.name, s.path, scan_id])
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
        let conn = self.conn.lock().unwrap();
        let fc_json = serde_json::to_string(format_counts).unwrap_or_default();
        let roots_json = serde_json::to_string(roots).unwrap_or_default();
        conn.execute(
            "INSERT OR REPLACE INTO audio_scans
             (id, timestamp, sample_count, total_bytes, format_counts, roots)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
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
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT id FROM audio_scans ORDER BY timestamp DESC LIMIT 1",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|e| e.to_string())
    }

    /// List all scans (metadata only).
    pub fn list_scans(&self) -> Result<Vec<ScanInfo>, String> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT id, timestamp, sample_count, total_bytes, format_counts, roots
                 FROM audio_scans ORDER BY timestamp DESC",
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

    /// Paginated, sortable, filterable query for audio samples.
    ///
    /// `scan_id` **Some(non-empty)** → rows for that scan only (history detail).
    /// `scan_id` **None** or empty → **library** mode: all rows across scans, deduped by `path`
    /// (`MAX(id)` per path).
    pub fn query_audio(&self, params: &AudioQueryParams) -> Result<AudioQueryResult, String> {
        let conn = self.conn.lock().unwrap();

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
        let fts_match = params.search.as_deref().and_then(fts_phrase);
        let like_pat = params.search.as_deref().and_then(short_like);
        if fts_match.is_some() {
            if single_scan {
                conditions.push(format!(
                    "id IN (SELECT rowid FROM audio_samples_fts WHERE audio_samples_fts MATCH ?{bind_idx} AND scan_id = ?{scan_idx})",
                    scan_idx = bind_idx + 1,
                ));
                bind_idx += 2;
            } else {
                conditions.push(format!(
                    "id IN (SELECT rowid FROM audio_samples_fts WHERE audio_samples_fts MATCH ?{bind_idx} AND rowid IN (SELECT MAX(id) FROM audio_samples GROUP BY path))",
                    bind_idx = bind_idx,
                ));
                bind_idx += 1;
            }
        } else if like_pat.is_some() {
            conditions.push(format!(
                "(name LIKE ?{bind_idx} ESCAPE '\\' OR path LIKE ?{bind_idx} ESCAPE '\\')"
            ));
            bind_idx += 1;
        }

        if let Some(fmt) = &params.format_filter {
            if !fmt.is_empty() && fmt != "all" {
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
            conn.query_row(
                "SELECT COUNT(*) FROM audio_samples WHERE id IN (SELECT MAX(id) FROM audio_samples GROUP BY path)",
                [],
                |row| row.get::<_, i64>(0).map(|v| v as u64),
            )
            .unwrap_or(0)
        };

        let query_sql = format!(
            "SELECT name, path, directory, format, size, size_formatted, modified,
                    duration, channels, sample_rate, bits_per_sample, bpm, key_name, lufs,
                    COUNT(*) OVER() AS _total
             FROM audio_samples
             WHERE {where_clause}
             ORDER BY {sort_col} {sort_dir} {nulls}
             LIMIT ?{limit_idx} OFFSET ?{offset_idx}",
            limit_idx = bind_idx,
            offset_idx = bind_idx + 1,
        );

        let mut stmt = conn.prepare(&query_sql).map_err(|e| e.to_string())?;
        let mut idx = 1;
        if single_scan {
            stmt.raw_bind_parameter(idx, &scan_id)
                .map_err(|e| e.to_string())?;
            idx += 1;
        }
        if let Some(ref m) = fts_match {
            stmt.raw_bind_parameter(idx, m)
                .map_err(|e| e.to_string())?;
            idx += 1;
            if single_scan {
                stmt.raw_bind_parameter(idx, &scan_id)
                    .map_err(|e| e.to_string())?;
                idx += 1;
            }
        } else if let Some(ref pat) = like_pat {
            stmt.raw_bind_parameter(idx, pat)
                .map_err(|e| e.to_string())?;
            idx += 1;
        }
        if let Some(ref fmt) = params.format_filter {
            if !fmt.is_empty() && fmt != "all" && !fmt.contains(',') {
                stmt.raw_bind_parameter(idx, fmt)
                    .map_err(|e| e.to_string())?;
                idx += 1;
            }
        }
        stmt.raw_bind_parameter(idx, params.limit as i64)
            .map_err(|e| e.to_string())?;
        stmt.raw_bind_parameter(idx + 1, params.offset as i64)
            .map_err(|e| e.to_string())?;

        let mut samples = Vec::new();
        let mut total_count = 0u64;
        let mut rows = stmt.raw_query();
        while let Some(row) = rows.next().map_err(|e| e.to_string())? {
            if total_count == 0 {
                total_count = row.get::<_, i64>(14).unwrap_or(0) as u64;
            }
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
            });
        }

        Ok(AudioQueryResult {
            samples,
            total_count,
            total_unfiltered,
        })
    }

    /// Get aggregate stats. `scan_id` None or empty → full library (deduped by path). Otherwise that scan only.
    pub fn audio_stats(&self, scan_id: Option<&str>) -> Result<AudioStatsResult, String> {
        let conn = self.conn.lock().unwrap();

        let library = scan_id.map(|s| s.is_empty()).unwrap_or(true);
        if library {
            let sample_count: u64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM audio_samples WHERE id IN (SELECT MAX(id) FROM audio_samples GROUP BY path)",
                    [],
                    |row| row.get::<_, i64>(0).map(|v| v as u64),
                )
                .unwrap_or(0);
            let total_bytes: u64 = conn
                .query_row(
                    "SELECT COALESCE(SUM(size), 0) FROM audio_samples WHERE id IN (SELECT MAX(id) FROM audio_samples GROUP BY path)",
                    [],
                    |row| row.get::<_, i64>(0).map(|v| v as u64),
                )
                .unwrap_or(0);
            let mut format_counts = HashMap::new();
            let mut stmt = conn
                .prepare(
                    "SELECT format, COUNT(*) FROM audio_samples WHERE id IN (SELECT MAX(id) FROM audio_samples GROUP BY path) GROUP BY format",
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
                    "SELECT COUNT(*) FROM audio_samples WHERE id IN (SELECT MAX(id) FROM audio_samples GROUP BY path) AND bpm IS NOT NULL",
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
        let conn = self.conn.lock().unwrap();
        let library = scan_id.map(|s| s.is_empty()).unwrap_or(true);
        if library {
            let project_count: u64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM daw_projects WHERE id IN (SELECT MAX(id) FROM daw_projects GROUP BY path)",
                    [],
                    |row| row.get::<_, i64>(0).map(|v| v as u64),
                )
                .unwrap_or(0);
            let total_bytes: u64 = conn
                .query_row(
                    "SELECT COALESCE(SUM(size), 0) FROM daw_projects WHERE id IN (SELECT MAX(id) FROM daw_projects GROUP BY path)",
                    [],
                    |row| row.get::<_, i64>(0).map(|v| v as u64),
                )
                .unwrap_or(0);
            let mut daw_counts = HashMap::new();
            let mut stmt = conn
                .prepare(
                    "SELECT daw, COUNT(*) FROM daw_projects WHERE id IN (SELECT MAX(id) FROM daw_projects GROUP BY path) GROUP BY daw",
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
        let conn = self.conn.lock().unwrap();
        let library = scan_id.map(|s| s.is_empty()).unwrap_or(true);
        if library {
            let preset_count: u64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM presets WHERE id IN (SELECT MAX(id) FROM presets GROUP BY path) AND format NOT IN ('MID', 'MIDI')",
                    [],
                    |row| row.get::<_, i64>(0).map(|v| v as u64),
                )
                .unwrap_or(0);
            let total_bytes: u64 = conn
                .query_row(
                    "SELECT COALESCE(SUM(size), 0) FROM presets WHERE id IN (SELECT MAX(id) FROM presets GROUP BY path) AND format NOT IN ('MID', 'MIDI')",
                    [],
                    |row| row.get::<_, i64>(0).map(|v| v as u64),
                )
                .unwrap_or(0);
            let mut format_counts = HashMap::new();
            let mut stmt = conn
                .prepare(
                    "SELECT format, COUNT(*) FROM presets WHERE id IN (SELECT MAX(id) FROM presets GROUP BY path) AND format NOT IN ('MID', 'MIDI') GROUP BY format",
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
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE audio_samples SET bpm = ?1 WHERE path = ?2",
            params![bpm, path],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Update musical key for a sample.
    pub fn update_key(&self, path: &str, key: Option<&str>) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
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
        let conn = self.conn.lock().unwrap();
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
        let conn = self.conn.lock().unwrap();
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
            stmt.raw_bind_parameter(idx, p)
                .map_err(|e| e.to_string())?;
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
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE audio_samples SET lufs = ?1 WHERE path = ?2",
            params![lufs, path],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Get analysis data for a single sample.
    pub fn get_analysis(&self, path: &str) -> Result<serde_json::Value, String> {
        let conn = self.conn.lock().unwrap();
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

    /// Get paths of samples that haven't been analyzed yet (bpm IS NULL on library row).
    pub fn unanalyzed_paths(&self, limit: u64) -> Result<Vec<String>, String> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(&format!(
                "SELECT path FROM audio_samples
                 WHERE bpm IS NULL AND ({AUDIO_LIBRARY_IDS})
                 LIMIT ?1"
            ))
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![limit as i64], |row| row.get(0))
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<String>, _>>()
            .map_err(|e| e.to_string())
    }

    /// Delete a scan and its samples.
    pub fn delete_scan(&self, scan_id: &str) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM audio_samples WHERE scan_id = ?1",
            params![scan_id],
        )
        .map_err(|e| e.to_string())?;
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
        sort_key: &str,
        sort_asc: bool,
        offset: u64,
        limit: u64,
    ) -> Result<PluginQueryResult, String> {
        let conn = self.conn.lock().unwrap();
        let total_unfiltered: u64 = conn
            .query_row(
                "SELECT COUNT(*) FROM plugins WHERE id IN (SELECT MAX(id) FROM plugins GROUP BY path)",
                [],
                |row| row.get::<_, i64>(0).map(|v| v as u64),
            )
            .unwrap_or(0);
        if total_unfiltered == 0 {
            return Ok(PluginQueryResult {
                plugins: vec![],
                total_count: 0,
                total_unfiltered: 0,
            });
        }

        let mut where_parts = vec![PLUGIN_LIBRARY_IDS.to_string()];
        let mut bind_idx = 1usize;
        let search_pat = search.and_then(|s| {
            let t = s.trim();
            if t.is_empty() {
                None
            } else {
                Some(format!(
                    "%{}%",
                    t.chars()
                        .map(|c| c.to_string())
                        .collect::<Vec<_>>()
                        .join("%")
                ))
            }
        });
        if search_pat.is_some() {
            where_parts.push(format!("(name LIKE ?{bind_idx} ESCAPE '\\' OR manufacturer LIKE ?{bind_idx} ESCAPE '\\' OR path LIKE ?{bind_idx} ESCAPE '\\')"));
            bind_idx += 1;
        }
        if let Some(tf) = type_filter {
            if !tf.is_empty() && tf != "all" {
                if tf.contains(',') {
                    let vals: Vec<String> = tf
                        .split(',')
                        .map(|s| format!("'{}'", s.trim().replace('\'', "''")))
                        .collect();
                    where_parts.push(format!("plugin_type IN ({})", vals.join(",")));
                } else {
                    where_parts.push(format!("plugin_type = ?{bind_idx}"));
                    bind_idx += 1;
                }
            }
        }
        let where_cl = where_parts.join(" AND ");

        let sort_col = match sort_key {
            "name" => "name COLLATE NOCASE",
            "type" => "plugin_type",
            "version" => "version",
            "manufacturer" => "manufacturer COLLATE NOCASE",
            "size" => "size_bytes",
            "modified" => "modified",
            _ => "name COLLATE NOCASE",
        };
        let dir = if sort_asc { "ASC" } else { "DESC" };

        let total_count: u64 = {
            let sql = format!("SELECT COUNT(*) FROM plugins WHERE {where_cl}");
            let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
            let mut bi = 1;
            if let Some(ref p) = search_pat {
                stmt.raw_bind_parameter(bi, p).map_err(|e| e.to_string())?;
                bi += 1;
            }
            if let Some(tf) = type_filter {
                if !tf.is_empty() && tf != "all" && !tf.contains(',') {
                    stmt.raw_bind_parameter(bi, tf).map_err(|e| e.to_string())?;
                    bi += 1;
                }
            }
            let mut rows = stmt.raw_query();
            rows.next()
                .map_err(|e| e.to_string())?
                .map(|r| r.get::<_, i64>(0).unwrap_or(0) as u64)
                .unwrap_or(0)
        };

        let mut bind_offset = 1usize;
        if search_pat.is_some() {
            bind_offset += 1;
        }
        if type_filter
            .map(|t| !t.is_empty() && t != "all" && !t.contains(','))
            .unwrap_or(false)
        {
            bind_offset += 1;
        }
        let sql = format!("SELECT name, path, plugin_type, version, manufacturer, manufacturer_url, size, size_bytes, modified, architectures FROM plugins WHERE {where_cl} ORDER BY {sort_col} {dir} LIMIT ?{bind_offset} OFFSET ?{}", bind_offset + 1);
        let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
        let mut bi = 1;
        if let Some(ref p) = search_pat {
            stmt.raw_bind_parameter(bi, p).map_err(|e| e.to_string())?;
            bi += 1;
        }
        if let Some(tf) = type_filter {
            if !tf.is_empty() && tf != "all" && !tf.contains(',') {
                stmt.raw_bind_parameter(bi, tf).map_err(|e| e.to_string())?;
                bi += 1;
            }
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
        offset: u64,
        limit: u64,
    ) -> Result<DawQueryResult, String> {
        let conn = self.conn.lock().unwrap();
        let total_unfiltered: u64 = conn
            .query_row(
                "SELECT COUNT(*) FROM daw_projects WHERE id IN (SELECT MAX(id) FROM daw_projects GROUP BY path)",
                [],
                |row| row.get::<_, i64>(0).map(|v| v as u64),
            )
            .unwrap_or(0);
        if total_unfiltered == 0 {
            return Ok(DawQueryResult {
                projects: vec![],
                total_count: 0,
                total_unfiltered: 0,
            });
        }

        let mut where_parts = vec![DAW_LIBRARY_IDS.to_string()];
        let mut bind_idx = 1usize;
        let fts_match = search.and_then(fts_phrase);
        let like_pat = search.and_then(short_like);
        if fts_match.is_some() {
            where_parts.push(format!(
                "id IN (SELECT rowid FROM daw_projects_fts WHERE daw_projects_fts MATCH ?{bind_idx} AND rowid IN (SELECT MAX(id) FROM daw_projects GROUP BY path))",
            ));
            bind_idx += 1;
        } else if like_pat.is_some() {
            where_parts.push(format!("(name LIKE ?{bind_idx} ESCAPE '\\' OR path LIKE ?{bind_idx} ESCAPE '\\')"));
            bind_idx += 1;
        }
        if let Some(f) = daw_filter {
            if !f.is_empty() && f != "all" {
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

        let total_count: u64 = {
            let sql = format!("SELECT COUNT(*) FROM daw_projects WHERE {where_cl}");
            let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
            let mut bi = 1;
            if let Some(ref m) = fts_match {
                stmt.raw_bind_parameter(bi, m).map_err(|e| e.to_string())?;
                bi += 1;
            } else if let Some(ref pat) = like_pat {
                stmt.raw_bind_parameter(bi, pat).map_err(|e| e.to_string())?;
                bi += 1;
            }
            if let Some(f) = daw_filter {
                if !f.is_empty() && f != "all" && !f.contains(',') {
                    stmt.raw_bind_parameter(bi, f).map_err(|e| e.to_string())?;
                }
            }
            let mut rows = stmt.raw_query();
            rows.next()
                .map_err(|e| e.to_string())?
                .map(|r| r.get::<_, i64>(0).unwrap_or(0) as u64)
                .unwrap_or(0)
        };

        let sql = format!("SELECT name, path, directory, format, daw, size, size_formatted, modified FROM daw_projects WHERE {where_cl} ORDER BY {sort_col} {dir} LIMIT ?{bind_idx} OFFSET ?{}", bind_idx + 1);
        let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
        let mut bi = 1;
        if let Some(ref m) = fts_match {
            stmt.raw_bind_parameter(bi, m).map_err(|e| e.to_string())?;
            bi += 1;
        } else if let Some(ref pat) = like_pat {
            stmt.raw_bind_parameter(bi, pat).map_err(|e| e.to_string())?;
            bi += 1;
        }
        if let Some(f) = daw_filter {
            // Comma-separated filters are inlined into `daw IN (...)` by the SQL builder
            // and have no placeholder to bind to — skip them here.
            if !f.is_empty() && f != "all" && !f.contains(',') {
                stmt.raw_bind_parameter(bi, f).map_err(|e| e.to_string())?;
                bi += 1;
            }
        }
        stmt.raw_bind_parameter(bi, limit as i64)
            .map_err(|e| e.to_string())?;
        stmt.raw_bind_parameter(bi + 1, offset as i64)
            .map_err(|e| e.to_string())?;

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
        offset: u64,
        limit: u64,
    ) -> Result<PresetQueryResult, String> {
        let conn = self.conn.lock().unwrap();
        let total_unfiltered: u64 = conn
            .query_row(
                "SELECT COUNT(*) FROM presets WHERE id IN (SELECT MAX(id) FROM presets GROUP BY path) AND format NOT IN ('MID', 'MIDI')",
                [],
                |row| row.get::<_, i64>(0).map(|v| v as u64),
            )
            .unwrap_or(0);
        if total_unfiltered == 0 {
            return Ok(PresetQueryResult {
                presets: vec![],
                total_count: 0,
                total_unfiltered: 0,
            });
        }

        let mut where_parts = vec![
            PRESET_LIBRARY_IDS.to_string(),
            "format NOT IN ('MID', 'MIDI')".to_string(),
        ];
        let mut bind_idx = 1usize;
        let fts_match = search.and_then(fts_phrase);
        let like_pat = search.and_then(short_like);
        if fts_match.is_some() {
            where_parts.push(format!(
                "id IN (SELECT rowid FROM presets_fts WHERE presets_fts MATCH ?{bind_idx} AND rowid IN (SELECT MAX(id) FROM presets GROUP BY path))",
            ));
            bind_idx += 1;
        } else if like_pat.is_some() {
            where_parts.push(format!("(name LIKE ?{bind_idx} ESCAPE '\\' OR path LIKE ?{bind_idx} ESCAPE '\\')"));
            bind_idx += 1;
        }
        if let Some(f) = format_filter {
            if !f.is_empty() && f != "all" {
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

        let total_count: u64 = {
            let sql = format!("SELECT COUNT(*) FROM presets WHERE {where_cl}");
            let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
            let mut bi = 1;
            if let Some(ref m) = fts_match {
                stmt.raw_bind_parameter(bi, m).map_err(|e| e.to_string())?;
                bi += 1;
            } else if let Some(ref pat) = like_pat {
                stmt.raw_bind_parameter(bi, pat).map_err(|e| e.to_string())?;
                bi += 1;
            }
            if let Some(f) = format_filter {
                if !f.is_empty() && f != "all" && !f.contains(',') {
                    stmt.raw_bind_parameter(bi, f).map_err(|e| e.to_string())?;
                }
            }
            let mut rows = stmt.raw_query();
            rows.next()
                .map_err(|e| e.to_string())?
                .map(|r| r.get::<_, i64>(0).unwrap_or(0) as u64)
                .unwrap_or(0)
        };

        let sql = format!("SELECT name, path, directory, format, size, size_formatted, modified FROM presets WHERE {where_cl} ORDER BY {sort_col} {dir} LIMIT ?{bind_idx} OFFSET ?{}", bind_idx + 1);
        let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
        let mut bi = 1;
        if let Some(ref m) = fts_match {
            stmt.raw_bind_parameter(bi, m).map_err(|e| e.to_string())?;
            bi += 1;
        } else if let Some(ref pat) = like_pat {
            stmt.raw_bind_parameter(bi, pat).map_err(|e| e.to_string())?;
            bi += 1;
        }
        if let Some(f) = format_filter {
            // Comma-separated filters are inlined into `format IN (...)` by the SQL builder
            // and have no placeholder to bind to — skip them here.
            if !f.is_empty() && f != "all" && !f.contains(',') {
                stmt.raw_bind_parameter(bi, f).map_err(|e| e.to_string())?;
                bi += 1;
            }
        }
        stmt.raw_bind_parameter(bi, limit as i64)
            .map_err(|e| e.to_string())?;
        stmt.raw_bind_parameter(bi + 1, offset as i64)
            .map_err(|e| e.to_string())?;

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
            total_unfiltered,
        })
    }

    pub fn save_plugin_scan(&self, snap: &ScanSnapshot) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        let dirs_json = serde_json::to_string(&snap.directories).unwrap_or_default();
        let roots_json = serde_json::to_string(&snap.roots).unwrap_or_default();
        conn.execute(
            "INSERT OR REPLACE INTO plugin_scans (id, timestamp, plugin_count, directories, roots) VALUES (?1,?2,?3,?4,?5)",
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
                stmt.execute(params![
                    p.name,
                    p.path,
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
        tx.commit().map_err(|e| e.to_string())
    }

    /// Begin a streaming plugin scan: parent row + clear prior rows for this id.
    pub fn plugin_scan_parent_create(
        &self,
        id: &str,
        timestamp: &str,
        roots: &[String],
    ) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        let dirs_json = "[]".to_string();
        let roots_json = serde_json::to_string(roots).unwrap_or_default();
        conn.execute(
            "INSERT OR REPLACE INTO plugin_scans (id, timestamp, plugin_count, directories, roots) VALUES (?1,?2,0,?3,?4)",
            params![id, timestamp, dirs_json, roots_json],
        )
        .map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM plugins WHERE scan_id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Append plugins for a streaming scan; updates `plugin_scans.plugin_count` incrementally.
    pub fn insert_plugin_batch(&self, scan_id: &str, batch: &[PluginInfo]) -> Result<u64, String> {
        let conn = self.conn.lock().unwrap();
        let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
        let mut inserted: u64 = 0;
        {
            let mut stmt = tx
                .prepare_cached(
                    "INSERT OR IGNORE INTO plugins (name, path, plugin_type, version, manufacturer, manufacturer_url, size, size_bytes, modified, architectures, scan_id) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
                )
                .map_err(|e| e.to_string())?;
            for p in batch {
                let arch_json = serde_json::to_string(&p.architectures).unwrap_or_default();
                let changed = stmt
                    .execute(params![
                        p.name,
                        p.path,
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
        let conn = self.conn.lock().unwrap();
        let plugin_count: i64 = conn
            .query_row(
                "SELECT COUNT(DISTINCT path) FROM plugins",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);
        let dirs_json = serde_json::to_string(directories).unwrap_or_default();
        let roots_json = serde_json::to_string(roots).unwrap_or_default();
        conn.execute(
            "UPDATE plugin_scans SET plugin_count = ?2, directories = ?3, roots = ?4 WHERE id = ?1",
            params![id, plugin_count, dirs_json, roots_json],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn get_plugin_scans(&self) -> Result<Vec<serde_json::Value>, String> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT s.id, s.timestamp, COALESCE(NULLIF(s.plugin_count,0),(SELECT COUNT(*) FROM plugins WHERE scan_id = s.id)), s.roots FROM plugin_scans s ORDER BY s.timestamp DESC",
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
        let conn = self.conn.lock().unwrap();
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
        let conn = self.conn.lock().unwrap();
        let id: Option<String> = conn
            .query_row(
                "SELECT id FROM plugin_scans ORDER BY timestamp DESC LIMIT 1",
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
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM plugins WHERE scan_id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM plugin_scans WHERE id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn clear_plugin_history(&self) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch("DELETE FROM plugins; DELETE FROM plugin_scans;")
            .map_err(|e| e.to_string())
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
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT s.id, s.timestamp, COALESCE(NULLIF(s.sample_count,0),(SELECT COUNT(*) FROM audio_samples WHERE scan_id = s.id)), COALESCE(NULLIF(s.total_bytes,0),(SELECT COALESCE(SUM(size),0) FROM audio_samples WHERE scan_id = s.id)), s.format_counts, s.roots FROM audio_scans s ORDER BY s.timestamp DESC",
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
        let conn = self.conn.lock().unwrap();
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
        let conn = self.conn.lock().unwrap();
        let id: Option<String> = conn
            .query_row(
                "SELECT id FROM audio_scans ORDER BY timestamp DESC LIMIT 1",
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
        let conn = self.conn.lock().unwrap();
        conn.execute_batch("DELETE FROM audio_samples; DELETE FROM audio_scans;")
            .map_err(|e| e.to_string())
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
        let conn = self.conn.lock().unwrap();
        let roots_json = serde_json::to_string(roots).unwrap_or_default();
        conn.execute(
            "INSERT OR REPLACE INTO daw_scans (id, timestamp, project_count, total_bytes, daw_counts, roots) VALUES (?1,?2,0,0,'{}',?3)",
            params![id, timestamp, roots_json],
        ).map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM daw_projects WHERE scan_id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM daw_projects_fts WHERE scan_id = ?1", params![id])
            .map_err(|e| e.to_string())?;
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
        let conn = self.conn.lock().unwrap();
        let project_count: i64 = conn
            .query_row(
                "SELECT COUNT(DISTINCT path) FROM daw_projects",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);
        let total_bytes: i64 = conn
            .query_row(
                "SELECT COALESCE(SUM(size), 0) FROM daw_projects WHERE id IN (SELECT MAX(id) FROM daw_projects GROUP BY path)",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);
        let mut daw_map: HashMap<String, usize> = HashMap::new();
        let mut stmt = conn
            .prepare(
                "SELECT daw, COUNT(*) FROM daw_projects WHERE id IN (SELECT MAX(id) FROM daw_projects GROUP BY path) GROUP BY daw",
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
        let conn = self.conn.lock().unwrap();
        let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
        let mut inserted_idx: Vec<usize> = Vec::new();
        let mut batch_bytes: u64 = 0;
        {
            let mut stmt = tx.prepare_cached("INSERT OR IGNORE INTO daw_projects (name, path, directory, format, daw, size, size_formatted, modified, scan_id) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)").map_err(|e| e.to_string())?;
            let mut fts_stmt = tx.prepare_cached("INSERT INTO daw_projects_fts(rowid, name, path, daw, scan_id) VALUES (?1,?2,?3,?4,?5)").map_err(|e| e.to_string())?;
            for (i, p) in projects.iter().enumerate() {
                let changed = stmt.execute(params![
                    p.name,
                    p.path,
                    p.directory,
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
                    fts_stmt.execute(params![id, p.name, p.path, p.daw, scan_id]).map_err(|e| e.to_string())?;
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
        Ok(inserted_idx)
    }

    pub fn save_daw_scan(&self, snap: &DawScanSnapshot) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        let daw_json = serde_json::to_string(&snap.daw_counts).unwrap_or_default();
        let roots_json = serde_json::to_string(&snap.roots).unwrap_or_default();
        conn.execute(
            "INSERT OR REPLACE INTO daw_scans (id, timestamp, project_count, total_bytes, daw_counts, roots) VALUES (?1,?2,?3,?4,?5,?6)",
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
                let changed = stmt.execute(params![
                    p.name,
                    p.path,
                    p.directory,
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
                    fts_stmt.execute(params![id, p.name, p.path, p.daw, snap.id]).map_err(|e| e.to_string())?;
                }
            }
        }
        tx.commit().map_err(|e| e.to_string())
    }

    pub fn get_daw_scans(&self) -> Result<Vec<serde_json::Value>, String> {
        let conn = self.conn.lock().unwrap();
        // Count from child rows so the History tab stays correct even if parent totals
        // were never finalized (streaming scans) or finalize failed silently.
        let mut stmt = conn.prepare(
            "SELECT s.id, s.timestamp, COALESCE(NULLIF(s.project_count,0),(SELECT COUNT(*) FROM daw_projects WHERE scan_id = s.id)), COALESCE(NULLIF(s.total_bytes,0),(SELECT COALESCE(SUM(size),0) FROM daw_projects WHERE scan_id = s.id)), s.daw_counts, s.roots FROM daw_scans s ORDER BY s.timestamp DESC",
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
        let conn = self.conn.lock().unwrap();
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
        let conn = self.conn.lock().unwrap();
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
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM daw_projects WHERE scan_id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM daw_scans WHERE id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn clear_daw_history(&self) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch("DELETE FROM daw_projects; DELETE FROM daw_scans;")
            .map_err(|e| e.to_string())
    }

    // ── Preset scan CRUD ──

    pub fn preset_scan_parent_create(
        &self,
        id: &str,
        timestamp: &str,
        roots: &[String],
    ) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        let roots_json = serde_json::to_string(roots).unwrap_or_default();
        conn.execute(
            "INSERT OR REPLACE INTO preset_scans (id, timestamp, preset_count, total_bytes, format_counts, roots) VALUES (?1,?2,0,0,'{}',?3)",
            params![id, timestamp, roots_json],
        ).map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM presets WHERE scan_id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM presets_fts WHERE scan_id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn preset_scan_parent_finalize(
        &self,
        id: &str,
        _preset_count: usize,
        _total_bytes: u64,
        _format_counts: &HashMap<String, usize>,
    ) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        let preset_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM presets WHERE id IN (SELECT MAX(id) FROM presets GROUP BY path) AND format NOT IN ('MID', 'MIDI')",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);
        let total_bytes: i64 = conn
            .query_row(
                "SELECT COALESCE(SUM(size), 0) FROM presets WHERE id IN (SELECT MAX(id) FROM presets GROUP BY path) AND format NOT IN ('MID', 'MIDI')",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);
        let mut format_map: HashMap<String, usize> = HashMap::new();
        let mut stmt = conn
            .prepare(
                "SELECT format, COUNT(*) FROM presets WHERE id IN (SELECT MAX(id) FROM presets GROUP BY path) AND format NOT IN ('MID', 'MIDI') GROUP BY format",
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
        let conn = self.conn.lock().unwrap();
        let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
        let mut inserted: u64 = 0;
        let mut batch_bytes: u64 = 0;
        {
            let mut stmt = tx.prepare_cached("INSERT OR IGNORE INTO presets (name, path, directory, format, size, size_formatted, modified, scan_id) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)").map_err(|e| e.to_string())?;
            let mut fts_stmt = tx.prepare_cached("INSERT INTO presets_fts(rowid, name, path, format, scan_id) VALUES (?1,?2,?3,?4,?5)").map_err(|e| e.to_string())?;
            for p in presets {
                let changed = stmt.execute(params![
                    p.name,
                    p.path,
                    p.directory,
                    p.format,
                    p.size as i64,
                    p.size_formatted,
                    p.modified,
                    scan_id
                ])
                .map_err(|e| e.to_string())?;
                if changed > 0 {
                    let id = tx.last_insert_rowid();
                    fts_stmt.execute(params![id, p.name, p.path, p.format, scan_id]).map_err(|e| e.to_string())?;
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
        Ok(inserted)
    }

    pub fn save_preset_scan(&self, snap: &PresetScanSnapshot) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        let fc_json = serde_json::to_string(&snap.format_counts).unwrap_or_default();
        let roots_json = serde_json::to_string(&snap.roots).unwrap_or_default();
        conn.execute(
            "INSERT OR REPLACE INTO preset_scans (id, timestamp, preset_count, total_bytes, format_counts, roots) VALUES (?1,?2,?3,?4,?5,?6)",
            params![snap.id, snap.timestamp, snap.preset_count as i64, snap.total_bytes as i64, fc_json, roots_json],
        ).map_err(|e| e.to_string())?;
        let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
        {
            tx.execute("DELETE FROM presets WHERE scan_id = ?1", params![snap.id])
                .map_err(|e| e.to_string())?;
            tx.execute("DELETE FROM presets_fts WHERE scan_id = ?1", params![snap.id])
                .map_err(|e| e.to_string())?;
            let mut stmt = tx.prepare_cached("INSERT OR IGNORE INTO presets (name, path, directory, format, size, size_formatted, modified, scan_id) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)").map_err(|e| e.to_string())?;
            let mut fts_stmt = tx.prepare_cached("INSERT INTO presets_fts(rowid, name, path, format, scan_id) VALUES (?1,?2,?3,?4,?5)").map_err(|e| e.to_string())?;
            for p in &snap.presets {
                let changed = stmt.execute(params![
                    p.name,
                    p.path,
                    p.directory,
                    p.format,
                    p.size as i64,
                    p.size_formatted,
                    p.modified,
                    snap.id
                ])
                .map_err(|e| e.to_string())?;
                if changed > 0 {
                    let id = tx.last_insert_rowid();
                    fts_stmt.execute(params![id, p.name, p.path, p.format, snap.id]).map_err(|e| e.to_string())?;
                }
            }
        }
        tx.commit().map_err(|e| e.to_string())
    }

    pub fn get_preset_scans(&self) -> Result<Vec<serde_json::Value>, String> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT s.id, s.timestamp, COALESCE(NULLIF(s.preset_count,0),(SELECT COUNT(*) FROM presets WHERE scan_id = s.id)), COALESCE(NULLIF(s.total_bytes,0),(SELECT COALESCE(SUM(size),0) FROM presets WHERE scan_id = s.id)), s.format_counts, s.roots FROM preset_scans s ORDER BY s.timestamp DESC",
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
        let conn = self.conn.lock().unwrap();
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
        let conn = self.conn.lock().unwrap();
        let id: Option<String> = conn
            .query_row(
                "SELECT id FROM preset_scans ORDER BY timestamp DESC LIMIT 1",
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
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM presets WHERE scan_id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM preset_scans WHERE id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn clear_preset_history(&self) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch("DELETE FROM presets; DELETE FROM preset_scans;")
            .map_err(|e| e.to_string())
    }

    // ── MIDI scan CRUD ──

    pub fn midi_scan_parent_create(
        &self,
        id: &str,
        timestamp: &str,
        roots: &[String],
    ) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        let roots_json = serde_json::to_string(roots).unwrap_or_default();
        conn.execute(
            "INSERT OR REPLACE INTO midi_scans (id, timestamp, midi_count, total_bytes, format_counts, roots) VALUES (?1,?2,0,0,'{}',?3)",
            params![id, timestamp, roots_json],
        ).map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM midi_files WHERE scan_id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM midi_files_fts WHERE scan_id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn midi_scan_parent_finalize(
        &self,
        id: &str,
        _midi_count: usize,
        _total_bytes: u64,
        _format_counts: &HashMap<String, usize>,
    ) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        let midi_count: i64 = conn
            .query_row(
                "SELECT COUNT(DISTINCT path) FROM midi_files",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);
        let total_bytes: i64 = conn
            .query_row(
                "SELECT COALESCE(SUM(size), 0) FROM midi_files WHERE id IN (SELECT MAX(id) FROM midi_files GROUP BY path)",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);
        let mut format_map: HashMap<String, usize> = HashMap::new();
        let mut stmt = conn
            .prepare(
                "SELECT format, COUNT(*) FROM midi_files WHERE id IN (SELECT MAX(id) FROM midi_files GROUP BY path) GROUP BY format",
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

    pub fn insert_midi_batch(
        &self,
        scan_id: &str,
        midi_files: &[MidiFile],
    ) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
        let mut inserted: u64 = 0;
        let mut batch_bytes: u64 = 0;
        {
            let mut stmt = tx.prepare_cached("INSERT OR IGNORE INTO midi_files (name, path, directory, format, size, size_formatted, modified, scan_id) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)").map_err(|e| e.to_string())?;
            let mut fts_stmt = tx.prepare_cached("INSERT INTO midi_files_fts(rowid, name, path, scan_id) VALUES (?1,?2,?3,?4)").map_err(|e| e.to_string())?;
            for m in midi_files {
                let changed = stmt.execute(params![
                    m.name,
                    m.path,
                    m.directory,
                    m.format,
                    m.size as i64,
                    m.size_formatted,
                    m.modified,
                    scan_id
                ])
                .map_err(|e| e.to_string())?;
                if changed > 0 {
                    let id = tx.last_insert_rowid();
                    fts_stmt.execute(params![id, m.name, m.path, scan_id]).map_err(|e| e.to_string())?;
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
        tx.commit().map_err(|e| e.to_string())
    }

    pub fn save_midi_scan(&self, snap: &MidiScanSnapshot) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        let fc_json = serde_json::to_string(&snap.format_counts).unwrap_or_default();
        let roots_json = serde_json::to_string(&snap.roots).unwrap_or_default();
        conn.execute(
            "INSERT OR REPLACE INTO midi_scans (id, timestamp, midi_count, total_bytes, format_counts, roots) VALUES (?1,?2,?3,?4,?5,?6)",
            params![snap.id, snap.timestamp, snap.midi_count as i64, snap.total_bytes as i64, fc_json, roots_json],
        ).map_err(|e| e.to_string())?;
        let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
        {
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
            let mut fts_stmt = tx.prepare_cached("INSERT INTO midi_files_fts(rowid, name, path, scan_id) VALUES (?1,?2,?3,?4)").map_err(|e| e.to_string())?;
            for m in &snap.midi_files {
                let changed = stmt.execute(params![
                    m.name,
                    m.path,
                    m.directory,
                    m.format,
                    m.size as i64,
                    m.size_formatted,
                    m.modified,
                    snap.id
                ])
                .map_err(|e| e.to_string())?;
                if changed > 0 {
                    let id = tx.last_insert_rowid();
                    fts_stmt.execute(params![id, m.name, m.path, snap.id]).map_err(|e| e.to_string())?;
                }
            }
        }
        tx.commit().map_err(|e| e.to_string())
    }

    pub fn get_midi_scans(&self) -> Result<Vec<serde_json::Value>, String> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT s.id, s.timestamp, COALESCE(NULLIF(s.midi_count,0),(SELECT COUNT(*) FROM midi_files WHERE scan_id = s.id)), COALESCE(NULLIF(s.total_bytes,0),(SELECT COALESCE(SUM(size),0) FROM midi_files WHERE scan_id = s.id)), s.format_counts, s.roots FROM midi_scans s ORDER BY s.timestamp DESC",
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
        let conn = self.conn.lock().unwrap();
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
        let conn = self.conn.lock().unwrap();
        let id: Option<String> = conn
            .query_row(
                "SELECT id FROM midi_scans ORDER BY timestamp DESC LIMIT 1",
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
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM midi_files WHERE scan_id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM midi_scans WHERE id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn clear_midi_history(&self) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch("DELETE FROM midi_files; DELETE FROM midi_scans;")
            .map_err(|e| e.to_string())
    }

    pub fn query_midi(
        &self,
        search: Option<&str>,
        format_filter: Option<&str>,
        sort_key: &str,
        sort_asc: bool,
        offset: u64,
        limit: u64,
    ) -> Result<MidiQueryResult, String> {
        let conn = self.conn.lock().unwrap();
        let total_unfiltered: u64 = conn
            .query_row(
                "SELECT COUNT(*) FROM midi_files WHERE id IN (SELECT MAX(id) FROM midi_files GROUP BY path)",
                [],
                |row| row.get::<_, i64>(0).map(|v| v as u64),
            )
            .unwrap_or(0);
        if total_unfiltered == 0 {
            return Ok(MidiQueryResult {
                midi_files: vec![],
                total_count: 0,
                total_unfiltered: 0,
            });
        }

        let mut where_parts = vec![MIDI_LIBRARY_IDS.to_string()];
        let mut bind_idx = 1usize;
        let fts_match = search.and_then(fts_phrase);
        let like_pat = search.and_then(short_like);
        if fts_match.is_some() {
            where_parts.push(format!(
                "id IN (SELECT rowid FROM midi_files_fts WHERE midi_files_fts MATCH ?{bind_idx} AND rowid IN (SELECT MAX(id) FROM midi_files GROUP BY path))",
            ));
            bind_idx += 1;
        } else if like_pat.is_some() {
            where_parts.push(format!("(name LIKE ?{bind_idx} ESCAPE '\\' OR path LIKE ?{bind_idx} ESCAPE '\\')"));
            bind_idx += 1;
        }
        if let Some(f) = format_filter {
            if !f.is_empty() && f != "all" {
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

        let total_count: u64 = {
            let sql = format!("SELECT COUNT(*) FROM midi_files WHERE {where_cl}");
            let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
            let mut bi = 1;
            if let Some(ref m) = fts_match {
                stmt.raw_bind_parameter(bi, m).map_err(|e| e.to_string())?;
                bi += 1;
            } else if let Some(ref pat) = like_pat {
                stmt.raw_bind_parameter(bi, pat).map_err(|e| e.to_string())?;
                bi += 1;
            }
            if let Some(f) = format_filter {
                if !f.is_empty() && f != "all" && !f.contains(',') {
                    stmt.raw_bind_parameter(bi, f).map_err(|e| e.to_string())?;
                }
            }
            let mut rows = stmt.raw_query();
            rows.next()
                .map_err(|e| e.to_string())?
                .map(|r| r.get::<_, i64>(0).unwrap_or(0) as u64)
                .unwrap_or(0)
        };

        let sql = format!(
            "SELECT name, path, directory, format, size, size_formatted, modified
             FROM midi_files WHERE {where_cl}
             ORDER BY {sort_col} {dir} LIMIT ?{limit_idx} OFFSET ?{off_idx}",
            limit_idx = bind_idx,
            off_idx = bind_idx + 1
        );
        let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
        let mut bi = 1;
        if let Some(ref m) = fts_match {
            stmt.raw_bind_parameter(bi, m).map_err(|e| e.to_string())?;
            bi += 1;
        } else if let Some(ref pat) = like_pat {
            stmt.raw_bind_parameter(bi, pat).map_err(|e| e.to_string())?;
            bi += 1;
        }
        if let Some(f) = format_filter {
            if !f.is_empty() && f != "all" && !f.contains(',') {
                stmt.raw_bind_parameter(bi, f).map_err(|e| e.to_string())?;
                bi += 1;
            }
        }
        stmt.raw_bind_parameter(bi, limit as i64)
            .map_err(|e| e.to_string())?;
        stmt.raw_bind_parameter(bi + 1, offset as i64)
            .map_err(|e| e.to_string())?;
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
            total_unfiltered,
        })
    }

    pub fn midi_filter_stats(
        &self,
        search: Option<&str>,
        format_filter: Option<&str>,
    ) -> Result<FilterStatsResult, String> {
        let conn = self.conn.lock().unwrap();
        let total_unfiltered: u64 = conn
            .query_row(
                "SELECT COUNT(*) FROM midi_files WHERE id IN (SELECT MAX(id) FROM midi_files GROUP BY path)",
                [],
                |r| r.get::<_, i64>(0).map(|v| v as u64),
            )
            .unwrap_or(0);
        let fts_match = search.and_then(fts_phrase);
        let like_pat = search.and_then(short_like);
        let mut where_parts = vec![MIDI_LIBRARY_IDS.to_string()];
        let mut bind_idx = 1usize;
        if fts_match.is_some() {
            where_parts.push(format!(
                "id IN (SELECT rowid FROM midi_files_fts WHERE midi_files_fts MATCH ?{bind_idx} AND rowid IN (SELECT MAX(id) FROM midi_files GROUP BY path))",
            ));
            bind_idx += 1;
        } else if like_pat.is_some() {
            where_parts.push(format!("(name LIKE ?{bind_idx} ESCAPE '\\' OR path LIKE ?{bind_idx} ESCAPE '\\')"));
            bind_idx += 1;
        }
        if let Some(f) = format_filter {
            if !f.is_empty() && f != "all" {
                if f.contains(',') {
                    where_parts.push(format!("format IN ({})", Self::in_list_sql(f)));
                } else {
                    where_parts.push(format!("format = ?{bind_idx}"));
                    bind_idx += 1;
                }
            }
        }
        let where_cl = where_parts.join(" AND ");
        let sql = format!(
            "SELECT format, COUNT(*), COALESCE(SUM(size),0) FROM midi_files WHERE {where_cl} GROUP BY format"
        );
        let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
        let mut bi = 1;
        if let Some(ref m) = fts_match {
            stmt.raw_bind_parameter(bi, m).map_err(|e| e.to_string())?;
            bi += 1;
        } else if let Some(ref pat) = like_pat {
            stmt.raw_bind_parameter(bi, pat).map_err(|e| e.to_string())?;
            bi += 1;
        }
        if let Some(f) = format_filter {
            if !f.is_empty() && f != "all" && !f.contains(',') {
                stmt.raw_bind_parameter(bi, f).map_err(|e| e.to_string())?;
            }
        }
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
            total_bytes,
            by_type,
            bytes_by_type,
            total_unfiltered,
        })
    }

    // ── PDF scan CRUD ──

    pub fn pdf_scan_parent_create(
        &self,
        id: &str,
        timestamp: &str,
        roots: &[String],
    ) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        let roots_json = serde_json::to_string(roots).unwrap_or_default();
        conn.execute(
            "INSERT OR REPLACE INTO pdf_scans (id, timestamp, pdf_count, total_bytes, roots) VALUES (?1,?2,0,0,?3)",
            params![id, timestamp, roots_json],
        ).map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM pdfs WHERE scan_id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM pdfs_fts WHERE scan_id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn pdf_scan_parent_finalize(
        &self,
        id: &str,
        _pdf_count: usize,
        _total_bytes: u64,
    ) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        let pdf_count: i64 = conn
            .query_row(
                "SELECT COUNT(DISTINCT path) FROM pdfs",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);
        let total_bytes: i64 = conn
            .query_row(
                "SELECT COALESCE(SUM(size), 0) FROM pdfs WHERE id IN (SELECT MAX(id) FROM pdfs GROUP BY path)",
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
        let conn = self.conn.lock().unwrap();
        let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
        let mut inserted: u64 = 0;
        let mut batch_bytes: u64 = 0;
        {
            let mut stmt = tx.prepare_cached("INSERT OR IGNORE INTO pdfs (name, path, directory, size, size_formatted, modified, scan_id) VALUES (?1,?2,?3,?4,?5,?6,?7)").map_err(|e| e.to_string())?;
            let mut fts_stmt = tx.prepare_cached("INSERT INTO pdfs_fts(rowid, name, path, scan_id) VALUES (?1,?2,?3,?4)").map_err(|e| e.to_string())?;
            for p in pdfs {
                let changed = stmt.execute(params![
                    p.name,
                    p.path,
                    p.directory,
                    p.size as i64,
                    p.size_formatted,
                    p.modified,
                    scan_id
                ])
                .map_err(|e| e.to_string())?;
                if changed > 0 {
                    let id = tx.last_insert_rowid();
                    fts_stmt.execute(params![id, p.name, p.path, scan_id]).map_err(|e| e.to_string())?;
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
        Ok(inserted)
    }

    // ── Directory scan state (incremental unified walker) ──

    /// Load stored directory mtimes for a domain (e.g. `"unified"`).
    pub fn load_directory_scan_snapshot(
        &self,
        domain: &str,
    ) -> Result<HashMap<String, i64>, String> {
        let conn = self.conn.lock().unwrap();
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
        let conn = self.conn.lock().unwrap();
        let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
        {
            let mut stmt = tx
                .prepare_cached(
                    "INSERT OR REPLACE INTO directory_scan_state (domain, path, mtime_secs, last_scan_id) VALUES (?1, ?2, ?3, ?4)",
                )
                .map_err(|e| e.to_string())?;
            for (path, mtime_secs) in rows {
                stmt.execute(params![domain, path, mtime_secs, last_scan_id])
                    .map_err(|e| e.to_string())?;
            }
        }
        tx.commit().map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn save_pdf_scan(&self, snap: &PdfScanSnapshot) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        let roots_json = serde_json::to_string(&snap.roots).unwrap_or_default();
        conn.execute(
            "INSERT OR REPLACE INTO pdf_scans (id, timestamp, pdf_count, total_bytes, roots) VALUES (?1,?2,?3,?4,?5)",
            params![snap.id, snap.timestamp, snap.pdf_count as i64, snap.total_bytes as i64, roots_json],
        ).map_err(|e| e.to_string())?;
        let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
        {
            tx.execute("DELETE FROM pdfs WHERE scan_id = ?1", params![snap.id])
                .map_err(|e| e.to_string())?;
            tx.execute("DELETE FROM pdfs_fts WHERE scan_id = ?1", params![snap.id])
                .map_err(|e| e.to_string())?;
            let mut stmt = tx.prepare_cached("INSERT OR IGNORE INTO pdfs (name, path, directory, size, size_formatted, modified, scan_id) VALUES (?1,?2,?3,?4,?5,?6,?7)").map_err(|e| e.to_string())?;
            let mut fts_stmt = tx.prepare_cached("INSERT INTO pdfs_fts(rowid, name, path, scan_id) VALUES (?1,?2,?3,?4)").map_err(|e| e.to_string())?;
            for p in &snap.pdfs {
                let changed = stmt.execute(params![
                    p.name,
                    p.path,
                    p.directory,
                    p.size as i64,
                    p.size_formatted,
                    p.modified,
                    snap.id
                ])
                .map_err(|e| e.to_string())?;
                if changed > 0 {
                    let id = tx.last_insert_rowid();
                    fts_stmt.execute(params![id, p.name, p.path, snap.id]).map_err(|e| e.to_string())?;
                }
            }
        }
        tx.commit().map_err(|e| e.to_string())
    }

    pub fn get_pdf_scans(&self) -> Result<Vec<serde_json::Value>, String> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT s.id, s.timestamp, COALESCE(NULLIF(s.pdf_count,0),(SELECT COUNT(*) FROM pdfs WHERE scan_id = s.id)), COALESCE(NULLIF(s.total_bytes,0),(SELECT COALESCE(SUM(size),0) FROM pdfs WHERE scan_id = s.id)), s.roots FROM pdf_scans s ORDER BY s.timestamp DESC",
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
        let conn = self.conn.lock().unwrap();
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
        let conn = self.conn.lock().unwrap();
        let id: Option<String> = conn
            .query_row(
                "SELECT id FROM pdf_scans ORDER BY timestamp DESC LIMIT 1",
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
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM pdfs WHERE scan_id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM pdf_scans WHERE id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn clear_pdf_history(&self) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch("DELETE FROM pdfs; DELETE FROM pdf_scans;")
            .map_err(|e| e.to_string())
    }

    // ── PDF metadata (page count) ──

    /// Return paths from the latest PDF scan that don't yet have metadata cached.
    pub fn unindexed_pdf_paths(&self, limit: u64) -> Result<Vec<String>, String> {
        let conn = self.conn.lock().unwrap();
        let scan_id: String = conn
            .query_row(
                "SELECT id FROM pdf_scans ORDER BY timestamp DESC LIMIT 1",
                [],
                |r| r.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())?
            .unwrap_or_default();
        if scan_id.is_empty() {
            return Ok(vec![]);
        }
        let mut stmt = conn
            .prepare(
                "SELECT p.path FROM pdfs p
             LEFT JOIN pdf_metadata m ON m.path = p.path
             WHERE p.scan_id = ?1 AND m.path IS NULL
             LIMIT ?2",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![scan_id, limit as i64], |row| {
                row.get::<_, String>(0)
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    }

    /// Batch upsert PDF page counts. Entries with None page count are still
    /// inserted (as a negative marker) so we don't re-attempt broken files.
    pub fn save_pdf_metadata(&self, batch: &[(String, Option<u32>)]) -> Result<(), String> {
        if batch.is_empty() {
            return Ok(());
        }
        let conn = self.conn.lock().unwrap();
        let now = chrono::Utc::now()
            .format("%Y-%m-%dT%H:%M:%S%.3fZ")
            .to_string();
        let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
        {
            let mut stmt = tx
                .prepare_cached("INSERT OR REPLACE INTO pdf_metadata (path, pages, updated_at) VALUES (?1, ?2, ?3)")
                .map_err(|e| e.to_string())?;
            for (path, pages) in batch {
                let pages_i: Option<i64> = pages.map(|n| n as i64);
                stmt.execute(params![path, pages_i, now])
                    .map_err(|e| e.to_string())?;
            }
        }
        tx.commit().map_err(|e| e.to_string())
    }

    /// Get page counts for a set of paths (returns only entries that exist).
    pub fn get_pdf_metadata(
        &self,
        paths: &[String],
    ) -> Result<std::collections::HashMap<String, Option<u32>>, String> {
        if paths.is_empty() {
            return Ok(std::collections::HashMap::new());
        }
        let conn = self.conn.lock().unwrap();
        let mut out = std::collections::HashMap::new();
        // SQLite IN clause with ~999 param limit — chunk to be safe.
        for chunk in paths.chunks(500) {
            let placeholders: Vec<String> = (1..=chunk.len()).map(|i| format!("?{i}")).collect();
            let sql = format!(
                "SELECT path, pages FROM pdf_metadata WHERE path IN ({})",
                placeholders.join(",")
            );
            let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
            for (i, p) in chunk.iter().enumerate() {
                stmt.raw_bind_parameter(i + 1, p)
                    .map_err(|e| e.to_string())?;
            }
            let mut rows = stmt.raw_query();
            while let Some(row) = rows.next().map_err(|e| e.to_string())? {
                let path: String = row.get(0).unwrap_or_default();
                let pages: Option<i64> = row.get(1).ok();
                out.insert(
                    path,
                    pages.and_then(|n| if n >= 0 { Some(n as u32) } else { None }),
                );
            }
        }
        Ok(out)
    }

    pub fn clear_pdf_metadata(&self) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch("DELETE FROM pdf_metadata;")
            .map_err(|e| e.to_string())
    }

    // ── Paginated PDF query ──
    pub fn query_pdfs(
        &self,
        search: Option<&str>,
        sort_key: &str,
        sort_asc: bool,
        offset: u64,
        limit: u64,
    ) -> Result<PdfQueryResult, String> {
        let conn = self.conn.lock().unwrap();
        let total_unfiltered: u64 = conn
            .query_row(
                "SELECT COUNT(*) FROM pdfs WHERE id IN (SELECT MAX(id) FROM pdfs GROUP BY path)",
                [],
                |row| row.get::<_, i64>(0).map(|v| v as u64),
            )
            .unwrap_or(0);
        if total_unfiltered == 0 {
            return Ok(PdfQueryResult {
                pdfs: vec![],
                total_count: 0,
                total_unfiltered: 0,
            });
        }

        let mut where_parts = vec![PDF_LIBRARY_IDS.to_string()];
        let mut bind_idx = 1usize;
        let fts_match = search.and_then(fts_phrase);
        let like_pat = search.and_then(short_like);
        if fts_match.is_some() {
            where_parts.push(format!(
                "id IN (SELECT rowid FROM pdfs_fts WHERE pdfs_fts MATCH ?{bind_idx} AND rowid IN (SELECT MAX(id) FROM pdfs GROUP BY path))",
            ));
            bind_idx += 1;
        } else if like_pat.is_some() {
            where_parts.push(format!("(name LIKE ?{bind_idx} ESCAPE '\\' OR path LIKE ?{bind_idx} ESCAPE '\\')"));
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

        let total_count: u64 = {
            let sql = format!("SELECT COUNT(*) FROM pdfs WHERE {where_cl}");
            let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
            let mut bi = 1;
            if let Some(ref m) = fts_match {
                stmt.raw_bind_parameter(bi, m).map_err(|e| e.to_string())?;
                bi += 1;
            } else if let Some(ref pat) = like_pat {
                stmt.raw_bind_parameter(bi, pat).map_err(|e| e.to_string())?;
                bi += 1;
            }
            let mut rows = stmt.raw_query();
            rows.next()
                .map_err(|e| e.to_string())?
                .map(|r| r.get::<_, i64>(0).unwrap_or(0) as u64)
                .unwrap_or(0)
        };

        let sql = format!("SELECT name, path, directory, size, size_formatted, modified FROM pdfs WHERE {where_cl} ORDER BY {sort_col} {dir} LIMIT ?{bind_idx} OFFSET ?{}", bind_idx + 1);
        let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
        let mut bi = 1;
        if let Some(ref m) = fts_match {
            stmt.raw_bind_parameter(bi, m).map_err(|e| e.to_string())?;
            bi += 1;
        } else if let Some(ref pat) = like_pat {
            stmt.raw_bind_parameter(bi, pat).map_err(|e| e.to_string())?;
            bi += 1;
        }
        stmt.raw_bind_parameter(bi, limit as i64)
            .map_err(|e| e.to_string())?;
        stmt.raw_bind_parameter(bi + 1, offset as i64)
            .map_err(|e| e.to_string())?;

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
            total_unfiltered,
        })
    }

    /// PDF aggregate stats. `scan_id` None or empty → full library (deduped by path).
    pub fn pdf_stats(&self, scan_id: Option<&str>) -> Result<PdfStatsResult, String> {
        let conn = self.conn.lock().unwrap();
        let library = scan_id.map(|s| s.is_empty()).unwrap_or(true);
        if library {
            let pdf_count: u64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM pdfs WHERE id IN (SELECT MAX(id) FROM pdfs GROUP BY path)",
                    [],
                    |row| row.get::<_, i64>(0).map(|v| v as u64),
                )
                .unwrap_or(0);
            let total_bytes: u64 = conn
                .query_row(
                    "SELECT COALESCE(SUM(size), 0) FROM pdfs WHERE id IN (SELECT MAX(id) FROM pdfs GROUP BY path)",
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
    // search/filter. Uses a single GROUP BY query for the breakdown (O(latest_scan)).

    fn fzf_like(s: Option<&str>) -> Option<String> {
        s.and_then(|t| {
            let t = t.trim();
            if t.is_empty() {
                return None;
            }
            let esc: String = t
                .chars()
                .map(|c| match c {
                    '%' => "\\%".to_string(),
                    '_' => "\\_".to_string(),
                    _ => c.to_string(),
                })
                .collect::<Vec<_>>()
                .join("%");
            Some(format!("%{esc}%"))
        })
    }

    fn in_list_sql(values: &str) -> String {
        values
            .split(',')
            .map(|s| format!("'{}'", s.trim().replace('\'', "''")))
            .collect::<Vec<_>>()
            .join(",")
    }

    fn get_latest_scan_id(&self, scan_table: &str, _count_col: &str) -> Result<String, String> {
        let conn = self.conn.lock().unwrap();
        Ok(conn
            .query_row(
                &format!("SELECT id FROM {scan_table} ORDER BY timestamp DESC LIMIT 1"),
                [],
                |r| r.get::<_, String>(0),
            )
            .optional()
            .map_err(|e| e.to_string())?
            .unwrap_or_default())
    }

    pub fn audio_filter_stats(
        &self,
        search: Option<&str>,
        format_filter: Option<&str>,
    ) -> Result<FilterStatsResult, String> {
        let conn = self.conn.lock().unwrap();
        let total_unfiltered: u64 = conn
            .query_row(
                "SELECT COUNT(*) FROM audio_samples WHERE id IN (SELECT MAX(id) FROM audio_samples GROUP BY path)",
                [],
                |r| r.get::<_, i64>(0).map(|v| v as u64),
            )
            .unwrap_or(0);
        let fts_match = search.and_then(fts_phrase);
        let like_pat = search.and_then(short_like);
        let mut where_parts = vec![AUDIO_LIBRARY_IDS.to_string()];
        let mut bind_idx = 1usize;
        if fts_match.is_some() {
            where_parts.push(format!(
                "id IN (SELECT rowid FROM audio_samples_fts WHERE audio_samples_fts MATCH ?{bind_idx} AND rowid IN (SELECT MAX(id) FROM audio_samples GROUP BY path))",
            ));
            bind_idx += 1;
        } else if like_pat.is_some() {
            where_parts.push(format!("(name LIKE ?{bind_idx} ESCAPE '\\' OR path LIKE ?{bind_idx} ESCAPE '\\')"));
            bind_idx += 1;
        }
        if let Some(f) = format_filter {
            if !f.is_empty() && f != "all" {
                if f.contains(',') {
                    where_parts.push(format!("format IN ({})", Self::in_list_sql(f)));
                } else {
                    where_parts.push(format!("format = ?{bind_idx}"));
                    bind_idx += 1;
                }
            }
        }
        let where_cl = where_parts.join(" AND ");
        let sql = format!("SELECT format, COUNT(*), COALESCE(SUM(size),0) FROM audio_samples WHERE {where_cl} GROUP BY format");
        let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
        let mut bi = 1;
        if let Some(ref m) = fts_match {
            stmt.raw_bind_parameter(bi, m).map_err(|e| e.to_string())?;
            bi += 1;
        } else if let Some(ref pat) = like_pat {
            stmt.raw_bind_parameter(bi, pat).map_err(|e| e.to_string())?;
            bi += 1;
        }
        if let Some(f) = format_filter {
            if !f.is_empty() && f != "all" && !f.contains(',') {
                stmt.raw_bind_parameter(bi, f).map_err(|e| e.to_string())?;
            }
        }
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
            total_bytes,
            by_type,
            bytes_by_type,
            total_unfiltered,
        })
    }

    pub fn daw_filter_stats(
        &self,
        search: Option<&str>,
        daw_filter: Option<&str>,
    ) -> Result<FilterStatsResult, String> {
        let conn = self.conn.lock().unwrap();
        let total_unfiltered: u64 = conn
            .query_row(
                "SELECT COUNT(*) FROM daw_projects WHERE id IN (SELECT MAX(id) FROM daw_projects GROUP BY path)",
                [],
                |r| r.get::<_, i64>(0).map(|v| v as u64),
            )
            .unwrap_or(0);
        let fts_match = search.and_then(fts_phrase);
        let like_pat = search.and_then(short_like);
        let mut where_parts = vec![DAW_LIBRARY_IDS.to_string()];
        let mut bind_idx = 1usize;
        if fts_match.is_some() {
            where_parts.push(format!(
                "id IN (SELECT rowid FROM daw_projects_fts WHERE daw_projects_fts MATCH ?{bind_idx} AND rowid IN (SELECT MAX(id) FROM daw_projects GROUP BY path))",
            ));
            bind_idx += 1;
        } else if like_pat.is_some() {
            where_parts.push(format!("(name LIKE ?{bind_idx} ESCAPE '\\' OR path LIKE ?{bind_idx} ESCAPE '\\')"));
            bind_idx += 1;
        }
        if let Some(f) = daw_filter {
            if !f.is_empty() && f != "all" {
                if f.contains(',') {
                    where_parts.push(format!("daw IN ({})", Self::in_list_sql(f)));
                } else {
                    where_parts.push(format!("daw = ?{bind_idx}"));
                    bind_idx += 1;
                }
            }
        }
        let where_cl = where_parts.join(" AND ");
        let sql = format!("SELECT daw, COUNT(*), COALESCE(SUM(size),0) FROM daw_projects WHERE {where_cl} GROUP BY daw");
        let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
        let mut bi = 1;
        if let Some(ref m) = fts_match {
            stmt.raw_bind_parameter(bi, m).map_err(|e| e.to_string())?;
            bi += 1;
        } else if let Some(ref pat) = like_pat {
            stmt.raw_bind_parameter(bi, pat).map_err(|e| e.to_string())?;
            bi += 1;
        }
        if let Some(f) = daw_filter {
            if !f.is_empty() && f != "all" && !f.contains(',') {
                stmt.raw_bind_parameter(bi, f).map_err(|e| e.to_string())?;
            }
        }
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
            total_bytes,
            by_type,
            bytes_by_type,
            total_unfiltered,
        })
    }

    pub fn preset_filter_stats(
        &self,
        search: Option<&str>,
        format_filter: Option<&str>,
    ) -> Result<FilterStatsResult, String> {
        let conn = self.conn.lock().unwrap();
        let total_unfiltered: u64 = conn
            .query_row(
                "SELECT COUNT(*) FROM presets WHERE id IN (SELECT MAX(id) FROM presets GROUP BY path) AND format NOT IN ('MID','MIDI')",
                [],
                |r| r.get::<_, i64>(0).map(|v| v as u64),
            )
            .unwrap_or(0);
        let fts_match = search.and_then(fts_phrase);
        let like_pat = search.and_then(short_like);
        let mut where_parts = vec![
            PRESET_LIBRARY_IDS.to_string(),
            "format NOT IN ('MID','MIDI')".to_string(),
        ];
        let mut bind_idx = 1usize;
        if fts_match.is_some() {
            where_parts.push(format!(
                "id IN (SELECT rowid FROM presets_fts WHERE presets_fts MATCH ?{bind_idx} AND rowid IN (SELECT MAX(id) FROM presets GROUP BY path))",
            ));
            bind_idx += 1;
        } else if like_pat.is_some() {
            where_parts.push(format!("(name LIKE ?{bind_idx} ESCAPE '\\' OR path LIKE ?{bind_idx} ESCAPE '\\')"));
            bind_idx += 1;
        }
        if let Some(f) = format_filter {
            if !f.is_empty() && f != "all" {
                if f.contains(',') {
                    where_parts.push(format!("format IN ({})", Self::in_list_sql(f)));
                } else {
                    where_parts.push(format!("format = ?{bind_idx}"));
                    bind_idx += 1;
                }
            }
        }
        let where_cl = where_parts.join(" AND ");
        let sql = format!("SELECT format, COUNT(*), COALESCE(SUM(size),0) FROM presets WHERE {where_cl} GROUP BY format");
        let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
        let mut bi = 1;
        if let Some(ref m) = fts_match {
            stmt.raw_bind_parameter(bi, m).map_err(|e| e.to_string())?;
            bi += 1;
        } else if let Some(ref pat) = like_pat {
            stmt.raw_bind_parameter(bi, pat).map_err(|e| e.to_string())?;
            bi += 1;
        }
        if let Some(f) = format_filter {
            if !f.is_empty() && f != "all" && !f.contains(',') {
                stmt.raw_bind_parameter(bi, f).map_err(|e| e.to_string())?;
            }
        }
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
            total_bytes,
            by_type,
            bytes_by_type,
            total_unfiltered,
        })
    }

    pub fn plugin_filter_stats(
        &self,
        search: Option<&str>,
        type_filter: Option<&str>,
    ) -> Result<FilterStatsResult, String> {
        let conn = self.conn.lock().unwrap();
        let total_unfiltered: u64 = conn
            .query_row(
                "SELECT COUNT(*) FROM plugins WHERE id IN (SELECT MAX(id) FROM plugins GROUP BY path)",
                [],
                |r| r.get::<_, i64>(0).map(|v| v as u64),
            )
            .unwrap_or(0);
        let search_pat = Self::fzf_like(search);
        let mut where_parts = vec![PLUGIN_LIBRARY_IDS.to_string()];
        let mut bind_idx = 1usize;
        if search_pat.is_some() {
            where_parts.push(format!("(name LIKE ?{bind_idx} ESCAPE '\\' OR manufacturer LIKE ?{bind_idx} ESCAPE '\\' OR path LIKE ?{bind_idx} ESCAPE '\\')"));
            bind_idx += 1;
        }
        if let Some(tf) = type_filter {
            if !tf.is_empty() && tf != "all" {
                if tf.contains(',') {
                    where_parts.push(format!("plugin_type IN ({})", Self::in_list_sql(tf)));
                } else {
                    where_parts.push(format!("plugin_type = ?{bind_idx}"));
                    bind_idx += 1;
                }
            }
        }
        let where_cl = where_parts.join(" AND ");
        let sql = format!("SELECT plugin_type, COUNT(*), COALESCE(SUM(size_bytes),0) FROM plugins WHERE {where_cl} GROUP BY plugin_type");
        let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
        let mut bi = 1;
        if let Some(ref p) = search_pat {
            stmt.raw_bind_parameter(bi, p).map_err(|e| e.to_string())?;
            bi += 1;
        }
        if let Some(tf) = type_filter {
            if !tf.is_empty() && tf != "all" && !tf.contains(',') {
                stmt.raw_bind_parameter(bi, tf).map_err(|e| e.to_string())?;
            }
        }
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
            total_bytes,
            by_type,
            bytes_by_type,
            total_unfiltered,
        })
    }

    pub fn pdf_filter_stats(&self, search: Option<&str>) -> Result<FilterStatsResult, String> {
        let conn = self.conn.lock().unwrap();
        let total_unfiltered: u64 = conn
            .query_row(
                "SELECT COUNT(*) FROM pdfs WHERE id IN (SELECT MAX(id) FROM pdfs GROUP BY path)",
                [],
                |r| r.get::<_, i64>(0).map(|v| v as u64),
            )
            .unwrap_or(0);
        let fts_match = search.and_then(fts_phrase);
        let like_pat = search.and_then(short_like);
        let sql = if fts_match.is_some() {
            "SELECT COUNT(*), COALESCE(SUM(size),0) FROM pdfs WHERE id IN (SELECT MAX(id) FROM pdfs GROUP BY path) AND id IN (SELECT rowid FROM pdfs_fts WHERE pdfs_fts MATCH ?1 AND rowid IN (SELECT MAX(id) FROM pdfs GROUP BY path))"
        } else if like_pat.is_some() {
            "SELECT COUNT(*), COALESCE(SUM(size),0) FROM pdfs WHERE id IN (SELECT MAX(id) FROM pdfs GROUP BY path) AND (name LIKE ?1 ESCAPE '\\' OR path LIKE ?1 ESCAPE '\\')"
        } else {
            "SELECT COUNT(*), COALESCE(SUM(size),0) FROM pdfs WHERE id IN (SELECT MAX(id) FROM pdfs GROUP BY path)"
        };
        let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;
        if let Some(ref m) = fts_match {
            stmt.raw_bind_parameter(1, m).map_err(|e| e.to_string())?;
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
            total_bytes,
            by_type: HashMap::new(),
            bytes_by_type: HashMap::new(),
            total_unfiltered,
        })
    }

    // ── KVR cache ──

    pub fn load_kvr_cache(&self) -> Result<HashMap<String, KvrCacheEntry>, String> {
        let conn = self.conn.lock().unwrap();
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
        let conn = self.conn.lock().unwrap();
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
        let conn = self.conn.lock().unwrap();
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
        let conn = self.conn.lock().unwrap();
        let col = match field {
            "bpm" => "bpm",
            "key" => "key_name",
            "lufs" => "lufs",
            _ => return Ok(()),
        };
        let sql = format!("UPDATE audio_samples SET {col} = ?1 WHERE path = ?2");
        let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
        {
            let mut stmt = tx.prepare_cached(&sql).map_err(|e| e.to_string())?;
            for (path, val) in obj {
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
        let conn = self.conn.lock().unwrap();
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
        let conn = self.conn.lock().unwrap();
        let sql = format!("INSERT OR REPLACE INTO {table} ({key_col}, {val_col}) VALUES (?1, ?2)");
        let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
        {
            let mut stmt = tx.prepare_cached(&sql).map_err(|e| e.to_string())?;
            for (k, v) in obj {
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

    /// Get row counts for all tables.
    pub fn table_counts(&self) -> Result<serde_json::Value, String> {
        let conn = self.conn.lock().unwrap();
        let tables = [
            "audio_samples",
            "audio_scans",
            "plugins",
            "plugin_scans",
            "daw_projects",
            "daw_scans",
            "presets",
            "preset_scans",
            "pdfs",
            "pdf_scans",
            "pdf_metadata",
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
        Ok(serde_json::Value::Object(map))
    }

    /// Row counts per inventory category for the **library** view: one canonical row per `path`
    /// (`id IN (SELECT MAX(id) FROM … GROUP BY path)`), not scoped to a single `scan_id`. Presets
    /// exclude `MID`/`MIDI` (same tab rules as elsewhere). This matches default `scan_id` handling
    /// on paginated queries and `*_filter_stats`, not raw `COUNT(*)` on whole tables.
    pub fn active_scan_inventory_counts(&self) -> Result<serde_json::Value, String> {
        let conn = self.conn.lock().unwrap();
        let count_plugins: u64 = conn
            .query_row(
                "SELECT COUNT(*) FROM plugins WHERE id IN (SELECT MAX(id) FROM plugins GROUP BY path)",
                [],
                |r| r.get::<_, i64>(0).map(|v| v as u64),
            )
            .unwrap_or(0);
        let count_audio: u64 = conn
            .query_row(
                "SELECT COUNT(*) FROM audio_samples WHERE id IN (SELECT MAX(id) FROM audio_samples GROUP BY path)",
                [],
                |r| r.get::<_, i64>(0).map(|v| v as u64),
            )
            .unwrap_or(0);
        let count_daw: u64 = conn
            .query_row(
                "SELECT COUNT(*) FROM daw_projects WHERE id IN (SELECT MAX(id) FROM daw_projects GROUP BY path)",
                [],
                |r| r.get::<_, i64>(0).map(|v| v as u64),
            )
            .unwrap_or(0);
        let count_presets: u64 = conn
            .query_row(
                "SELECT COUNT(*) FROM presets WHERE id IN (SELECT MAX(id) FROM presets GROUP BY path) AND format NOT IN ('MID','MIDI')",
                [],
                |r| r.get::<_, i64>(0).map(|v| v as u64),
            )
            .unwrap_or(0);
        let count_pdfs: u64 = conn
            .query_row(
                "SELECT COUNT(*) FROM pdfs WHERE id IN (SELECT MAX(id) FROM pdfs GROUP BY path)",
                [],
                |r| r.get::<_, i64>(0).map(|v| v as u64),
            )
            .unwrap_or(0);
        let count_midi: u64 = conn
            .query_row(
                "SELECT COUNT(*) FROM midi_files WHERE id IN (SELECT MAX(id) FROM midi_files GROUP BY path)",
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
        }))
    }

    /// Get stats for all caches: item count and estimated size.
    pub fn cache_stats(&self) -> Result<Vec<CacheStat>, String> {
        let conn = self.conn.lock().unwrap();
        let page_size: u64 = conn
            .query_row("PRAGMA page_size", [], |r| r.get::<_, i64>(0))
            .unwrap_or(4096) as u64;
        let mut stats = Vec::new();

        // Analysis caches (columns on audio_samples — library rows only)
        let total_samples: u64 = conn
            .query_row(
                &format!("SELECT COUNT(*) FROM audio_samples WHERE {AUDIO_LIBRARY_IDS}"),
                [],
                |r| r.get::<_, i64>(0).map(|v| v as u64),
            )
            .unwrap_or(0);
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
            stats.push(CacheStat {
                key: key.into(),
                label: label.into(),
                count,
                total: total_samples,
                size_bytes: count * 8,
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

        // Scan histories
        for (label, scan_table, item_table, key) in [
            ("Plugin Scans", "plugin_scans", "plugins", "plugin_scans"),
            ("Audio Scans", "audio_scans", "audio_samples", "audio_scans"),
            ("DAW Scans", "daw_scans", "daw_projects", "daw_scans"),
            ("Preset Scans", "preset_scans", "presets", "preset_scans"),
            ("MIDI Scans", "midi_scans", "midi_files", "midi_scans"),
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
            // Estimate size from number of items * avg row size
            let avg_row: u64 = if item_count > 0 {
                let pages: u64 = conn
                    .query_row("SELECT page_count FROM pragma_page_count()", [], |r| {
                        r.get::<_, i64>(0).map(|v| v as u64)
                    })
                    .unwrap_or(0);
                if pages > 0 {
                    (pages * page_size) / item_count.max(1)
                } else {
                    200
                }
            } else {
                0
            };
            stats.push(CacheStat {
                key: key.into(),
                label: label.into(),
                count: item_count,
                total: scan_count,
                size_bytes: item_count * avg_row,
            });
        }

        // Total DB file size
        let db_path = history::get_data_dir().join("audio_haxor.db");
        let db_size = std::fs::metadata(&db_path).map(|m| m.len()).unwrap_or(0);
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
    pub fn batch_update_analysis(&self, results: &[AnalysisBatchRow]) -> Result<u32, String> {
        let conn = self.conn.lock().unwrap();
        let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
        let mut count = 0u32;
        {
            let mut stmt = tx.prepare_cached(
                "UPDATE audio_samples SET bpm = ?1, key_name = ?2, lufs = ?3 WHERE path = ?4"
            ).map_err(|e| e.to_string())?;
            for (path, bpm, key, lufs) in results {
                let _ = stmt.execute(params![bpm, key, lufs, path]);
                count += 1;
            }
        }
        tx.commit().map_err(|e| e.to_string())?;
        Ok(count)
    }

    /// Clear a specific cache table.
    pub fn clear_cache_table(&self, table: &str) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        let sql = match table {
            "bpm" => "UPDATE audio_samples SET bpm = NULL",
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
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(
            "UPDATE audio_samples SET bpm = NULL, key_name = NULL, lufs = NULL;
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
            let conn = self.conn.lock().unwrap();
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
        let conn = self.conn.lock().unwrap();
        let mut count = 0;
        for snap in &history.scans {
            let dirs_json = serde_json::to_string(&snap.directories).unwrap_or_default();
            let roots_json = serde_json::to_string(&snap.roots).unwrap_or_default();
            conn.execute(
                "INSERT OR REPLACE INTO plugin_scans (id, timestamp, plugin_count, directories, roots) VALUES (?1,?2,?3,?4,?5)",
                params![snap.id, snap.timestamp, snap.plugin_count as i64, dirs_json, roots_json],
            ).map_err(|e| e.to_string())?;

            let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
            {
                let mut stmt = tx.prepare_cached(
                    "INSERT OR REPLACE INTO plugins (name, path, plugin_type, version, manufacturer, manufacturer_url, size, size_bytes, modified, architectures, scan_id) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)"
                ).map_err(|e| e.to_string())?;
                for p in &snap.plugins {
                    let arch_json = serde_json::to_string(&p.architectures).unwrap_or_default();
                    stmt.execute(params![
                        p.name,
                        p.path,
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
        let conn = self.conn.lock().unwrap();
        let mut count = 0;
        for snap in &history.scans {
            let daw_json = serde_json::to_string(&snap.daw_counts).unwrap_or_default();
            let roots_json = serde_json::to_string(&snap.roots).unwrap_or_default();
            conn.execute(
                "INSERT OR REPLACE INTO daw_scans (id, timestamp, project_count, total_bytes, daw_counts, roots) VALUES (?1,?2,?3,?4,?5,?6)",
                params![snap.id, snap.timestamp, snap.project_count as i64, snap.total_bytes as i64, daw_json, roots_json],
            ).map_err(|e| e.to_string())?;

            let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
            {
                let mut stmt = tx.prepare_cached(
                    "INSERT OR REPLACE INTO daw_projects (name, path, directory, format, daw, size, size_formatted, modified, scan_id) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)"
                ).map_err(|e| e.to_string())?;
                for p in &snap.projects {
                    stmt.execute(params![
                        p.name,
                        p.path,
                        p.directory,
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
        let conn = self.conn.lock().unwrap();
        let mut count = 0;
        for snap in &history.scans {
            let fc_json = serde_json::to_string(&snap.format_counts).unwrap_or_default();
            let roots_json = serde_json::to_string(&snap.roots).unwrap_or_default();
            conn.execute(
                "INSERT OR REPLACE INTO preset_scans (id, timestamp, preset_count, total_bytes, format_counts, roots) VALUES (?1,?2,?3,?4,?5,?6)",
                params![snap.id, snap.timestamp, snap.preset_count as i64, snap.total_bytes as i64, fc_json, roots_json],
            ).map_err(|e| e.to_string())?;

            let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
            {
                let mut stmt = tx.prepare_cached(
                    "INSERT OR REPLACE INTO presets (name, path, directory, format, size, size_formatted, modified, scan_id) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)"
                ).map_err(|e| e.to_string())?;
                for p in &snap.presets {
                    stmt.execute(params![
                        p.name,
                        p.path,
                        p.directory,
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
        let conn = self.conn.lock().unwrap();
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
        let conn = self.conn.lock().unwrap();
        let sql = format!("INSERT OR REPLACE INTO {table} ({key_col}, {val_col}) VALUES (?1, ?2)");
        let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
        let count = cache.len();
        {
            let mut stmt = tx.prepare_cached(&sql).map_err(|e| e.to_string())?;
            for (k, v) in &cache {
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

        let conn = self.conn.lock().unwrap();
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

    /// `history::set_test_data_dir_path` is process-global; serialize migrate JSON tests.
    static MIGRATE_JSON_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

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

    fn test_db() -> Database {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")
            .unwrap();
        let db = Database {
            conn: Mutex::new(conn),
        };
        db.migrate().unwrap();
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
                format_filter: None,
                sort_key: "name".into(),
                sort_asc: true,
                offset: 0,
                limit: 100,
            })
            .unwrap();

        assert_eq!(result.total_count, 2);
    }

    /// Search (name subsequence) + sort by size DESC + pagination: verifies full query_audio path.
    #[test]
    fn test_query_audio_search_subsequence_and_sort_size_desc() {
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
                format_filter: None,
                sort_key: "size".into(),
                sort_asc: false,
                offset: 0,
                limit: 10,
            })
            .unwrap();

        assert_eq!(result.total_count, 2);
        assert_eq!(result.samples[0].name, "big_kick.wav");
        assert_eq!(result.samples[0].size, 9_999);
        assert_eq!(result.samples[1].name, "small_kick.wav");
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
        let st = db.audio_filter_stats(None, None).unwrap();
        assert_eq!(st.count, 0);
        assert_eq!(st.total_bytes, 0);
        assert_eq!(st.total_unfiltered, 0);
        assert!(st.by_type.is_empty());
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

        let st = db.audio_filter_stats(None, None).unwrap();
        assert_eq!(st.total_unfiltered, 3);
        assert_eq!(st.count, 3);
        assert_eq!(st.total_bytes, 700);
        assert_eq!(st.by_type.get("WAV").copied().unwrap_or(0), 2);
        assert_eq!(st.by_type.get("MP3").copied().unwrap_or(0), 1);
        assert_eq!(st.bytes_by_type.get("WAV").copied().unwrap_or(0), 300);
        assert_eq!(st.bytes_by_type.get("MP3").copied().unwrap_or(0), 400);
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

        let st = db.audio_filter_stats(Some("kick"), None).unwrap();
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

        let w = db.audio_filter_stats(None, Some("WAV")).unwrap();
        assert_eq!(w.count, 1);
        assert_eq!(w.total_unfiltered, 3);
        assert_eq!(w.by_type.len(), 1);

        let wm = db.audio_filter_stats(None, Some("WAV,AIFF")).unwrap();
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

        let st = db.audio_filter_stats(None, Some("all")).unwrap();
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

        let unfiltered = db.daw_filter_stats(None, None).unwrap();
        assert_eq!(unfiltered.total_unfiltered, 3);
        assert_eq!(unfiltered.count, 3);
        assert_eq!(
            unfiltered.by_type.get("Ableton Live").copied().unwrap_or(0),
            2
        );
        assert_eq!(unfiltered.by_type.get("Logic Pro").copied().unwrap_or(0), 1);

        let abl = db.daw_filter_stats(None, Some("Ableton Live")).unwrap();
        assert_eq!(abl.count, 2);
        assert_eq!(abl.total_unfiltered, 3);
        assert_eq!(abl.total_bytes, 2000);

        let search = db.daw_filter_stats(Some("a.als"), None).unwrap();
        assert_eq!(search.count, 1);
        assert_eq!(search.total_unfiltered, 3);
    }

    #[test]
    fn test_daw_filter_stats_empty_db() {
        let db = test_db();
        let st = db.daw_filter_stats(None, None).unwrap();
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

        let st = db.preset_filter_stats(None, None).unwrap();
        assert_eq!(st.total_unfiltered, 2);
        assert_eq!(st.count, 2);
        assert_eq!(st.by_type.get("FXP").copied().unwrap_or(0), 2);

        let fx = db.preset_filter_stats(None, Some("FXP")).unwrap();
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

        let st = db.preset_filter_stats(Some("brass"), None).unwrap();
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

        let st = db.plugin_filter_stats(None, None).unwrap();
        assert_eq!(st.total_unfiltered, 3);
        assert_eq!(st.count, 3);
        assert_eq!(st.by_type.get("VST3").copied().unwrap_or(0), 2);

        let vst = db.plugin_filter_stats(None, Some("VST3")).unwrap();
        assert_eq!(vst.count, 2);
        assert_eq!(vst.total_bytes, 2_000_000);

        let xfer = db.plugin_filter_stats(Some("Xfer"), None).unwrap();
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

        let st = db.plugin_filter_stats(None, Some("VST3,AU")).unwrap();
        assert_eq!(st.count, 2);
        assert_eq!(st.total_unfiltered, 2);
    }

    #[test]
    fn test_plugin_filter_stats_empty_db() {
        let db = test_db();
        let st = db.plugin_filter_stats(None, None).unwrap();
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
                },
                PdfFile {
                    name: "readme_extra".into(),
                    path: "/docs/readme_extra.pdf".into(),
                    directory: "/docs".into(),
                    size: 200,
                    size_formatted: "200 B".into(),
                    modified: "2024-06-02".into(),
                },
            ],
            roots: vec!["/docs".into()],
        };
        db.save_pdf_scan(&snap).unwrap();

        let all = db.pdf_filter_stats(None).unwrap();
        assert_eq!(all.total_unfiltered, 2);
        assert_eq!(all.count, 2);
        assert_eq!(all.total_bytes, 300);
        assert!(all.by_type.is_empty());

        let sub = db.pdf_filter_stats(Some("readme")).unwrap();
        assert_eq!(sub.count, 1);
        assert_eq!(sub.total_bytes, 200);
        assert_eq!(sub.total_unfiltered, 2);
    }

    #[test]
    fn test_pdf_filter_stats_empty_db() {
        let db = test_db();
        let st = db.pdf_filter_stats(None).unwrap();
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
    fn test_batch_update_analysis_updates_latest_scan_and_audio_stats() {
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

        let unanalyzed = db.unanalyzed_paths(100).unwrap();
        assert_eq!(unanalyzed.len(), 1);
        assert_eq!(unanalyzed[0], "/b.wav");
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
            .query_plugins(Some("ser"), None, "size", false, 0, 100)
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
        db.daw_scan_parent_create(id, ts, &["/roots".into()]).unwrap();
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
        let scans = db.get_daw_scans().unwrap();
        assert_eq!(scans.len(), 1);
        assert_eq!(scans[0]["projectCount"], 1);
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
            .query_daw(Some("mix"), None, "size", false, 0, 100)
            .unwrap();
        assert_eq!(res.total_count, 2);
        assert_eq!(res.projects[0].name, "big_mix_master.als");
        assert_eq!(res.projects[0].size, 10_000);
        assert_eq!(res.projects[1].name, "small_mix_down.als");
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
        let res = db.query_pdfs(None, "name", true, 0, 100).unwrap();
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
            },
            PdfFile {
                name: "alpha".into(),
                path: "/a/a.pdf".into(),
                directory: "/b".into(),
                size: 50,
                size_formatted: "50 B".into(),
                modified: "2024-01-01".into(),
            },
            PdfFile {
                name: "alpha_notes".into(),
                path: "/a/notes.pdf".into(),
                directory: "/c".into(),
                size: 50,
                size_formatted: "50 B".into(),
                modified: "2024-01-02".into(),
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

        let filtered = db.query_pdfs(Some("alp"), "name", true, 0, 100).unwrap();
        assert_eq!(filtered.total_unfiltered, 3);
        assert_eq!(filtered.total_count, 2);
        assert_eq!(filtered.pdfs.len(), 2);
        assert_eq!(filtered.pdfs[0].name, "alpha");
        assert_eq!(filtered.pdfs[1].name, "alpha_notes");

        let by_size = db.query_pdfs(None, "size", false, 0, 10).unwrap();
        assert_eq!(by_size.pdfs[0].name, "zebra");

        let page = db.query_pdfs(None, "name", true, 1, 1).unwrap();
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
            }],
            roots: vec![],
        })
        .unwrap();
        let res = db.query_pdfs(None, "name", true, 0, 100).unwrap();
        assert_eq!(res.total_unfiltered, 2);
        assert_eq!(res.pdfs.len(), 2);
        assert_eq!(res.pdfs[0].name, "new");
        assert_eq!(res.pdfs[1].name, "old");
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
                },
                PdfFile {
                    name: "b".into(),
                    path: "/p/b.pdf".into(),
                    directory: "/p".into(),
                    size: 200,
                    size_formatted: "200 B".into(),
                    modified: "d".into(),
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
            .query_plugins(Some("nonexistent_xyz"), None, "name", true, 0, 100)
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

        let res = db.query_plugins(None, None, "name", true, 0, 100).unwrap();
        assert_eq!(res.total_count, 2);
        assert_eq!(res.total_unfiltered, 2);
    }

    #[test]
    fn test_query_plugins_total_unfiltered_empty_db() {
        let db = test_db();
        let res = db.query_plugins(None, None, "name", true, 0, 100).unwrap();
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
            .query_daw(None, Some("FL Studio"), "name", true, 0, 100)
            .unwrap();
        assert_eq!(res.total_count, 0);
        assert_eq!(
            res.total_unfiltered, 3,
            "unfiltered count must include all 3 projects in latest scan"
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
            .query_daw(Some("bass"), None, "name", true, 0, 100)
            .unwrap();
        assert_eq!(res.total_count, 1);
        assert_eq!(res.total_unfiltered, 2);
    }

    #[test]
    fn test_query_daw_total_unfiltered_empty_db() {
        let db = test_db();
        let res = db.query_daw(None, None, "name", true, 0, 100).unwrap();
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
            .query_presets(None, Some("H2P"), "name", true, 0, 100)
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

        let res = db.query_presets(None, None, "name", true, 0, 100).unwrap();
        assert_eq!(
            res.total_unfiltered, 1,
            "MIDI files must be excluded from preset header count"
        );
        assert_eq!(res.total_count, 1);
        assert!(res
            .presets
            .iter()
            .all(|p| p.format != "MID" && p.format != "MIDI"));
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
            .query_presets(Some("bass"), None, "name", true, 0, 100)
            .unwrap();
        assert_eq!(res.total_count, 2);
        assert_eq!(res.total_unfiltered, 3);
    }

    /// Subsequence search on name + sort by size DESC (full `query_presets` path).
    #[test]
    fn test_query_presets_search_subsequence_and_sort_size_desc() {
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
            .query_presets(Some("lead"), None, "size", false, 0, 100)
            .unwrap();
        assert_eq!(res.total_count, 2);
        assert_eq!(res.presets[0].name, "big_lead.fxp");
        assert_eq!(res.presets[0].size, 10_000);
        assert_eq!(res.presets[1].name, "small_lead.fxp");
    }

    #[test]
    fn test_query_presets_total_unfiltered_empty_db() {
        let db = test_db();
        let res = db.query_presets(None, None, "name", true, 0, 100).unwrap();
        assert_eq!(res.total_count, 0);
        assert_eq!(res.total_unfiltered, 0);
    }

    // ── Multi-scan semantics ──
    //
    // Each new scan inserts rows with a fresh scan_id (daw_projects/presets/plugins
    // accumulate rows across history). Queries must return the LATEST scan's count,
    // not the cumulative total.

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

        let res = db.query_daw(None, None, "name", true, 0, 100).unwrap();
        assert_eq!(res.total_unfiltered, 5, "library = union of distinct paths");
        assert_eq!(res.total_count, 5);
        assert_eq!(res.projects.len(), 5);
        let names: Vec<_> = res.projects.iter().map(|p| p.name.as_str()).collect();
        assert!(names.contains(&"old1.als"));
        assert!(names.contains(&"new2.als"));
    }

    #[test]
    fn test_query_daw_empty_latest_scan_ignored() {
        // Latest scan is the newest scan with ≥1 `daw_projects` row — a zero-result
        // scan saved after a successful one must NOT clobber the header count.
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

        let res = db.query_daw(None, None, "name", true, 0, 100).unwrap();
        assert_eq!(
            res.total_unfiltered, 1,
            "empty scans with project_count=0 must not hide the real latest scan"
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

        let p1 = db.query_daw(None, None, "name", true, 0, 10).unwrap();
        let p2 = db.query_daw(None, None, "name", true, 10, 10).unwrap();
        let p3 = db.query_daw(None, None, "name", true, 20, 10).unwrap();

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
            .query_daw(Some("bass"), Some("Ableton"), "name", true, 0, 100)
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
            .query_daw(None, Some("Ableton,Logic"), "name", true, 0, 100)
            .unwrap();
        assert_eq!(res.total_count, 2);
        assert_eq!(res.total_unfiltered, 4);
        assert_eq!(
            res.projects.len(),
            2,
            "main SELECT must return matching rows"
        );
        assert!(res
            .projects
            .iter()
            .all(|p| p.daw == "Ableton" || p.daw == "Logic"));
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
            .query_daw(None, Some("Ableton,Logic"), "name", true, 0, 5)
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

        let res = db.query_presets(None, None, "name", true, 0, 100).unwrap();
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
            .query_presets(None, Some("MID"), "name", true, 0, 100)
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
            .query_presets(None, Some("FXP,H2P"), "name", true, 0, 100)
            .unwrap();
        assert_eq!(res.total_count, 3);
        assert_eq!(res.total_unfiltered, 4);
        assert_eq!(res.presets.len(), 3);
        assert!(res
            .presets
            .iter()
            .all(|p| p.format == "FXP" || p.format == "H2P"));
    }

    #[test]
    fn test_query_presets_total_unfiltered_stable_across_pagination() {
        let db = test_db();
        let presets: Vec<_> = (0..30)
            .map(|i| preset_file(&format!("p{i:02}.fxp"), "FXP"))
            .collect();
        db.save_preset_scan(&preset_snap("pr-page", "2024-06-01T00:00:00", presets))
            .unwrap();

        let p1 = db.query_presets(None, None, "name", true, 0, 10).unwrap();
        let p2 = db.query_presets(None, None, "name", true, 10, 10).unwrap();
        let p3 = db.query_presets(None, None, "name", true, 25, 10).unwrap();

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

        let res = db.query_plugins(None, None, "name", true, 0, 100).unwrap();
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
            .query_plugins(None, Some("VST3,AU"), "name", true, 0, 100)
            .unwrap();
        assert_eq!(res.total_count, 4);
        assert_eq!(
            res.plugins.len(),
            4,
            "main SELECT must return the 4 matching rows, not 0"
        );
        assert!(res
            .plugins
            .iter()
            .all(|p| p.plugin_type == "VST3" || p.plugin_type == "AU"));
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
            .query_plugins(Some("al"), Some("VST3,AU"), "name", true, 0, 2)
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
            .query_plugins(None, Some("VST3"), "name", true, 0, 100)
            .unwrap();
        assert_eq!(res.total_count, 2);
        assert_eq!(res.total_unfiltered, 4);

        let res = db
            .query_plugins(None, Some("VST3,AU"), "name", true, 0, 100)
            .unwrap();
        assert_eq!(res.total_count, 3);
        assert_eq!(res.total_unfiltered, 4);
    }

    #[test]
    fn test_query_plugins_total_unfiltered_stable_across_pagination() {
        let db = test_db();
        let plugins: Vec<_> = (0..40)
            .map(|i| plugin_info(&format!("plug{i:02}"), "VST3", "X"))
            .collect();
        db.save_plugin_scan(&plugin_snap("ps-page", "2024-06-01T00:00:00", plugins))
            .unwrap();

        let p1 = db.query_plugins(None, None, "name", true, 0, 15).unwrap();
        let p2 = db.query_plugins(None, None, "name", true, 15, 15).unwrap();

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
            .query_plugins(Some("Xfer"), None, "name", true, 0, 100)
            .unwrap();
        assert_eq!(res.total_count, 2);
        assert_eq!(res.total_unfiltered, 3);
    }

    // ── Unfiltered aggregate stats ──
    // These power the stats sections in the DAW/preset tabs and MUST be
    // independent of any table filter the user has applied.

    #[test]
    fn test_daw_stats_returns_latest_scan_aggregates() {
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
        assert!(db
            .read_cache("spectrogram-cache.json")
            .unwrap()
            .as_object()
            .unwrap()
            .contains_key("/a.wav"));
        db.clear_cache_table("spectrogram").unwrap();
        assert!(db
            .read_cache("spectrogram-cache.json")
            .unwrap()
            .as_object()
            .unwrap()
            .is_empty());
    }

    #[test]
    fn test_clear_cache_table_fingerprint() {
        let db = test_db();
        let data = serde_json::json!({"/sample.wav": "fpabc"});
        db.write_cache("fingerprint-cache.json", &data).unwrap();
        db.clear_cache_table("fingerprint").unwrap();
        assert!(db
            .read_cache("fingerprint-cache.json")
            .unwrap()
            .as_object()
            .unwrap()
            .is_empty());
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
        assert_eq!(obj["audio_scans"], 1);
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
        db.plugin_scan_parent_create("ps-stream", "2024-01-01T00:00:00", &["/Applications".into()])
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
