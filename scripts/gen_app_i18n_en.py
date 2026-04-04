#!/usr/bin/env python3
"""Generate src-tauri/app_i18n_en.json: toasts + menus + tray + HTML + help + confirm dialogs.

Run from repo root: python3 scripts/gen_app_i18n_en.py
Optionally inject data-i18n* attributes into frontend/index.html (idempotent).
"""
from __future__ import annotations

import importlib.util
import json
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]

ALLOWED_PREFIXES = ("toast.", "menu.", "tray.", "ui.", "help.", "confirm.")


def load_toast_en() -> dict[str, str]:
    p = ROOT / "scripts" / "gen_toast_i18n_en.py"
    spec = importlib.util.spec_from_file_location("toast_src", p)
    if spec is None or spec.loader is None:
        raise SystemExit(f"cannot load {p}")
    m = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(m)
    return dict(m.TOAST_EN)


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


def extract_ui_from_html(html: str) -> dict[str, str]:
    """Map i18n key -> English text for placeholders, titles, and <option> labels."""
    out: dict[str, str] = {}
    seen_ph: dict[str, int] = {}
    seen_tt: dict[str, int] = {}
    seen_opt: dict[str, int] = {}

    for m in re.finditer(r'placeholder="([^"]*)"', html):
        val = m.group(1)
        if not val.strip():
            continue
        k = alloc_key("ui.ph", val, seen_ph)
        out[k] = val

    for m in re.finditer(r' title="([^"]*)"', html):
        val = m.group(1)
        if not val.strip():
            continue
        k = alloc_key("ui.tt", val, seen_tt)
        out[k] = val

    for m in re.finditer(r"<option[^>]*>([^<]+)</option>", html):
        val = m.group(1).strip()
        if not val:
            continue
        k = alloc_key("ui.opt", val, seen_opt)
        out[k] = val

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
    for k, v in ui.items():
        if k.startswith("ui.ph."):
            ph_vk.setdefault(v, k)
        elif k.startswith("ui.tt."):
            tt_vk.setdefault(v, k)
        elif k.startswith("ui.opt."):
            opt_vk.setdefault(v, k)

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
    "menu.clear_kvr": "Clear KVR Cache...",
    "menu.clear_favorites": "Clear Favorites...",
    "menu.reset_all_scans": "Reset All Scans...",
    "menu.tools": "Tools",
    "menu.find_duplicates": "Find Duplicates",
    "menu.dep_graph": "Dependency Graph",
    "menu.cmd_palette": "Command Palette",
    "menu.help_overlay": "Keyboard Shortcuts",
    "menu.window": "Window",
    "menu.help": "Help",
    "menu.github": "GitHub Repository",
    "menu.docs": "Documentation",
}

TRAY_EN: dict[str, str] = {
    "tray.show": "Show AUDIO_HAXOR",
    "tray.scan_all": "Scan All",
    "tray.stop_all": "Stop All",
    "tray.play_pause": "Play / Pause",
    "tray.next_track": "Next Track",
    "tray.quit": "Quit",
    "tray.tooltip": "AUDIO_HAXOR",
}

HELP_EN: dict[str, str] = {
    "help.title": "Keyboard Shortcuts",
    "help.close": "Close",
    "help.section.navigation": "Navigation",
    "help.section.playback": "Playback",
    "help.section.actions": "Actions",
    "help.section.fzf": "Search Operators (fzf)",
    "help.section.mouse": "Mouse",
    "help.nav.switch_tabs": "Switch tabs",
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

SETTINGS_UI_EN: dict[str, str] = {
    "ui.settings.interface_language": "Interface language",
    "ui.settings.interface_language_desc": "UI text (restart the app to apply the native menu bar language)",
    "ui.opt.lang_en": "English",
    "ui.opt.lang_de": "Deutsch",
    "ui.opt.lang_es": "Español",
    "ui.opt.lang_sv": "Svenska",
    "ui.opt.lang_fr": "Français",
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
    ui_en = extract_ui_from_html(html)

    merged: dict[str, str] = {}
    merged.update(toast_en)
    merged.update(MENU_EN)
    merged.update(TRAY_EN)
    merged.update(HELP_EN)
    merged.update(CONFIRM_EN)
    merged.update(ui_en)
    merged.update(SETTINGS_UI_EN)
    # Locale <option> text also creates ui.opt.english etc.; canonical keys are ui.opt.lang_*.
    for dup in (
        "ui.opt.english",
        "ui.opt.deutsch",
        "ui.opt.espa_ol",
        "ui.opt.svenska",
        "ui.opt.fran_ais",
    ):
        merged.pop(dup, None)
    merged.pop("ui.tt.interface_language", None)

    overlap = set(toast_en) & (
        set(MENU_EN) | set(TRAY_EN) | set(HELP_EN) | set(CONFIRM_EN) | set(SETTINGS_UI_EN) | set(ui_en)
    )
    if overlap:
        raise SystemExit(f"duplicate keys across sections: {sorted(overlap)[:20]}")

    validate_keys(merged)

    out = ROOT / "src-tauri" / "app_i18n_en.json"
    text = json.dumps(merged, ensure_ascii=False, indent=2, sort_keys=True) + "\n"
    out.write_text(text, encoding="utf-8")
    print(
        f"Wrote {len(merged)} entries ({len(toast_en)} toast + {len(ui_en)} ui html) to {out}",
        file=sys.stderr,
    )

    new_html = inject_open_tags(html, ui_en)
    if new_html != html:
        html_path.write_text(new_html, encoding="utf-8")
        print(f"Injected data-i18n* attributes into {html_path}", file=sys.stderr)


if __name__ == "__main__":
    main()
