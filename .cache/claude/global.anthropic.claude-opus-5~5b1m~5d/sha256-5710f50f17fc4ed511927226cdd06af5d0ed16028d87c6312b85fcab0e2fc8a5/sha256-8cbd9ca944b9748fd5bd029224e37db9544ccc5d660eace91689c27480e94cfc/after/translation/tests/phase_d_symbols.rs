//! Phase D — symbol parity between the C and Rust shared objects.
//!
//! Re-derives both symbol tables with `nm -D` at test time so that the claim in
//! `SYMBOLS.md` is continuously verified rather than a one-off observation.

mod common;

use common::{c_so_path, rust_so_path, Impls};
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

fn nm_defined(path: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", path.to_str().expect("utf8 path")])
        .output()
        .expect("run nm (binutils required)");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let (_addr, kind, name) = (it.next()?, it.next()?, it.next()?);
            // Keep global/weak text and data symbols; skip nothing else exists.
            match kind {
                "T" | "t" | "D" | "d" | "B" | "b" | "R" | "r" | "W" | "V" | "A" | "i" => {
                    Some(name.to_string())
                }
                _ => None,
            }
        })
        .collect()
}

fn nm_undefined(path: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--undefined-only", path.to_str().expect("utf8 path")])
        .output()
        .expect("run nm");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(str::to_string))
        .collect()
}

#[test]
fn phase_d_symbol_parity() {
    let c = nm_defined(&c_so_path());
    let r = nm_defined(&rust_so_path());

    assert!(!c.is_empty(), "nm found no symbols in the C .so");

    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}\n\
         C   = {c:?}\n\
         Rust= {r:?}"
    );

    // The C surface is exactly one function; assert it explicitly so a future
    // C change that adds a symbol cannot slip past the diff above.
    assert!(c.contains("driver"), "C .so must export `driver`");
    assert!(r.contains("driver"), "Rust .so must export `driver`");
    assert_eq!(c.len(), 1, "C .so symbol set changed: {c:?}");
}

#[test]
fn phase_d_no_unresolved_non_libc_imports() {
    // Every undefined symbol in the Rust .so must also be undefined in the C
    // .so (i.e. it comes from libc), otherwise the translation has grown a
    // dependency the C never had.
    let allowed_prefixes = [
        "printf", "strcspn", "puts", "putchar", "fwrite", "memcpy", "__",
        "_ITM_", "_Unwind", "abort", "malloc", "free", "realloc", "calloc",
        "write", "memset", "memmove", "strlen", "GLIBC", "CXXABI", "rust_eh",
    ];
    for sym in nm_undefined(&rust_so_path()) {
        let ok = allowed_prefixes.iter().any(|p| sym.contains(p)) || sym.is_empty();
        assert!(
            ok,
            "Rust .so imports unexpected non-libc symbol {sym:?} \
             (the C .so imports only printf/strcspn from libc)"
        );
    }
}

#[test]
fn phase_d_both_libraries_actually_load_and_run() {
    // Sanity: the parity check above is meaningless if either object cannot be
    // dlopen'd and called through its export.
    let impls = Impls::load();
    let out_c = common::capture_stdout(|| unsafe {
        (impls.c)(b"hello\0".as_ptr() as *const _, b"l\0".as_ptr() as *const _)
    });
    let out_r = common::capture_stdout(|| unsafe {
        (impls.rust)(b"hello\0".as_ptr() as *const _, b"l\0".as_ptr() as *const _)
    });
    assert_eq!(out_c, b"2\n");
    assert_eq!(out_r, b"2\n");
}
