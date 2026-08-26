//! Phase B — valid-path differential tests, one test per row of `CONFIGS.md`.
//!
//! Every test loads BOTH `.so`s via `libloading` and compares the raw bit
//! patterns returned through the FFI boundary.

mod common;

use std::ffi::c_void;

use common::*;

// ===========================================================================
// C01–C09 — the lowest-level c2 primitives
// ===========================================================================

/// C01 — `c2V`
#[test]
fn c01_c2v() {
    let p = both();
    let mut rng = Rng::new(0xC01);
    for i in 0..20_000 {
        let (x, y) = (rng.wild_f32(), rng.wild_f32());
        let c = unsafe { (p.c.c2V)(x, y) };
        let r = unsafe { (p.r.c2V)(x, y) };
        eq_v2(&format!("c01[{i}] c2V({x:?},{y:?})"), c, r);
    }
    for &x in SPECIAL_F32 {
        for &y in SPECIAL_F32 {
            let c = unsafe { (p.c.c2V)(x, y) };
            let r = unsafe { (p.r.c2V)(x, y) };
            eq_v2(&format!("c01 special c2V({x:?},{y:?})"), c, r);
        }
    }
}

fn bin2_row(
    label: &str,
    f: impl Fn(&Lib, C2v, C2v) -> C2v,
    p: &Pair,
    a: C2v,
    b: C2v,
    ctx: &str,
) {
    let c = f(&p.c, a, b);
    let r = f(&p.r, a, b);
    eq_v2(&format!("{label} {ctx} a={a:?} b={b:?}"), c, r);
}

/// C02 — `c2Maxv`
#[test]
fn c02_c2maxv() {
    let p = both();
    let mut rng = Rng::new(0xC02);
    for i in 0..20_000 {
        let (a, b) = (rng.wild_v2(), rng.wild_v2());
        bin2_row(
            "c02",
            |l, a, b| unsafe { (l.c2Maxv)(a, b) },
            &p,
            a,
            b,
            &format!("[{i}]"),
        );
    }
    // Full 25-combination ordered/unordered grid per lane.
    let grid = [f32::NEG_INFINITY, -0.0, 0.0, f32::INFINITY, NAN_A];
    let grid2 = [f32::NEG_INFINITY, -0.0, 0.0, f32::INFINITY, NAN_B];
    for &ax in &grid {
        for &bx in &grid2 {
            for &ay in &grid {
                for &by in &grid2 {
                    let a = C2v { x: ax, y: ay };
                    let b = C2v { x: bx, y: by };
                    bin2_row(
                        "c02",
                        |l, a, b| unsafe { (l.c2Maxv)(a, b) },
                        &p,
                        a,
                        b,
                        "grid",
                    );
                }
            }
        }
    }
}

/// C03 — `c2Minv`
#[test]
fn c03_c2minv() {
    let p = both();
    let mut rng = Rng::new(0xC03);
    for i in 0..20_000 {
        let (a, b) = (rng.wild_v2(), rng.wild_v2());
        bin2_row(
            "c03",
            |l, a, b| unsafe { (l.c2Minv)(a, b) },
            &p,
            a,
            b,
            &format!("[{i}]"),
        );
    }
    let grid = [f32::NEG_INFINITY, -0.0, 0.0, f32::INFINITY, NAN_A];
    let grid2 = [f32::NEG_INFINITY, -0.0, 0.0, f32::INFINITY, NAN_B];
    for &ax in &grid {
        for &bx in &grid2 {
            for &ay in &grid {
                for &by in &grid2 {
                    let a = C2v { x: ax, y: ay };
                    let b = C2v { x: bx, y: by };
                    bin2_row(
                        "c03",
                        |l, a, b| unsafe { (l.c2Minv)(a, b) },
                        &p,
                        a,
                        b,
                        "grid",
                    );
                }
            }
        }
    }
}

/// C04 — `c2Clampv` (incl. inverted `lo > hi` boxes and NaN in any lane)
#[test]
fn c04_c2clampv() {
    let p = both();
    let mut rng = Rng::new(0xC04);
    for i in 0..30_000 {
        let a = rng.wild_v2();
        let (lo, hi) = match rng.below(3) {
            // normalised box
            0 => {
                let m = rng.finite_v2();
                let s = C2v {
                    x: rng.range(0.0, 50.0),
                    y: rng.range(0.0, 50.0),
                };
                (
                    m,
                    C2v {
                        x: m.x + s.x,
                        y: m.y + s.y,
                    },
                )
            }
            // inverted box
            1 => (rng.wild_v2(), rng.wild_v2()),
            // degenerate box
            _ => {
                let m = rng.wild_v2();
                (m, m)
            }
        };
        let c = unsafe { (p.c.c2Clampv)(a, lo, hi) };
        let r = unsafe { (p.r.c2Clampv)(a, lo, hi) };
        eq_v2(
            &format!("c04[{i}] c2Clampv(a={a:?}, lo={lo:?}, hi={hi:?})"),
            c,
            r,
        );
    }
    // NaN in each of the 6 lanes in turn.
    for lane in 0..6 {
        for &nan in &[NAN_A, NAN_B, NAN_C, SNAN] {
            let mut v = [1.0f32, 2.0, -1.0, -2.0, 3.0, 4.0];
            v[lane] = nan;
            let a = C2v { x: v[0], y: v[1] };
            let lo = C2v { x: v[2], y: v[3] };
            let hi = C2v { x: v[4], y: v[5] };
            let c = unsafe { (p.c.c2Clampv)(a, lo, hi) };
            let r = unsafe { (p.r.c2Clampv)(a, lo, hi) };
            eq_v2(&format!("c04 nan lane {lane} {nan:?}"), c, r);
        }
    }
}

/// C05 — `c2Sub` (incl. NaN in BOTH operands with distinct payloads)
#[test]
fn c05_c2sub() {
    let p = both();
    let mut rng = Rng::new(0xC05);
    for i in 0..30_000 {
        let (a, b) = (rng.wild_v2(), rng.wild_v2());
        bin2_row(
            "c05",
            |l, a, b| unsafe { (l.c2Sub)(a, b) },
            &p,
            a,
            b,
            &format!("[{i}]"),
        );
    }
    // Both-NaN, distinct payloads: pins which operand survives `subss`.
    for i in 0..5_000 {
        let a = C2v {
            x: rng.nan_payload(),
            y: rng.nan_payload(),
        };
        let b = C2v {
            x: rng.nan_payload(),
            y: rng.nan_payload(),
        };
        bin2_row(
            "c05 both-nan",
            |l, a, b| unsafe { (l.c2Sub)(a, b) },
            &p,
            a,
            b,
            &format!("[{i}]"),
        );
    }
    for &x in SPECIAL_F32 {
        for &y in SPECIAL_F32 {
            let a = C2v { x, y };
            let b = C2v { x: y, y: x };
            bin2_row("c05 special", |l, a, b| unsafe { (l.c2Sub)(a, b) }, &p, a, b, "");
        }
    }
}

/// C06 — `c2Dot` (both-NaN payload survivor across `mulss`/`addss`)
#[test]
fn c06_c2dot() {
    let p = both();
    let mut rng = Rng::new(0xC06);
    for i in 0..30_000 {
        let (a, b) = (rng.wild_v2(), rng.wild_v2());
        let c = unsafe { (p.c.c2Dot)(a, b) };
        let r = unsafe { (p.r.c2Dot)(a, b) };
        eq_f32(&format!("c06[{i}] c2Dot(a={a:?}, b={b:?})"), c, r);
    }
    // All four lanes NaN with distinct payloads.
    for i in 0..10_000 {
        let a = C2v {
            x: rng.nan_payload(),
            y: rng.nan_payload(),
        };
        let b = C2v {
            x: rng.nan_payload(),
            y: rng.nan_payload(),
        };
        let c = unsafe { (p.c.c2Dot)(a, b) };
        let r = unsafe { (p.r.c2Dot)(a, b) };
        eq_f32(&format!("c06 nan[{i}] a={a:?} b={b:?}"), c, r);
    }
    // inf * 0 (invalid-operation default NaN) in all lane placements.
    let vals = [0.0f32, -0.0, f32::INFINITY, f32::NEG_INFINITY, 1.0, NAN_A];
    for &ax in &vals {
        for &ay in &vals {
            for &bx in &vals {
                for &by in &vals {
                    let a = C2v { x: ax, y: ay };
                    let b = C2v { x: bx, y: by };
                    let c = unsafe { (p.c.c2Dot)(a, b) };
                    let r = unsafe { (p.r.c2Dot)(a, b) };
                    eq_f32(&format!("c06 grid a={a:?} b={b:?}"), c, r);
                }
            }
        }
    }
}

/// C07 — `c2CircletoCircle`
#[test]
fn c07_c2circletocircle() {
    let p = both();
    let mut rng = Rng::new(0xC07);

    let check = |a: C2Circle, b: C2Circle, ctx: &str| {
        let c = unsafe { (p.c.c2CircletoCircle)(a, b) };
        let r = unsafe { (p.r.c2CircletoCircle)(a, b) };
        eq_i32(&format!("c07 {ctx} A={a:?} B={b:?}"), c, r);
    };

    for i in 0..20_000 {
        check(rng.wild_circle(), rng.wild_circle(), &format!("wild[{i}]"));
    }
    // Overlapping / disjoint / exactly touching.
    for i in 0..20_000 {
        let a = C2Circle {
            p: rng.finite_v2(),
            r: rng.range(0.0, 10.0),
        };
        let sep = match rng.below(3) {
            0 => a.r * 0.5,                       // overlapping
            1 => a.r + rng.range(0.0, 5.0) + 1.0, // disjoint
            _ => a.r,                             // exactly touching (d2 == r2)
        };
        let ang = rng.range(0.0, 6.2831855);
        let b = C2Circle {
            p: C2v {
                x: a.p.x + sep * ang.cos(),
                y: a.p.y + sep * ang.sin(),
            },
            r: 0.0,
        };
        check(a, b, &format!("geom[{i}]"));
    }
    // Exact touching with integral coordinates: d2 == r2 exactly.
    for d in 0..40 {
        let a = C2Circle {
            p: C2v { x: 0.0, y: 0.0 },
            r: d as f32,
        };
        for &br in &[0.0f32, 1.0, 2.0] {
            let b = C2Circle {
                p: C2v {
                    x: (d as f32) + br,
                    y: 0.0,
                },
                r: br,
            };
            check(a, b, "touch");
        }
    }
    // Negative / zero / NaN radii, including BOTH radii NaN (payload survivor).
    let radii = [0.0f32, -0.0, -1.0, 1.0, f32::INFINITY, NAN_A, NAN_B, SNAN];
    for &ra in &radii {
        for &rb in &radii {
            let a = C2Circle {
                p: C2v { x: 0.0, y: 0.0 },
                r: ra,
            };
            let b = C2Circle {
                p: C2v { x: 1.0, y: 1.0 },
                r: rb,
            };
            check(a, b, "radii-grid");
        }
    }
    for i in 0..5_000 {
        let a = C2Circle {
            p: rng.finite_v2(),
            r: rng.nan_payload(),
        };
        let b = C2Circle {
            p: rng.finite_v2(),
            r: rng.nan_payload(),
        };
        check(a, b, &format!("both-nan-radius[{i}]"));
    }
}

/// C08 — `c2CircletoAABB`
#[test]
fn c08_c2circletoaabb() {
    let p = both();
    let mut rng = Rng::new(0xC08);

    let check = |a: C2Circle, b: C2Aabb, ctx: &str| {
        let c = unsafe { (p.c.c2CircletoAABB)(a, b) };
        let r = unsafe { (p.r.c2CircletoAABB)(a, b) };
        eq_i32(&format!("c08 {ctx} A={a:?} B={b:?}"), c, r);
    };

    for i in 0..20_000 {
        check(rng.wild_circle(), rng.wild_aabb(), &format!("wild[{i}]"));
    }
    for i in 0..20_000 {
        // Well-formed box, circle centre inside / outside / on edge / on corner.
        let min = rng.finite_v2();
        let b = C2Aabb {
            min,
            max: C2v {
                x: min.x + rng.range(0.0, 20.0),
                y: min.y + rng.range(0.0, 20.0),
            },
        };
        let centre = match rng.below(4) {
            0 => C2v {
                x: rng.range(b.min.x, b.max.x),
                y: rng.range(b.min.y, b.max.y),
            }, // inside
            1 => C2v {
                x: b.min.x - rng.range(0.0, 10.0),
                y: b.min.y - rng.range(0.0, 10.0),
            }, // outside
            2 => C2v { x: b.min.x, y: b.min.y }, // corner
            _ => C2v {
                x: b.min.x,
                y: rng.range(b.min.y, b.max.y),
            }, // edge
        };
        let a = C2Circle {
            p: centre,
            r: rng.range(-2.0, 10.0),
        };
        check(a, b, &format!("geom[{i}]"));
    }
    // Inverted / zero-area boxes.
    for i in 0..10_000 {
        let m = rng.finite_v2();
        let b = match rng.below(2) {
            0 => C2Aabb {
                min: m,
                max: C2v {
                    x: m.x - rng.range(0.0, 5.0),
                    y: m.y - rng.range(0.0, 5.0),
                },
            },
            _ => C2Aabb { min: m, max: m },
        };
        let a = C2Circle {
            p: rng.finite_v2(),
            r: rng.range(0.0, 5.0),
        };
        check(a, b, &format!("degenerate-box[{i}]"));
    }
    // NaN in each of the 7 lanes.
    for lane in 0..7 {
        for &nan in &[NAN_A, NAN_B, NAN_C, SNAN] {
            let mut v = [1.0f32, 2.0, 3.0, -5.0, -5.0, 5.0, 5.0];
            v[lane] = nan;
            let a = C2Circle {
                p: C2v { x: v[0], y: v[1] },
                r: v[2],
            };
            let b = C2Aabb {
                min: C2v { x: v[3], y: v[4] },
                max: C2v { x: v[5], y: v[6] },
            };
            check(a, b, &format!("nan-lane{lane}"));
        }
    }
}

/// C09 — `c2AABBtoAABB`
#[test]
fn c09_c2aabbtoaabb() {
    let p = both();
    let mut rng = Rng::new(0xC09);

    let check = |a: C2Aabb, b: C2Aabb, ctx: &str| {
        let c = unsafe { (p.c.c2AABBtoAABB)(a, b) };
        let r = unsafe { (p.r.c2AABBtoAABB)(a, b) };
        eq_i32(&format!("c09 {ctx} A={a:?} B={b:?}"), c, r);
    };

    for i in 0..20_000 {
        check(rng.wild_aabb(), rng.wild_aabb(), &format!("wild[{i}]"));
    }
    // Integral coordinate lattice: separated on x / y / both / touching /
    // overlapping / contained — all with exact `==` boundaries.
    for ax in -3i32..=3 {
        for ay in -3i32..=3 {
            for bx in -3i32..=3 {
                for by in -3i32..=3 {
                    let a = C2Aabb {
                        min: C2v {
                            x: ax as f32,
                            y: ay as f32,
                        },
                        max: C2v {
                            x: ax as f32 + 2.0,
                            y: ay as f32 + 2.0,
                        },
                    };
                    let b = C2Aabb {
                        min: C2v {
                            x: bx as f32,
                            y: by as f32,
                        },
                        max: C2v {
                            x: bx as f32 + 1.0,
                            y: by as f32 + 1.0,
                        },
                    };
                    check(a, b, "lattice");
                }
            }
        }
    }
    // NaN in each of the 8 lanes.
    for lane in 0..8 {
        for &nan in &[NAN_A, NAN_B, NAN_C, SNAN] {
            let mut v = [0.0f32, 0.0, 1.0, 1.0, 0.5, 0.5, 2.0, 2.0];
            v[lane] = nan;
            let a = C2Aabb {
                min: C2v { x: v[0], y: v[1] },
                max: C2v { x: v[2], y: v[3] },
            };
            let b = C2Aabb {
                min: C2v { x: v[4], y: v[5] },
                max: C2v { x: v[6], y: v[7] },
            };
            check(a, b, &format!("nan-lane{lane}"));
        }
    }
}

// ===========================================================================
// C10–C15 — the `f2` dispatcher
// ===========================================================================

fn f2_call(l: &Lib, a: *const c_void, ta: i32, b: *const c_void, tb: i32) -> i32 {
    unsafe { (l.f2)(a, ta, b, tb) }
}

/// C10 — `f2(CIRCLE, CIRCLE)`
#[test]
fn c10_f2_circle_circle() {
    let p = both();
    let mut rng = Rng::new(0xC10);
    for i in 0..20_000 {
        let a = rng.wild_circle();
        let b = rng.wild_circle();
        let pa = &a as *const C2Circle as *const c_void;
        let pb = &b as *const C2Circle as *const c_void;
        let c = f2_call(&p.c, pa, C2_TYPE_CIRCLE, pb, C2_TYPE_CIRCLE);
        let r = f2_call(&p.r, pa, C2_TYPE_CIRCLE, pb, C2_TYPE_CIRCLE);
        eq_i32(&format!("c10[{i}] A={a:?} B={b:?}"), c, r);
        // Must agree with the direct low-level call too.
        let direct = unsafe { (p.c.c2CircletoCircle)(a, b) };
        eq_i32(&format!("c10[{i}] dispatch==direct"), direct, c);
    }
}

/// C11 — `f2(CIRCLE, AABB)`
#[test]
fn c11_f2_circle_aabb() {
    let p = both();
    let mut rng = Rng::new(0xC11);
    for i in 0..20_000 {
        let a = rng.wild_circle();
        let b = rng.wild_aabb();
        let pa = &a as *const C2Circle as *const c_void;
        let pb = &b as *const C2Aabb as *const c_void;
        let c = f2_call(&p.c, pa, C2_TYPE_CIRCLE, pb, C2_TYPE_AABB);
        let r = f2_call(&p.r, pa, C2_TYPE_CIRCLE, pb, C2_TYPE_AABB);
        eq_i32(&format!("c11[{i}] A={a:?} B={b:?}"), c, r);
        let direct = unsafe { (p.c.c2CircletoAABB)(a, b) };
        eq_i32(&format!("c11[{i}] dispatch==direct"), direct, c);
    }
}

/// C12 — `f2(AABB, CIRCLE)` — note the argument swap in the C source.
#[test]
fn c12_f2_aabb_circle() {
    let p = both();
    let mut rng = Rng::new(0xC12);
    for i in 0..20_000 {
        // A is the AABB, B is the circle.
        let a = rng.wild_aabb();
        let b = rng.wild_circle();
        let pa = &a as *const C2Aabb as *const c_void;
        let pb = &b as *const C2Circle as *const c_void;
        let c = f2_call(&p.c, pa, C2_TYPE_AABB, pb, C2_TYPE_CIRCLE);
        let r = f2_call(&p.r, pa, C2_TYPE_AABB, pb, C2_TYPE_CIRCLE);
        eq_i32(&format!("c12[{i}] A={a:?} B={b:?}"), c, r);
        let direct = unsafe { (p.c.c2CircletoAABB)(b, a) };
        eq_i32(&format!("c12[{i}] dispatch==direct(swapped)"), direct, c);
    }
}

/// C13 — `f2(AABB, AABB)`
#[test]
fn c13_f2_aabb_aabb() {
    let p = both();
    let mut rng = Rng::new(0xC13);
    for i in 0..20_000 {
        let a = rng.wild_aabb();
        let b = rng.wild_aabb();
        let pa = &a as *const C2Aabb as *const c_void;
        let pb = &b as *const C2Aabb as *const c_void;
        let c = f2_call(&p.c, pa, C2_TYPE_AABB, pb, C2_TYPE_AABB);
        let r = f2_call(&p.r, pa, C2_TYPE_AABB, pb, C2_TYPE_AABB);
        eq_i32(&format!("c13[{i}] A={a:?} B={b:?}"), c, r);
        let direct = unsafe { (p.c.c2AABBtoAABB)(a, b) };
        eq_i32(&format!("c13[{i}] dispatch==direct"), direct, c);
    }
}

/// C14 — `f2` with `A` and `B` aliasing the same buffer.
#[test]
fn c14_f2_aliased() {
    let p = both();
    let mut rng = Rng::new(0xC14);
    for i in 0..10_000 {
        // 16-byte buffer big enough for either shape.
        let buf: [f32; 4] = [
            rng.wild_f32(),
            rng.wild_f32(),
            rng.wild_f32(),
            rng.wild_f32(),
        ];
        let ptr = buf.as_ptr() as *const c_void;
        for (ta, tb) in [
            (C2_TYPE_CIRCLE, C2_TYPE_CIRCLE),
            (C2_TYPE_CIRCLE, C2_TYPE_AABB),
            (C2_TYPE_AABB, C2_TYPE_CIRCLE),
            (C2_TYPE_AABB, C2_TYPE_AABB),
        ] {
            let c = f2_call(&p.c, ptr, ta, ptr, tb);
            let r = f2_call(&p.r, ptr, ta, ptr, tb);
            eq_i32(&format!("c14[{i}] ({ta},{tb}) buf={buf:?}"), c, r);
        }
    }
}

/// C15 — `f2` with unaligned `A`/`B` pointers.
#[test]
fn c15_f2_unaligned() {
    let p = both();
    let mut rng = Rng::new(0xC15);
    for i in 0..4_000 {
        let mut raw = [0u8; 64];
        for b in raw.iter_mut() {
            *b = rng.next_u32() as u8;
        }
        for off_a in 0..8usize {
            for off_b in 0..8usize {
                let pa = unsafe { raw.as_ptr().add(off_a) } as *const c_void;
                let pb = unsafe { raw.as_ptr().add(16 + off_b) } as *const c_void;
                for (ta, tb) in [
                    (C2_TYPE_CIRCLE, C2_TYPE_CIRCLE),
                    (C2_TYPE_CIRCLE, C2_TYPE_AABB),
                    (C2_TYPE_AABB, C2_TYPE_CIRCLE),
                    (C2_TYPE_AABB, C2_TYPE_AABB),
                ] {
                    let c = f2_call(&p.c, pa, ta, pb, tb);
                    let r = f2_call(&p.r, pa, ta, pb, tb);
                    eq_i32(
                        &format!("c15[{i}] off=({off_a},{off_b}) types=({ta},{tb})"),
                        c,
                        r,
                    );
                }
            }
        }
    }
}

// ===========================================================================
// C16–C20 — `f3` (floor division)
// ===========================================================================

fn f3_row(p: &Pair, v1: i32, v2: i32, ctx: &str) {
    let c = unsafe { (p.c.f3)(v1, v2) };
    let r = unsafe { (p.r.f3)(v1, v2) };
    eq_i32(&format!("{ctx} f3({v1}, {v2})"), c, r);
}

/// C16 — `v1 >= 0, v2 > 0` (the `return v1/v2` fast path)
#[test]
fn c16_f3_pos_pos() {
    let p = both();
    let mut rng = Rng::new(0xC16);
    for i in 0..50_000 {
        let v1 = (rng.next_u32() >> 1) as i32;
        let v2 = ((rng.next_u32() >> 1) as i32).max(1);
        f3_row(&p, v1, v2, &format!("c16[{i}]"));
        // exact multiples
        let q = (rng.next_u32() % 1000) as i32;
        f3_row(&p, q.wrapping_mul(v2), v2, &format!("c16 exact[{i}]"));
    }
    for v1 in [0i32, 1, 2, 7, i32::MAX - 1, i32::MAX] {
        for v2 in [1i32, 2, 3, 7, i32::MAX] {
            f3_row(&p, v1, v2, "c16 edge");
        }
    }
}

/// C17 — `v1 >= 0, v2 < 0`
#[test]
fn c17_f3_pos_neg() {
    let p = both();
    let mut rng = Rng::new(0xC17);
    for i in 0..50_000 {
        let v1 = (rng.next_u32() >> 1) as i32;
        let v2 = -(((rng.next_u32() >> 1) as i32).max(1));
        f3_row(&p, v1, v2, &format!("c17[{i}]"));
        let q = (rng.next_u32() % 1000) as i32;
        f3_row(&p, q.wrapping_mul(v2.wrapping_neg()), v2, &format!("c17 exact[{i}]"));
    }
    for v1 in [0i32, 1, 2, 7, i32::MAX - 1, i32::MAX] {
        for v2 in [-1i32, -2, -3, -7, i32::MIN + 1, i32::MIN] {
            f3_row(&p, v1, v2, "c17 edge");
        }
    }
}

/// C18 — `v1 < 0 (!= INT_MIN), v2 > 0`
#[test]
fn c18_f3_neg_pos() {
    let p = both();
    let mut rng = Rng::new(0xC18);
    for i in 0..50_000 {
        let v1 = -(((rng.next_u32() >> 1) as i32).max(1));
        let v2 = ((rng.next_u32() >> 1) as i32).max(1);
        f3_row(&p, v1, v2, &format!("c18[{i}]"));
        let q = 1 + (rng.next_u32() % 1000) as i32;
        f3_row(&p, -(q.wrapping_mul(v2)), v2, &format!("c18 exact[{i}]"));
    }
    for v1 in [-1i32, -2, -7, i32::MIN + 1, i32::MIN] {
        for v2 in [1i32, 2, 3, 7, i32::MAX] {
            f3_row(&p, v1, v2, "c18 edge");
        }
    }
}

/// C19 — `v1 < 0 (!= INT_MIN), v2 < 0 (!= INT_MIN)`
#[test]
fn c19_f3_neg_neg() {
    let p = both();
    let mut rng = Rng::new(0xC19);
    for i in 0..50_000 {
        let v1 = -(((rng.next_u32() >> 1) as i32).max(1));
        let v2 = -(((rng.next_u32() >> 1) as i32).max(1));
        f3_row(&p, v1, v2, &format!("c19[{i}]"));
        let q = 1 + (rng.next_u32() % 1000) as i32;
        f3_row(&p, -(q.wrapping_mul(v2.wrapping_neg())), v2, &format!("c19 exact[{i}]"));
    }
    for v1 in [-1i32, -2, -7, i32::MIN + 1] {
        for v2 in [-1i32, -2, -3, -7, i32::MIN + 1] {
            f3_row(&p, v1, v2, "c19 edge");
        }
    }
}

/// C20 — fully random cross-product incl. all the extreme values
#[test]
fn c20_f3_random_all() {
    let p = both();
    let mut rng = Rng::new(0xC20);
    for i in 0..200_000 {
        f3_row(&p, rng.next_i32(), rng.next_i32(), &format!("c20 rand[{i}]"));
    }
    // Small dense sweep — catches every sign/remainder combination.
    for v1 in -40i32..=40 {
        for v2 in -40i32..=40 {
            f3_row(&p, v1, v2, "c20 dense");
        }
    }
    // Extremes cross-product.
    let ex = [
        i32::MIN,
        i32::MIN + 1,
        -3,
        -2,
        -1,
        0,
        1,
        2,
        3,
        i32::MAX - 1,
        i32::MAX,
    ];
    for &v1 in &ex {
        for &v2 in &ex {
            f3_row(&p, v1, v2, "c20 extremes");
        }
    }
    // Near-extreme magnitudes with small divisors and vice versa.
    for &v1 in &ex {
        for v2 in -8i32..=8 {
            f3_row(&p, v1, v2, "c20 mix1");
        }
    }
    for v1 in -8i32..=8 {
        for &v2 in &ex {
            f3_row(&p, v1, v2, "c20 mix2");
        }
    }
}

// ===========================================================================
// C21–C23 — `f4` (xorshift128+ RNG, mutates state in place)
// ===========================================================================

/// C21 — single call, random non-zero states
#[test]
fn c21_f4_single() {
    let p = both();
    let mut rng = Rng::new(0xC21);
    for i in 0..100_000 {
        let s = [rng.next_u64() | 1, rng.next_u64() | 1];
        let mut sc = CnRnd { state: s };
        let mut sr = CnRnd { state: s };
        let c = unsafe { (p.c.f4)(&mut sc) };
        let r = unsafe { (p.r.f4)(&mut sr) };
        eq_f64(&format!("c21[{i}] state={s:?}"), c, r);
        assert_eq!(
            sc.state, sr.state,
            "c21[{i}] state mutation differs: C={:?} Rust={:?}",
            sc.state, sr.state
        );
    }
}

/// C22 — sequences of successive calls on one state
#[test]
fn c22_f4_sequence() {
    let p = both();
    let mut rng = Rng::new(0xC22);
    for i in 0..2_000 {
        let s = [rng.next_u64(), rng.next_u64()];
        let mut sc = CnRnd { state: s };
        let mut sr = CnRnd { state: s };
        for step in 0..64 {
            let c = unsafe { (p.c.f4)(&mut sc) };
            let r = unsafe { (p.r.f4)(&mut sr) };
            eq_f64(&format!("c22[{i}] step {step} init={s:?}"), c, r);
            assert_eq!(
                sc.state, sr.state,
                "c22[{i}] step {step}: state C={:?} Rust={:?}",
                sc.state, sr.state
            );
        }
    }
}

/// C23 — edge-case states
#[test]
fn c23_f4_edge_states() {
    let p = both();
    let mut states: Vec<[u64; 2]> = vec![
        [0, 0],
        [0, 1],
        [1, 0],
        [1, 1],
        [u64::MAX, u64::MAX],
        [0, u64::MAX],
        [u64::MAX, 0],
    ];
    for bit in 0..64 {
        states.push([1u64 << bit, 0]);
        states.push([0, 1u64 << bit]);
        states.push([1u64 << bit, 1u64 << bit]);
        states.push([!(1u64 << bit), u64::MAX]);
    }
    for s in states {
        let mut sc = CnRnd { state: s };
        let mut sr = CnRnd { state: s };
        for step in 0..8 {
            let c = unsafe { (p.c.f4)(&mut sc) };
            let r = unsafe { (p.r.f4)(&mut sr) };
            eq_f64(&format!("c23 state={s:?} step {step}"), c, r);
            assert_eq!(sc.state, sr.state, "c23 state={s:?} step {step}");
        }
    }
}

// ===========================================================================
// C24–C25 — `f5` (16-bit reverse)
// ===========================================================================

/// C24 — exhaustive over the low 16 bits
#[test]
fn c24_f5_exhaustive_low16() {
    let p = both();
    for a in 0u32..=0xFFFF {
        let c = unsafe { (p.c.f5)(a) };
        let r = unsafe { (p.r.f5)(a) };
        eq_u32(&format!("c24 f5({a:#x})"), c, r);
    }
}

/// C25 — random full 32-bit values (high half must be discarded)
#[test]
fn c25_f5_random_u32() {
    let p = both();
    let mut rng = Rng::new(0xC25);
    for i in 0..200_000 {
        let a = rng.next_u32();
        let c = unsafe { (p.c.f5)(a) };
        let r = unsafe { (p.r.f5)(a) };
        eq_u32(&format!("c25[{i}] f5({a:#x})"), c, r);
    }
    for a in [0u32, u32::MAX, 0xFFFF_0000, 0x0000_FFFF, 0x8000_0001] {
        let c = unsafe { (p.c.f5)(a) };
        let r = unsafe { (p.r.f5)(a) };
        eq_u32(&format!("c25 edge f5({a:#x})"), c, r);
    }
}

// ===========================================================================
// C26–C30 — `f7` (tflac frame-size estimate)
// ===========================================================================

fn f7_row(p: &Pair, bs: u32, ch: u32, bd: u32, ctx: &str) {
    let c = unsafe { (p.c.f7)(bs, ch, bd) };
    let r = unsafe { (p.r.f7)(bs, ch, bd) };
    eq_u32(&format!("{ctx} f7({bs}, {ch}, {bd})"), c, r);
}

/// C26 — `channels == 2`, `bitdepth == 32`
#[test]
fn c26_f7_ch2_bd32() {
    let p = both();
    let mut rng = Rng::new(0xC26);
    for i in 0..50_000 {
        f7_row(&p, rng.next_u32(), 2, 32, &format!("c26[{i}]"));
    }
    for bs in [0u32, 1, 7, 8, 4096, 65535, u32::MAX / 32, u32::MAX] {
        f7_row(&p, bs, 2, 32, "c26 edge");
    }
}

/// C27 — `channels == 2`, `bitdepth != 32`
#[test]
fn c27_f7_ch2_bdother() {
    let p = both();
    let mut rng = Rng::new(0xC27);
    for i in 0..50_000 {
        let bd = loop {
            let v = rng.next_u32();
            if v != 32 {
                break v;
            }
        };
        f7_row(&p, rng.next_u32(), 2, bd, &format!("c27[{i}]"));
    }
    for bd in [0u32, 1, 8, 12, 16, 20, 24, 31, 33, u32::MAX] {
        for bs in [0u32, 1, 4096, u32::MAX] {
            f7_row(&p, bs, 2, bd, "c27 edge");
        }
    }
}

/// C28 — `channels != 2`, `bitdepth == 32`
#[test]
fn c28_f7_chother_bd32() {
    let p = both();
    let mut rng = Rng::new(0xC28);
    for i in 0..50_000 {
        let ch = loop {
            let v = rng.next_u32();
            if v != 2 {
                break v;
            }
        };
        f7_row(&p, rng.next_u32(), ch, 32, &format!("c28[{i}]"));
    }
    for ch in [0u32, 1, 3, 4, 8, u32::MAX] {
        for bs in [0u32, 1, 4096, u32::MAX] {
            f7_row(&p, bs, ch, 32, "c28 edge");
        }
    }
}

/// C29 — `channels != 2`, `bitdepth != 32`
#[test]
fn c29_f7_chother_bdother() {
    let p = both();
    let mut rng = Rng::new(0xC29);
    for i in 0..50_000 {
        let ch = loop {
            let v = rng.next_u32();
            if v != 2 {
                break v;
            }
        };
        let bd = loop {
            let v = rng.next_u32();
            if v != 32 {
                break v;
            }
        };
        f7_row(&p, rng.next_u32(), ch, bd, &format!("c29[{i}]"));
    }
    for ch in [0u32, 1, 3, 4, 8, u32::MAX] {
        for bd in [0u32, 1, 8, 16, 24, 31, 33, u32::MAX] {
            for bs in [0u32, 1, 4096, u32::MAX] {
                f7_row(&p, bs, ch, bd, "c29 edge");
            }
        }
    }
}

/// C30 — fully random (overflowing) triples + a dense small sweep
#[test]
fn c30_f7_random_all() {
    let p = both();
    let mut rng = Rng::new(0xC30);
    for i in 0..200_000 {
        f7_row(
            &p,
            rng.next_u32(),
            rng.next_u32(),
            rng.next_u32(),
            &format!("c30[{i}]"),
        );
    }
    for bs in [0u32, 1, 2, 3, 1024, 4096, 0xFFFF, 0xFFFF_FFFF] {
        for ch in 0u32..=9 {
            for bd in [0u32, 1, 4, 8, 12, 16, 20, 24, 31, 32, 33, 64, 0xFFFF_FFFF] {
                f7_row(&p, bs, ch, bd, "c30 sweep");
            }
        }
    }
}

// ===========================================================================
// C31–C34 — `f9` (barycentric coordinates)
// ===========================================================================

fn f9_row(p: &Pair, p1: LmVec2, p2: LmVec2, p3: LmVec2, pt: LmVec2, ctx: &str) {
    let c = unsafe { (p.c.f9)(p1, p2, p3, pt) };
    let r = unsafe { (p.r.f9)(p1, p2, p3, pt) };
    eq_lm(
        &format!("{ctx} f9(p1={p1:?}, p2={p2:?}, p3={p3:?}, p={pt:?})"),
        c,
        r,
    );
}

/// C31 — non-degenerate triangle, `p` strictly inside
#[test]
fn c31_f9_inside() {
    let p = both();
    let mut rng = Rng::new(0xC31);
    for i in 0..50_000 {
        let p1 = LmVec2 {
            x: rng.range(-10.0, 10.0),
            y: rng.range(-10.0, 10.0),
        };
        let p2 = LmVec2 {
            x: p1.x + rng.range(1.0, 10.0),
            y: p1.y + rng.range(0.1, 10.0),
        };
        let p3 = LmVec2 {
            x: p1.x - rng.range(1.0, 10.0),
            y: p1.y + rng.range(1.0, 10.0),
        };
        // barycentric-ish interior point
        let (a, b) = (rng.unit() * 0.5, rng.unit() * 0.5);
        let pt = LmVec2 {
            x: p1.x + a * (p2.x - p1.x) + b * (p3.x - p1.x),
            y: p1.y + a * (p2.y - p1.y) + b * (p3.y - p1.y),
        };
        f9_row(&p, p1, p2, p3, pt, &format!("c31[{i}]"));
    }
}

/// C32 — non-degenerate triangle, `p` outside
#[test]
fn c32_f9_outside() {
    let p = both();
    let mut rng = Rng::new(0xC32);
    for i in 0..50_000 {
        let p1 = LmVec2 {
            x: rng.range(-10.0, 10.0),
            y: rng.range(-10.0, 10.0),
        };
        let p2 = LmVec2 {
            x: p1.x + rng.range(1.0, 10.0),
            y: p1.y + rng.range(0.1, 10.0),
        };
        let p3 = LmVec2 {
            x: p1.x - rng.range(1.0, 10.0),
            y: p1.y + rng.range(1.0, 10.0),
        };
        let (a, b) = (rng.range(-3.0, 3.0), rng.range(-3.0, 3.0));
        let pt = LmVec2 {
            x: p1.x + a * (p2.x - p1.x) + b * (p3.x - p1.x),
            y: p1.y + a * (p2.y - p1.y) + b * (p3.y - p1.y),
        };
        f9_row(&p, p1, p2, p3, pt, &format!("c32[{i}]"));
    }
}

/// C33 — degenerate triangles (`invDenom` = ±inf) and `p == p1`
#[test]
fn c33_f9_degenerate() {
    let p = both();
    let mut rng = Rng::new(0xC33);
    for i in 0..30_000 {
        let p1 = LmVec2 {
            x: rng.range(-10.0, 10.0),
            y: rng.range(-10.0, 10.0),
        };
        let d = LmVec2 {
            x: rng.range(-5.0, 5.0),
            y: rng.range(-5.0, 5.0),
        };
        let t2 = rng.range(-4.0, 4.0);
        let t3 = rng.range(-4.0, 4.0);
        match rng.below(4) {
            // colinear
            0 => f9_row(
                &p,
                p1,
                LmVec2 {
                    x: p1.x + t2 * d.x,
                    y: p1.y + t2 * d.y,
                },
                LmVec2 {
                    x: p1.x + t3 * d.x,
                    y: p1.y + t3 * d.y,
                },
                rng.wild_lm(),
                &format!("c33 colinear[{i}]"),
            ),
            // all three identical
            1 => f9_row(&p, p1, p1, p1, rng.wild_lm(), &format!("c33 same[{i}]")),
            // p2 == p1
            2 => f9_row(
                &p,
                p1,
                p1,
                LmVec2 {
                    x: p1.x + t3 * d.x,
                    y: p1.y + t3 * d.y,
                },
                p1,
                &format!("c33 p2==p1[{i}]"),
            ),
            // p3 == p1
            _ => f9_row(
                &p,
                p1,
                LmVec2 {
                    x: p1.x + t2 * d.x,
                    y: p1.y + t2 * d.y,
                },
                p1,
                p1,
                &format!("c33 p3==p1[{i}]"),
            ),
        }
    }
    // Zero-size triangle at the origin.
    let z = LmVec2 { x: 0.0, y: 0.0 };
    f9_row(&p, z, z, z, z, "c33 origin");
}

/// C34 — fully random `f32` bit patterns in all 8 lanes
#[test]
fn c34_f9_random_bits() {
    let p = both();
    let mut rng = Rng::new(0xC34);
    for i in 0..100_000 {
        f9_row(
            &p,
            rng.wild_lm(),
            rng.wild_lm(),
            rng.wild_lm(),
            rng.wild_lm(),
            &format!("c34[{i}]"),
        );
    }
    // NaN in every lane with distinct payloads (payload-survivor coverage).
    for i in 0..20_000 {
        let mk = |rng: &mut Rng| LmVec2 {
            x: rng.nan_payload(),
            y: rng.nan_payload(),
        };
        let (a, b, c, d) = (mk(&mut rng), mk(&mut rng), mk(&mut rng), mk(&mut rng));
        f9_row(&p, a, b, c, d, &format!("c34 all-nan[{i}]"));
    }
    // Single NaN lane at a time.
    for lane in 0..8 {
        for &nan in &[NAN_A, NAN_B, NAN_C, SNAN] {
            let mut v = [0.0f32, 0.0, 1.0, 0.0, 0.0, 1.0, 0.25, 0.25];
            v[lane] = nan;
            f9_row(
                &p,
                LmVec2 { x: v[0], y: v[1] },
                LmVec2 { x: v[2], y: v[3] },
                LmVec2 { x: v[4], y: v[5] },
                LmVec2 { x: v[6], y: v[7] },
                &format!("c34 nan-lane{lane}"),
            );
        }
    }
}

// ===========================================================================
// C35 — `f10` (half-float decode), exhaustive
// ===========================================================================

/// C35 — all 65 536 `uint16_t` inputs
#[test]
fn c35_f10_exhaustive() {
    let p = both();
    for h in 0u32..=0xFFFF {
        let h = h as u16;
        let c = unsafe { (p.c.f10)(h) };
        let r = unsafe { (p.r.f10)(h) };
        eq_f32(&format!("c35 f10({h:#06x}) [row {}]", h >> 10), c, r);
    }
}

// ===========================================================================
// C36–C46 — `f11` (HSL -> RGB)
// ===========================================================================

fn color_row(p: &Pair, which: Which, src: [f32; 3], ctx: &str) {
    let c = p.c.color(which, src);
    let r = p.r.color(which, src);
    eq_arr3(&format!("{ctx} {}({src:?})", which.name()), c, r);
}

fn color_row_inplace(p: &Pair, which: Which, src: [f32; 3], ctx: &str) {
    let c = p.c.color_inplace(which, src);
    let r = p.r.color_inplace(which, src);
    eq_arr3(
        &format!("{ctx} {}(in-place {src:?})", which.name()),
        c,
        r,
    );
}

/// C36 — `f11` with `s == 0` (early-return path)
#[test]
fn c36_f11_s_zero() {
    let p = both();
    let mut rng = Rng::new(0xC36);
    for i in 0..30_000 {
        for &s in &[0.0f32, -0.0] {
            color_row(&p, Which::F11, [rng.wild_f32(), s, rng.wild_f32()], &format!("c36[{i}]"));
        }
    }
    for &h in SPECIAL_F32 {
        for &l in SPECIAL_F32 {
            color_row(&p, Which::F11, [h, 0.0, l], "c36 special");
            color_row(&p, Which::F11, [h, -0.0, l], "c36 special-neg0");
        }
    }
}

fn f11_band(p: &Pair, rng: &mut Rng, lo: f32, hi: f32, n: usize, tag: &str) {
    for i in 0..n {
        let h = rng.range(lo, hi);
        // `s` must be non-zero to reach the band code.
        let s = match rng.below(4) {
            0 => rng.range(0.0001, 1.0),
            1 => rng.range(-5.0, 5.0),
            2 => rng.range(1.0, 100.0),
            _ => rng.unit().max(f32::MIN_POSITIVE),
        };
        let l = match rng.below(4) {
            0 => rng.unit(),
            1 => rng.range(-2.0, 3.0),
            2 => rng.range(-1.0e6, 1.0e6),
            _ => rng.range(0.0, 1.0),
        };
        if s == 0.0 {
            continue;
        }
        color_row(p, Which::F11, [h, s, l], &format!("{tag}[{i}]"));
    }
}

/// C37 — band 1: `0 <= h < 60`
#[test]
fn c37_f11_band1() {
    let p = both();
    let mut rng = Rng::new(0xC37);
    f11_band(&p, &mut rng, 0.0, 60.0, 40_000, "c37");
}

/// C38 — band 2: `60 <= h < 120`
#[test]
fn c38_f11_band2() {
    let p = both();
    let mut rng = Rng::new(0xC38);
    f11_band(&p, &mut rng, 60.0, 120.0, 40_000, "c38");
}

/// C39 — band 3 arm (`h < 120 && h < 180`), reachable only via `h < 0`
#[test]
fn c39_f11_band3_negative_h() {
    let p = both();
    let mut rng = Rng::new(0xC39);
    f11_band(&p, &mut rng, -1.0e6, -1.0e-6, 40_000, "c39");
    f11_band(&p, &mut rng, -720.0, 0.0, 20_000, "c39 small");
    for &h in &[-0.0f32, -1.0, -60.0, -119.0, -180.0, -360.0, f32::NEG_INFINITY] {
        for &s in &[0.25f32, 1.0, -1.0] {
            for &l in &[0.0f32, 0.25, 0.5, 0.75, 1.0] {
                color_row(&p, Which::F11, [h, s, l], "c39 edge");
            }
        }
    }
}

/// C40 — band 4: `180 <= h < 240`
#[test]
fn c40_f11_band4() {
    let p = both();
    let mut rng = Rng::new(0xC40);
    f11_band(&p, &mut rng, 180.0, 240.0, 40_000, "c40");
}

/// C41 — band 5: `240 <= h < 300`
#[test]
fn c41_f11_band5() {
    let p = both();
    let mut rng = Rng::new(0xC41);
    f11_band(&p, &mut rng, 240.0, 300.0, 40_000, "c41");
}

/// C42 — band 6: `300 <= h < 360`
#[test]
fn c42_f11_band6() {
    let p = both();
    let mut rng = Rng::new(0xC42);
    f11_band(&p, &mut rng, 300.0, 360.0, 40_000, "c42");
}

/// C43 — `else` arm: `h >= 360`, `+inf`, `NaN`
#[test]
fn c43_f11_else_arm() {
    let p = both();
    let mut rng = Rng::new(0xC43);
    f11_band(&p, &mut rng, 360.0, 1.0e6, 40_000, "c43");
    for &h in &[360.0f32, 360.0001, 720.0, 1.0e30, f32::MAX, f32::INFINITY, NAN_A, NAN_B, NAN_C, SNAN] {
        for &s in &[0.25f32, 1.0, -1.0, f32::INFINITY, NAN_A] {
            for &l in &[0.0f32, 0.5, 1.0, NAN_B, f32::INFINITY] {
                color_row(&p, Which::F11, [h, s, l], "c43 edge");
            }
        }
    }
    // h, s, l all distinct NaN payloads — payload-survivor coverage.
    for i in 0..20_000 {
        let src = [rng.nan_payload(), rng.nan_payload(), rng.nan_payload()];
        color_row(&p, Which::F11, src, &format!("c43 all-nan[{i}]"));
    }
}

/// C44 — exact band boundaries × `l` × `s` grid
#[test]
fn c44_f11_boundaries() {
    let p = both();
    let hs = [
        -360.0f32, -180.0, -60.0, -0.0, 0.0, 59.999996, 60.0, 60.000004, 119.99999, 120.0,
        120.00001, 179.99998, 180.0, 180.00002, 239.99998, 240.0, 240.00003, 299.99997, 300.0,
        300.00003, 359.99997, 360.0, 360.00003,
    ];
    let ss = [
        f32::MIN_POSITIVE,
        1.0e-30,
        0.001,
        0.5,
        1.0,
        2.0,
        -1.0,
        f32::MAX,
    ];
    let ls = [
        -1.0f32,
        -0.5,
        -0.0,
        0.0,
        0.25,
        0.5,
        0.75,
        1.0,
        1.5,
        f32::MAX,
        f32::MIN_POSITIVE,
    ];
    for &h in &hs {
        for &s in &ss {
            for &l in &ls {
                color_row(&p, Which::F11, [h, s, l], "c44");
            }
        }
    }
}

/// C45 — fully random `f32` bit patterns
#[test]
fn c45_f11_random_bits() {
    let p = both();
    let mut rng = Rng::new(0xC45);
    for i in 0..200_000 {
        let src = [rng.wild_f32(), rng.wild_f32(), rng.wild_f32()];
        color_row(&p, Which::F11, src, &format!("c45[{i}]"));
    }
}

/// C46 — `dest` aliasing `src`
#[test]
fn c46_f11_aliased() {
    let p = both();
    let mut rng = Rng::new(0xC46);
    for i in 0..100_000 {
        let src = [rng.wild_f32(), rng.wild_f32(), rng.wild_f32()];
        color_row_inplace(&p, Which::F11, src, &format!("c46[{i}]"));
    }
    for i in 0..50_000 {
        let src = [rng.range(-720.0, 720.0), rng.unit(), rng.unit()];
        color_row_inplace(&p, Which::F11, src, &format!("c46 sane[{i}]"));
    }
}

// ===========================================================================
// C47–C57 — `f12` (HSV -> RGB)
// ===========================================================================

fn f12_sector(p: &Pair, rng: &mut Rng, lo: f32, hi: f32, n: usize, tag: &str) {
    for i in 0..n {
        let h = rng.range(lo, hi);
        let s = match rng.below(3) {
            0 => rng.range(0.0001, 1.0),
            1 => rng.range(-5.0, 5.0),
            _ => rng.range(1.0, 100.0),
        };
        if s == 0.0 {
            continue;
        }
        let v = match rng.below(3) {
            0 => rng.unit(),
            1 => rng.range(-10.0, 10.0),
            _ => rng.range(-1.0e6, 1.0e6),
        };
        color_row(p, Which::F12, [h, s, v], &format!("{tag}[{i}]"));
    }
}

/// C47 — `f12` with `s == 0`
#[test]
fn c47_f12_s_zero() {
    let p = both();
    let mut rng = Rng::new(0xC47);
    for i in 0..30_000 {
        for &s in &[0.0f32, -0.0] {
            color_row(&p, Which::F12, [rng.wild_f32(), s, rng.wild_f32()], &format!("c47[{i}]"));
        }
    }
    for &h in SPECIAL_F32 {
        for &v in SPECIAL_F32 {
            color_row(&p, Which::F12, [h, 0.0, v], "c47 special");
            color_row(&p, Which::F12, [h, -0.0, v], "c47 special-neg0");
        }
    }
}

/// C48 — sector `i == 0`
#[test]
fn c48_f12_sector0() {
    let p = both();
    let mut rng = Rng::new(0xC48);
    f12_sector(&p, &mut rng, 0.0, 60.0, 40_000, "c48");
}

/// C49 — sector `i == 1`
#[test]
fn c49_f12_sector1() {
    let p = both();
    let mut rng = Rng::new(0xC49);
    f12_sector(&p, &mut rng, 60.0, 120.0, 40_000, "c49");
}

/// C50 — sector `i == 2`
#[test]
fn c50_f12_sector2() {
    let p = both();
    let mut rng = Rng::new(0xC50);
    f12_sector(&p, &mut rng, 120.0, 180.0, 40_000, "c50");
}

/// C51 — sector `i == 3`
#[test]
fn c51_f12_sector3() {
    let p = both();
    let mut rng = Rng::new(0xC51);
    f12_sector(&p, &mut rng, 180.0, 240.0, 40_000, "c51");
}

/// C52 — sector `i == 4`
#[test]
fn c52_f12_sector4() {
    let p = both();
    let mut rng = Rng::new(0xC52);
    f12_sector(&p, &mut rng, 240.0, 300.0, 40_000, "c52");
}

/// C53 — `default` sector: `i == 5`, `i > 5`, `i < 0`
#[test]
fn c53_f12_sector_default() {
    let p = both();
    let mut rng = Rng::new(0xC53);
    f12_sector(&p, &mut rng, 300.0, 360.0, 30_000, "c53 i=5");
    f12_sector(&p, &mut rng, 360.0, 100_000.0, 30_000, "c53 i>5");
    f12_sector(&p, &mut rng, -100_000.0, -1.0e-6, 30_000, "c53 i<0");
    for &h in &[
        -1.0e6f32, -720.0, -360.0, -60.0, -1.0, -0.0, 300.0, 359.0, 360.0, 420.0, 1.0e6,
    ] {
        for &s in &[0.5f32, 1.0, -1.0] {
            for &v in &[0.0f32, 0.5, 1.0, -1.0] {
                color_row(&p, Which::F12, [h, s, v], "c53 edge");
            }
        }
    }
}

/// C54 — `(int)floorf(h/60)` out of `int` range or `NaN` (cvttss2si indefinite)
#[test]
fn c54_f12_int_indefinite() {
    let p = both();
    let mut rng = Rng::new(0xC54);
    let hs = [
        f32::NAN,
        NAN_A,
        NAN_B,
        NAN_C,
        SNAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::MAX,
        f32::MIN,
        1.0e30,
        -1.0e30,
        // exactly at the conversion boundary: h/60 == 2^31
        2147483648.0 * 60.0,
        -2147483648.0 * 60.0,
        // one ulp below / above 2^31 after the divide
        2147483520.0 * 60.0,
        -2147483904.0 * 60.0,
        1.28849018e11, // ~2^31 * 60
    ];
    for &h in &hs {
        for &s in &[0.5f32, 1.0, -1.0, f32::INFINITY, NAN_A] {
            for &v in &[0.0f32, 0.5, 1.0, NAN_B, f32::INFINITY] {
                color_row(&p, Which::F12, [h, s, v], "c54");
            }
        }
    }
    // h, s, v all distinct NaN payloads (both `s` and `1-f` NaN ⇒ pins the
    // payload survivor inside `t = v * (1 - s*(1-f))`).
    for i in 0..40_000 {
        let src = [rng.nan_payload(), rng.nan_payload(), rng.nan_payload()];
        color_row(&p, Which::F12, src, &format!("c54 all-nan[{i}]"));
    }
    // Very large finite h ⇒ h/60 huge but in range; and h/60 == ±inf.
    for i in 0..40_000 {
        let h = f32::from_bits(0x4f00_0000 | (rng.next_u32() & 0x0fff_ffff));
        color_row(&p, Which::F12, [h, rng.unit() + 0.001, rng.unit()], &format!("c54 big[{i}]"));
    }
}

/// C55 — exact sector boundaries × grid
#[test]
fn c55_f12_boundaries() {
    let p = both();
    let hs = [
        -360.0f32, -60.0, -1.0e-6, -0.0, 0.0, 1.0e-6, 59.999996, 60.0, 60.000004, 119.99999,
        120.0, 120.00001, 179.99998, 180.0, 180.00002, 239.99998, 240.0, 240.00003, 299.99997,
        300.0, 300.00003, 359.99997, 360.0, 360.00003,
    ];
    let ss = [
        f32::MIN_POSITIVE,
        1.0e-30,
        0.001,
        0.5,
        1.0,
        2.0,
        -1.0,
        f32::MAX,
        f32::INFINITY,
    ];
    let vs = [
        -1.0f32,
        -0.0,
        0.0,
        0.25,
        1.0,
        2.0,
        f32::MAX,
        f32::MIN_POSITIVE,
        f32::INFINITY,
    ];
    for &h in &hs {
        for &s in &ss {
            for &v in &vs {
                color_row(&p, Which::F12, [h, s, v], "c55");
            }
        }
    }
}

/// C56 — fully random `f32` bit patterns
#[test]
fn c56_f12_random_bits() {
    let p = both();
    let mut rng = Rng::new(0xC56);
    for i in 0..200_000 {
        let src = [rng.wild_f32(), rng.wild_f32(), rng.wild_f32()];
        color_row(&p, Which::F12, src, &format!("c56[{i}]"));
    }
}

/// C57 — `dest` aliasing `src`
#[test]
fn c57_f12_aliased() {
    let p = both();
    let mut rng = Rng::new(0xC57);
    for i in 0..100_000 {
        let src = [rng.wild_f32(), rng.wild_f32(), rng.wild_f32()];
        color_row_inplace(&p, Which::F12, src, &format!("c57[{i}]"));
    }
    for i in 0..50_000 {
        let src = [rng.range(-720.0, 720.0), rng.unit(), rng.unit()];
        color_row_inplace(&p, Which::F12, src, &format!("c57 sane[{i}]"));
    }
}

// ===========================================================================
// C58–C65 — `f13` (RGB -> HSV)
// ===========================================================================

/// C58 — `r` is the strict maximum (both `g > b` and `g < b`)
#[test]
fn c58_f13_r_max() {
    let p = both();
    let mut rng = Rng::new(0xC58);
    for i in 0..60_000 {
        let r = rng.range(0.001, 100.0);
        let g = rng.range(-100.0, r);
        let b = rng.range(-100.0, r);
        color_row(&p, Which::F13, [r, g, b], &format!("c58[{i}]"));
    }
    // Force the `h < 0` wrap: r == max and g < b.
    for i in 0..30_000 {
        let r = rng.range(0.001, 10.0);
        let b = rng.range(-10.0, r);
        let g = rng.range(-10.0, b);
        color_row(&p, Which::F13, [r, g, b], &format!("c58 wrap[{i}]"));
    }
}

/// C59 — `g` is the strict maximum
#[test]
fn c59_f13_g_max() {
    let p = both();
    let mut rng = Rng::new(0xC59);
    for i in 0..60_000 {
        let g = rng.range(0.001, 100.0);
        let r = rng.range(-100.0, g);
        let b = rng.range(-100.0, g);
        color_row(&p, Which::F13, [r, g, b], &format!("c59[{i}]"));
    }
}

/// C60 — `b` is the strict maximum
#[test]
fn c60_f13_b_max() {
    let p = both();
    let mut rng = Rng::new(0xC60);
    for i in 0..60_000 {
        let b = rng.range(0.001, 100.0);
        let r = rng.range(-100.0, b);
        let g = rng.range(-100.0, b);
        color_row(&p, Which::F13, [r, g, b], &format!("c60[{i}]"));
    }
}

/// C61 — ties between channels, incl. `delta == 0`
#[test]
fn c61_f13_ties() {
    let p = both();
    let mut rng = Rng::new(0xC61);
    for i in 0..40_000 {
        let hi = rng.range(-10.0, 10.0);
        let lo = hi - rng.range(0.0, 10.0);
        for src in [
            [hi, hi, lo], // r == g > b
            [lo, hi, hi], // g == b > r
            [hi, lo, hi], // r == b > g
            [hi, hi, hi], // delta == 0
        ] {
            color_row(&p, Which::F13, src, &format!("c61[{i}]"));
        }
    }
    // Integral lattice: every ordering and tie pattern.
    for r in -2i32..=2 {
        for g in -2i32..=2 {
            for b in -2i32..=2 {
                color_row(
                    &p,
                    Which::F13,
                    [r as f32, g as f32, b as f32],
                    "c61 lattice",
                );
            }
        }
    }
}

/// C62 — `max == 0` (incl. all-negative inputs and `-0.0` mixtures)
#[test]
fn c62_f13_max_zero() {
    let p = both();
    let mut rng = Rng::new(0xC62);
    color_row(&p, Which::F13, [0.0, 0.0, 0.0], "c62 zeros");
    for signs in 0..8u32 {
        let z = |bit: u32| if signs & (1 << bit) != 0 { -0.0f32 } else { 0.0f32 };
        color_row(&p, Which::F13, [z(0), z(1), z(2)], "c62 signed-zeros");
    }
    for i in 0..40_000 {
        let src = [
            -rng.range(0.0, 100.0),
            -rng.range(0.0, 100.0),
            -rng.range(0.0, 100.0),
        ];
        color_row(&p, Which::F13, src, &format!("c62 all-neg[{i}]"));
    }
    // max is exactly -0.0 with a strictly negative minimum ⇒ delta != 0 but
    // `max == 0` still triggers the early return.
    for i in 0..20_000 {
        let n = -rng.range(0.001, 100.0);
        for src in [[-0.0f32, n, n], [n, -0.0, n], [n, n, -0.0], [0.0, n, n]] {
            color_row(&p, Which::F13, src, &format!("c62 neg-zero-max[{i}]"));
        }
    }
}

/// C63 — out-of-gamut values, `±inf`, subnormals
#[test]
fn c63_f13_out_of_gamut() {
    let p = both();
    let vals = [
        -1.0e30f32,
        -100.0,
        -1.0,
        -f32::MIN_POSITIVE,
        -0.0,
        0.0,
        f32::MIN_POSITIVE,
        1.0e-30,
        1.0,
        100.0,
        1.0e30,
        f32::MAX,
        f32::MIN,
        f32::INFINITY,
        f32::NEG_INFINITY,
    ];
    for &r in &vals {
        for &g in &vals {
            for &b in &vals {
                color_row(&p, Which::F13, [r, g, b], "c63");
            }
        }
    }
}

/// C64 — fully random `f32` bit patterns, incl. `NaN` in 1/2/3 lanes
#[test]
fn c64_f13_random_bits() {
    let p = both();
    let mut rng = Rng::new(0xC64);
    for i in 0..200_000 {
        let src = [rng.wild_f32(), rng.wild_f32(), rng.wild_f32()];
        color_row(&p, Which::F13, src, &format!("c64[{i}]"));
    }
    // Exactly one NaN lane.
    for lane in 0..3 {
        for &nan in &[NAN_A, NAN_B, NAN_C, SNAN] {
            for i in 0..2_000 {
                let mut src = [rng.finite(), rng.finite(), rng.finite()];
                src[lane] = nan;
                color_row(&p, Which::F13, src, &format!("c64 nan-lane{lane}[{i}]"));
            }
        }
    }
    // Two and three NaN lanes with distinct payloads.
    for i in 0..40_000 {
        let src = [rng.nan_payload(), rng.nan_payload(), rng.nan_payload()];
        color_row(&p, Which::F13, src, &format!("c64 all-nan[{i}]"));
    }
    for i in 0..20_000 {
        let src = [rng.nan_payload(), rng.nan_payload(), rng.finite()];
        color_row(&p, Which::F13, src, &format!("c64 two-nan[{i}]"));
    }
}

/// C65 — `dest` aliasing `src`
#[test]
fn c65_f13_aliased() {
    let p = both();
    let mut rng = Rng::new(0xC65);
    for i in 0..100_000 {
        let src = [rng.wild_f32(), rng.wild_f32(), rng.wild_f32()];
        color_row_inplace(&p, Which::F13, src, &format!("c65[{i}]"));
    }
    for i in 0..50_000 {
        let src = [rng.unit(), rng.unit(), rng.unit()];
        color_row_inplace(&p, Which::F13, src, &format!("c65 sane[{i}]"));
    }
}

// ===========================================================================
// C66–C67 — composed pipelines across low-level entry points
// ===========================================================================

/// C66 — `f13` then `f12` round-trip, driven end to end through both `.so`s
#[test]
fn c66_f13_f12_roundtrip() {
    let p = both();
    let mut rng = Rng::new(0xC66);
    for i in 0..100_000 {
        let rgb = match rng.below(3) {
            0 => [rng.unit(), rng.unit(), rng.unit()],
            1 => [rng.finite(), rng.finite(), rng.finite()],
            _ => [rng.wild_f32(), rng.wild_f32(), rng.wild_f32()],
        };
        let hsv_c = p.c.color(Which::F13, rgb);
        let hsv_r = p.r.color(Which::F13, rgb);
        eq_arr3(&format!("c66[{i}] f13({rgb:?})"), hsv_c, hsv_r);
        // Feed each library its OWN intermediate (they are bit-identical) as
        // well as the cross product, so a divergence cannot hide.
        let rgb_c = p.c.color(Which::F12, hsv_c);
        let rgb_r = p.r.color(Which::F12, hsv_r);
        eq_arr3(&format!("c66[{i}] f12(f13({rgb:?}))"), rgb_c, rgb_r);
    }
}

/// C67 — `f13` output fed into `f11` (exercises out-of-range `h` into the bands)
#[test]
fn c67_f13_f11_pipeline() {
    let p = both();
    let mut rng = Rng::new(0xC67);
    for i in 0..100_000 {
        let rgb = match rng.below(3) {
            0 => [rng.unit(), rng.unit(), rng.unit()],
            1 => [rng.finite(), rng.finite(), rng.finite()],
            _ => [rng.wild_f32(), rng.wild_f32(), rng.wild_f32()],
        };
        let hsv_c = p.c.color(Which::F13, rgb);
        let hsv_r = p.r.color(Which::F13, rgb);
        eq_arr3(&format!("c67[{i}] f13({rgb:?})"), hsv_c, hsv_r);
        let out_c = p.c.color(Which::F11, hsv_c);
        let out_r = p.r.color(Which::F11, hsv_r);
        eq_arr3(&format!("c67[{i}] f11(f13({rgb:?}))"), out_c, out_r);
    }
}

// ===========================================================================
// C68–C70 — `agglom`, the aggregate one-shot entry point
// ===========================================================================

/// C68 — all 33 parameters fully random bit patterns
#[test]
fn c68_agglom_random() {
    let p = both();
    let mut rng = Rng::new(0xC68);
    for i in 0..200_000 {
        let a = rng.wild_agglom();
        let c = p.c.call_agglom(&a);
        let r = p.r.call_agglom(&a);
        eq_f64(&format!("c68[{i}] {a:?}"), c, r);
    }
}

/// C69 — "sane" randomized inputs
#[test]
fn c69_agglom_sane() {
    let p = both();
    let mut rng = Rng::new(0xC69);
    for i in 0..200_000 {
        let a = rng.sane_agglom();
        let c = p.c.call_agglom(&a);
        let r = p.r.call_agglom(&a);
        eq_f64(&format!("c69[{i}] {a:?}"), c, r);
    }
}

/// C70 — boundary matrix over the discrete axes
#[test]
fn c70_agglom_boundaries() {
    let p = both();
    let mut rng = Rng::new(0xC70);
    let base = rng.sane_agglom();
    for &f3_2 in &[0i32, 1, -1, i32::MIN, i32::MAX] {
        for &f3_1 in &[0i32, 1, -1, i32::MIN, i32::MAX] {
            for &(s0, s1) in &[(0u64, 0u64), (0, 1), (1, 0), (u64::MAX, u64::MAX)] {
                for &ch in &[0u32, 1, 2, 3] {
                    for &bd in &[0u32, 16, 32, 33] {
                        for &h10 in &[0u16, 0x3ff, 0x7c00, 0xfc00, 0xffff] {
                            let mut a = base;
                            a.f3_1 = f3_1;
                            a.f3_2 = f3_2;
                            a.f4_1 = s0;
                            a.f4_2 = s1;
                            a.f7_2 = ch;
                            a.f7_3 = bd;
                            a.f10_1 = h10;
                            // s == 0 for both colour ops (early-return paths)
                            a.f11_3 = 0.0;
                            a.f12_3 = 0.0;
                            // delta == 0 for f13
                            a.f13_2 = 0.5;
                            a.f13_3 = 0.5;
                            a.f13_4 = 0.5;
                            let c = p.c.call_agglom(&a);
                            let r = p.r.call_agglom(&a);
                            eq_f64(&format!("c70 {a:?}"), c, r);
                        }
                    }
                }
            }
        }
    }
    // Degenerate geometry / triangle / colour combinations layered on top.
    for i in 0..50_000 {
        let mut a = rng.sane_agglom();
        match rng.below(6) {
            0 => {
                // f2: identical circle & box corner
                a.f2_1 = 0.0;
                a.f2_2 = 0.0;
                a.f2_3 = 0.0;
                a.f2_7 = 0.0;
                a.f2_8 = 0.0;
                a.f2_9 = 0.0;
                a.f2_10 = 0.0;
            }
            1 => {
                // f9: all triangle vertices identical
                a.f9_1 = 1.0;
                a.f9_2 = 2.0;
                a.f9_4 = 1.0;
                a.f9_5 = 2.0;
                a.f9_7 = 1.0;
                a.f9_8 = 2.0;
            }
            2 => {
                a.f7_1 = u32::MAX;
                a.f7_2 = u32::MAX;
                a.f7_3 = u32::MAX;
            }
            3 => {
                a.f11_2 = -1.0;
                a.f12_2 = -1.0;
            }
            4 => {
                a.f11_2 = 1.0e30;
                a.f12_2 = 1.0e30;
            }
            _ => {
                a.f13_2 = -0.0;
                a.f13_3 = -1.0;
                a.f13_4 = -2.0;
            }
        }
        let c = p.c.call_agglom(&a);
        let r = p.r.call_agglom(&a);
        eq_f64(&format!("c70 degenerate[{i}] {a:?}"), c, r);
    }
}

// ===========================================================================
// C71–C72 — libm boundary: the C `.so` imports glibc's `fmodf`/`floorf`, while
// the Rust `.so` links `compiler-builtins`' own implementations (they do NOT
// appear in `nm -D --undefined-only` on the Rust side). These rows sweep the
// whole `f32` exponent range through the two entry points that call them, so a
// divergence between the two libm implementations cannot hide.
// ===========================================================================

/// Mantissa patterns that stress rounding / sticky bits.
fn mantissa_patterns() -> Vec<u32> {
    let mut v = vec![
        0x00_0000, 0x00_0001, 0x00_0002, 0x00_0003, 0x40_0000, 0x20_0000, 0x7f_ffff, 0x7f_fffe,
        0x55_5555, 0x2a_aaaa, 0x12_3456, 0x6d_b6db, 0x01_0000, 0x00_8000, 0x00_0100,
    ];
    for k in 0..23 {
        v.push(1u32 << k);
        v.push((1u32 << k) | 1);
    }
    v
}

/// C71 — `f11` (calls `fmodf(h/60.0f, 2)`) over the full exponent range
#[test]
fn c71_f11_fmodf_sweep() {
    let p = both();
    let mants = mantissa_patterns();
    for sign in [0u32, 0x8000_0000] {
        for exp in 0u32..=255 {
            for &m in &mants {
                let h = f32::from_bits(sign | (exp << 23) | m);
                for &(s, l) in &[(1.0f32, 0.5f32), (0.25, 0.75), (-1.0, 0.5), (2.0, -0.25)] {
                    color_row(&p, Which::F11, [h, s, l], "c71");
                }
            }
        }
    }
}

/// C72 — `f12` (calls `floorf(h/60.0f)`) over the full exponent range
#[test]
fn c72_f12_floorf_sweep() {
    let p = both();
    let mants = mantissa_patterns();
    for sign in [0u32, 0x8000_0000] {
        for exp in 0u32..=255 {
            for &m in &mants {
                let h = f32::from_bits(sign | (exp << 23) | m);
                for &(s, v) in &[(1.0f32, 1.0f32), (0.25, 0.75), (-1.0, 0.5), (2.0, -0.25)] {
                    color_row(&p, Which::F12, [h, s, v], "c72");
                }
            }
        }
    }
}

/// C73 — `agglom` driven with the same exponent sweep on every float parameter
/// simultaneously (the aggregate path through `fmodf`, `floorf` and the table).
#[test]
fn c73_agglom_exponent_sweep() {
    let p = both();
    let mut rng = Rng::new(0xC73);
    let mants = mantissa_patterns();
    for sign in [0u32, 0x8000_0000] {
        for exp in 0u32..=255 {
            let m = mants[(exp as usize) % mants.len()];
            let f = f32::from_bits(sign | (exp << 23) | m);
            let mut a = rng.sane_agglom();
            a.f11_2 = f;
            a.f12_2 = f;
            a.f13_2 = f;
            a.f9_1 = f;
            a.f2_1 = f;
            let c = p.c.call_agglom(&a);
            let r = p.r.call_agglom(&a);
            eq_f64(&format!("c73 exp={exp} sign={sign:#x} {a:?}"), c, r);
        }
    }
}
