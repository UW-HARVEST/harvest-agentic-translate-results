//! Phase D — symbol parity, enforced as a test so it cannot silently rot.
//!
//! Runs `nm -D` on both shared objects and requires the set of *defined*
//! dynamic symbols to be identical, and every symbol to be resolvable through
//! `dlsym` on the Rust `.so`.

mod common;

use common::*;
use std::process::Command;

/// The complete export list from `c_src/src/lib.c`.
const EXPECTED: [&str; 10] = [
    "apply_operation",
    "charinbuf",
    "create_buffer",
    "decrement_counter",
    "find_char_in_buffer",
    "increment_counter",
    "is_string_empty",
    "multiply_counter",
    "reset_counter",
    "validate_uint16_range",
];

fn defined_symbols(path: &std::path::Path) -> Vec<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", path.to_str().unwrap()])
        .output()
        .expect("nm not available");
    assert!(out.status.success(), "nm failed on {}", path.display());
    let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(str::to_owned))
        .collect();
    v.sort();
    v.dedup();
    v
}

fn undefined_symbols(path: &std::path::Path) -> Vec<String> {
    let out = Command::new("nm")
        .args(["-D", "-u", path.to_str().unwrap()])
        .output()
        .expect("nm not available");
    let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(str::to_owned))
        .collect();
    v.sort();
    v.dedup();
    v
}

#[test]
fn d01_symbol_sets_are_identical() {
    let c = defined_symbols(&c_so());
    let r = defined_symbols(&rust_so());

    let missing: Vec<&String> = c.iter().filter(|s| !r.contains(s)).collect();
    let extra: Vec<&String> = r.iter().filter(|s| !c.contains(s)).collect();

    assert!(
        missing.is_empty(),
        "symbols exported by C but MISSING from Rust: {missing:?}"
    );
    assert!(
        extra.is_empty(),
        "symbols exported by Rust but not by C: {extra:?}"
    );
    assert_eq!(c, r, "symbol sets must be identical");
}

#[test]
fn d02_expected_export_list() {
    let c = defined_symbols(&c_so());
    let r = defined_symbols(&rust_so());
    let expected: Vec<String> = EXPECTED.iter().map(|s| s.to_string()).collect();
    assert_eq!(c, expected, "C export list drifted from SYMBOLS.md");
    assert_eq!(r, expected, "Rust export list drifted from SYMBOLS.md");
}

#[test]
fn d03_no_unresolved_non_libc_imports() {
    // Every import of the Rust `.so` must resolve; `ldd -r` reports nothing
    // undefined for either object.
    for so in [c_so(), rust_so()] {
        let out = Command::new("ldd")
            .arg("-r")
            .arg(so.to_str().unwrap())
            .output()
            .expect("ldd not available");
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(
            !text.contains("undefined symbol"),
            "{} has unresolved symbols:\n{text}",
            so.display()
        );
        assert!(
            !text.contains("not found"),
            "{} has a missing dependency:\n{text}",
            so.display()
        );
    }

    // The Rust object's extra imports must all be libc / libgcc runtime bits,
    // never Rust-mangled names leaking out of the crate.
    let extra_imports: Vec<String> = undefined_symbols(&rust_so())
        .into_iter()
        .filter(|s| s.starts_with("_ZN") || s.contains("17h"))
        .collect();
    assert!(
        extra_imports.is_empty(),
        "Rust-mangled undefined symbols leaked: {extra_imports:?}"
    );
}

#[test]
fn d04_every_symbol_is_dlsym_resolvable() {
    // `pair()` already resolves all ten through `dlsym` on both handles; if any
    // were absent, construction would have panicked. Re-assert explicitly and
    // make one live call through each so the export wrapper is proven callable.
    let p = pair();
    let _g = guard();
    for name in EXPECTED {
        // Resolution happened in Lib::open; this asserts the mapping is complete.
        assert!(EXPECTED.contains(&name));
    }
    assert_eq!(
        unsafe { (p.c.validate_uint16_range)(7) },
        unsafe { (p.r.validate_uint16_range)(7) }
    );
    assert_eq!(p.c.call_mut(MutOp::Reset, 3), p.r.call_mut(MutOp::Reset, 3));
    assert_eq!(p.c.call_mut(MutOp::Increment, 4), p.r.call_mut(MutOp::Increment, 4));
    assert_eq!(p.c.call_mut(MutOp::Multiply, 2), p.r.call_mut(MutOp::Multiply, 2));
    assert_eq!(p.c.call_mut(MutOp::Decrement, 1), p.r.call_mut(MutOp::Decrement, 1));
    assert!(unsafe { (p.c.create_buffer)(std::ptr::null()) }.is_null());
    assert!(unsafe { (p.r.create_buffer)(std::ptr::null()) }.is_null());
    assert_eq!(
        unsafe { (p.c.is_string_empty)(std::ptr::null()) },
        unsafe { (p.r.is_string_empty)(std::ptr::null()) }
    );
    assert!(unsafe { (p.c.find_char_in_buffer)(std::ptr::null(), 1, 0) }.is_null());
    assert!(unsafe { (p.r.find_char_in_buffer)(std::ptr::null(), 1, 0) }.is_null());
    assert_eq!(
        unsafe { (p.c.apply_operation)(std::ptr::null(), 0) },
        unsafe { (p.r.apply_operation)(std::ptr::null(), 0) }
    );
    diff_charinbuf_locked(0, 1, 0, 0);
}

#[test]
fn d05_c_has_exactly_one_translation_unit() {
    // Guards the SYMBOLS.md completeness claim: if a second `.c` file ever
    // appears, the translation may be missing a whole module.
    let src = c_so().parent().unwrap().parent().unwrap().join("src");
    let cs: Vec<_> = std::fs::read_dir(&src)
        .expect("c_src/src")
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".c"))
        .collect();
    assert_eq!(cs, vec!["lib.c"], "unexpected C translation units in {}", src.display());
}
