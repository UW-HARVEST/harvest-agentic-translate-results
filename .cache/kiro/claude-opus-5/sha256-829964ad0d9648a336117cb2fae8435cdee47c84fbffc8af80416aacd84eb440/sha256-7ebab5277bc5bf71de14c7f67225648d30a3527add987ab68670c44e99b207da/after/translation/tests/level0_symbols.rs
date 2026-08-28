//! Step 8: dynamic-symbol parity between the C reference `.so` and the Rust
//! `cdylib`. Every symbol the C library exports must be exported by the Rust
//! library under the exact same name, and must be resolvable with `dlsym`.

mod common;
use common::*;

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

fn dynamic_symbols(path: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(path)
        .output()
        .expect("`nm` must be available to compare exported symbols");
    assert!(
        out.status.success(),
        "nm -D {} failed: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8_lossy(&out.stdout);
    let mut set = BTreeSet::new();
    for line in text.lines() {
        let mut it = line.split_whitespace();
        let (a, b, c) = (it.next(), it.next(), it.next());
        // "<addr> <type> <name>" for defined symbols.
        if let (Some(_), Some(ty), Some(name)) = (a, b, c) {
            // Only global/weak text & data symbols, which is what a caller can bind to.
            if matches!(ty, "T" | "t" | "W" | "D" | "B" | "R" | "i") {
                set.insert(name.to_string());
            }
        }
    }
    set
}

/// The complete list of externally visible functions in c_src/src/lib.c. Kept
/// explicit so a symbol silently disappearing from *both* libraries still fails.
const EXPECTED: &[&str] = &[
    "c22",
    "c23",
    "c2Add",
    "c2BBVerts",
    "c2CCW90",
    "c2Clampv",
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
    "gjk_cache",
];

#[test]
fn rust_exports_every_c_symbol() {
    let (c_path, rs_path) = library_paths();
    let c_syms = dynamic_symbols(&c_path);
    let rs_syms = dynamic_symbols(&rs_path);

    let missing: Vec<&String> = c_syms.difference(&rs_syms).collect();
    assert!(
        missing.is_empty(),
        "the Rust cdylib is missing {} symbol(s) exported by {}: {:?}",
        missing.len(),
        c_path.display(),
        missing
    );

    for want in EXPECTED {
        assert!(
            c_syms.contains(*want),
            "C library unexpectedly does not export `{want}`"
        );
        assert!(
            rs_syms.contains(*want),
            "Rust library does not export `{want}`"
        );
    }
}

#[test]
fn every_c_symbol_is_dlsym_resolvable_in_rust() {
    let p = load();
    let (c_path, _) = library_paths();
    for name in dynamic_symbols(&c_path) {
        // Skip compiler/runtime bookkeeping symbols that are not part of the API.
        if name.starts_with('_') {
            continue;
        }
        let _: libloading::Symbol<*const ()> = p.rs.get(&name);
        let _: libloading::Symbol<*const ()> = p.c.get(&name);
    }
}
