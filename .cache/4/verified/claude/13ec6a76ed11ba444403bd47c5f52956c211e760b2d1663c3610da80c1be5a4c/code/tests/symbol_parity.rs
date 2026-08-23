//! Phase D — symbol parity enforced as a test.
//!
//! Reads the exported-symbol table of the C `.so` and asserts that the Rust
//! `.so` exports EVERY one of them under the exact same name (including
//! macro-generated names such as the `XXH_NAMESPACE`-prefixed `LZ4_XXH*`
//! family), and that the Rust `.so` exports nothing extra.
//!
//! This is the mechanical gate behind `SYMBOLS.md`: if a future edit drops a
//! `#[no_mangle]` wrapper, this test fails.

mod common;

use common::libs;
use std::process::Command;

fn nm_defined(path: &str) -> Vec<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", path])
        .output()
        .expect("failed to run `nm` (binutils required)");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        path,
        String::from_utf8_lossy(&out.stderr)
    );
    let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let f: Vec<&str> = l.split_whitespace().collect();
            // "<addr> <type> <name>" — keep only exported data/code symbols.
            if f.len() == 3 && matches!(f[1], "T" | "D" | "B" | "R") {
                Some(f[2].to_string())
            } else {
                None
            }
        })
        .collect();
    v.sort();
    v.dedup();
    v
}

fn c_so() -> String {
    format!("{}/c_src/build/liblz4.so", env!("CARGO_MANIFEST_DIR"))
}

fn rust_so() -> String {
    let profile = if cfg!(debug_assertions) { "debug" } else { "release" };
    format!("{}/target/{}/liblz4.so", env!("CARGO_MANIFEST_DIR"), profile)
}

#[test]
fn every_c_symbol_is_exported_by_rust() {
    let c = nm_defined(&c_so());
    let r = nm_defined(&rust_so());

    assert!(!c.is_empty(), "C .so exported no symbols — bad build?");

    let missing: Vec<&String> = c.iter().filter(|s| !r.contains(s)).collect();
    let extra: Vec<&String> = r.iter().filter(|s| !c.contains(s)).collect();

    if !missing.is_empty() || !extra.is_empty() {
        panic!(
            "symbol parity broken\n  C exports:    {}\n  Rust exports: {}\n\
             \n  MISSING from Rust ({}): {:?}\n  EXTRA in Rust ({}): {:?}",
            c.len(),
            r.len(),
            missing.len(),
            missing,
            extra.len(),
            extra
        );
    }
    assert_eq!(c.len(), r.len(), "symbol counts must match exactly");
}

#[test]
fn rust_so_has_no_unresolved_non_libc_symbols() {
    let out = Command::new("nm")
        .args(["-D", "--undefined-only", &rust_so()])
        .output()
        .expect("failed to run nm");
    let undef: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().nth(1).map(|s| s.to_string()))
        .collect();

    // Everything the Rust cdylib imports must resolve to the C runtime. A
    // leftover translation-level symbol (an un-translated helper) would show up
    // here without a GLIBC/libgcc version tag.
    let suspicious: Vec<&String> = undef
        .iter()
        .filter(|s| {
            !s.contains("@GLIBC")
                && !s.contains("@GCC")
                && !s.starts_with("_ITM_")
                && !s.starts_with("__gmon_start__")
                && !s.starts_with("__cxa_")
                && !s.starts_with("_Unwind_")
                && !s.starts_with("__tls_get_addr")
        })
        .collect();
    assert!(
        suspicious.is_empty(),
        "Rust .so has unresolved non-libc symbols: {:?}",
        suspicious
    );
}

#[test]
fn both_libraries_load_and_expose_the_same_names() {
    // Belt-and-braces: resolve every C symbol name through the Rust library's
    // dynamic-symbol table via dlsym, which is what an external caller does.
    let c = nm_defined(&c_so());
    let l = libs();
    let mut unresolvable = Vec::new();
    for s in &c {
        if !l.rust.has(s) {
            unresolvable.push(s.clone());
        }
    }
    assert!(
        unresolvable.is_empty(),
        "these C symbols are not dlsym-resolvable in the Rust .so: {:?}",
        unresolvable
    );
}
