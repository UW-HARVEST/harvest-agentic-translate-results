//! Phase B — rows 77..87: the boolean collision helpers, the `c2Collided`
//! dispatcher over all nine valid type pairs, and the `aabb` entry point from
//! `include/lib.h`.

#![allow(non_snake_case)]

mod common;

use common::*;
use std::ffi::c_int;

const N: usize = 6000;

// ---- row 77: c2AABBtoAABB ------------------------------------------------

#[test]
fn cfg77_aabb_to_aabb() {
    let (cf, rf) = pair::<extern "C" fn(c2AABB, c2AABB) -> c_int>("c2AABBtoAABB");
    let mut rng = Rng::new(0x7777);
    let mut hits = [0usize; 2];

    for _ in 0..N {
        let a = rng.aabb();
        let b = rng.aabb();
        let v = cf(a, b);
        assert_same("c2AABBtoAABB", v, rf(a, b));
        hits[(v != 0) as usize] += 1;
    }
    // Deliberate configurations: disjoint on x only, on y only, edge touching,
    // fully contained, zero extent, inverted.
    let base = c2AABB {
        min: c2v { x: -10.0, y: -10.0 },
        max: c2v { x: 10.0, y: 10.0 },
    };
    let cases = [
        c2AABB { min: c2v { x: 11.0, y: 0.0 }, max: c2v { x: 20.0, y: 5.0 } }, // disjoint x
        c2AABB { min: c2v { x: 0.0, y: 11.0 }, max: c2v { x: 5.0, y: 20.0 } }, // disjoint y
        c2AABB { min: c2v { x: 10.0, y: 0.0 }, max: c2v { x: 20.0, y: 5.0 } }, // touching x
        c2AABB { min: c2v { x: 0.0, y: 10.0 }, max: c2v { x: 5.0, y: 20.0 } }, // touching y
        c2AABB { min: c2v { x: -1.0, y: -1.0 }, max: c2v { x: 1.0, y: 1.0 } }, // contained
        c2AABB { min: c2v { x: 0.0, y: 0.0 }, max: c2v { x: 0.0, y: 0.0 } },   // zero extent
        c2AABB { min: c2v { x: 5.0, y: 5.0 }, max: c2v { x: -5.0, y: -5.0 } }, // inverted
        c2AABB { min: c2v { x: f32::NAN, y: 0.0 }, max: c2v { x: 1.0, y: 1.0 } },
        c2AABB {
            min: c2v { x: f32::NEG_INFINITY, y: f32::NEG_INFINITY },
            max: c2v { x: f32::INFINITY, y: f32::INFINITY },
        },
    ];
    for b in cases {
        assert_same("c2AABBtoAABB case", cf(base, b), rf(base, b));
        assert_same("c2AABBtoAABB case rev", cf(b, base), rf(b, base));
    }
    for _ in 0..N {
        let a = c2AABB { min: rng.spicy_vec(), max: rng.spicy_vec() };
        let b = c2AABB { min: rng.spicy_vec(), max: rng.spicy_vec() };
        assert_same("c2AABBtoAABB spicy", cf(a, b), rf(a, b));
    }
    assert!(hits[0] > 0 && hits[1] > 0, "one-sided coverage: {hits:?}");
}

// ---- row 78: c2CircletoCircle -------------------------------------------

#[test]
fn cfg78_circle_to_circle() {
    let (cf, rf) = pair::<extern "C" fn(c2Circle, c2Circle) -> c_int>("c2CircletoCircle");
    let mut rng = Rng::new(0x7878);
    let mut hits = [0usize; 2];

    for _ in 0..N {
        let a = rng.circle();
        let b = rng.circle();
        let v = cf(a, b);
        assert_same("c2CircletoCircle", v, rf(a, b));
        hits[(v != 0) as usize] += 1;
    }
    // Exactly touching (`d2 == r2`, so the strict `<` must reject), concentric,
    // zero radius, negative radius, non-finite.
    for _ in 0..N {
        let ra = rng.radius();
        let rb = rng.radius();
        let a = c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: ra };
        let b = c2Circle { p: c2v { x: ra + rb, y: 0.0 }, r: rb };
        assert_same("c2CircletoCircle touching", cf(a, b), rf(a, b));
        let c = c2Circle { p: a.p, r: rb };
        assert_same("c2CircletoCircle concentric", cf(a, c), rf(a, c));
        let z = c2Circle { p: rng.vec(), r: 0.0 };
        assert_same("c2CircletoCircle r=0", cf(a, z), rf(a, z));
        let n = c2Circle { p: rng.vec(), r: -ra };
        assert_same("c2CircletoCircle r<0", cf(a, n), rf(a, n));
    }
    for _ in 0..N {
        let a = c2Circle { p: rng.spicy_vec(), r: rng.spicy_f32() };
        let b = c2Circle { p: rng.spicy_vec(), r: rng.spicy_f32() };
        assert_same("c2CircletoCircle spicy", cf(a, b), rf(a, b));
    }
    assert!(hits[0] > 0 && hits[1] > 0, "one-sided coverage: {hits:?}");
}

// ---- row 79: c2CircletoAABB ---------------------------------------------

#[test]
fn cfg79_circle_to_aabb() {
    let (cf, rf) = pair::<extern "C" fn(c2Circle, c2AABB) -> c_int>("c2CircletoAABB");
    let mut rng = Rng::new(0x7979);
    let mut hits = [0usize; 2];

    for _ in 0..N {
        let a = rng.circle();
        let b = rng.aabb();
        let v = cf(a, b);
        assert_same("c2CircletoAABB", v, rf(a, b));
        hits[(v != 0) as usize] += 1;
    }
    // All nine Voronoi regions of the box (inside, 4 edges, 4 corners) plus the
    // boundary, zero radius, inverted box.
    let bb = c2AABB {
        min: c2v { x: -10.0, y: -10.0 },
        max: c2v { x: 10.0, y: 10.0 },
    };
    let inv = c2AABB { min: bb.max, max: bb.min };
    let centres = [
        c2v { x: 0.0, y: 0.0 },
        c2v { x: -20.0, y: 0.0 },
        c2v { x: 20.0, y: 0.0 },
        c2v { x: 0.0, y: -20.0 },
        c2v { x: 0.0, y: 20.0 },
        c2v { x: -20.0, y: -20.0 },
        c2v { x: 20.0, y: -20.0 },
        c2v { x: -20.0, y: 20.0 },
        c2v { x: 20.0, y: 20.0 },
        c2v { x: 10.0, y: 10.0 },
        c2v { x: -10.0, y: 0.0 },
    ];
    for p in centres {
        for r in [0.0f32, 1e-7, 5.0, 10.0, 14.142136, 14.142135, 1e30, -5.0] {
            let a = c2Circle { p, r };
            assert_same("c2CircletoAABB region", cf(a, bb), rf(a, bb));
            assert_same("c2CircletoAABB inverted box", cf(a, inv), rf(a, inv));
        }
    }
    for _ in 0..N {
        let a = c2Circle { p: rng.spicy_vec(), r: rng.spicy_f32() };
        let b = c2AABB { min: rng.spicy_vec(), max: rng.spicy_vec() };
        assert_same("c2CircletoAABB spicy", cf(a, b), rf(a, b));
    }
    assert!(hits[0] > 0 && hits[1] > 0, "one-sided coverage: {hits:?}");
}

// ---- row 80: c2CircletoCapsule ------------------------------------------

#[test]
fn cfg80_circle_to_capsule() {
    let (cf, rf) = pair::<extern "C" fn(c2Circle, c2Capsule) -> c_int>("c2CircletoCapsule");
    let mut rng = Rng::new(0x8080);
    let mut hits = [0usize; 2];
    // Which of the three internal branches (da<0 / db<0 / else) was taken.
    let mut branch = [0usize; 3];

    for _ in 0..N {
        let a = rng.circle();
        let b = rng.capsule();
        let v = cf(a, b);
        assert_same("c2CircletoCapsule", v, rf(a, b));
        hits[(v != 0) as usize] += 1;

        let n = c2v { x: b.b.x - b.a.x, y: b.b.y - b.a.y };
        let ap = c2v { x: a.p.x - b.a.x, y: a.p.y - b.a.y };
        let da = ap.x * n.x + ap.y * n.y;
        if da < 0.0 {
            branch[0] += 1;
        } else {
            let bp = c2v { x: a.p.x - b.b.x, y: a.p.y - b.b.y };
            let db = bp.x * n.x + bp.y * n.y;
            if db < 0.0 {
                branch[1] += 1;
            } else {
                branch[2] += 1;
            }
        }
    }
    // Degenerate capsule (a == b): n == (0,0), so da == 0 and db == 0 and the
    // `else` branch is taken without any division.
    for _ in 0..N {
        let p = rng.vec();
        let b = c2Capsule { a: p, b: p, r: rng.radius() };
        let a = rng.circle();
        assert_same("c2CircletoCapsule degenerate", cf(a, b), rf(a, b));
        let zr = c2Capsule { a: rng.vec(), b: rng.vec(), r: 0.0 };
        assert_same("c2CircletoCapsule r=0", cf(a, zr), rf(a, zr));
        // Near-zero axis: da / dot(n,n) overflows.
        let tiny = c2Capsule {
            a: p,
            b: c2v { x: p.x + 1e-22, y: p.y + 1e-22 },
            r: rng.radius(),
        };
        assert_same("c2CircletoCapsule tiny axis", cf(a, tiny), rf(a, tiny));
    }
    for _ in 0..N {
        let a = c2Circle { p: rng.spicy_vec(), r: rng.spicy_f32() };
        let b = c2Capsule {
            a: rng.spicy_vec(),
            b: rng.spicy_vec(),
            r: rng.spicy_f32(),
        };
        assert_same("c2CircletoCapsule spicy", cf(a, b), rf(a, b));
    }
    assert!(hits[0] > 0 && hits[1] > 0, "one-sided coverage: {hits:?}");
    assert!(
        branch.iter().all(|&h| h > 0),
        "c2CircletoCapsule branch coverage: {branch:?}"
    );
}

// ---- rows 81 & 82: the two GJK-backed boolean helpers -------------------

#[test]
fn cfg81_82_gjk_backed_booleans() {
    let (c_ac, r_ac) = pair::<extern "C" fn(c2AABB, c2Capsule) -> c_int>("c2AABBtoCapsule");
    let (c_cc, r_cc) = pair::<extern "C" fn(c2Capsule, c2Capsule) -> c_int>("c2CapsuletoCapsule");
    let mut rng = Rng::new(0x8181);
    let mut hits_ac = [0usize; 2];
    let mut hits_cc = [0usize; 2];

    for _ in 0..N {
        let bb = rng.aabb();
        let cap = rng.capsule();
        let v = c_ac(bb, cap);
        assert_same("c2AABBtoCapsule", v, r_ac(bb, cap));
        hits_ac[(v != 0) as usize] += 1;

        let c1 = rng.capsule();
        let c2 = rng.capsule();
        let w = c_cc(c1, c2);
        assert_same("c2CapsuletoCapsule", w, r_cc(c1, c2));
        hits_cc[(w != 0) as usize] += 1;
    }
    // Structured capsule configurations: parallel, crossing, collinear,
    // separated, degenerate.
    let structured: [(c2Capsule, c2Capsule); 6] = [
        (
            c2Capsule { a: c2v { x: -10.0, y: 0.0 }, b: c2v { x: 10.0, y: 0.0 }, r: 2.0 },
            c2Capsule { a: c2v { x: -10.0, y: 3.0 }, b: c2v { x: 10.0, y: 3.0 }, r: 2.0 },
        ), // parallel, overlapping
        (
            c2Capsule { a: c2v { x: -10.0, y: 0.0 }, b: c2v { x: 10.0, y: 0.0 }, r: 2.0 },
            c2Capsule { a: c2v { x: -10.0, y: 5.0 }, b: c2v { x: 10.0, y: 5.0 }, r: 2.0 },
        ), // parallel, exactly touching
        (
            c2Capsule { a: c2v { x: -10.0, y: 0.0 }, b: c2v { x: 10.0, y: 0.0 }, r: 1.0 },
            c2Capsule { a: c2v { x: 0.0, y: -10.0 }, b: c2v { x: 0.0, y: 10.0 }, r: 1.0 },
        ), // crossing
        (
            c2Capsule { a: c2v { x: -10.0, y: 0.0 }, b: c2v { x: 0.0, y: 0.0 }, r: 1.0 },
            c2Capsule { a: c2v { x: 5.0, y: 0.0 }, b: c2v { x: 15.0, y: 0.0 }, r: 1.0 },
        ), // collinear, separated
        (
            c2Capsule { a: c2v { x: 0.0, y: 0.0 }, b: c2v { x: 0.0, y: 0.0 }, r: 3.0 },
            c2Capsule { a: c2v { x: 1.0, y: 0.0 }, b: c2v { x: 1.0, y: 0.0 }, r: 3.0 },
        ), // both degenerate
        (
            c2Capsule { a: c2v { x: 0.0, y: 0.0 }, b: c2v { x: 0.0, y: 0.0 }, r: 0.0 },
            c2Capsule { a: c2v { x: 0.0, y: 0.0 }, b: c2v { x: 0.0, y: 0.0 }, r: 0.0 },
        ), // fully degenerate
    ];
    for (x, y) in structured {
        assert_same("c2CapsuletoCapsule structured", c_cc(x, y), r_cc(x, y));
        assert_same("c2CapsuletoCapsule structured rev", c_cc(y, x), r_cc(y, x));
        let bb = c2AABB { min: c2v { x: -3.0, y: -3.0 }, max: c2v { x: 3.0, y: 3.0 } };
        assert_same("c2AABBtoCapsule structured", c_ac(bb, x), r_ac(bb, x));
        let deg = c2AABB { min: c2v { x: 0.0, y: 0.0 }, max: c2v { x: 0.0, y: 0.0 } };
        assert_same("c2AABBtoCapsule degenerate box", c_ac(deg, x), r_ac(deg, x));
        let inv = c2AABB { min: c2v { x: 3.0, y: 3.0 }, max: c2v { x: -3.0, y: -3.0 } };
        assert_same("c2AABBtoCapsule inverted box", c_ac(inv, x), r_ac(inv, x));
    }
    for _ in 0..N {
        let bb = c2AABB { min: rng.spicy_vec(), max: rng.spicy_vec() };
        let cap = c2Capsule { a: rng.spicy_vec(), b: rng.spicy_vec(), r: rng.spicy_f32() };
        let cap2 = c2Capsule { a: rng.spicy_vec(), b: rng.spicy_vec(), r: rng.spicy_f32() };
        assert_same("c2AABBtoCapsule spicy", c_ac(bb, cap), r_ac(bb, cap));
        assert_same("c2CapsuletoCapsule spicy", c_cc(cap, cap2), r_cc(cap, cap2));
    }
    assert!(hits_ac[0] > 0 && hits_ac[1] > 0, "AABBtoCapsule one-sided: {hits_ac:?}");
    assert!(hits_cc[0] > 0 && hits_cc[1] > 0, "CapsuletoCapsule one-sided: {hits_cc:?}");
}

// ---- rows 83 & 84: c2Collided over all nine pairs ----------------------

#[test]
fn cfg83_84_collided_all_pairs() {
    let (cf, rf) = pair::<FnCollided>("c2Collided");
    let mut rng = Rng::new(0x8383);

    for (ta, tb) in TYPE_PAIRS {
        let mut hits = [0usize; 2];
        for _ in 0..N {
            let a = rand_shape(&mut rng, ta);
            let b = rand_shape(&mut rng, tb);
            let cv = unsafe { cf(a.ptr(), ta, b.ptr(), tb) };
            let rv = unsafe { rf(a.ptr(), ta, b.ptr(), tb) };
            assert_same(&format!("c2Collided({ta},{tb})"), cv, rv);
            hits[(cv != 0) as usize] += 1;
        }
        assert!(
            hits[0] > 0 && hits[1] > 0,
            "c2Collided({ta},{tb}) one-sided coverage: {hits:?}"
        );

        // row 84 — degenerate / boundary / non-finite shapes.
        for _ in 0..N {
            let a = match ta {
                C2_TYPE_CIRCLE => Shape::Circle(c2Circle { p: rng.spicy_vec(), r: rng.spicy_f32() }),
                C2_TYPE_AABB => Shape::Aabb(c2AABB { min: rng.spicy_vec(), max: rng.spicy_vec() }),
                _ => Shape::Capsule(c2Capsule {
                    a: rng.spicy_vec(),
                    b: rng.spicy_vec(),
                    r: rng.spicy_f32(),
                }),
            };
            let b = match tb {
                C2_TYPE_CIRCLE => Shape::Circle(c2Circle { p: rng.spicy_vec(), r: rng.spicy_f32() }),
                C2_TYPE_AABB => Shape::Aabb(c2AABB { min: rng.spicy_vec(), max: rng.spicy_vec() }),
                _ => Shape::Capsule(c2Capsule {
                    a: rng.spicy_vec(),
                    b: rng.spicy_vec(),
                    r: rng.spicy_f32(),
                }),
            };
            let cv = unsafe { cf(a.ptr(), ta, b.ptr(), tb) };
            let rv = unsafe { rf(a.ptr(), ta, b.ptr(), tb) };
            if cv != rv {
                eprintln!("A={a:?} ta={ta} B={b:?} tb={tb}");
            }
            assert_same(&format!("c2Collided spicy({ta},{tb})"), cv, rv);
        }
    }
}

// ---- rows 85..87: the `aabb` entry point -------------------------------

#[test]
fn cfg85_87_aabb_entry_point() {
    let (cf, rf) = pair::<FnAabb>("aabb");
    let mut rng = Rng::new(0x8585);
    let mut seen = [0usize; 8];

    // row 85 — broad random sweep over the whole plane the fixture lives in.
    for _ in 0..(N * 8) {
        let (a, b, c, d) = (
            rng.range(-160.0, 60.0),
            rng.range(-60.0, 160.0),
            rng.range(-160.0, 60.0),
            rng.range(-60.0, 160.0),
        );
        let cv = cf(a, b, c, d);
        assert_same("aabb", cv, rf(a, b, c, d));
        if (0..8).contains(&cv) {
            seen[cv as usize] += 1;
        }
    }

    // row 86 — boxes placed to hit each result bit on purpose:
    //   bit0: overlaps circle   at (-70, 0) r 20
    //   bit1: overlaps AABB     [(-40,-40), (-15,-15)]
    //   bit2: overlaps capsule  (-40,40)..(-20,100) r 10
    let targeted: [(f32, f32, f32, f32); 12] = [
        (-1000.0, -1000.0, -999.0, -999.0), // nothing
        (-80.0, -5.0, -75.0, 5.0),          // circle only
        (-35.0, -35.0, -20.0, -20.0),       // aabb only
        (-38.0, 45.0, -30.0, 55.0),         // capsule only
        (-80.0, -40.0, -20.0, 0.0),         // circle + aabb
        (-80.0, -5.0, -30.0, 60.0),         // circle + capsule
        (-45.0, -45.0, -15.0, 60.0),        // aabb + capsule
        (-200.0, -200.0, 200.0, 200.0),     // all three
        (0.0, 0.0, 0.0, 0.0),               // zero extent
        (60.0, 160.0, -160.0, -60.0),       // fully inverted
        (-70.0, 0.0, -70.0, 0.0),           // degenerate on the circle centre
        (-50.0, -50.0, -50.0, 120.0),       // zero width, tall
    ];
    for (a, b, c, d) in targeted {
        let cv = cf(a, b, c, d);
        assert_same("aabb targeted", cv, rf(a, b, c, d));
        if (0..8).contains(&cv) {
            seen[cv as usize] += 1;
        }
    }

    // row 87 — non-finite / subnormal / extreme arguments.
    for _ in 0..(N * 2) {
        let (a, b, c, d) = (
            rng.spicy_f32(),
            rng.spicy_f32(),
            rng.spicy_f32(),
            rng.spicy_f32(),
        );
        assert_same("aabb spicy", cf(a, b, c, d), rf(a, b, c, d));
    }
    for v in [
        f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::MAX,
        f32::MIN,
        f32::MIN_POSITIVE,
        0.0,
        -0.0,
    ] {
        for w in [f32::NAN, f32::INFINITY, 0.0, -100.0, 100.0] {
            assert_same("aabb extreme", cf(v, w, w, v), rf(v, w, w, v));
            assert_same("aabb extreme2", cf(v, v, w, w), rf(v, v, w, w));
        }
    }

    eprintln!("aabb result histogram: {seen:?}");
    assert_eq!(
        seen.iter().filter(|&&h| h > 0).count(),
        8,
        "not all 8 result bit patterns were reached: {seen:?}"
    );
}
