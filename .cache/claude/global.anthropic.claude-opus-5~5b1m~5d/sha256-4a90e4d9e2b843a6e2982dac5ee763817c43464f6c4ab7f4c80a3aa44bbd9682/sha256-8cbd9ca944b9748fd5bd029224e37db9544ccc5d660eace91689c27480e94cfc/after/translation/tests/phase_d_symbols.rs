//! Phase D -- symbol parity between the C `.so` and the Rust `.so`.
//!
//! Enforces the `SYMBOLS.md` gate mechanically: every dynamic symbol the C
//! shared object defines must also be defined by the Rust shared object, with
//! the exact same name, and must be resolvable via `dlsym`.

mod common;

use common::*;
use std::path::PathBuf;
use std::process::Command;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn defined_dynamic_symbols(so: &PathBuf) -> Vec<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(so)
        .output()
        .expect("run nm");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(str::to_string))
        .filter(|s| !s.starts_with('_')) // skip CRT/ITM/compiler internals
        .collect();
    v.sort();
    v.dedup();
    v
}

fn c_so() -> PathBuf {
    let p = manifest_dir()
        .parent()
        .unwrap()
        .join("c_src/build/libdriver.so");
    // Force the harness to build it if needed.
    let _ = sym_char(Impl::C, "driver");
    assert!(p.exists(), "missing {}", p.display());
    p
}

fn rust_so() -> PathBuf {
    let p = common::rust_so_path();
    assert!(p.exists(), "missing {}", p.display());
    p
}

/// The symbol diff (C-defined minus Rust-defined) MUST be empty.
#[test]
fn symbol_diff_is_empty() {
    let c = defined_dynamic_symbols(&c_so());
    let r = defined_dynamic_symbols(&rust_so());
    let missing: Vec<&String> = c.iter().filter(|s| !r.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing {} symbol(s) exported by the C .so: {missing:?}\n  C   = {c:?}\n  Rust= {r:?}",
        missing.len()
    );
    // Sanity: the two documented symbols really are there.
    for name in SYMBOLS {
        assert!(c.contains(&name.to_string()), "C .so lost `{name}`");
        assert!(r.contains(&name.to_string()), "Rust .so lost `{name}`");
    }
}

/// Every documented symbol resolves via `dlsym` in BOTH libraries.
#[test]
fn every_symbol_is_dlsym_resolvable() {
    for name in SYMBOLS {
        assert!(has_symbol(Impl::C, name), "C .so: dlsym({name}) failed");
        assert!(has_symbol(Impl::Rust, name), "Rust .so: dlsym({name}) failed");
    }
}

/// The Rust `.so` must not leave any non-libc symbol undefined.
#[test]
fn rust_has_no_unresolved_non_libc_symbols() {
    let out = Command::new("nm")
        .args(["-D", "--undefined-only"])
        .arg(rust_so())
        .output()
        .expect("run nm");
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    let bad: Vec<&str> = text
        .lines()
        .filter_map(|l| l.split_whitespace().last())
        .filter(|s| {
            // Everything the Rust std runtime legitimately imports from
            // libc / libgcc / the dynamic loader.
            let s = s.split('@').next().unwrap_or(s);
            !(s.starts_with('_')
                || KNOWN_LIBC.contains(&s))
        })
        .collect();
    assert!(
        bad.is_empty(),
        "Rust .so has unresolved non-libc symbols: {bad:?}"
    );
}

const KNOWN_LIBC: &[&str] = &[
    "abort", "bcmp", "calloc", "close", "dl_iterate_phdr", "free", "fstat", "fstat64", "getcwd",
    "getenv", "gettid", "lseek", "lseek64", "malloc", "memcmp", "memcpy", "memmove", "memset",
    "mmap", "mmap64", "munmap", "open", "open64", "posix_memalign", "printf", "pthread_key_create",
    "pthread_key_delete", "pthread_getspecific", "pthread_setspecific", "read", "readlink",
    "realloc", "realpath", "stat", "stat64", "statx", "strlen", "syscall", "write", "writev",
    "sysconf", "pthread_self", "pthread_mutex_lock", "pthread_mutex_unlock", "pthread_rwlock_rdlock",
    "pthread_rwlock_unlock", "getrandom", "poll", "sigaltstack", "sigaction", "mprotect", "madvise",
    "environ", "qsort", "strerror_r", "signal", "raise", "exit",
];
