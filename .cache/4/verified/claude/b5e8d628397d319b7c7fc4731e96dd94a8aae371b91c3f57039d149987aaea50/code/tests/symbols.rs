//! Phase D — symbol parity between the C `.so` and the Rust `.so`.
//!
//! Mechanically compares `nm -D --defined-only` on both shared objects: every
//! symbol the C exports must be exported by the Rust with the exact same name.

mod common;

use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;

/// name -> (type letter, size or None)
fn dynamic_symbols(so: &Path) -> BTreeMap<String, (char, Option<u64>)> {
    let out = Command::new("nm")
        .args(["-D", "-S", "--defined-only"])
        .arg(so)
        .output()
        .expect("nm must be available");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );

    // Linker/toolchain-generated symbols that are not part of the API surface.
    const IGNORED: &[&str] = &[
        "_init",
        "_fini",
        "__bss_start",
        "_edata",
        "_end",
        "__bss_start__",
        "__bss_end__",
        "_bss_end__",
        "__end__",
        "__odr_asan_gen_array",
        "_ITM_deregisterTMCloneTable",
        "_ITM_registerTMCloneTable",
        "__gmon_start__",
        "__cxa_finalize",
        "rust_eh_personality",
    ];

    let text = String::from_utf8_lossy(&out.stdout);
    let mut map = BTreeMap::new();
    for line in text.lines() {
        let fields: Vec<&str> = line.split_whitespace().collect();
        // "<addr> [<size>] <type> <name>"
        let (ty, name, size) = match fields.len() {
            4 => (
                fields[2],
                fields[3],
                u64::from_str_radix(fields[1], 16).ok(),
            ),
            3 => (fields[1], fields[2], None),
            _ => continue,
        };
        let name = name.split('@').next().unwrap_or(name).to_string();
        if IGNORED.contains(&name.as_str()) || name.starts_with("_ZN") {
            continue;
        }
        let ty = ty.chars().next().unwrap_or('?');
        // Weak toolchain hooks carry no API meaning.
        if ty == 'w' || ty == 'V' || ty == 'v' {
            continue;
        }
        map.insert(name, (ty, size));
    }
    map
}

#[test]
fn every_c_symbol_is_exported_by_rust() {
    let rust_so = common::rust_so_path();
    let rust = dynamic_symbols(&rust_so);
    assert!(
        !rust.is_empty(),
        "no dynamic symbols found in {}",
        rust_so.display()
    );

    for c_so in [env!("C_DRIVER_SO_O0"), env!("C_DRIVER_SO_O2")] {
        let c = dynamic_symbols(Path::new(c_so));
        assert!(!c.is_empty(), "no dynamic symbols found in {c_so}");

        let missing: Vec<&String> = c.keys().filter(|k| !rust.contains_key(*k)).collect();
        assert!(
            missing.is_empty(),
            "symbols exported by {c_so} but missing from {}: {:?}\n\
             (C symbols: {:?})\n(Rust symbols: {:?})",
            rust_so.display(),
            missing,
            c.keys().collect::<Vec<_>>(),
            rust.keys().collect::<Vec<_>>()
        );

        // The three symbols the C translation unit actually publishes.
        for expected in ["array", "main", "perform_expensive_operations"] {
            assert!(
                c.contains_key(expected),
                "{c_so} unexpectedly lacks {expected}"
            );
            assert!(
                rust.contains_key(expected),
                "{} lacks {expected}",
                rust_so.display()
            );
        }

        // Same symbol kind (bss object vs text function).
        for name in c.keys() {
            let (c_ty, _) = c[name];
            let (r_ty, _) = rust[name];
            let kind = |t: char| match t.to_ascii_uppercase() {
                'B' | 'D' | 'R' | 'G' => 'D', // data
                _ => 'T',                     // code
            };
            assert_eq!(
                kind(c_ty),
                kind(r_ty),
                "{name}: C type {c_ty} vs Rust type {r_ty}"
            );
        }
    }
}

#[test]
fn rust_so_has_no_unresolved_non_libc_symbols() {
    let rust_so = common::rust_so_path();
    let out = Command::new("nm")
        .args(["-D", "-u"])
        .arg(&rust_so)
        .output()
        .expect("nm must be available");
    assert!(out.status.success(), "nm -D -u failed");
    let text = String::from_utf8_lossy(&out.stdout);

    let mut suspicious = Vec::new();
    for line in text.lines() {
        let name = line.split_whitespace().last().unwrap_or("");
        if name.is_empty() {
            continue;
        }
        // Everything the Rust cdylib imports must come from the platform's libc
        // / dynamic loader (they carry a @GLIBC / @GCC version tag) or be a weak
        // toolchain hook.
        let versioned = name.contains('@');
        let weak = line.contains(" w ");
        let known = matches!(
            name,
            "_ITM_deregisterTMCloneTable" | "_ITM_registerTMCloneTable" | "__gmon_start__"
        );
        if !versioned && !weak && !known {
            suspicious.push(name.to_string());
        }
    }
    assert!(
        suspicious.is_empty(),
        "{} has unresolved non-libc symbols: {:?}",
        rust_so.display(),
        suspicious
    );
}

#[test]
fn array_symbol_size_matches() {
    // CONFIGS.md row 1: the exported object must have the same footprint.
    let rust = dynamic_symbols(&common::rust_so_path());
    for c_so in [env!("C_DRIVER_SO_O0"), env!("C_DRIVER_SO_O2")] {
        let c = dynamic_symbols(Path::new(c_so));
        let c_size = c["array"].1.expect("nm -S reports the C array size");
        let r_size = rust["array"].1.expect("nm -S reports the Rust array size");
        assert_eq!(
            c_size, r_size,
            "array size differs: {c_so} has {c_size} bytes, rust has {r_size}"
        );
        assert_eq!(
            c_size as usize,
            common::ARRAY_SIZE * std::mem::size_of::<i32>(),
            "unexpected C array size"
        );
    }
}

#[test]
fn compile_time_constants_match() {
    // CONFIGS.md row 1: ARRAY_SIZE / ITERATIONS as compiled into the Rust .so
    // must equal the C #defines (ARRAY_SIZE is cross-checked against nm -S).
    let rust = common::rust_impl();
    let syms = dynamic_symbols(Path::new(env!("C_DRIVER_SO_O2")));
    let c_array_bytes = syms["array"].1.unwrap() as usize;

    assert_eq!(
        rust.harness_array_size(),
        c_array_bytes / std::mem::size_of::<i32>(),
        "ARRAY_SIZE mismatch"
    );
    assert_eq!(rust.harness_array_size(), 256 * 1024, "ARRAY_SIZE mismatch");
    assert_eq!(rust.harness_iterations(), 2000, "ITERATIONS mismatch");
}

#[test]
fn c_exports_no_symbol_the_suite_forgot_to_exercise() {
    // Guards against the C surface growing without the tests noticing.
    let c = dynamic_symbols(Path::new(env!("C_DRIVER_SO_O2")));
    let mut names: Vec<&str> = c.keys().map(|s| s.as_str()).collect();
    names.sort_unstable();
    assert_eq!(
        names,
        vec!["array", "main", "perform_expensive_operations"],
        "the C .so surface changed; update SYMBOLS.md / the test-suite"
    );
}
