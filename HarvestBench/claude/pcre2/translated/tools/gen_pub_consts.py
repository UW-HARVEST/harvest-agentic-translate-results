import re, sys
src = open('c_src/include/pcre2.h').read()
lines = src.split('\n')
skip = {'PCRE2_PRERELEASE','PCRE2_DATE','PCRE2_SIZE','PCRE2_SIZE_MAX','PCRE2_ZERO_TERMINATED','PCRE2_UNSET',
        'PCRE2_CALL_CONVENTION','PCRE2_EXP_DECL','PCRE2_JOIN','PCRE2_GLUE','PCRE2_SUFFIX','PCRE2_UCHAR',
        'PCRE2_SPTR','PCRE2_LOCAL_WIDTH','PCRE2_H_IDEMPOTENT_GUARD','PCRE2_TYPES_LIST','PCRE2_STRUCTURE_LIST'}
out=[]
u32_prefix = ('PCRE2_INFO_','PCRE2_CONFIG_','PCRE2_NEWLINE_','PCRE2_BSR_','PCRE2_OPTIMIZATION_',
              'PCRE2_SUBSTITUTE_CASE_','PCRE2_AUTO_POSSESS','PCRE2_DOTSTAR_ANCHOR','PCRE2_START_OPTIMIZE')
for ln in lines:
    m = re.match(r'#define\s+(PCRE2_[A-Za-z0-9_]+)\s+(\S+)\s*(/\*.*)?$', ln)
    if not m: continue
    name, val = m.group(1), m.group(2)
    if name in skip: continue
    if '(' in val and not re.match(r'^\(-\d+\)$', val): continue
    if val.endswith('u'):
        out.append(f'pub const {name}: u32 = {val[:-1]};')
    elif re.match(r'^\(-\d+\)$', val):
        out.append(f'pub const {name}: i32 = {val[1:-1]};')
    elif re.match(r'^\d+$', val):
        if name.startswith(u32_prefix):
            out.append(f'pub const {name}: u32 = {val};')
        else:
            out.append(f'pub const {name}: i32 = {val};')
    else:
        print('SKIP', ln, file=sys.stderr)
hdr = '''// Public API constants, mechanically translated from c_src/include/pcre2.h
#![allow(dead_code, non_upper_case_globals)]

'''
open('src/consts_pub.rs','w').write(hdr + '\n'.join(out) + '\n')
print(len(out), 'constants')
