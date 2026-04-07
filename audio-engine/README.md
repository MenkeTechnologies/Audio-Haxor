# audio-engine

Sidecar binary for **AUDIO_HAXOR**: cpal-based **input and output** device discovery, a **persistent** stdin line protocol (one JSON request in → one JSON line out), and a **single** thread that owns active `cpal::Stream` handles (`Stream` is not `Send` on macOS).

## Protocol

Each line is a JSON object with at least `cmd`. Optional fields include `device_id`, `tone` (bool, output only), `buffer_frames` (positive `u32`, fixed hardware buffer size in frames, clamped to the device’s supported range — applies to **both** input and output starts), and **`start_playback`** (bool, output only): when `true` after **`playback_load`**, the F32 output callback pulls decoded PCM from a ring buffer (library playback path) instead of tone/silence.

### Library playback (decode + ring + DSP)

Symphonia decodes the file on a **decoder thread**; resampling is linear from the file’s sample rate to the device rate. **`playback_load`** decodes the first audio packet and uses **`decoded.spec().rate`** for session **`src_rate`** when it differs from **`codec_params.sample_rate`** (some MP3/LAME probes disagree with the bitstream). The decoder thread still updates **`src_rate`** if a later packet reports a different rate. When **`start_playback`** opens the output stream, the sidecar enumerates **`supported_output_configs()`** and prefers an **F32** config whose rate range includes the session **`src_rate`** so the cpal stream often matches the file (e.g. 44.1 kHz file on a 48 kHz default device) and avoids an unnecessary large resample ratio. If several F32 ranges include that rate (e.g. stereo vs multichannel), **2** channels are preferred so the stream matches typical headphones / stereo output. The decoder thread seeds **`src_rate`** from the same **`playback_load`** probe before the first packet so the initial resample ratio matches the device choice. The real-time callback applies **3-band EQ** (lowshelf / peaking / highshelf, same corner frequencies as the Web Audio now‑playing graph), **gain**, and **constant-power stereo pan**, then writes interleaved samples to the cpal buffer.

| Command | Fields | Purpose |
|--------|--------|---------|
| `playback_load` | `path` (absolute file path) | Probe track; store session + duration; does **not** open cpal. |
| `start_output_stream` | `start_playback: true`, `device_id`, optional `buffer_frames` | After **`playback_load`**, starts output stream and the decoder thread; device stream rate is chosen to match **`src_rate`** when the device reports an F32 range that includes it, otherwise closest F32 rate in range, else **`default_output_config`**. |
| `playback_pause` | `paused` (bool) | Pause / resume decode (ring may underrun to silence while paused). |
| `playback_seek` | `position_sec` | Seek (symphonia `SeekMode::Accurate`). |
| `playback_set_dsp` | `gain`, `pan`, `eq_low_db`, `eq_mid_db`, `eq_high_db` | Update DSP atomics read in the callback. |
| `playback_status` | — | `position_sec`, `duration_sec`, `peak`, `paused`, `eof`, `sample_rate_hz` (device), `src_rate_hz` (file). |
| `playback_stop` | — | Stop decoder thread and clear session (host should **`stop_output_stream`** first). |

`stop_output_stream` signals the decoder to stop and joins the thread before dropping the stream.

Notable commands (devices + I/O):

| Command | Purpose |
|--------|---------|
| `ping` | Version + host id |
| `engine_state` | `version`, `host`, `stream` (output, same shape as `output_stream_status`), `input_stream` (same shape as `input_stream_status`) |
| `list_output_devices` / `list_input_devices` | Enumerate devices with stable string ids |
| `get_output_device_info` / `get_input_device_info` | Default config + `buffer_size` object (`kind`: `range` \| `unknown`). Omit `device_id` to query the host default input/output device. |
| `set_output_device` / `set_input_device` | Validate `device_id` only (no stream opened) |
| `start_output_stream` | Open **output** config (with **`start_playback`**, prefer F32 at loaded **`src_rate`** when supported); optional `buffer_frames`, **`start_playback`**; **F32** supports `tone` (440 Hz sine) **or** file PCM when `start_playback` is set. |
| `stop_output_stream` | Drop output stream |
| `output_stream_status` | Running + `tone_supported` / `tone_on` + `stream_buffer_frames` (null when idle or driver default) |
| `start_input_stream` | Open default **input** config; optional `buffer_frames`; callback discards samples and updates **`input_peak`** (0..1 linear, block peak + decay). |
| `stop_input_stream` | Drop input stream |
| `input_stream_status` | Running + `stream_buffer_frames` + **`input_peak`** (null when idle); no tone fields. Host UI may poll this while the Audio Engine tab is active and input capture is running (~100ms) so **`input_peak`** updates live; polling pauses when the tab or window is not visible. |
| `set_output_tone` | Toggle tone while output stream is running (F32 only; ignored when file playback owns the callback) |

## Build

From the repo root (workspace):

```bash
cargo build -p audio-engine --release
```

Tauri bundles this via `scripts/prepare-audio-engine-sidecar.mjs` and `bundle.externalBin`.

## Host app (WEB UI)

`frontend/js/audio-engine.js` drives the Audio Engine tab and exports **`enginePlaybackStart`**, **`enginePlaybackStop`**, **`syncEnginePlaybackDspFromPrefs`**, **`stopEnginePlaybackPoll`** on `window` for **`frontend/js/audio.js`**: when IPC exists, **library preview** uses **`playback_load`** + **`start_output_stream`** with **`start_playback: true`** so audible output comes **only** from the sidecar (the `<audio>` element stays disconnected). **`getAeAudioEngineInvoke()`** returns the preload `audioEngineInvoke` or `null` (guards missing `window.vstUpdater` / release load order). **`fillAeEngineStatusOkFromState`** / **`fillAeEngineStatusFromError`** / **`syncAeToneCheckboxFromStream`** centralize **`ui.ae.status_ok`**, **`ui.ae.status_error`**, and test-tone checkbox updates from **`engine_state`**. **`throwIfAeNotOk`** normalizes IPC `{ ok, error }` failures into **`Error`**s for the shared catch paths. Running stream lines share **`buildAeStreamStatusLineCore`** (output vs input catalog keys) and **`appendAeStreamBufferFixedSuffix`** for the fixed **`stream_buffer_frames`** segment (`ui.ae.stream_buffer_fixed`). Refresh rebuilds the input device **`<select>`** via **`aePopulateInputDeviceSelectOptions`** (system-default option + devices + pick validation). After Apply / tone / capture / stop failures, **`fillAeStreamsAfterEngineError`** (uses **`getAeAudioEngineInvoke`**) re-invokes `engine_state` so the output/input stream lines match the sidecar. If **`audioEngineInvoke`** is unavailable, **`aeNotifyNoAudioEngineIpc`** clears stream lines to `—` and sets **`ui.ae.err_no_ipc`** on the engine status line (same pattern as Refresh with no IPC). **Web Audio** analysers / spectrum in the now‑playing UI do not see engine‑routed audio (the graph is not in the signal path); position/time use **`playback_status`** polling (~100 ms).
