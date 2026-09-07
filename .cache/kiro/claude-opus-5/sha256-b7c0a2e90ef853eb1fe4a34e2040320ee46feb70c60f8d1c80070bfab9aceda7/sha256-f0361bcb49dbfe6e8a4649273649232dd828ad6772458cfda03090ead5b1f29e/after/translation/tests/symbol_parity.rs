//! Phase D — symbol parity between the C `.so` and the Rust `.so`.
//!
//! Runs `nm -D` on both libraries and requires the exported-symbol sets to be
//! identical, and requires every undefined symbol on the Rust side to be
//! satisfiable from libc / the Rust runtime (i.e. no unresolved non-libc symbol).

mod common;

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf()
}

fn nm(args: &[&str], so: &PathBuf) -> Vec<String> {
    let out = Command::new("nm")
        .args(args)
        .arg(so)
        .output()
        .expect("nm must be available on PATH");
    assert!(out.status.success(), "nm failed on {}", so.display());
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .collect()
}

fn defined(so: &PathBuf) -> BTreeSet<String> {
    nm(&["-D", "--defined-only"], so).into_iter().collect()
}

fn undefined(so: &PathBuf) -> BTreeSet<String> {
    nm(&["-D", "-u"], so).into_iter().collect()
}

fn c_so() -> PathBuf {
    repo_root().join("c_src/build/libdriver.so")
}

fn rust_so() -> PathBuf {
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target");
    for p in ["release", "debug"] {
        let c = base.join(p).join("libdriver.so");
        if c.exists() {
            return c;
        }
    }
    panic!("build the Rust cdylib first: cd translation && cargo build --release");
}

/// The core Phase D gate: the symbol diff must be empty.
#[test]
fn sym_01_exported_symbol_diff_is_empty() {
    let c = defined(&c_so());
    let r = defined(&rust_so());

    let missing: Vec<_> = c.difference(&r).cloned().collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}\n\
         Per Phase A these must be translated (not stubbed) and exported."
    );

    // The C .so exports exactly these 5; anything else on the Rust side would be
    // a surface the C does not have.
    let expected: BTreeSet<String> =
        common::EXPORTED_SYMBOLS.iter().map(|s| s.to_string()).collect();
    assert_eq!(c, expected, "the C .so's exported set changed unexpectedly");

    let extra: Vec<_> = r.difference(&c).cloned().collect();
    assert!(extra.is_empty(), "Rust .so exports symbols the C .so does not: {extra:?}");
}

/// The `static` C helpers must stay unexported on both sides.
#[test]
fn sym_02_static_c_helpers_are_not_exported() {
    for lib in [c_so(), rust_so()] {
        let d = defined(&lib);
        for hidden in ["goodG2B", "goodB2G"] {
            assert!(
                !d.contains(hidden),
                "{hidden} is `static` in the C source and must not be exported by {}",
                lib.display()
            );
        }
    }
}

/// No unresolved non-libc symbol in the Rust `.so`.
#[test]
fn sym_03_no_unresolved_non_libc_symbols() {
    // Everything a Rust cdylib legitimately imports from glibc / libgcc_s.
    const ALLOWED_PREFIXES: &[&str] = &["_Unwind_", "_ITM_", "__cxa_", "__gmon_start__", "pthread_"];
    const ALLOWED: &[&str] = &[
        "printf", "puts", "abort", "calloc", "free", "malloc", "realloc", "posix_memalign",
        "memcpy", "memmove", "memset", "bcmp", "strlen", "close", "open64", "open", "read",
        "readlink", "realpath", "write", "writev", "lseek64", "lseek", "fstat64", "stat64",
        "statx", "mmap64", "mmap", "munmap", "getcwd", "getenv", "syscall", "gettid",
        "dl_iterate_phdr", "__errno_location", "__tls_get_addr", "fflush", "dup", "dup2",
    ];

    let mut offenders = Vec::new();
    for s in undefined(&rust_so()) {
        let base = s.split('@').next().unwrap_or(&s).to_string();
        let ok = ALLOWED.contains(&base.as_str())
            || ALLOWED_PREFIXES.iter().any(|p| base.starts_with(p));
        if !ok {
            offenders.push(s);
        }
    }
    assert!(
        offenders.is_empty(),
        "Rust .so has undefined symbols that are not libc/Rust-runtime: {offenders:?}"
    );
}

/// Every exported symbol must be resolvable via `dlsym` on both handles with the
/// expected C signature — i.e. the `#[no_mangle]` wrappers really are callable.
#[test]
fn sym_04_all_symbols_resolve_via_dlsym_on_both() {
    let _ = common::print_line::c();
    let _ = common::print_line::rs();
    let _ = common::print_int_line::c();
    let _ = common::print_int_line::rs();
    let _ = common::bad::c();
    let _ = common::bad::rs();
    let _ = common::good::c();
    let _ = common::good::rs();
    let _ = common::driver::c();
    let _ = common::driver::rs();
}

/// The project builds no executable, so there is no binary stdout to compare.
/// Asserted mechanically so the claim cannot silently go stale.
#[test]
fn sym_05_project_defines_no_binary_target() {
    let cmake = std::fs::read_to_string(repo_root().join("c_src/CMakeLists.txt")).unwrap();
    assert!(
        !cmake.contains("add_executable"),
        "c_src/CMakeLists.txt now defines an executable; the C-vs-Rust binary \
         stdout comparison from Phase B is no longer vacuous and must be implemented"
    );

    let cargo = std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"),
    )
    .unwrap();
    assert!(
        !cargo.contains("[[bin]]"),
        "translation/Cargo.toml now defines a binary target; implement the \
         binary stdout comparison"
    );
    let src_main = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/main.rs");
    assert!(!src_main.exists(), "src/main.rs appeared; implement the binary comparison");
}

/// `Cargo.toml` declares no `[features]`, so "every feature combination" is a
/// single configuration. Fails if features are ever added without extending the
/// verification matrix.
#[test]
fn sym_06_no_cargo_features_declared() {
    let cargo = std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"),
    )
    .unwrap();
    assert!(
        !cargo.contains("[features]"),
        "a [features] section was added; Phases B-C must be re-run for each \
         combination (see scripts/check_features.sh)"
    );
}

#[test]
fn sym_00_harness_must_be_single_threaded() {
    common::assert_single_threaded();
}
