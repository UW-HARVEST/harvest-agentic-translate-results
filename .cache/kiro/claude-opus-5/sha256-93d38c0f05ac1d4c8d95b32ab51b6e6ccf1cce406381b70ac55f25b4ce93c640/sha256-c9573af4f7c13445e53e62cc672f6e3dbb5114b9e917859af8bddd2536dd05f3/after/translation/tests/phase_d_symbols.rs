//! Phase D — dynamic symbol parity, enforced as a test rather than a claim.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::process::Command;

fn defined_symbols(path: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", "--format=posix"])
        .arg(path)
        .output()
        .expect("run nm");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().next())
        .map(|s| s.to_string())
        .collect()
}

/// Symbols the Rust cdylib legitimately adds (Rust/LLVM runtime bookkeeping,
/// not library API).
fn is_toolchain_symbol(s: &str) -> bool {
    s.starts_with("_ZN")
        || s.starts_with("rust_")
        || s.starts_with("__rust")
        || s.starts_with("_R")
        || s == "_init"
        || s == "_fini"
        || s == "_edata"
        || s == "_end"
        || s == "__bss_start"
        || s.starts_with("_ITM_")
        || s.starts_with("__gmon")
}

#[test]
fn phase_d_every_c_symbol_is_exported_by_rust() {
    let c = defined_symbols(&c_so_path());
    let r = defined_symbols(&rust_so_path());

    let c_api: BTreeSet<_> = c.iter().filter(|s| !is_toolchain_symbol(s)).cloned().collect();
    let missing: Vec<_> = c_api.difference(&r).cloned().collect();

    assert!(
        missing.is_empty(),
        "the Rust .so is missing {} symbol(s) exported by the C .so: {missing:?}\n\
         C   ({}): {c_api:?}\n\
         Rust({}): {:?}",
        missing.len(),
        c_so_path().display(),
        rust_so_path().display(),
        r.iter().filter(|s| !is_toolchain_symbol(s)).collect::<Vec<_>>()
    );

    // The two documented entry points must actually be there.
    for want in ["w_utf8_drop", "w_utf8_filter"] {
        assert!(c.contains(want), "C .so should export {want}");
        assert!(r.contains(want), "Rust .so should export {want}");
    }
}

/// The Rust `.so` must not have undefined symbols beyond libc / the unwinder,
/// i.e. nothing that would indicate an untranslated module.
#[test]
fn phase_d_no_unexpected_undefined_symbols() {
    let out = Command::new("nm")
        .args(["-D", "--undefined-only", "--format=posix"])
        .arg(rust_so_path())
        .output()
        .expect("run nm");
    let mut unexpected = Vec::new();
    for line in String::from_utf8_lossy(&out.stdout).lines() {
        let sym = match line.split_whitespace().next() {
            Some(s) => s,
            None => continue,
        };
        let bare = sym.split('@').next().unwrap_or(sym);
        let ok = bare.starts_with('_') // libc/libgcc internals, _Unwind_*, __*
            || bare.starts_with("pthread_")
            || matches!(
                bare,
                "malloc"
                    | "realloc"
                    | "calloc"
                    | "free"
                    | "posix_memalign"
                    | "strdup"
                    | "strlen"
                    | "memcpy"
                    | "memmove"
                    | "memset"
                    | "bcmp"
                    | "abort"
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
                    | "lseek64"
                    | "fstat64"
                    | "stat64"
                    | "statx"
                    | "mmap64"
                    | "munmap"
                    | "syscall"
                    | "gettid"
                    | "dl_iterate_phdr"
                    | "sysconf"
                    | "poll"
                    | "sigaction"
                    | "sigaltstack"
                    | "signal"
                    | "raise"
                    | "memrchr"
                    | "strerror_r"
            );
        if !ok {
            unexpected.push(bare.to_string());
        }
    }
    assert!(
        unexpected.is_empty(),
        "Rust .so has unexpected undefined (non-libc) symbols: {unexpected:?}"
    );
}
