# App UI strings (`i18n/app_i18n_*.json`)

## English catalog

- **Source of truth:** `i18n/app_i18n_en.json` (sorted keys).
- **Runtime:** Strings are seeded into SQLite (`app_i18n`) from the bundled JSON at build time (`src-tauri/src/app_i18n.rs`).
- **Adding keys:** Prefer a small JSON batch file under `scripts/i18n_batches/` and merge:

```bash
python3 scripts/merge_i18n_keys.py scripts/i18n_batches/your_batch.json
```

The script fails if a key already exists (prevents accidental overwrites). Rebuild the Tauri app after changing English so the DB seed updates.

- **Canonical toasts:** Add new `toast.*` strings to `scripts/gen_toast_i18n_en.py` (then merge the emitted lines into `app_i18n_en.json` via `merge_i18n_keys.py` or a small batch) so the toast catalog stays traceable.

- **Avoid blind `gen_app_i18n_en.py`:** That script rebuilds the English file from HTML extraction plus fixed `MENU_EN` / `HELP_EN` blocks. It only carries over **previous `ui.*`** keys from the old JSON, not arbitrary `menu.*` entries that exist only on disk. Running it without checking the diff can delete hundreds of extended menu/context keys. Prefer batch merges or edit `app_i18n_en.json` directly for targeted additions, then `python3 scripts/sync_locale_keys_from_en.py`.

## Other locales (`de`, `es`, `sv`, `fr`, `nl`, `pt`)

- **Full machine translation** (slow; needs network):

```bash
python3 -m venv .venv-i18n
.venv-i18n/bin/pip install deep-translator
.venv-i18n/bin/python scripts/gen_app_i18n_de.py
.venv-i18n/bin/python scripts/gen_app_i18n_es.py
.venv-i18n/bin/python scripts/gen_app_i18n_sv.py
.venv-i18n/bin/python scripts/gen_app_i18n_fr.py
.venv-i18n/bin/python scripts/gen_app_i18n_nl.py
.venv-i18n/bin/python scripts/gen_app_i18n_pt.py
```

- **German (`de`) — translate keys that still match English** (after stub sync or partial merges): `fill_de_i18n_gaps.py` calls Google Translate only for keys where the German value is still identical to English (skips `ui.ph.ui_ph_*` indirection strings and branding). Run `de_i18n_manual_overrides.py` afterward for hyphenation (`Plug-ins`), menu labels, localized `/pfad/zu/…` placeholders, and other strings machine translation leaves as English cognates. Then run `cargo test app_i18n:: --lib` and `pnpm run test:js`.

```bash
.venv-i18n/bin/python scripts/fill_de_i18n_gaps.py
.venv-i18n/bin/python scripts/de_i18n_manual_overrides.py
```

- **Fast stub sync:** Copy any missing keys from English so every locale has the same key set (values stay English until you translate):

```bash
python3 scripts/sync_locale_keys_from_en.py
```

Run the stub sync after adding keys to `app_i18n_en.json` if you cannot run the generators yet.

## Batch merge into non-English locales only

If English already contains new keys and you need the same keys in `de`/`es`/`sv`/`fr`/`nl`/`pt` with English placeholder text until a full `gen_app_i18n_*` run:

```bash
python3 scripts/merge_batch_into_locales.py scripts/i18n_batches/your_batch.json
```

## Deprecated

- `merge_ui_i18n_keys.py` — stub only; prints instructions and exits with code 1. Use `merge_i18n_keys.py` + `scripts/i18n_batches/*.json` instead.
