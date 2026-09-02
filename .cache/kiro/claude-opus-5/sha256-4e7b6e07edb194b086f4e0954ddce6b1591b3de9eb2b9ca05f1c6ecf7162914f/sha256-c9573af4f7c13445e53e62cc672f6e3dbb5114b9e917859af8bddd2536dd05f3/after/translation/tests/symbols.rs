//! Phase A / Phase D — exported-symbol parity, checked mechanically with `nm -D`.

mod common;

use std::collections::BTreeSet;
use std::process::Command;

fn nm_defined(path: &std::path::Path) -> BTreeSet<String> {
    nm(path, "--defined-only")
}

fn nm_undefined(path: &std::path::Path) -> BTreeSet<String> {
    nm(path, "--undefined-only")
}

fn nm(path: &std::path::Path, flag: &str) -> BTreeSet<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg(flag)
        .arg(path)
        .output()
        .unwrap_or_else(|e| panic!("failed to run nm on {}: {e}", path.display()));
    assert!(
        out.status.success(),
        "nm -D {flag} {} failed: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            // "<addr> <type> <name>" or "                 U <name>"
            let mut it = l.split_whitespace().rev();
            let name = it.next()?;
            let kind = it.next()?;
            // Skip weak-undefined loader/ITM stubs and local symbols.
            if kind == "w" || kind == "v" {
                return None;
            }
            Some(name.split('@').next().unwrap().to_string())
        })
        .collect()
}

/// Symbols the Rust `.so` legitimately imports because of the Rust std runtime,
/// none of which the C `.so` needs. They are all libc / libgcc unwinder symbols.
fn is_runtime_import(name: &str) -> bool {
    const LIBC: &[&str] = &[
        "abort", "bcmp", "calloc", "close", "dl_iterate_phdr", "free", "fstat64", "getcwd",
        "getenv", "lseek64", "malloc", "memcpy", "memmove", "memset", "mmap64", "munmap",
        "open64", "posix_memalign", "read", "readlink", "realloc", "realpath", "stat64",
        "strlen", "syscall", "write", "writev", "expf", "exp", "gettid", "statx", "sysconf",
        "pipe2", "poll", "sigaction", "sigaltstack", "mprotect", "getauxval", "__libc_start_main",
    ];
    name.starts_with("_Unwind_")
        || name.starts_with("__")
        || name.starts_with("pthread_")
        || LIBC.contains(&name)
}

#[test]
fn c_exports_are_a_subset_of_rust_exports() {
    let c = common::c_so_path();
    let r = common::rust_so_path();
    let cdef = nm_defined(&c);
    let rdef = nm_defined(&r);

    println!("C   .so: {}\n  defined: {:?}", c.display(), cdef);
    println!("Rust.so: {}\n  defined: {:?}", r.display(), rdef);

    assert!(
        cdef.contains("gaussian_kernel"),
        "C .so must export gaussian_kernel, got {cdef:?}"
    );

    let missing: Vec<&String> = cdef.difference(&rdef).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is MISSING {} symbol(s) exported by the C .so: {:?}\n\
         C defined:    {:?}\n\
         Rust defined: {:?}",
        missing.len(),
        missing,
        cdef,
        rdef
    );
}

#[test]
fn rust_so_has_no_unresolved_non_libc_imports() {
    let r = common::rust_so_path();
    let und = nm_undefined(&r);
    let offenders: Vec<&String> = und.iter().filter(|n| !is_runtime_import(n)).collect();
    assert!(
        offenders.is_empty(),
        "Rust .so has undefined non-libc symbols (unfinished translation?): {offenders:?}\n\
         full undefined set: {und:?}"
    );
}

#[test]
fn both_import_the_same_libm_expf() {
    // The bit-identical transcendental results depend on both libraries calling
    // the *same* platform expf rather than a private/vendored implementation.
    for p in [common::c_so_path(), common::rust_so_path()] {
        let out = Command::new("nm").arg("-D").arg("--undefined-only").arg(&p).output().unwrap();
        let text = String::from_utf8_lossy(&out.stdout);
        assert!(
            text.contains("expf"),
            "{} does not import expf from libm; results may not be bit-identical.\n{}",
            p.display(),
            text
        );
    }
}
