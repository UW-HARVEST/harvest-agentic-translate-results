//! Phase D — symbol parity between the C `.so` and the Rust `.so`.
//!
//! Enforces the `SYMBOLS.md` gate as an executable test: every dynamic symbol
//! the C library defines must also be defined by the Rust library under the
//! exact same name, and every symbol the Rust library needs must resolve.

mod common;

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

use common::{c_lib_path, rust_lib_path, Diff};

fn nm(path: &Path, extra: &str) -> Vec<String> {
    let out = Command::new("nm")
        .args(["-D", extra])
        .arg(path)
        .output()
        .unwrap_or_else(|e| panic!("running nm on {}: {e}", path.display()));
    assert!(
        out.status.success(),
        "nm -D {extra} {} failed: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(str::to_string))
        .collect()
}

fn defined(path: &Path) -> BTreeSet<String> {
    nm(path, "--defined-only").into_iter().collect()
}

fn undefined(path: &Path) -> BTreeSet<String> {
    nm(path, "--undefined-only").into_iter().collect()
}

/// The gate: `nm -D` on the C `.so` minus `nm -D` on the Rust `.so` must be
/// empty.
#[test]
fn sym_01_every_c_symbol_is_exported_by_rust() {
    let c = defined(&c_lib_path());
    let r = defined(&rust_lib_path());

    assert!(
        !c.is_empty(),
        "the C library exports nothing — did the build produce a stub?"
    );
    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}\n  C: \
         {c:?}\n  Rust: {r:?}"
    );

    // The whole public surface of this library is the single entry point.
    assert!(c.contains("update_frame_header"), "C symbol set: {c:?}");
    assert!(r.contains("update_frame_header"), "Rust symbol set: {r:?}");
}

/// Every undefined symbol in the Rust `.so` must be a libc / libgcc-unwinder
/// import — i.e. there is no dangling reference to un-translated C code.
#[test]
fn sym_02_no_non_libc_undefined_symbols_in_rust() {
    let u = undefined(&rust_lib_path());
    let allowed_prefixes = [
        "_Unwind_",
        "__",
        "_ITM_",
        "pthread_",
        "gettid",
        "statx",
        "syscall",
        "abort",
        "bcmp",
        "calloc",
        "close",
        "dl_iterate_phdr",
        "free",
        "fstat",
        "getcwd",
        "getenv",
        "lseek",
        "malloc",
        "memcpy",
        "memmove",
        "memset",
        "mmap",
        "munmap",
        "open",
        "posix_memalign",
        "read",
        "readlink",
        "realloc",
        "realpath",
        "stat",
        "strlen",
        "write",
        "writev",
    ];
    let unexpected: Vec<&String> = u
        .iter()
        .filter(|s| {
            let bare = s.split('@').next().unwrap_or(s);
            !allowed_prefixes.iter().any(|p| bare.starts_with(p))
        })
        .collect();
    assert!(
        unexpected.is_empty(),
        "Rust .so has undefined non-libc symbols (un-translated C?): {unexpected:?}"
    );

    // And the loader agrees: dlopen of the Rust .so resolved everything.
    let _ = Diff::load();
}

/// Sanity: the Rust `.so` really is the crate's cdylib and the C `.so` really is
/// the CMake output, so the test above compared the right pair of files.
#[test]
fn sym_03_libraries_are_distinct_files() {
    let c = c_lib_path();
    let r = rust_lib_path();
    assert!(c.is_file(), "{} is not a file", c.display());
    assert!(r.is_file(), "{} is not a file", r.display());
    assert_ne!(
        std::fs::canonicalize(&c).unwrap(),
        std::fs::canonicalize(&r).unwrap(),
        "the differential tests must load two different shared objects"
    );
    assert_eq!(
        r.file_name().unwrap().to_str().unwrap(),
        "libupdate_frame_header_lib.so"
    );
}
