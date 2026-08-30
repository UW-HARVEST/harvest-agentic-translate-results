//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`, in order. Every test loads BOTH the C
//! `.so` and the Rust `.so` through `libloading` and compares their results
//! across the FFI boundary; the Rust code is never called directly.
//!
//! Rows that admit a value range are driven with many randomized inputs from a
//! fixed-seed SplitMix64 stream, so failures are reproducible.

mod common;

use common::{Pair, Rng};

const I32_MAX: i32 = i32::MAX;
const I32_MIN: i32 = i32::MIN;

/// How many randomized samples each value-range row draws.
const SAMPLES: usize = 400;

// ---------------------------------------------------------------------------
// Rows 1-8: `static_sum`, fresh instance, single call
// ---------------------------------------------------------------------------

/// Row 1 — fresh instance, single call, `update == 0` must yield 0.
#[test]
fn row01_sum_fresh_zero() {
    let p = Pair::fresh("row 1: static_sum fresh, update = 0");
    assert_eq!(p.assert_sum(0), 0, "a fresh accumulator plus 0 must be 0");
}

/// Row 2 — fresh instance, single call, `update == 1`.
#[test]
fn row02_sum_fresh_one() {
    let p = Pair::fresh("row 2: static_sum fresh, update = 1");
    assert_eq!(p.assert_sum(1), 1);
}

/// Row 3 — fresh instance, single call, `update == -1`.
#[test]
fn row03_sum_fresh_minus_one() {
    let p = Pair::fresh("row 3: static_sum fresh, update = -1");
    assert_eq!(p.assert_sum(-1), -1);
}

/// Row 4 — fresh instance, single call, randomized small positive `update`.
/// Each sample needs its own fresh pair, since the first call is the only one
/// that sees `sum == 0`.
#[test]
fn row04_sum_fresh_small_positive() {
    let mut rng = Rng::for_row(4);
    for _ in 0..SAMPLES {
        let p = Pair::fresh("row 4: static_sum fresh, small positive update");
        let u = rng.i32_in(1, 1000);
        assert_eq!(p.assert_sum(u), u);
    }
}

/// Row 5 — fresh instance, single call, randomized small negative `update`.
#[test]
fn row05_sum_fresh_small_negative() {
    let mut rng = Rng::for_row(5);
    for _ in 0..SAMPLES {
        let p = Pair::fresh("row 5: static_sum fresh, small negative update");
        let u = rng.i32_in(-1000, -1);
        assert_eq!(p.assert_sum(u), u);
    }
}

/// Row 6 — fresh instance, single call, randomized full-range `i32` `update`.
#[test]
fn row06_sum_fresh_full_range() {
    let mut rng = Rng::for_row(6);
    for _ in 0..SAMPLES {
        let p = Pair::fresh("row 6: static_sum fresh, full-range i32 update");
        let u = rng.i32_any();
        assert_eq!(p.assert_sum(u), u);
    }
}

/// Row 7 — fresh instance, `update == INT_MAX`.
#[test]
fn row07_sum_fresh_int_max() {
    let p = Pair::fresh("row 7: static_sum fresh, update = INT_MAX");
    assert_eq!(p.assert_sum(I32_MAX), I32_MAX);
}

/// Row 8 — fresh instance, `update == INT_MIN`.
#[test]
fn row08_sum_fresh_int_min() {
    let p = Pair::fresh("row 8: static_sum fresh, update = INT_MIN");
    assert_eq!(p.assert_sum(I32_MIN), I32_MIN);
}

// ---------------------------------------------------------------------------
// Rows 9-13: `static_sum` accumulation over call sequences
// ---------------------------------------------------------------------------

/// Row 9 — fresh instance, two randomized calls.
#[test]
fn row09_sum_two_calls() {
    let mut rng = Rng::for_row(9);
    for _ in 0..SAMPLES {
        let p = Pair::fresh("row 9: static_sum, n = 2 accumulation");
        let a = rng.i32_any();
        let b = rng.i32_any();
        assert_eq!(p.assert_sum(a), a);
        assert_eq!(p.assert_sum(b), a.wrapping_add(b));
    }
}

/// Row 10 — n = 10 all-positive sequence chosen so the running total cannot
/// leave the `i32` range.
#[test]
fn row10_sum_ten_positive_in_range() {
    let mut rng = Rng::for_row(10);
    for _ in 0..64 {
        let p = Pair::fresh("row 10: static_sum, n = 10 positive, no overflow");
        let mut expect: i32 = 0;
        for _ in 0..10 {
            // Cap each step so 10 steps stay well inside i32.
            let u = rng.i32_in(1, I32_MAX / 16);
            expect = expect.wrapping_add(u);
            assert_eq!(p.assert_sum(u), expect);
        }
        assert!(expect > 0, "the running total should have stayed positive");
    }
}

/// Row 11 — n = 10 all-negative sequence.
#[test]
fn row11_sum_ten_negative() {
    let mut rng = Rng::for_row(11);
    for _ in 0..64 {
        let p = Pair::fresh("row 11: static_sum, n = 10 negative");
        let mut expect: i32 = 0;
        for _ in 0..10 {
            let u = rng.i32_in(-(I32_MAX / 16), -1);
            expect = expect.wrapping_add(u);
            assert_eq!(p.assert_sum(u), expect);
        }
        assert!(expect < 0);
    }
}

/// Row 12 — n = 256 mixed-sign sequence that repeatedly crosses zero.
#[test]
fn row12_sum_mixed_sign_crossing_zero() {
    let mut rng = Rng::for_row(12);
    for _ in 0..16 {
        let p = Pair::fresh("row 12: static_sum, n = 256 mixed sign");
        let mut expect: i32 = 0;
        let mut crossings = 0usize;
        for step in 0..256 {
            // Every other step is deliberately biased against the accumulator's
            // current sign, with a magnitude large enough to overshoot zero, so
            // the row genuinely exercises repeated sign changes instead of
            // relying on a random walk happening to cross.
            let u = if step % 2 == 0 {
                rng.i32_in(-100_000, 100_000)
            } else {
                let overshoot = expect.unsigned_abs().min(1_000_000) as i32 + rng.i32_in(1, 1000);
                if expect >= 0 {
                    -overshoot
                } else {
                    overshoot
                }
            };
            let prev = expect;
            expect = expect.wrapping_add(u);
            if (prev < 0) != (expect < 0) {
                crossings += 1;
            }
            assert_eq!(p.assert_sum(u), expect);
        }
        assert!(
            crossings > 0,
            "sequence should cross zero at least once (got {crossings})"
        );
    }
}

/// Row 13 — n = 1000 full-range sequence; the accumulator wraps many times.
#[test]
fn row13_sum_full_range_sequence_wraps() {
    let mut rng = Rng::for_row(13);
    for _ in 0..8 {
        let p = Pair::fresh("row 13: static_sum, n = 1000 full-range, wraps");
        let mut expect: i32 = 0;
        let mut wraps = 0usize;
        for _ in 0..1000 {
            let u = rng.i32_any();
            let (next, overflowed) = expect.overflowing_add(u);
            if overflowed {
                wraps += 1;
            }
            expect = next;
            assert_eq!(p.assert_sum(u), expect);
        }
        assert!(wraps > 0, "full-range sequence should overflow repeatedly");
    }
}

// ---------------------------------------------------------------------------
// Rows 14-17: `static_sum` on a pre-driven accumulator
// ---------------------------------------------------------------------------

/// Row 14 — accumulator seeded to exactly `INT_MAX`, then a randomized update.
#[test]
fn row14_sum_from_int_max() {
    let mut rng = Rng::for_row(14);
    for _ in 0..SAMPLES {
        let p = Pair::fresh("row 14: static_sum from sum = INT_MAX");
        p.seed_to(I32_MAX);
        let u = rng.i32_any();
        assert_eq!(p.assert_sum(u), I32_MAX.wrapping_add(u));
    }
}

/// Row 15 — accumulator seeded to exactly `INT_MIN`, then a randomized update.
#[test]
fn row15_sum_from_int_min() {
    let mut rng = Rng::for_row(15);
    for _ in 0..SAMPLES {
        let p = Pair::fresh("row 15: static_sum from sum = INT_MIN");
        p.seed_to(I32_MIN);
        let u = rng.i32_any();
        assert_eq!(p.assert_sum(u), I32_MIN.wrapping_add(u));
    }
}

/// Row 16 — positive accumulator, then a large negative update crossing zero.
#[test]
fn row16_sum_positive_then_large_negative() {
    let mut rng = Rng::for_row(16);
    for _ in 0..SAMPLES {
        let p = Pair::fresh("row 16: static_sum, positive sum crossed downward");
        let seed = rng.i32_in(1, I32_MAX);
        p.seed_to(seed);
        let u = rng.i32_in(I32_MIN, -1);
        assert_eq!(p.assert_sum(u), seed.wrapping_add(u));
    }
}

/// Row 17 — negative accumulator, then a large positive update crossing zero.
#[test]
fn row17_sum_negative_then_large_positive() {
    let mut rng = Rng::for_row(17);
    for _ in 0..SAMPLES {
        let p = Pair::fresh("row 17: static_sum, negative sum crossed upward");
        let seed = rng.i32_in(I32_MIN, -1);
        p.seed_to(seed);
        let u = rng.i32_in(1, I32_MAX);
        assert_eq!(p.assert_sum(u), seed.wrapping_add(u));
    }
}

/// Row 18 — argument arrives in a 64-bit register with the high half set; both
/// sides must agree on how the `int` parameter is truncated.
#[test]
fn row18_sum_wide_argument_truncation() {
    let mut rng = Rng::for_row(18);
    let fixed: [i64; 8] = [
        0x0000_0001_0000_0005,
        0x7FFF_FFFF_0000_0000,
        -1,
        i64::MIN,
        i64::MAX,
        0xFFFF_FFFF_7FFF_FFFFu64 as i64,
        0x0000_0000_8000_0000,
        0x1234_5678_9ABC_DEF0,
    ];
    for &w in &fixed {
        let p = Pair::fresh("row 18: static_sum wide-argument truncation (fixed)");
        p.assert_sum_wide(w);
    }
    for _ in 0..SAMPLES {
        let p = Pair::fresh("row 18: static_sum wide-argument truncation (random)");
        let w = ((rng.next_u64() as i64) & !0xFFFF_FFFF) | (rng.i32_any() as u32 as i64);
        p.assert_sum_wide(w);
    }
}

// ---------------------------------------------------------------------------
// Rows 19-28: `driver` on a fresh instance
// ---------------------------------------------------------------------------

/// Row 19 — `stride == 0`: ten lines, all `0`.
#[test]
fn row19_driver_stride_zero() {
    let p = Pair::fresh("row 19: driver, stride = 0");
    let out = p.assert_driver(0);
    assert_eq!(out, b"0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n");
}

/// Row 20 — `stride == 1`: the canonical triangular numbers.
#[test]
fn row20_driver_stride_one() {
    let p = Pair::fresh("row 20: driver, stride = 1");
    let out = p.assert_driver(1);
    assert_eq!(out, b"0\n1\n3\n6\n10\n15\n21\n28\n36\n45\n");
}

/// Row 21 — `stride == -1`: negated triangular numbers.
#[test]
fn row21_driver_stride_minus_one() {
    let p = Pair::fresh("row 21: driver, stride = -1");
    let out = p.assert_driver(-1);
    assert_eq!(out, b"0\n-1\n-3\n-6\n-10\n-15\n-21\n-28\n-36\n-45\n");
}

/// Row 22 — randomized small positive `stride`.
#[test]
fn row22_driver_small_positive_stride() {
    let mut rng = Rng::for_row(22);
    for _ in 0..64 {
        let p = Pair::fresh("row 22: driver, small positive stride");
        let stride = rng.i32_in(1, 1000);
        let out = p.assert_driver(stride);
        assert_eq!(out, expected_driver_output(0, stride).into_bytes());
    }
}

/// Row 23 — randomized small negative `stride`.
#[test]
fn row23_driver_small_negative_stride() {
    let mut rng = Rng::for_row(23);
    for _ in 0..64 {
        let p = Pair::fresh("row 23: driver, small negative stride");
        let stride = rng.i32_in(-1000, -1);
        let out = p.assert_driver(stride);
        assert_eq!(out, expected_driver_output(0, stride).into_bytes());
    }
}

/// Row 24 — randomized full-range `stride`; both `i * stride` and the running
/// total wrap.
#[test]
fn row24_driver_full_range_stride() {
    let mut rng = Rng::for_row(24);
    for _ in 0..128 {
        let p = Pair::fresh("row 24: driver, full-range i32 stride");
        let stride = rng.i32_any();
        let out = p.assert_driver(stride);
        assert_eq!(out, expected_driver_output(0, stride).into_bytes());
    }
}

/// Row 25 — `stride == INT_MAX`; `i * stride` overflows from `i == 2` on.
#[test]
fn row25_driver_stride_int_max() {
    let p = Pair::fresh("row 25: driver, stride = INT_MAX");
    let out = p.assert_driver(I32_MAX);
    assert_eq!(out, expected_driver_output(0, I32_MAX).into_bytes());
    assert_eq!(out.iter().filter(|&&b| b == b'\n').count(), 10);
}

/// Row 26 — `stride == INT_MIN`.
#[test]
fn row26_driver_stride_int_min() {
    let p = Pair::fresh("row 26: driver, stride = INT_MIN");
    let out = p.assert_driver(I32_MIN);
    assert_eq!(out, expected_driver_output(0, I32_MIN).into_bytes());
    assert_eq!(out.iter().filter(|&&b| b == b'\n').count(), 10);
}

/// Row 27 — `stride == INT_MAX / 9`: every product `i * stride` fits (the
/// largest multiplier in the loop is `i == 9`), but the accumulated total
/// (`45 * stride`) overflows partway through the loop. `INT_MAX / 8` would NOT
/// satisfy the premise: `9 * (INT_MAX / 8)` already overflows.
#[test]
fn row27_driver_products_fit_accumulator_overflows() {
    for stride in [I32_MAX / 9, I32_MAX / 10, I32_MIN / 9, I32_MIN / 10] {
        let p = Pair::fresh(format!(
            "row 27: driver({stride}), products fit but accumulator overflows"
        ));
        let out = p.assert_driver(stride);
        assert_eq!(out, expected_driver_output(0, stride).into_bytes());

        // Confirm the row really is testing what it claims.
        let mut sum: i32 = 0;
        let mut saw_overflow = false;
        for i in 0..10i32 {
            assert!(
                i.checked_mul(stride).is_some(),
                "product i={i} * stride={stride} must not overflow"
            );
            let (n, o) = sum.overflowing_add(i.wrapping_mul(stride));
            saw_overflow |= o;
            sum = n;
        }
        assert!(
            saw_overflow,
            "the accumulator was expected to overflow for stride = {stride}"
        );
    }
}

/// Row 28 — powers of two near the wrap boundary.
#[test]
fn row28_driver_power_of_two_strides() {
    for shift in [28u32, 29, 30] {
        let stride = 1i32 << shift;
        let p = Pair::fresh(format!("row 28: driver, stride = 1 << {shift}"));
        let out = p.assert_driver(stride);
        assert_eq!(out, expected_driver_output(0, stride).into_bytes());
    }
    for shift in [28u32, 29, 30] {
        let stride = -(1i32 << shift);
        let p = Pair::fresh(format!("row 28: driver, stride = -(1 << {shift})"));
        let out = p.assert_driver(stride);
        assert_eq!(out, expected_driver_output(0, stride).into_bytes());
    }
}

// ---------------------------------------------------------------------------
// Rows 29-33: `driver` with carried-in state, and interleaving
// ---------------------------------------------------------------------------

/// Row 29 — `driver` on an instance whose accumulator was already moved by
/// `static_sum`; stdout must reflect the carried-in value.
#[test]
fn row29_driver_with_carried_in_sum() {
    let mut rng = Rng::for_row(29);
    for _ in 0..128 {
        let p = Pair::fresh("row 29: driver after static_sum, carried-in sum");
        let seed = rng.i32_any();
        p.seed_to(seed);
        let stride = rng.i32_any();
        let out = p.assert_driver(stride);
        assert_eq!(out, expected_driver_output(seed, stride).into_bytes());
    }
}

/// Row 30 — eight consecutive `driver` calls on the SAME instance.
#[test]
fn row30_driver_repeated_same_instance() {
    let mut rng = Rng::for_row(30);
    for _ in 0..16 {
        let p = Pair::fresh("row 30: driver called 8x on one instance");
        let mut sum: i32 = 0;
        for _ in 0..8 {
            let stride = rng.i32_any();
            let out = p.assert_driver(stride);
            let expect = expected_driver_output(sum, stride);
            assert_eq!(out, expect.into_bytes());
            sum = accumulate_driver(sum, stride);
        }
    }
}

/// Row 31 — `static_sum`, then `driver`, then `static_sum`: both the returned
/// integers and the emitted bytes are compared.
#[test]
fn row31_interleaved_sum_driver_sum() {
    let mut rng = Rng::for_row(31);
    for _ in 0..128 {
        let p = Pair::fresh("row 31: static_sum / driver / static_sum interleaved");
        let a = rng.i32_any();
        let stride = rng.i32_any();
        let b = rng.i32_any();

        assert_eq!(p.assert_sum(a), a);
        let out = p.assert_driver(stride);
        assert_eq!(out, expected_driver_output(a, stride).into_bytes());
        let after_driver = accumulate_driver(a, stride);
        assert_eq!(p.assert_sum(b), after_driver.wrapping_add(b));
    }
}

/// Row 32 — a long finely-alternated sequence of randomly chosen entry points,
/// checking every return value and every stdout byte against the C `.so`.
#[test]
fn row32_long_alternating_sequence() {
    let mut rng = Rng::for_row(32);
    let p = Pair::fresh("row 32: 200-step random interleaving of both entry points");
    let mut sum: i32 = 0;
    let mut used_driver = 0usize;
    let mut used_sum = 0usize;

    for _ in 0..200 {
        let arg = rng.i32_any();
        if rng.bool() {
            let out = p.assert_driver(arg);
            assert_eq!(out, expected_driver_output(sum, arg).into_bytes());
            sum = accumulate_driver(sum, arg);
            used_driver += 1;
        } else {
            sum = sum.wrapping_add(arg);
            assert_eq!(p.assert_sum(arg), sum);
            used_sum += 1;
        }
    }
    assert!(used_driver > 0 && used_sum > 0, "both entry points must be hit");
}

/// Row 33 — stdout byte-exactness: exactly ten `%d\n` records, no padding, no
/// thousands separators, no trailing extras.
#[test]
fn row33_driver_stdout_byte_shape() {
    let mut rng = Rng::for_row(33);
    for _ in 0..64 {
        let p = Pair::fresh("row 33: driver stdout byte shape");
        let stride = rng.i32_any();
        let out = p.assert_driver(stride);

        assert_eq!(
            out.iter().filter(|&&b| b == b'\n').count(),
            10,
            "driver must emit exactly 10 newline-terminated records"
        );
        assert_eq!(out.last(), Some(&b'\n'), "output must end with a newline");
        for b in &out {
            assert!(
                b.is_ascii_digit() || *b == b'-' || *b == b'\n',
                "unexpected byte {b:#04x} in driver output — no padding, spaces, \
                 or locale grouping is allowed"
            );
        }
        let text = String::from_utf8(out).expect("ascii");
        for line in text.lines() {
            assert!(!line.is_empty());
            assert!(!line.starts_with('+'));
            assert!(line.parse::<i32>().is_ok(), "line {line:?} must be a plain i32");
        }
    }
}

// ---------------------------------------------------------------------------
// Row 34: broad randomized sweep
// ---------------------------------------------------------------------------

/// Row 34 — 20000-call full-`i32` sweep on one shared pair, comparing every
/// return value.
#[test]
fn row34_bulk_random_sweep() {
    let mut rng = Rng::for_row(34);
    let p = Pair::fresh("row 34: 20000-call randomized static_sum sweep");
    let mut expect: i32 = 0;
    for _ in 0..20_000 {
        let u = rng.i32_any();
        expect = expect.wrapping_add(u);
        assert_eq!(p.assert_sum(u), expect);
    }
}

// ---------------------------------------------------------------------------
// Independent model of the C, used as a third opinion alongside the C `.so`
// ---------------------------------------------------------------------------

/// Reimplementation of:
/// ```c
/// void driver(int stride) {
///   for (int i = 0; i < 10; i++) printf("%d\n", static_sum(i * stride));
/// }
/// ```
/// starting from accumulator value `start`.
fn expected_driver_output(start: i32, stride: i32) -> String {
    let mut sum = start;
    let mut s = String::new();
    for i in 0..10i32 {
        sum = sum.wrapping_add(i.wrapping_mul(stride));
        s.push_str(&sum.to_string());
        s.push('\n');
    }
    s
}

/// The accumulator value left behind by `driver(stride)`.
fn accumulate_driver(start: i32, stride: i32) -> i32 {
    let mut sum = start;
    for i in 0..10i32 {
        sum = sum.wrapping_add(i.wrapping_mul(stride));
    }
    sum
}
