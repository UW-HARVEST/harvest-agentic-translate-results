//! Harness self-check: proves the two `.so`s really are loaded, that the
//! comparison actually observes stderr and errno, and that a real divergence
//! would be detected (rather than the suite passing vacuously).

mod common;

use common::*;

#[test]
fn both_shared_objects_export_my_pow() {
    // Panics inside `impls()` if either dlopen or dlsym fails.
    let im = impls();
    assert!(!(im.c as usize == 0));
    assert!(!(im.rust as usize == 0));
    // The two must be genuinely different code addresses, i.e. we are not
    // accidentally comparing one implementation against itself.
    assert_ne!(
        im.c as usize, im.rust as usize,
        "C and Rust my_pow resolved to the same address"
    );
}

#[test]
fn harness_observes_a_clean_call() {
    let o = c_outcome(2.0, 10.0);
    assert_eq!(o.bits, 1024.0f64.to_bits());
    assert_eq!(o.errno, 0);
    assert!(o.stderr.is_empty(), "unexpected stderr: {:?}", o.stderr);
}

#[test]
fn harness_observes_the_domain_error_branch() {
    let o = c_outcome(-2.0, 0.5);
    assert_eq!(o.bits, (-1.0f64).to_bits());
    assert_eq!(o.errno, EDOM);
    assert_eq!(
        String::from_utf8_lossy(&o.stderr),
        "Domain error: pow(-2.00, 0.50) is undefined in the real number domain.\n"
    );
}

#[test]
fn harness_observes_the_range_error_branch() {
    let o = c_outcome(10.0, 400.0);
    assert_eq!(o.bits, (-1.0f64).to_bits());
    assert_eq!(o.errno, ERANGE);
    assert_eq!(
        String::from_utf8_lossy(&o.stderr),
        "Range error: pow(10.00, 400.00) caused overflow or underflow.\n"
    );
}

/// The suite would be worthless if `-1.0` alone were treated as "an error";
/// this proves the harness distinguishes a legitimate `-1.0` from a rejection.
#[test]
fn harness_distinguishes_legit_minus_one_from_a_rejection() {
    let legit = c_outcome(-1.0, 3.0);
    let reject = c_outcome(-2.0, 0.5);
    assert_eq!(legit.bits, reject.bits, "both return -1.0 bit-identically");
    assert_ne!(
        (legit.errno, legit.stderr.is_empty()),
        (reject.errno, reject.stderr.is_empty()),
        "but they must be distinguishable via errno/stderr"
    );
    assert!(legit.stderr.is_empty());
    assert_eq!(legit.errno, 0);
}

/// The harness must be able to see a difference at all. Compare the C against a
/// deliberately wrong "implementation" and require the comparison to fail.
#[test]
fn harness_would_detect_a_divergence() {
    unsafe extern "C" fn wrong(_b: f64, _e: f64) -> f64 {
        0.0
    }
    let im = impls();
    let c = call_once(im.c, 2.0, 10.0, 0, StderrMode::Capture);
    let w = call_once(wrong as PowFn, 2.0, 10.0, 0, StderrMode::Capture);
    assert_ne!(c, w, "harness cannot distinguish differing implementations");
}

/// `-0.0` must not be conflated with `+0.0`; confirms the bit-level comparison.
#[test]
fn harness_distinguishes_signed_zero() {
    let neg = c_outcome(-0.0, 3.0);
    let pos = c_outcome(0.0, 3.0);
    assert_eq!(neg.bits, (-0.0f64).to_bits());
    assert_eq!(pos.bits, (0.0f64).to_bits());
    assert_ne!(neg.bits, pos.bits);
}
