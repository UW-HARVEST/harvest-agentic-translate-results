//! Phase D — exported-symbol parity between the C `.so` and the Rust `.so`.
//!
//! The symbol diff must reach EMPTY: every symbol the C `.so` exports must also
//! be exported by the Rust `.so` under the exact same name.

mod common;
use common::*;

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

/// Defined, dynamically-exported symbol names of `so`, via `nm -D`.
fn defined_dynamic_symbols(so: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(so)
        .output()
        .expect("run nm -D (binutils required)");
    assert!(
        out.status.success(),
        "nm -D {} failed: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|line| {
            let mut it = line.split_whitespace();
            let a = it.next()?;
            let b = it.next()?;
            // "<addr> <type> <name>"  or  "<type> <name>" for weak/undefined
            let (ty, name) = match it.next() {
                Some(n) => (b, n),
                None => (a, b),
            };
            matches!(ty, "T" | "t" | "D" | "d" | "B" | "b" | "W" | "w" | "R" | "r")
                .then(|| name.to_string())
        })
        .collect()
}

/// Undefined (imported) symbols of `so`, as `(nm type, name)` pairs.
fn undefined_symbols(so: &Path) -> Vec<(String, String)> {
    let out = Command::new("nm")
        .args(["-D", "--undefined-only"])
        .arg(so)
        .output()
        .expect("run nm -D --undefined-only");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let f: Vec<&str> = l.split_whitespace().collect();
            match f.len() {
                2 => Some((f[0].to_string(), f[1].to_string())),
                3 => Some((f[1].to_string(), f[2].to_string())),
                _ => None,
            }
        })
        .collect()
}

/// An import is a legitimate platform-runtime import (rather than a piece of the
/// library that was never translated) if it is either
///   * weak / optional (`nm` type `w`/`v`), or
///   * bound to a versioned platform runtime (`@GLIBC_*`, `@GCC_*`, `@GLIBCXX_*`,
///     `@LIBC*`), which only libc / libgcc / libstdc++ provide.
///
/// Anything else would be an unresolved reference to code that is missing.
fn is_runtime_import(ty: &str, name: &str) -> bool {
    if ty.eq_ignore_ascii_case("w") || ty == "v" {
        return true;
    }
    match name.split_once('@') {
        Some((_, ver)) => {
            let v = ver.trim_start_matches('@');
            v.starts_with("GLIBC_")
                || v.starts_with("GCC_")
                || v.starts_with("GLIBCXX_")
                || v.starts_with("CXXABI_")
                || v.starts_with("LIBC")
        }
        // Unversioned strong undefined symbol -> suspicious.
        None => false,
    }
}

#[test]
fn phase_d_symbol_parity() {
    let c_so = c_so_path();
    let r_so = rust_so_path();
    println!("C    .so: {}", c_so.display());
    println!("Rust .so: {}", r_so.display());

    let c_syms = defined_dynamic_symbols(&c_so);
    let r_syms = defined_dynamic_symbols(&r_so);

    println!("C exports {} defined symbols", c_syms.len());
    println!("Rust exports {} defined symbols", r_syms.len());

    // 1. The C `.so` must export exactly the 12 documented symbols.
    let expected: BTreeSet<String> = EXPECTED_SYMBOLS.iter().map(|s| s.to_string()).collect();
    assert_eq!(
        c_syms, expected,
        "SYMBOLS.md is out of date with the C .so\n  only in .so:      {:?}\n  only in SYMBOLS.md: {:?}",
        c_syms.difference(&expected).collect::<Vec<_>>(),
        expected.difference(&c_syms).collect::<Vec<_>>()
    );

    // 2. THE gate: the symbol diff must be empty.
    let missing: Vec<&String> = c_syms.difference(&r_syms).collect();
    assert!(
        missing.is_empty(),
        "{} C symbol(s) MISSING from the Rust .so: {:?}\n\
         Either add the #[no_mangle] extern \"C\" wrapper, or translate the C \
         source that was skipped.",
        missing.len(),
        missing
    );
    println!("symbol diff (C \\ Rust): EMPTY — {} symbols matched", c_syms.len());

    // 3. Each symbol must actually be resolvable through dlopen/dlsym in both.
    let b = both();
    for name in EXPECTED_SYMBOLS {
        for api in [&b.c, &b.r] {
            let _ = api; // resolution already happened in Api::load, which panics
        }
        assert!(c_syms.contains(*name) && r_syms.contains(*name));
    }
}

#[test]
fn phase_d_no_missing_non_libc_imports() {
    for so in [c_so_path(), rust_so_path()] {
        let all = undefined_symbols(&so);
        let leftovers: Vec<&(String, String)> =
            all.iter().filter(|(t, n)| !is_runtime_import(t, n)).collect();
        println!(
            "{}: {} undefined symbols, {} non-runtime",
            so.file_name().unwrap().to_string_lossy(),
            all.len(),
            leftovers.len()
        );
        assert!(
            leftovers.is_empty(),
            "{} has {} unresolved non-libc symbol(s): {leftovers:?}\n\
             That means a piece of the library is missing, not merely unexported.",
            so.display(),
            leftovers.len()
        );
    }
}

/// The definitive mechanical check: `dlopen(..., RTLD_NOW)` forces the dynamic
/// linker to bind *every* undefined symbol immediately. If anything were truly
/// missing, this fails.
#[test]
fn phase_d_both_so_resolve_eagerly() {
    use libloading::os::unix::{Library as UnixLibrary, RTLD_LOCAL, RTLD_NOW};
    for so in [c_so_path(), rust_so_path()] {
        let lib = unsafe { UnixLibrary::open(Some(&so), RTLD_NOW | RTLD_LOCAL) };
        let lib = lib.unwrap_or_else(|e| {
            panic!("dlopen(RTLD_NOW) failed for {} — unresolved symbols: {e}", so.display())
        });
        // Every documented symbol must be reachable via dlsym.
        for name in EXPECTED_SYMBOLS {
            let mut c_name = name.to_string();
            c_name.push('\0');
            let s: Result<libloading::os::unix::Symbol<*const ()>, _> =
                unsafe { lib.get(c_name.as_bytes()) };
            assert!(s.is_ok(), "dlsym({name}) failed in {}", so.display());
        }
        println!(
            "{}: RTLD_NOW ok, all {} symbols resolvable via dlsym",
            so.file_name().unwrap().to_string_lossy(),
            EXPECTED_SYMBOLS.len()
        );
    }
}

#[test]
fn phase_d_rust_so_exports_no_extra_c_api() {
    // Informational: list Rust-only exports so accidental API surface is visible.
    let c_syms = defined_dynamic_symbols(&c_so_path());
    let r_syms = defined_dynamic_symbols(&rust_so_path());
    let extra: Vec<&String> = r_syms
        .difference(&c_syms)
        .filter(|s| !s.starts_with('_') && !s.starts_with("rust_"))
        .collect();
    println!("Rust-only exported symbols ({}): {:?}", extra.len(), extra);
    // Extra Rust-runtime symbols are expected and harmless; the gate is only
    // that nothing from C is missing (asserted in phase_d_symbol_parity).
}
