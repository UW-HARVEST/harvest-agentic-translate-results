//! Every symbol the C `.so` exports must also be exported by the Rust `.so`
//! under the exact same name.

mod common;

use std::collections::BTreeSet;
use std::process::Command;

fn exported_symbols(path: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
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
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let (_addr, kind, name) = (it.next()?, it.next()?, it.next()?);
            // Only global/weak code & data definitions, and skip the linker's
            // own bookkeeping symbols which are not part of the API.
            if !matches!(kind, "T" | "D" | "B" | "R" | "W" | "V" | "G" | "S") {
                return None;
            }
            if name.starts_with('_') {
                return None;
            }
            Some(name.to_string())
        })
        .collect()
}

#[test]
fn rust_so_exports_every_c_symbol() {
    let c_syms = exported_symbols(&common::c_so_path());
    let rust_syms = exported_symbols(&common::rust_so_path());

    assert!(
        c_syms.contains("bin2hex"),
        "sanity check failed: C .so does not export bin2hex (found {c_syms:?})"
    );

    let missing: Vec<&String> = c_syms.difference(&rust_syms).collect();
    assert!(
        missing.is_empty(),
        "the Rust .so is missing these C exports: {missing:?}"
    );
}
