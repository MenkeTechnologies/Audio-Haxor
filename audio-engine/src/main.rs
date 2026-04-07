//! Separate **audio-engine** process: output device discovery (via cpal) and low-latency output.
//! Reads JSON lines on stdin until EOF; prints one JSON line per request. The host keeps one
//! child process and reuses stdin/stdout. **Output streams** are owned on a dedicated thread because
//! `cpal::Stream` is not `Send` on macOS.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, OnceLock};
use std::thread;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, SampleFormat, Stream, SupportedBufferSize, SupportedStreamConfig};
use serde::{Deserialize, Serialize};
use serde_json::json;

const ENGINE_VERSION: &str = env!("CARGO_PKG_VERSION");
const TEST_TONE_HZ: f32 = 440.0;
const TEST_TONE_GAIN: f32 = 0.05;

#[derive(Debug, Deserialize)]
struct Request {
    cmd: String,
    #[serde(default)]
    device_id: Option<String>,
    /// When starting the output stream, enable 440 Hz test tone (F32 only).
    #[serde(default)]
    tone: bool,
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
    /// F32 streams only: toggles silence vs test tone in the callback.
    tone_flag: Option<Arc<AtomicBool>>,
}

enum AudioCmd {
    Start {
        device_id: Option<String>,
        tone: bool,
        reply: mpsc::Sender<Result<serde_json::Value, String>>,
    },
    Stop {
        reply: mpsc::Sender<Result<bool, String>>,
    },
    Status {
        reply: mpsc::Sender<Result<serde_json::Value, String>>,
    },
    SetTone {
        enabled: bool,
        reply: mpsc::Sender<Result<(), String>>,
    },
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
    while let Ok(cmd) = rx.recv() {
        match cmd {
            AudioCmd::Start {
                device_id,
                tone,
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
                    let supported = device
                        .default_output_config()
                        .map_err(|e| format!("default_output_config: {e}"))?;
                    let sample_rate_hz = supported.sample_rate().0;
                    let channels = supported.channels();
                    let sample_format = format!("{:?}", supported.sample_format());
                    let buffer_size = buffer_size_json(supported.buffer_size());
                    let (stream, tone_flag) =
                        build_playback_stream(&device, supported, tone)?;
                    let tone_supported = tone_flag.is_some();
                    let tone_on = tone_flag
                        .as_ref()
                        .map(|f| f.load(Ordering::Relaxed))
                        .unwrap_or(false);
                    current = Some(ActiveStream {
                        stream,
                        device_id: resolved_id.clone(),
                        device_name: device_name.clone(),
                        sample_rate_hz,
                        channels,
                        sample_format: sample_format.clone(),
                        buffer_size: buffer_size.clone(),
                        tone_flag,
                    });
                    Ok(json!({
                        "ok": true,
                        "device_id": resolved_id,
                        "device_name": device_name,
                        "sample_rate_hz": sample_rate_hz,
                        "channels": channels,
                        "sample_format": sample_format,
                        "buffer_size": buffer_size,
                        "tone_supported": tone_supported,
                        "tone_on": tone_on,
                        "note": "output stream running (silence or test tone); mixer/plugin graph TBD",
                    }))
                })();
                let _ = reply.send(res);
            }
            AudioCmd::Stop { reply } => {
                let had = current.take().is_some();
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
                        "tone_supported": serde_json::Value::Null,
                        "tone_on": serde_json::Value::Null,
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
    match req.cmd.as_str() {
        "ping" => Ok(json!({
            "ok": true,
            "version": ENGINE_VERSION,
            "host": cpal::default_host().id().name(),
        })),
        "engine_state" => engine_state(),
        "list_output_devices" => list_output_devices(),
        "list_input_devices" => list_input_devices(),
        "get_output_device_info" => get_output_device_info(req.device_id.as_deref()),
        "set_output_device" => set_output_device(req.device_id.as_deref()),
        "start_output_stream" => start_output_stream(req.device_id.as_deref(), req.tone),
        "stop_output_stream" => stop_output_stream(),
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
    Ok(json!({
        "ok": true,
        "version": ENGINE_VERSION,
        "host": cpal::default_host().id().name(),
        "stream": stream,
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

fn list_input_devices() -> Result<serde_json::Value, String> {
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

    let default_id = out.iter().find(|d| d.is_default).map(|d| d.id.clone());

    Ok(json!({
        "ok": true,
        "default_device_id": default_id,
        "devices": out,
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

/// F32: optional test tone via `tone_flag`. Other formats: silence only, `tone_flag` = None.
fn build_playback_stream(
    device: &Device,
    supported: SupportedStreamConfig,
    tone_initial: bool,
) -> Result<(Stream, Option<Arc<AtomicBool>>), String> {
    let cfg = supported.config();
    let err = |e| eprintln!("audio-engine stream error: {e}");
    let tone_out = match supported.sample_format() {
        SampleFormat::F32 => {
            let sr = supported.sample_rate().0 as f32;
            let ch = supported.channels() as usize;
            let tone_flag = Arc::new(AtomicBool::new(tone_initial));
            let phase = Arc::new(AtomicU64::new(0));
            let t = tone_flag.clone();
            let p = phase.clone();
            let stream = device.build_output_stream(
                &cfg,
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    let ch = ch.max(1);
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
            stream.play().map_err(|e| e.to_string())?;
            Ok((stream, Some(tone_flag)))
        }
        SampleFormat::I16 => {
            let stream = device.build_output_stream(
                &cfg,
                |data: &mut [i16], _: &cpal::OutputCallbackInfo| data.fill(0),
                err,
                None,
            )
            .map_err(|e| e.to_string())?;
            stream.play().map_err(|e| e.to_string())?;
            Ok((stream, None))
        }
        SampleFormat::U16 => {
            let stream = device.build_output_stream(
                &cfg,
                |data: &mut [u16], _: &cpal::OutputCallbackInfo| data.fill(32768),
                err,
                None,
            )
            .map_err(|e| e.to_string())?;
            stream.play().map_err(|e| e.to_string())?;
            Ok((stream, None))
        }
        SampleFormat::I8 => {
            let stream = device.build_output_stream(
                &cfg,
                |data: &mut [i8], _: &cpal::OutputCallbackInfo| data.fill(0),
                err,
                None,
            )
            .map_err(|e| e.to_string())?;
            stream.play().map_err(|e| e.to_string())?;
            Ok((stream, None))
        }
        SampleFormat::U8 => {
            let stream = device.build_output_stream(
                &cfg,
                |data: &mut [u8], _: &cpal::OutputCallbackInfo| data.fill(128),
                err,
                None,
            )
            .map_err(|e| e.to_string())?;
            stream.play().map_err(|e| e.to_string())?;
            Ok((stream, None))
        }
        SampleFormat::I32 => {
            let stream = device.build_output_stream(
                &cfg,
                |data: &mut [i32], _: &cpal::OutputCallbackInfo| data.fill(0),
                err,
                None,
            )
            .map_err(|e| e.to_string())?;
            stream.play().map_err(|e| e.to_string())?;
            Ok((stream, None))
        }
        SampleFormat::U32 => {
            let stream = device.build_output_stream(
                &cfg,
                |data: &mut [u32], _: &cpal::OutputCallbackInfo| data.fill(1u32 << 31),
                err,
                None,
            )
            .map_err(|e| e.to_string())?;
            stream.play().map_err(|e| e.to_string())?;
            Ok((stream, None))
        }
        SampleFormat::I64 => {
            let stream = device.build_output_stream(
                &cfg,
                |data: &mut [i64], _: &cpal::OutputCallbackInfo| data.fill(0),
                err,
                None,
            )
            .map_err(|e| e.to_string())?;
            stream.play().map_err(|e| e.to_string())?;
            Ok((stream, None))
        }
        SampleFormat::U64 => {
            let stream = device.build_output_stream(
                &cfg,
                |data: &mut [u64], _: &cpal::OutputCallbackInfo| data.fill(1u64 << 63),
                err,
                None,
            )
            .map_err(|e| e.to_string())?;
            stream.play().map_err(|e| e.to_string())?;
            Ok((stream, None))
        }
        SampleFormat::F64 => {
            let stream = device.build_output_stream(
                &cfg,
                |data: &mut [f64], _: &cpal::OutputCallbackInfo| data.fill(0.0),
                err,
                None,
            )
            .map_err(|e| e.to_string())?;
            stream.play().map_err(|e| e.to_string())?;
            Ok((stream, None))
        }
        _ => {
            return Err(format!(
                "unsupported sample format {:?}",
                supported.sample_format()
            ));
        }
    }?;
    tone_out
}

fn start_output_stream(device_id: Option<&str>, tone: bool) -> Result<serde_json::Value, String> {
    let (reply_tx, reply_rx) = mpsc::channel();
    audio_thread_tx()
        .send(AudioCmd::Start {
            device_id: device_id.map(|s| s.to_string()),
            tone,
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

fn output_stream_status() -> Result<serde_json::Value, String> {
    let (reply_tx, reply_rx) = mpsc::channel();
    audio_thread_tx()
        .send(AudioCmd::Status { reply: reply_tx })
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
}
