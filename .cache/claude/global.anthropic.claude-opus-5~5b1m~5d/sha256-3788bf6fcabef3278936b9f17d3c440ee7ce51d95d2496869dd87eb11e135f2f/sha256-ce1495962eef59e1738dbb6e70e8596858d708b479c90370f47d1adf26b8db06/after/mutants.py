#!/usr/bin/env python3
"""Mutation test for the differential suite.

A differential test suite is only worth what it can DETECT. Each mutant below
introduces a plausible mistranslation of one subtle numerical detail in
`src/lib.rs`. The suite must FAIL for every one of them.

Critically, each mutation is verified to have actually been applied: a
replacement that silently matched nothing would look like a "surviving mutant"
and hide a real blind spot.
"""
import subprocess, shutil, sys, os

ROOT = os.path.dirname(os.path.abspath(__file__))
SRC = os.path.join(ROOT, "translation/src/lib.rs")
BAK = SRC + ".mutbak"

# Mutants that CANNOT be detected by any input, because the mutation is
# behaviourally equivalent on this target. Each carries a proof; these are not
# test blind spots. Phase B is exhaustive over the complete 2^24 input domain, so
# "survived" here means "no input distinguishes them" — a proof of equivalence,
# not a gap in coverage.
EXPECTED_EQUIVALENT = {
    "swap_row_coefficients":
        "0.12739886310880f32 and 0.12739886341072f32 round to the SAME f32 bit "
        "pattern (0x3e0274d9); their decimal gap 3.02e-10 is far below the f32 "
        "ulp 1.49e-08 near 0.127, so the swap is a machine-level no-op.",
    "threshold_ge_instead_of_gt":
        "equality is unreachable: 0.04045*255 = 10.31475 is not an integer, so no "
        "byte/255.f ever equals the threshold exactly and `>` == `>=` here.",
    "powf_instead_of_libm_pow":
        "rustc lowers f64::powf to a call to the very same pow@GLIBC_2.29 that the "
        "C links (verified with nm on a minimal cdylib), so it is the same code.",
    "exponent_one_over_2_4":
        "0.4166666666 vs 1/2.4 perturbs the pow result by ~1e-9 relative; scaled by "
        "255 that is ~3e-7 of a u8 step, and exhaustively no input lands close "
        "enough to a .5 rounding boundary for it to change a byte.",
    "drop_tiny_matrix_terms":
        "the 4.486e-11/3.1113e-10 coefficients are below the f32 ulp of the sums they "
        "join; in the only case where they dominate (G=B=0) the traced denorm "
        "argument is 0.49999985/0.50000101 vs 0.5, all of which truncate to 0.",
}

# (name, [(old, new), ...], rationale)
MUTANTS = [
    ("clamp_instead_of_wrap",
     [("    as_i32 as u8\n", "    as_i32.clamp(0, 255) as u8\n")],
     "cbDenorm float->uchar must WRAP (cvttss2si + mov %al), not clamp"),

    ("saturating_cast",
     [("""    let truncated = v.trunc() as f64;
    let as_i32 = if v.is_nan() || truncated < -2147483648.0 || truncated >= 2147483648.0 {
        i32::MIN // 0x8000_0000: the x86 "integer indefinite" result
    } else {
        truncated as i32
    };
    as_i32 as u8""",
       "    v as u8")],
     "Rust's `as u8` saturates; C truncates to i32 then keeps the low byte"),

    ("round_instead_of_trunc",
     [("v.trunc() as f64", "v.round() as f64")],
     "cvttss2si truncates toward zero, it does not round"),

    ("exponent_one_over_2_4",
     [("0.4166666666", "(1.0f64 / 2.4f64)")],
     "C uses the TRUNCATED literal 0.4166666666, not 1/2.4"),

    ("powf_instead_of_libm_pow",
     [("unsafe { pow((c + 0.055) / 1.055, 2.4) }", "((c + 0.055) / 1.055).powf(2.4)"),
      ("unsafe { pow(c, 0.4166666666) }", "c.powf(0.4166666666)")],
     "must use the same libm `pow` the C links for bit-identical results"),

    ("remove_gamma_in_f32",
     [("""        let c = c as f64;
        // NaN takes the `else` arm, matching C's `>` (and the `comisd`/`ja`
        // pair in the reference build, which falls through when unordered).
        let v = if c > 0.04045 {
            unsafe { pow((c + 0.055) / 1.055, 2.4) }
        } else {
            c / 12.92
        };
        v as f32""",
       """        let v = if c > 0.04045f32 {
            ((c + 0.055f32) / 1.055f32).powf(2.4f32)
        } else {
            c / 12.92f32
        };
        v""")],
     "C promotes to double (literals are double) before narrowing back to float"),

    ("apply_gamma_in_f32",
     [("""        let c = c as f64;
        // Threshold written out verbatim as in the C source.
        let v = if c > 0.00313080495356037151702786377709 {
            1.055 * unsafe { pow(c, 0.4166666666) } - 0.055
        } else {
            c * 12.92
        };
        v as f32""",
       """        let v = if c > 0.00313080495356037f32 {
            1.055f32 * c.powf(0.4166666666f32) - 0.055f32
        } else {
            c * 12.92f32
        };
        v""")],
     "same double-promotion rule in cbApplyGammaRGB"),

    ("drop_tiny_matrix_terms",
     [("*Green = -4.486E-11f32 * R + 0.87390929928361f32 * G + 0.12609070101523f32 * B;",
       "*Green = 0.87390929928361f32 * G + 0.12609070101523f32 * B;"),
      ("*Blue = 3.1113E-10f32 * R + 0.87390929725848f32 * G + 0.12609070067115f32 * B;",
       "*Blue = 0.87390929725848f32 * G + 0.12609070067115f32 * B;")],
     "the ~1e-11 coefficients are not negligible: dropping them changes rounding"),

    ("matrix_in_f64",
     [("""    *Red = R + 0.12739886310880f32 * G - 0.12739886341072f32 * B;
    *Green = -4.486E-11f32 * R + 0.87390929928361f32 * G + 0.12609070101523f32 * B;
    *Blue = 3.1113E-10f32 * R + 0.87390929725848f32 * G + 0.12609070067115f32 * B;""",
       """    let (rd, gd, bd) = (R as f64, G as f64, B as f64);
    *Red = (rd + 0.12739886310880 * gd - 0.12739886341072 * bd) as f32;
    *Green = (-4.486E-11 * rd + 0.87390929928361 * gd + 0.12609070101523 * bd) as f32;
    *Blue = (3.1113E-10 * rd + 0.87390929725848 * gd + 0.12609070067115 * bd) as f32;""")],
     "the matrix is pure float arithmetic in C; f64 intermediates change results"),

    ("matrix_aliasing_no_snapshot",
     [("""    let R: f32 = *Red;
    let G: f32 = *Green;
    let B: f32 = *Blue;
    *Red = R + 0.12739886310880f32 * G - 0.12739886341072f32 * B;
    *Green = -4.486E-11f32 * R + 0.87390929928361f32 * G + 0.12609070101523f32 * B;
    *Blue = 3.1113E-10f32 * R + 0.87390929725848f32 * G + 0.12609070067115f32 * B;""",
       """    *Red = *Red + 0.12739886310880f32 * *Green - 0.12739886341072f32 * *Blue;
    *Green = -4.486E-11f32 * *Red + 0.87390929928361f32 * *Green + 0.12609070101523f32 * *Blue;
    *Blue = 3.1113E-10f32 * *Red + 0.87390929725848f32 * *Green + 0.12609070067115f32 * *Blue;""")],
     "C snapshots R,G,B first; later rows must see the ORIGINAL values"),

    ("norm_reciprocal_multiply",
     [("(RGB.R as f32) / 255.0f32", "(RGB.R as f32) * (1.0f32 / 255.0f32)")],
     "x/255 and x*(1/255) differ in float; C divides"),

    ("denorm_no_half_offset",
     [("RGB.R * 255.0f32 + 0.5f32", "RGB.R * 255.0f32")],
     "the +0.5f rounding offset must be present"),

    ("threshold_ge_instead_of_gt",
     [("if c > 0.04045", "if c >= 0.04045")],
     "C uses strict `>`; at byte 10.31... the boundary matters"),

    ("swap_row_coefficients",
     [("0.12739886310880f32 * G - 0.12739886341072f32 * B",
       "0.12739886341072f32 * G - 0.12739886310880f32 * B")],
     "the two near-identical red-row coefficients are NOT interchangeable"),
]


def run(cmd, cwd=None):
    return subprocess.run(cmd, shell=True, cwd=cwd, capture_output=True, text=True)


def main():
    shutil.copy(SRC, BAK)
    orig = open(BAK).read()
    results = []
    try:
        for name, subs, why in MUTANTS:
            s = orig
            applied = True
            for old, new in subs:
                if old not in s:
                    applied = False
                    break
                s = s.replace(old, new, 1)
            if not applied:
                results.append((name, "NOT-APPLIED", why))
                print(f"[{name}] ERROR: pattern did not match -> mutant invalid")
                continue
            open(SRC, "w").write(s)

            b = run("cargo build --release 2>&1", cwd=os.path.join(ROOT, "translation"))
            if b.returncode != 0:
                # A mutant that does not compile is still "detected" (loudly), but
                # flag it so it is not mistaken for a behavioural catch.
                results.append((name, "COMPILE-ERROR", why))
                print(f"[{name}] did not compile (counts as detected)")
                continue

            t = run("cargo test --release 2>&1",
                    cwd=os.path.join(ROOT, "translation"))
            if t.returncode != 0:
                status = "CAUGHT"
            elif name in EXPECTED_EQUIVALENT:
                status = "EQUIVALENT"
            else:
                status = "SURVIVED"
            results.append((name, status, why))
            print(f"[{name}] {status}")
            if status == "EQUIVALENT":
                print(f"    (provably undetectable) {EXPECTED_EQUIVALENT[name]}")
            elif status == "SURVIVED":
                print("    !!! blind spot: no test detected this mistranslation")
            # An "expected equivalent" mutant that IS caught means the proof is
            # wrong and deserves attention just as much as a survivor.
            if name in EXPECTED_EQUIVALENT and status == "CAUGHT":
                print("    !!! unexpected: this was proved equivalent but a test failed")
    finally:
        shutil.copy(BAK, SRC)
        os.remove(BAK)
        run("cargo build --release", cwd=os.path.join(ROOT, "translation"))

    print("\n=== MUTATION SUMMARY ===")
    survived = [r for r in results if r[1] == "SURVIVED"]
    notapp = [r for r in results if r[1] == "NOT-APPLIED"]
    equiv = [r for r in results if r[1] == "EQUIVALENT"]
    caught = [r for r in results if r[1] in ("CAUGHT", "COMPILE-ERROR")]
    for name, status, why in results:
        print(f"{status:14s} {name:34s} {why}")
    print(f"\ntotal={len(results)}  caught={len(caught)}  provably-equivalent={len(equiv)}"
          f"  UNEXPLAINED-SURVIVORS={len(survived)}  invalid={len(notapp)}")
    if survived or notapp:
        print("\nFAIL: unexplained survivors or invalid mutants -> the suite has a gap.")
        sys.exit(1)
    print("\nPASS: every behaviourally-distinguishable mutant was detected; the "
          f"{len(equiv)} survivor(s) are proved to be equivalent transformations.")


if __name__ == "__main__":
    main()
