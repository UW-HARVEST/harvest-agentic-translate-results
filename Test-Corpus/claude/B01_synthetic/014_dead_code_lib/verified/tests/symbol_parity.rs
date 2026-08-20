//! Phase D — dynamic-symbol parity between the C `.so` and the Rust `.so`.
//!
//! Runs `nm -D` on both objects and requires the set of *defined* dynamic
//! symbols to be identical, and every documented symbol to be `dlsym`-able from
//! the Rust object.

mod common;

use libloading::{Library, Symbol};
use std::collections::BTreeSet;
use std::ffi::c_char;
use std::path::PathBuf;
use std::process::Command;

fn c_so() -> PathBuf {
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/c_src/build/libdriver.so"))
}

/// Rebuilds the cdylib if needed and returns its path, so `nm` is never run on
/// a stale artifact (`cargo test` alone does not rebuild the cdylib).
fn rust_so() -> PathBuf {
    common::ensure_rust_so_built()
}

/// Defined dynamic symbols, excluding the weak toolchain/CRT bookkeeping ones
/// (`_ITM_*`, `__gmon_start__`, `__cxa_*`) that neither library authors.
fn defined_dynamic_symbols(so: &PathBuf) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", so.to_str().unwrap()])
        .output()
        .expect("run nm");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(str::to_string))
        .filter(|s| {
            !s.starts_with("_ITM_") && !s.starts_with("__cxa_") && !s.starts_with("__gmon_")
        })
        .collect()
}

fn d1_defined_symbol_sets_are_identical() {
    let c = defined_dynamic_symbols(&c_so());
    let r = defined_dynamic_symbols(&rust_so());

    let missing: Vec<_> = c.difference(&r).cloned().collect();
    let extra: Vec<_> = r.difference(&c).cloned().collect();

    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}"
    );
    assert!(
        extra.is_empty(),
        "symbols exported by the Rust .so that the C .so does not export: {extra:?}"
    );
    assert_eq!(
        c,
        ["bad", "driver", "good", "printLine"]
            .iter()
            .map(|s| s.to_string())
            .collect::<BTreeSet<_>>(),
        "the C .so's exported surface changed — SYMBOLS.md must be regenerated"
    );
}

fn d2_every_symbol_is_dlsym_able_from_both() {
    for so in [c_so(), rust_so()] {
        unsafe {
            let lib = Library::new(&so).unwrap_or_else(|e| panic!("dlopen {}: {e}", so.display()));
            let _: Symbol<unsafe extern "C" fn(*const c_char)> =
                lib.get(b"printLine\0").unwrap_or_else(|e| panic!("{}: printLine: {e}", so.display()));
            for name in [&b"bad\0"[..], b"good\0", b"driver\0"] {
                let _: Symbol<unsafe extern "C" fn()> = lib
                    .get(name)
                    .unwrap_or_else(|e| panic!("{}: {:?}: {e}", so.display(), name));
            }
        }
    }
}

fn d3_static_helpers_are_not_exported_by_either() {
    for so in [c_so(), rust_so()] {
        let syms = defined_dynamic_symbols(&so);
        for hidden in ["helperBad", "helperGood", "helper_bad", "helper_good"] {
            assert!(
                !syms.contains(hidden),
                "{} must not export the C `static` helper {hidden}",
                so.display()
            );
        }
    }
}

fn d4_rust_so_has_no_unresolved_non_libc_symbols() {
    let out = Command::new("nm")
        .args(["-D", "-u", rust_so().to_str().unwrap()])
        .output()
        .expect("run nm");
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    let unresolved: Vec<&str> = text
        .lines()
        .filter_map(|l| l.split_whitespace().last())
        .filter(|s| {
            // Everything the Rust std/cdylib legitimately imports from the
            // platform: libc (versioned as name@GLIBC_x.y), libgcc unwinder,
            // and weak CRT hooks.
            !s.contains("@GLIBC")
                && !s.contains("@GCC")
                && !s.starts_with("_ITM_")
                && !s.starts_with("__cxa_")
                && !s.starts_with("__gmon_")
        })
        .collect();
    assert!(
        unresolved.is_empty(),
        "Rust .so has unresolved non-libc symbols: {unresolved:?}"
    );
}

fn d5_two_distinct_mappings_are_loaded() {
    // `common::pair()` asserts internally that no symbol resolves to the same
    // address in both libraries (the C object's SONAME is `libdriver.so`, same
    // basename as the Rust cdylib, so this guards against loader dedup making
    // the whole differential suite vacuous).
    let p = common::pair();
    let (c, r) = (p.c.addrs(), p.rust.addrs());
    assert_eq!(c.len(), 4);
    for i in 0..4 {
        assert_ne!(c[i], r[i]);
    }
}

fn main() {
    let mut r = common::Runner::new("symbol_parity (Phase D)");
    r.case("d1_defined_symbol_sets_are_identical", d1_defined_symbol_sets_are_identical);
    r.case("d2_every_symbol_is_dlsym_able_from_both", d2_every_symbol_is_dlsym_able_from_both);
    r.case("d3_static_helpers_are_not_exported_by_either", d3_static_helpers_are_not_exported_by_either);
    r.case("d4_rust_so_has_no_unresolved_non_libc_symbols", d4_rust_so_has_no_unresolved_non_libc_symbols);
    r.case("d5_two_distinct_mappings_are_loaded", d5_two_distinct_mappings_are_loaded);
    r.finish();
}
