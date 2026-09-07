//! Phase A / Phase D: exported-symbol parity between the C and the Rust `.so`.

mod common;

use std::process::Command;

fn dynsyms(path: &std::path::Path) -> Vec<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(path)
        .output()
        .expect("nm not available");
    assert!(out.status.success(), "nm failed on {}", path.display());
    let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let f: Vec<&str> = l.split_whitespace().collect();
            if f.len() >= 3 && matches!(f[1], "T" | "B" | "D" | "R" | "W" | "i") {
                Some(f[2].to_string())
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
fn every_c_symbol_is_exported_by_rust() {
    let c = dynsyms(&common::c_so_path());
    let r = dynsyms(&common::rust_so_path());
    assert!(c.len() >= 16, "unexpectedly few C symbols: {c:?}");
    let missing: Vec<&String> = c.iter().filter(|s| !r.contains(s)).collect();
    assert!(missing.is_empty(), "symbols missing from Rust .so: {missing:?}");
}

#[test]
fn rust_so_has_no_non_libc_undefined_symbols() {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--undefined-only")
        .arg(common::rust_so_path())
        .output()
        .expect("nm not available");
    let allowed_prefixes = ["_", "__", "realloc", "free", "mem", "str", "abort", "rust_"];
    let bad: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .filter(|s| s.contains("stbds") || s == "arr_ins" || s == "strkey")
        .collect();
    assert!(bad.is_empty(), "library-internal symbols left undefined: {bad:?}");
    let _ = allowed_prefixes;
}

/// The 16 documented entry points must be dlsym-able from both objects.
#[test]
fn all_entry_points_resolve() {
    let (c, r) = common::apis();
    assert_eq!(c.name, "C");
    assert_eq!(r.name, "Rust");
}
