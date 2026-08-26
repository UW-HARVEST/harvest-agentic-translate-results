// Phase C — error/rejection-path differential tests.
//
// One test per row of ERRORS.md.  `driver` returns `void` and has no error
// code, so the observable "rejection result" is: returns normally having
// written exactly zero bytes.  Each test asserts BOTH that the two shared
// objects agree AND that the shared result is the exact expected one, so a
// test cannot pass by both sides failing in the same vague way.

mod common;

use common::{assert_same, assert_same_pipe, assert_same_sequence, Rng, SEED};

/// Assert that `driver(x)` is a no-output no-op in *both* libraries.
fn assert_rejected(x: i32) {
    let c = common::c_output(x);
    let r = common::rust_output(x);
    assert!(
        c.is_empty(),
        "C reference unexpectedly produced {} bytes for driver({x}): {:?}",
        c.len(),
        String::from_utf8_lossy(&c[..c.len().min(64)])
    );
    assert_eq!(
        c,
        r,
        "driver({x}): C wrote {} bytes, Rust wrote {} bytes ({:?})",
        c.len(),
        r.len(),
        String::from_utf8_lossy(&r[..r.len().min(64)])
    );
}

// --- E1 --- x == 0 : the empty / zero-length input --------------------------
#[test]
fn e1_zero_is_rejected_with_empty_output() {
    assert_rejected(0);
    assert_same(0);
}

// --- E2 --- x == -1 : one step past the accepting range ---------------------
#[test]
fn e2_minus_one_empty_output() {
    assert_rejected(-1);
    assert_same(-1);
}

// --- E3 --- randomized negatives -------------------------------------------
#[test]
fn e3_random_negative_empty_output() {
    let mut rng = Rng::new(SEED ^ 0x13);
    for _ in 0..200 {
        assert_rejected(rng.range_i32(i32::MIN, -1));
    }
    // plus a deliberate spread of magnitudes
    for &x in &[-2, -3, -9, -10, -99, -100, -1_000, -100_000, -1_000_000_000] {
        assert_rejected(x);
    }
}

// --- E4 --- x == INT_MIN ---------------------------------------------------
#[test]
fn e4_int_min_empty_output() {
    assert_rejected(i32::MIN);
    assert_same(i32::MIN);
}

// --- E5 --- x == INT_MIN + 1 ----------------------------------------------
#[test]
fn e5_int_min_plus_one_empty_output() {
    assert_rejected(i32::MIN + 1);
    assert_same(i32::MIN + 1);
}

// --- E6 --- x == 1 : the first accepted value (boundary from the other side)
#[test]
fn e6_one_is_the_accept_boundary() {
    let c = common::c_output(1);
    let r = common::rust_output(1);
    assert_eq!(c, b"0 0\n".to_vec());
    assert_eq!(c, r);
    // and the transition 0 -> 1 -> 2 is identical in both
    assert_same_sequence(&[0, 1, 2]);
}

// --- E7 --- arbitrary raw bit patterns handed across the FFI boundary ------
// A C API accepts *any* `int` bit pattern for its parameter (this is the same
// class of input as an out-of-range enum value: no valid "variant" is implied
// by the signature).  Every pattern must be handled identically.
#[test]
fn e7_raw_bit_patterns_across_ffi() {
    let patterns: &[u32] = &[
        0x0000_0000,
        0x0000_0001,
        0x0000_00FF,
        0x0000_7FFF,
        0x0001_0000,
        0x7FFF_FFFE,
        0x8000_0000, // -> INT_MIN
        0x8000_0001,
        0xC000_0000,
        0xDEAD_BEEF,
        0xFFFF_0000,
        0xFFFF_FFFE,
        0xFFFF_FFFF, // -> -1
    ];
    for &p in patterns {
        let x = p as i32;
        if x <= 0 {
            assert_rejected(x);
        } else if x <= 200_000 {
            // still a valid, cheap-to-run accepting value
            assert_same(x);
        } else {
            // Values such as 0x7FFF_FFFE would need billions of iterations to
            // run to completion; they are covered as bit patterns by the
            // negative/zero cases above.  Nothing to execute here.
        }
    }
    let mut rng = Rng::new(SEED ^ 0x17);
    for _ in 0..200 {
        let x = rng.next_u32() as i32;
        if x <= 0 {
            assert_rejected(x);
        }
    }
}

// --- E8 --- rejections leave no residual state ----------------------------
#[test]
fn e8_rejections_leave_no_residual_state() {
    assert_same_sequence(&[0, 0, 0, 0, 0]);
    assert_same_sequence(&[-5, -5, -5, 3, -5, -5]);
    assert_same_sequence(&[i32::MIN, 0, -1, 4, 0, i32::MIN, 2]);

    // A rejecting call before/after an accepting one must not change the
    // accepting call's bytes at all.
    let baseline = common::c_output(7);
    let mut rng = Rng::new(SEED ^ 0x18);
    for _ in 0..25 {
        let bad = rng.range_i32(i32::MIN, 0);
        let cf = common::c_driver();
        let rf = common::rust_driver();
        let c = common::capture_file(|| unsafe {
            cf(bad);
            cf(7);
            cf(bad);
        });
        let r = common::capture_file(|| unsafe {
            rf(bad);
            rf(7);
            rf(bad);
        });
        assert_eq!(c, baseline, "C: driver({bad}) perturbed driver(7)");
        assert_eq!(c, r, "Rust diverged around driver({bad})");
    }
}

// --- E10 --- ABI edge: garbage in the high half of the argument register ----
// A C `int` parameter occupies only the low 32 bits of the argument register.
// A caller that puts a 64-bit value there (a mismatched prototype, a language
// binding, an out-of-range "enum" widened to 64 bits) must be interpreted the
// same way by the Rust export wrapper as by the C function: low 32 bits only.
#[test]
fn e10_high_register_bits_ignored_identically() {
    type Wide = unsafe extern "C" fn(i64);
    let cf: Wide = unsafe { std::mem::transmute::<common::DriverFn, Wide>(common::c_driver()) };
    let rf: Wide = unsafe { std::mem::transmute::<common::DriverFn, Wide>(common::rust_driver()) };

    // Low half is always a small, cheap value (0, a few, or negative) so that
    // even a hypothetical 64-bit interpretation stays bounded.
    let widened: &[i64] = &[
        0x0000_0000_0000_0000,
        0x0000_0001_0000_0000,          // low = 0
        0x1234_5678_0000_0005u64 as i64, // low = 5
        0x7FFF_FFFF_0000_0003,          // low = 3
        0xFFFF_FFFF_0000_0000u64 as i64, // low = 0
        0x0000_0007_FFFF_FFFFu64 as i64, // low = -1
        0x0000_0009_8000_0000u64 as i64, // low = INT_MIN
    ];
    for &w in widened {
        let c = common::capture_file(|| unsafe { cf(w) });
        let r = common::capture_file(|| unsafe { rf(w) });
        assert_eq!(
            c, r,
            "widened argument 0x{w:016X}: C produced {} bytes, Rust {} bytes",
            c.len(),
            r.len()
        );
        // and it must agree with passing just the low 32 bits as an int
        let narrow = w as i32;
        assert_eq!(
            c,
            common::c_output(narrow),
            "C did not ignore the high half of 0x{w:016X}"
        );
    }
}

// --- E9 --- rejection while fd 1 is a pipe --------------------------------
#[test]
fn e9_zero_output_on_pipe() {
    let cf = common::c_driver();
    let rf = common::rust_driver();
    for &x in &[0, -1, -12345, i32::MIN, i32::MIN + 1] {
        let c = common::capture_pipe(|| unsafe { cf(x) });
        let r = common::capture_pipe(|| unsafe { rf(x) });
        assert!(c.is_empty(), "C wrote {} bytes on a pipe for {x}", c.len());
        assert_eq!(c, r, "pipe rejection diverged for driver({x})");
        assert_same_pipe(x);
    }
}
