// Phase D — symbol parity between the C `.so` and the Rust `.so`.
//
// Every symbol the C library exports must also be exported by the Rust library
// with the exact same name, and every one must be reachable through dlsym.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::process::Command;

/// Exported (defined, dynamic) symbol names of a shared object, via `nm -D
/// --defined-only`.
fn exported_symbols(path: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", "--format=posix"])
        .arg(path)
        .output()
        .expect("run nm");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().next())
        .map(strip_version)
        .collect()
}

/// `printf@GLIBC_2.2.5` -> `printf`. Symbol *names* are compared, not the
/// glibc version tags the dynamic linker appends.
fn strip_version(s: &str) -> String {
    s.split('@').next().unwrap_or(s).to_string()
}

/// Undefined (imported) symbol names of a shared object.
fn undefined_symbols(path: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "-u", "--format=posix"])
        .arg(path)
        .output()
        .expect("run nm");
    assert!(out.status.success(), "nm -u failed on {}", path.display());
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().next())
        .map(strip_version)
        .collect()
}

#[test]
fn every_c_symbol_is_exported_by_rust() {
    let c = exported_symbols(&c_so_path());
    let r = exported_symbols(&rust_so_path());

    assert!(
        !c.is_empty(),
        "nm found no exported symbols in the C .so -- is it built?"
    );

    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "The Rust .so is MISSING {} symbol(s) exported by the C .so: {:?}\n\
         C exports:    {:?}\n\
         Rust exports: {:?}",
        missing.len(),
        missing,
        c,
        r
    );

    // Documented expectation from SYMBOLS.md: the C library exports exactly
    // `driver` (`print_hex` is `static`).
    assert_eq!(
        c,
        ["driver"].iter().map(|s| s.to_string()).collect::<BTreeSet<_>>(),
        "the C export set changed; SYMBOLS.md needs updating"
    );
    assert!(!c.contains("print_hex"), "print_hex is static in C");
    assert!(
        !r.contains("print_hex"),
        "Rust must not export print_hex either (it was static in C)"
    );
}

#[test]
fn every_c_symbol_is_resolvable_via_dlsym() {
    // Symbol table presence is not enough -- each name must actually resolve in
    // both libraries and be callable through the FFI boundary.
    ensure_loaded();
    for name in exported_symbols(&c_so_path()) {
        assert_eq!(name, "driver");
        let c = driver_fn(Impl::C);
        let r = driver_fn(Impl::Rust);
        let out_c = capture(|| unsafe { c(1.0) });
        let out_r = capture(|| unsafe { r(1.0) });
        assert_eq!(out_c, out_r, "dlsym'd `{name}` disagrees");
        assert_eq!(out_c, b"0000803f\n".to_vec());
    }
}

#[test]
fn rust_so_has_no_unresolved_non_libc_symbols() {
    // Everything the Rust .so imports must be a libc / libgcc runtime entry
    // point; nothing may be an unresolved symbol from the translation itself.
    let imports = undefined_symbols(&rust_so_path());
    let allowed_prefixes = ["_Unwind_", "_ITM_", "__", "pthread_"];
    let allowed_exact: BTreeSet<&str> = [
        "abort", "bcmp", "calloc", "close", "dl_iterate_phdr", "fflush", "fprintf",
        "free", "fstat", "fstat64", "getcwd", "getenv", "gettid", "lseek", "lseek64",
        "malloc", "memcmp", "memcpy", "memmove", "memset", "mmap", "mmap64", "munmap",
        "open", "open64", "posix_memalign", "printf", "putchar", "puts", "read",
        "readlink", "realloc", "realpath", "stat", "stat64", "statx", "strlen",
        "syscall", "write", "writev", "fwrite", "sysconf", "dlsym", "dladdr",
        "getrandom", "sigaction", "sigaltstack", "mprotect", "poll", "nanosleep",
        "environ", "memrchr", "strerror_r", "__errno_location", "qsort", "bsearch",
    ]
    .into_iter()
    .collect();

    let unexpected: Vec<&String> = imports
        .iter()
        .filter(|s| {
            !allowed_exact.contains(s.as_str())
                && !allowed_prefixes.iter().any(|p| s.starts_with(p))
        })
        .collect();
    assert!(
        unexpected.is_empty(),
        "Rust .so imports non-libc symbol(s), i.e. something is unresolved: {unexpected:?}"
    );

    // The Rust translation must emit through C stdio (the same libc `stdout`
    // FILE* the C library uses), not through Rust's own buffered stdout --
    // otherwise flush timing and interleaving can differ. `printf` is the call
    // the source actually makes, so require it.
    //
    // NOTE: do NOT require an exact match with the C library's stdio imports.
    // Both compilers rewrite `printf("\n")` into `putchar('\n')`, but only when
    // optimising: the C `.so` and the *release* Rust `.so` import `putchar`
    // while the *debug* Rust `.so` does not. That is a pure codegen artifact
    // with no observable difference -- both spellings write the same byte to the
    // same stream.
    assert!(
        imports.contains("printf"),
        "the Rust .so does not import `printf`; the translation is not going \
         through C stdio, so output buffering can diverge from the C library. \
         Imports were: {imports:?}"
    );

    let c_imports = undefined_symbols(&c_so_path());
    let stdio = ["printf", "putchar", "puts", "fwrite", "fputc", "fputs", "fprintf"];
    let c_stdio: BTreeSet<&str> = stdio.iter().copied().filter(|s| c_imports.contains(*s)).collect();
    let rust_stdio: BTreeSet<&str> =
        stdio.iter().copied().filter(|s| imports.contains(*s)).collect();
    assert!(
        !c_stdio.is_empty() && !rust_stdio.is_empty(),
        "both libraries must reach stdout via C stdio; C used {c_stdio:?}, Rust used {rust_stdio:?}"
    );
    // Rust must not pull in stdio entry points the C library never uses beyond
    // the printf/putchar pair the optimiser may swap between.
    let extra: Vec<&&str> = rust_stdio
        .iter()
        .filter(|s| !c_stdio.contains(*s) && **s != "putchar" && **s != "printf")
        .collect();
    assert!(
        extra.is_empty(),
        "Rust .so uses stdio functions the C library does not: {extra:?}"
    );
}
