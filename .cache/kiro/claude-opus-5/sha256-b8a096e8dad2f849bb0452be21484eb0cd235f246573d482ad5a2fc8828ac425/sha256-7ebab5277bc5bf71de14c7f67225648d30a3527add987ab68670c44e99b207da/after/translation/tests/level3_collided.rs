//! Level 3: the public entry point `collided` — type dispatch, the
//! circle/AABB argument swap, out-of-range `C2_TYPE` values and aliasing.

mod common;

use common::*;
use std::ffi::c_void;

/// 16-byte aligned scratch buffer big enough for either shape, so every
/// `typeA`/`typeB` pairing can reinterpret it the way the C code does.
#[repr(C, align(16))]
#[derive(Clone, Copy)]
struct Buf([f32; 4]);

impl Buf {
    fn ptr(&self) -> *const c_void {
        self as *const Buf as *const c_void
    }
}

fn buf_from_circle(c: c2Circle, pad: f32) -> Buf {
    Buf([c.p.x, c.p.y, c.r, pad])
}

fn buf_from_aabb(a: c2AABB) -> Buf {
    Buf([a.min.x, a.min.y, a.max.x, a.max.y])
}

const TYPES: [i32; 9] = [0, 1, 2, 3, -1, 7, 100, i32::MIN, i32::MAX];

#[test]
fn collided_dispatch_all_type_pairs() {
    let (c, r) = both();
    let circle = c2Circle {
        p: c2v { x: 0.5, y: -0.25 },
        r: 1.5,
    };
    let aabb = c2AABB {
        min: c2v { x: -1.0, y: -1.0 },
        max: c2v { x: 1.0, y: 2.0 },
    };
    // Every buffer is valid as either shape, so out-of-range types are safe.
    let bufs = [
        buf_from_circle(circle, 0.0),
        buf_from_aabb(aabb),
        Buf([0.0, 0.0, 0.0, 0.0]),
        Buf([f32::NAN, 1.0, -1.0, 3.0]),
    ];
    for &ta in &TYPES {
        for &tb in &TYPES {
            for a in &bufs {
                for b in &bufs {
                    let rc = unsafe { (c.collided)(a.ptr(), ta, b.ptr(), tb) };
                    let rr = unsafe { (r.collided)(a.ptr(), ta, b.ptr(), tb) };
                    assert_eq!(
                        rc, rr,
                        "collided mismatch for typeA={ta} typeB={tb} A={:?} B={:?}",
                        a.0, b.0
                    );
                }
            }
        }
    }
}

/// circle/circle path must agree with `c2CircletoCircle`.
#[test]
fn collided_circle_circle_random() {
    let (c, r) = both();
    let mut rng = Rng::new(0x000C_10C1);
    for _ in 0..150_000 {
        let ca = c2Circle {
            p: c2v {
                x: rng.coord(),
                y: rng.coord(),
            },
            r: rng.coord(),
        };
        let cb = c2Circle {
            p: c2v {
                x: rng.coord(),
                y: rng.coord(),
            },
            r: rng.coord(),
        };
        let (a, b) = (buf_from_circle(ca, rng.coord()), buf_from_circle(cb, rng.coord()));
        let rc = unsafe { (c.collided)(a.ptr(), C2_TYPE_CIRCLE, b.ptr(), C2_TYPE_CIRCLE) };
        let rr = unsafe { (r.collided)(a.ptr(), C2_TYPE_CIRCLE, b.ptr(), C2_TYPE_CIRCLE) };
        assert_eq!(rc, rr, "collided(circle,circle) mismatch A={ca:?} B={cb:?}");
        // Cross-check against the primitive, both through the C .so.
        assert_eq!(
            rc,
            unsafe { (c.c2CircletoCircle)(ca, cb) },
            "collided disagrees with c2CircletoCircle for A={ca:?} B={cb:?}"
        );
    }
}

#[test]
fn collided_aabb_aabb_random() {
    let (c, r) = both();
    let mut rng = Rng::new(0xBB_AABB);
    for _ in 0..150_000 {
        let aa = c2AABB {
            min: c2v {
                x: rng.coord(),
                y: rng.coord(),
            },
            max: c2v {
                x: rng.coord(),
                y: rng.coord(),
            },
        };
        let ab = c2AABB {
            min: c2v {
                x: rng.coord(),
                y: rng.coord(),
            },
            max: c2v {
                x: rng.coord(),
                y: rng.coord(),
            },
        };
        let (a, b) = (buf_from_aabb(aa), buf_from_aabb(ab));
        assert_eq!(
            unsafe { (c.collided)(a.ptr(), C2_TYPE_AABB, b.ptr(), C2_TYPE_AABB) },
            unsafe { (r.collided)(a.ptr(), C2_TYPE_AABB, b.ptr(), C2_TYPE_AABB) },
            "collided(aabb,aabb) mismatch A={aa:?} B={ab:?}"
        );
    }
}

/// The mixed cases: (circle, aabb) and the swapped (aabb, circle) — note the C
/// code passes `B` as the circle and `A` as the box in the swapped branch.
#[test]
fn collided_mixed_random() {
    let (c, r) = both();
    let mut rng = Rng::new(0x9E37_79B9);
    for _ in 0..150_000 {
        let circ = c2Circle {
            p: c2v {
                x: rng.coord(),
                y: rng.coord(),
            },
            r: rng.coord(),
        };
        let box_ = c2AABB {
            min: c2v {
                x: rng.coord(),
                y: rng.coord(),
            },
            max: c2v {
                x: rng.coord(),
                y: rng.coord(),
            },
        };
        let cb = buf_from_circle(circ, rng.coord());
        let bb = buf_from_aabb(box_);

        // typeA = CIRCLE, typeB = AABB
        assert_eq!(
            unsafe { (c.collided)(cb.ptr(), C2_TYPE_CIRCLE, bb.ptr(), C2_TYPE_AABB) },
            unsafe { (r.collided)(cb.ptr(), C2_TYPE_CIRCLE, bb.ptr(), C2_TYPE_AABB) },
            "collided(circle,aabb) mismatch circle={circ:?} box={box_:?}"
        );
        // typeA = AABB, typeB = CIRCLE (arguments swapped internally)
        assert_eq!(
            unsafe { (c.collided)(bb.ptr(), C2_TYPE_AABB, cb.ptr(), C2_TYPE_CIRCLE) },
            unsafe { (r.collided)(bb.ptr(), C2_TYPE_AABB, cb.ptr(), C2_TYPE_CIRCLE) },
            "collided(aabb,circle) mismatch circle={circ:?} box={box_:?}"
        );
    }
}

/// Reinterpreting a buffer as the "wrong" shape is what the C code does when
/// the caller lies about the type; exercise those crossed readings too.
#[test]
fn collided_crossed_reinterpretation() {
    let (c, r) = both();
    let mut rng = Rng::new(0xC0_5555);
    for _ in 0..150_000 {
        let a = Buf([rng.coord(), rng.coord(), rng.coord(), rng.coord()]);
        let b = Buf([rng.coord(), rng.coord(), rng.coord(), rng.coord()]);
        for (ta, tb) in [
            (C2_TYPE_CIRCLE, C2_TYPE_CIRCLE),
            (C2_TYPE_CIRCLE, C2_TYPE_AABB),
            (C2_TYPE_AABB, C2_TYPE_CIRCLE),
            (C2_TYPE_AABB, C2_TYPE_AABB),
        ] {
            assert_eq!(
                unsafe { (c.collided)(a.ptr(), ta, b.ptr(), tb) },
                unsafe { (r.collided)(a.ptr(), ta, b.ptr(), tb) },
                "collided mismatch typeA={ta} typeB={tb} A={:?} B={:?}",
                a.0,
                b.0
            );
        }
    }
}

/// Same pointer for both operands (self-collision), all type pairs.
#[test]
fn collided_aliased_pointers() {
    let (c, r) = both();
    let mut rng = Rng::new(0xA11A5F);
    for _ in 0..100_000 {
        let a = Buf([rng.coord(), rng.coord(), rng.coord(), rng.coord()]);
        for &ta in &TYPES {
            for &tb in &TYPES {
                assert_eq!(
                    unsafe { (c.collided)(a.ptr(), ta, a.ptr(), tb) },
                    unsafe { (r.collided)(a.ptr(), ta, a.ptr(), tb) },
                    "collided(aliased) mismatch typeA={ta} typeB={tb} A={:?}",
                    a.0
                );
            }
        }
    }
}

/// Non-finite payloads flowing through the dispatcher.
#[test]
fn collided_nonfinite() {
    let (c, r) = both();
    let specials = [
        0.0f32,
        -0.0,
        1.0,
        -1.0,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        f32::from_bits(0x7F80_0001),
        f32::MAX,
        f32::MIN,
        f32::MIN_POSITIVE,
        f32::from_bits(1),
    ];
    let mut rng = Rng::new(0xDEAD);
    let mut bufs = Vec::new();
    for &v0 in &specials {
        for &v1 in &specials {
            bufs.push(Buf([
                v0,
                v1,
                specials[(rng.next_u32() as usize) % specials.len()],
                specials[(rng.next_u32() as usize) % specials.len()],
            ]));
        }
    }
    for a in &bufs {
        for b in &bufs {
            for (ta, tb) in [
                (C2_TYPE_CIRCLE, C2_TYPE_CIRCLE),
                (C2_TYPE_CIRCLE, C2_TYPE_AABB),
                (C2_TYPE_AABB, C2_TYPE_CIRCLE),
                (C2_TYPE_AABB, C2_TYPE_AABB),
            ] {
                assert_eq!(
                    unsafe { (c.collided)(a.ptr(), ta, b.ptr(), tb) },
                    unsafe { (r.collided)(a.ptr(), ta, b.ptr(), tb) },
                    "collided mismatch typeA={ta} typeB={tb} A={:?} B={:?}",
                    a.0,
                    b.0
                );
            }
        }
    }
}

/// Returned `int` must be exactly 0 or 1 in the same places (not merely
/// truthy), since callers may compare against 1.
#[test]
fn collided_return_values_are_identical_ints() {
    let (c, r) = both();
    let mut rng = Rng::new(0x12345);
    let mut seen = [false; 2];
    for _ in 0..50_000 {
        let a = Buf([rng.coord(), rng.coord(), rng.coord(), rng.coord()]);
        let b = Buf([rng.coord(), rng.coord(), rng.coord(), rng.coord()]);
        let rc = unsafe { (c.collided)(a.ptr(), C2_TYPE_AABB, b.ptr(), C2_TYPE_AABB) };
        let rr = unsafe { (r.collided)(a.ptr(), C2_TYPE_AABB, b.ptr(), C2_TYPE_AABB) };
        assert_eq!(rc, rr);
        assert!(rc == 0 || rc == 1, "unexpected C return {rc}");
        seen[rc as usize] = true;
    }
    assert!(seen[0] && seen[1], "test data did not cover both outcomes");
}
