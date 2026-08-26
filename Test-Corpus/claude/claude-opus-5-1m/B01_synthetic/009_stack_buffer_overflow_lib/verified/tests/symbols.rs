//! Phase D — symbol parity between the C `.so` and the Rust `.so`.
//!
//! The check is mechanical (`nm -D`), so a symbol that is added to the C library
//! later, or a `#[no_mangle]` wrapper that is dropped from the Rust side, fails
//! the test suite instead of silently reducing coverage.

mod common;
use common::*;

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

fn nm(path: &Path, args: &[&str]) -> Vec<String> {
    let out = Command::new("nm")
        .args(args)
        .arg(path)
        .output()
        .unwrap_or_else(|e| panic!("failed to run nm on {}: {e}", path.display()));
    assert!(
        out.status.success(),
        "nm {args:?} {} failed: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .collect()
}

/// Names that a shared object always exports/imports because of the toolchain,
/// not because of the API under test.
fn is_runtime_noise(s: &str) -> bool {
    s.starts_with('_')
        || s.contains("@GLIBC")
        || s.contains("@GCC")
        || s.contains("@CXXABI")
        || matches!(s, "atexit" | "at_quick_exit")
}

fn defined_api_symbols(path: &Path) -> BTreeSet<String> {
    nm(path, &["-D", "--defined-only"])
        .into_iter()
        .filter(|s| !is_runtime_noise(s))
        .collect()
}

/// Every symbol the C `.so` exports must also be exported by the Rust `.so`,
/// with the exact same name.
#[test]
fn c_and_rust_export_the_same_symbols() {
    let c = defined_api_symbols(&c_so_path());
    let r = defined_api_symbols(&rust_so_path());

    // Sanity: the five documented entry points really are in the C .so.
    for expected in ["printLine", "printIntLine", "bad", "good", "driver"] {
        assert!(
            c.contains(expected),
            "C .so unexpectedly does not export {expected}; C symbols: {c:?}"
        );
    }

    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but missing from the Rust .so: {missing:?}\n\
         C: {c:?}\nRust: {r:?}"
    );

    // `static` C functions must stay unexported on both sides.
    for local in ["goodG2B", "goodB2G"] {
        assert!(!c.contains(local), "C .so should not export {local}");
        assert!(!r.contains(local), "Rust .so should not export {local}");
    }

    println!("C .so exports:    {c:?}");
    println!("Rust .so exports: {r:?}");
}

/// The Rust `.so` must not require anything beyond the C runtime, i.e. it has no
/// unresolved non-libc dependency (which would mean a module was left behind).
#[test]
fn rust_so_has_no_unexpected_undefined_symbols() {
    let undef: Vec<String> = nm(&rust_so_path(), &["-D", "-u"])
        .into_iter()
        .filter(|s| !is_runtime_noise(s))
        .filter(|s| {
            // plain libc names (no version tag on some toolchains)
            !matches!(
                s.as_str(),
                "printf"
                    | "puts"
                    | "putchar"
                    | "memcpy"
                    | "memmove"
                    | "memset"
                    | "malloc"
                    | "free"
                    | "realloc"
                    | "calloc"
                    | "abort"
                    | "write"
                    | "strlen"
            )
        })
        .collect();
    assert!(
        undef.is_empty(),
        "Rust .so has unexpected undefined symbols: {undef:?}"
    );
}

/// The `printf` import must be the same libc entry point in both libraries, so
/// formatting and stdout buffering behave identically.
#[test]
fn both_libraries_use_libc_printf() {
    let c_undef = nm(&c_so_path(), &["-D", "-u"]);
    let r_undef = nm(&rust_so_path(), &["-D", "-u"]);
    let uses_printf = |v: &[String]| {
        v.iter()
            .any(|s| s.starts_with("printf") || s.starts_with("puts"))
    };
    assert!(uses_printf(&c_undef), "C .so does not import printf/puts: {c_undef:?}");
    assert!(
        uses_printf(&r_undef),
        "Rust .so does not import printf/puts: {r_undef:?}"
    );
}
