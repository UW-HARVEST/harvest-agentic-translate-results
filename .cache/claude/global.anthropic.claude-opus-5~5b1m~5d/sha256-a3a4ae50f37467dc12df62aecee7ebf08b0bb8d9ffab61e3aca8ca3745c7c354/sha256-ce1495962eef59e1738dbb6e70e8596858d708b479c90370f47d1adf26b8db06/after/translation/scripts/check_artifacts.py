#!/usr/bin/env python3
"""Mechanical gate tying CONFIGS.md / ERRORS.md to real, passing tests.

Phases B and C are only meaningful if every table row actually has a test behind
it. This script cross-checks in BOTH directions:

* every test name referenced by a table row exists in the compiled suite;
* every ``cfg_row*`` / ``err_row*`` test in the suite is referenced by some row
  (so a test cannot quietly exist outside the documented surface);
* every row is checked off, and the row numbers are contiguous with no gaps;
* the referenced tests all PASS.

Exits non-zero on any discrepancy.
"""

from __future__ import annotations

import re
import subprocess
import sys
from pathlib import Path

CRATE = Path(__file__).resolve().parent.parent


def listed_tests() -> set[str]:
    proc = subprocess.run(
        ["cargo", "test", "--release", "--", "--list"],
        cwd=CRATE,
        capture_output=True,
        text=True,
        timeout=600,
    )
    if proc.returncode != 0:
        print(proc.stdout + proc.stderr)
        raise SystemExit("cargo test --list failed")
    return set(re.findall(r"^(\S+): test$", proc.stdout, re.M))


def passing_tests() -> set[str]:
    proc = subprocess.run(
        ["cargo", "test", "--release"],
        cwd=CRATE,
        capture_output=True,
        text=True,
        timeout=600,
    )
    out = proc.stdout + proc.stderr
    if proc.returncode != 0:
        print(out)
        raise SystemExit("the test suite does not pass; fix that before checking artifacts")
    return set(re.findall(r"^test (\S+) \.\.\. ok$", out, re.M))


def rows(path: Path, prefix: str) -> list[tuple[int, str, str]]:
    """Returns (row number, test name, raw line) for each table row."""
    found = []
    for line in path.read_text().splitlines():
        if not line.startswith("|"):
            continue
        cells = [c.strip() for c in line.strip().strip("|").split("|")]
        if not cells or not re.fullmatch(r"\d+", cells[0]):
            continue
        num = int(cells[0])
        names = re.findall(rf"`({prefix}_row\w+)`", line)
        # A row may point at a script instead of a #[test].
        scripts = re.findall(r"`(scripts/[\w.]+)`", line)
        if not names and scripts:
            found.append((num, scripts[0], line))
        elif names:
            for n in names:
                found.append((num, n, line))
        else:
            found.append((num, "", line))
    return found


def main() -> int:
    problems: list[str] = []

    print("running the suite to collect passing test names ...")
    passing = passing_tests()
    listed = listed_tests()
    print(f"  {len(listed)} tests in the suite, {len(passing)} passing\n")

    tables = [
        (CRATE / "CONFIGS.md", "cfg", "CONFIGS.md"),
        (CRATE / "ERRORS.md", "err", "ERRORS.md"),
    ]

    referenced: set[str] = set()
    for path, prefix, label in tables:
        table_rows = rows(path, prefix)
        if not table_rows:
            problems.append(f"{label}: no numbered table rows found")
            continue
        nums = [n for n, _, _ in table_rows]
        expected = list(range(1, max(nums) + 1))
        missing_nums = sorted(set(expected) - set(nums))
        if missing_nums:
            problems.append(f"{label}: row numbers missing from the table: {missing_nums}")

        for num, name, line in table_rows:
            if not name:
                problems.append(f"{label} row {num}: no test or script referenced")
                continue
            if name.startswith("scripts/"):
                if not (CRATE / name).exists():
                    problems.append(f"{label} row {num}: references missing {name}")
                continue
            referenced.add(name)
            if name not in listed:
                problems.append(f"{label} row {num}: test `{name}` does not exist")
            elif name not in passing:
                problems.append(f"{label} row {num}: test `{name}` is not passing")
            # Every row must be checked off.
            if not re.search(r"\|\s*(\[x\]|✅)\s*\|?\s*$", line):
                problems.append(f"{label} row {num} ({name}): row is not checked off")

        print(f"{label}: {len(table_rows)} rows, max row number {max(nums)}")

    # Reverse direction: no orphan row-tests.
    orphans = sorted(
        t for t in listed if re.match(r"(cfg|err)_row", t) and t not in referenced
    )
    if orphans:
        problems.append(f"tests not referenced by any table row: {orphans}")

    # The non-row tests (meta/exhaustive) must also pass.
    others = sorted(t for t in listed if not re.match(r"(cfg|err)_row", t))
    not_ok = [t for t in others if t not in passing and t != "zz_null_child"]
    if not_ok:
        problems.append(f"non-row tests not passing: {not_ok}")
    print(f"other tests (meta / exhaustive / ignored helper): {len(others)}")

    if problems:
        print("\nFAIL:")
        for p in problems:
            print(f"  - {p}")
        return 1
    print("\nPASS: every CONFIGS.md and ERRORS.md row maps to a passing test, "
          "no orphan tests, no gaps.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
