/**
 * Every key in `scripts/i18n_batches/*.json` must match `i18n/app_i18n_en.json` exactly.
 * Those batch files record merges from `merge_i18n_keys.py`; this proves the English catalog
 * was not partially reverted or edited out of sync with the batch source of truth.
 */
import assert from 'node:assert/strict';
import { readFileSync, readdirSync } from 'node:fs';
import { dirname, join } from 'node:path';
import test from 'node:test';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const batchDir = join(root, 'scripts/i18n_batches');
const enPath = join(root, 'i18n/app_i18n_en.json');

const batchFiles = readdirSync(batchDir)
  .filter((n) => n.endsWith('.json'))
  .sort();

assert.ok(batchFiles.length > 0, 'expected scripts/i18n_batches/*.json');

const en = JSON.parse(readFileSync(enPath, 'utf8'));

for (const name of batchFiles) {
  test(`i18n batch ${name} matches app_i18n_en.json`, () => {
    const batch = JSON.parse(readFileSync(join(batchDir, name), 'utf8'));
    assert.equal(typeof batch, 'object');
    assert.ok(Object.keys(batch).length > 0, `${name} is non-empty`);
    const mismatches = [];
    for (const [k, v] of Object.entries(batch)) {
      if (en[k] !== v) mismatches.push({ key: k, expected: v, actual: en[k] });
    }
    assert.deepEqual(
      mismatches,
      [],
      `${name}: ${mismatches.length} key(s) differ from i18n/app_i18n_en.json (re-run merge or fix catalog)`
    );
  });
}
