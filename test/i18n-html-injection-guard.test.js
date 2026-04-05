/**
 * Defense-in-depth: no shipped `app_i18n_*.json` value may contain angle-bracket tag openers for
 * `<script` or `<iframe` (case-insensitive). The app uses `textContent` / `appFmt` in normal
 * paths; this catches accidental catalog edits that would be dangerous if a value were ever
 * interpolated into `innerHTML` without escaping.
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
  ['el', 'app_i18n_el.json'],
  ['es', 'app_i18n_es.json'],
  ['sv', 'app_i18n_sv.json'],
  ['fr', 'app_i18n_fr.json'],
  ['nl', 'app_i18n_nl.json'],
  ['pl', 'app_i18n_pl.json'],
  ['pt', 'app_i18n_pt.json'],
  ['ru', 'app_i18n_ru.json'],
  ['it', 'app_i18n_it.json'],
];

function loadMap(name) {
  const raw = readFileSync(join(i18nDir, name), 'utf8');
  return JSON.parse(raw);
}

const SCRIPT = /<script/i;
const IFRAME = /<iframe/i;

for (const [loc, fname] of LOCALE_FILES) {
  const map = loadMap(fname);
  for (const [k, v] of Object.entries(map)) {
    test(`${fname} key ${k} has no script or iframe tag openers`, () => {
      assert.equal(typeof v, 'string', `${fname} ${k}: value must be a string`);
      assert.ok(!SCRIPT.test(v), `${fname} ${k}: must not contain <script`);
      assert.ok(!IFRAME.test(v), `${fname} ${k}: must not contain <iframe`);
    });
  }
}
