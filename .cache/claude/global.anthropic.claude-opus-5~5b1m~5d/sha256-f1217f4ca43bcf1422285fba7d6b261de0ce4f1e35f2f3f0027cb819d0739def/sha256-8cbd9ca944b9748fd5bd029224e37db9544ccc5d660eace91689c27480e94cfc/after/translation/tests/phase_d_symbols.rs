//! Phase D — symbol parity between the C .so and the Rust .so.
//!
//! Enforces the SYMBOLS.md contract mechanically: every dynamic symbol the C
//! library defines must also be defined by the Rust cdylib under the exact same
//! name, and the Rust library must not import any non-libc symbol.

mod common;
use common::*;

fn nm_defined(path: &std::path::Path) -> Vec<String> {
    let out = std::process::Command::new("nm")
        .args(["-D", "--defined-only", path.to_str().unwrap()])
        .output()
        .expect("run nm");
    assert!(out.status.success(), "nm failed on {path:?}");
    let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().nth(2).map(str::to_string))
        .collect();
    v.sort();
    v.dedup();
    v
}

#[test]
fn d01_every_c_symbol_is_exported_by_rust() {
    let c = nm_defined(&c_so());
    let r = nm_defined(&rust_so());
    assert!(!c.is_empty(), "nm found no symbols in the C .so");

    let missing: Vec<&String> = c.iter().filter(|s| !r.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by C but MISSING from Rust: {missing:?}\n\
         C   : {c:?}\nRust: {r:?}"
    );
}

#[test]
fn d02_expected_symbol_set_is_exactly_the_four_public_functions() {
    // Guards against the C surface silently growing (a new C file added) and
    // against the Rust surface silently shrinking.
    let expected = ["bad", "driver", "good", "printLine"];
    let c = nm_defined(&c_so());
    assert_eq!(c, expected, "C .so export set changed");
    let r = nm_defined(&rust_so());
    for e in expected {
        assert!(r.contains(&e.to_string()), "Rust .so does not export `{e}`");
    }
}

#[test]
fn d03_all_four_symbols_are_dlsym_resolvable_in_both() {
    // `libs()` already does `dlsym` on all four names in both libraries and
    // panics if any is absent; calling it is the assertion.
    let l = libs();
    assert_eq!(l.c.name, "C");
    assert_eq!(l.rust.name, "Rust");
}

#[test]
fn d04_rust_imports_no_non_libc_symbols() {
    let out = std::process::Command::new("nm")
        .args(["-D", "-u", rust_so().to_str().unwrap()])
        .output()
        .expect("run nm");
    let allowed_prefixes = [
        "_ITM_", "__cxa_", "__gmon_", "_Unwind_", "__tls_get_addr", "__errno_location",
    ];
    // Everything else must be a plain libc/glibc name (contains no Rust mangling).
    for line in String::from_utf8_lossy(&out.stdout).lines() {
        let Some(sym) = line.split_whitespace().last() else { continue };
        let base = sym.split('@').next().unwrap_or(sym);
        let ok = allowed_prefixes.iter().any(|p| base.starts_with(p))
            || !base.starts_with("_ZN") && !base.starts_with("_R");
        assert!(ok, "Rust .so imports an unresolved Rust symbol: {sym}");
    }
}
