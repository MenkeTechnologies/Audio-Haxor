//! System tray / menu bar icon: playback controls, dynamic title + tooltip, popup menu,
//! and (non-Linux) a **WebView popover** styled like macOS Now Playing (no artwork).
use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;
use tauri::image::Image;
use tauri::menu::MenuBuilder;
use tauri::tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};
use tauri::{
    App, AppHandle, Emitter, LogicalSize, Manager, PhysicalPosition, Position, Rect, Size, State,
    Wry,
};

use crate::history;

/// Max characters for the first row of the tray dropdown (macOS truncates visually; keep readable).
const TRAY_MENU_NOW_PLAYING_MAX: usize = 96;

const TRAY_POPOVER_W: u32 = 340;
/// Default height until JS measures `#shell` (`tray_popover_resize`); generous so first paint is not clipped
/// (multi-line title + meta + directory path + progress + volume + speed + transport + padding).
const TRAY_POPOVER_H: u32 = 480;

/// Set `AUDIO_HAXOR_TRAY_DEBUG=1` in the environment to print every successful `tray-popover-state` /
/// `tray-popover-ui-theme` emit to stderr (state includes the ~500 ms host poll). Emit **failures** always log.
fn emit_tray_popover_state(app: &AppHandle<Wry>, emit: &TrayPopoverEmit) {
    let appearance_n = emit.appearance.as_ref().map(|m| m.len()).unwrap_or(0);
    match app.emit_to("tray-popover", "tray-popover-state", emit) {
        Ok(()) => {
            if std::env::var_os("AUDIO_HAXOR_TRAY_DEBUG").is_some() {
                eprintln!(
                    "[tray-popover-host] emit tray-popover-state ok idle={} ui_theme={} appearance_vars={} title_ch={} subtitle_ch={} playing={} elapsed={:.2} total_sec={:?}",
                    emit.idle,
                    emit.ui_theme,
                    appearance_n,
                    emit.title.chars().count(),
                    emit.subtitle.chars().count(),
                    emit.playing,
                    emit.elapsed_sec,
                    emit.total_sec
                );
            }
        }
        Err(e) => {
            eprintln!("[tray-popover-host] emit tray-popover-state FAILED: {e}");
        }
    }
}

/// Lightweight toggle-only emit for shuffle/loop. Does **not** touch `elapsed_sec`/`total_sec`/
/// `playing`, so the tray popover's local rAF progress interpolation keeps running untouched.
/// Re-emitting `tray-popover-state` on a toggle while the main window is minimized was replaying
/// a stale `last_popover_emit.elapsed_sec` (frozen at the last main-window push before suspend),
/// yanking the progress thumb on every shuffle/loop click.
/// Lightweight favorite toggle sync — same rationale as [`emit_tray_popover_shuffle_loop`].
fn emit_tray_popover_favorite(app: &AppHandle<Wry>, favorite_on: bool) {
    let payload = serde_json::json!({ "favorite_on": favorite_on });
    match app.emit_to("tray-popover", "tray-popover-favorite", payload) {
        Ok(()) => {
            if std::env::var_os("AUDIO_HAXOR_TRAY_DEBUG").is_some() {
                eprintln!("[tray-popover-host] emit tray-popover-favorite ok favorite_on={favorite_on}");
            }
        }
        Err(e) => {
            eprintln!("[tray-popover-host] emit tray-popover-favorite FAILED: {e}");
        }
    }
}

fn emit_tray_popover_shuffle_loop(app: &AppHandle<Wry>, shuffle_on: bool, loop_on: bool) {
    let payload = serde_json::json!({
        "shuffle_on": shuffle_on,
        "loop_on": loop_on,
    });
    match app.emit_to("tray-popover", "tray-popover-shuffle-loop", payload) {
        Ok(()) => {
            if std::env::var_os("AUDIO_HAXOR_TRAY_DEBUG").is_some() {
                eprintln!(
                    "[tray-popover-host] emit tray-popover-shuffle-loop ok shuffle_on={shuffle_on} loop_on={loop_on}"
                );
            }
        }
        Err(e) => {
            eprintln!("[tray-popover-host] emit tray-popover-shuffle-loop FAILED: {e}");
        }
    }
}

/// Lightweight subtitle-only emit. Used after BPM/Key/LUFS analysis finishes — the main window
/// already pushed a full `tray-popover-state` at `previewAudio` time, but the caches were empty
/// then, so the subtitle had no analysis values. Re-running `syncTrayNowPlayingFromPlayback` just
/// to refresh the subtitle bundles a fresh `elapsed_sec` read that is stale when the main window
/// is minimized on macOS (WebKit freezes `<audio>` updates to background windows), yanking the
/// tray progress thumb backward. This emit carries only the subtitle so interpolation is untouched.
fn emit_tray_popover_subtitle(app: &AppHandle<Wry>, subtitle: &str) {
    let payload = serde_json::json!({ "subtitle": subtitle });
    match app.emit_to("tray-popover", "tray-popover-subtitle", payload) {
        Ok(()) => {
            if std::env::var_os("AUDIO_HAXOR_TRAY_DEBUG").is_some() {
                eprintln!(
                    "[tray-popover-host] emit tray-popover-subtitle ok subtitle_ch={}",
                    subtitle.chars().count()
                );
            }
        }
        Err(e) => {
            eprintln!("[tray-popover-host] emit tray-popover-subtitle FAILED: {e}");
        }
    }
}

/// Light/dark from prefs (`prefs_set` key `theme`). Same debug env / failure logging as [`emit_tray_popover_state`].
pub fn emit_tray_popover_ui_theme(app: &AppHandle<Wry>, ui_theme: &str) {
    let payload = serde_json::json!({ "ui_theme": ui_theme });
    match app.emit_to("tray-popover", "tray-popover-ui-theme", payload) {
        Ok(()) => {
            if std::env::var_os("AUDIO_HAXOR_TRAY_DEBUG").is_some() {
                eprintln!("[tray-popover-host] emit tray-popover-ui-theme ok ui_theme={ui_theme}");
            }
        }
        Err(e) => {
            eprintln!("[tray-popover-host] emit tray-popover-ui-theme FAILED: {e}");
        }
    }
}

/// Prefer the bundle window icon; otherwise embed `32x32.png` so dev/release always have pixels.
fn tray_menu_bar_icon(app: &App) -> tauri::Result<Image<'static>> {
    if let Some(icon) = app.default_window_icon() {
        return Ok(icon.clone().to_owned());
    }
    const TRAY_PNG: &[u8] = include_bytes!("../icons/32x32.png");
    Image::from_bytes(TRAY_PNG)
}

fn t(strings: &HashMap<String, String>, key: &str, fallback: &str) -> String {
    strings
        .get(key)
        .map(|s| s.as_str())
        .filter(|s| !s.is_empty())
        .unwrap_or(fallback)
        .to_string()
}

/// Prefs key `theme` → popover `data-theme` (`light` vs `dark` HUD).
fn tray_popover_ui_theme_from_prefs() -> String {
    match history::get_preference("theme") {
        Some(serde_json::Value::String(s)) if s == "light" => "light".to_string(),
        _ => "dark".to_string(),
    }
}

fn truncate_tray_menu_line(s: &str) -> String {
    let t = s.trim();
    if t.chars().count() <= TRAY_MENU_NOW_PLAYING_MAX {
        return t.to_string();
    }
    let mut out = String::new();
    for (i, ch) in t.chars().enumerate() {
        if i >= TRAY_MENU_NOW_PLAYING_MAX.saturating_sub(1) {
            break;
        }
        out.push(ch);
    }
    out.push('…');
    out
}

/// Serialized to the `tray-popover` WebView for frosted Now Playing UI.
#[derive(Clone, serde::Serialize)]
pub struct TrayPopoverEmit {
    pub idle: bool,
    pub title: String,
    pub subtitle: String,
    /// Absolute path of the playing file — tray popover reveal / copy / context menu.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reveal_path: Option<String>,
    pub elapsed_sec: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_sec: Option<f64>,
    pub playing: bool,
    /// Clamped 0.25..=4.0 — mirrors prefs `audioSpeed` / main `#npSpeed`.
    pub playback_speed: f64,
    /// 0..=100 — mirrors prefs `audioVolume` / main `#npVolume`.
    pub volume_pct: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idle_hint: Option<String>,
    /// `"light"` or `"dark"` — tray-popover.html uses this for `html[data-theme]`.
    pub ui_theme: String,
    /// Main-window scheme snapshot (`--cyan`, `--bg-primary`, …) applied on the popover root.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub appearance: Option<HashMap<String, String>>,
    /// Mirrors prefs `shuffleMode` — tray popover transport highlight.
    pub shuffle_on: bool,
    /// Mirrors prefs `audioLoop` — tray popover transport highlight.
    pub loop_on: bool,
    /// Current track is in favorites (`favCurrentTrack` / `isFavorite`).
    pub favorite_on: bool,
    /// Per-sample loop region is enabled for the current track (`_abLoop._fromSampleRegion` or manual A-B).
    pub loop_region_enabled: bool,
    /// Loop region start (sec) — drawn as a left brace on the tray progress bar.
    pub loop_region_start_sec: f64,
    /// Loop region end (sec) — drawn as a right brace on the tray progress bar.
    pub loop_region_end_sec: f64,
    /// Flat min/max waveform peaks for the current track: `[max0, min0, max1, min1, …]`, each
    /// value in `[-1, 1]`. Sent once per track change; the tray popover renders it on
    /// `#trayWaveformCanvas` and caches locally so 500 ms tray polls don't redraw needlessly.
    /// Empty while idle. Main side uses `#serde(skip_serializing_if = "Vec::is_empty")` to keep
    /// the emit compact when peaks aren't available yet.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub waveform_peaks: Vec<f32>,
}

/// Absolute path for **Reveal in Finder** when the user activates the native tray menu's first row
/// (now-playing title). `None` when idle, unknown, or empty — same source as [`TrayPopoverEmit::reveal_path`].
pub(crate) fn tray_now_playing_reveal_path(app: &AppHandle<Wry>) -> Option<String> {
    let tray_state = app.state::<TrayState>();
    let guard = tray_state.inner.lock().ok()?;
    guard.last_popover_emit.as_ref().and_then(|e| {
        e.reveal_path.as_ref().and_then(|s| {
            let t = s.trim();
            if t.is_empty() {
                None
            } else {
                Some(t.to_string())
            }
        })
    })
}

/// Per-tray state: icon + cached i18n for rebuilding the popup without hitting SQLite each tick.
pub struct TrayState {
    pub inner: Mutex<TrayStateInner>,
}

pub struct TrayStateInner {
    pub tray: Option<TrayIcon<Wry>>,
    pub menu_strings: HashMap<String, String>,
    pub now_playing_menu_line: Option<String>,
    pub last_popover_emit: Option<TrayPopoverEmit>,
    pub last_tray_appearance: Option<HashMap<String, String>>,
}

impl Default for TrayStateInner {
    fn default() -> Self {
        Self {
            tray: None,
            menu_strings: HashMap::new(),
            now_playing_menu_line: None,
            last_popover_emit: None,
            last_tray_appearance: None,
        }
    }
}

impl Default for TrayState {
    fn default() -> Self {
        Self {
            inner: Mutex::new(TrayStateInner::default()),
        }
    }
}

/// Rebuild tray popup labels from `app_i18n` (after UI locale change). Preserves last now-playing line.
pub fn refresh_tray_popup_menu(
    app: &AppHandle<Wry>,
    state: &TrayState,
    strings: &HashMap<String, String>,
) -> Result<(), String> {
    let mut guard = state
        .inner
        .lock()
        .map_err(|_| "tray state mutex poisoned".to_string())?;
    let Some(tray) = guard.tray.clone() else {
        return Ok(());
    };
    guard.menu_strings.clone_from(strings);
    let menu = build_tray_popup_menu(
        app,
        &guard.menu_strings,
        guard.now_playing_menu_line.as_deref(),
    )?;
    drop(guard);
    tray.set_menu(Some(menu)).map_err(|e| e.to_string())
}

fn build_tray_popup_menu(
    app: &AppHandle<Wry>,
    strings: &HashMap<String, String>,
    now_playing_line: Option<&str>,
) -> Result<tauri::menu::Menu<Wry>, String> {
    let mut b = MenuBuilder::new(app);
    if let Some(raw) = now_playing_line {
        let line = truncate_tray_menu_line(raw);
        if !line.is_empty() {
            b = b.text("tray_now_playing", line);
            b = b.separator();
        }
    }
    b.text("tray_show", t(strings, "tray.show", "Show AUDIO_HAXOR"))
        .separator()
        .text("tray_scan_all", t(strings, "tray.scan_all", "Scan All"))
        .text("tray_stop_all", t(strings, "tray.stop_all", "Stop All"))
        .separator()
        .text(
            "toggle_shuffle",
            t(strings, "menu.toggle_shuffle", "Toggle Shuffle"),
        )
        .text(
            "tray_prev",
            t(strings, "tray.previous_track", "Previous Track"),
        )
        .text(
            "tray_play_pause",
            t(strings, "tray.play_pause", "Play / Pause"),
        )
        .text("tray_next", t(strings, "tray.next_track", "Next Track"))
        .text("toggle_loop", t(strings, "menu.toggle_loop", "Toggle Loop"))
        .text(
            "toggle_favorite",
            t(strings, "menu.toggle_favorite", "Toggle Favorite"),
        )
        .separator()
        .text("tray_quit", t(strings, "tray.quit", "Quit"))
        .build()
        .map_err(|e| e.to_string())
}

/// `scale_factor` maps logical popover width to **physical** pixels when `rect` uses physical coordinates
/// (common on macOS tray events).
fn popover_xy_below_tray(rect: &Rect, scale_factor: f64) -> (i32, i32) {
    let physical_coords = matches!(rect.position, Position::Physical(..));
    let pop_w_half = if physical_coords {
        f64::from(TRAY_POPOVER_W) * scale_factor / 2.0
    } else {
        f64::from(TRAY_POPOVER_W) / 2.0
    };
    let gap = if physical_coords {
        4.0_f64 * scale_factor
    } else {
        4.0_f64
    };
    let (px, py) = match rect.position {
        Position::Physical(p) => (p.x as f64, p.y as f64),
        Position::Logical(p) => (p.x, p.y),
    };
    let (w, h) = match rect.size {
        Size::Physical(s) => (s.width as f64, s.height as f64),
        Size::Logical(s) => (s.width, s.height),
    };
    let x = px + w / 2.0 - pop_w_half;
    let y = py + h + gap;
    (x.floor() as i32, y.floor() as i32)
}

fn toggle_tray_popover(app: &AppHandle<Wry>, rect: &Rect) -> Result<(), String> {
    let tray_state = app.state::<TrayState>();
    let last = tray_state
        .inner
        .lock()
        .map_err(|_| "tray state mutex poisoned".to_string())?
        .last_popover_emit
        .clone();
    let Some(win) = app.get_webview_window("tray-popover") else {
        return Ok(());
    };
    if win.is_visible().unwrap_or(false) {
        let _ = win.hide();
        return Ok(());
    }
    let mut emit = last.unwrap_or(TrayPopoverEmit {
        idle: true,
        title: String::new(),
        subtitle: String::new(),
        reveal_path: None,
        elapsed_sec: 0.0,
        total_sec: None,
        playing: false,
        playback_speed: 1.0,
        volume_pct: 100,
        idle_hint: None,
        ui_theme: tray_popover_ui_theme_from_prefs(),
        appearance: None,
        shuffle_on: false,
        loop_on: false,
        favorite_on: false,
        loop_region_enabled: false,
        loop_region_start_sec: 0.0,
        loop_region_end_sec: 0.0,
        waveform_peaks: Vec::new(),
    });
    emit.ui_theme = tray_popover_ui_theme_from_prefs();
    emit_tray_popover_state(app, &emit);
    let scale = win.scale_factor().unwrap_or(1.0);
    let (mut x, y) = popover_xy_below_tray(rect, scale);
    x = x.max(8);
    let _ = win.set_size(tauri::Size::Logical(LogicalSize::new(
        f64::from(TRAY_POPOVER_W),
        f64::from(TRAY_POPOVER_H),
    )));
    let _ = win.set_position(tauri::Position::Physical(PhysicalPosition::new(x, y)));
    let _ = win.show();
    /* Re-apply after `show`: some platforms drop window level across `hide`/`show` cycles. */
    let _ = win.set_always_on_top(true);
    /* Force the popover to become the key window so keyboard events (Escape) reach its JS
     * `keydown` listener AND so `WindowEvent::Focused(false)` fires when the user clicks
     * outside. NSPanel with `visibleOnAllWorkspaces` defaults to non-activating — clicks only
     * transfer "active" status, not key status — so without this call the popover never gets
     * keyboard focus and never fires a blur event either. The historical comment said
     * `set_focus` causes Mission Control to jump Spaces, but that was about focusing the
     * main window; the popover itself is `visibleOnAllWorkspaces: true` so focusing it stays
     * on the current Space. If this turns out to Space-jump in practice we can hop to an
     * Objective-C `makeKeyWindow` via FFI that skips the `NSApp.activate` step. */
    let _ = win.set_focus();
    Ok(())
}

pub fn create_tray(app: &App, strings: &HashMap<String, String>) -> Result<TrayIcon<Wry>, String> {
    let handle = app.handle().clone();
    let tray_menu = build_tray_popup_menu(&handle, strings, None)?;
    let icon = tray_menu_bar_icon(app).map_err(|e| e.to_string())?;
    let mut builder = TrayIconBuilder::new()
        .menu(&tray_menu)
        .icon(icon)
        .tooltip(t(strings, "tray.tooltip", "AUDIO_HAXOR"))
        .show_menu_on_left_click(cfg!(target_os = "linux"));
    #[cfg(target_os = "macos")]
    {
        // Menu bar PNGs from the app bundle are full-color; `template=true` often draws them invisible.
        builder = builder.icon_as_template(false);
    }
    let tray = builder.build(app).map_err(|e| e.to_string())?;
    #[cfg(not(target_os = "linux"))]
    {
        tray.on_tray_icon_event(move |tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                rect,
                ..
            } = event
            {
                let app = tray.app_handle().clone();
                let _ = toggle_tray_popover(&app, &rect);
            }
        });
    }
    Ok(tray)
}

#[derive(serde::Deserialize)]
pub struct TrayNowPlayingPayload {
    #[serde(default)]
    pub title_bar: Option<String>,
    pub tooltip: String,
    #[serde(default)]
    pub idle: bool,
    #[serde(default)]
    pub popover_title: Option<String>,
    #[serde(default)]
    pub popover_subtitle: Option<String>,
    #[serde(default)]
    pub elapsed_sec: Option<f64>,
    #[serde(default)]
    pub total_sec: Option<f64>,
    #[serde(default)]
    pub popover_playing: Option<bool>,
    #[serde(default)]
    pub popover_idle_label: Option<String>,
    /// Optional: main prefs `audioSpeed` (0.25..=4). When omitted, last popover value is kept.
    #[serde(default)]
    pub playback_speed: Option<f64>,
    /// Optional: main prefs `audioVolume` (0..=100). When omitted, last popover value is kept.
    #[serde(default)]
    pub volume_pct: Option<f64>,
    /// Optional: main window `data-theme` (`"light"` / `"dark"`). When omitted, Rust reads prefs.
    #[serde(default)]
    pub ui_theme: Option<String>,
    /// Optional: `getComputedStyle(document.documentElement)` snapshot for scheme vars (`--cyan`, …).
    #[serde(default)]
    pub appearance: Option<HashMap<String, String>>,
    /// Optional: filesystem path for the playing item — popover reveal / copy.
    #[serde(default)]
    pub popover_reveal_path: Option<String>,
    #[serde(default)]
    pub shuffle_on: Option<bool>,
    #[serde(default)]
    pub loop_on: Option<bool>,
    #[serde(default)]
    pub favorite_on: Option<bool>,
    #[serde(default)]
    pub loop_region_enabled: Option<bool>,
    #[serde(default)]
    pub loop_region_start_sec: Option<f64>,
    #[serde(default)]
    pub loop_region_end_sec: Option<f64>,
    /// Waveform peaks for the current track — flat `[max0, min0, max1, min1, …]`. `None` means
    /// "keep whatever peaks the cached `last_popover_emit` already holds"; an empty `Vec` means
    /// "clear" (track change with no peaks yet); a non-empty `Vec` replaces the cached peaks.
    #[serde(default)]
    pub waveform_peaks: Option<Vec<f32>>,
}

fn normalized_popover_reveal_path(payload: &TrayNowPlayingPayload) -> Option<String> {
    payload
        .popover_reveal_path
        .as_ref()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn tray_emit_ui_theme(payload: &TrayNowPlayingPayload) -> String {
    match payload.ui_theme.as_deref() {
        Some("light") => "light".to_string(),
        Some(_) => "dark".to_string(),
        None => tray_popover_ui_theme_from_prefs(),
    }
}

fn tray_playback_speed_merge(
    payload: &TrayNowPlayingPayload,
    last: Option<&TrayPopoverEmit>,
) -> f64 {
    let fallback = || last.map(|e| e.playback_speed).unwrap_or(1.0);
    match payload.playback_speed {
        Some(s) if s.is_finite() => s.clamp(0.25, 4.0),
        _ => fallback(),
    }
}

fn tray_volume_pct_merge(payload: &TrayNowPlayingPayload, last: Option<&TrayPopoverEmit>) -> u8 {
    let fallback = || last.map(|e| e.volume_pct).unwrap_or(100);
    match payload.volume_pct {
        Some(v) if v.is_finite() => v.clamp(0.0, 100.0).round() as u8,
        _ => fallback(),
    }
}

fn tray_shuffle_merge(payload: &TrayNowPlayingPayload, last: Option<&TrayPopoverEmit>) -> bool {
    match payload.shuffle_on {
        Some(s) => s,
        None => last.map(|e| e.shuffle_on).unwrap_or(false),
    }
}

fn tray_loop_merge(payload: &TrayNowPlayingPayload, last: Option<&TrayPopoverEmit>) -> bool {
    match payload.loop_on {
        Some(s) => s,
        None => last.map(|e| e.loop_on).unwrap_or(false),
    }
}

fn tray_favorite_merge(payload: &TrayNowPlayingPayload, last: Option<&TrayPopoverEmit>) -> bool {
    match payload.favorite_on {
        Some(s) => s,
        None => last.map(|e| e.favorite_on).unwrap_or(false),
    }
}

fn normalize_fav_path_key(s: &str) -> String {
    s.replace('\\', "/").trim().to_string()
}

/// Toggle favorite for `path` in SQLite. Returns new `favorite_on` and the updated list.
fn tray_prefs_toggle_favorite(path: &str) -> Option<(bool, Vec<serde_json::Value>)> {
    let key = normalize_fav_path_key(path);
    if key.is_empty() {
        return None;
    }
    let db = crate::db::global();
    let is_fav = db.favorites_is(&key).unwrap_or(false);
    let now_fav = if is_fav {
        let _ = db.favorites_remove(&key);
        false
    } else {
        let name = Path::new(&key)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(key.as_str())
            .to_string();
        let _ = db.favorites_add("sample", &key, &name, "", "", &chrono::Utc::now().to_rfc3339());
        true
    };
    let arr = db.favorites_list().unwrap_or_default();
    Some((now_fav, arr))
}

/// Tray popover / menu-bar — same rationale as [`tray_popover_toggle_shuffle`]: prefs + tray cache
/// update without requiring a live main webview (minimized / backgrounded).
pub(crate) fn tray_popover_toggle_favorite(app: &AppHandle<Wry>) -> Result<(), String> {
    let path_raw = {
        let tray_state = app.state::<TrayState>();
        let guard = tray_state
            .inner
            .lock()
            .map_err(|_| "tray state mutex poisoned".to_string())?;
        let Some(ref last) = guard.last_popover_emit else {
            return Ok(());
        };
        if last.idle {
            return Ok(());
        }
        last.reveal_path.clone()
    };
    let Some(path) = path_raw else {
        return Ok(());
    };
    let Some((now_fav, fav_arr)) = tray_prefs_toggle_favorite(&path) else {
        return Ok(());
    };
    {
        let tray_state = app.state::<TrayState>();
        let mut guard = tray_state
            .inner
            .lock()
            .map_err(|_| "tray state mutex poisoned".to_string())?;
        if let Some(ref mut emit) = guard.last_popover_emit {
            emit.favorite_on = now_fav;
        }
    }
    emit_tray_popover_favorite(app, now_fav);
    let path_norm = normalize_fav_path_key(&path);
    let _ = app.emit_to(
        "main",
        "menu-action",
        serde_json::json!({
            "action": "tray_sync_favorite",
            "favorite_on": now_fav,
            "favorites": fav_arr,
            "path": path_norm,
        }),
    );
    Ok(())
}

fn pref_bool_on_off(v: Option<&serde_json::Value>) -> bool {
    match v {
        Some(serde_json::Value::String(s)) => s == "on",
        Some(serde_json::Value::Bool(b)) => *b,
        _ => false,
    }
}

fn tray_popover_emit_shuffle_loop_sync(app: &AppHandle<Wry>, shuffle_on: bool, loop_on: bool) {
    let _ = app.emit_to(
        "main",
        "menu-action",
        serde_json::json!({
            "action": "tray_sync_shuffle_loop",
            "shuffle_on": shuffle_on,
            "loop_on": loop_on,
        }),
    );
}

/// Tray popover only — same problem as `seek:`: when the main webview is minimized / hidden /
/// backgrounded, `listen('menu-action')` may not run until the window is shown, so `toggle_shuffle` /
/// `toggle_loop` would not update prefs or engine. Apply prefs + tray cache + engine here, then
/// tell the main window the **absolute** flag values (no toggle) when it wakes.
///
/// Also called directly from `lib.rs::on_menu_event` for the **menu-bar right-click tray menu**
/// (`build_tray_popup_menu`): routing that through the frontend's `listen('menu-action')` →
/// `toggleShuffle()` path invoked `syncTrayNowPlayingFromPlayback`, which read a frozen
/// `audioPlayer.currentTime` while the main window was minimized and yanked the tray popover
/// progress thumb backward on every menu click. Handling it in Rust keeps the frontend out of
/// the loop entirely.
pub(crate) fn tray_popover_toggle_shuffle(app: &AppHandle<Wry>) -> Result<(), String> {
    let cur = pref_bool_on_off(history::get_preference("shuffleMode").as_ref());
    let next = !cur;
    history::set_preference(
        "shuffleMode",
        serde_json::Value::String(if next { "on".into() } else { "off".into() }),
    );

    let loop_fallback = pref_bool_on_off(history::get_preference("audioLoop").as_ref());

    let emit_opt = match app.try_state::<TrayState>() {
        Some(tray_state) => {
            let mut guard = tray_state
                .inner
                .lock()
                .map_err(|_| "tray state mutex poisoned".to_string())?;
            if let Some(ref mut emit) = guard.last_popover_emit {
                emit.shuffle_on = next;
            }
            guard.last_popover_emit.clone()
        }
        None => None,
    };

    /* Lightweight toggle-only emit — see [`emit_tray_popover_shuffle_loop`]. Replaces the old
     * `emit_tray_popover_state` here which was replaying a stale `elapsed_sec` on every shuffle
     * click while the main window was minimized, visibly yanking the tray progress thumb. */
    if let Some(ref e) = emit_opt {
        emit_tray_popover_shuffle_loop(app, e.shuffle_on, e.loop_on);
        tray_popover_emit_shuffle_loop_sync(app, e.shuffle_on, e.loop_on);
    } else {
        emit_tray_popover_shuffle_loop(app, next, loop_fallback);
        tray_popover_emit_shuffle_loop_sync(app, next, loop_fallback);
    }
    Ok(())
}

/// Menu-bar right-click tray menu sibling of [`tray_popover_toggle_shuffle`] — same rationale
/// for handling it in Rust rather than round-tripping through the main webview.
pub(crate) fn tray_popover_toggle_loop(app: &AppHandle<Wry>) -> Result<(), String> {
    let cur = pref_bool_on_off(history::get_preference("audioLoop").as_ref());
    let next = !cur;
    history::set_preference(
        "audioLoop",
        serde_json::Value::String(if next { "on".into() } else { "off".into() }),
    );
    thread::spawn(move || {
        let _ = crate::audio_engine::dedicated_audio_engine_request(
            &serde_json::json!({ "cmd": "playback_set_loop", "loop": next }),
        );
    });

    let shuffle_fallback = pref_bool_on_off(history::get_preference("shuffleMode").as_ref());

    let emit_opt = match app.try_state::<TrayState>() {
        Some(tray_state) => {
            let mut guard = tray_state
                .inner
                .lock()
                .map_err(|_| "tray state mutex poisoned".to_string())?;
            if let Some(ref mut emit) = guard.last_popover_emit {
                emit.loop_on = next;
            }
            guard.last_popover_emit.clone()
        }
        None => None,
    };

    /* Lightweight toggle-only emit — see [`emit_tray_popover_shuffle_loop`] and the matching
     * note in `tray_popover_toggle_shuffle`. */
    if let Some(ref e) = emit_opt {
        emit_tray_popover_shuffle_loop(app, e.shuffle_on, e.loop_on);
        tray_popover_emit_shuffle_loop_sync(app, e.shuffle_on, e.loop_on);
    } else {
        emit_tray_popover_shuffle_loop(app, shuffle_fallback, next);
        tray_popover_emit_shuffle_loop_sync(app, shuffle_fallback, next);
    }
    Ok(())
}

#[tauri::command]
pub fn tray_popover_action(app: AppHandle<Wry>, action: String) -> Result<(), String> {
    /* For `volume:<N>` and `speed:<N>` actions, update `TrayState.last_popover_emit` synchronously
     * with the incoming value. The main window's JS debounces `syncTrayNowPlayingFromPlayback` by
     * 150 ms, but the `start_tray_host_poll` thread re-emits `tray-popover-state` every 500 ms
     * using the cached `volume_pct` / `playback_speed`. Without this pre-update, a host poll tick
     * that fires between the last drag input and the debounced JS sync would broadcast the stale
     * cached value — once the popover's `_trayVolUserActive` 400 ms guard expires, the slider
     * snaps back to the old value. Updating the cache here makes the host poll always emit the
     * latest user intent. */
    if let Some(rest) = action.strip_prefix("volume:") {
        if let Ok(n) = rest.parse::<f64>() {
            if let Some(tray_state) = app.try_state::<TrayState>() {
                if let Ok(mut guard) = tray_state.inner.lock() {
                    if let Some(emit) = guard.last_popover_emit.as_mut() {
                        emit.volume_pct = n.clamp(0.0, 100.0).round() as u8;
                    }
                }
            }
        }
    } else if let Some(rest) = action.strip_prefix("speed:") {
        if let Ok(s) = rest.parse::<f64>() {
            if s.is_finite() {
                if let Some(tray_state) = app.try_state::<TrayState>() {
                    if let Ok(mut guard) = tray_state.inner.lock() {
                        if let Some(emit) = guard.last_popover_emit.as_mut() {
                            emit.playback_speed = s.clamp(0.25, 4.0);
                        }
                    }
                }
            }
        }
    } else if let Some(rest) = action.strip_prefix("seek:") {
        /* Seek directly to the audio-engine from Rust rather than round-tripping through the
         * main window's `listen('menu-action')` → `seekPlaybackToPercent` path. The main webview
         * can be suspended by WebKit when it's in another Space, minimized, or occluded, which
         * means the JS listener doesn't run until the user clicks the main app to bring it
         * forward — the user observed: "moving the playback slider in tray popover doesn't move
         * the playhead until you click on main app". Firing `playback_seek` here is
         * webview-state-independent. We still emit `menu-action` below so the main window's
         * waveform / now-playing UI picks up the new position on the next poll tick OR as soon
         * as it resumes. */
        if let Ok(frac) = rest.parse::<f64>() {
            if frac.is_finite() {
                let frac = frac.clamp(0.0, 1.0);
                let total_sec = app.try_state::<TrayState>().and_then(|s| {
                    s.inner
                        .lock()
                        .ok()
                        .and_then(|g| g.last_popover_emit.as_ref().and_then(|e| e.total_sec))
                });
                if let Some(dur) = total_sec {
                    if dur > 0.0 {
                        let position_sec = frac * dur;
                        std::thread::spawn(move || {
                            let _ = crate::audio_engine::dedicated_audio_engine_request(
                                &serde_json::json!({
                                    "cmd": "playback_seek",
                                    "position_sec": position_sec,
                                }),
                            );
                        });
                    }
                }
            }
        }
    } else if action == "toggle_shuffle" {
        return tray_popover_toggle_shuffle(&app);
    } else if action == "toggle_loop" {
        return tray_popover_toggle_loop(&app);
    } else if action == "toggle_favorite" {
        return tray_popover_toggle_favorite(&app);
    }
    // Same delivery path as `on_menu_event` in lib.rs: only the **main** webview runs `ipc.js`
    // playback handlers — broadcast `emit` does not reliably hit the main window listener.
    let _ = app.emit_to("main", "menu-action", action);
    Ok(())
}

/// Fit the `tray-popover` WebView to content (title lines, meta, fonts). Called from `tray-popover.js`.
/// Width/height are **CSS / logical** pixels (same units as `getBoundingClientRect`); `PhysicalSize` would
/// undersize on HiDPI and clip the HUD.
#[tauri::command]
pub fn tray_popover_resize(app: AppHandle<Wry>, width: f64, height: f64) -> Result<(), String> {
    let Some(win) = app.get_webview_window("tray-popover") else {
        return Ok(());
    };
    let w = width.clamp(240.0, 620.0);
    /* Tall cap: meta + wrapped directory path; `tray-popover.js` measures `#shell` scroll height.
     * Low floor (60 px) so a near-empty idle popover can actually shrink — a higher minimum
     * leaves transparent padding at the bottom that swallows clicks outside the visible shell. */
    let h = height.clamp(60.0, 1200.0);
    let _ = win.set_size(tauri::Size::Logical(LogicalSize::new(w, h)));
    Ok(())
}

/// **Tauri v2 IPC:** call `invoke('update_tray_now_playing', { payload: … })` — the outer key must be
/// `payload` (matches this parameter name); a flat object fails deserialization.
#[tauri::command]
pub fn update_tray_now_playing(
    app: AppHandle<Wry>,
    tray_state: State<'_, TrayState>,
    payload: TrayNowPlayingPayload,
) -> Result<(), String> {
    let mut guard = tray_state
        .inner
        .lock()
        .map_err(|_| "tray state mutex poisoned".to_string())?;
    let Some(tray) = guard.tray.clone() else {
        return Ok(());
    };

    let np_line = if payload.idle {
        None
    } else {
        payload
            .title_bar
            .as_ref()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
    };
    guard.now_playing_menu_line.clone_from(&np_line);

    if let Some(ref map) = payload.appearance {
        if !map.is_empty() {
            guard.last_tray_appearance = Some(map.clone());
        }
    }

    let theme = tray_emit_ui_theme(&payload);
    let appearance = guard.last_tray_appearance.clone();
    let last_emit = guard.last_popover_emit.as_ref();
    let prev_reveal_path = last_emit.and_then(|e| e.reveal_path.clone());
    let playback_speed = tray_playback_speed_merge(&payload, last_emit);
    let volume_pct = tray_volume_pct_merge(&payload, last_emit);
    let shuffle_on = tray_shuffle_merge(&payload, last_emit);
    let loop_on = tray_loop_merge(&payload, last_emit);
    let favorite_on = if payload.idle {
        false
    } else {
        tray_favorite_merge(&payload, last_emit)
    };
    /* Per-sample loop region — when the JS payload omits the fields, fall back to the last emit
     * so tray polls (no loop info) don't blank the braces. Idle state always clears. */
    let loop_region_enabled = if payload.idle {
        false
    } else {
        payload
            .loop_region_enabled
            .unwrap_or_else(|| last_emit.map(|e| e.loop_region_enabled).unwrap_or(false))
    };
    let loop_region_start_sec = if payload.idle {
        0.0
    } else {
        payload
            .loop_region_start_sec
            .unwrap_or_else(|| last_emit.map(|e| e.loop_region_start_sec).unwrap_or(0.0))
    };
    let loop_region_end_sec = if payload.idle {
        0.0
    } else {
        payload
            .loop_region_end_sec
            .unwrap_or_else(|| last_emit.map(|e| e.loop_region_end_sec).unwrap_or(0.0))
    };
    /* Waveform peaks merge: on idle, clear; otherwise prefer the payload when present
     * (`Some(vec)`), fall back to the last emit's cached peaks when omitted (`None`). */
    let waveform_peaks: Vec<f32> = if payload.idle {
        Vec::new()
    } else {
        match payload.waveform_peaks.as_ref() {
            Some(v) => v.clone(),
            None => last_emit
                .map(|e| e.waveform_peaks.clone())
                .unwrap_or_default(),
        }
    };
    let emit = if payload.idle {
        TrayPopoverEmit {
            idle: true,
            title: String::new(),
            subtitle: String::new(),
            reveal_path: None,
            elapsed_sec: 0.0,
            total_sec: None,
            playing: false,
            playback_speed,
            volume_pct,
            idle_hint: payload
                .popover_idle_label
                .clone()
                .filter(|s| !s.trim().is_empty()),
            ui_theme: theme,
            appearance: appearance.clone(),
            shuffle_on,
            loop_on,
            favorite_on,
            loop_region_enabled: false,
            loop_region_start_sec: 0.0,
            loop_region_end_sec: 0.0,
            waveform_peaks: Vec::new(),
        }
    } else {
        TrayPopoverEmit {
            idle: false,
            title: payload.popover_title.clone().unwrap_or_default(),
            subtitle: payload.popover_subtitle.clone().unwrap_or_default(),
            reveal_path: normalized_popover_reveal_path(&payload),
            elapsed_sec: payload.elapsed_sec.unwrap_or(0.0),
            total_sec: payload.total_sec,
            playing: payload.popover_playing.unwrap_or(false),
            playback_speed,
            volume_pct,
            idle_hint: None,
            ui_theme: theme,
            appearance: appearance.clone(),
            shuffle_on,
            loop_on,
            favorite_on,
            loop_region_enabled,
            loop_region_start_sec,
            loop_region_end_sec,
            waveform_peaks,
        }
    };
    guard.last_popover_emit = Some(emit.clone());

    let menu = build_tray_popup_menu(
        &app,
        &guard.menu_strings,
        guard.now_playing_menu_line.as_deref(),
    )?;
    drop(guard);
    let _ = tray.set_menu(Some(menu));
    let _ = tray.set_tooltip(Some(payload.tooltip.as_str()));
    /* Menu-bar status item shows icon only — the track name still appears as the first row of the
     * dropdown menu + in the popover HUD + the hover tooltip, but nothing is drawn next to the icon. */
    #[cfg(target_os = "macos")]
    {
        let _ = tray.set_title(None::<&str>);
    }
    emit_tray_popover_state(&app, &emit);

    /* SMB directory metadata pre-warmer: when the now-playing `reveal_path` changes, walk the
     * parent directory on a detached thread so Finder's eventual "Reveal in Finder" click lands
     * on a warm SMB stat cache instead of paying a multi-second network round-trip for the
     * listing. For local SSD libraries this is microseconds and a no-op.
     *
     * We deliberately do NOT pre-read the audio file content here anymore — the audio-engine
     * now slurps the full file into a `MemoryBlock` at playback start (see
     * `Engine.cpp::playbackLoad`), so the file is pinned in process-heap RAM for the lifetime
     * of the track and is immune to `smbfs` UBC eviction under Finder's memory pressure. A Rust
     * parallel read-through would just double the initial SMB bandwidth used at track change
     * with zero benefit. */
    let new_reveal_path = emit.reveal_path.clone();
    if new_reveal_path != prev_reveal_path {
        if let Some(rp) = new_reveal_path {
            std::thread::spawn(move || {
                let p = std::path::Path::new(&rp);
                if let Some(parent) = p.parent() {
                    if !parent.as_os_str().is_empty() {
                        if let Ok(entries) = std::fs::read_dir(parent) {
                            for entry in entries.flatten() {
                                let _ = entry.metadata();
                            }
                        }
                    }
                }
            });
        }
    }
    Ok(())
}

#[tauri::command]
pub fn tray_popover_get_state(
    tray_state: State<'_, TrayState>,
) -> Result<Option<TrayPopoverEmit>, String> {
    let guard = tray_state
        .inner
        .lock()
        .map_err(|_| "tray state mutex poisoned".to_string())?;
    Ok(guard.last_popover_emit.clone())
}

/// Lightweight push of a refreshed subtitle (BPM/Key/LUFS after background analysis) to the
/// tray popover WITHOUT touching progress state. Called by main JS `ensureAudioAnalysisForPath`
/// once the analysis caches populate. Only writes `last_popover_emit.subtitle` (so subsequent
/// full emits carry the fresh value) and emits the lightweight event. See
/// [`emit_tray_popover_subtitle`] for why a full state re-emit would clobber the tray thumb.
#[tauri::command]
pub fn tray_popover_push_subtitle(
    app: AppHandle<Wry>,
    tray_state: State<'_, TrayState>,
    subtitle: String,
) -> Result<(), String> {
    {
        let mut guard = tray_state
            .inner
            .lock()
            .map_err(|_| "tray state mutex poisoned".to_string())?;
        if let Some(ref mut emit) = guard.last_popover_emit {
            if emit.subtitle == subtitle {
                return Ok(());
            }
            emit.subtitle = subtitle.clone();
        } else {
            /* No cached emit means the main window has not pushed an initial `update_tray_now_playing`
             * yet — there is nothing to decorate. Bail rather than creating a half-populated emit. */
            return Ok(());
        }
    }
    emit_tray_popover_subtitle(&app, &subtitle);
    Ok(())
}

#[tauri::command]
pub fn tray_popover_get_ui_theme() -> String {
    tray_popover_ui_theme_from_prefs()
}

/// Bring the main window forward (tray popover context menu — same intent as the native tray “Show” item).
#[tauri::command]
pub fn show_main_window(app: AppHandle<Wry>) -> Result<(), String> {
    let Some(w) = app.get_webview_window("main") else {
        return Ok(());
    };
    w.show().map_err(|e| e.to_string())?;
    w.unminimize().map_err(|e| e.to_string())?;
    w.set_focus().map_err(|e| e.to_string())?;
    Ok(())
}

/// Hide the tray popover. Invoked from the main window (Escape keybind in `ipc.js`) so the user
/// can dismiss the popover from any focused window — `tray-popover.js`'s own `document.keydown`
/// Escape listener only fires when the popover webview itself has keyboard focus, which doesn't
/// happen if the popover was shown with `focus: false` and the user never clicked into it.
#[tauri::command]
pub fn tray_popover_hide(app: AppHandle<Wry>) -> Result<(), String> {
    if let Some(win) = app.get_webview_window("tray-popover") {
        let _ = win.hide();
    }
    Ok(())
}

static TRAY_POLL_ACTIVE: AtomicBool = AtomicBool::new(false);
/// Host-side poll interval — 500 ms matches the popover UI's expected cadence and is cheap since the
/// audio-engine stdin/stdout JSON request is just a few bytes.
const TRAY_POLL_MS: u64 = 500;

fn fmt_tray_time(sec: f64) -> String {
    let s = sec.max(0.0);
    let m = (s / 60.0) as u64;
    let r = (s as u64) % 60;
    format!("{}:{:02}", m, r)
}

fn truncate_tray_title(s: &str) -> String {
    const MAX: usize = 44;
    let t = s.trim();
    if t.chars().count() <= MAX {
        return t.to_string();
    }
    let mut out: String = t.chars().take(MAX.saturating_sub(1)).collect();
    out.push('…');
    out
}

/// Background thread that polls `audio-engine` `playback_status` and pushes fresh elapsed / total /
/// paused state to the **tray icon** and the **tray-popover** WebView, regardless of JS timer
/// throttling. The JS side (`update_tray_now_playing` in `audio.js`) still owns the **title** and
/// **subtitle** — those come from DOM state that Rust cannot see — but this thread keeps the
/// **elapsed / total / playing** fields live when the main window is unfocused (on macOS the rAF
/// loop and `setInterval` both pause behind `isUiIdleHeavyCpu`, leaving the tray frozen).
///
/// The thread is **idempotent** (guarded by `TRAY_POLL_ACTIVE`) and runs for the lifetime of the
/// app. On each tick:
///  1. Poll `playback_status` from audio-engine.
///  2. If `loaded != true`, skip (HTML5 / reverse playback does not reach the engine poll; the JS
///     `timeupdate` + keepalive paths handle those).
///  3. Merge fresh position / duration / paused into the last JS-reported `TrayPopoverEmit`.
///  4. Update `tray.set_title` (macOS) + `tray.set_tooltip` and emit `tray-popover-state`.
pub fn start_tray_host_poll(app: AppHandle<Wry>) {
    if TRAY_POLL_ACTIVE.swap(true, Ordering::SeqCst) {
        return;
    }
    thread::spawn(move || {
        // Adaptive polling: ramp up to longer sleep intervals when idle so we don't
        // wake the CPU every 500 ms for nothing (saves power on battery / background).
        let mut idle_streak: u32 = 0;
        while TRAY_POLL_ACTIVE.load(Ordering::SeqCst) {
            let sleep_ms = if idle_streak > 6 { 2000 } else { TRAY_POLL_MS };
            thread::sleep(Duration::from_millis(sleep_ms));
            if !TRAY_POLL_ACTIVE.load(Ordering::SeqCst) {
                break;
            }
            /* Short-circuit BEFORE touching the audio-engine — no point spawning the child process
             * or locking its stdin mutex until JS has reported a non-idle track. */
            let Some(tray_state) = app.try_state::<TrayState>() else {
                idle_streak = idle_streak.saturating_add(1);
                continue;
            };
            {
                let guard = match tray_state.inner.lock() {
                    Ok(g) => g,
                    Err(_) => continue,
                };
                match guard.last_popover_emit.as_ref() {
                    Some(e) if !e.idle => {
                        idle_streak = 0;
                    }
                    _ => {
                        idle_streak = idle_streak.saturating_add(1);
                        continue;
                    }
                }
            }
            let v = match crate::audio_engine::dedicated_audio_engine_request(
                &serde_json::json!({ "cmd": "playback_status" }),
            ) {
                Ok(v) => v,
                Err(_) => continue,
            };
            let loaded = v.get("loaded").and_then(|x| x.as_bool()).unwrap_or(false);
            if !loaded {
                continue;
            }
            let pos = v
                .get("position_sec")
                .and_then(|x| x.as_f64())
                .unwrap_or(0.0);
            let dur = v
                .get("duration_sec")
                .and_then(|x| x.as_f64())
                .unwrap_or(0.0);
            let paused = v.get("paused").and_then(|x| x.as_bool()).unwrap_or(false);
            let (tray, new_emit, title_bar, tooltip) = {
                let mut guard = match tray_state.inner.lock() {
                    Ok(g) => g,
                    Err(_) => continue,
                };
                let Some(tray) = guard.tray.clone() else {
                    continue;
                };
                let Some(last) = guard.last_popover_emit.clone() else {
                    continue;
                };
                /* Do not overwrite an explicit idle state — JS has torn down playback; the thread
                 * must not resurrect a fake "still playing" state from a stale position read. */
                if last.idle {
                    continue;
                }
                /* If the engine reports a fresh duration, prefer it; otherwise hold the last value so
                 * the popover does not flash "—" mid-track. */
                let total_sec = if dur > 0.0 { Some(dur) } else { last.total_sec };
                let new_emit = TrayPopoverEmit {
                    idle: false,
                    title: last.title.clone(),
                    subtitle: last.subtitle.clone(),
                    reveal_path: last.reveal_path.clone(),
                    elapsed_sec: pos,
                    total_sec,
                    playing: !paused,
                    playback_speed: last.playback_speed,
                    volume_pct: last.volume_pct,
                    idle_hint: None,
                    ui_theme: last.ui_theme.clone(),
                    appearance: last.appearance.clone(),
                    shuffle_on: last.shuffle_on,
                    loop_on: last.loop_on,
                    favorite_on: last.favorite_on,
                    loop_region_enabled: last.loop_region_enabled,
                    loop_region_start_sec: last.loop_region_start_sec,
                    loop_region_end_sec: last.loop_region_end_sec,
                    waveform_peaks: last.waveform_peaks.clone(),
                };
                guard.last_popover_emit = Some(new_emit.clone());

                let total_str = match total_sec {
                    Some(t) if t > 0.0 => fmt_tray_time(t),
                    _ => "—".to_string(),
                };
                let elapsed_str = fmt_tray_time(pos);
                /* Menu-bar title is track name only — elapsed/total stay in the popover + tooltip. */
                let title_bar = truncate_tray_title(&new_emit.title);
                let status = if new_emit.playing {
                    "Playing"
                } else {
                    "Paused"
                };
                let tooltip = if new_emit.title.is_empty() {
                    format!("{} / {} • {}", elapsed_str, total_str, status)
                } else {
                    format!(
                        "{} — {} / {} • {}",
                        new_emit.title, elapsed_str, total_str, status
                    )
                };
                (tray, new_emit, title_bar, tooltip)
            };

            /* Status-item icon only — title stays unset (see `update_tray_now_playing`). */
            let _ = title_bar;
            let _ = tray.set_tooltip(Some(tooltip.as_str()));
            emit_tray_popover_state(&app, &new_emit);
        }
        TRAY_POLL_ACTIVE.store(false, Ordering::SeqCst);
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    #[test]
    fn fmt_tray_time_non_negative_mm_ss() {
        assert_eq!(fmt_tray_time(0.0), "0:00");
        assert_eq!(fmt_tray_time(59.0), "0:59");
        assert_eq!(fmt_tray_time(60.0), "1:00");
        assert_eq!(fmt_tray_time(125.0), "2:05");
    }

    #[test]
    fn fmt_tray_time_truncates_fractional_seconds() {
        assert_eq!(fmt_tray_time(61.9), "1:01");
    }

    #[test]
    fn fmt_tray_time_negative_clamped_to_zero() {
        assert_eq!(fmt_tray_time(-10.0), "0:00");
    }

    #[test]
    fn truncate_tray_menu_line_trims_and_respects_char_boundary() {
        assert_eq!(truncate_tray_menu_line("  hi  "), "hi");
        let s = "a".repeat(TRAY_MENU_NOW_PLAYING_MAX);
        assert_eq!(truncate_tray_menu_line(&s), s);
        let long = "a".repeat(TRAY_MENU_NOW_PLAYING_MAX + 1);
        let t = truncate_tray_menu_line(&long);
        assert!(t.ends_with('…'));
        assert_eq!(t.chars().count(), TRAY_MENU_NOW_PLAYING_MAX);
    }

    #[test]
    fn truncate_tray_menu_line_counts_unicode_scalars_not_utf8_bytes() {
        let euro = "€";
        let mut s = String::new();
        for _ in 0..TRAY_MENU_NOW_PLAYING_MAX {
            s.push_str(euro);
        }
        assert_eq!(truncate_tray_menu_line(&s), s);
        s.push_str(euro);
        let t = truncate_tray_menu_line(&s);
        assert!(t.ends_with('…'));
        assert_eq!(t.chars().count(), TRAY_MENU_NOW_PLAYING_MAX);
    }

    #[test]
    fn truncate_tray_title_matches_44_char_policy() {
        assert_eq!(truncate_tray_title("  x  "), "x");
        let s = "b".repeat(44);
        assert_eq!(truncate_tray_title(&s), s);
        let long = "b".repeat(45);
        let t = truncate_tray_title(&long);
        assert!(t.ends_with('…'));
        assert_eq!(t.chars().count(), 44);
    }

    fn minimal_tray_emit() -> TrayPopoverEmit {
        TrayPopoverEmit {
            idle: false,
            title: "t".into(),
            subtitle: "s".into(),
            reveal_path: None,
            elapsed_sec: 0.0,
            total_sec: None,
            playing: false,
            playback_speed: 1.0,
            volume_pct: 50,
            idle_hint: None,
            ui_theme: "dark".into(),
            appearance: None,
            shuffle_on: false,
            loop_on: false,
            favorite_on: false,
            loop_region_enabled: false,
            loop_region_start_sec: 0.0,
            loop_region_end_sec: 0.0,
            waveform_peaks: vec![],
        }
    }

    #[test]
    fn tray_popover_emit_json_omits_none_and_empty_waveform() {
        let v = serde_json::to_value(minimal_tray_emit()).unwrap();
        let o = v.as_object().unwrap();
        assert!(!o.contains_key("reveal_path"));
        assert!(!o.contains_key("total_sec"));
        assert!(!o.contains_key("idle_hint"));
        assert!(!o.contains_key("appearance"));
        assert!(!o.contains_key("waveform_peaks"));
        assert_eq!(o.get("title"), Some(&Value::String("t".into())));
        assert_eq!(o.get("ui_theme"), Some(&Value::String("dark".into())));
    }

    #[test]
    fn tray_popover_emit_json_includes_optional_fields_when_present() {
        let mut e = minimal_tray_emit();
        e.reveal_path = Some("/music/a.flac".into());
        e.total_sec = Some(120.0);
        e.idle_hint = Some("hint".into());
        e.appearance = Some(HashMap::from([("--cyan".into(), "#fff".into())]));
        e.waveform_peaks = vec![0.5, -0.5];
        let v = serde_json::to_value(e).unwrap();
        let o = v.as_object().unwrap();
        assert_eq!(
            o.get("reveal_path"),
            Some(&Value::String("/music/a.flac".into()))
        );
        assert_eq!(o.get("total_sec"), Some(&json!(120.0)));
        assert_eq!(o.get("idle_hint"), Some(&Value::String("hint".into())));
        assert!(o.get("appearance").is_some());
        assert_eq!(o.get("waveform_peaks"), Some(&json!([0.5, -0.5])));
    }
}
