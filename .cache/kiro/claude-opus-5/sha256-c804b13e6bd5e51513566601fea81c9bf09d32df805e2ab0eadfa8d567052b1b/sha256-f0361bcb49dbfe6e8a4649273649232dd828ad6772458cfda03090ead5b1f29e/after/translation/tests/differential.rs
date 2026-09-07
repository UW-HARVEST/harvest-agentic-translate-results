//! Phase B / Phase C differential tests against the PUBLIC exported surface.
//!
//! Loads both the C `.so` and the Rust `.so` with `libloading` and compares
//! `call_predict` -- the only symbol the C library exports. Nothing is called
//! directly; every call goes through `dlsym`, so the `#[no_mangle]` export
//! wrapper is under test too. Every test runs against the Rust cdylib built in
//! BOTH cargo profiles (see `common::rust_lib_paths`).
//!
//! Row numbers refer to `CONFIGS.md` (Phase B) and `ERRORS.md` (Phase C).

mod common;

use common::{PublicPair, Rng};

// ===========================================================================
// Phase B — CONFIGS.md rows 1..14
// ===========================================================================

mod configs {
    use super::*;

    /// Rows 1-12: each explicitly-handled `pfcn` case, `0..=11`.
    #[test]
    fn rows1_12_each_explicit_case() {
        for p in PublicPair::all() {
            for pfcn in 0..=11 {
                p.assert_same(pfcn);
                // Sanity: the C really does report a match for these.
                let (c, _) = p.call_predict(pfcn);
                assert_eq!(c, 1, "C call_predict({pfcn}) expected 1");
            }
        }
    }

    /// Row 13: exhaustive sweep across every case plus both `default:` labels.
    #[test]
    fn row13_exhaustive_sweep_minus64_to_64() {
        for p in PublicPair::all() {
            for pfcn in -64..=64 {
                p.assert_same(pfcn);
            }
        }
    }

    /// Row 14: 200 000 randomized 32-bit `pfcn` values, fixed seed, plus every
    /// power-of-two boundary and its immediate neighbours.
    #[test]
    fn row14_randomized_full_int_range() {
        for p in PublicPair::all() {
            let mut rng = Rng::new(0xC0FF_EE12_3456_789A);
            for _ in 0..200_000 {
                p.assert_same(rng.next_i32());
            }
            for bit in 0..31u32 {
                let v = 1i32 << bit;
                for delta in [-1, 0, 1] {
                    p.assert_same(v.wrapping_add(delta));
                    p.assert_same((-v).wrapping_add(delta));
                }
            }
            for v in [i32::MIN, i32::MIN + 1, i32::MAX, i32::MAX - 1, 0, -1] {
                p.assert_same(v);
            }
        }
    }
}

// ===========================================================================
// Phase C — ERRORS.md rows 1-4 and 10 (reachable through the public API)
// ===========================================================================

mod errors {
    use super::*;

    /// Row 1: `pfcn == -1`, one step below the valid range.
    #[test]
    fn row1_negative_one() {
        for p in PublicPair::all() {
            let (c, r) = p.call_predict(-1);
            assert_eq!(c, 0, "C must reject -1 with 0");
            assert_eq!(r, c, "Rust must reject -1 identically");
        }
    }

    /// Row 2: `pfcn == 12`, one step above the valid range, plus the rest of the
    /// FIR range the inner switch distinguishes and the first values past it.
    #[test]
    fn row2_twelve() {
        for p in PublicPair::all() {
            for pfcn in [12, 13, 14, 15, 16, 17] {
                let (c, r) = p.call_predict(pfcn);
                assert_eq!(c, 0, "C must reject {pfcn} with 0");
                assert_eq!(r, c, "Rust diverged at {pfcn}");
            }
        }
    }

    /// Row 3: `INT_MIN`.
    #[test]
    fn row3_int_min() {
        for p in PublicPair::all() {
            let (c, r) = p.call_predict(i32::MIN);
            assert_eq!(c, 0);
            assert_eq!(r, c);
        }
    }

    /// Row 4: `INT_MAX`.
    #[test]
    fn row4_int_max() {
        for p in PublicPair::all() {
            let (c, r) = p.call_predict(i32::MAX);
            assert_eq!(c, 0);
            assert_eq!(r, c);
        }
    }

    /// Row 10: `pfcn` is a bare `int`, so every 32-bit value is a legal input --
    /// including values no "enum variant" covers. Asserts the exact returned
    /// sentinel (`1` inside `0..=11`, `0` outside), not merely "both agree".
    #[test]
    fn row10_full_int_sweep_and_random() {
        for p in PublicPair::all() {
            for pfcn in -300..=300 {
                let (c, r) = p.call_predict(pfcn);
                let expected = if (0..=11).contains(&pfcn) { 1 } else { 0 };
                assert_eq!(c, expected, "C call_predict({pfcn})");
                assert_eq!(r, expected, "Rust call_predict({pfcn})");
            }
            let mut rng = Rng::new(0x5EED_0000_1111_2222);
            for _ in 0..100_000 {
                let pfcn = rng.next_i32();
                let (c, r) = p.call_predict(pfcn);
                let expected = if (0..=11).contains(&pfcn) { 1 } else { 0 };
                assert_eq!(c, expected, "C call_predict({pfcn})");
                assert_eq!(r, expected, "Rust call_predict({pfcn})");
            }
        }
    }
}
