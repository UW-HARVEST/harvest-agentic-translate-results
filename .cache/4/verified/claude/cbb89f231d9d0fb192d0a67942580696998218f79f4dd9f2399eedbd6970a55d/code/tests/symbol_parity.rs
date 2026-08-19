//! Phase D -- symbol parity between the C `.so` and the Rust `.so`.
//!
//! Re-derives both symbol sets with `nm -D` and asserts that every symbol the C
//! shared library exports is also exported, under the exact same name, by the
//! Rust shared library.

mod common;

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

fn nm_defined(lib: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(lib)
        .output()
        .expect("run nm");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        lib.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .collect()
}

fn nm_undefined(lib: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--undefined-only")
        .arg(lib)
        .output()
        .expect("run nm");
    assert!(out.status.success(), "nm failed on {}", lib.display());
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .collect()
}

/// Symbols emitted by every toolchain's crt/ITM glue rather than by the sources
/// under test.
fn is_toolchain_glue(name: &str) -> bool {
    matches!(
        name,
        "_ITM_deregisterTMCloneTable"
            | "_ITM_registerTMCloneTable"
            | "__cxa_finalize"
            | "__cxa_finalize@GLIBC_2.2.5"
            | "__gmon_start__"
            | "_init"
            | "_fini"
            | "__bss_start"
            | "_edata"
            | "_end"
    )
}

#[test]
fn test_c_exports_are_all_present_in_rust() {
    let c = common::c_shared_lib();
    let r = common::rust_shared_lib();
    let c_syms = nm_defined(&c);
    let r_syms = nm_defined(&r);

    let interesting: BTreeSet<&String> = c_syms
        .iter()
        .filter(|s| !is_toolchain_glue(s.split('@').next().unwrap_or(s)))
        .collect();

    // The C translation unit exports exactly `driver` and `main`
    // (`print_hex` is static).
    let names: Vec<&str> = interesting.iter().map(|s| s.as_str()).collect();
    assert!(
        names.contains(&"driver"),
        "C .so unexpectedly does not export `driver`: {names:?}"
    );
    assert!(
        names.contains(&"main"),
        "C .so unexpectedly does not export `main`: {names:?}"
    );

    let missing: Vec<&&String> = interesting.iter().filter(|s| !r_syms.contains(**s)).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by {} but missing from {}: {:?}",
        c.display(),
        r.display(),
        missing
    );
    eprintln!(
        "symbol parity ok: C exports {:?}, all present in the Rust .so",
        names
    );
}

#[test]
fn test_rust_so_has_no_unresolved_non_libc_symbols() {
    let r = common::rust_shared_lib();
    let undef = nm_undefined(&r);
    // Everything the Rust cdylib imports must come from libc / libgcc / ld.so.
    let allowed_prefixes = [
        "__", "_ITM_", "_Unwind_", "pthread_", "abort", "memcpy", "memmove", "memset", "memcmp",
        "malloc", "free", "calloc", "realloc", "posix_memalign", "write", "writev", "read",
        "close", "open", "open64", "fstat", "lseek", "lseek64", "poll", "getenv", "getcwd", "dl_iterate_phdr",
        "sysconf", "mmap", "mmap64", "munmap", "mprotect", "sigaction", "sigaltstack", "sigemptyset",
        "sigaddset", "syscall", "gettid", "getpid", "strlen", "strerror_r", "bcmp", "environ",
        "stat", "stat64", "statx", "readlink", "unlink", "mkdir", "rmdir", "rename", "opendir",
        "readdir64", "closedir", "fcntl", "dup", "dup2", "dup3", "pipe2", "fork", "execvp",
        "waitpid", "kill", "nanosleep", "clock_gettime", "sched_yield", "sched_getaffinity",
        "isatty", "copy_file_range", "pread64", "pwrite64", "ftruncate64", "fdatasync", "fsync",
        "linkat", "symlinkat", "unlinkat", "mkdirat", "openat64", "renameat", "fchmod", "fchown",
        "utimensat", "futimens", "getrandom", "arc4random_buf", "eventfd", "epoll_create1",
        "signal", "raise", "exit", "atexit", "realpath", "getuid", "geteuid", "getgid", "getegid",
    ];
    let unexpected: Vec<&String> = undef
        .iter()
        .filter(|s| {
            let base = s.split('@').next().unwrap_or(s);
            !allowed_prefixes
                .iter()
                .any(|p| base.starts_with(p) || base == *p)
        })
        .collect();
    assert!(
        unexpected.is_empty(),
        "Rust .so has unresolved non-libc symbols: {unexpected:?}"
    );
}

#[test]
fn test_c_so_imports_are_all_libc() {
    // Documents the C side of SYMBOLS.md: the C .so imports only libc.
    let c = common::c_shared_lib();
    let undef = nm_undefined(&c);
    for s in &undef {
        let base = s.split('@').next().unwrap_or(s);
        assert!(
            base.starts_with("__")
                || base.starts_with("_ITM_")
                || matches!(base, "printf" | "putchar" | "puts" | "scanf" | "memcpy"),
            "unexpected C import: {s}"
        );
    }
}
