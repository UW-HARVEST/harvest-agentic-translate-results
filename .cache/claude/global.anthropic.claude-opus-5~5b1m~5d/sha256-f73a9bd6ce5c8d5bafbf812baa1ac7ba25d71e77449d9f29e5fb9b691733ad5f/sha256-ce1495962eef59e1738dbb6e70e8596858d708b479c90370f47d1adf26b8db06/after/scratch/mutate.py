#!/usr/bin/env python3
"""Mutation study: each mutation injects a plausible translation bug into
translation/src/lib.rs, rebuilds the cdylib and runs the whole differential
suite.  Every mutation MUST be detected (some test must fail); a surviving
mutation means the test suite has a blind spot.
"""
import os, shutil, subprocess, sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SRC = os.path.join(ROOT, "translation", "src", "lib.rs")
BAK = os.path.join(ROOT, "scratch", "lib.rs.pristine")  # never overwritten

MUTATIONS = [
    ("M01 arrgrowf min_cap<4 -> <5",
     "} else if min_cap < 4 {\n            min_cap = 4;",
     "} else if min_cap < 5 {\n            min_cap = 5;"),

    ("M02 tombstone_count_threshold drops >>4 term",
     "(*t).tombstone_count_threshold = (slot_count >> 3).wrapping_add(slot_count >> 4);",
     "(*t).tombstone_count_threshold = slot_count >> 3;"),

    ("M03 used_count_shrink_threshold >>2 -> >>3",
     "(*t).used_count_shrink_threshold = slot_count >> 2;",
     "(*t).used_count_shrink_threshold = slot_count >> 3;"),

    ("M04 hash_string rotate 9 -> 8",
     "hash = STBDS_ROTATE_LEFT(hash, 9).wrapping_add(*s as u8 as usize);",
     "hash = STBDS_ROTATE_LEFT(hash, 8).wrapping_add(*s as u8 as usize);"),

    ("M05 hash_string loses the (unsigned char) cast",
     "hash = STBDS_ROTATE_LEFT(hash, 9).wrapping_add(*s as u8 as usize);",
     "hash = STBDS_ROTATE_LEFT(hash, 9).wrapping_add(*s as i8 as i64 as usize);"),

    ("M06 siphash tail case 4 zero-extends instead of sign-extends",
     "data |= ((*d.wrapping_add(3) as i32) << 24) as usize;",
     "data |= (*d.wrapping_add(3) as usize) << 24;"),

    ("M07 siphash main-loop word zero-extends instead of sign-extends",
     "            data = lo as usize;",
     "            data = lo as u32 as usize;"),

    ("M08 siphash D_ROUNDS 4 -> 3",
     "const STBDS_SIPHASH_D_ROUNDS: usize = 4;",
     "const STBDS_SIPHASH_D_ROUNDS: usize = 3;"),

    ("M09 siphash len<<56 -> len<<48",
     "data = len << (STBDS_SIZE_T_BITS - 8);",
     "data = len << (STBDS_SIZE_T_BITS - 16);"),

    ("M10 probe_position shifts the hash",
     "let pos = hash & (slot_count.wrapping_sub(1));",
     "let pos = (hash >> 1) & (slot_count.wrapping_sub(1));"),

    ("M11 hmput_key stores index i instead of i-1",
     "(*bucket).index[pos & STBDS_BUCKET_MASK] = i - 1;",
     "(*bucket).index[pos & STBDS_BUCKET_MASK] = i;"),

    ("M12 hmdel_key final_index off by one",
     "let final_index: isize = stbds_arrlen(raw_a) - 1 - 1;",
     "let final_index: isize = stbds_arrlen(raw_a) - 1;"),

    ("M13 hmfree_func strdup sweep starts at 0",
     "            if (*stbds_hash_table(a)).string.mode == STBDS_SH_STRDUP as u8 {\n                let mut i: usize = 1;",
     "            if (*stbds_hash_table(a)).string.mode == STBDS_SH_STRDUP as u8 {\n                let mut i: usize = 0;"),

    ("M14 stralloc blocksize shift block>>1 -> block",
     "blocksize = STBDS_STRING_ARENA_BLOCKSIZE_MIN << (blocksize >> 1);",
     "blocksize = STBDS_STRING_ARENA_BLOCKSIZE_MIN << blocksize;"),

    ("M15 stralloc block increment condition uses <=",
     "if blocksize < STBDS_STRING_ARENA_BLOCKSIZE_MAX {",
     "if blocksize <= STBDS_STRING_ARENA_BLOCKSIZE_MAX {"),

    ("M16 stralloc pointer offset off by one",
     "            .wrapping_add((*a).remaining)\n            .wrapping_sub(len);",
     "            .wrapping_add((*a).remaining)\n            .wrapping_sub(len + 1);"),

    ("M17 strkey format string",
     'let s = format!("test_{}", n);',
     'let s = format!("test{}", n);'),

    ("M18 hmget_key_ts table==NULL returns -2",
     "            if table.is_null() {\n                *temp = -1;",
     "            if table.is_null() {\n                *temp = -2;"),

    ("M19 is_key_equal mode >= STRING -> == STRING",
     "        if mode >= STBDS_HM_STRING {\n            let slot = (a as *mut u8)",
     "        if mode == STBDS_HM_STRING {\n            let slot = (a as *mut u8)"),

    ("M20 shmode_func mode truncation off by one",
     "(*h).string.mode = mode as u8;",
     "(*h).string.mode = (mode as u8).wrapping_add(1);"),

    ("M21 hmput_key grow uses slot_count*2 -> *4",
     "                (*table).slot_count.wrapping_mul(2)",
     "                (*table).slot_count.wrapping_mul(4)"),

    ("M22 used_count_threshold slot_count - slot_count>>2 -> >>1",
     "(*t).used_count_threshold = slot_count.wrapping_sub(slot_count >> 2);",
     "(*t).used_count_threshold = slot_count.wrapping_sub(slot_count >> 1);"),

    ("M23 hmput_default drops the length==0 case",
     "if a.is_null() || (*stbds_header(stbds_hash_to_arr(a, elemsize))).length == 0 {",
     "if a.is_null() {"),

    ("M24 hmdel_key sets temp=2 on success",
     "                    stbds_temp_set(raw_a, 1);",
     "                    stbds_temp_set(raw_a, 2);"),

    ("M25 seed advance constant b",
     "                let v32: usize = 715136305;",
     "                let v32: usize = 715136306;"),

    ("M26 hash_index storage alignment 64 -> 32",
     "            STBDS_CACHE_LINE_SIZE,\n        ) as *mut stbds_hash_bucket;",
     "            32,\n        ) as *mut stbds_hash_bucket;"),

    ("M27 shrink threshold zeroing uses < instead of <=",
     "        if slot_count <= STBDS_BUCKET_LENGTH {\n            (*t).used_count_shrink_threshold = 0;",
     "        if slot_count < STBDS_BUCKET_LENGTH {\n            (*t).used_count_shrink_threshold = 0;"),

    ("M28 hmput_key implicit string.mode for binary maps",
     "                (*nt).string.mode = if mode >= STBDS_HM_STRING {\n                    STBDS_SH_DEFAULT as u8\n                } else {\n                    0\n                };",
     "                (*nt).string.mode = STBDS_SH_DEFAULT as u8;"),
]


def run(cmd, cwd=None):
    return subprocess.run(cmd, cwd=cwd, shell=True, capture_output=True, text=True)


def restore(*_a):
    shutil.copyfile(BAK, SRC)
    run("cargo build --release", cwd=os.path.join(ROOT, "translation"))
    sys.exit(2)


def main():
    import signal
    signal.signal(signal.SIGTERM, restore)
    signal.signal(signal.SIGINT, restore)
    assert os.path.exists(BAK), "missing pristine baseline"
    orig = open(BAK).read()
    only = sys.argv[1] if len(sys.argv) > 1 else None
    survived, caught, skipped = [], [], []
    try:
        for name, old, new in MUTATIONS:
            if only and only not in name:
                continue
            if old not in orig:
                skipped.append((name, "pattern not found"))
                print(f"!! SKIP {name}: pattern not found")
                continue
            open(SRC, "w").write(orig.replace(old, new, 1))
            b = run("cargo build --release 2>&1 | tail -3",
                    cwd=os.path.join(ROOT, "translation"))
            if "error" in b.stdout:
                skipped.append((name, "build error"))
                print(f"!! SKIP {name}: does not compile\n{b.stdout}")
                continue
            t = run("timeout 900 cargo test --release -- --test-threads=1 > "
                    "/dev/null 2>&1",
                    cwd=os.path.join(ROOT, "translation"))
            failed = t.returncode != 0
            if failed:
                caught.append(name)
                print(f"OK   caught  {name}   (cargo test exit={t.returncode})")
            else:
                survived.append(name)
                print(f"XX   SURVIVED {name}")
    finally:
        shutil.copyfile(BAK, SRC)
        run("cargo build --release", cwd=os.path.join(ROOT, "translation"))

    print("\n==== mutation summary ====")
    print(f"caught   : {len(caught)}")
    print(f"survived : {len(survived)}")
    for s in survived:
        print(f"   SURVIVED: {s}")
    print(f"skipped  : {len(skipped)}")
    for s, why in skipped:
        print(f"   SKIPPED: {s} ({why})")
    return 1 if survived or skipped else 0


if __name__ == "__main__":
    sys.exit(main())
