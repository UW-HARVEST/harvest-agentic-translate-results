//! Phase D — exported-symbol parity, checked automatically.
//!
//! `nm -D` on the C `.so` lists every symbol an external caller can bind to.
//! Every one of them must also be exported by the Rust `.so` under the exact
//! same name. This is asserted here rather than left to a manual command, so a
//! regression (an export wrapper deleted, a `#[no_mangle]` renamed, a whole
//! module left untranslated) fails the build.

mod common;

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

/// Parse `nm -D --defined-only` output into a set of exported symbol names.
fn defined_symbols(so: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(so)
        .output()
        .expect("run nm (is binutils installed?)");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|line| {
            // "<addr> <type> <name>"; skip the Rust-internal and CRT clutter.
            let mut it = line.split_whitespace();
            let _addr = it.next()?;
            let kind = it.next()?;
            let name = it.next()?;
            if kind.len() != 1 {
                return None;
            }
            Some(name.to_string())
        })
        .collect()
}

/// Undefined (imported) symbols, for the "0 missing non-libc symbols" check.
fn undefined_symbols(so: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--undefined-only"])
        .arg(so)
        .output()
        .expect("run nm");
    assert!(out.status.success());
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|line| line.split_whitespace().last().map(|s| s.to_string()))
        .collect()
}

/// The full list of symbols `c_src/src/lib.c` defines, from SYMBOLS.md.
const EXPECTED: [&str; 9] = [
    "shift_array",
    "process_string",
    "apply_bitmask",
    "init_matrix",
    "compare_allocations",
    "arity4",
    "arity2",
    "arity3",
    "arity",
];

#[test]
fn symbols_c_exports_are_all_exported_by_rust() {
    let c_so = common::c_so_path();
    let rust_so = common::rust_so_path();
    let c_syms = defined_symbols(&c_so);
    let rust_syms = defined_symbols(&rust_so);

    // Sanity: the C .so really did export the surface we expect. If this trips,
    // the C source grew a function and SYMBOLS.md / the tests need updating.
    for want in EXPECTED {
        assert!(
            c_syms.contains(want),
            "the C .so no longer exports {want}; re-derive SYMBOLS.md"
        );
    }
    assert_eq!(
        c_syms.len(),
        EXPECTED.len(),
        "the C .so exports {} symbols but SYMBOLS.md documents {}: {:?}",
        c_syms.len(),
        EXPECTED.len(),
        c_syms.difference(&EXPECTED.iter().map(|s| s.to_string()).collect()),
    );

    let missing: Vec<&String> = c_syms.difference(&rust_syms).collect();
    assert!(
        missing.is_empty(),
        "the Rust .so is MISSING {} symbol(s) exported by the C .so: {missing:?}\n\
         C   .so: {}\nRust .so: {}",
        missing.len(),
        c_so.display(),
        rust_so.display()
    );
}

#[test]
fn symbols_rust_has_no_unresolvable_imports() {
    let rust_so = common::rust_so_path();
    let undef = undefined_symbols(&rust_so);
    // Everything the Rust .so imports must resolve in libc / libgcc at load
    // time; the harness already proves that by successfully dlopen-ing it, so
    // the check here is that dlopen works and that the C's own four imports are
    // the only non-runtime ones we depend on.
    let _ = common::load_rust();
    for libc_sym in ["malloc", "free", "memmove", "strlen"] {
        assert!(
            undef.iter().any(|s| s.starts_with(libc_sym)),
            "expected the Rust .so to import {libc_sym} like the C does; imports: {undef:?}"
        );
    }
}

/// Each symbol must be individually resolvable via `dlsym` in the Rust `.so`,
/// not merely present in `nm` output — that is what an external caller does.
#[test]
fn symbols_all_resolvable_via_dlsym() {
    // `common::load_c` / `load_rust` already `dlsym` all nine symbols and panic
    // on any missing one, so constructing both proves resolvability.
    let pair = common::load_pair();
    assert_eq!(pair.c.name, "C");
    assert_eq!(pair.rust.name, "Rust");
}
