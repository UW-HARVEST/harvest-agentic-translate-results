//! Phase D — symbol parity between the C `.so` and the Rust `.so`.

mod harness;

use harness::*;
use std::collections::BTreeSet;
use std::process::Command;

fn defined_dynamic_symbols(path: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", "--format=posix"])
        .arg(path)
        .output()
        .expect("failed to run nm");
    assert!(
        out.status.success(),
        "nm failed on {path:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().next().map(String::from))
        .filter(|s| !s.is_empty())
        .collect()
}

fn undefined_dynamic_symbols(path: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--undefined-only", "--format=posix"])
        .arg(path)
        .output()
        .expect("failed to run nm");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().next())
        // strip the `@GLIBC_2.2.5` / `@@VER` symbol-version suffix
        .map(|s| s.split('@').next().unwrap_or(s).to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Every symbol the C `.so` exports must also be exported by the Rust `.so`.
#[test]
fn symbols_c_subset_of_rust() {
    let c = defined_dynamic_symbols(&c_lib());
    let r = defined_dynamic_symbols(&rust_lib());

    assert!(
        c.contains("driver") && c.contains("run"),
        "sanity: C .so should export `driver` and `run`, got {c:?}"
    );

    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by C but MISSING from Rust: {missing:?}\n  C:    {c:?}\n  Rust: {r:?}"
    );
}

/// The `static` C functions must stay internal on both sides.
#[test]
fn static_c_functions_are_not_exported() {
    let c = defined_dynamic_symbols(&c_lib());
    let r = defined_dynamic_symbols(&rust_lib());
    for name in [
        "the_house",
        "add_floor",
        "add_bedrooms",
        "add_floor_to_the_house",
        "print_the_house",
        "parse_val",
    ] {
        assert!(!c.contains(name), "C unexpectedly exports `{name}`");
        assert!(
            !r.contains(name),
            "Rust must not export `{name}` (it is `static` in C)"
        );
    }
}

/// The Rust `.so` must not have unresolved symbols beyond libc / Rust runtime.
#[test]
fn rust_has_no_unresolved_project_symbols() {
    let u = undefined_dynamic_symbols(&rust_lib());
    let allowed_prefixes = ["_Unwind_", "_ITM_", "__", "pthread_"];
    let allowed_libc: &[&str] = &[
        "printf", "puts", "strtol", "malloc", "calloc", "realloc", "free", "posix_memalign",
        "memcpy", "memmove", "memset", "bcmp", "memcmp", "strlen", "abort", "getenv", "getcwd",
        "readlink", "realpath", "open", "open64", "close", "read", "write", "writev", "lseek",
        "lseek64", "stat", "stat64", "fstat", "fstat64", "statx", "mmap", "mmap64", "munmap",
        "syscall", "dl_iterate_phdr", "gettid", "sysconf", "gnu_get_libc_version", "fwrite",
        "fputs", "fflush", "signal", "sigaction", "sigaltstack", "raise", "getpid", "poll",
        "pipe2", "dlsym", "environ", "qsort_r", "strerror_r", "mprotect", "madvise",
    ];
    let bad: Vec<&String> = u
        .iter()
        .filter(|s| {
            !allowed_prefixes.iter().any(|p| s.starts_with(p)) && !allowed_libc.contains(&s.as_str())
        })
        .collect();
    assert!(
        bad.is_empty(),
        "Rust .so has unresolved non-libc symbols (a module may be untranslated): {bad:?}"
    );
}

/// The C project builds no executable driver (CMakeLists only defines a SHARED
/// library), so there is no binary stdout comparison to make. Guard that so the
/// claim stays true if `c_src` ever changes.
#[test]
fn c_project_builds_no_executable() {
    let cmake = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../c_src/CMakeLists.txt"
    ))
    .expect("read CMakeLists.txt");
    assert!(
        !cmake.contains("add_executable"),
        "c_src now builds an executable; add a binary stdout differential test"
    );
    assert!(cmake.contains("add_library(driver SHARED"));
}
