# Incremental directory scanning

## Behavior (unified scan)

- Each successfully scanned directory is stored in `directory_scan_state` with its modification time (Unix seconds from `metadata(path).modified()`). A directory is recorded **after** its subtree has been walked so partial runs do not mark a parent as fully scanned.
- On a later run, before reading a directory’s children, if the stored mtime exists and **current mtime ≤ stored mtime**, the walker **skips** that entire subtree (no `read_dir` for that branch).
- If **current mtime > stored mtime** (or there is no row yet), the directory is fully walked and the row is **updated** after processing that directory.

## Unified scan run state (`unified_scan_run`)

- A single SQLite row (`id = 1`) records the **last** unified scan: `run_id`, `started_at`, `finished_at`, `outcome` (`complete` | `stopped` | `error` | `in_progress`), per-type scan ids, `roots_json`, and optional `error_message` / `last_directory_path`.
- Incremental mtime snapshots in `directory_scan_state` are **only** used when the last persisted outcome is `complete`. If the user stopped the scan, the scan panicked, or a run was interrupted while still `in_progress`, the app does not trust the snapshot (and clears stored directory rows for non-complete outcomes). The Tauri command `get_unified_scan_run` returns the persisted row for UI or diagnostics.

## Limitations

- **Directory mtime** does not always change when a file inside is edited in place; some OS/filesystem combinations only bump the **file** mtime. Unchanged directory mtime can miss in-place edits. Turn **Incremental Unified Scan** off under **Settings → Scan Behavior** (or set `incrementalDirectoryScan` to `off` in `preferences.toml`) for a full tree walk when you need that guarantee.
- **Symlinks / canonical paths**: Keys use the same normalization as the walker’s visit deduplication (`canonicalize` when possible).
- **Per-scan “new files” in History**: Listing only the files first seen in a given `scan_id` requires append-only inserts (no wholesale replace per scan) and optional columns such as `discovered_in_scan_id`. The directory layer is the prerequisite; file-level history UI is a separate follow-up.

## Plugin-only scan

- **VST/AU root enumeration** (`discover_plugins` / plugin tab scan) does **not** use the shared `directory_scan_state` incremental skip. Those roots are cheap to list (one `read_dir` per platform directory), but reusing the same mtime map as unified scans would incorrectly skip them: a prior unified walk already records each directory’s mtime, and after any plugin scan that used incremental, the next run would skip every root and find zero plugins.

## Preference

- `incrementalDirectoryScan` — when `off`, directory snapshots are ignored and every scan is a full tree walk. Exposed in-app as **Settings → Scan Behavior → Incremental Unified Scan**. Default in `config.default.toml`: `on` (`[scanning]`).

## SQLite inventory (main UI)

When queries do not pass a specific `scan_id`, the app treats the database as a **library**: one canonical row per filesystem `path` (the row with the largest `id` for that path). New scans append rows; the UI aggregates across all scans without dropping prior paths. Analysis fields (BPM, key, LUFS) and cache stats for those columns use the same library scope; updates apply by `path` so duplicate rows from different scans stay consistent. History or drill-down APIs that pass an explicit `scan_id` still restrict to that scan only.
