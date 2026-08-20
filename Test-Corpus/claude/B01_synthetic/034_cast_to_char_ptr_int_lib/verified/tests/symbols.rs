//! Phase D — automated symbol-parity gate.
//!
//! Every symbol the C `.so` exports must also be exported by the Rust `.so`
//! under the exact same name, and the Rust `.so` must not have any undefined
//! symbol that is not provided by libc / the platform runtime.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

fn nm(path: &Path, extra: &str) -> Vec<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg(extra)
        .arg(path)
        .output()
        .unwrap_or_else(|e| panic!("failed to run nm on {}: {e}", path.display()));
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

/// Symbols that come from the C runtime / toolchain rather than from the library
/// being translated. `_ITM_*`, `__gmon_start__`, `__cxa_*` and `gettid` are weak
/// CRT hooks present in the C `.so` too.
fn is_platform_symbol(sym: &str) -> bool {
    let base = sym.split('@').next().unwrap_or(sym);
    const GLIBC: &[&str] = &[
        "printf", "putchar", "memcpy", "memmove", "memset", "bcmp", "memcmp", "strlen", "malloc",
        "calloc", "realloc", "free", "posix_memalign", "aligned_alloc", "abort", "write", "writev",
        "read", "close", "open", "open64", "lseek", "lseek64", "stat", "stat64", "fstat", "fstat64",
        "statx", "readlink", "realpath", "getcwd", "getenv", "mmap", "mmap64", "munmap", "mprotect",
        "syscall", "sysconf", "dl_iterate_phdr", "dlsym", "dladdr", "__errno_location",
        "__tls_get_addr", "__cxa_finalize", "__cxa_thread_atexit_impl", "__gmon_start__", "gettid",
        "pthread_key_create", "pthread_key_delete", "pthread_getspecific", "pthread_setspecific",
        "pthread_self", "pthread_mutex_lock", "pthread_mutex_unlock", "pthread_mutex_trylock",
        "pthread_mutex_destroy", "pthread_rwlock_rdlock", "pthread_rwlock_unlock",
        "pthread_rwlock_wrlock", "pthread_condattr_init", "pthread_condattr_setclock",
        "pthread_cond_init", "pthread_cond_destroy", "pthread_cond_signal",
        "pthread_cond_broadcast", "pthread_cond_wait", "pthread_cond_timedwait",
        "pthread_attr_init", "pthread_attr_destroy", "pthread_attr_setstacksize", "pthread_create",
        "pthread_join", "pthread_detach", "pthread_sigmask", "sigaction", "sigaltstack",
        "sigemptyset", "sigaddset", "raise", "poll", "nanosleep", "clock_gettime", "getrandom",
        "environ", "signal",
    ];
    GLIBC.contains(&base)
        || base.starts_with("_Unwind_")
        || base.starts_with("_ITM_")
        || base.starts_with("__libc_")
        || base.starts_with("__pthread_")
}

#[test]
fn phase_d_every_c_symbol_is_exported_by_rust() {
    let l = libs();
    let c_defined: BTreeSet<String> = nm(&l.c_path, "--defined-only").into_iter().collect();
    let rust_defined: BTreeSet<String> = nm(&l.rust_path, "--defined-only").into_iter().collect();

    assert!(
        c_defined.contains("driver"),
        "the C .so must export `driver`; found {c_defined:?}"
    );

    let missing: Vec<&String> = c_defined.difference(&rust_defined).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}\n\
         C   ({}) = {c_defined:?}\n\
         Rust({}) = {rust_defined:?}",
        c_defined.len(),
        rust_defined.len(),
    );
}

#[test]
fn phase_d_rust_has_no_unresolved_non_libc_symbols() {
    let l = libs();
    let undefined = nm(&l.rust_path, "--undefined-only");
    let unexpected: Vec<&String> = undefined
        .iter()
        .filter(|s| !is_platform_symbol(s))
        .collect();
    assert!(
        unexpected.is_empty(),
        "the Rust .so has undefined symbols that are not provided by libc / the \
         platform runtime: {unexpected:?}"
    );
}

#[test]
fn phase_d_internal_c_symbols_stay_internal() {
    let l = libs();
    let c_defined: BTreeSet<String> = nm(&l.c_path, "--defined-only").into_iter().collect();
    let rust_defined: BTreeSet<String> = nm(&l.rust_path, "--defined-only").into_iter().collect();
    // `print_hex` is `static` in the C: neither library may export it.
    assert!(!c_defined.contains("print_hex"));
    assert!(!rust_defined.contains("print_hex"));
    // The Rust side must not leak mangled Rust items either.
    let leaked: Vec<&String> = rust_defined
        .iter()
        .filter(|s| s.starts_with("_ZN") || s.starts_with("_R"))
        .collect();
    assert!(leaked.is_empty(), "mangled Rust symbols leaked: {leaked:?}");
}
