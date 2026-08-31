#!/usr/bin/env python3
"""Convert the sequential C enums in pcre2_ucp.h into Rust constants."""
import re, sys

src = open('$HARVEST_WORKDIR/c_src/src/pcre2_ucp.h').read()

# Strip comments
src = re.sub(r'/\*.*?\*/', '', src, flags=re.S)

out = ["// Generated from c_src/src/pcre2_ucp.h -- Unicode property value constants.",
       "#![allow(dead_code, non_upper_case_globals)]", ""]

for m in re.finditer(r'enum\s*\{(.*?)\}\s*;', src, flags=re.S):
    body = m.group(1)
    idx = 0
    for item in body.split(','):
        item = item.strip()
        if not item:
            continue
        if '=' in item:
            name, val = item.split('=')
            name = name.strip()
            val = val.strip()
            idx = int(val, 0)
        else:
            name = item
        out.append(f"pub const {name}: u32 = {idx};")
        idx += 1
    out.append("")

open('$HARVEST_WORKDIR/translation/src/ucp.rs', 'w').write('\n'.join(out) + '\n')
print("wrote", len(out), "lines")
