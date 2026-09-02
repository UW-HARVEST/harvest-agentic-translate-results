#!/usr/bin/env bash
# Negative control for the differential suite: deliberately break the Rust
# translation in several distinct ways and confirm the tests CATCH each one.
# A suite that cannot fail proves nothing, so this must report FAILED for every
# mutation and then restore the original source.
set -uo pipefail

CRATE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SRC="$CRATE/src/lib.rs"
BAK="$(mktemp)"
cp "$SRC" "$BAK"
restore() { cp "$BAK" "$SRC"; rm -f "$BAK"; }
trap restore EXIT

BAD=0

mutate() {
  local name="$1" from="$2" to="$3"
  cp "$BAK" "$SRC"
  if ! grep -qF "$from" "$SRC"; then
    echo "  [$name] SKIPPED: pattern not found -- update this script"
    BAD=1
    return
  fi
  # literal, single-occurrence replacement
  perl -0pi -e "BEGIN{\$f=quotemeta(\$ENV{FROM}); \$t=\$ENV{TO}} s/\$f/\$t/" "$SRC"
  ( cd "$CRATE" && timeout 300 cargo build >/dev/null 2>&1 ) || {
    echo "  [$name] build failed (mutation not compilable) -- treated as undetected"
    BAD=1
    return
  }
  out=$( cd "$CRATE" && timeout 600 cargo test 2>&1 | grep -E '^test result:' | tail -1 )
  if echo "$out" | grep -q ' 0 failed'; then
    echo "  [$name] NOT DETECTED  <-- suite is too weak!   ($out)"
    BAD=1
  else
    echo "  [$name] detected       ($out)"
  fi
}

run() { FROM="$2" TO="$3" mutate "$1" "$2" "$3"; }

echo "=== mutation testing the differential suite ==================="

run "M1 wrong samples stride (8 elems instead of 32)" \
    'const BYTE_STRIDE: usize = ELEM_STRIDE * size_of::<tflac_s32>();' \
    'const BYTE_STRIDE: usize = ELEM_STRIDE;'

run "M2 carry-down source off by one (63 instead of 64)" \
    'ld_u8(buffer.wrapping_add(64usize + bytes as usize)),' \
    'ld_u8(buffer.wrapping_add(63usize + bytes as usize)),'

run "M3 pos advanced from pos%64 instead of raw pos" \
    'st_u32(pos, ld_u32(pos).wrapping_add(bytes));' \
    'st_u32(pos, pos2.wrapping_add(bytes));'

run "M4 wrong step in b -= step (4 instead of sizeof)" \
    'const STEP: tflac_u32 = size_of::<tflac_uint>() as tflac_u32;' \
    'const STEP: tflac_u32 = 4;'

run "M5 total updated with bits/8 instead of bits" \
    'st_u64(total, ld_u64(total).wrapping_add(bits as tflac_u64));' \
    'st_u64(total, ld_u64(total).wrapping_add((bits / 8) as tflac_u64));'

run "M6 branch uses > 64 instead of >= 64" \
    'if ld_u32(pos) >= 64 {' \
    'if ld_u32(pos) > 64 {'

run "M7 while(bytes--) mistranslated as pre-decrement (indices bytes..1)" \
    'while bytes != 0 {
                bytes = bytes.wrapping_sub(1);
                st_u8(
                    buffer.wrapping_add(bytes as usize),
                    ld_u8(buffer.wrapping_add(64usize + bytes as usize)),
                );
            }' \
    'while bytes != 0 {
                st_u8(
                    buffer.wrapping_add(bytes as usize),
                    ld_u8(buffer.wrapping_add(64usize + bytes as usize)),
                );
                bytes = bytes.wrapping_sub(1);
            }'

run "M8 checked raw deref instead of unchecked memcpy store (null behaviour)" \
    'unsafe { memcpy(p, &v as *const u8, 1) };' \
    'unsafe { *p = v };'

run "M9 mask 0xFF -> 0xFFFF on the first lane" \
    'let mut v: tflac_uint = ((ld_i32(sp.wrapping_add(0 * E)) as tflac_uint) & 0xFF) << 0;' \
    'let mut v: tflac_uint = ((ld_i32(sp.wrapping_add(0 * E)) as tflac_uint) & 0xFFFF) << 0;'

run "M10 pack byte 5 shifted by 41 instead of 40" \
    'st_u8(d.wrapping_add(5), (n >> 40) as tflac_u8);' \
    'st_u8(d.wrapping_add(5), (n >> 41) as tflac_u8);'

run "M11 loop runs 4 times instead of 5" \
    'while i <= 4 {' \
    'while i < 4 {'

echo
restore
trap - EXIT
( cd "$CRATE" && cargo build >/dev/null 2>&1 )
if [ "$BAD" -eq 0 ]; then
  echo "ALL MUTATIONS DETECTED -- the differential suite has real teeth"
else
  echo "AT LEAST ONE MUTATION SURVIVED -- strengthen the tests"
fi
exit "$BAD"
