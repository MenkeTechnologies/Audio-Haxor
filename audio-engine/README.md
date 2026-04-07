# audio-engine

Sidecar binary for **AUDIO_HAXOR**: **JUCE 8** (`AudioDeviceManager`, `AudioTransportSource`, `AudioFormatReader`) for **input/output** device discovery, **library file playback** with 3-band EQ / gain / pan, **VST3** + **AU** (macOS) **plugin scanning** via `KnownPluginList` + `PluginDirectoryScanner`, and a **persistent** stdin line protocol (one JSON request in → one JSON line out).

## License (JUCE)

JUCE is distributed under the **GPLv3** (or a **commercial** license from Raw Material Software). Building and distributing this sidecar **without** a JUCE commercial license generally implies **GPL obligations** for the combined work. Confirm your licensing model before shipping binaries.

## Protocol

Each line is a JSON object with at least `cmd`. Optional fields include `device_id`, `tone` (bool, output only), `buffer_frames` (positive `u32`, **capped at 8192**), and **`start_playback`** (bool, output only): when `true` after **`playback_load`**, output is driven by **JUCE** transport on the selected device (not the test-tone path).

### Library playback (JUCE + DSP)

**`playback_load`** opens the path with **`AudioFormatManager`** (`registerBasicFormats`). Duration and source sample rate come from the reader. **`start_output_stream`** with **`start_playback: true`** wires **`AudioTransportSource`** → **`AudioSourcePlayer`** → **`AudioDeviceManager`** output callback. DSP (EQ / gain / pan) runs in **`DspStereoFileSource`** before the transport.

| Command | Fields | Purpose |
|--------|--------|---------|
| `playback_load` | `path` (absolute) | Open file; store session; does **not** open output. |
| `start_output_stream` | `start_playback: true`, `device_id`, optional `buffer_frames` | After **`playback_load`**, start transport on the device. |
| `playback_pause` | `paused` (bool) | Pause / resume transport. |
| `playback_seek` | `position_sec` | Seek (seconds on the forward timeline). |
| `playback_set_dsp` | `gain`, `pan`, `eq_low_db`, `eq_mid_db`, `eq_high_db` | Update DSP parameters. |
| `playback_set_speed` | `speed` (float) | Accepted; **rate change is not wired** — response may include a **note** (no resampler yet). |
| `playback_set_reverse` | `reverse` (bool) | When `true`, full-decode-to-RAM reverse path for the next playback. |
| `playback_status` | — | Position, duration, peak, pause, EOF, reverse, sample rates. |
| `playback_stop` | — | Stop transport and clear session. |

`stop_output_stream` tears down the output device graph and clears playback as needed.

| Command | Purpose |
|--------|---------|
| `ping` | Version + host id |
| `engine_state` | Aggregated stream snapshot |
| `list_output_devices` / `list_input_devices` | Enumerate devices |
| `get_output_device_info` / `get_input_device_info` | Default config + `buffer_size` |
| `set_output_device` / `set_input_device` | Validate `device_id` |
| `start_output_stream` / `stop_output_stream` | Output; optional tone or file playback |
| `start_input_stream` / `stop_input_stream` | Input; peak meter |
| `output_stream_status` / `input_stream_status` | Status lines |
| `set_output_tone` | 440 Hz sine when F32 output + not in file playback mode |
| `plugin_chain` | Scan progress + plugin list metadata; **live insert processing** is not wired yet |

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

### Stale sidecar / `unknown cmd`

The app keeps **one** long-lived `audio-engine` child. After rebuilding, an old process can still answer stdin until replaced. The Tauri host respawns when the resolved binary’s **size/mtime** changes. If IPC looks wrong, quit the app or set **`AUDIO_HAXOR_AUDIO_ENGINE`** to an absolute path to a fresh `target/debug/audio-engine` or `target/release/audio-engine`.

## Host app (WEB UI)

`frontend/js/audio-engine.js` drives the Audio Engine tab and coordinates **`playback_*`** with the floating player. Behavior matches the root **`README.md`** Audio Engine / dev-vs-build sections (IPC guards, `engine_state` resync, input peak polling, etc.).
