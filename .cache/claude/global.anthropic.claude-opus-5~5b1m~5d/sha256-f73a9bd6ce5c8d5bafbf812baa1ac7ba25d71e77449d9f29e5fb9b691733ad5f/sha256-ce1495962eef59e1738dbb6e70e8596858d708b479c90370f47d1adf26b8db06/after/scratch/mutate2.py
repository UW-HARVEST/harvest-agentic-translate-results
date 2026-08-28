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
    ("M47 hmfree_func never frees strdup keys",
     "                let mut i: usize = 1;\n                while i < (*stbds_header(a)).length {",
     "                let mut i: usize = (*stbds_header(a)).length;\n                while i < (*stbds_header(a)).length {"),

    ("M48 hmdel_key frees the key for out-of-range string modes too",
     "                    if mode == STBDS_HM_STRING\n                        && (*table).string.mode == STBDS_SH_STRDUP as u8",
     "                    if mode >= STBDS_HM_STRING\n                        && (*table).string.mode == STBDS_SH_STRDUP as u8"),

    ("M29 seed constant b v64_lo (the bits that survive)",
     "                let v64_lo: usize = 0xb504f32d;",
     "                let v64_lo: usize = 0xb504f32e;"),

    ("M30 seed constant a v64_hi",
     "                let v64_hi: usize = 0x27bb2ee6;",
     "                let v64_hi: usize = 0x27bb2ee7;"),

    ("M31 seed constant a v64_lo",
     "                let v64_lo: usize = 0x87b0b0fd;",
     "                let v64_lo: usize = 0x87b0b0fe;"),

    ("M32 hmput_key forgets to decrement tombstone_count on reuse",
     "                pos = tombstone as usize;\n                (*table).tombstone_count -= 1;",
     "                pos = tombstone as usize;"),

    ("M33 hmdel_key forgets to increment tombstone_count",
     "                    (*table).tombstone_count += 1;",
     "                    (*table).tombstone_count += 0;"),

    ("M34 arrgrowf min_len > min_cap -> >=",
     "        if min_len > min_cap {",
     "        if min_len >= min_cap {"),

    ("M35 find_slot wrap loop i < limit -> i <= limit",
     "            let limit = pos & STBDS_BUCKET_MASK;\n            let mut i = 0usize;\n            while i < limit {",
     "            let limit = pos & STBDS_BUCKET_MASK;\n            let mut i = 0usize;\n            while i <= limit {"),

    ("M36 make_hash_index rehash drops used_count copy",
     "            (*t).used_count = (*ot).used_count;",
     "            (*t).used_count = 0;"),

    ("M37 arrgrowf resets length on realloc",
     "        if a.is_null() {\n            (*stbds_header(b)).length = 0;",
     "        if true {\n            (*stbds_header(b)).length = 0;"),

    ("M38 hmdel_key shrink condition < -> <=",
     "                    if (*table).used_count < (*table).used_count_shrink_threshold",
     "                    if (*table).used_count <= (*table).used_count_shrink_threshold"),

    ("M39 hmdel_key rebuild condition > -> >=",
     "                    } else if (*table).tombstone_count > (*table).tombstone_count_threshold {",
     "                    } else if (*table).tombstone_count >= (*table).tombstone_count_threshold {"),

    ("M40 stralloc oversized splice order",
     "                    (*sb).next = (*(*a).storage).next;\n                    (*(*a).storage).next = sb;",
     "                    (*sb).next = (*a).storage;\n                    (*a).storage = sb;"),

    ("M41 hmput_key second probe loop scans the full bucket",
     "                let limit = pos & STBDS_BUCKET_MASK;\n                let mut i = 0usize;\n                while i < limit {\n                    if (*bucket).hash[i] == hash {\n                        if stbds_is_key_equal(\n                            raw_a,",
     "                let limit = STBDS_BUCKET_LENGTH;\n                let mut i = 0usize;\n                while i < limit {\n                    if (*bucket).hash[i] == hash {\n                        if stbds_is_key_equal(\n                            raw_a,"),

    ("M42 hmget_key does not publish temp into the header",
     "        stbds_temp_set(stbds_hash_to_arr(p, elemsize), temp);",
     "        let _ = temp;"),

    ("M43 hmput_key step growth uses a constant step",
     "                step = step.wrapping_add(STBDS_BUCKET_LENGTH);\n                pos &= (*table).slot_count.wrapping_sub(1);\n            }\n            // found_empty_slot:",
     "                pos &= (*table).slot_count.wrapping_sub(1);\n            }\n            // found_empty_slot:"),

    ("M44 hmdel_key returns NULL instead of a when table is NULL",
     "            stbds_temp_set(raw_a, 0);\n            if table.is_null() {\n                a",
     "            stbds_temp_set(raw_a, 0);\n            if table.is_null() {\n                ptr::null_mut()"),

    ("M45 strdup uses strlen without the NUL",
     "        let len = strlen(str_).wrapping_add(1);\n        let p = realloc(ptr::null_mut(), len) as *mut c_char;",
     "        let len = strlen(str_);\n        let p = realloc(ptr::null_mut(), len.wrapping_add(1)) as *mut c_char;"),

    ("M46 hmfree_func skips the strreset call",
     "            stbds_strreset(&raw mut (*stbds_hash_table(a)).string);",
     "            let _ = &raw mut (*stbds_hash_table(a)).string;"),
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
            t = run(f"timeout {os.environ.get('INNER_TIMEOUT','900')} cargo test --release -- --test-threads=1 > "
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
