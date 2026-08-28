//! Phase A / Phase D — symbol parity between the C `.so` and the Rust `.so`.

mod common;

use std::collections::BTreeSet;
use std::process::Command;

fn exported(path: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(path)
        .output()
        .expect("run nm");
    assert!(out.status.success(), "nm failed on {path:?}");
    let text = String::from_utf8_lossy(&out.stdout);
    let mut set = BTreeSet::new();
    for line in text.lines() {
        let cols: Vec<&str> = line.split_whitespace().collect();
        // "<addr> <type> <name>"  or  "         <type> <name>"
        if cols.len() >= 2 {
            let (ty, name) = (cols[cols.len() - 2], cols[cols.len() - 1]);
            // Only the global text/data symbols form the ABI surface.
            if ty == "T" || ty == "D" || ty == "B" || ty == "W" {
                set.insert(name.to_string());
            }
        }
    }
    set
}

/// Symbols the Rust toolchain always emits for a cdylib; not part of the C ABI
/// surface and therefore ignored when looking for *extra* Rust exports.
fn is_rust_runtime_symbol(s: &str) -> bool {
    s.starts_with("_ZN")
        || s.starts_with("_R")
        || s.starts_with("rust_")
        || s.starts_with("__rust")
        || s.starts_with("_ITM_")
        || s.starts_with("__cxa")
        || s == "_init"
        || s == "_fini"
        || s.contains("17h") // legacy mangled
}

#[test]
fn c_symbols_are_all_exported_by_rust() {
    let p = common::libs();
    let c = exported(&p.c.path);
    let r = exported(&p.r.path);

    let missing: Vec<&String> = c.iter().filter(|s| !r.contains(*s)).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}\n\
         C  = {:?}\nRUST = {:?}",
        p.c.path,
        p.r.path
    );

    // Sanity: the 16 documented symbols really are there.
    for want in [
        "stbds_arrgrowf",
        "stbds_arrfreef",
        "stbds_rand_seed",
        "stbds_hash_string",
        "stbds_hash_bytes",
        "stbds_hmfree_func",
        "stbds_hmget_key_ts",
        "stbds_hmget_key",
        "stbds_hmput_default",
        "stbds_hmput_key",
        "stbds_shmode_func",
        "stbds_hmdel_key",
        "stbds_stralloc",
        "stbds_strreset",
        "strkey",
        "arr_del",
    ] {
        assert!(c.contains(want), "C .so is missing {want}");
        assert!(r.contains(want), "Rust .so is missing {want}");
    }

    // `stbds_unit_tests` is declared but never defined in the C TU.
    assert!(!c.contains("stbds_unit_tests"));
    assert!(!r.contains("stbds_unit_tests"));

    // No unexpected extra C-ABI exports on the Rust side.
    let extra: Vec<&String> = r
        .iter()
        .filter(|s| !c.contains(*s) && !is_rust_runtime_symbol(s))
        .collect();
    assert!(extra.is_empty(), "unexpected extra Rust exports: {extra:?}");
}

/// `nm -D -u` must not report any *library* symbol (i.e. `stbds_*`, `arr_del`,
/// `strkey`) as undefined in the Rust `.so`; the only permitted undefined
/// symbols are libc / Rust-runtime imports, which the dynamic loader resolves
/// (proved by the fact that `dlopen` of the Rust `.so` succeeded at all).
#[test]
fn no_undefined_library_symbols_in_rust_so() {
    let p = common::libs();
    let out = Command::new("nm")
        .args(["-D", "-u"])
        .arg(&p.r.path)
        .output()
        .expect("run nm");
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    let bad: Vec<&str> = text
        .lines()
        .filter(|l| {
            let n = l.split_whitespace().last().unwrap_or("");
            n.starts_with("stbds_") || n.starts_with("arr_del") || n.starts_with("strkey")
        })
        .collect();
    assert!(bad.is_empty(), "unresolved library symbols: {bad:?}");
}
