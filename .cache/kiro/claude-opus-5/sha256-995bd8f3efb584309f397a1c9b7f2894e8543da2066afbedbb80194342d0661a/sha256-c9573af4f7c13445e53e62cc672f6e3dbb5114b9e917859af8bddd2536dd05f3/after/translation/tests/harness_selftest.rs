//! Meta-tests for the differential harness itself.
//!
//! A green Phase B/C is only meaningful if the comparison actually crosses the
//! FFI boundary into *both* `.so`s on every iteration and actually fails on a
//! real difference. These tests assert exactly that, so the rest of the suite
//! cannot pass by being optimized away or by comparing a value against itself.

#![allow(non_snake_case)]

mod common;
use common::*;

#[test]
fn same_detects_a_real_difference() {
    let p = pair();
    let (cNeg, _) = p.get::<FnVV>(b"c2Neg");
    let (_, rSkew) = p.get::<FnVV>(b"c2Skew");
    let a = c2v { x: 1.0, y: 2.0 };
    let (x, y) = unsafe { (cNeg(a), rSkew(a)) };
    assert!(
        std::panic::catch_unwind(|| same("deliberate mismatch", &x, &y)).is_err(),
        "same() failed to report a real difference"
    );
    same("deliberate match", &x, &x);
}

#[test]
fn same_distinguishes_signed_zero_and_nan_payloads() {
    // Byte comparison, not `f32 ==`: these must all be reported as different.
    for (a, b) in [
        (0.0f32, -0.0f32),
        (f32::from_bits(0x7fc0_0000), f32::from_bits(0xffc0_0000)),
        (f32::from_bits(0x7fc0_0000), f32::from_bits(0x7fc0_0001)),
    ] {
        assert!(
            std::panic::catch_unwind(move || same("bits", &a, &b)).is_err(),
            "same() treated {:08x} and {:08x} as equal",
            a.to_bits(),
            b.to_bits()
        );
    }
}

#[test]
fn poison_manifold_is_fully_non_zero() {
    // Every byte must be non-zero, otherwise a field the C leaves untouched
    // could coincidentally match a zeroed Rust output.
    let m = poison_manifold(0);
    assert!(raw(&m).iter().all(|&b| b != 0), "poison contains zero bytes");
    assert_ne!(raw(&poison_manifold(1)), raw(&poison_manifold(2)));
}

#[test]
fn loops_really_execute_both_libraries() {
    let p = pair();
    let (cNeg, _) = p.get::<FnVV>(b"c2Neg");
    let (_, rSkew) = p.get::<FnVV>(b"c2Skew");
    let mut rng = Rng::new(2);
    let mut diffs = 0u64;
    const ITERS: u64 = 1_000_000;
    for _ in 0..ITERS {
        let a = rng.vec_sym(1.0);
        unsafe {
            if raw(&cNeg(a)) != raw(&rSkew(a)) {
                diffs += 1;
            }
        }
    }
    assert_eq!(
        diffs, ITERS,
        "the harness is not calling both libraries on every iteration"
    );
}

#[test]
fn rng_is_deterministic_and_covers_the_value_families() {
    let a: Vec<u64> = (0..8).map(|_| Rng::new(9).next_u64()).collect();
    assert!(a.windows(2).all(|w| w[0] == w[1]), "seed is not reproducible");

    let mut rng = Rng::new(3);
    let mut saw_nan = false;
    let mut saw_inf = false;
    let mut saw_neg_zero = false;
    let mut saw_exact_tie = false;
    for _ in 0..20_000 {
        let v = rng.spicy();
        saw_nan |= v.is_nan();
        saw_inf |= v.is_infinite();
        saw_neg_zero |= v.to_bits() == 0x8000_0000;
        saw_exact_tie |= rng.grid(0.5, 4) == 0.0;
    }
    assert!(saw_nan && saw_inf && saw_neg_zero && saw_exact_tie);
}
