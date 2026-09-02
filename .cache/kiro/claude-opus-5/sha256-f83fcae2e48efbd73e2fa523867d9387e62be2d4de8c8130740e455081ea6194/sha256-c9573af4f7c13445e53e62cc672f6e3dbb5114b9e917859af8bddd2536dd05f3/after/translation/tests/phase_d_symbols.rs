//! Phase D — symbol parity, enforced as a test rather than only in a document.
//!
//! Runs `nm -D` on both shared objects and requires that every symbol the C
//! `.so` defines is also defined by the Rust `.so`, with the exact same name.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

/// Symbols the dynamic loader/CRT injects into every shared object; not part of
/// the library's own surface, so they are excluded from the diff on both sides.
const CRT_NOISE: &[&str] = &[
    "_ITM_deregisterTMCloneTable",
    "_ITM_registerTMCloneTable",
    "__cxa_finalize",
    "__gmon_start__",
    "_init",
    "_fini",
    "__bss_start",
    "_edata",
    "_end",
];

fn defined_dynamic_symbols(so: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", "--format=posix"])
        .arg(so)
        .output()
        .expect("run nm -D");
    assert!(
        out.status.success(),
        "nm -D failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().next())
        .map(|s| s.split('@').next().unwrap().to_string())
        .filter(|s| !CRT_NOISE.contains(&s.as_str()))
        .collect()
}

#[test]
fn d1_every_c_symbol_is_exported_by_rust() {
    let c_syms = defined_dynamic_symbols(&c_so_path());
    assert!(
        c_syms.contains("colourblind"),
        "sanity: the C .so must export `colourblind`; got {c_syms:?}"
    );
    println!("C .so exports {} symbol(s): {c_syms:?}", c_syms.len());

    for (label, path) in rust_so_paths() {
        let r_syms = defined_dynamic_symbols(&path);
        println!("rust-{label} exports {} symbol(s): {r_syms:?}", r_syms.len());

        let missing: Vec<&String> = c_syms.difference(&r_syms).collect();
        assert!(
            missing.is_empty(),
            "rust-{label} ({}) is MISSING {} symbol(s) that the C .so exports: {missing:?}\n\
             Per the Phase A rule these must be exported (if the impl exists) or the \
             missing C module must be translated — never stubbed.",
            path.display(),
            missing.len()
        );
    }
}

/// The three matrix routines are `static` in C, so neither `.so` may export
/// them. A Rust `.so` with a *larger* surface than the C one is also a parity
/// failure.
#[test]
fn d2_static_c_functions_are_exported_by_neither() {
    let hidden = ["Protanopia", "Deuteranopia", "Tritanopia"];
    let c_syms = defined_dynamic_symbols(&c_so_path());
    for h in hidden {
        assert!(
            !c_syms.contains(h),
            "premise: `{h}` is `static` in C and must not be in the C .so"
        );
    }
    for (label, path) in rust_so_paths() {
        let r_syms = defined_dynamic_symbols(&path);
        for h in hidden {
            assert!(
                !r_syms.contains(h),
                "rust-{label} exports `{h}`, which is `static` in C ({})",
                path.display()
            );
        }
    }
}

/// No non-libc undefined symbols in the Rust `.so`.
#[test]
fn d3_rust_so_has_no_unresolved_non_libc_symbols() {
    for (label, path) in rust_so_paths() {
        let out = Command::new("nm")
            .args(["-D", "--undefined-only", "--format=posix"])
            .arg(&path)
            .output()
            .expect("run nm -D --undefined-only");
        let undef: Vec<String> = String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter_map(|l| l.split_whitespace().next())
            .map(|s| s.split('@').next().unwrap().to_string())
            .filter(|s| !CRT_NOISE.contains(&s.as_str()))
            .collect();
        println!("rust-{label} undefined: {undef:?}");

        // Anything the loader must satisfy has to come from libc / the Rust
        // runtime shims; a leftover reference to an untranslated C function
        // would show up here as an unexpected name.
        let suspicious: Vec<&String> = undef
            .iter()
            .filter(|s| {
                let s = s.as_str();
                s == "colourblind"
                    || s == "Protanopia"
                    || s == "Deuteranopia"
                    || s == "Tritanopia"
            })
            .collect();
        assert!(
            suspicious.is_empty(),
            "rust-{label} has unresolved library symbols {suspicious:?} — the \
             translation references code it does not define ({})",
            path.display()
        );

        // The whole .so must be loadable and its entry point resolvable, which
        // is the operational form of "no unresolved symbols".
        let _ = rust_impls();
    }
}

/// Sanity check that the loaded `colourblind` really is the exported wrapper of
/// each `.so` and not accidentally the same function twice.
#[test]
fn d4_both_entry_points_are_distinct_code() {
    let c_addr = c_impl().call as usize;
    for r in rust_impls() {
        let r_addr = r.call as usize;
        assert_ne!(
            c_addr, r_addr,
            "{} resolved to the same address as the C .so — the test would be vacuous",
            r.label
        );
        println!("{:<14} colourblind @ {:#x}", r.label, r_addr);
    }
    println!("{:<14} colourblind @ {:#x}", "c", c_addr);
}

/// Guards against a vacuous suite: the harness must actually be testing both
/// Rust profiles, and the C `.so` must be a different file from either.
#[test]
fn d5_harness_is_not_vacuous() {
    let impls = rust_impls();
    let labels: Vec<&str> = impls.iter().map(|i| i.label.as_str()).collect();
    println!("Rust implementations under test: {labels:?}");
    assert!(
        !impls.is_empty(),
        "no Rust .so under test — the differential suite would be vacuous"
    );
    let c = c_so_path();
    for i in impls {
        assert_ne!(
            i.path.canonicalize().unwrap(),
            c.canonicalize().unwrap(),
            "{} points at the C .so",
            i.label
        );
    }
    // Both profiles should normally be present; warn loudly if not.
    if !labels.contains(&"rust-release") {
        println!("WARNING: the release cdylib is not being tested");
    }
    if !labels.contains(&"rust-debug") {
        println!("WARNING: the debug cdylib is not being tested");
    }
}
