#!/usr/bin/env python3
"""Mechanically extract every rejection/error site from the C sources.

Emits TSV: file<TAB>line<TAB>enclosing_function<TAB>kind<TAB>text
"""
import os
import re
import sys

SRC = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "c_src", "src")

# libpng puts the return type on its own line and the name in column 0.
NAME_AT_COL0 = re.compile(r"^([A-Za-z_][A-Za-z0-9_]*)\s*\(")

KINDS = [
    ("png_error", re.compile(r"\bpng_error\s*\(")),
    ("png_chunk_error", re.compile(r"\bpng_chunk_error\s*\(")),
    ("png_app_error", re.compile(r"\bpng_app_error\s*\(")),
    ("png_benign_error", re.compile(r"\bpng_benign_error\s*\(")),
    ("png_chunk_benign_error", re.compile(r"\bpng_chunk_benign_error\s*\(")),
    ("png_chunk_report", re.compile(r"\bpng_chunk_report\s*\(")),
    ("png_warning", re.compile(r"\bpng_warning\s*\(")),
    ("png_chunk_warning", re.compile(r"\bpng_chunk_warning\s*\(")),
    ("png_app_warning", re.compile(r"\bpng_app_warning\s*\(")),
    ("png_fixed_error", re.compile(r"\bpng_fixed_error\s*\(")),
    ("return NULL", re.compile(r"\breturn\s*\(?\s*NULL\s*\)?\s*;")),
    ("return 0", re.compile(r"\breturn\s*\(?\s*0\s*\)?\s*;")),
    ("return -1", re.compile(r"\breturn\s*\(?\s*-1\s*\)?\s*;")),
    ("PNG_ABORT", re.compile(r"\bPNG_ABORT\s*\(")),
    ("handled-enum", re.compile(r"\breturn\s+handled\w*\s*;")),
]

SKIP_LINE = re.compile(r"^\s*(/\*|\*|//)")


def strip_literals(s):
    s = re.sub(r"'(\\.|[^'])*'", "''", s)
    s = re.sub(r'"(\\.|[^"\\])*"', '""', s)
    s = re.sub(r"/\*.*?\*/", "", s)
    s = re.sub(r"//.*$", "", s)
    return s


def enclosing(lines):
    out = [""] * len(lines)
    cur = ""
    pending = None
    depth = 0
    for i, raw in enumerate(lines):
        ln = raw.rstrip("\n")
        if depth == 0 and not SKIP_LINE.match(ln):
            m = NAME_AT_COL0.match(ln)
            if m and m.group(1) not in ("if", "for", "while", "switch", "return", "else"):
                pending = m.group(1)
        s = strip_literals(ln)
        o = s.count("{")
        c = s.count("}")
        if depth == 0 and o > 0 and pending:
            cur = pending
            pending = None
        depth += o - c
        if depth < 0:
            depth = 0
        out[i] = cur
    return out


def main():
    rows = []
    for fn in sorted(os.listdir(SRC)):
        if not fn.endswith(".c"):
            continue
        with open(os.path.join(SRC, fn), encoding="utf-8", errors="replace") as fh:
            lines = fh.readlines()
        encl = enclosing(lines)
        for i, raw in enumerate(lines):
            if SKIP_LINE.match(raw):
                continue
            code = strip_literals(raw)
            for kind, rx in KINDS:
                if rx.search(code):
                    rows.append((fn, i + 1, encl[i] or "?", kind, raw.strip()))
                    break
    for r in rows:
        print("\t".join(str(x) for x in r))
    sys.stderr.write("total rejection sites: %d\n" % len(rows))


if __name__ == "__main__":
    main()
