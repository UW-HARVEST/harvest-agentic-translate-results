//! Harness smoke test plus full exported-symbol parity check between the C
//! reference .so and the Rust .so.
mod common;

use common::*;
use std::process::Command;

#[test]
fn harness_loads_both_libraries() {
    cmp_cstr("sodium_version_string");
    cmp_int("sodium_library_version_major");
    cmp_int("sodium_library_version_minor");
    cmp_int("sodium_library_minimal");
}

fn exported_symbols(path: &std::path::Path) -> Vec<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(path)
        .output()
        .expect("run nm");
    assert!(out.status.success(), "nm failed on {path:?}");
    let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().nth(2).map(|s| s.to_string()))
        .collect();
    v.sort();
    v.dedup();
    v
}

/// Every symbol the C .so exports must also be exported by the Rust .so under
/// the exact same name.
#[test]
fn exported_symbols_match() {
    let c_path = c_so_path();
    if !c_path.exists() {
        panic!("C reference .so not built at {c_path:?}");
    }
    let rs_path = rust_so_path();

    let c_syms = exported_symbols(&c_path);
    let rs_syms = exported_symbols(&rs_path);

    let missing: Vec<&String> = c_syms.iter().filter(|s| !rs_syms.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing {} symbol(s) exported by the C .so:\n{}",
        missing.len(),
        missing
            .iter()
            .map(|s| format!("  {s}"))
            .collect::<Vec<_>>()
            .join("\n")
    );
    eprintln!("symbol parity OK: {} C symbols all present in Rust", c_syms.len());
}
