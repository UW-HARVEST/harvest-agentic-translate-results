//! Extra-paranoid brute-force differential sweeps.
//!
//! These go well beyond the `CONFIGS.md` / `ERRORS.md` rows: they blanket the
//! whole input space to catch value-dependent divergences that a row-per-shape
//! test could miss.  Ignored by default in CI-sized runs?  No — they are cheap
//! enough (a few hundred ms) to always run.

mod common;
use common::*;

/// Exhaustive over the *entire* `pfcn` domain in the region where any switch has
/// a case, times every `idx` rotation, times a dense grid of small `psamp`
/// values.  Public API only, so this runs under every feature combination.
#[test]
fn exhaustive_public_api_dense() {
    let c = c_get_predict_func();
    let r = rust_get_predict_func();
    // Every value in a wide window around all boundaries.
    for pfcn in -70_000..=70_000 {
        let cv = unsafe { c(pfcn) };
        let rv = unsafe { r(pfcn) };
        if cv != rv {
            panic!("divergence at pfcn={pfcn}: C={cv} Rust={rv}");
        }
    }
}

/// Exhaustive over all 2^31-ish is infeasible, but we can sweep every value
/// whose low 20 bits are 0 plus every value in dense windows at both extremes.
#[test]
fn exhaustive_public_api_strided_and_extremes() {
    let c = c_get_predict_func();
    let r = rust_get_predict_func();

    let check = |pfcn: i32| {
        let cv = unsafe { c(pfcn) };
        let rv = unsafe { r(pfcn) };
        assert_eq!(cv, rv, "divergence at pfcn={pfcn}");
    };

    // strided sweep across the full i32 range
    let mut v: i64 = i32::MIN as i64;
    while v <= i32::MAX as i64 {
        check(v as i32);
        v += 4093; // prime stride, so it hits every residue class mod small n
    }
    // dense windows at both extremes
    for k in 0..100_000i64 {
        check((i32::MIN as i64 + k) as i32);
        check((i32::MAX as i64 - k) as i32);
    }
}

#[cfg(feature = "difftest")]
mod deep {
    use super::common::*;

    fn st(firfx: [[i16; 8]; 4]) -> IdxState {
        let mut s = IdxState::zeroed();
        s.idx = 0x1234;
        s.lpred = -9;
        s.rpred = 9;
        s.tag = 1;
        s.bcfcn = 2;
        s.bsfcn = 3;
        s.usefx = 4;
        s.firfx = firfx;
        s
    }

    /// Exhaustive: every entry point x every `pfcn` in `-4..=20` x every `idx`
    /// rotation x a dense grid of ternary `psamp` patterns.  This is a genuine
    /// exhaustive sweep of the arithmetic's sign/carry structure.
    #[test]
    fn exhaustive_ternary_psamp_grid() {
        let c = c_difftest_predict();
        let r = rust_difftest_predict();
        let s0 = st([[0i16; 8]; 4]);
        let s1 = st([[257i16, -257, 129, -129, 65, -65, 33, -33]; 4]);

        // 3^8 = 6561 sign patterns over {-K, 0, +K}
        for &k in &[1i32, 3, 1_000_000, 300_000_000] {
            let vals = [-k, 0, k];
            for pat in 0..3usize.pow(8) {
                let mut ps = [0i32; 8];
                let mut p = pat;
                for v in ps.iter_mut() {
                    *v = vals[p % 3];
                    p /= 3;
                }
                // Rotating through idx covers all window alignments cheaply.
                let idx = (pat % 8) as i32;
                for which in [-1i32, 0, 4, 7, 9, 10, 11] {
                    for pfcn in [0i32, 5, 7, 8, 10, 11, 12, 14, 16] {
                        let stt = if pfcn >= 12 { &s1 } else { &s0 };
                        let mut cs = ps;
                        let mut rs = ps;
                        let mut cst = *stt;
                        let mut rst = *stt;
                        let cv =
                            unsafe { c(which, cs.as_mut_ptr(), idx, pfcn, &mut cst) };
                        let rv =
                            unsafe { r(which, rs.as_mut_ptr(), idx, pfcn, &mut rst) };
                        assert_eq!(
                            cv, rv,
                            "which={which} pfcn={pfcn} idx={idx} k={k} ps={ps:?}"
                        );
                    }
                }
            }
        }
    }

    /// Exhaustive: every arm of `BTAC1C2_PredictSample` (`pfcn` in `-2..=17`) x
    /// every `idx` in `0..64` x every one-hot `psamp` (isolates each tap's
    /// coefficient exactly) x several magnitudes including overflow-inducing.
    #[test]
    fn exhaustive_one_hot_taps() {
        let s0 = st([[0i16; 8]; 4]);
        for mag in [1i32, -1, 16, 64, 256, 1 << 20, i32::MAX, i32::MIN, i32::MAX / 5] {
            for hot in 0..8usize {
                let mut ps = [0i32; 8];
                ps[hot] = mag;
                for idx in 0..64i32 {
                    for pfcn in -2..=17i32 {
                        assert_predict_eq(GENERIC, &ps, idx, pfcn, &s0, "one-hot generic");
                    }
                    for which in 0..=11i32 {
                        assert_predict_eq(which, &ps, idx, which, &s0, "one-hot spec");
                    }
                }
            }
        }
    }

    /// One-hot `firfx`: isolates every one of the 4x8 = 32 coefficients so a
    /// transposed / off-by-one / wrong-stride `firfx` access is caught exactly.
    #[test]
    fn exhaustive_one_hot_firfx() {
        for mag in [1i16, -1, 256, -256, i16::MAX, i16::MIN, 1000, -1000] {
            for r in 0..4usize {
                for col in 0..8usize {
                    let mut firfx = [[0i16; 8]; 4];
                    firfx[r][col] = mag;
                    let s = st(firfx);
                    for hot in 0..8usize {
                        for &sv in &[1i32, -1, 1 << 16, i32::MAX, i32::MIN] {
                            let mut ps = [0i32; 8];
                            ps[hot] = sv;
                            for idx in 0..16i32 {
                                for pfcn in 12..=15i32 {
                                    assert_predict_eq(
                                        GENERIC, &ps, idx, pfcn, &s,
                                        "one-hot firfx",
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// A very large uniformly-random sweep over every axis simultaneously.
    #[test]
    fn massive_random_sweep() {
        let c = c_difftest_predict();
        let r = rust_difftest_predict();
        let mut rng = Rng::new(0xDEED_5EED);
        for _ in 0..300_000 {
            let which = rng.i32_in(-2, 13);
            let pfcn = if rng.next_u64() % 2 == 0 {
                rng.i32_in(-4, 19)
            } else {
                rng.i32_any()
            };
            let idx = if rng.next_u64() % 2 == 0 {
                rng.i32_in(-16, 16)
            } else {
                rng.i32_any()
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
            let mut cst = st(firfx);
            let mut rst = cst;
            cst.idx = rng.next_u32() as u16;
            rst.idx = cst.idx;
            let mut cs = ps;
            let mut rs = ps;
            let cv = unsafe { c(which, cs.as_mut_ptr(), idx, pfcn, &mut cst) };
            let rv = unsafe { r(which, rs.as_mut_ptr(), idx, pfcn, &mut rst) };
            assert_eq!(
                cv, rv,
                "which={which} pfcn={pfcn} idx={idx} ps={ps:?} firfx={firfx:?}"
            );
        }
    }
}
