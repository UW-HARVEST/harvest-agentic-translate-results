//! Phase A / Phase D — symbol parity between the C and Rust shared objects,
//! plus a self-check that the differential harness really is comparing two
//! *different* binaries.

mod common;

use std::collections::BTreeSet;
use std::process::Command;

/// Every global symbol `nm -D --defined-only` reports for `path`.
fn defined_symbols(path: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(path)
        .output()
        .expect("nm must be available");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );

    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|line| {
            // "<addr> <type> <name>"
            let mut it = line.split_whitespace();
            let _addr = it.next()?;
            let kind = it.next()?;
            let name = it.next()?;
            // Only globally visible code/data, not the ELF weak plumbing.
            if matches!(kind, "T" | "D" | "B" | "R" | "G" | "S") {
                Some(name.to_string())
            } else {
                None
            }
        })
        .collect()
}

/// The Rust `.so` must export every symbol the C `.so` exports, byte-identical.
#[test]
fn symbol_parity_c_subset_of_rust() {
    let c_path = common::c_library_path();
    let rust_path = common::rust_library_path();

    let c_syms = defined_symbols(&c_path);
    let rust_syms = defined_symbols(&rust_path);

    assert!(
        c_syms.contains("process_decisions"),
        "the C .so should export process_decisions, found {c_syms:?}"
    );

    let missing: Vec<&String> = c_syms.difference(&rust_syms).collect();
    assert!(
        missing.is_empty(),
        "the Rust .so is missing {} symbol(s) exported by the C .so: {missing:?}\n\
         C   ({}): {c_syms:?}\n\
         Rust({}): {rust_syms:?}",
        missing.len(),
        c_path.display(),
        rust_path.display()
    );
}

/// The harness must load two distinct files; otherwise every differential test
/// would be trivially comparing an implementation against itself.
#[test]
fn harness_loads_two_distinct_libraries() {
    let c_path = common::c_library_path().canonicalize().unwrap();
    let rust_path = common::rust_library_path().canonicalize().unwrap();
    assert_ne!(c_path, rust_path, "both halves resolved to the same file");

    let c_bytes = std::fs::read(&c_path).unwrap();
    let rust_bytes = std::fs::read(&rust_path).unwrap();
    assert_ne!(
        c_bytes, rust_bytes,
        "the two shared objects have identical contents"
    );

    // And the two resolved function pointers must genuinely differ.
    let l = common::libs();
    assert_ne!(
        l.c as usize, l.rust as usize,
        "both process_decisions pointers resolved to the same address"
    );
}
