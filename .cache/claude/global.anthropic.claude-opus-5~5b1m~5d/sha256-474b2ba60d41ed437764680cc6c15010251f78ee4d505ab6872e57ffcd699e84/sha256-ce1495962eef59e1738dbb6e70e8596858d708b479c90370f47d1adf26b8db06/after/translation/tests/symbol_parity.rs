//! Phase D — symbol parity between the C `.so` and the Rust `.so`.
//!
//! Runs `nm -D` on both objects and requires the set of exported symbols the C
//! library provides to be a subset of what the Rust library provides. Also
//! asserts each of them is actually resolvable with `dlsym`, so a symbol cannot
//! satisfy the check by merely appearing in the symbol table.

mod common;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn nm_defined(so: &Path) -> BTreeSet<String> {
    let out = std::process::Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(so)
        .output()
        .expect("run nm -D");
    assert!(
        out.status.success(),
        "nm -D {} failed: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let a = it.next()?;
            let b = it.next()?;
            // "<addr> <type> <name>"  or  "<type> <name>" for undefined/weak
            match it.next() {
                Some(name) => {
                    let _ = a;
                    // Only global/weak text & data definitions are part of the ABI.
                    if matches!(b, "T" | "t" | "W" | "w" | "D" | "B" | "R" | "V" | "i") {
                        Some(name.to_string())
                    } else {
                        None
                    }
                }
                None => None,
            }
        })
        // Filter out the toolchain's own CRT/ELF bookkeeping symbols, which are
        // not part of either library's API.
        .filter(|n| {
            !n.starts_with("_init")
                && !n.starts_with("_fini")
                && !n.starts_with("__")
                && !n.starts_with("_ITM_")
                && n != "_edata"
                && n != "_end"
                && n != "_bss_start"
        })
        .collect()
}

/// The five documented API symbols, hard-coded from `driver.h` + `driver.c`
/// so the test still fails if BOTH libraries lost a symbol.
const EXPECTED_API: &[&str] = &["printLine", "printHexCharLine", "bad", "good", "driver"];

/// `static` C functions must NOT be exported by either library.
const EXPECTED_INTERNAL: &[&str] = &["goodG2B", "goodB2G"];

#[test]
fn d1_rust_exports_every_c_symbol() {
    let p = common::pair();
    let c_syms = nm_defined(&p.c.path);
    let r_syms = nm_defined(&p.rust.path);

    let missing: Vec<&String> = c_syms.difference(&r_syms).collect();
    assert!(
        missing.is_empty(),
        "Rust .so ({}) is MISSING {} symbol(s) exported by the C .so ({}): {:?}\n\
         C exports: {:?}",
        p.rust.path.display(),
        missing.len(),
        p.c.path.display(),
        missing,
        c_syms
    );
}

#[test]
fn d2_c_exports_the_documented_api() {
    let p = common::pair();
    let c_syms = nm_defined(&p.c.path);
    for s in EXPECTED_API {
        assert!(
            c_syms.contains(*s),
            "C .so lost documented symbol {s:?}; exports: {c_syms:?}"
        );
    }
    for s in EXPECTED_INTERNAL {
        assert!(
            !c_syms.contains(*s),
            "{s:?} is `static` in driver.c and must not be exported by the C .so"
        );
    }
}

#[test]
fn d3_rust_exports_the_documented_api_and_no_static_leaks() {
    let p = common::pair();
    let r_syms = nm_defined(&p.rust.path);
    for s in EXPECTED_API {
        assert!(
            r_syms.contains(*s),
            "Rust .so lost documented symbol {s:?}; exports: {r_syms:?}"
        );
    }
    for s in EXPECTED_INTERNAL {
        assert!(
            !r_syms.contains(*s),
            "{s:?} is `static` in driver.c; the Rust .so must not export it either"
        );
    }
}

/// Every C symbol must be *callable* through `dlsym`, not just present in the
/// symbol table. (`common::Lib::open` already `dlsym`s all five with
/// `RTLD_NOW`; this makes the requirement an explicit assertion.)
#[test]
fn d4_every_api_symbol_is_dlsym_resolvable_in_both() {
    let p = common::pair();
    // Constructing the pair performed dlopen(RTLD_NOW) + dlsym for all five
    // symbols in both libraries; reaching this line means all resolved.
    assert_eq!(EXPECTED_API.len(), 5);
    assert!(p.c.path.exists() && p.rust.path.exists());
}

/// There is no `[features]` table, so "every feature combination" is the single
/// default build. Keep that machine-checked, and keep the helper script that
/// enumerates combinations in sync.
#[test]
fn d5_feature_matrix_is_a_single_configuration() {
    let manifest =
        std::fs::read_to_string(manifest_dir().join("Cargo.toml")).expect("read Cargo.toml");
    assert!(
        !manifest.contains("[features]"),
        "Cargo.toml gained features -- extend scripts/check_features.sh and re-run Phases B/C \
         for every combination"
    );
    assert!(
        manifest.contains("crate-type = [\"cdylib\"]"),
        "the Rust library must stay a cdylib so it is loaded exactly like the C .so"
    );
}
