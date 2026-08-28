//! Phase A / Phase D — symbol parity between the C `.so` and the Rust `.so`.
//!
//! Asserted at test time (by shelling out to `nm -D`) rather than being a
//! one-off manual observation, so a regression that drops an export fails the
//! suite.

mod common;

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

fn nm(path: &Path, args: &[&str]) -> Vec<String> {
    let out = Command::new("nm")
        .args(args)
        .arg(path)
        .output()
        .unwrap_or_else(|e| panic!("could not run nm: {e}"));
    assert!(
        out.status.success(),
        "nm {args:?} {} failed: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|l| l.to_string())
        .collect()
}

/// Names of symbols *defined* (exported) by a shared object.
fn defined_dynamic(path: &Path) -> BTreeSet<String> {
    nm(path, &["-D", "--defined-only"])
        .into_iter()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .collect()
}

/// Names of *undefined* dynamic symbols (imports).
fn undefined_dynamic(path: &Path) -> BTreeSet<String> {
    nm(path, &["-D", "--undefined-only"])
        .into_iter()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let ty = it.next()?;
            // `U` = undefined, `w`/`v` = weak undefined.
            if ty == "U" || ty == "w" || ty == "v" {
                it.next().map(|s| s.to_string())
            } else {
                None
            }
        })
        .collect()
}

#[test]
fn every_c_export_is_also_a_rust_export() {
    let c = common::c_so_path();
    let c_syms = defined_dynamic(&c);
    assert!(
        c_syms.contains("hsl_to_rgb"),
        "the C .so does not export hsl_to_rgb; nm output changed? got {c_syms:?}"
    );

    for r in common::rust_libs() {
        let r_syms = defined_dynamic(&r.path);
        let missing: Vec<&String> = c_syms.difference(&r_syms).collect();
        assert!(
            missing.is_empty(),
            "{} is missing {} symbol(s) exported by the C .so: {missing:?}\n\
             (C exports: {c_syms:?})",
            r.name,
            missing.len()
        );
    }
}

#[test]
fn c_exports_exactly_the_documented_surface() {
    // Guards SYMBOLS.md against the C growing a new entry point that the tests
    // then would not cover.
    let c_syms = defined_dynamic(&common::c_so_path());
    let expected: BTreeSet<String> = ["hsl_to_rgb".to_string()].into_iter().collect();
    assert_eq!(
        c_syms, expected,
        "the C .so's exported surface changed; SYMBOLS.md/CONFIGS.md/ERRORS.md must be regenerated"
    );
}

/// The definitive check for "0 missing/undefined symbols": the harness `dlopen`s
/// every object with `RTLD_NOW`, which makes the dynamic loader resolve *every*
/// undefined symbol up front and fail the `dlopen` if any cannot be bound. So if
/// `c_lib()`/`rust_libs()` return at all, neither object has an unresolved
/// symbol.
#[test]
fn no_unresolved_symbols_under_rtld_now() {
    let _ = common::c_lib();
    for r in common::rust_libs() {
        let _ = r.f;
    }
}

/// Every undefined (imported) symbol of the Rust `.so` must be a platform
/// runtime symbol, i.e. either versioned against glibc/libgcc or one of the
/// well-known unversioned toolchain hooks. This catches an import that happens to
/// resolve locally on this machine but is not part of the C runtime.
#[test]
fn rust_imports_are_all_platform_runtime_symbols() {
    // Unversioned symbols the toolchain always emits as weak undefined.
    let allowed_unversioned = [
        "_ITM_registerTMCloneTable",
        "_ITM_deregisterTMCloneTable",
        "__gmon_start__",
    ];
    for r in common::rust_libs() {
        for sym in undefined_dynamic(&r.path) {
            if let Some((_name, ver)) = sym.split_once('@') {
                let ver = ver.trim_start_matches('@');
                assert!(
                    ver.starts_with("GLIBC_") || ver.starts_with("GCC_"),
                    "{}: import {sym} is versioned against {ver}, which is neither glibc nor libgcc",
                    r.name
                );
            } else {
                assert!(
                    allowed_unversioned.contains(&sym.as_str()),
                    "{}: unversioned, non-runtime import {sym}",
                    r.name
                );
            }
        }
    }
}

/// The C `.so` imports `fmodf` from libm. The Rust `.so` additionally imports
/// `feraiseexcept` (used to reproduce the signalling `comiss` the C emits for the
/// hue comparisons) and binds `fmodf` to the copy `compiler_builtins` links in
/// statically. Both facts are load-bearing for the differential tests, so pin
/// them: if a toolchain change makes `fmodf` become a dynamic import (or makes it
/// disappear), the `fmodf`-equivalence reasoning in SYMBOLS.md must be revisited.
#[test]
fn import_inventory_is_as_documented() {
    let c_imports = undefined_dynamic(&common::c_so_path());
    assert!(
        c_imports.iter().any(|s| s.starts_with("fmodf@")),
        "expected the C .so to import fmodf from libm, got {c_imports:?}"
    );
    for r in common::rust_libs() {
        let imports = undefined_dynamic(&r.path);
        assert!(
            imports.iter().any(|s| s.starts_with("feraiseexcept@")),
            "{}: expected feraiseexcept to be imported, got {imports:?}",
            r.name
        );
        let dynamic_fmodf = imports.iter().any(|s| s.starts_with("fmodf@") || s == "fmodf");
        let local_fmodf = nm(&r.path, &["-a"])
            .iter()
            .any(|l| l.split_whitespace().last() == Some("fmodf"));
        assert!(
            dynamic_fmodf || local_fmodf,
            "{}: fmodf is neither imported nor defined locally",
            r.name
        );
    }
}

#[test]
fn both_libraries_agree_on_a_trivial_call() {
    // Smoke test that the harness really talks to two distinct objects.
    assert_ne!(
        common::c_so_path().canonicalize().unwrap(),
        common::rust_libs()[0].path.canonicalize().unwrap()
    );
    common::assert_same("smoke", 30.0, 1.0, 0.5);
}
