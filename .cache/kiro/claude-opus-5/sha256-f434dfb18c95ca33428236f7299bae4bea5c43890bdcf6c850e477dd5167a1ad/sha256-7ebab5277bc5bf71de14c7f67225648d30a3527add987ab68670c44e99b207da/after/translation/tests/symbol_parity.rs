//! Verifies that the Rust `cdylib` exports every dynamic symbol the C shared
//! library exports, under the same names.

mod common;

use std::collections::BTreeSet;
use std::process::Command;

/// Defined dynamic symbols of a shared object, as reported by `nm -D`.
fn exported_symbols(path: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(path)
        .output()
        .expect("failed to run `nm`");
    assert!(
        out.status.success(),
        "nm -D failed on {}: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );

    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|line| {
            // "<addr> <type> <name>" or "         <type> <name>" for undefined.
            let mut it = line.split_whitespace();
            let first = it.next()?;
            let (ty, name) = if first.len() == 1 {
                (first, it.next()?)
            } else {
                (it.next()?, it.next()?)
            };
            // Only code/data definitions, matching what a caller could bind to.
            match ty {
                "T" | "t" | "D" | "d" | "B" | "b" | "R" | "r" | "W" | "w" | "V" | "v" | "i"
                | "G" | "g" | "S" | "s" => Some(name.to_string()),
                _ => None,
            }
        })
        .collect()
}

#[test]
fn rust_so_exports_every_c_symbol() {
    let c_syms = exported_symbols(&common::c_so_path());
    let rust_syms = exported_symbols(&common::rust_so_path());

    assert!(
        !c_syms.is_empty(),
        "expected the C library to export at least one symbol"
    );

    let missing: Vec<&String> = c_syms.difference(&rust_syms).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing symbols exported by the C .so: {:?}\n\
         C exports: {:?}",
        missing,
        c_syms
    );
}

#[test]
fn driver_is_among_the_exports() {
    let c_syms = exported_symbols(&common::c_so_path());
    let rust_syms = exported_symbols(&common::rust_so_path());
    assert!(c_syms.contains("driver"), "C exports: {:?}", c_syms);
    assert!(rust_syms.contains("driver"));
}

#[test]
fn print_hex_is_not_exported_by_either_library() {
    // `print_hex` is `static` in the C source, so it must not appear in either
    // library's dynamic symbol table.
    for path in [common::c_so_path(), common::rust_so_path()] {
        let syms = exported_symbols(&path);
        assert!(
            !syms.iter().any(|s| s == "print_hex"),
            "{} unexpectedly exports print_hex",
            path.display()
        );
    }
}
