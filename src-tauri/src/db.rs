//! SQLite database layer for scalable storage of audio samples, analysis caches,
//! and scan metadata. Replaces JSON file persistence for data that can grow to
//! millions of rows.

use crate::history::{
    self, AudioHistory, AudioSample, AudioScanSnapshot, DawHistory, DawProject, DawScanSnapshot,
    KvrCacheEntry, PresetFile, PresetHistory, PresetScanSnapshot, ScanHistory, ScanSnapshot,
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
}

/// Current schema version — bump when adding migrations.
#[allow(dead_code)]
const SCHEMA_VERSION: i64 = 3;

impl Database {
    /// Open or create the database in the app data directory.
    pub fn open() -> Result<Self, String> {
        let db_path = history::get_data_dir().join("audio_haxor.db");
        let _ = std::fs::create_dir_all(db_path.parent().unwrap());
        let conn =
            Connection::open(&db_path).map_err(|e| format!("Failed to open database: {e}"))?;

        // Performance pragmas
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA synchronous=NORMAL;
             PRAGMA cache_size=-65536;
             PRAGMA foreign_keys=ON;
             PRAGMA temp_store=MEMORY;
             PRAGMA wal_autocheckpoint=1000;",
        )
        .map_err(|e| format!("Failed to set pragmas: {e}"))?;

        // Checkpoint WAL to keep it small (prevents startup lag from huge WAL)
        let _ = conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);");

        let db = Self {
            conn: Mutex::new(conn),
        };
        db.migrate()?;
        // Auto-prune: keep only 3 most recent scans per type, then reclaim space
        db.prune_old_scans(3);
        db.vacuum_if_needed();
        Ok(db)
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
        ] {
            let _ = conn.execute_batch(&format!(
                "DELETE FROM {data_tbl} WHERE {id_col} NOT IN (SELECT id FROM {scan_tbl} ORDER BY timestamp DESC LIMIT {keep_i});
                 DELETE FROM {scan_tbl} WHERE id NOT IN (SELECT id FROM {scan_tbl} ORDER BY timestamp DESC LIMIT {keep_i});"
            ));
        }
    }

    /// Checkpoint WAL to merge it into the main DB file. Keeps WAL small.
    pub fn checkpoint(&self) {
        let conn = self.conn.lock().unwrap();
        let _ = conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);");
    }

    /// VACUUM if >20% of pages are free (dead space from deleted rows).
    pub fn vacuum_if_needed(&self) {
        let conn = self.conn.lock().unwrap();
        let page_size: u64 = conn.query_row("PRAGMA page_size", [], |r| r.get(0)).unwrap_or(4096);
        let page_count: u64 = conn.query_row("PRAGMA page_count", [], |r| r.get(0)).unwrap_or(0);
        let free_count: u64 = conn.query_row("PRAGMA freelist_count", [], |r| r.get(0)).unwrap_or(0);
        let pct = if page_count > 0 { free_count * 100 / page_count } else { 0 };
        if pct > 20 {
            let before = page_count * page_size;
            crate::append_log(format!(
                "DB VACUUM — {}% free ({} / {} pages) | before: {}",
                pct, free_count, page_count, crate::format_size(before),
            ));
            drop(conn);
            let conn = self.conn.lock().unwrap();
            let _ = conn.execute_batch("VACUUM;");
            let after: u64 = conn.query_row("PRAGMA page_count", [], |r| r.get(0)).unwrap_or(0) * page_size;
            crate::append_log(format!("DB VACUUM DONE — after: {}", crate::format_size(after)));
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
                |row| row.get(0),
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

        Ok(())
    }

    /// Insert a batch of audio samples in a single transaction.
    pub fn insert_audio_batch(&self, scan_id: &str, samples: &[AudioSample]) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
        {
            let mut stmt = tx
                .prepare_cached(
                    "INSERT OR REPLACE INTO audio_samples
                     (name, path, directory, format, size, size_formatted, modified,
                      duration, channels, sample_rate, bits_per_sample, scan_id)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                )
                .map_err(|e| e.to_string())?;

            for s in samples {
                stmt.execute(params![
                    s.name,
                    s.path,
                    s.directory,
                    s.format,
                    s.size,
                    s.size_formatted,
                    s.modified,
                    s.duration,
                    s.channels,
                    s.sample_rate,
                    s.bits_per_sample,
                    scan_id,
                ])
                .map_err(|e| e.to_string())?;
            }
        }
        tx.commit().map_err(|e| e.to_string())?;
        Ok(())
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
                sample_count,
                total_bytes,
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
            "SELECT id FROM audio_scans WHERE sample_count > 0 ORDER BY timestamp DESC LIMIT 1",
            [],
            |row| row.get(0),
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
                    sample_count: row.get(2)?,
                    total_bytes: row.get(3)?,
                    format_counts: serde_json::from_str(&fc_str).unwrap_or_default(),
                    roots: serde_json::from_str(&roots_str).unwrap_or_default(),
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    }

    /// Paginated, sortable, filterable query for audio samples.
    pub fn query_audio(&self, params: &AudioQueryParams) -> Result<AudioQueryResult, String> {
        let conn = self.conn.lock().unwrap();

        // Resolve scan_id
        let scan_id = match &params.scan_id {
            Some(id) => id.clone(),
            None => conn
                .query_row(
                    "SELECT id FROM audio_scans WHERE sample_count > 0 ORDER BY timestamp DESC LIMIT 1",
                    [],
                    |row| row.get::<_, String>(0),
                )
                .optional()
                .map_err(|e| e.to_string())?
                .unwrap_or_default(),
        };

        if scan_id.is_empty() {
            return Ok(AudioQueryResult {
                samples: vec![],
                total_count: 0,
                total_unfiltered: 0,
            });
        }

        // Build WHERE clause
        let mut conditions = vec!["scan_id = ?1".to_string()];
        let mut bind_idx = 2;

        // Search: convert to subsequence LIKE pattern
        let search_pattern = params.search.as_ref().and_then(|s| {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                None
            } else {
                // Build fzf-style subsequence: "abc" → "%a%b%c%"
                let pattern: String = trimmed
                    .chars()
                    .map(|c| {
                        // Escape SQL LIKE special chars
                        match c {
                            '%' => "\\%".to_string(),
                            '_' => "\\_".to_string(),
                            _ => c.to_string(),
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("%");
                Some(format!("%{pattern}%"))
            }
        });

        if search_pattern.is_some() {
            // Only match against name (short) — path LIKE is too slow at millions of rows
            conditions.push(format!("name LIKE ?{bind_idx} ESCAPE '\\'"));
            bind_idx += 1;
        }

        if let Some(fmt) = &params.format_filter {
            if !fmt.is_empty() && fmt != "all" {
                conditions.push(format!("format = ?{bind_idx}"));
                bind_idx += 1;
            }
        }
        let _ = bind_idx; // suppress unused warning

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

        // Total unfiltered count (cached per scan_id — cheap indexed lookup)
        let total_unfiltered: u64 = conn
            .query_row(
                "SELECT COUNT(*) FROM audio_samples WHERE scan_id = ?1",
                params![scan_id],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;

        // Single query: fetch page + filtered count via COUNT(*) OVER()
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
        stmt.raw_bind_parameter(idx, &scan_id)
            .map_err(|e| e.to_string())?;
        idx += 1;
        if let Some(ref pat) = search_pattern {
            stmt.raw_bind_parameter(idx, pat)
                .map_err(|e| e.to_string())?;
            idx += 1;
        }
        if let Some(ref fmt) = params.format_filter {
            if !fmt.is_empty() && fmt != "all" {
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

    /// Get aggregate stats for a scan.
    pub fn audio_stats(&self, scan_id: Option<&str>) -> Result<AudioStatsResult, String> {
        let conn = self.conn.lock().unwrap();

        let sid = match scan_id {
            Some(id) => id.to_string(),
            None => conn
                .query_row(
                    "SELECT id FROM audio_scans WHERE sample_count > 0 ORDER BY timestamp DESC LIMIT 1",
                    [],
                    |row| row.get::<_, String>(0),
                )
                .optional()
                .map_err(|e| e.to_string())?
                .unwrap_or_default(),
        };

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
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;

        let total_bytes: u64 = conn
            .query_row(
                "SELECT COALESCE(SUM(size), 0) FROM audio_samples WHERE scan_id = ?1",
                params![sid],
                |row| row.get(0),
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
                Ok((row.get::<_, String>(0)?, row.get::<_, u64>(1)?))
            })
            .map_err(|e| e.to_string())?;
        for (fmt, count) in rows.flatten() {
            format_counts.insert(fmt, count);
        }

        let analyzed_count: u64 = conn
            .query_row(
                "SELECT COUNT(*) FROM audio_samples WHERE scan_id = ?1 AND bpm IS NOT NULL",
                params![sid],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;

        Ok(AudioStatsResult {
            sample_count,
            total_bytes,
            format_counts,
            analyzed_count,
        })
    }

    /// Update BPM for a sample (by path, latest scan).
    pub fn update_bpm(&self, path: &str, bpm: Option<f64>) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE audio_samples SET bpm = ?1 WHERE path = ?2 AND scan_id = (
                SELECT id FROM audio_scans WHERE sample_count > 0 ORDER BY timestamp DESC LIMIT 1
            )",
            params![bpm, path],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Update musical key for a sample.
    pub fn update_key(&self, path: &str, key: Option<&str>) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE audio_samples SET key_name = ?1 WHERE path = ?2 AND scan_id = (
                SELECT id FROM audio_scans WHERE sample_count > 0 ORDER BY timestamp DESC LIMIT 1
            )",
            params![key, path],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Update LUFS for a sample.
    pub fn update_lufs(&self, path: &str, lufs: Option<f64>) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE audio_samples SET lufs = ?1 WHERE path = ?2 AND scan_id = (
                SELECT id FROM audio_scans WHERE sample_count > 0 ORDER BY timestamp DESC LIMIT 1
            )",
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
                "SELECT bpm, key_name, lufs, duration, channels, sample_rate, bits_per_sample
                 FROM audio_samples WHERE path = ?1 AND scan_id = (
                    SELECT id FROM audio_scans WHERE sample_count > 0 ORDER BY timestamp DESC LIMIT 1
                 )",
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

    /// Get paths of samples that haven't been analyzed yet (bpm IS NULL).
    pub fn unanalyzed_paths(&self, limit: u64) -> Result<Vec<String>, String> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT path FROM audio_samples
                 WHERE bpm IS NULL AND scan_id = (
                    SELECT id FROM audio_scans WHERE sample_count > 0 ORDER BY timestamp DESC LIMIT 1
                 )
                 LIMIT ?1",
            )
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
        let scan_id: String = conn
            .query_row(
                "SELECT id FROM plugin_scans ORDER BY timestamp DESC LIMIT 1",
                [],
                |r| r.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())?
            .unwrap_or_default();
        if scan_id.is_empty() {
            return Ok(PluginQueryResult {
                plugins: vec![],
                total_count: 0,
            });
        }

        let mut where_parts = vec!["scan_id = ?1".to_string()];
        let mut bind_idx = 2usize;
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
                where_parts.push(format!("plugin_type = ?{bind_idx}"));
                bind_idx += 1;
            }
        }
        let _ = bind_idx;
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
            stmt.raw_bind_parameter(bi, &scan_id).map_err(|e| e.to_string())?; bi += 1;
            if let Some(ref p) = search_pat { stmt.raw_bind_parameter(bi, p).map_err(|e| e.to_string())?; bi += 1; }
            if let Some(tf) = type_filter { if !tf.is_empty() && tf != "all" { stmt.raw_bind_parameter(bi, tf).map_err(|e| e.to_string())?; bi += 1; } }
            let _ = bi;
            let mut rows = stmt.raw_query();
            rows.next().map_err(|e| e.to_string())?.map(|r| r.get::<_, u64>(0).unwrap_or(0)).unwrap_or(0)
        };

        let mut bi = 1usize;
        let mut bind_offset = 2usize;
        if search_pat.is_some() { bind_offset += 1; }
        if type_filter.map(|t| !t.is_empty() && t != "all").unwrap_or(false) { bind_offset += 1; }
        let sql = format!("SELECT name, path, plugin_type, version, manufacturer, manufacturer_url, size, size_bytes, modified, architectures FROM plugins WHERE {where_cl} ORDER BY {sort_col} {dir} LIMIT ?{bind_offset} OFFSET ?{}", bind_offset + 1);
        let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
        bi = 1;
        stmt.raw_bind_parameter(bi, &scan_id).map_err(|e| e.to_string())?; bi += 1;
        if let Some(ref p) = search_pat { stmt.raw_bind_parameter(bi, p).map_err(|e| e.to_string())?; bi += 1; }
        if let Some(tf) = type_filter { if !tf.is_empty() && tf != "all" { stmt.raw_bind_parameter(bi, tf).map_err(|e| e.to_string())?; bi += 1; } }
        stmt.raw_bind_parameter(bi, limit as i64).map_err(|e| e.to_string())?; bi += 1;
        stmt.raw_bind_parameter(bi, offset as i64).map_err(|e| e.to_string())?;

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
        })
    }

    // ── Paginated DAW query ──
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
        let scan_id: String = conn
            .query_row(
                "SELECT id FROM daw_scans ORDER BY timestamp DESC LIMIT 1",
                [],
                |r| r.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())?
            .unwrap_or_default();
        if scan_id.is_empty() {
            return Ok(DawQueryResult {
                projects: vec![],
                total_count: 0,
            });
        }

        let mut where_parts = vec!["scan_id = ?1".to_string()];
        let mut bind_idx = 2usize;
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
            where_parts.push(format!(
                "(name LIKE ?{bind_idx} ESCAPE '\\' OR path LIKE ?{bind_idx} ESCAPE '\\')"
            ));
            bind_idx += 1;
        }
        if let Some(f) = daw_filter {
            if !f.is_empty() && f != "all" {
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

        let total_count: u64 = {
            let sql = format!("SELECT COUNT(*) FROM daw_projects WHERE {where_cl}");
            let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
            let mut bi = 1;
            stmt.raw_bind_parameter(bi, &scan_id)
                .map_err(|e| e.to_string())?;
            bi += 1;
            if let Some(ref p) = search_pat {
                stmt.raw_bind_parameter(bi, p).map_err(|e| e.to_string())?;
                bi += 1;
            }
            if let Some(f) = daw_filter {
                if !f.is_empty() && f != "all" {
                    stmt.raw_bind_parameter(bi, f).map_err(|e| e.to_string())?;
                }
            }
            let mut rows = stmt.raw_query();
            rows.next()
                .map_err(|e| e.to_string())?
                .map(|r| r.get::<_, u64>(0).unwrap_or(0))
                .unwrap_or(0)
        };

        let sql = format!("SELECT name, path, directory, format, daw, size, size_formatted, modified FROM daw_projects WHERE {where_cl} ORDER BY {sort_col} {dir} LIMIT ?{bind_idx} OFFSET ?{}", bind_idx + 1);
        let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
        let mut bi = 1;
        stmt.raw_bind_parameter(bi, &scan_id)
            .map_err(|e| e.to_string())?;
        bi += 1;
        if let Some(ref p) = search_pat {
            stmt.raw_bind_parameter(bi, p).map_err(|e| e.to_string())?;
            bi += 1;
        }
        if let Some(f) = daw_filter {
            if !f.is_empty() && f != "all" {
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
        let scan_id: String = conn
            .query_row(
                "SELECT id FROM preset_scans ORDER BY timestamp DESC LIMIT 1",
                [],
                |r| r.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())?
            .unwrap_or_default();
        if scan_id.is_empty() {
            return Ok(PresetQueryResult {
                presets: vec![],
                total_count: 0,
            });
        }

        let mut where_parts = vec!["scan_id = ?1".to_string()];
        let mut bind_idx = 2usize;
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
            where_parts.push(format!(
                "(name LIKE ?{bind_idx} ESCAPE '\\' OR path LIKE ?{bind_idx} ESCAPE '\\')"
            ));
            bind_idx += 1;
        }
        // Exclude MIDI files from presets
        where_parts.push("format NOT IN ('MID', 'MIDI')".into());
        if let Some(f) = format_filter {
            if !f.is_empty() && f != "all" {
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

        let total_count: u64 = {
            let sql = format!("SELECT COUNT(*) FROM presets WHERE {where_cl}");
            let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
            let mut bi = 1;
            stmt.raw_bind_parameter(bi, &scan_id)
                .map_err(|e| e.to_string())?;
            bi += 1;
            if let Some(ref p) = search_pat {
                stmt.raw_bind_parameter(bi, p).map_err(|e| e.to_string())?;
                bi += 1;
            }
            if let Some(f) = format_filter {
                if !f.is_empty() && f != "all" {
                    stmt.raw_bind_parameter(bi, f).map_err(|e| e.to_string())?;
                }
            }
            let mut rows = stmt.raw_query();
            rows.next()
                .map_err(|e| e.to_string())?
                .map(|r| r.get::<_, u64>(0).unwrap_or(0))
                .unwrap_or(0)
        };

        let sql = format!("SELECT name, path, directory, format, size, size_formatted, modified FROM presets WHERE {where_cl} ORDER BY {sort_col} {dir} LIMIT ?{bind_idx} OFFSET ?{}", bind_idx + 1);
        let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
        let mut bi = 1;
        stmt.raw_bind_parameter(bi, &scan_id)
            .map_err(|e| e.to_string())?;
        bi += 1;
        if let Some(ref p) = search_pat {
            stmt.raw_bind_parameter(bi, p).map_err(|e| e.to_string())?;
            bi += 1;
        }
        if let Some(f) = format_filter {
            if !f.is_empty() && f != "all" {
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
        })
    }

    pub fn save_plugin_scan(&self, snap: &ScanSnapshot) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        let dirs_json = serde_json::to_string(&snap.directories).unwrap_or_default();
        let roots_json = serde_json::to_string(&snap.roots).unwrap_or_default();
        conn.execute(
            "INSERT OR REPLACE INTO plugin_scans (id, timestamp, plugin_count, directories, roots) VALUES (?1,?2,?3,?4,?5)",
            params![snap.id, snap.timestamp, snap.plugin_count, dirs_json, roots_json],
        ).map_err(|e| e.to_string())?;
        let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
        {
            // Delete old plugins for this scan_id first
            tx.execute("DELETE FROM plugins WHERE scan_id = ?1", params![snap.id])
                .map_err(|e| e.to_string())?;
            let mut stmt = tx.prepare_cached(
                "INSERT INTO plugins (name, path, plugin_type, version, manufacturer, manufacturer_url, size, size_bytes, modified, architectures, scan_id) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)"
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
                    p.size_bytes,
                    p.modified,
                    arch_json,
                    snap.id
                ])
                .map_err(|e| e.to_string())?;
            }
        }
        tx.commit().map_err(|e| e.to_string())
    }

    pub fn get_plugin_scans(&self) -> Result<Vec<serde_json::Value>, String> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT id, timestamp, plugin_count, roots FROM plugin_scans ORDER BY timestamp DESC").map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| {
                let roots_str: String = row.get(3)?;
                Ok(serde_json::json!({
                    "id": row.get::<_,String>(0)?,
                    "timestamp": row.get::<_,String>(1)?,
                    "pluginCount": row.get::<_,u64>(2)?,
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
            params![id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        ).map_err(|e| e.to_string())?;
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
        self.save_scan(
            &snap.id,
            &snap.timestamp,
            snap.sample_count as u64,
            snap.total_bytes,
            &snap.format_counts,
            &snap.roots,
        )?;
        self.insert_audio_batch(&snap.id, &snap.samples)
    }

    pub fn get_audio_scans_list(&self) -> Result<Vec<serde_json::Value>, String> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT id, timestamp, sample_count, total_bytes, format_counts, roots FROM audio_scans ORDER BY timestamp DESC").map_err(|e| e.to_string())?;
        let rows = stmt.query_map([], |row| {
            let fc_str: String = row.get(4)?;
            let roots_str: String = row.get(5)?;
            Ok(serde_json::json!({
                "id": row.get::<_,String>(0)?,
                "timestamp": row.get::<_,String>(1)?,
                "sampleCount": row.get::<_,u64>(2)?,
                "totalBytes": row.get::<_,u64>(3)?,
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
            params![id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?))
        ).map_err(|e| e.to_string())?;
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
        Ok(AudioScanSnapshot {
            id: id.to_string(),
            timestamp: ts,
            sample_count: sc,
            total_bytes: tb,
            format_counts: serde_json::from_str(&fc_str).unwrap_or_default(),
            samples,
            roots: serde_json::from_str(&roots_str).unwrap_or_default(),
        })
    }

    pub fn get_latest_audio_scan(&self) -> Result<Option<AudioScanSnapshot>, String> {
        let conn = self.conn.lock().unwrap();
        let id: Option<String> = conn
            .query_row(
                "SELECT id FROM audio_scans WHERE sample_count > 0 ORDER BY timestamp DESC LIMIT 1",
                [],
                |r| r.get(0),
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

    pub fn save_daw_scan(&self, snap: &DawScanSnapshot) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        let daw_json = serde_json::to_string(&snap.daw_counts).unwrap_or_default();
        let roots_json = serde_json::to_string(&snap.roots).unwrap_or_default();
        conn.execute(
            "INSERT OR REPLACE INTO daw_scans (id, timestamp, project_count, total_bytes, daw_counts, roots) VALUES (?1,?2,?3,?4,?5,?6)",
            params![snap.id, snap.timestamp, snap.project_count, snap.total_bytes, daw_json, roots_json],
        ).map_err(|e| e.to_string())?;
        let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
        {
            tx.execute(
                "DELETE FROM daw_projects WHERE scan_id = ?1",
                params![snap.id],
            )
            .map_err(|e| e.to_string())?;
            let mut stmt = tx.prepare_cached("INSERT INTO daw_projects (name, path, directory, format, daw, size, size_formatted, modified, scan_id) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)").map_err(|e| e.to_string())?;
            for p in &snap.projects {
                stmt.execute(params![
                    p.name,
                    p.path,
                    p.directory,
                    p.format,
                    p.daw,
                    p.size,
                    p.size_formatted,
                    p.modified,
                    snap.id
                ])
                .map_err(|e| e.to_string())?;
            }
        }
        tx.commit().map_err(|e| e.to_string())
    }

    pub fn get_daw_scans(&self) -> Result<Vec<serde_json::Value>, String> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT id, timestamp, project_count, total_bytes, daw_counts, roots FROM daw_scans ORDER BY timestamp DESC").map_err(|e| e.to_string())?;
        let rows = stmt.query_map([], |row| {
            let dc_str: String = row.get(4)?;
            let roots_str: String = row.get(5)?;
            Ok(serde_json::json!({
                "id": row.get::<_,String>(0)?,
                "timestamp": row.get::<_,String>(1)?,
                "projectCount": row.get::<_,u64>(2)?,
                "totalBytes": row.get::<_,u64>(3)?,
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
            params![id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?))
        ).map_err(|e| e.to_string())?;
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
        Ok(DawScanSnapshot {
            id: id.to_string(),
            timestamp: ts,
            project_count: pc,
            total_bytes: tb,
            daw_counts: serde_json::from_str(&dc_str).unwrap_or_default(),
            projects,
            roots: serde_json::from_str(&roots_str).unwrap_or_default(),
        })
    }

    pub fn get_latest_daw_scan(&self) -> Result<Option<DawScanSnapshot>, String> {
        let conn = self.conn.lock().unwrap();
        let id: Option<String> = conn
            .query_row(
                "SELECT id FROM daw_scans ORDER BY timestamp DESC LIMIT 1",
                [],
                |r| r.get(0),
            )
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

    pub fn save_preset_scan(&self, snap: &PresetScanSnapshot) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        let fc_json = serde_json::to_string(&snap.format_counts).unwrap_or_default();
        let roots_json = serde_json::to_string(&snap.roots).unwrap_or_default();
        conn.execute(
            "INSERT OR REPLACE INTO preset_scans (id, timestamp, preset_count, total_bytes, format_counts, roots) VALUES (?1,?2,?3,?4,?5,?6)",
            params![snap.id, snap.timestamp, snap.preset_count, snap.total_bytes, fc_json, roots_json],
        ).map_err(|e| e.to_string())?;
        let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
        {
            tx.execute("DELETE FROM presets WHERE scan_id = ?1", params![snap.id])
                .map_err(|e| e.to_string())?;
            let mut stmt = tx.prepare_cached("INSERT INTO presets (name, path, directory, format, size, size_formatted, modified, scan_id) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)").map_err(|e| e.to_string())?;
            for p in &snap.presets {
                stmt.execute(params![
                    p.name,
                    p.path,
                    p.directory,
                    p.format,
                    p.size,
                    p.size_formatted,
                    p.modified,
                    snap.id
                ])
                .map_err(|e| e.to_string())?;
            }
        }
        tx.commit().map_err(|e| e.to_string())
    }

    pub fn get_preset_scans(&self) -> Result<Vec<serde_json::Value>, String> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT id, timestamp, preset_count, total_bytes, format_counts, roots FROM preset_scans ORDER BY timestamp DESC").map_err(|e| e.to_string())?;
        let rows = stmt.query_map([], |row| {
            let fc_str: String = row.get(4)?;
            let roots_str: String = row.get(5)?;
            Ok(serde_json::json!({
                "id": row.get::<_,String>(0)?,
                "timestamp": row.get::<_,String>(1)?,
                "presetCount": row.get::<_,u64>(2)?,
                "totalBytes": row.get::<_,u64>(3)?,
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
            params![id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?))
        ).map_err(|e| e.to_string())?;
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
        Ok(PresetScanSnapshot {
            id: id.to_string(),
            timestamp: ts,
            preset_count: pc,
            total_bytes: tb,
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
        // Pre-resolve scan_id to avoid subquery on every row
        let sid: String = conn
            .query_row(
                "SELECT id FROM audio_scans WHERE sample_count > 0 ORDER BY timestamp DESC LIMIT 1",
                [],
                |r| r.get(0),
            )
            .unwrap_or_default();
        if sid.is_empty() {
            return Ok(serde_json::json!({}));
        }
        let sql = format!(
            "SELECT path, {col} FROM audio_samples WHERE {col} IS NOT NULL AND scan_id = ?1"
        );
        let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
        stmt.raw_bind_parameter(1, &sid)
            .map_err(|e| e.to_string())?;
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
        let sid: String = conn
            .query_row(
                "SELECT id FROM audio_scans WHERE sample_count > 0 ORDER BY timestamp DESC LIMIT 1",
                [],
                |r| r.get(0),
            )
            .unwrap_or_default();
        if sid.is_empty() {
            return Ok(());
        }
        let sql = format!("UPDATE audio_samples SET {col} = ?1 WHERE path = ?2 AND scan_id = ?3");
        let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
        {
            let mut stmt = tx.prepare_cached(&sql).map_err(|e| e.to_string())?;
            for (path, val) in obj {
                if field == "key" {
                    if let Some(s) = val.as_str() {
                        let _ = stmt.execute(params![s, path, sid]);
                    }
                } else {
                    if let Some(v) = val.as_f64() {
                        let _ = stmt.execute(params![v, path, sid]);
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
            "kvr_cache",
            "waveform_cache",
            "spectrogram_cache",
            "xref_cache",
            "fingerprint_cache",
        ];
        let mut map = serde_json::Map::new();
        for t in &tables {
            let count: u64 = conn
                .query_row(&format!("SELECT COUNT(*) FROM {t}"), [], |r| r.get(0))
                .unwrap_or(0);
            map.insert(t.to_string(), serde_json::json!(count));
        }
        Ok(serde_json::Value::Object(map))
    }

    /// Get stats for all caches: item count and estimated size.
    pub fn cache_stats(&self) -> Result<Vec<CacheStat>, String> {
        let conn = self.conn.lock().unwrap();
        let page_size: u64 = conn
            .query_row("PRAGMA page_size", [], |r| r.get(0))
            .unwrap_or(4096);
        let mut stats = Vec::new();

        // Analysis caches (columns on audio_samples)
        let total_samples: u64 = conn.query_row(
            "SELECT COUNT(*) FROM audio_samples WHERE scan_id = (SELECT id FROM audio_scans WHERE sample_count > 0 ORDER BY timestamp DESC LIMIT 1)",
            [], |r| r.get(0),
        ).unwrap_or(0);
        for (label, col, key) in [
            ("BPM", "bpm", "bpm"),
            ("Key", "key_name", "key"),
            ("LUFS", "lufs", "lufs"),
        ] {
            let count: u64 = conn.query_row(
                &format!("SELECT COUNT(*) FROM audio_samples WHERE {col} IS NOT NULL AND scan_id = (SELECT id FROM audio_scans WHERE sample_count > 0 ORDER BY timestamp DESC LIMIT 1)"),
                [], |r| r.get(0),
            ).unwrap_or(0);
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
                    |r| Ok((r.get(0)?, r.get(1)?)),
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
        ] {
            let scan_count: u64 = conn
                .query_row(&format!("SELECT COUNT(*) FROM {scan_table}"), [], |r| {
                    r.get(0)
                })
                .unwrap_or(0);
            let item_count: u64 = conn
                .query_row(&format!("SELECT COUNT(*) FROM {item_table}"), [], |r| {
                    r.get(0)
                })
                .unwrap_or(0);
            // Estimate size from number of items * avg row size
            let avg_row: u64 = if item_count > 0 {
                let pages: u64 = conn
                    .query_row("SELECT page_count FROM pragma_page_count()", [], |r| {
                        r.get(0)
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
    pub fn batch_update_analysis(
        &self,
        results: &[(String, Option<f64>, Option<String>, Option<f64>)],
    ) -> Result<u32, String> {
        let conn = self.conn.lock().unwrap();
        let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
        let mut count = 0u32;
        {
            let mut stmt = tx.prepare_cached(
                "UPDATE audio_samples SET bpm = ?1, key_name = ?2, lufs = ?3 WHERE path = ?4 AND scan_id = (SELECT id FROM audio_scans WHERE sample_count > 0 ORDER BY timestamp DESC LIMIT 1)"
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
             DELETE FROM fingerprint_cache;",
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
                    |row| row.get(0),
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
                params![snap.id, snap.timestamp, snap.plugin_count, dirs_json, roots_json],
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
                        p.size_bytes,
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
                params![snap.id, snap.timestamp, snap.project_count, snap.total_bytes, daw_json, roots_json],
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
                        p.size,
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
                params![snap.id, snap.timestamp, snap.preset_count, snap.total_bytes, fc_json, roots_json],
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
                        p.size,
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

        // "kck" should match "kick" via subsequence
        let result = db
            .query_audio(&AudioQueryParams {
                scan_id: Some("s1".into()),
                search: Some("kck".into()),
                format_filter: None,
                sort_key: "name".into(),
                sort_asc: true,
                offset: 0,
                limit: 100,
            })
            .unwrap();

        assert_eq!(result.total_count, 2);
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

    /// Many lib tests call `init_global()` in parallel; migrations must not race on one file.
    #[test]
    fn init_global_concurrent_ok() {
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
    }

    #[test]
    fn init_global_idempotent_same_thread() {
        for _ in 0..64 {
            init_global().expect("init_global");
        }
        assert!(global_initialized());
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
