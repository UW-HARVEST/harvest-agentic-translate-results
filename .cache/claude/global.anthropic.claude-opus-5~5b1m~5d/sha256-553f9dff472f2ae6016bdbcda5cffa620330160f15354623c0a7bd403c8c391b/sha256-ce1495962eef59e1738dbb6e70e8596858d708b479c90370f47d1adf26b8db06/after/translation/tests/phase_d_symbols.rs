//! Phase D — symbol parity, enforced as a test so it cannot silently regress.

mod common;

use std::process::Command;

use common::{c_so_path, rust_so_path, Libs};

/// Defined (exported) dynamic symbol names from `nm -D --defined-only`.
fn defined_dynamic_symbols(so: &std::path::Path) -> Vec<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(so)
        .output()
        .expect("failed to run `nm` (binutils required for this test)");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8_lossy(&out.stdout);
    let mut names: Vec<String> = text
        .lines()
        .filter_map(|l| l.split_whitespace().nth(2))
        .map(str::to_owned)
        .collect();
    names.sort();
    names.dedup();
    names
}

#[test]
fn symbol_parity_c_subset_of_rust() {
    let c_syms = defined_dynamic_symbols(&c_so_path());
    let rust_syms = defined_dynamic_symbols(&rust_so_path());

    assert!(
        !c_syms.is_empty(),
        "no symbols found in the C .so — build it first"
    );

    let missing: Vec<&String> = c_syms.iter().filter(|s| !rust_syms.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "the Rust .so is MISSING {} symbol(s) exported by the C .so: {missing:?}\n\
         C   ({}): {c_syms:?}\n\
         Rust({}): {rust_syms:?}",
        missing.len(),
        c_syms.len(),
        rust_syms.len()
    );
}

/// The one documented public symbol must be loadable through `dlsym` from both
/// libraries (this is what the rest of the suite relies on).
#[test]
fn flip_horizontal_resolvable_in_both() {
    let libs = Libs::load();
    // Panics with a clear message if either export is absent.
    let _c = libs.c_flip();
    let _r = libs.rust_flip();
}

/// The C `.so` must not export anything the SYMBOLS.md inventory does not list.
/// If this fails, SYMBOLS.md (and probably the translation) is out of date.
#[test]
fn c_symbol_inventory_is_complete() {
    let c_syms = defined_dynamic_symbols(&c_so_path());
    let documented = ["flip_horizontal"];
    let undocumented: Vec<&String> = c_syms
        .iter()
        .filter(|s| !documented.contains(&s.as_str()))
        .collect();
    assert!(
        undocumented.is_empty(),
        "the C .so exports symbols not documented in SYMBOLS.md: {undocumented:?} \
         — the translation may be incomplete"
    );
}
