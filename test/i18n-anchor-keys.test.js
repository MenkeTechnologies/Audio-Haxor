/**
 * Per-locale, per-key spot checks: high-traffic menu and tray strings must not be a verbatim
 * copy of English (catches accidental `en` paste or failed translation in a seed JSON row).
 * Mirrors `seed_json_*_menu_scan_all_differs_from_en` in `src-tauri/src/app_i18n.rs` but covers
 * more anchors than `menu.scan_all` alone.
 */
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import test from 'node:test';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const i18nDir = join(root, 'i18n');

/** Keys that must be translated in every non-English shipped locale (values differ from `en`). */
const ANCHOR_KEYS = [
  'menu.scan_all',
  'menu.preferences',
  'tray.scan_all',
  'tray.quit',
];

const NON_EN = /** @type {const} */ (['de', 'es', 'fr', 'nl', 'pt', 'sv']);

function loadMap(name) {
  const raw = readFileSync(join(i18nDir, name), 'utf8');
  return JSON.parse(raw);
}

const en = loadMap('app_i18n_en.json');

for (const anchor of ANCHOR_KEYS) {
  assert.ok(
    en[anchor] != null && String(en[anchor]).trim() !== '',
    `English catalog must define non-empty ${anchor}`
  );
  for (const loc of NON_EN) {
    test(`locale ${loc} differs from en for ${anchor}`, () => {
      const m = loadMap(`app_i18n_${loc}.json`);
      assert.notEqual(
        m[anchor],
        en[anchor],
        `${loc} ${anchor}: value must not be a verbatim copy of English`
      );
    });
  }
}
