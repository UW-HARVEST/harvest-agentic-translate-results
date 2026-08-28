//! Phase D — symbol parity between the C `.so` and the Rust `cdylib`.
//!
//! Asserts mechanically (via `nm -D`) that every symbol the C shared library
//! exports is also exported, under the exact same name, by the Rust shared
//! library, and that the Rust library has no unresolved non-libc imports.

mod common;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn find_so(dir: &Path, prefix: &str) -> Option<PathBuf> {
    let mut found: Vec<PathBuf> = std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            let n = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            n.starts_with(prefix) && n.ends_with(".so")
        })
        .collect();
    found.sort();
    found.into_iter().next()
}

fn c_so() -> PathBuf {
    let dir = crate_root().parent().unwrap().join("c_src").join("build");
    find_so(&dir, "lib").unwrap_or_else(|| panic!("no C .so in {}", dir.display()))
}

/// Profile-strict, exactly like `common::rust_so_path` — see the note there.
fn rust_so() -> PathBuf {
    if let Ok(p) = std::env::var("BIN2HEX_RUST_SO") {
        return PathBuf::from(p);
    }
    let exe = std::env::current_exe().unwrap();
    let dir = exe.parent().and_then(|d| d.parent()).unwrap().to_path_buf();
    let p = dir.join("libbin2hex_lib.so");
    assert!(
        p.exists(),
        "cdylib for this profile missing: {} (run `cargo build` / `cargo build --release` first)",
        p.display()
    );
    p
}

/// Linker/runtime housekeeping entries that are not part of any library's API.
fn is_housekeeping(name: &str) -> bool {
    matches!(
        name,
        "_ITM_deregisterTMCloneTable"
            | "_ITM_registerTMCloneTable"
            | "__cxa_finalize"
            | "__cxa_thread_atexit_impl"
            | "__gmon_start__"
            | "_init"
            | "_fini"
            | "__bss_start"
            | "_edata"
            | "_end"
            | "gettid"
            | "statx"
    )
}

fn nm(path: &Path, extra: &[&str]) -> Vec<(String, String)> {
    let mut cmd = Command::new("nm");
    cmd.arg("-D");
    for e in extra {
        cmd.arg(e);
    }
    cmd.arg(path);
    let out = cmd.output().expect("failed to run `nm`");
    assert!(
        out.status.success(),
        "nm -D {} failed: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let toks: Vec<&str> = l.split_whitespace().collect();
            match toks.len() {
                2 => Some((toks[0].to_string(), toks[1].to_string())), // "U name"
                3 => Some((toks[1].to_string(), toks[2].to_string())), // "addr T name"
                _ => None,
            }
        })
        // strip @GLIBC_x.y version suffixes
        .map(|(k, n)| (k, n.split('@').next().unwrap().to_string()))
        .collect()
}

fn defined(path: &Path) -> BTreeSet<String> {
    nm(path, &["--defined-only"])
        .into_iter()
        .filter(|(kind, name)| {
            // Only real, strong, exported definitions.
            !matches!(kind.as_str(), "w" | "v") && !is_housekeeping(name)
        })
        .map(|(_, n)| n)
        .collect()
}

fn undefined(path: &Path) -> BTreeSet<String> {
    nm(path, &["--undefined-only"])
        .into_iter()
        .filter(|(kind, name)| kind == "U" && !is_housekeeping(name))
        .map(|(_, n)| n)
        .collect()
}

/// Every symbol imported by the Rust `.so` must come from libc / the platform
/// runtime, i.e. none of them may be a symbol the translation forgot to define.
fn is_platform_import(name: &str) -> bool {
    if name.starts_with("_Unwind_")
        || name.starts_with("pthread_")
        || name.starts_with("__")
        || name.starts_with("dl_")
    {
        return true;
    }
    matches!(
        name,
        "abort"
            | "bcmp"
            | "calloc"
            | "close"
            | "free"
            | "fstat"
            | "fstat64"
            | "getcwd"
            | "getenv"
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
            | "read"
            | "readlink"
            | "realloc"
            | "realpath"
            | "stat"
            | "stat64"
            | "strlen"
            | "syscall"
            | "write"
            | "writev"
            | "sysconf"
            | "getauxval"
            | "poll"
            | "sigaction"
            | "sigaltstack"
            | "mprotect"
    )
}

#[test]
fn phase_d_every_c_symbol_is_exported_by_rust() {
    let c = c_so();
    let r = rust_so();
    let c_defs = defined(&c);
    let r_defs = defined(&r);

    assert!(
        c_defs.contains("bin2hex"),
        "sanity: C .so must export bin2hex, got {c_defs:?}"
    );

    let missing: Vec<&String> = c_defs.difference(&r_defs).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}\n\
         C  ({}): {c_defs:?}\nRust({}): {r_defs:?}",
        c.display(),
        r.display()
    );
}

#[test]
fn phase_d_rust_has_no_unresolved_non_libc_imports() {
    let r = rust_so();
    let bad: Vec<String> = undefined(&r)
        .into_iter()
        .filter(|n| !is_platform_import(n))
        .collect();
    assert!(
        bad.is_empty(),
        "Rust .so imports non-libc symbols (unresolved / untranslated): {bad:?}"
    );
}

#[test]
fn phase_d_c_imports_are_covered() {
    // The C .so imports exactly one strong external: abort(). The Rust .so must
    // use the same libc abort so the failure mode is byte-identical (SIGABRT).
    let c_undef = undefined(&c_so());
    assert_eq!(
        c_undef.iter().cloned().collect::<Vec<_>>(),
        vec!["abort".to_string()],
        "unexpected C imports"
    );
    assert!(
        undefined(&rust_so()).contains("abort"),
        "the Rust .so must call libc abort(), not a Rust panic"
    );
}

/// Both `.so`s must resolve `bin2hex` through `dlsym` with the exact same name.
#[test]
fn phase_d_dlsym_resolves_in_both() {
    let f = common::impls();
    assert!(!(f.c as usize == 0));
    assert!(!(f.rust as usize == 0));
    assert_ne!(f.c as usize, f.rust as usize, "the two .so's symbols must be distinct");
}
