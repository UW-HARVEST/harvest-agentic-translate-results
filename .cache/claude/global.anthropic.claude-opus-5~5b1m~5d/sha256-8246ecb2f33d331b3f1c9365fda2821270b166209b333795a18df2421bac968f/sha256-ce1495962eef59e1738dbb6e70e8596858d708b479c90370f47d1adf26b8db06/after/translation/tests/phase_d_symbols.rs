//! Phase D — symbol parity enforced as an executable test.
//!
//! Runs `nm -D` on both shared objects and requires that the set of DEFINED
//! symbols exported by the C `.so` is a subset of those exported by the Rust
//! `.so`. The diff must be empty.

mod harness;
use harness::*;

use std::collections::BTreeSet;
use std::process::Command;

fn defined_symbols(so: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(so)
        .output()
        .expect("failed to run `nm` (binutils required)");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn symbol_parity_diff_is_empty() {
    let c = defined_symbols(&c_so_path());
    let r = defined_symbols(&rust_so_path());

    assert!(
        c.contains("driver"),
        "C .so does not export `driver`; symbol extraction is broken: {c:?}"
    );

    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing {} symbol(s) exported by the C .so: {:?}\n\
         C exports:    {:?}\n\
         Rust exports: {:?}",
        missing.len(),
        missing,
        c,
        r
    );
}

#[test]
fn rust_so_has_no_unresolved_symbols() {
    // `ldd -r` reports undefined symbols that would fail at load time.
    let out = Command::new("ldd")
        .arg("-r")
        .arg(rust_so_path())
        .output()
        .expect("failed to run `ldd`");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let bad: Vec<&str> = text
        .lines()
        .filter(|l| l.to_lowercase().contains("undefined symbol"))
        .collect();
    assert!(bad.is_empty(), "Rust .so has unresolved symbols: {bad:?}");
}

#[test]
fn both_libraries_export_callable_driver() {
    // Proves the symbol found by `nm` is genuinely callable through dlsym in
    // both libraries (i.e. the Rust `#[no_mangle] extern "C"` wrapper works).
    let c = c_driver();
    let r = rust_driver();
    assert!(!std::ptr::eq(c as *const (), r as *const ()));
    assert_same("D/callable", &[42]);
}
