//! Phase B -- valid-path differential tests, one test per row of `CONFIGS.md`.
//!
//! Both implementations are loaded from their `.so` and called through the
//! exported `encode_quant` symbol (see `tests/common/mod.rs`).

mod common;

use common::{diff, Rng, CORNERS, LSBIT_MODES};

/// Randomized samples per configuration row.
const ITERS: usize = 4000;

// ---------------------------------------------------------------------------
// Input-shape generators (axes B..H of CONFIGS.md)
// ---------------------------------------------------------------------------

/// `uni & 7 == 0` -> the `uni2` clamp fires (axis C_CLAMP).
fn uni_low0(r: &mut Rng) -> i32 {
    r.i32_any() & !7
}
/// `uni & 7 == 7` -> the `uni1` clamp fires (axis B_CLAMP).
fn uni_low7(r: &mut Rng) -> i32 {
    r.i32_any() | 7
}
/// `uni & 7` in `1..=6` -> neither clamp fires (B_FREE + C_FREE).
fn uni_mid(r: &mut Rng) -> i32 {
    (r.i32_any() & !7) | r.range(1, 6)
}
/// `step` shapes.
fn step_small_pos(r: &mut Rng) -> i32 {
    r.range(1, 1024)
}
fn step_small_neg(r: &mut Rng) -> i32 {
    r.range(-1024, -1)
}
/// `(2*(uni&7)+1)` is at most 15, so `step > i32::MAX/15` guarantees the
/// signed multiply on line 30/36/42 overflows for the top magnitudes.
fn step_ovf_pos(r: &mut Rng) -> i32 {
    r.range(i32::MAX / 15 + 1, i32::MAX)
}
fn step_ovf_neg(r: &mut Rng) -> i32 {
    r.range(i32::MIN, i32::MIN / 15 - 1)
}
/// Values within `mag` of either end of the `i32` range.
fn extreme(r: &mut Rng, mag: i32) -> i32 {
    if r.bool() {
        i32::MIN.wrapping_add(r.range(0, mag))
    } else {
        i32::MAX.wrapping_sub(r.range(0, mag))
    }
}

// ---------------------------------------------------------------------------
// Test-side transcription of lib.c, used ONLY to (a) target inputs at a given
// decision outcome and (b) count outcome coverage. It is additionally asserted
// to agree with the C `.so`, so a mistake here fails loudly instead of
// silently mis-counting.
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Outcome {
    Keep,
    Up,
    Down,
    Both,
    Tie1,
    Tie2,
    TieBoth,
}

fn fixup(uni: i32, lsbit: i32) -> (i32, i32, i32) {
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
    (u, u1, u2)
}

/// Reconstruction level for a (fixed-up) index.
fn level(u: i32, step: i32, pred: i32) -> i32 {
    let mut d = (2i32.wrapping_mul(u & 7).wrapping_add(1))
        .wrapping_mul(step)
        .wrapping_div(8);
    if u & 8 != 0 {
        d = d.wrapping_neg();
    }
    pred.wrapping_add(d)
}

fn pseudo_abs(x: i32) -> i32 {
    x ^ (x >> 31)
}

/// Returns `(result, outcome)` exactly as lib.c computes them.
fn model(uni: i32, step: i32, pred: i32, tgt: i32, tgt2: i32, lsbit: i32) -> (i32, Outcome) {
    let (u, u1, u2) = fixup(uni, lsbit);
    let p0 = level(u, step, pred);
    let p1 = level(u1, step, pred);
    let p2 = level(u2, step, pred);
    let d0 = pseudo_abs(tgt.wrapping_sub(p0))
        .wrapping_add(pseudo_abs(tgt2.wrapping_sub(p0)) >> 5);
    let d1 = pseudo_abs(tgt.wrapping_sub(p1))
        .wrapping_add(pseudo_abs(tgt2.wrapping_sub(p1)) >> 5);
    let d2 = pseudo_abs(tgt.wrapping_sub(p2))
        .wrapping_add(pseudo_abs(tgt2.wrapping_sub(p2)) >> 5);
    let outcome = if d1 < d0 && d2 < d0 {
        Outcome::Both
    } else if d1 < d0 {
        Outcome::Up
    } else if d2 < d0 {
        Outcome::Down
    } else if d1 == d0 && d2 == d0 {
        Outcome::TieBoth
    } else if d1 == d0 {
        Outcome::Tie1
    } else if d2 == d0 {
        Outcome::Tie2
    } else {
        Outcome::Keep
    };
    let mut res = u;
    if d1 < d0 {
        res = u1;
    }
    if d2 < d0 {
        res = u2;
    }
    (res, outcome)
}

/// Differential call that additionally cross-checks the transcription above.
#[track_caller]
fn diff_checked(uni: i32, step: i32, pred: i32, tgt: i32, tgt2: i32, lsbit: i32) -> Outcome {
    let got = diff(uni, step, pred, tgt, tgt2, lsbit);
    let (want, outcome) = model(uni, step, pred, tgt, tgt2, lsbit);
    assert_eq!(
        got, want,
        "test-side model disagrees with the libraries for \
         encode_quant({uni}, {step}, {pred}, {tgt}, {tgt2}, {lsbit})"
    );
    outcome
}

// ===========================================================================
// Row 1 -- A0, degenerate baseline
// ===========================================================================

#[test]
fn row01_a0_degenerate_zero_everything() {
    for uni in 0..=15 {
        diff(uni, 0, 0, 0, 0, 0);
    }
}

// ===========================================================================
// Rows 2-6 -- A0 with each clamp / sign-bit shape
// ===========================================================================

#[test]
fn row02_a0_no_clamp_small_positive_step() {
    let mut r = Rng::new(0x0002);
    for _ in 0..ITERS {
        let uni = uni_mid(&mut r);
        diff(
            uni,
            step_small_pos(&mut r),
            r.i32_any(),
            r.i32_any(),
            r.i32_any(),
            0,
        );
    }
}

#[test]
fn row03_a0_uni2_clamped() {
    let mut r = Rng::new(0x0003);
    for _ in 0..ITERS {
        let uni = uni_low0(&mut r);
        assert_eq!(uni & 7, 0);
        diff(uni, r.i32_any(), r.i32_any(), r.i32_any(), r.i32_any(), 0);
    }
}

#[test]
fn row04_a0_uni1_clamped() {
    let mut r = Rng::new(0x0004);
    for _ in 0..ITERS {
        let uni = uni_low7(&mut r);
        assert_eq!(uni & 7, 7);
        diff(uni, r.i32_any(), r.i32_any(), r.i32_any(), r.i32_any(), 0);
    }
}

#[test]
fn row05_a0_sign_bit_clear() {
    let mut r = Rng::new(0x0005);
    for _ in 0..ITERS {
        let uni = r.i32_any() & !8;
        diff(uni, r.i32_any(), r.i32_any(), r.i32_any(), r.i32_any(), 0);
    }
}

#[test]
fn row06_a0_sign_bit_set() {
    let mut r = Rng::new(0x0006);
    for _ in 0..ITERS {
        let uni = r.i32_any() | 8;
        diff(uni, r.i32_any(), r.i32_any(), r.i32_any(), r.i32_any(), 0);
    }
}

// ===========================================================================
// Rows 7-10 -- the `lsbit == 4` special branch
// ===========================================================================

#[test]
fn row07_a4_bits1and2_set_reor_fires() {
    let mut r = Rng::new(0x0007);
    for _ in 0..ITERS {
        let uni = r.i32_any() | 6;
        diff(uni, r.i32_any(), r.i32_any(), r.i32_any(), r.i32_any(), 4);
    }
}

#[test]
fn row08_a4_bits1and2_not_both_set() {
    let mut r = Rng::new(0x0008);
    for _ in 0..ITERS {
        let mut uni = r.i32_any();
        // clear at least one of bit 1 / bit 2
        if r.bool() {
            uni &= !2;
        } else {
            uni &= !4;
        }
        assert_ne!(uni & 6, 6);
        diff(uni, r.i32_any(), r.i32_any(), r.i32_any(), r.i32_any(), 4);
    }
}

#[test]
fn row09_a4_negative_uni_arithmetic_shift() {
    let mut r = Rng::new(0x0009);
    for _ in 0..ITERS {
        let uni = r.i32_any() | i32::MIN;
        assert!(uni < 0);
        diff(uni, r.i32_any(), r.i32_any(), r.i32_any(), r.i32_any(), 4);
    }
}

#[test]
fn row10_a4_clamp_interaction() {
    let mut r = Rng::new(0x000A);
    for _ in 0..ITERS {
        let uni = if r.bool() {
            uni_low0(&mut r)
        } else {
            uni_low7(&mut r)
        };
        diff(uni, r.i32_any(), r.i32_any(), r.i32_any(), r.i32_any(), 4);
    }
}

// ===========================================================================
// Rows 11-14 -- the odd / even `lsbit` branches
// ===========================================================================

#[test]
fn row11_aodd_lsbit_one() {
    let mut r = Rng::new(0x000B);
    for _ in 0..ITERS {
        diff(
            r.i32_any(),
            r.i32_any(),
            r.i32_any(),
            r.i32_any(),
            r.i32_any(),
            1,
        );
    }
}

#[test]
fn row12_aodd_random_odd_lsbit() {
    let mut r = Rng::new(0x000C);
    let fixed = [3i32, 5, 7, 9, 4095, i32::MAX];
    for i in 0..ITERS {
        let lsbit = if i < fixed.len() {
            fixed[i]
        } else {
            r.i32_any() | 1
        };
        assert!(lsbit & 1 != 0 && lsbit != 4);
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

#[test]
fn row13_aeven_lsbit_two() {
    let mut r = Rng::new(0x000D);
    for _ in 0..ITERS {
        diff(
            r.i32_any(),
            r.i32_any(),
            r.i32_any(),
            r.i32_any(),
            r.i32_any(),
            2,
        );
    }
}

#[test]
fn row14_aeven_random_even_lsbit_not_four() {
    let mut r = Rng::new(0x000E);
    let fixed = [6i32, 8, 12, 100, 0x4000_0000, i32::MAX - 1];
    for i in 0..ITERS {
        let lsbit = if i < fixed.len() {
            fixed[i]
        } else {
            loop {
                let v = r.i32_any() & !1;
                if v != 0 && v != 4 {
                    break v;
                }
            }
        };
        assert!(lsbit & 1 == 0 && lsbit != 0 && lsbit != 4);
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

// ===========================================================================
// Rows 15-20 -- `step` shapes crossed with all four `lsbit` modes
// ===========================================================================

#[test]
fn row15_step_zero_all_modes_three_way_tie() {
    let mut r = Rng::new(0x000F);
    for &lsbit in LSBIT_MODES.iter() {
        for _ in 0..ITERS {
            diff(r.i32_any(), 0, r.i32_any(), r.i32_any(), r.i32_any(), lsbit);
        }
        for uni in 0..=15 {
            diff(uni, 0, 100, 100, 100, lsbit);
        }
    }
}

#[test]
fn row16_step_tiny_truncation_all_modes_exhaustive_uni() {
    let mut r = Rng::new(0x0010);
    for &lsbit in LSBIT_MODES.iter() {
        for step in 1..=7 {
            for uni in 0..=15 {
                for _ in 0..8 {
                    diff(
                        uni,
                        step,
                        r.range(-1000, 1000),
                        r.range(-1000, 1000),
                        r.range(-1000, 1000),
                        lsbit,
                    );
                }
            }
        }
    }
}

#[test]
fn row17_step_small_negative_all_modes() {
    let mut r = Rng::new(0x0011);
    for &lsbit in LSBIT_MODES.iter() {
        for _ in 0..ITERS {
            diff(
                r.i32_any(),
                step_small_neg(&mut r),
                r.i32_any(),
                r.i32_any(),
                r.i32_any(),
                lsbit,
            );
        }
    }
}

#[test]
fn row18_step_overflow_positive_all_modes() {
    let mut r = Rng::new(0x0012);
    for &lsbit in LSBIT_MODES.iter() {
        for _ in 0..ITERS {
            diff(
                r.i32_any(),
                step_ovf_pos(&mut r),
                r.i32_any(),
                r.i32_any(),
                r.i32_any(),
                lsbit,
            );
        }
    }
}

#[test]
fn row19_step_overflow_negative_all_modes() {
    let mut r = Rng::new(0x0013);
    for &lsbit in LSBIT_MODES.iter() {
        for _ in 0..ITERS {
            diff(
                r.i32_any(),
                step_ovf_neg(&mut r),
                r.i32_any(),
                r.i32_any(),
                r.i32_any(),
                lsbit,
            );
        }
    }
}

#[test]
fn row20_step_extreme_all_modes_exhaustive_uni() {
    let mut r = Rng::new(0x0014);
    for &lsbit in LSBIT_MODES.iter() {
        for &step in &[i32::MAX, i32::MIN, i32::MIN + 1, i32::MAX - 1] {
            for uni in 0..=15 {
                diff(uni, step, 0, 0, 0, lsbit);
                for _ in 0..16 {
                    diff(uni, step, r.i32_any(), r.i32_any(), r.i32_any(), lsbit);
                }
            }
            for _ in 0..ITERS {
                diff(
                    r.i32_any(),
                    step,
                    r.i32_any(),
                    r.i32_any(),
                    r.i32_any(),
                    lsbit,
                );
            }
        }
    }
}

// ===========================================================================
// Rows 21-26 -- `pred` / `tgt` / `tgt2` shapes
// ===========================================================================

#[test]
fn row21_pred_extreme_add_overflow() {
    let mut r = Rng::new(0x0015);
    for &lsbit in LSBIT_MODES.iter() {
        for _ in 0..ITERS {
            let pred = extreme(&mut r, 4096);
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

#[test]
fn row22_tgt_and_tgt2_extreme_sub_overflow() {
    let mut r = Rng::new(0x0016);
    for &lsbit in LSBIT_MODES.iter() {
        for _ in 0..ITERS {
            let tgt = extreme(&mut r, 4096);
            let tgt2 = extreme(&mut r, 4096);
            diff(r.i32_any(), r.i32_any(), r.i32_any(), tgt, tgt2, lsbit);
        }
        // exact ends, including the INT_MIN pseudo-abs corner
        for &tgt in &[i32::MIN, i32::MAX, 0] {
            for &tgt2 in &[i32::MIN, i32::MAX, 0] {
                for &pred in &[i32::MIN, i32::MAX, 0] {
                    for uni in 0..=15 {
                        diff(uni, 1, pred, tgt, tgt2, lsbit);
                        diff(uni, i32::MAX, pred, tgt, tgt2, lsbit);
                        diff(uni, i32::MIN, pred, tgt, tgt2, lsbit);
                    }
                }
            }
        }
    }
}

#[test]
fn row23_tgt2_equals_tgt() {
    let mut r = Rng::new(0x0017);
    for &lsbit in LSBIT_MODES.iter() {
        for _ in 0..ITERS {
            let tgt = r.i32_any();
            diff(r.i32_any(), r.i32_any(), r.i32_any(), tgt, tgt, lsbit);
        }
    }
}

#[test]
fn row24_tgt2_zero() {
    let mut r = Rng::new(0x0018);
    for &lsbit in LSBIT_MODES.iter() {
        for _ in 0..ITERS {
            diff(r.i32_any(), r.i32_any(), r.i32_any(), r.i32_any(), 0, lsbit);
        }
    }
}

#[test]
fn row25_tgt2_far_penalty_dominates() {
    let mut r = Rng::new(0x0019);
    for &lsbit in LSBIT_MODES.iter() {
        for _ in 0..ITERS {
            let pred = r.range(-1_000_000, 1_000_000);
            let step = r.range(1, 4096);
            let tgt = pred.wrapping_add(r.range(-4096, 4096));
            // >= 32 away so `>> 5` is non-zero, and large enough to outweigh
            // the primary distortion term.
            let tgt2 = pred.wrapping_add(if r.bool() {
                r.range(100_000, 50_000_000)
            } else {
                -r.range(100_000, 50_000_000)
            });
            diff(r.i32_any(), step, pred, tgt, tgt2, lsbit);
        }
    }
}

#[test]
fn row26_tgt2_near_penalty_truncates_to_zero() {
    let mut r = Rng::new(0x001A);
    for &lsbit in LSBIT_MODES.iter() {
        for _ in 0..ITERS {
            let pred = r.range(-1_000_000, 1_000_000);
            let step = r.range(1, 64);
            let tgt = pred.wrapping_add(r.range(-64, 64));
            let tgt2 = pred.wrapping_add(r.range(-31, 31));
            diff(r.range(0, 15), step, pred, tgt, tgt2, lsbit);
        }
    }
}

// ===========================================================================
// Rows 27-30 -- forced decision outcomes (axis I)
// ===========================================================================

#[test]
fn row27_outcome_up_forced() {
    let mut r = Rng::new(0x001B);
    let mut hits = 0usize;
    for _ in 0..ITERS {
        let lsbit = 0;
        let uni = r.range(1, 6); // no clamp, sign bit clear
        let step = r.range(64, 100_000);
        let pred = r.range(-1_000_000, 1_000_000);
        let (_, u1, _) = fixup(uni, lsbit);
        let tgt = level(u1, step, pred);
        let outcome = diff_checked(uni, step, pred, tgt, tgt, lsbit);
        if outcome == Outcome::Up {
            hits += 1;
        }
    }
    assert!(hits > ITERS / 2, "outcome I_UP not reached often enough: {hits}");
}

#[test]
fn row28_outcome_down_forced() {
    let mut r = Rng::new(0x001C);
    let mut hits = 0usize;
    for _ in 0..ITERS {
        let lsbit = 0;
        let uni = r.range(1, 6);
        let step = r.range(64, 100_000);
        let pred = r.range(-1_000_000, 1_000_000);
        let (_, _, u2) = fixup(uni, lsbit);
        let tgt = level(u2, step, pred);
        let outcome = diff_checked(uni, step, pred, tgt, tgt, lsbit);
        if outcome == Outcome::Down {
            hits += 1;
        }
    }
    assert!(
        hits > ITERS / 2,
        "outcome I_DOWN not reached often enough: {hits}"
    );
}

#[test]
fn row29_outcome_both_uni2_wins() {
    // I_BOTH is only reachable once the reconstruction levels stop being
    // monotonic, i.e. under signed-multiply/add wraparound. Seeded search.
    let mut r = Rng::new(0x001D);
    let mut hits = 0usize;
    for _ in 0..400_000 {
        let uni = r.i32_any();
        let step = r.i32_any();
        let pred = r.i32_any();
        let tgt = r.i32_any();
        let tgt2 = r.i32_any();
        let lsbit = r.pick(&LSBIT_MODES);
        if model(uni, step, pred, tgt, tgt2, lsbit).1 == Outcome::Both {
            let got = diff(uni, step, pred, tgt, tgt2, lsbit);
            let (_, u1, u2) = fixup(uni, lsbit);
            // the second `if` compares against d0, so uni2 wins outright
            assert_eq!(got, u2, "I_BOTH must select uni2 (uni1 was {u1})");
            hits += 1;
        }
    }
    assert!(hits >= 100, "outcome I_BOTH not reached: {hits}");
    // The known-good witness found by brute force over the C reference.
    assert_eq!(
        model(-289075518, -366398573, -1105009411, -1074194524, 37612886, 0).1,
        Outcome::Both
    );
    diff(-289075518, -366398573, -1105009411, -1074194524, 37612886, 0);
}

#[test]
fn row30_outcome_keep_and_ties_forced() {
    let mut r = Rng::new(0x001E);
    let mut keep = 0usize;
    let mut tie1 = 0usize;
    let mut tie2 = 0usize;
    let mut tieboth = 0usize;
    for _ in 0..ITERS {
        let lsbit = 0;
        let uni = r.range(1, 6);
        let step = r.range(64, 100_000);
        let pred = r.range(-1_000_000, 1_000_000);
        let (u, _, _) = fixup(uni, lsbit);
        let tgt = level(u, step, pred);
        match diff_checked(uni, step, pred, tgt, tgt, lsbit) {
            Outcome::Keep => keep += 1,
            Outcome::Tie1 => tie1 += 1,
            Outcome::Tie2 => tie2 += 1,
            Outcome::TieBoth => tieboth += 1,
            other => panic!("unexpected outcome {other:?} for on-level target"),
        }
    }
    assert!(keep > 0, "outcome I_KEEP never reached");
    // Ties come from the clamping / step==0 shapes; make sure they are seen.
    let mut r2 = Rng::new(0x001E_1E1E);
    for _ in 0..ITERS {
        let uni = if r2.bool() {
            uni_low0(&mut r2)
        } else {
            uni_low7(&mut r2)
        };
        match diff_checked(
            uni,
            r2.range(1, 100_000),
            r2.i32_any(),
            r2.i32_any(),
            r2.i32_any(),
            0,
        ) {
            Outcome::Tie1 => tie1 += 1,
            Outcome::Tie2 => tie2 += 1,
            Outcome::TieBoth => tieboth += 1,
            _ => {}
        }
    }
    assert!(tie1 + tie2 + tieboth > 0, "no tie outcome reached");
}

// ===========================================================================
// Rows 31-34 -- broad sweeps
// ===========================================================================

#[test]
fn row31_realistic_codec_domain() {
    let mut r = Rng::new(0x001F);
    for &lsbit in LSBIT_MODES.iter() {
        for uni in 0..=15 {
            for step in 1..=256 {
                diff(
                    uni,
                    step,
                    r.range(-32768, 32767),
                    r.range(-32768, 32767),
                    r.range(-32768, 32767),
                    lsbit,
                );
            }
        }
    }
}

#[test]
fn row32_unconstrained_fuzz_full_i32() {
    let mut r = Rng::new(0xDEAD_BEEF);
    for _ in 0..200_000 {
        diff(
            r.i32_any(),
            r.i32_any(),
            r.i32_any(),
            r.i32_any(),
            r.i32_any(),
            r.i32_any(),
        );
    }
}

#[test]
fn row33_structured_corner_fuzz() {
    let mut r = Rng::new(0xC0FF_EE01);
    let lsbits: Vec<i32> = CORNERS.iter().copied().chain([0, 4, 1, 2]).collect();
    for _ in 0..200_000 {
        diff(
            r.pick(&CORNERS),
            r.pick(&CORNERS),
            r.pick(&CORNERS),
            r.pick(&CORNERS),
            r.pick(&CORNERS),
            r.pick(&lsbits),
        );
    }
}

#[test]
fn row34_exhaustive_small_grid() {
    let lsbits = [0i32, 1, 2, 3, 4, 5, 6, 8, 12, -1, -2, -4];
    let steps = [0i32, 1, 7, 8, 9, -1, -8, 255, -255];
    let vals = [-1000i32, -33, -1, 0, 31, 1000];
    for uni in 0..=15 {
        for &lsbit in &lsbits {
            for &step in &steps {
                for &pred in &vals {
                    for &tgt in &vals {
                        for &tgt2 in &vals {
                            diff(uni, step, pred, tgt, tgt2, lsbit);
                        }
                    }
                }
            }
        }
    }
}
