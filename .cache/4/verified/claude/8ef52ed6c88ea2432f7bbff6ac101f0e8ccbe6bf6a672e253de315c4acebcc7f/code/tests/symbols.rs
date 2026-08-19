//! Phase D — symbol parity between the C shared library and the Rust `cdylib`.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

fn dynamic_defined(lib: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(lib)
        .output()
        .expect("run nm");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        lib.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let (_addr, kind, name) = (it.next()?, it.next()?, it.next()?);
            // Keep only strong, global text/data symbols; skip the weak
            // toolchain glue (`_ITM_*`, `__gmon_start__`, ...) which is not part
            // of the API surface of either library.
            match kind {
                "T" | "D" | "B" | "R" => Some(name.to_string()),
                _ => None,
            }
        })
        .collect()
}

fn dynamic_undefined(lib: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("-u")
        .arg(lib)
        .output()
        .expect("run nm");
    assert!(out.status.success(), "nm -u failed");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .map(|s| s.split('@').next().unwrap().to_string())
        .collect()
}

#[test]
fn c_so_exports_exactly_main_and_print_hex_char_line() {
    let a = artifacts();
    let c = dynamic_defined(&a.c_so);
    let expected: BTreeSet<String> = ["main", "printHexCharLine"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    assert_eq!(c, expected, "unexpected C .so symbol surface: {c:?}");

    let c_o2 = dynamic_defined(&a.c_so_o2);
    assert_eq!(c_o2, expected, "unexpected C -O2 .so symbol surface");
}

#[test]
fn every_c_symbol_is_exported_by_the_rust_so() {
    let a = artifacts();
    let c = dynamic_defined(&a.c_so);
    let rust = dynamic_defined(&a.rust_so);

    let missing: Vec<&String> = c.difference(&rust).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but missing from the Rust .so: {missing:?}\n\
         Rust .so exports: {rust:?}"
    );

    // The Rust cdylib must not add API surface either.
    let extra: Vec<&String> = rust.difference(&c).collect();
    assert!(
        extra.is_empty(),
        "the Rust .so exports symbols the C .so does not: {extra:?}"
    );
}

#[test]
fn rust_so_has_no_unresolved_non_libc_symbols() {
    let a = artifacts();
    let undef = dynamic_undefined(&a.rust_so);
    // Everything must be provided by glibc / libgcc / the dynamic loader.
    let allowed_prefixes = [
        "_ITM_", "__gmon_start__", "_Unwind_", "__cxa_", "__tls_get_addr", "__libc_", "_dl_",
    ];
    let unresolved: Vec<&String> = undef
        .iter()
        .filter(|s| {
            if allowed_prefixes.iter().any(|p| s.starts_with(p)) {
                return false;
            }
            // Resolvable through the already-loaded libc/libm/libgcc?
            let probe = std::ffi::CString::new(s.as_str()).unwrap();
            unsafe {
                let this = libloading::os::unix::Library::this();
                this.get::<*const u8>(probe.as_bytes_with_nul()).is_err()
            }
        })
        .collect();
    assert!(
        unresolved.is_empty(),
        "Rust .so has undefined symbols that nothing provides: {unresolved:?}"
    );
}

#[test]
fn both_symbols_are_dlsym_able_in_both_libraries() {
    let l = libs();
    for (name, lib) in [("C", &l.c), ("C-O2", &l.c_o2), ("Rust", &l.rust)] {
        unsafe {
            lib.get::<unsafe extern "C" fn() -> std::os::raw::c_int>(b"main\0")
                .unwrap_or_else(|e| panic!("{name}: dlsym(main): {e}"));
            lib.get::<PrintHexCharLine>(b"printHexCharLine\0")
                .unwrap_or_else(|e| panic!("{name}: dlsym(printHexCharLine): {e}"));
        }
    }
}
