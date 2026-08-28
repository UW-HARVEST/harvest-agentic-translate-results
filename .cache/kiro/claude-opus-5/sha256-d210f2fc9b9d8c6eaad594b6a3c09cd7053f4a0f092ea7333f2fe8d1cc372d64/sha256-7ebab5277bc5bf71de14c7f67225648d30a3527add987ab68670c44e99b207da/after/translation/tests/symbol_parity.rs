//! Verifies that the Rust `cdylib` exports every dynamic symbol the C shared
//! library exports, under whatever feature configuration is active.

mod common;

use std::path::Path;
use std::process::Command;

/// Dynamic symbols that are *defined* by the object (i.e. actually exported),
/// ignoring undefined imports and toolchain-injected runtime symbols.
fn exported_symbols(so: &Path) -> Vec<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(so)
        .output()
        .expect("run nm");
    assert!(
        out.status.success(),
        "nm failed on {}:\n{}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );

    let mut names: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            let (_addr, kind, name) = (fields.next()?, fields.next()?, fields.next()?);
            // Keep code/data exports; drop absolute and debug entries.
            if !matches!(kind, "T" | "t" | "D" | "d" | "B" | "b" | "R" | "r" | "W" | "V") {
                return None;
            }
            Some(name.to_string())
        })
        // Symbols injected by the C/Rust runtimes rather than by the source.
        .filter(|n| {
            !n.starts_with("_ITM_")
                && !n.starts_with("__gnu")
                && !n.starts_with("_Jv_")
                && !matches!(
                    n.as_str(),
                    "_init"
                        | "_fini"
                        | "__bss_start"
                        | "_edata"
                        | "_end"
                        | "__cxa_finalize"
                        | "rust_eh_personality"
                )
        })
        .collect();

    names.sort();
    names.dedup();
    names
}

#[test]
fn rust_so_exports_every_c_symbol() {
    // Loading the pair is what builds both shared objects.
    let _pair = common::Pair::load();

    let c_so = common::c_library_path();
    let rust_so = common::rust_library_path();

    let c_syms = exported_symbols(&c_so);
    let rust_syms = exported_symbols(&rust_so);

    assert!(
        c_syms.contains(&"to_barycentric".to_string()),
        "expected the C .so to export to_barycentric, found: {c_syms:?}"
    );

    let missing: Vec<&String> = c_syms.iter().filter(|s| !rust_syms.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing symbols exported by the C .so: {missing:?}\n  \
         C exports:    {c_syms:?}\n  Rust exports: {rust_syms:?}"
    );
}
