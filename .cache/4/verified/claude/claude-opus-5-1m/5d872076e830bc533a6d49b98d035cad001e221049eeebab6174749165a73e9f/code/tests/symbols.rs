//! Phase D — symbol parity between the C `.so` and the Rust `.so`.
//!
//! Runs `nm -D --defined-only` on both libraries and asserts the C export set
//! is fully covered by the Rust export set, then verifies every C export is
//! actually resolvable via `dlsym` on the Rust library.

mod common;

use common::*;
use std::process::Command;

fn dynamic_exports(so: &std::path::Path) -> Vec<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", so.to_str().unwrap()])
        .output()
        .expect("run nm");
    assert!(
        out.status.success(),
        "nm failed on {so:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let _addr = it.next()?;
            let kind = it.next()?;
            let name = it.next()?;
            // exported code/data only ('T'/'D'/'B'/'R'), skip weak/local noise
            if matches!(kind, "T" | "D" | "B" | "R") {
                Some(name.to_string())
            } else {
                None
            }
        })
        .collect();
    v.sort();
    v.dedup();
    v
}

#[test]
fn c_exports_are_a_subset_of_rust_exports() {
    let c = dynamic_exports(&c_so_path());
    let r = dynamic_exports(&rust_so_path());
    assert!(!c.is_empty(), "no exports found in the C library");

    let missing: Vec<&String> = c.iter().filter(|s| !r.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}\n\
         C exports:    {c:?}\n\
         Rust exports: {r:?}"
    );

    // The two documented entry points must really be there.
    for want in ["extractFilename", "FIO_createFilename_fromOutDir"] {
        assert!(c.contains(&want.to_string()), "C .so missing {want}");
        assert!(r.contains(&want.to_string()), "Rust .so missing {want}");
    }
    eprintln!("C exports ({}) all present in Rust .so: {c:?}", c.len());
}

#[test]
fn every_c_export_is_resolvable_in_the_rust_library() {
    let c = dynamic_exports(&c_so_path());
    let lib = unsafe { libloading::Library::new(rust_so_path()) }.expect("dlopen rust .so");
    for name in &c {
        let mut sym = name.clone().into_bytes();
        sym.push(0);
        let found = unsafe { lib.get::<*const ()>(&sym) };
        assert!(
            found.is_ok(),
            "dlsym({name}) failed on the Rust library: {:?}",
            found.err()
        );
    }
}

/// Both libraries must be loadable at once, and the Rust one must not shadow or
/// fail to resolve any of its own imports.
#[test]
fn both_libraries_load_simultaneously() {
    let p = pair();
    assert_eq!(p.c.name, "C");
    assert_eq!(p.rust.name, "Rust");
}
