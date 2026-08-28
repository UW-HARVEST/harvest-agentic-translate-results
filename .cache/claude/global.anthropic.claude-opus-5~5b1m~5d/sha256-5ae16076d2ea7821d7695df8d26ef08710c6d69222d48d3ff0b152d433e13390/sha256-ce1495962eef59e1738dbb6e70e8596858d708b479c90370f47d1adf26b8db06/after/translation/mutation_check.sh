#!/usr/bin/env bash
# Negative control for the differential suite.
#
# Injects a series of small, behaviour-changing edits into `src/lib.rs`, builds
# each as a separate `.so`, and checks that the differential tests DETECT each
# one.  A mutant that survives means the suite has a blind spot.
#
# `src/lib.rs` is backed up and restored; the script refuses to run if the
# working tree is already dirty from a previous interrupted run.
set -uo pipefail
cd "$(cd "$(dirname "$0")" && pwd)"

BAK=target/lib.rs.orig
MUT=target/mutants
mkdir -p "$MUT"
[ -f "$BAK" ] && { echo "$BAK already exists -- a previous run was interrupted."; exit 1; }
cp src/lib.rs "$BAK"
trap 'cp "$BAK" src/lib.rs; rm -f "$BAK"; cargo build --release -q' EXIT

C_SO="$(ls ../c_src/build/lib*.so | head -1)"

build_mutant() {
  local name="$1" old="$2" new="$3"
  cp "$BAK" src/lib.rs
  python3 - "$old" "$new" <<'PY' || return 1
import sys
old, new = sys.argv[1], sys.argv[2]
s = open('src/lib.rs').read()
assert s.count(old) >= 1, "pattern not found: %r" % old
open('src/lib.rs', 'w').write(s.replace(old, new, 1))
PY
  cargo build --release -q 2>/dev/null || return 1
  cp target/release/libarr_push_lib.so "$MUT/$name.so"
  cp "$BAK" src/lib.rs
}

KILLED=0; SURVIVED=0; EQUIV=0
check() {
  local name="$1" note="${2:-}"
  local hits
  hits=$(C_SO="$C_SO" RUST_SO="$PWD/$MUT/$name.so" \
         timeout 600 cargo test --release -- --test-threads=1 2>&1 \
         | grep -cE '^test .* FAILED|SIGABRT|signal: 6')
  if [ "$hits" -gt 0 ]; then
    printf '  KILLED   %-24s (%s failing tests)\n' "$name" "$hits"; KILLED=$((KILLED+1))
  elif [ -n "$note" ]; then
    printf '  EQUIV    %-24s %s\n' "$name" "$note"; EQUIV=$((EQUIV+1))
  else
    printf '  SURVIVED %-24s <-- BLIND SPOT\n' "$name"; SURVIVED=$((SURVIVED+1))
  fi
}

echo "building mutants..."
build_mutant m2_mode_dispatch  'if mode >= STBDS_HM_STRING {
        let other' 'if mode > STBDS_HM_STRING {
        let other'
build_mutant m4_shrink_floor   'if slot_count <= STBDS_BUCKET_LENGTH {' 'if slot_count < STBDS_BUCKET_LENGTH {'
build_mutant m5_tempkey_wrap   'set_stbds_temp(a, (*bucket).index[i]);
                        return stbds_arr_to_hash(a, elemsize);' 'set_stbds_temp(a, (*bucket).index[i]);
                        set_stbds_temp_key(a, *(elem_ptr(raw_a, elemsize, (*bucket).index[i] as usize, keyoffset) as *mut *mut c_char));
                        return stbds_arr_to_hash(a, elemsize);'
build_mutant m6_strkey_sign    'if val < 0 {' 'if val < -1 {'
build_mutant m7_arena_block    'if blocksize < STBDS_STRING_ARENA_BLOCKSIZE_MAX {' 'if blocksize <= STBDS_STRING_ARENA_BLOCKSIZE_MAX {'
build_mutant m8_hash_lt2       'if hash < 2 {
            hash = hash.wrapping_add(2);' 'if hash < 1 {
            hash = hash.wrapping_add(2);'
build_mutant m1b_growth_clamp  'else if min_cap < 4 {
        min_cap = 4;' 'else if min_cap < 4 {
        min_cap = 5;'
build_mutant m3b_siphash_tail  'data |= ((*d.add(5) as usize) << 20) << 20;' 'data |= ((*d.add(5) as usize) << 20) << 21;'
build_mutant m9_use_threshold  '(*t).used_count_threshold = slot_count.wrapping_sub(slot_count >> 2);' '(*t).used_count_threshold = slot_count.wrapping_sub(slot_count >> 3);'
build_mutant m10_final_index   'let final_index: isize = stbds_arrlen(raw_a).wrapping_sub(1).wrapping_sub(1);' 'let final_index: isize = stbds_arrlen(raw_a).wrapping_sub(1);'
build_mutant m11_align         'STBDS_CACHE_LINE_SIZE,
    ) as *mut stbds_hash_bucket;' '32,
    ) as *mut stbds_hash_bucket;'
build_mutant m12_rot           'hash = stbds_rotate_left(hash, 9).wrapping_add(*str_ as usize);' 'hash = stbds_rotate_left(hash, 8).wrapping_add(*str_ as usize);'
build_mutant m13_remaining     '(*a).remaining = blocksize;' '(*a).remaining = blocksize - 1;'
build_mutant m14_default_cond  'if a.is_null() || (*stbds_header(stbds_hash_to_arr(a, elemsize))).length == 0 {' 'if a.is_null() || (*stbds_header(stbds_hash_to_arr(a, elemsize))).length <= 1 {'
build_mutant m15_del_temp      'set_stbds_temp(raw_a, 1);' 'set_stbds_temp(raw_a, 2);'
build_mutant m16_tombstone_thr '(*t).tombstone_count_threshold = (slot_count >> 3).wrapping_add(slot_count >> 4);' '(*t).tombstone_count_threshold = (slot_count >> 3).wrapping_add(slot_count >> 5);'
build_mutant m17_seed_lcg      'let mut a: usize = 0x27bb2ee6usize;' 'let mut a: usize = 0x27bb2ee7usize;'
build_mutant m18_hash_final    'hash.wrapping_add(seed)
}' 'hash.wrapping_sub(seed)
}'
# Provably equivalent mutants, kept as a control on the control:
build_mutant m1_equiv_clamp    'else if min_cap < 4 {' 'else if min_cap < 5 {'
build_mutant m3_equiv_shift    'data |= ((*d.add(5) as usize) << 20) << 20;' 'data |= ((*d.add(5) as usize) << 21) << 19;'

cp "$BAK" src/lib.rs
cargo build --release -q

echo
echo "running the suite against each mutant..."
for n in m2_mode_dispatch m4_shrink_floor m5_tempkey_wrap m6_strkey_sign \
         m7_arena_block m8_hash_lt2 m1b_growth_clamp m3b_siphash_tail \
         m9_use_threshold m10_final_index m11_align m12_rot m13_remaining \
         m14_default_cond m15_del_temp m16_tombstone_thr m17_seed_lcg \
         m18_hash_final; do
  check "$n"
done
# these two are semantically identical to the original, so surviving is correct
check m1_equiv_clamp  '(min_cap<5 then =4 is a no-op when min_cap==4)'
check m3_equiv_shift  '((x<<20)<<20 == (x<<21)<<19 for x <= 255)'

echo
echo "killed=$KILLED  survived=$SURVIVED  provably-equivalent=$EQUIV"
[ "$SURVIVED" -eq 0 ] || exit 1
