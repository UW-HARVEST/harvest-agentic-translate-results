//! Phase D — symbol parity between the C `.so` and the Rust `.so`.
//!
//! Every dynamic symbol the C library exports must also be exported by the Rust
//! library under the exact same name, and the Rust library must not import any
//! non-libc symbol.

mod common;

use std::collections::BTreeSet;
use std::process::Command;

fn nm(args: &[&str], path: &str) -> String {
    let out = Command::new("nm")
        .args(args)
        .arg(path)
        .output()
        .unwrap_or_else(|e| panic!("failed to run nm on {path}: {e}"));
    assert!(
        out.status.success(),
        "nm failed on {path}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// Defined (exported) dynamic text/data symbols.
fn defined_symbols(path: &str) -> BTreeSet<String> {
    nm(&["-D", "--defined-only"], path)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let _addr = it.next()?;
            let kind = it.next()?;
            let name = it.next()?;
            matches!(kind, "T" | "t" | "D" | "B" | "R").then(|| name.to_string())
        })
        .collect()
}

fn undefined_symbols(path: &str) -> BTreeSet<String> {
    nm(&["-D", "--undefined-only"], path)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .collect()
}

/// Symbols that legitimately come from libc / libgcc / the dynamic loader.
fn is_platform_symbol(name: &str) -> bool {
    const PREFIXES: [&str; 6] = ["_Unwind_", "__", "_ITM_", "_dl_", "pthread_", "_rust_"];
    const EXACT: [&str; 30] = [
        "abort", "bcmp", "calloc", "close", "dl_iterate_phdr", "free", "fstat64", "getcwd",
        "getenv", "gettid", "lseek64", "malloc", "memcpy", "memmove", "memset", "mmap64",
        "munmap", "open64", "posix_memalign", "read", "readlink", "realloc", "realpath",
        "stat64", "statx", "strlen", "syscall", "write", "writev", "sqrtf",
    ];
    let base = name.split('@').next().unwrap_or(name);
    PREFIXES.iter().any(|p| base.starts_with(p)) || EXACT.contains(&base)
}

#[test]
fn every_c_symbol_is_exported_by_rust() {
    let c_path = common::c_so_path();
    let r_path = common::rust_so_path();
    let c_syms = defined_symbols(c_path.to_str().unwrap());
    let r_syms = defined_symbols(r_path.to_str().unwrap());

    assert!(
        !c_syms.is_empty(),
        "no symbols read from the C .so at {}",
        c_path.display()
    );

    let missing: Vec<&String> = c_syms.difference(&r_syms).collect();
    assert!(
        missing.is_empty(),
        "{} C symbol(s) are NOT exported by the Rust .so ({}): {:?}\n\
         C .so:    {}\nRust .so: {}",
        missing.len(),
        r_syms.len(),
        missing,
        c_path.display(),
        r_path.display()
    );

    // Sanity: the full documented surface really is there.
    const EXPECTED: [&str; 31] = [
        "c22", "c23", "c2Add", "c2BBVerts", "c2CCW90", "c2Clampv", "c2D", "c2Det2", "c2Div",
        "c2Dot", "c2GJK", "c2GJKSimplexMetric", "c2L", "c2Len", "c2MakeProxy", "c2Maxv",
        "c2Minv", "c2Mulrv", "c2MulrvT", "c2Mulvs", "c2Mulxv", "c2Neg", "c2Norm",
        "c2RotIdentity", "c2Skew", "c2Sub", "c2Support", "c2V", "c2Witness", "c2xIdentity",
        "gjk",
    ];
    for s in EXPECTED {
        assert!(c_syms.contains(s), "C .so unexpectedly lacks {s}");
        assert!(r_syms.contains(s), "Rust .so lacks {s}");
    }
    assert_eq!(
        c_syms.len(),
        EXPECTED.len(),
        "C .so exports {} symbols, SYMBOLS.md documents {}: {:?}",
        c_syms.len(),
        EXPECTED.len(),
        c_syms
    );
}

#[test]
fn rust_so_has_no_unresolved_non_libc_symbols() {
    let r_path = common::rust_so_path();
    let undef = undefined_symbols(r_path.to_str().unwrap());
    let bad: Vec<&String> = undef.iter().filter(|s| !is_platform_symbol(s)).collect();
    assert!(
        bad.is_empty(),
        "Rust .so has unresolved non-libc symbol(s): {bad:?}"
    );
}
