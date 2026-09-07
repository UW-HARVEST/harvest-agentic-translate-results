// Phase D — symbol parity between the C .so and the Rust .so.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::process::Command;

fn defined_symbols(path: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(path)
        .output()
        .expect("run nm");
    assert!(out.status.success(), "nm failed on {path:?}");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().nth(2).map(|s| s.to_string()))
        .collect()
}

#[test]
fn rust_so_exports_every_c_symbol() {
    let c = defined_symbols(&c_so_path());
    let r = defined_symbols(&rust_so_path());
    assert!(!c.is_empty(), "no symbols read from the C .so");
    let missing: Vec<_> = c.difference(&r).cloned().collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}\n\
         C: {c:?}\nRust: {r:?}"
    );
}

#[test]
fn expected_symbol_set() {
    let c = defined_symbols(&c_so_path());
    for s in ["driver", "run"] {
        assert!(c.contains(s), "C .so unexpectedly lacks `{s}`");
    }
    let r = defined_symbols(&rust_so_path());
    for s in ["driver", "run"] {
        assert!(r.contains(s), "Rust .so lacks `{s}`");
    }
    // `static` C helpers must not leak out of either library.
    for s in ["add_floor", "add_bedrooms", "print_house", "parse_val"] {
        assert!(!c.contains(s), "C .so unexpectedly exports static `{s}`");
        assert!(
            !r.contains(s),
            "Rust .so exports `{s}`, but it is `static` in C"
        );
    }
}

#[test]
fn no_undefined_non_libc_symbols_in_rust_so() {
    let out = Command::new("nm")
        .args(["-D", "--undefined-only"])
        .arg(rust_so_path())
        .output()
        .expect("run nm");
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    let offenders: Vec<&str> = text
        .lines()
        .filter_map(|l| l.split_whitespace().last())
        .filter(|s| {
            let name = s.split('@').next().unwrap_or(s);
            // Anything that is not a libc / libgcc / ITM runtime import would be
            // an untranslated project dependency.
            !(name.starts_with("_Unwind_")
                || name.starts_with("_ITM_")
                || name.starts_with("__")
                || name.starts_with("pthread_")
                || matches!(
                    name,
                    "printf"
                        | "puts"
                        | "strtol"
                        | "strlen"
                        | "abort"
                        | "bcmp"
                        | "calloc"
                        | "close"
                        | "dl_iterate_phdr"
                        | "free"
                        | "fstat"
                        | "fstat64"
                        | "getcwd"
                        | "getenv"
                        | "gettid"
                        | "lseek"
                        | "lseek64"
                        | "malloc"
                        | "memcpy"
                        | "memmove"
                        | "memset"
                        | "mmap"
                        | "mmap64"
                        | "munmap"
                        | "open"
                        | "open64"
                        | "posix_memalign"
                        | "read"
                        | "readlink"
                        | "realloc"
                        | "realpath"
                        | "stat"
                        | "stat64"
                        | "statx"
                        | "syscall"
                        | "write"
                        | "writev"
                        | "sysconf"
                        | "poll"
                        | "sigaction"
                        | "sigaltstack"
                        | "signal"
                        | "raise"
                ))
        })
        .collect();
    assert!(
        offenders.is_empty(),
        "Rust .so has non-libc undefined symbols: {offenders:?}"
    );
}

#[test]
fn both_libs_load_and_resolve() {
    let l = libs();
    let _ = l.c.driver();
    let _ = l.c.run();
    let _ = l.rust.driver();
    let _ = l.rust.run();
}
