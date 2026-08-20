//! Phase D — exported-symbol parity between the C and Rust shared libraries.

mod common;

use std::collections::BTreeSet;
use std::process::Command;

fn defined_symbols(path: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(path)
        .output()
        .expect("run nm");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().nth(2).map(|s| s.to_string()))
        .collect()
}

fn undefined_symbols(path: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "-u"])
        .arg(path)
        .output()
        .expect("run nm");
    assert!(out.status.success(), "nm -u failed on {}", path.display());
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .filter(|s| s != "U" && s != "w")
        .collect()
}

/// Every symbol the C `.so` exports must be exported by the Rust `.so` too.
#[test]
fn c_symbols_are_all_exported_by_rust() {
    let c = defined_symbols(&common::c_so());
    let rust = defined_symbols(&common::rust_so());

    // The C library defines exactly the two functions of c_src/src/main.c.
    for expected in ["driver", "main"] {
        assert!(
            c.contains(expected),
            "C .so unexpectedly lacks `{expected}`; found {c:?}"
        );
    }

    let missing: Vec<&String> = c.difference(&rust).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but missing from the Rust .so: {missing:?}\n\
         C: {c:?}\nRust: {rust:?}"
    );
}

/// The Rust `.so` must have no undefined symbol outside libc / the dynamic
/// loader, which would mean a piece of C was never translated.
#[test]
fn rust_so_has_no_non_libc_undefined_symbols() {
    for path in [common::c_so(), common::rust_so()] {
        let undef = undefined_symbols(&path);
        let unexpected: Vec<&String> = undef
            .iter()
            .filter(|s| {
                // Versioned imports (`name@GLIBC_x.y`) come from libc itself.
                let versioned = s.contains("@GLIBC") || s.contains("@GCC");
                let name = s.split('@').next().unwrap_or(s);
                // Unversioned leftovers are the usual weak toolchain stubs.
                let toolchain = name.starts_with("__")
                    || name.starts_with("_ITM_")
                    || name.starts_with("_Unwind_");
                !(versioned || toolchain)
            })
            .collect();
        assert!(
            unexpected.is_empty(),
            "unexpected (non-libc) undefined symbols in {}: {unexpected:?}",
            path.display()
        );
    }

    // Successfully `dlopen`ing both libraries additionally proves that every
    // non-weak undefined symbol actually resolves at load time.
    unsafe {
        libloading::Library::new(common::c_so()).expect("dlopen C .so");
        libloading::Library::new(common::rust_so()).expect("dlopen Rust .so");
    }
}

/// Both exports must be reachable through `dlsym`, i.e. the `#[no_mangle]`
/// wrappers really are part of the dynamic symbol table.
#[test]
fn both_exports_are_dlsym_reachable() {
    use libloading::{Library, Symbol};
    for path in [common::c_so(), common::rust_so()] {
        unsafe {
            let lib = Library::new(&path).expect("dlopen");
            let driver: Result<Symbol<unsafe extern "C" fn(std::os::raw::c_int)>, _> =
                lib.get(b"driver\0");
            assert!(driver.is_ok(), "no `driver` in {}", path.display());
            let main: Result<Symbol<unsafe extern "C" fn() -> std::os::raw::c_int>, _> =
                lib.get(b"main\0");
            assert!(main.is_ok(), "no `main` in {}", path.display());
        }
    }
}
