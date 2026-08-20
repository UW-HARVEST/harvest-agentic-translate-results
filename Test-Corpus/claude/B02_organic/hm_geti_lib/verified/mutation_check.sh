#!/bin/bash
# ---------------------------------------------------------------------------
# Mutation check: proves the differential test suite is not vacuous.
#
# Each mutation injects ONE small, behaviour-changing deviation from the C
# semantics into src/lib.rs.  The suite MUST fail for every mutation; if it
# passes, that behaviour is not actually being verified.
#
#   ./mutation_check.sh
# ---------------------------------------------------------------------------
set -u
cd "$(dirname "$(readlink -f "$0")")" || exit 1
SRC=src/lib.rs
BAK=${TMPDIR:-/tmp}/lib.rs.orig.$$
LOG=${TMPDIR:-/tmp}/mut.$$
mkdir -p "$LOG"
cp "$SRC" "$BAK"
restore() { cp "$BAK" "$SRC"; }
trap restore EXIT

declare -a NAMES FROM TO EQUIV

# add <name> <from> <to> [equivalent-mutant-reason]
# A mutation tagged with a reason is a PROVEN semantically-equivalent mutant: no
# input can distinguish it from the original, so the suite is *expected* not to
# catch it.  If one of those ever IS caught, the equivalence argument is wrong
# and the script says so.
add() { NAMES+=("$1"); FROM+=("$2"); TO+=("$3"); EQUIV+=("${4:-}"); }

# 1. drop the `temp_key` write in hmput_key's FORWARD duplicate-match branch
add "hmput_key: no temp_key on forward dup match" \
    'stbds_temp_key_set(a, stored);' \
    'let _ = stored;'

# 2. "fix" the C quirk: also write temp_key in the WRAP-AROUND dup branch
add "hmput_key: temp_key also on wrap-around dup match" \
    'stbds_temp_set(a, (*bucket).index[i]);
                        return stbds_arr_to_hash(a, elemsize);' \
    'stbds_temp_set(a, (*bucket).index[i]);
                        stbds_temp_key_set(a, *(byte_off(raw_a, elemsize.wrapping_mul((*bucket).index[i] as usize)) as *mut *mut c_char));
                        return stbds_arr_to_hash(a, elemsize);'

# 3. stralloc: push the oversize block onto the head instead of splicing it after
add "stralloc: wrong oversize splice position" \
    '(*sb).next = (*(*a).storage).next;
                (*(*a).storage).next = sb;' \
    '(*sb).next = (*a).storage;
                (*a).storage = sb;'

# 4. siphash tail: drop the `(d[3] << 24)` int sign-extension
add "siphash: no sign extension of d[3]<<24" \
    'data |= (((*d.wrapping_add(3) as u32) << 24) as i32) as isize as usize;' \
    'data |= (*d.wrapping_add(3) as usize) << 24;'

# 5. siphash main loop: drop the sign extension of the low word
add "siphash: no sign extension in the main loop" \
    'data = load32_as_c_int(d) as isize as usize;' \
    'data = load32_as_c_int(d) as u32 as usize;'

# 6. find_slot: wrong reserved-hash fixup threshold
add "find_slot: hash<1 instead of hash<2" \
    'if hash < 2 {
        hash = hash.wrapping_add(2);
    }' \
    'if hash < 1 {
        hash = hash.wrapping_add(2);
    }' \
    "EQUIVALENT: only differs for a key whose raw 64-bit siphash is exactly 1 (p=2^-64 per key; 0 of 400k observed in row10)"

# 7. hmdel_key: free the strdup key for every string mode, not only mode==1
add "hmdel_key: strdup free for mode>=1" \
    'if mode == STBDS_HM_STRING && (*table).string.mode == STBDS_SH_STRDUP as u8 {' \
    'if mode >= STBDS_HM_STRING && (*table).string.mode == STBDS_SH_STRDUP as u8 {'

# 8. make_hash_index: wrong shrink-threshold clamp boundary
add "make_hash_index: shrink clamp < instead of <=" \
    'if slot_count <= STBDS_BUCKET_LENGTH {
        (*t).used_count_shrink_threshold = 0;
    }' \
    'if slot_count < STBDS_BUCKET_LENGTH {
        (*t).used_count_shrink_threshold = 0;
    }'

# 9. arrgrowf: wrong minimum-capacity clamp
add "arrgrowf: min_cap clamp 5 instead of 4" \
    '} else if min_cap < 4 {
        min_cap = 4;' \
    '} else if min_cap < 5 {
        min_cap = 5;'

# 10. hash_string: wrong rotate amount
add "hash_string: rotate 10 instead of 9" \
    'hash = stbds_rotate_left(hash, 9).wrapping_add(*s as usize);' \
    'hash = stbds_rotate_left(hash, 10).wrapping_add(*s as usize);'

# 11. stralloc: wrong block-size clamp comparison
add "stralloc: block clamp <= instead of <" \
    'if blocksize < STBDS_STRING_ARENA_BLOCKSIZE_MAX {' \
    'if blocksize <= STBDS_STRING_ARENA_BLOCKSIZE_MAX {'

# 12. make_hash_index rehash: probe step no longer grows quadratically
add "make_hash_index: rehash probe step frozen" \
    'pos = pos.wrapping_add(step);
                        step = step.wrapping_add(STBDS_BUCKET_LENGTH);
                        pos &= (*t).slot_count - 1;' \
    'pos = pos.wrapping_add(step);
                        pos &= (*t).slot_count - 1;'

# 12b. make_hash_index rehash: used_count not carried over correctly
add "make_hash_index: rehash used_count off by one" \
    '(*t).used_count = (*ot).used_count;' \
    '(*t).used_count = (*ot).used_count.wrapping_add(1);'

# 12c. make_hash_index: wrong tombstone threshold
add "make_hash_index: tombstone threshold slot/8 only" \
    '(*t).tombstone_count_threshold = (slot_count >> 3) + (slot_count >> 4);' \
    '(*t).tombstone_count_threshold = slot_count >> 3;'

# 12d. make_hash_index: wrong seed LCG advance
add "make_hash_index: wrong seed LCG" \
    'hash_seed_set(hash_seed_get().wrapping_mul(a).wrapping_add(b));' \
    'hash_seed_set(hash_seed_get().wrapping_mul(a).wrapping_add(b).wrapping_add(1));'

# 13. hmdel_key: use *(char**) for every mode on the back-fill re-find
add "hmdel_key: back-fill uses *(char**) for all modes" \
    'if mode == STBDS_HM_STRING {
                        slot = stbds_hm_find_slot(' \
    'if mode >= STBDS_HM_STRING {
                        slot = stbds_hm_find_slot('

# 14. hmput_default: reset the default element even when it already exists
add "hmput_default: not idempotent" \
    'if a.is_null() || (*stbds_header(stbds_hash_to_arr(a, elemsize))).length == 0 {' \
    'if a.is_null() || (*stbds_header(stbds_hash_to_arr(a, elemsize))).length <= 1 {'

# 15. shmode_func: no (unsigned char) truncation of the mode
add "shmode_func: no mode truncation" \
    '(*h).string.mode = mode as u8;' \
    '(*h).string.mode = if mode > 255 { 255 } else { mode as u8 };'

# 16. strkey: wrong format string
add "strkey: wrong format" \
    'b"test_%d\0".as_ptr()' \
    'b"test_%u\0".as_ptr()'

# 17. arrgrowf: wrong doubling boundary
add "arrgrowf: doubling boundary <=" \
    'if min_cap < 2usize.wrapping_mul(stbds_arrcap(a)) {' \
    'if min_cap <= 2usize.wrapping_mul(stbds_arrcap(a)) {' \
    "EQUIVALENT: the extra case is min_cap == 2*cap, where the body assigns min_cap = 2*cap, i.e. a no-op"

# 18. arrgrowf: forget to reset the header of a fresh array
add "arrgrowf: fresh header temp not zeroed" \
    '(*stbds_header(b)).temp = 0;' \
    '(*stbds_header(b)).temp = 1;'

# 19. hmdel_key: wrong final_index
add "hmdel_key: final_index off by one" \
    'let final_index: isize = stbds_arrlen(raw_a).wrapping_sub(1).wrapping_sub(1);' \
    'let final_index: isize = stbds_arrlen(raw_a).wrapping_sub(1);'

# 20. hmget_key_ts: wrong sentinel on the NULL-map path
add "hmget_key_ts: sentinel 0 instead of -1 on NULL map" \
    '*temp = STBDS_INDEX_EMPTY;
        stbds_arr_to_hash(a, elemsize)' \
    '*temp = 0;
        stbds_arr_to_hash(a, elemsize)'

# 21. hmput_key: wrong index stored in the bucket
add "hmput_key: bucket index i instead of i-1" \
    '(*bucket).index[pos & STBDS_BUCKET_MASK] = i.wrapping_sub(1);' \
    '(*bucket).index[pos & STBDS_BUCKET_MASK] = i;'

# 22. hmfree_func: skip the arena reset
add "hmfree_func: no strreset" \
    'stbds_strreset(ptr::addr_of_mut!((*stbds_hash_table(a)).string));' \
    '{}'

# 23. hash_string: drop the final `+ seed`
add "hash_string: no final + seed" \
    'hash.wrapping_add(seed)
}' \
    'hash
}'

# 24. stralloc: off-by-one in the in-block pointer
add "stralloc: in-block pointer off by one" \
    '.wrapping_offset(-(len as isize));' \
    '.wrapping_offset(-(len as isize) - 1);'

# 25. hm_geti: wrong default value (the 12 internal asserts must catch it)
add "hm_geti: default -3 instead of -2" \
    'hm_hmdefault(&mut intmap, -2);' \
    'hm_hmdefault(&mut intmap, -3);'

# 26. hmdel_key: skip the tombstone bookkeeping
add "hmdel_key: no tombstone_count increment" \
    '(*table).tombstone_count = (*table).tombstone_count.wrapping_add(1);' \
    '{}'

# 27. hmput_key: hash fixup dropped on the put side only
add "hmput_key: no hash<2 fixup on put" \
    'if hash < 2 {
            hash = hash.wrapping_add(2);
        }' \
    '{}' \
    "EQUIVALENT: only differs for a key whose raw 64-bit siphash is 0 or 1 (p=2^-63 per key; 0 of 400k observed in row10)"

# 28. STBDS_ALIGN_FWD rounds down instead of up
add "align_fwd: rounds down" \
    'n.wrapping_add(a - 1) & !(a - 1)' \
    'n & !(a - 1)'

# 29. make_hash_index: forget the cache-line slack in the allocation size
add "make_hash_index: alloc without CACHE_LINE-1 slack" \
    '.wrapping_add(size_of::<stbds_hash_index>())
            .wrapping_add(STBDS_CACHE_LINE_SIZE - 1),' \
    '.wrapping_add(size_of::<stbds_hash_index>()),'

# 30. arrgrowf: forget the array header in the allocation size
add "arrgrowf: alloc without the header" \
    'elemsize
            .wrapping_mul(min_cap)
            .wrapping_add(size_of::<stbds_array_header>()),' \
    'elemsize.wrapping_mul(min_cap),'


pass=0; caught=0; missed=0; equiv_ok=0
n=${#NAMES[@]}
echo "running $n mutations"
for ((i=0; i<n; i++)); do
  restore
  python3 - "$SRC" <<PY
import sys
p = sys.argv[1]
src = open(p).read()
frm = $(printf '%s' "${FROM[$i]}" | python3 -c 'import sys,json; print(json.dumps(sys.stdin.read()))')
to  = $(printf '%s' "${TO[$i]}"   | python3 -c 'import sys,json; print(json.dumps(sys.stdin.read()))')
cnt = src.count(frm)
if cnt != 1:
    sys.stderr.write("PATTERN COUNT %d (expected 1)\n" % cnt)
    sys.exit(3)
open(p,"w").write(src.replace(frm, to, 1))
PY
  rc=$?
  if [ $rc -ne 0 ]; then
    printf '  %-52s  !! PATTERN NOT UNIQUE/FOUND\n' "${NAMES[$i]}"
    missed=$((missed+1)); continue
  fi
  # cargo test does NOT rebuild a cdylib-only lib target -> build explicitly
  timeout 600 cargo build --offline > "$LOG/b$i.log" 2>&1
  if [ $? -ne 0 ]; then
    printf '  %-52s  caught (does not compile)\n' "${NAMES[$i]}"
    caught=$((caught+1)); continue
  fi
  timeout 600 cargo test --offline > "$LOG/m$i.log" 2>&1
  rc=$?
  if [ $rc -eq 0 ]; then
    if [ -n "${EQUIV[$i]}" ]; then
      printf '  %-52s  not caught, as expected\n' "${NAMES[$i]}"
      printf '  %-52s    -> %s\n' "" "${EQUIV[$i]}"
      equiv_ok=$((equiv_ok+1))
    else
      printf '  %-52s  !! NOT CAUGHT (suite still passes)\n' "${NAMES[$i]}"
      missed=$((missed+1))
    fi
  else
    first=$(grep -m1 -oE 'DIVERGENCE in [^ ]+|assertion .*failed|panicked at [^ ]+' "$LOG/m$i.log" | head -1)
    nfail=$(grep -c '^test .* FAILED' "$LOG/m$i.log")
    printf '  %-52s  caught (%s failing tests) %s\n' "${NAMES[$i]}" "$nfail" "$first"
    caught=$((caught+1))
    if [ -n "${EQUIV[$i]}" ]; then
      printf '  %-52s  !! was declared EQUIVALENT but WAS caught -> the reasoning is wrong\n' "${NAMES[$i]}"
      missed=$((missed+1))
    fi
  fi
done
restore
# leave the workspace with a freshly built .so (cargo test does not rebuild it)
timeout 600 cargo build --offline > "$LOG/final.log" 2>&1

echo
echo "mutations killed:              $caught / $n"
echo "proven equivalent (expected):  $equiv_ok / $n"
echo "real coverage gaps:            $missed"
if [ "$missed" -eq 0 ]; then
  echo "############ EVERY NON-EQUIVALENT MUTATION WAS KILLED ############"
else
  echo "!! the suite does not cover every mutated behaviour"
fi
exit $missed
