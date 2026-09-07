//! Phase D — symbol parity between the C `.so` and the Rust `.so`.
//!
//! Runs `nm -D` on both objects and requires the C exported-symbol set to be a
//! subset of the Rust exported-symbol set (the diff must be EMPTY). Also checks
//! that the Rust `.so` imports nothing outside libc / the Rust runtime.

mod common;
use common::Pair;

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

fn nm(path: &Path, extra: &str) -> Vec<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg(extra)
        .arg(path)
        .output()
        .expect("run nm -D");
    assert!(
        out.status.success(),
        "nm -D {extra} {} failed: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|l| l.to_string())
        .collect()
}

/// Exported (defined) code/data symbols: `T`, `t`, `D`, `B`, `R`, `W`, ...
/// We keep only globally visible definitions with a name.
fn defined_symbols(path: &Path) -> BTreeSet<String> {
    nm(path, "--defined-only")
        .into_iter()
        .filter_map(|line| {
            let mut it = line.split_whitespace();
            let a = it.next()?;
            let (kind, name) = match it.next() {
                Some(k) if k.len() == 1 => (k.to_string(), it.next()?.to_string()),
                Some(n) if a.len() == 1 => (a.to_string(), n.to_string()),
                _ => return None,
            };
            // Skip the linker/loader bookkeeping symbols neither library owns.
            if matches!(
                name.as_str(),
                "_init"
                    | "_fini"
                    | "__bss_start"
                    | "_edata"
                    | "_end"
                    | "_IO_stdin_used"
                    | "__gmon_start__"
                    | "_ITM_registerTMCloneTable"
                    | "_ITM_deregisterTMCloneTable"
                    | "__cxa_finalize"
                    | "__TMC_END__"
                    | "__dso_handle"
            ) {
                return None;
            }
            let _ = kind;
            Some(name)
        })
        .collect()
}

fn undefined_symbols(path: &Path) -> BTreeSet<String> {
    nm(path, "--undefined-only")
        .into_iter()
        .filter_map(|line| {
            let name = line.split_whitespace().last()?.to_string();
            if name == "U" || name == "w" || name.is_empty() {
                return None;
            }
            // strip the @GLIBC_x.y / @GCC_x.y version suffix
            Some(name.split('@').next().unwrap_or(&name).to_string())
        })
        .collect()
}

#[test]
fn d1_c_exported_symbols_are_all_exported_by_rust() {
    let p = Pair::load();
    let c = defined_symbols(&p.c_path);
    let r = defined_symbols(&p.rust_path);

    assert!(
        c.contains("dataentry"),
        "the C .so must export `dataentry`; got {c:?}"
    );

    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}\n\
         C   = {c:?}\n\
         Rust= {r:?}"
    );
    // Record the surface for the SYMBOLS.md artifact.
    println!("C exported ({}): {c:?}", c.len());
    println!("Rust exported ({}): {r:?}", r.len());
}

#[test]
fn d2_rust_so_has_no_unexpected_undefined_symbols() {
    let p = Pair::load();
    let und = undefined_symbols(&p.rust_path);

    // libc + Rust std/panic-runtime imports. Anything outside this set would
    // mean the Rust .so depends on a symbol it does not provide.
    let allowed_exact: &[&str] = &[
        "abort", "bcmp", "calloc", "close", "dl_iterate_phdr", "free", "fstat", "fstat64",
        "getcwd", "getenv", "gettid", "lseek", "lseek64", "malloc", "memcmp", "memcpy",
        "memmove", "memset", "mmap", "mmap64", "munmap", "open", "open64", "posix_memalign",
        "read", "readlink", "realloc", "realpath", "sprintf", "stat", "stat64", "statx",
        "strlen", "syscall", "write", "writev", "sysconf", "pthread_self", "pthread_getattr_np",
        "pthread_attr_getstack", "pthread_attr_destroy", "poll", "sigaction", "sigaltstack",
        "mprotect", "getrandom", "clock_gettime", "nanosleep", "exit", "_exit",
    ];
    let allowed_prefix: &[&str] = &[
        "_Unwind_",
        "__cxa_",
        "_ITM_",
        "__gmon_start__",
        "__errno_location",
        "__tls_get_addr",
        "__libc_",
        "pthread_",
        "__gxx_",
        "__rust",
        "_ZN",
    ];

    let unexpected: Vec<&String> = und
        .iter()
        .filter(|n| {
            !allowed_exact.contains(&n.as_str())
                && !allowed_prefix.iter().any(|p| n.starts_with(p))
        })
        .collect();

    assert!(
        unexpected.is_empty(),
        "Rust .so imports non-libc / non-runtime symbols: {unexpected:?}\nall undefined: {und:?}"
    );
}

#[test]
fn d3_no_stub_symbols_the_c_does_not_have() {
    // Sanity: the Rust .so must not be missing `dataentry` nor export a
    // differently-named variant of it (name mangling regression guard).
    let p = Pair::load();
    let r = defined_symbols(&p.rust_path);
    assert!(
        r.contains("dataentry"),
        "Rust .so must export the exact, unmangled name `dataentry`; got {r:?}"
    );
    assert!(
        !r.iter().any(|s| s.contains("dataentry") && s != "dataentry"),
        "unexpected mangled dataentry-like symbol in the Rust .so: {r:?}"
    );
    // And the loaded function must be callable and agree with C.
    assert_eq!(p.call_c(3, 3, 2, 0), p.call_rust(3, 3, 2, 0));
}

/// `translation/Cargo.toml` declares no `[features]`, so there is exactly one
/// build configuration; this test pins that fact so a future feature addition
/// forces the matrix to be revisited.
#[test]
fn d4_no_cargo_features_declared() {
    let manifest = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"))
        .expect("read Cargo.toml");
    let has_features_section = manifest
        .lines()
        .map(|l| l.trim())
        .any(|l| l == "[features]");
    assert!(
        !has_features_section,
        "Cargo.toml now declares [features]; Phase B/C must be re-run for every \
         feature combination (see check_features.sh)"
    );
    // No `#[cfg(feature = ...)]` in the library either.
    let lib = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/lib.rs"))
        .expect("read src/lib.rs");
    assert!(
        !lib.contains("feature ="),
        "src/lib.rs now has feature-gated code; re-run the feature matrix"
    );
}
