import re
src = open('c_src/src/pcre2_ucp.h').read()
out = []
# handle enums
for m in re.finditer(r'enum\s*\{(.*?)\}\s*;', src, re.S):
    body = m.group(1)
    body = re.sub(r'/\*.*?\*/', '', body, flags=re.S)
    n = 0
    for item in body.split(','):
        item = item.strip()
        if not item: continue
        if '=' in item:
            name, val = [x.strip() for x in item.split('=')]
            n = int(val, 0)
        else:
            name = item
        out.append(f'pub const {name}: u32 = {n};')
        n += 1
# handle #define lines
for ln in src.split('\n'):
    m = re.match(r'#define\s+(ucd_[A-Za-z0-9_]+|UCD_[A-Za-z0-9_]+)\s+(\S+)', ln)
    if m and 'GUARD' not in m.group(1):
        out.append(f'pub const {m.group(1)}: u32 = {m.group(2)};')
hdr = '''// Unicode property value constants, mechanically translated from c_src/src/pcre2_ucp.h
#![allow(dead_code, non_upper_case_globals)]

'''
open('src/ucp.rs','w').write(hdr + '\n'.join(out) + '\n')
print(len(out), 'ucp constants')
