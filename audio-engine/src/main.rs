//! Separate **audio-engine** process: output device discovery (via cpal) and low-latency output.
//! Reads JSON lines on stdin until EOF; prints one JSON line per request. The host keeps one
//! child process and reuses stdin/stdout. **Output streams** are owned on a dedicated thread because
//! `cpal::Stream` is not `Send` on macOS.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::sync::mpsc::{self, Sender};
use std::sync::OnceLock;
use std::thread;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, SampleFormat, Stream, SupportedStreamConfig};
use serde::{Deserialize, Serialize};
use serde_json::json;

const ENGINE_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Deserialize)]
struct Request {
    cmd: String,
    #[serde(default)]
    device_id: Option<String>,
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
    buffer_size: String,
}

enum AudioCmd {
    Start {
        device_id: Option<String>,
        reply: mpsc::Sender<Result<serde_json::Value, String>>,
    },
    Stop {
        reply: mpsc::Sender<Result<bool, String>>,
    },
    Status {
        reply: mpsc::Sender<Result<serde_json::Value, String>>,
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
            AudioCmd::Start { device_id, reply } => {
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
                    let buffer_size = format!("{:?}", supported.buffer_size());
                    let stream = build_silence_stream(&device, supported)?;
                    current = Some(ActiveStream {
                        stream,
                        device_id: resolved_id.clone(),
                        device_name: device_name.clone(),
                        sample_rate_hz,
                        channels,
                        sample_format: sample_format.clone(),
                        buffer_size: buffer_size.clone(),
                    });
                    Ok(json!({
                        "ok": true,
                        "device_id": resolved_id,
                        "device_name": device_name,
                        "sample_rate_hz": sample_rate_hz,
                        "channels": channels,
                        "sample_format": sample_format,
                        "buffer_size": buffer_size,
                        "note": "silence stream running (placeholder for mixer/plugin graph)",
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
                    Some(a) => json!({
                        "ok": true,
                        "running": true,
                        "device_id": a.device_id,
                        "device_name": a.device_name,
                        "sample_rate_hz": a.sample_rate_hz,
                        "channels": a.channels,
                        "sample_format": a.sample_format,
                        "buffer_size": a.buffer_size,
                    }),
                    None => json!({
                        "ok": true,
                        "running": false,
                        "device_id": serde_json::Value::Null,
                        "device_name": serde_json::Value::Null,
                        "sample_rate_hz": serde_json::Value::Null,
                        "channels": serde_json::Value::Null,
                        "sample_format": serde_json::Value::Null,
                        "buffer_size": serde_json::Value::Null,
                    }),
                };
                let _ = reply.send(Ok(v));
            }
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
    match req.cmd.as_str() {
        "ping" => Ok(json!({
            "ok": true,
            "version": ENGINE_VERSION,
            "host": cpal::default_host().id().name(),
        })),
        "list_output_devices" => list_output_devices(),
        "get_output_device_info" => get_output_device_info(req.device_id.as_deref()),
        "set_output_device" => set_output_device(req.device_id.as_deref()),
        "start_output_stream" => start_output_stream(req.device_id.as_deref()),
        "stop_output_stream" => stop_output_stream(),
        "output_stream_status" => output_stream_status(),
        "plugin_chain" => Ok(json!({
            "ok": true,
            "slots": [],
            "note": "plugin hosting will attach here",
        })),
        other => Err(format!("unknown cmd: {other}")),
    }
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

fn build_silence_stream(device: &Device, supported: SupportedStreamConfig) -> Result<Stream, String> {
    let cfg = supported.config();
    let err = |e| eprintln!("audio-engine stream error: {e}");
    let stream = match supported.sample_format() {
        SampleFormat::F32 => device.build_output_stream(
            &cfg,
            |data: &mut [f32], _: &cpal::OutputCallbackInfo| data.fill(0.0),
            err,
            None,
        ),
        SampleFormat::I16 => device.build_output_stream(
            &cfg,
            |data: &mut [i16], _: &cpal::OutputCallbackInfo| data.fill(0),
            err,
            None,
        ),
        SampleFormat::U16 => device.build_output_stream(
            &cfg,
            |data: &mut [u16], _: &cpal::OutputCallbackInfo| data.fill(32768),
            err,
            None,
        ),
        SampleFormat::I8 => device.build_output_stream(
            &cfg,
            |data: &mut [i8], _: &cpal::OutputCallbackInfo| data.fill(0),
            err,
            None,
        ),
        SampleFormat::U8 => device.build_output_stream(
            &cfg,
            |data: &mut [u8], _: &cpal::OutputCallbackInfo| data.fill(128),
            err,
            None,
        ),
        SampleFormat::I32 => device.build_output_stream(
            &cfg,
            |data: &mut [i32], _: &cpal::OutputCallbackInfo| data.fill(0),
            err,
            None,
        ),
        SampleFormat::U32 => device.build_output_stream(
            &cfg,
            |data: &mut [u32], _: &cpal::OutputCallbackInfo| data.fill(1 << 31),
            err,
            None,
        ),
        SampleFormat::I64 => device.build_output_stream(
            &cfg,
            |data: &mut [i64], _: &cpal::OutputCallbackInfo| data.fill(0),
            err,
            None,
        ),
        SampleFormat::U64 => device.build_output_stream(
            &cfg,
            |data: &mut [u64], _: &cpal::OutputCallbackInfo| data.fill(1u64 << 63),
            err,
            None,
        ),
        SampleFormat::F64 => device.build_output_stream(
            &cfg,
            |data: &mut [f64], _: &cpal::OutputCallbackInfo| data.fill(0.0),
            err,
            None,
        ),
        _ => {
            return Err(format!(
                "unsupported sample format {:?}",
                supported.sample_format()
            ));
        }
    }
    .map_err(|e| e.to_string())?;
    stream.play().map_err(|e| e.to_string())?;
    Ok(stream)
}

fn start_output_stream(device_id: Option<&str>) -> Result<serde_json::Value, String> {
    let (reply_tx, reply_rx) = mpsc::channel();
    audio_thread_tx()
        .send(AudioCmd::Start {
            device_id: device_id.map(|s| s.to_string()),
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
}
