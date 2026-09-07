//! Phase B — valid-path differential tests, one test per CONFIGS.md row.
//!
//! Every test calls BOTH the C `.so` and the Rust `.so` through `libloading`
//! and asserts the returned `int` matches byte-for-byte. Each row is driven
//! with many randomized inputs from a fixed-seed PRNG, so the rows are
//! reproducible property tests rather than single hand-picked values.

mod common;
use common::{check, check_args, Rng, LSBIT_MODES};

/// Number of randomized cases per row.
const N: usize = 4000;

// ---------------------------------------------------------------- row 1

#[test]
fn row01_baseline_lsbit0_mid_positive() {
    let mut rng = Rng::new(0x0101);
    for _ in 0..N {
        let uni = rng.uni_mid(false);
        let (step, pred, tgt, tgt2) = rng.typical_tail();
        check(uni, step, pred, tgt, tgt2, 0);
    }
}

// ---------------------------------------------------------------- row 2

#[test]
fn row02_lsbit0_low3_zero_clamps_uni2() {
    let mut rng = Rng::new(0x0202);
    for _ in 0..N {
        let uni = rng.uni_low3(0);
        let (step, pred, tgt, tgt2) = rng.typical_tail();
        check(uni, step, pred, tgt, tgt2, 0);
    }
}

// ---------------------------------------------------------------- row 3

#[test]
fn row03_lsbit0_low3_seven_clamps_uni1() {
    let mut rng = Rng::new(0x0303);
    for _ in 0..N {
        let uni = rng.uni_low3(7);
        let (step, pred, tgt, tgt2) = rng.typical_tail();
        check(uni, step, pred, tgt, tgt2, 0);
    }
}

// ---------------------------------------------------------------- row 4

#[test]
fn row04_lsbit0_bit3_set_negates_diff() {
    let mut rng = Rng::new(0x0404);
    for _ in 0..N {
        let uni = rng.uni_mid(true);
        let (step, pred, tgt, tgt2) = rng.typical_tail();
        check(uni, step, pred, tgt, tgt2, 0);
    }
}

// ---------------------------------------------------------------- row 5

#[test]
fn row05_low3_seven_and_bit3_set() {
    let mut rng = Rng::new(0x0505);
    for _ in 0..N {
        let uni = rng.uni_low4(7, true);
        let (step, pred, tgt, tgt2) = rng.typical_tail();
        check(uni, step, pred, tgt, tgt2, 0);
    }
}

// ---------------------------------------------------------------- row 6

#[test]
fn row06_low3_zero_and_bit3_set() {
    let mut rng = Rng::new(0x0606);
    for _ in 0..N {
        let uni = rng.uni_low4(0, true);
        let (step, pred, tgt, tgt2) = rng.typical_tail();
        check(uni, step, pred, tgt, tgt2, 0);
    }
}

// ---------------------------------------------------------------- row 7

#[test]
fn row07_mixed_sign_pattern_across_candidates() {
    // Sweep every low-4-bit pattern of `uni`, so all combinations of
    // "clamped / not clamped" and "bit 3 set / clear" across uni, uni1 and
    // uni2 occur (e.g. uni&7==7 with bit3 clear means uni1 would flip bit 3).
    let mut rng = Rng::new(0x0707);
    for low4 in 0..16i32 {
        for _ in 0..(N / 16).max(64) {
            let upper = rng.range_i32(-(1 << 20), 1 << 20) & !15;
            let uni = upper | low4;
            let (step, pred, tgt, tgt2) = rng.typical_tail();
            check(uni, step, pred, tgt, tgt2, 0);
        }
    }
}

// ---------------------------------------------------------------- row 8

#[test]
fn row08_lsbit_odd_forces_bit0_set() {
    let mut rng = Rng::new(0x0808);
    let odds = [1i32, 3, 5, 7, 9, 11, -1, -3, -5, -7];
    for _ in 0..N {
        let lsbit = rng.pick(&odds);
        let bit3 = rng.below(2) == 1;
        let uni = rng.uni_mid(bit3);
        let (step, pred, tgt, tgt2) = rng.typical_tail();
        check(uni, step, pred, tgt, tgt2, lsbit);
    }
}

// ---------------------------------------------------------------- row 9

#[test]
fn row09_lsbit_odd_at_group_boundaries() {
    let mut rng = Rng::new(0x0909);
    let odds = [1i32, 3, 5, 7, 9, -1, -3, i32::MAX];
    for _ in 0..N {
        let lsbit = rng.pick(&odds);
        let uni = rng.uni_boundary();
        let (step, pred, tgt, tgt2) = rng.typical_tail();
        check(uni, step, pred, tgt, tgt2, lsbit);
    }
}

// ---------------------------------------------------------------- row 10

#[test]
fn row10_lsbit_even_nonzero_forces_bit0_clear() {
    let mut rng = Rng::new(0x0a0a);
    let evens = [2i32, 6, 8, 10, 12, -2, -4, -6];
    for _ in 0..N {
        let lsbit = rng.pick(&evens);
        let bit3 = rng.below(2) == 1;
        let uni = rng.uni_mid(bit3);
        let (step, pred, tgt, tgt2) = rng.typical_tail();
        check(uni, step, pred, tgt, tgt2, lsbit);
    }
}

// ---------------------------------------------------------------- row 11

#[test]
fn row11_lsbit_even_at_group_boundaries() {
    let mut rng = Rng::new(0x0b0b);
    let evens = [2i32, 6, 8, 10, -2, -4, i32::MIN];
    for _ in 0..N {
        let lsbit = rng.pick(&evens);
        let uni = rng.uni_boundary();
        let (step, pred, tgt, tgt2) = rng.typical_tail();
        check(uni, step, pred, tgt, tgt2, lsbit);
    }
}

// ---------------------------------------------------------------- row 12

#[test]
fn row12_lsbit4_dither_positive_uni() {
    let mut rng = Rng::new(0x0c0c);
    for _ in 0..N {
        let uni = rng.range_i32(0, 1 << 20);
        let (step, pred, tgt, tgt2) = rng.typical_tail();
        check(uni, step, pred, tgt, tgt2, 4);
    }
}

// ---------------------------------------------------------------- row 13

#[test]
fn row13_lsbit4_dither_negative_uni_arithmetic_shift() {
    let mut rng = Rng::new(0x0d0d);
    for _ in 0..N {
        let uni = rng.range_i32(-(1 << 20), -1);
        let (step, pred, tgt, tgt2) = rng.typical_tail();
        check(uni, step, pred, tgt, tgt2, 4);
    }
}

// ---------------------------------------------------------------- row 14

#[test]
fn row14_lsbit4_at_group_boundaries() {
    let mut rng = Rng::new(0x0e0e);
    for _ in 0..N {
        let uni = rng.uni_boundary();
        let (step, pred, tgt, tgt2) = rng.typical_tail();
        check(uni, step, pred, tgt, tgt2, 4);
    }
}

// ---------------------------------------------------------------- row 15

#[test]
fn row15_lsbit4_exhaustive_low_bits() {
    // Every bit pattern of (uni>>1)&(uni>>2)&1 over a full -64..=64 window,
    // with randomized step/targets.
    let mut rng = Rng::new(0x0f0f);
    for uni in -64..=64i32 {
        for _ in 0..64 {
            let (step, pred, tgt, tgt2) = rng.typical_tail();
            check(uni, step, pred, tgt, tgt2, 4);
        }
    }
}

// ---------------------------------------------------------------- row 16

#[test]
fn row16_step_zero_all_ties() {
    let mut rng = Rng::new(0x1010);
    for _ in 0..N {
        let lsbit = rng.pick(&LSBIT_MODES);
        let uni = rng.interesting_i32();
        let pred = rng.typical_target();
        let tgt = rng.typical_target();
        let tgt2 = rng.typical_target();
        check(uni, 0, pred, tgt, tgt2, lsbit);
    }
}

// ---------------------------------------------------------------- row 17

#[test]
fn row17_step_small_truncates_to_zero() {
    let mut rng = Rng::new(0x1111);
    for _ in 0..N {
        let step = rng.range_i32(1, 7);
        let lsbit = rng.pick(&LSBIT_MODES);
        let uni = rng.uni_any_low4();
        let pred = rng.typical_target();
        let tgt = rng.typical_target();
        let tgt2 = rng.typical_target();
        check(uni, step, pred, tgt, tgt2, lsbit);
    }
}

// ---------------------------------------------------------------- row 18

#[test]
fn row18_step_negative_division_truncates_toward_zero() {
    let mut rng = Rng::new(0x1212);
    for _ in 0..N {
        let step = -rng.range_i32(1, 1_000_000);
        let lsbit = rng.pick(&LSBIT_MODES);
        let uni = rng.uni_any_low4();
        let pred = rng.typical_target();
        let tgt = rng.typical_target();
        let tgt2 = rng.typical_target();
        check(uni, step, pred, tgt, tgt2, lsbit);
    }
}

// ---------------------------------------------------------------- row 19

#[test]
fn row19_step_int_max_multiplication_overflow() {
    let mut rng = Rng::new(0x1313);
    for lsbit in LSBIT_MODES {
        for _ in 0..(N / 8).max(64) {
            let uni = rng.uni_any_low4();
            let [pred, tgt, tgt2, _] = rng.interesting4();
            check(uni, i32::MAX, pred, tgt, tgt2, lsbit);
        }
    }
}

// ---------------------------------------------------------------- row 20

#[test]
fn row20_step_int_min_overflow_and_negation() {
    let mut rng = Rng::new(0x1414);
    for lsbit in LSBIT_MODES {
        for _ in 0..(N / 8).max(64) {
            let uni = rng.uni_any_low4();
            let [pred, tgt, tgt2, _] = rng.interesting4();
            check(uni, i32::MIN, pred, tgt, tgt2, lsbit);
        }
    }
}

// ---------------------------------------------------------------- row 21

#[test]
fn row21_zero_targets_all_lsbit_modes() {
    let mut rng = Rng::new(0x1515);
    for lsbit in LSBIT_MODES {
        for _ in 0..(N / 8).max(64) {
            let uni = rng.interesting_i32();
            let step = rng.interesting_i32();
            check(uni, step, 0, 0, 0, lsbit);
        }
    }
}

// ---------------------------------------------------------------- row 22

#[test]
fn row22_targets_near_int_max_subtraction_overflow() {
    let mut rng = Rng::new(0x1616);
    for _ in 0..N {
        let lsbit = rng.pick(&LSBIT_MODES);
        let pred = i32::MAX - rng.range_i32(0, 1000);
        let tgt = i32::MAX - rng.range_i32(0, 1000);
        let tgt2 = i32::MAX - rng.range_i32(0, 1000);
        let uni = rng.interesting_i32();
        let step = rng.typical_step();
        check(uni, step, pred, tgt, tgt2, lsbit);
    }
}

// ---------------------------------------------------------------- row 23

#[test]
fn row23_targets_near_int_min_subtraction_overflow() {
    let mut rng = Rng::new(0x1717);
    for _ in 0..N {
        let lsbit = rng.pick(&LSBIT_MODES);
        let pred = i32::MIN + rng.range_i32(0, 1000);
        let tgt = i32::MIN + rng.range_i32(0, 1000);
        let tgt2 = i32::MIN + rng.range_i32(0, 1000);
        let uni = rng.interesting_i32();
        let step = rng.typical_step();
        check(uni, step, pred, tgt, tgt2, lsbit);
    }
}

// ---------------------------------------------------------------- row 24

#[test]
fn row24_tgt2_equals_tgt_symmetric_penalty() {
    let mut rng = Rng::new(0x1818);
    for _ in 0..N {
        let lsbit = rng.pick(&LSBIT_MODES);
        let tgt = rng.typical_target();
        let uni = rng.interesting_i32();
        let step = rng.typical_step();
        let pred = rng.typical_target();
        check(uni, step, pred, tgt, tgt, lsbit);
    }
}

// ---------------------------------------------------------------- row 25

#[test]
fn row25_secondary_penalty_vanishes() {
    // Keep every |tgt2 - p| < 32 so that d3 >> 5 == 0 and the secondary
    // penalty contributes nothing.
    let mut rng = Rng::new(0x1919);
    for _ in 0..N {
        let lsbit = rng.pick(&LSBIT_MODES);
        let pred = rng.range_i32(-4, 4);
        let step = rng.range_i32(-4, 4);
        let tgt2 = pred.wrapping_add(rng.range_i32(-3, 3));
        let uni = rng.range_i32(-32, 32);
        let tgt = rng.typical_target();
        check(uni, step, pred, tgt, tgt2, lsbit);
    }
}

// ---------------------------------------------------------------- row 26

#[test]
fn row26_secondary_penalty_dominates() {
    let mut rng = Rng::new(0x1a1a);
    for _ in 0..N {
        let lsbit = rng.pick(&LSBIT_MODES);
        let pred = rng.range_i32(-1000, 1000);
        let tgt = pred.wrapping_add(rng.range_i32(-10, 10));
        // tgt2 far away so the >>5 term is large relative to d0/d1/d2.
        let tgt2 = pred.wrapping_add(rng.range_i32(100_000, 10_000_000));
        let uni = rng.range_i32(-64, 64);
        let step = rng.range_i32(-64, 64);
        check(uni, step, pred, tgt, tgt2, lsbit);
    }
}

// ---------------------------------------------------------------- row 27

#[test]
fn row27_secondary_penalty_negative_d3_arithmetic_shift() {
    let mut rng = Rng::new(0x1b1b);
    for _ in 0..N {
        let lsbit = rng.pick(&LSBIT_MODES);
        let pred = rng.range_i32(-1000, 1000);
        let tgt = pred.wrapping_add(rng.range_i32(-10, 10));
        // tgt2 far *below* pred so tgt2 - p is strongly negative.
        let tgt2 = pred.wrapping_sub(rng.range_i32(100_000, 10_000_000));
        let uni = rng.range_i32(-64, 64);
        let step = rng.range_i32(-64, 64);
        check(uni, step, pred, tgt, tgt2, lsbit);
    }
}

// ------------------------------------------------------- rows 28-31

/// Mirror of the C's candidate conditioning, used ONLY to classify which
/// branch produced the C's answer (never as the oracle).
fn conditioned(uni: i32, lsbit: i32) -> (i32, i32, i32) {
    let mut u = uni;
    let mut u1 = uni.wrapping_add(1);
    let mut u2 = uni.wrapping_sub(1);
    if ((uni ^ u1) & !7) != 0 {
        u1 = uni;
    }
    if ((uni ^ u2) & !7) != 0 {
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
        } else if (lsbit & 1) != 0 {
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

#[test]
fn row28to31_selection_outcomes_all_covered() {
    let p = common::pair();
    let mut rng = Rng::new(0x1c1c);
    let mut counts = [0usize; 4];
    for _ in 0..N * 2 {
        let uni = rng.range_i32(-64, 64);
        let step = rng.range_i32(-256, 256);
        let pred = rng.range_i32(-512, 512);
        let tgt = rng.range_i32(-512, 512);
        let tgt2 = rng.range_i32(-4096, 4096);
        let lsbit = rng.pick(&LSBIT_MODES);
        check(uni, step, pred, tgt, tgt2, lsbit);

        // Classify against the C's own answer, purely to prove coverage of
        // rows 28 (Gkeep), 29 (Gup) and 30 (Gdown).
        let out = unsafe { (p.c)(uni, step, pred, tgt, tgt2, lsbit) };
        let (cu, cu1, cu2) = conditioned(uni, lsbit);
        if out == cu && out != cu1 && out != cu2 {
            counts[0] += 1;
        } else if out == cu1 && out != cu {
            counts[1] += 1;
        } else if out == cu2 && out != cu {
            counts[2] += 1;
        } else {
            counts[3] += 1; // candidates coincide — unclassifiable
        }
    }
    assert!(counts[0] > 0, "row 28 (Gkeep) never exercised: {counts:?}");
    assert!(counts[1] > 0, "row 29 (Gup) never exercised: {counts:?}");
    assert!(counts[2] > 0, "row 30 (Gdown) never exercised: {counts:?}");
}

#[test]
fn row31_both_better_second_if_wins() {
    // When BOTH d1<d0 and d2<d0 the C's second `if` overwrites the first, so
    // uni2 is returned even if uni1 was strictly better. Confirm the Rust
    // reproduces that, and that the case is actually reached.
    let mut rng = Rng::new(0x1f1f);
    let mut hits = 0usize;
    let p = common::pair();
    for _ in 0..N * 4 {
        let uni = rng.range_i32(-64, 64);
        let step = rng.range_i32(1, 4096);
        let pred = rng.range_i32(-4096, 4096);
        let tgt = rng.range_i32(-4096, 4096);
        let tgt2 = rng.range_i32(-4096, 4096);
        check(uni, step, pred, tgt, tgt2, 0);

        let (cu, cu1, cu2) = conditioned(uni, 0);
        let out = unsafe { (p.c)(uni, step, pred, tgt, tgt2, 0) };
        if cu1 != cu && cu2 != cu && out == cu2 {
            hits += 1;
        }
    }
    assert!(hits > 0, "row 31: never observed uni2 winning outright");
}

// ---------------------------------------------------------------- row 32

#[test]
fn row32_negative_uni_all_lsbit_modes() {
    let mut rng = Rng::new(0x2020);
    for lsbit in LSBIT_MODES {
        for _ in 0..(N / 4).max(128) {
            let uni = -rng.range_i32(1, 1 << 24);
            let (step, pred, tgt, tgt2) = rng.typical_tail();
            check(uni, step, pred, tgt, tgt2, lsbit);
        }
    }
}

// ---------------------------------------------------------------- row 33

#[test]
fn row33_exhaustive_small_grid() {
    // uni in -64..=64, lsbit in -4..=9, over a small step/target grid.
    for uni in -64..=64i32 {
        for lsbit in -4..=9i32 {
            for step in [-9i32, -1, 0, 1, 3, 8, 17, 64] {
                for pred in [-33i32, 0, 7, 100] {
                    for tgt in [-100i32, 0, 5, 250] {
                        for tgt2 in [-4096i32, -1, 0, 63, 4096] {
                            check(uni, step, pred, tgt, tgt2, lsbit);
                        }
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------- row 34

#[test]
fn row34_full_range_uniform_fuzz() {
    let mut rng = Rng::new(0xDEADBEEF);
    for _ in 0..200_000 {
        check_args(rng.uniform_args());
    }
}

// ---------------------------------------------------------------- row 35

#[test]
fn row35_structured_interesting_fuzz() {
    let mut rng = Rng::new(0xC0FFEE);
    for _ in 0..200_000 {
        check_args(rng.interesting_args());
    }
}
