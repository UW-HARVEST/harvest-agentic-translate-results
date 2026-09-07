//! Phase D -- symbol parity between the C `.so` and the Rust `.so`.
//!
//! Mechanised version of `SYMBOLS.md`: the artifact records the state, this test
//! keeps it true.

mod common;

use common::{c_so_path, rust_so_path};
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

fn nm(path: &Path, extra: &str) -> BTreeSet<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg(extra)
        .arg(path)
        .output()
        .unwrap_or_else(|e| panic!("running nm on {}: {e}", path.display()));
    assert!(
        out.status.success(),
        "nm -D {extra} {} failed: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last())
        // Strip the `@GLIBC_2.x` / `@@VER` version suffix nm prints for
        // versioned symbols; the bare name is what matters for ABI parity.
        .map(|s| s.split('@').next().unwrap_or(s).to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Prefixes/names that legitimately come from libc, libgcc's unwinder, or the
/// Rust std runtime. Everything else must be defined by the library itself.
fn is_platform_import(sym: &str) -> bool {
    const PREFIXES: &[&str] = &[
        "_Unwind_",
        "__",
        "_ITM_",
        "pthread_",
    ];
    const NAMES: &[&str] = &[
        "abort", "bcmp", "calloc", "close", "dl_iterate_phdr", "free", "fstat64", "getcwd",
        "getenv", "gettid", "lseek64", "malloc", "memcpy", "memmove", "memset", "mmap64",
        "munmap", "open64", "posix_memalign", "read", "readlink", "realloc", "realpath",
        "stat64", "statx", "strcpy", "strlen", "syscall", "write", "writev",
    ];
    PREFIXES.iter().any(|p| sym.starts_with(p)) || NAMES.contains(&sym)
}

#[test]
fn every_c_symbol_is_exported_by_rust() {
    let c = c_so_path();
    let r = rust_so_path();

    let c_defined = nm(&c, "--defined-only");
    let r_defined = nm(&r, "--defined-only");

    assert!(
        !c_defined.is_empty(),
        "nm found no defined symbols in {}",
        c.display()
    );

    let missing: Vec<&String> = c_defined.difference(&r_defined).collect();
    assert!(
        missing.is_empty(),
        "the Rust .so is missing {} symbol(s) exported by the C .so: {missing:?}\n\
         C defined: {c_defined:?}\nRust defined: {r_defined:?}",
        missing.len()
    );

    // Guard against the artifact drifting: the C library has exactly these five.
    let expected: BTreeSet<String> = [
        "allocate_block",
        "betagamma",
        "compute_hash",
        "create_block",
        "free_block",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    assert_eq!(
        c_defined, expected,
        "the C .so's exported surface changed; SYMBOLS.md must be regenerated"
    );
}

#[test]
fn rust_so_has_no_unresolved_non_libc_symbols() {
    let r = rust_so_path();
    let undefined = nm(&r, "--undefined-only");
    let unexpected: Vec<&String> = undefined.iter().filter(|s| !is_platform_import(s)).collect();
    assert!(
        unexpected.is_empty(),
        "Rust .so imports non-platform symbol(s), i.e. part of the library is \
         missing: {unexpected:?}"
    );
}

#[test]
fn both_libraries_load_with_rtld_now() {
    // RTLD_NOW forces every relocation to be resolved at load time, which is the
    // strongest available check that there are zero unresolvable symbols.
    use libloading::os::unix::{Library, RTLD_LOCAL, RTLD_NOW};
    for p in [c_so_path(), rust_so_path()] {
        unsafe {
            Library::open(Some(&p), RTLD_NOW | RTLD_LOCAL)
                .unwrap_or_else(|e| panic!("RTLD_NOW dlopen of {} failed: {e}", p.display()));
        }
    }
}

#[test]
fn rust_so_uses_the_platform_allocator() {
    // Fidelity requirement, not cosmetics: compute_hash observes raw heap
    // addresses, so the translation must drive the *same* allocator as the C
    // build with the same request sizes in the same order. If the Rust .so
    // stopped importing malloc/calloc/free it would have started using its own
    // allocator and betagamma's results would diverge.
    let undefined = nm(&rust_so_path(), "--undefined-only");
    for needed in ["malloc", "calloc", "free", "strcpy"] {
        assert!(
            undefined.contains(needed),
            "Rust .so does not import {needed} from libc; heap layout will not \
             match the C build"
        );
    }
}
