//! Differential test for the *floating-point status flags* the call leaves
//! behind.
//!
//! This is a side effect a real caller can observe (`feclearexcept` /
//! `fetestexcept` around the call, or `feenableexcept` to turn it into a trap),
//! so it is part of "behaves identically" even though it is not part of the
//! return value. It is also the one axis where a naive Rust translation
//! necessarily differs from the C:
//!
//!   * the C compares the hue with `comiss` (*signalling*: raises `FE_INVALID`
//!     for a quiet NaN too), whereas Rust's `>=`/`<` lower to `ucomiss`
//!     (*quiet*: only a signalling NaN raises);
//!   * a translation that short-circuits the arithmetic when an operand is NaN
//!     never executes the `addss`/`mulss`, so it never raises the `FE_INVALID`
//!     that the C's `addss` raises for a signalling-NaN operand.

mod common;

use common::*;

const FE_INVALID: i32 = 0x01;
const FE_DIVBYZERO: i32 = 0x04;
const FE_OVERFLOW: i32 = 0x08;
const FE_UNDERFLOW: i32 = 0x10;
const FE_INEXACT: i32 = 0x20;
/// glibc/x86: the five ISO C flags plus the x86-only `FE_DENORM` (0x02).
const FE_ALL: i32 = 0x3F;

unsafe extern "C" {
    fn feclearexcept(excepts: i32) -> i32;
    fn fetestexcept(excepts: i32) -> i32;
}

fn flags_of(lib: &Lib, h: f32, s: f32, l: f32) -> i32 {
    let src = [h, s, l];
    let mut dest = [0.0f32; 3];
    unsafe {
        feclearexcept(FE_ALL);
        (lib.f)(dest.as_mut_ptr(), src.as_ptr());
        fetestexcept(FE_ALL)
    }
}

fn describe(f: i32) -> String {
    let mut v = Vec::new();
    for (bit, name) in [
        (FE_INVALID, "INVALID"),
        (0x02, "DENORM"),
        (FE_DIVBYZERO, "DIVBYZERO"),
        (FE_OVERFLOW, "OVERFLOW"),
        (FE_UNDERFLOW, "UNDERFLOW"),
        (FE_INEXACT, "INEXACT"),
    ] {
        if f & bit != 0 {
            v.push(name);
        }
    }
    if v.is_empty() {
        "-".into()
    } else {
        v.join("|")
    }
}

fn check(ctx: &str, h: f32, s: f32, l: f32, failures: &mut Vec<String>) {
    let c = flags_of(c_lib(), h, s, l);
    for r in rust_libs() {
        let g = flags_of(r, h, s, l);
        if c != g {
            failures.push(format!(
                "[{ctx}] {} h={:#010x} s={:#010x} l={:#010x}: C={} rust={}",
                r.name,
                h.to_bits(),
                s.to_bits(),
                l.to_bits(),
                describe(c),
                describe(g)
            ));
        }
    }
}

/// The interesting cases: quiet NaN hues (C's `comiss` raises `FE_INVALID`,
/// `ucomiss` does not), signalling NaNs in every position (the arithmetic must
/// raise `FE_INVALID`), the invalid-operation-from-finite-input cases, and
/// overflow/underflow/inexact from ordinary values.
#[test]
fn fp_status_flags_match() {
    let mut failures = Vec::new();

    // (a) Quiet NaN hue -> the C's signalling `comiss` raises FE_INVALID.
    for &h in &nan_floats() {
        check("qNaN/sNaN hue", h, 1.0, 0.5, &mut failures);
        check("qNaN/sNaN hue, s=0", h, 0.0, 0.5, &mut failures);
    }
    // (b) NaNs in s and l.
    for &n in &nan_floats() {
        check("NaN s", 30.0, n, 0.5, &mut failures);
        check("NaN l", 30.0, 1.0, n, &mut failures);
        check("NaN s and l", 30.0, n, n, &mut failures);
    }
    // (c) Invalid operation reached from finite/infinite input.
    check("0*Inf chroma", 30.0, f32::INFINITY, 0.0, &mut failures);
    check("Inf-Inf midpoint", 30.0, -1.0, f32::INFINITY, &mut failures);
    check("Inf hue -> fmodf domain error", f32::INFINITY, 1.0, 0.5, &mut failures);
    check("-Inf hue", f32::NEG_INFINITY, 1.0, 0.5, &mut failures);
    // (d) Overflow / underflow / inexact from ordinary values.
    check("overflow", 30.0, f32::MAX, f32::MAX, &mut failures);
    check("underflow", 30.0, f32::from_bits(1), 0.5, &mut failures);
    check("inexact", 33.3, 0.7, 0.3, &mut failures);
    check("exact", 30.0, 1.0, 0.5, &mut failures);
    // (e) The early-return path raises nothing at all.
    check("early return", 30.0, 0.0, 0.5, &mut failures);
    check("early return, NaN l", 30.0, 0.0, f32::from_bits(0x7f80_0001), &mut failures);

    // (f) Randomized sweep over the whole space.
    let mut rng = Rng::new(0xFE01);
    for _ in 0..20_000 {
        let h = rng.bits_f32();
        let s = rng.bits_f32();
        let l = rng.bits_f32();
        check("fuzz", h, s, l, &mut failures);
    }
    let pool = specials_and_nans();
    for &h in &pool {
        for &s in &pool {
            for &l in &pool {
                check("specials^3", h, s, l, &mut failures);
            }
        }
    }
    for &h in HUE_BOUNDARIES {
        for &s in &pool {
            for &l in &pool {
                check("boundary hues x specials", h, s, l, &mut failures);
            }
        }
    }
    let mut rng2 = Rng::new(0xFE02);
    for _ in 0..20_000 {
        let h = random_hue_any_sector(&mut rng2);
        let s = rng2.log_uniform();
        let l = rng2.log_uniform();
        check("fuzz log-uniform", h, s, l, &mut failures);
    }

    if !failures.is_empty() {
        let shown: Vec<&String> = failures.iter().take(25).collect();
        panic!(
            "{} FP-status-flag divergence(s); first {}:\n{}",
            failures.len(),
            shown.len(),
            shown
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join("\n")
        );
    }
}
