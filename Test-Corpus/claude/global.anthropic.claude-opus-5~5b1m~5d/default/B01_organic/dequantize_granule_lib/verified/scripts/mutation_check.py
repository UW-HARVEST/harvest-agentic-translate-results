#!/usr/bin/env python3
"""Mutation-test the differential suite.

A differential test suite that passes is worthless if it would *also* pass
against a broken translation.  This script injects known mistranslations into
`src/lib.rs`, one at a time, rebuilds the cdylib and re-runs the whole suite.
Every mutation must make at least one test FAIL.

Mutations that are provably behaviour-preserving are listed separately and are
*expected* to survive.
"""
import shutil
import subprocess
import sys
import os

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SRC = os.path.join(ROOT, "src", "lib.rs")
BAK = SRC + ".mutation-backup"

# (name, old, new)
MUST_BE_CAUGHT = [
    (
        "guard uses >= instead of > (off-by-one on the last legal read)",
        "if (*bs).pos > (*bs).limit {",
        "if (*bs).pos >= (*bs).limit {",
    ),
    (
        "pos is not advanced when the underrun guard fires",
        """    (*bs).pos = (*bs).pos.wrapping_add(n);
    if (*bs).pos > (*bs).limit {
        return 0;
    }""",
        """    if (*bs).pos.wrapping_add(n) > (*bs).limit {
        return 0;
    }
    (*bs).pos = (*bs).pos.wrapping_add(n);""",
    ),
    (
        "left shift in the cache loop is not masked to 5 bits",
        "cache |= next.wrapping_shl(shl as u32);",
        "cache |= next.checked_shl(shl as u32).unwrap_or(0);",
    ),
    (
        "sub-byte bit phase `s` ignored (assumes byte-aligned reads)",
        "let s: u32 = ((*bs).pos & 7) as u32;",
        "let s: u32 = 0;",
    ),
    (
        "first byte is not masked with 255 >> s",
        "let mut next: u32 = (*p as u32) & (255u32 >> s);",
        "let mut next: u32 = *p as u32;",
    ),
    (
        "bitalloc indexing clamped to the declared 64 bytes (no OOB read)",
        "let ba: c_int = *bitalloc.wrapping_offset(i as isize) as c_int;",
        "let ba: c_int = *bitalloc.wrapping_offset((i & 63) as isize) as c_int;",
    ),
    (
        "mod/2 computed with signed instead of unsigned division",
        "let v = (code % m).wrapping_sub(m / 2) as c_int;",
        "let v = (code % m).wrapping_sub(((m as c_int) / 2) as u32) as c_int;",
    ),
    (
        "return value saturates instead of wrapping",
        "    group_size.wrapping_mul(4)\n}",
        "    group_size.saturating_mul(4)\n}",
    ),
    (
        "shift count for `2 << (ba - 17)` masked to 4 bits instead of 5",
        "let m: u32 = (2i32.wrapping_shl((ba - 17) as u32) as u32).wrapping_add(1);",
        "let m: u32 = (2i32.wrapping_shl(((ba - 17) as u32) & 15) as u32).wrapping_add(1);",
    ),
    (
        "code /= mod applied before the sample is written",
        """                        let v = (code % m).wrapping_sub(m / 2) as c_int;
                        *dst.wrapping_offset(k as isize) = v as f32;
                        k += 1;
                        code /= m;""",
        """                        code /= m;
                        let v = (code % m).wrapping_sub(m / 2) as c_int;
                        *dst.wrapping_offset(k as isize) = v as f32;
                        k += 1;""",
    ),
    (
        "get_bits called once per band instead of once per sample (half branch)",
        """                    let mut k: c_int = 0;
                    while k < group_size {
                        let v = (get_bits(bs, ba) as c_int).wrapping_sub(half);
                        *dst.wrapping_offset(k as isize) = v as f32;
                        k += 1;
                    }""",
        """                    let mut k: c_int = 0;
                    let raw = get_bits(bs, ba) as c_int;
                    while k < group_size {
                        let v = raw.wrapping_sub(half);
                        *dst.wrapping_offset(k as isize) = v as f32;
                        k += 1;
                    }""",
    ),
    (
        "choff walk uses +18 per band instead of alternating +576 / -558",
        """            dst = dst.wrapping_offset(choff as isize);
            choff = 18i32.wrapping_sub(choff);""",
        """            dst = dst.wrapping_offset(18);
            choff = 18i32.wrapping_sub(choff);""",
    ),
    (
        "granule stride uses a fixed 576 instead of group_size * j",
        "let mut dst: *mut f32 = grbuf.wrapping_offset(group_size.wrapping_mul(j) as isize);",
        "let mut dst: *mut f32 = grbuf.wrapping_offset((576 * j) as isize);",
    ),
    (
        "half branch threshold is `ba <= 17` instead of `ba < 17`",
        "if ba < 17 {",
        "if ba <= 17 {",
    ),
    (
        "get_bits width for the mod branch drops the `- (mod >> 3)` term",
        "get_bits(bs, m.wrapping_add(2).wrapping_sub(m >> 3) as c_int);",
        "get_bits(bs, m.wrapping_add(2) as c_int);",
    ),
]

# Provably behaviour-preserving: `2 * total_bands` is always even, so `choff`
# is back at 576 at the end of every `j` iteration.  A surviving mutation here
# is correct, not a gap in the suite.
EXPECTED_TO_SURVIVE = [
    (
        "choff re-initialised inside the j loop (equivalent: 2*total_bands is even)",
        """    let mut choff: c_int = 576;
    let mut j: c_int = 0;
    while j < 4 {
        let mut dst: *mut f32 =""",
        """    let mut j: c_int = 0;
    while j < 4 {
        let mut choff: c_int = 576;
        let mut dst: *mut f32 =""",
    ),
]


def run(cmd):
    return subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True)


def suite_passes():
    build = run(["cargo", "build", "--offline", "--quiet"])
    if build.returncode != 0:
        return None, "BUILD FAILED:\n" + build.stderr[-2000:]
    test = run(["cargo", "test", "--offline", "--quiet", "--", "--test-threads=8"])
    return test.returncode == 0, (test.stdout + test.stderr)[-1500:]


def main():
    shutil.copyfile(SRC, BAK)
    original = open(BAK).read()
    failures = []
    try:
        ok, log = suite_passes()
        if ok is not True:
            print("baseline suite does not pass; aborting\n" + log)
            return 1
        print("baseline: suite PASSES on the unmutated translation\n")

        for label, old, new in MUST_BE_CAUGHT:
            assert original.count(old) == 1, f"mutation anchor not unique: {label}"
            open(SRC, "w").write(original.replace(old, new, 1))
            ok, log = suite_passes()
            if ok is None:
                verdict = "caught (does not even compile)"
                caught = True
            elif ok:
                verdict = "*** SURVIVED — the suite is blind to this bug ***"
                caught = False
            else:
                verdict = "caught"
                caught = True
            print(f"[{'ok ' if caught else 'GAP'}] {label}: {verdict}")
            if not caught:
                failures.append(label)
                print(log)

        print()
        for label, old, new in EXPECTED_TO_SURVIVE:
            assert original.count(old) == 1, f"anchor not unique: {label}"
            open(SRC, "w").write(original.replace(old, new, 1))
            ok, _ = suite_passes()
            print(f"[{'ok ' if ok else 'HUH'}] {label}: "
                  f"{'survived as expected' if ok else 'unexpectedly caught'}")
    finally:
        shutil.copyfile(BAK, SRC)
        os.remove(BAK)
        run(["cargo", "build", "--offline", "--quiet"])

    if failures:
        print(f"\n{len(failures)} mutation(s) survived: {failures}")
        return 1
    print("\nall behaviour-changing mutations were caught")
    return 0


if __name__ == "__main__":
    sys.exit(main())
