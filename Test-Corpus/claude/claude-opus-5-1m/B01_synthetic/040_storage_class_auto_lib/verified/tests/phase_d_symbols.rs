// Phase D — symbol parity between the C `.so` and the Rust `.so`.
//
// Mechanically re-derives the SYMBOLS.md table with `nm -D` so the parity claim
// is enforced by the test suite, not just documented.

mod common;

use common::{c_so_path, impls, rust_so_path};
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

/// Toolchain-injected weak symbols that are not part of any library's API.
const TOOLCHAIN_WEAK: &[&str] = &[
    "_ITM_registerTMCloneTable",
    "_ITM_deregisterTMCloneTable",
    "__cxa_finalize",
    "__cxa_thread_atexit_impl",
    "__gmon_start__",
    "gettid",
    "statx",
];

fn nm_lines(so: &Path, extra_arg: &str) -> Vec<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg(extra_arg)
        .arg(so)
        .output()
        .unwrap_or_else(|e| panic!("running nm on {so:?}: {e}"));
    assert!(
        out.status.success(),
        "nm failed on {so:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|l| l.to_string())
        .collect()
}

/// Set of globally *defined* dynamic symbol names (strong definitions only).
fn defined_symbols(so: &Path) -> BTreeSet<String> {
    let mut set = BTreeSet::new();
    for line in nm_lines(so, "--defined-only") {
        let mut it = line.split_whitespace();
        let (kind, name) = match (it.next(), it.next(), it.next()) {
            // "<addr> <kind> <name>"
            (Some(_addr), Some(k), Some(n)) => (k.to_string(), n.to_string()),
            // "<kind> <name>" (no address, e.g. weak undefined)
            (Some(k), Some(n), None) => (k.to_string(), n.to_string()),
            _ => continue,
        };
        let base = name.split('@').next().unwrap().to_string();
        if TOOLCHAIN_WEAK.contains(&base.as_str()) {
            continue;
        }
        // Skip weak (w/V) toolchain placeholders; keep real definitions.
        if kind == "w" || kind == "v" {
            continue;
        }
        set.insert(base);
    }
    set
}

#[test]
fn sym_01_every_c_symbol_is_exported_by_rust() {
    let c = defined_symbols(&c_so_path());
    let r = defined_symbols(&rust_so_path());

    assert!(
        c.contains("driver"),
        "sanity: the C .so must export `driver`, got {c:?}"
    );

    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}\n\
         C   = {c:?}\n\
         Rust= {r:?}"
    );
}

#[test]
fn sym_02_rust_has_no_unresolvable_undefined_symbols() {
    // Every undefined symbol of the Rust .so must come from libc/libgcc, i.e.
    // it must be resolvable at load time.  `impls()` dlopen()s the library with
    // RTLD_NOW, which fails if anything is unresolvable, so a successful load
    // is the strongest possible check.  Additionally assert that no undefined
    // symbol is a *project* symbol (unversioned, non-libc, non-toolchain).
    let _ = impls();

    let unresolved: Vec<String> = nm_lines(&rust_so_path(), "--undefined-only")
        .iter()
        .filter_map(|line| {
            let name = line.split_whitespace().last()?.to_string();
            let base = name.split('@').next().unwrap().to_string();
            if name.contains('@') || TOOLCHAIN_WEAK.contains(&base.as_str()) {
                None // versioned => resolved from libc/libgcc; or toolchain weak
            } else {
                Some(base)
            }
        })
        .collect();
    assert!(
        unresolved.is_empty(),
        "Rust .so has undefined non-libc symbols: {unresolved:?}"
    );
}

#[test]
fn sym_03_both_libraries_expose_a_distinct_callable_driver() {
    let im = impls();
    assert_ne!(im.c.addr, 0);
    assert_ne!(im.rust.addr, 0);
    assert_ne!(
        im.c.addr, im.rust.addr,
        "the two .so files aliased to one object"
    );
}

#[test]
fn sym_04_no_extra_public_c_declarations_were_missed() {
    // The only public header must declare exactly the symbols we compare.
    let header = std::fs::read_to_string(common::manifest_dir().join("c_src/include/driver.h"))
        .expect("read driver.h");
    let decls: Vec<&str> = header
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with("//") && !l.starts_with('#'))
        .collect();
    assert_eq!(
        decls,
        vec!["void driver(int x);"],
        "the public header changed: re-derive SYMBOLS.md / CONFIGS.md / ERRORS.md"
    );
}
