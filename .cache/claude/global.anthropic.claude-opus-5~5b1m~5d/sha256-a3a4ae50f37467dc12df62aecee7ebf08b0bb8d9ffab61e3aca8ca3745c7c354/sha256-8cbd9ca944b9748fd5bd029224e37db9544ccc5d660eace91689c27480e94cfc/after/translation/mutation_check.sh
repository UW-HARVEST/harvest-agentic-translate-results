#!/usr/bin/env bash
# Sensitivity check for the differential suite: build deliberately-broken
# variants of the Rust library and confirm the tests CATCH each one.
# A suite that passes against a mutant is not testing anything.
#
# src/lib.rs is backed up inside the repo (stable path) and always restored.
set -u
cd "$(dirname "$0")"

WORK="$(pwd)/.mutants"
BAK="$WORK/lib.rs.orig"
mkdir -p "$WORK"
cp src/lib.rs "$BAK"
restore() { cp "$BAK" src/lib.rs; }
trap restore EXIT

fail=0

mutate() { # name  python-expression-file-content
  local name="$1"; shift
  restore
  if ! python3 -c "$1"; then
    echo "!! mutation '$name' could not be applied"; fail=1; restore; return
  fi
  cargo build --release --offline --target-dir "$WORK/$name" >/dev/null 2>&1
  local so="$WORK/$name/release/libcolourblind_lib.so"
  restore
  if [ ! -f "$so" ]; then
    echo "!! mutation '$name' failed to build"; fail=1; return
  fi
  local out
  out=$(RUST_SO="$so" cargo test --release --offline 2>&1)
  # A mutant is "killed" if any test reports FAILED, or if the harness itself
  # dies (a mutant that dereferences NULL on the no-op path segfaults, which is
  # equally a detected divergence).
  if echo "$out" | grep -qE 'test result: FAILED|\.\.\. FAILED|error: test failed|SIGSEGV|SIGABRT'; then
    local killers
    killers=$(echo "$out" | grep -c '\.\.\. FAILED')
    local extra=""
    echo "$out" | grep -q 'SIGSEGV' && extra=" + harness SIGSEGV"
    echo "KILLED   $name  ($killers test(s) caught it$extra)"
  else
    echo "SURVIVED $name  <-- THE SUITE IS BLIND TO THIS BUG"
    fail=1
  fi
}

mutate nan_operand_order "
s=open('src/lib.rs').read()
a='red.write_unaligned(add(t3, add(t1, t2)));'
assert s.count(a)>=1
open('src/lib.rs','w').write(s.replace(a,'red.write_unaligned(add(add(t1, t2), t3));',1))"

mutate coefficient_one_ulp "
s=open('src/lib.rs').read()
import struct
a='0.82944301379913f32'
old=struct.unpack('<f',struct.pack('<f',0.82944301379913))[0]
new=struct.unpack('<f',struct.pack('<I',struct.unpack('<I',struct.pack('<f',old))[0]+1))[0]
assert a in s
open('src/lib.rs','w').write(s.replace(a,repr(new)+'f32',1))"

mutate sign_flip "
s=open('src/lib.rs').read()
a='mul(r, -0.00451714424166f32)'
assert a in s
open('src/lib.rs','w').write(s.replace(a,'mul(r, 0.00451714424166f32)',1))"

mutate dispatch_swap "
s=open('src/lib.rs').read()
a='cbProtanopia => protanopia(r, g, b),'
assert a in s
open('src/lib.rs','w').write(s.replace(a,'cbProtanopia => deuteranopia(r, g, b),',1))"

mutate default_arm_acts "
s=open('src/lib.rs').read()
a='_ => {}'
assert a in s
open('src/lib.rs','w').write(s.replace(a,'_ => tritanopia(r, g, b),',1))"

mutate snapshot_reread "
s=open('src/lib.rs').read()
a='''    red.write_unaligned(sub(add(t_g, r), t_b));
'''
assert a in s
open('src/lib.rs','w').write(s.replace(a,a+'    let g = green.read_unaligned();\n    let b = blue.read_unaligned();\n',1))"

mutate store_order "
s=open('src/lib.rs').read()
a='''    // t3 + (t1 + t2)
    let t1 = mul(r, -4.486E-11f32);
    let t2 = mul(0.87390929928361f32, g);
    let t3 = mul(0.12609070101523f32, b);
    green.write_unaligned(add(t3, add(t1, t2)));
'''
assert a in s
open('src/lib.rs','w').write(s.replace(a,'''    // t3 + (t1 + t2)
    let t1 = mul(r, -4.486E-11f32);
    let t2 = mul(0.87390929928361f32, g);
    let t3 = mul(0.12609070101523f32, b);
    let gv = add(t3, add(t1, t2));
''',1).replace('    blue.write_unaligned(add(t3, add(t1, t2)));\n}','    blue.write_unaligned(add(t3, add(t1, t2)));\n    green.write_unaligned(gv);\n}',1))"

mutate subtraction_to_addition "
s=open('src/lib.rs').read()
a='green.write_unaligned(sub(add(t2, t1), t3));'
assert a in s
open('src/lib.rs','w').write(s.replace(a,'green.write_unaligned(add(add(t2, t1), t3));',1))"

restore
echo
if [ "$fail" -eq 0 ]; then
  echo 'ALL MUTANTS KILLED - the differential suite is sensitive.'
else
  echo 'SOME MUTANTS SURVIVED - see above.'
fi
exit "$fail"
