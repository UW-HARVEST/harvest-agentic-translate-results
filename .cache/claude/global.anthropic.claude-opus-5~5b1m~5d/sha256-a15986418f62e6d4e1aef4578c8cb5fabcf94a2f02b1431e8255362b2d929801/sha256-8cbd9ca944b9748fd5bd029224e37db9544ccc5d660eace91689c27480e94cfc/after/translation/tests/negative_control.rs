//! Negative controls: prove the differential harness can actually FAIL.
//!
//! A suite that passes because it never really compares anything is worthless.
//! These tests show that (a) both `.so`s are genuinely loaded and invoked,
//! (b) the bit-comparison is sensitive to a one-bit difference, and (c) a
//! plausible mis-translation is caught.

mod harness;

use harness::*;

#[test]
fn both_libraries_are_really_invoked() {
    // A value only a real barycentric computation produces.
    let r_c = call_c(
        Vec2::new(0.0, 0.0),
        Vec2::new(1.0, 0.0),
        Vec2::new(0.0, 1.0),
        Vec2::new(0.25, 0.75),
    );
    let r_r = call_rust(
        Vec2::new(0.0, 0.0),
        Vec2::new(1.0, 0.0),
        Vec2::new(0.0, 1.0),
        Vec2::new(0.25, 0.75),
    );
    // (u along p3-p1, v along p2-p1) = (0.75, 0.25) — not the input, not zero.
    assert_eq!(r_c.bits(), (0.75f32.to_bits(), 0.25f32.to_bits()), "C .so");
    assert_eq!(r_r.bits(), (0.75f32.to_bits(), 0.25f32.to_bits()), "Rust .so");
    assert_ne!(r_c.bits(), (0, 0), "results must not be trivially zero");
}

#[test]
fn comparison_detects_a_one_bit_difference() {
    let a = Vec2::from_bits(0x3f80_0000, 0x4000_0000);
    let b = Vec2::from_bits(0x3f80_0001, 0x4000_0000);
    assert_ne!(a.bits(), b.bits());
    // and NaN payloads are compared, unlike `==` on floats
    let n1 = Vec2::from_bits(0x7fc0_0001, 0x7fc0_0001);
    let n2 = Vec2::from_bits(0x7fc0_0002, 0x7fc0_0001);
    assert_ne!(n1.bits(), n2.bits());
    assert!(f32::from_bits(0x7fc0_0001).is_nan() && f32::from_bits(0x7fc0_0002).is_nan());
}

/// A deliberately WRONG translation: `u` and `v` swapped. The harness must be
/// able to tell it apart from the real one on real data, otherwise a swapped
/// translation would have slipped through Phase B.
fn wrong_swapped(p1: Vec2, p2: Vec2, p3: Vec2, p: Vec2) -> Vec2 {
    let r = call_c(p1, p2, p3, p);
    Vec2::new(r.y, r.x)
}

/// Another wrong translation: `f64` intermediates instead of `f32`.
fn wrong_double_precision(p1: Vec2, p2: Vec2, p3: Vec2, p: Vec2) -> Vec2 {
    let d = |a: Vec2, b: Vec2| (a.x as f64 - b.x as f64, a.y as f64 - b.y as f64);
    let dot = |a: (f64, f64), b: (f64, f64)| a.0 * b.0 + a.1 * b.1;
    let v0 = d(p3, p1);
    let v1 = d(p2, p1);
    let v2 = d(p, p1);
    let (d00, d01, d02, d11, d12) = (
        dot(v0, v0),
        dot(v0, v1),
        dot(v0, v2),
        dot(v1, v1),
        dot(v1, v2),
    );
    let inv = 1.0f64 / (d00 * d11 - d01 * d01);
    Vec2::new(
        ((d11 * d02 - d01 * d12) * inv) as f32,
        ((d00 * d12 - d01 * d02) * inv) as f32,
    )
}

#[test]
fn harness_would_catch_a_swapped_uv_translation() {
    let mut rng = Rng::new(0xDEAD);
    let mut caught = 0usize;
    for _ in 0..2_000 {
        let args = [
            rng.normal_vec(),
            rng.normal_vec(),
            rng.normal_vec(),
            rng.normal_vec(),
        ];
        let good = call_rust(args[0], args[1], args[2], args[3]);
        let bad = wrong_swapped(args[0], args[1], args[2], args[3]);
        if good.bits() != bad.bits() {
            caught += 1;
        }
    }
    assert!(
        caught > 1_900,
        "the bit comparison only distinguished a u/v swap in {caught}/2000 cases"
    );
}

#[test]
fn harness_would_catch_an_f64_intermediate_translation() {
    let mut rng = Rng::new(0xBEEF);
    let mut caught = 0usize;
    let total = 20_000;
    for _ in 0..total {
        let args = [
            rng.normal_vec(),
            rng.normal_vec(),
            rng.normal_vec(),
            rng.normal_vec(),
        ];
        let good = call_rust(args[0], args[1], args[2], args[3]);
        let bad = wrong_double_precision(args[0], args[1], args[2], args[3]);
        if good.bits() != bad.bits() {
            caught += 1;
        }
    }
    assert!(
        caught > total / 20,
        "an f64-intermediate mis-translation was only distinguished in {caught}/{total} cases — \
         the value generator is not exercising rounding-sensitive inputs"
    );
    eprintln!("f64-intermediate divergence detected in {caught}/{total} random cases");
}

#[test]
fn harness_would_catch_a_naive_nan_translation() {
    // The whole point of `mul_dst_lhs` / `mul_dst_rhs` / `add_dst_rhs`: a naive
    // `a.x*b.x + a.y*b.y` in Rust picks a DIFFERENT NaN payload than the C's
    // unoptimised codegen for some multi-NaN inputs. Show that such inputs
    // exist and are reachable by the generators used in Phase B/C, so C17-C19
    // and E8-E9 are meaningful rather than vacuous.
    fn naive(p1: Vec2, p2: Vec2, p3: Vec2, p: Vec2) -> Vec2 {
        let sub = |a: Vec2, b: Vec2| Vec2::new(a.x - b.x, a.y - b.y);
        let dot = |a: Vec2, b: Vec2| a.x * b.x + a.y * b.y;
        let v0 = sub(p3, p1);
        let v1 = sub(p2, p1);
        let v2 = sub(p, p1);
        let (d00, d01, d02, d11, d12) = (
            dot(v0, v0),
            dot(v0, v1),
            dot(v0, v2),
            dot(v1, v1),
            dot(v1, v2),
        );
        let inv = 1.0f32 / (d00 * d11 - d01 * d01);
        Vec2::new((d11 * d02 - d01 * d12) * inv, (d00 * d12 - d01 * d02) * inv)
    }

    let mut rng = Rng::new(0xF00D);
    let mut differs = 0usize;
    let total = 200_000;
    for _ in 0..total {
        let mut args = [
            rng.normal_vec(),
            rng.normal_vec(),
            rng.normal_vec(),
            rng.normal_vec(),
        ];
        let n = 2 + rng.below(7);
        for _ in 0..n {
            let slot = rng.below(8) as usize;
            *slot_mut(&mut args, slot) = if rng.bool() {
                rng.qnan_f32()
            } else {
                rng.snan_f32()
            };
        }
        // the reference (C) result vs a naive Rust translation
        let c = call_c(args[0], args[1], args[2], args[3]);
        let nv = naive(args[0], args[1], args[2], args[3]);
        if c.bits() != nv.bits() {
            differs += 1;
        }
        // meanwhile the actual Rust .so must match the C exactly
        assert_same("negative-control", args);
    }
    assert!(
        differs > 0,
        "no multi-NaN input distinguished the naive translation from the C — the NaN-payload \
         tests would be vacuous"
    );
    eprintln!(
        "naive-NaN mis-translation would diverge on {differs}/{total} multi-NaN inputs \
         (the real Rust .so matched on all {total})"
    );
}
