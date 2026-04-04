/**
 * Cross-locale consistency (same checks as `app_i18n::tests::seed_json_all_locales_share_exact_key_set`
 * and `seed_json_no_empty_values_any_locale`, but runnable via `pnpm run test:js`).
 * Plus structural rules on English keys so the catalog cannot drift into invalid namespaces.
 */
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import test from 'node:test';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const i18nDir = join(root, 'i18n');

const LOCALE_FILES = [
  ['en', 'app_i18n_en.json'],
  ['de', 'app_i18n_de.json'],
  ['es', 'app_i18n_es.json'],
  ['sv', 'app_i18n_sv.json'],
  ['fr', 'app_i18n_fr.json'],
  ['nl', 'app_i18n_nl.json'],
  ['pt', 'app_i18n_pt.json'],
];

/** Every key in shipped JSON must start with one of these prefixes (matches seed + `ipc.js` usage). */
const KEY_NS = /^(?:confirm|help|menu|toast|tray|ui)\.[a-zA-Z0-9_.]+$/;

function loadMap(name) {
  const raw = readFileSync(join(i18nDir, name), 'utf8');
  const map = JSON.parse(raw);
  assert.equal(typeof map, 'object', name);
  return map;
}

test('all shipped locales define the same key set as English', () => {
  const maps = LOCALE_FILES.map(([loc, file]) => [loc, loadMap(file)]);
  const keysEn = new Set(Object.keys(maps[0][1]));
  assert.ok(keysEn.size > 100, 'expected a large English catalog');
  for (const [loc, map] of maps.slice(1)) {
    const keys = new Set(Object.keys(map));
    const missing = [...keysEn].filter((k) => !keys.has(k));
    const extra = [...keys].filter((k) => !keysEn.has(k));
    assert.deepEqual(
      { missing, extra },
      { missing: [], extra: [] },
      `locale ${loc}: key set must match English (missing ${missing.length}, extra ${extra.length})`
    );
  }
});

test('every value in every locale JSON is a non-empty string', () => {
  for (const [loc, file] of LOCALE_FILES) {
    const map = loadMap(file);
    const bad = [];
    for (const [k, v] of Object.entries(map)) {
      if (typeof v !== 'string' || v.trim() === '') bad.push(k);
    }
    assert.deepEqual(bad, [], `locale ${loc}: empty or non-string values for keys: ${bad.slice(0, 12).join(', ')}${bad.length > 12 ? '…' : ''}`);
  }
});

test('every English catalog key matches allowed namespace pattern', () => {
  const en = loadMap('app_i18n_en.json');
  const bad = Object.keys(en).filter((k) => !KEY_NS.test(k));
  assert.deepEqual(bad, [], `invalid key shape: ${bad.slice(0, 24).join(', ')}${bad.length > 24 ? '…' : ''}`);
});
