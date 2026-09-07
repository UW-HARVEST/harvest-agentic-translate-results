//! Phase D — symbol parity gate, enforced as a test.
//!
//! Asserts mechanically (via `nm -D`) that the Rust `.so` exports exactly the
//! symbol set the C `.so` exports, and that no non-libc symbol is left
//! undefined in the Rust `.so`.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

fn nm(path: &Path, kind: &str) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", kind, "--format=posix", path.to_str().unwrap()])
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
        .map(|s| s.split('@').next().unwrap().to_string())
        .collect()
}

/// Symbols the Rust standard library / unwinder / allocator always import.
/// Anything undefined in the Rust `.so` must be one of these (or a libc symbol
/// the C also imports) — otherwise it is an unresolved reference to code that
/// was never translated.
fn is_platform_symbol(s: &str) -> bool {
    s.starts_with("_Unwind_")
        || s.starts_with("__")
        || s.starts_with("_ITM_")
        || matches!(
            s,
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
                | "printf"
                | "pthread_key_create"
                | "pthread_key_delete"
                | "pthread_getspecific"
                | "pthread_setspecific"
                | "puts"
                | "read"
                | "readlink"
                | "realloc"
                | "realpath"
                | "snprintf"
                | "stat"
                | "stat64"
                | "statx"
                | "strlen"
                | "strncmp"
                | "syscall"
                | "sysconf"
                | "write"
                | "writev"
        )
}

/// The exported-symbol diff between the two `.so`s MUST be empty.
#[test]
fn d1_exported_symbol_parity() {
    let l = libs();
    let c = nm(&l.c.path, "--defined-only");
    let r = nm(&l.rust.path, "--defined-only");

    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing {} symbol(s) exported by the C .so: {:?}\n\
         Per Phase A: add the #[no_mangle] wrapper if the impl exists, or \
         translate the missing C module.",
        missing.len(),
        missing
    );

    // Sanity: the three documented entry points really are there.
    for want in ["cleanup", "print_result", "cleanup_resources"] {
        assert!(c.contains(want), "C .so unexpectedly lacks `{want}`");
        assert!(r.contains(want), "Rust .so lacks `{want}`");
    }

    let extra: Vec<&String> = r.difference(&c).collect();
    assert!(
        extra.is_empty(),
        "Rust .so exports symbols the C .so does not: {extra:?}"
    );
}

/// No undefined symbol in the Rust `.so` may be anything other than a libc /
/// platform-runtime import.
#[test]
fn d2_no_unresolved_translated_symbols() {
    let l = libs();
    let undef = nm(&l.rust.path, "--undefined-only");
    let unexpected: Vec<&String> = undef
        .iter()
        .filter(|s| !is_platform_symbol(s))
        .collect();
    assert!(
        unexpected.is_empty(),
        "Rust .so has non-libc undefined symbol(s) — untranslated code: {unexpected:?}"
    );
}

/// The whole public API of the C `.so` is reachable through `dlsym` on the
/// Rust `.so` with a C-compatible signature (this is what the export wrappers
/// exist for; `libs()` panics if any lookup fails).
#[test]
fn d3_all_symbols_callable_through_dlsym() {
    let l = libs();
    let (v, out) = capture_stdout(|| unsafe { (l.rust.cleanup)(10, 20, 30, 40) });
    let (vc, outc) = capture_stdout(|| unsafe { (l.c.cleanup)(10, 20, 30, 40) });
    assert_eq!(v, vc);
    assert_eq!(out, outc);
    assert_eq!(v, 30 + 20 + 70 + 40, "fall-through arithmetic changed");
}
