/**
 * Filesystem invariants for `i18n/app_i18n_*.json` — must stay aligned with
 * `src-tauri/src/app_i18n.rs` `include_str!` seeds and CI expectations.
 * Also: strict UTF-8 (no invalid byte sequences) and lexicographically sorted
 * top-level keys (stable diffs / merge scripts).
 */
import assert from 'node:assert/strict';
import { readFileSync, readdirSync } from 'node:fs';
import { dirname, join } from 'node:path';
import test from 'node:test';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const i18nDir = join(root, 'i18n');

/** Same locales as `app_i18n.rs` `SEED_JSON_*` and `i18n-locales-and-shape.test.js`. */
const SHIPPED_APP_I18N = [
  'app_i18n_cs.json',
  'app_i18n_da.json',
  'app_i18n_de.json',
  'app_i18n_el.json',
  'app_i18n_en.json',
  'app_i18n_es.json',
  'app_i18n_fi.json',
  'app_i18n_fr.json',
  'app_i18n_hu.json',
  'app_i18n_it.json',
  'app_i18n_ja.json',
  'app_i18n_ko.json',
  'app_i18n_nb.json',
  'app_i18n_nl.json',
  'app_i18n_pl.json',
  'app_i18n_pt.json',
  'app_i18n_ro.json',
  'app_i18n_ru.json',
  'app_i18n_sv.json',
  'app_i18n_tr.json',
  'app_i18n_uk.json',
  'app_i18n_vi.json',
  'app_i18n_zh.json',
];

test('i18n/ has exactly the shipped app_i18n_*.json locale files (no extras, none missing)', () => {
  const jsonFiles = readdirSync(i18nDir)
    .filter((n) => n.endsWith('.json'))
    .sort();
  const appI18n = jsonFiles.filter((n) => n.startsWith('app_i18n_'));
  assert.deepEqual(
    appI18n,
    [...SHIPPED_APP_I18N].sort(),
    'add/remove locale JSON only with app_i18n.rs + all i18n test allowlists'
  );
});

test('shipped locale JSON files do not start with a UTF-8 / Unicode BOM', () => {
  for (const name of SHIPPED_APP_I18N) {
    const text = readFileSync(join(i18nDir, name), 'utf8');
    assert.ok(!text.startsWith('\uFEFF'), `${name} must not start with BOM (breaks parsers / seeds)`);
  }
});

test('shipped locale JSON files are well-formed UTF-8 (strict decode)', () => {
  const decoder = new TextDecoder('utf-8', { fatal: true });
  for (const name of SHIPPED_APP_I18N) {
    const buf = readFileSync(join(i18nDir, name));
    decoder.decode(buf);
  }
});

test('shipped locale maps have lexicographically sorted keys', () => {
  for (const name of SHIPPED_APP_I18N) {
    const map = JSON.parse(readFileSync(join(i18nDir, name), 'utf8'));
    const keys = Object.keys(map);
    for (let i = 1; i < keys.length; i++) {
      assert.ok(
        keys[i] >= keys[i - 1],
        `${name}: keys must be sorted — ${JSON.stringify(keys[i - 1])} then ${JSON.stringify(keys[i])}`
      );
    }
  }
});
