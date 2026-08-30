#!/usr/bin/env python3
"""Mutation testing: inject known-wrong behaviour into src/lib.rs one change at
a time and confirm the comparison suite reports a mismatch each time.

Run from the `translation/` directory. Restores src/lib.rs on exit.
"""
import shutil
import subprocess
import sys

SRC = "src/lib.rs"
BAK = "/tmp/lib.rs.mutation-backup"

MUTATIONS = [
    ("no-digits path returns 0 instead of failing",
     "        // No digits: matching failure (or input failure at end of string).\n        return None;",
     "        return Some((0, pos));"),
    ("LONG_MIN clamp replaced with -1",
     "        if neg < i128::from(i64::MIN) {\n            i64::MIN",
     "        if neg < i128::from(i64::MIN) {\n            -1"),
    ("LONG_MAX clamp replaced with 0",
     "    } else if acc > i128::from(i64::MAX) {\n        i64::MAX",
     "    } else if acc > i128::from(i64::MAX) {\n        0"),
    ("printf format %u instead of %d",
     'printf(c"%d\\n".as_ptr(), result)',
     'printf(c"%u\\n".as_ptr(), result)'),
    ("printf drops the trailing newline",
     'printf(c"%d\\n".as_ptr(), result)',
     'printf(c"%d".as_ptr(), result)'),
    ("leading whitespace not skipped",
     "    while pos < s.len() && is_c_space(s[pos]) {\n        pos += 1;\n    }",
     "    // leading whitespace skip removed"),
    ("call_fma len==0 early-out removed",
     "    if len == 0 {\n        return 0;\n    }",
     "    if len == 0 {\n        return 1;\n    }"),
    ("digit accumulation uses base 8",
     "acc = acc * 10 + i128::from(s[pos] - b'0');",
     "acc = acc * 8 + i128::from(s[pos] - b'0');"),
    ("i64 -> c_int truncation replaced by saturation",
     "Some((as_long as c_int, pos))",
     "Some((as_long.clamp(i32::MIN as i64, i32::MAX as i64) as c_int, pos))"),
]

# Mutations that are *behaviourally equivalent* to the original, so the suite is
# expected to keep passing. Recorded here so the reasoning is not lost.
EQUIVALENT = [
    # Stopping the accumulator early only ever shrinks `acc` while leaving it
    # >= 2^63. Positive values then still take the `acc > i64::MAX` branch and
    # clamp to LONG_MAX; negative values still land on exactly LONG_MIN
    # (either via `neg < i64::MIN` or via `neg == i64::MIN`). Same result.
    ("saturation threshold uses i64::MAX instead of u64::MAX",
     "if acc > i128::from(u64::MAX) {",
     "if acc > i128::from(i64::MAX) {"),
]

TESTS = ["level1_fma_array", "level2_call_fma", "level3_driver", "level4_symbols"]


def run_suite():
    """True if every test target passes."""
    for t in TESTS:
        r = subprocess.run(
            ["cargo", "test", "--test", t],
            capture_output=True, timeout=600,
        )
        if r.returncode != 0:
            return False, t
    return True, None


def main():
    shutil.copy(SRC, BAK)
    original = open(SRC).read()
    gaps, ok = [], 0
    try:
        for desc, old, new in MUTATIONS:
            if old not in original:
                print(f"  ERROR   {desc}: pattern not found")
                gaps.append(desc)
                continue
            open(SRC, "w").write(original.replace(old, new, 1))
            passed, failing = run_suite()
            if passed:
                print(f"  GAP     {desc}: suite still passes")
                gaps.append(desc)
            else:
                print(f"  caught  {desc} (by {failing})")
                ok += 1
    finally:
        shutil.copy(BAK, SRC)

    print(f"\n{ok}/{len(MUTATIONS)} mutations caught")

    # Sanity-check that the equivalent mutants really are equivalent.
    eq_bad = []
    try:
        for desc, old, new in EQUIVALENT:
            if old not in original:
                print(f"  ERROR   [equivalent] {desc}: pattern not found")
                eq_bad.append(desc)
                continue
            open(SRC, "w").write(original.replace(old, new, 1))
            passed, failing = run_suite()
            if passed:
                print(f"  equiv   {desc} (still matches C, as expected)")
            else:
                print(f"  SURPRISE {desc}: expected equivalent, but {failing} failed")
                eq_bad.append(desc)
    finally:
        shutil.copy(BAK, SRC)

    if gaps or eq_bad:
        if gaps:
            print("uncaught:")
            for g in gaps:
                print(f"  - {g}")
        if eq_bad:
            print("unexpectedly non-equivalent:")
            for g in eq_bad:
                print(f"  - {g}")
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
