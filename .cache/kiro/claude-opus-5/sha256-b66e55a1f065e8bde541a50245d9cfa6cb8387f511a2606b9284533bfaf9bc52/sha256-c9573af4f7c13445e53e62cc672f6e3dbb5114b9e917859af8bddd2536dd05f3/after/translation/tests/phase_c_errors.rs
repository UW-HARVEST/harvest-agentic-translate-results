//! Phase C — error-path differential tests.
//!
//! One test per row of `ERRORS.md`. The C function returns `void` and contains
//! zero explicit rejection paths, so "the same error" means one of two things:
//!
//!   * rows 1-14: the C silently ACCEPTS an input a caller would call invalid,
//!     and the Rust must produce the identical bit pattern — asserting a
//!     specific value, not merely "neither errored";
//!   * rows 15-18: the C leaves a pointer precondition unchecked, and both
//!     libraries must die with the SAME fatal signal (checked from a forked
//!     child, and additionally asserted to be exactly `SIGSEGV`, not merely
//!     "both died somehow").

mod common;

use common::*;

const ITERS: usize = 20_000;

/// Saturation guaranteed non-zero, so the `s == 0` fast path is bypassed.
fn sat(rng: &mut Rng) -> f32 {
    let v = rng.range(f32::MIN_POSITIVE, 1.0);
    if v == 0.0 { 1.0 } else { v }
}

// ---------------------------------------------------------------------------
// Rows 1-14: silently-accepted invalid input
// ---------------------------------------------------------------------------

/// Row 1: `h < 0` is below the documented range. No rejection — and it lands in
/// arm 3, whose predicate (`h < 120 && h < 180`) makes it the ONLY reachable
/// path for negative hue.
#[test]
fn err_row01_negative_hue() {
    let mut rng = Rng::new(101);
    let mut inputs = Vec::new();
    // Hand-picked boundary values plus randomized magnitudes.
    for &h in &[-0.0f32, -1.0, -1.0e-45, -59.9, -60.0, -120.0, -360.0, -1.0e30] {
        for _ in 0..500 {
            inputs.push([h, sat(&mut rng), rng.range(0.0, 1.0)]);
        }
    }
    for _ in 0..ITERS {
        let h = -rng.range(f32::MIN_POSITIVE, 1.0e9);
        inputs.push([h, sat(&mut rng), rng.range(-1.0, 2.0)]);
    }
    assert_same_batch("ERRORS row 1", inputs);
}

/// Row 2: `h >= 360`, i.e. one step past the top of the valid range and beyond.
/// No wraparound, no rejection: the terminal `else` yields flat grey.
#[test]
fn err_row02_hue_at_or_above_360() {
    let mut rng = Rng::new(102);
    let mut inputs = Vec::new();
    for &h in &[
        360.0f32,
        f32::from_bits(360.0f32.to_bits() + 1), // nextafter(360, +inf)
        360.5,
        720.0,
        1.0e9,
        f32::MAX,
    ] {
        for _ in 0..500 {
            inputs.push([h, sat(&mut rng), rng.range(0.0, 1.0)]);
        }
    }
    for _ in 0..ITERS {
        inputs.push([rng.range(360.0, 1.0e12), sat(&mut rng), rng.range(-1.0, 2.0)]);
    }
    assert_same_batch("ERRORS row 2", inputs);
}

/// Row 3: `h ∈ [120, 180)` — the range the arm-3 typo orphans. Must come out as
/// `(m, m, m)`, NOT as the `(m, c+m, x+m)` the author evidently intended.
#[test]
fn err_row03_hue_120_to_180_dead_range() {
    let mut rng = Rng::new(103);
    let mut inputs = Vec::new();
    for &h in &[
        120.0f32,
        f32::from_bits(120.0f32.to_bits() + 1),
        150.0,
        f32::from_bits(180.0f32.to_bits() - 1), // nextafter(180, -inf)
    ] {
        for _ in 0..500 {
            inputs.push([h, sat(&mut rng), rng.range(0.0, 1.0)]);
        }
    }
    for _ in 0..ITERS {
        inputs.push([rng.range(120.0, 180.0), sat(&mut rng), rng.range(0.0, 1.0)]);
    }
    assert_same_batch("ERRORS row 3", inputs);

    // Additionally pin the C's actual semantics so a future "fix" of the typo
    // cannot pass this file: R and B must both equal G's grey value.
    let p = pair();
    let src = [150.0f32, 1.0f32, 0.5f32];
    let mut out = [f32::NAN; 3];
    // SAFETY: 3 readable / 3 writable floats.
    unsafe { (p.rust)(out.as_mut_ptr(), src.as_ptr()) };
    assert_eq!(
        out[0].to_bits(),
        out[1].to_bits(),
        "ERRORS row 3: hue 150 must be flat grey (the C's arm-3 typo is not fixed)"
    );
    assert_eq!(out[1].to_bits(), out[2].to_bits(), "ERRORS row 3: flat grey");
}

/// Row 4: `h = NaN`. `comiss` reports unordered, so all six predicates are
/// false and control reaches the terminal `else`. No rejection, no trap.
#[test]
fn err_row04_hue_nan() {
    let mut rng = Rng::new(104);
    let nans: Vec<f32> = EDGE_BITS
        .iter()
        .filter(|&&b| f32::from_bits(b).is_nan())
        .map(|&b| f32::from_bits(b))
        .collect();
    assert!(!nans.is_empty());
    let mut inputs = Vec::new();
    for &h in &nans {
        for _ in 0..1000 {
            inputs.push([h, sat(&mut rng), rng.range(-1.0, 2.0)]);
        }
    }
    // Randomized NaN payloads too, both signs.
    for _ in 0..ITERS {
        let payload = rng.next_u32() & 0x007F_FFFF;
        let sign = (rng.next_u32() & 1) << 31;
        let h = f32::from_bits(sign | 0x7F80_0000 | payload.max(1));
        inputs.push([h, sat(&mut rng), rng.range(0.0, 1.0)]);
    }
    assert_same_batch("ERRORS row 4", inputs);
}

/// Row 5: `h = ±inf`. `h/60` is `±inf` and glibc `fmodf(±inf, 2)` returns NaN
/// (setting `EDOM`, which the C never reads). `-inf` additionally satisfies
/// arm 3, so a NaN `x` actually reaches the output there.
#[test]
fn err_row05_hue_infinite() {
    let mut rng = Rng::new(105);
    let mut inputs = Vec::new();
    for &h in &[f32::INFINITY, f32::NEG_INFINITY] {
        for _ in 0..2000 {
            inputs.push([h, sat(&mut rng), rng.range(-2.0, 3.0)]);
        }
        for &l in edge_values().iter() {
            inputs.push([h, 0.5, l]);
            inputs.push([h, 1.0, l]);
        }
    }
    assert_same_batch("ERRORS row 5", inputs);
}

/// Row 6: `s` outside `[0, 1]`, including `±inf`. No clamping, no rejection.
#[test]
fn err_row06_saturation_out_of_range() {
    let mut rng = Rng::new(106);
    let mut inputs = Vec::new();
    for &s in &[
        -1.0f32,
        -0.5,
        -1.0e-45,
        f32::from_bits(1.0f32.to_bits() + 1), // nextafter(1, +inf)
        1.5,
        100.0,
        f32::MAX,
        f32::MIN,
        f32::INFINITY,
        f32::NEG_INFINITY,
    ] {
        for _ in 0..1000 {
            inputs.push([rng.range(-400.0, 400.0), s, rng.range(-1.0, 2.0)]);
        }
    }
    for _ in 0..ITERS {
        let s = if rng.next_u32() & 1 == 0 {
            rng.range(-1.0e6, -f32::MIN_POSITIVE)
        } else {
            rng.range(1.0, 1.0e6)
        };
        inputs.push([rng.range(-400.0, 400.0), s, rng.range(-1.0, 2.0)]);
    }
    assert_same_batch("ERRORS row 6", inputs);
}

/// Row 7: `s = NaN`. `s == 0` is unordered → false, so the fast path is skipped
/// and the NaN propagates as the SOURCE operand of `mulss` (dest is the finite
/// `1 - |2l-1|`), which returns `s` quieted. Sign and payload must match.
#[test]
fn err_row07_saturation_nan() {
    let mut rng = Rng::new(107);
    let mut inputs = Vec::new();
    for &b in EDGE_BITS {
        let s = f32::from_bits(b);
        if !s.is_nan() {
            continue;
        }
        for _ in 0..1000 {
            inputs.push([rng.range(-400.0, 400.0), s, rng.range(-1.0, 2.0)]);
        }
    }
    for _ in 0..ITERS {
        let payload = (rng.next_u32() & 0x007F_FFFF).max(1);
        let sign = (rng.next_u32() & 1) << 31;
        let s = f32::from_bits(sign | 0x7F80_0000 | payload);
        inputs.push([rng.range(-400.0, 400.0), s, rng.range(0.0, 1.0)]);
    }
    assert_same_batch("ERRORS row 7", inputs);
}

/// Row 8: `s = -0.0`. The fast path IS taken (`-0.0 == 0` in IEEE), so `l` is
/// copied through verbatim — even a NaN or infinite `l`.
#[test]
fn err_row08_saturation_negative_zero() {
    let mut rng = Rng::new(108);
    let mut inputs = Vec::new();
    for &l in edge_values().iter() {
        for _ in 0..200 {
            inputs.push([rng.any_bits(), -0.0f32, l]);
        }
    }
    for _ in 0..ITERS {
        inputs.push([rng.any_bits(), -0.0f32, rng.any_bits()]);
    }
    assert_same_batch("ERRORS row 8", inputs);

    // Pin the semantics: the fast path must forward `l` bit-for-bit.
    let p = pair();
    for &l in edge_values().iter() {
        let src = [123.0f32, -0.0f32, l];
        let mut out = [0.0f32; 3];
        // SAFETY: 3 readable / 3 writable floats.
        unsafe { (p.rust)(out.as_mut_ptr(), src.as_ptr()) };
        assert_eq!(
            out.map(f32::to_bits),
            [l.to_bits(); 3],
            "ERRORS row 8: s=-0.0 must take the fast path and forward l verbatim"
        );
    }
}

/// Row 9: `l` outside `[0, 1]`. `|2l-1| > 1` drives `c` negative; no clamping.
#[test]
fn err_row09_lightness_out_of_range() {
    let mut rng = Rng::new(109);
    let mut inputs = Vec::new();
    for &l in &[
        -1.0f32,
        -0.5,
        -1.0e-45,
        f32::from_bits(1.0f32.to_bits() + 1),
        1.5,
        100.0,
        f32::MAX,
        f32::MIN,
    ] {
        for _ in 0..1000 {
            inputs.push([rng.range(-400.0, 400.0), sat(&mut rng), l]);
        }
    }
    for _ in 0..ITERS {
        let l = if rng.next_u32() & 1 == 0 {
            rng.range(-1.0e6, -f32::MIN_POSITIVE)
        } else {
            rng.range(1.0, 1.0e6)
        };
        inputs.push([rng.range(-400.0, 400.0), sat(&mut rng), l]);
    }
    assert_same_batch("ERRORS row 9", inputs);
}

/// Row 10: `l = ±inf`. Produces `c = ∓inf` and then `m = inf - inf = NaN` for
/// some sign combinations. Whatever NaN the C emits must match exactly.
#[test]
fn err_row10_lightness_infinite() {
    let mut rng = Rng::new(110);
    let mut inputs = Vec::new();
    for &l in &[f32::INFINITY, f32::NEG_INFINITY] {
        for _ in 0..2000 {
            inputs.push([rng.range(-400.0, 400.0), sat(&mut rng), l]);
        }
        // Cross with every edge saturation and edge hue.
        for &s in edge_values_nonzero().iter() {
            for &h in edge_values().iter() {
                inputs.push([h, s, l]);
            }
        }
    }
    assert_same_batch("ERRORS row 10", inputs);
}

/// Row 11: `l = NaN`. The interesting case: `c` and `x` acquire sign 0 (the
/// `andps` in `fabsf` clears it) while `m` re-propagates `l` and keeps `l`'s
/// sign, so `c + m` and `m + c` are DIFFERENT bit patterns. Every arm's operand
/// order is under test here.
#[test]
fn err_row11_lightness_nan() {
    let mut rng = Rng::new(111);
    let mut inputs = Vec::new();
    let nan_bits: Vec<u32> = EDGE_BITS
        .iter()
        .copied()
        .filter(|&b| f32::from_bits(b).is_nan())
        .collect();

    // Every NaN pattern crossed with a hue in EVERY arm, including the
    // thresholds, so all seven output formulas are exercised with NaN operands.
    let arm_hues: Vec<f32> = vec![
        -1.0, -1.0e9, 0.0, 30.0, 59.9, 60.0, 90.0, 119.9, 120.0, 150.0, 179.9, 180.0, 210.0,
        239.9, 240.0, 270.0, 299.9, 300.0, 330.0, 359.9, 360.0, 1.0e9,
    ];
    for &b in &nan_bits {
        let l = f32::from_bits(b);
        for &h in &arm_hues {
            for _ in 0..40 {
                inputs.push([h, sat(&mut rng), l]);
            }
        }
    }
    // Randomized NaN payloads, both signs, random hue.
    for _ in 0..ITERS {
        let payload = (rng.next_u32() & 0x007F_FFFF).max(1);
        let sign = (rng.next_u32() & 1) << 31;
        let l = f32::from_bits(sign | 0x7F80_0000 | payload);
        inputs.push([rng.range(-400.0, 400.0), sat(&mut rng), l]);
    }
    assert_same_batch("ERRORS row 11", inputs);
}

/// Row 12: signalling NaN in any slot. SSE exceptions are masked, so nothing
/// traps; the sNaN is quieted (`| 0x0040_0000`) by its first consumer, keeping
/// sign and payload.
#[test]
fn err_row12_signalling_nan() {
    let snans: [f32; 4] = [
        f32::from_bits(0x7FA0_0000),
        f32::from_bits(0xFFA0_0000),
        f32::from_bits(0x7F80_0001),
        f32::from_bits(0xFF80_0001),
    ];
    let mut rng = Rng::new(112);
    let mut inputs = Vec::new();
    for &n in &snans {
        for _ in 0..1000 {
            inputs.push([n, sat(&mut rng), rng.range(0.0, 1.0)]);
            inputs.push([rng.range(-400.0, 400.0), n, rng.range(0.0, 1.0)]);
            inputs.push([rng.range(-400.0, 400.0), sat(&mut rng), n]);
        }
        // All-sNaN and pairwise combinations.
        for &m in &snans {
            inputs.push([n, m, n]);
            inputs.push([n, n, m]);
            inputs.push([m, n, n]);
        }
    }
    assert_same_batch("ERRORS row 12", inputs);
}

/// Row 13: subnormal / minimum-magnitude inputs. No flush-to-zero under the
/// default MXCSR, and `s = MIN_POSITIVE` is not `== 0`, so the slow path runs
/// with a subnormal `c`.
#[test]
fn err_row13_subnormal_inputs() {
    let tiny: [f32; 8] = [
        f32::from_bits(0x0000_0001),
        f32::from_bits(0x8000_0001),
        f32::from_bits(0x0000_0002),
        f32::from_bits(0x007F_FFFF),
        f32::from_bits(0x807F_FFFF),
        f32::MIN_POSITIVE,
        -f32::MIN_POSITIVE,
        f32::from_bits(0x0040_0000),
    ];
    let mut rng = Rng::new(113);
    let mut inputs = Vec::new();
    for &a in &tiny {
        for _ in 0..500 {
            inputs.push([a, sat(&mut rng), rng.range(0.0, 1.0)]);
            inputs.push([rng.range(-400.0, 400.0), a, rng.range(0.0, 1.0)]);
            inputs.push([rng.range(-400.0, 400.0), sat(&mut rng), a]);
        }
        // Exhaustive cross-product over the tiny pool, plus the zeros.
        for &b in &tiny {
            for &c in &tiny {
                inputs.push([a, b, c]);
            }
            inputs.push([a, b, 0.0]);
            inputs.push([a, 0.0, b]);
            inputs.push([a, b, -0.0]);
        }
    }
    assert_same_batch("ERRORS row 13", inputs);
}

/// Row 14: maximum-magnitude inputs, forcing silent overflow to `±inf` and
/// `inf - inf` NaNs mid-computation.
#[test]
fn err_row14_extremal_magnitudes() {
    let big: [f32; 6] = [
        f32::MAX,
        f32::MIN, // == -f32::MAX
        f32::from_bits(0x7F7F_FFFE),
        f32::from_bits(0xFF7F_FFFE),
        1.0e38,
        -1.0e38,
    ];
    let mut rng = Rng::new(114);
    let mut inputs = Vec::new();
    for &a in &big {
        for _ in 0..500 {
            inputs.push([a, sat(&mut rng), rng.range(0.0, 1.0)]);
            inputs.push([rng.range(-400.0, 400.0), a, rng.range(0.0, 1.0)]);
            inputs.push([rng.range(-400.0, 400.0), sat(&mut rng), a]);
        }
        for &b in &big {
            for &c in &big {
                inputs.push([a, b, c]);
            }
        }
    }
    assert_same_batch("ERRORS row 14", inputs);
}

// ---------------------------------------------------------------------------
// Rows 15-18: unchecked pointer preconditions (fatal-signal differential)
// ---------------------------------------------------------------------------

/// Child-mode entry point. In a normal run the env var is absent and this test
/// does nothing; `assert_same_fatal` re-executes this binary with
/// `--exact crash_child` and the env var set, so the fault happens in a
/// throwaway process.
#[test]
fn crash_child() {
    let _ = run_as_crash_child_if_requested();
}

/// Row 15: `src == NULL`. Line 6 of the C dereferences it with no check.
#[test]
fn err_row15_null_src_faults() {
    assert_same_fatal("ERRORS row 15", CrashCase::NullSrc);
}

/// Row 16: `dest == NULL` on the slow path.
#[test]
fn err_row16_null_dest_faults() {
    assert_same_fatal("ERRORS row 16", CrashCase::NullDest);
}

/// Row 17: `dest == NULL` on the `s == 0` fast path — which is NOT a "no write"
/// path; it still stores three floats.
#[test]
fn err_row17_null_dest_fast_path_faults() {
    assert_same_fatal("ERRORS row 17", CrashCase::NullDestFastPath);
}

/// Row 18: both pointers `NULL`; the `src` read faults first.
#[test]
fn err_row18_both_null_faults() {
    assert_same_fatal("ERRORS row 18", CrashCase::BothNull);
}
