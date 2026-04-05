/**
 * Per-locale spot checks: for every **safe** shipped-catalog key under the UI namespaces below,
 * each locale in **TRANSLATED_LOCALES** must not copy the English string verbatim (catches pasted `en` rows or bad MT).
 *
 * **Safe key** = English value is non-empty and every locale in **TRANSLATED_LOCALES** differs from English
 * for that key (shared brand strings like `menu.app` / `tray.tooltip` are excluded automatically when any
 * locale still matches `en`).
 *
 * **`ru` is excluded:** `app_i18n_ru.json` is currently stub-synced from English (same key set and values as `en`
 * until a native Russian pass). It cannot satisfy “differs from English” yet, so it is not part of the anchor set.
 */
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import test from 'node:test';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const i18nDir = join(root, 'i18n');

/** Same namespaces as `ipc.js` / HTML `data-i18n*` usage (plus native menus + tray). */
const CATALOG_PREFIXES = /** @type {const} */ ([
  'menu.',
  'tray.',
  'confirm.',
  'toast.',
  'help.',
  'ui.',
]);

const NON_EN = /** @type {const} */ (['de', 'el', 'es', 'fr', 'it', 'nl', 'pl', 'pt', 'ru', 'sv']);

/** Locales required to differ from English for anchor keys (`ru` omitted while Russian ships as English-copy). */
const TRANSLATED_LOCALES = /** @type {const} */ (NON_EN.filter((loc) => loc !== 'ru'));

function matchesCatalogPrefix(k) {
  return CATALOG_PREFIXES.some((p) => k.startsWith(p));
}

function loadMap(name) {
  const raw = readFileSync(join(i18nDir, name), 'utf8');
  return JSON.parse(raw);
}

const en = loadMap('app_i18n_en.json');
const locMaps = Object.fromEntries(
  NON_EN.map((loc) => [loc, loadMap(`app_i18n_${loc}.json`)])
);

/** @returns {string[]} */
function anchorKeysWhereEveryLocaleDiffers() {
  const keys = [];
  for (const k of Object.keys(en).sort()) {
    if (!matchesCatalogPrefix(k)) continue;
    const ev = en[k];
    if (typeof ev !== 'string' || ev.trim() === '') continue;
    if (TRANSLATED_LOCALES.every((loc) => locMaps[loc][k] !== ev)) keys.push(k);
  }
  return keys;
}

const ANCHOR_KEYS = anchorKeysWhereEveryLocaleDiffers();

test('catalog yields a large safe anchor set across UI namespaces', () => {
  assert.ok(
    ANCHOR_KEYS.length > 1200,
    `expected 1200+ safe keys (excluding ru from diff requirement), got ${ANCHOR_KEYS.length}`
  );
});

test('ru locale mirrors English for UI keys (stub until native Russian translation)', () => {
  const ru = loadMap('app_i18n_ru.json');
  let uiKeys = 0;
  let verbatim = 0;
  for (const k of Object.keys(en)) {
    if (!matchesCatalogPrefix(k)) continue;
    const ev = en[k];
    if (typeof ev !== 'string' || ev.trim() === '') continue;
    uiKeys++;
    if (ru[k] === ev) verbatim++;
  }
  assert.ok(uiKeys > 1000, 'expected large UI key surface');
  assert.ok(
    verbatim / uiKeys > 0.95,
    `expected ru to mostly mirror en while stubbed (got ${verbatim}/${uiKeys} verbatim)`
  );
});

for (const anchor of ANCHOR_KEYS) {
  assert.ok(
    en[anchor] != null && String(en[anchor]).trim() !== '',
    `English catalog must define non-empty ${anchor}`
  );
  for (const loc of TRANSLATED_LOCALES) {
    test(`locale ${loc} differs from en for ${anchor}`, () => {
      const m = locMaps[loc];
      assert.notEqual(
        m[anchor],
        en[anchor],
        `${loc} ${anchor}: value must not be a verbatim copy of English`
      );
    });
  }
}
