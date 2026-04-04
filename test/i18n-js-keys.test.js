/**
 * Every string literal under `frontend/js` (recursive `.js` files) that looks like an `app_i18n` key
 * (`ui.…`, `menu.…`, `toast.…`, …) must exist and be non-empty in `i18n/app_i18n_en.json`.
 * Complements `i18n-html-keys.test.js` (static HTML `data-i18n*`) — JS uses `appFmt`, aliases
 * (`_ui`, `h`, `f`, …), and object fields like `labelKey` / `descKey`.
 */
import assert from 'node:assert/strict';
import { readFileSync, readdirSync } from 'node:fs';
import { dirname, join } from 'node:path';
import test from 'node:test';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');

/** Match literals for the six top-level namespaces used in `i18n/app_i18n_en.json`. */
const CATALOG_KEY_RE =
  /['"]((?:confirm|help|menu|toast|tray|ui)\.[a-zA-Z0-9_.]+)['"]/g;

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

test('frontend JS string literals that look like app_i18n keys exist in English catalog', () => {
  const jsRoot = join(root, 'frontend/js');
  const en = JSON.parse(readFileSync(join(root, 'i18n/app_i18n_en.json'), 'utf8'));
  const keys = new Set();
  for (const file of collectJsFiles(jsRoot)) {
    const text = readFileSync(file, 'utf8');
    for (const k of catalogKeysFromSource(text)) keys.add(k);
  }
  assert.ok(keys.size > 100, 'expected many catalog key literals under frontend/js');
  const missing = [];
  for (const k of keys) {
    const v = en[k];
    if (v == null || String(v).trim() === '') missing.push(k);
  }
  missing.sort();
  assert.deepEqual(
    missing,
    [],
    `Missing or empty in app_i18n_en.json (${missing.length}): ${missing.slice(0, 32).join(', ')}${missing.length > 32 ? '…' : ''}`
  );
});
