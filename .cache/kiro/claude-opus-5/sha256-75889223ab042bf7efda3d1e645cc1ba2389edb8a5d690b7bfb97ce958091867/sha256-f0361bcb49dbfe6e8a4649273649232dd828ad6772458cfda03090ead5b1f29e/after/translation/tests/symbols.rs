//! Phase D — machine-checked symbol parity between the two shared objects.
//!
//! Fails if the C `.so` ever exports a defined symbol the Rust `.so` does not,
//! or if the Rust `.so` has an undefined symbol that is not satisfied by
//! libc / libgcc-unwind / pthread.

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::process::Command;

fn c_so() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO") {
        return PathBuf::from(p);
    }
    let mut v: Vec<PathBuf> = std::fs::read_dir("../c_src/build")
        .expect("build the C library first")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == "so").unwrap_or(false))
        .collect();
    v.sort();
    v.remove(0)
}

fn rust_so() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO") {
        return PathBuf::from(p);
    }
    for c in ["target/release/libnormalize_lib.so", "target/debug/libnormalize_lib.so"] {
        let p = PathBuf::from(c);
        if p.exists() {
            return p;
        }
    }
    panic!("no Rust cdylib — run `cargo build --release`");
}

/// `nm -D` names, filtered by symbol-type predicate.
fn nm(path: &PathBuf, args: &[&str]) -> BTreeSet<String> {
    let out = Command::new("nm")
        .arg("-D")
        .args(args)
        .arg(path)
        .output()
        .expect("`nm` not available");
    assert!(out.status.success(), "nm failed on {}", path.display());
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let a = it.next()?;
            let (ty, name) = match it.next() {
                Some(b) => match it.next() {
                    Some(c) => (b, c),   // "addr T name"
                    None => (a, b),      // "w name" / "U name"
                },
                None => return None,
            };
            // Weak toolchain/CRT hooks are not part of the library API surface.
            if ty == "w" {
                return None;
            }
            Some(name.split('@').next().unwrap().to_string())
        })
        .collect()
}

#[test]
fn phase_d_every_c_export_is_exported_by_rust() {
    let c = c_so();
    let r = rust_so();
    let c_def = nm(&c, &["--defined-only"]);
    let r_def = nm(&r, &["--defined-only"]);
    eprintln!("C   defined ({}): {:?}", c_def.len(), c_def);
    eprintln!("Rust defined ({}): {:?}", r_def.len(), r_def);

    let missing: Vec<&String> = c_def.difference(&r_def).collect();
    assert!(
        missing.is_empty(),
        "the Rust .so is missing {} symbol(s) exported by the C .so: {missing:?}",
        missing.len()
    );
    assert!(c_def.contains("normalize"), "C .so lost `normalize`?");
    assert!(r_def.contains("normalize"), "Rust .so does not export `normalize`");
}

#[test]
fn phase_d_no_unresolved_non_libc_symbols_in_rust() {
    let r = rust_so();
    let undef = nm(&r, &["--undefined-only"]);
    // Everything the Rust std panic/backtrace machinery legitimately imports.
    let allowed_prefix = ["_Unwind_", "__", "pthread_", "dl_iterate_phdr"];
    let allowed_exact: BTreeSet<&str> = [
        "abort", "bcmp", "calloc", "close", "free", "fstat64", "getcwd", "getenv", "gettid",
        "lseek64", "malloc", "memcmp", "memcpy", "memmove", "memset", "mmap64", "munmap",
        "open64", "posix_memalign", "read", "readlink", "realloc", "realpath", "stat64",
        "statx", "strlen", "syscall", "write", "writev", "sqrtf", "sqrt", "exit",
        "sigaltstack", "mprotect", "sysconf", "getpid", "pipe2", "poll", "sched_yield",
        "nanosleep", "clock_gettime", "environ", "_exit", "raise", "signal", "sigaction",
        "sigemptyset", "sigaddset", "strerror_r", "isatty", "fcntl", "openat64", "getrandom",
    ]
    .into_iter()
    .collect();

    let bad: Vec<&String> = undef
        .iter()
        .filter(|s| {
            !allowed_exact.contains(s.as_str())
                && !allowed_prefix.iter().any(|p| s.starts_with(p))
        })
        .collect();
    assert!(
        bad.is_empty(),
        "Rust .so has unresolved non-libc symbol(s) — a translated module is missing: {bad:?}"
    );
}
