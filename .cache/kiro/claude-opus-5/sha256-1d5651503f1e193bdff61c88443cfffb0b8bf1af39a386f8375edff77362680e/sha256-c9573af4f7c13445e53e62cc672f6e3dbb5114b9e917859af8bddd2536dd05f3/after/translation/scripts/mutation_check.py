#!/usr/bin/env python3
"""Mutation check for the C-vs-Rust differential suite.

Deliberately breaks `src/lib.rs` in ways the C forbids and confirms the suite
CATCHES each one. A surviving mutant means the suite has a blind spot.

Literal (non-regex) replacement is used, and each mutation is verified to have
actually been applied before the mutant is judged -- a regex-based version of
this script silently no-op'd and reported false "SURVIVED" results.

Exit status: 0 if every non-equivalent mutant was caught.
"""

import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SRC = ROOT / "src/lib.rs"
BAK = ROOT / "target/lib.rs.mutation-backup"

# (name, literal_from, literal_to, expected)
#   expected="caught"     -> must make the suite fail
#   expected="equivalent" -> provably no observable difference; must survive
MUTANTS = [
    ("M1  drop the +1 (terminator not copied)",
     "wrapping_add(1)", "wrapping_add(0)", "caught"),
    ("M2  remove the NULL-input check",
     "if str.is_null() {", "if false {", "caught"),
    ("M3  return the input pointer instead of a copy",
     "    newstr as *mut c_char", "    str as *mut c_char", "caught"),
    ("M4  remove the malloc-failure check",
     "if newstr.is_null() {", "if false {", "caught"),
    ("M5  off-by-one long copy",
     "wrapping_add(1)", "wrapping_add(2)", "caught"),
    ("M6  copy one byte too few (terminator dropped from memcpy)",
     "memcpy(newstr, str as *const c_void, len)",
     "memcpy(newstr, str as *const c_void, len - 1)", "caught"),
    ("M7  saturating instead of wrapping length arithmetic",
     "wrapping_add(1)", "saturating_add(1)", "equivalent"),
    ("M8  allocate one byte extra (slack, not observable via the API)",
     "unsafe { malloc(len) }", "unsafe { malloc(len + 1) }", "equivalent"),
]


def run(cmd, **kw):
    return subprocess.run(cmd, cwd=ROOT, capture_output=True, **kw)


def main():
    BAK.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy(SRC, BAK)
    original = BAK.read_text()
    problems = []
    try:
        for name, frm, to, expected in MUTANTS:
            if original.count(frm) != 1:
                problems.append(name)
                print(f"ERROR     {name}  (pattern occurs "
                      f"{original.count(frm)}x, expected exactly 1)")
                continue
            SRC.write_text(original.replace(frm, to, 1))

            if run(["cargo", "build", "-q"]).returncode != 0:
                print(f"skip      {name}  (mutant does not compile)")
                continue

            # Any nonzero status -- assertion, abort, or signal -- means caught.
            failed = run(["cargo", "test", "-q"],
                         timeout=600).returncode != 0
            actual = "caught" if failed else "survived"

            if expected == "caught" and actual == "caught":
                print(f"caught    {name}")
            elif expected == "equivalent" and actual == "survived":
                print(f"survived  {name}  (expected: semantically equivalent)")
            else:
                print(f"MISMATCH  {name}  expected={expected} actual={actual}")
                problems.append(name)
    finally:
        shutil.copy(BAK, SRC)
        run(["cargo", "build", "-q"])
        BAK.unlink(missing_ok=True)

    print()
    if problems:
        print("Mutation check FAILED for:")
        for p in problems:
            print(f"  - {p}")
        return 1
    print("Mutation check PASSED: every behaviour-changing mutant is detected "
          "by the differential suite, and only provably equivalent mutants "
          "survive.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
