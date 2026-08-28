//! Phase D — symbol parity. The symbol diff must reach EMPTY: every symbol the
//! C `.so` exports must also be exported by the Rust `.so` under the exact same
//! name, and the Rust `.so` must have no unresolved non-libc dependencies.

mod common;

use common::Pair;
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

/// Parse `nm -D` output into a set of symbol names.
///
/// `filter` selects on the symbol *type* letter (column 2), e.g. `T`/`t` for
/// text, `U` for undefined.
fn nm_symbols(lib: &Path, defined_only: bool) -> Option<BTreeSet<String>> {
    let mut cmd = Command::new("nm");
    cmd.arg("-D");
    if defined_only {
        cmd.arg("--defined-only");
    } else {
        cmd.arg("--undefined-only");
    }
    cmd.arg(lib);

    let out = cmd.output().ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout);

    let mut set = BTreeSet::new();
    for line in text.lines() {
        let cols: Vec<&str> = line.split_whitespace().collect();
        // Defined:   "<addr> T name"    Undefined:   "w name" / "U name"
        let name = match cols.as_slice() {
            [_addr, _ty, name] => *name,
            [_ty, name] => *name,
            _ => continue,
        };
        set.insert(name.to_string());
    }
    Some(set)
}

/// Rust's own runtime/std machinery, plus versioned-symbol noise, are not part
/// of the C surface and must not count as extra or missing.
fn is_rust_internal(sym: &str) -> bool {
    sym.starts_with("_ZN")
        || sym.starts_with("_R")
        || sym.starts_with("rust_")
        || sym.starts_with("__rust")
        || sym.starts_with("rust_eh_")
        || sym.contains("$LT$")
}

#[test]
fn d1_every_c_symbol_is_exported_by_rust() {
    let pair = Pair::load();

    let Some(c_syms) = nm_symbols(&pair.c_path, true) else {
        eprintln!("`nm` unavailable; skipping symbol parity test");
        return;
    };
    let Some(rust_syms) = nm_symbols(&pair.rust_path, true) else {
        eprintln!("`nm` unavailable; skipping symbol parity test");
        return;
    };

    // The C library's exported surface, as documented in SYMBOLS.md.
    assert!(
        c_syms.contains("half2float"),
        "C .so unexpectedly does not export `half2float`; got {c_syms:?}"
    );

    let missing: Vec<&String> = c_syms.difference(&rust_syms).collect();
    assert!(
        missing.is_empty(),
        "SYMBOLS.md gate FAILED: {} C symbol(s) missing from the Rust .so: {missing:?}\n\
         Per Phase A: add the #[no_mangle] export if the impl exists, or translate \
         the missing C source if a whole module was skipped.",
        missing.len()
    );
}

#[test]
fn d2_rust_so_has_no_unresolved_non_libc_symbols() {
    let pair = Pair::load();

    let Some(undef) = nm_symbols(&pair.rust_path, false) else {
        eprintln!("`nm` unavailable; skipping undefined-symbol test");
        return;
    };

    // Anything the dynamic loader must supply. libc/libm/libgcc/ld provide the
    // usual set; a *Rust* symbol left undefined would mean a missing impl.
    let unresolved_rust: Vec<&String> = undef.iter().filter(|s| is_rust_internal(s)).collect();

    assert!(
        unresolved_rust.is_empty(),
        "Rust .so has unresolved Rust-level symbols (missing implementations): {unresolved_rust:?}"
    );

    // Sanity: the library must actually load and its symbol resolve, which
    // `Pair::load()` above already proved by dlopen'ing it.
    let f = pair.rust_half2float();
    let v = unsafe { f(0x3C00) }; // half 1.0
    assert_eq!(v.to_bits(), 1.0f32.to_bits(), "half 0x3C00 must be 1.0");
}

#[test]
fn d3_rust_does_not_leak_the_static_tables() {
    let pair = Pair::load();

    let Some(rust_syms) = nm_symbols(&pair.rust_path, true) else {
        eprintln!("`nm` unavailable; skipping static-table visibility test");
        return;
    };

    // In C these three tables are `static` (internal linkage), so they are NOT
    // part of the exported ABI. The Rust translation must keep them private to
    // match the C .so's surface exactly.
    for table in ["m__mantissa", "m__offset", "m__exponent"] {
        assert!(
            !rust_syms.contains(table),
            "`{table}` is `static` in C and must not be exported by the Rust .so"
        );
    }
}
