//! Harness smoke test: both `.so`s load and the whole C symbol table resolves
//! in the Rust `.so` too (Phase A / Phase D symbol parity, enforced at runtime).

mod common;

use common::*;

/// Every symbol `nm -D` reports for the C `.so`.
pub const C_SYMBOLS: [&str; 38] = [
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
    "c2V",
    "c2Witness",
    "c2xIdentity",
];

#[test]
fn both_libraries_load() {
    let l = libs();
    eprintln!("C   .so: {}", l.c_path.display());
    eprintln!("Rust.so: {}", l.rust_path.display());
    assert!(l.c_path.is_file());
    assert!(l.rust_path.is_file());
}

#[test]
fn every_c_symbol_is_exported_by_rust() {
    let l = libs();
    let mut missing = Vec::new();
    for name in C_SYMBOLS {
        let n = format!("{name}\0");
        let c = unsafe { l.c.get::<*const ()>(n.as_bytes()) };
        assert!(c.is_ok(), "C .so does not export {name}");
        if unsafe { l.rust.get::<*const ()>(n.as_bytes()) }.is_err() {
            missing.push(name);
        }
    }
    assert!(missing.is_empty(), "Rust .so is missing symbols: {missing:?}");
}

#[test]
fn aabb_entry_point_agrees() {
    let l = libs();
    let (c, r) = l.pair::<FnAabb>("aabb");
    for &(a, b, cc, d) in &[
        (-80.0f32, -10.0f32, -60.0f32, 10.0f32),
        (0.0, 0.0, 1.0, 1.0),
        (-100.0, -100.0, 100.0, 100.0),
    ] {
        let cv = unsafe { c(a, b, cc, d) };
        let rv = unsafe { r(a, b, cc, d) };
        assert_eq!(cv, rv, "aabb({a},{b},{cc},{d})");
    }
}
