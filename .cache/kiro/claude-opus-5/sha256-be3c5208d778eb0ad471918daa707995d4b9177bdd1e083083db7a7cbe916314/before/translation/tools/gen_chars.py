#!/usr/bin/env python3
"""Generate Rust CHAR_* / STR_* / STRING_* constants from pcre2_internal.h.

Only the "UTF-8 support is enabled" (ASCII) branch is used, matching a build
with SUPPORT_UNICODE defined on an ASCII platform.
"""
import re

H = '$HARVEST_WORKDIR/c_src/src/pcre2_internal.h'
OUT = '$HARVEST_WORKDIR/translation/src/chars.rs'

lines = open(H).read().split('\n')
# The ASCII/UTF-8 branch runs from the marker comment to the end-of-section marker.
start = next(i for i, l in enumerate(lines)
             if l.startswith('/* UTF-8 support is enabled; always use UTF-8'))
end = next(i for i, l in enumerate(lines)
           if 'End of character and string names' in l)
body = '\n'.join(lines[start:end])
body = re.sub(r'/\*.*?\*/', ' ', body, flags=re.S)

chars = {}
strs = {}
strings = {}


def char_value(tok):
    tok = tok.strip()
    m = re.fullmatch(r"\(\(unsigned char\)('.*')\)", tok)
    if m:
        tok = m.group(1)
    if tok in chars:
        return chars[tok]
    m = re.fullmatch(r"'(.*)'", tok)
    if not m:
        raise SystemExit('bad char token %r' % tok)
    s = m.group(1)
    if s.startswith('\\x'):
        return int(s[2:], 16)
    if s.startswith('\\'):
        return int(s[1:], 8)
    assert len(s) == 1, s
    return ord(s)


def str_value(tok):
    """Expand a sequence of STR_x tokens and/or string literals to bytes."""
    out = b''
    for t in re.findall(r'STR_\w+|"(?:[^"\\]|\\.)*"', tok):
        if t.startswith('STR_'):
            out += strs[t]
        else:
            lit = t[1:-1]
            i = 0
            while i < len(lit):
                if lit[i] == '\\':
                    m = re.match(r'\\([0-7]{1,3})', lit[i:])
                    if m:
                        out += bytes([int(m.group(1), 8)])
                        i += m.end()
                        continue
                    m = re.match(r'\\x([0-9a-fA-F]{1,2})', lit[i:])
                    if m:
                        out += bytes([int(m.group(1), 16)])
                        i += m.end()
                        continue
                    raise SystemExit('bad escape in %r' % lit)
                out += lit[i].encode()
                i += 1
    return out


for m in re.finditer(r'#define\s+(CHAR_\w+)\s+(.*)', body):
    chars[m.group(1)] = char_value(m.group(2))

for m in re.finditer(r'#define\s+(STR_\w+)\s+(.*)', body):
    strs[m.group(1)] = str_value(m.group(2))

for m in re.finditer(r'#define\s+(STRING_\w+)\s+(.*)', body):
    strings[m.group(1)] = str_value(m.group(2))

out = [
    "// Generated from c_src/src/pcre2_internal.h -- character and string names.",
    "#![allow(dead_code, non_upper_case_globals)]",
    "",
]
for k, v in chars.items():
    out.append(f"pub const {k}: u32 = {v};")
out.append("")
for k, v in strs.items():
    body_lit = ''.join('\\x%02x' % b for b in v)
    out.append(f'pub const {k}: &[u8] = b"{body_lit}";')
out.append("")
for k, v in strings.items():
    body_lit = ''.join('\\x%02x' % b for b in v)
    out.append(f'pub const {k}: &[u8] = b"{body_lit}";')

open(OUT, 'w').write('\n'.join(out) + '\n')
print('CHAR_:', len(chars), 'STR_:', len(strs), 'STRING_:', len(strings))
