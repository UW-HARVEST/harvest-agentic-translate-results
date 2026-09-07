//! Phase D — symbol parity between the C `.so` and the Rust `.so`, enforced as a
//! test so it cannot silently regress.

mod common;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so() -> PathBuf {
    let build = crate_root().join("../c_src/build");
    std::fs::read_dir(&build)
        .unwrap_or_else(|e| panic!("read {}: {e}", build.display()))
        .flatten()
        .map(|e| e.path())
        .find(|p| p.extension().and_then(|s| s.to_str()) == Some("so"))
        .unwrap_or_else(|| panic!("no .so in {}", build.display()))
}

fn rust_so() -> PathBuf {
    let exe = std::env::current_exe().unwrap();
    let profile = exe.parent().and_then(|p| p.parent()).unwrap().to_path_buf();
    let p = profile.join("libmerge_sort_lib.so");
    if p.exists() {
        return p;
    }
    for prof in ["release", "debug"] {
        let q = crate_root().join("target").join(prof).join("libmerge_sort_lib.so");
        if q.exists() {
            return q;
        }
    }
    panic!("libmerge_sort_lib.so not found near {}", profile.display());
}

/// Exported (defined, dynamic) symbol names.
fn defined(path: &Path) -> BTreeSet<String> {
    nm(path, &["-D", "--defined-only"])
}

/// Undefined (imported) symbol names.
fn undefined(path: &Path) -> BTreeSet<String> {
    nm(path, &["-D", "--undefined-only"])
}

fn nm(path: &Path, args: &[&str]) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(args)
        .arg(path)
        .output()
        .unwrap_or_else(|e| panic!("run nm on {}: {e}", path.display()));
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last())
        .map(|s| s.split('@').next().unwrap().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Symbols that legitimately come from libc / libgcc / the CRT and are resolved
/// from the process image for any consumer.
fn is_runtime_symbol(s: &str) -> bool {
    const EXACT: &[&str] = &[
        "_ITM_deregisterTMCloneTable",
        "_ITM_registerTMCloneTable",
        "__cxa_finalize",
        "__cxa_thread_atexit_impl",
        "__errno_location",
        "__gmon_start__",
        "__tls_get_addr",
        "abort",
        "bcmp",
        "calloc",
        "close",
        "dl_iterate_phdr",
        "free",
        "fstat",
        "fstat64",
        "getcwd",
        "getenv",
        "gettid",
        "lseek",
        "lseek64",
        "malloc",
        "memcmp",
        "memcpy",
        "memmove",
        "memset",
        "mmap",
        "mmap64",
        "munmap",
        "open",
        "open64",
        "posix_memalign",
        "pthread_key_create",
        "pthread_key_delete",
        "pthread_getspecific",
        "pthread_setspecific",
        "read",
        "readlink",
        "realloc",
        "realpath",
        "stat",
        "stat64",
        "statx",
        "strlen",
        "syscall",
        "sysconf",
        "write",
        "writev",
    ];
    EXACT.contains(&s) || s.starts_with("_Unwind_") || s.starts_with("__libc_")
}

// ---------------------------------------------------------------------------

#[test]
fn phase_d_every_c_export_is_exported_by_rust() {
    let c = defined(&c_so());
    let r = defined(&rust_so());

    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is MISSING {} symbol(s) exported by the C .so: {missing:?}\n\
         C exports:    {c:?}\n\
         Rust exports: {r:?}",
        missing.len()
    );

    // The C library's single documented export must actually be there.
    assert!(c.contains("merge_sort"), "C .so does not export merge_sort");
    assert!(r.contains("merge_sort"), "Rust .so does not export merge_sort");
}

#[test]
fn phase_d_rust_has_no_non_runtime_undefined_symbols() {
    let u = undefined(&rust_so());
    let leftovers: Vec<&String> = u.iter().filter(|s| !is_runtime_symbol(s)).collect();
    assert!(
        leftovers.is_empty(),
        "Rust .so has non-libc undefined symbols: {leftovers:?}"
    );
}

#[test]
fn phase_d_no_stubs_in_translation() {
    let src = std::fs::read_to_string(crate_root().join("src/lib.rs")).unwrap();
    for bad in ["unimplemented!", "todo!", "unreachable!"] {
        assert!(
            !src.contains(bad),
            "translation/src/lib.rs contains `{bad}` — a stub that lies about \
             behaviour is worse than a missing symbol"
        );
    }
}

#[test]
fn phase_d_both_libraries_load_and_resolve() {
    // Loading through libloading with RTLD_NOW-equivalent resolution would fail
    // if any symbol were unresolvable; `load_pair` also dlsym's merge_sort.
    let p = common::pair();
    assert_eq!(p.c.name, "C");
    assert_eq!(p.rust.name, "Rust");
}

#[test]
fn phase_d_no_binary_targets_to_compare() {
    // The completion gate asks for stdout comparison IF the project builds a
    // binary. Assert mechanically that neither side does, so the exemption in
    // CONFIGS.md is verified rather than assumed.
    let cmake = std::fs::read_to_string(crate_root().join("../c_src/CMakeLists.txt")).unwrap();
    assert!(
        !cmake.contains("add_executable"),
        "c_src builds an executable; the stdout comparison is required"
    );
    let cargo = std::fs::read_to_string(crate_root().join("Cargo.toml")).unwrap();
    assert!(
        !cargo.contains("[[bin]]"),
        "translation declares a [[bin]] target; the stdout comparison is required"
    );
    assert!(
        !crate_root().join("src/main.rs").exists(),
        "translation has src/main.rs; the stdout comparison is required"
    );
}

#[test]
fn phase_d_only_one_feature_combination_exists() {
    // Verifies the CONFIGS.md/SYMBOLS.md claim that there is a single build
    // configuration, so "all feature combinations" is satisfied by the default.
    let cargo = std::fs::read_to_string(crate_root().join("Cargo.toml")).unwrap();
    let has_features = cargo
        .lines()
        .any(|l| l.trim_start().starts_with("[features]"));
    assert!(
        !has_features,
        "Cargo.toml now declares [features]; Phases B and C must be re-run for \
         every combination and CONFIGS.md/SYMBOLS.md updated"
    );
}
