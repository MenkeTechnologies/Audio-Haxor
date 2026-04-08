#!/usr/bin/env python3
"""Fill non-English locale keys that still match English (stub drift after sync).

For each shipped locale, finds keys where `app_i18n_<loc>.json` equals `app_i18n_en.json`,
skips branding / language-selector endonyms / indirection placeholders, translates each
**distinct** English string once via Google Translate, then maps keys — same efficiency
idea as `gen_app_i18n_*.py`, but only touches gap keys so existing good translations stay.

Requires: pip install deep-translator (e.g. `.venv-i18n`).

Usage:
  .venv-i18n/bin/python scripts/fill_all_locale_i18n_gaps.py
  .venv-i18n/bin/python scripts/fill_all_locale_i18n_gaps.py --locale de

Run `de_i18n_manual_overrides.py` after this if you refreshed `de`.
"""
from __future__ import annotations

import argparse
import json
import pathlib
import re
import sys
import time

ROOT = pathlib.Path(__file__).resolve().parents[1]
I18N = ROOT / "i18n"

# (file suffix without app_i18n_ / .json, GoogleTranslate target code)
# Targets match `scripts/gen_app_i18n_*.py`.
LOCALE_TARGETS: tuple[tuple[str, str], ...] = (
    ("de", "de"),
    ("es", "es"),
    ("es_419", "es"),
    ("sv", "sv"),
    ("fr", "fr"),
    ("nl", "nl"),
    ("pt", "pt"),
    ("pt_br", "pt"),
    ("it", "it"),
    ("el", "el"),
    ("pl", "pl"),
    ("ru", "ru"),
    ("zh", "zh-CN"),
    ("ja", "ja"),
    ("ko", "ko"),
    ("fi", "fi"),
    ("da", "da"),
    ("nb", "no"),
    ("tr", "tr"),
    ("cs", "cs"),
    ("hu", "hu"),
    ("ro", "ro"),
    ("uk", "uk"),
    ("vi", "vi"),
    ("id", "id"),
    ("hi", "hi"),
)

SKIP_KEYS_EXACT = frozenset(
    {
        "menu.app",
        "tray.tooltip",
        "ui.logo.app_name",
        "ui.page_title",
        "ui.st.audio_haxor",
        "ui.opt.lang_en",
    }
)


def keep_english_branding(v: str) -> bool:
    return v.strip() in ("AUDIO_HAXOR", "AUDIO HAXOR")


def align_brace_tokens(en_val: str, loc_val: str) -> str:
    """Keep English `{token}` names for appFmt when MT rewrites brace contents."""
    if not isinstance(loc_val, str):
        return en_val
    ph_en = re.findall(r"\{(\w+)\}", en_val)
    ph_loc = re.findall(r"\{(\w+)\}", loc_val)
    if len(ph_en) != len(ph_loc):
        return loc_val
    it = iter(ph_en)
    return re.sub(r"\{[^}]+\}", lambda _: "{" + next(it) + "}", loc_val)


def gap_keys(en: dict[str, str], loc: dict[str, str]) -> list[str]:
    return [
        k
        for k, v in en.items()
        if loc.get(k) == v
        and k not in SKIP_KEYS_EXACT
        and not k.startswith("ui.ph.ui_ph_")
        and not k.startswith("ui.opt.lang_")
        and not keep_english_branding(v)
    ]


def fill_one_locale(
    en: dict[str, str],
    loc: dict[str, str],
    suffix: str,
    google_target: str,
    translator,
    ph_re: re.Pattern[str],
    sleep_s: float,
) -> tuple[dict[str, str], int]:
    to_fix = gap_keys(en, loc)
    if not to_fix:
        return loc, 0

    uniq_vals = list(dict.fromkeys(en[k] for k in to_fix))
    val_to_t: dict[str, str] = {}
    for i, v in enumerate(uniq_vals):
        try:
            t = translator.translate(v)
        except Exception:
            t = v
        if t is None or not isinstance(t, str):
            t = v
        en_ph = ph_re.findall(v)
        for p in en_ph:
            if p not in t:
                t = v
                break
        if t != v:
            t = align_brace_tokens(v, t)
        val_to_t[v] = t
        if (i + 1) % 80 == 0:
            print(f"  [{suffix}] {i + 1}/{len(uniq_vals)}", flush=True)
        time.sleep(sleep_s)

    out = dict(loc)
    for k in to_fix:
        v = en[k]
        out[k] = val_to_t[v]

    for k in list(out.keys()):
        if "{Name}" in out[k]:
            out[k] = out[k].replace("{Name}", "{name}")
        if "{Wert}" in out[k]:
            out[k] = out[k].replace("{Wert}", "{value}")

    return out, len(to_fix)


def main() -> None:
    try:
        from deep_translator import GoogleTranslator
    except ImportError:
        print(
            "Install: python3 -m venv .venv-i18n && .venv-i18n/bin/pip install deep-translator",
            file=sys.stderr,
        )
        raise SystemExit(1) from None

    ap = argparse.ArgumentParser()
    ap.add_argument(
        "--locale",
        help="Only process this file suffix (e.g. de, es_419, pt_br)",
    )
    ap.add_argument(
        "--sleep",
        type=float,
        default=0.07,
        help="Seconds between Google Translate calls (default 0.07)",
    )
    args = ap.parse_args()

    en_path = I18N / "app_i18n_en.json"
    en: dict[str, str] = json.loads(en_path.read_text(encoding="utf-8"))
    ph_re = re.compile(r"\{[a-zA-Z_][a-zA-Z0-9_]*\}")

    pairs = LOCALE_TARGETS
    if args.locale:
        want = args.locale.strip()
        pairs = tuple((s, t) for s, t in LOCALE_TARGETS if s == want)
        if not pairs:
            raise SystemExit(f"Unknown --locale {want!r}; not in LOCALE_TARGETS")

    for suffix, google_target in pairs:
        path = I18N / f"app_i18n_{suffix}.json"
        loc: dict[str, str] = json.loads(path.read_text(encoding="utf-8"))
        translator = GoogleTranslator(source="en", target=google_target)
        print(f"=== {suffix} (target={google_target}) ===", flush=True)
        updated, n_filled = fill_one_locale(
            en, loc, suffix, google_target, translator, ph_re, args.sleep
        )
        path.write_text(
            json.dumps(updated, ensure_ascii=False, indent=2, sort_keys=True) + "\n",
            encoding="utf-8",
        )
        n_remain = len(gap_keys(en, updated))
        print(
            f"Wrote {path} (filled {n_filled} keys; {n_remain} still match English)",
            flush=True,
        )


if __name__ == "__main__":
    main()
