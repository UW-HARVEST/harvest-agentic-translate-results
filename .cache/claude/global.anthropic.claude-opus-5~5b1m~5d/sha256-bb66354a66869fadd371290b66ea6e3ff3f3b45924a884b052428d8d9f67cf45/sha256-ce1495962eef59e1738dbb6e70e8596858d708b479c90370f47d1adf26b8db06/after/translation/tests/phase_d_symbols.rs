//! Phase D — symbol parity enforced as a test, so it cannot silently rot.
//!
//! Runs `nm -D` on both shared objects and requires that every symbol the C
//! `.so` exports is also exported by the Rust `.so` under the exact same name.

mod common;

use common::*;
use std::path::PathBuf;
use std::process::Command;

fn nm_defined(path: &PathBuf) -> Vec<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", path.to_str().unwrap()])
        .output()
        .expect("run nm");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let (_addr, kind, name) = (it.next()?, it.next()?, it.next()?);
            // Exported code/data symbols only.
            if matches!(kind, "T" | "t" | "D" | "B" | "R" | "W" | "V" | "G" | "S") {
                Some(name.to_string())
            } else {
                None
            }
        })
        .collect();
    v.sort();
    v.dedup();
    v
}

fn nm_undefined(path: &PathBuf) -> Vec<String> {
    let out = Command::new("nm")
        .args(["-D", "--undefined-only", path.to_str().unwrap()])
        .output()
        .expect("run nm");
    let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .collect();
    v.sort();
    v.dedup();
    v
}

/// Symbols provided by the platform C runtime / unwinder, which both objects
/// legitimately import.
fn is_platform_symbol(name: &str) -> bool {
    let base = name.split('@').next().unwrap_or(name);
    if base.starts_with("_Unwind_")
        || base.starts_with("__")
        || base.starts_with("_ITM_")
        || base.starts_with("_dl_")
    {
        return true;
    }
    matches!(
        base,
        "printf"
            | "puts"
            | "fflush"
            | "abort"
            | "malloc"
            | "calloc"
            | "realloc"
            | "free"
            | "posix_memalign"
            | "memcpy"
            | "memmove"
            | "memset"
            | "bcmp"
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
            | "dl_iterate_phdr"
            | "pthread_key_create"
            | "pthread_key_delete"
            | "pthread_setspecific"
            | "pthread_getspecific"
    )
}

#[test]
fn phase_d_every_c_symbol_is_exported_by_rust() {
    let c_path = c_lib().path.clone();
    let r_path = rust_lib().path.clone();

    let c_syms = nm_defined(&c_path);
    let r_syms = nm_defined(&r_path);

    let missing: Vec<&String> = c_syms.iter().filter(|s| !r_syms.contains(s)).collect();

    assert!(
        missing.is_empty(),
        "Rust .so is missing {} symbol(s) exported by the C .so: {:?}\n\
         C   .so = {}\n\
         RUST.so = {}\n\
         (per Phase A: add the #[no_mangle] wrapper if the impl exists, or \
          translate the missing C module)",
        missing.len(),
        missing,
        c_path.display(),
        r_path.display()
    );

    // The C surface is exactly these two functions; guard against the C build
    // silently changing under us.
    for expected in ["siphash", "stbds_hash_bytes"] {
        assert!(c_syms.iter().any(|s| s == expected), "C .so lost symbol {expected}");
        assert!(r_syms.iter().any(|s| s == expected), "Rust .so lost symbol {expected}");
    }

    // `stbds_siphash_bytes` is `static` in C -- it must not be part of either
    // dynamic surface.
    assert!(
        !c_syms.iter().any(|s| s == "stbds_siphash_bytes"),
        "C .so unexpectedly exports the static helper"
    );
    assert!(
        !r_syms.iter().any(|s| s == "stbds_siphash_bytes"),
        "Rust .so exports stbds_siphash_bytes, but it is `static` in the C source"
    );
}

#[test]
fn phase_d_rust_so_has_no_unresolved_non_libc_symbols() {
    let r_path = rust_lib().path.clone();
    let undef = nm_undefined(&r_path);
    let bad: Vec<&String> = undef.iter().filter(|s| !is_platform_symbol(s)).collect();
    assert!(
        bad.is_empty(),
        "Rust .so has unresolved non-libc symbols: {bad:?}\n(from {})",
        r_path.display()
    );
}

#[test]
fn phase_d_both_libraries_resolve_all_documented_entry_points() {
    // Resolving through dlsym is what `c_lib()`/`rust_lib()` already do; this
    // test makes the requirement explicit and prints the resolved addresses.
    let c = c_lib();
    let r = rust_lib();
    println!("C    {} -> hash_bytes={:p} siphash={:p}", c.path.display(), c.hash_bytes as *const (), c.siphash as *const ());
    println!("RUST {} -> hash_bytes={:p} siphash={:p}", r.path.display(), r.hash_bytes as *const (), r.siphash as *const ());
    assert_ne!(c.hash_bytes as usize, r.hash_bytes as usize, "same address: only one .so loaded?");
    assert_ne!(c.siphash as usize, r.siphash as usize, "same address: only one .so loaded?");
}
