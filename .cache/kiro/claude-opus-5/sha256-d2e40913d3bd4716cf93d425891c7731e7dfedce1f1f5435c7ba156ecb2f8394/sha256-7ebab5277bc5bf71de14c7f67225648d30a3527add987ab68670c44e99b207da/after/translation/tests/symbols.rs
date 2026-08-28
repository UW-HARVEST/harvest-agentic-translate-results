//! Verifies the Rust cdylib exports every dynamic symbol the C .so exports.

mod common;

use std::collections::BTreeSet;
use std::process::Command;

fn dynamic_defined_symbols(path: &std::path::Path) -> BTreeSet<String> {
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
    let text = String::from_utf8_lossy(&out.stdout);
    text.lines()
        .filter_map(|line| line.split_whitespace().last().map(str::to_string))
        .filter(|s| !s.is_empty())
        .collect()
}

#[test]
fn rust_so_exports_every_c_symbol() {
    let c_path = common::c_lib_path();
    let rust_path = common::rust_lib_path();

    let c_syms = dynamic_defined_symbols(&c_path);
    let rust_syms = dynamic_defined_symbols(&rust_path);

    assert!(
        c_syms.contains("float2half"),
        "sanity: C .so should export float2half, got {:?}",
        c_syms
    );

    let missing: Vec<&String> = c_syms.difference(&rust_syms).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing symbols exported by the C .so: {:?}\nC: {:?}\nRust: {:?}",
        missing,
        c_syms,
        rust_syms
    );
}
