//! Verifies the Rust cdylib exports every dynamic symbol the C shared library
//! exports, under the exact same name.

mod common;

use common::Harness;
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

/// Returns the set of dynamic symbols *defined* by a shared object, as
/// reported by `nm -D --defined-only`.
fn defined_dynamic_symbols(path: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(path)
        .output()
        .expect("failed to run `nm`");
    assert!(
        out.status.success(),
        "nm -D --defined-only {} failed: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );

    let text = String::from_utf8_lossy(&out.stdout);
    let mut set = BTreeSet::new();
    for line in text.lines() {
        let mut fields = line.split_whitespace();
        // Format: "<addr> <type> <name>" for defined symbols.
        let (Some(_addr), Some(kind), Some(name)) =
            (fields.next(), fields.next(), fields.next())
        else {
            continue;
        };
        // Keep only globally visible code/data definitions, skipping the
        // toolchain-injected bookkeeping symbols that are not part of the API.
        if !matches!(kind, "T" | "D" | "B" | "R" | "W" | "i") {
            continue;
        }
        if is_toolchain_symbol(name) {
            continue;
        }
        set.insert(name.to_string());
    }
    set
}

/// Symbols emitted by the C toolchain / CRT that are not part of the library's
/// own API surface.
fn is_toolchain_symbol(name: &str) -> bool {
    matches!(
        name,
        "_init"
            | "_fini"
            | "__bss_start"
            | "_edata"
            | "_end"
            | "__gmon_start__"
            | "_ITM_registerTMCloneTable"
            | "_ITM_deregisterTMCloneTable"
            | "__cxa_finalize"
    ) || name.starts_with("_ZN")
        || name.starts_with("__rust")
        || name.starts_with("rust_")
}

#[test]
fn rust_so_exports_every_c_symbol() {
    let h = Harness::load();

    let c_syms = defined_dynamic_symbols(&h.c_path);
    let rust_syms = defined_dynamic_symbols(&h.rust_path);

    assert!(
        !c_syms.is_empty(),
        "no symbols found in the C .so at {} — is it built?",
        h.c_path.display()
    );

    let missing: Vec<&String> = c_syms.difference(&rust_syms).collect();
    assert!(
        missing.is_empty(),
        "Rust .so ({}) is missing symbols exported by the C .so ({}): {:?}\n\
         C symbols: {:?}\nRust symbols: {:?}",
        h.rust_path.display(),
        h.c_path.display(),
        missing,
        c_syms,
        rust_syms
    );
}

#[test]
fn memchra2_is_exported_by_both() {
    let h = Harness::load();
    for path in [&h.c_path, &h.rust_path] {
        let syms = defined_dynamic_symbols(path);
        assert!(
            syms.contains("memchra2"),
            "{} does not export `memchra2` (has: {:?})",
            path.display(),
            syms
        );
    }
}
