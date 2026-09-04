#!/usr/bin/env python3
"""Mechanically extract every rejection/error site from the zstd C sources."""
import os, re, sys

ROOT = "c_src/src"
FILES = []
for d, _, fs in os.walk(ROOT):
    for f in sorted(fs):
        if f.endswith((".c", ".h")):
            FILES.append(os.path.join(d, f))
FILES.sort()

func_re = re.compile(r'^[A-Za-z_][A-Za-z0-9_ \*\(\),]*\b([A-Za-z_][A-Za-z0-9_]*)\s*\(')

PATTERNS = [
    ("RETURN_ERROR_IF", re.compile(r'RETURN_ERROR_IF\s*\(')),
    ("RETURN_ERROR", re.compile(r'RETURN_ERROR\s*\(')),
    ("return ERROR", re.compile(r'return\s+ERROR\s*\(')),
    ("ERROR()", re.compile(r'\bERROR\s*\(\s*ZSTD_error')),
    ("FORWARD_IF_ERROR", re.compile(r'FORWARD_IF_ERROR\s*\(')),
    ("return -1", re.compile(r'return\s+-1\s*;')),
    ("return NULL", re.compile(r'return\s+NULL\s*;')),
    ("return 0/err", re.compile(r'return\s+ERROR')),
    ("assert", re.compile(r'(^|[^_A-Za-z])assert\s*\(')),
]

rows = []
for path in FILES:
    with open(path, errors="replace") as fh:
        lines = fh.readlines()
    cur = "?"
    for i, ln in enumerate(lines):
        s = ln.rstrip("\n")
        if ln and ln[0] not in " \t\n#/*}" and "(" in ln and ";" not in ln.split("(")[0]:
            m = func_re.match(ln)
            if m:
                cur = m.group(1)
        # also catch "static size_t NAME(" style at col0 handled above
        for kind, rx in PATTERNS:
            if rx.search(s):
                rows.append((path, i + 1, cur, kind, s.strip()))
                break

# print summary
from collections import Counter
c = Counter(r[3] for r in rows)
for k, v in c.most_common():
    print(f"{k:20s} {v}")
print("TOTAL", len(rows))

with open("wk/error_sites.tsv", "w") as out:
    for r in rows:
        out.write("\t".join(str(x) for x in r) + "\n")
