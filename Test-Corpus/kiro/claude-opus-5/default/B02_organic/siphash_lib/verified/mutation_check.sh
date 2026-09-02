#!/usr/bin/env bash
# Test-sensitivity check (negative control): inject a deliberate divergence into
# the Rust translation, confirm the differential suite CATCHES it, then restore.
# A suite that passes on a mutant is not verifying anything.
set -uo pipefail
cd "$(dirname "$0")"
BAK=$(mktemp); cp src/lib.rs "$BAK"
trap 'cp "$BAK" src/lib.rs; rm -f "$BAK"; cargo build -q; cargo build -q --release' EXIT

patch () {
  python3 -c '
import sys
p="src/lib.rs"; s=open(p).read()
old,new=sys.argv[1],sys.argv[2]
if old not in s: sys.exit("PATTERN NOT FOUND: "+old)
open(p,"w").write(s.replace(old,new,1))' "$1" "$2"
}

KILLED=0; SURVIVED=0
mutant () {
  local name="$1"; shift
  cp "$BAK" src/lib.rs
  patch "$1" "$2" || { echo "SKIP  $name (pattern drifted)"; return; }
  cargo build -q 2>/dev/null && cargo build -q --release 2>/dev/null || {
    echo "SKIP  $name (mutant does not compile)"; return; }
  local failed=0
  for t in phase_b_valid phase_c_errors siphash_stdout; do
    timeout 600 cargo test -q --test "$t" >/dev/null 2>&1 || failed=1
  done
  if [ "$failed" -eq 1 ]; then echo "KILLED   $name"; KILLED=$((KILLED+1))
  else                         echo "SURVIVED $name"; SURVIVED=$((SURVIVED+1)); fi
}

mutant "M1  tail sign-extension removed" \
  '(((byte as u32) << shift) as i32) as i64 as u64 as usize' '((byte as u32) << shift) as usize'
mutant "M2  block low-half sign-extension removed" \
  'data = (lo as i32) as i64 as u64 as usize;' 'data = lo as usize;'
mutant "M4  sipround rotate 21 -> 20" \
  '$v3 = $v3.rotate_left(21);' '$v3 = $v3.rotate_left(20);'
mutant "M5  siphash z wrapping -> saturating" \
  'z = z.wrapping_add(1);' 'z = z.saturating_add(1);'
mutant "M6  tail case 7 dropped" \
  'if rem >= 7 {' 'if rem > 7 {'
mutant "M7  length term len<<56 -> len<<55" \
  'data = len << (SIZE_T_BITS - 8);' 'data = len << (SIZE_T_BITS - 9);'
mutant "M8  finalisation v2 ^= 0xff dropped" \
  'v2 ^= 0xff;' 'v2 ^= 0x00;'
mutant "M9  finalisation rounds 4 -> 3" \
  'for _j in 0..4 {' 'for _j in 0..3 {'
mutant "M10 v1 init !seed -> seed" \
  '0x6e64_6f6d)) ^ !seed;' '0x6e64_6f6d)) ^ seed;'
mutant "M11 tail case 6 shift 40 -> 41" \
  '((*d.add(5) as usize) << 20) << 20' '((*d.add(5) as usize) << 21) << 20'
mutant "M14 tail case 5 shift 32 -> 31" \
  '((*d.add(4) as usize) << 16) << 16' '((*d.add(4) as usize) << 16) << 15'
mutant "M15 tail case 7 shift 48 -> 47" \
  '((*d.add(6) as usize) << 24) << 24' '((*d.add(6) as usize) << 24) << 23'
mutant "M16 tail case 4 boundary off-by-one" \
  'if rem >= 4 {' 'if rem >= 5 {'
mutant "M18 block loop bound <= -> <" \
  'while i + core::mem::size_of::<usize>() <= len' 'while i + core::mem::size_of::<usize>() < len'
mutant "M19 final xor drops v3" \
  'v0 ^ v1 ^ v2 ^ v3' 'v0 ^ v1 ^ v2'
mutant "M20 block high-half shift 32 -> 31" \
  'data |= (hi_sext << 16) << 16;' 'data |= hi_sext << 31;'
mutant "M13 siphash mem fill masked to 7 bits" \
  'mem[i] = z as u8;' 'mem[i] = (z & 0x7f) as u8;'

echo
echo "killed=$KILLED survived=$SURVIVED"
# Note: the mutation "block high-half sign-extension removed"
#   let hi_sext = (hi as i32) as i64 as u64 as usize;  ->  let hi_sext = hi as usize;
# is deliberately NOT in this list: it is a semantically EQUIVALENT mutant,
# because the subsequent `<< 16 << 16` discards every bit the sign extension
# sets. See `d6_high_half_sign_extension_is_a_no_op` in tests/phase_d_symbols.rs.
[ "$SURVIVED" -eq 0 ]
