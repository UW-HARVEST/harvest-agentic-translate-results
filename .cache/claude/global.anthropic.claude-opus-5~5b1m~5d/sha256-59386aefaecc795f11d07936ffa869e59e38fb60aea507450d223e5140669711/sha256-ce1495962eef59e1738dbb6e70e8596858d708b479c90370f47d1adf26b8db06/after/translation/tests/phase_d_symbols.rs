//! Phase D — symbol parity, enforced as a test rather than a one-off command.
//!
//! Asserts that every dynamic symbol the C `.so` exports is exported by the
//! Rust `.so` under the exact same name, and that the Rust `.so` leaves no
//! non-libc symbol undefined.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

fn nm(path: &Path, extra: &str) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", extra])
        .arg(path)
        .output()
        .expect("`nm` must be available to check symbol parity");
    assert!(
        out.status.success(),
        "nm -D {extra} {} failed: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect()
}

/// Symbols the Rust runtime legitimately imports: libc, the libgcc\_s
/// unwinder, and the weak CRT/ITM hooks the C `.so` imports as well.
fn is_runtime_import(sym: &str) -> bool {
    let base = sym.split('@').next().unwrap_or(sym);
    const EXACT: &[&str] = &[
        // weak CRT / ITM hooks (present in the C .so too)
        "_ITM_deregisterTMCloneTable",
        "_ITM_registerTMCloneTable",
        "__cxa_finalize",
        "__gmon_start__",
        "__cxa_thread_atexit_impl",
        // libc
        "__errno_location", "__tls_get_addr", "abort", "bcmp", "calloc", "close",
        "dl_iterate_phdr", "free", "fstat64", "getcwd", "getenv", "gettid",
        "lseek64", "malloc", "memcpy", "memmove", "memset", "mmap64", "munmap",
        "open64", "posix_memalign", "pthread_key_create", "pthread_key_delete",
        "pthread_setspecific", "read", "readlink", "realloc", "realpath",
        "stat64", "statx", "strlen", "syscall", "write", "writev",
    ];
    EXACT.contains(&base) || base.starts_with("_Unwind_")
}

/// The C library's complete export set, derived from its single translation
/// unit (`add_library(... src/lib.c)`) and its 1-line public header.
const EXPECTED_C_EXPORTS: &[&str] = &["rgb_to_hsv"];

#[test]
fn sym_c_export_set_is_as_documented() {
    let c = nm(&c_so(), "--defined-only");
    let expected: BTreeSet<String> = EXPECTED_C_EXPORTS.iter().map(|s| s.to_string()).collect();
    assert_eq!(
        c, expected,
        "the C .so's export set changed; SYMBOLS.md and the test surface must be revisited"
    );
}

/// The Phase D gate: the symbol diff must be EMPTY.
#[test]
fn sym_every_c_export_is_exported_by_rust() {
    let c = nm(&c_so(), "--defined-only");
    let r = nm(&rust_so(), "--defined-only");

    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is MISSING {} symbol(s) the C .so exports: {:?}\n\
         Per the Phase A rule these must be exported (if implemented) or the \
         missing C module must be translated -- never stubbed.",
        missing.len(),
        missing
    );

    // Report (without failing on) any extra exports, so unexpected surface is visible.
    let extra: Vec<&String> = r.difference(&c).collect();
    eprintln!(
        "symbol parity: {} C exports, {} Rust exports, 0 missing, {} extra {:?}",
        c.len(),
        r.len(),
        extra.len(),
        extra
    );
}

/// Also check the uninstrumented (release) artifact, which is what ships.
#[test]
fn sym_parity_holds_for_release_artifact() {
    let c = nm(&c_so(), "--defined-only");
    let r = nm(&rust_so_nochecks(), "--defined-only");
    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "release Rust .so is missing: {missing:?}"
    );
}

/// No unresolved non-libc symbol may remain in the Rust `.so`.
#[test]
fn sym_no_undefined_non_libc_symbols_in_rust() {
    for path in [rust_so(), rust_so_nochecks()] {
        let undef = nm(&path, "--undefined-only");
        let offenders: Vec<&String> = undef.iter().filter(|s| !is_runtime_import(s)).collect();
        assert!(
            offenders.is_empty(),
            "{} has {} undefined NON-libc symbol(s): {:?}",
            path.display(),
            offenders.len(),
            offenders
        );
        eprintln!(
            "{}: {} undefined symbols, all libc/unwinder",
            path.display(),
            undef.len()
        );
    }
}

/// The exported symbol must be a real, callable implementation — not a stub.
/// A stub that returns without writing `dest`, or that always writes the same
/// value, would leave the canary in place or fail to vary with the input.
#[test]
fn sym_rust_export_is_not_a_stub() {
    let (_c, rust) = both();
    let a = call_bits(rust.f, &[1.0, 0.0, 0.0]);
    let b = call_bits(rust.f, &[0.0, 1.0, 0.0]);
    let c3 = call_bits(rust.f, &[0.25, 0.5, 0.75]);
    for (i, out) in [a, b, c3].iter().enumerate() {
        for (lane, &bits) in out.iter().enumerate() {
            assert_ne!(
                bits, CANARY,
                "vector {i}: lane {lane} was never written -- exported symbol is a stub"
            );
        }
    }
    assert_ne!(a, b, "output does not depend on the input -- stub?");
    assert_ne!(b, c3, "output does not depend on the input -- stub?");
    // Spot-check the documented semantics (pure red -> h=0, s=1, v=1).
    assert_eq!(f32::from_bits(a[0]), 0.0);
    assert_eq!(f32::from_bits(a[1]), 1.0);
    assert_eq!(f32::from_bits(a[2]), 1.0);
    // Pure green -> h=120.
    assert_eq!(f32::from_bits(b[0]), 120.0);
}
