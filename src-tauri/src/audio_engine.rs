//! **Audio engine** subprocess: the main app spawns `audio-engine` (crate `audio-engine/`),
//! sends JSON lines on stdin, reads one JSON line per request. Keeps **one** child process alive
//! (stdin loop in the sidecar) so stream state and IPC stay cheap.

use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;

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

struct EngineChild {
    child: Child,
    stdin: std::process::ChildStdin,
    stdout: BufReader<std::process::ChildStdout>,
}

static ENGINE_CHILD: Mutex<Option<EngineChild>> = Mutex::new(None);

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

fn child_dead(child: &mut Child) -> bool {
    match child.try_wait() {
        Ok(Some(_)) => true,
        Ok(None) => false,
        Err(_) => true,
    }
}

fn spawn_engine_child(path: &Path) -> Result<EngineChild, String> {
    let mut child = Command::new(path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|e| format!("spawn {}: {e}", path.display()))?;
    let stdin = child.stdin.take().ok_or_else(|| "audio-engine: no stdin".to_string())?;
    let stdout = child.stdout.take().ok_or_else(|| "audio-engine: no stdout".to_string())?;
    let stdout = BufReader::new(stdout);
    Ok(EngineChild { child, stdin, stdout })
}

fn ensure_engine_child(path: &Path) -> Result<(), String> {
    let mut guard = ENGINE_CHILD
        .lock()
        .map_err(|_| "audio-engine child mutex poisoned")?;
    let need_spawn = match guard.as_mut() {
        None => true,
        Some(eng) => child_dead(&mut eng.child),
    };
    if need_spawn {
        *guard = Some(spawn_engine_child(path)?);
    }
    Ok(())
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

    for attempt in 0..2 {
        ensure_engine_child(path)?;
        let mut guard = ENGINE_CHILD
            .lock()
            .map_err(|_| "audio-engine child mutex poisoned")?;
        let eng = guard.as_mut().ok_or_else(|| "audio-engine child missing".to_string())?;

        if eng
            .stdin
            .write_all(payload.as_bytes())
            .map_err(|e| e.to_string())
            .and_then(|_| {
                eng.stdin
                    .write_all(b"\n")
                    .map_err(|e| format!("audio-engine stdin: {e}"))
            })
            .and_then(|_| eng.stdin.flush().map_err(|e| format!("audio-engine stdin: {e}")))
            .is_err()
        {
            *guard = None;
            if attempt == 1 {
                return Err("audio-engine stdin write failed".to_string());
            }
            continue;
        }

        let mut line = String::new();
        match eng.stdout.read_line(&mut line) {
            Ok(0) => {
                *guard = None;
                if attempt == 1 {
                    return Err("audio-engine closed stdout".to_string());
                }
                continue;
            }
            Ok(_) => {
                let line = line.trim();
                if line.is_empty() {
                    *guard = None;
                    if attempt == 1 {
                        return Err("audio-engine returned empty stdout".to_string());
                    }
                    continue;
                }
                return serde_json::from_str(line)
                    .map_err(|e| format!("audio-engine JSON: {e}: {line}"));
            }
            Err(e) => {
                *guard = None;
                if attempt == 1 {
                    return Err(format!("audio-engine stdout: {e}"));
                }
            }
        }
    }
    Err("audio-engine request failed after retry".to_string())
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
