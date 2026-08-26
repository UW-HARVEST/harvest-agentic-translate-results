//! Phase D — symbol parity between the C `.so` and the Rust `.so`.
//!
//! Re-derives `SYMBOLS.md` mechanically: every symbol *defined* in the C shared
//! object's dynamic symbol table must also be defined by the Rust shared object
//! under the exact same name.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

/// `nm -D --defined-only <so>` -> set of symbol names.
fn defined_symbols(so: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(so)
        .output()
        .expect("failed to run nm");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(str::to_string))
        .collect()
}

/// `nm -D <so>` -> set of undefined (`U`) symbol names.
fn undefined_symbols(so: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg(so)
        .output()
        .expect("failed to run nm");
    assert!(out.status.success());
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|l| {
            let mut it = l.split_whitespace();
            matches!((it.next(), it.next()), (Some("U"), Some(_)))
        })
        .filter_map(|l| l.split_whitespace().nth(1).map(str::to_string))
        .collect()
}

#[test]
fn every_c_symbol_is_exported_by_rust() {
    let c = defined_symbols(&c_so_path());
    let r = defined_symbols(&rust_so_path());

    assert!(
        c.contains("bin2hex"),
        "sanity: C .so must define bin2hex, got {c:?}"
    );

    let missing: Vec<_> = c.difference(&r).cloned().collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}\n\
         C   ({}) = {c:?}\nRust({}) = <{} symbols>",
        c.len(),
        r.len(),
        r.len()
    );
}

/// The Rust `.so` must resolve standalone: the only unresolved symbols may come
/// from libc / the dynamic loader, never from the C library under test.
#[test]
fn rust_so_has_no_unexpected_undefined_symbols() {
    let u = undefined_symbols(&rust_so_path());
    let c_defined = defined_symbols(&c_so_path());
    let borrowed: Vec<_> = u.intersection(&c_defined).cloned().collect();
    assert!(
        borrowed.is_empty(),
        "Rust .so imports symbols from the C library instead of defining them: {borrowed:?}"
    );
    // dlopen with full relocation would fail if anything were unresolvable.
    let _ = impls();
}

#[test]
fn bin2hex_is_dlsym_able_from_both() {
    let f = impls();
    assert!(!(f.c as usize == 0));
    assert!(!(f.r as usize == 0));
    assert_ne!(
        f.c as usize, f.r as usize,
        "the same function was loaded twice — the two .so paths must differ"
    );
}

/// Print the two symbol tables so `SYMBOLS.md` can be checked by eye if needed.
#[test]
fn dump_symbol_tables() {
    let c = defined_symbols(&c_so_path());
    let r = defined_symbols(&rust_so_path());
    println!("C   .so ({}) defined: {c:?}", c.len());
    println!("Rust .so defines {} symbols; bin2hex present: {}", r.len(), r.contains("bin2hex"));
}
