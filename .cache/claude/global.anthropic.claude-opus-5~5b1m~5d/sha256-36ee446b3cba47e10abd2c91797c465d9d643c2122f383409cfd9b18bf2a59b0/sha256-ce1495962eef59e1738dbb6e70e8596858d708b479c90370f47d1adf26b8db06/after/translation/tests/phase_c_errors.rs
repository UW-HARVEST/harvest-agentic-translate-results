//! Phase C - error-path differential tests, one test per row of `ERRORS.md`.
//!
//! `encode_quant` has exactly one `return` statement and no rejection path
//! (see `ERRORS.md` for the mechanical grep), so "same error/rejection" means:
//! for every invalid/out-of-range/UB-triggering input, BOTH libraries must
//! return the identical `int` sentinel-free result and neither may trap, abort
//! or panic. Each test below constructs one exact invalid condition.

mod common;

use common::*;

// ---------------------------------------------------------------------------
// Rows 1-3: out-of-range values for the `lsbit` mode "enum".
//
// `lsbit` is a mode selector with 4 meaningful variants (0 / 4 / odd / other
// even) but is declared `int`, so C accepts any int. These rows push values with
// no valid variant across the FFI boundary.
// ---------------------------------------------------------------------------

/// Every `lsbit` value in this list must land on the same branch in both libs.
fn lsbit_row(row: &str, lsbits: &[i32]) {
    let mut rng = Rng::for_row(row);
    for &lsbit in lsbits {
        // Sweep the whole canonical uni domain plus randomized shapes so the
        // branch is observed with many different data values, not just one.
        for uni in -16..=16i32 {
            for step in [-1000i32, -8, -1, 0, 1, 7, 8, 255, i32::MAX, i32::MIN] {
                for (pred, tgt, tgt2) in [
                    (0i32, 0i32, 0i32),
                    (100, 137, 90),
                    (-5000, 4999, -1),
                    (i32::MAX, i32::MIN, 0),
                    (i32::MIN, i32::MAX, i32::MAX),
                ] {
                    check(row, Args::new(uni, step, pred, tgt, tgt2, lsbit));
                }
            }
        }
        for _ in 0..5_000 {
            let u = U_CLASSES[(rng.next_u64() % 12) as usize];
            let v = V_CLASSES[(rng.next_u64() % 9) as usize];
            let uni = gen_uni(u, &mut rng);
            let (step, pred, tgt, tgt2) = gen_values(v, &mut rng);
            check(row, Args::new(uni, step, pred, tgt, tgt2, lsbit));
        }
    }
}

/// ERRORS.md row 1 - out-of-range positive "enum" values for `lsbit`.
#[test]
fn err01_lsbit_out_of_range_positive() {
    lsbit_row(
        "err01",
        &[2, 3, 5, 6, 7, 8, 9, 10, 11, 15, 16, 17, 100, 12345, 1 << 20],
    );
}

/// ERRORS.md row 2 - negative "enum" values for `lsbit` (two's-complement `&1`).
#[test]
fn err02_lsbit_negative_values() {
    lsbit_row("err02", &[-1, -2, -3, -4, -5, -6, -7, -8, -100, -12345]);
}

/// ERRORS.md row 3 - extreme `lsbit` values and one step either side of `4`.
#[test]
fn err03_lsbit_extremes_and_off_by_one_from_4() {
    lsbit_row(
        "err03",
        &[
            i32::MIN,
            i32::MIN + 1,
            i32::MAX,
            i32::MAX - 1,
            3,
            4,
            5,
            -4, // note: -4 != 4, so this must NOT take the dither branch
        ],
    );
    // Explicitly assert the `lsbit == 4` branch is distinguishable from its
    // neighbours in BOTH libraries (i.e. neither lib collapsed the 3-way switch).
    let a3 = Args::new(6, 100, 0, 33, 71, 3);
    let a4 = Args::new(6, 100, 0, 33, 71, 4);
    let a5 = Args::new(6, 100, 0, 33, 71, 5);
    let (c3, c4, c5) = (check("err03", a3), check("err03", a4), check("err03", a5));
    assert_eq!(c3, c5, "odd lsbit 3 and 5 take the same branch");
    assert_ne!(
        c4, c3,
        "lsbit==4 must be its own branch, distinct from odd lsbit"
    );
}

// ---------------------------------------------------------------------------
// Rows 4-5: signed overflow of `uni + 1` / `uni - 1`.
// ---------------------------------------------------------------------------

/// ERRORS.md row 4 - `uni == INT_MAX` makes `uni + 1` overflow (UB in C).
#[test]
fn err04_uni_plus_one_overflow_at_int_max() {
    let mut rng = Rng::for_row("err04");
    for lsbit in [0i32, 1, 2, 3, 4, 5, 6, 8, -1, -4, i32::MIN, i32::MAX] {
        for step in [
            i32::MIN,
            -0x1000_0000,
            -8,
            -1,
            0,
            1,
            7,
            8,
            255,
            0x1000_0000,
            i32::MAX,
        ] {
            for pred in EXTREMES {
                for tgt in EXTREMES {
                    check("err04", Args::new(i32::MAX, step, pred, tgt, 0, lsbit));
                    check("err04", Args::new(i32::MAX, step, pred, tgt, tgt, lsbit));
                }
            }
        }
    }
    for _ in 0..200_000 {
        check(
            "err04",
            Args::new(
                i32::MAX,
                rng.i32_any(),
                rng.i32_any(),
                rng.i32_any(),
                rng.i32_any(),
                rng.i32_any(),
            ),
        );
    }
    // INT_MAX has uni & 7 == 7, so the clamp guard must restore uni1 = uni; with
    // step == 0 all three candidates tie and the (lsbit-conditioned) uni is
    // returned. Both libs must agree on that exact value.
    let got = check("err04", Args::new(i32::MAX, 0, 0, 0, 0, 0));
    assert_eq!(got, i32::MAX, "step=0, lsbit=0 returns uni unchanged");
}

/// ERRORS.md row 5 - `uni == INT_MIN` makes `uni - 1` overflow (UB in C).
#[test]
fn err05_uni_minus_one_overflow_at_int_min() {
    let mut rng = Rng::for_row("err05");
    for lsbit in [0i32, 1, 2, 3, 4, 5, 6, 8, -1, -4, i32::MIN, i32::MAX] {
        for step in [
            i32::MIN,
            -0x1000_0000,
            -8,
            -1,
            0,
            1,
            7,
            8,
            255,
            0x1000_0000,
            i32::MAX,
        ] {
            for pred in EXTREMES {
                for tgt in EXTREMES {
                    check("err05", Args::new(i32::MIN, step, pred, tgt, 0, lsbit));
                    check("err05", Args::new(i32::MIN, step, pred, tgt, tgt, lsbit));
                }
            }
        }
    }
    for _ in 0..200_000 {
        check(
            "err05",
            Args::new(
                i32::MIN,
                rng.i32_any(),
                rng.i32_any(),
                rng.i32_any(),
                rng.i32_any(),
                rng.i32_any(),
            ),
        );
    }
    let got = check("err05", Args::new(i32::MIN, 0, 0, 0, 0, 0));
    assert_eq!(got, i32::MIN, "step=0, lsbit=0 returns uni unchanged");
}

// ---------------------------------------------------------------------------
// Rows 6-7: overflow inside the `diff` computation.
// ---------------------------------------------------------------------------

/// ERRORS.md row 6 - `(2 * (uni & 7) + 1) * step` overflows for large `step`.
/// The multiplier takes the values 1,3,5,7,9,11,13,15, so every
/// `step > INT_MAX / 15` overflows for some `uni`.
#[test]
fn err06_step_multiply_overflow() {
    // Overflow-triggering steps, including exact thresholds per multiplier.
    let mut steps: Vec<i32> = vec![
        i32::MAX,
        i32::MAX - 1,
        i32::MIN,
        i32::MIN + 1,
        0x1000_0000,
        0x2000_0000,
        0x4000_0000,
        0x7FFF_FFF8,
        -0x1000_0000,
        -0x4000_0000,
    ];
    for mult in [1i32, 3, 5, 7, 9, 11, 13, 15] {
        // The exact overflow threshold for this multiplier, and one step either
        // side of it. Use wrapping arithmetic: for mult == 1 the threshold IS
        // i32::MAX/i32::MIN, so `t + 1` would overflow the test itself.
        let t = i32::MAX / mult;
        let b = i32::MIN / mult;
        steps.push(t);
        steps.push(t.wrapping_add(1));
        steps.push(t.wrapping_sub(1));
        steps.push(t.wrapping_neg());
        steps.push(t.wrapping_neg().wrapping_sub(1));
        steps.push(b);
        steps.push(b.wrapping_sub(1));
        steps.push(b.wrapping_add(1));
    }
    let mut n = 0usize;
    for &step in &steps {
        // uni over the full canonical range so every multiplier 1..15 is used.
        for uni in 0..=15i32 {
            for lsbit in [0i32, 1, 2, 4, 8, -1, -4] {
                for pred in [0i32, 1, -1, 12345, i32::MAX, i32::MIN] {
                    for tgt in [0i32, -1, 7777, i32::MAX, i32::MIN] {
                        for tgt2 in [0i32, 1, i32::MAX, i32::MIN] {
                            check("err06", Args::new(uni, step, pred, tgt, tgt2, lsbit));
                            n += 1;
                        }
                    }
                }
            }
        }
    }
    assert!(n > 100_000, "err06 ran only {n} cases");
}

/// ERRORS.md row 7 - the `diff = -diff` negation branch (lines 32/38/44).
///
/// `diff == INT_MIN` (the only value whose negation overflows) is PROVEN
/// UNREACHABLE: `2*(uni&7)+1` is always odd, so the wrapped 32-bit product `P`
/// can be any `int`, but `diff = P / 8` is therefore bounded to
/// `[-2^28, 2^28-1]`, which excludes `INT_MIN`. This test
///   (a) verifies that bound mechanically for every multiplier, and
///   (b) drives `diff` to BOTH reachable extremes with `uni & 8` set so the
///       negation actually executes, and diff-checks C vs Rust there.
#[test]
fn err07_negate_diff_branch_and_no_negation_overflow() {
    // Modular inverse of an odd multiplier mod 2^32, so we can hit any product.
    fn inv_mod_2p32(m: u32) -> u32 {
        debug_assert!(m % 2 == 1);
        let mut inv: u32 = 1;
        for _ in 0..5 {
            inv = inv.wrapping_mul(2u32.wrapping_sub(m.wrapping_mul(inv)));
        }
        debug_assert_eq!(m.wrapping_mul(inv), 1);
        inv
    }

    let mut n = 0usize;
    let mut saw_min_diff = false;
    let mut saw_max_diff = false;

    for low in 0..8i32 {
        // bit3 set => the `if (uni & 8) diff = -diff;` branch is taken.
        let uni = 8 | low;
        let mult = (2 * (uni & 7) + 1) as u32;
        let inv = inv_mod_2p32(mult);

        // (a) The bound: check the extreme products for this multiplier.
        for want_product in [u32::MIN, 0x8000_0000u32, 0x7FFF_FFF8, u32::MAX, 0x7FFF_FFFF] {
            let step = want_product.wrapping_mul(inv) as i32;
            let produced = (mult as i32).wrapping_mul(step);
            assert_eq!(
                produced as u32, want_product,
                "failed to construct product 0x{want_product:08x} for mult {mult}"
            );
            let diff = produced / 8;
            assert!(
                diff >= -(1 << 28) && diff <= (1 << 28) - 1,
                "diff {diff} escaped [-2^28, 2^28-1]; ERRORS.md row 7 must be revised"
            );
            assert_ne!(
                diff,
                i32::MIN,
                "diff == INT_MIN is supposed to be unreachable"
            );
            if diff == -(1 << 28) {
                saw_min_diff = true;
            }
            if diff == (1 << 28) - 1 {
                saw_max_diff = true;
            }

            // (b) Exercise the negation branch at this extreme diff.
            for lsbit in [0i32, 2, 4, 8, -4, 1, -1] {
                for pred in EXTREMES {
                    for tgt in EXTREMES {
                        for tgt2 in [0i32, i32::MAX, i32::MIN, tgt] {
                            check("err07", Args::new(uni, step, pred, tgt, tgt2, lsbit));
                            n += 1;
                        }
                    }
                }
            }
        }
    }

    assert!(saw_min_diff, "err07 never reached diff == -2^28");
    assert!(saw_max_diff, "err07 never reached diff == 2^28-1");
    assert!(n > 10_000, "err07 ran only {n} cases");

    // Independently confirm the bound by brute force over many random steps.
    let mut rng = Rng::for_row("err07-bound");
    for _ in 0..300_000 {
        let step = rng.i32_any();
        for low in 0..8i32 {
            let mult = 2 * low + 1;
            let diff = mult.wrapping_mul(step) / 8;
            assert!(
                diff != i32::MIN,
                "diff == INT_MIN reachable with mult={mult}, step={step}"
            );
        }
        check(
            "err07",
            Args::new(8 | rng.range_i32(0, 7), step, rng.i32_any(), rng.i32_any(), rng.i32_any(), 0),
        );
    }
}

// ---------------------------------------------------------------------------
// Rows 8-10: overflow in the accumulation chain.
// ---------------------------------------------------------------------------

/// ERRORS.md row 8 - `pred + diff` overflows.
#[test]
fn err08_pred_plus_diff_overflow() {
    let mut rng = Rng::for_row("err08");
    let mut n = 0usize;
    for pred in [i32::MAX, i32::MAX - 1, i32::MAX - 7, i32::MIN, i32::MIN + 1] {
        for step in [
            1i32,
            8,
            9,
            255,
            i32::MAX,
            i32::MIN,
            -1,
            -8,
            -255,
            0x1000_0000,
        ] {
            for uni in 0..=15i32 {
                for lsbit in [0i32, 1, 4, 2, -1] {
                    for tgt in EXTREMES {
                        for tgt2 in EXTREMES {
                            check("err08", Args::new(uni, step, pred, tgt, tgt2, lsbit));
                            n += 1;
                        }
                    }
                }
            }
        }
    }
    for _ in 0..100_000 {
        let pred = rng.pick(&[i32::MAX, i32::MIN, i32::MAX - 1, i32::MIN + 1]);
        check(
            "err08",
            Args::new(
                rng.i32_any(),
                rng.i32_any(),
                pred,
                rng.i32_any(),
                rng.i32_any(),
                rng.i32_any(),
            ),
        );
        n += 1;
    }
    assert!(n > 100_000, "err08 ran only {n} cases");
}

/// ERRORS.md row 9 - `tgt - p` and `tgt2 - p` overflow.
#[test]
fn err09_target_minus_prediction_overflow() {
    let mut n = 0usize;
    // tgt = INT_MAX with p = INT_MIN (and vice versa) maximizes the subtraction.
    for tgt in [i32::MAX, i32::MAX - 1, i32::MIN, i32::MIN + 1] {
        for tgt2 in EXTREMES {
            for pred in [i32::MIN, i32::MIN + 1, i32::MAX, i32::MAX - 1, 0] {
                for step in [0i32, 1, 8, 255, -1, -255, i32::MAX, i32::MIN] {
                    for uni in 0..=15i32 {
                        for lsbit in [0i32, 4, 1, 2, 8, -1, -4] {
                            check("err09", Args::new(uni, step, pred, tgt, tgt2, lsbit));
                            n += 1;
                        }
                    }
                }
            }
        }
    }
    assert_eq!(n, 4 * 7 * 5 * 8 * 16 * 7);
    assert!(n > 100_000, "err09 ran only {n} cases");
}

/// ERRORS.md row 10 - `d += d3 >> 5` overflows, which can flip a "distortion"
/// negative and invert the line-57/59 comparisons.
#[test]
fn err10_distortion_accumulate_overflow() {
    let mut rng = Rng::for_row("err10");
    let mut n = 0usize;
    // Make |tgt - p| near INT_MAX and |tgt2 - p| huge so d + (d3>>5) overflows.
    for tgt in [i32::MAX, i32::MIN, i32::MAX - 1, i32::MIN + 1] {
        for tgt2 in [i32::MIN, i32::MAX, i32::MIN + 1, i32::MAX - 1] {
            for pred in [0i32, 1, -1, i32::MAX, i32::MIN, 1 << 30, -(1 << 30)] {
                for step in [0i32, 1, 8, 64, 1 << 20, i32::MAX, i32::MIN, -64] {
                    for uni in 0..=15i32 {
                        check("err10", Args::new(uni, step, pred, tgt, tgt2, 0));
                        check("err10", Args::new(uni, step, pred, tgt, tgt2, 4));
                        n += 2;
                    }
                }
            }
        }
    }
    // Randomized: force both targets to opposite extremes of pred.
    for _ in 0..200_000 {
        let pred = rng.range_i32(-(1 << 30), 1 << 30);
        let tgt = if rng.bool() { i32::MAX } else { i32::MIN };
        let tgt2 = if rng.bool() { i32::MIN } else { i32::MAX };
        check(
            "err10",
            Args::new(rng.i32_any(), rng.i32_any(), pred, tgt, tgt2, rng.i32_any()),
        );
        n += 1;
    }
    assert!(n > 200_000, "err10 ran only {n} cases");
}

// ---------------------------------------------------------------------------
// Rows 11-13: implementation-defined right shifts of negative values.
// ---------------------------------------------------------------------------

/// ERRORS.md row 11 - `d ^ (d >> 31)` branchless abs, incl. the `INT_MIN` case
/// where it yields `INT_MAX` rather than a true absolute value.
#[test]
fn err11_shift_right_31_on_negative() {
    let mut n = 0usize;
    // Drive tgt - p through INT_MIN exactly: pred = 0, p = diff, tgt = INT_MIN.
    for uni in 0..=15i32 {
        for step in [0i32, 1, 8, 9, 255, 1 << 20, i32::MAX, i32::MIN, -8, -255] {
            for pred in [0i32, 1, -1, i32::MAX, i32::MIN, 1 << 30] {
                for tgt in [i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX] {
                    for tgt2 in [i32::MIN, -1, 0, 1, i32::MAX] {
                        for lsbit in [0i32, 4] {
                            check("err11", Args::new(uni, step, pred, tgt, tgt2, lsbit));
                            n += 1;
                        }
                    }
                }
            }
        }
    }
    assert!(n > 50_000, "err11 ran only {n} cases");

    // Pin the INT_MIN -> INT_MAX abs behaviour: with step = 0 all three
    // candidates are pred, so d0 == d1 == d2 and uni is returned unchanged even
    // though the intermediate distortion overflowed.
    let got = check("err11", Args::new(5, 0, 0, i32::MIN, i32::MIN, 0));
    assert_eq!(got, 5);
}

/// ERRORS.md row 12 - `d3 >> 5` shift semantics (arithmetic, toward -inf).
#[test]
fn err12_shift_right_5_semantics() {
    let mut rng = Rng::for_row("err12");
    let mut n = 0usize;
    // Sweep tgt2 across sign changes and small magnitudes so the low 5 bits
    // discarded by `>> 5` matter, plus the values whose abs is INT_MAX.
    for delta in -80i32..=80 {
        for pred in [0i32, 1000, -1000, i32::MAX, i32::MIN] {
            for step in [0i32, 8, 64, 255, -64] {
                for uni in 0..=15i32 {
                    let tgt2 = pred.wrapping_add(delta);
                    check("err12", Args::new(uni, step, pred, 0, tgt2, 0));
                    n += 1;
                }
            }
        }
    }
    for _ in 0..100_000 {
        let a = Args::new(
            rng.range_i32(0, 15),
            rng.range_i32(-64, 64),
            rng.range_i32(-64, 64),
            rng.range_i32(-64, 64),
            rng.range_i32(-64, 64),
            rng.range_i32(-8, 8),
        );
        check("err12", a);
        n += 1;
    }
    assert!(n > 100_000, "err12 ran only {n} cases");
}

/// ERRORS.md row 13 - `(uni >> 1) & (uni >> 2) & 1` in the `lsbit == 4` dither,
/// with negative `uni`/`uni1`/`uni2` (arithmetic shift; `-1 >> 1 == -1`).
#[test]
fn err13_dither_shift_on_negative_uni() {
    let mut n = 0usize;
    // Exhaustive over a signed window around zero, plus the extremes, all with
    // lsbit == 4 so the dither branch always runs.
    for uni in -64..=64i32 {
        for step in [0i32, 1, 8, 9, 255, -255, i32::MAX, i32::MIN] {
            for pred in [0i32, 100, -100, i32::MAX, i32::MIN] {
                for tgt in [0i32, 137, -137, i32::MAX, i32::MIN] {
                    for tgt2 in [0i32, -1, i32::MAX, i32::MIN] {
                        check("err13", Args::new(uni, step, pred, tgt, tgt2, 4));
                        n += 1;
                    }
                }
            }
        }
    }
    for uni in [
        i32::MIN,
        i32::MIN + 1,
        i32::MIN + 7,
        i32::MIN + 8,
        -1,
        -2,
        -7,
        -8,
        -9,
        i32::MAX,
        i32::MAX - 1,
    ] {
        for step in EXTREMES {
            for pred in EXTREMES {
                for tgt in EXTREMES {
                    check("err13", Args::new(uni, step, pred, tgt, tgt, 4));
                    n += 1;
                }
            }
        }
    }
    assert!(n > 50_000, "err13 ran only {n} cases");

    // -1 stays -1 under the dither: bit0 cleared to -2, then OR'd back to -1
    // because (-2>>1)&(-2>>2)&1 == 1. With step = 0 the result is returned as is.
    let got = check("err13", Args::new(-1, 0, 0, 0, 0, 4));
    assert_eq!(got, -1, "dither on uni = -1 with step = 0 returns -1");
}

// ---------------------------------------------------------------------------
// Rows 14-16: degenerate and out-of-range `step` ("length"-like parameter).
// ---------------------------------------------------------------------------

/// ERRORS.md row 14 - `step == 0`: all diffs collapse to 0, no candidate wins.
#[test]
fn err14_step_zero() {
    let mut rng = Rng::for_row("err14");
    let mut n = 0usize;
    for uni in -32..=32i32 {
        for lsbit in -8..=8i32 {
            for pred in EXTREMES {
                for tgt in EXTREMES {
                    for tgt2 in EXTREMES {
                        check("err14", Args::new(uni, 0, pred, tgt, tgt2, lsbit));
                        n += 1;
                    }
                }
            }
        }
    }
    for _ in 0..100_000 {
        check(
            "err14",
            Args::new(
                rng.i32_any(),
                0,
                rng.i32_any(),
                rng.i32_any(),
                rng.i32_any(),
                rng.i32_any(),
            ),
        );
        n += 1;
    }
    assert!(n > 100_000, "err14 ran only {n} cases");
    // With step == 0 and lsbit == 0 the input uni must come back untouched.
    for uni in [-1i32, 0, 1, 7, 8, 15, i32::MIN, i32::MAX] {
        let got = check("err14", Args::new(uni, 0, 12345, -999, 42, 0));
        assert_eq!(got, uni, "step=0,lsbit=0 must return uni unchanged");
    }
}

/// ERRORS.md row 15 - negative `step` (a "length" a real API would reject).
#[test]
fn err15_step_negative() {
    let mut rng = Rng::for_row("err15");
    let mut n = 0usize;
    for step in [-1i32, -2, -7, -8, -9, -255, -1000, -0x1000_0000, i32::MIN, i32::MIN + 1] {
        for uni in -16..=16i32 {
            for lsbit in [0i32, 1, 2, 4, 8, -1, -4] {
                for pred in [0i32, 1234, -1234, i32::MAX, i32::MIN] {
                    for tgt in [0i32, 4321, -4321, i32::MAX, i32::MIN] {
                        check("err15", Args::new(uni, step, pred, tgt, 0, lsbit));
                        n += 1;
                    }
                }
            }
        }
    }
    for _ in 0..200_000 {
        let step = -(rng.range_i32(1, i32::MAX));
        check(
            "err15",
            Args::new(
                rng.i32_any(),
                step,
                rng.i32_any(),
                rng.i32_any(),
                rng.i32_any(),
                rng.i32_any(),
            ),
        );
        n += 1;
    }
    assert!(n > 200_000, "err15 ran only {n} cases");
}

/// ERRORS.md row 16 - oversized `step`: overflow plus truncating division of a
/// negative product (C truncates toward zero, not floor).
#[test]
fn err16_step_oversized() {
    let mut rng = Rng::for_row("err16");
    let mut n = 0usize;
    for step in [
        i32::MAX,
        i32::MAX - 1,
        0x7FFF_FFF8,
        0x7FFF_FFF9,
        0x4000_0000,
        0x2AAA_AAAB,
        1 << 28,
    ] {
        for uni in -16..=16i32 {
            for lsbit in [0i32, 1, 2, 4, 8, -1, -4] {
                for pred in EXTREMES {
                    for tgt in EXTREMES {
                        check("err16", Args::new(uni, step, pred, tgt, tgt, lsbit));
                        n += 1;
                    }
                }
            }
        }
    }
    for _ in 0..200_000 {
        let step = rng.range_i32(i32::MAX / 15, i32::MAX);
        check(
            "err16",
            Args::new(
                rng.i32_any(),
                step,
                rng.i32_any(),
                rng.i32_any(),
                rng.i32_any(),
                rng.i32_any(),
            ),
        );
        n += 1;
    }
    assert!(n > 200_000, "err16 ran only {n} cases");
}

// ---------------------------------------------------------------------------
// Row 17: the full "one step past the valid range" cross-product.
// ---------------------------------------------------------------------------

/// ERRORS.md row 17 - all six parameters simultaneously drawn from the signed
/// extremes: 7^6 = 117 649 tuples, every combination checked.
#[test]
fn err17_all_params_extremes_cross_product() {
    let mut n = 0usize;
    for uni in EXTREMES {
        for step in EXTREMES {
            for pred in EXTREMES {
                for tgt in EXTREMES {
                    for tgt2 in EXTREMES {
                        for lsbit in EXTREMES {
                            check("err17", Args::new(uni, step, pred, tgt, tgt2, lsbit));
                            n += 1;
                        }
                    }
                }
            }
        }
    }
    assert_eq!(n, 7usize.pow(6));
}

// ---------------------------------------------------------------------------
// Rows 18-19: surfaces proven not to exist.
// ---------------------------------------------------------------------------

/// ERRORS.md row 18 - there is no pointer or length parameter to make invalid.
/// Proven from the ABI rather than assumed: the exported symbol takes six
/// by-value `int`s (see `c_src/include/lib.h`). The nearest reachable analogue
/// is `0` / `INT_MIN` / `INT_MAX` in every slot, which is asserted here.
#[test]
fn err18_no_pointer_or_length_parameter() {
    let header = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("c_src/include/lib.h"),
    )
    .expect("read c_src/include/lib.h");
    assert!(
        !header.contains('*'),
        "header declares a pointer parameter; ERRORS.md row 18 must be revised: {header}"
    );
    assert!(
        header.contains("int encode_quant(int uni, int step, int pred, int tgt, int tgt2, int lsbit)"),
        "unexpected public ABI: {header}"
    );
    // All-zero and all-extreme argument tuples still agree.
    for v in [0i32, i32::MIN, i32::MAX] {
        check("err18", Args::new(v, v, v, v, v, v));
    }
}

/// ERRORS.md row 19 - the `/ 8` divisor is a literal, so no divide-by-zero and
/// no `INT_MIN / -1` trap is reachable. Demonstrated by driving `step` (the only
/// value feeding the dividend) to `INT_MIN` without either library crashing.
#[test]
fn err19_division_cannot_trap() {
    let source = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("c_src/src/lib.c"),
    )
    .expect("read c_src/src/lib.c");
    let divisions: Vec<&str> = source.matches("/ 8").collect();
    assert_eq!(
        divisions.len(),
        3,
        "expected exactly 3 `/ 8` divisions by a literal in the C source"
    );
    assert!(
        !source.contains("% ") && !source.contains("/ step") && !source.contains("/ pred"),
        "C source divides by a variable; ERRORS.md row 19 must be revised"
    );
    for uni in 0..=15i32 {
        for step in [i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX] {
            for lsbit in [0i32, 4, 1, 2] {
                check("err19", Args::new(uni, step, i32::MIN, i32::MAX, 0, lsbit));
                check("err19", Args::new(uni, step, i32::MAX, i32::MIN, -1, lsbit));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 20: argument widening / calling convention.
// ---------------------------------------------------------------------------

/// ERRORS.md row 20 - values with the high bits set, passed as `c_int`. If the
/// Rust wrapper used the wrong width or signedness these would diverge.
#[test]
fn err20_argument_widening_and_calling_convention() {
    let raw: [u32; 12] = [
        0xFFFF_FFFF,
        0x8000_0000,
        0x8000_0001,
        0x7FFF_FFFF,
        0xFFFF_FFF8,
        0x0000_0008,
        0xDEAD_BEEF,
        0xCAFE_BABE,
        0xFFFF_0000,
        0x0000_FFFF,
        0x5555_5555,
        0xAAAA_AAAA,
    ];
    let mut n = 0usize;
    for &a in &raw {
        for &b in &raw {
            for &c in &raw {
                let (x, y, z) = (a as i32, b as i32, c as i32);
                // Rotate the same nasty bit patterns through every slot.
                check("err20", Args::new(x, y, z, x, y, z));
                check("err20", Args::new(z, x, y, z, x, y));
                check("err20", Args::new(y, z, x, y, z, x));
                n += 3;
            }
        }
    }
    assert_eq!(n, raw.len().pow(3) * 3);

    // Verify the return value's full 32 bits survive: pick an input whose result
    // has the sign bit set (uni is returned verbatim when step == 0).
    let got = check("err20", Args::new(0xDEAD_BEEFu32 as i32, 0, 0, 0, 0, 0));
    assert_eq!(got, 0xDEAD_BEEFu32 as i32, "full 32-bit return must round-trip");
}
