//! Phase D — exported-symbol parity between the C `.so` and the Rust `cdylib`.
//!
//! Everything the C shared library exports must also be exported by the Rust
//! shared library under the exact same name, and must be reachable through
//! `dlsym`.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

/// Reserved / CRT-generated names that are not part of the library API.
const RESERVED_PREFIXES: [&str; 1] = ["_"];

fn defined_dynamic_symbols(so: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(so)
        .output()
        .unwrap_or_else(|e| panic!("running nm on {}: {e}", so.display()));
    assert!(
        out.status.success(),
        "nm -D --defined-only {} failed: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .collect()
}

fn api_symbols(syms: &BTreeSet<String>) -> BTreeSet<String> {
    syms.iter()
        .filter(|s| !RESERVED_PREFIXES.iter().any(|p| s.starts_with(p)))
        .cloned()
        .collect()
}

#[test]
fn sym_every_c_symbol_is_exported_by_rust() {
    let rust = defined_dynamic_symbols(&rust_so());
    for (tag, so) in c_so_variants() {
        let c_api = api_symbols(&defined_dynamic_symbols(&so));
        assert!(
            !c_api.is_empty(),
            "no API symbols found in the C[{tag}] .so -- nm parsing is broken"
        );
        let missing: Vec<&String> = c_api.iter().filter(|s| !rust.contains(*s)).collect();
        assert!(
            missing.is_empty(),
            "Rust .so {} is missing symbols exported by the C[{tag}] .so: {missing:?}\n\
             C API symbols: {c_api:?}",
            rust_so().display()
        );
    }
}

#[test]
fn sym_expected_symbol_set() {
    // Pinned so that a future C addition that goes untranslated fails loudly
    // instead of silently shrinking the comparison.
    let expected: BTreeSet<String> = ["call_fma", "fma_array", "main"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    for (tag, so) in c_so_variants() {
        let c_api = api_symbols(&defined_dynamic_symbols(&so));
        assert_eq!(
            c_api, expected,
            "the C[{tag}] .so's API surface changed; update SYMBOLS.md and the tests"
        );
    }
    let rust_api = api_symbols(&defined_dynamic_symbols(&rust_so()));
    assert_eq!(
        rust_api, expected,
        "the Rust .so's API surface does not match the C's"
    );
}

#[test]
fn sym_no_unresolved_symbols_in_rust_so() {
    // RTLD_NOW: dlopen fails outright if anything is unresolved.
    let lib = unsafe {
        libloading::os::unix::Library::open(
            Some(rust_so()),
            libloading::os::unix::RTLD_NOW | libloading::os::unix::RTLD_LOCAL,
        )
    };
    let lib = lib.unwrap_or_else(|e| panic!("dlopen(RTLD_NOW) of the Rust .so failed: {e}"));
    for name in ["fma_array", "call_fma", "main"] {
        let mut n = name.as_bytes().to_vec();
        n.push(0);
        let s = unsafe { lib.get::<*const ()>(&n) };
        assert!(s.is_ok(), "dlsym({name}) failed in the Rust .so");
    }
}

#[test]
fn sym_all_symbols_resolve_out_of_process_in_both() {
    let rp = probe(&rust_so(), "symbols", &[], None);
    assert_eq!(
        rp.code,
        Some(0),
        "Rust .so symbol resolution failed: {}",
        rp.describe()
    );
    for (tag, so) in c_so_variants() {
        let cp = probe(&so, "symbols", &[], None);
        assert_eq!(
            cp.code,
            Some(0),
            "C[{tag}] .so symbol resolution failed: {}",
            cp.describe()
        );
        assert_eq!(
            cp.stdout, rp.stdout,
            "symbol resolution differs between C[{tag}] and Rust\n  C   : {}\n  Rust: {}",
            cp.describe(),
            rp.describe()
        );
    }
}
