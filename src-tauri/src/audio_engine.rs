//! **Audio engine** subprocess: the main app spawns `audio-engine` (see `src/bin/audio_engine.rs`),
//! sends one JSON request line on stdin, reads one JSON line from stdout. Used for output device
//! discovery (cpal) and stubs for future real-time I/O and plugin hosting.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Placeholder struct kept for serde stability / future prefs sync.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AudioEngineStub {
    pub state: String,
}

impl Default for AudioEngineStub {
    fn default() -> Self {
        Self {
            state: "not_started".to_string(),
        }
    }
}

fn binary_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "audio-engine.exe"
    } else {
        "audio-engine"
    }
}

/// Resolve path to the `audio-engine` executable next to the running app binary (dev and bundled
/// sidecar both land in the same directory as `audio-haxor`).
pub fn resolve_audio_engine_binary() -> Result<PathBuf, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let dir = exe
        .parent()
        .ok_or_else(|| "current_exe has no parent directory".to_string())?;
    let p = dir.join(binary_name());
    if p.is_file() {
        return Ok(p);
    }
    Err(format!(
        "audio engine binary not found (expected {})",
        p.display()
    ))
}

/// Run one request against the audio-engine subprocess (stdin / stdout JSON lines).
pub fn spawn_audio_engine_request(request: &serde_json::Value) -> Result<serde_json::Value, String> {
    let path = resolve_audio_engine_binary()?;
    spawn_audio_engine_request_at(&path, request)
}

fn spawn_audio_engine_request_at(
    path: &Path,
    request: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    let payload = serde_json::to_string(request).map_err(|e| e.to_string())?;
    let mut child = Command::new(path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("spawn {}: {e}", path.display()))?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(payload.as_bytes())
            .map_err(|e| format!("audio-engine stdin: {e}"))?;
        stdin
            .write_all(b"\n")
            .map_err(|e| format!("audio-engine stdin: {e}"))?;
    }

    let out = child
        .wait_with_output()
        .map_err(|e| format!("audio-engine wait: {e}"))?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        let err = err.trim();
        if err.is_empty() {
            return Err(format!(
                "audio-engine exited with status {:?}",
                out.status.code()
            ));
        }
        return Err(err.to_string());
    }
    let text = String::from_utf8(out.stdout).map_err(|e| e.to_string())?;
    let line = text.lines().next().unwrap_or("{}").trim();
    if line.is_empty() {
        return Err("audio-engine returned empty stdout".to_string());
    }
    serde_json::from_str(line).map_err(|e| format!("audio-engine JSON: {e}: {line}"))
}

#[cfg(test)]
mod tests {
    #[test]
    fn parse_engine_response_line() {
        let s = r#"{"ok":true,"version":"1.0.0"}"#;
        let v: serde_json::Value = serde_json::from_str(s).unwrap();
        assert_eq!(v["ok"], true);
    }
}
