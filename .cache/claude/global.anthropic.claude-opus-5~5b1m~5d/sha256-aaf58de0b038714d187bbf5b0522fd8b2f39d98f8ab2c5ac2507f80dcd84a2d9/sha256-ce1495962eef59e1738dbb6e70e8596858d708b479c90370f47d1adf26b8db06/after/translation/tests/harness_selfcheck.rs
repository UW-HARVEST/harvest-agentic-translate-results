//! Guards the guard: proves the differential harness really does detect a
//! divergence.  Without this, "all tests pass" could just mean "the assertions
//! never fire".

mod common;

use common::*;

/// `c2Skew` and `c2CCW90` differ for every non-zero input, so cross-comparing
/// them MUST make `assert_bits_eq!` panic.  If this test's `catch_unwind`
/// observes no panic, every other test in the suite is worthless.
#[test]
fn assert_bits_eq_detects_divergence() {
    let p = load();
    let c_skew: FnV1 = p.c.sym("c2Skew");
    let r_ccw: FnV1 = p.rs.sym("c2CCW90");
    let a = c2v { x: 3.0, y: 7.0 };

    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let res = std::panic::catch_unwind(|| unsafe {
        assert_bits_eq!(c_skew(a), r_ccw(a), "deliberate mismatch");
    });
    std::panic::set_hook(hook);

    assert!(
        res.is_err(),
        "assert_bits_eq! failed to detect C c2Skew({}) != Rust c2CCW90({})",
        v_hex(&a),
        v_hex(&a)
    );
}

/// Same for the scalar macro, and for NaN payloads specifically: a `+qNaN` and
/// a `-qNaN` compare equal under `==` but must NOT compare equal here.
#[test]
fn assert_f32_bits_eq_detects_nan_payload_divergence() {
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let res = std::panic::catch_unwind(|| {
        assert_f32_bits_eq!(
            f32::from_bits(0x7fc0_0000),
            f32::from_bits(0xffc0_0000),
            "deliberate NaN sign mismatch"
        );
    });
    let res2 = std::panic::catch_unwind(|| {
        assert_f32_bits_eq!(0.0f32, -0.0f32, "deliberate signed-zero mismatch");
    });
    std::panic::set_hook(hook);
    assert!(res.is_err(), "NaN sign-bit divergence went undetected");
    assert!(res2.is_err(), "signed-zero divergence went undetected");
}

/// Proves both `.so`s really were loaded from disk (and are different files).
#[test]
fn both_libraries_are_distinct_files_and_export_everything() {
    let p = load();
    // Every symbol from SYMBOLS.md must resolve in BOTH libraries.
    const SYMS: &[&str] = &[
        "c22", "c23", "c2AABBtoAABB", "c2AABBtoCapsule", "c2Add", "c2BBVerts", "c2CCW90",
        "c2CapsuletoCapsule", "c2CircletoAABB", "c2CircletoCapsule", "c2CircletoCircle",
        "c2Clampv", "c2Collided", "c2D", "c2Det2", "c2Div", "c2Dot", "c2GJK",
        "c2GJKSimplexMetric", "c2L", "c2Len", "c2MakeProxy", "c2Maxv", "c2Minv", "c2Mulrv",
        "c2MulrvT", "c2Mulvs", "c2Mulxv", "c2Neg", "c2Norm", "c2RotIdentity", "c2Skew",
        "c2Sub", "c2Support", "c2V", "c2Witness", "c2xIdentity", "capsule",
    ];
    assert_eq!(SYMS.len(), 38, "SYMBOLS.md lists 38 exported symbols");
    for s in SYMS {
        let _c: *const () = p.c.sym(s);
        let _r: *const () = p.rs.sym(s);
    }
}

/// Sanity: the struct layouts the harness uses match the C ABI sizes.
#[test]
fn struct_layouts_match_c() {
    use std::mem::size_of;
    assert_eq!(size_of::<c2v>(), 8);
    assert_eq!(size_of::<c2r>(), 8);
    assert_eq!(size_of::<c2x>(), 16);
    assert_eq!(size_of::<c2Circle>(), 12);
    assert_eq!(size_of::<c2AABB>(), 16);
    assert_eq!(size_of::<c2Capsule>(), 20);
    assert_eq!(size_of::<c2GJKCache>(), 36);
    assert_eq!(size_of::<c2Proxy>(), 72);
    assert_eq!(size_of::<c2sv>(), 36);
    assert_eq!(size_of::<c2Simplex>(), 152);
}
