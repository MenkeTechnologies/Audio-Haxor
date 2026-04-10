//! System tray / menu bar icon: playback controls and dynamic title + tooltip (now playing).
use std::collections::HashMap;
use std::sync::Mutex;
use tauri::menu::MenuBuilder;
use tauri::tray::{TrayIcon, TrayIconBuilder};
use tauri::{App, AppHandle, Emitter, Manager, State, Wry};

fn t(strings: &HashMap<String, String>, key: &str, fallback: &str) -> String {
    strings
        .get(key)
        .map(|s| s.as_str())
        .filter(|s| !s.is_empty())
        .unwrap_or(fallback)
        .to_string()
}

/// Held in app state; populated in `setup` after the tray is built.
pub struct TrayState(pub Mutex<Option<TrayIcon<Wry>>>);

/// Rebuild tray popup labels from `app_i18n` (after UI locale change).
pub fn refresh_tray_popup_menu(
    app: &AppHandle<Wry>,
    tray: &TrayIcon<Wry>,
    strings: &HashMap<String, String>,
) -> Result<(), String> {
    let menu = build_tray_popup_menu(app, strings)?;
    tray.set_menu(Some(menu)).map_err(|e| e.to_string())
}

fn build_tray_popup_menu(
    app: &AppHandle<Wry>,
    strings: &HashMap<String, String>,
) -> Result<tauri::menu::Menu<Wry>, String> {
    MenuBuilder::new(app)
        .text("tray_show", t(strings, "tray.show", "Show AUDIO_HAXOR"))
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

pub fn create_tray(app: &App, strings: &HashMap<String, String>) -> Result<TrayIcon<Wry>, String> {
    let handle = app.handle().clone();
    let tray_menu = build_tray_popup_menu(&handle, strings)?;
    let mut builder = TrayIconBuilder::new()
        .menu(&tray_menu)
        .tooltip(t(strings, "tray.tooltip", "AUDIO_HAXOR"))
        .show_menu_on_left_click(true);
    if let Some(icon) = app.default_window_icon().cloned() {
        builder = builder.icon(icon).icon_as_template(true);
    }
    builder
        .on_menu_event(move |app_handle, event| {
            let id = event.id().as_ref();
            if id == "tray_quit" {
                app_handle.exit(0);
            } else if id == "tray_show" {
                if let Some(win) = app_handle.get_webview_window("main") {
                    let _ = win.show();
                    let _ = win.set_focus();
                }
            } else if let Some(win) = app_handle.get_webview_window("main") {
                let action = match id {
                    "tray_scan_all" => "scan_all",
                    "tray_stop_all" => "stop_all",
                    "tray_prev" => "prev_track",
                    "tray_play_pause" => "play_pause",
                    "tray_next" => "next_track",
                    _ => return,
                };
                let _ = win.emit("menu-action", action);
            }
        })
        .build(app)
        .map_err(|e| e.to_string())
}

#[derive(serde::Deserialize)]
pub struct TrayNowPlayingPayload {
    #[serde(default)]
    pub title_bar: Option<String>,
    pub tooltip: String,
    #[serde(default)]
    pub idle: bool,
}

#[tauri::command]
pub fn update_tray_now_playing(
    tray: State<'_, TrayState>,
    payload: TrayNowPlayingPayload,
) -> Result<(), String> {
    let guard = tray
        .0
        .lock()
        .map_err(|_| "tray state mutex poisoned".to_string())?;
    let Some(icon) = guard.as_ref() else {
        return Ok(());
    };
    let _ = icon.set_tooltip(Some(payload.tooltip.as_str()));
    #[cfg(target_os = "macos")]
    {
        if payload.idle {
            let _ = icon.set_title(None::<&str>);
        } else {
            let _ = icon.set_title(payload.title_bar.as_deref());
        }
    }
    Ok(())
}
