/**
 * `frontend/js/ipc.js` substitutes `\{(\w+)\}` (ASCII word chars only). Any `{…}` in a catalog
 * value must be a single token like `{name}` or `{n}` — not `{a-b}` or nested `{{x}}`.
 */
import assert from 'node:assert/strict';
import { readFileSync, readdirSync } from 'node:fs';
import { dirname, join } from 'node:path';
import test from 'node:test';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const i18nDir = join(root, 'i18n');

/** Same token pattern as `ipc.js` `appFmt`: `\{(\w+)\}` */
const IPC_TOKEN = /^\{\w+\}$/;
const BRACE_SEGMENT = /\{[^}]+\}/g;

function assertPlaceholdersOk(fileLabel, key, value) {
  const segs = value.match(BRACE_SEGMENT);
  if (!segs) return;
  for (const s of segs) {
    assert.ok(
      IPC_TOKEN.test(s),
      `${fileLabel} key ${key}: placeholder segment ${JSON.stringify(s)} is not ipc.js-compatible (use {token} with letters/digits/underscore only)`
    );
  }
}

test('locale JSON: brace placeholders match ipc.js appFmt token pattern', () => {
  const files = readdirSync(i18nDir).filter((n) => n.startsWith('app_i18n_') && n.endsWith('.json'));
  assert.ok(files.length > 0, 'expected i18n/app_i18n_*.json files');
  for (const name of files.sort()) {
    const raw = readFileSync(join(i18nDir, name), 'utf8');
    const map = JSON.parse(raw);
    assert.equal(typeof map, 'object', name);
    for (const [k, v] of Object.entries(map)) {
      assert.equal(typeof v, 'string', `${name} ${k}`);
      assertPlaceholdersOk(name, k, v);
    }
  }
});
