```
  ▄▄▄       █    ██ ▓█████▄  ██▓ ▒█████
 ▒████▄     ██  ▓██▒▒██▀ ██▌▓██▒▒██▒  ██▒
 ▒██  ▀█▄  ▓██  ▒██░░██   █▌▒██▒▒██░  ██▒
 ░██▄▄▄▄██ ▓▓█  ░██░░▓█▄   ▌░██░▒██   ██░
  ▓█   ▓██▒▒▒█████▓ ░▒████▓ ░██░░ ████▓▒░
  ▒▒   ▓▒█░░▒▓▒ ▒ ▒  ▒▒▓  ▒ ░▓  ░ ▒░▒░▒░

 ██░ ██  ▄▄▄      ▒██   ██▒ ▒█████   ██▀███
▓██░ ██▒▒████▄    ▒▒ █ █ ▒░▒██▒  ██▒▓██ ▒ ██▒
▒██▀▀██░▒██  ▀█▄  ░░  █   ░▒██░  ██▒▓██ ░▄█ ▒
░▓█ ░██ ░██▄▄▄▄██  ░ █ █ ▒ ▒██   ██░▒██▀▀█▄
░▓████▒   ▓█   ▓██▒▒██▒ ▒██▒░ ████▓▒░░██▓ ▒██▒
```

> **// SYSTEM ONLINE -- AUDIO_HAXOR v1.3.0 // by MenkeTechnologies**

A high-voltage **Tauri v2** desktop app that jacks into your system's audio plugin directories, maps every VST2/VST3/AU module it finds, scans audio sample libraries, discovers DAW project files, checks the web for the latest plugin versions, and maintains a full changelog of every scan -- so nothing slips through the cracks. Rust backend with a cyberpunk CRT interface featuring neon glow, scanline overlays, glitch effects, and multiple color schemes.

---

[![CI](https://github.com/MenkeTechnologies/universal-plugin-update-manager/actions/workflows/ci.yml/badge.svg)](https://github.com/MenkeTechnologies/universal-plugin-update-manager/actions/workflows/ci.yml)


## // VISUAL INTERFACE //

### `> BOOT SEQUENCE`

![Welcome Screen](screenshots/welcome.png)

*Initial state -- the grid is dark, awaiting your scan command. Last scan auto-loads on startup.*

---

### `> SCANNING AUDIO NODES...`

![Scan Results](screenshots/scan-results.png)

*Plugins stream into the list in real-time as the background worker discovers them. Progress counter and bar show live status. Hit Stop to cancel anytime -- discovered plugins are kept.*

---

### `> UPDATE MATRIX LOADED`

![Update Checker](screenshots/updates.png)

*Checks KVR Audio for the latest version of each plugin. Cards update incrementally with `Update Available` or `Up to Date` badges as results arrive. Yellow `KVR` buttons link directly to the product page. A live status bar shows the current plugin being checked and running tallies.*

---

### `> SCAN HISTORY // DIFF ENGINE`

![History & Diff](screenshots/history.png)

*Every scan is timestamped and archived. Select any two snapshots and the diff engine shows plugins added, removed, or version-changed between them.*

---

## // CORE MODULES //

| Module | Function |
|--------|----------|
| **Plugin Scanner** | Detects VST2, VST3, and AU plugins from platform-specific directories on macOS, Windows, and Linux. Shows architecture badges (ARM64, x86_64, Universal) per plugin via direct Mach-O/PE header parsing. Tracks raw byte sizes for accurate disk usage charts. Runs in a background worker thread -- UI stays fully responsive |
| **Audio Scanner** | Discovers audio samples (WAV, FLAC, AIFF, MP3, OGG, etc.) with metadata extraction, file size formatting, and symlink deduplication. Double-click any sample row to start playback (or single-click with the setting enabled). Floating music player with volume, playback speed, seek bar, and loop controls persists across all tabs |
| **DAW Scanner** | Finds DAW project files across 14+ formats -- Ableton (.als), Logic (.logicx), FL Studio (.flp), REAPER (.rpp), Cubase/Nuendo (.cpr/.npr), Pro Tools (.ptx/.ptf), Bitwig (.bwproject), Studio One (.song), Reason (.reason), Audacity (.aup/.aup3), GarageBand (.band), Ardour (.ardour), and dawproject (.dawproject). Double-click any project row to open it directly in its DAW |
| **Plugin Cross-Reference** | Parses Ableton Live (.als) and REAPER (.rpp) project files to extract plugin references (VST2, VST3, AU, CLAP). Shows plugin count badges on DAW rows. Click to see full plugin list. Reverse lookup: right-click any plugin to find which projects use it. Build full index across all supported projects with one click |
| **Version Intel** | Reads version, manufacturer, and website URL from macOS bundle plists (`CFBundleShortVersionString`, `CFBundleIdentifier`, `NSHumanReadableCopyright`) |
| **Update Checker** | Searches [KVR Audio](https://www.kvraudio.com) for each plugin's latest version. Falls back to DuckDuckGo site-restricted KVR search. Runs in a worker thread with rate limiting and streams results back incrementally |
| **KVR Integration** | Yellow KVR button on every plugin links directly to its KVR Audio product page. Double-click any plugin card to open it on KVR. URL is constructed from plugin name + manufacturer with smart slug generation (camelCase splitting, manufacturer lookup table). Falls back to KVR search if the direct URL doesn't exist |
| **KVR Cache** | Resolved KVR data (product URLs, download links, versions) is persisted to `kvr-cache.json`. On restart, cached results are restored instantly and the background resolver resumes from where it left off |
| **Download Button** | Green download button appears on plugins with a confirmed newer version and a KVR download link (platform-specific when available) |
| **Export/Import** | Export all tabs (plugins, samples, DAW projects, presets) to JSON, TOML, CSV, or TSV via native file dialogs. Import from JSON or TOML. Format auto-detected from file extension |
| **Scan History** | Stores up to 50 scan snapshots locally (plugins, audio, and DAW scans merged) with full diff support between any two scans |
| **Batch Updater** | Walk through all outdated plugins one by one with skip/open controls |
| **Manufacturer Link** | Globe button on each plugin opens the manufacturer's website directly (derived from bundle ID). Shows a disabled icon when no website is available |
| **Reveal in Finder** | Folder button opens the plugin's filesystem location. Double-click any preset row to reveal it in Finder. Tooltip shows the full path on hover |
| **Directory Breakdown** | Expandable table showing plugin counts and type breakdown per scanned directory |
| **Stop Control** | Cancel any in-progress scan, update check, or KVR resolution without losing already-discovered results |
| **Auto-Restore** | Last scan results + KVR cache load automatically on app startup -- no need to re-scan or re-check every launch |
| **Unknown Tracking** | Plugins where no version info was found online show "Unknown Latest" badge and are counted separately from "Up to Date" |
| **Color Schemes** | Multiple themes including cyberpunk (default), light mode, and custom schemes with configurable CSS variables |
| **Fuzzy Search** | All search bars default to fuzzy matching (characters match in order, not contiguous). Toggle the `.*` button to switch to regex mode with full pattern support. Available in all tabs |
| **Favorites** | Right-click any plugin, sample, DAW project, or preset to add/remove from favorites. Dedicated Favorites tab shows all starred items with type filter, search, reveal in Finder, and remove actions. Persisted across sessions |
| **Resizable Columns** | Drag column borders to resize. Widths persist across sessions |
| **Floating Player** | Draggable audio player that docks to any corner with quadrant zone UI. Resizable from all 8 edges/corners. Play/pause, loop, shuffle, seek bar, volume, speed (0.25x-2x), recently played (50 tracks), song search with fzf matching, favorite/tag buttons. Expanded mode adds 3-band EQ, preamp gain, stereo pan, mono toggle, A-B loop. 60fps waveform playhead via requestAnimationFrame. Player state/size/dock persisted across sessions |
| **Waveform Preview** | Full-width waveform rendered in the metadata panel when clicking a sample row. Seekable — click anywhere to jump. 60fps playhead cursor via rAF. Close button on metadata panel. Expand-on-click toggle in settings |
| **Dependency Graph** | Visual plugin dependency map with search, 3 tabs (Most Used, By Project with inline drill-down + back button, Orphaned), bar charts, context menus on all rows. Prompts to build plugin index if empty. Persisted xref cache |
| **ALS XML Viewer** | Right-click any .als file to decompress and view raw XML. fzf search with highlighted matches and line numbers. Export decompressed XML to file |
| **Context Menus** | 36+ right-click context menus on every interactive element — plugins, samples, DAW projects, presets, favorites, notes, tags, history entries, audio player songs, dep graph rows, file browser, breadcrumbs, waveforms, EQ sliders, color schemes, shortcut keys, progress bars |
| **Toast Notifications** | Slide-in notifications for actions like opening DAW projects or revealing files in Finder |
| **Disk Usage** | Stacked bar charts showing space breakdown by format/type per tab. Visual representation of storage usage with color-coded legends |
| **Batch Selection** | Checkbox column in all tables for multi-item operations. Select all/deselect, batch favorite, copy paths, export selected as JSON |
| **Duplicate Detection** | Find duplicate files by name+size across plugins, samples, DAW projects, and presets. Modal report grouped by type with full paths |
| **Notes & Tags** | Add notes and comma-separated tags to any item via right-click. Notes persisted in preferences with note indicator icon on tagged items |
| **Keyboard Navigation** | Arrow keys/j/k to navigate table rows and file browser (Ableton-style: right enters dir, left goes up). 38 customizable keybindings including Cmd+E export, Cmd+I import, Cmd+Tab cycle tabs, Cmd+G dep graph, Cmd+T toggle theme |
| **Help Overlay** | Press <kbd>?</kbd> to show all 38 keyboard shortcuts. Covers navigation, playback, actions, search operators, and mouse interactions |
| **Sort Persistence** | Last-used sort column and direction saved per tab, restored on app restart |
| **Multi-Select Filters** | All filter dropdowns support multiple selections (e.g. VST2 + AU, WAV + FLAC). Checkbox-based custom dropdown with "All" toggle |
| **Native Menu Bar** | Full menu bar with File, Edit, Scan, View, Playback, Data, Window, Help menus. All functionality accessible from menus with keyboard accelerators |
| **ETA Timers** | Estimated time remaining on plugin scans and update checks. Elapsed time on audio, DAW, and preset scans |
| **Draggable Tabs** | Trello-style drag and drop for tabs and settings sections. Floating ghost follows cursor with placeholder. Order persisted across sessions |
| **Cyberpunk Visualizer** | Animated equalizer bars in the floating player with cyan-to-magenta gradient. Bars bounce when playing, freeze on pause. Border glow pulse effect |
| **PDF Export** | Export any tab to PDF (A4 landscape, auto-sized columns proportional to content, 7pt font for maximum data density, background export with toast notification) |
| **TOML Export/Import** | Export/import all tabs in TOML format alongside JSON, CSV, TSV |
| **BPM Estimation** | Estimates tempo for all audio formats (WAV, AIFF, MP3, FLAC, OGG, M4A, AAC, OPUS) using symphonia decoder + onset-strength autocorrelation. Compressed formats decoded to PCM (30s max). Shown in metadata panel with spinner. Cached in memory |
| **Parametric EQ** | Visual frequency response curve with draggable band nodes (Low/Mid/High). Real-time FFT spectrum overlay at 60fps via Web Audio AnalyserNode. Log frequency axis (20Hz-20kHz), drag to change frequency and gain simultaneously |
| **Audio Similarity Search** | Right-click any sample → "Find Similar" to find samples that sound alike. Spectral fingerprinting: RMS energy, spectral centroid, zero-crossing rate, 3-band energy split, attack time. Parallel computation via rayon. Click results to play. Shortcut: W key |
| **Full Vim Keybindings** | j/k move, gg top, G bottom, Ctrl+D/U half-page, / search, o reveal, y yank path, p play, x favorite, v select, V select-all, w find-similar. 39 total customizable keybindings including Cmd+E export, Cmd+I import, Cmd+Tab cycle tabs |
| **Command Palette** | Press <kbd>Cmd+K</kbd> to open a fuzzy search across all items — plugins, samples, DAW projects, presets, bookmarked directories, tags, tabs, and actions. Arrow keys to navigate, Enter to select, Escape to dismiss. Uses the same fzf scoring engine as tab search bars |
| **Directory Bookmarks** | Bookmark favorite directories in the File Browser for instant navigation. Chips displayed above the file list, persisted across sessions. Right-click any folder to bookmark it |
| **Quick Nav Buttons** | File browser toolbar has Desktop, Downloads, Music, Documents, and Root (/) buttons for instant navigation |
| **Cyberpunk Animations** | 30 CSS animations: neon focus pulse, button hover glow, modal zoom-in, context menu scale, format badge shimmer, toast glow pulse, neon gradient scrollbars, tactile depth shadows on every interactive element |
| **System Info** | Real-time display of CPU cores/usage, memory RSS/VIRT, PID, threads, FDs, uptime, thread pool sizes, scanner config, active scan states with green dots, data file sizes, data directory path |
| **fzf Tuning** | 8 configurable fuzzy search parameters (match score, gap penalties, boundary/camelCase/consecutive bonuses) in Settings with live preview and reset to defaults |
| **Filter Persistence** | All 6 filter dropdowns (plugin type, status, favorite type, audio format, DAW, preset format) saved to prefs and restored on startup. Multi-select values preserved |
| **Plugin Name Normalization** | Cross-reference matching normalizes plugin names: strips arch suffixes (x64, ARM64, Stereo), case-folds, collapses whitespace. "Serum", "SERUM (x64)", "serum" all match |
| **macOS Firmlink Dedup** | Scanners normalize /System/Volumes/Data paths to prevent duplicate file discovery when scanning / |
| **Browse Button** | Native folder picker in scan directory settings to grant macOS TCC permissions for mounted volumes |

---

## // QUICK START //

```bash
# Clone the repo
git clone https://github.com/MenkeTechnologies/Audio-Haxor.git
cd Audio-Haxor

# Install dependencies
pnpm install

# Run in development mode
pnpm tauri dev

# Build for distribution
pnpm tauri build
```

Requires [Node.js](https://nodejs.org/), [pnpm](https://pnpm.io/), and [Rust](https://rustup.rs/). The Tauri CLI is pulled in as a dev dependency.

---

## // DEV vs BUILD — IMPORTANT //

**Dev (`pnpm tauri dev`) and Build (`pnpm tauri build`) behave differently.** Always verify in the build before shipping.

### Differences

| | Dev | Build |
|---|---|---|
| URL scheme | `http://localhost` | `tauri://localhost` |
| CSP | Relaxed | Strict (no inline JS) |
| Frontend | Served from disk (live) | Embedded in binary |
| WebView cache | None (dev server) | Aggressive (survives app restart) |
| CSS layout | Standard web | Slightly different height inheritance |
| Canvas | `clientWidth` stable | `clientWidth` can fluctuate |

### Rules

1. **Never use inline `onclick`/`onchange`** — blocked by CSP in build. Always use `addEventListener` in JS files.
2. **Never set `canvas.width`/`canvas.height` in a render loop** — causes infinite resize loops in build. Set once, or use fixed HTML attributes with CSS `width:100%;height:100%`.
3. **Never rely on `height: 100%` cascading** — use explicit pixel values.
4. **Guard cross-file function calls** with `typeof fn === 'function'` — script load order differs.
5. **Don't use CSS classes on dynamically created elements** if the class has layout-affecting styles — use inline styles or dedicated CSS classes that only set visual properties.

### Cache Busting

Build looks different from dev? Clear **all 4** WebView cache directories:

```bash
find ~/Library/WebKit/audio-haxor \
     ~/Library/WebKit/com.menketechnologies.audio-haxor \
     ~/Library/Caches/audio-haxor \
     ~/Library/Caches/com.menketechnologies.audio-haxor \
     -delete 2>/dev/null
```

Also bump the `?v=XXX` query strings on all `<script>` tags in `index.html` to bust compiled JS bytecode cache.

### Build Commands

```bash
# Normal build (~14s)
pnpm tauri build

# Clean build (when frontend changes aren't picked up, ~40s)
cargo clean --manifest-path src-tauri/Cargo.toml --release && pnpm tauri build

# Nuclear option: clear caches + clean build
find ~/Library/WebKit/audio-haxor ~/Library/WebKit/com.menketechnologies.audio-haxor \
     ~/Library/Caches/audio-haxor ~/Library/Caches/com.menketechnologies.audio-haxor \
     -delete 2>/dev/null
cargo clean --manifest-path src-tauri/Cargo.toml --release && pnpm tauri build
```

### Data Location

All preferences, scan history, and KVR cache persist at:
```
~/Library/Application Support/com.menketechnologies.audio-haxor/
```
This directory survives app reinstalls. Never deleted by builds.

---

## // TESTING //

```bash
# Run all tests (JS + Rust)
pnpm test

# Run Rust backend tests only
cd src-tauri && cargo test

# Run JavaScript unit tests only
node --test test/scanner.test.js test/update-worker.test.js test/ui.test.js
```

### Rust tests (234 tests)

| Module | Tests | Coverage |
|--------|-------|----------|
| **lib** | 51 | Export/import roundtrips (JSON, TOML, CSV, TSV), preset export/import, DAW/audio import, CSV escaping, file ops (list dir, rename, delete), `.band` validation, serde payloads |
| **history** | 35 | Scan CRUD, 50-scan limit, diff (added/removed/version-changed), KVR cache CRUD, audio history CRUD, audio diff, DAW history CRUD, DAW diff, ID generation, preference storage |
| **kvr** | 27 | Version parsing, version comparison, HTML version extraction (6 formats), download URL extraction, platform keyword detection, date filtering |
| **scanner** | 26 | Plugin type mapping, file size formatting, directory size calculation, plugin discovery, VST directory enumeration, architecture detection, edge cases |
| **audio_scanner** | 28 | Audio file discovery, metadata extraction (WAV/FLAC/AIFF), format size formatting, symlink deduplication, directory walking, stop signal, skip directories, batching, scan completeness (all files found across nested dirs), deep nesting (20 levels), mixed format detection, stop-midway partial results, exclude paths, simulated SMB/NFS (880 files across 3 roots, zero dropped, throughput verified), unreadable file resilience, concurrent scan isolation (4 parallel scans) |
| **daw_scanner** | 19 | DAW project discovery, extension-to-DAW mapping (14 DAW types), file size formatting, directory walking, stop signal, skip directories |
| **xref** | 25 | Ableton .als gzip XML parsing (VST2/VST3/AU), REAPER .rpp plaintext parsing (VST/VST3/AU/CLAP), plugin name normalization (arch/platform suffix stripping, case folding, whitespace collapse), case-insensitive deduplication, sorting, error handling, empty projects, .rpp-bak support |
| **bpm** | 16 | WAV/AIFF PCM reading, onset-strength autocorrelation, click track detection (90/120/140/174 BPM), silence rejection, short file handling, 8/16/24-bit decode, stereo mixdown, extra chunk handling, AIFF parsing |
| **preset_scanner** | 6 | Preset discovery, directory walking, stop signal, format detection, batching |
| **similarity** | 9 | Fingerprint distance (identical=0, different>0.5), similar-kicks-closer-than-kick-hihat, sorted results, self-exclusion, max results cap, nonexistent/unsupported file handling, WAV fingerprint computation |

### JavaScript tests (265 tests)

| Module | Tests | Coverage |
|--------|-------|----------|
| **ui** | 208 | `escapeHtml` (null, numeric, double-escaping), `escapePath` (spaces, quotes, unicode), `slugify` (camelCase, numbers, special chars), `buildKvrUrl` (spaces, parens, empty manufacturer), `formatAudioSize`, `formatTime` (0s, 60s, 3661s, negative), `getFormatClass` (all formats), `timeAgo` (seconds, minutes, hours, days), `kvrCacheKey` (special chars, unicode), `buildDirsTable`, `applyKvrCache`, `metaItem`, `buildPluginCardHtml`, `normalizePluginName` (case folding, arch suffix stripping, whitespace collapse, bracket/bare variants) |
| **scanner** | 110 | Plugin type mapping (`.vst`/`.vst3`/`.component`/`.dll`/`.aaxplugin`/`.clap`), file size formatting (0B, 1B, 1023B, 1TB), DAW type mapping (all 14+ formats), audio format detection (WAV, MP3, FLAC, M4A, AAC, OPUS, REX) |
| **update-worker** | 27 | Version parsing (pre-release, leading v, multi-part), version comparison (identical, long versions), KVR URL builder (slug generation, manufacturer suffix) |

---

## // BENCHMARKS //

Criterion micro-benchmarks on Apple M5 Max (18 cores, 64 GB):

| Operation | Time |
|-----------|------|
| `parse_version("1.2.3")` | 25 ns |
| `compare_versions` | 46 ns |
| `extract_version` (HTML) | 211 ns |
| `extract_version` (7KB HTML) | 2.5 µs |
| `extract_download_url` | 778 ns |
| `format_size` | 80 ns |
| `daw_ext_matches` | 38 ns |
| `daw_name_for_format` | 1.1 ns |
| `get_plugin_type` | 25 ns |
| `get_audio_metadata` (WAV) | 4.8 µs |
| `gen_id` | 42 ns |

Scanner architecture: each scanner (plugins, audio, DAW, presets) runs on its own dedicated rayon thread pool (`num_cpus × 2` threads, min 4) with `sync_channel(2048)` buffering and non-blocking `try_recv` drain loops. All 4 scanners run fully in parallel via `Promise.all`. Optimized for I/O-bound workloads including SMB/NFS network mounts where high thread counts overlap network round-trip latency.

```bash
cargo bench --manifest-path src-tauri/Cargo.toml
```

---

## // BUILD & DISTRIBUTE //

```bash
# Build optimized release bundle
pnpm tauri:build
```

Built packages land in `src-tauri/target/release/bundle/`:

| Platform | Format | Output |
|----------|--------|--------|
| macOS    | App Bundle | `AUDIO_HAXOR.app` |
| macOS    | DMG    | `AUDIO_HAXOR_x.x.x_aarch64.dmg` |
| Windows  | NSIS   | `AUDIO_HAXOR Setup x.x.x.exe` |
| Windows  | MSI    | `AUDIO_HAXOR_x.x.x_x64_en-US.msi` |
| Linux    | AppImage | `audio-haxor_x.x.x_amd64.AppImage` |
| Linux    | Debian   | `audio-haxor_x.x.x_amd64.deb` |

> Each platform builds natively. The macOS DMG is unsigned -- users right-click → Open on first launch to bypass Gatekeeper.

---

## // HOW IT WORKS //

```
[1] SCAN -----> Rust backend crawls platform-specific plugin directories.
                Streams results to the WebView via Tauri events in batches
                of 10. Collects name, type, version, manufacturer, website,
                size, and mod date.

[2] AUDIO ----> Separate scanner discovers audio samples (WAV, FLAC, AIFF,
                MP3, OGG) with metadata extraction. Inline playback via
                Tauri's native asset:// protocol for zero-latency local
                file streaming.

[3] DAW ------> DAW scanner finds project files across 14+ DAW formats
                (Ableton, Logic, FL Studio, REAPER, Cubase, Pro Tools,
                Bitwig, Studio One, Reason, Audacity, GarageBand, Ardour,
                dawproject). Tracks file sizes and modification dates.

[4] CHECK ----> Async Rust tasks search KVR Audio for each plugin's product
                page, scrape version info. Falls back to DuckDuckGo
                site:kvraudio.com search. Groups by manufacturer to reduce
                duplicate queries. Cards update in-place as results arrive.
                Status bar shows current plugin and live tallies.

[5] HISTORY --> Each scan is persisted to disk as JSON. Plugin, audio, and
                DAW scans are merged in the history timeline. Diff any two
                snapshots to see what was added, removed, or version-bumped.
                Last scan auto-restores on startup.

[6] EXPORT ---> Export any tab to JSON, TOML, CSV, TSV, or PDF via native
                file dialogs. Import from JSON or TOML.
```

---

## // KVR AUDIO RATE LIMITING //

This app queries [KVR Audio](https://www.kvraudio.com) to find plugin versions and
download links. To avoid overloading KVR's servers, strict rate limiting is enforced:

| Setting | Value |
|---------|-------|
| Concurrent requests | 1 (sequential) |
| Delay between plugins | 2 seconds |
| Delay between KVR page fetches | 1.5 seconds |
| Delay before DuckDuckGo fallback | 1.5 seconds |
| Max product pages checked per plugin | 2 |

The background KVR resolver (auto-runs on startup) also uses 2-second delays
between plugins. You can stop it anytime with the Stop button.

> With ~2600 plugins at 2s each, a full update check takes roughly 90 minutes.
> Results are cached on each plugin card -- subsequent clicks are instant.
> Use the Stop button to cancel early; already-resolved plugins keep their results.

---

## // PROJECT ARCHITECTURE //

```
src-tauri/
  src/
    main.rs            -- Tauri entry point
    lib.rs             -- IPC command handlers, export/import, file ops
    bpm.rs             -- BPM estimation via onset-strength autocorrelation
    scanner.rs         -- Plugin filesystem scanner (VST2/VST3/AU)
    audio_scanner.rs   -- Audio sample discovery + metadata extraction
    daw_scanner.rs     -- DAW project scanner (14+ formats)
    preset_scanner.rs  -- Plugin preset discovery
    similarity.rs      -- Audio similarity search via spectral fingerprinting
    xref.rs            -- Plugin ↔ DAW cross-reference engine
    history.rs         -- Scan history persistence + diff engine
    kvr.rs             -- KVR Audio scraper + version checker
  Cargo.toml           -- Rust dependencies
  tauri.conf.json      -- Tauri app configuration + CSP + bundling

frontend/
  index.html           -- Cyberpunk CRT UI (HTML/CSS)
  js/
    app.js             -- Startup, auto-load last scan, restore preferences
    audio.js           -- Audio sample scanning + inline playback + floating player
    batch-select.js    -- Checkbox selection + batch operations
    command-palette.js -- Cmd+K universal fuzzy search across all items
    columns.js         -- Resizable table columns with width persistence
    context-menu.js    -- Right-click context menus for all elements
    daw.js             -- DAW project scanning + stats
    disk-usage.js      -- Stacked bar charts for storage breakdown
    duplicates.js      -- Duplicate detection modal
    export.js          -- Export/import (JSON/TOML/CSV/TSV/PDF)
    favorites.js       -- Favorites management
    file-browser.js    -- Filesystem navigation with tags + notes
    help-overlay.js    -- Keyboard shortcuts reference overlay
    history.js         -- Scan history management + merged timeline
    ipc.js             -- Tauri v2 IPC bridge + event delegation
    keyboard-nav.js    -- Arrow key / j/k table row navigation
    kvr.js             -- KVR Audio resolver + cache management
    multi-filter.js    -- Multi-select checkbox dropdowns
    notes.js           -- Note editor + tag management + tag cloud
    plugins.js         -- Plugin scanning, filtering, sorting, updates
    presets.js         -- Preset scanning + filtering
    settings.js        -- Color schemes, themes, toggles, preferences
    shortcuts.js       -- Customizable keyboard shortcuts
    sort-persist.js    -- Sort column/direction persistence per tab
    utils.js           -- fzf search, escaping, slugs, formatting
    xref.js            -- Plugin ↔ DAW cross-reference UI + index
    dep-graph.js       -- Plugin dependency graph visualization

test/
  scanner.test.js      -- Plugin/audio/DAW type mapping, size formatting
  update-worker.test.js -- Version comparison, KVR URL builder
  ui.test.js           -- UI helper functions, HTML builders
```

---

## // SUPPORTED DIRECTORIES //

```
macOS
  /Library/Audio/Plug-Ins/VST
  /Library/Audio/Plug-Ins/VST3
  /Library/Audio/Plug-Ins/Components
  ~/Library/Audio/Plug-Ins/VST
  ~/Library/Audio/Plug-Ins/VST3
  ~/Library/Audio/Plug-Ins/Components

Windows
  C:\Program Files\Common Files\VST3
  C:\Program Files\VSTPlugins
  C:\Program Files\Steinberg\VSTPlugins
  C:\Program Files (x86)\Common Files\VST3
  C:\Program Files (x86)\VSTPlugins
  C:\Program Files (x86)\Steinberg\VSTPlugins

Linux
  /usr/lib/vst
  /usr/lib/vst3
  /usr/local/lib/vst
  /usr/local/lib/vst3
  ~/.vst
  ~/.vst3
```

---

## // LICENSE //

ISC

## // AUTHOR //

Created by [MenkeTechnologies](https://github.com/MenkeTechnologies)
