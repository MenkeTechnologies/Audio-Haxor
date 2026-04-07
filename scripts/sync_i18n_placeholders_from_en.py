#!/usr/bin/env python3
"""
For each non-English `i18n/app_i18n_*.json`, copy the English string for any key whose value
does not contain every `{placeholder}` token present in `app_i18n_en.json` for that key.

`appFmt` / `toastFmt` require English token names (`{name}`, `{path}`, …). Localized spellings
inside braces break runtime substitution and `app_i18n` seed tests.

Run after bulk locale edits or `sync_locale_keys_from_en.py` when tests report placeholder drift:

  python3 scripts/sync_i18n_placeholders_from_en.py
"""
from __future__ import annotations

import json
import re
from pathlib import Path

_PLACEHOLDER_RE = re.compile(r"\{[a-zA-Z_][a-zA-Z0-9_]*\}")


def placeholders(s: str) -> set[str]:
    return {m.group(0) for m in _PLACEHOLDER_RE.finditer(s)}


def main() -> None:
    root = Path(__file__).resolve().parents[1]
    en_path = root / "i18n" / "app_i18n_en.json"
    en: dict[str, str] = json.loads(en_path.read_text(encoding="utf-8"))
    for p in sorted((root / "i18n").glob("app_i18n_*.json")):
        if p.name == "app_i18n_en.json":
            continue
        data: dict[str, str] = json.loads(p.read_text(encoding="utf-8"))
        n = 0
        for k, ev in en.items():
            pe = placeholders(ev)
            if not pe or k not in data:
                continue
            v = data[k]
            if all(t in v for t in pe):
                continue
            data[k] = ev
            n += 1
        if n:
            p.write_text(json.dumps(data, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
            print(f"{p.name}: aligned {n} keys from English (placeholder fix)")


if __name__ == "__main__":
    main()
