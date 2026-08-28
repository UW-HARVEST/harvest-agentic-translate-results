//! Every symbol the C `.so` exports must also be exported by the Rust `cdylib`,
//! under exactly the same name.
//!
//! `c2Clip`, `c2SidePlanes`, `c2SidePlanesFromPoly`, `c2KeepDeep` and `c2Incident`
//! are `static` in the C and therefore correctly absent from both.

mod common;
use common::*;
use std::collections::BTreeSet;
use std::process::Command;

fn exported(path: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(path)
        .output()
        .expect("run nm");
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
            // Exported code symbols only; ignore data/BSS and linker-generated ones.
            if (kind == "T" || kind == "W") && !name.starts_with('_') {
                Some(name.to_string())
            } else {
                None
            }
        })
        .collect()
}

#[test]
fn rust_exports_every_c_symbol() {
    let (c_so, rust_so) = so_paths();
    let c = exported(&c_so);
    let r = exported(&rust_so);
    assert!(!c.is_empty(), "no symbols found in {}", c_so.display());

    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing {} symbol(s) exported by the C .so: {:?}",
        missing.len(),
        missing
    );

    // Sanity: the public API and every non-static helper are present.
    for name in [
        "omni_manifold",
        "ptr_from_parts",
        "c2Collide",
        "c2GJK",
        "c22",
        "c23",
        "c2CapsuletoPolyManifold",
    ] {
        assert!(c.contains(name), "C should export {name}");
        assert!(r.contains(name), "Rust should export {name}");
    }

    // The C's `static` helpers must not have leaked into either export table.
    for name in [
        "c2Clip",
        "c2SidePlanes",
        "c2SidePlanesFromPoly",
        "c2KeepDeep",
        "c2Incident",
    ] {
        assert!(!c.contains(name), "C unexpectedly exports static {name}");
        assert!(!r.contains(name), "Rust must not export static {name}");
    }
}
