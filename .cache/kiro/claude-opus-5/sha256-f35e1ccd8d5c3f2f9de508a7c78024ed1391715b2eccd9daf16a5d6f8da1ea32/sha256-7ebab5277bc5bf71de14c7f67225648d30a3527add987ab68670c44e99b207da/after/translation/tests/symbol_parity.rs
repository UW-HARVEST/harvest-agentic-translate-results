//! Every dynamic symbol the C .so defines must also be defined by the Rust .so
//! under the exact same name (macro-generated symbols included).

mod common;

use std::collections::BTreeSet;
use std::process::Command;

fn defined_dynamic_symbols(so: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(so)
        .output()
        .expect("run nm");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );

    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|line| {
            let mut it = line.split_whitespace();
            let (_addr, kind, name) = (it.next()?, it.next()?, it.next()?);
            // Skip toolchain/runtime bookkeeping symbols that are not part of
            // the library's API surface.
            let ignored = matches!(
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
                    | "__gnu_lto_slim"
            );
            if ignored || name.starts_with("_ZN") || name.starts_with("__rust") {
                return None;
            }
            match kind {
                "T" | "t" | "D" | "d" | "B" | "b" | "R" | "r" | "W" | "w" | "V" | "i" => {
                    Some(name.to_string())
                }
                _ => None,
            }
        })
        .collect()
}

#[test]
fn rust_so_exports_every_c_symbol() {
    let c = defined_dynamic_symbols(&common::c_lib_path());
    let r = defined_dynamic_symbols(&common::rust_lib_path());

    assert!(
        c.contains("next_double"),
        "sanity: C .so should export next_double, got {c:?}"
    );

    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing symbols exported by the C .so: {missing:?}\n\
         C symbols:    {c:?}\n\
         Rust symbols: {r:?}"
    );
}
