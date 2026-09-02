//! Phase D — symbol parity enforced as a test.
//!
//! Fails if the Rust `.so` does not export every dynamic symbol the C `.so`
//! exports, or if the Rust `.so` has any *non-libc* undefined symbol.

mod common;

use std::path::Path;
use std::process::Command;

fn nm(args: &[&str], so: &Path) -> Vec<String> {
    let out = Command::new("nm")
        .args(args)
        .arg(so)
        .output()
        .expect("nm must be available on PATH");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .map(|s| s.split('@').next().unwrap().to_string())
        .collect()
}

/// Every symbol the C `.so` defines must also be defined by the Rust `.so`.
#[test]
fn symbol_parity_defined() {
    let c = common::c_so_path();
    let r = common::rust_so_path();

    let mut cs = nm(&["-D", "--defined-only"], &c);
    let rs = nm(&["-D", "--defined-only"], &r);
    cs.sort();

    let missing: Vec<&String> = cs.iter().filter(|s| !rs.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing {} symbol(s) exported by the C .so: {missing:?}\n  C   defined: {cs:?}\n  Rust defined: {rs:?}",
        missing.len()
    );
    assert!(
        cs.contains(&"slice".to_string()),
        "sanity: the C .so should export `slice`, got {cs:?}"
    );
    assert_eq!(
        cs.len(),
        1,
        "the C library's public surface changed; SYMBOLS.md must be regenerated: {cs:?}"
    );
}

/// The Rust `.so` must not reference any undefined symbol that is not part of
/// libc / the platform runtime (i.e. no untranslated C module left dangling).
#[test]
fn no_unresolved_non_libc_symbols() {
    let r = common::rust_so_path();
    let undef = nm(&["-D", "-u"], &r);

    // Everything the Rust std / libgcc unwinder legitimately imports.
    let allowed_exact = [
        "_ITM_deregisterTMCloneTable",
        "_ITM_registerTMCloneTable",
        "__gmon_start__",
        "__cxa_finalize",
        "__cxa_thread_atexit_impl",
        "__errno_location",
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
        "memcpy",
        "memmove",
        "memset",
        "mmap64",
        "munmap",
        "open64",
        "posix_memalign",
        "printf",
        "puts",
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

    let leftovers: Vec<&String> = undef
        .iter()
        .filter(|s| {
            !allowed_exact.contains(&s.as_str())
                && !s.starts_with("_Unwind_")
                && !s.starts_with("pthread_")
                && !s.starts_with("__pthread_")
        })
        .collect();

    assert!(
        leftovers.is_empty(),
        "Rust .so has non-libc undefined symbols (untranslated code?): {leftovers:?}"
    );
}
