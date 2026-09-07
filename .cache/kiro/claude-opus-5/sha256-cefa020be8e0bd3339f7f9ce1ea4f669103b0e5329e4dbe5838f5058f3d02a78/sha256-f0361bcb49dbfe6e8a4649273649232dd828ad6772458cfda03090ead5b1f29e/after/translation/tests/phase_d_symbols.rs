//! Phase D — symbol parity, enforced from inside the test suite.
//!
//! Asserts that every dynamic symbol the C `.so` defines is also defined by the
//! Rust `.so` with the exact same name, and that the Rust `.so` has no
//! undefined non-libc symbols. This is the `nm -D` diff from `SYMBOLS.md`,
//! mechanised so it cannot silently regress.

mod common;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

fn c_so() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("c_src/build/libdriver.so")
}

fn rust_so() -> PathBuf {
    let mut p = std::env::current_exe().unwrap();
    p.pop();
    if p.file_name().and_then(|s| s.to_str()) == Some("deps") {
        p.pop();
    }
    p.join("libdriver.so")
}

fn nm(path: &Path, extra: &str) -> BTreeSet<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg(extra)
        .arg(path)
        .output()
        .unwrap_or_else(|e| panic!("running nm on {path:?}: {e}"));
    assert!(
        out.status.success(),
        "nm -D {extra} {path:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(str::to_string))
        .collect()
}

/// Symbols that legitimately appear as undefined in a Rust cdylib: libc,
/// libgcc unwinder, and the weak ELF/ITM boilerplate present in the C `.so`
/// too.
fn is_libc_or_runtime(sym: &str) -> bool {
    let base = sym.split('@').next().unwrap_or(sym);
    if base.starts_with("_Unwind_")
        || base.starts_with("_ITM_")
        || base.starts_with("__cxa_")
        || base.starts_with("pthread_")
        || base.starts_with("__")
    {
        return true;
    }
    const LIBC: &[&str] = &[
        "printf", "putchar", "abort", "bcmp", "calloc", "close", "dl_iterate_phdr", "free",
        "fstat64", "getcwd", "getenv", "gettid", "lseek64", "malloc", "memcpy", "memmove",
        "memset", "mmap64", "munmap", "open64", "posix_memalign", "read", "readlink", "realloc",
        "realpath", "stat64", "statx", "strlen", "syscall", "write", "writev", "fflush", "dup",
        "dup2", "fwrite", "puts", "memcmp", "sigaltstack", "sysconf", "mprotect", "pipe2",
        "poll", "getpid", "sched_getaffinity", "sigaction", "signal", "raise", "exit",
    ];
    LIBC.contains(&base)
}

#[test]
fn symbols_defined_parity_is_exact() {
    let c = c_so();
    let r = rust_so();
    assert!(c.is_file(), "C .so missing at {c:?}");
    assert!(r.is_file(), "Rust .so missing at {r:?}");

    let c_defined = nm(&c, "--defined-only");
    let r_defined = nm(&r, "--defined-only");

    let missing: Vec<&String> = c_defined.difference(&r_defined).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}"
    );

    // The one documented public symbol must actually be there on both sides.
    assert!(c_defined.contains("driver"), "C .so lacks `driver`");
    assert!(r_defined.contains("driver"), "Rust .so lacks `driver`");

    // `print_hex` is `static` in C: exported by neither.
    assert!(!c_defined.contains("print_hex"));
    assert!(!r_defined.contains("print_hex"));
}

#[test]
fn rust_so_has_no_undefined_non_libc_symbols() {
    let r = rust_so();
    let undef = nm(&r, "--undefined-only");
    let bad: Vec<&String> = undef.iter().filter(|s| !is_libc_or_runtime(s)).collect();
    assert!(
        bad.is_empty(),
        "Rust .so has undefined non-libc symbols: {bad:?}"
    );
}

#[test]
fn no_binary_target_on_either_side() {
    // The completion gate's "compare binary stdout" item is N/A: neither side
    // builds an executable. Assert that mechanically so the claim stays true.
    let cmake = std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("c_src/CMakeLists.txt"),
    )
    .unwrap();
    assert!(
        !cmake.contains("add_executable"),
        "c_src now builds an executable; the binary-stdout comparison is no longer N/A"
    );
    let manifest =
        std::fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"))
            .unwrap();
    assert!(
        !manifest.contains("[[bin]]"),
        "the crate now declares a binary target"
    );
    assert!(
        !PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("src/main.rs")
            .exists(),
        "src/main.rs now exists; a binary comparison is required"
    );
}
