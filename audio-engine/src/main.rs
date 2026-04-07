//! Separate **audio-engine** process: input/output device discovery (via cpal) and streams on a
//! dedicated thread. Reads JSON lines on stdin until EOF; prints one JSON line per request. The
//! host keeps one child process and reuses stdin/stdout. **`cpal::Stream` is not `Send` on macOS**,
//! so output and input streams live on the same audio thread.

mod playback;

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, OnceLock};
use std::thread;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{
    BufferSize, Device, SampleFormat, SampleRate, Stream, StreamConfig, SupportedBufferSize,
    SupportedStreamConfig,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

const ENGINE_VERSION: &str = env!("CARGO_PKG_VERSION");
const TEST_TONE_HZ: f32 = 440.0;
const TEST_TONE_GAIN: f32 = 0.05;
/// Exponential decay between input callbacks (0–1 linear peak).
const INPUT_PEAK_DECAY: f32 = 0.95;

#[derive(Debug, Deserialize)]
struct Request {
    cmd: String,
    #[serde(default)]
    device_id: Option<String>,
    /// When starting the output stream, enable 440 Hz test tone (F32 only).
    #[serde(default)]
    tone: bool,
    /// Optional hardware buffer size in frames (clamped to device `buffer_size` range).
    #[serde(default)]
    buffer_frames: Option<u32>,
    /// After `playback_load`, start decoder + pull PCM from ring (requires F32 output path).
    #[serde(default)]
    start_playback: bool,
    #[serde(default)]
    path: Option<String>,
    #[serde(default)]
    position_sec: Option<f64>,
    #[serde(default)]
    gain: Option<f32>,
    #[serde(default)]
    pan: Option<f32>,
    #[serde(default)]
    eq_low_db: Option<f32>,
    #[serde(default)]
    eq_mid_db: Option<f32>,
    #[serde(default)]
    eq_high_db: Option<f32>,
    /// `playback_pause` { "paused": true|false }
    #[serde(default)]
    paused: Option<bool>,
}

#[derive(Debug, Serialize)]
struct OutputDeviceInfo {
    id: String,
    name: String,
    is_default: bool,
}

struct ActiveStream {
    /// Held so [`Stream`] stays open; drop stops playback.
    #[allow(dead_code)]
    stream: Stream,
    device_id: String,
    device_name: String,
    sample_rate_hz: u32,
    channels: u16,
    sample_format: String,
    buffer_size: serde_json::Value,
    /// `Some` when `BufferSize::Fixed` was requested/applied.
    stream_buffer_frames: Option<u32>,
    /// F32 streams only: toggles silence vs test tone in the callback.
    tone_flag: Option<Arc<AtomicBool>>,
}

struct ActiveInputStream {
    #[allow(dead_code)]
    stream: Stream,
    device_id: String,
    device_name: String,
    sample_rate_hz: u32,
    channels: u16,
    sample_format: String,
    buffer_size: serde_json::Value,
    stream_buffer_frames: Option<u32>,
    /// `f32` peak 0..1 stored as [`f32::to_bits`].
    peak_bits: Arc<AtomicU32>,
}

enum AudioCmd {
    Start {
        device_id: Option<String>,
        tone: bool,
        buffer_frames: Option<u32>,
        start_playback: bool,
        reply: mpsc::Sender<Result<serde_json::Value, String>>,
    },
    StartInput {
        device_id: Option<String>,
        buffer_frames: Option<u32>,
        reply: mpsc::Sender<Result<serde_json::Value, String>>,
    },
    Stop {
        reply: mpsc::Sender<Result<bool, String>>,
    },
    StopInput {
        reply: mpsc::Sender<Result<bool, String>>,
    },
    Status {
        reply: mpsc::Sender<Result<serde_json::Value, String>>,
    },
    InputStatus {
        reply: mpsc::Sender<Result<serde_json::Value, String>>,
    },
    SetTone {
        enabled: bool,
        reply: mpsc::Sender<Result<(), String>>,
    },
}

/// Prefer opening the output device at `prefer_hz` (from `playback_load`) when the device reports
/// an F32 config that includes that rate — avoids large `src_rate`/`device_rate` ratios when the
/// default device config is e.g. 48 kHz but the file is 44.1 kHz.
///
/// When several F32 ranges include the same rate (e.g. stereo vs multichannel), prefer **2
/// channels** so the stream matches typical headphones / stereo output instead of a surround layout.
fn output_config_preference_key(cfg: &SupportedStreamConfig) -> (u8, u16) {
    let ch = cfg.channels();
    if ch == 2 {
        (0, ch)
    } else {
        (1, ch)
    }
}

fn pick_output_config_for_playback(device: &Device, prefer_hz: u32) -> Result<SupportedStreamConfig, String> {
    let ranges: Vec<_> = device
        .supported_output_configs()
        .map_err(|e| format!("supported_output_configs: {e}"))?
        .collect();
    let mut best_exact: Option<SupportedStreamConfig> = None;
    for r in &ranges {
        if r.sample_format() != SampleFormat::F32 {
            continue;
        }
        if let Some(cfg) = r.try_with_sample_rate(SampleRate(prefer_hz)) {
            let replace = match &best_exact {
                None => true,
                Some(b) => output_config_preference_key(&cfg) < output_config_preference_key(b),
            };
            if replace {
                best_exact = Some(cfg);
            }
        }
    }
    if let Some(cfg) = best_exact {
        return Ok(cfg);
    }
    let mut best: Option<(SupportedStreamConfig, u32)> = None;
    for r in ranges {
        if r.sample_format() != SampleFormat::F32 {
            continue;
        }
        let mn = r.min_sample_rate().0;
        let mx = r.max_sample_rate().0;
        let target = prefer_hz.clamp(mn, mx);
        let cfg = r.with_sample_rate(SampleRate(target));
        let diff = target.abs_diff(prefer_hz);
        let replace = match &best {
            None => true,
            Some((bcfg, bd)) => {
                if diff < *bd {
                    true
                } else if diff == *bd {
                    output_config_preference_key(&cfg) < output_config_preference_key(bcfg)
                } else {
                    false
                }
            }
        };
        if replace {
            best = Some((cfg, diff));
        }
    }
    if let Some((cfg, _)) = best {
        return Ok(cfg);
    }
    device.default_output_config().map_err(|e| e.to_string())
}

fn audio_thread_tx() -> &'static Sender<AudioCmd> {
    static TX: OnceLock<Sender<AudioCmd>> = OnceLock::new();
    TX.get_or_init(|| {
        let (tx, rx) = mpsc::channel::<AudioCmd>();
        thread::spawn(move || audio_thread_main(rx));
        tx
    })
}

fn audio_thread_main(rx: mpsc::Receiver<AudioCmd>) {
    let mut current: Option<ActiveStream> = None;
    let mut current_input: Option<ActiveInputStream> = None;
    while let Ok(cmd) = rx.recv() {
        match cmd {
            AudioCmd::Start {
                device_id,
                tone,
                buffer_frames,
                start_playback,
                reply,
            } => {
                let res = (|| {
                    current.take();
                    let device = resolve_device(device_id.as_deref())?;
                    let device_name = device.name().unwrap_or_default();
                    let resolved_id = match device_id.as_deref().filter(|s| !s.is_empty()) {
                        Some(id) => id.to_string(),
                        None => {
                            let rows = enumerate_output_devices()?;
                            rows.into_iter()
                                .find(|d| d.name == device_name)
                                .map(|d| d.id)
                                .unwrap_or(device_name.clone())
                        }
                    };
                    let file_pb = if start_playback {
                        playback::current_playback_shared()
                    } else {
                        None
                    };
                    let want_hz = file_pb
                        .as_ref()
                        .map(|p| p.src_rate.load(Ordering::Relaxed))
                        .filter(|&r| r > 0);
                    let supported = match want_hz {
                        Some(want) => match pick_output_config_for_playback(&device, want) {
                            Ok(cfg) => cfg,
                            Err(e) => {
                                eprintln!("audio-engine: pick_output_config_for_playback ({want} Hz): {e}; using default_output_config");
                                device
                                    .default_output_config()
                                    .map_err(|e2| format!("default_output_config: {e2}"))?
                            }
                        },
                        None => device
                            .default_output_config()
                            .map_err(|e| format!("default_output_config: {e}"))?,
                    };
                    let channels = supported.channels();
                    let sample_format = format!("{:?}", supported.sample_format());
                    let buffer_size = buffer_size_json(supported.buffer_size());
                    let defer_play = start_playback && file_pb.is_some();
                    let (stream, tone_flag, stream_buffer_frames, stream_sr, stream_started) =
                        build_playback_stream(
                            &device,
                            supported,
                            tone,
                            buffer_frames,
                            file_pb.clone(),
                            defer_play,
                        )?;
                    if start_playback {
                        if file_pb.is_none() {
                            return Err("playback_load required before start_playback".to_string());
                        }
                        playback::begin_playback(stream_sr)
                            .map_err(|e| format!("playback: {e}"))?;
                    }
                    if !stream_started {
                        stream
                            .play()
                            .map_err(|e| format!("stream.play: {e}"))?;
                    }
                    let tone_supported = tone_flag.is_some();
                    let tone_on = tone_flag
                        .as_ref()
                        .map(|f| f.load(Ordering::Relaxed))
                        .unwrap_or(false);
                    current = Some(ActiveStream {
                        stream,
                        device_id: resolved_id.clone(),
                        device_name: device_name.clone(),
                        sample_rate_hz: stream_sr,
                        channels,
                        sample_format: sample_format.clone(),
                        buffer_size: buffer_size.clone(),
                        stream_buffer_frames,
                        tone_flag,
                    });
                    Ok(json!({
                        "ok": true,
                        "device_id": resolved_id,
                        "device_name": device_name,
                        "sample_rate_hz": stream_sr,
                        "channels": channels,
                        "sample_format": sample_format,
                        "buffer_size": buffer_size,
                        "stream_buffer_frames": stream_buffer_frames,
                        "tone_supported": tone_supported,
                        "tone_on": tone_on,
                        "note": if start_playback { "file playback PCM" } else { "silence or test tone" },
                    }))
                })();
                let _ = reply.send(res);
            }
            AudioCmd::StartInput {
                device_id,
                buffer_frames,
                reply,
            } => {
                let res = (|| {
                    current_input.take();
                    let device = resolve_input_device(device_id.as_deref())?;
                    let device_name = device.name().unwrap_or_default();
                    let resolved_id = match device_id.as_deref().filter(|s| !s.is_empty()) {
                        Some(id) => id.to_string(),
                        None => {
                            let rows = enumerate_input_devices()?;
                            rows.into_iter()
                                .find(|d| d.name == device_name)
                                .map(|d| d.id)
                                .unwrap_or(device_name.clone())
                        }
                    };
                    let supported = device
                        .default_input_config()
                        .map_err(|e| format!("default_input_config: {e}"))?;
                    let sample_rate_hz = supported.sample_rate().0;
                    let channels = supported.channels();
                    let sample_format = format!("{:?}", supported.sample_format());
                    let buffer_size = buffer_size_json(supported.buffer_size());
                    let peak_bits = Arc::new(AtomicU32::new(0.0f32.to_bits()));
                    let (stream, stream_buffer_frames) = build_capture_stream(
                        &device,
                        supported,
                        buffer_frames,
                        peak_bits.clone(),
                    )?;
                    current_input = Some(ActiveInputStream {
                        stream,
                        device_id: resolved_id.clone(),
                        device_name: device_name.clone(),
                        sample_rate_hz,
                        channels,
                        sample_format: sample_format.clone(),
                        buffer_size: buffer_size.clone(),
                        stream_buffer_frames,
                        peak_bits,
                    });
                    Ok(json!({
                        "ok": true,
                        "device_id": resolved_id,
                        "device_name": device_name,
                        "sample_rate_hz": sample_rate_hz,
                        "channels": channels,
                        "sample_format": sample_format,
                        "buffer_size": buffer_size,
                        "stream_buffer_frames": stream_buffer_frames,
                        "input_peak": 0.0,
                        "note": "input capture running; samples discarded; input_peak is block peak with decay",
                    }))
                })();
                let _ = reply.send(res);
            }
            AudioCmd::Stop { reply } => {
                playback::stop_playback_thread();
                let had = current.take().is_some();
                let _ = reply.send(Ok(had));
            }
            AudioCmd::StopInput { reply } => {
                let had = current_input.take().is_some();
                let _ = reply.send(Ok(had));
            }
            AudioCmd::Status { reply } => {
                let v = match &current {
                    Some(a) => {
                        let tone_on = a
                            .tone_flag
                            .as_ref()
                            .map(|f| f.load(Ordering::Relaxed))
                            .unwrap_or(false);
                        json!({
                            "ok": true,
                            "running": true,
                            "device_id": a.device_id,
                            "device_name": a.device_name,
                            "sample_rate_hz": a.sample_rate_hz,
                            "channels": a.channels,
                            "sample_format": a.sample_format,
                            "buffer_size": a.buffer_size,
                            "stream_buffer_frames": a.stream_buffer_frames,
                            "tone_supported": a.tone_flag.is_some(),
                            "tone_on": tone_on,
                        })
                    }
                    None => json!({
                        "ok": true,
                        "running": false,
                        "device_id": serde_json::Value::Null,
                        "device_name": serde_json::Value::Null,
                        "sample_rate_hz": serde_json::Value::Null,
                        "channels": serde_json::Value::Null,
                        "sample_format": serde_json::Value::Null,
                        "buffer_size": serde_json::Value::Null,
                        "stream_buffer_frames": serde_json::Value::Null,
                        "tone_supported": serde_json::Value::Null,
                        "tone_on": serde_json::Value::Null,
                    }),
                };
                let _ = reply.send(Ok(v));
            }
            AudioCmd::InputStatus { reply } => {
                let v = match &current_input {
                    Some(a) => {
                        let pk = f32::from_bits(a.peak_bits.load(Ordering::Relaxed));
                        json!({
                            "ok": true,
                            "running": true,
                            "device_id": a.device_id,
                            "device_name": a.device_name,
                            "sample_rate_hz": a.sample_rate_hz,
                            "channels": a.channels,
                            "sample_format": a.sample_format,
                            "buffer_size": a.buffer_size,
                            "stream_buffer_frames": a.stream_buffer_frames,
                            "input_peak": pk,
                        })
                    }
                    None => json!({
                        "ok": true,
                        "running": false,
                        "device_id": serde_json::Value::Null,
                        "device_name": serde_json::Value::Null,
                        "sample_rate_hz": serde_json::Value::Null,
                        "channels": serde_json::Value::Null,
                        "sample_format": serde_json::Value::Null,
                        "buffer_size": serde_json::Value::Null,
                        "stream_buffer_frames": serde_json::Value::Null,
                        "input_peak": serde_json::Value::Null,
                    }),
                };
                let _ = reply.send(Ok(v));
            }
            AudioCmd::SetTone { enabled, reply } => {
                let r = match &current {
                    Some(a) => {
                        if let Some(flag) = &a.tone_flag {
                            flag.store(enabled, Ordering::Relaxed);
                            Ok(())
                        } else {
                            Err("test tone requires F32 output (device default format)".to_string())
                        }
                    }
                    None => Err("no output stream".to_string()),
                };
                let _ = reply.send(r);
            }
        }
    }
}

fn buffer_size_json(bs: &SupportedBufferSize) -> serde_json::Value {
    match bs {
        SupportedBufferSize::Range { min, max } => json!({
            "kind": "range",
            "min": min,
            "max": max,
        }),
        SupportedBufferSize::Unknown => json!({ "kind": "unknown" }),
    }
}

/// Upper bound for user-requested hardware buffer size (frames per callback).
/// Values like `144000` are easy typos: at 48 kHz that is **3 seconds** per buffer — stop/restart then
/// sounds like audio shutting off seconds after halt, and underrun recovery is hopeless.
const MAX_BUFFER_FRAMES: u32 = 8192;

/// Sets `cfg.buffer_size` to [`BufferSize::Fixed`] when `requested` is present; returns the frame count applied.
fn apply_buffer_frames_request(
    cfg: &mut StreamConfig,
    supported_bs: &SupportedBufferSize,
    requested: Option<u32>,
) -> Option<u32> {
    let Some(req) = requested.filter(|n| *n > 0) else {
        return None;
    };
    let req = req.min(MAX_BUFFER_FRAMES).max(1);
    match supported_bs {
        SupportedBufferSize::Range { min, max } => {
            let f = req.clamp(*min, *max);
            cfg.buffer_size = BufferSize::Fixed(f);
            Some(f)
        }
        SupportedBufferSize::Unknown => {
            cfg.buffer_size = BufferSize::Fixed(req);
            Some(req)
        }
    }
}

fn main() {
    let stdin = std::io::stdin();
    let mut reader = BufReader::new(stdin.lock());
    let stdout = std::io::stdout();
    let mut out = std::io::LineWriter::new(stdout.lock());
    let mut line = String::new();
    loop {
        line.clear();
        let n = reader.read_line(&mut line).unwrap_or(0);
        if n == 0 {
            break;
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let req: Request = match serde_json::from_str(trimmed) {
            Ok(r) => r,
            Err(e) => {
                let resp = json!({"ok": false, "error": format!("bad JSON: {e}")});
                writeln!(out, "{resp}").ok();
                out.flush().ok();
                continue;
            }
        };
        let resp = match dispatch(&req) {
            Ok(v) => v,
            Err(msg) => json!({ "ok": false, "error": msg }),
        };
        writeln!(out, "{resp}").ok();
        out.flush().ok();
    }
}

fn dispatch(req: &Request) -> Result<serde_json::Value, String> {
    let cmd = req.cmd.trim().to_ascii_lowercase();
    match cmd.as_str() {
        "ping" => Ok(json!({
            "ok": true,
            "version": ENGINE_VERSION,
            "host": cpal::default_host().id().name(),
        })),
        "engine_state" => engine_state(),
        "list_output_devices" => list_output_devices(),
        "list_input_devices" => list_input_devices(),
        "get_output_device_info" => get_output_device_info(req.device_id.as_deref()),
        "get_input_device_info" => get_input_device_info(req.device_id.as_deref()),
        "set_output_device" => set_output_device(req.device_id.as_deref()),
        "set_input_device" => set_input_device(req.device_id.as_deref()),
        "start_input_stream" => start_input_stream(req.device_id.as_deref(), req.buffer_frames),
        "start_output_stream" => start_output_stream(
            req.device_id.as_deref(),
            req.tone,
            req.buffer_frames,
            req.start_playback,
        ),
        "playback_load" => playback::playback_load(req.path.clone().ok_or_else(|| "path required".to_string())?),
        "playback_stop" => playback::playback_stop(),
        "playback_pause" => playback::playback_pause(req.paused.unwrap_or(true)),
        "playback_seek" => playback::playback_seek(req.position_sec.unwrap_or(0.0)),
        "playback_set_dsp" => playback::playback_set_dsp(
            req.gain.unwrap_or(1.0),
            req.pan.unwrap_or(0.0),
            req.eq_low_db.unwrap_or(0.0),
            req.eq_mid_db.unwrap_or(0.0),
            req.eq_high_db.unwrap_or(0.0),
        ),
        "playback_status" => playback::playback_status(),
        "stop_input_stream" => stop_input_stream(),
        "stop_output_stream" => stop_output_stream(),
        "input_stream_status" => input_stream_status(),
        "output_stream_status" => output_stream_status(),
        "set_output_tone" => set_output_tone(req.tone),
        "plugin_chain" => Ok(json!({
            "ok": true,
            "slots": [],
            "note": "plugin hosting will attach here",
        })),
        other => Err(format!("unknown cmd: {other}")),
    }
}

fn engine_state() -> Result<serde_json::Value, String> {
    let stream = output_stream_status()?;
    let input_stream = input_stream_status()?;
    Ok(json!({
        "ok": true,
        "version": ENGINE_VERSION,
        "host": cpal::default_host().id().name(),
        "stream": stream,
        "input_stream": input_stream,
    }))
}

fn unique_device_id(name: &str, seen: &mut HashMap<String, u32>) -> String {
    let n = seen.entry(name.to_string()).or_insert(0);
    *n += 1;
    if *n == 1 {
        name.to_string()
    } else {
        format!("{name}#{}", n)
    }
}

fn list_output_devices() -> Result<serde_json::Value, String> {
    let rows = enumerate_output_devices()?;
    let default_id = rows.iter().find(|d| d.is_default).map(|d| d.id.clone());

    Ok(json!({
        "ok": true,
        "default_device_id": default_id,
        "devices": rows,
    }))
}

fn enumerate_input_devices() -> Result<Vec<OutputDeviceInfo>, String> {
    let host = cpal::default_host();
    let default_dev = host.default_input_device();
    let default_name = default_dev.as_ref().and_then(|d| d.name().ok());

    let mut seen = HashMap::new();
    let mut out = Vec::new();

    for (i, dev) in host
        .input_devices()
        .map_err(|e| format!("cpal input_devices: {e}"))?
        .enumerate()
    {
        let name = dev.name().unwrap_or_else(|_| format!("in {i}"));
        let id = unique_device_id(&name, &mut seen);
        let is_default = default_name
            .as_ref()
            .map(|dn| dn == &name)
            .unwrap_or(false);
        out.push(OutputDeviceInfo {
            id,
            name,
            is_default,
        });
    }

    Ok(out)
}

fn list_input_devices() -> Result<serde_json::Value, String> {
    let rows = enumerate_input_devices()?;
    let default_id = rows.iter().find(|d| d.is_default).map(|d| d.id.clone());

    Ok(json!({
        "ok": true,
        "default_device_id": default_id,
        "devices": rows,
    }))
}

fn enumerate_output_devices() -> Result<Vec<OutputDeviceInfo>, String> {
    let host = cpal::default_host();
    let default_dev = host.default_output_device();
    let default_name = default_dev.as_ref().and_then(|d| d.name().ok());

    let mut seen = HashMap::new();
    let mut out = Vec::new();

    for (i, dev) in host
        .output_devices()
        .map_err(|e| format!("cpal output_devices: {e}"))?
        .enumerate()
    {
        let name = dev.name().unwrap_or_else(|_| format!("device {i}"));
        let id = unique_device_id(&name, &mut seen);
        let is_default = default_name
            .as_ref()
            .map(|dn| dn == &name)
            .unwrap_or(false);
        out.push(OutputDeviceInfo {
            id,
            name,
            is_default,
        });
    }

    Ok(out)
}

fn find_output_device_by_id(id: &str) -> Result<Device, String> {
    let host = cpal::default_host();

    if let Ok(idx) = id.parse::<usize>() {
        let list: Vec<_> = host
            .output_devices()
            .map_err(|e| format!("cpal output_devices: {e}"))?
            .collect();
        return list
            .into_iter()
            .nth(idx)
            .ok_or_else(|| format!("device_id out of range: {id}"));
    }

    let mut seen = HashMap::new();
    for dev in host
        .output_devices()
        .map_err(|e| format!("cpal output_devices: {e}"))?
    {
        let name = dev.name().unwrap_or_else(|_| String::new());
        let uid = unique_device_id(&name, &mut seen);
        if uid == id {
            return Ok(dev);
        }
    }
    Err(format!("unknown device_id: {id}"))
}

fn resolve_device(device_id: Option<&str>) -> Result<Device, String> {
    match device_id.filter(|s| !s.is_empty()) {
        None => cpal::default_host()
            .default_output_device()
            .ok_or_else(|| "no default output device".to_string()),
        Some(id) => find_output_device_by_id(id),
    }
}

fn find_input_device_by_id(id: &str) -> Result<Device, String> {
    let host = cpal::default_host();

    if let Ok(idx) = id.parse::<usize>() {
        let list: Vec<_> = host
            .input_devices()
            .map_err(|e| format!("cpal input_devices: {e}"))?
            .collect();
        return list
            .into_iter()
            .nth(idx)
            .ok_or_else(|| format!("device_id out of range: {id}"));
    }

    let mut seen = HashMap::new();
    for dev in host
        .input_devices()
        .map_err(|e| format!("cpal input_devices: {e}"))?
    {
        let name = dev.name().unwrap_or_else(|_| String::new());
        let uid = unique_device_id(&name, &mut seen);
        if uid == id {
            return Ok(dev);
        }
    }
    Err(format!("unknown device_id: {id}"))
}

fn resolve_input_device(device_id: Option<&str>) -> Result<Device, String> {
    match device_id.filter(|s| !s.is_empty()) {
        None => cpal::default_host()
            .default_input_device()
            .ok_or_else(|| "no default input device".to_string()),
        Some(id) => find_input_device_by_id(id),
    }
}

fn get_output_device_info(device_id: Option<&str>) -> Result<serde_json::Value, String> {
    let device = resolve_device(device_id)?;

    let name = device.name().unwrap_or_default();
    let cfg = device
        .default_output_config()
        .map_err(|e| format!("default_output_config: {e}"))?;

    let mut rate_min = None::<u32>;
    let mut rate_max = None::<u32>;
    if let Ok(configs) = device.supported_output_configs() {
        for range in configs {
            let mn = range.min_sample_rate().0;
            let mx = range.max_sample_rate().0;
            rate_min = Some(rate_min.map_or(mn, |a: u32| a.min(mn)));
            rate_max = Some(rate_max.map_or(mx, |a: u32| a.max(mx)));
        }
    }

    let mut j = json!({
        "ok": true,
        "device_name": name,
        "sample_rate_hz": cfg.sample_rate().0,
        "channels": cfg.channels(),
        "sample_format": format!("{:?}", cfg.sample_format()),
        "buffer_size": buffer_size_json(cfg.buffer_size()),
    });
    if let (Some(lo), Some(hi)) = (rate_min, rate_max) {
        j.as_object_mut().unwrap().insert(
            "sample_rate_range_hz".to_string(),
            json!({ "min": lo, "max": hi }),
        );
    }
    Ok(j)
}

fn get_input_device_info(device_id: Option<&str>) -> Result<serde_json::Value, String> {
    let device = resolve_input_device(device_id)?;

    let name = device.name().unwrap_or_default();
    let cfg = device
        .default_input_config()
        .map_err(|e| format!("default_input_config: {e}"))?;

    let mut rate_min = None::<u32>;
    let mut rate_max = None::<u32>;
    if let Ok(configs) = device.supported_input_configs() {
        for range in configs {
            let mn = range.min_sample_rate().0;
            let mx = range.max_sample_rate().0;
            rate_min = Some(rate_min.map_or(mn, |a: u32| a.min(mn)));
            rate_max = Some(rate_max.map_or(mx, |a: u32| a.max(mx)));
        }
    }

    let mut j = json!({
        "ok": true,
        "device_name": name,
        "sample_rate_hz": cfg.sample_rate().0,
        "channels": cfg.channels(),
        "sample_format": format!("{:?}", cfg.sample_format()),
        "buffer_size": buffer_size_json(cfg.buffer_size()),
    });
    if let (Some(lo), Some(hi)) = (rate_min, rate_max) {
        j.as_object_mut().unwrap().insert(
            "sample_rate_range_hz".to_string(),
            json!({ "min": lo, "max": hi }),
        );
    }
    Ok(j)
}

fn set_output_device(device_id: Option<&str>) -> Result<serde_json::Value, String> {
    let Some(id) = device_id.filter(|s| !s.is_empty()) else {
        return Err("device_id required".to_string());
    };
    let _device = find_output_device_by_id(id)?;
    Ok(json!({
        "ok": true,
        "device_id": id,
        "note": "validated; use start_output_stream to open the device",
    }))
}

fn set_input_device(device_id: Option<&str>) -> Result<serde_json::Value, String> {
    let Some(id) = device_id.filter(|s| !s.is_empty()) else {
        return Err("device_id required".to_string());
    };
    let _device = find_input_device_by_id(id)?;
    Ok(json!({
        "ok": true,
        "device_id": id,
        "note": "validated; use start_input_stream to open capture",
    }))
}

fn update_input_peak_linear(peak_bits: &Arc<AtomicU32>, block_peak: f32) {
    let a = block_peak.clamp(0.0, 1.0);
    let old = f32::from_bits(peak_bits.load(Ordering::Relaxed));
    let next = if a > old { a } else { old * INPUT_PEAK_DECAY };
    peak_bits.store(next.min(1.0).to_bits(), Ordering::Relaxed);
}

/// Input: consume buffers; update `peak_bits` (0..1 linear) for status JSON.
fn build_capture_stream(
    device: &Device,
    supported: SupportedStreamConfig,
    buffer_frames: Option<u32>,
    peak_bits: Arc<AtomicU32>,
) -> Result<(Stream, Option<u32>), String> {
    let sf = supported.sample_format();
    let bs = supported.buffer_size();
    let mut stream_config = supported.config();
    let stream_buffer_frames = apply_buffer_frames_request(&mut stream_config, bs, buffer_frames);
    let err = |e| eprintln!("audio-engine input stream error: {e}");
    match sf {
        SampleFormat::F32 => {
            let p = peak_bits;
            let stream = device
                .build_input_stream(
                    &stream_config,
                    move |data: &[f32], _: &cpal::InputCallbackInfo| {
                        let m = data.iter().fold(0.0f32, |acc, &x| acc.max(x.abs()));
                        update_input_peak_linear(&p, m);
                    },
                    err,
                    None,
                )
                .map_err(|e| e.to_string())?;
            stream.play().map_err(|e| e.to_string())?;
            Ok((stream, stream_buffer_frames))
        }
        SampleFormat::I16 => {
            let p = peak_bits;
            let stream = device
                .build_input_stream(
                    &stream_config,
                    move |data: &[i16], _: &cpal::InputCallbackInfo| {
                        let m = data.iter().fold(0.0f32, |acc, &x| {
                            acc.max((x as f32 / 32768.0).abs())
                        });
                        update_input_peak_linear(&p, m);
                    },
                    err,
                    None,
                )
                .map_err(|e| e.to_string())?;
            stream.play().map_err(|e| e.to_string())?;
            Ok((stream, stream_buffer_frames))
        }
        SampleFormat::U16 => {
            let p = peak_bits;
            let stream = device
                .build_input_stream(
                    &stream_config,
                    move |data: &[u16], _: &cpal::InputCallbackInfo| {
                        let m = data.iter().fold(0.0f32, |acc, &x| {
                            acc.max(((x as f32 - 32768.0) / 32768.0).abs())
                        });
                        update_input_peak_linear(&p, m);
                    },
                    err,
                    None,
                )
                .map_err(|e| e.to_string())?;
            stream.play().map_err(|e| e.to_string())?;
            Ok((stream, stream_buffer_frames))
        }
        SampleFormat::I8 => {
            let p = peak_bits;
            let stream = device
                .build_input_stream(
                    &stream_config,
                    move |data: &[i8], _: &cpal::InputCallbackInfo| {
                        let m = data.iter().fold(0.0f32, |acc, &x| {
                            acc.max((x as f32 / 128.0).abs())
                        });
                        update_input_peak_linear(&p, m);
                    },
                    err,
                    None,
                )
                .map_err(|e| e.to_string())?;
            stream.play().map_err(|e| e.to_string())?;
            Ok((stream, stream_buffer_frames))
        }
        SampleFormat::U8 => {
            let p = peak_bits;
            let stream = device
                .build_input_stream(
                    &stream_config,
                    move |data: &[u8], _: &cpal::InputCallbackInfo| {
                        let m = data.iter().fold(0.0f32, |acc, &x| {
                            acc.max(((x as f32 - 128.0) / 128.0).abs())
                        });
                        update_input_peak_linear(&p, m);
                    },
                    err,
                    None,
                )
                .map_err(|e| e.to_string())?;
            stream.play().map_err(|e| e.to_string())?;
            Ok((stream, stream_buffer_frames))
        }
        SampleFormat::I32 => {
            let p = peak_bits;
            let stream = device
                .build_input_stream(
                    &stream_config,
                    move |data: &[i32], _: &cpal::InputCallbackInfo| {
                        let m = data.iter().fold(0.0f32, |acc, &x| {
                            let v = (x as f64 / i32::MAX as f64).abs().min(1.0) as f32;
                            acc.max(v)
                        });
                        update_input_peak_linear(&p, m);
                    },
                    err,
                    None,
                )
                .map_err(|e| e.to_string())?;
            stream.play().map_err(|e| e.to_string())?;
            Ok((stream, stream_buffer_frames))
        }
        SampleFormat::U32 => {
            let p = peak_bits;
            let stream = device
                .build_input_stream(
                    &stream_config,
                    move |data: &[u32], _: &cpal::InputCallbackInfo| {
                        let m = data.iter().fold(0.0f32, |acc, &x| {
                            let v = (x as f64 / u32::MAX as f64).abs().min(1.0) as f32;
                            acc.max(v)
                        });
                        update_input_peak_linear(&p, m);
                    },
                    err,
                    None,
                )
                .map_err(|e| e.to_string())?;
            stream.play().map_err(|e| e.to_string())?;
            Ok((stream, stream_buffer_frames))
        }
        SampleFormat::I64 => {
            let p = peak_bits;
            let stream = device
                .build_input_stream(
                    &stream_config,
                    move |data: &[i64], _: &cpal::InputCallbackInfo| {
                        let m = data.iter().fold(0.0f32, |acc, &x| {
                            let v = (x as f64 / i64::MAX as f64).abs().min(1.0) as f32;
                            acc.max(v)
                        });
                        update_input_peak_linear(&p, m);
                    },
                    err,
                    None,
                )
                .map_err(|e| e.to_string())?;
            stream.play().map_err(|e| e.to_string())?;
            Ok((stream, stream_buffer_frames))
        }
        SampleFormat::U64 => {
            let p = peak_bits;
            let stream = device
                .build_input_stream(
                    &stream_config,
                    move |data: &[u64], _: &cpal::InputCallbackInfo| {
                        let m = data.iter().fold(0.0f32, |acc, &x| {
                            let v = (x as f64 / u64::MAX as f64).abs().min(1.0) as f32;
                            acc.max(v)
                        });
                        update_input_peak_linear(&p, m);
                    },
                    err,
                    None,
                )
                .map_err(|e| e.to_string())?;
            stream.play().map_err(|e| e.to_string())?;
            Ok((stream, stream_buffer_frames))
        }
        SampleFormat::F64 => {
            let p = peak_bits;
            let stream = device
                .build_input_stream(
                    &stream_config,
                    move |data: &[f64], _: &cpal::InputCallbackInfo| {
                        let m = data
                            .iter()
                            .fold(0.0f64, |acc, &x| acc.max(x.abs()))
                            .min(1.0) as f32;
                        update_input_peak_linear(&p, m);
                    },
                    err,
                    None,
                )
                .map_err(|e| e.to_string())?;
            stream.play().map_err(|e| e.to_string())?;
            Ok((stream, stream_buffer_frames))
        }
        _ => Err(format!("unsupported sample format {:?}", sf)),
    }
}

/// F32: file playback ring, optional test tone, or silence. Other formats: silence only, `tone_flag` = None.
///
/// When `defer_play` is true and `file_playback` is some, the stream is built but **`play()` is not
/// called** — the caller must run [`playback::begin_playback`] (prefill) then [`Stream::play`].
/// This avoids draining an empty ring before the decoder thread exists.
fn build_playback_stream(
    device: &Device,
    supported: SupportedStreamConfig,
    tone_initial: bool,
    buffer_frames: Option<u32>,
    file_playback: Option<Arc<playback::PlaybackShared>>,
    defer_play: bool,
) -> Result<(Stream, Option<Arc<AtomicBool>>, Option<u32>, u32, bool), String> {
    let sf = supported.sample_format();
    let bs = supported.buffer_size();
    let mut stream_config = supported.config();
    let stream_buffer_frames = apply_buffer_frames_request(&mut stream_config, bs, buffer_frames);
    let stream_sr = stream_config.sample_rate.0;
    let err = |e| eprintln!("audio-engine stream error: {e}");
    match sf {
        SampleFormat::F32 => {
            let sr = stream_sr as f32;
            let ch = supported.channels() as usize;
            let tone_flag = Arc::new(AtomicBool::new(tone_initial));
            let phase = Arc::new(AtomicU64::new(0));
            let t = tone_flag.clone();
            let p = phase.clone();
            let pb = file_playback.clone();
            let stream = device.build_output_stream(
                &stream_config,
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    let ch = ch.max(1);
                    if let Some(ref shared) = pb {
                        shared.fill_interleaved_f32(data, ch);
                        return;
                    }
                    let frames = data.len() / ch;
                    if t.load(Ordering::Relaxed) {
                        let mut i = p.load(Ordering::Relaxed);
                        for f in 0..frames {
                            let x = (i as f32) * TEST_TONE_HZ * 2.0 * std::f32::consts::PI / sr;
                            let s = x.sin() * TEST_TONE_GAIN;
                            for c in 0..ch {
                                data[f * ch + c] = s;
                            }
                            i += 1;
                        }
                        p.store(i, Ordering::Relaxed);
                    } else {
                        data.fill(0.0);
                        let mut i = p.load(Ordering::Relaxed);
                        i += frames as u64;
                        p.store(i, Ordering::Relaxed);
                    }
                },
                err,
                None,
            )
            .map_err(|e| e.to_string())?;
            let started = if defer_play && file_playback.is_some() {
                false
            } else {
                stream.play().map_err(|e| e.to_string())?;
                true
            };
            Ok((stream, Some(tone_flag), stream_buffer_frames, stream_sr, started))
        }
        SampleFormat::I16 => {
            let stream = device.build_output_stream(
                &stream_config,
                |data: &mut [i16], _: &cpal::OutputCallbackInfo| data.fill(0),
                err,
                None,
            )
            .map_err(|e| e.to_string())?;
            stream.play().map_err(|e| e.to_string())?;
            Ok((stream, None, stream_buffer_frames, stream_sr, true))
        }
        SampleFormat::U16 => {
            let stream = device.build_output_stream(
                &stream_config,
                |data: &mut [u16], _: &cpal::OutputCallbackInfo| data.fill(32768),
                err,
                None,
            )
            .map_err(|e| e.to_string())?;
            stream.play().map_err(|e| e.to_string())?;
            Ok((stream, None, stream_buffer_frames, stream_sr, true))
        }
        SampleFormat::I8 => {
            let stream = device.build_output_stream(
                &stream_config,
                |data: &mut [i8], _: &cpal::OutputCallbackInfo| data.fill(0),
                err,
                None,
            )
            .map_err(|e| e.to_string())?;
            stream.play().map_err(|e| e.to_string())?;
            Ok((stream, None, stream_buffer_frames, stream_sr, true))
        }
        SampleFormat::U8 => {
            let stream = device.build_output_stream(
                &stream_config,
                |data: &mut [u8], _: &cpal::OutputCallbackInfo| data.fill(128),
                err,
                None,
            )
            .map_err(|e| e.to_string())?;
            stream.play().map_err(|e| e.to_string())?;
            Ok((stream, None, stream_buffer_frames, stream_sr, true))
        }
        SampleFormat::I32 => {
            let stream = device.build_output_stream(
                &stream_config,
                |data: &mut [i32], _: &cpal::OutputCallbackInfo| data.fill(0),
                err,
                None,
            )
            .map_err(|e| e.to_string())?;
            stream.play().map_err(|e| e.to_string())?;
            Ok((stream, None, stream_buffer_frames, stream_sr, true))
        }
        SampleFormat::U32 => {
            let stream = device.build_output_stream(
                &stream_config,
                |data: &mut [u32], _: &cpal::OutputCallbackInfo| data.fill(1u32 << 31),
                err,
                None,
            )
            .map_err(|e| e.to_string())?;
            stream.play().map_err(|e| e.to_string())?;
            Ok((stream, None, stream_buffer_frames, stream_sr, true))
        }
        SampleFormat::I64 => {
            let stream = device.build_output_stream(
                &stream_config,
                |data: &mut [i64], _: &cpal::OutputCallbackInfo| data.fill(0),
                err,
                None,
            )
            .map_err(|e| e.to_string())?;
            stream.play().map_err(|e| e.to_string())?;
            Ok((stream, None, stream_buffer_frames, stream_sr, true))
        }
        SampleFormat::U64 => {
            let stream = device.build_output_stream(
                &stream_config,
                |data: &mut [u64], _: &cpal::OutputCallbackInfo| data.fill(1u64 << 63),
                err,
                None,
            )
            .map_err(|e| e.to_string())?;
            stream.play().map_err(|e| e.to_string())?;
            Ok((stream, None, stream_buffer_frames, stream_sr, true))
        }
        SampleFormat::F64 => {
            let stream = device.build_output_stream(
                &stream_config,
                |data: &mut [f64], _: &cpal::OutputCallbackInfo| data.fill(0.0),
                err,
                None,
            )
            .map_err(|e| e.to_string())?;
            stream.play().map_err(|e| e.to_string())?;
            Ok((stream, None, stream_buffer_frames, stream_sr, true))
        }
        _ => Err(format!("unsupported sample format {:?}", sf)),
    }
}

fn start_output_stream(
    device_id: Option<&str>,
    tone: bool,
    buffer_frames: Option<u32>,
    start_playback: bool,
) -> Result<serde_json::Value, String> {
    let (reply_tx, reply_rx) = mpsc::channel();
    audio_thread_tx()
        .send(AudioCmd::Start {
            device_id: device_id.map(|s| s.to_string()),
            tone,
            buffer_frames,
            start_playback,
            reply: reply_tx,
        })
        .map_err(|_| "audio engine thread unavailable".to_string())?;
    reply_rx
        .recv()
        .map_err(|_| "audio engine thread stopped".to_string())?
}

fn stop_output_stream() -> Result<serde_json::Value, String> {
    let (reply_tx, reply_rx) = mpsc::channel();
    audio_thread_tx()
        .send(AudioCmd::Stop { reply: reply_tx })
        .map_err(|_| "audio engine thread unavailable".to_string())?;
    let had = reply_rx
        .recv()
        .map_err(|_| "audio engine thread stopped".to_string())??;

    Ok(json!({
        "ok": true,
        "was_running": had,
    }))
}

fn start_input_stream(
    device_id: Option<&str>,
    buffer_frames: Option<u32>,
) -> Result<serde_json::Value, String> {
    let (reply_tx, reply_rx) = mpsc::channel();
    audio_thread_tx()
        .send(AudioCmd::StartInput {
            device_id: device_id.map(|s| s.to_string()),
            buffer_frames,
            reply: reply_tx,
        })
        .map_err(|_| "audio engine thread unavailable".to_string())?;
    reply_rx
        .recv()
        .map_err(|_| "audio engine thread stopped".to_string())?
}

fn stop_input_stream() -> Result<serde_json::Value, String> {
    let (reply_tx, reply_rx) = mpsc::channel();
    audio_thread_tx()
        .send(AudioCmd::StopInput { reply: reply_tx })
        .map_err(|_| "audio engine thread unavailable".to_string())?;
    let had = reply_rx
        .recv()
        .map_err(|_| "audio engine thread stopped".to_string())??;

    Ok(json!({
        "ok": true,
        "was_running": had,
    }))
}

fn output_stream_status() -> Result<serde_json::Value, String> {
    let (reply_tx, reply_rx) = mpsc::channel();
    audio_thread_tx()
        .send(AudioCmd::Status { reply: reply_tx })
        .map_err(|_| "audio engine thread unavailable".to_string())?;
    reply_rx
        .recv()
        .map_err(|_| "audio engine thread stopped".to_string())?
}

fn input_stream_status() -> Result<serde_json::Value, String> {
    let (reply_tx, reply_rx) = mpsc::channel();
    audio_thread_tx()
        .send(AudioCmd::InputStatus { reply: reply_tx })
        .map_err(|_| "audio engine thread unavailable".to_string())?;
    reply_rx
        .recv()
        .map_err(|_| "audio engine thread stopped".to_string())?
}

fn set_output_tone(enabled: bool) -> Result<serde_json::Value, String> {
    let (reply_tx, reply_rx) = mpsc::channel();
    audio_thread_tx()
        .send(AudioCmd::SetTone {
            enabled,
            reply: reply_tx,
        })
        .map_err(|_| "audio engine thread unavailable".to_string())?;
    reply_rx
        .recv()
        .map_err(|_| "audio engine thread stopped".to_string())??;
    Ok(json!({
        "ok": true,
        "tone": enabled,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use cpal::SampleRate;

    #[test]
    fn unique_device_id_counts_duplicates() {
        let mut seen = HashMap::new();
        assert_eq!(unique_device_id("A", &mut seen), "A");
        assert_eq!(unique_device_id("A", &mut seen), "A#2");
        assert_eq!(unique_device_id("A", &mut seen), "A#3");
        assert_eq!(unique_device_id("B", &mut seen), "B");
    }

    #[test]
    fn buffer_size_json_range_and_unknown() {
        let j = buffer_size_json(&SupportedBufferSize::Range { min: 64, max: 512 });
        assert_eq!(j["kind"], "range");
        assert_eq!(j["min"], 64);
        assert_eq!(j["max"], 512);
        let u = buffer_size_json(&SupportedBufferSize::Unknown);
        assert_eq!(u["kind"], "unknown");
    }

    #[test]
    fn apply_buffer_frames_clamps_to_range() {
        let mut cfg = StreamConfig {
            channels: 2,
            sample_rate: SampleRate(48_000),
            buffer_size: BufferSize::Default,
        };
        let r = apply_buffer_frames_request(
            &mut cfg,
            &SupportedBufferSize::Range { min: 64, max: 512 },
            Some(10_000),
        );
        assert_eq!(r, Some(512));
        assert_eq!(cfg.buffer_size, BufferSize::Fixed(512));
    }

    #[test]
    fn apply_buffer_frames_none_leaves_default() {
        let mut cfg = StreamConfig {
            channels: 2,
            sample_rate: SampleRate(48_000),
            buffer_size: BufferSize::Default,
        };
        let r = apply_buffer_frames_request(
            &mut cfg,
            &SupportedBufferSize::Range { min: 64, max: 512 },
            None,
        );
        assert_eq!(r, None);
        assert_eq!(cfg.buffer_size, BufferSize::Default);
    }

    #[test]
    fn apply_buffer_frames_caps_absurd_request_before_unknown() {
        let mut cfg = StreamConfig {
            channels: 2,
            sample_rate: SampleRate(48_000),
            buffer_size: BufferSize::Default,
        };
        let r = apply_buffer_frames_request(
            &mut cfg,
            &SupportedBufferSize::Unknown,
            Some(144_000),
        );
        assert_eq!(r, Some(MAX_BUFFER_FRAMES));
        assert_eq!(cfg.buffer_size, BufferSize::Fixed(MAX_BUFFER_FRAMES));
    }
}
