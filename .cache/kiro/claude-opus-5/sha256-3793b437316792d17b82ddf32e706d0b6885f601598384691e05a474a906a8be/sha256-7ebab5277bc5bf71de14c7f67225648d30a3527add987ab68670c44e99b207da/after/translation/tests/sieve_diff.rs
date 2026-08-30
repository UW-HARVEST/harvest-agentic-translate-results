//! Differential tests: C `libSieve.so` vs Rust `libSieve.so`, both loaded via
//! `libloading` and called only through their exported symbols.
//!
//! The public API (see `c_src/include/sieve.h`) is a single function,
//! `void sieve(int)`, so there is no deeper call hierarchy to walk: the
//! low-level unit *is* the exported entry point.
//!
//! Everything lives in one `#[test]`: `sieve` reports its results through
//! `printf`, so the harness has to capture file descriptor 1, and libtest's own
//! per-test progress output from a concurrently running test would otherwise
//! land inside a capture window.

mod common;

use common::{assert_same, Libs};
use std::collections::BTreeSet;
use std::process::Command;

/// `sieve` counts upward one at a time, so any input whose next "…9" value lies
/// above `i32::MAX` would make the C original increment past `INT_MAX` (signed
/// overflow, undefined behaviour). The largest input that terminates without
/// overflowing is `i32::MAX - 8`.
const MAX_SAFE: i32 = i32::MAX - 8; // 2_147_483_639

#[test]
fn c_and_rust_agree() {
    exports_are_a_superset_of_c();

    let libs = Libs::load();
    harness_self_check(&libs);
    small_non_negative_inputs(&libs);
    values_ending_in_nine(&libs);
    negative_inputs(&libs);
    powers_of_ten_and_wide_values(&libs);
    pseudo_random_inputs(&libs);
}

/// Guards against a silently broken capture (e.g. both sides returning nothing,
/// which would make every comparison trivially pass) by checking both libraries
/// against literal expected bytes.
fn harness_self_check(libs: &Libs) {
    let cases: &[(i32, &str)] = &[
        (7, "7\n8\n9\n"),
        (9, "9\n"),
        (10, "10\n11\n12\n13\n14\n15\n16\n17\n18\n19\n"),
        (-3, "-3\n-2\n-1\n0\n1\n2\n3\n4\n5\n6\n7\n8\n9\n"),
    ];
    for &(val, expected) in cases {
        let c = libs.c_sieve();
        let rust = libs.rust_sieve();
        let c_out = common::capture_stdout(|| unsafe { c(val) });
        let rust_out = common::capture_stdout(|| unsafe { rust(val) });
        assert_eq!(
            String::from_utf8_lossy(&c_out),
            expected,
            "C sieve({val}) did not produce the expected bytes; capture harness is suspect"
        );
        assert_eq!(String::from_utf8_lossy(&rust_out), expected, "Rust sieve({val})");
    }
}

/// Step 8: every dynamic symbol the C `.so` exports must also be exported by
/// the Rust `.so`, under the same name.
fn exports_are_a_superset_of_c() {
    let dynamic_symbols = |path: &std::path::Path| -> BTreeSet<String> {
        let out = Command::new("nm")
            .args(["-D", "--defined-only", "--format=posix"])
            .arg(path)
            .output()
            .expect("failed to run nm");
        assert!(
            out.status.success(),
            "nm failed on {}: {}",
            path.display(),
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter_map(|l| l.split_whitespace().next().map(str::to_string))
            .collect()
    };

    let c = dynamic_symbols(&common::c_lib_path());
    let rust = dynamic_symbols(&common::rust_lib_path());

    assert!(c.contains("sieve"), "C .so unexpectedly lacks `sieve`: {c:?}");

    let missing: Vec<_> = c.difference(&rust).cloned().collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing symbols exported by the C .so: {missing:?}\n  C: {c:?}\n  Rust: {rust:?}"
    );
}

/// Covers every residue class mod 10 many times over, including the
/// immediate-break case (values ending in 9) and full ten-step runs.
fn small_non_negative_inputs(libs: &Libs) {
    for val in 0..=200 {
        assert_same(libs, val);
    }
}

fn values_ending_in_nine(libs: &Libs) {
    for val in [9, 19, 99, 109, 999, 1_000_009, MAX_SAFE] {
        assert_same(libs, val);
    }
}

/// C's `%` truncates toward zero, so a negative multiple-of-ten-plus-nine such
/// as -19 yields -9, never 9: the loop keeps counting up through zero and only
/// stops at 9. These inputs pin that behaviour down.
fn negative_inputs(libs: &Libs) {
    for val in -200..0 {
        assert_same(libs, val);
    }
    for val in [-1_000, -1_009, -9_999, -12_345] {
        assert_same(libs, val);
    }
}

fn powers_of_ten_and_wide_values(libs: &Libs) {
    let mut vals = vec![
        1_000,
        1_000_000,
        1_000_000_000,
        2_000_000_000,
        MAX_SAFE - 9,
        MAX_SAFE - 1,
        MAX_SAFE,
    ];
    for p in [10i32, 100, 1_000, 10_000, 100_000, 1_000_000] {
        vals.extend([p - 1, p, p + 1, -p - 1, -p, -p + 1]);
    }
    for val in vals {
        assert_same(libs, val);
    }
}

fn pseudo_random_inputs(libs: &Libs) {
    // Deterministic xorshift so failures are reproducible; no rand dependency.
    let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };

    for _ in 0..300 {
        // Full-range positives up to the last input the C code can handle
        // without signed overflow.
        let val = (next() % (MAX_SAFE as u64 + 1)) as i32;
        assert_same(libs, val);
    }
    for _ in 0..300 {
        // Negatives are kept small in magnitude: the loop walks up to 9 one
        // step at a time, so large negatives would print billions of lines.
        let val = -((next() % 5_000) as i32);
        assert_same(libs, val);
    }
}
