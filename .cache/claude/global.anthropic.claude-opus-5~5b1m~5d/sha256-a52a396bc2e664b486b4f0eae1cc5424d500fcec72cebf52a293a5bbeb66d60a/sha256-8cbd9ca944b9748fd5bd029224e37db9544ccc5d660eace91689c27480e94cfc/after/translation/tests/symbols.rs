//! Phase D — symbol parity between the C `.so` and the Rust `.so`.

mod common;

use std::collections::BTreeSet;
use std::process::Command;

/// Global text symbols defined by a shared object, per `nm -D --defined-only`.
fn defined_text_symbols(path: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(path)
        .output()
        .expect("nm not available");
    assert!(out.status.success(), "nm failed on {}", path.display());
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let _addr = it.next()?;
            let kind = it.next()?;
            let name = it.next()?;
            if kind == "T" {
                Some(name.to_string())
            } else {
                None
            }
        })
        .collect()
}

/// Undefined symbols the object imports.
fn undefined_symbols(path: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm").args(["-D", "-u"]).arg(path).output().expect("nm not available");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .filter(|s| s != "U")
        .collect()
}

#[test]
fn symbol_parity() {
    let c = defined_text_symbols(&common::c_so_path());
    let r = defined_text_symbols(&common::rust_so_path());
    assert!(!c.is_empty(), "no symbols found in the C .so");

    let missing: Vec<_> = c.difference(&r).cloned().collect();
    assert!(
        missing.is_empty(),
        "symbols exported by C but MISSING from Rust: {missing:?}\nC: {c:?}\nRust: {r:?}"
    );
    assert_eq!(c.len(), 12, "C symbol count changed: {c:?}");
}

#[test]
fn every_c_symbol_is_callable_through_the_rust_so() {
    // dlsym each one, so a symbol that exists in `nm` but is unresolvable fails here.
    let p = common::Pair::fresh();
    for name in defined_text_symbols(&common::c_so_path()) {
        p.r.op3_addr(&name); // any fn type works for taking the address
    }
}

#[test]
fn rust_so_has_no_undefined_non_libc_symbols() {
    let allowed_prefixes = [
        "_", "__", "malloc", "free", "memmove", "memset", "memcpy", "memcmp", "time", "difftime",
        "snprintf", "abort", "calloc", "realloc", "posix_memalign", "write", "writev", "dl_",
        "pthread", "getenv", "sysconf", "strlen", "bcmp", "gnu_get_libc_version", "syscall",
        "mmap", "munmap", "open", "close", "read", "poll", "sigaltstack", "sigaction",
        "pipe2", "getcwd", "readlink", "statx", "stat", "fstat", "lseek", "unlink", "rmdir",
        "mkdir", "rename", "opendir", "readdir", "closedir", "getrandom", "qsort", "strerror",
        "raise", "exit", "environ", "gettid", "realpath",
    ];
    let undef = undefined_symbols(&common::rust_so_path());
    let unexpected: Vec<_> = undef
        .iter()
        .filter(|s| !allowed_prefixes.iter().any(|p| s.starts_with(p)))
        .cloned()
        .collect();
    assert!(unexpected.is_empty(), "unexpected undefined symbols in Rust .so: {unexpected:?}");
}

#[test]
fn no_driver_binary_to_diff() {
    // Documents the Phase B/D "if the project builds a binary" clause: it does not.
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    assert!(!root.join("src/main.rs").exists(), "a Rust binary appeared; add stdout diffing");
    let cmake = std::fs::read_to_string(root.parent().unwrap().join("c_src/CMakeLists.txt")).unwrap();
    assert!(!cmake.contains("add_executable"), "CMake now builds an executable; add stdout diffing");
}
