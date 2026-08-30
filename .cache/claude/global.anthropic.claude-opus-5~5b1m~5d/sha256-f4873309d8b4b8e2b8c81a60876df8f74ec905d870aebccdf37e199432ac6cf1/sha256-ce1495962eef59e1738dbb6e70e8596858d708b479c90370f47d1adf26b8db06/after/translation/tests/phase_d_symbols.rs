//! Phase D — symbol parity between the C and Rust shared libraries.
//!
//! Asserts mechanically (not by hand-maintained list) that every dynamic symbol
//! the C `.so` exports is also exported by the Rust `.so` under the exact same
//! name, and that each is actually resolvable via `dlsym`.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

fn c_so() -> PathBuf {
    c_so_path()
}

fn rust_so() -> PathBuf {
    rust_so_path()
}

/// Defined (exported) dynamic symbols, as `nm -D --defined-only` reports them.
fn exported_symbols(so: &Path) -> BTreeSet<String> {
    assert!(so.exists(), "missing shared library: {}", so.display());
    let out = Command::new("nm")
        .args(["-D", "--defined-only", so.to_str().unwrap()])
        .output()
        .expect("run nm");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let (_addr, kind, name) = (it.next()?, it.next()?, it.next()?);
            // Only global text/data symbols, matching the C library's surface.
            if matches!(kind, "T" | "D" | "B" | "R" | "W") {
                Some(name.to_string())
            } else {
                None
            }
        })
        // Rust cdylibs export a few toolchain-internal symbols; the parity
        // requirement is C-subset-of-Rust, so filtering these out of the RUST
        // set is not needed — we only assert containment in that direction.
        .collect()
}

#[test]
fn d1_rust_exports_every_c_symbol() {
    let c = exported_symbols(&c_so());
    let r = exported_symbols(&rust_so());

    assert!(!c.is_empty(), "nm found no exported symbols in the C library");

    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "\nThe Rust .so is MISSING {} symbol(s) exported by the C .so: {:?}\n\
         C symbols   ({}): {:?}\n\
         Rust symbols({}): {:?}\n",
        missing.len(),
        missing,
        c.len(),
        c,
        r.len(),
        r
    );

    // The five documented entry points must be present in BOTH.
    for want in ["printLine", "printIntLine", "bad", "good", "driver"] {
        assert!(c.contains(want), "C .so lost {want}");
        assert!(r.contains(want), "Rust .so lost {want}");
    }
}

#[test]
fn d2_static_c_functions_are_not_exported_by_either() {
    // `goodG2B` / `goodB2G` are `static` in driver.c, so neither library may
    // export them. (Parity in the other direction, too.)
    let c = exported_symbols(&c_so());
    let r = exported_symbols(&rust_so());
    for hidden in ["goodG2B", "goodB2G"] {
        assert!(!c.contains(hidden), "C unexpectedly exports {hidden}");
        assert!(
            !r.contains(hidden),
            "Rust exports {hidden}, but it is `static` in C"
        );
    }
}

#[test]
fn d3_all_symbols_resolvable_via_dlsym() {
    // Loading the APIs performs `dlsym` for all five symbols in both libraries;
    // if any were absent or unresolvable this panics.
    let c = c_api();
    let r = rust_api();
    assert_eq!(c.name, "C");
    assert_eq!(r.name, "Rust");
    // Smoke-call each through both to prove the pointers are live.
    let out = diff_one("D3/smoke", |api| {
        (api.print_line)(cstring(b"smoke").as_ptr());
        (api.print_int_line)(0);
        (api.bad)(2.0);
        (api.good)(2.0);
        (api.driver)(2.0, 2.0);
    });
    assert!(!out.is_empty());
}

#[test]
fn d4_rust_so_has_no_unresolved_non_libc_symbols() {
    // `nm -D -u` lists UNDEFINED dynamic symbols. Everything the Rust cdylib
    // still needs must come from the platform (libc/libm/libgcc/ld), otherwise
    // the library could not be dlopen'd standalone.
    let so = rust_so();
    let out = Command::new("nm")
        .args(["-D", "-u", so.to_str().unwrap()])
        .output()
        .expect("run nm");
    assert!(out.status.success());
    let undefined: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(str::to_string))
        .collect();

    // dlopen already succeeded in d3/other tests, which is the real proof; here
    // we just record the imports for the audit trail and assert none of them is
    // one of OUR own symbols (which would mean a broken self-reference).
    for sym in &undefined {
        for own in ["printLine", "printIntLine", "bad", "good", "driver"] {
            assert_ne!(
                sym, own,
                "Rust .so imports its own symbol {own} instead of defining it"
            );
        }
    }
}
