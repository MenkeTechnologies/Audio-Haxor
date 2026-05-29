/**
 * Drag/drop toast coverage invariants.
 *
 * Every drag-reorder / drag-drop / drag-out surface in the frontend
 * MUST emit a toast on a successful gesture. These tests pin every
 * surface so a future refactor that removes a toast emit is caught.
 *
 * The audit was completed in v1.28.13; these tests are the regression
 * net so the audit conclusions cannot silently rot.
 */
const fs = require('fs');
const path = require('path');
const { describe, it } = require('node:test');
const assert = require('node:assert/strict');

const FRONT = path.join(__dirname, '..', 'frontend', 'js');
const read = (rel) => fs.readFileSync(path.join(FRONT, rel), 'utf8');

describe('drag/drop toast coverage (v1.28.13 audit)', () => {

  it('drag-reorder.js universal initDragReorder mouseup emits toastKey / toastName / toast.reordered', () => {
    const src = read('drag-reorder.js');
    // The mouseup body must reference both showToast and a toastFmt path
    // that resolves to a generic reordered key when no toastKey is set.
    const mouseupIdx = src.indexOf("addEventListener('mouseup'");
    assert.ok(mouseupIdx > 0, 'expected initDragReorder mouseup listener');
    const body = src.slice(mouseupIdx, mouseupIdx + 3000);
    assert.match(body, /d\.toastKey/, 'must read d.toastKey from drag state');
    assert.match(body, /d\.toastName/, 'must read d.toastName from drag state');
    assert.match(body, /toast\.reordered/, 'must fall through to toast.reordered');
    assert.match(body, /showToast\(toastFmt\(/, 'must emit via showToast(toastFmt(...))');
  });

  it('drag-reorder.js table column reorder mouseup emits toast.reordered_columns', () => {
    const src = read('drag-reorder.js');
    const block = src.match(/Table column reorder[\s\S]+?_colDrag\s*=\s*null;\s*\}\);/);
    assert.ok(block, 'expected table column reorder IIFE');
    assert.match(block[0], /toast\.reordered_columns/, 'column reorder must emit toast.reordered_columns');
    assert.match(block[0], /showToast\(toastFmt/, 'must use showToast(toastFmt(...))');
  });

  it('drag-reorder.js initFloatingElement onUp emits toast.relocated_button_group', () => {
    const src = read('drag-reorder.js');
    // The onUp handler must emit relocated_button_group inside the
    // dropTarget block — fired only when a real drop landed.
    const fnStart = src.indexOf('function initFloatingElement');
    assert.ok(fnStart > 0, 'expected initFloatingElement');
    // Walk forward to the end of the function (matching brace).
    const fnEnd = src.indexOf('\n}', fnStart);
    assert.ok(fnEnd > fnStart, 'expected closing brace of initFloatingElement');
    const body = src.slice(fnStart, fnEnd);
    assert.match(body, /toast\.relocated_button_group/, 'must emit toast.relocated_button_group');
  });

  it('utils.js main app tab bar drag mouseup emits toast.reordered_main_tabs', () => {
    const src = read('utils.js');
    assert.match(src, /toast\.reordered_main_tabs/, 'main tab bar must emit toast.reordered_main_tabs');
    // The emit must be inside an `isDragging` guard so click-without-drag
    // does not produce a spurious toast.
    const emitIdx = src.indexOf('toast.reordered_main_tabs');
    const slice = src.slice(Math.max(0, emitIdx - 800), emitIdx);
    assert.match(slice, /isDragging/, 'emit must be guarded by isDragging');
  });

  it('audio.js Similar panel dock change emits toast.similar_panel_docked', () => {
    const src = read('audio.js');
    assert.match(src, /toast\.similar_panel_docked/, 'Similar panel dock must emit toast.similar_panel_docked');
    // Adjacent context must include similarDock pref save.
    const i = src.indexOf('toast.similar_panel_docked');
    const slice = src.slice(Math.max(0, i - 400), i);
    assert.match(slice, /similarDock/, 'must save similarDock pref alongside emit');
  });

  it('audio.js floating now-playing player dock change emits toast.player_docked', () => {
    const src = read('audio.js');
    assert.match(src, /toast\.player_docked/, 'player dock must emit toast.player_docked');
    const i = src.indexOf('toast.player_docked');
    const slice = src.slice(Math.max(0, i - 600), i);
    assert.match(slice, /modal_audioNowPlaying/, 'must save dock geometry alongside emit');
  });

  it('file-browser.js cross-pane drop emits toast.fb_drop_copied_n / _moved_n / _failed', () => {
    const src = read('file-browser.js');
    assert.match(src, /toast\.fb_drop_copied_n/, 'must emit copied toast');
    assert.match(src, /toast\.fb_drop_moved_n/, 'must emit moved toast');
    assert.match(src, /toast\.fb_drop_failed/, 'must emit failed toast');
  });

  it('native-file-drag.js startNativeFileDrag emits toast.fb_drag_started_n on success', () => {
    const src = read('native-file-drag.js');
    assert.match(src, /toast\.fb_drag_started_n/, 'must emit drag-started toast');
    // Must be AFTER the `await tauri.drag.startDrag(...)` call (only on success).
    const fnIdx = src.indexOf('async function startNativeFileDrag');
    assert.ok(fnIdx > 0);
    const fnBody = src.slice(fnIdx, fnIdx + 2000);
    const awaitIdx = fnBody.indexOf('await tauri.drag.startDrag');
    const toastIdx = fnBody.indexOf('toast.fb_drag_started_n');
    assert.ok(awaitIdx > 0 && toastIdx > awaitIdx, 'success toast must follow the awaited startDrag call');
    // Must be inside the try (not the catch) so failures use the error path.
    const tryEnd = fnBody.indexOf('} catch (err) {');
    assert.ok(toastIdx < tryEnd, 'success toast must live in the try block, not the catch');
  });

});

describe('every documented reorder toast key is present in app_i18n_en.json', () => {
  const en = JSON.parse(fs.readFileSync(
    path.join(__dirname, '..', 'i18n', 'app_i18n_en.json'),
    'utf8',
  ));
  const REQUIRED = [
    'toast.reordered',
    'toast.reordered_panes',
    'toast.reordered_tabs',
    'toast.reordered_columns',
    'toast.reordered_main_tabs',
    'toast.reordered_stats',
    'toast.reordered_audio_stats',
    'toast.reordered_fav_chips',
    'toast.reordered_fav_items',
    'toast.reordered_toolbar',
    'toast.reordered_settings_sections',
    'toast.reordered_audio_engine_sections',
    'toast.reordered_notes',
    'toast.reordered_tag_cards',
    'toast.reordered_smart_playlists',
    'toast.reordered_walker_tiles',
    'toast.reordered_viz_tiles',
    'toast.reordered_recently_played',
    'toast.reordered_shortcut_rows',
    'toast.reordered_np_sections',
    'toast.reordered_heatmap',
    'toast.fb_drop_copied_n',
    'toast.fb_drop_moved_n',
    'toast.fb_drop_failed',
    'toast.fb_drag_started_n',
    'toast.relocated_button_group',
    'toast.similar_panel_docked',
    'toast.player_docked',
  ];
  for (const k of REQUIRED) {
    it(`has English value for ${k}`, () => {
      assert.ok(en[k] && en[k].trim().length > 0, `${k} missing or empty in app_i18n_en.json`);
    });
  }
});
