#!/usr/bin/env python3
"""Merge a JSON object of i18n key → English string into i18n/app_i18n_en.json.

Keys are merged and the catalog is rewritten sorted by key (same style as the repo).

Usage:
  python3 scripts/merge_i18n_keys.py path/to/new_keys.json

Fails if any key already exists (use to avoid accidental overwrites).

For locale JSON (de/es/sv/fr/nl/pt/it), re-run scripts/gen_app_i18n_*.py after updating English.
"""
from __future__ import annotations

import json
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
EN_PATH = ROOT / "i18n" / "app_i18n_en.json"


def main() -> None:
    if len(sys.argv) < 2:
        print(__doc__.strip(), file=sys.stderr)
        raise SystemExit(2)
    batch_path = pathlib.Path(sys.argv[1]).resolve()
    incoming: dict[str, str] = json.loads(batch_path.read_text(encoding="utf-8"))
    en: dict[str, str] = json.loads(EN_PATH.read_text(encoding="utf-8"))
    for k, v in incoming.items():
        if k in en:
            raise SystemExit(f"Key already in catalog: {k}")
        en[k] = v
    EN_PATH.write_text(
        json.dumps(dict(sorted(en.items())), ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
    )
    print(f"Merged {len(incoming)} keys → {EN_PATH} (total {len(en)} keys)", file=sys.stderr)


if __name__ == "__main__":
    main()
