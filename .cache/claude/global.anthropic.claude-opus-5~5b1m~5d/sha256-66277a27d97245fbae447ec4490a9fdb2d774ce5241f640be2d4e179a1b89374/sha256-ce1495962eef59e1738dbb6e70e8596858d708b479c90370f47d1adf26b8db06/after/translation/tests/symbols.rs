// Phase D -- symbol parity between the C .so and the Rust .so.

mod common;
use common::*;

use std::process::Command;

fn global_text_symbols(path: &std::path::Path) -> Vec<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(path)
        .output()
        .expect("run nm");
    assert!(out.status.success(), "nm failed on {path:?}");
    let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let (_addr, kind, name) = (it.next()?, it.next()?, it.next()?);
            // Global text symbols only; drop crt glue and section markers.
            if kind == "T" && !matches!(name, "_init" | "_fini") {
                Some(name.to_string())
            } else {
                None
            }
        })
        .collect();
    v.sort();
    v.dedup();
    v
}

#[test]
fn symbol_parity_c_subset_of_rust() {
    let c = global_text_symbols(&c_so_path());
    let r = global_text_symbols(&rust_so_path());
    assert!(!c.is_empty(), "no symbols found in the C .so");
    let missing: Vec<&String> = c.iter().filter(|s| !r.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but missing from the Rust .so: {missing:?}\n\
         C: {c:?}\nRust: {r:?}"
    );
    assert_eq!(c, vec!["driver".to_string()], "C export set changed");
}

#[test]
fn both_libraries_expose_callable_driver() {
    // Resolving the symbol through dlsym on both objects.
    let cf = c_driver();
    let rf = rust_driver();
    assert!(cf as usize != 0 && rf as usize != 0);
    assert!(
        cf as usize != rf as usize,
        "the two .so files must be distinct objects"
    );
    assert_same(5, 5);
}
