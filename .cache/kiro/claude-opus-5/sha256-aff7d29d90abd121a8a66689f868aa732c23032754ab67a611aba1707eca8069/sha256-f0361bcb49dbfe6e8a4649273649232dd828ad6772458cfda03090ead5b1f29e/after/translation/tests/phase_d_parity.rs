//! Phase D — symbol parity between the two shared objects, checked at runtime
//! through `dlsym` and statically through `nm -D`.

mod common;

use common::{EXPORTED_SYMBOLS, Impl, exports};

use std::path::{Path, PathBuf};
use std::process::Command;

fn c_so() -> PathBuf {
    common::c_so_path()
}

fn rust_so() -> PathBuf {
    common::rust_so_path()
}

/// Global `T`/`D`/`B` symbols from `nm -D --defined-only`, sorted.
fn defined_dynamic_symbols(so: &Path) -> Vec<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", "--format=posix"])
        .arg(so)
        .output()
        .expect("run nm -D (binutils must be installed)");
    assert!(
        out.status.success(),
        "nm failed on {so:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let mut syms: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let name = it.next()?;
            let kind = it.next()?;
            // Global text/data/bss only; skip weak (w/V/v) and local symbols.
            if matches!(kind, "T" | "D" | "B" | "R") {
                Some(name.to_string())
            } else {
                None
            }
        })
        .collect();
    syms.sort();
    syms.dedup();
    syms
}

/// The Rust `.so` must export every symbol the C `.so` exports.
#[test]
fn nm_symbol_diff_is_empty() {
    let c = defined_dynamic_symbols(&c_so());
    let rs = defined_dynamic_symbols(&rust_so());

    let missing: Vec<&String> = c.iter().filter(|s| !rs.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing {} symbol(s) exported by the C .so: {missing:?}\n\
         C exports:    {c:?}\n  Rust exports: {rs:?}",
        missing.len()
    );

    // Sanity: the C side really did export the five functions we test.
    for want in EXPORTED_SYMBOLS {
        assert!(
            c.contains(&want.to_string()),
            "C .so unexpectedly does not export `{want}`; C exports: {c:?}"
        );
    }
}

/// The C `.so` must not export anything we forgot to enumerate in SYMBOLS.md.
#[test]
fn c_exports_exactly_the_documented_set() {
    let c = defined_dynamic_symbols(&c_so());
    let mut want: Vec<String> = EXPORTED_SYMBOLS.iter().map(|s| s.to_string()).collect();
    want.sort();
    assert_eq!(
        c, want,
        "the C .so's exported-symbol set changed; SYMBOLS.md must be regenerated"
    );
}

/// The Rust `.so` must have no undefined symbols outside libc / the unwinder.
#[test]
fn rust_so_has_no_unresolved_non_libc_symbols() {
    let out = Command::new("nm")
        .args(["-D", "--undefined-only", "--format=posix"])
        .arg(rust_so())
        .output()
        .expect("run nm -D --undefined-only");
    assert!(out.status.success(), "nm failed");

    let text = String::from_utf8_lossy(&out.stdout);
    let mut suspicious = Vec::new();
    for line in text.lines() {
        let name = match line.split_whitespace().next() {
            Some(n) => n,
            None => continue,
        };
        let base = name.split('@').next().unwrap_or(name);
        let is_known = base.starts_with('_')            // _Unwind_*, _ITM_*, __cxa_*, __tls_*, ...
            || KNOWN_LIBC.contains(&base);
        if !is_known {
            suspicious.push(base.to_string());
        }
    }
    assert!(
        suspicious.is_empty(),
        "Rust .so has unresolved non-libc symbols: {suspicious:?}"
    );
}

const KNOWN_LIBC: &[&str] = &[
    "abort",
    "bcmp",
    "calloc",
    "close",
    "dl_iterate_phdr",
    "free",
    "fstat",
    "fstat64",
    "getcwd",
    "getenv",
    "gettid",
    "lseek",
    "lseek64",
    "malloc",
    "memchr",
    "memcmp",
    "memcpy",
    "memmove",
    "memset",
    "mmap",
    "mmap64",
    "munmap",
    "open",
    "open64",
    "posix_memalign",
    "printf",
    "pthread_getspecific",
    "pthread_key_create",
    "pthread_key_delete",
    "pthread_setspecific",
    "pthread_mutex_lock",
    "pthread_mutex_trylock",
    "pthread_mutex_unlock",
    "pthread_self",
    "puts",
    "read",
    "readlink",
    "realloc",
    "realpath",
    "sigaction",
    "sigaltstack",
    "stat",
    "stat64",
    "statx",
    "strlen",
    "syscall",
    "sysconf",
    "write",
    "writev",
];

/// Every documented symbol must be resolvable via `dlsym` in *both* objects —
/// this is what an external caller actually does.
#[test]
fn dlsym_resolves_every_symbol_in_both() {
    for sym in EXPORTED_SYMBOLS {
        for which in [Impl::C, Impl::Rust] {
            assert!(
                exports(which, sym),
                "dlsym failed for `{sym}` in the {} .so",
                which.name()
            );
        }
    }
}
