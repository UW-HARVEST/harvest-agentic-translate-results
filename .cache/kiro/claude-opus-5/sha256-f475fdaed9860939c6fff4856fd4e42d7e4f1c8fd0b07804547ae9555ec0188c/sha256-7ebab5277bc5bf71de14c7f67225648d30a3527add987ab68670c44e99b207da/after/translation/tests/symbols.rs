//! Every dynamic symbol the C `.so` exports must also be exported by the Rust
//! `.so` under the exact same name.

mod common;

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

/// Defined (exported) dynamic symbol names of a shared object, via `nm -D
/// --defined-only`.
fn exported_symbols(so: &Path) -> BTreeSet<String> {
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
    let text = String::from_utf8_lossy(&out.stdout);
    let mut set = BTreeSet::new();
    for line in text.lines() {
        let fields: Vec<&str> = line.split_whitespace().collect();
        // "<addr> <type> <name>"
        if fields.len() >= 3 {
            let ty = fields[fields.len() - 2];
            let name = fields[fields.len() - 1];
            // Ignore compiler/runtime-provided housekeeping symbols; they are
            // not part of the library's API.
            let housekeeping = matches!(
                name,
                "_init"
                    | "_fini"
                    | "__bss_start"
                    | "_edata"
                    | "_end"
                    | "__libc_csu_init"
                    | "__libc_csu_fini"
            );
            if !housekeeping && ty != "a" && ty != "N" {
                set.insert(name.to_string());
            }
        }
    }
    set
}

#[test]
fn rust_so_exports_every_c_symbol() {
    let c_syms = exported_symbols(common::c_lib_path());
    let rust_syms = exported_symbols(common::rust_lib_path());

    assert!(
        c_syms.contains("FIO_createFilename_fromOutDir"),
        "sanity: C symbols were parsed, got {c_syms:?}"
    );

    let missing: Vec<&String> = c_syms.difference(&rust_syms).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing symbols exported by the C .so: {missing:?}\n  C:    {c_syms:?}\n  Rust: {rust_syms:?}"
    );
}
