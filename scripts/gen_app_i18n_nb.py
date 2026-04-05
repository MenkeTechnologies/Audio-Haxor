#!/usr/bin/env python3
"""Build i18n/app_i18n_nb.json from app_i18n_en.json (Norwegian Bokmål UI).

Requires: pip install deep-translator (use a venv, e.g. .venv-i18n).

Google Translate uses language code `no` for Norwegian; app locale id is `nb`.
"""
from __future__ import annotations

import json
import pathlib
import re
import sys
import time

ROOT = pathlib.Path(__file__).resolve().parents[1]
I18N_DIR = ROOT / "i18n"

# Native names for the interface-language dropdown (do not leave as MT gibberish)
LANG_SELECTOR_NATIVE = {
    "ui.opt.lang_da": "Dansk",
    "ui.opt.lang_de": "Deutsch",
    "ui.opt.lang_el": "Ελληνικά",
    "ui.opt.lang_en": "English",
    "ui.opt.lang_es": "Español",
    "ui.opt.lang_fi": "Suomi",
    "ui.opt.lang_fr": "Français",
    "ui.opt.lang_it": "Italiano",
    "ui.opt.lang_ja": "日本語",
    "ui.opt.lang_ko": "한국어",
    "ui.opt.lang_nb": "Norsk (bokmål)",
    "ui.opt.lang_nl": "Nederlands",
    "ui.opt.lang_pl": "Polski",
    "ui.opt.lang_pt": "Português",
    "ui.opt.lang_ru": "Русский",
    "ui.opt.lang_sv": "Svenska",
    "ui.opt.lang_tr": "Türkçe",
    "ui.opt.lang_zh": "简体中文",
}


def align_placeholders(en_val: str, nb_val: str) -> str:
    """MT often translates `{name}` inside braces; keep English token names for appFmt."""
    ph_en = re.findall(r"\{(\w+)\}", en_val)
    ph_nb = re.findall(r"\{(\w+)\}", nb_val)
    if len(ph_en) != len(ph_nb):
        return nb_val
    it = iter(ph_en)
    return re.sub(r"\{[^}]+\}", lambda _: "{" + next(it) + "}", nb_val)


def main() -> None:
    try:
        from deep_translator import GoogleTranslator
    except ImportError:
        print(
            "Install deep-translator in a venv: python3 -m venv .venv-i18n && "
            ".venv-i18n/bin/pip install deep-translator && .venv-i18n/bin/python scripts/gen_app_i18n_nb.py",
            file=sys.stderr,
        )
        raise SystemExit(1) from None

    en_path = I18N_DIR / "app_i18n_en.json"
    out_path = I18N_DIR / "app_i18n_nb.json"
    en: dict[str, str] = json.loads(en_path.read_text(encoding="utf-8"))
    translator = GoogleTranslator(source="en", target="no")

    uniq_vals = list(dict.fromkeys(en.values()))
    val_to_nb: dict[str, str] = {}
    for i, v in enumerate(uniq_vals):
        try:
            val_to_nb[v] = translator.translate(v)
        except Exception:
            val_to_nb[v] = v
        if (i + 1) % 80 == 0:
            print(f"{i + 1}/{len(uniq_vals)}", flush=True)
        time.sleep(0.06)

    nb_map = {k: val_to_nb[v] for k, v in en.items()}
    for k in nb_map:
        nb_map[k] = align_placeholders(en[k], nb_map[k])
    for k, native in LANG_SELECTOR_NATIVE.items():
        if k in nb_map:
            nb_map[k] = native
    for k in list(nb_map.keys()):
        if "{Name}" in nb_map[k]:
            nb_map[k] = nb_map[k].replace("{Name}", "{name}")
        if "{Wert}" in nb_map[k]:
            nb_map[k] = nb_map[k].replace("{Wert}", "{value}")

    out_path.write_text(json.dumps(nb_map, ensure_ascii=False, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(f"Wrote {len(nb_map)} keys to {out_path}", file=sys.stderr)


if __name__ == "__main__":
    main()
