//! Phase D — symbol parity, enforced as a test.
//!
//! Runs `nm -D --defined-only` on both shared objects and requires the set of
//! exported symbol names to be identical (the diff must be empty), and requires
//! the Rust `.so` to have no undefined non-libc symbols.

mod common;
use common::Libs;
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

fn nm(path: &Path, extra: &str) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", extra])
        .arg(path)
        .output()
        .expect("run nm (binutils required)");
    assert!(
        out.status.success(),
        "nm {extra} {} failed: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last())
        .map(|s| s.split('@').next().unwrap_or(s).to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Symbols the Rust runtime unavoidably imports from libc / libgcc.
fn is_runtime_import(s: &str) -> bool {
    const PREFIXES: &[&str] = &["_Unwind_", "_ITM_", "__cxa_", "__gmon_", "__tls_", "__errno"];
    const NAMES: &[&str] = &[
        "abort", "bcmp", "calloc", "close", "dl_iterate_phdr", "free", "fstat", "fstat64",
        "getcwd", "getenv", "gettid", "lseek", "lseek64", "malloc", "memcpy", "memmove", "memset",
        "mmap", "mmap64", "munmap", "open", "open64", "posix_memalign", "pthread_key_create",
        "pthread_key_delete", "pthread_setspecific", "pthread_getspecific", "read", "readlink",
        "realloc", "realpath", "stat", "stat64", "statx", "strlen", "syscall", "write", "writev",
        "sysconf", "sigaltstack", "mprotect", "pthread_self", "pthread_getattr_np",
        "pthread_attr_getstack", "pthread_attr_destroy", "memrchr", "poll", "getrandom",
        "__libc_start_main", "environ", "qsort", "signal", "sigaction", "raise",
    ];
    PREFIXES.iter().any(|p| s.starts_with(p)) || NAMES.contains(&s)
}

#[test]
fn every_c_symbol_is_exported_by_rust() {
    let l = Libs::load();
    let c_syms = nm(&l.c_path, "--defined-only");
    let rust_syms = nm(&l.rust_path, "--defined-only");

    println!("C   ({}) exports: {:?}", l.c_path.display(), c_syms);
    println!("Rust({}) exports: {:?}", l.rust_path.display(), rust_syms);

    let missing: Vec<_> = c_syms.difference(&rust_syms).cloned().collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing {} symbol(s) exported by the C .so: {:?}\n\
         Every one must be translated and exported with the exact same name.",
        missing.len(),
        missing
    );

    // The C library's single public symbol must be present.
    assert!(c_syms.contains("pow43"), "C .so must export pow43");
    assert!(rust_syms.contains("pow43"), "Rust .so must export pow43");

    // `g_pow43` is `static const` in C -> must NOT be exported by either side.
    assert!(
        !c_syms.contains("g_pow43"),
        "g_pow43 is `static const`; it must not appear in the C .dynsym"
    );
    assert!(
        !rust_syms.contains("g_pow43"),
        "g_pow43 must not be exported by the Rust .so either"
    );

    println!("symbol diff (C -> Rust) is EMPTY: {} symbol(s) in parity", c_syms.len());
}

#[test]
fn rust_has_no_unresolved_project_symbols() {
    let l = Libs::load();
    let undef = nm(&l.rust_path, "--undefined-only");
    let leftover: Vec<_> = undef
        .iter()
        .filter(|s| !is_runtime_import(s))
        .cloned()
        .collect();
    assert!(
        leftover.is_empty(),
        "Rust .so has undefined non-libc symbols (unimplemented externs?): {leftover:?}"
    );
    println!("Rust .so: 0 undefined non-libc symbols ({} runtime imports)", undef.len());
}

#[test]
fn exported_symbol_is_callable_through_the_so() {
    // Guards against a symbol that exists in `nm` but is not a real, callable
    // export (e.g. a data symbol or a stub). Calls it via dlsym in both libs.
    let l = Libs::load();
    for x in [-16, 0, 1, 128, 129, 1023, 1024, 8223] {
        l.assert_same(x, "symbol_parity/callable");
    }
    println!("pow43 is callable through both .so exports and agrees");
}
