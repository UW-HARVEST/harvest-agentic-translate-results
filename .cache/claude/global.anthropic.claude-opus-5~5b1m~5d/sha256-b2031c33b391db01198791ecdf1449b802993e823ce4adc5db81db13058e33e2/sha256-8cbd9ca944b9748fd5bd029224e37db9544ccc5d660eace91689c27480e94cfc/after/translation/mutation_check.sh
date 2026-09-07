#!/usr/bin/env bash
# NEGATIVE CONTROL for the differential suite.
#
# Proves the tests are not vacuous: each mutation below is injected into
# src/lib.rs, a .so is built from it, and the suite is pointed at that mutant.
# A mutant that SURVIVES is either a test blind spot or a semantically
# equivalent rewrite (see the notes at the bottom).
#
# src/lib.rs is always restored, and the real .so is rebuilt at the end.
set -uo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
cd "$here"

MUT="$here/target/mutants"
mkdir -p "$MUT"
cp src/lib.rs "$MUT/lib.rs.orig"
restore() { cp "$MUT/lib.rs.orig" src/lib.rs; }
trap 'restore; touch src/lib.rs; cargo build --offline -q 2>/dev/null; cargo build --release --offline -q 2>/dev/null' EXIT

# name : from : to
mutations=(
  "endianness|c_int::from_le_bytes(raw)|c_int::from_be_bytes(raw)"
  "float_lt_to_le|if f > 0.0f32 && f < 1000.0f32|if f > 0.0f32 && f <= 1000.0f32"
  "char_unsigned|result.wrapping_add(byte as i8 as c_int)|result.wrapping_add(byte as c_int)"
  "dash_multiplier|dash_count.wrapping_mul(10)|dash_count.wrapping_mul(11)"
  "complex_mask|result ^= (u & 0xFF) as c_int;|result ^= (u & 0xFFFF) as c_int;"
  "fzero_gt_to_ge|if f > 0.0f32 &&|if f >= 0.0f32 &&"
  "strings_prefix|if s.len() >= n && &s[..n] == target|if s.len() == n && &s[..n] == target"
  "sum_to_xor|sum = sum.wrapping_add(arr[idx]);|sum ^= arr[idx];"
  "buf_mod|buf_sum % 256|buf_sum % 255"
  "matches_mul|matches.wrapping_mul(5)|matches.wrapping_mul(6)"
)

for m in "${mutations[@]}"; do
  name="${m%%|*}"; rest="${m#*|}"; from="${rest%%|*}"; to="${rest##*|}"
  restore
  python3 - "$from" "$to" <<'PY'
import sys
frm, to = sys.argv[1], sys.argv[2]
s = open('src/lib.rs').read()
assert frm in s, f"pattern not found: {frm!r}"
open('src/lib.rs','w').write(s.replace(frm, to))
PY
  if [ $? -ne 0 ]; then echo "SKIP $name (pattern not found)"; continue; fi
  touch src/lib.rs
  cargo build --offline -q 2>/dev/null || { echo "SKIP $name (build failed)"; continue; }
  cp target/debug/libmemchra2_lib.so "$MUT/$name.so"
  restore
  HARVEST_RUST_SO="$MUT/$name.so" \
    cargo test --release --offline --test phase_b_configs --test phase_c_errors \
      --no-fail-fast >/dev/null 2>&1
  if [ $? -ne 0 ]; then echo "KILLED   $name"; else echo "SURVIVED $name"; fi
  rm -f "$MUT/$name.so"
done

restore
rm -f "$MUT/lib.rs.orig"
cat <<'NOTES'

Expected result: every mutant KILLED except these two, which are provably
EQUIVALENT to the C on all reachable inputs (not test gaps):

  char_unsigned   - the snprintf buffer only ever holds "test", ASCII digits and
                    '-', all <= 0x7F, so signed vs unsigned `char` widening give
                    identical values. (`as i8` is still the faithful spelling:
                    C `char` is signed on x86-64.)
  fzero_gt_to_ge  - `f >= 0.0` additionally admits f == +/-0.0, but then
                    `result += (int)0.0` adds 0, so the return value is
                    unchanged.
NOTES
