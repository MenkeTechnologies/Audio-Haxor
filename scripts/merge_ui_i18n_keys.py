#!/usr/bin/env python3
"""Deprecated — use merge_i18n_keys.py with a JSON batch under scripts/i18n_batches/.

See scripts/README-i18n.md
"""
from __future__ import annotations

import sys

def main() -> None:
    print(
        "merge_ui_i18n_keys.py is deprecated.\n"
        "  python3 scripts/merge_i18n_keys.py scripts/i18n_batches/your_batch.json\n"
        "See scripts/README-i18n.md",
        file=sys.stderr,
    )
    raise SystemExit(1)


if __name__ == "__main__":
    main()
