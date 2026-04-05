/**
 * Proof-oriented i18n tests: the shipped catalog is complete in **every** locale, and every
 * key referenced from static HTML or `frontend/js` resolves in **every** locale (not only English).
 *
 * - Structural parity is also enforced in `i18n-locales-and-shape.test.js` and
 *   `app_i18n::tests::seed_json_all_locales_share_exact_key_set`.
 * - English-only reference checks: `i18n-html-keys.test.js`, `i18n-js-keys.test.js`.
 */
import assert from 'node:assert/strict';
import { readFileSync, readdirSync } from 'node:fs';
import { dirname, join } from 'node:path';
import test from 'node:test';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const i18nDir = join(root, 'i18n');

/** Same order as `i18n-locales-and-shape.test.js` / `app_i18n.rs` seeds. */
const LOCALE_FILES = [
  ['en', 'app_i18n_en.json'],
  ['de', 'app_i18n_de.json'],
  ['el', 'app_i18n_el.json'],
  ['es', 'app_i18n_es.json'],
  ['sv', 'app_i18n_sv.json'],
  ['da', 'app_i18n_da.json'],
  ['nb', 'app_i18n_nb.json'],
  ['fr', 'app_i18n_fr.json'],
  ['nl', 'app_i18n_nl.json'],
  ['pl', 'app_i18n_pl.json'],
  ['pt', 'app_i18n_pt.json'],
  ['ru', 'app_i18n_ru.json'],
  ['it', 'app_i18n_it.json'],
  ['zh', 'app_i18n_zh.json'],
  ['ja', 'app_i18n_ja.json'],
  ['ko', 'app_i18n_ko.json'],
  ['fi', 'app_i18n_fi.json'],
  ['tr', 'app_i18n_tr.json'],
];

/** Match literals for namespaces in `i18n/app_i18n_en.json` (same as `i18n-js-keys.test.js`). */
const CATALOG_KEY_RE =
  /['"]((?:confirm|help|menu|toast|tray|ui)\.[a-zA-Z0-9_.]+)['"]/g;

function loadMap(file) {
  const map = JSON.parse(readFileSync(join(i18nDir, file), 'utf8'));
  assert.equal(typeof map, 'object', file);
  return map;
}

function collectDataI18nKeys(html) {
  const keys = new Set();
  const re = /data-i18n(?:-placeholder|-title)?="([^"]+)"/g;
  let m;
  while ((m = re.exec(html)) !== null) {
    keys.add(m[1]);
  }
  return keys;
}

function collectJsFiles(dir, out = []) {
  for (const ent of readdirSync(dir, { withFileTypes: true })) {
    const p = join(dir, ent.name);
    if (ent.isDirectory()) collectJsFiles(p, out);
    else if (ent.isFile() && ent.name.endsWith('.js')) out.push(p);
  }
  return out;
}

function catalogKeysFromSource(text) {
  const keys = new Set();
  let m;
  while ((m = CATALOG_KEY_RE.exec(text)) !== null) {
    keys.add(m[1]);
  }
  return keys;
}

const en = loadMap('app_i18n_en.json');
const keysEn = Object.keys(en);
const localeMaps = Object.fromEntries(
  LOCALE_FILES.map(([loc, file]) => [loc, loadMap(file)])
);

test('every key in app_i18n_en.json exists in every other locale with a non-empty string', () => {
  assert.ok(keysEn.length > 100, 'expected a large English catalog');
  for (const [loc, file] of LOCALE_FILES) {
    if (loc === 'en') continue;
    const m = localeMaps[loc];
    const problems = [];
    for (const k of keysEn) {
      const v = m[k];
      if (v == null || typeof v !== 'string' || v.trim() === '') {
        problems.push(`${file}: missing or empty for key ${k}`);
      }
    }
    for (const k of Object.keys(m)) {
      if (!(k in en)) problems.push(`${file}: extra key not in English catalog: ${k}`);
    }
    assert.deepEqual(
      problems,
      [],
      problems.length ? problems.slice(0, 24).join('\n') + (problems.length > 24 ? '\n…' : '') : ''
    );
  }
});

test('every key in app_i18n_en.json is non-empty in English', () => {
  const problems = [];
  for (const k of keysEn) {
    const v = en[k];
    if (v == null || typeof v !== 'string' || v.trim() === '') problems.push(k);
  }
  assert.deepEqual(problems, [], `empty English values: ${problems.slice(0, 24).join(', ')}`);
});

test('every index.html data-i18n key exists in every locale with a non-empty string', () => {
  const html = readFileSync(join(root, 'frontend/index.html'), 'utf8');
  const htmlKeys = [...collectDataI18nKeys(html)].sort();
  assert.ok(htmlKeys.length > 0, 'expected data-i18n* keys in index.html');
  const problems = [];
  for (const k of htmlKeys) {
    for (const [loc, file] of LOCALE_FILES) {
      const v = localeMaps[loc][k];
      if (v == null || String(v).trim() === '') {
        problems.push(`${file} (${loc}): missing or empty for HTML key ${k}`);
      }
    }
  }
  assert.deepEqual(
    problems,
    [],
    problems.length ? problems.slice(0, 32).join('\n') + (problems.length > 32 ? '\n…' : '') : ''
  );
});

test('every catalog key literal in frontend/js exists in every locale with a non-empty string', () => {
  const jsRoot = join(root, 'frontend/js');
  const keys = new Set();
  for (const file of collectJsFiles(jsRoot)) {
    const text = readFileSync(file, 'utf8');
    for (const k of catalogKeysFromSource(text)) keys.add(k);
  }
  assert.ok(keys.size > 100, 'expected many catalog key literals under frontend/js');
  const sorted = [...keys].sort();
  const problems = [];
  for (const k of sorted) {
    for (const [loc, file] of LOCALE_FILES) {
      const v = localeMaps[loc][k];
      if (v == null || String(v).trim() === '') {
        problems.push(`${file} (${loc}): missing or empty for JS key ${k}`);
      }
    }
  }
  assert.deepEqual(
    problems,
    [],
    problems.length ? problems.slice(0, 32).join('\n') + (problems.length > 32 ? '\n…' : '') : ''
  );
});
