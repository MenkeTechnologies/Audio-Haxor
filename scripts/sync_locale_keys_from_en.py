#!/usr/bin/env python3
"""Ensure i18n/app_i18n_{de,es,es_419,sv,fr,nl,pt,pt_br,it,el,pl,ru,zh,ja,ko,fi,da,nb,tr,cs,hu,ro,uk,vi,id,hi}.json match app_i18n_en.json key-for-key.

For each locale: output has exactly the English key set. Existing translations
are kept where the key still exists; new keys get the English string as a stub;
keys removed from English are dropped from the locale file.

Re-run scripts/gen_app_i18n_*.py with a venv when you want full machine
translation of the catalog.

Usage:
  python3 scripts/sync_locale_keys_from_en.py
"""
from __future__ import annotations

import json
import pathlib

ROOT = pathlib.Path(__file__).resolve().parents[1]
I18N = ROOT / "i18n"


def main() -> None:
    en_path = I18N / "app_i18n_en.json"
    en: dict[str, str] = json.loads(en_path.read_text(encoding="utf-8"))
    for loc in (
        "de",
        "es",
        "es_419",
        "sv",
        "fr",
        "nl",
        "pt",
        "pt_br",
        "it",
        "el",
        "pl",
        "ru",
        "zh",
        "ja",
        "ko",
        "fi",
        "da",
        "nb",
        "tr",
        "cs",
        "hu",
        "ro",
        "uk",
        "vi",
        "id",
        "hi",
    ):
        path = I18N / f"app_i18n_{loc}.json"
        cur: dict[str, str] = json.loads(path.read_text(encoding="utf-8"))
        merged = {k: cur.get(k, en[k]) for k in en}
        added = sum(1 for k in en if k not in cur)
        removed = sum(1 for k in cur if k not in en)
        path.write_text(
            json.dumps(dict(sorted(merged.items())), ensure_ascii=False, indent=2) + "\n",
            encoding="utf-8",
        )
        print(
            f"{loc}: added {added}, removed {removed} (kept {len(merged)} keys) → {path}",
            flush=True,
        )


if __name__ == "__main__":
    main()
