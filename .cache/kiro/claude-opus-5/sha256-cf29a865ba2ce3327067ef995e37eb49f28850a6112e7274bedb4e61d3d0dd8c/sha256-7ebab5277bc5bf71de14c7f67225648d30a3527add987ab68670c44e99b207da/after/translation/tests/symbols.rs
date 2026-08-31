//! Exported-symbol parity: every dynamic symbol the C shared object defines
//! must also be defined by the Rust shared object, under the exact same name.

mod common;

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

use common::{c_library_path, rust_library_path};

/// Names of the dynamic symbols *defined* by `lib`.
fn exported_symbols(lib: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(lib)
        .output()
        .expect("run nm");
    assert!(
        out.status.success(),
        "nm -D --defined-only {lib:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|line| {
            // "<value> <type> <name>" or "         U <name>"
            let mut parts = line.split_whitespace();
            let a = parts.next()?;
            let b = parts.next()?;
            match parts.next() {
                Some(name) => {
                    let _ = (a, b);
                    Some(name.to_string())
                }
                None => Some(b.to_string()),
            }
        })
        .collect()
}

#[test]
fn rust_exports_every_c_symbol() {
    let c = c_library_path();
    let r = rust_library_path();
    let c_syms = exported_symbols(&c);
    let r_syms = exported_symbols(&r);

    assert!(
        c_syms.contains("driver"),
        "the C library should export `driver`; got {c_syms:?}"
    );

    let missing: Vec<&String> = c_syms.difference(&r_syms).collect();
    assert!(
        missing.is_empty(),
        "the Rust library is missing {} symbol(s) exported by the C library: {missing:?}\n\
         C exports: {c_syms:?}\nRust exports: {r_syms:?}",
        missing.len()
    );
}

/// `driver` must be a global function symbol (`T`) in both objects, not a weak
/// or data symbol, so external callers bind to it identically.
#[test]
fn driver_has_the_same_symbol_kind() {
    for lib in [c_library_path(), rust_library_path()] {
        let out = Command::new("nm")
            .arg("-D")
            .arg("--defined-only")
            .arg(&lib)
            .output()
            .expect("run nm");
        let text = String::from_utf8_lossy(&out.stdout);
        let kind = text
            .lines()
            .find_map(|l| {
                let mut p = l.split_whitespace();
                let _addr = p.next()?;
                let kind = p.next()?;
                let name = p.next()?;
                (name == "driver").then(|| kind.to_string())
            })
            .unwrap_or_else(|| panic!("no `driver` symbol in {lib:?}"));
        assert_eq!(kind, "T", "`driver` in {lib:?} has symbol type {kind}");
    }
}
