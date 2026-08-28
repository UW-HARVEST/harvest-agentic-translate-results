//! Step 8: every dynamic symbol the C `.so` defines must also be defined by
//! the Rust `.so` under the exact same name.
#![allow(non_snake_case)]

mod common;

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::process::Command;

fn so_paths() -> (PathBuf, PathBuf) {
    // Same resolution (and staleness guard) the FFI harness uses.
    let (c, rs) = common::so_paths();
    (c.clone(), rs.clone())
}

/// Names of symbols *defined* (not merely referenced) in the dynamic table.
fn defined_dynamic_symbols(so: &PathBuf) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(so)
        .output()
        .expect("failed to run `nm`");
    assert!(
        out.status.success(),
        "nm failed on {so:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let _addr = it.next()?;
            let kind = it.next()?;
            let name = it.next()?;
            // Code and data definitions: T/t text, W/w weak, D/d data,
            // B/b bss, R/r read-only data.
            if kind.len() == 1 && "TtWwDdBbRr".contains(kind) {
                Some(name.to_string())
            } else {
                None
            }
        })
        .collect()
}

#[test]
fn rust_so_exports_every_c_symbol() {
    let (c_so, rs_so) = so_paths();
    let c_syms = defined_dynamic_symbols(&c_so);
    let rs_syms = defined_dynamic_symbols(&rs_so);

    assert!(
        !c_syms.is_empty(),
        "no symbols read from the C .so -- check the nm invocation"
    );

    let missing: Vec<&String> = c_syms.difference(&rs_syms).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing {} symbol(s) exported by the C .so: {:?}\n  C  : {c_so:?}\n  RS : {rs_so:?}",
        missing.len(),
        missing
    );

    // Every C symbol must additionally be *callable* by name through
    // libloading from both libraries.
    let libs = common::libs();
    for name in &c_syms {
        let cname = std::ffi::CString::new(name.as_str()).unwrap();
        unsafe {
            let a: Result<libloading::Symbol<*const ()>, _> =
                libs.c.get(cname.as_bytes_with_nul());
            let b: Result<libloading::Symbol<*const ()>, _> =
                libs.rs.get(cname.as_bytes_with_nul());
            assert!(a.is_ok(), "C .so: cannot dlsym `{name}`");
            assert!(b.is_ok(), "Rust .so: cannot dlsym `{name}`");
        }
    }
}
