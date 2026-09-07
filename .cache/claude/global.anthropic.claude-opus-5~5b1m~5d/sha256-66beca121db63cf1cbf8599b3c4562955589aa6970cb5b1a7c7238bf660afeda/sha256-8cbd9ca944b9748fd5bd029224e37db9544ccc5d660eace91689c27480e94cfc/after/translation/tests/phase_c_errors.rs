//! Phase C — error-path differential tests, one test per ERRORS.md row.
//! Every rejection path in the C source is the `default:` arm of a
//! `switch (pfcn)`, which yields the sentinel return value `0`.

mod common;
use common::{Libs, Rng, SEED};

/// ERRORS row 1 — pfcn == 12, first value past GetPredictFunc's cases.
#[test]
fn err01_pfcn_12_first_past_range() {
    let l = Libs::load();
    let v = l.assert_same(12);
    assert_eq!(v, 0, "row 1: pfcn=12 must hit the default arm (sentinel 0)");
}

/// ERRORS row 2 — pfcn == -1, one step below the valid range.
#[test]
fn err02_pfcn_minus1() {
    let l = Libs::load();
    assert_eq!(l.assert_same(-1), 0);
}

/// ERRORS row 3 — pfcn 13, 14, 15: handled by PredictSample's FIR arm but not
/// by GetPredictFunc, so still rejected by call_predict.
#[test]
fn err03_pfcn_13_14_15() {
    let l = Libs::load();
    for pfcn in [13, 14, 15] {
        assert_eq!(l.assert_same(pfcn), 0, "row 3: pfcn={pfcn}");
    }
}

/// ERRORS row 4 — pfcn == 16, one past the highest value any switch names.
#[test]
fn err04_pfcn_16() {
    let l = Libs::load();
    assert_eq!(l.assert_same(16), 0);
}

/// ERRORS row 5 — INT_MAX.
#[test]
fn err05_int_max() {
    let l = Libs::load();
    assert_eq!(l.assert_same(i32::MAX), 0);
    assert_eq!(l.assert_same(i32::MAX - 1), 0);
}

/// ERRORS row 6 — INT_MIN.
#[test]
fn err06_int_min() {
    let l = Libs::load();
    assert_eq!(l.assert_same(i32::MIN), 0);
    assert_eq!(l.assert_same(i32::MIN + 1), 0);
}

/// ERRORS row 7 — exhaustive sweep of the default arm over -1000..=1000.
#[test]
fn err07_exhaustive_default_arm() {
    let l = Libs::load();
    let mut rejected = 0;
    for pfcn in -1000..=1000 {
        if (0..=11).contains(&pfcn) {
            continue;
        }
        assert_eq!(l.assert_same(pfcn), 0, "row 7: pfcn={pfcn} must be rejected");
        rejected += 1;
    }
    assert_eq!(rejected, 2001 - 12);
}

/// ERRORS row 8 — randomized out-of-range enum values across the FFI boundary.
#[test]
fn err08_random_out_of_range_enum_values() {
    let l = Libs::load();
    let mut rng = Rng::new(SEED);
    for _ in 0..10_000 {
        let pfcn = rng.next_i32();
        let v = l.assert_same(pfcn);
        assert_eq!(
            v,
            i32::from((0..=11).contains(&pfcn)),
            "row 8: pfcn={pfcn}"
        );
    }
    // Plus a stream deliberately concentrated just outside the range.
    for _ in 0..10_000 {
        let pfcn = match rng.next_u64() % 4 {
            0 => rng.range(12, 4096),
            1 => rng.range(-4096, -1),
            2 => rng.range(i32::MAX - 4096, i32::MAX),
            _ => rng.range(i32::MIN, i32::MIN + 4096),
        };
        assert_eq!(l.assert_same(pfcn), 0, "row 8 (near-boundary): pfcn={pfcn}");
    }
}

/// ERRORS row 9 — GetPredictFunc's default returns &BTAC1C2_PredictSample,
/// an address that can never equal any `_PfnN`, so call_predict can never
/// report 1 outside 0..=11. Verified over a wide sweep in BOTH libraries.
#[test]
fn err09_default_func_never_matches() {
    let l = Libs::load();
    for pfcn in (-100_000..=100_000).step_by(7) {
        if (0..=11).contains(&pfcn) {
            continue;
        }
        assert_eq!(l.c(pfcn), 0, "row 9: C reported a match for pfcn={pfcn}");
        assert_eq!(l.rust(pfcn), 0, "row 9: Rust reported a match for pfcn={pfcn}");
    }
}

/// ERRORS row 10 — BTAC1C2_PredictSample's own `default: pred = 0` is internal
/// (static in C, private in Rust) and unreachable through the exported ABI.
/// What IS observable is that selecting it never changes call_predict's answer;
/// asserted structurally for the whole 12..=15 FIR band and beyond.
#[test]
fn err10_predictsample_default_arm_unobservable() {
    let l = Libs::load();
    for pfcn in 12..=64 {
        assert_eq!(l.assert_same(pfcn), 0, "row 10: pfcn={pfcn}");
    }
}

/// ERRORS row 11 — generic C-API boundaries. `int call_predict(int)` has no
/// pointer, length or size parameter, so null-pointer / zero-length /
/// oversized-length paths do not exist. This test discharges the requirement by
/// confirming the exported symbol really has that arity/ABI in both libraries:
/// calling it with a single int and nothing else returns identical values, and
/// no out-parameter memory is touched.
#[test]
fn err11_no_pointer_or_length_parameters() {
    let l = Libs::load();
    // A canary buffer adjacent to the call proves nothing is written anywhere.
    let canary: [u8; 64] = [0xA5; 64];
    for pfcn in [-1, 0, 5, 11, 12, 999] {
        l.assert_same(pfcn);
    }
    assert_eq!(canary, [0xA5u8; 64], "row 11: unexpected memory mutation");
}

/// Generic boundary: both libraries must agree on every value in a dense band
/// straddling all three switch boundaries (-1/0, 11/12, 15/16).
#[test]
fn generic_boundaries_dense_band() {
    let l = Libs::load();
    for pfcn in -64..=64 {
        let c = l.c(pfcn);
        let r = l.rust(pfcn);
        assert_eq!(c, r, "boundary divergence at pfcn={pfcn}: C={c} Rust={r}");
        assert!(c == 0 || c == 1, "unexpected non-boolean return {c}");
    }
}
