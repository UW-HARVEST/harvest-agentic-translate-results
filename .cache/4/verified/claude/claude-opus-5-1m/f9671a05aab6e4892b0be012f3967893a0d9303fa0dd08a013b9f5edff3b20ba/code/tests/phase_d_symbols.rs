//! Phase D — exported-symbol parity between the C `.so` and the Rust `.so`.
//!
//! Every symbol the C shared object exports must be exported by the Rust
//! shared object under the exact same linker name.

mod common;

use std::collections::BTreeSet;
use std::process::Command;

fn defined_symbols(so: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(so)
        .output()
        .expect("run nm (binutils required)");
    assert!(
        out.status.success(),
        "nm failed on {}:\n{}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let (a, b, c) = (it.next(), it.next(), it.next());
            match (a, b, c) {
                // "<addr> <type> <name>"
                (Some(_), Some(_), Some(name)) => Some(name.to_string()),
                // "         <type> <name>" (undefined-style, shouldn't appear)
                (Some(_), Some(name), None) => Some(name.to_string()),
                _ => None,
            }
        })
        .collect()
}

#[test]
fn symbol_parity_c_so_vs_rust_so() {
    let c_so = common::c_so_path();
    let rust_so = common::rust_so_path();
    assert!(c_so.is_file(), "missing {}", c_so.display());
    assert!(rust_so.is_file(), "missing {}", rust_so.display());

    let c_syms = defined_symbols(&c_so);
    let r_syms = defined_symbols(&rust_so);

    assert!(
        !c_syms.is_empty(),
        "nm reported no defined symbols for the C .so"
    );

    let missing: Vec<&String> = c_syms.difference(&r_syms).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing {} symbol(s) exported by the C .so: {:?}\n\
         C  ({}): {:?}\nRUST ({}): {:?}",
        missing.len(),
        missing,
        c_so.display(),
        c_syms,
        rust_so.display(),
        r_syms,
    );
}

/// The three functions with external linkage in `driver.c` must all be
/// resolvable through `dlsym` in both objects (this is what `common::both()`
/// does, so a missing export makes every other test fail too).
#[test]
fn all_three_entry_points_are_dlsym_resolvable() {
    let (c, r) = common::both();
    assert_eq!(c.name, "C");
    assert_eq!(r.name, "RUST");
    // Non-null function pointers by construction; assert the addresses differ
    // so we really did load two distinct objects.
    assert_ne!(
        c.fma_array as usize, r.fma_array as usize,
        "C and Rust fma_array resolved to the same address — only one .so loaded?"
    );
    assert_ne!(c.call_fma as usize, r.call_fma as usize);
    assert_ne!(c.driver as usize, r.driver as usize);
}

/// The C `.so` must not export anything beyond what `driver.c` defines, and the
/// Rust `.so` must cover exactly that set (guards against a future C file being
/// added without a matching translation).
#[test]
fn c_export_inventory_is_fully_translated() {
    let expected: BTreeSet<String> = ["call_fma", "driver", "fma_array"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    let c_syms = defined_symbols(&common::c_so_path());
    assert_eq!(
        c_syms, expected,
        "the C .so export set changed; SYMBOLS.md and the Rust translation must be updated"
    );
    let r_syms = defined_symbols(&common::rust_so_path());
    assert!(
        expected.is_subset(&r_syms),
        "Rust .so does not export the full C inventory: {:?}",
        expected.difference(&r_syms).collect::<Vec<_>>()
    );
}
