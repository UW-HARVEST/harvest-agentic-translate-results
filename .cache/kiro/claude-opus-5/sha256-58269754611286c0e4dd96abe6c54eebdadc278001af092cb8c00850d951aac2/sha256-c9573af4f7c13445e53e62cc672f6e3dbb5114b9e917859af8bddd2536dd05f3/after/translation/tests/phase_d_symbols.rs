//! Phase D — exported-symbol parity between the C `.so` and the Rust `.so`.
//!
//! Runs `nm -D` on both libraries and requires that the set of *defined* dynamic
//! symbols the C exports is a subset of what the Rust exports, with identical
//! names (including any macro-generated ones), and that the Rust imports nothing
//! outside libc / the unwinder.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::process::Command;

fn nm(path: &std::path::Path, mode: &str) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", mode])
        .arg(path)
        .output()
        .expect("nm not available");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .filter(|s| !s.is_empty())
        .collect()
}

/// Symbols every Rust `cdylib` emits or imports that are not part of the C ABI.
fn is_toolchain_symbol(s: &str) -> bool {
    const LIBC: &[&str] = &[
        "malloc", "calloc", "realloc", "free", "posix_memalign", "memcpy", "memmove", "memset",
        "memcmp", "bcmp", "strlen", "abort", "getenv", "getcwd", "readlink", "realpath", "syscall",
        "gettid", "read", "write", "writev", "close", "open64", "lseek64", "fstat64", "stat64",
        "statx", "mmap64", "munmap", "dl_iterate_phdr", "sysconf", "pthread_self", "sigaltstack",
        "sigaction", "mprotect", "madvise", "getrandom", "clock_gettime", "nanosleep", "sched_yield",
        "pthread_getattr_np", "pthread_attr_getstack", "pthread_attr_destroy", "poll", "pipe2",
        "prctl", "raise", "signal", "getpid", "openat", "fcntl", "exit", "environ", "qsort",
        "strerror_r", "vfprintf", "fwrite", "fflush", "fputc", "fputs",
    ];
    let base = s.split('@').next().unwrap_or(s);
    if base.starts_with('_') {
        // __cxa_*, __errno_location, __tls_get_addr, _Unwind_*, _ITM_*, __gmon_start__,
        // __rust_*, _init/_fini, ...
        return true;
    }
    if base.starts_with("pthread_") || base.starts_with("rust_") {
        return true;
    }
    LIBC.contains(&base)
}

#[test]
fn d_exported_symbol_parity() {
    let c = c_so_path();
    let r = rust_so_path();
    println!("C   .so: {}", c.display());
    println!("Rust.so: {}", r.display());

    let c_defined = nm(&c, "--defined-only");
    let r_defined = nm(&r, "--defined-only");

    let missing: Vec<&String> = c_defined.difference(&r_defined).collect();
    assert!(
        missing.is_empty(),
        "the Rust .so is missing {} symbol(s) exported by the C .so: {:?}\n\
         (per SYMBOLS.md these must be real translations, never stubs)",
        missing.len(),
        missing
    );

    // The nine ABI symbols documented in SYMBOLS.md.
    let expected = [
        "convert_pix",
        "cp_inflate",
        "cp_error_reason",
        "cp_fixed_table",
        "cp_permutation_order",
        "cp_len_extra_bits",
        "cp_len_base",
        "cp_dist_extra_bits",
        "cp_dist_base",
    ];
    for s in expected {
        assert!(c_defined.contains(s), "C .so lost {s}?");
        assert!(r_defined.contains(s), "Rust .so does not export {s}");
    }
    assert_eq!(
        c_defined.len(),
        expected.len(),
        "the C .so exports symbols SYMBOLS.md does not list: {:?}",
        c_defined
            .iter()
            .filter(|s| !expected.contains(&s.as_str()))
            .collect::<Vec<_>>()
    );

    // Everything the Rust .so exports beyond the C ABI must be toolchain noise.
    let extra: Vec<&String> = r_defined
        .difference(&c_defined)
        .filter(|s| !is_toolchain_symbol(s))
        .collect();
    assert!(
        extra.is_empty(),
        "the Rust .so exports non-toolchain symbols the C does not: {extra:?}"
    );

    println!("symbol diff is empty ({} ABI symbols)", expected.len());
}

#[test]
fn d_no_unresolved_non_libc_imports() {
    let r = rust_so_path();
    let undefined = nm(&r, "--undefined-only");
    let bad: Vec<&String> = undefined.iter().filter(|s| !is_toolchain_symbol(s)).collect();
    assert!(
        bad.is_empty(),
        "the Rust .so imports non-libc symbols: {bad:?}"
    );
    println!(
        "{} undefined symbols, all libc/unwinder",
        undefined.len()
    );
}

/// Both `.so`s must agree on whether `assert()` is compiled in: the C imports
/// `__assert_fail` exactly when it was built without `NDEBUG`, and the Rust's
/// `c_asserts` feature must be set to match. A mismatch would make every
/// malformed-input comparison meaningless.
#[test]
fn d_assert_configuration_matches() {
    let c_has_asserts = nm(&c_so_path(), "--undefined-only")
        .iter()
        .any(|s| s.starts_with("__assert_fail"));
    let rust_has_asserts = cfg!(feature = "c_asserts");
    println!(
        "C asserts: {c_has_asserts}, Rust c_asserts feature: {rust_has_asserts}"
    );
    assert_eq!(
        c_has_asserts, rust_has_asserts,
        "assert configuration mismatch: the C .so at {} {} `__assert_fail`, but the Rust crate was \
         built {} the `c_asserts` feature.\n\
         Use CP_C_SO to point at the matching C build:\n  \
         default features   -> c_src/build            (no NDEBUG, asserts live)\n  \
         --no-default-features -> c_ndebug_build      (-DNDEBUG, asserts removed)",
        c_so_path().display(),
        if c_has_asserts { "imports" } else { "does not import" },
        if rust_has_asserts { "with" } else { "without" },
    );
}
