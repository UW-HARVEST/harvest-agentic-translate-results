//! Step 8: every symbol the C `.so` exports, the Rust `.so` must export too,
//! under exactly the same name — including the two functions that are absent
//! from `lib.h` (`get_os_arch`, `w_regexec`) but still have external linkage.

mod common;

use common::*;

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

/// Dynamic symbols defined (not imported) by an ELF shared object.
fn exported_symbols(so: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(so)
        .output()
        .expect("`nm` must be available to compare exported symbols");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );

    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|line| {
            let mut it = line.split_whitespace();
            let (_addr, kind, name) = (it.next()?, it.next()?, it.next()?);
            // Global code/data definitions only; skip local and weak-object
            // bookkeeping symbols that are an artefact of the toolchain.
            matches!(kind, "T" | "D" | "B" | "R").then(|| name.to_string())
        })
        .collect()
}

#[test]
fn rust_so_exports_every_c_symbol() {
    let c = c_so();
    let r = rust_so();
    let c_syms = exported_symbols(&c);
    let r_syms = exported_symbols(&r);

    assert!(
        !c_syms.is_empty(),
        "no exported symbols found in {}",
        c.display()
    );

    let missing: Vec<&String> = c_syms.difference(&r_syms).collect();
    assert!(
        missing.is_empty(),
        "Rust .so ({}) is missing symbols exported by the C .so ({}): {missing:?}",
        r.display(),
        c.display(),
    );
}

#[test]
fn known_public_api_is_exported_by_both() {
    // Guards against the C side silently losing an export and the comparison
    // above passing vacuously.
    let c_syms = exported_symbols(&c_so());
    let r_syms = exported_symbols(&rust_so());

    for name in ["parse_uname_string", "get_os_arch", "w_regexec"] {
        assert!(c_syms.contains(name), "C .so does not export {name}");
        assert!(r_syms.contains(name), "Rust .so does not export {name}");
    }
}

#[test]
fn all_c_symbols_are_dlsym_resolvable_in_both() {
    // nm parity is necessary but not sufficient: check each name actually
    // resolves through dlsym in both libraries.
    let c_syms = exported_symbols(&c_so());
    let (c, rust) = load_both();

    for name in &c_syms {
        let mut key = name.clone().into_bytes();
        key.push(0);
        assert!(
            c.has_symbol(&key),
            "dlsym failed for {name} in the C library"
        );
        assert!(
            rust.has_symbol(&key),
            "dlsym failed for {name} in the Rust library"
        );
    }
}
