#!/usr/bin/env python3
"""Build i18n/app_i18n_fr.json from app_i18n_en.json (French UI).

Requires: pip install deep-translator (use a venv, e.g. .venv-i18n).

Translates each unique English value once, then maps keys — re-run when app_i18n_en.json grows.
"""
from __future__ import annotations

import json
import pathlib
import re
import sys
import time

ROOT = pathlib.Path(__file__).resolve().parents[1]
I18N_DIR = ROOT / "i18n"


def align_placeholders(en_val: str, fr_val: str) -> str:
    """MT often translates `{name}` inside braces; keep English token names for appFmt."""
    ph_en = re.findall(r"\{(\w+)\}", en_val)
    ph_fr = re.findall(r"\{(\w+)\}", fr_val)
    if len(ph_en) != len(ph_fr):
        return fr_val
    it = iter(ph_en)
    return re.sub(r"\{[^}]+\}", lambda _: "{" + next(it) + "}", fr_val)


def main() -> None:
    try:
        from deep_translator import GoogleTranslator
    except ImportError:
        print(
            "Install deep-translator in a venv: python3 -m venv .venv-i18n && "
            ".venv-i18n/bin/pip install deep-translator && .venv-i18n/bin/python scripts/gen_app_i18n_fr.py",
            file=sys.stderr,
        )
        raise SystemExit(1) from None

    en_path = I18N_DIR / "app_i18n_en.json"
    out_path = I18N_DIR / "app_i18n_fr.json"
    en: dict[str, str] = json.loads(en_path.read_text(encoding="utf-8"))
    translator = GoogleTranslator(source="en", target="fr")

    uniq_vals = list(dict.fromkeys(en.values()))
    val_to_fr: dict[str, str] = {}
    for i, v in enumerate(uniq_vals):
        try:
            val_to_fr[v] = translator.translate(v)
        except Exception:
            val_to_fr[v] = v
        if (i + 1) % 80 == 0:
            print(f"{i + 1}/{len(uniq_vals)}", flush=True)
        time.sleep(0.06)

    fr = {k: val_to_fr[v] for k, v in en.items()}
    for k in fr:
        fr[k] = align_placeholders(en[k], fr[k])
    # Keep English label for the locale selector
    if fr.get("ui.opt.lang_en") in ("Anglais", "anglais"):
        fr["ui.opt.lang_en"] = "English"
    # Native language names in selector
    if "ui.opt.lang_de" in fr:
        fr["ui.opt.lang_de"] = "Deutsch"
    if "ui.opt.lang_es" in fr:
        fr["ui.opt.lang_es"] = "Español"
    if "ui.opt.lang_sv" in fr:
        fr["ui.opt.lang_sv"] = "Svenska"
    if "ui.opt.lang_fr" in fr:
        fr["ui.opt.lang_fr"] = "Français"
    for k in list(fr.keys()):
        if "{Name}" in fr[k]:
            fr[k] = fr[k].replace("{Name}", "{name}")
        if "{Wert}" in fr[k]:
            fr[k] = fr[k].replace("{Wert}", "{value}")

    out_path.write_text(json.dumps(fr, ensure_ascii=False, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(f"Wrote {len(fr)} keys to {out_path}", file=sys.stderr)


if __name__ == "__main__":
    main()
