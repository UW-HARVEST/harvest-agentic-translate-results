//! Phase D — symbol parity between the C `.so` and the Rust `.so`.
//!
//! Asserts mechanically (via `nm -D`) what `SYMBOLS.md` documents.

mod common;

use common::{c_so_path, rust_so_path};
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

fn nm(args: &[&str], so: &Path) -> Vec<String> {
    let out = Command::new("nm")
        .args(args)
        .arg(so)
        .output()
        .unwrap_or_else(|e| panic!("failed to run nm: {e}"));
    assert!(
        out.status.success(),
        "nm {args:?} {} failed: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|l| l.to_string())
        .collect()
}

fn defined_symbols(so: &Path) -> BTreeSet<String> {
    nm(&["-D", "--defined-only"], so)
        .into_iter()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .collect()
}

/// Every symbol the C `.so` exports must also be exported by the Rust `.so`.
#[test]
fn every_c_symbol_is_exported_by_rust() {
    let c = defined_symbols(&c_so_path());
    let r = defined_symbols(&rust_so_path());

    assert!(
        c.contains("memchra2"),
        "sanity: the C .so must export memchra2, got {c:?}"
    );

    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}\n\
         C   = {c:?}\nRust = {r:?}"
    );
}

/// With the default feature set the two symbol sets must be *identical* — the
/// Rust `.so` must not add or drop anything.
#[cfg(not(feature = "test_internals"))]
#[test]
fn symbol_sets_are_identical_with_default_features() {
    let c = defined_symbols(&c_so_path());
    let r = defined_symbols(&rust_so_path());
    assert_eq!(
        c, r,
        "default-feature symbol sets diverge:\n  only in C: {:?}\n  only in Rust: {:?}",
        c.difference(&r).collect::<Vec<_>>(),
        r.difference(&c).collect::<Vec<_>>()
    );
}

/// With `test_internals` the Rust `.so` adds exactly the documented `harness_*`
/// wrappers and nothing else.
#[cfg(feature = "test_internals")]
#[test]
fn symbol_sets_differ_only_by_documented_harness_exports() {
    let c = defined_symbols(&c_so_path());
    let r = defined_symbols(&rust_so_path());
    let extra: BTreeSet<String> = r.difference(&c).cloned().collect();
    let expected: BTreeSet<String> = [
        "harness_complex_iteration",
        "harness_count_occurrences",
        "harness_int_to_float_bits",
        "harness_interpret_as_int",
        "harness_memchra",
        "harness_process_buffer",
        "harness_process_strings",
        "harness_safe_sum_array",
        "harness_snprintf_fmt",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    assert_eq!(extra, expected, "unexpected extra Rust exports");
}

/// The Rust `.so` must not need any non-libc symbol: `ldd -r` reports no
/// unresolved relocation.
#[test]
fn rust_so_has_no_unresolved_non_libc_symbols() {
    let so = rust_so_path();
    let out = Command::new("ldd").arg("-r").arg(&so).output();
    let out = match out {
        Ok(o) => o,
        Err(e) => {
            eprintln!("ldd unavailable ({e}); falling back to nm classification");
            return fallback_nm_undefined_check(&so);
        }
    };
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let bad: Vec<&str> = text
        .lines()
        .filter(|l| l.contains("undefined symbol") || l.contains("not found"))
        .collect();
    assert!(
        bad.is_empty(),
        "Rust .so has unresolved symbols:\n{}",
        bad.join("\n")
    );
}

fn fallback_nm_undefined_check(so: &Path) {
    let undefined: Vec<String> = nm(&["-D", "--undefined-only"], so)
        .into_iter()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .map(|s| s.split('@').next().unwrap_or("").to_string())
        .filter(|s| !s.is_empty())
        .collect();
    let libc_like = |s: &str| {
        s.starts_with("__")
            || s.starts_with("_ITM_")
            || s.starts_with("_Unwind_")
            || s.starts_with("pthread_")
            || matches!(
                s,
                "abort"
                    | "bcmp"
                    | "calloc"
                    | "close"
                    | "dl_iterate_phdr"
                    | "free"
                    | "fstat64"
                    | "getcwd"
                    | "getenv"
                    | "gettid"
                    | "lseek64"
                    | "malloc"
                    | "memcmp"
                    | "memcpy"
                    | "memmove"
                    | "memset"
                    | "mmap64"
                    | "munmap"
                    | "open64"
                    | "posix_memalign"
                    | "read"
                    | "readlink"
                    | "realloc"
                    | "realpath"
                    | "snprintf"
                    | "stat64"
                    | "statx"
                    | "strlen"
                    | "strncmp"
                    | "syscall"
                    | "write"
                    | "writev"
            )
    };
    let bad: Vec<&String> = undefined.iter().filter(|s| !libc_like(s)).collect();
    assert!(bad.is_empty(), "non-libc undefined symbols: {bad:?}");
}

/// The exported `memchra2` must be a *global text* symbol in both objects.
#[test]
fn memchra2_is_global_text_in_both() {
    for so in [c_so_path(), rust_so_path()] {
        let lines = nm(&["-D", "--defined-only"], &so);
        let line = lines
            .iter()
            .find(|l| l.split_whitespace().last() == Some("memchra2"))
            .unwrap_or_else(|| panic!("memchra2 not found in {}", so.display()));
        let kind = line.split_whitespace().nth(1).unwrap_or("");
        assert_eq!(kind, "T", "memchra2 in {} has kind {kind}", so.display());
    }
}
