#!/usr/bin/env python3
"""Generate Rust constant definitions from the PCRE2 C headers."""
import re, sys, os

SRC = os.path.join(os.path.dirname(__file__), '..', 'c_src')
OUT = os.path.join(os.path.dirname(__file__), '..', 'translation', 'src')

def read(p):
    with open(p) as f:
        return f.read()

pcre2_h = read(os.path.join(SRC, 'include', 'pcre2.h'))
internal_h = read(os.path.join(SRC, 'src', 'pcre2_internal.h'))
compile_h = read(os.path.join(SRC, 'src', 'pcre2_compile.h'))
ucp_h = read(os.path.join(SRC, 'src', 'pcre2_ucp.h'))

lines = []
lines.append('// Auto-generated from the PCRE2 C headers by _gen/gen_consts.py. Do not edit.')
lines.append('#![allow(dead_code, non_upper_case_globals)]')
lines.append('')

# ---------------- pcre2.h #defines ----------------
lines.append('// ---- from pcre2.h ----')
skip = {'PCRE2_SIZE', 'PCRE2_SIZE_MAX', 'PCRE2_ZERO_TERMINATED', 'PCRE2_UNSET',
        'PCRE2_EXP_DECL', 'PCRE2_CALL_CONVENTION', 'PCRE2_PRERELEASE', 'PCRE2_DATE',
        'PCRE2_JOIN', 'PCRE2_GLUE', 'PCRE2_SUFFIX', 'PCRE2_UCHAR', 'PCRE2_SPTR',
        'PCRE2_H_IDEMPOTENT_GUARD'}
seen = set()
for m in re.finditer(r'^#define\s+(PCRE2_[A-Za-z0-9_]+)\s+(\S+)\s*(?:/\*.*)?$', pcre2_h, re.M):
    name, val = m.group(1), m.group(2)
    if name in skip or name in seen:
        continue
    if name.endswith('_LIST') or name.endswith('FUNCTIONS') or name.endswith('FUNCTION'):
        continue
    v = val.strip()
    neg = False
    mm = re.fullmatch(r'\((-\d+)\)', v)
    if mm:
        v = mm.group(1); neg = True
    if not re.fullmatch(r'-?(0x[0-9a-fA-F]+|\d+)u?', v):
        continue
    seen.add(name)
    rusty = v.rstrip('u')
    if name.startswith('PCRE2_ERROR_') or neg:
        lines.append('pub const %s: i32 = %s;' % (name, rusty))
    else:
        lines.append('pub const %s: u32 = %s;' % (name, rusty))
lines.append('')

# ---------------- internal.h simple defines ----------------
lines.append('// ---- from pcre2_internal.h ----')
int_defs_u32 = ['NOTACHAR', 'MAX_UTF_CODE_POINT', 'PCRE2_MODE8', 'PCRE2_MODE16', 'PCRE2_MODE32',
        'PCRE2_FIRSTSET', 'PCRE2_FIRSTCASELESS', 'PCRE2_FIRSTMAPSET', 'PCRE2_LASTSET',
        'PCRE2_LASTCASELESS', 'PCRE2_STARTLINE', 'PCRE2_JCHANGED', 'PCRE2_HASCRORLF',
        'PCRE2_HASTHEN', 'PCRE2_MATCH_EMPTY', 'PCRE2_BSR_SET', 'PCRE2_NL_SET',
        'PCRE2_NOTEMPTY_SET', 'PCRE2_NE_ATST_SET', 'PCRE2_DEREF_TABLES', 'PCRE2_NOJIT',
        'PCRE2_HASBKPORX', 'PCRE2_DUPCAPUSED', 'PCRE2_HASBKC', 'PCRE2_HASACCEPT',
        'PCRE2_HASBSK', 'PCRE2_OPTIM_AUTO_POSSESS', 'PCRE2_OPTIM_DOTSTAR_ANCHOR',
        'PCRE2_OPTIM_START_OPTIMIZE', 'PCRE2_OPTIMIZATION_ALL']
for m in re.finditer(r'^#define\s+([A-Za-z_][A-Za-z0-9_]*)\s+(0x[0-9a-fA-F]+u?|\d+)\s*(?:/\*.*)?$', internal_h, re.M):
    name, v = m.group(1), m.group(2).rstrip('u')
    if name in int_defs_u32:
        lines.append('pub const %s: u32 = %s;' % (name, v))
lines.append('pub const PCRE2_MODE_MASK: u32 = PCRE2_MODE8 | PCRE2_MODE16 | PCRE2_MODE32;')
lines.append('pub const PCRE2_MD_COPIED_SUBJECT: u8 = 0x01;')
lines.append('pub const MAGIC_NUMBER: u32 = 0x50435245;')
lines.append('pub const COMPILE_ERROR_BASE: i32 = 100;')
lines.append('pub const START_FRAMES_SIZE: usize = 20480;')
lines.append('pub const DFA_START_RWS_SIZE: usize = 30720;')
lines.append('pub const BSR_DEFAULT: u32 = PCRE2_BSR_UNICODE;')
lines.append('pub const REQ_CU_MAX: usize = 5000;')
lines.append('pub const ECLASS_NEST_LIMIT: usize = 15;')
lines.append('pub const NLTYPE_FIXED: u32 = 0;')
lines.append('pub const NLTYPE_ANY: u32 = 1;')
lines.append('pub const NLTYPE_ANYCRLF: u32 = 2;')
lines.append('pub const PCRE2_MATCHEDBY_INTERPRETER: u8 = 0;')
lines.append('pub const PCRE2_MATCHEDBY_DFA_INTERPRETER: u8 = 1;')
lines.append('pub const PCRE2_MATCHEDBY_JIT: u8 = 2;')
# cbits offsets
for name, v in [('cbit_space', 0), ('cbit_xdigit', 32), ('cbit_digit', 64), ('cbit_upper', 96),
                ('cbit_lower', 128), ('cbit_word', 160), ('cbit_graph', 192), ('cbit_print', 224),
                ('cbit_punct', 256), ('cbit_cntrl', 288), ('cbit_length', 320)]:
    lines.append('pub const %s: usize = %d;' % (name, v))
for name, v in [('ctype_space', 0x01), ('ctype_letter', 0x02), ('ctype_lcletter', 0x04),
                ('ctype_digit', 0x08), ('ctype_word', 0x10)]:
    lines.append('pub const %s: u8 = 0x%02x;' % (name, v))
for name, v in [('lcc_offset', 0), ('fcc_offset', 256), ('cbits_offset', 512)]:
    lines.append('pub const %s: usize = %d;' % (name, v))
lines.append('pub const ctypes_offset: usize = cbits_offset + cbit_length;')
lines.append('pub const TABLES_LENGTH: usize = ctypes_offset + 256;')
# config.h values
lines.append('// ---- from config.h ----')
for name, v, t in [('HEAP_LIMIT', 20000000, 'u32'), ('LINK_SIZE', 2, 'usize'),
                   ('MATCH_LIMIT', 10000000, 'u32'), ('MATCH_LIMIT_DEPTH', 10000000, 'u32'),
                   ('MAX_NAME_COUNT', 10000, 'u32'), ('MAX_NAME_SIZE', 128, 'u32'),
                   ('MAX_VARLOOKBEHIND', 255, 'u32'), ('NEWLINE_DEFAULT', 2, 'u32'),
                   ('PARENS_NEST_LIMIT', 250, 'u32')]:
    lines.append('pub const %s: %s = %d;' % (name, t, v))
lines.append('pub const IMM2_SIZE: usize = 2;')
lines.append('pub const MAX_PATTERN_SIZE: usize = 1 << 16;')
lines.append('pub const MAX_MARK: u32 = (1u32 << 8) - 1;')
lines.append('pub const MAX_UTF_SINGLE_CU: u32 = 127;')
lines.append('pub const MAX_NON_UTF_CHAR: u32 = 0xff;')
lines.append('pub const MAX_UCHAR_VALUE: u32 = 0xff;')
lines.append('pub const LOOKBEHIND_MAX: i32 = 65535;')
lines.append('pub const UCD_BLOCK_SIZE: usize = 128;')
lines.append('pub const UCD_SCRIPTX_MASK: u32 = 0x3ff;')
lines.append('pub const UCD_BIDICLASS_SHIFT: u32 = 11;')
lines.append('pub const UCD_BPROPS_MASK: u32 = 0xfff;')
lines.append('pub const RREF_ANY: u32 = 0xffff;')
lines.append('pub const REFI_FLAG_CASELESS_RESTRICT: u32 = 0x1;')
lines.append('pub const REFI_FLAG_TURKISH_CASING: u32 = 0x2;')
lines.append('pub const PCRE2_UNSET: usize = usize::MAX;')
lines.append('pub const PCRE2_ZERO_TERMINATED: usize = usize::MAX;')
lines.append('pub const PCRE2_SIZE_MAX: usize = usize::MAX;')
lines.append('')

# PT_ / XCL_ / ECL_ defines from internal.h
lines.append('// ---- PT_, XCL_, ECL_ ----')
for pat, ty in [(r'^#define\s+(PT_[A-Z0-9_]+)\s+(\d+)', 'u32'),
                (r'^#define\s+(XCL_[A-Z0-9_]+)\s+(0x[0-9a-fA-F]+|\d+)', 'u32'),
                (r'^#define\s+(ECL_[A-Z0-9_]+)\s+(0x[0-9a-fA-F]+|\d+)', 'u32')]:
    for m in re.finditer(pat, internal_h, re.M):
        lines.append('pub const %s: %s = %s;' % (m.group(1), ty, m.group(2)))
lines.append('pub const PT_TABSIZE: u32 = PT_ANY;')
lines.append('pub const XCL_LIST: u32 = 0x10;')
lines.append('')

# ---------------- ESC_ enum from internal.h ----------------
m = re.search(r'enum \{ ESC_A = 1,(.*?)\};', internal_h, re.S)
names = ['ESC_A'] + [x.strip() for x in re.split(r',', m.group(1)) if x.strip()]
lines.append('// ---- ESC_ codes ----')
for i, n in enumerate(names):
    lines.append('pub const %s: u32 = %d;' % (n.replace('\n', ''), i + 1))
lines.append('')

# ---------------- opcodes ----------------
m = re.search(r'\nenum \{\n  OP_END,(.*?)\n\};', internal_h, re.S)
body = m.group(1)
body = re.sub(r'/\*.*?\*/', '', body, flags=re.S)
ops = ['OP_END'] + [x.strip() for x in body.split(',') if x.strip()]
lines.append('// ---- opcodes ----')
for i, n in enumerate(ops):
    lines.append('pub const %s: u32 = %d;' % (n, i))
lines.append('pub const FIRST_AUTOTAB_OP: u32 = OP_NOT_DIGIT;')
lines.append('pub const LAST_AUTOTAB_LEFT_OP: u32 = OP_EXTUNI;')
lines.append('pub const LAST_AUTOTAB_RIGHT_OP: u32 = OP_DOLLM;')
lines.append('')

# ---------------- META codes ----------------
lines.append('// ---- META codes (pcre2_compile.h) ----')
for m in re.finditer(r'^#define\s+(META_[A-Z0-9_]+)\s+(0x[0-9a-fA-F]+)u', compile_h, re.M):
    lines.append('pub const %s: u32 = %s;' % (m.group(1), m.group(2)))
lines.append('pub const META_FIRST_QUANTIFIER: u32 = META_ASTERISK;')
lines.append('pub const META_LAST_QUANTIFIER: u32 = META_MINMAX_QUERY;')
lines.append('pub const SIZEOFFSET: usize = 2;')
lines.append('pub const CLASS_IS_ECLASS: u32 = 0x1;')
lines.append('pub const PC_DIGIT: usize = 7;')
lines.append('pub const PC_GRAPH: usize = 8;')
lines.append('pub const PC_PRINT: usize = 9;')
lines.append('pub const PC_PUNCT: usize = 10;')
lines.append('pub const PC_XDIGIT: usize = 13;')
lines.append('pub const NAMED_GROUP_HASH_MASK: u16 = 0x7fff;')
lines.append('pub const NAMED_GROUP_IS_DUPNAME: u16 = 0x8000;')
lines.append('')
lines.append('// ---- compile error numbers ----')
for i in range(0, 121):
    lines.append('pub const ERR%d: i32 = %d;' % (i, 100 + i))
lines.append('')

# ---------------- ucp values ----------------
lines.append('// ---- Unicode property values (pcre2_ucp.h) ----')
for m in re.finditer(r'enum \{(.*?)\};', ucp_h, re.S):
    body = re.sub(r'/\*.*?\*/', '', m.group(1), flags=re.S)
    items = [x.strip() for x in body.split(',') if x.strip()]
    for i, n in enumerate(items):
        assert re.fullmatch(r'[A-Za-z_][A-Za-z0-9_]*', n), n
        lines.append('pub const %s: u32 = %d;' % (n, i))
lines.append('pub const ucd_boolprop_sets_item_size: usize = 2;')
lines.append('pub const ucd_script_sets_item_size: usize = 4;')
lines.append('')
lines.append('pub const TRUE: crate::types::BOOL = 1;')
lines.append('pub const FALSE: crate::types::BOOL = 0;')

with open(os.path.join(OUT, 'consts.rs'), 'w') as f:
    f.write('\n'.join(lines) + '\n')
print('wrote consts.rs with %d lines' % len(lines))
