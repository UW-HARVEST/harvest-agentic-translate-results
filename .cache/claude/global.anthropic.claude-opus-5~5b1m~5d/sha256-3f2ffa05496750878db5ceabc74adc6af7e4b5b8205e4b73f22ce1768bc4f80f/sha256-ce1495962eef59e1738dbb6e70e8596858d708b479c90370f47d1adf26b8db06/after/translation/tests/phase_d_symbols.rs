//! Phase D — symbol parity between the C `.so` and the Rust `.so`.
//!
//! Runs `nm -D` on both shared objects and requires the symbol diff to be empty:
//! every symbol the C library *defines* must also be defined by the Rust
//! library, under the exact same name.

mod common;
use common::*;

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

/// Parse `nm -D` output into (defined symbols, undefined symbols).
///
/// `nm -D` lines look like:
///   `0000000000001139 T get_predict_func`
///   `                 U memcpy@GLIBC_2.14`
///   `                 w __gmon_start__`
fn nm(path: &Path) -> (BTreeSet<String>, BTreeSet<String>) {
    let out = Command::new("nm")
        .arg("-D")
        .arg(path)
        .output()
        .unwrap_or_else(|e| panic!("failed to run nm on {}: {e}", path.display()));
    assert!(
        out.status.success(),
        "nm -D {} failed: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8_lossy(&out.stdout).into_owned();

    let mut defined = BTreeSet::new();
    let mut undefined = BTreeSet::new();
    for line in text.lines() {
        let mut it = line.split_whitespace();
        let (kind, name) = match (it.next(), it.next(), it.next()) {
            // "<addr> <kind> <name>"
            (Some(_addr), Some(k), Some(n)) => (k.to_string(), n.to_string()),
            // "<kind> <name>"  (undefined / weak-undefined: no address)
            (Some(k), Some(n), None) => (k.to_string(), n.to_string()),
            _ => continue,
        };
        // Strip the @GLIBC_x.y / @@GLIBC_x.y version suffix.
        let base = name.split('@').next().unwrap_or(&name).to_string();
        match kind.as_str() {
            "U" | "w" | "v" => {
                undefined.insert(base);
            }
            _ => {
                defined.insert(base);
            }
        }
    }
    (defined, undefined)
}

/// Symbols that come from the C runtime / linker glue rather than from the
/// translated source.  They are not part of the library's API surface and are
/// present or absent purely as a function of the toolchain.
fn is_toolchain_symbol(name: &str) -> bool {
    matches!(
        name,
        "_init"
            | "_fini"
            | "__bss_start"
            | "_edata"
            | "_end"
            | "_ITM_deregisterTMCloneTable"
            | "_ITM_registerTMCloneTable"
            | "__cxa_finalize"
            | "__gmon_start__"
            | "__cxa_thread_atexit_impl"
            | "__tls_get_addr"
    ) || name.starts_with("_rust_")
        || name.starts_with("rust_")
        || name.starts_with("__rust")
        || name.starts_with("_ZN")
}

/// Symbols the Rust library is *allowed* to export in addition to the C set.
/// Only the feature-gated test hooks qualify; they are absent from the default
/// build, so the default build's surface is an exact match.
fn is_permitted_rust_extra(name: &str) -> bool {
    name.starts_with("__difftest_")
}

#[test]
fn every_c_symbol_is_exported_by_rust() {
    let l = libs();
    let (c_def, _c_undef) = nm(&l.c_so);
    let (r_def, _r_undef) = nm(&l.rust_so);

    let c_api: BTreeSet<_> = c_def
        .iter()
        .filter(|s| !is_toolchain_symbol(s))
        .cloned()
        .collect();
    let r_api: BTreeSet<_> = r_def
        .iter()
        .filter(|s| !is_toolchain_symbol(s))
        .cloned()
        .collect();

    assert!(
        !c_api.is_empty(),
        "sanity: the C .so must define at least one API symbol (got {c_def:?})"
    );
    assert!(
        c_api.contains("get_predict_func"),
        "sanity: the C .so must define get_predict_func, got {c_api:?}"
    );

    let missing: Vec<_> = c_api.difference(&r_api).cloned().collect();
    assert!(
        missing.is_empty(),
        "Rust .so ({}) is MISSING {} symbol(s) that the C .so ({}) defines: {:?}",
        l.rust_so.display(),
        missing.len(),
        l.c_so.display(),
        missing
    );

    let extra: Vec<_> = r_api
        .difference(&c_api)
        .filter(|s| !is_permitted_rust_extra(s))
        .cloned()
        .collect();
    assert!(
        extra.is_empty(),
        "Rust .so exports unexpected extra symbol(s) not present in the C .so: {extra:?}"
    );

    eprintln!(
        "symbol parity OK: {} C API symbol(s) all present in Rust: {:?}",
        c_api.len(),
        c_api
    );
}

/// No unresolved non-libc symbols in the Rust `.so`: everything it imports must
/// come from the C runtime, not from a Rust module that was never translated.
#[test]
fn rust_so_has_no_unresolved_non_libc_symbols() {
    let l = libs();
    let (_def, undef) = nm(&l.rust_so);

    // Everything the Rust cdylib may legitimately import from the platform.
    let libc_ok = |n: &str| {
        is_toolchain_symbol(n)
            || n.starts_with("__libc_")
            || n.starts_with("__errno")
            || n.starts_with("pthread_")
            || matches!(
                n,
                "memcpy"
                    | "memmove"
                    | "memset"
                    | "memcmp"
                    | "bcmp"
                    | "strlen"
                    | "malloc"
                    | "free"
                    | "calloc"
                    | "realloc"
                    | "posix_memalign"
                    | "abort"
                    | "write"
                    | "writev"
                    | "dl_iterate_phdr"
                    | "_Unwind_Resume"
                    | "_Unwind_Backtrace"
                    | "_Unwind_GetIP"
                    | "_Unwind_GetIPInfo"
                    | "_Unwind_GetLanguageSpecificData"
                    | "_Unwind_GetRegionStart"
                    | "_Unwind_GetTextRelBase"
                    | "_Unwind_GetDataRelBase"
                    | "_Unwind_SetGR"
                    | "_Unwind_SetIP"
                    | "_Unwind_RaiseException"
                    | "_Unwind_DeleteException"
                    | "_Unwind_GetCFA"
                    | "__gxx_personality_v0"
                    | "gnu_get_libc_version"
                    | "getauxval"
                    | "sysconf"
                    | "syscall"
                    | "open64"
                    | "close"
                    | "read"
                    | "readlink"
                    | "mmap"
                    | "munmap"
                    | "mprotect"
                    | "sigaction"
                    | "sigaltstack"
                    | "__tunable_get_val"
                    // pulled in by Rust std's io/fs/backtrace machinery
                    | "fstat64"
                    | "stat64"
                    | "lstat64"
                    | "statx"
                    | "lseek64"
                    | "mmap64"
                    | "open"
                    | "openat64"
                    | "getcwd"
                    | "getenv"
                    | "gettid"
                    | "realpath"
                    | "signal"
                    | "raise"
                    | "poll"
                    | "pipe2"
            )
    };

    let bad: Vec<_> = undef.iter().filter(|n| !libc_ok(n)).cloned().collect();
    assert!(
        bad.is_empty(),
        "Rust .so has unresolved non-libc symbol(s) — a module may be untranslated: {bad:?}"
    );
}

/// The public API symbol must be reachable by `dlsym` on both objects and must
/// have the same observable behaviour — the ultimate parity check.
#[test]
fn exported_symbol_is_callable_on_both() {
    let c = c_get_predict_func();
    let r = rust_get_predict_func();
    for pfcn in -64..=64 {
        assert_eq!(
            unsafe { c(pfcn) },
            unsafe { r(pfcn) },
            "dlsym'd get_predict_func diverges at pfcn={pfcn}"
        );
    }
}

/// Under the `difftest` feature the Rust `.so` gains exactly the two test hooks
/// and nothing else; without the feature it gains nothing.
#[test]
fn feature_gated_extras_are_exactly_as_declared() {
    let l = libs();
    let (r_def, _) = nm(&l.rust_so);
    let extras: BTreeSet<_> = r_def
        .iter()
        .filter(|s| s.starts_with("__difftest_"))
        .cloned()
        .collect();
    if difftest_feature_enabled() {
        let want: BTreeSet<String> = [
            "__difftest_call_selected",
            "__difftest_layout",
            "__difftest_predict",
            "__difftest_selector",
        ]
        .into_iter()
        .map(String::from)
        .collect();
        assert_eq!(extras, want, "difftest feature hook set mismatch");
    } else {
        assert!(
            extras.is_empty(),
            "default build must export no test hooks, found {extras:?}"
        );
    }
}
