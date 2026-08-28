//! ABI surface checks: every dynamic symbol the C `.so` exports must also be
//! exported by the Rust `.so` under the exact same name, and each one must be
//! resolvable through `dlsym` (which is what `libloading` uses).

mod common;

use common::*;
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

/// Dynamic symbols defined (not imported) by a shared object, as reported by
/// `nm -D --defined-only`, minus the toolchain/runtime boilerplate that is not
/// part of the library's own API.
fn exported_symbols(so: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(so)
        .output()
        .expect("failed to run nm - is binutils installed?");
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
        let (_addr, kind, name) = match (it.next(), it.next(), it.next()) {
            (Some(a), Some(k), Some(n)) => (a, k, n),
            _ => continue,
        };
        // Only real code/data definitions.
        if !matches!(kind, "T" | "t" | "D" | "d" | "B" | "b" | "R" | "r" | "W" | "i") {
            continue;
        }
        // Skip C-runtime / linker / Rust-runtime scaffolding.
        if name.starts_with('_')
            || name.starts_with("rust_")
            || name.contains("@")
            || matches!(
                name,
                "atexit" | "register_tm_clones" | "deregister_tm_clones" | "frame_dummy"
            )
        {
            continue;
        }
        set.insert(name.to_string());
    }
    set
}

#[test]
fn rust_so_exports_every_c_symbol() {
    let c_so = c_lib_path();
    let r_so = rust_lib_path();
    let c_syms = exported_symbols(&c_so);
    let r_syms = exported_symbols(&r_so);

    assert!(
        !c_syms.is_empty(),
        "no symbols parsed from the C .so at {}",
        c_so.display()
    );

    let missing: Vec<_> = c_syms.difference(&r_syms).cloned().collect();
    assert!(
        missing.is_empty(),
        "the Rust .so is missing symbols exported by the C .so: {missing:?}\n\
         C exports:    {c_syms:?}\n\
         Rust exports: {r_syms:?}"
    );
}

/// Guards against silently dropping a function from the export surface: the
/// full documented API of lib.c, spelled out.
#[test]
fn expected_api_is_present_in_both() {
    let expected = [
        "c2V",
        "c2Maxv",
        "c2Minv",
        "c2Clampv",
        "c2Sub",
        "c2Dot",
        "c2CircletoCircle",
        "c2CircletoAABB",
        "c2AABBtoAABB",
        "collided",
    ];
    for so in [c_lib_path(), rust_lib_path()] {
        let syms = exported_symbols(&so);
        for name in expected {
            assert!(
                syms.contains(name),
                "{} does not export {name}",
                so.display()
            );
        }
    }
}

/// Every expected symbol must be reachable via `dlsym` in both libraries;
/// `Impl::load` panics if any lookup fails.
#[test]
fn all_symbols_resolve_via_dlsym() {
    let (c, r) = both();
    assert_eq!(c.name, "C");
    assert_eq!(r.name, "Rust");
}
