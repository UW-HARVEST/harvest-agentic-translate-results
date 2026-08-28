//! Phase B — valid-path differential tests, one test per row of `CONFIGS.md`.
//!
//! Every test drives BOTH shared libraries through `libloading` with identical
//! inputs and asserts bit-identical return values and bit-identical `c2Raycast`
//! out-structs (pre-filled with a poison pattern, so "field not written" is
//! observable).  Inputs are property-style randomized from a fixed seed.

mod common;
use common::paths::*;
use common::*;

/* ------------------------------ diff drivers ------------------------------ */

fn diff_circle(ck: &mut Checker, ray: C2Ray, c: C2Circle) {
    let p = apis();
    let mut oc = POISON;
    let mut or_ = POISON;
    let (rc, rr) = unsafe {
        (
            (p.c.c2RaytoCircle)(ray, c, &mut oc),
            (p.r.c2RaytoCircle)(ray, c, &mut or_),
        )
    };
    let ctx = || format!("c2RaytoCircle({}, {})", fray(ray), fcircle(c));
    ck.int("ret", rc, rr, &ctx);
    ck.cast("out", oc, or_, &ctx);
}

fn diff_aabb(ck: &mut Checker, ray: C2Ray, b: C2AABB) {
    let p = apis();
    let mut oc = POISON;
    let mut or_ = POISON;
    let (rc, rr) = unsafe {
        (
            (p.c.c2RaytoAABB)(ray, b, &mut oc),
            (p.r.c2RaytoAABB)(ray, b, &mut or_),
        )
    };
    let ctx = || format!("c2RaytoAABB({}, {})", fray(ray), faabb(b));
    ck.int("ret", rc, rr, &ctx);
    ck.cast("out", oc, or_, &ctx);
}

fn diff_capsule(ck: &mut Checker, ray: C2Ray, b: C2Capsule) {
    let p = apis();
    let mut oc = POISON;
    let mut or_ = POISON;
    let (rc, rr) = unsafe {
        (
            (p.c.c2RaytoCapsule)(ray, b, &mut oc),
            (p.r.c2RaytoCapsule)(ray, b, &mut or_),
        )
    };
    let ctx = || format!("c2RaytoCapsule({}, {})", fray(ray), fcap(b));
    ck.int("ret", rc, rr, &ctx);
    ck.cast("out", oc, or_, &ctx);
}

fn diff_spec(ck: &mut Checker, a: [f32; 7]) {
    let p = apis();
    let mut oc = POISON;
    let mut or_ = POISON;
    let (rc, rr) = unsafe {
        (
            (p.c.spec_ray)(&mut oc, a[0], a[1], a[2], a[3], a[4], a[5], a[6]),
            (p.r.spec_ray)(&mut or_, a[0], a[1], a[2], a[3], a[4], a[5], a[6]),
        )
    };
    let ctx = || {
        format!(
            "spec_ray(mp=({:e},{:e}) c=({:e},{:e}) r={:e} rp=({:e},{:e})) bits={:08x?}",
            a[0],
            a[1],
            a[2],
            a[3],
            a[4],
            a[5],
            a[6],
            a.iter().map(|x| x.to_bits()).collect::<Vec<_>>()
        )
    };
    ck.int("ret", rc, rr, &ctx);
    ck.cast("out", oc, or_, &ctx);
}

/* ============================ rows 1-10: helpers ========================== */

#[test]
fn cfg_01_c2v() {
    let p = apis();
    let mut ck = Checker::new("cfg_01_c2v");
    let mut rng = Rng::new(0x0101);
    for s in SPECIALS {
        for s2 in SPECIALS {
            let ctx = || format!("c2V({:08x}, {:08x})", s.to_bits(), s2.to_bits());
            unsafe { ck.vec("ret", (p.c.c2V)(s, s2), (p.r.c2V)(s, s2), &ctx) };
        }
    }
    for _ in 0..n_iters() {
        let (x, y) = (rng.wild(), rng.wild());
        let ctx = || format!("c2V({:08x}, {:08x})", x.to_bits(), y.to_bits());
        unsafe { ck.vec("ret", (p.c.c2V)(x, y), (p.r.c2V)(x, y), &ctx) };
    }
    ck.finish();
}

#[test]
fn cfg_02_c2dot() {
    let p = apis();
    let mut ck = Checker::new("cfg_02_c2dot");
    let mut rng = Rng::new(0x0202);
    let mut cases: Vec<(C2v, C2v)> = Vec::new();
    for s in SPECIALS {
        for s2 in SPECIALS {
            cases.push((v(s, s2), v(s2, s)));
            cases.push((v(s, s2), v(s, s2)));
        }
    }
    for _ in 0..n_iters() {
        let a = rng.wild_v();
        cases.push((a, rng.wild_v()));
        cases.push((a, v(-a.x, -a.y)));
        cases.push((a, a));
    }
    for (a, b) in cases {
        let ctx = || format!("c2Dot({}, {})", fv(a), fv(b));
        unsafe { ck.f32("ret", (p.c.c2Dot)(a, b), (p.r.c2Dot)(a, b), &ctx) };
    }
    ck.finish();
}

#[test]
fn cfg_03_c2len() {
    let p = apis();
    let mut ck = Checker::new("cfg_03_c2len");
    let mut rng = Rng::new(0x0303);
    let mut cases: Vec<C2v> = vec![v(0.0, 0.0), v(-0.0, -0.0), v(3.0, 4.0), v(f32::MAX, f32::MAX)];
    for s in SPECIALS {
        for s2 in SPECIALS {
            cases.push(v(s, s2));
        }
    }
    for _ in 0..n_iters() {
        cases.push(rng.wild_v());
        cases.push(rng.coord_v());
    }
    for a in cases {
        let ctx = || format!("c2Len({})", fv(a));
        unsafe { ck.f32("ret", (p.c.c2Len)(a), (p.r.c2Len)(a), &ctx) };
    }
    ck.finish();
}

#[test]
fn cfg_04_add_sub() {
    let p = apis();
    let mut ck = Checker::new("cfg_04_add_sub");
    let mut rng = Rng::new(0x0404);
    let mut cases: Vec<(C2v, C2v)> = Vec::new();
    for s in SPECIALS {
        for s2 in SPECIALS {
            cases.push((v(s, s2), v(s2, s)));
            cases.push((v(s, s), v(s, s)));
        }
    }
    for _ in 0..n_iters() {
        cases.push((rng.wild_v(), rng.wild_v()));
        let a = rng.coord_v();
        cases.push((a, a));
    }
    for (a, b) in cases {
        let ctx = || format!("({}, {})", fv(a), fv(b));
        unsafe {
            ck.vec("c2Add", (p.c.c2Add)(a, b), (p.r.c2Add)(a, b), &ctx);
            ck.vec("c2Sub", (p.c.c2Sub)(a, b), (p.r.c2Sub)(a, b), &ctx);
        }
    }
    ck.finish();
}

#[test]
fn cfg_05_mulvs() {
    let p = apis();
    let mut ck = Checker::new("cfg_05_mulvs");
    let mut rng = Rng::new(0x0505);
    let mut cases: Vec<(C2v, f32)> = Vec::new();
    for s in SPECIALS {
        for s2 in SPECIALS {
            cases.push((v(s, s2), s2));
            cases.push((v(s, s2), s));
        }
    }
    for _ in 0..n_iters() {
        cases.push((rng.wild_v(), rng.wild()));
        cases.push((rng.coord_v(), rng.range(-1.0e20, 1.0e20)));
    }
    for (a, b) in cases {
        let ctx = || format!("c2Mulvs({}, {:e}/0x{:08x})", fv(a), b, b.to_bits());
        unsafe { ck.vec("ret", (p.c.c2Mulvs)(a, b), (p.r.c2Mulvs)(a, b), &ctx) };
    }
    ck.finish();
}

#[test]
fn cfg_06_div() {
    let p = apis();
    let mut ck = Checker::new("cfg_06_div");
    let mut rng = Rng::new(0x0606);
    let mut cases: Vec<(C2v, f32)> = Vec::new();
    for s in SPECIALS {
        for s2 in SPECIALS {
            cases.push((v(s, s2), s2));
            cases.push((v(s, s2), s));
        }
    }
    for _ in 0..n_iters() {
        cases.push((rng.wild_v(), rng.wild()));
        cases.push((rng.coord_v(), rng.coord()));
        // denormal divisor -> 1/b overflows to +inf
        cases.push((rng.coord_v(), f32::from_bits(rng.next_u32() & 0x007F_FFFF)));
    }
    for (a, b) in cases {
        let ctx = || format!("c2Div({}, {:e}/0x{:08x})", fv(a), b, b.to_bits());
        unsafe { ck.vec("ret", (p.c.c2Div)(a, b), (p.r.c2Div)(a, b), &ctx) };
    }
    ck.finish();
}

#[test]
fn cfg_07_norm() {
    let p = apis();
    let mut ck = Checker::new("cfg_07_norm");
    let mut rng = Rng::new(0x0707);
    let mut cases: Vec<C2v> = vec![
        v(0.0, 0.0),
        v(-0.0, 0.0),
        v(1.0, 0.0),
        v(3.0, 4.0),
        v(f32::MAX, f32::MAX),
        v(1.0e-45, 1.0e-45),
    ];
    for s in SPECIALS {
        for s2 in SPECIALS {
            cases.push(v(s, s2));
        }
    }
    for _ in 0..n_iters() {
        cases.push(rng.wild_v());
        cases.push(rng.coord_v());
    }
    for a in cases {
        let ctx = || format!("c2Norm({})", fv(a));
        unsafe { ck.vec("ret", (p.c.c2Norm)(a), (p.r.c2Norm)(a), &ctx) };
    }
    ck.finish();
}

#[test]
fn cfg_08_minv_maxv() {
    let p = apis();
    let mut ck = Checker::new("cfg_08_minv_maxv");
    let mut rng = Rng::new(0x0808);
    let mut cases: Vec<(C2v, C2v)> = Vec::new();
    for s in SPECIALS {
        for s2 in SPECIALS {
            // both operand orders — the C ternaries are NOT symmetric in NaN
            cases.push((v(s, s2), v(s2, s)));
            cases.push((v(s2, s), v(s, s2)));
            cases.push((v(s, s), v(s, s)));
        }
    }
    for _ in 0..n_iters() {
        cases.push((rng.wild_v(), rng.wild_v()));
        let a = rng.coord_v();
        cases.push((a, a));
    }
    for (a, b) in cases {
        let ctx = || format!("({}, {})", fv(a), fv(b));
        unsafe {
            ck.vec("c2Minv", (p.c.c2Minv)(a, b), (p.r.c2Minv)(a, b), &ctx);
            ck.vec("c2Maxv", (p.c.c2Maxv)(a, b), (p.r.c2Maxv)(a, b), &ctx);
        }
    }
    ck.finish();
}

#[test]
fn cfg_09_skew_ccw90_absv() {
    let p = apis();
    let mut ck = Checker::new("cfg_09_skew_ccw90_absv");
    let mut rng = Rng::new(0x0909);
    let mut cases: Vec<C2v> = Vec::new();
    for s in SPECIALS {
        for s2 in SPECIALS {
            cases.push(v(s, s2));
        }
    }
    for _ in 0..n_iters() {
        cases.push(rng.wild_v());
    }
    for a in cases {
        let ctx = || format!("({})", fv(a));
        unsafe {
            ck.vec("c2Skew", (p.c.c2Skew)(a), (p.r.c2Skew)(a), &ctx);
            ck.vec("c2CCW90", (p.c.c2CCW90)(a), (p.r.c2CCW90)(a), &ctx);
            ck.vec("c2Absv", (p.c.c2Absv)(a), (p.r.c2Absv)(a), &ctx);
        }
    }
    ck.finish();
}

#[test]
fn cfg_10_mulmvt() {
    let p = apis();
    let mut ck = Checker::new("cfg_10_mulmvt");
    let mut rng = Rng::new(0x0A0A);
    let mut cases: Vec<(C2m, C2v)> = vec![
        (C2m { x: v(1.0, 0.0), y: v(0.0, 1.0) }, v(3.0, -4.0)),
        (C2m { x: v(0.0, 0.0), y: v(0.0, 0.0) }, v(f32::INFINITY, 0.0)),
    ];
    for s in SPECIALS {
        for s2 in SPECIALS {
            cases.push((C2m { x: v(s, s2), y: v(s2, s) }, v(s, s2)));
            cases.push((C2m { x: v(s, s), y: v(s2, s2) }, v(s2, s)));
        }
    }
    for _ in 0..n_iters() {
        cases.push((
            C2m {
                x: rng.wild_v(),
                y: rng.wild_v(),
            },
            rng.wild_v(),
        ));
        cases.push((
            C2m {
                x: rng.coord_v(),
                y: rng.coord_v(),
            },
            rng.coord_v(),
        ));
    }
    for (m, b) in cases {
        let ctx = || format!("c2MulmvT({}, {})", fm(m), fv(b));
        unsafe { ck.vec("ret", (p.c.c2MulmvT)(m, b), (p.r.c2MulmvT)(m, b), &ctx) };
    }
    ck.finish();
}

/* ========================= rows 11-15: predicates ======================== */

#[test]
fn cfg_11_aabbtoaabb_proper() {
    let p = apis();
    let mut ck = Checker::new("cfg_11_aabbtoaabb_proper");
    let mut rng = Rng::new(0x0B0B);
    let mut overlap = 0usize;
    let mut disjoint = 0usize;
    for _ in 0..n_iters() {
        let a = rng.proper_aabb();
        let b = rng.proper_aabb();
        let ctx = || format!("({}, {})", faabb(a), faabb(b));
        let (rc, rr) = unsafe { ((p.c.c2AABBtoAABB)(a, b), (p.r.c2AABBtoAABB)(a, b)) };
        ck.int("ret", rc, rr, &ctx);
        if rc != 0 {
            overlap += 1
        } else {
            disjoint += 1
        }
        // also the reversed argument order (d0/d1 vs d2/d3 asymmetry)
        let ctx2 = || format!("({}, {}) reversed", faabb(b), faabb(a));
        unsafe {
            ck.int("ret_rev", (p.c.c2AABBtoAABB)(b, a), (p.r.c2AABBtoAABB)(b, a), &ctx2);
        }
        // touching boxes: share exactly one edge coordinate
        let t = C2AABB {
            min: v(a.max.x, a.min.y),
            max: v(a.max.x + 1.0, a.max.y),
        };
        let ctx3 = || format!("touching ({}, {})", faabb(a), faabb(t));
        unsafe {
            ck.int("ret_touch", (p.c.c2AABBtoAABB)(a, t), (p.r.c2AABBtoAABB)(a, t), &ctx3);
        }
    }
    eprintln!("[cfg_11] overlapping={overlap} disjoint={disjoint}");
    assert!(overlap > 100 && disjoint > 100, "poor overlap/disjoint balance");
    ck.finish();
}

#[test]
fn cfg_12_aabbtoaabb_degenerate() {
    let p = apis();
    let mut ck = Checker::new("cfg_12_aabbtoaabb_degenerate");
    let mut rng = Rng::new(0x0C0C);
    let mut cases: Vec<(C2AABB, C2AABB)> = Vec::new();
    // degenerate / inverted / line boxes built from the same coordinates
    for _ in 0..n_iters() {
        let q = rng.coord_v();
        let w = rng.coord_v();
        let variants = [
            C2AABB { min: q, max: q },                       // point
            C2AABB { min: q, max: v(q.x, w.y) },             // vertical line
            C2AABB { min: q, max: v(w.x, q.y) },             // horizontal line
            C2AABB { min: w, max: q },                       // possibly inverted
            C2AABB { min: v(q.x, w.y), max: v(w.x, q.y) },   // mixed inversion
        ];
        let i = rng.below(variants.len() as u32) as usize;
        let j = rng.below(variants.len() as u32) as usize;
        cases.push((variants[i], variants[j]));
        cases.push((rng.wild_aabb(), rng.wild_aabb()));
    }
    for s in SPECIALS {
        for s2 in SPECIALS {
            cases.push((
                C2AABB { min: v(s, s2), max: v(s2, s) },
                C2AABB { min: v(s2, s), max: v(s, s2) },
            ));
        }
    }
    for (a, b) in cases {
        let ctx = || format!("({}, {})", faabb(a), faabb(b));
        unsafe { ck.int("ret", (p.c.c2AABBtoAABB)(a, b), (p.r.c2AABBtoAABB)(a, b), &ctx) };
    }
    ck.finish();
}

#[test]
fn cfg_13_aabbtopoint() {
    let p = apis();
    let mut ck = Checker::new("cfg_13_aabbtopoint");
    let mut rng = Rng::new(0x0D0D);
    let mut inside = 0usize;
    let mut outside = 0usize;
    for _ in 0..n_iters() {
        let b = rng.proper_aabb();
        let pts = [
            rng.coord_v(),                                  // anywhere
            v(rng.range(b.min.x, b.max.x), rng.range(b.min.y, b.max.y)), // inside
            b.min,                                          // corner
            b.max,                                          // corner
            v(b.min.x, b.max.y),
            v(b.max.x, b.min.y),
            v(b.min.x, rng.range(b.min.y, b.max.y)),        // on the -x edge
            v(rng.range(b.min.x, b.max.x), b.max.y),        // on the +y edge
        ];
        for q in pts {
            let ctx = || format!("({}, {})", faabb(b), fv(q));
            let (rc, rr) = unsafe { ((p.c.c2AABBtoPoint)(b, q), (p.r.c2AABBtoPoint)(b, q)) };
            ck.int("ret", rc, rr, &ctx);
            if rc != 0 {
                inside += 1
            } else {
                outside += 1
            }
        }
    }
    eprintln!("[cfg_13] inside={inside} outside={outside}");
    assert!(inside > 100 && outside > 100);
    ck.finish();
}

#[test]
fn cfg_14_aabbtopoint_degenerate() {
    let p = apis();
    let mut ck = Checker::new("cfg_14_aabbtopoint_degenerate");
    let mut rng = Rng::new(0x0E0E);
    for _ in 0..n_iters() {
        let q = rng.coord_v();
        let w = rng.coord_v();
        let boxes = [
            C2AABB { min: q, max: q },
            C2AABB { min: w, max: q },
            C2AABB { min: v(q.x, w.y), max: v(w.x, q.y) },
            rng.wild_aabb(),
        ];
        let pts = [q, w, rng.wild_v(), v(0.0, 0.0), v(-0.0, -0.0)];
        let b = boxes[rng.below(boxes.len() as u32) as usize];
        for pt in pts {
            let ctx = || format!("({}, {})", faabb(b), fv(pt));
            unsafe { ck.int("ret", (p.c.c2AABBtoPoint)(b, pt), (p.r.c2AABBtoPoint)(b, pt), &ctx) };
        }
    }
    for s in SPECIALS {
        for s2 in SPECIALS {
            let b = C2AABB { min: v(s, s2), max: v(s2, s) };
            let q = v(s2, s2);
            let ctx = || format!("({}, {})", faabb(b), fv(q));
            unsafe { ck.int("ret", (p.c.c2AABBtoPoint)(b, q), (p.r.c2AABBtoPoint)(b, q), &ctx) };
        }
    }
    ck.finish();
}

#[test]
fn cfg_15_circletopoint() {
    let p = apis();
    let mut ck = Checker::new("cfg_15_circletopoint");
    let mut rng = Rng::new(0x0F0F);
    let mut inside = 0usize;
    let mut outside = 0usize;
    for _ in 0..n_iters() {
        let center = rng.coord_v();
        let r = match rng.below(6) {
            0 => 0.0,
            1 => -rng.radius(), // negative radius: r*r is still positive
            2 => 1.0e20,
            _ => rng.radius(),
        };
        let c = C2Circle { p: center, r };
        let ang = rng.range(-3.15, 3.15);
        let pts = [
            center,                                                    // exact center
            v(center.x + r * ang.cos(), center.y + r * ang.sin()),      // on the rim
            v(center.x + r * 0.5 * ang.cos(), center.y + r * 0.5 * ang.sin()), // inside
            v(center.x + r * 1.5 * ang.cos(), center.y + r * 1.5 * ang.sin()), // outside
            rng.coord_v(),
            rng.wild_v(),
        ];
        for q in pts {
            let ctx = || format!("({}, {})", fcircle(c), fv(q));
            let (rc, rr) = unsafe { ((p.c.c2CircleToPoint)(c, q), (p.r.c2CircleToPoint)(c, q)) };
            ck.int("ret", rc, rr, &ctx);
            if rc != 0 {
                inside += 1
            } else {
                outside += 1
            }
        }
    }
    eprintln!("[cfg_15] inside={inside} outside={outside}");
    assert!(inside > 100 && outside > 100);
    ck.finish();
}

/* ====================== rows 16-19: c2RaytoCircle ======================== */

#[test]
fn cfg_16_raytocircle_hit() {
    let mut ck = Checker::new("cfg_16_raytocircle_hit");
    let mut cov = Cover::new("cfg_16_raytocircle_hit", &CIRCLE_NAMES);
    let mut rng = Rng::new(0x1010);
    for _ in 0..n_iters() {
        let center = rng.coord_v();
        let r = rng.range(0.5, 20.0);
        let ang = rng.range(-3.15, 3.15);
        let dist = r + rng.range(0.01, 50.0);
        let p0 = v(center.x + dist * ang.cos(), center.y + dist * ang.sin());
        // aim back at the centre, with a jitter small enough to still hit
        let max_jit = (r / dist).asin();
        let dir_ang = ang + std::f32::consts::PI + rng.range(-max_jit, max_jit);
        let ray = C2Ray {
            p: p0,
            d: v(dir_ang.cos(), dir_ang.sin()),
            t: dist + rng.range(0.0, 10.0),
        };
        let c = C2Circle { p: center, r };
        cov.hit(circle_path(ray, c));
        diff_circle(&mut ck, ray, c);
    }
    cov.require(&[CIRCLE_HIT], 100);
    ck.finish();
}

#[test]
fn cfg_17_raytocircle_t_shapes() {
    let mut ck = Checker::new("cfg_17_raytocircle_t_shapes");
    let mut cov = Cover::new("cfg_17_raytocircle_t_shapes", &CIRCLE_NAMES);
    let mut rng = Rng::new(0x1111);
    for _ in 0..n_iters() {
        let c = C2Circle {
            p: rng.coord_v(),
            r: rng.radius(),
        };
        let ray = rng.nice_ray(); // unnormalized / zero / axis-aligned d, odd t
        cov.hit(circle_path(ray, c));
        diff_circle(&mut ck, ray, c);
        // the same ray with the four special t values
        for t in [0.0f32, -1.0, 1.0e30, f32::MAX] {
            let r2 = C2Ray { t, ..ray };
            cov.hit(circle_path(r2, c));
            diff_circle(&mut ck, r2, c);
        }
    }
    cov.require(&[CIRCLE_DISC_NEG, CIRCLE_T_NEG, CIRCLE_T_BEYOND, CIRCLE_HIT], 20);
    ck.finish();
}

#[test]
fn cfg_18_raytocircle_inside_tangent() {
    let mut ck = Checker::new("cfg_18_raytocircle_inside_tangent");
    let mut cov = Cover::new("cfg_18_raytocircle_inside_tangent", &CIRCLE_NAMES);
    let mut rng = Rng::new(0x1212);
    for _ in 0..n_iters() {
        let center = rng.coord_v();
        let r = rng.range(0.25, 20.0);
        let c = C2Circle { p: center, r };
        let ang = rng.range(-3.15, 3.15);

        // (a) origin strictly inside the circle -> t < 0
        let f = rng.range(0.0, 0.999);
        let inside = C2Ray {
            p: v(center.x + r * f * ang.cos(), center.y + r * f * ang.sin()),
            d: v(ang.cos(), ang.sin()),
            t: rng.range(0.0, 100.0),
        };
        cov.hit(circle_path(inside, c));
        diff_circle(&mut ck, inside, c);

        // (b) tangent: the ray line passes exactly through a rim point
        let rim = v(center.x + r * ang.cos(), center.y + r * ang.sin());
        let tang = v(-ang.sin(), ang.cos()); // unit tangent at `rim`
        let back = rng.range(1.0, 50.0);
        let tangent = C2Ray {
            p: v(rim.x - tang.x * back, rim.y - tang.y * back),
            d: tang,
            t: back + rng.range(-1.0, 5.0),
        };
        cov.hit(circle_path(tangent, c));
        diff_circle(&mut ck, tangent, c);

        // (c) origin exactly on the rim
        let on_rim = C2Ray {
            p: rim,
            d: v(-ang.cos(), -ang.sin()),
            t: rng.range(0.0, 4.0) * r,
        };
        cov.hit(circle_path(on_rim, c));
        diff_circle(&mut ck, on_rim, c);
    }
    cov.require(&[CIRCLE_T_NEG, CIRCLE_HIT], 50);
    ck.finish();
}

#[test]
fn cfg_19_raytocircle_random_bits() {
    let mut ck = Checker::new("cfg_19_raytocircle_random_bits");
    let mut cov = Cover::new("cfg_19_raytocircle_random_bits", &CIRCLE_NAMES);
    let mut rng = Rng::new(0x1313);
    for _ in 0..n_iters() {
        let ray = rng.wild_ray();
        let c = rng.wild_circle();
        cov.hit(circle_path(ray, c));
        diff_circle(&mut ck, ray, c);
    }
    // mixed: sane geometry with one wild field
    for _ in 0..n_iters() {
        let mut ray = rng.nice_ray();
        let mut c = C2Circle { p: rng.coord_v(), r: rng.radius() };
        match rng.below(6) {
            0 => ray.p.x = rng.special(),
            1 => ray.d.y = rng.special(),
            2 => ray.t = rng.special(),
            3 => c.r = rng.special(),
            4 => c.p.y = rng.special(),
            _ => {}
        }
        cov.hit(circle_path(ray, c));
        diff_circle(&mut ck, ray, c);
    }
    cov.require(&[CIRCLE_DISC_NEG, CIRCLE_NAN], 20);
    ck.finish();
}

/* ======================= rows 20-24: c2RaytoAABB ========================= */

#[test]
fn cfg_20_raytoaabb_hit() {
    let mut ck = Checker::new("cfg_20_raytoaabb_hit");
    let mut cov = Cover::new("cfg_20_raytoaabb_hit", &AABB_NAMES);
    let mut rng = Rng::new(0x1414);
    for _ in 0..n_iters() {
        let b = rng.proper_aabb();
        let center = v((b.min.x + b.max.x) * 0.5, (b.min.y + b.max.y) * 0.5);
        // start outside, aim at a random point of the box
        let ang = rng.range(-3.15, 3.15);
        let dist = rng.range(1.0, 60.0);
        let p0 = v(center.x + dist * ang.cos(), center.y + dist * ang.sin());
        let target = v(
            rng.range(b.min.x, b.max.x),
            rng.range(b.min.y, b.max.y),
        );
        let (dx, dy) = (target.x - p0.x, target.y - p0.y);
        let l = (dx * dx + dy * dy).sqrt();
        let ray = C2Ray {
            p: p0,
            d: v(dx / l, dy / l),
            t: l * rng.range(0.5, 2.0),
        };
        cov.hit(aabb_path(ray, b));
        diff_aabb(&mut ck, ray, b);
    }
    cov.require(
        &[AABB_WIN_T0, AABB_WIN_T1, AABB_WIN_T2, AABB_WIN_T3],
        50,
    );
    ck.finish();
}

#[test]
fn cfg_21_raytoaabb_axis_aligned() {
    let mut ck = Checker::new("cfg_21_raytoaabb_axis_aligned");
    let mut cov = Cover::new("cfg_21_raytoaabb_axis_aligned", &AABB_NAMES);
    let mut rng = Rng::new(0x1515);
    for _ in 0..n_iters() {
        let b = rng.proper_aabb();
        let dirs = [v(1.0, 0.0), v(-1.0, 0.0), v(0.0, 1.0), v(0.0, -1.0)];
        let di = rng.below(4) as usize;
        let d = dirs[di];
        let (mid_x, mid_y) = ((b.min.x + b.max.x) * 0.5, (b.min.y + b.max.y) * 0.5);
        let away = rng.range(0.01, 30.0);
        // an origin outside the face the ray enters through -> makes each of
        // t0/t1/t2/t3 the winning axis in turn
        let entry = match di {
            0 => v(b.min.x - away, rng.range(b.min.y, b.max.y)), // enters -x face
            1 => v(b.max.x + away, rng.range(b.min.y, b.max.y)), // enters +x face
            2 => v(rng.range(b.min.x, b.max.x), b.min.y - away), // enters -y face
            _ => v(rng.range(b.min.x, b.max.x), b.max.y + away), // enters +y face
        };
        // origins that make da/db degenerate: exactly on a face plane, on the
        // face extension, and on the centre lines
        let origins = [
            entry,
            v(b.min.x, rng.range(b.min.y, b.max.y)),
            v(b.max.x, rng.range(b.min.y, b.max.y)),
            v(rng.range(b.min.x, b.max.x), b.min.y),
            v(rng.range(b.min.x, b.max.x), b.max.y),
            v(b.min.x - rng.range(0.0, 30.0), mid_y),
            v(mid_x, b.max.y + rng.range(0.0, 30.0)),
            v(b.min.x, b.min.y),
            v(b.max.x, b.max.y),
            // exactly on the face plane extension (da == 0 -> d == 0 sub-path)
            v(b.min.x, b.max.y + away),
            v(b.max.x + away, b.min.y),
        ];
        for p0 in origins {
            let ray = C2Ray {
                p: p0,
                d,
                t: match rng.below(5) {
                    0 => 0.0,
                    1 => rng.range(0.0, 1.0),
                    2 => away, // stops exactly on the face
                    _ => away + rng.range(0.0, 100.0),
                },
            };
            cov.hit(aabb_path(ray, b));
            diff_aabb(&mut ck, ray, b);
        }
    }
    cov.require(&[AABB_WIN_T0, AABB_WIN_T1, AABB_WIN_T2, AABB_WIN_T3], 20);
    ck.finish();
}

#[test]
fn cfg_22_raytoaabb_inside() {
    let mut ck = Checker::new("cfg_22_raytoaabb_inside");
    let mut cov = Cover::new("cfg_22_raytoaabb_inside", &AABB_NAMES);
    let mut rng = Rng::new(0x1616);
    for _ in 0..n_iters() {
        let b = rng.proper_aabb();
        let p0 = v(rng.range(b.min.x, b.max.x), rng.range(b.min.y, b.max.y));
        let ang = rng.range(-3.15, 3.15);
        for t in [
            0.0f32,
            rng.range(0.0, 0.01),
            rng.range(0.0, 5.0),
            rng.range(0.0, 1.0e6),
        ] {
            let ray = C2Ray {
                p: p0,
                d: v(ang.cos(), ang.sin()),
                t,
            };
            cov.hit(aabb_path(ray, b));
            diff_aabb(&mut ck, ray, b);
        }
        // ray entirely inside the box
        let p1 = v(rng.range(b.min.x, b.max.x), rng.range(b.min.y, b.max.y));
        let (dx, dy) = (p1.x - p0.x, p1.y - p0.y);
        let l = (dx * dx + dy * dy).sqrt();
        let ray = C2Ray {
            p: p0,
            d: if l > 0.0 { v(dx / l, dy / l) } else { v(0.0, 0.0) },
            t: l,
        };
        cov.hit(aabb_path(ray, b));
        diff_aabb(&mut ck, ray, b);
    }
    cov.report();
    ck.finish();
}

#[test]
fn cfg_23_raytoaabb_degenerate_box() {
    let mut ck = Checker::new("cfg_23_raytoaabb_degenerate_box");
    let mut cov = Cover::new("cfg_23_raytoaabb_degenerate_box", &AABB_NAMES);
    let mut rng = Rng::new(0x1717);
    for _ in 0..n_iters() {
        let q = rng.coord_v();
        let w = rng.coord_v();
        let boxes = [
            C2AABB { min: q, max: q },                     // point box
            C2AABB { min: q, max: v(q.x, w.y) },           // vertical line
            C2AABB { min: q, max: v(w.x, q.y) },           // horizontal line
            C2AABB { min: w, max: q },                     // inverted
            C2AABB { min: v(-1.0e30, -1.0e30), max: v(1.0e30, 1.0e30) }, // huge
            C2AABB { min: v(1.0e-40, 1.0e-40), max: v(2.0e-40, 2.0e-40) }, // denormal
        ];
        let b = boxes[rng.below(boxes.len() as u32) as usize];
        let ray = if rng.below(2) == 0 {
            rng.nice_ray()
        } else {
            C2Ray { p: q, d: v(1.0, 1.0), t: rng.range(0.0, 100.0) }
        };
        cov.hit(aabb_path(ray, b));
        diff_aabb(&mut ck, ray, b);
    }
    cov.report();
    ck.finish();
}

#[test]
fn cfg_24_raytoaabb_random_bits() {
    let mut ck = Checker::new("cfg_24_raytoaabb_random_bits");
    let mut cov = Cover::new("cfg_24_raytoaabb_random_bits", &AABB_NAMES);
    let mut rng = Rng::new(0x1818);
    for _ in 0..n_iters() {
        let ray = rng.wild_ray();
        let b = rng.wild_aabb();
        cov.hit(aabb_path(ray, b));
        diff_aabb(&mut ck, ray, b);
    }
    for _ in 0..n_iters() {
        let mut ray = rng.nice_ray();
        let mut b = rng.proper_aabb();
        match rng.below(7) {
            0 => ray.p.x = rng.special(),
            1 => ray.d.x = rng.special(),
            2 => ray.d.y = rng.special(),
            3 => ray.t = rng.special(),
            4 => b.min.x = rng.special(),
            5 => b.max.y = rng.special(),
            _ => {}
        }
        cov.hit(aabb_path(ray, b));
        diff_aabb(&mut ck, ray, b);
    }
    cov.require(&[AABB_BROAD_REJECT], 20);
    ck.finish();
}

/* ====================== rows 25-31: c2RaytoCapsule ======================= */

/// Builds a capsule plus a ray specified in the capsule's own frame:
/// `M.y = norm(b-a)`, `M.x = CCW90(M.y)`, so a local point `(lx, ly)` maps to
/// the world point `a + lx*M.x + ly*M.y` and `c2MulmvT(M, p-a) == (lx, ly)`.
struct CapFrame {
    cap: C2Capsule,
    mx: C2v,
    my: C2v,
    #[allow(dead_code)]
    len: f32,
}

impl CapFrame {
    fn new(rng: &mut Rng, r: f32, len: f32) -> CapFrame {
        let phi = rng.range(-3.15, 3.15);
        let my = v(phi.cos(), phi.sin());
        let mx = v(my.y, -my.x); // c2CCW90
        let a = rng.coord_v();
        let b = v(a.x + my.x * len, a.y + my.y * len);
        CapFrame {
            cap: C2Capsule { a, b, r },
            mx,
            my,
            len,
        }
    }
    fn world(&self, lx: f32, ly: f32) -> C2v {
        v(
            self.cap.a.x + lx * self.mx.x + ly * self.my.x,
            self.cap.a.y + lx * self.mx.y + ly * self.my.y,
        )
    }
    /// Ray from local `(lx,ly)` to local `(ex,ey)` with `A.t == tt`.
    fn ray(&self, lx: f32, ly: f32, ex: f32, ey: f32, tt: f32) -> C2Ray {
        let (dlx, dly) = ((ex - lx) / tt, (ey - ly) / tt);
        C2Ray {
            p: self.world(lx, ly),
            d: v(
                dlx * self.mx.x + dly * self.my.x,
                dlx * self.mx.y + dly * self.my.y,
            ),
            t: tt,
        }
    }
}

#[test]
fn cfg_25_raytocapsule_side_hit_pos() {
    let mut ck = Checker::new("cfg_25_raytocapsule_side_hit_pos");
    let mut cov = Cover::new("cfg_25_raytocapsule_side_hit_pos", &CAP_NAMES);
    let mut rng = Rng::new(0x1919);
    for _ in 0..n_iters() {
        let r = rng.range(0.1, 5.0);
        let len = rng.range(1.0, 40.0);
        let f = CapFrame::new(&mut rng, r, len);
        // start on the +x side (c == +r), cross to the -x side
        let lx = r + rng.range(0.01, 20.0);
        let ly = rng.range(0.05, len - 0.05);
        let ex = -(r + rng.range(0.01, 20.0));
        let ey = ly + rng.range(-0.2, 0.2) * len * 0.1;
        let ray = f.ray(lx, ly, ex, ey, rng.range(0.5, 5.0));
        cov.hit(capsule_path(ray, f.cap));
        diff_capsule(&mut ck, ray, f.cap);
    }
    cov.require(&[CAP_SIDE_POS], 100);
    ck.finish();
}

#[test]
fn cfg_26_raytocapsule_side_hit_neg() {
    let mut ck = Checker::new("cfg_26_raytocapsule_side_hit_neg");
    let mut cov = Cover::new("cfg_26_raytocapsule_side_hit_neg", &CAP_NAMES);
    let mut rng = Rng::new(0x1A1A);
    for _ in 0..n_iters() {
        let r = rng.range(0.1, 5.0);
        let len = rng.range(1.0, 40.0);
        let f = CapFrame::new(&mut rng, r, len);
        // start on the -x side (c == -r), cross to the +x side
        let lx = -(r + rng.range(0.01, 20.0));
        let ly = rng.range(0.05, len - 0.05);
        let ex = r + rng.range(0.01, 20.0);
        let ey = ly + rng.range(-0.2, 0.2) * len * 0.1;
        let ray = f.ray(lx, ly, ex, ey, rng.range(0.5, 5.0));
        cov.hit(capsule_path(ray, f.cap));
        diff_capsule(&mut ck, ray, f.cap);
    }
    cov.require(&[CAP_SIDE_NEG], 100);
    ck.finish();
}

#[test]
fn cfg_27_raytocapsule_origin_inside() {
    let mut ck = Checker::new("cfg_27_raytocapsule_origin_inside");
    let mut cov = Cover::new("cfg_27_raytocapsule_origin_inside", &CAP_NAMES);
    let mut rng = Rng::new(0x1B1B);
    for _ in 0..n_iters() {
        let r = rng.range(0.1, 5.0);
        let len = rng.range(1.0, 40.0);
        let f = CapFrame::new(&mut rng, r, len);
        let tt = rng.range(0.5, 5.0);
        // (a) inside the rotated bounding box: |lx| <= r, 0 <= ly <= len
        let a_ray = f.ray(
            rng.range(-r, r),
            rng.range(0.0, len),
            rng.range(-20.0, 20.0),
            rng.range(-20.0, 20.0),
            tt,
        );
        cov.hit(capsule_path(a_ray, f.cap));
        diff_capsule(&mut ck, a_ray, f.cap);
        // (b) inside end-cap a: ly < 0 and lx^2+ly^2 < r^2
        let ang = rng.range(0.0, 3.14);
        let rad = rng.range(0.0, 0.999) * r;
        let b_ray = f.ray(
            rad * ang.cos(),
            -rad * ang.sin(),
            rng.range(-20.0, 20.0),
            rng.range(-20.0, 20.0),
            tt,
        );
        cov.hit(capsule_path(b_ray, f.cap));
        diff_capsule(&mut ck, b_ray, f.cap);
        // (c) inside end-cap b: ly > len and lx^2+(ly-len)^2 < r^2
        let c_ray = f.ray(
            rad * ang.cos(),
            len + rad * ang.sin(),
            rng.range(-20.0, 20.0),
            rng.range(-20.0, 20.0),
            tt,
        );
        cov.hit(capsule_path(c_ray, f.cap));
        diff_capsule(&mut ck, c_ray, f.cap);
    }
    cov.require(&[CAP_IN_BB, CAP_IN_CAP_A, CAP_IN_CAP_B], 100);
    ck.finish();
}

#[test]
fn cfg_28_raytocapsule_delegate_caps() {
    let mut ck = Checker::new("cfg_28_raytocapsule_delegate_caps");
    let mut cov = Cover::new("cfg_28_raytocapsule_delegate_caps", &CAP_NAMES);
    let mut rng = Rng::new(0x1C1C);
    for _ in 0..n_iters() {
        let r = rng.range(0.1, 5.0);
        let len = rng.range(1.0, 40.0);
        let f = CapFrame::new(&mut rng, r, len);
        let tt = rng.range(0.5, 5.0);
        // |yAp.x| < r but the origin is beyond an end cap:
        //   ly < -sqrt(r^2 - lx^2)  ->  yAp.y < 0  -> circle a
        let lx = rng.range(-0.9, 0.9) * r;
        let cap_h = (r * r - lx * lx).sqrt();
        let below = -(cap_h + rng.range(0.01, 30.0));
        let ray_a = f.ray(lx, below, lx, below + rng.range(1.0, 60.0), tt);
        cov.hit(capsule_path(ray_a, f.cap));
        diff_capsule(&mut ck, ray_a, f.cap);
        //   ly > len + sqrt(r^2 - lx^2)  ->  yAp.y >= 0 -> circle b
        let above = len + cap_h + rng.range(0.01, 30.0);
        let ray_b = f.ray(lx, above, lx, above - rng.range(1.0, 60.0), tt);
        cov.hit(capsule_path(ray_b, f.cap));
        diff_capsule(&mut ck, ray_b, f.cap);
    }
    cov.require(&[CAP_DELEG_A_BY_X, CAP_DELEG_B_BY_X], 100);
    ck.finish();
}

#[test]
fn cfg_29_raytocapsule_delegate_by_y() {
    let mut ck = Checker::new("cfg_29_raytocapsule_delegate_by_y");
    let mut cov = Cover::new("cfg_29_raytocapsule_delegate_by_y", &CAP_NAMES);
    let mut rng = Rng::new(0x1D1D);
    for _ in 0..n_iters() {
        let r = rng.range(0.1, 3.0);
        let len = rng.range(1.0, 40.0);
        let f = CapFrame::new(&mut rng, r, len);
        let tt = rng.range(0.5, 5.0);
        // |lx| > r, sign flip, but the side-plane crossing happens below the
        // segment (y <= 0) -> delegate to circle a
        let lx = r + rng.range(0.5, 20.0);
        let ly = -(r + rng.range(0.5, 20.0));
        let ray_a = f.ray(lx, ly, -lx, ly + rng.range(-1.0, 1.0), tt);
        cov.hit(capsule_path(ray_a, f.cap));
        diff_capsule(&mut ck, ray_a, f.cap);
        // ... or above it (y >= yBb.y) -> delegate to circle b
        let ly2 = len + r + rng.range(0.5, 20.0);
        let ray_b = f.ray(lx, ly2, -lx, ly2 + rng.range(-1.0, 1.0), tt);
        cov.hit(capsule_path(ray_b, f.cap));
        diff_capsule(&mut ck, ray_b, f.cap);
        // ... and the fall-through case: same side, both |x| >= r
        let ray_f = f.ray(lx, ly, lx + rng.range(0.0, 5.0), ly + 1.0, tt);
        cov.hit(capsule_path(ray_f, f.cap));
        diff_capsule(&mut ck, ray_f, f.cap);
    }
    cov.require(&[CAP_DELEG_A_BY_Y, CAP_DELEG_B_BY_Y, CAP_FALLTHROUGH], 50);
    ck.finish();
}

#[test]
fn cfg_30_raytocapsule_axis_shapes() {
    let mut ck = Checker::new("cfg_30_raytocapsule_axis_shapes");
    let mut cov = Cover::new("cfg_30_raytocapsule_axis_shapes", &CAP_NAMES);
    let mut rng = Rng::new(0x1E1E);
    for _ in 0..n_iters() {
        let base = rng.coord_v();
        let len = match rng.below(4) {
            0 => 0.0, // degenerate a == b
            1 => rng.range(0.0, 0.01),
            2 => rng.range(0.0, 1.0e6),
            _ => rng.range(0.0, 50.0),
        };
        // axis-aligned, diagonal and reversed axes
        let dir = match rng.below(6) {
            0 => v(0.0, 1.0),
            1 => v(0.0, -1.0),
            2 => v(1.0, 0.0),
            3 => v(-1.0, 0.0),
            4 => v(0.70710678, 0.70710678),
            _ => v(-0.70710678, 0.70710678),
        };
        let r = match rng.below(5) {
            0 => 0.0,
            1 => rng.range(0.0, 1.0e-6),
            2 => rng.range(0.0, 1.0e6),
            _ => rng.range(0.0, 10.0),
        };
        let cap = C2Capsule {
            a: base,
            b: v(base.x + dir.x * len, base.y + dir.y * len),
            r,
        };
        let ray = rng.nice_ray();
        cov.hit(capsule_path(ray, cap));
        diff_capsule(&mut ck, ray, cap);
        // and a ray aimed at the capsule centre
        let mid = v((cap.a.x + cap.b.x) * 0.5, (cap.a.y + cap.b.y) * 0.5);
        let ang = rng.range(-3.15, 3.15);
        let dist = rng.range(0.0, 50.0);
        let p0 = v(mid.x + dist * ang.cos(), mid.y + dist * ang.sin());
        let ray2 = C2Ray {
            p: p0,
            d: v(-ang.cos(), -ang.sin()),
            t: dist * rng.range(0.0, 2.0),
        };
        cov.hit(capsule_path(ray2, cap));
        diff_capsule(&mut ck, ray2, cap);
    }
    cov.report();
    ck.finish();
}

#[test]
fn cfg_31_raytocapsule_random_bits() {
    let mut ck = Checker::new("cfg_31_raytocapsule_random_bits");
    let mut cov = Cover::new("cfg_31_raytocapsule_random_bits", &CAP_NAMES);
    let mut rng = Rng::new(0x1F1F);
    for _ in 0..n_iters() {
        let ray = rng.wild_ray();
        let cap = rng.wild_capsule();
        cov.hit(capsule_path(ray, cap));
        diff_capsule(&mut ck, ray, cap);
    }
    for _ in 0..n_iters() {
        let mut ray = rng.nice_ray();
        let mut cap = C2Capsule {
            a: rng.coord_v(),
            b: rng.coord_v(),
            r: rng.radius(),
        };
        match rng.below(8) {
            0 => ray.p.x = rng.special(),
            1 => ray.d.y = rng.special(),
            2 => ray.t = rng.special(),
            3 => cap.r = rng.special(),
            4 => cap.a.y = rng.special(),
            5 => cap.b.x = rng.special(),
            6 => cap.b = cap.a, // degenerate axis -> c2Norm(0,0) == NaN
            _ => {}
        }
        cov.hit(capsule_path(ray, cap));
        diff_capsule(&mut ck, ray, cap);
    }
    cov.report();
    ck.finish();
}

/* ========================= rows 32-35: c2CastRay ========================= */

#[test]
fn cfg_32_castray_circle() {
    let p = apis();
    let mut ck = Checker::new("cfg_32_castray_circle");
    let mut cov = Cover::new("cfg_32_castray_circle", &CIRCLE_NAMES);
    let mut rng = Rng::new(0x2020);
    for i in 0..n_iters() {
        let ray = if i % 2 == 0 { rng.nice_ray() } else { rng.wild_ray() };
        let c = if i % 2 == 0 {
            C2Circle { p: rng.coord_v(), r: rng.radius() }
        } else {
            rng.wild_circle()
        };
        cov.hit(circle_path(ray, c));
        let mut oc = POISON;
        let mut or_ = POISON;
        let ctx = || format!("c2CastRay(CIRCLE, {}, {})", fray(ray), fcircle(c));
        let (rc, rr) = unsafe {
            (
                (p.c.c2CastRay)(ray, &c as *const C2Circle as *const _, C2_TYPE_CIRCLE, &mut oc),
                (p.r.c2CastRay)(ray, &c as *const C2Circle as *const _, C2_TYPE_CIRCLE, &mut or_),
            )
        };
        ck.int("ret", rc, rr, &ctx);
        ck.cast("out", oc, or_, &ctx);
        // the dispatcher must agree with the direct low-level call, in both libs
        let mut dc = POISON;
        let mut dr = POISON;
        let (dcr, drr) = unsafe {
            (
                (p.c.c2RaytoCircle)(ray, c, &mut dc),
                (p.r.c2RaytoCircle)(ray, c, &mut dr),
            )
        };
        ck.int("direct_c_vs_dispatch_c", rc, dcr, &ctx);
        ck.int("direct_r_vs_dispatch_r", rr, drr, &ctx);
        ck.cast("direct_out", dc, dr, &ctx);
    }
    cov.require(&[CIRCLE_HIT, CIRCLE_DISC_NEG], 20);
    ck.finish();
}

#[test]
fn cfg_33_castray_aabb() {
    let p = apis();
    let mut ck = Checker::new("cfg_33_castray_aabb");
    let mut cov = Cover::new("cfg_33_castray_aabb", &AABB_NAMES);
    let mut rng = Rng::new(0x2121);
    for i in 0..n_iters() {
        let ray = if i % 2 == 0 { rng.nice_ray() } else { rng.wild_ray() };
        let b = if i % 2 == 0 { rng.proper_aabb() } else { rng.wild_aabb() };
        cov.hit(aabb_path(ray, b));
        let mut oc = POISON;
        let mut or_ = POISON;
        let ctx = || format!("c2CastRay(AABB, {}, {})", fray(ray), faabb(b));
        let (rc, rr) = unsafe {
            (
                (p.c.c2CastRay)(ray, &b as *const C2AABB as *const _, C2_TYPE_AABB, &mut oc),
                (p.r.c2CastRay)(ray, &b as *const C2AABB as *const _, C2_TYPE_AABB, &mut or_),
            )
        };
        ck.int("ret", rc, rr, &ctx);
        ck.cast("out", oc, or_, &ctx);
        let mut dc = POISON;
        let mut dr = POISON;
        let (dcr, drr) = unsafe {
            (
                (p.c.c2RaytoAABB)(ray, b, &mut dc),
                (p.r.c2RaytoAABB)(ray, b, &mut dr),
            )
        };
        ck.int("direct_c_vs_dispatch_c", rc, dcr, &ctx);
        ck.int("direct_r_vs_dispatch_r", rr, drr, &ctx);
        ck.cast("direct_out", dc, dr, &ctx);
    }
    cov.require(&[AABB_BROAD_REJECT], 20);
    ck.finish();
}

#[test]
fn cfg_34_castray_capsule() {
    let p = apis();
    let mut ck = Checker::new("cfg_34_castray_capsule");
    let mut cov = Cover::new("cfg_34_castray_capsule", &CAP_NAMES);
    let mut rng = Rng::new(0x2222);
    for i in 0..n_iters() {
        let ray = if i % 2 == 0 { rng.nice_ray() } else { rng.wild_ray() };
        let cap = if i % 2 == 0 {
            C2Capsule { a: rng.coord_v(), b: rng.coord_v(), r: rng.radius() }
        } else {
            rng.wild_capsule()
        };
        cov.hit(capsule_path(ray, cap));
        let mut oc = POISON;
        let mut or_ = POISON;
        let ctx = || format!("c2CastRay(CAPSULE, {}, {})", fray(ray), fcap(cap));
        let (rc, rr) = unsafe {
            (
                (p.c.c2CastRay)(ray, &cap as *const C2Capsule as *const _, C2_TYPE_CAPSULE, &mut oc),
                (p.r.c2CastRay)(ray, &cap as *const C2Capsule as *const _, C2_TYPE_CAPSULE, &mut or_),
            )
        };
        ck.int("ret", rc, rr, &ctx);
        ck.cast("out", oc, or_, &ctx);
        let mut dc = POISON;
        let mut dr = POISON;
        let (dcr, drr) = unsafe {
            (
                (p.c.c2RaytoCapsule)(ray, cap, &mut dc),
                (p.r.c2RaytoCapsule)(ray, cap, &mut dr),
            )
        };
        ck.int("direct_c_vs_dispatch_c", rc, dcr, &ctx);
        ck.int("direct_r_vs_dispatch_r", rr, drr, &ctx);
        ck.cast("direct_out", dc, dr, &ctx);
    }
    cov.report();
    ck.finish();
}

#[test]
fn cfg_35_castray_mixed_stream() {
    let p = apis();
    let mut ck = Checker::new("cfg_35_castray_mixed_stream");
    let mut rng = Rng::new(0x2323);
    // ONE `out` struct reused across the whole stream, in both libraries: any
    // difference in *which* fields a path writes shows up as drift later on.
    let mut oc = POISON;
    let mut or_ = POISON;
    for i in 0..n_iters() {
        let ray = if i % 3 == 0 { rng.wild_ray() } else { rng.nice_ray() };
        let which = rng.below(3);
        let (rc, rr, tag) = unsafe {
            match which {
                0 => {
                    let c = if i % 2 == 0 {
                        C2Circle { p: rng.coord_v(), r: rng.radius() }
                    } else {
                        rng.wild_circle()
                    };
                    let pc = &c as *const C2Circle as *const _;
                    (
                        (p.c.c2CastRay)(ray, pc, C2_TYPE_CIRCLE, &mut oc),
                        (p.r.c2CastRay)(ray, pc, C2_TYPE_CIRCLE, &mut or_),
                        format!("CIRCLE {}", fcircle(c)),
                    )
                }
                1 => {
                    let b = if i % 2 == 0 { rng.proper_aabb() } else { rng.wild_aabb() };
                    let pb = &b as *const C2AABB as *const _;
                    (
                        (p.c.c2CastRay)(ray, pb, C2_TYPE_AABB, &mut oc),
                        (p.r.c2CastRay)(ray, pb, C2_TYPE_AABB, &mut or_),
                        format!("AABB {}", faabb(b)),
                    )
                }
                _ => {
                    let cap = if i % 2 == 0 {
                        C2Capsule { a: rng.coord_v(), b: rng.coord_v(), r: rng.radius() }
                    } else {
                        rng.wild_capsule()
                    };
                    let pc = &cap as *const C2Capsule as *const _;
                    (
                        (p.c.c2CastRay)(ray, pc, C2_TYPE_CAPSULE, &mut oc),
                        (p.r.c2CastRay)(ray, pc, C2_TYPE_CAPSULE, &mut or_),
                        format!("CAPSULE {}", fcap(cap)),
                    )
                }
            }
        };
        let ctx = || format!("step {i}: {tag} {}", fray(ray));
        ck.int("ret", rc, rr, &ctx);
        ck.cast("out(carried)", oc, or_, &ctx);
    }
    ck.finish();
}

/* ========================== rows 36-39: spec_ray ========================= */

#[test]
fn cfg_36_spec_ray_hit() {
    let mut ck = Checker::new("cfg_36_spec_ray_hit");
    let mut rng = Rng::new(0x2424);
    let mut hits = 0usize;
    let p = apis();
    for _ in 0..n_iters() {
        let center = rng.coord_v();
        let r = rng.range(0.5, 20.0);
        let ang = rng.range(-3.15, 3.15);
        let (ca, sa) = (ang.cos(), ang.sin());
        // ray origin on one side, mouse point beyond the circle on the other
        let d0 = r + rng.range(0.1, 40.0);
        let d1 = r + rng.range(0.1, 40.0);
        let off = rng.range(-0.95, 0.95) * r; // lateral offset, still a hit
        let rp = v(center.x - ca * d0 - sa * off, center.y - sa * d0 + ca * off);
        let mp = v(center.x + ca * d1 - sa * off, center.y + sa * d1 + ca * off);
        let a = [mp.x, mp.y, center.x, center.y, r, rp.x, rp.y];
        let mut oc = POISON;
        if unsafe { (p.c.spec_ray)(&mut oc, a[0], a[1], a[2], a[3], a[4], a[5], a[6]) } != 0 {
            hits += 1;
        }
        diff_spec(&mut ck, a);
    }
    eprintln!("[cfg_36] hits={hits}/{}", n_iters());
    assert!(hits > n_iters() / 2, "the hit configuration barely hits");
    ck.finish();
}

#[test]
fn cfg_37_spec_ray_miss() {
    let mut ck = Checker::new("cfg_37_spec_ray_miss");
    let mut rng = Rng::new(0x2525);
    let mut misses = 0usize;
    let p = apis();
    for _ in 0..n_iters() {
        let center = rng.coord_v();
        let r = rng.range(0.5, 20.0);
        let ang = rng.range(-3.15, 3.15);
        let (ca, sa) = (ang.cos(), ang.sin());
        let variant = rng.below(3);
        let (rp, mp) = match variant {
            // circle behind the ray origin
            0 => (
                v(center.x - ca * (r + 5.0), center.y - sa * (r + 5.0)),
                v(
                    center.x - ca * (r + 5.0 + rng.range(1.0, 40.0)),
                    center.y - sa * (r + 5.0 + rng.range(1.0, 40.0)),
                ),
            ),
            // mouse point stops short of the circle
            1 => (
                v(center.x - ca * (r + 30.0), center.y - sa * (r + 30.0)),
                v(center.x - ca * (r + rng.range(1.0, 20.0)), center.y - sa * (r + 5.0)),
            ),
            // laterally offset past the radius
            _ => {
                let off = r * rng.range(1.05, 10.0);
                (
                    v(center.x - ca * 30.0 - sa * off, center.y - sa * 30.0 + ca * off),
                    v(center.x + ca * 30.0 - sa * off, center.y + sa * 30.0 + ca * off),
                )
            }
        };
        let a = [mp.x, mp.y, center.x, center.y, r, rp.x, rp.y];
        let mut oc = POISON;
        if unsafe { (p.c.spec_ray)(&mut oc, a[0], a[1], a[2], a[3], a[4], a[5], a[6]) } == 0 {
            misses += 1;
        }
        diff_spec(&mut ck, a);
    }
    eprintln!("[cfg_37] misses={misses}/{}", n_iters());
    assert!(misses > n_iters() / 2, "the miss configuration barely misses");
    ck.finish();
}

#[test]
fn cfg_38_spec_ray_integer_grid() {
    let mut ck = Checker::new("cfg_38_spec_ray_integer_grid");
    // exhaustive small integer grid: exercises exact ties (t == A.t), zero
    // direction vectors, r == 0 and points exactly on the rim
    for mx in -3..=3 {
        for my in -3..=3 {
            for cx in -2..=2 {
                for cy in -2..=2 {
                    for r in 0..=3 {
                        for rx in -2..=2 {
                            diff_spec(
                                &mut ck,
                                [
                                    mx as f32,
                                    my as f32,
                                    cx as f32,
                                    cy as f32,
                                    r as f32,
                                    rx as f32,
                                    (mx - cy) as f32,
                                ],
                            );
                        }
                    }
                }
            }
        }
    }
    ck.finish();
}

#[test]
fn cfg_39_spec_ray_random_bits() {
    let mut ck = Checker::new("cfg_39_spec_ray_random_bits");
    let mut rng = Rng::new(0x2626);
    for _ in 0..n_iters() {
        let a = [
            rng.wild(),
            rng.wild(),
            rng.wild(),
            rng.wild(),
            rng.wild(),
            rng.wild(),
            rng.wild(),
        ];
        diff_spec(&mut ck, a);
    }
    // pure random bit patterns
    for _ in 0..n_iters() {
        let a = [
            rng.bits_f32(),
            rng.bits_f32(),
            rng.bits_f32(),
            rng.bits_f32(),
            rng.bits_f32(),
            rng.bits_f32(),
            rng.bits_f32(),
        ];
        diff_spec(&mut ck, a);
    }
    // one wild field in an otherwise sane configuration
    for _ in 0..n_iters() {
        let mut a = [
            rng.coord(),
            rng.coord(),
            rng.coord(),
            rng.coord(),
            rng.radius(),
            rng.coord(),
            rng.coord(),
        ];
        let i = rng.below(7) as usize;
        a[i] = rng.special();
        diff_spec(&mut ck, a);
    }
    ck.finish();
}

/* ===================== row 40: out-write tracking ======================== */

#[test]
fn cfg_40_out_poison_write_tracking() {
    let p = apis();
    let mut ck = Checker::new("cfg_40_out_poison_write_tracking");
    let mut rng = Rng::new(0x2727);
    let mut untouched_circle = 0usize;
    let mut untouched_aabb = 0usize;
    let mut written_capsule_on_miss = 0usize;
    for i in 0..n_iters() {
        let ray = if i % 2 == 0 { rng.nice_ray() } else { rng.wild_ray() };

        let c = C2Circle { p: rng.coord_v(), r: rng.radius() };
        let mut oc = POISON;
        let mut or_ = POISON;
        let ctx = || format!("{} {}", fray(ray), fcircle(c));
        let (rc, rr) = unsafe {
            (
                (p.c.c2RaytoCircle)(ray, c, &mut oc),
                (p.r.c2RaytoCircle)(ray, c, &mut or_),
            )
        };
        ck.int("circle ret", rc, rr, &ctx);
        ck.cast("circle out", oc, or_, &ctx);
        if rc == 0 {
            // the C never writes `out` when it rejects — so must the Rust
            assert_eq!(oc.t.to_bits(), POISON.t.to_bits(), "C wrote out on a miss");
            assert_eq!(or_.t.to_bits(), POISON.t.to_bits(), "rust wrote out on a miss");
            assert_eq!(or_.n.x.to_bits(), POISON.n.x.to_bits());
            assert_eq!(or_.n.y.to_bits(), POISON.n.y.to_bits());
            untouched_circle += 1;
        }

        let b = rng.proper_aabb();
        let mut oc = POISON;
        let mut or_ = POISON;
        let ctx = || format!("{} {}", fray(ray), faabb(b));
        let (rc, rr) = unsafe {
            (
                (p.c.c2RaytoAABB)(ray, b, &mut oc),
                (p.r.c2RaytoAABB)(ray, b, &mut or_),
            )
        };
        ck.int("aabb ret", rc, rr, &ctx);
        ck.cast("aabb out", oc, or_, &ctx);
        if rc == 0 {
            assert_eq!(oc.t.to_bits(), POISON.t.to_bits());
            assert_eq!(or_.t.to_bits(), POISON.t.to_bits());
            untouched_aabb += 1;
        }

        let cap = C2Capsule { a: rng.coord_v(), b: rng.coord_v(), r: rng.radius() };
        let mut oc = POISON;
        let mut or_ = POISON;
        let ctx = || format!("{} {}", fray(ray), fcap(cap));
        let (rc, rr) = unsafe {
            (
                (p.c.c2RaytoCapsule)(ray, cap, &mut oc),
                (p.r.c2RaytoCapsule)(ray, cap, &mut or_),
            )
        };
        ck.int("capsule ret", rc, rr, &ctx);
        ck.cast("capsule out", oc, or_, &ctx);
        if rc == 0 {
            // c2RaytoCapsule ALWAYS writes out (L243/244) before deciding
            assert_ne!(
                oc.t.to_bits(),
                POISON.t.to_bits(),
                "C left out untouched on a capsule miss"
            );
            assert_ne!(or_.t.to_bits(), POISON.t.to_bits());
            written_capsule_on_miss += 1;
        }
    }
    eprintln!(
        "[cfg_40] circle-miss-untouched={untouched_circle} aabb-miss-untouched={untouched_aabb} \
         capsule-miss-but-written={written_capsule_on_miss}"
    );
    assert!(untouched_circle > 100 && untouched_aabb > 100 && written_capsule_on_miss > 100);
    ck.finish();
}
