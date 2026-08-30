//! Phase D — symbol parity between the C `.so` and the Rust `.so`.
//!
//! Runs `nm -D --defined-only` on both objects and requires the diff of the C
//! side against the Rust side to be empty, so a partially translated library
//! cannot pass verification.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::process::Command;

/// Symbols the Rust toolchain adds to every `cdylib`, plus the libc/unwind
/// machinery. These are allowed to appear on the Rust side only.
fn is_toolchain_symbol(name: &str) -> bool {
    name.starts_with("_ZN")            // mangled Rust
        || name.starts_with("_R")      // v0 mangled Rust
        || name.starts_with("rust_")
        || name.starts_with("__rust")
        || name.starts_with("_Unwind_")
        || name.starts_with("__Unwind")
        || name.starts_with("_ITM_")
        || name.starts_with("__cxa")
        || name.starts_with("__gnu")
        || name.starts_with("_init")
        || name.starts_with("_fini")
        || name.starts_with("__bss_start")
        || name == "_edata"
        || name == "_end"
        || name.is_empty()
}

fn defined_dynamic_symbols(path: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", path.to_str().unwrap()])
        .output()
        .expect("failed to run `nm` — it is required for the Phase D check");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().nth(2).map(str::to_string))
        .collect()
}

/// The exact set the C library exports. Hard-coded from `nm -D` so that the
/// test also fails if the C `.so` is rebuilt with a different surface.
const EXPECTED_C_SYMBOLS: [&str; 2] = ["driver", "fma_array"];

#[test]
fn phase_d_c_symbol_surface_is_as_documented() {
    let c_syms = defined_dynamic_symbols(&c_so_path());
    let expected: BTreeSet<String> = EXPECTED_C_SYMBOLS.iter().map(|s| s.to_string()).collect();
    assert_eq!(
        c_syms, expected,
        "the C .so's exported surface changed; SYMBOLS.md must be regenerated"
    );
}

#[test]
fn phase_d_rust_exports_every_c_symbol() {
    let c_syms = defined_dynamic_symbols(&c_so_path());
    let rust_syms = defined_dynamic_symbols(&rust_so_path());

    let missing: Vec<&String> = c_syms.difference(&rust_syms).collect();
    assert!(
        missing.is_empty(),
        "the Rust .so is missing {} symbol(s) exported by the C .so: {missing:?}\n\
         C:    {c_syms:?}\n\
         Rust: {rust_syms:?}",
        missing.len()
    );
}

#[test]
fn phase_d_rust_exports_no_extra_public_symbols() {
    let c_syms = defined_dynamic_symbols(&c_so_path());
    let rust_syms = defined_dynamic_symbols(&rust_so_path());

    let extra: Vec<&String> = rust_syms
        .difference(&c_syms)
        .filter(|s| !is_toolchain_symbol(s))
        .collect();
    assert!(
        extra.is_empty(),
        "the Rust .so exports public symbols the C .so does not: {extra:?}"
    );
}

/// Every symbol must actually be callable through `dlsym`, not merely present
/// in the symbol table.
#[test]
fn phase_d_every_symbol_is_dlsym_resolvable() {
    let p = pair();
    for imp in [&p.c, &p.rust] {
        // Forces a panic if `dlsym` cannot resolve them.
        let _ = imp.driver_sym();
        let _ = imp.fma_sym();
    }
}

/// The Rust `.so` must not have unresolved non-libc dependencies.
#[test]
fn phase_d_rust_has_no_missing_non_libc_symbols() {
    let out = Command::new("nm")
        .args(["-D", "--undefined-only", rust_so_path().to_str().unwrap()])
        .output()
        .expect("failed to run nm");
    assert!(out.status.success());

    // Anything the dynamic linker resolves out of glibc or libgcc carries a
    // version tag (`name@GLIBC_x.y` / `name@GCC_x.y`). The only untagged
    // imports a `cdylib` legitimately has are these weak toolchain hooks.
    const ALLOWED_UNVERSIONED: [&str; 3] = [
        "_ITM_deregisterTMCloneTable",
        "_ITM_registerTMCloneTable",
        "__gmon_start__",
    ];

    let undefined: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(str::to_string))
        .collect();

    let unresolved: Vec<&String> = undefined
        .iter()
        .filter(|s| {
            !(s.contains("@GLIBC")
                || s.contains("@GCC")
                || ALLOWED_UNVERSIONED.contains(&s.as_str()))
        })
        .collect();

    assert!(
        unresolved.is_empty(),
        "Rust .so has undefined symbols that are not libc/runtime: {unresolved:?}"
    );

    // The translation must import the platform `printf`, not reimplement
    // formatting, so that `driver`'s output is byte-identical to the C's.
    assert!(
        undefined.iter().any(|s| s.starts_with("printf")),
        "Rust .so does not import libc `printf`; formatting may diverge from the C. \
         Undefined symbols: {undefined:?}"
    );
}
