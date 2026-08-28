//! Export-table parity: every dynamic symbol the C `.so` defines must also be
//! defined by the Rust `.so` under the exact same name.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

fn defined_dynamic_symbols(so: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", so.to_str().unwrap()])
        .output()
        .expect("failed to run nm");
    assert!(
        out.status.success(),
        "nm -D {} failed: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8_lossy(&out.stdout);
    let mut set = BTreeSet::new();
    for line in text.lines() {
        let cols: Vec<&str> = line.split_whitespace().collect();
        if cols.len() < 3 {
            continue;
        }
        let kind = cols[cols.len() - 2];
        let name = cols[cols.len() - 1];
        // ignore linker-provided / toolchain internals
        if name.starts_with("_init")
            || name.starts_with("_fini")
            || name.starts_with("__")
            || name == "_edata"
            || name == "_end"
            || name == "__bss_start"
        {
            continue;
        }
        // T/t: text, D/d/B/b: data, W: weak
        if !matches!(kind, "T" | "t" | "D" | "d" | "B" | "b" | "W" | "V" | "R" | "r") {
            continue;
        }
        set.insert(name.to_string());
    }
    set
}

#[test]
fn rust_so_exports_every_c_symbol() {
    let c = defined_dynamic_symbols(&c_so_path());
    let r = defined_dynamic_symbols(&rust_so_path());

    assert!(
        !c.is_empty(),
        "no symbols parsed from {}",
        c_so_path().display()
    );

    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing symbols exported by the C .so: {missing:?}\n\
         C: {c:?}\nRust: {r:?}"
    );
}

#[test]
fn all_public_api_symbols_are_loadable() {
    // Resolving every symbol through dlsym in both libraries.
    let _ = load_pair();
}
