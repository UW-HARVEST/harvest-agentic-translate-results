//! Phase D — symbol parity between the C `.so` and the Rust `.so`.
//!
//! Enforces mechanically what SYMBOLS.md documents: every symbol the C shared
//! object exports must also be exported by the Rust shared object, under the
//! exact same name, and the Rust `.so` must not have any undefined symbol that
//! is not provided by libc/libgcc.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::process::Command;

fn nm(path: &PathBuf, args: &[&str]) -> Vec<String> {
    let out = Command::new("nm")
        .args(args)
        .arg(path)
        .output()
        .unwrap_or_else(|e| panic!("failed to run nm on {}: {e}", path.display()));
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|l| l.to_string())
        .collect()
}

/// Global (uppercase-type) defined symbol names.
fn exported_globals(path: &PathBuf) -> BTreeSet<String> {
    nm(path, &["-D", "--defined-only"])
        .iter()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let (a, b) = (it.next()?, it.next()?);
            // Either "<addr> <type> <name>" or "<type> <name>".
            let (ty, name) = match it.next() {
                Some(name) => (b, name),
                None => (a, b),
            };
            let t = ty.chars().next()?;
            // Keep only global (uppercase) symbols; skip local (lowercase) and
            // the compiler/linker-generated ELF housekeeping entries.
            if !t.is_ascii_uppercase() {
                return None;
            }
            if name.starts_with("_ITM_")
                || name.starts_with("__cxa_")
                || name.starts_with("__gmon")
                || name.starts_with("_fini")
                || name.starts_with("_init")
                || name.starts_with("__bss_start")
                || name == "_edata"
                || name == "_end"
            {
                return None;
            }
            Some(name.to_string())
        })
        .collect()
}

fn undefined(path: &PathBuf) -> BTreeSet<String> {
    nm(path, &["-D", "--undefined-only"])
        .iter()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .collect()
}

#[test]
fn d01_every_c_exported_symbol_is_exported_by_rust() {
    let (cp, rp) = lib_paths();
    let c = exported_globals(&cp);
    let r = exported_globals(&rp);

    assert!(
        !c.is_empty(),
        "nm found no exported symbols in the C .so at {} — the build is wrong",
        cp.display()
    );

    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}\n\
         C exports:    {c:?}\n Rust exports: {r:?}"
    );

    // The known surface, spelled out so a silently-shrinking export list fails.
    for want in ["driver", "fma_array"] {
        assert!(c.contains(want), "C .so should export `{want}`");
        assert!(r.contains(want), "Rust .so should export `{want}`");
    }
}

#[test]
fn d02_rust_has_no_undefined_non_libc_symbols() {
    let (_, rp) = lib_paths();
    let leftover: Vec<String> = undefined(&rp)
        .into_iter()
        .filter(|s| {
            // Everything legitimately provided by glibc / libgcc.
            !(s.starts_with("_Unwind_")
                || s.starts_with("__")
                || s.starts_with("_ITM_")
                || s.contains("@GLIBC")
                || s.contains("@GCC")
                || s.starts_with("pthread_")
                || matches!(
                    s.as_str(),
                    "malloc" | "calloc" | "realloc" | "free" | "posix_memalign"
                        | "memcpy" | "memmove" | "memset" | "bcmp" | "strlen"
                        | "printf" | "abort" | "getenv" | "getcwd" | "readlink"
                        | "realpath" | "syscall" | "read" | "write" | "writev"
                        | "open" | "open64" | "close" | "lseek" | "lseek64"
                        | "stat" | "stat64" | "fstat" | "fstat64" | "statx"
                        | "mmap" | "mmap64" | "munmap" | "dl_iterate_phdr"
                        | "gettid" | "sysconf"
                ))
        })
        .collect();
    assert!(
        leftover.is_empty(),
        "Rust .so has undefined symbols that are not libc/libgcc: {leftover:?}"
    );
}

#[test]
fn d03_both_libraries_resolve_and_are_callable() {
    // Proves the exports are not just names in the symbol table: both are
    // dlopen-able and both symbols are actually invocable through the FFI.
    let l = libs();
    let mut out = [0i32; 4];
    let a = [2i32, 3, 4, 5];
    for imp in [&l.c, &l.rs] {
        unsafe {
            (imp.fma_array)(out.as_mut_ptr(), a.as_ptr(), a.as_ptr(), a.as_ptr(), 4);
        }
        assert_eq!(out, [6, 12, 20, 30], "{} fma_array smoke test", imp.name);
    }
}
