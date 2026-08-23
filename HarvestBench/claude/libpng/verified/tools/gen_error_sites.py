#!/usr/bin/env python3
"""Extract every diagnostic (error/warning) call site that is actually COMPILED
into the reference C library, and write `tests/error_sites.txt`
(`file:line|reporter|message`, one line per site).

The extraction runs the real C preprocessor (`cc -E -I c_src/include`) over each
`c_src/src/*.c`, so sites inside `#if`/`#else` branches that this build
configuration does not compile are excluded — they are dead code in
`libpng.so` and cannot be reached by any input.  `#line` markers are used to map
each hit back to its original file and line.

`tests/errors.rs` and `tools/error_coverage.py` read this file to report which of
libpng's rejection sites the differential tests actually reached.
"""
import glob
import os
import re
import subprocess
import sys

CALLS = ['png_error', 'png_chunk_error', 'png_app_error', 'png_benign_error',
         'png_chunk_benign_error', 'png_app_warning', 'png_warning',
         'png_chunk_warning', 'png_fixed_error']
CALLRE = re.compile(r'\b(' + '|'.join(CALLS) + r')\s*\(')
LINEMARK = re.compile(r'^#\s+(\d+)\s+"([^"]*)"')


def unescape(s):
    return (s.replace('\\n', '\n').replace('\\t', '\t')
             .replace('\\"', '"').replace("\\'", "'").replace('\\\\', '\\'))


def preprocess(path):
    r = subprocess.run(
        ['cc', '-E', '-std=c99', '-I', 'c_src/include', path],
        capture_output=True, text=True)
    if r.returncode != 0:
        sys.exit('cc -E failed on %s:\n%s' % (path, r.stderr))
    return r.stdout


rows = []
for src in sorted(glob.glob('c_src/src/*.c')):
    text = preprocess(src)
    lines = text.split('\n')
    cur_file, cur_line = src, 0
    i = 0
    while i < len(lines):
        l = lines[i]
        m = LINEMARK.match(l)
        if m:
            cur_line = int(m.group(1))
            cur_file = m.group(2)
            i += 1
            continue
        if CALLRE.search(l) and os.path.basename(cur_file).endswith('.c'):
            # gather the whole statement (the preprocessor keeps the original
            # line structure, so a call can still span several lines)
            stmt = l.strip()
            j = i
            while stmt.count('(') > stmt.count(')') and j + 1 < len(lines):
                j += 1
                nxt = lines[j]
                if LINEMARK.match(nxt):
                    break
                stmt += ' ' + nxt.strip()
            msgs = re.findall(r'"((?:[^"\\]|\\.)*)"', stmt)
            msg = unescape(''.join(msgs))
            rows.append((os.path.basename(cur_file), cur_line,
                         CALLRE.search(l).group(1), msg))
        cur_line += 1
        i += 1

# de-duplicate (a header-inlined call could appear twice)
seen = set()
out = []
for f, ln, kind, msg in rows:
    key = (f, ln, kind, msg)
    if key in seen:
        continue
    seen.add(key)
    out.append('%s:%d|%s|%s' % (f, ln, kind, msg))

open('tests/error_sites.txt', 'w').write('\n'.join(out) + '\n')
print('wrote tests/error_sites.txt: %d compiled diagnostic sites '
      '(%d with a literal message)'
      % (len(out), sum(1 for r in out if r.rsplit('|', 1)[1])))
