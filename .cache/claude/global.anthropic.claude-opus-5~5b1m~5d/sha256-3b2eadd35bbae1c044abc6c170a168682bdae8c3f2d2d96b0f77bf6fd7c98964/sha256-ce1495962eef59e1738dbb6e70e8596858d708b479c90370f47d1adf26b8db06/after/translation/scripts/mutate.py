#!/usr/bin/env python3
"""Sensitivity check for the differential suite: swap `old` for `new` in
translation/src/lib.rs, so a run of `cargo test` can be confirmed to FAIL.
Always run it a second time with the arguments reversed to restore the file."""
import sys, pathlib

p = pathlib.Path(__file__).resolve().parent.parent / "src" / "lib.rs"
old, new = sys.argv[1], sys.argv[2]
s = p.read_text()
if old not in s:
    sys.exit(f"PATTERN NOT FOUND: {old!r}")
p.write_text(s.replace(old, new, 1))
print(f"mutated: {old!r} -> {new!r}")
