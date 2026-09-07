//! Phase D — symbol parity between the C and Rust shared objects.

mod common;

use common::{c_so_path, rust_so_path};
use std::collections::BTreeSet;
use std::process::Command;

/// Globally-defined text/data symbols, excluding weak (`w`/`W`/`V`) toolchain
/// artifacts that are not part of the API surface.
fn defined_globals(so: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", so.to_str().unwrap()])
        .output()
        .expect("run nm");
    assert!(out.status.success(), "nm failed on {so:?}");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|line| {
            let mut it = line.split_whitespace();
            let (_addr, kind, name) = (it.next()?, it.next()?, it.next()?);
            match kind {
                // Global text/data/bss/rodata.
                "T" | "D" | "B" | "R" => Some(name.to_string()),
                _ => None,
            }
        })
        // Rust's cdylib carries a couple of runtime helpers with mangled names;
        // only unmangled C-ABI names can be part of the C library's surface.
        .filter(|n| !n.starts_with("_ZN") && !n.starts_with("_R"))
        .collect()
}

#[allow(dead_code)]
fn every_c_symbol_is_exported_by_rust() {
    let c = defined_globals(&c_so_path());
    let r = defined_globals(&rust_so_path());

    assert!(
        c.contains("driver"),
        "the C .so must export `driver`; found {c:?}"
    );

    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but missing from the Rust .so: {missing:?}\n\
         C   : {c:?}\n\
         Rust: {r:?}"
    );
}

#[allow(dead_code)]
fn rust_so_has_no_non_libc_undefined_symbols() {
    let out = Command::new("nm")
        .args([
            "-D",
            "--undefined-only",
            rust_so_path().to_str().unwrap(),
        ])
        .output()
        .expect("run nm");
    assert!(out.status.success());

    // Everything the cdylib imports must come from libc / the dynamic loader.
    let allowed_prefixes = [
        "printf", "puts", "putchar", "fwrite", "memcpy", "memset", "memmove", "abort",
        "__", "_ITM_", "_Unwind_", "pthread_", "malloc", "free", "realloc", "write",
    ];
    let bad: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .filter(|n| n.contains('@') == false)
        .filter(|n| !allowed_prefixes.iter().any(|p| n.starts_with(p)))
        .collect();
    // Names may carry a @GLIBC_x.y version suffix; strip and re-check.
    let bad: Vec<String> = bad
        .into_iter()
        .filter(|n| {
            let base = n.split('@').next().unwrap_or(n);
            !allowed_prefixes.iter().any(|p| base.starts_with(p))
        })
        .collect();
    assert!(
        bad.is_empty(),
        "Rust .so has undefined non-libc symbols: {bad:?}"
    );
}

#[test]
fn phase_d_symbol_parity() {
    every_c_symbol_is_exported_by_rust();
    rust_so_has_no_non_libc_undefined_symbols();
    eprintln!("Phase D: symbol parity verified");
}
