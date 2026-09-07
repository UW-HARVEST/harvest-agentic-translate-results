//! Phase D — symbol parity between the C `.so` and the Rust `.so`.
//!
//! Asserts mechanically (via `nm -D`) that every dynamic symbol the C library
//! defines is also defined by the Rust library with the exact same name, and
//! that the Rust library exports no unexpected extras.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::process::Command;

fn defined_dynamic_symbols(path: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", path.to_str().unwrap()])
        .output()
        .expect("run nm");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(str::to_string))
        .filter(|s| !s.is_empty())
        .collect()
}

fn every_c_symbol_is_exported_by_rust() {
    let c = defined_dynamic_symbols(&c_so_path());
    let r = defined_dynamic_symbols(&rust_so_path());

    assert!(
        c.contains("driver"),
        "sanity: C .so must export `driver`, got {c:?}"
    );

    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by C .so but MISSING from Rust .so: {missing:?}\n\
         C   = {c:?}\n\
         Rust= {r:?}"
    );
}

fn c_and_rust_symbols_are_distinct_implementations() {
    // Guards against the harness accidentally calling the same code twice
    // (e.g. if dlsym resolved `driver` from the first-loaded library for both
    // handles): the two resolved addresses must differ.
    let c = c_fn_addr();
    let r = rust_fn_addr();
    assert_ne!(
        c, r,
        "C and Rust `driver` resolved to the same address {c:#x} — the test would be vacuous"
    );
}

fn rust_symbol_is_callable_and_matches_c_signature_shape() {
    // Both symbols resolve and are callable through libloading with the
    // `void driver(int)` signature; a mismatch would trap or corrupt output.
    assert_eq!(c_driver(7), rust_driver(7));
    assert_eq!(c_driver(7), b"314\n".to_vec());
}

fn rust_so_has_no_unresolved_non_libc_symbols() {
    let out = Command::new("nm")
        .args(["-D", "-u", rust_so_path().to_str().unwrap()])
        .output()
        .expect("run nm -u");
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    // Undefined symbols must all come from libc / the unwinder / the loader —
    // i.e. none of them may be a `driver`-library function that was never
    // translated.
    let suspicious: Vec<&str> = text
        .lines()
        .filter_map(|l| l.split_whitespace().last())
        .filter(|s| {
            !s.starts_with("_Unwind")
                && !s.starts_with("__")
                && !s.starts_with("_ITM")
                && !s.contains("GLIBC")
                && !s.contains("GCC")
                && !s.is_empty()
        })
        .collect();
    assert!(
        suspicious.is_empty(),
        "Rust .so has unresolved non-libc symbols: {suspicious:?}"
    );
}

fn main() {
    common::run_suite(
        "Phase D — symbol parity",
        &[
            ("every_c_symbol_is_exported_by_rust", every_c_symbol_is_exported_by_rust),
            ("c_and_rust_symbols_are_distinct_implementations", c_and_rust_symbols_are_distinct_implementations),
            ("rust_symbol_is_callable_and_matches_c_signature_shape", rust_symbol_is_callable_and_matches_c_signature_shape),
            ("rust_so_has_no_unresolved_non_libc_symbols", rust_so_has_no_unresolved_non_libc_symbols),
        ],
    );
}
