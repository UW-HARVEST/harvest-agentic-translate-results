#!/usr/bin/env python3
"""Mutation campaign: prove the differential suite is actually sensitive.

For each mutation we patch `src/lib.rs`, rebuild the cdylib, run the whole
differential suite, and require that it FAILS. A mutation that survives means
the corresponding C branch is not really covered by any CONFIGS/ERRORS row.

`src/lib.rs` is always restored, even on error/interrupt.
"""
import re
import shutil
import subprocess
import sys
import os

HERE = os.path.dirname(os.path.abspath(__file__))
LIB = os.path.join(HERE, "src", "lib.rs")
BAK = LIB + ".mutbak"

# (id, description, needle, replacement)
MUTATIONS = [
    ("M01", "v2==0 guard returns 1 instead of 0 (ERRORS row 1)",
     "    if v2 == 0 {\n        return 0;\n    }",
     "    if v2 == 0 {\n        return 1;\n    }"),

    ("M02", "L1 quotient off by one (CONFIGS rows 1-7)",
     "        return v1.wrapping_div(v2);",
     "        return v1.wrapping_div(v2).wrapping_add(1);"),

    ("M03", "L2 remainder wrongly negated (CONFIGS row 9 - the subtle one)",
     "            r = v1.wrapping_rem(nv2);",
     "            r = v1.wrapping_rem(nv2).wrapping_neg();"),

    ("M04", "L2 quotient not negated (CONFIGS row 8)",
     "            q = v1.wrapping_div(nv2).wrapping_neg();",
     "            q = v1.wrapping_div(nv2);"),

    ("M05", "L3 q=0 -> q=1 (ERRORS row 2)",
     "            q = 0;\n            r = v1;",
     "            q = 1;\n            r = v1;"),

    ("M06", "L4 remainder not negated (CONFIGS rows 15-20)",
     "            r = nv1.wrapping_rem(v2).wrapping_neg();",
     "            r = nv1.wrapping_rem(v2);"),

    ("M07", "L5 quotient negated (CONFIGS rows 21-26)",
     "            q = nv1.wrapping_div(nv2);",
     "            q = nv1.wrapping_div(nv2).wrapping_neg();"),

    ("M08", "L5 remainder not negated (CONFIGS row 22 tail q+1)",
     "            r = nv1.wrapping_rem(nv2).wrapping_neg();",
     "            r = nv1.wrapping_rem(nv2);"),

    ("M09", "L6 r computed before q (comma-operator sequencing, ERRORS row 4)",
     "            q = 1;\n            r = v1.wrapping_sub(q.wrapping_mul(v2));",
     "            q = 1;\n            r = v1.wrapping_sub(0i32.wrapping_mul(v2));"),

    ("M10", "L7 -1 becomes +1 (ERRORS row 6)",
     "        q = t.wrapping_div(v2).wrapping_neg().wrapping_sub(1);",
     "        q = t.wrapping_div(v2).wrapping_neg().wrapping_add(1);"),

    ("M11", "L7 uses v1-v2 instead of v1+v2 (ERRORS row 6)",
     "        let t = v1.wrapping_add(v2).wrapping_neg(); // -(v1 + v2)",
     "        let t = v1.wrapping_sub(v2).wrapping_neg(); // -(v1 + v2)"),

    ("M12", "L8 +1 becomes -1 (ERRORS rows 7/8, overflow path)",
     "        q = t.wrapping_div(nv2).wrapping_add(1);",
     "        q = t.wrapping_div(nv2).wrapping_sub(1);"),

    ("M13", "L8 uses v1+v2 instead of v1-v2 (ERRORS row 7)",
     "        let t = v1.wrapping_sub(v2).wrapping_neg(); // -(v1 - v2)",
     "        let t = v1.wrapping_add(v2).wrapping_neg(); // -(v1 - v2)"),

    ("M14", "L9 q=1 -> q=0 (ERRORS row 5, both INT_MIN)",
     "        q = 1;\n        r = 0;",
     "        q = 0;\n        r = 0;"),

    ("M15", "tail r>=0 becomes r>0 (ERRORS row 9)",
     "    if r >= 0 {",
     "    if r > 0 {"),

    ("M16", "tail correction signs swapped (ERRORS rows 10/11)",
     "        return q.wrapping_add(if v2 > 0 { -1 } else { 1 });",
     "        return q.wrapping_add(if v2 > 0 { 1 } else { -1 });"),

    ("M17", "INT_MIN constant off by one (all three range checks)",
     "const C_INT_MIN: c_int = -0x7fffffff - 1;",
     "const C_INT_MIN: c_int = -0x7fffffff;"),

    ("M18", "v1 sign test uses > instead of >= (v1==0 misrouted)",
     "    if v1 >= 0 {",
     "    if v1 > 0 {"),

    ("M19", "L1/L2 v2 sign test uses > instead of >= (unreachable-equivalent?)",
     "        if v2 >= 0 {\n            // Truncating division, same as C's `/` for i32.",
     "        if v2 > 0 {\n            // Truncating division, same as C's `/` for i32."),

    ("M20", "wrapping_div replaced by non-wrapping semantics on L4 quotient",
     "            q = nv1.wrapping_div(v2).wrapping_neg();",
     "            q = nv1.wrapping_div(v2);"),
]


def run(cmd):
    return subprocess.run(cmd, cwd=HERE, capture_output=True, text=True, timeout=600)


def main():
    shutil.copyfile(LIB, BAK)
    original = open(BAK).read()
    results = []
    try:
        for mid, desc, needle, repl in MUTATIONS:
            if needle not in original:
                results.append((mid, desc, "NEEDLE-NOT-FOUND", ""))
                continue
            if original.count(needle) != 1:
                results.append((mid, desc, "NEEDLE-AMBIGUOUS", ""))
                continue

            open(LIB, "w").write(original.replace(needle, repl))

            b = run(["cargo", "build", "--offline", "--lib"])
            if b.returncode != 0:
                results.append((mid, desc, "BUILD-FAIL", b.stderr[-400:]))
                continue

            t = run(["cargo", "test", "--offline", "--", "--test-threads", "4"])
            if t.returncode != 0:
                names = sorted(set(re.findall(r"^\s{4}(\w+)$", t.stdout, re.M)))
                results.append((mid, desc, "KILLED", ",".join(names[:6])))
            else:
                results.append((mid, desc, "SURVIVED", ""))
            print(f"{mid} {results[-1][2]:16s} {desc}", flush=True)
    finally:
        shutil.copyfile(BAK, LIB)
        os.remove(BAK)
        run(["cargo", "build", "--offline", "--lib"])

    print("\n=== MUTATION SUMMARY ===")
    survived = [r for r in results if r[2] != "KILLED"]
    for mid, desc, status, extra in results:
        print(f"{mid}  {status:16s}  {desc}")
        if status == "KILLED" and extra:
            print(f"      killed by: {extra}")
        elif extra:
            print(f"      {extra}")
    print(f"\nkilled {len(results) - len(survived)}/{len(results)}")
    if survived:
        print("SURVIVORS (coverage gaps!):")
        for mid, desc, status, _ in survived:
            print(f"  {mid} [{status}] {desc}")
        return 1
    print("ALL MUTATIONS KILLED")
    return 0


if __name__ == "__main__":
    sys.exit(main())
