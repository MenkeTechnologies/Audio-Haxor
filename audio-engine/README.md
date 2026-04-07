# audio-engine

Sidecar binary for **AUDIO_HAXOR**: cpal-based **input and output** device discovery, a **persistent** stdin line protocol (one JSON request in → one JSON line out), and a **single** thread that owns active `cpal::Stream` handles (`Stream` is not `Send` on macOS).

## Protocol

Each line is a JSON object with at least `cmd`. Optional fields: `device_id`, `tone` (bool, output only), `buffer_frames` (positive `u32`, fixed hardware buffer size in frames, clamped to the device’s supported range — applies to **both** input and output starts).

Notable commands:

| Command | Purpose |
|--------|---------|
| `ping` | Version + host id |
| `engine_state` | `version`, `host`, `stream` (output, same shape as `output_stream_status`), `input_stream` (same shape as `input_stream_status`) |
| `list_output_devices` / `list_input_devices` | Enumerate devices with stable string ids |
| `get_output_device_info` / `get_input_device_info` | Default config + `buffer_size` object (`kind`: `range` \| `unknown`). Omit `device_id` to query the host default input/output device. |
| `set_output_device` / `set_input_device` | Validate `device_id` only (no stream opened) |
| `start_output_stream` | Open default **output** config; optional `buffer_frames`; **F32** supports `tone` (440 Hz sine at low gain). |
| `stop_output_stream` | Drop output stream |
| `output_stream_status` | Running + `tone_supported` / `tone_on` + `stream_buffer_frames` (null when idle or driver default) |
| `start_input_stream` | Open default **input** config; optional `buffer_frames`; callback discards samples until metering/DSP exists. |
| `stop_input_stream` | Drop input stream |
| `input_stream_status` | Running + `stream_buffer_frames` (null when idle); no tone fields |
| `set_output_tone` | Toggle tone while output stream is running (F32 only) |

## Build

From the repo root (workspace):

```bash
cargo build -p audio-engine --release
```

Tauri bundles this via `scripts/prepare-audio-engine-sidecar.mjs` and `bundle.externalBin`.
