#!/usr/bin/env python3
"""Incremental translator: only translate the file-browser keys defined
in `i18n_inject_fb_keys.py` for every non-English locale. Existing
translations in those locales are PRESERVED (we only touch the keys we
added). Sorts keys before write so the i18n-sort CI check stays green.

Run from repo root (venv with deep-translator):
    .venv-i18n/bin/python scripts/i18n_translate_fb_keys.py
"""
from __future__ import annotations

import importlib.util
import json
import pathlib
import sys
import time

ROOT = pathlib.Path(__file__).resolve().parents[1]
I18N_DIR = ROOT / "i18n"

LOCALES: tuple[str, ...] = (
    "de", "es", "es_419", "sv", "fr", "nl", "pt", "pt_br", "it", "el",
    "pl", "ru", "zh", "ja", "ko", "fi", "da", "nb", "tr", "cs", "hu",
    "id", "ro", "uk", "vi", "hi",
)


def load_new_keys() -> dict[str, str]:
    spec = importlib.util.spec_from_file_location(
        "inj", ROOT / "scripts" / "i18n_inject_fb_keys.py"
    )
    if spec is None or spec.loader is None:
        raise SystemExit("cannot load i18n_inject_fb_keys.py")
    m = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(m)
    return dict(m.NEW_KEYS)


def main() -> None:
    try:
        from deep_translator import GoogleTranslator
    except ImportError as exc:
        print(
            "Install deep-translator: .venv-i18n/bin/pip install deep-translator",
            file=sys.stderr,
        )
        raise SystemExit(1) from exc

    new_keys = load_new_keys()
    print(f"Translating {len(new_keys)} keys across {len(LOCALES)} locales", file=sys.stderr)

    for locale in LOCALES:
        p = I18N_DIR / f"app_i18n_{locale}.json"
        cat = json.loads(p.read_text(encoding="utf-8"))
        # Pick the target language code Google Translate understands.
        # Most locales pass through; a couple need mapping.
        gt_code = {"zh": "zh-CN", "es_419": "es", "pt_br": "pt", "nb": "no"}.get(locale, locale)
        try:
            tr = GoogleTranslator(source="en", target=gt_code)
        except Exception as e:
            print(f"  {locale}: translator init failed: {e}", file=sys.stderr)
            continue
        # Cache translations by EN value (many keys share the same EN
        # value, e.g. "Close" / "Apply") so we don't re-hit the API.
        cache: dict[str, str] = {}
        n_new = 0
        n_kept = 0
        for k, en in new_keys.items():
            # Skip if this locale ALREADY has a non-English value for
            # this key — leaves manually-curated translations alone.
            if k in cat and cat[k] and cat[k] != en:
                n_kept += 1
                continue
            if en in cache:
                cat[k] = cache[en]
                n_new += 1
                continue
            try:
                t = tr.translate(en)
                if not t:
                    t = en
            except Exception:
                t = en
            cache[en] = t
            cat[k] = t
            n_new += 1
            time.sleep(0.06)
        # Re-sort + write (sorted keys is the CI invariant).
        ordered = {k: cat[k] for k in sorted(cat)}
        p.write_text(
            json.dumps(ordered, ensure_ascii=False, indent=2, sort_keys=False) + "\n",
            encoding="utf-8",
        )
        print(f"  {locale}: +{n_new} translated, {n_kept} kept", file=sys.stderr)


if __name__ == "__main__":
    main()
