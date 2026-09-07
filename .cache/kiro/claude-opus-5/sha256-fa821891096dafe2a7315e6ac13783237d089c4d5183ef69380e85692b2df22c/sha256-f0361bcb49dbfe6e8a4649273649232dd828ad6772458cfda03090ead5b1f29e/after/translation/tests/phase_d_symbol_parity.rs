//! Phase D — symbol parity, enforced mechanically from `nm -D`.
//!
//! Asserts the Rust `.so` exports every symbol the C `.so` exports, with the
//! exact same name and the same nm type letter, and that the Rust `.so` has no
//! undefined non-libc symbols. This is the executable form of `SYMBOLS.md`.

mod common;

use common::*;
use std::collections::BTreeMap;
use std::process::Command;

/// (name -> nm type letter) for defined, dynamic symbols.
fn defined_dynamic_symbols(so: &std::path::Path) -> BTreeMap<String, char> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(so)
        .output()
        .expect("failed to run `nm` (binutils required)");
    assert!(
        out.status.success(),
        "nm failed on {:?}: {}",
        so,
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8_lossy(&out.stdout);
    let mut map = BTreeMap::new();
    for line in text.lines() {
        let cols: Vec<&str> = line.split_whitespace().collect();
        // "<addr> <type> <name>"
        if cols.len() >= 3 {
            let ty = cols[cols.len() - 2];
            let name = cols[cols.len() - 1];
            if ty.len() == 1 {
                map.insert(name.to_string(), ty.chars().next().unwrap());
            }
        }
    }
    map
}

fn undefined_dynamic_symbols(so: &std::path::Path) -> Vec<String> {
    let out = Command::new("nm")
        .args(["-D", "-u"])
        .arg(so)
        .output()
        .expect("failed to run `nm`");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(str::to_string))
        .collect()
}

/// Symbols the Rust std runtime legitimately imports from libc / libgcc.
fn is_runtime_import(name: &str) -> bool {
    let base = name.split('@').next().unwrap_or(name);
    if base.starts_with("_Unwind_")
        || base.starts_with("__cxa_")
        || base.starts_with("_ITM_")
        || base.starts_with("pthread_")
        || base.starts_with("__")
    {
        return true;
    }
    matches!(
        base,
        // allocator (shared with the C)
        "malloc" | "realloc" | "free" | "calloc" | "posix_memalign"
            // mem/str primitives
            | "memcpy" | "memmove" | "memset" | "bcmp" | "memcmp" | "strlen"
            // libc misc pulled in by std
            | "abort" | "getenv" | "getcwd" | "realpath" | "readlink" | "syscall"
            | "dl_iterate_phdr" | "gettid" | "statx"
            // fs / io syscalls used by std's panic + backtrace machinery
            | "open" | "open64" | "close" | "read" | "write" | "writev" | "lseek"
            | "lseek64" | "fstat" | "fstat64" | "stat" | "stat64" | "mmap" | "mmap64"
            | "munmap"
    )
}

#[test]
fn phase_d_rust_so_exports_every_c_symbol() {
    let c = defined_dynamic_symbols(&c_so_path());
    let r = defined_dynamic_symbols(&rust_so_path());

    assert!(!c.is_empty(), "no symbols read from the C .so — is it built?");

    // Sanity: the full known export surface of lib.c must be present in the C .so.
    for expected in [
        "init_array",
        "expand_array",
        "add_element",
        "free_array",
        "process_flags",
        "calculate_matrix_checksum",
        "matrixsum",
        "matrix",
    ] {
        assert!(c.contains_key(expected), "C .so unexpectedly lacks {expected}");
    }

    let missing: Vec<&String> = c.keys().filter(|k| !r.contains_key(*k)).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is MISSING {} symbol(s) exported by the C .so: {:?}\n\
         C exports:    {:?}\n\
         Rust exports: {:?}",
        missing.len(),
        missing,
        c.keys().collect::<Vec<_>>(),
        r.keys().collect::<Vec<_>>()
    );

    // Same nm type letter: a function must not become a data object, etc.
    for (name, cty) in &c {
        let rty = r[name];
        assert_eq!(
            *cty, rty,
            "symbol {name} has nm type '{cty}' in C but '{rty}' in Rust"
        );
    }
}

#[test]
fn phase_d_rust_so_has_no_unresolved_non_libc_symbols() {
    let undef = undefined_dynamic_symbols(&rust_so_path());
    let bad: Vec<&String> = undef.iter().filter(|s| !is_runtime_import(s)).collect();
    assert!(
        bad.is_empty(),
        "Rust .so has undefined non-libc symbols: {:?}",
        bad
    );
}

/// The exported `matrix` data object must have identical size in both `.so`s,
/// otherwise a caller writing all 12 cells would corrupt one of them.
#[test]
fn phase_d_matrix_data_object_same_size() {
    let size_of = |so: &std::path::Path| -> u64 {
        let out = Command::new("readelf")
            .args(["-sW"])
            .arg(so)
            .output()
            .expect("failed to run `readelf`");
        let text = String::from_utf8_lossy(&out.stdout);
        for line in text.lines() {
            let cols: Vec<&str> = line.split_whitespace().collect();
            if cols.last() == Some(&"matrix") && cols.len() >= 4 {
                if let Ok(sz) = cols[2].parse::<u64>() {
                    return sz;
                }
            }
        }
        panic!("no `matrix` OBJECT found in {:?}", so);
    };
    let cs = size_of(&c_so_path());
    let rs = size_of(&rust_so_path());
    assert_eq!(cs, rs, "matrix object size differs (C {cs} vs Rust {rs})");
    assert_eq!(cs, 48, "expected 3*4*sizeof(int) == 48 bytes");
}

/// Every exported symbol must actually be resolvable through `dlsym`, not just
/// present in the symbol table.
#[test]
fn phase_d_all_symbols_dlsym_resolvable() {
    // `load_pair` panics if any of the 8 symbols is missing from either .so.
    let p = load_pair();
    unsafe {
        // touch every one so an unresolved lazy binding would surface here
        let a = p.c.init_array(2);
        let b = p.rust.init_array(2);
        assert!(!a.is_null() && !b.is_null());
        diff_eq!(p.c.add_element(a, 1), p.rust.add_element(b, 1), "add_element");
        diff_eq!(p.c.expand_array(a), p.rust.expand_array(b), "expand_array");
        p.c.free_array(a);
        p.rust.free_array(b);
        diff_eq!(p.c.process_flags(5), p.rust.process_flags(5), "process_flags");
        diff_eq!(
            p.c.calculate_matrix_checksum(),
            p.rust.calculate_matrix_checksum(),
            "calculate_matrix_checksum"
        );
        diff_eq!(p.c.matrixsum(1, 2, 3, 4), p.rust.matrixsum(1, 2, 3, 4), "matrixsum");
        diff_eq!(p.c.read_matrix(), p.rust.read_matrix(), "matrix data symbol");
    }
}
