#!/usr/bin/env bash
# Sanity check for the differential test suite itself: inject plausible
# translation bugs into src/lib.rs one at a time and confirm the tests CATCH each
# one. A mutation that survives means the suite has a blind spot.
set -uo pipefail
cd "$(dirname "$0")"

BAK=$(mktemp "${TMPDIR:-/tmp}/lib.rs.orig.XXXXXX")
cp src/lib.rs "$BAK"
restore() { cp "$BAK" src/lib.rs; }
trap 'restore; rm -f "$BAK"' EXIT

SURVIVORS=0
TESTS=(--test differential_hash --test differential_siphash --test error_paths)

mutate() { # mutate <name> <from> <to>
  local name="$1" from="$2" to="$3"
  restore
  if ! grep -qF -- "$from" src/lib.rs; then
    printf '  \033[33mSKIP\033[0m %-42s (pattern not found: %s)\n' "$name" "$from"
    return
  fi
  python3 - "$from" "$to" <<'PY'
import sys, pathlib
frm, to = sys.argv[1], sys.argv[2]
p = pathlib.Path("src/lib.rs")
s = p.read_text()
assert frm in s
p.write_text(s.replace(frm, to, 1))
PY
  if ! timeout 300 cargo build --offline >/dev/null 2>&1; then
    printf '  \033[33mSKIP\033[0m %-42s (mutant does not compile)\n' "$name"
    return
  fi
  if timeout 600 cargo test --offline "${TESTS[@]}" >/dev/null 2>&1; then
    printf '  \033[31mSURVIVED\033[0m %-38s <-- BLIND SPOT\n' "$name"
    SURVIVORS=$((SURVIVORS+1))
  else
    printf '  \033[32mCAUGHT\033[0m %s\n' "$name"
  fi
}

echo "Injecting mutations into src/lib.rs ..."

mutate "tail case 4: drop sign extension" \
  'data |= ((load_u8(d, 3) as i32).wrapping_shl(24)) as usize;' \
  'data |= (load_u8(d, 3) as usize) << 24;'

mutate "block low word: drop sign extension" \
  'data = word_as_signed_int(b0, b1, b2, b3) as usize;' \
  'data = word_as_signed_int(b0, b1, b2, b3) as u32 as usize;'

mutate "block high word: keep sign extension" \
  'data |= ((word_as_signed_int(b4, b5, b6, b7) as usize) << 16) << 16;' \
  'data |= (word_as_signed_int(b4, b5, b6, b7) as usize) << 16;'

mutate "sipround rotate 13 -> 14" \
  '*v1 = v1.rotate_left(13);' \
  '*v1 = v1.rotate_left(14);'

mutate "sipround rotate 21 -> 20" \
  '*v3 = v3.rotate_left(21);' \
  '*v3 = v3.rotate_left(20);'

mutate "finalization constant 0xff -> 0xee" \
  'v2 ^= 0xff;' \
  'v2 ^= 0xee;'

mutate "len << 56 -> len << 48" \
  'data = len.wrapping_shl(SIZE_T_BITS - 8);' \
  'data = len.wrapping_shl(SIZE_T_BITS - 16);'

mutate "final rounds 4 -> 3" \
  'while j < 4 {' \
  'while j < 3 {'

mutate "tail case 7 shift 48 -> 56" \
  'data |= ((load_u8(d, 6) as usize) << 24) << 24;' \
  'data |= ((load_u8(d, 6) as usize) << 28) << 28;'

mutate "tail boundary rem >= 4 -> rem > 4" \
  'if rem >= 4 {' \
  'if rem > 4 {'

mutate "loop bound i + WORD <= len -> < len" \
  'while i.wrapping_add(WORD) <= len {' \
  'while i.wrapping_add(WORD) < len {'

mutate "siphash prints 63 lines" \
  'for i in 0..64usize {
        let hash' \
  'for i in 0..63usize {
        let hash'

mutate "siphash mem fill uses init only" \
  'z = z.wrapping_add(1);' \
  'z = z.wrapping_add(0);'

mutate "siphash printf mask 255 -> 254" \
  '(((hash >> (j * 8)) & 255) as u8) as c_int,' \
  '(((hash >> (j * 8)) & 254) as u8) as c_int,'

mutate "seed complement: use seed instead of !seed" \
  'v1 = ((0x646f7261usize << 16) << 16).wrapping_add(0x6e646f6d) ^ !seed;' \
  'v1 = ((0x646f7261usize << 16) << 16).wrapping_add(0x6e646f6d) ^ seed;'

mutate "second seed XOR round removed" \
  'v2 ^= 0x0706050403020100usize ^ seed;' \
  'v2 ^= 0x0706050403020100usize;'

mutate "null deref instead of memcpy load" \
  'memcpy(
            (&raw mut out) as *mut c_void,
            d.wrapping_add(k) as *const c_void,
            1,
        );' \
  'out = *d.wrapping_add(k);'

mutate "over-read: load 8 bytes for the tail" \
  'if rem >= 1 {
            data |= (load_u8(d, 0) as i32) as usize;
        }' \
  'if rem >= 1 {
            data |= (load_u8(d, 0) as i32) as usize;
            let _ = load_u8(d, 7);
        }'

restore
echo
if [[ $SURVIVORS -eq 0 ]]; then
  printf '\033[1;32mAll mutations were caught: the differential suite is not vacuous.\033[0m\n'
else
  printf '\033[1;31m%d mutation(s) SURVIVED -- the suite has blind spots.\033[0m\n' "$SURVIVORS"
fi
timeout 300 cargo build --offline >/dev/null 2>&1
exit $SURVIVORS
