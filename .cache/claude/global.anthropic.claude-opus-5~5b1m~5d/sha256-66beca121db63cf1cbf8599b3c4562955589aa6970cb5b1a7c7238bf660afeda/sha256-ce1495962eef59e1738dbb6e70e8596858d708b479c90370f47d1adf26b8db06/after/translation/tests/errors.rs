//! Phase C — error-path / rejection differential tests.
//!
//! One test per row of `ERRORS.md`. Each asserts the two libraries reject the
//! same input with the *same* sentinel (`call_predict` → `0`, the generic
//! predictor → `pred = 0`), not merely "both failed somehow".

mod common;

use common::*;
use std::ffi::c_int;

fn both(c: &Lib, r: &Lib, pfcn: i32) -> (c_int, c_int) {
    let cf = c.call_predict();
    let rf = r.call_predict();
    (unsafe { cf(pfcn) }, unsafe { rf(pfcn) })
}

/// Assert both libraries return the *same* value AND that the value is the
/// expected rejection sentinel `0`.
fn assert_rejects_identically(c: &Lib, r: &Lib, pfcn: i32) {
    let (cv, rv) = both(c, r, pfcn);
    assert_eq!(
        cv, rv,
        "call_predict({pfcn}): {} = {cv}, {} = {rv}",
        c.name, r.name
    );
    assert_eq!(cv, 0, "call_predict({pfcn}) should hit `default:` and yield 0, got {cv}");
}

// ---------------------------------------------------------------------------
// E1 — pfcn == 12, first value past `call_predict`'s case range
// ---------------------------------------------------------------------------
#[test]
fn err_e1_pfcn_12_first_past_range() {
    let (c, r) = open_pair();
    assert_rejects_identically(&c, &r, 12);
    // and 11 (the last valid) must still be accepted, identically
    let (cv, rv) = both(&c, &r, 11);
    assert_eq!((cv, rv), (1, 1), "pfcn 11 must still be accepted by both");
}

// ---------------------------------------------------------------------------
// E2 — pfcn 13,14,15: inner switch has cases, outer does not
// ---------------------------------------------------------------------------
#[test]
fn err_e2_pfcn_13_14_15() {
    let (c, r) = open_pair();
    for pfcn in [13, 14, 15] {
        assert_rejects_identically(&c, &r, pfcn);
    }
}

// ---------------------------------------------------------------------------
// E3 — pfcn == 16, past even the inner switch
// ---------------------------------------------------------------------------
#[test]
fn err_e3_pfcn_16_past_inner_switch() {
    let (c, r) = open_pair();
    assert_rejects_identically(&c, &r, 16);
}

// ---------------------------------------------------------------------------
// E4 — pfcn == -1, one step below the valid range
// ---------------------------------------------------------------------------
#[test]
fn err_e4_pfcn_minus_one() {
    let (c, r) = open_pair();
    assert_rejects_identically(&c, &r, -1);
    let (cv, rv) = both(&c, &r, 0);
    assert_eq!((cv, rv), (1, 1), "pfcn 0 must still be accepted by both");
}

// ---------------------------------------------------------------------------
// E5 — all negatives in a dense band
// ---------------------------------------------------------------------------
#[test]
fn err_e5_pfcn_negative_range() {
    let (c, r) = open_pair();
    for pfcn in -1000..0 {
        assert_rejects_identically(&c, &r, pfcn);
    }
}

// ---------------------------------------------------------------------------
// E6 / E7 — INT_MIN / INT_MAX across the FFI boundary
// ---------------------------------------------------------------------------
#[test]
fn err_e6_pfcn_int_min() {
    let (c, r) = open_pair();
    assert_rejects_identically(&c, &r, i32::MIN);
    assert_rejects_identically(&c, &r, i32::MIN + 1);
}

#[test]
fn err_e7_pfcn_int_max() {
    let (c, r) = open_pair();
    assert_rejects_identically(&c, &r, i32::MAX);
    assert_rejects_identically(&c, &r, i32::MAX - 1);
}

// ---------------------------------------------------------------------------
// E8 — out-of-range "enum" sweep: 12..=4096, powers of two, and random i32s.
// C enums accept any int, so every one of these is a legal FFI input.
// ---------------------------------------------------------------------------
#[test]
fn err_e8_exhaustive_out_of_range_sweep() {
    let (c, r) = open_pair();
    for pfcn in 12..=4096 {
        assert_rejects_identically(&c, &r, pfcn);
    }
    for k in 0..31 {
        let v = 1i32 << k;
        if !(0..=11).contains(&v) {
            assert_rejects_identically(&c, &r, v);
        }
        assert_rejects_identically(&c, &r, -v);
        let w = v - 1;
        if !(0..=11).contains(&w) {
            assert_rejects_identically(&c, &r, w);
        }
    }
}

#[test]
fn err_e8b_random_i32_sweep() {
    let (c, r) = open_pair();
    let cf = c.call_predict();
    let rf = r.call_predict();
    let mut rng = Rng::new(0x5EED_00E8);
    for _ in 0..20_000 {
        let pfcn = rng.next_i32();
        let cv = unsafe { cf(pfcn) };
        let rv = unsafe { rf(pfcn) };
        assert_eq!(cv, rv, "call_predict({pfcn}) diverged: C={cv} Rust={rv}");
        let expected = if (0..=11).contains(&pfcn) { 1 } else { 0 };
        assert_eq!(cv, expected, "C contract broken at pfcn={pfcn}");
    }
}

// ---------------------------------------------------------------------------
// E9 — BTAC1C2_GetPredictFunc default fallback returns the *generic* predictor
// ---------------------------------------------------------------------------
#[test]
fn err_e9_getpredictfunc_default_fallback() {
    // Observable part through the public ABI: result is 0.
    let (c, r) = open_pair();
    for pfcn in [12, 13, 14, 15, 16, -1, -2, i32::MIN, i32::MAX, 9999] {
        assert_rejects_identically(&c, &r, pfcn);
    }

    // Direct part: the returned pointer must be the generic predictor in both.
    let Some((c, r)) = open_pair_debug() else {
        eprintln!("E9: skipping direct check, debug .so not built");
        return;
    };
    let cg = c.get_predict_func_fn().expect("C GetPredictFunc");
    let rg = r.get_predict_func_fn().expect("Rust GetPredictFunc");
    let c_generic = c.internal_addr("BTAC1C2_PredictSample").expect("C generic");
    let r_generic = r.internal_addr("BTAC1C2_PredictSample").expect("Rust generic");

    for pfcn in [12, 13, 14, 15, 16, -1, -2, 1000, i32::MIN, i32::MAX] {
        let cp = unsafe { cg(pfcn) } as usize;
        let rp = unsafe { rg(pfcn) } as usize;
        assert_eq!(cp, c_generic, "C: GetPredictFunc({pfcn}) is not the generic fn");
        assert_eq!(rp, r_generic, "Rust: GetPredictFunc({pfcn}) is not the generic fn");
    }
}

// ---------------------------------------------------------------------------
// E10 — BTAC1C2_PredictSample `default:` → pred = 0 for pfcn outside 0..=15
// ---------------------------------------------------------------------------
#[test]
fn err_e10_predictsample_default_zero() {
    let Some((c, r)) = open_pair_debug() else {
        eprintln!("E10: skipping, debug .so not built");
        return;
    };
    let cfn = c.predict_fn("BTAC1C2_PredictSample").expect("C generic");
    let rfn = r.predict_fn("BTAC1C2_PredictSample").expect("Rust generic");

    let bad: Vec<c_int> = {
        let mut v = vec![16, 17, 31, 32, 100, 1000, -1, -2, -16, i32::MIN, i32::MAX, i32::MIN + 1];
        let mut rng = Rng::new(0x5EED_00E1);
        for _ in 0..500 {
            let x = rng.next_i32();
            if !(0..=15).contains(&x) {
                v.push(x);
            }
        }
        v
    };

    for ps in &psamp_shapes() {
        for fir in firfx_shapes().iter().take(6) {
            let mut st = IdxState::zeroed();
            st.firfx = *fir;
            for &pfcn in &bad {
                for &idx in &[0i32, 3, 7, -1, i32::MIN, i32::MAX] {
                    let mut cb = *ps;
                    let mut rb = *ps;
                    let mut cs = st;
                    let mut rs = st;
                    let cv = unsafe { cfn(cb.as_mut_ptr(), idx, pfcn, &mut cs) };
                    let rv = unsafe { rfn(rb.as_mut_ptr(), idx, pfcn, &mut rs) };
                    assert_eq!(
                        cv, rv,
                        "PredictSample(psamp={ps:?}, idx={idx}, pfcn={pfcn}) C={cv} Rust={rv}"
                    );
                    assert_eq!(cv, 0, "C `default:` must yield 0 (pfcn={pfcn})");
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// E11 — cases 12..15 dereference `ridx` with no NULL check, but they are
// unreachable from the public ABI. Assert the reachable contract: pfcn 12..15
// via `call_predict` returns 0 and touches no memory.
// ---------------------------------------------------------------------------
#[test]
fn err_e11_no_ridx_deref_via_public_api() {
    let (c, r) = open_pair();
    for pfcn in 12..=15 {
        assert_rejects_identically(&c, &r, pfcn);
    }
    // Called many times: if either library dereferenced a bogus `ridx` it would
    // fault; surviving 100k calls proves the public path never does.
    let cf = c.call_predict();
    let rf = r.call_predict();
    for _ in 0..100_000 {
        for pfcn in 12..=15 {
            assert_eq!(unsafe { cf(pfcn) }, unsafe { rf(pfcn) });
        }
    }

    // Directly: a NULL `ridx` with `pfcn` in 0..=11 is fine in the C (those
    // cases never touch `ridx`); the Rust must tolerate it identically.
    let Some((c, r)) = open_pair_debug() else {
        eprintln!("E11: skipping NULL-ridx check, debug .so not built");
        return;
    };
    let cfn = c.predict_fn("BTAC1C2_PredictSample").expect("C generic");
    let rfn = r.predict_fn("BTAC1C2_PredictSample").expect("Rust generic");
    for ps in psamp_shapes().iter().take(8) {
        for pfcn in 0..=11 {
            for idx in 0..8 {
                let mut cb = *ps;
                let mut rb = *ps;
                let cv = unsafe { cfn(cb.as_mut_ptr(), idx, pfcn, std::ptr::null_mut()) };
                let rv = unsafe { rfn(rb.as_mut_ptr(), idx, pfcn, std::ptr::null_mut()) };
                assert_eq!(cv, rv, "NULL ridx, pfcn={pfcn} idx={idx}: C={cv} Rust={rv}");
            }
        }
    }
    // ... and for the out-of-range `default:` case, which also never reads ridx.
    for pfcn in [16, -1, 100, i32::MIN, i32::MAX] {
        let mut cb = [1i32, 2, 3, 4, 5, 6, 7, 8];
        let mut rb = cb;
        let cv = unsafe { cfn(cb.as_mut_ptr(), 0, pfcn, std::ptr::null_mut()) };
        let rv = unsafe { rfn(rb.as_mut_ptr(), 0, pfcn, std::ptr::null_mut()) };
        assert_eq!(cv, rv);
        assert_eq!(cv, 0);
    }
    // Specialised variants ignore `ridx` entirely -> NULL must be harmless.
    for i in 0..12usize {
        let name = format!("BTAC1C2_PredictSample_Pfn{i}");
        let cf2 = c.predict_fn(&name).expect("C Pfn");
        let rf2 = r.predict_fn(&name).expect("Rust Pfn");
        for ps in psamp_shapes().iter().take(8) {
            for idx in [0i32, 5, -3, i32::MIN, i32::MAX] {
                let mut cb = *ps;
                let mut rb = *ps;
                let cv = unsafe { cf2(cb.as_mut_ptr(), idx, i as c_int, std::ptr::null_mut()) };
                let rv = unsafe { rf2(rb.as_mut_ptr(), idx, i as c_int, std::ptr::null_mut()) };
                assert_eq!(cv, rv, "{name} NULL ridx idx={idx}: C={cv} Rust={rv}");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// E12 — extreme `idx` never escapes the 8-word window (`& 7` masking)
// ---------------------------------------------------------------------------
#[test]
fn err_e12_idx_masking_no_oob() {
    let Some((c, r)) = open_pair_debug() else {
        eprintln!("E12: skipping, debug .so not built");
        return;
    };
    // Guard pattern around the 8-word window: if either library reads outside
    // it, the value would leak into the result and the two would diverge (and
    // the masking identity below would break).
    let mut rng = Rng::new(0x5EED_00E2);
    let mut st = IdxState::zeroed();
    for row in st.firfx.iter_mut() {
        for cf in row.iter_mut() {
            *cf = rng.next_i16();
        }
    }

    let extreme: Vec<c_int> = {
        let mut v = vec![
            i32::MIN,
            i32::MIN + 1,
            i32::MIN + 7,
            i32::MAX,
            i32::MAX - 1,
            i32::MAX - 7,
            -1,
            -8,
            -9,
            1 << 30,
            -(1 << 30),
        ];
        for _ in 0..300 {
            v.push(rng.next_i32());
        }
        v
    };

    let names: Vec<String> = (0..12)
        .map(|i| format!("BTAC1C2_PredictSample_Pfn{i}"))
        .chain(std::iter::once("BTAC1C2_PredictSample".to_string()))
        .collect();

    for name in &names {
        let cfn = c.predict_fn(name).unwrap_or_else(|| panic!("C {name}"));
        let rfn = r.predict_fn(name).unwrap_or_else(|| panic!("Rust {name}"));
        for pfcn in [0i32, 7, 11, 12, 15] {
            for ps in psamp_shapes().iter().take(12) {
                for &idx in &extreme {
                    let mut cb = *ps;
                    let mut rb = *ps;
                    let mut cs = st;
                    let mut rs = st;
                    let cv = unsafe { cfn(cb.as_mut_ptr(), idx, pfcn, &mut cs) };
                    let rv = unsafe { rfn(rb.as_mut_ptr(), idx, pfcn, &mut rs) };
                    assert_eq!(
                        cv, rv,
                        "{name}(idx={idx}, pfcn={pfcn}, psamp={ps:?}): C={cv} Rust={rv}"
                    );
                    // masking identity: idx and (idx & 7) must be equivalent
                    let m = idx & 7;
                    let mut cb2 = *ps;
                    let mut rb2 = *ps;
                    let mut cs2 = st;
                    let mut rs2 = st;
                    let cv2 = unsafe { cfn(cb2.as_mut_ptr(), m, pfcn, &mut cs2) };
                    let rv2 = unsafe { rfn(rb2.as_mut_ptr(), m, pfcn, &mut rs2) };
                    assert_eq!(cv, cv2, "C {name}: idx={idx} != idx&7={m}");
                    assert_eq!(rv, rv2, "Rust {name}: idx={idx} != idx&7={m}");
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// E13 / E14 — the public ABI has no pointer and no length parameter at all.
// Verified structurally: `call_predict` is the only exported symbol and its C
// prototype is `int call_predict(int)`.
// ---------------------------------------------------------------------------
#[test]
fn err_e13_no_pointer_params_in_public_abi() {
    use std::process::Command;

    for path in [c_so_path(), rust_so_release()] {
        let out = Command::new("nm")
            .arg("-D")
            .arg("--defined-only")
            .arg(&path)
            .output()
            .expect("nm");
        let text = String::from_utf8_lossy(&out.stdout);
        let names: Vec<&str> = text
            .lines()
            .filter_map(|l| l.split_whitespace().nth(2))
            .collect();
        assert_eq!(
            names,
            vec!["call_predict"],
            "{} exports {:?}, expected exactly [call_predict]",
            path.display(),
            names
        );
    }

    // The header prototype takes a single `int` and no pointer/length, so there
    // is no NULL / zero-length / oversized-length input to construct. Calling
    // through a *wrong* arity would be UB in C too, so the strongest check is
    // that the single-int signature works from both and that garbage in the
    // unused argument registers does not leak into the result. (SysV AMD64
    // passes the extra integer args in registers, so this is well-defined for
    // a variadic-less callee there; restrict it to that ABI.)
    #[cfg(all(target_arch = "x86_64", target_family = "unix"))]
    {
        let (c, r) = open_pair();
        type Wide = unsafe extern "C" fn(c_int, u64, u64, u64, u64, u64) -> c_int;
        let cf: Wide = unsafe { std::mem::transmute(c.call_predict()) };
        let rf: Wide = unsafe { std::mem::transmute(r.call_predict()) };
        for pfcn in [-1i32, 0, 5, 11, 12, 16, i32::MIN, i32::MAX] {
            let cv = unsafe { cf(pfcn, !0, !0, !0, !0, !0) };
            let rv = unsafe { rf(pfcn, !0, !0, !0, !0, !0) };
            assert_eq!(cv, rv, "extra-garbage-args call_predict({pfcn}): C={cv} Rust={rv}");
            assert_eq!(cv, unsafe { c.call_predict()(pfcn) });
            assert_eq!(rv, unsafe { r.call_predict()(pfcn) });
        }
    }

    // Sign/width behaviour at the boundary: `int` is 32-bit on both sides, so a
    // 64-bit value truncated to `int` must be interpreted identically.
    let (c, r) = open_pair();
    let cf = c.call_predict();
    let rf = r.call_predict();
    for hi in [0u64, 1, 0xFFFF_FFFF, 0xDEAD_BEEF] {
        for lo in [0u32, 5, 11, 12, u32::MAX, 0x8000_0000] {
            let packed = ((hi << 32) | lo as u64) as u32 as i32;
            assert_eq!(
                unsafe { cf(packed) },
                unsafe { rf(packed) },
                "call_predict({packed}) (from {hi:#x}:{lo:#x})"
            );
        }
    }
}
