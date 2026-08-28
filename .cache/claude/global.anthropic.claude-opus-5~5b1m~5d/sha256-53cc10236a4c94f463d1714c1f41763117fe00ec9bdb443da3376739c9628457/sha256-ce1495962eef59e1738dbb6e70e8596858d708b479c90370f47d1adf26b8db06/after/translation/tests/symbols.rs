//! Phase A.1 / Phase D — symbol parity between the C `.so` and the Rust `.so`.

mod common;

use std::process::Command;

/// `nm -D --defined-only <so>` → the set of exported names.
fn exported_symbols(so: &std::path::Path) -> std::collections::BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", so.to_str().unwrap()])
        .output()
        .expect("failed to run `nm` -- binutils is required for this test");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );

    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|line| {
            // "0000000000001179 T convert_double_to_int"  |  "         w  weaksym"
            let mut it = line.split_whitespace();
            let a = it.next()?;
            let b = it.next()?;
            match it.next() {
                Some(name) => {
                    let _ = (a, b);
                    Some(name.to_string())
                }
                // Two-column form: "<type> <name>".
                None => {
                    if a.len() == 1 {
                        Some(b.to_string())
                    } else {
                        None
                    }
                }
            }
        })
        .map(|s| s.split('@').next().unwrap().to_string())
        .collect()
}

/// The six symbols `nm -D` reports for the C library.
const EXPECTED: &[&str] = &[
    "calculate_with_doubles",
    "convert_double_to_int",
    "create_numeric_buffer",
    "doubleneg",
    "find_value_in_buffer",
    "process_negation",
];

#[test]
fn c_symbols_are_all_exported_by_rust() {
    let c_so = common::c_so_path();
    let rust_so = common::rust_so_path();
    eprintln!("C   .so: {}", c_so.display());
    eprintln!("Rust.so: {}", rust_so.display());

    let c_syms = exported_symbols(&c_so);
    let rust_syms = exported_symbols(&rust_so);

    let missing: Vec<&String> = c_syms.difference(&rust_syms).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}"
    );

    // Guard against the C library silently losing symbols (which would make the
    // difference trivially empty).
    for want in EXPECTED {
        assert!(
            c_syms.contains(*want),
            "C .so unexpectedly does not export {want}"
        );
    }
    assert_eq!(
        c_syms.len(),
        EXPECTED.len(),
        "C .so export set changed: {c_syms:?}"
    );
}

#[test]
fn every_symbol_is_dynamically_resolvable_in_both() {
    // `Api::load` panics with a descriptive message if any `dlsym` fails, so
    // simply loading both libraries proves all six names resolve.
    let (c, rust) = common::both();
    assert_eq!(c.label, "C");
    assert_eq!(rust.label, "Rust");
}

#[test]
fn rust_so_has_no_unresolved_non_libc_symbols() {
    let rust_so = common::rust_so_path();
    let out = Command::new("nm")
        .args(["-D", "--undefined-only", rust_so.to_str().unwrap()])
        .output()
        .expect("nm");
    assert!(out.status.success());

    // Anything that is not satisfied by libc/libm/libgcc would show up here and
    // would also have made `dlopen` fail; `dlopen` succeeding (below) is the
    // real gate, this test documents the list.
    let names: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(str::to_string))
        .collect();
    eprintln!("undefined symbols in Rust .so ({}): {names:?}", names.len());

    // `dlopen` with RTLD_NOW would fail on a genuinely unresolved symbol; the
    // successful load in `common::rust_api()` is the assertion.
    let _ = common::rust_api();
}
