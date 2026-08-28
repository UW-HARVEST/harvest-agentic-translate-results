//! Phase B — CONFIGS.md rows 20..26: the boolean predicates
//! `c2AABBtoAABB`, `c2AABBtoPoint`, `c2CircleToPoint`.

mod common;

use common::*;

fn rand_box(rng: &mut Rng, scale: f32) -> c2AABB {
    let a = rng.vec_uniform(scale);
    let b = rng.vec_uniform(scale);
    c2AABB {
        min: v(a.x.min(b.x), a.y.min(b.y)),
        max: v(a.x.max(b.x), a.y.max(b.y)),
    }
}

// ---------------------------------------------------------------------------
// Rows 20, 21, 22 — c2AABBtoAABB
// ---------------------------------------------------------------------------

#[test]
fn cfg_20_aabb_aabb_random() {
    let p = load();
    let mut d = Diff::new("cfg_20_aabb_aabb_random");
    let mut rng = Rng::new(0x2020);
    for scale in [1e-3f32, 1.0, 1e3, 1e18] {
        for _ in 0..30_000 {
            let a = rand_box(&mut rng, scale);
            let b = rand_box(&mut rng, scale);
            d.eq_i(
                || format!("c2AABBtoAABB({:?}, {:?})", a, b),
                unsafe { (p.c.c2AABBtoAABB)(a, b) },
                unsafe { (p.r.c2AABBtoAABB)(a, b) },
            );
        }
        // Deliberately near-touching boxes so all four separating axes are hit
        // with the tiny offsets that expose off-by-one comparisons.
        for _ in 0..30_000 {
            let a = rand_box(&mut rng, scale);
            let dx = [0.0f32, -0.0, 1e-7, -1e-7, scale, -scale][rng.below(6) as usize];
            let dy = [0.0f32, -0.0, 1e-7, -1e-7, scale, -scale][rng.below(6) as usize];
            let b = c2AABB {
                min: v(a.max.x + dx, a.max.y + dy),
                max: v(a.max.x + dx + rng.positive(scale), a.max.y + dy + rng.positive(scale)),
            };
            d.eq_i(
                || format!("c2AABBtoAABB({:?}, {:?})", a, b),
                unsafe { (p.c.c2AABBtoAABB)(a, b) },
                unsafe { (p.r.c2AABBtoAABB)(a, b) },
            );
        }
    }
    d.finish();
}

#[test]
fn cfg_21_aabb_aabb_edges_and_inverted() {
    let p = load();
    let mut d = Diff::new("cfg_21_aabb_aabb_edges_and_inverted");
    let mut rng = Rng::new(0x2121);
    // Exactly touching edges, zero-area boxes, and inverted (min > max) boxes.
    let a = c2AABB {
        min: v(-1.0, -1.0),
        max: v(1.0, 1.0),
    };
    let cases: Vec<c2AABB> = vec![
        c2AABB { min: v(1.0, 0.0), max: v(2.0, 1.0) },   // touching +x
        c2AABB { min: v(-2.0, 0.0), max: v(-1.0, 1.0) }, // touching -x
        c2AABB { min: v(0.0, 1.0), max: v(1.0, 2.0) },   // touching +y
        c2AABB { min: v(0.0, -2.0), max: v(1.0, -1.0) }, // touching -y
        c2AABB { min: v(1.0, 1.0), max: v(1.0, 1.0) },   // degenerate point at corner
        c2AABB { min: v(0.0, 0.0), max: v(0.0, 0.0) },   // degenerate point inside
        c2AABB { min: v(5.0, 5.0), max: v(-5.0, -5.0) }, // fully inverted
        c2AABB { min: v(1.0, -5.0), max: v(-1.0, 5.0) }, // x inverted
        c2AABB { min: v(-0.0, -0.0), max: v(0.0, 0.0) },
        c2AABB { min: v(0.0, 0.0), max: v(-0.0, -0.0) },
    ];
    for b in &cases {
        d.eq_i(
            || format!("c2AABBtoAABB({:?}, {:?})", a, b),
            unsafe { (p.c.c2AABBtoAABB)(a, *b) },
            unsafe { (p.r.c2AABBtoAABB)(a, *b) },
        );
        d.eq_i(
            || format!("c2AABBtoAABB({:?}, {:?})", b, a),
            unsafe { (p.c.c2AABBtoAABB)(*b, a) },
            unsafe { (p.r.c2AABBtoAABB)(*b, a) },
        );
    }
    // Random *unsorted* corner pairs → half of them inverted.
    for _ in 0..100_000 {
        let ba = c2AABB {
            min: rng.vec_uniform(10.0),
            max: rng.vec_uniform(10.0),
        };
        let bb = c2AABB {
            min: rng.vec_uniform(10.0),
            max: rng.vec_uniform(10.0),
        };
        d.eq_i(
            || format!("c2AABBtoAABB({:?}, {:?})", ba, bb),
            unsafe { (p.c.c2AABBtoAABB)(ba, bb) },
            unsafe { (p.r.c2AABBtoAABB)(ba, bb) },
        );
    }
    d.finish();
}

#[test]
fn cfg_22_aabb_aabb_specials() {
    let p = load();
    let mut d = Diff::new("cfg_22_aabb_aabb_specials");
    let mut rng = Rng::new(0x2222);
    let sp = specials();
    // Sweep every one of the 8 coordinate slots against the special pool while
    // the rest hold a fixed sorted box.
    let base = [-1.0f32, -1.0, 1.0, 1.0, -0.5, -0.5, 0.5, 0.5];
    for slot in 0..8usize {
        for &s in &sp {
            for &s2 in &sp {
                let mut f = base;
                f[slot] = s;
                f[(slot + 3) % 8] = s2;
                let a = c2AABB { min: v(f[0], f[1]), max: v(f[2], f[3]) };
                let b = c2AABB { min: v(f[4], f[5]), max: v(f[6], f[7]) };
                d.eq_i(
                    || format!("c2AABBtoAABB({:?}, {:?})", a, b),
                    unsafe { (p.c.c2AABBtoAABB)(a, b) },
                    unsafe { (p.r.c2AABBtoAABB)(a, b) },
                );
            }
        }
    }
    for _ in 0..100_000 {
        let a = c2AABB { min: rng.vec_spicy(10.0), max: rng.vec_spicy(10.0) };
        let b = c2AABB { min: rng.vec_spicy(10.0), max: rng.vec_spicy(10.0) };
        d.eq_i(
            || format!("c2AABBtoAABB({:?}, {:?})", a, b),
            unsafe { (p.c.c2AABBtoAABB)(a, b) },
            unsafe { (p.r.c2AABBtoAABB)(a, b) },
        );
    }
    d.finish();
}

// ---------------------------------------------------------------------------
// Rows 23, 24 — c2AABBtoPoint
// ---------------------------------------------------------------------------

#[test]
fn cfg_23_aabb_point_random() {
    let p = load();
    let mut d = Diff::new("cfg_23_aabb_point_random");
    let mut rng = Rng::new(0x2323);
    let a = c2AABB { min: v(-1.0, -2.0), max: v(3.0, 4.0) };
    // Exact edges & corners.
    let pts = [
        v(-1.0, -2.0), v(3.0, 4.0), v(-1.0, 4.0), v(3.0, -2.0),
        v(-1.0, 0.0), v(3.0, 0.0), v(0.0, -2.0), v(0.0, 4.0),
        v(-1.000001, 0.0), v(3.000001, 0.0), v(0.0, -2.000001), v(0.0, 4.000001),
        v(0.0, 0.0), v(-0.0, -0.0),
    ];
    for &pt in &pts {
        d.eq_i(
            || format!("c2AABBtoPoint({:?}, {})", a, vs(pt)),
            unsafe { (p.c.c2AABBtoPoint)(a, pt) },
            unsafe { (p.r.c2AABBtoPoint)(a, pt) },
        );
    }
    for scale in [1e-3f32, 1.0, 1e3, 1e18] {
        for _ in 0..40_000 {
            let bx = rand_box(&mut rng, scale);
            let pt = rng.vec_uniform(scale);
            d.eq_i(
                || format!("c2AABBtoPoint({:?}, {})", bx, vs(pt)),
                unsafe { (p.c.c2AABBtoPoint)(bx, pt) },
                unsafe { (p.r.c2AABBtoPoint)(bx, pt) },
            );
            // A point pinned to one of the box's own coordinates (edge case).
            let edge = match rng.below(4) {
                0 => v(bx.min.x, pt.y),
                1 => v(bx.max.x, pt.y),
                2 => v(pt.x, bx.min.y),
                _ => v(pt.x, bx.max.y),
            };
            d.eq_i(
                || format!("c2AABBtoPoint({:?}, {})", bx, vs(edge)),
                unsafe { (p.c.c2AABBtoPoint)(bx, edge) },
                unsafe { (p.r.c2AABBtoPoint)(bx, edge) },
            );
        }
    }
    d.finish();
}

#[test]
fn cfg_24_aabb_point_specials() {
    let p = load();
    let mut d = Diff::new("cfg_24_aabb_point_specials");
    let mut rng = Rng::new(0x2424);
    let sp = specials();
    let base = [-1.0f32, -1.0, 1.0, 1.0, 0.0, 0.0];
    for slot in 0..6usize {
        for &s in &sp {
            for &s2 in &sp {
                let mut f = base;
                f[slot] = s;
                f[(slot + 2) % 6] = s2;
                let bx = c2AABB { min: v(f[0], f[1]), max: v(f[2], f[3]) };
                let pt = v(f[4], f[5]);
                d.eq_i(
                    || format!("c2AABBtoPoint({:?}, {})", bx, vs(pt)),
                    unsafe { (p.c.c2AABBtoPoint)(bx, pt) },
                    unsafe { (p.r.c2AABBtoPoint)(bx, pt) },
                );
            }
        }
    }
    for _ in 0..100_000 {
        let bx = c2AABB { min: rng.vec_spicy(10.0), max: rng.vec_spicy(10.0) };
        let pt = rng.vec_spicy(10.0);
        d.eq_i(
            || format!("c2AABBtoPoint({:?}, {})", bx, vs(pt)),
            unsafe { (p.c.c2AABBtoPoint)(bx, pt) },
            unsafe { (p.r.c2AABBtoPoint)(bx, pt) },
        );
    }
    d.finish();
}

// ---------------------------------------------------------------------------
// Rows 25, 26 — c2CircleToPoint
// ---------------------------------------------------------------------------

#[test]
fn cfg_25_circle_point_random() {
    let p = load();
    let mut d = Diff::new("cfg_25_circle_point_random");
    let mut rng = Rng::new(0x2525);
    for scale in [1e-3f32, 1.0, 1e3, 1e18] {
        for _ in 0..40_000 {
            let ci = c2Circle {
                p: rng.vec_uniform(scale),
                r: rng.positive(scale),
            };
            // Inside, outside, and exactly-on-the-rim points.
            let pts = [
                rng.vec_uniform(scale),
                v(ci.p.x + ci.r, ci.p.y),
                v(ci.p.x - ci.r, ci.p.y),
                v(ci.p.x, ci.p.y + ci.r),
                v(ci.p.x + ci.r * 0.5, ci.p.y),
                v(ci.p.x + ci.r * 1.0000001, ci.p.y),
                ci.p,
            ];
            for &pt in &pts {
                d.eq_i(
                    || format!("c2CircleToPoint({:?}, {})", ci, vs(pt)),
                    unsafe { (p.c.c2CircleToPoint)(ci, pt) },
                    unsafe { (p.r.c2CircleToPoint)(ci, pt) },
                );
            }
        }
    }
    d.finish();
}

#[test]
fn cfg_26_circle_point_specials() {
    let p = load();
    let mut d = Diff::new("cfg_26_circle_point_specials");
    let mut rng = Rng::new(0x2626);
    let sp = specials();
    let base = [0.0f32, 0.0, 1.0, 0.5, 0.5];
    for slot in 0..5usize {
        for &s in &sp {
            for &s2 in &sp {
                let mut f = base;
                f[slot] = s;
                f[(slot + 2) % 5] = s2;
                let ci = c2Circle { p: v(f[0], f[1]), r: f[2] };
                let pt = v(f[3], f[4]);
                d.eq_i(
                    || format!("c2CircleToPoint({:?}, {})", ci, vs(pt)),
                    unsafe { (p.c.c2CircleToPoint)(ci, pt) },
                    unsafe { (p.r.c2CircleToPoint)(ci, pt) },
                );
            }
        }
    }
    // r ∈ {0, negative, inf, NaN, subnormal, huge}
    for &r in &[
        0.0f32,
        -0.0,
        -1.0,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        f32::MIN_POSITIVE,
        f32::from_bits(1),
        f32::MAX,
        3.4e38,
    ] {
        for _ in 0..5_000 {
            let ci = c2Circle { p: rng.vec_uniform(10.0), r };
            let pt = rng.vec_uniform(10.0);
            d.eq_i(
                || format!("c2CircleToPoint({:?}, {})", ci, vs(pt)),
                unsafe { (p.c.c2CircleToPoint)(ci, pt) },
                unsafe { (p.r.c2CircleToPoint)(ci, pt) },
            );
        }
    }
    for _ in 0..100_000 {
        let ci = c2Circle { p: rng.vec_bits(), r: rng.any_bits() };
        let pt = rng.vec_bits();
        d.eq_i(
            || format!("c2CircleToPoint({:?}, {})", ci, vs(pt)),
            unsafe { (p.c.c2CircleToPoint)(ci, pt) },
            unsafe { (p.r.c2CircleToPoint)(ci, pt) },
        );
    }
    d.finish();
}
