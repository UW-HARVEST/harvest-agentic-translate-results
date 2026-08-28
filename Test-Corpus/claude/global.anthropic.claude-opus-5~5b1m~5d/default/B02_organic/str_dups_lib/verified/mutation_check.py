#!/usr/bin/env python3
"""Sensitivity check for the differential suite.

Injects one small, C-semantics-breaking change into `src/lib.rs` at a time,
REBUILDS the cdylib (mandatory: `cargo test` alone does NOT rebuild a
cdylib-only lib target - that is what `assert_fresh()` in tests/common/mod.rs
now guards against), reruns the suite and records whether it was caught.

A mutation that survives means the suite has a blind spot there.

Run:  cd translation && python3 mutation_check.py
"""
import atexit
import os
import re
import subprocess
import sys

SRC = "src/lib.rs"
PRISTINE = "src/lib.rs.pristine"

ORIG = open(SRC).read()
# Keep a pristine copy on disk so an interrupted run can still be recovered.
open(PRISTINE, "w").write(ORIG)


def restore():
    open(SRC, "w").write(ORIG)
    if os.path.exists(PRISTINE):
        os.remove(PRISTINE)
    subprocess.run(["cargo", "build", "--offline"], capture_output=True, timeout=900)


atexit.register(restore)

# (name, original snippet, mutated snippet, needs_the_slow_fork_tests)
MUTATIONS = [
    ("siphash tail: drop the sign-extension of d[3]<<24",
     'data |= ((*d.add(3) as i32).wrapping_shl(24)) as i64 as u64 as usize;',
     'data |= ((*d.add(3) as usize) << 24) as usize;', False),
    ("siphash main loop: drop the sign-extension of the low int",
     'data = lo as i64 as u64 as usize;',
     'data = lo as u32 as usize;', False),
    ("siphash: C_ROUNDS 2 -> 1 for the tail block only",
     '    v0 ^= data;\n    v2 ^= 0xff;',
     '    v0 ^= data;\n    v1 ^= 1;\n    v2 ^= 0xff;', False),
    ("hash_string: rotate 9 -> 8",
     'hash = stbds_rotate_left(hash, 9).wrapping_add(*(s as *const u8) as usize);',
     'hash = stbds_rotate_left(hash, 8).wrapping_add(*(s as *const u8) as usize);', False),
    ("hash_string: (unsigned char) -> (signed char)",
     'hash = stbds_rotate_left(hash, 9).wrapping_add(*(s as *const u8) as usize);',
     'hash = stbds_rotate_left(hash, 9).wrapping_add(*(s as *const i8) as isize as usize);', False),
    ("hash_string: final `hash + seed` -> `hash`",
     '    hash.wrapping_add(seed)\n}',
     '    hash\n}', False),
    ("make_hash_index: used_count_threshold sc-sc/4 -> sc-sc/2",
     '(*t).used_count_threshold = slot_count.wrapping_sub(slot_count >> 2);',
     '(*t).used_count_threshold = slot_count.wrapping_sub(slot_count >> 1);', False),
    ("make_hash_index: tombstone_count_threshold drops the >>4 term",
     '(*t).tombstone_count_threshold = (slot_count >> 3).wrapping_add(slot_count >> 4);',
     '(*t).tombstone_count_threshold = slot_count >> 3;', False),
    ("make_hash_index: shrink-threshold gate <= 8 -> < 8",
     'if slot_count <= STBDS_BUCKET_LENGTH {\n        (*t).used_count_shrink_threshold = 0;',
     'if slot_count < STBDS_BUCKET_LENGTH {\n        (*t).used_count_shrink_threshold = 0;', False),
    ("make_hash_index: seed taken AFTER the advance instead of before",
     '        (*t).seed = stbds_hash_seed;',
     '        (*t).seed = stbds_hash_seed.wrapping_add(1);', False),
    ("make_hash_index: cache-line alignment 64 -> 32",
     'stbds_align_fwd(t.add(1) as usize, STBDS_CACHE_LINE_SIZE) as *mut stbds_hash_bucket',
     'stbds_align_fwd(t.add(1) as usize, 32) as *mut stbds_hash_bucket', False),
    ("hmput_key: temp_key refresh guard mode >= -> mode ==",
     '                        if mode >= STBDS_HM_STRING {\n                            let src = padd(',
     '                        if mode == STBDS_HM_STRING {\n                            let src = padd(', False),
    ("hmput_key: wrap-around dup hit ALSO refreshes temp_key ('fixing' the C bug)",
     '                        (*stbds_header(a)).temp = (*bucket).index[i];\n                        return stbds_arr_to_hash(a, elemsize);',
     '                        (*stbds_header(a)).temp = (*bucket).index[i];\n'
     '                        if mode >= STBDS_HM_STRING {\n'
     '                            let src = padd(raw_a, elemsize.wrapping_mul((*bucket).index[i] as usize).wrapping_add(keyoffset)) as *mut *mut c_char;\n'
     '                            (*stbds_hash_table(a)).temp_key = *src;\n'
     '                        }\n'
     '                        return stbds_arr_to_hash(a, elemsize);', False),
    ("hmput_key: bucket index i-1 -> i",
     '(*bucket).index[pos & STBDS_BUCKET_MASK] = i - 1;',
     '(*bucket).index[pos & STBDS_BUCKET_MASK] = i;', False),
    ("hmput_key: initial string.mode DEFAULT -> STRDUP",
     '            (*nt).string.mode = if mode >= STBDS_HM_STRING {\n                STBDS_SH_DEFAULT',
     '            (*nt).string.mode = if mode >= STBDS_HM_STRING {\n                STBDS_SH_STRDUP', False),
    ("hmput_key: tombstone not reused",
     '        if tombstone >= 0 {\n            pos = tombstone as usize;',
     '        if false {\n            pos = tombstone as usize;', False),
    ("hmdel_key: strdup free guard mode == -> mode >=",
     'if mode == STBDS_HM_STRING && (*table).string.mode == STBDS_SH_STRDUP {',
     'if mode >= STBDS_HM_STRING && (*table).string.mode == STBDS_SH_STRDUP {', False),
    ("hmdel_key: final_index off by one",
     'let final_index: isize = stbds_arrlen(raw_a) - 1 - 1;',
     'let final_index: isize = stbds_arrlen(raw_a) - 1;', False),
    ("hmdel_key: temp = 1 -> temp = 2 on a successful delete",
     '(*stbds_header(raw_a)).temp = 1;',
     '(*stbds_header(raw_a)).temp = 2;', False),
    ("hmdel_key: shrink gate slot_count > 8 -> >= 8",
     '                    && (*table).slot_count > STBDS_BUCKET_LENGTH',
     '                    && (*table).slot_count >= STBDS_BUCKET_LENGTH', False),
    ("hmdel_key: rebuild gate > -> >=",
     '} else if (*table).tombstone_count > (*table).tombstone_count_threshold {',
     '} else if (*table).tombstone_count >= (*table).tombstone_count_threshold {', False),
    ("hmget_key_ts: *temp = -1 -> -2 on a missing key",
     '            if slot < 0 {\n                *temp = STBDS_INDEX_EMPTY;',
     '            if slot < 0 {\n                *temp = STBDS_INDEX_DELETED;', False),
    ("hmput_default: drops the `length == 0` disjunct",
     'if a.is_null() || (*stbds_header(stbds_hash_to_arr(a, elemsize))).length == 0 {',
     'if a.is_null() {', False),
    ("hmfree_func: STRDUP sweep starts at 0 instead of 1",
     '            let mut i: usize = 1;\n            while i < (*stbds_header(a)).length {',
     '            let mut i: usize = 0;\n            while i < (*stbds_header(a)).length {', False),
    ("shmode_func: (unsigned char) truncation -> saturation",
     '(*h).string.mode = mode as u8;',
     '(*h).string.mode = if mode > 255 { 255 } else { mode as u8 };', False),
    ("stralloc: `++block` gate < MAX -> <= MAX",
     'if blocksize < STBDS_STRING_ARENA_BLOCKSIZE_MAX {',
     'if blocksize <= STBDS_STRING_ARENA_BLOCKSIZE_MAX {', False),
    ("stralloc: len > blocksize -> len >= blocksize",
     'if len > blocksize {',
     'if len >= blocksize {', False),
    ("stralloc: oversized splice becomes a head insert",
     '                (*sb).next = (*(*a).storage).next;\n                (*(*a).storage).next = sb;',
     '                (*sb).next = (*a).storage;\n                (*a).storage = sb;', False),
    ("stralloc: BLOCKSIZE_MIN 512 -> 256",
     'const STBDS_STRING_ARENA_BLOCKSIZE_MIN: usize = 512;',
     'const STBDS_STRING_ARENA_BLOCKSIZE_MIN: usize = 256;', False),
    ("stralloc: block>>1 -> block",
     'blocksize = STBDS_STRING_ARENA_BLOCKSIZE_MIN.wrapping_shl((blocksize >> 1) as u32);',
     'blocksize = STBDS_STRING_ARENA_BLOCKSIZE_MIN.wrapping_shl(blocksize as u32);', False),
    ("strreset: forgets to memset the arena",
     '    memset(\n        a as *mut c_void,\n        0,\n        core::mem::size_of::<stbds_string_arena>(),\n    );',
     '    (*a).storage = ptr::null_mut();', False),
    ("arrgrowf: forgets to init temp on a fresh allocation",
     '        (*stbds_header(b)).temp = 0;',
     '        (*stbds_header(b)).temp = 1;', False),
    ("arrgrowf: doubling uses 3x",
     'min_cap = stbds_arrcap(a).wrapping_mul(2);',
     'min_cap = stbds_arrcap(a).wrapping_mul(3);', False),
    ("stbds_hmlen: length-1 -> length",
     '((*stbds_header(psub(t, elemsize))).length as isize) - 1',
     '((*stbds_header(psub(t, elemsize))).length as isize)', False),
    ("strkey: format test_%d -> test%d",
     'sprintf(buf, b"test_%d\\0".as_ptr() as *const c_char, n);',
     'sprintf(buf, b"test%d\\0".as_ptr() as *const c_char, n);', False),
    ("str_dups: sh_new_strdup -> sh_new_arena",
     'strmap = stbds_shmode_func(elemsize, STBDS_SH_STRDUP as c_int) as *mut str_dups_entry;',
     'strmap = stbds_shmode_func(elemsize, STBDS_SH_ARENA as c_int) as *mut str_dups_entry;', False),
    ("str_dups: skips the strreset between the arena loop and the map block",
     '    stbds_strreset(&mut sa);\n\n    {',
     '    {', False),
    # The two asserts guarding the post-swap-delete re-lookup are mutually
    # redundant: removing either one alone still aborts at the other, i.e. that
    # is a behaviourally EQUIVALENT mutant. Removing both is the real test, and
    # it needs the slow fork-based abort comparisons.
    ("drop BOTH reachable asserts (c_src/src/lib.c:846 and :849) in hmdel_key",
     '                    STBDS_ASSERT!(slot >= 0, 846);',
     '                    // removed by the mutation check\n'
     '                    let _ = final_index;', True),
]

# Applied on top of the entry above, so that "both asserts" really means both.
EXTRA_FOR_LAST = (
    '                    STBDS_ASSERT!((*b).index[i as usize] == final_index, 849);',
    '                    // removed by the mutation check',
)

FAST_SKIP = ["--skip", "err_39_", "--skip", "err_34_"]

caught, missed, skipped = [], [], []
for name, o, m, slow in MUTATIONS:
    if o not in ORIG:
        skipped.append(name)
        print(f"SKIP    {name}   (patch target not found)", flush=True)
        continue
    mutated = ORIG.replace(o, m, 1)
    if slow:  # the "both asserts" case needs a second edit
        mutated = mutated.replace(EXTRA_FOR_LAST[0], EXTRA_FOR_LAST[1], 1)
    open(SRC, "w").write(mutated)
    b = subprocess.run(["cargo", "build", "--offline"], capture_output=True, text=True, timeout=900)
    if b.returncode != 0:
        skipped.append(name)
        print(f"NOBUILD {name}", flush=True)
        continue
    args = ["cargo", "test", "--offline", "--", "--test-threads=1"]
    if not slow:
        args += FAST_SKIP
    try:
        r = subprocess.run(args, capture_output=True, text=True, timeout=300)
        out = r.stdout + r.stderr
        rc = r.returncode
    except subprocess.TimeoutExpired as e:
        out = (e.stdout or b"").decode(errors="replace") if isinstance(e.stdout, bytes) else (e.stdout or "")
        rc = -1
        out += "\n<TIMEOUT>"
    failing = re.findall(r"^test (\S+) \.\.\. FAILED", out, re.M)
    crash = re.findall(r"signal: \d+, SIG\w+", out)
    if rc != 0:
        detail = ",".join(failing[:4]) or (crash[0] if crash else ("timeout/hang" if rc == -1 else "nonzero exit"))
        caught.append(name)
        print(f"CAUGHT  {name}\n          -> {detail}", flush=True)
    else:
        missed.append(name)
        print(f"MISSED  {name}", flush=True)

print()
print(f"CAUGHT  {len(caught)}")
print(f"MISSED  {len(missed)}")
print(f"SKIPPED {len(skipped)}")
EXPECTED_SURVIVORS = {
    # Behaviourally EQUIVALENT mutants - the C cannot distinguish them either.
    "make_hash_index: cache-line alignment 64 -> 32":
        "purely an internal allocation-layout detail; bucket contents, thresholds "
        "and every probe result are unchanged, and no address is ever compared",
    "hmdel_key: shrink gate slot_count > 8 -> >= 8":
        "dead clause: at slot_count == 8 make_hash_index forces "
        "used_count_shrink_threshold = 0 (c_src/src/lib.c:399-400), and "
        "`used_count < 0` is vacuously false for a size_t, so the extra "
        "slot_count test can never change the outcome",
    # Real behaviour differences that a caller of the PUBLIC API cannot observe.
    "hmdel_key: strdup free guard mode == -> mode >=":
        "the only difference is whether a strdup'd key leaks; the element is "
        "already past `length` so hmfree_func never touches it either way, and "
        "no address is compared - detecting this needs an allocator hook",
    "str_dups: sh_new_strdup -> sh_new_arena":
        "str_dups returns void and its only observable is stdout; both key "
        "ownership modes satisfy the three asserts and print the identical "
        "`a <num>` line",
    "str_dups: skips the strreset between the arena loop and the map block":
        "a pure memory leak; stdout is byte-identical",
}

if missed:
    print("\nSURVIVING MUTATIONS:")
    for n in missed:
        why = EXPECTED_SURVIVORS.get(n)
        if why:
            print(f"  [expected/equivalent] {n}\n        {why}")
        else:
            print(f"  [!! BLIND SPOT !!]    {n}")
    unexplained = [n for n in missed if n not in EXPECTED_SURVIVORS]
    print()
    print("UNEXPLAINED SURVIVORS:", unexplained if unexplained else "NONE")
    if unexplained:
        sys.exit(1)
sys.stdout.flush()
