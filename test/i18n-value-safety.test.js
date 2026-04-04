/**
 * Catalog hygiene beyond placeholders and key parity: avoid invisible / control characters
 * that break SQLite, HTML `textContent`, or diff tooling; plus spot-checks that mirror
 * `app_i18n::tests::seed_json_fr_menu_scan_all_differs_from_en` (every non-English locale
 * should not copy English verbatim for `menu.scan_all`).
 */
import assert from 'node:assert/strict';
import { readFileSync, readdirSync } from 'node:fs';
import { dirname, join } from 'node:path';
import test from 'node:test';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const i18nDir = join(root, 'i18n');

/** Disallow C0 controls, DEL, and Unicode line/paragraph separators (unsafe in UI / seeds). */
const UNSAFE_CHAR = /[\u0000-\u001F\u007F\u2028\u2029]/;

function shippedAppI18nJsonFiles() {
  return readdirSync(i18nDir)
    .filter((n) => n.startsWith('app_i18n_') && n.endsWith('.json'))
    .sort();
}

function loadMap(name) {
  const raw = readFileSync(join(i18nDir, name), 'utf8');
  return JSON.parse(raw);
}

test('every catalog value is free of C0 controls, DEL, and U+2028/U+2029', () => {
  for (const file of shippedAppI18nJsonFiles()) {
    const map = loadMap(file);
    const bad = [];
    for (const [k, v] of Object.entries(map)) {
      if (typeof v !== 'string') {
        bad.push({ k, reason: 'non-string' });
        continue;
      }
      if (UNSAFE_CHAR.test(v)) bad.push({ k, reason: 'unsafe char' });
    }
    assert.deepEqual(
      bad,
      [],
      `${file}: ${bad.length} value(s) contain control chars or line/paragraph separators`
    );
  }
});

test('non-English locales do not copy English verbatim for menu.scan_all', () => {
  const en = loadMap('app_i18n_en.json');
  const baseline = en['menu.scan_all'];
  assert.ok(baseline != null && String(baseline).trim() !== '', 'menu.scan_all missing in English');
  for (const loc of ['de', 'es', 'fr', 'nl', 'pt', 'sv']) {
    const m = loadMap(`app_i18n_${loc}.json`);
    assert.notEqual(
      m['menu.scan_all'],
      baseline,
      `locale ${loc}: menu.scan_all must differ from English (spot-check translation)`
    );
  }
});
