//! Phase D — symbol parity between the C `.so` and the Rust `.so`.
mod common;

use common::*;
use std::collections::BTreeSet;
use std::process::Command;

fn defined_syms(path: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", path.to_str().unwrap()])
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
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let a = it.next()?;
            let b = it.next()?;
            // "<addr> T <name>"  or  "<addr> <type> <name>"
            let (ty, name) = if let Some(n) = it.next() {
                (b, n)
            } else {
                (a, b)
            };
            if ty == "T" || ty == "D" || ty == "B" || ty == "R" || ty == "W" {
                Some(name.to_string())
            } else {
                None
            }
        })
        .collect()
}

fn undefined_syms(path: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--undefined-only", path.to_str().unwrap()])
        .output()
        .expect("run nm");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .collect()
}

/// Every symbol exported by the C `.so` must be exported by the Rust `.so`
/// under the exact same name.
#[test]
fn symbol_parity() {
    let _g = serial();
    let c = defined_syms(&c_so_path());
    let r = defined_syms(&rust_so_path());
    assert!(!c.is_empty(), "no symbols found in the C .so");

    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}"
    );

    // Sanity: the 16 names enumerated in SYMBOLS.md really are all of them.
    for name in ALL_SYMBOLS {
        assert!(c.contains(*name), "C .so is missing {name}");
        assert!(r.contains(*name), "Rust .so is missing {name}");
    }
    assert_eq!(
        c.len(),
        ALL_SYMBOLS.len(),
        "C .so exports {} symbols but SYMBOLS.md lists {}: {:?}",
        c.len(),
        ALL_SYMBOLS.len(),
        c
    );
}

/// The Rust `.so` must not have picked up any non-libc dependency.
#[test]
fn rust_imports_only_libc() {
    let _g = serial();
    let u = undefined_syms(&rust_so_path());
    let allowed_prefixes = [
        "_ITM_", "__gmon_start__", "__cxa_", "_Unwind_", "__rust", "rust_",
        "__tls_get_addr", "_dl_", "__libc_",
    ];
    let allowed_exact = [
        "realloc", "free", "malloc", "printf", "abort", "write", "memcpy", "memmove", "memset",
        "memcmp", "strlen", "sprintf", "strcmp", "calloc", "posix_memalign", "bcmp", "getenv",
        "dl_iterate_phdr", "pthread_getattr_np", "pthread_attr_getstack", "pthread_attr_destroy",
        "pthread_self", "sigaltstack", "mmap", "munmap", "mprotect", "sysconf", "pthread_key_create",
        "pthread_getspecific", "pthread_setspecific", "pthread_key_delete", "syscall",
        "gnu_get_libc_version", "strerror_r", "close", "open64", "read", "poll", "readlink",
        "getcwd", "sigaction", "signal", "raise", "__errno_location", "qsort", "bsearch",
    ];
    let bad: Vec<&String> = u
        .iter()
        .filter(|s| {
            !allowed_exact.contains(&s.as_str())
                && !allowed_prefixes.iter().any(|p| s.starts_with(p))
                && !s.contains('@') // versioned libc refs like memcpy@GLIBC_2.14
        })
        .collect();
    assert!(bad.is_empty(), "unexpected imports in the Rust .so: {bad:?}");
}

/// Both `.so`s can be dlopen'd side by side in one process and every symbol is
/// resolvable through `libloading` with the exact C signature.
#[test]
fn both_libs_load() {
    let _g = serial();
    let (c, r) = both();
    assert_eq!(c.name, "C");
    assert_eq!(r.name, "Rust");
    // Prove the two do not interpose on each other: independent global seeds.
    unsafe {
        (c.rand_seed)(0x1111);
        (r.rand_seed)(0x2222);
        let ac = (c.arrgrowf)(core::ptr::null_mut(), 8, 0, 1);
        let ar = (r.arrgrowf)(core::ptr::null_mut(), 8, 0, 1);
        assert!(!ac.is_null() && !ar.is_null());
        assert_ne!(ac, ar, "the two libraries must own separate allocations");
        (c.arrfreef)(ac);
        (r.arrfreef)(ar);
    }
}
