//! Native application menu bar — labels from SQLite `app_i18n` (see `appFmt` keys `menu.*`).

use std::collections::HashMap;
use tauri::menu::*;
use tauri::{AppHandle, Runtime};

/// Rebuilds the full menu bar using merged strings for the active UI locale.
pub fn build_native_menu_bar<R: Runtime>(
    handle: &AppHandle<R>,
    strings: &HashMap<String, String>,
) -> Result<Menu<R>, tauri::Error> {
    let t = |key: &str, fallback: &str| -> String {
        strings
            .get(key)
            .map(|s| s.as_str())
            .filter(|s| !s.is_empty())
            .unwrap_or(fallback)
            .to_string()
    };

    // App menu (macOS convention — first menu shows app name)
    let about_title = t("menu.about", "About AUDIO_HAXOR");
    let app_about = PredefinedMenuItem::about(handle, Some(about_title.as_str()), None)?;
    let app_sep1 = PredefinedMenuItem::separator(handle)?;
    let app_prefs = MenuItem::with_id(
        handle,
        "open_prefs_app",
        t("menu.preferences", "Preferences..."),
        true,
        Some("CmdOrCtrl+,"),
    )?;
    let app_sep2 = PredefinedMenuItem::separator(handle)?;
    let app_services = PredefinedMenuItem::services(handle, None)?;
    let app_sep3 = PredefinedMenuItem::separator(handle)?;
    let app_hide = PredefinedMenuItem::hide(handle, None)?;
    let app_hide_others = PredefinedMenuItem::hide_others(handle, None)?;
    let app_show_all = PredefinedMenuItem::show_all(handle, None)?;
    let app_sep4 = PredefinedMenuItem::separator(handle)?;
    let app_quit = PredefinedMenuItem::quit(handle, None)?;

    let app_menu = Submenu::with_id_and_items(
        handle,
        "app",
        t("menu.app", "AUDIO_HAXOR"),
        true,
        &[
            &app_about,
            &app_sep1,
            &app_prefs,
            &app_sep2,
            &app_services,
            &app_sep3,
            &app_hide,
            &app_hide_others,
            &app_show_all,
            &app_sep4,
            &app_quit,
        ],
    )?;

    // File menu
    let scan_all = MenuItem::with_id(
        handle,
        "scan_all",
        t("menu.scan_all", "Scan All"),
        true,
        Some("CmdOrCtrl+Shift+S"),
    )?;
    let stop_all = MenuItem::with_id(
        handle,
        "stop_all",
        t("menu.stop_all", "Stop All"),
        true,
        Some("CmdOrCtrl+."),
    )?;
    let sep1 = PredefinedMenuItem::separator(handle)?;
    let export_plugins = MenuItem::with_id(
        handle,
        "export_plugins",
        t("menu.export_plugins", "Export Plugins..."),
        true,
        Some("CmdOrCtrl+E"),
    )?;
    let import_plugins = MenuItem::with_id(
        handle,
        "import_plugins",
        t("menu.import_plugins", "Import Plugins..."),
        true,
        Some("CmdOrCtrl+I"),
    )?;
    let sep2 = PredefinedMenuItem::separator(handle)?;
    let export_audio = MenuItem::with_id(
        handle,
        "export_audio",
        t("menu.export_samples", "Export Samples..."),
        true,
        Some("CmdOrCtrl+Shift+E"),
    )?;
    let import_audio = MenuItem::with_id(
        handle,
        "import_audio",
        t("menu.import_samples", "Import Samples..."),
        true,
        Some("CmdOrCtrl+Shift+I"),
    )?;
    let sep3 = PredefinedMenuItem::separator(handle)?;
    let export_daw = MenuItem::with_id(
        handle,
        "export_daw",
        t("menu.export_daw", "Export DAW Projects..."),
        true,
        Some("CmdOrCtrl+Shift+O"),
    )?;
    let import_daw = MenuItem::with_id(
        handle,
        "import_daw",
        t("menu.import_daw", "Import DAW Projects..."),
        true,
        Some("CmdOrCtrl+Shift+J"),
    )?;
    let sep4 = PredefinedMenuItem::separator(handle)?;
    let export_presets = MenuItem::with_id(
        handle,
        "export_presets",
        t("menu.export_presets", "Export Presets..."),
        true,
        Some("CmdOrCtrl+Shift+Alt+Y"),
    )?;
    let import_presets = MenuItem::with_id(
        handle,
        "import_presets",
        t("menu.import_presets", "Import Presets..."),
        true,
        Some("CmdOrCtrl+Shift+Alt+Z"),
    )?;
    let file_menu = Submenu::with_id_and_items(
        handle,
        "file",
        t("menu.file", "File"),
        true,
        &[
            &scan_all,
            &stop_all,
            &sep1,
            &export_plugins,
            &import_plugins,
            &sep2,
            &export_audio,
            &import_audio,
            &sep3,
            &export_daw,
            &import_daw,
            &sep4,
            &export_presets,
            &import_presets,
        ],
    )?;

    // Edit menu
    let edit_undo = PredefinedMenuItem::undo(handle, None)?;
    let edit_redo = PredefinedMenuItem::redo(handle, None)?;
    let edit_sep1 = PredefinedMenuItem::separator(handle)?;
    let edit_cut = PredefinedMenuItem::cut(handle, None)?;
    let edit_copy = PredefinedMenuItem::copy(handle, None)?;
    let edit_paste = PredefinedMenuItem::paste(handle, None)?;
    let edit_select_all = PredefinedMenuItem::select_all(handle, None)?;
    let edit_sep2 = PredefinedMenuItem::separator(handle)?;
    let find = MenuItem::with_id(
        handle,
        "find",
        t("menu.find", "Find..."),
        true,
        Some("CmdOrCtrl+F"),
    )?;

    let edit_menu = Submenu::with_id_and_items(
        handle,
        "edit",
        t("menu.edit", "Edit"),
        true,
        &[
            &edit_undo,
            &edit_redo,
            &edit_sep1,
            &edit_cut,
            &edit_copy,
            &edit_paste,
            &edit_select_all,
            &edit_sep2,
            &find,
        ],
    )?;

    // Scan menu
    let scan_plugins = MenuItem::with_id(
        handle,
        "scan_plugins",
        t("menu.scan_plugins", "Scan Plugins"),
        true,
        Some("CmdOrCtrl+Shift+P"),
    )?;
    let scan_audio = MenuItem::with_id(
        handle,
        "scan_audio",
        t("menu.scan_samples", "Scan Samples"),
        true,
        Some("CmdOrCtrl+Shift+A"),
    )?;
    let scan_daw = MenuItem::with_id(
        handle,
        "scan_daw",
        t("menu.scan_daw", "Scan DAW Projects"),
        true,
        Some("CmdOrCtrl+Shift+D"),
    )?;
    let scan_presets = MenuItem::with_id(
        handle,
        "scan_presets",
        t("menu.scan_presets", "Scan Presets"),
        true,
        Some("CmdOrCtrl+Shift+R"),
    )?;
    let scan_pdfs = MenuItem::with_id(
        handle,
        "scan_pdfs",
        t("menu.scan_pdf", "Scan PDFs"),
        true,
        Some("CmdOrCtrl+Shift+F"),
    )?;
    let stop_pdf_scan = MenuItem::with_id(
        handle,
        "stop_pdf_scan",
        t("menu.stop_pdf_scan", "Stop PDF scan"),
        true,
        Some("CmdOrCtrl+Shift+Y"),
    )?;
    let scan_sep = PredefinedMenuItem::separator(handle)?;
    let check_updates = MenuItem::with_id(
        handle,
        "check_updates",
        t("menu.check_updates", "Check Updates"),
        true,
        Some("CmdOrCtrl+U"),
    )?;

    let scan_menu = Submenu::with_id_and_items(
        handle,
        "scan",
        t("menu.scan", "Scan"),
        true,
        &[
            &scan_plugins,
            &scan_audio,
            &scan_daw,
            &scan_presets,
            &scan_pdfs,
            &stop_pdf_scan,
            &scan_sep,
            &check_updates,
        ],
    )?;

    // View menu
    let tab_plugins = MenuItem::with_id(
        handle,
        "tab_plugins",
        t("menu.tab_plugins", "Plugins"),
        true,
        Some("CmdOrCtrl+1"),
    )?;
    let tab_samples = MenuItem::with_id(
        handle,
        "tab_samples",
        t("menu.tab_samples", "Samples"),
        true,
        Some("CmdOrCtrl+2"),
    )?;
    let tab_daw = MenuItem::with_id(
        handle,
        "tab_daw",
        t("menu.tab_daw", "DAW Projects"),
        true,
        Some("CmdOrCtrl+3"),
    )?;
    let tab_presets = MenuItem::with_id(
        handle,
        "tab_presets",
        t("menu.tab_presets", "Presets"),
        true,
        Some("CmdOrCtrl+4"),
    )?;
    let tab_favorites = MenuItem::with_id(
        handle,
        "tab_favorites",
        t("menu.tab_favorites", "Favorites"),
        true,
        Some("CmdOrCtrl+5"),
    )?;
    let tab_notes = MenuItem::with_id(
        handle,
        "tab_notes",
        t("menu.tab_notes", "Notes"),
        true,
        Some("CmdOrCtrl+6"),
    )?;
    let tab_history = MenuItem::with_id(
        handle,
        "tab_history",
        t("menu.tab_history", "History"),
        true,
        Some("CmdOrCtrl+7"),
    )?;
    let tab_settings = MenuItem::with_id(
        handle,
        "tab_settings",
        t("menu.tab_settings", "Settings"),
        true,
        Some("CmdOrCtrl+8"),
    )?;
    let tab_files = MenuItem::with_id(
        handle,
        "tab_files",
        t("menu.tab_files", "Files"),
        true,
        Some("CmdOrCtrl+9"),
    )?;
    let tab_audio_engine = MenuItem::with_id(
        handle,
        "tab_audio_engine",
        t("menu.tab_audio_engine", "Audio Engine"),
        true,
        Some("CmdOrCtrl+F6"),
    )?;
    let view_sep = PredefinedMenuItem::separator(handle)?;
    let toggle_theme = MenuItem::with_id(
        handle,
        "toggle_theme",
        t("menu.toggle_theme", "Toggle Light/Dark"),
        true,
        Some("CmdOrCtrl+T"),
    )?;
    let toggle_crt = MenuItem::with_id(
        handle,
        "toggle_crt",
        t("menu.toggle_crt", "Toggle CRT Effects"),
        true,
        Some("F1"),
    )?;
    let view_sep2 = PredefinedMenuItem::separator(handle)?;
    let reset_columns = MenuItem::with_id(
        handle,
        "reset_columns",
        t("menu.reset_columns", "Reset Column Widths"),
        true,
        Some("CmdOrCtrl+Shift+W"),
    )?;
    let reset_tabs = MenuItem::with_id(
        handle,
        "reset_tabs",
        t("menu.reset_tabs", "Reset Tab Order"),
        true,
        Some("CmdOrCtrl+Shift+T"),
    )?;

    let view_menu = Submenu::with_id_and_items(
        handle,
        "view",
        t("menu.view", "View"),
        true,
        &[
            &tab_plugins,
            &tab_samples,
            &tab_daw,
            &tab_presets,
            &tab_favorites,
            &tab_notes,
            &tab_history,
            &tab_settings,
            &tab_files,
            &tab_audio_engine,
            &view_sep,
            &toggle_theme,
            &toggle_crt,
            &view_sep2,
            &reset_columns,
            &reset_tabs,
        ],
    )?;

    // Playback menu
    let play_pause = MenuItem::with_id(
        handle,
        "play_pause",
        t("menu.play_pause", "Play / Pause"),
        true,
        Some("Space"),
    )?;
    let toggle_loop = MenuItem::with_id(
        handle,
        "toggle_loop",
        t("menu.toggle_loop", "Toggle Loop"),
        true,
        Some("CmdOrCtrl+L"),
    )?;
    let stop_playback = MenuItem::with_id(
        handle,
        "stop_playback",
        t("menu.stop_playback", "Stop Playback"),
        true,
        Some("CmdOrCtrl+Shift+."),
    )?;
    let expand_player = MenuItem::with_id(
        handle,
        "expand_player",
        t("menu.expand_player", "Expand / Collapse Player"),
        true,
        Some("CmdOrCtrl+Shift+Alt+E"),
    )?;
    let video_audio_route_sep = PredefinedMenuItem::separator(handle)?;
    let video_audio_route_engine = MenuItem::with_id(
        handle,
        "video_audio_route_engine",
        t(
            "menu.video_audio_route_engine",
            "Video Audio: Audio Engine (plugins)",
        ),
        true,
        Some("CmdOrCtrl+Shift+7"),
    )?;
    let video_audio_route_html5 = MenuItem::with_id(
        handle,
        "video_audio_route_html5",
        t(
            "menu.video_audio_route_html5",
            "Video Audio: WebView (HTML5)",
        ),
        true,
        Some("CmdOrCtrl+Shift+8"),
    )?;

    let next_track = MenuItem::with_id(
        handle,
        "next_track",
        t("menu.next_track", "Next Track"),
        true,
        Some("CmdOrCtrl+Right"),
    )?;
    let prev_track = MenuItem::with_id(
        handle,
        "prev_track",
        t("menu.prev_track", "Previous Track"),
        true,
        Some("CmdOrCtrl+Left"),
    )?;
    let toggle_shuffle = MenuItem::with_id(
        handle,
        "toggle_shuffle",
        t("menu.toggle_shuffle", "Toggle Shuffle"),
        true,
        Some("S"),
    )?;
    let toggle_mute = MenuItem::with_id(
        handle,
        "toggle_mute",
        t("menu.toggle_mute", "Mute / Unmute"),
        true,
        Some("M"),
    )?;
    let playback_sep = PredefinedMenuItem::separator(handle)?;

    let playback_menu = Submenu::with_id_and_items(
        handle,
        "playback",
        t("menu.playback", "Playback"),
        true,
        &[
            &play_pause,
            &stop_playback,
            &playback_sep,
            &next_track,
            &prev_track,
            &toggle_loop,
            &toggle_shuffle,
            &toggle_mute,
            &playback_sep,
            &expand_player,
            &video_audio_route_sep,
            &video_audio_route_engine,
            &video_audio_route_html5,
        ],
    )?;

    // Data menu
    let clear_history = MenuItem::with_id(
        handle,
        "clear_history",
        t("menu.clear_history", "Clear All History..."),
        true,
        Some("CmdOrCtrl+Shift+Delete"),
    )?;
    let clear_all_databases = MenuItem::with_id(
        handle,
        "clear_all_databases",
        t("menu.clear_all_databases", "Clear All Databases"),
        true,
        Some("CmdOrCtrl+Shift+Alt+D"),
    )?;
    let clear_kvr = MenuItem::with_id(
        handle,
        "clear_kvr",
        t("menu.clear_kvr", "Clear KVR Cache..."),
        true,
        Some("CmdOrCtrl+Shift+Alt+K"),
    )?;
    let clear_favorites = MenuItem::with_id(
        handle,
        "clear_favorites",
        t("menu.clear_favorites", "Clear Favorites..."),
        true,
        Some("CmdOrCtrl+Shift+Alt+F"),
    )?;

    let reset_all = MenuItem::with_id(
        handle,
        "reset_all",
        t("menu.reset_all_scans", "Reset All Scans..."),
        true,
        Some("CmdOrCtrl+Shift+Backspace"),
    )?;
    let data_sep = PredefinedMenuItem::separator(handle)?;
    let find_duplicates = MenuItem::with_id(
        handle,
        "find_duplicates",
        t("menu.find_duplicates", "Find Duplicates"),
        true,
        Some("CmdOrCtrl+D"),
    )?;
    let tools_bg_sep = PredefinedMenuItem::separator(handle)?;
    let bpm_key_lufs_start = MenuItem::with_id(
        handle,
        "bpm_key_lufs_start",
        t(
            "menu.start_bpm_key_lufs_analysis",
            "Start BPM/Key/LUFS background analysis",
        ),
        true,
        Some("CmdOrCtrl+Shift+V"),
    )?;
    let bpm_key_lufs_stop = MenuItem::with_id(
        handle,
        "bpm_key_lufs_stop",
        t(
            "menu.stop_bpm_key_lufs_analysis",
            "Stop BPM/Key/LUFS background analysis",
        ),
        true,
        Some("CmdOrCtrl+Shift+C"),
    )?;
    let extract_pdf_metadata = MenuItem::with_id(
        handle,
        "extract_pdf_metadata",
        t("menu.extract_pdf_metadata", "Start PDF metadata"),
        true,
        Some("CmdOrCtrl+Shift+M"),
    )?;
    // Not Cmd+Shift+Z: that collides with Edit → Redo on macOS, so the key never appears on the item.
    let stop_pdf_metadata = MenuItem::with_id(
        handle,
        "stop_pdf_metadata",
        t("menu.stop_pdf_metadata", "Stop PDF metadata"),
        true,
        Some("CmdOrCtrl+Shift+K"),
    )?;
    let build_fingerprint_cache_menu = MenuItem::with_id(
        handle,
        "build_fingerprint_cache_menu",
        t("menu.build_fingerprint_cache", "Start fingerprint cache"),
        true,
        Some("CmdOrCtrl+Shift+B"),
    )?;
    let stop_fingerprint_cache_menu = MenuItem::with_id(
        handle,
        "stop_fingerprint_cache_menu",
        t("menu.stop_fingerprint_cache", "Stop fingerprint cache"),
        true,
        Some("CmdOrCtrl+Shift+F4"),
    )?;
    let start_content_dup_scan = MenuItem::with_id(
        handle,
        "start_content_dup_scan",
        t(
            "menu.start_content_dup_scan",
            "Start byte-duplicate scan (SHA-256)",
        ),
        true,
        Some("CmdOrCtrl+Shift+Comma"),
    )?;
    let stop_content_dup_scan = MenuItem::with_id(
        handle,
        "stop_content_dup_scan",
        t(
            "menu.stop_content_dup_scan",
            "Stop byte-duplicate scan",
        ),
        true,
        Some("CmdOrCtrl+Shift+Period"),
    )?;
    let start_all_background_jobs = MenuItem::with_id(
        handle,
        "start_all_background_jobs",
        t("menu.start_all_background_jobs", "Start all background jobs"),
        true,
        Some("CmdOrCtrl+Shift+F5"),
    )?;
    let stop_all_background_jobs = MenuItem::with_id(
        handle,
        "stop_all_background_jobs",
        t("menu.stop_all_background_jobs", "Stop all background jobs"),
        true,
        Some("CmdOrCtrl+Shift+F6"),
    )?;
    let dep_graph = MenuItem::with_id(
        handle,
        "dep_graph",
        t("menu.dep_graph", "Dependency Graph"),
        true,
        Some("CmdOrCtrl+G"),
    )?;
    let cmd_palette = MenuItem::with_id(
        handle,
        "cmd_palette",
        t("menu.cmd_palette", "Command Palette"),
        true,
        Some("CmdOrCtrl+K"),
    )?;
    let help_overlay = MenuItem::with_id(
        handle,
        "help_overlay",
        t("menu.help_overlay", "Keyboard Shortcuts"),
        true,
        Some("CmdOrCtrl+Shift+F3"),
    )?;

    let data_menu = Submenu::with_id_and_items(
        handle,
        "data",
        t("menu.data", "Data"),
        true,
        &[
            &clear_history,
            &clear_all_databases,
            &clear_kvr,
            &clear_favorites,
            &data_sep,
            &reset_all,
        ],
    )?;

    let tools_menu = Submenu::with_id_and_items(
        handle,
        "tools",
        t("menu.tools", "Tools"),
        true,
        &[
            &find_duplicates,
            &tools_bg_sep,
            &bpm_key_lufs_start,
            &bpm_key_lufs_stop,
            &extract_pdf_metadata,
            &stop_pdf_metadata,
            &build_fingerprint_cache_menu,
            &stop_fingerprint_cache_menu,
            &start_content_dup_scan,
            &stop_content_dup_scan,
            &start_all_background_jobs,
            &stop_all_background_jobs,
            &dep_graph,
            &data_sep,
            &cmd_palette,
            &help_overlay,
        ],
    )?;

    // Window menu
    let minimize = PredefinedMenuItem::minimize(handle, None)?;
    let zoom = PredefinedMenuItem::maximize(handle, None)?;
    let win_sep = PredefinedMenuItem::separator(handle)?;
    let close_win = PredefinedMenuItem::close_window(handle, None)?;

    let window_menu = Submenu::with_id_and_items(
        handle,
        "window",
        t("menu.window", "Window"),
        true,
        &[&minimize, &zoom, &win_sep, &close_win],
    )?;

    // Help menu
    let github = MenuItem::with_id(
        handle,
        "github",
        t("menu.github", "GitHub Repository"),
        true,
        Some("CmdOrCtrl+Shift+G"),
    )?;
    let docs = MenuItem::with_id(
        handle,
        "docs",
        t("menu.docs", "Documentation"),
        true,
        Some("CmdOrCtrl+Shift+Alt+P"),
    )?;

    let help_menu = Submenu::with_id_and_items(
        handle,
        "help",
        t("menu.help", "Help"),
        true,
        &[&github, &docs],
    )?;

    Menu::with_items(
        handle,
        &[
            &app_menu,
            &file_menu,
            &edit_menu,
            &scan_menu,
            &view_menu,
            &playback_menu,
            &data_menu,
            &tools_menu,
            &window_menu,
            &help_menu,
        ],
    )
}
