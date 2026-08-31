//! Every symbol the C `.so` exports must also be exported by the Rust `.so`
//! under the exact same name.

mod common;

use common::{c_so, rust_so};
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

/// Dynamic symbols that a shared object defines (i.e. `nm -D` entries that are
/// not undefined `U`). Linker/toolchain bookkeeping symbols are filtered out
/// because they are artifacts of the toolchain, not of the translated source.
fn defined_dynamic_symbols(so: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(so)
        .output()
        .expect("run nm -D");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );

    let ignored_exact = [
        "_init",
        "_fini",
        "__bss_start",
        "_edata",
        "_end",
        "__bss_start__",
        "__bss_end__",
        "_bss_end__",
        "__end__",
        "__TMC_END__",
        "_ITM_deregisterTMCloneTable",
        "_ITM_registerTMCloneTable",
        "__cxa_finalize",
        "__gmon_start__",
        "__dso_handle",
    ];

    let mut set = BTreeSet::new();
    for line in String::from_utf8_lossy(&out.stdout).lines() {
        // Format: "<addr> <type> <name>"
        let mut parts = line.split_whitespace();
        let (Some(_addr), Some(kind), Some(name)) = (parts.next(), parts.next(), parts.next())
        else {
            continue;
        };
        // Only globally visible, non-undefined symbols.
        if kind == "U" || kind == "w" || kind == "v" {
            continue;
        }
        if ignored_exact.contains(&name) {
            continue;
        }
        // Rust std / compiler runtime symbols leaking out of the cdylib.
        if name.starts_with("_ZN") || name.starts_with("rust_") || name.starts_with("__rust") {
            continue;
        }
        set.insert(name.to_string());
    }
    set
}

#[test]
fn rust_so_exports_every_c_symbol() {
    let c = defined_dynamic_symbols(&c_so());
    let r = defined_dynamic_symbols(&rust_so());

    eprintln!("C .so exports:    {c:?}");
    eprintln!("Rust .so exports: {r:?}");

    // The C library must at least export its documented entry point; guards
    // against a silently empty symbol set making this test vacuous.
    assert!(
        c.contains("driver"),
        "C .so unexpectedly does not export `driver`: {c:?}"
    );

    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing {} symbol(s) exported by the C .so: {:?}",
        missing.len(),
        missing
    );
}
