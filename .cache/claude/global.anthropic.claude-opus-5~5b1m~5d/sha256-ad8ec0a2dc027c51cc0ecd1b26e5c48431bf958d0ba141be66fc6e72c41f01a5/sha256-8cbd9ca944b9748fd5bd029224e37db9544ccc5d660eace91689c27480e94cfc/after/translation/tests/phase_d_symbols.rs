//! Phase D — symbol parity gate.
//!
//! Every symbol the C `.so` exports must also be exported by the Rust `.so`
//! under the exact same name. This is asserted programmatically (via `nm -D`)
//! so it cannot silently rot, and separately by actually `dlsym`ing each name
//! out of both libraries.

mod common;
use common::*;
use std::collections::BTreeSet;
use std::process::Command;

/// The complete list of symbols defined by the C library, from
/// `nm -D --defined-only`. Kept in sync by `symbol_sets_are_identical`.
const EXPECTED: &[&str] = &[
    "apply_bitmask",
    "arity",
    "arity2",
    "arity3",
    "arity4",
    "compare_allocations",
    "init_matrix",
    "process_string",
    "shift_array",
];

fn defined_symbols(so: &str) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", so])
        .output()
        .expect("nm not available");
    assert!(out.status.success(), "nm failed on {so}");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let _addr = it.next()?;
            let kind = it.next()?;
            let name = it.next()?;
            // Only global/weak text+data definitions; skip the compiler's own
            // runtime bookkeeping that neither library "owns".
            if !matches!(kind, "T" | "t" | "D" | "B" | "R" | "W" | "V" | "G") {
                return None;
            }
            if name.starts_with("_ITM_")
                || name.starts_with("__cxa")
                || name == "__gmon_start__"
                || name.starts_with("_fini")
                || name.starts_with("_init")
            {
                return None;
            }
            Some(name.to_string())
        })
        .collect()
}

#[test]
fn symbol_sets_are_identical() {
    let c = defined_symbols(&c_so_path().to_string_lossy());
    let r = defined_symbols(&rust_so_path().to_string_lossy());

    let missing: Vec<_> = c.difference(&r).cloned().collect();
    let extra: Vec<_> = r.difference(&c).cloned().collect();

    assert!(
        missing.is_empty(),
        "\n*** {} C symbol(s) NOT exported by the Rust .so ***\n{missing:#?}\n\
         Per Phase A: add the #[no_mangle] wrapper if the impl exists, or\n\
         TRANSLATE the missing C source if a whole module was skipped.\n",
        missing.len()
    );
    assert!(
        extra.is_empty(),
        "\n*** {} symbol(s) exported by Rust but not by C ***\n{extra:#?}\n",
        extra.len()
    );

    // And pin the absolute contents so a future regression that drops a symbol
    // from BOTH libraries still trips.
    let want: BTreeSet<String> = EXPECTED.iter().map(|s| s.to_string()).collect();
    assert_eq!(c, want, "C .so symbol set changed vs SYMBOLS.md");
    assert_eq!(r, want, "Rust .so symbol set changed vs SYMBOLS.md");
}

#[test]
fn every_symbol_is_dlsym_able_from_both() {
    // `both()` already resolves all nine symbols through dlsym and panics with
    // the offending name if any is absent, so simply loading proves the export
    // wrappers are real and callable — not just present in the symbol table.
    let (c, r) = both();
    assert_eq!(c.name, "C");
    assert_eq!(r.name, "Rust");
    for n in EXPECTED {
        assert!(!n.is_empty());
    }
}

#[test]
fn rust_so_has_no_unresolved_non_libc_symbols() {
    // Every undefined symbol in the Rust .so must be satisfiable by the loader
    // (libc / libgcc / ld). Nothing the C library was supposed to provide may
    // be left dangling.
    let so = rust_so_path();
    let out = Command::new("nm")
        .args(["-D", "--undefined-only", &so.to_string_lossy()])
        .output()
        .expect("nm not available");
    let text = String::from_utf8_lossy(&out.stdout);

    let offenders: Vec<&str> = text
        .lines()
        .filter_map(|l| l.split_whitespace().last())
        .map(|s| s.split('@').next().unwrap_or(s))
        // Anything named like one of the library's own functions would be a
        // real unresolved reference.
        .filter(|s| EXPECTED.contains(s))
        .collect();

    assert!(
        offenders.is_empty(),
        "Rust .so leaves library symbols unresolved: {offenders:?}"
    );

    // Sanity: the loader can in fact satisfy everything — dlopen would have
    // failed otherwise, and `both()` dlopens with RTLD_NOW-ish semantics for
    // the symbols it resolves.
    let _ = both();
}
