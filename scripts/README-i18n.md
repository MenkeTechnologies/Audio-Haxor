# App UI strings (`i18n/app_i18n_*.json`)

## English catalog

- **Source of truth:** `i18n/app_i18n_en.json` (sorted keys).
- **Runtime:** Strings are seeded into SQLite (`app_i18n`) from the bundled JSON at build time (`src-tauri/src/app_i18n.rs`).
- **Adding keys:** Prefer a small JSON batch file under `scripts/i18n_batches/` and merge:

```bash
python3 scripts/merge_i18n_keys.py scripts/i18n_batches/your_batch.json
```

The script fails if a key already exists (prevents accidental overwrites). Rebuild the Tauri app after changing English so the DB seed updates.

## Other locales (`de`, `es`, `sv`, `fr`)

- **Full machine translation** (slow; needs network):

```bash
python3 -m venv .venv-i18n
.venv-i18n/bin/pip install deep-translator
.venv-i18n/bin/python scripts/gen_app_i18n_de.py
.venv-i18n/bin/python scripts/gen_app_i18n_es.py
.venv-i18n/bin/python scripts/gen_app_i18n_sv.py
.venv-i18n/bin/python scripts/gen_app_i18n_fr.py
```

- **Fast stub sync:** Copy any missing keys from English so every locale has the same key set (values stay English until you translate):

```bash
python3 scripts/sync_locale_keys_from_en.py
```

Run the stub sync after adding keys to `app_i18n_en.json` if you cannot run the generators yet.

## Batch merge into non-English locales only

If English already contains new keys and you need the same keys in `de`/`es`/`sv`/`fr` with English placeholder text until a full `gen_app_i18n_*` run:

```bash
python3 scripts/merge_batch_into_locales.py scripts/i18n_batches/your_batch.json
```

## Deprecated

- `merge_ui_i18n_keys.py` — stub only; prints instructions and exits with code 1. Use `merge_i18n_keys.py` + `scripts/i18n_batches/*.json` instead.
