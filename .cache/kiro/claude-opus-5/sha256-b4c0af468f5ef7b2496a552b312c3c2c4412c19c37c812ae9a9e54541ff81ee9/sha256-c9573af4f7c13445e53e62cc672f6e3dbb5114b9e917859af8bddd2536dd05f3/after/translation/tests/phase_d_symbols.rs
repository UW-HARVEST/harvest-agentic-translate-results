//! Phase D — symbol parity between the C `.so`s and the Rust `cdylib`.

mod common;

use std::collections::BTreeSet;
use std::process::Command;

fn defined_symbols(path: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", path.to_str().unwrap()])
        .output()
        .expect("nm not available");
    assert!(out.status.success(), "nm failed on {}", path.display());
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().nth(2).map(|s| s.to_string()))
        .collect()
}

fn undefined_symbols(path: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--undefined-only", path.to_str().unwrap()])
        .output()
        .expect("nm not available");
    assert!(out.status.success());
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .collect()
}

#[test]
fn every_c_symbol_is_exported_by_rust() {
    let c1 = defined_symbols(&common::c_lib_path());
    let c2 = defined_symbols(&common::c_driver_lib_path());
    let rust = defined_symbols(&common::rust_lib_path());

    let mut c_all: BTreeSet<String> = c1.union(&c2).cloned().collect();
    // `driver`'s own .so also re-exports nothing else; keep everything.
    c_all.retain(|s| !s.is_empty());

    let missing: Vec<&String> = c_all.difference(&rust).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but missing from the Rust .so: {:?}",
        missing
    );
    assert!(
        c_all.len() >= 79,
        "expected at least 79 C symbols, found {}",
        c_all.len()
    );
}

#[test]
fn rust_has_no_unresolved_non_libc_symbols() {
    let undef = undefined_symbols(&common::rust_lib_path());
    // Everything the Rust .so imports must be a libc / runtime symbol.
    let allowed_prefixes = [
        "_ITM_", "_Unwind_", "__cxa_", "__gmon_", "__errno", "__tls_", "_GLOBAL_",
    ];
    let allowed_exact: BTreeSet<&str> = [
        "abort", "bcmp", "calloc", "close", "dl_iterate_phdr", "exit", "free", "fstat",
        "fstat64", "getcwd", "getenv", "gettid", "localeconv", "lseek", "lseek64", "malloc",
        "memcmp", "memcpy", "memmove", "memset", "mmap", "mmap64", "munmap", "open", "open64",
        "posix_memalign", "printf", "pthread_key_create", "pthread_key_delete",
        "pthread_getspecific", "pthread_setspecific", "puts", "read", "readlink", "realloc",
        "realpath", "sigaction", "sigaltstack", "signal", "snprintf", "sscanf", "stat",
        "stat64", "statx", "strcmp", "strcpy", "strlen", "strncmp", "strtod", "syscall",
        "sysconf", "tolower", "write", "writev", "memrchr", "getauxval", "pthread_self",
        "pthread_mutex_lock", "pthread_mutex_unlock", "pthread_mutex_trylock",
        "pthread_rwlock_rdlock", "pthread_rwlock_unlock", "pthread_rwlock_wrlock",
        "__libc_start_main", "environ", "poll", "sched_yield", "nanosleep", "sigemptyset",
        "sigaddset", "pipe2", "dup", "dup2", "dup3", "fcntl", "isatty", "readv",
    ]
    .into_iter()
    .collect();

    let mut unexpected = Vec::new();
    for s in &undef {
        let base = s.split('@').next().unwrap_or(s);
        if base.is_empty() {
            continue;
        }
        if allowed_prefixes.iter().any(|p| base.starts_with(p)) {
            continue;
        }
        if allowed_exact.contains(base) {
            continue;
        }
        // Any remaining cJSON_* / driver import would be a real completeness bug.
        if base.starts_with("cJSON") || base == "driver" {
            unexpected.push(s.clone());
        }
    }
    assert!(
        unexpected.is_empty(),
        "Rust .so imports cJSON symbols it should define: {:?}",
        unexpected
    );
}

#[test]
fn both_libraries_load_and_agree_on_version() {
    let p = common::pair();
    unsafe {
        let cv = common::take_cstr((p.c.cJSON_Version)()).unwrap();
        let rv = common::take_cstr((p.r.cJSON_Version)()).unwrap();
        assert_eq!(cv, rv, "cJSON_Version mismatch");
        assert_eq!(cv, b"1.7.19".to_vec());
    }
}
