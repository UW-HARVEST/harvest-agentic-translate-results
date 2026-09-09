#!/bin/bash
# Independent mutation check: each entry breaks one line of the Rust
# translation and asserts the named differential suite CATCHES it.
set -uo pipefail
cd $HARVEST_WORKDIR/translation
L=$HARVEST_WORKDIR/_logs
run() {
  local file="$1" from="$2" to="$3" test="$4"
  cp "src/$file" "$L/mut.bak"
  python3 - "$file" "$from" "$to" <<'PY'
import sys
f,a,b=sys.argv[1],sys.argv[2],sys.argv[3]
p="src/"+f; s=open(p).read()
assert a in s, f"pattern not found in {f}: {a!r}"
open(p,"w").write(s.replace(a,b,1))
PY
  if [ $? -ne 0 ]; then echo "SKIP  $file (pattern absent)"; cp "$L/mut.bak" "src/$file"; return; fi
  cargo build --offline --release >/dev/null 2>&1
  if cargo test --offline --release --test "$test" -- --test-threads=1 >"$L/mut_$test.log" 2>&1; then
      echo "*** NOT CAUGHT: $file [$from -> $to] did not fail $test ***"
  else
      echo "caught: $file [$from -> $to] -> $test fails as expected"
  fi
  cp "$L/mut.bak" "src/$file"
  cmp -s "$L/mut.bak" "src/$file" && echo "  reverted ok"
}
run aead_chacha20poly1305.rs "const CRYPTO_STREAM_CHUNK: u64 = 131072;" "const CRYPTO_STREAM_CHUNK: u64 = 65536;" t05_g4_aead
run ed25519_ref10_ge.rs "*s.add(31) ^= (fe25519_isnegative(&x) << 7) as u8;" "*s.add(31) ^= (fe25519_isnegative(&x) << 6) as u8;" t07_g5_asym
run scrypt.rs "x[4] ^= rotl32(x[0].wrapping_add(x[12]), 7);" "x[4] ^= rotl32(x[0].wrapping_add(x[12]), 8);" t11_g2_pwhash
cargo build --offline --release >/dev/null 2>&1
