#!/usr/bin/env python3
"""Convert pcre2_ucptables_inc.h (utt_names / utt) into Rust statics."""
import re

C = '$HARVEST_WORKDIR/c_src/src/pcre2_ucptables_inc.h'
OUT = '$HARVEST_WORKDIR/translation/src/ucptables.rs'

src = open(C).read()
src_nc = re.sub(r'/\*.*?\*/', ' ', src, flags=re.S)

STR = {'STR_AMPERSAND': '&'}
for ch in 'abcdefghijklmnopqrstuvwxyz':
    STR['STR_' + ch] = ch

# STRING_xxx macros -> literal text (each ends with an explicit NUL)
strings = {}
for m in re.finditer(r'#define\s+(STRING_\w+)\s+(.*)', src_nc):
    name, body = m.group(1), m.group(2)
    text = ''
    for tok in body.split():
        if tok in STR:
            text += STR[tok]
        elif tok == '"\\0"':
            text += '\0'
        else:
            raise SystemExit('unexpected token %r in %s' % (tok, name))
    strings[name] = text

# utt_names: ordered list of STRING_xxx tokens
m = re.search(r'PRIV\(utt_names\)\[\]\s*=\s*(.*?);', src_nc, flags=re.S)
names_body = m.group(1)
tokens = re.findall(r'STRING_\w+', names_body)
blob = ''.join(strings[t] for t in tokens)
# The C array is a concatenation of string literals; the last literal carries an
# implicit terminating NUL of its own.
blob += '\0'

# utt table
m = re.search(r'PRIV\(utt\)\[\]\s*=\s*\{(.*?)\n\};', src_nc, flags=re.S)
entries = re.findall(r'\{\s*(\d+)\s*,\s*(\w+)\s*,\s*(\w+)\s*\}', m.group(1))
assert len(entries) == m.group(1).count('{'), (len(entries), m.group(1).count('{'))

# sanity: every offset must be the start of a name in the blob
for off, _, _ in entries:
    off = int(off)
    assert off == 0 or blob[off - 1] == '\0', off

out = [
    "// Generated from c_src/src/pcre2_ucptables_inc.h -- Unicode property name tables.",
    "#![allow(dead_code, non_upper_case_globals)]",
    "",
    "use crate::internal::UcpTypeTable;",
    "use crate::internal::*;",
    "use crate::ucp::*;",
    "",
]

out.append(f"pub static UTT_NAMES: [u8; {len(blob)}] = [")
line = "   "
for c in blob:
    tok = f" {ord(c)},"
    if len(line) + len(tok) > 96:
        out.append(line)
        line = "   "
    line += tok
if line.strip():
    out.append(line)
out.append("];")
out.append("")

out.append(f"pub static UTT: [UcpTypeTable; {len(entries)}] = [")
for off, typ, val in entries:
    val_expr = val if val != '0' else '0'
    out.append(f"    UcpTypeTable {{ name_offset: {off}, type_: {typ} as u16, "
               f"value: {val_expr} as u16 }},")
out.append("];")
out.append("")
out.append(f"pub const UTT_SIZE: usize = {len(entries)};")

open(OUT, 'w').write('\n'.join(out) + '\n')
print("wrote", OUT, "names blob", len(blob), "entries", len(entries))
