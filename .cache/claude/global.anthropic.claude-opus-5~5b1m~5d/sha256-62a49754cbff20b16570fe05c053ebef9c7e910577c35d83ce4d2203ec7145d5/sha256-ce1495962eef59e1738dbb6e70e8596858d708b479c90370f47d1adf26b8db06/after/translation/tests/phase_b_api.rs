//! Phase B — valid-path differential tests, `CONFIGS.md` rows 51..=71.
//!
//! Group 5: the six boolean shape-vs-shape predicates.
//! Group 6: `ptr_from_parts`, `c2Collided` and the public `omni_collide`.

mod common;
use common::*;
use std::ffi::c_void;
use std::os::raw::c_int;

// ---------------------------------------------------------------------------
// Row 51 / 52 — c2AABBtoAABB
// ---------------------------------------------------------------------------

#[test]
fn row51_aabb_to_aabb_poses() {
    for_each_pair(|c, r, label| {
        let mut rng = Rng::new(0x0200);
        let mut collide = 0usize;
        let mut apart = 0usize;
        for i in 0..N {
            let a = rng.aabb(10.0);
            let b = match rng.below(6) {
                // separated on x
                0 => c2AABB {
                    min: c2v { x: a.max.x + 1.0, y: a.min.y },
                    max: c2v { x: a.max.x + 3.0, y: a.max.y },
                },
                // separated on y
                1 => c2AABB {
                    min: c2v { x: a.min.x, y: a.max.y + 1.0 },
                    max: c2v { x: a.max.x, y: a.max.y + 3.0 },
                },
                // exactly touching (shared edge)
                2 => c2AABB {
                    min: c2v { x: a.max.x, y: a.min.y },
                    max: c2v { x: a.max.x + 2.0, y: a.max.y },
                },
                // nested
                3 => c2AABB {
                    min: c2v {
                        x: (a.min.x + a.max.x) * 0.5,
                        y: (a.min.y + a.max.y) * 0.5,
                    },
                    max: a.max,
                },
                // identical
                4 => a,
                _ => rng.aabb(10.0),
            };
            let cv = (c.c2AABBtoAABB)(a, b);
            let rv = (r.c2AABBtoAABB)(a, b);
            assert_eq!(cv, rv, "{label} row51 #{i}: A={a:?} B={b:?} -> C {cv} vs R {rv}");
            if cv != 0 {
                collide += 1
            } else {
                apart += 1
            }
        }
        assert!(collide > 0 && apart > 0, "{label} row51: one-sided coverage");
    });
}

#[test]
fn row52_aabb_to_aabb_degenerate() {
    for_each_pair(|c, r, label| {
        let mut rng = Rng::new(0x0201);
        for i in 0..N {
            let a = match rng.below(3) {
                0 => c2AABB { min: rng.v_special(), max: rng.v_special() },
                1 => {
                    let p = rng.v_ordinary(10.0);
                    let q = rng.v_ordinary(10.0);
                    c2AABB {
                        min: c2v { x: p.x.max(q.x), y: p.y.max(q.y) },
                        max: c2v { x: p.x.min(q.x), y: p.y.min(q.y) },
                    }
                }
                _ => {
                    let p = rng.v_ordinary(10.0);
                    c2AABB { min: p, max: p }
                }
            };
            let b = match rng.below(3) {
                0 => c2AABB { min: rng.v_special(), max: rng.v_special() },
                1 => rng.aabb(10.0),
                _ => {
                    let p = rng.v_ordinary(10.0);
                    c2AABB { min: p, max: p }
                }
            };
            let cv = (c.c2AABBtoAABB)(a, b);
            let rv = (r.c2AABBtoAABB)(a, b);
            assert_eq!(cv, rv, "{label} row52 #{i}: A={a:?} B={b:?} -> C {cv} vs R {rv}");
        }
        // Pinned NaN case: every `<` is false so the C returns 1.
        let nanbox = c2AABB {
            min: c2v { x: f32::NAN, y: f32::NAN },
            max: c2v { x: f32::NAN, y: f32::NAN },
        };
        let unit = c2AABB {
            min: c2v { x: 0.0, y: 0.0 },
            max: c2v { x: 1.0, y: 1.0 },
        };
        for (a, b) in [(nanbox, unit), (unit, nanbox), (nanbox, nanbox)] {
            assert_eq!(
                (c.c2AABBtoAABB)(a, b),
                (r.c2AABBtoAABB)(a, b),
                "{label} row52 pinned NaN"
            );
            assert_eq!((c.c2AABBtoAABB)(a, b), 1, "{label} row52: NaN box sanity");
        }
    });
}

// ---------------------------------------------------------------------------
// Row 53 / 54 — c2CircletoCircle
// ---------------------------------------------------------------------------

#[test]
fn row53_circle_to_circle_poses() {
    for_each_pair(|c, r, label| {
        let mut rng = Rng::new(0x0202);
        let mut hits = 0usize;
        let mut miss = 0usize;
        for i in 0..N {
            let a = rng.circle(10.0);
            let b = match rng.below(6) {
                // exactly tangent: |c| == rA + rB  (d2 == r2 -> not colliding)
                0 => c2Circle {
                    p: c2v { x: a.p.x + a.r + 2.0, y: a.p.y },
                    r: 2.0,
                },
                // clearly overlapping
                1 => c2Circle {
                    p: c2v { x: a.p.x + a.r * 0.25, y: a.p.y },
                    r: a.r + 1.0,
                },
                // coincident centres
                2 => c2Circle { p: a.p, r: rng.radius(5.0) },
                // nested
                3 => c2Circle { p: a.p, r: a.r * 0.25 },
                // far away
                4 => c2Circle {
                    p: c2v { x: a.p.x + 1000.0, y: a.p.y },
                    r: rng.radius(5.0),
                },
                _ => rng.circle(10.0),
            };
            let cv = (c.c2CircletoCircle)(a, b);
            let rv = (r.c2CircletoCircle)(a, b);
            assert_eq!(cv, rv, "{label} row53 #{i}: A={a:?} B={b:?} -> C {cv} vs R {rv}");
            if cv != 0 { hits += 1 } else { miss += 1 }
        }
        assert!(hits > 0 && miss > 0, "{label} row53: one-sided coverage");
    });
}

#[test]
fn row54_circle_to_circle_degenerate() {
    for_each_pair(|c, r, label| {
        let mut rng = Rng::new(0x0203);
        for i in 0..N {
            let mk = |rng: &mut Rng| match rng.below(4) {
                0 => c2Circle { p: rng.v_ordinary(10.0), r: 0.0 },
                1 => c2Circle { p: rng.v_ordinary(10.0), r: -rng.radius(10.0) },
                2 => c2Circle { p: rng.v_special_no_nan(), r: rng.special_no_nan() },
                _ => c2Circle { p: rng.v_special(), r: rng.special() },
            };
            let a = mk(&mut rng);
            let b = mk(&mut rng);
            let cv = (c.c2CircletoCircle)(a, b);
            let rv = (r.c2CircletoCircle)(a, b);
            assert_eq!(cv, rv, "{label} row54 #{i}: A={a:?} B={b:?} -> C {cv} vs R {rv}");
        }
        // A.r + B.r == 0 with coincident centres -> C returns 0.
        let z = c2Circle { p: c2v { x: 1.0, y: 2.0 }, r: 0.0 };
        assert_eq!((c.c2CircletoCircle)(z, z), (r.c2CircletoCircle)(z, z));
        assert_eq!((c.c2CircletoCircle)(z, z), 0, "{label} row54: zero-radius sanity");
        // negative radii that cancel
        let n = c2Circle { p: c2v { x: 1.0, y: 2.0 }, r: -3.0 };
        let p = c2Circle { p: c2v { x: 1.0, y: 2.0 }, r: 3.0 };
        assert_eq!((c.c2CircletoCircle)(n, p), (r.c2CircletoCircle)(n, p));
    });
}

// ---------------------------------------------------------------------------
// Row 55 / 56 — c2CircletoAABB
// ---------------------------------------------------------------------------

#[test]
fn row55_circle_to_aabb_poses() {
    for_each_pair(|c, r, label| {
        let mut rng = Rng::new(0x0204);
        let mut hits = 0usize;
        let mut miss = 0usize;
        for i in 0..N {
            let b = rng.aabb(10.0);
            let a = match rng.below(6) {
                // centre inside
                0 => c2Circle {
                    p: c2v {
                        x: (b.min.x + b.max.x) * 0.5,
                        y: (b.min.y + b.max.y) * 0.5,
                    },
                    r: rng.radius(5.0),
                },
                // exactly on a face
                1 => c2Circle {
                    p: c2v { x: b.max.x, y: (b.min.y + b.max.y) * 0.5 },
                    r: rng.radius(5.0),
                },
                // exactly on a corner
                2 => c2Circle { p: b.max, r: rng.radius(5.0) },
                // outside on the +x face at exactly r distance (tangent)
                3 => c2Circle {
                    p: c2v { x: b.max.x + 2.0, y: (b.min.y + b.max.y) * 0.5 },
                    r: 2.0,
                },
                // far outside
                4 => c2Circle {
                    p: c2v { x: b.max.x + 1000.0, y: b.max.y },
                    r: rng.radius(5.0),
                },
                _ => rng.circle(12.0),
            };
            let cv = (c.c2CircletoAABB)(a, b);
            let rv = (r.c2CircletoAABB)(a, b);
            assert_eq!(cv, rv, "{label} row55 #{i}: A={a:?} B={b:?} -> C {cv} vs R {rv}");
            if cv != 0 { hits += 1 } else { miss += 1 }
        }
        assert!(hits > 0 && miss > 0, "{label} row55: one-sided coverage");
    });
}

#[test]
fn row56_circle_to_aabb_degenerate() {
    for_each_pair(|c, r, label| {
        let mut rng = Rng::new(0x0205);
        for i in 0..N {
            let a = match rng.below(4) {
                0 => c2Circle { p: rng.v_ordinary(10.0), r: 0.0 },
                1 => c2Circle { p: rng.v_ordinary(10.0), r: -rng.radius(10.0) },
                2 => c2Circle { p: rng.v_special(), r: rng.special() },
                _ => rng.circle(10.0),
            };
            let b = match rng.below(4) {
                0 => {
                    let p = rng.v_ordinary(10.0);
                    let q = rng.v_ordinary(10.0);
                    c2AABB {
                        min: c2v { x: p.x.max(q.x), y: p.y.max(q.y) },
                        max: c2v { x: p.x.min(q.x), y: p.y.min(q.y) },
                    }
                }
                1 => {
                    let p = rng.v_ordinary(10.0);
                    c2AABB { min: p, max: p }
                }
                2 => c2AABB { min: rng.v_special(), max: rng.v_special() },
                _ => rng.aabb(10.0),
            };
            let cv = (c.c2CircletoAABB)(a, b);
            let rv = (r.c2CircletoAABB)(a, b);
            assert_eq!(cv, rv, "{label} row56 #{i}: A={a:?} B={b:?} -> C {cv} vs R {rv}");
        }
    });
}

// ---------------------------------------------------------------------------
// Row 57 / 58 — c2CircletoCapsule
// ---------------------------------------------------------------------------

/// Which of the three nearest-feature branches the C takes (coverage only).
fn classify_circle_capsule(a: c2Circle, b: c2Capsule) -> usize {
    let n = c2v { x: b.b.x - b.a.x, y: b.b.y - b.a.y };
    let ap = c2v { x: a.p.x - b.a.x, y: a.p.y - b.a.y };
    let da = ap.x * n.x + ap.y * n.y;
    if da < 0.0 {
        return 0;
    }
    let bp = c2v { x: a.p.x - b.b.x, y: a.p.y - b.b.y };
    let db = bp.x * n.x + bp.y * n.y;
    if db < 0.0 {
        1
    } else {
        2
    }
}

#[test]
fn row57_circle_to_capsule_branches() {
    for_each_pair(|c, r, label| {
        let mut rng = Rng::new(0x0206);
        let mut seen = [0usize; 3];
        for i in 0..N {
            let cap = c2Capsule {
                a: c2v { x: -5.0, y: 0.0 },
                b: c2v { x: 5.0, y: 0.0 },
                r: rng.radius(3.0),
            };
            let cap = match rng.below(3) {
                0 => cap,
                1 => rng.capsule(10.0),
                _ => c2Capsule {
                    a: rng.v_ordinary(10.0),
                    b: rng.v_ordinary(10.0),
                    r: rng.radius(3.0),
                },
            };
            // Place the circle before a, on the segment, or past b.
            let circ = match rng.below(4) {
                0 => c2Circle {
                    p: c2v {
                        x: cap.a.x - (cap.b.x - cap.a.x),
                        y: cap.a.y - (cap.b.y - cap.a.y),
                    },
                    r: rng.radius(3.0),
                },
                1 => c2Circle {
                    p: c2v {
                        x: (cap.a.x + cap.b.x) * 0.5 + rng.ordinary(1.0),
                        y: (cap.a.y + cap.b.y) * 0.5 + rng.ordinary(1.0),
                    },
                    r: rng.radius(3.0),
                },
                2 => c2Circle {
                    p: c2v {
                        x: cap.b.x + (cap.b.x - cap.a.x),
                        y: cap.b.y + (cap.b.y - cap.a.y),
                    },
                    r: rng.radius(3.0),
                },
                _ => rng.circle(12.0),
            };
            seen[classify_circle_capsule(circ, cap)] += 1;
            let cv = (c.c2CircletoCapsule)(circ, cap);
            let rv = (r.c2CircletoCapsule)(circ, cap);
            assert_eq!(
                cv, rv,
                "{label} row57 #{i}: A={circ:?} B={cap:?} -> C {cv} vs R {rv}"
            );
        }
        assert!(
            seen.iter().all(|&n| n > 50),
            "{label} row57: branch coverage {seen:?}"
        );
        println!("{label} row57 circle/capsule branch coverage: {seen:?}");
    });
}

#[test]
fn row58_circle_to_capsule_degenerate() {
    for_each_pair(|c, r, label| {
        let mut rng = Rng::new(0x0207);
        for i in 0..N {
            let circ = match rng.below(4) {
                0 => c2Circle { p: rng.v_ordinary(10.0), r: 0.0 },
                1 => c2Circle { p: rng.v_ordinary(10.0), r: -rng.radius(10.0) },
                2 => c2Circle { p: rng.v_special(), r: rng.special() },
                _ => rng.circle(10.0),
            };
            let cap = match rng.below(4) {
                0 => {
                    let p = rng.v_ordinary(10.0);
                    c2Capsule { a: p, b: p, r: rng.radius(3.0) }
                }
                1 => c2Capsule {
                    a: rng.v_ordinary(10.0),
                    b: rng.v_ordinary(10.0),
                    r: -rng.radius(3.0),
                },
                2 => c2Capsule {
                    a: rng.v_special(),
                    b: rng.v_special(),
                    r: rng.special(),
                },
                _ => rng.capsule(10.0),
            };
            let cv = (c.c2CircletoCapsule)(circ, cap);
            let rv = (r.c2CircletoCapsule)(circ, cap);
            assert_eq!(
                cv, rv,
                "{label} row58 #{i}: A={circ:?} B={cap:?} -> C {cv} vs R {rv}"
            );
        }
        // Pinned: degenerate capsule (a == b) => n == (0,0), da == 0, db == 0.
        let p = c2v { x: 2.0, y: -3.0 };
        let cap = c2Capsule { a: p, b: p, r: 1.0 };
        for circ in [
            c2Circle { p, r: 1.0 },
            c2Circle { p: c2v { x: 2.5, y: -3.0 }, r: 1.0 },
            c2Circle { p: c2v { x: 20.0, y: -3.0 }, r: 1.0 },
        ] {
            assert_eq!(
                (c.c2CircletoCapsule)(circ, cap),
                (r.c2CircletoCapsule)(circ, cap),
                "{label} row58 pinned degenerate capsule"
            );
        }
    });
}

// ---------------------------------------------------------------------------
// Row 59 / 60 — c2AABBtoCapsule  (delegates to c2GJK)
// ---------------------------------------------------------------------------

#[test]
fn row59_aabb_to_capsule_poses() {
    for_each_pair(|c, r, label| {
        let mut rng = Rng::new(0x0208);
        let mut hits = 0usize;
        let mut miss = 0usize;
        for i in 0..N_SLOW * 4 {
            let bb = rng.aabb(10.0);
            let cap = match rng.below(4) {
                // crossing the box
                0 => c2Capsule {
                    a: c2v { x: bb.min.x - 3.0, y: (bb.min.y + bb.max.y) * 0.5 },
                    b: c2v { x: bb.max.x + 3.0, y: (bb.min.y + bb.max.y) * 0.5 },
                    r: rng.radius(2.0),
                },
                // fully inside
                1 => c2Capsule {
                    a: c2v { x: (bb.min.x + bb.max.x) * 0.5, y: (bb.min.y + bb.max.y) * 0.5 },
                    b: c2v { x: (bb.min.x + bb.max.x) * 0.5, y: (bb.min.y + bb.max.y) * 0.5 },
                    r: rng.radius(1.0),
                },
                // far away
                2 => c2Capsule {
                    a: c2v { x: bb.max.x + 100.0, y: bb.max.y + 100.0 },
                    b: c2v { x: bb.max.x + 110.0, y: bb.max.y + 105.0 },
                    r: rng.radius(2.0),
                },
                _ => rng.capsule(12.0),
            };
            let cv = (c.c2AABBtoCapsule)(bb, cap);
            let rv = (r.c2AABBtoCapsule)(bb, cap);
            assert_eq!(
                cv, rv,
                "{label} row59 #{i}: A={bb:?} B={cap:?} -> C {cv} vs R {rv}"
            );
            if cv != 0 { hits += 1 } else { miss += 1 }
        }
        assert!(hits > 0 && miss > 0, "{label} row59: one-sided coverage");
    });
}

#[test]
fn row60_aabb_to_capsule_degenerate() {
    for_each_pair(|c, r, label| {
        let mut rng = Rng::new(0x0209);
        for i in 0..N_SLOW * 4 {
            let bb = match rng.below(4) {
                0 => {
                    let p = rng.v_ordinary(10.0);
                    let q = rng.v_ordinary(10.0);
                    c2AABB {
                        min: c2v { x: p.x.max(q.x), y: p.y.max(q.y) },
                        max: c2v { x: p.x.min(q.x), y: p.y.min(q.y) },
                    }
                }
                1 => {
                    let p = rng.v_ordinary(10.0);
                    c2AABB { min: p, max: p }
                }
                2 => c2AABB { min: rng.v_special_no_nan(), max: rng.v_special_no_nan() },
                _ => rng.aabb(10.0),
            };
            let cap = match rng.below(4) {
                0 => {
                    let p = rng.v_ordinary(10.0);
                    c2Capsule { a: p, b: p, r: 0.0 }
                }
                1 => c2Capsule {
                    a: rng.v_ordinary(10.0),
                    b: rng.v_ordinary(10.0),
                    r: -rng.radius(3.0),
                },
                2 => c2Capsule {
                    a: rng.v_special_no_nan(),
                    b: rng.v_special_no_nan(),
                    r: rng.special_no_nan(),
                },
                _ => rng.capsule(10.0),
            };
            let cv = (c.c2AABBtoCapsule)(bb, cap);
            let rv = (r.c2AABBtoCapsule)(bb, cap);
            assert_eq!(
                cv, rv,
                "{label} row60 #{i}: A={bb:?} B={cap:?} -> C {cv} vs R {rv}"
            );
        }
    });
}

// ---------------------------------------------------------------------------
// Row 61 / 62 — c2CapsuletoCapsule
// ---------------------------------------------------------------------------

#[test]
fn row61_capsule_to_capsule_poses() {
    for_each_pair(|c, r, label| {
        let mut rng = Rng::new(0x020A);
        let mut hits = 0usize;
        let mut miss = 0usize;
        for i in 0..N_SLOW * 4 {
            let a = c2Capsule {
                a: c2v { x: -5.0, y: 0.0 },
                b: c2v { x: 5.0, y: 0.0 },
                r: rng.radius(2.0),
            };
            let b = match rng.below(6) {
                // crossing
                0 => c2Capsule {
                    a: c2v { x: 0.0, y: -5.0 },
                    b: c2v { x: 0.0, y: 5.0 },
                    r: rng.radius(2.0),
                },
                // parallel, offset in y
                1 => c2Capsule {
                    a: c2v { x: -5.0, y: rng.ordinary(6.0) },
                    b: c2v { x: 5.0, y: rng.ordinary(6.0) },
                    r: rng.radius(2.0),
                },
                // collinear, overlapping
                2 => c2Capsule {
                    a: c2v { x: 0.0, y: 0.0 },
                    b: c2v { x: 10.0, y: 0.0 },
                    r: rng.radius(2.0),
                },
                // coincident
                3 => a,
                // far away
                4 => c2Capsule {
                    a: c2v { x: 1000.0, y: 1000.0 },
                    b: c2v { x: 1010.0, y: 1005.0 },
                    r: rng.radius(2.0),
                },
                _ => rng.capsule(12.0),
            };
            let a = if rng.below(3) == 0 { rng.capsule(12.0) } else { a };
            let cv = (c.c2CapsuletoCapsule)(a, b);
            let rv = (r.c2CapsuletoCapsule)(a, b);
            assert_eq!(
                cv, rv,
                "{label} row61 #{i}: A={a:?} B={b:?} -> C {cv} vs R {rv}"
            );
            if cv != 0 { hits += 1 } else { miss += 1 }
        }
        assert!(hits > 0 && miss > 0, "{label} row61: one-sided coverage");
    });
}

#[test]
fn row62_capsule_to_capsule_degenerate() {
    for_each_pair(|c, r, label| {
        let mut rng = Rng::new(0x020B);
        for i in 0..N_SLOW * 4 {
            let mk = |rng: &mut Rng| match rng.below(4) {
                0 => {
                    let p = rng.v_ordinary(10.0);
                    c2Capsule { a: p, b: p, r: 0.0 }
                }
                1 => {
                    let p = rng.v_ordinary(10.0);
                    c2Capsule { a: p, b: p, r: rng.radius(3.0) }
                }
                2 => c2Capsule {
                    a: rng.v_ordinary(10.0),
                    b: rng.v_ordinary(10.0),
                    r: -rng.radius(3.0),
                },
                _ => c2Capsule {
                    a: rng.v_special_no_nan(),
                    b: rng.v_special_no_nan(),
                    r: rng.special_no_nan(),
                },
            };
            let a = mk(&mut rng);
            let b = mk(&mut rng);
            let cv = (c.c2CapsuletoCapsule)(a, b);
            let rv = (r.c2CapsuletoCapsule)(a, b);
            assert_eq!(
                cv, rv,
                "{label} row62 #{i}: A={a:?} B={b:?} -> C {cv} vs R {rv}"
            );
        }
    });
}

// ---------------------------------------------------------------------------
// Row 63..=65 — ptr_from_parts
// ---------------------------------------------------------------------------

fn ptr_from_parts_axis(c: &Api, r: &Api, label: &str, row: &str, seed: u64, ty: c_int, size: usize) {
    let mut rng = Rng::new(seed);
    for i in 0..N {
        let p: [f32; 5] = [
            rng.special(),
            rng.special(),
            rng.special(),
            rng.special(),
            rng.special(),
        ];
        let p = if rng.below(2) == 0 {
            [
                rng.ordinary(1.0e3),
                rng.ordinary(1.0e3),
                rng.ordinary(1.0e3),
                rng.ordinary(1.0e3),
                rng.ordinary(1.0e3),
            ]
        } else {
            p
        };
        unsafe {
            let cp = (c.ptr_from_parts)(ty, p[0], p[1], p[2], p[3], p[4]);
            let rp = (r.ptr_from_parts)(ty, p[0], p[1], p[2], p[3], p[4]);
            assert!(!cp.is_null(), "{label} {row} #{i}: C returned NULL for ty={ty}");
            assert!(!rp.is_null(), "{label} {row} #{i}: Rust returned NULL for ty={ty}");
            let cb = std::slice::from_raw_parts(cp as *const u8, size);
            let rb = std::slice::from_raw_parts(rp as *const u8, size);
            // Compare as f32 lanes so NaN payload differences are tolerated the
            // same way as everywhere else in the harness.
            for k in 0..size / 4 {
                let cf = f32::from_bits(u32::from_ne_bytes(cb[k * 4..k * 4 + 4].try_into().unwrap()));
                let rf = f32::from_bits(u32::from_ne_bytes(rb[k * 4..k * 4 + 4].try_into().unwrap()));
                assert!(
                    f32_same(cf, rf),
                    "{label} {row} #{i} ty={ty} lane {k}: parts={p:?} -> C {} vs R {}",
                    fmt_f32(cf),
                    fmt_f32(rf)
                );
            }
            free(cp);
            free(rp);
        }
    }
}

#[test]
fn row63_ptr_from_parts_circle() {
    for_each_pair(|c, r, label| {
        ptr_from_parts_axis(c, r, label, "row63", 0x020C, C2_TYPE_CIRCLE, 12)
    });
}

#[test]
fn row64_ptr_from_parts_aabb() {
    for_each_pair(|c, r, label| {
        ptr_from_parts_axis(c, r, label, "row64", 0x020D, C2_TYPE_AABB, 16)
    });
}

#[test]
fn row65_ptr_from_parts_capsule() {
    for_each_pair(|c, r, label| {
        ptr_from_parts_axis(c, r, label, "row65", 0x020E, C2_TYPE_CAPSULE, 20)
    });
}

// ---------------------------------------------------------------------------
// Row 66 / 67 — c2Collided over all nine type pairs
// ---------------------------------------------------------------------------

fn collided_axis(c: &Api, r: &Api, label: &str, row: &str, seed: u64, degenerate: bool) {
    let mut rng = Rng::new(seed);
    let mut hits = 0usize;
    let mut miss = 0usize;
    for &ta in VALID_TYPES.iter() {
        for &tb in VALID_TYPES.iter() {
            for i in 0..N_SLOW {
                let (sa, sb) = if degenerate {
                    (
                        Shape::random_degenerate(&mut rng, ta, 10.0),
                        Shape::random_degenerate(&mut rng, tb, 10.0),
                    )
                } else {
                    (
                        Shape::random(&mut rng, ta, 10.0),
                        Shape::random(&mut rng, tb, 10.0),
                    )
                };
                let (cv, rv) = collided_both(c, r, &sa, &sb);
                assert_eq!(
                    cv, rv,
                    "{label} {row} #{i} ta={ta} tb={tb}: A={sa:?} B={sb:?} -> C {cv} vs R {rv}"
                );
                if cv != 0 { hits += 1 } else { miss += 1 }
            }
        }
    }
    assert!(hits > 0 && miss > 0, "{label} {row}: one-sided coverage");
    println!("{label} {row}: hits={hits} misses={miss}");
}

#[test]
fn row66_collided_all_type_pairs() {
    for_each_pair(|c, r, label| collided_axis(c, r, label, "row66", 0x020F, false));
}

#[test]
fn row67_collided_degenerate() {
    for_each_pair(|c, r, label| collided_axis(c, r, label, "row67", 0x0210, true));
}

// ---------------------------------------------------------------------------
// Row 68..=71 — omni_collide (the public one-shot API)
// ---------------------------------------------------------------------------

#[derive(Copy, Clone, Debug)]
enum OmniMode {
    Ordinary,
    Targeted,
    Extreme,
    Degenerate,
}

fn omni_axis(c: &Api, r: &Api, label: &str, row: &str, seed: u64, mode: OmniMode) {
    let mut rng = Rng::new(seed);
    let mut hits = 0usize;
    let mut miss = 0usize;
    for &ta in VALID_TYPES.iter() {
        for &tb in VALID_TYPES.iter() {
            for i in 0..N_SLOW {
                let (pa, pb) = match mode {
                    OmniMode::Ordinary => (
                        Shape::random(&mut rng, ta, 10.0).parts(),
                        Shape::random(&mut rng, tb, 10.0).parts(),
                    ),
                    OmniMode::Targeted => {
                        // Guaranteed-overlapping or guaranteed-separated placements.
                        let overlap = rng.bool();
                        let s = if overlap { 0.5f32 } else { 500.0f32 };
                        let mk = |t: c_int, off: f32| -> [f32; 5] {
                            match t {
                                C2_TYPE_CIRCLE => [off, off, 2.0, 0.0, 0.0],
                                C2_TYPE_AABB => [off - 1.0, off - 1.0, off + 1.0, off + 1.0, 0.0],
                                _ => [off - 1.0, off, off + 1.0, off, 1.0],
                            }
                        };
                        (mk(ta, 0.0), mk(tb, s))
                    }
                    OmniMode::Extreme => (
                        Shape::random_extreme(&mut rng, ta).parts(),
                        Shape::random_extreme(&mut rng, tb).parts(),
                    ),
                    OmniMode::Degenerate => (
                        Shape::random_degenerate(&mut rng, ta, 10.0).parts(),
                        Shape::random_degenerate(&mut rng, tb, 10.0).parts(),
                    ),
                };
                let cv = unsafe {
                    (c.omni_collide)(
                        ta, pa[0], pa[1], pa[2], pa[3], pa[4], tb, pb[0], pb[1], pb[2], pb[3],
                        pb[4],
                    )
                };
                let rv = unsafe {
                    (r.omni_collide)(
                        ta, pa[0], pa[1], pa[2], pa[3], pa[4], tb, pb[0], pb[1], pb[2], pb[3],
                        pb[4],
                    )
                };
                assert_eq!(
                    cv, rv,
                    "{label} {row} #{i} ta={ta} tb={tb}: pa={pa:?} pb={pb:?} -> C {cv} vs R {rv}"
                );
                if cv != 0 { hits += 1 } else { miss += 1 }
            }
        }
    }
    println!("{label} {row}: hits={hits} misses={miss}");
    assert!(hits > 0 && miss > 0, "{label} {row}: one-sided coverage");
}

#[test]
fn row68_omni_ordinary() {
    for_each_pair(|c, r, label| omni_axis(c, r, label, "row68", 0x0211, OmniMode::Ordinary));
}

#[test]
fn row69_omni_targeted() {
    for_each_pair(|c, r, label| omni_axis(c, r, label, "row69", 0x0212, OmniMode::Targeted));
}

#[test]
fn row70_omni_extreme() {
    for_each_pair(|c, r, label| omni_axis(c, r, label, "row70", 0x0213, OmniMode::Extreme));
}

#[test]
fn row71_omni_degenerate() {
    for_each_pair(|c, r, label| omni_axis(c, r, label, "row71", 0x0214, OmniMode::Degenerate));
}

// A tiny compile-time use so `c_void` import is not dead.
#[allow(dead_code)]
fn _unused(_: *const c_void) {}
