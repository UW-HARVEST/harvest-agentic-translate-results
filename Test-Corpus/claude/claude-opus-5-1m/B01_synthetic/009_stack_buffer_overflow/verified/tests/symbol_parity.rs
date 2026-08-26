//! Phase D — symbol parity between the C and Rust shared objects.
//!
//! The requirement is one-directional: every symbol the C `.so` exports, the Rust
//! `.so` must export under the exact same name. The Rust `.so` may export more
//! (its own `std` internals are not part of the contract), but the diff of
//! "C exports minus Rust exports" must be empty.

mod common;

use std::collections::BTreeSet;
use std::process::Command;

/// The five functions with external linkage in `c_src/src/main.c`.
/// `goodG2B` / `goodB2G` are `static` and must NOT appear.
const EXPECTED: [&str; 5] = ["bad", "good", "main", "printIntLine", "printLine"];

/// Defined, exported symbols of a shared object, as reported by `nm -D`.
fn exported(path: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", path.to_str().unwrap()])
        .output()
        .expect("run nm -D");
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
            let _addr = it.next()?;
            let kind = it.next()?;
            let name = it.next()?;
            // Exported code/data symbols.
            if matches!(kind, "T" | "t" | "D" | "B" | "R" | "W" | "i") {
                Some(name.to_string())
            } else {
                None
            }
        })
        .collect()
}

#[test]
fn c_so_exports_exactly_the_five_external_functions() {
    common::ensure_built();
    let c = exported(&common::c_lib());
    let got: BTreeSet<String> = EXPECTED.iter().map(|s| s.to_string()).collect();
    assert_eq!(
        c, got,
        "the C shared object's export set changed; regenerate SYMBOLS.md"
    );
    for hidden in ["goodG2B", "goodB2G"] {
        assert!(
            !c.contains(hidden),
            "{hidden} is `static` in C and must not be exported"
        );
    }
}

#[test]
fn every_c_symbol_is_exported_by_rust() {
    common::ensure_built();
    let c = exported(&common::c_lib());
    let r = exported(&common::rust_lib());

    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "the Rust .so is missing {} symbol(s) exported by the C .so: {missing:?}\n\
         C exports:    {c:?}\n\
         Rust exports: {:?}",
        missing.len(),
        r.intersection(&c).collect::<Vec<_>>()
    );
}

#[test]
fn rust_so_does_not_export_the_static_c_functions() {
    common::ensure_built();
    let r = exported(&common::rust_lib());
    for hidden in ["goodG2B", "goodB2G", "good_g2b", "good_b2g"] {
        assert!(
            !r.contains(hidden),
            "{hidden} has internal linkage in C; the Rust .so must not export it"
        );
    }
}

/// A symbol that resolves but is a stub would satisfy `nm` while lying about
/// behavior. Loading with `RTLD_NOW` forces every relocation to resolve, and each
/// of the five names is then actually invoked by the differential suites.
#[test]
fn both_shared_objects_load_eagerly_and_resolve_all_five() {
    common::ensure_built();
    for lib in [common::c_lib(), common::rust_lib()] {
        let l = unsafe {
            libloading::os::unix::Library::open(
                Some(&lib),
                libloading::os::unix::RTLD_NOW | libloading::os::unix::RTLD_LOCAL,
            )
        }
        .unwrap_or_else(|e| panic!("dlopen(RTLD_NOW) {}: {e}", lib.display()));
        for name in EXPECTED {
            let mut z = name.as_bytes().to_vec();
            z.push(0);
            let sym = unsafe { l.get::<*const ()>(&z) };
            assert!(
                sym.is_ok(),
                "dlsym({name}) failed in {}: {:?}",
                lib.display(),
                sym.err()
            );
        }
    }
}

/// The Rust `.so` must not need anything outside the standard system libraries,
/// so it is as self-contained as the C one.
#[test]
fn rust_so_needs_only_system_libraries() {
    common::ensure_built();
    let out = Command::new("objdump")
        .args(["-p", common::rust_lib().to_str().unwrap()])
        .output()
        .expect("objdump -p");
    let text = String::from_utf8_lossy(&out.stdout);
    let needed: Vec<&str> = text
        .lines()
        .filter(|l| l.trim_start().starts_with("NEEDED"))
        .map(|l| l.split_whitespace().nth(1).unwrap_or(""))
        .collect();
    assert!(!needed.is_empty(), "objdump reported no NEEDED entries");
    let allowed = [
        "libc.so.6",
        "libm.so.6",
        "libgcc_s.so.1",
        "libdl.so.2",
        "libpthread.so.0",
        "librt.so.1",
        "ld-linux-x86-64.so.2",
    ];
    for n in &needed {
        assert!(
            allowed.contains(n),
            "Rust .so depends on non-system library {n:?} (NEEDED = {needed:?})"
        );
    }
}
