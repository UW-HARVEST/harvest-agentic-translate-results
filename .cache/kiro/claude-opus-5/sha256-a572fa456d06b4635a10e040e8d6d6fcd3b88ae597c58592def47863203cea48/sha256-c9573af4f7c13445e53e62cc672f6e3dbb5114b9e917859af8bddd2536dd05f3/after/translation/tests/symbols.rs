//! Phase D — symbol parity between the C `.so` and the Rust `.so`.

mod common;

use std::process::Command;

fn nm_defined(path: &std::path::Path) -> Vec<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", "--format=posix"])
        .arg(path)
        .output()
        .expect("nm must be available");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().next().map(|s| s.to_string()))
        .filter(|s| !s.starts_with('_')) // skip toolchain/CRT internals
        .collect();
    v.sort();
    v.dedup();
    v
}

fn nm_undefined(path: &std::path::Path) -> Vec<String> {
    let out = Command::new("nm")
        .args(["-D", "--undefined-only", "--format=posix"])
        .arg(path)
        .output()
        .expect("nm must be available");
    let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().next().map(|s| s.to_string()))
        .collect();
    v.sort();
    v.dedup();
    v
}

/// Anything that legitimately comes from libc / libgcc-unwind / the dynamic
/// loader rather than from an untranslated C module.
fn is_platform_symbol(s: &str) -> bool {
    let name = s.split('@').next().unwrap_or(s);
    if name.starts_with('_') {
        // _Unwind_*, __cxa_*, __errno_location, __tls_get_addr, _ITM_*, ...
        return true;
    }
    const LIBC: &[&str] = &[
        "abort", "bcmp", "calloc", "close", "dl_iterate_phdr", "free", "fstat64", "getcwd",
        "getenv", "gettid", "lseek64", "malloc", "memcmp", "memcpy", "memmove", "memset",
        "mmap64", "munmap", "open64", "posix_memalign", "pthread_key_create",
        "pthread_key_delete", "pthread_getspecific", "pthread_setspecific", "read", "readlink",
        "realloc", "realpath", "stat64", "statx", "strlen", "syscall", "write", "writev",
        "sysconf", "getpid", "pthread_self", "pthread_mutex_lock", "pthread_mutex_unlock",
        "pthread_mutex_trylock", "pthread_mutex_destroy", "pthread_rwlock_rdlock",
        "pthread_rwlock_unlock", "pthread_rwlock_wrlock", "pthread_condattr_init",
        "pthread_condattr_setclock", "pthread_cond_timedwait", "pthread_cond_signal",
        "pthread_cond_broadcast", "pthread_cond_destroy", "pthread_cond_wait",
        "pthread_condattr_destroy", "pthread_attr_init", "pthread_attr_destroy", "sigaltstack",
        "mprotect", "sigaction", "sigaddset", "sigemptyset", "signal", "poll", "environ",
    ];
    LIBC.contains(&name)
}

#[test]
fn c_and_rust_export_the_same_symbols() {
    let c_path = common::c_so_path();
    let r_path = common::rust_so_path();
    let c = nm_defined(&c_path);
    let r = nm_defined(&r_path);

    let missing: Vec<&String> = c.iter().filter(|s| !r.contains(s)).collect();
    let extra: Vec<&String> = r.iter().filter(|s| !c.contains(s)).collect();

    eprintln!("C   ({}) exports: {:?}", c_path.display(), c);
    eprintln!("RUST({}) exports: {:?}", r_path.display(), r);

    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}"
    );
    assert!(
        !c.is_empty(),
        "sanity: the C .so must export at least one symbol"
    );
    // Not a hard failure requirement in the task, but the diff happens to be
    // empty in both directions here, so keep it pinned.
    assert!(
        extra.is_empty(),
        "symbols exported by the Rust .so but not by the C .so: {extra:?}"
    );
}

#[test]
fn c_exports_ima_parse() {
    let c = nm_defined(&common::c_so_path());
    assert!(c.contains(&"ima_parse".to_string()), "got {c:?}");
    assert_eq!(c.len(), 1, "the C library has exactly one public symbol: {c:?}");
}

#[test]
fn rust_so_has_no_unresolved_non_libc_symbols() {
    let u = nm_undefined(&common::rust_so_path());
    let bad: Vec<&String> = u.iter().filter(|s| !is_platform_symbol(s)).collect();
    assert!(
        bad.is_empty(),
        "Rust .so has undefined non-libc symbols (untranslated C?): {bad:?}"
    );
}

#[test]
fn both_libraries_dlopen_and_resolve_ima_parse() {
    common::ensure_loaded();
}

/// Guard against the harness accidentally resolving *both* names to the same
/// implementation (global symbol interposition would make every differential
/// test pass vacuously).
#[test]
fn the_two_resolved_implementations_are_distinct() {
    let (c, r) = common::fn_ptrs();
    assert_ne!(c, 0);
    assert_ne!(r, 0);
    assert_ne!(
        c, r,
        "both `ima_parse` symbols resolved to the SAME address ({c:#x}) — the \
         differential tests would be comparing the C library against itself"
    );
}
