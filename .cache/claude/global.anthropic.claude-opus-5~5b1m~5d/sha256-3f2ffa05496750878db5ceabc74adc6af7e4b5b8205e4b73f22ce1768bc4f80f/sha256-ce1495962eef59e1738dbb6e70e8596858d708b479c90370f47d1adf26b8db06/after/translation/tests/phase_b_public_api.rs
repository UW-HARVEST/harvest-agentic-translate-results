//! Phase B — valid-path differential tests for the **public** entry point.
//!
//! Rows C1..C5 and C37 of `CONFIGS.md`.  Every call goes through `dlsym` on both
//! shared objects; no Rust function is called directly.

mod common;
use common::*;

/// C1 — `pfcn` = each of `0..=11` exhaustively (every specialised arm of both
/// `switch (pfcn)` statements in `get_predict_func` / `BTAC1C2_GetPredictFunc`).
#[test]
fn c1_every_specialised_arm() {
    for pfcn in 0..=11 {
        assert_gpf_eq(pfcn, "C1");
    }
    // Ground truth: each specialised arm must report a match.
    let c = c_get_predict_func();
    for pfcn in 0..=11 {
        assert_eq!(unsafe { c(pfcn) }, 1, "C1: C get_predict_func({pfcn})");
    }
}

/// C2 — exhaustive over `-4096..=4096`, crossing every boundary of both
/// switches (`-1/0`, `11/12`, `15/16`).
#[test]
fn c2_exhaustive_small_range() {
    for pfcn in -4096..=4096 {
        assert_gpf_eq(pfcn, "C2");
    }
}

/// C3 — randomized full-range `int`, fixed seed, 20000 draws.
#[test]
fn c3_randomized_full_range() {
    let mut rng = Rng::new(0xC3_5EED);
    for _ in 0..20_000 {
        assert_gpf_eq(rng.i32_any(), "C3 uniform");
    }
    let mut rng = Rng::new(0xC3_5EED_2);
    for _ in 0..20_000 {
        assert_gpf_eq(rng.i32_shaped(), "C3 shaped");
    }
    // Bias a chunk of draws into the neighbourhood of the interesting
    // boundaries so the sweep does not consist almost entirely of `default`.
    let mut rng = Rng::new(0xC3_5EED_3);
    for _ in 0..20_000 {
        assert_gpf_eq(rng.i32_in(-40, 40), "C3 near-boundary");
    }
}

/// C4 — the extreme and one-past-boundary values.
#[test]
fn c4_extremes_and_boundaries() {
    let cases = [
        i32::MIN,
        i32::MIN + 1,
        i32::MIN + 12,
        -100,
        -2,
        -1,
        0,
        1,
        10,
        11,
        12,
        13,
        14,
        15,
        16,
        17,
        100,
        i32::MAX - 12,
        i32::MAX - 1,
        i32::MAX,
    ];
    for &pfcn in &cases {
        assert_gpf_eq(pfcn, "C4");
    }
}

/// C5 — `BTAC1C2_GetPredictFunc`'s selector behaviour, observed through the
/// wrapper: exactly `0..=11` yield 1 and nothing else does, on **both** sides.
#[test]
fn c5_selector_indicator_shape() {
    let c = c_get_predict_func();
    let r = rust_get_predict_func();
    for pfcn in -200..=200 {
        let expect = if (0..=11).contains(&pfcn) { 1 } else { 0 };
        let cv = unsafe { c(pfcn) };
        let rv = unsafe { r(pfcn) };
        assert_eq!(cv, expect, "C5: C get_predict_func({pfcn})");
        assert_eq!(rv, expect, "C5: Rust get_predict_func({pfcn})");
    }
}

/// C37 — the public result is identical for the CMake ground-truth artifact and
/// for the separately-compiled shim of the same untouched `lib.c`, and the Rust
/// `.so` agrees with both.  This pins the behaviour as compile-invariant rather
/// than an artifact of one particular gcc invocation.
#[test]
fn c37_compile_invariant_public_result() {
    let c = c_get_predict_func();
    let cs = c_shim_get_predict_func();
    let r = rust_get_predict_func();
    for pfcn in -300..=300 {
        let a = unsafe { c(pfcn) };
        let b = unsafe { cs(pfcn) };
        let d = unsafe { r(pfcn) };
        assert_eq!(a, b, "C37: cmake C vs shim C at pfcn={pfcn}");
        assert_eq!(a, d, "C37: C vs Rust at pfcn={pfcn}");
    }
    let mut rng = Rng::new(0xC37);
    for _ in 0..5_000 {
        let pfcn = rng.i32_any();
        assert_eq!(unsafe { c(pfcn) }, unsafe { cs(pfcn) }, "C37 rnd {pfcn}");
        assert_eq!(unsafe { c(pfcn) }, unsafe { r(pfcn) }, "C37 rnd {pfcn}");
    }
}

/// Repeated / interleaved invocation: the library is stateless, so results must
/// not drift across calls or depend on call order.
#[test]
fn public_api_is_stateless() {
    let c = c_get_predict_func();
    let r = rust_get_predict_func();
    let mut rng = Rng::new(0xA11_5EED);
    let mut first: Vec<(i32, i32)> = Vec::new();
    for _ in 0..2_000 {
        let pfcn = rng.i32_in(-32, 32);
        first.push((pfcn, unsafe { r(pfcn) }));
    }
    // Replay in reverse; every result must be unchanged and match C.
    for &(pfcn, want) in first.iter().rev() {
        assert_eq!(unsafe { r(pfcn) }, want, "Rust not stateless at {pfcn}");
        assert_eq!(unsafe { c(pfcn) }, want, "C disagrees at {pfcn}");
    }
}
