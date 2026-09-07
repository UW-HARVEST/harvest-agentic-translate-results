//! Phase C — error / rejection-path differential tests, one test per ERRORS.md
//! row.  Every assertion compares the *same* sentinel value returned by both
//! implementations, not merely "both failed".

mod common;

use common::*;

// ===========================================================================
// Rows 1-7 — get_predict_func's out-of-range rejection (returns the sentinel 0).
// ===========================================================================

fn public_pair() -> (Side, Side) {
    pair_public()
}

fn check_public(label: &str, pfcn: i32, expected: i32) {
    let (c, r) = public_pair();
    let cf: libloading::Symbol<GetPredictFunc> = c.sym("get_predict_func");
    let rf: libloading::Symbol<GetPredictFunc> = r.sym("get_predict_func");
    let a = unsafe { cf(pfcn) };
    let b = unsafe { rf(pfcn) };
    assert_eq!(a, b, "{label}: get_predict_func({pfcn}) C={a} Rust={b}");
    assert_eq!(
        a, expected,
        "{label}: C returned {a}, ERRORS.md predicts {expected} for pfcn={pfcn}"
    );
}

/// ERRORS row 1: `pfcn == 12`, the first value past the valid range.
#[test]
fn err01_pfcn_12() {
    check_public("row1", 12, 0);
}

/// ERRORS row 2: `pfcn == 13, 14, 15` — handled by the generic switch but with
/// no specialised predictor, so `get_predict_func` still rejects with 0.
#[test]
fn err02_pfcn_13_14_15() {
    for pfcn in 13..=15 {
        check_public("row2", pfcn, 0);
    }
}

/// ERRORS row 3: `pfcn == 16`, one past every `case` label in the file.
#[test]
fn err03_pfcn_16() {
    check_public("row3", 16, 0);
    check_public("row3", 17, 0);
}

/// ERRORS row 4: `pfcn == -1`, one step below the valid range.
#[test]
fn err04_pfcn_negative_one() {
    check_public("row4", -1, 0);
    check_public("row4", -2, 0);
    check_public("row4", -12, 0);
}

/// ERRORS row 5: `pfcn == INT_MIN`.
#[test]
fn err05_pfcn_int_min() {
    check_public("row5", i32::MIN, 0);
    check_public("row5", i32::MIN + 1, 0);
}

/// ERRORS row 6: `pfcn == INT_MAX`.
#[test]
fn err06_pfcn_int_max() {
    check_public("row6", i32::MAX, 0);
    check_public("row6", i32::MAX - 1, 0);
}

/// ERRORS row 7: property test over the whole `i32` domain — every value
/// outside `0..=11` (including out-of-range "enum" values, which C accepts as
/// plain ints) must be rejected with the same sentinel by both sides.
#[test]
fn err07_pfcn_full_domain_property() {
    let (c, r) = public_pair();
    let cf: libloading::Symbol<GetPredictFunc> = c.sym("get_predict_func");
    let rf: libloading::Symbol<GetPredictFunc> = r.sym("get_predict_func");
    let mut rng = Rng::new(0xC0FFEE);
    for _ in 0..20_000 {
        let pfcn = rng.next_i32();
        let a = unsafe { cf(pfcn) };
        let b = unsafe { rf(pfcn) };
        assert_eq!(a, b, "get_predict_func({pfcn}) C={a} Rust={b}");
        let want = if (0..=11).contains(&pfcn) { 1 } else { 0 };
        assert_eq!(a, want, "C returned {a} for pfcn={pfcn}, expected {want}");
    }
    // Dense sweep of the whole neighbourhood of the valid range.
    for pfcn in -64i32..=64 {
        let a = unsafe { cf(pfcn) };
        let b = unsafe { rf(pfcn) };
        assert_eq!(a, b, "get_predict_func({pfcn}) C={a} Rust={b}");
        let want = if (0..=11).contains(&pfcn) { 1 } else { 0 };
        assert_eq!(a, want);
    }
}

// ===========================================================================
// Row 8 — the dispatcher's fallback: unknown id yields the generic predictor,
// never NULL.
// ===========================================================================

#[test]
fn err08_dispatcher_fallback_is_generic_not_null() {
    if !difftest_enabled() {
        return;
    }
    let (c, r) = pair_internal();
    let cf: libloading::Symbol<DifftestDispatchTag> = c.sym("__difftest_dispatch_tag");
    let rf: libloading::Symbol<DifftestDispatchTag> = r.sym("__difftest_dispatch_tag");
    let mut rng = Rng::new(0xBADF00D);
    let mut cases: Vec<i32> = PFCN_ALL.to_vec();
    for _ in 0..2000 {
        cases.push(rng.next_i32());
    }
    for pfcn in cases {
        let a = unsafe { cf(pfcn) };
        let b = unsafe { rf(pfcn) };
        assert_eq!(a, b, "dispatch tag pfcn={pfcn}: C={a} Rust={b}");
        let want = if (0..=11).contains(&pfcn) { pfcn } else { 12 };
        assert_eq!(a, want, "C dispatch tag for pfcn={pfcn} was {a}, want {want}");
    }
}

// ===========================================================================
// Rows 9-10 — the generic predictor's `default:` arm returns the sentinel 0.
// ===========================================================================

const GENERIC: i32 = 99;

fn generic_default_arm(pfcns: &[i32], seed: u64) {
    let (c, r) = pair_internal();
    let cf: libloading::Symbol<DifftestPredict> = c.sym("__difftest_predict");
    let rf: libloading::Symbol<DifftestPredict> = r.sym("__difftest_predict");
    let mut rng = Rng::new(seed);
    for &pfcn in pfcns {
        for shape in Shape::ALL {
            for _ in 0..40 {
                let psamp = shape.gen(&mut rng);
                let idx = rng.next_i32();
                let mut st = IdxState::default();
                for row in st.firfx.iter_mut() {
                    for cell in row.iter_mut() {
                        *cell = rng.next_u64() as i16;
                    }
                }
                let mut cs = psamp;
                let mut rs = psamp;
                let mut cst = st;
                let mut rst = st;
                let a = unsafe { cf(GENERIC, cs.as_mut_ptr(), idx, pfcn, &mut cst) };
                let b = unsafe { rf(GENERIC, rs.as_mut_ptr(), idx, pfcn, &mut rst) };
                assert_eq!(a, b, "default arm pfcn={pfcn} idx={idx}: C={a} Rust={b}");
                assert_eq!(
                    a, 0,
                    "C default arm must yield the sentinel 0 (pfcn={pfcn}), got {a}"
                );
            }
        }
    }
}

/// ERRORS row 9: `pfcn == 16`, one past `case 15`.
#[test]
fn err09_generic_pfcn_16() {
    if !difftest_enabled() {
        return;
    }
    generic_default_arm(&[16], 0xE001);
}

/// ERRORS row 10: `pfcn` anywhere outside `0..=15`, incl. INT_MIN / INT_MAX.
#[test]
fn err10_generic_pfcn_out_of_range() {
    if !difftest_enabled() {
        return;
    }
    generic_default_arm(
        &[17, 18, 99, 1000, -1, -2, -12, -100, i32::MIN, i32::MIN + 1, i32::MAX, i32::MAX - 1],
        0xE002,
    );
    // Dense sweep of the boundary neighbourhood, verifying the exact 0/non-0
    // split matches between C and Rust.
    let (c, r) = pair_internal();
    let cf: libloading::Symbol<DifftestPredict> = c.sym("__difftest_predict");
    let rf: libloading::Symbol<DifftestPredict> = r.sym("__difftest_predict");
    let mut rng = Rng::new(0xE003);
    for pfcn in -32i32..=32 {
        let psamp = Shape::Small.gen(&mut rng);
        let mut st = IdxState::default();
        for row in st.firfx.iter_mut() {
            for cell in row.iter_mut() {
                *cell = 7;
            }
        }
        for &idx in IDX_BOUNDARIES {
            let mut cs = psamp;
            let mut rs = psamp;
            let mut cst = st;
            let mut rst = st;
            let a = unsafe { cf(GENERIC, cs.as_mut_ptr(), idx, pfcn, &mut cst) };
            let b = unsafe { rf(GENERIC, rs.as_mut_ptr(), idx, pfcn, &mut rst) };
            assert_eq!(a, b, "pfcn={pfcn} idx={idx}: C={a} Rust={b}");
            if !(0..=15).contains(&pfcn) {
                assert_eq!(a, 0, "out-of-range pfcn={pfcn} must give 0");
            }
        }
    }
}

// ===========================================================================
// Rows 11-12 — pointer arguments are NEVER validated by the C.  Discharged by
// asserting the Rust introduces no extra validation (a check the C lacks would
// be a divergence), and that with a *valid* pointer both sides agree.
// ===========================================================================

/// ERRORS row 11: `pfcn` in 12..=15 dereferences `ridx` unconditionally.  The C
/// has no null check; assert the Rust source has none either (no `is_null`
/// guard on the FIR path) and that a valid `ridx` gives identical results.
#[test]
fn err11_ridx_never_validated() {
    let src = std::fs::read_to_string(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs"),
    )
    .expect("read src/lib.rs");
    assert!(
        !src.contains("ridx.is_null()"),
        "Rust added a NULL check on ridx that the C does not have"
    );
    assert!(
        !src.contains("unimplemented!") && !src.contains("todo!"),
        "Rust contains a stub"
    );
    if !difftest_enabled() {
        return;
    }
    // ...and a valid ridx must behave identically (covers the reachable half).
    let (c, r) = pair_internal();
    let cf: libloading::Symbol<DifftestPredict> = c.sym("__difftest_predict");
    let rf: libloading::Symbol<DifftestPredict> = r.sym("__difftest_predict");
    let mut rng = Rng::new(0xE011);
    for pfcn in 12..=15 {
        for _ in 0..200 {
            let psamp = Shape::Medium.gen(&mut rng);
            let idx = rng.next_i32();
            let mut st = IdxState::default();
            for row in st.firfx.iter_mut() {
                for cell in row.iter_mut() {
                    *cell = rng.next_u64() as i16;
                }
            }
            let mut cs = psamp;
            let mut rs = psamp;
            let mut cst = st;
            let mut rst = st;
            let a = unsafe { cf(GENERIC, cs.as_mut_ptr(), idx, pfcn, &mut cst) };
            let b = unsafe { rf(GENERIC, rs.as_mut_ptr(), idx, pfcn, &mut rst) };
            assert_eq!(a, b, "pfcn={pfcn} idx={idx}: C={a} Rust={b}");
        }
    }
}

/// ERRORS row 12: `psamp` is dereferenced with no null check anywhere.  Same
/// treatment: assert the Rust adds no guard the C lacks.
#[test]
fn err12_psamp_never_validated() {
    let src = std::fs::read_to_string(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs"),
    )
    .expect("read src/lib.rs");
    assert!(
        !src.contains("psamp.is_null()"),
        "Rust added a NULL check on psamp that the C does not have"
    );
    assert!(
        !src.contains("assert!") && !src.contains("panic!"),
        "Rust added a panic-based rejection the C does not have"
    );
}

// ===========================================================================
// Row 13 — `idx` is never rejected: `(idx - k) & 7` masks every value, and
// INT_MIN - 8 signed-overflow-wraps.  No bounds check exists on either side.
// ===========================================================================

#[test]
fn err13_idx_never_rejected() {
    if !difftest_enabled() {
        return;
    }
    let (c, r) = pair_internal();
    let cf: libloading::Symbol<DifftestPredict> = c.sym("__difftest_predict");
    let rf: libloading::Symbol<DifftestPredict> = r.sym("__difftest_predict");
    // psamp values 0..7 make the *identity* of the chosen slot visible in the
    // result, so an out-of-range or mis-masked index cannot hide.
    let psamp: [i32; 8] = [10, 20, 30, 40, 50, 60, 70, 80];
    let mut st = IdxState::default();
    for row in st.firfx.iter_mut() {
        for (k, cell) in row.iter_mut().enumerate() {
            *cell = (k as i16 + 1) * 32;
        }
    }
    let mut idxs: Vec<i32> = IDX_BOUNDARIES.to_vec();
    let mut rng = Rng::new(0xE013);
    for _ in 0..3000 {
        idxs.push(rng.next_i32());
    }
    for which in [0i32, 1, 5, 7, 9, 11, GENERIC] {
        for pfcn in [0i32, 7, 10, 11, 12, 15] {
            for &idx in &idxs {
                let mut cs = psamp;
                let mut rs = psamp;
                let mut cst = st;
                let mut rst = st;
                let a = unsafe { cf(which, cs.as_mut_ptr(), idx, pfcn, &mut cst) };
                let b = unsafe { rf(which, rs.as_mut_ptr(), idx, pfcn, &mut rst) };
                assert_eq!(
                    a, b,
                    "which={which} pfcn={pfcn} idx={idx}: C={a} Rust={b}"
                );
            }
        }
    }
}

// ===========================================================================
// Row 14 — the only divisors are the non-zero literals 16 / 64 / 256, so no
// division trap is reachable.  Verified by driving those arms over the whole
// value domain (incl. INT_MIN dividends) and checking both sides truncate
// toward zero identically.
// ===========================================================================

#[test]
fn err14_division_arms_cannot_trap() {
    if !difftest_enabled() {
        return;
    }
    let (c, r) = pair_internal();
    let cf: libloading::Symbol<DifftestPredict> = c.sym("__difftest_predict");
    let rf: libloading::Symbol<DifftestPredict> = r.sym("__difftest_predict");
    let mut rng = Rng::new(0xE014);
    // which = 7/8/9 (specialised) and pfcn = 7/8/9/12..15 (generic) are the
    // only `/` sites in lib.c.
    for shape in [Shape::Extreme, Shape::Negative, Shape::Positive, Shape::Zero] {
        for _ in 0..300 {
            let psamp = shape.gen(&mut rng);
            let idx = rng.next_i32();
            let mut st = IdxState::default();
            for row in st.firfx.iter_mut() {
                for cell in row.iter_mut() {
                    *cell = match rng.below(4) {
                        0 => i16::MIN,
                        1 => i16::MAX,
                        _ => rng.next_u64() as i16,
                    };
                }
            }
            for (which, pfcn) in [
                (7i32, 0i32),
                (8, 0),
                (9, 0),
                (GENERIC, 7),
                (GENERIC, 8),
                (GENERIC, 9),
                (GENERIC, 12),
                (GENERIC, 13),
                (GENERIC, 14),
                (GENERIC, 15),
            ] {
                let mut cs = psamp;
                let mut rs = psamp;
                let mut cst = st;
                let mut rst = st;
                let a = unsafe { cf(which, cs.as_mut_ptr(), idx, pfcn, &mut cst) };
                let b = unsafe { rf(which, rs.as_mut_ptr(), idx, pfcn, &mut rst) };
                assert_eq!(
                    a, b,
                    "div arm which={which} pfcn={pfcn} idx={idx} shape={shape:?} \
                     psamp={psamp:?}: C={a} Rust={b}"
                );
            }
        }
    }
}
