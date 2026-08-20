//! Phase D — symbol parity. Re-runs `nm -D` on both shared objects at test time
//! and asserts the diff is empty, so the check cannot drift out of date.

mod common;

use common::{c_so_path, rust_so_path};
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

fn nm(path: &Path, extra: &[&str]) -> Vec<String> {
    let mut cmd = Command::new("nm");
    cmd.arg("-D");
    cmd.args(extra);
    cmd.arg(path);
    let out = cmd
        .output()
        .unwrap_or_else(|e| panic!("failed to run nm on {}: {e}", path.display()));
    assert!(
        out.status.success(),
        "nm {} failed: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|l| l.to_string())
        .collect()
}

/// Exported (defined) global symbol names, ignoring the CRT/compiler-internal set.
fn defined_symbols(path: &Path) -> BTreeSet<String> {
    nm(path, &["--defined-only"])
        .into_iter()
        .filter_map(|l| {
            // "<addr> <type> <name>"
            let mut it = l.split_whitespace();
            let _addr = it.next()?;
            let ty = it.next()?;
            let name = it.next()?;
            // Only strong global code/data symbols; skip weak CRT hooks.
            if matches!(ty, "T" | "D" | "B" | "R" | "G" | "S") {
                Some(name.to_string())
            } else {
                let _ = ty;
                None
            }
        })
        .filter(|n| {
            !n.starts_with("_ITM_")
                && !n.starts_with("__cxa_")
                && !n.starts_with("__gmon_")
                && *n != "_init"
                && *n != "_fini"
                && *n != "__bss_start"
                && *n != "_edata"
                && *n != "_end"
        })
        .collect()
}

fn undefined_symbols(path: &Path) -> BTreeSet<String> {
    nm(path, &["-u"])
        .into_iter()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .collect()
}

/// Symbols that the dynamic loader resolves from libc / libgcc / libm.
fn is_system_symbol(name: &str) -> bool {
    let base = name.split('@').next().unwrap_or(name);
    if base.starts_with("_Unwind_")
        || base.starts_with("_ITM_")
        || base.starts_with("__cxa_")
        || base.starts_with("__gmon_")
        || base.starts_with("pthread_")
        || base.starts_with("__tls_get_addr")
        || base.starts_with("__errno_location")
        || base.starts_with("__libc_")
    {
        return true;
    }
    const LIBC: &[&str] = &[
        "abort", "bcmp", "calloc", "close", "dl_iterate_phdr", "free", "fstat", "fstat64",
        "getcwd", "getenv", "gettid", "lseek", "lseek64", "malloc", "memcmp", "memcpy",
        "memmove", "memset", "mmap", "mmap64", "munmap", "open", "open64", "posix_memalign",
        "printf", "puts", "read", "readlink", "realloc", "realpath", "stat", "stat64",
        "statx", "strlen", "syscall", "write", "writev", "fflush", "fwrite", "putchar",
        "sysconf", "sigaltstack", "sigaction", "sigemptyset", "sigaddset", "raise",
        "pipe2", "poll", "mprotect", "madvise", "environ", "__environ",
    ];
    LIBC.contains(&base)
}

#[test]
fn every_c_exported_symbol_is_exported_by_rust() {
    let c = c_so_path();
    let r = rust_so_path();
    let cs = defined_symbols(&c);
    let rs = defined_symbols(&r);

    eprintln!("C   .so ({}): {:?}", c.display(), cs);
    eprintln!("RUST.so ({}): {:?}", r.display(), rs);

    // The C library's public surface, spelled out so a regression in either
    // direction is caught.
    let expected: BTreeSet<String> = ["siphash", "stbds_hash_bytes"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    assert_eq!(
        cs, expected,
        "the C .so's exported set changed; update SYMBOLS.md"
    );

    let missing: Vec<_> = cs.difference(&rs).cloned().collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}"
    );

    let extra: Vec<_> = rs.difference(&cs).cloned().collect();
    assert!(
        extra.is_empty(),
        "symbols exported by the Rust .so that the C .so does not export: {extra:?}"
    );
}

#[test]
fn static_c_function_is_exported_by_neither() {
    // `stbds_siphash_bytes` is `static` in src/lib.c:6 -> internal linkage.
    for p in [c_so_path(), rust_so_path()] {
        let s = defined_symbols(&p);
        assert!(
            !s.contains("stbds_siphash_bytes"),
            "{} must not export the static helper stbds_siphash_bytes",
            p.display()
        );
    }
}

#[test]
fn rust_so_has_no_unresolved_non_libc_symbols() {
    let r = rust_so_path();
    let leftovers: Vec<String> = undefined_symbols(&r)
        .into_iter()
        .filter(|n| !is_system_symbol(n))
        .collect();
    assert!(
        leftovers.is_empty(),
        "Rust .so has undefined non-libc symbols (would fail to load): {leftovers:?}"
    );

    // And prove it actually loads and both symbols resolve.
    let (c, rr) = common::impls();
    assert_ne!(c.hash_bytes as usize, rr.hash_bytes as usize);
}

#[test]
fn both_objects_can_be_dlopened_and_symbols_called() {
    let (c, r) = common::impls();
    let mut buf = *b"abcdefghij";
    let cv = unsafe { (c.hash_bytes)(buf.as_mut_ptr() as *mut std::ffi::c_void, 10, 0) };
    let rv = unsafe { (r.hash_bytes)(buf.as_mut_ptr() as *mut std::ffi::c_void, 10, 0) };
    assert_eq!(cv, rv, "smoke test through both .so exports");
}
