//! Harness self-checks.
//!
//! Both shared libraries export the *same* symbol names, so the suite would be
//! worthless if `dlopen` symbol interposition made one library's internal calls
//! land in the other one (the C `c2Maxv` calls `c2V` through its PLT). These
//! tests assert the two objects are genuinely distinct and that every symbol
//! resolves inside its own library.

#![allow(non_snake_case)]

mod common;
use common::*;

/// Every symbol must resolve to a different address in each library, otherwise
/// we would be comparing a library against itself.
#[test]
fn harness_loads_two_distinct_libraries() {
    let (c, r) = apis();
    let pairs: [(&str, usize, usize); 10] = [
        ("c2V", c.c2V as usize, r.c2V as usize),
        ("c2Maxv", c.c2Maxv as usize, r.c2Maxv as usize),
        ("c2Minv", c.c2Minv as usize, r.c2Minv as usize),
        ("c2Clampv", c.c2Clampv as usize, r.c2Clampv as usize),
        ("c2Sub", c.c2Sub as usize, r.c2Sub as usize),
        ("c2Dot", c.c2Dot as usize, r.c2Dot as usize),
        (
            "c2CircletoCircle",
            c.c2CircletoCircle as usize,
            r.c2CircletoCircle as usize,
        ),
        (
            "c2CircletoAABB",
            c.c2CircletoAABB as usize,
            r.c2CircletoAABB as usize,
        ),
        (
            "c2AABBtoAABB",
            c.c2AABBtoAABB as usize,
            r.c2AABBtoAABB as usize,
        ),
        ("collided", c.collided as usize, r.collided as usize),
    ];
    for (name, ca, ra) in pairs {
        assert_ne!(
            ca, ra,
            "{name} resolved to the SAME address in both libraries — the C and \
             Rust .so were not loaded independently, so every differential \
             assertion in this suite would be vacuous"
        );
        assert_ne!(ca, 0, "{name} null in C lib");
        assert_ne!(ra, 0, "{name} null in Rust lib");
    }
}

/// The C `c2Maxv`/`c2Minv` call `c2V` through the PLT. If that call were
/// interposed by the Rust `c2V`, the two libraries could agree while both being
/// wrong. Verify the C library composes with *its own* `c2V`, using a value
/// where a swapped `c2V` would be detectable (x != y).
#[test]
fn harness_no_cross_library_interposition() {
    let (c, r) = apis();
    let a = C2v { x: 10.0, y: -20.0 };
    let b = C2v { x: -30.0, y: 40.0 };

    // c2Maxv(a, b) must be (max(ax,bx), max(ay,by)) = (10, 40) in BOTH libs.
    let expect = C2v { x: 10.0, y: 40.0 };
    same("C c2Maxv self-consistency", (a, b), (c.c2Maxv)(a, b), expect);
    same("Rust c2Maxv self-consistency", (a, b), (r.c2Maxv)(a, b), expect);

    // c2Minv(a, b) = (-30, -20)
    let expect = C2v { x: -30.0, y: -20.0 };
    same("C c2Minv self-consistency", (a, b), (c.c2Minv)(a, b), expect);
    same("Rust c2Minv self-consistency", (a, b), (r.c2Minv)(a, b), expect);

    // c2V must not transpose its arguments in either library.
    let v = C2v { x: 1.0, y: 2.0 };
    same("C c2V identity", v, (c.c2V)(1.0, 2.0), v);
    same("Rust c2V identity", v, (r.c2V)(1.0, 2.0), v);
}

/// Confirm the loaded C library really is the one built from `c_src` and that
/// the Rust library is a *different* file on disk.
#[test]
fn harness_uses_expected_artifacts() {
    // Simply constructing the pair runs the path resolution + staleness guards.
    let (c, r) = apis();
    assert_eq!(c.name, "C");
    assert_eq!(r.name, "Rust");
}
