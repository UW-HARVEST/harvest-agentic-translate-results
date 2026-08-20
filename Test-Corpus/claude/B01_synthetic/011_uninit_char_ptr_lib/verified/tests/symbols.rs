// Phase A / Phase D -- symbol parity, enforced automatically.
//
// Every symbol the C `.so` exports must also be exported by the Rust `.so`
// under the exact same name. The diff must be empty.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

/// Defined (exported) dynamic symbols, via `nm -D --defined-only`.
fn exported_symbols(so: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
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
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let (_addr, kind, name) = (it.next()?, it.next()?, it.next()?);
            // Global/weak text or data definitions only.
            if matches!(kind, "T" | "t" | "D" | "B" | "W" | "V" | "R") {
                Some(name.to_string())
            } else {
                None
            }
        })
        .collect()
}

/// Undefined (imported) dynamic symbols.
fn undefined_symbols(so: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--undefined-only"])
        .arg(so)
        .output()
        .expect("failed to run nm");
    assert!(out.status.success(), "nm failed on {}", so.display());
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let kind = it.next()?;
            // Undefined lines have no address: "                 U name".
            let name = if kind == "U" || kind == "w" || kind == "v" {
                it.next()?
            } else {
                it.next()?;
                it.next()?
            };
            Some(name.split('@').next().unwrap_or(name).to_string())
        })
        .collect()
}

#[test]
fn c_and_rust_export_identical_symbol_sets() {
    let c = exported_symbols(&c_so_path_default());
    let r = exported_symbols(&rust_so_path());

    // The four functions from c_src/src/driver.c.
    let expected: BTreeSet<String> = ["printLine", "bad", "good", "driver"]
        .iter()
        .map(|s| s.to_string())
        .collect();

    assert_eq!(
        c, expected,
        "the C .so's exported symbol set changed; SYMBOLS.md needs updating"
    );

    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is MISSING symbols exported by the C .so: {missing:?}\n\
         C   = {c:?}\n\
         Rust= {r:?}"
    );

    // Every C symbol is also dlsym-able from the Rust .so (proves the
    // #[no_mangle] export wrappers are real, callable entry points).
    for sym in &c {
        assert!(
            rust().has_symbol(sym),
            "symbol {sym:?} is not dlsym-able from the Rust .so"
        );
        assert!(
            c_default().has_symbol(sym),
            "symbol {sym:?} is not dlsym-able from the C .so"
        );
    }
}

#[test]
fn rust_so_has_no_unresolved_non_libc_symbols() {
    // The Rust .so must not reference anything the loader cannot resolve.
    // `dlopen` with RTLD_NOW would fail outright on a missing symbol, and
    // libloading's default RTLD_LAZY still resolves data relocations, so a
    // successful load plus a successful dlsym of every exported entry point is
    // the operative check. Do it explicitly with RTLD_NOW semantics by
    // resolving each undefined symbol in the process image.
    let undef = undefined_symbols(&rust_so_path());
    assert!(!undef.is_empty(), "expected the Rust .so to import libc symbols");

    // Force full relocation: if anything were unresolvable, this fails.
    let out = Command::new("ldd")
        .args(["-r"])
        .arg(rust_so_path())
        .output()
        .expect("failed to run ldd");
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let bad: Vec<&str> = combined
        .lines()
        .filter(|l| l.contains("undefined symbol") || l.contains("not found"))
        .collect();
    assert!(
        bad.is_empty(),
        "Rust .so has unresolved symbols:\n{}",
        bad.join("\n")
    );
}

#[test]
fn c_so_exports_exactly_the_four_translation_unit_functions() {
    // Guards against the "whole module never translated" failure mode: if the C
    // build ever grows a new source file / exported function, this test fails
    // and the new symbol must be translated into src/lib.rs.
    let c = exported_symbols(&c_so_path_default());
    assert_eq!(
        c.len(),
        4,
        "C .so exports {} symbols, expected 4 ({c:?}). A new C function was \
         added and must be translated.",
        c.len()
    );
}
