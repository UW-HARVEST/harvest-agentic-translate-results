#!/usr/bin/env python3
"""Apply a single literal text substitution to a file (used by mutation_check.sh).

Usage: mutate.py <file> <old-text> <new-text>
Exits non-zero if the old text is not present, so a stale mutation target can
never be silently skipped.
"""
import sys

if len(sys.argv) != 4:
    sys.exit("usage: mutate.py <file> <old> <new>")

path, old, new = sys.argv[1], sys.argv[2], sys.argv[3]
with open(path) as fh:
    src = fh.read()

if old not in src:
    sys.exit("MUTATION TARGET NOT FOUND:\n" + old)

with open(path, "w") as fh:
    fh.write(src.replace(old, new, 1))
