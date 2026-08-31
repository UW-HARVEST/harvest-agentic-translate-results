//! Every symbol the C shared object exports must also be exported, under the
//! exact same name, by the Rust `cdylib`.

mod common;

use common::*;
use std::process::Command;

/// Global function/data symbols defined by an ELF shared object, as reported by
/// `nm -D --defined-only`.
fn exported_symbols(path: &std::path::Path) -> Vec<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(path)
        .output()
        .expect("running `nm` failed");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8_lossy(&out.stdout);
    let mut syms: Vec<String> = text
        .lines()
        .filter_map(|line| {
            let mut it = line.split_whitespace();
            let (a, b, c) = (it.next(), it.next(), it.next());
            let (kind, name) = match (a, b, c) {
                // "<addr> <type> <name>"
                (Some(_), Some(k), Some(n)) => (k, n),
                // "         <type> <name>" (undefined/absolute, no address)
                (Some(k), Some(n), None) => (k, n),
                _ => return None,
            };
            // Keep global text and data symbols; skip weak/local/compiler noise.
            if matches!(kind, "T" | "D" | "B" | "R" | "G" | "S" | "i") {
                Some(name.to_string())
            } else {
                None
            }
        })
        .collect();
    syms.sort();
    syms.dedup();
    syms
}

#[test]
fn rust_so_exports_every_c_symbol() {
    let c_syms = exported_symbols(&c_so_path());
    let rust_syms = exported_symbols(&rust_so_path());

    assert!(
        !c_syms.is_empty(),
        "no exported symbols found in the C library"
    );

    let missing: Vec<&String> = c_syms.iter().filter(|s| !rust_syms.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but missing from the Rust .so: {missing:?}\n\
         C   : {c_syms:?}\n Rust: {rust_syms:?}"
    );
}

/// The documented API plus the two non-static helpers, which the C translation
/// unit also exposes.
#[test]
fn expected_symbols_are_present_in_both() {
    for name in ["driver", "call_fma", "fma_array"] {
        for (label, syms) in [
            ("C", exported_symbols(&c_so_path())),
            ("Rust", exported_symbols(&rust_so_path())),
        ] {
            assert!(
                syms.contains(&name.to_string()),
                "{label} .so does not export `{name}`"
            );
        }
    }
}

/// All three symbols must be resolvable via `dlsym` in both objects.
#[test]
fn all_symbols_resolve_via_dlsym() {
    let _ = c_fma_array();
    let _ = rust_fma_array();
    let _ = c_call_fma();
    let _ = rust_call_fma();
    let _ = c_driver();
    let _ = rust_driver();
}
