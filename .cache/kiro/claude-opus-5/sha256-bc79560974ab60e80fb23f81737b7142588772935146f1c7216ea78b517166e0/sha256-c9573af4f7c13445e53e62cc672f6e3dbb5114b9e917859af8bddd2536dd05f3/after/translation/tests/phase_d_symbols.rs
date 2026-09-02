//! Phase D — symbol parity, checked by running `nm -D` on both `.so`s from
//! inside the test suite so the gate cannot silently rot.

mod common;

use common::pair;
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

fn defined_dynsyms(so: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(so)
        .output()
        .expect("nm must be available");
    assert!(
        out.status.success(),
        "nm -D failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().nth(2).map(str::to_string))
        // Drop toolchain/runtime bookkeeping symbols that are an artifact of
        // how each toolchain links, not part of the library's API surface.
        .filter(|s| {
            !s.starts_with("_init")
                && !s.starts_with("_fini")
                && !s.starts_with("__bss_start")
                && !s.starts_with("_edata")
                && !s.starts_with("_end")
                && !s.starts_with("_ITM_")
                && !s.starts_with("__cxa")
                && !s.starts_with("rust_eh_personality")
        })
        .collect()
}

#[test]
fn every_c_symbol_is_exported_by_rust() {
    let p = pair();
    let c = defined_dynsyms(&p.c_so);
    let r = defined_dynsyms(&p.rust_so);

    println!("C   .so {} -> {:?}", p.c_so.display(), c);
    println!("Rust.so {} -> {:?}", p.rust_so.display(), r);

    let missing: Vec<_> = c.difference(&r).cloned().collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but missing from the Rust .so: {missing:?}"
    );

    // The C header declares exactly one function; make sure it is really there
    // (guards against both sets being empty for an unrelated reason).
    assert!(c.contains("dataentry"), "C .so must export `dataentry`, got {c:?}");
    assert!(r.contains("dataentry"), "Rust .so must export `dataentry`, got {r:?}");
}

#[test]
fn rust_so_has_no_non_libc_undefined_symbols() {
    let p = pair();
    let out = Command::new("nm")
        .arg("-D")
        .arg("--undefined-only")
        .arg(&p.rust_so)
        .output()
        .expect("nm must be available");
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    let mut suspicious = Vec::new();
    for line in text.lines() {
        let sym = match line.split_whitespace().nth(1) {
            Some(s) => s,
            None => continue,
        };
        let base = sym.split('@').next().unwrap_or(sym);
        let known = base.starts_with("_Unwind_")
            || base.starts_with("_ITM_")
            || base.starts_with("__")
            || matches!(
                base,
                "abort"
                    | "bcmp"
                    | "calloc"
                    | "close"
                    | "dl_iterate_phdr"
                    | "free"
                    | "getcwd"
                    | "getenv"
                    | "gettid"
                    | "lseek64"
                    | "malloc"
                    | "memcpy"
                    | "memmove"
                    | "memset"
                    | "mmap64"
                    | "munmap"
                    | "open64"
                    | "posix_memalign"
                    | "read"
                    | "readlink"
                    | "realloc"
                    | "realpath"
                    | "statx"
                    | "stat64"
                    | "fstat64"
                    | "strlen"
                    | "syscall"
                    | "write"
                    | "writev"
            )
            || base.starts_with("pthread_");
        if !known {
            suspicious.push(base.to_string());
        }
    }
    assert!(
        suspicious.is_empty(),
        "Rust .so has non-libc undefined symbols: {suspicious:?}"
    );
}

/// The exported symbol must be callable with the exact C ABI signature and
/// behave identically — a smoke check that the wrapper itself is correct.
#[test]
fn exported_wrapper_is_abi_compatible() {
    let p = pair();
    for (m, a, b, c) in [
        (1, 5, 2, 0),
        (2, 3, 2, 1),
        (3, 2, 1, 5),
        (0, 4, 0, 0),
        (-1, -1, -1, -1),
        (i32::MAX, i32::MIN, i32::MAX, i32::MIN),
    ] {
        p.assert_same(m, a, b, c);
    }
}
