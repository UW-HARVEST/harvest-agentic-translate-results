//! Phase C — error / rejection path differential tests.
//!
//! One test per row of `ERRORS.md`. Each constructs the exact invalid input
//! and asserts C and Rust return the SAME sentinel (`0`), not merely "both
//! failed somehow".

mod common;

use common::{c_source_oracle, Pair, Rng};

// ---------------------------------------------------------------------------
// Row 1 — get_predict_func's `default:` arm leaves `result` at 0.
// ---------------------------------------------------------------------------

#[test]
fn err_row1_default_arm_out_of_range() {
    let p = Pair::load();
    // A spread of values that all fall into the `default:` arm.
    for pfcn in [
        -1, -2, -7, -12, -16, -100, -1_000, -65_536, 12, 13, 14, 15, 16, 17, 32, 100, 1_000,
        65_536, 1 << 20, 1 << 30,
    ] {
        p.assert_same_and_eq(pfcn, 0);
    }
}

// ---------------------------------------------------------------------------
// Row 2 — BTAC1C2_GetPredictFunc's `default:` returns the generic fallback
// pointer (NOT NULL). Observable only as `get_predict_func` returning 0,
// because the `default:` arm of get_predict_func performs no comparison.
// ---------------------------------------------------------------------------

#[test]
fn err_row2_fallback_pointer_not_null() {
    let p = Pair::load();
    // If either implementation had returned NULL *and* then compared it
    // against a specialised predictor, we would still see 0 -- so pin the
    // distinguishing property instead: for pfcn in the fallback region the
    // result is 0, while for every in-range pfcn it is 1. Both implementations
    // must agree on that partition exactly.
    for pfcn in -256..=256i32 {
        let expected = if (0..=11).contains(&pfcn) { 1 } else { 0 };
        p.assert_same_and_eq(pfcn, expected);
    }
}

// ---------------------------------------------------------------------------
// Row 3 — BTAC1C2_PredictSample's `default:` (pfcn outside 0..=15) yields 0.
// Unreachable through the exported ABI; assert the observable consequence:
// values in 12..=15 (which DO reach the FIR arms internally) and values
// >= 16 (which reach that `default:`) are indistinguishable at the boundary.
// ---------------------------------------------------------------------------

#[test]
fn err_row3_generic_fallback_unreachable() {
    let p = Pair::load();
    let fir_region: Vec<i32> = (12..=15).map(|v| p.assert_same(v)).collect();
    let past_region: Vec<i32> = (16..=32).map(|v| p.assert_same(v)).collect();
    assert!(
        fir_region.iter().all(|&v| v == 0),
        "FIR region must be indistinguishable from other fallbacks: {fir_region:?}"
    );
    assert!(
        past_region.iter().all(|&v| v == 0),
        "past-all-cases region must return 0: {past_region:?}"
    );
}

// ---------------------------------------------------------------------------
// Rows 4..=6 — one step past the valid range, and past the widest internal
// case range.
// ---------------------------------------------------------------------------

#[test]
fn err_row4_pfcn_12() {
    let p = Pair::load();
    p.assert_same_and_eq(12, 0);
}

#[test]
fn err_row5_pfcn_13_14_15() {
    let p = Pair::load();
    p.assert_same_and_eq(13, 0);
    p.assert_same_and_eq(14, 0);
    p.assert_same_and_eq(15, 0);
}

#[test]
fn err_row6_pfcn_16() {
    let p = Pair::load();
    p.assert_same_and_eq(16, 0);
}

// ---------------------------------------------------------------------------
// Row 7 — one step below the valid range.
// ---------------------------------------------------------------------------

#[test]
fn err_row7_pfcn_minus_1() {
    let p = Pair::load();
    p.assert_same_and_eq(-1, 0);
}

// ---------------------------------------------------------------------------
// Rows 8..=9 — integer extremes. These are the "oversized / out-of-range
// enum value" probes for a bare `int` parameter: C enums accept any int, so
// a tag with no valid variant is a real input.
// ---------------------------------------------------------------------------

#[test]
fn err_row8_int_min() {
    let p = Pair::load();
    p.assert_same_and_eq(i32::MIN, 0);
    p.assert_same_and_eq(i32::MIN + 1, 0);
    // i32::MIN has the property that -x overflows; make sure neither impl
    // negates or abs()es the tag anywhere.
    p.assert_same_and_eq(i32::MIN / 2, 0);
}

#[test]
fn err_row9_int_max() {
    let p = Pair::load();
    p.assert_same_and_eq(i32::MAX, 0);
    p.assert_same_and_eq(i32::MAX - 1, 0);
    p.assert_same_and_eq(i32::MAX / 2, 0);
}

// ---------------------------------------------------------------------------
// Rows 10..=11 — randomized out-of-range tags, negative and positive.
// ---------------------------------------------------------------------------

#[test]
fn err_row10_large_negative_random() {
    let p = Pair::load();
    let mut rng = Rng::new(Rng::SEED ^ 0xC0DE_C0DE);
    for _ in 0..50_000 {
        let pfcn = rng.in_range(i32::MIN, -1);
        p.assert_same_and_eq(pfcn, 0);
    }
}

#[test]
fn err_row11_large_positive_random() {
    let p = Pair::load();
    let mut rng = Rng::new(Rng::SEED ^ 0x5AFE_5AFE);
    for _ in 0..50_000 {
        let pfcn = rng.in_range(12, i32::MAX);
        p.assert_same_and_eq(pfcn, 0);
        // sanity: the oracle agrees this is a rejection
        assert_eq!(c_source_oracle(pfcn), 0);
    }
}

// ---------------------------------------------------------------------------
// Generic boundary probes required regardless of the table.
// ---------------------------------------------------------------------------

/// Every bit pattern that is a power of two, plus its negation, plus
/// off-by-one neighbours — cheap systematic coverage of the whole width.
#[test]
fn err_generic_bitwidth_boundaries() {
    let p = Pair::load();
    for bit in 0..32u32 {
        let v = 1i32.wrapping_shl(bit);
        for cand in [v, v.wrapping_neg(), v.wrapping_sub(1), v.wrapping_add(1)] {
            p.assert_same_and_eq(cand, c_source_oracle(cand));
        }
    }
}

/// Zero is the *lowest valid* tag, not an error — pin that it is not treated
/// as a "null / empty" sentinel by either side.
#[test]
fn err_generic_zero_is_valid_not_sentinel() {
    let p = Pair::load();
    p.assert_same_and_eq(0, 1);
}

/// The upper valid boundary and its immediate neighbours.
#[test]
fn err_generic_valid_range_edges() {
    let p = Pair::load();
    p.assert_same_and_eq(-1, 0);
    p.assert_same_and_eq(0, 1);
    p.assert_same_and_eq(11, 1);
    p.assert_same_and_eq(12, 0);
}
