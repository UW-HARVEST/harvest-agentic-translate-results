//! Phase D: symbol parity between the C `.so` and the Rust `.so`.
//!
//! Every dynamic symbol the C library defines must be defined by the Rust
//! library under the exact same name.

mod common;

use common::{c_so_path, rust_so_path};
use std::collections::BTreeSet;
use std::process::Command;

/// `nm -D --defined-only <so>` reduced to the set of symbol names.
fn defined_symbols(so: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(so)
        .output()
        .expect("failed to run `nm` (binutils required)");
    assert!(
        out.status.success(),
        "nm failed on {so:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(str::to_owned))
        .collect()
}

fn undefined_symbols(so: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--undefined-only"])
        .arg(so)
        .output()
        .expect("failed to run `nm`");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(str::to_owned))
        .collect()
}

#[test]
fn every_c_symbol_is_exported_by_rust() {
    let c = defined_symbols(&c_so_path());
    let rust = defined_symbols(&rust_so_path());

    assert!(
        c.contains("driver"),
        "sanity: the C .so must export `driver`, got {c:?}"
    );

    let missing: Vec<_> = c.difference(&rust).cloned().collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but missing from the Rust .so: {missing:?}"
    );
}

/// The Rust `.so` must not depend on any unresolved non-libc symbol (which would
/// mean a piece of the translation is missing or stubbed out to an external).
#[test]
fn rust_so_has_no_unexpected_undefined_symbols() {
    let allowed_prefixes = [
        "_ITM_", "_Unwind_", "__cxa_", "__gmon_", "__tls_get_addr", "__errno_location",
        "__libc_", "__rust_",
    ];
    let allowed_exact: BTreeSet<&str> = [
        "abort", "bcmp", "calloc", "close", "dl_iterate_phdr", "free", "fstat", "fstat64",
        "getcwd", "getenv", "gettid", "lseek", "lseek64", "malloc", "memcmp", "memcpy",
        "memmove", "memset", "mmap", "mmap64", "munmap", "open", "open64", "posix_memalign",
        "pthread_getspecific", "pthread_key_create", "pthread_key_delete", "pthread_mutex_lock",
        "pthread_mutex_trylock", "pthread_mutex_unlock", "pthread_setspecific", "pthread_self",
        "puts", "printf", "putchar", "fwrite", "read", "readlink", "realloc", "realpath",
        "sigaction", "sigaltstack", "stat", "stat64", "statx", "strlen", "syscall", "write",
        "writev", "sysconf", "getrandom",
    ]
    .into_iter()
    .collect();

    let unexpected: Vec<String> = undefined_symbols(&rust_so_path())
        .into_iter()
        .map(|s| s.split('@').next().unwrap_or(&s).to_owned())
        .filter(|s| {
            !allowed_exact.contains(s.as_str())
                && !allowed_prefixes.iter().any(|p| s.starts_with(p))
        })
        .collect();

    assert!(
        unexpected.is_empty(),
        "Rust .so imports unexpected (non-libc) undefined symbols: {unexpected:?}"
    );
}

/// The single public entry point must be reachable through `dlsym` on both
/// libraries — this is what the differential tests rely on.
#[test]
fn both_libraries_resolve_driver_via_dlsym() {
    let _c = common::c_driver();
    let _r = common::rust_driver();
}
