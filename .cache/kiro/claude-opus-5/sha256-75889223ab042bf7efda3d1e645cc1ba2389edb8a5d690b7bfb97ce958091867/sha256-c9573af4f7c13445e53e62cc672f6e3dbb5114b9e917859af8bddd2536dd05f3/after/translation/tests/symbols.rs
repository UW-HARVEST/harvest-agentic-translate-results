//! Phase D — exported-symbol parity between the C and Rust shared objects.
//!
//! This is the machine-checkable form of `SYMBOLS.md`: every dynamic symbol the
//! C `.so` defines must also be defined by the Rust `.so` under the exact same
//! name, and the Rust `.so` must not depend on any undefined symbol outside
//! libc / the compiler runtime.

mod common;

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

use common::*;

fn nm(path: &Path, extra: &str) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", extra])
        .arg(path)
        .output()
        .expect("`nm` must be on PATH");
    assert!(
        out.status.success(),
        "nm -D {extra} {} failed: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last())
        .filter(|s| !s.is_empty())
        // Strip glibc/GCC version tags so `memset@GLIBC_2.2.5` and `memset`
        // compare equal.
        .map(|s| s.split('@').next().unwrap().to_string())
        .collect()
}

/// Symbols every Rust cdylib pulls in from libc / libgcc / the CRT. Anything
/// undefined in the Rust `.so` that is not one of these (or one of the C `.so`'s
/// own imports) would mean a missing implementation.
fn is_runtime_import(s: &str) -> bool {
    const PREFIXES: &[&str] = &["_ITM_", "_Unwind_", "__cxa_", "__gmon_", "__tls_", "_dl_"];
    const NAMES: &[&str] = &[
        "abort", "bcmp", "calloc", "close", "dl_iterate_phdr", "free", "fstat", "fstat64", "getcwd",
        "getenv", "gettid", "lseek", "lseek64", "malloc", "memcmp", "memcpy", "memmove", "memset",
        "mmap", "mmap64", "munmap", "open", "open64", "posix_memalign", "read", "readlink",
        "realloc", "realpath", "sqrtf", "sqrt", "stat", "stat64", "statx", "strlen", "syscall",
        "write", "writev", "__errno_location", "pthread_key_create", "pthread_key_delete",
        "pthread_setspecific", "pthread_getspecific", "__cxa_thread_atexit_impl", "getauxval",
        "sysconf", "pthread_self", "pthread_getattr_np", "pthread_attr_getstack",
        "pthread_attr_destroy", "sigaltstack", "sigaction", "mprotect", "poll", "pipe2",
        "__libc_start_main",
    ];
    PREFIXES.iter().any(|p| s.starts_with(p)) || NAMES.contains(&s)
}

#[test]
fn symbols_rust_defines_everything_the_c_so_defines() {
    let l = libs();
    let c_def = nm(&l.c_path, "--defined-only");
    let r_def = nm(&l.r_path, "--defined-only");

    assert!(c_def.contains("normalize"), "C .so does not export `normalize`: {c_def:?}");

    let missing: Vec<&String> = c_def.difference(&r_def).collect();
    assert!(
        missing.is_empty(),
        "Rust .so ({}) is missing {} symbol(s) exported by the C .so ({}): {missing:?}\n\
         Per SYMBOLS.md/Phase A these must be exported (if the impl exists) or the \
         corresponding C source must be translated.",
        l.r_path.display(),
        missing.len(),
        l.c_path.display(),
    );
}

#[test]
fn symbols_rust_has_no_unresolved_non_runtime_imports() {
    let l = libs();
    let c_undef = nm(&l.c_path, "--undefined-only");
    let r_undef = nm(&l.r_path, "--undefined-only");

    let unexplained: Vec<&String> = r_undef
        .iter()
        .filter(|s| !is_runtime_import(s) && !c_undef.contains(*s))
        .collect();
    assert!(
        unexplained.is_empty(),
        "Rust .so has undefined non-libc symbols (missing implementations?): {unexplained:?}"
    );
}

/// The public surface really is a single function: guard against `SYMBOLS.md`
/// going stale if the C source grows another entry point.
#[test]
fn symbols_c_surface_is_still_just_normalize() {
    let l = libs();
    let c_def = nm(&l.c_path, "--defined-only");
    let interesting: Vec<&String> = c_def
        .iter()
        .filter(|s| !s.starts_with('_') && !is_runtime_import(s))
        .collect();
    assert_eq!(
        interesting,
        vec![&"normalize".to_string()],
        "the C .so's public surface changed; SYMBOLS.md, ERRORS.md and CONFIGS.md \
         must be regenerated"
    );
}
