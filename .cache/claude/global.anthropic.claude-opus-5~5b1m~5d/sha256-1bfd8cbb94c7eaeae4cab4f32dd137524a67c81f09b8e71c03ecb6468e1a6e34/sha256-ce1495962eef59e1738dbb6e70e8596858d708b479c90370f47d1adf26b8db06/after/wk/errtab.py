#!/usr/bin/env python3
"""Build the ERROR-SURFACE TABLE rows: one row per distinct rejection site."""
import re, sys
from collections import Counter, defaultdict

rows = []
for line in open("wk/error_sites.tsv"):
    path, ln, fn, kind, text = line.rstrip("\n").split("\t", 4)
    if kind in ("assert", "FORWARD_IF_ERROR"):
        continue
    rows.append((path, int(ln), fn, kind, text))

errrx = re.compile(r'ZSTD_error_([A-Za-z_]+)')
frx = re.compile(r'FSE_error_([A-Za-z_]+)')

def errcode(text):
    m = errrx.search(text)
    if m:
        return "ZSTD_error_" + m.group(1)
    m = frx.search(text)
    if m:
        return "FSE_error_" + m.group(1)
    if re.search(r'return\s+NULL', text):
        return "NULL"
    if re.search(r'return\s+-1', text):
        return "-1"
    if re.search(r'ERROR\s*\(\s*([A-Za-z_]+)\s*\)', text):
        return "ERROR(" + re.search(r'ERROR\s*\(\s*([A-Za-z_]+)\s*\)', text).group(1) + ")"
    return "?"

out = []
for path, ln, fn, kind, text in rows:
    out.append((path.replace("c_src/src/", ""), ln, fn, kind, errcode(text), text))

print("rows:", len(out))
c = Counter(r[4] for r in out)
for k, v in c.most_common():
    print(f"  {k:50s} {v}")
byfile = Counter(r[0] for r in out)
print()
for k, v in byfile.most_common():
    print(f"  {k:45s} {v}")

import json
json.dump(out, open("wk/errrows.json", "w"))
