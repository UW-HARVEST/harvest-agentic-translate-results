//! Phase D — symbol parity, enforced automatically rather than by hand.
//!
//! Runs `nm -D` on both shared objects and requires that every symbol the C
//! `.so` exports is also exported by the Rust `.so` under the exact same name,
//! and that the Rust `.so` imports no non-libc / non-runtime symbol (which would
//! mean part of the library was never translated).

mod common;

use common::*;
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

fn nm(path: &Path, args: &[&str]) -> Vec<String> {
    let out = Command::new("nm")
        .args(args)
        .arg(path)
        .output()
        .unwrap_or_else(|e| panic!("failed to run nm on {}: {e}", path.display()));
    assert!(
        out.status.success(),
        "nm {args:?} {} failed: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .map(|s| s.split('@').next().unwrap().to_string())
        .collect()
}

fn defined(path: &Path) -> BTreeSet<String> {
    nm(path, &["-D", "--defined-only"]).into_iter().collect()
}

fn undefined(path: &Path) -> BTreeSet<String> {
    nm(path, &["-D", "-u"]).into_iter().collect()
}

/// Symbols the C compiler / dynamic loader injects into any shared object, and
/// the libc + unwinder imports Rust's `std` pulls in. Anything outside this set
/// that the Rust `.so` leaves undefined would be untranslated code.
fn is_runtime_or_libc(sym: &str) -> bool {
    const EXACT: &[&str] = &[
        "_ITM_deregisterTMCloneTable",
        "_ITM_registerTMCloneTable",
        "__cxa_finalize",
        "__cxa_thread_atexit_impl",
        "__gmon_start__",
        "__errno_location",
        "__tls_get_addr",
        "abort",
        "bcmp",
        "calloc",
        "close",
        "dl_iterate_phdr",
        "free",
        "fstat",
        "fstat64",
        "getcwd",
        "getenv",
        "gettid",
        "lseek",
        "lseek64",
        "malloc",
        "memcmp",
        "memcpy",
        "memmove",
        "memset",
        "mmap",
        "mmap64",
        "munmap",
        "open",
        "open64",
        "posix_memalign",
        "pthread_key_create",
        "pthread_key_delete",
        "pthread_getspecific",
        "pthread_setspecific",
        "read",
        "readlink",
        "realloc",
        "realpath",
        "stat",
        "stat64",
        "statx",
        "strlen",
        "syscall",
        "write",
        "writev",
    ];
    EXACT.contains(&sym) || sym.starts_with("_Unwind_") || sym.starts_with("__libc_")
}

#[test]
fn every_c_exported_symbol_is_exported_by_rust() {
    let l = libs();
    let c = defined(&l.c_path);
    let r = defined(&l.rust_path);

    assert!(
        c.contains("hdr_compare"),
        "C .so unexpectedly lacks hdr_compare; symbol list: {c:?}"
    );

    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing {} symbol(s) exported by the C .so: {missing:?}\n\
         C exports:    {c:?}\nRust exports: {r:?}",
        missing.len()
    );
    eprintln!("C exports {} symbol(s): {c:?}", c.len());
    eprintln!("Rust exports {} symbol(s): {r:?}", r.len());
}

#[test]
fn rust_so_has_no_untranslated_undefined_symbols() {
    let l = libs();
    let leftovers: Vec<String> = undefined(&l.rust_path)
        .into_iter()
        .filter(|s| !is_runtime_or_libc(s))
        .collect();
    assert!(
        leftovers.is_empty(),
        "Rust .so imports non-libc symbols, i.e. some C source was not translated: {leftovers:?}"
    );
}

/// `hdr_valid` is `static` in the C, so neither `.so` may export it. A Rust
/// translation that exported it would be an ABI mismatch in the other direction.
#[test]
fn static_c_helper_is_not_exported_by_either() {
    let l = libs();
    for path in [&l.c_path, &l.rust_path] {
        let d = defined(path);
        assert!(
            !d.contains("hdr_valid"),
            "{} unexpectedly exports the file-local helper hdr_valid",
            path.display()
        );
    }
}

/// The loaded Rust `.so` really is a dynamically loaded shared object an external
/// consumer would use, not an inlined call into the test binary.
#[test]
fn rust_symbol_is_reached_through_dlopen() {
    let l = libs();
    assert_eq!(
        l.rust_path.extension().and_then(|e| e.to_str()),
        Some("so"),
        "unexpected Rust artifact: {}",
        l.rust_path.display()
    );
    if std::env::var_os("HDR_RUST_SO").is_none() {
        assert!(
            l.rust_path.to_string_lossy().ends_with("libhdr_compare_lib.so"),
            "unexpected Rust artifact: {}",
            l.rust_path.display()
        );
    }
    let ok = [0xffu8, 0xfb, 0x90];
    assert_eq!(unsafe { (l.rust)(ok.as_ptr(), ok.as_ptr()) }, 1);
    assert_eq!(unsafe { (l.c)(ok.as_ptr(), ok.as_ptr()) }, 1);
}
