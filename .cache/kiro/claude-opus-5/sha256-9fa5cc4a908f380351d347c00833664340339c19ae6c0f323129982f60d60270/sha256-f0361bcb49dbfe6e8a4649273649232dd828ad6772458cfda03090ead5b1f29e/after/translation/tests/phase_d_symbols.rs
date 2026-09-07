//! Phase D — symbol parity between the C `.so` and the Rust `.so`.
//!
//! Runs `nm -D` on both libraries and requires that every symbol the C exports
//! is also exported by Rust under the exact same name, and that Rust has no
//! undefined non-libc symbols. Also re-checks that every exported symbol is
//! actually resolvable via `dlsym` (a name in `nm` is not proof it is usable).

mod common;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so() -> PathBuf {
    let build = manifest_dir().parent().unwrap().join("c_src/build");
    let mut v: Vec<PathBuf> = std::fs::read_dir(&build)
        .unwrap_or_else(|e| panic!("cannot read {build:?}: {e}"))
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == "so").unwrap_or(false))
        .collect();
    v.sort();
    assert_eq!(v.len(), 1, "expected exactly one C .so in {build:?}, got {v:?}");
    v.pop().unwrap()
}

/// The Rust `cdylib`, via the shared harness so the stale-`.so` guard applies
/// here too (a symbol diff against a stale library would be worthless).
fn rust_so() -> PathBuf {
    common::rust_so_path_checked()
}

/// Dynamic symbols DEFINED by a shared object.
fn defined(path: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", "--format=posix"])
        .arg(path)
        .output()
        .expect("failed to run nm");
    assert!(out.status.success(), "nm failed on {path:?}");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().next())
        .map(str::to_string)
        .collect()
}

/// Dynamic symbols left UNDEFINED (imported) by a shared object.
fn undefined(path: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "-u", "--format=posix"])
        .arg(path)
        .output()
        .expect("failed to run nm");
    assert!(out.status.success(), "nm failed on {path:?}");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().next())
        .map(str::to_string)
        .collect()
}

/// Runtime / toolchain imports that are not project code.
fn is_runtime_import(name: &str) -> bool {
    let base = name.split('@').next().unwrap_or(name);
    base.starts_with("_ITM_")
        || base.starts_with("_Unwind_")
        || base.starts_with("__")
        || name.contains("@GLIBC")
        || name.contains("@GCC")
        || matches!(
            base,
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
                | "memcpy"
                | "memmove"
                | "memset"
                | "mmap64"
                | "munmap"
                | "open64"
                | "posix_memalign"
                | "pthread_key_create"
                | "pthread_key_delete"
                | "pthread_setspecific"
                | "read"
                | "readlink"
                | "realloc"
                | "realpath"
                | "stat64"
                | "statx"
                | "strlen"
                | "syscall"
                | "write"
                | "writev"
        )
}

/// The 12 non-static functions of `c_src/src/lib.c`, listed independently of
/// `nm` so a truncated `nm` run cannot make this test pass vacuously.
const EXPECTED: &[&str] = &[
    "c2V",
    "c2Mulvs",
    "c2Maxv",
    "c2Minv",
    "c2Clampv",
    "c2Sub",
    "c2Dot",
    "c2CircletoCircle",
    "c2CircletoAABB",
    "c2CircletoCapsule",
    "c2Collided",
    "circle_collide",
];

#[test]
fn symbol_parity_c_to_rust() {
    let cpath = c_so();
    let rpath = rust_so();
    let cs = defined(&cpath);
    let rs = defined(&rpath);

    // The C .so must expose exactly the 12 functions we expect (no module of
    // the C source went unnoticed).
    let c_project: BTreeSet<&str> = cs
        .iter()
        .map(String::as_str)
        .filter(|s| !is_runtime_import(s) && !s.starts_with('_'))
        .collect();
    let expected: BTreeSet<&str> = EXPECTED.iter().copied().collect();
    assert_eq!(
        c_project, expected,
        "the C .so's public surface is not what SYMBOLS.md records"
    );

    // Every C symbol must also be exported by Rust, under the exact same name.
    let missing: Vec<&String> = cs.iter().filter(|s| !rs.contains(*s)).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}"
    );

    // No undefined non-libc symbols in the Rust .so.
    let undef = undefined(&rpath);
    let dangling: Vec<&String> = undef.iter().filter(|s| !is_runtime_import(s)).collect();
    assert!(
        dangling.is_empty(),
        "Rust .so has undefined non-libc symbols: {dangling:?}"
    );
}

/// A name in `nm` is not proof the symbol is callable: confirm each one
/// resolves through `dlsym` in BOTH libraries.
#[test]
fn every_symbol_resolves_via_dlsym() {
    for path in [c_so(), rust_so()] {
        let lib = unsafe { libloading::Library::new(&path) }.expect("dlopen failed");
        for name in EXPECTED {
            let mut bytes = name.as_bytes().to_vec();
            bytes.push(0);
            let sym: Result<libloading::Symbol<*const ()>, _> = unsafe { lib.get(&bytes) };
            assert!(sym.is_ok(), "{name} not resolvable in {path:?}");
        }
    }
}
