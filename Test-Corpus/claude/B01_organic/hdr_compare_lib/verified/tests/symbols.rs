//! Phase D — symbol parity, enforced as a test.
//!
//! Every symbol the C `.so` exports must also be exported by the Rust `.so`
//! under the exact same name, and each must be callable through `dlsym`.

mod common;

use common::*;
use std::process::Command;

fn exported_symbols(so: &std::path::Path) -> Vec<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", "--format=posix"])
        .arg(so)
        .output()
        .expect("run nm");
    assert!(out.status.success(), "nm failed on {}", so.display());
    let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let name = it.next()?;
            let kind = it.next()?;
            // Global text/data/bss/rodata symbols only; skip the ELF/glibc
            // boilerplate that both toolchains inject.
            if !matches!(kind, "T" | "D" | "B" | "R" | "W" | "V") {
                return None;
            }
            if name.starts_with("_ITM_")
                || name.starts_with("__")
                || name == "_init"
                || name == "_fini"
                || name == "_edata"
                || name == "_end"
                || name == "__bss_start"
            {
                return None;
            }
            Some(name.to_string())
        })
        .collect();
    v.sort();
    v.dedup();
    v
}

#[test]
fn rust_so_exports_every_c_symbol() {
    let im = load();
    let c_syms = exported_symbols(&im.c_path);
    let rust_syms = exported_symbols(&im.rust_path);

    assert!(
        !c_syms.is_empty(),
        "no exported symbols found in {}",
        im.c_path.display()
    );
    // The C library's documented surface.
    assert_eq!(c_syms, vec!["hdr_compare".to_string()], "C export set changed");

    let missing: Vec<&String> = c_syms.iter().filter(|s| !rust_syms.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}\n\
         C   ({}): {c_syms:?}\n\
         Rust({}): {rust_syms:?}",
        im.c_path.display(),
        im.rust_path.display()
    );

    // Every C symbol must additionally be resolvable via dlsym in the Rust .so.
    unsafe {
        let lib = libloading::Library::new(&im.rust_path).expect("dlopen rust .so");
        for s in &c_syms {
            let mut name = s.clone().into_bytes();
            name.push(0);
            lib.get::<*const ()>(&name)
                .unwrap_or_else(|e| panic!("dlsym {s} in Rust .so: {e}"));
        }
    }
}

/// `hdr_valid` is `static` in the C source: it must not be exported by either
/// library (a translation that leaks it would change the ABI surface).
#[test]
fn static_helper_is_not_exported() {
    let im = load();
    for path in [&im.c_path, &im.rust_path] {
        let syms = exported_symbols(path);
        assert!(
            !syms.iter().any(|s| s == "hdr_valid"),
            "{} unexpectedly exports hdr_valid",
            path.display()
        );
    }
}
