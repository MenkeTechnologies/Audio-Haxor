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

> **// SYSTEM ONLINE -- AUDIO_HAXOR v1.1.0 // by MenkeTechnologies**

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
| **Plugin Scanner** | Detects VST2, VST3, and AU plugins from platform-specific directories on macOS, Windows, and Linux. Shows architecture badges (ARM64, x86_64, Universal) per plugin via `lipo -archs` with Mach-O fallback. Runs in a background worker thread -- UI stays fully responsive |
| **Audio Scanner** | Discovers audio samples (WAV, FLAC, AIFF, MP3, OGG, etc.) with metadata extraction, file size formatting, and symlink deduplication. Double-click any sample row to start playback (or single-click with the setting enabled). Floating music player with volume, playback speed, seek bar, and loop controls persists across all tabs |
| **DAW Scanner** | Finds DAW project files across 14+ formats -- Ableton (.als), Logic (.logicx), FL Studio (.flp), REAPER (.rpp), Cubase/Nuendo (.cpr/.npr), Pro Tools (.ptx/.ptf), Bitwig (.bwproject), Studio One (.song), Reason (.reason), Audacity (.aup/.aup3), GarageBand (.band), Ardour (.ardour), and dawproject (.dawproject). Double-click any project row to open it directly in its DAW |
| **Version Intel** | Reads version, manufacturer, and website URL from macOS bundle plists (`CFBundleShortVersionString`, `CFBundleIdentifier`, `NSHumanReadableCopyright`) |
| **Update Checker** | Searches [KVR Audio](https://www.kvraudio.com) for each plugin's latest version. Falls back to DuckDuckGo site-restricted KVR search. Runs in a worker thread with rate limiting and streams results back incrementally |
| **KVR Integration** | Yellow KVR button on every plugin links directly to its KVR Audio product page. Double-click any plugin card to open it on KVR. URL is constructed from plugin name + manufacturer with smart slug generation (camelCase splitting, manufacturer lookup table). Falls back to KVR search if the direct URL doesn't exist |
| **KVR Cache** | Resolved KVR data (product URLs, download links, versions) is persisted to `kvr-cache.json`. On restart, cached results are restored instantly and the background resolver resumes from where it left off |
| **Download Button** | Green download button appears on plugins with a confirmed newer version and a KVR download link (platform-specific when available) |
| **Export/Import** | Export plugin lists to JSON, CSV, or TSV via native file dialogs. Import previously exported scans |
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
| **Floating Player** | Draggable audio player that docks to any corner (drag the handle or toolbar to reposition, snaps to nearest quadrant). Features play/pause, loop, seek bar, volume slider, and playback speed (0.25x-2x). Double-click to expand with recently played history (up to 50 tracks). Dock position persists across sessions |
| **Context Menus** | Right-click context menus on every interactive element -- plugins (KVR, manufacturer, reveal, copy), samples (play, loop, reveal, copy), DAW projects (open in DAW, reveal, copy), presets (reveal, copy), and history entries (view, delete) |
| **Toast Notifications** | Slide-in notifications for actions like opening DAW projects or revealing files in Finder |

---

## // QUICK START //

```bash
# Clone the repo
git clone https://github.com/MenkeTechnologies/universal-plugin-update-manager.git
cd universal-plugin-update-manager

# Install dependencies
pnpm install

# Run in development mode
pnpm tauri:dev

# Build for distribution
pnpm tauri:build
```

Requires [Node.js](https://nodejs.org/), [pnpm](https://pnpm.io/), and [Rust](https://rustup.rs/). The Tauri CLI is pulled in as a dev dependency.

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

### Rust tests (118 tests)

| Module | Tests | Coverage |
|--------|-------|----------|
| **kvr** | 27 | Version parsing, version comparison, HTML version extraction (6 formats), download URL extraction, platform keyword detection, date filtering |
| **history** | 19 | Scan CRUD, 50-scan limit, diff (added/removed/version-changed), KVR cache CRUD, audio history CRUD, audio diff, ID generation |
| **audio_scanner** | 18 | Audio file discovery, metadata extraction (WAV/FLAC/AIFF), format size formatting, symlink deduplication, directory walking, stop signal, skip directories |
| **daw_scanner** | 39 | DAW project discovery, extension-to-DAW mapping (14 DAW types), file size formatting, directory walking, stop signal, skip directories |
| **scanner** | 15 | Plugin type mapping, file size formatting, directory size calculation, plugin discovery, VST directory enumeration |

### JavaScript tests (207 tests)

| Module | Tests | Coverage |
|--------|-------|----------|
| **ui** | 96 | `escapeHtml`, `escapePath`, `slugify`, `buildKvrUrl`, `formatAudioSize`, `formatTime`, `getFormatClass`, `timeAgo`, `kvrCacheKey`, `buildDirsTable`, `applyKvrCache`, `metaItem`, `buildPluginCardHtml` |
| **update-worker** | 17 | Version parsing, version comparison, KVR URL builder (slug generation, manufacturer suffix) |
| **scanner** | 94 | Plugin type mapping (`.vst`/`.vst3`/`.component`/`.dll`), file size formatting, DAW type mapping, audio format detection |

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

[6] EXPORT ---> Export plugin lists to JSON, CSV, or TSV via native file
                dialogs. Import previously exported scans.
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
    lib.rs             -- IPC command handlers + app setup
    scanner.rs         -- Plugin filesystem scanner (VST2/VST3/AU)
    audio_scanner.rs   -- Audio sample discovery + metadata extraction
    daw_scanner.rs     -- DAW project scanner (14+ formats)
    history.rs         -- Scan history persistence + diff engine
    kvr.rs             -- KVR Audio scraper + version checker
  Cargo.toml           -- Rust dependencies
  tauri.conf.json      -- Tauri app configuration + CSP + bundling

frontend/
  index.html           -- Cyberpunk CRT UI (HTML/CSS)
  js/
    app.js             -- Startup, auto-load last scan, restore preferences
    audio.js           -- Audio sample scanning + inline playback
    columns.js         -- Resizable table columns with width persistence
    context-menu.js    -- Right-click context menus for all tabs
    daw.js             -- DAW project scanning + stats
    export.js          -- Export/import plugins (JSON/CSV/TSV)
    history.js         -- Scan history management + merged timeline
    ipc.js             -- Tauri v2 IPC bridge + event delegation
    kvr.js             -- KVR Audio resolver + cache management
    plugins.js         -- Plugin scanning, filtering, sorting, updates
    settings.js        -- Color schemes + theme switching
    utils.js           -- Shared helpers (escaping, slugs, formatting)

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
