//! Phase D — symbol parity between the C `.so` and the Rust `.so`.
//!
//! Runs `nm -D --defined-only` on both objects and requires the C's exported
//! symbol set to be a subset of the Rust's (and, here, exactly equal).

#![allow(non_snake_case)]

mod common;

use common::{c_so_path_pub, libs, rust_so_path_pub, Api};
use std::collections::BTreeSet;
use std::process::Command;

fn defined_dynamic_symbols(so: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", so.to_str().unwrap()])
        .output()
        .expect("`nm` must be available to run the symbol-parity test");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let a = it.next()?;
            let b = it.next()?;
            match it.next() {
                // "<addr> T name"
                Some(name) => Some((b.to_string(), name.to_string())),
                // "         U name" / "w name"
                None => Some((a.to_string(), b.to_string())),
            }
        })
        // Only globally-visible *defined* symbols (T/D/B/R/W/V).
        .filter(|(kind, _)| matches!(kind.as_str(), "T" | "D" | "B" | "R" | "W" | "V"))
        .map(|(_, name)| name)
        .collect()
}

#[test]
fn c_exports_are_all_present_in_rust() {
    let c = defined_dynamic_symbols(&c_so_path_pub());
    let r = defined_dynamic_symbols(&rust_so_path_pub());

    assert!(!c.is_empty(), "the C .so exported nothing — bad build?");

    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "\n{} C symbol(s) MISSING from the Rust .so: {:?}\n\
         C  ({}): {:?}\nRUST ({}): {:?}\n",
        missing.len(),
        missing,
        c.len(),
        c,
        r.len(),
        r
    );

    // Informational: the Rust side must not be a strict superset either, since
    // the C's `static inline` helpers are deliberately private in Rust too.
    let extra: Vec<&String> = r.difference(&c).collect();
    assert!(
        extra.is_empty(),
        "Rust .so exports {} symbol(s) the C does not: {:?}",
        extra.len(),
        extra
    );

    assert_eq!(c.len(), 28, "expected 28 exported C symbols, got {}", c.len());
    assert_eq!(c, r);
}

#[test]
fn every_symbol_the_harness_needs_resolves_in_both_libraries() {
    // `libs()` panics if any of the 28 symbols is missing from either object.
    let p = libs();
    assert_eq!(p.c.tag, "C");
    assert_eq!(p.rs.tag, "RUST");
    assert_eq!(Api::SYMBOLS.len(), 28);
}

#[test]
fn rust_so_has_no_undefined_non_libc_symbols() {
    let so = rust_so_path_pub();
    let out = Command::new("nm")
        .args(["-D", "--undefined-only", so.to_str().unwrap()])
        .output()
        .expect("nm");
    let text = String::from_utf8_lossy(&out.stdout);

    // Everything the Rust cdylib imports must come from libc / libgcc_s / the
    // dynamic loader. Anything else would mean an un-translated dependency.
    let allow_prefixes = [
        "_ITM_",
        "_Unwind_",
        "__cxa_",
        "__errno_location",
        "__gmon_start__",
        "__tls_get_addr",
        "__libc_",
        "__assert",
        "__stack_chk",
        "_dl_",
    ];
    let allow_exact = [
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
        "memchr",
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
        "pthread_getspecific",
        "pthread_key_create",
        "pthread_key_delete",
        "pthread_setspecific",
        "pthread_mutex_lock",
        "pthread_mutex_trylock",
        "pthread_mutex_unlock",
        "pthread_self",
        "read",
        "readlink",
        "realloc",
        "realpath",
        "sigaltstack",
        "sigaction",
        "sigemptyset",
        "stat",
        "stat64",
        "statx",
        "strlen",
        "sysconf",
        "syscall",
        "write",
        "writev",
        "sqrtf",
        "sqrt",
        "qsort",
        "mprotect",
        "pipe2",
        "poll",
    ];

    let mut bad = Vec::new();
    for line in text.lines() {
        let name = match line.split_whitespace().last() {
            Some(n) => n,
            None => continue,
        };
        let base = name.split('@').next().unwrap();
        if allow_prefixes.iter().any(|p| base.starts_with(p)) {
            continue;
        }
        if allow_exact.contains(&base) {
            continue;
        }
        bad.push(base.to_string());
    }
    assert!(
        bad.is_empty(),
        "Rust .so imports non-libc symbols (un-translated dependency?): {:?}",
        bad
    );
}
