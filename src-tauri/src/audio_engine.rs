//! **AudioEngine** subprocess: the main app spawns the **`audio-engine`** JUCE binary (`audio-engine/` CMake target),
//! sends JSON lines on stdin, reads one JSON line per request. Keeps a **main** child for playback
//! and device/plugin IPC, plus a **preview** child used only for `waveform_preview` /
//! `spectrogram_preview` so long visual decodes never block `playback_load` / transport on the main
//! stdin loop (one JSON line in → one line out per process).
//! On app quit, [`shutdown_audio_engine_child`] runs from Tauri `RunEvent::Exit` / `ExitRequested` and from `libc::atexit`
//! so the AudioEngine is always terminated with the host. **`AUDIO_HAXOR_PARENT_PID`** is set at spawn so the AudioEngine can
//! exit if the host disappears without cleanup (e.g. macOS force quit / SIGKILL).
//!
//! **Shutdown must not take [`ENGINE_CHILD`] before killing the OS process.** Another thread can
//! hold that mutex while blocked on `stdout.read_line()`; waiting on the mutex first deadlocks
//! quit (AudioEngine never receives `kill`). We keep the child PID in [`ENGINE_CHILD_PID`] and send
//! `SIGKILL` / `taskkill /F` first, then clear the slot.

use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, LazyLock, Mutex};
use std::thread;
use std::time::{Duration, SystemTime};

use tauri::{AppHandle, Emitter};

// ── Dedicated Audio Engine IPC Threads ──
// Background jobs use tokio::spawn_blocking which shares a thread pool. When saturated,
// audio IPC commands queue behind CPU-intensive work causing playback lag.
// These dedicated threads ensure audio commands never wait on background jobs.

type IpcRequest = (
    serde_json::Value,
    std::sync::mpsc::SyncSender<Result<serde_json::Value, String>>,
);

/// Dedicated thread for main engine IPC (playback, transport).
static MAIN_IPC_TX: LazyLock<std::sync::mpsc::SyncSender<IpcRequest>> = LazyLock::new(|| {
    let (tx, rx) = std::sync::mpsc::sync_channel::<IpcRequest>(256);
    thread::Builder::new()
        .name("ae-main-ipc".to_string())
        .spawn(move || {
            for (req, resp_tx) in rx {
                let r = do_main_engine_request(&req);
                let _ = resp_tx.send(r);
            }
        })
        .expect("ae-main-ipc thread");
    tx
});

/// Dedicated thread for preview engine IPC (waveform, spectrogram).
static PREVIEW_IPC_TX: LazyLock<std::sync::mpsc::SyncSender<IpcRequest>> = LazyLock::new(|| {
    let (tx, rx) = std::sync::mpsc::sync_channel::<IpcRequest>(128);
    thread::Builder::new()
        .name("ae-preview-ipc".to_string())
        .spawn(move || {
            for (req, resp_tx) in rx {
                let r = spawn_preview_engine_request_at(&req);
                let _ = resp_tx.send(r);
            }
        })
        .expect("ae-preview-ipc thread");
    tx
});

/// Route request to dedicated IPC thread and wait for response.
/// Does NOT use tokio::spawn_blocking — completely bypasses that pool.
pub fn dedicated_audio_engine_request(
    request: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    let cmd = request.get("cmd").and_then(|c| c.as_str()).unwrap_or("");
    let is_preview = cmd == "waveform_preview" || cmd == "spectrogram_preview";

    let (resp_tx, resp_rx) = std::sync::mpsc::sync_channel(1);

    let started = std::time::Instant::now();
    let (inflight_slot, peak_slot) = if is_preview {
        (&PREVIEW_INFLIGHT, &PREVIEW_INFLIGHT_PEAK)
    } else {
        (&MAIN_INFLIGHT, &MAIN_INFLIGHT_PEAK)
    };
    bump_inflight(inflight_slot, peak_slot);

    let send_result = if is_preview {
        PREVIEW_IPC_TX
            .send((request.clone(), resp_tx))
            .map_err(|_| "preview IPC thread dead")
    } else {
        MAIN_IPC_TX
            .send((request.clone(), resp_tx))
            .map_err(|_| "main IPC thread dead")
    };
    if let Err(e) = send_result {
        inflight_slot.fetch_sub(1, Ordering::Relaxed);
        return Err(e.to_string());
    }

    let result = if is_preview {
        // Preview decodes (waveform/spectrogram) get a 60 s timeout so a hung JUCE decoder
        // on a corrupt file doesn't block the waveform prefetch loop forever overnight.
        resp_rx
            .recv_timeout(Duration::from_secs(60))
            .map_err(|e| match e {
                std::sync::mpsc::RecvTimeoutError::Timeout => {
                    log_ipc_failure("preview IPC response timed out after 60 s", None);
                    "preview IPC response timed out".to_string()
                }
                std::sync::mpsc::RecvTimeoutError::Disconnected => {
                    "IPC response channel closed".to_string()
                }
            })
            .and_then(|r| r)
    } else {
        // Main engine (playback, transport) — no timeout; transport commands must not be dropped.
        resp_rx
            .recv()
            .map_err(|_| "IPC response channel closed".to_string())
            .and_then(|r| r)
    };

    let elapsed_us = started.elapsed().as_micros() as u64;
    inflight_slot.fetch_sub(1, Ordering::Relaxed);
    if is_preview {
        PREVIEW_COUNT.fetch_add(1, Ordering::Relaxed);
        PREVIEW_TOTAL_US.fetch_add(elapsed_us, Ordering::Relaxed);
        record_max(&PREVIEW_MAX_US, elapsed_us);
    } else {
        MAIN_COUNT.fetch_add(1, Ordering::Relaxed);
        MAIN_TOTAL_US.fetch_add(elapsed_us, Ordering::Relaxed);
        record_max(&MAIN_MAX_US, elapsed_us);
    }
    result
}

/// Async wrapper that runs the blocking IPC wait on the tokio blocking pool.
/// Uses `spawn_blocking` so the tokio runtime stays unblocked **without** allocating
/// a fresh OS thread per call — the 30 Hz `playback_status` poll alone can fire
/// hundreds of thousands of these per session, so reusing the pooled threads matters.
pub async fn async_dedicated_audio_engine_request(
    request: serde_json::Value,
) -> Result<serde_json::Value, String> {
    tokio::task::spawn_blocking(move || dedicated_audio_engine_request(&request))
        .await
        .map_err(|e| format!("async IPC join: {e}"))?
}

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
    /// Recent stderr from the AudioEngine (crash/assert output) for `app.log` when IPC fails.
    stderr_tail: Arc<Mutex<String>>,
    /// Which binary we spawned; must respawn if [`resolve_audio_engine_binary`] starts returning a different path.
    binary_path: PathBuf,
    /// `metadata().modified()` + `len()` when spawned — same path can be overwritten when the AudioEngine is rebuilt.
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

/// Log host-side IPC failure to `app.log`, appending recent AudioEngine stderr when available.
fn log_ipc_failure(msg: impl Into<String>, tail: Option<&Arc<Mutex<String>>>) {
    let msg = msg.into();
    let suffix = tail.map(format_stderr_suffix).unwrap_or_default();
    crate::write_app_log(format!("audio-engine IPC error: {msg}{suffix}"));
}

static ENGINE_CHILD: Mutex<Option<EngineChild>> = Mutex::new(None);
/// Second process: visual preview only (`waveform_preview` / `spectrogram_preview`). Same binary as main.
static PREVIEW_ENGINE_CHILD: Mutex<Option<EngineChild>> = Mutex::new(None);
/// OS PID of the current AudioEngine (`Child::id`), or `0` if none. Used for kill-on-exit without
/// waiting on [`ENGINE_CHILD`] (see module comment).
static ENGINE_CHILD_PID: AtomicU32 = AtomicU32::new(0);
static PREVIEW_ENGINE_CHILD_PID: AtomicU32 = AtomicU32::new(0);

/// Host-side `playback_status` poll for library playback EOF when the WebView defers its **`setInterval`**
/// poll (**`isUiIdleHeavyCpu`** — hidden, unfocused, minimized; see `syncEnginePlaybackEofWatchdog` in
/// `audio-engine.js`). Throttled background timers can miss EOF for autoplay-next; this thread emits
/// `audio-engine-playback-eof` on the same `loaded && eof` rising edge as the UI poll.
static EOF_WATCHDOG_ACTIVE: AtomicBool = AtomicBool::new(false);

/// Only runs while the WebView poll is deferred — ~1 Hz keeps idle CPU low; foreground-focused playback
/// does not run this thread.
const EOF_WATCHDOG_POLL_MS: u64 = 1000;


#[inline]
fn record_engine_pid(child: &Child) {
    ENGINE_CHILD_PID.store(child.id(), Ordering::SeqCst);
}

#[inline]
fn clear_engine_pid() {
    ENGINE_CHILD_PID.store(0, Ordering::SeqCst);
}

#[inline]
fn record_preview_engine_pid(child: &Child) {
    PREVIEW_ENGINE_CHILD_PID.store(child.id(), Ordering::SeqCst);
}

#[inline]
fn clear_preview_engine_pid() {
    PREVIEW_ENGINE_CHILD_PID.store(0, Ordering::SeqCst);
}

/// OS PID of the current AudioEngine subprocess (`Child::id`), or `0` if none has been spawned yet.
#[inline]
pub fn audio_engine_child_pid() -> u32 {
    ENGINE_CHILD_PID.load(Ordering::SeqCst)
}

/// OS PID of the current preview AudioEngine subprocess, or `0` if none.
#[inline]
pub fn preview_engine_child_pid() -> u32 {
    PREVIEW_ENGINE_CHILD_PID.load(Ordering::SeqCst)
}

// ── IPC health metrics ──
//
// Wraps `dedicated_audio_engine_request` to track per-channel inflight depth, RTT, and counts.
// The `lib.rs` health sampler drains these every 30 s and writes one HEALTH line to `app.log`.
// Without this, progressive slowdown (queue backpressure behind a slow `playback_load`,
// engine RSS bloat, etc.) is invisible until the user notices stutter.

static MAIN_INFLIGHT: AtomicUsize = AtomicUsize::new(0);
static MAIN_INFLIGHT_PEAK: AtomicUsize = AtomicUsize::new(0);
static MAIN_COUNT: AtomicU64 = AtomicU64::new(0);
static MAIN_TOTAL_US: AtomicU64 = AtomicU64::new(0);
static MAIN_MAX_US: AtomicU64 = AtomicU64::new(0);
static PREVIEW_INFLIGHT: AtomicUsize = AtomicUsize::new(0);
static PREVIEW_INFLIGHT_PEAK: AtomicUsize = AtomicUsize::new(0);
static PREVIEW_COUNT: AtomicU64 = AtomicU64::new(0);
static PREVIEW_TOTAL_US: AtomicU64 = AtomicU64::new(0);
static PREVIEW_MAX_US: AtomicU64 = AtomicU64::new(0);

#[inline]
fn bump_inflight(cur: &AtomicUsize, peak: &AtomicUsize) -> usize {
    let new = cur.fetch_add(1, Ordering::Relaxed) + 1;
    let mut p = peak.load(Ordering::Relaxed);
    while new > p {
        match peak.compare_exchange_weak(p, new, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => break,
            Err(actual) => p = actual,
        }
    }
    new
}

#[inline]
fn record_max(slot: &AtomicU64, val: u64) {
    let mut cur = slot.load(Ordering::Relaxed);
    while val > cur {
        match slot.compare_exchange_weak(cur, val, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => break,
            Err(actual) => cur = actual,
        }
    }
}

pub struct IpcMetricsSnapshot {
    pub main_count: u64,
    pub main_total_us: u64,
    pub main_max_us: u64,
    pub main_peak_inflight: usize,
    pub main_inflight_now: usize,
    pub preview_count: u64,
    pub preview_total_us: u64,
    pub preview_max_us: u64,
    pub preview_peak_inflight: usize,
    pub preview_inflight_now: usize,
}

/// Atomically read + reset accumulating counters; current inflight depth is read but NOT reset.
/// Peak inflight resets to the current depth so the next window's peak measures only that window.
pub fn drain_ipc_metrics() -> IpcMetricsSnapshot {
    let main_now = MAIN_INFLIGHT.load(Ordering::Relaxed);
    let preview_now = PREVIEW_INFLIGHT.load(Ordering::Relaxed);
    IpcMetricsSnapshot {
        main_count: MAIN_COUNT.swap(0, Ordering::Relaxed),
        main_total_us: MAIN_TOTAL_US.swap(0, Ordering::Relaxed),
        main_max_us: MAIN_MAX_US.swap(0, Ordering::Relaxed),
        main_peak_inflight: MAIN_INFLIGHT_PEAK.swap(main_now, Ordering::Relaxed),
        main_inflight_now: main_now,
        preview_count: PREVIEW_COUNT.swap(0, Ordering::Relaxed),
        preview_total_us: PREVIEW_TOTAL_US.swap(0, Ordering::Relaxed),
        preview_max_us: PREVIEW_MAX_US.swap(0, Ordering::Relaxed),
        preview_peak_inflight: PREVIEW_INFLIGHT_PEAK.swap(preview_now, Ordering::Relaxed),
        preview_inflight_now: preview_now,
    }
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

fn take_and_reap_preview_engine_child(guard: &mut Option<EngineChild>) {
    if let Some(mut eng) = guard.take() {
        clear_preview_engine_pid();
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
/// Prefer `audio-engine-artifacts/debug|release` (or legacy `target/debug|release`) found by walking **up** from [`std::env::current_exe`].
/// That covers `pnpm dev` when the app runs inside a macOS **bundle** (`…/target/debug/bundle/…/Contents/MacOS/audio-haxor`)
/// where the sibling `audio-engine` can be stale, while the real AudioEngine from `beforeDevCommand` lives
/// at the workspace `audio-engine-artifacts/<profile>/audio-engine`. Also works when `CARGO_TARGET_DIR` is non-default
/// (compile-time `CARGO_MANIFEST_DIR` alone is insufficient).
///
/// Override for debugging: set `AUDIO_HAXOR_AUDIO_ENGINE` to an absolute path to the AudioEngine binary.
/// Release installs use the sibling next to the main executable when no workspace `target/` is found.
///
/// Tauri [`bundle.externalBin`](https://v2.tauri.app/develop/sidecar/) places **`audio-engine-<host-triple>`**
/// next to the main executable (see `scripts/prepare-audio-engine-audioengine.mjs`). We spawn via
/// [`std::process::Command`], not Tauri’s sidecar API, so we must resolve that suffixed name when
/// plain `audio-engine` is absent (typical in a shipped `.app` under `/Applications`).
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

    // macOS release-bundle layout: nested helper .app at
    // `<bundle>/Contents/MacOS/AudioHaxorEngine.app/Contents/MacOS/audio-engine`
    // (sibling of the main `audio-haxor` binary). The audio-engine sidecar is wrapped in
    // its own minimal .app so [NSBundle mainBundle] resolves to the helper bundle (its
    // own bundle ID + Info.plist) instead of the parent AUDIO_HAXOR.app — required for
    // `audiocomponentd` to authorize the host process for out-of-process AU plugin loading
    // and XPC view-controller delivery (otherwise plugin editor windows render as a
    // permanent 1×1 stub from `_RemoteAUv2ViewFactory`).
    //
    // The helper .app lives in Contents/MacOS/, NOT Contents/Frameworks/. We tried
    // Frameworks/ first; LaunchServices treats `.app` bundles inside Contents/Frameworks/
    // as embedded frameworks rather than registrable apps and audiocomponentd still
    // refuses XPC view delivery. Real DAWs (Bitwig's `Bitwig Plug-in Host ARM64-NEON.app`,
    // Reaper's helpers, etc.) put their helpers in Contents/MacOS/ and that's what works.
    // See `scripts/postbundle-audio-engine-helper.sh` for the bundling pipeline.
    //
    // This check runs first so any release `.app` always uses the helper even if a stale
    // sibling `audio-engine` binary is also present from a previous build.
    #[cfg(target_os = "macos")]
    {
        let helper = dir
            .join("AudioHaxorEngine.app")
            .join("Contents")
            .join("MacOS")
            .join(binary_name());
        if helper.is_file() {
            return Ok(helper);
        }
    }

    if let Some(p) = find_audio_engine_under_target_ancestors(&exe) {
        return Ok(p);
    }

    if sibling.is_file() {
        return Ok(sibling);
    }

    if let Some(triple) = option_env!("AUDIO_HAXOR_TARGET_TRIPLE") {
        let suffixed = if cfg!(target_os = "windows") {
            dir.join(format!("audio-engine-{triple}.exe"))
        } else {
            dir.join(format!("audio-engine-{triple}"))
        };
        if suffixed.is_file() {
            return Ok(suffixed);
        }
    }

    Err(format!(
        "audio engine binary not found (tried helper .app in Contents/MacOS/, workspace walk, `{}`, and `audio-engine-{}` next to host)",
        sibling.display(),
        option_env!("AUDIO_HAXOR_TARGET_TRIPLE").unwrap_or("?")
    ))
}

/// Walk parents of `exe` until `audio-engine-artifacts/…` or legacy `target/…/<binary>` exists (workspace root).
fn find_audio_engine_under_target_ancestors(exe: &Path) -> Option<PathBuf> {
    let mut dir = exe.parent()?;
    for _ in 0..40 {
        let ae_dbg = dir
            .join("audio-engine-artifacts")
            .join("debug")
            .join(binary_name());
        if ae_dbg.is_file() {
            return Some(ae_dbg);
        }
        let ae_rel = dir
            .join("audio-engine-artifacts")
            .join("release")
            .join(binary_name());
        if ae_rel.is_file() {
            return Some(ae_rel);
        }
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

fn spawn_engine_child(path: &Path, preview_only_process: bool) -> Result<EngineChild, String> {
    let identity = std::fs::metadata(path).ok().map(|m| {
        (
            m.modified().unwrap_or(SystemTime::UNIX_EPOCH),
            m.len(),
        )
    });
    let data_dir = crate::history::get_data_dir();
    let engine_log = data_dir.join("engine.log");
    let app_log = data_dir.join("app.log");
    let scan_timeout_sec = crate::history::get_preference("pluginScanTimeoutSec")
        .and_then(|v| v.as_u64())
        .unwrap_or(30);
    let mut cmd = Command::new(path);
    cmd.env("AUDIO_HAXOR_ENGINE_LOG", engine_log.as_os_str())
        .env("AUDIO_HAXOR_APP_LOG", app_log.as_os_str())
        .env("AUDIO_HAXOR_PARENT_PID", format!("{}", std::process::id()))
        .env(
            "AUDIO_HAXOR_PLUGIN_SCAN_TIMEOUT_SEC",
            scan_timeout_sec.to_string(),
        );

    /* Opt-in libmalloc heap debugging for diagnosing plugin-chain corruption crashes. Off by
     * default (significant allocation overhead + massive engine.log growth from stack logs),
     * on when `AUDIO_HAXOR_ENGINE_MALLOC_DEBUG=1` is set in the user's environment.
     *
     * - `MallocGuardEdges=1` places unmapped guard pages around large allocations so a write
     *   past the end of a buffer SIGSEGVs at the write site, not when the allocator's
     *   freelist is later walked by an unrelated call.
     * - `MallocScribble=1` fills freed memory with `0x55` so use-after-free derefs crash
     *   loudly on the next read of the UAF pointer instead of silently seeing stale data.
     * - `MallocStackLogging=1` + `MallocStackLoggingNoCompact=1` record every malloc/free
     *   call site to a temp file — `malloc_history <pid> <addr>` then prints the allocation
     *   + free backtrace for any address the crash handler logs. Critical for tracing
     *   corruption caught by the above detectors back to the code that wrote the bad value.
     *
     * Do NOT enable unconditionally: each of these has measurable impact. Stack logging
     * especially can make the engine 3–5× slower and consume gigabytes. Set the env var
     * explicitly when trying to reproduce a crash, then unset. */
    if std::env::var_os("AUDIO_HAXOR_ENGINE_MALLOC_DEBUG").is_some() {
        cmd.env("MallocGuardEdges", "1")
            .env("MallocScribble", "1")
            .env("MallocStackLogging", "1")
            .env("MallocStackLoggingNoCompact", "1");
    }

    let mut child = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            log_ipc_failure(format!("failed to spawn {}: {e}", path.display()), None);
            format!("spawn {}: {e}", path.display())
        })?;
    let stdin = child
        .stdin
        .take()
        .ok_or_else(|| "audio-engine: no stdin".to_string())?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "audio-engine: no stdout".to_string())?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| "audio-engine: no stderr".to_string())?;

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
    if preview_only_process {
        record_preview_engine_pid(&child);
    } else {
        record_engine_pid(&child);
    }
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
    let disk_identity = std::fs::metadata(path).ok().map(|m| {
        (
            m.modified().unwrap_or(SystemTime::UNIX_EPOCH),
            m.len(),
        )
    });
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
            take_and_reap_engine_child(&mut guard);
        }
        *guard = Some(spawn_engine_child(path, false)?);
    }
    Ok(())
}

fn ensure_preview_engine_child(path: &Path) -> Result<(), String> {
    let mut guard = PREVIEW_ENGINE_CHILD
        .lock()
        .map_err(|_| "audio-engine preview child mutex poisoned")?;
    let disk_identity = std::fs::metadata(path).ok().map(|m| {
        (
            m.modified().unwrap_or(SystemTime::UNIX_EPOCH),
            m.len(),
        )
    });
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
            take_and_reap_preview_engine_child(&mut guard);
        }
        *guard = Some(spawn_engine_child(path, true)?);
    }
    Ok(())
}

/// Drop the long-lived `audio-engine` child so the next IPC spawns a fresh process.
/// Use after a crash or when the AudioEngine stops responding.
///
/// Returns immediately after sending SIGKILL so the UI / toast is not blocked by mutex cleanup
/// (which can take many seconds if an IPC thread still holds [`ENGINE_CHILD`]). Reaping runs on
/// a background thread; the next `spawn_audio_engine_request` will spawn a fresh child if needed.
pub fn restart_audio_engine_child() -> Result<(), String> {
    audio_engine_eof_watchdog_stop();
    let pid = ENGINE_CHILD_PID.swap(0, Ordering::SeqCst);
    if pid != 0 {
        kill_pid_raw(pid);
    }
    let pidp = PREVIEW_ENGINE_CHILD_PID.swap(0, Ordering::SeqCst);
    if pidp != 0 {
        kill_pid_raw(pidp);
    }
    std::thread::spawn(|| {
        let reaped_main = clear_engine_slot_after_os_kill();
        let reaped_prev = clear_preview_engine_slot_after_os_kill();
        if reaped_main && reaped_prev {
            crate::write_app_log(
                "audio-engine: AudioEngine + preview processes restarted (user request)".to_string(),
            );
        } else {
            log_ipc_failure(
                "ENGINE_CHILD / PREVIEW_ENGINE_CHILD mutex not acquired after OS kill (timeout); next IPC may respawn",
                None,
            );
        }
    });
    Ok(())
}

/// Kill only the **preview** `audio-engine` child so the next preview IPC spawns a fresh process.
/// Used by the waveform prefetch loop to periodically release JUCE decoder memory that accumulates
/// over thousands of file decodes. Does **not** touch the main engine (playback stays live).
///
/// Synchronous: waits up to 30 s for the mutex (the `ae-preview-ipc` thread may hold it while
/// blocked on `read_line`; SIGKILL makes that return quickly).
pub fn restart_preview_engine_child() {
    let pid = PREVIEW_ENGINE_CHILD_PID.swap(0, Ordering::SeqCst);
    if pid != 0 {
        kill_pid_raw(pid);
    }
    if clear_preview_engine_slot_after_os_kill() {
        crate::write_app_log(
            "audio-engine: preview process recycled (periodic memory relief)".to_string(),
        );
    }
}

/// Reap `Child` handles after the OS process is gone. Never uses blocking `Mutex::lock()`.
/// Returns `true` if the slot was cleared; `false` if we gave up waiting (~30s).
fn clear_engine_slot_after_os_kill() -> bool {
    const MAX_ITERS: u32 = 6000;
    for _ in 0..MAX_ITERS {
        if let Ok(mut g) = ENGINE_CHILD.try_lock() {
            take_and_reap_engine_child(&mut g);
            return true;
        }
        thread::sleep(Duration::from_millis(5));
    }
    false
}

fn clear_preview_engine_slot_after_os_kill() -> bool {
    const MAX_ITERS: u32 = 6000;
    for _ in 0..MAX_ITERS {
        if let Ok(mut g) = PREVIEW_ENGINE_CHILD.try_lock() {
            take_and_reap_preview_engine_child(&mut g);
            return true;
        }
        thread::sleep(Duration::from_millis(5));
    }
    false
}

/// Kill the JUCE AudioEngine when the host app exits. Idempotent (safe if no child was spawned).
pub fn shutdown_audio_engine_child() -> Result<(), String> {
    audio_engine_eof_watchdog_stop();
    let pid = ENGINE_CHILD_PID.swap(0, Ordering::SeqCst);
    if pid != 0 {
        kill_pid_raw(pid);
    }
    let pidp = PREVIEW_ENGINE_CHILD_PID.swap(0, Ordering::SeqCst);
    if pidp != 0 {
        kill_pid_raw(pidp);
    }
    let _ = clear_engine_slot_after_os_kill();
    let _ = clear_preview_engine_slot_after_os_kill();
    crate::write_app_log("audio-engine: AudioEngine + preview processes terminated (app shutdown)".to_string());
    Ok(())
}

/// Start a background thread that polls `playback_status` every [`EOF_WATCHDOG_POLL_MS`] and emits
/// `audio-engine-playback-eof` when `loaded && eof` transitions to true (same edge `audio-engine.js`
/// uses for autoplay next).
/// Idempotent — if already running, returns immediately.
pub fn audio_engine_eof_watchdog_start(app: AppHandle) {
    if EOF_WATCHDOG_ACTIVE.swap(true, Ordering::SeqCst) {
        return;
    }
    thread::spawn(move || {
        let mut prev_eof = false;
        while EOF_WATCHDOG_ACTIVE.load(Ordering::SeqCst) {
            thread::sleep(Duration::from_millis(EOF_WATCHDOG_POLL_MS));
            if !EOF_WATCHDOG_ACTIVE.load(Ordering::SeqCst) {
                break;
            }
            // Use dedicated IPC thread to avoid mutex contention with playback commands
            let v = match dedicated_audio_engine_request(
                &serde_json::json!({ "cmd": "playback_status" }),
            ) {
                Ok(v) => v,
                Err(_) => {
                    prev_eof = false;
                    continue;
                }
            };
            let loaded = v.get("loaded").and_then(|v| v.as_bool()).unwrap_or(false);
            let eof = v.get("eof").and_then(|v| v.as_bool()).unwrap_or(false);
            let at_eof = loaded && eof;
            if at_eof && !prev_eof {
                let _ = app.emit("audio-engine-playback-eof", serde_json::Value::Null);
            }
            prev_eof = at_eof;
        }
    });
}

/// Stop the host EOF poll (e.g. when library playback polling stops or the engine restarts).
pub fn audio_engine_eof_watchdog_stop() {
    EOF_WATCHDOG_ACTIVE.store(false, Ordering::SeqCst);
}

/// Run one request against the audio-engine subprocess (stdin / stdout JSON lines).
pub fn spawn_audio_engine_request(
    request: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    spawn_audio_engine_request_at(request)
}

/// Tauri may pass `{ "request": { "cmd": ... } }` from `invoke(..., { request: payload })`; unwrap so
/// stdin matches the audio-engine line protocol. If the top-level object already has `cmd`, it is
/// left unchanged (even when `request` is also present).
pub(crate) fn normalize_ipc_request_payload(v: &serde_json::Value) -> serde_json::Value {
    if let Some(obj) = v.as_object()
        && !obj.contains_key("cmd")
        && let Some(inner) = obj.get("request")
    {
        return inner.clone();
    }
    v.clone()
}

/// One response is one JSON object/array per line. A linked library may print warnings to **stdout**
/// before the JSON line (e.g. `Warning: thread locking is not implemented`), which would break
/// `serde_json::from_str` on the first `read_line`. Skip lines until one starts with `{` or `[`.
fn read_engine_json_line<R: Read>(stdout: &mut BufReader<R>) -> Result<String, String> {
    const MAX_LINE_READS: usize = 256;
    let mut line = String::new();
    for _ in 0..MAX_LINE_READS {
        line.clear();
        match stdout.read_line(&mut line) {
            Ok(0) => return Err("audio-engine closed stdout".to_string()),
            Ok(_) => {
                let mut s = line.trim();
                if s.starts_with('\u{feff}') {
                    s = s.trim_start_matches('\u{feff}').trim_start();
                }
                if s.is_empty() {
                    continue;
                }
                let first = s.as_bytes().first().copied();
                if first == Some(b'{') || first == Some(b'[') {
                    return Ok(s.to_string());
                }
                continue;
            }
            Err(e) => return Err(format!("audio-engine stdout: {e}")),
        }
    }
    Err("audio-engine stdout: no JSON line (exceeded line read limit)".to_string())
}

/// Visual decode IPC on a **dedicated** `audio-engine` child so a slow `waveform_preview` /
/// `spectrogram_preview` never blocks the main child's stdin (playback, devices, `ping`, …).
fn spawn_preview_engine_request_at(request: &serde_json::Value) -> Result<serde_json::Value, String> {
    let payload = serde_json::to_string(request).map_err(|e| e.to_string())?;

    for attempt in 0..2 {
        let path = resolve_audio_engine_binary().map_err(|e| {
            log_ipc_failure(format!("failed to resolve audio-engine binary: {e}"), None);
            e
        })?;
        ensure_preview_engine_child(&path)?;
        let mut guard = PREVIEW_ENGINE_CHILD
            .lock()
            .map_err(|_| "audio-engine preview child mutex poisoned".to_string())?;
        let eng = guard
            .as_mut()
            .ok_or_else(|| "audio-engine preview child missing".to_string())?;

        if eng
            .stdin
            .write_all(payload.as_bytes())
            .map_err(|e| e.to_string())
            .and_then(|_| {
                eng.stdin
                    .write_all(b"\n")
                    .map_err(|e| format!("audio-engine preview stdin: {e}"))
            })
            .and_then(|_| {
                eng.stdin
                    .flush()
                    .map_err(|e| format!("audio-engine preview stdin: {e}"))
            })
            .is_err()
        {
            let stderr_tail = Arc::clone(&eng.stderr_tail);
            clear_preview_engine_pid();
            *guard = None;
            if attempt == 1 {
                log_ipc_failure("preview engine stdin write failed", Some(&stderr_tail));
                return Err("audio-engine preview stdin write failed".to_string());
            }
            continue;
        }

        match read_engine_json_line(&mut eng.stdout) {
            Ok(json_line) => {
                let v: serde_json::Value = match serde_json::from_str(&json_line) {
                    Ok(v) => v,
                    Err(e) => {
                        let stderr_tail = Arc::clone(&eng.stderr_tail);
                        log_ipc_failure(
                            format!("preview engine invalid JSON on stdout: {e}; line={json_line:?}"),
                            Some(&stderr_tail),
                        );
                        return Err(format!("audio-engine preview JSON: {e}: {json_line}"));
                    }
                };
                if attempt == 0
                    && let Some(err) = v.get("error").and_then(|e| e.as_str())
                        && err.to_ascii_lowercase().contains("unknown cmd") {
                            clear_preview_engine_pid();
                            *guard = None;
                            continue;
                        }
                return Ok(v);
            }
            Err(e) => {
                let stderr_tail = Arc::clone(&eng.stderr_tail);
                let is_eof = e == "audio-engine closed stdout";
                clear_preview_engine_pid();
                *guard = None;
                if attempt == 1 {
                    if is_eof {
                        log_ipc_failure(
                            "AudioEngine preview process closed stdout (exited or crashed)",
                            Some(&stderr_tail),
                        );
                    } else {
                        log_ipc_failure(format!("preview engine stdout read: {e}"), Some(&stderr_tail));
                    }
                    return Err(e);
                }
            }
        }
    }
    log_ipc_failure("preview engine request failed after retry", None);
    Err("audio-engine preview request failed after retry".to_string())
}

/// Direct main engine request — no routing, used by dedicated IPC thread.
fn do_main_engine_request(request: &serde_json::Value) -> Result<serde_json::Value, String> {
    let (effective_request, _transcoded_temp) =
        crate::waveform_container_extract::rewrite_visual_preview_for_juce(request);
    let payload = serde_json::to_string(&effective_request).map_err(|e| e.to_string())?;

    for attempt in 0..2 {
        let path = resolve_audio_engine_binary().map_err(|e| {
            log_ipc_failure(format!("failed to resolve audio-engine binary: {e}"), None);
            e
        })?;
        ensure_engine_child(&path)?;
        let mut guard = ENGINE_CHILD
            .lock()
            .map_err(|_| "audio-engine child mutex poisoned".to_string())?;
        let eng = guard
            .as_mut()
            .ok_or_else(|| "audio-engine child missing".to_string())?;

        if eng
            .stdin
            .write_all(payload.as_bytes())
            .map_err(|e| e.to_string())
            .and_then(|_| {
                eng.stdin
                    .write_all(b"\n")
                    .map_err(|e| format!("audio-engine stdin: {e}"))
            })
            .and_then(|_| {
                eng.stdin
                    .flush()
                    .map_err(|e| format!("audio-engine stdin: {e}"))
            })
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

        match read_engine_json_line(&mut eng.stdout) {
            Ok(json_line) => {
                let v: serde_json::Value = match serde_json::from_str(&json_line) {
                    Ok(v) => v,
                    Err(e) => {
                        let stderr_tail = Arc::clone(&eng.stderr_tail);
                        log_ipc_failure(
                            format!("invalid JSON on stdout: {e}; line={json_line:?}"),
                            Some(&stderr_tail),
                        );
                        return Err(format!("audio-engine JSON: {e}: {json_line}"));
                    }
                };
                if attempt == 0
                    && let Some(err) = v.get("error").and_then(|e| e.as_str())
                        && err.to_ascii_lowercase().contains("unknown cmd") {
                            clear_engine_pid();
                            *guard = None;
                            continue;
                        }
                return Ok(v);
            }
            Err(e) => {
                let stderr_tail = Arc::clone(&eng.stderr_tail);
                let is_eof = e == "audio-engine closed stdout";
                clear_engine_pid();
                *guard = None;
                if attempt == 1 {
                    if is_eof {
                        log_ipc_failure(
                            "AudioEngine closed stdout (process exited or crashed)",
                            Some(&stderr_tail),
                        );
                    } else {
                        log_ipc_failure(format!("stdout read: {e}"), Some(&stderr_tail));
                    }
                    return Err(e);
                }
            }
        }
    }
    log_ipc_failure("main engine request failed after retry", None);
    Err("audio-engine request failed after retry".to_string())
}

fn spawn_audio_engine_request_at(request: &serde_json::Value) -> Result<serde_json::Value, String> {
    let (effective_request, _transcoded_temp) =
        crate::waveform_container_extract::rewrite_visual_preview_for_juce(request);
    let cmd = effective_request
        .get("cmd")
        .and_then(|c| c.as_str())
        .unwrap_or("");
    if cmd == "waveform_preview" || cmd == "spectrogram_preview" {
        return spawn_preview_engine_request_at(&effective_request);
    }

    let payload = serde_json::to_string(&effective_request).map_err(|e| e.to_string())?;

    for attempt in 0..2 {
        let path = resolve_audio_engine_binary().map_err(|e| {
            log_ipc_failure(format!("failed to resolve audio-engine binary: {e}"), None);
            e
        })?;
        ensure_engine_child(&path)?;
        let mut guard = ENGINE_CHILD
            .lock()
            .map_err(|_| "audio-engine child mutex poisoned".to_string())?;
        let eng = guard
            .as_mut()
            .ok_or_else(|| "audio-engine child missing".to_string())?;

        if eng
            .stdin
            .write_all(payload.as_bytes())
            .map_err(|e| e.to_string())
            .and_then(|_| {
                eng.stdin
                    .write_all(b"\n")
                    .map_err(|e| format!("audio-engine stdin: {e}"))
            })
            .and_then(|_| {
                eng.stdin
                    .flush()
                    .map_err(|e| format!("audio-engine stdin: {e}"))
            })
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

        match read_engine_json_line(&mut eng.stdout) {
            Ok(json_line) => {
                let v: serde_json::Value = match serde_json::from_str(&json_line) {
                    Ok(v) => v,
                    Err(e) => {
                        let stderr_tail = Arc::clone(&eng.stderr_tail);
                        log_ipc_failure(
                            format!("invalid JSON on stdout: {e}; line={json_line:?}"),
                            Some(&stderr_tail),
                        );
                        return Err(format!("audio-engine JSON: {e}: {json_line}"));
                    }
                };
                // Long-lived child can outlive a fresh `node scripts/build-audio-engine.mjs`; the old process may
                // return `unknown cmd` for verbs added in a newer AudioEngine. Respawn once (see also
                // [`ensure_engine_child`] binary identity). Retry even if `ok` is missing — some
                // older builds only set `error`.
                if attempt == 0
                    && let Some(err) = v.get("error").and_then(|e| e.as_str())
                        && err.to_ascii_lowercase().contains("unknown cmd") {
                            clear_engine_pid();
                            *guard = None;
                            continue;
                        }
                return Ok(v);
            }
            Err(e) => {
                let stderr_tail = Arc::clone(&eng.stderr_tail);
                let is_eof = e == "audio-engine closed stdout";
                clear_engine_pid();
                *guard = None;
                if attempt == 1 {
                    if is_eof {
                        log_ipc_failure(
                            "AudioEngine closed stdout (process exited or crashed)",
                            Some(&stderr_tail),
                        );
                    } else {
                        log_ipc_failure(format!("stdout read: {e}"), Some(&stderr_tail));
                    }
                    return Err(e);
                }
            }
        }
    }
    log_ipc_failure("request failed after retry", None);
    Err("audio-engine request failed after retry".to_string())
}

#[cfg(test)]
mod tests {
    use std::io::{BufReader, Cursor};

    use super::{normalize_ipc_request_payload, read_engine_json_line};
    use serde_json::json;

    #[test]
    fn parse_engine_response_line() {
        let s = r#"{"ok":true,"version":"1.0.0"}"#;
        let v: serde_json::Value = serde_json::from_str(s).unwrap();
        assert_eq!(v["ok"], true);
    }

    #[test]
    fn audio_engine_stub_default_and_json_roundtrip() {
        let s = super::AudioEngineStub::default();
        assert_eq!(s.state, "not_started");
        let v = serde_json::to_value(&s).unwrap();
        let back: super::AudioEngineStub = serde_json::from_value(v).unwrap();
        assert_eq!(back.state, "not_started");
    }

    #[test]
    fn normalize_ipc_request_payload_passes_through_when_cmd_present() {
        let v = json!({ "cmd": "ping", "request": { "cmd": "other" } });
        let n = normalize_ipc_request_payload(&v);
        assert_eq!(n, v);
    }

    #[test]
    fn normalize_ipc_request_payload_unwraps_tauri_request_wrapper() {
        let v = json!({ "request": { "cmd": "ping", "x": 1 } });
        let n = normalize_ipc_request_payload(&v);
        assert_eq!(n, json!({ "cmd": "ping", "x": 1 }));
    }

    #[test]
    fn normalize_ipc_request_payload_unwraps_when_no_top_level_cmd() {
        let v = json!({ "foo": true, "request": { "cmd": "engine_state" } });
        let n = normalize_ipc_request_payload(&v);
        assert_eq!(n, json!({ "cmd": "engine_state" }));
    }

    #[test]
    fn normalize_ipc_request_payload_leaves_non_object_unchanged() {
        let v = json!("literal");
        assert_eq!(normalize_ipc_request_payload(&v), v);
    }

    #[test]
    fn normalize_ipc_request_payload_empty_object_unchanged() {
        let v = json!({});
        assert_eq!(normalize_ipc_request_payload(&v), json!({}));
    }

    #[test]
    fn normalize_ipc_request_payload_does_not_unwrap_when_cmd_key_is_empty_string() {
        let v = json!({ "cmd": "", "request": { "cmd": "ping" } });
        let n = normalize_ipc_request_payload(&v);
        assert_eq!(n, v);
    }

    #[test]
    fn normalize_ipc_request_payload_does_not_unwrap_when_cmd_is_null() {
        let v = json!({ "cmd": null, "request": { "cmd": "ping" } });
        let n = normalize_ipc_request_payload(&v);
        assert_eq!(n, v);
    }

    #[test]
    fn normalize_ipc_request_payload_unwraps_when_only_request_has_cmd() {
        let v = json!({ "request": { "cmd": "playback_status" } });
        let n = normalize_ipc_request_payload(&v);
        assert_eq!(n, json!({ "cmd": "playback_status" }));
    }

    #[test]
    fn read_engine_json_line_skips_leading_warning_on_stdout() {
        let data = b"Warning: thread locking is not implemented\n{\"ok\":true,\"x\":1}\n";
        let mut r = BufReader::new(Cursor::new(&data[..]));
        let line = read_engine_json_line(&mut r).unwrap();
        assert_eq!(line, r#"{"ok":true,"x":1}"#);
    }

    #[test]
    fn read_engine_json_line_strips_bom() {
        let data = "\u{feff}{\"ok\":false}\n".as_bytes();
        let mut r = BufReader::new(Cursor::new(data));
        let line = read_engine_json_line(&mut r).unwrap();
        assert_eq!(line, r#"{"ok":false}"#);
    }

    #[test]
    fn read_engine_json_line_accepts_json_array_line() {
        let data = b"[1,2,3]\n";
        let mut r = BufReader::new(Cursor::new(&data[..]));
        let line = read_engine_json_line(&mut r).unwrap();
        assert_eq!(line, "[1,2,3]");
    }

    #[test]
    fn read_engine_json_line_trims_leading_whitespace_before_json() {
        let data = b"   {\"a\":1}\n";
        let mut r = BufReader::new(Cursor::new(&data[..]));
        let line = read_engine_json_line(&mut r).unwrap();
        assert_eq!(line, r#"{"a":1}"#);
    }

    #[test]
    fn read_engine_json_line_skips_empty_and_blank_lines() {
        let data = b"  \n\t\nnot json\n{\"ok\":true}\n";
        let mut r = BufReader::new(Cursor::new(&data[..]));
        let line = read_engine_json_line(&mut r).unwrap();
        assert_eq!(line, r#"{"ok":true}"#);
    }

    #[test]
    fn read_engine_json_line_eof_on_empty_stream() {
        let data: &[u8] = b"";
        let mut r = BufReader::new(Cursor::new(data));
        let err = read_engine_json_line(&mut r).unwrap_err();
        assert_eq!(err, "audio-engine closed stdout");
    }

    #[test]
    fn read_engine_json_line_errors_after_max_non_json_lines() {
        let mut s = String::with_capacity(256 * 6 + 32);
        for _ in 0..256 {
            s.push_str("noise\n");
        }
        s.push_str(r#"{"ok":true}"#);
        s.push('\n');
        let mut r = BufReader::new(Cursor::new(s.into_bytes()));
        let err = read_engine_json_line(&mut r).unwrap_err();
        assert!(
            err.contains("exceeded line read limit"),
            "unexpected err: {err}"
        );
    }

    #[test]
    fn read_engine_json_line_multiple_warnings_before_object() {
        let data = b"Warning: a\nWarning: b\n{\"x\":0}\n";
        let mut r = BufReader::new(Cursor::new(&data[..]));
        let line = read_engine_json_line(&mut r).unwrap();
        assert_eq!(line, r#"{"x":0}"#);
    }
}
