//! Phase D — symbol parity between the C `.so` and the Rust `.so`.
//!
//! Enforces mechanically what `SYMBOLS.md` records, so the parity claim cannot
//! silently rot.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::process::Command;

fn nm(path: &std::path::Path, extra: &str) -> BTreeSet<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg(extra)
        .arg(path)
        .output()
        .expect("run nm");
    assert!(
        out.status.success(),
        "nm {extra} {path:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last())
        .map(|s| s.split('@').next().unwrap_or(s).to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Every symbol the C `.so` exports must be exported by the Rust `.so` too,
/// with the exact same name. The diff must be empty.
#[test]
fn d_exported_symbol_diff_is_empty() {
    let l = libs();
    let c_syms = nm(&l.c_path, "--defined-only");
    let r_syms = nm(&l.r_path, "--defined-only");

    assert!(
        !c_syms.is_empty(),
        "nm found no exported symbols in the C library at {:?}",
        l.c_path
    );

    let missing: Vec<&String> = c_syms.difference(&r_syms).collect();
    assert!(
        missing.is_empty(),
        "the Rust .so is missing {} symbol(s) exported by the C .so: {missing:?}\n\
         C exports:    {c_syms:?}\n\
         Rust exports: {r_syms:?}",
        missing.len()
    );

    // The known surface, so a newly added C symbol is noticed rather than
    // silently accepted.
    assert!(
        c_syms.contains("my_pow"),
        "C .so should export my_pow, exports: {c_syms:?}"
    );
    assert_eq!(
        c_syms.len(),
        1,
        "the C .so's exported surface changed (expected only `my_pow`): \
         {c_syms:?} -- re-derive SYMBOLS.md / ERRORS.md / CONFIGS.md"
    );
}

/// The Rust `.so` must not leave any non-libc symbol unresolved.
#[test]
fn d_no_unresolved_non_libc_symbols() {
    let l = libs();
    let r_undef = nm(&l.r_path, "--undefined-only");

    // Everything the Rust runtime legitimately imports from libc / libm /
    // libgcc / the dynamic loader.
    let allowed_prefixes = [
        "_ITM_", "__cxa_", "__gmon_", "_Unwind_", "__tls_get_addr",
        "__errno_location", "pthread_", "statx", "gettid", "__libc_",
        "__rust_", "_dl_",
    ];
    let allowed_exact: BTreeSet<&str> = [
        "abort", "bcmp", "calloc", "close", "dl_iterate_phdr", "fprintf",
        "free", "fstat64", "fstat", "getcwd", "getenv", "lseek64", "lseek",
        "malloc", "memcpy", "memmove", "memset", "mmap64", "mmap", "munmap",
        "open64", "open", "posix_memalign", "pow", "read", "readlink",
        "realloc", "realpath", "stat64", "stat", "stderr", "stdout", "strlen",
        "syscall", "write", "writev", "memcmp", "sysconf", "sigaltstack",
        "mprotect", "pipe2", "poll", "fwrite", "fputs", "fflush", "exit",
        "raise", "signal", "sigaction", "sigemptyset", "sigaddset",
        "pthread_self", "environ", "strerror_r", "abs", "qsort", "getpid",
    ]
    .into_iter()
    .collect();

    let unexpected: Vec<&String> = r_undef
        .iter()
        .filter(|s| {
            !allowed_exact.contains(s.as_str())
                && !allowed_prefixes.iter().any(|p| s.starts_with(p))
        })
        .collect();

    assert!(
        unexpected.is_empty(),
        "the Rust .so has {} unresolved symbol(s) that are not libc/libm/\
         libgcc/loader imports: {unexpected:?}\nfull undefined set: {r_undef:?}",
        unexpected.len()
    );
}

/// Both libraries must import the SAME `pow` and the same `errno` accessor, so
/// the numeric results and `errno` side effects are identical by construction
/// rather than by luck.
#[test]
fn d_both_import_the_same_libm_pow_and_errno() {
    let l = libs();
    let c_undef = nm(&l.c_path, "--undefined-only");
    let r_undef = nm(&l.r_path, "--undefined-only");

    for sym in ["pow", "__errno_location", "fprintf", "stderr"] {
        assert!(
            c_undef.contains(sym),
            "expected the C .so to import {sym}, imports: {c_undef:?}"
        );
        assert!(
            r_undef.contains(sym),
            "the Rust .so does not import {sym}; it must use the same libc/libm \
             entry point as the C to stay bit-identical. imports: {r_undef:?}"
        );
    }
}

/// The exported symbol must be reachable and callable through `dlsym` on both
/// handles, i.e. the `#[no_mangle] extern "C"` wrapper really is the ABI entry
/// point an external C caller would bind to.
#[test]
fn d_exported_symbol_is_callable_via_dlsym() {
    let l = libs();
    let _q = quiet();
    let c = unsafe { (l.c_pow)(2.0, 10.0) };
    let r = unsafe { (l.r_pow)(2.0, 10.0) };
    assert_eq!(c, 1024.0, "C my_pow(2,10) should be 1024");
    assert_eq!(r.to_bits(), c.to_bits(), "Rust my_pow(2,10) should match C");
}
