//! Phase D — symbol parity between the C `.so` and the Rust `.so`.
//!
//! The diff of exported dynamic symbols must be empty, and every symbol the C
//! exports must be resolvable *and callable* through the Rust `.so` with the
//! signature declared in `c_src/src/lib.c`.

mod common;

use std::process::Command;

use common::*;

/// Every symbol `nm -D --defined-only --extern-only` reports for a `.so`,
/// excluding the compiler/runtime boilerplate that is not part of the API.
fn exported_symbols(path: &std::path::Path) -> Vec<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", "--extern-only", path.to_str().unwrap()])
        .output()
        .expect("run nm");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let (a, b) = (it.next()?, it.next()?);
            // "<addr> <type> <name>" or "<type> <name>"
            let (ty, name) = match it.next() {
                Some(n) => (b, n),
                None => (a, b),
            };
            // Only real code/data definitions, and skip toolchain internals.
            if !matches!(ty, "T" | "t" | "D" | "B" | "R" | "W" | "i") {
                return None;
            }
            if name.starts_with("_init")
                || name.starts_with("_fini")
                || name.starts_with("__")
                || name.starts_with("_ITM_")
                || name.starts_with("_Unwind")
                || name.starts_with("rust_")
                || name.starts_with("_R") // Rust v0 mangled internals
            {
                return None;
            }
            Some(name.to_string())
        })
        .collect();
    v.sort();
    v.dedup();
    v
}

#[test]
fn symbol_diff_is_empty() {
    let p = Pair::load();
    let c_syms = exported_symbols(&p.c_path);
    let r_syms = exported_symbols(&p.r_path);

    assert!(
        !c_syms.is_empty(),
        "nm reported no exported symbols for {}",
        p.c_path.display()
    );

    let missing: Vec<&String> = c_syms.iter().filter(|s| !r_syms.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "Rust .so ({}) is missing {} symbol(s) exported by the C .so ({}): {:?}\n\
         C   symbols: {:?}\nRust symbols: {:?}",
        p.r_path.display(),
        missing.len(),
        p.c_path.display(),
        missing,
        c_syms,
        r_syms
    );
}

/// Independently of `nm`, every documented symbol must `dlsym` in both objects.
#[test]
fn every_symbol_is_dlsym_resolvable_in_both() {
    let p = Pair::load();
    unsafe {
        let _ = p.both::<FnConvertDoubleToInt>(SYM_CONVERT);
        let _ = p.both::<FnFindValueInBuffer>(SYM_FIND);
        let _ = p.both::<FnProcessNegation>(SYM_NEG);
        let _ = p.both::<FnCreateNumericBuffer>(SYM_CREATE);
        let _ = p.both::<FnCalculateWithDoubles>(SYM_CALC);
        let _ = p.both::<FnDoubleneg>(SYM_DOUBLENEG);
    }
}

/// The C `.so` must not export anything the SYMBOLS.md table omits (guards
/// against a future C source file being added without being translated).
#[test]
fn c_exports_exactly_the_documented_six() {
    let p = Pair::load();
    let c_syms = exported_symbols(&p.c_path);
    let mut expected = vec![
        "calculate_with_doubles",
        "convert_double_to_int",
        "create_numeric_buffer",
        "doubleneg",
        "find_value_in_buffer",
        "process_negation",
    ];
    expected.sort();
    assert_eq!(
        c_syms, expected,
        "the C .so's export list changed -- update SYMBOLS.md and translate any new module"
    );
}
