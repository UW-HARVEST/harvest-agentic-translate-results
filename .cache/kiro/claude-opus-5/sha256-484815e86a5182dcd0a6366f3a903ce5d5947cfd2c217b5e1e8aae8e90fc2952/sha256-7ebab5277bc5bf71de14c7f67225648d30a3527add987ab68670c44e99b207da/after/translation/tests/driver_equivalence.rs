//! Differential test of the single public entry point, `void driver(char c)`.
//!
//! `driver` is the whole public API declared in `c_src/include/driver.h`; its
//! observable behaviour is the text it prints. Both the C reference `.so` and
//! the Rust `cdylib` are loaded with `libloading` and invoked through their
//! exported `driver` symbol, and the captured `stdout` is compared byte-for-byte.

mod common;

use common::{exported_symbols, pair, show};
use std::ffi::c_char;

/// Every value a `char` argument can take on this platform.
fn all_char_values() -> Vec<c_char> {
    (c_char::MIN..=c_char::MAX).collect()
}

#[test]
fn driver_matches_c_for_every_char_value() {
    let p = pair();

    let mut mismatches = Vec::new();
    for c in all_char_values() {
        let expected = p.c.call(c);
        let actual = p.rust.call(c);
        if expected != actual {
            mismatches.push(format!(
                "c = {c} (0x{:02x})\n  C:    {}\n  Rust: {}",
                c as u8,
                show(&expected),
                show(&actual),
            ));
        }
    }

    assert!(
        mismatches.is_empty(),
        "{} of 256 char values mismatched:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// The C reference output is non-empty and shaped as expected, guarding against
/// a capture bug that would make the comparison above vacuously pass.
#[test]
fn capture_is_not_vacuous() {
    let p = pair();

    let c_out = p.c.call(b'a' as c_char);
    assert_eq!(
        c_out.iter().filter(|&&b| b == b'\n').count(),
        14,
        "expected 14 printed lines from the C implementation, got: {}",
        show(&c_out)
    );
    assert!(
        c_out.starts_with(b"alphanumeric: "),
        "unexpected C output: {}",
        show(&c_out)
    );

    let rust_out = p.rust.call(b'a' as c_char);
    assert_eq!(show(&c_out), show(&rust_out));
}

/// Line-by-line comparison, so a failure names the specific classification
/// routine that diverged rather than dumping the whole block.
#[test]
fn each_reported_field_matches() {
    let p = pair();

    for c in all_char_values() {
        let expected = p.c.call(c);
        let actual = p.rust.call(c);

        let e_lines: Vec<&[u8]> = expected.split(|&b| b == b'\n').collect();
        let a_lines: Vec<&[u8]> = actual.split(|&b| b == b'\n').collect();
        assert_eq!(
            e_lines.len(),
            a_lines.len(),
            "line count differs for c = {c}: C {} vs Rust {}",
            show(&expected),
            show(&actual)
        );

        for (i, (e, a)) in e_lines.iter().zip(a_lines.iter()).enumerate() {
            assert_eq!(
                show(e),
                show(a),
                "line {i} differs for c = {c} (0x{:02x})",
                c as u8
            );
        }
    }
}

/// Every symbol the C `.so` exports must also be exported by the Rust `.so`.
#[test]
fn rust_exports_every_c_symbol() {
    let c_syms = exported_symbols(common::c_library());
    let rust_syms = exported_symbols(common::rust_library());

    assert!(
        c_syms.contains(&"driver".to_string()),
        "C library unexpectedly does not export `driver`: {c_syms:?}"
    );

    let missing: Vec<&String> = c_syms.iter().filter(|s| !rust_syms.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing symbols exported by the C .so: {missing:?}\n\
         C exports:    {c_syms:?}\n\
         Rust exports: {rust_syms:?}"
    );
}
