//! Verifies that every dynamic symbol exported by the C `.so` is also exported
//! by the Rust `cdylib` under the exact same name.

mod common;

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

/// Names of dynamic symbols *defined* (not merely referenced) by `path`.
fn exported_symbols(path: &Path) -> Option<BTreeSet<String>> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", "--"])
        .arg(path)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    Some(
        text.lines()
            .filter_map(|line| {
                let mut it = line.split_whitespace();
                let (_addr, kind, name) = (it.next()?, it.next()?, it.next()?);
                // Skip linker/loader bookkeeping symbols that are an artefact of
                // the toolchain rather than part of the library's API.
                let ignored = matches!(
                    name,
                    "_init"
                        | "_fini"
                        | "__bss_start"
                        | "_edata"
                        | "_end"
                        | "__data_start"
                        | "data_start"
                        | "_IO_stdin_used"
                        | "__TMC_END__"
                        | "__dso_handle"
                        | "_DYNAMIC"
                        | "_GLOBAL_OFFSET_TABLE_"
                );
                if ignored || kind == "w" || kind == "W" {
                    None
                } else {
                    Some(name.to_string())
                }
            })
            .collect(),
    )
}

#[test]
fn rust_exports_every_c_symbol() {
    let c_path = common::c_lib_path();
    let rust_path = common::rust_lib_path();

    let (Some(c_syms), Some(rust_syms)) = (exported_symbols(&c_path), exported_symbols(&rust_path))
    else {
        eprintln!("`nm` unavailable; skipping symbol-parity check");
        return;
    };

    assert!(
        c_syms.contains("driver"),
        "sanity: C library should export `driver`, got {c_syms:?}"
    );

    let missing: Vec<_> = c_syms.difference(&rust_syms).cloned().collect();
    assert!(
        missing.is_empty(),
        "Rust cdylib is missing symbols exported by the C library: {missing:?}\n\
         C:    {c_syms:?}\n\
         Rust: {:?}",
        rust_syms.iter().take(20).collect::<Vec<_>>()
    );
}
