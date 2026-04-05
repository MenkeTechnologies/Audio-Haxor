# App UI strings (`i18n/app_i18n_*.json`)

## English catalog

- **Source of truth:** `i18n/app_i18n_en.json` (sorted keys).
- **Action vs. noun compounds:** English strings like **“Scan Plugins”** (and the matching keys `menu.scan_plugins`, `ui.btn.8635_scan_plugins`, `ui.js.scan_plugins_btn`) describe a **button action** — *scan for plugins* — not a category of plugin (“scanning plugins”, “analysis plugins”, etc.). Automated translation often inverts word order or picks the wrong sense; keep `toast.scanning_plugins` consistent (progress wording).
- **Runtime:** Strings are seeded into SQLite (`app_i18n`) from the bundled JSON at build time (`src-tauri/src/app_i18n.rs`).
- **Adding keys:** Prefer a small JSON batch file under `scripts/i18n_batches/` and merge:

```bash
python3 scripts/merge_i18n_keys.py scripts/i18n_batches/your_batch.json
```

The script fails if a key already exists (prevents accidental overwrites). Rebuild the Tauri app after changing English so the DB seed updates.

**Settings → Interface language** only saves `uiLocale` to prefs. The **next app launch** runs `reloadAppStrings` (and `refresh_native_menu`) so the UI and native menu bar match the saved locale. Changing the dropdown does **not** reload strings in the current session. Editing `i18n/app_i18n_*.json` still requires a rebuild (and restart) for the bundled SQLite seed to change.

## Other locales (`da`, `de`, `es`, `sv`, `fr`, `nl`, `pt`, `it`, `el`, `pl`, `ru`, `zh` — Simplified Chinese, `ja` — Japanese, `ko` — Korean, `fi` — Finnish, `nb` — Norwegian Bokmål, `tr` — Turkish)

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
.venv-i18n/bin/python scripts/gen_app_i18n_sv.py
.venv-i18n/bin/python scripts/gen_app_i18n_fr.py
.venv-i18n/bin/python scripts/gen_app_i18n_nl.py
.venv-i18n/bin/python scripts/gen_app_i18n_pt.py
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
```

- **German (`de`) — translate keys that still match English** (after stub sync or partial merges): `fill_de_i18n_gaps.py` calls Google Translate only for keys where the German value is still identical to English (skips `ui.ph.ui_ph_*` indirection strings and branding). Run `de_i18n_manual_overrides.py` afterward for hyphenation (`Plug-ins`), menu labels, localized `/pfad/zu/…` placeholders, and other strings machine translation leaves as English cognates.

```bash
.venv-i18n/bin/python scripts/fill_de_i18n_gaps.py
.venv-i18n/bin/python scripts/de_i18n_manual_overrides.py
```

- **Fast stub sync:** Copy any missing keys from English so every locale has the same key set (values stay English until you translate):

```bash
python3 scripts/sync_locale_keys_from_en.py
```

Run the stub sync after adding keys to `app_i18n_en.json` if you cannot run the generators yet.

### `appFmt` placeholders (`{token}`)

Dynamic strings substitute **English token names** from `ipc.js` / Rust callers (e.g. `{name}`, `{err}`, `{n}`). Non-English catalogs must use the **same** `{token}` spellings as `app_i18n_en.json` for each key — translated prose around them is fine, but renaming a token to a localized word (e.g. `{nombre}` for `{name}`) breaks substitution. `node --test test/i18n-seed-parity.test.js` and `test/i18n-per-key-placeholder-parity.test.js` enforce multiset parity vs English for every shipped locale (including `es`).

### Audit all locales (placeholder parity vs English)

From repo root — should report **0 failures**; any mismatch names the locale and key:

```bash
node --test test/i18n-seed-parity.test.js test/i18n-per-key-placeholder-parity.test.js
```

Broader catalog checks (same key set in every JSON, HTML/JS key coverage, etc.): `node scripts/run-js-tests.mjs` (see **Automated checks** below).

## Batch merge into non-English locales only

If English already contains new keys and you need the same keys in `de`/`es`/`sv`/`fr`/`nl`/`pt`/`it`/`el`/`pl`/`ru`/`zh`/`ja`/`ko`/`fi`/`da`/`nb`/`tr` with English placeholder text until a full `gen_app_i18n_*` run:

```bash
python3 scripts/merge_batch_into_locales.py scripts/i18n_batches/your_batch.json
```

## Automated checks (CI)

- `test/i18n-html-keys.test.js` — every `data-i18n*` key in `frontend/index.html` exists in `app_i18n_en.json`.
- `test/i18n-js-keys.test.js` — string literals that look like catalog keys (`ui.*`, `menu.*`, `toast.*`, …) under `frontend/js` are defined in English.
- `test/i18n-locales-and-shape.test.js` — every shipped locale JSON has the **same key set** as English and only non-empty string values.
- `test/i18n-prove-all-locales-complete.test.js` — exhaustive proof: every English key exists in every locale with a non-empty value; every HTML- and JS-referenced catalog key exists in **every** locale (not only `en`).
- `test/i18n-anchor-keys.test.js` — for keys where **da/de/el/es/fi/fr/it/ja/ko/nb/nl/pl/pt/ru/sv/tr/zh** all differ from English, none of those locales may copy `en` verbatim.
- `test/i18n-no-raw-showtoast.test.js` — `showToast` is not called with a raw `'…'` / `"…"` first argument (use `toastFmt('toast.*')` or `String(err)`).
- `test/i18n-catalog-files.test.js` — shipped locale JSON files match `app_i18n.rs` seeds, UTF-8, and **lexicographically sorted keys** (stable merges).

Run `node scripts/run-js-tests.mjs` (or `pnpm run test:js` if wired) after catalog edits.

- **PDF tab:** `frontend/js/pdf.js` builds table rows and scan/load-more UI with `appFmt` / shared keys (`ui.js.load_more_hint`, `ui.audio.scan_progress_line`, etc.); `index.html` uses `data-i18n` on `ui.pdf.*` for the stats bar and PDF walker tile header.
- **Sample / MIDI / preset / DAW tables:** Row tooltips and action buttons use `appFmt` / `_audioFmt` / `_midiFmt` / `_presetFmt` / `_dawFmt` (e.g. `ui.audio.row_btn_*`, `menu.reveal_in_finder`, `ui.js.load_more_hint`, `ui.tt.daw_open_in_project`). Cell text is still file metadata (paths, formats, BPM numbers).

## Deprecated

- `merge_ui_i18n_keys.py` — stub only; prints instructions and exits with code 1. Use `merge_i18n_keys.py` + `scripts/i18n_batches/*.json` instead.
