// Phase D — symbol parity between the C .so and the Rust .so, plus a smoke
// test that the harness really loads two distinct libraries.

mod support;

use std::collections::BTreeSet;
use std::process::Command;

fn defined_symbols(path: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", "--format=posix"])
        .arg(path)
        .output()
        .expect("failed to run nm");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let name = it.next()?;
            let kind = it.next()?;
            // Only global text/data symbols; skip nothing else exists here.
            if matches!(kind, "T" | "D" | "B" | "R" | "W" | "V") {
                Some(name.to_string())
            } else {
                None
            }
        })
        .collect()
}

fn undefined_symbols(path: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "-u", "--format=posix"])
        .arg(path)
        .output()
        .expect("failed to run nm");
    assert!(out.status.success(), "nm -u failed on {}", path.display());
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().next().map(|s| s.to_string()))
        .collect()
}

#[test]
fn d1_every_c_symbol_is_exported_by_rust() {
    let c_path = support::c_lib_path();
    let r_path = support::rust_lib_path();
    let c_syms = defined_symbols(&c_path);
    let r_syms = defined_symbols(&r_path);

    assert!(
        c_syms.len() >= 10,
        "unexpectedly few C symbols: {c_syms:?} — is {} the right library?",
        c_path.display()
    );

    let missing: Vec<&String> = c_syms.difference(&r_syms).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}\n\
         C   ({}): {c_syms:?}\n\
         Rust({}): {r_syms:?}",
        c_path.display(),
        r_path.display()
    );
}

#[test]
fn d2_expected_symbol_set() {
    // The exhaustive list from SYMBOLS.md, checked against both libraries so a
    // renamed/typo'd `#[no_mangle]` cannot slip through.
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
    let c_syms = defined_symbols(&support::c_lib_path());
    let r_syms = defined_symbols(&support::rust_lib_path());
    for s in EXPECTED {
        assert!(c_syms.contains(s), "C .so is missing {s} (build stale?)");
        assert!(r_syms.contains(s), "Rust .so is missing {s}");
    }
    // The C library exports exactly these ten.
    let extra_c: Vec<&String> = c_syms
        .iter()
        .filter(|s| !EXPECTED.contains(&s.as_str()))
        .collect();
    assert!(extra_c.is_empty(), "unexpected extra C exports: {extra_c:?}");
}

#[test]
fn d3_rust_has_no_dangling_non_libc_imports() {
    let undef = undefined_symbols(&support::rust_lib_path());
    // Everything the Rust .so imports must be resolvable from libc/libgcc,
    // which is guaranteed by dlopen(RTLD_NOW) succeeding.
    let _ = support::rust_api(); // RTLD_NOW load: fails loudly on any dangling symbol
    assert!(!undef.is_empty(), "expected libc imports, got none");
}

#[test]
fn d4_two_distinct_libraries_are_loaded() {
    let (c, r) = support::both();
    assert_ne!(c.path, r.path);
    assert_ne!(
        c.charinbuf as usize, r.charinbuf as usize,
        "both handles resolved to the same charinbuf — the symbol scopes leaked"
    );
    assert_ne!(c.increment_counter as usize, r.increment_counter as usize);
}

#[test]
fn d5_smoke_charinbuf_all_modes() {
    for mode in -1..=5 {
        support::diff_charinbuf(mode, 7, 3, 2);
    }
}
