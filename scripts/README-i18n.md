# App UI strings (`i18n/app_i18n_*.json`)

## English catalog

- **Source of truth:** `i18n/app_i18n_en.json` (sorted keys). **CI** (`test/i18n-catalog-files.test.js`) asserts every shipped `app_i18n_*.json` has **lexicographically sorted** top-level keys — `merge_i18n_keys.py` / `sync_locale_keys_from_en.py` emit sorted maps; if you hand-edit JSON, re-sort before push (`pnpm run i18n:sort` → `scripts/sort_app_i18n_catalogs.py`) or CI fails with “keys must be sorted”. (`sync_i18n_placeholders_from_en.py` also rewrites sorted maps.)
- **`gen_app_i18n_en.py`:** Rebuilds toast/menu/help/confirm/ui sections from `scripts/` + `frontend/index.html`. Any key already in `app_i18n_en.json` that the generator does not emit (e.g. batch-merged `menu.*` for context menus) is **kept**. Extracted HTML text is normalized to a single line (no raw newlines in JSON values) so `test/i18n-value-safety.test.js` passes.
- **Action vs. noun compounds:** English strings like **“Scan Plugins”** (and the matching keys `menu.scan_plugins`, `ui.btn.8635_scan_plugins`, `ui.js.scan_plugins_btn`) describe a **button action** — *scan for plugins* — not a category of plugin (“scanning plugins”, “analysis plugins”, etc.). Automated translation often inverts word order or picks the wrong sense; keep `toast.scanning_plugins` consistent (progress wording).
- **Runtime:** Strings are seeded into SQLite (`app_i18n`) from the bundled JSON at build time (`src-tauri/src/app_i18n.rs`).
- **Adding keys:** Prefer a small JSON batch file under `scripts/i18n_batches/` and merge:

```bash
python3 scripts/merge_i18n_keys.py scripts/i18n_batches/your_batch.json
```

  Example: History tab strings live in `scripts/i18n_batches/history_tab_i18n.json` (`ui.history.*`, `confirm.clear_all_history_*`); merge into English, then run `sync_locale_keys_from_en.py`.

The script fails if a key already exists (prevents accidental overwrites). Rebuild the Tauri app after changing English so the DB seed updates.

**Updating English copy for an existing key:** `sync_locale_keys_from_en.py` does not overwrite values in other locales. To copy the new English string into every non-English catalog as a stub until translators re-run `gen_app_i18n_*.py`):

```bash
python3 scripts/sync_i18n_values_from_en.py ui.example.key
```

**Settings → Interface language** only saves `uiLocale` to prefs. The **next app launch** runs `reloadAppStrings` (and `refresh_native_menu`) so the UI and native menu bar match the saved locale. Changing the dropdown does **not** reload strings in the current session. Editing `i18n/app_i18n_*.json` still requires a rebuild (and restart) for the bundled SQLite seed to change.

**`ui.ph.*` (HTML `data-i18n-placeholder`):** Multi-line placeholder text uses the literal characters `&#10;` in JSON (same convention as `placeholder="…&#10;…"` in static HTML). `frontend/js/i18n-ui.js` decodes `&#10;` and `&#13;` to real newline characters when assigning `element.placeholder`, because JavaScript does not apply HTML entity decoding to that property.

**Tauri save/export dialogs:** Use `catalogFmt('menu.*')` or `catalogFmt('ui.dialog.*')` for `title`, and `catalogFmt('ui.file_filter.*')` for `filters[].name`, so OS file sheets follow the current UI locale.

## Other locales (`cs` — Czech, `da`, `de`, `es`, `es-419` — Latin American Spanish (`app_i18n_es_419.json`), `sv`, `fr`, `nl`, `pt` — Portuguese, `pt-BR` — Brazilian Portuguese (`app_i18n_pt_br.json`), `it`, `el`, `pl`, `ru`, `zh` — Simplified Chinese, `ja` — Japanese, `ko` — Korean, `fi` — Finnish, `nb` — Norwegian Bokmål, `tr` — Turkish, `hu` — Hungarian, `id` — Indonesian, `hi` — Hindi, `ro` — Romanian, `uk` — Ukrainian, `vi` — Vietnamese)

- **Full machine translation** (slow; needs network). Regenerate **all** shipped non-English catalogs from English in one go:

```bash
python3 -m venv .venv-i18n
.venv-i18n/bin/pip install deep-translator
.venv-i18n/bin/python scripts/gen_all_app_i18n_locales.py
```

Or run per-locale generators individually:

```bash
.venv-i18n/bin/python scripts/gen_app_i18n_de.py
.venv-i18n/bin/python scripts/gen_app_i18n_es.py
.venv-i18n/bin/python scripts/gen_app_i18n_es_419.py
.venv-i18n/bin/python scripts/gen_app_i18n_sv.py
.venv-i18n/bin/python scripts/gen_app_i18n_fr.py
.venv-i18n/bin/python scripts/gen_app_i18n_nl.py
.venv-i18n/bin/python scripts/gen_app_i18n_pt.py
.venv-i18n/bin/python scripts/gen_app_i18n_pt_br.py
.venv-i18n/bin/python scripts/gen_app_i18n_it.py
.venv-i18n/bin/python scripts/gen_app_i18n_el.py
.venv-i18n/bin/python scripts/gen_app_i18n_pl.py
.venv-i18n/bin/python scripts/gen_app_i18n_ru.py
.venv-i18n/bin/python scripts/gen_app_i18n_zh.py
.venv-i18n/bin/python scripts/gen_app_i18n_ja.py
.venv-i18n/bin/python scripts/gen_app_i18n_ko.py
.venv-i18n/bin/python scripts/gen_app_i18n_fi.py
.venv-i18n/bin/python scripts/gen_app_i18n_da.py
.venv-i18n/bin/python scripts/gen_app_i18n_nb.py
.venv-i18n/bin/python scripts/gen_app_i18n_tr.py
.venv-i18n/bin/python scripts/gen_app_i18n_cs.py
.venv-i18n/bin/python scripts/gen_app_i18n_hu.py
.venv-i18n/bin/python scripts/gen_app_i18n_id.py
.venv-i18n/bin/python scripts/gen_app_i18n_hi.py
.venv-i18n/bin/python scripts/gen_app_i18n_ro.py
.venv-i18n/bin/python scripts/gen_app_i18n_uk.py
.venv-i18n/bin/python scripts/gen_app_i18n_vi.py
```

- **German (`de`) — translate keys that still match English** (after stub sync or partial merges): `fill_de_i18n_gaps.py` calls Google Translate only for keys where the German value is still identical to English (skips `ui.ph.ui_ph_*` indirection strings and branding). Run `de_i18n_manual_overrides.py` afterward for hyphenation (`Plug-ins`), menu labels, localized `/pfad/zu/…` placeholders, and other strings machine translation leaves as English cognates.

```bash
.venv-i18n/bin/python scripts/fill_de_i18n_gaps.py
.venv-i18n/bin/python scripts/de_i18n_manual_overrides.py
```

- **All non-English locales — fill English stubs in one pass:** `fill_all_locale_i18n_gaps.py` translates each **distinct** gap string once per locale (Google Translate), same target codes as `gen_app_i18n_*.py`. Skips `ui.opt.lang_*` endonyms, branding, and `ui.ph.ui_ph_*`. Slower than stub sync but fixes visible English leftovers after `sync_locale_keys_from_en.py`. Optional `--locale de` processes one file. After refreshing `de`, still run `de_i18n_manual_overrides.py`.

```bash
.venv-i18n/bin/python scripts/fill_all_locale_i18n_gaps.py
.venv-i18n/bin/python scripts/de_i18n_manual_overrides.py
```

- **Fast stub sync:** Align every non-English file with English: missing keys get the English string as a stub; keys removed from English are dropped; existing translations are kept where the key still exists.

```bash
python3 scripts/sync_locale_keys_from_en.py
```

Run the stub sync after changing `app_i18n_en.json` (or after `gen_app_i18n_en.py`) if you cannot run the per-locale generators yet.

### `appFmt` placeholders (`{token}`)

Dynamic strings substitute **English token names** from `ipc.js` / Rust callers (e.g. `{name}`, `{err}`, `{n}`). Non-English catalogs must use the **same** `{token}` spellings as `app_i18n_en.json` for each key — translated prose around them is fine, but renaming a token to a localized word (e.g. `{nombre}` for `{name}`) breaks substitution. `node --test test/i18n-seed-parity.test.js` and `test/i18n-per-key-placeholder-parity.test.js` enforce multiset parity vs English for every shipped locale (including `es` and `es_419`).

### Audit all locales (placeholder parity vs English)

From repo root — should report **0 failures**; any mismatch names the locale and key:

```bash
node --test test/i18n-seed-parity.test.js test/i18n-per-key-placeholder-parity.test.js
```

Broader catalog checks (same key set in every JSON, HTML/JS key coverage, etc.): `node scripts/run-js-tests.mjs` (see **Automated checks** below).

## Batch merge into non-English locales only

If English already contains new keys and you need the same keys in `de`/`es`/`es_419`/`sv`/`fr`/`nl`/`pt`/`it`/`el`/`pl`/`ru`/`zh`/`ja`/`ko`/`fi`/`da`/`nb`/`tr`/`cs`/`hu`/`id`/`ro`/`uk`/`vi` with English placeholder text until a full `gen_app_i18n_*` run:

```bash
python3 scripts/merge_batch_into_locales.py scripts/i18n_batches/your_batch.json
```

## Automated checks (CI)

- `test/i18n-html-keys.test.js` — every `data-i18n*` key in `frontend/index.html` exists in `app_i18n_en.json`.
- `test/i18n-js-keys.test.js` — string literals that look like catalog keys (`ui.*`, `menu.*`, `toast.*`, …) under `frontend/js` are defined in English.
- `test/i18n-locales-and-shape.test.js` — every shipped locale JSON has the **same key set** as English and only non-empty string values.
- `test/i18n-prove-all-locales-complete.test.js` — exhaustive proof: every English key exists in every locale with a non-empty value; every HTML- and JS-referenced catalog key exists in **every** locale (not only `en`).
- `test/i18n-anchor-keys.test.js` — for keys where **cs/da/de/el/es/es_419/fi/fr/hu/id/it/ja/ko/nb/nl/pl/pt/ro/ru/sv/tr/uk/vi/zh** all differ from English, none of those locales may copy `en` verbatim.
- `test/i18n-no-raw-showtoast.test.js` — `showToast` is not called with a raw `'…'` / `"…"` first argument (use `toastFmt('toast.*')` or `String(err)`).
- `test/i18n-proof-contract.test.js` — no `? appFmt('…') : 'English'` / `toastFmt` patterns in `frontend/js`; use `catalogFmt` / `catalogFmtOrUnit` (`utils.js`) so strings resolve through the catalog (or the key when `appFmt` is missing in VM tests). Byte/time unit suffixes (`B`, `s`, …) use `catalogFmtOrUnit` only.
- `test/i18n-catalog-files.test.js` — shipped locale JSON files match `app_i18n.rs` seeds, UTF-8, and **lexicographically sorted keys** (stable merges).

Run `node scripts/run-js-tests.mjs` (or `pnpm run test:js` if wired) after catalog edits.

- **PDF tab:** `frontend/js/pdf.js` builds table rows and scan/load-more UI with `appFmt` / shared keys (`ui.js.load_more_hint`, `ui.audio.scan_progress_line`, etc.); `index.html` uses `data-i18n` on `ui.pdf.*` for the stats bar and PDF walker tile header.
- **Sample / MIDI / preset / DAW tables:** Row tooltips and action buttons use `appFmt` / `_audioFmt` / `_midiFmt` / `_presetFmt` / `_dawFmt` (e.g. `ui.audio.row_btn_*`, `menu.reveal_in_finder`, `ui.js.load_more_hint`, `ui.tt.daw_open_in_project`). Cell text is still file metadata (paths, formats, BPM numbers).

## Deprecated

- `merge_ui_i18n_keys.py` — stub only; prints instructions and exits with code 1. Use `merge_i18n_keys.py` + `scripts/i18n_batches/*.json` instead.
