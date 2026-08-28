//! Step 8: every dynamic symbol the C .so exports must also be exported by the
//! Rust .so under the exact same name.

mod common;
use common::*;
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

/// Global text/data symbols defined by a shared object, per `nm -D`.
fn exported_symbols(so: &Path) -> BTreeSet<String> {
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
    let text = String::from_utf8_lossy(&out.stdout);
    let mut set = BTreeSet::new();
    for line in text.lines() {
        let mut parts = line.split_whitespace();
        let (kind, name) = match (parts.next(), parts.next(), parts.next()) {
            // "<addr> <kind> <name>"
            (Some(_addr), Some(k), Some(n)) => (k, n),
            // "        <kind> <name>" (undefined-address form)
            (Some(k), Some(n), None) => (k, n),
            _ => continue,
        };
        // Only strong global definitions: T (text), D/B (data/bss), R (rodata).
        if matches!(kind, "T" | "D" | "B" | "R") {
            set.insert(name.to_string());
        }
    }
    set
}

#[test]
fn rust_exports_every_c_symbol() {
    let _ = libs(); // ensure both artifacts exist and are current
    let c_so = c_so_path();
    let rust_so = rust_so_path();

    let c_syms = exported_symbols(&c_so);
    let rust_syms = exported_symbols(&rust_so);

    assert!(
        !c_syms.is_empty(),
        "no symbols parsed from {}",
        c_so.display()
    );

    // Symbols the C library gets from its own compilation unit only; toolchain
    // bookkeeping symbols present in every ELF object are not part of the API.
    let ignorable = |s: &str| {
        s.starts_with("_init")
            || s.starts_with("_fini")
            || s.starts_with("__bss_start")
            || s == "_edata"
            || s == "_end"
    };

    let missing: Vec<&String> = c_syms
        .iter()
        .filter(|s| !ignorable(s) && !rust_syms.contains(*s))
        .collect();

    assert!(
        missing.is_empty(),
        "the Rust .so is missing {} symbol(s) exported by the C .so: {missing:?}",
        missing.len()
    );

    // The nine functions of the C translation unit must all be present.
    for expected in [
        "arity",
        "arity2",
        "arity3",
        "arity4",
        "apply_bitmask",
        "compare_allocations",
        "init_matrix",
        "process_string",
        "shift_array",
    ] {
        assert!(
            c_syms.contains(expected),
            "sanity check: C .so should export {expected}"
        );
        assert!(
            rust_syms.contains(expected),
            "Rust .so does not export {expected}"
        );
    }
}
