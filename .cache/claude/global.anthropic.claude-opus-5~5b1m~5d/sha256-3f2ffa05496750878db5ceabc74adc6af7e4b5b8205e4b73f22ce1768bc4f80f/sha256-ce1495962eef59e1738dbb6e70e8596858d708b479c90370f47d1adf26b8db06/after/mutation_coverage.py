#!/usr/bin/env python3
"""Mutation-coverage check for the differential test suite.

Applies one small, behaviour-changing edit at a time to translation/src/lib.rs,
re-runs the test suite, and requires the suite to FAIL.  A mutation that the
suite fails to notice is a coverage hole.

The original file is always restored.  Nothing under c_src/ is touched.
"""
import os
import re
import shutil
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
CRATE = os.path.join(HERE, "translation")
LIB = os.path.join(CRATE, "src", "lib.rs")
BAK = LIB + ".mutbak"

ORIG = open(LIB).read()


def body(name):
    """(start, end) character range of `fn <name>`'s definition."""
    start = ORIG.index("fn " + name + "(")
    # next top-level `fn ` / `#[` after it
    m = re.search(r"\n(?:#\[|/// |// -----|fn |pub |unsafe extern)", ORIG[start + 10 :])
    end = start + 10 + (m.start() if m else len(ORIG) - start - 10)
    return start, end


def generic_arms():
    """Character ranges of each `N => { ... }` arm of BTAC1C2_PredictSample."""
    s, e = body("BTAC1C2_PredictSample")
    seg = ORIG[s:e]
    marks = [
        (m.group(1), s + m.start())
        for m in re.finditer(r"\n            (\d+(?: \| \d+)*|_) => \{", seg)
    ]
    out = {}
    for i, (label, pos) in enumerate(marks):
        nxt = marks[i + 1][1] if i + 1 < len(marks) else e
        out[label] = (pos, nxt)
    return out


MUTATIONS = []  # (id, lo, hi, old, new)

# --- every arm of the generic dispatcher: shift tap 1 -> tap 3 ---------------
for label, (lo, hi) in generic_arms().items():
    seg = ORIG[lo:hi]
    if "s(psamp, i, 1)" in seg:
        MUTATIONS.append(
            (f"generic arm {label}: tap1->tap3", lo, hi, "s(psamp, i, 1)", "s(psamp, i, 3)")
        )
    elif "pred = 0;" in seg:  # the default arm
        MUTATIONS.append((f"generic arm {label}: 0->1", lo, hi, "pred = 0;", "pred = 1;"))

# --- every arm of the generic dispatcher: perturb the divisor / shift --------
for label, (lo, hi) in generic_arms().items():
    seg = ORIG[lo:hi]
    m = re.search(r">> (\d)", seg)
    if m:
        MUTATIONS.append(
            (
                f"generic arm {label}: shift {m.group(1)}->{int(m.group(1)) + 1}",
                lo,
                hi,
                m.group(0),
                f">> {int(m.group(1)) + 1}",
            )
        )
    m = re.search(r"wrapping_div\((\d+)\)", seg)
    if m:
        MUTATIONS.append(
            (
                f"generic arm {label}: div {m.group(1)}->{int(m.group(1)) * 2}",
                lo,
                hi,
                m.group(0),
                f"wrapping_div({int(m.group(1)) * 2})",
            )
        )

# --- the firfx row index ----------------------------------------------------
lo, hi = generic_arms()["12 | 13 | 14 | 15"]
MUTATIONS.append(
    ("generic firfx: row index pfcn-12 -> pfcn-12 clamped to 0", lo, hi,
     "firfx[(pfcn - 12) as usize]", "firfx[0]")
)
MUTATIONS.append(
    ("generic firfx: column 7 -> column 6", lo, hi,
     "(row[7] as c_int)", "(row[6] as c_int)")
)

# --- each specialised predictor ---------------------------------------------
for n in range(12):
    lo, hi = body(f"BTAC1C2_PredictSample_Pfn{n}")
    seg = ORIG[lo:hi]
    MUTATIONS.append(
        (f"Pfn{n}: tap1->tap3", lo, hi, "s(psamp, idx, 1)", "s(psamp, idx, 3)")
    )
    m = re.search(r">> (\d)", seg)
    if m:
        MUTATIONS.append(
            (f"Pfn{n}: shift {m.group(1)}->{int(m.group(1)) + 1}", lo, hi,
             m.group(0), f">> {int(m.group(1)) + 1}")
        )
    m = re.search(r"wrapping_div\((\d+)\)", seg)
    if m:
        MUTATIONS.append(
            (f"Pfn{n}: div {m.group(1)}->{int(m.group(1)) * 2}", lo, hi,
             m.group(0), f"wrapping_div({int(m.group(1)) * 2})")
        )

# --- the index masking helper ----------------------------------------------
lo, hi = body("s")
MUTATIONS.append(("helper s(): mask 7 -> 3", lo, hi, "& 7", "& 3"))

# --- the selector -----------------------------------------------------------
lo, hi = body("BTAC1C2_GetPredictFunc")
for n in range(12):
    MUTATIONS.append(
        (f"selector: pfcn {n} -> generic", lo, hi,
         f"{n} => BTAC1C2_PredictSample_Pfn{n},", f"{n} => BTAC1C2_PredictSample,")
    )
    MUTATIONS.append(
        (f"selector: pfcn {n} -> Pfn{(n + 1) % 12}", lo, hi,
         f"{n} => BTAC1C2_PredictSample_Pfn{n},",
         f"{n} => BTAC1C2_PredictSample_Pfn{(n + 1) % 12},")
    )
MUTATIONS.append(
    ("selector: default -> Pfn0", lo, hi,
     "_ => BTAC1C2_PredictSample,", "_ => BTAC1C2_PredictSample_Pfn0,")
)

# --- the public wrapper -----------------------------------------------------
lo, hi = body("get_predict_func")
MUTATIONS.append(
    ("public: default arm returns 1", lo, hi, "_ => {}", "_ => result = 1,")
)
MUTATIONS.append(
    ("public: pfcn 11 arm dropped", lo, hi,
     "11 => result = (fcn == BTAC1C2_PredictSample_Pfn11 as *const ()) as c_int,",
     "11 => result = 0,")
)
MUTATIONS.append(
    ("public: initial result 0 -> 1", lo, hi,
     "let mut result: c_int = 0;", "let mut result: c_int = 1;")
)

# ---------------------------------------------------------------------------

FEATURE_SETS = [["--no-default-features"],
                ["--no-default-features", "--features", "difftest"]]


def run_suite():
    """True if the whole suite passes under every feature set."""
    for fs in FEATURE_SETS:
        p = subprocess.run(
            ["cargo", "test", "--offline"] + fs,
            cwd=CRATE, capture_output=True, text=True, timeout=600,
        )
        if p.returncode != 0:
            return False, " ".join(fs)
    return True, ""


def main():
    shutil.copy(LIB, BAK)
    try:
        ok, which = run_suite()
        if not ok:
            print(f"BASELINE FAILS under {which} — fix that first")
            return 1
        print(f"baseline: PASS\nrunning {len(MUTATIONS)} mutations\n")

        survived = []
        for i, (name, lo, hi, old, new) in enumerate(MUTATIONS, 1):
            seg = ORIG[lo:hi]
            if old not in seg:
                print(f"[{i:2}/{len(MUTATIONS)}] SKIP (pattern absent): {name}")
                continue
            mutated = ORIG[:lo] + seg.replace(old, new, 1) + ORIG[hi:]
            open(LIB, "w").write(mutated)
            ok, which = run_suite()
            if ok:
                print(f"[{i:2}/{len(MUTATIONS)}] *** SURVIVED *** {name}")
                survived.append(name)
            else:
                print(f"[{i:2}/{len(MUTATIONS)}] killed   {name}")
            open(LIB, "w").write(ORIG)

        print()
        if survived:
            print(f"COVERAGE HOLES: {len(survived)} mutation(s) survived:")
            for s in survived:
                print("  -", s)
            return 1
        print(f"ALL {len(MUTATIONS)} MUTATIONS KILLED — no coverage holes found")
        return 0
    finally:
        shutil.copy(BAK, LIB)
        os.remove(BAK)


if __name__ == "__main__":
    sys.exit(main())
