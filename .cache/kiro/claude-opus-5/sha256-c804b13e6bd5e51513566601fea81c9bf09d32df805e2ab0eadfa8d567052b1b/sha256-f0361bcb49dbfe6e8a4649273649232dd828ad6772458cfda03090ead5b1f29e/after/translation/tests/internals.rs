//! Phase B / Phase C differential tests against the LOWEST-LEVEL entry points.
//!
//! The C `.so` exports only `call_predict`; the twelve `BTAC1C2_PredictSample_Pfn*`
//! helpers, the generic `BTAC1C2_PredictSample`, and the `BTAC1C2_GetPredictFunc`
//! dispatcher are all `static`. Testing only `call_predict` would leave every
//! line of prediction arithmetic unverified, so:
//!
//!   * `harness/wrap.c` (outside `c_src/`, which stays untouched) textually
//!     includes `c_src/src/lib.c` and re-exports the statics as `wrap_*`;
//!   * the Rust crate's non-default `test_internals` feature re-exports its
//!     private helpers as `rsw_*`.
//!
//! Both are loaded via `libloading` and compared. Run with:
//!   cargo test --features test_internals
//!
//! Row numbers refer to `CONFIGS.md` (Phase B) and `ERRORS.md` (Phase C).

#![cfg(feature = "test_internals")]

mod common;

use common::*;
use libloading::Library;
use std::ffi::c_int;

/// The internals harness: the C wrapper library plus EVERY Rust cdylib artifact
/// (one per cargo profile), with all symbols resolved up-front through `dlsym`.
///
/// Each `assert_*` method compares the C result against every Rust artifact, so
/// a divergence that only appears under optimisation is still caught.
struct RustSyms {
    path: std::path::PathBuf,
    _lib: Library,
    predict_sample: FnPredict,
    pfn: [FnPredict; 12],
    index: FnInt,
    through: FnCallThrough,
}

struct InternalPair {
    _c: Library,
    c_predict_sample: FnPredict,
    c_pfn: [FnPredict; 12],
    c_index: FnInt,
    c_through: FnCallThrough,
    rust: Vec<RustSyms>,
}

macro_rules! pfns {
    ($lib:expr, $prefix:literal) => {{
        let mut out: [Option<FnPredict>; 12] = [None; 12];
        for n in 0..12usize {
            out[n] = Some(*sym::<FnPredict>(&$lib, &format!("{}{}", $prefix, n)));
        }
        out.map(|f| f.unwrap())
    }};
}

impl InternalPair {
    fn load() -> Self {
        let c = dlopen(&c_wrap_path());
        let c_predict_sample = *sym::<FnPredict>(&c, "wrap_predict_sample");
        let c_pfn = pfns!(c, "wrap_pfn");
        let c_index = *sym::<FnInt>(&c, "wrap_get_predict_func_index");
        let c_through = *sym::<FnCallThrough>(&c, "wrap_call_through");

        let mut rust = Vec::new();
        for path in rust_lib_paths() {
            let r = dlopen(path);
            let predict_sample = *sym::<FnPredict>(&r, "rsw_predict_sample");
            let pfn = pfns!(r, "rsw_pfn");
            let index = *sym::<FnInt>(&r, "rsw_get_predict_func_index");
            let through = *sym::<FnCallThrough>(&r, "rsw_call_through");

            // Struct layout must agree before any `firfx` access is meaningful.
            for (name_c, name_r) in [
                ("wrap_sizeof_idxstate", "rsw_sizeof_idxstate"),
                ("wrap_alignof_idxstate", "rsw_alignof_idxstate"),
                ("wrap_offsetof_firfx", "rsw_offsetof_firfx"),
                ("wrap_offsetof_usefx", "rsw_offsetof_usefx"),
                ("wrap_offsetof_lpred", "rsw_offsetof_lpred"),
            ] {
                let cv = unsafe { (*sym::<FnProbe>(&c, name_c))() };
                let rv = unsafe { (*sym::<FnProbe>(&r, name_r))() };
                assert_eq!(
                    cv,
                    rv,
                    "struct layout mismatch in {}: {name_c}={cv} vs {name_r}={rv}",
                    path.display()
                );
            }

            rust.push(RustSyms {
                path: path.clone(),
                _lib: r,
                predict_sample,
                pfn,
                index,
                through,
            });
        }
        assert!(!rust.is_empty(), "no Rust cdylib artifacts to test");

        InternalPair {
            _c: c,
            c_predict_sample,
            c_pfn,
            c_index,
            c_through,
            rust,
        }
    }

    /// `BTAC1C2_PredictSample(psamp, idx, pfcn, ridx)`: the C result and the
    /// (asserted-identical) result from every Rust artifact.
    fn predict_sample(
        &self,
        psamp: &[c_int; 8],
        idx: c_int,
        pfcn: c_int,
        st: &IdxState,
    ) -> (c_int, c_int) {
        // Each call gets its own mutable copies; these functions do not write,
        // but this rules out any cross-contamination.
        let mut pc = *psamp;
        let mut sc = *st;
        let cv = unsafe { (self.c_predict_sample)(pc.as_mut_ptr(), idx, pfcn, &mut sc) };
        let mut rv: Option<c_int> = None;
        for rs in &self.rust {
            let mut pr = *psamp;
            let mut sr = *st;
            let v = unsafe { (rs.predict_sample)(pr.as_mut_ptr(), idx, pfcn, &mut sr) };
            if let Some(prev) = rv {
                assert_eq!(
                    prev,
                    v,
                    "Rust artifacts disagree with each other ({} gave {v}) for \
                     PredictSample(psamp={psamp:?}, idx={idx}, pfcn={pfcn})",
                    rs.path.display()
                );
            }
            rv = Some(v);
        }
        (cv, rv.unwrap())
    }

    fn assert_predict_sample(&self, psamp: &[c_int; 8], idx: c_int, pfcn: c_int, st: &IdxState) {
        let mut pc = *psamp;
        let mut sc = *st;
        let cv = unsafe { (self.c_predict_sample)(pc.as_mut_ptr(), idx, pfcn, &mut sc) };
        for rs in &self.rust {
            let mut pr = *psamp;
            let mut sr = *st;
            let rv = unsafe { (rs.predict_sample)(pr.as_mut_ptr(), idx, pfcn, &mut sr) };
            assert_eq!(
                cv,
                rv,
                "BTAC1C2_PredictSample(psamp={psamp:?}, idx={idx}, pfcn={pfcn}, \
                 firfx={:?}): C={cv} Rust={rv} [{}]",
                st.firfx,
                rs.path.display()
            );
        }
    }

    /// `BTAC1C2_PredictSample_Pfn<n>`: the C result and the (asserted-identical)
    /// result from every Rust artifact.
    fn pfn_call(
        &self,
        n: usize,
        psamp: &[c_int; 8],
        idx: c_int,
        pfcn: c_int,
        st: &IdxState,
    ) -> (c_int, c_int) {
        let mut pc = *psamp;
        let mut sc = *st;
        let cv = unsafe { (self.c_pfn[n])(pc.as_mut_ptr(), idx, pfcn, &mut sc) };
        let mut rv: Option<c_int> = None;
        for rs in &self.rust {
            let mut pr = *psamp;
            let mut sr = *st;
            let v = unsafe { (rs.pfn[n])(pr.as_mut_ptr(), idx, pfcn, &mut sr) };
            if let Some(prev) = rv {
                assert_eq!(
                    prev,
                    v,
                    "Rust artifacts disagree with each other ({} gave {v}) for \
                     Pfn{n}(psamp={psamp:?}, idx={idx})",
                    rs.path.display()
                );
            }
            rv = Some(v);
        }
        (cv, rv.unwrap())
    }

    fn assert_pfn(&self, n: usize, psamp: &[c_int; 8], idx: c_int, pfcn: c_int, st: &IdxState) {
        let (c, r) = self.pfn_call(n, psamp, idx, pfcn, st);
        assert_eq!(
            c, r,
            "Pfn{n}(psamp={psamp:?}, idx={idx}, pfcn={pfcn}): C={c} Rust={r}"
        );
    }

    fn assert_index(&self, pfcn: c_int) -> c_int {
        let cv = unsafe { (self.c_index)(pfcn) };
        assert_ne!(cv, -2, "C dispatcher returned an unrecognised pointer");
        for rs in &self.rust {
            let rv = unsafe { (rs.index)(pfcn) };
            assert_eq!(
                cv,
                rv,
                "GetPredictFunc({pfcn}) identity: C selected {cv}, Rust selected {rv} [{}]",
                rs.path.display()
            );
        }
        cv
    }

    fn assert_through(&self, pfcn: c_int, psamp: &[c_int; 8], idx: c_int, st: &IdxState) {
        let mut pc = *psamp;
        let mut sc = *st;
        let cv = unsafe { (self.c_through)(pfcn, pc.as_mut_ptr(), idx, &mut sc) };
        for rs in &self.rust {
            let mut pr = *psamp;
            let mut sr = *st;
            let rv = unsafe { (rs.through)(pfcn, pr.as_mut_ptr(), idx, &mut sr) };
            assert_eq!(
                cv,
                rv,
                "dispatch+call pfcn={pfcn} psamp={psamp:?} idx={idx}: C={cv} Rust={rv} [{}]",
                rs.path.display()
            );
        }
    }

    /// Raw guarded-arena call used by the out-of-range-index test: returns the C
    /// result and each Rust artifact's result, with the caller's own buffers.
    fn predict_sample_raw(
        &self,
        arena_c: &mut [c_int],
        arena_r: &mut [c_int],
        offset: usize,
        idx: c_int,
        pfcn: c_int,
        st: &IdxState,
    ) -> (c_int, c_int) {
        let mut sc = *st;
        let cv = unsafe {
            (self.c_predict_sample)(arena_c.as_mut_ptr().add(offset), idx, pfcn, &mut sc)
        };
        let mut rv: Option<c_int> = None;
        for rs in &self.rust {
            let mut sr = *st;
            let v =
                unsafe { (rs.predict_sample)(arena_r.as_mut_ptr().add(offset), idx, pfcn, &mut sr) };
            if let Some(prev) = rv {
                assert_eq!(prev, v, "Rust artifacts disagree ({})", rs.path.display());
            }
            rv = Some(v);
        }
        (cv, rv.unwrap())
    }
}

// ---------------------------------------------------------------------------
// input generators
// ---------------------------------------------------------------------------

fn small_samples(rng: &mut Rng) -> [c_int; 8] {
    let mut a = [0i32; 8];
    for v in a.iter_mut() {
        *v = rng.small_i32();
    }
    a
}

fn full_samples(rng: &mut Rng) -> [c_int; 8] {
    let mut a = [0i32; 8];
    for v in a.iter_mut() {
        *v = rng.next_i32();
    }
    a
}

fn random_state(rng: &mut Rng) -> IdxState {
    let mut st = IdxState::default();
    st.idx = rng.next_u64() as u16;
    st.lpred = rng.next_i16();
    st.rpred = rng.next_i16();
    st.tag = rng.next_u64() as u8;
    st.bcfcn = rng.next_u64() as u8;
    st.bsfcn = rng.next_u64() as u8;
    st.usefx = rng.next_u64() as u8;
    for row in st.firfx.iter_mut() {
        for c in row.iter_mut() {
            // Mix ordinary coefficients with the s16 extremes.
            *c = match rng.below(8) {
                0 => i16::MIN,
                1 => i16::MAX,
                2 => 0,
                _ => rng.next_i16(),
            };
        }
    }
    st
}

/// The `psamp` shapes the arithmetic special-cases: zeros, the signed extremes,
/// alternating extremes, and single-hot extremes.
fn extreme_sample_sets() -> Vec<[c_int; 8]> {
    let mut v: Vec<[c_int; 8]> = vec![
        [0; 8],
        [i32::MAX; 8],
        [i32::MIN; 8],
        [1; 8],
        [-1; 8],
        [i32::MAX, i32::MIN, i32::MAX, i32::MIN, i32::MAX, i32::MIN, i32::MAX, i32::MIN],
        [i32::MIN, i32::MAX, i32::MIN, i32::MAX, i32::MIN, i32::MAX, i32::MIN, i32::MAX],
        [0, i32::MAX, 0, i32::MIN, 0, i32::MAX, 0, i32::MIN],
        [1 << 30; 8],
        [-(1 << 30); 8],
        [0x7FFF, -0x8000, 0x7FFF, -0x8000, 0x7FFF, -0x8000, 0x7FFF, -0x8000],
    ];
    for hot in 0..8usize {
        let mut a = [0i32; 8];
        a[hot] = i32::MAX;
        v.push(a);
        let mut b = [0i32; 8];
        b[hot] = i32::MIN;
        v.push(b);
    }
    v
}

const EXTREME_IDX: [c_int; 14] = [
    0,
    1,
    -1,
    2,
    -2,
    7,
    8,
    -7,
    -8,
    9,
    -9,
    i32::MIN,
    i32::MAX,
    i32::MIN + 1,
];

// ===========================================================================
// Phase B — CONFIGS.md rows 15..25
// ===========================================================================

mod configs {
    use super::*;

    /// Row 15: `BTAC1C2_PredictSample`, `pfcn` in `0..=11`, small mixed-sign
    /// samples, random `idx`.
    #[test]
    fn row15_predict_sample_small_random() {
        let p = InternalPair::load();
        let mut rng = Rng::new(0x1111_2222_3333_4444);
        let st = IdxState::default();
        for _ in 0..20_000 {
            let psamp = small_samples(&mut rng);
            let idx = rng.next_i32();
            for pfcn in 0..=11 {
                p.assert_predict_sample(&psamp, idx, pfcn, &st);
            }
        }
    }

    /// Row 16: same, but samples drawn from the full 32-bit range -- exercises
    /// signed overflow in the multiply/accumulate, `>>` on negative values, and
    /// truncating division.
    #[test]
    fn row16_predict_sample_full_range_random() {
        let p = InternalPair::load();
        let mut rng = Rng::new(0xABCD_EF01_2345_6789);
        let st = IdxState::default();
        for _ in 0..20_000 {
            let psamp = full_samples(&mut rng);
            let idx = rng.next_i32();
            for pfcn in 0..=11 {
                p.assert_predict_sample(&psamp, idx, pfcn, &st);
            }
        }
    }

    /// Row 17: the hand-built extreme `psamp` shapes, crossed with `pfcn 0..=11`
    /// and every boundary `idx`.
    #[test]
    fn row17_predict_sample_extreme_shapes() {
        let p = InternalPair::load();
        let st = IdxState::default();
        for psamp in extreme_sample_sets() {
            for &idx in EXTREME_IDX.iter() {
                for pfcn in 0..=11 {
                    p.assert_predict_sample(&psamp, idx, pfcn, &st);
                }
            }
        }
    }

    /// Row 18: the FIR arms (`pfcn 12..=15`), which are the only ones that read
    /// `ridx->firfx`. Randomized coefficients including the `s16` extremes.
    #[test]
    fn row18_predict_sample_fir_arms() {
        let p = InternalPair::load();
        let mut rng = Rng::new(0x0F1E_2D3C_4B5A_6978);
        for _ in 0..20_000 {
            let st = random_state(&mut rng);
            let psamp = if rng.below(2) == 0 {
                small_samples(&mut rng)
            } else {
                full_samples(&mut rng)
            };
            let idx = rng.next_i32();
            for pfcn in 12..=15 {
                p.assert_predict_sample(&psamp, idx, pfcn, &st);
            }
        }
        // Extreme sample shapes against extreme coefficient rows.
        let mut st = IdxState::default();
        for (r, row) in st.firfx.iter_mut().enumerate() {
            for (c, coef) in row.iter_mut().enumerate() {
                *coef = match (r + c) % 4 {
                    0 => i16::MIN,
                    1 => i16::MAX,
                    2 => -1,
                    _ => 1,
                };
            }
        }
        for psamp in extreme_sample_sets() {
            for &idx in EXTREME_IDX.iter() {
                for pfcn in 12..=15 {
                    p.assert_predict_sample(&psamp, idx, pfcn, &st);
                }
            }
        }
    }

    /// Row 19: `BTAC1C2_PredictSample`'s `default:` arm (`pfcn` outside `0..=15`).
    #[test]
    fn row19_predict_sample_default_arm() {
        let p = InternalPair::load();
        let mut rng = Rng::new(0x2222_3333_4444_5555);
        for _ in 0..5_000 {
            let st = random_state(&mut rng);
            let psamp = full_samples(&mut rng);
            let idx = rng.next_i32();
            for pfcn in [-1000, -16, -2, -1, 16, 17, 100, i32::MIN, i32::MAX] {
                p.assert_predict_sample(&psamp, idx, pfcn, &st);
            }
            p.assert_predict_sample(&psamp, idx, rng.next_i32(), &st);
        }
    }

    /// Row 20: `idx` boundary shapes for every `pfcn` in `0..=15`.
    #[test]
    fn row20_idx_boundaries() {
        let p = InternalPair::load();
        let mut rng = Rng::new(0x9999_8888_7777_6666);
        let st = random_state(&mut rng);
        let psamp: [c_int; 8] = [7, -13, 1000, -999, 32767, -32768, 5, -5];
        for &idx in EXTREME_IDX.iter() {
            for pfcn in 0..=15 {
                p.assert_predict_sample(&psamp, idx, pfcn, &st);
            }
        }
        for _ in 0..20_000 {
            let idx = rng.next_i32();
            for pfcn in 0..=15 {
                p.assert_predict_sample(&psamp, idx, pfcn, &st);
            }
        }
    }

    /// Row 21: each specialised `Pfn<n>` helper, small samples, random `idx`.
    #[test]
    fn row21_pfn_helpers_small_random() {
        let p = InternalPair::load();
        let mut rng = Rng::new(0x1357_9BDF_0246_8ACE);
        let st = IdxState::default();
        for _ in 0..20_000 {
            let psamp = small_samples(&mut rng);
            let idx = rng.next_i32();
            let pfcn = rng.next_i32();
            for n in 0..12usize {
                p.assert_pfn(n, &psamp, idx, pfcn, &st);
            }
        }
    }

    /// Row 22: each specialised helper with full-32-bit samples. The helpers are
    /// NOT all equivalent to the matching `switch` arm -- `Pfn10` shifts by 3
    /// where `case 10:` shifts by 4, and `Pfn11` shifts by 1 where `case 11:`
    /// shifts by 3. This test pins the helper behaviour, and additionally
    /// asserts those two really do disagree with the switch in BOTH libraries.
    #[test]
    fn row22_pfn_helpers_full_range_random() {
        let p = InternalPair::load();
        let mut rng = Rng::new(0xFEDC_BA98_7654_3210);
        let st = IdxState::default();
        let mut saw_pfn10_mismatch = false;
        let mut saw_pfn11_mismatch = false;
        for _ in 0..20_000 {
            let psamp = full_samples(&mut rng);
            let idx = rng.next_i32();
            for n in 0..12usize {
                p.assert_pfn(n, &psamp, idx, n as c_int, &st);
            }
            // Cross-check the documented C inconsistencies, identically on both sides.
            for n in [10usize, 11usize] {
                let (via_helper_c, via_helper_r) = p.pfn_call(n, &psamp, idx, n as c_int, &st);
                let (sw_c, sw_r) = p.predict_sample(&psamp, idx, n as c_int, &st);
                assert_eq!(sw_c, sw_r);
                assert_eq!(
                    via_helper_c, via_helper_r,
                    "Pfn{n} helper diverged (psamp={psamp:?}, idx={idx})"
                );
                assert_eq!(
                    via_helper_c != sw_c,
                    via_helper_r != sw_r,
                    "helper-vs-switch agreement for arm {n} differs between C and Rust"
                );
                if via_helper_c != sw_c {
                    if n == 10 {
                        saw_pfn10_mismatch = true;
                    } else {
                        saw_pfn11_mismatch = true;
                    }
                }
            }
        }
        assert!(
            saw_pfn10_mismatch,
            "expected Pfn10 (>>3) to differ from case 10 (>>4) for some input"
        );
        assert!(
            saw_pfn11_mismatch,
            "expected Pfn11 (>>1) to differ from case 11 (>>3) for some input"
        );
    }

    /// Row 23: each specialised helper over the extreme `psamp` x extreme `idx`
    /// cross-product.
    #[test]
    fn row23_pfn_helpers_extremes() {
        let p = InternalPair::load();
        let st = IdxState::default();
        for psamp in extreme_sample_sets() {
            for &idx in EXTREME_IDX.iter() {
                for n in 0..12usize {
                    p.assert_pfn(n, &psamp, idx, n as c_int, &st);
                }
            }
        }
    }

    /// Row 24: the dispatcher's identity -- which helper `BTAC1C2_GetPredictFunc`
    /// hands back -- swept over `pfcn` and checked against the C's own mapping.
    #[test]
    fn row24_dispatcher_identity() {
        let p = InternalPair::load();
        for pfcn in -64..=64 {
            let selected = p.assert_index(pfcn);
            let expected = if (0..=11).contains(&pfcn) { pfcn } else { -1 };
            assert_eq!(selected, expected, "dispatcher mapping for pfcn={pfcn}");
        }
        let mut rng = Rng::new(0x4242_4242_4242_4242);
        for _ in 0..50_000 {
            let pfcn = rng.next_i32();
            let selected = p.assert_index(pfcn);
            let expected = if (0..=11).contains(&pfcn) { pfcn } else { -1 };
            assert_eq!(selected, expected, "dispatcher mapping for pfcn={pfcn}");
        }
    }

    /// Row 25: the composed pipeline a real consumer would run -- dispatch, then
    /// call THROUGH the returned function pointer. This is the path per-function
    /// tests cannot see, because for `pfcn` outside `0..=11` the dispatcher hands
    /// back the generic `BTAC1C2_PredictSample`, which is then invoked with that
    /// same out-of-range `pfcn`.
    #[test]
    fn row25_dispatch_then_call_through() {
        let p = InternalPair::load();
        let mut rng = Rng::new(0xDEAD_BEEF_CAFE_BABE);
        for _ in 0..20_000 {
            let st = random_state(&mut rng);
            let psamp = if rng.below(2) == 0 {
                small_samples(&mut rng)
            } else {
                full_samples(&mut rng)
            };
            let idx = rng.next_i32();
            for pfcn in -20..=20 {
                p.assert_through(pfcn, &psamp, idx, &st);
            }
        }
        // Boundary `idx`/`psamp` shapes through the composed path.
        let st = IdxState::default();
        for psamp in extreme_sample_sets() {
            for &idx in EXTREME_IDX.iter() {
                for pfcn in -18..=18 {
                    p.assert_through(pfcn, &psamp, idx, &st);
                }
            }
        }
    }
}

// ===========================================================================
// Phase C — ERRORS.md rows 5..8 (reachable only via the internals harness)
// ===========================================================================

mod errors {
    use super::*;

    /// Row 5: `BTAC1C2_GetPredictFunc`'s `default:` must yield the GENERIC
    /// `BTAC1C2_PredictSample`, not any `Pfn*` helper. Encoded as `-1`.
    #[test]
    fn row5_dispatch_default_is_generic() {
        let p = InternalPair::load();
        for pfcn in [-1, -2, -100, 12, 13, 14, 15, 16, 1000, i32::MIN, i32::MAX] {
            let c = p.assert_index(pfcn);
            assert_eq!(
                c, -1,
                "GetPredictFunc({pfcn}) must select the generic dispatcher"
            );
        }
        for pfcn in 0..=11 {
            assert_eq!(p.assert_index(pfcn), pfcn);
        }
    }

    /// Row 6: `BTAC1C2_PredictSample`'s `default:` returns exactly `0`,
    /// independently of `psamp`, `idx` and `ridx`.
    #[test]
    fn row6_predict_sample_default_returns_zero() {
        let p = InternalPair::load();
        let mut rng = Rng::new(0x0BAD_C0DE_0BAD_C0DE);
        for _ in 0..2_000 {
            let st = random_state(&mut rng);
            let psamp = full_samples(&mut rng);
            let idx = rng.next_i32();
            for pfcn in [-1, -2, -16, -17, 16, 17, 18, 12345, i32::MIN, i32::MAX] {
                let (c, r) = p.predict_sample(&psamp, idx, pfcn, &st);
                assert_eq!(c, 0, "C default arm must return 0 (pfcn={pfcn})");
                assert_eq!(r, 0, "Rust default arm must return 0 (pfcn={pfcn})");
            }
        }
    }

    /// Row 7: `case 12..=15` indexes `firfx[pfcn - 12]` with no bounds check. All
    /// four rows must be selected identically, and the boundary values `11` and
    /// `16` must NOT enter the FIR path.
    #[test]
    fn row7_fir_arms_12_to_15() {
        let p = InternalPair::load();
        // Distinct per-row coefficients: only the correct row can produce the
        // expected value, so a mis-indexed row is caught.
        let mut st = IdxState::default();
        for (r, row) in st.firfx.iter_mut().enumerate() {
            for coef in row.iter_mut() {
                *coef = (r as i16 + 1) * 100;
            }
        }
        let psamp: [c_int; 8] = [1, 2, 3, 4, 5, 6, 7, 8];
        let mut seen = Vec::new();
        for pfcn in 12..=15 {
            let (c, r) = p.predict_sample(&psamp, 0, pfcn, &st);
            assert_eq!(c, r, "FIR arm {pfcn} diverged");
            seen.push(c);
        }
        assert_eq!(
            seen.len(),
            seen.iter().collect::<std::collections::HashSet<_>>().len(),
            "the four FIR rows must give four distinct results, got {seen:?}"
        );
        // One step either side of the FIR range must not read firfx.
        let (c11, r11) = p.predict_sample(&psamp, 0, 11, &st);
        assert_eq!(c11, r11);
        let (c16, r16) = p.predict_sample(&psamp, 0, 16, &st);
        assert_eq!(c16, r16);
        assert_eq!(c16, 0, "pfcn=16 falls into default:, must be 0");
        // Randomized coefficients across all four rows.
        let mut rng = Rng::new(0x7777_7777_7777_7777);
        for _ in 0..10_000 {
            let st = random_state(&mut rng);
            let psamp = full_samples(&mut rng);
            let idx = rng.next_i32();
            for pfcn in 12..=15 {
                p.assert_predict_sample(&psamp, idx, pfcn, &st);
            }
        }
    }

    /// Row 8: `psamp[(idx - k) & 7]` is masked, so no `idx` -- however extreme --
    /// can index out of the eight-element buffer. Verified by running every
    /// prediction arm with the buffer sandwiched between poison guard values: if
    /// either implementation read out of bounds it would pick up a guard and the
    /// two sides would (at minimum) have to agree on it, and the results below
    /// are additionally checked against a guard-free buffer.
    #[test]
    fn row8_extreme_idx_never_out_of_range() {
        let p = InternalPair::load();
        let mut rng = Rng::new(0x1234_5678_9ABC_DEF0);
        let st = random_state(&mut rng);

        // 24-element arena: 8 guard | 8 payload | 8 guard.
        let payload: [c_int; 8] = [11, -22, 33, -44, 55, -66, 77, -88];
        let mut arena_c = [0i32; 24];
        let mut arena_r = [0i32; 24];
        for i in 0..24 {
            let poison = if i % 2 == 0 { 0x5A5A_5A5A } else { -0x5A5A_5A5B };
            arena_c[i] = poison;
            arena_r[i] = poison;
        }
        arena_c[8..16].copy_from_slice(&payload);
        arena_r[8..16].copy_from_slice(&payload);

        let mut idxs: Vec<c_int> = EXTREME_IDX.to_vec();
        for _ in 0..5_000 {
            idxs.push(rng.next_i32());
        }
        for bit in 0..31u32 {
            idxs.push(1i32 << bit);
            idxs.push(-(1i32 << bit));
        }

        for &idx in &idxs {
            for pfcn in -4..=19 {
                // Guarded arena, pointer aimed at the payload.
                let (gc, gr) =
                    p.predict_sample_raw(&mut arena_c, &mut arena_r, 8, idx, pfcn, &st);
                assert_eq!(gc, gr, "guarded arena diverged (idx={idx}, pfcn={pfcn})");
                // Tight 8-element buffer must give the identical answer, proving
                // no guard element was ever read.
                let (tc, tr) = p.predict_sample(&payload, idx, pfcn, &st);
                assert_eq!(tc, tr);
                assert_eq!(
                    gc, tc,
                    "out-of-range read detected: guarded={gc} tight={tc} (idx={idx}, pfcn={pfcn})"
                );
                // Guards must be untouched.
                assert_eq!(&arena_c[0..8], &arena_r[0..8]);
                assert_eq!(&arena_c[16..24], &arena_r[16..24]);
            }
        }
    }
}
