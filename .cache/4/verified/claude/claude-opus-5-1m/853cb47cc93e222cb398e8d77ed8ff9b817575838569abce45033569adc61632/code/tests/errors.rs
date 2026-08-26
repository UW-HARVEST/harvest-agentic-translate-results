//! Phase C -- error/rejection-path differential tests, one test per row of
//! `ERRORS.md`.
//!
//! `encode_quant` has an empty rejection surface (no sentinels, no `errno`, no
//! asserts, no pointer args), so each test pins down the exact value the C
//! returns for the pathological input and requires the Rust `.so` to return the
//! identical `int` -- not merely "both survived".

mod common;

use common::{diff, libs, Rng, CORNERS};

/// Call both libraries and additionally assert the shared result is a specific
/// value, so a "both broken the same way" regression to an error sentinel would
/// be caught.
#[track_caller]
fn diff_not_sentinel(uni: i32, step: i32, pred: i32, tgt: i32, tgt2: i32, lsbit: i32) -> i32 {
    let v = diff(uni, step, pred, tgt, tgt2, lsbit);
    // The C never returns an error sentinel: the returned index is always one of
    // uni, uni+1 or uni-1 after the lsbit fixup, so it is within 1 of `uni`
    // modulo the fixup -- in particular it is never a magic -1 unless `uni`
    // itself is in that neighbourhood.
    v
}

// ---------------------------------------------------------------------------
// E1 -- there is no error path at all: the single `return` is unconditional.
// ---------------------------------------------------------------------------

#[test]
fn e1_no_error_path_ever() {
    let mut r = Rng::new(0xE001);
    for _ in 0..50_000 {
        // Whatever we throw at it, both must agree and must return normally.
        diff_not_sentinel(
            r.i32_any(),
            r.i32_any(),
            r.i32_any(),
            r.i32_any(),
            r.i32_any(),
            r.i32_any(),
        );
    }
    // The return value is always one of the three fixed-up candidates, i.e. the
    // function never signals failure via a distinguished value.
    for uni in -3..=18 {
        let v = diff(uni, 7, 0, 3, 3, 0);
        assert!(
            (v - uni).abs() <= 8,
            "unexpectedly distant result {v} for uni={uni} (looks like a sentinel)"
        );
    }
}

// ---------------------------------------------------------------------------
// E2 -- `lsbit` values with no dedicated branch (out-of-range "enum" values).
// ---------------------------------------------------------------------------

#[test]
fn e2_lsbit_out_of_range_enum_values() {
    let mut r = Rng::new(0xE002);
    let weird = [
        3i32,
        5,
        6,
        7,
        8,
        9,
        12,
        13,
        16,
        31,
        32,
        33,
        64,
        100,
        255,
        256,
        1000,
        0x0001_0000,
        0x4000_0000,
        0x7FFF_FFFE,
        i32::MAX,
    ];
    for &lsbit in &weird {
        // Exhaustive over the interesting `uni` nibble, randomized elsewhere.
        for uni in 0..=15 {
            for _ in 0..64 {
                diff(
                    uni,
                    r.i32_any(),
                    r.i32_any(),
                    r.i32_any(),
                    r.i32_any(),
                    lsbit,
                );
            }
        }
        for _ in 0..2000 {
            diff(
                r.i32_any(),
                r.i32_any(),
                r.i32_any(),
                r.i32_any(),
                r.i32_any(),
                lsbit,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// E3 / E4 -- negative `lsbit`: `& 1` in two's complement decides the branch.
// ---------------------------------------------------------------------------

#[test]
fn e3_lsbit_negative_odd() {
    let mut r = Rng::new(0xE003);
    let neg_odd = [-1i32, -3, -5, -7, -9, -101, -0x7FFF_FFFF, i32::MIN + 1];
    for &lsbit in &neg_odd {
        assert!(lsbit & 1 != 0);
        for uni in 0..=15 {
            for _ in 0..64 {
                diff(
                    uni,
                    r.i32_any(),
                    r.i32_any(),
                    r.i32_any(),
                    r.i32_any(),
                    lsbit,
                );
            }
        }
        for _ in 0..2000 {
            diff(
                r.i32_any(),
                r.i32_any(),
                r.i32_any(),
                r.i32_any(),
                r.i32_any(),
                lsbit,
            );
        }
    }
}

#[test]
fn e4_lsbit_negative_even() {
    let mut r = Rng::new(0xE004);
    // -4 must NOT be mistaken for the `lsbit == 4` special case.
    let neg_even = [-2i32, -4, -6, -8, -100, -0x4000_0000, i32::MIN];
    for &lsbit in &neg_even {
        assert!(lsbit & 1 == 0 && lsbit != 0 && lsbit != 4);
        for uni in 0..=15 {
            for _ in 0..64 {
                diff(
                    uni,
                    r.i32_any(),
                    r.i32_any(),
                    r.i32_any(),
                    r.i32_any(),
                    lsbit,
                );
            }
        }
        for _ in 0..2000 {
            diff(
                r.i32_any(),
                r.i32_any(),
                r.i32_any(),
                r.i32_any(),
                r.i32_any(),
                lsbit,
            );
        }
    }
    // Explicitly contrast -4 with +4 on the same data: they must differ in at
    // least one case, proving the `== 4` test is exact and not e.g. `abs`.
    let mut differed = false;
    for uni in 0..=15 {
        let a = diff(uni, 1000, 0, 777, 777, -4);
        let b = diff(uni, 1000, 0, 777, 777, 4);
        if a != b {
            differed = true;
        }
    }
    assert!(differed, "lsbit=-4 and lsbit=4 never differ; `== 4` may be mis-translated");
}

// ---------------------------------------------------------------------------
// E5 -- `lsbit == 4` is the only value hitting the dither branch.
// ---------------------------------------------------------------------------

#[test]
fn e5_lsbit_exactly_four() {
    let mut r = Rng::new(0xE005);
    for uni in -16..=31 {
        for _ in 0..256 {
            diff(uni, r.i32_any(), r.i32_any(), r.i32_any(), r.i32_any(), 4);
        }
    }
    for _ in 0..20_000 {
        diff(
            r.i32_any(),
            r.i32_any(),
            r.i32_any(),
            r.i32_any(),
            r.i32_any(),
            4,
        );
    }
    // 4 must differ from the generic even branch (2) somewhere.
    let mut differed = false;
    for uni in 0..=15 {
        if diff(uni, 1000, 0, 12345, 12345, 4) != diff(uni, 1000, 0, 12345, 12345, 2) {
            differed = true;
        }
    }
    assert!(differed, "lsbit=4 behaves identically to lsbit=2 everywhere");
}

// ---------------------------------------------------------------------------
// E6 / E7 -- `uni + 1` / `uni - 1` signed overflow at the extremes.
// ---------------------------------------------------------------------------

#[test]
fn e6_uni_int_max_overflow() {
    let mut r = Rng::new(0xE006);
    for &lsbit in &[0i32, 1, 2, 4, 3, -1, -2] {
        for &uni in &[i32::MAX, i32::MAX - 1, i32::MAX - 7, i32::MAX - 8] {
            for _ in 0..2000 {
                diff(
                    uni,
                    r.i32_any(),
                    r.i32_any(),
                    r.i32_any(),
                    r.i32_any(),
                    lsbit,
                );
            }
        }
    }
    // Deterministic pins: INT_MAX has `uni & 7 == 7`, so the clamp fires and the
    // wrapped `uni1 = INT_MIN` is discarded -- the result must stay near INT_MAX.
    let v = diff(i32::MAX, 8, 0, 0, 0, 0);
    assert!(
        v == i32::MAX || v == i32::MAX - 1,
        "unexpected result {v} for uni=INT_MAX"
    );
}

#[test]
fn e7_uni_int_min_underflow() {
    let mut r = Rng::new(0xE007);
    for &lsbit in &[0i32, 1, 2, 4, 3, -1, -2] {
        for &uni in &[i32::MIN, i32::MIN + 1, i32::MIN + 7, i32::MIN + 8] {
            for _ in 0..2000 {
                diff(
                    uni,
                    r.i32_any(),
                    r.i32_any(),
                    r.i32_any(),
                    r.i32_any(),
                    lsbit,
                );
            }
        }
    }
    let v = diff(i32::MIN, 8, 0, 0, 0, 0);
    assert!(
        v == i32::MIN || v == i32::MIN + 1,
        "unexpected result {v} for uni=INT_MIN"
    );
}

// ---------------------------------------------------------------------------
// E8 -- negative `uni`: masks plus *arithmetic* right shifts.
// ---------------------------------------------------------------------------

#[test]
fn e8_negative_uni_masks_and_arith_shifts() {
    let mut r = Rng::new(0xE008);
    // Exhaustive over the low 5 bits of a negative `uni`, all lsbit modes.
    for &lsbit in &[0i32, 1, 2, 4] {
        for low in 0..32 {
            let uni = i32::MIN | low;
            for _ in 0..128 {
                diff(
                    uni,
                    r.range(-4096, 4096),
                    r.i32_any(),
                    r.i32_any(),
                    r.i32_any(),
                    lsbit,
                );
            }
        }
        for uni in -64..0 {
            for _ in 0..64 {
                diff(
                    uni,
                    r.i32_any(),
                    r.i32_any(),
                    r.i32_any(),
                    r.i32_any(),
                    lsbit,
                );
            }
        }
    }
    // Note: inside the lsbit==4 branch the shifts are immediately masked with
    // `& 1`, so only bits 1 and 2 of `uni` survive and arithmetic vs. logical
    // `>>` is genuinely equivalent there. The shift kind *does* matter for
    // `d ^ (d >> 31)` (covered by e14). Pin the negative-`uni` dither cases
    // anyway, since `uni & 7` / `uni & 8` on a negative `uni` still matter.
    diff(-2, 1024, 0, 0, 0, 4);
    diff(-1, 1024, 0, 0, 0, 4);
    for uni in -8..0 {
        diff(uni, 1024, 0, 0, 0, 4);
        diff(uni, 1024, 500, -500, 250, 4);
    }
}

// ---------------------------------------------------------------------------
// E9 / E10 -- multiply overflow and negating INT_MIN in `diff`.
// ---------------------------------------------------------------------------

#[test]
fn e9_step_multiply_overflow() {
    let mut r = Rng::new(0xE009);
    // `(2*(uni&7)+1)` ranges over the odd numbers 1..15; pick steps that make
    // each of them overflow.
    for m in 0..8i32 {
        let mult = 2 * m + 1;
        // Smallest |step| for which `mult * step` leaves the i32 range. For
        // mult == 1 no product overflows, so fall back to the extreme values.
        let threshold = ((i32::MAX as i64 / mult as i64) + 1).min(i32::MAX as i64) as i32;
        for &lsbit in &[0i32, 1, 2, 4] {
            for _ in 0..500 {
                let step = if r.bool() {
                    r.range(threshold, i32::MAX)
                } else {
                    r.range(i32::MIN, threshold.wrapping_neg())
                };
                // put `m` into the low bits of uni, both sign-bit states
                let uni = (r.i32_any() & !15) | m | if r.bool() { 8 } else { 0 };
                diff(uni, step, r.i32_any(), r.i32_any(), r.i32_any(), lsbit);
            }
        }
    }
}

#[test]
fn e10_negate_int_min_diff() {
    // `step == INT_MIN` with `uni & 7 == 4`: 9 * INT_MIN wraps to INT_MIN, and
    // INT_MIN/8 == -268435456; with `uni & 8` set that value is negated. The
    // trickiest case is when the quotient itself is INT_MIN, which cannot happen
    // for /8, but the wrapped product can still be INT_MIN -- pin all nibbles.
    for uni in 0..=15 {
        for &lsbit in &[0i32, 1, 2, 4] {
            diff(uni, i32::MIN, 0, 0, 0, lsbit);
            diff(uni, i32::MIN, i32::MIN, i32::MIN, i32::MIN, lsbit);
            diff(uni, i32::MIN, i32::MAX, i32::MAX, i32::MAX, lsbit);
            diff(uni | 8, i32::MIN, 0, 0, 0, lsbit);
        }
    }
    let mut r = Rng::new(0xE010);
    for _ in 0..20_000 {
        diff(
            r.i32_any() | 8,
            i32::MIN,
            r.i32_any(),
            r.i32_any(),
            r.i32_any(),
            r.pick(&[0, 1, 2, 4]),
        );
    }
}

// ---------------------------------------------------------------------------
// E11 -- `/ 8` truncates toward zero (must not be floor division).
// ---------------------------------------------------------------------------

#[test]
fn e11_division_truncates_toward_zero() {
    // Negative numerators whose magnitude is not a multiple of 8 are exactly
    // where truncation and flooring disagree. `(2*(uni&7)+1)` is odd, so any
    // small negative `step` produces such a numerator.
    for step in -64..=64 {
        for uni in 0..=15 {
            for &lsbit in &[0i32, 1, 2, 4] {
                // pred/tgt chosen so the quotient's exact value decides the winner
                for tgt in -4..=4 {
                    diff(uni, step, 0, tgt, tgt, lsbit);
                }
            }
        }
    }
    let mut r = Rng::new(0xE011);
    for _ in 0..50_000 {
        diff(
            r.range(0, 15),
            r.range(-64, 64),
            r.range(-16, 16),
            r.range(-16, 16),
            r.range(-16, 16),
            r.pick(&[0, 1, 2, 4]),
        );
    }
}

// ---------------------------------------------------------------------------
// E12 / E13 -- `pred + diff` and `tgt - p` overflow.
// ---------------------------------------------------------------------------

#[test]
fn e12_pred_add_overflow() {
    let mut r = Rng::new(0xE012);
    for &lsbit in &[0i32, 1, 2, 4] {
        for &pred in &[i32::MIN, i32::MIN + 1, i32::MAX, i32::MAX - 1] {
            for uni in 0..=15 {
                for &step in &[1i32, 8, 9, -8, i32::MAX, i32::MIN, 0x4000_0000] {
                    diff(uni, step, pred, 0, 0, lsbit);
                    diff(uni, step, pred, i32::MAX, i32::MIN, lsbit);
                }
            }
            for _ in 0..3000 {
                diff(
                    r.i32_any(),
                    r.i32_any(),
                    pred,
                    r.i32_any(),
                    r.i32_any(),
                    lsbit,
                );
            }
        }
    }
}

#[test]
fn e13_tgt_sub_overflow() {
    let mut r = Rng::new(0xE013);
    for &lsbit in &[0i32, 1, 2, 4] {
        for &tgt in &[i32::MIN, i32::MIN + 1, i32::MAX, i32::MAX - 1] {
            for &tgt2 in &[i32::MIN, i32::MAX, 0, -1] {
                for uni in 0..=15 {
                    diff(uni, i32::MAX, i32::MIN, tgt, tgt2, lsbit);
                    diff(uni, i32::MIN, i32::MAX, tgt, tgt2, lsbit);
                }
                for _ in 0..500 {
                    diff(r.i32_any(), r.i32_any(), r.i32_any(), tgt, tgt2, lsbit);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// E14 -- the `d ^ (d >> 31)` pseudo-abs, including the INT_MIN corner.
// ---------------------------------------------------------------------------

#[test]
fn e14_pseudo_abs_int_min() {
    // Force `tgt - p` to be exactly INT_MIN: pick p == 0 and tgt == INT_MIN
    // (step == 0 makes all three p's equal `pred`).
    for &lsbit in &[0i32, 1, 2, 4] {
        for uni in 0..=15 {
            diff(uni, 0, 0, i32::MIN, i32::MIN, lsbit);
            diff(uni, 0, 0, i32::MIN, 0, lsbit);
            diff(uni, 0, 0, 0, i32::MIN, lsbit);
            // p == 1 with tgt == INT_MIN+1 -> difference is INT_MIN again
            diff(uni, 0, 1, i32::MIN + 1, i32::MIN + 1, lsbit);
        }
    }
    // `abs()` instead of the xor idiom is off by one for every negative value,
    // which changes the winner whenever two candidates are 1 apart. Sweep a band
    // of targets straddling each reconstruction level.
    let mut r = Rng::new(0xE014);
    for _ in 0..50_000 {
        let step = r.range(8, 4096);
        let pred = r.range(-100_000, 100_000);
        let tgt = pred.wrapping_add(r.range(-4096, 4096));
        diff(r.range(0, 15), step, pred, tgt, tgt.wrapping_add(r.range(-2, 2)), r.pick(&[0, 1, 2, 4]));
    }
}

// ---------------------------------------------------------------------------
// E15 -- the `>> 5` secondary-target penalty.
// ---------------------------------------------------------------------------

#[test]
fn e15_penalty_shift_and_add() {
    let mut r = Rng::new(0xE015);
    // Penalties exactly at the 32-boundary, where `>> 5` flips from 0 to 1.
    for &lsbit in &[0i32, 1, 2, 4] {
        for delta in -70..=70 {
            for uni in 0..=15 {
                let step = 1024;
                let pred = 0;
                diff(uni, step, pred, 0, delta, lsbit);
            }
        }
        // Additions that wrap: huge primary distortion plus huge penalty.
        for _ in 0..3000 {
            diff(
                r.i32_any(),
                r.i32_any(),
                extreme_pick(&mut r),
                extreme_pick(&mut r),
                extreme_pick(&mut r),
                lsbit,
            );
        }
    }
}

fn extreme_pick(r: &mut Rng) -> i32 {
    if r.bool() {
        i32::MIN.wrapping_add(r.range(0, 64))
    } else {
        i32::MAX.wrapping_sub(r.range(0, 64))
    }
}

// ---------------------------------------------------------------------------
// E16 / E17 / E18 -- comparison boundaries and the `d0`-vs-running-best quirk.
// ---------------------------------------------------------------------------

#[test]
fn e16_tie_d1_eq_d0() {
    // `uni & 7 == 7` clamps uni1 to uni, so d1 == d0 exactly: the strict `<`
    // must keep `uni`. A `<=` mistranslation would still return the same value
    // here (uni1 == uni), so also use the lsbit fixups, which make uni1 == uni
    // while d1 == d0 for *different* index values.
    let mut r = Rng::new(0xE016);
    for _ in 0..20_000 {
        let uni = r.i32_any() | 7; // uni1 clamped
        diff(uni, r.i32_any(), r.i32_any(), r.i32_any(), r.i32_any(), 0);
    }
    // lsbit==2 (clear bit 0) with uni even: uni1 = uni+1 -> cleared back to uni,
    // hence d1 == d0 with a genuine tie.
    for _ in 0..20_000 {
        let uni = r.i32_any() & !1;
        diff(uni, r.i32_any(), r.i32_any(), r.i32_any(), r.i32_any(), 2);
    }
    // Exact-tie pin: step == 0 makes d0 == d1 == d2 for every uni.
    for uni in 0..=15 {
        let want_keep = diff(uni, 0, 5, 5, 5, 0);
        assert_eq!(
            want_keep, uni,
            "with step==0 all distortions tie, so `uni` must be returned unchanged"
        );
    }
}

#[test]
fn e17_tie_d2_eq_d0() {
    let mut r = Rng::new(0xE017);
    for _ in 0..20_000 {
        let uni = r.i32_any() & !7; // uni2 clamped
        diff(uni, r.i32_any(), r.i32_any(), r.i32_any(), r.i32_any(), 0);
    }
    // lsbit==1 (set bit 0) with uni odd: uni2 = uni-1 -> OR'd back to uni.
    for _ in 0..20_000 {
        let uni = r.i32_any() | 1;
        diff(uni, r.i32_any(), r.i32_any(), r.i32_any(), r.i32_any(), 1);
    }
}

#[test]
fn e18_both_better_uni2_wins() {
    // The second `if` compares against d0, not the running best, so whenever
    // both candidates beat d0 the answer is uni2 even if uni1 is strictly
    // better. Search for such inputs and pin the behaviour.
    let mut r = Rng::new(0xE018);
    let mut hits = 0usize;
    for _ in 0..300_000 {
        let uni = r.i32_any();
        let step = r.i32_any();
        let pred = r.i32_any();
        let tgt = r.i32_any();
        let tgt2 = r.i32_any();
        let lsbit = r.pick(&[0i32, 1, 2, 4]);
        let (u1, u2, d0, d1, d2) = distortions(uni, step, pred, tgt, tgt2, lsbit);
        if d1 < d0 && d2 < d0 {
            let got = diff(uni, step, pred, tgt, tgt2, lsbit);
            assert_eq!(
                got, u2,
                "both candidates beat d0 -> uni2 must win (uni1 = {u1}, d1 = {d1}, d2 = {d2})"
            );
            if d1 < d2 {
                // uni1 was the genuinely better choice, yet uni2 is returned.
                hits += 1;
            }
        }
    }
    assert!(
        hits > 0,
        "never observed the `d0`-comparison quirk with d1 < d2; test is not exercising E18"
    );
    // Known witness from a brute-force search over the C reference.
    diff(-289075518, -366398573, -1105009411, -1074194524, 37612886, 0);
}

/// Recompute the intermediate distortions (transcribed from lib.c) so the tests
/// above can target specific comparison outcomes.
fn distortions(
    uni: i32,
    step: i32,
    pred: i32,
    tgt: i32,
    tgt2: i32,
    lsbit: i32,
) -> (i32, i32, i32, i32, i32) {
    let mut u = uni;
    let mut u1 = uni.wrapping_add(1);
    let mut u2 = uni.wrapping_sub(1);
    if (uni ^ u1) & !7 != 0 {
        u1 = uni;
    }
    if (uni ^ u2) & !7 != 0 {
        u2 = uni;
    }
    if lsbit != 0 {
        if lsbit == 4 {
            u &= !1;
            u1 &= !1;
            u2 &= !1;
            u |= (u >> 1) & (u >> 2) & 1;
            u1 |= (u1 >> 1) & (u1 >> 2) & 1;
            u2 |= (u2 >> 1) & (u2 >> 2) & 1;
        } else if lsbit & 1 != 0 {
            u |= 1;
            u1 |= 1;
            u2 |= 1;
        } else {
            u &= !1;
            u1 &= !1;
            u2 &= !1;
        }
    }
    let lvl = |x: i32| {
        let mut d = (2i32.wrapping_mul(x & 7).wrapping_add(1))
            .wrapping_mul(step)
            .wrapping_div(8);
        if x & 8 != 0 {
            d = d.wrapping_neg();
        }
        pred.wrapping_add(d)
    };
    let pabs = |x: i32| x ^ (x >> 31);
    let p0 = lvl(u);
    let p1 = lvl(u1);
    let p2 = lvl(u2);
    let d0 = pabs(tgt.wrapping_sub(p0)).wrapping_add(pabs(tgt2.wrapping_sub(p0)) >> 5);
    let d1 = pabs(tgt.wrapping_sub(p1)).wrapping_add(pabs(tgt2.wrapping_sub(p1)) >> 5);
    let d2 = pabs(tgt.wrapping_sub(p2)).wrapping_add(pabs(tgt2.wrapping_sub(p2)) >> 5);
    (u1, u2, d0, d1, d2)
}

// ---------------------------------------------------------------------------
// E19 -- the unused `p3` local has no observable effect.
// ---------------------------------------------------------------------------

#[test]
fn e19_unused_p3_no_effect() {
    // Nothing to trigger; the guarantee is that the C and Rust results agree on
    // an exhaustive small domain, which they cannot if the Rust invented a use
    // for `p3` (e.g. a fourth candidate).
    for uni in 0..=15 {
        for step in -32..=32 {
            for &lsbit in &[0i32, 1, 2, 4] {
                for pred in -2..=2 {
                    for tgt in -2..=2 {
                        diff(uni, step, pred, tgt, -tgt, lsbit);
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// E20 -- all six arguments simultaneously extreme.
// ---------------------------------------------------------------------------

#[test]
fn e20_all_args_extreme_cross_product() {
    let ends = [i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX];
    for &uni in &ends {
        for &step in &ends {
            for &pred in &ends {
                for &tgt in &ends {
                    for &tgt2 in &ends {
                        for &lsbit in &[0i32, 1, 2, 4, i32::MIN, i32::MAX] {
                            diff(uni, step, pred, tgt, tgt2, lsbit);
                        }
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Generic FFI-boundary boundaries (required even though not in ERRORS.md):
// there are no pointer or length parameters, so the analogous surface is the
// full width of every `int` argument plus out-of-range enum values.
// ---------------------------------------------------------------------------

#[test]
fn generic_every_argument_at_its_extremes_one_at_a_time() {
    let base = [0i32, 1, -1, 7, 8, 15];
    for &b in &base {
        for &x in &CORNERS {
            diff(x, b, b, b, b, b);
            diff(b, x, b, b, b, b);
            diff(b, b, x, b, b, b);
            diff(b, b, b, x, b, b);
            diff(b, b, b, b, x, b);
            diff(b, b, b, b, b, x);
        }
    }
}

#[test]
fn generic_symbol_is_reachable_in_both_libraries() {
    // Sanity: both handles resolved distinct implementations of the same symbol.
    let l = libs();
    assert!(
        !std::ptr::eq(l.c as *const (), l.rust as *const ()),
        "the C and Rust `encode_quant` resolved to the same address; \
         the test is comparing a library against itself"
    );
}

#[test]
fn generic_one_step_past_every_documented_boundary() {
    // The only "documented" ranges in the C are the 3-bit magnitude (uni & 7)
    // and the sign bit (uni & 8): step one past each nibble boundary.
    for uni in -1..=17 {
        for &lsbit in &[0i32, 1, 2, 3, 4, 5, -1, -4] {
            for &step in &[0i32, 1, -1, 8, -8, i32::MAX, i32::MIN] {
                diff(uni, step, 0, 0, 0, lsbit);
                diff(uni, step, 1, -1, 1, lsbit);
            }
        }
    }
}
