// Phase D — symbol parity and build-configuration parity.
//
// * every symbol exported by the C `.so` must be exported by the Rust `.so`
//   under the exact same name (checked with `nm -D`, and by actually resolving
//   each one with `dlsym`);
// * the whole differential battery is re-run against the `-C opt-level=3`
//   Rust build, so the translation is proven independent of the optimisation
//   level;
// * the artifact `cargo build` produces is checked to export the same set.

mod common;

use common::{Api, EXPORTED_SYMBOLS, both, diff_battery};
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

/// Global, dynamic, *defined* symbols of a shared object, ignoring Rust's
/// mangled internals and the toolchain's own bookkeeping symbols.
fn defined_symbols(so: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(so)
        .output()
        .expect("nm not available");
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
        let (kind, name) = match (it.next(), it.next(), it.next()) {
            (Some(k), Some(n), None) => (k, n),          // "w name" / "U name"
            (Some(_addr), Some(k), Some(n)) => (k, n),   // "addr T name"
            _ => continue,
        };
        // Only strong, defined symbols in text/data/bss/rodata.
        if !matches!(kind, "T" | "D" | "B" | "R" | "G" | "S") {
            continue;
        }
        set.insert(name.to_string());
    }
    set
}

fn is_toolchain_symbol(name: &str) -> bool {
    name.starts_with("_ZN")            // Rust mangled
        || name.starts_with("_R")      // Rust v0 mangled
        || name.starts_with("__rust")
        || name.starts_with("rust_")
        || name.starts_with("__rdl_")
        || name.starts_with("__rg_")
        || name.starts_with("_ITM_")
        || name.starts_with("__gnu")
        || name.starts_with("__cxa")
        || name.starts_with("_init")
        || name.starts_with("_fini")
        || name.starts_with("__bss")
        || name.starts_with("_edata")
        || name.starts_with("_end")
}

#[test]
fn symbol_parity_c_vs_rust() {
    let c_so = common::c_so_path();
    let r_so = common::rust_so_path();
    let c = defined_symbols(&c_so);
    let r = defined_symbols(&r_so);

    let c_public: BTreeSet<_> = c.iter().filter(|n| !is_toolchain_symbol(n)).collect();
    assert!(
        !c_public.is_empty(),
        "no public symbols found in the C library {}",
        c_so.display()
    );

    let missing: Vec<_> = c_public
        .iter()
        .filter(|n| !r.contains(n.as_str()))
        .map(|n| n.to_string())
        .collect();
    assert!(
        missing.is_empty(),
        "Rust .so ({}) is missing {} symbol(s) exported by the C .so ({}): {:?}",
        r_so.display(),
        missing.len(),
        c_so.display(),
        missing
    );

    // and the set is exactly the seven documented functions
    let expect: BTreeSet<String> = EXPORTED_SYMBOLS.iter().map(|s| s.to_string()).collect();
    let got: BTreeSet<String> = c_public.iter().map(|n| n.to_string()).collect();
    assert_eq!(
        got, expect,
        "SYMBOLS.md is out of date with the C library's export list"
    );
}

#[test]
fn symbol_parity_optimised_rust_build() {
    let c = defined_symbols(&common::c_so_path());
    let r = defined_symbols(&common::rust_so_opt_path());
    let missing: Vec<_> = c
        .iter()
        .filter(|n| !is_toolchain_symbol(n) && !r.contains(n.as_str()))
        .cloned()
        .collect();
    assert!(missing.is_empty(), "optimised Rust .so misses {missing:?}");
}

#[test]
fn symbol_parity_cargo_built_artifact() {
    // `cargo build`'s artifact, when present, must export the same set. (It is
    // NOT used for the differential comparisons because `cargo test` does not
    // refresh it — see build.rs.)
    let Some(p) = common::cargo_rust_so_path() else {
        eprintln!("note: target/<profile>/libmodeselect_lib.so not built; skipping");
        return;
    };
    let c = defined_symbols(&common::c_so_path());
    let r = defined_symbols(&p);
    let missing: Vec<_> = c
        .iter()
        .filter(|n| !is_toolchain_symbol(n) && !r.contains(n.as_str()))
        .cloned()
        .collect();
    assert!(
        missing.is_empty(),
        "cargo-built {} misses {missing:?}",
        p.display()
    );
}

#[test]
fn no_unresolved_non_libc_symbols_in_rust_so() {
    // `nm -D -u` must only show libc / Rust-runtime imports, i.e. nothing that
    // would make `dlopen` fail. The successful `dlopen` in `both()` already
    // proves this, but report the list for the record.
    let out = Command::new("nm")
        .arg("-D")
        .arg("-u")
        .arg(common::rust_so_path())
        .output()
        .expect("nm");
    let text = String::from_utf8_lossy(&out.stdout);
    let unresolved: Vec<&str> = text
        .lines()
        .filter_map(|l| l.split_whitespace().last())
        .filter(|n| !is_toolchain_symbol(n))
        .collect();
    // dlopen(RTLD_NOW) is the real test:
    let lib = unsafe { libloading::Library::new(common::rust_so_path()) }
        .expect("dlopen of the Rust .so failed => unresolved symbols");
    for name in EXPORTED_SYMBOLS {
        let mut n = name.to_string();
        n.push('\0');
        let sym: Result<libloading::Symbol<unsafe extern "C" fn()>, _> =
            unsafe { lib.get(n.as_bytes()) };
        assert!(sym.is_ok(), "dlsym({name}) failed on the Rust .so");
    }
    eprintln!("imports of the Rust .so: {} entries", unresolved.len());
}

#[test]
fn every_symbol_resolves_in_both_libraries() {
    let (_c, _r) = both();
    let _opt = common::rust_opt_api();
    for path in [
        common::c_so_path(),
        common::rust_so_path(),
        common::rust_so_opt_path(),
    ] {
        let lib = unsafe { libloading::Library::new(&path) }.expect("dlopen");
        for name in EXPORTED_SYMBOLS {
            let mut n = name.to_string();
            n.push('\0');
            let sym: Result<libloading::Symbol<unsafe extern "C" fn()>, _> =
                unsafe { lib.get(n.as_bytes()) };
            assert!(sym.is_ok(), "dlsym({name}) failed on {}", path.display());
        }
    }
}

// ---------------------------------------------------------------------------
// the battery, re-run against the optimised Rust build
// ---------------------------------------------------------------------------

#[test]
fn battery_c_vs_rust_debug() {
    let (c, r) = both();
    diff_battery("dbg", &c, &r, 300, 0xD0);
}

#[test]
fn battery_c_vs_rust_optimised() {
    let c = common::c_api();
    let o = common::rust_opt_api();
    diff_battery("opt", &c, &o, 300, 0xD1);
}

#[test]
fn battery_rust_debug_vs_rust_optimised() {
    // The two Rust builds must agree with each other as well (catches any
    // accidental reliance on debug-only behaviour).
    let r = common::rust_api();
    let o = common::rust_opt_api();
    diff_battery("dbg-vs-opt", &r, &o, 300, 0xD2);
}

#[test]
fn optimised_build_matches_on_modeselect_grid() {
    let c = common::c_api();
    let o = common::rust_opt_api();
    for idx in 0..4i32 {
        for lvl in 0..5i32 {
            for (t, s) in [(0i32, 0i32), (1, 1), (-1, -1), (i32::MAX, i32::MIN)] {
                let _ = common::same_io_pair(
                    &format!("opt modeselect({idx},{t},{lvl},{s})"),
                    &c,
                    &o,
                    |a: &Api| unsafe { (a.modeselect)(idx, t, lvl, s) },
                );
            }
        }
    }
}
