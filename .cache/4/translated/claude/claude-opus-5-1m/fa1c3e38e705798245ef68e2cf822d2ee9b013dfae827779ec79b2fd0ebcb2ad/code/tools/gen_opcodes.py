import re
src = open('c_src/src/pcre2_internal.h').read()
out=[]
# ESC_ enum
m = re.search(r'enum \{ ESC_A = 1,(.*?)\};', src, re.S)
body = 'ESC_A = 1,' + m.group(1)
body = re.sub(r'/\*.*?\*/','',body,flags=re.S)
n=0
for item in body.split(','):
    item=item.strip()
    if not item: continue
    if '=' in item:
        name,val=[x.strip() for x in item.split('=')]; n=int(val)
    else: name=item
    out.append(f'pub const {name}: i32 = {n};'); n+=1
out.append('')
# OP_ enum
m = re.search(r'\nenum \{\n  OP_END,(.*?)\n\};', src, re.S)
body='OP_END,'+m.group(1)
body = re.sub(r'/\*.*?\*/','',body,flags=re.S)
n=0
for item in body.split(','):
    item=item.strip()
    if not item: continue
    assert re.match(r'^OP_[A-Za-z0-9_]+$', item), item
    out.append(f'pub const {item}: u32 = {n};'); n+=1
print('num opcodes', n)
hdr='''// Opcode and escape constants, mechanically translated from c_src/src/pcre2_internal.h
#![allow(dead_code, non_upper_case_globals)]

'''
open('src/opcodes.rs','w').write(hdr+'\n'.join(out)+'\n')

# META codes and ERR codes from pcre2_compile.h
src2 = open('c_src/src/pcre2_compile.h').read()
out2=[]
m = re.search(r'enum \{ ERR0 = COMPILE_ERROR_BASE,(.*?)\};', src2, re.S)
body='ERR0 = 100,'+m.group(1)
n=0
for item in body.split(','):
    item=item.strip()
    if not item: continue
    if '=' in item:
        name,val=[x.strip() for x in item.split('=')]; n=int(val)
    else: name=item
    out2.append(f'pub const {name}: i32 = {n};'); n+=1
out2.append('')
for ln in src2.split('\n'):
    m = re.match(r'#define\s+(META_[A-Za-z0-9_]+)\s+(0x[0-9a-fA-F]+)u', ln)
    if m: out2.append(f'pub const {m.group(1)}: u32 = {m.group(2)};')
hdr2='''// META codes and compile error codes, mechanically translated from c_src/src/pcre2_compile.h
#![allow(dead_code, non_upper_case_globals)]

'''
open('src/meta.rs','w').write(hdr2+'\n'.join(out2)+'\n')
print('meta/err lines', len(out2))
