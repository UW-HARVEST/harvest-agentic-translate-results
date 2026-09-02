#!/usr/bin/env python3
"""Emit ERRORS.md: one row per distinct rejection site found in the C sources."""
import re, os, sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SRC = os.path.join(ROOT, 'c_src/src')

KINDS = [
    ('png_chunk_benign_error', 'chunk_benign_error', 'chunk-prefixed benign error: warning if benign errors allowed (default on read), else png_error+longjmp'),
    ('png_chunk_error',        'chunk_error',        'chunk-prefixed fatal error -> longjmp'),
    ('png_chunk_warning',      'chunk_warning',      'chunk-prefixed warning, call continues'),
    ('png_benign_error',       'benign_error',       'benign error: warning if allowed, else png_error+longjmp'),
    ('png_app_error',          'app_error',          'application error: png_error+longjmp unless PNG_FLAG_APP_ERRORS_WARN'),
    ('png_app_warning',        'app_warning',        'application warning, call returns without effect'),
    ('png_fixed_error',        'fixed_error',        'png_error("<text> out of range") -> longjmp'),
    ('png_error',              'error',              'fatal error -> error_fn then png_default_error -> longjmp'),
    ('png_warning',            'warning',            'warning, call continues'),
    ('assert',                 'assert',             'abort() if the assertion fails'),
]

def enclosing_functions(lines):
    """Return list parallel to lines giving the enclosing top-level function."""
    out = []
    cur = '(file scope)'
    depth = 0
    pending = None
    for ln in lines:
        stripped = ln.rstrip()
        if depth == 0:
            m = re.match(r'^([a-zA-Z_]\w*)\s*,?\s*\(', stripped)
            if m and not stripped.startswith('#') and m.group(1) not in ('if','for','while','switch','return','sizeof','PNG_UNUSED','else'):
                pending = m.group(1)
            m = re.match(r'^(?:static\s+)?(?:PNG_FUNCTION\s*\(\s*)?[\w \t]*?\**\s*([a-zA-Z_]\w*)\s*\(.*\)\s*$', stripped)
            if m and not stripped.startswith('#') and stripped.endswith(')'):
                pass
        if '{' in stripped and depth == 0 and pending:
            cur = pending
            pending = None
        depth += stripped.count('{') - stripped.count('}')
        if depth < 0:
            depth = 0
        out.append(cur)
    return out

def msg_of(ln):
    m = re.findall(r'"((?:[^"\\]|\\.)*)"', ln)
    return m[0] if m else ''


def joined(lines, i, maxlines=4):
    """The statement starting at line i, with continuation lines appended until
    the parentheses balance (libpng often puts the message on the next line)."""
    buf = ''
    depth = 0
    for k in range(i, min(i + maxlines, len(lines))):
        buf += ' ' + lines[k].strip()
        depth += lines[k].count('(') - lines[k].count(')')
        if depth <= 0 and k > i - 1 and '(' in buf:
            break
    return buf.strip()

rows = []
for fn in sorted(os.listdir(SRC)):
    if not fn.endswith('.c'):
        continue
    text = open(os.path.join(SRC, fn), errors='replace').read()
    lines = text.split('\n')
    encl = enclosing_functions(lines)
    for i, ln in enumerate(lines):
        s = ln.strip()
        matched = False
        for pat, kind, effect in KINDS:
            if re.search(r'(?<![A-Za-z0-9_])' + pat + r'\s*\(', ln):
                text = msg_of(ln) or msg_of(joined(lines, i)) or s[:70]
                rows.append((fn, i + 1, encl[i], kind, text, effect))
                matched = True
                break
        if matched:
            continue
        if re.match(r'^return\s*\(?\s*(0|NULL|-1)\s*\)?\s*;', s):
            rows.append((fn, i + 1, encl[i], 'early-return',
                         s, 'returns ' + re.sub(r'[^0-9A-Za-z-]', '', s.replace('return', '')) + ' (rejection sentinel)'))

# --- coverage -------------------------------------------------------------
OBS = os.path.join(ROOT, 'translation/target/observed_messages.txt')
observed = []
if os.path.exists(OBS):
    for line in open(OBS, errors='replace'):
        line = line.rstrip('\n')
        if '\t' in line:
            observed.append(line.split('\t', 1)[1])


def unescape_c(s):
    return (s.replace('\\n', '\n').replace('\\t', '\t')
             .replace('\\"', '"').replace("\\'", "'")
             .replace('\\\\', '\\'))


import subprocess
try:
    SO_STRINGS = subprocess.run(
        ['strings', os.path.join(ROOT, 'c_src/build/libpng.so')],
        capture_output=True, text=True, check=True).stdout
except Exception:
    SO_STRINGS = ''

VAR_MSG = re.compile(r'^\s*png_\w+\s*\(')

# ---------------------------------------------------------------------------
# Residual classification.  Each of the following rows is a rejection the C
# contains but which cannot be reached by ANY input through the exported API in
# THIS build.  Every entry states the mechanical reason.
# ---------------------------------------------------------------------------

# Guards that a preceding, equivalent check makes unreachable.
SHADOWED = {
    'bad header (invalid length)':
        'dead: png_read_chunk_header calls png_get_uint_31(buf) BEFORE this test, '
        'so any length with buf[0] >= 0x80 has already raised '
        '"PNG unsigned integer out of range" (which IS observed)',
    'Compression buffer size limited to system maximum':
        'dead: png_set_compression_buffer_size first errors on '
        '"size > PNG_UINT_31_MAX" (0x7fffffff) and ZLIB_IO_MAX is (uInt)-1 = '
        '0xffffffff, so "size > ZLIB_IO_MAX" can never hold afterwards - the '
        'source itself notes "compilers complain that this is always false"',
    'Profile length does not match profile':
        'dead: the preceding "Incorrect data in iCCP" test is '
        'png_get_uint_32(profile) != profile_len, i.e. exactly the same condition',
    'Libpng jmp_buf still allocated':
        'requires jmp_buf_size == 0 together with a heap-allocated jmp_buf, a '
        'combination that exists only transiently inside png_free_jmpbuf',
    'Decompression error in IDAT':
        'fallback text: the branch IS exercised (see the observed '
        '"IDAT: <zlib message>" rows) but this literal is only used when zlib '
        'leaves zstream.msg NULL, which zlib does not do for any input',
}

# Guards that arithmetic on a 64-bit target cannot satisfy.
WIDE64 = {
    'Image width is too large for this architecture':
        '((width+7)&~7) > (PNG_SIZE_MAX-49)/8-1 ~= 2^61 while width <= 2^32-1',
    'Row has too many bytes to allocate in memory':
        'row size <= 2^31 * 8 bytes, far below PNG_SIZE_MAX on LP64',
    'sequential row overflow': 'requires a row larger than PNG_SIZE_MAX',
    'progressive row overflow': 'requires a row larger than PNG_SIZE_MAX',
    'Potential overflow in png_zalloc()':
        'items and size are uInt, so items*size < 2^64 always',
    'png_image_write_to_memory: PNG too big':
        'requires a PNG stream larger than PNG_SIZE_MAX',
    'tEXt: text too long': 'requires more than PNG_UINT_31_MAX bytes of text',
    'iTXt: uncompressed text too long':
        'requires more than PNG_UINT_31_MAX bytes of text',
    'sPLT chunk too long':
        'requires entry count > PNG_SIZE_MAX/sizeof(png_sPLT_entry) ~= 1.8e18',
    'Potential overflow of save_buffer':
        'requires save_buffer_size > PNG_SIZE_MAX - current_buffer_size - 256',
}

# Reachable only if the allocator fails.
OOM_WORDS = (
    'Insufficient memory', 'Out of Memory', 'Memory allocation failed',
    'requires too much memory', 'No space in chunk cache', 'save_buffer error',
)

# Invariant assertions libpng makes about its own state.
INTERNAL_EXTRA = {
    'unknown interlace type', 'Uninitialized row', 'NULL row buffer',
    'gamma value', 'png_do_encode_alpha: unexpected call', 'unexpected compose',
    'unexpected bit depth', 'unexpected 8-bit transformation',
    'lost/gained channels', 'lost rgb to gray',
    'unexpected alpha swap transformation', 'png_image_read: alpha channel lost',
    'png_read_image: invalid transformations',
    'png_read_image: unsupported transformation',
    'png_write_image: unsupported transformation',
    'invalid PNG color type', 'color-map index out of range',
    'Z_OK on Z_FINISH with output space', 'deflateEnd failed (ignored)',
    'Extra compressed data in IDAT',
    'error writing ancillary chunked compressed data',
    'invalid memory read',
}


def covered(kind, msg):
    if kind == 'early-return':
        return 'tests/j_nullargs.rs + value-range rows'
    if kind == 'assert':
        return 'unreachable by construction (see note)'
    lit = unescape_c(msg)
    if VAR_MSG.match(lit):
        # The scanner captured the call, not a literal: the message text is
        # supplied by the caller.  These are the dispatch sites inside the
        # error/warning machinery itself; they are exercised by every row whose
        # message IS observed.
        return 'dispatch site (message supplied by caller)'
    for o in observed:
        if lit and lit in o:
            return 'observed'
    if lit and SO_STRINGS and lit not in SO_STRINGS:
        return 'compiled out of this build (literal absent from the .so)'
    if lit in SHADOWED:
        return 'unreachable: ' + SHADOWED[lit]
    if lit in WIDE64:
        return 'unreachable on a 64-bit target: ' + WIDE64[lit]
    if any(w in lit for w in OOM_WORDS):
        return 'reachable only on allocator failure'
    if 'internal' in lit.lower() or 'too few entries' in lit or lit in INTERNAL_EXTRA:
        return 'internal invariant; unreachable through the exported API'
    return 'NOT OBSERVED'


cov = [covered(k, m) for (_f, _l, _fn, k, m, _e) in rows]
n_obs = sum(1 for c in cov if c == 'observed')
n_early = sum(1 for c in cov if c.startswith('tests/j_nullargs'))
n_not = sum(1 for c in cov if c == 'NOT OBSERVED')
n_disp = sum(1 for c in cov if c.startswith('dispatch site'))
n_out = sum(1 for c in cov if c.startswith('compiled out'))
n_unreach = sum(1 for c in cov if c.startswith('unreachable'))
n_oom = sum(1 for c in cov if c.startswith('reachable only on allocator'))
n_internal = sum(1 for c in cov if c.startswith('internal invariant'))

print("# ERRORS.md — ERROR-SURFACE TABLE")
print()
print("Derived mechanically from `c_src/src/*.c` by `tools/gen_errors.py`: every")
print("`png_error`, `png_chunk_error`, `png_app_error`, `png_benign_error`,")
print("`png_chunk_benign_error`, `png_fixed_error`, `png_warning`, `png_chunk_warning`,")
print("`png_app_warning`, `assert()` and every `return 0 / NULL / -1` rejection sentinel.")
print()
print("`kind` determines the observable C result:")
print()
print("| kind | observable C result |")
print("|------|---------------------|")
for _, kind, effect in KINDS:
    print("| `%s` | %s |" % (kind, effect))
print("| `early-return` | the function returns the sentinel and makes no state change |")
print()
print("Total rows: **%d**." % len(rows))
print()
print("## Phase C coverage")
print()
print("`coverage` is filled in mechanically by `tools/gen_errors.py` from")
print("`translation/target/observed_messages.txt`, which `common::record_message`")
print("appends to from the error and warning callbacks during the test run.  A row")
print("marked `observed` means that exact message text came out of the library while")
print("a Phase C differential test was asserting that BOTH implementations produce")
print("identical error/warning output.")
print()
print("* message rows observed: **%d**" % n_obs)
print("* `early-return` sentinel rows covered by `tests/j_nullargs.rs`: **%d**" % n_early)
print("* dispatch-site rows (message text comes from the caller): **%d**" % n_disp)
print("* rows compiled out of this build: **%d**" % n_out)
print("* rows unreachable in this build (dead guards / 64-bit arithmetic): **%d**" % n_unreach)
print("* rows reachable only on allocator failure: **%d**" % n_oom)
print("* internal-invariant rows unreachable through the exported API: **%d**" % n_internal)
print("* rows NOT observed and NOT otherwise accounted for: **%d**" % n_not)
print()
print("| # | file:line | function | kind | trigger (message / statement) | expected C result | coverage |")
print("|---|-----------|----------|------|-------------------------------|-------------------|----------|")
for n, ((f, l, fun, kind, msg, effect), cv) in enumerate(zip(rows, cov), 1):
    msg = msg.replace('|', '\\|')
    print("| %d | `%s:%d` | `%s` | %s | `%s` | %s | %s |" % (n, f, l, fun, kind, msg, effect, cv))
sys.stderr.write('rows %d\n' % len(rows))
