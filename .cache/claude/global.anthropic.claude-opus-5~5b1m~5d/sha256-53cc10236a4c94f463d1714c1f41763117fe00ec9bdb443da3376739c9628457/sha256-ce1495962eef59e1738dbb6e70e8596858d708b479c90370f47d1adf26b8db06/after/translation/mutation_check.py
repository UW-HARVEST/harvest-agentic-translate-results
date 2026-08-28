#!/usr/bin/env python3
"""Sanity-check that the differential suite actually detects divergence.

Applies ONE deliberate bug to the Rust source at a time, rebuilds the cdylib,
runs the suites, and records whether they caught it. Every mutant must be
caught; a surviving mutant means the suite has a blind spot.

The pristine sources are held in memory and always restored, including on error.
"""

import pathlib
import subprocess
import sys

SRC = pathlib.Path("src")
FILES = sorted(SRC.glob("*.rs"))
PRISTINE = {p: p.read_text() for p in FILES}

# Two mutants are provably *semantically equivalent* to the original on this
# target, so no test can catch them. They are kept (marked EQUIVALENT) because
# demonstrating the equivalence is itself part of the verification argument.
EQUIVALENT = {
    "create_numeric_buffer: rem_euclid instead of C's truncating %":
        "C's `%` and `rem_euclid` differ by exactly 256 for negative operands, "
        "and the subsequent `(char)`/`as i8` keeps only the low 8 bits, where "
        "256 == 0. Verified exhaustively in C over 200001 values: 0 differences.",
    "doubleneg: pass the %ld offset as a 32-bit int (varargs width bug)":
        "The offset is always 0..=255 and x86-64 32-bit moves zero-extend into "
        "the full 64-bit varargs register, so `%ld` reads the same value. The "
        "translation nonetheless uses the correct `c_long`.",
}

# (label, file, needle, replacement)
MUTANTS = [
    ("cvt: NaN yields 0 instead of the cvttsd2si indefinite value",
     "cvt.rs", "        return c_int::MIN;\n    }", "        return 0;\n    }"),

    ("cvt: Rust's saturating `as i32` instead of emulating cvttsd2si",
     "cvt.rs",
     "    let truncated = value.trunc();\n"
     "    if truncated >= -2_147_483_648.0_f64 && truncated <= 2_147_483_647.0_f64 {\n"
     "        truncated as c_int\n"
     "    } else {\n"
     "        c_int::MIN\n"
     "    }",
     "    value as c_int"),

    ("create_numeric_buffer: rem_euclid instead of C's truncating %",
     "buffer.rs",
     "let value = seed.wrapping_add((i as c_int).wrapping_mul(7)) % 256;",
     "let value = seed.wrapping_add((i as c_int).wrapping_mul(7)).rem_euclid(256);"),

    ("create_numeric_buffer: stride 8 instead of 7",
     "buffer.rs",
     "(i as c_int).wrapping_mul(7)",
     "(i as c_int).wrapping_mul(8)"),

    ("find_value_in_buffer: last match instead of first",
     "buffer.rs", "bytes.iter().position(|&b| b == needle)",
     "bytes.iter().rposition(|&b| b == needle)"),

    ("find_value_in_buffer: sign-extend the needle instead of truncating",
     "buffer.rs", "    let target = search_val as u8;",
     "    let target = if search_val < 0 { 0u8 } else { search_val as u8 };"),

    ("find_value_in_buffer: return 0 instead of -1 on a miss",
     "buffer.rs", "        None => -1,", "        None => 0,"),

    ("calculate_with_doubles: rem_euclid exponent (loses negative exponents)",
     "dmath.rs", "let exponent = f64::from(c % 10);",
     "let exponent = f64::from(c.rem_euclid(10));"),

    ("calculate_with_doubles: drop the `b != 0` guard",
     "dmath.rs",
     "    if b != 0 {\n        result = f64::from(a) / f64::from(b);\n    }",
     "    result = f64::from(a) / f64::from(b);"),

    ("process_negation: `> 0` instead of `!= 0` (breaks negative inputs)",
     "negation.rs", "c_int::from(var1 != 0)", "c_int::from(var1 > 0)"),

    ("doubleneg: pass the %ld offset as a 32-bit int (varargs width bug)",
     "doubleneg.rs", "                offset as c_long,", "                offset as c_int,"),

    ("doubleneg: rem_euclid on the combined-loop stride",
     "doubleneg.rs",
     "param1.wrapping_add(i.wrapping_mul(param2)) % 256",
     "param1.wrapping_add(i.wrapping_mul(param2)).rem_euclid(256)"),

    ("doubleneg: search_values uses rem_euclid instead of C's %",
     "doubleneg.rs",
     "[param2 % 256, param3 % 256, param4 % 256, 42]",
     "[param2.rem_euclid(256), param3.rem_euclid(256), param4.rem_euclid(256), 42]"),

    ("doubleneg: negation_result weighted by 100 instead of 10",
     "doubleneg.rs", "negation_result.wrapping_mul(10)", "negation_result.wrapping_mul(100)"),

    ("doubleneg: pow(2.0, 40) becomes pow(2.0, 30) (in-range, changes output)",
     "doubleneg.rs", "ffi::pow(2.0, 40.0)", "ffi::pow(2.0, 30.0)"),
]

SUITES = ["symbols", "configs", "errors", "doubleneg"]


def restore():
    for p, text in PRISTINE.items():
        p.write_text(text)


def run(cmd):
    return subprocess.run(cmd, capture_output=True, text=True, timeout=600)


def main():
    # Verify the clean baseline first.
    restore()
    assert run(["cargo", "build", "--offline", "--release"]).returncode == 0
    for s in SUITES:
        r = run(["cargo", "test", "--offline", "--release", "--test", s,
                 "--", "--test-threads=1"])
        if r.returncode != 0:
            print(f"BASELINE FAILURE in suite {s}:\n{r.stdout}\n{r.stderr}")
            return 1
    print("baseline: all suites pass\n")

    results = []
    for label, fname, needle, repl in MUTANTS:
        path = SRC / fname
        original = PRISTINE[path]
        if needle not in original:
            restore()
            print(f"MUTANT SPEC STALE (needle not found in {fname}): {label}")
            return 1
        path.write_text(original.replace(needle, repl, 1))

        build = run(["cargo", "build", "--offline", "--release"])
        if build.returncode != 0:
            restore()
            print(f"MUTANT DID NOT COMPILE: {label}\n{build.stderr[-2000:]}")
            return 1

        caught_by = []
        for s in SUITES:
            r = run(["cargo", "test", "--offline", "--release", "--test", s,
                     "--", "--test-threads=1"])
            if r.returncode != 0:
                caught_by.append(s)

        restore()
        if caught_by:
            status = "CAUGHT  "
        elif label in EQUIVALENT:
            status = "EQUIVALENT"
        else:
            status = "SURVIVED"
        print(f"{status} {label}")
        if caught_by:
            print(f"           by: {', '.join(caught_by)}")
        elif label in EQUIVALENT:
            print(f"           justification: {EQUIVALENT[label]}")
        results.append((label, caught_by))

    restore()
    run(["cargo", "build", "--offline", "--release"])

    survived = [lbl for lbl, c in results if not c and lbl not in EQUIVALENT]
    equivalent = [lbl for lbl, c in results if not c and lbl in EQUIVALENT]
    caught = len(results) - len(survived) - len(equivalent)
    print(f"\n{caught}/{len(results)} mutants caught, "
          f"{len(equivalent)} provably equivalent, {len(survived)} surviving")
    if survived:
        print("SURVIVING MUTANTS (suite has a blind spot):")
        for lbl in survived:
            print(f"  - {lbl}")
        return 1
    print("No unexplained survivors -- the differential suite has teeth.")
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except BaseException:
        restore()
        raise
