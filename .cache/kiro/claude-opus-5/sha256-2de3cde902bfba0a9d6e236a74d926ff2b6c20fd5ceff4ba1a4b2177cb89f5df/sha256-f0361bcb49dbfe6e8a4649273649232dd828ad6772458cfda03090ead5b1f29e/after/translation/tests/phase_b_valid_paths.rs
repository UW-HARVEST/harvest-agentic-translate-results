//! Phase B — valid-path differential tests. One test per row of `CONFIGS.md`.
//!
//! Every assertion goes through `dlopen` on both the C `.so` and the Rust
//! `.so`; the Rust implementation is never called directly.

mod common;

use common::{Lcg, Pair};

/// The 12 values `BTAC1C2_GetPredictFunc` has a dedicated `case` for.
const VALID: [i32; 12] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11];

/// Rows 1..=12: each individual valid `pfcn`, its own code path in both
/// `BTAC1C2_GetPredictFunc` and `get_predict_func`.
macro_rules! valid_row {
    ($name:ident, $pfcn:expr, $row:expr) => {
        #[test]
        fn $name() {
            let p = Pair::load();
            let got = p.assert_same($pfcn);
            assert_eq!(
                got, 1,
                "CONFIGS.md row {}: C itself must select the matching _Pfn{} \
                 (ground truth), got {}",
                $row, $pfcn, got
            );
            // Repeat: stateless, so the answer must not drift.
            for _ in 0..64 {
                assert_eq!(p.assert_same($pfcn), got);
            }
        }
    };
}

valid_row!(cfg_row01_pfcn_0, 0, 1);
valid_row!(cfg_row02_pfcn_1, 1, 2);
valid_row!(cfg_row03_pfcn_2, 2, 3);
valid_row!(cfg_row04_pfcn_3, 3, 4);
valid_row!(cfg_row05_pfcn_4, 4, 5);
valid_row!(cfg_row06_pfcn_5, 5, 6);
valid_row!(cfg_row07_pfcn_6, 6, 7);
valid_row!(cfg_row08_pfcn_7, 7, 8);
valid_row!(cfg_row09_pfcn_8, 8, 9);
valid_row!(cfg_row10_pfcn_9, 9, 10);
valid_row!(cfg_row11_pfcn_10, 10, 11);
valid_row!(cfg_row12_pfcn_11, 11, 12);

/// Row 13: drive the whole valid domain end to end the way a consumer would —
/// ascending, descending, and pseudo-random interleavings — asserting the
/// result is order-independent (no hidden state / caching in the translation).
#[test]
fn cfg_row13_domain_sweep_orderings() {
    let p = Pair::load();

    // Ascending.
    for &v in VALID.iter() {
        assert_eq!(p.assert_same(v), 1, "ascending sweep, pfcn={v}");
    }
    // Descending.
    for &v in VALID.iter().rev() {
        assert_eq!(p.assert_same(v), 1, "descending sweep, pfcn={v}");
    }
    // Random interleaving, fixed seed.
    let mut rng = Lcg::new(0x5EED_1234_ABCD_0001);
    for _ in 0..20_000 {
        let v = VALID[rng.below(VALID.len() as u32) as usize];
        assert_eq!(p.assert_same(v), 1, "interleaved sweep, pfcn={v}");
    }
    // Interleave valid with invalid so a stateful bug would show up.
    for _ in 0..20_000 {
        let v = VALID[rng.below(VALID.len() as u32) as usize];
        let bad = rng.next_i32();
        p.assert_same(bad);
        assert_eq!(p.assert_same(v), 1, "valid after invalid {bad}, pfcn={v}");
    }
}

/// Row 14: randomized property sweep over the whole `i32` domain, fixed seed.
/// Biased so the valid domain, the boundaries, and masking aliases all get hit
/// frequently instead of being lost in 2^32.
#[test]
fn cfg_row14_randomized_full_domain() {
    let p = Pair::load();
    let mut rng = Lcg::new(0xC0FF_EE00_1234_5678);

    for _ in 0..200_000 {
        let raw = rng.next_i32();
        // Five biasing strategies, so ~4/5 of draws land near interesting values.
        let pfcn = match rng.below(5) {
            0 => raw,                              // anywhere in i32
            1 => raw % 32,                         // -31..=31, straddles the domain
            2 => (raw % 12).abs(),                 // strictly valid
            3 => raw & 0xF,                        // 0..=15: valid ∪ PredictSample-only
            _ => raw.wrapping_mul(16),             // multiples of 16: masking aliases
        };
        p.assert_same(pfcn);
    }
}

/// Row 15: the pointer-identity invariant. For every valid `pfcn` the C
/// returns 1, meaning `BTAC1C2_GetPredictFunc` handed back the *matching*
/// specialised predictor rather than the shared `BTAC1C2_PredictSample`
/// fallback. A Rust translation that merged distinct predictor addresses or
/// produced a fresh trampoline per call would fail this.
#[test]
fn cfg_row15_pointer_identity_invariant() {
    let p = Pair::load();
    for &v in VALID.iter() {
        assert_eq!(p.c(v), 1, "C ground truth: pfcn={v} must select _Pfn{v}");
        assert_eq!(
            p.rust(v),
            1,
            "Rust must also select the matching specialised predictor for pfcn={v}"
        );
    }
    // And the fallback must never masquerade as a match.
    for v in 12..=15 {
        assert_eq!(p.c(v), 0);
        assert_eq!(p.rust(v), 0);
    }
}

/// Cross-check that the two libraries really are two distinct objects (guards
/// against the harness accidentally loading the same `.so` twice and passing
/// trivially).
#[test]
fn harness_loads_two_distinct_libraries() {
    let c = common::c_lib_path();
    let r = common::rust_lib_path();
    assert_ne!(c, r, "C and Rust .so paths must differ");
    assert!(c.exists(), "missing {}", c.display());
    assert!(r.exists(), "missing {}", r.display());
    let cs = std::fs::read(&c).unwrap();
    let rs = std::fs::read(&r).unwrap();
    assert_ne!(cs, rs, "the two .so files must not be byte-identical");
}
