#!/usr/bin/env python3
"""Ensure i18n/app_i18n_{de,es,sv,fr,nl,pt,it,el,pl,ru,zh,ja}.json contain every key from app_i18n_en.json.

Missing keys are filled with the English string (stub) so the app never shows raw
keys. Re-run scripts/gen_app_i18n_*.py with a venv when you want full machine
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
    for loc in ("de", "es", "sv", "fr", "nl", "pt", "it", "el", "pl", "ru", "zh", "ja"):
        path = I18N / f"app_i18n_{loc}.json"
        cur: dict[str, str] = json.loads(path.read_text(encoding="utf-8"))
        added = 0
        for k, v in en.items():
            if k not in cur:
                cur[k] = v
                added += 1
        path.write_text(
            json.dumps(dict(sorted(cur.items())), ensure_ascii=False, indent=2) + "\n",
            encoding="utf-8",
        )
        print(f"{loc}: added {added} keys → {path}", flush=True)


if __name__ == "__main__":
    main()
