//! Phase D — symbol parity, enforced as a test so it cannot silently regress.
//!
//! Asserts that every symbol exported by the C `.so` is also exported by the
//! Rust `.so` under the exact same name, and that the Rust `.so` has no
//! undefined symbols beyond libc / libgcc-unwind.

mod common;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

fn c_so() -> PathBuf {
    let build = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("c_src")
        .join("build");
    let mut v: Vec<PathBuf> = std::fs::read_dir(&build)
        .expect("c_src/build exists (build the C library first)")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            let n = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
            n.starts_with("lib") && n.ends_with(".so") && p.is_file()
        })
        .collect();
    v.sort();
    assert_eq!(v.len(), 1, "expected 1 C .so, found {v:?}");
    v.pop().unwrap()
}

fn rust_so() -> PathBuf {
    let exe = std::env::current_exe().unwrap();
    let p = exe
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("libdiv_euclid_lib.so");
    assert!(p.is_file(), "missing {}", p.display());
    p
}

/// `nm -D --defined-only <so>` -> set of symbol names.
fn defined_symbols(so: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(so)
        .output()
        .expect("`nm` must be on PATH");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(str::to_string))
        .collect()
}

fn undefined_symbols(so: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--undefined-only"])
        .arg(so)
        .output()
        .expect("`nm` must be on PATH");
    assert!(out.status.success(), "nm failed on {}", so.display());
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(str::to_string))
        .map(|s| s.split('@').next().unwrap_or("").to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

#[test]
fn phase_d_every_c_symbol_is_exported_by_rust() {
    let c = defined_symbols(&c_so());
    let r = defined_symbols(&rust_so());

    assert!(
        c.contains("div_euclid"),
        "sanity: C .so must export div_euclid, got {c:?}"
    );

    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing {} C symbol(s): {missing:?}\nC exports:   {c:?}\nRust exports: {r:?}",
        missing.len()
    );
}

#[test]
fn phase_d_rust_has_no_untranslated_undefined_symbols() {
    let undef = undefined_symbols(&rust_so());

    // Everything Rust's std/panic runtime legitimately imports.
    let allowed_prefixes = [
        "_Unwind_",
        "__cxa_",
        "__libc_",
        "__tls_",
        "__errno",
        "__gmon_start__",
        "_ITM_",
        "pthread_",
        "std::",
        "_rust",
        "rust_",
    ];
    let allowed_exact: BTreeSet<&str> = [
        "abort", "bcmp", "calloc", "close", "dl_iterate_phdr", "free", "fstat64",
        "getcwd", "getenv", "gettid", "lseek64", "malloc", "memcmp", "memcpy",
        "memmove", "memset", "mmap64", "munmap", "open64", "posix_memalign",
        "read", "readlink", "realloc", "realpath", "stat64", "statx", "strlen",
        "syscall", "write", "writev", "sigaction", "sigaltstack", "sysconf",
        "environ", "__environ", "dlsym", "dladdr", "poll", "getrandom",
    ]
    .into_iter()
    .collect();

    let unexpected: Vec<&String> = undef
        .iter()
        .filter(|s| {
            !allowed_exact.contains(s.as_str())
                && !allowed_prefixes.iter().any(|p| s.starts_with(p))
        })
        .collect();

    assert!(
        unexpected.is_empty(),
        "Rust .so has {} undefined non-libc symbol(s) (suggests untranslated C): {unexpected:?}",
        unexpected.len()
    );
}

#[test]
fn phase_d_rust_so_is_loadable_and_symbol_is_callable_via_dlsym() {
    // Proves the #[no_mangle] wrapper is reachable exactly as an external C
    // caller would reach it (dlopen + dlsym), not via Rust linkage.
    let (c, r) = common::funcs();
    for &(a, b) in &[(7i32, 2i32), (-7, 2), (7, -2), (-7, -2), (0, 0)] {
        let cv = unsafe { c(a, b) };
        let rv = unsafe { r(a, b) };
        assert_eq!(cv, rv, "div_euclid({a}, {b}) C={cv} Rust={rv}");
    }
}
