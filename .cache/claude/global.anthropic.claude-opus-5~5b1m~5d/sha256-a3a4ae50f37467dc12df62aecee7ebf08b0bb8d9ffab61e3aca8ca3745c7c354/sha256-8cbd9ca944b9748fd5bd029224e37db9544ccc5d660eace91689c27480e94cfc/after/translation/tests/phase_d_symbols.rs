//! Phase D — symbol parity enforced as a test.
//!
//! Every symbol the C `.so` exports must also be exported by the Rust `.so`
//! with the exact same name. The C statics must remain unexported.

mod common;
use common::*;

use std::collections::BTreeSet;
use std::process::Command;

/// Defined, global symbols from `nm -D --defined-only`.
fn defined_symbols(path: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(path)
        .output()
        .expect("`nm` must be available on PATH");
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
            let a = it.next()?;
            let (kind, name) = match it.next() {
                Some(k) => (k, it.next()?),
                None => return None,
            };
            let _ = a;
            // keep only global/defined code+data symbols
            if matches!(kind, "T" | "D" | "B" | "R" | "G" | "S") {
                Some(name.to_string())
            } else {
                None
            }
        })
        .collect()
}

#[test]
fn symbols_c_subset_of_rust() {
    let c = defined_symbols(c_so_path());
    let rust = defined_symbols(rust_so_path());

    assert!(
        c.contains("colourblind"),
        "the C .so must export `colourblind`; got {c:?}"
    );

    let missing: Vec<&String> = c.difference(&rust).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}\n\
         C:    {c:?}\n\
         Rust: {rust:?}"
    );
}

#[test]
fn c_statics_stay_unexported_in_rust() {
    let rust = defined_symbols(rust_so_path());
    for s in ["Protanopia", "Deuteranopia", "Tritanopia", "protanopia", "deuteranopia", "tritanopia"] {
        assert!(
            !rust.contains(s),
            "`{s}` is `static` in the C source and must NOT be a dynamic symbol in Rust"
        );
    }
}

#[test]
fn rust_has_no_unresolved_project_symbols() {
    let out = Command::new("nm")
        .args(["-D", "-u"])
        .arg(rust_so_path())
        .output()
        .expect("nm");
    let text = String::from_utf8_lossy(&out.stdout);
    let offenders: Vec<&str> = text
        .lines()
        .filter_map(|l| l.split_whitespace().last())
        .filter(|name| {
            // Everything Rust legitimately imports: libc, glibc-versioned, the
            // unwinder, TLS/pthread helpers, and toolchain weak symbols.
            !(name.contains('@')
                || name.starts_with("_Unwind_")
                || name.starts_with("__")
                || name.starts_with("_ITM_")
                || matches!(
                    *name,
                    "abort" | "bcmp" | "calloc" | "close" | "dl_iterate_phdr" | "free"
                        | "getcwd" | "getenv" | "gettid" | "malloc" | "memcpy" | "memmove"
                        | "memset" | "mmap" | "munmap" | "open" | "posix_memalign" | "read"
                        | "readlink" | "realloc" | "realpath" | "statx" | "strlen"
                        | "syscall" | "write" | "writev" | "lseek" | "fstat" | "stat"
                        | "pthread_key_create" | "pthread_key_delete"
                        | "pthread_setspecific"
                ))
        })
        .collect();
    assert!(
        offenders.is_empty(),
        "Rust .so has unresolved non-libc symbols: {offenders:?}"
    );
}
