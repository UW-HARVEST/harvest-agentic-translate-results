//! Phase D — symbol parity, enforced as a test so it cannot silently regress.

use crate::common::*;
use std::collections::BTreeSet;
use std::process::Command;

fn nm_defined(path: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(path)
        .output()
        .expect("run nm");
    assert!(
        out.status.success(),
        "nm failed on {path:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(str::to_string))
        .collect()
}

fn nm_undefined(path: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--undefined-only"])
        .arg(path)
        .output()
        .expect("run nm");
    assert!(out.status.success());
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(str::to_string))
        .collect()
}

pub fn symbols_exported_by_c_are_all_exported_by_rust() {
    let c = nm_defined(&c_so_path());
    let r = nm_defined(&rust_so_path());

    assert!(!c.is_empty(), "nm found no exports in the C .so");
    assert!(c.contains("driver"));

    let missing: Vec<_> = c.difference(&r).cloned().collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}"
    );
}

pub fn rust_exports_nothing_extra() {
    let c = nm_defined(&c_so_path());
    let r = nm_defined(&rust_so_path());
    let extra: Vec<_> = r.difference(&c).cloned().collect();
    assert!(
        extra.is_empty(),
        "the Rust .so exports symbols the C .so does not: {extra:?}"
    );
}

pub fn rust_has_no_undefined_non_libc_symbols() {
    let und = nm_undefined(&rust_so_path());
    // Everything the Rust cdylib imports must be resolvable from the platform
    // (libc / libgcc_s / libm / ld.so). Anything else means a missing module.
    let unresolved: Vec<_> = und
        .iter()
        .filter(|s| {
            let base = s.split('@').next().unwrap_or(s);
            let is_platform = base.starts_with("_Unwind_")
                || base.starts_with("__")
                || base.starts_with("_ITM_")
                || base.starts_with("pthread_")
                || base.starts_with("dl_")
                || matches!(
                    base,
                    "printf"
                        | "putchar"
                        | "puts"
                        | "fwrite"
                        | "abort"
                        | "bcmp"
                        | "memcmp"
                        | "memcpy"
                        | "memmove"
                        | "memset"
                        | "malloc"
                        | "calloc"
                        | "realloc"
                        | "free"
                        | "posix_memalign"
                        | "strlen"
                        | "getenv"
                        | "getcwd"
                        | "readlink"
                        | "realpath"
                        | "open"
                        | "open64"
                        | "close"
                        | "read"
                        | "write"
                        | "writev"
                        | "lseek"
                        | "lseek64"
                        | "stat"
                        | "stat64"
                        | "fstat"
                        | "fstat64"
                        | "statx"
                        | "mmap"
                        | "mmap64"
                        | "munmap"
                        | "syscall"
                        | "gettid"
                        | "sysconf"
                );
            !is_platform
        })
        .cloned()
        .collect();
    assert!(
        unresolved.is_empty(),
        "Rust .so has undefined NON-libc symbols (untranslated module?): {unresolved:?}"
    );
}

/// The C library is 2 files and 1 public function; assert that inventory so a
/// future C addition forces this verification to be revisited.
pub fn c_source_inventory_is_fully_covered() {
    let c = nm_defined(&c_so_path());
    assert_eq!(
        c.len(),
        1,
        "the C .so exports {} symbols, but this verification was written for \
         exactly 1 (`driver`). New C symbols need new CONFIGS.md/ERRORS.md rows. \
         Found: {c:?}",
        c.len()
    );
}
