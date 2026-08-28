#!/usr/bin/env python3
"""Systematically swap the operands of EVERY `addss` / `mulss` / `subss` / `divss`
call site in src/lib.rs and check whether the differential suite notices.

`ADDSS`/`MULSS` are commutative in value but NOT in NaN-payload selection: x86
returns the destination (first) operand when both are NaN. So each swap is a
real, potentially observable change, and every site's order was transcribed from
the disassembly of the C `.so`.

Output classes:
  CAUGHT      - the suite fails, i.e. the order is pinned by a test.
  EQUIVALENT  - the suite passes; printed so each one can be justified by
                argument (the site is unreachable with two distinct NaNs).
"""
import re
import shutil
import subprocess
import sys
import os

SRC = "src/lib.rs"
BAK = ".scratch/lib.rs.orderbak"


def in_comment(text, pos):
    """True if `pos` sits on a line whose first non-blank chars are `//`."""
    bol = text.rfind("\n", 0, pos) + 1
    return text[bol:pos].lstrip().startswith("//")


def find_calls(text, fname):
    """Yield (start, end, arg1, arg2) for each `fname(a, b)` with balanced parens.

    Comment lines are skipped: the module docs enumerate several call sites by
    name, and mutating prose would silently inflate the EQUIVALENT count.
    """
    out = []
    for m in re.finditer(r"\b" + fname + r"\(", text):
        if in_comment(text, m.start()):
            continue
        i = m.end()  # just after '('
        depth = 1
        comma = None
        j = i
        while j < len(text) and depth > 0:
            ch = text[j]
            if ch == "(":
                depth += 1
            elif ch == ")":
                depth -= 1
                if depth == 0:
                    break
            elif ch == "," and depth == 1 and comma is None:
                comma = j
            j += 1
        if depth != 0 or comma is None:
            continue
        a1 = text[i:comma].strip()
        a2 = text[comma + 1 : j].strip()
        out.append((m.start(), j + 1, fname, a1, a2))
    return out


def main():
    os.makedirs(".scratch", exist_ok=True)
    shutil.copy(SRC, BAK)
    original = open(BAK).read()

    # addss/mulss: commutative in value, NOT in NaN-payload selection.
    # subss/divss: NOT commutative — swapping changes the VALUE, so every such
    # site must be CAUGHT. An "equivalent" result there would mean the site is
    # unreachable, i.e. a genuine coverage hole.
    sites = (
        find_calls(original, "addss")
        + find_calls(original, "mulss")
        + find_calls(original, "subss")
        + find_calls(original, "divss")
    )
    # Deterministic order, and process from the end so offsets stay valid.
    sites.sort(key=lambda s: s[0])

    print(f"found {len(sites)} commutative call sites\n")
    caught, equivalent, broken = [], [], []

    try:
        for idx, (start, end, fname, a1, a2) in enumerate(sites):
            if a1 == a2:
                equivalent.append((idx, fname, a1, a2, "identical operands"))
                continue
            line = original[:start].count("\n") + 1
            mutated = original[:start] + f"{fname}({a2}, {a1})" + original[end:]
            open(SRC, "w").write(mutated)
            r = subprocess.run(
                ["cargo", "test", "--release", "-q"],
                capture_output=True,
                timeout=600,
            )
            label = f"L{line:<4} {fname}({a1}, {a2})"
            if r.returncode != 0:
                if b"error[" in r.stderr or b"error: could not compile" in r.stderr:
                    broken.append((label, "did not compile"))
                    print(f"  SKIP (no compile)  {label}")
                else:
                    caught.append(label)
                    print(f"  CAUGHT             {label}")
            else:
                equivalent.append((idx, fname, a1, a2, label))
                print(f"  EQUIVALENT         {label}")
    finally:
        shutil.copy(BAK, SRC)

    print()
    print(f"CAUGHT:     {len(caught)}")
    print(f"EQUIVALENT: {len(equivalent)}")
    print(f"SKIPPED:    {len(broken)}")
    if equivalent:
        print("\nSites whose operand order the suite cannot observe:")
        for e in equivalent:
            print("  -", e[-1])
    return 0


if __name__ == "__main__":
    sys.exit(main())
