import re, sys
W='$HARVEST_WORKDIR'
D=W+'/c_src/libsodium/crypto_core/ed25519/ref10/fe_25_5/'

def nums(path):
    txt=open(path).read()
    txt=re.sub(r'/\*.*?\*/','',txt,flags=re.S)
    return [int(x) for x in re.findall(r'-?\d+', txt)]

b=nums(D+'base.h')
assert len(b)==32*8*3*10, len(b)
b2=nums(D+'base2.h')
assert len(b2)==8*3*10, len(b2)

def fe(v):
    return '['+', '.join(str(x) for x in v)+']'

out=[]
out.append('''//! Precomputed tables and field constants for ed25519 ref10 (fe_25_5).
//! Generated verbatim from
//! `c_src/libsodium/crypto_core/ed25519/ref10/fe_25_5/{base.h,base2.h,constants.h}`.
#![allow(dead_code)]

use crate::types::{fe25519, ge25519_precomp};

#[inline]
const fn p(yplusx: fe25519, yminusx: fe25519, xy2d: fe25519) -> ge25519_precomp {
    ge25519_precomp { yplusx, yminusx, xy2d }
}
''')

i=0
out.append('/// `static const ge25519_precomp base[32][8]`')
out.append('pub static base: [[ge25519_precomp; 8]; 32] = [')
for r in range(32):
    out.append('    [ // %d/31' % r)
    for c in range(8):
        a=b[i:i+10]; bb=b[i+10:i+20]; cc=b[i+20:i+30]; i+=30
        out.append('        p(%s, %s, %s),' % (fe(a), fe(bb), fe(cc)))
    out.append('    ],')
out.append('];')
out.append('')

i=0
out.append('/// `static const ge25519_precomp base2[8]`')
out.append('pub static base2: [ge25519_precomp; 8] = [')
for c in range(8):
    a=b2[i:i+10]; bb=b2[i+10:i+20]; cc=b2[i+20:i+30]; i+=30
    out.append('    p(%s, %s, %s),' % (fe(a), fe(bb), fe(cc)))
out.append('];')
out.append('')

# constants.h
txt=open(D+'constants.h').read()
consts=re.findall(r'static const fe25519 (\w+)\s*=\s*\{([^}]*)\};', txt, re.S)
for name, body in consts:
    body=re.sub(r'/\*.*?\*/','',body,flags=re.S)
    body=body.replace('ed25519_A_32','486662')
    v=[int(x) for x in re.findall(r'-?\d+', body)]
    assert len(v)==10, (name, v)
    out.append('pub static %s: fe25519 = %s;' % (name, fe(v)))
out.append('')
out.append('pub const ed25519_A_32: i32 = 486662;')
out.append('')
open(W+'/translation/src/ed25519_ref10_tables.rs','w').write('\n'.join(out)+'\n')
print('constants:', [n for n,_ in consts])
