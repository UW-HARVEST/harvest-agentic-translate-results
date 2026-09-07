//! Phase D — symbol parity enforced as a test, so the gate cannot silently
//! regress. Mirrors `SYMBOLS.md`.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so() -> PathBuf {
    manifest_dir().join("../c_src/build/libdriver.so")
}

fn rust_so() -> PathBuf {
    match std::env::var_os("DRIVER_RUST_SO") {
        Some(p) => PathBuf::from(p),
        None => {
            let rel = manifest_dir().join("target/release/libdriver.so");
            if rel.exists() {
                rel
            } else {
                manifest_dir().join("target/debug/libdriver.so")
            }
        }
    }
}

/// `nm -D --defined-only` names, text/data symbols only.
fn exported(path: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(path)
        .output()
        .expect("run nm");
    assert!(out.status.success(), "nm failed on {}", path.display());
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let (_addr, kind, name) = (it.next()?, it.next()?, it.next()?);
            // Skip the linker-synthesized housekeeping symbols.
            matches!(kind, "T" | "t" | "D" | "B" | "R" | "W")
                .then(|| name.to_string())
                .filter(|n| !n.starts_with("_init") && !n.starts_with("_fini"))
        })
        .collect()
}

/// The Rust `.so` links `libstd`, so it legitimately imports libc, the unwinder
/// and TLS helpers. Anything outside these families would be a missing module.
fn is_runtime_or_libc(sym: &str) -> bool {
    let base = sym.split('@').next().unwrap_or(sym);
    const LIBC: &[&str] = &[
        "puts", "abort", "malloc", "calloc", "realloc", "free", "posix_memalign", "memcpy",
        "memmove", "memset", "bcmp", "strlen", "write", "writev", "read", "close", "open64",
        "lseek64", "fstat64", "stat64", "statx", "mmap64", "munmap", "getcwd", "getenv",
        "readlink", "realpath", "syscall", "gettid", "dl_iterate_phdr", "pthread_key_create",
        "pthread_key_delete", "pthread_setspecific", "pthread_getspecific", "sysconf",
        "pthread_self", "pthread_mutex_lock", "pthread_mutex_unlock", "memrchr", "memchr",
        "strerror_r", "sigaction", "sigaltstack", "mprotect", "poll", "nanosleep", "sched_yield",
    ];
    LIBC.contains(&base)
        || base.starts_with("_Unwind_")
        || base.starts_with("__cxa_")
        || base.starts_with("__tls_get_addr")
        || base.starts_with("__errno_location")
        || base.starts_with("_ITM_")
        || base.starts_with("__gmon_start__")
        || base.starts_with("__libc_")
        || base.starts_with("__stack_chk")
        || base.starts_with("__rust")
}

fn undefined(path: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "-u"])
        .arg(path)
        .output()
        .expect("run nm");
    assert!(out.status.success());
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(str::to_string))
        .filter(|n| *n != "U" && *n != "w")
        .collect()
}

/// SYMBOLS.md gate: every symbol the C `.so` exports must also be exported by
/// the Rust `.so`, under the exact same name.
#[test]
fn symbols_c_exports_are_all_present_in_rust() {
    let c = exported(&c_so());
    let r = exported(&rust_so());

    assert_eq!(
        c,
        ["bad", "driver", "good", "printLine"]
            .iter()
            .map(|s| s.to_string())
            .collect::<BTreeSet<_>>(),
        "the C export set changed; SYMBOLS.md must be regenerated"
    );

    let missing: Vec<_> = c.difference(&r).cloned().collect();
    assert!(
        missing.is_empty(),
        "Rust `.so` is missing C-exported symbols: {missing:?}\n  C:    {c:?}\n  Rust: {r:?}"
    );

    // And each one must actually be callable through dlsym.
    for name in &c {
        if name == "printLine" {
            let _ = print_line(rust_lib());
        } else {
            let _ = nullary(rust_lib(), name);
        }
    }
}

/// The C `.so` keeps `helperGood`/`helperBad` out of its dynamic symbol table;
/// Rust must not export extra public symbols beyond the C surface either.
#[test]
fn symbols_rust_exports_no_extra_c_named_symbols() {
    let c = exported(&c_so());
    let r = exported(&rust_so());
    let extra: Vec<_> = r
        .difference(&c)
        .filter(|s| !s.starts_with("__") && !s.starts_with('_'))
        .cloned()
        .collect();
    assert!(
        extra.is_empty(),
        "Rust `.so` exports symbols absent from the C surface: {extra:?}"
    );
}

/// No unresolved non-libc / non-runtime imports in the Rust `.so` — that would
/// mean a whole module was never translated.
#[test]
fn symbols_rust_has_no_unexpected_undefined() {
    let unexpected: Vec<_> = undefined(&rust_so())
        .into_iter()
        .filter(|s| !is_runtime_or_libc(s))
        .collect();
    assert!(
        unexpected.is_empty(),
        "Rust `.so` has undefined non-libc symbols (untranslated module?): {unexpected:?}"
    );
}

/// Both libraries must go through the same libc `puts`, which is what keeps
/// their `stdout` buffering identical (see `SYMBOLS.md`).
#[test]
fn symbols_both_import_puts() {
    for (label, path) in [("C", c_so()), ("Rust", rust_so())] {
        let u = undefined(&path);
        assert!(
            u.iter().any(|s| s.split('@').next() == Some("puts")),
            "{label} `.so` does not import `puts`; imports: {u:?}"
        );
    }
}

/// The project builds no executable, so there is no binary stdout to compare.
/// Asserted here so the claim in CONFIGS.md stays true.
#[test]
fn no_binary_target_exists() {
    let cml = std::fs::read_to_string(manifest_dir().join("../c_src/CMakeLists.txt")).unwrap();
    assert!(
        !cml.contains("add_executable"),
        "c_src now builds an executable; Phase B must add a binary stdout comparison"
    );
    let cargo = std::fs::read_to_string(manifest_dir().join("Cargo.toml")).unwrap();
    assert!(
        !cargo.contains("[[bin]]"),
        "translation now builds a binary; Phase B must add a binary stdout comparison"
    );
}
