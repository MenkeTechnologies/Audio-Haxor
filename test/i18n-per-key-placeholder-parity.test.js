/**
 * Granular companion to `i18n-seed-parity.test.js`: one `node:test` case per catalog key
 * (and locale) so failures name the exact `app_i18n_*.json` row — the bulk multiset tests
 * only report the first mismatch.
 *
 * Rules mirror `src-tauri/src/app_i18n.rs` + `i18n-seed-parity.test.js`:
 * - `da` / `de` / `el` / `es` / `fi` / `fr` / `it` / `nb` / `nl` / `pl` / `pt` / `ru` / `sv` / `tr` / `zh` / `ja` / `ko`: IPC `{token}` multiset matches English for every key that uses placeholders
 * - `es`: every English `{token}` substring must appear in the translation for `menu.*`,
 *   `ui.palette.*`, `ui.sp_*`, `confirm.*` when English has placeholders (`seed_json_es_critical_prefixes`)
 */
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import test from 'node:test';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const i18nDir = join(root, 'i18n');

/** Same as `ipc.js` `appFmt` — `\{\w+\}` */
const IPC_PLACEHOLDER = /\{\w+\}/g;

/** Rust `seed_json_es_critical_prefixes_match_en_placeholders` */
const RUST_PLACEHOLDER = /\{[a-zA-Z_][a-zA-Z0-9_]*\}/g;

function loadMap(name) {
  const raw = readFileSync(join(i18nDir, name), 'utf8');
  return JSON.parse(raw);
}

function ipcTokenMultiset(s) {
  return (s.match(IPC_PLACEHOLDER) ?? []).slice().sort();
}

function isEsCriticalPrefix(k) {
  return (
    k.startsWith('menu.') ||
    k.startsWith('ui.palette.') ||
    k.startsWith('ui.sp_') ||
    k.startsWith('confirm.')
  );
}

const en = loadMap('app_i18n_en.json');
const de = loadMap('app_i18n_de.json');
const fr = loadMap('app_i18n_fr.json');
const nl = loadMap('app_i18n_nl.json');
const pt = loadMap('app_i18n_pt.json');
const sv = loadMap('app_i18n_sv.json');
const it = loadMap('app_i18n_it.json');
const el = loadMap('app_i18n_el.json');
const pl = loadMap('app_i18n_pl.json');
const ru = loadMap('app_i18n_ru.json');
const zh = loadMap('app_i18n_zh.json');
const ja = loadMap('app_i18n_ja.json');
const ko = loadMap('app_i18n_ko.json');
const fi = loadMap('app_i18n_fi.json');
const es = loadMap('app_i18n_es.json');
const da = loadMap('app_i18n_da.json');
const nb = loadMap('app_i18n_nb.json');
const tr = loadMap('app_i18n_tr.json');

const keysWithIpcTokens = Object.keys(en).filter(
  (k) => (en[k].match(IPC_PLACEHOLDER) ?? []).length > 0
);

const localeMaps = { da, de, el, es, fi, fr, it, nb, nl, pl, pt, ru, sv, tr, zh, ja, ko };

for (const loc of /** @type {const} */ ([
  'da',
  'de',
  'el',
  'es',
  'fi',
  'fr',
  'it',
  'nb',
  'nl',
  'pl',
  'pt',
  'ru',
  'sv',
  'tr',
  'zh',
  'ja',
  'ko',
])) {
  const m = localeMaps[loc];
  for (const k of keysWithIpcTokens) {
    test(`seed multiset parity ${loc} ${k}`, () => {
      const a = JSON.stringify(ipcTokenMultiset(en[k]));
      const b = JSON.stringify(ipcTokenMultiset(m[k]));
      assert.equal(
        a,
        b,
        `${loc} key ${k}: IPC {token} multiset must match English (en=${JSON.stringify(en[k])} ${loc}=${JSON.stringify(m[k])})`
      );
    });
  }
}

for (const k of Object.keys(en)) {
  if (!isEsCriticalPrefix(k)) continue;
  const placeholders = en[k].match(RUST_PLACEHOLDER) ?? [];
  if (placeholders.length === 0) continue;
  test(`es critical English {token} substrings preserved ${k}`, () => {
    const v = es[k];
    for (const p of placeholders) {
      assert.ok(
        v.includes(p),
        `es key ${k}: must contain ${p} (en=${JSON.stringify(en[k])} es=${JSON.stringify(v)})`
      );
    }
  });
}
