```
 ██▓███   ██▓     █    ██   ▄████  ██▓ ███▄    █
▓██░  ██▒▓██▒     ██  ▓██▒ ██▒ ▀█▒▓██▒ ██ ▀█   █
▓██░ ██▓▒▒██░    ▓██  ▒██░▒██░▄▄▄░▒██▒▓██  ▀█ ██▒
▒██▄█▓▒ ▒▒██░    ▓▓█  ░██░░▓█  ██▓░██░▓██▒  ▐▌██▒
▒██▒ ░  ░░██████▒▒▒█████▓ ░▒▓███▀▒░██░▒██░   ▓██░
▒▓▒░ ░  ░░ ▒░▓  ░░▒▓▒ ▒ ▒  ░▒   ▒ ░▓  ░ ▒░   ▒ ▒
░▒ ░      ░ ░ ▒  ░░░▒░ ░ ░  ░   ░  ▒ ░░ ░░   ░ ▒░
░░          ░ ░    ░░░ ░ ░░ ░   ░  ▒ ░   ░   ░ ░
              ░  ░   ░          ░  ░           ░

 █    ██  ██▓███  ▓█████▄  ▄▄▄     ▄▄▄█████▓▓█████
 ██  ▓██▒▓██░  ██▒▒██▀ ██▌▒████▄   ▓  ██▒ ▓▒▓█   ▀
▓██  ▒██░▓██░ ██▓▒░██   █▌▒██  ▀█▄ ▒ ▓██░ ▒░▒███
▓▓█  ░██░▒██▄█▓▒ ▒░▓█▄   ▌░██▄▄▄▄██░ ▓██▓ ░ ▒▓█  ▄
▒▒█████▓ ▒██▒ ░  ░░▒████▓  ▓█   ▓██▒ ▒██▒ ░ ░▒████▒
░▒▓▒ ▒ ▒ ▒▓▒░ ░  ░ ▒▒▓  ▒  ▒▒   ▓▒█░ ▒ ░░   ░░ ▒░ ░
 ███▄ ▄███▓ ▄▄▄       ███▄    █  ▄▄▄        ▄████ ▓█████  ██▀███
▓██▒▀█▀ ██▒▒████▄     ██ ▀█   █ ▒████▄     ██▒ ▀█▒▓█   ▀ ▓██ ▒ ██▒
▓██    ▓██░▒██  ▀█▄   ▓██  ▀█ ██▒▒██  ▀█▄  ▒██░▄▄▄░▒███   ▓██ ░▄█ ▒
▒██    ▒██ ░██▄▄▄▄██  ▓██▒  ▐▌██▒░██▄▄▄▄██ ░▓█  ██▓▒▓█  ▄ ▒██▀▀█▄
▒██▒   ░██▒ ▓█   ▓██▒ ▒██░   ▓██░ ▓█   ▓██▒░▒▓███▀▒░▒████▒░██▓ ▒██▒
```

> **// SYSTEM ONLINE -- UNIVERSAL PLUGIN UPDATE MANAGER v1.0.0 // by MenkeTechnologies**

A high-voltage Electron desktop app that jacks into your system's audio plugin directories, maps every VST2/VST3/AU module it finds, checks the web for the latest versions, and maintains a full changelog of every scan -- so nothing slips through the cracks. Cyberpunk CRT interface with neon glow, scanline overlays, and glitch effects.

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
| **Plugin Scanner** | Detects VST2, VST3, and AU plugins from platform-specific directories on macOS, Windows, and Linux. Runs in a background worker thread -- UI stays fully responsive |
| **Version Intel** | Reads version, manufacturer, and website URL from macOS bundle plists (`CFBundleShortVersionString`, `CFBundleIdentifier`, `NSHumanReadableCopyright`) |
| **Update Checker** | Searches [KVR Audio](https://www.kvraudio.com) for each plugin's latest version. Falls back to DuckDuckGo site-restricted KVR search. Runs in a worker thread with rate limiting and streams results back incrementally |
| **KVR Integration** | Yellow KVR button on every plugin links directly to its KVR Audio product page. URL is constructed from plugin name + manufacturer with smart slug generation (camelCase splitting, manufacturer lookup table). Falls back to KVR search if the direct URL doesn't exist |
| **KVR Cache** | Resolved KVR data (product URLs, download links, versions) is persisted to `kvr-cache.json`. On restart, cached results are restored instantly and the background resolver resumes from where it left off |
| **Download Button** | Green download button appears on plugins with a confirmed newer version and a KVR download link (platform-specific when available) |
| **Scan History** | Stores up to 50 scan snapshots locally with full diff support between any two scans |
| **Batch Updater** | Walk through all outdated plugins one by one with skip/open controls |
| **Manufacturer Link** | Globe button on each plugin opens the manufacturer's website directly (derived from bundle ID). Shows a disabled icon when no website is available |
| **Reveal in Finder** | Folder button opens the plugin's filesystem location. Tooltip shows the full path on hover |
| **Directory Breakdown** | Expandable table showing plugin counts and type breakdown per scanned directory |
| **Stop Control** | Cancel any in-progress scan, update check, or KVR resolution without losing already-discovered results |
| **Auto-Restore** | Last scan results + KVR cache load automatically on app startup -- no need to re-scan or re-check every launch |
| **Unknown Tracking** | Plugins where no version info was found online show "Unknown Latest" badge and are counted separately from "Up to Date" |

---

## // QUICK START //

```bash
# Clone the repo
git clone https://github.com/MenkeTechnologies/universal-plugin-update-manager.git
cd universal-plugin-update-manager

# Install dependencies
npm install

# Boot the system
npm start
```

Requires [Node.js](https://nodejs.org/) and npm. Electron is pulled in as a dev dependency.

---

## // TESTING //

```bash
# Run all unit tests
npm test
```

Tests cover:
- **History module** -- save, retrieve, delete, clear, 50-scan limit, diff (added/removed/version-changed), latest scan retrieval
- **Scanner helpers** -- plugin type mapping (`.vst`/`.vst3`/`.component`/`.dll`), file size formatting
- **Update worker logic** -- version parsing, version comparison, KVR URL builder (slug generation, manufacturer suffix)

Tests use Node.js built-in test runner (`node --test`) -- no external test framework needed.

---

## // BUILD & DISTRIBUTE //

```bash
# Build for all platforms (macOS, Windows, Linux)
npm run dist

# Build for a single platform
npm run dist:mac      # DMG + ZIP (universal binary)
npm run dist:win      # NSIS installer + portable EXE
npm run dist:linux    # AppImage + .deb package
```

Built packages land in `dist/`:

| Platform | Format | Output |
|----------|--------|--------|
| macOS    | DMG    | `Universal Plugin Update Manager-x.x.x-universal.dmg` |
| macOS    | ZIP    | `Universal Plugin Update Manager-x.x.x-universal-mac.zip` |
| Windows  | NSIS   | `Universal Plugin Update Manager Setup x.x.x.exe` |
| Windows  | Portable | `Universal Plugin Update Manager x.x.x.exe` |
| Linux    | AppImage | `Universal Plugin Update Manager-x.x.x.AppImage` |
| Linux    | Debian   | `universal-plugin-update-manager_x.x.x_amd64.deb` |

> Cross-compilation: macOS can build for all three platforms. Windows and Linux
> can only build for their own platform natively.

---

## // HOW IT WORKS //

```
[1] SCAN -----> Background worker crawls platform-specific plugin directories.
                Streams results to the UI in batches of 10. Collects name,
                type, version, manufacturer, website, size, and mod date.

[2] CHECK ----> Worker thread searches KVR Audio for each plugin's product
                page, scrapes version info. Falls back to DuckDuckGo
                site:kvraudio.com search. Groups by manufacturer to reduce
                duplicate queries. Cards update in-place as results arrive.
                Status bar shows current plugin and live tallies.

[3] HISTORY --> Each scan is persisted to disk. Diff any two snapshots
                to see what was added, removed, or version-bumped.
                Last scan auto-restores on startup.
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
main.js            -- Electron main process + IPC handlers + worker management
preload.js         -- Context bridge exposing APIs to the renderer
scanner.js         -- Plugin filesystem scanner (sync, legacy)
scanner-worker.js  -- Worker thread for non-blocking plugin scanning
update-worker.js   -- Worker thread for version checking via KVR Audio
history.js         -- Scan history persistence + diff engine
index.html         -- Single-file cyberpunk UI (HTML/CSS/JS)
test/              -- Unit tests (node --test)
  history.test.js  -- History CRUD, diff, limits
  scanner.test.js  -- Plugin type mapping, size formatting
  update-worker.test.js -- Version comparison, KVR URL builder
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
