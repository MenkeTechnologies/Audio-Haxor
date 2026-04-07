//! **Audio engine** subprocess: the main app spawns the **`audio-engine`** JUCE sidecar (`audio-engine/` CMake target),
//! sends JSON lines on stdin, reads one JSON line per request. Keeps **one** child process alive
//! (stdin loop in the sidecar) so stream state and IPC stay cheap.
//! On app quit, [`shutdown_audio_engine_child`] runs from Tauri `RunEvent::Exit` / `ExitRequested` and from `libc::atexit`
//! so the sidecar is always terminated with the host. **`AUDIO_HAXOR_PARENT_PID`** is set at spawn so the sidecar can
//! exit if the host disappears without cleanup (e.g. macOS force quit / SIGKILL).
//!
//! **Shutdown must not take [`ENGINE_CHILD`] before killing the OS process.** Another thread can
//! hold that mutex while blocked on `stdout.read_line()`; waiting on the mutex first deadlocks
//! quit (sidecar never receives `kill`). We keep the child PID in [`ENGINE_CHILD_PID`] and send
//! `SIGKILL` / `taskkill /F` first, then clear the slot.

use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime};

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
    /// Recent stderr from the sidecar (crash/assert output) for `app.log` when IPC fails.
    stderr_tail: Arc<Mutex<String>>,
    /// Which binary we spawned; must respawn if [`resolve_audio_engine_binary`] starts returning a different path.
    binary_path: PathBuf,
    /// `metadata().modified()` + `len()` when spawned — same path can be overwritten when the sidecar is rebuilt.
    binary_identity: Option<(SystemTime, u64)>,
}

fn format_stderr_suffix(tail: &Arc<Mutex<String>>) -> String {
    tail.lock()
        .ok()
        .map(|g| {
            let s = g.trim();
            if s.is_empty() {
                String::new()
            } else {
                format!(
                    " | stderr (last): {}",
                    s.chars().take(800).collect::<String>()
                )
            }
        })
        .unwrap_or_default()
}

/// Log host-side IPC failure to `app.log`, appending recent sidecar stderr when available.
fn log_ipc_failure(msg: impl Into<String>, tail: Option<&Arc<Mutex<String>>>) {
    let msg = msg.into();
    let suffix = tail.map(format_stderr_suffix).unwrap_or_default();
    crate::write_app_log(format!("audio-engine IPC error: {msg}{suffix}"));
}

static ENGINE_CHILD: Mutex<Option<EngineChild>> = Mutex::new(None);
/// OS PID of the current sidecar (`Child::id`), or `0` if none. Used for kill-on-exit without
/// waiting on [`ENGINE_CHILD`] (see module comment).
static ENGINE_CHILD_PID: AtomicU32 = AtomicU32::new(0);

#[inline]
fn record_engine_pid(child: &Child) {
    ENGINE_CHILD_PID.store(child.id(), Ordering::SeqCst);
}

#[inline]
fn clear_engine_pid() {
    ENGINE_CHILD_PID.store(0, Ordering::SeqCst);
}

/// Hard-kill by PID so quit works even when no [`Child`] handle is available.
fn kill_pid_raw(pid: u32) {
    if pid == 0 {
        return;
    }
    #[cfg(unix)]
    unsafe {
        let _ = libc::kill(pid as libc::pid_t, libc::SIGKILL);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let _ = std::process::Command::new("taskkill")
            .args(["/F", "/PID", &pid.to_string()])
            .creation_flags(CREATE_NO_WINDOW)
            .status();
    }
}

/// Drop the in-memory slot after the OS process is dead or being replaced. Caller must have
/// cleared or updated [`ENGINE_CHILD_PID`] appropriately.
fn take_and_reap_engine_child(guard: &mut Option<EngineChild>) {
    if let Some(mut eng) = guard.take() {
        clear_engine_pid();
        let _ = eng.child.kill();
        let _ = eng.child.wait();
    }
}

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
    let app_log = crate::history::get_data_dir().join("app.log");
    let mut child = Command::new(path)
        .env("AUDIO_HAXOR_APP_LOG", app_log.as_os_str())
        .env(
            "AUDIO_HAXOR_PARENT_PID",
            format!("{}", std::process::id()),
        )
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            log_ipc_failure(format!("failed to spawn {}: {e}", path.display()), None);
            format!("spawn {}: {e}", path.display())
        })?;
    let stdin = child.stdin.take().ok_or_else(|| "audio-engine: no stdin".to_string())?;
    let stdout = child.stdout.take().ok_or_else(|| "audio-engine: no stdout".to_string())?;
    let stderr = child.stderr.take().ok_or_else(|| "audio-engine: no stderr".to_string())?;

    let stderr_tail = Arc::new(Mutex::new(String::new()));
    let tail_for_thread = Arc::clone(&stderr_tail);
    thread::spawn(move || {
        let mut reader = BufReader::new(stderr);
        let mut line = String::new();
        loop {
            line.clear();
            match reader.read_line(&mut line) {
                Ok(0) => break,
                Ok(_) => {
                    if let Ok(mut g) = tail_for_thread.lock() {
                        g.push_str(&line);
                        if g.len() > 8192 {
                            let trim = g.len().saturating_sub(4096);
                            g.drain(..trim);
                        }
                    }
                }
                Err(_) => break,
            }
        }
    });

    let stdout = BufReader::new(stdout);
    record_engine_pid(&child);
    Ok(EngineChild {
        child,
        stdin,
        stdout,
        stderr_tail,
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
        if guard.is_some() {
            take_and_reap_engine_child(&mut *guard);
        }
        *guard = Some(spawn_engine_child(path)?);
    }
    Ok(())
}

/// Drop the long-lived `audio-engine` child so the next IPC spawns a fresh process.
/// Use after a crash or when the sidecar stops responding.
///
/// Returns immediately after sending SIGKILL so the UI / toast is not blocked by mutex cleanup
/// (which can take many seconds if an IPC thread still holds [`ENGINE_CHILD`]). Reaping runs on
/// a background thread; the next `spawn_audio_engine_request` will spawn a fresh child if needed.
pub fn restart_audio_engine_child() -> Result<(), String> {
    let pid = ENGINE_CHILD_PID.swap(0, Ordering::SeqCst);
    if pid != 0 {
        kill_pid_raw(pid);
    }
    std::thread::spawn(|| {
        let reaped = clear_engine_slot_after_os_kill();
        if reaped {
            crate::write_app_log("audio-engine: sidecar process restarted (user request)".to_string());
        } else {
            log_ipc_failure(
                "ENGINE_CHILD mutex not acquired after OS kill (timeout); next IPC may respawn",
                None,
            );
        }
    });
    Ok(())
}

/// Reap `Child` handles after the OS process is gone. Never uses blocking `Mutex::lock()`.
/// Returns `true` if the slot was cleared; `false` if we gave up waiting (~30s).
fn clear_engine_slot_after_os_kill() -> bool {
    const MAX_ITERS: u32 = 6000;
    for _ in 0..MAX_ITERS {
        if let Ok(mut g) = ENGINE_CHILD.try_lock() {
            take_and_reap_engine_child(&mut *g);
            return true;
        }
        thread::sleep(Duration::from_millis(5));
    }
    false
}

/// Kill the JUCE sidecar when the host app exits. Idempotent (safe if no child was spawned).
pub fn shutdown_audio_engine_child() -> Result<(), String> {
    let pid = ENGINE_CHILD_PID.swap(0, Ordering::SeqCst);
    if pid != 0 {
        kill_pid_raw(pid);
    }
    let _ = clear_engine_slot_after_os_kill();
    crate::write_app_log("audio-engine: sidecar terminated (app shutdown)".to_string());
    Ok(())
}

/// Run one request against the audio-engine subprocess (stdin / stdout JSON lines).
pub fn spawn_audio_engine_request(request: &serde_json::Value) -> Result<serde_json::Value, String> {
    spawn_audio_engine_request_at(request)
}

fn spawn_audio_engine_request_at(request: &serde_json::Value) -> Result<serde_json::Value, String> {
    let payload = serde_json::to_string(request).map_err(|e| e.to_string())?;

    for attempt in 0..2 {
        let path = resolve_audio_engine_binary().map_err(|e| {
            log_ipc_failure(format!("failed to resolve audio-engine binary: {e}"), None);
            e
        })?;
        ensure_engine_child(&path)?;
        let mut guard = ENGINE_CHILD
            .lock()
            .map_err(|_| "audio-engine child mutex poisoned".to_string())?;
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
            let stderr_tail = Arc::clone(&eng.stderr_tail);
            clear_engine_pid();
            *guard = None;
            if attempt == 1 {
                log_ipc_failure("stdin write failed", Some(&stderr_tail));
                return Err("audio-engine stdin write failed".to_string());
            }
            continue;
        }

        let mut line = String::new();
        match eng.stdout.read_line(&mut line) {
            Ok(0) => {
                let stderr_tail = Arc::clone(&eng.stderr_tail);
                clear_engine_pid();
                *guard = None;
                if attempt == 1 {
                    log_ipc_failure(
                        "sidecar closed stdout (process exited or crashed)",
                        Some(&stderr_tail),
                    );
                    return Err("audio-engine closed stdout".to_string());
                }
                continue;
            }
            Ok(_) => {
                let line = line.trim();
                if line.is_empty() {
                    let stderr_tail = Arc::clone(&eng.stderr_tail);
                    clear_engine_pid();
                    *guard = None;
                    if attempt == 1 {
                        log_ipc_failure("empty JSON line on stdout", Some(&stderr_tail));
                        return Err("audio-engine returned empty stdout".to_string());
                    }
                    continue;
                }
                let v: serde_json::Value = match serde_json::from_str(line) {
                    Ok(v) => v,
                    Err(e) => {
                        log_ipc_failure(
                            format!("invalid JSON on stdout: {e}; line={line:?}"),
                            Some(&eng.stderr_tail),
                        );
                        return Err(format!("audio-engine JSON: {e}: {line}"));
                    }
                };
                // Long-lived child can outlive a fresh `node scripts/build-audio-engine.mjs`; the old process may
                // return `unknown cmd` for verbs added in a newer sidecar. Respawn once (see also
                // [`ensure_engine_child`] binary identity). Retry even if `ok` is missing — some
                // older builds only set `error`.
                if attempt == 0 {
                    if let Some(err) = v.get("error").and_then(|e| e.as_str()) {
                        if err.to_ascii_lowercase().contains("unknown cmd") {
                            clear_engine_pid();
                            *guard = None;
                            continue;
                        }
                    }
                }
                return Ok(v);
            }
            Err(e) => {
                let stderr_tail = Arc::clone(&eng.stderr_tail);
                clear_engine_pid();
                *guard = None;
                if attempt == 1 {
                    log_ipc_failure(format!("stdout read: {e}"), Some(&stderr_tail));
                    return Err(format!("audio-engine stdout: {e}"));
                }
            }
        }
    }
    log_ipc_failure("request failed after retry", None);
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
