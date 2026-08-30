#!/bin/bash
# Run the differential test suite for every feature combination.
# Usage: test_all.sh [extra cargo test args...]
set -u
W="$(cd "$(dirname "$0")/.." && pwd)"
RUSTLIB=lib005_sphincs_PQCgenKAT_sign_blake_128f_simple.so
mkdir -p "$W/work" "$W/rustlibs"
fail=0
pass=0
while IFS=, read -r b t s; do
  cfg="${b}_${s}_${t}"
  "$W/scripts/build_c.sh" "$b" "$s" "$t" >/dev/null 2>&1 || { echo "C-BUILD-FAIL $cfg"; fail=1; continue; }
  (cd "$W/translation" && timeout 400 cargo build --offline --release --quiet \
      --no-default-features --features "$b,$t,$s" >/dev/null 2>&1) \
      || { echo "RUST-BUILD-FAIL $cfg"; fail=1; continue; }
  cp "$W/translation/target/release/$RUSTLIB" "$W/rustlibs/librust_$cfg.so"
  out=$(cd "$W/translation" && SPX_C_LIB="$W/cbuild/$cfg/libc_sphincs.so" \
        SPX_RUST_LIB="$W/rustlibs/librust_$cfg.so" \
        timeout 900 cargo test --offline --release --no-default-features \
        --features "$b,$t,$s" "$@" 2>&1)
  if echo "$out" | grep -qE '^(error|test result: FAILED)|FAILED|panicked'; then
    echo "TEST-FAIL $cfg"
    echo "$out" | grep -A 12 -E "panicked|^failures:|^error" | head -40
    fail=1
  else
    n=$(echo "$out" | grep -oE '[0-9]+ passed' | head -1)
    echo "ok      $cfg ($n)"
    pass=$((pass+1))
  fi
done < <("$W/scripts/all_combos.sh")
echo "=== $pass configurations passed, fail=$fail ==="
exit $fail
