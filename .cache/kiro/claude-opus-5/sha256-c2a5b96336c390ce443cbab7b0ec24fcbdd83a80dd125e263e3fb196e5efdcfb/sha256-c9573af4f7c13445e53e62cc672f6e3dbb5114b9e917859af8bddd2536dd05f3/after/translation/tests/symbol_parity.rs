//! Phase D — symbol parity between the C `.so` and the Rust `.so`.
//!
//! Re-derives the `SYMBOLS.md` tables at test time with `nm -D` so the artifact
//! cannot silently go stale.

mod common;

use std::collections::BTreeSet;
use std::process::Command;

fn nm(args: &[&str], path: &std::path::Path) -> Vec<String> {
    let out = Command::new("nm")
        .args(args)
        .arg(path)
        .output()
        .unwrap_or_else(|e| panic!("failed to run nm on {}: {e}", path.display()));
    assert!(
        out.status.success(),
        "nm {:?} {} failed: {}",
        args,
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(str::to_string))
        .collect()
}

/// Symbols an ELF object always references or defines regardless of source.
fn is_toolchain_noise(sym: &str) -> bool {
    matches!(
        sym,
        "_init"
            | "_fini"
            | "__bss_start"
            | "_edata"
            | "_end"
            | "_ITM_deregisterTMCloneTable"
            | "_ITM_registerTMCloneTable"
            | "__cxa_finalize"
            | "__gmon_start__"
            | "__gnu_lto_slim"
    ) || sym.starts_with("__cxa_finalize@")
        || sym.starts_with("_ITM_")
}

fn defined_exports(path: &std::path::Path) -> BTreeSet<String> {
    nm(&["-D", "--defined-only"], path)
        .into_iter()
        .filter(|s| !is_toolchain_noise(s))
        .collect()
}

#[test]
fn every_c_symbol_is_exported_by_rust() {
    let c = defined_exports(&common::c_so_path());
    let r = defined_exports(&common::rust_so_path());

    println!("C exports   ({}): {c:?}", c.len());
    println!("Rust exports({}): {r:?}", r.len());

    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}"
    );

    // The C library exports exactly one function; guard against the artifact
    // going stale if `c_src` ever grows a second symbol.
    assert!(
        c.contains("bin2hex"),
        "expected `bin2hex` among the C exports, got {c:?}"
    );
    assert_eq!(
        c.len(),
        1,
        "SYMBOLS.md documents exactly 1 C export; nm now reports {}: {c:?}",
        c.len()
    );
}

#[test]
fn rust_so_has_no_undefined_non_libc_symbols() {
    let undef = nm(&["-D", "-u"], &common::rust_so_path());
    let unexpected: Vec<&String> = undef
        .iter()
        .filter(|s| !is_toolchain_noise(s))
        .filter(|s| {
            // Everything the Rust cdylib imports must come from libc, libm,
            // libpthread/libdl (all merged into glibc) or libgcc's unwinder.
            let bare = s.split('@').next().unwrap_or(s);
            !(bare.starts_with("_Unwind_")
                || bare.starts_with("__libc_")
                || bare.starts_with("__cxa_")
                || bare.starts_with("pthread_")
                || bare.starts_with("__pthread_")
                || bare.starts_with("__tls_get_addr")
                || bare.starts_with("__errno_location")
                || bare.starts_with("__stack_chk")
                || bare.starts_with("__memcpy_chk")
                || matches!(
                    bare,
                    "abort"
                        | "bcmp"
                        | "calloc"
                        | "close"
                        | "dl_iterate_phdr"
                        | "free"
                        | "fstat"
                        | "fstat64"
                        | "getcwd"
                        | "getenv"
                        | "gettid"
                        | "lseek"
                        | "lseek64"
                        | "malloc"
                        | "memcmp"
                        | "memcpy"
                        | "memmove"
                        | "memset"
                        | "mmap"
                        | "mmap64"
                        | "munmap"
                        | "open"
                        | "open64"
                        | "posix_memalign"
                        | "read"
                        | "readlink"
                        | "realloc"
                        | "realpath"
                        | "stat"
                        | "stat64"
                        | "statx"
                        | "strlen"
                        | "syscall"
                        | "write"
                        | "writev"
                ))
        })
        .collect();
    assert!(
        unexpected.is_empty(),
        "Rust .so has undefined non-libc symbols (unresolved at load time): {unexpected:?}"
    );
}

#[test]
fn both_libraries_actually_expose_the_symbol_via_dlsym() {
    // Loading through libloading is the real proof: `nm` inspects the table,
    // `dlsym` proves the symbol is resolvable by an external caller.
    let (c, r) = common::both();
    assert_eq!(c.name, "C");
    assert_eq!(r.name, "Rust");
    println!("loaded C   : {}", c.path.display());
    println!("loaded Rust: {}", r.path.display());
}
