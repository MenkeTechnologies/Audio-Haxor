//! **Audio engine** subprocess: the main app spawns the **`audio-engine`** JUCE sidecar (`audio-engine/` CMake target),
//! sends JSON lines on stdin, reads one JSON line per request. Keeps **one** child process alive
//! (stdin loop in the sidecar) so stream state and IPC stay cheap.

use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use std::time::SystemTime;

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
    /// Which binary we spawned; must respawn if [`resolve_audio_engine_binary`] starts returning a different path.
    binary_path: PathBuf,
    /// `metadata().modified()` + `len()` when spawned — same path can be overwritten when the sidecar is rebuilt.
    binary_identity: Option<(SystemTime, u64)>,
}

static ENGINE_CHILD: Mutex<Option<EngineChild>> = Mutex::new(None);

fn binary_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "audio-engine.exe"
    } else {
        "audio-engine"
    }
}

/// Resolve path to the `audio-engine` executable.
///
/// Prefer a `target/debug` or `target/release` build found by walking **up** from [`std::env::current_exe`].
/// That covers `pnpm dev` when the app runs inside a macOS **bundle** (`…/target/debug/bundle/…/Contents/MacOS/audio-haxor`)
/// where the sibling `audio-engine` can be stale, while the real sidecar from `beforeDevCommand` lives
/// at the workspace `target/debug/audio-engine`. Also works when `CARGO_TARGET_DIR` is non-default
/// (compile-time `CARGO_MANIFEST_DIR` alone is insufficient).
///
/// Override for debugging: set `AUDIO_HAXOR_AUDIO_ENGINE` to an absolute path to the sidecar binary.
/// Release installs use the sibling next to the main executable when no workspace `target/` is found.
pub fn resolve_audio_engine_binary() -> Result<PathBuf, String> {
    if let Ok(p) = std::env::var("AUDIO_HAXOR_AUDIO_ENGINE") {
        let p = PathBuf::from(p.trim());
        if p.is_file() {
            return Ok(p);
        }
    }

    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let dir = exe
        .parent()
        .ok_or_else(|| "current_exe has no parent directory".to_string())?;
    let sibling = dir.join(binary_name());

    if let Some(p) = find_audio_engine_under_target_ancestors(&exe) {
        return Ok(p);
    }

    if sibling.is_file() {
        return Ok(sibling);
    }

    Err(format!(
        "audio engine binary not found (expected {} or workspace target/**/{})",
        sibling.display(),
        binary_name()
    ))
}

/// Walk parents of `exe` until `dir/target/debug|release/<binary>` exists (workspace root).
fn find_audio_engine_under_target_ancestors(exe: &Path) -> Option<PathBuf> {
    let mut dir = exe.parent()?;
    for _ in 0..40 {
        let dbg = dir.join("target").join("debug").join(binary_name());
        if dbg.is_file() {
            return Some(dbg);
        }
        let rel = dir.join("target").join("release").join(binary_name());
        if rel.is_file() {
            return Some(rel);
        }
        dir = dir.parent()?;
    }
    None
}

fn child_dead(child: &mut Child) -> bool {
    match child.try_wait() {
        Ok(Some(_)) => true,
        Ok(None) => false,
        Err(_) => true,
    }
}

fn spawn_engine_child(path: &Path) -> Result<EngineChild, String> {
    let identity = std::fs::metadata(path).ok().map(|m| (m.modified().unwrap_or_else(|_| SystemTime::UNIX_EPOCH), m.len()));
    let mut child = Command::new(path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|e| format!("spawn {}: {e}", path.display()))?;
    let stdin = child.stdin.take().ok_or_else(|| "audio-engine: no stdin".to_string())?;
    let stdout = child.stdout.take().ok_or_else(|| "audio-engine: no stdout".to_string())?;
    let stdout = BufReader::new(stdout);
    Ok(EngineChild {
        child,
        stdin,
        stdout,
        binary_path: path.to_path_buf(),
        binary_identity: identity,
    })
}

fn ensure_engine_child(path: &Path) -> Result<(), String> {
    let mut guard = ENGINE_CHILD
        .lock()
        .map_err(|_| "audio-engine child mutex poisoned")?;
    let disk_identity = std::fs::metadata(path).ok().map(|m| (m.modified().unwrap_or_else(|_| SystemTime::UNIX_EPOCH), m.len()));
    let need_spawn = match guard.as_mut() {
        None => true,
        Some(eng) => {
            child_dead(&mut eng.child)
                || eng.binary_path != path
                || disk_identity.is_some() && disk_identity != eng.binary_identity
        }
    };
    if need_spawn {
        *guard = Some(spawn_engine_child(path)?);
    }
    Ok(())
}

/// Run one request against the audio-engine subprocess (stdin / stdout JSON lines).
pub fn spawn_audio_engine_request(request: &serde_json::Value) -> Result<serde_json::Value, String> {
    spawn_audio_engine_request_at(request)
}

fn spawn_audio_engine_request_at(request: &serde_json::Value) -> Result<serde_json::Value, String> {
    let payload = serde_json::to_string(request).map_err(|e| e.to_string())?;

    for attempt in 0..2 {
        let path = resolve_audio_engine_binary()?;
        ensure_engine_child(&path)?;
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
                let v: serde_json::Value = serde_json::from_str(line)
                    .map_err(|e| format!("audio-engine JSON: {e}: {line}"))?;
                // Long-lived child can outlive a fresh `node scripts/build-audio-engine.mjs`; the old process may
                // return `unknown cmd` for verbs added in a newer sidecar. Respawn once (see also
                // [`ensure_engine_child`] binary identity). Retry even if `ok` is missing — some
                // older builds only set `error`.
                if attempt == 0 {
                    if let Some(err) = v.get("error").and_then(|e| e.as_str()) {
                        if err.to_ascii_lowercase().contains("unknown cmd") {
                            *guard = None;
                            continue;
                        }
                    }
                }
                return Ok(v);
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
