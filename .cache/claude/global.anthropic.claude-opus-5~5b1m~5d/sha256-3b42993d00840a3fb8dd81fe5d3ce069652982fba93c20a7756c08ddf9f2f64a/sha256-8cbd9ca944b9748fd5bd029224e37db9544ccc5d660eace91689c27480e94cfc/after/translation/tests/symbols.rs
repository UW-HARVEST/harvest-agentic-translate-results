// Phase D — symbol parity.
//
// Runs `nm -D` on both shared objects and asserts the set of dynamic symbols
// the C .so DEFINES is a subset of what the Rust .so defines, with the exact
// same names (macro-generated names included). Also asserts every symbol is
// actually resolvable through `dlsym` in both.

mod common;
use common::*;

use std::collections::BTreeSet;
use std::ffi::{c_char, c_void};
use std::path::Path;
use std::process::Command;

extern "C" {
    /// `dlsym(RTLD_DEFAULT, name)` — RTLD_DEFAULT is the null handle on glibc.
    fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
}

/// Symbols that `nm -D` reports for a shared object and that belong to the
/// library's own API surface. `nm` type letters:
///   T/t text, D/d data, B/b bss, R/r rodata, W/w/V/v weak, A/a absolute,
///   U undefined, i indirect.
/// Weak/absolute entries are toolchain artifacts, not API.
fn defined_symbols(so: &Path) -> BTreeSet<String> {
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
    let text = String::from_utf8_lossy(&out.stdout);
    let mut set = BTreeSet::new();
    for line in text.lines() {
        let mut it = line.split_whitespace();
        let a = it.next();
        let b = it.next();
        let c = it.next();
        let (ty, name) = match (a, b, c) {
            (Some(_addr), Some(ty), Some(name)) => (ty, name),
            (Some(ty), Some(name), None) => (ty, name),
            _ => continue,
        };
        if ty.len() != 1 {
            continue;
        }
        let t = ty.chars().next().unwrap();
        // Skip weak / absolute / indirect linker bookkeeping.
        if matches!(t, 'a' | 'A' | 'w' | 'W' | 'v' | 'V' | 'i') {
            continue;
        }
        // Skip toolchain-internal names present in every shared object.
        if name.starts_with("_init")
            || name.starts_with("_fini")
            || name.starts_with("__bss_start")
            || name.starts_with("_edata")
            || name.starts_with("_end")
            || name.starts_with("_IO_stdin_used")
            || name.starts_with("__gnu_lto")
        {
            continue;
        }
        set.insert(name.to_string());
    }
    set
}

/// Symbols the Rust `cdylib` necessarily exports on top of the C API: the Rust
/// runtime / compiler-builtins glue. These are extra, never missing, so they
/// cannot hide an untranslated C module.
fn is_rust_runtime_symbol(name: &str) -> bool {
    name.starts_with("rust_")
        || name.starts_with("__rust")
        || name.starts_with("_ZN")
        || name.starts_with("_R")
        || name.starts_with("rust_eh_")
        || name.starts_with("_Unwind")
        || name.contains("$LT$")
}

#[test]
fn c_and_rust_export_identical_symbols() {
    let _g = lock();
    let l = libs();
    let c_syms = defined_symbols(&l.c_path);
    let rs_syms = defined_symbols(&l.rs_path);

    println!("C  .so {} defines {} symbols", l.c_path.display(), c_syms.len());
    for s in &c_syms {
        println!("  C:    {s}");
    }
    let extra: Vec<&String> = rs_syms
        .iter()
        .filter(|s| !c_syms.contains(*s) && !is_rust_runtime_symbol(s))
        .collect();
    println!("Rust .so extra non-runtime symbols: {extra:?}");

    // THE GATE: nothing the C .so exports may be missing from the Rust .so.
    let missing: Vec<&String> = c_syms.difference(&rs_syms).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is MISSING {} symbol(s) exported by the C .so: {missing:?}\n\
         Each of these means either a missing #[no_mangle] wrapper or an \
         entirely untranslated C module.",
        missing.len()
    );

    // Sanity: the C library really did export the five API functions, so an
    // empty-vs-empty comparison cannot pass vacuously.
    for expected in [
        "envy",
        "parse_env_numeric",
        "init_config_from_env",
        "perform_operation",
        "apply_bit_operations",
    ] {
        assert!(
            c_syms.contains(expected),
            "sanity check: C .so must export {expected}"
        );
        assert!(
            rs_syms.contains(expected),
            "Rust .so must export {expected}"
        );
    }
    assert_eq!(c_syms.len(), 5, "the C API surface is exactly five functions");
}

#[test]
fn every_c_symbol_is_dlsym_resolvable_in_rust() {
    let _g = lock();
    let l = libs();
    for name in defined_symbols(&l.c_path) {
        let mut sym = name.clone().into_bytes();
        sym.push(0);
        unsafe {
            l.c.lib
                .get::<*const ()>(&sym)
                .unwrap_or_else(|e| panic!("dlsym {name} in C .so: {e}"));
            l.rs
                .lib
                .get::<*const ()>(&sym)
                .unwrap_or_else(|e| panic!("dlsym {name} in Rust .so: {e}"));
        }
    }
}

#[test]
fn rust_so_has_no_unresolved_non_libc_symbols() {
    let _g = lock();
    let l = libs();
    let out = Command::new("nm")
        .arg("-D")
        .arg("--undefined-only")
        .arg(&l.rs_path)
        .output()
        .expect("nm");
    let text = String::from_utf8_lossy(&out.stdout);
    let undefined: Vec<&str> = text
        .lines()
        .filter_map(|line| line.split_whitespace().last())
        .filter(|n| !n.is_empty())
        .collect();
    println!("Rust .so undefined symbols: {undefined:?}");

    // Every import must resolve at run time out of libc / libgcc / ld.so.
    // Anything that does not resolve would be a dangling reference to code that
    // was never translated. `__gmon_start__` and `_ITM_*` are the standard weak
    // profiling / transactional-memory hooks that are intentionally absent.
    const WEAK_HOOKS: [&str; 3] = [
        "__gmon_start__",
        "_ITM_registerTMCloneTable",
        "_ITM_deregisterTMCloneTable",
    ];
    let mut unresolved: Vec<String> = Vec::new();
    for sym in &undefined {
        let bare = sym.split('@').next().unwrap();
        if WEAK_HOOKS.contains(&bare) {
            continue;
        }
        let mut cname = bare.as_bytes().to_vec();
        cname.push(0);
        // RTLD_DEFAULT: search every object already loaded into this process,
        // exactly as the dynamic loader did when it mapped the .so.
        let addr = unsafe { dlsym(std::ptr::null_mut(), cname.as_ptr() as *const _) };
        if addr.is_null() {
            unresolved.push(sym.to_string());
        }
    }
    assert!(
        unresolved.is_empty(),
        "Rust .so imports symbols that resolve nowhere (untranslated code?): {unresolved:?}"
    );

    // None of the library's OWN API functions may appear as an import: that
    // would mean the Rust side only forwards to the C .so instead of
    // implementing the logic.
    for api in [
        "envy",
        "parse_env_numeric",
        "init_config_from_env",
        "perform_operation",
        "apply_bit_operations",
    ] {
        assert!(
            !undefined.iter().any(|s| s.split('@').next() == Some(api)),
            "Rust .so imports {api} instead of defining it"
        );
    }

    // dlopen already succeeded (libs() would have panicked otherwise), which is
    // the real proof that every import resolved at load time.
    let _ = l.rs.envy();
}
