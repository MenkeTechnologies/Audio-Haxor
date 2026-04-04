#!/usr/bin/env python3
"""Build i18n/app_i18n_es.json from app_i18n_en.json (Spanish UI).

Requires: pip install deep-translator (use a venv, e.g. .venv-i18n).

Translates each unique English value once, then maps keys — re-run when app_i18n_en.json grows.
"""
from __future__ import annotations

import json
import pathlib
import sys
import time

ROOT = pathlib.Path(__file__).resolve().parents[1]
I18N_DIR = ROOT / "i18n"


def main() -> None:
    try:
        from deep_translator import GoogleTranslator
    except ImportError:
        print(
            "Install deep-translator in a venv: python3 -m venv .venv-i18n && "
            ".venv-i18n/bin/pip install deep-translator && .venv-i18n/bin/python scripts/gen_app_i18n_es.py",
            file=sys.stderr,
        )
        raise SystemExit(1) from None

    en_path = I18N_DIR / "app_i18n_en.json"
    out_path = I18N_DIR / "app_i18n_es.json"
    en: dict[str, str] = json.loads(en_path.read_text(encoding="utf-8"))
    translator = GoogleTranslator(source="en", target="es")

    uniq_vals = list(dict.fromkeys(en.values()))
    val_to_es: dict[str, str] = {}
    for i, v in enumerate(uniq_vals):
        try:
            val_to_es[v] = translator.translate(v)
        except Exception:
            val_to_es[v] = v
        if (i + 1) % 80 == 0:
            print(f"{i + 1}/{len(uniq_vals)}", flush=True)
        time.sleep(0.06)

    es = {k: val_to_es[v] for k, v in en.items()}
    # Keep English label for the locale selector
    if es.get("ui.opt.lang_en") in ("Inglés", "Ingles"):
        es["ui.opt.lang_en"] = "English"
    # Native language names in selector
    if "ui.opt.lang_de" in es:
        es["ui.opt.lang_de"] = "Deutsch"
    if "ui.opt.lang_es" in es:
        es["ui.opt.lang_es"] = "Español"
    for k in list(es.keys()):
        if "{Name}" in es[k]:
            es[k] = es[k].replace("{Name}", "{name}")

    out_path.write_text(json.dumps(es, ensure_ascii=False, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(f"Wrote {len(es)} keys to {out_path}", file=sys.stderr)


if __name__ == "__main__":
    main()
