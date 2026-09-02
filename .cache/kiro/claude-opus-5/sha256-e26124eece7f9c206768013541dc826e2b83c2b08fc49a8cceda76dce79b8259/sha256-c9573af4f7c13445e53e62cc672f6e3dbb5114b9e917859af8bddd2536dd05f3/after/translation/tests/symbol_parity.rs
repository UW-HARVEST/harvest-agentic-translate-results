//! Phase D — symbol parity between the two shared objects.
//!
//! Enforces the `SYMBOLS.md` gate mechanically: every symbol the C `.so`
//! exports must also be exported by the Rust `.so` under the exact same name,
//! and the Rust `.so` must not depend on any non-libc symbol.

mod harness;

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

fn nm(path: &Path, extra: &str) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", extra])
        .arg(path)
        .output()
        .unwrap_or_else(|e| panic!("failed to run nm on {}: {e}", path.display()));
    assert!(
        out.status.success(),
        "nm {extra} {} failed: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|line| {
            // "<addr> <type> <name>" or "        <type> <name>" (undefined)
            let mut it = line.split_whitespace().rev();
            let name = it.next()?;
            let kind = it.next()?;
            if kind.len() != 1 {
                return None;
            }
            Some(name.split('@').next().unwrap_or(name).to_string())
        })
        .collect()
}

/// Symbols the Rust standard library legitimately imports from glibc/libgcc,
/// on top of the four the C library itself imports.
const ALLOWED_IMPORT_PREFIXES: &[&str] = &["_Unwind_", "__", "_ITM_", "pthread_"];

const ALLOWED_IMPORTS: &[&str] = &[
    // Imported by the C library too.
    "memchr",
    "pow",
    "printf",
    "puts",
    // Rust std runtime.
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
    "sigaction",
    "sigaltstack",
    "stat",
    "stat64",
    "statx",
    "strlen",
    "syscall",
    "sysconf",
    "write",
    "writev",
];

#[test]
fn symbol_parity_defined() {
    let p = harness::apis();
    let c_syms = nm(&p.c.path, "--defined-only");
    let r_syms = nm(&p.rust.path, "--defined-only");

    assert!(
        !c_syms.is_empty(),
        "nm found no exported symbols in {}",
        p.c.path.display()
    );

    let missing: Vec<&String> = c_syms.difference(&r_syms).collect();
    assert!(
        missing.is_empty(),
        "Rust .so ({}) is missing {} symbol(s) exported by the C .so ({}): {:?}\n\
         C exports  : {:?}\nRust exports: {:?}",
        p.rust.path.display(),
        missing.len(),
        p.c.path.display(),
        missing,
        c_syms,
        r_syms
    );

    // Documented in SYMBOLS.md: exactly these six.
    let expected: BTreeSet<String> = [
        "calculate_with_doubles",
        "convert_double_to_int",
        "create_numeric_buffer",
        "doubleneg",
        "find_value_in_buffer",
        "process_negation",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    assert_eq!(
        c_syms, expected,
        "the C .so's exported set changed; SYMBOLS.md must be regenerated"
    );
}

#[test]
fn symbol_parity_undefined() {
    let p = harness::apis();
    let undefined = nm(&p.rust.path, "--undefined-only");
    let unexpected: Vec<&String> = undefined
        .iter()
        .filter(|s| {
            !ALLOWED_IMPORTS.contains(&s.as_str())
                && !ALLOWED_IMPORT_PREFIXES.iter().any(|pre| s.starts_with(pre))
        })
        .collect();
    assert!(
        unexpected.is_empty(),
        "Rust .so has {} undefined non-libc symbol(s): {:?}",
        unexpected.len(),
        unexpected
    );
}

#[test]
fn both_libraries_actually_loaded_from_distinct_files() {
    // Guards against the harness accidentally loading the same object twice,
    // which would make every differential assertion vacuous.
    let p = harness::apis();
    assert_ne!(
        std::fs::canonicalize(&p.c.path).unwrap(),
        std::fs::canonicalize(&p.rust.path).unwrap(),
        "the C and Rust paths resolve to the same file"
    );
    // And that the function pointers really come from different mappings.
    assert_ne!(
        p.c.doubleneg as usize, p.rust.doubleneg as usize,
        "doubleneg resolved to the same address in both libraries"
    );
    assert_ne!(
        p.c.convert_double_to_int as usize, p.rust.convert_double_to_int as usize,
        "convert_double_to_int resolved to the same address in both libraries"
    );
}
