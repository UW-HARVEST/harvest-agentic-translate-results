//! Phase D: every dynamic symbol the C `.so` exports must also be exported by
//! the Rust `.so`, under the exact same name.

mod common;

use common::{c_lib_path, rust_lib_path};
use std::collections::BTreeSet;
use std::process::Command;

fn exported(path: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", path.to_str().unwrap()])
        .output()
        .expect("running nm");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .collect()
}

#[test]
fn every_c_symbol_is_exported_by_rust() {
    let c = exported(&c_lib_path());
    let r = exported(&rust_lib_path());

    assert_eq!(
        c.len(),
        5,
        "the C .so is expected to export exactly 5 symbols, got {:?}",
        c
    );
    for want in [
        "betagamma",
        "create_block",
        "allocate_block",
        "free_block",
        "compute_hash",
    ] {
        assert!(c.contains(want), "C .so is missing {want}?");
    }

    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but NOT by the Rust .so: {missing:?}"
    );
}

#[test]
fn rust_so_has_no_unresolved_non_libc_symbols() {
    let out = Command::new("nm")
        .args(["-D", "--undefined-only", rust_lib_path().to_str().unwrap()])
        .output()
        .expect("running nm");
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    // Everything must come from libc / libgcc_s / ld.so, i.e. `ldd` must not
    // report any "not found" entry.
    let ldd = Command::new("ldd")
        .arg(rust_lib_path())
        .output()
        .expect("running ldd");
    let ldd_text = String::from_utf8_lossy(&ldd.stdout);
    assert!(
        !ldd_text.contains("not found"),
        "unresolved shared-object dependency:\n{ldd_text}\nundefined syms:\n{text}"
    );
}
