#!/usr/bin/env python3
"""Mechanically verify that every ERRORS.md / CONFIGS.md row has a test.

Coverage is declared in the test sources with annotation fields:

    rows:     &[12, 13]     in tests/phase_c_*.rs   -> ERRORS.md  rows
    cfg_rows: &[7]          in tests/phase_b_*.rs   -> CONFIGS.md rows

Usage:
    python3 check_coverage.py            # report
    python3 check_coverage.py --tick     # also tick CONFIGS.md checkboxes
"""
import glob
import re
import sys

ROOT = "."


def table_rows(path):
    """Row numbers present in a markdown table in `path`."""
    out = []
    for line in open(path, encoding="utf-8"):
        m = re.match(r"^\|\s*(\d+)\s*\|", line)
        if m:
            out.append(int(m.group(1)))
    return out


def declared(patterns, field):
    """Row numbers declared via `field: &[...]` across the given globs."""
    got = {}
    rx = re.compile(field + r"\s*:\s*&\[([0-9,\s]*)\]")
    for g in patterns:
        for path in sorted(glob.glob(g)):
            for n in rx.finditer(open(path, encoding="utf-8").read()):
                for tok in n.group(1).replace(",", " ").split():
                    got.setdefault(int(tok), set()).add(path.split("/")[-1])
    return got


def report(name, table_path, globs, field):
    rows = table_rows(table_path)
    if not rows:
        print(f"{name}: NO ROWS FOUND in {table_path}")
        return False, {}
    expected = set(range(1, len(rows) + 1))
    seq_ok = rows == sorted(expected)
    cov = declared(globs, field)
    covered = set(cov) & expected
    missing = sorted(expected - covered)
    extra = sorted(set(cov) - expected)

    print(f"=== {name} ({table_path}) ===")
    print(f"  rows in table          : {len(rows)}  (1..{len(rows)})")
    print(f"  numbering sequential   : {seq_ok}")
    print(f"  rows covered by tests  : {len(covered)}")
    print(f"  rows MISSING coverage  : {len(missing)}")
    if missing:
        print(f"    -> {compress(missing)}")
    if extra:
        print(f"  annotations out of range: {extra}")
    files = sorted({f for s in cov.values() for f in s})
    print(f"  declaring files        : {files}")
    ok = seq_ok and not missing and not extra
    print(f"  RESULT: {'OK' if ok else 'INCOMPLETE'}")
    print()
    return ok, cov


def compress(ns):
    """[1,2,3,7] -> '1-3,7'"""
    out, i = [], 0
    while i < len(ns):
        j = i
        while j + 1 < len(ns) and ns[j + 1] == ns[j] + 1:
            j += 1
        out.append(str(ns[i]) if i == j else f"{ns[i]}-{ns[j]}")
        i = j + 1
    return ",".join(out)


def tick(path, cov):
    """Turn `| [ ] |` into `| [x] |` for covered rows."""
    lines = open(path, encoding="utf-8").read().split("\n")
    n = 0
    for i, line in enumerate(lines):
        m = re.match(r"^\|\s*(\d+)\s*\|", line)
        if m and int(m.group(1)) in cov and line.rstrip().endswith("[ ] |"):
            lines[i] = re.sub(r"\[ \] \|\s*$", "[x] |", line.rstrip())
            n += 1
    open(path, "w", encoding="utf-8").write("\n".join(lines))
    print(f"ticked {n} checkboxes in {path}")


def main():
    ok1, _ = report(
        "ERRORS.md / Phase C", f"{ROOT}/ERRORS.md", [f"{ROOT}/tests/phase_c_*.rs"], "rows"
    )
    ok2, cfgcov = report(
        "CONFIGS.md / Phase B",
        f"{ROOT}/CONFIGS.md",
        [f"{ROOT}/tests/phase_b_*.rs"],
        "cfg_rows",
    )
    if "--tick" in sys.argv:
        tick(f"{ROOT}/CONFIGS.md", set(cfgcov))
    print("OVERALL:", "OK" if (ok1 and ok2) else "INCOMPLETE")
    return 0 if (ok1 and ok2) else 1


if __name__ == "__main__":
    sys.exit(main())
