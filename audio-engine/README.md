# audio-engine

Sidecar binary for **AUDIO_HAXOR**: **JUCE 8** (`AudioDeviceManager`, `AudioTransportSource`, `AudioFormatReader`) for **input/output** device discovery, **library file playback** with 3-band EQ / gain / pan, **VST3** + **AU** (macOS) **plugin scanning** via `KnownPluginList` + `PluginDirectoryScanner`, **native plug-in editor windows** (`DocumentWindow` + `AudioProcessorEditor`), and a **persistent** stdin line protocol (one JSON request in → one JSON line out). Requests run on the **JUCE message thread** (`MessageManager::callAsync` from a stdin reader thread; **main** runs `runDispatchLoop()`).

## License (JUCE)

JUCE is distributed under the **GPLv3** (or a **commercial** license from Raw Material Software). Building and distributing this sidecar **without** a JUCE commercial license generally implies **GPL obligations** for the combined work. Confirm your licensing model before shipping binaries.

## Protocol

### JSON responses (`okObj()`)

**Never** call `okObj().getDynamicObject()` on the temporary `var` — the `var` is destroyed at the end of the full-expression, so the `DynamicObject*` dangles. Keep a named `juce::var out = okObj();`, mutate `out.getDynamicObject()`, and `return out`.

### Crash diagnostics

The sidecar installs **POSIX signal handlers** (SIGSEGV, SIGBUS, SIGILL, SIGFPE, SIGABRT) and **`std::terminate`**: on fatal signals it writes **`ENGINE [fatal signal …]`** plus a **`backtrace_symbols_fd`** stack to **stderr** and, when **`AUDIO_HAXOR_APP_LOG`** is set, appends the same to that file (raw lines, not the usual `[timestamp] ENGINE:` prefix). **SIGPIPE is ignored** so a closed stdout/pipe does not kill the process silently before the next write fails. After a crash, the host’s **`audio-engine IPC error`** line often includes a **stderr tail** with this text. For symbol names, link with **`-rdynamic`** on Linux if stacks are opaque.

The Tauri host sets **`AUDIO_HAXOR_APP_LOG`** to the absolute path of **`app.log`** in the app data directory when it spawns the sidecar. The sidecar appends lines as **`[YYYY-MM-DD HH:MM:SS] ENGINE: …`** (UTC timestamp; same bracket style as the host’s `write_app_log`, 5MB rotate to **`app.log.1`**). Typical messages: startup, **`stdin` parse failure**, one line per request with the command name (for example **`cmd list_output_devices`**) except **`ping`** (to avoid UI polling noise), and exit when stdin closes. For a **standalone** binary (no env var), file logging is a no-op.

Each line is a JSON object with at least `cmd`. Optional fields include `device_id`, `tone` (bool, output only), `buffer_frames` (positive `u32`, **capped at 8192**), **`sample_rate_hz`** (positive integer, optional on **`start_output_stream`** / **`start_input_stream`**), and **`start_playback`** (bool, output only): when `true` after **`playback_load`**, output is driven by **JUCE** transport on the selected device (not the test-tone path). Omit **`sample_rate_hz`** to let the driver pick a rate (or, for playback without an explicit rate, the file’s sample rate is used when **`start_playback`** is set).

### Library playback (JUCE + DSP)

**`playback_load`** opens the path with **`AudioFormatManager`** (`registerBasicFormats`). Duration and source sample rate come from the reader. **`start_output_stream`** with **`start_playback: true`** wires **`AudioTransportSource`** → **`AudioSourcePlayer`** → **`AudioDeviceManager`** output callback. DSP (EQ / gain / pan) runs in **`DspStereoFileSource`**, then **optional VST3 / AU inserts** (see **`playback_set_inserts`**), before audio reaches the device. **Reverse playback** uses a sample-wise path — **inserts are skipped** in that mode.

| Command | Fields | Purpose |
|--------|--------|---------|
| `playback_load` | `path` (absolute) | Open file; store session; does **not** open output. |
| `start_output_stream` | `start_playback: true`, `device_id`, optional `buffer_frames`, optional `sample_rate_hz` | After **`playback_load`**, start transport on the device. |
| `playback_pause` | `paused` (bool) | Pause / resume transport. |
| `playback_seek` | `position_sec` | Seek (seconds on the forward timeline). |
| `playback_set_dsp` | `gain`, `pan`, `eq_low_db`, `eq_mid_db`, `eq_high_db` | Update DSP parameters. |
| `playback_set_speed` | `speed` (float) | Accepted; **rate change is not wired** — response may include a **note** (no resampler yet). |
| `playback_set_reverse` | `reverse` (bool) | When `true`, full-decode-to-RAM reverse path for the next playback. |
| `playback_status` | — | Position, duration, peak, pause, EOF, reverse, sample rates. |
| `playback_stop` | — | Stop transport and clear session. |
| `playback_set_inserts` | `paths` (JSON array of absolute `.vst3` / `.component` paths, max 8, empty clears) | Load **VST3** / **AU** (macOS) effect instances in order. **Requires** `stop_output_stream` first (no hot-swap while the device is open). Closes any open insert editor windows. |
| `playback_open_insert_editor` | `slot` (int, 0-based index into the **loaded** insert chain) | Opens a **native** editor for that instance (`createEditorIfNeeded`). Fails with `plugin has no editor` if the plug-in has no UI. |
| `playback_close_insert_editor` | `slot` (int) | Closes the hosted editor window for that chain index, if open. |

`stop_output_stream` tears down the output device graph and clears playback as needed.

| Command | Purpose |
|--------|---------|
| `ping` | Version + host id |
| `engine_state` | Aggregated stream snapshot |
| `list_output_devices` / `list_input_devices` | Enumerate devices (uses the active JUCE device type; falls back across types if the current type lists nothing) |
| `list_audio_device_types` | Lists JUCE driver types: `types` is a **JSON array of strings** (driver names); `current` is the active type |
| `set_audio_device_type` | `type` (string) — switches **AudioDeviceManager** driver; **stops** active output/input streams |
| `get_output_device_info` / `get_input_device_info` | Default config + `buffer_size` + `sample_rates` + `buffer_sizes` + `audio_device_type` |
| `set_output_device` / `set_input_device` | Validate `device_id` |
| `start_output_stream` / `stop_output_stream` | Output; optional tone or file playback |
| `start_input_stream` / `stop_input_stream` | Input; peak meter |
| `output_stream_status` / `input_stream_status` | Status lines |
| `set_output_tone` | 440 Hz sine when F32 output + not in file playback mode |
| `plugin_chain` | Scanned plugin list + **`insert_paths`** / **`slots`**. First request starts a **background** VST3/AU directory scan; responses use **`phase`: `"scanning"`** until done, then **`phase`: `"juce"`** with **`plugins`**. **`scanNextFile`** is wrapped in **try/catch** (bad plug-ins skipped); JUCE **dead-man's-pedal** file: `~/Library/Application Support/MenkeTechnologies/audio-haxor/plugin-scan-dead-mans-pedal.txt` (macOS) defers crashy modules to the end on later runs. **Native crashes** inside a vendor binary cannot be caught in-process. |

## Build

**Prerequisites:** **CMake** ≥ 3.22, **Ninja**, and a C++20 toolchain. Platform libs (e.g. **ALSA** on Linux) must match your JUCE audio backend expectations.

From the **repository root**:

```bash
# Debug (matches pnpm tauri dev — beforeDevCommand)
node scripts/build-audio-engine.mjs

# Release (matches prepare sidecar)
AUDIO_ENGINE_BUILD_TYPE=release node scripts/build-audio-engine.mjs
```

Artifacts land at **`target/debug/audio-engine`** or **`target/release/audio-engine`** (same layout as the old Cargo output). **Release** bundles use `scripts/prepare-audio-engine-sidecar.mjs` → `src-tauri/binaries/audio-engine-<triple>` for Tauri `externalBin`.

### Linux (typical)

```bash
sudo apt-get install -y cmake ninja-build build-essential \
  libasound2-dev libfreetype6-dev libx11-dev libxinerama-dev libxrandr-dev libxcursor-dev libgl1-mesa-dev
```

### Device types (`list_audio_device_types`)

Use **`createAudioDeviceTypes()`** into a **local** `OwnedArray` — do **not** call **`getAvailableDeviceTypes()`** for this IPC. The latter runs **`scanDevicesIfNeeded()`** on the manager’s internal list; with **two** `AudioDeviceManager` instances (output + input) in the same process, that path can destabilize or crash the sidecar on some platforms (macOS), which shows up in the host as **`sidecar closed stdout`** after a respawn.

The **`types`** field is a **JSON array of strings** (one name per driver). Older builds used an array of `{ "type": "…" }` objects; the UI accepts both.

## Stale sidecar / `unknown cmd`

The app keeps **one** long-lived `audio-engine` child. After rebuilding, an old process can still answer stdin until replaced. The Tauri host respawns when the resolved binary’s **size/mtime** changes. If IPC looks wrong, quit the app or set **`AUDIO_HAXOR_AUDIO_ENGINE`** to an absolute path to a fresh `target/debug/audio-engine` or `target/release/audio-engine`.

## Host app (WEB UI)

`frontend/js/audio-engine.js` drives the Audio Engine tab and coordinates **`playback_*`** with the floating player. Behavior matches the root **`README.md`** Audio Engine / dev-vs-build sections (IPC guards, `engine_state` resync, input peak polling, etc.). The **panel refresh** completes after device lists and caps; **VST3/AU directory scan** (`plugin_chain` → `phase: scanning`) continues in the **background** so the tab does not stay on “loading” for the full scan duration.
