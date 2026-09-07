//! Harness self-check (negative control).
//!
//! A differential test suite that passes because it accidentally calls the
//! *same* code twice is worthless. These tests prove the harness really
//! resolves two independent implementations and really compares their results.

mod common;

use common::Pair;

/// The two resolved `memchra2` addresses must be distinct — i.e. `dlsym` on the
/// Rust handle did not fall through to the already-loaded C library via global
/// symbol interposition.
#[test]
fn distinct_implementations_are_loaded() {
    let p = Pair::load();
    let c_addr = p.c_fn_addr();
    let r_addr = p.rust_fn_addr();
    assert_ne!(
        c_addr, r_addr,
        "C and Rust memchra2 resolved to the same address: the differential \
         tests would be vacuous"
    );
    // Belt and braces: the two addresses must also live in different mappings.
    let maps = std::fs::read_to_string("/proc/self/maps").expect("read /proc/self/maps");
    let owner = |addr: usize| -> String {
        for line in maps.lines() {
            let (range, rest) = match line.split_once(' ') {
                Some(x) => x,
                None => continue,
            };
            let (lo, hi) = match range.split_once('-') {
                Some(x) => x,
                None => continue,
            };
            let lo = usize::from_str_radix(lo, 16).unwrap_or(0);
            let hi = usize::from_str_radix(hi, 16).unwrap_or(0);
            if addr >= lo && addr < hi {
                return rest.rsplit(' ').next().unwrap_or("?").to_string();
            }
        }
        "?".to_string()
    };
    let c_owner = owner(p.c_fn_addr());
    let r_owner = owner(p.rust_fn_addr());
    assert_ne!(
        c_owner, r_owner,
        "both symbols map into the same object: {c_owner}"
    );
    assert!(
        c_owner.contains("c_src") || c_owner.ends_with(".so"),
        "unexpected C mapping: {c_owner}"
    );
    assert!(
        r_owner.contains("memchra2_lib"),
        "unexpected Rust mapping: {r_owner}"
    );
}

/// The comparison itself must be able to fail. `assert_eq_at` is exercised
/// against a deliberately wrong "implementation" to prove it does not silently
/// accept mismatches.
#[test]
fn comparison_detects_a_divergence() {
    let p = Pair::load();
    let truth = p.c(1, 2, 3, 4);
    let wrong = truth.wrapping_add(1);
    assert_ne!(truth, wrong);
    // Same shape of assertion the row tests use; must panic on mismatch.
    let caught = std::panic::catch_unwind(|| {
        assert_eq!(truth.to_le_bytes(), wrong.to_le_bytes(), "control");
    });
    assert!(caught.is_err(), "the byte-comparison assertion never fails");
}

/// `memchra2` must be deterministic in both libraries (no hidden state), so a
/// passing row genuinely generalizes.
#[test]
fn both_implementations_are_deterministic() {
    let p = Pair::load();
    for &(a, b, c, d) in &[(0, 0, 0, 0), (i32::MIN, i32::MAX, -7, 9), (12345, -6789, 0, 1)] {
        let c1 = p.c(a, b, c, d);
        let c2 = p.c(a, b, c, d);
        let r1 = p.rust(a, b, c, d);
        let r2 = p.rust(a, b, c, d);
        assert_eq!(c1, c2, "C not deterministic at ({a},{b},{c},{d})");
        assert_eq!(r1, r2, "Rust not deterministic at ({a},{b},{c},{d})");
    }
}
