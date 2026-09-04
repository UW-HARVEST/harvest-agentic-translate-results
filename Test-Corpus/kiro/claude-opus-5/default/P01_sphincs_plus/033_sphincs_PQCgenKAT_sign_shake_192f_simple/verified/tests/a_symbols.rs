//! Phase D — symbol parity, checked from inside the test suite so that it is
//! re-verified for whatever feature combination cargo was invoked with.
//!
//! CONFIGS.md is not involved here; see SYMBOLS.md.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::process::Command;

fn defined_symbols(path: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(path)
        .output()
        .expect("run nm");
    assert!(out.status.success(), "nm failed on {path:?}");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let _addr = it.next()?;
            let kind = it.next()?;
            let name = it.next()?;
            if matches!(kind, "T" | "D" | "B" | "R" | "W") {
                Some(name.to_string())
            } else {
                None
            }
        })
        .collect()
}

fn undefined_symbols(path: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--undefined-only"])
        .arg(path)
        .output()
        .expect("run nm");
    assert!(out.status.success(), "nm failed on {path:?}");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .collect()
}

#[test]
fn every_c_symbol_is_exported_by_rust() {
    let libs = load();
    let mut c = defined_symbols(&libs.c_core_path);
    c.extend(defined_symbols(&libs.c_back_path));
    let r = defined_symbols(&libs.rs_path);

    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "{} C symbols missing from the Rust .so: {:?}",
        missing.len(),
        missing
    );
    // Every C symbol must also be reachable through dlsym on the Rust handle,
    // which is what an external caller would do.
    for name in &c {
        let _ = libs.r::<*const ()>(name);
    }
    eprintln!(
        "[{}] symbol parity: {} C symbols, {} Rust symbols, 0 missing",
        tag(),
        c.len(),
        r.len()
    );
}

#[test]
fn rust_so_has_no_unresolved_non_libc_symbols() {
    let libs = load();
    let undef = undefined_symbols(&libs.rs_path);
    // Whatever the Rust .so still needs must come from the platform C runtime,
    // never from the SPHINCS+ surface itself.
    let c = {
        let mut s = defined_symbols(&libs.c_core_path);
        s.extend(defined_symbols(&libs.c_back_path));
        s
    };
    let bad: Vec<&String> = undef.intersection(&c).collect();
    assert!(
        bad.is_empty(),
        "Rust .so expects these SPHINCS+ symbols from outside: {bad:?}"
    );
    // RTLD_NOW in load() already proved every remaining undefined symbol
    // resolves against the process image.
}
