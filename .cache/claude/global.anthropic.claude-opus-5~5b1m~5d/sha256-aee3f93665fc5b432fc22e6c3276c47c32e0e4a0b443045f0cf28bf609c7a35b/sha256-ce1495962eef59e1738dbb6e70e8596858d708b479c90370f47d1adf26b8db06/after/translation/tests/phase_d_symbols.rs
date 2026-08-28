//! Phase D — symbol-parity gate: every dynamic symbol exported by the C `.so`
//! must be exported by the Rust `.so` under the exact same name.

mod common;
use common::*;

use std::collections::BTreeSet;
use std::process::Command;

fn exported_symbols(so: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", "--format=posix"])
        .arg(so)
        .output()
        .expect("failed to run nm");
    assert!(out.status.success(), "nm failed on {}", so.display());
    let text = String::from_utf8_lossy(&out.stdout);
    let mut set = BTreeSet::new();
    for line in text.lines() {
        let mut it = line.split_whitespace();
        let name = match it.next() {
            Some(n) => n,
            None => continue,
        };
        let kind = it.next().unwrap_or("");
        // Only global text/data symbols; skip rust/compiler internals of the
        // form `_ZN...`, `__rust*`, `_*` and local (lowercase kind) symbols.
        if !matches!(kind, "T" | "D" | "B" | "R" | "W" | "V" | "i") {
            continue;
        }
        if name.starts_with('_') {
            continue;
        }
        set.insert(name.to_string());
    }
    set
}

#[test]
fn phase_d_symbol_parity() {
    let l = libs();
    let c = exported_symbols(&l.c.path);
    let r = exported_symbols(&l.rust.path);

    let missing: Vec<&String> = c.difference(&r).collect();
    println!("C symbols   ({}): {:?}", c.len(), c);
    println!("Rust symbols({}): {:?}", r.len(), r);
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}"
    );
    assert!(
        c.contains("flac_validate") && c.contains("tflac_size_memory"),
        "sanity: the C .so must export both known symbols, got {c:?}"
    );
    assert_eq!(c.len(), 2, "the C .so is expected to export exactly 2 symbols, got {c:?}");
}

#[test]
fn phase_d_no_undefined_non_libc_symbols() {
    let l = libs();
    let out = Command::new("nm")
        .args(["-D", "-u", "--format=posix"])
        .arg(&l.rust.path)
        .output()
        .expect("failed to run nm");
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    // An undefined symbol is acceptable only if it comes from the platform C
    // runtime / unwinder: either it carries a `@GLIBC_*` / `@GCC_*` version tag
    // or it is a reserved-namespace (`_`-prefixed) compiler/runtime import.
    let mut unexpected = Vec::new();
    for line in text.lines() {
        let mut it = line.split_whitespace();
        let name = match it.next() {
            Some(n) => n,
            None => continue,
        };
        let from_libc = name.contains("@GLIBC_")
            || name.contains("@GCC_")
            || name.contains("@GLIBC_PRIVATE")
            || name.starts_with('_');
        if !from_libc {
            unexpected.push(name.to_string());
        }
    }
    println!("undefined symbols in the Rust .so: {text}");
    assert!(
        unexpected.is_empty(),
        "unexpected undefined (non-libc) symbols in the Rust .so: {unexpected:?}"
    );
}

#[test]
fn phase_d_both_symbols_callable_through_dlsym() {
    // Loading already resolved both symbols in both libraries via dlsym; make a
    // trivial call through each so the test genuinely exercises the exports.
    let l = libs();
    assert_eq!(unsafe { (l.c.size_memory)(4096) }, unsafe { (l.rust.size_memory)(4096) });
    let mut a = Fields::default().to_raw();
    let mut b = Fields::default().to_raw();
    assert_eq!(unsafe { (l.c.validate)(a.0.as_mut_ptr()) }, unsafe {
        (l.rust.validate)(b.0.as_mut_ptr())
    });
    assert_eq!(a, b);
}
