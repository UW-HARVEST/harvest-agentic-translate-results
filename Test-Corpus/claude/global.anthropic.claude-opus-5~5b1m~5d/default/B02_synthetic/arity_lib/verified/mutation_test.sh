#!/usr/bin/env bash
# Mutation testing: prove the differential suite actually detects divergence,
# and in particular that diff_norm's retry loop does NOT mask real bugs.
set -uo pipefail
cd "$(dirname "$0")"
cp src/lib.rs src/lib.rs.orig
trap 'cp src/lib.rs.orig src/lib.rs; rm -f src/lib.rs.orig' EXIT

run_case() {
  local name="$1"; shift
  cp src/lib.rs.orig src/lib.rs
  "$@" || { echo "!! could not apply mutation $name"; return; }
  if ! cmp -s src/lib.rs src/lib.rs.orig; then :; else echo "!! mutation $name changed NOTHING"; return; fi
  cargo build --offline --release >/dev/null 2>&1 || { echo "  $name -> BUILD FAILED (counts as detected)"; return; }
  local out
  out=$(cargo test --offline --release -- --test-threads=1 2>&1 | grep -oE '[0-9]+ passed; [0-9]+ failed' | awk -F'[ ;]' '{p+=$1; f+=$4} END{print f" failed"}')
  echo "  $name -> $out"
}

echo "=== mutation battery ==="
run_case "negative-modulo (rem_euclid)"      sed -i 's/param1.wrapping_rem(4)/param1.rem_euclid(4)/' src/lib.rs
run_case "arity len<2 -> len<1"              sed -i 's/if len < 2 {/if len < 1 {/' src/lib.rs
run_case "arity no u8 truncation"            sed -i 's/let len: u8 = (len as u32 \& 0xff) as u8;/let len: i64 = len as i64;/' src/lib.rs
run_case "shift guard < -> <="               sed -i 's/if positions > 0 \&\& positions < size {/if positions > 0 \&\& positions <= size {/' src/lib.rs
run_case "bitmask default -> 0"              sed -i 's/^        _ => value,$/        _ => 0,/' src/lib.rs
run_case "cmp_alloc >0 -> >=0"               sed -i 's/if unsafe { \*uninit_ptr } > 0 { 10 }/if unsafe { *uninit_ptr } >= 0 { 10 }/' src/lib.rs
run_case "div floor instead of trunc"        sed -i 's/.wrapping_mul(param3).wrapping_div(100)/.wrapping_mul(param3).div_euclid(100)/' src/lib.rs
run_case "matrix[2][3] -> matrix[2][2]"      sed -i 's/.wrapping_add(matrix\[2\]\[3\])/.wrapping_add(matrix[2][2])/' src/lib.rs
run_case "process_string adds NULL check"    sed -i 's/    if unsafe { \*str } != 0 {/    if str.is_null() { return 0; }\n    if unsafe { *str } != 0 {/' src/lib.rs
run_case "ptr cmp signed instead of usize"   sed -i 's/if (ptr1 as usize) < (ptr2 as usize) {/if (ptr1 as isize) < (ptr2 as isize) {/' src/lib.rs
run_case "shift zero-fill loop dropped"      sed -i 's/^        while i < positions {$/        while i < 0 {/' src/lib.rs
run_case "arity3 forwards param4=1"          sed -i 's/    arity4(p1, p2, p3, 0)/    arity4(p1, p2, p3, 1)/' src/lib.rs
echo "=== restoring original ==="
