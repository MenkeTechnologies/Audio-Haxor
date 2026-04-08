#!/usr/bin/env python3
"""Rewrite every i18n/app_i18n_*.json with lexicographically sorted top-level keys.

CI (`test/i18n-catalog-files.test.js`) requires sorted keys. Use after hand-editing
JSON or when a script wrote `json.dumps(data)` without sorting.

Usage:
  python3 scripts/sort_app_i18n_catalogs.py
"""
from __future__ import annotations

import json
import pathlib

ROOT = pathlib.Path(__file__).resolve().parents[1]
I18N = ROOT / "i18n"


def main() -> None:
    n_changed = 0
    for path in sorted(I18N.glob("app_i18n_*.json")):
        raw = path.read_text(encoding="utf-8")
        data: dict[str, str] = json.loads(raw)
        text = json.dumps(dict(sorted(data.items())), ensure_ascii=False, indent=2) + "\n"
        if raw != text:
            path.write_text(text, encoding="utf-8")
            n_changed += 1
            print(f"sorted keys → {path.name}", flush=True)
    if n_changed == 0:
        print("all app_i18n_*.json catalogs already sorted", flush=True)


if __name__ == "__main__":
    main()
