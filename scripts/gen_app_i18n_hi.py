#!/usr/bin/env python3
"""Build i18n/app_i18n_hi.json from app_i18n_en.json (Hindi UI).

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
    "ui.opt.lang_es_419": "Español (Latinoamérica)",
    "ui.opt.lang_fi": "Suomi",
    "ui.opt.lang_fr": "Français",
    "ui.opt.lang_hi": "हिन्दी",
    "ui.opt.lang_hu": "Magyar",
    "ui.opt.lang_id": "Bahasa Indonesia",
    "ui.opt.lang_it": "Italiano",
    "ui.opt.lang_ja": "日本語",
    "ui.opt.lang_ko": "한국어",
    "ui.opt.lang_nb": "Norsk (bokmål)",
    "ui.opt.lang_nl": "Nederlands",
    "ui.opt.lang_pl": "Polski",
    "ui.opt.lang_pt": "Português",
    "ui.opt.lang_pt_br": "Português (Brasil)",
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
            ".venv-i18n/bin/python scripts/gen_app_i18n_hi.py",
            file=sys.stderr,
        )
        raise SystemExit(1) from None

    en_path = I18N_DIR / "app_i18n_en.json"
    out_path = I18N_DIR / "app_i18n_hi.json"
    en: dict[str, str] = json.loads(en_path.read_text(encoding="utf-8"))
    translator = GoogleTranslator(source="en", target="hi")

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
    # MT often drops `{count}` for this string in Hindi; `appFmt` requires the English token name.
    k_pdf = "ui.history.footer_pdfs_other"
    if k_pdf in out_map and "{count}" not in out_map[k_pdf]:
        out_map[k_pdf] = "{count} PDF इस स्कैन में"
    # Long AU insert tooltip: MT often leaves English; keep stable Hindi aligned with `app_i18n_en.json` meaning.
    k_au = "ui.ae.insert_hide_audio_units"
    k_au_tt = "ui.ae.insert_hide_audio_units_tt"
    if k_au in out_map:
        out_map[k_au] = "Audio Units छिपाएं"
    if k_au_tt in out_map:
        out_map[k_au_tt] = (
            "इस बिल्ड में AU प्लगइन इडिटर विंडो खाली रहती हैं क्योंकि audiocomponentd "
            "ad-hoc हस्ताक्षरित होस्ट्स को प्रोसेस-बाहर AU व्यू डिलीवरी अस्वीकार करता है। "
            "जब तक प्रोजेक्ट वास्तविक Developer ID से हस्ताक्षरित न हो, इंसर्ट पिकर से AU छिपाएँ (डिफ़ॉल्ट)। "
            "यदि आपको केवल AU ऑडियो प्रोसेसिंग चाहिए और इडिटर नहीं खोलना, तो बंद करें।"
        )

    out_path.write_text(json.dumps(out_map, ensure_ascii=False, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(f"Wrote {len(out_map)} keys to {out_path}", file=sys.stderr)


if __name__ == "__main__":
    main()
