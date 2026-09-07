#!/usr/bin/env bash
# Mutation check: inject a deliberate divergence into the Rust translation and
# confirm the differential suite FAILS. Proves the tests have detection power
# rather than passing vacuously. Always restores src/lib.rs.
set -uo pipefail
cd "$(dirname "$0")"

SRC=src/lib.rs
BAK=$(mktemp)
cp "$SRC" "$BAK"
restore() { cp "$BAK" "$SRC"; rm -f "$BAK"; cargo build --release --lib >/dev/null 2>&1; }
trap restore EXIT

# name | sed expression
MUTATIONS=(
  "memchra target byte|s/let target = c as u8;/let target = (c as u8) ^ 1;/"
  "interpret_as_int endianness|s/c_int::from_le_bytes/c_int::from_be_bytes/"
  "safe_sum_array init|s/let mut sum: c_int = 0;/let mut sum: c_int = 1;/"
  "float window upper bound|s/f < 1000.0f32/f < 100.0f32/"
  "complex_iteration mask|s/(u \& 0xFF) as c_int/(u \& 0xFE) as c_int/"
  "process_strings scale|s/matches.wrapping_mul(5)/matches.wrapping_mul(6)/"
  "buf_sum modulus|s/buf_sum % 256/buf_sum % 255/"
  "dash_count scale|s/dash_count.wrapping_mul(10)/dash_count.wrapping_mul(11)/"
  "snprintf cap off-by-one|s/let max = buffer.len() - 1;/let max = buffer.len() - 2;/"
  "process_buffer signedness|s/(buffer\[i\] as i8) as c_int/(buffer[i] as u8) as c_int/"
)

fail=0
for m in "${MUTATIONS[@]}"; do
  name="${m%%|*}"; expr="${m#*|}"
  cp "$BAK" "$SRC"
  sed -i "$expr" "$SRC"
  if diff -q "$BAK" "$SRC" >/dev/null; then
    echo "SKIP (pattern not found): $name"
    continue
  fi
  if ! cargo build --release --lib >/dev/null 2>&1; then
    echo "SKIP (mutant does not compile): $name"
    continue
  fi
  if timeout 300 cargo test --release --no-fail-fast >/dev/null 2>&1; then
    echo "SURVIVED (tests did NOT catch it): $name"
    fail=1
  else
    echo "KILLED: $name"
  fi
done

cp "$BAK" "$SRC"
cargo build --release --lib >/dev/null 2>&1
if timeout 300 cargo test --release >/dev/null 2>&1; then
  echo "baseline restored and green"
else
  echo "ERROR: baseline is NOT green after restore"; fail=1
fi
exit "$fail"
