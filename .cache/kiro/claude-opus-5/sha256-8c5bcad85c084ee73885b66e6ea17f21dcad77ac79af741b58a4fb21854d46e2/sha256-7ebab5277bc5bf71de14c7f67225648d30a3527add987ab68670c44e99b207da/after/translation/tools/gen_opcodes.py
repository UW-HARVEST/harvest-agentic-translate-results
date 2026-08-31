#!/usr/bin/env python3
"""Generate Rust opcode constants, OP_lengths and ESC_* constants."""
import re

H = '$HARVEST_WORKDIR/c_src/src/pcre2_internal.h'
OUT = '$HARVEST_WORKDIR/translation/src/opcodes.rs'

src = open(H).read()
nc = re.sub(r'/\*.*?\*/', ' ', src, flags=re.S)

# ---- opcode enum ----
m = re.search(r'enum\s*\{\s*OP_END\s*,(.*?)\n\};', nc, flags=re.S)
body = 'OP_END,' + m.group(1)
names = [t.strip() for t in body.split(',') if t.strip()]
assert names[0] == 'OP_END' and names[-1] == 'OP_TABLE_LENGTH', names[:2] + names[-2:]

# ---- escapes enum ----
m = re.search(r'enum\s*\{\s*ESC_A\s*=\s*1\s*,(.*?)\};', nc, flags=re.S)
esc = ['ESC_A'] + [t.strip() for t in m.group(1).split(',') if t.strip()]

# ---- OP_LENGTHS macro ----
i = src.index('#define OP_LENGTHS')
j = src.index('\n\n', i)
macro = src[i:j]
macro = re.sub(r'/\*.*?\*/', ' ', macro, flags=re.S)
macro = macro.replace('#define OP_LENGTHS', '').replace('\\', ' ')
macro = macro.replace('IMM2_SIZE', '2').replace('LINK_SIZE', '2')
macro = macro.replace('sizeof(PCRE2_UCHAR)', '1')
lengths = []
for tok in macro.split(','):
    tok = tok.strip()
    if not tok:
        continue
    lengths.append(int(eval(tok)))

assert len(lengths) == len(names) - 1, (len(lengths), len(names) - 1)

out = [
    "// Generated from c_src/src/pcre2_internal.h -- opcodes, opcode lengths, escapes.",
    "#![allow(dead_code, non_upper_case_globals)]",
    "",
]
for i, n in enumerate(names):
    out.append(f"pub const {n}: u8 = {i};")
out.append("")
out.append("pub const FIRST_AUTOTAB_OP: u8 = OP_NOT_DIGIT;")
out.append("pub const LAST_AUTOTAB_LEFT_OP: u8 = OP_EXTUNI;")
out.append("pub const LAST_AUTOTAB_RIGHT_OP: u8 = OP_DOLLM;")
out.append("")
out.append(f"pub static OP_LENGTHS: [u8; {len(lengths)}] = [")
line = "   "
for x in lengths:
    tok = f" {x},"
    if len(line) + len(tok) > 96:
        out.append(line)
        line = "   "
    line += tok
if line.strip():
    out.append(line)
out.append("];")
out.append("")
for i, n in enumerate(esc):
    out.append(f"pub const {n}: i32 = {i + 1};")

open(OUT, 'w').write('\n'.join(out) + '\n')
print('opcodes:', len(names), 'lengths:', len(lengths), 'escapes:', len(esc))
