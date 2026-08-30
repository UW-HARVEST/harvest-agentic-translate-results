//! Phase D — symbol parity between the C `.so` and the Rust `.so`.
//!
//! Enforces `SYMBOLS.md` as an executable check: the symbol diff must be EMPTY.

mod common;
use common::*;

use std::collections::BTreeSet;
use std::path::Path;

/// Defined, dynamic, global text/data symbols of a shared object.
fn exported(so: &Path) -> BTreeSet<String> {
    let out = std::process::Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(so)
        .output()
        .expect("run nm");
    assert!(out.status.success(), "nm failed on {so:?}");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let _addr = it.next()?;
            let kind = it.next()?;
            let name = it.next()?;
            // Global text / initialised-data / bss symbols only.
            if matches!(kind, "T" | "D" | "B" | "R") {
                Some(name.to_string())
            } else {
                None
            }
        })
        // Ignore linker/runtime bookkeeping the Rust toolchain adds.
        .filter(|n| {
            !matches!(
                n.as_str(),
                "_init" | "_fini" | "__bss_start" | "_edata" | "_end"
            )
        })
        .collect()
}

#[test]
fn every_c_symbol_is_exported_by_rust() {
    let c = exported(&c_so_path());
    let r = exported(&rust_so_path());

    println!("C   exports ({}): {c:?}", c.len());
    println!("Rust exports ({}): {r:?}", r.len());

    // The C surface, from the source: `driver` (declared in driver.h) and
    // `run` (non-static definition => external linkage).
    assert_eq!(
        c,
        ["driver", "run"].iter().map(|s| s.to_string()).collect(),
        "unexpected C export set — SYMBOLS.md needs updating"
    );

    let missing: Vec<_> = c.difference(&r).cloned().collect();
    assert!(
        missing.is_empty(),
        "Rust .so is MISSING {} symbol(s) exported by the C .so: {missing:?}\n\
         Add the #[no_mangle] extern \"C\" wrapper, or translate the missing C module.",
        missing.len()
    );
}

/// The `static` C items must not leak into the Rust dynamic symbol table.
#[test]
fn rust_does_not_export_c_internal_symbols() {
    let r = exported(&rust_so_path());
    for internal in [
        "the_house",
        "THE_HOUSE",
        "add_floor",
        "add_bedrooms",
        "add_floor_to_the_house",
        "print_the_house",
    ] {
        assert!(
            !r.contains(internal),
            "`{internal}` has internal linkage in the C and must not be exported by Rust"
        );
    }
}

/// Both `.so`s must be loadable and every C symbol resolvable via `dlsym`
/// (exact name), which is what an external consumer actually does.
#[test]
fn all_symbols_resolvable_via_dlsym() {
    unsafe {
        let c_lib = libloading::Library::new(c_so_path()).expect("dlopen C");
        let r_lib = libloading::Library::new(rust_so_path()).expect("dlopen Rust");
        for name in [b"run\0".as_ref(), b"driver\0".as_ref()] {
            let pretty = String::from_utf8_lossy(&name[..name.len() - 1]).to_string();
            c_lib
                .get::<unsafe extern "C" fn(std::ffi::c_int)>(name)
                .unwrap_or_else(|e| panic!("C dlsym {pretty} failed: {e}"));
            r_lib
                .get::<unsafe extern "C" fn(std::ffi::c_int)>(name)
                .unwrap_or_else(|e| panic!("Rust dlsym {pretty} failed: {e}"));
        }
    }
}

/// The Rust `.so` must not have unresolved non-libc dependencies: it has to be
/// dlopen-able standalone (already proven above) and its only *library-level*
/// import in common with the C is `printf`.
#[test]
fn rust_so_has_no_unresolved_library_symbols() {
    let out = std::process::Command::new("nm")
        .args(["-D", "--undefined-only"])
        .arg(rust_so_path())
        .output()
        .expect("run nm");
    let text = String::from_utf8_lossy(&out.stdout);

    // Everything undefined must come from libc / the compiler runtime, i.e.
    // carry a GLIBC/GCC version tag or be a known weak/unversioned runtime hook.
    let allowed_unversioned = [
        "_ITM_deregisterTMCloneTable",
        "_ITM_registerTMCloneTable",
        "__gmon_start__",
    ];
    for line in text.lines() {
        let name = match line.split_whitespace().last() {
            Some(n) => n,
            None => continue,
        };
        let base = name.split('@').next().unwrap_or(name);
        let versioned = name.contains("@GLIBC") || name.contains("@GCC");
        assert!(
            versioned || allowed_unversioned.contains(&base),
            "Rust .so has an unresolved non-libc symbol: {name}"
        );
    }
    assert!(text.contains("printf"), "Rust .so must import printf");
}
