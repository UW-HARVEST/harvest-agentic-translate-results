//! Phase D — symbol parity, enforced as a test rather than a one-off command.
//!
//! Runs `nm -D` on both shared objects and asserts the exported-symbol sets are
//! identical. This is the gate that catches a partially translated library: a
//! Rust `.so` exporting only a subset of the C `.so`'s symbols.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

/// Symbols the C source defines, split by linkage. Derived from
/// `c_src/src/driver.c`, cross-checked against `nm -D` below.
const EXPECTED_EXPORTS: [&str; 4] = ["bad", "driver", "good", "printLine"];
const EXPECTED_PRIVATE: [&str; 2] = ["helperBad", "helperGood"];

fn nm(path: &Path, extra: &[&str]) -> Vec<String> {
    let mut cmd = Command::new("nm");
    cmd.arg("-D");
    cmd.args(extra);
    cmd.arg(path);
    let out = cmd
        .output()
        .unwrap_or_else(|e| panic!("running nm on {}: {e}", path.display()));
    assert!(
        out.status.success(),
        "nm on {} failed: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|l| l.to_string())
        .collect()
}

/// Names of symbols *defined* by the object (nm type not `U` and not weak).
fn defined_symbols(path: &Path) -> BTreeSet<String> {
    nm(path, &["--defined-only"])
        .iter()
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            let (_addr, b) = (parts.next()?, parts.next()?);
            // "<addr> <type> <name>" for defined symbols; weak-undefined lines
            // ("w name") have only two fields and are filtered out here.
            let (ty, name) = match parts.next() {
                Some(name) => (b, name),
                None => return None,
            };
            // Skip weak/absolute link-editor bookkeeping symbols.
            if matches!(ty, "w" | "V" | "A" | "U") {
                return None;
            }
            Some(name.to_string())
        })
        .collect()
}

/// Names of symbols the object *imports*, stripped of their version suffix.
fn undefined_symbols(path: &Path) -> BTreeSet<String> {
    nm(path, &["-u"])
        .iter()
        .filter_map(|line| {
            let name = line.split_whitespace().last()?;
            Some(name.split('@').next().unwrap_or(name).to_string())
        })
        .collect()
}

#[test]
fn phase_d_exported_symbol_sets_are_identical() {
    let c = c_so_path_pub();
    let rust = rust_so_path();

    let c_syms = defined_symbols(&c);
    let rust_syms = defined_symbols(&rust);

    let missing: Vec<_> = c_syms.difference(&rust_syms).cloned().collect();
    let extra: Vec<_> = rust_syms.difference(&c_syms).cloned().collect();

    assert!(
        missing.is_empty(),
        "the Rust .so is missing {} symbol(s) exported by the C .so: {missing:?}\n\
         C: {c_syms:?}\nRust: {rust_syms:?}",
        missing.len()
    );
    assert!(
        extra.is_empty(),
        "the Rust .so exports {} symbol(s) the C .so does not: {extra:?}",
        extra.len()
    );

    // Pin the absolute expectation too, so an empty-vs-empty diff cannot pass.
    let expected: BTreeSet<String> = EXPECTED_EXPORTS.iter().map(|s| s.to_string()).collect();
    assert_eq!(
        c_syms, expected,
        "the C .so's export set changed; update SYMBOLS.md"
    );
    assert_eq!(rust_syms, expected);
}

#[test]
fn phase_d_static_helpers_absent_from_both() {
    for name in EXPECTED_PRIVATE {
        for path in [c_so_path_pub(), rust_so_path()] {
            assert!(
                !defined_symbols(&path).contains(name),
                "{} exports `{name}`, which the C declares `static`",
                path.display()
            );
        }
    }
}

#[test]
fn phase_d_rust_imports_only_libc_and_unwinder_symbols() {
    // Every symbol the Rust .so imports must be a C-runtime / unwinder symbol,
    // not an untranslated library function that was left dangling.
    let allowed_prefixes = [
        "_Unwind_", "__", "_ITM_", "pthread_", "posix_", "std", "_",
    ];
    let allowed_exact: BTreeSet<&str> = [
        "abort", "bcmp", "calloc", "close", "dl_iterate_phdr", "free", "fstat", "fstat64",
        "getcwd", "getenv", "gettid", "lseek", "lseek64", "malloc", "memcmp", "memcpy", "memmove",
        "memset", "mmap", "mmap64", "munmap", "open", "open64", "puts", "printf", "fwrite",
        "fputc", "read", "readlink", "realloc", "realpath", "stat", "stat64", "statx", "strlen",
        "syscall", "write", "writev", "sysconf", "getrandom", "poll", "sigaction", "sigaltstack",
        "mprotect", "madvise",
    ]
    .into_iter()
    .collect();

    let rust_imports = undefined_symbols(&rust_so_path());
    let suspicious: Vec<_> = rust_imports
        .iter()
        .filter(|s| {
            !allowed_exact.contains(s.as_str())
                && !allowed_prefixes.iter().any(|p| s.starts_with(p))
        })
        .cloned()
        .collect();
    assert!(
        suspicious.is_empty(),
        "the Rust .so imports non-libc symbol(s), suggesting untranslated code: {suspicious:?}"
    );

    // The C .so imports exactly one non-weak libc symbol; make sure the Rust
    // .so imports it too (i.e. it really does route output through libc stdio
    // rather than Rust's own buffered writer).
    let c_imports = undefined_symbols(&c_so_path_pub());
    assert!(
        c_imports.contains("puts"),
        "unexpected C import set: {c_imports:?}"
    );
    assert!(
        rust_imports.contains("puts") || rust_imports.contains("printf"),
        "the Rust .so imports neither puts nor printf; it is not using libc stdio: {rust_imports:?}"
    );
}

#[test]
fn phase_d_no_stub_markers_in_rust_source() {
    // Guard against a symbol being made to "appear" in nm -D by a stub.
    let src = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs"),
    )
    .expect("read src/lib.rs");
    for marker in ["unimplemented!", "todo!", "unreachable!"] {
        assert!(
            !src.contains(marker),
            "src/lib.rs contains `{marker}` — a stub that lies about behaviour"
        );
    }
}
