//! Phase B — valid-path differential tests for the cute_c2 geometry layer.
//!
//! Covers `CONFIGS.md` rows C1 … C14: `c2V`, `c2Maxv`, `c2Minv`, `c2Clampv`,
//! `c2Sub`, `c2Dot`, `c2CircletoCircle`, `c2CircletoAABB`, `c2AABBtoAABB` and
//! the four valid `f2` dispatch combinations (plus aliasing).
//!
//! Everything goes through the two `.so`s — the Rust crate is never linked.

mod common;

use common::*;
use std::ffi::c_void;

const N: usize = 20_000;

// ---------------------------------------------------------------------------
// C1 — c2V
// ---------------------------------------------------------------------------

#[test]
fn c1_c2v_all_float_classes() {
    let p = pair();
    // exhaustive over the special corpus (28 x 28 = 784 pairs)
    for &xb in SPECIAL_F32 {
        for &yb in SPECIAL_F32 {
            let (x, y) = (f32::from_bits(xb), f32::from_bits(yb));
            same(
                "c2V",
                (xb, yb),
                unsafe { (p.c.c2V)(x, y) },
                unsafe { (p.rs.c2V)(x, y) },
            );
        }
    }
    // randomized raw bit patterns
    let mut r = Rng::new(SEED ^ 0x01);
    for _ in 0..N {
        let (x, y) = (r.raw_f32(), r.raw_f32());
        same(
            "c2V/raw",
            (x.to_bits(), y.to_bits()),
            unsafe { (p.c.c2V)(x, y) },
            unsafe { (p.rs.c2V)(x, y) },
        );
    }
}

// ---------------------------------------------------------------------------
// C2 / C3 — c2Maxv, c2Minv
// ---------------------------------------------------------------------------

#[test]
fn c2_c3_maxv_minv() {
    let p = pair();
    // exhaustive x-component cross product with a fixed y, then swapped
    for &ab in SPECIAL_F32 {
        for &bb in SPECIAL_F32 {
            let a = C2v {
                x: f32::from_bits(ab),
                y: f32::from_bits(bb),
            };
            let b = C2v {
                x: f32::from_bits(bb),
                y: f32::from_bits(ab),
            };
            same(
                "c2Maxv",
                (ab, bb),
                unsafe { (p.c.c2Maxv)(a, b) },
                unsafe { (p.rs.c2Maxv)(a, b) },
            );
            same(
                "c2Minv",
                (ab, bb),
                unsafe { (p.c.c2Minv)(a, b) },
                unsafe { (p.rs.c2Minv)(a, b) },
            );
            // reversed argument order matters for the `>`/`<` ternaries
            same(
                "c2Maxv/rev",
                (bb, ab),
                unsafe { (p.c.c2Maxv)(b, a) },
                unsafe { (p.rs.c2Maxv)(b, a) },
            );
            same(
                "c2Minv/rev",
                (bb, ab),
                unsafe { (p.c.c2Minv)(b, a) },
                unsafe { (p.rs.c2Minv)(b, a) },
            );
        }
    }
    let mut r = Rng::new(SEED ^ 0x02);
    for _ in 0..N {
        let (a, b) = (r.raw_c2v(), r.raw_c2v());
        same(
            "c2Maxv/raw",
            (a.x.to_bits(), a.y.to_bits(), b.x.to_bits(), b.y.to_bits()),
            unsafe { (p.c.c2Maxv)(a, b) },
            unsafe { (p.rs.c2Maxv)(a, b) },
        );
        same(
            "c2Minv/raw",
            (a.x.to_bits(), a.y.to_bits(), b.x.to_bits(), b.y.to_bits()),
            unsafe { (p.c.c2Minv)(a, b) },
            unsafe { (p.rs.c2Minv)(a, b) },
        );
    }
    // equal operands, incl. +0 vs -0 both ways
    for &v in &[0.0f32, -0.0, 1.0, -1.0, f32::NAN, f32::INFINITY] {
        let a = C2v { x: v, y: v };
        same("c2Maxv/eq", v.to_bits(), unsafe { (p.c.c2Maxv)(a, a) }, unsafe {
            (p.rs.c2Maxv)(a, a)
        });
        same("c2Minv/eq", v.to_bits(), unsafe { (p.c.c2Minv)(a, a) }, unsafe {
            (p.rs.c2Minv)(a, a)
        });
    }
}

// ---------------------------------------------------------------------------
// C4 — c2Clampv
// ---------------------------------------------------------------------------

#[test]
fn c4_clampv() {
    let p = pair();
    let mut r = Rng::new(SEED ^ 0x04);

    // structured: below / inside / above, plus inverted ranges
    let lohi: &[(f32, f32)] = &[
        (0.0, 1.0),
        (1.0, 0.0), // inverted
        (-1.0, 1.0),
        (0.0, 0.0),
        (-0.0, 0.0),
        (f32::NEG_INFINITY, f32::INFINITY),
        (f32::NAN, 1.0),
        (0.0, f32::NAN),
        (f32::NAN, f32::NAN),
    ];
    for &(lo, hi) in lohi {
        for &ab in SPECIAL_F32 {
            let a = C2v {
                x: f32::from_bits(ab),
                y: -f32::from_bits(ab),
            };
            let l = C2v { x: lo, y: lo };
            let h = C2v { x: hi, y: hi };
            same(
                "c2Clampv",
                (ab, lo.to_bits(), hi.to_bits()),
                unsafe { (p.c.c2Clampv)(a, l, h) },
                unsafe { (p.rs.c2Clampv)(a, l, h) },
            );
        }
    }
    for _ in 0..N {
        let (a, l, h) = (r.raw_c2v(), r.raw_c2v(), r.raw_c2v());
        same(
            "c2Clampv/raw",
            (a.x.to_bits(), l.x.to_bits(), h.x.to_bits()),
            unsafe { (p.c.c2Clampv)(a, l, h) },
            unsafe { (p.rs.c2Clampv)(a, l, h) },
        );
    }
    // "nice" scaled values so the ordinary in/out-of-range paths dominate
    for _ in 0..N {
        let a = r.c2v(4.0);
        let l = r.c2v(2.0);
        let h = r.c2v(2.0);
        same(
            "c2Clampv/nice",
            (a.x.to_bits(), l.x.to_bits(), h.x.to_bits()),
            unsafe { (p.c.c2Clampv)(a, l, h) },
            unsafe { (p.rs.c2Clampv)(a, l, h) },
        );
    }
}

// ---------------------------------------------------------------------------
// C5 — c2Sub
// ---------------------------------------------------------------------------

#[test]
fn c5_sub() {
    let p = pair();
    for &ab in SPECIAL_F32 {
        for &bb in SPECIAL_F32 {
            let a = C2v {
                x: f32::from_bits(ab),
                y: f32::from_bits(bb),
            };
            let b = C2v {
                x: f32::from_bits(bb),
                y: f32::from_bits(ab),
            };
            same(
                "c2Sub",
                (ab, bb),
                unsafe { (p.c.c2Sub)(a, b) },
                unsafe { (p.rs.c2Sub)(a, b) },
            );
        }
    }
    let mut r = Rng::new(SEED ^ 0x05);
    for _ in 0..N {
        let (a, b) = (r.raw_c2v(), r.raw_c2v());
        same(
            "c2Sub/raw",
            (a.x.to_bits(), b.x.to_bits()),
            unsafe { (p.c.c2Sub)(a, b) },
            unsafe { (p.rs.c2Sub)(a, b) },
        );
    }
    for _ in 0..N {
        let (a, b) = (r.c2v(1e30), r.c2v(1e30));
        same(
            "c2Sub/big",
            (a.x.to_bits(), b.x.to_bits()),
            unsafe { (p.c.c2Sub)(a, b) },
            unsafe { (p.rs.c2Sub)(a, b) },
        );
    }
}

// ---------------------------------------------------------------------------
// C6 — c2Dot
// ---------------------------------------------------------------------------

#[test]
fn c6_dot() {
    let p = pair();
    for &ab in SPECIAL_F32 {
        for &bb in SPECIAL_F32 {
            // all four assignments of the two specials to the four components
            for (ax, ay, bx, by) in [
                (ab, ab, bb, bb),
                (ab, bb, bb, ab),
                (bb, ab, ab, bb),
                (ab, bb, ab, bb),
            ] {
                let a = C2v {
                    x: f32::from_bits(ax),
                    y: f32::from_bits(ay),
                };
                let b = C2v {
                    x: f32::from_bits(bx),
                    y: f32::from_bits(by),
                };
                same(
                    "c2Dot",
                    (ax, ay, bx, by),
                    unsafe { (p.c.c2Dot)(a, b) },
                    unsafe { (p.rs.c2Dot)(a, b) },
                );
            }
        }
    }
    let mut r = Rng::new(SEED ^ 0x06);
    for _ in 0..N {
        let (a, b) = (r.raw_c2v(), r.raw_c2v());
        same(
            "c2Dot/raw",
            (a.x.to_bits(), a.y.to_bits(), b.x.to_bits(), b.y.to_bits()),
            unsafe { (p.c.c2Dot)(a, b) },
            unsafe { (p.rs.c2Dot)(a, b) },
        );
    }
    // catastrophic cancellation: a.x*b.x == -(a.y*b.y)
    for _ in 0..N {
        let v = r.finite_f32(1e18);
        let a = C2v { x: v, y: v };
        let b = C2v { x: 1.0, y: -1.0 };
        same(
            "c2Dot/cancel",
            v.to_bits(),
            unsafe { (p.c.c2Dot)(a, b) },
            unsafe { (p.rs.c2Dot)(a, b) },
        );
    }
}

// ---------------------------------------------------------------------------
// C7 — c2CircletoCircle
// ---------------------------------------------------------------------------

#[test]
fn c7_circle_to_circle() {
    let p = pair();
    let mut r = Rng::new(SEED ^ 0x07);

    // structured: distance exactly == r1+r2 (strict `<` must reject)
    for d in [0.0f32, 0.5, 1.0, 1.9999999, 2.0, 2.0000002, 3.0, 1e30] {
        let a = C2Circle {
            p: C2v { x: 0.0, y: 0.0 },
            r: 1.0,
        };
        let b = C2Circle {
            p: C2v { x: d, y: 0.0 },
            r: 1.0,
        };
        same(
            "c2CircletoCircle/touch",
            d.to_bits(),
            unsafe { (p.c.c2CircletoCircle)(a, b) },
            unsafe { (p.rs.c2CircletoCircle)(a, b) },
        );
    }
    // zero / negative / infinite / NaN radii
    for &ra in &[0.0f32, -0.0, -1.0, 1.0, f32::INFINITY, f32::NEG_INFINITY, f32::NAN] {
        for &rb in &[0.0f32, -1.0, 1.0, f32::INFINITY, f32::NAN] {
            let a = C2Circle {
                p: C2v { x: 0.0, y: 0.0 },
                r: ra,
            };
            let b = C2Circle {
                p: C2v { x: 1.0, y: 0.0 },
                r: rb,
            };
            same(
                "c2CircletoCircle/radii",
                (ra.to_bits(), rb.to_bits()),
                unsafe { (p.c.c2CircletoCircle)(a, b) },
                unsafe { (p.rs.c2CircletoCircle)(a, b) },
            );
        }
    }
    // identical circles
    for _ in 0..2000 {
        let a = r.circle(4.0);
        same(
            "c2CircletoCircle/same",
            (a.p.x.to_bits(), a.r.to_bits()),
            unsafe { (p.c.c2CircletoCircle)(a, a) },
            unsafe { (p.rs.c2CircletoCircle)(a, a) },
        );
    }
    // randomized, clustered so overlap/no-overlap are both common
    for _ in 0..N {
        let a = r.circle(4.0);
        let b = r.circle(4.0);
        same(
            "c2CircletoCircle/rand",
            (
                a.p.x.to_bits(),
                a.p.y.to_bits(),
                a.r.to_bits(),
                b.p.x.to_bits(),
                b.p.y.to_bits(),
                b.r.to_bits(),
            ),
            unsafe { (p.c.c2CircletoCircle)(a, b) },
            unsafe { (p.rs.c2CircletoCircle)(a, b) },
        );
    }
    // fully random bit patterns
    for _ in 0..N {
        let a = C2Circle {
            p: r.raw_c2v(),
            r: r.raw_f32(),
        };
        let b = C2Circle {
            p: r.raw_c2v(),
            r: r.raw_f32(),
        };
        same(
            "c2CircletoCircle/raw",
            (a.p.x.to_bits(), a.r.to_bits(), b.p.x.to_bits(), b.r.to_bits()),
            unsafe { (p.c.c2CircletoCircle)(a, b) },
            unsafe { (p.rs.c2CircletoCircle)(a, b) },
        );
    }
}

// ---------------------------------------------------------------------------
// C8 — c2CircletoAABB
// ---------------------------------------------------------------------------

#[test]
fn c8_circle_to_aabb() {
    let p = pair();
    let mut r = Rng::new(SEED ^ 0x08);

    let unit = C2Aabb {
        min: C2v { x: -1.0, y: -1.0 },
        max: C2v { x: 1.0, y: 1.0 },
    };
    // centre inside, on each edge, on each corner, just outside
    let centres: &[(f32, f32)] = &[
        (0.0, 0.0),
        (1.0, 0.0),
        (-1.0, 0.0),
        (0.0, 1.0),
        (0.0, -1.0),
        (1.0, 1.0),
        (-1.0, -1.0),
        (1.0, -1.0),
        (-1.0, 1.0),
        (1.5, 0.0),
        (2.0, 0.0),
        (2.0, 2.0),
        (1.5, 1.5),
        (1e30, 0.0),
    ];
    for &(cx, cy) in centres {
        for &rad in &[0.0f32, -0.0, -1.0, 0.5, 1.0, 1.4142135, 1.4142137, 1e30, f32::INFINITY, f32::NAN] {
            let c = C2Circle {
                p: C2v { x: cx, y: cy },
                r: rad,
            };
            same(
                "c2CircletoAABB/struct",
                (cx.to_bits(), cy.to_bits(), rad.to_bits()),
                unsafe { (p.c.c2CircletoAABB)(c, unit) },
                unsafe { (p.rs.c2CircletoAABB)(c, unit) },
            );
        }
    }
    // inverted / degenerate AABBs
    let boxes: &[C2Aabb] = &[
        C2Aabb {
            min: C2v { x: 1.0, y: 1.0 },
            max: C2v { x: -1.0, y: -1.0 },
        },
        C2Aabb {
            min: C2v { x: 0.0, y: 0.0 },
            max: C2v { x: 0.0, y: 0.0 },
        },
        C2Aabb {
            min: C2v { x: f32::NAN, y: 0.0 },
            max: C2v { x: 1.0, y: 1.0 },
        },
        C2Aabb {
            min: C2v { x: 0.0, y: 0.0 },
            max: C2v { x: f32::NAN, y: f32::NAN },
        },
        C2Aabb {
            min: C2v {
                x: f32::NEG_INFINITY,
                y: f32::NEG_INFINITY,
            },
            max: C2v {
                x: f32::INFINITY,
                y: f32::INFINITY,
            },
        },
    ];
    for bx in boxes {
        for &rad in &[0.0f32, 1.0, f32::NAN, f32::INFINITY] {
            let c = C2Circle {
                p: C2v { x: 0.25, y: -0.25 },
                r: rad,
            };
            same(
                "c2CircletoAABB/box",
                (bx.min.x.to_bits(), bx.max.x.to_bits(), rad.to_bits()),
                unsafe { (p.c.c2CircletoAABB)(c, *bx) },
                unsafe { (p.rs.c2CircletoAABB)(c, *bx) },
            );
        }
    }
    for _ in 0..N {
        let c = r.circle(4.0);
        let bx = r.aabb(4.0);
        same(
            "c2CircletoAABB/rand",
            (
                c.p.x.to_bits(),
                c.p.y.to_bits(),
                c.r.to_bits(),
                bx.min.x.to_bits(),
                bx.min.y.to_bits(),
                bx.max.x.to_bits(),
                bx.max.y.to_bits(),
            ),
            unsafe { (p.c.c2CircletoAABB)(c, bx) },
            unsafe { (p.rs.c2CircletoAABB)(c, bx) },
        );
    }
    for _ in 0..N {
        let c = C2Circle {
            p: r.raw_c2v(),
            r: r.raw_f32(),
        };
        let bx = C2Aabb {
            min: r.raw_c2v(),
            max: r.raw_c2v(),
        };
        same(
            "c2CircletoAABB/raw",
            (c.p.x.to_bits(), c.r.to_bits(), bx.min.x.to_bits(), bx.max.x.to_bits()),
            unsafe { (p.c.c2CircletoAABB)(c, bx) },
            unsafe { (p.rs.c2CircletoAABB)(c, bx) },
        );
    }
}

// ---------------------------------------------------------------------------
// C9 — c2AABBtoAABB
// ---------------------------------------------------------------------------

#[test]
fn c9_aabb_to_aabb() {
    let p = pair();
    let mut r = Rng::new(SEED ^ 0x09);

    let a = C2Aabb {
        min: C2v { x: 0.0, y: 0.0 },
        max: C2v { x: 1.0, y: 1.0 },
    };
    // slide b along x and y so each of the four `<` tests flips
    for t in [-3.0f32, -2.0, -1.0000001, -1.0, -0.9999999, -0.5, 0.0, 0.5, 1.0, 1.0000001, 2.0] {
        for axis in 0..2 {
            let b = if axis == 0 {
                C2Aabb {
                    min: C2v { x: t, y: 0.0 },
                    max: C2v { x: t + 1.0, y: 1.0 },
                }
            } else {
                C2Aabb {
                    min: C2v { x: 0.0, y: t },
                    max: C2v { x: 1.0, y: t + 1.0 },
                }
            };
            same(
                "c2AABBtoAABB/slide",
                (t.to_bits(), axis),
                unsafe { (p.c.c2AABBtoAABB)(a, b) },
                unsafe { (p.rs.c2AABBtoAABB)(a, b) },
            );
            same(
                "c2AABBtoAABB/slide-rev",
                (t.to_bits(), axis),
                unsafe { (p.c.c2AABBtoAABB)(b, a) },
                unsafe { (p.rs.c2AABBtoAABB)(b, a) },
            );
        }
    }
    // NaN-laden boxes: every `<` is false → `!(0) == 1`
    for &nb in SPECIAL_F32 {
        let v = f32::from_bits(nb);
        let b = C2Aabb {
            min: C2v { x: v, y: v },
            max: C2v { x: v, y: v },
        };
        same(
            "c2AABBtoAABB/special",
            nb,
            unsafe { (p.c.c2AABBtoAABB)(a, b) },
            unsafe { (p.rs.c2AABBtoAABB)(a, b) },
        );
        same(
            "c2AABBtoAABB/special-both",
            nb,
            unsafe { (p.c.c2AABBtoAABB)(b, b) },
            unsafe { (p.rs.c2AABBtoAABB)(b, b) },
        );
    }
    for _ in 0..N {
        let x = r.aabb(4.0);
        let y = r.aabb(4.0);
        same(
            "c2AABBtoAABB/rand",
            (
                x.min.x.to_bits(),
                x.min.y.to_bits(),
                x.max.x.to_bits(),
                x.max.y.to_bits(),
                y.min.x.to_bits(),
                y.min.y.to_bits(),
                y.max.x.to_bits(),
                y.max.y.to_bits(),
            ),
            unsafe { (p.c.c2AABBtoAABB)(x, y) },
            unsafe { (p.rs.c2AABBtoAABB)(x, y) },
        );
    }
    for _ in 0..N {
        let x = C2Aabb {
            min: r.raw_c2v(),
            max: r.raw_c2v(),
        };
        let y = C2Aabb {
            min: r.raw_c2v(),
            max: r.raw_c2v(),
        };
        same(
            "c2AABBtoAABB/raw",
            (x.min.x.to_bits(), x.max.x.to_bits(), y.min.x.to_bits(), y.max.x.to_bits()),
            unsafe { (p.c.c2AABBtoAABB)(x, y) },
            unsafe { (p.rs.c2AABBtoAABB)(x, y) },
        );
    }
}

// ---------------------------------------------------------------------------
// C10 … C13 — the four valid f2 dispatch combinations
// ---------------------------------------------------------------------------

/// Byte buffer big enough for either shape, so the same memory can legally be
/// reinterpreted as `c2Circle` (12 B) or `c2AABB` (16 B) — exactly what the C
/// `f2` does with its `const void *` arguments.
#[repr(C, align(16))]
#[derive(Copy, Clone)]
struct ShapeBuf([f32; 4]);

impl ShapeBuf {
    fn circle(c: C2Circle) -> ShapeBuf {
        ShapeBuf([c.p.x, c.p.y, c.r, 0.0])
    }
    fn aabb(b: C2Aabb) -> ShapeBuf {
        ShapeBuf([b.min.x, b.min.y, b.max.x, b.max.y])
    }
    fn ptr(&self) -> *const c_void {
        self as *const ShapeBuf as *const c_void
    }
}

#[test]
fn c10_f2_circle_circle() {
    let p = pair();
    let mut r = Rng::new(SEED ^ 0x10);
    for _ in 0..N {
        let a = ShapeBuf::circle(r.circle(4.0));
        let b = ShapeBuf::circle(r.circle(4.0));
        same(
            "f2(CIRCLE,CIRCLE)",
            (a.0, b.0),
            unsafe { (p.c.f2)(a.ptr(), C2_TYPE_CIRCLE, b.ptr(), C2_TYPE_CIRCLE) },
            unsafe { (p.rs.f2)(a.ptr(), C2_TYPE_CIRCLE, b.ptr(), C2_TYPE_CIRCLE) },
        );
    }
    for _ in 0..N {
        let a = ShapeBuf([r.raw_f32(), r.raw_f32(), r.raw_f32(), 0.0]);
        let b = ShapeBuf([r.raw_f32(), r.raw_f32(), r.raw_f32(), 0.0]);
        same(
            "f2(CIRCLE,CIRCLE)/raw",
            (a.0.map(f32::to_bits), b.0.map(f32::to_bits)),
            unsafe { (p.c.f2)(a.ptr(), C2_TYPE_CIRCLE, b.ptr(), C2_TYPE_CIRCLE) },
            unsafe { (p.rs.f2)(a.ptr(), C2_TYPE_CIRCLE, b.ptr(), C2_TYPE_CIRCLE) },
        );
    }
}

#[test]
fn c11_f2_circle_aabb() {
    let p = pair();
    let mut r = Rng::new(SEED ^ 0x11);
    for _ in 0..N {
        let a = ShapeBuf::circle(r.circle(4.0));
        let b = ShapeBuf::aabb(r.aabb(4.0));
        same(
            "f2(CIRCLE,AABB)",
            (a.0, b.0),
            unsafe { (p.c.f2)(a.ptr(), C2_TYPE_CIRCLE, b.ptr(), C2_TYPE_AABB) },
            unsafe { (p.rs.f2)(a.ptr(), C2_TYPE_CIRCLE, b.ptr(), C2_TYPE_AABB) },
        );
    }
    for _ in 0..N {
        let a = ShapeBuf([r.raw_f32(), r.raw_f32(), r.raw_f32(), 0.0]);
        let b = ShapeBuf([r.raw_f32(), r.raw_f32(), r.raw_f32(), r.raw_f32()]);
        same(
            "f2(CIRCLE,AABB)/raw",
            (a.0.map(f32::to_bits), b.0.map(f32::to_bits)),
            unsafe { (p.c.f2)(a.ptr(), C2_TYPE_CIRCLE, b.ptr(), C2_TYPE_AABB) },
            unsafe { (p.rs.f2)(a.ptr(), C2_TYPE_CIRCLE, b.ptr(), C2_TYPE_AABB) },
        );
    }
}

#[test]
fn c12_f2_aabb_circle_swapped_args() {
    let p = pair();
    let mut r = Rng::new(SEED ^ 0x12);
    // This arm calls c2CircletoAABB(*(c2Circle*)B, *(c2AABB*)A) — the operands
    // are swapped relative to the declared parameter order.
    for _ in 0..N {
        let a = ShapeBuf::aabb(r.aabb(4.0));
        let b = ShapeBuf::circle(r.circle(4.0));
        same(
            "f2(AABB,CIRCLE)",
            (a.0, b.0),
            unsafe { (p.c.f2)(a.ptr(), C2_TYPE_AABB, b.ptr(), C2_TYPE_CIRCLE) },
            unsafe { (p.rs.f2)(a.ptr(), C2_TYPE_AABB, b.ptr(), C2_TYPE_CIRCLE) },
        );
    }
    // cross-check that the swap really happened, using the direct primitive
    for _ in 0..2000 {
        let bx = r.aabb(4.0);
        let ci = r.circle(4.0);
        let a = ShapeBuf::aabb(bx);
        let b = ShapeBuf::circle(ci);
        let via_f2 = unsafe { (p.c.f2)(a.ptr(), C2_TYPE_AABB, b.ptr(), C2_TYPE_CIRCLE) };
        let direct = unsafe { (p.c.c2CircletoAABB)(ci, bx) };
        assert_eq!(via_f2, direct, "C f2 AABB/CIRCLE arm should swap operands");
        let rs_f2 = unsafe { (p.rs.f2)(a.ptr(), C2_TYPE_AABB, b.ptr(), C2_TYPE_CIRCLE) };
        let rs_direct = unsafe { (p.rs.c2CircletoAABB)(ci, bx) };
        assert_eq!(rs_f2, rs_direct, "Rust f2 AABB/CIRCLE arm should swap operands");
        assert_eq!(via_f2, rs_f2);
    }
    for _ in 0..N {
        let a = ShapeBuf([r.raw_f32(), r.raw_f32(), r.raw_f32(), r.raw_f32()]);
        let b = ShapeBuf([r.raw_f32(), r.raw_f32(), r.raw_f32(), 0.0]);
        same(
            "f2(AABB,CIRCLE)/raw",
            (a.0.map(f32::to_bits), b.0.map(f32::to_bits)),
            unsafe { (p.c.f2)(a.ptr(), C2_TYPE_AABB, b.ptr(), C2_TYPE_CIRCLE) },
            unsafe { (p.rs.f2)(a.ptr(), C2_TYPE_AABB, b.ptr(), C2_TYPE_CIRCLE) },
        );
    }
}

#[test]
fn c13_f2_aabb_aabb() {
    let p = pair();
    let mut r = Rng::new(SEED ^ 0x13);
    for _ in 0..N {
        let a = ShapeBuf::aabb(r.aabb(4.0));
        let b = ShapeBuf::aabb(r.aabb(4.0));
        same(
            "f2(AABB,AABB)",
            (a.0, b.0),
            unsafe { (p.c.f2)(a.ptr(), C2_TYPE_AABB, b.ptr(), C2_TYPE_AABB) },
            unsafe { (p.rs.f2)(a.ptr(), C2_TYPE_AABB, b.ptr(), C2_TYPE_AABB) },
        );
    }
    for _ in 0..N {
        let a = ShapeBuf([r.raw_f32(), r.raw_f32(), r.raw_f32(), r.raw_f32()]);
        let b = ShapeBuf([r.raw_f32(), r.raw_f32(), r.raw_f32(), r.raw_f32()]);
        same(
            "f2(AABB,AABB)/raw",
            (a.0.map(f32::to_bits), b.0.map(f32::to_bits)),
            unsafe { (p.c.f2)(a.ptr(), C2_TYPE_AABB, b.ptr(), C2_TYPE_AABB) },
            unsafe { (p.rs.f2)(a.ptr(), C2_TYPE_AABB, b.ptr(), C2_TYPE_AABB) },
        );
    }
}

// ---------------------------------------------------------------------------
// C14 — f2 with A and B aliasing the same buffer, all four valid type pairs
// ---------------------------------------------------------------------------

#[test]
fn c14_f2_aliased_pointers() {
    let p = pair();
    let mut r = Rng::new(SEED ^ 0x14);
    for _ in 0..N {
        let buf = ShapeBuf([r.nice_f32(4.0), r.nice_f32(4.0), r.nice_f32(4.0), r.nice_f32(4.0)]);
        for &(ta, tb) in &[
            (C2_TYPE_CIRCLE, C2_TYPE_CIRCLE),
            (C2_TYPE_CIRCLE, C2_TYPE_AABB),
            (C2_TYPE_AABB, C2_TYPE_CIRCLE),
            (C2_TYPE_AABB, C2_TYPE_AABB),
        ] {
            same(
                "f2/aliased",
                (buf.0.map(f32::to_bits), ta, tb),
                unsafe { (p.c.f2)(buf.ptr(), ta, buf.ptr(), tb) },
                unsafe { (p.rs.f2)(buf.ptr(), ta, buf.ptr(), tb) },
            );
        }
    }
}
