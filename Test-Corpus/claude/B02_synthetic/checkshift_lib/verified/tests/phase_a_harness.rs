//! Phase A — self-checks on the differential harness itself.
//!
//! A differential suite is worthless if it silently compares a library against
//! itself, or if the stdout capture always yields nothing. These tests assert
//! the harness is actually discriminating before Phases B/C draw conclusions.

mod common;

use common::{c, capture, r};

const SYMBOLS: [&str; 10] = [
    "multiply_with_static",
    "add_with_static",
    "xor_operation",
    "shift_with_static",
    "get_operation",
    "execute_operation",
    "compute_checksum",
    "init_state",
    "apply_operation",
    "checkshift",
];

#[test]
fn a_two_distinct_shared_objects_are_loaded() {
    assert_ne!(
        c().path.canonicalize().unwrap(),
        r().path.canonicalize().unwrap(),
        "the harness must load two different files"
    );
    assert!(
        c().path.to_string_lossy().contains("c_src"),
        "C lib should come from c_src/build, got {}",
        c().path.display()
    );
    assert!(
        r().path.to_string_lossy().contains("checkshift_lib"),
        "Rust lib should be libcheckshift_lib.so, got {}",
        r().path.display()
    );
    eprintln!("C   : {}", c().path.display());
    eprintln!("Rust: {}", r().path.display());
}

#[test]
fn a_every_symbol_resolves_to_a_distinct_address_per_library() {
    // If dlsym were resolving both handles to the same object (e.g. through
    // global symbol interposition) every differential test would be vacuous.
    let ca = c().all_symbol_addrs();
    let ra = r().all_symbol_addrs();
    for (i, name) in SYMBOLS.iter().enumerate() {
        assert_ne!(
            ca[i], ra[i],
            "symbol {name} resolved to the same address in both .so files \
             ({:#x}) - the differential comparison would be vacuous",
            ca[i]
        );
        assert_ne!(ca[i], 0, "symbol {name} resolved to NULL in the C .so");
        assert_ne!(ra[i], 0, "symbol {name} resolved to NULL in the Rust .so");
    }
    // and within a library all ten are distinct functions
    let mut u = ca.to_vec();
    u.sort_unstable();
    u.dedup();
    assert_eq!(u.len(), 10, "C .so symbols are not all distinct");
    let mut u = ra.to_vec();
    u.sort_unstable();
    u.dedup();
    assert_eq!(u.len(), 10, "Rust .so symbols are not all distinct");
}

#[test]
fn a_all_ten_symbols_are_present_in_both() {
    // `Lib::open` panics on a missing symbol, so simply constructing both
    // libraries proves all ten resolved. Assert the count explicitly.
    assert_eq!(SYMBOLS.len(), 10);
    let _ = c();
    let _ = r();
}

#[test]
fn a_capture_actually_captures_and_restores() {
    // non-empty, and clearly the library's transcript
    let (v, out) = capture(|| c().checkshift(1, 2, 3, 4));
    assert!(!out.is_empty(), "capture returned no bytes");
    assert!(
        out.starts_with(b"\n=== Starting foo function ==="),
        "unexpected transcript start: {}",
        common::show(&out)
    );
    assert!(
        out.ends_with(b"=== Ending foo function ===\n\n"),
        "unexpected transcript end: {}",
        common::show(&out)
    );
    assert!(out.len() > 200, "transcript suspiciously short: {}", out.len());
    let _ = v;

    // fd 1 is usable again afterwards
    println!("(harness: fd 1 restored)");

    // nested/sequential captures stay independent
    let (_, a) = capture(|| c().checkshift(1, 1, 1, 1));
    let (_, b) = capture(|| c().checkshift(2, 2, 2, 2));
    assert_ne!(a, b, "two different calls produced identical transcripts");

    // a closure that prints nothing must capture nothing
    let (_, empty) = capture(|| c().get_operation(0));
    assert!(empty.is_empty(), "expected no output, got {}", common::show(&empty));
}

#[test]
fn a_capture_is_not_swallowing_the_libraries_buffered_output() {
    // Both .so files share this process's glibc stdout. Verify that output
    // produced by the *Rust* .so lands in the capture too (not just the C one).
    let (_, out) = capture(|| r().checkshift(7, 8, 9, 10));
    assert!(
        out.contains_str(b"Result of SHIFT:"),
        "Rust .so output missing from capture: {}",
        common::show(&out)
    );
}

/// tiny helper for readability above
trait ContainsStr {
    fn contains_str(&self, needle: &[u8]) -> bool;
}
impl ContainsStr for Vec<u8> {
    fn contains_str(&self, needle: &[u8]) -> bool {
        self.windows(needle.len()).any(|w| w == needle)
    }
}

#[test]
fn a_differential_helper_detects_an_injected_difference() {
    // Prove `diff`'s comparison logic is not vacuous: comparing the C library
    // against itself with deliberately different arguments must be detected.
    let (v1, o1) = capture(|| c().checkshift(1, 2, 3, 4));
    let (v2, o2) = capture(|| c().checkshift(4, 3, 2, 1));
    assert_ne!(
        (v1, &o1),
        (v2, &o2),
        "the comparison would not notice differing behaviour"
    );

    // ... and comparing identical calls across the two libraries matches.
    let (v3, o3) = capture(|| c().checkshift(1, 2, 3, 4));
    let (v4, o4) = capture(|| r().checkshift(1, 2, 3, 4));
    assert_eq!(v3, v4);
    assert_eq!(o3, o4);
}
