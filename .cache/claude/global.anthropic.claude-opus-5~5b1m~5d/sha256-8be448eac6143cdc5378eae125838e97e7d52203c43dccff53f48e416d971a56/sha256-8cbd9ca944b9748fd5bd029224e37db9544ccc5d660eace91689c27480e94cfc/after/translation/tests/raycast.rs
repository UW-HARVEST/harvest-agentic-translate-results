//! Phase B — CONFIGS.md rows 30..68: the raycast entry points, `c2CastRay` and
//! `gen_ray`, driven through both `.so` exports.
//!
//! Branch coverage is verified with a *classifier* that recomputes each branch
//! predicate using the **C** library's own exported vector helpers, so the
//! classification is ground truth rather than a Rust-side guess.

mod common;
use common::*;
use std::collections::BTreeMap;
use std::ffi::c_void;

const N: usize = 20_000;

// ---------------------------------------------------------------------------
// Differential drivers
// ---------------------------------------------------------------------------

/// Calls both `.so`s with `out` pre-filled with the same sentinel, so "the C
/// never wrote through `out`" is itself compared bit-for-bit.
fn diff_circle(d: &mut Diffs, ray: C2Ray, c: C2Circle) {
    let l = libs();
    let mut oc = SENTINEL;
    let mut or = SENTINEL;
    let rc = unsafe { (l.c.c2RaytoCircle)(ray, c, &mut oc) };
    let rr = unsafe { (l.r.c2RaytoCircle)(ray, c, &mut or) };
    d.check(rc == rr && rceq(oc, or), || {
        format!(
            "c2RaytoCircle({}, {}) -> C ret={} out={} | R ret={} out={}",
            rays(ray),
            circs(c),
            rc,
            rcs(oc),
            rr,
            rcs(or)
        )
    });
}

fn diff_aabb(d: &mut Diffs, ray: C2Ray, b: C2AABB) {
    let l = libs();
    let mut oc = SENTINEL;
    let mut or = SENTINEL;
    let rc = unsafe { (l.c.c2RaytoAABB)(ray, b, &mut oc) };
    let rr = unsafe { (l.r.c2RaytoAABB)(ray, b, &mut or) };
    d.check(rc == rr && rceq(oc, or), || {
        format!(
            "c2RaytoAABB({}, {}) -> C ret={} out={} | R ret={} out={}",
            rays(ray),
            aabbs(b),
            rc,
            rcs(oc),
            rr,
            rcs(or)
        )
    });
}

fn diff_capsule(d: &mut Diffs, ray: C2Ray, b: C2Capsule) {
    let l = libs();
    let mut oc = SENTINEL;
    let mut or = SENTINEL;
    let rc = unsafe { (l.c.c2RaytoCapsule)(ray, b, &mut oc) };
    let rr = unsafe { (l.r.c2RaytoCapsule)(ray, b, &mut or) };
    d.check(rc == rr && rceq(oc, or), || {
        format!(
            "c2RaytoCapsule({}, {}) -> C ret={} out={} | R ret={} out={}",
            rays(ray),
            caps(b),
            rc,
            rcs(oc),
            rr,
            rcs(or)
        )
    });
}

// ---------------------------------------------------------------------------
// Ground-truth branch classifiers (all arithmetic done by the C library)
// ---------------------------------------------------------------------------

fn class_circle(ray: C2Ray, c: C2Circle) -> &'static str {
    let l = &libs().c;
    let p = c.p;
    let m = (l.c2Sub)(ray.p, p);
    let cc = (l.c2Dot)(m, m) - c.r * c.r;
    let b = (l.c2Dot)(m, ray.d);
    let disc = b * b - cc;
    if disc < 0.0 {
        return "circle:disc<0";
    }
    let t = -b - disc.sqrt();
    if !(t >= 0.0) {
        return "circle:t<0";
    }
    if !(t <= ray.t) {
        return "circle:t>A.t";
    }
    "circle:hit"
}

fn class_aabb(ray: C2Ray, b: C2AABB, out_on_hit: Option<C2Raycast>) -> &'static str {
    let l = &libs().c;
    let p0 = ray.p;
    let p1 = (l.c2Add)(ray.p, (l.c2Mulvs)(ray.d, ray.t));
    let a_box = C2AABB {
        min: (l.c2Minv)(p0, p1),
        max: (l.c2Maxv)(p0, p1),
    };
    if (l.c2AABBtoAABB)(a_box, b) == 0 {
        return "aabb:bbox-miss";
    }
    let n = (l.c2Skew)((l.c2Sub)(p1, p0));
    let abs_n = (l.c2Absv)(n);
    let half = (l.c2Mulvs)((l.c2Sub)(b.max, b.min), 0.5);
    let ctr = (l.c2Mulvs)((l.c2Add)(b.min, b.max), 0.5);
    let dv = (l.c2Dot)(n, (l.c2Sub)(p0, ctr));
    let d = (if dv < 0.0 { -dv } else { dv }) - (l.c2Dot)(abs_n, half);
    if d > 0.0 {
        return "aabb:sat-miss";
    }
    match out_on_hit {
        None => "aabb:t>1-miss",
        Some(o) => {
            if o.n.x == -1.0 {
                "aabb:face-x"
            } else if o.n.x == 1.0 {
                "aabb:face+x"
            } else if o.n.y == -1.0 {
                "aabb:face-y"
            } else {
                "aabb:face+y"
            }
        }
    }
}

fn class_capsule(ray: C2Ray, b: C2Capsule) -> &'static str {
    let l = &libs().c;
    let my = (l.c2Norm)((l.c2Sub)(b.b, b.a));
    let mx = (l.c2CCW90)(my);
    let m = C2m { x: mx, y: my };
    let cap_n = (l.c2Sub)(b.b, b.a);
    let y_bb = (l.c2MulmvT)(m, cap_n);
    let y_ap = (l.c2MulmvT)(m, (l.c2Sub)(ray.p, b.a));
    let y_ad = (l.c2MulmvT)(m, ray.d);
    let y_ae = (l.c2Add)(y_ap, (l.c2Mulvs)(y_ad, ray.t));
    let bb = C2AABB {
        min: C2v { x: -b.r, y: 0.0 },
        max: C2v { x: b.r, y: y_bb.y },
    };
    if (l.c2AABBtoPoint)(bb, y_ap) != 0 {
        return "cap:slab";
    }
    if (l.c2CircleToPoint)(C2Circle { p: b.a, r: b.r }, ray.p) != 0 {
        return "cap:in-capA";
    }
    if (l.c2CircleToPoint)(C2Circle { p: b.b, r: b.r }, ray.p) != 0 {
        return "cap:in-capB";
    }
    let absx = |v: f32| if v < 0.0 { -v } else { v };
    let mn = {
        let (p, q) = (absx(y_ae.x), absx(y_ap.x));
        if p < q { p } else { q }
    };
    if y_ae.x * y_ap.x < 0.0 || mn < b.r {
        if absx(y_ap.x) < b.r {
            if y_ap.y < 0.0 {
                return "cap:circA";
            } else {
                return "cap:circB";
            }
        }
        let c = if y_ap.x > 0.0 { b.r } else { -b.r };
        let dd = y_ae.x - y_ap.x;
        let t = (c - y_ap.x) / dd;
        let y = (y_ae.y - y_ap.y) * t + y_ap.y;
        if y <= 0.0 {
            return "cap:crossA";
        }
        if y >= y_bb.y {
            return "cap:crossB";
        }
        if c > 0.0 {
            return "cap:flat-Mx";
        }
        return "cap:flat-skew";
    }
    "cap:miss"
}

struct Cov(BTreeMap<&'static str, usize>);
impl Cov {
    fn new() -> Self {
        Cov(BTreeMap::new())
    }
    fn hit(&mut self, k: &'static str) {
        *self.0.entry(k).or_insert(0) += 1;
    }
    fn require(&self, keys: &[&str], what: &str) {
        eprintln!("  coverage[{what}]: {:?}", self.0);
        for k in keys {
            assert!(
                self.0.get(k).copied().unwrap_or(0) > 0,
                "branch {k:?} was never reached in {what}; coverage = {:?}",
                self.0
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Scenario generators
// ---------------------------------------------------------------------------

fn structured_circle(rng: &mut Rng) -> (C2Ray, C2Circle) {
    let l = &libs().c;
    let c = C2Circle {
        p: rng.geo_v(30.0),
        r: rng.range(0.05, 12.0),
    };
    let p = rng.geo_v(40.0);
    // Aim at a point near (sometimes inside, sometimes far from) the circle.
    let spread = [0.0f32, 0.5, 1.0, 1.5, 3.0][rng.below(5) as usize] * c.r;
    let target = (l.c2Add)(c.p, rng.geo_v(spread.max(1e-6)));
    let toward = if rng.next_u32() & 7 == 0 {
        // sometimes point *away* from the circle -> disc<0 / t<0 branches
        (l.c2Sub)(p, target)
    } else {
        (l.c2Sub)(target, p)
    };
    let dir = (l.c2Norm)(toward);
    let dist = (l.c2Len)(toward);
    let factor = [0.0f32, 0.05, 0.3, 0.9, 1.0, 1.5, 5.0, -1.0][rng.below(8) as usize];
    let ray = C2Ray {
        p,
        d: dir,
        t: dist * factor,
    };
    (ray, c)
}

fn structured_aabb(rng: &mut Rng) -> (C2Ray, C2AABB) {
    let l = &libs().c;
    let q = rng.geo_v(30.0);
    let s = C2v {
        x: rng.range(0.05, 15.0),
        y: rng.range(0.05, 15.0),
    };
    let b = C2AABB {
        min: q,
        max: (l.c2Add)(q, s),
    };
    let ctr = (l.c2Mulvs)((l.c2Add)(b.min, b.max), 0.5);
    let p = rng.geo_v(45.0);
    let target = match rng.below(6) {
        0 => ctr,
        1 => b.min,
        2 => b.max,
        3 => C2v { x: b.min.x, y: b.max.y },
        4 => (l.c2Add)(ctr, rng.geo_v(20.0)),
        _ => (l.c2Add)(ctr, rng.geo_v(2.0)),
    };
    // Occasionally force an axis-parallel ray (da == db -> the `d != 0` guard).
    let mut target = target;
    match rng.below(8) {
        0 => target.y = p.y,
        1 => target.x = p.x,
        _ => {}
    }
    let toward = (l.c2Sub)(target, p);
    let dir = (l.c2Norm)(toward);
    let dist = (l.c2Len)(toward);
    let factor = [0.0f32, 0.1, 0.5, 0.95, 1.0, 1.2, 4.0, -1.0][rng.below(8) as usize];
    (
        C2Ray {
            p,
            d: dir,
            t: dist * factor,
        },
        b,
    )
}

fn structured_capsule(rng: &mut Rng) -> (C2Ray, C2Capsule) {
    let l = &libs().c;
    let a = rng.geo_v(25.0);
    let axis = C2v {
        x: rng.geo(25.0),
        y: rng.geo(25.0),
    };
    let cap = C2Capsule {
        a,
        b: (l.c2Add)(a, axis),
        r: rng.range(0.05, 10.0),
    };
    let mid = (l.c2Mulvs)((l.c2Add)(cap.a, cap.b), 0.5);
    let p = match rng.below(8) {
        0 => (l.c2Add)(cap.a, rng.geo_v(cap.r * 0.7)), // inside end-cap A
        1 => (l.c2Add)(cap.b, rng.geo_v(cap.r * 0.7)), // inside end-cap B
        2 => (l.c2Add)(mid, rng.geo_v(cap.r * 0.5)),   // inside the slab
        3 => (l.c2Add)(mid, rng.geo_v(cap.r * 1.3)),   // just outside
        _ => rng.geo_v(45.0),
    };
    let target = match rng.below(5) {
        0 => mid,
        1 => cap.a,
        2 => cap.b,
        3 => (l.c2Add)(mid, rng.geo_v(cap.r * 2.0)),
        _ => (l.c2Add)(mid, rng.geo_v(20.0)),
    };
    let toward = (l.c2Sub)(target, p);
    let dir = (l.c2Norm)(toward);
    let dist = (l.c2Len)(toward);
    let factor = [0.0f32, 0.1, 0.6, 1.0, 1.4, 6.0, -1.0, 100.0][rng.below(8) as usize];
    (
        C2Ray {
            p,
            d: dir,
            t: dist * factor,
        },
        cap,
    )
}

// ---------------------------------------------------------------------------
// Rows 30-37: c2RaytoCircle
// ---------------------------------------------------------------------------

#[test]
fn rows30_35_circle_branches_and_degenerate() {
    let mut d = Diffs::new("rows30-35 c2RaytoCircle branches");
    let mut cov = Cov::new();
    let v = |x: f32, y: f32| C2v { x, y };
    let mut cases: Vec<(C2Ray, C2Circle)> = vec![
        // row 30: line miss (disc < 0)
        (
            C2Ray { p: v(-10.0, 20.0), d: v(1.0, 0.0), t: 100.0 },
            C2Circle { p: v(0.0, 0.0), r: 5.0 },
        ),
        // row 31: genuine hit
        (
            C2Ray { p: v(-10.0, 0.0), d: v(1.0, 0.0), t: 100.0 },
            C2Circle { p: v(0.0, 0.0), r: 5.0 },
        ),
        // row 32: tangent, disc == 0
        (
            C2Ray { p: v(-10.0, 5.0), d: v(1.0, 0.0), t: 100.0 },
            C2Circle { p: v(0.0, 0.0), r: 5.0 },
        ),
        // row 33: root behind origin (pointing away)
        (
            C2Ray { p: v(-10.0, 0.0), d: v(-1.0, 0.0), t: 100.0 },
            C2Circle { p: v(0.0, 0.0), r: 5.0 },
        ),
        // row 33: origin inside the circle
        (
            C2Ray { p: v(1.0, 1.0), d: v(1.0, 0.0), t: 100.0 },
            C2Circle { p: v(0.0, 0.0), r: 5.0 },
        ),
        // row 34: hit exists but t > A.t
        (
            C2Ray { p: v(-10.0, 0.0), d: v(1.0, 0.0), t: 1.0 },
            C2Circle { p: v(0.0, 0.0), r: 5.0 },
        ),
        // row 35: A.t == 0 / negative / inf
        (
            C2Ray { p: v(-5.0, 0.0), d: v(1.0, 0.0), t: 0.0 },
            C2Circle { p: v(0.0, 0.0), r: 5.0 },
        ),
        (
            C2Ray { p: v(-10.0, 0.0), d: v(1.0, 0.0), t: -1.0 },
            C2Circle { p: v(0.0, 0.0), r: 5.0 },
        ),
        (
            C2Ray { p: v(-10.0, 0.0), d: v(1.0, 0.0), t: f32::INFINITY },
            C2Circle { p: v(0.0, 0.0), r: 5.0 },
        ),
        // row 35: r == 0 / r < 0
        (
            C2Ray { p: v(-10.0, 0.0), d: v(1.0, 0.0), t: 100.0 },
            C2Circle { p: v(0.0, 0.0), r: 0.0 },
        ),
        (
            C2Ray { p: v(-10.0, 0.0), d: v(1.0, 0.0), t: 100.0 },
            C2Circle { p: v(0.0, 0.0), r: -5.0 },
        ),
        // row 35: unnormalised d, zero d
        (
            C2Ray { p: v(-10.0, 0.0), d: v(7.0, 0.0), t: 100.0 },
            C2Circle { p: v(0.0, 0.0), r: 5.0 },
        ),
        (
            C2Ray { p: v(-10.0, 0.0), d: v(0.0, 0.0), t: 100.0 },
            C2Circle { p: v(0.0, 0.0), r: 5.0 },
        ),
        // ERRORS 39-style: NaN d from normalising a zero vector
        (
            C2Ray { p: v(1.0, 2.0), d: (libs().c.c2Norm)(v(0.0, 0.0)), t: 3.0 },
            C2Circle { p: v(0.0, 0.0), r: 5.0 },
        ),
    ];
    // Special values injected into A.t and B.r.
    for &s in SPECIALS {
        cases.push((
            C2Ray { p: v(-10.0, 0.0), d: v(1.0, 0.0), t: s },
            C2Circle { p: v(0.0, 0.0), r: 5.0 },
        ));
        cases.push((
            C2Ray { p: v(-10.0, 0.0), d: v(1.0, 0.0), t: 100.0 },
            C2Circle { p: v(0.0, 0.0), r: s },
        ));
        cases.push((
            C2Ray { p: v(-10.0, 0.0), d: v(s, 0.0), t: 100.0 },
            C2Circle { p: v(0.0, 0.0), r: 5.0 },
        ));
        cases.push((
            C2Ray { p: v(s, 0.0), d: v(1.0, 0.0), t: 100.0 },
            C2Circle { p: v(0.0, 0.0), r: 5.0 },
        ));
    }
    for (ray, c) in cases {
        cov.hit(class_circle(ray, c));
        diff_circle(&mut d, ray, c);
    }
    cov.require(
        &["circle:disc<0", "circle:t<0", "circle:t>A.t", "circle:hit"],
        "c2RaytoCircle rows 30-35",
    );
    d.finish();
}

#[test]
fn row36_circle_random_bits() {
    let mut rng = Rng::new(0x5EED_0036);
    let mut d = Diffs::new("row36 c2RaytoCircle random bit patterns");
    for _ in 0..N {
        let ray = C2Ray {
            p: rng.mixed_v(),
            d: rng.mixed_v(),
            t: rng.mixed_f32(),
        };
        let c = C2Circle {
            p: rng.mixed_v(),
            r: rng.mixed_f32(),
        };
        diff_circle(&mut d, ray, c);
    }
    d.finish();
}

#[test]
fn row37_circle_structured_random() {
    let mut rng = Rng::new(0x5EED_0037);
    let mut d = Diffs::new("row37 c2RaytoCircle structured random");
    let mut cov = Cov::new();
    for _ in 0..N {
        let (ray, c) = structured_circle(&mut rng);
        cov.hit(class_circle(ray, c));
        diff_circle(&mut d, ray, c);
    }
    cov.require(
        &["circle:disc<0", "circle:t<0", "circle:t>A.t", "circle:hit"],
        "c2RaytoCircle row 37",
    );
    d.finish();
}

// ---------------------------------------------------------------------------
// Rows 38-45: c2RaytoAABB
// ---------------------------------------------------------------------------

fn aabb_classify_and_diff(d: &mut Diffs, cov: &mut Cov, ray: C2Ray, b: C2AABB) {
    let l = libs();
    let mut oc = SENTINEL;
    let rc = unsafe { (l.c.c2RaytoAABB)(ray, b, &mut oc) };
    cov.hit(class_aabb(ray, b, if rc != 0 { Some(oc) } else { None }));
    diff_aabb(d, ray, b);
}

#[test]
fn rows38_43_aabb_branches_and_degenerate() {
    let mut d = Diffs::new("rows38-43 c2RaytoAABB branches");
    let mut cov = Cov::new();
    let v = |x: f32, y: f32| C2v { x, y };
    let bx = C2AABB { min: v(-5.0, -5.0), max: v(5.0, 5.0) };
    let mut cases: Vec<(C2Ray, C2AABB)> = vec![
        // row 38: ray bbox misses B
        (C2Ray { p: v(50.0, 50.0), d: v(1.0, 0.0), t: 1.0 }, bx),
        // row 39: SAT reject (bbox overlaps but the ray line misses)
        (C2Ray { p: v(-20.0, 9.0), d: (libs().c.c2Norm)(v(1.0, -1.0)), t: 40.0 }, bx),
        // row 40: hit each face
        (C2Ray { p: v(-20.0, 0.0), d: v(1.0, 0.0), t: 100.0 }, bx),
        (C2Ray { p: v(20.0, 0.0), d: v(-1.0, 0.0), t: 100.0 }, bx),
        (C2Ray { p: v(0.0, -20.0), d: v(0.0, 1.0), t: 100.0 }, bx),
        (C2Ray { p: v(0.0, 20.0), d: v(0.0, -1.0), t: 100.0 }, bx),
        (C2Ray { p: v(-20.0, -20.0), d: (libs().c.c2Norm)(v(1.0, 1.0)), t: 100.0 }, bx),
        // row 42: origin inside B
        (C2Ray { p: v(0.0, 0.0), d: v(1.0, 0.0), t: 100.0 }, bx),
        // row 42: degenerate box (min == max)
        (
            C2Ray { p: v(-20.0, 0.0), d: v(1.0, 0.0), t: 100.0 },
            C2AABB { min: v(0.0, 0.0), max: v(0.0, 0.0) },
        ),
        // row 42: inverted box (min > max)
        (
            C2Ray { p: v(-20.0, 0.0), d: v(1.0, 0.0), t: 100.0 },
            C2AABB { min: v(5.0, 5.0), max: v(-5.0, -5.0) },
        ),
        // row 43: A.t = 0 / negative / inf
        (C2Ray { p: v(-20.0, 0.0), d: v(1.0, 0.0), t: 0.0 }, bx),
        (C2Ray { p: v(-20.0, 0.0), d: v(1.0, 0.0), t: -50.0 }, bx),
        (C2Ray { p: v(-20.0, 0.0), d: v(1.0, 0.0), t: f32::INFINITY }, bx),
        // row 43: zero / unnormalised d
        (C2Ray { p: v(-20.0, 0.0), d: v(0.0, 0.0), t: 100.0 }, bx),
        (C2Ray { p: v(0.0, 0.0), d: v(0.0, 0.0), t: 100.0 }, bx),
        (C2Ray { p: v(-20.0, 0.0), d: v(11.0, 0.0), t: 100.0 }, bx),
        // signed zeros in the bounds
        (
            C2Ray { p: v(-1.0, 0.0), d: v(1.0, 0.0), t: 2.0 },
            C2AABB { min: v(-0.0, -0.0), max: v(0.0, 0.0) },
        ),
    ];
    for &s in SPECIALS {
        cases.push((C2Ray { p: v(-20.0, 0.0), d: v(1.0, 0.0), t: s }, bx));
        cases.push((C2Ray { p: v(-20.0, 0.0), d: v(s, 0.0), t: 100.0 }, bx));
        cases.push((C2Ray { p: v(s, 0.0), d: v(1.0, 0.0), t: 100.0 }, bx));
        cases.push((
            C2Ray { p: v(-20.0, 0.0), d: v(1.0, 0.0), t: 100.0 },
            C2AABB { min: v(s, -5.0), max: v(5.0, 5.0) },
        ));
        cases.push((
            C2Ray { p: v(-20.0, 0.0), d: v(1.0, 0.0), t: 100.0 },
            C2AABB { min: v(-5.0, -5.0), max: v(s, 5.0) },
        ));
    }
    for (ray, b) in cases {
        aabb_classify_and_diff(&mut d, &mut cov, ray, b);
    }
    cov.require(
        &[
            "aabb:bbox-miss",
            "aabb:sat-miss",
            "aabb:face-x",
            "aabb:face+x",
            "aabb:face-y",
            "aabb:face+y",
        ],
        "c2RaytoAABB rows 38-43",
    );
    d.finish();
}

#[test]
fn row44_aabb_random_bits() {
    let mut rng = Rng::new(0x5EED_0044);
    let mut d = Diffs::new("row44 c2RaytoAABB random bit patterns");
    let mut cov = Cov::new();
    for _ in 0..N {
        let ray = C2Ray {
            p: rng.mixed_v(),
            d: rng.mixed_v(),
            t: rng.mixed_f32(),
        };
        let b = C2AABB {
            min: rng.mixed_v(),
            max: rng.mixed_v(),
        };
        aabb_classify_and_diff(&mut d, &mut cov, ray, b);
    }
    // CONFIGS row 41 (`hit == 0`, i.e. all four `t > 1.0f`) is UNREACHABLE with
    // finite inputs: `c2RayToPlane_OneDimensional` returns 0, 1.0, or
    // `da / (da - db)` with `da >= 0 >= db`, which is always <= 1. It is only
    // reachable when some `t` is NaN (NaN <= 1.0 is false), which needs a NaN /
    // infinity somewhere in the ray or the box -- hence it is asserted here in
    // the bit-pattern fuzz row rather than in the geometric row 45.
    cov.require(&["aabb:t>1-miss"], "c2RaytoAABB row 44 (row 41 branch)");
    d.finish();
}

#[test]
fn row45_aabb_structured_random() {
    let mut rng = Rng::new(0x5EED_0045);
    let mut d = Diffs::new("row45 c2RaytoAABB structured random");
    let mut cov = Cov::new();
    for _ in 0..N {
        let (ray, b) = structured_aabb(&mut rng);
        aabb_classify_and_diff(&mut d, &mut cov, ray, b);
    }
    // "aabb:t>1-miss" is not required here -- see the note in row44.
    cov.require(
        &[
            "aabb:bbox-miss",
            "aabb:sat-miss",
            "aabb:face-x",
            "aabb:face+x",
            "aabb:face-y",
            "aabb:face+y",
        ],
        "c2RaytoAABB row 45",
    );
    d.finish();
}

// ---------------------------------------------------------------------------
// Rows 46-60: c2RaytoCapsule
// ---------------------------------------------------------------------------

#[test]
fn rows46_58_capsule_branches_and_degenerate() {
    let l = libs();
    let mut d = Diffs::new("rows46-58 c2RaytoCapsule branches");
    let mut cov = Cov::new();
    let v = |x: f32, y: f32| C2v { x, y };
    let cap = C2Capsule { a: v(0.0, 0.0), b: v(0.0, 20.0), r: 3.0 };
    let mut cases: Vec<(C2Ray, C2Capsule)> = vec![
        // row 46: origin inside the rotated slab
        (C2Ray { p: v(1.0, 10.0), d: v(1.0, 0.0), t: 5.0 }, cap),
        // rows 47/48: origin inside end-cap A / B (outside the slab in y)
        (C2Ray { p: v(1.0, -1.0), d: v(1.0, 0.0), t: 5.0 }, cap),
        (C2Ray { p: v(1.0, 21.0), d: v(1.0, 0.0), t: 5.0 }, cap),
        // rows 49/50: |yAp.x| < r, outside caps -> delegate to a cap circle
        (C2Ray { p: v(1.0, -10.0), d: v(0.0, 1.0), t: 50.0 }, cap),
        (C2Ray { p: v(1.0, 40.0), d: v(0.0, -1.0), t: 50.0 }, cap),
        // rows 51/52: slab crossing that lands on a cap
        (C2Ray { p: v(-20.0, -8.0), d: (l.c.c2Norm)(v(1.0, 0.35)), t: 60.0 }, cap),
        (C2Ray { p: v(-20.0, 28.0), d: (l.c.c2Norm)(v(1.0, -0.35)), t: 60.0 }, cap),
        // rows 53/54: flat-side hits from each side
        (C2Ray { p: v(-20.0, 10.0), d: v(1.0, 0.0), t: 60.0 }, cap),
        (C2Ray { p: v(20.0, 10.0), d: v(-1.0, 0.0), t: 60.0 }, cap),
        // row 55: fall-through miss
        (C2Ray { p: v(30.0, 10.0), d: v(1.0, 0.0), t: 5.0 }, cap),
        (C2Ray { p: v(-30.0, 10.0), d: v(-1.0, 0.0), t: 5.0 }, cap),
        // row 56: degenerate capsule a == b
        (
            C2Ray { p: v(-20.0, 0.0), d: v(1.0, 0.0), t: 60.0 },
            C2Capsule { a: v(0.0, 0.0), b: v(0.0, 0.0), r: 3.0 },
        ),
        // row 57: r == 0 / r < 0
        (
            C2Ray { p: v(-20.0, 10.0), d: v(1.0, 0.0), t: 60.0 },
            C2Capsule { a: v(0.0, 0.0), b: v(0.0, 20.0), r: 0.0 },
        ),
        (
            C2Ray { p: v(-20.0, 10.0), d: v(1.0, 0.0), t: 60.0 },
            C2Capsule { a: v(0.0, 0.0), b: v(0.0, 20.0), r: -3.0 },
        ),
        // row 57: yAe.x == yAp.x -> division by zero at L278
        (C2Ray { p: v(-20.0, 10.0), d: v(0.0, 1.0), t: 60.0 }, cap),
        (C2Ray { p: v(-20.0, 10.0), d: v(0.0, 0.0), t: 60.0 }, cap),
        // row 58: A.t = 0 / negative / inf
        (C2Ray { p: v(-20.0, 10.0), d: v(1.0, 0.0), t: 0.0 }, cap),
        (C2Ray { p: v(-20.0, 10.0), d: v(1.0, 0.0), t: -60.0 }, cap),
        (C2Ray { p: v(-20.0, 10.0), d: v(1.0, 0.0), t: f32::INFINITY }, cap),
        // row 58: reversed capsule (yBb.y < 0)
        (
            C2Ray { p: v(-20.0, 10.0), d: v(1.0, 0.0), t: 60.0 },
            C2Capsule { a: v(0.0, 20.0), b: v(0.0, 0.0), r: 3.0 },
        ),
        // diagonal capsule (non-axis-aligned M)
        (
            C2Ray { p: v(-20.0, 0.0), d: (l.c.c2Norm)(v(1.0, 0.5)), t: 60.0 },
            C2Capsule { a: v(-5.0, -5.0), b: v(12.0, 7.0), r: 2.5 },
        ),
    ];
    for &s in SPECIALS {
        cases.push((C2Ray { p: v(-20.0, 10.0), d: v(1.0, 0.0), t: s }, cap));
        cases.push((C2Ray { p: v(-20.0, 10.0), d: v(s, 0.0), t: 60.0 }, cap));
        cases.push((C2Ray { p: v(s, 10.0), d: v(1.0, 0.0), t: 60.0 }, cap));
        cases.push((
            C2Ray { p: v(-20.0, 10.0), d: v(1.0, 0.0), t: 60.0 },
            C2Capsule { a: v(0.0, 0.0), b: v(0.0, 20.0), r: s },
        ));
        cases.push((
            C2Ray { p: v(-20.0, 10.0), d: v(1.0, 0.0), t: 60.0 },
            C2Capsule { a: v(s, 0.0), b: v(0.0, 20.0), r: 3.0 },
        ));
        cases.push((
            C2Ray { p: v(-20.0, 10.0), d: v(1.0, 0.0), t: 60.0 },
            C2Capsule { a: v(0.0, 0.0), b: v(s, 20.0), r: 3.0 },
        ));
    }
    for (ray, c) in cases {
        cov.hit(class_capsule(ray, c));
        diff_capsule(&mut d, ray, c);
    }
    cov.require(
        &[
            "cap:slab",
            "cap:in-capA",
            "cap:in-capB",
            "cap:circA",
            "cap:circB",
            "cap:crossA",
            "cap:crossB",
            "cap:flat-Mx",
            "cap:flat-skew",
            "cap:miss",
        ],
        "c2RaytoCapsule rows 46-58",
    );
    d.finish();
}

#[test]
fn row59_capsule_random_bits() {
    let mut rng = Rng::new(0x5EED_0059);
    let mut d = Diffs::new("row59 c2RaytoCapsule random bit patterns");
    for _ in 0..N {
        let ray = C2Ray {
            p: rng.mixed_v(),
            d: rng.mixed_v(),
            t: rng.mixed_f32(),
        };
        let c = C2Capsule {
            a: rng.mixed_v(),
            b: rng.mixed_v(),
            r: rng.mixed_f32(),
        };
        diff_capsule(&mut d, ray, c);
    }
    d.finish();
}

#[test]
fn row60_capsule_structured_random() {
    let mut rng = Rng::new(0x5EED_0060);
    let mut d = Diffs::new("row60 c2RaytoCapsule structured random");
    let mut cov = Cov::new();
    for _ in 0..N {
        let (ray, c) = structured_capsule(&mut rng);
        cov.hit(class_capsule(ray, c));
        diff_capsule(&mut d, ray, c);
    }
    cov.require(
        &[
            "cap:slab",
            "cap:in-capA",
            "cap:in-capB",
            "cap:circA",
            "cap:circB",
            "cap:crossA",
            "cap:crossB",
            "cap:flat-Mx",
            "cap:flat-skew",
            "cap:miss",
        ],
        "c2RaytoCapsule row 60",
    );
    d.finish();
}

// ---------------------------------------------------------------------------
// Rows 61-63: c2CastRay dispatch
// ---------------------------------------------------------------------------

#[test]
fn rows61_63_cast_ray_dispatch() {
    let l = libs();
    let mut rng = Rng::new(0x5EED_0061);
    let mut d = Diffs::new("rows61-63 c2CastRay dispatch (all 3 types)");
    for i in 0..N {
        // Row 61 — circle
        let (ray, c) = if i % 2 == 0 {
            structured_circle(&mut rng)
        } else {
            (
                C2Ray { p: rng.mixed_v(), d: rng.mixed_v(), t: rng.mixed_f32() },
                C2Circle { p: rng.mixed_v(), r: rng.mixed_f32() },
            )
        };
        let mut oc = SENTINEL;
        let mut or = SENTINEL;
        let rc = unsafe {
            (l.c.c2CastRay)(ray, &c as *const C2Circle as *const c_void, C2_TYPE_CIRCLE, &mut oc)
        };
        let rr = unsafe {
            (l.r.c2CastRay)(ray, &c as *const C2Circle as *const c_void, C2_TYPE_CIRCLE, &mut or)
        };
        d.check(rc == rr && rceq(oc, or), || {
            format!(
                "c2CastRay(CIRCLE, {}, {}) -> C ret={} out={} | R ret={} out={}",
                rays(ray), circs(c), rc, rcs(oc), rr, rcs(or)
            )
        });
        // ...and it must agree with a direct c2RaytoCircle call on the C side.
        let mut od = SENTINEL;
        let rd = unsafe { (l.c.c2RaytoCircle)(ray, c, &mut od) };
        d.check(rd == rc && rceq(od, oc), || {
            "c2CastRay(CIRCLE) disagrees with c2RaytoCircle in the C library".to_string()
        });

        // Row 62 — AABB
        let (ray, b) = if i % 2 == 0 {
            structured_aabb(&mut rng)
        } else {
            (
                C2Ray { p: rng.mixed_v(), d: rng.mixed_v(), t: rng.mixed_f32() },
                C2AABB { min: rng.mixed_v(), max: rng.mixed_v() },
            )
        };
        let mut oc = SENTINEL;
        let mut or = SENTINEL;
        let rc = unsafe {
            (l.c.c2CastRay)(ray, &b as *const C2AABB as *const c_void, C2_TYPE_AABB, &mut oc)
        };
        let rr = unsafe {
            (l.r.c2CastRay)(ray, &b as *const C2AABB as *const c_void, C2_TYPE_AABB, &mut or)
        };
        d.check(rc == rr && rceq(oc, or), || {
            format!(
                "c2CastRay(AABB, {}, {}) -> C ret={} out={} | R ret={} out={}",
                rays(ray), aabbs(b), rc, rcs(oc), rr, rcs(or)
            )
        });

        // Row 63 — capsule
        let (ray, cp) = if i % 2 == 0 {
            structured_capsule(&mut rng)
        } else {
            (
                C2Ray { p: rng.mixed_v(), d: rng.mixed_v(), t: rng.mixed_f32() },
                C2Capsule { a: rng.mixed_v(), b: rng.mixed_v(), r: rng.mixed_f32() },
            )
        };
        let mut oc = SENTINEL;
        let mut or = SENTINEL;
        let rc = unsafe {
            (l.c.c2CastRay)(ray, &cp as *const C2Capsule as *const c_void, C2_TYPE_CAPSULE, &mut oc)
        };
        let rr = unsafe {
            (l.r.c2CastRay)(ray, &cp as *const C2Capsule as *const c_void, C2_TYPE_CAPSULE, &mut or)
        };
        d.check(rc == rr && rceq(oc, or), || {
            format!(
                "c2CastRay(CAPSULE, {}, {}) -> C ret={} out={} | R ret={} out={}",
                rays(ray), caps(cp), rc, rcs(oc), rr, rcs(or)
            )
        });
    }
    d.finish();
}

// ---------------------------------------------------------------------------
// Rows 64-68: gen_ray (the public header's only entry point)
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn diff_gen_ray(d: &mut Diffs, p: [f32; 18]) -> i32 {
    let l = libs();
    let mut c1 = SENTINEL;
    let mut c2 = SENTINEL;
    let mut c3 = SENTINEL;
    let mut r1 = SENTINEL;
    let mut r2 = SENTINEL;
    let mut r3 = SENTINEL;
    let rc = unsafe {
        (l.c.gen_ray)(
            &mut c1, &mut c2, &mut c3, p[0], p[1], p[2], p[3], p[4], p[5], p[6], p[7], p[8], p[9],
            p[10], p[11], p[12], p[13], p[14], p[15],
        )
    };
    let rr = unsafe {
        (l.r.gen_ray)(
            &mut r1, &mut r2, &mut r3, p[0], p[1], p[2], p[3], p[4], p[5], p[6], p[7], p[8], p[9],
            p[10], p[11], p[12], p[13], p[14], p[15],
        )
    };
    d.check(
        rc == rr && rceq(c1, r1) && rceq(c2, r2) && rceq(c3, r3),
        || {
            format!(
                "gen_ray({:?})\n    C  ret={} c1={} c2={} c3={}\n    R  ret={} c1={} c2={} c3={}",
                &p[..16],
                rc,
                rcs(c1),
                rcs(c2),
                rcs(c3),
                rr,
                rcs(r1),
                rcs(r2),
                rcs(r3)
            )
        },
    );
    rc
}

/// `gen_ray` takes 18 float parameters but only 16 are read by the body
/// (`bb_max_x`, `bb_max_y` are the 17th/18th); build the array in signature
/// order.
fn params(
    mp: (f32, f32),
    rp: (f32, f32),
    c: (f32, f32, f32),
    cap: (f32, f32, f32, f32, f32),
    bb: (f32, f32, f32, f32),
) -> [f32; 18] {
    [
        mp.0, mp.1, rp.0, rp.1, c.0, c.1, c.2, cap.0, cap.1, cap.2, cap.3, cap.4, bb.0, bb.1, bb.2,
        bb.3, 0.0, 0.0,
    ]
}

#[test]
fn row64_gen_ray_all_hit_bit_combinations() {
    let mut d = Diffs::new("row64 gen_ray all 8 hit-bit combinations");
    let mut seen = [false; 8];
    let mut rng = Rng::new(0x5EED_0064);
    // Hand-built scenes: place / displace each shape so each of circle,
    // capsule and AABB independently hits or misses.
    for circ_hit in [false, true] {
        for cap_hit in [false, true] {
            for bb_hit in [false, true] {
                let far = 5000.0f32;
                let c = if circ_hit { (0.0, 0.0, 4.0) } else { (far, far, 4.0) };
                let cap = if cap_hit {
                    (3.0, -6.0, 3.0, 6.0, 1.5)
                } else {
                    (far, -6.0, far, 6.0, 1.5)
                };
                let bb = if bb_hit {
                    (6.0, -2.0, 9.0, 2.0)
                } else {
                    (far, -2.0, far + 3.0, 2.0)
                };
                let p = params((20.0, 0.0), (-20.0, 0.0), c, cap, bb);
                let ret = diff_gen_ray(&mut d, p);
                if (0..8).contains(&ret) {
                    seen[ret as usize] = true;
                }
            }
        }
    }
    // Fill any remaining combinations from randomized scenes.
    for _ in 0..40_000 {
        let p = params(
            (rng.geo(30.0), rng.geo(30.0)),
            (rng.geo(30.0), rng.geo(30.0)),
            (rng.geo(30.0), rng.geo(30.0), rng.range(0.1, 8.0)),
            (
                rng.geo(30.0),
                rng.geo(30.0),
                rng.geo(30.0),
                rng.geo(30.0),
                rng.range(0.1, 6.0),
            ),
            {
                let x = rng.geo(30.0);
                let y = rng.geo(30.0);
                (x, y, x + rng.range(0.1, 12.0), y + rng.range(0.1, 12.0))
            },
        );
        let ret = diff_gen_ray(&mut d, p);
        if (0..8).contains(&ret) {
            seen[ret as usize] = true;
        }
    }
    for (i, s) in seen.iter().enumerate() {
        assert!(*s, "gen_ray hit-bit combination {i} was never produced");
    }
    d.finish();
}

#[test]
fn row65_gen_ray_random_bits() {
    let mut rng = Rng::new(0x5EED_0065);
    let mut d = Diffs::new("row65 gen_ray random bit patterns");
    for _ in 0..N {
        let mut p = [0.0f32; 18];
        for q in p.iter_mut() {
            *q = rng.mixed_f32();
        }
        diff_gen_ray(&mut d, p);
    }
    d.finish();
}

#[test]
fn row66_gen_ray_structured_random() {
    let mut rng = Rng::new(0x5EED_0066);
    let mut d = Diffs::new("row66 gen_ray structured random scenes");
    for _ in 0..N {
        let x = rng.geo(40.0);
        let y = rng.geo(40.0);
        let p = params(
            (rng.geo(40.0), rng.geo(40.0)),
            (rng.geo(40.0), rng.geo(40.0)),
            (rng.geo(40.0), rng.geo(40.0), rng.range(0.05, 15.0)),
            (
                rng.geo(40.0),
                rng.geo(40.0),
                rng.geo(40.0),
                rng.geo(40.0),
                rng.range(0.05, 10.0),
            ),
            (x, y, x + rng.range(0.05, 20.0), y + rng.range(0.05, 20.0)),
        );
        diff_gen_ray(&mut d, p);
    }
    d.finish();
}

#[test]
fn row67_gen_ray_degenerate() {
    let mut d = Diffs::new("row67 gen_ray degenerate scenes");
    // mp == ray.p  ->  c2Norm of the zero vector  ->  NaN ray.d and ray.t
    diff_gen_ray(
        &mut d,
        params(
            (1.0, 2.0),
            (1.0, 2.0),
            (0.0, 0.0, 4.0),
            (3.0, -6.0, 3.0, 6.0, 1.5),
            (6.0, -2.0, 9.0, 2.0),
        ),
    );
    // degenerate capsule (a == b)
    diff_gen_ray(
        &mut d,
        params(
            (20.0, 0.0),
            (-20.0, 0.0),
            (0.0, 0.0, 4.0),
            (3.0, 0.0, 3.0, 0.0, 1.5),
            (6.0, -2.0, 9.0, 2.0),
        ),
    );
    // inverted AABB
    diff_gen_ray(
        &mut d,
        params(
            (20.0, 0.0),
            (-20.0, 0.0),
            (0.0, 0.0, 4.0),
            (3.0, -6.0, 3.0, 6.0, 1.5),
            (9.0, 2.0, 6.0, -2.0),
        ),
    );
    // zero and negative radii
    for (cr, capr) in [(0.0f32, 0.0f32), (-4.0, -1.5), (0.0, -1.5), (-4.0, 0.0)] {
        diff_gen_ray(
            &mut d,
            params(
                (20.0, 0.0),
                (-20.0, 0.0),
                (0.0, 0.0, cr),
                (3.0, -6.0, 3.0, 6.0, capr),
                (6.0, -2.0, 9.0, 2.0),
            ),
        );
    }
    // degenerate (point) AABB, and AABB with signed zeros
    diff_gen_ray(
        &mut d,
        params(
            (20.0, 0.0),
            (-20.0, 0.0),
            (0.0, 0.0, 4.0),
            (3.0, -6.0, 3.0, 6.0, 1.5),
            (7.0, 0.0, 7.0, 0.0),
        ),
    );
    diff_gen_ray(
        &mut d,
        params(
            (20.0, 0.0),
            (-20.0, 0.0),
            (0.0, 0.0, 4.0),
            (3.0, -6.0, 3.0, 6.0, 1.5),
            (-0.0, -0.0, 0.0, 0.0),
        ),
    );
    d.finish();
}

#[test]
fn row68_gen_ray_special_value_per_parameter() {
    let mut d = Diffs::new("row68 gen_ray specials, one parameter at a time");
    let base = params(
        (20.0, 0.0),
        (-20.0, 0.0),
        (0.0, 0.0, 4.0),
        (3.0, -6.0, 3.0, 6.0, 1.5),
        (6.0, -2.0, 9.0, 2.0),
    );
    for i in 0..18 {
        for &s in SPECIALS {
            let mut p = base;
            p[i] = s;
            diff_gen_ray(&mut d, p);
        }
    }
    // pairs of special values across the first 16 (read) parameters
    for i in 0..16 {
        for j in 0..16 {
            for &s in &[f32::NAN, f32::INFINITY, f32::NEG_INFINITY, 0.0f32, -0.0f32] {
                let mut p = base;
                p[i] = s;
                p[j] = s;
                diff_gen_ray(&mut d, p);
            }
        }
    }
    d.finish();
}
