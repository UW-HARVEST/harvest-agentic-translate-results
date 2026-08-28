#!/usr/bin/env python3
"""Sensitivity check for the differential test suite.

The differential tests are only meaningful if they actually fail when the Rust
translation stops matching the C.  This script injects one small, behaviour
changing mutation into `src/lib.rs` at a time, runs the full test suite and
records whether the mutant was *killed* (some test failed).  The original file
is always restored, even on error.

Usage:  python3 scripts/mutation_check.py [--profile release|dev]
Exit code 0 iff every non-equivalent mutant was killed.
"""

import argparse
import os
import re
import subprocess
import sys

CRATE = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
LIB = os.path.join(CRATE, "src", "lib.rs")

# (label, old_text, new_text, expect_killed)
#
# `expect_killed = False` marks a *provably equivalent* mutant: the C program
# cannot observe the difference, so the suite is expected NOT to fail.
MUTANTS = [
    ("mode0: UINT16_MAX off-by-one",
     "const UINT16_MAX: c_int = 65535;",
     "const UINT16_MAX: c_int = 65534;", True),
    ("validate: `<` becomes `<=`",
     "    if value < 0 {\n        return 0;\n    }",
     "    if value <= 0 {\n        return 0;\n    }", True),
    ("validate: `>` becomes `>=`",
     "    if value > UINT16_MAX {\n        return 0;\n    }",
     "    if value >= UINT16_MAX {\n        return 0;\n    }", True),
    ("is_string_empty: NULL returns 0",
     "    if str.is_null() {\n        return 1;\n    }",
     "    if str.is_null() {\n        return 0;\n    }", True),
    ("is_string_empty: signed byte test",
     "    if *str != 0 {\n        return 0;\n    }",
     "    if *str > 0 {\n        return 0;\n    }", True),
    ("find_char_in_buffer: NULL not rejected",
     "    if buffer.is_null() {\n        return ptr::null_mut();\n    }",
     "    if false {\n        return ptr::null_mut();\n    }", True),
    ("find_char_in_buffer: size - 1",
     "memchr(buffer as *const c_void, target as c_int, size) as *mut c_char",
     "memchr(buffer as *const c_void, target as c_int, size.saturating_sub(1)) as *mut c_char",
     True),
    ("create_buffer: NULL not rejected",
     "    if initial.is_null() {\n        return ptr::null_mut();\n    }",
     "    if false {\n        return ptr::null_mut();\n    }", True),
    ("apply_operation: NULL returns 0",
     "        None => -1,",
     "        None => 0,", True),
    ("increment_counter: subtract instead of add",
     "        COUNTER = COUNTER.wrapping_add(value);",
     "        COUNTER = COUNTER.wrapping_sub(value);", True),
    ("multiply_counter: add instead of multiply",
     "        COUNTER = COUNTER.wrapping_mul(value);",
     "        COUNTER = COUNTER.wrapping_add(value);", True),
    ("reset_counter: off by one",
     "    unsafe {\n        COUNTER = value;\n        COUNTER\n    }",
     "    unsafe {\n        COUNTER = value.wrapping_add(1);\n        COUNTER\n    }", True),
    ("charinbuf: drop the counter reset on entry",
     "    COUNTER = 0;\n",
     "", True),
    ("charinbuf mode 1: += 10 becomes += 11",
     "result = result.wrapping_add(10);",
     "result = result.wrapping_add(11);", True),
    ("charinbuf mode 2: strlen + 1",
     "result = strlen(buffer) as c_int;",
     "result = strlen(buffer) as c_int + 1;", True),
    ("charinbuf mode 3: swap increment and multiply",
     "current_op = Some(multiply_counter as unsafe extern \"C\" fn(c_int) -> c_int);\n            result = apply_operation(current_op, opt2);",
     "current_op = Some(multiply_counter as unsafe extern \"C\" fn(c_int) -> c_int);\n            result = apply_operation(current_op, opt1);", True),
    ("charinbuf mode 3: decrement by 6",
     "result = apply_operation(current_op, 5);",
     "result = apply_operation(current_op, 6);", True),
    ("charinbuf mode 4: search for 'Y'",
     "let search_char: c_char = b'X' as c_char;",
     "let search_char: c_char = b'Y' as c_char;", True),
    ("charinbuf mode 4: offset + 1",
     "result = found_pos.offset_from(buffer) as c_int;",
     "result = found_pos.offset_from(buffer) as c_int + 1;", True),
    ("charinbuf default: returns -2",
     "            printf(c_str(b\"Invalid mode: %d\\n\\0\"), mode);\n            result = -1;",
     "            printf(c_str(b\"Invalid mode: %d\\n\\0\"), mode);\n            result = -2;", True),
    ("charinbuf: mode 5 accepted as mode 0",
     "    match mode {\n        0 => {",
     "    match mode {\n        0 | 5 => {", True),
    ("printf text typo (mode 2)",
     "b\"Buffer freed successfully\\n\\0\"",
     "b\"Buffer freed successfuly\\n\\0\"", True),
    ("printf text typo (mode 0 header)",
     "b\"Mode 0: UINT16_MAX validation\\n\\0\"",
     "b\"Mode 0: UINT16_MAX validation \\n\\0\"", True),
    # Equivalent mutants: the observable behaviour cannot change.
    ("EQUIVALENT: %u for a positive constant",
     "c_str(b\"UINT16_MAX constant value: %u\\n\\0\"),\n                UINT16_MAX as c_uint,",
     "c_str(b\"UINT16_MAX constant value: %d\\n\\0\"),\n                UINT16_MAX as c_uint,", False),
    ("EQUIVALENT: memchr target zero-extended",
     "memchr(buffer as *const c_void, target as c_int, size) as *mut c_char",
     "memchr(buffer as *const c_void, target as u8 as c_int, size) as *mut c_char", False),
]


def run_tests(profile):
    cmd = ["cargo", "test", "--quiet"]
    if profile == "release":
        cmd.append("--release")
    p = subprocess.run(cmd, cwd=CRATE, capture_output=True, text=True, timeout=900)
    return p.returncode, p.stdout + p.stderr


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--profile", default="release", choices=["release", "dev"])
    args = ap.parse_args()

    with open(LIB) as f:
        original = f.read()

    results = []
    try:
        rc, out = run_tests(args.profile)
        if rc != 0:
            print("baseline test run FAILED — fix that before mutation testing")
            print(out[-4000:])
            return 1
        print("baseline: all tests pass\n")

        for label, old, new, expect_killed in MUTANTS:
            if original.count(old) != 1:
                print(f"SKIP  {label}: pattern occurs {original.count(old)} times")
                results.append((label, None, expect_killed))
                continue
            with open(LIB, "w") as f:
                f.write(original.replace(old, new))
            try:
                rc, out = run_tests(args.profile)
            finally:
                with open(LIB, "w") as f:
                    f.write(original)
            killed = rc != 0
            nfail = sum(int(m) for m in re.findall(r"(\d+) failed", out))
            mark = "KILLED " if killed else "SURVIVED"
            print(f"{mark}  {label}" + (f"  ({nfail} failing test(s))" if nfail else ""))
            results.append((label, killed, expect_killed))
    finally:
        with open(LIB, "w") as f:
            f.write(original)
        print("\nsrc/lib.rs restored")

    bad = [(l, k, e) for (l, k, e) in results if k is not None and k != e]
    skipped = [l for (l, k, _) in results if k is None]
    print(f"\n{sum(1 for _, k, e in results if k == e and e)} behaviour-changing mutants killed, "
          f"{sum(1 for _, k, e in results if k == e and not e)} equivalent mutants correctly survived, "
          f"{len(bad)} unexpected, {len(skipped)} skipped")
    for l, k, e in bad:
        print(f"  UNEXPECTED: {l} (killed={k}, expected_killed={e})")
    # A final rebuild so the .so on disk matches the pristine source again.
    subprocess.run(["cargo", "build", "--release"], cwd=CRATE, check=False,
                   capture_output=True)
    subprocess.run(["cargo", "build"], cwd=CRATE, check=False, capture_output=True)
    return 1 if bad or skipped else 0


if __name__ == "__main__":
    sys.exit(main())
