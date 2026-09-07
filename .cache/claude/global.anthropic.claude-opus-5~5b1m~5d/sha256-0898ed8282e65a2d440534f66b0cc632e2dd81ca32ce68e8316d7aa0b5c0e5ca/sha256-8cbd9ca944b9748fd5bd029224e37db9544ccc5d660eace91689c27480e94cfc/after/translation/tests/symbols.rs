//! Phase D — symbol parity between the C `.so` and the Rust `.so`.
//!
//! Asserts mechanically (via `nm -D`) that the Rust shared library exports
//! *every* symbol the C shared library exports, with the exact same name.

mod common;

use common::{c_so_path, rust_so_path};
use std::collections::BTreeSet;
use std::process::Command;

/// Symbols the library *defines* and exports dynamically.
fn exported_symbols(so: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(so)
        .output()
        .expect("failed to run `nm` (binutils required)");
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
            let (a, b) = (it.next()?, it.next()?);
            // "<addr> <type> <name>" or "         <type> <name>"
            let (kind, name) = match it.next() {
                Some(n) => (b, n),
                None => (a, b),
            };
            // Keep only strong, globally-visible code/data symbols. Rust's
            // cdylib additionally emits weak (w/W/V/v) unwinder and allocator
            // shims that are not part of the C surface.
            match kind {
                "T" | "D" | "B" | "R" | "G" | "S" => Some(name.to_string()),
                _ => None,
            }
        })
        // Filter the toolchain-injected boilerplate that both libraries get.
        .filter(|n| !n.starts_with("_init") && !n.starts_with("_fini"))
        .filter(|n| !n.starts_with("__bss_start") && !n.starts_with("_edata"))
        .filter(|n| *n != "_end")
        .collect()
}

#[test]
fn symbol_parity_c_vs_rust() {
    let c_path = c_so_path();
    let rust_path = rust_so_path();
    let c = exported_symbols(&c_path);
    let rust = exported_symbols(&rust_path);

    // The C library's surface must be non-trivial, or the test proves nothing.
    assert!(
        c.contains("driver") && c.contains("print_foo"),
        "the C .so at {} does not export the expected symbols: {c:?}",
        c_path.display()
    );

    let missing: Vec<&String> = c.difference(&rust).collect();
    assert!(
        missing.is_empty(),
        "the Rust .so ({}) is MISSING {} symbol(s) exported by the C .so ({}): {:?}\n\
         C exports:    {:?}\n\
         Rust exports: {:?}",
        rust_path.display(),
        missing.len(),
        c_path.display(),
        missing,
        c,
        rust
    );
}

/// Every symbol the Rust `.so` imports must actually resolve against the system
/// libraries it links — i.e. no undefined non-libc symbols left behind by a
/// partially translated module.  `ldd -r` performs exactly this check (data and
/// function relocations) and lists anything that cannot be bound.
#[test]
fn rust_so_has_no_unresolved_non_libc_symbols() {
    for so in [c_so_path(), rust_so_path()] {
        let out = Command::new("ldd")
            .arg("-r")
            .arg(&so)
            .output()
            .expect("failed to run `ldd -r`");
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        let bad: Vec<&str> = text
            .lines()
            .filter(|l| {
                let l = l.trim();
                l.starts_with("undefined symbol:") || l.contains("not found")
            })
            .collect();
        assert!(
            bad.is_empty(),
            "{} has unresolved symbols, which would mean part of the C library \
             was never translated:\n{}",
            so.display(),
            bad.join("\n")
        );
    }
}

/// Both `.so`s must actually load and expose the two entry points through
/// `dlopen`/`dlsym` — the check an external consumer really performs.
#[test]
fn both_libraries_dlopen_and_expose_entry_points() {
    // `common::pair()` panics with a descriptive message if either `dlopen` or
    // either `dlsym` fails.
    let p = common::pair();
    assert_eq!(p.c.name, "C");
    assert_eq!(p.rust.name, "Rust");
}
