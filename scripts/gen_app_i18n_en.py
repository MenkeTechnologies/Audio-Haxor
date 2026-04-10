#!/usr/bin/env python3
"""Generate i18n/app_i18n_en.json: toasts + menus + tray + HTML + help + confirm dialogs.

Run from repo root: python3 scripts/gen_app_i18n_en.py
Optionally inject data-i18n* attributes into frontend/index.html (idempotent).
"""
from __future__ import annotations

import html as html_module
import importlib.util
import json
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
I18N_DIR = ROOT / "i18n"

ALLOWED_PREFIXES = ("toast.", "menu.", "tray.", "ui.", "help.", "confirm.")


def load_toast_en() -> dict[str, str]:
    p = ROOT / "scripts" / "gen_toast_i18n_en.py"
    spec = importlib.util.spec_from_file_location("toast_src", p)
    if spec is None or spec.loader is None:
        raise SystemExit(f"cannot load {p}")
    m = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(m)
    return dict(m.TOAST_EN)


_CTRL_OR_SEP = re.compile(r"[\u0000-\u001F\u007F\u2028\u2029]")


def norm_catalog_text(s: str) -> str:
    """Collapse HTML formatting whitespace so JSON seeds pass i18n-value-safety (no raw newlines)."""
    t = html_module.unescape(s)
    return re.sub(r"\s+", " ", t).strip()


def sanitize_catalog_value(v: str) -> str:
    """Ensure stored strings have no C0/DEL/U+2028/U+2029 (SQLite / UI safety tests)."""
    if not _CTRL_OR_SEP.search(v):
        return v
    return norm_catalog_text(v)


def slugify(text: str) -> str:
    s = text.strip().lower()
    s = re.sub(r"[^a-z0-9]+", "_", s)
    s = re.sub(r"_+", "_", s).strip("_")
    return (s[:50] or "empty")


def alloc_key(prefix: str, text: str, seen: dict[str, int]) -> str:
    base = slugify(text)
    n = seen.get(base, 0)
    seen[base] = n + 1
    if n == 0:
        return f"{prefix}.{base}"
    return f"{prefix}.{base}_{n}"


def is_translatable_visible_text(s: str) -> bool:
    """True if inner HTML fragment has at least one letter (after entity decode)."""
    t = html_module.unescape(s).strip()
    if not t:
        return False
    return any(c.isalpha() for c in t)


def extract_ui_from_html(html: str) -> dict[str, str]:
    """Map i18n key -> English text for placeholders, titles, options, settings rows, buttons, etc."""
    out: dict[str, str] = {}
    seen_ph: dict[str, int] = {}
    seen_tt: dict[str, int] = {}
    seen_opt: dict[str, int] = {}
    seen_st: dict[str, int] = {}
    seen_sd: dict[str, int] = {}
    seen_sh: dict[str, int] = {}
    seen_btn: dict[str, int] = {}
    seen_h2: dict[str, int] = {}
    seen_p: dict[str, int] = {}
    seen_lbl: dict[str, int] = {}

    for m in re.finditer(r'placeholder="([^"]*)"', html):
        val = m.group(1)
        val = norm_catalog_text(val)
        if not val:
            continue
        k = alloc_key("ui.ph", val, seen_ph)
        out[k] = val

    for m in re.finditer(r' title="([^"]*)"', html):
        val = m.group(1)
        val = norm_catalog_text(val)
        if not val:
            continue
        k = alloc_key("ui.tt", val, seen_tt)
        out[k] = val

    for m in re.finditer(r"<option[^>]*>([^<]+)</option>", html):
        val = norm_catalog_text(m.group(1))
        if not val:
            continue
        k = alloc_key("ui.opt", val, seen_opt)
        out[k] = val

    for m in re.finditer(r'<span\s+class="settings-title"([^>]*)>([^<]+)</span>', html, re.IGNORECASE):
        attrs, text = m.group(1), norm_catalog_text(m.group(2))
        if "data-i18n=" in attrs:
            continue
        if not text:
            continue
        k = alloc_key("ui.st", text, seen_st)
        out[k] = text

    for m in re.finditer(r'<span\s+class="settings-desc"([^>]*)>([^<]+)</span>', html, re.IGNORECASE):
        attrs, text = m.group(1), norm_catalog_text(m.group(2))
        if "data-i18n=" in attrs:
            continue
        if not text:
            continue
        k = alloc_key("ui.sd", text, seen_sd)
        out[k] = text

    for m in re.finditer(r'<h2\s+class="settings-heading"([^>]*)>([^<]+)</h2>', html, re.IGNORECASE):
        attrs, text = m.group(1), norm_catalog_text(m.group(2))
        if "data-i18n=" in attrs:
            continue
        if not text:
            continue
        k = alloc_key("ui.sh", text, seen_sh)
        out[k] = text

    for m in re.finditer(r"<button([^>]*)>([^<]+)</button>", html, re.IGNORECASE):
        attrs, text = m.group(1), norm_catalog_text(m.group(2))
        if "data-i18n=" in attrs:
            continue
        if not is_translatable_visible_text(text):
            continue
        k = alloc_key("ui.btn", text, seen_btn)
        out[k] = text

    for m in re.finditer(r"<h2([^>]*)>([^<]+)</h2>", html, re.IGNORECASE):
        attrs, text = m.group(1), norm_catalog_text(m.group(2))
        if "data-i18n=" in attrs or "settings-heading" in attrs:
            continue
        if not text:
            continue
        k = alloc_key("ui.h2", text, seen_h2)
        out[k] = text

    for m in re.finditer(r"<p([^>]*)>([^<]+)</p>", html, re.IGNORECASE):
        attrs, text = m.group(1), norm_catalog_text(m.group(2))
        if "data-i18n=" in attrs:
            continue
        if not text:
            continue
        k = alloc_key("ui.p", text, seen_p)
        out[k] = text

    for m in re.finditer(
        r'<div class="dirs-toggle"[^>]*>\s*<span[^>]*>[^<]*</span>\s*<span>([^<]+)</span>',
        html,
        re.IGNORECASE,
    ):
        text = norm_catalog_text(m.group(1))
        if not text:
            continue
        k = alloc_key("ui.lbl", text, seen_lbl)
        out[k] = text

    for m in re.finditer(
        r'<div class="history-sidebar-header"[^>]*>\s*<span>([^<]+)</span>',
        html,
        re.IGNORECASE,
    ):
        text = norm_catalog_text(m.group(1))
        if not text:
            continue
        k = alloc_key("ui.lbl", text, seen_lbl)
        out[k] = text

    return out


def extract_existing_data_i18n(html: str) -> dict[str, str]:
    """Read ui.* keys already present in HTML (survives re-run after inject)."""
    out: dict[str, str] = {}
    patterns = [
        r'<span\b[^>]*\sdata-i18n="(ui\.[^"]+)"[^>]*>([^<]+)</span>',
        r'<button\b[^>]*\sdata-i18n="(ui\.[^"]+)"[^>]*>([^<]+)</button>',
        r'<h2\b[^>]*\sdata-i18n="(ui\.[^"]+)"[^>]*>([^<]+)</h2>',
        r'<p\b[^>]*\sdata-i18n="(ui\.[^"]+)"[^>]*>([^<]+)</p>',
        r'<option\b[^>]*\sdata-i18n="(ui\.[^"]+)"[^>]*>([^<]+)</option>',
    ]
    for pat in patterns:
        for m in re.finditer(pat, html, re.IGNORECASE):
            out[m.group(1)] = norm_catalog_text(m.group(2))
    return out


def insert_before_gt(tag: str, insert: str) -> str:
    tag = tag.rstrip()
    if tag.endswith("/>"):
        return tag[:-2].rstrip() + " " + insert + " />"
    if tag.endswith(">"):
        return tag[:-1] + " " + insert + ">"
    return tag


def inject_open_tags(html: str, ui: dict[str, str]) -> str:
    """Build value->key maps and add data-i18n* to tags (skip if already present)."""
    ph_vk: dict[str, str] = {}
    tt_vk: dict[str, str] = {}
    opt_vk: dict[str, str] = {}
    st_vk: dict[str, str] = {}
    sd_vk: dict[str, str] = {}
    sh_vk: dict[str, str] = {}
    btn_vk: dict[str, str] = {}
    h2_vk: dict[str, str] = {}
    p_vk: dict[str, str] = {}
    lbl_vk: dict[str, str] = {}
    for k, v in ui.items():
        if k.startswith("ui.ph."):
            ph_vk.setdefault(v, k)
        elif k.startswith("ui.tt."):
            tt_vk.setdefault(v, k)
        elif k.startswith("ui.opt."):
            opt_vk.setdefault(v, k)
        elif k.startswith("ui.st."):
            st_vk.setdefault(v, k)
        elif k.startswith("ui.sd."):
            sd_vk.setdefault(v, k)
        elif k.startswith("ui.sh."):
            sh_vk.setdefault(v, k)
        elif k.startswith("ui.btn."):
            btn_vk.setdefault(v, k)
        elif k.startswith("ui.h2."):
            h2_vk.setdefault(v, k)
        elif k.startswith("ui.p."):
            p_vk.setdefault(v, k)
        elif k.startswith("ui.lbl."):
            lbl_vk.setdefault(v, k)

    def patch_tag(m: re.Match[str]) -> str:
        tag = m.group(0)
        if tag.startswith("<!") or tag.startswith("<?"):
            return tag
        low = tag.lower()
        if not low.startswith("<option") and "placeholder=" in tag and "data-i18n-placeholder" not in tag:
            m = re.search(r'placeholder="([^"]*)"', tag)
            if m:
                val = m.group(1)
                k = ph_vk.get(val)
                if k:
                    tag = insert_before_gt(tag, f'data-i18n-placeholder="{k}"')
        if " title=" in tag and "data-i18n-title" not in tag:
            m = re.search(r' title="([^"]*)"', tag)
            if m:
                val = m.group(1)
                if val.strip():
                    k = tt_vk.get(val)
                    if k:
                        tag = insert_before_gt(tag, f'data-i18n-title="{k}"')
        return tag

    html = re.sub(r"<[^>]+>", patch_tag, html)

    def inject_option(m: re.Match[str]) -> str:
        attrs, text = m.group(1), m.group(2).strip()
        if "data-i18n=" in attrs:
            return m.group(0)
        k = opt_vk.get(text)
        if not k:
            return m.group(0)
        return f"<option{attrs} data-i18n=\"{k}\">{text}</option>"

    html = re.sub(r"<option([^>]*)>([^<]+)</option>", inject_option, html, flags=re.IGNORECASE)

    def inject_st(m: re.Match[str]) -> str:
        attrs, text = m.group(1), m.group(2).strip()
        if "data-i18n=" in attrs:
            return m.group(0)
        k = st_vk.get(text)
        if not k:
            return m.group(0)
        return f'<span class="settings-title"{attrs} data-i18n="{k}">{text}</span>'

    html = re.sub(
        r'<span\s+class="settings-title"([^>]*)>([^<]+)</span>',
        inject_st,
        html,
        flags=re.IGNORECASE,
    )

    def inject_sd(m: re.Match[str]) -> str:
        attrs, text = m.group(1), m.group(2).strip()
        if "data-i18n=" in attrs:
            return m.group(0)
        k = sd_vk.get(text)
        if not k:
            return m.group(0)
        return f'<span class="settings-desc"{attrs} data-i18n="{k}">{text}</span>'

    html = re.sub(
        r'<span\s+class="settings-desc"([^>]*)>([^<]+)</span>',
        inject_sd,
        html,
        flags=re.IGNORECASE,
    )

    def inject_sh(m: re.Match[str]) -> str:
        attrs, text = m.group(1), m.group(2).strip()
        if "data-i18n=" in attrs:
            return m.group(0)
        k = sh_vk.get(text)
        if not k:
            return m.group(0)
        return f'<h2 class="settings-heading"{attrs} data-i18n="{k}">{text}</h2>'

    html = re.sub(
        r'<h2\s+class="settings-heading"([^>]*)>([^<]+)</h2>',
        inject_sh,
        html,
        flags=re.IGNORECASE,
    )

    def inject_p(m: re.Match[str]) -> str:
        attrs, text = m.group(1), m.group(2).strip()
        if "data-i18n=" in attrs:
            return m.group(0)
        k = p_vk.get(text)
        if not k:
            return m.group(0)
        return f"<p{attrs} data-i18n=\"{k}\">{text}</p>"

    html = re.sub(r"<p([^>]*)>([^<]+)</p>", inject_p, html, flags=re.IGNORECASE)

    def inject_dirs_lbl(m: re.Match[str]) -> str:
        text = m.group(1).strip()
        k = lbl_vk.get(text)
        if not k:
            return m.group(0)
        return f'<span id="dirsArrow">&#9654;</span>\n        <span data-i18n="{k}">{text}</span>'

    html = re.sub(
        r"<span id=\"dirsArrow\">&#9654;</span>\s*<span>([^<]+)</span>",
        inject_dirs_lbl,
        html,
        flags=re.IGNORECASE,
    )

    def inject_history_header(m: re.Match[str]) -> str:
        prefix, text = m.group(1), m.group(2).strip()
        k = lbl_vk.get(text)
        if not k:
            return m.group(0)
        return f'{prefix}<span data-i18n="{k}">{text}</span>'

    html = re.sub(
        r'(<div class="history-sidebar-header"[^>]*>\s*)<span>([^<]+)</span>',
        inject_history_header,
        html,
        flags=re.IGNORECASE,
    )

    def inject_h2_plain(m: re.Match[str]) -> str:
        attrs, text = m.group(1), m.group(2).strip()
        if "data-i18n=" in attrs or "settings-heading" in attrs:
            return m.group(0)
        k = h2_vk.get(text)
        if not k:
            return m.group(0)
        return f"<h2{attrs} data-i18n=\"{k}\">{text}</h2>"

    html = re.sub(r"<h2([^>]*)>([^<]+)</h2>", inject_h2_plain, html, flags=re.IGNORECASE)

    def inject_btn(m: re.Match[str]) -> str:
        attrs, text = m.group(1), m.group(2).strip()
        if "data-i18n=" in attrs:
            return m.group(0)
        if not is_translatable_visible_text(text):
            return m.group(0)
        k = btn_vk.get(text)
        if not k:
            return m.group(0)
        return f"<button{attrs} data-i18n=\"{k}\">{text}</button>"

    html = re.sub(r"<button([^>]*)>([^<]+)</button>", inject_btn, html, flags=re.IGNORECASE)
    return html


MENU_EN: dict[str, str] = {
    "menu.app": "AUDIO_HAXOR",
    "menu.about": "About AUDIO_HAXOR",
    "menu.preferences": "Preferences...",
    "menu.file": "File",
    "menu.scan_all": "Scan All",
    "menu.stop_all": "Stop All",
    "menu.export_plugins": "Export Plugins...",
    "menu.import_plugins": "Import Plugins...",
    "menu.export_samples": "Export Samples...",
    "menu.import_samples": "Import Samples...",
    "menu.export_daw": "Export DAW Projects...",
    "menu.import_daw": "Import DAW Projects...",
    "menu.export_presets": "Export Presets...",
    "menu.import_presets": "Import Presets...",
    "menu.edit": "Edit",
    "menu.find": "Find...",
    "menu.scan": "Scan",
    "menu.scan_plugins": "Scan Plugins",
    "menu.scan_samples": "Scan Samples",
    "menu.scan_daw": "Scan DAW Projects",
    "menu.scan_presets": "Scan Presets",
    "menu.check_updates": "Check Updates",
    "menu.view": "View",
    "menu.tab_plugins": "Plugins",
    "menu.tab_samples": "Samples",
    "menu.tab_daw": "DAW Projects",
    "menu.tab_audio_engine": "Audio Engine",
    "menu.tab_presets": "Presets",
    "menu.tab_favorites": "Favorites",
    "menu.tab_notes": "Notes",
    "menu.tab_history": "History",
    "menu.tab_settings": "Settings",
    "menu.tab_files": "Files",
    "menu.toggle_theme": "Toggle Light/Dark",
    "menu.toggle_crt": "Toggle CRT Effects",
    "menu.reset_columns": "Reset Column Widths",
    "menu.reset_tabs": "Reset Tab Order",
    "menu.playback": "Playback",
    "menu.play_pause": "Play / Pause",
    "menu.toggle_loop": "Toggle Loop",
    "menu.stop_playback": "Stop Playback",
    "menu.expand_player": "Expand / Collapse Player",
    "menu.next_track": "Next Track",
    "menu.prev_track": "Previous Track",
    "menu.toggle_shuffle": "Toggle Shuffle",
    "menu.toggle_mute": "Mute / Unmute",
    "menu.data": "Data",
    "menu.clear_history": "Clear All History...",
    "menu.clear_all_databases": "Clear All Databases",
    "menu.clear_kvr": "Clear KVR Cache...",
    "menu.clear_favorites": "Clear Favorites...",
    "menu.reset_all_scans": "Reset All Scans...",
    "menu.tools": "Tools",
    "menu.find_duplicates": "Find Duplicates",
    "menu.dep_graph": "Dependency Graph",
    "menu.cmd_palette": "Command Palette",
    "menu.context_settings_section_fallback": "Section",
    "menu.context_walker_fallback": "Walker",
    "menu.help_overlay": "Keyboard Shortcuts",
    "menu.window": "Window",
    "menu.help": "Help",
    "menu.github": "GitHub Repository",
    "menu.docs": "Documentation",
    "menu.plugins_with_updates": "{n} plugins with updates",
    "menu.plugins_with_updates_one": "1 plugin with updates",
    "menu.batch_selected": "{n} selected",
}

TRAY_EN: dict[str, str] = {
    "tray.show": "Show AUDIO_HAXOR",
    "tray.scan_all": "Scan All",
    "tray.stop_all": "Stop All",
    "tray.previous_track": "Previous Track",
    "tray.play_pause": "Play / Pause",
    "tray.next_track": "Next Track",
    "tray.quit": "Quit",
    "tray.tooltip": "AUDIO_HAXOR",
    "tray.status_playing": "Playing",
    "tray.status_paused": "Paused",
}

HELP_EN: dict[str, str] = {
    "help.title": "Keyboard Shortcuts",
    "help.close": "Close",
    "help.section.navigation": "Navigation",
    "help.section.playback": "Playback",
    "help.section.actions": "Actions",
    "help.section.fzf": "Search Operators (fzf)",
    "help.section.mouse": "Mouse",
    "help.section.more": "More shortcuts",
    "help.nav.switch_tabs": "Switch tabs",
    "help.nav.switch_tabs_1_9_desc": "Tabs 1–9 (Plugins … Tags)",
    "help.nav.switch_tabs_10_desc": "Tab 10 (Files)",
    "help.nav.switch_tabs_f_keys_desc": "History, Visualizer, Walkers",
    "help.nav.switch_tabs_settings_desc": "Settings",
    "help.nav.cmd_palette": "Command palette",
    "help.nav.focus_search": "Focus search",
    "help.nav.next_item": "Next item",
    "help.nav.prev_item": "Previous item",
    "help.nav.first_item": "First item",
    "help.nav.last_item": "Last item",
    "help.nav.half_down": "Half-page down",
    "help.nav.half_up": "Half-page up",
    "help.nav.focus_search_slash": "Focus search",
    "help.nav.open_activate": "Open / activate item",
    "help.nav.reveal_finder": "Reveal in Finder",
    "help.nav.yank": "Yank (copy path)",
    "help.nav.play_preview": "Play / preview",
    "help.nav.toggle_fav": "Toggle favorite",
    "help.nav.toggle_select": "Toggle select",
    "help.nav.select_all": "Select all",
    "help.play.pause": "Play / pause",
    "help.play.next": "Next track",
    "help.play.prev": "Previous track",
    "help.play.loop": "Toggle loop",
    "help.play.mute": "Mute / unmute",
    "help.play.vol_up": "Volume up",
    "help.play.vol_down": "Volume down",
    "help.act.scan_all": "Scan all",
    "help.act.stop_scans": "Stop all scans",
    "help.act.select_visible": "Select all visible",
    "help.act.export_tab": "Export current tab",
    "help.act.import_tab": "Import to current tab",
    "help.act.dupes": "Find duplicates",
    "help.act.deps": "Dependency graph",
    "help.act.theme": "Toggle theme",
    "help.act.prefs_file": "Open preferences file",
    "help.act.next_tab": "Next tab",
    "help.act.prev_tab": "Previous tab",
    "help.act.reveal": "Reveal in Finder",
    "help.act.copy_path": "Copy path",
    "help.act.toggle_fav": "Toggle favorite",
    "help.act.add_note": "Add note",
    "help.act.shuffle": "Toggle shuffle",
    "help.act.similar": "Find similar samples",
    "help.act.delete": "Delete selected",
    "help.act.esc": "Close / clear / stop",
    "help.act.toggle_help": "Toggle this help",
    "help.fzf.fuzzy": "Fuzzy match",
    "help.fzf.exact": "Exact substring",
    "help.fzf.prefix": "Starts with",
    "help.fzf.suffix": "Ends with",
    "help.fzf.exclude": "Exclude",
    "help.fzf.or": "OR match",
    "help.fzf.regex": "Toggle regex mode",
    "help.mouse.click": "Click",
    "help.mouse.dblclick": "Double-click",
    "help.mouse.right": "Right-click",
    "help.mouse.drag_tabs": "Drag tabs",
    "help.mouse.drag_player": "Drag player",
    "help.mouse.click_desc": "Play sample / expand metadata",
    "help.mouse.dblclick_desc": "Open in DAW / KVR / Finder",
    "help.mouse.right_desc": "Context menu everywhere",
    "help.mouse.drag_tabs_desc": "Reorder tabs",
    "help.mouse.drag_player_desc": "Dock to any corner",
}

# Dynamic / innerHTML strings (plugins.js, settings toggles, etc.)
UI_JS_EN: dict[str, str] = {
    "ui.js.loading_plugins": "Loading plugins...",
    "ui.js.scanning_for_plugins": "Scanning for plugins...",
    "ui.js.discovering_plugin_files": "Discovering plugin files across system directories...",
    "ui.js.no_plugins_found": "No Plugins Found",
    "ui.js.no_plugins_found_body": "No VST2, VST3, CLAP, or Audio Unit plugins were found in the standard system directories.",
    "ui.js.scan_error": "Scan Error",
    "ui.js.scanning_btn": "Scanning...",
    "ui.js.resuming_btn": "Resuming...",
    "ui.js.scan_plugins_btn": "Scan Plugins",
    "ui.js.no_matching_plugins": "No matching plugins",
    "ui.js.load_more_hint": "Showing {shown} of {total} — click to load more",
    "ui.js.no_mfg_website": "No manufacturer website",
    "ui.js.badge_update_available": "Update Available",
    "ui.js.badge_unknown_latest": "Unknown Latest",
    "ui.js.badge_up_to_date": "Up to Date",
    "ui.js.download": "Download",
    "ui.js.checking_updates_btn": "Checking...",
    "ui.js.check_updates_btn": "Check Updates",
    "ui.js.init_update_check": "Initializing update check...",
    "ui.js.searching_updates": "Searching for updates across {n} plugins...",
    "ui.js.status_checking_plugin": "Checking {mfg}{name} ({processed}/{total}){remaining}",
    "ui.js.remaining": " — {eta} remaining",
    "ui.js.stat_updates": "updates",
    "ui.js.stat_current": "current",
    "ui.js.stat_unknown": "unknown",
    "ui.js.stat_kvr_label": "KVR",
    "ui.js.stat_pending": "pending",
    "ui.js.batch_all_done": "All done!",
    "ui.js.batch_open_next": "Open Next Update",
    "ui.js.batch_all_done_btn": "All Done",
    "ui.js.batch_next": "Next: {name}",
    "ui.js.batch_progress": "{n} of {total}",
    "ui.js.theme_light": "Light",
    "ui.js.theme_dark": "Dark",
    "ui.js.toggle_on": "On",
    "ui.js.toggle_off": "Off",
}

SETTINGS_UI_EN: dict[str, str] = {
    "ui.settings.interface_language": "Interface language",
    "ui.settings.interface_language_desc": "UI text (restart the app to apply the native menu bar language)",
    "ui.opt.lang_de": "Deutsch",
    "ui.opt.lang_el": "Ελληνικά",
    "ui.opt.lang_en": "English",
    "ui.opt.lang_es": "Español",
    "ui.opt.lang_fr": "Français",
    "ui.opt.lang_it": "Italiano",
    "ui.opt.lang_nl": "Nederlands",
    "ui.opt.lang_pl": "Polski",
    "ui.opt.lang_pt": "Português",
    "ui.opt.lang_ru": "Русский",
    "ui.opt.lang_sv": "Svenska",
}

CONFIRM_EN: dict[str, str] = {
    "confirm.delete_smart_playlist": 'Delete "{name}"?',
    "confirm.delete_all_notes": "Delete all notes and tags?",
    "confirm.delete_tag_globally": 'Delete tag "{tag}" from all items?',
    "confirm.remove_tag_globally": 'Remove tag "{tag}" from all items?',
    "confirm.remove_all_favorites": "Remove all favorites?",
    "confirm.delete_file_browser": 'Delete "{name}"? This cannot be undone.',
    "confirm.delete_shortcuts": 'Delete "{name}"?',
    "confirm.delete_data_file": "Delete {name}?",
}


def validate_keys(merged: dict[str, str]) -> None:
    for k in merged:
        if not any(k.startswith(p) for p in ALLOWED_PREFIXES):
            raise SystemExit(f"invalid key prefix: {k}")


def main() -> None:
    toast_en = load_toast_en()
    html_path = ROOT / "frontend" / "index.html"
    html = html_path.read_text(encoding="utf-8")
    out_path = I18N_DIR / "app_i18n_en.json"
    prev: dict[str, str] = {}
    prev_ui: dict[str, str] = {}
    if out_path.exists():
        try:
            prev = json.loads(out_path.read_text(encoding="utf-8"))
            prev_ui = {k: v for k, v in prev.items() if k.startswith("ui.")}
        except (OSError, json.JSONDecodeError):
            prev = {}
            prev_ui = {}
    ui_fresh = extract_ui_from_html(html)
    ui_from_attrs = extract_existing_data_i18n(html)
    ui_en = {**prev_ui, **ui_fresh, **ui_from_attrs}

    merged: dict[str, str] = {}
    merged.update(toast_en)
    merged.update(MENU_EN)
    merged.update(TRAY_EN)
    merged.update(HELP_EN)
    merged.update(CONFIRM_EN)
    merged.update(ui_en)
    merged.update(UI_JS_EN)
    merged.update(SETTINGS_UI_EN)
    # Locale <option> text also creates ui.opt.english etc.; canonical keys are ui.opt.lang_*.
    for dup in (
        "ui.opt.english",
        "ui.opt.deutsch",
        "ui.opt.espa_ol",
        "ui.opt.svenska",
        "ui.opt.fran_ais",
        "ui.opt.portugu_s",
        "ui.opt.nederlands",
        "ui.opt.italiano",
        "ui.opt.ellinika",
        "ui.opt.polski",
        "ui.opt.russkiy",
    ):
        merged.pop(dup, None)
    merged.pop("ui.tt.interface_language", None)

    # Keys from merge_i18n_keys batches / manual edits (not in toast/MENU/HELP/CONFIRM/ui
    # sections above) must survive regeneration — otherwise context menu and JS-only strings
    # disappear when MENU_EN / CONFIRM_EN are incomplete.
    for k, v in prev.items():
        if k not in merged:
            merged[k] = v

    for k, v in list(merged.items()):
        if isinstance(v, str):
            merged[k] = sanitize_catalog_value(v)

    overlap = set(toast_en) & (
        set(MENU_EN)
        | set(TRAY_EN)
        | set(HELP_EN)
        | set(CONFIRM_EN)
        | set(UI_JS_EN)
        | set(SETTINGS_UI_EN)
        | set(ui_en)
    )
    if overlap:
        raise SystemExit(f"duplicate keys across sections: {sorted(overlap)[:20]}")

    validate_keys(merged)

    text = json.dumps(merged, ensure_ascii=False, indent=2, sort_keys=True) + "\n"
    out_path.write_text(text, encoding="utf-8")
    print(
        f"Wrote {len(merged)} entries ({len(toast_en)} toast + {len(ui_en)} ui) to {out_path}",
        file=sys.stderr,
    )

    new_html = inject_open_tags(html, ui_en)
    if new_html != html:
        html_path.write_text(new_html, encoding="utf-8")
        print(f"Injected data-i18n* attributes into {html_path}", file=sys.stderr)


if __name__ == "__main__":
    main()
