//! Phase D — symbol parity enforced as a test, so it cannot silently rot.
//!
//! Every symbol the C `.so` exports must be exported by the Rust `.so` under
//! the exact same name, and must actually resolve via `dlsym`.

mod common;

use common::*;

use std::collections::BTreeSet;
use std::process::Command;

fn defined_dynamic_symbols(path: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(path)
        .output()
        .expect("nm must be available");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let _addr = it.next()?;
            let kind = it.next()?;
            let name = it.next()?;
            // Only global text/data symbols; skip Rust/compiler internals.
            if !matches!(kind, "T" | "D" | "B" | "R" | "W" | "V") {
                return None;
            }
            if name.starts_with("_ITM_")
                || name.starts_with("__")
                || name == "_init"
                || name == "_fini"
                || name.starts_with("_Z")
                || name.starts_with("rust_")
            {
                return None;
            }
            Some(name.to_string())
        })
        .collect()
}

#[test]
fn every_c_symbol_is_exported_by_rust() {
    let c = defined_dynamic_symbols(&c_so_path());
    let rs = defined_dynamic_symbols(&rust_so_path());

    assert!(
        !c.is_empty(),
        "nm found no exported symbols in the C .so — build it first"
    );

    let missing: Vec<&String> = c.difference(&rs).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}\n\
         C   = {c:?}\n\
         Rust = {rs:?}"
    );

    // The C library's full public surface, spelled out, so a future C change
    // that adds a symbol makes this test fail loudly rather than pass vacuously.
    let expected: BTreeSet<String> = ["w_utf8_drop", "w_utf8_filter"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    assert_eq!(
        c, expected,
        "the C .so's exported surface changed; update SYMBOLS.md and the tests"
    );
}

#[test]
fn every_c_symbol_resolves_via_dlsym_in_rust() {
    // load_pair() itself dlsym's both names in both libraries and panics if a
    // symbol is absent, so a successful load is the assertion.
    let pair = load_pair();
    assert_eq!(pair.c.name, "C");
    assert_eq!(pair.rs.name, "Rust");
}

#[test]
fn rust_so_has_no_undefined_non_libc_symbols() {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--undefined-only")
        .arg(rust_so_path())
        .output()
        .expect("nm must be available");
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    let bad: Vec<&str> = text
        .lines()
        .filter_map(|l| l.split_whitespace().last())
        .filter(|name| {
            // Everything left over must be satisfied by libc / libgcc / libm.
            !(name.contains("@GLIBC")
                || name.contains("@GCC")
                || name.starts_with("_ITM_")
                || name.starts_with("__")
                || name.starts_with("_Unwind_")
                || *name == "gettid"
                || *name == "statx")
        })
        .collect();
    assert!(
        bad.is_empty(),
        "Rust .so has undefined non-libc symbols: {bad:?}"
    );
}

#[test]
fn project_builds_no_driver_binary_to_compare() {
    // c_src/CMakeLists.txt only has `add_library(driver SHARED src/lib.c)` and
    // the crate only declares `crate-type = ["cdylib"]`, so there is no
    // executable whose stdout could be diffed. Assert that stays true; if a
    // binary appears, this test fails and the stdout comparison must be added.
    let root = c_so_path()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    let cml = std::fs::read_to_string(root.join("c_src/CMakeLists.txt")).expect("read CMakeLists");
    assert!(
        !cml.contains("add_executable"),
        "c_src now builds an executable; add a stdout differential test"
    );
    let cargo = std::fs::read_to_string(root.join("translation/Cargo.toml")).expect("read Cargo");
    assert!(
        !cargo.contains("[[bin]]"),
        "the crate now builds a binary; add a stdout differential test"
    );
    assert!(
        !root.join("translation/src/main.rs").exists(),
        "src/main.rs appeared; add a stdout differential test"
    );
}
