#!/usr/bin/env python3
"""Mechanically extract every PNG_EXPORT* prototype from c_src/include/png.h and
emit a Rust `api!{ ... }` block of FFI signatures for the differential harness."""
import re, sys, subprocess, os

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
src = open(os.path.join(ROOT, 'c_src/include/png.h')).read()
src = re.sub(r'/\*.*?\*/', ' ', src, flags=re.S)

MARK = re.compile(r'\bPNG_(?:EXPORTA?|FP_EXPORT|FIXED_EXPORT)\s*\(')

def balanced(s, i):
    """i points at '('; return (inner, index after ')')"""
    d = 0
    j = i
    while j < len(s):
        if s[j] == '(':
            d += 1
        elif s[j] == ')':
            d -= 1
            if d == 0:
                return s[i+1:j], j+1
        j += 1
    raise ValueError('unbalanced')

def split_top(s):
    out, d, cur = [], 0, ''
    for ch in s:
        if ch == '(':
            d += 1
        elif ch == ')':
            d -= 1
        if ch == ',' and d == 0:
            out.append(cur)
            cur = ''
        else:
            cur += ch
    out.append(cur)
    return [x.strip() for x in out]

protos = []
for m in MARK.finditer(src):
    inner, _ = balanced(src, m.end()-1)
    parts = split_top(inner)
    if len(parts) < 3:
        continue
    ordinal = parts[0].strip()
    rtype = ' '.join(parts[1].split())
    name = parts[2].strip()
    args = ' '.join(parts[3].split()) if len(parts) > 3 else 'void'
    if args.startswith('(') and args.endswith(')'):
        args = args[1:-1].strip()
    if not re.fullmatch(r'\d+', ordinal):
        continue
    if not re.fullmatch(r'png_[A-Za-z0-9_]+', name):
        continue
    protos.append((int(ordinal), rtype, name, args))

# de-dup, keep first
seen = {}
for o, r, n, a in protos:
    if n not in seen:
        seen[n] = (o, r, n, a)
protos = sorted(seen.values())

# --- C type -> Rust type -------------------------------------------------
BASE = {
    'void': '()',
    'int': 'c_int',
    'unsigned int': 'c_uint',
    'char': 'c_char',
    'unsigned char': 'u8',
    'double': 'f64',
    'float': 'f32',
    'size_t': 'usize',
    'time_t': 'i64',
    'png_uint_32': 'u32',
    'png_int_32': 'i32',
    'png_uint_16': 'u16',
    'png_int_16': 'i16',
    'png_byte': 'u8',
    'png_fixed_point': 'i32',
    'png_alloc_size_t': 'usize',
    'png_size_t': 'usize',
    'jmp_buf': '[u64; 25]',
    'png_structp': '*mut c_void',
    'png_structrp': '*mut c_void',
    'png_const_structrp': '*mut c_void',
    'png_structpp': '*mut *mut c_void',
    'png_infop': '*mut c_void',
    'png_inforp': '*mut c_void',
    'png_const_inforp': '*mut c_void',
    'png_infopp': '*mut *mut c_void',
    'png_const_infopp': '*mut *mut c_void',
    'png_voidp': '*mut c_void',
    'png_const_voidp': '*const c_void',
    'png_bytep': '*mut u8',
    'png_const_bytep': '*const u8',
    'png_bytepp': '*mut *mut u8',
    'png_const_bytepp': '*const *const u8',
    'png_charp': '*mut c_char',
    'png_const_charp': '*const c_char',
    'png_charpp': '*mut *mut c_char',
    'png_const_charpp': '*const *const c_char',
    'png_uint_32p': '*mut u32',
    'png_const_uint_32p': '*const u32',
    'png_int_32p': '*mut i32',
    'png_uint_16p': '*mut u16',
    'png_const_uint_16p': '*const u16',
    'png_int_16p': '*mut i16',
    'png_fixed_point_p': '*mut i32',
    'png_const_fixed_point_p': '*const i32',
    'png_const_structp': '*mut c_void',
    'png_const_infop': '*mut c_void',
    'png_const_colorp': '*const PngColor',
    'png_const_structrp': '*mut c_void',
    'png_doublep': '*mut f64',
    'png_const_doublep': '*const f64',
    'png_size_tp': '*mut usize',
    'png_alloc_size_tp': '*mut usize',
    'png_colorp': '*mut PngColor',
    'png_const_colorp': '*const PngColor',
    'png_colorpp': '*mut *mut PngColor',
    'png_color_8p': '*mut PngColor8',
    'png_const_color_8p': '*const PngColor8',
    'png_color_16p': '*mut PngColor16',
    'png_const_color_16p': '*const PngColor16',
    'png_color_16pp': '*mut *mut PngColor16',
    'png_sPLT_tp': '*mut c_void',
    'png_const_sPLT_tp': '*const c_void',
    'png_sPLT_tpp': '*mut *mut c_void',
    'png_textp': '*mut PngText',
    'png_const_textp': '*const PngText',
    'png_textpp': '*mut *mut PngText',
    'png_timep': '*mut PngTime',
    'png_const_timep': '*const PngTime',
    'png_unknown_chunkp': '*mut PngUnknownChunk',
    'png_const_unknown_chunkp': '*const PngUnknownChunk',
    'png_unknown_chunkpp': '*mut *mut PngUnknownChunk',
    'png_imagep': '*mut PngImage',
    'png_controlp': '*mut c_void',
    'png_row_infop': '*mut c_void',
    'png_error_ptr': '*mut c_void',
    'png_rw_ptr': '*mut c_void',
    'png_flush_ptr': '*mut c_void',
    'png_read_status_ptr': '*mut c_void',
    'png_write_status_ptr': '*mut c_void',
    'png_progressive_info_ptr': '*mut c_void',
    'png_progressive_row_ptr': '*mut c_void',
    'png_progressive_end_ptr': '*mut c_void',
    'png_user_transform_ptr': '*mut c_void',
    'png_user_chunk_ptr': '*mut c_void',
    'png_longjmp_ptr': '*mut c_void',
    'png_malloc_ptr': '*mut c_void',
    'png_free_ptr': '*mut c_void',
    'png_transform_fn': '*mut c_void',
    'FILE': 'c_void',
    'struct tm': 'c_void',
}


def ctype(t):
    t = re.sub(r'\bconst\b', ' const ', t)
    t = re.sub(r'\bPNG_RESTRICT\b', ' ', t)
    t = ' '.join(t.split())
    stars = t.count('*')
    t = t.replace('*', ' ')
    words = [w for w in t.split() if w not in ('const', 'PNGCBAPI', 'PNGAPI', 'PNGCAPI')]
    base = ' '.join(words)
    if base in BASE and stars == 0:
        return BASE[base]
    if base in BASE:
        inner = 'c_void' if base == 'void' else BASE[base]
        # a pointer to something already-a-pointer alias
        r = inner
        for _ in range(stars):
            r = '*mut ' + r
        return r
    if base == '':
        return None
    return None


def parse_args(a):
    a = a.strip()
    if a in ('void', ''):
        return []
    out = []
    for p in split_top(a):
        p = p.strip()
        if p == '...':
            return None
        # drop the parameter name: last identifier
        m = re.match(r'^(.*?)([A-Za-z_][A-Za-z0-9_]*)\s*(\[\s*\d*\s*\])?$', p)
        if not m:
            return None
        pre, nm, arr = m.group(1), m.group(2), m.group(3)
        if pre.strip() == '' or pre.strip() == 'const':
            # the "name" was actually the type (e.g. "void")
            pre, nm = p, ''
        t = ctype(pre)
        if arr:
            t = '*mut ' + (t or 'c_void')
        if t is None:
            return None
        out.append(t)
    return out


lines = []
skipped = []
for o, r, n, a in protos:
    rt = ctype(r)
    at = parse_args(a)
    if rt is None or at is None:
        skipped.append((n, r, a))
        continue
    rs = '' if rt == '()' else ' -> ' + rt
    lines.append('    fn %s(%s)%s;' % (n, ', '.join(at), rs))

print('// GENERATED by tools/gen_api.py from c_src/include/png.h -- do not edit by hand.')
print('api! {')
for l in lines:
    print(l)
print('}')
sys.stderr.write('emitted %d, skipped %d\n' % (len(lines), len(skipped)))
for s in skipped:
    sys.stderr.write('SKIP %s | %s | %s\n' % s)
