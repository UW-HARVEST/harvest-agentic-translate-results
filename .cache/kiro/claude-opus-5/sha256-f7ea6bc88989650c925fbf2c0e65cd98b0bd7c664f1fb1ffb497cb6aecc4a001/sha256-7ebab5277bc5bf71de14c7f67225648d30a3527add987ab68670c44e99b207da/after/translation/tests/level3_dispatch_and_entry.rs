//! Level 3: the dispatcher `c2Collided` and the public entry point
//! `circle_collide` declared in `c_src/include/lib.h`.

#![allow(non_snake_case)]

mod common;

use common::*;
use std::ffi::{c_int, c_void};

macro_rules! check_i {
    ($what:expr, $c:expr, $rs:expr, $($ctx:tt)*) => {{
        let (c, rs) = ($c, $rs);
        if c != rs {
            assert_int($what, &format!($($ctx)*), c, rs);
        }
    }};
}

fn as_void<T>(t: &T) -> *const c_void {
    (t as *const T).cast()
}

#[test]
fn collided_circle_dispatch() {
    let p = pair();
    let mut rng = Rng::new();
    for i in 0..600_000u32 {
        let A = c2Circle {
            p: rng.v(150.0),
            r: rng.range(60.0),
        };
        let B = c2Circle {
            p: rng.v(150.0),
            r: rng.range(60.0),
        };
        check_i!(
            "c2Collided/CIRCLE",
            unsafe { (p.c.c2Collided)(as_void(&A), as_void(&B), C2_TYPE_CIRCLE) },
            unsafe { (p.rs.c2Collided)(as_void(&A), as_void(&B), C2_TYPE_CIRCLE) },
            "iter {i}: {A:?} vs {B:?}"
        );
        // The dispatcher must agree with the predicate it forwards to.
        check_i!(
            "c2Collided/CIRCLE vs direct",
            unsafe { (p.c.c2CircletoCircle)(A, B) },
            unsafe { (p.rs.c2Collided)(as_void(&A), as_void(&B), C2_TYPE_CIRCLE) },
            "iter {i}: {A:?} vs {B:?}"
        );
    }
}

#[test]
fn collided_aabb_dispatch() {
    let p = pair();
    let mut rng = Rng::new();
    for i in 0..600_000u32 {
        let A = c2Circle {
            p: rng.v(120.0),
            r: rng.range(40.0),
        };
        let B = c2AABB {
            min: rng.v(60.0),
            max: rng.v(60.0),
        };
        check_i!(
            "c2Collided/AABB",
            unsafe { (p.c.c2Collided)(as_void(&A), as_void(&B), C2_TYPE_AABB) },
            unsafe { (p.rs.c2Collided)(as_void(&A), as_void(&B), C2_TYPE_AABB) },
            "iter {i}: {A:?} vs {B:?}"
        );
        check_i!(
            "c2Collided/AABB vs direct",
            unsafe { (p.c.c2CircletoAABB)(A, B) },
            unsafe { (p.rs.c2Collided)(as_void(&A), as_void(&B), C2_TYPE_AABB) },
            "iter {i}: {A:?} vs {B:?}"
        );
    }
}

#[test]
fn collided_capsule_dispatch() {
    let p = pair();
    let mut rng = Rng::new();
    for i in 0..600_000u32 {
        let A = c2Circle {
            p: rng.v(150.0),
            r: rng.range(30.0),
        };
        let B = c2Capsule {
            a: rng.v(80.0),
            b: rng.v(80.0),
            r: rng.range(20.0),
        };
        check_i!(
            "c2Collided/CAPSULE",
            unsafe { (p.c.c2Collided)(as_void(&A), as_void(&B), C2_TYPE_CAPSULE) },
            unsafe { (p.rs.c2Collided)(as_void(&A), as_void(&B), C2_TYPE_CAPSULE) },
            "iter {i}: {A:?} vs {B:?}"
        );
        check_i!(
            "c2Collided/CAPSULE vs direct",
            unsafe { (p.c.c2CircletoCapsule)(A, B) },
            unsafe { (p.rs.c2Collided)(as_void(&A), as_void(&B), C2_TYPE_CAPSULE) },
            "iter {i}: {A:?} vs {B:?}"
        );
    }
}

#[test]
fn collided_nonfinite_payloads() {
    let p = pair();
    let mut rng = Rng::new();
    for i in 0..600_000u32 {
        let A = c2Circle {
            p: rng.any_v(),
            r: rng.any_f32(),
        };
        let circle = c2Circle {
            p: rng.any_v(),
            r: rng.any_f32(),
        };
        let aabb = c2AABB {
            min: rng.any_v(),
            max: rng.any_v(),
        };
        let capsule = c2Capsule {
            a: rng.any_v(),
            b: rng.any_v(),
            r: rng.any_f32(),
        };
        check_i!(
            "c2Collided/CIRCLE",
            unsafe { (p.c.c2Collided)(as_void(&A), as_void(&circle), C2_TYPE_CIRCLE) },
            unsafe { (p.rs.c2Collided)(as_void(&A), as_void(&circle), C2_TYPE_CIRCLE) },
            "bit iter {i}: {A:?} vs {circle:?}"
        );
        check_i!(
            "c2Collided/AABB",
            unsafe { (p.c.c2Collided)(as_void(&A), as_void(&aabb), C2_TYPE_AABB) },
            unsafe { (p.rs.c2Collided)(as_void(&A), as_void(&aabb), C2_TYPE_AABB) },
            "bit iter {i}: {A:?} vs {aabb:?}"
        );
        check_i!(
            "c2Collided/CAPSULE",
            unsafe { (p.c.c2Collided)(as_void(&A), as_void(&capsule), C2_TYPE_CAPSULE) },
            unsafe { (p.rs.c2Collided)(as_void(&A), as_void(&capsule), C2_TYPE_CAPSULE) },
            "bit iter {i}: {A:?} vs {capsule:?}"
        );
    }
}

/// Every `typeB` outside the enum must take the `default: return 0` arm.
#[test]
fn collided_unknown_type_returns_zero() {
    let p = pair();
    let A = c2Circle {
        p: c2v { x: 1.0, y: 2.0 },
        r: 3.0,
    };
    // Oversized backing store so the C side can never read out of bounds no
    // matter which arm it takes.
    let payload: [f32; 8] = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
    for t in [
        3i32,
        4,
        -1,
        -2,
        100,
        255,
        256,
        0x0100_0000,
        c_int::MAX,
        c_int::MIN,
    ] {
        let c = unsafe { (p.c.c2Collided)(as_void(&A), payload.as_ptr().cast(), t) };
        let rs = unsafe { (p.rs.c2Collided)(as_void(&A), payload.as_ptr().cast(), t) };
        check_i!("c2Collided/default", c, rs, "typeB = {t}");
        assert_eq!(c, 0, "C default arm should return 0 for typeB = {t}");
    }
}

/// `A` is reinterpreted as a `c2Circle` regardless of `typeB`; feed raw bytes so
/// both sides must perform the same unchecked reinterpretation.
#[test]
fn collided_raw_byte_reinterpretation() {
    let p = pair();
    let mut rng = Rng::new();
    for i in 0..300_000u32 {
        // 8 floats is enough backing store for any of the three B layouts.
        let a_words: [u32; 4] = [
            rng.next_u32(),
            rng.next_u32(),
            rng.next_u32(),
            rng.next_u32(),
        ];
        let b_words: [u32; 8] = [
            rng.next_u32(),
            rng.next_u32(),
            rng.next_u32(),
            rng.next_u32(),
            rng.next_u32(),
            rng.next_u32(),
            rng.next_u32(),
            rng.next_u32(),
        ];
        for t in [C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_CAPSULE] {
            check_i!(
                "c2Collided/raw",
                unsafe { (p.c.c2Collided)(a_words.as_ptr().cast(), b_words.as_ptr().cast(), t) },
                unsafe { (p.rs.c2Collided)(a_words.as_ptr().cast(), b_words.as_ptr().cast(), t) },
                "raw iter {i} typeB={t}: A={a_words:08x?} B={b_words:08x?}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// circle_collide — the one function in the public header
// ---------------------------------------------------------------------------

#[test]
fn circle_collide_scalar_corpus() {
    let p = pair();
    for &x in SCALARS {
        for &y in SCALARS {
            for &r in SCALARS {
                check_i!(
                    "circle_collide",
                    unsafe { (p.c.circle_collide)(x, y, r) },
                    unsafe { (p.rs.circle_collide)(x, y, r) },
                    "circle_collide({x:?}, {y:?}, {r:?})"
                );
            }
        }
    }
}

/// Dense grid over the region occupied by the three hard-coded shapes, so every
/// bit of the returned bitmask (1 | 2 | 4) is produced and combined.
#[test]
fn circle_collide_dense_grid() {
    let p = pair();
    let mut seen = [false; 8];
    let mut x = -140.0f32;
    while x <= 60.0 {
        let mut y = -100.0f32;
        while y <= 160.0 {
            for &r in &[0.0f32, 0.25, 1.0, 5.0, 12.0, 25.0, 60.0, 200.0] {
                let c = unsafe { (p.c.circle_collide)(x, y, r) };
                let rs = unsafe { (p.rs.circle_collide)(x, y, r) };
                check_i!("circle_collide", c, rs, "circle_collide({x}, {y}, {r})");
                if (0..8).contains(&c) {
                    seen[c as usize] = true;
                }
            }
            y += 0.5;
        }
        x += 0.5;
    }
    // Sanity check on coverage: the individually-reachable masks must all occur.
    for bit in [0usize, 1, 2, 4] {
        assert!(seen[bit], "grid never produced result mask {bit}");
    }
}

#[test]
fn circle_collide_random() {
    let p = pair();
    let mut rng = Rng::new();
    for i in 0..1_000_000u32 {
        let x = rng.range(200.0);
        let y = rng.range(200.0);
        let r = rng.range(80.0);
        check_i!(
            "circle_collide",
            unsafe { (p.c.circle_collide)(x, y, r) },
            unsafe { (p.rs.circle_collide)(x, y, r) },
            "iter {i}: circle_collide({x:?}, {y:?}, {r:?})"
        );
    }
    for i in 0..1_000_000u32 {
        let x = rng.any_f32();
        let y = rng.any_f32();
        let r = rng.any_f32();
        check_i!(
            "circle_collide",
            unsafe { (p.c.circle_collide)(x, y, r) },
            unsafe { (p.rs.circle_collide)(x, y, r) },
            "bit iter {i}: circle_collide({x:?}, {y:?}, {r:?})"
        );
    }
}

/// One-ULP walks around the exact tangency radii/positions of each hard-coded
/// shape, where the strict `<` comparisons flip.
#[test]
fn circle_collide_tangency_ulps() {
    let p = pair();
    let anchors: &[(f32, f32, f32)] = &[
        // circle at (-70, 0) r = 20
        (-50.0, 0.0, 0.0),
        (-90.0, 0.0, 0.0),
        (-70.0, 20.0, 0.0),
        (0.0, 0.0, 70.0),
        (-70.0, 0.0, 20.0),
        // aabb [-40,-40] .. [-15,-15]
        (-15.0, -15.0, 0.0),
        (-40.0, -40.0, 0.0),
        (0.0, 0.0, 21.213_203),
        (-27.5, 0.0, 15.0),
        // capsule (-40,40) -> (-20,100) r = 10
        (-40.0, 40.0, 0.0),
        (-20.0, 100.0, 0.0),
        (-30.0, 70.0, 0.0),
        (0.0, 0.0, 56.568_542),
        (-50.0, 40.0, 10.0),
    ];
    for &(ax, ay, ar) in anchors {
        for dx in -4i64..=4 {
            for dy in -4i64..=4 {
                for dr in -4i64..=4 {
                    let x = f32::from_bits((ax.to_bits() as i64 + dx) as u32);
                    let y = f32::from_bits((ay.to_bits() as i64 + dy) as u32);
                    let r = f32::from_bits((ar.to_bits() as i64 + dr) as u32);
                    check_i!(
                        "circle_collide",
                        unsafe { (p.c.circle_collide)(x, y, r) },
                        unsafe { (p.rs.circle_collide)(x, y, r) },
                        "ulp walk around ({ax}, {ay}, {ar}): circle_collide({x:?}, {y:?}, {r:?})"
                    );
                }
            }
        }
    }
}

/// Exhaustive over the low 24 bits of the exponent/mantissa space is not
/// feasible, but a full sweep of every `f32` bit pattern for `r` with a fixed
/// position is: 2^32 is too many, so stride through the space deterministically.
#[test]
fn circle_collide_strided_bit_sweep() {
    let p = pair();
    // 1 << 23 samples spread over the whole u32 space via a large odd stride.
    const STRIDE: u32 = 0x0002_5A17; // odd => hits every residue class
    let mut bits: u32 = 0;
    for i in 0..(1u32 << 23) {
        let r = f32::from_bits(bits);
        let x = f32::from_bits(bits.rotate_left(11));
        let y = f32::from_bits(bits.rotate_left(22));
        check_i!(
            "circle_collide",
            unsafe { (p.c.circle_collide)(x, y, r) },
            unsafe { (p.rs.circle_collide)(x, y, r) },
            "sweep {i} bits=0x{bits:08x}: circle_collide({x:?}, {y:?}, {r:?})"
        );
        bits = bits.wrapping_add(STRIDE);
    }
}
