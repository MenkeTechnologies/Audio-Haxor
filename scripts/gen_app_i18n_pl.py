#!/usr/bin/env python3
"""Build i18n/app_i18n_pl.json from app_i18n_en.json (Polish UI).

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


def align_placeholders(en_val: str, pl_val: str) -> str:
    """MT often translates `{name}` inside braces; keep English token names for appFmt."""
    ph_en = re.findall(r"\{(\w+)\}", en_val)
    ph_pl = re.findall(r"\{(\w+)\}", pl_val)
    if len(ph_en) != len(ph_pl):
        return pl_val
    it = iter(ph_en)
    return re.sub(r"\{[^}]+\}", lambda _: "{" + next(it) + "}", pl_val)


def restore_ipc_placeholders(en_val: str, loc_val: str) -> str:
    """Same token names as `app_i18n::tests` / `ipc.js` — MT may rewrite `{uptime}` in braces."""
    re_en = re.compile(r"\{[a-zA-Z_][a-zA-Z0-9_]*\}")
    re_any = re.compile(r"\{[^}]+\}")
    en_phs = re_en.findall(en_val)
    if not en_phs:
        return loc_val
    loc_phs = re_any.findall(loc_val)
    if len(loc_phs) != len(en_phs):
        return loc_val
    out = loc_val
    for wrong, right in zip(loc_phs, en_phs):
        if wrong != right:
            out = out.replace(wrong, right, 1)
    return out


def main() -> None:
    try:
        from deep_translator import GoogleTranslator
    except ImportError:
        print(
            "Install deep-translator in a venv: python3 -m venv .venv-i18n && "
            ".venv-i18n/bin/pip install deep-translator && .venv-i18n/bin/python scripts/gen_app_i18n_pl.py",
            file=sys.stderr,
        )
        raise SystemExit(1) from None

    en_path = I18N_DIR / "app_i18n_en.json"
    out_path = I18N_DIR / "app_i18n_pl.json"
    en: dict[str, str] = json.loads(en_path.read_text(encoding="utf-8"))
    translator = GoogleTranslator(source="en", target="pl")

    uniq_vals = list(dict.fromkeys(en.values()))
    val_to_pl: dict[str, str] = {}
    for i, v in enumerate(uniq_vals):
        try:
            val_to_pl[v] = translator.translate(v)
        except Exception:
            val_to_pl[v] = v
        if (i + 1) % 80 == 0:
            print(f"{i + 1}/{len(uniq_vals)}", flush=True)
        time.sleep(0.06)

    pl_map = {k: val_to_pl[v] for k, v in en.items()}
    for k in pl_map:
        pl_map[k] = align_placeholders(en[k], pl_map[k])
        pl_map[k] = restore_ipc_placeholders(en[k], pl_map[k])
    if pl_map.get("ui.opt.lang_en") in ("Angielski", "angielski"):
        pl_map["ui.opt.lang_en"] = "English"
    if "ui.opt.lang_de" in pl_map:
        pl_map["ui.opt.lang_de"] = "Deutsch"
    if "ui.opt.lang_es" in pl_map:
        pl_map["ui.opt.lang_es"] = "Español"
    if "ui.opt.lang_sv" in pl_map:
        pl_map["ui.opt.lang_sv"] = "Svenska"
    if "ui.opt.lang_fr" in pl_map:
        pl_map["ui.opt.lang_fr"] = "Français"
    if "ui.opt.lang_it" in pl_map:
        pl_map["ui.opt.lang_it"] = "Italiano"
    if "ui.opt.lang_el" in pl_map:
        pl_map["ui.opt.lang_el"] = "Ελληνικά"
    if "ui.opt.lang_pt" in pl_map:
        pl_map["ui.opt.lang_pt"] = "Português"
    if "ui.opt.lang_nl" in pl_map:
        pl_map["ui.opt.lang_nl"] = "Nederlands"
    if "ui.opt.lang_pl" in pl_map:
        pl_map["ui.opt.lang_pl"] = "Polski"
    for k in list(pl_map.keys()):
        if "{Name}" in pl_map[k]:
            pl_map[k] = pl_map[k].replace("{Name}", "{name}")

    out_path.write_text(json.dumps(pl_map, ensure_ascii=False, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(f"Wrote {len(pl_map)} keys to {out_path}", file=sys.stderr)


if __name__ == "__main__":
    main()
