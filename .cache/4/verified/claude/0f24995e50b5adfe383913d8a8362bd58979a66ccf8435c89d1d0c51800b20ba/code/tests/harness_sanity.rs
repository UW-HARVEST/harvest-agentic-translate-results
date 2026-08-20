//! Meta-test: proves the differential harness is actually differential.
//!
//! Without this, a bug in `common/mod.rs` (e.g. accidentally loading the same
//! `.so` twice, or an `eq_*` helper that never fails) would make every other
//! test vacuously green.

#![allow(non_snake_case)]
#![allow(clippy::useless_format, clippy::manual_range_patterns, clippy::needless_late_init, clippy::excessive_precision, clippy::too_many_arguments, clippy::needless_range_loop)]

#[macro_use]
mod common;

use common::*;
use std::os::raw::c_void;

/// The two libraries must be distinct objects with distinct symbol addresses.
#[test]
fn loads_two_distinct_libraries() {
    let (c, r) = fnpair!("omni_collide", FnOmniCollide);
    assert_ne!(
        c as usize, r as usize,
        "the C and Rust .so resolved to the SAME address -- the harness is not differential"
    );
    let (cv, rv) = fnpair!("c2V", FnV);
    assert_ne!(cv as usize, rv as usize);
}

/// Every one of the 39 C symbols must resolve in BOTH libraries.
#[test]
fn all_39_symbols_resolve_in_both() {
    const NAMES: [&str; 39] = [
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
        "omni_collide",
        "ptr_from_parts",
    ];
    let (clib, rlib) = libs();
    let mut missing_c = Vec::new();
    let mut missing_r = Vec::new();
    for n in NAMES {
        let mut b = n.as_bytes().to_vec();
        b.push(0);
        unsafe {
            if clib.get::<*const c_void>(&b).is_err() {
                missing_c.push(n);
            }
            if rlib.get::<*const c_void>(&b).is_err() {
                missing_r.push(n);
            }
        }
    }
    assert!(missing_c.is_empty(), "missing from C .so: {missing_c:?}");
    assert!(missing_r.is_empty(), "missing from Rust .so: {missing_r:?}");
}

/// `eq_raw` / `eq_f32` / `eq_int` must actually fail on a difference.
#[test]
fn comparison_helpers_detect_divergence() {
    let bad = std::panic::catch_unwind(|| {
        eq_raw("self-check", &c2v { x: 1.0, y: 2.0 }, &c2v { x: 1.0, y: 3.0 });
    });
    assert!(bad.is_err(), "eq_raw failed to detect a difference");

    // -0.0 == 0.0 numerically, but their bits differ: eq_f32 must reject.
    let bad = std::panic::catch_unwind(|| eq_f32("self-check", 0.0, -0.0));
    assert!(bad.is_err(), "eq_f32 failed to detect 0.0 vs -0.0");

    // Two NaNs with different payloads must be rejected too.
    let bad = std::panic::catch_unwind(|| {
        eq_f32(
            "self-check",
            f32::from_bits(0x7FC0_0000),
            f32::from_bits(0x7FC0_0001),
        )
    });
    assert!(bad.is_err(), "eq_f32 failed to detect differing NaN payloads");

    let bad = std::panic::catch_unwind(|| eq_int("self-check", 0, 1));
    assert!(bad.is_err(), "eq_int failed to detect a difference");

    // ... and must pass on identical bits.
    eq_raw("self-check", &c2v { x: 1.0, y: 2.0 }, &c2v { x: 1.0, y: 2.0 });
    eq_f32("self-check", -0.0, -0.0);
    eq_f32(
        "self-check",
        f32::from_bits(0x7FC0_1234),
        f32::from_bits(0x7FC0_1234),
    );
    eq_int("self-check", 7, 7);
}

/// Ground-truth spot check: the C values are what the header/source promise,
/// so a harness that silently compared "nothing to nothing" would be caught.
#[test]
fn c_library_returns_expected_ground_truth() {
    let (c_omni, _) = fnpair!("omni_collide", FnOmniCollide);
    unsafe {
        // two unit circles at the same point -> collide
        assert_eq!(
            c_omni(
                C2_TYPE_CIRCLE,
                0.0,
                0.0,
                1.0,
                0.0,
                0.0,
                C2_TYPE_CIRCLE,
                0.0,
                0.0,
                1.0,
                0.0,
                0.0
            ),
            1
        );
        // ... and far apart -> no collision
        assert_eq!(
            c_omni(
                C2_TYPE_CIRCLE,
                0.0,
                0.0,
                1.0,
                0.0,
                0.0,
                C2_TYPE_CIRCLE,
                100.0,
                0.0,
                1.0,
                0.0,
                0.0
            ),
            0
        );
    }
    let (c_aabb, _) = fnpair!("c2AABBtoAABB", FnAABBtoAABB);
    let unit = c2AABB {
        min: c2v { x: 0.0, y: 0.0 },
        max: c2v { x: 1.0, y: 1.0 },
    };
    let far = c2AABB {
        min: c2v { x: 9.0, y: 9.0 },
        max: c2v { x: 10.0, y: 10.0 },
    };
    assert_eq!(c_aabb(unit, unit), 1);
    assert_eq!(c_aabb(unit, far), 0);
}

/// The `.so` under test must be the one the caller asked for.
#[test]
fn print_loaded_paths() {
    let _ = libs();
}
