#!/usr/bin/env python3
"""Verify the Phase A artifacts against reality.

Every row in ERRORS.md / CONFIGS.md names a test. This script checks that:
  1. each named test actually exists as a #[test] fn in tests/,
  2. each named test actually RAN and PASSED in the last suite execution,
  3. no #[test] fn is orphaned (present in code but not claimed by a row),
  4. every row is checked off.

It exists because a checked box in a markdown table is a claim, not evidence.
"""
import re
import subprocess
import sys
import os

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))


def rows(path):
    out = []
    with open(os.path.join(ROOT, path)) as fh:
        for line in fh:
            line = line.strip()
            if not line.startswith("|"):
                continue
            cells = [c.strip() for c in line.strip("|").split("|")]
            if len(cells) < 3:
                continue
            ident = cells[0]
            if not re.fullmatch(r"[CE]\d+", ident):
                continue
            test = cells[-2].strip("`")
            checked = cells[-1] == "[x]"
            out.append((ident, test, checked))
    return out


def declared_tests():
    found = {}
    tdir = os.path.join(ROOT, "tests")
    for name in sorted(os.listdir(tdir)):
        if not name.endswith(".rs"):
            continue
        text = open(os.path.join(tdir, name)).read()
        for m in re.finditer(r"#\[test\]\s*\n\s*fn\s+(\w+)", text):
            found[m.group(1)] = name
    return found


def passing_tests():
    """Run the suite and collect the names cargo reports as passing."""
    p = subprocess.run(
        ["cargo", "test"],
        cwd=ROOT,
        capture_output=True,
        text=True,
        timeout=600,
    )
    names = set(re.findall(r"^test (\S+) \.\.\. ok$", p.stdout, re.M))
    failed = set(re.findall(r"^test (\S+) \.\.\. FAILED$", p.stdout, re.M))
    return names, failed, p.returncode


def main():
    err = rows("ERRORS.md")
    cfg = rows("CONFIGS.md")
    decl = declared_tests()
    passed, failed, rc = passing_tests()

    problems = []

    print("ERRORS.md rows:  %d" % len(err))
    print("CONFIGS.md rows: %d" % len(cfg))
    print("#[test] fns:     %d" % len(decl))
    print("passed:          %d" % len(passed))
    print("failed:          %d" % len(failed))
    print()

    if failed:
        problems.append("tests reported FAILED: %s" % sorted(failed))
    if rc != 0:
        problems.append("cargo test exited %d" % rc)

    claimed = set()
    for label, table in (("ERRORS.md", err), ("CONFIGS.md", cfg)):
        for ident, test, checked in table:
            claimed.add(test)
            if not checked:
                problems.append("%s row %s is not checked off" % (label, ident))
            if test not in decl:
                problems.append(
                    "%s row %s names test '%s' which does not exist" % (label, ident, test)
                )
            elif test not in passed:
                problems.append(
                    "%s row %s names test '%s' which did not pass" % (label, ident, test)
                )

    for name in sorted(decl):
        if name not in claimed and not name.startswith(("boundary_", "d1_", "d2_", "d3_")):
            problems.append("test '%s' is not referenced by any table row" % name)

    if problems:
        print("PROBLEMS:")
        for p in problems:
            print("  - %s" % p)
        return 1
    print("ARTIFACT AUDIT: OK")
    print("  every ERRORS.md and CONFIGS.md row names an existing, passing test")
    print("  every row is checked off; no orphaned tests")
    return 0


if __name__ == "__main__":
    sys.exit(main())
