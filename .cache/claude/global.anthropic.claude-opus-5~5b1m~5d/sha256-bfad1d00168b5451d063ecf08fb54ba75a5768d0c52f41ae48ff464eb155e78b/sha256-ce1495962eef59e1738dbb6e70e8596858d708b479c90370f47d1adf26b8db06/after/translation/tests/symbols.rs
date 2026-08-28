//! Phase D — symbol parity between the C `.so` and the Rust `.so`.
//!
//! Runs `nm -D` on both objects and requires that every symbol the C `.so`
//! exports is also exported by the Rust `.so` with the exact same name, and
//! that the Rust `.so` has no undefined non-libc symbols.

mod common;

use common::impls;
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

/// Symbols defined (exported) by an object, per `nm -D --defined-only`.
fn defined_symbols(so: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(so)
        .output()
        .expect("`nm` must be available to run the symbol-parity test");
    assert!(
        out.status.success(),
        "nm -D --defined-only {} failed: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    parse_nm(&String::from_utf8_lossy(&out.stdout))
}

/// Symbols undefined (imported) by an object, per `nm -D -u`.
fn undefined_symbols(so: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "-u"])
        .arg(so)
        .output()
        .expect("`nm` must be available");
    assert!(out.status.success(), "nm -D -u failed");
    parse_nm(&String::from_utf8_lossy(&out.stdout))
}

fn parse_nm(stdout: &str) -> BTreeSet<String> {
    stdout
        .lines()
        .filter_map(|line| {
            // "<addr> <type> <name>" or "         <type> <name>"
            let name = line.split_whitespace().last()?;
            if name.is_empty() || name.len() == 1 {
                return None;
            }
            // strip the @GLIBC_x.y / @GCC_x.y version suffix
            Some(name.split('@').next().unwrap_or(name).to_string())
        })
        .collect()
}

/// Symbols that are legitimately provided by libc / the compiler runtime /
/// the dynamic loader, and therefore do not indicate a missing translation.
fn is_runtime_symbol(name: &str) -> bool {
    const EXACT: &[&str] = &[
        // weak toolchain hooks (present in the C .so too)
        "_ITM_deregisterTMCloneTable",
        "_ITM_registerTMCloneTable",
        "__cxa_finalize",
        "__gmon_start__",
        "__cxa_thread_atexit_impl",
        "__errno_location",
        "__tls_get_addr",
        // libc
        "abort", "bcmp", "calloc", "close", "dl_iterate_phdr", "free", "fstat64",
        "getcwd", "getenv", "gettid", "lseek64", "malloc", "memcmp", "memcpy",
        "memmove", "memset", "mmap64", "munmap", "open64", "posix_memalign",
        "pthread_key_create", "pthread_key_delete", "pthread_getspecific",
        "pthread_setspecific", "read", "readlink", "realloc", "realpath",
        "stat64", "statx", "strlen", "syscall", "write", "writev",
        "sysconf", "sigaction", "sigaltstack", "mprotect", "poll", "pipe2",
        "getpid",
    ];
    EXACT.contains(&name)
        || name.starts_with("_Unwind_")
        || name.starts_with("__libc_")
        || name.starts_with("pthread_")
        || name.starts_with("__pthread_")
        || name.starts_with("__rust_")
        || name.starts_with("_ZN")   // mangled Rust internals, never part of the C ABI
        || name.starts_with("__gxx_")
        || name.starts_with("_dl_")
}

#[test]
fn phase_d_symbol_parity() {
    let f = impls();
    let c_defined = defined_symbols(&f.c_path);
    let r_defined = defined_symbols(&f.rust_path);

    eprintln!("C   .so: {}", f.c_path.display());
    eprintln!("Rust.so: {}", f.rust_path.display());
    eprintln!("C   defined ({}): {:?}", c_defined.len(), c_defined);
    eprintln!("Rust defined ({}): {:?}", r_defined.len(), r_defined);

    // The C library must actually have been built with its one public symbol.
    assert!(
        c_defined.contains("max_size_frame"),
        "C .so does not export max_size_frame -- is the build stale?"
    );

    // EVERY symbol exported by C must be exported by Rust under the same name.
    let missing: Vec<&String> = c_defined.difference(&r_defined).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is MISSING {} symbol(s) exported by the C .so: {:?}\n\
         Per Phase A: add the #[no_mangle] export if the impl exists, or \
         translate the missing C source.",
        missing.len(),
        missing
    );
}

#[test]
fn phase_d_no_undefined_non_libc_symbols_in_rust() {
    let f = impls();
    let leftovers: Vec<String> = undefined_symbols(&f.rust_path)
        .into_iter()
        .filter(|s| !is_runtime_symbol(s))
        .collect();
    assert!(
        leftovers.is_empty(),
        "Rust .so has undefined non-libc symbols (unresolved at load time, \
         i.e. untranslated C dependencies): {leftovers:?}"
    );
}

#[test]
fn phase_d_rust_exports_no_extra_public_symbols() {
    let f = impls();
    let c_defined = defined_symbols(&f.c_path);
    let r_defined = defined_symbols(&f.rust_path);
    let extra: Vec<&String> = r_defined.difference(&c_defined).collect();
    assert!(
        extra.is_empty(),
        "Rust .so exports symbols the C .so does not (ABI surface mismatch): {extra:?}"
    );
}

/// Both `.so`s must resolve the symbol by its exact C name via `dlsym`.
#[test]
fn phase_d_dlsym_exact_name() {
    let f = impls();
    // `impls()` already did `get(b"max_size_frame\0")` on both libraries; a
    // successful load proves the exact-name export in both. Sanity-call each.
    assert_eq!(f.c(4096, 2, 16), 16916);
    assert_eq!(f.rust(4096, 2, 16), 16916);
}
