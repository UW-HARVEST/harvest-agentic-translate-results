//! Phase C — error/rejection-path differential tests, one per row of
//! `ERRORS.md`.
//!
//! `driver` returns `void` and has no error codes, so its only rejection
//! mechanism is the loop guard `i < x`: a rejected input produces exactly zero
//! bytes of output and returns normally. "Same rejection" therefore means both
//! libraries return normally AND emit the identical empty byte string — not
//! merely "both did something".

mod common;

use common::{assert_same, impls, Rng};
use std::ffi::c_int;

/// Assert C and every Rust `.so` reject `x` in the same way: normal return,
/// zero bytes emitted, identical to each other.
fn assert_rejects_identically(x: c_int) {
    let impls = impls();
    let c_out = impls.c.run(x);
    assert!(
        c_out.out.is_empty(),
        "expected C driver({x}) to emit nothing, got {} bytes: {:?}",
        c_out.out.len(),
        String::from_utf8_lossy(&c_out.out).chars().take(80).collect::<String>()
    );
    for r in &impls.rust {
        let out = r.run(x);
        assert_eq!(
            out, c_out,
            "{} did not reject driver({x}) the same way as C: {out:?}",
            r.name
        );
    }
}

// ------------------------------------------------------------------- E1
#[test]
fn err_e1_zero() {
    // Zero length: guard `0 < 0` is false on the first evaluation.
    assert_rejects_identically(0);
    assert_same(0);
}

// ------------------------------------------------------------------- E2
#[test]
fn err_e2_negative_one() {
    assert_rejects_identically(-1);
    assert_same(-1);
}

// ------------------------------------------------------------------- E3
#[test]
fn err_e3_int_min() {
    // The extreme negative input; must not underflow, trap, or loop.
    assert_rejects_identically(i32::MIN);
    assert_same(i32::MIN);
    // Also the immediate neighbourhood of INT_MIN.
    for x in [i32::MIN, i32::MIN + 1, i32::MIN + 2] {
        assert_rejects_identically(x);
    }
}

// ------------------------------------------------------------------- E4
#[test]
fn err_e4_random_negatives() {
    let mut rng = Rng::new(0x0E04_5EED);
    for _ in 0..500 {
        assert_rejects_identically(rng.range_i32(i32::MIN, -1));
    }
    // Plus every small negative exhaustively.
    for x in -300..=0 {
        assert_rejects_identically(x);
    }
}

// ------------------------------------------------------------------- E5
#[test]
fn err_e5_int_max_documented() {
    // INT_MAX is an *accepted* input (the C code performs no bounds check), but
    // running it would require ~2.1e9 printf calls and tens of GB of output, so
    // it cannot be compared differentially. See the note in ERRORS.md.
    //
    // What is checkable here is that the largest values we *can* run stay in
    // agreement, and that neither library rejects a large positive count.
    let impls = impls();
    for x in [200_000, 250_000] {
        let c_out = impls.c.run(x);
        assert!(!c_out.out.is_empty(), "large positive x should not be rejected");
        for r in &impls.rust {
            assert_eq!(r.run(x), c_out, "{} diverged for large driver({x})", r.name);
        }
    }
}

// ------------------------------------------------------------------- E6
#[test]
fn err_e6_no_enum_surface() {
    // The API exposes no enum parameter, so there is no "no valid variant"
    // value to reject. The parameter is a plain `int`: every bit pattern is a
    // legal input. Feed the values that would be out-of-range enum sentinels in
    // a typical C API and confirm identical handling rather than divergence.
    let sentinels: [c_int; 12] = [
        -1,
        -2,
        -999,
        i32::MIN,
        i32::MIN + 1,
        0x7FFF_FFFF_u32 as i32,
        0x7FFF_FFFE_u32 as i32,
        0xFFFF_FFFF_u32 as i32, // == -1
        0x8000_0000_u32 as i32, // == INT_MIN
        255,
        256,
        65535,
    ];
    let impls = impls();
    for x in sentinels {
        // Skip the two enormous positives: covered as accepted inputs in E5.
        if x > 300_000 {
            assert!(x == i32::MAX || x == i32::MAX - 1);
            continue;
        }
        let c_out = impls.c.run(x);
        for r in &impls.rust {
            assert_eq!(r.run(x), c_out, "{} diverged for out-of-range sentinel driver({x})", r.name);
        }
    }
}

// ------------------------------------------------------------------- E7
#[test]
fn err_e7_no_pointer_surface() {
    // `driver` takes no pointer and dereferences nothing, so there is no null
    // check and no null input to construct. Confirm mechanically that the
    // exported symbol really has the single-int signature both libraries agree
    // on, by calling it through a pointer-sized-argument-free FFI type and
    // checking both still behave identically.
    let impls = impls();
    let c_fn = impls.c.driver();
    let c_out = common::capture(|| unsafe { c_fn(3) });
    for r in &impls.rust {
        let f = r.driver();
        let out = common::capture(|| unsafe { f(3) });
        assert_eq!(out, c_out, "{} diverged when called through the raw symbol", r.name);
    }
    assert_eq!(c_out.out, b"0 0\n1 2\n2 4\n".to_vec());
}

// ------------------------------------------------------------------- E8
#[test]
fn err_e8_no_latched_state() {
    // A rejected call must not latch any error state: the following valid call
    // has to behave exactly as if the rejection never happened.
    let impls = impls();
    let baseline = impls.c.run(11);

    for bad in [0, -1, -12345, i32::MIN] {
        // C: reject, then valid.
        assert!(impls.c.run(bad).out.is_empty());
        assert_eq!(impls.c.run(11), baseline, "C latched state after driver({bad})");

        // Rust: reject, then valid.
        for r in &impls.rust {
            assert!(r.run(bad).out.is_empty(), "{} emitted output for driver({bad})", r.name);
            assert_eq!(r.run(11), baseline, "{} latched state after driver({bad})", r.name);
        }
    }
}

// ---------------------------------------------------- generic FFI boundaries
#[test]
fn boundary_sweep_around_the_guard() {
    // One step either side of the accept/reject boundary.
    for x in [-2, -1, 0, 1, 2] {
        assert_same(x);
    }
}

#[test]
fn boundary_zero_and_oversized_lengths() {
    // Zero length, and lengths well past any plausible internal buffer size.
    assert_same(0);
    for x in [4095, 4096, 4097, 8191, 8192, 8193] {
        assert_same(x);
    }
}
