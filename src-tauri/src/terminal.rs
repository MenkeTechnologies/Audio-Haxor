//! Embedded PTY terminal — spawns a persistent zsh session with full ANSI support.
//!
//! The frontend drives this via four Tauri commands:
//! - `terminal_spawn`  — create the PTY + reader thread
//! - `terminal_write`  — pipe user keystrokes into the PTY
//! - `terminal_resize` — notify the PTY of a new viewport size
//! - `terminal_kill`   — tear down the session

use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use std::io::{Read, Write};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, State};

/// Managed state for the embedded terminal.
pub struct TerminalState {
    master: Mutex<Option<Box<dyn MasterPty + Send>>>,
    writer: Mutex<Option<Box<dyn Write + Send>>>,
    /// PID of the child shell (for cleanup).
    child_pid: Mutex<Option<u32>>,
}

impl Default for TerminalState {
    fn default() -> Self {
        Self {
            master: Mutex::new(None),
            writer: Mutex::new(None),
            child_pid: Mutex::new(None),
        }
    }
}

/// Spawn a new PTY shell session.  Kills any existing session first.
#[tauri::command]
pub async fn terminal_spawn(
    rows: Option<u16>,
    cols: Option<u16>,
    app: AppHandle,
    state: State<'_, TerminalState>,
) -> Result<(), String> {
    // Tear down previous session
    kill_inner(&state);

    let pty_system = native_pty_system();
    let pair = pty_system
        .openpty(PtySize {
            rows: rows.unwrap_or(24),
            cols: cols.unwrap_or(80),
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|e| e.to_string())?;

    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".into());
    let mut cmd = CommandBuilder::new(&shell);
    cmd.arg("-l"); // login shell — loads user's .zprofile / .zshrc

    // Inherit HOME so dotfiles are found.
    if let Ok(home) = std::env::var("HOME") {
        cmd.env("HOME", &home);
        cmd.cwd(&home);
    }
    // Set TERM so curses apps render correctly.
    cmd.env("TERM", "xterm-256color");

    let child = pair.slave.spawn_command(cmd).map_err(|e| e.to_string())?;

    // Store child PID.
    {
        let pid = child.process_id();
        *state.child_pid.lock().unwrap_or_else(|e| e.into_inner()) = pid;
    }

    // Writer half — frontend sends keystrokes here.
    let writer = pair
        .master
        .take_writer()
        .map_err(|e| e.to_string())?;

    // Reader half — stream PTY output to the frontend.
    let mut reader = pair
        .master
        .try_clone_reader()
        .map_err(|e| e.to_string())?;

    *state.writer.lock().unwrap_or_else(|e| e.into_inner()) = Some(writer);
    *state.master.lock().unwrap_or_else(|e| e.into_inner()) = Some(pair.master);

    // Drop the slave — we only need the master side.
    drop(pair.slave);

    // Background thread: read PTY output → emit events.
    let app2 = app.clone();
    std::thread::Builder::new()
        .name("terminal-reader".into())
        .spawn(move || {
            let mut buf = [0u8; 4096];
            let mut carry = Vec::new(); // incomplete UTF-8 tail from previous read
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        // Prepend any leftover bytes from the previous read.
                        carry.extend_from_slice(&buf[..n]);

                        // Find the last valid UTF-8 boundary. If the tail is an
                        // incomplete multi-byte sequence, hold it for the next read
                        // instead of replacing it with U+FFFD.
                        let valid_up_to = match std::str::from_utf8(&carry) {
                            Ok(_) => carry.len(),
                            Err(e) => {
                                let valid = e.valid_up_to();
                                // If the error is at the very end, it's likely a
                                // split multi-byte char — carry the tail bytes.
                                // If it's mid-stream, include up to the error and
                                // let from_utf8_lossy handle the isolated bad byte.
                                if valid + 4 >= carry.len() {
                                    valid
                                } else {
                                    carry.len()
                                }
                            }
                        };

                        if valid_up_to > 0 {
                            let text = String::from_utf8_lossy(&carry[..valid_up_to]);
                            let _ = app2.emit("terminal-output", text.as_ref());
                        }

                        // Keep only the incomplete tail bytes for next iteration.
                        carry = carry[valid_up_to..].to_vec();
                    }
                    Err(_) => break,
                }
            }
            // Shell exited — notify frontend.
            let _ = app2.emit("terminal-exit", ());
        })
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Write raw bytes (user keystrokes) into the PTY.
#[tauri::command]
pub fn terminal_write(data: String, state: State<'_, TerminalState>) -> Result<(), String> {
    let mut guard = state.writer.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(ref mut w) = *guard {
        w.write_all(data.as_bytes()).map_err(|e| e.to_string())?;
        w.flush().map_err(|e| e.to_string())?;
        Ok(())
    } else {
        Err("No terminal session".into())
    }
}

/// Notify the PTY of a viewport resize.
#[tauri::command]
pub fn terminal_resize(
    rows: u16,
    cols: u16,
    state: State<'_, TerminalState>,
) -> Result<(), String> {
    let guard = state.master.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(ref master) = *guard {
        master
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| e.to_string())
    } else {
        Err("No terminal session".into())
    }
}

/// Kill the terminal session.
#[tauri::command]
pub fn terminal_kill(state: State<'_, TerminalState>) -> Result<(), String> {
    kill_inner(&state);
    Ok(())
}

fn kill_inner(state: &TerminalState) {
    // Drop writer first — this closes the PTY stdin and lets the reader thread exit.
    *state.writer.lock().unwrap_or_else(|e| e.into_inner()) = None;

    // Kill child process if still alive.
    if let Some(pid) = state.child_pid.lock().unwrap_or_else(|e| e.into_inner()).take() {
        #[cfg(unix)]
        unsafe {
            libc::kill(pid as i32, libc::SIGKILL);
        }
    }

    // Drop master — closes the PTY fd.
    *state.master.lock().unwrap_or_else(|e| e.into_inner()) = None;
}
