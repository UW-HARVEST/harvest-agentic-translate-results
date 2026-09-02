#!/usr/bin/env bash
# Sanity check that the differential suite actually discriminates: inject a
# small behavioural mutation into the Rust translation, confirm the suite FAILS,
# then restore. A mutation that the suite does not notice is a coverage hole.
set -u
cd "$(dirname "$0")/.."

SRC=src/lib.rs
BAK=$(mktemp)
cp "$SRC" "$BAK"
restore() { cp "$BAK" "$SRC"; cargo build --release >/dev/null 2>&1; rm -f "$BAK"; }
trap restore EXIT

declare -a NAMES=(
  "hash_string rotate constant"
  "siphash tail sign-extension"
  "tombstone threshold formula"
  "bucket index off-by-one"
  "temp_key also set in wrap-around half"
  "arrgrowf min capacity 4 -> 5"
  "shrink threshold guard"
  "strkey digit encoding"
  "stralloc block growth"
  "hmput_default length check"
)
declare -a FROM=(
  'hash ^= rotate_right(hash, 22);'
  'data |= ((*d.add(3) as i32).wrapping_shl(24)) as isize as usize;'
  '(*t).tombstone_count_threshold = (slot_count >> 3) + (slot_count >> 4);'
  '(*bucket).index[pos & STBDS_BUCKET_MASK] = i - 1;'
  '                            stbds_temp_set(a, (*bucket).index[i]);
                            return arr_to_hash(a, elemsize);'
  'min_cap = 4;'
  'if slot_count <= STBDS_BUCKET_LENGTH {'
  'digits[ndigits] = b'"'"'0'"'"' + (v % 10) as u8;'
  'blocksize = STBDS_STRING_ARENA_BLOCKSIZE_MIN.wrapping_shl((blocksize >> 1) as u32);'
  'if a.is_null() || (*stbds_header(hash_to_arr(a, elemsize))).length == 0 {'
)
declare -a TO=(
  'hash ^= rotate_right(hash, 23);'
  'data |= (*d.add(3) as usize) << 24;'
  '(*t).tombstone_count_threshold = (slot_count >> 3);'
  '(*bucket).index[pos & STBDS_BUCKET_MASK] = i;'
  '                            stbds_temp_set(a, (*bucket).index[i]);
                            if mode >= STBDS_HM_STRING {
                                let src = *(elem_ptr(raw_a, elemsize, (*bucket).index[i] as usize).add(keyoffset) as *mut *mut c_char);
                                stbds_temp_key_set(a, src);
                            }
                            return arr_to_hash(a, elemsize);'
  'min_cap = 5;'
  'if slot_count < STBDS_BUCKET_LENGTH {'
  'digits[ndigits] = b'"'"'0'"'"' + (v % 9) as u8;'
  'blocksize = STBDS_STRING_ARENA_BLOCKSIZE_MIN.wrapping_shl(((blocksize >> 1) + 1) as u32);'
  'if a.is_null() {'
)

pass=0
fail=0
for i in "${!NAMES[@]}"; do
  cp "$BAK" "$SRC"
  python3 - "$SRC" "${FROM[$i]}" "${TO[$i]}" <<'PY'
import sys
path, frm, to = sys.argv[1], sys.argv[2], sys.argv[3]
s = open(path).read()
if frm not in s:
    sys.exit(3)
open(path, 'w').write(s.replace(frm, to, 1))
PY
  rc=$?
  if [ $rc -eq 3 ]; then
    echo "SKIP  ${NAMES[$i]} (pattern not found)"
    continue
  fi
  if ! cargo build --release >/dev/null 2>&1; then
    echo "SKIP  ${NAMES[$i]} (mutant does not compile)"
    continue
  fi
  if timeout 600 cargo test --release --tests >/dev/null 2>&1; then
    echo "MISSED  ${NAMES[$i]} — suite still passed!"
    fail=$((fail+1))
  else
    echo "CAUGHT  ${NAMES[$i]}"
    pass=$((pass+1))
  fi
done

echo
echo "caught $pass / $((pass+fail)) mutants"
[ "$fail" -eq 0 ]
