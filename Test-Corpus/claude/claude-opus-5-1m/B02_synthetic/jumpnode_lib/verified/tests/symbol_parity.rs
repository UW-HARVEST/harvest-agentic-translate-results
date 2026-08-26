//! Phase D — symbol parity between the C `.so` and the Rust `.so`.

mod common;

use std::path::PathBuf;
use std::process::Command;

/// ELF/toolchain bookkeeping that is not part of the C API surface.
fn is_bookkeeping(name: &str) -> bool {
    matches!(
        name,
        "_init"
            | "_fini"
            | "__bss_start"
            | "_edata"
            | "_end"
            | "__gmon_start__"
            | "_ITM_registerTMCloneTable"
            | "_ITM_deregisterTMCloneTable"
            | "__cxa_finalize"
            | "__cxa_thread_atexit_impl"
    ) || name.starts_with("_ZN")          // Rust mangled internals
        || name.starts_with("_RN")        // Rust v0 mangling
        || name.starts_with("__rust")
        || name.starts_with("rust_")
        || name.starts_with("_$LT$")
}

fn defined_exports(so: &PathBuf) -> Vec<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", "--format=posix"])
        .arg(so)
        .output()
        .expect("failed to run `nm` (binutils required for symbol-parity test)");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8_lossy(&out.stdout);
    let mut syms: Vec<String> = text
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let name = it.next()?;
            let kind = it.next()?;
            // Only global text/data definitions form the callable API surface.
            if matches!(kind, "T" | "D" | "B" | "R" | "W" | "i") && !is_bookkeeping(name) {
                Some(name.to_string())
            } else {
                None
            }
        })
        .collect();
    syms.sort();
    syms.dedup();
    syms
}

/// Resolve both `.so`s through the shared harness, so this test inspects exactly
/// the same artifacts the differential tests call into (and so the Rust cdylib is
/// built for this binary's feature set rather than picked up stale from disk).
fn c_so() -> PathBuf {
    common::c_so_path_for_tests()
}

fn rust_so() -> PathBuf {
    common::rust_so_path_for_tests()
}

#[test]
fn phase_d_every_c_symbol_is_exported_by_rust() {
    let c_syms = defined_exports(&c_so());
    let r_syms = defined_exports(&rust_so());

    assert!(
        c_syms.contains(&"jumpnode".to_string()),
        "sanity: C .so must export `jumpnode`, got {c_syms:?}"
    );

    let missing: Vec<&String> = c_syms.iter().filter(|s| !r_syms.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is MISSING {} C symbol(s): {missing:?}\n  C exports:    {c_syms:?}\n  Rust exports: {r_syms:?}",
        missing.len()
    );
}

/// In the default configuration the Rust `.so` must export EXACTLY the same
/// symbol set as the C `.so` — no extras. This is what guarantees the test-only
/// `shadow_probe` feature cannot leak into a normal build.
#[cfg(not(feature = "shadow_probe"))]
#[test]
fn phase_d_default_build_symbol_set_is_exactly_the_c_set() {
    let c_syms = defined_exports(&c_so());
    let r_syms = defined_exports(&rust_so());
    assert_eq!(
        r_syms, c_syms,
        "default build must export exactly the C symbol set\n  C:    {c_syms:?}\n  Rust: {r_syms:?}"
    );
    assert_eq!(r_syms, vec!["jumpnode".to_string()]);
}

/// With the probe feature on, the extra symbols must all be `probe_*` — the
/// real API surface is still a superset of the C's, never a different one.
#[cfg(feature = "shadow_probe")]
#[test]
fn phase_d_probe_build_adds_only_probe_symbols() {
    let c_syms = defined_exports(&c_so());
    let r_syms = defined_exports(&rust_so());
    for s in &c_syms {
        assert!(r_syms.contains(s), "probe build lost C symbol `{s}`");
    }
    for s in &r_syms {
        assert!(
            c_syms.contains(s) || s.starts_with("probe_"),
            "probe build exports unexpected symbol `{s}`"
        );
    }
}

#[test]
fn phase_d_rust_does_not_export_c_internal_statics() {
    // Every `static` function/object in lib.c has internal linkage and must not
    // leak out of the Rust cdylib either.
    let r_syms = defined_exports(&rust_so());
    for internal in [
        "find_node_by_id",
        "add_node",
        "process_backward",
        "compute_size_metric",
        "safe_double_to_int",
        "initialize_test_data",
        "node_storage",
        "node_count",
        "NODE_STORAGE",
        "NODE_COUNT",
    ] {
        assert!(
            !r_syms.iter().any(|s| s == internal),
            "Rust .so exports `{internal}`, but it is `static` (internal linkage) in lib.c"
        );
    }
}

#[test]
fn phase_d_rust_has_no_undefined_non_libc_symbols() {
    let out = Command::new("nm")
        .args(["-D", "--undefined-only", "--format=posix"])
        .arg(rust_so())
        .output()
        .expect("failed to run nm");
    let text = String::from_utf8_lossy(&out.stdout);
    let bad: Vec<&str> = text
        .lines()
        .filter_map(|l| l.split_whitespace().next())
        .filter(|n| {
            let base = n.split('@').next().unwrap_or(n);
            !(is_bookkeeping(base)
                || base.starts_with("_Unwind_")   // libgcc_s unwinder
                || base.starts_with("__")         // glibc internals
                || base.starts_with("pthread_")
                || KNOWN_LIBC.contains(&base))
        })
        .collect();
    assert!(
        bad.is_empty(),
        "Rust .so has undefined non-libc symbols: {bad:?}"
    );
}

const KNOWN_LIBC: &[&str] = &[
    "abort", "bcmp", "calloc", "close", "dl_iterate_phdr", "free", "fstat", "fstat64", "getcwd",
    "getenv", "gettid", "lseek", "lseek64", "malloc", "memcmp", "memcpy", "memmove", "memset",
    "mmap", "mmap64", "munmap", "open", "open64", "posix_memalign", "read", "readlink", "realloc",
    "realpath", "sprintf", "sqrt", "stat", "stat64", "statx", "strlen", "syscall", "write",
    "writev", "sysconf", "getauxval", "poll", "sigaltstack", "sigaction", "mprotect",
    "pthread_self", "pthread_getattr_np", "pthread_attr_getstack", "pthread_attr_destroy",
    "environ", "qsort", "signal", "raise", "dlsym",
];

#[test]
fn phase_d_symbol_is_callable_through_dlopen() {
    // Loading both and resolving `jumpnode` proves the export wrapper works.
    let p = common::pair();
    let cf = p.c();
    let rf = p.rust();
    let c_val = unsafe { cf(3, 1, 2, 3) };
    let r_val = unsafe { rf(3, 1, 2, 3) };
    assert_eq!(c_val, r_val);
}
