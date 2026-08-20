//! Harness self-checks and Phase D symbol parity, automated as tests.

mod common;

use common::*;
use std::process::Command;

/// Both `.so`s must be distinct files and must resolve to distinct
/// implementations of `hex2bin` (otherwise the differential tests would be
/// comparing an implementation against itself and could never fail).
#[test]
fn harness_loads_two_distinct_implementations() {
    let (c_path, rust_path) = so_paths();
    assert_ne!(c_path, rust_path);
    eprintln!("C   .so: {c_path:?}");
    eprintln!("RS  .so: {rust_path:?}");
    let (c, r) = impls();
    assert_ne!(c as usize, r as usize);
}

/// The harness must be able to observe a difference at all: feed a case whose
/// expected result is known and verify both agree with the hand-computed value.
#[test]
fn harness_detects_known_values() {
    let out = check_and_get(&Case::new(b"0f10ffAB".to_vec()).bin_maxlen(4));
    assert_eq!(out.ret, 4);
    assert_eq!(&out.bin[..4], &[0x0f, 0x10, 0xff, 0xab]);
}

fn nm_symbols(path: &std::path::Path) -> Option<Vec<String>> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", path.to_str().unwrap()])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let mut syms: Vec<String> = text
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .collect();
    syms.sort();
    syms.dedup();
    Some(syms)
}

/// Phase D: every symbol exported by the C `.so` must also be exported by the
/// Rust `.so`, under the exact same name.
#[test]
fn symbol_parity() {
    let (c_path, rust_path) = so_paths();
    let (Some(c_syms), Some(rust_syms)) = (nm_symbols(&c_path), nm_symbols(&rust_path)) else {
        eprintln!("`nm` unavailable; skipping symbol parity check");
        return;
    };
    assert!(
        c_syms.contains(&"hex2bin".to_string()),
        "sanity: C .so must export hex2bin, got {c_syms:?}"
    );
    let missing: Vec<&String> = c_syms.iter().filter(|s| !rust_syms.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but missing from the Rust .so: {missing:?}"
    );
    eprintln!("C symbols: {c_syms:?}\nRust exports all of them.");
}
