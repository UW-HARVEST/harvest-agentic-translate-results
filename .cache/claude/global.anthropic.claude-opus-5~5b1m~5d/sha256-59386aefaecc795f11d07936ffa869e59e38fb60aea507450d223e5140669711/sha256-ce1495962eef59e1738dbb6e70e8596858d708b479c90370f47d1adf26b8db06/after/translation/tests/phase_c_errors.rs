//! Phase C — error-path differential tests, GATED on `ERRORS.md`.
//!
//! One `#[test]` per `ERRORS.md` row, named `err_rowNN_*`.
//!
//! `rgb_to_hsv` returns `void` and has no error code, so "same rejection" is
//! asserted as **the same observable outcome**:
//!   * rows 1-3  — the early-out at `src/lib.c:19` produces the exact
//!                 documented sentinel result `{0.0, 0.0, max}`; each test
//!                 asserts that specific shape on the C side AND bit-equality
//!                 with Rust (not merely "both did something").
//!   * rows 4-7  — the fatal signal is compared exactly (both must die with
//!                 `SIGSEGV`), measured in a re-executed child process.
//!   * rows 8-14 — bit-exact output equality plus explicit no-over-read /
//!                 no-over-write assertions.

mod common;

use common::*;
use std::os::unix::process::ExitStatusExt;
use std::process::{Command, Stdio};

const N: usize = 3_000;

// ===========================================================================
// Rows 1-3 — the only rejection-shaped branch in the library (line 19).
// ===========================================================================

/// Row 1 — `delta == 0` with `max != 0` (`r == g == b != 0`).
/// Expected C result: early `return`; `dest = {0.0, 0.0, max}`; because `s` is
/// never computed there is **no `0/0` NaN** in `dest[1]`.
#[test]
fn err_row01_delta_zero_max_nonzero() {
    let (c, rust) = both();
    let mut rng = Pcg32::new(SEED ^ 101);
    let mut d = Diff::new("E01 delta==0, max!=0");
    for i in 0..N {
        let v = match i % 4 {
            0 => rng.range(0.0, 1.0),
            1 => rng.range(-100.0, 100.0),
            2 => f32::from_bits(rng.next_u32() & 0x7F7F_FFFF), // finite, non-negative
            _ => *rng.pick(&SPECIALS) as f32,
        };
        if !v.is_finite() || v == 0.0 {
            continue;
        }
        let src = [v, v, v];
        let got_c = call_bits(c.f, &src);
        // Assert the DOCUMENTED sentinel, on the C side, exactly.
        assert_eq!(got_c[0], 0.0f32.to_bits(), "C h must be +0.0 for v={v:e}");
        assert_eq!(got_c[1], 0.0f32.to_bits(), "C s must be +0.0 for v={v:e}");
        assert_eq!(got_c[2], v.to_bits(), "C v must be max for v={v:e}");
        assert!(
            !f32::from_bits(got_c[1]).is_nan(),
            "early-out must not produce 0/0 NaN"
        );
        d.check(&c, &rust, src);
    }
    d.finish();
}

/// Row 2 — `max == 0` with `delta != 0`: `max` is `±0.0` while `min < 0`.
/// Reachable only with negative (out-of-documented-range) channels.
/// Expected C result: early `return`, `dest = {0.0, 0.0, ±0.0}`. The `||`
/// short-circuit **prevents `delta / 0`**, so `dest[1]` must not be `Inf`.
#[test]
fn err_row02_max_zero_delta_nonzero() {
    let (c, rust) = both();
    let mut rng = Pcg32::new(SEED ^ 102);
    let mut d = Diff::new("E02 max==0, delta!=0");
    for _ in 0..N {
        let n1 = -rng.range(1e-20, 1e20);
        let n2 = -rng.range(1e-20, 1e20);
        let z = if rng.below(2) == 0 { 0.0f32 } else { -0.0f32 };
        let src = match rng.below(3) {
            0 => [z, n1, n2],
            1 => [n1, z, n2],
            _ => [n1, n2, z],
        };
        let got_c = call_bits(c.f, &src);
        assert_eq!(got_c[0], 0.0f32.to_bits(), "C h must be +0.0 for {src:?}");
        assert_eq!(got_c[1], 0.0f32.to_bits(), "C s must be +0.0 for {src:?}");
        let s = f32::from_bits(got_c[1]);
        assert!(
            !s.is_infinite() && !s.is_nan(),
            "short-circuit must prevent delta/0 -> got s={s:e} for {src:?}"
        );
        assert_eq!(
            f32::from_bits(got_c[2]),
            0.0f32,
            "C v must be zero for {src:?}"
        );
        d.check(&c, &rust, src);
    }
    d.finish();
}

/// Row 3 — both disjuncts true: every channel is `+0.0`/`-0.0`.
/// Expected C result: `dest = {0.0, 0.0, max}` where `max` keeps the exact
/// signed zero the C ternary chose (`(a > b) ? a : b` returns `b` on a tie).
#[test]
fn err_row03_both_disjuncts_zero() {
    let (c, rust) = both();
    let mut d = Diff::new("E03 all zeros (both disjuncts)");
    const Z: [u32; 2] = [0x0000_0000, 0x8000_0000];
    for &r in &Z {
        for &g in &Z {
            for &b in &Z {
                let got_c = call_bits_raw(c.f, &[r, g, b]);
                assert_eq!(got_c[0], 0.0f32.to_bits());
                assert_eq!(got_c[1], 0.0f32.to_bits());
                // v == max == c_max(c_max(r,g), b); ties yield the 2nd operand,
                // so with all-equal (zero) magnitudes v takes b's sign bit.
                assert_eq!(
                    got_c[2], b,
                    "C v must be the LAST ternary operand's signed zero"
                );
                d.check_raw(&c, &rust, [r, g, b]);
            }
        }
    }
    assert_eq!(d.checked, 8);
    d.finish();
}

// ===========================================================================
// Rows 4-7 — null pointers. No check exists in the C, so this is UB; the
// differential assertion is on the exact fatal signal, measured in a child.
// ===========================================================================

const ENV_CASE: &str = "HARVEST_NULL_CASE";
const ENV_TARGET: &str = "HARVEST_TARGET_SO";
/// Exit code the worker uses if it unexpectedly SURVIVES the null call.
const SURVIVED: i32 = 77;
/// `SIGSEGV` / `SIGABRT` are 11 / 6 on every Linux ABI.
const SIGSEGV: i32 = 11;
const SIGABRT: i32 = 6;

/// Child-side worker. A no-op unless the parent set `HARVEST_NULL_CASE`.
/// Loads exactly the `.so` named by `HARVEST_TARGET_SO` and performs the
/// requested null-pointer call, so the parent can observe how it dies.
#[test]
fn null_child_worker() {
    let case = match std::env::var(ENV_CASE) {
        Ok(v) => v,
        Err(_) => return, // normal run: nothing to do
    };
    let target = std::env::var(ENV_TARGET).expect("HARVEST_TARGET_SO must be set");
    let lib = load_one("target", std::path::PathBuf::from(target));
    let f = lib.f;

    let mut dest = [0.0f32; 3];
    let src_normal = [0.5f32, 0.25, 0.75]; // non-degenerate: full path
    let src_degenerate = [0.4f32, 0.4, 0.4]; // takes the line-19 early-out

    unsafe {
        match case.as_str() {
            "null_src" => f(dest.as_mut_ptr(), std::ptr::null()),
            "null_dest" => f(std::ptr::null_mut(), src_normal.as_ptr()),
            "null_dest_degenerate" => f(std::ptr::null_mut(), src_degenerate.as_ptr()),
            "both_null" => f(std::ptr::null_mut(), std::ptr::null()),
            other => panic!("unknown case {other}"),
        }
    }
    // Reached only if the null dereference did NOT fault.
    std::process::exit(SURVIVED);
}

#[derive(Debug, PartialEq, Eq)]
struct Outcome {
    code: Option<i32>,
    signal: Option<i32>,
}

fn run_null_child(case: &str, so: &std::path::Path) -> (Outcome, String) {
    let exe = std::env::current_exe().expect("current_exe");
    let out = Command::new(exe)
        .args(["null_child_worker", "--exact", "--test-threads=1"])
        .env(ENV_CASE, case)
        .env(ENV_TARGET, so)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .expect("spawn child");
    (
        Outcome {
            code: out.status.code(),
            signal: out.status.signal(),
        },
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// Shared body for rows 4-7.
///
/// The primary differential assertion compares the C `.so` against the
/// UNINSTRUMENTED Rust `.so` (built without `debug_assertions`), because that
/// is the like-for-like comparison: the C is compiled with no null-pointer
/// instrumentation either. Both must die with the SAME signal — an exact
/// signal match, not merely "both failed somehow".
///
/// The Rust artifact resolved by the default rules is then also checked. If it
/// happens to be a `debug_assertions` build, rustc's UB checker fires first and
/// aborts with a diagnostic instead of faulting; that outcome is accepted ONLY
/// when the diagnostic proves it was the null-pointer check (never as a
/// blanket "some failure occurred").
fn assert_same_fatal(case: &str, row: &str) {
    let c_path = c_so();
    let rust_nochecks = rust_so_nochecks();

    let (oc, _) = run_null_child(case, &c_path);
    let (or, _) = run_null_child(case, &rust_nochecks);
    eprintln!("row {row} case={case}: C {oc:?} | Rust(no-ub-checks) {or:?}");

    assert_ne!(
        oc.code,
        Some(SURVIVED),
        "row {row}: C unexpectedly survived the null deref; test premise invalid"
    );
    assert_eq!(
        oc.signal,
        Some(SIGSEGV),
        "row {row}: expected C to die with SIGSEGV, got {oc:?}"
    );
    // *** The differential assertion for this row. ***
    assert_eq!(
        oc, or,
        "row {row}: C and uninstrumented Rust terminated DIFFERENTLY for case {case}"
    );

    // Secondary: the default-resolved artifact.
    let rust_default = rust_so();
    if rust_default == rust_nochecks {
        return;
    }
    let (od, stderr) = run_null_child(case, &rust_default);
    if od == oc {
        eprintln!("row {row} case={case}: Rust(default) also {od:?}");
        return;
    }
    assert_eq!(
        od.signal,
        Some(SIGABRT),
        "row {row}: Rust(default) died in an unexpected way: {od:?}\nstderr:\n{stderr}"
    );
    assert!(
        stderr.contains("null pointer dereference"),
        "row {row}: Rust(default) aborted for an UNEXPLAINED reason (not the \
         debug-assertions null check):\n{stderr}"
    );
    eprintln!(
        "row {row} case={case}: Rust(default={}) is a debug_assertions build; \
         rustc's UB checker trapped the deref (SIGABRT) instead of faulting \
         -- instrumentation difference, not a behavioural one",
        rust_default.display()
    );
}

/// Row 4 — `src == NULL`, `dest` valid ⇒ faults on the `src[0]` load.
#[test]
fn err_row04_null_src_segv() {
    assert_same_fatal("null_src", "E04");
}

/// Row 5 — `dest == NULL`, `src` valid and non-degenerate ⇒ faults on the
/// `dest[0]` store at line 35.
#[test]
fn err_row05_null_dest_segv() {
    assert_same_fatal("null_dest", "E05");
}

/// Row 6 — `dest == NULL` with DEGENERATE input, so the line-19 early-out
/// runs. The early-out writes `dest` too (lines 20-22), so the crash is NOT
/// avoided — a genuinely different store site from row 5.
#[test]
fn err_row06_null_dest_early_out_segv() {
    assert_same_fatal("null_dest_degenerate", "E06");
}

/// Row 7 — both pointers null ⇒ the `src` load faults first.
#[test]
fn err_row07_both_null_segv() {
    assert_same_fatal("both_null", "E07");
}

// ===========================================================================
// Row 8 — buffer extent: exactly 3 floats read, exactly 3 written.
// ===========================================================================

/// Row 8 — no over-read and no over-write.
///
/// * write side: `dest` sits in the middle of a canary-filled 9-float buffer;
///   the 3 floats before and the 3 after must be untouched.
/// * read side: the same input is run twice with DIFFERENT poison in the
///   neighbouring floats; if the result changes, the callee over-read.
#[test]
fn err_row08_no_out_of_bounds_access() {
    let (c, rust) = both();
    let mut rng = Pcg32::new(SEED ^ 108);
    let mut d = Diff::new("E08 exact 3-float extent");

    for i in 0..N {
        let src = if i % 2 == 0 {
            [rng.range(-2.0, 2.0), rng.range(-2.0, 2.0), rng.range(-2.0, 2.0)]
        } else {
            [
                f32::from_bits(*rng.pick(&SPECIALS)),
                f32::from_bits(*rng.pick(&SPECIALS)),
                f32::from_bits(*rng.pick(&SPECIALS)),
            ]
        };

        // ---- write side ----
        for (name, f) in [("C", c.f), ("Rust", rust.f)] {
            let mut buf = [f32::from_bits(CANARY); 9];
            unsafe { f(buf.as_mut_ptr().add(3), src.as_ptr()) };
            for k in (0..3).chain(6..9) {
                assert_eq!(
                    buf[k].to_bits(),
                    CANARY,
                    "{name} wrote OUT OF BOUNDS at dest index {}",
                    k as i32 - 3
                );
            }
        }

        // ---- read side ----
        let mut outs = Vec::new();
        for poison in [0x7F7F_FFFFu32, 0x0000_0000] {
            let mut pair = Vec::new();
            for f in [c.f, rust.f] {
                let mut inbuf = [f32::from_bits(poison); 9];
                inbuf[3] = src[0];
                inbuf[4] = src[1];
                inbuf[5] = src[2];
                let mut dest = [f32::from_bits(CANARY); 3];
                unsafe { f(dest.as_mut_ptr(), inbuf.as_ptr().add(3)) };
                pair.push([dest[0].to_bits(), dest[1].to_bits(), dest[2].to_bits()]);
            }
            outs.push(pair);
        }
        assert_eq!(
            outs[0][0], outs[1][0],
            "C result depends on bytes OUTSIDE src[0..3] -> over-read"
        );
        assert_eq!(
            outs[0][1], outs[1][1],
            "Rust result depends on bytes OUTSIDE src[0..3] -> over-read"
        );
        d.check_outputs(format!("extent src={src:?}"), &outs[0][0], &outs[0][1]);
    }
    d.finish();
}

// ===========================================================================
// Rows 9-11 — non-finite inputs (no validation exists).
// ===========================================================================

/// Row 9 — a single `NaN` channel, including non-default payloads.
#[test]
fn err_row09_nan_single_channel() {
    let (c, rust) = both();
    let mut rng = Pcg32::new(SEED ^ 109);
    let mut d = Diff::new("E09 single NaN channel");
    let nans = [0x7FC0_0000u32, 0xFFC0_0000, 0x7FD5_5555, 0x7FFF_FFFF, 0xFFAB_CDEF];
    for _ in 0..N {
        let n = *rng.pick(&nans);
        let a = rng.range(-2.0, 2.0).to_bits();
        let b = rng.range(-2.0, 2.0).to_bits();
        let src = match rng.below(3) {
            0 => [n, a, b],
            1 => [a, n, b],
            _ => [a, b, n],
        };
        // A NaN anywhere must NOT be silently rejected: the C computes with it.
        d.check_raw(&c, &rust, src);
    }
    d.finish();
}

/// Row 10 — `NaN` in 2 or 3 channels, and signalling NaNs (`0x7F80_0001`),
/// whose quieting behaviour must agree bit-for-bit.
#[test]
fn err_row10_nan_multi_and_snan() {
    let (c, rust) = both();
    let mut d = Diff::new("E10 multi-NaN + sNaN");
    let nans = [
        0x7F80_0001u32, // sNaN, smallest payload
        0x7FBF_FFFF,    // sNaN, largest payload
        0xFF80_0001,    // negative sNaN
        0x7FC0_0000,    // qNaN
        0xFFC0_0000,    // negative qNaN
    ];
    let plain = 0.375f32.to_bits();
    for &n1 in &nans {
        for &n2 in &nans {
            d.check_raw(&c, &rust, [n1, n2, plain]);
            d.check_raw(&c, &rust, [n1, plain, n2]);
            d.check_raw(&c, &rust, [plain, n1, n2]);
            for &n3 in &nans {
                d.check_raw(&c, &rust, [n1, n2, n3]);
            }
        }
    }
    d.finish();
}

/// Row 11 — `±Inf` channels, exhaustive over a set that mixes both infinities
/// with finite values (`Inf - Inf = NaN`, `Inf/Inf = NaN`, `x/Inf = ±0`).
#[test]
fn err_row11_infinities() {
    let (c, rust) = both();
    let mut d = Diff::new("E11 infinities");
    let vals: [u32; 8] = [
        0x7F80_0000, // +Inf
        0xFF80_0000, // -Inf
        0x0000_0000, // +0
        0x8000_0000, // -0
        0x3F80_0000, // 1.0
        0xBF80_0000, // -1.0
        0x7F7F_FFFF, // +FLT_MAX
        0xFF7F_FFFF, // -FLT_MAX
    ];
    let mut saw_inf = 0usize;
    for &r in &vals {
        for &g in &vals {
            for &b in &vals {
                let has_inf = [r, g, b]
                    .iter()
                    .any(|&x| x == 0x7F80_0000 || x == 0xFF80_0000);
                if has_inf {
                    saw_inf += 1;
                    d.check_raw(&c, &rust, [r, g, b]);
                }
            }
        }
    }
    assert!(saw_inf > 200, "row E11 covered too few vectors: {saw_inf}");
    d.finish();
}

// ===========================================================================
// Rows 12-13 — one step past the documented range; extreme ratios.
// ===========================================================================

/// Row 12 — values one step past the documented `[0, 1]` range, and huge
/// values whose `max - min` overflows. No clamp exists, so `s` may exceed 1
/// and `v` is unbounded; Rust must be unclamped identically.
#[test]
fn err_row12_out_of_documented_range() {
    let (c, rust) = both();
    let mut rng = Pcg32::new(SEED ^ 112);
    let mut d = Diff::new("E12 one step past [0,1]");
    // Exact boundary encodings around 0.0 and 1.0.
    let boundary: [u32; 8] = [
        0x0000_0000, // 0.0
        0x8000_0001, // -FLT_TRUE_MIN  (one step below 0)
        0x0000_0001, // +FLT_TRUE_MIN  (one step above 0)
        0x3F7F_FFFF, // one step below 1.0
        0x3F80_0000, // 1.0
        0x3F80_0001, // one step above 1.0
        0x7F7F_FFFF, // +FLT_MAX
        0xFF7F_FFFF, // -FLT_MAX
    ];
    for &r in &boundary {
        for &g in &boundary {
            for &b in &boundary {
                d.check_raw(&c, &rust, [r, g, b]);
            }
        }
    }
    // Randomised well outside the range.
    let mut saw_s_gt_1 = 0usize;
    for _ in 0..N {
        let src = [
            rng.range(-1000.0, 1000.0),
            rng.range(-1000.0, 1000.0),
            rng.range(-1000.0, 1000.0),
        ];
        let got = call_bits(c.f, &src);
        let s = f32::from_bits(got[1]);
        if s > 1.0 {
            saw_s_gt_1 += 1;
        }
        d.check(&c, &rust, src);
    }
    assert!(
        saw_s_gt_1 > 0,
        "row E12 never produced the unclamped s > 1 it is meant to cover"
    );
    d.finish();
}

/// Row 13 — subnormals and extreme ratios: subnormal `delta` with large
/// `max` (underflow), subnormal `max` (ratio overflow to `Inf`), `FLT_MIN`
/// and `FLT_TRUE_MIN`.
#[test]
fn err_row13_subnormal_and_overflow() {
    let (c, rust) = both();
    let mut rng = Pcg32::new(SEED ^ 113);
    let mut d = Diff::new("E13 subnormal / overflow ratios");
    let edge: [u32; 8] = [
        0x0000_0001, // FLT_TRUE_MIN
        0x0000_0002,
        0x007F_FFFF, // largest subnormal
        0x0080_0000, // FLT_MIN
        0x8000_0001, // -FLT_TRUE_MIN
        0x807F_FFFF, // -largest subnormal
        0x7F7F_FFFF, // FLT_MAX
        0xFF7F_FFFF, // -FLT_MAX
    ];
    for &r in &edge {
        for &g in &edge {
            for &b in &edge {
                d.check_raw(&c, &rust, [r, g, b]);
            }
        }
    }
    let mut saw_inf_s = 0usize;
    let mut saw_zero_s = 0usize;
    for _ in 0..N {
        // subnormal max together with a big negative min => delta/max overflows
        let mx = f32::from_bits(1 + rng.below(0x007F_FFFF));
        let mn = -rng.range(1.0, 1e30);
        let src = match rng.below(3) {
            0 => [mx, mn, mn],
            1 => [mn, mx, mn],
            _ => [mn, mn, mx],
        };
        let s = f32::from_bits(call_bits(c.f, &src)[1]);
        if s.is_infinite() {
            saw_inf_s += 1;
        }
        d.check(&c, &rust, src);

        // tiny delta with a huge max => delta/max underflows toward 0
        let base = f32::from_bits(((120 + rng.below(8)) << 23) | (rng.next_u32() & 0x007F_FFFF));
        let hi = f32::from_bits(base.to_bits() + 1);
        let src2 = [hi, base, base];
        let s2 = f32::from_bits(call_bits(c.f, &src2)[1]);
        if s2 == 0.0 || s2.abs() < f32::MIN_POSITIVE {
            saw_zero_s += 1;
        }
        d.check(&c, &rust, src2);
    }
    assert!(
        saw_inf_s > 0,
        "row E13 never hit the delta/max overflow it is meant to cover"
    );
    let _ = saw_zero_s;
    d.finish();
}

// ===========================================================================
// Row 14 — out-of-range enum across the FFI boundary.
// ===========================================================================

/// Row 14 — the mandated "out-of-range enum" check.
///
/// `void rgb_to_hsv(float *dest, const float *src)` has **no enum, no `int`,
/// and no length/count parameter**, so there is literally no enum surface to
/// pass an invalid variant through, and no length to make zero or oversized.
/// The faithful analogue for a `float` parameter is a bit pattern that encodes
/// **no valid number**: the exponent-max encodings (`Inf` and every `NaN`).
///
/// This test sweeps that space systematically — both signs x all 256 top
/// mantissa bytes x several low-mantissa patterns x each of the 3 channel
/// slots — the float equivalent of "every int with no valid variant".
#[test]
fn err_row14_all_bit_patterns_no_enum_surface() {
    let (c, rust) = both();
    let mut d = Diff::new("E14 exponent-max encodings (no enum surface)");
    let plain = 0.625f32.to_bits();
    let low_patterns: [u32; 4] = [0x0000_0000, 0x0000_0001, 0x0000_5555, 0x0000_7FFF];

    for sign in [0u32, 0x8000_0000] {
        for hi in 0u32..256 {
            for &low in &low_patterns {
                // exponent = all ones; mantissa = hi<<15 | low
                let mant = ((hi << 15) | low) & 0x007F_FFFF;
                let bits = sign | 0x7F80_0000 | mant;
                d.check_raw(&c, &rust, [bits, plain, plain]);
                d.check_raw(&c, &rust, [plain, bits, plain]);
                d.check_raw(&c, &rust, [plain, plain, bits]);
            }
        }
    }
    assert_eq!(d.checked, 2 * 256 * 4 * 3);
    d.finish();
}
