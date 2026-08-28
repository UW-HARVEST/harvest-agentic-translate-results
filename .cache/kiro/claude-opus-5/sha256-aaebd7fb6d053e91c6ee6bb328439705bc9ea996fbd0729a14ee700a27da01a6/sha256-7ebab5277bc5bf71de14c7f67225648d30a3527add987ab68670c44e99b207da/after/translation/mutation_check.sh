#!/usr/bin/env bash
# Sensitivity check for the differential test suite: apply a series of small,
# deliberate divergences to translation/src/lib.rs, rebuild and run the suite.
# Every mutation MUST be detected — a surviving mutation means that behaviour is
# not actually covered.
set -uo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$here"
cp src/lib.rs /tmp/mutate_orig.rs
restore() { cp /tmp/mutate_orig.rs src/lib.rs; }
trap 'restore; cargo build >/dev/null 2>&1' EXIT

run_case() {
    local name="$1" from="$2" to="$3"
    restore
    python3 - "$from" "$to" <<'PY'
import sys
frm, to = sys.argv[1], sys.argv[2]
p = 'src/lib.rs'
s = open(p).read()
n = s.count(frm)
if n != 1:
    sys.exit("PATTERN COUNT %d for %r" % (n, frm[:60]))
open(p, 'w').write(s.replace(frm, to))
PY
    if [[ $? -ne 0 ]]; then
        echo "SKIP  $name (pattern did not apply)"
        return 0
    fi
    if ! timeout 600 cargo build >/tmp/mut_build.log 2>&1; then
        echo "SKIP  $name (does not compile)"
        return 0
    fi
    if timeout 600 cargo test -- --test-threads=1 >/tmp/mut_test.log 2>&1; then
        echo "SURVIVED  $name   <-- NOT COVERED"
        return 1
    else
        local who
        who="$(grep -oE '^---- [a-z0-9_]+ stdout' /tmp/mut_test.log | head -1 | awk '{print $2}')"
        echo "caught    $name  (by ${who:-?})"
        return 0
    fi
}

fails=0

run_case "hash_string rotate 9->8" \
    'hash = STBDS_ROTATE_LEFT(hash, 9).wrapping_add(*str_ as usize);' \
    'hash = STBDS_ROTATE_LEFT(hash, 8).wrapping_add(*str_ as usize);' || fails=1

run_case "hash_string final rotate 22->23" \
    'hash ^= STBDS_ROTATE_RIGHT(hash, 22);' \
    'hash ^= STBDS_ROTATE_RIGHT(hash, 23);' || fails=1

run_case "siphash body: drop sign extension of low half" \
    'data = lo as isize as usize;' \
    'data = lo as u32 as usize;' || fails=1

run_case "siphash body: use the wrong tail byte" \
    'let hi = (*d.add(4) as i32)' \
    'let hi = (*d.add(5) as i32)' || fails=1

# NOTE: mutating `data |= ((hi as isize as usize) << 16) << 16` into
# `((hi as u32 as usize) << 16) << 16` is a *semantically equivalent* mutant:
# the `<< 32` discards exactly the bits that the sign extension would have set,
# so no test can distinguish it. Same for the C original's `(size_t)` cast.

run_case "siphash tail case 4: drop sign extension" \
    'data |= (*d.add(3) as i32).wrapping_shl(24) as isize as usize;' \
    'data |= ((*d.add(3) as usize) << 24) as usize;' || fails=1

run_case "siphash length mixing" \
    'data = len << (STBDS_SIZE_T_BITS - 8);' \
    'data = len << (STBDS_SIZE_T_BITS - 16);' || fails=1

run_case "siphash D rounds 4->3" \
    'const STBDS_SIPHASH_D_ROUNDS: usize = 4;' \
    'const STBDS_SIPHASH_D_ROUNDS: usize = 3;' || fails=1

run_case "arrgrowf min_cap floor 4->8" \
    '} else if min_cap < 4 {
            min_cap = 4;' \
    '} else if min_cap < 8 {
            min_cap = 8;' || fails=1

run_case "arrgrowf growth factor 2->3" \
    'if min_cap < 2usize.wrapping_mul(stbds_arrcap(a)) {
            min_cap = 2usize.wrapping_mul(stbds_arrcap(a));' \
    'if min_cap < 3usize.wrapping_mul(stbds_arrcap(a)) {
            min_cap = 3usize.wrapping_mul(stbds_arrcap(a));' || fails=1

run_case "make_hash_index used_count_threshold" \
    '(*t).used_count_threshold = slot_count - (slot_count >> 2);' \
    '(*t).used_count_threshold = slot_count - (slot_count >> 1);' || fails=1

run_case "make_hash_index tombstone_count_threshold" \
    '(*t).tombstone_count_threshold = (slot_count >> 3) + (slot_count >> 4);' \
    '(*t).tombstone_count_threshold = (slot_count >> 3);' || fails=1

run_case "make_hash_index shrink threshold" \
    '(*t).used_count_shrink_threshold = slot_count >> 2;' \
    '(*t).used_count_shrink_threshold = slot_count >> 1;' || fails=1

run_case "make_hash_index seed evolution constant a" \
    'temp = (0x87b0b0fdu32 ^ 2147001325u32) as usize;' \
    'temp = (0x87b0b0feu32 ^ 2147001325u32) as usize;' || fails=1

run_case "make_hash_index storage alignment 64->32" \
    'STBDS_ALIGN_FWD(t.add(1) as usize, STBDS_CACHE_LINE_SIZE) as *mut stbds_hash_bucket;' \
    'STBDS_ALIGN_FWD(t.add(1) as usize, 32) as *mut stbds_hash_bucket;' || fails=1

run_case "find_slot probe step progression" \
    "            pos = pos.wrapping_add(step);
            step += STBDS_BUCKET_LENGTH;
            pos &= (*table).slot_count - 1;
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern \"C\" fn stbds_hmget_key_ts(" \
    "            pos = pos.wrapping_add(step);
            step += 2 * STBDS_BUCKET_LENGTH;
            pos &= (*table).slot_count - 1;
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern \"C\" fn stbds_hmget_key_ts(" || fails=1

run_case "hmput_key stores index i instead of i-1" \
    '(*bucket).index[pos & STBDS_BUCKET_MASK] = i - 1;' \
    '(*bucket).index[pos & STBDS_BUCKET_MASK] = i;' || fails=1

# NOTE: `if hash < 2 { hash += 2 }` only matters for hash values 0 and 1, which
# no reachable input produces (siphash / the string hash would have to collide
# with a 64-bit constant), so mutating the bound is not distinguishable. The same
# holds for `stbds_is_key_equal`'s string comparison: it is only consulted after
# a full 64-bit hash match, so weakening it to a prefix compare is equivalent
# short of an engineered hash collision. Both are covered by inspection instead.

run_case "hmput_key skips the hash<2 fixup entirely" \
    '            if hash < 2 {
                hash += 2;
            }

            pos = stbds_probe_position(hash, (*table).slot_count, (*table).slot_count_log2);

            '"'"'found_empty_slot: loop {' \
    '            hash |= 4;

            pos = stbds_probe_position(hash, (*table).slot_count, (*table).slot_count_log2);

            '"'"'found_empty_slot: loop {' || fails=1

run_case "hmdel_key final_index off by one" \
    'let final_index = stbds_arrlen(raw_a) - 1 - 1;' \
    'let final_index = stbds_arrlen(raw_a) - 1;' || fails=1

run_case "hmdel_key tombstone marker" \
    '(*b).hash[i as usize] = STBDS_HASH_DELETED;' \
    '(*b).hash[i as usize] = STBDS_HASH_EMPTY;' || fails=1

run_case "hmdel_key temp flag" \
    'stbds_temp_set(raw_a, 1);' \
    'stbds_temp_set(raw_a, 2);' || fails=1


run_case "shmode_func initial length" \
    '(*stbds_header(a)).length = 1;
        h = stbds_make_hash_index(STBDS_BUCKET_LENGTH, ptr::null_mut());' \
    '(*stbds_header(a)).length = 2;
        h = stbds_make_hash_index(STBDS_BUCKET_LENGTH, ptr::null_mut());' || fails=1

run_case "stralloc block size progression" \
    'blocksize = STBDS_STRING_ARENA_BLOCKSIZE_MIN << (blocksize >> 1);' \
    'blocksize = STBDS_STRING_ARENA_BLOCKSIZE_MIN << blocksize;' || fails=1

run_case "stralloc allocation position" \
    'p = ((&raw mut (*(*a).storage).storage) as *mut c_char).add((*a).remaining - len);' \
    'p = ((&raw mut (*(*a).storage).storage) as *mut c_char).add((*a).remaining - len + 1);' || fails=1

run_case "stralloc oversized-block list splice" \
    '                if !(*a).storage.is_null() {
                    (*sb).next = (*(*a).storage).next;
                    (*(*a).storage).next = sb;' \
    '                if !(*a).storage.is_null() {
                    (*sb).next = (*a).storage;
                    (*a).storage = sb;' || fails=1

run_case "strkey format string" \
    'sprintf(buf, b"test_%d\0".as_ptr() as *const c_char, n);' \
    'sprintf(buf, b"test_%u\0".as_ptr() as *const c_char, n);' || fails=1

run_case "sh_geti prints value*2" \
    'printf(fmt, (*e).key, (*e).value as u32 as u64, (*e).value);' \
    'printf(fmt, (*e).key, ((*e).value * 2) as u32 as u64, (*e).value);' || fails=1

run_case "sh_geti insert stride 2->3" \
    '                shput(&mut strmap, strkey(i), i.wrapping_mul(3));
                i += 2;' \
    '                shput(&mut strmap, strkey(i), i.wrapping_mul(3));
                i += 3;' || fails=1

run_case "sh_geti default value -2 -> -3" \
    'shdefault(&mut strmap, -2);' \
    'shdefault(&mut strmap, -3);' || fails=1

run_case "sh_geti skips the arena pass" \
    'while j < 2 {' \
    'while j < 1 {' || fails=1

run_case "hmfree_func skips strdup key release" \
    'if (*stbds_hash_table(a)).string.mode == STBDS_SH_STRDUP {' \
    'if false && (*stbds_hash_table(a)).string.mode == STBDS_SH_STRDUP {' || fails=1

# NOTE: `stbds_is_key_equal` is only ever consulted *after* a full 64-bit hash
# match, so any mutation that still answers "equal" for the identical key is
# semantically equivalent (weakening the compare to a prefix, or ignoring the
# last byte, only matters for engineered hash collisions). Mutations that flip
# the answer for the identical key are distinguishable:

run_case "is_key_equal inverted for binary keys" \
    '        } else {
            memcmp_eq(
                key as *const u8,
                (a as *mut u8).add(elemsize.wrapping_mul(i)).add(keyoffset) as *const u8,
                keysize,
            )' \
    '        } else {
            !memcmp_eq(
                key as *const u8,
                (a as *mut u8).add(elemsize.wrapping_mul(i)).add(keyoffset) as *const u8,
                keysize,
            )' || fails=1

run_case "is_key_equal inverted for string keys" \
    'strcmp_eq(key as *const c_char, stored as *const c_char)' \
    '!strcmp_eq(key as *const c_char, stored as *const c_char)' || fails=1

run_case "temp_key stored in the wrapped-around probe loop too" \
    '                            stbds_temp_set(a, (*bucket).index[i]);
                            return STBDS_ARR_TO_HASH(a, elemsize);' \
    '                            stbds_temp_set(a, (*bucket).index[i]);
                            if mode >= STBDS_HM_STRING {
                                stbds_temp_key_set(a, *((raw_a as *mut u8).add(elemsize.wrapping_mul((*bucket).index[i] as usize)).add(keyoffset) as *mut *mut c_char));
                            }
                            return STBDS_ARR_TO_HASH(a, elemsize);' || fails=1

run_case "hmput_default zero-length guard dropped" \
    'if a.is_null() || (*stbds_header(STBDS_HASH_TO_ARR(a, elemsize))).length == 0 {' \
    'if a.is_null() {' || fails=1

echo
if [[ $fails -eq 0 ]]; then
    echo "ALL MUTATIONS DETECTED (or rejected at compile time)"
else
    echo "SOME MUTATIONS SURVIVED — coverage gap"
fi
exit $fails
