//! Every dynamic symbol the C shared object exports must also be exported by
//! the Rust shared object under the exact same name.

mod common;

use std::collections::BTreeSet;
use std::process::Command;

/// Dynamic symbols *defined* (not merely referenced) by a shared object,
/// as reported by `nm -D`.
fn exported_symbols(path: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(path)
        .output()
        .expect("failed to run `nm -D`");
    assert!(
        out.status.success(),
        "`nm -D {}` failed: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );

    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|line| {
            // Format: "<addr> <type> <name>" or " <type> <name>" for undefined.
            let mut fields = line.split_whitespace();
            let first = fields.next()?;
            let (kind, name) = if first.len() == 1 {
                (first, fields.next()?)
            } else {
                (fields.next()?, fields.next()?)
            };
            // Skip local symbols and anything without a definition.
            if kind == "U" || kind == "w" || kind == "v" {
                return None;
            }
            Some(name.to_string())
        })
        .collect()
}

/// Symbols that any shared object gets from the linker / toolchain rather than
/// from the translated source, and which therefore carry no API meaning.
fn is_toolchain_symbol(name: &str) -> bool {
    const EXACT: &[&str] = &[
        "_init",
        "_fini",
        "__bss_start",
        "_edata",
        "_end",
        "_IO_stdin_used",
        "__gmon_start__",
        "_ITM_registerTMCloneTable",
        "_ITM_deregisterTMCloneTable",
        "__cxa_finalize",
    ];
    EXACT.contains(&name)
        || name.starts_with("__cxa_")
        || name.starts_with("_Z")
        || name.starts_with("_R")
        || name.starts_with("rust_")
        || name.starts_with("__rust")
        || name.starts_with("__rg_")
}

#[test]
fn rust_so_exports_every_c_symbol() {
    let c_syms = exported_symbols(&common::c_library_path());
    let rust_syms = exported_symbols(&common::rust_library_path());

    // Sanity check: the C library must at least export the documented API.
    assert!(
        c_syms.contains("driver"),
        "C .so does not export `driver`; symbol extraction is broken. Got: {c_syms:?}"
    );

    let missing: Vec<&String> = c_syms
        .iter()
        .filter(|s| !is_toolchain_symbol(s))
        .filter(|s| !rust_syms.contains(*s))
        .collect();

    assert!(
        missing.is_empty(),
        "Rust .so is missing {} symbol(s) exported by the C .so: {:?}\n\
         C symbols: {:?}\nRust symbols: {:?}",
        missing.len(),
        missing,
        c_syms,
        rust_syms
    );
}
