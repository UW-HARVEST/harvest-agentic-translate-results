//! Phase B, Tier 5 + Tier 6: `CONFIGS.md` rows 72-79.
//!
//! The boolean helpers, the `c2Collided` dispatcher (including its three
//! argument-swapping branches) and the `aabb` entry point from `include/lib.h`.

mod common;

use common::*;
use std::ffi::{c_int, c_void};

const N: usize = 5000;

// ---------------------------------------------------------------------------
// Row 72 — c2AABBtoAABB
// ---------------------------------------------------------------------------

#[test]
fn row72_c2AABBtoAABB() {
    let l = libs();
    let (c, r) = (l.c.sym::<FnAABBtoAABB>("c2AABBtoAABB"), l.rs.sym::<FnAABBtoAABB>("c2AABBtoAABB"));
    let mut g = Rng::new(0x48);
    let mut rep = Report::new();
    let mut probe = |rep: &mut Report, A: c2AABB, B: c2AABB, tag: &str| {
        let (x, y) = (c(A, B), r(A, B));
        rep.check(x == y, || {
            format!(
                "c2AABBtoAABB[{tag}] A(min={} max={}) B(min={} max={}): C={x} Rust={y}",
                show_v(A.min), show_v(A.max), show_v(B.min), show_v(B.max)
            )
        });
    };
    for _ in 0..N {
        probe(&mut rep, g.aabb(), g.aabb(), "random");
    }
    // Every separating / touching / nesting configuration, exactly.
    let unit = c2AABB { min: c2v { x: 0.0, y: 0.0 }, max: c2v { x: 1.0, y: 1.0 } };
    for (dx, dy) in [
        (0.0, 0.0),   // identical
        (0.5, 0.5),   // partial overlap
        (1.0, 0.0),   // edge-touching in x
        (0.0, 1.0),   // edge-touching in y
        (1.0, 1.0),   // corner-touching
        (1.0000001, 0.0),
        (-1.0, 0.0),
        (2.0, 0.0),   // separated in x
        (0.0, -2.0),  // separated in y
    ] {
        let b = c2AABB {
            min: c2v { x: unit.min.x + dx, y: unit.min.y + dy },
            max: c2v { x: unit.max.x + dx, y: unit.max.y + dy },
        };
        probe(&mut rep, unit, b, "grid");
        probe(&mut rep, b, unit, "grid rev");
    }
    // Nested, degenerate, inverted, NaN, inf.
    for (A, B) in [
        (unit, c2AABB { min: c2v { x: 0.25, y: 0.25 }, max: c2v { x: 0.75, y: 0.75 } }),
        (unit, c2AABB { min: c2v { x: 0.5, y: 0.5 }, max: c2v { x: 0.5, y: 0.5 } }),
        (unit, c2AABB { min: c2v { x: 5.0, y: 5.0 }, max: c2v { x: -5.0, y: -5.0 } }),
        (
            c2AABB { min: c2v { x: f32::NAN, y: 0.0 }, max: c2v { x: 1.0, y: 1.0 } },
            unit,
        ),
        (
            unit,
            c2AABB { min: c2v { x: 0.0, y: f32::NAN }, max: c2v { x: f32::NAN, y: 1.0 } },
        ),
        (
            c2AABB {
                min: c2v { x: f32::NEG_INFINITY, y: f32::NEG_INFINITY },
                max: c2v { x: f32::INFINITY, y: f32::INFINITY },
            },
            unit,
        ),
        (
            c2AABB { min: c2v { x: -0.0, y: -0.0 }, max: c2v { x: 0.0, y: 0.0 } },
            c2AABB { min: c2v { x: 0.0, y: 0.0 }, max: c2v { x: -0.0, y: -0.0 } },
        ),
    ] {
        probe(&mut rep, A, B, "edge");
        probe(&mut rep, B, A, "edge rev");
    }
    rep.finish("row72_c2AABBtoAABB");
}

// ---------------------------------------------------------------------------
// Row 73 — c2CircletoCircle
// ---------------------------------------------------------------------------

#[test]
fn row73_c2CircletoCircle() {
    let l = libs();
    let (c, r) = (
        l.c.sym::<FnCircletoCircle>("c2CircletoCircle"),
        l.rs.sym::<FnCircletoCircle>("c2CircletoCircle"),
    );
    let mut g = Rng::new(0x49);
    let mut rep = Report::new();
    let mut probe = |rep: &mut Report, A: c2Circle, B: c2Circle, tag: &str| {
        let (x, y) = (c(A, B), r(A, B));
        rep.check(x == y, || {
            format!(
                "c2CircletoCircle[{tag}] A(p={} r={}) B(p={} r={}): C={x} Rust={y}",
                show_v(A.p), show_f32(A.r), show_v(B.p), show_f32(B.r)
            )
        });
    };
    for _ in 0..N {
        probe(&mut rep, g.circle(), g.circle(), "random");
    }
    // Exact-touch boundary (d2 == r2 -> strict `<` -> 0) and the sign-loss quirk
    // where a negative radius sum behaves like its magnitude.
    for (rA, rB) in [(1.0f32, 2.0f32), (0.0, 0.0), (0.0, 3.0), (-1.0, -2.0), (-3.0, 1.0), (5.0, -5.0)] {
        for scale in [0.0f32, 0.5, 0.999999, 1.0, 1.000001, 2.0] {
            let d = (rA + rB) * scale;
            let A = c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: rA };
            let B = c2Circle { p: c2v { x: d, y: 0.0 }, r: rB };
            probe(&mut rep, A, B, "touch");
            probe(&mut rep, B, A, "touch rev");
            // 3-4-5 triangle: d2 is exact, so the boundary is exact.
            let B2 = c2Circle { p: c2v { x: d * 0.6, y: d * 0.8 }, r: rB };
            probe(&mut rep, A, B2, "touch diag");
        }
    }
    for (A, B) in [
        (
            c2Circle { p: c2v { x: f32::NAN, y: 0.0 }, r: 1.0 },
            c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: 1.0 },
        ),
        (
            c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: f32::INFINITY },
            c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: f32::NEG_INFINITY },
        ),
        (
            c2Circle { p: c2v { x: 1.0e20, y: 0.0 }, r: 1.0e20 },
            c2Circle { p: c2v { x: -1.0e20, y: 0.0 }, r: 1.0e20 },
        ),
        (
            c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: f32::NAN },
            c2Circle { p: c2v { x: 1.0, y: 0.0 }, r: 1.0 },
        ),
    ] {
        probe(&mut rep, A, B, "edge");
        probe(&mut rep, B, A, "edge rev");
    }
    rep.finish("row73_c2CircletoCircle");
}

// ---------------------------------------------------------------------------
// Row 74 — c2CircletoAABB
// ---------------------------------------------------------------------------

#[test]
fn row74_c2CircletoAABB() {
    let l = libs();
    let (c, r) =
        (l.c.sym::<FnCircletoAABB>("c2CircletoAABB"), l.rs.sym::<FnCircletoAABB>("c2CircletoAABB"));
    let mut g = Rng::new(0x4a);
    let mut rep = Report::new();
    let mut probe = |rep: &mut Report, A: c2Circle, B: c2AABB, tag: &str| {
        let (x, y) = (c(A, B), r(A, B));
        rep.check(x == y, || {
            format!(
                "c2CircletoAABB[{tag}] circle(p={} r={}) box(min={} max={}): C={x} Rust={y}",
                show_v(A.p), show_f32(A.r), show_v(B.min), show_v(B.max)
            )
        });
    };
    for _ in 0..N {
        probe(&mut rep, g.circle(), g.aabb(), "random");
    }
    // Centre inside / on each face / on each corner / outside, for several radii.
    let bb = c2AABB { min: c2v { x: -1.0, y: -1.0 }, max: c2v { x: 1.0, y: 1.0 } };
    for rad in [0.0f32, 0.5, 1.0, 2.0, -1.0, f32::from_bits(1)] {
        for (px, py) in [
            (0.0f32, 0.0f32),  // inside
            (1.0, 0.0),        // on +x face
            (-1.0, 0.0),
            (0.0, 1.0),
            (0.0, -1.0),
            (1.0, 1.0),        // on corner
            (-1.0, -1.0),
            (1.5, 0.0),        // outside +x
            (1.5, 1.5),        // outside diagonal
            (2.0, 0.0),
            (1.0 + rad, 0.0),  // exactly `rad` past the face -> d2 == r2
            (5.0, 5.0),
        ] {
            probe(&mut rep, c2Circle { p: c2v { x: px, y: py }, r: rad }, bb, "grid");
        }
    }
    // Inverted / degenerate / flat boxes and NaN.
    for b in [
        c2AABB { min: c2v { x: 1.0, y: 1.0 }, max: c2v { x: -1.0, y: -1.0 } },
        c2AABB { min: c2v { x: 0.0, y: 0.0 }, max: c2v { x: 0.0, y: 0.0 } },
        c2AABB { min: c2v { x: -3.0, y: 2.0 }, max: c2v { x: 3.0, y: 2.0 } },
        c2AABB { min: c2v { x: f32::NAN, y: 0.0 }, max: c2v { x: 1.0, y: 1.0 } },
        c2AABB {
            min: c2v { x: f32::NEG_INFINITY, y: f32::NEG_INFINITY },
            max: c2v { x: f32::INFINITY, y: f32::INFINITY },
        },
    ] {
        for rad in [0.0f32, 1.0, f32::NAN, f32::INFINITY] {
            for p in [c2v { x: 0.0, y: 0.0 }, c2v { x: 4.0, y: -4.0 }, c2v { x: f32::NAN, y: 0.0 }] {
                probe(&mut rep, c2Circle { p, r: rad }, b, "edge");
            }
        }
    }
    rep.finish("row74_c2CircletoAABB");
}

// ---------------------------------------------------------------------------
// Row 75 — c2CircletoCapsule (all three distance regions)
// ---------------------------------------------------------------------------

#[test]
fn row75_c2CircletoCapsule() {
    let l = libs();
    let (c, r) = (
        l.c.sym::<FnCircletoCapsule>("c2CircletoCapsule"),
        l.rs.sym::<FnCircletoCapsule>("c2CircletoCapsule"),
    );
    let mut g = Rng::new(0x4b);
    let mut rep = Report::new();
    let mut probe = |rep: &mut Report, A: c2Circle, B: c2Capsule, tag: &str| {
        let (x, y) = (c(A, B), r(A, B));
        rep.check(x == y, || {
            format!(
                "c2CircletoCapsule[{tag}] circle(p={} r={}) capsule(a={} b={} r={}): C={x} Rust={y}",
                show_v(A.p), show_f32(A.r), show_v(B.a), show_v(B.b), show_f32(B.r)
            )
        });
    };
    for _ in 0..N {
        probe(&mut rep, g.circle(), g.capsule(), "random");
    }
    // Capsule along +x from (0,0) to (10,0). Sweep t past both ends so the
    // `da < 0`, `db < 0` and `db >= 0` regions are all hit.
    let cap = c2Capsule { a: c2v { x: 0.0, y: 0.0 }, b: c2v { x: 10.0, y: 0.0 }, r: 2.0 };
    for ti in -6i32..=16 {
        let t = ti as f32;
        for off in [0.0f32, 1.0, 2.0, 2.0000002, 3.0, -2.5] {
            for rad in [0.0f32, 0.5, 2.0, -1.0] {
                probe(
                    &mut rep,
                    c2Circle { p: c2v { x: t, y: off }, r: rad },
                    cap,
                    "sweep",
                );
            }
        }
    }
    // Degenerate capsule a == b: n == 0, so da == 0 and db == 0 -> the
    // `db >= 0` else-branch (distance to b), never the 0/0 division.
    for a in [c2v { x: 0.0, y: 0.0 }, c2v { x: -4.0, y: 7.0 }] {
        for rad in [0.0f32, 3.0, -3.0] {
            for p in [c2v { x: 0.0, y: 0.0 }, c2v { x: 1.0, y: 1.0 }, c2v { x: 100.0, y: 0.0 }] {
                probe(
                    &mut rep,
                    c2Circle { p, r: 1.0 },
                    c2Capsule { a, b: a, r: rad },
                    "degenerate",
                );
            }
        }
    }
    // NaN / inf in every position.
    for cap in [
        c2Capsule { a: c2v { x: f32::NAN, y: 0.0 }, b: c2v { x: 1.0, y: 0.0 }, r: 1.0 },
        c2Capsule { a: c2v { x: 0.0, y: 0.0 }, b: c2v { x: f32::INFINITY, y: 0.0 }, r: 1.0 },
        c2Capsule { a: c2v { x: 0.0, y: 0.0 }, b: c2v { x: 1.0, y: 0.0 }, r: f32::NAN },
        c2Capsule { a: c2v { x: -1.0e20, y: 0.0 }, b: c2v { x: 1.0e20, y: 0.0 }, r: 1.0 },
    ] {
        for p in [c2v { x: 0.5, y: 0.5 }, c2v { x: f32::NAN, y: 0.0 }, c2v { x: 0.0, y: 0.0 }] {
            probe(&mut rep, c2Circle { p, r: 1.0 }, cap, "edge");
        }
    }
    rep.finish("row75_c2CircletoCapsule");
}

// ---------------------------------------------------------------------------
// Row 76 — c2AABBtoCapsule (goes through c2GJK with use_radius = 1)
// ---------------------------------------------------------------------------

#[test]
fn row76_c2AABBtoCapsule() {
    let l = libs();
    let (c, r) = (
        l.c.sym::<FnAABBtoCapsule>("c2AABBtoCapsule"),
        l.rs.sym::<FnAABBtoCapsule>("c2AABBtoCapsule"),
    );
    let mut g = Rng::new(0x4c);
    let mut rep = Report::new();
    let mut probe = |rep: &mut Report, A: c2AABB, B: c2Capsule, tag: &str| {
        let (x, y) = (c(A, B), r(A, B));
        rep.check(x == y, || {
            format!(
                "c2AABBtoCapsule[{tag}] box(min={} max={}) capsule(a={} b={} r={}): C={x} Rust={y}",
                show_v(A.min), show_v(A.max), show_v(B.a), show_v(B.b), show_f32(B.r)
            )
        });
    };
    for _ in 0..N {
        probe(&mut rep, g.aabb(), g.capsule(), "random");
    }
    // A deliberate near/far sweep so both the overlapping and separated GJK
    // outcomes are exercised many times.
    let bb = c2AABB { min: c2v { x: -5.0, y: -5.0 }, max: c2v { x: 5.0, y: 5.0 } };
    for d in -12i32..=12 {
        let x = d as f32 * 1.5;
        for rad in [0.0f32, 1.0, 5.0, -2.0] {
            probe(
                &mut rep,
                bb,
                c2Capsule { a: c2v { x, y: -20.0 }, b: c2v { x, y: 20.0 }, r: rad },
                "sweep vertical",
            );
            probe(
                &mut rep,
                bb,
                c2Capsule { a: c2v { x: x - 3.0, y: x }, b: c2v { x: x + 3.0, y: x }, r: rad },
                "sweep diagonal",
            );
            // Degenerate capsule (a == b) and degenerate box.
            probe(
                &mut rep,
                bb,
                c2Capsule { a: c2v { x, y: x }, b: c2v { x, y: x }, r: rad },
                "degenerate capsule",
            );
            probe(
                &mut rep,
                c2AABB { min: c2v { x: 0.0, y: 0.0 }, max: c2v { x: 0.0, y: 0.0 } },
                c2Capsule { a: c2v { x, y: 0.0 }, b: c2v { x: x + 1.0, y: 0.0 }, r: rad },
                "degenerate box",
            );
        }
    }
    rep.finish("row76_c2AABBtoCapsule");
}

// ---------------------------------------------------------------------------
// Row 77 — c2CapsuletoCapsule
// ---------------------------------------------------------------------------

#[test]
fn row77_c2CapsuletoCapsule() {
    let l = libs();
    let (c, r) = (
        l.c.sym::<FnCapsuletoCapsule>("c2CapsuletoCapsule"),
        l.rs.sym::<FnCapsuletoCapsule>("c2CapsuletoCapsule"),
    );
    let mut g = Rng::new(0x4d);
    let mut rep = Report::new();
    let mut probe = |rep: &mut Report, A: c2Capsule, B: c2Capsule, tag: &str| {
        let (x, y) = (c(A, B), r(A, B));
        rep.check(x == y, || {
            format!(
                "c2CapsuletoCapsule[{tag}] A(a={} b={} r={}) B(a={} b={} r={}): C={x} Rust={y}",
                show_v(A.a), show_v(A.b), show_f32(A.r),
                show_v(B.a), show_v(B.b), show_f32(B.r)
            )
        });
    };
    for _ in 0..N {
        probe(&mut rep, g.capsule(), g.capsule(), "random");
    }
    let base = c2Capsule { a: c2v { x: -5.0, y: 0.0 }, b: c2v { x: 5.0, y: 0.0 }, r: 1.0 };
    for d in -10i32..=10 {
        let t = d as f32;
        for rad in [0.0f32, 1.0, 2.0, -1.0] {
            // Crossing (perpendicular).
            probe(
                &mut rep,
                base,
                c2Capsule { a: c2v { x: t, y: -5.0 }, b: c2v { x: t, y: 5.0 }, r: rad },
                "crossing",
            );
            // Parallel, offset in y by exactly the radius sum.
            probe(
                &mut rep,
                base,
                c2Capsule {
                    a: c2v { x: -5.0, y: base.r + rad },
                    b: c2v { x: 5.0, y: base.r + rad },
                    r: rad,
                },
                "parallel touching",
            );
            // Collinear, end to end.
            probe(
                &mut rep,
                base,
                c2Capsule { a: c2v { x: 5.0 + t, y: 0.0 }, b: c2v { x: 15.0 + t, y: 0.0 }, r: rad },
                "collinear",
            );
            // Degenerate (a == b) on both sides.
            probe(
                &mut rep,
                c2Capsule { a: c2v { x: 0.0, y: 0.0 }, b: c2v { x: 0.0, y: 0.0 }, r: base.r },
                c2Capsule { a: c2v { x: t, y: 0.0 }, b: c2v { x: t, y: 0.0 }, r: rad },
                "both degenerate",
            );
        }
    }
    // Identical capsules -> coincident, dist == 0.
    probe(&mut rep, base, base, "identical");
    rep.finish("row77_c2CapsuletoCapsule");
}

// ---------------------------------------------------------------------------
// Row 78 — c2Collided dispatch, all 9 valid combinations
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Copy, Clone)]
union ShapeU {
    circle: c2Circle,
    aabb: c2AABB,
    capsule: c2Capsule,
}

fn shape_of(g: &mut Rng, ty: c_int) -> (ShapeU, String) {
    match ty {
        C2_TYPE_CIRCLE => {
            let s = g.circle();
            (ShapeU { circle: s }, format!("CIRCLE(p={} r={})", show_v(s.p), show_f32(s.r)))
        }
        C2_TYPE_AABB => {
            let s = g.aabb();
            (ShapeU { aabb: s }, format!("AABB(min={} max={})", show_v(s.min), show_v(s.max)))
        }
        _ => {
            let s = g.capsule();
            (
                ShapeU { capsule: s },
                format!("CAPSULE(a={} b={} r={})", show_v(s.a), show_v(s.b), show_f32(s.r)),
            )
        }
    }
}

#[test]
fn row78_c2Collided_dispatch() {
    let l = libs();
    let (c, r) = (l.c.sym::<FnCollided>("c2Collided"), l.rs.sym::<FnCollided>("c2Collided"));
    let mut g = Rng::new(0x4e);
    let mut rep = Report::new();
    for ta in [C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_CAPSULE] {
        for tb in [C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_CAPSULE] {
            for _ in 0..2000 {
                let (a, da) = shape_of(&mut g, ta);
                let (b, db) = shape_of(&mut g, tb);
                let (x, y) = unsafe {
                    (
                        c(&raw const a as *const c_void, ta, &raw const b as *const c_void, tb),
                        r(&raw const a as *const c_void, ta, &raw const b as *const c_void, tb),
                    )
                };
                rep.check(x == y, || {
                    format!("c2Collided(typeA={ta}, typeB={tb}) A={da} B={db}: C={x} Rust={y}")
                });
            }
        }
    }
    rep.finish("row78_c2Collided_dispatch");
}

/// The three swapping branches deserve a dedicated check: `c2Collided` calls
/// `c2CircletoAABB(*(c2Circle*)B, *(c2AABB*)A)` for `(AABB, CIRCLE)`, i.e. the
/// pointers are reinterpreted the *other* way round. A translation that forgot
/// the swap would still pass a symmetric random test, so assert the equivalence
/// against the underlying primitive explicitly.
#[test]
fn row78_c2Collided_argument_swaps() {
    let l = libs();
    let coll = (l.c.sym::<FnCollided>("c2Collided"), l.rs.sym::<FnCollided>("c2Collided"));
    let cta = (
        l.c.sym::<FnCircletoAABB>("c2CircletoAABB"),
        l.rs.sym::<FnCircletoAABB>("c2CircletoAABB"),
    );
    let ctc = (
        l.c.sym::<FnCircletoCapsule>("c2CircletoCapsule"),
        l.rs.sym::<FnCircletoCapsule>("c2CircletoCapsule"),
    );
    let atc = (
        l.c.sym::<FnAABBtoCapsule>("c2AABBtoCapsule"),
        l.rs.sym::<FnAABBtoCapsule>("c2AABBtoCapsule"),
    );
    let mut g = Rng::new(0x4f);
    let mut rep = Report::new();
    for _ in 0..3000 {
        let circle = g.circle();
        let bb = g.aabb();
        let cap = g.capsule();

        // (AABB, CIRCLE): A is the *box*, B is the *circle*.
        let a = ShapeU { aabb: bb };
        let b = ShapeU { circle };
        let (x, y) = unsafe {
            (
                coll.0(&raw const a as *const c_void, C2_TYPE_AABB, &raw const b as *const c_void, C2_TYPE_CIRCLE),
                coll.1(&raw const a as *const c_void, C2_TYPE_AABB, &raw const b as *const c_void, C2_TYPE_CIRCLE),
            )
        };
        let want = (cta.0(circle, bb), cta.1(circle, bb));
        rep.check(x == y && x == want.0 && want.0 == want.1, || {
            format!("c2Collided(AABB, CIRCLE) swap: C={x} Rust={y} want={} / {}", want.0, want.1)
        });

        // (CAPSULE, CIRCLE): A is the *capsule*, B is the *circle*.
        let a = ShapeU { capsule: cap };
        let b = ShapeU { circle };
        let (x, y) = unsafe {
            (
                coll.0(&raw const a as *const c_void, C2_TYPE_CAPSULE, &raw const b as *const c_void, C2_TYPE_CIRCLE),
                coll.1(&raw const a as *const c_void, C2_TYPE_CAPSULE, &raw const b as *const c_void, C2_TYPE_CIRCLE),
            )
        };
        let want = (ctc.0(circle, cap), ctc.1(circle, cap));
        rep.check(x == y && x == want.0 && want.0 == want.1, || {
            format!("c2Collided(CAPSULE, CIRCLE) swap: C={x} Rust={y} want={} / {}", want.0, want.1)
        });

        // (CAPSULE, AABB): A is the *capsule*, B is the *box*.
        let a = ShapeU { capsule: cap };
        let b = ShapeU { aabb: bb };
        let (x, y) = unsafe {
            (
                coll.0(&raw const a as *const c_void, C2_TYPE_CAPSULE, &raw const b as *const c_void, C2_TYPE_AABB),
                coll.1(&raw const a as *const c_void, C2_TYPE_CAPSULE, &raw const b as *const c_void, C2_TYPE_AABB),
            )
        };
        let want = (atc.0(bb, cap), atc.1(bb, cap));
        rep.check(x == y && x == want.0 && want.0 == want.1, || {
            format!("c2Collided(CAPSULE, AABB) swap: C={x} Rust={y} want={} / {}", want.0, want.1)
        });
    }
    rep.finish("row78_c2Collided_argument_swaps");
}

// ---------------------------------------------------------------------------
// Row 79 — aabb, the only symbol declared in include/lib.h
// ---------------------------------------------------------------------------

#[test]
fn row79_aabb() {
    let l = libs();
    let (c, r) = (l.c.sym::<FnAabb>("aabb"), l.rs.sym::<FnAabb>("aabb"));
    let mut g = Rng::new(0x50);
    let mut rep = Report::new();
    let mut masks = [0usize; 8];
    let mut probe = |rep: &mut Report, masks: &mut [usize; 8], q: [f32; 4], tag: &str| {
        let (x, y) = (c(q[0], q[1], q[2], q[3]), r(q[0], q[1], q[2], q[3]));
        rep.check(x == y, || {
            format!(
                "aabb({}, {}, {}, {})[{tag}]: C={x} Rust={y}",
                show_f32(q[0]), show_f32(q[1]), show_f32(q[2]), show_f32(q[3])
            )
        });
        if (0..8).contains(&x) {
            masks[x as usize] += 1;
        }
    };

    // The three fixed shapes in `aabb()` are:
    //   circle  p=(-70,0)  r=20      -> spans x in [-90,-50], y in [-20,20]
    //   box     (-40,-40)..(-15,-15)
    //   capsule (-40,40)..(-20,100) r=10
    // Sweep a grid that covers, misses and straddles each of them so every one
    // of the 8 result masks is produced.
    for i in -13i32..=6 {
        for j in -13i32..=13 {
            let (x0, y0) = (i as f32 * 10.0, j as f32 * 10.0);
            for (w, h) in [(5.0f32, 5.0f32), (30.0, 30.0), (80.0, 200.0), (0.0, 0.0)] {
                probe(&mut rep, &mut masks, [x0, y0, x0 + w, y0 + h], "grid");
            }
        }
    }
    // Fully randomized, including inverted boxes and nasty floats.
    for _ in 0..N {
        let q = [g.finite_f32(), g.finite_f32(), g.finite_f32(), g.finite_f32()];
        probe(&mut rep, &mut masks, q, "random finite");
        let q = [g.nasty_f32(), g.nasty_f32(), g.nasty_f32(), g.nasty_f32()];
        probe(&mut rep, &mut masks, q, "random nasty");
    }
    // A box that swallows everything, and one far away.
    probe(&mut rep, &mut masks, [-1000.0, -1000.0, 1000.0, 1000.0], "everything");
    probe(&mut rep, &mut masks, [5000.0, 5000.0, 6000.0, 6000.0], "nothing");
    probe(&mut rep, &mut masks, [f32::NEG_INFINITY, f32::NEG_INFINITY, f32::INFINITY, f32::INFINITY], "inf");
    probe(&mut rep, &mut masks, [f32::NAN, f32::NAN, f32::NAN, f32::NAN], "nan");

    eprintln!("row79: aabb() result-mask histogram = {masks:?}");
    let covered = masks.iter().filter(|&&n| n > 0).count();
    assert!(covered >= 6, "aabb() only produced {covered} of 8 distinct masks: {masks:?}");
    rep.finish("row79_aabb");
}
