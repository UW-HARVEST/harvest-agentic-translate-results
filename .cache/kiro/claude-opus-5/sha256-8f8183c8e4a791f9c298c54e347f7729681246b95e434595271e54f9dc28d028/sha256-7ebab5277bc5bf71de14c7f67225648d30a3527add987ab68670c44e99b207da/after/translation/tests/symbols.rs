//! Step 8 as an enforced test: every dynamic symbol the C `.so` exports must be
//! exported by the Rust `cdylib` under the exact same name, and with the same
//! kind (code vs. data).

mod common;

use std::collections::BTreeMap;
use std::process::Command;

/// Runs `nm -D --defined-only` and returns `name -> symbol type letter`.
fn exported_symbols(path: &std::path::Path) -> BTreeMap<String, char> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(path)
        .output()
        .unwrap_or_else(|e| panic!("failed to run nm on {}: {e}", path.display()));
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );

    let mut map = BTreeMap::new();
    for line in String::from_utf8_lossy(&out.stdout).lines() {
        // "<addr> <type> <name>" — the address column is blank for undefined
        // entries, which --defined-only already filters out.
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() < 3 {
            continue;
        }
        let ty = fields[fields.len() - 2];
        let name = fields[fields.len() - 1];
        if ty.len() != 1 {
            continue;
        }
        map.insert(name.to_string(), ty.chars().next().unwrap());
    }
    assert!(
        !map.is_empty(),
        "nm reported no defined symbols for {}",
        path.display()
    );
    map
}

#[test]
fn rust_exports_every_symbol_the_c_library_exports() {
    let c_syms = exported_symbols(&common::c_library_path());
    let rust_syms = exported_symbols(&common::rust_library_path());

    let missing: Vec<&String> = c_syms.keys().filter(|k| !rust_syms.contains_key(*k)).collect();
    assert!(
        missing.is_empty(),
        "Rust cdylib is missing exports present in the C .so: {missing:?}\n\
         C exports: {:?}",
        c_syms.keys().collect::<Vec<_>>()
    );
}

/// A function must not be exported as data or vice versa; `matrix` in
/// particular has to stay a `D` (initialised data) symbol, not a getter.
#[test]
fn shared_symbols_have_the_same_kind() {
    let c_syms = exported_symbols(&common::c_library_path());
    let rust_syms = exported_symbols(&common::rust_library_path());

    for (name, c_ty) in &c_syms {
        let rust_ty = rust_syms
            .get(name)
            .unwrap_or_else(|| panic!("Rust cdylib does not export `{name}`"));
        // nm reports upper case for global symbols; compare the kind letter.
        assert_eq!(
            c_ty.to_ascii_uppercase(),
            rust_ty.to_ascii_uppercase(),
            "symbol `{name}` is `{c_ty}` in C but `{rust_ty}` in Rust"
        );
    }
}

/// Guards against the C library being rebuilt with a different public surface
/// than the one these tests were written against.
#[test]
fn c_library_exports_the_expected_api() {
    let c_syms = exported_symbols(&common::c_library_path());
    for expected in [
        "matrix",
        "init_array",
        "expand_array",
        "add_element",
        "free_array",
        "process_flags",
        "calculate_matrix_checksum",
        "matrixsum",
    ] {
        assert!(
            c_syms.contains_key(expected),
            "C .so unexpectedly does not export `{expected}`; \
             exports are {:?}",
            c_syms.keys().collect::<Vec<_>>()
        );
    }
}
