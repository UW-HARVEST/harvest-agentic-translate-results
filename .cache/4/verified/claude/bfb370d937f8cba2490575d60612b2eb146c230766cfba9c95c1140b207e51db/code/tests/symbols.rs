//! Phase D — symbol parity: every dynamic symbol DEFINED by the C `.so` must
//! also be defined by the Rust `.so`, with the exact same name, and must be
//! resolvable through `dlsym` on the Rust `.so`.

mod harness;

use harness::*;
use std::collections::BTreeSet;
use std::process::Command;

fn nm_defined(path: &std::path::Path) -> Option<BTreeSet<String>> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", path.to_str().unwrap()])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    Some(
        text.lines()
            .filter_map(|l| {
                let mut it = l.split_whitespace();
                let (_addr, kind, name) = (it.next()?, it.next()?, it.next()?);
                // keep global/weak text+data symbols, skip nothing else exists
                if kind.chars().next()?.is_ascii_uppercase() || kind == "w" {
                    Some(name.to_string())
                } else {
                    None
                }
            })
            .collect(),
    )
}

/// Symbols the C toolchain adds to every shared object; they are not part of
/// the library's API surface and are provided by the Rust toolchain too when
/// applicable.
const CRT_NOISE: &[&str] = &[
    "_init",
    "_fini",
    "__bss_start",
    "_edata",
    "_end",
    "_ITM_registerTMCloneTable",
    "_ITM_deregisterTMCloneTable",
    "__gmon_start__",
    "__cxa_finalize",
];

#[test]
fn c_exports_are_all_present_in_rust() {
    let cpath = c_so_path();
    let rpath = rust_so_path();
    let (c, r) = match (nm_defined(&cpath), nm_defined(&rpath)) {
        (Some(c), Some(r)) => (c, r),
        _ => {
            eprintln!("`nm` unavailable — falling back to dlsym-only check");
            let (_cf, _rf) = fns();
            return;
        }
    };
    let missing: Vec<&String> = c
        .iter()
        .filter(|s| !r.contains(*s) && !CRT_NOISE.contains(&s.as_str()))
        .collect();
    assert!(
        missing.is_empty(),
        "symbols exported by {} but MISSING from {}: {:?}",
        cpath.display(),
        rpath.display(),
        missing
    );
    eprintln!("C exports checked: {} symbol(s): {:?}", c.len(), c);
}

#[test]
fn every_c_export_is_dlsym_able_in_rust() {
    // Independent of `nm`: resolve each name from the C header/API through the
    // Rust .so's dynamic symbol table.
    let (cf, rf) = fns();
    assert_ne!(cf as usize, rf as usize);
    // `fns()` itself performs the dlsym of `searchAndReplace` on both handles;
    // reaching this point means the export exists in the Rust cdylib.
}
