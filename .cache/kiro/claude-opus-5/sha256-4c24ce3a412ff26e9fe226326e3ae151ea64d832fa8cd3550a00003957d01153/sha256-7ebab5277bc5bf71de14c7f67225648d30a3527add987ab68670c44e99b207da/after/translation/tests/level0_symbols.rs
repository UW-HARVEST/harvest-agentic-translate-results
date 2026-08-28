//! Level 0: ABI surface.
//!
//! Every dynamic symbol the C library exports must also be exported by the Rust
//! library under the exact same name, and every symbol must be callable through
//! `dlsym` with the expected signature.

mod common;

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::process::Command;

/// Exported (defined, dynamic) symbol names of a shared object.
fn exported_symbols(so: &PathBuf) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", "--format=posix"])
        .arg(so)
        .output()
        .unwrap_or_else(|e| panic!("failed to run nm on {}: {e}", so.display()));
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8_lossy(&out.stdout);
    let mut set = BTreeSet::new();
    for line in text.lines() {
        let mut it = line.split_whitespace();
        let (Some(name), Some(kind)) = (it.next(), it.next()) else {
            continue;
        };
        // Keep only global text/data definitions; skip the linker's own
        // bookkeeping symbols, which are not part of the API.
        if !matches!(kind, "T" | "D" | "B" | "R" | "W" | "i") {
            continue;
        }
        if matches!(
            name,
            "_init" | "_fini" | "__bss_start" | "_edata" | "_end" | "__libc_csu_init"
        ) {
            continue;
        }
        if name.starts_with("_ITM_")
            || name.starts_with("__gmon")
            || name.starts_with("__cxa")
            || name.starts_with("rust_eh_")
            || name.starts_with("__rust")
            || name.starts_with("_Unwind")
        {
            continue;
        }
        set.insert(name.to_string());
    }
    set
}

#[test]
fn rust_so_exports_every_c_symbol() {
    let (c_so, rust_so) = common::library_paths();
    let c = exported_symbols(&c_so);
    let r = exported_symbols(&rust_so);

    assert!(
        !c.is_empty(),
        "no symbols found in {} - is nm working?",
        c_so.display()
    );

    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "the Rust library is missing {} symbol(s) exported by the C library: {:?}\n\
         C exports {} symbols, Rust exports {}",
        missing.len(),
        missing,
        c.len(),
        r.len()
    );

    // Every C symbol must be reachable via dlsym on the Rust library too.
    let lib = unsafe { libloading::Library::new(&rust_so) }.expect("dlopen rust so");
    for name in c.iter() {
        let mut sym = name.clone().into_bytes();
        sym.push(0);
        let got = unsafe { lib.get::<*const ()>(&sym) };
        assert!(got.is_ok(), "dlsym failed for {name} on the Rust library");
    }
}

#[test]
fn all_expected_api_symbols_are_present() {
    // The complete set of functions with external linkage in c_src/src/lib.c.
    const EXPECTED: &[&str] = &[
        "c22",
        "c23",
        "c2AABBtoAABB",
        "c2AABBtoCapsule",
        "c2Add",
        "c2BBVerts",
        "c2CCW90",
        "c2CapsuletoCapsule",
        "c2CircletoAABB",
        "c2CircletoCapsule",
        "c2CircletoCircle",
        "c2Clampv",
        "c2Collided",
        "c2D",
        "c2Det2",
        "c2Div",
        "c2Dot",
        "c2GJK",
        "c2GJKSimplexMetric",
        "c2L",
        "c2Len",
        "c2MakeProxy",
        "c2Maxv",
        "c2Minv",
        "c2Mulrv",
        "c2MulrvT",
        "c2Mulvs",
        "c2Mulxv",
        "c2Neg",
        "c2Norm",
        "c2RotIdentity",
        "c2Skew",
        "c2Sub",
        "c2Support",
        "c2V",
        "c2Witness",
        "c2xIdentity",
        "reverse_collide",
    ];
    let (c_so, rust_so) = common::library_paths();
    let c = exported_symbols(&c_so);
    let r = exported_symbols(&rust_so);
    for name in EXPECTED {
        assert!(c.contains(*name), "C library does not export {name}");
        assert!(r.contains(*name), "Rust library does not export {name}");
    }
    // Guard against the C source growing a new exported function that this
    // suite does not know about.
    let unknown: Vec<&String> = c
        .iter()
        .filter(|n| !EXPECTED.contains(&n.as_str()))
        .collect();
    assert!(
        unknown.is_empty(),
        "C library exports symbols not covered by the test suite: {unknown:?}"
    );
}

#[test]
fn both_libraries_load_and_every_symbol_is_callable() {
    // `apis()` resolves all 38 symbols in both libraries with their expected
    // signatures; a mismatch or a missing export panics during loading.
    let (c, r) = common::apis();
    assert_eq!(c.label, "C");
    assert_eq!(r.label, "Rust");
}
