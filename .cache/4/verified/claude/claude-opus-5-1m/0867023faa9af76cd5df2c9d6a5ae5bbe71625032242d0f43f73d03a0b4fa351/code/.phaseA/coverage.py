#!/usr/bin/env python3
"""Cross-reference the differential tests against ERRORS.md / CONFIGS.md.

Scans every `tests/*.rs` for `G<g>-<nnn>` row ids (explicit ids and ranges),
attributes them to the enclosing `#[test] fn`, then rewrites the `test` column
(and, for CONFIGS.md, the checkbox) of the two tables.

A row is checked off only if (a) some test claims it and (b) that test is in
the passing set recorded by `--passed <file>` (one `file::test_name` per line,
or `*` to accept all).

Usage:
    python3 .phaseA/coverage.py --report          # coverage summary only
    python3 .phaseA/coverage.py --write --passed passed.txt
"""
import argparse
import glob
import os
import re
import sys
from collections import defaultdict

ID = re.compile(r"\bG([1-6])-(\d{1,3})\b")
# ranges: G1-001..G1-010 / ..= / - / – / — / … / " to " / "through"
RANGE = re.compile(
    r"\bG([1-6])-(\d{1,3})\s*(?:\.\.=?|--|–|—|…|\.\.\.|\s+to\s+|\s+through\s+)\s*G?\1?-?(\d{1,3})\b"
)
# slash shorthand: G4-037/038/039/040
SLASH = re.compile(r"\bG([1-6])-(\d{1,3})((?:/\d{1,3})+)")


def parse_ids(text):
    """Return the set of (group, number) ids mentioned in `text`."""
    out = set()
    for m in RANGE.finditer(text):
        g, a, b = int(m.group(1)), int(m.group(2)), int(m.group(3))
        if a <= b and b - a < 400:
            for n in range(a, b + 1):
                out.add((g, n))
    for m in SLASH.finditer(text):
        g = int(m.group(1))
        out.add((g, int(m.group(2))))
        for part in m.group(3).split("/"):
            if part:
                out.add((g, int(part)))
    for m in ID.finditer(text):
        out.add((int(m.group(1)), int(m.group(2))))
    return out


# the test name must be the item the `#[test]` attribute is attached to:
# anchored at the start of the chunk, allowing further attributes / visibility
TESTFN = re.compile(
    r"\A\s*(?:#\s*\[[^\]]*\]\s*)*(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?(?:unsafe\s+)?"
    r"(?:extern\s+\"[^\"]*\"\s+)?fn\s+([A-Za-z_][A-Za-z0-9_]*)"
)
# a `//! | `test_name` | CONFIGS|ERRORS | ids... |` row of an injected row-map
MAPROW = re.compile(
    r"^\s*//!\s*\|\s*`([A-Za-z_][A-Za-z0-9_]*)`\s*\|\s*(CONFIGS|ERRORS)\s*\|(.*)\|\s*$"
)


def scan_tests(testdir):
    """id -> set of "file::test" strings."""
    claims = defaultdict(set)
    files = sorted(glob.glob(os.path.join(testdir, "*.rs")))
    for path in files:
        stem = os.path.splitext(os.path.basename(path))[0]
        src = open(path, errors="replace").read()

        # (1) precise attribution from an injected row-map table
        mapped = set()
        for line in src.splitlines():
            m = MAPROW.match(line)
            if m:
                fn, _table, ids = m.group(1), m.group(2), m.group(3)
                for i in parse_ids(ids):
                    claims[i].add(f"{stem}::{fn}")
                    mapped.add(i)

        # (2) module-level //! comments apply to the whole file
        head = "\n".join(l for l in src.splitlines() if l.lstrip().startswith("//!"))
        file_ids = parse_ids(head) - mapped

        # (3) per-#[test] chunks
        chunks = src.split("#[test]")
        for chunk in chunks[1:]:
            m = TESTFN.match(chunk)
            if not m:
                continue
            name = m.group(1)
            for i in parse_ids(chunk):
                claims[i].add(f"{stem}::{name}")
        for i in file_ids:
            claims[i].add(f"{stem}::*")
    return claims, files


def load_passed(path):
    if not path:
        return None
    txt = open(path, errors="replace").read()
    if txt.strip() == "*":
        return "*"
    return set(l.strip() for l in txt.splitlines() if l.strip())


def is_passing(tests, passed):
    if passed is None or passed == "*":
        return True
    for t in tests:
        if t in passed:
            return True
        if t.endswith("::*"):
            pre = t[:-1]
            if any(p.startswith(pre) for p in passed):
                return True
    return False


def rewrite(table, claims, passed, has_checkbox):
    lines = open(table, errors="replace").read().splitlines()
    out = []
    rows = 0
    covered = 0
    per_group = defaultdict(lambda: [0, 0])
    for line in lines:
        m = re.match(r"^\|\s*G([1-6])-(\d{3})\s*\|", line)
        if not m:
            out.append(line)
            continue
        rows += 1
        key = (int(m.group(1)), int(m.group(2)))
        g = int(m.group(1))
        per_group[g][1] += 1
        tests = sorted(claims.get(key, ()))
        ok = bool(tests) and is_passing(tests, passed)
        if ok:
            covered += 1
            per_group[g][0] += 1
        cells = re.split(r"(?<!\\)\|", line)
        # cells[0] == '' ; last cell == '' (trailing pipe)
        label = ", ".join(f"`{t}`" for t in tests) if tests else "—"
        if has_checkbox:
            # ... | source | test | [ ] |
            cells[-3] = f" {label} "
            cells[-2] = " [x] " if ok else " [ ] "
        else:
            cells[-2] = f" {label} "
        out.append("|".join(cells))
    open(table, "w").write("\n".join(out) + "\n")
    return rows, covered, per_group


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--write", action="store_true")
    ap.add_argument("--passed", default=None)
    ap.add_argument("--report", action="store_true")
    a = ap.parse_args()

    claims, files = scan_tests("tests")
    passed = load_passed(a.passed)

    print(f"scanned {len(files)} test files; {len(claims)} distinct row ids claimed")
    for table, cb in (("ERRORS.md", False), ("CONFIGS.md", True)):
        if not os.path.exists(table):
            print(f"  !! {table} missing")
            continue
        if a.write:
            rows, cov, pg = rewrite(table, claims, passed, cb)
        else:
            # dry run: count only
            rows = cov = 0
            pg = defaultdict(lambda: [0, 0])
            for line in open(table, errors="replace"):
                m = re.match(r"^\|\s*G([1-6])-(\d{3})\s*\|", line)
                if not m:
                    continue
                rows += 1
                g = int(m.group(1))
                pg[g][1] += 1
                key = (g, int(m.group(2)))
                tests = claims.get(key, ())
                if tests and is_passing(tests, passed):
                    cov += 1
                    pg[g][0] += 1
        pct = 100.0 * cov / rows if rows else 0.0
        print(f"\n{table}: {cov}/{rows} rows covered ({pct:.1f}%)")
        for g in sorted(pg):
            c, t = pg[g]
            print(f"   G{g}: {c}/{t}")
        missing = []
        for line in open(table, errors="replace"):
            m = re.match(r"^\|\s*G([1-6])-(\d{3})\s*\|", line)
            if not m:
                continue
            key = (int(m.group(1)), int(m.group(2)))
            tests = claims.get(key, ())
            if not (tests and is_passing(tests, passed)):
                missing.append(f"G{m.group(1)}-{m.group(2)}")
        if missing:
            print(f"   UNCOVERED ({len(missing)}): {' '.join(missing[:80])}"
                  + (" ..." if len(missing) > 80 else ""))
    return 0


if __name__ == "__main__":
    sys.exit(main())
