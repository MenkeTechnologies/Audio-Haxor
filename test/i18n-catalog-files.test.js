/**
 * Filesystem invariants for `i18n/app_i18n_*.json` — must stay aligned with
 * `src-tauri/src/app_i18n.rs` `include_str!` seeds and CI expectations.
 */
import assert from 'node:assert/strict';
import { readFileSync, readdirSync } from 'node:fs';
import { dirname, join } from 'node:path';
import test from 'node:test';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const i18nDir = join(root, 'i18n');

/** Same five locales as `app_i18n.rs` `SEED_JSON_*` and `i18n-locales-and-shape.test.js`. */
const SHIPPED_APP_I18N = [
  'app_i18n_de.json',
  'app_i18n_en.json',
  'app_i18n_es.json',
  'app_i18n_fr.json',
  'app_i18n_sv.json',
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
