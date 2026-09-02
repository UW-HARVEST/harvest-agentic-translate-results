#!/usr/bin/env python3
"""Mechanically dump every error-return site in the C source, with enclosing function."""
import re, sys, os, glob, collections

ROOT = os.path.abspath(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", ".."))

SRC = os.path.join(ROOT, "c_src", "src")

func_re = re.compile(r'^([A-Za-z_][A-Za-z0-9_ \*]*\b)?([a-zA-Z_][a-zA-Z0-9_]*)\s*\(')

def enclosing_funcs(path):
    """Return list of (lineno, funcname) for top-level function definitions."""
    out = []
    lines = open(path, encoding='utf-8', errors='replace').read().split('\n')
    for i, l in enumerate(lines):
        # a function body opening brace at column 0 preceded by a signature
        if l.startswith('{'):
            # walk back to find the signature line
            j = i - 1
            while j >= 0 and (lines[j].strip() == '' or lines[j].lstrip().startswith('*') or lines[j].lstrip().startswith('/*')):
                j -= 1
            sig = []
            while j >= 0 and lines[j].strip() != '' and not lines[j].startswith('*'):
                sig.insert(0, lines[j])
                if '(' in lines[j]:
                    break
                j -= 1
            s = ' '.join(x.strip() for x in sig)
            m = re.search(r'([A-Za-z_][A-Za-z0-9_]*)\s*\(', s)
            if m:
                out.append((i + 1, m.group(1)))
    return out, lines

def func_at(funcs, lineno):
    name = "(file scope)"
    for ln, nm in funcs:
        if ln <= lineno:
            name = nm
        else:
            break
    return name

PATS = [
    ('PCRE2_ERROR', re.compile(r'\bPCRE2_ERROR_[A-Z0-9_]+')),
    ('ERRn',        re.compile(r'\bERR\d+\b')),
    ('assert',      re.compile(r'\bPCRE2_ASSERT\s*\(|\bassert\s*\(|\bPCRE2_DEBUG_UNREACHABLE\s*\(')),
    ('ret_null',    re.compile(r'return\s+NULL\s*;')),
    ('ret_neg',     re.compile(r'return\s+-\d+\s*;')),
]

results = collections.defaultdict(list)
for path in sorted(glob.glob(os.path.join(SRC, '*.c'))) + sorted(glob.glob(os.path.join(SRC, '*.h'))):
    funcs, lines = enclosing_funcs(path)
    base = os.path.basename(path)
    for i, l in enumerate(lines, 1):
        ls = l.lstrip()
        if ls.startswith('/*') or ls.startswith('*/') or re.match(r'^\*(\s|$)', ls):
            continue
        for kind, rx in PATS:
            for m in rx.finditer(l):
                results[kind].append((base, i, func_at(funcs, i), m.group(0), l.strip()[:150]))

which = sys.argv[1] if len(sys.argv) > 1 else 'PCRE2_ERROR'
if which == 'summary':
    for k, v in results.items():
        print(k, len(v))
    sys.exit()
for base, i, fn, tok, txt in results[which]:
    print(f"{base}:{i}\t{fn}\t{tok}\t{txt}")
