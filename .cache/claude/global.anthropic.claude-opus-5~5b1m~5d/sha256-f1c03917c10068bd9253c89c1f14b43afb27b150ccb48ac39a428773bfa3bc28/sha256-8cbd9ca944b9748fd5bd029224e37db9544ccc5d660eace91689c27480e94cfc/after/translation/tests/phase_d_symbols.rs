//! Phase D — symbol parity, enforced as a test so it cannot silently regress.
//!
//! Every symbol the C `.so` exports must also be exported by the Rust `.so`
//! under the exact same name, and must be loadable via `dlsym`.

mod common;

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::process::Command;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn rust_so() -> PathBuf {
    common::rust_so_path()
}

/// Names of dynamic symbols DEFINED (uppercase type letter) by `so`.
fn defined_dynamic_symbols(so: &PathBuf) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", so.to_str().unwrap()])
        .output()
        .expect("run nm");
    assert!(out.status.success(), "nm failed on {so:?}");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let a = it.next()?;
            let b = it.next()?;
            // "<addr> <type> <name>" or "<type> <name>" for absolute symbols
            match it.next() {
                Some(name) if a.chars().all(|c| c.is_ascii_hexdigit()) && b.len() == 1 => {
                    Some(name.to_string())
                }
                None if a.len() == 1 => Some(b.to_string()),
                _ => None,
            }
        })
        // Ignore compiler/linker bookkeeping symbols that are not part of the API.
        .filter(|n| !n.starts_with("_ITM_") && !n.starts_with("__gmon") && n != "_init" && n != "_fini")
        .collect()
}

#[test]
fn d1_every_c_symbol_is_exported_by_rust() {
    let c_syms = defined_dynamic_symbols(&manifest_dir().join("../c_src/build/libdriver.so"));
    let r_syms = defined_dynamic_symbols(&rust_so());

    assert!(
        c_syms.contains("driver")
            && c_syms.contains("forward_goto_example")
            && c_syms.contains("open_with_cleanup"),
        "sanity: C .so should export the three goto.c functions, got {c_syms:?}"
    );

    let missing: Vec<&String> = c_syms.difference(&r_syms).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing {} symbol(s) exported by the C .so: {missing:?}\n\
         C   : {c_syms:?}\n Rust: {r_syms:?}",
        missing.len()
    );
}

#[test]
fn d2_every_c_symbol_is_dlsym_resolvable_in_rust() {
    // Independent of `nm`: actually resolve each name through dlopen/dlsym.
    let c_syms = defined_dynamic_symbols(&manifest_dir().join("../c_src/build/libdriver.so"));
    let lib = unsafe { libloading::Library::new(rust_so()) }.expect("dlopen Rust .so");
    for name in &c_syms {
        let r: Result<libloading::Symbol<*const ()>, _> = unsafe { lib.get(name.as_bytes()) };
        assert!(r.is_ok(), "dlsym(\"{name}\") failed on the Rust .so");
    }
}

#[test]
fn d3_rust_so_has_no_unresolved_non_libc_symbols() {
    // `nm -D -u` lists undefined dynamic symbols; loading the library proves
    // they all resolve against the process's libc / runtime.
    let out = Command::new("nm")
        .args(["-D", "-u", rust_so().to_str().unwrap()])
        .output()
        .expect("run nm");
    assert!(out.status.success());
    let undef: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(str::to_string))
        .collect();
    // The definitive check: dlopen succeeds only if every undefined symbol that
    // is needed can be bound.
    let _lib = unsafe { libloading::Library::new(rust_so()) }
        .unwrap_or_else(|e| panic!("Rust .so failed to load ({e}); undefined symbols: {undef:?}"));
}
