//! Phase D — symbol parity between the C `.so` and the Rust `.so`.
//!
//! Re-derives both symbol tables with `nm -D` at test time so the check cannot
//! go stale, and asserts the diff (C-defined symbols missing from Rust) is
//! empty. Also asserts the Rust `.so` imports nothing outside the libc /
//! platform-runtime allowlist.

mod common;

use std::collections::BTreeSet;
use std::process::Command;

fn nm(path: &std::path::Path, extra: &str) -> Vec<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg(extra)
        .arg(path)
        .output()
        .expect("run nm");
    assert!(
        out.status.success(),
        "nm -D {extra} {} failed: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .filter(|s| !s.is_empty())
        .collect()
}

fn defined(path: &std::path::Path) -> BTreeSet<String> {
    nm(path, "--defined-only").into_iter().collect()
}

fn undefined(path: &std::path::Path) -> BTreeSet<String> {
    nm(path, "--undefined-only").into_iter().collect()
}

#[test]
fn d1_every_c_symbol_is_exported_by_rust() {
    let l = common::libs();
    let c = defined(&l.c_path);
    let r = defined(&l.rust_path);

    assert!(
        c.contains("div_euclid"),
        "the C .so does not define div_euclid; symbol extraction is broken: {c:?}"
    );

    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "symbols defined by the C .so but MISSING from the Rust .so: {missing:?}\n\
         C  ({}): {c:?}\nRust ({}): {r:?}",
        l.c_path.display(),
        l.rust_path.display()
    );
}

#[test]
fn d2_rust_so_has_no_unresolved_non_libc_symbols() {
    let l = common::libs();
    let undef = undefined(&l.rust_path);

    // libc / platform runtime imports are expected from the Rust std runtime.
    let allow_exact: &[&str] = &[
        "_ITM_deregisterTMCloneTable",
        "_ITM_registerTMCloneTable",
        "__gmon_start__",
    ];
    let allow_prefix: &[&str] = &["_Unwind_", "__libc_", "__cxa_", "__tls_get_addr", "__errno"];
    let libc_names: &[&str] = &[
        "abort", "bcmp", "calloc", "close", "dl_iterate_phdr", "free", "fstat", "fstat64",
        "getcwd", "getenv", "gettid", "lseek", "lseek64", "malloc", "memcmp", "memcpy", "memmove",
        "memset", "mmap", "mmap64", "munmap", "open", "open64", "posix_memalign",
        "pthread_getspecific", "pthread_key_create", "pthread_key_delete", "pthread_mutex_lock",
        "pthread_mutex_trylock", "pthread_mutex_unlock", "pthread_rwlock_rdlock",
        "pthread_rwlock_unlock", "pthread_self", "pthread_setspecific", "read", "readlink",
        "realloc", "realpath", "sigaction", "sigaltstack", "stat", "stat64", "statx", "strlen",
        "sysconf", "syscall", "write", "writev", "__errno_location", "__cxa_thread_atexit_impl",
        "__cxa_finalize", "qsort", "poll", "pipe2", "getrandom",
    ];

    let bad: Vec<&String> = undef
        .iter()
        .filter(|s| {
            let base = s.split('@').next().unwrap_or(s);
            !(allow_exact.contains(&base)
                || libc_names.contains(&base)
                || allow_prefix.iter().any(|p| base.starts_with(p)))
        })
        .collect();

    assert!(
        bad.is_empty(),
        "Rust .so imports non-libc symbols (translation incomplete / external dependency): {bad:?}"
    );
}

#[test]
fn d3_symbol_tables_are_reported() {
    // Informational: printed with `--nocapture`, and keeps SYMBOLS.md honest.
    let l = common::libs();
    println!("C   .so: {}", l.c_path.display());
    for s in defined(&l.c_path) {
        println!("  C defined: {s}");
    }
    println!("Rust .so: {}", l.rust_path.display());
    for s in defined(&l.rust_path) {
        println!("  R defined: {s}");
    }
}
