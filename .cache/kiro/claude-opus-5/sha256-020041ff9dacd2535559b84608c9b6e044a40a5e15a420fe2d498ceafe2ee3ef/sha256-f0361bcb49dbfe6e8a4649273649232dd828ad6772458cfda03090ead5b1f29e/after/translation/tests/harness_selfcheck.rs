//! Harness self-checks (negative controls).
//!
//! A differential suite that silently compares nothing would pass everything.
//! These tests prove the harness actually loads two distinct shared objects,
//! actually crosses the FFI boundary, and actually detects a difference.

mod common;
use common::*;

/// Both shared objects must really be loaded, and the Rust side must be loaded
/// from a file distinct from the C one.
#[test]
fn harness_loads_both_libraries() {
    let c = c_impl();
    let rs = rust_impls();
    assert!(!rs.is_empty(), "no Rust cdylib was loaded");
    for r in rs {
        assert_ne!(
            c.f as usize, r.f as usize,
            "{} resolved to the same code address as C — the wrong .so was loaded",
            r.name
        );
    }
    // Both profiles of the Rust cdylib should be under test.
    assert!(
        rs.iter().any(|r| r.name.contains("release")),
        "the release cdylib must be under test; run `cargo build --release`"
    );
}

/// `call` must actually invoke the loaded symbol: its result has to depend on
/// the input, and it has to write the destination buffer.
#[test]
fn harness_output_depends_on_input() {
    let c = c_impl();
    let a = call(c, &[30.0, 1.0, 1.0]);
    let b = call(c, &[210.0, 1.0, 1.0]);
    assert_ne!(a, b, "call() output does not depend on its input");
    // dest[0..3] must have been overwritten (canary is a fixed NaN pattern).
    for i in 0..3 {
        assert_ne!(a[i], 0x7F80_1D0Du32, "dest[{i}] was never written");
    }
}

/// The comparison is bit-exact, not `==`-based: `+0.0` and `-0.0` compare equal
/// under `==` but must be treated as different results here.
#[test]
fn harness_distinguishes_signed_zero() {
    let c = c_impl();
    let pos = call(c, &[0.0, 0.0, 0.0]);
    let neg = call(c, &[0.0, 0.0, -0.0]);
    assert_eq!(pos[0], 0x0000_0000, "expected +0.0 bits");
    assert_eq!(neg[0], 0x8000_0000, "expected -0.0 bits");
    assert_ne!(pos, neg, "harness collapses +0.0 and -0.0");
}

/// The comparison distinguishes distinct NaN bit patterns — the exact class of
/// difference that was actually found and fixed in this translation (SSE
/// operand-order-dependent NaN propagation, `0xFFC0_0000` vs `0x7FC0_0000`).
#[test]
fn harness_distinguishes_nan_payloads() {
    assert_ne!(
        f32::from_bits(0x7FC0_0000).to_bits(),
        f32::from_bits(0xFFC0_0000).to_bits()
    );
    // This input previously diverged: C yielded 0xFFC00000 for dest[2] while the
    // Rust yielded 0x7FC00000. Pin the C reference value so a regression in the
    // Rust is caught by value, not just by comparison.
    let out = call(c_impl(), &[-60.0, f32::INFINITY, f32::NAN]);
    assert_eq!(
        out[2], 0xFFC0_0000,
        "C reference changed for the known operand-order-sensitive NaN case"
    );
    assert_same("negative-control", &[-60.0, f32::INFINITY, f32::NAN]);
}

/// `assert_same` must be capable of failing. Feed the comparison two different
/// inputs through the same code path and confirm the bits differ, which is the
/// signal `assert_same` relies on.
#[test]
fn harness_would_detect_a_divergence() {
    let c = c_impl();
    let r = rust_impls()[0];
    // Same input -> must agree.
    assert_eq!(call(c, &[123.0, 0.6, 0.7]), call(r, &[123.0, 0.6, 0.7]));
    // Different input -> must disagree, proving the assertion is not vacuous.
    assert_ne!(
        call(c, &[123.0, 0.6, 0.7]),
        call(r, &[124.0, 0.6, 0.7]),
        "assert_same could never fail: outputs are input-independent"
    );
}

/// Aliased calls must also actually depend on the aliasing offsets.
#[test]
fn harness_aliasing_is_observable() {
    let c = c_impl();
    let buf = [30.0f32, 1.0, 1.0, 0.0, 0.0, 0.0];
    let disjoint = call_aliased(c, &buf, 3, 0);
    let overlapping = call_aliased(c, &buf, 1, 0);
    assert_ne!(disjoint, overlapping, "aliasing offsets have no observable effect");
    // Disjoint case must leave src untouched.
    assert_eq!(&disjoint[0..3], &[30.0f32.to_bits(), 1.0f32.to_bits(), 1.0f32.to_bits()]);
}
