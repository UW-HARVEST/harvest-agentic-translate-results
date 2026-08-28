//! Phase A / Phase D: exported-symbol parity between the C `.so` and the Rust
//! `.so`, checked by shelling out to `nm -D`.

mod common;

use std::collections::BTreeSet;
use std::process::Command;

fn nm_defined(path: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(path)
        .output()
        .expect("run nm");
    assert!(out.status.success(), "nm failed on {}", path.display());
    let text = String::from_utf8_lossy(&out.stdout);
    let mut set = BTreeSet::new();
    for line in text.lines() {
        // "<addr> <type> <name>"
        let mut it = line.split_whitespace();
        let _addr = it.next();
        let ty = it.next();
        let name = it.next();
        if let (Some(ty), Some(name)) = (ty, name) {
            // ignore the linker/CRT bookkeeping symbols that only differ
            // because of the toolchain, never part of the library's API
            if name.starts_with("_ITM_")
                || name.starts_with("__gmon")
                || name == "_init"
                || name == "_fini"
                || name == "__bss_start"
                || name == "_edata"
                || name == "_end"
            {
                continue;
            }
            let _ = ty;
            set.insert(name.to_string());
        }
    }
    set
}

fn nm_undefined_non_libc(path: &std::path::Path) -> Vec<String> {
    let out = Command::new("nm")
        .args(["-D", "--undefined-only"])
        .arg(path)
        .output()
        .expect("run nm");
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    let mut v = Vec::new();
    for line in text.lines() {
        let mut it = line.split_whitespace();
        let first = it.next().unwrap_or("");
        // weak symbols show as "w <name>", strong as "U <name>"
        let (kind, name) = if first == "U" || first == "w" {
            (first, it.next().unwrap_or(""))
        } else {
            (it.next().unwrap_or(""), it.next().unwrap_or(""))
        };
        if kind != "U" {
            continue; // weak/optional
        }
        let bare = name.split('@').next().unwrap_or(name);
        v.push(bare.to_string());
    }
    v
}

/// Every symbol the C `.so` exports must also be exported by the Rust `.so`,
/// with the exact same name.
#[test]
fn symbol_parity_c_so_vs_rust_so() {
    let c = common::c_so_path();
    let rs = common::rust_so_path();
    let c_syms = nm_defined(&c);
    let rs_syms = nm_defined(&rs);

    let missing: Vec<&String> = c_syms.difference(&rs_syms).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing {} symbol(s) exported by the C .so: {:?}\nC:    {:?}\nRust: {:?}",
        missing.len(),
        missing,
        c_syms,
        rs_syms
    );

    // The nine documented API symbols must really be there (guards against a
    // regression where `nm` output parsing silently yields an empty set).
    for want in [
        "convert_pix",
        "cp_inflate",
        "cp_error_reason",
        "cp_fixed_table",
        "cp_permutation_order",
        "cp_len_extra_bits",
        "cp_len_base",
        "cp_dist_extra_bits",
        "cp_dist_base",
    ] {
        assert!(c_syms.contains(want), "C .so lost {want}");
        assert!(rs_syms.contains(want), "Rust .so lost {want}");
    }
}

/// No undefined symbol of either object may be one of the *library's own* API
/// symbols (that would mean the implementation is missing and only a reference
/// survived), and every undefined symbol must actually resolve — enforced by
/// dlopen'ing both objects with `RTLD_NOW`, which fails on the first
/// unresolvable reference.
#[test]
fn no_undefined_non_libc_symbols() {
    let c = common::c_so_path();
    let rs = common::rust_so_path();
    let api = nm_defined(&c);

    for path in [&c, &rs] {
        for sym in nm_undefined_non_libc(path) {
            assert!(
                !api.contains(&sym),
                "{}: API symbol {sym} is undefined (implementation missing)",
                path.display()
            );
        }
        // RTLD_NOW: resolve everything eagerly.
        let flags = libloading::os::unix::RTLD_NOW | libloading::os::unix::RTLD_LOCAL;
        let lib = unsafe { libloading::os::unix::Library::open(Some(path), flags) };
        assert!(
            lib.is_ok(),
            "{} has unresolvable undefined symbols: {:?}",
            path.display(),
            lib.err()
        );
    }
}
