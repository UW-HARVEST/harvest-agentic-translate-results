#!/usr/bin/env python3
"""Convert the numeric data tables in pcre2_ucd.c into Rust statics.

Only the SUPPORT_UNICODE branch is emitted (the CMake build defines
SUPPORT_UNICODE) and the 32-bit-only dummy record is skipped.
"""
import re

C = '$HARVEST_WORKDIR/c_src/src/pcre2_ucd.c'
OUT = '$HARVEST_WORKDIR/translation/src/ucd.rs'

src = open(C).read()

# Keep only the SUPPORT_UNICODE branch: from '#else' after '#ifndef SUPPORT_UNICODE'
i = src.index('#ifndef SUPPORT_UNICODE')
j = src.index('#else', i)
src = src[j:]

# Drop the 32-bit only dummy record block
src = re.sub(r'#if PCRE2_CODE_UNIT_WIDTH == 32.*?#endif', '', src, flags=re.S)

# Strip comments
src = re.sub(r'/\*.*?\*/', ' ', src, flags=re.S)


SYMBOLS = {
    'NOTACHAR': 0xffffffff,
}


def numbers(text):
    """Parse a C initialiser list, rejecting anything unrecognised."""
    out = []
    for tok in re.split(r'[,\s]+', text.strip()):
        if not tok:
            continue
        tok = tok.rstrip('uUlL')
        if tok in SYMBOLS:
            out.append(SYMBOLS[tok])
            continue
        m = re.fullmatch(r'-?(?:0[xX][0-9a-fA-F]+|\d+)', tok)
        if not m:
            raise SystemExit('unrecognised token in table: %r' % tok)
        out.append(int(tok, 0))
    return out


out = [
    "// Generated from c_src/src/pcre2_ucd.c -- Unicode character database tables.",
    "#![allow(dead_code, non_upper_case_globals)]",
    "",
    "use crate::internal::UcdRecord;",
    "",
]

# unicode_version
m = re.search(r'PRIV\(unicode_version\)\s*=\s*"([^"]*)"', src)
out.append(f'pub const UNICODE_VERSION: &[u8] = b"{m.group(1)}\\0";')
out.append("")

# ucd_records
m = re.search(r'PRIV\(ucd_records\)\[\]\s*=\s*\{(.*?)\n\};', src, flags=re.S)
recs = re.findall(r'\{([^{}]*)\}', m.group(1))
out.append(f"pub static UCD_RECORDS: [UcdRecord; {len(recs)}] = [")
for r in recs:
    v = numbers(r)
    assert len(v) == 7, (v, r)
    out.append(
        "    UcdRecord { script: %d, chartype: %d, gbprop: %d, caseset: %d, "
        "other_case: %d, scriptx_bidiclass: %d, bprops: %d }," % tuple(v))
out.append("];")
out.append("")


def emit_array(cname, rname, rtype):
    m = re.search(r'PRIV\(' + cname + r'\)\[\]\s*=\s*\{(.*?)\n\};', src, flags=re.S)
    v = numbers(m.group(1))
    out.append(f"pub static {rname}: [{rtype}; {len(v)}] = [")
    line = "   "
    for x in v:
        tok = f" {x},"
        if len(line) + len(tok) > 96:
            out.append(line)
            line = "   "
        line += tok
    if line.strip():
        out.append(line)
    out.append("];")
    out.append("")


emit_array('ucd_caseless_sets', 'UCD_CASELESS_SETS', 'u32')
emit_array('ucd_nocase_ranges', 'UCD_NOCASE_RANGES', 'u32')
emit_array('ucd_digit_sets', 'UCD_DIGIT_SETS', 'u32')
emit_array('ucd_script_sets', 'UCD_SCRIPT_SETS', 'u32')
emit_array('ucd_boolprop_sets', 'UCD_BOOLPROP_SETS', 'u32')
emit_array('ucd_stage1', 'UCD_STAGE1', 'u16')
emit_array('ucd_stage2', 'UCD_STAGE2', 'u16')

for cname, rname in (('ucd_turkish_dotted_i_caseset', 'UCD_TURKISH_DOTTED_I_CASESET'),
                     ('ucd_nocase_ranges_size', 'UCD_NOCASE_RANGES_SIZE')):
    m = re.search(r'PRIV\(' + cname + r'\)\s*=\s*([0-9xa-fA-F]+)\s*;', src)
    out.append(f"pub const {rname}: u32 = {int(m.group(1), 0)};")
out.append("")

open(OUT, 'w').write('\n'.join(out) + '\n')
print("wrote", OUT, len(out), "lines")
