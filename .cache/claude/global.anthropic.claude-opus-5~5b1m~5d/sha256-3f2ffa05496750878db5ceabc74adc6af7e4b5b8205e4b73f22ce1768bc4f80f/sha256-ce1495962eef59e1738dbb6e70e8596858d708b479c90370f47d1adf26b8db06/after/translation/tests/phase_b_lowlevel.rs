//! Phase B — valid-path differential tests for the **lowest-level** entry
//! points (`CONFIGS.md` rows C6..C36, C38).
//!
//! `BTAC1C2_PredictSample`, the twelve `BTAC1C2_PredictSample_PfnN` helpers and
//! `BTAC1C2_GetPredictFunc` are all `static` in the C, so they are reached
//! through the `__difftest_predict` dispatcher that both sides export under the
//! `difftest` feature.  Both sides are still loaded with `libloading` and called
//! only through `dlsym`.
//!
//! Requires `--features difftest`.

#![cfg(feature = "difftest")]

mod common;
use common::*;

// ---------------------------------------------------------------------------
// input shape generators
// ---------------------------------------------------------------------------

/// The eight `idx` rotations plus large/negative/extreme values.  `(idx - k) & 7`
/// means only `idx & 7` selects the window rotation, but the extremes also
/// exercise the signed overflow of `idx - k`.
fn idx_shapes() -> Vec<i32> {
    let mut v: Vec<i32> = (0..8).collect();
    v.extend_from_slice(&[
        8, 9, 15, 16, 1000, 1001, 1007, -1, -2, -7, -8, -9, -1000, -1007,
        i32::MAX,
        i32::MAX - 1,
        i32::MAX - 7,
        i32::MIN,
        i32::MIN + 1,
        i32::MIN + 7,
    ]);
    v
}

fn psamp_shapes() -> Vec<[i32; 8]> {
    vec![
        [0; 8],
        [1; 8],
        [-1; 8],
        [7; 8],
        [-7; 8],
        [0, 1, 2, 3, 4, 5, 6, 7],
        [7, 6, 5, 4, 3, 2, 1, 0],
        [-4, 3, -2, 1, 0, -1, 2, -3],
        [i32::MAX; 8],
        [i32::MIN; 8],
        [
            i32::MAX,
            i32::MIN,
            i32::MAX,
            i32::MIN,
            i32::MAX,
            i32::MIN,
            i32::MAX,
            i32::MIN,
        ],
        [
            i32::MIN,
            i32::MAX,
            i32::MIN,
            i32::MAX,
            i32::MIN,
            i32::MAX,
            i32::MIN,
            i32::MAX,
        ],
        [i32::MAX / 2; 8],
        [i32::MIN / 2; 8],
        [-1, -2, -3, -4, -5, -6, -7, -8],
        [1000000, -1000000, 999999, -999999, 3, -3, 0, 12345],
    ]
}

fn firfx_shapes() -> Vec<[[i16; 8]; 4]> {
    let mut zeros = [[0i16; 8]; 4];
    let mut identity = [[0i16; 8]; 4];
    for r in 0..4 {
        identity[r][0] = 256;
    }
    // Distinct per row, so a wrong `pfcn - 12` row index is detectable.
    let mut per_row = [[0i16; 8]; 4];
    for r in 0..4 {
        for c in 0..8 {
            per_row[r][c] = ((r as i16 + 1) * 1000) + c as i16;
        }
    }
    // Distinct per column too.
    let mut ramp = [[0i16; 8]; 4];
    for r in 0..4 {
        for c in 0..8 {
            ramp[r][c] = (r * 8 + c) as i16 - 16;
        }
    }
    zeros[0][0] = 0; // explicit
    vec![
        zeros,
        identity,
        per_row,
        ramp,
        [[i16::MAX; 8]; 4],
        [[i16::MIN; 8]; 4],
        [[256, -256, 128, -128, 64, -64, 32, -32]; 4],
    ]
}

fn st_with(firfx: [[i16; 8]; 4]) -> IdxState {
    let mut st = IdxState::zeroed();
    // Non-zero values in the neighbouring fields: if the Rust struct layout
    // differed from C's, `firfx` reads would pick these up and diverge.
    st.idx = 0xBEEF;
    st.lpred = -12345;
    st.rpred = 23456;
    st.tag = 0xA5;
    st.bcfcn = 0x5A;
    st.bsfcn = 0x3C;
    st.usefx = 0xC3;
    st.firfx = firfx;
    st
}

// ---------------------------------------------------------------------------
// struct layout parity (prerequisite for the firfx rows)
// ---------------------------------------------------------------------------

#[test]
fn struct_layout_matches() {
    let c = c_difftest_layout();
    let r = rust_difftest_layout();
    let names = [
        "sizeof", "off idx", "off lpred", "off rpred", "off tag", "off bcfcn",
        "off bsfcn", "off usefx", "off firfx", "sizeof firfx", "alignof",
    ];
    for (i, name) in names.iter().enumerate() {
        let cv = unsafe { c(i as i32) };
        let rv = unsafe { r(i as i32) };
        assert_eq!(cv, rv, "btac1c_idxstate layout mismatch: {name}");
    }
    // Out-of-range selector must agree too.
    for w in [-1, 11, 12, 1000, i32::MIN, i32::MAX] {
        assert_eq!(unsafe { c(w) }, unsafe { r(w) }, "layout selector {w}");
    }
}

// ---------------------------------------------------------------------------
// C6..C7 — pfcn 0 through the generic dispatcher
// ---------------------------------------------------------------------------

#[test]
fn c6_generic_pfcn0_zeros() {
    let st = st_with([[0i16; 8]; 4]);
    for idx in idx_shapes() {
        assert_predict_eq(GENERIC, &[0; 8], idx, 0, &st, "C6");
    }
}

#[test]
fn c7_generic_pfcn0_constant() {
    let st = st_with([[0i16; 8]; 4]);
    for &k in &[1i32, -1, 12345, -12345, i32::MAX, i32::MIN] {
        let ps = [k; 8];
        for idx in idx_shapes() {
            assert_predict_eq(GENERIC, &ps, idx, 0, &st, "C7");
        }
    }
}

// ---------------------------------------------------------------------------
// C8..C12, C17..C19 — the generic dispatcher, every arm x every shape
// ---------------------------------------------------------------------------

/// Drives `BTAC1C2_PredictSample` over a `pfcn` set x all `idx` shapes x all
/// hand-picked `psamp` shapes x all `firfx` shapes, plus a seeded random sweep.
fn sweep_generic(pfcns: &[i32], ctx: &str, seed: u64) {
    for &pfcn in pfcns {
        for firfx in firfx_shapes() {
            let st = st_with(firfx);
            for ps in psamp_shapes() {
                for idx in idx_shapes() {
                    assert_predict_eq(GENERIC, &ps, idx, pfcn, &st, ctx);
                }
            }
        }
    }
    let mut rng = Rng::new(seed);
    for _ in 0..4_000 {
        let pfcn = pfcns[(rng.next_u64() as usize) % pfcns.len()];
        let mut ps = [0i32; 8];
        for v in ps.iter_mut() {
            *v = rng.i32_shaped();
        }
        let mut firfx = [[0i16; 8]; 4];
        for row in firfx.iter_mut() {
            for v in row.iter_mut() {
                *v = rng.i16_shaped();
            }
        }
        let st = st_with(firfx);
        let idx = if rng.next_u64() % 2 == 0 {
            rng.i32_in(-64, 64)
        } else {
            rng.i32_any()
        };
        assert_predict_eq(GENERIC, &ps, idx, pfcn, &st, ctx);
    }
}

#[test]
fn c8_generic_two_tap_family() {
    sweep_generic(&[1, 2, 3], "C8", 0xC8);
}

#[test]
fn c9_generic_paired_sum_family() {
    sweep_generic(&[4, 5, 6], "C9", 0xC9);
}

#[test]
fn c10_generic_five_tap_div16() {
    sweep_generic(&[7], "C10", 0xC10);
}

#[test]
fn c11_generic_eight_tap_div64() {
    sweep_generic(&[8, 9], "C11", 0xC11);
}

#[test]
fn c12_generic_block_sums() {
    sweep_generic(&[10, 11], "C12", 0xC12);
}

// ---------------------------------------------------------------------------
// C13..C16 — the firfx (data-driven FIR) arms, pfcn 12..=15
// ---------------------------------------------------------------------------

#[test]
fn c13_firfx_all_zero() {
    let st = st_with([[0i16; 8]; 4]);
    for pfcn in 12..=15 {
        for ps in psamp_shapes() {
            for idx in idx_shapes() {
                let v = assert_predict_eq(GENERIC, &ps, idx, pfcn, &st, "C13");
                assert_eq!(v, 0, "C13: all-zero firfx must give 0");
            }
        }
    }
}

#[test]
fn c14_firfx_distinct_per_row() {
    // Rows are distinct, so a wrong `pfcn - 12` index shows up as a mismatch.
    let mut firfx = [[0i16; 8]; 4];
    for r in 0..4 {
        for c in 0..8 {
            firfx[r][c] = ((r as i16 + 1) * 300) + (c as i16 * 7) - 100;
        }
    }
    let st = st_with(firfx);
    let mut seen = std::collections::HashSet::new();
    for pfcn in 12..=15 {
        for ps in psamp_shapes() {
            for idx in idx_shapes() {
                assert_predict_eq(GENERIC, &ps, idx, pfcn, &st, "C14");
            }
        }
        // Row selection is observable: a ramp input yields a row-specific value.
        let ps = [1, 2, 3, 4, 5, 6, 7, 8];
        let v = assert_predict_eq(GENERIC, &ps, 0, pfcn, &st, "C14 row-id");
        seen.insert(v);
    }
    assert_eq!(seen.len(), 4, "C14: the four firfx rows must be distinguishable");
}

#[test]
fn c15_firfx_randomized() {
    let mut rng = Rng::new(0xC15);
    for _ in 0..6_000 {
        let pfcn = 12 + (rng.next_u64() % 4) as i32;
        let mut ps = [0i32; 8];
        for v in ps.iter_mut() {
            *v = rng.i32_shaped();
        }
        let mut firfx = [[0i16; 8]; 4];
        for row in firfx.iter_mut() {
            for v in row.iter_mut() {
                *v = rng.i16_any();
            }
        }
        let st = st_with(firfx);
        let idx = rng.i32_any();
        assert_predict_eq(GENERIC, &ps, idx, pfcn, &st, "C15");
    }
}

#[test]
fn c16_firfx_saturated_overflow() {
    for firfx in [[[i16::MAX; 8]; 4], [[i16::MIN; 8]; 4]] {
        let st = st_with(firfx);
        for pfcn in 12..=15 {
            for ps in [
                [i32::MAX; 8],
                [i32::MIN; 8],
                [i32::MAX, i32::MIN, i32::MAX, i32::MIN, i32::MAX, i32::MIN, i32::MAX, i32::MIN],
                [i32::MAX / 3; 8],
            ] {
                for idx in idx_shapes() {
                    assert_predict_eq(GENERIC, &ps, idx, pfcn, &st, "C16");
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C17..C19
// ---------------------------------------------------------------------------

#[test]
fn c17_generic_default_arm() {
    let st = st_with([[i16::MAX; 8]; 4]);
    let mut pfcns: Vec<i32> = vec![-1, -2, -12, -13, 16, 17, 100, i32::MIN, i32::MAX];
    let mut rng = Rng::new(0xC17);
    for _ in 0..500 {
        let p = rng.i32_any();
        if !(0..=15).contains(&p) {
            pfcns.push(p);
        }
    }
    for &pfcn in &pfcns {
        for ps in psamp_shapes() {
            for idx in [0, 3, 7, -1, i32::MIN, i32::MAX] {
                let v = assert_predict_eq(GENERIC, &ps, idx, pfcn, &st, "C17");
                assert_eq!(v, 0, "C17: default arm must return 0 (pfcn={pfcn})");
            }
        }
    }
}

#[test]
fn c18_generic_all_arms_extreme_samples() {
    let st = st_with([[i16::MIN; 8]; 4]);
    let extremes = [
        [i32::MAX; 8],
        [i32::MIN; 8],
        [i32::MAX, i32::MIN, i32::MAX, i32::MIN, i32::MAX, i32::MIN, i32::MAX, i32::MIN],
        [i32::MIN + 1; 8],
        [i32::MAX - 1; 8],
    ];
    for pfcn in 0..=15 {
        for ps in extremes {
            for idx in idx_shapes() {
                assert_predict_eq(GENERIC, &ps, idx, pfcn, &st, "C18");
            }
        }
    }
}

#[test]
fn c19_generic_all_arms_extreme_idx() {
    let st = st_with([[123i16; 8]; 4]);
    let ps = [3, -7, 11, -13, 17, -19, 23, -29];
    let mut idxs: Vec<i32> = vec![
        i32::MIN,
        i32::MIN + 1,
        i32::MIN + 8,
        i32::MAX,
        i32::MAX - 1,
        i32::MAX - 8,
    ];
    let mut rng = Rng::new(0xC19);
    for _ in 0..400 {
        idxs.push(rng.i32_in(i32::MIN, -1));
    }
    for pfcn in 0..=15 {
        for &idx in &idxs {
            assert_predict_eq(GENERIC, &ps, idx, pfcn, &st, "C19");
        }
    }
}

// ---------------------------------------------------------------------------
// C20..C31 — each specialised predictor driven directly
// ---------------------------------------------------------------------------

fn sweep_specialised(which: i32, ctx: &str, seed: u64) {
    // The specialised predictors ignore `pfcn` and `ridx`; vary them anyway.
    let sts = [st_with([[0i16; 8]; 4]), st_with([[i16::MAX; 8]; 4])];
    for st in &sts {
        for ps in psamp_shapes() {
            for idx in idx_shapes() {
                for pfcn in [0, which, 15, -1, i32::MAX] {
                    assert_predict_eq(which, &ps, idx, pfcn, st, ctx);
                }
            }
        }
    }
    let mut rng = Rng::new(seed);
    for _ in 0..3_000 {
        let mut ps = [0i32; 8];
        for v in ps.iter_mut() {
            *v = rng.i32_shaped();
        }
        let mut firfx = [[0i16; 8]; 4];
        for row in firfx.iter_mut() {
            for v in row.iter_mut() {
                *v = rng.i16_shaped();
            }
        }
        let st = st_with(firfx);
        let idx = if rng.next_u64() % 2 == 0 {
            rng.i32_in(-64, 64)
        } else {
            rng.i32_any()
        };
        let pfcn = rng.i32_any();
        assert_predict_eq(which, &ps, idx, pfcn, &st, ctx);
    }
}

macro_rules! specialised_test {
    ($name:ident, $which:expr, $ctx:expr, $seed:expr) => {
        #[test]
        fn $name() {
            sweep_specialised($which, $ctx, $seed);
        }
    };
}

specialised_test!(c20_pfn0, 0, "C20 Pfn0", 0x2000);
specialised_test!(c21_pfn1, 1, "C21 Pfn1", 0x2001);
specialised_test!(c22_pfn2, 2, "C22 Pfn2", 0x2002);
specialised_test!(c23_pfn3, 3, "C23 Pfn3", 0x2003);
specialised_test!(c24_pfn4, 4, "C24 Pfn4", 0x2004);
specialised_test!(c25_pfn5, 5, "C25 Pfn5", 0x2005);
specialised_test!(c26_pfn6, 6, "C26 Pfn6", 0x2006);
specialised_test!(c27_pfn7, 7, "C27 Pfn7", 0x2007);
specialised_test!(c28_pfn8, 8, "C28 Pfn8", 0x2008);
specialised_test!(c29_pfn9, 9, "C29 Pfn9", 0x2009);
specialised_test!(c30_pfn10, 10, "C30 Pfn10", 0x200A);
specialised_test!(c31_pfn11, 11, "C31 Pfn11", 0x200B);

// ---------------------------------------------------------------------------
// C32..C33 — the deliberate shift-amount quirks must be preserved, not "fixed"
// ---------------------------------------------------------------------------

/// `_Pfn10` uses `>> 3` while `case 10` of the generic dispatcher uses `>> 4`.
/// Both must be reproduced, so the two entry points must *disagree* on the Rust
/// side exactly where they disagree on the C side.
#[test]
fn c32_pfn10_disagrees_with_case10() {
    let st = st_with([[0i16; 8]; 4]);
    let mut differ = 0usize;
    let mut rng = Rng::new(0xC32);
    for _ in 0..3_000 {
        let mut ps = [0i32; 8];
        for v in ps.iter_mut() {
            *v = rng.i32_in(-100_000, 100_000);
        }
        let idx = rng.i32_in(-64, 64);
        let (c_spec, r_spec) = diff_predict(10, &ps, idx, 10, &st);
        let (c_gen, r_gen) = diff_predict(GENERIC, &ps, idx, 10, &st);
        assert_eq!(c_spec, r_spec, "C32 Pfn10 mismatch ps={ps:?} idx={idx}");
        assert_eq!(c_gen, r_gen, "C32 case10 mismatch ps={ps:?} idx={idx}");
        assert_eq!(
            c_spec == c_gen,
            r_spec == r_gen,
            "C32: agreement pattern differs (C {c_spec}/{c_gen}, Rust {r_spec}/{r_gen})"
        );
        if c_spec != c_gen {
            differ += 1;
        }
    }
    assert!(
        differ > 100,
        "C32: expected the >>3 / >>4 quirk to be observable, saw {differ} diffs"
    );
}

/// `_Pfn11` uses `>> 1` while `case 11` uses `>> 3`.
#[test]
fn c33_pfn11_disagrees_with_case11() {
    let st = st_with([[0i16; 8]; 4]);
    let mut differ = 0usize;
    let mut rng = Rng::new(0xC33);
    for _ in 0..3_000 {
        let mut ps = [0i32; 8];
        for v in ps.iter_mut() {
            *v = rng.i32_in(-100_000, 100_000);
        }
        let idx = rng.i32_in(-64, 64);
        let (c_spec, r_spec) = diff_predict(11, &ps, idx, 11, &st);
        let (c_gen, r_gen) = diff_predict(GENERIC, &ps, idx, 11, &st);
        assert_eq!(c_spec, r_spec, "C33 Pfn11 mismatch ps={ps:?} idx={idx}");
        assert_eq!(c_gen, r_gen, "C33 case11 mismatch ps={ps:?} idx={idx}");
        assert_eq!(
            c_spec == c_gen,
            r_spec == r_gen,
            "C33: agreement pattern differs (C {c_spec}/{c_gen}, Rust {r_spec}/{r_gen})"
        );
        if c_spec != c_gen {
            differ += 1;
        }
    }
    assert!(
        differ > 100,
        "C33: expected the >>1 / >>3 quirk to be observable, saw {differ} diffs"
    );
}

/// The other ten specialised predictors *do* mirror their generic counterparts;
/// verify that equivalence holds identically on both sides.
#[test]
fn specialised_vs_generic_agreement_pattern() {
    let st = st_with([[0i16; 8]; 4]);
    let mut rng = Rng::new(0xAB1D);
    for _ in 0..4_000 {
        let mut ps = [0i32; 8];
        for v in ps.iter_mut() {
            *v = rng.i32_shaped();
        }
        let idx = rng.i32_any();
        for which in 0..=11 {
            let (c_spec, r_spec) = diff_predict(which, &ps, idx, which, &st);
            let (c_gen, r_gen) = diff_predict(GENERIC, &ps, idx, which, &st);
            assert_eq!(c_spec, r_spec, "spec {which} mismatch");
            assert_eq!(c_gen, r_gen, "generic {which} mismatch");
            assert_eq!(
                c_spec == c_gen,
                r_spec == r_gen,
                "agreement pattern differs for which={which} ps={ps:?} idx={idx}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// C34..C35
// ---------------------------------------------------------------------------

/// C34 — the `pfcn` parameter is unused by every specialised predictor, so the
/// result must not depend on it, on either side.
#[test]
fn c34_pfcn_parameter_unused_by_specialised() {
    let st = st_with([[i16::MAX; 8]; 4]);
    let mut rng = Rng::new(0xC34);
    for _ in 0..800 {
        let mut ps = [0i32; 8];
        for v in ps.iter_mut() {
            *v = rng.i32_shaped();
        }
        let idx = rng.i32_any();
        for which in 0..=11 {
            let base = assert_predict_eq(which, &ps, idx, 0, &st, "C34 base");
            for pfcn in [1, 7, 11, 12, 15, 16, -1, i32::MIN, i32::MAX] {
                let v = assert_predict_eq(which, &ps, idx, pfcn, &st, "C34");
                assert_eq!(v, base, "C34: which={which} varied with pfcn={pfcn}");
            }
        }
    }
}

/// C35 — the specialised predictors never dereference `ridx`, so a NULL `ridx`
/// is safe and must give the same answer on both sides.
#[test]
fn c35_null_ridx_specialised() {
    let c = c_difftest_predict();
    let r = rust_difftest_predict();
    let st = st_with([[7i16; 8]; 4]);
    let mut rng = Rng::new(0xC35);
    for _ in 0..500 {
        let mut ps = [0i32; 8];
        for v in ps.iter_mut() {
            *v = rng.i32_shaped();
        }
        let idx = rng.i32_any();
        for which in 0..=11 {
            let mut cs = ps;
            let mut rs = ps;
            let cv = unsafe {
                c(which, cs.as_mut_ptr(), idx, which, std::ptr::null_mut())
            };
            let rv = unsafe {
                r(which, rs.as_mut_ptr(), idx, which, std::ptr::null_mut())
            };
            assert_eq!(cv, rv, "C35: which={which} idx={idx} ps={ps:?}");
            // Same as the non-null case.
            let with_st = assert_predict_eq(which, &ps, idx, which, &st, "C35 ref");
            assert_eq!(cv, with_st, "C35: NULL ridx changed the result");
        }
    }
}

// ---------------------------------------------------------------------------
// C36 — the big randomized property sweep over every axis at once
// ---------------------------------------------------------------------------

#[test]
fn c36_full_random_property_sweep() {
    let mut rng = Rng::new(0xC36_5EED);
    for _ in 0..50_000 {
        // which: cover the 12 specialised entry points and the generic one.
        let which = rng.i32_in(-3, 14);
        let pfcn = match rng.next_u64() % 3 {
            0 => rng.i32_in(-32, 32),
            1 => rng.i32_in(0, 15),
            _ => rng.i32_any(),
        };
        let idx = match rng.next_u64() % 3 {
            0 => rng.i32_in(0, 7),
            1 => rng.i32_in(-64, 64),
            _ => rng.i32_any(),
        };
        let mut ps = [0i32; 8];
        for v in ps.iter_mut() {
            *v = rng.i32_shaped();
        }
        let mut firfx = [[0i16; 8]; 4];
        for row in firfx.iter_mut() {
            for v in row.iter_mut() {
                *v = rng.i16_shaped();
            }
        }
        let mut st = st_with(firfx);
        st.idx = rng.next_u32() as u16;
        st.lpred = rng.i16_any();
        st.rpred = rng.i16_any();
        st.tag = rng.next_u32() as u8;
        st.bcfcn = rng.next_u32() as u8;
        st.bsfcn = rng.next_u32() as u8;
        st.usefx = rng.next_u32() as u8;
        assert_predict_eq(which, &ps, idx, pfcn, &st, "C36");
    }
}

/// C38 — with the `difftest` feature on, the public entry point must still
/// behave exactly as it does in the default build.
#[test]
fn c38_public_api_unchanged_under_difftest_feature() {
    assert!(difftest_feature_enabled());
    for pfcn in -2000..=2000 {
        assert_gpf_eq(pfcn, "C38");
    }
    let mut rng = Rng::new(0xC38);
    for _ in 0..10_000 {
        assert_gpf_eq(rng.i32_any(), "C38 rnd");
    }
}

// ---------------------------------------------------------------------------
// C5b / C39 — BTAC1C2_GetPredictFunc's *choice* of predictor, directly observed
//
// The public wrapper's own `default:` never inspects the pointer the selector
// returns, so the selector's `default:` arm is invisible through the public API.
// `__difftest_selector` exposes the choice itself, closing that blind spot.
// ---------------------------------------------------------------------------

/// Every `pfcn` in `0..=11` must select `_PfnN`; everything else must select the
/// generic dispatcher (index 12).
#[test]
fn c39_selector_choice_exhaustive() {
    for pfcn in 0..=11 {
        let v = assert_selector_eq(pfcn, "C39 valid");
        assert_eq!(v, pfcn, "C39: pfcn={pfcn} must select _Pfn{pfcn}");
    }
    for pfcn in -4096..=4096 {
        let v = assert_selector_eq(pfcn, "C39 sweep");
        let want = if (0..=11).contains(&pfcn) { pfcn } else { 12 };
        assert_eq!(v, want, "C39: selector choice at pfcn={pfcn}");
    }
    for pfcn in [
        i32::MIN, i32::MIN + 1, i32::MIN + 12, -1, 12, 13, 15, 16,
        i32::MAX - 1, i32::MAX,
    ] {
        let v = assert_selector_eq(pfcn, "C39 extremes");
        assert_eq!(v, 12, "C39: out-of-range pfcn={pfcn} must select the generic fn");
    }
    let mut rng = Rng::new(0xC39);
    for _ in 0..20_000 {
        let pfcn = rng.i32_shaped();
        let v = assert_selector_eq(pfcn, "C39 rnd");
        let want = if (0..=11).contains(&pfcn) { pfcn } else { 12 };
        assert_eq!(v, want, "C39: selector choice at pfcn={pfcn}");
    }
}

/// C40 — the composed pipeline: `BTAC1C2_GetPredictFunc(pfcn)` followed by a call
/// through the returned pointer.  This is how a real consumer drives the library,
/// and it catches selector/predictor mismatches that per-unit tests cannot.
#[test]
fn c40_composed_selector_then_predict() {
    let firfxs = [
        [[0i16; 8]; 4],
        [[256i16, -256, 128, -128, 64, -64, 32, -32]; 4],
        [[i16::MAX; 8]; 4],
        [[i16::MIN; 8]; 4],
    ];
    for firfx in firfxs {
        let st = st_with(firfx);
        for ps in psamp_shapes() {
            for idx in idx_shapes() {
                // Only 0..=11 are safe to *call*: the generic fallback with a
                // pfcn in 12..=15 would dereference ridx, which is fine here
                // because we pass a valid struct, and >15 hits the default arm.
                for pfcn in -4..=17 {
                    assert_call_selected_eq(&ps, idx, pfcn, &st, "C40");
                }
            }
        }
    }
    let mut rng = Rng::new(0xC40);
    for _ in 0..20_000 {
        let mut ps = [0i32; 8];
        for v in ps.iter_mut() {
            *v = rng.i32_shaped();
        }
        let mut firfx = [[0i16; 8]; 4];
        for row in firfx.iter_mut() {
            for v in row.iter_mut() {
                *v = rng.i16_shaped();
            }
        }
        let st = st_with(firfx);
        let idx = rng.i32_any();
        let pfcn = if rng.next_u64() % 2 == 0 {
            rng.i32_in(-8, 24)
        } else {
            rng.i32_any()
        };
        assert_call_selected_eq(&ps, idx, pfcn, &st, "C40 rnd");
    }
}

/// C41 — cross-check: for `pfcn` in `0..=11` the composed pipeline must equal a
/// direct call to `_PfnN` (**not** to the generic `case N`, which differs for
/// `N == 10` and `N == 11`), and for every other `pfcn` it must equal the generic
/// dispatcher.  This nails down exactly which function the selector returns.
#[test]
fn c41_composed_matches_the_right_unit() {
    let st = st_with([[256i16, -1, 2, -3, 4, -5, 6, -7]; 4]);
    let mut rng = Rng::new(0xC41);
    for _ in 0..4_000 {
        let mut ps = [0i32; 8];
        for v in ps.iter_mut() {
            *v = rng.i32_in(-1_000_000, 1_000_000);
        }
        let idx = rng.i32_in(-64, 64);
        for pfcn in 0..=11 {
            let composed = assert_call_selected_eq(&ps, idx, pfcn, &st, "C41");
            let direct = assert_predict_eq(pfcn, &ps, idx, pfcn, &st, "C41 direct");
            assert_eq!(
                composed, direct,
                "C41: selector for pfcn={pfcn} did not return _Pfn{pfcn}"
            );
        }
        for pfcn in [-1i32, 12, 13, 14, 15, 16, 100] {
            let composed = assert_call_selected_eq(&ps, idx, pfcn, &st, "C41 oor");
            let generic = assert_predict_eq(GENERIC, &ps, idx, pfcn, &st, "C41 generic");
            assert_eq!(
                composed, generic,
                "C41: selector for pfcn={pfcn} did not return the generic fn"
            );
        }
    }
}
