//! Verifies the Rust `cdylib` exports every dynamic symbol the C shared library
//! exports, under the same names.

mod common;

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

/// Dynamic symbols *defined* by an object, as reported by `nm -D`.
fn defined_dynamic_symbols(path: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", "--format=posix"])
        .arg(path)
        .output()
        .expect("run nm");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );

    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            let name = parts.next()?;
            let kind = parts.next()?;
            // Keep code and data symbols; drop the rest.
            matches!(kind, "T" | "D" | "B" | "R" | "W" | "V" | "G" | "S" | "i")
                .then(|| name.to_string())
        })
        .collect()
}

#[test]
fn rust_library_exports_every_c_symbol() {
    let c_syms = defined_dynamic_symbols(&common::c_library_path());
    let rust_syms = defined_dynamic_symbols(&common::rust_library_path());

    assert!(
        !c_syms.is_empty(),
        "no symbols parsed from the C library; nm output format may have changed"
    );

    let missing: Vec<&String> = c_syms.difference(&rust_syms).collect();
    assert!(
        missing.is_empty(),
        "Rust library is missing exports present in the C library: {missing:?}"
    );
}

/// Guards against the C library growing a symbol that the tests never exercise.
#[test]
fn c_library_exports_the_expected_public_api() {
    let c_syms = defined_dynamic_symbols(&common::c_library_path());
    let expected: BTreeSet<String> = ["bad", "driver", "good", "printLine"]
        .iter()
        .map(|s| s.to_string())
        .collect();

    assert_eq!(
        c_syms, expected,
        "C library's exported symbol set changed; update the differential tests"
    );
}
