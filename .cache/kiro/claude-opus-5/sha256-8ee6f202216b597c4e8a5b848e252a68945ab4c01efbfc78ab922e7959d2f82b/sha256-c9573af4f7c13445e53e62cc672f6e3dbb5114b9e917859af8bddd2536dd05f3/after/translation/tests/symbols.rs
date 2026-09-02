//! Phase D — exported-symbol parity, checked mechanically with `nm -D`.
//!
//! Backs the claims in `SYMBOLS.md`: every symbol the C `.so` defines must be
//! defined by the Rust `.so` under the exact same name, and every one of them
//! must be resolvable through `dlsym`.

mod harness;

use harness::*;
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

/// All *defined* dynamic symbol names in `so`.
fn defined_dynamic_symbols(so: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(so)
        .output()
        .expect("running `nm -D --defined-only` (binutils must be installed)");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().nth(2).map(str::to_owned))
        .collect()
}

/// All *undefined* dynamic symbol names in `so`.
fn undefined_dynamic_symbols(so: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--undefined-only"])
        .arg(so)
        .output()
        .expect("running `nm -D --undefined-only`");
    assert!(out.status.success(), "nm failed on {}", so.display());
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let kind = it.next()?; // "U" or "w"
            let name = it.next()?;
            if kind == "U" || kind == "w" {
                Some(name.split('@').next().unwrap_or(name).to_owned())
            } else {
                None
            }
        })
        .collect()
}

/// The 12 symbols `lib.c` defines with external linkage.
const EXPECTED: [&str; 12] = [
    "add_three",
    "apply_operation",
    "complex_calc",
    "compute_with_dynamic_memory",
    "get_time_based_value",
    "hatch",
    "increment_counter",
    "manipulate_records",
    "multiply_add",
    "process_pointer_data",
    "shift_array_data",
    "update_accumulator",
];

#[test]
fn symbol_parity_c_vs_rust() {
    let c = c_so_path();
    let r = rust_so_path();

    let c_syms = defined_dynamic_symbols(&c);
    let r_syms = defined_dynamic_symbols(&r);

    let missing: Vec<&String> = c_syms.difference(&r_syms).collect();
    assert!(
        missing.is_empty(),
        "Rust .so ({}) is MISSING {} symbol(s) exported by the C .so ({}): {:?}\n\
         Per Phase A: add the #[no_mangle] wrapper, or translate the missing C source.",
        r.display(),
        missing.len(),
        c.display(),
        missing
    );

    // Sanity: the C .so really does export the full expected set, so the
    // comparison above is not vacuously satisfied by an empty C symbol table.
    for name in EXPECTED {
        assert!(
            c_syms.contains(name),
            "C .so unexpectedly lacks `{name}` — the symbol list is stale"
        );
        assert!(r_syms.contains(name), "Rust .so lacks `{name}`");
    }
    assert_eq!(
        c_syms.len(),
        EXPECTED.len(),
        "C .so exports {:?}, but SYMBOLS.md documents {} symbols",
        c_syms,
        EXPECTED.len()
    );

    // The `static`s must stay internal in both.
    for internal in ["global_counter", "global_accumulator"] {
        assert!(!c_syms.contains(internal), "C leaked `{internal}`");
        assert!(!r_syms.contains(internal), "Rust leaked `{internal}`");
    }
}

#[test]
fn rust_so_has_no_unresolved_non_libc_symbols() {
    let r = rust_so_path();
    let undef = undefined_dynamic_symbols(&r);

    // Everything the Rust cdylib imports must come from libc / libgcc_s /
    // the loader — i.e. it must be resolvable in the already-loaded process.
    // Loading the library is the real test: dlopen fails on any unresolved
    // symbol at load time for the non-lazy (data) relocations, and the tests
    // that follow exercise every function relocation.
    let _ = libs();

    // Additionally assert none of the imports look like an untranslated
    // in-crate symbol (Rust-mangled `_ZN...` / `_R...` imports would mean part
    // of the crate failed to link in).
    let suspicious: Vec<&String> = undef
        .iter()
        .filter(|s| s.starts_with("_ZN") || s.starts_with("_RN"))
        .collect();
    assert!(
        suspicious.is_empty(),
        "Rust .so has unresolved Rust-mangled imports: {suspicious:?}"
    );

    // And the C library's own libc imports must all be present in the Rust
    // library's import set too (it performs the same libc calls).
    let c_undef = undefined_dynamic_symbols(&c_so_path());
    for name in ["malloc", "free", "memmove", "memset", "time", "difftime", "snprintf"] {
        assert!(
            c_undef.contains(name),
            "C .so no longer imports `{name}` — the C changed?"
        );
        assert!(
            undef.contains(name),
            "Rust .so does not import `{name}`; the translation replaced a libc \
             call with something else, which can change observable behaviour"
        );
    }
}

#[test]
fn every_expected_symbol_is_dlsym_resolvable_in_both() {
    // `libs()` panics with the symbol name if any `dlsym` fails, so simply
    // loading both libraries proves all 12 exports are reachable through the
    // dynamic linker in both `.so`s.
    let p = libs();
    assert_eq!(p.c.name, "C");
    assert_eq!(p.r.name, "Rust");
    assert_eq!(EXPECTED.len(), 12);
}
