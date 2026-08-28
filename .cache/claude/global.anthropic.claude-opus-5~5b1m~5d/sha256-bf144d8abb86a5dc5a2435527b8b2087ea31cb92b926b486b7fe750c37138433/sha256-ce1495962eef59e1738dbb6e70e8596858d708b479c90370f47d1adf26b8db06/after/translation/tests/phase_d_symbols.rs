//! Phase D — automated symbol-parity gate.
//!
//! Asserts that every dynamic symbol the C `libdriver.so` exports is also
//! exported by the Rust `libdriver.so`, under the exact same name. This is the
//! machine-checked version of `SYMBOLS.md`, so the gate cannot silently
//! regress.

mod common;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so() -> PathBuf {
    if let Some(p) = std::env::var_os("DRIVER_C_SO") {
        return PathBuf::from(p);
    }
    crate_root().parent().unwrap().join("c_src/build/libdriver.so")
}

fn rust_so() -> PathBuf {
    if let Some(p) = std::env::var_os("DRIVER_RUST_SO") {
        return PathBuf::from(p);
    }
    let base = crate_root().join("target");
    let a = base.join(if cfg!(debug_assertions) { "debug" } else { "release" }).join("libdriver.so");
    if a.is_file() {
        a
    } else {
        base.join("release/libdriver.so")
    }
}

/// Defined (exported) dynamic symbol names from `nm -D --defined-only`.
fn defined_symbols(so: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(so)
        .output()
        .unwrap_or_else(|e| panic!("failed to run `nm` on {}: {e}", so.display()));
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().nth(2))
        // Ignore linker/CRT bookkeeping that is not part of the library's API.
        .filter(|s| {
            !s.starts_with("_init")
                && !s.starts_with("_fini")
                && !s.starts_with("__bss_start")
                && !s.starts_with("_edata")
                && !s.starts_with("_end")
        })
        .map(str::to_owned)
        .collect()
}

#[test]
fn every_c_symbol_is_exported_by_rust() {
    let c_path = c_so();
    let rust_path = rust_so();
    assert!(c_path.is_file(), "missing C .so at {}", c_path.display());
    assert!(rust_path.is_file(), "missing Rust .so at {}", rust_path.display());

    let c = defined_symbols(&c_path);
    let rust = defined_symbols(&rust_path);

    let missing: Vec<_> = c.difference(&rust).cloned().collect();
    assert!(
        missing.is_empty(),
        "the Rust .so is missing {} symbol(s) exported by the C .so: {missing:?}\n\
         C exports:    {c:?}\nRust exports: {rust:?}",
        missing.len()
    );

    // The C library's whole API is this one function; assert it is really there
    // rather than trusting an empty-set comparison.
    assert!(
        c.contains("tool_basename"),
        "sanity check failed: the C .so does not export tool_basename ({c:?})"
    );
    assert!(rust.contains("tool_basename"));
}

#[test]
fn exported_symbol_is_callable_through_dlsym_in_both() {
    // Symbol presence is not enough — resolve and call it in both objects.
    let c = common::c_driver();
    let r = common::rust_driver();
    let out_c = common::call(c, b"/a/b/c");
    let out_r = common::call(r, b"/a/b/c");
    assert_eq!(out_c.offset, 5);
    assert_eq!(out_c, out_r);
    assert_eq!(out_c.result, b"c");
}

#[test]
fn only_expected_extra_symbols_in_rust() {
    // Informational, but assert the Rust object does not accidentally export a
    // second, differently-named copy of the API (e.g. a mangled leftover).
    let rust = defined_symbols(&rust_so());
    let suspicious: Vec<_> = rust
        .iter()
        .filter(|s| s.contains("basename") && s.as_str() != "tool_basename")
        .collect();
    assert!(
        suspicious.is_empty(),
        "Rust .so exports unexpected basename-like symbols: {suspicious:?}"
    );
}
