#!/usr/bin/env python3
"""Build i18n/app_i18n_nl.json from app_i18n_en.json (Dutch UI).

Requires: pip install deep-translator (use a venv, e.g. .venv-i18n).

Translates each unique English value once, then maps keys — re-run when app_i18n_en.json grows.

After MT, verify `ui.perf.line_db_caches` and `ui.perf.line_uptime` keep English `{token}` names
(Google Translate may rewrite multi-word placeholders; fix manually if tests fail).
"""
from __future__ import annotations

import json
import pathlib
import re
import sys
import time

ROOT = pathlib.Path(__file__).resolve().parents[1]
I18N_DIR = ROOT / "i18n"


def align_placeholders(en_val: str, nl_val: str) -> str:
    """MT often translates `{name}` inside braces; keep English token names for appFmt."""
    ph_en = re.findall(r"\{(\w+)\}", en_val)
    ph_nl = re.findall(r"\{(\w+)\}", nl_val)
    if len(ph_en) != len(ph_nl):
        return nl_val
    it = iter(ph_en)
    return re.sub(r"\{[^}]+\}", lambda _: "{" + next(it) + "}", nl_val)


def main() -> None:
    try:
        from deep_translator import GoogleTranslator
    except ImportError:
        print(
            "Install deep-translator in a venv: python3 -m venv .venv-i18n && "
            ".venv-i18n/bin/pip install deep-translator && .venv-i18n/bin/python scripts/gen_app_i18n_nl.py",
            file=sys.stderr,
        )
        raise SystemExit(1) from None

    en_path = I18N_DIR / "app_i18n_en.json"
    out_path = I18N_DIR / "app_i18n_nl.json"
    en: dict[str, str] = json.loads(en_path.read_text(encoding="utf-8"))
    translator = GoogleTranslator(source="en", target="nl")

    uniq_vals = list(dict.fromkeys(en.values()))
    val_to_nl: dict[str, str] = {}
    for i, v in enumerate(uniq_vals):
        try:
            val_to_nl[v] = translator.translate(v)
        except Exception:
            val_to_nl[v] = v
        if (i + 1) % 80 == 0:
            print(f"{i + 1}/{len(uniq_vals)}", flush=True)
        time.sleep(0.06)

    nl = {k: val_to_nl[v] for k, v in en.items()}
    for k in nl:
        nl[k] = align_placeholders(en[k], nl[k])
    # Keep English label for the locale selector
    if nl.get("ui.opt.lang_en") in ("Engels", "engels"):
        nl["ui.opt.lang_en"] = "English"
    # Native language names in selector
    if "ui.opt.lang_de" in nl:
        nl["ui.opt.lang_de"] = "Deutsch"
    if "ui.opt.lang_es" in nl:
        nl["ui.opt.lang_es"] = "Español"
    if "ui.opt.lang_sv" in nl:
        nl["ui.opt.lang_sv"] = "Svenska"
    if "ui.opt.lang_fr" in nl:
        nl["ui.opt.lang_fr"] = "Français"
    if "ui.opt.lang_pt" in nl:
        nl["ui.opt.lang_pt"] = "Português"
    if "ui.opt.lang_nl" in nl:
        nl["ui.opt.lang_nl"] = "Nederlands"
    for k in list(nl.keys()):
        if "{Name}" in nl[k]:
            nl[k] = nl[k].replace("{Name}", "{name}")

    out_path.write_text(json.dumps(nl, ensure_ascii=False, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(f"Wrote {len(nl)} keys to {out_path}", file=sys.stderr)


if __name__ == "__main__":
    main()
