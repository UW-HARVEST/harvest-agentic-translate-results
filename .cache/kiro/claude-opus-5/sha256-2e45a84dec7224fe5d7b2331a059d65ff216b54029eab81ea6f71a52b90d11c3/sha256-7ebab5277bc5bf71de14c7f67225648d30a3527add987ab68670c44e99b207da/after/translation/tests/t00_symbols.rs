//! Level 0: exported-symbol parity.
//!
//! Every dynamic symbol the C `.so` exports must also be exported, under the
//! exact same name, by the Rust `.so`.

mod common;

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

/// Names of globally-visible defined symbols in a shared object, read with
/// `nm -D --defined-only`.
fn exported_symbols(so: &Path) -> Option<BTreeSet<String>> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", "--format=posix"])
        .arg(so)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let mut set = BTreeSet::new();
    for line in text.lines() {
        let mut it = line.split_whitespace();
        let (Some(name), Some(kind)) = (it.next(), it.next()) else {
            continue;
        };
        // T/t: text, D/d/B/b: data/bss, W/w/V/v: weak, i/I: indirect.
        // Only globals (uppercase) are part of the public ABI.
        if matches!(kind, "T" | "D" | "B" | "R" | "W" | "V" | "i") {
            set.insert(name.to_string());
        }
    }
    Some(set)
}

#[test]
fn rust_so_exports_every_c_symbol() {
    let (c_so, rs_so) = common::library_paths();

    let Some(c_syms) = exported_symbols(&c_so) else {
        eprintln!("`nm` unavailable; skipping symbol parity check");
        return;
    };
    let rs_syms = exported_symbols(&rs_so).expect("nm worked on the C .so but not the Rust .so");

    assert!(
        !c_syms.is_empty(),
        "no exported symbols found in {}",
        c_so.display()
    );

    let missing: Vec<&String> = c_syms.difference(&rs_syms).collect();
    assert!(
        missing.is_empty(),
        "the Rust .so ({}) is missing {} symbol(s) exported by the C .so ({}): {:?}",
        rs_so.display(),
        missing.len(),
        c_so.display(),
        missing
    );
}

/// The functions named in `include/lib.h` plus the internal helpers the C `.so`
/// happens to export must all be resolvable via `dlsym` on both libraries.
#[test]
fn every_public_function_is_loadable_from_both() {
    let p = common::pair();
    // Touching each accessor performs the `dlsym` and panics with a clear
    // message if either library lacks the symbol.
    for lib in [&p.c, &p.rs] {
        let _ = lib.create_block();
        let _ = lib.allocate_block();
        let _ = lib.free_block();
        let _ = lib.compute_hash();
        let _ = lib.betagamma();
    }
}
