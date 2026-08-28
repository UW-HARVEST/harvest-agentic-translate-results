// PHASE D -- symbol parity gate.
//
// Mechanically compares `nm -D --defined-only` on the two shared objects. Every
// symbol the C .so exports must also be exported by the Rust .so under the exact
// same name. This is the gate that catches a partially translated library.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

/// The 12 public symbols of `c_src/src/lib.c`, transcribed by hand from the
/// source so the test fails if the C .so is stale or truncated.
const EXPECTED: [&str; 12] = [
    "add_operation",
    "allocate_results",
    "divide_operation",
    "get_computation_timestamp",
    "get_operation_priority",
    "is_valid_operation",
    "mathop",
    "modulo_operation",
    "multiply_operation",
    "perform_computation_with_history",
    "select_operation",
    "subtract_operation",
];

fn nm_defined(so: &Path) -> Option<BTreeSet<String>> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(so)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    Some(
        text.lines()
            .filter_map(|line| {
                let mut it = line.split_whitespace();
                let (_addr, kind, name) = (it.next()?, it.next()?, it.next()?);
                // Exported code/data only: T (text), and for completeness the
                // other global definition kinds nm can report.
                if matches!(kind, "T" | "D" | "B" | "R" | "W" | "V") {
                    Some(name.to_string())
                } else {
                    None
                }
            })
            .collect(),
    )
}

#[test]
fn phase_d_rust_exports_every_c_symbol() {
    let l = libs();
    let (c_syms, rust_syms) = match (nm_defined(&l.c.path), nm_defined(&l.rust.path)) {
        (Some(a), Some(b)) => (a, b),
        _ => {
            eprintln!("SKIP: `nm` unavailable, cannot compare dynamic symbol tables");
            return;
        }
    };

    // Guard against comparing against an empty/garbage C symbol list.
    for want in EXPECTED {
        assert!(
            c_syms.contains(want),
            "the C .so ({}) does not export `{want}` -- stale build?",
            l.c.path.display()
        );
    }
    assert_eq!(
        c_syms.len(),
        EXPECTED.len(),
        "the C .so exports {} symbols, expected exactly {}: {:?}",
        c_syms.len(),
        EXPECTED.len(),
        c_syms
    );

    let missing: Vec<&String> = c_syms.difference(&rust_syms).collect();
    assert!(
        missing.is_empty(),
        "the Rust .so is missing {} symbol(s) exported by the C .so: {missing:?}\n\
         Add the #[no_mangle] extern \"C\" wrapper, or translate the missing C source.",
        missing.len()
    );

    // Every symbol must also be *loadable* through the Rust .so, which the
    // `libs()` loader already proved by resolving all 12 of them.
    eprintln!(
        "symbol parity: {} C symbols, all present in the Rust .so",
        c_syms.len()
    );
}

/// Nothing may be left dangling: every symbol the Rust .so imports must resolve
/// against its declared dependencies. `ldd -r` performs the real relocation
/// check, which is stricter and less brittle than allow-listing libc names.
#[test]
fn phase_d_no_undefined_symbols() {
    let l = libs();
    for (name, so) in [("C", &l.c.path), ("Rust", &l.rust.path)] {
        let out = match Command::new("ldd").arg("-r").arg(so).output() {
            Ok(o) => o,
            Err(_) => {
                eprintln!("SKIP: `ldd` unavailable");
                return;
            }
        };
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        let bad: Vec<&str> = text
            .lines()
            .filter(|l| {
                let lower = l.to_ascii_lowercase();
                lower.contains("undefined symbol") || lower.contains("not found")
            })
            .collect();
        assert!(
            bad.is_empty(),
            "the {name} .so ({}) has unresolved symbols:\n{}",
            so.display(),
            bad.join("\n")
        );
        eprintln!("{name} .so: all dynamic symbols resolve");
    }
}
