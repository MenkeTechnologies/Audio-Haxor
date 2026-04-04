/**
 * Ensures every `data-i18n*=` key in `frontend/index.html` exists in `i18n/app_i18n_en.json`.
 * Catches drift when HTML references a key the catalog never defined.
 */
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import test from 'node:test';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');

function collectDataI18nKeys(html) {
  const keys = new Set();
  const re = /data-i18n(?:-placeholder|-title)?="([^"]+)"/g;
  let m;
  while ((m = re.exec(html)) !== null) {
    keys.add(m[1]);
  }
  return keys;
}

test('index.html data-i18n keys exist in English catalog', () => {
  const html = readFileSync(join(root, 'frontend/index.html'), 'utf8');
  const en = JSON.parse(readFileSync(join(root, 'i18n/app_i18n_en.json'), 'utf8'));
  const keys = collectDataI18nKeys(html);
  assert.ok(keys.size > 0, 'expected at least one data-i18n* attribute in index.html');
  const missing = [];
  for (const k of keys) {
    const v = en[k];
    if (v == null || String(v).trim() === '') missing.push(k);
  }
  assert.deepEqual(
    missing,
    [],
    `Missing or empty in app_i18n_en.json (${missing.length}): ${missing.slice(0, 24).join(', ')}${missing.length > 24 ? '…' : ''}`
  );
});
