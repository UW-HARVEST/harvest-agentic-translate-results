//! Export parity: every dynamic symbol the C `.so` defines must also be
//! defined by the Rust `.so` under the exact same name.

mod common;

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

/// Names of symbols defined (not undefined) in the dynamic symbol table.
fn defined_dynamic_symbols(path: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(path)
        .output()
        .unwrap_or_else(|e| panic!("failed to run nm on {}: {e}", path.display()));
    assert!(
        out.status.success(),
        "nm -D --defined-only {} failed: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8_lossy(&out.stdout);
    let mut set = BTreeSet::new();
    for line in text.lines() {
        // "<addr> <type> <name>" or "         <type> <name>"
        let name = match line.split_whitespace().last() {
            Some(n) => n,
            None => continue,
        };
        if line.split_whitespace().count() < 2 {
            continue;
        }
        // Skip the toolchain/runtime bookkeeping symbols that every ELF gets.
        if matches!(
            name,
            "_init" | "_fini" | "__bss_start" | "_edata" | "_end" | "_DYNAMIC" | "_GLOBAL_OFFSET_TABLE_"
        ) {
            continue;
        }
        set.insert(name.to_string());
    }
    set
}

#[test]
fn rust_exports_every_c_symbol() {
    let c_path = common::c_library_path();
    let rust_path = common::rust_library_path();

    let c_syms = defined_dynamic_symbols(&c_path);
    let rust_syms = defined_dynamic_symbols(&rust_path);

    let missing: Vec<&String> = c_syms.difference(&rust_syms).collect();
    assert!(
        missing.is_empty(),
        "Rust .so ({}) is missing symbols exported by the C .so ({}): {missing:?}\n\
         C symbols: {c_syms:?}\nRust symbols: {rust_syms:?}",
        rust_path.display(),
        c_path.display(),
    );

    // Sanity: the public API from c_src/include/lib.h must actually be there.
    assert!(c_syms.contains("crc16"), "C .so does not export crc16");
    assert!(rust_syms.contains("crc16"), "Rust .so does not export crc16");
}
