//! Phase D — symbol parity between the C `.so` and the Rust `.so`.
//!
//! Enforces the completion gate from `SYMBOLS.md`: every symbol the C library
//! exports must also be exported by the Rust cdylib under the exact same name,
//! and the Rust cdylib must have no undefined non-libc symbols.

mod common;

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::process::Command;

/// `nm -D -S --defined-only` -> map of symbol name -> (nm type letter, size).
fn defined_symbols(so: &Path) -> BTreeMap<String, (char, Option<u64>)> {
    let out = Command::new("nm")
        .args(["-D", "-S", "--defined-only"])
        .arg(so)
        .output()
        .unwrap_or_else(|e| panic!("failed to run nm on {}: {e}", so.display()));
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    let mut map = BTreeMap::new();
    for line in text.lines() {
        let f: Vec<&str> = line.split_whitespace().collect();
        // "<addr> <size> <type> <name>"  or  "<addr> <type> <name>"
        match f.len() {
            4 => {
                let size = u64::from_str_radix(f[1], 16).ok();
                map.insert(f[3].to_string(), (f[2].chars().next().unwrap(), size));
            }
            3 => {
                map.insert(f[2].to_string(), (f[1].chars().next().unwrap(), None));
            }
            _ => {}
        }
    }
    map
}

fn undefined_symbols(so: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--undefined-only"])
        .arg(so)
        .output()
        .unwrap_or_else(|e| panic!("failed to run nm on {}: {e}", so.display()));
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    let mut set = BTreeSet::new();
    for line in text.lines() {
        if let Some(name) = line.split_whitespace().last() {
            // strip the "@GLIBC_2.2.5" style version suffix
            let base = name.split('@').next().unwrap_or(name);
            set.insert(base.to_string());
        }
    }
    set
}

/// The eight symbols the C translation unit exports (`c_src/src/lib.c`).
const EXPECTED: [&str; 8] = [
    "init_array",
    "expand_array",
    "add_element",
    "free_array",
    "process_flags",
    "calculate_matrix_checksum",
    "matrixsum",
    "matrix",
];

#[test]
fn d1_every_c_symbol_is_exported_by_rust() {
    let p = common::load();
    let c_syms = defined_symbols(&p.c.path);
    let rs_syms = defined_symbols(&p.rs.path);

    assert!(!c_syms.is_empty(), "nm found no symbols in the C .so");

    let missing: Vec<&String> = c_syms.keys().filter(|k| !rs_syms.contains_key(*k)).collect();
    assert!(
        missing.is_empty(),
        "Rust .so ({}) is MISSING symbols exported by the C .so ({}): {missing:?}",
        p.rs.path.display(),
        p.c.path.display()
    );

    // and the symbol set is exactly the documented one
    let c_names: BTreeSet<&str> = c_syms.keys().map(|s| s.as_str()).collect();
    let expected: BTreeSet<&str> = EXPECTED.iter().copied().collect();
    assert_eq!(
        c_names, expected,
        "the C .so's exported symbol set changed; update SYMBOLS.md"
    );
}

#[test]
fn d2_symbol_kinds_and_data_sizes_match() {
    let p = common::load();
    let c_syms = defined_symbols(&p.c.path);
    let rs_syms = defined_symbols(&p.rs.path);
    for (name, (c_kind, c_size)) in &c_syms {
        let (r_kind, r_size) = rs_syms
            .get(name)
            .unwrap_or_else(|| panic!("Rust .so missing {name}"));
        assert_eq!(
            c_kind.to_ascii_uppercase(),
            r_kind.to_ascii_uppercase(),
            "{name}: nm type letter differs (C={c_kind}, Rust={r_kind})"
        );
        // Text sizes legitimately differ between compilers; DATA objects are ABI.
        if c_kind.eq_ignore_ascii_case(&'D') || c_kind.eq_ignore_ascii_case(&'B') {
            assert_eq!(
                c_size, r_size,
                "{name}: exported data object size differs (C={c_size:?}, Rust={r_size:?})"
            );
        }
    }
}

#[test]
fn d3_all_symbols_are_dlsym_resolvable_in_both() {
    // `common::load()` already dlsym's all eight symbols out of BOTH libraries
    // and panics with the offending name if any is absent; loading proves it.
    let p = common::load();
    for name in EXPECTED {
        assert!(
            defined_symbols(&p.c.path).contains_key(name),
            "C .so missing {name}"
        );
        assert!(
            defined_symbols(&p.rs.path).contains_key(name),
            "Rust .so missing {name}"
        );
    }
    // and they are callable through the FFI boundary
    assert_eq!(p.c.process_flags(0xF), p.rs.process_flags(0xF));
    assert_eq!(
        p.c.calculate_matrix_checksum(),
        p.rs.calculate_matrix_checksum()
    );
    assert_eq!(p.c.matrixsum(1, 2, 3, 4), p.rs.matrixsum(1, 2, 3, 4));
    unsafe {
        let a = p.c.init_array(2);
        let b = p.rs.init_array(2);
        assert_eq!(p.c.add_element(a, 1), p.rs.add_element(b, 1));
        assert_eq!(p.c.expand_array(a), p.rs.expand_array(b));
        p.c.free_array(a);
        p.rs.free_array(b);
    }
}

#[test]
fn d4_rust_has_no_undefined_non_libc_symbols() {
    let p = common::load();
    let undef = undefined_symbols(&p.rs.path);
    // Everything the Rust cdylib imports must come from libc / libgcc's
    // unwinder / the standard weak ELF boilerplate.
    let allowed_prefixes = [
        "_Unwind_",
        "__",
        "_ITM_",
        "pthread_",
        "posix_",
        "gettid",
        "statx",
    ];
    let allowed_exact: BTreeSet<&str> = [
        "abort", "bcmp", "calloc", "close", "dl_iterate_phdr", "free", "fstat", "fstat64",
        "getcwd", "getenv", "lseek", "lseek64", "malloc", "memcmp", "memcpy", "memmove",
        "memset", "mmap", "mmap64", "munmap", "open", "open64", "read", "readlink", "realloc",
        "realpath", "stat", "stat64", "strlen", "syscall", "write", "writev", "sysconf",
        "getrandom", "sigaction", "sigaltstack", "mprotect", "pipe2", "poll", "signal",
        "raise", "exit", "environ", "qsort", "strerror_r", "madvise", "nanosleep",
        "clock_gettime", "sched_getaffinity", "sched_yield", "getpid", "openat", "openat64",
    ]
    .into_iter()
    .collect();

    let bad: Vec<&String> = undef
        .iter()
        .filter(|s| {
            !allowed_exact.contains(s.as_str())
                && !allowed_prefixes.iter().any(|pre| s.starts_with(pre))
        })
        .collect();
    assert!(
        bad.is_empty(),
        "Rust .so has undefined non-libc symbols: {bad:?}"
    );

    // The three allocator symbols the C imports must be imported by Rust too,
    // otherwise allocator-edge behaviour (malloc(0), realloc(p, 0)) could differ.
    for a in ["malloc", "realloc", "free"] {
        assert!(
            undef.contains(a),
            "Rust .so does not import glibc `{a}` — allocator behaviour could diverge"
        );
    }
}
