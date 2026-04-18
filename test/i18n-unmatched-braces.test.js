/**
 * Defense-in-depth: Ensure that NO internationalization catalog (all app_i18n_*.json files)
 * contains unmatched braces (e.g. "Hello {name" or "Hello name}").
 *
 * This catches typos in placeholder syntax.
 */
import assert from 'node:assert/strict';
import { readFileSync, readdirSync } from 'node:fs';
import { dirname, join } from 'node:path';
import test from 'node:test';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const i18nDir = join(root, 'i18n');

function getLocaleFiles() {
  return readdirSync(i18nDir)
    .filter((f) => f.startsWith('app_i18n_') && f.endsWith('.json'));
}

for (const fname of getLocaleFiles()) {
  const loc = fname.match(/app_i18n_(.*)\.json/)[1];
  const map = JSON.parse(readFileSync(join(i18nDir, fname), 'utf8'));

  for (const [key, value] of Object.entries(map)) {
    if (typeof value !== 'string') continue;

    test(`locale ${loc} key ${key} has matched braces`, () => {
      let open = 0;
      for (let i = 0; i < value.length; i++) {
        if (value[i] === '{') {
          open++;
          assert.ok(open === 1, `${fname} key ${key}: nested braces found: ${JSON.stringify(value)}`);
        } else if (value[i] === '}') {
          open--;
          assert.ok(open === 0, `${fname} key ${key}: unmatched closing brace found: ${JSON.stringify(value)}`);
        }
      }
      assert.ok(open === 0, `${fname} key ${key}: unmatched opening brace found: ${JSON.stringify(value)}`);
    });
  }
}
