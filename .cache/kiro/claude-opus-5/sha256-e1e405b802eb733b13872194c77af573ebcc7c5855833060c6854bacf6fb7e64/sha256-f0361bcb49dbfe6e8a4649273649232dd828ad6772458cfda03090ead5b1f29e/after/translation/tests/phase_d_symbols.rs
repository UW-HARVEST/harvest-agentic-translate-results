//! Phase D — symbol parity enforced from inside the test suite, so a
//! regression in the export surface fails `cargo test` and not just the
//! shell script.

mod common;

use std::process::Command;

fn defined_symbols(so: &std::path::Path) -> Vec<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(so)
        .output()
        .expect("failed to run nm");
    assert!(out.status.success(), "nm failed on {}", so.display());
    let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().nth(2).map(str::to_string))
        .collect();
    v.sort();
    v.dedup();
    v
}

fn undefined_symbols(so: &std::path::Path) -> Vec<String> {
    let out = Command::new("nm")
        .args(["-D", "--undefined-only"])
        .arg(so)
        .output()
        .expect("failed to run nm");
    let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(str::to_string))
        .collect();
    v.sort();
    v.dedup();
    v
}

#[test]
fn phase_d_symbol_parity() {
    let (c_so, r_so) = common::lib_paths();
    eprintln!("C    .so: {}", c_so.display());
    eprintln!("Rust .so: {}", r_so.display());
    assert_ne!(c_so, r_so, "the two libraries must be different files");

    let c_syms = defined_symbols(&c_so);
    let r_syms = defined_symbols(&r_so);

    let missing: Vec<&String> = c_syms.iter().filter(|s| !r_syms.contains(s)).collect();
    eprintln!(
        "C defines {} symbols, Rust defines {}",
        c_syms.len(),
        r_syms.len()
    );
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}"
    );
    // Sanity: the C library is not empty and every name really is present.
    assert_eq!(c_syms.len(), 38, "unexpected C export count: {c_syms:?}");
    for s in &c_syms {
        assert!(r_syms.contains(s), "{s} missing");
    }
}

#[test]
fn phase_d_no_unresolved_symbols() {
    let (_, r_so) = common::lib_paths();
    let bad: Vec<String> = undefined_symbols(&r_so)
        .into_iter()
        .filter(|s| {
            !(s.contains("@GLIBC")
                || s.contains("@GCC")
                || s.starts_with("_ITM_")
                || s.starts_with("_Unwind_")
                || s.starts_with("__cxa_")
                || s.starts_with("__tls_get_addr")
                || s == "__gmon_start__")
        })
        .collect();
    assert!(
        bad.is_empty(),
        "Rust .so has non-libc undefined symbols: {bad:?}"
    );
}
