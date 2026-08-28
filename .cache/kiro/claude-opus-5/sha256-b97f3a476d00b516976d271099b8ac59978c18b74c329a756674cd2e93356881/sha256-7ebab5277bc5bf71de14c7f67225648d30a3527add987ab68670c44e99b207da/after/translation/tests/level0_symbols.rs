//! Step 8: every symbol the C `.so` exports must be exported by the Rust
//! `.so` under the exact same name.

mod common;
use common::*;

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

/// Dynamic, defined, code/data symbols of a shared object.
fn exported_symbols(so: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(so)
        .output()
        .expect("failed to run nm");
    assert!(
        out.status.success(),
        "nm failed on {:?}: {}",
        so,
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let a = it.next()?;
            let b = it.next()?;
            // "<addr> <type> <name>"; skip undefined / no-address forms.
            let (ty, name) = if let Some(c) = it.next() {
                (b, c)
            } else {
                (a, b)
            };
            if ty.len() == 1 && ty.chars().all(|c| c.is_ascii_alphabetic()) {
                Some(name.to_string())
            } else {
                None
            }
        })
        .collect()
}

#[test]
fn every_c_symbol_is_exported_by_rust() {
    let c = exported_symbols(&c_lib_path());
    let r = exported_symbols(&rust_lib_path());

    assert!(!c.is_empty(), "no symbols read from the C .so");

    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "Rust .so ({:?}) is missing {} symbol(s) exported by the C .so ({:?}): {:?}",
        rust_lib_path(),
        missing.len(),
        c_lib_path(),
        missing
    );

    // Sanity check that we are actually looking at the right libraries: the
    // whole public API of lib.c must be in the intersection.
    for name in [
        "c2V", "c2Dot", "c2Len", "c2Add", "c2Sub", "c2Mulvs", "c2Div", "c2Norm",
        "c2Minv", "c2Maxv", "c2Skew", "c2Absv", "c2CCW90", "c2MulmvT",
        "c2AABBtoAABB", "c2AABBtoPoint", "c2CircleToPoint", "c2RaytoCircle",
        "c2RaytoAABB", "c2RaytoCapsule", "c2CastRay", "spec_ray",
    ] {
        assert!(c.contains(name), "C .so unexpectedly lacks {name}");
        assert!(r.contains(name), "Rust .so lacks {name}");
    }
}

/// Both libraries must agree on struct layout, otherwise every other test
/// would be comparing garbage.
#[test]
fn abi_layout_matches_c() {
    use std::mem::{align_of, size_of};
    assert_eq!(size_of::<c2v>(), 8);
    assert_eq!(align_of::<c2v>(), 4);
    assert_eq!(size_of::<c2Raycast>(), 12);
    assert_eq!(size_of::<c2Circle>(), 12);
    assert_eq!(size_of::<c2AABB>(), 16);
    assert_eq!(size_of::<c2Capsule>(), 20);
    assert_eq!(size_of::<c2Ray>(), 20);
    assert_eq!(size_of::<c2m>(), 16);
}
