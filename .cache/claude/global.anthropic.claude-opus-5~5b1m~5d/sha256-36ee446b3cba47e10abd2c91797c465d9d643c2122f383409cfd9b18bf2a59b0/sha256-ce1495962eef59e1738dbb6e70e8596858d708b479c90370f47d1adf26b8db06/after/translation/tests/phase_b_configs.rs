//! Phase B - valid-path differential tests, one test per row of `CONFIGS.md`.
//!
//! Every test calls BOTH the C `.so` export and the Rust `.so` export with the
//! same arguments and asserts the returned `int`s are identical. All rows use
//! many randomized inputs from a fixed seed.

mod common;

use common::*;

/// Number of randomized draws per pinned-axis row.
const CASES: usize = 20_000;

// ---------------------------------------------------------------------------
// Rows 1-13: axis L (the `lsbit` mode selector), U and V randomized.
// ---------------------------------------------------------------------------

macro_rules! l_row {
    ($name:ident, $row:literal, $class:literal) => {
        #[test]
        fn $name() {
            let n = sweep_row($row, Some($class), None, None, CASES);
            assert_eq!(n, CASES);
        }
    };
}

l_row!(row01_lsbit_0_block_skipped, "row01/L0", "L0");
l_row!(row02_lsbit_4_dither_branch, "row02/L1", "L1");
l_row!(row03_lsbit_1_set_bit0, "row03/L2", "L2");
l_row!(row04_lsbit_3_set_bit0, "row04/L3", "L3");
l_row!(row05_lsbit_5_set_bit0_past_4, "row05/L4", "L4");
l_row!(row06_lsbit_2_clear_bit0, "row06/L5", "L5");
l_row!(row07_lsbit_6_clear_bit0_past_4, "row07/L6", "L6");
l_row!(row08_lsbit_8_clear_bit0, "row08/L7", "L7");
l_row!(row09_lsbit_neg1_negative_odd, "row09/L8", "L8");
l_row!(row10_lsbit_neg4_negative_even, "row10/L9", "L9");
l_row!(row11_lsbit_int_max_odd, "row11/L10", "L10");
l_row!(row12_lsbit_int_min_even, "row12/L11", "L11");
l_row!(row13_lsbit_fully_random, "row13/L12", "L12");

// ---------------------------------------------------------------------------
// Rows 14-25: axis U (the `uni` shape), L and V randomized.
// ---------------------------------------------------------------------------

macro_rules! u_row {
    ($name:ident, $row:literal, $class:literal) => {
        #[test]
        fn $name() {
            let n = sweep_row($row, None, Some($class), None, CASES);
            assert_eq!(n, CASES);
        }
    };
}

u_row!(row14_uni_0_uni2_clamped, "row14/U0", "U0");
u_row!(row15_uni_8_clamped_and_negated, "row15/U1", "U1");
u_row!(row16_uni_7_uni1_clamped, "row16/U2", "U2");
u_row!(row17_uni_15_clamped_and_negated, "row17/U3", "U3");
u_row!(row18_uni_1_to_6_no_clamp_positive, "row18/U4", "U4");
u_row!(row19_uni_9_to_14_no_clamp_negated, "row19/U5", "U5");
u_row!(row20_uni_canonical_0_to_15, "row20/U6", "U6");
u_row!(row21_uni_positive_high_bits, "row21/U7", "U7");
u_row!(row22_uni_negative, "row22/U8", "U8");
u_row!(row23_uni_int_max_overflow_plus1, "row23/U9", "U9");
u_row!(row24_uni_int_min_overflow_minus1, "row24/U10", "U10");
u_row!(row25_uni_fully_random, "row25/U11", "U11");

// ---------------------------------------------------------------------------
// Rows 26-34: axis V (step / pred / tgt / tgt2 shape), L and U randomized.
// ---------------------------------------------------------------------------

macro_rules! v_row {
    ($name:ident, $row:literal, $class:literal) => {
        #[test]
        fn $name() {
            let n = sweep_row($row, None, None, Some($class), CASES);
            assert_eq!(n, CASES);
        }
    };
}

v_row!(row26_values_typical_codec_range, "row26/V0", "V0");
v_row!(row27_step_0_to_7_division_truncates, "row27/V1", "V1");
v_row!(row28_step_zero_all_candidates_tie, "row28/V2", "V2");
v_row!(row29_step_negative, "row29/V3", "V3");
v_row!(row30_step_overflowing, "row30/V4", "V4");
v_row!(row31_tgt2_equals_tgt, "row31/V5", "V5");
v_row!(row32_tgt2_far_d3_dominates, "row32/V6", "V6");
v_row!(row33_all_params_at_extremes, "row33/V7", "V7");
v_row!(row34_all_params_fully_random, "row34/V8", "V8");

// ---------------------------------------------------------------------------
// Rows 35-38: axis S (candidate-selection outcome).
//
// A test-side model of the C control flow is used ONLY to classify which of the
// four selection outcomes a tuple reaches, so that each outcome is provably
// exercised. The pass/fail assertion is always C-vs-Rust, never model-vs-Rust.
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Outcome {
    /// `d1 >= d0 && d2 >= d0` -> `uni`
    S0,
    /// `d1 < d0 && d2 >= d0` -> `uni1`
    S1,
    /// `d1 >= d0 && d2 < d0` -> `uni2`
    S2,
    /// `d1 < d0 && d2 < d0` -> `uni2` (the second `if` overwrites the first)
    S3,
}

/// Internal state of the C algorithm, recovered for classification only.
#[derive(Clone, Copy, Debug)]
struct Model {
    /// `uni` after lsbit conditioning (the value returned when `d0` wins).
    uni: i32,
    /// `uni1` after clamping + lsbit conditioning.
    uni1: i32,
    /// `uni2` after clamping + lsbit conditioning.
    uni2: i32,
    d0: i32,
    d1: i32,
    d2: i32,
}

/// Mirror of `c_src/src/lib.c` used purely to label a tuple's selection outcome.
fn model(a: Args) -> Model {
    let mut uni = a.uni;
    let mut uni1 = uni.wrapping_add(1);
    let mut uni2 = uni.wrapping_sub(1);
    if ((uni ^ uni1) & !7) != 0 {
        uni1 = uni;
    }
    if ((uni ^ uni2) & !7) != 0 {
        uni2 = uni;
    }
    if a.lsbit != 0 {
        if a.lsbit == 4 {
            uni &= !1;
            uni1 &= !1;
            uni2 &= !1;
            uni |= (uni >> 1) & (uni >> 2) & 1;
            uni1 |= (uni1 >> 1) & (uni1 >> 2) & 1;
            uni2 |= (uni2 >> 1) & (uni2 >> 2) & 1;
        } else if (a.lsbit & 1) != 0 {
            uni |= 1;
            uni1 |= 1;
            uni2 |= 1;
        } else {
            uni &= !1;
            uni1 &= !1;
            uni2 &= !1;
        }
    }
    let diff_of = |u: i32| -> i32 {
        let mut d = 2i32
            .wrapping_mul(u & 7)
            .wrapping_add(1)
            .wrapping_mul(a.step)
            / 8;
        if (u & 8) != 0 {
            d = d.wrapping_neg();
        }
        d
    };
    let p0 = a.pred.wrapping_add(diff_of(uni));
    let p1 = a.pred.wrapping_add(diff_of(uni1));
    let p2 = a.pred.wrapping_add(diff_of(uni2));
    let abs = |x: i32| x ^ (x >> 31);
    let mut d0 = abs(a.tgt.wrapping_sub(p0));
    let mut d1 = abs(a.tgt.wrapping_sub(p1));
    let mut d2 = abs(a.tgt.wrapping_sub(p2));
    d0 = d0.wrapping_add(abs(a.tgt2.wrapping_sub(p0)) >> 5);
    d1 = d1.wrapping_add(abs(a.tgt2.wrapping_sub(p1)) >> 5);
    d2 = d2.wrapping_add(abs(a.tgt2.wrapping_sub(p2)) >> 5);

    Model { uni, uni1, uni2, d0, d1, d2 }
}

fn classify(a: Args) -> Outcome {
    let m = model(a);
    match (m.d1 < m.d0, m.d2 < m.d0) {
        (false, false) => Outcome::S0,
        (true, false) => Outcome::S1,
        (false, true) => Outcome::S2,
        (true, true) => Outcome::S3,
    }
}

/// The value the C returns for a given outcome, per lines 57-61.
fn expected_return(m: &Model, o: Outcome) -> i32 {
    match o {
        Outcome::S0 => m.uni,
        Outcome::S1 => m.uni1,
        // Both S2 and S3 return uni2: in S3 the second `if` overwrites uni1.
        Outcome::S2 | Outcome::S3 => m.uni2,
    }
}

/// Search randomized tuples for the requested outcome and diff-check each hit.
fn selection_row(row: &str, want: Outcome, wanted_hits: usize) -> usize {
    let mut rng = Rng::for_row(row);
    let mut hits = 0usize;
    let mut tried = 0usize;
    // Generous search budget; every row below is reached easily in practice.
    while hits < wanted_hits && tried < 40_000_000 {
        tried += 1;
        // Mix across all axis classes so the hits are diverse, not one shape.
        let l = L_CLASSES[(rng.next_u64() % 13) as usize];
        let u = U_CLASSES[(rng.next_u64() % 12) as usize];
        let v = V_CLASSES[(rng.next_u64() % 9) as usize];
        let a = gen_args(l, u, v, &mut rng);
        let m = model(a);
        if classify(a) == want {
            let got = check(row, a);
            // Also confirm the outcome really is the one claimed, so a row can
            // never pass by silently exercising a different branch.
            assert_eq!(
                got,
                expected_return(&m, want),
                "[{row}] outcome {want:?} should return {} for args={a:?} (model={m:?})",
                expected_return(&m, want)
            );
            hits += 1;
        }
    }
    assert_eq!(
        hits, wanted_hits,
        "[{row}] only found {hits}/{wanted_hits} tuples with outcome {want:?} in {tried} tries"
    );
    hits
}

#[test]
fn row35_selection_d0_wins_returns_uni() {
    selection_row("row35/S0", Outcome::S0, 20_000);
}

#[test]
fn row36_selection_only_d1_lt_d0_returns_uni1() {
    selection_row("row36/S1", Outcome::S1, 20_000);
}

#[test]
fn row37_selection_only_d2_lt_d0_returns_uni2() {
    selection_row("row37/S2", Outcome::S2, 20_000);
}

/// The quirk row: when both candidates beat `d0`, the C's second `if`
/// unconditionally overwrites the first, so `uni2` wins even if `d1 < d2`.
#[test]
fn row38_selection_both_lt_d0_uni2_overwrites_uni1() {
    selection_row("row38/S3", Outcome::S3, 20_000);

    // Additionally pin down the quirk: find tuples where BOTH candidates beat
    // d0, uni1 is strictly the *better* of the two (d1 < d2), and the two
    // candidates are distinguishable (uni1 != uni2). A "sensible" implementation
    // would return uni1; the C returns uni2 because the second `if` overwrites
    // the first. Assert that C and Rust both return uni2.
    //
    // Note: this outcome is unreachable for the plain monotone shape
    // (uni in 1..6, lsbit = 0, step > 0) because clamping forces uni/uni1/uni2 to
    // share bit 3, making p2 < p0 < p1 monotone so at most one candidate can
    // beat d0. It needs a non-monotone shape (negative/overflowing step, lsbit
    // conditioning, or wrapping arithmetic), so the search ranges over all axes.
    let mut rng = Rng::for_row("row38/S3-quirk");
    let mut found = 0usize;
    let mut tried = 0usize;
    while found < 200 && tried < 40_000_000 {
        tried += 1;
        let l = L_CLASSES[(rng.next_u64() % 13) as usize];
        let u = U_CLASSES[(rng.next_u64() % 12) as usize];
        let v = V_CLASSES[(rng.next_u64() % 9) as usize];
        let a = gen_args(l, u, v, &mut rng);
        let m = model(a);
        if !(m.d1 < m.d0 && m.d2 < m.d0 && m.d1 < m.d2 && m.uni1 != m.uni2) {
            continue;
        }
        let got = check("row38/S3-quirk", a);
        assert_eq!(
            got, m.uni2,
            "row38: with d1<d0, d2<d0 and d1<d2 the C must still return uni2 \
             (={}) not the better uni1 (={}); args={a:?}",
            m.uni2, m.uni1
        );
        found += 1;
    }
    assert!(
        found >= 200,
        "row38: only found {found} quirk tuples in {tried} tries"
    );
}

// ---------------------------------------------------------------------------
// Rows 39-42: exhaustive / large sweeps.
// ---------------------------------------------------------------------------

/// Row 39: exhaustive over the canonical 4-bit `uni`, the small `lsbit` modes,
/// and `step` 0..=64, with a fixed representative `pred`/`tgt`/`tgt2` set.
#[test]
fn row39_exhaustive_canonical_uni_lsbit_step() {
    const VALS: [i32; 4] = [-1000, 0, 7, 100_000];
    let mut n = 0usize;
    for uni in 0..=15i32 {
        for lsbit in 0..=8i32 {
            for step in 0..=64i32 {
                for pred in VALS {
                    for tgt in VALS {
                        for tgt2 in VALS {
                            check(
                                "row39",
                                Args::new(uni, step, pred, tgt, tgt2, lsbit),
                            );
                            n += 1;
                        }
                    }
                }
            }
        }
    }
    assert_eq!(n, 16 * 9 * 65 * 4 * 4 * 4);
}

/// Row 40: exhaustive over `uni` in -16..=16 and `lsbit` in -8..=8 (so the
/// negative-odd / negative-even branches and negative shifts are all covered),
/// with randomized `step`/`pred`/`tgt`/`tgt2` per combination.
#[test]
fn row40_exhaustive_small_signed_uni_and_lsbit() {
    let mut rng = Rng::for_row("row40");
    let mut n = 0usize;
    for uni in -16..=16i32 {
        for lsbit in -8..=8i32 {
            for _ in 0..512 {
                let v = V_CLASSES[(rng.next_u64() % 9) as usize];
                let (step, pred, tgt, tgt2) = gen_values(v, &mut rng);
                check("row40", Args::new(uni, step, pred, tgt, tgt2, lsbit));
                n += 1;
            }
        }
    }
    assert_eq!(n, 33 * 17 * 512);
}

/// Row 41: exhaustive cross-product of the signed extremes in all six slots.
#[test]
fn row41_exhaustive_extremes_cross_product() {
    let mut n = 0usize;
    for uni in EXTREMES {
        for step in EXTREMES {
            for pred in EXTREMES {
                for tgt in EXTREMES {
                    for tgt2 in EXTREMES {
                        for lsbit in EXTREMES {
                            check(
                                "row41",
                                Args::new(uni, step, pred, tgt, tgt2, lsbit),
                            );
                            n += 1;
                        }
                    }
                }
            }
        }
    }
    assert_eq!(n, 7usize.pow(6));
}

/// Row 42: large unconstrained random fuzz.
#[test]
fn row42_large_random_fuzz() {
    let mut rng = Rng::for_row("row42");
    let mut n = 0usize;
    for _ in 0..1_048_576 {
        let a = Args::new(
            rng.i32_any(),
            rng.i32_any(),
            rng.i32_any(),
            rng.i32_any(),
            rng.i32_any(),
            rng.i32_any(),
        );
        check("row42", a);
        n += 1;
    }
    assert_eq!(n, 1_048_576);
}

// ---------------------------------------------------------------------------
// Row 43: the full cross-product of every axis class.
// ---------------------------------------------------------------------------

/// Row 43: every L class x every U class x every V class (13 * 12 * 9 = 1404
/// combinations), 128 randomized draws each. This is where option/data-shape
/// *interactions* are exercised rather than one axis at a time.
#[test]
fn row43_full_axis_cross_product() {
    let mut combos = 0usize;
    let mut n = 0usize;
    for l in L_CLASSES {
        for u in U_CLASSES {
            for v in V_CLASSES {
                let row = format!("row43/{l}x{u}x{v}");
                let mut rng = Rng::for_row(&row);
                for _ in 0..128 {
                    let a = gen_args(l, u, v, &mut rng);
                    check(&row, a);
                    n += 1;
                }
                combos += 1;
            }
        }
    }
    assert_eq!(combos, 13 * 12 * 9);
    assert_eq!(n, 13 * 12 * 9 * 128);
}
