#!/usr/bin/env python3
"""Negative control for the differential suite.

Applies one small mutation at a time to `src/lib.rs`, runs the differential
tests, and reports whether the suite caught it.  A mutation that is *not* caught
means the suite has a blind spot (or the mutation is genuinely semantics
preserving, which is then noted explicitly).

Usage:  python3 mutate.py [test-target ...]
"""
import subprocess
import sys
import shutil
import os

ORIG = ".mutbak/lib.rs.orig"
SRC = "src/lib.rs"

# (name, old, new, expected: "caught" | "equivalent")
MUTATIONS = [
    (
        "fix-the-C-typo (h<120 && h<180  ->  h>=120 && h<180)",
        "} else if h < 120.0 && h < 180.0 {",
        "} else if h >= 120.0 && h < 180.0 {",
        "caught",
    ),
    (
        "swap-x-product-operands (NaN preference of `c * term`)",
        "    let x: c_float = mul_ss(\n"
        "        sub_ss(1.0, fabsf(sub_ss(fmodf(div_ss(h, 60.0), 2.0), 1.0))),\n"
        "        c,\n"
        "    );",
        "    let x: c_float = mul_ss(\n"
        "        c,\n"
        "        sub_ss(1.0, fabsf(sub_ss(fmodf(div_ss(h, 60.0), 2.0), 1.0))),\n"
        "    );",
        "caught",
    ),
    (
        "plain-arithmetic-instead-of-ss-helpers (drops NaN payload control)",
        "    let raw = black_box(src1 + src2);",
        "    let raw = black_box(src2 + src1);",
        "equivalent",  # addition is commutative once the NaN cases are handled
    ),
    (
        "no-quieting (forward the signalling NaN unchanged)",
        "    c_float::from_bits(v.to_bits() | 0x0040_0000)",
        "    v",
        "caught",
    ),
    (
        "rust-%-instead-of-fmodf",
        "fmodf(div_ss(h, 60.0), 2.0)",
        "(div_ss(h, 60.0) % 2.0)",
        # Rust's `%` on f32 lowers to `frem`, which LLVM lowers to a call to the
        # very same `fmodf`, so this really is the same code.
        "equivalent",
    ),
    (
        "s==0 test uses bit equality (so -0.0 no longer takes the early return)",
        "    if s == 0.0 {",
        "    if s.to_bits() == 0 {",
        "caught",
    ),
    (
        "s!=0 test also accepts NaN into the early return",
        "    if s == 0.0 {",
        "    if s == 0.0 || s.is_nan() {",
        "caught",
    ),
    (
        "reintroduce the literal `1.0f *` on m as a mul_ss with m first",
        "    let m: c_float = sub_ss(l, mul_ss(0.5, c));",
        "    let m: c_float = mul_ss(sub_ss(l, mul_ss(0.5, c)), 1.0);",
        "equivalent",  # multiplying by 1.0f cannot change a value
    ),
    (
        "2.0*l computed as l+l (as gcc actually emits)",
        "fabsf(sub_ss(mul_ss(2.0, l), 1.0))",
        "fabsf(sub_ss(add_ss(l, l), 1.0))",
        "equivalent",  # this is literally what the C compiles to
    ),
    (
        "wrong sector: swap the stores of branch 5 and 6",
        "        store(add_ss(x, m), m, add_ss(c, m));",
        "        store(m, add_ss(x, m), add_ss(c, m));",
        "caught",
    ),
    (
        "hue upper bound 360 -> inclusive",
        "    } else if h >= 300.0 && h < 360.0 {",
        "    } else if h >= 300.0 && h <= 360.0 {",
        "caught",
    ),
    (
        "fabsf implemented with f32::abs (identical) ",
        "    c_float::from_bits(v.to_bits() & 0x7fff_ffff)",
        "    if v.is_nan() { c_float::from_bits(v.to_bits() & 0x7fff_ffff) } else { v.abs() }",
        "equivalent",
    ),
    (
        "store order reversed (b, g, r)",
        "        *dest.add(0) = r;\n        *dest.add(1) = g;\n        *dest.add(2) = b;",
        "        *dest.add(2) = b;\n        *dest.add(1) = g;\n        *dest.add(0) = r;",
        "equivalent",  # all reads precede all writes, so order cannot matter
    ),
    (
        "premature store before l is read (breaks dest == src+2 aliasing)",
        "    let l: c_float = unsafe { *src.add(2) };",
        "    unsafe { *dest.add(0) = 0.0 };\n"
        "    let l: c_float = unsafe { *src.add(2) };",
        "caught",
    ),
    (
        "swap add_ss(c, m) -> add_ss(m, c) in branch 1 (NaN preference)",
        "        store(add_ss(c, m), add_ss(x, m), m);",
        "        store(add_ss(m, c), add_ss(x, m), m);",
        "caught",
    ),
    (
        "swap x and c in branch 2's stores",
        "        store(add_ss(x, m), add_ss(c, m), m);",
        "        store(add_ss(c, m), add_ss(x, m), m);",
        "caught",
    ),
    (
        "sector lower bound 60 -> 61",
        "    } else if h >= 60.0 && h < 120.0 {",
        "    } else if h >= 61.0 && h < 120.0 {",
        "caught",
    ),
    (
        "l - 0.5*c  ->  0.5*c - l",
        "    let m: c_float = sub_ss(l, mul_ss(0.5, c));",
        "    let m: c_float = sub_ss(mul_ss(0.5, c), l);",
        "caught",
    ),
    (
        "write a 4th output word (out-of-bounds store)",
        "        *dest.add(2) = b;",
        "        *dest.add(2) = b;\n        *dest.add(3) = b;",
        "caught",
    ),
    (
        "divide by 60 as multiply by 1/60",
        "fmodf(div_ss(h, 60.0), 2.0)",
        "fmodf(mul_ss(h, 1.0 / 60.0), 2.0)",
        "caught",
    ),
    (
        "0.5*c computed as c/2 via div",
        "mul_ss(0.5, c)",
        "div_ss(c, 2.0)",
        "equivalent",  # exact for every f32, including subnormals
    ),
    (
        "c uses `1 - |2l-1|` with the subtraction operands swapped",
        "sub_ss(1.0, fabsf(sub_ss(mul_ss(2.0, l), 1.0)))",
        "sub_ss(1.0, fabsf(sub_ss(1.0, mul_ss(2.0, l))))",
        # x-y is exactly -(y-x) in IEEE-754 and `fabsf` then clears the sign, so
        # both spellings agree on every input including NaNs.
        "equivalent",
    ),
    (
        "double precision intermediate for c",
        "    let c: c_float = mul_ss(sub_ss(1.0, fabsf(sub_ss(mul_ss(2.0, l), 1.0))), s);",
        "    let c: c_float = ((1.0f64 - ((2.0f64 * l as f64) - 1.0).abs()) * s as f64) as c_float;",
        "caught",
    ),
    (
        "else-branch stores l instead of m",
        "        // Hues outside [0, 360) that reach here (>= 360, or NaN) yield grey.\n        store(m, m, m);",
        "        // Hues outside [0, 360) that reach here (>= 360, or NaN) yield grey.\n        store(l, l, l);",
        "caught",
    ),
    (
        "drop the FE_INVALID raise for a NaN hue (C uses signalling comiss)",
        "    if h.is_nan() {\n        raise_invalid();\n    }",
        "    // mutated: no raise",
        "caught",
    ),
    (
        "raise the wrong FP flag (FE_INVALID -> FE_OVERFLOW)",
        "const FE_INVALID: c_int = 0x01;",
        "const FE_INVALID: c_int = 0x08;",
        "caught",
    ),
    (
        "drop the black_box barrier in add_ss",
        "fn add_ss(src1: c_float, src2: c_float) -> c_float {\n    let raw = black_box(src1 + src2);",
        "fn add_ss(src1: c_float, src2: c_float) -> c_float {\n    let raw = src1 + src2;",
        # LLVM 1.94 happens to keep the dead `fadd` (and therefore its effect on
        # the FP status word), so removing the barrier is behaviour-preserving
        # *today*. The barrier stays because LLVM is free to DCE a dead `fadd`,
        # which would silently lose the FE_INVALID an sNaN operand must raise.
        "equivalent",
    ),
    (
        "drop the black_box barrier in mul_ss",
        "fn mul_ss(src1: c_float, src2: c_float) -> c_float {\n    let raw = black_box(src1 * src2);",
        "fn mul_ss(src1: c_float, src2: c_float) -> c_float {\n    let raw = src1 * src2;",
        "equivalent",  # see add_ss above
    ),
    (
        "add_ss drops the addition entirely (returns src1)",
        "fn add_ss(src1: c_float, src2: c_float) -> c_float {\n    let raw = black_box(src1 + src2);",
        "fn add_ss(src1: c_float, src2: c_float) -> c_float {\n    let raw = black_box(src1);",
        "caught",
    ),
    (
        "div_ss rounds through f64 (h/60 computed in double precision)",
        "    let raw = black_box(src1 / src2);",
        "    let raw = black_box(((src1 as f64) / (src2 as f64)) as c_float);",
        # Verified exhaustively over all 2^32 hue patterns: for the divisor 60,
        # `h/60f32` and `((h as f64)/60.0) as f32` agree bit for bit (f64 has
        # 53 >= 2*24+2 bits, so the double rounding is harmless -- Figueroa).
        "equivalent",
    ),
]


def run_tests(targets):
    args = ["cargo", "test", "--offline", "--release", "-q"]
    for t in targets:
        args += ["--test", t]
    p = subprocess.run(args, capture_output=True, text=True, timeout=900)
    return p.returncode == 0, p.stdout + p.stderr


def main():
    targets = sys.argv[1:] or ["configs", "errors", "fenv", "exhaustive"]
    targets = [t for t in targets if os.path.exists(f"tests/{t}.rs")]
    shutil.copy(ORIG, SRC)
    ok, out = run_tests(targets)
    if not ok:
        print("BASELINE FAILS - fix the translation first")
        print(out[-4000:])
        return 1

    print(f"baseline: PASS  (targets: {', '.join(targets)})\n")
    bad = []
    for name, old, new, expect in MUTATIONS:
        shutil.copy(ORIG, SRC)
        s = open(SRC).read()
        if old not in s:
            print(f"  SKIP (pattern not found)  {name}")
            bad.append((name, "pattern not found"))
            continue
        open(SRC, "w").write(s.replace(old, new, 1))
        ok, out = run_tests(targets)
        caught = not ok
        if "error[E" in out or "could not compile" in out:
            verdict = "DID NOT COMPILE"
        else:
            verdict = "caught" if caught else "NOT CAUGHT"
        flag = "ok " if verdict == expect or (verdict == "caught" and expect == "caught") else "!! "
        if verdict == "NOT CAUGHT" and expect == "equivalent":
            flag = "ok "
        print(f"  {flag}{verdict:<16} (expected {expect:<11}) {name}")
        if flag == "!! ":
            bad.append((name, verdict))
    shutil.copy(ORIG, SRC)
    print()
    if bad:
        print("UNEXPECTED RESULTS:")
        for n, v in bad:
            print(f"  {v}: {n}")
        return 1
    print("all mutations behaved as expected -> the suite is sensitive, not vacuous")
    return 0


if __name__ == "__main__":
    sys.exit(main())
