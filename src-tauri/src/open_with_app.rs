//! Cross-platform **open file with named application** for context menus and the command palette.
//!
//! macOS uses `open -a`. Windows and Linux resolve the same human-readable labels the UI uses
//! (often macOS-oriented) to real executables, then spawn `program path/to/file`.

use std::path::Path;

#[cfg(any(target_os = "windows", target_os = "linux"))]
use std::path::PathBuf;

/// Opens an existing file with a named application.
pub fn open_with_application(file_path: &Path, app_name: &str) -> Result<(), String> {
    let app = app_name.trim();
    if app.is_empty() {
        return Err("application name is empty".into());
    }
    if !file_path.exists() {
        return Err(format!("File not found: {}", file_path.display()));
    }

    #[cfg(target_os = "macos")]
    {
        return open_macos(file_path, app);
    }
    #[cfg(target_os = "windows")]
    {
        return open_windows(file_path, app);
    }
    #[cfg(target_os = "linux")]
    {
        return open_linux(file_path, app);
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        let _ = (file_path, app);
        Err("open_with_app is not supported on this platform".into())
    }
}

#[cfg(target_os = "macos")]
fn open_macos(file_path: &Path, app_name: &str) -> Result<(), String> {
    let output = std::process::Command::new("open")
        .args(["-a", app_name, &file_path.to_string_lossy()])
        .output()
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "Could not open with {}: {}",
            app_name,
            stderr.trim()
        ));
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn open_windows(file_path: &Path, app_name: &str) -> Result<(), String> {
    if matches!(app_name, "GarageBand" | "Logic Pro") {
        return Err(format!("{app_name} is only available on macOS"));
    }
    if app_name == "Preview" {
        return opener::open(file_path).map_err(|e| e.to_string());
    }
    let exe = resolve_windows_executable(app_name)?;
    std::process::Command::new(&exe)
        .arg(file_path)
        .spawn()
        .map_err(|e| format!("failed to start {}: {e}", exe.display()))?;
    Ok(())
}

#[cfg(target_os = "windows")]
fn resolve_windows_executable(app_name: &str) -> Result<PathBuf, String> {
    let p = Path::new(app_name);
    if p.is_file() {
        return Ok(p.to_path_buf());
    }
    if app_name.contains('/') || app_name.contains('\\') {
        if p.exists() {
            return Ok(p.to_path_buf());
        }
        return Err(format!("application path not found: {app_name}"));
    }

    match app_name {
        "TextEdit" => {
            return where_win("notepad.exe")
                .ok_or_else(|| "notepad.exe not found (expected on Windows)".into());
        }
        "Music" => {
            return first_existing_file(&[r"C:\Program Files\Windows Media Player\wmplayer.exe"])
                .or_else(|_| vlc_windows());
        }
        "QuickTime Player" => return vlc_windows(),
        "Audacity" => {
            return where_win("audacity.exe").ok_or_else(|| {
                "Audacity not found in PATH (install Audacity or pass the full path to Audacity.exe)"
                    .into()
            });
        }
        "Ableton Live 12 Standard" | "Ableton Live 11 Suite" | "Ableton Live 12 Suite" => {
            if let Some(exe) = find_ableton_live_exe() {
                return Ok(exe);
            }
            return Err(
                "Ableton Live executable not found under Program Files or ProgramData".into(),
            );
        }
        "Adobe Acrobat" => return find_adobe_acrobat_windows(),
        _ => {}
    }

    if let Some(pb) = where_win(app_name) {
        return Ok(pb);
    }
    if let Some(pb) = where_win(&format!("{app_name}.exe")) {
        return Ok(pb);
    }

    Err(format!(
        "Could not resolve application {app_name:?}. Install the app, add it to PATH, or pass the full path to the .exe."
    ))
}

#[cfg(target_os = "windows")]
fn where_win(name: &str) -> Option<PathBuf> {
    let output = std::process::Command::new("where")
        .arg(name)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let line = stdout.lines().next()?.trim();
    if line.is_empty() {
        return None;
    }
    let pb = PathBuf::from(line);
    pb.is_file().then_some(pb)
}

#[cfg(target_os = "windows")]
fn first_existing_file(paths: &[&str]) -> Result<PathBuf, String> {
    for s in paths {
        let p = PathBuf::from(s);
        if p.is_file() {
            return Ok(p);
        }
    }
    Err("no candidate executable found".into())
}

#[cfg(target_os = "windows")]
fn vlc_windows() -> Result<PathBuf, String> {
    where_win("vlc.exe")
        .or_else(|| {
            first_existing_file(&[
                r"C:\Program Files\VideoLAN\VLC\vlc.exe",
                r"C:\Program Files (x86)\VideoLAN\VLC\vlc.exe",
            ])
            .ok()
        })
        .ok_or_else(|| "VLC not found (install VLC or map QuickTime to another player)".into())
}

#[cfg(target_os = "windows")]
fn find_ableton_live_exe() -> Option<PathBuf> {
    let roots = [
        std::env::var_os("ProgramData").map(PathBuf::from),
        std::env::var_os("ProgramFiles").map(PathBuf::from),
    ];
    for opt in roots.into_iter().flatten() {
        let ableton = opt.join("Ableton");
        let Ok(entries) = std::fs::read_dir(&ableton) else {
            continue;
        };
        for e in entries.flatten() {
            let dir = e.path();
            if !dir.is_dir() {
                continue;
            }
            let name = dir.file_name()?.to_string_lossy();
            if !name.contains("Live") {
                continue;
            }
            let program = dir.join("Program");
            let Ok(files) = std::fs::read_dir(&program) else {
                continue;
            };
            for f in files.flatten() {
                let path = f.path();
                if path.is_file() && path.extension().is_some_and(|x| x == "exe") {
                    let stem = path.file_name()?.to_string_lossy();
                    if stem.contains("Live") {
                        return Some(path);
                    }
                }
            }
        }
    }
    None
}

#[cfg(target_os = "windows")]
fn find_adobe_acrobat_windows() -> Result<PathBuf, String> {
    let roots = [
        std::env::var_os("ProgramFiles").map(PathBuf::from),
        std::env::var_os("ProgramFiles(x86)").map(PathBuf::from),
    ];
    for opt in roots.into_iter().flatten() {
        let adobe = opt.join("Adobe");
        let Ok(entries) = std::fs::read_dir(&adobe) else {
            continue;
        };
        for e in entries.flatten() {
            let dir = e.path();
            if !dir.is_dir() {
                continue;
            }
            let name = dir.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if !name.starts_with("Acrobat") && !name.starts_with("Adobe Acrobat") {
                continue;
            }
            let exe = dir.join("Acrobat").join("AcroRd32.exe");
            if exe.is_file() {
                return Ok(exe);
            }
        }
    }
    where_win("AcroRd32.exe")
        .ok_or_else(|| "Adobe Acrobat Reader not found under Program Files or PATH".into())
}

#[cfg(target_os = "linux")]
fn open_linux(file_path: &Path, app_name: &str) -> Result<(), String> {
    if matches!(app_name, "GarageBand" | "Logic Pro") {
        return Err(format!("{app_name} is only available on macOS"));
    }
    if app_name == "Preview" {
        return opener::open(file_path).map_err(|e| e.to_string());
    }
    let exe = resolve_linux_executable(app_name)?;
    std::process::Command::new(&exe)
        .arg(file_path)
        .spawn()
        .map_err(|e| format!("failed to start {}: {e}", exe.display()))?;
    Ok(())
}

#[cfg(target_os = "linux")]
fn which_linux(candidates: &[&str]) -> Option<PathBuf> {
    for c in candidates {
        let output = std::process::Command::new("which").arg(c).output().ok()?;
        if !output.status.success() {
            continue;
        }
        let stdout = String::from_utf8_lossy(&output.stdout);
        let line = stdout.lines().next()?.trim();
        if line.is_empty() {
            continue;
        }
        let pb = PathBuf::from(line);
        if pb.is_file() {
            return Some(pb);
        }
    }
    None
}

#[cfg(target_os = "linux")]
fn resolve_linux_executable(app_name: &str) -> Result<PathBuf, String> {
    let p = Path::new(app_name);
    if p.is_file() {
        return Ok(p.to_path_buf());
    }
    if app_name.contains('/') {
        if p.exists() {
            return Ok(p.to_path_buf());
        }
        return Err(format!("application path not found: {app_name}"));
    }

    match app_name {
        "TextEdit" => which_linux(&[
            "gnome-text-editor",
            "gedit",
            "kwrite",
            "mousepad",
            "xed",
            "pluma",
            "leafpad",
        ])
        .ok_or_else(|| {
            "No text editor found in PATH (install gedit, gnome-text-editor, or similar)".into()
        }),
        "Music" => which_linux(&["vlc", "mpv", "audacious", "rhythmbox"]).ok_or_else(|| {
            "No music player found in PATH (install vlc or mpv)".into()
        }),
        "QuickTime Player" => which_linux(&["vlc", "mpv", "totem", "celluloid"]).ok_or_else(|| {
            "No video player found in PATH (install vlc or mpv)".into()
        }),
        "Audacity" => which_linux(&["audacity"]).ok_or_else(|| {
            "Audacity not found in PATH (install Audacity or pass the full path)".into()
        }),
        "Ableton Live 12 Standard" | "Ableton Live 11 Suite" | "Ableton Live 12 Suite" => {
            which_linux(&["Live", "live", "ableton"]).ok_or_else(|| {
                "Ableton Live not found in PATH (install Ableton or pass the full path to the Live binary)".into()
            })
        }
        "Adobe Acrobat" => which_linux(&["acroread", "evince", "okular", "atril", "xreader"])
            .ok_or_else(|| {
                "PDF viewer not found in PATH (install evince, okular, or Adobe Reader)".into()
            }),
        _ => {
            if let Some(pb) = which_linux(&[app_name]) {
                return Ok(pb);
            }
            Err(format!(
                "Could not resolve application {app_name:?} in PATH. Pass a full path to the binary."
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_with_errors_on_empty_app_name() {
        let tmp = std::env::temp_dir().join("audio_haxor_open_with_test_file");
        let _ = std::fs::write(&tmp, b"x");
        let r = open_with_application(&tmp, "  ");
        let _ = std::fs::remove_file(&tmp);
        assert!(r.is_err());
    }
}
