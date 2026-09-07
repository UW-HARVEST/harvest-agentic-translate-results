//! Phase D — symbol parity enforced as a test, so regressions in the export
//! surface fail the suite.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

/// Global, defined symbols from `nm -D --defined-only`, excluding the
/// toolchain/runtime noise that is not part of the library's own API.
fn defined_symbols(so: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(so)
        .output()
        .expect("failed to run nm");
    assert!(out.status.success(), "nm failed on {so:?}");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let _addr = it.next()?;
            let kind = it.next()?;
            let name = it.next()?;
            // Only text/data/bss globals; skip Rust's own runtime exports.
            if !matches!(kind, "T" | "D" | "B" | "R") {
                return None;
            }
            if name.starts_with("_ZN")
                || name.starts_with("rust_")
                || name.starts_with("__rust")
                || name.starts_with("_R")
            {
                return None;
            }
            Some(name.to_string())
        })
        .collect()
}

#[test]
fn symbols_c_so_exports_expected_set() {
    let c = defined_symbols(&c_so_path());
    let expected: BTreeSet<String> = ["driver", "foo"].iter().map(|s| s.to_string()).collect();
    assert_eq!(
        c, expected,
        "the C .so's export surface changed; SYMBOLS.md must be regenerated"
    );
}

#[test]
fn symbols_rust_so_covers_every_c_symbol() {
    let c = defined_symbols(&c_so_path());
    let r = defined_symbols(&rust_so_path());
    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}\n\
         C:    {c:?}\n\
         Rust: {r:?}"
    );
}

#[test]
fn symbols_rust_so_has_no_undefined_non_libc_symbols() {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--undefined-only")
        .arg(rust_so_path())
        .output()
        .expect("failed to run nm");
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    let unresolved: Vec<String> = text
        .lines()
        .filter_map(|l| l.split_whitespace().last())
        // Strip the ELF symbol-version suffix (`printf@GLIBC_2.2.5`).
        .map(|n| n.split('@').next().unwrap_or(n).to_string())
        .filter(|name| {
            // libc, libgcc unwinder and ld.so internals: the C .so depends on
            // the same class of imports (`printf`, `strchr`).
            !(name.starts_with("__")
                || name.starts_with("_ITM_")
                || name.starts_with("_Unwind_")
                || name.starts_with("_ZN")
                || name.starts_with("_R")
                || KNOWN_LIBC.contains(&name.as_str()))
        })
        .collect();
    assert!(
        unresolved.is_empty(),
        "unexpected undefined symbols in the Rust .so: {unresolved:?}"
    );
}

/// Imports that glibc provides; the C `.so` relies on the same mechanism.
const KNOWN_LIBC: &[&str] = &[
    "printf", "strchr", "strlen", "memcpy", "memmove", "memset", "memcmp", "bcmp", "puts",
    "putchar", "abort", "malloc", "free", "realloc", "calloc", "posix_memalign", "write",
    "writev", "read", "close", "open64", "lseek64", "fstat64", "stat64", "statx", "readlink",
    "realpath", "getcwd", "getenv", "mmap64", "munmap", "syscall", "sysconf", "gettid",
    "dl_iterate_phdr", "pthread_key_create", "pthread_key_delete", "pthread_setspecific",
    "pthread_getspecific", "pthread_mutex_lock", "pthread_mutex_unlock",
];

/// Stronger, name-independent check: ask the dynamic loader to resolve every
/// relocation. This is what actually proves nothing is missing.
#[test]
fn symbols_rust_so_fully_resolves_at_load_time() {
    for so in [c_so_path(), rust_so_path()] {
        let out = Command::new("ldd").arg("-r").arg(&so).output();
        let Ok(out) = out else { continue }; // ldd unavailable: skip
        let combined = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        let bad: Vec<&str> = combined
            .lines()
            .filter(|l| l.contains("undefined symbol") || l.contains("not found"))
            .collect();
        assert!(
            bad.is_empty(),
            "`ldd -r {so:?}` reports unresolved relocations: {bad:?}"
        );
    }
}
