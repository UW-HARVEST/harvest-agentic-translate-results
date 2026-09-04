#!/usr/bin/env python3
"""Audit: every ERRORS.md / CONFIGS.md row must be cited by a test file.

Row citations are recognised in tests/ as:
  - explicit "row N" / "rows N-M" / "rows N, M" mentions in doc comments
  - test function names of the form rNN_... / cNN_... / rNN_MM_...
"""
import re, glob, os, sys

BASE = os.path.dirname(os.path.abspath(__file__))


def table_rows(path):
    rows = set()
    for line in open(os.path.join(BASE, path)):
        m = re.match(r'^\|\s*(\d+)\s*\|', line)
        if m:
            rows.add(int(m.group(1)))
    return rows


def cited(kind):
    """kind: 'errors' -> c*.rs files; 'configs' -> b*.rs files."""
    pat = 'tests/c*.rs' if kind == 'errors' else 'tests/b*.rs'
    got = set()
    for f in glob.glob(os.path.join(BASE, pat)):
        txt = open(f).read()
        # "row 12", "rows 12-19", "rows 12, 13, 14", "12,13,14"
        for m in re.finditer(r'\brows?\s+([0-9,\-\u2013 and]+)', txt, re.I):
            blob = m.group(1)
            for part in re.split(r'[,\s]+(?:and\s+)?', blob):
                part = part.strip()
                if not part:
                    continue
                r = re.match(r'^(\d+)[-\u2013](\d+)$', part)
                if r:
                    got.update(range(int(r.group(1)), int(r.group(2)) + 1))
                elif part.isdigit():
                    got.add(int(part))
        # fn names: r120_..., c207_..., r91_93_...
        for m in re.finditer(r'\bfn\s+[rc]((?:\d+_)+)', txt):
            for n in m.group(1).rstrip('_').split('_'):
                if n.isdigit():
                    got.add(int(n))
        # ranged fn names r75_81 style -> also fill the range
        for m in re.finditer(r'\bfn\s+[rc](\d+)_(\d+)_', txt):
            a, b = int(m.group(1)), int(m.group(2))
            if 0 < b - a <= 20:
                got.update(range(a, b + 1))
    return got


bad = 0
for tbl, kind in (('ERRORS.md', 'errors'), ('CONFIGS.md', 'configs')):
    rows = table_rows(tbl)
    got = cited(kind)
    missing = sorted(rows - got)
    print(f'{tbl}: {len(rows)} rows, {len(rows & got)} cited, {len(missing)} uncited')
    if missing:
        print('  UNCITED:', missing)
        bad = 1
sys.exit(bad)
