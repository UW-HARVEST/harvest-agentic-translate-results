//! Phase D — symbol parity between the C `.so` and the Rust `.so`.
//!
//! Re-runs the `nm -D` diff on every test run so `SYMBOLS.md` cannot silently
//! go stale.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::process::Command;

/// Symbols the C `.so` defines (`nm -D --defined-only`).
fn defined_symbols(so: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(so)
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
        .filter_map(|l| l.split_whitespace().last())
        .map(|s| s.to_string())
        .collect()
}

fn undefined_symbols(so: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--undefined-only"])
        .arg(so)
        .output()
        .expect("run nm");
    assert!(out.status.success());
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn symbol_parity_c_so_vs_rust_so() {
    let c = defined_symbols(c_so_path());
    let r = defined_symbols(rust_so_path());

    assert!(!c.is_empty(), "the C .so defines no symbols at all?");

    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}\n\
         C   ({}) = {c:?}\n\
         Rust({}) = {r:?}",
        c.len(),
        r.len()
    );

    // The C library exports exactly `driver`; nail that down so a future
    // regression that drops it is caught even if both sides drop it.
    assert!(c.contains("driver"), "C .so must export `driver`");
    assert!(r.contains("driver"), "Rust .so must export `driver`");
}

#[test]
fn rust_so_has_no_unresolved_non_libc_symbols() {
    // Rather than maintain a hand-written libc allowlist (which is brittle:
    // an optimised build legitimately imports `putchar` instead of `puts`
    // because LLVM rewrites `puts("")` -> `putchar('\n')`), ask the dynamic
    // linker itself. `ldd -r` performs both data and function relocation
    // resolution and prints an "undefined symbol: ..." line for anything that
    // cannot be satisfied by the library's recorded dependencies.
    for so in [c_so_path(), rust_so_path()] {
        let out = Command::new("ldd").arg("-r").arg(so).output().expect("run ldd -r");
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        let unresolved: Vec<&str> = text
            .lines()
            .filter(|l| l.contains("undefined symbol"))
            .collect();
        assert!(
            unresolved.is_empty(),
            "{} has unresolved (non-libc) symbols:\n{}",
            so.display(),
            unresolved.join("\n")
        );
    }

    // Sanity: the import list is non-empty and still routes stdout through libc
    // (either `puts` or the `putchar` form LLVM may substitute for it), which is
    // what makes the two libraries share the process's `stdout` FILE.
    let imports = undefined_symbols(rust_so_path());
    let bare: BTreeSet<String> = imports
        .iter()
        .map(|s| s.split('@').next().unwrap_or(s).to_string())
        .collect();
    assert!(
        bare.contains("printf") || bare.contains("puts") || bare.contains("putchar"),
        "Rust .so no longer writes through libc stdio; imports = {bare:?}"
    );
}

#[test]
fn both_libraries_expose_a_callable_driver_through_dlsym() {
    // Loading each .so and resolving `driver` via dlsym is exactly what an
    // external consumer does; this is what makes the `#[no_mangle]` wrapper
    // part of the test surface.
    let c = c_lib();
    let r = rust_lib();
    let out_c = capture_stdout("c", || unsafe { (c.driver)(7, 3) });
    let out_r = capture_stdout("rust", || unsafe { (r.driver)(7, 3) });
    assert_eq!(out_c, out_r);
    // 7 | ~3 = 7 | -4 = -1
    assert_eq!(out_c, b"-1\n");
}

#[test]
fn feature_configuration_surface_is_exhaustive() {
    // `Cargo.toml` declares no [features] table, so there is exactly one
    // build configuration. If that ever changes, this test fails and forces
    // CONFIGS.md / check_features.sh to be revisited.
    let toml = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"))
        .expect("read Cargo.toml");
    assert!(
        !toml.contains("[features]"),
        "Cargo.toml gained a [features] table: re-enumerate the feature \
         combinations in CONFIGS.md and re-run ./check_features.sh"
    );
    assert!(enabled_features().is_empty());
}
