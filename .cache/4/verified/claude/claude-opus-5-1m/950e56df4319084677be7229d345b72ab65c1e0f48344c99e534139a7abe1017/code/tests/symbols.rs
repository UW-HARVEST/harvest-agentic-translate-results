//! Phase D -- exported-symbol parity between the C and the Rust `.so`.
//!
//! Every symbol the C `.so` exports must be exported by the Rust `.so` under the
//! exact same name; the diff must be empty. Also checks that the Rust `.so` has
//! no undefined symbol outside libc / the toolchain runtime.

mod harness;

use harness::*;
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

fn nm(path: &Path, extra: &str) -> Vec<String> {
    let out = Command::new("nm")
        .args(["-D", extra])
        .arg(path)
        .output()
        .unwrap_or_else(|e| panic!("running nm on {}: {e}", path.display()));
    assert!(
        out.status.success(),
        "nm -D {extra} {} failed: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .collect()
}

fn defined(path: &Path) -> BTreeSet<String> {
    nm(path, "--defined-only").into_iter().collect()
}

fn undefined(path: &Path) -> BTreeSet<String> {
    nm(path, "--undefined-only").into_iter().collect()
}

/// Symbols glibc / libgcc / the Rust runtime legitimately imports.
fn is_toolchain_symbol(name: &str) -> bool {
    let base = name.split('@').next().unwrap_or(name);
    const KNOWN: &[&str] = &[
        "_ITM_deregisterTMCloneTable",
        "_ITM_registerTMCloneTable",
        "__cxa_finalize",
        "__cxa_thread_atexit_impl",
        "__errno_location",
        "__gmon_start__",
        "__tls_get_addr",
        "abort",
        "bcmp",
        "calloc",
        "close",
        "dl_iterate_phdr",
        "free",
        "fstat64",
        "getcwd",
        "getenv",
        "gettid",
        "lseek64",
        "malloc",
        "memcmp",
        "memcpy",
        "memmove",
        "memset",
        "mmap64",
        "munmap",
        "open64",
        "posix_memalign",
        "printf",
        "pthread_key_create",
        "pthread_key_delete",
        "pthread_getspecific",
        "pthread_setspecific",
        "putchar",
        "read",
        "readlink",
        "realloc",
        "realpath",
        "stat64",
        "statx",
        "strlen",
        "syscall",
        "write",
        "writev",
    ];
    base.starts_with("_Unwind_") || base.starts_with("__libc_") || KNOWN.contains(&base)
}

#[test]
fn exported_symbol_diff_is_empty() {
    let i = impls();
    let c = defined(&i.c_path);
    let rust = defined(&i.rust_path);

    assert!(
        c.contains("driver"),
        "the C .so must export `driver`, found: {c:?}"
    );

    let missing: Vec<&String> = c.difference(&rust).collect();
    assert!(
        missing.is_empty(),
        "the Rust .so is missing {} symbol(s) exported by the C .so: {missing:?}\n\
         C exports:    {c:?}\n\
         Rust exports: {rust:?}",
        missing.len()
    );

    println!("C exports ({}): {c:?}", c.len());
    println!("Rust exports ({}): {rust:?}", rust.len());
}

#[test]
fn rust_so_has_no_non_libc_undefined_symbols() {
    let i = impls();
    let stray: Vec<String> = undefined(&i.rust_path)
        .into_iter()
        .filter(|s| !is_toolchain_symbol(s))
        .collect();
    assert!(
        stray.is_empty(),
        "the Rust .so imports {} symbol(s) that are not libc/toolchain: {stray:?}",
        stray.len()
    );
}

#[test]
fn static_helper_is_not_exported_by_either_library() {
    let i = impls();
    for (label, path) in [("C", &i.c_path), ("Rust", &i.rust_path)] {
        assert!(
            !defined(path).contains("print_hex"),
            "{label} .so exports `print_hex`, which is `static` in the C source"
        );
    }
}
