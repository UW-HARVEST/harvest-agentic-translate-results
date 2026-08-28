//! Step 8: every dynamic symbol the C `.so` exports must also be exported by
//! the Rust `.so`, under the exact same name.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

/// Names of the globally-visible symbols *defined* by `path`.
fn exported_symbols(path: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
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
            let mut it = line.split_whitespace();
            let (_addr, kind, name) = (it.next()?, it.next()?, it.next()?);
            // Code/data definitions only; skip linker-synthesised entries.
            match kind {
                "T" | "D" | "B" | "R" | "W" => Some(name.to_string()),
                _ => None,
            }
        })
        .filter(|n| {
            !matches!(
                n.as_str(),
                "_init" | "_fini" | "_edata" | "_end" | "__bss_start"
            )
        })
        .collect()
}

#[test]
fn rust_exports_every_c_symbol() {
    // Loading the pair also forces the Rust cdylib to be built.
    let _pair = Pair::load();

    let c = exported_symbols(&c_lib_path_pub());
    let rs = exported_symbols(&rust_lib_path_pub());

    assert!(!c.is_empty(), "nm reported no symbols for the C library");

    let missing: Vec<&String> = c.difference(&rs).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing symbols exported by the C .so: {missing:?}\n\
         C exports:    {c:?}\n\
         Rust exports: {rs:?}"
    );

    // The seven functions in lib.c, for good measure.
    for f in [
        "classify_mode",
        "apply_multiplier",
        "convert_time_factor",
        "convert_negative_overflow",
        "get_modified_time",
        "hash_time_value",
        "modeselect",
    ] {
        assert!(c.contains(f), "C .so unexpectedly lacks {f}");
        assert!(rs.contains(f), "Rust .so lacks {f}");
    }
}
