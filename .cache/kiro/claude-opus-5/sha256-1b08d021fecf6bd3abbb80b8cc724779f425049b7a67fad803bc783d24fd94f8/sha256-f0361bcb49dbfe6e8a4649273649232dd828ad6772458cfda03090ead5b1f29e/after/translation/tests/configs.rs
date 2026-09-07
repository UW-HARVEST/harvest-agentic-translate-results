//! Phase B — valid-path differential tests.
//!
//! One `#[test]` per row of `CONFIGS.md`, in the same order. Every test loads
//! both shared objects through `libloading` and compares results bit-for-bit
//! (`f32::to_bits`), so NaN payloads and signed zeros must agree — numeric
//! equality is not accepted.
//!
//! Randomized rows use a fixed splitmix64 seed derived from the row number, so
//! a failure reproduces exactly.

#![allow(non_snake_case)]

mod common;
use common::*;

/// Randomized cases per row for the L0/L1 scalar and predicate functions.
const N_SMALL: usize = 4000;
/// Randomized cases per row for the L2–L4 raycast functions.
const N_RAY: usize = 3000;

// ===========================================================================
// Row 1 — c2V
// ===========================================================================

#[test]
fn cfg_01_c2V() {
    let a = apis();
    let mut vals: Vec<f32> = EDGE_F32.to_vec();
    vals.extend(exotic_f32());
    // Exhaustive over the edge cross-product, then randomized.
    for &x in &vals {
        for &y in &vals {
            let c = unsafe { (a.c.c2V)(x, y) };
            let r = unsafe { (a.rust.c2V)(x, y) };
            assert_eq!(
                c.bits(),
                r.bits(),
                "c2V({}, {}): C=({},{}) Rust=({},{})",
                f(x),
                f(y),
                f(c.x),
                f(c.y),
                f(r.x),
                f(r.y)
            );
        }
    }
    let mut rng = Rng::new(1);
    for _ in 0..N_SMALL {
        let (x, y) = (rng.pathological(), rng.pathological());
        let c = unsafe { (a.c.c2V)(x, y) };
        let r = unsafe { (a.rust.c2V)(x, y) };
        assert_eq!(c.bits(), r.bits(), "c2V({}, {})", f(x), f(y));
    }
}

// ===========================================================================
// Rows 2-3 — c2Dot
// ===========================================================================

fn check_dot(p: C2v, q: C2v, ctx: &str) {
    let a = apis();
    let c = unsafe { (a.c.c2Dot)(p, q) };
    let r = unsafe { (a.rust.c2Dot)(p, q) };
    assert_eq!(
        c.to_bits(),
        r.to_bits(),
        "c2Dot divergence [{ctx}]: a=({},{}) b=({},{}) C={} Rust={}",
        f(p.x),
        f(p.y),
        f(q.x),
        f(q.y),
        f(c),
        f(r)
    );
}

#[test]
fn cfg_02_c2Dot_finite() {
    let mut rng = Rng::new(2);
    for i in 0..N_SMALL {
        let (p, q) = (rng.v(), rng.v());
        check_dot(p, q, &format!("iter {i}"));
    }
    // Exact-cancellation and overflow shapes.
    for &(a, b) in &[
        (1.0f32, -1.0f32),
        (f32::MAX, f32::MAX),
        (f32::MAX, -f32::MAX),
        (f32::MIN_POSITIVE, f32::MIN_POSITIVE),
        (0.0, 0.0),
        (-0.0, 0.0),
    ] {
        check_dot(C2v::new(a, b), C2v::new(b, a), "cancellation");
        check_dot(C2v::new(a, a), C2v::new(b, b), "cancellation2");
    }
}

#[test]
fn cfg_03_c2Dot_pathological() {
    // Distinct NaN payloads in every lane position: this is what pins the
    // operand order of `a.x*b.x + a.y*b.y`, since addss/mulss keep the
    // destination operand's payload.
    let n = exotic_f32();
    let mut pool: Vec<f32> = EDGE_F32.to_vec();
    pool.extend(n);
    for (i, &ax) in pool.iter().enumerate() {
        for (j, &by) in pool.iter().enumerate() {
            check_dot(
                C2v::new(ax, by),
                C2v::new(by, ax),
                &format!("edge {i}x{j} swapped"),
            );
            check_dot(
                C2v::new(ax, ax),
                C2v::new(by, by),
                &format!("edge {i}x{j} uniform"),
            );
        }
    }
    let mut rng = Rng::new(3);
    for i in 0..N_SMALL {
        let (p, q) = (rng.v_path(), rng.v_path());
        check_dot(p, q, &format!("rand {i}"));
    }
}

// ===========================================================================
// Row 4 — c2Len
// ===========================================================================

#[test]
fn cfg_04_c2Len() {
    let a = apis();
    let check = |v: C2v, ctx: &str| {
        let c = unsafe { (a.c.c2Len)(v) };
        let r = unsafe { (a.rust.c2Len)(v) };
        assert_eq!(
            c.to_bits(),
            r.to_bits(),
            "c2Len divergence [{ctx}]: v=({},{}) C={} Rust={}",
            f(v.x),
            f(v.y),
            f(c),
            f(r)
        );
    };
    let mut pool: Vec<f32> = EDGE_F32.to_vec();
    pool.extend(exotic_f32());
    for &x in &pool {
        for &y in &pool {
            check(C2v::new(x, y), "edge");
        }
    }
    let mut rng = Rng::new(4);
    for i in 0..N_SMALL {
        check(rng.v_path(), &format!("rand {i}"));
        check(rng.v(), &format!("rand-finite {i}"));
    }
}

// ===========================================================================
// Rows 5-6 — c2Add / c2Sub
// ===========================================================================

#[test]
fn cfg_05_c2Add() {
    let a = apis();
    let mut pool: Vec<f32> = EDGE_F32.to_vec();
    pool.extend(exotic_f32());
    for &x in &pool {
        for &y in &pool {
            cmp_v2("c2Add", a.c.c2Add, a.rust.c2Add, C2v::new(x, y), C2v::new(y, x), "edge");
            cmp_v2("c2Add", a.c.c2Add, a.rust.c2Add, C2v::new(x, x), C2v::new(y, y), "edge2");
        }
    }
    let mut rng = Rng::new(5);
    for i in 0..N_SMALL {
        cmp_v2("c2Add", a.c.c2Add, a.rust.c2Add, rng.v_path(), rng.v_path(), &format!("rand {i}"));
    }
}

#[test]
fn cfg_06_c2Sub() {
    let a = apis();
    let mut pool: Vec<f32> = EDGE_F32.to_vec();
    pool.extend(exotic_f32());
    for &x in &pool {
        for &y in &pool {
            cmp_v2("c2Sub", a.c.c2Sub, a.rust.c2Sub, C2v::new(x, y), C2v::new(y, x), "edge");
            cmp_v2("c2Sub", a.c.c2Sub, a.rust.c2Sub, C2v::new(x, x), C2v::new(y, y), "edge2");
        }
    }
    let mut rng = Rng::new(6);
    for i in 0..N_SMALL {
        cmp_v2("c2Sub", a.c.c2Sub, a.rust.c2Sub, rng.v_path(), rng.v_path(), &format!("rand {i}"));
    }
}

// ===========================================================================
// Rows 7-8 — c2Mulvs / c2Div
// ===========================================================================

#[test]
fn cfg_07_c2Mulvs() {
    let a = apis();
    let mut pool: Vec<f32> = EDGE_F32.to_vec();
    pool.extend(exotic_f32());
    for &s in &pool {
        for &x in &pool {
            cmp_vs("c2Mulvs", a.c.c2Mulvs, a.rust.c2Mulvs, C2v::new(x, -x), s, "edge");
        }
    }
    let mut rng = Rng::new(7);
    for i in 0..N_SMALL {
        cmp_vs(
            "c2Mulvs",
            a.c.c2Mulvs,
            a.rust.c2Mulvs,
            rng.v_path(),
            rng.pathological(),
            &format!("rand {i}"),
        );
    }
}

#[test]
fn cfg_08_c2Div() {
    let a = apis();
    // The C is `c2Mulvs(a, 1.0f / b)`: reciprocal-then-multiply, which differs
    // from a true divide for many b. Sweeping b over denormals and MIN_POSITIVE
    // is what exposes that (1/MIN_POSITIVE overflows the reciprocal).
    let mut pool: Vec<f32> = EDGE_F32.to_vec();
    pool.extend(exotic_f32());
    pool.extend([3.0, 7.0, 0.1, 1e30, 1e-30, -1e-30]);
    for &b in &pool {
        for &x in &pool {
            cmp_vs("c2Div", a.c.c2Div, a.rust.c2Div, C2v::new(x, -x), b, "edge");
            cmp_vs("c2Div", a.c.c2Div, a.rust.c2Div, C2v::new(x, x), b, "edge2");
        }
    }
    let mut rng = Rng::new(8);
    for i in 0..N_SMALL {
        cmp_vs("c2Div", a.c.c2Div, a.rust.c2Div, rng.v_path(), rng.pathological(), &format!("rand {i}"));
        cmp_vs("c2Div", a.c.c2Div, a.rust.c2Div, rng.v(), rng.interesting(), &format!("randf {i}"));
    }
}

// ===========================================================================
// Row 9 — c2Norm
// ===========================================================================

#[test]
fn cfg_09_c2Norm() {
    let a = apis();
    let mut pool: Vec<f32> = EDGE_F32.to_vec();
    pool.extend(exotic_f32());
    pool.extend([3.0, 4.0, 5.0, 1e20, 1e-20]);
    for &x in &pool {
        for &y in &pool {
            cmp_v1("c2Norm", a.c.c2Norm, a.rust.c2Norm, C2v::new(x, y), "edge");
        }
    }
    let mut rng = Rng::new(9);
    for i in 0..N_SMALL {
        cmp_v1("c2Norm", a.c.c2Norm, a.rust.c2Norm, rng.v_path(), &format!("rand {i}"));
        cmp_v1("c2Norm", a.c.c2Norm, a.rust.c2Norm, rng.v(), &format!("randf {i}"));
    }
}

// ===========================================================================
// Row 10 — c2Minv / c2Maxv (ternary, not fminf/fmaxf)
// ===========================================================================

#[test]
fn cfg_10_c2Minv_c2Maxv() {
    let a = apis();
    let mut pool: Vec<f32> = EDGE_F32.to_vec();
    pool.extend(exotic_f32());
    for &x in &pool {
        for &y in &pool {
            // Both operand orders: `NaN < y` is false so the ternary picks the
            // second operand, which is order-sensitive in a way fminf is not.
            cmp_v2("c2Minv", a.c.c2Minv, a.rust.c2Minv, C2v::new(x, y), C2v::new(y, x), "edge");
            cmp_v2("c2Minv", a.c.c2Minv, a.rust.c2Minv, C2v::new(y, x), C2v::new(x, y), "edge-rev");
            cmp_v2("c2Maxv", a.c.c2Maxv, a.rust.c2Maxv, C2v::new(x, y), C2v::new(y, x), "edge");
            cmp_v2("c2Maxv", a.c.c2Maxv, a.rust.c2Maxv, C2v::new(y, x), C2v::new(x, y), "edge-rev");
        }
    }
    let mut rng = Rng::new(10);
    for i in 0..N_SMALL {
        let (p, q) = (rng.v_path(), rng.v_path());
        cmp_v2("c2Minv", a.c.c2Minv, a.rust.c2Minv, p, q, &format!("rand {i}"));
        cmp_v2("c2Maxv", a.c.c2Maxv, a.rust.c2Maxv, p, q, &format!("rand {i}"));
    }
}

// ===========================================================================
// Row 11 — c2Skew / c2CCW90 / c2Absv
// ===========================================================================

#[test]
fn cfg_11_skew_ccw90_absv() {
    let a = apis();
    let mut pool: Vec<f32> = EDGE_F32.to_vec();
    pool.extend(exotic_f32());
    for &x in &pool {
        for &y in &pool {
            let v = C2v::new(x, y);
            cmp_v1("c2Skew", a.c.c2Skew, a.rust.c2Skew, v, "edge");
            cmp_v1("c2CCW90", a.c.c2CCW90, a.rust.c2CCW90, v, "edge");
            // c2Absv uses `x < 0 ? -x : x`, so abs(-0.0) == -0.0 and
            // abs(-NaN) == -NaN -- both differ from fabsf.
            cmp_v1("c2Absv", a.c.c2Absv, a.rust.c2Absv, v, "edge");
        }
    }
    let mut rng = Rng::new(11);
    for i in 0..N_SMALL {
        let v = rng.v_path();
        cmp_v1("c2Skew", a.c.c2Skew, a.rust.c2Skew, v, &format!("rand {i}"));
        cmp_v1("c2CCW90", a.c.c2CCW90, a.rust.c2CCW90, v, &format!("rand {i}"));
        cmp_v1("c2Absv", a.c.c2Absv, a.rust.c2Absv, v, &format!("rand {i}"));
    }
}

// ===========================================================================
// Row 12 — c2MulmvT
// ===========================================================================

#[test]
fn cfg_12_c2MulmvT() {
    let a = apis();
    let check = |m: C2m, v: C2v, ctx: &str| {
        let c = unsafe { (a.c.c2MulmvT)(m, v) };
        let r = unsafe { (a.rust.c2MulmvT)(m, v) };
        assert_eq!(
            c.bits(),
            r.bits(),
            "c2MulmvT divergence [{ctx}]: m=[({},{}),({},{})] v=({},{}) C=({},{}) Rust=({},{})",
            f(m.x.x), f(m.x.y), f(m.y.x), f(m.y.y),
            f(v.x), f(v.y), f(c.x), f(c.y), f(r.x), f(r.y)
        );
    };
    let mut pool: Vec<f32> = EDGE_F32.to_vec();
    pool.extend(exotic_f32());
    for &p in &pool {
        for &q in &pool {
            check(
                C2m { x: C2v::new(p, q), y: C2v::new(q, p) },
                C2v::new(p, q),
                "edge",
            );
            check(
                C2m { x: C2v::new(p, p), y: C2v::new(q, q) },
                C2v::new(q, p),
                "edge2",
            );
        }
    }
    let mut rng = Rng::new(12);
    for i in 0..N_SMALL {
        let m = C2m { x: rng.v_path(), y: rng.v_path() };
        check(m, rng.v_path(), &format!("rand {i}"));
        let m = C2m { x: rng.v(), y: rng.v() };
        check(m, rng.v(), &format!("randf {i}"));
    }
}

// ===========================================================================
// Rows 13-15 — L1 predicates
// ===========================================================================

fn check_aabb2(x: C2AABB, y: C2AABB, ctx: &str) {
    let a = apis();
    let c = unsafe { (a.c.c2AABBtoAABB)(x, y) };
    let r = unsafe { (a.rust.c2AABBtoAABB)(x, y) };
    assert_eq!(c, r, "c2AABBtoAABB divergence [{ctx}]: A={x:?} B={y:?} C={c} Rust={r}");
}

#[test]
fn cfg_13_c2AABBtoAABB() {
    // Hand-picked shapes: disjoint per axis, touching, nested, identical,
    // inverted, degenerate, plus NaN (where every `<` is false so the C
    // reports "overlapping").
    let base = C2AABB { min: C2v::new(0.0, 0.0), max: C2v::new(1.0, 1.0) };
    let shapes = [
        base,
        C2AABB { min: C2v::new(2.0, 0.0), max: C2v::new(3.0, 1.0) }, // disjoint x
        C2AABB { min: C2v::new(0.0, 2.0), max: C2v::new(1.0, 3.0) }, // disjoint y
        C2AABB { min: C2v::new(2.0, 2.0), max: C2v::new(3.0, 3.0) }, // disjoint both
        C2AABB { min: C2v::new(1.0, 0.0), max: C2v::new(2.0, 1.0) }, // touching edge
        C2AABB { min: C2v::new(0.25, 0.25), max: C2v::new(0.75, 0.75) }, // nested
        C2AABB { min: C2v::new(1.0, 1.0), max: C2v::new(0.0, 0.0) }, // inverted
        C2AABB { min: C2v::new(0.5, 0.5), max: C2v::new(0.5, 0.5) }, // degenerate
        C2AABB { min: C2v::new(f32::NAN, 0.0), max: C2v::new(1.0, 1.0) },
        C2AABB { min: C2v::new(0.0, 0.0), max: C2v::new(f32::NAN, f32::NAN) },
        C2AABB {
            min: C2v::new(f32::NEG_INFINITY, f32::NEG_INFINITY),
            max: C2v::new(f32::INFINITY, f32::INFINITY),
        },
        C2AABB { min: C2v::new(-0.0, -0.0), max: C2v::new(0.0, 0.0) },
    ];
    for (i, &p) in shapes.iter().enumerate() {
        for (j, &q) in shapes.iter().enumerate() {
            check_aabb2(p, q, &format!("shape {i} vs {j}"));
        }
    }
    let mut rng = Rng::new(13);
    for i in 0..N_SMALL {
        check_aabb2(
            C2AABB { min: rng.v(), max: rng.v() },
            C2AABB { min: rng.v(), max: rng.v() },
            &format!("rand {i}"),
        );
        check_aabb2(
            C2AABB { min: rng.v_path(), max: rng.v_path() },
            C2AABB { min: rng.v_path(), max: rng.v_path() },
            &format!("randp {i}"),
        );
        // Proper boxes on a coarse integer grid, so edge-touching and
        // exact-equality cases actually occur.
        let g = |r: &mut Rng| (r.next_u32() % 7) as f32 - 3.0;
        let (x0, y0) = (g(&mut rng), g(&mut rng));
        let (x1, y1) = (g(&mut rng), g(&mut rng));
        check_aabb2(
            C2AABB { min: C2v::new(x0.min(x1), y0.min(y1)), max: C2v::new(x0.max(x1), y0.max(y1)) },
            base,
            &format!("grid {i}"),
        );
    }
}

#[test]
fn cfg_14_c2AABBtoPoint() {
    let a = apis();
    let check = |b: C2AABB, p: C2v, ctx: &str| {
        let c = unsafe { (a.c.c2AABBtoPoint)(b, p) };
        let r = unsafe { (a.rust.c2AABBtoPoint)(b, p) };
        assert_eq!(c, r, "c2AABBtoPoint divergence [{ctx}]: box={b:?} p=({},{}) C={c} Rust={r}", f(p.x), f(p.y));
    };
    let boxes = [
        C2AABB { min: C2v::new(0.0, 0.0), max: C2v::new(1.0, 1.0) },
        C2AABB { min: C2v::new(1.0, 1.0), max: C2v::new(0.0, 0.0) }, // inverted
        C2AABB { min: C2v::new(0.0, 0.0), max: C2v::new(0.0, 0.0) }, // degenerate
        C2AABB { min: C2v::new(-0.0, -0.0), max: C2v::new(0.0, 0.0) },
        C2AABB { min: C2v::new(-1.0, 0.0), max: C2v::new(1.0, f32::NAN) },
    ];
    // Exact edges (inclusive), one ULP inside, one ULP outside, corners.
    let pts = [
        C2v::new(0.5, 0.5),
        C2v::new(0.0, 0.5), C2v::new(1.0, 0.5),
        C2v::new(0.5, 0.0), C2v::new(0.5, 1.0),
        C2v::new(0.0, 0.0), C2v::new(1.0, 1.0), C2v::new(0.0, 1.0), C2v::new(1.0, 0.0),
        C2v::new(step(0.0, -1), 0.5), C2v::new(step(1.0, 1), 0.5),
        C2v::new(0.5, step(0.0, -1)), C2v::new(0.5, step(1.0, 1)),
        C2v::new(-0.0, -0.0),
        C2v::new(f32::NAN, 0.5), C2v::new(0.5, f32::NAN), C2v::new(f32::NAN, f32::NAN),
        C2v::new(f32::INFINITY, 0.5), C2v::new(f32::NEG_INFINITY, 0.5),
    ];
    for (i, &b) in boxes.iter().enumerate() {
        for (j, &p) in pts.iter().enumerate() {
            check(b, p, &format!("box {i} pt {j}"));
        }
    }
    let mut rng = Rng::new(14);
    for i in 0..N_SMALL {
        check(C2AABB { min: rng.v(), max: rng.v() }, rng.v(), &format!("rand {i}"));
        check(C2AABB { min: rng.v_path(), max: rng.v_path() }, rng.v_path(), &format!("randp {i}"));
    }
}

#[test]
fn cfg_15_c2CircleToPoint() {
    let a = apis();
    let check = |ci: C2Circle, p: C2v, ctx: &str| {
        let c = unsafe { (a.c.c2CircleToPoint)(ci, p) };
        let r = unsafe { (a.rust.c2CircleToPoint)(ci, p) };
        assert_eq!(c, r, "c2CircleToPoint divergence [{ctx}]: circle={ci:?} p=({},{}) C={c} Rust={r}", f(p.x), f(p.y));
    };
    // The rim test is `d2 < r*r`, i.e. EXCLUSIVE: a point exactly on the rim
    // must be rejected. Sweep exactly on / one ULP in / one ULP out.
    for &r in &[1.0f32, 0.0, -1.0, 2.0, 0.5, f32::MIN_POSITIVE, f32::MAX, f32::INFINITY, f32::NAN] {
        let ci = C2Circle { p: C2v::new(0.0, 0.0), r };
        for &d in &[0.0f32, r, step(r, -1), step(r, 1), r * 0.5, r * 2.0, -r] {
            check(ci, C2v::new(d, 0.0), &format!("r={} d={}", f(r), f(d)));
            check(ci, C2v::new(0.0, d), &format!("r={} d={} (y)", f(r), f(d)));
        }
        check(ci, C2v::new(f32::NAN, 0.0), &format!("r={} nan", f(r)));
        check(ci, C2v::new(-0.0, -0.0), &format!("r={} negzero", f(r)));
    }
    let mut rng = Rng::new(15);
    for i in 0..N_SMALL {
        check(C2Circle { p: rng.v(), r: rng.interesting() }, rng.v(), &format!("rand {i}"));
        check(
            C2Circle { p: rng.v_path(), r: rng.pathological() },
            rng.v_path(),
            &format!("randp {i}"),
        );
    }
}

// ===========================================================================
// Rows 16-21 — c2RaytoCircle
// ===========================================================================

fn ray_circle(ray: C2Ray, circle: C2Circle, ctx: &str) {
    let a = apis();
    cmp_ray("c2RaytoCircle", a.c.c2RaytoCircle, a.rust.c2RaytoCircle, ray, circle, ctx);
}

/// A unit-length direction from `p` towards `target`, computed on the Rust
/// side only as *test input*; both libraries then receive the identical bits.
fn unit_towards(p: C2v, target: C2v) -> C2v {
    let (dx, dy) = (target.x - p.x, target.y - p.y);
    let l = (dx * dx + dy * dy).sqrt();
    C2v::new(dx / l, dy / l)
}

#[test]
fn cfg_16_raytocircle_unit_hit() {
    let mut rng = Rng::new(16);
    for i in 0..N_RAY {
        let center = C2v::new(rng.range(-20.0, 20.0), rng.range(-20.0, 20.0));
        let r = rng.range(0.05, 5.0);
        // Origin placed outside the circle at a random bearing.
        let ang = rng.range(0.0, 6.2831855);
        let dist = r + rng.range(0.01, 30.0);
        let origin = C2v::new(center.x + dist * ang.cos(), center.y + dist * ang.sin());
        let d = unit_towards(origin, center);
        let t = dist + rng.range(0.0, 5.0); // long enough to reach
        ray_circle(
            C2Ray { p: origin, d, t },
            C2Circle { p: center, r },
            &format!("rand hit {i}"),
        );
        // Same geometry, but aimed slightly off so the ray may graze or miss.
        let jitter = rng.range(-1.5, 1.5);
        let d2 = unit_towards(
            origin,
            C2v::new(center.x + jitter, center.y + rng.range(-1.5, 1.5)),
        );
        ray_circle(
            C2Ray { p: origin, d: d2, t },
            C2Circle { p: center, r },
            &format!("rand graze {i}"),
        );
    }
}

#[test]
fn cfg_17_raytocircle_nonunit_dir() {
    // The C never normalises A.d, so `t` is measured in |d| units and the
    // `t <= A.t` test scales with it. Feed deliberately un-normalised d.
    let mut rng = Rng::new(17);
    for i in 0..N_RAY {
        let center = C2v::new(rng.range(-10.0, 10.0), rng.range(-10.0, 10.0));
        let r = rng.range(0.05, 4.0);
        let origin = C2v::new(rng.range(-20.0, 20.0), rng.range(-20.0, 20.0));
        let u = unit_towards(origin, center);
        let scale = match rng.next_u32() % 6 {
            0 => 0.0,
            1 => 1e-4,
            2 => 1e4,
            3 => -1.0,
            4 => 2.0,
            _ => rng.range(-8.0, 8.0),
        };
        ray_circle(
            C2Ray { p: origin, d: C2v::new(u.x * scale, u.y * scale), t: rng.range(-5.0, 60.0) },
            C2Circle { p: center, r },
            &format!("scaled {i} scale={}", f(scale)),
        );
        // Fully arbitrary (non-unit, non-aimed) direction.
        ray_circle(
            C2Ray { p: origin, d: rng.v(), t: rng.interesting() },
            C2Circle { p: center, r },
            &format!("arb {i}"),
        );
    }
}

#[test]
fn cfg_18_raytocircle_t_shapes() {
    let mut rng = Rng::new(18);
    let t_shapes = [
        0.0f32, -0.0, 1e-6, 1.0, 1e6, f32::MAX, f32::INFINITY, f32::NEG_INFINITY,
        -1.0, f32::NAN, f32::MIN_POSITIVE,
    ];
    for i in 0..N_RAY {
        let center = C2v::new(rng.range(-10.0, 10.0), rng.range(-10.0, 10.0));
        let r = rng.range(0.1, 4.0);
        let ang = rng.range(0.0, 6.2831855);
        let dist = r + rng.range(0.01, 20.0);
        let origin = C2v::new(center.x + dist * ang.cos(), center.y + dist * ang.sin());
        let d = unit_towards(origin, center);
        for &t in &t_shapes {
            ray_circle(
                C2Ray { p: origin, d, t },
                C2Circle { p: center, r },
                &format!("t={} iter {i}", f(t)),
            );
        }
        // A.t set exactly to the hit distance, and one ULP either side: this is
        // the `t <= A.t` inclusive boundary.
        let hit_t = dist - r;
        for n in [-2i32, -1, 0, 1, 2] {
            ray_circle(
                C2Ray { p: origin, d, t: step(hit_t, n) },
                C2Circle { p: center, r },
                &format!("t=hit{n:+} iter {i}"),
            );
        }
    }
}

#[test]
fn cfg_19_raytocircle_tangent() {
    // disc = b*b - c; the `disc < 0` branch flips exactly at tangency.
    let mut rng = Rng::new(19);
    for i in 0..N_RAY {
        let r = rng.range(0.1, 5.0);
        let center = C2v::new(0.0, 0.0);
        let dist = r + rng.range(0.1, 20.0);
        // Ray travelling +x at perpendicular offset `off` from the centre;
        // tangency is exactly off == r.
        let origin_x = -dist;
        for n in [-3i32, -2, -1, 0, 1, 2, 3] {
            let off = step(r, n);
            ray_circle(
                C2Ray { p: C2v::new(origin_x, off), d: C2v::new(1.0, 0.0), t: 4.0 * dist },
                C2Circle { p: center, r },
                &format!("tangent{n:+} iter {i} r={}", f(r)),
            );
            ray_circle(
                C2Ray { p: C2v::new(off, origin_x), d: C2v::new(0.0, 1.0), t: 4.0 * dist },
                C2Circle { p: center, r },
                &format!("tangent-y{n:+} iter {i}"),
            );
        }
        // Random near-tangent offsets.
        let off = r * rng.range(0.995, 1.005);
        ray_circle(
            C2Ray { p: C2v::new(origin_x, off), d: C2v::new(1.0, 0.0), t: 4.0 * dist },
            C2Circle { p: center, r },
            &format!("near-tangent {i}"),
        );
    }
}

#[test]
fn cfg_20_raytocircle_origin_inside_or_on() {
    let mut rng = Rng::new(20);
    for i in 0..N_RAY {
        let center = C2v::new(rng.range(-10.0, 10.0), rng.range(-10.0, 10.0));
        let r = rng.range(0.1, 5.0);
        let ang = rng.range(0.0, 6.2831855);
        // Strictly inside: t comes out negative, so the C rejects.
        let frac = rng.range(0.0, 0.999);
        let inside = C2v::new(center.x + r * frac * ang.cos(), center.y + r * frac * ang.sin());
        ray_circle(
            C2Ray { p: inside, d: C2v::new(ang.cos(), ang.sin()), t: 10.0 * r },
            C2Circle { p: center, r },
            &format!("inside {i}"),
        );
        // Exactly at the centre.
        ray_circle(
            C2Ray { p: center, d: C2v::new(1.0, 0.0), t: 10.0 * r },
            C2Circle { p: center, r },
            &format!("centre {i}"),
        );
        // Exactly on the surface: t == 0 is accepted, and the impact point
        // equals the origin, so out->n = c2Norm(0,0) -- a NaN/inf normal that
        // must match bit-for-bit.
        let on = C2v::new(center.x + r, center.y);
        ray_circle(
            C2Ray { p: on, d: C2v::new(1.0, 0.0), t: 5.0 },
            C2Circle { p: center, r },
            &format!("on-surface out {i}"),
        );
        ray_circle(
            C2Ray { p: on, d: C2v::new(-1.0, 0.0), t: 5.0 },
            C2Circle { p: center, r },
            &format!("on-surface in {i}"),
        );
        // One ULP inside / outside the surface.
        for n in [-1i32, 1] {
            let px = step(center.x + r, n);
            ray_circle(
                C2Ray { p: C2v::new(px, center.y), d: C2v::new(-1.0, 0.0), t: 5.0 },
                C2Circle { p: center, r },
                &format!("surface{n:+} {i}"),
            );
        }
    }
}

#[test]
fn cfg_21_raytocircle_shotgun() {
    let mut rng = Rng::new(21);
    for i in 0..N_RAY * 4 {
        ray_circle(
            C2Ray { p: rng.v_path(), d: rng.v_path(), t: rng.pathological() },
            C2Circle { p: rng.v_path(), r: rng.pathological() },
            &format!("shotgun {i}"),
        );
    }
    // Small-integer lattice: makes exact equalities (t == 0, t == A.t,
    // disc == 0) actually occur instead of being astronomically unlikely.
    let mut rng = Rng::new(2100);
    let g = |r: &mut Rng| (r.next_u32() % 9) as f32 - 4.0;
    for i in 0..N_RAY * 2 {
        let (px, py) = (g(&mut rng), g(&mut rng));
        let (dx, dy) = (g(&mut rng), g(&mut rng));
        let (cx, cy) = (g(&mut rng), g(&mut rng));
        let (t, r) = (g(&mut rng), g(&mut rng));
        ray_circle(
            C2Ray { p: C2v::new(px, py), d: C2v::new(dx, dy), t },
            C2Circle { p: C2v::new(cx, cy), r },
            &format!("lattice {i}"),
        );
    }
}

// ===========================================================================
// Rows 22-31 — c2RaytoAABB
// ===========================================================================

fn ray_aabb(ray: C2Ray, b: C2AABB, ctx: &str) {
    let a = apis();
    cmp_ray("c2RaytoAABB", a.c.c2RaytoAABB, a.rust.c2RaytoAABB, ray, b, ctx);
}

/// Random axis-aligned-ish ray aimed at a face of `b`, offset so it enters
/// through the named side. `face`: 0=-x, 1=+x, 2=-y, 3=+y.
fn face_ray(rng: &mut Rng, b: C2AABB, face: u32) -> C2Ray {
    let (w, h) = (b.max.x - b.min.x, b.max.y - b.min.y);
    let inx = b.min.x + rng.range(0.05, 0.95) * w;
    let iny = b.min.y + rng.range(0.05, 0.95) * h;
    let back = rng.range(0.5, 10.0);
    let (p, d) = match face {
        0 => (C2v::new(b.min.x - back, iny), C2v::new(1.0, 0.0)),
        1 => (C2v::new(b.max.x + back, iny), C2v::new(-1.0, 0.0)),
        2 => (C2v::new(inx, b.min.y - back), C2v::new(0.0, 1.0)),
        _ => (C2v::new(inx, b.max.y + back), C2v::new(0.0, -1.0)),
    };
    C2Ray { p, d, t: back + rng.range(0.0, w.max(h) + 5.0) }
}

fn rand_proper_box(rng: &mut Rng) -> C2AABB {
    let (cx, cy) = (rng.range(-20.0, 20.0), rng.range(-20.0, 20.0));
    let (hw, hh) = (rng.range(0.05, 8.0), rng.range(0.05, 8.0));
    C2AABB { min: C2v::new(cx - hw, cy - hh), max: C2v::new(cx + hw, cy + hh) }
}

#[test]
fn cfg_22_raytoaabb_face_neg_x() {
    let mut rng = Rng::new(22);
    for i in 0..N_RAY {
        let b = rand_proper_box(&mut rng);
        ray_aabb(face_ray(&mut rng, b, 0), b, &format!("-x face {i}"));
    }
}

#[test]
fn cfg_23_raytoaabb_face_pos_x() {
    let mut rng = Rng::new(23);
    for i in 0..N_RAY {
        let b = rand_proper_box(&mut rng);
        ray_aabb(face_ray(&mut rng, b, 1), b, &format!("+x face {i}"));
    }
}

#[test]
fn cfg_24_raytoaabb_face_neg_y() {
    let mut rng = Rng::new(24);
    for i in 0..N_RAY {
        let b = rand_proper_box(&mut rng);
        ray_aabb(face_ray(&mut rng, b, 2), b, &format!("-y face {i}"));
    }
}

#[test]
fn cfg_25_raytoaabb_face_pos_y() {
    let mut rng = Rng::new(25);
    for i in 0..N_RAY {
        let b = rand_proper_box(&mut rng);
        ray_aabb(face_ray(&mut rng, b, 3), b, &format!("+y face {i}"));
    }
}

#[test]
fn cfg_26_raytoaabb_corner_ties() {
    // Diagonal rays through corners produce equal t values, so the `>=`
    // cascade has ties and the FIRST matching arm decides the normal. A
    // reordering bug in the translation only shows up here.
    let mut rng = Rng::new(26);
    let unit = C2AABB { min: C2v::new(-1.0, -1.0), max: C2v::new(1.0, 1.0) };
    let diags = [
        C2v::new(1.0, 1.0), C2v::new(-1.0, 1.0), C2v::new(1.0, -1.0), C2v::new(-1.0, -1.0),
    ];
    for i in 0..N_RAY {
        for &d in &diags {
            let back = rng.range(1.0, 10.0);
            ray_aabb(
                C2Ray { p: C2v::new(-d.x * back, -d.y * back), d, t: back * 3.0 },
                unit,
                &format!("corner {i} d=({},{})", f(d.x), f(d.y)),
            );
        }
        // Symmetric box + exactly diagonal ray on a random scale.
        let s = rng.range(0.1, 50.0);
        let b = C2AABB { min: C2v::new(-s, -s), max: C2v::new(s, s) };
        ray_aabb(
            C2Ray { p: C2v::new(-2.0 * s, -2.0 * s), d: C2v::new(1.0, 1.0), t: 4.0 * s },
            b,
            &format!("sym corner {i}"),
        );
        // Ray whose origin is exactly a corner.
        for &c in &[unit.min, unit.max, C2v::new(unit.min.x, unit.max.y), C2v::new(unit.max.x, unit.min.y)] {
            ray_aabb(
                C2Ray { p: c, d: C2v::new(rng.range(-1.0, 1.0), rng.range(-1.0, 1.0)), t: 3.0 },
                unit,
                &format!("origin-at-corner {i}"),
            );
        }
    }
}

#[test]
fn cfg_27_raytoaabb_origin_inside() {
    let mut rng = Rng::new(27);
    for i in 0..N_RAY {
        let b = rand_proper_box(&mut rng);
        let p = C2v::new(
            b.min.x + rng.range(0.0, 1.0) * (b.max.x - b.min.x),
            b.min.y + rng.range(0.0, 1.0) * (b.max.y - b.min.y),
        );
        let ang = rng.range(0.0, 6.2831855);
        ray_aabb(
            C2Ray { p, d: C2v::new(ang.cos(), ang.sin()), t: rng.range(0.0, 40.0) },
            b,
            &format!("inside {i}"),
        );
        // Origin exactly at the centre and exactly on a face.
        let centre = C2v::new((b.min.x + b.max.x) * 0.5, (b.min.y + b.max.y) * 0.5);
        ray_aabb(C2Ray { p: centre, d: C2v::new(1.0, 0.0), t: 100.0 }, b, &format!("centre {i}"));
        ray_aabb(
            C2Ray { p: C2v::new(b.min.x, centre.y), d: C2v::new(1.0, 0.0), t: 100.0 },
            b,
            &format!("on-min-face {i}"),
        );
    }
}

#[test]
fn cfg_28_raytoaabb_parallel_slabs() {
    // Axis-aligned rays make `da - db == 0` for the perpendicular slabs,
    // hitting the line-132 zero-denominator guard, and `da*db > 0` for slabs
    // the segment stays on one side of (returning 1.0f).
    let mut rng = Rng::new(28);
    for i in 0..N_RAY {
        let b = rand_proper_box(&mut rng);
        let centre_y = (b.min.y + b.max.y) * 0.5;
        let centre_x = (b.min.x + b.max.x) * 0.5;
        // Purely horizontal / vertical, inside and outside the other axis.
        for &(p, d) in &[
            (C2v::new(b.min.x - 5.0, centre_y), C2v::new(1.0, 0.0)),
            (C2v::new(b.min.x - 5.0, b.min.y - 5.0), C2v::new(1.0, 0.0)),
            (C2v::new(b.min.x - 5.0, b.min.y), C2v::new(1.0, 0.0)),
            (C2v::new(b.min.x - 5.0, b.max.y), C2v::new(1.0, 0.0)),
            (C2v::new(centre_x, b.min.y - 5.0), C2v::new(0.0, 1.0)),
            (C2v::new(b.min.x, b.min.y - 5.0), C2v::new(0.0, 1.0)),
            (C2v::new(b.max.x, b.min.y - 5.0), C2v::new(0.0, 1.0)),
            // Zero direction: p1 == p0 so ab == 0 and every slab has d == 0.
            (C2v::new(centre_x, centre_y), C2v::new(0.0, 0.0)),
            (C2v::new(b.min.x - 5.0, centre_y), C2v::new(0.0, 0.0)),
            (C2v::new(centre_x, centre_y), C2v::new(-0.0, -0.0)),
        ] {
            ray_aabb(C2Ray { p, d, t: rng.range(0.0, 30.0) }, b, &format!("parallel {i}"));
        }
    }
}

#[test]
fn cfg_29_raytoaabb_box_shapes() {
    let mut rng = Rng::new(29);
    for i in 0..N_RAY {
        let (cx, cy) = (rng.range(-5.0, 5.0), rng.range(-5.0, 5.0));
        let shapes = [
            // degenerate point
            C2AABB { min: C2v::new(cx, cy), max: C2v::new(cx, cy) },
            // zero width, non-zero height
            C2AABB { min: C2v::new(cx, cy - 1.0), max: C2v::new(cx, cy + 1.0) },
            // zero height, non-zero width
            C2AABB { min: C2v::new(cx - 1.0, cy), max: C2v::new(cx + 1.0, cy) },
            // inverted on x
            C2AABB { min: C2v::new(cx + 1.0, cy - 1.0), max: C2v::new(cx - 1.0, cy + 1.0) },
            // inverted on both
            C2AABB { min: C2v::new(cx + 1.0, cy + 1.0), max: C2v::new(cx - 1.0, cy - 1.0) },
            // huge
            C2AABB { min: C2v::new(-1e30, -1e30), max: C2v::new(1e30, 1e30) },
            // tiny
            C2AABB { min: C2v::new(cx, cy), max: C2v::new(cx + 1e-6, cy + 1e-6) },
            // f32::MAX extents -- half_extents overflows to inf
            C2AABB { min: C2v::new(-f32::MAX, -f32::MAX), max: C2v::new(f32::MAX, f32::MAX) },
            // signed zeros
            C2AABB { min: C2v::new(-0.0, -0.0), max: C2v::new(0.0, 0.0) },
            // infinite
            C2AABB { min: C2v::new(f32::NEG_INFINITY, f32::NEG_INFINITY), max: C2v::new(f32::INFINITY, f32::INFINITY) },
        ];
        let ang = rng.range(0.0, 6.2831855);
        let ray = C2Ray {
            p: C2v::new(rng.range(-10.0, 10.0), rng.range(-10.0, 10.0)),
            d: C2v::new(ang.cos(), ang.sin()),
            t: rng.range(0.0, 40.0),
        };
        for (j, &b) in shapes.iter().enumerate() {
            ray_aabb(ray, b, &format!("shape {j} iter {i}"));
            // Also aim straight at the shape's min corner.
            ray_aabb(
                C2Ray { p: C2v::new(b.min.x - 3.0, b.min.y), d: C2v::new(1.0, 0.0), t: 6.0 },
                b,
                &format!("shape {j} aimed iter {i}"),
            );
        }
    }
}

#[test]
fn cfg_30_raytoaabb_t_shapes() {
    let mut rng = Rng::new(30);
    let t_shapes = [
        0.0f32, -0.0, -1.0, 1e-6, 1.0, 1e6, f32::MAX, f32::INFINITY,
        f32::NEG_INFINITY, f32::NAN, f32::MIN_POSITIVE,
    ];
    for i in 0..N_RAY {
        let b = rand_proper_box(&mut rng);
        let centre_y = (b.min.y + b.max.y) * 0.5;
        let ang = rng.range(0.0, 6.2831855);
        for &t in &t_shapes {
            // unit direction
            ray_aabb(
                C2Ray { p: C2v::new(b.min.x - 2.0, centre_y), d: C2v::new(1.0, 0.0), t },
                b,
                &format!("t={} unit {i}", f(t)),
            );
            // non-unit direction
            let s = rng.range(-6.0, 6.0);
            ray_aabb(
                C2Ray {
                    p: C2v::new(b.min.x - 2.0, centre_y),
                    d: C2v::new(ang.cos() * s, ang.sin() * s),
                    t,
                },
                b,
                &format!("t={} nonunit {i}", f(t)),
            );
        }
    }
}

#[test]
fn cfg_31_raytoaabb_shotgun() {
    let mut rng = Rng::new(31);
    for i in 0..N_RAY * 4 {
        ray_aabb(
            C2Ray { p: rng.v_path(), d: rng.v_path(), t: rng.pathological() },
            C2AABB { min: rng.v_path(), max: rng.v_path() },
            &format!("shotgun {i}"),
        );
    }
    let mut rng = Rng::new(3100);
    let g = |r: &mut Rng| (r.next_u32() % 9) as f32 - 4.0;
    for i in 0..N_RAY * 3 {
        ray_aabb(
            C2Ray {
                p: C2v::new(g(&mut rng), g(&mut rng)),
                d: C2v::new(g(&mut rng), g(&mut rng)),
                t: g(&mut rng),
            },
            C2AABB {
                min: C2v::new(g(&mut rng), g(&mut rng)),
                max: C2v::new(g(&mut rng), g(&mut rng)),
            },
            &format!("lattice {i}"),
        );
    }
}

// ===========================================================================
// Rows 32-41 — c2RaytoCapsule
// ===========================================================================

fn ray_capsule(ray: C2Ray, cap: C2Capsule, ctx: &str) {
    let a = apis();
    cmp_ray("c2RaytoCapsule", a.c.c2RaytoCapsule, a.rust.c2RaytoCapsule, ray, cap, ctx);
}

/// Builds a capsule with a random centre, orientation, length and radius.
fn rand_capsule(rng: &mut Rng) -> C2Capsule {
    let (cx, cy) = (rng.range(-15.0, 15.0), rng.range(-15.0, 15.0));
    let ang = rng.range(0.0, 6.2831855);
    let half = rng.range(0.1, 8.0);
    let (ux, uy) = (ang.cos(), ang.sin());
    C2Capsule {
        a: C2v::new(cx - ux * half, cy - uy * half),
        b: C2v::new(cx + ux * half, cy + uy * half),
        r: rng.range(0.05, 4.0),
    }
}

/// Capsule-local frame, mirroring `M` in the C so a test can place the ray
/// origin in a chosen region. Computed test-side only.
fn capsule_frame(cap: C2Capsule) -> (C2v, C2v, f32) {
    let (dx, dy) = (cap.b.x - cap.a.x, cap.b.y - cap.a.y);
    let l = (dx * dx + dy * dy).sqrt();
    let my = C2v::new(dx / l, dy / l); // M.y
    let mx = C2v::new(my.y, -my.x); // M.x = c2CCW90(M.y)
    // yBb.y = dot(M.y, cap_n) = l
    (mx, my, l)
}

/// Maps capsule-local (u along M.x, v along M.y) to world space.
/// `c2MulmvT` computes `(dot(M.x, w), dot(M.y, w))`, so the inverse is
/// `w = u*M.x + v*M.y`.
fn capsule_local_to_world(cap: C2Capsule, mx: C2v, my: C2v, u: f32, v: f32) -> C2v {
    C2v::new(cap.a.x + u * mx.x + v * my.x, cap.a.y + u * mx.y + v * my.y)
}

#[test]
fn cfg_32_raytocapsule_origin_in_slab() {
    // capsule_bb = {min: (-r, 0), max: (r, yBb.y)}; c2AABBtoPoint on yAp is
    // inclusive, so an origin anywhere in that rectangle takes the early
    // `return 1` with out->t = 0 and out->n = c2Norm(b-a).
    let mut rng = Rng::new(32);
    for i in 0..N_RAY {
        let cap = rand_capsule(&mut rng);
        let (mx, my, l) = capsule_frame(cap);
        for &(u, v) in &[
            (0.0f32, l * 0.5),
            (cap.r * 0.5, l * 0.5),
            (-cap.r * 0.5, l * 0.25),
            (cap.r, l),            // exact corner (inclusive)
            (-cap.r, 0.0),         // exact corner
            (cap.r, 0.0),
            (-cap.r, l),
            (0.0, 0.0),            // exactly at a
            (0.0, l),              // exactly at b
        ] {
            let p = capsule_local_to_world(cap, mx, my, u, v);
            let ang = rng.range(0.0, 6.2831855);
            ray_capsule(
                C2Ray { p, d: C2v::new(ang.cos(), ang.sin()), t: rng.range(0.0, 20.0) },
                cap,
                &format!("in-slab u={} v={} iter {i}", f(u), f(v)),
            );
        }
    }
}

#[test]
fn cfg_33_raytocapsule_origin_in_caps() {
    // Outside the slab rectangle but strictly inside end cap A or B, so
    // c2CircleToPoint takes the `return 1` at line 255 / 257.
    let mut rng = Rng::new(33);
    for i in 0..N_RAY {
        let cap = rand_capsule(&mut rng);
        let (mx, my, l) = capsule_frame(cap);
        // Just below a (v < 0) and just above b (v > l), within radius.
        for &(u, v) in &[
            (0.0f32, -cap.r * 0.5),
            (cap.r * 0.3, -cap.r * 0.3),
            (0.0, l + cap.r * 0.5),
            (-cap.r * 0.3, l + cap.r * 0.3),
            // Exactly on the cap rim -- c2CircleToPoint is EXCLUSIVE, so these
            // must be rejected by the cap test and fall through.
            (0.0, -cap.r),
            (0.0, l + cap.r),
        ] {
            let p = capsule_local_to_world(cap, mx, my, u, v);
            let ang = rng.range(0.0, 6.2831855);
            ray_capsule(
                C2Ray { p, d: C2v::new(ang.cos(), ang.sin()), t: rng.range(0.0, 20.0) },
                cap,
                &format!("in-cap u={} v={} iter {i}", f(u), f(v)),
            );
        }
    }
}

#[test]
fn cfg_34_raytocapsule_delegate_by_yAp_y() {
    // abs(yAp.x) < B.r with the origin OUTSIDE the slab (so v < 0 or v > l)
    // and outside both caps: line 270 is true, and yAp.y's sign picks Ca or Cb.
    let mut rng = Rng::new(34);
    for i in 0..N_RAY {
        let cap = rand_capsule(&mut rng);
        let (mx, my, l) = capsule_frame(cap);
        for &(u, v) in &[
            (0.0f32, -cap.r - 3.0),        // yAp.y < 0 -> Ca
            (cap.r * 0.5, -cap.r - 10.0),  // yAp.y < 0 -> Ca
            (0.0, l + cap.r + 3.0),        // yAp.y > 0 -> Cb
            (-cap.r * 0.5, l + cap.r + 9.0),
            (0.0, -0.0),                   // yAp.y == -0.0: `< 0` is FALSE -> Cb
        ] {
            let p = capsule_local_to_world(cap, mx, my, u, v);
            // Aim back towards the capsule so the delegate can actually hit.
            let mid = C2v::new((cap.a.x + cap.b.x) * 0.5, (cap.a.y + cap.b.y) * 0.5);
            let d = unit_towards(p, mid);
            for &t in &[0.0f32, 1.0, 100.0, rng.range(0.0, 60.0), -1.0, f32::INFINITY] {
                ray_capsule(
                    C2Ray { p, d, t },
                    cap,
                    &format!("delegate u={} v={} t={} iter {i}", f(u), f(v), f(t)),
                );
            }
            let ang = rng.range(0.0, 6.2831855);
            ray_capsule(
                C2Ray { p, d: C2v::new(ang.cos(), ang.sin()), t: rng.range(0.0, 60.0) },
                cap,
                &format!("delegate-rand u={} v={} iter {i}", f(u), f(v)),
            );
        }
    }
}

/// Places the origin at local `(u0, v0)` and the endpoint at `(u1, v1)` so the
/// segment crosses `x = c` at a controlled `y`, exercising rows 35-37.
fn capsule_crossing_ray(
    rng: &mut Rng,
    cap: C2Capsule,
    mx: C2v,
    my: C2v,
    u0: f32,
    v0: f32,
    u1: f32,
    v1: f32,
) -> C2Ray {
    let p = capsule_local_to_world(cap, mx, my, u0, v0);
    let e = capsule_local_to_world(cap, mx, my, u1, v1);
    let (dx, dy) = (e.x - p.x, e.y - p.y);
    let len = (dx * dx + dy * dy).sqrt();
    // Half the time hand over a unit direction with t == len, half the time a
    // non-unit direction with t == 1, which reach the same endpoint but take
    // different `t` scaling through `out->t = t * A.t`.
    if rng.next_u32() % 2 == 0 {
        C2Ray { p, d: C2v::new(dx / len, dy / len), t: len }
    } else {
        C2Ray { p, d: C2v::new(dx, dy), t: 1.0 }
    }
}

#[test]
fn cfg_35_raytocapsule_slab_hit_pos_c() {
    // Origin at u > r (so c = +B.r) crossing into the slab at 0 < y < yBb.y:
    // the `else` branch takes `out->n = M.x`, `out->t = t * A.t`.
    let mut rng = Rng::new(35);
    for i in 0..N_RAY {
        let cap = rand_capsule(&mut rng);
        let (mx, my, l) = capsule_frame(cap);
        let v_mid = l * rng.range(0.15, 0.85);
        let u0 = cap.r + rng.range(0.2, 10.0);
        let ray = capsule_crossing_ray(&mut rng, cap, mx, my, u0, v_mid, -u0, v_mid);
        ray_capsule(ray, cap, &format!("pos-c straight {i}"));
        // Oblique crossing, still landing inside (0, yBb.y).
        let v1 = l * rng.range(0.05, 0.95);
        let ray = capsule_crossing_ray(&mut rng, cap, mx, my, u0, v_mid, -u0, v1);
        ray_capsule(ray, cap, &format!("pos-c oblique {i}"));
        // Crossing exactly at y == 0 and y == yBb.y (the <= / >= boundaries).
        for &vt in &[0.0f32, l, step(0.0, 1), step(l, -1)] {
            let ray = capsule_crossing_ray(&mut rng, cap, mx, my, u0, vt, -u0, vt);
            ray_capsule(ray, cap, &format!("pos-c boundary v={} {i}", f(vt)));
        }
    }
}

#[test]
fn cfg_36_raytocapsule_slab_hit_neg_c() {
    // Mirror of row 35 from u < -r, so c = -B.r and out->n = c2Skew(M.y).
    let mut rng = Rng::new(36);
    for i in 0..N_RAY {
        let cap = rand_capsule(&mut rng);
        let (mx, my, l) = capsule_frame(cap);
        let v_mid = l * rng.range(0.15, 0.85);
        let u0 = -(cap.r + rng.range(0.2, 10.0));
        let ray = capsule_crossing_ray(&mut rng, cap, mx, my, u0, v_mid, -u0, v_mid);
        ray_capsule(ray, cap, &format!("neg-c straight {i}"));
        let v1 = l * rng.range(0.05, 0.95);
        let ray = capsule_crossing_ray(&mut rng, cap, mx, my, u0, v_mid, -u0, v1);
        ray_capsule(ray, cap, &format!("neg-c oblique {i}"));
        // u0 exactly -B.r: `abs(yAp.x) < B.r` is false (not <), so it takes the
        // else arm with c = -B.r since yAp.x > 0 is false.
        let ray = capsule_crossing_ray(&mut rng, cap, mx, my, -cap.r, v_mid, cap.r, v_mid);
        ray_capsule(ray, cap, &format!("neg-c exact-r {i}"));
        let ray = capsule_crossing_ray(&mut rng, cap, mx, my, cap.r, v_mid, -cap.r, v_mid);
        ray_capsule(ray, cap, &format!("pos-c exact-r {i}"));
    }
}

#[test]
fn cfg_37_raytocapsule_cap_by_y() {
    // Crossing x = c at y <= 0 (delegate to Ca) or y >= yBb.y (delegate to Cb).
    let mut rng = Rng::new(37);
    for i in 0..N_RAY {
        let cap = rand_capsule(&mut rng);
        let (mx, my, l) = capsule_frame(cap);
        let u0 = cap.r + rng.range(0.2, 8.0);
        for &(v0, v1) in &[
            (l * 0.5f32, -l),            // ends below 0 -> Ca
            (l * 0.5, 2.0 * l),          // ends above yBb.y -> Cb
            (-l, l * 0.5),
            (2.0 * l, l * 0.5),
            (0.0, -l),
            (l, 2.0 * l),
        ] {
            let ray = capsule_crossing_ray(&mut rng, cap, mx, my, u0, v0, -u0, v1);
            ray_capsule(ray, cap, &format!("cap-by-y +u v0={} v1={} {i}", f(v0), f(v1)));
            let ray = capsule_crossing_ray(&mut rng, cap, mx, my, -u0, v0, u0, v1);
            ray_capsule(ray, cap, &format!("cap-by-y -u v0={} v1={} {i}", f(v0), f(v1)));
        }
        // Endpoint with the same u sign as the origin: yAe.x * yAp.x > 0 and
        // min(|yAe.x|,|yAp.x|) may or may not be < r -- both sub-cases.
        for &u1 in &[u0 * 0.5, cap.r * 0.5, u0 * 2.0] {
            let ray = capsule_crossing_ray(&mut rng, cap, mx, my, u0, l * 0.5, u1, l * 0.5);
            ray_capsule(ray, cap, &format!("same-side u1={} {i}", f(u1)));
        }
    }
}

#[test]
fn cfg_38_raytocapsule_orientations() {
    let mut rng = Rng::new(38);
    for i in 0..N_RAY {
        let r = rng.range(0.05, 4.0);
        let half = rng.range(0.1, 8.0);
        let caps = [
            // vertical, a below b
            C2Capsule { a: C2v::new(0.0, -half), b: C2v::new(0.0, half), r },
            // vertical REVERSED: yBb.y is negative, so capsule_bb is inverted
            C2Capsule { a: C2v::new(0.0, half), b: C2v::new(0.0, -half), r },
            // horizontal both ways
            C2Capsule { a: C2v::new(-half, 0.0), b: C2v::new(half, 0.0), r },
            C2Capsule { a: C2v::new(half, 0.0), b: C2v::new(-half, 0.0), r },
            // 45 degrees both ways
            C2Capsule { a: C2v::new(-half, -half), b: C2v::new(half, half), r },
            C2Capsule { a: C2v::new(half, half), b: C2v::new(-half, -half), r },
            // arbitrary
            rand_capsule(&mut rng),
        ];
        let ang = rng.range(0.0, 6.2831855);
        for (j, &cap) in caps.iter().enumerate() {
            let start = C2v::new(rng.range(-20.0, 20.0), rng.range(-20.0, 20.0));
            ray_capsule(
                C2Ray { p: start, d: C2v::new(ang.cos(), ang.sin()), t: rng.range(0.0, 50.0) },
                cap,
                &format!("orient {j} rand {i}"),
            );
            let mid = C2v::new((cap.a.x + cap.b.x) * 0.5, (cap.a.y + cap.b.y) * 0.5);
            ray_capsule(
                C2Ray { p: start, d: unit_towards(start, mid), t: 100.0 },
                cap,
                &format!("orient {j} aimed {i}"),
            );
        }
    }
}

#[test]
fn cfg_39_raytocapsule_radius_shapes() {
    let mut rng = Rng::new(39);
    for i in 0..N_RAY {
        let base = rand_capsule(&mut rng);
        let mid = C2v::new((base.a.x + base.b.x) * 0.5, (base.a.y + base.b.y) * 0.5);
        for &r in &[
            0.0f32, -0.0, -1.0, -base.r, 1e-6, 1e6, f32::MIN_POSITIVE,
            f32::MAX, f32::INFINITY, f32::NAN, base.r,
        ] {
            let cap = C2Capsule { r, ..base };
            let start = C2v::new(rng.range(-25.0, 25.0), rng.range(-25.0, 25.0));
            ray_capsule(
                C2Ray { p: start, d: unit_towards(start, mid), t: 200.0 },
                cap,
                &format!("r={} aimed {i}", f(r)),
            );
            let ang = rng.range(0.0, 6.2831855);
            ray_capsule(
                C2Ray { p: start, d: C2v::new(ang.cos(), ang.sin()), t: rng.range(-5.0, 50.0) },
                cap,
                &format!("r={} rand {i}", f(r)),
            );
            // Origin near the axis, where a negative/zero radius makes
            // capsule_bb inconsistent.
            ray_capsule(
                C2Ray { p: mid, d: C2v::new(1.0, 0.0), t: 10.0 },
                cap,
                &format!("r={} at-mid {i}", f(r)),
            );
        }
    }
}

#[test]
fn cfg_40_raytocapsule_degenerate_axis() {
    // B.a == B.b makes c2Norm(0,0) divide by zero, so M.y is NaN/inf and every
    // downstream comparison follows the NaN paths.
    let mut rng = Rng::new(40);
    for i in 0..N_RAY {
        let p0 = C2v::new(rng.range(-10.0, 10.0), rng.range(-10.0, 10.0));
        for &r in &[1.0f32, 0.0, -1.0, f32::NAN, f32::INFINITY] {
            let cap = C2Capsule { a: p0, b: p0, r };
            let start = C2v::new(rng.range(-20.0, 20.0), rng.range(-20.0, 20.0));
            let ang = rng.range(0.0, 6.2831855);
            ray_capsule(
                C2Ray { p: start, d: C2v::new(ang.cos(), ang.sin()), t: rng.range(0.0, 40.0) },
                cap,
                &format!("degenerate r={} {i}", f(r)),
            );
            // Origin exactly at the degenerate point.
            ray_capsule(
                C2Ray { p: p0, d: C2v::new(1.0, 0.0), t: 5.0 },
                cap,
                &format!("degenerate-at r={} {i}", f(r)),
            );
        }
        // Nearly-degenerate: b - a is a denormal, so c2Len underflows.
        let cap = C2Capsule {
            a: p0,
            b: C2v::new(p0.x + f32::from_bits(1), p0.y),
            r: rng.range(0.1, 2.0),
        };
        ray_capsule(
            C2Ray { p: C2v::new(p0.x + 5.0, p0.y), d: C2v::new(-1.0, 0.0), t: 10.0 },
            cap,
            &format!("near-degenerate {i}"),
        );
        // Signed-zero axis.
        let cap = C2Capsule { a: C2v::new(0.0, 0.0), b: C2v::new(-0.0, -0.0), r: 1.0 };
        ray_capsule(
            C2Ray { p: C2v::new(-3.0, 0.0), d: C2v::new(1.0, 0.0), t: 6.0 },
            cap,
            &format!("signed-zero axis {i}"),
        );
    }
}

#[test]
fn cfg_41_raytocapsule_shotgun() {
    let mut rng = Rng::new(41);
    for i in 0..N_RAY * 4 {
        ray_capsule(
            C2Ray { p: rng.v_path(), d: rng.v_path(), t: rng.pathological() },
            C2Capsule { a: rng.v_path(), b: rng.v_path(), r: rng.pathological() },
            &format!("shotgun {i}"),
        );
    }
    let mut rng = Rng::new(4100);
    let g = |r: &mut Rng| (r.next_u32() % 9) as f32 - 4.0;
    for i in 0..N_RAY * 3 {
        ray_capsule(
            C2Ray {
                p: C2v::new(g(&mut rng), g(&mut rng)),
                d: C2v::new(g(&mut rng), g(&mut rng)),
                t: g(&mut rng),
            },
            C2Capsule {
                a: C2v::new(g(&mut rng), g(&mut rng)),
                b: C2v::new(g(&mut rng), g(&mut rng)),
                r: g(&mut rng),
            },
            &format!("lattice {i}"),
        );
    }
}

// ===========================================================================
// Rows 42-44 — c2CastRay dispatcher (all three valid tags)
// ===========================================================================

#[test]
fn cfg_42_castray_circle() {
    let a = apis();
    let mut rng = Rng::new(42);
    for i in 0..N_RAY {
        let circle = C2Circle { p: rng.v(), r: rng.interesting() };
        let ray = C2Ray { p: rng.v(), d: rng.v(), t: rng.interesting() };
        cmp_castray(ray, &circle, C2_TYPE_CIRCLE, &format!("circle {i}"));
        // The dispatcher must agree with the direct call, in both libraries.
        let mut direct_c = C2Raycast::poison();
        let mut via_c = C2Raycast::poison();
        let rd = unsafe { (a.c.c2RaytoCircle)(ray, circle, &mut direct_c) };
        let rv = unsafe {
            (a.c.c2CastRay)(ray, &circle as *const _ as *const std::ffi::c_void, C2_TYPE_CIRCLE, &mut via_c)
        };
        assert_eq!(
            (rd, direct_c.bits()),
            (rv, via_c.bits()),
            "C: c2CastRay(CIRCLE) disagrees with c2RaytoCircle at iter {i}"
        );
        let mut direct_r = C2Raycast::poison();
        let mut via_r = C2Raycast::poison();
        let rd = unsafe { (a.rust.c2RaytoCircle)(ray, circle, &mut direct_r) };
        let rv = unsafe {
            (a.rust.c2CastRay)(ray, &circle as *const _ as *const std::ffi::c_void, C2_TYPE_CIRCLE, &mut via_r)
        };
        assert_eq!(
            (rd, direct_r.bits()),
            (rv, via_r.bits()),
            "Rust: c2CastRay(CIRCLE) disagrees with c2RaytoCircle at iter {i}"
        );
        // Pathological variant.
        let circle = C2Circle { p: rng.v_path(), r: rng.pathological() };
        let ray = C2Ray { p: rng.v_path(), d: rng.v_path(), t: rng.pathological() };
        cmp_castray(ray, &circle, C2_TYPE_CIRCLE, &format!("circle-path {i}"));
    }
}

#[test]
fn cfg_43_castray_aabb() {
    let mut rng = Rng::new(43);
    for i in 0..N_RAY {
        let b = rand_proper_box(&mut rng);
        let face = rng.next_u32() % 4;
        let ray = face_ray(&mut rng, b, face);
        cmp_castray(ray, &b, C2_TYPE_AABB, &format!("aabb aimed {i}"));
        let b = C2AABB { min: rng.v_path(), max: rng.v_path() };
        let ray = C2Ray { p: rng.v_path(), d: rng.v_path(), t: rng.pathological() };
        cmp_castray(ray, &b, C2_TYPE_AABB, &format!("aabb-path {i}"));
    }
}

#[test]
fn cfg_44_castray_capsule() {
    let mut rng = Rng::new(44);
    for i in 0..N_RAY {
        let cap = rand_capsule(&mut rng);
        let mid = C2v::new((cap.a.x + cap.b.x) * 0.5, (cap.a.y + cap.b.y) * 0.5);
        let start = C2v::new(rng.range(-25.0, 25.0), rng.range(-25.0, 25.0));
        cmp_castray(
            C2Ray { p: start, d: unit_towards(start, mid), t: 100.0 },
            &cap,
            C2_TYPE_CAPSULE,
            &format!("capsule aimed {i}"),
        );
        let cap = C2Capsule { a: rng.v_path(), b: rng.v_path(), r: rng.pathological() };
        let ray = C2Ray { p: rng.v_path(), d: rng.v_path(), t: rng.pathological() };
        cmp_castray(ray, &cap, C2_TYPE_CAPSULE, &format!("capsule-path {i}"));
    }
}

// ===========================================================================
// Rows 45-48 — spec_ray (the only symbol in include/lib.h)
// ===========================================================================

#[test]
fn cfg_45_specray_hit() {
    // spec_ray derives d = norm(mp - ray.p) and t = dot(mp,d) - dot(ray.p,d),
    // i.e. the ray reaches exactly as far as the mouse point. Put the circle
    // between the origin and the mouse point so it hits.
    let mut rng = Rng::new(45);
    for i in 0..N_RAY {
        let origin = C2v::new(rng.range(-20.0, 20.0), rng.range(-20.0, 20.0));
        let ang = rng.range(0.0, 6.2831855);
        let r = rng.range(0.05, 4.0);
        let d_centre = r + rng.range(0.1, 15.0);
        let centre = C2v::new(origin.x + d_centre * ang.cos(), origin.y + d_centre * ang.sin());
        // Mouse point beyond the circle along the same bearing.
        let d_mouse = d_centre + r + rng.range(0.01, 15.0);
        let mp = C2v::new(origin.x + d_mouse * ang.cos(), origin.y + d_mouse * ang.sin());
        cmp_specray(
            [mp.x, mp.y, centre.x, centre.y, r, origin.x, origin.y],
            &format!("hit {i}"),
        );
        // Mouse point exactly at the near surface: t is exactly the hit
        // distance, the inclusive `t <= A.t` boundary.
        let d_surface = d_centre - r;
        let mp2 = C2v::new(origin.x + d_surface * ang.cos(), origin.y + d_surface * ang.sin());
        cmp_specray(
            [mp2.x, mp2.y, centre.x, centre.y, r, origin.x, origin.y],
            &format!("exact-surface {i}"),
        );
        // Off-axis mouse point (ray no longer aimed at the centre).
        let jx = rng.range(-3.0, 3.0);
        let jy = rng.range(-3.0, 3.0);
        cmp_specray(
            [mp.x + jx, mp.y + jy, centre.x, centre.y, r, origin.x, origin.y],
            &format!("off-axis {i}"),
        );
    }
}

#[test]
fn cfg_46_specray_short_and_behind() {
    let mut rng = Rng::new(46);
    for i in 0..N_RAY {
        let origin = C2v::new(rng.range(-20.0, 20.0), rng.range(-20.0, 20.0));
        let ang = rng.range(0.0, 6.2831855);
        let r = rng.range(0.05, 4.0);
        let d_centre = r + rng.range(2.0, 15.0);
        let centre = C2v::new(origin.x + d_centre * ang.cos(), origin.y + d_centre * ang.sin());
        // Mouse point short of the circle -> ray.t too small.
        let d_short = (d_centre - r) * rng.range(0.0, 0.99);
        let mp = C2v::new(origin.x + d_short * ang.cos(), origin.y + d_short * ang.sin());
        cmp_specray([mp.x, mp.y, centre.x, centre.y, r, origin.x, origin.y], &format!("short {i}"));
        // Mouse point on the OPPOSITE side: d flips, so the circle is behind
        // the ray and ray.t is positive but pointing away.
        let d_back = rng.range(0.5, 15.0);
        let mpb = C2v::new(origin.x - d_back * ang.cos(), origin.y - d_back * ang.sin());
        cmp_specray([mpb.x, mpb.y, centre.x, centre.y, r, origin.x, origin.y], &format!("behind {i}"));
        // Mouse point sideways so the ray misses entirely.
        let mps = C2v::new(origin.x - d_back * ang.sin(), origin.y + d_back * ang.cos());
        cmp_specray([mps.x, mps.y, centre.x, centre.y, r, origin.x, origin.y], &format!("sideways {i}"));
    }
}

#[test]
fn cfg_47_specray_origin_inside_or_on() {
    let mut rng = Rng::new(47);
    for i in 0..N_RAY {
        let centre = C2v::new(rng.range(-15.0, 15.0), rng.range(-15.0, 15.0));
        let r = rng.range(0.1, 5.0);
        let ang = rng.range(0.0, 6.2831855);
        // Strictly inside.
        let frac = rng.range(0.0, 0.999);
        let origin = C2v::new(centre.x + r * frac * ang.cos(), centre.y + r * frac * ang.sin());
        let mp = C2v::new(centre.x + r * 5.0 * ang.cos(), centre.y + r * 5.0 * ang.sin());
        cmp_specray([mp.x, mp.y, centre.x, centre.y, r, origin.x, origin.y], &format!("inside {i}"));
        // Exactly at the centre.
        cmp_specray([mp.x, mp.y, centre.x, centre.y, r, centre.x, centre.y], &format!("at-centre {i}"));
        // Exactly on the surface, and one ULP either side.
        for n in [-1i32, 0, 1] {
            let ox = step(centre.x + r, n);
            cmp_specray(
                [mp.x, mp.y, centre.x, centre.y, r, ox, centre.y],
                &format!("surface{n:+} {i}"),
            );
        }
        // mp == ray.p: c2Norm(0,0) divides by zero, so d and t are NaN.
        cmp_specray(
            [origin.x, origin.y, centre.x, centre.y, r, origin.x, origin.y],
            &format!("mp==origin {i}"),
        );
    }
}

#[test]
fn cfg_48_specray_shotgun() {
    let mut rng = Rng::new(48);
    for i in 0..N_RAY * 4 {
        cmp_specray(
            [
                rng.pathological(), rng.pathological(), rng.pathological(), rng.pathological(),
                rng.pathological(), rng.pathological(), rng.pathological(),
            ],
            &format!("shotgun {i}"),
        );
    }
    let mut rng = Rng::new(4800);
    let g = |r: &mut Rng| (r.next_u32() % 9) as f32 - 4.0;
    for i in 0..N_RAY * 3 {
        cmp_specray(
            [
                g(&mut rng), g(&mut rng), g(&mut rng), g(&mut rng),
                g(&mut rng), g(&mut rng), g(&mut rng),
            ],
            &format!("lattice {i}"),
        );
    }
    // Full edge-value cross-product on the two most influential arguments.
    let mut pool: Vec<f32> = EDGE_F32.to_vec();
    pool.extend(exotic_f32());
    for &u in &pool {
        for &v in &pool {
            cmp_specray([u, v, 0.0, 0.0, 1.0, 3.0, 0.0], "edge mp");
            cmp_specray([5.0, 0.0, 0.0, 0.0, u, v, 0.0], "edge r/origin");
            cmp_specray([5.0, 0.0, u, v, 1.0, 0.0, 0.0], "edge centre");
        }
    }
}

// ===========================================================================
// Row 49 — cross-library composed pipeline
// ===========================================================================

/// Reproduces the internal pipeline of `c2RaytoCapsule` out of individually
/// exported calls, letting each stage be taken from either library. Every one
/// of the 2^7 mixes must produce identical bits; a wrapper pair that is wrong
/// in the *same* way still passes a per-wrapper test but fails here.
#[test]
fn cfg_49_cross_library_pipeline() {
    let a = apis();
    let mut rng = Rng::new(49);
    for i in 0..600 {
        let cap = if i % 3 == 0 {
            C2Capsule { a: rng.v_path(), b: rng.v_path(), r: rng.pathological() }
        } else {
            rand_capsule(&mut rng)
        };
        let ray = if i % 3 == 0 {
            C2Ray { p: rng.v_path(), d: rng.v_path(), t: rng.pathological() }
        } else {
            C2Ray { p: rng.v(), d: rng.v(), t: rng.interesting() }
        };

        let mut results: Vec<(u32, (i32, (u32, u32), (u32, u32, u32)))> = Vec::new();
        for mask in 0u32..128 {
            let pick = |bit: u32| if mask & (1 << bit) != 0 { &a.rust } else { &a.c };

            // stage 0: cap_n = c2Sub(b, a)
            let cap_n = unsafe { (pick(0).c2Sub)(cap.b, cap.a) };
            // stage 1: M.y = c2Norm(cap_n)
            let my = unsafe { (pick(1).c2Norm)(cap_n) };
            // stage 2: M.x = c2CCW90(M.y)
            let mx = unsafe { (pick(2).c2CCW90)(my) };
            let m = C2m { x: mx, y: my };
            // stage 3: yBb = c2MulmvT(M, cap_n)
            let ybb = unsafe { (pick(3).c2MulmvT)(m, cap_n) };
            // stage 4: yAp = c2MulmvT(M, c2Sub(A.p, B.a))
            let rel = unsafe { (pick(4).c2Sub)(ray.p, cap.a) };
            let yap = unsafe { (pick(4).c2MulmvT)(m, rel) };
            // stage 5: capsule_bb via c2V, tested with c2AABBtoPoint
            let bb = C2AABB {
                min: unsafe { (pick(5).c2V)(-cap.r, 0.0) },
                max: unsafe { (pick(5).c2V)(cap.r, ybb.y) },
            };
            let inside = unsafe { (pick(5).c2AABBtoPoint)(bb, yap) };
            // stage 6: delegate to c2RaytoCircle on end cap A
            let mut out = C2Raycast::poison();
            let hit = unsafe {
                (pick(6).c2RaytoCircle)(ray, C2Circle { p: cap.a, r: cap.r }, &mut out)
            };
            results.push((mask, (inside * 2 + hit, yap.bits(), out.bits())));
        }
        let (m0, r0) = results[0];
        for &(mask, r) in &results[1..] {
            assert_eq!(
                r0, r,
                "cross-library pipeline divergence at iter {i}: mask {m0:#09b} vs {mask:#09b}\n  cap={cap:?}\n  ray p=({},{}) d=({},{}) t={}",
                f(ray.p.x), f(ray.p.y), f(ray.d.x), f(ray.d.y), f(ray.t)
            );
        }
    }
}

// ===========================================================================
// Row 50 — uniform ABI sweep across all 22 symbols
// ===========================================================================

/// Calls every exported symbol with the same pathological argument tuple and
/// compares bitwise. Catches a wrapper whose ABI differs (struct-by-value
/// classification, register class, return-in-memory) even when the arithmetic
/// would agree.
#[test]
fn cfg_50_all_symbols_abi_sweep() {
    let a = apis();
    let mut rng = Rng::new(50);
    for i in 0..2000 {
        let p = rng.v_path();
        let q = rng.v_path();
        let s = rng.pathological();
        let m = C2m { x: p, y: q };
        let ray = C2Ray { p, d: q, t: s };
        let circle = C2Circle { p, r: s };
        let aabb = C2AABB { min: p, max: q };
        let cap = C2Capsule { a: p, b: q, r: s };
        let ctx = format!("abi {i}");

        // 1 c2V
        let cv = unsafe { (a.c.c2V)(p.x, q.y) };
        let rv = unsafe { (a.rust.c2V)(p.x, q.y) };
        assert_eq!(cv.bits(), rv.bits(), "c2V [{ctx}]");
        // 2 c2Dot, 3 c2Len
        assert_eq!(
            unsafe { (a.c.c2Dot)(p, q) }.to_bits(),
            unsafe { (a.rust.c2Dot)(p, q) }.to_bits(),
            "c2Dot [{ctx}]"
        );
        assert_eq!(
            unsafe { (a.c.c2Len)(p) }.to_bits(),
            unsafe { (a.rust.c2Len)(p) }.to_bits(),
            "c2Len [{ctx}]"
        );
        // 4-6, 9-10 binary c2v
        cmp_v2("c2Add", a.c.c2Add, a.rust.c2Add, p, q, &ctx);
        cmp_v2("c2Sub", a.c.c2Sub, a.rust.c2Sub, p, q, &ctx);
        cmp_v2("c2Minv", a.c.c2Minv, a.rust.c2Minv, p, q, &ctx);
        cmp_v2("c2Maxv", a.c.c2Maxv, a.rust.c2Maxv, p, q, &ctx);
        // 7-8 c2v x float
        cmp_vs("c2Mulvs", a.c.c2Mulvs, a.rust.c2Mulvs, p, s, &ctx);
        cmp_vs("c2Div", a.c.c2Div, a.rust.c2Div, p, s, &ctx);
        // 11-13 unary c2v
        cmp_v1("c2Norm", a.c.c2Norm, a.rust.c2Norm, p, &ctx);
        cmp_v1("c2Skew", a.c.c2Skew, a.rust.c2Skew, p, &ctx);
        cmp_v1("c2Absv", a.c.c2Absv, a.rust.c2Absv, p, &ctx);
        cmp_v1("c2CCW90", a.c.c2CCW90, a.rust.c2CCW90, p, &ctx);
        // 14 c2MulmvT
        assert_eq!(
            unsafe { (a.c.c2MulmvT)(m, p) }.bits(),
            unsafe { (a.rust.c2MulmvT)(m, p) }.bits(),
            "c2MulmvT [{ctx}]"
        );
        // 15-17 predicates
        assert_eq!(
            unsafe { (a.c.c2AABBtoAABB)(aabb, C2AABB { min: q, max: p }) },
            unsafe { (a.rust.c2AABBtoAABB)(aabb, C2AABB { min: q, max: p }) },
            "c2AABBtoAABB [{ctx}]"
        );
        assert_eq!(
            unsafe { (a.c.c2AABBtoPoint)(aabb, p) },
            unsafe { (a.rust.c2AABBtoPoint)(aabb, p) },
            "c2AABBtoPoint [{ctx}]"
        );
        assert_eq!(
            unsafe { (a.c.c2CircleToPoint)(circle, q) },
            unsafe { (a.rust.c2CircleToPoint)(circle, q) },
            "c2CircleToPoint [{ctx}]"
        );
        // 18-20 raycasts
        ray_circle(ray, circle, &ctx);
        ray_aabb(ray, aabb, &ctx);
        ray_capsule(ray, cap, &ctx);
        // 21 dispatcher, all three tags
        cmp_castray(ray, &circle, C2_TYPE_CIRCLE, &ctx);
        cmp_castray(ray, &aabb, C2_TYPE_AABB, &ctx);
        cmp_castray(ray, &cap, C2_TYPE_CAPSULE, &ctx);
        // 22 spec_ray
        cmp_specray([p.x, p.y, q.x, q.y, s, rng.pathological(), rng.pathological()], &ctx);
    }
}
