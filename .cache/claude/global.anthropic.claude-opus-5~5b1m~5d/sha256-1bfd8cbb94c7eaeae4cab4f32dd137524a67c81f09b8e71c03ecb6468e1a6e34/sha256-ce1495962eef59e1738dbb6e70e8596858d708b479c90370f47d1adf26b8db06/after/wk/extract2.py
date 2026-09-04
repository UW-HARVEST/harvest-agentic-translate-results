#!/usr/bin/env python3
"""Mechanical extraction of every rejection site, with multi-line joining."""
import os, re, json
from collections import Counter

ROOT = "c_src/src"
FILES = []
for d, _, fs in os.walk(ROOT):
    for f in sorted(fs):
        if f.endswith((".c", ".h")):
            FILES.append(os.path.join(d, f))
FILES.sort()

func_re = re.compile(r'^(?:[A-Za-z_][A-Za-z0-9_]*[ \t\*]+)+([A-Za-z_][A-Za-z0-9_]*)\s*\(')

SITE = re.compile(r'RETURN_ERROR_IF\s*\(|RETURN_ERROR\s*\(|return\s+ERROR\s*\(|return\s+NULL\s*;|return\s+-1\s*;|=\s*ERROR\s*\(')
ERRRX = re.compile(r'\b(?:ZSTD_error_|FSE_error_|HUF_error_)?([A-Za-z_][A-Za-z0-9_]*)\s*[,)]')

def kind_of(t):
    if 'RETURN_ERROR_IF' in t: return 'RETURN_ERROR_IF'
    if 'RETURN_ERROR' in t: return 'RETURN_ERROR'
    if re.search(r'return\s+ERROR', t): return 'return ERROR'
    if re.search(r'return\s+NULL', t): return 'return NULL'
    if re.search(r'return\s+-1', t): return 'return -1'
    return 'ERROR()'

rows = []
for path in FILES:
    lines = open(path, errors="replace").read().split("\n")
    cur = "?"
    for i, ln in enumerate(lines):
        if ln[:1] not in (" ", "\t", "", "#", "}", "/", "*"):
            m = func_re.match(ln)
            if m and m.group(1) not in ("if", "while", "for", "switch", "return", "sizeof"):
                cur = m.group(1)
        if not SITE.search(ln):
            continue
        joined = " ".join(x.strip() for x in lines[i:i + 4])
        # cut at the closing of the statement (first ';' after start) heuristically
        st = SITE.search(joined)
        seg = joined[st.start():]
        depth = 0
        end = len(seg)
        for j, ch in enumerate(seg):
            if ch == '(':
                depth += 1
            elif ch == ')':
                depth -= 1
                if depth == 0:
                    end = j + 1
                    break
            elif ch == ';' and depth == 0:
                end = j + 1
                break
        seg = seg[:end]
        k = kind_of(seg)
        code = "?"

        def args_of(s):
            """split top-level args of the first (...) group"""
            a = s[s.index('(') + 1:]
            res, d, cur2 = [], 0, ""
            for ch in a:
                if ch in '([': d += 1
                elif ch in ')]':
                    if d == 0: break
                    d -= 1
                elif ch == ',' and d == 0:
                    res.append(cur2); cur2 = ""; continue
                cur2 += ch
            res.append(cur2)
            return [x.strip() for x in res]

        if k == 'return NULL':
            code = 'NULL'
        elif k == 'return -1':
            code = '-1'
        elif k == 'RETURN_ERROR_IF':
            a = args_of(seg)
            code = a[1].replace('ZSTD_error_', '').replace('\\', '').strip() if len(a) > 1 else '?'
        elif k == 'RETURN_ERROR':
            a = args_of(seg)
            code = a[0].replace('ZSTD_error_', '').replace('\\', '').strip() if a else '?'
        else:
            m = re.search(r'(?:ZSTD_error_|FSE_error_|HUF_error_)([A-Za-z_][A-Za-z0-9_]*)', seg)
            if m:
                code = m.group(1)
            else:
                m = re.search(r'ERROR\s*\(\s*([A-Za-z_][A-Za-z0-9_]*)\s*\)', seg)
                if m:
                    code = m.group(1)
                else:
                    m = re.search(r'RETURN_ERROR_IF\s*\((.*)', seg)
                    code = 'macro-arg'
        cond = ""
        if k == 'RETURN_ERROR_IF':
            inner = seg[seg.index('(') + 1:]
            # first top-level comma
            d = 0
            for j, ch in enumerate(inner):
                if ch in '([': d += 1
                elif ch in ')]': d -= 1
                elif ch == ',' and d == 0:
                    cond = inner[:j]
                    break
            else:
                cond = inner
        rows.append({
            "file": path.replace("c_src/src/", ""),
            "line": i + 1,
            "func": cur,
            "kind": k,
            "code": code,
            "cond": cond.strip(),
            "text": re.sub(r'\s+', ' ', seg)[:180],
        })

json.dump(rows, open("wk/errrows.json", "w"), indent=0)
print("rows", len(rows))
for k, v in Counter(r["code"] for r in rows).most_common():
    print(f"  {k:40s} {v}")
