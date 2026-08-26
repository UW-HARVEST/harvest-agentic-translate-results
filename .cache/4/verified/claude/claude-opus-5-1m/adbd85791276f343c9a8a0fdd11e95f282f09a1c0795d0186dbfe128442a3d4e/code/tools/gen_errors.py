#!/usr/bin/env python3
"""Regenerate ERRORS.md — the error-surface table — mechanically from the C source.

Three independent scans, all restricted to code the reference build actually
compiles (the site lists come from `cc -E`, see `tools/gen_error_sites.py` and
`compiled_sites()` below, so `#if`-disabled branches are excluded: they are dead
code in `libpng.so` and unreachable by any input):

  Part 1  every call to png_error / png_chunk_error / png_fixed_error /
          png_app_error / png_app_warning / png_benign_error /
          png_chunk_benign_error / png_warning / png_chunk_warning
  Part 2  every guarded `return 0 / NULL / -1`   (sentinel-return rejection)
  Part 3  every guarded `return;`                (silent rejection)
  Part 4  the generic FFI-boundary rejections, covered by tests/sweep.rs

The `trigger` column is the enclosing/preceding control condition, taken verbatim
from the C source; the `expected C result` column is what that reporter does in
this build configuration.
"""
import glob
import os
import re
import subprocess
import sys

LINEMARK = re.compile(r'^#\s+(\d+)\s+"([^"]*)"')


def preprocess(path):
    r = subprocess.run(['cc', '-E', '-std=c99', '-I', 'c_src/include', path],
                       capture_output=True, text=True)
    if r.returncode != 0:
        sys.exit('cc -E failed on %s:\n%s' % (path, r.stderr))
    return r.stdout


def compiled_lines(src):
    """set of original line numbers of `src` that survive preprocessing."""
    keep = set()
    cur_file, cur_line = src, 0
    base = os.path.basename(src)
    for l in preprocess(src).split('\n'):
        m = LINEMARK.match(l)
        if m:
            cur_line = int(m.group(1))
            cur_file = m.group(2)
            continue
        if os.path.basename(cur_file) == base and l.strip():
            keep.add(cur_line)
        cur_line += 1
    return keep


def collapse(s):
    s = re.sub(r'/\*.*?\*/', ' ', s)
    return re.sub(r'\s+', ' ', s).strip()


def indent(l):
    return len(l) - len(l.lstrip())


def full_stmt(lines, i):
    stmt = lines[i].strip()
    j = i
    while stmt.count('(') > stmt.count(')') and j + 1 < len(lines):
        j += 1
        stmt += ' ' + lines[j].strip()
    return collapse(stmt)


def guard(lines, i, depth=2):
    """The nearest enclosing / preceding control conditions, verbatim."""
    base = indent(lines[i])
    out = []
    k = i - 1
    while k >= 0 and len(out) < depth:
        l = lines[k]
        s = l.strip()
        if not s or s.startswith('*') or s.startswith('/'):
            k -= 1
            continue
        ind = indent(l)
        if re.match(r'^(if|else if|while|for|switch)\b', s) and ind <= base:
            out.append(full_stmt(lines, k))
            base = ind
        elif re.match(r'^(case|default)\b', s) and ind <= base:
            out.append(collapse(s))
            base = ind
        elif s.startswith('}') and ind < base:
            base = ind
        k -= 1
    return ' <- '.join(out) if out else '(unconditional in this function)'


def func_of(lines):
    starts = {}
    for i, l in enumerate(lines):
        if not l or not (l[0].isalpha() or l[0] == '_'):
            continue
        m = re.match(r'^([A-Za-z_][A-Za-z0-9_]*)\s*\(', l)
        if not m or l.rstrip().endswith(';'):
            continue
        n = m.group(1)
        if n in ('if', 'for', 'while', 'switch', 'return', 'sizeof',
                 'PNG_UNUSED', 'else'):
            continue
        if n == 'PNG_FUNCTION':
            m2 = re.search(
                r'PNG_FUNCTION\s*\([^,]*,\s*(?:\w+\s+)?([A-Za-z_][A-Za-z0-9_]*)', l)
            if m2:
                n = m2.group(1)
        starts[i] = n
    keys = sorted(starts)

    def of(idx):
        best = '<file scope>'
        for k in keys:
            if k <= idx:
                best = starts[k]
            else:
                break
        return best
    return of


RESULT = {
    'png_error': 'fatal: error_fn(msg) then png_default_error -> png_longjmp; never returns',
    'png_chunk_error': 'fatal: error_fn("<chunk>: msg") -> png_longjmp; never returns',
    'png_fixed_error': 'fatal: png_error("<text>: fixed point overflow"); never returns',
    'png_app_error': 'PNG_FLAG_APP_ERRORS_WARN set -> png_warning, else png_error (fatal)',
    'png_app_warning': 'PNG_FLAG_APP_WARNINGS_WARN set -> png_warning, else png_error (fatal)',
    'png_benign_error': 'read struct: benign-errors-warn -> png_chunk_warning else png_chunk_error; write struct: png_error (fatal in this build)',
    'png_chunk_benign_error': 'benign-errors-warn -> png_chunk_warning, else png_chunk_error (fatal)',
    'png_warning': 'non-fatal: warning_fn(msg); the caller continues',
    'png_chunk_warning': 'non-fatal: warning_fn("<chunk>: msg"); the caller continues',
}

SENT = re.compile(r'^return\s*\(?\s*(0|NULL|-1|0U|\(png_uint_32\)0)\s*\)?\s*;')

sources = sorted(glob.glob('c_src/src/*.c'))
raw = {}
keep = {}
for s in sources:
    raw[s] = open(s, encoding='utf-8', errors='replace').read().split('\n')
    keep[s] = compiled_lines(s)

# ---- Part 1: from tests/error_sites.txt (already preprocessor-filtered) -----
sites = []
for line in open('tests/error_sites.txt', encoding='utf-8'):
    line = line.rstrip('\n')
    if not line:
        continue
    loc, kind, msg = line.split('|', 2)
    f, ln = loc.split(':')
    sites.append((f, int(ln), kind, msg))

# ---- Parts 2 and 3 ----------------------------------------------------------
part2, part3 = [], []
for s in sources:
    lines = raw[s]
    of = func_of(lines)
    base = os.path.basename(s)
    for i, l in enumerate(lines):
        if (i + 1) not in keep[s]:
            continue
        t = l.strip()
        g = guard(lines, i, 1)
        if g == '(unconditional in this function)':
            continue
        if SENT.match(t):
            part2.append((base, i + 1, of(i), t, g))
        elif t == 'return;':
            part3.append((base, i + 1, of(i), t, g))

out = []
w = out.append
w('# ERRORS.md — the error surface of libpng (C ground truth)')
w('')
w('Every row is a distinct place where the C library rejects or complains about an')
w('input.  The table is generated by `tools/gen_errors.py`; do not edit it by hand.')
w('')
w('All three scans are restricted to code the reference build actually **compiles**:')
w('the line lists come from the real C preprocessor (`cc -E -std=c99 -I')
w('c_src/include`), so a diagnostic inside an `#if`/`#else` branch that')
w('`c_src/include/pnglibconf.h` disables is excluded — it is dead code in')
w('`libpng.so` and no input can reach it.  (That removes, for example, the')
w('`"PNG_WRITE_BGR_SUPPORTED is not defined"` family in `pngwrite.c`.)')
w('')
w('| part | what | rows |')
w('|------|------|------|')
w('| 1 | diagnostic rejections (`png_error`, `png_warning`, `png_chunk_*`, `png_app_*`, `png_benign_error`, `png_fixed_error`) | %d |' % len(sites))
w('| 2 | silent sentinel-return rejections (`return 0 / NULL / -1` behind a check) | %d |' % len(part2))
w('| 3 | silent void-return rejections (`return;` behind a check) | %d |' % len(part3))
w('| 4 | generic FFI-boundary rejections (NULL, out-of-range enum, bad length) | 6 |')
w('')
w('**`assert()`**: libpng contains none.  `grep -n assert c_src/src/*.c` finds')
w('exactly three hits and all three are comments (`png.c:116`, `pngrtran.c:2452`,')
w('`pngrtran.c:2467`).  The only abort path is `PNG_ABORT()` (`= abort()`,')
w('`pngpriv.h:580`) at `pngerror.c:690`, reached from `png_longjmp` when the')
w('application installed neither a `jmp_buf` nor a `png_longjmp_fn` — row **G-1**')
w('in part 4.')
w('')
w('**min/max constants that gate input** (each is the bound of at least one row')
w('below): `PNG_UINT_31_MAX` (0x7fffffff), `PNG_UINT_32_MAX`, `PNG_SIZE_MAX`,')
w('`PNG_USER_WIDTH_MAX` (1000000), `PNG_USER_HEIGHT_MAX` (1000000),')
w('`PNG_USER_CHUNK_CACHE_MAX` (1000), `PNG_USER_CHUNK_MALLOC_MAX` (8000000),')
w('`PNG_MAX_PALETTE_LENGTH` (256), `PNG_MAX_GAMMA_8` (11), `PNG_GAMMA_THRESHOLD`,')
w('`PNG_ZBUF_SIZE` (8192), `ZLIB_IO_MAX` (0xffffffff), `PNG_INFLATE_BUF_SIZE`')
w('(1024), `PNG_ROW_MAX`, `PNG_UNEXPECTED_ZLIB_RETURN` (-3), `PNG_FP_MAX` /')
w('`PNG_FP_MIN`, `DBL_DIG` (15), `PNG_NUMBER_BUFFER_SIZE` (24),')
w('`PNG_WARNING_PARAMETER_COUNT` (8), `PNG_WARNING_PARAMETER_SIZE` (32),')
w('`PNG_TEXT_COMPRESSION_LAST`, `PNG_INTERLACE_LAST`, `PNG_EQUATION_LAST`,')
w('`PNG_SCALE_LAST`, `PNG_sRGB_INTENT_LAST`, `PNG_HANDLE_CHUNK_LAST`,')
w('`PNG_FILTER_VALUE_LAST`, `PNG_OPTION_NEXT` (16).')
w('')
w('## How a row is checked off')
w('')
w('`tests/errors.rs` (and every other test file) drives BOTH shared libraries')
w('through the same input and compares the complete ordered trace of')
w('`(warning|error, message)` events plus the return values and output bytes.')
w('Only *after* a comparison has succeeded is the diagnostic recorded, so a')
w('recorded message is by construction one that both libraries produced')
w('identically.  `tools/error_coverage.py` then diffs the union of everything the')
w('whole suite recorded (`target/observed/*.txt`) against part 1 and writes')
w('`ERROR_COVERAGE.md`; `--stamp` copies the result into the `seen` column below.')
w('So the check-marks are machine-derived, never hand-asserted.')
w('')
w('The `seen` column has three states: `[x]` the suite observed this exact')
w('message from both libraries; `[-]` the message is assembled at run time (a')
w('variable, or `png_formatted_warning` parameters) so there is no literal to')
w('match on -- the site is still exercised, it just cannot be checked off by')
w('text; `[ ]` not reached, see the list at the end of `ERROR_COVERAGE.md` for')
w('why (all of them are internal-consistency guards, need a >2 GiB object, or')
w('are shadowed by an identical earlier test in the same function).')
w('')
w('Parts 2 and 3 are silent rejections: they have no message to observe, and they')
w('are covered exhaustively by `tests/sweep.rs::null_arguments`, which calls all')
w('381 exported entry points with NULL/0 in every position and compares the return')
w('value, and by `tests/sweep.rs::hostile_scalars_*` / `enum_boundaries` /')
w('`length_boundaries`, which do the same with a live `png_ptr` and out-of-range')
w('scalars.')
w('')
w('## Part 1 — diagnostic rejections')
w('')
w('| # | function (file:line) | trigger (the exact invalid input/condition) | expected C result | seen |')
w('|---|----------------------|---------------------------------------------|-------------------|------|')
n = 0
for f, ln, kind, msg in sites:
    src = 'c_src/src/' + f
    lines = raw.get(src, [])
    fn = func_of(lines)(ln - 1) if lines else '?'
    g = guard(lines, ln - 1) if lines else ''
    trig = g
    if msg:
        trig += '  ->  message: "%s"' % msg.replace('\n', '\\n')
    n += 1
    w('| D-%d | `%s` (%s:%d) | `%s` | %s | [ ] |'
      % (n, fn, f, ln, trig.replace('|', '\\|').replace('`', "'"),
         RESULT[kind].replace('|', '\\|')))
w('')
w('## Part 2 — silent sentinel-return rejections')
w('')
w('| # | function (file:line) | trigger | expected C result |')
w('|---|----------------------|---------|-------------------|')
for i, (f, ln, fn, t, g) in enumerate(part2):
    w('| R-%d | `%s` (%s:%d) | `%s` | `%s` |'
      % (i + 1, fn, f, ln, g.replace('|', '\\|').replace('`', "'"), t))
w('')
w('## Part 3 — silent void-return rejections')
w('')
w('| # | function (file:line) | trigger | expected C result |')
w('|---|----------------------|---------|-------------------|')
for i, (f, ln, fn, t, g) in enumerate(part3):
    w('| V-%d | `%s` (%s:%d) | `%s` | returns without doing anything |'
      % (i + 1, fn, f, ln, g.replace('|', '\\|').replace('`', "'")))
w('')
w('## Part 4 — generic FFI-boundary rejections')
w('')
w('These are not tied to one source line; they are the boundaries every C API has.')
w('All six are covered exhaustively (over **all 381 exported entry points**) by')
w('`tests/sweep.rs`, which runs each call in a forked child so that a call which')
w('is fatal to the C library is a *compared observation* rather than the end of the')
w('test run.')
w('')
w('| # | trigger | expected C result | test |')
w('|---|---------|-------------------|------|')
w('| G-1 | `png_error`/`png_longjmp` with neither a `jmp_buf` nor a `longjmp_fn` installed | `PNG_ABORT()` -> `abort()` -> SIGABRT | `errors::png_abort_row_A1` |')
w('| G-2 | NULL in every pointer argument of every entry point | mostly an early `return`; a NULL deref where the C dereferences before its own guard (e.g. `png_write_chunk_start`) | `sweep::null_arguments` |')
w('| G-3 | out-of-range enum value (no valid variant) in every scalar argument | per-function: clamp, `png_app_error`, `png_warning`, or silently ignored | `sweep::hostile_scalars_read` / `_write`, `sweep::enum_boundaries` |')
w('| G-4 | one step past each documented enum range (`PNG_*_LAST`, `PNG_OPTION_NEXT`, …) | as G-3 | `sweep::enum_boundaries` |')
w('| G-5 | zero length / size where a positive one is expected | early return or `png_error` | `sweep::length_boundaries`, `errors::setter_validation` |')
w('| G-6 | oversized length (`0x7fffffff`, `0x80000000`, `0xffffffff`, `SIZE_MAX`) | allocation failure -> `png_error`/`png_warning`, or a range check | `sweep::length_boundaries`, `errors::user_limits_rejections` |')
open('ERRORS.md', 'w').write('\n'.join(out) + '\n')
print('ERRORS.md: %d D-rows, %d R-rows, %d V-rows, 6 G-rows'
      % (len(sites), len(part2), len(part3)))
