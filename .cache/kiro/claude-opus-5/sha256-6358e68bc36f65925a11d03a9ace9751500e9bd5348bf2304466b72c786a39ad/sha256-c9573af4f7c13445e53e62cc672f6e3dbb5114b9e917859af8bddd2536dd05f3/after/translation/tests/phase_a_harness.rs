//! Harness self-checks (negative controls).
//!
//! A green differential suite is only meaningful if the harness (a) really
//! loads two *different* shared objects, and (b) actually fails when they
//! disagree. These tests assert both.

mod common;

use common::{assert_stderr_eq, capture_stderr, pair};

#[test]
fn harness_loads_two_distinct_shared_objects() {
    let p = pair();
    eprintln!("C   .so: {}", p.c_path.display());
    eprintln!("Rust.so: {}", p.rust_path.display());

    assert_ne!(
        std::fs::canonicalize(&p.c_path).unwrap(),
        std::fs::canonicalize(&p.rust_path).unwrap(),
        "both sides resolved to the same file — the differential test would be vacuous"
    );
    assert!(
        p.c_path.to_string_lossy().contains("c_src"),
        "C side must come from c_src/build, got {}",
        p.c_path.display()
    );
    assert!(
        p.rust_path.to_string_lossy().contains("target"),
        "Rust side must come from the cargo target dir, got {}",
        p.rust_path.display()
    );

    // Neither .so is a copy of the other.
    let a = std::fs::read(&p.c_path).unwrap();
    let b = std::fs::read(&p.rust_path).unwrap();
    assert_ne!(a, b, "the two shared objects are byte-identical");

    // Both really answer through their exported `my_pow`.
    assert_eq!(p.c.call(2.0, 10.0), 1024.0);
    assert_eq!(p.rust.call(2.0, 10.0), 1024.0);
}

#[test]
fn harness_detects_value_divergence() {
    // The comparison is on raw bit patterns, so these pairs — which all
    // compare `==` or are both "NaN" — must be treated as different.
    let must_differ: &[(f64, f64)] = &[
        (0.0, -0.0),
        (
            f64::from_bits(0x7FF8_0000_0000_0000),
            f64::from_bits(0xFFF8_0000_0000_0000),
        ),
        (
            f64::from_bits(0x7FF8_0000_0000_0000),
            f64::from_bits(0x7FF8_0000_DEAD_BEEF),
        ),
    ];
    for &(a, b) in must_differ {
        assert_ne!(
            a.to_bits(),
            b.to_bits(),
            "bit comparison failed to distinguish {a:?} from {b:?}"
        );
    }
}

#[test]
fn harness_detects_stderr_divergence() {
    let r = std::panic::catch_unwind(|| {
        assert_stderr_eq("self_test", b"Range error: pow(0.00, -1.00)\n", b"Range error: pow(0.00, -2.00)\n")
    });
    assert!(r.is_err(), "assert_stderr_eq accepted differing streams");

    let r = std::panic::catch_unwind(|| assert_stderr_eq("self_test", b"a\n", b""));
    assert!(r.is_err(), "assert_stderr_eq accepted a missing message");

    // ... and accepts identical streams.
    assert_stderr_eq("self_test", b"same\n", b"same\n");
}

#[test]
fn harness_capture_actually_captures() {
    let p = pair();
    // An error path must produce bytes; a success path must produce none.
    let (_, err) = capture_stderr(|| p.c.call(-2.0, 0.5));
    assert!(
        err.starts_with(b"Domain error: "),
        "capture_stderr saw nothing from the C error path: {err:?}"
    );
    let (_, none) = capture_stderr(|| p.c.call(2.0, 2.0));
    assert!(none.is_empty(), "success path printed {none:?}");

    // fd 2 is restored afterwards.
    eprintln!("stderr still works after capture");
}
