/**
 * Mirrors `src-tauri/src/app_i18n.rs` seed invariants that are not covered by
 * `i18n-locales-and-shape.test.js` / `i18n-placeholders.test.js`:
 * - `seed_json_appfmt_placeholders_preserved_…_uk_vi_id_hi` (Rust; here: strict token multiset parity for `de`/`el`/`es`/`es_419`/`fi`/`fr`/`it`/`nl`/`pl`/`pt`/`pt_br`/`ru`/`sv`/`zh`/`ja`/`ko`/`da`/`nb`/`tr`/`cs`/`hu`/`ro`/`uk`/`vi`/`id`/`hi`; JS multiset is stricter than Rust substring check)
 * - `seed_json_es_critical_prefixes_match_en_placeholders` + `seed_json_es_419_critical_prefixes_match_en_placeholders`
 * - `seed_json_en_defines_all_native_menu_bar_keys` + `seed_json_en_defines_all_tray_keys`
 *
 * Keep `NATIVE_MENU_BAR_KEYS` / `TRAY_KEYS` in sync with `app_i18n.rs` `#[cfg(test)]` consts.
 */
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { dirname, join } from 'path';
import test from 'node:test';
import { fileURLToPath } from 'url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const i18nDir = join(root, 'i18n');

/** Same token capture as `ipc.js` `appFmt` / `i18n-placeholders.test.js` (`\{\w+\}`). */
const IPC_PLACEHOLDER = /\{\w+\}/g;

/** Rust `app_i18n::tests` uses this for ES critical-prefix checks. */
const RUST_PLACEHOLDER = /\{[a-zA-Z_][a-zA-Z0-9_]*\}/g;

/** @see `src-tauri/src/app_i18n.rs` — `NATIVE_MENU_BAR_KEYS` */
const NATIVE_MENU_BAR_KEYS = [
  'menu.about',
  'menu.app',
  'menu.check_updates',
  'menu.clear_favorites',
  'menu.clear_history',
  'menu.clear_kvr',
  'menu.cmd_palette',
  'menu.data',
  'menu.dep_graph',
  'menu.docs',
  'menu.edit',
  'menu.expand_player',
  'menu.export_daw',
  'menu.export_plugins',
  'menu.export_presets',
  'menu.export_samples',
  'menu.file',
  'menu.find',
  'menu.find_duplicates',
  'menu.github',
  'menu.help',
  'menu.help_overlay',
  'menu.import_daw',
  'menu.import_plugins',
  'menu.import_presets',
  'menu.import_samples',
  'menu.next_track',
  'menu.play_pause',
  'menu.playback',
  'menu.preferences',
  'menu.prev_track',
  'menu.reset_all_scans',
  'menu.reset_columns',
  'menu.reset_tabs',
  'menu.scan',
  'menu.scan_all',
  'menu.scan_daw',
  'menu.scan_plugins',
  'menu.scan_presets',
  'menu.scan_samples',
  'menu.stop_all',
  'menu.stop_playback',
  'menu.tab_daw',
  'menu.tab_favorites',
  'menu.tab_files',
  'menu.tab_history',
  'menu.tab_notes',
  'menu.tab_plugins',
  'menu.tab_presets',
  'menu.tab_samples',
  'menu.tab_settings',
  'menu.toggle_crt',
  'menu.toggle_favorite',
  'menu.toggle_loop',
  'menu.toggle_mute',
  'menu.toggle_shuffle',
  'menu.toggle_theme',
  'menu.tools',
  'menu.view',
  'menu.window',
];

/** @see `src-tauri/src/app_i18n.rs` — `TRAY_KEYS` */
const TRAY_KEYS = [
  'tray.show',
  'tray.scan_all',
  'tray.stop_all',
  'tray.play_pause',
  'tray.next_track',
  'tray.quit',
  'tray.tooltip',
];

function loadMap(name) {
  const raw = readFileSync(join(i18nDir, name), 'utf8');
  return JSON.parse(raw);
}

function ipcTokenMultiset(s) {
  return (s.match(IPC_PLACEHOLDER) ?? []).slice().sort();
}

function isEsCriticalPrefix(k) {
  return (
    k.startsWith('menu.') ||
    k.startsWith('ui.palette.') ||
    k.startsWith('ui.sp_') ||
    k.startsWith('confirm.')
  );
}

test('de, el, es, es_419, fi, fr, it, nl, pl, pt, pt_br, ru, sv, zh, ja, ko, da, nb, tr, cs, hu, ro, uk, vi, id, hi: appFmt placeholder token multiset matches English for every key', () => {
  const en = loadMap('app_i18n_en.json');
  for (const loc of [
    'de',
    'el',
    'es',
    'es_419',
    'fi',
    'fr',
    'it',
    'nl',
    'pl',
    'pt',
    'pt_br',
    'ru',
    'sv',
    'zh',
    'ja',
    'ko',
    'da',
    'nb',
    'tr',
    'cs',
    'hu',
    'ro',
    'uk',
    'vi',
    'id',
    'hi',
  ]) {
    const m = loadMap(`app_i18n_${loc}.json`);
    const bad = [];
    for (const k of Object.keys(en)) {
      const a = JSON.stringify(ipcTokenMultiset(en[k]));
      const b = JSON.stringify(ipcTokenMultiset(m[k]));
      if (a !== b) bad.push({ k, en: en[k], locVal: m[k] });
    }
    assert.deepEqual(
      bad,
      [],
      `${loc}: ${bad.length} key(s) differ in {token} multiset vs English (first: ${bad[0]?.k ?? ''})`
    );
  }
});

test('es: critical prefixes preserve every English {token} substring (Rust seed_json rule)', () => {
  const en = loadMap('app_i18n_en.json');
  const es = loadMap('app_i18n_es.json');
  const bad = [];
  for (const [k, enVal] of Object.entries(en)) {
    if (!isEsCriticalPrefix(k)) continue;
    const placeholders = enVal.match(RUST_PLACEHOLDER) ?? [];
    if (placeholders.length === 0) continue;
    const v = es[k];
    for (const p of placeholders) {
      if (!v.includes(p)) bad.push({ k, missing: p, en: enVal, es: v });
    }
  }
  assert.deepEqual(
    bad,
    [],
    `es critical: ${bad.length} missing placeholder(s) (first key ${bad[0]?.k ?? ''})`
  );
});

test('es_419: critical prefixes preserve every English {token} substring (Rust seed_json rule)', () => {
  const en = loadMap('app_i18n_en.json');
  const es419 = loadMap('app_i18n_es_419.json');
  const bad = [];
  for (const [k, enVal] of Object.entries(en)) {
    if (!isEsCriticalPrefix(k)) continue;
    const placeholders = enVal.match(RUST_PLACEHOLDER) ?? [];
    if (placeholders.length === 0) continue;
    const v = es419[k];
    for (const p of placeholders) {
      if (!v.includes(p)) bad.push({ k, missing: p, en: enVal, es_419: v });
    }
  }
  assert.deepEqual(
    bad,
    [],
    `es_419 critical: ${bad.length} missing placeholder(s) (first key ${bad[0]?.k ?? ''})`
  );
});

test('es: menu.* and tray.* appFmt token multiset matches English (native menu + tray)', () => {
  const en = loadMap('app_i18n_en.json');
  const es = loadMap('app_i18n_es.json');
  const bad = [];
  for (const k of Object.keys(en)) {
    if (!k.startsWith('menu.') && !k.startsWith('tray.')) continue;
    const a = JSON.stringify(ipcTokenMultiset(en[k]));
    const b = JSON.stringify(ipcTokenMultiset(es[k]));
    if (a !== b) bad.push({ k, en: en[k], es: es[k] });
  }
  assert.deepEqual(
    bad,
    [],
    `es menu/tray: ${bad.length} key(s) differ in {token} multiset vs English (first: ${bad[0]?.k ?? ''})`
  );
});

test('es_419: menu.* and tray.* appFmt token multiset matches English (native menu + tray)', () => {
  const en = loadMap('app_i18n_en.json');
  const es419 = loadMap('app_i18n_es_419.json');
  const bad = [];
  for (const k of Object.keys(en)) {
    if (!k.startsWith('menu.') && !k.startsWith('tray.')) continue;
    const a = JSON.stringify(ipcTokenMultiset(en[k]));
    const b = JSON.stringify(ipcTokenMultiset(es419[k]));
    if (a !== b) bad.push({ k, en: en[k], es_419: es419[k] });
  }
  assert.deepEqual(
    bad,
    [],
    `es_419 menu/tray: ${bad.length} key(s) differ in {token} multiset vs English (first: ${bad[0]?.k ?? ''})`
  );
});

test('English catalog defines every native menu bar key (app_i18n.rs NATIVE_MENU_BAR_KEYS)', () => {
  const en = loadMap('app_i18n_en.json');
  const missing = NATIVE_MENU_BAR_KEYS.filter((k) => {
    const v = en[k];
    return v == null || String(v).trim() === '';
  });
  assert.deepEqual(missing, []);
});

test('English catalog defines every tray key (app_i18n.rs TRAY_KEYS)', () => {
  const en = loadMap('app_i18n_en.json');
  const missing = TRAY_KEYS.filter((k) => {
    const v = en[k];
    return v == null || String(v).trim() === '';
  });
  assert.deepEqual(missing, []);
});
