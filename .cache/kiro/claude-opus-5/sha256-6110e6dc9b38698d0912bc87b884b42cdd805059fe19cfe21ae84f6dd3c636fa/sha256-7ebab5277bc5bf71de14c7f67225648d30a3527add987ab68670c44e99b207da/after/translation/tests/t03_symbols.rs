//! Every symbol the C shared library exports must also be exported, under the
//! exact same name, by the Rust shared library.

mod common;

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

/// Names of the dynamic symbols *defined* (i.e. exported) by `so`, as reported
/// by `nm -D --defined-only`.
fn exported_symbols(so: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(so)
        .output()
        .expect("run nm");
    assert!(
        out.status.success(),
        "nm -D {} failed: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );

    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|line| line.split_whitespace().nth(2).map(str::to_string))
        .collect()
}

#[test]
fn rust_exports_every_c_symbol() {
    let c_so = common::c_lib_path();
    let rs_so = common::rust_lib_path();

    let c_syms = exported_symbols(c_so);
    let rs_syms = exported_symbols(rs_so);

    assert!(
        c_syms.contains("static_sum") && c_syms.contains("driver"),
        "sanity check: the C library should export static_sum and driver, got {c_syms:?}"
    );

    let missing: Vec<&String> = c_syms.difference(&rs_syms).collect();
    assert!(
        missing.is_empty(),
        "the Rust shared library is missing symbols exported by the C library: {missing:?}\n\
         C exports  : {c_syms:?}\n\
         Rust exports: {rs_syms:?}"
    );

    // Every exported C symbol must also be resolvable through `dlsym`, which is
    // what an external caller actually relies on.
    let pair = common::Pair::load();
    for name in &c_syms {
        let mut key = name.as_bytes().to_vec();
        key.push(0);
        unsafe {
            pair.c
                .get::<*const ()>(&key)
                .unwrap_or_else(|e| panic!("dlsym {name} in the C library: {e}"));
            pair.rs
                .get::<*const ()>(&key)
                .unwrap_or_else(|e| panic!("dlsym {name} in the Rust library: {e}"));
        }
    }
}
