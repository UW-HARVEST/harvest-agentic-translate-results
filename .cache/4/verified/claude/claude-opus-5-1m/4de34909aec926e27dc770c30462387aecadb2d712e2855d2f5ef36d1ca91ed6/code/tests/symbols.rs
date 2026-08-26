//! Phase D -- dynamic-symbol parity between the C and the Rust shared object.

mod common;

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

use common::{c_so_path, rust_so_path};

/// `nm -D --defined-only <so>` -> set of exported symbol names.
fn defined_symbols(so: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(so)
        .output()
        .expect("run nm");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|line| {
            let mut it = line.split_whitespace();
            let (a, b, c) = (it.next(), it.next(), it.next());
            match (a, b, c) {
                // "<addr> <type> <name>"
                (Some(_), Some(_), Some(name)) => Some(name.to_string()),
                // "<type> <name>" (weak/undefined-style rows)
                (Some(_), Some(name), None) => Some(name.to_string()),
                _ => None,
            }
        })
        .map(|s| s.split('@').next().unwrap().to_string())
        .collect()
}

#[test]
fn c_defined_symbols_are_all_exported_by_rust() {
    let c = defined_symbols(&c_so_path());
    let rust = defined_symbols(&rust_so_path());

    // The C translation unit exports exactly these two symbols.
    assert!(
        c.contains("driver") && c.contains("main"),
        "unexpected C symbol set: {c:?}"
    );

    let missing: Vec<&String> = c.difference(&rust).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but missing from the Rust .so: {missing:?}\n\
         C:    {c:?}\nRust: {rust:?}"
    );
}

#[test]
fn rust_so_exports_the_expected_surface() {
    let rust = defined_symbols(&rust_so_path());
    assert!(rust.contains("driver"), "Rust .so must export `driver`");
    assert!(rust.contains("main"), "Rust .so must export `main`");
}

#[test]
fn rust_so_has_no_unresolved_non_libc_symbols() {
    // `ldd -r` performs data and function relocation checks and prints
    // "undefined symbol: ..." for anything that cannot be resolved.
    for so in [c_so_path(), rust_so_path()] {
        let out = Command::new("ldd").arg("-r").arg(&so).output().expect("ldd");
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        let bad: Vec<&str> = text
            .lines()
            .filter(|l| l.contains("undefined symbol"))
            .collect();
        assert!(
            bad.is_empty(),
            "{} has unresolved symbols: {bad:?}",
            so.display()
        );
    }
}

#[test]
fn exported_symbols_are_callable_through_dlsym() {
    // Loading both objects resolves `driver` and `main` in each of them; this
    // is what every other test relies on.
    let l = common::libs();
    assert_eq!(l.c.name, "C");
    assert_eq!(l.rust.name, "Rust");
}
