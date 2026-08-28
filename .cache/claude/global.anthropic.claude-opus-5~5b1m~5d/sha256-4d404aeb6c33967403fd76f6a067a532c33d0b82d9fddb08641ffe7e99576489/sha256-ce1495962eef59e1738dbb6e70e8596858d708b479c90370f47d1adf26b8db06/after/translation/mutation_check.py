#!/usr/bin/env python3
"""Mutation check: prove the differential tests actually detect divergence.

Each mutation injects a plausible mistranslation into src/lib.rs, rebuilds the
cdylib, and runs the full test suite. A mutation that survives means the test
suite has a blind spot. src/lib.rs is always restored.
"""
import shutil
import subprocess
import sys
import os

HERE = os.path.dirname(os.path.abspath(__file__))
LIB = os.path.join(HERE, "src/lib.rs")
BAK = LIB + ".mutation-backup"

# (name, old, new, why it should be caught)
MUTATIONS = [
    (
        "compute_hash: signed pointer comparison",
        "    if d1 < d2 {\n        hash = hash.wrapping_add(100);\n    } else if d1 > d2 {",
        "    if (d1 as isize) < (d2 as isize) {\n        hash = hash.wrapping_add(100);\n    } else if (d1 as isize) > (d2 as isize) {",
        "C compares pointers UNSIGNED",
    ),
    (
        "compute_hash: swap the 100/200 constants",
        "        hash = hash.wrapping_add(100);\n    } else if d1 > d2 {\n        hash = hash.wrapping_add(200);",
        "        hash = hash.wrapping_add(200);\n    } else if d1 > d2 {\n        hash = hash.wrapping_add(100);",
        "ordering constants swapped",
    ),
    (
        "compute_hash: struct-pointer term dropped",
        "    if (mb1 as *const MemoryBlock) < (mb2 as *const MemoryBlock) {\n        hash = hash.wrapping_add(10);",
        "    if false && (mb1 as *const MemoryBlock) < (mb2 as *const MemoryBlock) {\n        hash = hash.wrapping_add(10);",
        "the +10 branch never fires",
    ),
    (
        "allocate_block: saturating instead of truncating fill",
        "        data.add(i).write(v as u32 as c_int);",
        "        data.add(i).write(if v > i32::MAX as usize { i32::MAX } else { v as c_int });",
        "C truncates init_value + i modulo 2^32",
    ),
    (
        "allocate_block: fill uses i32 add before widening",
        "        let v = (init_value as usize).wrapping_add(i);",
        "        let v = (init_value.wrapping_add(1) as usize).wrapping_add(i);",
        "off-by-one in the fill value",
    ),
    (
        "betagamma: floor division instead of truncation",
        "    result = result.wrapping_add(sum1.wrapping_sub(sum2).wrapping_div(10));",
        "    result = result.wrapping_add(sum1.wrapping_sub(sum2).div_euclid(10));",
        "C division truncates toward zero",
    ),
    (
        "betagamma: wrong error sentinel",
        "        free_block(mem1);\n        free_block(mem2);\n        return -1;",
        "        free_block(mem1);\n        free_block(mem2);\n        return 0;",
        "C returns -1",
    ),
    (
        "betagamma: block_size zero-extended instead of sign-extended",
        "    let block_size: usize = (param1 % 10).wrapping_add(5) as isize as usize;",
        "    let block_size: usize = (param1 % 10).wrapping_add(5) as u32 as usize;",
        "C sign-extends int -> size_t",
    ),
    (
        "create_block: flags written before strcpy",
        "    strcpy(core::ptr::addr_of_mut!((*p).name) as *mut c_char, name);\n\n    // block.flags = flags;\n    core::ptr::addr_of_mut!((*p).flags).write(flags);",
        "    core::ptr::addr_of_mut!((*p).flags).write(flags);\n\n    strcpy(core::ptr::addr_of_mut!((*p).name) as *mut c_char, name);",
        "C assigns flags AFTER the strcpy, so it wins over the overflow",
    ),
    (
        "betagamma: 0b00001111 mask flipped to a full mask",
        "        if flags & 0b0000_1111 != 0 {",
        "        if flags & 0b1111_1111 != 0 {",
        "block 3 (flags 0xF0) must NOT add param1",
    ),
    (
        "allocate_block: NULL check on data dropped",
        "    if data.is_null() {\n        free(mb as *mut c_void);\n        return core::ptr::null_mut();\n    }",
        "    if false {\n        free(mb as *mut c_void);\n        return core::ptr::null_mut();\n    }",
        "C returns NULL when calloc fails",
    ),
    (
        "free_block: NULL guard dropped",
        "    if !mb.is_null() {",
        "    if true {",
        "C tolerates free_block(NULL)",
    ),
]


def run(cmd, **kw):
    return subprocess.run(cmd, shell=True, cwd=HERE, capture_output=True, text=True, **kw)


def main():
    shutil.copyfile(LIB, BAK)
    original = open(LIB).read()
    caught, survived, broken = [], [], []
    try:
        for name, old, new, why in MUTATIONS:
            if old not in original:
                broken.append((name, "anchor text not found"))
                print(f"[SKIP ] {name}: anchor not found", flush=True)
                continue
            if original.count(old) != 1:
                broken.append((name, f"anchor appears {original.count(old)}x"))
                print(f"[SKIP ] {name}: ambiguous anchor", flush=True)
                continue
            open(LIB, "w").write(original.replace(old, new, 1))

            b = run("cargo build --release 2>&1")
            if b.returncode != 0:
                broken.append((name, "mutant did not compile"))
                print(f"[SKIP ] {name}: mutant does not compile", flush=True)
                continue

            t = run("timeout 400 cargo test --release --tests -- --test-threads=1 2>&1")
            failing = [
                l.split()[1]
                for l in t.stdout.splitlines()
                if l.startswith("test ") and l.rstrip().endswith("FAILED")
            ]
            # A mutant is caught if the suite does not come back clean. That
            # includes an assertion failure AND a hard crash: a divergent
            # mutant often segfaults the test binary outright, which prints no
            # "... FAILED" line at all.
            crashed = "SIGSEGV" in t.stdout or "signal:" in t.stdout
            if t.returncode != 0:
                how = (
                    f"{len(failing)} assert(s): {failing[:4]}"
                    if failing
                    else ("test binary crashed (SIGSEGV)" if crashed else "non-zero exit")
                )
                caught.append((name, how))
                print(f"[CAUGHT] {name}  -> {how}", flush=True)
            else:
                survived.append((name, why))
                print(f"[SURVIVED!] {name}  ({why})", flush=True)
    finally:
        shutil.copyfile(BAK, LIB)
        os.remove(BAK)
        run("cargo build --release")

    print("\n================ mutation summary ================")
    print(f"caught   : {len(caught)}")
    print(f"survived : {len(survived)}")
    print(f"skipped  : {len(broken)}")
    for n, w in survived:
        print(f"  SURVIVED: {n} ({w})")
    for n, w in broken:
        print(f"  SKIPPED : {n} ({w})")
    # Verify restore really happened.
    assert open(LIB).read() == original, "FAILED TO RESTORE src/lib.rs"
    print("src/lib.rs restored byte-for-byte: OK")
    return 1 if survived else 0


if __name__ == "__main__":
    sys.exit(main())
