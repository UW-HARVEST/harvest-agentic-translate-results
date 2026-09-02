//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. Every test calls BOTH the C `.so` and
//! the Rust `.so` through `libloading` and compares the returned `int`
//! byte-for-byte.

mod common;

use common::{c_source_oracle, Pair, Rng};

// ---------------------------------------------------------------------------
// Rows 1..=12 — one row per specialised `_PfnNN` predictor arm.
// ---------------------------------------------------------------------------

/// Helper: a single specialised arm, plus a randomized re-hit of that same
/// arm interleaved with neighbours so the call order varies.
fn check_specialised_arm(pfcn: i32) {
    let p = Pair::load();
    // The C's `case <pfcn>` compares the pointer returned by
    // BTAC1C2_GetPredictFunc against _Pfn<pfcn>, which always matches -> 1.
    p.assert_same_and_eq(pfcn, 1);
    p.assert_same_and_eq(pfcn, c_source_oracle(pfcn));

    // Randomized re-entry: hammer this arm interleaved with random other
    // inputs, to catch any order/state dependence in the identity compare.
    let mut rng = Rng::new(Rng::SEED ^ (pfcn as u64));
    for _ in 0..2_000 {
        let other = rng.in_range(-64, 64);
        p.assert_same(other);
        p.assert_same_and_eq(pfcn, 1);
    }
}

#[test]
fn cfg_row01_pfcn_0() {
    check_specialised_arm(0);
}

#[test]
fn cfg_row02_pfcn_1() {
    check_specialised_arm(1);
}

#[test]
fn cfg_row03_pfcn_2() {
    check_specialised_arm(2);
}

#[test]
fn cfg_row04_pfcn_3() {
    check_specialised_arm(3);
}

#[test]
fn cfg_row05_pfcn_4() {
    check_specialised_arm(4);
}

#[test]
fn cfg_row06_pfcn_5() {
    check_specialised_arm(5);
}

#[test]
fn cfg_row07_pfcn_6() {
    check_specialised_arm(6);
}

#[test]
fn cfg_row08_pfcn_7() {
    check_specialised_arm(7);
}

#[test]
fn cfg_row09_pfcn_8() {
    check_specialised_arm(8);
}

#[test]
fn cfg_row10_pfcn_9() {
    check_specialised_arm(9);
}

#[test]
fn cfg_row11_pfcn_10() {
    check_specialised_arm(10);
}

#[test]
fn cfg_row12_pfcn_11() {
    check_specialised_arm(11);
}

// ---------------------------------------------------------------------------
// Rows 13..=17 — the fallback region (12..=15 are the `firfx` FIR arms of
// BTAC1C2_PredictSample; 16 is past every internal `case`).
// ---------------------------------------------------------------------------

#[test]
fn cfg_row13_pfcn_12_fir_arm_0() {
    let p = Pair::load();
    p.assert_same_and_eq(12, 0);
}

#[test]
fn cfg_row14_pfcn_13_fir_arm_1() {
    let p = Pair::load();
    p.assert_same_and_eq(13, 0);
}

#[test]
fn cfg_row15_pfcn_14_fir_arm_2() {
    let p = Pair::load();
    p.assert_same_and_eq(14, 0);
}

#[test]
fn cfg_row16_pfcn_15_fir_arm_3() {
    let p = Pair::load();
    p.assert_same_and_eq(15, 0);
}

#[test]
fn cfg_row17_pfcn_16_past_all_cases() {
    let p = Pair::load();
    p.assert_same_and_eq(16, 0);
}

// ---------------------------------------------------------------------------
// Row 18 — exhaustive contiguous sweep over -2048..=2048.
// ---------------------------------------------------------------------------

#[test]
fn cfg_row18_exhaustive_sweep_pm_2048() {
    let p = Pair::load();
    for pfcn in -2048..=2048i32 {
        p.assert_same_and_eq(pfcn, c_source_oracle(pfcn));
    }
}

// ---------------------------------------------------------------------------
// Row 19 — 200 000 randomized values over the FULL i32 range, fixed seed.
// ---------------------------------------------------------------------------

#[test]
fn cfg_row19_random_full_i32_range() {
    let p = Pair::load();
    let mut rng = Rng::new(Rng::SEED);
    for _ in 0..200_000 {
        let pfcn = rng.next_i32();
        p.assert_same_and_eq(pfcn, c_source_oracle(pfcn));
    }
}

// ---------------------------------------------------------------------------
// Row 20 — dense randomized re-hit of the near-valid band.
// ---------------------------------------------------------------------------

#[test]
fn cfg_row20_random_near_valid_band() {
    let p = Pair::load();
    let mut rng = Rng::new(Rng::SEED ^ 0xDEAD_BEEF);
    for _ in 0..20_000 {
        let pfcn = rng.in_range(-64, 64);
        p.assert_same_and_eq(pfcn, c_source_oracle(pfcn));
    }
}

// ---------------------------------------------------------------------------
// Row 21 — extreme bands next to i32::MIN / i32::MAX.
// ---------------------------------------------------------------------------

#[test]
fn cfg_row21_random_extreme_bands() {
    let p = Pair::load();
    let mut rng = Rng::new(Rng::SEED ^ 0x0BAD_F00D);
    for _ in 0..20_000 {
        let lo = rng.in_range(i32::MIN, i32::MIN + 4096);
        p.assert_same_and_eq(lo, 0);
        let hi = rng.in_range(i32::MAX - 4096, i32::MAX);
        p.assert_same_and_eq(hi, 0);
    }
}

// ---------------------------------------------------------------------------
// Row 22 — statelessness: the valid set in randomized order, many times.
// ---------------------------------------------------------------------------

#[test]
fn cfg_row22_stateless_interleaved_valid_set() {
    let p = Pair::load();
    let mut rng = Rng::new(Rng::SEED ^ 0xFEED_FACE);
    for _ in 0..50_000 {
        let pfcn = rng.in_range(0, 11);
        p.assert_same_and_eq(pfcn, 1);
    }
    // And once more in strict order, to prove nothing drifted.
    for pfcn in 0..=11i32 {
        p.assert_same_and_eq(pfcn, 1);
    }
}

// ---------------------------------------------------------------------------
// Row 23 — concurrency: 4 threads driving both .so exports at once.
// ---------------------------------------------------------------------------

#[test]
fn cfg_row23_concurrent_four_threads() {
    use std::sync::Arc;

    struct Fns {
        c: common::GetPredictFunc,
        rust: common::GetPredictFunc,
    }
    // Raw `extern "C" fn` pointers are Send+Sync; the Library handles stay
    // alive on the main thread for the duration of the joins.
    unsafe impl Send for Fns {}
    unsafe impl Sync for Fns {}

    let p = Pair::load();
    let fns = Arc::new(Fns { c: p.c, rust: p.rust });

    let mut handles = Vec::new();
    for t in 0..4u64 {
        let fns = Arc::clone(&fns);
        handles.push(std::thread::spawn(move || {
            let mut rng = Rng::new(Rng::SEED ^ (t * 0x1234_5678_9ABC_DEF1));
            for _ in 0..25_000 {
                let pfcn = rng.in_range(-4096, 4096);
                let cv = unsafe { (fns.c)(pfcn) };
                let rv = unsafe { (fns.rust)(pfcn) };
                assert_eq!(cv, rv, "thread {t}: divergence at pfcn = {pfcn}");
                assert_eq!(cv, c_source_oracle(pfcn), "thread {t}: pfcn = {pfcn}");
            }
        }));
    }
    for h in handles {
        h.join().expect("worker thread panicked");
    }
    drop(p);
}
