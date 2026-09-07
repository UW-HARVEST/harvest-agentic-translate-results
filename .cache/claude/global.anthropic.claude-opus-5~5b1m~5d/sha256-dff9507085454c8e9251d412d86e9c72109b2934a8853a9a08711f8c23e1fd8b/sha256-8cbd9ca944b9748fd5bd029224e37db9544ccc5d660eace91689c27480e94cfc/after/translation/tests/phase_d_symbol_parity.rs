//! Phase D — symbol parity between the two shared objects, enforced in CI.

mod common;

use common::so_paths;
use std::collections::BTreeSet;
use std::process::Command;

fn defined_dynamic_symbols(so: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", &so.to_string_lossy()])
        .output()
        .expect("`nm` must be available");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(str::to_string))
        .filter(|s| !s.is_empty())
        .collect()
}

fn undefined_symbols(so: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--undefined-only", &so.to_string_lossy()])
        .output()
        .expect("`nm` must be available");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(str::to_string))
        .filter(|s| !s.is_empty())
        .collect()
}

#[test]
fn every_c_symbol_is_exported_by_rust() {
    let (c_so, rust_so) = so_paths();
    let c_syms = defined_dynamic_symbols(&c_so);
    let rust_syms = defined_dynamic_symbols(&rust_so);

    // The public C ABI.
    assert!(
        c_syms.contains("driver"),
        "C .so unexpectedly does not export `driver`: {c_syms:?}"
    );

    let missing: Vec<&String> = c_syms
        .iter()
        // Ignore the linker/loader bookkeeping symbols that are not part of the
        // library's own API surface.
        .filter(|s| {
            !matches!(
                s.as_str(),
                "_init" | "_fini" | "__bss_start" | "_edata" | "_end"
            )
        })
        .filter(|s| !rust_syms.contains(*s))
        .collect();

    assert!(
        missing.is_empty(),
        "Rust .so is missing these C-exported symbols: {missing:?}\n\
         C   : {c_syms:?}\n\
         RUST: {rust_syms:?}"
    );
}

#[test]
fn rust_so_has_no_unresolvable_non_libc_symbols() {
    let (_c_so, rust_so) = so_paths();
    // The library dlopen'd successfully in `so_paths()`, which already proves
    // every undefined symbol resolves.  Assert the imports are only the libc /
    // runtime ones the C library also relies on.
    let undef = undefined_symbols(&rust_so);
    let unexpected: Vec<&String> = undef
        .iter()
        .filter(|s| {
            // libc / glibc / loader / unwinder symbols.
            !(s.starts_with("__")
                || s.starts_with("_ITM_")
                || s.starts_with("_Unwind_")
                || s.contains("@GLIBC")
                || matches!(
                    s.as_str(),
                    "printf"
                        | "setlocale"
                        | "memcpy"
                        | "memset"
                        | "memmove"
                        | "memcmp"
                        | "bcmp"
                        | "abort"
                        | "free"
                        | "malloc"
                        | "realloc"
                        | "calloc"
                        | "write"
                        | "writev"
                        | "getenv"
                        | "sysconf"
                        | "dl_iterate_phdr"
                        | "posix_memalign"
                        | "strlen"
                        | "GLIBC_2.2.5"
                        | "GLIBC_2.14"
                        | "GLIBC_2.17"
                        | "GLIBC_2.34"
                ))
        })
        .collect();
    assert!(
        unexpected.is_empty(),
        "Rust .so imports unexpected non-libc symbols: {unexpected:?}"
    );
}
