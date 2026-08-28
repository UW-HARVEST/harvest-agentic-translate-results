//! Level 2: integer-returning overlap predicates.

#![allow(non_snake_case)]

mod common;
use common::*;

#[test]
fn t_c2AABBtoAABB() {
    let p = Pair::load();
    let (c, r) = p.sym::<FnAABBAABB_i>("c2AABBtoAABB");
    let small: &[f32] = &[-1.0, 0.0, 1.0, f32::NAN, f32::INFINITY];
    for &a0 in small {
        for &a1 in small {
            for &b0 in small {
                for &b1 in small {
                    let A = c2AABB {
                        min: c2v { x: a0, y: a1 },
                        max: c2v { x: a1, y: a0 },
                    };
                    let B = c2AABB {
                        min: c2v { x: b0, y: b1 },
                        max: c2v { x: b1, y: b0 },
                    };
                    let ctx = format!("{a0:?},{a1:?},{b0:?},{b1:?}");
                    unsafe { assert_i_eq("c2AABBtoAABB", &ctx, c(A, B), r(A, B)) };
                }
            }
        }
    }
    let mut rng = Rng::new(0x2222);
    for _ in 0..200_000 {
        let A = c2AABB {
            min: rng.vec_wild(),
            max: rng.vec_wild(),
        };
        let B = c2AABB {
            min: rng.vec_wild(),
            max: rng.vec_wild(),
        };
        let ctx = format!("A=({:?},{:?})-({:?},{:?}) B=({:?},{:?})-({:?},{:?})",
            A.min.x, A.min.y, A.max.x, A.max.y, B.min.x, B.min.y, B.max.x, B.max.y);
        unsafe { assert_i_eq("c2AABBtoAABB", &ctx, c(A, B), r(A, B)) };
    }
    // Well-ordered, overlapping-ish boxes.
    let mut rng = Rng::new(0x2223);
    for _ in 0..200_000 {
        let A = ordered_box(&mut rng);
        let B = ordered_box(&mut rng);
        let ctx = format!("A=({:?},{:?})-({:?},{:?}) B=({:?},{:?})-({:?},{:?})",
            A.min.x, A.min.y, A.max.x, A.max.y, B.min.x, B.min.y, B.max.x, B.max.y);
        unsafe { assert_i_eq("c2AABBtoAABB", &ctx, c(A, B), r(A, B)) };
    }
}

fn ordered_box(rng: &mut Rng) -> c2AABB {
    let cx = rng.sym(10.0);
    let cy = rng.sym(10.0);
    let ex = rng.unit() * 8.0;
    let ey = rng.unit() * 8.0;
    c2AABB {
        min: c2v { x: cx - ex, y: cy - ey },
        max: c2v { x: cx + ex, y: cy + ey },
    }
}

#[test]
fn t_c2AABBtoPoint() {
    let p = Pair::load();
    let (c, r) = p.sym::<FnAABBV_i>("c2AABBtoPoint");
    for &a in EDGE_SCALARS {
        for &b in EDGE_SCALARS {
            for &q in EDGE_SCALARS {
                let A = c2AABB {
                    min: c2v { x: a, y: b },
                    max: c2v { x: b, y: a },
                };
                let B = c2v { x: q, y: q };
                let ctx = format!("A=({a:?},{b:?}) q={q:?}");
                unsafe { assert_i_eq("c2AABBtoPoint", &ctx, c(A, B), r(A, B)) };
            }
        }
    }
    let mut rng = Rng::new(0x3333);
    for _ in 0..200_000 {
        let A = c2AABB {
            min: rng.vec_wild(),
            max: rng.vec_wild(),
        };
        let B = rng.vec_wild();
        let ctx = format!("A=({:?},{:?})-({:?},{:?}) B=({:?},{:?})",
            A.min.x, A.min.y, A.max.x, A.max.y, B.x, B.y);
        unsafe { assert_i_eq("c2AABBtoPoint", &ctx, c(A, B), r(A, B)) };
    }
    let mut rng = Rng::new(0x3334);
    for _ in 0..200_000 {
        let A = ordered_box(&mut rng);
        let B = c2v { x: rng.sym(12.0), y: rng.sym(12.0) };
        let ctx = format!("A=({:?},{:?})-({:?},{:?}) B=({:?},{:?})",
            A.min.x, A.min.y, A.max.x, A.max.y, B.x, B.y);
        unsafe { assert_i_eq("c2AABBtoPoint", &ctx, c(A, B), r(A, B)) };
    }
}

#[test]
fn t_c2CircleToPoint() {
    let p = Pair::load();
    let (c, r) = p.sym::<FnCircleV_i>("c2CircleToPoint");
    for &px in EDGE_SCALARS {
        for &py in EDGE_SCALARS {
            for &rad in EDGE_SCALARS {
                for &q in EDGE_SCALARS {
                    let A = c2Circle {
                        p: c2v { x: px, y: py },
                        r: rad,
                    };
                    let B = c2v { x: q, y: -q };
                    let ctx = format!("p=({px:?},{py:?}) r={rad:?} q={q:?}");
                    unsafe { assert_i_eq("c2CircleToPoint", &ctx, c(A, B), r(A, B)) };
                }
            }
        }
    }
    let mut rng = Rng::new(0x4444);
    for _ in 0..200_000 {
        let A = c2Circle {
            p: rng.vec_wild(),
            r: rng.float(),
        };
        let B = rng.vec_wild();
        let ctx = format!("p=({:?},{:?}) r={:?} B=({:?},{:?})", A.p.x, A.p.y, A.r, B.x, B.y);
        unsafe { assert_i_eq("c2CircleToPoint", &ctx, c(A, B), r(A, B)) };
    }
    // Points hovering right on the circle boundary.
    let mut rng = Rng::new(0x4445);
    for _ in 0..200_000 {
        let A = c2Circle {
            p: c2v { x: rng.sym(5.0), y: rng.sym(5.0) },
            r: rng.unit() * 5.0,
        };
        let ang = rng.unit() * 6.283_185_5;
        let scale = 0.999 + rng.unit() * 0.002;
        let B = c2v {
            x: A.p.x + A.r * scale * ang.cos(),
            y: A.p.y + A.r * scale * ang.sin(),
        };
        let ctx = format!("p=({:?},{:?}) r={:?} B=({:?},{:?})", A.p.x, A.p.y, A.r, B.x, B.y);
        unsafe { assert_i_eq("c2CircleToPoint", &ctx, c(A, B), r(A, B)) };
    }
}
