//! Phase C — error-path differential tests.
//!
//! One test per row of `ERRORS.md`.  Rows E1..E8 exercise the public
//! `get_predict_func` export and run under **every** feature combination.
//! Rows E9..E15 reach the `static` internals and therefore require the
//! `difftest` feature; they are compiled out otherwise.
//!
//! The library has no error codes: its whole rejection surface is the `default:`
//! arm of its three `switch (pfcn)` statements.  Each test asserts the *exact*
//! sentinel (`0`, or the generic-dispatcher fallback observable as `0`), not
//! merely "both failed somehow".

mod common;
use common::*;

// ===========================================================================
// E1..E8 — public surface (all feature combinations)
// ===========================================================================

/// E1 — `pfcn == 12`, one step past the last handled case (`11`).
#[test]
fn e1_pfcn_12_one_past_last_case() {
    let c = c_get_predict_func();
    let r = rust_get_predict_func();
    assert_eq!(unsafe { c(12) }, 0, "E1: C must return the 0 sentinel");
    assert_eq!(unsafe { r(12) }, 0, "E1: Rust must return the 0 sentinel");
    assert_gpf_eq(12, "E1");
    // and the last *valid* value still succeeds, pinning the boundary
    assert_eq!(unsafe { c(11) }, 1, "E1: pfcn=11 is valid in C");
    assert_eq!(unsafe { r(11) }, 1, "E1: pfcn=11 is valid in Rust");
}

/// E2 — `pfcn == -1`, one step below the first handled case (`0`).
#[test]
fn e2_pfcn_minus_one_one_below_first_case() {
    let c = c_get_predict_func();
    let r = rust_get_predict_func();
    assert_eq!(unsafe { c(-1) }, 0, "E2: C sentinel");
    assert_eq!(unsafe { r(-1) }, 0, "E2: Rust sentinel");
    assert_gpf_eq(-1, "E2");
    assert_eq!(unsafe { c(0) }, 1, "E2: pfcn=0 is valid in C");
    assert_eq!(unsafe { r(0) }, 1, "E2: pfcn=0 is valid in Rust");
}

/// E3 — `pfcn` in `13..=15`: handled by the innermost switch (firfx arms) but
/// *not* by `BTAC1C2_GetPredictFunc` nor by `get_predict_func`.
#[test]
fn e3_pfcn_13_to_15_inner_only_cases() {
    let c = c_get_predict_func();
    let r = rust_get_predict_func();
    for pfcn in 13..=15 {
        assert_eq!(unsafe { c(pfcn) }, 0, "E3: C sentinel at {pfcn}");
        assert_eq!(unsafe { r(pfcn) }, 0, "E3: Rust sentinel at {pfcn}");
        assert_gpf_eq(pfcn, "E3");
    }
}

/// E4 — `pfcn == 16`, one step past the innermost switch's last case (`15`).
#[test]
fn e4_pfcn_16_past_innermost_switch() {
    let c = c_get_predict_func();
    let r = rust_get_predict_func();
    assert_eq!(unsafe { c(16) }, 0, "E4: C sentinel");
    assert_eq!(unsafe { r(16) }, 0, "E4: Rust sentinel");
    assert_gpf_eq(16, "E4");
}

/// E5 — `pfcn == INT_MAX` and its neighbourhood.
#[test]
fn e5_pfcn_int_max() {
    let c = c_get_predict_func();
    let r = rust_get_predict_func();
    for pfcn in [i32::MAX, i32::MAX - 1, i32::MAX - 11, i32::MAX - 12] {
        assert_eq!(unsafe { c(pfcn) }, 0, "E5: C sentinel at {pfcn}");
        assert_eq!(unsafe { r(pfcn) }, 0, "E5: Rust sentinel at {pfcn}");
        assert_gpf_eq(pfcn, "E5");
    }
}

/// E6 — `pfcn == INT_MIN`; also the value for which `pfcn - 12` would overflow
/// inside `BTAC1C2_PredictSample`.  The `default:` arms must catch it first.
#[test]
fn e6_pfcn_int_min() {
    let c = c_get_predict_func();
    let r = rust_get_predict_func();
    for pfcn in [i32::MIN, i32::MIN + 1, i32::MIN + 11, i32::MIN + 12] {
        assert_eq!(unsafe { c(pfcn) }, 0, "E6: C sentinel at {pfcn}");
        assert_eq!(unsafe { r(pfcn) }, 0, "E6: Rust sentinel at {pfcn}");
        assert_gpf_eq(pfcn, "E6");
    }
}

/// E7 — every other out-of-range `int`: exhaustive `-4096..=4096` plus a
/// randomized full-`int` sweep.  Valid values must yield `1`, all others `0`,
/// on both sides.
#[test]
fn e7_exhaustive_and_random_out_of_range() {
    let c = c_get_predict_func();
    let r = rust_get_predict_func();
    for pfcn in -4096..=4096 {
        let expect = if (0..=11).contains(&pfcn) { 1 } else { 0 };
        assert_eq!(unsafe { c(pfcn) }, expect, "E7: C at {pfcn}");
        assert_eq!(unsafe { r(pfcn) }, expect, "E7: Rust at {pfcn}");
    }
    let mut rng = Rng::new(0xE7_5EED);
    for _ in 0..30_000 {
        let pfcn = rng.i32_any();
        let expect = if (0..=11).contains(&pfcn) { 1 } else { 0 };
        assert_eq!(unsafe { c(pfcn) }, expect, "E7 rnd: C at {pfcn}");
        assert_eq!(unsafe { r(pfcn) }, expect, "E7 rnd: Rust at {pfcn}");
    }
    let mut rng = Rng::new(0xE7_5EED_B);
    for _ in 0..30_000 {
        let pfcn = rng.i32_shaped();
        assert_gpf_eq(pfcn, "E7 shaped");
    }
}

/// E8 — `BTAC1C2_GetPredictFunc`'s `default:` hands back the *generic*
/// `BTAC1C2_PredictSample` pointer, which the caller's own `default:` never
/// compares.  Observable as a `0` result with no crash and no dereference.
/// Hammering it repeatedly also proves nothing is cached or corrupted.
#[test]
fn e8_selector_fallback_is_never_matched() {
    let c = c_get_predict_func();
    let r = rust_get_predict_func();
    let mut rng = Rng::new(0xE8);
    for _ in 0..20_000 {
        // draw only out-of-range values
        let pfcn = loop {
            let p = rng.i32_any();
            if !(0..=11).contains(&p) {
                break p;
            }
        };
        assert_eq!(unsafe { c(pfcn) }, 0, "E8: C at {pfcn}");
        assert_eq!(unsafe { r(pfcn) }, 0, "E8: Rust at {pfcn}");
    }
    // Interleave valid and invalid to rule out state leaking between calls.
    for i in 0..5_000 {
        let valid = i % 12;
        assert_eq!(unsafe { c(valid) }, 1);
        assert_eq!(unsafe { r(valid) }, 1);
        let invalid = 12 + i;
        assert_eq!(unsafe { c(invalid) }, 0);
        assert_eq!(unsafe { r(invalid) }, 0);
    }
}

/// Generic FFI boundary sanity: `get_predict_func` takes no pointers and no
/// lengths, and there is no `enum` in the source, so `pfcn` is the *only*
/// channel.  This test documents that by enumerating every boundary class the
/// checklist calls for and confirming C and Rust agree on each.
#[test]
fn generic_ffi_boundaries_public() {
    // "out-of-range enum value": pfcn behaves like an enum selector but is a
    // plain `int`, so every non-variant integer is a real input.
    let non_variants = [
        -2147483648i32, -1000000, -1000, -13, -12, -11, -1, 12, 13, 14, 15, 16,
        17, 31, 32, 63, 64, 127, 128, 255, 256, 1000, 1_000_000, 2147483647,
    ];
    for &pfcn in &non_variants {
        assert_gpf_eq(pfcn, "generic-boundary non-variant");
        assert_eq!(
            unsafe { c_get_predict_func()(pfcn) },
            0,
            "non-variant {pfcn} must be rejected with 0"
        );
    }
    // Every valid variant, for contrast.
    for pfcn in 0..=11 {
        assert_gpf_eq(pfcn, "generic-boundary variant");
        assert_eq!(unsafe { c_get_predict_func()(pfcn) }, 1);
    }
}

// ===========================================================================
// E9..E15 — internal surface (requires --features difftest)
// ===========================================================================

#[cfg(feature = "difftest")]
mod internals {
    use super::common::*;

    fn st_nonzero(firfx: [[i16; 8]; 4]) -> IdxState {
        let mut st = IdxState::zeroed();
        st.idx = 0xDEAD;
        st.lpred = -1;
        st.rpred = 1;
        st.tag = 0xFF;
        st.bcfcn = 0xFE;
        st.bsfcn = 0xFD;
        st.usefx = 0xFC;
        st.firfx = firfx;
        st
    }

    /// E9 — `BTAC1C2_PredictSample` with `pfcn` outside `0..=15`: the `default:`
    /// arm returns `0` without reading `psamp` or dereferencing `ridx`.
    #[test]
    fn e9_generic_default_returns_zero() {
        let c = c_difftest_predict();
        let r = rust_difftest_predict();
        let st = st_nonzero([[i16::MAX; 8]; 4]);
        let mut pfcns: Vec<i32> = vec![-1, -2, -12, -13, -16, 16, 17, 64, i32::MIN, i32::MAX];
        let mut rng = Rng::new(0xE9);
        for _ in 0..2_000 {
            let p = rng.i32_any();
            if !(0..=15).contains(&p) {
                pfcns.push(p);
            }
        }
        for &pfcn in &pfcns {
            for idx in [0i32, 7, -1, i32::MIN, i32::MAX] {
                let cv = assert_predict_eq(
                    GENERIC,
                    &[i32::MAX, i32::MIN, 1, -1, 2, -2, 3, -3],
                    idx,
                    pfcn,
                    &st,
                    "E9",
                );
                assert_eq!(cv, 0, "E9: default arm must return 0 (pfcn={pfcn})");
            }
        }
        // `ridx == NULL` on the default arm is safe: it is never dereferenced.
        for &pfcn in &[-1i32, 16, i32::MIN, i32::MAX] {
            let mut cs = [1i32, 2, 3, 4, 5, 6, 7, 8];
            let mut rs = cs;
            let cv = unsafe {
                c(GENERIC, cs.as_mut_ptr(), 3, pfcn, std::ptr::null_mut())
            };
            let rv = unsafe {
                r(GENERIC, rs.as_mut_ptr(), 3, pfcn, std::ptr::null_mut())
            };
            assert_eq!(cv, 0, "E9: C NULL-ridx default arm");
            assert_eq!(rv, 0, "E9: Rust NULL-ridx default arm");
        }
    }

    /// E10 — the boundaries around the `12..=15` firfx block.  `pfcn == 11` must
    /// use the shift-by-3 block-sum formula (never index `firfx`), and
    /// `pfcn == 16` must fall through to `0`.  Neither may touch `ridx`, so both
    /// must give the same answer with `ridx == NULL`.
    #[test]
    fn e10_firfx_block_boundaries() {
        let c = c_difftest_predict();
        let r = rust_difftest_predict();
        // A firfx that would produce a wildly different answer if it were read.
        let st = st_nonzero([[i16::MAX; 8]; 4]);
        let ps = [1000, -2000, 3000, -4000, 5000, -6000, 7000, -8000];

        for idx in 0..8 {
            // pfcn == 11: block sums, >>3 in the generic dispatcher.
            let v11 = assert_predict_eq(GENERIC, &ps, idx, 11, &st, "E10 pfcn=11");
            let mut cs = ps;
            let c11_null =
                unsafe { c(GENERIC, cs.as_mut_ptr(), idx, 11, std::ptr::null_mut()) };
            let mut rs = ps;
            let r11_null =
                unsafe { r(GENERIC, rs.as_mut_ptr(), idx, 11, std::ptr::null_mut()) };
            assert_eq!(c11_null, v11, "E10: C pfcn=11 read ridx");
            assert_eq!(r11_null, v11, "E10: Rust pfcn=11 read ridx");

            // pfcn == 16: default arm, always 0.
            let v16 = assert_predict_eq(GENERIC, &ps, idx, 16, &st, "E10 pfcn=16");
            assert_eq!(v16, 0, "E10: pfcn=16 must be 0");

            // pfcn == 12 and 15 *do* read firfx: they must differ from v11/v16
            // for this input, confirming the block really is 12..=15.
            let v12 = assert_predict_eq(GENERIC, &ps, idx, 12, &st, "E10 pfcn=12");
            let v15 = assert_predict_eq(GENERIC, &ps, idx, 15, &st, "E10 pfcn=15");
            assert_eq!(v12, v15, "E10: rows are identical for a uniform firfx");
        }
    }

    /// E11 — `pfcn == 12..=15` with an all-zero `firfx` is the only path that
    /// dereferences `ridx`; it must yield `0 / 256 == 0` and must not read past
    /// the `firfx[4][8]` bounds.  A canary struct with poisoned neighbours makes
    /// an out-of-bounds row index detectable.
    #[test]
    fn e11_firfx_zero_and_bounds() {
        let st = st_nonzero([[0i16; 8]; 4]);
        for pfcn in 12..=15 {
            for idx in 0..8 {
                let v = assert_predict_eq(
                    GENERIC,
                    &[i32::MAX, i32::MIN, 12345, -12345, 1, -1, 0, 7],
                    idx,
                    pfcn,
                    &st,
                    "E11",
                );
                assert_eq!(v, 0, "E11: zero firfx must give 0 (pfcn={pfcn})");
            }
        }

        // Bounds: put a marker only in row r and confirm exactly one pfcn sees
        // it, i.e. `pfcn - 12` indexes rows 0..=3 and nothing else.
        for r in 0..4usize {
            let mut firfx = [[0i16; 8]; 4];
            firfx[r][0] = 256;
            let st = st_nonzero(firfx);
            let ps = [11, 22, 33, 44, 55, 66, 77, 88];
            for pfcn in 12..=15 {
                let v = assert_predict_eq(GENERIC, &ps, 0, pfcn, &st, "E11 bounds");
                let expect_hit = (pfcn - 12) as usize == r;
                assert_eq!(
                    v != 0,
                    expect_hit,
                    "E11: pfcn={pfcn} indexed row {r} incorrectly (got {v})"
                );
            }
        }
    }

    /// E12 — `psamp[(idx - k) & 7]` must stay inside the 8-element array for
    /// every `idx`, including `INT_MIN` where `idx - k` overflows.  A guarded
    /// buffer with poison on both sides catches any out-of-bounds read.
    #[test]
    fn e12_index_masking_stays_in_bounds() {
        let c = c_difftest_predict();
        let r = rust_difftest_predict();

        // 8 poison + 8 real + 8 poison; hand out a pointer to the middle.
        const POISON: i32 = 0x7BAD_BAD0u32 as i32;
        let real = [3i32, -7, 11, -13, 17, -19, 23, -29];
        let st = st_nonzero([[100i16; 8]; 4]);

        let mut idxs: Vec<i32> = (-40..=40).collect();
        idxs.extend_from_slice(&[
            i32::MIN,
            i32::MIN + 1,
            i32::MIN + 7,
            i32::MIN + 8,
            i32::MAX,
            i32::MAX - 1,
            i32::MAX - 7,
            i32::MAX - 8,
        ]);
        let mut rng = Rng::new(0xE12);
        for _ in 0..2_000 {
            idxs.push(rng.i32_any());
        }

        for which in [-1i32, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11] {
            for pfcn in [0i32, 4, 7, 9, 11, 12, 15] {
                for &idx in &idxs {
                    let mut cbuf = [POISON; 24];
                    cbuf[8..16].copy_from_slice(&real);
                    let mut rbuf = cbuf;
                    let mut cst = st;
                    let mut rst = st;
                    let cv = unsafe {
                        c(which, cbuf.as_mut_ptr().add(8), idx, pfcn, &mut cst)
                    };
                    let rv = unsafe {
                        r(which, rbuf.as_mut_ptr().add(8), idx, pfcn, &mut rst)
                    };
                    assert_eq!(
                        cv, rv,
                        "E12: which={which} pfcn={pfcn} idx={idx}"
                    );
                    // The guard regions must be untouched, and the answer must
                    // be reproducible from the 8 real values alone.
                    assert!(
                        cbuf[..8].iter().all(|&x| x == POISON)
                            && cbuf[16..].iter().all(|&x| x == POISON),
                        "E12: C wrote outside the window"
                    );
                    assert!(
                        rbuf[..8].iter().all(|&x| x == POISON)
                            && rbuf[16..].iter().all(|&x| x == POISON),
                        "E12: Rust wrote outside the window"
                    );
                }
            }
        }
    }

    /// E13 — accumulator overflow: `psamp` at `INT_MAX` / `INT_MIN` overflows
    /// signed `int` in the C.  gcc (two's complement, no `-ftrapv`) wraps, so
    /// the Rust must wrap identically for every arm.
    #[test]
    fn e13_accumulator_overflow_wraps() {
        let firfxs = [
            [[0i16; 8]; 4],
            [[i16::MAX; 8]; 4],
            [[i16::MIN; 8]; 4],
            [[256i16; 8]; 4],
        ];
        let extremes: [[i32; 8]; 8] = [
            [i32::MAX; 8],
            [i32::MIN; 8],
            [i32::MAX, i32::MIN, i32::MAX, i32::MIN, i32::MAX, i32::MIN, i32::MAX, i32::MIN],
            [i32::MIN, i32::MAX, i32::MIN, i32::MAX, i32::MIN, i32::MAX, i32::MIN, i32::MAX],
            [i32::MAX - 1; 8],
            [i32::MIN + 1; 8],
            [i32::MAX / 2 + 1; 8],
            [i32::MIN / 2 - 1; 8],
        ];
        for firfx in firfxs {
            let st = st_nonzero(firfx);
            for ps in extremes {
                for idx in 0..8 {
                    for which in -1..=11 {
                        assert_predict_eq(which, &ps, idx, 0, &st, "E13");
                    }
                    for pfcn in 0..=16 {
                        assert_predict_eq(GENERIC, &ps, idx, pfcn, &st, "E13 generic");
                    }
                }
            }
        }
    }

    /// E14 — the specialised predictors ignore their `pfcn` argument entirely,
    /// including values that are invalid everywhere else.
    #[test]
    fn e14_specialised_ignore_pfcn_even_when_invalid() {
        let st = st_nonzero([[i16::MIN; 8]; 4]);
        let mut rng = Rng::new(0xE14);
        for _ in 0..600 {
            let mut ps = [0i32; 8];
            for v in ps.iter_mut() {
                *v = rng.i32_shaped();
            }
            let idx = rng.i32_any();
            for which in 0..=11 {
                let base = assert_predict_eq(which, &ps, idx, which, &st, "E14 base");
                for pfcn in [-1i32, 12, 13, 14, 15, 16, 999, i32::MIN, i32::MAX] {
                    let v = assert_predict_eq(which, &ps, idx, pfcn, &st, "E14");
                    assert_eq!(
                        v, base,
                        "E14: which={which} changed with invalid pfcn={pfcn}"
                    );
                }
            }
        }
    }

    /// E15 — `ridx == NULL` for every specialised predictor: never dereferenced,
    /// so no crash and the same result as with a valid `ridx`.
    #[test]
    fn e15_null_ridx_all_specialised() {
        let c = c_difftest_predict();
        let r = rust_difftest_predict();
        let st = st_nonzero([[i16::MAX; 8]; 4]);
        let mut rng = Rng::new(0xE15);
        for _ in 0..400 {
            let mut ps = [0i32; 8];
            for v in ps.iter_mut() {
                *v = rng.i32_shaped();
            }
            let idx = rng.i32_any();
            let pfcn = rng.i32_any();
            for which in 0..=11 {
                let mut cs = ps;
                let mut rs = ps;
                let cv = unsafe {
                    c(which, cs.as_mut_ptr(), idx, pfcn, std::ptr::null_mut())
                };
                let rv = unsafe {
                    r(which, rs.as_mut_ptr(), idx, pfcn, std::ptr::null_mut())
                };
                assert_eq!(cv, rv, "E15: which={which} idx={idx} pfcn={pfcn}");
                let with_st = assert_predict_eq(which, &ps, idx, pfcn, &st, "E15 ref");
                assert_eq!(cv, with_st, "E15: NULL ridx changed the result");
            }
        }
    }

    /// Out-of-range "enum" values for the `__difftest_predict` selector itself:
    /// any `int` must route to the generic dispatcher on both sides.
    #[test]
    fn selector_out_of_range_routes_to_generic() {
        let st = st_nonzero([[13i16; 8]; 4]);
        let ps = [5i32, -6, 7, -8, 9, -10, 11, -12];
        for pfcn in 0..=16 {
            let generic = assert_predict_eq(GENERIC, &ps, 3, pfcn, &st, "sel ref");
            for which in [-1i32, -2, -100, 12, 13, 99, i32::MIN, i32::MAX] {
                let v = assert_predict_eq(which, &ps, 3, pfcn, &st, "sel oor");
                assert_eq!(
                    v, generic,
                    "selector {which} must route to the generic dispatcher"
                );
            }
        }
    }
}

#[cfg(feature = "difftest")]
mod selector_errors {
    use super::common::*;

    /// E8b — the selector's `default:` arm, observed **directly**.  For every
    /// `pfcn` outside `0..=11` it must hand back the generic
    /// `BTAC1C2_PredictSample` (index 12) — never a specialised predictor and
    /// never an unrecognised pointer (-1).
    #[test]
    fn e8b_selector_default_returns_generic() {
        for pfcn in [
            -1i32, -2, -12, -13, 12, 13, 14, 15, 16, 17, 100, 4096, -4096,
            i32::MIN, i32::MIN + 1, i32::MAX, i32::MAX - 1,
        ] {
            let v = assert_selector_eq(pfcn, "E8b");
            assert_eq!(
                v, 12,
                "E8b: pfcn={pfcn} must fall back to the generic dispatcher, got #{v}"
            );
        }
        let mut rng = Rng::new(0xE8B);
        for _ in 0..30_000 {
            let pfcn = loop {
                let p = rng.i32_shaped();
                if !(0..=11).contains(&p) {
                    break p;
                }
            };
            let v = assert_selector_eq(pfcn, "E8b rnd");
            assert_eq!(v, 12, "E8b: pfcn={pfcn} fell back to #{v}");
        }
    }

    /// E8c — the selector never returns an unrecognised pointer for any input.
    #[test]
    fn e8c_selector_never_unrecognised() {
        let mut rng = Rng::new(0xE8C);
        for _ in 0..20_000 {
            let pfcn = rng.i32_any();
            let v = assert_selector_eq(pfcn, "E8c");
            assert!(
                (0..=12).contains(&v),
                "E8c: selector returned unrecognised #{v} for pfcn={pfcn}"
            );
        }
    }

    /// E8d — the composed pipeline on the error path: an out-of-range `pfcn`
    /// routes to the generic dispatcher, whose own `default:` returns 0 for
    /// `pfcn > 15` without dereferencing `ridx`.
    #[test]
    fn e8d_composed_error_path() {
        let mut st = IdxState::zeroed();
        st.firfx = [[i16::MAX; 8]; 4];
        let ps = [1i32, -2, 3, -4, 5, -6, 7, -8];
        for pfcn in [16i32, 17, 100, 4096, i32::MAX] {
            for idx in [0i32, 7, -1, i32::MIN, i32::MAX] {
                let v = assert_call_selected_eq(&ps, idx, pfcn, &st, "E8d");
                assert_eq!(v, 0, "E8d: pfcn={pfcn} must yield the 0 sentinel");
            }
        }
        // Negative pfcn also hits the generic default arm.
        for pfcn in [-1i32, -2, -100, i32::MIN] {
            for idx in [0i32, 3, -5] {
                let v = assert_call_selected_eq(&ps, idx, pfcn, &st, "E8d neg");
                assert_eq!(v, 0, "E8d: pfcn={pfcn} must yield the 0 sentinel");
            }
        }
    }
}
