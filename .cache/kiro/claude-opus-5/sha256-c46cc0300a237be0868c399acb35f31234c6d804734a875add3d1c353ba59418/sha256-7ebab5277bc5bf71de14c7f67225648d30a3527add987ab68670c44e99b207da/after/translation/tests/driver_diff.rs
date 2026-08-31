//! Differential test for the public API of `driver.h`:
//!
//! ```c
//! void driver(int x);
//! ```
//!
//! The C implementation computes `2*x`, adds 300 and prints it with `%d\n`.
//! Both implementations are invoked through their `.so` exports and the bytes
//! written to stdout are compared byte-for-byte.

mod common;

use common::{capture_stdout, load_both};
use std::ffi::c_int;

type DriverFn = unsafe extern "C" fn(c_int);

/// Every input we exercise: boundaries, signs, powers of two, and values that
/// make `2*x` and `+300` overflow so the wrapping behaviour is pinned down.
fn inputs() -> Vec<c_int> {
    let mut v: Vec<c_int> = vec![
        0,
        1,
        -1,
        2,
        -2,
        3,
        -3,
        7,
        -7,
        10,
        -10,
        42,
        -42,
        99,
        -99,
        100,
        -100,
        149,
        -149,
        150,
        -150,
        151,
        -151,
        255,
        -255,
        256,
        -256,
        1000,
        -1000,
        65535,
        -65535,
        65536,
        -65536,
        i32::MAX,
        i32::MIN,
        i32::MAX - 1,
        i32::MIN + 1,
        // 2*x lands exactly on the signed boundaries
        i32::MAX / 2,
        i32::MAX / 2 + 1,
        i32::MIN / 2,
        i32::MIN / 2 - 1,
        // 2*x + 300 lands exactly on / just past INT_MAX
        (i32::MAX - 300) / 2,
        (i32::MAX - 300) / 2 + 1,
        // 2*x + 300 == 0
        -150,
        1 << 30,
        -(1 << 30),
        (1 << 30) + 1,
        0x5555_5555u32 as c_int,
        0x7fff_fffeu32 as c_int,
        -0x7fff_ffffi32,
    ];

    // A deterministic pseudo-random sweep over the whole int range.
    let mut state: u64 = 0x1234_5678_9abc_def0;
    for _ in 0..2000 {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        v.push((state >> 32) as u32 as c_int);
    }

    v.sort_unstable();
    v.dedup();
    v
}

#[test]
fn driver_matches_c_for_all_inputs() {
    let (c_lib, rust_lib) = load_both();

    let c_driver: libloading::Symbol<DriverFn> =
        unsafe { c_lib.get(b"driver\0").expect("C .so exports `driver`") };
    let rust_driver: libloading::Symbol<DriverFn> =
        unsafe { rust_lib.get(b"driver\0").expect("Rust .so exports `driver`") };

    let mut mismatches = Vec::new();

    for x in inputs() {
        let c_out = capture_stdout("c", || unsafe { c_driver(x) });
        let r_out = capture_stdout("rs", || unsafe { rust_driver(x) });

        if c_out != r_out {
            mismatches.push(format!(
                "driver({x}): C = {:?} ({:x?}), Rust = {:?} ({:x?})",
                String::from_utf8_lossy(&c_out),
                c_out,
                String::from_utf8_lossy(&r_out),
                r_out
            ));
        }
    }

    assert!(
        mismatches.is_empty(),
        "{} mismatch(es):\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Sanity check that the capture harness really observes output, so that an
/// "everything matches" result cannot be produced by two empty captures.
#[test]
fn capture_harness_observes_output() {
    let (c_lib, rust_lib) = load_both();
    let c_driver: libloading::Symbol<DriverFn> = unsafe { c_lib.get(b"driver\0").unwrap() };
    let rust_driver: libloading::Symbol<DriverFn> = unsafe { rust_lib.get(b"driver\0").unwrap() };

    let c_out = capture_stdout("c", || unsafe { c_driver(0) });
    let r_out = capture_stdout("rs", || unsafe { rust_driver(0) });

    assert_eq!(c_out, b"300\n", "C driver(0) should print 300");
    assert_eq!(r_out, b"300\n", "Rust driver(0) should print 300");
}

/// Repeated calls in a row must interleave identically, catching any
/// difference in stdio buffering or state kept across calls.
#[test]
fn driver_repeated_calls_match() {
    let (c_lib, rust_lib) = load_both();
    let c_driver: libloading::Symbol<DriverFn> = unsafe { c_lib.get(b"driver\0").unwrap() };
    let rust_driver: libloading::Symbol<DriverFn> = unsafe { rust_lib.get(b"driver\0").unwrap() };

    let seq: [c_int; 8] = [0, -150, 1, i32::MAX, i32::MIN, -1, 12345, -67890];

    let c_out = capture_stdout("c", || {
        for &x in &seq {
            unsafe { c_driver(x) };
        }
    });
    let r_out = capture_stdout("rs", || {
        for &x in &seq {
            unsafe { rust_driver(x) };
        }
    });

    assert_eq!(
        c_out,
        r_out,
        "batched output differs:\nC    = {:?}\nRust = {:?}",
        String::from_utf8_lossy(&c_out),
        String::from_utf8_lossy(&r_out)
    );
}
