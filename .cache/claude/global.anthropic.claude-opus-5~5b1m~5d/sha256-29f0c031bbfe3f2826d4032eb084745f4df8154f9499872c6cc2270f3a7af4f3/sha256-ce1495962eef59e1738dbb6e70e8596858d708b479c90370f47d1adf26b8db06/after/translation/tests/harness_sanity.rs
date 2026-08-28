//! Harness self-checks — these guard against a *falsely green* suite.
//!
//! Every other test asserts "C output == Rust output", which would also pass if
//! the harness accidentally loaded the same library twice, or called the crate's
//! functions directly instead of going through the Rust `.so`. These tests prove
//! that the two `Api` values really come from two different shared objects and
//! that each function pointer lives inside the corresponding `.so`'s mapping.

#![allow(non_snake_case)]

mod common;

use common::*;

/// The two libraries must be distinct files.
#[test]
fn loads_two_distinct_shared_objects() {
    let (c, r) = both();
    assert_ne!(c.path, r.path, "C and Rust .so paths are identical");
    assert!(c.path.exists() && r.path.exists());
    let rname = r.path.file_name().unwrap().to_string_lossy().to_string();
    assert_eq!(
        rname, "libcollided_lib.so",
        "the Rust side must be the crate's cdylib, got {rname}"
    );
    println!("C   .so: {}", c.path.display());
    println!("Rust.so: {}", r.path.display());
}

/// Corresponding symbols must resolve to *different* addresses; identical
/// addresses would mean both handles point at the same library.
#[test]
fn c_and_rust_symbols_are_different_addresses() {
    let (c, r) = both();
    let cs: [(&str, usize); 10] = [
        ("c2V", c.c2V as usize),
        ("c2Maxv", c.c2Maxv as usize),
        ("c2Minv", c.c2Minv as usize),
        ("c2Clampv", c.c2Clampv as usize),
        ("c2Sub", c.c2Sub as usize),
        ("c2Dot", c.c2Dot as usize),
        ("c2CircletoCircle", c.c2CircletoCircle as usize),
        ("c2CircletoAABB", c.c2CircletoAABB as usize),
        ("c2AABBtoAABB", c.c2AABBtoAABB as usize),
        ("collided", c.collided as usize),
    ];
    let rs: [(&str, usize); 10] = [
        ("c2V", r.c2V as usize),
        ("c2Maxv", r.c2Maxv as usize),
        ("c2Minv", r.c2Minv as usize),
        ("c2Clampv", r.c2Clampv as usize),
        ("c2Sub", r.c2Sub as usize),
        ("c2Dot", r.c2Dot as usize),
        ("c2CircletoCircle", r.c2CircletoCircle as usize),
        ("c2CircletoAABB", r.c2CircletoAABB as usize),
        ("c2AABBtoAABB", r.c2AABBtoAABB as usize),
        ("collided", r.collided as usize),
    ];
    for ((name, ca), (_, ra)) in cs.iter().zip(rs.iter()) {
        assert_ne!(ca, ra, "{name} resolved to the same address in both libraries");
        assert_ne!(*ca, 0);
        assert_ne!(*ra, 0);
    }
}

/// Each function pointer must lie inside the mapping of its own `.so`, proving
/// the calls really cross the FFI boundary into the intended library (and that
/// nothing is being statically linked or short-circuited).
#[cfg(target_os = "linux")]
#[test]
fn symbols_live_inside_their_own_shared_object_mapping() {
    fn ranges_for(substr: &str) -> Vec<(usize, usize)> {
        let maps = std::fs::read_to_string("/proc/self/maps").expect("read /proc/self/maps");
        maps.lines()
            .filter(|l| l.contains(substr))
            .filter_map(|l| {
                let range = l.split_whitespace().next()?;
                let (a, b) = range.split_once('-')?;
                Some((usize::from_str_radix(a, 16).ok()?, usize::from_str_radix(b, 16).ok()?))
            })
            .collect()
    }
    fn inside(addr: usize, ranges: &[(usize, usize)]) -> bool {
        ranges.iter().any(|&(lo, hi)| addr >= lo && addr < hi)
    }

    let (c, r) = both();
    let c_name = c.path.file_name().unwrap().to_string_lossy().to_string();
    let c_ranges = ranges_for(&c_name);
    let r_ranges = ranges_for("libcollided_lib.so");
    assert!(!c_ranges.is_empty(), "no mapping found for {c_name}");
    assert!(!r_ranges.is_empty(), "no mapping found for libcollided_lib.so");

    for (name, addr) in [
        ("c2V", c.c2V as usize),
        ("c2Dot", c.c2Dot as usize),
        ("collided", c.collided as usize),
        ("c2AABBtoAABB", c.c2AABBtoAABB as usize),
    ] {
        assert!(inside(addr, &c_ranges), "C {name} at {addr:#x} is outside {c_name}: {c_ranges:x?}");
        assert!(!inside(addr, &r_ranges), "C {name} unexpectedly inside the Rust .so");
    }
    for (name, addr) in [
        ("c2V", r.c2V as usize),
        ("c2Dot", r.c2Dot as usize),
        ("collided", r.collided as usize),
        ("c2AABBtoAABB", r.c2AABBtoAABB as usize),
    ] {
        assert!(inside(addr, &r_ranges), "Rust {name} at {addr:#x} is outside libcollided_lib.so");
        assert!(!inside(addr, &c_ranges), "Rust {name} unexpectedly inside the C .so");
    }
}

/// Both libraries must actually compute something: a suite where every call
/// returned a constant would compare equal and prove nothing.
#[test]
fn both_libraries_produce_varying_results() {
    let (c, r) = both();
    let mut rng = Rng::new(0xFEED);
    let (mut zeros, mut ones) = (0usize, 0usize);
    let mut distinct_dots = std::collections::HashSet::new();
    for _ in 0..500 {
        let (A, B) = (rng.c_small(), rng.c_small());
        let cv = unsafe { (c.c2CircletoCircle)(A, B) };
        let rv = unsafe { (r.c2CircletoCircle)(A, B) };
        assert_eq!(cv, rv);
        if cv == 0 { zeros += 1 } else { ones += 1 }
        distinct_dots.insert(fb(unsafe { (c.c2Dot)(A.p, B.p) }));
        distinct_dots.insert(fb(unsafe { (r.c2Dot)(A.p, B.p) }));
    }
    assert!(zeros > 10 && ones > 10, "predicate is nearly constant: {zeros}/{ones}");
    assert!(distinct_dots.len() > 50, "c2Dot looks constant: {} values", distinct_dots.len());
}
