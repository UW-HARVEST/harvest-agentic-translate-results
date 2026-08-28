//! Verifies that the Rust shared object exports at least every dynamic symbol
//! the C shared object exports, under the same names.

mod common;

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

/// Dynamic symbols *defined* (not merely referenced) by a shared object.
fn defined_dynamic_symbols(lib: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(lib)
        .output()
        .expect("`nm` must be available to compare exported symbols");
    assert!(
        out.status.success(),
        "nm -D failed for {lib:?}:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );

    let mut symbols = BTreeSet::new();
    for line in String::from_utf8_lossy(&out.stdout).lines() {
        // Format: "<addr> <type> <name>"; addr may be blank for undefined.
        let mut fields = line.split_whitespace();
        let (Some(first), Some(second)) = (fields.next(), fields.next()) else {
            continue;
        };
        let (kind, name) = match fields.next() {
            Some(name) => (second, name),
            // Two-column form: "<type> <name>".
            None => (first, second),
        };
        // Keep global/weak text & data definitions; skip local ones.
        if matches!(kind, "T" | "t" | "W" | "w" | "D" | "d" | "B" | "b" | "R" | "r" | "i" | "I") {
            symbols.insert(name.to_string());
        }
    }
    symbols
}

#[test]
fn rust_exports_superset_of_c_exports() {
    let c_lib = common::c_library_path();
    let rust_lib = common::rust_library_path();

    let c_syms = defined_dynamic_symbols(&c_lib);
    let rust_syms = defined_dynamic_symbols(&rust_lib);

    assert!(
        c_syms.contains("tool_basename"),
        "sanity check: C library should export `tool_basename`, got {c_syms:?}"
    );

    let missing: Vec<&String> = c_syms.difference(&rust_syms).collect();
    assert!(
        missing.is_empty(),
        "Rust library is missing symbols exported by the C library: {missing:?}\n\
         C exports: {c_syms:?}\nRust exports: {rust_syms:?}"
    );
}
