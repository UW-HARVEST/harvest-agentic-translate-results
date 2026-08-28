#!/usr/bin/env python3
"""Anti-vacuity check for the C-vs-Rust differential suite.

Injects a known bug into ``src/lib.rs``, runs the suite, and requires it to
FAIL. A suite that passes on a mutant is not verifying anything.

Each mutant is classified:

* ``caught``      - a real behavioural change; the suite MUST fail.
* ``equivalent``  - provably produces identical results for every input, so the
                    suite MUST pass. These are kept because they document
                    exactly which parts of the transcribed operand order are
                    load-bearing and which are free. A mutant marked
                    ``equivalent`` that suddenly gets caught means the reasoning
                    below is wrong and needs revisiting.

``src/lib.rs`` is always restored, including on error or interrupt.
"""

from __future__ import annotations

import re
import shutil
import subprocess
import sys
from pathlib import Path

CRATE = Path(__file__).resolve().parent.parent
SRC = CRATE / "src" / "lib.rs"
SCRATCH = CRATE / "target" / "mutation"

CAUGHT = "caught"
EQUIVALENT = "equivalent"


def sub(pattern: str, repl: str, count: int = 1):
    """Literal-ish substitution helper (pattern is a regex, repl is literal)."""

    def apply(text: str) -> str:
        new, n = re.subn(pattern, lambda _m: repl, text, count=count)
        if n == 0:
            raise RuntimeError(f"mutation pattern did not match: {pattern!r}")
        return new

    return apply


def literal(old: str, new: str, count: int = 1):
    def apply(text: str) -> str:
        if text.count(old) < count:
            raise RuntimeError(f"mutation text not found: {old!r}")
        return text.replace(old, new, count)

    return apply


# The exact protanopia body, used by the store-order mutant.
PROTANOPIA_BODY = """    let (r, g, b) = (load(red), load(green), load(blue));

    // t3 + (t1 + t2)
    let t1 = mul(r, 0.17055699213417f32);
    let t2 = mul(0.82944301379913f32, g);
    let t3 = mul(2.91188E-9f32, b);
    store(red, add(t3, add(t1, t2)));

    // (t2 + t1) - t3
    let t1 = mul(r, 0.17055699092998f32);
    let t2 = mul(0.82944300785005f32, g);
    let t3 = mul(5.98679E-10f32, b);
    store(green, sub(add(t2, t1), t3));

    // (t2 + t1) + B
    let t1 = mul(r, -0.00451714424166f32);
    let t2 = mul(0.00451714427397f32, g);
    store(blue, add(add(t2, t1), b));"""

PROTANOPIA_REVERSED_STORES = """    let (r, g, b) = (load(red), load(green), load(blue));
    let nr = { let t1 = mul(r, 0.17055699213417f32); let t2 = mul(0.82944301379913f32, g);
               let t3 = mul(2.91188E-9f32, b); add(t3, add(t1, t2)) };
    let ng = { let t1 = mul(r, 0.17055699092998f32); let t2 = mul(0.82944300785005f32, g);
               let t3 = mul(5.98679E-10f32, b); sub(add(t2, t1), t3) };
    let nb = { let t1 = mul(r, -0.00451714424166f32); let t2 = mul(0.00451714427397f32, g);
               add(add(t2, t1), b) };
    // MUTANT: stores reversed (C stores Red, Green, Blue).
    store(blue, nb);
    store(green, ng);
    store(red, nr);"""

MUTANTS: list[tuple[str, str, object]] = [
    (
        "M01 coefficient digit changed (protanopia red, G coeff)",
        CAUGHT,
        literal("0.82944301379913f32", "0.82944311379913f32"),
    ),
    (
        "M02 addss operand order swapped (protanopia red)",
        CAUGHT,
        literal("store(red, add(t3, add(t1, t2)));", "store(red, add(add(t1, t2), t3));"),
    ),
    (
        # mulss is commutative, and every `mul` in the translation has one
        # constant (never-NaN) operand, so the NaN-tie rule that makes operand
        # order observable for add/sub can never fire here. Provably equivalent.
        "M03 mulss operand order swapped (protanopia red, G term)",
        EQUIVALENT,
        literal(
            "let t2 = mul(0.82944301379913f32, g);",
            "let t2 = mul(g, 0.82944301379913f32);",
        ),
    ),
    (
        "M04 re-associated: t3+(t1+t2) -> t1+(t2+t3)",
        CAUGHT,
        literal("store(red, add(t3, add(t1, t2)));", "store(red, add(t1, add(t2, t3)));"),
    ),
    (
        "M05 out-of-range impairment dispatches to protanopia",
        CAUGHT,
        literal("        _ => {}", "        _ => protanopia(r, g, b),"),
    ),
    (
        "M06 impairment compared as signed (negatives -> protanopia)",
        CAUGHT,
        literal(
            "match impairment {",
            "match if (impairment as i32) < 0 { 0 } else { impairment } {",
        ),
    ),
    (
        "M07 impairment masked with 3 (aliases 3 -> tritanopia)",
        CAUGHT,
        literal("match impairment {", "match impairment & 3 {"),
    ),
    (
        "M08 protanopia stores reversed (Blue, Green, Red)",
        CAUGHT,
        literal(PROTANOPIA_BODY, PROTANOPIA_REVERSED_STORES),
    ),
    (
        "M09 protanopia re-reads *green after writing *red",
        CAUGHT,
        literal(
            "let t1 = mul(r, 0.17055699092998f32);",
            "let g = load(green);\n    let t1 = mul(r, 0.17055699092998f32);",
        ),
    ),
    (
        "M10 protanopia green computed in f64",
        CAUGHT,
        literal(
            "store(green, sub(add(t2, t1), t3));",
            "*green = ((t2 as f64 + t1 as f64) - t3 as f64) as f32;",
        ),
    ),
    (
        "M11 fused multiply-add contraction (protanopia red)",
        CAUGHT,
        literal(
            "store(red, add(t3, add(t1, t2)));",
            "*red = f32::mul_add(2.91188E-9f32, b, add(t1, t2));",
        ),
    ),
    (
        # x * 1.0 is the identity on every f32 bit pattern except that it
        # quietens an sNaN -- and the following addss quietens it anyway, so the
        # stored result is unchanged for every input. Provably equivalent.
        "M12 tritanopia red: raw R replaced by mul(r, 1.0)",
        EQUIVALENT,
        literal(
            "store(red, sub(add(t_g, r), t_b));",
            "store(red, sub(add(t_g, mul(r, 1.0f32)), t_b));",
        ),
    ),
    (
        # IEEE-754 defines x - y as x + (-y), and negating the *constant* factor
        # yields exactly the negated product (round-to-nearest is sign
        # symmetric). NaN propagation returns the NaN operand unchanged in both
        # forms. Provably equivalent -- i.e. GCC's `subss` and LLVM's preferred
        # "add a negated constant" rewrite really are interchangeable here.
        "M13 tritanopia red: sub rewritten as add of negated coefficient",
        EQUIVALENT,
        lambda text: literal(
            "let t_b = mul(0.12739886341072f32, b);\n    store(red, sub(add(t_g, r), t_b));",
            "let t_b = mul(-0.12739886341072f32, b);\n    store(red, add(add(t_g, r), t_b));",
        )(text),
    ),
    (
        "M14 protanopia outputs clamped to [0,1]",
        CAUGHT,
        literal(
            "store(blue, add(add(t2, t1), b));",
            "*blue = add(add(t2, t1), b);\n    *red = (*red).clamp(0.0, 1.0);"
            " *green = (*green).clamp(0.0, 1.0); *blue = (*blue).clamp(0.0, 1.0);",
        ),
    ),
    (
        "M15 deuteranopia and tritanopia swapped in the dispatch",
        CAUGHT,
        lambda text: literal(
            "cbTritanopia => tritanopia(r, g, b),",
            "cbTritanopia => deuteranopia(r, g, b),",
        )(
            literal(
                "cbDeuteranopia => deuteranopia(r, g, b),",
                "cbDeuteranopia => tritanopia(r, g, b),",
            )(text)
        ),
    ),
    (
        "M16 NaN canonicalised on output (protanopia red)",
        CAUGHT,
        literal(
            "store(red, add(t3, add(t1, t2)));",
            "{ let v = add(t3, add(t1, t2));"
            " *red = if v.is_nan() { f32::NAN } else { v }; }",
        ),
    ),
    (
        "M17 signed zero normalised (protanopia red)",
        CAUGHT,
        literal(
            "store(red, add(t3, add(t1, t2)));",
            "{ let v = add(t3, add(t1, t2));"
            " *red = if v == 0.0 { 0.0 } else { v }; }",
        ),
    ),
    (
        "M18 NULL guard added (the C has none)",
        CAUGHT,
        literal(
            "match impairment {",
            "if r.is_null() || g.is_null() || b.is_null() { return; }\n    match impairment {",
        ),
    ),
    (
        "M19 subnormal inputs flushed to zero (protanopia)",
        CAUGHT,
        literal(
            "let (r, g, b) = (load(red), load(green), load(blue));",
            "let (r, g, b) = (load(red), load(green), load(blue));\n"
            "    let fz = |v: f32| if v != 0.0 && v.abs() < f32::MIN_POSITIVE { 0.0 } else { v };\n"
            "    let (r, g, b) = (fz(r), fz(g), fz(b));",
        ),
    ),
    (
        "M20 exported symbol renamed (ABI break)",
        CAUGHT,
        literal(
            'pub unsafe extern "C" fn colourblind(',
            '#[export_name = "colourblind_v2"]\npub unsafe extern "C" fn colourblind(',
        ),
    ),
    (
        "M21 deuteranopia green add order swapped",
        CAUGHT,
        literal(
            "store(green, sub(add(t2, t1), t3));\n\n    // (t2 + t1) + B\n"
            "    let t1 = mul(r, -0.02785538261323f32);",
            "*green = sub(add(t1, t2), t3);\n\n    // (t2 + t1) + B\n"
            "    let t1 = mul(r, -0.02785538261323f32);",
        ),
    ),
    (
        "M22 tritanopia green add order swapped",
        CAUGHT,
        literal(
            "let t3 = mul(0.12609070101523f32, b);\n    store(green, add(t3, add(t1, t2)));",
            "let t3 = mul(0.12609070101523f32, b);\n    store(green, add(add(t1, t2), t3));",
        ),
    ),
    (
        "M23 protanopia blue: raw B addend dropped from the sum order",
        CAUGHT,
        literal(
            "store(blue, add(add(t2, t1), b));",
            "store(blue, add(b, add(t2, t1)));",
        ),
    ),
    (
        # The C writes 0.87390929928361f for tritanopia's green row and
        # 0.87390929725848f for its blue row, but BOTH decimal literals round to
        # the same f32 (0x3F5FB885) -- which is why GCC emitted a single
        # .rodata entry shared by the two `mulss`es. The same holds for the other
        # six near-duplicate pairs in lib.c. So swapping one for the other cannot
        # change any result. Provably equivalent.
        "M24 tritanopia blue G coeff <- green row's G coeff (same f32)",
        EQUIVALENT,
        literal(
            "let t2 = mul(0.87390929725848f32, g);",
            "let t2 = mul(0.87390929928361f32, g);",
        ),
    ),
    (
        "M25 green and blue outputs swapped (protanopia)",
        CAUGHT,
        lambda text: literal(
            "store(green, sub(add(t2, t1), t3));", "store(blue, sub(add(t2, t1), t3));"
        )(text).replace("store(blue, add(add(t2, t1), b));", "store(green, add(add(t2, t1), b));", 1),
    ),
    (
        # Unlike M24, these two coefficients really are different f32 values
        # (0x3E011DEC vs 0x3E0274D9), so this one must be caught.
        "M26 tritanopia blue B coeff <- red row's B coeff (different f32)",
        CAUGHT,
        literal(
            "let t3 = mul(0.12609070067115f32, b);",
            "let t3 = mul(0.12739886341072f32, b);",
        ),
    ),
    (
        "M27 protanopia red B coeff sign flipped (tiny 2.9e-9 term)",
        CAUGHT,
        literal(
            "let t3 = mul(2.91188E-9f32, b);",
            "let t3 = mul(-2.91188E-9f32, b);",
        ),
    ),
    (
        "M28 deuteranopia green: subtraction turned into addition",
        CAUGHT,
        literal(
            "let t3 = mul(1.758327E-9f32, b);\n    store(green, sub(add(t2, t1), t3));",
            "let t3 = mul(1.758327E-9f32, b);\n    store(green, add(add(t2, t1), t3));",
        ),
    ),
    (
        "M29 tritanopia red drops the R term entirely",
        CAUGHT,
        literal(
            "store(red, sub(add(t_g, r), t_b));",
            "store(red, sub(t_g, t_b));",
        ),
    ),
    (
        "M30 protanopia blue drops the raw B addend",
        CAUGHT,
        literal(
            "store(blue, add(add(t2, t1), b));",
            "store(blue, add(t2, t1));",
        ),
    ),
]


def run_suite() -> tuple[bool, str]:
    """Returns (failed, failing-test-names)."""
    proc = subprocess.run(
        ["cargo", "test", "--release"],
        cwd=CRATE,
        capture_output=True,
        text=True,
        timeout=600,
    )
    out = proc.stdout + proc.stderr
    failed = bool(
        re.search(r"^test result: FAILED", out, re.M)
        or re.search(r"^error(\[|:)", out, re.M)
    )
    names = sorted(set(re.findall(r"^test (\S+) \.\.\. FAILED", out, re.M)))
    if failed and not names:
        names = ["<compile error>"]
    return failed, " ".join(names)


def main() -> int:
    SCRATCH.mkdir(parents=True, exist_ok=True)
    backup = SCRATCH / "lib.rs.orig"
    shutil.copy2(SRC, backup)
    original = SRC.read_text()

    problems: list[str] = []
    print("=== mutation testing ===")
    print(f"    {len(MUTANTS)} mutants; 'caught' must fail the suite, "
          f"'equivalent' must pass it\n")
    try:
        for name, expectation, apply in MUTANTS:
            try:
                mutated = apply(original)
            except RuntimeError as exc:
                print(f"  [{name}]\n      BROKEN MUTATION: {exc}")
                problems.append(f"{name}: broken mutation")
                continue
            if mutated == original:
                print(f"  [{name}]\n      NO-OP MUTATION (file unchanged)")
                problems.append(f"{name}: no-op mutation")
                continue
            SRC.write_text(mutated)
            failed, who = run_suite()
            if expectation == CAUGHT:
                if failed:
                    print(f"  [{name}]\n      CAUGHT by: {who}")
                else:
                    print(f"  [{name}]\n      *** NOT CAUGHT — suite is blind to this bug ***")
                    problems.append(f"{name}: not caught")
            else:  # EQUIVALENT
                if failed:
                    print(
                        f"  [{name}]\n      *** UNEXPECTEDLY CAUGHT by: {who} —"
                        f" the equivalence argument is wrong ***"
                    )
                    problems.append(f"{name}: equivalent mutant was caught")
                else:
                    print(f"  [{name}]\n      equivalent, as expected (suite passes)")
    finally:
        SRC.write_text(original)
        print("\n=== src/lib.rs restored ===")

    n_caught = sum(1 for _, e, _ in MUTANTS if e == CAUGHT)
    n_equiv = len(MUTANTS) - n_caught
    print(f"    behavioural mutants: {n_caught}   equivalent mutants: {n_equiv}")
    if problems:
        print("\nFAIL:")
        for p in problems:
            print(f"  - {p}")
        return 1
    print("\nPASS: every behavioural mutant was detected and every equivalent "
          "mutant was accepted.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
