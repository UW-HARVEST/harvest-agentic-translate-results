#!/bin/bash
set -uo pipefail
cd $HARVEST_WORKDIR/translation
L=$HARVEST_WORKDIR/_logs
run() {
  local file="$1" from="$2" to="$3" test="$4"
  cp "src/$file" "$L/mut.bak"
  python3 - "$file" "$from" "$to" <<'PY' || { echo "SKIP $file (pattern absent)"; exit 0; }
import sys
f,a,b=sys.argv[1],sys.argv[2],sys.argv[3]
p="src/"+f; s=open(p).read()
assert a in s, f"pattern not found in {f}: {a!r}"
open(p,"w").write(s.replace(a,b,1))
PY
  cargo build --offline --release >/dev/null 2>&1
  if cargo test --offline --release --test "$test" -- --test-threads=1 >"$L/mut_$test.log" 2>&1; then
      echo "*** NOT CAUGHT: $file -> $test ***"
  else
      echo "caught: $file [${from:0:44}...] -> $test"
  fi
  cp "$L/mut.bak" "src/$file"
}
run softaes.rs "    0x63, 0x7c, 0x77, 0x7b, 0xf2, 0x6b, 0x6f, 0xc5, 0x30, 0x01, 0x67, 0x2b, 0xfe, 0xd7, 0xab, 0x76," "    0x63, 0x7c, 0x77, 0x7b, 0xf2, 0x6b, 0x6f, 0xc5, 0x30, 0x01, 0x67, 0x2b, 0xfe, 0xd7, 0xab, 0x77," t14_large_inputs
run ipcrypt.rs "diff[i] = *k.add(i) ^ 0x5a;" "diff[i] = *k.add(i) ^ 0x5b;" t09_g6_rand
run kem_mlkem768_ref.rs "t = (((a as u16 as u32).wrapping_mul(62209u32)) as u16) as i16;" "t = (((a as u16 as u32).wrapping_mul(62208u32)) as u16) as i16;" t09_g6_rand
run auth_hmac.rs "        *b = 0x36;" "        *b = 0x37;" t03_g3_hash
run shorthash.rs "    *v1 = rotl64(*v1, 13);" "    *v1 = rotl64(*v1, 14);" t03_g3_hash
run poly1305.rs "    let s1: u64 = r1.wrapping_mul(5);" "    let s1: u64 = r1.wrapping_mul(6);" t14_large_inputs
run blake2b.rs "    v[d] = rotr64(v[d] ^ v[a], 32);" "    v[d] = rotr64(v[d] ^ v[a], 31);" t03_g3_hash
run x25519_ref10.rs "" "" t07_g5_asym
cargo build --offline --release >/dev/null 2>&1
