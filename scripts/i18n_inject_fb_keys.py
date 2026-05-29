#!/usr/bin/env python3
"""One-shot injector for the file-browser power-feature i18n keys.

Adds each `(key, english_value)` pair to every `i18n/app_i18n_*.json`
catalog. Non-EN locales get the English text as a placeholder — the
existing `scripts/gen_all_app_i18n_locales.py` translator pipeline can
backfill real translations later. Idempotent: keys already present are
overwritten so re-running with a corrected English string takes effect.

Run from repo root:  python3 scripts/i18n_inject_fb_keys.py
"""

from __future__ import annotations

import json
import pathlib

ROOT = pathlib.Path(__file__).resolve().parents[1]
I18N_DIR = ROOT / "i18n"

# Every English string the file-browser power-feature batches added.
# Organized by namespace so cross-locale review is easier later.
NEW_KEYS: dict[str, str] = {
    # ── Reorder toasts (universal — every initDragReorder caller) ──
    "toast.reordered": "Reordered",
    "toast.reordered_panes": "File-browser panes reordered",
    "toast.reordered_tabs": "File-browser tabs reordered",
    "toast.reordered_stats": "Stats bar reordered",
    "toast.reordered_header_stats": "Header stats reordered",
    "toast.reordered_audio_stats": "Audio stats reordered",
    "toast.reordered_fav_chips": "Favorite chips reordered",
    "toast.reordered_fav_items": "Favorites reordered",
    "toast.reordered_toolbar": "Toolbar reordered",
    "toast.reordered_settings_sections": "Settings sections reordered",
    "toast.reordered_audio_engine_sections": "Audio Engine sections reordered",
    "toast.reordered_notes": "Notes reordered",
    "toast.reordered_tag_cards": "Tag cards reordered",
    "toast.reordered_smart_playlists": "Smart playlists reordered",
    "toast.reordered_walker_tiles": "Walker tiles reordered",
    "toast.reordered_viz_tiles": "Visualizer tiles reordered",
    "toast.reordered_recently_played": "Recently played reordered",
    "toast.reordered_shortcut_rows": "Shortcut rows reordered",
    "toast.reordered_np_sections": "Player sections reordered",
    "toast.reordered_heatmap": "Heatmap tiles reordered",
    # OS-drop receiver (cross-pane file move/copy via native drag).
    "toast.fb_drop_copied_n": "Copied {n} item(s) → Pane {pane}",
    "toast.fb_drop_moved_n": "Moved {n} item(s) → Pane {pane}",
    "toast.fb_drop_failed": "Drop failed: {err}",
    "toast.reordered_columns": "Table columns reordered",
    # Native OS drag-out (Finder / Desktop / DAW drop target). Fires
    # from `startNativeFileDrag` after `tauri-plugin-drag` accepts the
    # drag payload. The destination app is opaque to us so the message
    # only reports the count.
    "toast.fb_drag_started_n": "Started OS drag with {n} item(s)",
    # Replacements for raw-English `toast.fb_action` calls — each
    # gets its own catalog entry with proper placeholders so the
    # message can be translated.
    "toast.fb_copied_to_pane_n": "Copied {n} item(s) → Pane {pane}",
    "toast.fb_moved_to_pane_n": "Moved {n} item(s) → Pane {pane}",
    "toast.fb_swapped_panes": "Swapped Pane {a} ↔ Pane {b}",
    "toast.fb_show_hidden_active_pane": "Showing hidden files in active pane",
    "toast.fb_hide_hidden_active_pane": "Hiding hidden files in active pane",
    "toast.fb_renamed_n": "Renamed {n} file(s)",
    "toast.fb_renamed_to": "Renamed to {name}",
    "toast.fb_created_name": "Created {name}",
    "toast.fb_clipboard_marked": "{verb} {target} — paste in target folder",
    "toast.fb_pasted_n": "Pasted {n} item(s)",
    "toast.fb_moved_n_into": "Moved {n} item(s) into {folder}",
    "toast.fb_chmod_on_n": "chmod {mode} on {n} file(s)",
    "toast.fb_chmod_one": "chmod {mode} {name}",
    "toast.fb_symlink_repointed": "Re-pointed → {target}",
    "toast.fb_selected_matching": "Selected {n} matching {pattern}",
    "toast.fb_compressed_to": "Compressed → {name}",
    "toast.fb_extracted_to": "Extracted → {name}",
    "toast.fb_extracted_n_archives": "Extracted {n} archive(s)",
    "toast.fb_touched_n": "Touched {n} item(s)",
    "toast.fb_aliased_to": "Aliased → {name}",
    "toast.fb_duplicated_to": "Duplicated → {name}",
    "toast.fb_running_name": "Running {name}",
    "toast.fb_opening_in_bin": "Opening in {bin}",
    "toast.fb_label_applied_n": "Labeled {n} item(s) {label}",
    "toast.fb_chmod_n_files_done": "chmod {mode} on {n} file(s)",
    "toast.fb_saved_n_bookmarks": "Saved {n} bookmark(s)",
    "toast.fb_moved_path_to": "Moved {name} → {target}",
    "toast.fb_trashed_n_dupes": "Trashed {n} duplicate(s)",
    "toast.fb_permanently_deleted_n": "Permanently deleted {n} item(s)",
    "toast.fb_sync_scroll_on": "Sync scroll ON",
    "toast.fb_sync_scroll_off": "Sync scroll OFF",
    "toast.fb_moved_to_trash_name": "Moved \"{name}\" to Trash",
    "toast.fb_moved_n_to_trash": "Moved {n} item(s) to Trash",
    "toast.fb_failed_n": "{n} item(s) failed",
    # ── Toasts ──
    # Generic "action done" toast — replaces my abuse of
    # `toast.deleted_name` ("Deleted {name}") as a status message,
    # which made "opening in nvim" render as "Deleted opening in nvim"
    # and made users think their file got deleted.
    "toast.fb_action": "{name}",
    # ── Row context menu (per-file ops) ──
    "menu.fb_quick_look": "Quick Look",
    "menu.fb_get_info": "Get Info",
    "menu.fb_hash_sha256": "Hash (SHA-256)",
    "menu.fb_permissions": "Permissions…",
    "menu.fb_touch": "Touch (set mtime to now)",
    "menu.fb_edit_symlink": "Edit Symlink Target…",
    "menu.fb_copy_file": "Copy File",
    "menu.fb_cut_file": "Cut File",
    "menu.fb_make_alias": "Make Alias",
    "menu.fb_duplicate": "Duplicate",
    "menu.fb_compress_name": "Compress \"{name}\"",
    "menu.fb_extract_here": "Extract Here",
    "menu.fb_move_to_trash": "Move to Trash",
    "menu.fb_delete_permanently": "Delete Permanently",
    "menu.fb_secure_delete": "Secure Delete (overwrite with zeros + unlink)",
    "confirm.fb_secure_delete_title": "Secure Delete",
    "confirm.fb_secure_delete_body": "Overwrite every byte of \"{name}\" with zeros, sync to disk, then unlink? On HDD + ext4/NTFS this defeats simple undeletion. On SSDs and copy-on-write filesystems (APFS, btrfs, zfs) the guarantee is weaker — the original physical cells may survive until garbage-collection or snapshot expiry. NOT undoable.",
    "menu.fb_open_external_editor": "Open in External Editor",
    "menu.fb_run_as_program": "Run as Program",
    # ── Color labels (8 entries) ──
    "menu.fb_label_prefix": "Label",
    "menu.fb_label_none": "None",
    "menu.fb_label_red": "Red",
    "menu.fb_label_orange": "Orange",
    "menu.fb_label_yellow": "Yellow",
    "menu.fb_label_green": "Green",
    "menu.fb_label_cyan": "Cyan",
    "menu.fb_label_purple": "Purple",
    "menu.fb_label_gray": "Gray",
    # ── Empty-space context menu ──
    "menu.fb_new_folder": "New Folder",
    "menu.fb_new_file": "New File",
    "menu.fb_paste_n_here": "Paste {n} item(s) here",
    "menu.fb_move_n_here": "Move {n} item(s) here",
    "menu.fb_new_folder_with_selection": "New Folder with {n} Item(s)",
    "menu.fb_refresh": "Refresh",
    "menu.fb_open_terminal": "Open in Terminal",
    "menu.fb_open_default_app": "Open in Default App",
    "menu.fb_open": "Open",
    "menu.fb_open_folder": "Open Folder",
    "menu.fb_open_directory": "Open Directory",
    "menu.fb_reveal_in_finder": "Reveal in Finder",
    "menu.fb_copy_path": "Copy Path",
    "menu.fb_copy_name": "Copy Name",
    "menu.fb_rename": "Rename (F2)",
    "menu.fb_move_to_label": "Move to →",
    "menu.fb_show_hidden": "Show Hidden Files",
    "menu.fb_hide_hidden": "Hide Hidden Files",
    "menu.fb_show_tree_sidebar": "Show Tree Sidebar",
    "menu.fb_hide_tree_sidebar": "Hide Tree Sidebar",
    "menu.fb_panes_n": "Panes: {n}/4 (Cmd+\\ to cycle)",
    "menu.fb_swap_panes": "Swap Pane {a} ⇄ Pane {b}",
    "menu.fb_sync_scroll_on": "Sync Scroll: ON",
    "menu.fb_sync_scroll_off": "Sync Scroll: OFF",
    "menu.fb_copy_to_next_pane": "Copy Active Selection → Next Pane",
    "menu.fb_move_to_next_pane": "Move Active Selection → Next Pane",
    "menu.fb_find_in_files": "Find in Files… (grep)",
    "menu.fb_find_duplicates": "Find Duplicates… (by content)",
    "menu.fb_quick_open": "Quick Open…",
    "menu.fb_manage_bookmarks": "Manage Bookmarks…",
    "menu.fb_spotlight": "Spotlight — search all inventory",
    "menu.fb_compare_with_other_pane": "Compare with Other Pane (folder tree diff)",
    "menu.fb_diff_pair": "Diff {a} ⇄ {b}",
    "menu.fb_select_by_pattern": "Select by Pattern…",
    "menu.fb_invert_selection": "Invert Selection",
    "menu.fb_hash_n": "Hash {n} Item(s)",
    "menu.fb_chmod_n": "Permissions on {n} Item(s)…",
    "menu.fb_touch_n": "Touch {n} Item(s)",
    "menu.fb_compress_n": "Compress {n} Item(s) into Archive…",
    "menu.fb_extract_n": "Extract {n} Archive(s) Here",
    # ── Modal headers ──
    "ui.fb_modal_get_info": "Get Info — {name}",
    "ui.fb_modal_hash": "SHA-256",
    "ui.fb_modal_hash_n": "SHA-256 ({n} files)",
    "ui.fb_modal_chmod": "Permissions — {name}",
    "ui.fb_modal_chmod_n": "Permissions — {n} files",
    "ui.fb_modal_diff": "Diff — {a} ⇄ {b}",
    "ui.fb_modal_compare": "Compare Folders",
    "ui.fb_modal_grep": "Find in Files — {dir}",
    "ui.fb_modal_duplicates": "Find Duplicates — {dir}",
    "ui.fb_modal_spotlight": "Spotlight — search all scanned inventory",
    "ui.fb_modal_quick": "Quick Open — recent folders & files",
    "ui.fb_modal_bookmarks": "Bookmarks ({n})",
    "ui.fb_modal_symlink": "Edit Symlink — {name}",
    # ── Modal action buttons ──
    "ui.fb_btn_apply": "Apply",
    "ui.fb_btn_apply_all": "Apply to all",
    "ui.fb_btn_cancel": "Cancel",
    "ui.fb_btn_close": "Close",
    "ui.fb_btn_save": "Save",
    "ui.fb_btn_scan": "Scan",
    "ui.fb_btn_search": "Search",
    "ui.fb_btn_copy_all": "Copy All",
    "ui.fb_btn_repoint": "Re-point",
    "ui.fb_btn_fit": "Fit",
    "ui.fb_btn_actual": "100%",
    "ui.fb_btn_zoom_in": "Zoom in",
    "ui.fb_btn_zoom_out": "Zoom out",
    "ui.fb_btn_fit_tip": "Fit to pane",
    "ui.fb_btn_actual_tip": "Actual size",
    # ── Modal labels / fields / placeholders ──
    "ui.fb_lbl_octal_mode": "Octal mode (e.g. 0644, 755).",
    "ui.fb_lbl_octal_mode_current": "Current: {mode}",
    "ui.fb_lbl_pattern": "Glob pattern (e.g. *.wav, song-??.mp3):",
    "ui.fb_lbl_folder_name": "Folder name:",
    "ui.fb_lbl_file_name": "New file name:",
    "ui.fb_lbl_recursive": "Recursive (walk subfolders)",
    "ui.fb_lbl_case_insensitive": "Case insensitive",
    "ui.fb_lbl_min_size": "Min size",
    "ui.fb_lbl_files_identical": "files are identical",
    "ui.fb_lbl_trees_identical": "trees are identical",
    "ui.fb_lbl_only_in_a": "Only in A",
    "ui.fb_lbl_only_in_b": "Only in B",
    "ui.fb_lbl_different_content": "Different content (same path)",
    "ui.fb_lbl_no_matches": "no matches",
    "ui.fb_lbl_no_duplicates": "no duplicates found",
    "ui.fb_lbl_searching": "searching…",
    "ui.fb_lbl_scanning": "scanning…",
    "ui.fb_lbl_computing_diff": "computing diff…",
    "ui.fb_lbl_comparing": "comparing…",
    "ui.fb_lbl_loading": "loading…",
    "ui.fb_lbl_no_input": "Type ≥ 3 chars for FTS, 1-2 for LIKE fallback…",
    "ui.fb_placeholder_fuzzy": "Fuzzy search…",
    "ui.fb_placeholder_spotlight": "search audio, DAW, presets, MIDI, PDFs, videos…",
    "ui.fb_placeholder_search_text": "search text…",
    "ui.fb_placeholder_default_folder": "untitled folder",
    "ui.fb_placeholder_default_file": "untitled.txt",
    # ── Command palette entries (25) ──
    "menu.fb_cp_quick_open": "File browser: Quick Open (recent files/folders)",
    "menu.fb_cp_spotlight": "File browser: Spotlight (search all inventory)",
    "menu.fb_cp_manage_bookmarks": "File browser: Manage Bookmarks",
    "menu.fb_cp_toggle_tree": "File browser: Toggle Tree Sidebar",
    "menu.fb_cp_toggle_hidden": "File browser: Toggle Hidden Files",
    "menu.fb_cp_clear_label_filter": "File browser: Filter — clear color label",
    "menu.fb_cp_invert_selection": "File browser: Invert Selection",
    "menu.fb_cp_select_pattern": "File browser: Select by Pattern",
    "menu.fb_cp_cycle_panes": "File browser: Cycle Pane Count (1 → 2 → 3 → 4)",
    "menu.fb_cp_toggle_sync_scroll": "File browser: Toggle Sync Scroll across panes",
    "menu.fb_cp_copy_next_pane": "File browser: Copy Active Selection → Next Pane",
    "menu.fb_cp_move_next_pane": "File browser: Move Active Selection → Next Pane",
    "menu.fb_cp_swap_panes": "File browser: Swap Active Pane ↔ Next Pane",
    "menu.fb_cp_new_folder": "File browser: New Folder",
    "menu.fb_cp_new_file": "File browser: New File",
    "menu.fb_cp_paste": "File browser: Paste from File Clipboard",
    "menu.fb_cp_new_folder_with_selection": "File browser: New Folder with Selection",
    "menu.fb_cp_grep": "File browser: Find in Files (grep contents)",
    "menu.fb_cp_find_duplicates": "File browser: Find Duplicates (by content)",
    "menu.fb_cp_diff": "File browser: Diff Two Selected Files",
    "menu.fb_cp_compare_folders": "File browser: Compare Folders (active ↔ next pane)",
    "menu.fb_cp_hash_selected": "File browser: Hash Selected (SHA-256)",
    "menu.fb_cp_chmod_selected": "File browser: Permissions (chmod) on Selection",
    "menu.fb_cp_touch_selected": "File browser: Touch (set mtime) on Selection",
    "menu.fb_cp_compress_selected": "File browser: Compress Selection (zip)",
    "menu.fb_cp_extract_selected": "File browser: Extract Selected Archives Here",
    # Current-folder actions — operate on the active pane's path.
    "menu.fb_cp_refresh": "File browser: Refresh current folder",
    "menu.fb_cp_open_in_terminal_current": "File browser: Open current folder in Terminal",
    "menu.fb_cp_reveal_current": "File browser: Reveal current folder in Finder",
    "menu.fb_cp_copy_current_path": "File browser: Copy current folder path",
    "menu.fb_cp_bookmark_current": "File browser: Bookmark current folder",
    "menu.fb_cp_up_one_folder": "File browser: Go up one folder",
    "menu.fb_cp_go_home": "File browser: Go to Home folder",
    "menu.fb_cp_select_all": "File browser: Select all visible",
    "menu.fb_cp_clear_selection": "File browser: Clear selection",
}


def main() -> None:
    locale_paths = sorted(I18N_DIR.glob("app_i18n_*.json"))
    if not locale_paths:
        raise SystemExit(f"no locale files under {I18N_DIR}")
    added_total = 0
    updated_total = 0
    for p in locale_paths:
        with p.open() as fh:
            cat = json.load(fh)
        before = len(cat)
        new_or_updated = 0
        for k, v in NEW_KEYS.items():
            if k not in cat or cat[k] != v:
                cat[k] = v
                new_or_updated += 1
        after = len(cat)
        added = after - before
        updated = new_or_updated - added
        added_total += added
        updated_total += updated
        # Re-sort alphabetically + 2-space indent (preserves existing JSON
        # style across the repo).
        ordered = {k: cat[k] for k in sorted(cat)}
        with p.open("w") as fh:
            json.dump(ordered, fh, indent=2, ensure_ascii=False)
            fh.write("\n")
        print(f"{p.name:24} +{added}  ~{updated}")
    print(f"\ntotal: +{added_total} new, ~{updated_total} updated across {len(locale_paths)} locales")


if __name__ == "__main__":
    main()
