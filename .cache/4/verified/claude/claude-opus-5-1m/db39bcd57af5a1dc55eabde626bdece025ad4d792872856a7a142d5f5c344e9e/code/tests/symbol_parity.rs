//! Phase D — symbol parity, enforced mechanically on every test run.
//!
//! Runs `nm -D` on both shared objects and asserts the symbol diff
//! (C-exported minus Rust-exported) is EMPTY, and that the Rust `.so` has no
//! missing/undefined non-libc symbols.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

/// Parse `nm -D --defined-only <so>` into the set of exported symbol names.
fn defined_symbols(so: &Path) -> BTreeSet<String> {
    nm(so, &["-D", "--defined-only"])
}

/// Parse `nm -D --undefined-only <so>` into the set of imported symbol names.
fn undefined_symbols(so: &Path) -> BTreeSet<String> {
    nm(so, &["-D", "--undefined-only"])
}

fn nm(so: &Path, args: &[&str]) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(args)
        .arg(so)
        .output()
        .unwrap_or_else(|e| panic!("failed to run nm on {}: {e}", so.display()));
    assert!(
        out.status.success(),
        "nm {:?} {} failed: {}",
        args,
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|line| {
            // Formats: "<addr> <type> <name>" or "                 <type> <name>"
            let mut parts = line.split_whitespace().collect::<Vec<_>>();
            let name = parts.pop()?;
            // Require a type letter to be present so blank lines are skipped.
            let ty = parts.pop()?;
            if ty.len() != 1 {
                return None;
            }
            Some(name.to_string())
        })
        .collect()
}

/// Symbols that are toolchain/CRT/libc glue rather than library API.
///
/// Deliberately an EXPLICIT allowlist of known glue names plus a few narrow
/// patterns. An earlier version filtered every name starting with `__`, which
/// would have silently hidden a genuine C API symbol living in the
/// implementation-reserved namespace (e.g. `__tflac_internal`) — the exact class
/// of "missing symbol" this test exists to catch.
fn is_runtime_glue(name: &str) -> bool {
    const GLUE: &[&str] = &[
        "_ITM_deregisterTMCloneTable",
        "_ITM_registerTMCloneTable",
        "__gmon_start__",
        "__cxa_finalize",
        "__cxa_atexit",
        "_init",
        "_fini",
        "__bss_start",
        "_edata",
        "_end",
        "__bss_start__",
        "__bss_end__",
        "__end__",
        "_IO_stdin_used",
        "__odr_asan_gen_",
        "__rust_no_alloc_shim_is_unstable_v2",
    ];
    if GLUE.contains(&name) {
        return true;
    }
    // `nm` renders versioned libc imports as `memcpy@GLIBC_2.2.5`; strip the
    // version and re-check against the glue list, then treat any remaining
    // versioned symbol as a libc import (only imports carry versions here).
    if let Some((base, _)) = name.split_once('@') {
        return GLUE.contains(&base) || !base.is_empty();
    }
    // Rust-mangled internals and std runtime hooks, which the C .so never has.
    name.starts_with("_ZN")
        || name.starts_with("_R")
        || name.starts_with("rust_")
        || name.starts_with("_rust_")
        || name.starts_with("__rust_")
        || name.starts_with("__rdl_")
        || name.starts_with("__rg_")
}

#[test]
fn every_c_exported_symbol_is_exported_by_rust() {
    let l = libs();

    let c_syms = defined_symbols(&l.c_path);
    let rust_syms = defined_symbols(&l.rust_path);

    // Only compare real API symbols, not CRT glue emitted by either toolchain.
    let c_api: BTreeSet<&String> = c_syms.iter().filter(|s| !is_runtime_glue(s)).collect();
    let rust_api: BTreeSet<&String> = rust_syms.iter().filter(|s| !is_runtime_glue(s)).collect();

    println!("C   .so ({}) exports: {:?}", l.c_path.display(), c_api);
    println!("Rust.so ({}) exports: {:?}", l.rust_path.display(), rust_api);

    let missing: Vec<&&String> = c_api.difference(&rust_api).collect();
    assert!(
        missing.is_empty(),
        "the Rust .so is MISSING {} symbol(s) exported by the C .so: {:?}\n\
         Each must be either exported with #[unsafe(no_mangle)] extern \"C\" or, if the \
         whole C module was never translated, translated now.",
        missing.len(),
        missing
    );

    // The C library's public surface is exactly one function; assert we really
    // compared something rather than two empty sets.
    assert!(
        c_api.contains(&"max_size_frame".to_string()),
        "expected `max_size_frame` in the C exports, got {c_api:?}"
    );
    assert_eq!(
        c_api.len(),
        1,
        "the C .so's API surface changed; update SYMBOLS.md: {c_api:?}"
    );
}

#[test]
fn rust_so_has_no_missing_non_libc_symbols() {
    let l = libs();

    let undef = undefined_symbols(&l.rust_path);
    let unresolved: Vec<&String> = undef.iter().filter(|s| !is_runtime_glue(s)).collect();

    println!("Rust.so undefined (raw): {undef:?}");
    assert!(
        unresolved.is_empty(),
        "Rust .so has unresolved non-libc symbols: {unresolved:?}"
    );
}

#[test]
fn c_so_has_no_missing_non_libc_symbols() {
    let l = libs();
    let undef = undefined_symbols(&l.c_path);
    let unresolved: Vec<&String> = undef.iter().filter(|s| !is_runtime_glue(s)).collect();
    println!("C.so undefined (raw): {undef:?}");
    assert!(
        unresolved.is_empty(),
        "C .so has unresolved non-libc symbols: {unresolved:?}"
    );
}

/// The exported Rust symbol must be reachable by the *exact* C name via dlsym —
/// already exercised by every other test, asserted explicitly here.
#[test]
fn rust_symbol_resolves_under_exact_c_name() {
    let l = libs();
    // If the name were mangled or renamed, `libs()` would have panicked during
    // dlsym; calling it proves the wrapper has the right ABI as well.
    assert_eq!(l.c(4096, 2, 16), l.rust(4096, 2, 16));
}
