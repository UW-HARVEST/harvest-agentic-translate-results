#!/usr/bin/env python3
"""Harness self-validation by mutation testing.

Injects one small behavioural mutation at a time into the Rust translation and
requires the differential test suite to FAIL for it. A suite that cannot fail
proves nothing.

Some mutants are *semantically equivalent* to the original C behaviour and must
NOT be detected; they are marked ``equivalent`` and serve as a control group.
"""

import os
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SRC = os.path.join(ROOT, "src", "hex2bin.rs")

# (name, expectation, from, to)
MUTANTS = [
    ("nibble accumulator c_val*16 -> c_val*17", "detect",
     "c_val.wrapping_mul(16)", "c_val.wrapping_mul(17)"),
    ("ignore-skip drops the state==0 guard", "detect",
     "!ignore.is_null() && state == 0 &&", "!ignore.is_null() &&"),
    ("ignore-skip drops the ignore!=NULL guard", "detect",
     "if !ignore.is_null() && state == 0 &&", "if state == 0 &&"),
    ("no hex_pos-- on odd digit count", "detect",
     "hex_pos = hex_pos.wrapping_sub(1);", "hex_pos = hex_pos.wrapping_sub(0);"),
    ("odd-digit error suppressed", "detect",
     "if state != 0 {", "if false {"),
    ("off-by-one in the bin_maxlen check", "detect",
     "if bin_pos >= bin_maxlen {", "if bin_pos > bin_maxlen {"),
    ("strchr no longer matches the NUL terminator", "detect",
     "        if b == c {", "        if b == c && b != 0 {"),
    ("strchr scans one byte too far", "detect",
     "        if b == 0 {\n            return false;\n        }",
     "        if b == 0 && p != s {\n            return false;\n        }"),
    ("classifier ignores the alpha class", "detect",
     "if (c_num0 | c_alpha0) == 0 {", "if (c_num0) == 0 {"),
    ("classifier ignores the digit class", "detect",
     "if (c_num0 | c_alpha0) == 0 {", "if (c_alpha0) == 0 {"),
    ("case folding mask ~32 -> ~16", "detect",
     "((c as u32) & !32u32)", "((c as u32) & !16u32)"),
    ("case folding mask dropped", "detect",
     "(((c as u32) & !32u32).wrapping_sub(55u32)) as u8",
     "((c as u32).wrapping_sub(55u32)) as u8"),
    ("alpha bias 55 -> 54", "detect",
     ".wrapping_sub(55u32)", ".wrapping_sub(54u32)"),
    ("strict-mode unconsumed-input check removed", "detect",
     "} else if hex_pos != hex_len {", "} else if false {"),
    ("hex_end_p written unconditionally", "detect",
     "if !hex_end_p.is_null() {", "if true {"),
    ("stored byte perturbed (| 1)", "detect",
     "*bin.wrapping_add(bin_pos) = c_acc | c_val;",
     "*bin.wrapping_add(bin_pos) = c_acc | c_val | 1;"),
    ("hex_end reported one byte too far", "detect",
     "*hex_end_p = hex.wrapping_add(hex_pos);",
     "*hex_end_p = hex.wrapping_add(hex_pos.wrapping_add(1));"),
    ("digit XOR constant 48 -> 49", "detect",
     "((c as u32) ^ 48u32) as u8", "((c as u32) ^ 49u32) as u8"),
    ("digit range bound 10 -> 11", "detect",
     "((c_num as u32).wrapping_sub(10u32) >> 8)",
     "((c_num as u32).wrapping_sub(11u32) >> 8)"),
    ("alpha range upper bound 16 -> 17", "detect",
     ".wrapping_sub(16u32)", ".wrapping_sub(17u32)"),
    ("alpha range lower bound 10 -> 11", "detect",
     "((c_alpha as u32).wrapping_sub(10u32))",
     "((c_alpha as u32).wrapping_sub(11u32))"),
    ("c_val loses the num class", "detect",
     "let c_val: u8 = (c_num0 & c_num) | (c_alpha0 & c_alpha);",
     "let c_val: u8 = (c_alpha0 & c_alpha);"),
    ("c_val loses the alpha class", "detect",
     "let c_val: u8 = (c_num0 & c_num) | (c_alpha0 & c_alpha);",
     "let c_val: u8 = (c_num0 & c_num);"),
    ("shift 8 -> 7 in the digit classifier", "detect",
     "wrapping_sub(10u32) >> 8) as u8", "wrapping_sub(10u32) >> 7) as u8"),
    ("nibble halves swapped", "detect",
     "if state == 0 {\n            // c_acc = c_val * 16U;",
     "if state != 0 {\n            // c_acc = c_val * 16U;"),
    ("error value -1 -> -2 (buffer full)", "detect",
     "        if bin_pos >= bin_maxlen {\n            ret = -1;",
     "        if bin_pos >= bin_maxlen {\n            ret = -2;"),
    ("c_alpha truncated via u16 (identical low byte)", "equivalent",
     "let c_alpha: u8 = (((c as u32) & !32u32).wrapping_sub(55u32)) as u8;",
     "let c_alpha: u8 = (((c as u32) & !32u32).wrapping_sub(55u32)) as u16 as u8;"),
    # ---- control group: must NOT be detected ----
    ("state toggle uses ^1 instead of ~ (state only tested against 0)",
     "equivalent", "state = !state;", "state = state ^ 1;"),
    ("dead store bin_pos = 0 -> 1 on the error path", "equivalent",
     "    if ret != 0 {\n        bin_pos = 0;\n    }",
     "    if ret != 0 {\n        bin_pos = 1;\n    }"),
]


def run(cmd):
    return subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True)


def main():
    original = open(SRC).read()
    ok = bad = 0
    try:
        for name, expect, a, b in MUTANTS:
            open(SRC, "w").write(original)
            if original.count(a) != 1:
                print(f"ERROR  pattern not unique ({original.count(a)}): {name}")
                bad += 1
                continue
            open(SRC, "w").write(original.replace(a, b, 1))

            build = run(["cargo", "build", "--no-default-features"])
            if build.returncode != 0:
                print(f"ERROR  mutant does not compile: {name}")
                bad += 1
                continue
            test = run(["cargo", "test", "--no-default-features"])
            out = test.stdout + test.stderr
            if "STALE SHARED OBJECT" in out:
                print(f"ERROR  stale .so while testing: {name}")
                bad += 1
                continue
            # A mutant may also be caught by aborting (SIGSEGV/SIGABRT), in
            # which case libtest never prints its summary line, so key off the
            # process exit status instead of the summary text.
            got = "detect" if test.returncode != 0 else "equivalent"
            if got == expect:
                print(f"PASS   [{expect}] {name}")
                ok += 1
            else:
                print(f"FAIL   expected={expect} got={got}: {name}")
                bad += 1
    finally:
        open(SRC, "w").write(original)
        run(["cargo", "build", "--no-default-features"])

    print(f"---- {ok}/{len(MUTANTS)} mutants behaved as expected ({bad} unexpected) ----")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
