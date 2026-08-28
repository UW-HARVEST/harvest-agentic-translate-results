//! Step 8: every symbol the C `.so` exports must also be exported by the
//! Rust `.so`, under the exact same name — and must be *loadable*, not just
//! present in the symbol table.

mod common;

use common::*;
use std::process::Command;

/// Names of every non-static function in `c_src/src/lib.c`, as a cross-check
/// against whatever `nm` reports (guards against the C build silently
/// dropping something).
const EXPECTED: &[&str] = &[
    "aabb",
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
    "c2Witness",
    "c2xIdentity",
    "c2V",
];

/// Dynamic-table function symbols defined by a shared object.
fn dynamic_defined_symbols(path: &std::path::Path) -> Vec<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(path)
        .output()
        .expect("failed to run nm");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8_lossy(&out.stdout);
    let mut syms: Vec<String> = text
        .lines()
        .filter_map(|line| {
            let mut it = line.split_whitespace();
            let _addr = it.next()?;
            let kind = it.next()?;
            let name = it.next()?;
            // T/t = text (function), D/d/B/b/R/r = data.
            if matches!(kind, "T" | "t" | "D" | "d" | "B" | "b" | "R" | "r") {
                Some(name.to_string())
            } else {
                None
            }
        })
        .collect();
    syms.sort();
    syms.dedup();
    syms
}

/// Symbols the C toolchain injects into every shared object; not part of the
/// translated surface.
fn is_toolchain_symbol(name: &str) -> bool {
    matches!(
        name,
        "_init"
            | "_fini"
            | "__bss_start"
            | "_edata"
            | "_end"
            | "__gmon_start__"
            | "_ITM_registerTMCloneTable"
            | "_ITM_deregisterTMCloneTable"
            | "__cxa_finalize"
    ) || name.starts_with("__odr_asan")
}

#[test]
fn t_c_so_exports_expected_set() {
    let c_syms: Vec<String> = dynamic_defined_symbols(&c_so_path())
        .into_iter()
        .filter(|s| !is_toolchain_symbol(s))
        .collect();
    let mut expected: Vec<String> = EXPECTED.iter().map(|s| s.to_string()).collect();
    expected.sort();
    assert_eq!(
        c_syms, expected,
        "the C .so's export set is not what the source implies; \
         update EXPECTED (or the translation) to match"
    );
}

#[test]
fn t_rust_so_exports_every_c_symbol() {
    let c_syms = dynamic_defined_symbols(&c_so_path());
    let r_syms = dynamic_defined_symbols(&rust_so_path());

    let missing: Vec<&String> = c_syms
        .iter()
        .filter(|s| !is_toolchain_symbol(s))
        .filter(|s| !r_syms.contains(s))
        .collect();

    assert!(
        missing.is_empty(),
        "Rust .so is missing {} symbol(s) exported by the C .so: {:?}",
        missing.len(),
        missing
    );
}

/// Symbol-table presence is not enough — each name must resolve through
/// `dlsym`, which is how a real caller reaches it.
#[test]
fn t_every_symbol_is_dlsym_resolvable() {
    let l = libs();
    for name in EXPECTED {
        let mut bytes = name.as_bytes().to_vec();
        bytes.push(0);
        let _: (libloading::Symbol<*const ()>, libloading::Symbol<*const ()>) = l.sym(&bytes);
    }
}
