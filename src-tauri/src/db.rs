//! SQLite database layer for scalable storage of audio samples, analysis caches,
//! and scan metadata. Replaces JSON file persistence for data that can grow to
//! millions of rows.

use crate::history::{self, AudioHistory, AudioSample,
    DawHistory, KvrCacheEntry, PresetHistory, ScanHistory};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;

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

/// Current schema version — bump when adding migrations.
#[allow(dead_code)]
const SCHEMA_VERSION: i64 = 2;

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
             PRAGMA temp_store=MEMORY;",
        )
        .map_err(|e| format!("Failed to set pragmas: {e}"))?;

        let db = Self {
            conn: Mutex::new(conn),
        };
        db.migrate()?;
        Ok(db)
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

        Ok(())
    }

    /// Insert a batch of audio samples in a single transaction.
    pub fn insert_audio_batch(
        &self,
        scan_id: &str,
        samples: &[AudioSample],
    ) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        let tx = conn
            .unchecked_transaction()
            .map_err(|e| e.to_string())?;
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
            params![id, timestamp, sample_count, total_bytes, fc_json, roots_json],
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
                    "SELECT id FROM audio_scans ORDER BY timestamp DESC LIMIT 1",
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
            conditions.push(format!(
                "(name LIKE ?{bind_idx} ESCAPE '\\' OR path LIKE ?{bind_idx} ESCAPE '\\')"
            ));
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
        let nulls = if params.sort_asc {
            "NULLS LAST"
        } else {
            "NULLS LAST"
        };

        // Count total unfiltered
        let total_unfiltered: u64 = conn
            .query_row(
                &format!("SELECT COUNT(*) FROM audio_samples WHERE scan_id = ?1"),
                params![scan_id],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;

        // Count filtered
        let count_sql = format!("SELECT COUNT(*) FROM audio_samples WHERE {where_clause}");
        let total_count: u64 = {
            let mut stmt = conn.prepare(&count_sql).map_err(|e| e.to_string())?;
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
                }
            }
            let mut rows = stmt.raw_query();
            rows.next()
                .map_err(|e| e.to_string())?
                .map(|r| r.get::<_, u64>(0).unwrap_or(0))
                .unwrap_or(0)
        };

        // Fetch page
        let query_sql = format!(
            "SELECT name, path, directory, format, size, size_formatted, modified,
                    duration, channels, sample_rate, bits_per_sample, bpm, key_name, lufs
             FROM audio_samples
             WHERE {where_clause}
             ORDER BY {sort_col} {sort_dir} {nulls}
             LIMIT ?{limit_idx} OFFSET ?{offset_idx}",
            limit_idx = bind_idx,
            offset_idx = bind_idx + 1,
            where_clause = where_clause,
            sort_col = sort_col,
            sort_dir = sort_dir,
            nulls = nulls,
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
                channels: row.get::<_, Option<i32>>(8).ok().flatten().map(|v| v as u16),
                sample_rate: row.get::<_, Option<i32>>(9).ok().flatten().map(|v| v as u32),
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
                    "SELECT id FROM audio_scans ORDER BY timestamp DESC LIMIT 1",
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
        for r in rows {
            if let Ok((fmt, count)) = r {
                format_counts.insert(fmt, count);
            }
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
                SELECT id FROM audio_scans ORDER BY timestamp DESC LIMIT 1
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
                SELECT id FROM audio_scans ORDER BY timestamp DESC LIMIT 1
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
                SELECT id FROM audio_scans ORDER BY timestamp DESC LIMIT 1
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
                    SELECT id FROM audio_scans ORDER BY timestamp DESC LIMIT 1
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
                    SELECT id FROM audio_scans ORDER BY timestamp DESC LIMIT 1
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
        conn.execute(
            "DELETE FROM audio_scans WHERE id = ?1",
            params![scan_id],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
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
        total += self.migrate_kv_cache(&data_dir, "xref-cache.json", "xref_cache", "project_path", "plugins_json")?;
        total += self.migrate_kv_cache(&data_dir, "waveform-cache.json", "waveform_cache", "path", "data")?;
        total += self.migrate_kv_cache(&data_dir, "spectrogram-cache.json", "spectrogram_cache", "path", "data")?;
        total += self.migrate_kv_cache(&data_dir, "fingerprint-cache.json", "fingerprint_cache", "path", "fingerprint")?;

        // Rename all migrated JSON files to .bak
        for name in &[
            "audio-scan-history.json", "bpm-cache.json", "key-cache.json", "lufs-cache.json",
            "scan-history.json", "daw-scan-history.json", "preset-scan-history.json",
            "kvr-cache.json", "xref-cache.json", "waveform-cache.json",
            "spectrogram-cache.json", "fingerprint-cache.json",
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
        if !path.exists() { return Ok(0); }
        let data = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
        let history: AudioHistory = serde_json::from_str(&data).map_err(|e| format!("audio JSON: {e}"))?;
        let mut count = 0;
        for snap in &history.scans {
            self.save_scan(&snap.id, &snap.timestamp, snap.sample_count as u64, snap.total_bytes, &snap.format_counts, &snap.roots)?;
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
        if !path.exists() { return Ok(0); }
        let data = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
        let history: ScanHistory = serde_json::from_str(&data).map_err(|e| format!("plugin JSON: {e}"))?;
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
                        p.name, p.path, p.plugin_type, p.version, p.manufacturer,
                        p.manufacturer_url, p.size, p.size_bytes, p.modified, arch_json, snap.id
                    ]).map_err(|e| e.to_string())?;
                }
            }
            tx.commit().map_err(|e| e.to_string())?;
            count += snap.plugins.len();
        }
        Ok(count)
    }

    fn migrate_daw_json(&self, data_dir: &std::path::Path) -> Result<usize, String> {
        let path = data_dir.join("daw-scan-history.json");
        if !path.exists() { return Ok(0); }
        let data = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
        let history: DawHistory = serde_json::from_str(&data).map_err(|e| format!("daw JSON: {e}"))?;
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
                        p.name, p.path, p.directory, p.format, p.daw, p.size, p.size_formatted, p.modified, snap.id
                    ]).map_err(|e| e.to_string())?;
                }
            }
            tx.commit().map_err(|e| e.to_string())?;
            count += snap.projects.len();
        }
        Ok(count)
    }

    fn migrate_preset_json(&self, data_dir: &std::path::Path) -> Result<usize, String> {
        let path = data_dir.join("preset-scan-history.json");
        if !path.exists() { return Ok(0); }
        let data = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
        let history: PresetHistory = serde_json::from_str(&data).map_err(|e| format!("preset JSON: {e}"))?;
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
                        p.name, p.path, p.directory, p.format, p.size, p.size_formatted, p.modified, snap.id
                    ]).map_err(|e| e.to_string())?;
                }
            }
            tx.commit().map_err(|e| e.to_string())?;
            count += snap.presets.len();
        }
        Ok(count)
    }

    fn migrate_kvr_json(&self, data_dir: &std::path::Path) -> Result<usize, String> {
        let path = data_dir.join("kvr-cache.json");
        if !path.exists() { return Ok(0); }
        let data = std::fs::read_to_string(&path).unwrap_or_default();
        let cache: HashMap<String, KvrCacheEntry> = serde_json::from_str(&data).unwrap_or_default();
        if cache.is_empty() { return Ok(0); }
        let conn = self.conn.lock().unwrap();
        let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
        let count = cache.len();
        {
            let mut stmt = tx.prepare_cached(
                "INSERT OR REPLACE INTO kvr_cache (plugin_key, kvr_url, update_url, latest_version, has_update, source, timestamp) VALUES (?1,?2,?3,?4,?5,?6,?7)"
            ).map_err(|e| e.to_string())?;
            for (key, entry) in &cache {
                stmt.execute(params![
                    key, entry.kvr_url, entry.update_url, entry.latest_version,
                    entry.has_update as i32, entry.source, entry.timestamp
                ]).map_err(|e| e.to_string())?;
            }
        }
        tx.commit().map_err(|e| e.to_string())?;
        Ok(count)
    }

    /// Generic key→value JSON cache migration (xref, waveform, spectrogram, fingerprint).
    fn migrate_kv_cache(
        &self, data_dir: &std::path::Path, filename: &str,
        table: &str, key_col: &str, val_col: &str,
    ) -> Result<usize, String> {
        let path = data_dir.join(filename);
        if !path.exists() { return Ok(0); }
        let data = std::fs::read_to_string(&path).unwrap_or_default();
        let cache: HashMap<String, serde_json::Value> = serde_json::from_str(&data).unwrap_or_default();
        if cache.is_empty() { return Ok(0); }
        let conn = self.conn.lock().unwrap();
        let sql = format!("INSERT OR REPLACE INTO {table} ({key_col}, {val_col}) VALUES (?1, ?2)");
        let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
        let count = cache.len();
        {
            let mut stmt = tx.prepare_cached(&sql).map_err(|e| e.to_string())?;
            for (k, v) in &cache {
                let val_str = if v.is_string() { v.as_str().unwrap_or("").to_string() } else { v.to_string() };
                stmt.execute(params![k, val_str]).map_err(|e| e.to_string())?;
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
        conn.execute_batch(
            "PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;",
        )
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
        db.save_scan("scan1", "2024-01-01T00:00:00", 3, 3500, &HashMap::new(), &[])
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
            .map(|i| sample(&format!("s{i:03}.wav"), &format!("/s{i:03}.wav"), "WAV", 100))
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
            println!("  Scan {} — {} samples, {} bytes, {} roots",
                s.id, s.sample_count, s.total_bytes, s.roots.len());
        }
        if let Ok(stats) = db.audio_stats(None) {
            println!("Stats: {} samples, {} bytes, {} analyzed, {} formats",
                stats.sample_count, stats.total_bytes, stats.analyzed_count, stats.format_counts.len());
        }
    }
}
