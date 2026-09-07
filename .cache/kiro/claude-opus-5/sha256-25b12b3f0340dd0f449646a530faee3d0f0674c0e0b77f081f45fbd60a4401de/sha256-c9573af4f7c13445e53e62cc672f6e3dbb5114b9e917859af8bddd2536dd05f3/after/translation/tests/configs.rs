//! Phase B — valid-path differential tests, one `#[test]` per row of
//! `CONFIGS.md` (rows 1..28).
//!
//! Every test drives BOTH shared objects through their `ldexp_q2` export and
//! asserts the returned `f32` is bit-identical. Inputs are property-style:
//! many randomized values per row from a fixed-seed SplitMix64 PRNG, so the
//! runs are reproducible.

mod common;

use common::{assert_batch, assert_same, specials, Rng, HUGE_EXPS, SPECIAL_EXPS};

/// How many randomized values each row uses.
const N: usize = 4000;

/// Build `n` random `(y, exp_q2)` cases where `exp_q2` comes from `pick`.
fn cases(seed: u64, n: usize, mut pick: impl FnMut(&mut Rng) -> i32) -> Vec<(f32, i32)> {
    let mut rng = Rng::new(seed);
    let mut out = Vec::with_capacity(n * 2);
    for i in 0..n {
        // Alternate between wide-range normals and moderate magnitudes so both
        // the saturating and the precision-sensitive paths get hit.
        let y = if i % 3 == 0 {
            rng.normal_f32()
        } else if i % 3 == 1 {
            rng.moderate_f32()
        } else {
            rng.any_f32()
        };
        let e = pick(&mut rng);
        out.push((y, e));
    }
    out
}

/// Cross every special `f32` with the given fixed exponents.
fn specials_x(exps: &[i32]) -> Vec<(f32, i32)> {
    let mut out = Vec::new();
    for y in specials() {
        for &e in exps {
            out.push((y, e));
        }
    }
    out
}

// --- Row 1: exp_q2 == 0, random normals -----------------------------------

#[test]
fn row01_exp_zero_random_normals() {
    assert_batch("row01", cases(0x1001, N, |_| 0));
}

// --- Row 2: exp_q2 == 0, special f32 values -------------------------------

#[test]
fn row02_exp_zero_specials() {
    assert_batch("row02", specials_x(&[0]));
}

// --- Row 3: exp_q2 in 1..=119 (clamp takes exp_q2, 1 iteration) ------------

#[test]
fn row03_exp_1_to_119() {
    assert_batch("row03", cases(0x1003, N, |r| r.range_i32(1, 119)));
}

// --- Row 4: exp_q2 == 120 exactly (clamp boundary, else branch) ------------

#[test]
fn row04_exp_120_boundary() {
    assert_batch("row04", cases(0x1004, N, |_| 120));
    assert_batch("row04_specials", specials_x(&[120]));
}

// --- Rows 5..8: each g_expfrac index via positive exp_q2 -------------------

#[test]
fn row05_index0_positive() {
    assert_batch("row05", cases(0x1005, N, |r| 4 * r.range_i32(1, 29)));
}

#[test]
fn row06_index1_positive() {
    assert_batch("row06", cases(0x1006, N, |r| 4 * r.range_i32(0, 29) + 1));
}

#[test]
fn row07_index2_positive() {
    assert_batch("row07", cases(0x1007, N, |r| 4 * r.range_i32(0, 29) + 2));
}

#[test]
fn row08_index3_positive() {
    assert_batch("row08", cases(0x1008, N, |r| 4 * r.range_i32(0, 29) + 3));
}

// --- Row 9: sweep every in-range shift e>>2 = 0..=30 ----------------------

#[test]
fn row09_shift_sweep_in_range() {
    let mut rng = Rng::new(0x1009);
    let mut out = Vec::new();
    for k in 0..=30i32 {
        let e = 4 * k;
        for _ in 0..200 {
            out.push((rng.moderate_f32(), e));
        }
        for _ in 0..100 {
            out.push((rng.any_f32(), e));
        }
        for y in specials() {
            out.push((y, e));
        }
    }
    assert_batch("row09", out);
}

// --- Row 10: exp_q2 == 121 (exactly 2 iterations) -------------------------

#[test]
fn row10_exp_121_two_iterations() {
    assert_batch("row10", cases(0x100A, N, |_| 121));
    assert_batch("row10_specials", specials_x(&[121]));
}

// --- Row 11: exp_q2 in 121..=240 (2 iterations, all remainders) -----------

#[test]
fn row11_exp_121_to_240() {
    assert_batch("row11", cases(0x100B, N, |r| r.range_i32(121, 240)));
    // deterministic full sweep of the range as well
    let mut rng = Rng::new(0x100B_2);
    let mut out = Vec::new();
    for e in 121..=240i32 {
        for _ in 0..20 {
            out.push((rng.moderate_f32(), e));
        }
    }
    assert_batch("row11_sweep", out);
}

// --- Row 12: exp_q2 == 240 (remainder exactly 120) ------------------------

#[test]
fn row12_exp_240() {
    assert_batch("row12", cases(0x100C, N, |_| 240));
    assert_batch("row12_specials", specials_x(&[240]));
}

// --- Row 13: exp_q2 == 241 (3 iterations) --------------------------------

#[test]
fn row13_exp_241() {
    assert_batch("row13", cases(0x100D, N, |_| 241));
    assert_batch("row13_specials", specials_x(&[241]));
}

// --- Row 14: exp_q2 in 241..=10_000 (many iterations) --------------------

#[test]
fn row14_exp_241_to_10000() {
    assert_batch("row14", cases(0x100E, N, |r| r.range_i32(241, 10_000)));
}

// --- Row 15: exp_q2 in 10_001..=1_000_000 (thousands of iterations) ------

#[test]
fn row15_exp_large_positive() {
    assert_batch("row15", cases(0x100F, 600, |r| r.range_i32(10_001, 1_000_000)));
}

// --- Row 16: large positive exp_q2 with special y ------------------------

#[test]
fn row16_large_positive_with_specials() {
    assert_batch(
        "row16",
        specials_x(&[241, 1_000, 12_345, 100_000, 999_983, 1_000_000]),
    );
}

// --- Row 17: exp_q2 == INT_MAX (maximum trip count) ---------------------

#[test]
fn row17_exp_int_max() {
    let mut out = Vec::new();
    for &e in HUGE_EXPS {
        for y in [
            1.0f32,
            -1.0f32,
            f32::MAX,
            f32::MIN_POSITIVE,
            0.0f32,
            -0.0f32,
            f32::INFINITY,
            f32::NEG_INFINITY,
            f32::NAN,
            f32::from_bits(0x7F80_0001),
            3.14159f32,
        ] {
            out.push((y, e));
        }
    }
    assert_batch("row17", out);
}

// --- Row 18: exp_q2 == -1 (masked shift 31 -> factor 0, index 3) ---------

#[test]
fn row18_exp_minus_one() {
    assert_batch("row18", cases(0x1012, N, |_| -1));
    assert_batch("row18_specials", specials_x(&[-1]));
}

// --- Row 19: exp_q2 == -2 / -3 / -4 -------------------------------------

#[test]
fn row19_exp_minus_2_3_4() {
    for e in [-2i32, -3, -4] {
        assert_batch(&format!("row19_e{e}"), cases(0x1013 ^ (e as u64), N, |_| e));
        assert_batch(&format!("row19_specials_e{e}"), specials_x(&[e]));
    }
}

// --- Row 20: exp_q2 in -127..=-1 ---------------------------------------

#[test]
fn row20_exp_minus127_to_minus1() {
    assert_batch("row20", cases(0x1014, N, |r| r.range_i32(-127, -1)));
    let mut rng = Rng::new(0x1014_2);
    let mut out = Vec::new();
    for e in -127..=-1i32 {
        for _ in 0..20 {
            out.push((rng.moderate_f32(), e));
        }
        for y in specials() {
            out.push((y, e));
        }
    }
    assert_batch("row20_sweep", out);
}

// --- Row 21: exp_q2 == -128 (masked shift wraps to 0) -------------------

#[test]
fn row21_exp_minus_128() {
    assert_batch("row21", cases(0x1015, N, |_| -128));
    assert_batch("row21_specials", specials_x(&[-128]));
}

// --- Row 22: negative multiples of -128 --------------------------------

#[test]
fn row22_negative_multiples_of_128() {
    let exps: Vec<i32> = (1..=64).map(|k| -128 * k).collect();
    let mut rng = Rng::new(0x1016);
    let mut out = Vec::new();
    for &e in &exps {
        for _ in 0..60 {
            out.push((rng.moderate_f32(), e));
            out.push((rng.any_f32(), e));
        }
        for y in specials() {
            out.push((y, e));
        }
    }
    assert_batch("row22", out);
}

// --- Row 23: sweep the masked shift range for negatives ----------------

#[test]
fn row23_negative_shift_sweep() {
    let mut rng = Rng::new(0x1017);
    let mut out = Vec::new();
    for k in 1..=64i32 {
        let e = -4 * k;
        for _ in 0..80 {
            out.push((rng.moderate_f32(), e));
            out.push((rng.any_f32(), e));
        }
        for y in specials() {
            out.push((y, e));
        }
    }
    // and the same sweep offset by 1..3 so the table index varies too
    for k in 1..=64i32 {
        for off in 1..=3i32 {
            let e = -4 * k + off;
            for _ in 0..20 {
                out.push((rng.moderate_f32(), e));
            }
        }
    }
    assert_batch("row23", out);
}

// --- Row 24: exp_q2 in INT_MIN..=-1_000_000 ---------------------------

#[test]
fn row24_large_negative() {
    assert_batch(
        "row24",
        cases(0x1018, N, |r| r.range_i32(i32::MIN, -1_000_000)),
    );
}

// --- Row 25: exp_q2 == INT_MIN ---------------------------------------

#[test]
fn row25_exp_int_min() {
    assert_batch("row25", cases(0x1019, N, |_| i32::MIN));
    assert_batch("row25_specials", specials_x(&[i32::MIN]));
}

// --- Row 26: exp_q2 == INT_MIN + 1..4 --------------------------------

#[test]
fn row26_exp_int_min_plus() {
    let exps = [
        i32::MIN + 1,
        i32::MIN + 2,
        i32::MIN + 3,
        i32::MIN + 4,
        i32::MIN + 5,
    ];
    assert_batch(
        "row26",
        cases(0x101A, N, move |r| exps[(r.next_u32() as usize) % exps.len()]),
    );
    assert_batch("row26_specials", specials_x(&exps));
}

// --- Row 27: unconstrained fuzz over the whole input space -------------

#[test]
fn row27_full_fuzz() {
    let mut rng = Rng::new(0xDEAD_BEEF_CAFE_F00D);
    let mut out: Vec<(f32, i32)> = Vec::new();
    for _ in 0..20_000 {
        let y = f32::from_bits(rng.next_u32());
        // Bias toward the interesting boundaries but still cover the whole i32
        // range. Positive exponents are capped so the loop trip count stays
        // bounded (the millions-of-iterations case is row 17).
        let e = match rng.next_u32() % 8 {
            0 => SPECIAL_EXPS[(rng.next_u32() as usize) % SPECIAL_EXPS.len()],
            1 => rng.range_i32(-300, 300),
            2 => rng.range_i32(-1, 1),
            3 => rng.range_i32(i32::MIN, -1),
            4 => rng.range_i32(115, 130),
            5 => rng.range_i32(-135, -115),
            6 => rng.range_i32(0, 200_000),
            _ => {
                let v = rng.next_i32();
                if v > 2_000_000 {
                    v % 2_000_000
                } else {
                    v
                }
            }
        };
        out.push((y, e));
    }
    assert_batch("row27", out);
}

// --- Row 28: statefulness / repeatability -----------------------------

#[test]
fn row28_no_hidden_state() {
    // The C table is `static const`; calling with other inputs in between must
    // not change the answer for a fixed input.
    let probes: [(f32, i32); 6] = [
        (1.0, 0),
        (-3.5, 7),
        (1e30, -1),
        (f32::MIN_POSITIVE, 121),
        (f32::NAN, -128),
        (0.25, 1_000),
    ];
    let mut baseline = Vec::new();
    for &(y, e) in &probes {
        let v = common::check(y, e).unwrap_or_else(|m| panic!("{m}"));
        baseline.push(v.to_bits());
    }

    let mut rng = Rng::new(0x101C);
    for round in 0..200 {
        for _ in 0..20 {
            let y = rng.any_f32();
            let e = rng.range_i32(-500, 500);
            assert_same(y, e);
        }
        for (i, &(y, e)) in probes.iter().enumerate() {
            let v = common::check(y, e).unwrap_or_else(|m| panic!("{m}"));
            assert_eq!(
                v.to_bits(),
                baseline[i],
                "round {round}: ldexp_q2({y:?}, {e}) changed across calls — hidden state?"
            );
        }
    }
}
