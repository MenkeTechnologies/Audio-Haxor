#!/usr/bin/env python3
"""Build i18n/app_i18n_vi.json from app_i18n_en.json (Vietnamese UI).

Requires: pip install deep-translator (use a venv, e.g. .venv-i18n).
"""
from __future__ import annotations

import json
import pathlib
import re
import sys
import time

ROOT = pathlib.Path(__file__).resolve().parents[1]
I18N_DIR = ROOT / "i18n"

LANG_SELECTOR_NATIVE = {
    "ui.opt.lang_cs": "Čeština",
    "ui.opt.lang_da": "Dansk",
    "ui.opt.lang_de": "Deutsch",
    "ui.opt.lang_el": "Ελληνικά",
    "ui.opt.lang_en": "English",
    "ui.opt.lang_es": "Español",
    "ui.opt.lang_fi": "Suomi",
    "ui.opt.lang_fr": "Français",
    "ui.opt.lang_hu": "Magyar",
    "ui.opt.lang_it": "Italiano",
    "ui.opt.lang_ja": "日本語",
    "ui.opt.lang_ko": "한국어",
    "ui.opt.lang_nb": "Norsk (bokmål)",
    "ui.opt.lang_nl": "Nederlands",
    "ui.opt.lang_pl": "Polski",
    "ui.opt.lang_pt": "Português",
    "ui.opt.lang_ro": "Română",
    "ui.opt.lang_ru": "Русский",
    "ui.opt.lang_sv": "Svenska",
    "ui.opt.lang_tr": "Türkçe",
    "ui.opt.lang_uk": "Українська",
    "ui.opt.lang_vi": "Tiếng Việt",
    "ui.opt.lang_zh": "简体中文",
}


def align_placeholders(en_val: str, loc_val: str) -> str:
    ph_en = re.findall(r"\{(\w+)\}", en_val)
    ph_loc = re.findall(r"\{(\w+)\}", loc_val)
    if len(ph_en) != len(ph_loc):
        return loc_val
    it = iter(ph_en)
    return re.sub(r"\{[^}]+\}", lambda _: "{" + next(it) + "}", loc_val)


def restore_ipc_placeholders(en_val: str, loc_val: str) -> str:
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
            ".venv-i18n/bin/pip install deep-translator && "
            ".venv-i18n/bin/python scripts/gen_app_i18n_vi.py",
            file=sys.stderr,
        )
        raise SystemExit(1) from None

    en_path = I18N_DIR / "app_i18n_en.json"
    out_path = I18N_DIR / "app_i18n_vi.json"
    en: dict[str, str] = json.loads(en_path.read_text(encoding="utf-8"))
    translator = GoogleTranslator(source="en", target="vi")

    uniq_vals = list(dict.fromkeys(en.values()))
    val_to: dict[str, str] = {}
    for i, v in enumerate(uniq_vals):
        try:
            val_to[v] = translator.translate(v)
        except Exception:
            val_to[v] = v
        if (i + 1) % 80 == 0:
            print(f"{i + 1}/{len(uniq_vals)}", flush=True)
        time.sleep(0.06)

    out_map = {k: val_to[v] for k, v in en.items()}
    for k in out_map:
        out_map[k] = align_placeholders(en[k], out_map[k])
        out_map[k] = restore_ipc_placeholders(en[k], out_map[k])
    for k, native in LANG_SELECTOR_NATIVE.items():
        if k in out_map:
            out_map[k] = native
    for k in list(out_map.keys()):
        if "{Name}" in out_map[k]:
            out_map[k] = out_map[k].replace("{Name}", "{name}")

    out_path.write_text(json.dumps(out_map, ensure_ascii=False, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(f"Wrote {len(out_map)} keys to {out_path}", file=sys.stderr)


if __name__ == "__main__":
    main()
