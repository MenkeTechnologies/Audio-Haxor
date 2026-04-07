# audio-engine

Sidecar binary for **AUDIO_HAXOR**: **cpal**-based **input and output** device discovery, a **persistent** stdin line protocol (one JSON request in → one JSON line out), and a **single** thread that owns active output handles. **Library file playback** uses **[rodio](https://github.com/RustAudio/rodio)** (`DeviceSinkBuilder` → `MixerDeviceSink` / `Player` → `Decoder`); test tone and silence still use a **`cpal::Stream`** when `start_playback` is false (`Stream` is not `Send` on macOS, so streams stay on this thread).

## Protocol

Each line is a JSON object with at least `cmd`. Optional fields include `device_id`, `tone` (bool, output only), `buffer_frames` (positive `u32`, fixed hardware buffer size in frames — **capped at 8192** before the device’s range clamp; prevents typos like `144000` which are ~**3** seconds at 48 kHz per callback and sound like audio continuing after **stop**), and **`start_playback`** (bool, output only): when `true` after **`playback_load`**, output is driven by **rodio** on the selected device (not the F32 cpal callback).

### Library playback (rodio + DSP)

**`playback_load`** uses **symphonia** only to probe the file (duration, **`src_rate`** from the first decoded packet when it differs from **`codec_params.sample_rate`** — some MP3/LAME probes disagree with the bitstream). **`start_output_stream`** with **`start_playback: true`** opens **`rodio::stream::DeviceSinkBuilder`** on the resolved device, applies the same **F32** **`supported_output_configs()`** preference as before (rate range including **`src_rate`**, stereo preferred when multiple ranges match), optional **`buffer_frames`**, then **`Player::connect_new`**, **`Decoder::try_from`**, and a custom **`Source`** that applies **3-band EQ** (lowshelf / peaking / highshelf, same corner frequencies as the Web Audio now‑playing graph), **gain**, and **constant-power stereo pan** on interleaved samples. Resampling and mixing are handled inside rodio/cpal.

| Command | Fields | Purpose |
|--------|--------|---------|
| `playback_load` | `path` (absolute file path) | Probe track; store session + duration; does **not** open output. Replacing a session clears the previous **`Player`** via **`stop_playback_thread`**. |
| `start_output_stream` | `start_playback: true`, `device_id`, optional `buffer_frames` | After **`playback_load`**, opens rodio on the device and appends the decoded source. |
| `playback_pause` | `paused` (bool) | **`Player::pause`** / **`Player::play`**. |
| `playback_seek` | `position_sec` | **`Player::try_seek`**. **`position_sec` is always the normal (forward) timeline** (0 = start of track). In **reverse** mode the sidecar maps to the reversed buffer position (`duration - position`). |
| `playback_set_dsp` | `gain`, `pan`, `eq_low_db`, `eq_mid_db`, `eq_high_db` | Update DSP atomics read in the rodio **`Source`** iterator. |
| `playback_set_speed` | `speed` (float, clamped **0.25–2**) | **`Player::set_speed`** — same pitch behavior as `<audio>.playbackRate`. |
| `playback_set_reverse` | `reverse` (bool) | When `true`, **fully decodes** the loaded file to stereo **f32** in RAM, reverses frame order, and uses **`SamplesBuffer`** on the next **`start_playback`** instead of streaming from disk. When `false`, clears the buffer. Large files use significant memory. |
| `playback_status` | — | **`Player::get_pos`**, `duration_sec`, `peak`, **`Player::is_paused`**, **`Player::empty`** (`eof`), `reverse` (bool), `sample_rate_hz` (device), `src_rate_hz` (file probe). **`position_sec`** is mapped to the forward timeline when `reverse` is true. |
| `playback_stop` | — | Stop **`Player`** and clear session (host should **`stop_output_stream`** first). |

`stop_output_stream` drops the cpal stream **or** rodio sink handle and calls **`stop_playback_thread`** so the **`Player`** is cleared before a new output starts.

Notable commands (devices + I/O):

| Command | Purpose |
|--------|--------|
| `ping` | Version + host id |
| `engine_state` | `version`, `host`, `stream` (output, same shape as `output_stream_status`), `input_stream` (same shape as `input_stream_status`) |
| `list_output_devices` / `list_input_devices` | Enumerate devices with stable string ids |
| `get_output_device_info` / `get_input_device_info` | Default config + `buffer_size` object (`kind`: `range` \| `unknown`). Omit `device_id` to query the host default input/output device. |
| `set_output_device` / `set_input_device` | Validate `device_id` only (no stream opened) |
| `start_output_stream` | Open **output** config (with **`start_playback`**, prefer F32 at loaded **`src_rate`** when supported); optional `buffer_frames`, **`start_playback`**; **F32** supports `tone` (440 Hz sine) **or** rodio file playback when `start_playback` is set. |
| `stop_output_stream` | Drop output stream / rodio sink. The host Audio Engine tab also sends **`playback_stop`** so the library session is cleared and the floating player state stays in sync. |
| `output_stream_status` | Running + `tone_supported` / `tone_on` + `stream_buffer_frames` (null when idle or driver default) |
| `start_input_stream` | Open default **input** config; optional `buffer_frames`; callback discards samples and updates **`input_peak`** (0..1 linear, block peak + decay). |
| `stop_input_stream` | Drop input stream |
| `input_stream_status` | Running + `stream_buffer_frames` + **`input_peak`** (null when idle); no tone fields. Host UI may poll this while the Audio Engine tab is active and input capture is running (~100ms) so **`input_peak`** updates live; polling pauses when the tab or window is not visible. |
| `set_output_tone` | Toggle tone while output stream is running (F32 only; not available when **`start_playback`** file playback is active — no cpal tone path). |

## Build

From the repo root (workspace):

```bash
cargo build -p audio-engine --release
```

Tauri bundles this via `scripts/prepare-audio-engine-sidecar.mjs` and `bundle.externalBin`.

### Stale sidecar / `unknown cmd: playback_load`

The app keeps **one** long-lived `audio-engine` child. After you **`cargo build -p audio-engine`**, the on-disk binary updates, but an old process could still answer stdin until replaced. The Tauri host respawns when the resolved binary’s **size/mtime** changes, and retries once if a response’s **`error`** contains `unknown cmd`. If you still see **`unknown cmd`** for a valid verb, **fully quit and reopen the app**, or set **`AUDIO_HAXOR_AUDIO_ENGINE`** to an absolute path to a fresh `target/debug/audio-engine` (or `release`) so the host spawns that binary.

## Host app (WEB UI)

`frontend/js/audio-engine.js` drives the Audio Engine tab and exports **`enginePlaybackStart`**, **`enginePlaybackStop`**, **`syncEnginePlaybackDspFromPrefs`**, **`stopEnginePlaybackPoll`** on `window` for **`frontend/js/audio.js`**: when IPC exists, **library preview** uses **`playback_load`** + **`start_output_stream`** with **`start_playback: true`** so audible output comes **only** from the sidecar (the `<audio>` element stays disconnected). **Apply** (`applyAudioEngineDevice`) queries **`playback_status`** first; if **`loaded`**, it passes **`start_playback: true`** when restarting **`start_output_stream`** so changing device, buffer size, or test tone does **not** replace the stream with silence/tone-only while a session still exists (which previously broke preview until **Stop stream** or a new **`playback_load`**). On **`start_output_stream`** with **`start_playback`**, the sidecar calls **`stop_playback_thread`** after replacing output so the previous **`Player`** is stopped before attaching a new one — matching the **`stop_output_stream`** step in **`enginePlaybackStart`** and avoiding a stuck decoder or duplicate playback on Apply. **`getAeAudioEngineInvoke()`** returns the preload `audioEngineInvoke` or `null` (guards missing `window.vstUpdater` / release load order). **`fillAeEngineStatusOkFromState`** / **`fillAeEngineStatusFromError`** / **`syncAeToneCheckboxFromStream`** centralize **`ui.ae.status_ok`**, **`ui.ae.status_error`**, and test-tone checkbox updates from **`engine_state`**. **`throwIfAeNotOk`** normalizes IPC `{ ok, error }` failures into **`Error`**s for the shared catch paths. Running stream lines share **`buildAeStreamStatusLineCore`** (output vs input catalog keys) and **`appendAeStreamBufferFixedSuffix`** for the fixed **`stream_buffer_frames`** segment (`ui.ae.stream_buffer_fixed`). Refresh rebuilds the input device **`<select>`** via **`aePopulateInputDeviceSelectOptions`** (system-default option + devices + pick validation). After Apply / tone / capture / stop failures, **`fillAeStreamsAfterEngineError`** (uses **`getAeAudioEngineInvoke`**) re-invokes `engine_state` so the output/input stream lines match the sidecar. If **`audioEngineInvoke`** is unavailable, **`aeNotifyNoAudioEngineIpc`** clears stream lines to `—` and sets **`ui.ae.err_no_ipc`** on the engine status line (same pattern as Refresh with no IPC). **Web Audio** analysers / spectrum in the now‑playing UI do not see engine‑routed audio (the graph is not in the signal path); position/time use **`playback_status`** polling (~100 ms).
