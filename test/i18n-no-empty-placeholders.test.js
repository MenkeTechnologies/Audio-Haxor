/**
 * Defense-in-depth: Ensure that NO internationalization catalog (all app_i18n_*.json files)
 * contains empty placeholders like "{}" which are likely typos for "{name}".
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

const EMPTY_PLACEHOLDER = /\{\}/;

for (const fname of getLocaleFiles()) {
  const loc = fname.match(/app_i18n_(.*)\.json/)[1];
  const map = JSON.parse(readFileSync(join(i18nDir, fname), 'utf8'));

  for (const [key, value] of Object.entries(map)) {
    if (typeof value !== 'string') continue;

    test(`locale ${loc} key ${key} has no empty placeholders`, () => {
      assert.ok(
        !EMPTY_PLACEHOLDER.test(value),
        `${fname} key ${key}: empty placeholder "{}" found in value: ${JSON.stringify(value)}`
      );
    });
  }
}
