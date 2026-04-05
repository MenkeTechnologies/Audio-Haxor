#!/usr/bin/env python3
"""Run every `scripts/gen_app_i18n_<locale>.py` generator (except English) in sequence.

Rebuilds `i18n/app_i18n_*.json` from `i18n/app_i18n_en.json` via Google Translate
(`deep-translator`). Slow (~tens of minutes per locale on a good network); safe to
interrupt and re-run — each script is idempotent for a given English catalog.

Requires: `.venv-i18n` with `pip install deep-translator` (see `scripts/README-i18n.md`).

Usage:
  .venv-i18n/bin/python scripts/gen_all_app_i18n_locales.py
"""
from __future__ import annotations

import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

# Order: European locales first, then CJK (same as README “full MT” block).
LOCALE_SCRIPTS: tuple[tuple[str, str], ...] = (
    ("de", "gen_app_i18n_de.py"),
    ("es", "gen_app_i18n_es.py"),
    ("sv", "gen_app_i18n_sv.py"),
    ("fr", "gen_app_i18n_fr.py"),
    ("nl", "gen_app_i18n_nl.py"),
    ("pt", "gen_app_i18n_pt.py"),
    ("it", "gen_app_i18n_it.py"),
    ("el", "gen_app_i18n_el.py"),
    ("pl", "gen_app_i18n_pl.py"),
    ("ru", "gen_app_i18n_ru.py"),
    ("zh", "gen_app_i18n_zh.py"),
    ("ja", "gen_app_i18n_ja.py"),
    ("ko", "gen_app_i18n_ko.py"),
    ("fi", "gen_app_i18n_fi.py"),
)


def main() -> None:
    failed: list[str] = []
    for loc, script in LOCALE_SCRIPTS:
        path = ROOT / "scripts" / script
        if not path.is_file():
            print(f"SKIP missing {path}", file=sys.stderr)
            failed.append(loc)
            continue
        print(f"\n=== [{loc}] {script} ===\n", flush=True)
        r = subprocess.run([sys.executable, str(path)], cwd=str(ROOT))
        if r.returncode != 0:
            print(f"ERROR: {script} exited {r.returncode}", file=sys.stderr)
            failed.append(loc)
    if failed:
        raise SystemExit(f"Failed locales: {', '.join(failed)}")
    print("\nAll locale generators finished OK.", file=sys.stderr)


if __name__ == "__main__":
    main()
