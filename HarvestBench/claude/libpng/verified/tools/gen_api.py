#!/usr/bin/env python3
"""Mechanically derive the complete public API of libpng from c_src/include/png.h
and emit `tests/common/api.rs`: a Rust struct with one function-pointer field per
PNG_EXPORT declaration, loaded from a dlopen'd shared library.

Both the C reference .so and the Rust .so are loaded through this same table, so
every differential test necessarily goes through the exported symbols.
"""
import re, sys, os

HDR = 'c_src/include/png.h'
PRIV = 'c_src/include/pngpriv.h'
EXPORTS = 'tools/c_exports.txt'
OUT = 'tests/common/api.rs'

def load(path):
    t = open(path, encoding='utf-8').read()
    return re.sub(r'/\*.*?\*/', ' ', t, flags=re.S)

src = load(HDR)
priv = load(PRIV)
exported = set(x.strip() for x in open(EXPORTS) if x.strip())

HEAD = re.compile(
    r'\bPNG_(EXPORTA|EXPORT|FIXED_EXPORT|FP_EXPORT)\s*\(\s*(\d+)\s*,\s*'
    r'([A-Za-z_][A-Za-z0-9_ \*]*)\s*,\s*(png_[A-Za-z0-9_]+)\s*,\s*')

def balanced(s, i):
    """s[i] == '(' -- return (inner_text, index_after_close)."""
    assert s[i] == '(', s[i:i+40]
    depth = 0
    j = i
    while j < len(s):
        if s[j] == '(':
            depth += 1
        elif s[j] == ')':
            depth -= 1
            if depth == 0:
                return s[i + 1:j], j + 1
        j += 1
    raise SystemExit('unbalanced parens at %d' % i)

IHEAD = re.compile(
    r'\bPNG_INTERNAL_(?:FUNCTION|CALLBACK)\s*\(\s*([A-Za-z_][A-Za-z0-9_ \*]*?)\s*,\s*'
    r'(png_[A-Za-z0-9_]+)\s*,\s*')

def internal_declarations(src):
    for m in IHEAD.finditer(src):
        ret, name = m.groups()
        i = m.end()
        while i < len(src) and src[i].isspace():
            i += 1
        if src[i] != '(':
            continue
        args, _ = balanced(src, i)
        yield ret, name, args

def declarations(src):
    for m in HEAD.finditer(src):
        kind, ordinal, ret, name = m.groups()
        i = m.end()
        while i < len(src) and src[i].isspace():
            i += 1
        args, _ = balanced(src, i)
        yield kind, ordinal, ret, name, args

TYPE = {
    'void': '()',
    'int': 'c_int',
    'unsigned int': 'c_uint',
    'float': 'f32',
    'double': 'f64',
    'size_t': 'usize',
    'time_t': 'i64',
    'png_byte': 'u8',
    'png_uint_16': 'u16',
    'png_int_16': 'i16',
    'png_uint_32': 'u32',
    'png_int_32': 'i32',
    'png_fixed_point': 'i32',
    'png_alloc_size_t': 'usize',
    'png_size_t': 'usize',
    'jmp_buf*': '*mut JmpBuf',
    'png_voidp': '*mut c_void',
    'png_const_voidp': '*const c_void',
    'png_structp': '*mut PngStruct',
    'png_structrp': '*mut PngStruct',
    'png_const_structp': '*mut PngStruct',
    'png_const_structrp': '*mut PngStruct',
    'png_infop': '*mut PngInfo',
    'png_infopp': '*mut *mut PngInfo',
    'png_structpp': '*mut *mut PngStruct',
    'png_inforp': '*mut PngInfo',
    'png_const_infop': '*mut PngInfo',
    'png_const_inforp': '*mut PngInfo',
    'png_imagep': '*mut png_image',
    'png_controlp': '*mut c_void',
    'png_bytep': '*mut u8',
    'png_const_bytep': '*const u8',
    'png_bytepp': '*mut *mut u8',
    'png_const_bytepp': '*const *mut u8',
    'png_charp': '*mut c_char',
    'png_const_charp': '*const c_char',
    'png_charpp': '*mut *mut c_char',
    'png_const_charpp': '*mut *const c_char',
    'png_uint_32p': '*mut u32',
    'png_const_uint_32p': '*const u32',
    'png_int_32p': '*mut i32',
    'png_uint_16p': '*mut u16',
    'png_const_uint_16p': '*const u16',
    'png_fixed_point_p': '*mut i32',
    'png_const_fixed_point_p': '*const i32',
    'png_doublep': '*mut f64',
    'png_const_doublep': '*const f64',
    'png_size_tp': '*mut usize',
    'png_colorp': '*mut png_color',
    'png_const_colorp': '*const png_color',
    'png_colorpp': '*mut *mut png_color',
    'png_color_8p': '*mut png_color_8',
    'png_const_color_8p': '*const png_color_8',
    'png_color_8pp': '*mut *mut png_color_8',
    'png_color_16p': '*mut png_color_16',
    'png_const_color_16p': '*const png_color_16',
    'png_color_16pp': '*mut *mut png_color_16',
    'png_sPLT_tp': '*mut png_sPLT_t',
    'png_const_sPLT_tp': '*const png_sPLT_t',
    'png_sPLT_tpp': '*mut *mut png_sPLT_t',
    'png_textp': '*mut png_text',
    'png_const_textp': '*const png_text',
    'png_textpp': '*mut *mut png_text',
    'png_timep': '*mut png_time',
    'png_const_timep': '*const png_time',
    'png_timepp': '*mut *mut png_time',
    'png_unknown_chunkp': '*mut png_unknown_chunk',
    'png_const_unknown_chunkp': '*const png_unknown_chunk',
    'png_unknown_chunkpp': '*mut *mut png_unknown_chunk',
    'png_row_infop': '*mut png_row_info',
    'png_error_ptr': 'png_error_ptr',
    'png_rw_ptr': 'png_rw_ptr',
    'png_flush_ptr': 'png_flush_ptr',
    'png_read_status_ptr': 'png_read_status_ptr',
    'png_write_status_ptr': 'png_write_status_ptr',
    'png_progressive_info_ptr': 'png_progressive_info_ptr',
    'png_progressive_end_ptr': 'png_progressive_end_ptr',
    'png_progressive_row_ptr': 'png_progressive_row_ptr',
    'png_user_transform_ptr': 'png_user_transform_ptr',
    'png_user_chunk_ptr': 'png_user_chunk_ptr',
    'png_longjmp_ptr': 'png_longjmp_ptr',
    'png_malloc_ptr': 'png_malloc_ptr',
    'png_free_ptr': 'png_free_ptr',
    'png_FILE_p': '*mut c_void',
    'FILE*': '*mut c_void',
    'voidpf': '*mut c_void',
    'png_warning_parameters': '*mut c_char',

    'png_handle_result_code': 'c_uint',

    'uInt': 'c_uint',
    'uLong': 'c_ulong',
    'unsigned long': 'c_ulong',
    'z_streamp': '*mut c_void',
    'png_const_structrp png_ptr': '*mut PngStruct',

    'png_compression_bufferp': '*mut c_void',
    'png_compression_bufferpp': '*mut *mut c_void',
    'png_xy': 'png_xy',
    'png_XYZ': 'png_XYZ',
    'png_const_xyp': '*const png_xy',
    'png_xy*': '*mut png_xy',
    'png_XYZ*': '*mut png_XYZ',
    'png_const_XYZp': '*const png_XYZ',
    'png_colorspacerp': '*mut c_void',
    'png_const_colorspacerp': '*const c_void',
    'png_colorspacep': '*mut c_void',
    'png_read_filter_fn': '*mut c_void',
    'png_alloc_size_tp': '*mut usize',

    'png_color_16p*': '*mut *mut png_color_16',
    'png_color_8p*': '*mut *mut png_color_8',
    'png_colorp*': '*mut *mut png_color',
    'png_textp*': '*mut *mut png_text',
    'png_timep*': '*mut *mut png_time',
    'png_sPLT_tp*': '*mut *mut png_sPLT_t',
    'png_unknown_chunkp*': '*mut *mut png_unknown_chunk',
    'png_bytep*': '*mut *mut u8',
    'png_charp*': '*mut *mut c_char',
    'png_const_charp*': '*mut *const c_char',
    'png_uint_32*': '*mut u32',
    'png_int_32*': '*mut i32',
    'png_uint_16*': '*mut u16',
    'png_byte*': '*mut u8',
    'png_fixed_point*': '*mut i32',
    'double*': '*mut f64',
    'int*': '*mut c_int',
    'png_infop*': '*mut *mut PngInfo',
    'png_structp*': '*mut *mut PngStruct',
    'png_size_t*': '*mut usize',
    'size_t*': '*mut usize',
    'png_voidp*': '*mut *mut c_void',

    'const struct tm *': '*const Tm',
    'constchar*': '*const c_char',
    'char*': '*mut c_char',
    'constvoid*': '*const c_void',
    'void*': '*mut c_void',
    'conststructtm*': '*const Tm',
}

def rty(t):
    t = re.sub(r'\bPNGAPI\b', '', t)
    t = re.sub(r'\bPNG_RESTRICT\b', '', t)
    t = re.sub(r'\bPNGCBAPI\b', '', t)
    t = re.sub(r'\bPNGFAPI\b', '', t)
    t = re.sub(r'\bPNG_NORETURN\b', '', t)
    t = re.sub(r'\brestrict\b', '', t)
    t = re.sub(r'\bconst\b', 'const', t)
    t = re.sub(r'\s+', ' ', t).strip()
    if t in TYPE:
        return TYPE[t]
    t2 = t.replace(' ', '')
    if t2 in TYPE:
        return TYPE[t2]
    # generic: `X *` where X is a known type -> pointer to it
    while t2.endswith('*'):
        inner = t2[:-1]
        if inner.startswith('const'):
            inner2 = inner[5:]
            if inner2 in TYPE:
                return '*const ' + TYPE[inner2]
        if inner in TYPE:
            return '*mut ' + TYPE[inner]
        break
    raise SystemExit('UNMAPPED TYPE: %r' % t)

def split_args(a):
    a = re.sub(r'\s+', ' ', a).strip()
    if a in ('', 'void'):
        return []
    out, depth, cur = [], 0, ''
    for ch in a:
        if ch == '(':
            depth += 1
        elif ch == ')':
            depth -= 1
        if ch == ',' and depth == 0:
            out.append(cur); cur = ''
        else:
            cur += ch
    out.append(cur)
    return [x.strip() for x in out]

FNPTR = re.compile(r'^(.*?)\(\s*\*\s*[A-Za-z_][A-Za-z0-9_]*\s*\)\s*\((.*)\)$')

def arg_type(a):
    a = a.strip()
    m = FNPTR.match(a)
    if m:
        r = rty(m.group(1))
        at = [arg_type(x) for x in split_args(m.group(2))]
        return 'Option<unsafe extern "C" fn(%s)%s>' % (
            ', '.join(at), '' if r == '()' else ' -> ' + r)
    # array parameters decay to pointers: "char out[29]" -> char*
    m = re.match(r'^(.*?)([A-Za-z_][A-Za-z0-9_]*)\s*\[[^\]]*\]$', a)
    if m:
        base = m.group(1).strip()
        if base == 'char':
            return '*mut c_char'
        if base == 'png_byte':
            return '*mut u8'
        return rty(base) + '_ARRAY_UNMAPPED'
    # "png_structrp png_ptr", "int val", "png_const_charp name", "jmp_buf*" ...
    m = re.match(r'^(.*?)([A-Za-z_][A-Za-z0-9_]*)$', a)
    if m and m.group(1).strip():
        return rty(m.group(1))
    return rty(a)

seen = {}
entries = []
for kind, ordinal, ret, name, args in declarations(src):
    if name in seen or name not in exported:
        continue
    seen[name] = True
    r = rty(ret)
    at = [arg_type(x) for x in split_args(args)]
    sig = 'unsafe extern "C" fn(%s)%s' % (', '.join(at),
                                          '' if r == '()' else ' -> ' + r)
    entries.append((name, sig))

for ret, name, args in internal_declarations(priv):
    if name in seen or name not in exported:
        continue
    seen[name] = True
    r = rty(ret)
    at = [arg_type(x) for x in split_args(args)]
    sig = 'unsafe extern "C" fn(%s)%s' % (', '.join(at),
                                          '' if r == '()' else ' -> ' + r)
    entries.append((name, sig))

DATA = {
    'png_sRGB_table': '*const u16',
    'png_sRGB_base': '*const u16',
    'png_sRGB_delta': '*const u8',
}

missing = sorted(exported - set(seen))
entries.sort()
hdrtxt = '''// @generated by tools/gen_api.py from c_src/include/png.h -- do not edit.
//
// One function-pointer field per PNG_EXPORT declaration in png.h (%d of them).
// `Api::load` resolves every one of them from a dlopen'd shared object, so the
// *same* Rust code drives the C reference .so and the translated Rust .so.
#![allow(non_snake_case, non_camel_case_types, dead_code)]

use super::types::*;
use core::ffi::{c_char, c_int, c_uint, c_void};

pub struct Api {
    /// "C" or "Rust" -- used in assertion messages.
    pub which: &'static str,
    /// Keeps the library mapped for as long as the Api lives.
    pub lib: &'static libloading::Library,
''' % len(entries)

lines = [hdrtxt]
for name, sig in entries:
    lines.append('    pub %s: %s,' % (name, sig))
for name, ty in sorted(DATA.items()):
    lines.append('    pub %s: %s,' % (name, ty))
lines.append('}')
lines.append('')
lines.append('impl Api {')
lines.append('    pub unsafe fn load(lib: &\'static libloading::Library, which: &\'static str) -> Api {')
lines.append('        macro_rules! g {')
lines.append('            ($n:literal) => {')
lines.append('                *lib.get($n.as_bytes())')
lines.append('                    .unwrap_or_else(|e| panic!("{}: symbol {}: {}", which, $n, e))')
lines.append('            };')
lines.append('        }')
lines.append('        Api {')
lines.append('            which,')
lines.append('            lib,')
for name, sig in entries:
    lines.append('            %s: g!("%s"),' % (name, name))
for name, ty in sorted(DATA.items()):
    lines.append('            %s: *lib.get::<%s>(b"%s").expect("%s"),' % (name, ty, name, name))
lines.append('        }')
lines.append('    }')
lines.append('}')
lines.append('')
lines.append('/// Every exported entry point, by name (for the mechanical sweeps).')
lines.append('pub const API_NAMES: [&str; %d] = [' % len(entries))
for name, _ in entries:
    lines.append('    "%s",' % name)
lines.append('];')
open(OUT, 'w').write('\n'.join(lines) + '\n')
print('wrote %s with %d entry points + %d data symbols' % (OUT, len(entries), len(DATA)))
still = sorted(exported - set(seen) - set(DATA))
if still:
    print('NOT COVERED BY api.rs (%d):' % len(still))
    for n in still:
        print('   ', n)
