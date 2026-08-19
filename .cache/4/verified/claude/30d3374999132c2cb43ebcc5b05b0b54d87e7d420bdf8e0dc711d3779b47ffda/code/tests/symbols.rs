//! Phase D — exported-symbol parity between the C and the Rust shared object.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::process::Command;

fn dynamic_symbols(path: &std::path::Path, args: &[&str]) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(args)
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
        .filter_map(|l| l.split_whitespace().last())
        .map(|s| s.split('@').next().unwrap_or(s).to_string())
        .collect()
}

/// Every symbol exported by the C `.so` must be exported by the Rust `.so`.
#[test]
fn c_exports_are_a_subset_of_rust_exports() {
    let c = dynamic_symbols(&c_so(), &["-D", "--defined-only"]);
    let r = dynamic_symbols(&rust_so(), &["-D", "--defined-only"]);

    assert_eq!(
        c,
        BTreeSet::from(["main".to_string()]),
        "the C shared object is expected to export exactly `main`"
    );

    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but missing from the Rust .so: {missing:?}"
    );
}

/// The Rust `.so` must not have unresolved non-libc dependencies: every
/// undefined symbol has to be satisfiable from the platform libraries.
#[test]
fn rust_so_has_no_unresolved_non_libc_symbols() {
    let undef = dynamic_symbols(&rust_so(), &["-D", "--undefined-only"]);
    // Everything the Rust standard library imports from glibc / libgcc.
    let allowed_prefixes = [
        "_ITM_", "_Unwind_", "__cxa_", "__tls_", "__errno", "__gmon_", "pthread_", "_dl_",
    ];
    let allowed_exact: BTreeSet<&str> = [
        "abort",
        "bcmp",
        "calloc",
        "close",
        "dl_iterate_phdr",
        "free",
        "fstat",
        "fstat64",
        "getcwd",
        "getenv",
        "gettid",
        "lseek",
        "lseek64",
        "malloc",
        "memcmp",
        "memcpy",
        "memmove",
        "memset",
        "mmap",
        "mmap64",
        "munmap",
        "open",
        "open64",
        "posix_memalign",
        "read",
        "readlink",
        "realloc",
        "realpath",
        "stat",
        "stat64",
        "statx",
        "strlen",
        "syscall",
        "write",
        "writev",
    ]
    .into_iter()
    .collect();

    let unexpected: Vec<&String> = undef
        .iter()
        .filter(|s| {
            !allowed_exact.contains(s.as_str())
                && !allowed_prefixes.iter().any(|p| s.starts_with(p))
        })
        .collect();
    assert!(
        unexpected.is_empty(),
        "the Rust .so imports symbols that are neither libc nor libgcc: {unexpected:?}"
    );

    // And the loader must be able to resolve all of them.
    let out = Command::new("ldd")
        .arg("-r")
        .arg(rust_so())
        .output()
        .expect("run ldd");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        !text.contains("undefined symbol"),
        "ldd -r reports unresolved symbols:\n{text}"
    );
}

/// Both shared objects must be loadable and expose a callable `main`.
#[test]
fn both_libraries_expose_a_callable_main() {
    let out = assert_same(&[b"driver", b"abc", b"1"], Layout::Contiguous);
    assert_eq!(out.status, 0);
    assert_eq!(out.stdout, b"bc\n");
}
