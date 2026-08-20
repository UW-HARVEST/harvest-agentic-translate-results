//! Phase D — dynamic-symbol parity between the C and the Rust shared objects.
//!
//! Runs `nm -D --defined-only` on both `.so` files and asserts that every
//! symbol exported by the C library is also exported by the Rust library with
//! the exact same name, and that the Rust library has no unresolved non-libc
//! dependency.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::process::Command;

fn nm(args: &[&str], path: &std::path::Path) -> String {
    let out = Command::new("nm")
        .args(args)
        .arg(path)
        .output()
        .unwrap_or_else(|e| panic!("run nm on {}: {e}", path.display()));
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// Global text/data symbols exported through the dynamic symbol table.
fn exported(path: &std::path::Path) -> BTreeSet<String> {
    nm(&["-D", "--defined-only"], path)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let _addr = it.next()?;
            let kind = it.next()?;
            let name = it.next()?;
            // Skip the linker-provided bookkeeping symbols.
            if name.starts_with("_ITM_") || name == "__gmon_start__" {
                return None;
            }
            Some(format!("{kind} {name}"))
        })
        .collect()
}

fn exported_names(path: &std::path::Path) -> BTreeSet<String> {
    exported(path)
        .into_iter()
        .map(|s| s.split_once(' ').unwrap().1.to_string())
        .collect()
}

#[test]
fn every_c_symbol_is_exported_by_rust() {
    ensure_c_artifacts();
    let c = exported_names(&c_so());
    let r = exported_names(&rust_so());

    assert!(
        c.contains("run") && c.contains("main"),
        "unexpected C symbol surface: {c:?}"
    );

    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but missing from the Rust .so: {missing:?}\n\
         C   = {c:?}\nRust = {r:?}"
    );

    // Same symbol *type* (T = text/code) for the shared names.
    let c_typed = exported(&c_so());
    let r_typed = exported(&rust_so());
    for entry in &c_typed {
        assert!(
            r_typed.contains(entry),
            "Rust .so does not export {entry:?} with the same type; Rust has {r_typed:?}"
        );
    }
}

#[test]
fn rust_so_has_no_unresolved_non_libc_symbols() {
    ensure_c_artifacts();
    let undefined = nm(&["-D", "-u"], &rust_so());
    let allowed_prefixes = [
        "_ITM_", "_Unwind_", "__cxa_", "__gmon_start__", "__errno_location", "__tls_get_addr",
        "__libc_", "_dl_", "std", "core", "alloc",
    ];
    let libc_symbols: BTreeSet<&str> = [
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
        "pthread_setspecific",
        "pthread_getspecific",
        "read",
        "readv",
        "readlink",
        "realloc",
        "realpath",
        "signal",
        "stat",
        "stat64",
        "statx",
        "strlen",
        "syscall",
        "sysconf",
        "write",
        "writev",
        "__cxa_finalize",
        "__cxa_thread_atexit_impl",
    ]
    .into_iter()
    .collect();

    let mut unexpected = Vec::new();
    for line in undefined.lines() {
        let name = match line.split_whitespace().last() {
            Some(n) => n,
            None => continue,
        };
        let bare = name.split('@').next().unwrap();
        if libc_symbols.contains(bare) || allowed_prefixes.iter().any(|p| bare.starts_with(p)) {
            continue;
        }
        unexpected.push(bare.to_string());
    }
    assert!(
        unexpected.is_empty(),
        "Rust .so has unresolved non-libc symbols: {unexpected:?}"
    );
}

#[test]
fn c_static_functions_are_not_exported_by_either_library() {
    ensure_c_artifacts();
    let c = exported_names(&c_so());
    let r = exported_names(&rust_so());
    for hidden in [
        "the_house",
        "add_floor",
        "add_bedrooms",
        "add_floor_to_the_house",
        "print_the_house",
    ] {
        assert!(!c.contains(hidden), "C unexpectedly exports {hidden}");
        assert!(!r.contains(hidden), "Rust unexpectedly exports {hidden}");
    }
}
