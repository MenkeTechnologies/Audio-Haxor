#!/usr/bin/env python3
"""Merge keys from a batch JSON into de/es/sv/fr/nl/pt/it/el/pl/ru/zh/ja/ko/fi/da/nb/tr using English values for new keys.

Values are taken from app_i18n_en.json. Re-writes each locale file sorted by key
(matching the English catalog style).

Usage:
  python3 scripts/merge_batch_into_locales.py scripts/i18n_batches/ui_perf.json
"""
from __future__ import annotations

import json
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
I18N = ROOT / "i18n"


def main() -> None:
    if len(sys.argv) < 2:
        print(__doc__.strip(), file=sys.stderr)
        raise SystemExit(2)
    batch_path = pathlib.Path(sys.argv[1]).resolve()
    batch_keys = set(json.loads(batch_path.read_text(encoding="utf-8")).keys())
    en: dict[str, str] = json.loads((I18N / "app_i18n_en.json").read_text(encoding="utf-8"))
    for loc in ("de", "es", "sv", "fr", "nl", "pt", "it", "el", "pl", "ru", "zh", "ja", "ko", "fi", "da", "nb", "tr"):
        path = I18N / f"app_i18n_{loc}.json"
        cur: dict[str, str] = json.loads(path.read_text(encoding="utf-8"))
        added = 0
        for k in batch_keys:
            if k not in en:
                raise SystemExit(f"Key {k} not in app_i18n_en.json")
            if k not in cur:
                cur[k] = en[k]
                added += 1
            elif cur[k] != en[k] and k in batch_keys:
                # Keep existing translation; only fill missing
                pass
        path.write_text(
            json.dumps(dict(sorted(cur.items())), ensure_ascii=False, indent=2) + "\n",
            encoding="utf-8",
        )
        print(f"{loc}: +{added} keys from batch → {path}", flush=True)


if __name__ == "__main__":
    main()
