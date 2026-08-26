//! Phase C — error / rejection-path differential tests.
//!
//! One test per row of `ERRORS.md`. `c_src/src/lib.c` has no error returns, no
//! asserts and no pointer/enum parameters, so the rejection surface consists of
//! the degenerate / boundary / undefined-behaviour conditions enumerated there.
//! Each test asserts that C and Rust produce the *same specific* result
//! (documented bit pattern), not merely that "both did something".

mod common;

use common::{check, check_exact, y_panel, Rng, SEED};

const P_INF: u32 = 0x7F80_0000;
const N_INF: u32 = 0xFF80_0000;

/// E1 — UB: negative shift count. gcc emits `sar %cl`, x86 masks `%cl` to 5 bits.
/// Must not panic and must not behave like a 0 or saturating shift.
#[test]
fn e01_negative_shift_count_is_masked_to_5_bits() {
    // Hand-checked reference values from the C .so.
    check_exact("E1", 1.0, -5, 0x3018_37F0);
    check_exact("E1", 1.0, -8, 0x3080_0000);
    check_exact("E1", 1.0, -16, 0x3180_0000);
    check_exact("E1", 1.0, -32, 0x3380_0000);
    check_exact("E1", 1.0, -33, 0x3398_37F0);
    check_exact("E1", 1.0, -64, 0x3780_0000);

    // A 0-shift would give 2^30 for every negative input and a saturating shift
    // would give 0; assert the whole negative range agrees with C instead.
    let mut rng = Rng::new(SEED ^ 101);
    for e in -4096..0 {
        check("E1", 1.0, e);
        check("E1", -1.0, e);
    }
    for _ in 0..20_000 {
        let e = rng.range_i32(i32::MIN, -1);
        check("E1", rng.any_f32(), e);
    }
}

/// E2 — shift count lands on 31 → `shifted == 0` → signed-zero result.
#[test]
fn e02_shift_count_31_yields_signed_zero() {
    check_exact("E2", 1.0, -1, 0x0000_0000);
    check_exact("E2", 1.0, -2, 0x0000_0000);
    check_exact("E2", 1.0, -3, 0x0000_0000);
    check_exact("E2", -1.0, -4, 0x8000_0000);
    check_exact("E2", 1.0, -129, 0x0000_0000);
    check_exact("E2", -1.0, -132, 0x8000_0000);

    // Every `exp_q2` whose (exp_q2>>2) ≡ 31 (mod 32) within a few periods.
    for k in 0..32 {
        for r in 0..4 {
            let e = -1 - r - 128 * k;
            check("E2", 1.0, e);
            check("E2", -1.0, e);
            check("E2", 3.5, e);
        }
    }
}

/// E3 — `0 * inf` invalid operation → x86 "real indefinite" QNaN `0xffc00000`.
#[test]
fn e03_zero_times_infinity() {
    check_exact("E3", f32::from_bits(P_INF), -1, 0xFFC0_0000);
    check_exact("E3", f32::from_bits(N_INF), -2, 0xFFC0_0000);
    check_exact("E3", f32::from_bits(P_INF), -3, 0xFFC0_0000);
    check_exact("E3", f32::from_bits(N_INF), -4, 0xFFC0_0000);
    check_exact("E3", f32::from_bits(P_INF), -129, 0xFFC0_0000);
    for k in 0..8 {
        for r in 0..4 {
            let e = -1 - r - 128 * k;
            check_exact("E3", f32::from_bits(P_INF), e, 0xFFC0_0000);
            check_exact("E3", f32::from_bits(N_INF), e, 0xFFC0_0000);
        }
    }
}

/// E4 — `e & 3` with negative `e` stays in `0..3` (no out-of-bounds table read).
#[test]
fn e04_negative_index_never_out_of_bounds() {
    // If the Rust used `%` or a wrapping cast the index would go OOB and either
    // panic or read garbage. Sweep every negative residue, incl. i32::MIN.
    assert_eq!(-129i32 & 3, 3);
    assert_eq!(-4i32 & 3, 0);
    assert_eq!(i32::MIN & 3, 0);
    for e in -1024..0 {
        check("E4", 1.0, e);
    }
    for k in 0..4 {
        check("E4", 1.0, i32::MIN + k);
        check("E4", 1.0, -2_000_000_000 + k);
    }
}

/// E5 — `INT_MIN`: `exp_q2 -= e` must not overflow-trap; `cnt == 0`.
#[test]
fn e05_int_min() {
    check_exact("E5", 1.0, i32::MIN, 0x3F80_0000);
    check_exact("E5", -1.0, i32::MIN, 0xBF80_0000);
    for y in y_panel() {
        check("E5", y, i32::MIN);
    }
}

/// E6 — `INT_MIN+1 .. INT_MIN+3`: the other residues at the extreme.
#[test]
fn e06_int_min_plus_small() {
    check_exact("E6", 1.0, i32::MIN + 1, 0x3F57_44FD);
    check_exact("E6", 1.0, i32::MIN + 2, 0x3F35_04F3);
    check_exact("E6", 1.0, i32::MIN + 3, 0x3F18_37F0);
    for k in 0..=8 {
        for y in y_panel() {
            check("E6", y, i32::MIN + k);
        }
    }
}

/// E7 — `INT_MAX`: maximum trip count; the loop must terminate exactly.
#[test]
fn e07_int_max_terminates() {
    check_exact("E7", 1.0, i32::MAX, 0x0000_0000);
    check_exact("E7", f32::from_bits(P_INF), i32::MAX, P_INF);
    check_exact("E7", -1.0, i32::MAX, 0x8000_0000);
    check("E7", f32::from_bits(0x7FC0_0001), i32::MAX);
}

/// E8 — `exp_q2 == 0` still runs the body once; `product == 1.0` exactly, so the
/// result is `y` unchanged for every bit pattern (identity, but not by shortcut).
#[test]
fn e08_exp_zero_is_bitwise_identity() {
    let mut rng = Rng::new(SEED ^ 108);
    for y in y_panel() {
        let got = check("E8", y, 0);
        if y.is_nan() {
            // sNaN is quieted by the multiply; qNaN passes through unchanged.
            let quiet = f32::from_bits(y.to_bits() | 0x0040_0000);
            assert_eq!(
                got.to_bits(),
                quiet.to_bits(),
                "[E8] NaN 0x{:08x} -> 0x{:08x}",
                y.to_bits(),
                got.to_bits()
            );
        } else {
            assert_eq!(got.to_bits(), y.to_bits(), "[E8] 0x{:08x} not identity", y.to_bits());
        }
    }
    for _ in 0..20_000 {
        check("E8", rng.any_f32(), 0);
    }
}

/// E9 — there is no `if (exp_q2 <= 0) return y;` guard: negative inputs are
/// still scaled. A translation that adds an early return diverges here.
#[test]
fn e09_no_early_return_for_non_positive_exp() {
    // These would all be 1.0 if an early-return guard existed.
    check_exact("E9", 1.0, -1, 0x0000_0000);
    check_exact("E9", 1.0, -5, 0x3018_37F0);
    check_exact("E9", 1.0, -124, 0x3F00_0000);
    assert_ne!(
        common::harness().c.ldexp_q2(1.0, -1).to_bits(),
        1.0f32.to_bits(),
        "reference C unexpectedly behaves like an early return"
    );
    for e in -256..=0 {
        for y in y_panel() {
            check("E9", y, e);
        }
    }
}

/// E10 — clamp uses `>` not `>=`: 119 / 120 / 121 are three distinct behaviours.
#[test]
fn e10_clamp_boundary_off_by_one() {
    check_exact("E10", 1.0, 119, 0x3098_37F0);
    check_exact("E10", 1.0, 120, 0x3080_0000);
    check_exact("E10", 1.0, 121, 0x3057_44FD);
    for y in y_panel() {
        for e in 115..=125 {
            check("E10", y, e);
        }
    }
}

/// E11 — signalling NaN crossing the FFI boundary is quieted, payload kept.
#[test]
fn e11_signalling_nan_quieted() {
    check_exact("E11", f32::from_bits(0x7F80_0001), 5, 0x7FC0_0001);
    check_exact("E11", f32::from_bits(0xFF80_0001), 5, 0xFFC0_0001);
    check_exact("E11", f32::from_bits(0x7FBF_FFFF), 7, 0x7FFF_FFFF);
    let mut rng = Rng::new(SEED ^ 111);
    for _ in 0..2000 {
        let y = rng.nan_f32(false);
        let e = rng.range_i32(-512, 512);
        check("E11", y, e);
    }
}

/// E12 — quiet NaN payload and sign are preserved (y is the mulss source).
#[test]
fn e12_quiet_nan_payload_preserved() {
    check_exact("E12", f32::from_bits(0x7FC0_0001), 5, 0x7FC0_0001);
    check_exact("E12", f32::from_bits(0xFFC0_DEAD), 7, 0xFFC0_DEAD);
    let mut rng = Rng::new(SEED ^ 112);
    for _ in 0..2000 {
        let y = rng.nan_f32(true);
        let e = rng.range_i32(-512, 512);
        let got = check("E12", y, e);
        // Except for the inf*0 case there is no other NaN source, so the payload
        // must survive verbatim.
        assert_eq!(got.to_bits(), y.to_bits(), "[E12] qNaN payload changed");
    }
}

/// E13 — signed zero input keeps its sign.
#[test]
fn e13_signed_zero() {
    check_exact("E13", 0.0, 3, 0x0000_0000);
    check_exact("E13", -0.0, 3, 0x8000_0000);
    for e in -600..=600 {
        check_exact("E13", 0.0, e, 0x0000_0000);
        check_exact("E13", -0.0, e, 0x8000_0000);
    }
}

/// E14 — gradual underflow / flush-to-zero with round-to-nearest-even ties.
#[test]
fn e14_gradual_underflow() {
    check_exact("E14", f32::from_bits(0x0000_0001), 1, 0x0000_0001);
    check_exact("E14", f32::from_bits(0x0000_0001), 4, 0x0000_0000);
    check_exact("E14", f32::from_bits(0x8000_0001), 4, 0x8000_0000);
    let mut rng = Rng::new(SEED ^ 114);
    for _ in 0..20_000 {
        let y = rng.subnormal_f32();
        let e = rng.range_i32(0, 400);
        check("E14", y, e);
    }
}

/// E15 — overflow is impossible: `product <= 1.0`, so `|result| <= |y|`.
#[test]
fn e15_no_overflow() {
    check_exact("E15", f32::from_bits(0x7F7F_FFFF), -128, 0x7F7F_FFFF);
    check_exact("E15", f32::from_bits(0xFF7F_FFFF), -128, 0xFF7F_FFFF);
    let mut rng = Rng::new(SEED ^ 115);
    for _ in 0..40_000 {
        let y = rng.normal_f32();
        let e = rng.range_i32(i32::MIN, 20_000);
        let got = check("E15", y, e);
        assert!(
            got.abs() <= y.abs(),
            "[E15] |result| grew: ldexp_q2(0x{:08x}, {}) = 0x{:08x}",
            y.to_bits(),
            e,
            got.to_bits()
        );
    }
}

/// E16 — shift count wraps back to 0 → amplification instead of decay.
#[test]
fn e16_shift_count_wraps_to_zero() {
    check_exact("E16", 1.0, -128, 0x3F80_0000);
    check_exact("E16", 1.0, -127, 0x3F57_44FD);
    check_exact("E16", 1.0, -126, 0x3F35_04F3);
    check_exact("E16", 1.0, -125, 0x3F18_37F0);
    check_exact("E16", 1.0, -124, 0x3F00_0000);
    for k in 0..16 {
        for r in 0..4 {
            let e = -128 - 128 * k + r;
            check("E16", 1.0, e);
            check("E16", -3.25, e);
        }
    }
}

/// E17 — generic FFI boundary sweep: every `int` boundary × every float class.
/// (No pointer or enum parameters exist, so this is the full boundary surface.)
#[test]
fn e17_generic_ffi_boundary_sweep() {
    let exps: Vec<i32> = vec![
        i32::MIN,
        i32::MIN + 1,
        -2_147_483_647,
        -1_000_000_001,
        -3,
        -2,
        -1,
        0,
        1,
        2,
        3,
        119,
        120,
        121,
        1_000_000_001,
        i32::MAX - 1,
        i32::MAX,
    ];
    for e in exps {
        for y in y_panel() {
            check("E17", y, e);
        }
    }
}
