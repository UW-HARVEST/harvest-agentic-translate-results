//! Phase D — symbol parity between the C `.so` and the Rust `.so`.
//!
//! Asserts, at test time, that the exported-symbol diff is EMPTY: every symbol
//! the C `.so` exports is also exported by the Rust `.so` under the exact same
//! name.

mod common;
use common::{c_so_path, libs, rust_so_path};
use std::collections::BTreeSet;
use std::process::Command;

/// Exported (defined, global) dynamic symbols of a shared object, via `nm -D`.
fn exported_symbols(so: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", "--format=posix"])
        .arg(so)
        .output()
        .expect("failed to run `nm` (binutils required for the symbol-parity test)");
    assert!(
        out.status.success(),
        "nm failed on {so:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|line| {
            let mut it = line.split_whitespace();
            let name = it.next()?;
            let kind = it.next()?;
            // Keep global code/data symbols only: T (text), D/B (data/bss),
            // R (rodata). Skip weak/unique/local classes.
            match kind {
                "T" | "D" | "B" | "R" | "G" | "S" => Some(name.to_string()),
                _ => None,
            }
        })
        // Filter out toolchain/runtime-provided names that are not part of the
        // library's own API surface.
        .filter(|n| {
            !n.starts_with("_ITM_")
                && !n.starts_with("__")
                && n != "_init"
                && n != "_fini"
                && n != "_edata"
                && n != "_end"
                && n != "_IO_stdin_used"
                && !n.starts_with("rust_")
                && !n.starts_with("_ZN")
                && !n.starts_with("_R")
        })
        .collect()
}

#[test]
fn c_and_rust_export_the_same_symbols() {
    let c = exported_symbols(&c_so_path());
    let r = exported_symbols(&rust_so_path());

    println!("C   exports ({}): {:?}", c.len(), c);
    println!("RUST exports ({}): {:?}", r.len(), r);

    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}"
    );

    // The C library's whole documented API is `driver`; make sure the parity
    // check is not trivially passing on an empty set.
    assert!(c.contains("driver"), "C .so must export `driver`; got {c:?}");
    assert!(r.contains("driver"), "Rust .so must export `driver`; got {r:?}");
    assert_eq!(c.len(), 1, "unexpected extra C exports: {c:?}");
}

#[test]
fn rust_so_has_no_unresolved_project_symbols() {
    let out = Command::new("nm")
        .args(["-D", "--undefined-only", "--format=posix"])
        .arg(rust_so_path())
        .output()
        .expect("nm");
    let text = String::from_utf8_lossy(&out.stdout);
    let suspicious: Vec<&str> = text
        .lines()
        .map(|l| l.split_whitespace().next().unwrap_or(""))
        .filter(|n| !n.is_empty())
        // Everything the Rust .so imports must come from libc / the Rust
        // runtime; nothing may reference an untranslated project symbol.
        .filter(|n| *n == "print_hex" || *n == "driver")
        .collect();
    assert!(
        suspicious.is_empty(),
        "Rust .so has unresolved project symbols (untranslated C?): {suspicious:?}"
    );
}

#[test]
fn both_libraries_resolve_driver_at_runtime() {
    // Prove the exports are actually callable through dlsym in both libraries.
    let l = libs();
    println!("C   .so: {:?}", l.c_path);
    println!("RUST .so: {:?}", l.rust_path);
    let c = common::capture_stdout(|| unsafe { (l.c_driver)(3.5f32) });
    let r = common::capture_stdout(|| unsafe { (l.rust_driver)(3.5f32) });
    assert_eq!(c, r);
    assert_eq!(c, b"00006040\n", "got {:?}", String::from_utf8_lossy(&c));
}

/// The crate declares no `[features]`, so the default (empty) feature set is the
/// only combination. Guard that invariant so a future feature addition forces
/// the Phase B/C matrix to be re-run under every combination.
#[test]
fn crate_has_no_feature_axes() {
    let toml = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"),
    )
    .expect("read Cargo.toml");
    let has_features = toml
        .lines()
        .any(|l| l.trim() == "[features]" || l.trim().starts_with("[features."));
    assert!(
        !has_features,
        "Cargo.toml now declares [features]; CONFIGS.md row 25 must be re-verified \
         for every feature combination with `cargo test --no-default-features --features <combo>`"
    );
}
