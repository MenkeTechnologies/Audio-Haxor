//! Separate **audio-engine** process: output device discovery (via cpal) and stubs for future
//! real-time I/O and plugin hosting. Invoked by the main app with one JSON line on stdin; responds
//! with one JSON line on stdout.

use cpal::traits::{DeviceTrait, HostTrait};
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

fn main() {
    let mut line = String::new();
    if let Err(e) = std::io::stdin().read_line(&mut line) {
        eprintln!("audio-engine: stdin read failed: {e}");
        std::process::exit(1);
    }
    let trimmed = line.trim();
    if trimmed.is_empty() {
        eprintln!("audio-engine: empty request");
        std::process::exit(1);
    }
    let req: Request = match serde_json::from_str(trimmed) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("audio-engine: bad JSON: {e}");
            std::process::exit(1);
        }
    };
    let resp = match dispatch(&req) {
        Ok(v) => v,
        Err(msg) => json!({ "ok": false, "error": msg }),
    };
    println!("{}", resp);
}

fn dispatch(req: &Request) -> Result<serde_json::Value, String> {
    match req.cmd.as_str() {
        "ping" => Ok(json!({
            "ok": true,
            "version": ENGINE_VERSION,
            "host": cpal::default_host().id().name(),
        })),
        "list_output_devices" => list_output_devices(),
        "set_output_device" => set_output_device(req.device_id.as_deref()),
        "plugin_chain" => Ok(json!({
            "ok": true,
            "slots": [],
            "note": "plugin hosting will attach here",
        })),
        other => Err(format!("unknown cmd: {other}")),
    }
}

fn list_output_devices() -> Result<serde_json::Value, String> {
    let host = cpal::default_host();
    let default_dev = host.default_output_device();
    let default_name = default_dev
        .as_ref()
        .and_then(|d| d.name().ok());

    let devices: Vec<OutputDeviceInfo> = host
        .output_devices()
        .map_err(|e| format!("cpal output_devices: {e}"))?
        .enumerate()
        .map(|(i, dev)| {
            let name = dev.name().unwrap_or_else(|_| format!("device {i}"));
            let is_default = default_name
                .as_ref()
                .map(|dn| dn == &name)
                .unwrap_or(false);
            OutputDeviceInfo {
                id: i.to_string(),
                name,
                is_default,
            }
        })
        .collect();

    let default_id = devices
        .iter()
        .find(|d| d.is_default)
        .map(|d| d.id.clone());

    Ok(json!({
        "ok": true,
        "default_device_id": default_id,
        "devices": devices,
    }))
}

fn set_output_device(device_id: Option<&str>) -> Result<serde_json::Value, String> {
    let Some(id) = device_id.filter(|s| !s.is_empty()) else {
        return Err("device_id required".to_string());
    };
    let idx: usize = id
        .parse()
        .map_err(|_| format!("invalid device_id: {id}"))?;
    let host = cpal::default_host();
    let list: Vec<_> = host
        .output_devices()
        .map_err(|e| format!("cpal output_devices: {e}"))?
        .collect();
    if idx >= list.len() {
        return Err(format!("device_id out of range: {id}"));
    }
    let _device = &list[idx];
    Ok(json!({
        "ok": true,
        "device_id": id,
        "note": "selection stored by UI; real-time stream not started yet",
    }))
}
