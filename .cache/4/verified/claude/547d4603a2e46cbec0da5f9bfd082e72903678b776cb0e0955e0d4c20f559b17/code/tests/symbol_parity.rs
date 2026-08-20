//! Phase D — symbol parity between the C `.so` and the Rust `.so`.
//!
//! The diff of `nm -D --defined-only` must be empty in the C -> Rust direction,
//! every C-exported symbol must additionally be resolvable with `dlsym` through
//! `libloading`, and the Rust `.so` must not have any undefined symbol outside
//! the libc / unwind runtime.

mod common;

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

use common::*;

fn nm(path: &Path, extra: &str) -> Vec<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg(extra)
        .arg(path)
        .output()
        .expect("run nm");
    assert!(
        out.status.success(),
        "nm {extra} {path:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace().rev();
            let name = it.next()?;
            let kind = it.next()?;
            // "<addr> T name" -> kind == "T"; "U name"/"w name" -> kind is the type
            if kind.len() == 1 {
                Some(name.to_string())
            } else {
                None
            }
        })
        .collect()
}

fn defined(path: &Path) -> BTreeSet<String> {
    nm(path, "--defined-only").into_iter().collect()
}

fn undefined(path: &Path) -> BTreeSet<String> {
    nm(path, "--undefined-only").into_iter().collect()
}

/// libc / compiler-runtime imports that `std` legitimately pulls in.
fn is_runtime_symbol(name: &str) -> bool {
    let base = name.split('@').next().unwrap_or(name);
    const PREFIXES: [&str; 6] = ["_ITM_", "_Unwind_", "__", "_dl_", "pthread_", "_rust"];
    if PREFIXES.iter().any(|p| base.starts_with(p)) {
        return true;
    }
    const LIBC: [&str; 44] = [
        "abort", "bcmp", "calloc", "close", "dl_iterate_phdr", "free", "fstat", "fstat64",
        "getcwd", "getenv", "gettid", "lseek64", "malloc", "memcmp", "memcpy", "memmove",
        "memset", "mmap", "mmap64", "munmap", "open", "open64", "posix_memalign", "read",
        "readlink", "realloc", "realpath", "sigaltstack", "sigaction", "signal", "stat",
        "stat64", "statx", "strlen", "syscall", "sysconf", "write", "writev", "sqrtf", "sqrt",
        "mprotect", "poll", "getrandom", "environ",
    ];
    LIBC.contains(&base)
}

#[test]
fn phase_d_c_symbols_are_all_exported_by_rust() {
    let cpath = c_so_path();
    let rpath = rust_so_path();
    let cdef = defined(&cpath);
    let rdef = defined(&rpath);

    println!("C   .so {:?}: {} defined symbol(s): {:?}", cpath, cdef.len(), cdef);
    println!("Rust.so {:?}: {} defined symbol(s): {:?}", rpath, rdef.len(), rdef);

    assert!(!cdef.is_empty(), "nm found no defined symbol in the C .so");

    let missing: Vec<&String> = cdef.difference(&rdef).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}"
    );
}

#[test]
fn phase_d_every_c_symbol_is_dlsym_resolvable_in_rust() {
    let cdef = defined(&c_so_path());
    let lib = unsafe { libloading::Library::new(rust_so_path()).expect("dlopen rust .so") };
    for name in &cdef {
        let mut bytes = name.as_bytes().to_vec();
        bytes.push(0);
        let sym: Result<libloading::Symbol<*const ()>, _> = unsafe { lib.get(&bytes) };
        assert!(sym.is_ok(), "dlsym({name}) failed on the Rust .so");
    }
}

#[test]
fn phase_d_rust_has_no_non_libc_undefined_symbols() {
    let rund = undefined(&rust_so_path());
    let bad: Vec<&String> = rund.iter().filter(|n| !is_runtime_symbol(n)).collect();
    assert!(
        bad.is_empty(),
        "Rust .so has undefined non-libc symbols: {bad:?}"
    );
}

#[test]
fn phase_d_symbols_md_lists_every_c_symbol() {
    let doc = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("SYMBOLS.md"),
    )
    .expect("read SYMBOLS.md");
    for name in defined(&c_so_path()) {
        assert!(
            doc.contains(&name),
            "SYMBOLS.md does not document the exported symbol {name}"
        );
    }
}
