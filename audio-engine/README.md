# audio-engine

Sidecar binary for **AUDIO_HAXOR**: cpal-based **output** device discovery, a **persistent** stdin line protocol (one JSON request in → one JSON line out), and a dedicated thread that owns the active `cpal::Stream` (`Stream` is not `Send` on macOS).

## Protocol

Each line is a JSON object with at least `cmd`. Optional fields: `device_id`, `tone` (bool), `buffer_frames` (positive `u32`, fixed hardware buffer size in frames, clamped to the device’s supported range).

Notable commands:

| Command | Purpose |
|--------|---------|
| `ping` | Version + host id |
| `engine_state` | `version`, `host`, and `stream` (same shape as `output_stream_status`) |
| `list_output_devices` / `list_input_devices` | Enumerate devices with stable string ids |
| `get_output_device_info` / `get_input_device_info` | Default config + `buffer_size` object (`kind`: `range` \| `unknown`) |
| `set_output_device` | Validate `device_id` only |
| `start_output_stream` | Open default config; optional `buffer_frames` → `BufferSize::Fixed` (clamped); **F32** supports `tone` (440 Hz sine at low gain). Response + `output_stream_status` include `stream_buffer_frames` when fixed buffering was applied. |
| `stop_output_stream` | Drop stream |
| `output_stream_status` | Running + `tone_supported` / `tone_on` + `stream_buffer_frames` (null when idle or driver default) |
| `set_output_tone` | Toggle tone while stream is running (F32 only) |

## Build

From the repo root (workspace):

```bash
cargo build -p audio-engine --release
```

Tauri bundles this via `scripts/prepare-audio-engine-sidecar.mjs` and `bundle.externalBin`.
