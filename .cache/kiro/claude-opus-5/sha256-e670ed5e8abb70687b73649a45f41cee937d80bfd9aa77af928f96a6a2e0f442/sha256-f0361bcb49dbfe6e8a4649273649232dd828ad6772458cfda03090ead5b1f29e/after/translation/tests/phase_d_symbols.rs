//! Phase D — symbol parity between the C `.so` and the Rust `.so`.
//!
//! Asserts mechanically (via `nm -D`) that every symbol the C shared object
//! exports is also exported by the Rust shared object under the exact same
//! name, and that the Rust `.so` has no unresolved non-libc imports.

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::process::Command;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so() -> PathBuf {
    std::env::var("C_DRIVER_SO")
        .map(PathBuf::from)
        .unwrap_or_else(|_| manifest_dir().join("../c_src/build/libdriver.so"))
}

fn rust_so() -> PathBuf {
    std::env::var("RUST_DRIVER_SO")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let rel = manifest_dir().join("target/release/libdriver.so");
            if rel.exists() {
                rel
            } else {
                manifest_dir().join("target/debug/libdriver.so")
            }
        })
}

/// Globally-defined dynamic symbols, i.e. what the object actually exports.
fn exported(path: &PathBuf) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", path.to_str().unwrap()])
        .output()
        .expect("run nm");
    assert!(out.status.success(), "nm failed on {}", path.display());
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let (a, b) = (it.next()?, it.next()?);
            // "<addr> T name" for defined symbols; weak/absolute forms too.
            let (kind, name) = match it.next() {
                Some(name) => (b, name),
                None => (a, b),
            };
            // Keep global text/data symbols; skip weak ELF boilerplate.
            if matches!(kind, "T" | "D" | "B" | "R" | "W" | "V" | "i") {
                Some(name.to_string())
            } else {
                None
            }
        })
        .collect()
}

#[test]
fn symbol_diff_is_empty() {
    let c = c_so();
    let r = rust_so();
    assert!(c.exists(), "C .so missing at {}", c.display());
    assert!(r.exists(), "Rust .so missing at {}", r.display());

    let c_syms = exported(&c);
    let r_syms = exported(&r);
    assert!(
        c_syms.contains("driver"),
        "C .so does not export `driver`; got {c_syms:?}"
    );

    let missing: Vec<_> = c_syms.difference(&r_syms).cloned().collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing {} symbol(s) exported by the C .so: {missing:?}\n\
         C exports:    {c_syms:?}\n Rust exports: {r_syms:?}",
        missing.len()
    );
}

#[test]
fn rust_so_has_no_unresolved_non_libc_imports() {
    let r = rust_so();
    let out = Command::new("nm")
        .args(["-D", "-u", r.to_str().unwrap()])
        .output()
        .expect("run nm");
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);

    // Everything the Rust cdylib imports must come from libc, libgcc's
    // unwinder, or be weak ELF boilerplate. Anything else would be a dangling
    // reference to code that was never translated.
    let allowed_prefixes = [
        "_ITM_", "__cxa_", "__gmon_start__", "_Unwind_", "__tls_get_addr",
        "__errno_location", "statx", "gettid", "pthread_", "syscall",
    ];
    let allowed_exact: BTreeSet<&str> = [
        "abort", "bcmp", "calloc", "close", "dl_iterate_phdr", "free", "fstat64",
        "getcwd", "getenv", "lseek64", "malloc", "memcpy", "memmove", "memset",
        "mmap64", "munmap", "open64", "posix_memalign", "printf", "putchar",
        "puts", "read", "readlink", "realloc", "realpath", "stat64", "strlen",
        "write", "writev", "fflush", "fwrite", "putc", "fputc", "fputs",
        "memcmp", "qsort", "getauxval", "sysconf", "sigaltstack", "signal",
        "sigaction", "mprotect", "madvise", "environ", "__libc_start_main",
    ]
    .into_iter()
    .collect();

    let mut unexpected = Vec::new();
    for line in text.lines() {
        let name = match line.split_whitespace().last() {
            Some(n) => n,
            None => continue,
        };
        let bare = name.split('@').next().unwrap_or(name);
        if allowed_exact.contains(bare) || allowed_prefixes.iter().any(|p| bare.starts_with(p)) {
            continue;
        }
        unexpected.push(bare.to_string());
    }
    assert!(
        unexpected.is_empty(),
        "Rust .so imports symbols that are neither libc nor runtime: {unexpected:?}"
    );
}

/// The project builds no executable on either side, so there is no binary
/// stdout comparison to perform. Asserted here so the claim stays true.
#[test]
fn project_builds_no_executable() {
    let cmake =
        std::fs::read_to_string(manifest_dir().join("../c_src/CMakeLists.txt")).expect("CMakeLists");
    assert!(
        !cmake.contains("add_executable"),
        "c_src now builds an executable -- a binary stdout comparison is required"
    );
    assert!(
        !manifest_dir().join("src/main.rs").exists(),
        "translation now has a binary target -- a binary stdout comparison is required"
    );
    let cargo = std::fs::read_to_string(manifest_dir().join("Cargo.toml")).expect("Cargo.toml");
    assert!(
        !cargo.contains("[[bin]]"),
        "translation declares a [[bin]] target -- compare its stdout too"
    );
}

/// `Cargo.toml` declares no `[features]`, so the default build is the only
/// configuration. If a feature is ever added, Phases B-C must be re-run per
/// combination and this test will fail to say so.
#[test]
fn no_feature_combinations_to_sweep() {
    let cargo = std::fs::read_to_string(manifest_dir().join("Cargo.toml")).expect("Cargo.toml");
    assert!(
        !cargo.contains("[features]"),
        "Cargo.toml gained a [features] section -- re-run Phases B and C for \
         every feature combination"
    );
}
