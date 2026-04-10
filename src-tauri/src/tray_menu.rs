//! System tray / menu bar icon: playback controls, dynamic title + tooltip, popup menu,
//! and (non-Linux) a **WebView popover** styled like macOS Now Playing (no artwork).
use std::collections::HashMap;
use std::sync::Mutex;
use tauri::image::Image;
use tauri::menu::MenuBuilder;
use tauri::tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};
use tauri::{
    App, AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, Position, Rect, Size, State,
    Wry,
};

/// Max characters for the first row of the tray dropdown (macOS truncates visually; keep readable).
const TRAY_MENU_NOW_PLAYING_MAX: usize = 96;

const TRAY_POPOVER_W: u32 = 300;
const TRAY_POPOVER_H: u32 = 168;

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
    pub elapsed_sec: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_sec: Option<f64>,
    pub playing: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idle_hint: Option<String>,
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
}

impl Default for TrayStateInner {
    fn default() -> Self {
        Self {
            tray: None,
            menu_strings: HashMap::new(),
            now_playing_menu_line: None,
            last_popover_emit: None,
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
        .text("tray_prev", t(strings, "tray.previous_track", "Previous Track"))
        .text("tray_play_pause", t(strings, "tray.play_pause", "Play / Pause"))
        .text("tray_next", t(strings, "tray.next_track", "Next Track"))
        .separator()
        .text("tray_quit", t(strings, "tray.quit", "Quit"))
        .build()
        .map_err(|e| e.to_string())
}

fn popover_xy_below_tray(rect: &Rect) -> (i32, i32) {
    let pop_w = f64::from(TRAY_POPOVER_W);
    let gap = 4.0;
    let (px, py) = match rect.position {
        Position::Physical(p) => (p.x as f64, p.y as f64),
        Position::Logical(p) => (p.x, p.y),
    };
    let (w, h) = match rect.size {
        Size::Physical(s) => (s.width as f64, s.height as f64),
        Size::Logical(s) => (s.width, s.height),
    };
    let x = px + w / 2.0 - pop_w / 2.0;
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
    if let Some(ref emit) = last {
        let _ = app.emit("tray-popover-state", emit);
    }
    let (mut x, y) = popover_xy_below_tray(rect);
    x = x.max(8);
    let _ = win.set_size(tauri::Size::Physical(PhysicalSize::new(
        TRAY_POPOVER_W,
        TRAY_POPOVER_H,
    )));
    let _ = win.set_position(tauri::Position::Physical(PhysicalPosition::new(x, y)));
    let _ = win.show();
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
}

#[tauri::command]
pub fn tray_popover_action(app: AppHandle<Wry>, action: String) -> Result<(), String> {
    let _ = app.emit("menu-action", action);
    Ok(())
}

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

    let emit = if payload.idle {
        TrayPopoverEmit {
            idle: true,
            title: String::new(),
            subtitle: String::new(),
            elapsed_sec: 0.0,
            total_sec: None,
            playing: false,
            idle_hint: payload
                .popover_idle_label
                .clone()
                .filter(|s| !s.trim().is_empty()),
        }
    } else {
        TrayPopoverEmit {
            idle: false,
            title: payload.popover_title.clone().unwrap_or_default(),
            subtitle: payload.popover_subtitle.clone().unwrap_or_default(),
            elapsed_sec: payload.elapsed_sec.unwrap_or(0.0),
            total_sec: payload.total_sec,
            playing: payload.popover_playing.unwrap_or(false),
            idle_hint: None,
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
    #[cfg(target_os = "macos")]
    {
        if payload.idle {
            let _ = tray.set_title(None::<&str>);
        } else {
            let _ = tray.set_title(payload.title_bar.as_deref());
        }
    }
    let _ = app.emit("tray-popover-state", &emit);
    Ok(())
}
