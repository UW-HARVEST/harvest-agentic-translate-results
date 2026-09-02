//! Phase D — symbol parity, enforced as a test rather than a one-off shell diff.
//!
//! Every symbol the C `.so` exports must also be exported by the Rust `.so`
//! under the exact same name. The diff must be empty.

mod common;

use common::*;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Weak toolchain/CRT stubs that appear in every ELF shared object and carry no
/// library semantics.
const TOOLCHAIN_STUBS: &[&str] = &[
    "_ITM_deregisterTMCloneTable",
    "_ITM_registerTMCloneTable",
    "__cxa_finalize",
    "__cxa_thread_atexit_impl",
    "__gmon_start__",
];

fn strip_version(sym: &str) -> &str {
    sym.split('@').next().unwrap_or(sym)
}

fn nm(so: &Path, extra: &[&str]) -> BTreeSet<String> {
    let out = Command::new("nm")
        .arg("-D")
        .args(extra)
        .arg(so)
        .output()
        .unwrap_or_else(|e| panic!("failed to run nm on {}: {e}", so.display()));
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|line| line.split_whitespace().last())
        .map(|s| strip_version(s).to_string())
        .filter(|s| !TOOLCHAIN_STUBS.contains(&s.as_str()))
        .collect()
}

fn c_so() -> PathBuf {
    let dir = workspace_root().join("c_src").join("build");
    let mut found: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", dir.display()))
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.is_file()
                && p.file_name()
                    .map(|n| {
                        let n = n.to_string_lossy();
                        n.starts_with("lib") && n.ends_with(".so")
                    })
                    .unwrap_or(false)
        })
        .collect();
    found.sort();
    found
        .into_iter()
        .next()
        .unwrap_or_else(|| panic!("no C .so in {}", dir.display()))
}

fn rust_so() -> PathBuf {
    const SO: &str = "libhsl_to_rgb_lib.so";
    if let Ok(exe) = std::env::current_exe()
        && let Some(deps) = exe.parent()
    {
        for dir in [Some(deps), deps.parent()].into_iter().flatten() {
            let candidate = dir.join(SO);
            if candidate.is_file() {
                return candidate;
            }
        }
    }
    let target = workspace_root().join("translation").join("target");
    for profile in ["release", "debug"] {
        let candidate = target.join(profile).join(SO);
        if candidate.is_file() {
            return candidate;
        }
    }
    panic!("could not locate {SO}");
}

/// Every exported symbol of the C `.so` must be exported by the Rust `.so`.
/// The set difference must be EMPTY.
#[test]
fn symbols_c_exports_are_all_present_in_rust() {
    let c = nm(&c_so(), &["--defined-only"]);
    let rust = nm(&rust_so(), &["--defined-only"]);

    assert!(
        !c.is_empty(),
        "nm found no exported symbols in the C .so — is it built?"
    );

    let missing: Vec<&String> = c.difference(&rust).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing {} symbol(s) exported by the C .so: {missing:?}\n\
         C exports:    {c:?}\n\
         Rust exports: {rust:?}",
        missing.len()
    );
}

/// The one documented entry point must be present, so the test above cannot
/// pass vacuously if `nm` output parsing ever breaks.
#[test]
fn symbols_hsl_to_rgb_is_exported_by_both() {
    let c = nm(&c_so(), &["--defined-only"]);
    let rust = nm(&rust_so(), &["--defined-only"]);
    assert!(c.contains("hsl_to_rgb"), "C .so must export hsl_to_rgb");
    assert!(
        rust.contains("hsl_to_rgb"),
        "Rust .so must export hsl_to_rgb"
    );
}

/// The Rust `.so` must not leave any non-runtime symbol undefined. Everything it
/// imports has to be libc or the libgcc unwinder that the Rust standard library
/// pulls in for a `cdylib`.
#[test]
fn symbols_rust_has_no_unresolved_non_runtime_imports() {
    let undefined = nm(&rust_so(), &["-u"]);

    let is_runtime = |s: &str| {
        s.starts_with("_Unwind_")
            || s.starts_with("__")
            || s.starts_with("pthread_")
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
                    | "fmodf"
                    | "fmod"
                    | "fabsf"
                    | "sysconf"
                    | "getauxval"
            )
    };

    let unexpected: Vec<&String> = undefined.iter().filter(|s| !is_runtime(s)).collect();
    assert!(
        unexpected.is_empty(),
        "Rust .so has unexpected undefined symbols: {unexpected:?}"
    );
}

/// Sanity: the harness really did load two DISTINCT shared objects, so a
/// mis-resolved path cannot make the whole differential suite compare a library
/// against itself.
#[test]
fn symbols_harness_loaded_two_distinct_libraries() {
    let p = pair();
    let c_addr = p.c as usize;
    let rust_addr = p.rust as usize;
    assert_ne!(
        c_addr, rust_addr,
        "the C and Rust hsl_to_rgb resolved to the same address — the differential \
         tests would be comparing one library against itself"
    );
}
