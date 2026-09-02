//! Harness smoke test + full `nm -D` symbol-parity assertion performed from
//! inside the test suite (Phase D gate, mechanised).

mod harness;
use harness::*;

use std::process::Command;

fn nm_syms(path: &str) -> Vec<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", path])
        .output()
        .expect("run nm");
    assert!(out.status.success(), "nm failed on {path}");
    let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().nth(2).map(|s| s.to_string()))
        .collect();
    v.sort();
    v.dedup();
    v
}

#[test]
fn harness_loads_both_libraries() {
    let (c, r) = sym::<unsafe extern "C" fn() -> *const std::ffi::c_char>("sodium_version_string");
    unsafe {
        let cs = std::ffi::CStr::from_ptr(c());
        let rs = std::ffi::CStr::from_ptr(r());
        assert_eq!(cs, rs, "sodium_version_string");
    }
}

#[test]
fn symbol_parity() {
    let manifest = env!("CARGO_MANIFEST_DIR");
    let cpath = std::env::var("C_SO")
        .unwrap_or_else(|_| format!("{manifest}/../c_src/build/libsodium.so"));
    let rrel = format!("{manifest}/target/release/liblibsodium.so");
    let rpath = std::env::var("RUST_SO").unwrap_or_else(|_| {
        if std::path::Path::new(&rrel).exists() {
            rrel.clone()
        } else {
            format!("{manifest}/target/debug/liblibsodium.so")
        }
    });

    let cs = nm_syms(&cpath);
    let rs = nm_syms(&rpath);

    let missing: Vec<&String> = cs.iter().filter(|s| !rs.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "{} C symbols missing from the Rust .so: {:?}",
        missing.len(),
        missing
    );
    // Sanity: we really did read a full libsodium symbol table.
    assert!(cs.len() > 800, "unexpectedly few C symbols: {}", cs.len());
}
