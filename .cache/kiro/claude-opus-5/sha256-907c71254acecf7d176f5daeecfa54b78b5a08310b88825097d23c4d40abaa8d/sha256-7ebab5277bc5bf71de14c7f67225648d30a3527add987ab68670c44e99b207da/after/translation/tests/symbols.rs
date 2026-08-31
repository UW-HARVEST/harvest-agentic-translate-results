//! Every dynamic symbol the C `libdriver.so` exports must also be exported by
//! the Rust `libdriver.so`, under the exact same name.

mod common;

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

/// Names of the `T`/`D`/`B`/`W`/... (defined) dynamic symbols in `lib`.
fn exported_symbols(lib: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(lib)
        .output()
        .expect("failed to invoke nm");
    assert!(
        out.status.success(),
        "nm -D {} failed: {}",
        lib.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|line| line.split_whitespace().nth(2).map(str::to_owned))
        .collect()
}

#[test]
fn rust_exports_every_c_symbol() {
    let c_path = common::c_so();
    let rust_path = common::rust_so();

    let c_syms = exported_symbols(&c_path);
    let rust_syms = exported_symbols(&rust_path);

    assert!(
        c_syms.contains("driver") && c_syms.contains("run"),
        "unexpected C symbol table: {c_syms:?}"
    );

    let missing: Vec<_> = c_syms.difference(&rust_syms).cloned().collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing symbols exported by the C .so: {missing:?}\n\
         C:    {c_syms:?}\n\
         Rust: {rust_syms:?}"
    );
}
