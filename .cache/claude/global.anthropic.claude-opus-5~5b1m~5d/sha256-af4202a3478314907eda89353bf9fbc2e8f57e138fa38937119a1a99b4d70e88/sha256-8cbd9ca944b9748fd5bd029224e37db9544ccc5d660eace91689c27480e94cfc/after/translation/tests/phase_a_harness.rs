//! Harness self-checks (negative controls).
//!
//! Proves the differential machinery is actually loading both `.so`s, actually
//! calling them, and actually *fails* when the outputs differ — so a green
//! Phase B/C run is meaningful rather than vacuous.

mod common;

use common::*;

#[test]
fn both_shared_objects_load_and_export_tfm() {
    let l = libs();
    // Distinct function addresses => two distinct libraries really are loaded.
    assert_ne!(
        l.c_tfm as usize, l.rust_tfm as usize,
        "C and Rust `tfm` resolved to the same address; only one .so was loaded"
    );
}

#[test]
fn c_produces_the_hand_computed_reference_value() {
    // if-branch: src[0]=1 < src[1]=4, dxy=0
    //   sqd    = 16 - 8 + 1 + 0 = 9
    //   lambda = 0.5*(4 + 1 + 3) = 4
    //   dest   = [1 - 4, 0] = [-3, 0]
    let out = diff_one(1.0, 4.0, 0.0, "reference if-branch");
    assert_eq!(out[0].to_bits(), (-3.0f32).to_bits(), "got {}", show(&out));
    assert_eq!(out[1].to_bits(), 0.0f32.to_bits(), "got {}", show(&out));

    // else-branch: src[0]=4 >= src[1]=1 => dy2=4, dx2=1, dxy=0
    //   same sqd/lambda; dest = [dxy, dx2 - lambda] = [0, -3]
    let out = diff_one(4.0, 1.0, 0.0, "reference else-branch");
    assert_eq!(out[0].to_bits(), 0.0f32.to_bits(), "got {}", show(&out));
    assert_eq!(out[1].to_bits(), (-3.0f32).to_bits(), "got {}", show(&out));
}

#[test]
fn negative_control_bit_comparison_is_strict() {
    // -0.0 vs +0.0 must NOT compare equal, or every signed-zero assertion in
    // Phase B/C would be vacuous.
    assert!(!bits_eq(&[-0.0], &[0.0]));
    assert!(bits_eq(&[-0.0], &[-0.0]));

    // Distinct NaN payloads must NOT compare equal either.
    let a = f32::from_bits(0x7FC0_0000);
    let b = f32::from_bits(0xFFC0_0000);
    assert!(a.is_nan() && b.is_nan());
    assert!(!bits_eq(&[a], &[b]), "NaN payload/sign comparison is not strict");
}

#[test]
fn negative_control_harness_detects_a_planted_divergence() {
    // Run the C function, then perturb one output bit and confirm the very
    // comparison Phase B relies on rejects it.
    let l = libs();
    let src = [1.0f32, 4.0, 0.5];
    let mut c_out = [0.0f32; 2];
    let mut r_out = [0.0f32; 2];
    unsafe {
        (l.c_tfm)(c_out.as_mut_ptr(), src.as_ptr(), 1);
        (l.rust_tfm)(r_out.as_mut_ptr(), src.as_ptr(), 1);
    }
    assert!(bits_eq(&c_out, &r_out), "C and Rust must agree here");

    // One-bit perturbation must be caught.
    let mut tampered = r_out;
    tampered[0] = f32::from_bits(r_out[0].to_bits() ^ 1);
    assert!(
        !bits_eq(&c_out, &tampered),
        "harness failed to detect a 1-ulp divergence"
    );

    // ...and so must a sign-of-zero-only perturbation.
    let z = [0.0f32, 0.0];
    let z2 = [0.0f32, -0.0];
    assert!(!bits_eq(&z, &z2));
}

#[test]
fn negative_control_diff_call_actually_invokes_both() {
    // If `diff_call` silently did nothing, the destination would still hold the
    // FILL sentinel. Check the live region really changed.
    let out = diff_call(&[3.0, 7.0, 2.0], 1, "liveness");
    assert_eq!(out.len(), 2);
    assert!(
        out[0].to_bits() != FILL.to_bits() && out[1].to_bits() != FILL.to_bits(),
        "diff_call did not write the destination: {}",
        show(&out)
    );

    // And for a multi-element call, every one of the 2*count slots is written.
    let src: Vec<f32> = (0..3 * 37).map(|i| (i as f32) * 0.37 - 5.0).collect();
    let out = diff_call(&src, 37, "liveness multi");
    assert_eq!(out.len(), 74);
    for (i, v) in out.iter().enumerate() {
        assert_ne!(v.to_bits(), FILL.to_bits(), "slot {i} was never written");
    }
}

#[test]
fn negative_control_prng_is_not_degenerate() {
    let mut rng = Rng::new(0xDEAD);
    let mut seen = std::collections::HashSet::new();
    for _ in 0..10_000 {
        seen.insert(rng.any_bits().to_bits());
    }
    assert!(seen.len() > 9_900, "PRNG produced only {} distinct values", seen.len());

    // `finite()` must really be finite and span many exponents.
    let mut rng = Rng::new(0xBEEF);
    let mut exps = std::collections::HashSet::new();
    for _ in 0..10_000 {
        let v = rng.finite();
        assert!(v.is_finite() && v != 0.0, "finite() produced {v}");
        exps.insert((v.to_bits() >> 23) & 0xff);
    }
    assert!(exps.len() > 20, "finite() spans only {} exponents", exps.len());
}
