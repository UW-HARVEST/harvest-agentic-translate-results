#!/usr/bin/env python3
"""Negative control for the differential suite.

Applies one literal mutation at a time to translation/src/lib.rs, rebuilds the
Rust cdylib, and re-runs the whole test suite. Each mutation MUST make the suite
fail (non-zero cargo exit status, which also covers SIGSEGV / timeout, not just
`panicked`). A mutation the suite still passes is either an equivalent mutant or
a blind spot; equivalent mutants are listed explicitly with their proof.

src/lib.rs is restored after every mutation and on any exit.
"""

import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
LIB = ROOT / "src" / "lib.rs"

# (name, old_literal, new_literal, expectation)
#   expectation "catch"      -> the suite must fail
#   expectation "equivalent" -> provably identical observable behaviour; the
#                               suite is expected to still pass
MUTATIONS = [
    (
        "swap-final-sqd-accumulation",
        "let sqd = addss(mulss(mulss(4.0f32, dxy), dxy), acc);",
        "let sqd = addss(acc, mulss(mulss(4.0f32, dxy), dxy));",
        "catch",
    ),
    (
        "swap-dy2-dx2-in-lambda-sum",
        "mulss(0.5f32, addss(addss(dy2, dx2), sqrtf(clamped)))",
        "mulss(0.5f32, addss(addss(dx2, dy2), sqrtf(clamped)))",
        # EQUIVALENT (masked downstream). The order of `addss(dy2, dx2)` is only
        # observable when BOTH are NaN with different payloads. That requires the
        # compare to be unordered, i.e. the `else` arm, where `dest[0] = dxy` and
        # `dest[1] = subss(dx2, lambda)` -- and `subss` returns the quieted
        # *dest* when dest is a NaN, so `dx2` overrides `lambda` entirely and the
        # payload chosen inside lambda never reaches memory. If only one is NaN,
        # both orders yield that same NaN. Confirmed over the full 36^3
        # special-value cross product in tests/exhaustive.rs.
        "equivalent",
    ),
    (
        "swap-subss-acc-operands",
        "acc = subss(acc, mulss(two_dx2, dy2));",
        "acc = subss(mulss(two_dx2, dy2), acc);",
        "catch",
    ),
    (
        "swap-addss-acc-dx2sq",
        "acc = addss(acc, mulss(dx2, dx2));",
        "acc = addss(mulss(dx2, dx2), acc);",
        # EQUIVALENT (same NaN, or masked). If exactly one of dx2/dy2 is NaN,
        # `acc` and `dx2*dx2` are the SAME quieted NaN, so the roles are
        # interchangeable. If both are NaN, `acc`'s payload is masked: the final
        # `addss(4*dxy*dxy, acc)` and then `addss(addss(dy2,dx2), sqrt)` /
        # `subss(dx2, lambda)` all take their NaN from an earlier dest operand.
        # Confirmed over the full 36^3 cross product in tests/exhaustive.rs.
        "equivalent",
    ),
    (
        "swap-lambda-sum-with-sqrt",
        "addss(addss(dy2, dx2), sqrtf(clamped))",
        "addss(sqrtf(clamped), addss(dy2, dx2))",
        "catch",
    ),
    (
        "clamp-nan-to-zero",
        "let clamped = if 0.0f32 > sqd { 0.0f32 } else { sqd };",
        "let clamped = if !(sqd >= 0.0f32) { 0.0f32 } else { sqd };",
        "catch",
    ),
    (
        "unsigned-loop-bound",
        "while i < count {",
        "while (i as u32) < (count as u32) {",
        "catch",
    ),
    (
        "arm-select-le",
        "if s0 < s1 {",
        "if s0 <= s1 {",
        "catch",
    ),
    (
        "arm-select-gt",
        "if s0 < s1 {",
        "if s0 > s1 {",
        "catch",
    ),
    (
        "loop-off-by-one",
        "while i < count {",
        "while i <= count {",
        "catch",
    ),
    (
        "src-stride-two",
        "src = src.add(3);",
        "src = src.add(2);",
        "catch",
    ),
    (
        "dest-stride-three",
        "dest = dest.add(2);",
        "dest = dest.add(3);",
        "catch",
    ),
    (
        "swap-store-order-if-arm",
        "*dest = subss(dx2, lambda);\n            *dest.add(1) = dxy;",
        "*dest = dxy;\n            *dest.add(1) = subss(dx2, lambda);",
        "catch",
    ),
    (
        "subss-operands-reversed",
        "*dest.add(1) = subss(dx2, lambda);",
        "*dest.add(1) = subss(lambda, dx2);",
        "catch",
    ),
    (
        "sqrtf-drop-nan-branch",
        "fn sqrtf(x: f32) -> f32 {\n    if x.is_nan() {\n        quiet(x)\n    } else {\n        x.sqrt()\n    }\n}",
        "fn sqrtf(x: f32) -> f32 {\n    x.sqrt()\n}",
        "equivalent",  # SQRTSS already quiets a NaN operand identically
    ),
    (
        "addss-drop-nan-modelling",
        "fn addss(dest: f32, src: f32) -> f32 {\n    if dest.is_nan() {\n        quiet(dest)\n    } else if src.is_nan() {\n        quiet(src)\n    } else {\n        dest + src\n    }\n}",
        "fn addss(dest: f32, src: f32) -> f32 {\n    dest + src\n}",
        # EQUIVALENT UNDER THIS rustc/LLVM. `a + b` lowers to a single ADDSS and
        # LLVM currently happens to assign the same operand roles the C's -O0
        # codegen uses. The explicit modelling is kept deliberately: it PINS the
        # roles instead of depending on register allocation, and the suite does
        # detect a wrong role (see swap-final-sqd-accumulation, caught).
        "equivalent",
    ),
    (
        "mulss-drop-nan-modelling",
        "fn mulss(dest: f32, src: f32) -> f32 {\n    if dest.is_nan() {\n        quiet(dest)\n    } else if src.is_nan() {\n        quiet(src)\n    } else {\n        dest * src\n    }\n}",
        "fn mulss(dest: f32, src: f32) -> f32 {\n    dest * src\n}",
        # EQUIVALENT. Every mulss in the kernel either squares one value
        # (mulss(dy2,dy2), mulss(dx2,dx2)) or has a never-NaN constant as dest
        # (4.0f, 0.5f), so the NaN winner is the same for both roles. The one
        # two-variable case, mulss(two_dx2, dy2), is masked by the following
        # subss(acc, ..) whose dest `acc` already carries that NaN.
        "equivalent",
    ),
    (
        "subss-drop-nan-modelling",
        "fn subss(dest: f32, src: f32) -> f32 {\n    if dest.is_nan() {\n        quiet(dest)\n    } else if src.is_nan() {\n        quiet(src)\n    } else {\n        dest - src\n    }\n}",
        "fn subss(dest: f32, src: f32) -> f32 {\n    dest - src\n}",
        # EQUIVALENT. Subtraction is non-commutative, so LLVM cannot swap the
        # operands: `a - b` must emit SUBSS with `a` as dest, which is exactly
        # what the model encodes.
        "equivalent",
    ),
    (
        "two-dx2-via-multiply",
        "let two_dx2 = addss(dx2, dx2);",
        "let two_dx2 = mulss(2.0f32, dx2);",
        # dest role differs (2.0f vs dx2) but 2.0f is never NaN, so the NaN
        # winner is dx2 either way, and 2*x == x+x exactly for every float.
        "equivalent",
    ),
    (
        "clamp-ge-instead-of-gt",
        "let clamped = if 0.0f32 > sqd { 0.0f32 } else { sqd };",
        "let clamped = if 0.0f32 >= sqd { 0.0f32 } else { sqd };",
        # Differs only when sqd == -0.0, which is unreachable: the final
        # accumulation is addss(4*dxy*dxy, acc) and 4*dxy*dxy is never -0.0
        # (mulss(4.0f, +-0.0) then squaring yields +0.0), and +0.0 + x is never
        # -0.0. Proven exhaustively by tests/negative_zero_sqd.rs.
        "equivalent",
    ),
]


def run(cmd, **kw):
    return subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True, **kw)


def main():
    backup = Path(tempfile.mkstemp(suffix=".rs")[1])
    shutil.copy(LIB, backup)
    results = []
    try:
        original = LIB.read_text()
        for name, old, new, expect in MUTATIONS:
            if original.count(old) != 1:
                results.append((name, expect, f"PATTERN-NOT-UNIQUE ({original.count(old)})"))
                continue
            LIB.write_text(original.replace(old, new, 1))
            build = run(["cargo", "build", "--release", "--quiet"])
            if build.returncode != 0:
                results.append((name, expect, "WONT-COMPILE"))
                continue
            test = run(["timeout", "300", "cargo", "test", "--release", "--quiet"])
            caught = test.returncode != 0
            if expect == "catch":
                results.append((name, expect, "caught" if caught else "*** BLIND ***"))
            else:
                results.append(
                    (name, expect, "equivalent (still passes)" if not caught else "*** UNEXPECTEDLY CAUGHT ***")
                )
    finally:
        shutil.copy(backup, LIB)
        backup.unlink(missing_ok=True)
        run(["cargo", "build", "--release", "--quiet"])

    print(f"{'MUTATION':<32} {'EXPECT':<12} RESULT")
    print(f"{'-' * 32} {'-' * 12} ------")
    bad = 0
    for name, expect, res in results:
        print(f"{name:<32} {expect:<12} {res}")
        if "***" in res or "PATTERN" in res or res == "WONT-COMPILE":
            bad += 1
    print()
    if bad:
        print(f"FAIL: {bad} mutation(s) not handled as expected")
        return 1
    print(f"OK: all {len(results)} mutations behaved as expected")
    return 0


if __name__ == "__main__":
    sys.exit(main())
