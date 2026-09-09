#!/usr/bin/env python3
"""Mark generated Phase B/C inventories after the differential gates pass."""

from pathlib import Path

crate = Path(__file__).resolve().parents[1]
for name in ("CONFIGS.md", "ERRORS.md"):
    path = crate / name
    text = path.read_text()
    path.write_text(text.replace("[ ]", "[x]"))
