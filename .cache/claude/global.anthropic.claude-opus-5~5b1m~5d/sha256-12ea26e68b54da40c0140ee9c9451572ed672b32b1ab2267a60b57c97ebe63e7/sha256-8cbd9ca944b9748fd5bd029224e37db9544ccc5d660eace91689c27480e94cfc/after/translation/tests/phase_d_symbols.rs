//! Phase D — symbol parity enforced as a test.
//!
//! Every dynamic symbol the C `.so` defines must also be defined by the Rust
//! `.so` under the exact same name, and every one must be loadable via
//! `libloading` (i.e. really reachable by an external caller).

mod common;

use common::*;
use std::path::PathBuf;
use std::process::Command;

fn so_paths() -> (PathBuf, PathBuf) {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let c = std::env::var("DRIVER_C_SO")
        .map(PathBuf::from)
        .unwrap_or_else(|_| manifest.join("../c_src/build/libdriver.so"));
    let r = std::env::var("DRIVER_RUST_SO").map(PathBuf::from).unwrap_or_else(|_| {
        let exe = std::env::current_exe().unwrap();
        let mut d = exe.parent().unwrap().to_path_buf();
        if d.file_name().map(|s| s == "deps").unwrap_or(false) {
            d.pop();
        }
        let cand = d.join("libdriver.so");
        if cand.exists() {
            cand
        } else {
            manifest.join("target/release/libdriver.so")
        }
    });
    (c, r)
}

/// Names of `T`/`D`/`B`/`R` (defined) dynamic symbols, sorted.
fn defined_dynamic_symbols(path: &PathBuf) -> Vec<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", "--format=posix"])
        .arg(path)
        .output()
        .expect("running `nm` failed — is binutils installed?");
    assert!(out.status.success(), "nm failed on {}", path.display());
    let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().next().map(|s| s.to_string()))
        .filter(|n| !n.starts_with("_ITM_") && !n.starts_with("__cxa") && n != "__gmon_start__")
        .collect();
    v.sort();
    v.dedup();
    v
}

#[test]
fn symbols_c_so_is_fully_covered_by_rust_so() {
    let (c_path, r_path) = so_paths();
    let c_syms = defined_dynamic_symbols(&c_path);
    let r_syms = defined_dynamic_symbols(&r_path);

    assert_eq!(
        c_syms,
        vec!["bad", "driver", "good", "printIntLine", "printLine"],
        "the C .so no longer exports the symbol set SYMBOLS.md was derived from"
    );

    let missing: Vec<&String> = c_syms.iter().filter(|s| !r_syms.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}\n\
         C   : {c_syms:?}\n\
         Rust: {r_syms:?}"
    );
}

#[test]
fn symbols_rust_so_has_no_undefined_non_libc_symbols() {
    let (_, r_path) = so_paths();
    let out = Command::new("nm")
        .args(["-D", "-u", "--format=posix"])
        .arg(&r_path)
        .output()
        .expect("nm");
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    let bad: Vec<&str> = text
        .lines()
        .filter_map(|l| l.split_whitespace().next())
        .filter(|n| {
            // Everything the Rust std runtime legitimately imports resolves
            // through glibc / libgcc, which nm marks with a version tag or a
            // reserved `_`-prefixed name.
            !n.contains("@GLIBC")
                && !n.contains("@GCC")
                && !n.starts_with('_')
                && !n.starts_with("__")
        })
        .collect();
    assert!(bad.is_empty(), "Rust .so has undefined non-libc symbols: {bad:?}");
}

#[test]
fn symbols_all_are_callable_through_libloading() {
    // Loading the Api resolves all five symbols from each .so via dlsym; a
    // successful load of both proves external callers can reach every export.
    let c = c_api();
    let r = rust_api();
    assert_eq!(c.name, "C");
    assert_eq!(r.name, "Rust");
    // And they must actually run.
    assert_same("D smoke: all five entry points", |api| {
        let s = CBuf::new(b"smoke");
        api.print_line(s.as_ptr());
        api.print_line(std::ptr::null());
        api.print_int_line(-7);
        api.good();
        api.bad();
        api.driver();
    });
}

/// `CONFIGS.md` records that the crate declares no `[features]`; if that ever
/// changes, the Phase D feature sweep must be extended, so pin it here.
#[test]
fn no_cargo_features_declared() {
    let manifest = std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"),
    )
    .unwrap();
    assert!(
        !manifest.contains("[features]"),
        "Cargo.toml now declares features — extend the Phase D feature sweep \
         (scripts/verify_all.sh) and CONFIGS.md accordingly"
    );
}
