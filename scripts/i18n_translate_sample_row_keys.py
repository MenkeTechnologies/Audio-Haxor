#!/usr/bin/env python3
"""One-shot translator for the Expand/Collapse Sample Row context-menu keys.

Mirrors `i18n_translate_toast_history_keys.py` but force-overwrites existing
locale values for the listed keys (the manual stubs inserted at key-injection
time should be replaced by the machine-translated value derived from current
English).

Run from the repo root with the i18n venv:
    .venv-i18n/bin/python scripts/i18n_translate_sample_row_keys.py
"""
from __future__ import annotations

import json
import pathlib
import sys
import time

ROOT = pathlib.Path(__file__).resolve().parents[1]
I18N_DIR = ROOT / "i18n"

NEW_KEYS: tuple[str, ...] = (
    "menu.collapse_sample_row",
    "menu.expand_sample_row",
)

LOCALES: tuple[str, ...] = (
    "de", "es", "es_419", "sv", "fr", "nl", "pt", "pt_br", "it", "el",
    "pl", "ru", "zh", "ja", "ko", "fi", "da", "nb", "tr", "cs", "hu",
    "id", "ro", "uk", "vi", "hi",
)

# Google Translate codes that don't match our locale code 1:1.
GT_CODE = {"zh": "zh-CN", "es_419": "es", "pt_br": "pt", "nb": "no"}


def main() -> None:
    try:
        from deep_translator import GoogleTranslator
    except ImportError as exc:
        print(
            "Install deep-translator: .venv-i18n/bin/pip install deep-translator",
            file=sys.stderr,
        )
        raise SystemExit(1) from exc

    en_path = I18N_DIR / "app_i18n_en.json"
    en: dict[str, str] = json.loads(en_path.read_text(encoding="utf-8"))

    missing = [k for k in NEW_KEYS if k not in en]
    if missing:
        raise SystemExit(f"EN catalog missing keys: {missing}")
    new_values = {k: en[k] for k in NEW_KEYS}

    print(
        f"Translating {len(new_values)} sample-row keys across {len(LOCALES)} locales",
        file=sys.stderr,
    )

    for locale in LOCALES:
        p = I18N_DIR / f"app_i18n_{locale}.json"
        cat: dict[str, str] = json.loads(p.read_text(encoding="utf-8"))
        tgt = GT_CODE.get(locale, locale)
        try:
            tr = GoogleTranslator(source="en", target=tgt)
        except Exception as e:
            print(f"  {locale}: translator init failed: {e}", file=sys.stderr)
            continue

        cache: dict[str, str] = {}
        n_new = 0
        for k, en_val in new_values.items():
            # Force-overwrite: these keys were inserted with hand-rolled stubs
            # that should be replaced by GoogleTranslate output for consistency
            # with how the rest of each locale catalog was produced.
            if en_val in cache:
                cat[k] = cache[en_val]
                n_new += 1
                continue
            try:
                t = tr.translate(en_val)
                if not t:
                    t = en_val
            except Exception:
                t = en_val
            cache[en_val] = t
            cat[k] = t
            n_new += 1
            time.sleep(0.06)

        ordered = {k: cat[k] for k in sorted(cat)}
        p.write_text(
            json.dumps(ordered, ensure_ascii=False, indent=2, sort_keys=False) + "\n",
            encoding="utf-8",
        )
        print(f"  {locale}: translated {n_new} key(s)", file=sys.stderr)


if __name__ == "__main__":
    main()
