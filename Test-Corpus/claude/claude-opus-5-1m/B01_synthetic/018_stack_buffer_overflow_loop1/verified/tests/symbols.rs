//! Phase D — dynamic symbol parity between the C and the Rust shared object.
//!
//! Every symbol exported by the C `.so` must also be exported, under the exact
//! same name, by the Rust `.so`, and every one of them must be reachable with
//! `dlsym`.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

/// `nm -D --defined-only` -> the set of exported symbol names.
fn exported_symbols(so: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(so)
        .output()
        .expect("spawn nm");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let (_addr, kind, name) = (it.next()?, it.next()?, it.next()?);
            // Exported code/data only (T/D/B/R/W and their weak forms).
            if matches!(kind, "T" | "t" | "D" | "d" | "B" | "b" | "R" | "r" | "W" | "V") {
                Some(name.to_string())
            } else {
                None
            }
        })
        .collect()
}

/// Undefined symbols that are neither libc/libgcc imports nor the standard weak
/// toolchain hooks that the C object also leaves undefined.
fn foreign_undefined(so: &Path) -> BTreeSet<String> {
    const WEAK_TOOLCHAIN_HOOKS: [&str; 4] = [
        "_ITM_deregisterTMCloneTable",
        "_ITM_registerTMCloneTable",
        "__gmon_start__",
        "__cxa_finalize",
    ];
    let out = Command::new("nm")
        .arg("-D")
        .arg("--undefined-only")
        .arg(so)
        .output()
        .expect("spawn nm");
    assert!(out.status.success());
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .filter(|n| !n.contains("@GLIBC") && !n.contains("@GCC") && !n.contains("@CXXABI"))
        .filter(|n| !WEAK_TOOLCHAIN_HOOKS.contains(&n.as_str()))
        .collect()
}

#[test]
fn c_and_rust_shared_objects_export_identical_symbol_sets() {
    let c = c_so_path();
    let r = rust_so_path();

    let c_syms = exported_symbols(&c);
    let r_syms = exported_symbols(&r);

    // Sanity: the five functions of c_src/src/main.c really are there.
    let expected: BTreeSet<String> = ["bad", "good", "main", "printIntLine", "printLine"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    assert_eq!(
        c_syms, expected,
        "unexpected C export surface (SYMBOLS.md is stale)"
    );

    let missing: Vec<&String> = c_syms.difference(&r_syms).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing {} symbol(s) exported by the C .so: {:?}\n\
         C   : {:?}\nRust: {:?}",
        missing.len(),
        missing,
        c_syms,
        r_syms
    );
}

#[test]
fn rust_shared_object_has_no_foreign_undefined_symbols() {
    let missing = foreign_undefined(&rust_so_path());
    assert!(
        missing.is_empty(),
        "Rust .so has unresolved non-libc symbols: {missing:?}"
    );
}

#[test]
fn every_c_symbol_is_resolvable_with_dlsym_in_both_objects() {
    let pair = Pair::load();
    for which in Which::BOTH {
        // Touching each accessor performs the dlsym and panics if absent.
        let _ = pair.print_line(which);
        let _ = pair.print_int_line(which);
        let _ = pair.bad(which);
        let _ = pair.good(which);
        let _ = pair.main_fn(which);
    }
}

/// The C `.so` compiled at every optimisation level must still export the same
/// five names (row C1 of CONFIGS.md, symbol half).
#[test]
fn symbol_parity_holds_for_every_c_optimisation_level() {
    let r_syms = exported_symbols(&rust_so_path());
    for opt in ["", "-O0", "-O1", "-O2", "-Os"] {
        let c = build_c_so(opt);
        let c_syms = exported_symbols(&c);
        let missing: Vec<&String> = c_syms.difference(&r_syms).collect();
        assert!(
            missing.is_empty(),
            "gcc {opt}: Rust .so missing {missing:?}"
        );
    }
}
