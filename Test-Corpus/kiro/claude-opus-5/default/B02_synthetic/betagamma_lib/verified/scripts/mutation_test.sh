#!/usr/bin/env bash
# Mutation test: inject a deliberate bug into the Rust, confirm the suite
# CATCHES it, restore. A mutation that survives means the suite has a blind
# spot. Not part of normal verification -- this validates the TESTS.
set -uo pipefail
cd "$(dirname "$0")/.." || exit 1

GOOD=/tmp/good_lib.rs
cp src/lib.rs "$GOOD"

suite() {
  timeout 600 cargo build --release >/dev/null 2>&1 || { echo "BUILD-FAIL"; return; }
  timeout 600 cargo test --release -- --test-threads=1 >/tmp/t.log 2>&1
  local f p
  f=$(grep -oE '[0-9]+ failed' /tmp/t.log | awk '{s+=$1} END{print s+0}')
  p=$(grep -oE '[0-9]+ passed' /tmp/t.log | awk '{s+=$1} END{print s+0}')
  if [ "$f" -gt 0 ]; then
    echo "CAUGHT   ($f failed) :: $(grep -oE '^test [a-z_0-9]+ \.\.\. FAILED' /tmp/t.log | sed 's/^test //; s/ \.\.\. FAILED//' | paste -sd, -)"
  else
    echo "SURVIVED ($p passed, 0 failed)"
  fi
}

survivors=0
# Mutations that are PROVABLY semantically equivalent to the original are
# expected to survive; no test can detect them. Each is justified below.
mut() {
  local desc="$1" from="$2" to="$3" expect="${4:-catch}"
  cp "$GOOD" src/lib.rs
  python3 - "$from" "$to" <<'PY'
import sys, pathlib
frm, to = sys.argv[1], sys.argv[2]
p = pathlib.Path("src/lib.rs")
s = p.read_text()
if frm not in s:
    print("PATTERN-NOT-FOUND", file=sys.stderr); sys.exit(9)
p.write_text(s.replace(frm, to, 1))
PY
  if [ $? -ne 0 ]; then
    printf '%-52s -> PATTERN NOT FOUND\n' "$desc"; survivors=$((survivors+1)); return
  fi
  local r; r=$(suite)
  printf '%-52s -> %s\n' "$desc" "$r"
  if [ "$expect" = "equivalent" ]; then
    case "$r" in
      SURVIVED*) printf '%-52s    (expected: provably equivalent mutant)\n' "";;
      *) printf '%-52s    !! expected to survive but was CAUGHT\n' ""; survivors=$((survivors+1));;
    esac
    return
  fi
  case "$r" in SURVIVED*|BUILD-FAIL*|*"NOT FOUND"*) survivors=$((survivors+1));; esac
}

echo "=== baseline ==="
cp "$GOOD" src/lib.rs
printf '%-52s -> %s\n' "unmutated" "$(suite)"
echo
echo "=== mutations ==="

mut "M1  signed pointer compare (d1<d2)" \
    'if d1 < d2 {' 'if (d1 as isize) < (d2 as isize) {'
mut "M2  floor div instead of C truncation" \
    'wrapping_div(10)' 'div_euclid(10)'
mut "M3  flags sign-extended 255 -> -1" \
    'special.flags as c_int' 'special.flags as i8 as c_int'
mut "M4  drop the 0x55 (param4) mask branch" \
    'if flags & 0b0101_0101 != 0 {' 'if false {'
mut "M5  init_value+i saturating not wrapping" \
    '(init_value as isize as usize).wrapping_add(i) as u32 as c_int' \
    'init_value.saturating_add(i as c_int)'
mut "M6  add a null guard the C does not have" \
    'let mut hash: c_int = 0;' 'if mb1.is_null() || mb2.is_null() { return -1; }
    let mut hash: c_int = 0;'
mut "M7  allocate_block returns NULL for count==0" \
    'let mb = malloc(core::mem::size_of::<MemoryBlock>())' \
    'if count == 0 { return ptr::null_mut(); }
    let mb = malloc(core::mem::size_of::<MemoryBlock>())'
mut "M8  block_size zero-extended not sign-extended" \
    'param1.wrapping_rem(10).wrapping_add(5) as isize as usize' \
    'param1.wrapping_rem(10).wrapping_add(5) as u32 as usize'
mut "M9  hash 100 -> 101" \
    'hash.wrapping_add(100)' 'hash.wrapping_add(101)'
mut "M10 hash struct-order 10 <-> 20" \
    'if p1 < p2 {
        hash = hash.wrapping_add(10);' 'if p1 < p2 {
        hash = hash.wrapping_add(20);'
mut "M11 special.id 99 -> 98" \
    'wrapping_add(special.id)' 'wrapping_add(98)'
# EQUIVALENT: none of the three fixed flag bytes (0xAA, 0xCC, 0xF0) has bit 0
# set, so masking with 0x0E instead of 0x0F cannot change any branch outcome.
mut "M12 mask 0x0F -> 0x0E  [equivalent]" \
    'if flags & 0b0000_1111 != 0 {' 'if flags & 0b0000_1110 != 0 {' equivalent
# Non-equivalent variant of the same branch: 0x01 zeroes the test for all three
# blocks, so param1 stops contributing entirely.
mut "M12b mask 0x0F -> 0x01 (kills param1 branch)" \
    'if flags & 0b0000_1111 != 0 {' 'if flags & 0b0000_0001 != 0 {'
# EQUIVALENT: 0xCD and 0xCC test non-zero against all four masks
# (0x0F, 0xF0, 0xAA, 0x55) alike, and `flags` is used for nothing else.
mut "M13 block2 flags 0xCC -> 0xCD  [equivalent]" \
    'flags: 0b1100_1100,' 'flags: 0b1100_1101,' equivalent
# Non-equivalent variant: 0x0C clears the 0xF0 and 0xAA tests for block 2.
mut "M13b block2 flags 0xCC -> 0x0C" \
    'flags: 0b1100_1100,' 'flags: 0b0000_1100,'
mut "M14 block ids 1,2,3 -> id 2 becomes 3" \
    'id: 2,' 'id: 3,'
mut "M15 create_block skips the strcpy" \
    'strcpy(ptr::addr_of_mut!((*p).name) as *mut c_char, name);' \
    'let _ = name;'
mut "M16 allocate_block size not stored" \
    '(*mb).size = count;' '(*mb).size = count.wrapping_add(0) & !1;'
mut "M17 free_block skips the inner free" \
    'free((*mb).data as *mut c_void);' '{}'
mut "M18 data!=data guard inverted" \
    'if (*mem1).data != (*mem2).data {' 'if (*mem1).data == (*mem2).data {'
# EQUIVALENT: mem1 and mem2 are both allocate_block(block_size, ..) with the
# SAME block_size, so mem1->size == mem2->size for every possible input.
mut "M19 sum bound mem1->size -> mem2->size  [equivalent]" \
    'while k < (*mem1).size {' 'while k < (*mem2).size {' equivalent
# Non-equivalent variant: actually drop the last element of the first sum.
mut "M19b sum loop drops mem1's last element" \
    'while k < (*mem1).size {' 'while k + 1 < (*mem1).size {'
mut "M20 calloc elem size 4 -> 8" \
    'calloc(count, core::mem::size_of::<c_int>())' 'calloc(count, 8)'

cp "$GOOD" src/lib.rs
timeout 600 cargo build --release >/dev/null 2>&1
echo
if [ "$survivors" -eq 0 ]; then
  echo "ALL MUTATIONS CAUGHT -- the suite has no detected blind spot."
else
  echo "$survivors mutation(s) SURVIVED -- the suite has a blind spot; add coverage."
fi
if diff -q "$GOOD" src/lib.rs >/dev/null; then echo "src/lib.rs restored."; else echo "WARNING: src/lib.rs NOT restored"; fi
exit "$survivors"
